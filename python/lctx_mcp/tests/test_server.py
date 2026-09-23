"""The FastMCP contract (DESIGN §11.3) over the fixture generation, in both protocol eras."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

import numpy as np
import pytest
from fastmcp import Client
from fastmcp.exceptions import ToolError

from lctx_mcp.embedder import EmbedderError, FakeEmbedder, HttpEmbedder, Spec
from lctx_mcp.generation import GenerationError, generation_key, load
from lctx_mcp.server import build_server

pytestmark = pytest.mark.anyio

LIBRARY = "analysis_shapes"


def structured(result: object) -> dict:
    """A tool result's structured content, which every tool here has (object outputs)."""
    content = getattr(result, "structured_content", None)
    assert isinstance(content, dict)
    return content


class Down:
    """A query embedder whose service is unreachable."""

    spec = FakeEmbedder().spec

    async def embed(self, texts: list[str]) -> np.ndarray:
        raise EmbedderError("no embedding service at http://127.0.0.1:9")


@pytest.mark.parametrize(("mode", "protocol"), [("auto", "2026-07-28"), ("legacy", "2025-11-25")])
async def test_the_tools_round_trip_in_both_protocol_eras(
    generation: Path, mode: str, protocol: str
) -> None:
    async with Client(build_server(generation, FakeEmbedder()), mode=mode) as client:
        assert client.protocol_version == protocol
        tools = {t.name: t for t in await client.list_tools()}
        assert set(tools) == {"search_capabilities", "get_capability"}
        for t in tools.values():
            a = t.annotations
            assert a is not None
            assert (a.read_only_hint, a.idempotent_hint, a.open_world_hint) == (True, True, False)
            assert t.output_schema is not None and t.output_schema.get("type") == "object"
        found = await client.call_tool(
            "search_capabilities", {"library": LIBRARY, "query": "register fn as a tool"}
        )
        result = structured(found)
        assert result is not None
        assert result["mode"] == "hybrid" and result["degraded_reason"] is None
        assert result["library"] == LIBRARY and len(result["generation"]) == 16
        assert result["coverage"]["invocations"] == {"complete_under_stated_model": 4}
        assert 1 <= len(result["hits"]) <= 5
        tool = next(h for h in result["hits"] if h["title"] == "pkg.Server.tool")
        brief = await client.call_tool(
            "get_capability",
            {"snapshot_id": result["snapshot_id"], "capability_id": tool["capability_id"]},
        )
        c = structured(brief)
        assert c["outcome"] == "Register `fn` as a tool." and c["outcome_status"] == "documented"
        assert c["review_state"] == "unreviewed" and not c["documentation_only"]
        sections = [a["section"] for a in c["assertions"]]
        assert sections[0] == "outcome" and "limits" in sections
        outcome = c["assertions"][0]
        cited = {e["evidence_id"]: e for e in c["evidence"]}
        span = cited[outcome["supports"][0]["evidence_id"]]
        assert span["kind"] == "span" and span["text"] == "Register `fn` as a tool."
        assert span["path"].endswith("pkg/server.py")


async def test_a_public_path_is_promoted(generation: Path) -> None:
    async with Client(build_server(generation, FakeEmbedder())) as client:
        found = await client.call_tool(
            "search_capabilities", {"library": LIBRARY, "query": "pkg.helpers.helper", "limit": 1}
        )
        (hit,) = structured(found)["hits"]
        assert hit["title"] == "pkg.helper" and hit["promoted"]
        assert hit["rank_source"] == "exact_symbol"


async def test_a_down_embedder_degrades_to_lexical_and_says_so(generation: Path) -> None:
    async with Client(build_server(generation, Down())) as client:
        found = await client.call_tool(
            "search_capabilities", {"library": LIBRARY, "query": "register fn as a tool"}
        )
        result = structured(found)
        assert result["mode"] == "lexical-only"
        assert "embedding service failed" in result["degraded_reason"]
        assert result["hits"][0]["title"] == "pkg.Server.tool"
        assert all(h["rank_source"] == "lexical" for h in result["hits"])


