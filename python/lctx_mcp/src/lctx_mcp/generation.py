"""One serving generation, loaded and checked once (DESIGN §6.4, §11.3).

The expected schemas mirror `cpg_schema::bundle::files`. A generation is refused at load when a
file's sha256, row count or serving schema digest differs from its manifest or from what this
server expects, when its key does not match its manifest, or when its embedding spec differs
from the query client's.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
import pyarrow as pa
import pyarrow.ipc as ipc

from lctx_mcp.digest import schema_digest
from lctx_mcp.embedder import Spec

FORMAT = 4


class GenerationError(RuntimeError):
    """A generation this server must not serve; startup fails."""


def _id(name: str, nullable: bool = False) -> pa.Field:
    return pa.field(name, pa.binary(16), nullable=nullable)


def _utf8(name: str, nullable: bool = False) -> pa.Field:
    return pa.field(name, pa.string(), nullable=nullable)


def _int(name: str, nullable: bool = False) -> pa.Field:
    return pa.field(name, pa.int64(), nullable=nullable)


def expected_schemas(dimensions: int) -> dict[str, pa.Schema]:
    """Every served file's schema, in manifest order (`cpg_schema::bundle::files`)."""
    vector = pa.list_(pa.field("item", pa.float32(), nullable=False), dimensions)
    return {
        "briefs": pa.schema(
            [
                _id("brief_id"),
                _id("seed_node_id"),
                _utf8("access_path"),
                _utf8("title"),
                _utf8("applicable_case", True),
                pa.field("documentation_only", pa.bool_(), nullable=False),
                _utf8("review_state"),
                _utf8("outcome", True),
                _utf8("outcome_status"),
            ]
        ),
        "assertions": pa.schema(
            [
                _id("brief_id"),
                _int("ordinal"),
                _id("assertion_id"),
                _utf8("kind"),
                _utf8("section"),
                _utf8("status"),
                _utf8("text", True),
                _utf8("applicable_case", True),
                _utf8("conditions", True),
                _utf8("limitations", True),
                _int("template_version"),
            ]
        ),
        "supports": pa.schema(
            [
                _id("assertion_id"),
                _utf8("role"),
                _int("ordinal"),
                _id("finding_id", True),
                _utf8("finding_kind", True),
                _id("evidence_id", True),
            ]
        ),
        "evidence": pa.schema(
            [
                _id("evidence_id"),
                _utf8("kind"),
                _id("node_id", True),
                _utf8("path", True),
                _int("start_byte", True),
                _int("end_byte", True),
                _utf8("text", True),
            ]
        ),
        "brief_members": pa.schema(
            [
                _id("brief_id"),
                _utf8("access_path"),
                _id("export_node_id"),
                _id("declaration_node_id"),
                pa.field("own", pa.bool_(), nullable=False),
            ]
        ),
        "symbol_map": pa.schema([_utf8("symbol"), _id("brief_id")]),
        "public_paths": pa.schema(
            [
                _id("node_id"),
                _utf8("access_path"),
                _utf8("kind"),
                pa.field("own", pa.bool_(), nullable=False),
                pa.field("preferred", pa.bool_(), nullable=False),
            ]
        ),
        "lexical_text": pa.schema([_id("brief_id"), _utf8("text")]),
        "embedding_spec": pa.schema(
            [pa.field("spec_hash", pa.binary(32), nullable=False), _utf8("spec")]
        ),
        "vectors": pa.schema(
            [
                _id("brief_id"),
                _int("chunk"),
                pa.field("input_hash", pa.binary(32), nullable=False),
                pa.field("vector", vector, nullable=False),
            ]
        ),
        # FORMAT 3 (ADR-0021): the whole public surface.
        "operations": pa.schema(
            [
                _id("node_id"),
                _utf8("access_path"),
                _utf8("kind"),
                pa.field("is_method", pa.bool_(), nullable=False),
                _utf8("qualified_name"),
                _utf8("module"),
                _utf8("docstring_summary", True),
                _utf8("behavior_status"),
                # FORMAT 4 (increment 3's deep review, F2).
                _utf8("boundary_reason", True),
                _utf8("status_reason", True),
                _id("brief_id", True),
            ]
        ),
        "operation_facets": pa.schema(
            [_id("node_id"), _utf8("facet"), _utf8("value"), _utf8("verdict")]
        ),
        # FORMAT 4 (F3, F4): whether each operation's rows for each facet are complete.
        "operation_facet_status": pa.schema(
            [_id("node_id"), _utf8("facet"), _utf8("verdict"), _utf8("reason", True)]
        ),
        "behaviors": pa.schema(
            [
                _id("behavior_id"),
                _id("operation_node_id"),
                _utf8("kind"),
                _utf8("parameter_name", True),
                _id("callee_node_id", True),
                _utf8("callee", True),
                _utf8("target_name", True),
                _utf8("value", True),
                _int("depth"),
                pa.field("conditional", pa.bool_(), nullable=False),
                _utf8("verdict"),
                _utf8("boundary_reason", True),
                _int("occurrences"),
                _utf8("path", True),
                _int("line", True),
                _utf8("site_text", True),
            ]
        ),
        "operation_text": pa.schema([_id("node_id"), _utf8("text")]),
        "operation_vectors": pa.schema(
            [
                _id("node_id"),
                _utf8("embedding_view"),
                _int("chunk"),
                pa.field("input_hash", pa.binary(32), nullable=False),
                pa.field("vector", vector, nullable=False),
            ]
        ),
    }


