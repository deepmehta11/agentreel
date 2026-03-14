use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A complete recording of an AI agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub version: String,
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub metadata: Metadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models_used: Vec<ModelUsed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Input>,
    pub steps: Vec<Step>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<Stats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Annotation>,
}

impl Trajectory {
    pub fn new() -> Self {
        Self {
            version: "0.1.0".to_string(),
            id: Uuid::new_v4(),
            parent_id: None,
            metadata: Metadata::new(),
            models_used: Vec::new(),
            input: None,
            steps: Vec::new(),
            outcome: None,
            stats: None,
            annotations: Vec::new(),
        }
    }

    /// Compute stats from the steps.
    pub fn compute_stats(&mut self) {
        let total_steps = self.steps.len() as u32;
        let mut total_llm_calls = 0u32;
        let mut total_tool_calls = 0u32;
        let mut total_input_tokens = 0u64;
        let mut total_output_tokens = 0u64;
        let mut total_cost_usd = 0.0f64;
        let mut total_duration_ms = 0.0f64;
        let mut errors_count = 0u32;

        for step in &self.steps {
            if let Some(ms) = step.duration_ms {
                total_duration_ms += ms;
            }
            if let Some(cost) = step.cost_usd {
                total_cost_usd += cost;
            }
            if let Some(ref tokens) = step.tokens {
                total_input_tokens += tokens.input_tokens.unwrap_or(0);
                total_output_tokens += tokens.output_tokens.unwrap_or(0);
            }
            match step.step_type {
                StepType::LlmCall => total_llm_calls += 1,
                StepType::ToolCall | StepType::ToolResult => total_tool_calls += 1,
                StepType::Error => errors_count += 1,
                _ => {}
            }
        }

        self.stats = Some(Stats {
            total_steps,
            total_llm_calls,
            total_tool_calls,
            total_tokens: Some(TokenUsage {
                input_tokens: Some(total_input_tokens),
                output_tokens: Some(total_output_tokens),
                cache_read_tokens: None,
                cache_write_tokens: None,
                thinking_tokens: None,
                tool_use_tokens: None,
            }),
            total_cost_usd: Some(total_cost_usd),
            total_duration_ms: Some(total_duration_ms),
            errors_count,
            retries_count: 0,
        });
    }

    /// Add a step and return its index.
    pub fn add_step(&mut self, step: Step) -> usize {
        let idx = self.steps.len();
        self.steps.push(step);
        idx
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Fork this trajectory — creates a new one with a parent_id link.
    pub fn fork(&self) -> Self {
        let mut forked = self.clone();
        forked.parent_id = Some(self.id);
        forked.id = Uuid::new_v4();
        forked.metadata.created_at = Utc::now();
        forked.metadata.completed_at = None;
        forked.annotations = Vec::new();
        forked
    }
}

impl Default for Trajectory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            created_at: Utc::now(),
            completed_at: None,
            agent: None,
            model: None,
            environment: None,
            tags: Vec::new(),
            title: None,
            description: None,
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}

/// A model that was invoked during this trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsed {
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub index: u32,
    #[serde(rename = "type")]
    pub step_type: StepType,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    /// Parent step index (e.g., tool_result links to its tool_call)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_step_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_call: Option<LlmCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub human_input: Option<HumanInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<Thought>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_decision: Option<AgentDecision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_operation: Option<FileOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// Per-step annotations (comments, ratings, flags)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_annotations: Option<StepAnnotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    LlmCall,
    ToolCall,
    ToolResult,
    HumanInput,
    Error,
    Retry,
    Thought,
    AgentDecision,
    FileOperation,
    Screenshot,
    NetworkRequest,
    Handoff,
    Checkpoint,
}

/// Agent's internal reasoning / chain-of-thought.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_tokens: Option<u64>,
}

/// When the agent explicitly chose between options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDecision {
    pub decision: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives_considered: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

/// Result of a tool call (separate step linked via parent_step_id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<ToolType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
}

/// File system operation performed by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub operation: FileOp,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOp {
    Read,
    Write,
    Create,
    Delete,
    Move,
    Copy,
}

impl Step {
    /// Create a new step with only the required fields, rest defaults to None.
    pub fn new(index: u32, step_type: StepType) -> Self {
        Self {
            index,
            step_type,
            timestamp: Utc::now(),
            duration_ms: None,
            parent_step_id: None,
            llm_call: None,
            tool_call: None,
            tool_result: None,
            human_input: None,
            error: None,
            thought: None,
            agent_decision: None,
            file_operation: None,
            tokens: None,
            cost_usd: None,
            step_annotations: None,
        }
    }
}

