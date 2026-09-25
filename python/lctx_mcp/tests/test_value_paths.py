"""Typed native value-path inspection over the generated source fixture."""

from pathlib import Path

import pytest
from fastmcp import Client
from fastmcp.exceptions import ToolError
from pydantic import ValidationError

from lctx_mcp.generation import load
from lctx_mcp.operations import OperationError
from lctx_mcp.server import build_server
from lctx_mcp.value_paths import ExactPrimitive, inspect


def test_exact_primitive_rejects_cross_kind_values() -> None:
    with pytest.raises(ValidationError):
        ExactPrimitive(kind="int", value=True)
    with pytest.raises(ValidationError):
        ExactPrimitive(kind="none", value="None")
    with pytest.raises(ValidationError):
        ExactPrimitive(kind="str", value="x" * 501)


def test_native_page_reports_path_local_refutation_and_open_boundary(generation: Path) -> None:
    gen = load(generation, None)
    exact = ExactPrimitive(kind="none", value=None)
    page = inspect(gen, gen.snapshot_id, "pkg.controls.strict", "value", exact, True, 1, None)
    assert page.total_rows == page.examined_rows == 1
    assert not page.truncated and page.next_cursor is None
    assert page.paths[0].exact_input_result == "refuted_under_model"
    assert page.paths[0].value_links[0].path == "pkg/controls.py"
    assert page.paths[0].theory_work.assignments_applied == 1
    assert page.theory_work.bdd_preflight_pairs > 0
    assert page.theory_work == page.paths[0].theory_work

    open_page = inspect(
        gen,
        gen.snapshot_id,
        "pkg.controls.build",
        "label",
        ExactPrimitive(kind="str", value="x"),
        True,
        1,
        None,
    )
    assert open_page.paths == [] and open_page.boundaries[0].reason == "call_transfer"
    assert open_page.theory_work.bdd_preflight_pairs == 0
    with pytest.raises(OperationError, match="cursor"):
        inspect(
            gen, gen.snapshot_id, "pkg.controls.strict", "value", exact, True, 1, "not a cursor"
        )
    with pytest.raises(OperationError, match="snapshot"):
        inspect(gen, "00" * 16, "pkg.controls.strict", "value", exact, True, 1, None)


@pytest.mark.anyio
async def test_value_path_inspection_round_trips_as_structured_mcp(generation: Path) -> None:
    snapshot = load(generation, None).snapshot_id
    async with Client(build_server(generation, None)) as client:
        tools = {tool.name for tool in await client.list_tools()}
        assert "inspect_value_paths" in tools
        result = await client.call_tool(
            "inspect_value_paths",
            {
                "snapshot_id": snapshot,
                "operation": "pkg.controls.strict",
                "formal": "value",
                "exact_input": {"kind": "none", "value": None},
            },
        )
        assert result.structured_content is not None
        assert result.structured_content["paths"][0]["exact_input_result"] == (
            "refuted_under_model"
        )
        compatible = await client.call_tool(
            "inspect_value_paths",
            {
                "snapshot_id": snapshot,
                "operation": "pkg.controls.strict",
                "formal": "value",
                "exact_input": {"kind": "int", "value": 1},
            },
        )
        assert compatible.structured_content is not None
        assert compatible.structured_content["paths"][0]["exact_input_result"] == (
            "compatible_under_model"
        )
        assert compatible.structured_content["paths"][0]["value_links"]
        assert compatible.structured_content["theory_work"]["assignments_applied"] == 1
        with pytest.raises(ToolError):
            await client.call_tool(
                "inspect_value_paths",
                {
                    "snapshot_id": snapshot,
                    "operation": "pkg.controls.strict",
                    "formal": "value",
                    "exact_input": {"kind": "int", "value": True},
                },
            )
