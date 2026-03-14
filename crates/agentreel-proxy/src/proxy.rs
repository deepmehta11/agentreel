use crate::Recorder;
use anyhow::Result;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Target API provider.
#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    OpenAI,
    Anthropic,
    /// Auto-detect based on request headers and path.
    Auto,
}

impl Provider {
    fn base_url(&self) -> &str {
        match self {
            Provider::OpenAI => "https://api.openai.com",
            Provider::Anthropic => "https://api.anthropic.com",
            Provider::Auto => "",
        }
    }

    /// Detect provider from environment variables.
    pub fn from_env() -> Self {
        let has_anthropic = std::env::var("ANTHROPIC_API_KEY").is_ok();
        let has_openai = std::env::var("OPENAI_API_KEY").is_ok();

        match (has_anthropic, has_openai) {
            (true, false) => Provider::Anthropic,
            (false, true) => Provider::OpenAI,
            _ => Provider::Auto,
        }
    }
}

/// Detect provider from request characteristics.
fn detect_provider(path: &str, headers: &hyper::HeaderMap) -> Provider {
    // Anthropic uses /v1/messages and x-api-key header
    if path.contains("/v1/messages") || headers.contains_key("x-api-key") {
        return Provider::Anthropic;
    }

    // Anthropic-specific header
    if headers.contains_key("anthropic-version") {
        return Provider::Anthropic;
    }

    // OpenAI uses /v1/chat/completions and Authorization: Bearer
    if path.contains("/chat/completions") || path.contains("/completions") {
        return Provider::OpenAI;
    }

    // Default to OpenAI
    Provider::OpenAI
}

/// Configuration for the proxy server.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Which provider to proxy to. Use Auto for auto-detection.
    pub provider: Provider,
    /// Override base URL for OpenAI-compatible endpoints.
    pub openai_url: Option<String>,
    /// Override base URL for Anthropic endpoints.
    pub anthropic_url: Option<String>,
}

impl ProxyConfig {
    /// Create config that auto-detects provider and reads upstream URLs from env.
    pub fn from_env() -> Self {
        Self {
            provider: Provider::from_env(),
            openai_url: std::env::var("AGENTREEL_OPENAI_UPSTREAM").ok(),
            anthropic_url: std::env::var("AGENTREEL_ANTHROPIC_UPSTREAM").ok(),
        }
    }

    fn upstream_url(&self, detected: &Provider) -> String {
        match detected {
            Provider::Anthropic => self
                .anthropic_url
                .clone()
                .unwrap_or_else(|| Provider::Anthropic.base_url().to_string()),
            Provider::OpenAI | Provider::Auto => self
                .openai_url
                .clone()
                .unwrap_or_else(|| Provider::OpenAI.base_url().to_string()),
        }
    }
}

/// An HTTP proxy that intercepts LLM API calls and records them.
pub struct ProxyServer {
    config: ProxyConfig,
    recorder: Arc<Recorder>,
    client: Client,
}

impl ProxyServer {
    pub fn new(config: ProxyConfig, recorder: Recorder) -> Self {
        Self {
            config,
            recorder: Arc::new(recorder),
            client: Client::new(),
        }
    }

    /// Start the proxy server and return the bound address.
    pub async fn start(self) -> Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        info!("Proxy listening on {}", addr);

        let shared = Arc::new(SharedState {
            config: self.config,
            recorder: self.recorder,
            client: self.client,
        });

        let handle = tokio::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Accept error: {}", e);
                        continue;
                    }
                };

                let state = shared.clone();
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    if let Err(e) = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req| {
                                let state = state.clone();
                                async move { handle_request(req, state).await }
                            }),
                        )
                        .await
                    {
                        error!("Connection error: {}", e);
                    }
                });
            }
        });

        Ok((addr, handle))
    }
}

struct SharedState {
    config: ProxyConfig,
    recorder: Arc<Recorder>,
    client: Client,
}

