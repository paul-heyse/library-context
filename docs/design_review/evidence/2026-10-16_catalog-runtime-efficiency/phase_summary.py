"""Summarize attributed historical observations; never infer live work from a missing end."""

from __future__ import annotations

import json
import re
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from contextlib import suppress
from pathlib import Path
from typing import Any, TypedDict


class PhaseObservation(TypedDict):
    begins: int
    terminals: int
    statuses: Counter[str]
    max_terminal_elapsed_ms: float
    last_payload_line: int
    last_observed_timestamp: str


ROOT = Path(__file__).resolve().parents[4]

RUN = "20261016T190337.280Z-029373"
INVOCATION = "49584dfdb8244ff7b0d3bb9ec5b81d9f"


def summarize() -> dict[str, Any]:
    sys.path.insert(0, str(ROOT / "scripts"))
    from verify_evidence.runtime_adapter import ANSI, FIELD, observation

    path = ROOT / "build/runs" / RUN / "verify-evidence" / INVOCATION / "junit.xml"
    cases = []
    for _, element in ET.iterparse(path, events=("end",)):
        if element.tag != "testcase":
            continue
        if element.find("skipped") is not None:
            element.clear()
            continue
        phases: dict[str, PhaseObservation] = {}
        incomplete = {}
        transactions = Counter()
        max_retries = 0
        producers = Counter()
        output = "\n".join(
            child.text or "" for child in element if child.tag in {"system-out", "system-err"}
        )
        for number, raw in enumerate(output.splitlines(), 1):
            line = ANSI.sub("", raw)
            if "lctx_phase" not in line:
                continue
            producer = re.search(r'compiler_producer\{producer="([^"]+)"', line)
            if producer:
                producers[producer[1]] += 1
            if "native transaction terminal" in line:
                fields = {key: value.strip('"') for key, value in FIELD.findall(line)}
                transactions[
                    (fields.get("stage", "unattributed"), fields.get("status", "unattributed"))
                ] += 1
                with suppress(ValueError):
                    max_retries = max(max_retries, int(fields.get("retries", "0")))
            parsed = observation(line, INVOCATION)
            if parsed is None:
                continue
            fields = parsed.get("observation", {})
            if "phase_id" not in fields or "phase" not in fields or "status" not in fields:
                continue
            key = (fields.get("process_id"), fields.get("process_start_ticks"), fields["phase_id"])
            name = fields["phase"]
            aggregate = phases.setdefault(
                name,
                {
                    "begins": 0,
                    "terminals": 0,
                    "statuses": Counter(),
                    "max_terminal_elapsed_ms": 0.0,
                    "last_payload_line": 0,
                    "last_observed_timestamp": "",
                },
            )
            aggregate["statuses"][fields["status"]] += 1
            aggregate["last_payload_line"] = number
            aggregate["last_observed_timestamp"] = line.split(" ", 1)[0]
            if fields["status"] == "begin":
                aggregate["begins"] += 1
                incomplete[key] = {
                    "phase": name,
                    "payload_line": number,
                    "timestamp": line.split(" ", 1)[0],
                }
            else:
                aggregate["terminals"] += 1
                aggregate["max_terminal_elapsed_ms"] = max(
                    aggregate["max_terminal_elapsed_ms"], float(fields.get("elapsed_ms", 0))
                )
                incomplete.pop(key, None)
        cases.append(
            {
                "case": element.get("classname", "") + "$" + element.get("name", ""),
                "seconds": float(element.get("time", "0")),
                "source": str(path.relative_to(ROOT)),
                "producer_context_line_counts": dict(producers),
                "phases": phases,
                "incomplete_observations": list(incomplete.values()),
                "observed_transaction_terminals": [
                    {"stage": key[0], "status": key[1], "count": count}
                    for key, count in sorted(transactions.items())
                ],
                "max_observed_transaction_retries": max_retries,
            }
        )
        element.clear()
    return {
        "run": RUN,
        "invocation": INVOCATION,
        "limits": [
            "Historical JUnit-normalized payload; capture truncation is unknown.",
            "Phase waits overlap, ready operations can emit no phase, "
            "and durations are not additive.",
            "Missing terminal is incomplete observation, not proof of live work or deadlock.",
            "Flat producer fields identify context, not complete structured span parentage.",
            "Transaction terminal observations are client events, "
            "not a complete count of database transactions.",
        ],
        "cases": cases,
    }


if __name__ == "__main__":
    result = summarize()
    Path(__file__).with_name("phase-summary.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n"
    )
    print(
        json.dumps(
            {
                "cases": len(result["cases"]),
                "phase_kinds": sum(len(case["phases"]) for case in result["cases"]),
            }
        )
    )
