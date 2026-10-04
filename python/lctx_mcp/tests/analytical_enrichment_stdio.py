"""Actual stdio consumer of one live Rust enrichment fixture; no mocked server.

Invoked by the owned serving_packets control while its disposable PG18 generation
is live. Native expectations are supplied by that same fixture, never synthesized
by the Python adapter. Source fixtures are not executed.
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
from pathlib import Path

from fastmcp import Client
from fastmcp.client.transports import StdioTransport
from lctx_storage import open_service


async def observe(spec: dict) -> None:
    # Native admission compares its compiled model and physical contract with the
    # exact CLI-created generation before starting the stdio protocol. A mismatch
    # is an adapter/generation prerequisite failure, separate from section parity.
    admitted = await open_service(spec["config"], generation=spec["generation"], vectors=False)
    await admitted.shutdown()
    print("native adapter admitted the exact CLI generation model before stdio")
    transport = StdioTransport(
        sys.executable,
        ["-m", "lctx_mcp", "--config", spec["config"], "--generation", spec["generation"]],
        cwd=str(Path.cwd()),
        env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"},
        keep_alive=False,
    )
    controls = set()
    async with Client(transport, timeout=90, init_timeout=90) as client:
        assert "get_operation" in {tool.name for tool in await client.list_tools()}
        for case in spec["cases"]:
            result = await client.call_tool("get_operation", case["request"])
            assert not result.is_error
            received = result.structured_content
            assert received is not None
            native = case["native"]
            assert received["generation"] == native["generation"]
            packet = received["operation"]["packet"]
            expected = native["operation"]["packet"]
            assert packet["core"]["member"] == expected["core"]["member"]
            for section in case["sections"]:
                actual, authored = packet[section], expected[section]
                for field in ["items", "availability", "omitted", "truncated"]:
                    assert actual[field] == authored[field], (case["control"], section, field)
                # The adapter preserves the existence of a native continuation grant;
                # its opaque value is not a semantic equality or interpretation.
                assert bool(actual.get("continuation")) == bool(authored.get("continuation"))
            if "incoming_references" in case["sections"]:
                assert packet["reference_scope"] == expected["reference_scope"]
                assert packet["reference_scope"]["external_consumers_unknown"]
            control = case["control"]
            if control == "omitted_truncated":
                page = packet["incoming_references"]
                assert page["omitted"] > 0 and page["truncated"]
                assert page["items"] and page.get("continuation")
            elif control == "empty":
                assert packet["incoming_references"]["items"] == []
            elif control == "context_unknown":
                assert packet["contextual_typing"]["items"]
                assert all(
                    not row["error_recovery_known"]
                    for row in packet["contextual_typing"]["items"]
                )
            elif control == "runtime_unknown":
                fields = [
                    row for row in packet["relationships"]["items"]
                    if row["kind"] == "source_field"
                ]
                assert fields and all(
                    row["runtime_value"] == spec["unknown_label"] for row in fields
                )
            controls.add(control)
    assert controls == {
        "comparison", "context_unknown", "omitted_truncated", "empty", "runtime_unknown"
    }
    print(
        "actual stdio enrichment: native comparison, contextual typing, references, "
        "source fields; empty/unknown/omitted/truncated preserved"
    )


if __name__ == "__main__":
    asyncio.run(observe(json.loads(Path(sys.argv[1]).read_text())))