async fn handle_request(
    req: Request<Incoming>,
    state: Arc<SharedState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let headers = req.headers().clone();

    // Auto-detect provider if configured as Auto
    let provider = if state.config.provider == Provider::Auto {
        detect_provider(&path, &headers)
    } else {
        state.config.provider.clone()
    };

    info!("{} {} (provider: {:?})", method, path, provider);

    // Read request body
    let body_bytes = match req.collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return Ok(error_response(500, "Failed to read request body"));
        }
    };

    let request_body: serde_json::Value =
        serde_json::from_slice(&body_bytes).unwrap_or_default();

    // Check if client requested streaming
    let is_streaming = request_body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Determine upstream URL
    let base = state.config.upstream_url(&provider);
    let upstream_url = format!("{}{}", base, path);

    // Build upstream request — force non-streaming so we can capture full response
    let mut modified_body = request_body.clone();
    if is_streaming {
        if let Some(obj) = modified_body.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(false));
        }
    }

    let mut upstream_req = state.client.request(method.clone(), &upstream_url);

    // Forward relevant headers
    for (name, value) in headers.iter() {
        let name_str = name.as_str().to_lowercase();
        match name_str.as_str() {
            "host" | "connection" | "transfer-encoding" | "content-length" => continue,
            _ => {
                upstream_req = upstream_req.header(name, value);
            }
        }
    }

    let send_body = serde_json::to_vec(&modified_body).unwrap_or_else(|_| body_bytes.to_vec());
    upstream_req = upstream_req.body(send_body);

    // Send request and time it
    let start = Instant::now();
    let upstream_resp = match upstream_req.send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Upstream error: {}", e);
            return Ok(error_response(502, &format!("Upstream error: {}", e)));
        }
    };
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();
    let resp_bytes = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read upstream response: {}", e);
            return Ok(error_response(502, "Failed to read upstream response"));
        }
    };

    // Record the call if it's an LLM endpoint (success OR error)
    if is_llm_endpoint(&path) {
        // Collect headers
        let req_headers: std::collections::HashMap<String, String> = headers
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let resp_header_map: std::collections::HashMap<String, String> = resp_headers
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let base = state.config.upstream_url(&provider);
        let full_url = format!("{}{}", base, path);

        if status.is_success() {
            if let Ok(response_body) = serde_json::from_slice::<serde_json::Value>(&resp_bytes) {
                let model = extract_model(&request_body, &response_body);

                state
                    .recorder
                    .record_llm_call_full(
                        &model, &request_body, &response_body, duration_ms,
                        method.as_str(), &full_url, &req_headers,
                        status.as_u16(), &resp_header_map,
                    )
                    .await;

                // Extract tool_use blocks and record as separate ToolCall steps
                let parent_idx = state.recorder.step_count().await as u32 - 1;
                state
                    .recorder
                    .extract_and_record_tool_calls(&response_body, parent_idx)
                    .await;

                info!("Recorded LLM call: model={}, duration={:.0}ms", model, duration_ms);
            }
        } else {
            // Record API errors (429, 500, etc.) as error steps
            let response_body = serde_json::from_slice::<serde_json::Value>(&resp_bytes)
                .unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&resp_bytes).to_string()}));

            state
                .recorder
                .record_api_error(
                    status.as_u16(), method.as_str(), &full_url,
                    &request_body, &response_body, &req_headers,
                    &resp_header_map, duration_ms,
                )
                .await;

            info!("Recorded API error: status={}, duration={:.0}ms", status.as_u16(), duration_ms);
        }
    }

    // If client wanted streaming, convert the non-streaming response to SSE format
    let final_body = if is_streaming && status.is_success() && is_llm_endpoint(&path) {
        convert_to_sse(&resp_bytes, &provider)
    } else {
        resp_bytes.to_vec()
    };

    // Build response
    let mut response = Response::builder().status(status);
    for (name, value) in resp_headers.iter() {
        let name_str = name.as_str().to_lowercase();
        if name_str != "transfer-encoding"
            && name_str != "connection"
            && name_str != "content-length"
            && name_str != "content-type"
        {
            response = response.header(name, value);
        }
    }

    if is_streaming && status.is_success() && is_llm_endpoint(&path) {
        response = response.header("content-type", "text/event-stream");
        response = response.header("cache-control", "no-cache");
    } else {
        // Forward original content-type
        if let Some(ct) = resp_headers.get("content-type") {
            response = response.header("content-type", ct);
        }
    }

    Ok(response
        .body(Full::new(Bytes::from(final_body)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new()))))
}

/// Convert a non-streaming response to SSE format for clients that expect streaming.
fn convert_to_sse(resp_bytes: &Bytes, provider: &Provider) -> Vec<u8> {
    let mut output = Vec::new();

    match provider {
        Provider::OpenAI | Provider::Auto => {
            // OpenAI SSE format: data: {chunk}\n\ndata: [DONE]\n\n
            if let Ok(body) = serde_json::from_slice::<serde_json::Value>(resp_bytes) {
                // Convert to a streaming chunk format
                let chunk = convert_openai_to_chunk(&body);
                output.extend_from_slice(b"data: ");
                output.extend_from_slice(
                    serde_json::to_string(&chunk).unwrap_or_default().as_bytes(),
                );
                output.extend_from_slice(b"\n\n");
                output.extend_from_slice(b"data: [DONE]\n\n");
            }
        }
        Provider::Anthropic => {
            // Anthropic SSE: event: message_start, content_block_start, etc.
            if let Ok(body) = serde_json::from_slice::<serde_json::Value>(resp_bytes) {
                // message_start
                let start_event = serde_json::json!({
                    "type": "message_start",
                    "message": body
                });
                output.extend_from_slice(b"event: message_start\n");
                output.extend_from_slice(b"data: ");
                output.extend_from_slice(
                    serde_json::to_string(&start_event)
                        .unwrap_or_default()
                        .as_bytes(),
                );
                output.extend_from_slice(b"\n\n");

                // message_stop
                let stop_event = serde_json::json!({"type": "message_stop"});
                output.extend_from_slice(b"event: message_stop\n");
                output.extend_from_slice(b"data: ");
                output.extend_from_slice(
                    serde_json::to_string(&stop_event)
                        .unwrap_or_default()
                        .as_bytes(),
                );
                output.extend_from_slice(b"\n\n");
            }
        }
    }

    output
}

