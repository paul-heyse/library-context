"""Thin views and FastMCP transport for the Rust-owned wire contracts.

No field definitions, coercion rules or domain schemas live here. Packet attribute access is
only a rendering convenience over the JSON returned by the native decoder.
"""

from __future__ import annotations

import asyncio
import json
from typing import Any

from fastmcp.exceptions import ToolError, ValidationError
from fastmcp.server.dependencies import get_context
from fastmcp.tools import Tool, ToolResult
from lctx_semantics import wire_decode, wire_schema, wire_tool, wire_tool_result
from mcp_types import CallToolResult
from pydantic import PrivateAttr

REQUEST_SECONDS = 30.0


def plain(value):
    if isinstance(value, Packet):
        return {key: plain(item) for key, item in value._data.items()}
    if isinstance(value, dict):
        return {key: plain(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [plain(item) for item in value]
    return value


def view(value):
    if isinstance(value, dict):
        return Packet(value)
    if isinstance(value, list):
        return [view(item) for item in value]
    return value


class Packet:
    """Read-only presentation access; values have already passed the Rust contract."""

    def __init__(self, value: dict):
        self._data = value

    def __getattr__(self, name: str) -> Any:
        try:
            return view(self._data[name])
        except KeyError as exc:
            raise AttributeError(name) from exc

    def __getitem__(self, name: str) -> Any:
        return view(self._data[name])

    def __iter__(self):
        return iter(self._data)

    def __eq__(self, other):
        return plain(self) == plain(other)

    def __len__(self):
        return len(self._data)

    def items(self):
        return ((key, view(value)) for key, value in self._data.items())

    def model_dump(self, *, mode=None):
        return plain(self)

    def model_dump_json(self):
        return json.dumps(self._data, ensure_ascii=False, separators=(",", ":"))


class Contract:
    """Names an exported native contract, without reproducing its definition."""

    def __init__(self, name: str):
        self.name = name

    def __call__(self, **values) -> Packet:
        return self.model_validate(values)

    def model_validate(self, value) -> Packet:
        return self.model_validate_json(json.dumps(plain(value), ensure_ascii=False))

    def model_validate_json(self, raw: str) -> Packet:
        try:
            return Packet(json.loads(wire_decode(self.name, raw)))
        except ValueError as exc:
            if str(exc).startswith("resource_refused:"):
                raise ToolError(str(exc)) from exc
            raise

    def model_json_schema(self):
        return json.loads(wire_schema(self.name, True))


class SchemaTool(Tool):
    _callback: Any = PrivateAttr()
    _request: str = PrivateAttr()
    _byte_limits: dict[str, int] = PrivateAttr()

    async def run(self, arguments: dict[str, Any]) -> ToolResult:
        ctx = get_context()
        served = ctx.lifespan_context["served"]

        def decode():
            try:
                decoded = json.loads(wire_decode(self._request, json.dumps(arguments)))
            except ValueError as exc:
                raise ValidationError(str(exc)) from exc
            for key in ("exact_input", "selection"):
                if isinstance(decoded.get(key), dict):
                    decoded[key] = Packet(decoded[key])
            return decoded

        try:
            async with asyncio.timeout(REQUEST_SECONDS):
                decoded = await served.workers.run(decode)
                result = await self._callback(**decoded, ctx=ctx)

                def finish():
                    payload = (
                        result.structured_content
                        if isinstance(result, ToolResult)
                        else plain(result)
                    )
                    try:
                        raw = wire_tool_result(
                            self.name,
                            json.dumps(payload, ensure_ascii=False),
                            decoded.get("expanded", False),
                        )
                    except ValueError as exc:
                        if str(exc).startswith("resource_refused:"):
                            raise ToolError(str(exc)) from exc
                        raise
                    tool_result = ToolResult(
                        content=(
                            f"{self.name}: structured result; "
                            "inspect structuredContent for the complete contract and evidence."
                        ),
                        structured_content=json.loads(raw),
                    )
                    # Measure the SDK's actual MCP result, including content/metadata/escaping.
                    encoded = (
                        CallToolResult(
                            content=tool_result.content,
                            structured_content=tool_result.structured_content,
                            is_error=False,
                        )
                        .model_dump_json(by_alias=True, exclude_none=True)
                        .encode("utf-8")
                    )
                    limit = self._byte_limits[
                        "expanded" if decoded.get("expanded", False) else "default"
                    ]
                    if len(encoded) > limit:
                        raise ToolError(
                            "resource_refused: final MCP result byte budget; "
                            "request expanded=true or a smaller page"
                        )
                    return tool_result

                return await served.workers.run(finish)
        except TimeoutError as exc:
            raise ToolError("resource_refused: request deadline") from exc


def register(mcp, annotations):
    """Register callbacks without FunctionTool/Pydantic reinterpreting the domain contract."""

    def add(callback):
        contract = json.loads(wire_tool(callback.__name__))
        tool = SchemaTool(
            name=callback.__name__,
            description=callback.__doc__,
            parameters=contract["parameters"],
            output_schema=contract["output_schema"],
            annotations=annotations,
            meta={"lctx_wire_format": contract["format"]},
        )
        tool._callback = callback
        tool._request = contract["request"]
        tool._byte_limits = contract["byte_limits"]
        mcp.add_tool(tool)
        return callback

    return add
