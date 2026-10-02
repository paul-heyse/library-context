"""Closed Rust schemas, opaque grants and actual MCP envelope serialization."""
from __future__ import annotations

import json
from typing import Any

from fastmcp.exceptions import ResourceError, ToolError, ValidationError
from fastmcp.resources import Resource, ResourceContent, ResourceResult, ResourceTemplate
from fastmcp.server.dependencies import get_context
from fastmcp.tools import Tool, ToolResult
from mcp_types import (
    SERVER_INFO_META_KEY,
    CallToolResult,
    Implementation,
    JSONRPCResponse,
    ReadResourceResult,
    ToolAnnotations,
)
from mcp_types.methods import serialize_server_result
from mcp_types.version import MODERN_PROTOCOL_VERSIONS
from pydantic import PrivateAttr


def response_encodings(result: CallToolResult | ReadResourceResult, request_id: int | str, *, protocol_version: str | None = None, server_info: dict | None = None) -> tuple[bytes, bytes]:
    """Pinned SDK stdio and modern HTTP writers, including the actual JSON-RPC ID."""
    shaped = result.model_dump(by_alias=True, mode="json", exclude_none=True)
    if protocol_version is not None:
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
    envelope = JSONRPCResponse(jsonrpc="2.0", id=request_id, result=shaped)
    stdio = (envelope.model_dump_json(by_alias=True, exclude_unset=True) + "\n").encode("utf-8")
    http = json.dumps(
        envelope.model_dump(by_alias=True, mode="json", exclude_none=True),
        separators=(",", ":"),
    ).encode("utf-8")
    return stdio, http


def negotiated_encodings(result: CallToolResult | ReadResourceResult, request_id: int | str, protocol_version: str, server) -> tuple[bytes, bytes]:
    identity = Implementation(
        name=server.name, version=server.version,
        website_url=server.website_url, icons=server.icons or None,
    ).model_dump(by_alias=True, mode="json", exclude_none=True)
    return response_encodings(result, request_id, protocol_version=protocol_version, server_info=identity)


async def admit_request_id(service, grant, request_context, expanded: bool, byte_limits: dict[str, int]) -> int | str:
    request_id = request_context.request_id
    bound = byte_limits["expanded" if expanded else "default"]
    # An ID alone is a lower bound on both envelopes. Avoid an unbounded copy before
    # checking the Rust-declared limit; native admission checks both full encodings.
    if isinstance(request_id, str) and len(request_id) > bound:
        raise ToolError("resource_refused: final MCP request ID bytes")
    def encode_id():
        encoded = json.dumps(request_id).encode("utf-8")
        return encoded, encoded
    await service.encode_envelope(grant, encode_id, expanded)
    return request_id


class SchemaTool(Tool):
    _byte_limits: dict[str, int] = PrivateAttr()

    async def run(self, arguments: dict[str, Any]) -> ToolResult:
        from lctx_storage import StorageError

        ctx = get_context()
        request_context = ctx.request_context
        if request_context is None:
            raise ToolError("MCP request context unavailable")
        # Capture plain values before serialization runs in the original Rust CPU worker.
        protocol_version, server = request_context.protocol_version, ctx.fastmcp
        served = ctx.lifespan_context["served"]
        try:
            grant = await served.service.admit()
        except StorageError as exc:
            raise ToolError(str(exc)) from exc
        try:
            raw = await served.service.encode_request(
                grant, arguments,
                lambda args: json.dumps(args, ensure_ascii=False, separators=(",", ":"), allow_nan=False),
            )
            info = json.loads(await served.service.request_info(grant, self.name, raw))
            expanded = info["expanded"]
            request_id = await admit_request_id(served.service, grant, request_context, expanded, self._byte_limits)
            vector, degradation = await served.query_vector(grant, info["query"])
            response = await served.service.dispatch(
                grant, self.name, raw, query_vector=vector, degradation=degradation,
                numerical_callback=served.numerical if info["query"] is not None else None,
            )
            rendered = await served.service.tool_result(grant, self.name, response, expanded)
            wrapped = None
            def encode_result():
                nonlocal wrapped
                protocol_result = CallToolResult.model_validate_json(rendered)
                wrapped = ToolResult.from_mcp_result(protocol_result)
                return negotiated_encodings(protocol_result, request_id, protocol_version, server)
            await served.service.encode_envelope(grant, encode_result, expanded)
            assert wrapped is not None
            return wrapped
        except StorageError as exc:
            raise ToolError(str(exc)) from exc
        except ValueError as exc:
            if str(exc).startswith("resource_refused:"):
                raise ToolError(str(exc)) from exc
            raise ValidationError(str(exc)) from exc
        finally:
            # Native in-flight work retains its own handle until actual completion.
            grant.release()


class CapabilityResource(Resource):
    _capability: str = PrivateAttr()
    _byte_limits: dict[str, int] = PrivateAttr()

    async def read(self) -> ResourceResult:
        from lctx_storage import StorageError

        ctx = get_context()
        request_context = ctx.request_context
        if request_context is None:
            raise ResourceError("MCP request context unavailable")
        protocol_version, server = request_context.protocol_version, ctx.fastmcp
        served = ctx.lifespan_context["served"]
        try:
            grant = await served.service.admit()
        except StorageError as exc:
            raise ResourceError(str(exc)) from exc
        try:
            request_id = await admit_request_id(served.service, grant, request_context, True, self._byte_limits)
            uri, mime_type = self.uri, self.mime_type
            text = await served.service.capability_resource(grant, self._capability)
            result = None
            def encode_result():
                nonlocal result
                result = ResourceResult([ResourceContent(text, mime_type=mime_type)])
                return negotiated_encodings(result.to_mcp_result(uri), request_id, protocol_version, server)
            await served.service.encode_envelope(grant, encode_result, True)
            assert result is not None
            return result
        except StorageError as exc:
            raise ResourceError(str(exc)) from exc
        except ToolError as exc:
            raise ResourceError(str(exc)) from exc
        except ValueError as exc:
            raise ValidationError(str(exc)) from exc
        finally:
            grant.release()


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
            description=f"{name}: generation-bound API and original evidence result.",
            parameters=declaration["request_schema"], output_schema=declaration["response_schema"],
            annotations=ToolAnnotations(
                read_only_hint=declaration["read_only"], idempotent_hint=declaration["idempotent"], open_world_hint=False,
            ),
            meta={"lctx_wire_identity": contract["wire_identity"]},
        )
        tool._byte_limits = contract["byte_limits"]
        server.add_tool(tool)
    for declaration in json.loads(wire_resources()):
        template = CapabilityTemplate(
            uri_template=declaration["uri_template"], name=declaration["name"],
            mime_type=declaration["mime_type"],
            parameters={"type": "object", "properties": {"capability": {"type": "string"}}, "required": ["capability"]},
        )
        template._byte_limits = json.loads(wire_tool("get_capability"))["byte_limits"]
        server.add_template(template)
