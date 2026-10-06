"""Native MCP journeys require a real fixture; ownership controls run in isolation."""

import asyncio
import json
import os
import sys
import threading
from pathlib import Path

import anyio
import pytest
from fastmcp import Client, FastMCP
from fastmcp.client.transports import StdioTransport
from fastmcp.exceptions import ToolError
from fastmcp.tools import ToolResult
from lctx_semantics import NativeSession
from mcp.shared.message import SessionMessage
from mcp_types import CallToolResult, JSONRPCResponse, TextContent, TextResourceContents

from lctx_mcp.__main__ import create_server
from lctx_mcp.native import NativeExecutor
from lctx_mcp.wire import BoundedStdioWriter, EnvelopeAdmission


@pytest.mark.anyio
@pytest.mark.parametrize("transport", ["inprocess", "stdio"])
async def test_native_mcp_lifespan_uses_one_pinned_viewer_snapshot(transport):
    """Exercise the actual Rust bridge, SDK, native server and MCP without live embedding."""
    configured = os.environ.get("LCTX_NATIVE_SERVING_CONFIG")
    assert configured, "LCTX_NATIVE_SERVING_CONFIG must name an owned published native fixture"
    path = Path(configured)
    config = json.loads(path.read_text())
    assert set(config) == {"endpoint", "username", "password", "snapshot"}
    library = os.environ.get("LCTX_NATIVE_TEST_LIBRARY")
    assert library, "LCTX_NATIVE_TEST_LIBRARY must name the published fixture's library"
    target = (
        create_server(path)
        if transport == "inprocess"
        else StdioTransport(
            command=sys.executable,
            args=["-m", "lctx_mcp", "--serving-config", str(path)],
            env=dict(os.environ),
            keep_alive=False,
        )
    )
    async with Client(target) as client:
        assert len(await client.list_tools()) == 10
        request = {"library": library, "page": {"size": 1}}
        first = await client.call_tool("browse_library", request)
        second = await client.call_tool("browse_library", request)
        assert first.structured_content is not None
        assert second.structured_content is not None
        assert first.structured_content["snapshot"] == config["snapshot"]
        assert second.structured_content == first.structured_content
        assert first.is_error is False
        assert len(first.content) == 1
        assert isinstance(first.content[0], TextContent)
        assert first.content[0].text == "browse_library: snapshot-bound result"
        assert first.structured_content["entries"]["items"]
        found = await client.call_tool(
            "search_capabilities",
            {
                "library": library,
                "query": "carefully",
                "page": {"size": 1, "expanded": True},
            },
        )
        assert found.structured_content is not None
        assert found.structured_content["channels"]["vector"]["status"] == "disabled"
        capability = found.structured_content["results"]["items"][0]["capability"]
        packet = await client.call_tool(
            "get_capability", {"capability": capability, "page": {"expanded": True}}
        )
        assert packet.structured_content is not None
        uri = "lctx://capability/" + bytes(capability).hex()
        resource = await client.read_resource(uri)
        assert len(resource) == 1
        assert isinstance(resource[0], TextResourceContents)
        assert resource[0].mime_type == "text/markdown"
        assert resource[0].text.startswith(packet.structured_content["capability"]["rendered"])
        assert "## Snapshot metadata" in resource[0].text
        assert "## Assertion evidence" in resource[0].text
        with pytest.raises(ToolError):
            await client.call_tool("browse_library", {**request, "ignored": True})
    # A fresh viewer can be opened after the MCP-owned session drained and invalidated.
    session = await asyncio.to_thread(NativeSession, str(path))
    assert json.loads(session.handle_json()) == config["snapshot"]
    await asyncio.to_thread(session.close)
    with pytest.raises(RuntimeError, match="closed"):
        await asyncio.to_thread(session.execute, "browse_library", json.dumps(request))


