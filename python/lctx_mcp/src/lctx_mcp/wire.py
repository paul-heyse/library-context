"""Rust-owned schemas and complete MCP serialization over a pinned native operation session."""

from __future__ import annotations

import json
from types import TracebackType
from typing import Any

import anyio
from fastmcp.exceptions import (
    DisabledError,
    FastMCPError,
    NotFoundError,
)
from fastmcp.resources import Resource, ResourceContent, ResourceResult, ResourceTemplate
from fastmcp.server.middleware import Middleware
from fastmcp.tools import Tool, ToolResult
from mcp.shared.exceptions import MCPError
from mcp_types import (
    INTERNAL_ERROR,
    SERVER_INFO_META_KEY,
    CallToolResult,
    ErrorData,
    Implementation,
    JSONRPCError,
    JSONRPCResponse,
    ReadResourceResult,
    TextContent,
    ToolAnnotations,
)
from mcp_types.methods import serialize_server_result
from mcp_types.version import HANDSHAKE_PROTOCOL_VERSIONS, MODERN_PROTOCOL_VERSIONS
from pydantic import PrivateAttr


def _failure(kind: str) -> dict:
    from lctx_semantics import wire_failure

    return json.loads(wire_failure(kind))


def native_failure(exc: Exception) -> dict:
    from lctx_semantics import NativeFailure

    if isinstance(exc, NativeFailure):
        try:
            value = json.loads(exc.lctx_failure_json)
            expected = _failure(value["kind"])
            if value == expected:
                return expected
        except AttributeError, KeyError, TypeError, ValueError:
            pass
    return _failure("unavailable")


def failure_result(failure: dict) -> CallToolResult:
    return CallToolResult(
        content=[TextContent(type="text", text=failure["message"])],
        is_error=True,
        meta={"lctx_failure": failure},
    )


def failure_error(failure: dict) -> MCPError:
    return MCPError(code=INTERNAL_ERROR, message=failure["message"], data={"lctx_failure": failure})


def checked_resource_error(exc: MCPError) -> MCPError:
    """Only exact model-owned failure data may cross the resource error boundary."""
    try:
        failure = exc.error.data["lctx_failure"]
        expected = _failure(failure["kind"])
        if exc.error.data == {"lctx_failure": expected} and failure == expected:
            return failure_error(expected)
    except AttributeError, KeyError, TypeError, ValueError:
        pass
    return failure_error(_failure("unavailable"))


def _response_envelope(
    result: CallToolResult | ReadResourceResult | ErrorData,
    request_id: int | str,
    *,
    protocol_version: str | None = None,
    server_info: dict | None = None,
) -> JSONRPCError | JSONRPCResponse:
    """Shape the pinned SDK envelope with the actual ID and negotiated fields."""
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
    return (
        JSONRPCError(jsonrpc="2.0", id=request_id, error=result)
        if isinstance(result, ErrorData)
        else JSONRPCResponse(jsonrpc="2.0", id=request_id, result=shaped)
    )


