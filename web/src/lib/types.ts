export interface Trajectory {
  version: string;
  id: string;
  parent_id?: string;
  metadata: {
    created_at: string;
    completed_at?: string;
    agent?: { name?: string; version?: string };
    model?: { provider?: string; model_id?: string };
    environment?: { os?: string; arch?: string; runtime?: string };
    tags?: string[];
    title?: string;
    description?: string;
  };
  models_used?: Array<{ provider: string; model: string; api_base?: string }>;
  input?: {
    prompt?: string;
    system_prompt?: string;
  };
  steps: Step[];
  outcome?: {
    status?: "success" | "failure" | "partial" | "aborted";
    summary?: string;
    files_changed?: Array<{ path: string; action: string; diff?: string }>;
  };
  stats?: {
    total_steps: number;
    total_llm_calls: number;
    total_tool_calls: number;
    total_tokens?: { input_tokens?: number; output_tokens?: number };
    total_cost_usd?: number;
    total_duration_ms?: number;
    errors_count: number;
  };
}

export interface Step {
  index: number;
  type: "llm_call" | "tool_call" | "tool_result" | "human_input" | "error" | "thought" | "agent_decision" | "file_operation" | "retry" | "screenshot" | "checkpoint";
  timestamp: string;
  duration_ms?: number;
  parent_step_id?: number;
  llm_call?: LlmCallData;
  tool_call?: {
    name: string;
    tool_type?: string;
    input?: Record<string, unknown>;
    output?: unknown;
    status?: string;
    mcp_server?: string;
    mcp_server_name?: string;
  };
  tool_result?: {
    tool_name: string;
    output?: unknown;
    is_error?: boolean;
    error_message?: string;
  };
  human_input?: { content?: string; action?: string };
  error?: { code?: string; message?: string; recoverable?: boolean; stack_trace?: string };
  thought?: { content: string; thinking_tokens?: number };
  agent_decision?: {
    decision: string;
    alternatives_considered?: string[];
    reasoning?: string;
    confidence?: number;
  };
  file_operation?: {
    operation: string;
    path: string;
    content_preview?: string;
    size_bytes?: number;
  };
  tokens?: { input_tokens?: number; output_tokens?: number; thinking_tokens?: number; cache_read_tokens?: number };
  cost_usd?: number;
}

export interface LlmCallData {
  model?: string;
  provider?: string;
  messages?: Array<{ role: string; content?: string }>;
  system_prompt?: string;
  response?: { role: string; content?: string };
  response_blocks?: Array<{
    type: string;
    text?: string;
    tool_use_id?: string;
    tool_name?: string;
    input?: Record<string, unknown>;
  }>;
  stop_reason?: string;
  thinking?: string;
  config?: {
    temperature?: number;
    top_p?: number;
    max_tokens?: number;
    [key: string]: unknown;
  };
  available_tools?: Array<{ name: string; description?: string }>;
  http?: {
    method?: string;
    url?: string;
    request_headers?: Record<string, string>;
    request_body?: unknown;
    status_code?: number;
    response_headers?: Record<string, string>;
    response_body?: unknown;
  };
}
