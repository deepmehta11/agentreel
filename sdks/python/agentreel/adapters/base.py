"""Base adapter class for framework integrations."""

from __future__ import annotations

from agentreel.tracer import Tracer


class BaseAdapter:
    """Base class for all framework adapters.

    Wraps a Tracer instance and provides a common interface for
    framework-specific callback/hook implementations.
    """

    def __init__(
        self,
        title: str = "",
        tags: list[str] | None = None,
        framework: str = "custom",
        tracer: Tracer | None = None,
    ):
        self.tracer = tracer or Tracer(
            title=title,
            tags=tags,
            framework=framework,
        )

    def complete(self, outcome: str = "success", summary: str | None = None) -> str:
        """Mark the trajectory as complete."""
        return self.tracer.complete(outcome, summary)

    def save(self, path: str | None = None) -> str:
        """Save the trajectory to disk."""
        return self.tracer.save(path)

    @property
    def trajectory(self):
        return self.tracer.trajectory
