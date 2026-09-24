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
    # Stage 2: the site is the raise; its guard is the condition, in the operation's places.
    assert raises[0].site_text == "raise KeyError(key)"
    assert raises[0].condition == 'opaque("key < 0")'


def test_a_parameter_never_read_is_refuted_only_under_its_premise(generation: Path) -> None:
    """`add_tool`'s body is its docstring, a stub whose behavior an override or caller supplies:
    "never read" is `unknown` (`abstract_body`), not refuted (ADR-0022 §Verdicts; the Stage 2 end
    review's R1). Every refutation served cites its parameter premise."""
    gen = load(generation, None)
    op = ops.get_operation(gen, gen.snapshot_id, "pkg.Catalog.add_tool")
    for p in op.parameters:
        claims = [f for f in p.fates if f.kind == "is_read"]
        assert claims, p.name
        assert (claims[0].verdict, claims[0].boundary_reason) == ("unknown", "abstract_body")
    refuted = [
        r
        for rows in gen.behaviors.values()
        for r in rows
        if r["kind"] == "is_read" and r["verdict"] == "refuted_under_model"
    ]
    assert refuted
    assert all(r["premise_key"] and r["premise_key"].startswith("Parameter[") for r in refuted)
    # A parameter with no fate at all is never called unused.
    for q in ops.get_operation(gen, gen.snapshot_id, "pkg.Catalog.remove").parameters:
        assert q.fates or (q.note and "never read this as unused" in q.note)


def test_an_unknown_operation_names_near_spellings(generation: Path) -> None:
    gen = load(generation, None)
    with pytest.raises(ops.OperationError, match="did you mean"):
        ops.get_operation(gen, gen.snapshot_id, "pkg.Catalog.remov")
    with pytest.raises(ops.OperationError, match="snapshot"):
        ops.get_operation(gen, "0" * 32, "pkg.Catalog.remove")


def test_declared_facets_answer_completely_and_raises_never_does(generation: Path) -> None:
    gen = load(generation, None)
    where = ops.Where(kind="method", facets=[ops.FacetTerm(facet="parameter", value="key")])
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
    forwarding = sorted(
        v
        for (f, v), rows in gen.by_facet.items()
        if f == "forwards_to" and any(x in ops.MATCHING for x in rows.values())
    )
    assert forwarding, "the fixture has established forwarding"
    where = ops.Where(facets=[ops.FacetTerm(facet="forwards_to", value=forwarding[0])])
    found = ops.find_operations(gen, where, limit=50, cursor=None)
    assert found.total >= 1
    for u in found.unknown:
        node = bytes.fromhex(u.operation_id)
        status = gen.facet_status[node]["forwards_to"][0]
        row = gen.by_facet.get(("forwards_to", forwarding[0]), {}).get(node)
        assert status != "established" or row == "unknown", u.access_path
    assert found.complete == (found.unknown_total == 0)


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
    """Only where every operation's rows for the facet are complete: `module` is."""
    gen = load(generation, None)
    where = ops.Where(facets=[ops.FacetTerm(facet="module", value="pkg.registr")])
    with pytest.raises(ops.OperationError, match=r"pkg\.registry"):
        ops.find_operations(gen, where, limit=5, cursor=None)
    # `raises` is never complete, so an absent value is not known to be absent.
    never = ops.Where(facets=[ops.FacetTerm(facet="raises", value="KeyErr")])
    found = ops.find_operations(gen, never, limit=5, cursor=None)
    assert found.total == 0 and not found.complete


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
                "where": {"kind": "method", "facets": [{"facet": "parameter", "value": "key"}]},
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


def test_the_facet_names_are_the_codebook_s() -> None:
    """One authority for the facet vocabulary (increment 3's deep review, F4)."""
    import json
    from typing import get_args

    spec = json.loads((Path(__file__).parents[3] / "specs/serving/facets.json").read_text())
    assert list(get_args(ops.FacetName)) == spec["facets"]


def test_a_class_without_a_public_constructor_could_match_any_parameter(
    generation: Path,
) -> None:
    """F3: a class's parameters are its public `__init__`'s; with none, it is never absent."""
    gen = load(generation, None)
    where = ops.Where(kind="class", facets=[ops.FacetTerm(facet="parameter", value="key")])
    found = ops.find_operations(gen, where, limit=50, cursor=None)
    assert not found.complete
    assert "pkg.Catalog" in [u.access_path for u in found.unknown]
    assert all(
        gen.facet_status[bytes.fromhex(u.operation_id)]["parameter"][0] != "established"
        for u in found.unknown
    )


def test_a_value_on_unknown_rows_only_is_open_not_absent(generation: Path) -> None:
    """F3: a behavioral value with no established row answers `complete: false`, never "no
    operation has"."""
    gen = load(generation, None)
    open_values = [
        key
        for key, rows in gen.by_facet.items()
        if rows and all(v not in ops.MATCHING for v in rows.values())
    ]
    assert open_values, "the fixture has a behavior only known through an override-open call"
    facet, value = open_values[0]
    term = ops.FacetTerm.model_validate({"facet": facet, "value": value})
    found = ops.find_operations(gen, ops.Where(facets=[term]), limit=5, cursor=None)
    assert found.total == 0 and not found.complete and found.unknown_total >= 1


def test_a_fate_states_its_condition_in_the_operations_places(generation: Path) -> None:
    """Stage 2: a raise is stated under its guard, and a conditional fate says so."""
    gen = load(generation, None)
    op = ops.get_operation(gen, gen.snapshot_id, "pkg.Catalog.load")
    path = next(p for p in op.parameters if p.name == "path")
    raises = [f for f in path.fates if f.kind == "raises_when"]
    assert raises and raises[0].condition == "!truthy(path)"
    assert all(f.verdict != "established" for f in path.fates if f.condition)
