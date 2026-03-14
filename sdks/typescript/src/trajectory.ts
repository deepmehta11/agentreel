import { randomUUID } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import type {
  Annotation,
  Input,
  Metadata,
  Outcome,
  Stats,
  Step,
  TokenUsage,
  TrajectoryData,
} from "./types.js";

export class Trajectory {
  version: string = "0.1.0";
  id: string;
  parent_id?: string;
  metadata: Metadata;
  input?: Input;
  steps: Step[] = [];
  outcome?: Outcome;
  stats?: Stats;
  annotations: Annotation[] = [];

  constructor(data?: Partial<TrajectoryData>) {
    this.id = data?.id ?? randomUUID();
    this.metadata = data?.metadata ?? { created_at: new Date().toISOString() };
    if (data) {
      this.version = data.version ?? "0.1.0";
      this.parent_id = data.parent_id;
      this.input = data.input;
      this.steps = data.steps ?? [];
      this.outcome = data.outcome;
      this.stats = data.stats;
      this.annotations = data.annotations ?? [];
    }
  }

  addStep(step: Omit<Step, "index" | "timestamp"> & { timestamp?: string }): number {
    const index = this.steps.length;
    this.steps.push({
      ...step,
      index,
      timestamp: step.timestamp ?? new Date().toISOString(),
    } as Step);
    return index;
  }

  computeStats(): Stats {
    let totalInputTokens = 0;
    let totalOutputTokens = 0;
    let totalCost = 0;
    let totalDuration = 0;
    let llmCalls = 0;
    let toolCalls = 0;
    let errors = 0;

    for (const step of this.steps) {
      if (step.duration_ms) totalDuration += step.duration_ms;
      if (step.cost_usd) totalCost += step.cost_usd;
      if (step.tokens) {
        totalInputTokens += step.tokens.input_tokens ?? 0;
        totalOutputTokens += step.tokens.output_tokens ?? 0;
      }
      if (step.type === "llm_call") llmCalls++;
      else if (step.type === "tool_call") toolCalls++;
      else if (step.type === "error") errors++;
    }

    this.stats = {
      total_steps: this.steps.length,
      total_llm_calls: llmCalls,
      total_tool_calls: toolCalls,
      total_tokens: {
        input_tokens: totalInputTokens,
        output_tokens: totalOutputTokens,
      },
      total_cost_usd: totalCost,
      total_duration_ms: totalDuration,
      errors_count: errors,
      retries_count: 0,
    };

    return this.stats;
  }

  fork(): Trajectory {
    const data = JSON.parse(this.toJSON()) as TrajectoryData;
    data.parent_id = this.id;
    data.id = randomUUID();
    data.metadata.created_at = new Date().toISOString();
    data.metadata.completed_at = undefined;
    data.annotations = [];
    return new Trajectory(data);
  }

  toJSON(indent: number = 2): string {
    const data: TrajectoryData = {
      version: this.version,
      id: this.id,
      parent_id: this.parent_id,
      metadata: this.metadata,
      input: this.input,
      steps: this.steps,
      outcome: this.outcome,
      stats: this.stats,
      annotations: this.annotations.length > 0 ? this.annotations : undefined,
    };
    return JSON.stringify(cleanObject(data), null, indent);
  }

  static fromJSON(json: string): Trajectory {
    const data = JSON.parse(json) as TrajectoryData;
    return new Trajectory(data);
  }

  save(path: string): void {
    writeFileSync(path, this.toJSON());
  }

  static load(path: string): Trajectory {
    const content = readFileSync(path, "utf-8");
    return Trajectory.fromJSON(content);
  }
}

function cleanObject(obj: unknown): unknown {
  if (obj === null || obj === undefined) return undefined;
  if (Array.isArray(obj)) {
    const cleaned = obj.map(cleanObject);
    return cleaned.length > 0 ? cleaned : undefined;
  }
  if (typeof obj === "object") {
    const result: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(obj as Record<string, unknown>)) {
      const cleaned = cleanObject(value);
      if (cleaned !== undefined) {
        result[key] = cleaned;
      }
    }
    return Object.keys(result).length > 0 ? result : undefined;
  }
  return obj;
}
