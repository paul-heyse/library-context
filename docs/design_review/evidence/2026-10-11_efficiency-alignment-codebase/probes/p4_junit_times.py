"""P4: per-test durations from existing nextest JUnit captures (read-only, historical).

Run with `python3 -I probes/p4_junit_times.py RUNS_DIR`. For each run's ADR-0147 evidence
invocation with a junit.xml, reports the boundary/step it belongs to, the step duration from
summary.json, the JUnit suite time (test execution wall time as nextest reports it), test count,
and the slowest tests. The difference between step duration and suite time approximates build,
list and setup time outside test execution. Historical receipts only: revisions, load and cache
state differ per run.
"""
import json
import statistics
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

runs = Path(sys.argv[1])
rows = []
slow = defaultdict(list)
for summary in sorted(runs.glob("*/summary.json")):
    try:
        data = json.loads(summary.read_text())
    except (OSError, ValueError):
        continue
    for boundary in data.get("boundaries", []):
        for step in boundary.get("steps", []):
            inv = step.get("invocation")
            junit = summary.parent / "verify-evidence" / str(inv) / "junit.xml"
            if not inv or not junit.exists() or not isinstance(step.get("duration"), (int, float)):
                continue
            try:
                root = ET.parse(junit).getroot()
            except ET.ParseError:
                continue
            suite_time = float(root.get("time") or 0.0)
            cases = root.iter("testcase")
            times = []
            for case in cases:
                t = float(case.get("time") or 0.0)
                times.append(t)
                slow[(boundary["boundary"], case.get("classname"), case.get("name"))].append(t)
            rows.append((boundary["boundary"], step.get("name"), step["duration"], suite_time, len(times),
                         sum(times), summary.parent.name[:15]))

print("boundary | step | run | step s | junit suite s | step - suite s | tests | sum test s")
for r in rows:
    print(f"{r[0]} | {r[1]} | {r[6]} | {r[2]:.1f} | {r[3]:.1f} | {r[2]-r[3]:.1f} | {r[4]} | {r[5]:.1f}")
print()
print("slowest tests by median duration (boundary, binary, test) | runs | median s")
ranked = sorted(slow.items(), key=lambda kv: statistics.median(kv[1]), reverse=True)[:40]
for key, values in ranked:
    print(f"{key} | {len(values)} | {statistics.median(values):.1f}")