def generation_key(manifest: dict) -> str:
    """The first 16 hex digits of the SHA-256 of the manifest without its key (as Rust computes)."""
    body = {k: v for k, v in manifest.items() if k != "generation"}
    canonical = json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(canonical.encode()).hexdigest()[:16]


@dataclass
class Generation:
    """A loaded generation: its tables as rows, and the vectors aligned to `brief_ids`."""

    root: Path
    manifest: dict
    tables: dict[str, pa.Table]
    brief_ids: list[bytes]
    briefs: dict[bytes, dict]
    lexical: list[str]
    symbols: dict[str, list[bytes]]
    spec_hash: str | None
    vectors: np.ndarray | None
    vector_rows: list[bytes] = field(default_factory=list)
    # FORMAT 3: the public surface (ADR-0021).
    op_ids: list[bytes] = field(default_factory=list)
    operations: dict[bytes, dict] = field(default_factory=dict)
    op_text: list[str] = field(default_factory=list)
    paths: dict[str, bytes] = field(default_factory=dict)
    spellings: dict[bytes, list[tuple[str, bool]]] = field(default_factory=dict)
    # Per operation, its facet rows `(facet, value, verdict)`.
    facets: dict[bytes, list[tuple[str, str, str]]] = field(default_factory=dict)
    # Per `(facet, value)`, the operations with that row and its verdict.
    by_facet: dict[tuple[str, str], dict[bytes, str]] = field(default_factory=dict)
    # Per operation and facet, `(verdict, reason)`: `established` when its rows are complete.
    facet_status: dict[bytes, dict[str, tuple[str, str | None]]] = field(default_factory=dict)
    behaviors: dict[bytes, list[dict]] = field(default_factory=dict)
    op_vectors: dict[str, tuple[np.ndarray, list[bytes]]] = field(default_factory=dict)

    @property
    def snapshot_id(self) -> str:
        return self.manifest["snapshot_id"]

    @property
    def key(self) -> str:
        return self.manifest["generation"]

    @property
    def library(self) -> str:
        return self.manifest["library"]


def _read(root: Path, manifest: dict, name: str, schema: pa.Schema) -> pa.Table:
    entry = manifest["files"].get(name)
    if entry is None:
        raise GenerationError(f"MANIFEST.json lists no {name}")
    data = (root / entry["file"]).read_bytes()
    if hashlib.sha256(data).hexdigest() != entry["sha256"]:
        raise GenerationError(f"{entry['file']}: its sha256 differs from the manifest")
    want = schema_digest(schema)
    if entry["schema_digest"] != want:
        raise GenerationError(
            f"{entry['file']}: its schema digest is not the one this server serves"
        )
    table = ipc.open_file(pa.BufferReader(data)).read_all()
    if schema_digest(table.schema) != want:
        raise GenerationError(f"{entry['file']}: its schema differs from its schema digest")
    if table.num_rows != entry["rows"]:
        raise GenerationError(f"{entry['file']}: {table.num_rows} rows, not the manifest's")
    return table


