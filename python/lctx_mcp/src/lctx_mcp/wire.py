"""Rust-owned schemas and MCP serialization; native serving is unavailable."""

from __future__ import annotations

import json
from typing import Any

from fastmcp.exceptions import ResourceError, ToolError
from fastmcp.resources import Resource, ResourceResult, ResourceTemplate
from fastmcp.tools import Tool, ToolResult
from mcp_types import (
    SERVER_INFO_META_KEY,
    CallToolResult,
    ErrorData,
    Implementation,
    JSONRPCError,
    JSONRPCResponse,
    ReadResourceResult,
    ToolAnnotations,
)
from mcp_types.methods import serialize_server_result
from mcp_types.version import HANDSHAKE_PROTOCOL_VERSIONS, MODERN_PROTOCOL_VERSIONS
from pydantic import PrivateAttr


def response_encodings(
    result: CallToolResult | ReadResourceResult | ErrorData,
    request_id: int | str,
    *,
    protocol_version: str | None = None,
    server_info: dict | None = None,
) -> tuple[bytes, bytes]:
    """Pinned SDK stdio and modern HTTP writers, including the actual JSON-RPC ID."""
    shaped = result.model_dump(by_alias=True, mode="json", exclude_none=True)
    if protocol_version is not None and not isinstance(result, ErrorData):
        method = "tools/call" if isinstance(result, CallToolResult) else "resources/read"
        # The pinned SDK owns version field omission and order. Its runner stamps
        # modern serverInfo after this public surface serializer has shaped the result.
        shaped = serialize_server_result(method, protocol_version, shaped)
        if protocol_version in MODERN_PROTOCOL_VERSIONS and server_info is not None:
            meta = shaped.get("_meta")
            if meta is None:
                shaped["_meta"] = {SERVER_INFO_META_KEY: server_info}
            elif isinstance(meta, dict) and meta.get(SERVER_INFO_META_KEY) is None:
                shaped["_meta"] = {**meta, SERVER_INFO_META_KEY: server_info}
    envelope = (
        JSONRPCError(jsonrpc="2.0", id=request_id, error=result)
        if isinstance(result, ErrorData)
        else JSONRPCResponse(jsonrpc="2.0", id=request_id, result=shaped)
    )
    serialized = envelope.model_dump_json(by_alias=True, exclude_unset=True).encode("utf-8")
    stdio = serialized + b"\n"
    http = (
        serialized
        if protocol_version in HANDSHAKE_PROTOCOL_VERSIONS
        else json.dumps(
            envelope.model_dump(by_alias=True, mode="json", exclude_none=True),
            separators=(",", ":"),
        ).encode("utf-8")
    )
    return stdio, http


def negotiated_encodings(
    result: CallToolResult | ReadResourceResult | ErrorData,
    request_id: int | str,
    protocol_version: str,
    server,
) -> tuple[bytes, bytes]:
    identity = Implementation(
        name=server.name,
        version=server.version,
        website_url=server.website_url,
        icons=server.icons or None,
    ).model_dump(by_alias=True, mode="json", exclude_none=True)
    return response_encodings(
        result, request_id, protocol_version=protocol_version, server_info=identity
    )


class SchemaTool(Tool):
    _byte_limits: dict[str, int] = PrivateAttr()

    async def run(self, arguments: dict[str, Any]) -> ToolResult:
        raise ToolError("unavailable: native graph serving is not implemented")


class CapabilityResource(Resource):
    _capability: str = PrivateAttr()
    _byte_limits: dict[str, int] = PrivateAttr()

    async def read(self) -> ResourceResult:
        raise ResourceError("unavailable: native graph serving is not implemented")


class CapabilityTemplate(ResourceTemplate):
    _byte_limits: dict[str, int] = PrivateAttr()

    async def create_resource(self, uri: str, params: dict[str, Any]) -> Resource:
        resource = CapabilityResource(uri=uri, name=self.name, mime_type=self.mime_type)
        resource._capability = params["capability"]
        resource._byte_limits = self._byte_limits
        return resource


def register(server) -> None:
    """Register the sole Rust inventory without Python domain field definitions."""
    from lctx_semantics import wire_resources, wire_tool, wire_tools

    for declaration in json.loads(wire_tools()):
        name = declaration["name"]
        contract = json.loads(wire_tool(name))
        tool = SchemaTool(
            name=name,
            description=declaration["description"],
            parameters=declaration["request_schema"],
            output_schema=declaration["response_schema"],
            annotations=ToolAnnotations(
                read_only_hint=declaration["read_only"],
                idempotent_hint=declaration["idempotent"],
                open_world_hint=False,
            ),
            meta={"lctx_wire_identity": contract["wire_identity"]},
        )
        tool._byte_limits = contract["byte_limits"]
        server.add_tool(tool)
    for declaration in json.loads(wire_resources()):
        template = CapabilityTemplate(
            uri_template=declaration["uri_template"],
            name=declaration["name"],
            mime_type=declaration["mime_type"],
            description=declaration["description"],
            parameters={
                "type": "object",
                "properties": {"capability": {"type": "string"}},
                "required": ["capability"],
            },
        )
        template._byte_limits = json.loads(wire_tool("get_capability"))["byte_limits"]
        server.add_template(template)
