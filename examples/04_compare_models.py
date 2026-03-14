#!/usr/bin/env python3
"""
Example 4: Multi-Model Comparison

Runs the same task on Claude and GPT, captures both trajectories,
then compares them with the CLI.

    python examples/04_compare_models.py
    agentreel compare claude_run.json gpt_run.json --format markdown

Requires both ANTHROPIC_API_KEY and OPENAI_API_KEY set.
"""

import json
import os
import time
import urllib.request

from agentreel import Tracer
from agentreel.trajectory import LlmCall, Message, TokenUsage
from agentreel.costs import estimate_cost

ANTHROPIC_KEY = os.environ.get("ANTHROPIC_API_KEY", "")
OPENAI_KEY = os.environ.get("OPENAI_API_KEY", "")

TASK = "Explain the difference between concurrency and parallelism in 3 bullet points. Be concise."


def call_anthropic(messages, model="claude-sonnet-4-20250514"):
    body = {
        "model": model,
        "max_tokens": 512,
        "messages": messages,
    }
    req = urllib.request.Request(
        "https://api.anthropic.com/v1/messages",
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "x-api-key": ANTHROPIC_KEY,
            "anthropic-version": "2023-06-01",
        },
    )
    start = time.time()
    with urllib.request.urlopen(req) as resp:
        result = json.loads(resp.read())
    duration = (time.time() - start) * 1000
    return result, duration


def call_openai(messages, model="gpt-4o"):
    body = {
        "model": model,
        "max_tokens": 512,
        "messages": messages,
    }
    req = urllib.request.Request(
        "https://api.openai.com/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {OPENAI_KEY}",
        },
    )
    start = time.time()
    with urllib.request.urlopen(req) as resp:
        result = json.loads(resp.read())
    duration = (time.time() - start) * 1000
    return result, duration


def record_anthropic_call(tracer, messages, model):
    resp, duration = call_anthropic(messages, model)
    text = "".join(b["text"] for b in resp.get("content", []) if b.get("type") == "text")
    usage = resp.get("usage", {})
    inp, out = usage.get("input_tokens", 0), usage.get("output_tokens", 0)

    tracer._add_step(
        "llm_call",
        duration_ms=duration,
        llm_call=LlmCall(
            model=model,
            messages=[Message(role=m["role"], content=m["content"]) for m in messages],
            response=Message(role="assistant", content=text),
            stop_reason=resp.get("stop_reason"),
        ),
        tokens=TokenUsage(input_tokens=inp, output_tokens=out),
        cost_usd=estimate_cost(model, inp, out),
    )
    return text


def record_openai_call(tracer, messages, model):
    resp, duration = call_openai(messages, model)
    text = resp["choices"][0]["message"]["content"]
    usage = resp.get("usage", {})
    inp = usage.get("prompt_tokens", 0)
    out = usage.get("completion_tokens", 0)

    tracer._add_step(
        "llm_call",
        duration_ms=duration,
        llm_call=LlmCall(
            model=model,
            messages=[Message(role=m["role"], content=m["content"]) for m in messages],
            response=Message(role="assistant", content=text),
            stop_reason="end_turn",
        ),
        tokens=TokenUsage(input_tokens=inp, output_tokens=out),
        cost_usd=estimate_cost(model, inp, out),
    )
    return text


print("=== Multi-Model Comparison Demo ===\n")
print(f"Task: {TASK}\n")

messages = [{"role": "user", "content": TASK}]

# ── Run Claude ───────────────────────────────────────────────

if ANTHROPIC_KEY:
    print("Running Claude Sonnet...")
    claude_tracer = Tracer(title="Claude Sonnet: Concurrency vs Parallelism", tags=["comparison", "claude"])
    claude_text = record_anthropic_call(claude_tracer, messages, "claude-sonnet-4-20250514")
    claude_tracer.complete("success")
    claude_tracer.save("claude_run.json")
    print(f"Claude:\n{claude_text}\n")
else:
    print("Skipping Claude (ANTHROPIC_API_KEY not set)\n")

# ── Run GPT ──────────────────────────────────────────────────

if OPENAI_KEY:
    print("Running GPT-4o...")
    gpt_tracer = Tracer(title="GPT-4o: Concurrency vs Parallelism", tags=["comparison", "gpt"])
    gpt_text = record_openai_call(gpt_tracer, messages, "gpt-4o")
    gpt_tracer.complete("success")
    gpt_tracer.save("gpt_run.json")
    print(f"GPT-4o:\n{gpt_text}\n")
else:
    print("Skipping GPT-4o (OPENAI_API_KEY not set)\n")

# ── Compare ──────────────────────────────────────────────────

files = []
if ANTHROPIC_KEY:
    files.append("claude_run.json")
if OPENAI_KEY:
    files.append("gpt_run.json")

if len(files) >= 2:
    print("Compare with:")
    print(f"  agentreel compare {' '.join(files)} --format markdown")
elif len(files) == 1:
    print(f"Only one model available. View with:")
    print(f"  agentreel view {files[0]} --full")
else:
    print("No API keys set. Set ANTHROPIC_API_KEY and/or OPENAI_API_KEY.")

print("\n=== Done ===")