def response_encodings(
    result: CallToolResult | ReadResourceResult | ErrorData,
    request_id: int | str,
    *,
    protocol_version: str | None = None,
    server_info: dict | None = None,
) -> tuple[bytes, bytes]:
    """SDK writer contract controls; product serving admits its actual stdio form."""
    envelope = _response_envelope(
        result,
        request_id,
        protocol_version=protocol_version,
        server_info=server_info,
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


class EnvelopeAdmission(Middleware):
    """Admit the pinned stdio envelope after domain results are complete, without truncation."""

    def __init__(self, server) -> None:
        self.server_info = Implementation(
            name=server.name,
            version=server.version,
            website_url=server.website_url,
            icons=server.icons or None,
        ).model_dump(by_alias=True, mode="json", exclude_none=True)
        self._requests: dict[tuple[type, int | str], bool] = {}

    def _request_id(self, context):
        current = context.fastmcp_context
        request = current.request_context if current is not None else None
        return request._srctx.request_id if request is not None else None

    async def on_request(self, context, call_next):
        if context.method in {"tools/call", "resources/read"}:
            request_id = self._request_id(context)
            if request_id is not None:
                self._requests[(type(request_id), request_id)] = context.method == "resources/read"
        return await call_next(context)

    def _admit(self, context, result, expanded: bool) -> None:
        from lctx_semantics import admit_envelope

        current = context.fastmcp_context
        request = current.request_context if current is not None else None
        if request is None:
            # Direct server calls have no JSON-RPC envelope. Domain admission still applies.
            return
        # The pinned FastMCP wrapper normalizes request_id to str; its documented SDK
        # escape hatch retains the wire's integer/string distinction.
        request_id = request._srctx.request_id
        if request_id is None:
            return
        self._requests[(type(request_id), request_id)] = expanded
        envelope = _response_envelope(
            result,
            request_id,
            protocol_version=request.protocol_version,
            server_info=self.server_info,
        )
        encoded = envelope.model_dump_json(by_alias=True, exclude_unset=True) + "\n"
        admit_envelope(encoded, expanded)

    @staticmethod
    def _refusal() -> CallToolResult:
        return failure_result(_failure("resource_refused"))

    async def on_call_tool(self, context, call_next):
        page = (context.message.arguments or {}).get("page")
        expanded = isinstance(page, dict) and page.get("expanded") is True
        # Refuse an ID that cannot fit even a minimal result before native work starts.
        self._admit(context, self._refusal(), expanded)
        try:
            result = await call_next(context)
        except (FastMCPError, NotFoundError, DisabledError) as exc:
            result = ToolResult.from_mcp_result(failure_result(native_failure(exc)))
        try:
            self._admit(context, result.to_mcp_result(), expanded)
        except ValueError, RuntimeError:
            refusal = self._refusal()
            self._admit(context, refusal, expanded)
            return ToolResult.from_mcp_result(refusal)
        return result

    async def on_read_resource(self, context, call_next):
        expanded = True
        self._admit(context, failure_error(_failure("resource_refused")).error, expanded)
        try:
            result = await call_next(context)
        except (FastMCPError, NotFoundError, DisabledError, MCPError) as exc:
            error = (
                checked_resource_error(exc)
                if isinstance(exc, MCPError)
                else failure_error(_failure("unavailable"))
            )
            try:
                self._admit(context, error.error, expanded)
            except ValueError, RuntimeError:
                error = failure_error(_failure("resource_refused"))
                self._admit(context, error.error, expanded)
            raise error from exc
        try:
            self._admit(context, result.to_mcp_result(str(context.message.uri)), expanded)
        except (ValueError, RuntimeError) as exc:
            error = failure_error(_failure("resource_refused"))
            self._admit(context, error.error, expanded)
            raise error from exc
        return result


class BoundedStdioWriter:
    """SDK stream adapter; refuse a complete oversized packet before it reaches stdout."""

    def __init__(self, stream, admission: EnvelopeAdmission) -> None:
        self.stream = stream
        self.admission = admission
        self.closed = False

    async def send(self, message) -> None:
        from lctx_semantics import admit_envelope

        if self.closed:
            raise anyio.BrokenResourceError
        packet = message.message
        request_id = getattr(packet, "id", None)
        key = (type(request_id), request_id) if isinstance(request_id, (int, str)) else None
        expanded = self.admission._requests.get(key) if key is not None else None
        if expanded is not None:
            encoded = packet.model_dump_json(by_alias=True, exclude_unset=True) + "\n"
            try:
                admit_envelope(encoded, expanded)
            except (ValueError, RuntimeError) as exc:
                # A huge ID cannot be answered within its cap: close the transport.
                # Lifecycle teardown still drains every owned native call.
                await self.aclose()
                raise anyio.BrokenResourceError from exc
        await self.stream.send(message)
        if key is not None:
            self.admission._requests.pop(key, None)

    async def __aenter__(self):
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> bool | None:
        await self.aclose()

    async def aclose(self) -> None:
        self.closed = True
        self.admission._requests.clear()
        await self.stream.aclose()


class SchemaTool(Tool):
    _executor: Any = PrivateAttr()
    _byte_limits: dict[str, int] = PrivateAttr()

    async def run(self, arguments: dict[str, Any]) -> ToolResult:
        from lctx_semantics import wire_tool_result

        page = arguments.get("page")
        expanded = isinstance(page, dict) and page.get("expanded") is True
        try:
            raw = await self._executor.execute(self.name, arguments)
            result = CallToolResult.model_validate_json(wire_tool_result(self.name, raw, expanded))
            return ToolResult.from_mcp_result(result)
        except Exception as exc:
            return ToolResult.from_mcp_result(failure_result(native_failure(exc)))


class CapabilityResource(Resource):
    _executor: Any = PrivateAttr()
    _capability: str = PrivateAttr()
    _byte_limits: dict[str, int] = PrivateAttr()

    async def read(self) -> ResourceResult:
        from lctx_semantics import wire_capability_resource

        try:
            if len(self._capability) != 32:
                raise failure_error(_failure("incompatible"))
            try:
                capability = list(bytes.fromhex(self._capability))
            except ValueError as exc:
                raise failure_error(_failure("incompatible")) from exc
            raw = await self._executor.execute(
                "get_capability", {"capability": capability, "page": {"expanded": True}}
            )
            return ResourceResult(
                [ResourceContent(wire_capability_resource(raw), mime_type=self.mime_type)]
            )
        except MCPError:
            raise
        except Exception as exc:
            raise failure_error(native_failure(exc)) from exc


class CapabilityTemplate(ResourceTemplate):
    _executor: Any = PrivateAttr()
    _byte_limits: dict[str, int] = PrivateAttr()

    async def create_resource(self, uri: str, params: dict[str, Any]) -> Resource:
        resource = CapabilityResource(uri=uri, name=self.name, mime_type=self.mime_type)
        resource._executor = self._executor
        resource._capability = params["capability"]
        resource._byte_limits = self._byte_limits
        return resource


def register(server, executor) -> None:
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
        tool._executor = executor
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
        template._executor = executor
        template._byte_limits = json.loads(wire_tool("get_capability"))["byte_limits"]
        server.add_template(template)