/// Convert an OpenAI chat completion response to a streaming chunk format.
fn convert_openai_to_chunk(body: &serde_json::Value) -> serde_json::Value {
    let choices = body.get("choices").and_then(|v| v.as_array());

    let chunk_choices: Vec<serde_json::Value> = choices
        .map(|cs| {
            cs.iter()
                .map(|c| {
                    let content = c
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let role = c
                        .get("message")
                        .and_then(|m| m.get("role"))
                        .cloned()
                        .unwrap_or(serde_json::json!("assistant"));

                    serde_json::json!({
                        "index": c.get("index").unwrap_or(&serde_json::json!(0)),
                        "delta": {
                            "role": role,
                            "content": content
                        },
                        "finish_reason": c.get("finish_reason")
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    serde_json::json!({
        "id": body.get("id").unwrap_or(&serde_json::json!("chatcmpl-agentreel")),
        "object": "chat.completion.chunk",
        "created": body.get("created").unwrap_or(&serde_json::json!(0)),
        "model": body.get("model").unwrap_or(&serde_json::json!("unknown")),
        "choices": chunk_choices,
        "usage": body.get("usage")
    })
}

fn is_llm_endpoint(path: &str) -> bool {
    path.contains("/chat/completions")
        || path.contains("/v1/messages")
        || path.contains("/completions")
}

fn extract_model(request: &serde_json::Value, response: &serde_json::Value) -> String {
    response
        .get("model")
        .or_else(|| request.get("model"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn error_response(status: u16, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({"error": message}).to_string();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_llm_endpoint() {
        assert!(is_llm_endpoint("/v1/chat/completions"));
        assert!(is_llm_endpoint("/v1/messages"));
        assert!(!is_llm_endpoint("/v1/models"));
        assert!(!is_llm_endpoint("/health"));
    }

    #[test]
    fn test_extract_model() {
        let req = serde_json::json!({"model": "gpt-4o"});
        let resp = serde_json::json!({"model": "gpt-4o-2024-01-01"});
        assert_eq!(extract_model(&req, &resp), "gpt-4o-2024-01-01");

        let empty_resp = serde_json::json!({});
        assert_eq!(extract_model(&req, &empty_resp), "gpt-4o");
    }

    #[test]
    fn test_detect_provider_anthropic() {
        let mut headers = hyper::HeaderMap::new();
        headers.insert("x-api-key", "test".parse().unwrap());
        assert_eq!(detect_provider("/v1/messages", &headers), Provider::Anthropic);
    }

    #[test]
    fn test_detect_provider_anthropic_version() {
        let mut headers = hyper::HeaderMap::new();
        headers.insert("anthropic-version", "2024-01-01".parse().unwrap());
        assert_eq!(detect_provider("/some/path", &headers), Provider::Anthropic);
    }

    #[test]
    fn test_detect_provider_openai() {
        let headers = hyper::HeaderMap::new();
        assert_eq!(
            detect_provider("/v1/chat/completions", &headers),
            Provider::OpenAI
        );
    }

    #[test]
    fn test_detect_provider_default() {
        let headers = hyper::HeaderMap::new();
        assert_eq!(detect_provider("/unknown", &headers), Provider::OpenAI);
    }

    #[test]
    fn test_convert_openai_to_chunk() {
        let body = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }]
        });

        let chunk = convert_openai_to_chunk(&body);
        assert_eq!(chunk["object"], "chat.completion.chunk");
        assert_eq!(chunk["choices"][0]["delta"]["content"], "Hello!");
    }

    #[test]
    fn test_convert_to_sse_openai() {
        let body = serde_json::json!({
            "choices": [{
                "message": {"role": "assistant", "content": "Hi"},
                "finish_reason": "stop"
            }]
        });
        let bytes = Bytes::from(serde_json::to_vec(&body).unwrap());
        let sse = convert_to_sse(&bytes, &Provider::OpenAI);
        let output = String::from_utf8(sse).unwrap();
        assert!(output.contains("data: "));
        assert!(output.contains("[DONE]"));
    }

    #[test]
    fn test_convert_to_sse_anthropic() {
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "Hi"}],
            "stop_reason": "end_turn"
        });
        let bytes = Bytes::from(serde_json::to_vec(&body).unwrap());
        let sse = convert_to_sse(&bytes, &Provider::Anthropic);
        let output = String::from_utf8(sse).unwrap();
        assert!(output.contains("event: message_start"));
        assert!(output.contains("event: message_stop"));
    }
}
