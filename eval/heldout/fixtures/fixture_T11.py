"""Held-out fixture T11: parameterised readable data with an optional query setting and a
declared JSON content type.

Tests a candidate module `solution` that exposes a module-level `mcp` (a
`fastmcp.FastMCP`). Offline and deterministic: in-memory client only.
"""

import asyncio
import json

from fastmcp import Client, FastMCP

import solution


async def _read_json(client, uri):
    contents = await client.read_resource(uri)
    assert contents, f"no contents for {uri}"
    item = contents[0]
    assert item.mime_type == "application/json", f"{uri}: mime_type={item.mime_type!r}"
    return json.loads(item.text)


def test_server_object():
    assert isinstance(solution.mcp, FastMCP)


def test_address_pattern_is_discoverable():
    async def run():
        async with Client(solution.mcp) as client:
            templates = await client.list_resource_templates()
            patterns = [t.uri_template for t in templates]
            assert any(p.startswith("weather://{city}/current") for p in patterns), patterns

    asyncio.run(run())


def test_default_units():
    async def run():
        async with Client(solution.mcp) as client:
            payload = await _read_json(client, "weather://london/current")
            assert payload == {"city": "london", "temperature": 22, "units": "c"}, payload

    asyncio.run(run())


def test_query_setting():
    async def run():
        async with Client(solution.mcp) as client:
            payload = await _read_json(client, "weather://paris/current?units=f")
            assert payload == {"city": "paris", "temperature": 71.6, "units": "f"}, payload
            payload = await _read_json(client, "weather://new-york/current?units=c")
            assert payload == {"city": "new-york", "temperature": 22, "units": "c"}, payload

    asyncio.run(run())


if __name__ == "__main__":
    for _name, _fn in sorted(globals().items()):
        if _name.startswith("test_") and callable(_fn):
            _fn()
    print("all tests passed")
