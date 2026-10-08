# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Paul Heyse
"""Read historical Rust build campaign receipts without changing their captures.

``just bench-builds report build/perf/<name>`` summarizes retained receipts.
New compilation profiles use ``just compile-profile record``. This reader
never starts builds, copies source, configures toolchains or changes caches.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path


def summarize(campaign: Path) -> dict:
    manifest = json.loads((campaign / "snapshot.json").read_text())
    trials: dict[str, dict[str, dict[str, list[float]]]] = {}
    failures = []
    for path in sorted((campaign / "results").glob("*/trial-*/summary.json")):
        variant = path.parent.parent.name
        trial = path.parent.name
        samples = json.loads(path.read_text())
        for name, result in samples.items():
            if result["exit_code"]:
                failures.append(f"{variant}/{trial}/{name}")
                continue
            scenario = (
                name.rsplit("-", 1)[0] if name.startswith(("warm-tests-", "edit-tests-")) else name
            )
            trials.setdefault(variant, {}).setdefault(trial, {}).setdefault(scenario, []).append(
                result["wall_seconds"]
            )
    per_trial = {
        variant: {
            trial: {name: statistics.median(values) for name, values in scenarios.items()}
            for trial, scenarios in cases.items()
        }
        for variant, cases in trials.items()
    }
    groups: dict[str, dict[str, list[float]]] = {}
    for variant, cases in per_trial.items():
        for scenarios in cases.values():
            for name, value in scenarios.items():
                groups.setdefault(variant, {}).setdefault(name, []).append(value)
    rows = {
        variant: {
            name: {
                "count": len(values),
                "median_seconds": statistics.median(values),
                "min_seconds": min(values),
                "max_seconds": max(values),
            }
            for name, values in samples.items()
        }
        for variant, samples in groups.items()
    }
    comparisons = {}
    for variant in per_trial:
        baseline = "stable" if variant == "stable-cache" else "stable-cache"
        if variant == "stable" or baseline not in per_trial:
            continue
        common_trials = set(per_trial[variant]) & set(per_trial[baseline])
        common_scenarios = set(groups[variant]) & set(groups[baseline])
        comparisons[variant] = {}
        for scenario in sorted(common_scenarios):
            pairs = [
                (per_trial[baseline][trial][scenario], per_trial[variant][trial][scenario])
                for trial in sorted(common_trials)
                if scenario in per_trial[baseline][trial] and scenario in per_trial[variant][trial]
            ]
            if pairs:
                comparisons[variant][scenario] = {
                    "baseline": baseline,
                    "paired_trials": len(pairs),
                    "median_wall_improvement_percent": statistics.median(
                        100 * (before - after) / before for before, after in pairs
                    ),
                    "every_pair_faster": all(after < before for before, after in pairs),
                }
    return {
        "source_sha256": manifest["source_sha256"],
        "measurements": rows,
        "comparisons": comparisons,
        "failures": failures,
        "decision": "not_run: comparisons require paired, uncontended trials and correctness gates",
    }


def main(argv: list[str] | None = None) -> int:
    arguments = list(sys.argv[1:] if argv is None else argv)
    retired_routes = {
        "run": "just compile-profile record",
        "capture": "just compile-profile record",
        "preflight": "just compile-profile doctor",
    }
    if arguments and arguments[0] in retired_routes:
        print(
            f"bench-builds {arguments[0]} is retired; use {retired_routes[arguments[0]]}",
            file=sys.stderr,
        )
        return 1
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    report = commands.add_parser(
        "report", help="summarize historical trial receipts without builds"
    )
    report.add_argument("campaign", type=Path)
    args = parser.parse_args(arguments)
    try:
        print(json.dumps(summarize(args.campaign.resolve()), indent=2))
    except (OSError, ValueError) as error:
        print(f"build measurement report failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
