export interface AgentInfo {
  name?: string;
  version?: string;
  url?: string;
}

export interface ModelInfo {
  provider?: string;
  model_id?: string;
  parameters?: Record<string, unknown>;
}

export interface EnvironmentInfo {
  os?: string;
  arch?: string;
  runtime?: string;
  [key: string]: unknown;
}

export interface TokenUsage {
  input_tokens?: number;
  output_tokens?: number;
  cache_read_tokens?: number;
  cache_write_tokens?: number;
}

export interface ContentBlock {
  type: string;
  text?: string;
  media_type?: string;
  data?: string;
  url?: string;
  tool_use_id?: string;
  tool_name?: string;
  input?: Record<string, unknown>;
  output?: unknown;
}

export interface Message {
  role: "system" | "user" | "assistant" | "tool";
  content?: string | ContentBlock[];
}

export interface LlmCall {
  model?: string;
  messages?: Message[];
  response?: Message;
  stop_reason?: "end_turn" | "tool_use" | "max_tokens" | "stop_sequence";
}

export interface ToolCall {
  name: string;
  input?: Record<string, unknown>;
  output?: unknown;
  status?: "success" | "error" | "timeout" | "denied";
  screenshots?: Screenshot[];
}

export interface HumanInput {
  content?: string;
  action?: "approve" | "deny" | "edit" | "message";
}

export interface Error {
  code?: string;
  message?: string;
  recoverable?: boolean;
  stack_trace?: string;
}

export interface Screenshot {
  timestamp?: string;
  media_type?: string;
  data?: string;
  url?: string;
  label?: string;
}

export interface FileSnapshot {
  path: string;
  content?: string;
  hash?: string;
  language?: string;
}

export interface Step {
  index: number;
  type: "llm_call" | "tool_call" | "human_input" | "error" | "checkpoint";
  timestamp: string;
  duration_ms?: number;
  llm_call?: LlmCall;
  tool_call?: ToolCall;
  human_input?: HumanInput;
  error?: Error;
  tokens?: TokenUsage;
  cost_usd?: number;
}

export interface FileDiff {
  path: string;
  action: "created" | "modified" | "deleted";
  diff?: string;
}

export interface Artifact {
  name: string;
  type: string;
  data?: string;
  url?: string;
}

export interface Outcome {
  status?: "success" | "failure" | "partial" | "aborted";
  summary?: string;
  files_changed?: FileDiff[];
  artifacts?: Artifact[];
}

export interface Stats {
  total_steps: number;
  total_llm_calls: number;
  total_tool_calls: number;
  total_tokens?: TokenUsage;
  total_cost_usd?: number;
  total_duration_ms?: number;
  errors_count: number;
  retries_count: number;
}

export interface Annotation {
  type: "comment" | "rating" | "label" | "flag";
  content: string;
  author?: string;
  step_index?: number;
  created_at?: string;
}

export interface Input {
  prompt?: string;
  system_prompt?: string;
  files?: FileSnapshot[];
  context?: Record<string, unknown>;
}

export interface Metadata {
  created_at: string;
  completed_at?: string;
  agent?: AgentInfo;
  model?: ModelInfo;
  environment?: EnvironmentInfo;
  tags?: string[];
  title?: string;
  description?: string;
}

export interface TrajectoryData {
  version: string;
  id: string;
  parent_id?: string;
  metadata: Metadata;
  input?: Input;
  steps: Step[];
  outcome?: Outcome;
  stats?: Stats;
  annotations?: Annotation[];
}
