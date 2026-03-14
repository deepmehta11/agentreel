"""Tracer — wraps LLM clients and tool executors to automatically capture every call."""

from __future__ import annotations

import functools
import time
from datetime import datetime, timezone
from typing import Any

from agentreel.costs import estimate_cost
from agentreel.trajectory import (
    AgentDecision,
    Error,
    FileOperation,
    HumanInput,
    Input,
    LlmCall,
    Message,
    Outcome,
    Step,
    Thought,
    TokenUsage,
    ToolCall,
    ToolResult,
    Trajectory,
)


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


class Tracer:
    """Main tracing interface. Wraps LLM clients and tool executors.

    Usage:
        tracer = Tracer(title="Build a todo app")

        # Wrap LLM client — all API calls captured
        client = tracer.wrap_openai(openai.OpenAI())
        # OR
        client = tracer.wrap_anthropic(anthropic.Anthropic())

        # Wrap tool executor — all tool executions captured
        execute_tool = tracer.wrap_tool_executor(my_tool_executor)

        # Manual logging
        tracer.log_thought("Let me think about this...")
        tracer.log_decision("Use React", alternatives=["Vue", "Svelte"])
        tracer.log_file_op("write", "/tmp/app.py", content_preview="import flask...")

        # Finalize
        tracer.complete("success", summary="Built the todo app")
        tracer.save("my_run.trajectory.json")
    """

    def __init__(
        self,
        title: str = "",
        framework: str = "custom",
        agent_name: str = "",
        task_prompt: str = "",
        tags: list[str] | None = None,
    ):
        self.trajectory = Trajectory()
        self.trajectory.metadata.title = title
        self.trajectory.metadata.tags = tags or []
        if task_prompt or title:
            self.trajectory.input = Input(prompt=task_prompt or title)
        self._step_counter = 0

    def _add_step(self, step_type: str, **kwargs) -> int:
        idx = self._step_counter
        step = Step(
            index=idx,
            type=step_type,
            timestamp=_now_iso(),
            **{k: v for k, v in kwargs.items() if v is not None},
        )
        self.trajectory.steps.append(step)
        self._step_counter += 1
        return idx

    # ── LLM Client Wrapping ──────────────────────────────────

    def wrap_openai(self, client: Any) -> Any:
        """Wrap an OpenAI client to trace all chat completion calls."""
        original_create = client.chat.completions.create
        tracer = self

        @functools.wraps(original_create)
        def traced_create(*args, **kwargs):
            start = time.time()
            messages = kwargs.get("messages", args[0] if args else [])
            model = kwargs.get("model", "unknown")

            try:
                response = original_create(*args, **kwargs)
                duration_ms = (time.time() - start) * 1000

                usage = getattr(response, "usage", None)
                input_tokens = getattr(usage, "prompt_tokens", 0) if usage else 0
                output_tokens = getattr(usage, "completion_tokens", 0) if usage else 0

                resp_text = ""
                finish_reason = None
                if hasattr(response, "choices") and response.choices:
                    resp_text = getattr(response.choices[0].message, "content", "") or ""
                    finish_reason = getattr(response.choices[0], "finish_reason", None)

                tracer._add_step(
                    "llm_call",
                    duration_ms=duration_ms,
                    llm_call=LlmCall(
                        model=model,
                        messages=[
                            Message(
                                role=m.get("role", "user") if isinstance(m, dict) else getattr(m, "role", "user"),
                                content=str(m.get("content", "") if isinstance(m, dict) else getattr(m, "content", ""))[:2000],
                            )
                            for m in (messages if isinstance(messages, list) else [])
                        ],
                        response=Message(role="assistant", content=resp_text[:2000]),
                        stop_reason=_map_openai_stop(finish_reason),
                    ),
                    tokens=TokenUsage(input_tokens=input_tokens, output_tokens=output_tokens),
                    cost_usd=estimate_cost(model, input_tokens, output_tokens),
                )

                # Extract tool_use from response
                if hasattr(response, "choices") and response.choices:
                    msg = response.choices[0].message
                    if hasattr(msg, "tool_calls") and msg.tool_calls:
                        for tc in msg.tool_calls:
                            tracer.log_tool_call(
                                tc.function.name,
                                input_data={"arguments": tc.function.arguments},
                                tool_type="function",
                            )

                return response

            except Exception as e:
                duration_ms = (time.time() - start) * 1000
                tracer.log_error(str(e), code=type(e).__name__)
                raise

        client.chat.completions.create = traced_create
        return client

    def wrap_anthropic(self, client: Any) -> Any:
        """Wrap an Anthropic client to trace all message creation calls."""
        original_create = client.messages.create
        tracer = self

        @functools.wraps(original_create)
        def traced_create(*args, **kwargs):
            start = time.time()
            messages = kwargs.get("messages", [])
            model = kwargs.get("model", "unknown")
            system = kwargs.get("system", "")

            try:
                response = original_create(*args, **kwargs)
                duration_ms = (time.time() - start) * 1000

                input_tokens = getattr(response.usage, "input_tokens", 0)
                output_tokens = getattr(response.usage, "output_tokens", 0)

                resp_text = ""
                thinking_text = ""
                if hasattr(response, "content"):
                    for block in response.content:
                        if hasattr(block, "type") and block.type == "thinking":
                            thinking_text += getattr(block, "thinking", "")
                        elif hasattr(block, "text"):
                            resp_text += block.text

                tracer._add_step(
                    "llm_call",
                    duration_ms=duration_ms,
                    llm_call=LlmCall(
                        model=model,
                        messages=[
                            Message(
                                role=m.get("role", "user"),
                                content=str(m.get("content", ""))[:2000],
                            )
                            for m in messages
                        ],
                        response=Message(role="assistant", content=resp_text[:2000]),
                        stop_reason=getattr(response, "stop_reason", None),
                    ),
                    tokens=TokenUsage(input_tokens=input_tokens, output_tokens=output_tokens),
                    cost_usd=estimate_cost(model, input_tokens, output_tokens),
                )

                # Capture thinking
                if thinking_text:
                    tracer.log_thought(thinking_text)

                # Extract tool_use blocks
                if hasattr(response, "content"):
                    for block in response.content:
                        if hasattr(block, "type") and block.type == "tool_use":
                            tracer.log_tool_call(
                                block.name,
                                input_data=block.input if isinstance(block.input, dict) else {},
                                tool_type="function",
                            )

                return response

            except Exception as e:
                duration_ms = (time.time() - start) * 1000
                tracer.log_error(str(e), code=type(e).__name__)
                raise

        client.messages.create = traced_create
        return client

    # ── Tool Execution Wrapping ──────────────────────────────

    def wrap_tool_executor(self, executor_fn: Any) -> Any:
        """Wrap a tool execution function to capture every tool call.

        The executor_fn should have signature: (tool_name, tool_input, **kwargs) -> result

        Usage:
            def my_executor(tool_name, tool_input):
                if tool_name == "bash":
                    return subprocess.run(tool_input["command"], capture_output=True).stdout
                elif tool_name == "read_file":
                    return open(tool_input["path"]).read()

            traced_executor = tracer.wrap_tool_executor(my_executor)
            result = traced_executor("bash", {"command": "ls -la"})
            # ^ automatically captured in trajectory
        """
        tracer = self

        @functools.wraps(executor_fn)
        def traced_executor(tool_name: str, tool_input: dict | None = None, **kwargs):
            start = time.time()
            call_idx = tracer.log_tool_call(tool_name, input_data=tool_input or {})

            try:
                result = executor_fn(tool_name, tool_input, **kwargs)
                duration_ms = (time.time() - start) * 1000

                tracer.log_tool_result(
                    tool_name,
                    output=result,
                    parent_step_id=call_idx,
                    duration_ms=duration_ms,
                )

                # Update the tool_call step duration
                if call_idx < len(tracer.trajectory.steps):
                    tracer.trajectory.steps[call_idx].duration_ms = duration_ms

                return result

            except Exception as e:
                duration_ms = (time.time() - start) * 1000
                tracer.log_tool_result(
                    tool_name,
                    output=str(e),
                    parent_step_id=call_idx,
                    is_error=True,
                    duration_ms=duration_ms,
                )
                if call_idx < len(tracer.trajectory.steps):
                    tracer.trajectory.steps[call_idx].duration_ms = duration_ms
                raise

        return traced_executor

    def wrap_function(self, tool_name: str | None = None):
        """Decorator to trace any function as a tool call.

        Usage:
            @tracer.wrap_function("web_search")
            def search(query: str) -> str:
                return requests.get(f"https://api.example.com/search?q={query}").text

            result = search("hello world")  # automatically traced
        """
        tracer = self

        def decorator(fn):
            name = tool_name or fn.__name__

            @functools.wraps(fn)
            def wrapper(*args, **kwargs):
                start = time.time()
                input_data = kwargs if kwargs else ({"args": list(args)} if args else {})
                call_idx = tracer.log_tool_call(name, input_data=input_data)

                try:
                    result = fn(*args, **kwargs)
                    duration_ms = (time.time() - start) * 1000
                    output_str = str(result)[:2000] if result is not None else None
                    tracer.log_tool_result(name, output=output_str, parent_step_id=call_idx, duration_ms=duration_ms)
                    if call_idx < len(tracer.trajectory.steps):
                        tracer.trajectory.steps[call_idx].duration_ms = duration_ms
                    return result
                except Exception as e:
                    duration_ms = (time.time() - start) * 1000
                    tracer.log_tool_result(name, output=str(e), parent_step_id=call_idx, is_error=True, duration_ms=duration_ms)
                    raise

            return wrapper

        return decorator

    # ── Manual Logging ───────────────────────────────────────

    def log_tool_call(
        self,
        tool_name: str,
        input_data: dict | None = None,
        tool_type: str = "function",
        mcp_server: str | None = None,
        mcp_server_name: str | None = None,
    ) -> int:
        """Log a tool call. Returns step index for linking result."""
        return self._add_step(
            "tool_call",
            tool_call=ToolCall(
                name=tool_name,
                tool_type=tool_type,
                input=input_data,
                status="pending",
                mcp_server=mcp_server,
                mcp_server_name=mcp_server_name,
            ),
        )

    def log_tool_result(
        self,
        tool_name: str,
        output: Any = None,
        parent_step_id: int | None = None,
        is_error: bool = False,
        duration_ms: float | None = None,
    ) -> int:
        """Log the result of a tool call."""
        # Update parent tool_call status
        if parent_step_id is not None and parent_step_id < len(self.trajectory.steps):
            parent = self.trajectory.steps[parent_step_id]
            if parent.tool_call:
                parent.tool_call.status = "error" if is_error else "success"
                parent.tool_call.output = output

        return self._add_step(
            "tool_result",
            parent_step_id=parent_step_id,
            duration_ms=duration_ms,
            tool_result=ToolResult(
                tool_name=tool_name,
                output=str(output)[:2000] if output is not None else None,
                is_error=is_error,
                error_message=str(output)[:500] if is_error else None,
            ),
        )

    def log_thought(self, content: str, thinking_tokens: int | None = None) -> int:
        """Log agent internal reasoning / chain-of-thought."""
        return self._add_step(
            "thought",
            thought=Thought(content=content, thinking_tokens=thinking_tokens),
        )

    def log_decision(
        self,
        decision: str,
        alternatives: list[str] | None = None,
        reasoning: str = "",
        confidence: float | None = None,
    ) -> int:
        """Log an agent decision point with alternatives considered."""
        return self._add_step(
            "agent_decision",
            agent_decision=AgentDecision(
                decision=decision,
                alternatives_considered=alternatives or [],
                reasoning=reasoning or None,
                confidence=confidence,
            ),
        )

    def log_error(
        self,
        message: str,
        code: str | None = None,
        recoverable: bool = True,
        recovery_action: str | None = None,
    ) -> int:
        """Log an error."""
        return self._add_step(
            "error",
            error=Error(
                code=code,
                message=message,
                recoverable=recoverable,
                recovery_action=recovery_action,
            ),
        )

    def log_human_input(self, content: str, action: str = "message") -> int:
        """Log human-in-the-loop interaction."""
        return self._add_step(
            "human_input",
            human_input=HumanInput(content=content, action=action),
        )

    def log_file_op(
        self,
        operation: str,
        path: str,
        content_preview: str | None = None,
        size_bytes: int | None = None,
    ) -> int:
        """Log a file operation (read/write/create/delete)."""
        return self._add_step(
            "file_operation",
            file_operation=FileOperation(
                operation=operation,
                path=path,
                content_preview=content_preview[:500] if content_preview else None,
                size_bytes=size_bytes,
            ),
        )

    # ── MCP Tracing ──────────────────────────────────────────

    def log_mcp_call(
        self,
        tool_name: str,
        mcp_server: str,
        input_data: dict | None = None,
        server_name: str = "",
    ) -> int:
        """Log an MCP tool call."""
        return self.log_tool_call(
            tool_name,
            input_data=input_data,
            tool_type="mcp",
            mcp_server=mcp_server,
            mcp_server_name=server_name,
        )

    def log_mcp_result(
        self,
        tool_name: str,
        output: Any = None,
        parent_step_id: int | None = None,
        is_error: bool = False,
        mcp_server: str = "",
    ) -> int:
        """Log the result of an MCP tool call."""
        return self.log_tool_result(
            tool_name, output=output, parent_step_id=parent_step_id, is_error=is_error,
        )

    def wrap_mcp_client(self, mcp_client: Any, server_name: str = "", server_url: str = "") -> Any:
        """Wrap an MCP client to automatically trace all tool calls."""
        tracer = self

        if hasattr(mcp_client, "call_tool"):
            original_call = mcp_client.call_tool

            @functools.wraps(original_call)
            async def traced_call_tool(name, arguments=None, **kwargs):
                start = time.time()
                step_id = tracer.log_mcp_call(
                    tool_name=name, mcp_server=server_url,
                    input_data=arguments or {}, server_name=server_name,
                )
                try:
                    result = await original_call(name, arguments, **kwargs)
                    duration_ms = (time.time() - start) * 1000
                    output = ""
                    if hasattr(result, "content"):
                        for block in result.content:
                            if hasattr(block, "text"):
                                output += block.text
                    else:
                        output = str(result)
                    tracer.log_mcp_result(name, output=output, parent_step_id=step_id, mcp_server=server_url)
                    if step_id < len(tracer.trajectory.steps):
                        tracer.trajectory.steps[step_id].duration_ms = duration_ms
                    return result
                except Exception as e:
                    tracer.log_mcp_result(name, output=str(e), parent_step_id=step_id, is_error=True, mcp_server=server_url)
                    raise

            mcp_client.call_tool = traced_call_tool

        return mcp_client

    # ── Finalization ─────────────────────────────────────────

    def complete(self, outcome: str = "success", summary: str | None = None) -> str:
        """Mark trajectory as complete."""
        self.trajectory.metadata.completed_at = _now_iso()
        self.trajectory.outcome = Outcome(status=outcome, summary=summary)
        self.trajectory.compute_stats()
        return self.trajectory.id

    def save(self, path: str | None = None) -> str:
        """Save trajectory to disk."""
        if path is None:
            from pathlib import Path
            output_dir = Path.home() / ".agentreel" / "trajectories"
            output_dir.mkdir(parents=True, exist_ok=True)
            path = str(output_dir / f"{self.trajectory.id}.trajectory.json")
        self.trajectory.save(path)
        return path


def _map_openai_stop(reason: str | None) -> str | None:
    if reason == "stop":
        return "end_turn"
    if reason == "tool_calls":
        return "tool_use"
    if reason == "length":
        return "max_tokens"
    return reason
