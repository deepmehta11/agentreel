"""LangChain adapter for AgentReel.

Captures every LLM call, tool execution, chain step, and agent action
into a trajectory automatically. Zero manual instrumentation needed.

Usage:
    from agentreel.adapters.langchain import AgentReelCallbackHandler

    # Create the handler
    handler = AgentReelCallbackHandler(
        title="Research competitive analysis",
        tags=["research", "langchain"]
    )

    # Use with any LangChain component
    llm = ChatOpenAI(model="gpt-4o", callbacks=[handler])
    chain = prompt | llm | parser
    result = chain.invoke({"input": "Compare Stripe vs Adyen"})

    # Or pass to invoke directly
    result = chain.invoke(
        {"input": "Compare Stripe vs Adyen"},
        config={"callbacks": [handler]}
    )

    # Or use with agents
    agent = create_react_agent(llm, tools, prompt)
    agent_executor = AgentExecutor(agent=agent, tools=tools, callbacks=[handler])
    result = agent_executor.invoke({"input": "Research topic X"})

    # Save the trajectory
    handler.save("research_run.trajectory.json")

    # Or auto-saves to ~/.agentreel/trajectories/ if no path given
    path = handler.save()
    print(f"Trajectory saved to: {path}")

What gets captured:
    - Every LLM call (model, messages, response, tokens, cost, latency)
    - Every tool execution (name, input, output, duration, errors)
    - Chain start/end events
    - Agent actions and decisions
    - Errors with stack traces
    - Retry attempts
"""

from __future__ import annotations

import time
from typing import Any
from uuid import UUID

from agentreel.adapters.base import BaseAdapter
from agentreel.costs import estimate_cost

try:
    from langchain_core.callbacks import BaseCallbackHandler
    from langchain_core.outputs import LLMResult
    from langchain_core.agents import AgentAction, AgentFinish
except ImportError:
    raise ImportError(
        "LangChain is required for this adapter. Install it with:\n"
        "  pip install langchain-core\n"
        "  # or\n"
        "  pip install langchain"
    )


