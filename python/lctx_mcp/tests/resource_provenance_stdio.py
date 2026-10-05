"""Actual PG/native/MCP capability resource provenance, invoked by serving_packets."""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
import sys
from pathlib import Path

from fastmcp import Client
from fastmcp.client.transports import StdioTransport
from lctx_semantics import wire_tool
from lctx_storage import open_service
from mcp_types import ReadResourceResult, TextResourceContents

from lctx_mcp.wire import response_encodings


async def observe(spec: dict) -> None:
    admitted = await open_service(spec["config"], generation=spec["generation"], vectors=False)
    await admitted.shutdown()
    transport = StdioTransport(
        sys.executable,
        ["-m", "lctx_mcp", "--config", spec["config"], "--generation", spec["generation"]],
        cwd=str(Path.cwd()),
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
        keep_alive=False,
    )
    expected = spec["native"]
    capability = expected["capability"]["capability"]
    uri = f"lctx://capability/{bytes(capability).hex()}"
    async with Client(transport, timeout=90, init_timeout=90) as client:
        tool = await client.call_tool(
            "get_capability", {"capability": capability, "page": {"expanded": True}}
        )
        assert not tool.is_error
        assert tool.structured_content == expected
        contents = await client.read_resource(uri)
        assert len(contents) == 1
        resource = contents[0]
        assert isinstance(resource, TextResourceContents)
        assert str(resource.uri) == uri
        assert resource.mime_type == "text/markdown"
        body = expected["capability"]["rendered"]
        text = resource.text
        assert text[: len(body)] == body
        assert hashlib.sha256(text[: len(body)].encode()).hexdigest() == spec["authored_sha256"]
        suffix = text[len(body) :]
        metadata = json.loads(suffix.split("```json\n", 1)[1].split("\n```", 1)[0])
        assert metadata == {
            "generation": expected["generation"],
            "capability": capability,
            "uri_scope": "process",
        }
        assertion_text = suffix.split("## Assertion evidence\n", 1)[1]
        retained = [
            json.loads(block.split("\n```", 1)[0])
            for block in assertion_text.split("```json\n")[1:]
        ]
        assert len(retained) == len(expected["capability"]["assertions"])
        for actual, original in zip(retained, expected["capability"]["assertions"], strict=True):
            for field in [
                "assertion",
                "status",
                "kind",
                "qualification",
                "claim_basis",
                "terminal_question",
                "text",
                "supports",
            ]:
                assert actual[field] == original[field]
        result = ReadResourceResult(contents=contents)
        bound = json.loads(wire_tool("get_capability"))["byte_limits"]["expanded"]
        stdio, http = response_encodings(result, "snapshot-λ")
        assert len(stdio) <= bound and len(http) <= bound
    print(
        "actual capability stdio: authored bytes/hash, tool/resource snapshot, "
        "assertion evidence and SDK envelopes passed"
    )


if __name__ == "__main__":
    asyncio.run(observe(json.loads(Path(sys.argv[1]).read_text())))
