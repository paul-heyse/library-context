"""The native member uses the workspace condition-kernel crate."""

from pathlib import Path

import pytest
from lctx_semantics import (
    ConditionGraph,
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