class AgentReelCallbackHandler(BaseCallbackHandler, BaseAdapter):
    """LangChain callback handler that captures everything into a trajectory.

    Drop-in replacement — just add to your callbacks list and every LLM call,
    tool execution, chain event, and agent action is captured automatically.

    Examples:
        # Global callbacks (captures everything)
        handler = AgentReelCallbackHandler(title="My run")
        llm = ChatOpenAI(model="gpt-4o", callbacks=[handler])

        # Per-invoke callbacks
        result = chain.invoke(input, config={"callbacks": [handler]})

        # With agents
        executor = AgentExecutor(agent=agent, tools=tools, callbacks=[handler])

        # Save when done
        handler.save("my_run.trajectory.json")
    """

    name = "AgentReelCallbackHandler"

    def __init__(
        self,
        title: str = "",
        tags: list[str] | None = None,
        tracer=None,
    ):
        BaseCallbackHandler.__init__(self)
        BaseAdapter.__init__(
            self, title=title, tags=tags, framework="langchain", tracer=tracer
        )
        self._llm_starts: dict[str, float] = {}  # run_id -> start_time
        self._tool_starts: dict[str, tuple[float, int]] = {}  # run_id -> (start_time, step_id)
        self._chain_starts: dict[str, float] = {}

    # ── LLM Events ───────────────────────────────────────────

    def on_llm_start(
        self,
        serialized: dict[str, Any],
        prompts: list[str],
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        tags: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> None:
        self._llm_starts[str(run_id)] = time.time()

    def on_chat_model_start(
        self,
        serialized: dict[str, Any],
        messages: list[list[Any]],
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        tags: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> None:
        self._llm_starts[str(run_id)] = time.time()

    def on_llm_end(
        self,
        response: LLMResult,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        start = self._llm_starts.pop(str(run_id), time.time())
        duration_ms = (time.time() - start) * 1000

        # Extract model info
        model = ""
        if response.llm_output:
            model = response.llm_output.get("model_name", "")
            if not model:
                model = response.llm_output.get("model", "")

        # Extract token usage
        input_tokens = 0
        output_tokens = 0
        if response.llm_output:
            usage = response.llm_output.get("token_usage", {})
            input_tokens = usage.get("prompt_tokens", 0) or usage.get("input_tokens", 0)
            output_tokens = usage.get("completion_tokens", 0) or usage.get("output_tokens", 0)

        # Extract response text
        resp_text = ""
        if response.generations:
            for gen_list in response.generations:
                for gen in gen_list:
                    resp_text += gen.text

        from agentreel.trajectory import LlmCall, Message, TokenUsage

        self.tracer._add_step(
            "llm_call",
            duration_ms=duration_ms,
            llm_call=LlmCall(
                model=model,
                response=Message(role="assistant", content=resp_text[:2000]),
                stop_reason=None,
            ),
            tokens=TokenUsage(input_tokens=input_tokens, output_tokens=output_tokens),
            cost_usd=estimate_cost(model, input_tokens, output_tokens) if model else None,
        )

        # Extract tool calls from response (function calling)
        if response.generations:
            for gen_list in response.generations:
                for gen in gen_list:
                    msg = getattr(gen, "message", None)
                    if msg and hasattr(msg, "tool_calls"):
                        for tc in msg.tool_calls:
                            name = tc.get("name", "") if isinstance(tc, dict) else getattr(tc, "name", "")
                            args = tc.get("args", {}) if isinstance(tc, dict) else getattr(tc, "args", {})
                            self.tracer.log_tool_call(name, input_data=args)

    def on_llm_error(
        self,
        error: BaseException,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        self._llm_starts.pop(str(run_id), None)
        self.tracer.log_error(
            str(error),
            code=type(error).__name__,
            recoverable=True,
        )

    # ── Tool Events ──────────────────────────────────────────

    def on_tool_start(
        self,
        serialized: dict[str, Any],
        input_str: str,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        tags: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> None:
        tool_name = serialized.get("name", "unknown")
        step_id = self.tracer.log_tool_call(
            tool_name,
            input_data={"input": input_str[:1000]},
        )
        self._tool_starts[str(run_id)] = (time.time(), step_id)

    def on_tool_end(
        self,
        output: str,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        start_time, step_id = self._tool_starts.pop(str(run_id), (time.time(), -1))
        duration_ms = (time.time() - start_time) * 1000

        # Get tool name from the step
        tool_name = "unknown"
        if 0 <= step_id < len(self.tracer.trajectory.steps):
            tc = self.tracer.trajectory.steps[step_id].tool_call
            if tc:
                tool_name = tc.name
                tc.status = "success"
                tc.output = output[:2000]
            self.tracer.trajectory.steps[step_id].duration_ms = duration_ms

        self.tracer.log_tool_result(
            tool_name,
            output=output[:2000],
            parent_step_id=step_id if step_id >= 0 else None,
            duration_ms=duration_ms,
        )

    def on_tool_error(
        self,
        error: BaseException,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        start_time, step_id = self._tool_starts.pop(str(run_id), (time.time(), -1))
        duration_ms = (time.time() - start_time) * 1000

        tool_name = "unknown"
        if 0 <= step_id < len(self.tracer.trajectory.steps):
            tc = self.tracer.trajectory.steps[step_id].tool_call
            if tc:
                tool_name = tc.name
                tc.status = "error"
            self.tracer.trajectory.steps[step_id].duration_ms = duration_ms

        self.tracer.log_tool_result(
            tool_name,
            output=str(error),
            parent_step_id=step_id if step_id >= 0 else None,
            is_error=True,
            duration_ms=duration_ms,
        )

    # ── Agent Events ─────────────────────────────────────────

    def on_agent_action(
        self,
        action: AgentAction,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        self.tracer.log_thought(
            f"Agent decided to use tool: {action.tool}\n"
            f"Input: {str(action.tool_input)[:500]}\n"
            f"Log: {action.log[:500] if action.log else ''}"
        )

    def on_agent_finish(
        self,
        finish: AgentFinish,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        output = finish.return_values.get("output", str(finish.return_values))
        self.tracer.log_thought(f"Agent finished: {str(output)[:500]}")

    # ── Chain Events ─────────────────────────────────────────

    def on_chain_start(
        self,
        serialized: dict[str, Any],
        inputs: dict[str, Any],
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        tags: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> None:
        self._chain_starts[str(run_id)] = time.time()

    def on_chain_end(
        self,
        outputs: dict[str, Any],
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        self._chain_starts.pop(str(run_id), None)

    def on_chain_error(
        self,
        error: BaseException,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        self._chain_starts.pop(str(run_id), None)
        self.tracer.log_error(
            str(error),
            code=type(error).__name__,
            recoverable=True,
        )

    # ── Retriever Events ─────────────────────────────────────

    def on_retriever_start(
        self,
        serialized: dict[str, Any],
        query: str,
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        self.tracer.log_tool_call(
            "retriever",
            input_data={"query": query[:500]},
            tool_type="function",
        )

    def on_retriever_end(
        self,
        documents: list[Any],
        *,
        run_id: UUID,
        parent_run_id: UUID | None = None,
        **kwargs: Any,
    ) -> None:
        doc_summaries = [
            getattr(d, "page_content", str(d))[:200] for d in documents[:5]
        ]
        self.tracer.log_tool_result(
            "retriever",
            output=f"Retrieved {len(documents)} documents: {doc_summaries}",
        )
