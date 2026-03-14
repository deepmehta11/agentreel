# AgentReel Examples

Real agent scripts you can run to see AgentReel in action.

## Prerequisites

```bash
# Install the CLI
cargo install --path crates/agentreel-cli

# Set your API key (pick one)
export ANTHROPIC_API_KEY="your-key"
# or
export OPENAI_API_KEY="your-key"
```

## Examples

### 1. Simple Chat Agent (zero code changes)

A basic agent that has a multi-turn conversation. Shows proxy-based recording.

```bash
agentreel record -t "Chat Agent" --tags demo,chat -- python examples/01_chat_agent.py
agentreel view trajectory.json --full
agentreel stats trajectory.json
```

### 2. Tool-Using Agent (proxy captures tool calls)

An agent that uses tools (calculator, web search). The proxy auto-extracts tool_use blocks.

```bash
agentreel record -t "Tool Agent" --tags demo,tools -- python examples/02_tool_agent.py
agentreel view trajectory.json --full
```

### 3. SDK-Instrumented Agent (captures everything)

Uses the Python Tracer SDK to capture LLM calls, tool execution, thoughts, and decisions.

```bash
python examples/03_sdk_agent.py
agentreel view ~/.agentreel/trajectories/*.json --full
```

### 4. Multi-Model Comparison

Runs the same task on Claude and GPT, then compares.

```bash
python examples/04_compare_models.py
agentreel compare claude_run.json gpt_run.json --format markdown
```
