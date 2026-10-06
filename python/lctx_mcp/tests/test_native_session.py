"""Native MCP journey and isolated ownership controls; the native journey requires a real fixture."""

import asyncio
import json
import os
import threading
from pathlib import Path

import pytest
from fastmcp import Client, FastMCP
from fastmcp.exceptions import ToolError
from lctx_semantics import NativeSession
from mcp_types import CallToolResult, TextContent
from fastmcp.tools import ToolResult

from lctx_mcp.__main__ import create_server
from lctx_mcp.native import NativeExecutor
from lctx_mcp.wire import EnvelopeAdmission


@pytest.mark.anyio
async def test_native_mcp_lifespan_uses_one_pinned_viewer_snapshot():
    """Actual Rust bridge, SDK, native server and MCP; no fake provider or live embedding service."""
    configured = os.environ.get("LCTX_NATIVE_SERVING_CONFIG")
    assert configured, "LCTX_NATIVE_SERVING_CONFIG must name an owned published native fixture"
    path = Path(configured)
    config = json.loads(path.read_text())
    assert set(config) == {"endpoint", "username", "password", "snapshot"}
    library = os.environ.get("LCTX_NATIVE_TEST_LIBRARY")
    assert library, "LCTX_NATIVE_TEST_LIBRARY must name the published fixture's library"
    server = create_server(path)
    async with Client(server) as client:
        assert len(await client.list_tools()) == 10
        request = {"library": library, "page": {"size": 1}}
        first = await client.call_tool("list_domains", request)
        second = await client.call_tool("list_domains", request)
        assert first.structured_content["snapshot"] == config["snapshot"]
        assert second.structured_content == first.structured_content
        assert first.is_error is False
        assert len(first.content) == 1
        assert first.content[0].text == "list_domains: snapshot-bound result"
        with pytest.raises(ToolError):
            await client.call_tool("list_domains", {**request, "ignored": True})
    # A fresh viewer can be opened after the MCP-owned session drained and invalidated.
    session = await asyncio.to_thread(NativeSession, str(path))
    assert json.loads(session.handle_json()) == config["snapshot"]
    await asyncio.to_thread(session.close)
    with pytest.raises(RuntimeError, match="closed"):
        await asyncio.to_thread(session.execute, "list_domains", json.dumps(request))


@pytest.mark.anyio
async def test_transport_admission_refuses_complete_oversized_results_and_errors():
    """An actual MCP exchange exercises the byte adapter, independently of graph providers."""
    server = FastMCP("envelope control", cache_ttl=0)
    server.add_middleware(EnvelopeAdmission(server))

    @server.tool
    def oversized():
        return ToolResult.from_mcp_result(CallToolResult(
            content=[TextContent(type="text", text="雪" * 12_000)],
            structured_content={"complete": "雪" * 12_000},
        ))

    @server.tool
    def oversized_error():
        raise ToolError("雪" * 12_000)

    async with Client(server) as client:
        for name in ("oversized", "oversized_error"):
            result = await client.call_tool(name, {}, raise_on_error=False)
            assert result.is_error is True
            assert result.structured_content is None
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
    request = asyncio.create_task(executor.execute("list_domains", {"library": "control"}))
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
        await executor.execute("list_domains", {"library": "control"})
