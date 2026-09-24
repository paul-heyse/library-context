"""The gold matcher, version 2 (ADR-0010's 2026-09-24 amendment), over constructed rows."""

from __future__ import annotations

from gold_match import MATCHER_VERSION, hits, jaccard, node_of, resolve
from score_gold import status

C, M, SUB_M, OTHER = b"C" * 16, b"M" * 16, b"M" * 16, b"O" * 16

PUBLIC = [
    {"node_id": C, "access_path": "lib.C"},
    {"node_id": M, "access_path": "lib.C.m"},
    # `m` inherited through a public subclass: another spelling of the same node.
    {"node_id": SUB_M, "access_path": "lib.Sub.m"},
    {"node_id": OTHER, "access_path": "lib.other"},
]


def family(*operations: str) -> dict:
    return {"id": "f", "operations": list(operations)}


def test_an_inherited_spelling_is_the_members_node() -> None:
    (f,) = resolve([family("lib.Sub.m")], PUBLIC, "lib")
    assert f.nodes == {M}
    assert hits(f, M) and jaccard(f, M) == 1.0
    assert node_of(PUBLIC, "lib.Sub.m") == M
    assert MATCHER_VERSION == 2


def test_a_class_operation_matches_only_a_brief_seeded_by_the_class() -> None:
    (f,) = resolve([family("lib.C")], PUBLIC, "lib")
    assert hits(f, C)
    assert not hits(f, M)


def test_unresolved_operations_are_apart_and_stay_in_the_union() -> None:
    (f,) = resolve([family("lib.C.m", "lib.C.gone", "pydantic.BaseModel")], PUBLIC, "lib")
    assert f.unresolved_under_root == ("lib.C.gone",)
    assert f.outside_root == ("pydantic.BaseModel",)
    assert f.size == 3
    assert jaccard(f, M) == 1 / 3
    assert jaccard(f, OTHER) == 0.0


def test_a_degraded_alias_blocks_a_live_run_only() -> None:
    assert status("vllm", 1) == "blocked"
    assert status("vllm", 0) == "measured"
    assert status("fake", 3) == "measured"
