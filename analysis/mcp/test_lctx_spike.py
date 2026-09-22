"""ADR-0010 spike tests: MCP round trip in both protocol eras, and startup mismatch fixtures."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import numpy as np
import pyarrow as pa
import pytest
from fastmcp import Client

import lctx_spike as S

pytestmark = pytest.mark.anyio


@pytest.fixture
def anyio_backend():
    return "asyncio"


def fake_embed(texts: list[str]) -> np.ndarray:
    """Deterministic hashed bag-of-words unit vectors (the GPU-free test embedder)."""
    out = np.zeros((len(texts), S.DIM), dtype=np.float32)
    for row, text in enumerate(texts):
        for tok in S.tokenize(text.split("Query:")[-1]):
            h = int.from_bytes(hashlib.blake2b(tok.encode(), digest_size=8).digest(), "little")
            out[row, h % S.DIM] += 1.0
        n = np.linalg.norm(out[row])
        out[row] = out[row] / n if n else out[row]
    return out


def down(_texts: list[str]) -> np.ndarray:
    raise ConnectionError("embedding service down")


BRIEFS = [
    ("register a python function as an mcp tool with a decorator", "fastmcp.FastMCP.tool"),
    ("expose a file or dynamic data as an mcp resource", "fastmcp.FastMCP.resource"),
    ("define a reusable prompt template", "fastmcp.FastMCP.prompt"),
    ("run code around every request with middleware", "fastmcp.FastMCP.add_middleware"),
]


def rows() -> list[dict]:
    return [
        {
            "brief_id": hashlib.blake2b(sym.encode(), digest_size=16).digest(),
            "title": text.capitalize(),
            "outcome": text,
            "outcome_status": "documented",
            "public_symbol": sym,
            "lexical_text": f"{text} {sym}",
        }
        for text, sym in BRIEFS
    ]


@pytest.fixture
def generation(tmp_path: Path) -> Path:
    briefs = rows()
    vectors = fake_embed([b["lexical_text"] for b in briefs])
    S.write_generation(tmp_path / "gen", briefs, vectors)
    return tmp_path / "gen"


@pytest.mark.parametrize("mode", ["auto", "legacy"])
async def test_round_trip_in_both_protocol_eras(generation: Path, mode: str):
    mcp = S.build_server(generation, fake_embed)
    async with Client(mcp, mode=mode) as client:
        tools = {t.name: t for t in await client.list_tools()}
        assert set(tools) == {"search_capabilities", "get_capability"}
        for t in tools.values():
            ann = t.annotations
            assert (ann.read_only_hint, ann.idempotent_hint, ann.open_world_hint) == (
                True,
                True,
                False,
            )
            assert t.output_schema is not None and t.output_schema.get("type") == "object"
        r = await client.call_tool(
            "search_capabilities",
            {"library": "fastmcp", "query": "register my function as a tool", "limit": 2},
        )
        sc = r.structured_content
        assert sc["mode"] == "hybrid" and len(sc["hits"]) == 2
        top = sc["hits"][0]
        assert top["title"].startswith("Register a python function")
        g = await client.call_tool(
            "get_capability", {"snapshot_id": sc["snapshot_id"], "capability_id": top["brief_id"]}
        )
        assert g.structured_content["public_symbol"] == "fastmcp.FastMCP.tool"
        print(f"mode={mode} protocol={client.protocol_version} top={top['title']!r}")


async def test_exact_symbol_is_promoted(generation: Path):
    async with Client(S.build_server(generation, fake_embed)) as client:
        r = await client.call_tool(
            "search_capabilities", {"library": "fastmcp", "query": "fastmcp.FastMCP.prompt"}
        )
        top = r.structured_content["hits"][0]
        assert top["promoted"] and top["rank_source"] == "exact_symbol"


async def test_degraded_lexical_only_mode_is_reported(generation: Path):
    async with Client(S.build_server(generation, down)) as client:
        r = await client.call_tool(
            "search_capabilities", {"library": "fastmcp", "query": "middleware around requests"}
        )
        assert r.structured_content["mode"] == "lexical-only"
        assert r.structured_content["hits"][0]["title"].startswith("Run code around")


async def test_unknown_library_and_capability_raise_tool_errors(generation: Path):
    async with Client(S.build_server(generation, fake_embed)) as client:
        r = await client.call_tool(
            "search_capabilities", {"library": "numpy", "query": "x"}, raise_on_error=False
        )
        assert r.is_error
        r = await client.call_tool(
            "get_capability",
            {"snapshot_id": "00" * 16, "capability_id": "ff" * 16},
            raise_on_error=False,
        )
        assert r.is_error


async def test_schema_mismatch_fails_at_startup(tmp_path: Path):
    wrong = S.BRIEFS.set(3, pa.field("outcome_status", pa.int16(), nullable=False))
    briefs = [dict(b, outcome_status=1) for b in rows()]
    S.write_generation(
        tmp_path / "gen", briefs, fake_embed(["x"] * len(briefs)), briefs_schema=wrong
    )
    with pytest.raises(Exception) as err:
        async with Client(S.build_server(tmp_path / "gen", fake_embed)):
            pass
    print("schema mismatch:", repr(err.value)[:200])
    assert "schema digest" in repr(err.value) or "schema digest" in str(err.value.__cause__)


async def test_spec_mismatch_fails_at_startup(tmp_path: Path):
    other = dict(S.SPEC, model="Qwen/Qwen3-Embedding-4B", dimensions=2560)
    briefs = rows()
    S.write_generation(tmp_path / "gen", briefs, fake_embed(["x"] * len(briefs)), spec=other)
    with pytest.raises(Exception) as err:
        async with Client(S.build_server(tmp_path / "gen", fake_embed)):
            pass
    print("spec mismatch:", repr(err.value)[:200])
    assert "spec" in repr(err.value) or "spec" in str(err.value.__cause__)


def test_manifest_digests_are_the_canonical_form(generation: Path):
    m = json.loads((generation / "MANIFEST.json").read_text())
    assert m["files"]["briefs.arrow"]["schema_digest"] == S.schema_digest(S.BRIEFS)
