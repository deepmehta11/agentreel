# AgentReel

**GitHub for AI Agent Runs** — capture, replay, fork, and compare AI agent trajectories.

Every AI agent run becomes a shareable, forkable, diffable object. AgentReel captures the full trajectory: inputs, tool calls, LLM decisions, internal reasoning, screenshots, failures, fixes, and final output.

```bash
# Record any agent — zero code changes
agentreel record -- python my_agent.py

# View what happened
agentreel view trajectory.json --full

# Compare Claude vs GPT on the same task
agentreel compare claude_run.json gpt_run.json --format markdown
```

## Why?

Debugging AI agents is like debugging distributed systems in the 90s — no stack traces, no profiler, no way to know what happened.

| Problem | Solution |
|---|---|
| "What did my agent actually do?" | Step-by-step trajectory replay |
| "Why did it fail at step 7?" | Full inspection: messages, thinking, tool calls, HTTP details |
| "Is Claude better than GPT for this?" | Side-by-side comparison with cost/latency/quality diffs |
| "Someone solved this before — how?" | Fork their trajectory and swap your model |
| "$14 on a simple task — where'd the money go?" | Cost waterfall with per-step breakdown |

## Quick Start

### Option A: Zero-code proxy (recommended)

The proxy sits between your agent and LLM APIs. Your agent runs normally — it doesn't know it's being recorded.

```bash
# Install
cargo install --path crates/agentreel-cli

# Record — captures every LLM call and tool use automatically
agentreel record -t "My agent run" -- python my_agent.py

# View the trajectory
agentreel view trajectory.json --full
```

The proxy:
- Auto-detects OpenAI vs Anthropic from request headers
- Captures full request/response bodies, headers, rate limits
- Extracts tool_use blocks as separate ToolCall steps
- Records API errors (429, 500) as error steps
- Estimates cost per call using built-in model pricing
- Handles streaming responses

### Option B: Python SDK (richer instrumentation)

```python
from agentreel import Tracer

tracer = Tracer(title="Research cloud providers", tags=["research", "cloud"])

# Wrap your LLM client — all calls traced automatically
import openai
client = tracer.wrap_openai(openai.OpenAI())
# OR
import anthropic
client = tracer.wrap_anthropic(anthropic.Anthropic())

# Wrap your tool executor — captures every tool call + result
def my_tool_executor(tool_name, tool_input):
    if tool_name == "bash":
        return subprocess.run(tool_input["command"], capture_output=True).stdout.decode()
    elif tool_name == "read_file":
        return open(tool_input["path"]).read()

traced_executor = tracer.wrap_tool_executor(my_tool_executor)

# Or use the decorator for individual functions
@tracer.wrap_function("web_search")
def search(query: str) -> str:
    return requests.get(f"https://api.example.com/search?q={query}").text

# Manual logging for agent internals
tracer.log_thought("I should search for pricing data first...")
tracer.log_decision(
    "Use DuckDuckGo API",
    alternatives=["Google Search", "Bing"],
    reasoning="Free, no API key needed"
)
tracer.log_file_op("write", "report.md", content_preview="# Cloud Report\n...")

# MCP tool tracing
step_id = tracer.log_mcp_call("verify_email", mcp_server="https://mcp360.ai", input_data={"email": "test@example.com"})
tracer.log_mcp_result("verify_email", output={"valid": True}, parent_step_id=step_id)

# Finalize and save
tracer.complete("success", summary="Generated cloud provider comparison")
tracer.save("my_run.trajectory.json")
```

### Option C: TypeScript SDK

```typescript
import { Recorder } from "agentreel";

const recorder = new Recorder({ title: "My agent run" });

recorder.recordLlmCall({
  model: "claude-opus-4-6",
  messages: [{ role: "user", content: "Hello" }],
  response: { /* API response */ },
  duration_ms: 500,
});

recorder.recordToolCall({
  name: "bash",
  input: { command: "ls -la" },
  output: "total 42\n...",
  duration_ms: 100,
});

const trajectory = recorder.finalize({ summary: "Done" });
trajectory.save("run.trajectory.json");
```

## CLI Reference

```
agentreel record -- <cmd>     Record an agent run via LLM API proxy
agentreel view <file> [--full] View a trajectory step by step
agentreel stats <file>         Show totals: steps, tokens, cost, duration
agentreel diff <a> <b>         Side-by-side comparison of two trajectories
agentreel compare <a> <b> ...  Multi-trajectory comparison table
agentreel fork <file>          Create a linked copy for re-running
agentreel redact <file>        Strip secrets before sharing
agentreel validate <file>      Check if file is a valid trajectory
agentreel list                 Browse local trajectories
```

