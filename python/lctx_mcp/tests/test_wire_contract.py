"""Actual native decoder and FastMCP transport controls; no storage provider involved."""

import json
from contextlib import asynccontextmanager
from pathlib import Path
from types import SimpleNamespace

import pytest
from fastmcp import Client, FastMCP
from fastmcp.exceptions import ToolError
from lctx_semantics import wire_decode, wire_tool
from mcp_types import TextContent, ToolAnnotations

from lctx_mcp.server import NativeWorkers
from lctx_mcp.wire import register


def test_nested_contracts_preserve_required_nulls_and_closed_vocabulary():
    evidence = {
        "evidence_id": "11" * 16,
        "kind": "source",
        "path": None,
        "start_byte": None,
        "end_byte": None,
        "text": None,
    }
    assert json.loads(wire_decode("Evidence", json.dumps(evidence))) == evidence
    del evidence["text"]
    with pytest.raises(ValueError):
        wire_decode("Evidence", json.dumps(evidence))
    assert json.loads(wire_decode("ExactPrimitive", '{"kind":"none","value":null}')) == {
        "kind": "none",
        "value": None,
    }
    for invalid in ['{"kind":"int","value":true}', '{"kind":"bool","value":1}', '{"unexpected":1}']:
        with pytest.raises(ValueError):
            wire_decode("ExactPrimitive", invalid)


@pytest.mark.anyio
async def test_real_mcp_listing_and_call_use_native_contracts():
    @asynccontextmanager
    async def lifespan(_):
        workers = NativeWorkers()
        try:
            yield {"served": SimpleNamespace(workers=workers)}
        finally:
            await workers.close()

    mcp = FastMCP("contract control", lifespan=lifespan, dereference_schemas=False)
    annotations = ToolAnnotations(read_only_hint=True)
    received = []

    @register(mcp, annotations)
    async def get_operation(snapshot_id, operation, expanded, view, ctx):
        """Contract transport control."""
        received.append((snapshot_id, operation, expanded, view))
        return {
            "snapshot_id": snapshot_id,
            "generation": "11" * 32,
            "resolution": "ambiguous",
            "requested": operation,
            "choices": [],
        }

    async with Client(mcp) as client:
        listed = (await client.list_tools())[0]
        expected = json.loads(wire_tool("get_operation"))
        assert listed.input_schema == expected["parameters"]
        assert listed.output_schema == expected["output_schema"]
        result = await client.call_tool(
            "get_operation", {"snapshot_id": "AB" * 16, "operation": "pkg.member"}
        )
        assert received == [("ab" * 16, "pkg.member", False, {"kind": "packet"})]
        assert result.structured_content is not None
        assert result.structured_content["result_kind"] == "ambiguous"
        for extra in [{"unexpected": True}, {"expanded": 1}]:
            with pytest.raises(ToolError):
                await client.call_tool(
                    "get_operation", {"snapshot_id": "ab" * 16, "operation": "pkg.member", **extra}
                )


def test_shared_native_request_fixtures():
    path = Path(__file__).resolve().parents[3] / "specs/wire/requests.json"
    for case in json.loads(path.read_text()):
        if case["valid"]:
            assert json.loads(wire_decode(case["contract"], json.dumps(case["input"])))
        else:
            with pytest.raises(ValueError):
                wire_decode(case["contract"], json.dumps(case["input"]))


@pytest.mark.anyio
async def test_native_wire_work_retains_cancelled_lease_and_event_loop_progress():
    import asyncio
    import threading

    workers = NativeWorkers()
    started = threading.Event()
    finished = threading.Event()
    raw = json.dumps(
        {
            "evidence_id": "11" * 16,
            "kind": "source",
            "path": None,
            "start_byte": None,
            "end_byte": None,
            "text": "x" * 2_000_000,
        }
    )

    def normalize_many():
        started.set()
        try:
            for _ in range(80):
                wire_decode("Evidence", raw)
        finally:
            finished.set()

    task = asyncio.create_task(workers.run(normalize_many))
    try:
        while not started.is_set():
            await asyncio.sleep(0.001)
        assert not finished.is_set()
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task
        # Cancellation belongs to the caller; the admitted native work still owns its slot.
        assert workers.slots._value == 1
        ticks = 0
        while not finished.is_set():
            ticks += 1
            await asyncio.sleep(0.001)
        assert ticks > 1
    finally:
        await workers.close()
    assert workers.slots._value == 2


@pytest.mark.anyio
async def test_final_mcp_bytes_include_content_and_unicode():
    from lctx_semantics import wire_tool_result
    from mcp_types import CallToolResult

    @asynccontextmanager
    async def lifespan(_):
        workers = NativeWorkers()
        try:
            yield {"served": SimpleNamespace(workers=workers)}
        finally:
            await workers.close()

    server = FastMCP("final byte control", lifespan=lifespan, dereference_schemas=False)
    payload = {
        "snapshot_id": "11" * 16,
        "generation": "22" * 32,
        "resolution": "ambiguous",
        "requested": "",
        "choices": [],
    }
    overhead = len(wire_tool_result("get_operation", json.dumps(payload), False).encode())
    payload["requested"] = "🦀" * ((32768 - overhead - 50) // 4)
    assert len(wire_tool_result("get_operation", json.dumps(payload), False).encode()) < 32768

    @register(server, ToolAnnotations(read_only_hint=True))
    async def get_operation(snapshot_id, operation, expanded, view, ctx):
        """Final transport budget control."""
        return payload

    async with Client(server) as client:
        args = {"snapshot_id": "11" * 16, "operation": "member"}
        with pytest.raises(ToolError, match="final MCP result byte budget"):
            await client.call_tool("get_operation", args)
        result = await client.call_tool("get_operation", {**args, "expanded": True})
        actual = CallToolResult(
            content=result.content, structured_content=result.structured_content, is_error=False
        )
        assert len(actual.model_dump_json(by_alias=True, exclude_none=True).encode()) <= 256 * 1024
        assert len(result.content) == 1 and isinstance(result.content[0], TextContent)
        assert "🦀" not in result.content[0].text
