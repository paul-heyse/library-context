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
from fastmcp.client.client import CallToolResult as ClientCallToolResult
from fastmcp.client.transports import StdioTransport
from fastmcp.exceptions import ToolError
from fastmcp.tools import ToolResult
from lctx_semantics import NativeFailure, NativeSession, wire_failure
from mcp.shared.exceptions import MCPError
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
    assert set(config) == {"endpoint", "username", "password", "selection"}
    selected = json.loads(Path(config["selection"]).read_text())
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
        assert first.structured_content["snapshot"] == selected
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
        for arguments, kind in [
            ({**request, "ignored": True}, "incompatible"),
            ({"library": "missing-private-name"}, "unknown_library"),
        ]:
            failure = await client.call_tool("browse_library", arguments, raise_on_error=False)
            expected = json.loads(wire_failure(kind))
            assert failure.is_error is True
            assert failure.meta is not None
            assert failure.meta["lctx_failure"] == expected
            assert isinstance(failure.content[0], TextContent)
            assert failure.content[0].text == expected["message"]
        with pytest.raises(MCPError) as missing:
            await client.read_resource("lctx://capability/" + "00" * 16)
        expected = json.loads(wire_failure("corrupt"))
        assert missing.value.error.data == {"lctx_failure": expected}
        assert missing.value.error.message == expected["message"]
    # A fresh viewer can be opened after the MCP-owned session drained and invalidated.
    session = await asyncio.to_thread(NativeSession, str(path))
    assert json.loads(session.handle_json()) == selected
    await asyncio.to_thread(session.close)
    with pytest.raises(NativeFailure) as closed:
        await asyncio.to_thread(session.execute, "browse_library", json.dumps(request))
    assert json.loads(closed.value.lctx_failure_json) == json.loads(wire_failure("unavailable"))


@pytest.mark.anyio
async def test_actual_native_zero_deadline_returns_safe_resource_refusal():
    configured = os.environ.get("LCTX_NATIVE_SERVING_CONFIG")
    library = os.environ.get("LCTX_NATIVE_TEST_LIBRARY")
    assert configured and library, "owned published native fixture required"
    native = await asyncio.to_thread(NativeSession, configured)
    try:
        with pytest.raises(NativeFailure) as refused:
            await asyncio.to_thread(
                native.execute, "browse_library", json.dumps({"library": library}), None, 0
            )
        assert json.loads(refused.value.lctx_failure_json) == json.loads(
            wire_failure("resource_refused")
        )
        # Refusal releases admission; the same actual pinned session remains usable.
        result = await asyncio.to_thread(
            native.execute, "browse_library", json.dumps({"library": library})
        )
        assert json.loads(result)["entries"]["items"]
    finally:
        await asyncio.to_thread(native.close)


@pytest.mark.anyio
async def test_cancelled_worker_drains_actual_native_dispatch_before_session_close(tmp_path):
    """Delay an actual gRPC response after dispatch; cancellation still owns its worker."""
    from native_transport import ResponseGate, viewer_config

    configured = os.environ.get("LCTX_NATIVE_SERVING_CONFIG")
    library = os.environ.get("LCTX_NATIVE_TEST_LIBRARY")
    assert configured and library, "owned published native fixture required"
    endpoint = json.loads(Path(configured).read_text())["endpoint"]
    async with ResponseGate(endpoint) as proxy:
        path = viewer_config(configured, proxy.endpoint, tmp_path / "viewer.json")
        native = await asyncio.to_thread(NativeSession, str(path))
        executor = NativeExecutor(native)
        try:
            proxy.pause()
            request = asyncio.create_task(executor.execute("browse_library", {"library": library}))
            # Receipt of real upstream DATA is later than NativeSession.execute and SDK
            # dispatch. This is an injected delayed read, not engine preemption evidence.
            await asyncio.wait_for(proxy.entered.wait(), timeout=5)
            assert not request.done()
            request.cancel()
            with pytest.raises(asyncio.CancelledError):
                await request
            assert len(executor._pending) == 1 and executor._slots._value == 1
            worker = next(iter(executor._pending))
            proxy.resume()
            result = await asyncio.wait_for(asyncio.shield(worker), timeout=10)
            assert json.loads(result)["entries"]["items"]
            await asyncio.sleep(0)
            assert not executor._pending and executor._slots._value == 2
            # The same actual native session remains usable after its abandoned read drains.
            result = await executor.execute("browse_library", {"library": library})
            assert json.loads(result)["entries"]["items"]
            await executor.close()
            with pytest.raises(NativeFailure) as closed:
                await asyncio.to_thread(
                    native.execute, "browse_library", json.dumps({"library": library})
                )
            assert json.loads(closed.value.lctx_failure_json) == json.loads(
                wire_failure("unavailable")
            )
        finally:
            proxy.resume()
            await executor.close()


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
        for name, kind in [("oversized", "resource_refused"), ("oversized_error", "unavailable")]:
            result = await client.call_tool(name, {}, raise_on_error=False)
            assert result.is_error is True
            assert result.structured_content is None
            assert isinstance(result.content[0], TextContent)
            expected = json.loads(wire_failure(kind))
            assert result.content[0].text == expected["message"]
            assert result.meta is not None
            assert result.meta["lctx_failure"] == expected


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
    with pytest.raises(RuntimeError, match="Canonical serving is unavailable"):
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


