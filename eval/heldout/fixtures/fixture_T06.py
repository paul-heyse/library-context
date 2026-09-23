"""Held-out fixture T06: typed structured results with advertised result shapes.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`). Offline and deterministic: in-memory client only.
"""

import asyncio

from fastmcp import Client, FastMCP

import solution

PROFILE = {"name": "Alice", "age": 30, "email": "alice@example.com"}


async def _tools(client):
    return {t.name: t for t in await client.list_tools()}


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)


def test_profile_result_shape_is_advertised():
    async def run():
        async with Client(solution.mcp) as client:
            tools = await _tools(client)
            assert "get_profile" in tools, sorted(tools)
            schema = tools["get_profile"].output_schema
            assert schema is not None, "get_profile advertises no result shape"
            assert schema.get("type") == "object", schema
            props = schema.get("properties", {})
            assert props.get("name", {}).get("type") == "string", props
            assert props.get("age", {}).get("type") == "integer", props
            assert props.get("email", {}).get("type") == "string", props

    asyncio.run(run())


def test_count_result_shape_is_advertised():
    async def run():
        async with Client(solution.mcp) as client:
            tools = await _tools(client)
            assert "count_users" in tools, sorted(tools)
            schema = tools["count_users"].output_schema
            assert schema is not None, "count_users advertises no result shape"
            assert schema.get("type") == "object", schema
            integer_fields = [
                p for p in schema.get("properties", {}).values() if p.get("type") == "integer"
            ]
            assert integer_fields, f"no integer field in {schema}"

    asyncio.run(run())


def test_profile_call_returns_structured_and_text_content():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("get_profile", {"user_id": "u1"})
            assert result.structured_content == PROFILE, result.structured_content
            assert result.content, "no text content for display"
            data = result.data
            name = data.get("name") if isinstance(data, dict) else getattr(data, "name", None)
            assert name == "Alice", data

    asyncio.run(run())


def test_count_call_is_structured_and_unwraps_to_int():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("count_users", {})
            assert result.structured_content is not None, "count_users is text-only"
            assert result.data == 3, result.data

    asyncio.run(run())


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