/// Per-step annotation (inline, not the top-level annotations array).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flagged: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Message>,
    /// Full response content blocks (text, tool_use, thinking, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub response_blocks: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// Model parameters used for this call (temperature, top_p, max_tokens, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<LlmConfig>,
    /// Tool definitions that were available to the model
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_tools: Vec<ToolDefinition>,
    /// Raw HTTP request/response for full transparency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpExchange>,
    /// Internal reasoning/thinking from the model (Claude extended thinking, CoT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// Model configuration parameters for an LLM call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Any extra parameters (frequency_penalty, presence_penalty, etc.)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Tool definition that was available to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// Raw HTTP request/response capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpExchange {
    /// HTTP method (POST, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Request URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Request headers (redacted)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub request_headers: HashMap<String, String>,
    /// Raw request body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<serde_json::Value>,
    /// HTTP status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Response headers (rate limits, request ID, etc.)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub response_headers: HashMap<String, String>,
    /// Raw response body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<ToolType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolStatus>,
    /// MCP server URL if this is an MCP tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server: Option<String>,
    /// MCP server name (e.g., "mcp360", "slack", "github")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<Screenshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Function,
    Mcp,
    ComputerUse,
    CodeExecution,
    WebSearch,
    FileSystem,
    Api,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    Timeout,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<HumanAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanAction {
    Approve,
    Deny,
    Edit,
    Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recoverable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    /// Tokens used for internal reasoning/thinking (Claude extended thinking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_tokens: Option<u64>,
    /// Tokens used for tool-use content blocks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default = "default_media_type")]
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

fn default_media_type() -> String {
    "image/png".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OutcomeStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_changed: Vec<FileDiff>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
    Success,
    Failure,
    Partial,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub action: FileAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    #[serde(rename = "type")]
    pub artifact_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_steps: u32,
    pub total_llm_calls: u32,
    pub total_tool_calls: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration_ms: Option<f64>,
    pub errors_count: u32,
    pub retries_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    #[serde(rename = "type")]
    pub annotation_type: AnnotationType,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationType {
    Comment,
    Rating,
    Label,
    Flag,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_trajectory() {
        let t = Trajectory::new();
        assert_eq!(t.version, "0.1.0");
        assert!(t.steps.is_empty());
        assert!(t.parent_id.is_none());
    }

    #[test]
    fn test_fork_trajectory() {
        let original = Trajectory::new();
        let original_id = original.id;
        let forked = original.fork();

        assert_ne!(forked.id, original_id);
        assert_eq!(forked.parent_id, Some(original_id));
        assert_eq!(forked.version, "0.1.0");
    }

    #[test]
    fn test_serialize_roundtrip() {
        let mut t = Trajectory::new();
        t.metadata.title = Some("Test run".to_string());
        t.input = Some(Input {
            prompt: Some("Hello".to_string()),
            system_prompt: None,
            files: Vec::new(),
            context: None,
        });

        let json = t.to_json().unwrap();
        let parsed = Trajectory::from_json(&json).unwrap();

        assert_eq!(parsed.id, t.id);
        assert_eq!(parsed.metadata.title, Some("Test run".to_string()));
    }

    #[test]
    fn test_compute_stats() {
        let mut t = Trajectory::new();
        let mut s0 = Step::new(0, StepType::LlmCall);
        s0.duration_ms = Some(1000.0);
        s0.tokens = Some(TokenUsage {
            input_tokens: Some(100),
            output_tokens: Some(50),
            ..Default::default()
        });
        s0.cost_usd = Some(0.01);
        t.steps.push(s0);

        let mut s1 = Step::new(1, StepType::ToolCall);
        s1.duration_ms = Some(500.0);
        t.steps.push(s1);

        t.compute_stats();
        let stats = t.stats.as_ref().unwrap();
        assert_eq!(stats.total_steps, 2);
        assert_eq!(stats.total_llm_calls, 1);
        assert_eq!(stats.total_tool_calls, 1);
        assert_eq!(stats.total_duration_ms, Some(1500.0));
    }

    #[test]
    fn test_parse_example_file() {
        let example = include_str!("../../../spec/example.trajectory.json");
        let t = Trajectory::from_json(example).unwrap();
        assert_eq!(t.version, "0.1.0");
        assert_eq!(t.steps.len(), 4);
        assert_eq!(t.outcome.as_ref().unwrap().status.as_ref().unwrap(), &OutcomeStatus::Success);
    }
}
