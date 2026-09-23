"""Held-out fixture T14: one redacted audit record per tool call, from a cross-cutting layer.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`) and a module-level list `AUDIT_LOG`. Offline and deterministic:
in-memory client only.
"""

import asyncio

from fastmcp import Client, FastMCP
from fastmcp.exceptions import ToolError

import solution


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)
    assert isinstance(solution.AUDIT_LOG, list)


def test_listing_adds_no_records():
    solution.AUDIT_LOG.clear()

    async def run():
        async with Client(solution.mcp) as client:
            await client.list_tools()
            await client.list_resources()
        assert solution.AUDIT_LOG == [], solution.AUDIT_LOG

    asyncio.run(run())


def test_successful_call_is_recorded_and_redacted():
    solution.AUDIT_LOG.clear()

    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("echo", {"text": "top secret"})
            assert result.data == "top secret", result.data
        assert len(solution.AUDIT_LOG) == 1, solution.AUDIT_LOG
        record = solution.AUDIT_LOG[0]
        assert record["tool"] == "echo", record
        assert record["status"] == "completed", record
        assert record["arguments"] == {"text": "<redacted>"}, record
        assert "top secret" not in repr(record), record

    asyncio.run(run())


def test_failed_call_is_recorded_and_still_errors():
    solution.AUDIT_LOG.clear()

    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("explode", {}, raise_on_error=False)
            assert result.is_error, "explode did not reach the client as an error"
            try:
                await client.call_tool("explode", {})
            except ToolError:
                pass
            else:
                raise AssertionError("client did not raise ToolError")
        assert len(solution.AUDIT_LOG) == 2, solution.AUDIT_LOG
        for record in solution.AUDIT_LOG:
            assert record["tool"] == "explode", record
            assert record["status"] == "failed", record
            assert record["arguments"] == {}, record

    asyncio.run(run())


def test_one_record_per_call_across_tools():
    solution.AUDIT_LOG.clear()

    async def run():
        async with Client(solution.mcp) as client:
            await client.call_tool("echo", {"text": "a"})
            await client.call_tool("explode", {}, raise_on_error=False)
            await client.call_tool("echo", {"text": "b"})
        assert [(r["tool"], r["status"]) for r in solution.AUDIT_LOG] == [
            ("echo", "completed"),
            ("explode", "failed"),
            ("echo", "completed"),
        ], solution.AUDIT_LOG

    asyncio.run(run())


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