def load(root: Path, client_spec: Spec | None) -> Generation:
    """Load and check the generation at `root` for a server querying with `client_spec`."""
    manifest = json.loads((root / "MANIFEST.json").read_text(encoding="utf-8"))
    if manifest.get("format") != FORMAT:
        raise GenerationError(f"manifest format {manifest.get('format')}, not {FORMAT}")
    key = generation_key(manifest)
    if manifest.get("generation") != key:
        raise GenerationError(f"the generation key is {key}, not the manifest's")
    spec_hash: str | None = manifest.get("spec_hash")
    specs = _read(root, manifest, "embedding_spec", expected_schemas(0)["embedding_spec"])
    dimensions = 0
    if spec_hash is not None:
        if specs.num_rows != 1 or specs.column("spec_hash")[0].as_py().hex() != spec_hash:
            raise GenerationError("embedding_spec does not hold the manifest's spec")
        spec = Spec.from_json(specs.column("spec")[0].as_py())
        if spec.hash != spec_hash:
            raise GenerationError("the served spec's JSON does not hash to its spec hash")
        if client_spec is not None and client_spec.hash != spec_hash:
            raise GenerationError(
                f"the generation's embedding spec {spec_hash[:12]}… is not the query "
                f"client's {client_spec.hash[:12]}…"
            )
        dimensions = spec.dimensions
    schemas = expected_schemas(dimensions)
    tables = {name: _read(root, manifest, name, schema) for name, schema in schemas.items()}

    brief_rows = tables["briefs"].to_pylist()
    brief_ids = [r["brief_id"] for r in brief_rows]
    lexical_of = {r["brief_id"]: r["text"] for r in tables["lexical_text"].to_pylist()}
    symbols: dict[str, list[bytes]] = {}
    for r in tables["symbol_map"].to_pylist():
        symbols.setdefault(r["symbol"], []).append(r["brief_id"])

    vectors = None
    vector_rows: list[bytes] = []
    if tables["vectors"].num_rows:
        column = tables["vectors"].column("vector").combine_chunks()
        vectors = column.flatten().to_numpy().reshape(-1, dimensions).astype(np.float32)
        norms = np.linalg.norm(vectors.astype(np.float64), axis=1)
        if not np.all(np.isfinite(vectors)) or np.any(np.abs(norms - 1.0) > 1e-3):
            raise GenerationError("the vectors are not finite unit vectors")
        vector_rows = tables["vectors"].column("brief_id").to_pylist()
    op_rows = tables["operations"].to_pylist()
    op_ids = [r["node_id"] for r in op_rows]
    op_text_of = {r["node_id"]: r["text"] for r in tables["operation_text"].to_pylist()}
    paths: dict[str, bytes] = {}
    spellings: dict[bytes, list[tuple[str, bool]]] = {}
    for r in tables["public_paths"].to_pylist():
        paths[r["access_path"]] = r["node_id"]
        spellings.setdefault(r["node_id"], []).append((r["access_path"], r["own"]))
    facets: dict[bytes, list[tuple[str, str, str]]] = {}
    by_facet: dict[tuple[str, str], dict[bytes, str]] = {}
    for r in tables["operation_facets"].to_pylist():
        facets.setdefault(r["node_id"], []).append((r["facet"], r["value"], r["verdict"]))
        by_facet.setdefault((r["facet"], r["value"]), {})[r["node_id"]] = r["verdict"]
    facet_status: dict[bytes, dict[str, tuple[str, str | None]]] = {}
    for r in tables["operation_facet_status"].to_pylist():
        facet_status.setdefault(r["node_id"], {})[r["facet"]] = (r["verdict"], r["reason"])
    behaviors: dict[bytes, list[dict]] = {}
    for r in tables["behaviors"].to_pylist():
        behaviors.setdefault(r["operation_node_id"], []).append(r)
    op_vectors: dict[str, tuple[np.ndarray, list[bytes]]] = {}
    ov = tables["operation_vectors"]
    if ov.num_rows:
        matrix = ov.column("vector").combine_chunks().flatten().to_numpy()
        matrix = matrix.reshape(-1, dimensions).astype(np.float32)
        norms = np.linalg.norm(matrix.astype(np.float64), axis=1)
        if not np.all(np.isfinite(matrix)) or np.any(np.abs(norms - 1.0) > 1e-3):
            raise GenerationError("the operation vectors are not finite unit vectors")
        views = ov.column("embedding_view").to_pylist()
        nodes = ov.column("node_id").to_pylist()
        for view in sorted(set(views)):
            rows = [i for i, v in enumerate(views) if v == view]
            op_vectors[view] = (matrix[rows], [nodes[i] for i in rows])
    return Generation(
        root=root,
        manifest=manifest,
        tables=tables,
        brief_ids=brief_ids,
        briefs={r["brief_id"]: r for r in brief_rows},
        lexical=[lexical_of.get(b, "") for b in brief_ids],
        symbols=symbols,
        spec_hash=spec_hash,
        vectors=vectors,
        vector_rows=vector_rows,
        op_ids=op_ids,
        operations={r["node_id"]: r for r in op_rows},
        op_text=[op_text_of.get(n, "") for n in op_ids],
        paths=paths,
        spellings=spellings,
        facets=facets,
        by_facet=by_facet,
        facet_status=facet_status,
        behaviors=behaviors,
        op_vectors=op_vectors,
    )
