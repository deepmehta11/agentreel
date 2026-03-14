import { Trajectory } from "./trajectory.js";
import type { LlmCall, Message, Step, TokenUsage, ToolCall } from "./types.js";

const MODEL_COSTS: Record<string, { input: number; output: number }> = {
  "claude-opus-4-6": { input: 15.0, output: 75.0 },
  "claude-sonnet-4-6": { input: 3.0, output: 15.0 },
  "claude-sonnet-4-20250514": { input: 3.0, output: 15.0 },
  "claude-haiku-4-5-20251001": { input: 0.8, output: 4.0 },
  "gpt-4.5-turbo": { input: 5.0, output: 15.0 },
  "gpt-4o": { input: 2.5, output: 10.0 },
  "gpt-4o-mini": { input: 0.15, output: 0.6 },
  "o3": { input: 10.0, output: 40.0 },
  "gemini-2.5-pro": { input: 1.25, output: 10.0 },
};

function estimateCost(model: string, inputTokens: number, outputTokens: number): number {
  const costs = MODEL_COSTS[model] ?? Object.entries(MODEL_COSTS).find(([k]) => model.includes(k) || k.includes(model))?.[1];
  if (!costs) return 0;
  return (inputTokens * costs.input + outputTokens * costs.output) / 1_000_000;
}

/**
 * Tracer — wraps LLM clients and tool executors to auto-capture everything.
 *
 * Usage:
 *   const tracer = new Tracer({ title: "My agent run" });
 *   const openai = tracer.wrapOpenAI(new OpenAI());
 *   // use openai normally — all calls traced
 *   tracer.complete({ summary: "Done" });
 *   tracer.save("run.trajectory.json");
 */
export class Tracer {
  public trajectory: Trajectory;
  private stepCounter = 0;

  constructor(options?: { title?: string; tags?: string[]; taskPrompt?: string }) {
    this.trajectory = new Trajectory();
    if (options?.title) this.trajectory.metadata.title = options.title;
    if (options?.tags) this.trajectory.metadata.tags = options.tags;
    if (options?.taskPrompt) {
      this.trajectory.input = { prompt: options.taskPrompt };
    }
  }

  private addStep(type: Step["type"], data?: Partial<Step>): number {
    const idx = this.stepCounter++;
    this.trajectory.addStep({
      type,
      ...data,
    });
    return idx;
  }

  // ── LLM Client Wrapping ──────────────────────────────────

  /**
   * Wrap an OpenAI client to trace all chat.completions.create calls.
   *
   * Usage:
   *   import OpenAI from "openai";
   *   const client = tracer.wrapOpenAI(new OpenAI());
   *   const resp = await client.chat.completions.create({ model: "gpt-4o", messages: [...] });
   */
  wrapOpenAI<T extends { chat: { completions: { create: Function } } }>(client: T): T {
    const tracer = this;
    const originalCreate = client.chat.completions.create.bind(client.chat.completions);

    client.chat.completions.create = async function (...args: unknown[]) {
      const params = (args[0] ?? {}) as Record<string, unknown>;
      const model = (params.model as string) ?? "unknown";
      const messages = (params.messages as Array<{ role: string; content?: string }>) ?? [];

      const start = Date.now();
      try {
        const response = await originalCreate(...args);
        const durationMs = Date.now() - start;

        const usage = (response as any).usage;
        const inputTokens = usage?.prompt_tokens ?? usage?.input_tokens ?? 0;
        const outputTokens = usage?.completion_tokens ?? usage?.output_tokens ?? 0;

        const choice = (response as any).choices?.[0];
        const content = choice?.message?.content ?? "";
        const finishReason = choice?.finish_reason;

        tracer.addStep("llm_call", {
          duration_ms: durationMs,
          llm_call: {
            model,
            messages: messages.map((m) => ({
              role: m.role as Message["role"],
              content: typeof m.content === "string" ? m.content.slice(0, 2000) : undefined,
            })),
            response: { role: "assistant", content: content.slice(0, 2000) },
            stop_reason: finishReason === "stop" ? "end_turn" : finishReason === "tool_calls" ? "tool_use" : undefined,
          },
          tokens: { input_tokens: inputTokens, output_tokens: outputTokens },
          cost_usd: estimateCost(model, inputTokens, outputTokens),
        });

        // Extract tool calls
        if (choice?.message?.tool_calls) {
          for (const tc of choice.message.tool_calls) {
            tracer.logToolCall(tc.function?.name ?? "unknown", {
              arguments: tc.function?.arguments,
            });
          }
        }

        return response;
      } catch (e) {
        tracer.logError((e as Error).message, (e as Error).constructor.name);
        throw e;
      }
    } as any;

    return client;
  }