### Record

```bash
# Basic recording
agentreel record -- python my_agent.py

# With metadata
agentreel record -t "Build a REST API" --tags coding,python -o my_run.json -- python agent.py

# Works with any language/framework
agentreel record -- node my_agent.js
agentreel record -- ./my_rust_agent
```

The proxy sets `OPENAI_BASE_URL` and `ANTHROPIC_BASE_URL` on the child process. Any agent that uses these env vars (all official SDKs do) is captured automatically.

### View

```
$ agentreel view run.json --full

╔══════════════════════════════════════════════════════════════╗
║  Calculator Agent                                           ║
║  ID: 0e24bb4d-c1df-460f-88f6-fa5d32a8e44d                  ║
║  Steps: 3                                                   ║
╚══════════════════════════════════════════════════════════════╝

  🧠 Step 0 — LLM call (anthropic/claude-sonnet-4-20250514) [1413ms] [411in/81out]
     ├─ system: You are a helpful assistant. Always use tools when available.
     ├─ config: temp=0.3, max_tokens=512
     ├─ tools: [calculator]
     ├─ user: What is 47 * 89 + 123?
     ├─ 📝 text: I'll calculate that for you.
     ├─ 🔧 tool_use: calculator({"expression":"47 * 89 + 123"})
     ├─ 🌐 POST https://api.anthropic.com/v1/messages → 200
     │  request-id: req_011CZ...
     │  anthropic-ratelimit-requests-remaining: 1999
     └─ stop: ToolUse
  🔧 Step 1 — calculator [success]
     ├─ input: {"expression":"47 * 89 + 123"}
  🧠 Step 2 — LLM call (anthropic/claude-sonnet-4-20250514) [1100ms] [506in/23out]
     ├─ 📝 text: The result of 47 * 89 + 123 is **4,306**.
     └─ stop: EndTurn
```

### Compare

```
$ agentreel compare claude_run.json gpt_run.json gemini_run.json

Multi-Trajectory Comparison (3 runs)
================================================================

Metric              claude_run          gpt_run             gemini_run
                    ---                 ---                 ---
Model               claude-sonnet-4-6   gpt-4o              gemini-2.5-pro
Steps               3                   5                   4
Tokens              1.0k                2.3k                1.5k
Cost                $0.0105             $0.0325             $0.0188
Duration            3.3s                5.1s                4.2s
Errors              0                   1                   0

Analysis:
  Cheapest:  claude_run
  Fastest:   claude_run
  Fewest errors: claude_run
```

### Redact

Strips 15+ secret patterns before sharing:

- OpenAI keys (`sk-...`)
- Anthropic keys (`sk-ant-...`)
- GitHub tokens (`ghp_...`, `github_pat_...`)
- Stripe keys (`sk_live_...`, `sk_test_...`)
- AWS keys (`AKIA...`), secrets
- Google API keys (`AIza...`)
- Slack tokens (`xoxb-...`)
- Bearer tokens, passwords, emails, IP addresses, private keys

```bash
agentreel redact trajectory.json -o redacted.json
```

## What Gets Captured

Every trajectory records:

### Per LLM Call
- **Provider + model** (anthropic/claude-opus-4-6, openai/gpt-4o)
- **System prompt**
- **All input messages** (user, assistant, tool results)
- **Model config** (temperature, max_tokens, top_p)
- **Available tools** (tool definitions sent to the model)
- **Response content blocks** (text, tool_use, thinking)
- **Internal reasoning** (Claude extended thinking, CoT)
- **Stop reason** (end_turn, tool_use, max_tokens)
- **Token usage** (input, output, cache, thinking)
- **Cost estimate** (built-in pricing for all major models)
- **HTTP details** (URL, status, request-id, rate limits)
- **Raw request/response bodies**

### Per Tool Call
- **Tool name and type** (function, mcp, web_search, code_execution)
- **Input parameters**
- **Output/result** with success/error status
- **Duration**
- **MCP server info** (URL, server name)
- **Parent step link** (which LLM call requested this tool)

### Other Step Types
- **Thoughts** — agent internal reasoning
- **Decisions** — explicit choice points with alternatives and reasoning
- **File operations** — read/write/create/delete with content preview
- **Human input** — approval, correction, cancellation
- **Errors** — with recovery actions and stack traces
- **Screenshots** — captured UI state

## Trajectory Format

Open spec. JSON-based. Vendor-neutral. See [`spec/trajectory.schema.json`](spec/trajectory.schema.json).

