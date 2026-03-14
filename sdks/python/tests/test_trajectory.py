"""Tests for the AgentReel Python SDK."""

import json
from pathlib import Path

from agentreel import Trajectory, Recorder, redact


def test_create_trajectory():
    t = Trajectory()
    assert t.version == "0.1.0"
    assert t.id
    assert t.steps == []


def test_fork_trajectory():
    t = Trajectory()
    original_id = t.id
    forked = t.fork()
    assert forked.id != original_id
    assert forked.parent_id == original_id


def test_serialize_roundtrip():
    t = Trajectory()
    t.metadata.title = "Test run"

    json_str = t.to_json()
    parsed = Trajectory.from_json(json_str)

    assert parsed.id == t.id
    assert parsed.metadata.title == "Test run"


def test_recorder_basic():
    recorder = Recorder(title="Test recording")

    idx = recorder.record_llm_call(
        model="gpt-4o",
        messages=[{"role": "user", "content": "Hello"}],
        response={
            "choices": [{"message": {"role": "assistant", "content": "Hi!"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5},
        },
        duration_ms=500,
    )
    assert idx == 0

    idx = recorder.record_tool_call(
        name="bash",
        input={"command": "echo hello"},
        output="hello",
        duration_ms=100,
    )
    assert idx == 1

    traj = recorder.finalize(summary="Test complete")
    assert traj.stats.total_steps == 2
    assert traj.stats.total_llm_calls == 1
    assert traj.stats.total_tool_calls == 1
    assert traj.outcome.status == "success"


def test_recorder_anthropic_format():
    recorder = Recorder()

    recorder.record_llm_call(
        model="claude-opus-4-6",
        messages=[{"role": "user", "content": "Hello"}],
        response={
            "content": [{"type": "text", "text": "Hi there!"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5},
        },
    )

    traj = recorder.finalize()
    step = traj.steps[0]
    assert step.llm_call.model == "claude-opus-4-6"
    assert step.llm_call.response.content == "Hi there!"
    assert step.llm_call.stop_reason == "end_turn"


def test_redact_api_key():
    result = redact("Using key sk-abcdefghijklmnopqrstuvwxyz123456")
    assert "[REDACTED_API_KEY]" in result
    assert "sk-abcdef" not in result


def test_redact_email():
    result = redact("Contact user@example.com for details")
    assert "[REDACTED_EMAIL]" in result
    assert "user@example.com" not in result


def test_load_example_file():
    example_path = Path(__file__).parent.parent.parent.parent / "spec" / "example.trajectory.json"
    if example_path.exists():
        t = Trajectory.load(str(example_path))
        assert t.version == "0.1.0"
        assert len(t.steps) == 4
