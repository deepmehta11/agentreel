#!/usr/bin/env python3
"""
Example 2: Tool-Using Agent

An agent that uses tools to solve a problem. The proxy auto-extracts
tool_use blocks from Claude's response as separate ToolCall steps.

    agentreel record -t "Tool Agent" --tags demo,tools -- python examples/02_tool_agent.py
    agentreel view trajectory.json --full

You'll see:
  🧠 Step 0 — LLM call (decides to use tool)
  🔧 Step 1 — calculator (auto-extracted from response)
  🧠 Step 2 — LLM call (processes tool result)
"""

import json
import os
import urllib.request

BASE_URL = os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com")
API_KEY = os.environ.get("ANTHROPIC_API_KEY", "")

if not API_KEY:
    print("Set ANTHROPIC_API_KEY environment variable")
    exit(1)

TOOLS = [
    {
        "name": "calculator",
        "description": "Evaluate a math expression. Use this for any arithmetic.",
        "input_schema": {
            "type": "object",
            "properties": {
                "expression": {"type": "string", "description": "Math expression (e.g., '47 * 89 + 123')"}
            },
            "required": ["expression"],
        },
    },
    {
        "name": "get_weather",
        "description": "Get the current weather for a city.",
        "input_schema": {
            "type": "object",
            "properties": {
                "city": {"type": "string", "description": "City name"}
            },
            "required": ["city"],
        },
    },
]


def call_claude(messages, system=None):
    body = {
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 1024,
        "temperature": 0.3,
        "messages": messages,
        "tools": TOOLS,
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


def execute_tool(name, input_data):
    """Execute a tool and return the result."""
    if name == "calculator":
        try:
            result = eval(input_data["expression"])  # safe for demo
            return str(result)
        except Exception as e:
            return f"Error: {e}"
    elif name == "get_weather":
        # Simulated weather data
        weather = {
            "san francisco": "Foggy, 58°F",
            "new york": "Sunny, 72°F",
            "london": "Rainy, 55°F",
            "tokyo": "Clear, 68°F",
        }
        city = input_data["city"].lower()
        return weather.get(city, f"Weather data not available for {input_data['city']}")
    return f"Unknown tool: {name}"


def agent_loop(user_message):
    """Run the agent loop — handles tool calls automatically."""
    messages = [{"role": "user", "content": user_message}]

    while True:
        resp = call_claude(messages, system="You are a helpful assistant. Use tools when needed.")

        # Check if the response has tool_use blocks
        tool_uses = [b for b in resp.get("content", []) if b.get("type") == "tool_use"]

        if not tool_uses:
            # No tool use — return the text response
            text = "".join(b["text"] for b in resp.get("content", []) if b.get("type") == "text")
            return text

        # Execute tools and continue
        messages.append({"role": "assistant", "content": resp["content"]})

        tool_results = []
        for tool_use in tool_uses:
            print(f"  🔧 Executing: {tool_use['name']}({json.dumps(tool_use['input'])})")
            result = execute_tool(tool_use["name"], tool_use["input"])
            print(f"     Result: {result}")
            tool_results.append({
                "type": "tool_result",
                "tool_use_id": tool_use["id"],
                "content": result,
            })

        messages.append({"role": "user", "content": tool_results})


print("=== Tool Agent Demo ===\n")

# Task 1: Math with calculator
print("Task 1: Complex math")
print("User: What is (47 * 89) + (123 * 456) - 7890?")
result = agent_loop("What is (47 * 89) + (123 * 456) - 7890? Use the calculator.")
print(f"Agent: {result}\n")

# Task 2: Weather
print("Task 2: Weather check")
print("User: What's the weather in Tokyo and San Francisco?")
result = agent_loop("What's the weather in Tokyo and San Francisco? Check both.")
print(f"Agent: {result}\n")

print("=== Done ===")
