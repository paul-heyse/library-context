"""The gold matcher, version 2 (ADR-0010's 2026-09-24 amendments), over constructed rows."""

from __future__ import annotations

import asyncio
from pathlib import Path
from types import SimpleNamespace

import ranking_check
from gold_match import MATCHER_VERSION, hits, jaccard, node_of, resolve, roots, status
from score_gold import span_recall

C, M, OTHER, T = b"C" * 16, b"M" * 16, b"O" * 16, b"T" * 16

PUBLIC = [
    {"node_id": C, "access_path": "lib.C", "kind": "class"},
    {"node_id": M, "access_path": "lib.C.m", "kind": "function"},
    # `m` inherited through a public subclass: another spelling of the same node.
    {"node_id": M, "access_path": "lib.Sub.m", "kind": "function"},
    {"node_id": OTHER, "access_path": "lib.other", "kind": "function"},
    # A second public root.
    {"node_id": T, "access_path": "lib_extra.tool", "kind": "function"},
]


def family(*operations: str) -> dict:
    return {"id": "f", "operations": list(operations)}


def test_an_inherited_spelling_is_the_members_node() -> None:
    (f,) = resolve([family("lib.Sub.m")], PUBLIC)
    assert f.nodes == {M}
    assert hits(f, M) and jaccard(f, M) == 1.0
    assert node_of(PUBLIC, "lib.Sub.m") == M
    assert MATCHER_VERSION == 2


def test_a_class_operation_matches_only_a_brief_seeded_by_the_class() -> None:
    (f,) = resolve([family("lib.C")], PUBLIC)
    assert hits(f, C)
    assert not hits(f, M)


def test_unresolved_operations_are_apart_and_stay_in_the_union() -> None:
    (f,) = resolve([family("lib.C.m", "lib.C.gone", "pydantic.BaseModel")], PUBLIC)
    assert f.unresolved_under_root == ("lib.C.gone",)
    assert f.outside_root == ("pydantic.BaseModel",)
    assert f.size == 3
    assert jaccard(f, M) == 1 / 3
    assert jaccard(f, OTHER) == 0.0


def test_the_roots_are_the_served_paths_not_the_distribution_name() -> None:
    """R2 F2: an operation under any public root is under the root."""
    assert roots(PUBLIC) == {"lib", "lib_extra"}
    (f,) = resolve([family("lib_extra.gone")], PUBLIC)
    assert f.unresolved_under_root == ("lib_extra.gone",)


def test_a_degraded_alias_blocks_a_live_run_only() -> None:
    assert status("vllm", 1) == "blocked"
    assert status("vllm", 0) == "measured"
    assert status("fake", 3) == "measured"


def test_an_unmapped_span_is_a_miss_in_the_denominator(tmp_path: Path) -> None:
    """R2 F2: a span the sources cannot map stays in (c)'s denominator."""
    (tmp_path / "a.py").write_text("x = 1\ny = 2\n")
    gold = [
        {
            "id": "f",
            "source_spans": [
                {"path": "a.py", "line_start": 1, "line_end": 1},
                {"path": "missing.py", "line_start": 1, "line_end": 1},
            ],
        }
    ]
    recalled, touched, unmapped = span_recall(gold, {"a.py": [(0, 3)]}, tmp_path, {"f"})
    assert recalled == [1, 2] and touched == [1, 2] and unmapped == 1


def test_the_ranking_check_is_blocked_by_a_degraded_live_alias(monkeypatch) -> None:
    """R2 F2: a lexical-only first rank under `vllm` is not evidence; the check exits 2."""
    seed = b"S" * 16
    brief = b"B" * 16
    gen = SimpleNamespace(
        library="fastmcp",
        tables={
            "public_paths": SimpleNamespace(
                to_pylist=lambda: [
                    {"node_id": seed, "access_path": "fastmcp.FastMCP.tool", "kind": "function"}
                ]
            )
        },
        briefs={brief: {"seed_node_id": seed}},
    )

    async def search(served, library, alias, limit):
        hit = SimpleNamespace(title="fastmcp.FastMCP.tool", capability_id=brief.hex())
        return SimpleNamespace(mode="lexical-only", hits=[hit], degraded_reason="down")

    monkeypatch.setattr(ranking_check, "load", lambda path, spec: gen)
    monkeypatch.setattr(ranking_check, "serve", lambda g, e: None)
    monkeypatch.setattr(ranking_check, "search", search)
    live = ranking_check.check(Path("gen"), "vllm", "http://127.0.0.1:1", "fm.register")
    assert asyncio.run(live) == 2
    assert asyncio.run(ranking_check.check(Path("gen"), "none", "", "fm.register")) == 0
