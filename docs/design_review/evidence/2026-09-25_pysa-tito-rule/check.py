"""Check a pinned Pysa source-to-sink control and its inferred TITO ports."""

from __future__ import annotations

import ast
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUTPUT = ROOT / "raw" / "out"


def expected_issue_lines() -> dict[str, int]:
    tree = ast.parse((ROOT / "probe.py").read_text())
    lines: dict[str, int] = {}
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Name):
            continue
        if node.func.id != "sink" or len(node.args) != 1:
            continue
        argument = node.args[0]
        if isinstance(argument, ast.Call) and isinstance(argument.func, ast.Name):
            lines[argument.func.id] = node.lineno
    return lines


def main() -> None:
    compiler_fixture = ROOT.parents[3] / "fixtures/python/pysa_tito_shapes/probe/__init__.py"
    if compiler_fixture.read_bytes() != (ROOT / "probe.py").read_bytes():
        raise SystemExit("Pysa and compiler fixture source bytes differ")
    log = (ROOT / "raw" / "analyze.log").read_text()
    if f"Checking project configured at `{ROOT / 'pyproject.toml'}`" not in log:
        raise SystemExit("Pysa did not use the isolated Pyrefly project")
    metadata = json.loads((OUTPUT / "taint-metadata.json").read_text())
    if metadata["stats"]["model_verification_errors"]:
        raise SystemExit("Pysa model verification failed")

    models: dict[str, list[dict]] = {}
    issue_lines: set[int] = set()
    for line in (OUTPUT / "taint-output.json").read_text().splitlines():
        record = json.loads(line)
        if record.get("kind") == "model":
            data = record["data"]
            if data["callable"].startswith("probe."):
                models[data["callable"]] = data.get("tito", [])
        elif record.get("kind") == "issue":
            data = record["data"]
            if data["code"] != 9001 or data["callable"] != "probe.exercise":
                raise SystemExit(f"unexpected Pysa issue: {data}")
            issue_lines.add(data["line"])

    lines = expected_issue_lines()
    if issue_lines != {lines["identity"], lines["wrapper"]}:
        raise SystemExit(f"source-to-sink control failed: {issue_lines} != {lines}")
    for name in ("identity", "wrapper"):
        ports = models.get(f"probe.{name}", [])
        if len(ports) != 1 or ports[0]["port"] != "formal(value, position=0)":
            raise SystemExit(f"missing exact {name} TITO formal: {ports}")
        taint = ports[0]["taint"]
        if not any(kind.get("kind") == "LocalReturn"
                   for entry in taint for kind in entry.get("kinds", [])):
            raise SystemExit(f"missing local return in {name}: {taint}")
        if "obscure" in json.dumps(taint):
            raise SystemExit(f"{name} relies on an obscure-callee default")
    if models.get("probe.constant"):
        raise SystemExit("constant control unexpectedly has TITO")
    print("passed: verified rule, two source-to-sink issues, exact TITO ports, constant control")


if __name__ == "__main__":
    main()
