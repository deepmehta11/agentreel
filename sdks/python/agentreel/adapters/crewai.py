"""CrewAI adapter for AgentReel.

Captures task execution, agent actions, tool calls, and delegation
events into a trajectory.

Usage:
    from agentreel.adapters.crewai import AgentReelCrewCallback
    from crewai import Agent, Task, Crew

    # Create callback
    callback = AgentReelCrewCallback(title="Content creation crew")

    # Use with Crew
    crew = Crew(
        agents=[researcher, writer],
        tasks=[research_task, write_task],
    )

    # Attach callback and kickoff
    result = crew.kickoff()

    # Or wrap individual tasks
    callback.on_task_start(research_task)
    # ... task executes ...
    callback.on_task_end(research_task, "Research complete")

    callback.save("crew_run.trajectory.json")

What gets captured:
    - Crew kickoff and completion
    - Individual task start/end with agent assignment
    - Tool calls within tasks
    - Agent delegation events
    - Task outputs and quality scores
    - Errors during execution
"""

from __future__ import annotations

import time
from typing import Any

from agentreel.adapters.base import BaseAdapter


class AgentReelCrewCallback(BaseAdapter):
    """CrewAI callback that captures crew execution into a trajectory.

    Examples:
        callback = AgentReelCrewCallback(title="My crew run")

        # Manual event logging
        callback.on_crew_start(crew)
        callback.on_task_start(task)
        callback.on_tool_use(agent, "web_search", {"query": "..."}, "results...")
        callback.on_task_end(task, "Task output here")
        callback.on_crew_end(crew, "Final result")
        callback.save()
    """

    def __init__(
        self,
        title: str = "",
        tags: list[str] | None = None,
        tracer=None,
    ):
        super().__init__(
            title=title, tags=tags, framework="crewai", tracer=tracer
        )
        self._task_starts: dict[str, float] = {}
        self._tool_starts: dict[str, tuple[float, int]] = {}

    # ── Crew Events ──────────────────────────────────────────

    def on_crew_start(self, crew: Any) -> None:
        """Called when a Crew starts executing."""
        agent_names = []
        if hasattr(crew, "agents"):
            agent_names = [getattr(a, "role", str(a)) for a in crew.agents]

        task_names = []
        if hasattr(crew, "tasks"):
            task_names = [getattr(t, "description", str(t))[:80] for t in crew.tasks]

        self.tracer.log_thought(
            f"Crew started with {len(agent_names)} agents: {', '.join(agent_names)}\n"
            f"Tasks: {'; '.join(task_names)}"
        )

    def on_crew_end(self, crew: Any, result: Any = None) -> None:
        """Called when a Crew finishes executing."""
        output = str(result)[:1000] if result else "completed"
        self.tracer.log_thought(f"Crew finished: {output}")

    # ── Task Events ──────────────────────────────────────────

    def on_task_start(self, task: Any) -> None:
        """Called when a Task starts executing."""
        task_id = str(id(task))
        self._task_starts[task_id] = time.time()

        description = getattr(task, "description", str(task))[:200]
        agent_role = ""
        if hasattr(task, "agent") and task.agent:
            agent_role = getattr(task.agent, "role", str(task.agent))

        self.tracer.log_decision(
            f"Starting task: {description}",
            reasoning=f"Assigned to agent: {agent_role}" if agent_role else None,
        )

    def on_task_end(self, task: Any, output: Any = None) -> None:
        """Called when a Task finishes executing."""
        task_id = str(id(task))
        start = self._task_starts.pop(task_id, time.time())
        duration_ms = (time.time() - start) * 1000

        description = getattr(task, "description", "task")[:100]
        output_str = str(output)[:1000] if output else "completed"

        self.tracer.log_thought(
            f"Task completed ({duration_ms:.0f}ms): {description}\n"
            f"Output: {output_str}"
        )

    def on_task_error(self, task: Any, error: Exception) -> None:
        """Called when a Task fails."""
        task_id = str(id(task))
        self._task_starts.pop(task_id, None)

        description = getattr(task, "description", "task")[:100]
        self.tracer.log_error(
            f"Task failed: {description} — {error}",
            code=type(error).__name__,
            recoverable=True,
        )

    # ── Tool Events ──────────────────────────────────────────

    def on_tool_use(
        self,
        agent: Any,
        tool_name: str,
        tool_input: Any,
        tool_output: Any = None,
        duration_ms: float | None = None,
        error: Exception | None = None,
    ) -> None:
        """Log a tool call made by an agent during task execution."""
        agent_role = getattr(agent, "role", str(agent)) if agent else "unknown"

        input_data = {}
        if isinstance(tool_input, dict):
            input_data = tool_input
        elif tool_input is not None:
            input_data = {"input": str(tool_input)[:1000]}

        step_id = self.tracer.log_tool_call(
            tool_name,
            input_data=input_data,
        )

        if error:
            self.tracer.log_tool_result(
                tool_name,
                output=str(error),
                parent_step_id=step_id,
                is_error=True,
                duration_ms=duration_ms,
            )
        elif tool_output is not None:
            self.tracer.log_tool_result(
                tool_name,
                output=str(tool_output)[:2000],
                parent_step_id=step_id,
                duration_ms=duration_ms,
            )

        if step_id < len(self.tracer.trajectory.steps) and duration_ms:
            self.tracer.trajectory.steps[step_id].duration_ms = duration_ms

    # ── Delegation Events ────────────────────────────────────

    def on_delegation(
        self,
        from_agent: Any,
        to_agent: Any,
        task: Any = None,
    ) -> None:
        """Called when one agent delegates to another."""
        from_role = getattr(from_agent, "role", str(from_agent))
        to_role = getattr(to_agent, "role", str(to_agent))
        task_desc = getattr(task, "description", "")[:100] if task else ""

        self.tracer.log_decision(
            f"Delegation: {from_role} -> {to_role}",
            reasoning=f"Task: {task_desc}" if task_desc else None,
        )

    # ── LLM Events (if CrewAI exposes them) ──────────────────

    def on_llm_call(
        self,
        agent: Any,
        model: str,
        messages: list[dict] | None = None,
        response: str | None = None,
        tokens: dict | None = None,
        duration_ms: float | None = None,
    ) -> None:
        """Log an LLM call made by an agent."""
        from agentreel.trajectory import LlmCall, Message, TokenUsage

        input_tokens = tokens.get("input", 0) if tokens else 0
        output_tokens = tokens.get("output", 0) if tokens else 0

        self.tracer._add_step(
            "llm_call",
            duration_ms=duration_ms,
            llm_call=LlmCall(
                model=model,
                messages=[
                    Message(role=m.get("role", "user"), content=str(m.get("content", ""))[:2000])
                    for m in (messages or [])
                ],
                response=Message(role="assistant", content=response[:2000] if response else None),
            ),
            tokens=TokenUsage(input_tokens=input_tokens, output_tokens=output_tokens),
            cost_usd=estimate_cost(model, input_tokens, output_tokens) if model else None,
        )


def estimate_cost(model: str, input_tokens: int, output_tokens: int) -> float:
    from agentreel.costs import estimate_cost as _estimate
    return _estimate(model, input_tokens, output_tokens)
