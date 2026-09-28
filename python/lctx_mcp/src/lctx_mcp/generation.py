"""One serving generation, loaded and checked once (DESIGN §6.4, §11.3).

The expected schemas are exported by `cpg_schema::bundle::files`. A generation is refused when a
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
from lctx_semantics import (
    SemanticExecutor,
    catalog_limits,
    native_files,
    serving_schemas,
    validate_projection_ipc,
)

from lctx_mcp.digest import schema_digest
from lctx_mcp.embedder import Spec

FORMAT = 11
KERNEL_FORMAT = 1
MAX_CONDITION_FILE_BYTES = 64 * 1024 * 1024
MAX_SUPPORT_FILE_BYTES = 64 * 1024 * 1024
# The native executor owns which files it loads and their row caps; Python validates exactly those.
_CONDITION_FILES, _SURFACE_FILES, _SUMMARY_FILES, (MAX_SUMMARY_ROWS, MAX_SURFACE_ROWS) = (
    native_files()
)
NATIVE_IPC_FILES = (
    frozenset(_CONDITION_FILES) | frozenset(_SURFACE_FILES) | frozenset(_SUMMARY_FILES)
)


class GenerationError(RuntimeError):
    """A generation this server must not serve; startup fails."""


def expected_schemas(dimensions: int) -> dict[str, pa.Schema]:
    """Schemas exported by Rust; Python independently checks their digests."""
    return {
        name: ipc.open_file(pa.BufferReader(data)).schema
        for name, data in serving_schemas(dimensions)
    }


def generation_key(manifest: dict) -> str:
    """The first 16 hex digits of the SHA-256 of the manifest without its key (as Rust computes)."""
    body = {k: v for k, v in manifest.items() if k != "generation"}
    canonical = json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(canonical.encode()).hexdigest()[:16]


@dataclass(frozen=True)
class GenerationHandle:
    """Pinned file-generation identity, separate from optional relational hydration."""

    root: Path
    manifest: dict
    spec_hash: str | None


@dataclass
class NativeState:
    executor: SemanticExecutor | None


@dataclass
class LexicalState:
    brief_ids: list[bytes]
    brief_text: list[str]
    symbols: dict[str, list[bytes]]
    operation_ids: list[bytes]
    operation_text: list[str]


@dataclass
class Generation:
    """A loaded generation: its tables as rows, and the vectors aligned to `brief_ids`."""

    handle: GenerationHandle
    lexical_state: LexicalState
    native_state: NativeState
    tables: dict[str, pa.Table]
    briefs: dict[bytes, dict]
    vectors: np.ndarray | None
    vector_rows: list[bytes] = field(default_factory=list)
    # FORMAT 3: the public surface (ADR-0021).
    operations: dict[bytes, dict] = field(default_factory=dict)
    paths: dict[str, bytes] = field(default_factory=dict)
    spellings: dict[bytes, list[tuple[str, bool]]] = field(default_factory=dict)
    # Per operation, its facet rows `(facet, value, verdict)`.
    facets: dict[bytes, list[tuple[str, str, str]]] = field(default_factory=dict)
    # Per `(facet, value)`, the operations with that row and its verdict.
    by_facet: dict[tuple[str, str], dict[bytes, str]] = field(default_factory=dict)
    # Per operation and facet, `(verdict, reason)`: `established` when its rows are complete.
    facet_status: dict[bytes, dict[str, tuple[str, str | None]]] = field(default_factory=dict)
    behaviors: dict[bytes, list[dict]] = field(default_factory=dict)
    # FORMAT 5: a singleton global's class, its fields' reads, and place claims by key.
    singletons: dict[str, bytes] = field(default_factory=dict)
    ambient: dict[str, list[dict]] = field(default_factory=dict)
    claims: dict[str, dict] = field(default_factory=dict)
    op_vectors: dict[str, tuple[np.ndarray, list[bytes]]] = field(default_factory=dict)

    @property
    def root(self) -> Path:
        return self.handle.root

    @property
    def manifest(self) -> dict:
        return self.handle.manifest

    @property
    def spec_hash(self) -> str | None:
        return self.handle.spec_hash

    @property
    def condition_graph(self) -> SemanticExecutor | None:
        return self.native_state.executor

    @property
    def brief_ids(self) -> list[bytes]:
        return self.lexical_state.brief_ids

    @property
    def lexical(self) -> list[str]:
        return self.lexical_state.brief_text

    @property
    def symbols(self) -> dict[str, list[bytes]]:
        return self.lexical_state.symbols

    @property
    def op_ids(self) -> list[bytes]:
        return self.lexical_state.operation_ids

    @property
    def op_text(self) -> list[str]:
        return self.lexical_state.operation_text

    @property
    def snapshot_id(self) -> str:
        return self.manifest["snapshot_id"]

    @property
    def key(self) -> str:
        return self.manifest["generation"]

    @property
    def library(self) -> str:
        return self.manifest["library"]


def _read(
    root: Path,
    manifest: dict,
    name: str,
    schema: pa.Schema,
    native_ipc: dict[str, bytes] | None = None,
) -> pa.Table:
    entry = manifest["files"].get(name)
    if entry is None:
        raise GenerationError(f"MANIFEST.json lists no {name}")
    if entry.get("file") != f"{name}.arrow":
        raise GenerationError(f"{name}: unexpected served file path")
    path = root / entry["file"]
    if path.is_symlink():
        raise GenerationError(f"{entry['file']}: a served file cannot be a symlink")
    if (
        name in NATIVE_IPC_FILES
        and name not in _SURFACE_FILES
        and path.stat().st_size > MAX_CONDITION_FILE_BYTES
    ):
        raise GenerationError(f"{entry['file']}: condition file exceeds the load budget")
    if name.startswith("support_") and path.stat().st_size > MAX_SUPPORT_FILE_BYTES:
        raise GenerationError(f"{entry['file']}: support file exceeds the load budget")
    data = path.read_bytes()
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
    if native_ipc is not None:
        native_ipc[name] = data
    return table


def load(root: Path, client_spec: Spec | None) -> Generation:
    """Load and check the generation at `root` for a server querying with `client_spec`."""
    manifest = json.loads((root / "MANIFEST.json").read_text(encoding="utf-8"))
    if manifest.get("format") != FORMAT:
        raise GenerationError(f"manifest format {manifest.get('format')}, not {FORMAT}")
    if manifest.get("condition_kernel_format") != KERNEL_FORMAT:
        raise GenerationError(
            f"condition kernel format {manifest.get('condition_kernel_format')}, "
            f"not {KERNEL_FORMAT}"
        )
    key = generation_key(manifest)
    if manifest.get("generation") != key:
        raise GenerationError(f"the generation key is {key}, not the manifest's")
    if root.name != key:
        raise GenerationError(f"the generation directory is {root.name}, not {key}")
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
    native_ipc: dict[str, bytes] = {}
    tables = {
        name: _read(root, manifest, name, schema, native_ipc) for name, schema in schemas.items()
    }
    try:
        projection_key = validate_projection_ipc(
            json.dumps(manifest["projection"]), list(native_ipc.items()), json.dumps(manifest)
        )
    except (KeyError, ValueError) as exc:
        raise GenerationError(f"invalid serving projection: {exc}") from exc
    if projection_key != manifest.get("projection_generation"):
        raise GenerationError("projection generation identity mismatch")
    max_conditions, max_nodes, _max_retained_nodes = catalog_limits()
    if (
        tables["conditions"].num_rows + tables["analysis_conditions"].num_rows > max_conditions
        or tables["condition_nodes"].num_rows + tables["analysis_condition_nodes"].num_rows
        > max_nodes
    ):
        raise GenerationError("condition catalog exceeds native load limits")
    for name in _SUMMARY_FILES:
        if tables[name].num_rows > MAX_SUMMARY_ROWS:
            raise GenerationError(f"{name} exceeds native load limits")
    for name in _SURFACE_FILES:
        if tables[name].num_rows > MAX_SURFACE_ROWS:
            raise GenerationError(f"{name} exceeds native load limits")
    try:
        condition_graph = SemanticExecutor.from_ipc(
            KERNEL_FORMAT,
            manifest["snapshot_id"],
            manifest["entry_value_effect_digest"],
            [(name, data) for name, data in native_ipc.items() if name in NATIVE_IPC_FILES],
        )
    except ValueError as e:
        raise GenerationError(f"invalid semantic index: {e}") from e

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
    # FORMAT 10 (ADR-0064): each graded call-transfer claim's member evidence, by claim.
    discharges: dict[bytes, list[dict]] = {}
    for r in tables["behavior_discharges"].to_pylist():
        discharges.setdefault(r["behavior_id"], []).append(r)
    behaviors: dict[bytes, list[dict]] = {}
    for r in tables["behaviors"].to_pylist():
        r["discharges"] = discharges.get(r["behavior_id"], [])
        behaviors.setdefault(r["operation_node_id"], []).append(r)
    singletons = {r["global"]: r["class_node_id"] for r in tables["singletons"].to_pylist()}
    ambient: dict[str, list[dict]] = {}
    for r in tables["ambient_reads"].to_pylist():
        ambient.setdefault(r["global"], []).append(r)
    claims = {r["place_key"]: r for r in tables["place_claims"].to_pylist()}
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
        handle=GenerationHandle(root, manifest, spec_hash),
        native_state=NativeState(condition_graph),
        lexical_state=LexicalState(
            brief_ids,
            [lexical_of.get(b, "") for b in brief_ids],
            symbols,
            op_ids,
            [op_text_of.get(n, "") for n in op_ids],
        ),
        tables=tables,
        briefs={r["brief_id"]: r for r in brief_rows},
        vectors=vectors,
        vector_rows=vector_rows,
        operations={r["node_id"]: r for r in op_rows},
        paths=paths,
        spellings=spellings,
        facets=facets,
        by_facet=by_facet,
        facet_status=facet_status,
        behaviors=behaviors,
        singletons=singletons,
        ambient=ambient,
        claims=claims,
        op_vectors=op_vectors,
    )
