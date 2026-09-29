"""The behavioral tools over the fixture generation (ADR-0021; DESIGN §11.3; plan Stage 1)."""

from __future__ import annotations

from pathlib import Path

import pytest
from fastmcp import Client
from fastmcp.exceptions import ToolError
from support import load_native

from lctx_mcp import operations as ops
from lctx_mcp.embedder import FakeEmbedder
from lctx_mcp.server import NativeWorkers, search_members, serve

LIBRARY = "analysis_shapes"


def test_an_operation_reads_whole_with_fates_verdicts_and_lines(
    generation: Path, pg_serving
) -> None:
    gen = pg_serving.load()
    op = pg_serving.operation(gen, gen.snapshot_id, "pkg.Catalog.remove")
    assert op.access_path == "pkg.Catalog.remove"
    assert "pkg.Catalog.remove" in op.own_paths
    assert op.behavior_status == "established"
    assert [(f.value, f.verdict) for f in op.facets["raises"]] == [("KeyError", "established")]
    key = next(p for p in op.parameters if p.name == "key")
    raises = [f for f in key.fates if f.kind == "raises_when"]
    assert raises and raises[0].verdict == "conditional"
    assert raises[0].line is not None and raises[0].path is not None
    # Stage 2: the site is the raise; its guard is the condition, in the operation's places.
    assert raises[0].site_text == "raise KeyError(key)"
    assert raises[0].condition is not None
    assert raises[0].condition.startswith('opaque("key < 0")#')


def test_a_parameter_never_read_is_refuted_only_under_its_premise(
    generation: Path, pg_serving
) -> None:
    """`add_tool`'s body is its docstring, a stub whose behavior an override or caller supplies:
    "never read" is `unknown` (`abstract_body`), not refuted (ADR-0022 §Verdicts; the Stage 2 end
    review's R1). Every refutation served cites its parameter premise."""
    gen = pg_serving.load()
    op = pg_serving.operation(gen, gen.snapshot_id, "pkg.Catalog.add_tool")
    parameters = {p.name: p for p in op.parameters}
    assert set(parameters) == {"self", "fn", "name", "tags", "title"}
    # The source contract now includes the receiver. Behavioral premises cover explicit
    # inputs; retaining the receiver must not fabricate an unused/abstract-body verdict.
    receiver = parameters.pop("self")
    assert receiver.fates == []
    assert receiver.note and "never read this as unused" in receiver.note
    for p in parameters.values():
        claims = [f for f in p.fates if f.kind == "is_read"]
        assert claims, p.name
        assert (claims[0].verdict, claims[0].boundary_reason) == ("unknown", "abstract_body")
    refuted = [
        r
        for r in pg_serving.reference.table("behaviors").to_pylist()
        if r["kind"] == "is_read" and r["verdict"] == "refuted_under_model"
    ]
    assert refuted
    assert all(r["premise_key"] and r["premise_key"].startswith("Parameter[") for r in refuted)
    # A parameter with no fate at all is never called unused.
    for q in pg_serving.operation(gen, gen.snapshot_id, "pkg.Catalog.remove").parameters:
        assert q.fates or (q.note and "never read this as unused" in q.note)


def test_an_unknown_operation_names_near_spellings(generation: Path, pg_serving) -> None:
    gen = pg_serving.load()
    with pytest.raises(ValueError, match="no public operation"):
        pg_serving.operation(gen, gen.snapshot_id, "pkg.Catalog.remov")
    with pytest.raises(ValueError, match="snapshot"):
        pg_serving.operation(gen, "0" * 32, "pkg.Catalog.remove")


def test_declared_facets_answer_completely_and_raises_never_does(
    generation: Path, pg_serving
) -> None:
    gen = pg_serving.load()
    where = ops.Selection(
        requirements=[
            {"predicate": "member_kind", "kind": "method"},
            {"predicate": "facet_membership", "facet": "parameter", "value": "key"},
        ]
    )
    found = pg_serving.find(gen, where, limit=50, cursor=None)
    assert "pkg.Catalog.remove" in [m.access_path for m in found.supported["items"]]
    assert (found.unresolved.total == 0 and found.conflicting.total == 0) and found.unresolved[
        "items"
    ] == []
    raises = ops.Selection(
        requirements=[{"predicate": "facet_membership", "facet": "raises", "value": "KeyError"}]
    )
    found = pg_serving.find(gen, raises, limit=50, cursor=None)
    paths = [m.access_path for m in found.supported["items"]]
    assert {"pkg.Catalog.load", "pkg.Catalog.remove", "pkg.Catalog.reload"} <= set(paths)
    assert not (found.unresolved.total == 0 and found.conflicting.total == 0), (
        "raises is typed by Pyrefly: an untyped raise can hide a match"
    )


