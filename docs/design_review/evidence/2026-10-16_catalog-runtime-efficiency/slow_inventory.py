"""Reproduce the slow-case inventory from retained CE90 captures; starts no tests."""

from __future__ import annotations

import json
import re
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from typing import Any, NotRequired, TypedDict


class Case(TypedDict):
    case: str
    seconds: float
    outcome: str
    source: str
    locator: dict[str, str | int | None]
    channel: str
    termination_evidence: NotRequired[str]
    review_group: NotRequired[str]
    run: NotRequired[str]
    invocation: NotRequired[str]


def review_group(case: str) -> str:
    if case.startswith("cpg-core::fixture_corpus$"):
        return "T1"
    if case.startswith("cpg-extract"):
        return "T2"
    if case.startswith("cpg-core"):
        return "T3"
    if case.startswith("lctx-native"):
        return "T4"
    if case.startswith("lctx-publisher"):
        return "T5"
    raise ValueError(f"Unmapped slow case: {case}")


def inventory(root: Path) -> dict[str, Any]:
    runs: list[dict[str, Any]] = []
    cases: list[Case] = []
    latest: dict[str, tuple[tuple[str, str], Case]] = {}
    for directory in sorted((root / "build/runs").glob("20261016T*")):
        if not "20261016T154844" <= directory.name <= "20261016T193502":
            continue
        record = json.loads((directory / "record.json").read_text())
        if not str(record.get("label", "")).startswith("efficiency-"):
            continue
        for source in sorted(directory.glob("verify-evidence/*/invocation.json")):
            invocation = json.loads(source.read_text())
            if invocation.get("tool") != "nextest":
                continue
            junit = source.parent / "junit.xml"
            events = source.parent / "nextest-events.jsonl"
            results: list[Case] = []
            started = 0
            partial_tail = False
            suites = {}
            if events.exists():
                with events.open() as stream:
                    for number, line in enumerate(stream, 1):
                        if not line.endswith("\n"):
                            partial_tail = True
                            break
                        try:
                            event = json.loads(line)
                        except json.JSONDecodeError:
                            partial_tail = True
                            continue
                        if event.get("type") == "suite" and event.get("nextest"):
                            meta = event["nextest"]
                            suites[f"{meta['crate']}::{meta['test_binary']}"] = meta
                        if event.get("type") != "test":
                            continue
                        if event.get("event") == "started":
                            started += 1
                        duration = event.get("exec_time")
                        if event.get("event") not in {"ok", "failed"} or not isinstance(
                            duration, (int, float)
                        ):
                            continue
                        payload = event.get("stdout", "")
                        outcome = "passed" if event["event"] == "ok" else "failed"
                        if outcome == "failed" and "(test timed out)" in payload:
                            outcome = "timed_out"
                        prefix, _, test = event["name"].partition("$")
                        meta = suites.get(prefix, {})
                        binary = meta["crate"] if meta.get("kind") == "lib" else prefix
                        results.append(
                            {
                                "case": binary + "$" + test,
                                "seconds": duration,
                                "outcome": outcome,
                                "source": str(events.relative_to(root)),
                                "locator": {"line": number},
                                "channel": "libtest complete-line prefix",
                            }
                        )
            if junit.exists():
                results = []
                for _, element in ET.iterparse(junit, events=("end",)):
                    if element.tag != "testcase":
                        continue
                    if element.find("skipped") is None:
                        failures = [child for child in element if child.tag in {"failure", "error"}]
                        outcome = "passed" if not failures else "failed"
                        if any(child.get("type") == "test timeout" for child in failures):
                            outcome = "timed_out"
                        results.append(
                            {
                                "case": element.get("classname", "")
                                + "$"
                                + element.get("name", ""),
                                "seconds": float(element.get("time", "0")),
                                "outcome": outcome,
                                "source": str(junit.relative_to(root)),
                                "locator": {
                                    "test": element.get("name"),
                                    "binary": element.get("classname"),
                                },
                                "channel": "JUnit terminal case",
                            }
                        )
                    element.clear()
            # The producer's human summary can name timeouts missing from libtest JSON.
            # It classifies only an exact binary/test key from this same invocation.
            timed_out = set()
            native_log = source.parent / "native.log"
            if native_log.exists():
                with native_log.open(errors="replace") as stream:
                    for line in stream:
                        match = re.search(
                            r"TIMEOUT\s+\[\s*[\d.]+s\]\s+\([^)]*\)\s+(\S+)\s+(\S+)", line
                        )
                        if match:
                            timed_out.add(match[1] + "$" + match[2])
            for item in results:
                if item["outcome"] == "failed" and item["case"] in timed_out:
                    item["outcome"] = "timed_out"
                    item["termination_evidence"] = str(native_log.relative_to(root))
                prior = latest.get(item["case"])
                order = (directory.name, invocation.get("started", ""))
                if prior is None or order >= prior[0]:
                    latest[item["case"]] = (
                        order,
                        {"run": directory.name, "invocation": invocation["id"], **item},
                    )
            counts = Counter(item["outcome"] for item in results)
            slow = [item for item in results if item["seconds"] > 60]
            runs.append(
                {
                    "run": directory.name,
                    "label": record.get("label"),
                    "invocation": invocation["id"],
                    "started_events": started,
                    "terminal_cases": len(results),
                    "outcomes": dict(counts),
                    "over_60": len(slow),
                    "partial_json_tail": partial_tail,
                    "junit_available": junit.exists(),
                    "argv": invocation.get("effective_argv", invocation.get("argv")),
                    "source_context": invocation.get("source_context"),
                    "fixture_attachment": (invocation.get("fixture") or {}).get("attachment"),
                    "host_cleanup": invocation.get("host_cleanup"),
                }
            )
            cases.extend(
                {"run": directory.name, "invocation": invocation["id"], **item} for item in slow
            )
    slow_keys = {item["case"] for item in cases}
    for item in cases:
        item["review_group"] = review_group(item["case"])
    latest_slow = [
        {**latest[key][1], "review_group": review_group(key)} for key in sorted(slow_keys)
    ]
    return {
        "baseline": "2193bbfc3be4e2c93bf2d5374b3c2f0ba394fc1a",
        "threshold_seconds": 60,
        "meaning": "Historical test-body occurrences; not current-source reruns or a runtime SLA.",
        "runs": runs,
        "slow_cases": cases,
        "latest_for_slow_cases": latest_slow,
    }


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[4]
    result = inventory(root)
    destination = Path(__file__).with_name("slow-inventory.json")
    destination.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(
        json.dumps(
            {
                "runs": len(result["runs"]),
                "terminal_cases": sum(row["terminal_cases"] for row in result["runs"]),
                "slow_occurrences": len(result["slow_cases"]),
                "distinct_slow_cases": len({row["case"] for row in result["slow_cases"]}),
                "slow_outcomes": dict(Counter(row["outcome"] for row in result["slow_cases"])),
            }
        )
    )
