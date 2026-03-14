import { describe, it } from "node:test";
import assert from "node:assert";
import { Trajectory } from "../trajectory.js";
import { Recorder } from "../recorder.js";
import { redact } from "../redact.js";

describe("Trajectory", () => {
  it("creates with defaults", () => {
    const t = new Trajectory();
    assert.strictEqual(t.version, "0.1.0");
    assert.ok(t.id);
    assert.deepStrictEqual(t.steps, []);
  });

  it("forks with parent link", () => {
    const t = new Trajectory();
    const forked = t.fork();
    assert.notStrictEqual(forked.id, t.id);
    assert.strictEqual(forked.parent_id, t.id);
  });

  it("serializes and deserializes", () => {
    const t = new Trajectory();
    t.metadata.title = "Test run";
    const json = t.toJSON();
    const parsed = Trajectory.fromJSON(json);
    assert.strictEqual(parsed.id, t.id);
    assert.strictEqual(parsed.metadata.title, "Test run");
  });
});

describe("Recorder", () => {
  it("records LLM calls (OpenAI format)", () => {
    const recorder = new Recorder({ title: "Test" });
    const idx = recorder.recordLlmCall({
      model: "gpt-4o",
      messages: [{ role: "user", content: "Hello" }],
      response: {
        choices: [
          { message: { role: "assistant", content: "Hi!" }, finish_reason: "stop" },
        ],
        usage: { prompt_tokens: 10, completion_tokens: 5 },
      },
      duration_ms: 500,
    });
    assert.strictEqual(idx, 0);

    const traj = recorder.finalize({ summary: "Done" });
    assert.strictEqual(traj.steps.length, 1);
    assert.strictEqual(traj.stats?.total_llm_calls, 1);
  });

  it("records LLM calls (Anthropic format)", () => {
    const recorder = new Recorder();
    recorder.recordLlmCall({
      model: "claude-opus-4-6",
      messages: [{ role: "user", content: "Hello" }],
      response: {
        content: [{ type: "text", text: "Hi there!" }],
        stop_reason: "end_turn",
        usage: { input_tokens: 10, output_tokens: 5 },
      },
    });

    const traj = recorder.finalize();
    const step = traj.steps[0];
    assert.strictEqual(step.llm_call?.model, "claude-opus-4-6");
    assert.strictEqual(step.llm_call?.response?.content, "Hi there!");
    assert.strictEqual(step.llm_call?.stop_reason, "end_turn");
  });

  it("records tool calls", () => {
    const recorder = new Recorder();
    recorder.recordToolCall({
      name: "bash",
      input: { command: "echo hello" },
      output: "hello",
      duration_ms: 100,
    });

    const traj = recorder.finalize();
    assert.strictEqual(traj.stats?.total_tool_calls, 1);
  });
});

describe("Redact", () => {
  it("redacts API keys", () => {
    const result = redact("Using key sk-abcdefghijklmnopqrstuvwxyz123456");
    assert.ok(result.includes("[REDACTED_API_KEY]"));
    assert.ok(!result.includes("sk-abcdef"));
  });

  it("redacts emails", () => {
    const result = redact("Contact user@example.com for details");
    assert.ok(result.includes("[REDACTED_EMAIL]"));
    assert.ok(!result.includes("user@example.com"));
  });

  it("redacts AWS keys", () => {
    const result = redact("AWS key: AKIAIOSFODNN7EXAMPLE");
    assert.ok(result.includes("[REDACTED_AWS_KEY]"));
  });
});
