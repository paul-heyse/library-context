"""The behavioral tools over the fixture generation (ADR-0021; DESIGN §11.3; plan Stage 1)."""

from __future__ import annotations

from pathlib import Path

import pytest
from fastmcp import Client
from fastmcp.exceptions import ToolError

from lctx_mcp import operations as ops
from lctx_mcp.embedder import FakeEmbedder
from lctx_mcp.generation import load
from lctx_mcp.server import build_server

LIBRARY = "analysis_shapes"


def test_an_operation_reads_whole_with_fates_verdicts_and_lines(generation: Path) -> None:
    gen = load(generation, None)
    op = ops.get_operation(gen, gen.snapshot_id, "pkg.Catalog.remove")
    assert op.access_path == "pkg.Catalog.remove"
    assert "pkg.Catalog.remove" in op.own_paths
    assert op.behavior_status == "established"
    assert op.facets["raises"] == ["KeyError"]
    key = next(p for p in op.parameters if p.name == "key")
    raises = [f for f in key.fates if f.kind == "raises_when"]
    assert raises and raises[0].verdict == "conditional"
    assert raises[0].line is not None and raises[0].path is not None
    assert raises[0].site_text == "key < 0"


def test_a_parameter_without_a_fate_is_not_called_unused(generation: Path) -> None:
    gen = load(generation, None)
    op = ops.get_operation(gen, gen.snapshot_id, "pkg.Catalog.add_tool")
    silent = [p for p in op.parameters if not p.fates]
    assert silent, "add_tool's parameters are stored nowhere a call shows"
    assert all(p.note and "never read this as unused" in p.note for p in silent)


def test_an_unknown_operation_names_near_spellings(generation: Path) -> None:
    gen = load(generation, None)
    with pytest.raises(ops.OperationError, match="did you mean"):
        ops.get_operation(gen, gen.snapshot_id, "pkg.Catalog.remov")
    with pytest.raises(ops.OperationError, match="snapshot"):
        ops.get_operation(gen, "0" * 32, "pkg.Catalog.remove")


def test_declared_facets_answer_completely_and_raises_never_does(generation: Path) -> None:
    gen = load(generation, None)
    where = ops.Where(facets=[ops.FacetTerm(facet="parameter", value="key")])
    found = ops.find_operations(gen, where, limit=50, cursor=None)
    assert "pkg.Catalog.remove" in [m.access_path for m in found.matches]
    assert found.complete and found.unknown == []
    raises = ops.Where(facets=[ops.FacetTerm(facet="raises", value="KeyError")])
    found = ops.find_operations(gen, raises, limit=50, cursor=None)
    paths = [m.access_path for m in found.matches]
    assert {"pkg.Catalog.load", "pkg.Catalog.remove", "pkg.Catalog.reload"} <= set(paths)
    assert not found.complete, "raises is typed by Pyrefly: an untyped raise can hide a match"


def test_a_behavioral_facet_lists_what_could_hide_a_match(generation: Path) -> None:
    gen = load(generation, None)
    forwarding = sorted({v for f, v in gen.by_facet if f == "forwards_to"})
    assert forwarding, "the fixture has forwarding"
    where = ops.Where(facets=[ops.FacetTerm(facet="forwards_to", value=forwarding[0])])
    found = ops.find_operations(gen, where, limit=50, cursor=None)
    assert found.total >= 1
    assert not found.complete, "a behavioral facet is never complete in Stage 1"
    for u in found.unknown:
        rows = gen.behaviors.get(bytes.fromhex(u.operation_id), [])
        assert u.behavior_status == "unknown" or any(
            r["kind"] == "unfollowed" and r["verdict"] == "unknown" for r in rows
        )


def test_pages_follow_a_bound_cursor(generation: Path) -> None:
    gen = load(generation, None)
    where = ops.Where(kind="method")
    first = ops.find_operations(gen, where, limit=2, cursor=None)
    assert first.truncated and first.next_cursor
    second = ops.find_operations(gen, where, limit=2, cursor=first.next_cursor)
    assert {m.access_path for m in first.matches}.isdisjoint(
        {m.access_path for m in second.matches}
    )
    other = ops.Where(kind="function")
    with pytest.raises(ops.OperationError, match="another generation or request"):
        ops.find_operations(gen, other, limit=2, cursor=first.next_cursor)
    with pytest.raises(ops.OperationError, match="issued"):
        ops.find_operations(gen, where, limit=2, cursor="not-a-cursor")


def test_an_unknown_facet_value_is_refused_with_near_values(generation: Path) -> None:
    gen = load(generation, None)
    where = ops.Where(facets=[ops.FacetTerm(facet="raises", value="KeyErr")])
    with pytest.raises(ops.OperationError, match="KeyError"):
        ops.find_operations(gen, where, limit=5, cursor=None)


@pytest.mark.anyio
async def test_search_ranks_operations_and_says_it_is_discovery(generation: Path) -> None:
    gen = load(generation, FakeEmbedder().spec)
    index = ops.OperationIndex(gen)
    hits = await ops.search_operations(gen, index, None, "remove a component", None, 5)
    assert hits.ranked_discovery and hits.mode == "lexical-only"
    assert hits.hits[0].access_path == "pkg.Catalog.remove"
    exact = await ops.search_operations(gen, index, None, "pkg.Catalog.load", None, 5)
    assert exact.hits[0].promoted and exact.hits[0].rank_source == "exact_symbol"


@pytest.mark.anyio
async def test_the_behavioral_tools_round_trip(generation: Path) -> None:
    async with Client(build_server(generation, FakeEmbedder())) as client:
        tools = {t.name for t in await client.list_tools()}
        assert {"get_operation", "find_operations", "search_operations"} <= tools
        snapshot = load(generation, None).snapshot_id
        op = await client.call_tool(
            "get_operation", {"snapshot_id": snapshot, "operation": "pkg.Catalog.remove"}
        )
        assert op.structured_content is not None
        assert op.structured_content["access_path"] == "pkg.Catalog.remove"
        found = await client.call_tool(
            "find_operations",
            {
                "library": LIBRARY,
                "where": {"facets": [{"facet": "parameter", "value": "key"}]},
            },
        )
        assert found.structured_content is not None
        assert found.structured_content["complete"] is True
        hits = await client.call_tool(
            "search_operations", {"library": LIBRARY, "query": "remove a component"}
        )
        assert hits.structured_content is not None
        assert hits.structured_content["ranked_discovery"] is True
        with pytest.raises(ToolError):
            await client.call_tool(
                "get_operation", {"snapshot_id": snapshot, "operation": "pkg.Nope"}
            )


def test_a_class_carries_its_constructor(generation: Path) -> None:
    gen = load(generation, None)
    inits = sorted(p for p in gen.paths if p.endswith(".__init__"))
    assert inits, "the fixture has a public constructor"
    cls = inits[0].removesuffix(".__init__")
    op = ops.get_operation(gen, gen.snapshot_id, cls)
    assert op.kind == "class" and op.behavior_status == "not_analyzed"
    assert op.constructor is not None and op.constructor.access_path.endswith("__init__")
