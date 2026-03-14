"""OpenAI Agents SDK adapter for AgentReel.

Captures agent runs, tool calls, handoffs, and guardrail checks
into a trajectory. Works with the OpenAI Agents SDK tracing system.

Usage:
    from agentreel.adapters.openai_agents import AgentReelTracer
    from agents import Agent, Runner

    # Create the tracer
    tracer = AgentReelTracer(title="Customer support agent")

    # Option 1: Use as a trace processor
    from agents.tracing import set_trace_processors
    set_trace_processors([tracer])

    # Run your agent normally — everything is captured
    agent = Agent(name="Support", instructions="Help customers...")
    result = Runner.run_sync(agent, "I need help with my order")

    # Save
    tracer.save("support_run.trajectory.json")

    # Option 2: Manual wrapping for more control
    tracer = AgentReelTracer(title="My agent")

    # Wrap the runner
    runner = tracer.wrap_runner(Runner)
    result = runner.run_sync(agent, "Do something")
    tracer.save()

What gets captured:
    - Agent creation and configuration
    - Every LLM call with full messages and response
    - Tool/function calls with input and output
    - Agent-to-agent handoffs
    - Guardrail checks (input/output validation)
    - Errors and retries
"""

from __future__ import annotations

import functools
import time
from typing import Any

from agentreel.adapters.base import BaseAdapter
from agentreel.costs import estimate_cost