def test_a_behavioral_facet_lists_what_could_hide_a_match(generation: Path, pg_serving) -> None:
    gen = pg_serving.load()
    facets = pg_serving.reference.table("operation_facets").to_pylist()
    statuses = pg_serving.reference.table("operation_facet_status").to_pylist()
    forwarding = sorted(
        {
            r["value"]
            for r in facets
            if r["facet"] == "forwards_to" and r["verdict"] in ("established", "conditional")
        }
    )
    assert forwarding, "the fixture has established forwarding"
    where = ops.Selection(
        requirements=[
            {"predicate": "facet_membership", "facet": "forwards_to", "value": forwarding[0]}
        ]
    )
    found = pg_serving.find(gen, where, limit=50, cursor=None)
    assert found.supported.total >= 1
    for u in found.unresolved["items"]:
        if u.operation_id is None:
            continue  # A public member without an operation has no behavioral facet closure.
        node = bytes.fromhex(u.operation_id)
        status = next(
            r["verdict"] for r in statuses if r["node_id"] == node and r["facet"] == "forwards_to"
        )
        row = next(
            (
                r["verdict"]
                for r in facets
                if r["node_id"] == node
                and r["facet"] == "forwards_to"
                and r["value"] == forwarding[0]
            ),
            None,
        )
        assert status != "established" or row == "unknown", u.access_path
    assert (found.unresolved.total == 0 and found.conflicting.total == 0) == (
        found.unresolved.total == 0
    )


def test_lookup_keeps_each_facet_value_verdict_separate_from_completeness(
    generation: Path,
    pg_serving,
) -> None:
    gen = pg_serving.load()
    op = pg_serving.operation(gen, gen.snapshot_id, "pkg.configure", expanded=True)
    unknown = next(f for f in op.facets["delegates_to"] if f.value == "pkg.controls.Registry.add")
    assert unknown.verdict == "unknown"
    assert any(f.verdict == "established" for f in op.facets["delegates_to"])
    where = ops.Selection(
        requirements=[
            {"predicate": "facet_membership", "facet": "delegates_to", "value": unknown.value}
        ]
    )
    found = pg_serving.find(gen, where, limit=50, cursor=None)
    assert op.access_path not in {m.access_path for m in found.supported["items"]}
    assert op.access_path in {m.access_path for m in found.unresolved["items"]}
    schema = ops.Operation.model_json_schema()
    assert "$defs" in schema and "FacetValue" in schema["$defs"]


def test_pages_follow_a_bound_cursor(generation: Path, pg_serving) -> None:
    gen = pg_serving.load()
    where = ops.Selection(requirements=[{"predicate": "member_kind", "kind": "method"}])
    first = pg_serving.find(gen, where, limit=2, cursor=None)
    assert not first.supported.page_complete and first.supported.next_cursor
    second = pg_serving.find(gen, where, limit=2, cursor=first.supported.next_cursor)
    assert {m.access_path for m in first.supported["items"]}.isdisjoint(
        {m.access_path for m in second.supported["items"]}
    )
    other = ops.Selection(requirements=[{"predicate": "member_kind", "kind": "function"}])
    with pytest.raises(ValueError, match="another generation, request or ordering"):
        pg_serving.find(gen, other, limit=2, cursor=first.supported.next_cursor)
    with pytest.raises(ValueError, match="cursor"):
        pg_serving.find(gen, where, limit=2, cursor="not-a-cursor")


def test_an_unknown_facet_value_is_refused_with_near_values(generation: Path, pg_serving) -> None:
    """Only where every operation's rows for the facet are complete: `module` is."""
    gen = pg_serving.load()
    where = ops.Selection(
        requirements=[{"predicate": "facet_membership", "facet": "module", "value": "pkg.registr"}]
    )
    absent = pg_serving.find(gen, where, limit=5, cursor=None)
    assert absent.supported.total == 0 and absent.contradicted_count > 0
    # `raises` is never complete, so an absent value is not known to be absent.
    never = ops.Selection(
        requirements=[{"predicate": "facet_membership", "facet": "raises", "value": "KeyErr"}]
    )
    found = pg_serving.find(gen, never, limit=5, cursor=None)
    assert found.supported.total == 0 and not (
        found.unresolved.total == 0 and found.conflicting.total == 0
    )


@pytest.mark.anyio
async def test_search_ranks_operations_and_says_it_is_discovery(
    generation: Path, pg_serving
) -> None:
    gen = pg_serving.load(FakeEmbedder().spec)
    workers = NativeWorkers()
    served = serve(gen, None, workers)
    hits = await search_members(served, "remove a component", limit=5)
    assert hits.ranked_discovery and hits.retrieval.actual_route == "lexical-only"
    remove = next(
        row["node_id"].hex()
        for row in pg_serving.reference.table("operations").to_pylist()
        if row["access_path"] == "pkg.Catalog.remove"
    )
    assert (
        hits.supported["items"][0].operation_id == remove
    )  # Each public alias keeps its own rank.
    exact = await search_members(served, "pkg.Catalog.load", limit=5)
    await workers.close()
    assert exact.supported["items"][0].ranking.promoted
    assert exact.supported["items"][0].access_path == "pkg.Catalog.load"


