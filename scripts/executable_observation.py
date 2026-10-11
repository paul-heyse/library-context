"""Retained completion observations of mutable executable paths, never launch proof."""

from __future__ import annotations

import hashlib
import os
import stat
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


def identity(info: os.stat_result) -> dict[str, int]:
    return {name: getattr(info, "st_" + name) for name in
            ("dev", "ino", "mode", "size", "mtime_ns", "ctime_ns")}


def observe(path: Path) -> dict[str, Any]:
    """Open once; an unchanged pathname stat alone never authorizes digest reuse.

    The descriptor survives rename/unlink, but cannot protect in-place writes. Before/after
    descriptor and pathname observations therefore refuse detectable races. This remains an
    observation at completion, not proof of executed bytes or a complete source closure.
    """
    row: dict[str, Any] = {
        "path": str(path), "sha256": None,
        "observed_at": datetime.now(UTC).isoformat(timespec="microseconds"),
        "policy": "completion-descriptor-sha256-v1",
        "digest_reuse": "none: mutable output has no immutable producer lease",
        "executed_bytes": "unknown", "source_closure": "unknown",
    }
    try:
        with path.open("rb") as stream:
            before = os.fstat(stream.fileno())
            row["descriptor"] = identity(before)
            if not stat.S_ISREG(before.st_mode):
                raise OSError("executable observation requires a regular file")
            bound_before = path.stat()
            row["pathname_before"] = identity(bound_before)
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
            after = os.fstat(stream.fileno())
            bound_after = path.stat()
            row.update(descriptor=identity(before), descriptor_after=identity(after),
                       pathname_before=identity(bound_before), pathname_after=identity(bound_after))
            stable = identity(before) == identity(after) == identity(bound_before) == identity(bound_after)
            row.update(sha256=digest if stable else None,
                       observation="artifact bytes at completion" if stable else "changed")
    except OSError as error:
        row.update(observation="unavailable", unavailable=type(error).__name__)
    row["observation_finished_at"] = datetime.now(UTC).isoformat(timespec="microseconds")
    return row
