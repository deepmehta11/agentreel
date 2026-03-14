"""Recorder — wraps LLM API calls and builds a trajectory automatically."""

from __future__ import annotations

import time
from datetime import datetime, timezone
from typing import Any

from agentreel.trajectory import (
    LlmCall,
    Message,
    Step,
    TokenUsage,
    ToolCall,
    Trajectory,
)


class Recorder:
    """Records LLM interactions into a Trajectory.

    Usage:
        recorder = Recorder(title="My agent run")

        # Record an LLM call
        recorder.record_llm_call(
            model="gpt-4o",
            messages=[{"role": "user", "content": "Hello"}],
            response={"choices": [{"message": {"role": "assistant", "content": "Hi!"}}]},
            duration_ms=500,
        )

        # Get the trajectory
        trajectory = recorder.finalize()
        trajectory.save("my_run.trajectory.json")
    """

    def __init__(
        self,
        title: str | None = None,
        tags: list[str] | None = None,
        trajectory: Trajectory | None = None,
    ):
        self.trajectory = trajectory or Trajectory()
        if title:
            self.trajectory.metadata.title = title
        if tags:
            self.trajectory.metadata.tags = tags

    def record_llm_call(
        self,
        model: str,
        messages: list[dict[str, Any]],
        response: dict[str, Any],
        duration_ms: float | None = None,
    ) -> int:
        """Record an LLM API call and return the step index."""
        # Extract response content
        resp_content = _extract_response_content(response)
        tokens = _extract_tokens(response)
        stop_reason = _extract_stop_reason(response)

        step = Step(
            type="llm_call",
            timestamp=_now_iso(),
            duration_ms=duration_ms,
            llm_call=LlmCall(
                model=model,
                messages=[Message(role=m.get("role", "user"), content=m.get("content")) for m in messages],
                response=Message(role="assistant", content=resp_content),
                stop_reason=stop_reason,
            ),
            tokens=tokens,
        )

        return self.trajectory.add_step(step)

    def record_tool_call(
        self,
        name: str,
        input: dict[str, Any] | None = None,
        output: Any = None,
        status: str = "success",
        duration_ms: float | None = None,
    ) -> int:
        """Record a tool call and return the step index."""
        step = Step(
            type="tool_call",
            timestamp=_now_iso(),
            duration_ms=duration_ms,
            tool_call=ToolCall(
                name=name,
                input=input,
                output=output,
                status=status,
            ),
        )
        return self.trajectory.add_step(step)

    def record_error(
        self,
        message: str,
        code: str | None = None,
        recoverable: bool = True,
    ) -> int:
        """Record an error and return the step index."""
        from agentreel.trajectory import Error

        step = Step(
            type="error",
            timestamp=_now_iso(),
            error=Error(code=code, message=message, recoverable=recoverable),
        )
        return self.trajectory.add_step(step)

    def finalize(self, status: str = "success", summary: str | None = None) -> Trajectory:
        """Finalize and return the trajectory with computed stats."""
        from agentreel.trajectory import Outcome

        self.trajectory.metadata.completed_at = _now_iso()
        self.trajectory.outcome = Outcome(status=status, summary=summary)
        self.trajectory.compute_stats()
        return self.trajectory


def _extract_response_content(response: dict[str, Any]) -> str | None:
    # OpenAI format
    choices = response.get("choices", [])
    if choices:
        msg = choices[0].get("message", {})
        return msg.get("content")

    # Anthropic format
    content = response.get("content", [])
    if content:
        texts = [b["text"] for b in content if b.get("type") == "text" and "text" in b]
        return "\n".join(texts) if texts else None

    return None


def _extract_tokens(response: dict[str, Any]) -> TokenUsage | None:
    usage = response.get("usage")
    if not usage:
        return None

    return TokenUsage(
        input_tokens=usage.get("input_tokens") or usage.get("prompt_tokens"),
        output_tokens=usage.get("output_tokens") or usage.get("completion_tokens"),
        cache_read_tokens=usage.get("cache_read_input_tokens"),
        cache_write_tokens=usage.get("cache_creation_input_tokens"),
    )


def _extract_stop_reason(response: dict[str, Any]) -> str | None:
    # Anthropic
    if reason := response.get("stop_reason"):
        return reason
    # OpenAI
    choices = response.get("choices", [])
    if choices:
        reason = choices[0].get("finish_reason")
        if reason == "stop":
            return "end_turn"
        if reason == "tool_calls":
            return "tool_use"
        if reason == "length":
            return "max_tokens"
        return reason
    return None


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()
