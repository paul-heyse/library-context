"""Current Rust decoder and actual FastMCP schema listing; no stored fixture required."""
import json
from typing import Any

import pytest
from fastmcp import Client, FastMCP
from lctx_semantics import (
    wire_decode,
    wire_resources,
    wire_schema,
    wire_tool,
    wire_tool_result,
    wire_tools,
)
from mcp_types import CallToolResult, TextContent

from lctx_mcp.wire import register


def test_closed_nested_requests_keep_nominal_ids_and_refuse_coercion():
    request = {"library": "control", "operation": {"kind": "member", "member": [7] * 16}}
    decoded = json.loads(wire_decode("get_operation", json.dumps(request)))
    assert decoded["operation"] == request["operation"]
    assert decoded["page"] == {"size": 20, "expanded": False}
    invalid = [
        {**request, "ignored": True},
        {**request, "operation": {"kind": "member", "member": [7] * 16, "ignored": True}},
        {**request, "operation": {"kind": "member", "member": "07" * 16}},
        {**request, "operation": {"kind": "member", "member": [7] * 15}},
        {**request, "operation": {"kind": "member", "member": [True] * 16}},
        {**request, "page": {"expanded": 1}},
        {**request, "page": {"size": True}},
        {**request, "page": {"cursor": None}},
    ]
    for value in invalid:
        with pytest.raises(ValueError):
            wire_decode("get_operation", json.dumps(value))


def test_exact_scalars_use_current_transient_input_owner():
    request: dict[str, Any] = {"member": [1] * 16, "analysis": [2] * 16,
        "inputs": [{"formal": [3] * 16, "value": {"kind": "integer", "decimal": "10000000000000000000000000000000000000000"}}],
        "assumptions": {"builtin_namespace": "unknown"}}
    decoded = json.loads(wire_decode("inspect_value_paths", json.dumps(request)))
    assert decoded["inputs"] == request["inputs"]
    for value in [{"kind": "bool", "value": 1}, {"kind": "integer", "decimal": "+1"},
                  {"kind": "none", "value": None}, {"kind": "invented"}]:
        request["inputs"][0]["value"] = value
        with pytest.raises(ValueError):
            wire_decode("inspect_value_paths", json.dumps(request))


@pytest.mark.anyio
async def test_real_mcp_listing_preserves_the_sole_native_inventory():
    server = FastMCP("current schema control", dereference_schemas=False)
    register(server)
    declarations = json.loads(wire_tools())
    async with Client(server) as client:
        listed = await client.list_tools()
        assert {tool.name for tool in listed} == {row["name"] for row in declarations}
        assert len(listed) == 10
        for tool in listed:
            declared = next(row for row in declarations if row["name"] == tool.name)
            metadata = json.loads(wire_tool(tool.name))
            assert tool.input_schema == declared["request_schema"] == metadata["parameters"]
            assert tool.output_schema == declared["response_schema"] == metadata["output_schema"]
            assert tool.annotations is not None
            assert tool.annotations.read_only_hint is True
            assert tool.annotations.idempotent_hint is True
            assert tool.meta is not None
            assert tool.meta["lctx_wire_identity"] == metadata["wire_identity"]
            assert tool.input_schema["additionalProperties"] is False
        assert await client.list_resources() == []
        templates = await client.list_resource_templates()
        assert [{"uri_template": t.uri_template, "name": t.name, "mime_type": t.mime_type} for t in templates] == json.loads(wire_resources())


def test_current_response_wraps_complete_dto_and_actual_generation_key():
    response = {"generation": [5] * 16, "operation": {"resolution": "ambiguous", "candidates": []}}
    protocol = CallToolResult.model_validate_json(wire_tool_result("get_operation", json.dumps(response), False))
    assert protocol.structured_content == response
    assert protocol.is_error is False
    assert len(protocol.content) == 1
    assert isinstance(protocol.content[0], TextContent)
    assert protocol.content[0].text == "get_operation: generation-bound result"
    for invalid in [{**response, "generation": [5] * 32}, {**response, "generation": "05" * 16},
                    {**response, "legacy_snapshot": "05" * 16}, {"generation": [5] * 16}]:
        with pytest.raises(ValueError):
            wire_tool_result("get_operation", json.dumps(invalid), False)
    schema = json.loads(wire_schema("get_operation", True))
    assert schema["additionalProperties"] is False
