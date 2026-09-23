"""The server over stdio in a subprocess (DESIGN §11.3): nothing but the protocol on stdout, at
import, in the lifespan or at start, and no update check (ADR-0010 amendment)."""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest
from fastmcp import Client
from fastmcp.client.transports import StdioTransport

pytestmark = pytest.mark.anyio


async def test_the_server_speaks_only_the_protocol_on_stdout(generation: Path) -> None:
    transport = StdioTransport(
        command=sys.executable,
        args=["-m", "lctx_mcp", "--generation", str(generation), "--embedder", "fake"],
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
    )
    async with Client(transport) as client:
        tools = await client.list_tools()
        assert {t.name for t in tools} == {"search_capabilities", "get_capability"}
        found = await client.call_tool(
            "search_capabilities", {"library": "analysis_shapes", "query": "pkg.describe"}
        )
        result = found.structured_content
        assert isinstance(result, dict) and result["mode"] == "hybrid"
        assert result["hits"][0]["title"] == "pkg.describe"
