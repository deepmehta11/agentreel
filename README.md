# AgentReel

**GitHub for AI Agent Runs** — capture, replay, fork, and compare AI agent trajectories.

Every AI agent run becomes a shareable, forkable, diffable object. AgentReel captures the full trajectory: inputs, tool calls, LLM decisions, internal reasoning, failures, costs, and final output.

```bash
# One-line install
curl -fsSL https://raw.githubusercontent.com/deepmehta11/agentreel/main/install.sh | bash

# Record any agent — zero code changes
agentreel record -- python my_agent.py

# See everything that happened
agentreel view trajectory.json --full

# Compare models on the same task
agentreel compare claude_run.json gpt_run.json --format markdown
```

> **Status:** Work in progress. Core is functional. Contributions welcome.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [Python SDK](#python-sdk)
- [TypeScript SDK](#typescript-sdk)
- [Framework Adapters](#framework-adapters)
- [What Gets Captured](#what-gets-captured)
- [Trajectory Format](#trajectory-format)
- [Web Viewer](#web-viewer)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Contributing](#contributing)

---

## Installation

### One-liner (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/deepmehta11/agentreel/main/install.sh | bash
```

### From source (requires Rust)

```bash
git clone https://github.com/deepmehta11/agentreel.git
cd agentreel
cargo install --path crates/agentreel-cli
```

### Python SDK

```bash
cd sdks/python
pip install -e .
```

### TypeScript SDK

```bash
cd sdks/typescript
npm install
npm run build
```

---

## Quick Start

### Option A: Zero-code proxy (recommended)

Record any agent with zero code changes. The proxy sits between your agent and LLM APIs — your agent doesn't know it's being recorded.

```bash
# Record a Python agent
agentreel record -t "Build a REST API" --tags coding,python -- python my_agent.py

# Record a Node.js agent
agentreel record -t "Research task" -- node my_agent.js

# Record any executable
agentreel record -- ./my_rust_agent
```

The proxy automatically:
- Detects OpenAI vs Anthropic from request headers
- Captures full request/response bodies, headers, rate limits
- Extracts `tool_use` blocks as separate ToolCall steps
- Records API errors (429, 500) as error steps
- Estimates cost per call
- Handles streaming responses

Then inspect:

```bash
agentreel view trajectory.json --full
agentreel stats trajectory.json
```

### Option B: Python SDK (captures everything)

```python
from agentreel import Tracer

tracer = Tracer(title="Research cloud providers", tags=["research"])

# Wrap your LLM client — all calls traced
import anthropic
client = tracer.wrap_anthropic(anthropic.Anthropic())

# Wrap your tool executor — all tools traced
def execute_tool(name, input):
    if name == "bash":
        return subprocess.run(input["command"], capture_output=True).stdout.decode()
    elif name == "read_file":
        return open(input["path"]).read()

traced_execute = tracer.wrap_tool_executor(execute_tool)

# Or decorate individual functions
@tracer.wrap_function("web_search")
def search(query: str) -> str:
    return requests.get(f"https://api.example.com/search?q={query}").text

# Log agent internals
tracer.log_thought("I should search for pricing data first...")
tracer.log_decision("Use DuckDuckGo", alternatives=["Google", "Bing"], reasoning="Free API")
tracer.log_file_op("write", "report.md", content_preview="# Report\n...")

# Finalize and save
tracer.complete("success", summary="Generated comparison report")
tracer.save("my_run.trajectory.json")
```

### Option C: Framework adapters (one-line integration)

```python
# LangChain
from agentreel.adapters.langchain import AgentReelCallbackHandler
handler = AgentReelCallbackHandler(title="My LangChain run")
result = chain.invoke(input, config={"callbacks": [handler]})
handler.save()

# OpenAI Agents SDK
from agentreel.adapters.openai_agents import AgentReelTracer
tracer = AgentReelTracer(title="My agent")
from agents.tracing import set_trace_processors
set_trace_processors([tracer])
# agent runs are captured automatically
tracer.save()

# CrewAI
from agentreel.adapters.crewai import AgentReelCrewCallback
callback = AgentReelCrewCallback(title="My crew")
# attach to crew events
callback.save()
```

---

## CLI Reference

### `agentreel record`

Record an agent run by proxying LLM API calls.

```bash
agentreel record [OPTIONS] -- <COMMAND>

Options:
  -t, --title <TITLE>    Title for this run
  -o, --output <FILE>    Output file [default: trajectory.json]
      --tags <TAGS>      Comma-separated tags

Examples:
  agentreel record -- python my_agent.py
  agentreel record -t "Bug fix" --tags coding,fix -o bugfix.json -- python agent.py
```

### `agentreel view`

View a trajectory step by step.

```bash
agentreel view <FILE> [--full]

Without --full:
  🧠 Step 0 — LLM call (claude-sonnet-4-20250514) [1413ms] [411in/81out]
  🔧 Step 1 — calculator [] success
  🧠 Step 2 — LLM call (claude-sonnet-4-20250514) [1100ms] [506in/23out]

With --full (shows everything):
  🧠 Step 0 — LLM call (anthropic/claude-sonnet-4-20250514) [1413ms] [411in/81out]
     ├─ system: You are a helpful assistant.
     ├─ config: temp=0.3, max_tokens=512
     ├─ tools: [calculator]
     ├─ user: What is 47 * 89 + 123?
     ├─ 💭 thinking: Let me calculate this step by step...
     ├─ 📝 text: I'll calculate that for you.
     ├─ 🔧 tool_use: calculator({"expression":"47 * 89 + 123"})
     ├─ 🌐 POST https://api.anthropic.com/v1/messages → 200
     │  request-id: req_011CZ...
     │  anthropic-ratelimit-requests-remaining: 1999
     └─ stop: ToolUse
```

### `agentreel stats`

Show totals for a trajectory.

```bash
agentreel stats <FILE>

Trajectory Stats
────────────────────────────────
  Steps:      3
  LLM calls:  2
  Tool calls: 1
  Errors:     0
  Tokens:     917 in / 104 out (1021 total)
  Cost:       $0.0192
  Duration:   5.4s
```

### `agentreel diff`

Compare two trajectories side by side.

```bash
agentreel diff <LEFT> <RIGHT>

Trajectory Diff
  Left:  a1b2c3d4...
  Right: e5f6g7h8...

Metadata Changes:
  model: claude-sonnet-4-6 -> gpt-4o

Steps:
  = Step 0 (identical)
  ~ Step 1 (changed)
      model: claude-sonnet-4-6 -> gpt-4o
      duration: 1200ms -> 800ms
  + Step 2 (right only): tool_call(web_search)

Stats Comparison:
  Tokens:     607 vs 2341
  Cost:       $0.0105 vs $0.0325
  Duration:   5.4s vs 12.1s
```

### `agentreel compare`

Compare multiple trajectories in a table.

```bash
agentreel compare <FILE1> <FILE2> [FILE3...] [--format text|json|markdown]

Multi-Trajectory Comparison (3 runs)
================================================================
Metric              claude_run          gpt_run             gemini_run
Model               claude-sonnet-4-6   gpt-4o              gemini-2.5-pro
Steps               3                   5                   4
Tokens              1.0k                2.3k                1.5k
Cost                $0.0105             $0.0325             $0.0188
Duration            3.3s                5.1s                4.2s
Errors              0                   1                   0

Analysis:
  Cheapest:      claude_run
  Fastest:       claude_run
  Fewest errors: claude_run
```

### `agentreel list`

Browse local trajectories.

```bash
agentreel list [-n LIMIT] [--tags TAG1,TAG2] [--dir PATH]

STATUS   TITLE                          STEPS  COST       DURATION   MODEL    CREATED
----------------------------------------------------------------------------------------------------
✅  Calculator Agent               3      $0.0192    5.4s     claude   2026-03-14
✅  Research Task                  17     $0.2916    45.2s    opus     2026-03-13
❌  Failed Deploy                  8      $0.0450    12.1s    gpt-4o   2026-03-12

3 trajectories in /Users/deep/.agentreel/trajectories
```

### `agentreel fork`

Fork a trajectory for re-running with different parameters.

```bash
agentreel fork <SOURCE> [-o OUTPUT]

Forked trajectory:
  Parent: a1b2c3d4...
  New:    e5f6g7h8...
  Saved:  trajectory_fork.json
```

### `agentreel redact`

Strip secrets before sharing. Detects 15+ patterns:

- OpenAI keys (`sk-...`), Anthropic keys (`sk-ant-...`)
- GitHub tokens (`ghp_...`, `github_pat_...`)
- Stripe keys (`sk_live_...`, `sk_test_...`)
- AWS keys (`AKIA...`), secrets
- Google API keys (`AIza...`), Slack tokens (`xoxb-...`)
- Bearer tokens, passwords, emails, IP addresses, private keys

```bash
agentreel redact trajectory.json -o safe.json
```

### `agentreel validate`

Check if a file is a valid trajectory.

```bash
agentreel validate trajectory.json

Valid trajectory (v0.1.0)
  ID:    a1b2c3d4...
  Steps: 3
  Title: Calculator Agent
```

---

## Python SDK

### Tracer (recommended)

Auto-capture everything with client wrapping:

```python
from agentreel import Tracer

tracer = Tracer(title="My agent run", tags=["demo"])

# Wrap LLM clients
client = tracer.wrap_openai(openai.OpenAI())       # OpenAI
client = tracer.wrap_anthropic(anthropic.Anthropic()) # Anthropic

# Wrap tool executor
traced_exec = tracer.wrap_tool_executor(my_executor)

# Decorator for individual functions
@tracer.wrap_function("web_search")
def search(query): ...

# Manual logging
tracer.log_thought("Reasoning about approach...")
tracer.log_decision("Use approach A", alternatives=["B", "C"], reasoning="...")
tracer.log_tool_call("bash", input_data={"command": "ls"})
tracer.log_tool_result("bash", output="file1.py file2.py", parent_step_id=0)
tracer.log_file_op("write", "/tmp/output.txt", content_preview="Hello...")
tracer.log_error("Rate limited", code="429", recoverable=True)
tracer.log_human_input("Approved", action="approve")
tracer.log_mcp_call("verify_email", mcp_server="https://mcp360.ai", input_data={...})

# Finalize
tracer.complete("success", summary="Task completed")
tracer.save("my_run.trajectory.json")
```

### Recorder (manual)

For step-by-step control:

```python
from agentreel import Recorder

recorder = Recorder(title="My run")
recorder.record_llm_call(model="gpt-4o", messages=[...], response={...}, duration_ms=500)
recorder.record_tool_call(name="bash", input={"cmd": "ls"}, output="files", duration_ms=100)
trajectory = recorder.finalize(summary="Done")
trajectory.save("run.json")
```

### Cost Estimation

```python
from agentreel import estimate_cost

cost = estimate_cost("claude-opus-4-6", input_tokens=1000, output_tokens=500)
# $0.0525
```

---

## TypeScript SDK

```typescript
import { Recorder, Trajectory, redact } from "agentreel";

// Record
const recorder = new Recorder({ title: "My run", tags: ["demo"] });
recorder.recordLlmCall({ model: "gpt-4o", messages: [...], response: {...} });
recorder.recordToolCall({ name: "bash", input: {...}, output: "..." });
const trajectory = recorder.finalize({ summary: "Done" });

// Save / Load
trajectory.save("run.json");
const loaded = Trajectory.load("run.json");

// Fork
const forked = loaded.fork();

// Redact
const safe = redact("key: sk-abc123...");
```

---

## Framework Adapters

### LangChain

```python
from agentreel.adapters.langchain import AgentReelCallbackHandler

handler = AgentReelCallbackHandler(title="Research task", tags=["langchain"])

# Use with any LangChain component
llm = ChatOpenAI(model="gpt-4o", callbacks=[handler])
chain = prompt | llm | parser
result = chain.invoke({"input": "Compare Stripe vs Adyen"})

# Or with agents
agent_executor = AgentExecutor(agent=agent, tools=tools, callbacks=[handler])
result = agent_executor.invoke({"input": "Research X"})

# Save
handler.complete("success")
handler.save("langchain_run.trajectory.json")
```

**Captures:** LLM calls (tokens, cost, latency), tool executions (input, output, duration, errors), agent actions/decisions, chain events, retriever queries, all errors with stack traces.

### OpenAI Agents SDK

```python
from agentreel.adapters.openai_agents import AgentReelTracer
from agents import Agent, Runner
from agents.tracing import set_trace_processors

tracer = AgentReelTracer(title="Customer support")
set_trace_processors([tracer])

agent = Agent(name="Support", instructions="Help customers...")
result = Runner.run_sync(agent, "I need help with my order")

tracer.complete("success")
tracer.save()
```

**Captures:** LLM spans, function/tool calls, agent-to-agent handoffs, guardrail checks, agent lifecycle events.

### CrewAI

```python
from agentreel.adapters.crewai import AgentReelCrewCallback

callback = AgentReelCrewCallback(title="Content crew")

# Log crew events
callback.on_crew_start(crew)
callback.on_task_start(task)
callback.on_tool_use(agent, "web_search", {"query": "..."}, "results...", duration_ms=500)
callback.on_delegation(from_agent, to_agent, task)
callback.on_task_end(task, "Output here")
callback.on_crew_end(crew, "Final result")

callback.complete("success")
callback.save()
```

**Captures:** Crew execution, task assignment/completion, tool use, inter-agent delegation, LLM calls, errors.

---

## What Gets Captured

### Per LLM Call
| Field | Description |
|-------|-------------|
| Provider + model | `anthropic/claude-opus-4-6`, `openai/gpt-4o` |
| System prompt | Full system instructions |
| Input messages | All user/assistant/tool messages |
| Model config | temperature, max_tokens, top_p |
| Available tools | Tool definitions sent to the model |
| Response blocks | text, tool_use, thinking — all content types |
| Internal reasoning | Claude extended thinking, chain-of-thought |
| Stop reason | end_turn, tool_use, max_tokens |
| Token usage | input, output, cache, thinking tokens |
| Cost estimate | Built-in pricing for all major models |
| HTTP details | URL, method, status, request-id, rate limits |
| Raw bodies | Full JSON request/response |

### Per Tool Call
| Field | Description |
|-------|-------------|
| Tool name + type | function, mcp, web_search, code_execution |
| Input parameters | Full arguments |
| Output/result | Return value or error |
| Status | success, error, timeout, denied |
| Duration | Execution time in ms |
| MCP server | URL and name for MCP tools |
| Parent link | Which LLM call requested this tool |

### Other Step Types
| Type | What it captures |
|------|-----------------|
| `thought` | Agent internal reasoning |
| `agent_decision` | Choice points with alternatives and reasoning |
| `tool_result` | Tool output linked to parent tool_call |
| `file_operation` | read/write/create/delete with content preview |
| `human_input` | Approval, correction, cancellation |
| `error` | Code, message, recoverability, recovery action |
| `retry` | Retry attempts |
| `screenshot` | Captured UI state |
| `handoff` | Agent-to-agent delegation |

---

## Trajectory Format

Open spec. JSON-based. Vendor-neutral. See [`spec/trajectory.schema.json`](spec/trajectory.schema.json).

```json
{
  "version": "0.1.0",
  "id": "uuid",
  "metadata": {
    "title": "Research cloud providers",
    "tags": ["research"],
    "created_at": "2026-03-14T00:00:00Z"
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
        "provider": "anthropic",
        "system_prompt": "You are a research assistant.",
        "config": { "temperature": 0.3, "max_tokens": 4096 },
        "available_tools": [{ "name": "web_search" }],
        "response_blocks": [
          { "type": "thinking", "text": "I should compare prices..." },
          { "type": "text", "text": "Let me search for that." },
          { "type": "tool_use", "tool_name": "web_search", "input": { "query": "H100 pricing" } }
        ],
        "thinking": "I should compare on-demand vs spot pricing...",
        "http": {
          "url": "https://api.anthropic.com/v1/messages",
          "status_code": 200,
          "response_headers": { "request-id": "req_abc123" }
        }
      },
      "tokens": { "input_tokens": 1847, "output_tokens": 312 },
      "cost_usd": 0.0102,
      "duration_ms": 2340
    },
    {
      "type": "tool_call",
      "parent_step_id": 1,
      "tool_call": {
        "name": "web_search",
        "tool_type": "web_search",
        "input": { "query": "H100 pricing" },
        "status": "success"
      }
    },
    {
      "type": "tool_result",
      "parent_step_id": 2,
      "tool_result": {
        "tool_name": "web_search",
        "output": "AWS p5.48xlarge: $98.32/hr..."
      }
    }
  ],
  "outcome": { "status": "success", "summary": "Generated report" },
  "stats": {
    "total_steps": 4,
    "total_llm_calls": 1,
    "total_tool_calls": 2,
    "total_cost_usd": 0.0102,
    "total_duration_ms": 3400
  }
}
```

---

## Web Viewer

A Next.js app for browsing trajectories in the browser.

```bash
cd web
npm install
npm run dev
# Open http://localhost:3000
```

**Features:**
- Visual swim-lane timeline (click to jump to step)
- Expandable step detail (messages, thinking, tools, HTTP, raw JSON)
- Markdown rendering in responses
- Collapsible JSON tree viewer (like Firefox DevTools)
- Search across all steps (Cmd+F)
- Replay/debugger mode (play/pause/step, keyboard shortcuts)
- Cost waterfall chart
- Smart analysis (retry loops, token waste, slow steps, error recovery)
- Export standalone HTML
- Side-by-side trajectory comparison

---

## Architecture

```
┌─────────────────┐     ┌───────────────────────┐     ┌──────────────────┐
│   Your Agent     │────▶│   agentreel proxy     │────▶│  LLM APIs        │
│   (any framework)│◀────│                       │◀────│  OpenAI/Anthropic│
└─────────────────┘     │  Auto-detects provider │     └──────────────────┘
                        │  Captures everything   │
                        │  Extracts tool calls   │
                        │  Records errors (429)  │
                        │  Estimates costs       │
                        └───────────┬───────────┘
                                    │
                           ┌────────▼─────────┐
                           │ .trajectory.json  │
                           └────────┬──────────┘
                                    │
              ┌─────────────────────┼────────────────────┐
              │              │              │              │
       ┌──────▼──────┐ ┌────▼─────┐ ┌─────▼─────┐ ┌─────▼─────┐
       │  view/replay │ │diff/     │ │list/redact│ │  web UI   │
       │  stats       │ │compare   │ │fork/      │ │  (Next.js)│
       └─────────────┘ └──────────┘ │validate   │ └───────────┘
                                    └───────────┘
```

### Project Structure

```
agentreel/
├── spec/                          # Trajectory format specification
│   ├── trajectory.schema.json     # JSON Schema
│   └── example.trajectory.json    # Example file
├── crates/
│   ├── agentreel-core/            # Rust: types, diff, redact, config, costs
│   ├── agentreel-cli/             # Rust: 9 CLI commands
│   └── agentreel-proxy/           # Rust: HTTP proxy, recorder, cost estimation
├── sdks/
│   ├── python/
│   │   ├── agentreel/             # Core SDK: Tracer, Recorder, types
│   │   │   ├── adapters/          # Framework integrations
│   │   │   │   ├── langchain.py   # LangChain CallbackHandler
│   │   │   │   ├── openai_agents.py # OpenAI Agents SDK TracingProcessor
│   │   │   │   └── crewai.py      # CrewAI callbacks
│   │   │   ├── tracer.py          # Main Tracer class
│   │   │   ├── recorder.py        # Manual Recorder
│   │   │   ├── trajectory.py      # Data types
│   │   │   ├── costs.py           # Model pricing
│   │   │   └── redact.py          # Secret redaction
│   │   └── tests/
│   └── typescript/                # TypeScript SDK
│       └── src/
├── web/                           # Next.js viewer
│   └── src/
│       ├── components/            # Timeline, Replay, CostWaterfall, etc.
│       └── lib/                   # Types, utilities
├── .github/workflows/
│   ├── ci.yml                     # Test on every push/PR
│   └── release.yml                # Build binaries on tag
├── install.sh                     # One-line installer
└── LICENSE (MIT)
```

---

## Configuration

Create `~/.agentreel/config.toml`:

```toml
# Where trajectories are stored
trajectory_dir = "~/.agentreel/trajectories"

# Auto-redact secrets before saving
redact_by_default = true

# Default tags for every trajectory
default_tags = ["my-project"]

# Registry URL for push/pull (future)
# registry_url = "https://registry.agentreel.dev"

[proxy]
# Fixed proxy port (default: random)
# port = 8080

# Auto-detect OpenAI vs Anthropic
auto_detect_provider = true

# Override upstream URLs
# openai_upstream = "https://api.openai.com"
# anthropic_upstream = "https://api.anthropic.com"

# Custom model costs (USD per 1M tokens)
# [model_costs.my-custom-model]
# input = 5.0
# output = 15.0
```

Config is loaded from: defaults → `~/.agentreel/config.toml` → `./agentreel.toml` → environment variables.

Environment variables:
- `AGENTREEL_TRAJECTORY_DIR` — override trajectory directory
- `AGENTREEL_REGISTRY_URL` — registry URL
- `AGENTREEL_OPENAI_UPSTREAM` — OpenAI upstream URL
- `AGENTREEL_ANTHROPIC_UPSTREAM` — Anthropic upstream URL

---

## Model Pricing

Built-in cost estimation for all major models (March 2026):

| Model | Input ($/1M) | Output ($/1M) |
|-------|------------:|-------------:|
| Claude Opus 4.6 | $15.00 | $75.00 |
| Claude Sonnet 4.6 | $3.00 | $15.00 |
| Claude Haiku 4.5 | $0.80 | $4.00 |
| GPT-4.5 Turbo | $5.00 | $15.00 |
| GPT-4o | $2.50 | $10.00 |
| GPT-4o Mini | $0.15 | $0.60 |
| o3 | $10.00 | $40.00 |
| o3-mini / o4-mini | $1.10 | $4.40 |
| Gemini 2.5 Pro | $1.25 | $10.00 |
| Gemini 2.5 Flash | $0.15 | $0.60 |
| DeepSeek R1 | $0.55 | $2.19 |
| DeepSeek V3 | $0.27 | $1.10 |

Add custom models in config:

```toml
[model_costs.my-fine-tuned-model]
input = 8.0
output = 24.0
```

---

## Contributing

MIT licensed. Contributions welcome:

- **Trajectory spec** — propose additions to the format
- **Framework adapters** — add support for new frameworks
- **CLI features** — new commands or improvements
- **Redaction patterns** — new secret patterns to detect
- **Cost data** — updated model pricing
- **Bug reports** — file issues

```bash
# Development setup
git clone https://github.com/deepmehta11/agentreel.git
cd agentreel

# Run Rust tests
cargo test --workspace

# Run Python tests
cd sdks/python && pip install -e . && pytest tests/ -v

# Run TypeScript tests
cd sdks/typescript && npm install && npx tsc && node --test dist/test/trajectory.test.js

# Run web viewer
cd web && npm install && npm run dev
```

---

## Roadmap

- [ ] `agentreel push` / `agentreel pull` — share trajectories to a public registry
- [ ] Homebrew tap — `brew install agentreel`
- [ ] PyPI package — `pip install agentreel`
- [ ] npm package — `npx agentreel`
- [ ] More framework adapters (AutoGen, Semantic Kernel, Google ADK)
- [ ] Live recording dashboard (watch steps appear in real-time)
- [ ] VS Code extension
- [ ] OpenTelemetry export

---

## License

MIT. See [LICENSE](LICENSE).

---

**Built by [Deep Mehta](https://github.com/deepmehta11)**

*"Every agent run should be as inspectable as a git commit."*
