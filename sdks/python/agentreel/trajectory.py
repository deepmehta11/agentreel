"""Core trajectory types for AgentReel."""

from __future__ import annotations

import json
import uuid
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from typing import Any


@dataclass
class AgentInfo:
    name: str | None = None
    version: str | None = None
    url: str | None = None


@dataclass
class ModelInfo:
    provider: str | None = None
    model_id: str | None = None
    parameters: dict[str, Any] | None = None


@dataclass
class EnvironmentInfo:
    os: str | None = None
    arch: str | None = None
    runtime: str | None = None


@dataclass
class TokenUsage:
    input_tokens: int | None = None
    output_tokens: int | None = None
    cache_read_tokens: int | None = None
    cache_write_tokens: int | None = None


@dataclass
class ContentBlock:
    type: str = "text"
    text: str | None = None
    media_type: str | None = None
    data: str | None = None
    url: str | None = None
    tool_use_id: str | None = None
    tool_name: str | None = None
    input: dict[str, Any] | None = None
    output: Any = None


@dataclass
class Message:
    role: str = "user"
    content: str | list[ContentBlock] | None = None


@dataclass
class LlmCall:
    model: str | None = None
    messages: list[Message] = field(default_factory=list)
    response: Message | None = None
    stop_reason: str | None = None


@dataclass
class ToolCall:
    name: str = ""
    tool_type: str | None = None  # "function", "mcp", "web_search", "code_execution", etc.
    input: dict[str, Any] | None = None
    output: Any = None
    status: str | None = None
    mcp_server: str | None = None
    mcp_server_name: str | None = None
    screenshots: list[Screenshot] = field(default_factory=list)


@dataclass
class ToolResult:
    tool_name: str = ""
    output: Any = None
    is_error: bool = False
    error_message: str | None = None
    tool_type: str | None = None
    mcp_server: str | None = None


@dataclass
class HumanInput:
    content: str | None = None
    action: str | None = None


@dataclass
class Error:
    code: str | None = None
    message: str | None = None
    recoverable: bool | None = None
    stack_trace: str | None = None
    recovery_action: str | None = None


@dataclass
class Thought:
    content: str = ""
    thinking_tokens: int | None = None


@dataclass
class AgentDecision:
    decision: str = ""
    alternatives_considered: list[str] = field(default_factory=list)
    reasoning: str | None = None
    confidence: float | None = None


@dataclass
class FileOperation:
    operation: str = "write"  # "read", "write", "create", "delete", "move", "copy"
    path: str = ""
    content_preview: str | None = None
    size_bytes: int | None = None


@dataclass
class Screenshot:
    timestamp: str | None = None
    media_type: str = "image/png"
    data: str | None = None
    url: str | None = None
    label: str | None = None


@dataclass
class FileSnapshot:
    path: str = ""
    content: str | None = None
    hash: str | None = None
    language: str | None = None


@dataclass
class Step:
    index: int = 0
    type: str = "llm_call"
    timestamp: str = ""
    duration_ms: float | None = None
    parent_step_id: int | None = None
    llm_call: LlmCall | None = None
    tool_call: ToolCall | None = None
    tool_result: ToolResult | None = None
    human_input: HumanInput | None = None
    error: Error | None = None
    thought: Thought | None = None
    agent_decision: AgentDecision | None = None
    file_operation: FileOperation | None = None
    tokens: TokenUsage | None = None
    cost_usd: float | None = None


@dataclass
class FileDiff:
    path: str = ""
    action: str = "modified"
    diff: str | None = None


@dataclass
class Artifact:
    name: str = ""
    type: str = ""
    data: str | None = None
    url: str | None = None


@dataclass
class Outcome:
    status: str | None = None
    summary: str | None = None
    files_changed: list[FileDiff] = field(default_factory=list)
    artifacts: list[Artifact] = field(default_factory=list)


@dataclass
class Stats:
    total_steps: int = 0
    total_llm_calls: int = 0
    total_tool_calls: int = 0
    total_tokens: TokenUsage | None = None
    total_cost_usd: float | None = None
    total_duration_ms: float | None = None
    errors_count: int = 0
    retries_count: int = 0


@dataclass
class Annotation:
    type: str = "comment"
    content: str = ""
    author: str | None = None
    step_index: int | None = None
    created_at: str | None = None


@dataclass
class Input:
    prompt: str | None = None
    system_prompt: str | None = None
    files: list[FileSnapshot] = field(default_factory=list)
    context: dict[str, Any] | None = None


@dataclass
class Metadata:
    created_at: str = ""
    completed_at: str | None = None
    agent: AgentInfo | None = None
    model: ModelInfo | None = None
    environment: EnvironmentInfo | None = None
    tags: list[str] = field(default_factory=list)
    title: str | None = None
    description: str | None = None


