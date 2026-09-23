"""Held-out fixture T01: docstring-driven tool metadata and behaviour hints.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`). Offline and deterministic: in-memory client only.
"""

import asyncio

from fastmcp import Client, FastMCP

import solution


def _hint(annotations, snake, camel):
    if annotations is None:
        return None
    value = getattr(annotations, snake, None)
    if value is None:
        value = getattr(annotations, camel, None)
    return value


async def _get_tool(client, name):
    tools = {t.name: t for t in await client.list_tools()}
    assert name in tools, f"tool {name!r} not listed; got {sorted(tools)}"
    return tools[name]


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)


def test_description_comes_from_docstring_summary():
    async def run():
        async with Client(solution.mcp) as client:
            tool = await _get_tool(client, "lookup_order")
            description = (tool.description or "").strip()
            assert description.startswith("Look up a customer order by its ID."), description
            assert "Args" not in description, description
            assert "The order identifier" not in description, description

    asyncio.run(run())


def test_parameter_descriptions_come_from_args_section():
    async def run():
        async with Client(solution.mcp) as client:
            tool = await _get_tool(client, "lookup_order")
            props = tool.input_schema["properties"]
            assert set(props) == {"order_id", "include_items"}, props
            assert props["order_id"].get("description") == "The order identifier, e.g. A-100."
            assert props["include_items"].get("description") == (
                "Whether to include line items in the result."
            )
            assert tool.input_schema.get("required") == ["order_id"]

    asyncio.run(run())


def test_behaviour_hints_and_title():
    async def run():
        async with Client(solution.mcp) as client:
            tool = await _get_tool(client, "lookup_order")
            assert tool.title == "Look up order", tool.title
            ann = tool.annotations
            assert ann is not None, "no behaviour hints advertised"
            assert _hint(ann, "read_only_hint", "readOnlyHint") is True
            assert _hint(ann, "idempotent_hint", "idempotentHint") is True
            assert _hint(ann, "open_world_hint", "openWorldHint") is False

    asyncio.run(run())


def test_call_behaviour():
    async def run():
        async with Client(solution.mcp) as client:
            result = await client.call_tool("lookup_order", {"order_id": "A-100"})
            assert result.structured_content == {"id": "A-100", "status": "shipped"}
            result = await client.call_tool(
                "lookup_order", {"order_id": "A-100", "include_items": True}
            )
            assert result.structured_content == {
                "id": "A-100",
                "status": "shipped",
                "items": ["widget", "gadget"],
            }

    asyncio.run(run())


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
