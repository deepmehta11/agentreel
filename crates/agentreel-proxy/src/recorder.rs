use agentreel_core::{
    ContentBlock, HttpExchange, LlmCall, LlmConfig, Message, MessageContent, Role, Step,
    StepType, TokenUsage, ToolCall, ToolDefinition, ToolStatus, Trajectory,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Records LLM API interactions into a Trajectory.
#[derive(Clone)]
pub struct Recorder {
    trajectory: Arc<Mutex<Trajectory>>,
}

impl Recorder {
    pub fn new(mut trajectory: Trajectory) -> Self {
        trajectory.metadata.created_at = Utc::now();
        Self {
            trajectory: Arc::new(Mutex::new(trajectory)),
        }
    }

    /// Record an LLM API call with full details.
    pub async fn record_llm_call_full(
        &self,
        model: &str,
        request_body: &serde_json::Value,
        response_body: &serde_json::Value,
        duration_ms: f64,
        method: &str,
        url: &str,
        request_headers: &HashMap<String, String>,
        status_code: u16,
        response_headers: &HashMap<String, String>,
    ) {
        let mut traj = self.trajectory.lock().await;
        let index = traj.steps.len() as u32;

        let messages = extract_messages(request_body);
        let system_prompt = extract_system_prompt(request_body);
        let response = extract_response(response_body);
        let response_blocks = extract_response_blocks(response_body);
        let tokens = extract_token_usage(response_body);
        let stop_reason = extract_stop_reason(response_body);
        let config = extract_config(request_body);
        let available_tools = extract_tool_definitions(request_body);
        let thinking = extract_thinking(response_body);
        let provider = detect_provider_from_url(url);

        // Redact auth headers before storing
        let safe_req_headers = redact_headers(request_headers);
        let safe_resp_headers = filter_interesting_headers(response_headers);

        let mut step = Step::new(index, StepType::LlmCall);
        step.duration_ms = Some(duration_ms);
        step.llm_call = Some(LlmCall {
                model: Some(model.to_string()),
                provider: Some(provider),
                messages,
                system_prompt,
                response: Some(response),
                response_blocks,
                stop_reason,
                config: Some(config),
                available_tools,
                http: Some(HttpExchange {
                    method: Some(method.to_string()),
                    url: Some(url.to_string()),
                    request_headers: safe_req_headers,
                    request_body: Some(request_body.clone()),
                    status_code: Some(status_code),
                    response_headers: safe_resp_headers,
                    response_body: Some(response_body.clone()),
                }),
                thinking,
            });
        step.tokens = tokens.clone();

        // Estimate cost
        if let Some(ref t) = tokens {
            let input = t.input_tokens.unwrap_or(0);
            let output = t.output_tokens.unwrap_or(0);
            let cost = crate::costs::estimate_cost(model, input, output);
            if cost > 0.0 {
                step.cost_usd = Some(cost);
            }
        }

        traj.steps.push(step);
    }

    /// Record an LLM API call (simplified — for backward compat).
    pub async fn record_llm_call(
        &self,
        model: &str,
        request_body: &serde_json::Value,
        response_body: &serde_json::Value,
        duration_ms: f64,
    ) {
        self.record_llm_call_full(
            model,
            request_body,
            response_body,
            duration_ms,
            "POST",
            "",
            &HashMap::new(),
            200,
            &HashMap::new(),
        )
        .await;
    }

    /// Record a tool call extracted from LLM response.
    pub async fn record_tool_call(
        &self,
        name: &str,
        input: serde_json::Value,
        output: serde_json::Value,
        status: ToolStatus,
        duration_ms: f64,
    ) {
        let mut traj = self.trajectory.lock().await;
        let index = traj.steps.len() as u32;

        let input_map = if let serde_json::Value::Object(m) = input {
            Some(m.into_iter().collect())
        } else {
            None
        };

        let mut step = Step::new(index, StepType::ToolCall);
        step.duration_ms = Some(duration_ms);
        step.tool_call = Some(ToolCall {
            name: name.to_string(),
            tool_type: None,
            input: input_map,
            output: Some(output),
            status: Some(status),
            mcp_server: None,
            mcp_server_name: None,
            screenshots: Vec::new(),
        });

        traj.steps.push(step);
    }

    /// Record an API error (429, 500, etc.) as an error step.
    pub async fn record_api_error(
        &self,
        status_code: u16,
        method: &str,
        url: &str,
        request_body: &serde_json::Value,
        response_body: &serde_json::Value,
        request_headers: &HashMap<String, String>,
        response_headers: &HashMap<String, String>,
        duration_ms: f64,
    ) {
        let mut traj = self.trajectory.lock().await;
        let index = traj.steps.len() as u32;

        let error_msg = response_body
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .or_else(|| response_body.get("message").and_then(|m| m.as_str()))
            .unwrap_or("API error")
            .to_string();

        let error_type = response_body
            .get("error")
            .and_then(|e| e.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let safe_req_headers = redact_headers(request_headers);
        let safe_resp_headers = filter_interesting_headers(response_headers);

        let mut step = Step::new(index, StepType::Error);
        step.duration_ms = Some(duration_ms);
        step.error = Some(agentreel_core::Error {
            code: Some(format!("HTTP {}", status_code)),
            message: Some(format!("{}: {}", error_type, error_msg)),
            recoverable: Some(status_code == 429 || status_code >= 500),
            stack_trace: None,
        });
        step.llm_call = Some(LlmCall {
            model: request_body.get("model").and_then(|m| m.as_str()).map(|s| s.to_string()),
            provider: Some(detect_provider_from_url(url)),
            messages: extract_messages(request_body),
            system_prompt: extract_system_prompt(request_body),
            response: None,
            response_blocks: Vec::new(),
            stop_reason: None,
            config: Some(extract_config(request_body)),
            available_tools: extract_tool_definitions(request_body),
            http: Some(HttpExchange {
                method: Some(method.to_string()),
                url: Some(url.to_string()),
                request_headers: safe_req_headers,
                request_body: Some(request_body.clone()),
                status_code: Some(status_code),
                response_headers: safe_resp_headers,
                response_body: Some(response_body.clone()),
            }),
            thinking: None,
        });

        traj.steps.push(step);
    }

    /// Extract tool_use blocks from an LLM response and record them as separate ToolCall steps.
    pub async fn extract_and_record_tool_calls(
        &self,
        response_body: &serde_json::Value,
        parent_step_id: u32,
    ) {
        // Anthropic: content blocks with type "tool_use"
        if let Some(content) = response_body.get("content").and_then(|v| v.as_array()) {
            for block in content {
                if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                    let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));
                    let tool_use_id = block.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());

                    let mut traj = self.trajectory.lock().await;
                    let index = traj.steps.len() as u32;
                    let mut step = Step::new(index, StepType::ToolCall);
                    step.parent_step_id = Some(parent_step_id);

                    let input_map = if let serde_json::Value::Object(m) = input {
                        Some(m.into_iter().collect())
                    } else {
                        None
                    };

                    step.tool_call = Some(agentreel_core::ToolCall {
                        name: name.to_string(),
                        tool_type: Some(agentreel_core::ToolType::Function),
                        input: input_map,
                        output: None,
                        status: Some(agentreel_core::ToolStatus::Success),
                        mcp_server: None,
                        mcp_server_name: None,
                        screenshots: Vec::new(),
                    });

                    traj.steps.push(step);
                }
            }
        }

        // OpenAI: choices[0].message.tool_calls
        if let Some(tool_calls) = response_body
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("tool_calls"))
            .and_then(|v| v.as_array())
        {
            for tc in tool_calls {
                let name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let input_map = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| serde_json::from_str::<HashMap<String, serde_json::Value>>(s).ok());

                let mut traj = self.trajectory.lock().await;
                let index = traj.steps.len() as u32;
                let mut step = Step::new(index, StepType::ToolCall);
                step.parent_step_id = Some(parent_step_id);
                step.tool_call = Some(agentreel_core::ToolCall {
                    name: name.to_string(),
                    tool_type: Some(agentreel_core::ToolType::Function),
                    input: input_map,
                    output: None,
                    status: Some(agentreel_core::ToolStatus::Success),
                    mcp_server: None,
                    mcp_server_name: None,
                    screenshots: Vec::new(),
                });

                traj.steps.push(step);
            }
        }
    }

    /// Finalize and return the trajectory.
    pub async fn finalize(self) -> Trajectory {
        let mut traj = self.trajectory.lock().await;
        traj.metadata.completed_at = Some(Utc::now());
        traj.compute_stats();
        traj.clone()
    }

    /// Get current step count.
    pub async fn step_count(&self) -> usize {
        self.trajectory.lock().await.steps.len()
    }
}

