"""Read live contextual evidence and both observed tasks through actual stdio MCP."""
import asyncio
import base64
import json
import os
from pathlib import Path
import sys
import tempfile

from fastmcp import Client
from fastmcp.client.transports import StdioTransport
import pyarrow.ipc as ipc

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / "scripts"))
from postgres_test_support import database
from postgres_expand import write_secret


def rows(generation, table):
    return ipc.open_file(generation / (table + ".arrow")).read_all().to_pylist()


async def check(generation, config):
    manifest = json.loads((generation / "MANIFEST.json").read_text())
    artifacts = {r["artifact_id"]: r for r in rows(generation, "catalog_artifacts")}
    spans = {r["span_id"]: r for r in rows(generation, "catalog_spans")}
    scenarios = rows(generation, "catalog_scenarios")
    deployments = rows(generation, "catalog_deployments")
    tasks = [r for r in deployments if json.loads(r["detail"])["task"] is not None]
    assert len(tasks) == 2
    associations = rows(generation, "catalog_associations")
    release = [r for r in associations if r["release_id"] is not None]
    assert release and all(r["member_id"] is None for r in release)
    selected = [("span", next(iter(spans))), ("scenario", scenarios[0]["scenario_id"])]
    selected += [("deployment", r["deployment_id"]) for r in tasks]
    transport = StdioTransport(sys.executable, ["-m", "lctx_mcp", "--config", str(config), "--library", manifest["library"], "--generation", manifest["projection_generation"], "--embedder", "none"], env={**os.environ, "FASTMCP_CHECK_FOR_UPDATES": "off"}, keep_alive=False)
    observed = []
    async with Client(transport, timeout=60) as client:
        assert "get_evidence" in {tool.name for tool in await client.list_tools()}
        for kind, identity in selected:
            cursor = None
            collected = {}
            pages = 0
            while True:
                packet = (await client.call_tool("get_evidence", {"snapshot_id": manifest["snapshot_id"], "evidence": {"kind": kind, "id": identity.hex()}, "expanded": True, "cursor": cursor})).structured_content
                assert packet and packet["result_kind"] == "evidence"
                assert len(json.dumps(packet, ensure_ascii=False, separators=(",", ":")).encode()) <= 262144
                for content in packet["content"]:
                    original = artifacts[bytes.fromhex(content["artifact_id"])]
                    raw = content["text"].encode() if content["text"] is not None else base64.b64decode(content["bytes_base64"])
                    assert raw == original["body"][content["chunk_start"]:content["chunk_end"]]
                    collected.setdefault(content["span_id"], bytearray()).extend(raw)
                if kind == "deployment":
                    assert packet["deployment"]["task"]["receipt"]["execution"] == "passed"
                    assert packet["deployment"]["task"]["receipt"]["result"] == "5"
                pages += 1
                assert pages < 1000
                cursor = packet["next_cursor"]
                if cursor is None:
                    break
            for span_id, body in collected.items():
                span = spans[bytes.fromhex(span_id)]
                assert body == artifacts[span["artifact_id"]]["body"][span["start_byte"]:span["end_byte"]]
            observed.append({"kind": kind, "id": identity.hex(), "pages": pages, "original_bytes": "passed"})
    return {"profile": manifest["capabilities"], "generation": manifest["projection_generation"], "release_associations": len(release), "all_associations": len(associations), "evidence": observed}


def main():
    output = Path(sys.argv[1])
    pilots = json.loads((output / "receipt.json").read_text())
    result = []
    with tempfile.TemporaryDirectory(prefix="lctx-pr3-evidence-") as tmp:
        config = Path(tmp)
        with database(config) as (serving, _, call, role, _):
            importer = config / "postgres-importer.json"
            write_secret(importer, {**role, "role": "importer", "url": role["url"].replace("lctx_serving:", "lctx_importer:"), "statement_timeout_seconds": 300})
            for pilot in pilots:
                generation = Path(pilot["generation"])
                call([str(ROOT / "target/release/lctx"), "serving", "--importer-config", str(importer), "import-bundle", "--bundle", str(generation), "--artifacts", str(output / "evidence-artifacts")])
                result.append(asyncio.run(check(generation, serving)))
    (output / "evidence-receipt.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
