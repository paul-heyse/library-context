"""Native Cargo JSON diagnostics, including failures before any test case exists."""

from __future__ import annotations

import json
from collections.abc import Iterator
from pathlib import Path


def records(
    directory: Path, *, watermarks: dict | None = None, ends: dict | None = None
) -> Iterator[dict]:
    source = "cargo.jsonl" if (directory / "cargo.jsonl").exists() else "nextest-events.jsonl"
    path = directory / source
    if not path.exists():
        return
    with path.open("rb") as stream:
        stream.seek((watermarks or {}).get(source, 0))
        bound = (ends or {}).get(source, path.stat().st_size)
        while stream.tell() < bound:
            start = stream.tell()
            line = stream.readline()
            end = stream.tell()
            if not line.endswith(b"\n") or end > bound:
                return
            try:
                native = json.loads(line)
                if isinstance(native, dict) and "type" in native:
                    continue  # libtest's independent native channel shares stdout.
                if not isinstance(native, dict) or "reason" not in native:
                    raise ValueError("not a Cargo record")
            except ValueError:
                yield {
                    "case_key": f"unrecognized:{start}",
                    "phase": "build",
                    "event_kind": "unrecognized-output",
                    "attempt": 1,
                    "outcome": "unknown",
                    "source": source,
                    "source_start": start,
                    "source_end": end,
                    "payload": line.decode(errors="replace"),
                    "coverage": {
                        "native": False,
                        "reason": "raw output retained without semantic interpretation",
                    },
                }
                continue
            reason = native["reason"]
            message = native.get("message", {})
            outcome = (
                "failed"
                if message.get("level") == "error" or native.get("success") is False
                else "observed"
            )
            yield {
                "case_key": f"cargo:{reason}:{start}",
                "phase": "build",
                "event_kind": reason,
                "attempt": 1,
                "outcome": outcome,
                "package_id": native.get("package_id"),
                "target": native.get("target"),
                "executable": native.get("executable"),
                "source": source,
                "source_start": start,
                "source_end": end,
                "payload": message.get("rendered") or json.dumps(native, ensure_ascii=False),
                "coverage": {"authority": "native-cargo-json", "case_execution": "not established"},
            }