@pytest.mark.anyio
@pytest.mark.parametrize("transport", ["inprocess", "stdio"])
async def test_actual_native_corruption_and_delayed_read_failures_are_safe_on_both_transports(
    transport, tmp_path
):
    """Real persisted corruption and injected read loss traverse Rust, PyO3 and MCP."""
    from native_transport import ResponseGate, fixture_query, viewer_config

    configured = os.environ.get("LCTX_NATIVE_SERVING_CONFIG")
    library = os.environ.get("LCTX_NATIVE_TEST_LIBRARY")
    assert configured and library, "owned published native fixture required"
    config = json.loads(Path(configured).read_text())
    selected = json.loads(Path(config["selection"]).read_text())
    async with ResponseGate(config["endpoint"]) as proxy:
        path = viewer_config(configured, proxy.endpoint, tmp_path / "viewer.json")
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
            found = await client.call_tool(
                "search_capabilities", {"library": library, "query": "carefully"}
            )
            assert found.structured_content is not None
            capability = found.structured_content["results"]["items"][0]["capability"]
            uri = "lctx://capability/" + bytes(capability).hex()
            key = bytes(capability).hex()
            # Capture original bytes through the explicit fixture admin, then alter only
            # canonical bytes. Public request decoding and current definitions still pass.
            saved = await asyncio.to_thread(
                fixture_query,
                selected,
                "SELECT id,encoding::base64::encode(canonical) AS saved FROM entity "
                f"WHERE semantic_type='synthesis_briefs' AND semantic_key='{key}';",
            )
            assert len(saved[0]["result"]) == 1
            record = saved[0]["result"][0]
            row_id = record["id"]
            assert row_id.startswith("entity:") and all(
                char.isalnum() or char in ":_" for char in row_id
            )
            try:
                await asyncio.to_thread(
                    fixture_query, selected, f'UPDATE {row_id} SET canonical=b"00";'
                )
                corrupt = json.loads(wire_failure("corrupt"))
                result = await client.call_tool(
                    "get_capability", {"capability": capability}, raise_on_error=False
                )
                assert result.is_error
                assert result.meta is not None
                assert result.meta["lctx_failure"] == corrupt
                assert isinstance(result.content[0], TextContent)
                assert result.content[0].text == corrupt["message"]
                with pytest.raises(MCPError) as caught:
                    await client.read_resource(uri)
                assert caught.value.error.message == corrupt["message"]
                assert caught.value.error.data == {"lctx_failure": corrupt}
            finally:
                await asyncio.to_thread(
                    fixture_query,
                    selected,
                    f"UPDATE {row_id} SET canonical=encoding::base64::decode('{record['saved']}');",
                )
            healthy = await client.call_tool("get_capability", {"capability": capability})
            assert not healthy.is_error

            # A real continuation from a different operation request must retain the exact
            # incompatible cause through the actual transport, including persisted pin.
            first = await client.call_tool(
                "browse_library", {"library": library, "page": {"size": 1}}
            )
            assert first.structured_content is not None
            cursor = first.structured_content["entries"]["continuation"]
            assert cursor
            wrong = await client.call_tool(
                "browse_library",
                {"library": library, "view": "vocabulary", "page": {"size": 1, "cursor": cursor}},
                raise_on_error=False,
            )
            incompatible = json.loads(wire_failure("incompatible"))
            assert wrong.is_error
            assert wrong.meta is not None
            assert wrong.meta["lctx_failure"] == incompatible
            assert isinstance(wrong.content[0], TextContent)
            assert wrong.content[0].text == incompatible["message"]

            for resource in [False, True]:
                proxy.pause()
                pending = asyncio.create_task(
                    client.read_resource(uri)
                    if resource
                    else client.call_tool(
                        "get_capability", {"capability": capability}, raise_on_error=False
                    )
                )
                try:
                    await asyncio.wait_for(proxy.entered.wait(), timeout=5)
                    assert not pending.done(), (
                        "actual native response must remain unread at injection"
                    )
                    proxy.disconnect()
                    unavailable = json.loads(wire_failure("unavailable"))
                    if resource:
                        with pytest.raises(MCPError) as caught:
                            await asyncio.wait_for(pending, timeout=10)
                        assert caught.value.error.message == unavailable["message"]
                        assert caught.value.error.data == {"lctx_failure": unavailable}
                    else:
                        result = await asyncio.wait_for(pending, timeout=10)
                        assert isinstance(result, ClientCallToolResult)
                        assert result.is_error
                        assert result.meta is not None
                        assert result.meta["lctx_failure"] == unavailable
                        assert isinstance(result.content[0], TextContent)
                        assert result.content[0].text == unavailable["message"]
                finally:
                    proxy.resume()
                # Reconnection/drain must permit a subsequent actual request on this pin.
                result = await client.call_tool("get_capability", {"capability": capability})
                assert not result.is_error
