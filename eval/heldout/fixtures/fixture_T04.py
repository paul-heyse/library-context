"""Held-out fixture T04: advertised input constraints and strict (non-coercing) validation.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`). Offline and deterministic: in-memory client only.
"""

import asyncio

from fastmcp import Client, FastMCP

import solution


async def _schema(client):
    tools = {t.name: t for t in await client.list_tools()}
    assert "set_volume" in tools, sorted(tools)
    return tools["set_volume"].input_schema


async def _call(client, args):
    return await client.call_tool("set_volume", args, raise_on_error=False)


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)


def test_schema_advertises_bounds_and_choices():
    async def run():
        async with Client(solution.mcp) as client:
            schema = await _schema(client)
            props = schema["properties"]
            assert props["level"].get("type") == "integer", props["level"]
            assert props["level"].get("minimum") == 0, props["level"]
            assert props["level"].get("maximum") == 100, props["level"]
            assert set(props["channel"].get("enum", [])) == {"left", "right", "both"}
            assert props["channel"].get("default") == "both"
            assert schema.get("required") == ["level"]

    asyncio.run(run())


def test_valid_calls():
    async def run():
        async with Client(solution.mcp) as client:
            assert (await _call(client, {"level": 50})).data == "both:50"
            assert (await _call(client, {"level": 0, "channel": "left"})).data == "left:0"
            assert (await _call(client, {"level": 100, "channel": "right"})).data == "right:100"

    asyncio.run(run())


def test_invalid_values_are_rejected():
    async def run():
        async with Client(solution.mcp) as client:
            for args in (
                {"level": 101},
                {"level": -1},
                {"level": 50, "channel": "up"},
            ):
                result = await _call(client, args)
                assert result.is_error, f"{args} was accepted: {result.data!r}"

    asyncio.run(run())


def test_string_numbers_are_not_coerced():
    async def run():
        async with Client(solution.mcp) as client:
            result = await _call(client, {"level": "50"})
            assert result.is_error, f"'50' was coerced: {result.data!r}"

    asyncio.run(run())


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
