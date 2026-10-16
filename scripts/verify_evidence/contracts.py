"""Verification sidecar contracts. Native results and capture health are independent."""

from __future__ import annotations

from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Any

SCHEMA = 1
ADAPTER_VERSION = 2


class CaptureUnsupported(ValueError):
    """Capture cannot safely compose with the requested invocation."""


class CaptureState(StrEnum):
    REQUESTED = "requested"
    AVAILABLE = "available"
    MISSING = "missing"
    PARTIAL = "partial"
    TRUNCATED = "truncated"
    UNSUPPORTED = "unsupported"
    FAILED = "failed"


@dataclass(frozen=True)
class LaunchRequest:
    argv: Sequence[str]
    env: Mapping[str, str]
    log: Path
    live: bool
    cwd: Path
    stdout: Path | None = None
    observer: Callable[[dict[str, Any]], None] | None = None


@dataclass(frozen=True)
class EvidenceReference:
    """Semantic attribution independent of derived payload positions.

    Native source ranges are original locators, not projection offsets. The
    payload digest binds the adapter's interpretation to this native identity.
    """

    invocation: str
    source_generation: str
    case_key: str
    attempt: int
    phase: str
    source: str
    source_start: int | None
    source_end: int | None
    source_digest: str
    payload_digest: str
    locator: Any = None
    event_kind: str | None = None
    adapter: int = ADAPTER_VERSION

    def __post_init__(self) -> None:
        for name in ("invocation", "source_generation", "source", "phase"):
            value = getattr(self, name)
            if not isinstance(value, str) or not value:
                raise ValueError(f"evidence reference requires {name}")
        if not isinstance(self.case_key, str):
            raise ValueError("evidence reference requires a native case key")
        for name in ("attempt", "adapter"):
            value = getattr(self, name)
            if type(value) is not int or value < 1:
                raise ValueError(f"evidence reference requires positive {name}")
        for name in ("source_digest", "payload_digest"):
            value = getattr(self, name)
            if (
                not isinstance(value, str)
                or len(value) != 64
                or any(c not in "0123456789abcdef" for c in value)
            ):
                raise ValueError(f"evidence reference requires SHA256 {name}")
        if (self.source_start is not None or self.source_end is not None) and (
            type(self.source_start) is not int
            or type(self.source_end) is not int
            or not 0 <= self.source_start < self.source_end
        ):
            raise ValueError("evidence reference has invalid native source range")


@dataclass(frozen=True)
class PrefixWatermark:
    """The complete native and derived prefixes committed in one generation."""

    source_generation: str
    source_end: int
    payload_end: int
    projection_generation: str

    def __post_init__(self) -> None:
        if not self.source_generation or not self.projection_generation:
            raise ValueError("prefix watermark requires native and projection generations")
        for name in ("source_end", "payload_end"):
            value = getattr(self, name)
            if type(value) is not int or value < 0:
                raise ValueError(f"prefix watermark requires nonnegative {name}")
