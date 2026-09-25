"""The native member uses the workspace condition-kernel crate."""

from pathlib import Path

import pytest
from lctx_semantics import (
    ConditionGraph,
    SemanticExecutor,
    catalog_limits,
    kernel_format,
    probe_compatible,
    probe_implies,
)

from lctx_mcp.generation import load


def test_native_condition_boundary_uses_the_rust_kernel() -> None:
    assert kernel_format() == 1
    assert catalog_limits() == (100_000, 100_000)
    assert probe_compatible("truthy(a)", "!truthy(a)") is False
    assert probe_compatible("truthy(a)", "!truthy(b)") is True
    assert probe_implies("truthy(a) & truthy(b)", "truthy(a)") is True
    assert probe_implies("truthy(a)", "truthy(b)") is False
    assert probe_compatible("over_budget", "true") is None
    assert probe_compatible("truthy(a)" * 10_000, "true") is None
    with pytest.raises(ValueError):
        probe_compatible("not a condition", "true")


def test_generation_condition_graph_is_hydrated_and_rejects_bad_nodes(generation: Path) -> None:
    loaded = load(generation, None)
    graph = loaded.condition_graph
    assert graph is not None and graph.condition_count > 0 and graph.node_count > 0
    rows = loaded.tables["conditions"].to_pylist()
    condition = next(r["condition_id"].hex() for r in rows if r["root_id"] is not None)
    assert graph.compatible(condition, condition) == (True, None)
    assert graph.implies(condition, condition) == (True, None)

    conditions = [
        (
            r["condition_id"].hex(),
            None if r["root_id"] is None else r["root_id"].hex(),
            r["boundary_reason"],
        )
        for r in rows
    ]
    nodes = [
        (r["node_id"].hex(), r["atom"], r["low_id"].hex(), r["high_id"].hex())
        for r in loaded.tables["condition_nodes"].to_pylist()
    ]
    first = nodes[0]
    nodes[0] = (first[0], first[1] + "x", first[2], first[3])
    with pytest.raises(ValueError):
        ConditionGraph(kernel_format(), conditions, nodes)
    with pytest.raises(ValueError, match="format"):
        ConditionGraph(kernel_format() + 1, conditions, nodes)


def test_native_index_resolves_one_public_formal_and_its_proof(generation: Path) -> None:
    index = load(generation, None).condition_graph
    assert index is not None
    paths, reasons, truncated, work = index.value_paths(
        "pkg.controls.passthrough", "options", 10
    )
    assert len(paths) == 1 and paths[0][1] == "established"
    assert len(paths[0][3]) > 0 and reasons == []
    assert not truncated and work == 1
    with pytest.raises(ValueError, match="unknown operation formal"):
        index.value_paths("pkg.controls.passthrough", "absent", 10)
    with pytest.raises(ValueError, match="unknown public operation"):
        index.value_paths("pkg.absent", "options", 10)
    with pytest.raises(ValueError, match="limit"):
        index.value_paths("pkg.controls.passthrough", "options", 0)


def test_native_index_refuses_missing_proof_steps(generation: Path) -> None:
    loaded = load(generation, None)
    conditions = [
        (r["condition_id"].hex(),
         None if r["root_id"] is None else r["root_id"].hex(), r["boundary_reason"])
        for r in loaded.tables["conditions"].to_pylist()
    ]
    nodes = [
        (r["node_id"].hex(), r["atom"], r["low_id"].hex(), r["high_id"].hex())
        for r in loaded.tables["condition_nodes"].to_pylist()
    ]
    condition = next(r[0] for r in conditions if r[1] is not None)
    operation, formal, summary = ("01" * 16, "02" * 16, "03" * 16)
    args = (kernel_format(), loaded.snapshot_id, loaded.manifest["entry_value_effect_digest"],
            conditions, nodes, [operation], [("pkg.one", operation)],
            [(operation, formal, "value")],
            [(summary, operation, formal, condition, "established", None, 0)])
    with pytest.raises(ValueError, match="no proof steps"):
        SemanticExecutor(*args, [], [], [], [])
    with pytest.raises(ValueError, match="missing cited callee summary"):
        SemanticExecutor(*args, [(summary, 0, "callee_summary", "04" * 16, condition)],
                         [], [], [])


def test_exact_input_refutes_only_a_cited_summary_path(generation: Path) -> None:
    index = load(generation, None).condition_graph
    assert index is not None
    paths, _, _, _ = index.value_paths("pkg.controls.strict", "value", 10)
    assert len(paths) == 1
    summary = paths[0][0]

    verdict, proof, boundary = index.assess_value_path(
        "pkg.controls.strict", "value", summary, "none", "", True
    )
    assert verdict == "refuted_under_model" and boundary is None
    assert len(proof) == 1 and proof[0][1] == "pkg/controls.py"
    assert proof[0][2] < proof[0][3]

    # A satisfiable remainder after a checked link is may-model compatibility only.
    compatible, linked, boundary = index.assess_value_path(
        "pkg.controls.strict", "value", summary, "int", "1", True
    )
    assert compatible == "compatible_under_model" and len(linked) == 1
    assert boundary is None
    # `is None` does not depend on a builtin name in the runtime namespace.
    assert index.assess_value_path(
        "pkg.controls.strict", "value", summary, "none", "", False
    )[0] == "refuted_under_model"

    paths, boundaries, total, truncated, work = index.inspect_value_paths(
        "pkg.controls.strict", "value", "none", "", True, 0, 1
    )
    assert len(paths) == total == work == 1
    assert paths[0][0] == summary and paths[0][4] == "refuted_under_model"
    assert boundaries == [] and not truncated
    assert index.inspect_value_paths(
        "pkg.controls.strict", "value", "none", "", True, 1, 1
    ) == ([], [], 1, False, 0)
    with pytest.raises(ValueError, match="cursor offset"):
        index.inspect_value_paths("pkg.controls.strict", "value", "none", "", True, 2, 1)

    paths, boundaries, total, truncated, work = index.inspect_value_paths(
        "pkg.controls.build", "label", "str", "x", True, 0, 1
    )
    assert paths == [] and len(boundaries) == total == work == 1
    assert boundaries[0][2] == "call_transfer" and not truncated


def test_native_value_path_page_has_stable_bounded_boundary_order(generation: Path) -> None:
    loaded = load(generation, None)
    conditions = [
        (r["condition_id"].hex(),
         None if r["root_id"] is None else r["root_id"].hex(), r["boundary_reason"])
        for r in loaded.tables["conditions"].to_pylist()
    ]
    nodes = [
        (r["node_id"].hex(), r["atom"], r["low_id"].hex(), r["high_id"].hex())
        for r in loaded.tables["condition_nodes"].to_pylist()
    ]
    condition = next(r[0] for r in conditions if r[1] is not None)
    operation, formal = "01" * 16, "02" * 16
    index = SemanticExecutor(
        kernel_format(), loaded.snapshot_id, loaded.manifest["entry_value_effect_digest"],
        conditions, nodes, [operation], [("pkg.one", operation)],
        [(operation, formal, "value")], [], [],
        [(operation, formal, "04" * 16, condition, "call_transfer"),
         (operation, formal, "03" * 16, condition, "unsupported_control_flow")],
        [], [],
    )
    first = index.inspect_value_paths("pkg.one", "value", "none", "", True, 0, 1)
    second = index.inspect_value_paths("pkg.one", "value", "none", "", True, 1, 1)
    assert first[2:] == (2, True, 1)
    assert second[2:] == (2, False, 1)
    assert first[1][0][0] == "03" * 16 and second[1][0][0] == "04" * 16
