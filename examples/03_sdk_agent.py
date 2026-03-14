#!/usr/bin/env python3
"""
Example 3: SDK-Instrumented Agent

Uses the Python Tracer SDK to capture EVERYTHING — LLM calls, tool
execution, thoughts, decisions, file operations, and errors.

    python examples/03_sdk_agent.py
    agentreel view agent_run.trajectory.json --full

This shows the full power of SDK instrumentation vs proxy-only recording.
"""

import json
import os
import subprocess
import urllib.request

# Import AgentReel SDK
from agentreel import Tracer

API_KEY = os.environ.get("ANTHROPIC_API_KEY", "")
if not API_KEY:
    print("Set ANTHROPIC_API_KEY environment variable")
    exit(1)

# ── Initialize Tracer ────────────────────────────────────────

tracer = Tracer(
    title="Code Generation Agent",
    tags=["demo", "sdk", "coding"],
    task_prompt="Write a Python function to check if a number is prime, test it, and save to a file.",
)

# ── Agent Logic ──────────────────────────────────────────────


def call_claude(messages, system=None):
    """Call Claude API directly (in production, use tracer.wrap_anthropic())."""
    body = {
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "messages": messages,
    }
    if system:
        body["system"] = system

    start = __import__("time").time()
    req = urllib.request.Request(
        "https://api.anthropic.com/v1/messages",
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "x-api-key": API_KEY,
            "anthropic-version": "2023-06-01",
        },
    )
    with urllib.request.urlopen(req) as resp:
        result = json.loads(resp.read())

    duration_ms = (__import__("time").time() - start) * 1000
    text = "".join(b["text"] for b in result.get("content", []) if b.get("type") == "text")
    usage = result.get("usage", {})

    # Record LLM call in trajectory
    from agentreel.trajectory import LlmCall, Message, TokenUsage
    from agentreel.costs import estimate_cost

    tracer._add_step(
        "llm_call",
        duration_ms=duration_ms,
        llm_call=LlmCall(
            model="claude-sonnet-4-20250514",
            messages=[Message(role=m["role"], content=str(m["content"])[:2000]) for m in messages],
            response=Message(role="assistant", content=text[:2000]),
            stop_reason=result.get("stop_reason"),
        ),
        tokens=TokenUsage(
            input_tokens=usage.get("input_tokens", 0),
            output_tokens=usage.get("output_tokens", 0),
        ),
        cost_usd=estimate_cost("claude-sonnet-4-20250514", usage.get("input_tokens", 0), usage.get("output_tokens", 0)),
    )

    return text


# Wrap tool executor — captures every tool call automatically
def execute_tool(tool_name, tool_input):
    if tool_name == "write_file":
        path = tool_input["path"]
        content = tool_input["content"]
        with open(path, "w") as f:
            f.write(content)
        return f"Written {len(content)} bytes to {path}"
    elif tool_name == "run_python":
        result = subprocess.run(
            ["python3", "-c", tool_input["code"]],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode == 0:
            return result.stdout
        return f"Error (exit {result.returncode}): {result.stderr}"
    return f"Unknown tool: {tool_name}"


traced_execute = tracer.wrap_tool_executor(execute_tool)

# ── Run the Agent ────────────────────────────────────────────

print("=== SDK Agent Demo ===\n")

# Step 1: Think about approach
tracer.log_thought(
    "I need to: 1) Ask Claude to write a prime checker, "
    "2) Save it to a file, 3) Run tests on it."
)

# Step 2: Decide on approach
tracer.log_decision(
    "Write function and tests in one file",
    alternatives=["Separate test file", "Use pytest", "Interactive REPL"],
    reasoning="Single file is simpler for a demo and easier to run",
)

# Step 3: Ask Claude to write code
print("Asking Claude to write a prime checker...")
code = call_claude(
    [{"role": "user", "content": "Write a Python function `is_prime(n)` that checks if a number is prime. Include 5 test assertions at the bottom. Output ONLY the Python code, no explanation."}],
    system="You are a Python expert. Output only valid Python code.",
)
print(f"Generated code:\n{code[:200]}...\n")

# Step 4: Save to file
print("Saving to /tmp/prime_checker.py...")
traced_execute("write_file", {"path": "/tmp/prime_checker.py", "content": code})
tracer.log_file_op("write", "/tmp/prime_checker.py", content_preview=code[:200], size_bytes=len(code))

# Step 5: Run the code
print("Running tests...")
result = traced_execute("run_python", {"code": code})
print(f"Output: {result}")

if "Error" in str(result):
    tracer.log_error(f"Test failed: {result}", code="TEST_FAILURE", recoverable=True)
    print("\nTests failed! Asking Claude to fix...")

    # Step 6: Ask Claude to fix
    fixed_code = call_claude(
        [
            {"role": "user", "content": f"This Python code has errors:\n\n```python\n{code}\n```\n\nError: {result}\n\nFix it. Output ONLY the corrected Python code."},
        ],
    )

    traced_execute("write_file", {"path": "/tmp/prime_checker.py", "content": fixed_code})
    result2 = traced_execute("run_python", {"code": fixed_code})
    print(f"Fixed output: {result2}")

# Step 7: Finalize
tracer.complete("success", summary="Generated and tested a prime checker function")
path = tracer.save("agent_run.trajectory.json")

print(f"\nTrajectory saved to: {path}")
print(f"Steps: {len(tracer.trajectory.steps)}")
print(f"\nView with: agentreel view {path} --full")
print("=== Done ===")
