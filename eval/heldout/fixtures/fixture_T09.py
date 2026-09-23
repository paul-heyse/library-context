"""Held-out fixture T09: deliberate error messages reach the client; internal ones are masked
server-wide, including for tools registered later.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`). Offline and deterministic: in-memory client only.
"""

import asyncio

from fastmcp import Client, FastMCP
from fastmcp.exceptions import ToolError

import solution


def _text(result):
    return " ".join(getattr(c, "text", "") for c in result.content)


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)


def test_divide_happy_path():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("divide", {"a": 6, "b": 3})
            assert result.data == 2.0, result.data

    asyncio.run(run())


def test_division_by_zero_message_reaches_client():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("divide", {"a": 1, "b": 0}, raise_on_error=False)
            assert result.is_error, "division by zero did not produce a tool error"
            assert "Division by zero is not allowed." in _text(result), _text(result)
            try:
                await client.call_tool("divide", {"a": 1, "b": 0})
            except ToolError as exc:
                assert "Division by zero is not allowed." in str(exc), str(exc)
            else:
                raise AssertionError("client did not raise ToolError")

    asyncio.run(run())


def test_internal_failure_is_masked():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("load_secret", {}, raise_on_error=False)
            assert result.is_error, "load_secret did not fail"
            text = _text(result)
            assert "hunter2" not in text, text
            assert "db password" not in text, text

    asyncio.run(run())


def test_masking_covers_tools_added_later():
    @solution.mcp.tool
    def late_failure_probe() -> str:
        raise KeyError("internal-key-xyz")

    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("late_failure_probe", {}, raise_on_error=False)
            assert result.is_error
            assert "internal-key-xyz" not in _text(result), _text(result)

    asyncio.run(run())


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