class AgentReelTracer(BaseAdapter):
    """Traces OpenAI Agents SDK runs into an AgentReel trajectory.

    Can be used as a trace processor or for manual wrapping.

    Examples:
        # As trace processor (recommended)
        tracer = AgentReelTracer(title="My run")
        from agents.tracing import set_trace_processors
        set_trace_processors([tracer])

        # Manual wrapping
        tracer = AgentReelTracer(title="My run")
        runner = tracer.wrap_runner(Runner)
    """

    def __init__(
        self,
        title: str = "",
        tags: list[str] | None = None,
        tracer=None,
    ):
        super().__init__(
            title=title, tags=tags, framework="openai-agents-sdk", tracer=tracer
        )
        self._pending_spans: dict[str, dict] = {}

    # ── Trace Processor Interface ────────────────────────────
    # These methods match the OpenAI Agents SDK TracingProcessor protocol

    def on_trace_start(self, trace: Any) -> None:
        """Called when a new trace starts."""
        pass

    def on_trace_end(self, trace: Any) -> None:
        """Called when a trace ends."""
        pass

    def on_span_start(self, span: Any) -> None:
        """Called when a span starts."""
        span_id = getattr(span, "span_id", str(id(span)))
        span_type = getattr(span, "span_type", "unknown")
        self._pending_spans[span_id] = {
            "start_time": time.time(),
            "type": span_type,
            "span": span,
        }

    def on_span_end(self, span: Any) -> None:
        """Called when a span ends."""
        span_id = getattr(span, "span_id", str(id(span)))
        pending = self._pending_spans.pop(span_id, None)
        if not pending:
            return

        duration_ms = (time.time() - pending["start_time"]) * 1000
        span_type = pending["type"]
        span_data = getattr(span, "span_data", None)

        if span_type == "llm" and span_data:
            self._record_llm_span(span_data, duration_ms)
        elif span_type == "function" and span_data:
            self._record_function_span(span_data, duration_ms)
        elif span_type == "handoff" and span_data:
            self._record_handoff_span(span_data, duration_ms)
        elif span_type == "guardrail" and span_data:
            self._record_guardrail_span(span_data, duration_ms)
        elif span_type == "agent" and span_data:
            self._record_agent_span(span_data, duration_ms)

    def shutdown(self) -> None:
        """Called when the processor shuts down."""
        pass

    def force_flush(self) -> None:
        """Force flush pending spans."""
        pass

    # ── Span Recording ───────────────────────────────────────

    def _record_llm_span(self, data: Any, duration_ms: float) -> None:
        model = getattr(data, "model", "unknown")
        input_tokens = getattr(getattr(data, "usage", None), "input_tokens", 0) or 0
        output_tokens = getattr(getattr(data, "usage", None), "output_tokens", 0) or 0

        # Extract response
        response_text = ""
        output = getattr(data, "output", None)
        if output:
            if isinstance(output, str):
                response_text = output
            elif hasattr(output, "content"):
                response_text = str(output.content)[:2000]

        from agentreel.trajectory import LlmCall, Message, TokenUsage

        self.tracer._add_step(
            "llm_call",
            duration_ms=duration_ms,
            llm_call=LlmCall(
                model=model,
                response=Message(role="assistant", content=response_text[:2000]),
            ),
            tokens=TokenUsage(input_tokens=input_tokens, output_tokens=output_tokens),
            cost_usd=estimate_cost(model, input_tokens, output_tokens),
        )

    def _record_function_span(self, data: Any, duration_ms: float) -> None:
        name = getattr(data, "name", "unknown")
        input_data = getattr(data, "input", None)
        output_data = getattr(data, "output", None)

        input_dict = {}
        if isinstance(input_data, dict):
            input_dict = input_data
        elif input_data is not None:
            input_dict = {"input": str(input_data)[:1000]}

        step_id = self.tracer.log_tool_call(name, input_data=input_dict)

        output_str = None
        if output_data is not None:
            output_str = str(output_data)[:2000]

        self.tracer.log_tool_result(
            name,
            output=output_str,
            parent_step_id=step_id,
            duration_ms=duration_ms,
        )
        if step_id < len(self.tracer.trajectory.steps):
            self.tracer.trajectory.steps[step_id].duration_ms = duration_ms

    def _record_handoff_span(self, data: Any, duration_ms: float) -> None:
        from_agent = getattr(data, "from_agent", "?")
        to_agent = getattr(data, "to_agent", "?")

        self.tracer.log_decision(
            f"Handoff: {from_agent} -> {to_agent}",
            reasoning=f"Agent {from_agent} handed off to {to_agent}",
        )

    def _record_guardrail_span(self, data: Any, duration_ms: float) -> None:
        name = getattr(data, "name", "guardrail")
        triggered = getattr(data, "triggered", False)

        self.tracer.log_thought(
            f"Guardrail '{name}': {'TRIGGERED' if triggered else 'passed'} ({duration_ms:.0f}ms)"
        )

    def _record_agent_span(self, data: Any, duration_ms: float) -> None:
        name = getattr(data, "name", "agent")
        self.tracer.log_thought(f"Agent '{name}' completed in {duration_ms:.0f}ms")

    # ── Runner Wrapping ──────────────────────────────────────

    def wrap_runner(self, runner_class: Any) -> Any:
        """Wrap an OpenAI Agents SDK Runner class to auto-trace.

        Usage:
            tracer = AgentReelTracer(title="My run")
            runner = tracer.wrap_runner(Runner)
            result = runner.run_sync(agent, "Do something")
        """
        adapter = self

        class TracedRunner:
            @staticmethod
            def run_sync(agent, input_text, **kwargs):
                adapter.tracer.log_thought(
                    f"Starting agent '{getattr(agent, 'name', '?')}' with: {str(input_text)[:500]}"
                )
                start = time.time()
                try:
                    result = runner_class.run_sync(agent, input_text, **kwargs)
                    duration = (time.time() - start) * 1000
                    output = getattr(result, "final_output", str(result))
                    adapter.tracer.log_thought(
                        f"Agent completed in {duration:.0f}ms: {str(output)[:500]}"
                    )
                    return result
                except Exception as e:
                    adapter.tracer.log_error(str(e), code=type(e).__name__)
                    raise

            @staticmethod
            async def run(agent, input_text, **kwargs):
                adapter.tracer.log_thought(
                    f"Starting agent '{getattr(agent, 'name', '?')}' with: {str(input_text)[:500]}"
                )
                start = time.time()
                try:
                    result = await runner_class.run(agent, input_text, **kwargs)
                    duration = (time.time() - start) * 1000
                    output = getattr(result, "final_output", str(result))
                    adapter.tracer.log_thought(
                        f"Agent completed in {duration:.0f}ms: {str(output)[:500]}"
                    )
                    return result
                except Exception as e:
                    adapter.tracer.log_error(str(e), code=type(e).__name__)
                    raise

        return TracedRunner
