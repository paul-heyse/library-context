"""ADR-0010 spike: a FastMCP server over one pinned, file-based serving generation (DESIGN §11).

Throwaway. It checks the contract, not the product: a lifespan that rejects schema-digest and
embedding-spec mismatches at startup, the two tools with object outputs, exact cosine + BM25 +
RRF (K=60, 1-based ranks, ties by brief_id), and a degraded lexical-only mode.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
from collections.abc import Callable
from contextlib import asynccontextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Annotated, Literal

import numpy as np
import pyarrow as pa
import pyarrow.ipc as ipc
from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp_types import ToolAnnotations
from pydantic import BaseModel, Field

DIM = 4096
LIBRARY = "fastmcp"
QUERY_TASK = "Given a coding task, retrieve capability briefs of a Python library that solve it"

SPEC = {
    "model": "Qwen/Qwen3-Embedding-8B",
    "revision": "1d8ad4ca9b3dd8059ad90a75d4983776a23d44af",
    "dimensions": DIM,
    "dtype": "float32",
    "pooling": "last-token (model default)",
    "normalization": "l2",
    "query_template": "Instruct: {task_description}\nQuery:{query}",
    "document_template": "{text}",
}


def canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def spec_hash(spec: dict) -> str:
    return hashlib.sha256(canonical_json(spec)).hexdigest()


def schema_digest(schema: pa.Schema) -> str:
    """Canonical schema form: name, type, nullability, sorted metadata (DESIGN §6.4)."""
    fields = [
        {
            "name": f.name,
            "type": str(f.type),
            "nullable": f.nullable,
            "metadata": sorted((k.decode(), v.decode()) for k, v in (f.metadata or {}).items()),
        }
        for f in schema
    ]
    return hashlib.sha256(canonical_json(fields)).hexdigest()


BRIEFS = pa.schema(
    [
        pa.field("brief_id", pa.binary(16), nullable=False),
        pa.field("title", pa.string(), nullable=False),
        pa.field("outcome", pa.string(), nullable=True),
        pa.field("outcome_status", pa.string(), nullable=False),
        pa.field("public_symbol", pa.string(), nullable=False),
        pa.field("lexical_text", pa.string(), nullable=False),
    ]
)
VECTORS = pa.schema(
    [
        pa.field("brief_id", pa.binary(16), nullable=False),
        pa.field("vector", pa.list_(pa.field("item", pa.float32(), nullable=False), DIM), False),
    ]
)
EXPECTED = {"briefs.arrow": BRIEFS, "vectors.arrow": VECTORS}


def query_text(query: str) -> str:
    return SPEC["query_template"].format(task_description=QUERY_TASK, query=query)


Embedder = Callable[[list[str]], np.ndarray]


class GenerationError(RuntimeError):
    """A generation that does not match what this server expects; startup must fail."""


@dataclass
class Generation:
    snapshot_id: str
    briefs: pa.Table
    vectors: np.ndarray  # rows aligned with briefs, unit norm
    tokens: list[list[str]]


def tokenize(text: str) -> list[str]:
    return re.findall(r"[a-z0-9]+", text.lower().replace("_", " "))


def load_generation(root: Path) -> Generation:
    manifest = json.loads((root / "MANIFEST.json").read_text())
    if manifest["embedding_spec_hash"] != spec_hash(SPEC):
        raise GenerationError("embedding spec hash differs from the query client's spec")
    tables = {}
    for name, schema in EXPECTED.items():
        entry = manifest["files"][name]
        data = (root / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != entry["sha256"]:
            raise GenerationError(f"{name}: sha256 differs from MANIFEST")
        if entry["schema_digest"] != schema_digest(schema):
            raise GenerationError(f"{name}: schema digest differs from the expected schema")
        table = ipc.open_file(pa.BufferReader(data)).read_all()
        if schema_digest(table.schema) != schema_digest(schema):
            raise GenerationError(f"{name}: file schema differs from the expected schema")
        tables[name] = table
    briefs, vecs = tables["briefs.arrow"], tables["vectors.arrow"]
    if briefs.column("brief_id").to_pylist() != vecs.column("brief_id").to_pylist():
        raise GenerationError("vectors are not aligned with briefs")
    flat = vecs.column("vector").combine_chunks().flatten().to_numpy()
    matrix = flat.reshape(-1, DIM).astype(np.float32)
    norms = np.linalg.norm(matrix, axis=1)
    if not np.all(np.isfinite(matrix)) or np.any(np.abs(norms - 1.0) > 1e-3):
        raise GenerationError("vectors are not finite unit vectors")
    tokens = [tokenize(t) for t in briefs.column("lexical_text").to_pylist()]
    return Generation(manifest["snapshot_id"], briefs, matrix, tokens)


def bm25(query: list[str], docs: list[list[str]], k1: float = 1.2, b: float = 0.75) -> list[float]:
    n = len(docs)
    avg = sum(len(d) for d in docs) / max(n, 1)
    df = {t: sum(1 for d in docs if t in d) for t in set(query)}
    scores = []
    for d in docs:
        s = 0.0
        for t in query:
            f = d.count(t)
            if f:
                idf = math.log(1 + (n - df[t] + 0.5) / (df[t] + 0.5))
                s += idf * f * (k1 + 1) / (f + k1 * (1 - b + b * len(d) / avg))
        scores.append(s)
    return scores


def ranks(scores: list[float], ids: list[bytes]) -> dict[bytes, int]:
    order = sorted(
        (i for i in range(len(ids)) if scores[i] > 0), key=lambda i: (-scores[i], ids[i])
    )
    return {ids[i]: r + 1 for r, i in enumerate(order)}


class Hit(BaseModel):
    brief_id: str
    title: str
    outcome: str | None
    outcome_status: str
    relevance: float
    rank_source: Literal["hybrid", "lexical", "vector", "exact_symbol"]
    promoted: bool


class SearchResult(BaseModel):
    snapshot_id: str
    mode: Literal["hybrid", "lexical-only"]
    coverage: str
    hits: list[Hit]


class Capability(BaseModel):
    snapshot_id: str
    capability_id: str
    title: str
    outcome: str | None
    outcome_status: str
    public_symbol: str


READ_ONLY = ToolAnnotations(read_only_hint=True, idempotent_hint=True, open_world_hint=False)


def build_server(generation_dir: Path, embed: Embedder) -> FastMCP:
    @asynccontextmanager
    async def lifespan(_server: FastMCP):
        yield {"gen": load_generation(generation_dir)}

    mcp = FastMCP("lctx-spike", lifespan=lifespan, mask_error_details=True)

    @mcp.tool(annotations=READ_ONLY)
    def search_capabilities(
        library: str,
        query: Annotated[str, Field(min_length=1, max_length=4000)],
        ctx: Context,
        limit: Annotated[int, Field(ge=1, le=10)] = 5,
    ) -> SearchResult:
        """Find published capability briefs relevant to a coding task."""
        if library != LIBRARY:
            raise ToolError(f"unknown library: {library}")
        gen: Generation = ctx.lifespan_context["gen"]
        ids = gen.briefs.column("brief_id").to_pylist()
        lexical = ranks(bm25(tokenize(query), gen.tokens), ids)
        try:
            q = embed([query_text(query)])[0]
            vector = ranks((gen.vectors @ q).tolist(), ids)
            mode = "hybrid"
        except Exception:
            vector, mode = {}, "lexical-only"
        fused = {
            i: sum(1.0 / (60 + r[i]) for r in (lexical, vector) if i in r)
            for i in set(lexical) | set(vector)
        }
        symbols = gen.briefs.column("public_symbol").to_pylist()
        promoted = {ids[k] for k, s in enumerate(symbols) if s == query.strip()}
        for i in promoted:
            fused[i] = fused.get(i, 0.0) + 1.0
        order = sorted(fused, key=lambda i: (-fused[i], i))[:limit]
        rows = {ids[k]: k for k in range(len(ids))}
        hits = []
        for i in order:
            k = rows[i]
            hits.append(
                Hit(
                    brief_id=i.hex(),
                    title=gen.briefs.column("title")[k].as_py(),
                    outcome=gen.briefs.column("outcome")[k].as_py(),
                    outcome_status=gen.briefs.column("outcome_status")[k].as_py(),
                    relevance=round(fused[i], 6),
                    rank_source="exact_symbol"
                    if i in promoted
                    else (
                        "hybrid"
                        if i in lexical and i in vector
                        else ("vector" if i in vector else "lexical")
                    ),
                    promoted=i in promoted,
                )
            )
        return SearchResult(
            snapshot_id=gen.snapshot_id, mode=mode, coverage="fixture corpus", hits=hits
        )

    @mcp.tool(annotations=READ_ONLY)
    def get_capability(snapshot_id: str, capability_id: str, ctx: Context) -> Capability:
        """Return one published capability brief by id."""
        gen: Generation = ctx.lifespan_context["gen"]
        if snapshot_id != gen.snapshot_id:
            raise ToolError("snapshot mismatch")
        ids = [b.hex() for b in gen.briefs.column("brief_id").to_pylist()]
        if capability_id not in ids:
            raise ToolError(f"unknown capability: {capability_id}")
        k = ids.index(capability_id)
        return Capability(
            snapshot_id=gen.snapshot_id,
            capability_id=capability_id,
            title=gen.briefs.column("title")[k].as_py(),
            outcome=gen.briefs.column("outcome")[k].as_py(),
            outcome_status=gen.briefs.column("outcome_status")[k].as_py(),
            public_symbol=gen.briefs.column("public_symbol")[k].as_py(),
        )

    return mcp


def write_generation(
    root: Path,
    briefs: list[dict],
    vectors: np.ndarray,
    *,
    spec: dict = SPEC,
    briefs_schema: pa.Schema = BRIEFS,
) -> None:
    """Fixture writer (the Rust bundle builder's job in the product)."""
    root.mkdir(parents=True, exist_ok=True)
    tables = {
        "briefs.arrow": pa.Table.from_pylist(briefs, schema=briefs_schema),
        "vectors.arrow": pa.Table.from_arrays(
            [
                pa.array([b["brief_id"] for b in briefs], pa.binary(16)),
                pa.FixedSizeListArray.from_arrays(
                    pa.array(vectors.astype(np.float32).ravel(), pa.float32()), DIM
                ).cast(VECTORS.field("vector").type),
            ],
            schema=VECTORS,
        ),
    }
    files = {}
    for name, table in tables.items():
        sink = pa.BufferOutputStream()
        with ipc.new_file(sink, table.schema) as w:
            w.write_table(table)
        data = sink.getvalue().to_pybytes()
        (root / name).write_bytes(data)
        files[name] = {
            "sha256": hashlib.sha256(data).hexdigest(),
            "rows": table.num_rows,
            "schema_digest": schema_digest(table.schema),
        }
    manifest = {"snapshot_id": "00" * 16, "embedding_spec_hash": spec_hash(spec), "files": files}
    (root / "MANIFEST.json").write_text(json.dumps(manifest, indent=2, sort_keys=True))
