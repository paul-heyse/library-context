"""Typed native payloads traverse the actual FastMCP tool and resource routes safely."""

import json
import os
import sys
from pathlib import Path

import pytest
from fastmcp import Client
from fastmcp.client.transports import StdioTransport
from lctx_semantics import NativeFailure, wire_decode, wire_failure
from mcp.shared.exceptions import MCPError
from mcp_types import TextContent

from lctx_mcp.wire import checked_resource_error, failure_error, native_failure, response_encodings

KINDS = ["resource_refused", "incompatible", "corrupt", "unavailable", "unknown_library"]


def typed_failure(kind):
    raw = wire_failure(kind)
    error = NativeFailure("internal diagnostic must not escape")
    error.lctx_failure_json = raw
    return error


@pytest.mark.anyio
@pytest.mark.parametrize("kind", [*KINDS, "unexpected"])
@pytest.mark.parametrize("transport", ["inprocess", "stdio"])
async def test_actual_tool_and_resource_routes_keep_typed_fixed_failure(kind, transport):
    """Injected executor faults exercise both real MCP transports and final envelopes."""
    from fixtures_safe_failure import server

    target = (
        server(kind)
        if transport == "inprocess"
        else StdioTransport(
            command=sys.executable,
            args=[str(Path(__file__).with_name("fixtures_safe_failure.py")), kind],
            env=dict(os.environ),
            keep_alive=False,
        )
    )
    expected = json.loads(wire_failure("unavailable" if kind == "unexpected" else kind))
    async with Client(target) as client:
        result = await client.call_tool(
            "browse_library", {"library": "fixture"}, raise_on_error=False
        )
        assert result.is_error is True
        assert result.meta is not None
        assert result.meta["lctx_failure"] == expected
        assert isinstance(result.content[0], TextContent)
        assert result.content[0].text == expected["message"]
        assert "sentinel" not in str(result) and "/private/" not in str(result)
        with pytest.raises(MCPError) as caught:
            await client.read_resource("lctx://capability/" + "01" * 16)
        assert caught.value.error.data == {"lctx_failure": expected}
        assert caught.value.error.message == expected["message"]
        assert "sentinel" not in str(caught.value.error) and "/private/" not in str(
            caught.value.error
        )


def test_native_decoder_emits_checked_type_and_unexpected_text_is_never_classified():
    with pytest.raises(NativeFailure) as caught:
        wire_decode("browse_library", '{"library":"fixture","ignored":true}')
    assert native_failure(caught.value) == json.loads(wire_failure("incompatible"))
    for error in [
        RuntimeError("resource_refused password secret /private/path"),
        typed_failure("corrupt"),
    ]:
        if isinstance(error, NativeFailure):
            error.lctx_failure_json = '{"kind":"corrupt","message":"secret"}'
        assert native_failure(error) == json.loads(wire_failure("unavailable"))
    forged = MCPError(
        code=-32603,
        message="secret",
        data={"lctx_failure": {"kind": "corrupt", "message": "secret"}},
    )
    safe = checked_resource_error(forged)
    assert safe.error.message == json.loads(wire_failure("unavailable"))["message"]
    assert safe.error.data == {"lctx_failure": json.loads(wire_failure("unavailable"))}


@pytest.mark.parametrize("request_id", [71, "nested-71"])
def test_resource_error_envelope_retains_actual_id_and_safe_data(request_id):
    expected = json.loads(wire_failure("corrupt"))
    encoded, _ = response_encodings(failure_error(expected).error, request_id)
    value = json.loads(encoded)
    assert encoded.endswith(b"\n")
    assert value["id"] == request_id
    assert value["error"]["data"] == {"lctx_failure": expected}
