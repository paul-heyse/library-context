"""Current Rust decoder and actual FastMCP schema listing; no stored fixture required."""

import json
from typing import Any

import pytest
from fastmcp import Client, FastMCP
from lctx_semantics import (
    NativeFailure,
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
        with pytest.raises(NativeFailure):
            wire_decode("get_operation", json.dumps(value))


def test_exact_scalars_use_current_transient_input_owner():
    request: dict[str, Any] = {
        "member": [1] * 16,
        "analysis": [2] * 16,
        "inputs": [
            {
                "formal": [3] * 16,
                "value": {
                    "kind": "integer",
                    "decimal": "10000000000000000000000000000000000000000",
                },
            }
        ],
        "assumptions": {"builtin_namespace": "unknown"},
    }
    decoded = json.loads(wire_decode("inspect_value_paths", json.dumps(request)))
    assert decoded["inputs"] == request["inputs"]
    for value in [
        {"kind": "bool", "value": 1},
        {"kind": "integer", "decimal": "+1"},
        {"kind": "none", "value": None},
        {"kind": "invented"},
    ]:
        request["inputs"][0]["value"] = value
        with pytest.raises(NativeFailure):
            wire_decode("inspect_value_paths", json.dumps(request))


def test_enrichment_sections_keep_native_variant_selection_and_scope_schema():
    request: dict[str, Any] = {
        "library": "demo",
        "operation": {"kind": "member", "member": [1] * 16},
        "sections": ["callable_comparison", "contextual_typing", "incoming_references"],
        "comparison": {"analysis": [2] * 16, "left": [3] * 16, "right": [4] * 16},
    }
    decoded = json.loads(wire_decode("get_operation", json.dumps(request)))
    assert decoded["comparison"] == request["comparison"]
    del request["comparison"]
    with pytest.raises(NativeFailure):
        wire_decode("get_operation", json.dumps(request))
    response_schema = wire_schema("GetOperationResponse", True)
    assert "external_consumers_unknown" in response_schema
    assert "runtime_value" in response_schema
    assert "formal_identity" in response_schema
    evidence_schema = wire_schema("GetEvidenceResponse", True)
    assert "flow_inventory" in evidence_schema
    assert "entry_value_reason" in evidence_schema
    assert "native_count" in evidence_schema
    for field in ["reachability", "narrowing", "narrowing_unavailable", "narrowing_precision_lost"]:
        assert field in evidence_schema


@pytest.mark.anyio
async def test_real_mcp_listing_preserves_the_sole_native_inventory():
    server = FastMCP("current schema control", dereference_schemas=False)
    register(server, object())
    declarations = json.loads(wire_tools())
    async with Client(server) as client:
        listed = await client.list_tools()
        assert {tool.name for tool in listed} == {row["name"] for row in declarations}
        assert len(listed) == 10
        for tool in listed:
            declared = next(row for row in declarations if row["name"] == tool.name)
            metadata = json.loads(wire_tool(tool.name))
            assert tool.description == declared["description"]
            assert tool.description
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
        assert [
            {
                "uri_template": t.uri_template,
                "name": t.name,
                "mime_type": t.mime_type,
                "description": t.description,
            }
            for t in templates
        ] == json.loads(wire_resources())


def test_current_response_wraps_complete_dto_and_actual_snapshot_handle():
    response = {
        "snapshot": {
            "semantic": [5] * 32,
            "realization": [6] * 32,
            "database": {"namespace": "control", "database": "snapshot"},
        },
        "domains": [],
        "operation": {"resolution": "ambiguous", "candidates": []},
    }
    protocol = CallToolResult.model_validate_json(
        wire_tool_result("get_operation", json.dumps(response), False)
    )
    assert protocol.structured_content == response
    assert protocol.is_error is False
    assert len(protocol.content) == 1
    assert isinstance(protocol.content[0], TextContent)
    assert protocol.content[0].text == "get_operation: snapshot-bound result"
    for invalid in [
        {key: value for key, value in response.items() if key != "domains"},
        {**response, "snapshot": [5] * 32},
        {**response, "snapshot": "05" * 16},
        {**response, "legacy_snapshot": "05" * 16},
        {"snapshot": response["snapshot"]},
    ]:
        with pytest.raises(NativeFailure):
            wire_tool_result("get_operation", json.dumps(invalid), False)
    schema = json.loads(wire_schema("get_operation", True))
    assert schema["additionalProperties"] is False


def test_unsupported_facets_are_refused_by_native_request_admission():
    request = {
        "library": "control",
        "selection": {
            "requirements": [
                {
                    "predicate": {
                        "FacetMembership": {
                            "facet": 6,
                            "value": {"ParameterName": {"name": "timeout"}},
                        }
                    },
                    "quantifier": 0,
                }
            ],
            "mode": 0,
            "joint": 1,
        },
    }
    with pytest.raises(ValueError, match="facet membership"):
        wire_decode("find_operations", json.dumps(request))


def test_native_response_requires_resolved_claim_assumption_basis():
    schema = json.loads(wire_schema("inspect_value_paths", True))
    definitions = schema["$defs"]
    packet = definitions["NativeAssessmentPacket"]
    assert "claim_basis" in packet["required"]
    basis = definitions["ClaimBasisPacket"]
    assert {"set", "members_digest", "definitions"}.issubset(basis["required"])
    premise = definitions["ClaimAssumptionPacket"]
    arms = premise.get("oneOf", premise.get("anyOf", []))
    assert len(arms) == 2
    for arm in arms:
        assert "support" in arm["required"]
    universe = definitions["AssumptionUniversePacket"]
    assert {"model_definition", "source", "support", "catalog", "model"}.issubset(
        universe["required"]
    )
