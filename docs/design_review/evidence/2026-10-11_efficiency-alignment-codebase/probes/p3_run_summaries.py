"""P3: historical verify-boundary timings from existing run summaries (read-only).

Run with `python3 -I probes/p3_run_summaries.py RUNS_DIR`. For every `summary.json`, reports per
boundary: runs, median boundary duration, median sum of step durations, and the median of the
difference (time inside a boundary that no step accounts for: readiness, fixture attachment and
release, evidence capture, cleanup). These are historical receipts under whatever load and
source revision each run had; they are not fresh runs and do not attribute cost to a mechanism.
"""
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

runs = Path(sys.argv[1])
by_boundary = defaultdict(list)
steps_by_name = defaultdict(list)
for summary in sorted(runs.glob("*/summary.json")):
    try:
        data = json.loads(summary.read_text())
    except (OSError, ValueError):
        continue
    for boundary in data.get("boundaries", []):
        duration = boundary.get("duration")
        steps = [s for s in boundary.get("steps", []) if isinstance(s.get("duration"), (int, float))]
        if not isinstance(duration, (int, float)) or not steps:
            continue
        step_total = sum(s["duration"] for s in steps)
        by_boundary[boundary["boundary"]].append(
            (duration, step_total, duration - step_total, bool(boundary.get("fixture")), summary.parent.name[:8])
        )
        for step in steps:
            steps_by_name[(boundary["boundary"], step.get("name"))].append(step["duration"])

print("boundary | runs | fixture | median total s | median steps s | median unaccounted s | max unaccounted s | first..last date")
for name, rows in sorted(by_boundary.items()):
    med = lambda i: statistics.median(r[i] for r in rows)
    print(
        f"{name} | {len(rows)} | {any(r[3] for r in rows)} | {med(0):.1f} | {med(1):.1f} | {med(2):.1f} | "
        f"{max(r[2] for r in rows):.1f} | {min(r[4] for r in rows)}..{max(r[4] for r in rows)}"
    )
print()
print("step (boundary, name) | runs | median s | min s")
for key, values in sorted(steps_by_name.items()):
    print(f"{key} | {len(values)} | {statistics.median(values):.1f} | {min(values):.1f}")
