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
from lctx_semantics import SemanticExecutor, catalog_limits, native_files

from lctx_mcp.digest import schema_digest
from lctx_mcp.embedder import Spec

FORMAT = 10
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
        "support_findings": pa.schema(
            [
                _id("finding_id"),
                _utf8("finding_kind"),
                _utf8("evidence_status"),
                _id("subject_node_id"),
                _id("related_node_id", True),
                _id("invocation_id"),
                _utf8("model_id"),
                _utf8("method"),
                _utf8("parameters"),
                _utf8("completion"),
                _utf8("stop_reason", True),
                pa.field("witnesses_omitted", pa.bool_(), nullable=False),
            ]
        ),
        "support_witnesses": pa.schema(
            [
                _id("finding_id"),
                _int("path"),
                _int("step"),
                _id("caller_node_id"),
                _id("call_site_node_id"),
                _id("callee_node_id"),
                _utf8("arc_kind"),
                _utf8("modality"),
                _utf8("phase", True),
                _id("source_fact_id", True),
                _utf8("source_path", True),
                _int("start_byte", True),
                _int("end_byte", True),
            ]
        ),
        "support_members": pa.schema(
            [
                _id("finding_id"),
                _utf8("role"),
                _int("ordinal"),
                _id("node_id", True),
                _id("cited_fact_id", True),
                _id("attribute_id", True),
                _utf8("fact_table", True),
                _utf8("fact_model_id", True),
                _utf8("label", True),
            ]
        ),
        "support_attributes": pa.schema(
            [
                _id("attribute_id"),
                _utf8("kind"),
                _utf8("symbol", True),
                _utf8("parameter_kind", True),
                _id("type_term_id", True),
                _utf8("class_module", True),
                _utf8("class_key", True),
                _id("target_node_id", True),
                _utf8("modality", True),
                _utf8("phase", True),
                _utf8("producer_modality", True),
                _utf8("producer_phase", True),
                _utf8("display"),
            ]
        ),
        "support_attribute_incidences": pa.schema(
            [
                _id("finding_id"),
                _id("incidence_id"),
                _id("attribute_id"),
                _id("object_node_id"),
                _id("source_fact_id"),
                _utf8("fact_table"),
                _utf8("fact_model_id"),
                _id("site_node_id", True),
                _id("edge_id", True),
                _id("other_site_node_id", True),
                _id("other_edge_id", True),
                _id("consumer_formal_id", True),
                _id("other_fact_id", True),
                _utf8("other_fact_table", True),
                _utf8("other_fact_model_id", True),
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
                _utf8("transfer", True),
                _id("condition_scope_node_id"),
                _utf8("parameter_name", True),
                _id("callee_node_id", True),
                _utf8("callee", True),
                _utf8("target_name", True),
                _utf8("value", True),
                _int("depth"),
                pa.field("conditional", pa.bool_(), nullable=False),
                _utf8("verdict"),
                _utf8("boundary_reason", True),
                # FORMAT 5 (Stage 2; ADR-0022).
                _utf8("condition", True),
                _utf8("callee_text", True),
                _utf8("phase", True),
                _utf8("premise_key", True),
                _int("occurrences"),
                _utf8("path", True),
                _int("line", True),
                _utf8("site_text", True),
            ]
        ),
        # FORMAT 6 (ADR-0024): the condition graph is retained for native semantic queries.
        "conditions": pa.schema(
            [_id("condition_id"), _id("root_id", True), _utf8("boundary_reason", True)]
        ),
        "condition_nodes": pa.schema(
            [_id("node_id"), _utf8("atom"), _id("low_id"), _id("high_id")]
        ),
        "analysis_conditions": pa.schema(
            [_id("condition_id"), _id("root_id", True), _utf8("boundary_reason", True)]
        ),
        "analysis_condition_nodes": pa.schema(
            [_id("node_id"), _utf8("atom"), _id("low_id"), _id("high_id")]
        ),
        "callable_parameters": pa.schema(
            [_id("function_node_id"), _id("formal_node_id"), _utf8("name")]
        ),
        "summary_flows": pa.schema(
            [
                _id("summary_id"),
                _id("function_node_id"),
                _id("parameter_node_id"),
                _utf8("input_path"),
                _utf8("output_path"),
                _utf8("kind"),
                _id("condition_id"),
                _utf8("verdict"),
                _utf8("boundary_reason", True),
                _id("source_flow_fact_id"),
                _id("source_origin_id"),
                _id("return_site_fact_id"),
                _id("return_region_fact_id"),
                pa.field("approximated", pa.bool_(), nullable=False),
                _int("path_depth"),
            ]
        ),
        "source_context_value_identities": pa.schema(
            [
                _id("identity_id"),
                _id("function_node_id"),
                _id("parameter_node_id"),
                _utf8("parameter_name"),
                _id("source_flow_fact_id"),
                _id("source_origin_id"),
                _id("condition_id"),
                _id("return_site_fact_id"),
                _id("return_region_fact_id"),
                _id("return_condition_id"),
                _int("return_start_byte"),
                _id("context_site_id"),
                _id("argument_fact_id"),
                _id("argument_expression_fact_id"),
                _id("argument_reference_fact_id"),
                _id("argument_resolution_fact_id"),
                _id("parameter_binding_fact_id"),
                _id("parameter_fact_id"),
                _id("target_binding_fact_id"),
                _id("expression_fact_id"),
                _id("reference_fact_id"),
                _id("resolution_fact_id"),
                _id("scope_fact_id"),
                _id("module_node_id"),
                _int("start_byte"),
                _int("end_byte"),
            ]
        ),
        "source_body_completions": pa.schema(
            [
                _id("body_id"),
                _id("function_node_id"),
                _id("declaration_fact_id"),
                _id("syntax_fact_id"),
                _utf8("kind"),
                _id("terminal_fact_id", True),
                _utf8("exception", True),
                _utf8("reason", True),
                _utf8("release_reason", True),
                pa.field("function_retainer_required", pa.bool_(), nullable=False),
                _int("runtime_statement_count"),
                _int("step_count"),
                pa.field("steps_digest", pa.binary(32), nullable=False),
                _int("release_count"),
                pa.field("releases_digest", pa.binary(32), nullable=False),
                _int("work"),
            ]
        ),
        "source_body_steps": pa.schema(
            [
                _id("body_id"),
                _int("ordinal"),
                _utf8("kind"),
                _id("evidence_id"),
            ]
        ),
        "source_body_release_inputs": pa.schema(
            [
                _id("body_id"),
                _int("ordinal"),
                _id("syntax_fact_id"),
                _utf8("safety"),
                _int("proof_offset"),
                _int("proof_count"),
                pa.field("proof_digest", pa.binary(32), nullable=False),
                _id("evaluation_evidence_id"),
            ]
        ),
        "source_call_bindings": pa.schema(
            [
                _id("binding_id"),
                _id("function_node_id"),
                _id("call_node_id"),
                _id("call_fact_id"),
                _id("syntax_fact_id"),
                _id("callee_node_id"),
                _id("pysa_fact_id"),
                _id("signature_fact_id"),
                _id("declaration_fact_id"),
                _id("header_fact_id"),
                _id("statement_fact_id"),
                _id("binding_fact_id"),
                _id("reference_fact_id"),
                _id("resolution_fact_id"),
                _int("header_count"),
                pa.field("header_digest", pa.binary(32), nullable=False),
            ]
        ),
        "source_call_normals": pa.schema(
            [
                _id("certificate_id"),
                _id("binding_id"),
                _id("body_id"),
                _int("body_count"),
                _utf8("body_kind"),
            ]
        ),
        "source_call_header_steps": pa.schema(
            [_id("binding_id"), _int("ordinal"), _utf8("kind"), _id("evidence_id")]
        ),
        "model_frame_exits": pa.schema(
            [
                _id("frame_exit_id"),
                _id("function_node_id"),
                _id("call_node_id"),
                _id("call_fact_id"),
                _id("syntax_fact_id"),
                _id("target_node_id"),
                _id("pysa_fact_id"),
                _id("model_id"),
                _utf8("return_parameter"),
                _id("return_argument_fact_id"),
                _int("argument_count"),
                _int("signature_count"),
                pa.field("arguments_digest", pa.binary(32), nullable=False),
                _int("invocation_count"),
                pa.field("invocation_digest", pa.binary(32), nullable=False),
            ]
        ),
        "model_frame_exit_arguments": pa.schema(
            [
                _id("frame_exit_id"),
                _int("ordinal"),
                _id("argument_fact_id"),
                _id("expression_fact_id"),
                _utf8("safety"),
                _utf8("parameter_name"),
                _int("expression_offset"),
                _int("expression_count"),
                pa.field("expression_digest", pa.binary(32), nullable=False),
                pa.field("parameters_digest", pa.binary(32), nullable=False),
            ]
        ),
        "model_frame_exit_steps": pa.schema(
            [
                _id("frame_exit_id"),
                _int("ordinal"),
                _id("operand_fact_id"),
                _id("evidence_id"),
                _utf8("status"),
                _utf8("kind"),
            ]
        ),
        "source_modeled_identities": pa.schema(
            [
                _id("identity_id"),
                _id("function_node_id"),
                _id("parameter_node_id"),
                _id("source_flow_fact_id"),
                _id("source_origin_id"),
                _id("condition_id"),
                _id("return_site_fact_id"),
                _id("call_fact_id"),
                _id("call_expression_fact_id"),
                _id("source_argument_fact_id"),
                _id("pysa_fact_id"),
                _id("model_id"),
                _id("rule_id"),
                _id("callee_resolution_fact_id"),
                _id("expression_fact_id"),
                _id("reference_fact_id"),
                _id("resolution_fact_id"),
                _id("binding_fact_id"),
                _id("parameter_fact_id"),
                _id("scope_fact_id"),
                _id("module_node_id"),
                _int("start_byte"),
                _int("end_byte"),
                _int("model_proof_count"),
                pa.field("model_proof_digest", pa.binary(32), nullable=False),
            ]
        ),
        "source_parameter_identities": pa.schema(
            [
                _id("identity_id"),
                _id("function_node_id"),
                _id("parameter_node_id"),
                _id("source_flow_fact_id"),
                _id("source_origin_id"),
                _id("condition_id"),
                _id("return_site_fact_id"),
                _id("expression_fact_id"),
                _id("reference_fact_id"),
                _id("resolution_fact_id"),
                _id("binding_fact_id"),
                _id("parameter_fact_id"),
                _id("module_node_id"),
                _int("start_byte"),
                _int("end_byte"),
            ]
        ),
        "return_completion_certificates": pa.schema(
            [
                _id("certificate_id"),
                _id("function_node_id"),
                _id("return_site_fact_id"),
                _id("entry_condition_id"),
                _id("exit_condition_id"),
                _int("entry_count"),
                _int("exit_count"),
                pa.field("entry_digest", pa.binary(32), nullable=False),
                pa.field("exit_digest", pa.binary(32), nullable=False),
            ]
        ),
        "model_context_protocols": pa.schema(
            [
                _id("model_id"),
                _int("revision"),
                _id("class_node_id"),
                _id("class_fact_id"),
                _id("class_module_fact_id"),
                _id("allocation_node_id"),
                _id("allocation_fact_id"),
                _id("allocation_module_fact_id"),
                _id("initialization_node_id"),
                _id("initialization_fact_id"),
                _id("initialization_module_fact_id"),
                _utf8("entry"),
                _utf8("entry_formal", True),
                _utf8("exit"),
                _utf8("exception_formal", True),
                _utf8("origin"),
            ]
        ),
        "source_context_sites": pa.schema(
            [
                _id("site_id"),
                _id("function_node_id"),
                _id("with_node_id"),
                _id("with_fact_id"),
                _id("item_node_id"),
                _id("item_fact_id"),
                _int("item_ordinal"),
                _id("call_node_id"),
                _id("call_fact_id"),
                _id("expression_fact_id"),
                _id("protocol_id"),
                _id("model_id"),
                _id("class_node_id"),
                _id("reference_fact_id"),
                _id("resolution_fact_id"),
                _id("import_binding_fact_id"),
                _id("import_region_fact_id"),
                _id("import_condition_id"),
                _id("export_fact_id"),
                _id("allocation_call_fact_id"),
                _id("initialization_call_fact_id"),
                pa.field("constructor_valid", pa.bool_(), nullable=False),
                _id("entry_argument_fact_id", True),
            ]
        ),
        "source_context_arguments": pa.schema(
            [
                _id("site_id"),
                _int("ordinal"),
                _id("argument_fact_id"),
                _id("expression_fact_id"),
                _id("parameter_fact_id", True),
                _id("exception_class_node_id", True),
                _id("exception_class_fact_id", True),
                _id("exception_module_fact_id", True),
                _id("reference_fact_id", True),
                _id("resolution_fact_id", True),
            ]
        ),
        "summary_flow_steps": pa.schema(
            [
                _id("summary_id"),
                _int("ordinal"),
                _utf8("kind"),
                _id("evidence_id"),
                _id("condition_id"),
            ]
        ),
        "summary_boundaries": pa.schema(
            [
                _id("function_node_id"),
                _id("parameter_node_id"),
                _id("source_flow_fact_id"),
                _id("source_origin_id"),
                _id("condition_id"),
                _utf8("reason"),
                pa.field("local_through_call", pa.bool_(), nullable=False),
                pa.field("upstream_through_call", pa.bool_(), nullable=False),
                pa.field("raw_approximated", pa.bool_(), nullable=False),
            ]
        ),
        "behavior_discharges": pa.schema(
            [
                _id("behavior_id"),
                _id("origin_id"),
                _utf8("proof_kind"),
                _utf8("decision"),
                _id("summary_id", True),
                _utf8("reason", True),
            ]
        ),
        "flow_test_leaves": pa.schema(
            [
                _id("fact_id"),
                _id("module_node_id"),
                _id("condition_id"),
                _id("atom_id"),
                _utf8("atom"),
                _utf8("path", True),
                _int("leaf_start_byte"),
                _int("leaf_end_byte"),
            ]
        ),
        "flow_test_value_links": pa.schema(
            [
                _id("link_id"),
                _id("operation_node_id"),
                _id("formal_node_id"),
                _id("module_node_id"),
                _id("leaf_fact_id"),
                _id("atom_id"),
                _id("condition_id"),
                _utf8("place"),
                _utf8("origin"),
                pa.field("effect_model_digest", pa.binary(32), nullable=False),
                _id("use_id"),
                _id("use_fact_id"),
                _id("reaching_fact_id"),
                _id("definition_fact_id"),
                _id("stability_origin_id", True),
                _id("stability_condition_id", True),
                _utf8("path", True),
                _int("operand_start_byte"),
                _int("operand_end_byte"),
            ]
        ),
        # FORMAT 5 (Stage 2): singletons, their fields' reads, and field and setting claims.
        "singletons": pa.schema([_utf8("global"), _id("class_node_id")]),
        "ambient_reads": pa.schema(
            [
                _utf8("global"),
                _utf8("field"),
                _id("reader_node_id", True),
                _utf8("reader", True),
                _utf8("phase"),
                _utf8("path", True),
                _int("line"),
                _int("start_byte"),
                _utf8("spelled"),
                _utf8("condition", True),
            ]
        ),
        "place_claims": pa.schema(
            [
                _utf8("place_key"),
                _utf8("kind"),
                _id("subject_node_id", True),
                pa.field("holds", pa.bool_(), nullable=False),
                _utf8("boundary_reason", True),
                _utf8("reason", True),
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
    # FORMAT 5: a singleton global's class, its fields' reads, and place claims by key.
    singletons: dict[str, bytes] = field(default_factory=dict)
    ambient: dict[str, list[dict]] = field(default_factory=dict)
    claims: dict[str, dict] = field(default_factory=dict)
    op_vectors: dict[str, tuple[np.ndarray, list[bytes]]] = field(default_factory=dict)
    condition_graph: SemanticExecutor | None = None

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
    if native_ipc is not None and name in NATIVE_IPC_FILES:
        native_ipc[name] = data
    return table


def _validate_support_closure(tables: dict[str, pa.Table]) -> None:
    """Refuse a generation whose served support edges cannot resolve in its own projection."""
    for name in (
        "support_findings",
        "support_witnesses",
        "support_members",
        "support_attributes",
        "support_attribute_incidences",
    ):
        if tables[name].num_rows > 100_000:
            raise GenerationError(f"{name} exceeds the support projection row limit")
    findings = {row["finding_id"]: row for row in tables["support_findings"].to_pylist()}
    if len(findings) != tables["support_findings"].num_rows:
        raise GenerationError("duplicate support finding")
    evidence = {row["evidence_id"] for row in tables["evidence"].to_pylist()}
    cited: set[bytes] = set()
    for support in tables["supports"].to_pylist():
        finding_id = support["finding_id"]
        if finding_id is not None:
            cited.add(finding_id)
            finding = findings.get(finding_id)
            if finding is None or finding["finding_kind"] != support["finding_kind"]:
                raise GenerationError("assertion support has no matching finding closure")
        evidence_id = support["evidence_id"]
        if evidence_id is not None and evidence_id not in evidence:
            raise GenerationError("assertion support has no matching evidence")
    if cited != findings.keys():
        raise GenerationError("finding closure differs from served assertion supports")
    for witness in tables["support_witnesses"].to_pylist():
        if witness["finding_id"] not in findings:
            raise GenerationError("witness has no served finding")
        if any(witness[name] is None for name in ("source_path", "start_byte", "end_byte")):
            raise GenerationError("witness source span is unavailable")
    for member in tables["support_members"].to_pylist():
        if member["finding_id"] not in findings:
            raise GenerationError("member has no served finding")
        if member["cited_fact_id"] is not None and member["fact_table"] is None:
            raise GenerationError("member cited fact is unavailable")

    attributes = {r["attribute_id"]: r for r in tables["support_attributes"].to_pylist()}
    if len(attributes) != tables["support_attributes"].num_rows:
        raise GenerationError("duplicate support attribute")
    referenced = set()
    members_by_finding: dict[bytes, list[dict]] = {}
    required = set()
    for member in tables["support_members"].to_pylist():
        members_by_finding.setdefault(member["finding_id"], []).append(member)
        attribute = member["attribute_id"]
        if attribute is not None:
            referenced.add(attribute)
            if attribute not in attributes:
                raise GenerationError("member attribute is unavailable")
    if referenced != attributes.keys():
        raise GenerationError("attribute closure differs from served members")
    handoff_pairs = {}
    handoff_formals = {}
    for finding, members in members_by_finding.items():
        if findings[finding]["finding_kind"] == "handoff":
            by_ordinal = {m["ordinal"]: m for m in members}
            count = len(members)
            if count < 4 or count % 2 or set(by_ordinal) != set(range(count)):
                raise GenerationError("handoff members have an invalid order")
            for ordinal, member in by_ordinal.items():
                role = (
                    "formal"
                    if ordinal == 0
                    else "handoff_attribute"
                    if ordinal == count - 1
                    else "producer_site"
                    if ordinal % 2
                    else "consumer_site"
                )
                if member["role"] != role or (role == "handoff_attribute") != (
                    member["attribute_id"] is not None
                ):
                    raise GenerationError("handoff members have an invalid role")
                if role != "handoff_attribute" and member["node_id"] is None:
                    raise GenerationError("handoff member has no source node")
        objects = {m["node_id"] for m in members if m["role"] == "extent_member"}
        attrs = {
            m["attribute_id"]
            for m in members
            if m["attribute_id"] is not None and m["role"] != "handoff_attribute"
        }
        if len(required) + len(objects) * len(attrs) > 100_000:
            raise GenerationError("attribute supporter closure exceeds the support budget")
        required.update((finding, obj, attr) for obj in objects for attr in attrs)
        for member in members:
            if member["role"] == "handoff_attribute":
                required.add(
                    (finding, findings[finding]["subject_node_id"], member["attribute_id"])
                )
        consumers = {m["ordinal"]: m["node_id"] for m in members if m["role"] == "consumer_site"}
        pairs = {
            (m["node_id"], consumers[m["ordinal"] + 1])
            for m in members
            if m["role"] == "producer_site" and m["ordinal"] + 1 in consumers
        }
        if any(m["role"] == "handoff_attribute" for m in members):
            if not pairs:
                raise GenerationError("handoff attribute has no retained pair")
            handoff_pairs[finding] = pairs
            handoff_formals[finding] = {m["node_id"] for m in members if m["role"] == "formal"}
    observed = set()
    incidence_keys = set()
    observed_pairs = {}
    for row in tables["support_attribute_incidences"].to_pylist():
        key = (row["finding_id"], row["incidence_id"])
        if key in incidence_keys:
            raise GenerationError("duplicate attribute incidence")
        incidence_keys.add(key)
        scope = (row["finding_id"], row["object_node_id"], row["attribute_id"])
        if scope not in required:
            raise GenerationError("foreign attribute incidence")
        observed.add(scope)
        if any(row[n] is None for n in ("source_fact_id", "fact_table", "fact_model_id")):
            raise GenerationError("attribute source evidence is unavailable")
        kind = attributes[row["attribute_id"]]["kind"]
        call_fields = ("site_node_id", "edge_id")
        pair_fields = (
            "other_site_node_id",
            "other_edge_id",
            "consumer_formal_id",
            "other_fact_id",
            "other_fact_table",
            "other_fact_model_id",
        )
        if kind in ("hands_off", "takes_from"):
            if any(row[n] is None for n in (*call_fields, *pair_fields)):
                raise GenerationError("paired attribute evidence is unavailable")
        elif kind == "calls":
            if any(row[n] is None for n in call_fields) or any(
                row[n] is not None for n in pair_fields
            ):
                raise GenerationError("call attribute evidence has an invalid shape")
        elif kind in ("parameter", "parameter_type", "returns", "raises", "decorator"):
            if any(row[n] is not None for n in (*call_fields, *pair_fields)):
                raise GenerationError("structural attribute evidence has an invalid shape")
        else:
            raise GenerationError("unknown attribute kind")
        finding = row["finding_id"]
        if finding in handoff_pairs:
            pair = (row["site_node_id"], row["other_site_node_id"])
            if (
                pair not in handoff_pairs[finding]
                or row["consumer_formal_id"] not in handoff_formals[finding]
            ):
                raise GenerationError("attribute incidence does not support retained handoff")
            observed_pairs.setdefault(finding, set()).add(pair)
    if {
        key for key, row in findings.items() if row["finding_kind"] == "handoff"
    } != handoff_pairs.keys():
        raise GenerationError("handoff has no complete retained pair")
    if observed_pairs != handoff_pairs:
        raise GenerationError("missing attribute incidence for retained handoff pair")
    if observed != required:
        raise GenerationError("missing attribute incidence for finding supporter")


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
    _validate_support_closure(tables)
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
            list(native_ipc.items()),
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
        root=root,
        manifest=manifest,
        tables=tables,
        brief_ids=brief_ids,
        briefs={r["brief_id"]: r for r in brief_rows},
        lexical=[lexical_of.get(b, "") for b in brief_ids],
        symbols=symbols,
        spec_hash=spec_hash,
        vectors=vectors,
        condition_graph=condition_graph,
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
        singletons=singletons,
        ambient=ambient,
        claims=claims,
        op_vectors=op_vectors,
    )