@dataclass
class Trajectory:
    version: str = "0.1.0"
    id: str = ""
    parent_id: str | None = None
    metadata: Metadata = field(default_factory=Metadata)
    input: Input | None = None
    steps: list[Step] = field(default_factory=list)
    outcome: Outcome | None = None
    stats: Stats | None = None
    annotations: list[Annotation] = field(default_factory=list)

    def __post_init__(self):
        if not self.id:
            self.id = str(uuid.uuid4())
        if not self.metadata.created_at:
            self.metadata.created_at = _now_iso()

    def add_step(self, step: Step) -> int:
        """Add a step and return its index."""
        step.index = len(self.steps)
        if not step.timestamp:
            step.timestamp = _now_iso()
        self.steps.append(step)
        return step.index

    def compute_stats(self) -> Stats:
        """Compute stats from steps."""
        total_input = 0
        total_output = 0
        total_cost = 0.0
        total_duration = 0.0
        llm_calls = 0
        tool_calls = 0
        errors = 0

        for step in self.steps:
            if step.duration_ms:
                total_duration += step.duration_ms
            if step.cost_usd:
                total_cost += step.cost_usd
            if step.tokens:
                total_input += step.tokens.input_tokens or 0
                total_output += step.tokens.output_tokens or 0
            if step.type == "llm_call":
                llm_calls += 1
            elif step.type == "tool_call":
                tool_calls += 1
            elif step.type == "error":
                errors += 1

        self.stats = Stats(
            total_steps=len(self.steps),
            total_llm_calls=llm_calls,
            total_tool_calls=tool_calls,
            total_tokens=TokenUsage(input_tokens=total_input, output_tokens=total_output),
            total_cost_usd=total_cost,
            total_duration_ms=total_duration,
            errors_count=errors,
        )
        return self.stats

    def fork(self) -> Trajectory:
        """Fork this trajectory — creates a new one linked to this parent."""
        import copy

        forked = copy.deepcopy(self)
        forked.parent_id = self.id
        forked.id = str(uuid.uuid4())
        forked.metadata.created_at = _now_iso()
        forked.metadata.completed_at = None
        forked.annotations = []
        return forked

    def to_json(self, indent: int = 2) -> str:
        """Serialize to JSON."""
        return json.dumps(_clean_dict(asdict(self)), indent=indent)

    @classmethod
    def from_json(cls, data: str) -> Trajectory:
        """Deserialize from JSON."""
        raw = json.loads(data)
        return cls._from_dict(raw)

    def save(self, path: str) -> None:
        """Save to a file."""
        with open(path, "w") as f:
            f.write(self.to_json())

    @classmethod
    def load(cls, path: str) -> Trajectory:
        """Load from a file."""
        with open(path) as f:
            return cls.from_json(f.read())

    @classmethod
    def _from_dict(cls, d: dict) -> Trajectory:
        """Build a Trajectory from a raw dict (best-effort)."""
        t = cls()
        t.version = d.get("version", "0.1.0")
        t.id = d.get("id", str(uuid.uuid4()))
        t.parent_id = d.get("parent_id")

        meta = d.get("metadata", {})
        t.metadata = Metadata(
            created_at=meta.get("created_at", ""),
            completed_at=meta.get("completed_at"),
            agent=AgentInfo(**meta["agent"]) if meta.get("agent") else None,
            model=ModelInfo(**meta["model"]) if meta.get("model") else None,
            environment=EnvironmentInfo(**{
                k: v for k, v in meta["environment"].items()
                if k in ("os", "arch", "runtime")
            }) if meta.get("environment") else None,
            tags=meta.get("tags", []),
            title=meta.get("title"),
            description=meta.get("description"),
        )

        inp = d.get("input")
        if inp:
            t.input = Input(
                prompt=inp.get("prompt"),
                system_prompt=inp.get("system_prompt"),
                context=inp.get("context"),
            )

        for step_data in d.get("steps", []):
            step = Step(
                index=step_data.get("index", 0),
                type=step_data.get("type", "llm_call"),
                timestamp=step_data.get("timestamp", ""),
                duration_ms=step_data.get("duration_ms"),
                cost_usd=step_data.get("cost_usd"),
            )

            if step_data.get("tokens"):
                step.tokens = TokenUsage(**step_data["tokens"])

            if step_data.get("llm_call"):
                lc = step_data["llm_call"]
                step.llm_call = LlmCall(
                    model=lc.get("model"),
                    stop_reason=lc.get("stop_reason"),
                )

            if step_data.get("tool_call"):
                tc = step_data["tool_call"]
                step.tool_call = ToolCall(
                    name=tc.get("name", ""),
                    input=tc.get("input"),
                    output=tc.get("output"),
                    status=tc.get("status"),
                )

            t.steps.append(step)

        outcome = d.get("outcome")
        if outcome:
            t.outcome = Outcome(
                status=outcome.get("status"),
                summary=outcome.get("summary"),
            )

        return t


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _clean_dict(d: Any) -> Any:
    """Remove None values and empty lists for clean JSON output."""
    if isinstance(d, dict):
        return {k: _clean_dict(v) for k, v in d.items() if v is not None and v != [] and v != {}}
    if isinstance(d, list):
        return [_clean_dict(v) for v in d]
    return d