@pytest.mark.anyio
async def test_transport_admission_refuses_complete_oversized_results_and_errors():
    """An actual MCP exchange exercises the byte adapter, independently of graph providers."""
    server = FastMCP("envelope control", cache_ttl=None)
    server.add_middleware(EnvelopeAdmission(server))

    @server.tool
    def oversized():
        return ToolResult.from_mcp_result(
            CallToolResult(
                content=[TextContent(type="text", text="雪" * 12_000)],
                structured_content={"complete": "雪" * 12_000},
            )
        )

    @server.tool
    def oversized_error():
        raise ToolError("雪" * 12_000)

    @server.tool
    def unicode_fits_stdio():
        return ToolResult.from_mcp_result(
            CallToolResult(
                content=[TextContent(type="text", text="雪" * 6_000)],
                is_error=False,
            )
        )

    async with Client(server) as client:
        complete = await client.call_tool("unicode_fits_stdio", {})
        assert complete.is_error is False
        assert isinstance(complete.content[0], TextContent)
        assert complete.content[0].text == "雪" * 6_000
        for name in ("oversized", "oversized_error"):
            result = await client.call_tool(name, {}, raise_on_error=False)
            assert result.is_error is True
            assert result.structured_content is None
            assert isinstance(result.content[0], TextContent)
            assert result.content[0].text == "resource_refused: final MCP envelope bytes"


@pytest.mark.anyio
async def test_cancellation_retains_worker_admission_until_shutdown_drains():
    """A controlled thread checks Python ownership only; it does not stand in for native serving."""
    entered = threading.Event()
    released = threading.Event()
    closed = threading.Event()

    class OwnedThread:
        def execute(self, tool, request, vector, remaining_ms):
            assert 0 < remaining_ms <= 30_000
            entered.set()
            assert released.wait(5), "test did not release its owned thread"
            return "{}"

        def close(self):
            assert released.is_set()
            closed.set()

    executor = NativeExecutor(OwnedThread())
    request = asyncio.create_task(executor.execute("browse_library", {"library": "control"}))
    assert await asyncio.to_thread(entered.wait, 2)
    request.cancel()
    with pytest.raises(asyncio.CancelledError):
        await request
    assert len(executor._pending) == 1
    assert executor._slots._value == 1
    shutdown = asyncio.create_task(executor.close())
    await asyncio.sleep(0)
    assert not shutdown.done()
    assert not closed.is_set()
    released.set()
    await shutdown
    assert closed.is_set()
    assert executor._slots._value == 2
    with pytest.raises(RuntimeError, match="closed"):
        await executor.execute("browse_library", {"library": "control"})


@pytest.mark.anyio
async def test_stdio_writer_checks_final_bytes_and_keeps_numeric_ids_distinct():
    """Writer control measures the exact SDK model, including newline, before any send."""
    send, receive = anyio.create_memory_object_stream(2)
    admission = EnvelopeAdmission(FastMCP("writer control"))
    writer = BoundedStdioWriter(send, admission)
    admission._requests[(str, "7")] = True
    admission._requests[(int, 7)] = False
    result = {"content": [{"type": "text", "text": "x" * 40_000}], "isError": False}
    accepted = SessionMessage(message=JSONRPCResponse(jsonrpc="2.0", id="7", result=result))
    await writer.send(accepted)
    assert await receive.receive() is accepted
    assert (str, "7") not in admission._requests
    refused = SessionMessage(message=JSONRPCResponse(jsonrpc="2.0", id=7, result=result))
    with pytest.raises(anyio.BrokenResourceError):
        await writer.send(refused)
    assert writer.closed
    with pytest.raises(anyio.EndOfStream):
        await receive.receive()
    with pytest.raises(anyio.BrokenResourceError):
        await writer.send(accepted)
    await receive.aclose()


@pytest.mark.anyio
async def test_stdio_writer_refuses_a_huge_id_without_emitting_its_error():
    send, receive = anyio.create_memory_object_stream(1)
    admission = EnvelopeAdmission(FastMCP("ID refusal control"))
    writer = BoundedStdioWriter(send, admission)
    request_id = "x" * 40_000
    admission._requests[(str, request_id)] = False
    packet = SessionMessage(message=JSONRPCResponse(jsonrpc="2.0", id=request_id, result={}))
    with pytest.raises(anyio.BrokenResourceError):
        await writer.send(packet)
    with pytest.raises(anyio.EndOfStream):
        await receive.receive()
    await receive.aclose()