@pytest.mark.anyio
async def test_the_behavioral_tools_round_trip(generation: Path, pg_serving) -> None:
    async with Client(pg_serving.server(FakeEmbedder())) as client:
        tools = {t.name for t in await client.list_tools()}
        assert {"get_operation", "find_operations", "search_operations"} <= tools
        snapshot = load_native(generation).snapshot_id
        op = await client.call_tool(
            "get_operation", {"snapshot_id": snapshot, "operation": "pkg.Catalog.remove"}
        )
        assert op.structured_content is not None
        assert op.structured_content["access_path"] == "pkg.Catalog.remove"
        found = await client.call_tool(
            "find_operations",
            {
                "library": LIBRARY,
                "selection": {
                    "requirements": [
                        {"predicate": "member_kind", "kind": "method"},
                        {"predicate": "facet_membership", "facet": "parameter", "value": "key"},
                    ]
                },
            },
        )
        assert found.structured_content is not None
        assert found.structured_content["supported"]["total"] > 0
        hits = await client.call_tool(
            "search_operations", {"library": LIBRARY, "query": "remove a component"}
        )
        assert hits.structured_content is not None
        assert hits.structured_content["ranked_discovery"] is True
        with pytest.raises(ToolError):
            await client.call_tool(
                "get_operation", {"snapshot_id": snapshot, "operation": "pkg.Nope"}
            )


def test_a_class_carries_its_constructor(generation: Path, pg_serving) -> None:
    gen = pg_serving.load()
    inits = sorted(
        r["access_path"]
        for r in pg_serving.reference.table("public_paths").to_pylist()
        if r["access_path"].endswith(".__init__")
    )
    assert inits, "the fixture has a public constructor"
    cls = inits[0].removesuffix(".__init__")
    op = pg_serving.operation(gen, gen.snapshot_id, cls)
    assert op.kind == "class" and op.behavior_status == "not_analyzed"
    assert op.constructor is not None and op.constructor.access_path.endswith("__init__")


def test_the_facet_names_are_the_codebook_s() -> None:
    """One authority for the facet vocabulary (increment 3's deep review, F4)."""
    import json

    spec = json.loads((Path(__file__).parents[3] / "specs/serving/facets.json").read_text())
    assert ops.FacetTerm.model_json_schema()["$defs"]["FacetName"]["enum"] == spec["facets"]


def test_a_class_without_a_public_constructor_could_match_any_parameter(
    generation: Path,
    pg_serving,
) -> None:
    """F3: a class's parameters are its public `__init__`'s; with none, it is never absent."""
    gen = pg_serving.load()
    where = ops.Selection(
        requirements=[
            {"predicate": "member_kind", "kind": "class"},
            {"predicate": "facet_membership", "facet": "parameter", "value": "key"},
        ]
    )
    found = pg_serving.find(gen, where, limit=50, cursor=None)
    assert not (found.unresolved.total == 0 and found.conflicting.total == 0)
    assert "pkg.Catalog" in [u.access_path for u in found.unresolved["items"]]
    assert all(
        next(
            r["verdict"]
            for r in pg_serving.reference.table("operation_facet_status").to_pylist()
            if r["node_id"] == bytes.fromhex(u.operation_id) and r["facet"] == "parameter"
        )
        != "established"
        for u in found.unresolved["items"]
    )


def test_a_value_on_unknown_rows_only_is_open_not_absent(generation: Path, pg_serving) -> None:
    """F3: a behavioral value with no established row answers `complete: false`, never "no
    operation has"."""
    gen = pg_serving.load()
    facets = pg_serving.reference.table("operation_facets").to_pylist()
    keys = {(r["facet"], r["value"]) for r in facets}
    open_values = sorted(
        key
        for key in keys
        if all(
            r["verdict"] not in ("established", "conditional")
            for r in facets
            if (r["facet"], r["value"]) == key
        )
    )
    assert open_values, "the fixture has a behavior only known through an override-open call"
    facet, value = open_values[0]
    term = ops.FacetTerm.model_validate({"facet": facet, "value": value})
    found = pg_serving.find(
        gen,
        ops.Selection(
            requirements=[
                {"predicate": "facet_membership", "facet": term.facet, "value": term.value}
            ]
        ),
        limit=5,
        cursor=None,
    )
    assert (
        found.supported.total == 0
        and not (found.unresolved.total == 0 and found.conflicting.total == 0)
        and found.unresolved.total >= 1
    )


def test_a_fate_states_its_condition_in_the_operations_places(generation: Path, pg_serving) -> None:
    """Stage 2: a raise is stated under its guard, and a conditional fate says so."""
    gen = pg_serving.load()
    op = pg_serving.operation(gen, gen.snapshot_id, "pkg.Catalog.load")
    path = next(p for p in op.parameters if p.name == "path")
    raises = [f for f in path.fates if f.kind == "raises_when"]
    assert raises and raises[0].condition is not None
    assert raises[0].condition.startswith("!truthy(path)#")
    assert all(f.verdict != "established" for f in path.fates if f.condition)
