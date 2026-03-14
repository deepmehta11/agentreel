"""Redact secrets and PII from trajectory data."""

from __future__ import annotations

import re

_PATTERNS: list[tuple[re.Pattern, str]] = [
    # API keys (sk-...)
    (re.compile(r"(?i)(sk-[a-zA-Z0-9]{20,})"), "[REDACTED_API_KEY]"),
    # API key assignments
    (re.compile(r'(?i)(api[_-]?key\s*[:=]\s*)[\'"]?([a-zA-Z0-9_\-]{20,})[\'"]?'), r"\1[REDACTED]"),
    # Bearer tokens
    (re.compile(r"(?i)(bearer\s+)([a-zA-Z0-9_\-.]{20,})"), r"\1[REDACTED]"),
    # AWS keys
    (re.compile(r"AKIA[0-9A-Z]{16}"), "[REDACTED_AWS_KEY]"),
    # Passwords
    (re.compile(r'(?i)(password\s*[:=]\s*)[\'"]?([^\s\'"]{4,})[\'"]?'), r"\1[REDACTED]"),
    # Tokens
    (re.compile(r'(?i)(token\s*[:=]\s*)[\'"]?([a-zA-Z0-9_\-.]{20,})[\'"]?'), r"\1[REDACTED]"),
    # Email addresses
    (re.compile(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"), "[REDACTED_EMAIL]"),
]


def redact(text: str) -> str:
    """Redact secrets and PII from text."""
    result = text
    for pattern, replacement in _PATTERNS:
        result = pattern.sub(replacement, result)
    return result


def redact_trajectory_file(input_path: str, output_path: str | None = None) -> None:
    """Redact a trajectory JSON file."""
    with open(input_path) as f:
        content = f.read()

    redacted = redact(content)

    out = output_path or input_path
    with open(out, "w") as f:
        f.write(redacted)
