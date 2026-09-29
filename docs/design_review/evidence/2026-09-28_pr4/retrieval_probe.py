"""Current PR4 lexical MCP selection, winning-unit and original-byte controls."""
import base64
import json
import os
from pathlib import Path
import sys

from fastmcp import Client
from fastmcp.exceptions import ToolError
from fastmcp.client.transports import StdioTransport
import pyarrow.ipc as ipc


def rows(generation, name):
    return ipc.open_file(generation / (name + ".arrow")).read_all().to_pylist()


async def check(generation, config):
    generation = Path(generation)
    manifest = json.loads((generation / "MANIFEST.json").read_text())
    artifacts = {r["artifact_id"].hex(): r for r in rows(generation, "catalog_artifacts")}
    units = {r["unit_id"].hex(): json.loads(r["detail"]) for r in rows(generation, "retrieval_units")}
    fragments = rows(generation, "retrieval_fragments")
    assert fragments and all(r["embedding_status"] == "not_requested" for r in fragments)
    assert not rows(generation, "retrieval_vectors")
    transport = StdioTransport(sys.executable, ["-m", "lctx_mcp", "--config", str(config), "--library", manifest["library"], "--generation", manifest["projection_generation"], "--embedder", "none"], env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"}, keep_alive=False)
    queries = ["register a tool", "configure transport and authentication", "FastMCP", "mount a server", "deployment environment variables", "expected failure validation"]
    results = []
    inspected = set()
    packet_refusals = []
    async with Client(transport, timeout=120) as client:
        async def call(tool, arguments):
            result = (await client.call_tool(tool, arguments)).structured_content
            assert result is not None
            return result

        selection = {"requirements": [{"predicate": "declares_parameter", "name": "transport"}]}
        find = await call("find_operations", {"library": manifest["library"], "selection": selection, "limit": 2})
        assert find["supported"]["total"] > 0
        assert find["unresolved"]["total"] > 0
        strict = await call("find_operations", {"library": manifest["library"], "selection": {**selection, "mode": "strict"}, "limit": 2})
        assert strict["supported"] == find["supported"] or strict["supported"]["total"] == find["supported"]["total"]
        assert strict["unresolved"]["total"] == strict["conflicting"]["total"] == 0
        typed = await call("find_operations", {"library": manifest["library"], "selection": {"requirements": [{"predicate": "parameter_type", "name": "transport", "type": {"operator": "category", "category": "union"}}]}, "limit": 2})
        assert typed["supported"]["total"] > 0
        for query in queries:
            page = await call("search_operations", {"library": manifest["library"], "query": query, "limit": 5})
            assert page["retrieval"]["actual_route"] == "lexical-only"
            hits = page["supported"]["items"]
            assert hits
            for member in hits:
                packet_refused = False
                try:
                    await call("get_operation", {"snapshot_id": manifest["snapshot_id"], "operation": member["member_id"], "expanded": True})
                except ToolError as error:
                    if not str(error).startswith("storage resource_refused: ResourceRefused: operation packet exceeds expanded byte budget"):
                        raise RuntimeError(f"get_operation {member['access_path']}: {error}") from error
                    packet_refused = True
                    packet_refusals.append({"member": member["access_path"], "member_id": member["member_id"], "query": query, "reason": str(error)})
                for winner in member["ranking"]["winners"]:
                    unit = units[winner["unit_id"]]
                    assert unit["family"] == winner["family"]
                    assert {"kind": "member", "id": member["member_id"]} in unit["subjects"]
                    if winner["family"] in inspected and not packet_refused:
                        continue
                    cursor = None
                    original_seen = False
                    for _ in range(1000):
                        packet = await call("get_evidence", {"snapshot_id": manifest["snapshot_id"], "evidence": {"kind": "retrieval_unit", "id": winner["unit_id"]}, "cursor": cursor, "expanded": True})
                        assert packet["unit"]["unit_id"] == winner["unit_id"]
                        original = packet.get("original")
                        if original:
                            for content in original["content"]:
                                raw = content["text"].encode() if content["text"] is not None else base64.b64decode(content["bytes_base64"])
                                assert raw == artifacts[content["artifact_id"]]["body"][content["chunk_start"]:content["chunk_end"]]
                                original_seen = True
                        cursor = packet["next_cursor"]
                        if cursor is None:
                            break
                    assert cursor is None, "original continuation exceeded probe bound"
                    assert original_seen
                    inspected.add(winner["family"])
            results.append({"query": query, "members": [m["access_path"] for m in hits], "route": page["retrieval"]["actual_route"]})
    assert inspected == {"api_options", "source", "scenario", "documentation_deployment"}
    return {"outcome": "passed", "operation_packets_hydrated": sum(len(row["members"]) for row in results) - len(packet_refusals), "packet_refusals": packet_refusals, "embedding_qualification": "not_run: operator waiver for critical GPU benchmark", "units": len(units), "fragments": len(fragments), "original_families": sorted(inspected), "queries": results}
