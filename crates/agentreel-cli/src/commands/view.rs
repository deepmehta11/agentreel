use anyhow::Result;
use agentreel_core::{Trajectory, StepType};
use std::path::PathBuf;

pub fn run(path: PathBuf, full: bool) -> Result<()> {
    let content = std::fs::read_to_string(&path)?;
    let trajectory = Trajectory::from_json(&content)?;

    // Header
    println!("╔══════════════════════════════════════════════════════════════╗");
    if let Some(ref title) = trajectory.metadata.title {
        println!("║  {:<58} ║", title);
    }
    println!("║  ID: {:<55} ║", trajectory.id);
    if let Some(ref model) = trajectory.metadata.model {
        if let Some(ref model_id) = model.model_id {
            let provider = model.provider.as_deref().unwrap_or("unknown");
            println!("║  Model: {}/{:<49} ║", provider, model_id);
        }
    }
    println!("║  Steps: {:<52} ║", trajectory.steps.len());
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Steps
    for step in &trajectory.steps {
        let icon = match step.step_type {
            StepType::LlmCall => "🧠",
            StepType::ToolCall => "🔧",
            StepType::ToolResult => "📥",
            StepType::HumanInput => "👤",
            StepType::Error => "❌",
            StepType::Retry => "🔄",
            StepType::Thought => "💭",
            StepType::AgentDecision => "🔀",
            StepType::FileOperation => "📁",
            StepType::Screenshot => "📸",
            StepType::NetworkRequest => "🌐",
            StepType::Handoff => "🤝",
            StepType::Checkpoint => "📌",
        };

        let duration = step.duration_ms
            .map(|d| format!("{:.0}ms", d))
            .unwrap_or_default();

        let tokens_str = step.tokens.as_ref()
            .map(|t| {
                let input = t.input_tokens.unwrap_or(0);
                let output = t.output_tokens.unwrap_or(0);
                let mut s = format!("{}in/{}out", input, output);
                if let Some(thinking) = t.thinking_tokens {
                    if thinking > 0 {
                        s.push_str(&format!("/{}think", thinking));
                    }
                }
                if let Some(cache) = t.cache_read_tokens {
                    if cache > 0 {
                        s.push_str(&format!("/{}cache", cache));
                    }
                }
                s
            })
            .unwrap_or_default();

        match step.step_type {
            StepType::LlmCall => {
                let call = step.llm_call.as_ref();
                let model = call.and_then(|c| c.model.as_deref()).unwrap_or("?");
                let provider = call.and_then(|c| c.provider.as_deref()).unwrap_or("");

                let model_display = if provider.is_empty() {
                    model.to_string()
                } else {
                    format!("{}/{}", provider, model)
                };

                println!("  {} Step {} — LLM call ({}) [{}] [{}]",
                    icon, step.index, model_display, duration, tokens_str);

                if full {
                    if let Some(ref call) = step.llm_call {
                        // Show system prompt
                        if let Some(ref sys) = call.system_prompt {
                            println!("     ├─ system: {}", truncate(sys, 150));
                        }

                        // Show config
                        if let Some(ref config) = call.config {
                            let mut params = Vec::new();
                            if let Some(t) = config.temperature {
                                params.push(format!("temp={}", t));
                            }
                            if let Some(m) = config.max_tokens {
                                params.push(format!("max_tokens={}", m));
                            }
                            if let Some(p) = config.top_p {
                                params.push(format!("top_p={}", p));
                            }
                            if !params.is_empty() {
                                println!("     ├─ config: {}", params.join(", "));
                            }
                        }

                        // Show available tools
                        if !call.available_tools.is_empty() {
                            let tool_names: Vec<&str> = call.available_tools.iter()
                                .map(|t| t.name.as_str())
                                .collect();
                            println!("     ├─ tools: [{}]", tool_names.join(", "));
                        }

                        // Show input messages
                        for msg in &call.messages {
                            let role = format!("{:?}", msg.role).to_lowercase();
                            if let Some(ref content) = msg.content {
                                match content {
                                    agentreel_core::MessageContent::Text(t) => {
                                        println!("     ├─ {}: {}", role, truncate(t, 150));
                                    }
                                    agentreel_core::MessageContent::Blocks(blocks) => {
                                        for block in blocks {
                                            if let Some(ref t) = block.text {
                                                println!("     ├─ {}: {}", role, truncate(t, 150));
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Show thinking
                        if let Some(ref thinking) = call.thinking {
                            println!("     ├─ 💭 thinking: {}", truncate(thinking, 200));
                        }

                        // Show response content blocks
                        if !call.response_blocks.is_empty() {
                            for block in &call.response_blocks {
                                match block.block_type.as_str() {
                                    "text" => {
                                        if let Some(ref t) = block.text {
                                            println!("     ├─ 📝 text: {}", truncate(t, 200));
                                        }
                                    }
                                    "thinking" => {
                                        if let Some(ref t) = block.text {
                                            println!("     ├─ 💭 thinking: {}", truncate(t, 200));
                                        }
                                    }
                                    "tool_use" => {
                                        let name = block.tool_name.as_deref().unwrap_or("?");
                                        let id = block.tool_use_id.as_deref().unwrap_or("");
                                        let input_str = block.input.as_ref()
                                            .map(|i| serde_json::to_string(i).unwrap_or_default())
                                            .unwrap_or_default();
                                        println!("     ├─ 🔧 tool_use: {}({}) [{}]",
                                            name, truncate(&input_str, 100), id);
                                    }
                                    other => {
                                        println!("     ├─ [{}]", other);
                                    }
                                }
                            }
                        } else if let Some(ref resp) = call.response {
                            // Fallback to simple response
                            if let Some(ref content) = resp.content {
                                match content {
                                    agentreel_core::MessageContent::Text(t) => {
                                        println!("     └─ {}", truncate(t, 200));
                                    }
                                    agentreel_core::MessageContent::Blocks(blocks) => {
                                        for block in blocks {
                                            if let Some(ref t) = block.text {
                                                println!("     └─ {}", truncate(t, 200));
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Show HTTP info
                        if let Some(ref http) = call.http {
                            if let Some(ref url) = http.url {
                                if !url.is_empty() {
                                    let status = http.status_code.unwrap_or(0);
                                    println!("     ├─ 🌐 {} {} → {}",
                                        http.method.as_deref().unwrap_or("?"), url, status);
                                }
                            }
                            if !http.response_headers.is_empty() {
                                for (k, v) in &http.response_headers {
                                    println!("     │  {}: {}", k, v);
                                }
                            }
                        }

                        // Stop reason
                        if let Some(ref reason) = call.stop_reason {
                            println!("     └─ stop: {:?}", reason);
                        }
                    }
                }
            }
            StepType::ToolCall => {
                let name = step.tool_call.as_ref()
                    .map(|c| c.name.as_str())
                    .unwrap_or("?");
                let status = step.tool_call.as_ref()
                    .and_then(|c| c.status.as_ref())
                    .map(|s| format!("{:?}", s).to_lowercase())
                    .unwrap_or_default();
                println!("  {} Step {} — {} [{}] {}", icon, step.index, name, duration, status);

                if full {
                    if let Some(ref tc) = step.tool_call {
                        if let Some(ref input) = tc.input {
                            let input_str = serde_json::to_string(input).unwrap_or_default();
                            println!("     ├─ input: {}", truncate(&input_str, 200));
                        }
                        if let Some(ref output) = tc.output {
                            let output_str = if let Some(s) = output.as_str() {
                                s.to_string()
                            } else {
                                serde_json::to_string(output).unwrap_or_default()
                            };
                            println!("     └─ output: {}", truncate(&output_str, 200));
                        }
                    }
                }
            }
            StepType::HumanInput => {
                let action = step.human_input.as_ref()
                    .and_then(|h| h.action.as_ref())
                    .map(|a| format!("{:?}", a).to_lowercase())
                    .unwrap_or("input".to_string());
                println!("  {} Step {} — human {} [{}]", icon, step.index, action, duration);

                if full {
                    if let Some(ref hi) = step.human_input {
                        if let Some(ref content) = hi.content {
                            println!("     └─ {}", truncate(content, 200));
                        }
                    }
                }
            }
            StepType::Error => {
                let msg = step.error.as_ref()
                    .and_then(|e| e.message.as_deref())
                    .unwrap_or("unknown error");
                println!("  {} Step {} — error: {}", icon, step.index, msg);
            }
            StepType::ToolResult => {
                let name = step.tool_result.as_ref()
                    .map(|r| r.tool_name.as_str())
                    .unwrap_or("?");
                let is_error = step.tool_result.as_ref()
                    .and_then(|r| r.is_error)
                    .unwrap_or(false);
                let status = if is_error { "error" } else { "ok" };
                println!("  {} Step {} — result: {} [{}] {}", icon, step.index, name, duration, status);

                if full {
                    if let Some(ref tr) = step.tool_result {
                        if let Some(ref output) = tr.output {
                            let output_str = if let Some(s) = output.as_str() {
                                s.to_string()
                            } else {
                                serde_json::to_string(output).unwrap_or_default()
                            };
                            println!("     └─ {}", truncate(&output_str, 200));
                        }
                    }
                }
            }
            StepType::Thought => {
                let content = step.thought.as_ref()
                    .map(|t| t.content.as_str())
                    .unwrap_or("...");
                println!("  {} Step {} — {}", icon, step.index, truncate(content, 120));
            }
            StepType::AgentDecision => {
                let decision = step.agent_decision.as_ref()
                    .map(|d| d.decision.as_str())
                    .unwrap_or("...");
                println!("  {} Step {} — decision: {}", icon, step.index, truncate(decision, 120));

                if full {
                    if let Some(ref ad) = step.agent_decision {
                        if let Some(ref reasoning) = ad.reasoning {
                            println!("     ├─ reasoning: {}", truncate(reasoning, 150));
                        }
                        if !ad.alternatives_considered.is_empty() {
                            println!("     └─ alternatives: {}", ad.alternatives_considered.join(", "));
                        }
                    }
                }
            }
            StepType::FileOperation => {
                let op = step.file_operation.as_ref()
                    .map(|f| format!("{:?} {}", f.operation, f.path))
                    .unwrap_or_else(|| "?".to_string());
                println!("  {} Step {} — file: {} [{}]", icon, step.index, op, duration);
            }
            StepType::Retry => {
                println!("  {} Step {} — retry [{}]", icon, step.index, duration);
            }
            StepType::Screenshot => {
                println!("  {} Step {} — screenshot [{}]", icon, step.index, duration);
            }
            StepType::NetworkRequest => {
                println!("  {} Step {} — network request [{}]", icon, step.index, duration);
            }
            StepType::Handoff => {
                println!("  {} Step {} — handoff [{}]", icon, step.index, duration);
            }
            StepType::Checkpoint => {
                println!("  {} Step {} — checkpoint", icon, step.index);
            }
        }
    }

    // Outcome
    if let Some(ref outcome) = trajectory.outcome {
        println!();
        if let Some(ref status) = outcome.status {
            let icon = match status {
                agentreel_core::OutcomeStatus::Success => "✅",
                agentreel_core::OutcomeStatus::Failure => "❌",
                agentreel_core::OutcomeStatus::Partial => "⚠️",
                agentreel_core::OutcomeStatus::Aborted => "🛑",
            };
            println!("  {} Outcome: {:?}", icon, status);
        }
        if let Some(ref summary) = outcome.summary {
            println!("     {}", summary);
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.replace('\n', " ")
    } else {
        format!("{}...", &s[..max].replace('\n', " "))
    }
}