fn extract_messages(body: &serde_json::Value) -> Vec<Message> {
    let mut messages = Vec::new();

    if let Some(msgs) = body.get("messages").and_then(|v| v.as_array()) {
        for msg in msgs {
            let role = match msg.get("role").and_then(|v| v.as_str()) {
                Some("system") => Role::System,
                Some("user") => Role::User,
                Some("assistant") => Role::Assistant,
                Some("tool") => Role::Tool,
                _ => Role::User,
            };

            let content = msg.get("content").map(|v| {
                if let Some(s) = v.as_str() {
                    MessageContent::Text(s.to_string())
                } else {
                    MessageContent::Text(v.to_string())
                }
            });

            messages.push(Message { role, content });
        }
    }

    messages
}

/// Extract system prompt (Anthropic: top-level "system", OpenAI: first system message).
fn extract_system_prompt(body: &serde_json::Value) -> Option<String> {
    // Anthropic: top-level "system" field
    if let Some(s) = body.get("system").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    // Anthropic: system as array of blocks
    if let Some(blocks) = body.get("system").and_then(|v| v.as_array()) {
        let texts: Vec<&str> = blocks
            .iter()
            .filter_map(|b| {
                if b.get("type").and_then(|v| v.as_str()) == Some("text") {
                    b.get("text").and_then(|v| v.as_str())
                } else {
                    None
                }
            })
            .collect();
        if !texts.is_empty() {
            return Some(texts.join("\n"));
        }
    }
    // OpenAI: first message with role "system"
    if let Some(msgs) = body.get("messages").and_then(|v| v.as_array()) {
        for msg in msgs {
            if msg.get("role").and_then(|v| v.as_str()) == Some("system") {
                if let Some(s) = msg.get("content").and_then(|v| v.as_str()) {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

fn extract_response(body: &serde_json::Value) -> Message {
    // OpenAI format
    if let Some(choices) = body.get("choices").and_then(|v| v.as_array()) {
        if let Some(choice) = choices.first() {
            if let Some(msg) = choice.get("message") {
                let content = msg
                    .get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| MessageContent::Text(s.to_string()));
                return Message {
                    role: Role::Assistant,
                    content,
                };
            }
        }
    }

    // Anthropic format — extract text blocks only for the summary response
    if let Some(content) = body.get("content").and_then(|v| v.as_array()) {
        let text: Vec<String> = content
            .iter()
            .filter_map(|block| {
                if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                    block.get("text").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();

        return Message {
            role: Role::Assistant,
            content: if text.is_empty() {
                None
            } else {
                Some(MessageContent::Text(text.join("\n")))
            },
        };
    }

    Message {
        role: Role::Assistant,
        content: None,
    }
}

/// Extract ALL response content blocks — text, tool_use, thinking, images, etc.
fn extract_response_blocks(body: &serde_json::Value) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();

    // Anthropic: content array with typed blocks
    if let Some(content) = body.get("content").and_then(|v| v.as_array()) {
        for block in content {
            let block_type = block
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let mut cb = ContentBlock {
                block_type: block_type.clone(),
                text: None,
                media_type: None,
                data: None,
                url: None,
                tool_use_id: None,
                tool_name: None,
                input: None,
                output: None,
            };

            match block_type.as_str() {
                "text" => {
                    cb.text = block.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
                "thinking" => {
                    cb.text = block
                        .get("thinking")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                "tool_use" => {
                    cb.tool_use_id = block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    cb.tool_name = block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(input) = block.get("input").and_then(|v| v.as_object()) {
                        cb.input = Some(input.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
                    }
                }
                "tool_result" => {
                    cb.tool_use_id = block
                        .get("tool_use_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    cb.output = block.get("content").cloned();
                }
                "image" => {
                    if let Some(source) = block.get("source") {
                        cb.media_type = source
                            .get("media_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        cb.data = source
                            .get("data")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                    }
                }
                _ => {
                    // Store unknown blocks as JSON text
                    cb.text = Some(block.to_string());
                }
            }

            blocks.push(cb);
        }
    }

    // OpenAI: choices[0].message.tool_calls
    if let Some(tool_calls) = body
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("tool_calls"))
        .and_then(|v| v.as_array())
    {
        for tc in tool_calls {
            let mut cb = ContentBlock {
                block_type: "tool_use".to_string(),
                text: None,
                media_type: None,
                data: None,
                url: None,
                tool_use_id: tc.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                tool_name: tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                input: None,
                output: None,
            };

            // Parse function arguments
            if let Some(args_str) = tc
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(|v| v.as_str())
            {
                if let Ok(args) = serde_json::from_str::<HashMap<String, serde_json::Value>>(args_str)
                {
                    cb.input = Some(args);
                }
            }

            blocks.push(cb);
        }

        // Also add the text content if present
        if let Some(content) = body
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
        {
            if !content.is_empty() {
                blocks.insert(
                    0,
                    ContentBlock {
                        block_type: "text".to_string(),
                        text: Some(content.to_string()),
                        media_type: None,
                        data: None,
                        url: None,
                        tool_use_id: None,
                        tool_name: None,
                        input: None,
                        output: None,
                    },
                );
            }
        }
    }

    blocks
}

/// Extract thinking/reasoning content from response.
fn extract_thinking(body: &serde_json::Value) -> Option<String> {
    // Anthropic: content blocks with type "thinking"
    if let Some(content) = body.get("content").and_then(|v| v.as_array()) {
        let thinking: Vec<String> = content
            .iter()
            .filter_map(|block| {
                if block.get("type").and_then(|v| v.as_str()) == Some("thinking") {
                    block
                        .get("thinking")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();

        if !thinking.is_empty() {
            return Some(thinking.join("\n\n"));
        }
    }

    // OpenAI: reasoning_content or reasoning field
    if let Some(reasoning) = body
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("reasoning_content").or_else(|| m.get("reasoning")))
        .and_then(|v| v.as_str())
    {
        return Some(reasoning.to_string());
    }

    None
}

fn extract_token_usage(body: &serde_json::Value) -> Option<TokenUsage> {
    let usage = body.get("usage")?;

    Some(TokenUsage {
        input_tokens: usage
            .get("input_tokens")
            .or_else(|| usage.get("prompt_tokens"))
            .and_then(|v| v.as_u64()),
        output_tokens: usage
            .get("output_tokens")
            .or_else(|| usage.get("completion_tokens"))
            .and_then(|v| v.as_u64()),
        cache_read_tokens: usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64()),
        cache_write_tokens: usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64()),
        thinking_tokens: usage
            .get("thinking_tokens")
            .and_then(|v| v.as_u64()),
        tool_use_tokens: None,
    })
}

fn extract_stop_reason(body: &serde_json::Value) -> Option<agentreel_core::StopReason> {
    // Anthropic: stop_reason
    if let Some(reason) = body.get("stop_reason").and_then(|v| v.as_str()) {
        return match reason {
            "end_turn" => Some(agentreel_core::StopReason::EndTurn),
            "tool_use" => Some(agentreel_core::StopReason::ToolUse),
            "max_tokens" => Some(agentreel_core::StopReason::MaxTokens),
            "stop_sequence" => Some(agentreel_core::StopReason::StopSequence),
            _ => None,
        };
    }

    // OpenAI: choices[0].finish_reason
    if let Some(reason) = body
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("finish_reason"))
        .and_then(|v| v.as_str())
    {
        return match reason {
            "stop" => Some(agentreel_core::StopReason::EndTurn),
            "tool_calls" => Some(agentreel_core::StopReason::ToolUse),
            "length" => Some(agentreel_core::StopReason::MaxTokens),
            _ => None,
        };
    }

    None
}

/// Extract model config (temperature, max_tokens, etc.) from request.
fn extract_config(body: &serde_json::Value) -> LlmConfig {
    let mut extra = HashMap::new();

    // Capture any non-standard parameters
    if let Some(obj) = body.as_object() {
        for (key, value) in obj {
            match key.as_str() {
                "model" | "messages" | "system" | "tools" | "stream" | "temperature"
                | "top_p" | "top_k" | "max_tokens" | "stop" | "stop_sequences" => {}
                _ => {
                    extra.insert(key.clone(), value.clone());
                }
            }
        }
    }

    LlmConfig {
        temperature: body.get("temperature").and_then(|v| v.as_f64()),
        top_p: body.get("top_p").and_then(|v| v.as_f64()),
        top_k: body.get("top_k").and_then(|v| v.as_u64()).map(|v| v as u32),
        max_tokens: body
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        stop_sequences: body
            .get("stop")
            .or_else(|| body.get("stop_sequences"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            }),
        extra,
    }
}

/// Extract tool definitions from request.
fn extract_tool_definitions(body: &serde_json::Value) -> Vec<ToolDefinition> {
    let mut tools = Vec::new();

    if let Some(tool_array) = body.get("tools").and_then(|v| v.as_array()) {
        for tool in tool_array {
            // Anthropic format: {name, description, input_schema}
            if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
                tools.push(ToolDefinition {
                    name: name.to_string(),
                    description: tool
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    input_schema: tool.get("input_schema").cloned(),
                });
            }
            // OpenAI format: {type: "function", function: {name, description, parameters}}
            else if let Some(func) = tool.get("function") {
                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                    tools.push(ToolDefinition {
                        name: name.to_string(),
                        description: func
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        input_schema: func.get("parameters").cloned(),
                    });
                }
            }
        }
    }

    tools
}

fn detect_provider_from_url(url: &str) -> String {
    if url.contains("anthropic") {
        "anthropic".to_string()
    } else if url.contains("openai") {
        "openai".to_string()
    } else if url.contains("googleapis") || url.contains("generativelanguage") {
        "google".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Redact sensitive headers (API keys, auth tokens).
fn redact_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    headers
        .iter()
        .map(|(k, v)| {
            let lower = k.to_lowercase();
            if lower == "authorization"
                || lower == "x-api-key"
                || lower == "api-key"
                || lower.contains("secret")
                || lower.contains("token")
            {
                (k.clone(), "[REDACTED]".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

/// Keep only interesting response headers (rate limits, request ID, etc.)
fn filter_interesting_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    headers
        .iter()
        .filter(|(k, _)| {
            let lower = k.to_lowercase();
            lower.contains("ratelimit")
                || lower.contains("rate-limit")
                || lower.contains("request-id")
                || lower.contains("x-request-id")
                || lower == "retry-after"
                || lower.contains("anthropic")
                || lower.contains("openai")
                || lower.contains("x-ratelimit")
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recorder_basic() {
        let traj = Trajectory::new();
        let recorder = Recorder::new(traj);

        let request = serde_json::json!({
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let response = serde_json::json!({
            "choices": [{
                "message": {"role": "assistant", "content": "Hi there!"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5
            }
        });

        recorder.record_llm_call("gpt-4o", &request, &response, 500.0).await;
        assert_eq!(recorder.step_count().await, 1);

        let result = recorder.finalize().await;
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].step_type, StepType::LlmCall);
        assert!(result.stats.is_some());
    }

    #[tokio::test]
    async fn test_recorder_anthropic_format() {
        let traj = Trajectory::new();
        let recorder = Recorder::new(traj);

        let request = serde_json::json!({
            "model": "claude-opus-4-6",
            "system": "You are helpful.",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.7,
            "max_tokens": 1024
        });

        let response = serde_json::json!({
            "content": [
                {"type": "text", "text": "Hi there!"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        recorder.record_llm_call("claude-opus-4-6", &request, &response, 800.0).await;

        let result = recorder.finalize().await;
        let step = &result.steps[0];
        let llm_call = step.llm_call.as_ref().unwrap();
        assert_eq!(llm_call.model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(llm_call.system_prompt.as_deref(), Some("You are helpful."));
        assert_eq!(llm_call.config.as_ref().unwrap().temperature, Some(0.7));
        assert_eq!(llm_call.config.as_ref().unwrap().max_tokens, Some(1024));

        if let Some(MessageContent::Text(t)) = &llm_call.response.as_ref().unwrap().content {
            assert_eq!(t, "Hi there!");
        } else {
            panic!("Expected text response");
        }
    }

    #[tokio::test]
    async fn test_recorder_thinking() {
        let traj = Trajectory::new();
        let recorder = Recorder::new(traj);

        let request = serde_json::json!({
            "model": "claude-opus-4-6",
            "messages": [{"role": "user", "content": "Think about this"}]
        });

        let response = serde_json::json!({
            "content": [
                {"type": "thinking", "thinking": "Let me reason about this step by step..."},
                {"type": "text", "text": "Here's my answer."}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 50,
                "thinking_tokens": 200
            }
        });

        recorder.record_llm_call("claude-opus-4-6", &request, &response, 2000.0).await;

        let result = recorder.finalize().await;
        let llm_call = result.steps[0].llm_call.as_ref().unwrap();

        // Thinking captured
        assert_eq!(
            llm_call.thinking.as_deref(),
            Some("Let me reason about this step by step...")
        );

        // Response blocks capture all content types
        assert_eq!(llm_call.response_blocks.len(), 2);
        assert_eq!(llm_call.response_blocks[0].block_type, "thinking");
        assert_eq!(llm_call.response_blocks[1].block_type, "text");

        // Thinking tokens captured
        let tokens = result.steps[0].tokens.as_ref().unwrap();
        assert_eq!(tokens.thinking_tokens, Some(200));
    }

    #[tokio::test]
    async fn test_recorder_tool_use_response() {
        let traj = Trajectory::new();
        let recorder = Recorder::new(traj);

        let request = serde_json::json!({
            "model": "claude-opus-4-6",
            "messages": [{"role": "user", "content": "Read the file"}],
            "tools": [
                {
                    "name": "read_file",
                    "description": "Read a file from disk",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"}
                        }
                    }
                },
                {
                    "name": "write_file",
                    "description": "Write a file to disk",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "content": {"type": "string"}
                        }
                    }
                }
            ]
        });

        let response = serde_json::json!({
            "content": [
                {"type": "text", "text": "I'll read the file."},
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "read_file",
                    "input": {"path": "/tmp/test.txt"}
                }
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 100, "output_tokens": 30}
        });

        recorder.record_llm_call("claude-opus-4-6", &request, &response, 1000.0).await;

        let result = recorder.finalize().await;
        let llm_call = result.steps[0].llm_call.as_ref().unwrap();

        // Tool definitions captured
        assert_eq!(llm_call.available_tools.len(), 2);
        assert_eq!(llm_call.available_tools[0].name, "read_file");
        assert_eq!(llm_call.available_tools[1].name, "write_file");

        // Response blocks capture text + tool_use
        assert_eq!(llm_call.response_blocks.len(), 2);
        assert_eq!(llm_call.response_blocks[0].block_type, "text");
        assert_eq!(llm_call.response_blocks[1].block_type, "tool_use");
        assert_eq!(
            llm_call.response_blocks[1].tool_name.as_deref(),
            Some("read_file")
        );
        assert_eq!(
            llm_call.response_blocks[1].tool_use_id.as_deref(),
            Some("toolu_123")
        );

        // Stop reason
        assert_eq!(llm_call.stop_reason, Some(agentreel_core::StopReason::ToolUse));
    }

    #[tokio::test]
    async fn test_recorder_full_with_headers() {
        let traj = Trajectory::new();
        let recorder = Recorder::new(traj);

        let mut req_headers = HashMap::new();
        req_headers.insert("x-api-key".to_string(), "sk-secret-key".to_string());
        req_headers.insert("content-type".to_string(), "application/json".to_string());
        req_headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());

        let mut resp_headers = HashMap::new();
        resp_headers.insert("x-request-id".to_string(), "req_abc123".to_string());
        resp_headers.insert(
            "x-ratelimit-remaining".to_string(),
            "99".to_string(),
        );
        resp_headers.insert("content-type".to_string(), "application/json".to_string());

        let request = serde_json::json!({"model": "claude-opus-4-6", "messages": [{"role": "user", "content": "Hi"}], "max_tokens": 100});
        let response = serde_json::json!({"content": [{"type": "text", "text": "Hello!"}], "stop_reason": "end_turn", "usage": {"input_tokens": 5, "output_tokens": 3}});

        recorder
            .record_llm_call_full(
                "claude-opus-4-6",
                &request,
                &response,
                500.0,
                "POST",
                "https://api.anthropic.com/v1/messages",
                &req_headers,
                200,
                &resp_headers,
            )
            .await;

        let result = recorder.finalize().await;
        let http = result.steps[0]
            .llm_call
            .as_ref()
            .unwrap()
            .http
            .as_ref()
            .unwrap();

        // API key redacted
        assert_eq!(http.request_headers.get("x-api-key").unwrap(), "[REDACTED]");
        // Non-sensitive headers preserved
        assert_eq!(
            http.request_headers.get("anthropic-version").unwrap(),
            "2023-06-01"
        );
        // Only interesting response headers kept
        assert!(http.response_headers.contains_key("x-request-id"));
        assert!(http.response_headers.contains_key("x-ratelimit-remaining"));
        assert!(!http.response_headers.contains_key("content-type"));
        // Raw bodies preserved
        assert!(http.request_body.is_some());
        assert!(http.response_body.is_some());
        assert_eq!(http.status_code, Some(200));
        assert_eq!(http.url.as_deref(), Some("https://api.anthropic.com/v1/messages"));
    }
}