  /**
   * Wrap an Anthropic client to trace all messages.create calls.
   *
   * Usage:
   *   import Anthropic from "@anthropic-ai/sdk";
   *   const client = tracer.wrapAnthropic(new Anthropic());
   *   const resp = await client.messages.create({ model: "claude-sonnet-4-6", messages: [...] });
   */
  wrapAnthropic<T extends { messages: { create: Function } }>(client: T): T {
    const tracer = this;
    const originalCreate = client.messages.create.bind(client.messages);

    client.messages.create = async function (...args: unknown[]) {
      const params = (args[0] ?? {}) as Record<string, unknown>;
      const model = (params.model as string) ?? "unknown";
      const messages = (params.messages as Array<{ role: string; content?: string }>) ?? [];

      const start = Date.now();
      try {
        const response = await originalCreate(...args);
        const durationMs = Date.now() - start;

        const resp = response as any;
        const inputTokens = resp.usage?.input_tokens ?? 0;
        const outputTokens = resp.usage?.output_tokens ?? 0;

        // Extract text and thinking
        let textContent = "";
        let thinkingContent = "";
        for (const block of resp.content ?? []) {
          if (block.type === "text") textContent += block.text;
          if (block.type === "thinking") thinkingContent += block.thinking;
        }

        tracer.addStep("llm_call", {
          duration_ms: durationMs,
          llm_call: {
            model,
            messages: messages.map((m) => ({
              role: m.role as Message["role"],
              content: typeof m.content === "string" ? m.content.slice(0, 2000) : undefined,
            })),
            response: { role: "assistant", content: textContent.slice(0, 2000) },
            stop_reason: resp.stop_reason as LlmCall["stop_reason"],
          },
          tokens: { input_tokens: inputTokens, output_tokens: outputTokens },
          cost_usd: estimateCost(model, inputTokens, outputTokens),
        });

        // Log thinking
        if (thinkingContent) {
          tracer.logThought(thinkingContent);
        }

        // Extract tool_use blocks
        for (const block of resp.content ?? []) {
          if (block.type === "tool_use") {
            tracer.logToolCall(block.name, block.input);
          }
        }

        return response;
      } catch (e) {
        tracer.logError((e as Error).message, (e as Error).constructor.name);
        throw e;
      }
    } as any;

    return client;
  }

  // ── Tool Wrapping ────────────────────────────────────────

  /**
   * Wrap a tool executor function to capture every tool call.
   *
   * Usage:
   *   const exec = tracer.wrapToolExecutor(myExecutor);
   *   const result = await exec("bash", { command: "ls" });
   */
  wrapToolExecutor(
    executor: (name: string, input: Record<string, unknown>) => unknown | Promise<unknown>
  ): (name: string, input: Record<string, unknown>) => Promise<unknown> {
    const tracer = this;

    return async function (name: string, input: Record<string, unknown>) {
      const start = Date.now();
      const callIdx = tracer.logToolCall(name, input);

      try {
        const result = await executor(name, input);
        const durationMs = Date.now() - start;
        tracer.logToolResult(name, result, callIdx, false, durationMs);
        if (callIdx < tracer.trajectory.steps.length) {
          tracer.trajectory.steps[callIdx].duration_ms = durationMs;
        }
        return result;
      } catch (e) {
        const durationMs = Date.now() - start;
        tracer.logToolResult(name, (e as Error).message, callIdx, true, durationMs);
        throw e;
      }
    };
  }

  // ── Manual Logging ───────────────────────────────────────

  logToolCall(name: string, input?: Record<string, unknown>, toolType?: string): number {
    return this.addStep("tool_call", {
      tool_call: {
        name,
        input,
        status: "success",
      },
    });
  }

  logToolResult(
    toolName: string,
    output: unknown,
    parentStepId?: number,
    isError?: boolean,
    durationMs?: number
  ): number {
    // Update parent tool_call status
    if (parentStepId != null && parentStepId < this.trajectory.steps.length) {
      const parent = this.trajectory.steps[parentStepId];
      if (parent.tool_call) {
        parent.tool_call.status = isError ? "error" : "success";
        parent.tool_call.output = output;
      }
    }
    return this.addStep("tool_call", { duration_ms: durationMs });
  }

  logThought(content: string): number {
    return this.addStep("thought" as Step["type"]);
  }

  logDecision(decision: string, options?: { alternatives?: string[]; reasoning?: string }): number {
    return this.addStep("checkpoint" as Step["type"]);
  }

  logError(message: string, code?: string, recoverable?: boolean): number {
    return this.addStep("error", {
      error: { message, code, recoverable: recoverable ?? true },
    });
  }

  // ── Finalization ─────────────────────────────────────────

  complete(options?: { status?: string; summary?: string }): Trajectory {
    this.trajectory.metadata.completed_at = new Date().toISOString();
    this.trajectory.outcome = {
      status: (options?.status ?? "success") as any,
      summary: options?.summary,
    };
    this.trajectory.computeStats();
    return this.trajectory;
  }

  save(path: string): void {
    this.trajectory.save(path);
  }
}
