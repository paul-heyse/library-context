"""The server over stdio in a subprocess (DESIGN §11.3): nothing but the protocol on stdout, at
import, in the lifespan or at start, and no update check (ADR-0010 amendment).

The MCP client skips a stdout line it cannot parse, so a round trip through it cannot see stray
output (increment-1 deep review F6). This test speaks JSON-RPC over the raw pipe instead and
requires every stdout line to be a JSON-RPC message; a noisy server fails it (the control).
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
from pathlib import Path

import pytest

pytestmark = pytest.mark.anyio

MESSAGES = [
    {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "raw", "version": "0"},
        },
    },
    {"jsonrpc": "2.0", "method": "notifications/initialized"},
    {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
    {
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "search_capabilities",
            "arguments": {"library": "analysis_shapes", "query": "pkg.describe"},
        },
    },
]


async def stdout_lines(args: list[str]) -> list[str]:
    """Every stdout line the server writes until it answers the last request."""
    process = await asyncio.create_subprocess_exec(
        *args,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.DEVNULL,
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
    )
    assert process.stdin is not None and process.stdout is not None
    lines: list[str] = []
    try:
        for message in MESSAGES:
            process.stdin.write((json.dumps(message) + "\n").encode())
        await process.stdin.drain()
        while True:
            raw = await asyncio.wait_for(process.stdout.readline(), timeout=60)
            if not raw:
                break
            line = raw.decode().rstrip("\n")
            lines.append(line)
            try:
                if json.loads(line).get("id") == 3:
                    break
            except ValueError, AttributeError:
                continue
    finally:
        process.kill()
        await process.wait()
    return lines


def not_protocol(lines: list[str]) -> list[str]:
    bad = []
    for line in lines:
        try:
            message = json.loads(line)
            if not isinstance(message, dict) or message.get("jsonrpc") != "2.0":
                bad.append(line)
        except ValueError:
            bad.append(line)
    return bad


def server(generation: Path) -> list[str]:
    return ["-m", "lctx_mcp", "--generation", str(generation), "--embedder", "fake"]


async def test_the_server_speaks_only_the_protocol_on_stdout(generation: Path) -> None:
    lines = await stdout_lines([sys.executable, *server(generation)])
    assert not_protocol(lines) == []
    answer = json.loads(lines[-1])
    assert answer["id"] == 3 and not answer["result"].get("isError")
    hits = answer["result"]["structuredContent"]["hits"]
    assert hits[0]["title"] == "pkg.describe"


async def test_a_server_that_prints_fails_the_check(generation: Path) -> None:
    """The control: the same check sees a line printed before the server starts."""
    noisy = (
        "import runpy, sys; print('noise', flush=True); "
        f"sys.argv = ['lctx_mcp', *{server(generation)[2:]!r}]; "
        "runpy.run_module('lctx_mcp', run_name='__main__')"
    )
    lines = await stdout_lines([sys.executable, "-c", noisy])
    assert not_protocol(lines) == ["noise"]
