"""Read selected historical captures; event counts are not RPC or test counts."""

from __future__ import annotations

import collections
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
CAPTURES = (
    ("20261015T181356.562Z-8a3347", "80231803896d4ef6a3fe53fc0bc76a37"),
    ("20261015T181515.206Z-7c15e3", "d6b4959571144b13870c492fda4d1477"),
)
PHASE = re.compile(r'phase="([^"]+)".*?phase_id=(\d+).*?status="([^"]+)"')
ELAPSED = re.compile(r"elapsed_ms=([\d.]+)")
STAMP = re.compile(r"2026-10-15T[\d:.]+Z")


def inspect(run: str, invocation: str) -> dict:
    path = ROOT / "build/runs" / run / "verify-evidence" / invocation / "native.log"
    digest = hashlib.sha256()
    counts: collections.Counter[str] = collections.Counter()
    conflicts: collections.Counter[str] = collections.Counter()
    max_retries: dict[str, int] = {}
    long = []
    truncation_mentions = []
    with path.open("rb") as stream:
        for number, raw in enumerate(stream, 1):
            digest.update(raw)
            line = raw.decode(errors="replace")
            if "truncat" in line.lower():
                truncation_mentions.append(number)
            if "native transaction conflict;" in line:
                stage = re.search(r' stage="([^"]+)"', line)
                recovery = re.search(r" recovery=(true|false)", line)
                retries = re.search(r" retries=(\d+)", line)
                if stage and recovery and retries:
                    key = f"{stage[1]};recovery={recovery[1]}"
                    conflicts[key] += 1
                    max_retries[key] = max(max_retries.get(key, 0), int(retries[1]))
            if "lctx_phase:" not in line or not (match := PHASE.search(line)):
                continue
            name, _, status = match.groups()
            if status != "begin":
                counts[name] += 1
            elapsed = ELAPSED.search(line)
            if elapsed and float(elapsed[1]) >= 1000:
                timestamp = STAMP.search(line)
                long.append(
                    {
                        "phase": name,
                        "status": status,
                        "line": number,
                        "timestamp": timestamp[0] if timestamp else None,
                        "elapsed_ms": float(elapsed[1]),
                    }
                )
    return {
        "run": run,
        "invocation": invocation,
        "bytes": path.stat().st_size,
        "sha256": digest.hexdigest(),
        "terminal_event_counts": counts.most_common(18),
        "conflict_log_observations": dict(conflicts),
        "maximum_observed_retries": max_retries,
        "longest_observed_intervals": sorted(long, key=lambda row: -row["elapsed_ms"])[:20],
        "literal_truncation_mentions_at_lines": truncation_mentions,
        "limits": (
            "Nested/concurrent intervals are not additive. Conflict logs are sampled at powers "
            "of two, not a complete retry count. No RPC counts, complete execution coverage, "
            "causal shares or comparative speed are inferred."
        ),
    }


if __name__ == "__main__":
    print(json.dumps([inspect(*capture) for capture in CAPTURES], indent=2))
