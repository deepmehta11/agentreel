import { Trajectory } from "./trajectory.js";
import type { LlmCall, Message, Step, TokenUsage, ToolCall } from "./types.js";

export class Recorder {
  private trajectory: Trajectory;

  constructor(options?: { title?: string; tags?: string[]; trajectory?: Trajectory }) {
    this.trajectory = options?.trajectory ?? new Trajectory();
    if (options?.title) this.trajectory.metadata.title = options.title;
    if (options?.tags) this.trajectory.metadata.tags = options.tags;
  }

  recordLlmCall(params: {
    model: string;
    messages: Array<{ role: string; content?: string }>;
    response: Record<string, unknown>;
    duration_ms?: number;
  }): number {
    const respContent = extractResponseContent(params.response);
    const tokens = extractTokens(params.response);
    const stopReason = extractStopReason(params.response);

    return this.trajectory.addStep({
      type: "llm_call",
      duration_ms: params.duration_ms,
      llm_call: {
        model: params.model,
        messages: params.messages.map((m) => ({
          role: m.role as Message["role"],
          content: m.content,
        })),
        response: { role: "assistant", content: respContent ?? undefined },
        stop_reason: stopReason ?? undefined,
      },
      tokens: tokens ?? undefined,
    });
  }

  recordToolCall(params: {
    name: string;
    input?: Record<string, unknown>;
    output?: unknown;
    status?: "success" | "error" | "timeout" | "denied";
    duration_ms?: number;
  }): number {
    return this.trajectory.addStep({
      type: "tool_call",
      duration_ms: params.duration_ms,
      tool_call: {
        name: params.name,
        input: params.input,
        output: params.output,
        status: params.status ?? "success",
      },
    });
  }

  recordError(params: {
    message: string;
    code?: string;
    recoverable?: boolean;
  }): number {
    return this.trajectory.addStep({
      type: "error",
      error: {
        message: params.message,
        code: params.code,
        recoverable: params.recoverable ?? true,
      },
    });
  }

  finalize(options?: {
    status?: "success" | "failure" | "partial" | "aborted";
    summary?: string;
  }): Trajectory {
    this.trajectory.metadata.completed_at = new Date().toISOString();
    this.trajectory.outcome = {
      status: options?.status ?? "success",
      summary: options?.summary,
    };
    this.trajectory.computeStats();
    return this.trajectory;
  }
}

function extractResponseContent(response: Record<string, unknown>): string | null {
  // OpenAI
  const choices = response.choices as Array<Record<string, unknown>> | undefined;
  if (choices?.length) {
    const msg = choices[0].message as Record<string, unknown> | undefined;
    if (msg?.content && typeof msg.content === "string") return msg.content;
  }

  // Anthropic
  const content = response.content as Array<Record<string, unknown>> | undefined;
  if (content?.length) {
    const texts = content
      .filter((b) => b.type === "text" && typeof b.text === "string")
      .map((b) => b.text as string);
    return texts.length > 0 ? texts.join("\n") : null;
  }

  return null;
}

function extractTokens(response: Record<string, unknown>): TokenUsage | null {
  const usage = response.usage as Record<string, unknown> | undefined;
  if (!usage) return null;

  return {
    input_tokens:
      (usage.input_tokens as number) ?? (usage.prompt_tokens as number) ?? undefined,
    output_tokens:
      (usage.output_tokens as number) ?? (usage.completion_tokens as number) ?? undefined,
    cache_read_tokens: (usage.cache_read_input_tokens as number) ?? undefined,
    cache_write_tokens: (usage.cache_creation_input_tokens as number) ?? undefined,
  };
}

function extractStopReason(
  response: Record<string, unknown>
): LlmCall["stop_reason"] | null {
  // Anthropic
  if (typeof response.stop_reason === "string") {
    const map: Record<string, LlmCall["stop_reason"]> = {
      end_turn: "end_turn",
      tool_use: "tool_use",
      max_tokens: "max_tokens",
      stop_sequence: "stop_sequence",
    };
    return map[response.stop_reason] ?? null;
  }

  // OpenAI
  const choices = response.choices as Array<Record<string, unknown>> | undefined;
  if (choices?.length) {
    const reason = choices[0].finish_reason as string;
    const map: Record<string, LlmCall["stop_reason"]> = {
      stop: "end_turn",
      tool_calls: "tool_use",
      length: "max_tokens",
    };
    return map[reason] ?? null;
  }

  return null;
}
