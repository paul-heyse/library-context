"""Held-out fixture T03: re-expose an existing function under a new name and argument names,
with a server-supplied hidden argument.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`). Offline and deterministic: in-memory client only.
"""

import asyncio

from fastmcp import Client, FastMCP

import solution


async def _tools(client):
    return {t.name: t for t in await client.list_tools()}


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)


def test_only_the_new_name_is_exposed():
    async def run():
        async with Client(solution.mcp) as client:
            tools = await _tools(client)
            assert "find_items" in tools, sorted(tools)
            assert "search" not in tools, sorted(tools)
            assert (tools["find_items"].description or "").strip() == (
                "Find items matching a query."
            )

    asyncio.run(run())


def test_schema_uses_new_argument_names_and_hides_the_key():
    async def run():
        async with Client(solution.mcp) as client:
            schema = (await _tools(client))["find_items"].input_schema
            props = schema["properties"]
            assert set(props) == {"query", "max_results"}, props
            assert props["query"].get("description") == "Search terms"
            assert schema.get("required") == ["query"], schema.get("required")

    asyncio.run(run())


def test_calls_map_to_the_original_function():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("find_items", {"query": "x", "max_results": 2})
            assert result.data == ["x:0", "x:1"], result.data
            result = await client.call_tool("find_items", {"query": "y"})
            assert result.data == [f"y:{i}" for i in range(10)], result.data

    asyncio.run(run())


def test_client_cannot_supply_the_key():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool(
                "find_items",
                {"query": "x", "max_results": 1, "api_key": "evil"},
                raise_on_error=False,
            )
            if result.is_error:
                text = " ".join(getattr(c, "text", "") for c in result.content)
                # Rejected before reaching the function, not passed through to it.
                assert "invalid api key" not in text, text
            else:
                # Ignored: the server's own key was used.
                assert result.data == ["x:0"], result.data

    asyncio.run(run())


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
