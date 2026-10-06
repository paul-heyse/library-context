"""Typed native payloads traverse the actual FastMCP tool and resource routes safely."""
import json

import pytest
from fastmcp import Client, FastMCP
from lctx_semantics import NativeFailure, wire_decode, wire_failure
from mcp.shared.exceptions import MCPError

from lctx_mcp.wire import register, native_failure, response_encodings, failure_error, checked_resource_error

KINDS = ["resource_refused", "incompatible", "corrupt", "unavailable", "unknown_library"]

class FailingExecutor:
    def __init__(self, error):
        self.error = error

    async def execute(self, name, arguments):
        raise self.error


def typed_failure(kind):
    raw = wire_failure(kind)
    error = NativeFailure("internal diagnostic must not escape")
    error.lctx_failure_json = raw
    return error

@pytest.mark.anyio
@pytest.mark.parametrize("kind", KINDS)
async def test_actual_tool_and_resource_routes_keep_typed_fixed_failure(kind):
    server = FastMCP("safe error control")
    register(server, FailingExecutor(typed_failure(kind)))
    expected = json.loads(wire_failure(kind))
    async with Client(server) as client:
        result = await client.call_tool("browse_library", {"library": "fixture"}, raise_on_error=False)
        assert result.is_error is True
        assert result.meta["lctx_failure"] == expected
        assert result.content[0].text == expected["message"]
        with pytest.raises(MCPError) as caught:
            await client.read_resource("lctx://capability/" + "01" * 16)
        assert caught.value.error.data == {"lctx_failure": expected}
        assert caught.value.error.message == expected["message"]


def test_native_decoder_emits_checked_type_and_unexpected_text_is_never_classified():
    with pytest.raises(NativeFailure) as caught:
        wire_decode("browse_library", '{"library":"fixture","ignored":true}')
    assert native_failure(caught.value) == json.loads(wire_failure("incompatible"))
    for error in [RuntimeError("resource_refused password secret /private/path"), typed_failure("corrupt")]:
        if isinstance(error, NativeFailure):
            error.lctx_failure_json = '{"kind":"corrupt","message":"secret"}'
        assert native_failure(error) == json.loads(wire_failure("unavailable"))
    forged = MCPError(code=-32603, message="secret", data={"lctx_failure":{"kind":"corrupt","message":"secret"}})
    safe = checked_resource_error(forged)
    assert safe.error.message == json.loads(wire_failure("unavailable"))["message"]
    assert safe.error.data == {"lctx_failure":json.loads(wire_failure("unavailable"))}

@pytest.mark.parametrize("request_id", [71, "nested-71"])
def test_resource_error_envelope_retains_actual_id_and_safe_data(request_id):
    expected = json.loads(wire_failure("corrupt"))
    encoded, _ = response_encodings(failure_error(expected).error, request_id)
    value = json.loads(encoded)
    assert encoded.endswith(b"\n")
    assert value["id"] == request_id
    assert value["error"]["data"] == {"lctx_failure":expected}
