"""Independent hand-derived finite populations, not a production compiler input.

The two miniature grammars explicitly differ at falsey override. Their world
answers are separately authored and never obtained from production classification.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any


def evidence(text: str, anchor: str, role: str, variant: str = "A") -> dict[str, Any]:
    return {"context": {"release": "1", "variant": variant}, "evidence": [{
        "text": text, "anchor": anchor, "role": role,
        "qualifications": [{"id": "source", "text": "From the pinned release source"}],
        "candidate_status": "supported"}]}


def observation(groups: list[dict[str, Any]]) -> dict[str, Any]:
    packet = {"realization": "finite-render-1", "groups": copy.deepcopy(groups), "references": []}
    return {"realization": "finite-render-1", "segments": [json.dumps(packet, ensure_ascii=False)],
            "observer_format": "finite_packet_v1", "expansions": [],
            "status": "completed", "failure": None}


def mutate_packet(case: dict[str, Any], change: Any) -> None:
    packet = json.loads(case["observation"]["segments"][0])
    change(packet)
    case["observation"]["segments"][0] = json.dumps(packet, ensure_ascii=False)


def predicate(name: str, text: list[str], role: str | None = None) -> dict[str, Any]:
    return {"name": name, "role": role or name, "accepted_text": text,
            "anchors": [name, name + "-alternate"], "context": {"release": "1"},
            "qualifications": [{"id": "source", "accepted_text": ["From the pinned release source"]}], "candidate_status": "supported"}


def leaf(name: str) -> dict[str, Any]:
    return {"kind": "leaf", "predicate": name}


def base_case() -> dict[str, Any]:
    requirements = [predicate("signature", ["def connect(timeout=None)"]),
                    predicate("default", ["default timeout is 10", "timeout defaults to ten"]),
                    predicate("setup", ["requires installed transport"])]
    task = {"id": "compatible", "split": "development", "family": "context-conjunction",
            "request": {"question": "How do I call connect with its default?",
                        "context": {"release": "1"}, "allowed_followups": ["section"]},
            "envelope": {"max_calls": 2, "max_bytes": 4096}, "intent": "positive",
            "oracle": {"kind": "hand_derived_finite", "input_digest": "finite-source-v1",
                       "qualification": "supported miniature class only", "supported_domain": "connect-A-B",
                       "revision": "1", "completeness": "complete", "unknown_limits": []},
            "predicates": requirements,
            "witness": {"kind": "exists", "variables": ["variant"],
                        "child": {"kind": "all", "children": [leaf(p["name"]) for p in requirements]}},
            "model": None}
    return {"task": task, "observation": observation([
        evidence("def connect(timeout=None)", "signature", "signature"),
        evidence("default timeout is 10", "default", "default"),
        evidence("requires installed transport", "setup", "setup")]), "mode": "immediate"}


def populations() -> list[tuple[str, dict[str, Any], str]]:
    base = base_case()
    rows = [("compatible", base, "sufficient")]
    def add(name: str, change: Any, expected: str) -> None:
        case = copy.deepcopy(base)
        case["task"]["id"] = name
        change(case)
        rows.append((name, case, expected))
    add("foreign-variant", lambda c: mutate_packet(c, lambda p: p["groups"][1]["context"].update(variant="B")), "insufficient")
    add("foreign-release", lambda c: mutate_packet(c, lambda p: p["groups"][1]["context"].update(release="2")), "insufficient")
    add("wrong-anchor", lambda c: mutate_packet(c, lambda p: p["groups"][1]["evidence"][0].update(anchor="unrelated")), "insufficient")
    add("drop-setup", lambda c: mutate_packet(c, lambda p: p["groups"].pop()), "insufficient")
    add("qualification-id-only", lambda c: mutate_packet(c, lambda p: p["groups"][1]["evidence"][0]["qualifications"][0].update(text="")), "insufficient")
    add("same-substring-foreign-association", lambda c: mutate_packet(c, lambda p: (
        p["groups"][1]["context"].update(variant="B"),
        p["groups"].append(evidence("default timeout is 10", "unrelated", "default", "A")))), "insufficient")
    add("correct-ids-hidden-meaning", lambda c: c.update(observation=observation([
        evidence("signature default setup", "signature", "signature")])), "insufficient")
    add("positive-alternate", lambda c: c.update(observation=observation([
        evidence("def connect(timeout=None)", "signature-alternate", "signature"),
        evidence("timeout defaults to ten", "default-alternate", "default"),
        evidence("requires installed transport", "setup-alternate", "setup")])), "sufficient")
    add("empty-positive", lambda c: c["task"].update(witness={"kind": "all", "children": []}), "inconclusive")
    add("explicit-not-applicable", lambda c: c["task"].update(intent="not_applicable", witness=None), "not_applicable")
    add("unsupported", lambda c: c["task"]["oracle"].update(completeness="unsupported"), "inconclusive")
    add("incomplete", lambda c: c["task"]["oracle"].update(completeness="incomplete"), "inconclusive")
    add("unknown-intent", lambda c: c["task"].update(intent="unknown"), "model_relative_unknown")
    add("mislabel-expected-failure", lambda c: mutate_packet(c, lambda p: p["groups"][1]["evidence"][0].update(candidate_status="expected_failure")), "insufficient")
    add("no-budget", lambda c: c["task"]["envelope"].update(max_bytes=1), "budget_infeasible")
    add("execution-failed", lambda c: c["observation"].update(status="failed", failure="renderer failed"), "inconclusive")
    twin = copy.deepcopy(base)
    twin["task"].update(id="satisfiable-twins", family="falsey-override", predicates=[
        predicate("signature", ["def connect(timeout=None)"]),
        predicate("none-rule", ["timeout = 10 if timeout is None else timeout"], "source"),
        predicate("truthy-rule", ["timeout = timeout or 10"], "source")], witness=leaf("signature"),
        model={"worlds": [
            {"name": "none-override", "context": {"release": "1", "variant": "A"}, "facts": {"rule": "none"}, "answer": "0"},
            {"name": "truthiness-override", "context": {"release": "1", "variant": "A"}, "facts": {"rule": "truthy"}, "answer": "10"}],
            "information": [{"fact": "rule", "value": "none", "predicate": "none-rule"},
                            {"fact": "rule", "value": "truthy", "predicate": "truthy-rule"}]})
    twin["task"]["request"]["question"] = "What is timeout when caller passes zero?"
    twin["observation"] = observation([evidence("def connect(timeout=None)", "signature", "signature")])
    rows.append(("satisfiable-twins", twin, "insufficient"))
    distinguished = copy.deepcopy(twin)
    distinguished["task"]["id"] = "readable-distinction"
    distinguished["observation"] = observation([evidence("def connect(timeout=None)", "signature", "signature"),
        evidence("timeout = 10 if timeout is None else timeout", "none-rule", "source")])
    rows.append(("readable-distinction", distinguished, "sufficient"))
    inconsistent = copy.deepcopy(twin)
    inconsistent["task"]["id"] = "empty-worlds"
    inconsistent["task"]["model"]["worlds"] = []
    rows.append(("empty-worlds", inconsistent, "inconsistent_model"))
    conflict = copy.deepcopy(distinguished)
    conflict["task"]["id"] = "qualified-conflict"
    conflict["observation"] = observation([evidence("def connect(timeout=None)", "signature", "signature"),
        evidence("timeout = 10 if timeout is None else timeout", "none-rule", "source"),
        evidence("timeout = timeout or 10", "truthy-rule", "source")])
    rows.append(("qualified-conflict", conflict, "insufficient"))
    for _, case, _ in rows:
        if case["task"]["family"] == "falsey-override":
            case["task"]["request"]["context"]["override"] = "0"
            for world in case["task"]["model"]["worlds"]:
                world["context"]["override"] = "0"
            mutate_packet(case, lambda p: [group["context"].update(override="0") for group in p["groups"]])
    return rows


if __name__ == "__main__":
    root = Path(__file__).resolve().parent
    rows = populations()
    (root / "finite-cases.jsonl").write_text("".join(json.dumps(case, ensure_ascii=False) + "\n" for _, case, _ in rows), encoding="utf-8")
    (root / "finite-expectations.json").write_text(json.dumps({name: expected for name, _, expected in rows}, indent=2) + "\n", encoding="utf-8")
    (root / "none-override.py").write_text("def connect(timeout=None):\n    return 10 if timeout is None else timeout\n", encoding="utf-8")
    (root / "truthiness-override.py").write_text("def connect(timeout=None):\n    return timeout or 10\n", encoding="utf-8")