async def test_unknown_libraries_ids_and_snapshots_are_tool_errors(generation: Path) -> None:
    async with Client(build_server(generation, None)) as client:
        found = await client.call_tool(
            "search_capabilities", {"library": LIBRARY, "query": "helper"}
        )
        assert structured(found)["mode"] == "lexical-only"
        snapshot = structured(found)["snapshot_id"]
        cases = [
            ("search_capabilities", {"library": "numpy", "query": "x"}, "serves"),
            ("get_capability", {"snapshot_id": "00" * 16, "capability_id": "ff" * 16}, "search"),
            ("get_capability", {"snapshot_id": snapshot, "capability_id": "ff" * 16}, "no cap"),
            ("get_capability", {"snapshot_id": snapshot, "capability_id": "zz"}, "not a cap"),
            ("search_capabilities", {"library": LIBRARY, "query": ""}, None),
            ("search_capabilities", {"library": LIBRARY, "query": "x", "limit": 11}, None),
        ]
        for name, args, message in cases:
            with pytest.raises(ToolError) as err:
                await client.call_tool(name, args)
            if message is not None:
                assert message in str(err.value), (name, args, str(err.value))


async def test_the_resource_is_the_brief_as_markdown(generation: Path) -> None:
    async with Client(build_server(generation, None)) as client:
        found = await client.call_tool(
            "search_capabilities", {"library": LIBRARY, "query": "pkg.Server.route"}
        )
        hit = structured(found)["hits"][0]
        uri = f"capability://{structured(found)['snapshot_id']}/{hit['capability_id']}"
        (content,) = await client.read_resource(uri)
        text = getattr(content, "text", "")
        assert text.startswith("# pkg.Server.route\n")
        assert "Register a handler at `path`. [documented]" in text
        with pytest.raises(Exception, match="no capability"):
            await client.read_resource(
                f"capability://{structured(found)['snapshot_id']}/{'ff' * 16}"
            )


def tampered(generation: Path, tmp: Path, edit) -> Path:
    """A copy of the generation with its manifest edited and its key recomputed."""
    manifest = json.loads((generation / "MANIFEST.json").read_text(encoding="utf-8"))
    edit(manifest)
    manifest["generation"] = generation_key(manifest)
    copy = tmp / manifest["generation"]
    shutil.copytree(generation, copy)
    (copy / "MANIFEST.json").write_text(json.dumps(manifest, indent=2, sort_keys=True))
    return copy


async def test_a_mismatched_generation_fails_at_connect(generation: Path, tmp_path: Path) -> None:
    def schema(m: dict) -> None:
        m["files"]["briefs"]["schema_digest"] = "0" * 64

    wrong_schema = tampered(generation, tmp_path, schema)
    with pytest.raises(GenerationError, match="schema digest"):
        load(wrong_schema, FakeEmbedder().spec)
    with pytest.raises(RuntimeError, match="Client failed to connect") as err:
        async with Client(build_server(wrong_schema, FakeEmbedder())):
            pass
    assert "schema digest" in repr(err.value.__cause__) or "schema digest" in str(err.value)

    live = HttpEmbedder("http://127.0.0.1:9", spec=Spec.packaged("qwen3-embedding-8b.json"))
    with pytest.raises(RuntimeError, match="Client failed to connect") as err:
        async with Client(build_server(generation, live)):
            pass
    assert "embedding spec" in str(err.value)

    moved = tmp_path / "moved"
    shutil.copytree(generation, moved)
    (moved / "MANIFEST.json").write_text(
        (generation / "MANIFEST.json").read_text().replace('"rows": 4', '"rows": 5', 1)
    )
    with pytest.raises(GenerationError, match="generation key"):
        load(moved, None)
