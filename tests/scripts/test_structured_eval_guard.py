"""The Stage 3 evaluation set and its evaluation-only requests stay as pre-registered."""

from __future__ import annotations

import hashlib
import tomllib
from pathlib import Path

from gold_match import source_ranges

REPO = Path(__file__).resolve().parents[2]
QUESTIONS = REPO / "eval" / "behavior" / "fastmcp-4.0.5.toml"
REQUESTS = REPO / "eval" / "behavior" / "fastmcp-4.0.5.requests.toml"
# The stage-3 exit rule as committed in f6f00b7; it never relaxes after a failed exit.
STAGE3_RULE = "db8fb555d5dfc8ef499bc817120b70be5b53e866f7d11f21abb8237115d7a7ac"
TOOLS = {"get_operation", "inspect_value_paths", "find_operations", "search_operations"}


def _load(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def test_stage3_set_is_the_preregistered_one() -> None:
    spec = _load(QUESTIONS)
    stage3 = [q for q in spec["question"] if q["stage"] == 3]
    assert [q["id"] for q in stage3] == ["Q01", "Q03", "Q05", "Q09"]
    polarity = [i["polarity"] for q in stage3 for i in q["item"]]
    assert polarity.count("positive") == 23
    assert polarity.count("negative") == 5
    (rule,) = [e["rule"] for e in spec["exit"] if e["stage"] == 3]
    assert hashlib.sha256(rule.encode()).hexdigest() == STAGE3_RULE


def test_requests_are_evaluation_only() -> None:
    questions = {q["id"]: q for q in _load(QUESTIONS)["question"]}
    requests = _load(REQUESTS)["request"]
    ids = [r["id"] for r in requests]
    assert len(ids) == len(set(ids))
    for r in requests:
        assert r["question"] in questions, r["id"]
        assert r["id"].startswith(r["question"] + ".r"), r["id"]
        assert r["tool"] in TOOLS, r["id"]
        assert not {"claim", "polarity", "item", "rule"} & r.keys(), r["id"]
        if r["tool"] in ("get_operation", "inspect_value_paths"):
            assert r["operation"], r["id"]
        if r["tool"] == "inspect_value_paths":
            assert r["formal"], r["id"]
            # This is the sealed preregistration vocabulary, not the current serving DTO.
            assert r["exact_input"]["kind"] == "str"
            assert isinstance(r["exact_input"]["value"], str)


def test_source_ranges_map_to_served_paths() -> None:
    item = {"source": ["server/server.py:1951-1966", "../fastmcp_tasks/a.py:3", "docs/x.mdx"]}
    assert source_ranges("fastmcp", item) == [
        ("fastmcp/server/server.py", 1951, 1966),
        ("fastmcp_tasks/a.py", 3, 3),
    ]