```json
{
  "version": "0.1.0",
  "id": "uuid",
  "metadata": {
    "title": "Research cloud providers",
    "tags": ["research", "cloud"],
    "agent": { "name": "ResearchAgent" },
    "model": { "provider": "anthropic", "model_id": "claude-opus-4-6" }
  },
  "steps": [
    {
      "type": "thought",
      "thought": { "content": "I'll search for GPU pricing first..." }
    },
    {
      "type": "llm_call",
      "llm_call": {
        "model": "claude-opus-4-6",
        "system_prompt": "You are a research assistant.",
        "config": { "temperature": 0.3 },
        "available_tools": [{"name": "web_search"}],
        "response_blocks": [
          {"type": "text", "text": "Let me search for that."},
          {"type": "tool_use", "tool_name": "web_search", "input": {"query": "H100 pricing"}}
        ],
        "thinking": "I should compare on-demand vs spot pricing..."
      },
      "tokens": { "input_tokens": 1847, "output_tokens": 312 },
      "cost_usd": 0.0102
    },
    {
      "type": "tool_call",
      "tool_call": { "name": "web_search", "input": {"query": "H100 pricing"} }
    },
    {
      "type": "tool_result",
      "parent_step_id": 2,
      "tool_result": { "tool_name": "web_search", "output": "AWS p5.48xlarge: $98.32/hr..." }
    }
  ],
  "stats": {
    "total_steps": 4,
    "total_cost_usd": 0.0102,
    "total_duration_ms": 3400
  }
}
```

## Architecture

```
┌─────────────────┐     ┌───────────────────────┐     ┌──────────────────┐
│   Your Agent     │────▶│   agentreel proxy     │────▶│  LLM APIs        │
│   (any framework)│◀────│                       │◀────│  (OpenAI/Claude) │
└─────────────────┘     │  Auto-detects provider │     └──────────────────┘
                        │  Captures everything   │
                        │  Extracts tool calls   │
                        │  Records errors        │
                        │  Estimates costs       │
                        └───────────┬───────────┘
                                    │
                           ┌────────▼─────────┐
                           │ .trajectory.json  │
                           └────────┬──────────┘
                                    │
                  ┌─────────────────┼─────────────────┐
                  │                 │                  │
           ┌──────▼──────┐  ┌──────▼──────┐  ┌───────▼──────┐
           │  view/replay │  │ diff/compare│  │ list/redact  │
           └─────────────┘  └─────────────┘  └──────────────┘
```

## Project Structure

```
agentreel/
├── spec/
│   ├── trajectory.schema.json    # Open trajectory format spec
│   └── example.trajectory.json
├── crates/
│   ├── agentreel-core/           # Rust types, diff, redaction, cost estimation
│   ├── agentreel-cli/            # CLI: record, view, stats, diff, compare, list, fork, redact, validate
│   └── agentreel-proxy/          # HTTP proxy with auto-detect, SSE, tool extraction, error capture
├── sdks/
│   ├── python/                   # Python SDK: Tracer, Recorder, redact, costs
│   └── typescript/               # TypeScript SDK: Recorder, Trajectory, redact
├── web/                          # Next.js viewer (timeline, search, replay, cost waterfall, smart analysis)
├── Cargo.toml
└── LICENSE (MIT)
```

## Model Pricing

Built-in cost estimation for all major models:

| Model | Input ($/1M tokens) | Output ($/1M tokens) |
|-------|-------------------:|--------------------:|
| Claude Opus 4.6 | $15.00 | $75.00 |
| Claude Sonnet 4.6 | $3.00 | $15.00 |
| Claude Haiku 4.5 | $0.80 | $4.00 |
| GPT-4.5 Turbo | $5.00 | $15.00 |
| GPT-4o | $2.50 | $10.00 |
| GPT-4o Mini | $0.15 | $0.60 |
| o3 | $10.00 | $40.00 |
| Gemini 2.5 Pro | $1.25 | $10.00 |
| DeepSeek R1 | $0.55 | $2.19 |

## Contributing

Apache 2.0. Contributions welcome:

- **Trajectory spec** — propose additions to the format
- **Framework adapters** — LangChain, CrewAI, OpenAI Agents SDK hooks
- **Viewers** — new visualization modes
- **Benchmarks** — share interesting trajectory comparisons

## License

MIT. See [LICENSE](LICENSE).

---

**Built by [Deep Mehta](https://github.com/deepmehta11)**

*"Every agent run should be as inspectable as a git commit."*
