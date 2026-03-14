#!/usr/bin/env python3
"""
Example 1: Simple Chat Agent

A multi-turn conversation with Claude. Record with zero code changes:

    agentreel record -t "Chat Agent" --tags demo -- python examples/01_chat_agent.py
    agentreel view trajectory.json --full

The proxy captures every LLM call automatically — no SDK needed.
"""

import json
import os
import urllib.request

BASE_URL = os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com")
API_KEY = os.environ.get("ANTHROPIC_API_KEY", "")

if not API_KEY:
    print("Set ANTHROPIC_API_KEY environment variable")
    exit(1)


def call_claude(messages, system=None):
    body = {
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "messages": messages,
    }
    if system:
        body["system"] = system

    req = urllib.request.Request(
        f"{BASE_URL}/v1/messages",
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "x-api-key": API_KEY,
            "anthropic-version": "2023-06-01",
        },
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read())


def get_text(response):
    return "".join(
        b["text"] for b in response.get("content", []) if b.get("type") == "text"
    )


# Multi-turn conversation
print("=== Chat Agent Demo ===\n")

messages = []

# Turn 1
messages.append({"role": "user", "content": "What are the top 3 programming languages for AI development in 2026?"})
print(f"User: {messages[-1]['content']}")
resp = call_claude(messages, system="You are a concise tech advisor. Keep answers under 100 words.")
assistant_text = get_text(resp)
print(f"Claude: {assistant_text}\n")
messages.append({"role": "assistant", "content": assistant_text})

# Turn 2
messages.append({"role": "user", "content": "Why is Rust gaining popularity for AI? Give me 2 reasons."})
print(f"User: {messages[-1]['content']}")
resp = call_claude(messages)
assistant_text = get_text(resp)
print(f"Claude: {assistant_text}\n")
messages.append({"role": "assistant", "content": assistant_text})

# Turn 3
messages.append({"role": "user", "content": "Summarize our conversation in one sentence."})
print(f"User: {messages[-1]['content']}")
resp = call_claude(messages)
assistant_text = get_text(resp)
print(f"Claude: {assistant_text}\n")

print("=== Done ===")
