//! Pure serving contracts. Physical locations, COPY batches and retrieval indexes are not content.
use crate::{bundle, embedding_spec::check_vector};
use arrow_array::{
    Array, BooleanArray, FixedSizeBinaryArray, FixedSizeListArray, Float32Array, Int64Array,
    RecordBatch, StringArray,
};
use arrow_schema::DataType;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;

pub const NATIVE_FILES: &[&str] = &[
    "conditions",
    "condition_nodes",
    "analysis_conditions",
    "analysis_condition_nodes",
    "operations",
    "public_paths",
    "callable_parameters",
    "summary_flows",
    "source_parameter_identities",
    "source_context_value_identities",
    "source_modeled_identities",
    "model_frame_exits",
    "model_frame_exit_arguments",
    "model_frame_exit_steps",
    "source_body_completions",
    "source_body_steps",
    "source_body_release_inputs",
    "source_call_bindings",
    "source_call_normals",
    "source_call_header_steps",
    "model_context_protocols",
    "source_context_sites",
    "source_context_arguments",
    "return_completion_certificates",
    "summary_flow_steps",
    "summary_boundaries",
    "behavior_discharges",
    "flow_test_leaves",
    "flow_test_value_links",
];

pub const FORMAT: u32 = 3;
pub const BUNDLE_FORMAT: u32 = 13;
pub fn artifact_names() -> Vec<String> {
    artifacts_for(&crate::catalog::Capabilities::for_profile(
        crate::catalog::CompileProfile::Behavioral,
    ))
}
pub fn artifacts_for(capabilities: &crate::catalog::Capabilities) -> Vec<String> {
    NATIVE_FILES
        .iter()
        .copied()
        .filter(|_| capabilities.native_value_paths)
        .chain(["lexical_text", "operation_text", "embedding_spec"])
        .map(|s| format!("{s}.arrow"))
        .collect()
}
pub fn definition_digest() -> String {
    // Explicit struct order is invariant under serde_json's optional preserve_order feature.
    // Preserve the compiler's original encoding while making standalone wheels agree with it.
    #[derive(Serialize)]
    struct Relation {
        name: &'static str,
        schema: String,
        key: Option<&'static str>,
        codes: Vec<(String, &'static str)>,
        vocabularies: Vec<(String, Vec<&'static str>)>,
    }
    #[derive(Serialize)]
    struct Definition {
        format: u32,
        validation_version: u32,
        relations: Vec<Relation>,
        links: Vec<(&'static str, &'static str, &'static str, &'static str)>,
        native_files: &'static [&'static str],
        artifacts: Vec<String>,
        kernel_format: u32,
    }
    let relations = bundle::files(1024)
        .into_iter()
        .map(|file| Relation {
            name: file.name,
            schema: bundle::schema_digest(&file.schema).expect("declared serving schema"),
            key: unique_key(file.name),
            vocabularies: file
                .schema
                .fields()
                .iter()
                .filter_map(|f| {
                    crate::catalog::vocabulary(file.name, f.name()).map(|v| (f.name().clone(), v))
                })
                .collect(),
            codes: file
                .schema
                .fields()
                .iter()
                .filter_map(|f| codebook_name(file.name, f.name()).map(|c| (f.name().clone(), c)))
                .collect(),
        })
        .collect();
    let definition = Definition {
        format: FORMAT,
        validation_version: 1,
        relations,
        links: foreign_keys(),
        native_files: NATIVE_FILES,
        artifacts: artifact_names(),
        kernel_format: crate::condition_kernel::KERNEL_FORMAT,
    };
    hex(Sha256::digest(
        serde_json::to_vec(&definition).expect("declared projection definition"),
    ))
}
pub const MAX_RELATION_ROWS: usize = 200_000;
pub const MAX_RELATION_BYTES: usize = 128 * 1024 * 1024;

/// Stable error categories cross the service boundary; underlying SQL and credentials do not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Unavailable,
    Incomplete,
    Corrupt,
    Incompatible,
    ResourceRefused,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionError {
    pub kind: FailureKind,
    pub message: String,
}
impl std::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}
impl std::error::Error for ProjectionError {}
pub fn corrupt(message: impl Into<String>) -> ProjectionError {
    ProjectionError {
        kind: FailureKind::Corrupt,
        message: message.into(),
    }
}
pub fn refused(message: impl Into<String>) -> ProjectionError {
    ProjectionError {
        kind: FailureKind::ResourceRefused,
        message: message.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationReceipt {
    pub schema_digest: String,
    pub rows: u64,
    pub content_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReceipt {
    pub sha256: String,
    pub bytes: u64,
    pub format: u32,
}

/// Physical relations stay typed in every generation; unselected enrichment projects no rows.
pub fn relation_requested(capabilities: &crate::catalog::Capabilities, name: &str) -> bool {
    !((!capabilities.native_value_paths
        && NATIVE_FILES.contains(&name)
        && !matches!(name, "operations" | "public_paths"))
        || (!capabilities.behavioral_claims
            && matches!(
                name,
                "behaviors" | "singletons" | "ambient_reads" | "place_claims"
            ))
        || (!capabilities.briefs
            && matches!(
                name,
                "briefs" | "assertions" | "supports" | "lexical_text" | "vectors"
            )))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServingContext {
    pub library: String,
    pub requirement: String,
    pub summary: CoverageSummary,
}

/// Context rendered by MCP, derived once from the canonical published snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageSummary {
    pub coverage: BTreeMap<String, BTreeMap<String, BTreeMap<String, u64>>>,
    pub boundaries: BTreeMap<String, u64>,
    pub invocations: BTreeMap<String, u64>,
    pub briefs: BTreeMap<String, BTreeMap<String, u64>>,
    pub unresolved_slots: BTreeMap<String, u64>,
    pub absent_slots: BTreeMap<String, u64>,
    pub slot_sections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub capabilities: crate::catalog::Capabilities,
    pub format: u32,
    pub bundle_format: u32,
    pub context: ServingContext,
    pub snapshot_id: String,
    pub snapshot_digest: String,
    pub compiler_digest: String,
    pub projection_digest: String,
    pub catalog_digest: Option<String>,
    pub kernel_format: Option<u32>,
    pub entry_value_effect_digest: Option<String>,
    pub spec_hash: Option<String>,
    pub dimensions: i32,
    pub relations: BTreeMap<String, RelationReceipt>,
    pub artifacts: BTreeMap<String, ArtifactReceipt>,
}
fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
fn is_hex(text: &str, width: usize) -> bool {
    text.len() == width * 2
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Manifest {
    /// Bind the projection to the canonical provenance recorded by the file publisher.
    /// Database readers instead obtain this manifest from their pinned generation handle.
    pub fn validate_envelope(&self, envelope: &serde_json::Value) -> Result<(), ProjectionError> {
        if envelope["library"].as_str() != Some(&self.context.library)
            || envelope["requirement"].as_str() != Some(&self.context.requirement)
            || envelope["summary"]
                != serde_json::to_value(&self.context.summary)
                    .map_err(|_| corrupt("context encoding"))?
        {
            return Err(corrupt("projection serving context mismatch"));
        }
        for (field, expected) in [
            ("snapshot_id", self.snapshot_id.as_str()),
            ("content_digest", self.snapshot_digest.as_str()),
            ("compiler_digest", self.compiler_digest.as_str()),
        ] {
            if envelope[field].as_str() != Some(expected) {
                return Err(corrupt(format!("projection envelope mismatch: {field}")));
            }
        }
        if envelope["format"].as_u64() != Some(self.bundle_format.into())
            || envelope["condition_kernel_format"] != serde_json::json!(self.kernel_format)
            || envelope["entry_value_effect_digest"]
                != serde_json::json!(self.entry_value_effect_digest)
            || envelope["capabilities"] != serde_json::json!(self.capabilities)
            || envelope.get("spec_hash")
                != Some(&serde_json::to_value(&self.spec_hash).expect("spec hash"))
        {
            return Err(corrupt("projection envelope format/spec mismatch"));
        }
        for (name, receipt) in &self.relations {
            let entry = &envelope["files"][name];
            if entry["schema_digest"].as_str() != Some(&receipt.schema_digest)
                || entry["rows"].as_u64() != Some(receipt.rows)
            {
                return Err(corrupt(format!(
                    "projection envelope relation metadata mismatch: {name}"
                )));
            }
        }
        Ok(())
    }

    /// Validate content and bind its embedding identity to this manifest.
    pub fn validate_relations(
        &self,
        tables: &BTreeMap<String, Vec<RecordBatch>>,
    ) -> Result<(), ProjectionError> {
        self.validate()?;
        for (name, batches) in tables {
            if self.relations.get(name) != Some(&receipt(name, self.dimensions, batches)?) {
                return Err(corrupt("projection relation content drift"));
            }
        }
        validate_relations(self.dimensions, tables)?;
        for batch in &tables["embedding_spec"] {
            let hashes = batch
                .column_by_name("spec_hash")
                .unwrap()
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap();
            for row in 0..batch.num_rows() {
                if self.spec_hash.as_deref() != Some(hex(hashes.value(row)).as_str()) {
                    return Err(corrupt("projection embedding specification mismatch"));
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ProjectionError> {
        for (name, receipt) in &self.relations {
            if !relation_requested(&self.capabilities, name) && receipt.rows != 0 {
                return Err(corrupt(format!("unadvertised capability has rows: {name}")));
            }
        }
        if self.format != FORMAT
            || self.bundle_format != BUNDLE_FORMAT
            || !matches!(self.dimensions, 0 | 1024)
        {
            return Err(ProjectionError {
                kind: FailureKind::Incompatible,
                message: "unsupported projection/spec format".into(),
            });
        }
        if self.context.library.is_empty() {
            return Err(corrupt("missing serving release context"));
        }
        if !self.capabilities.valid() {
            return Err(corrupt("invalid capability dependencies"));
        }
        if self.capabilities.native_value_paths {
            if self.catalog_digest.as_deref()
                != Some(crate::models::Catalog::committed_digest().hex().as_str())
                || self.kernel_format != Some(crate::condition_kernel::KERNEL_FORMAT)
                || self
                    .entry_value_effect_digest
                    .as_ref()
                    .is_none_or(|d| !is_hex(d, 32))
            {
                return Err(corrupt(
                    "missing or incompatible native capability identity",
                ));
            }
        } else if self.catalog_digest.is_some()
            || self.kernel_format.is_some()
            || self.entry_value_effect_digest.is_some()
        {
            return Err(corrupt(
                "unselected native capability advertises model identities",
            ));
        }
        if self.projection_digest != definition_digest() {
            return Err(ProjectionError {
                kind: FailureKind::Incompatible,
                message: "incompatible projection definition or catalog".into(),
            });
        }
        if !is_hex(&self.snapshot_id, 16)
            || [
                &self.snapshot_digest,
                &self.compiler_digest,
                &self.projection_digest,
            ]
            .iter()
            .any(|s| !is_hex(s, 32))
            || self.spec_hash.as_ref().is_some_and(|s| !is_hex(s, 32))
            || self.spec_hash.is_some() != (self.dimensions > 0)
        {
            return Err(corrupt("invalid manifest identity"));
        }
        let files = bundle::files(self.dimensions);
        if files.len() != self.relations.len() {
            return Err(corrupt("incomplete relation inventory"));
        }
        for file in files {
            let r = self
                .relations
                .get(file.name)
                .ok_or_else(|| corrupt("missing relation"))?;
            if r.schema_digest != bundle::schema_digest(&file.schema).map_err(corrupt)?
                || !is_hex(&r.content_digest, 32)
            {
                return Err(corrupt("relation schema/content identity"));
            }
            if r.rows > MAX_RELATION_ROWS as u64 {
                return Err(refused("relation row budget"));
            }
        }
        if self
            .artifacts
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            != artifacts_for(&self.capabilities).into_iter().collect()
        {
            return Err(corrupt("incomplete native/lexical artifact inventory"));
        }
        for (name, a) in &self.artifacts {
            if name.is_empty()
                || name.contains('/')
                || name.contains("..")
                || !is_hex(&a.sha256, 32)
                || a.format != 1
            {
                return Err(corrupt("artifact identity"));
            }
            if a.bytes > MAX_RELATION_BYTES as u64
                || (NATIVE_FILES
                    .iter()
                    .any(|native| format!("{native}.arrow") == *name)
                    && a.bytes > 64 * 1024 * 1024)
            {
                return Err(refused("artifact byte budget"));
            }
        }
        Ok(())
    }
    pub fn verify_artifact(&self, name: &str, bytes: &[u8]) -> Result<(), ProjectionError> {
        if let Some(expected) = self.artifacts.get(name)
            && (expected.bytes != bytes.len() as u64
                || expected.sha256 != hex(Sha256::digest(bytes)))
        {
            return Err(corrupt("artifact content identity"));
        }
        Ok(())
    }
    /// Full 256-bit immutable generation key; no state, timing, location or retrieval profile.
    pub fn generation(&self) -> Result<String, ProjectionError> {
        self.validate()?;
        Ok(hex(Sha256::digest(
            serde_json::to_vec(self).map_err(|_| corrupt("manifest encoding"))?,
        )))
    }
}

/// Text codebooks map from the existing domain registry, never PostgreSQL catalog inference.
pub fn codebook_name(relation: &str, field: &str) -> Option<&'static str> {
    Some(match (relation, field) {
        ("public_paths" | "operations", "kind") => "declaration_kind",
        ("operations", "behavior_status") => "verdict",
        ("behaviors" | "ambient_reads", "phase") => "read_phase",
        ("behaviors", "kind") => "behavior_kind",
        ("behaviors", "transfer") => "flow_transfer",
        ("support_attributes", "parameter_kind") => "parameter_kind",
        ("source_call_normals", "body_kind") => "completion_kind",
        ("source_call_header_steps" | "model_frame_exit_steps" | "summary_flow_steps", "kind") => {
            "summary_flow_step_kind"
        }
        ("model_frame_exit_steps", "status") => "modeled_argument_evaluation_status",
        ("model_frame_exit_arguments", "safety") => "release_safety",
        ("summary_flows", "kind") => "summary_flow_kind",
        ("summary_boundaries" | "behavior_discharges", "reason") => "boundary_reason",
        ("behavior_discharges", "proof_kind") => "discharge_proof_kind",
        ("behavior_discharges", "decision") => "discharge_decision",
        ("flow_test_value_links", "origin") => "test_value_link_origin",
        ("operations" | "behaviors" | "summary_flows", "boundary_reason") => "boundary_reason",
        ("briefs", "review_state") => "review_state",
        ("briefs", "outcome_status") | ("assertions", "status") | (_, "evidence_status") => {
            "evidence_status"
        }
        ("assertions", "kind") => "assertion_kind",
        ("supports", "role") => "support_role",
        (_, "finding_kind") => "finding_kind",
        ("support_findings", "method") => "analytic_method",
        ("support_findings", "completion") => "coverage_status",
        (_, "stop_reason") => "stop_reason",
        ("support_members", "role") => "member_role",
        ("support_attributes", "kind") => "concept_attribute_kind",
        ("evidence", "kind") => "evidence_kind",
        (_, "verdict") => "verdict",
        (_, "embedding_view") => "embedding_view",
        (_, "arc_kind") => "arc_kind",
        (_, "modality" | "producer_modality") => "modality",
        (_, "phase" | "producer_phase") => "invocation_phase",
        ("operation_facets" | "operation_facet_status", "facet") => "operation_facet",
        _ => return None,
    })
}

/// Logical Arrow buffer bytes, independent of shared IPC allocations and slices.
/// RecordBatch::get_array_memory_size may count one IPC buffer once per column.
pub fn batch_bytes(batch: &RecordBatch) -> Result<usize, ProjectionError> {
    batch.columns().iter().try_fold(0usize, |total, array| {
        let bytes = array
            .to_data()
            .get_slice_memory_size()
            .map_err(|_| corrupt("unsupported Arrow memory layout"))?;
        total
            .checked_add(bytes)
            .ok_or_else(|| refused("relation byte budget"))
    })
}

/// Reject drift before any cast. PG reconstruction supplies declared metadata, not driver guesses.
pub fn validate_batch(
    name: &str,
    dimensions: i32,
    batch: &RecordBatch,
) -> Result<(), ProjectionError> {
    let expected = bundle::files(dimensions)
        .into_iter()
        .find(|f| f.name == name)
        .ok_or_else(|| corrupt("unknown relation"))?;
    if batch.schema() != expected.schema {
        return Err(corrupt(format!("{name}: schema/metadata drift")));
    }
    if batch.num_rows() > MAX_RELATION_ROWS || batch_bytes(batch)? > MAX_RELATION_BYTES {
        return Err(refused(format!("{name}: relation budget")));
    }
    let codebooks = crate::codebook::registry();
    for (field, array) in batch.schema().fields().iter().zip(batch.columns()) {
        if !field.is_nullable() && array.null_count() != 0 {
            return Err(corrupt(format!("{name}: null {}", field.name())));
        }
        if let Some(values) = crate::catalog::vocabulary(name, field.name()) {
            let a = array
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| corrupt("catalog code type"))?;
            if a.iter().flatten().any(|v| !values.contains(&v)) {
                return Err(corrupt(format!("{name}: unknown {}", field.name())));
            }
        }
        if let Some(book) = codebook_name(name, field.name()) {
            let book = codebooks
                .iter()
                .find(|b| b.name == book)
                .ok_or_else(|| corrupt("unknown declared codebook"))?;
            let a = array
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| corrupt("codebook type"))?;
            if a.iter()
                .flatten()
                .any(|s| !book.values.iter().any(|(_, v)| *v == s))
            {
                return Err(corrupt(format!("{name}: unknown {} code", field.name())));
            }
        }
        if matches!(field.data_type(), DataType::FixedSizeList(..)) {
            let list = array
                .as_any()
                .downcast_ref::<FixedSizeListArray>()
                .ok_or_else(|| corrupt("vector type"))?;
            for row in 0..list.len() {
                if list.is_null(row) {
                    continue;
                }
                let values = list.value(row);
                let values = values
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .ok_or_else(|| corrupt("vector scalar type"))?;
                if values.null_count() != 0 {
                    return Err(corrupt("null vector element"));
                }
                check_vector(values.values(), dimensions as u32).map_err(corrupt)?;
            }
        }
    }
    Ok(())
}
fn frame(h: &mut Sha256, bytes: &[u8]) {
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}
/// Logical multiset digest: independent of row order, slicing, COPY batch size and null buffers.
/// Hashes preserve null vs empty, signed zero, every field, and repeated equal rows.
pub fn receipt(
    name: &str,
    dimensions: i32,
    batches: &[RecordBatch],
) -> Result<RelationReceipt, ProjectionError> {
    let file = bundle::files(dimensions)
        .into_iter()
        .find(|f| f.name == name)
        .ok_or_else(|| corrupt("unknown relation"))?;
    let mut hashes = Vec::<[u8; 32]>::new();
    let mut bytes = 0usize;
    for batch in batches {
        validate_batch(name, dimensions, batch)?;
        bytes = bytes
            .checked_add(batch_bytes(batch)?)
            .ok_or_else(|| refused("byte budget"))?;
        if hashes.len() + batch.num_rows() > MAX_RELATION_ROWS || bytes > MAX_RELATION_BYTES {
            return Err(refused("relation budget"));
        }
        for row in 0..batch.num_rows() {
            let mut h = Sha256::new();
            h.update(b"lctx-serving-row-v1");
            for array in batch.columns() {
                if array.is_null(row) {
                    h.update([0]);
                    continue;
                }
                h.update([1]);
                match array.data_type() {
                    DataType::FixedSizeBinary(_) => frame(
                        &mut h,
                        array
                            .as_any()
                            .downcast_ref::<FixedSizeBinaryArray>()
                            .unwrap()
                            .value(row),
                    ),
                    DataType::Utf8 => frame(
                        &mut h,
                        array
                            .as_any()
                            .downcast_ref::<StringArray>()
                            .unwrap()
                            .value(row)
                            .as_bytes(),
                    ),
                    DataType::Int64 => h.update(
                        array
                            .as_any()
                            .downcast_ref::<Int64Array>()
                            .unwrap()
                            .value(row)
                            .to_le_bytes(),
                    ),
                    DataType::Boolean => h.update([u8::from(
                        array
                            .as_any()
                            .downcast_ref::<BooleanArray>()
                            .unwrap()
                            .value(row),
                    )]),
                    DataType::FixedSizeList(..) => {
                        let a = array
                            .as_any()
                            .downcast_ref::<FixedSizeListArray>()
                            .unwrap()
                            .value(row);
                        let a = a.as_any().downcast_ref::<Float32Array>().unwrap();
                        for v in a.values() {
                            h.update(v.to_bits().to_le_bytes());
                        }
                    }
                    _ => return Err(corrupt("unsupported serving type")),
                }
            }
            hashes.push(h.finalize().into());
        }
    }
    hashes.sort_unstable();
    let schema_digest = bundle::schema_digest(&file.schema).map_err(corrupt)?;
    let mut h = Sha256::new();
    h.update(b"lctx-serving-relation-v1");
    frame(&mut h, name.as_bytes());
    frame(&mut h, schema_digest.as_bytes());
    h.update((hashes.len() as u64).to_le_bytes());
    for row in &hashes {
        h.update(row);
    }
    Ok(RelationReceipt {
        schema_digest,
        rows: hashes.len() as u64,
        content_digest: hex(h.finalize()),
    })
}

pub fn unique_key(name: &str) -> Option<&'static str> {
    Some(match name {
        "catalog_members" => "member_id",
        "catalog_constructors" => "class_node_id,signature_id",
        "catalog_bindings" => "member_id,binding_id",
        "catalog_signatures" => "signature_id",
        "catalog_parameters" => "signature_id,ordinal",
        "catalog_evidence" => "evidence_id",
        "catalog_types" => "term_id",
        "catalog_type_args" => "parent_term_id,role,ordinal",
        "catalog_type_observations" => "source_fact_id",
        "briefs" => "brief_id",
        "behaviors" => "behavior_id",
        "assertions" => "brief_id,ordinal",
        "operations" => "node_id",
        "support_findings" => "finding_id",
        "support_attributes" => "attribute_id",
        "evidence" => "evidence_id",
        "public_paths" => "access_path",
        "lexical_text" => "brief_id",
        "operation_text" => "node_id",
        "operation_facet_status" => "node_id,facet",
        "vectors" => "brief_id,chunk",
        "operation_vectors" => "node_id,embedding_view,chunk",
        "embedding_spec" => "spec_hash",
        "support_attribute_incidences" => "finding_id,incidence_id",
        "support_witnesses" => "finding_id,path,step",
        "support_members" => "finding_id,role,ordinal",
        _ => return None,
    })
}
pub fn foreign_keys() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "catalog_members",
            "operation_node_id",
            "operations",
            "node_id",
        ),
        (
            "catalog_bindings",
            "member_id",
            "catalog_members",
            "member_id",
        ),
        (
            "catalog_constructors",
            "signature_id",
            "catalog_signatures",
            "signature_id",
        ),
        (
            "catalog_parameters",
            "signature_id",
            "catalog_signatures",
            "signature_id",
        ),
        (
            "catalog_type_observations",
            "term_id",
            "catalog_types",
            "term_id",
        ),
        (
            "catalog_type_args",
            "parent_term_id",
            "catalog_types",
            "term_id",
        ),
        (
            "catalog_type_args",
            "child_term_id",
            "catalog_types",
            "term_id",
        ),
        ("assertions", "brief_id", "briefs", "brief_id"),
        ("brief_members", "brief_id", "briefs", "brief_id"),
        ("symbol_map", "brief_id", "briefs", "brief_id"),
        ("lexical_text", "brief_id", "briefs", "brief_id"),
        ("vectors", "brief_id", "briefs", "brief_id"),
        ("supports", "finding_id", "support_findings", "finding_id"),
        ("supports", "evidence_id", "evidence", "evidence_id"),
        (
            "support_witnesses",
            "finding_id",
            "support_findings",
            "finding_id",
        ),
        (
            "support_members",
            "finding_id",
            "support_findings",
            "finding_id",
        ),
        (
            "support_members",
            "attribute_id",
            "support_attributes",
            "attribute_id",
        ),
        (
            "support_attribute_incidences",
            "finding_id",
            "support_findings",
            "finding_id",
        ),
        (
            "support_attribute_incidences",
            "attribute_id",
            "support_attributes",
            "attribute_id",
        ),
        ("public_paths", "node_id", "operations", "node_id"),
        ("operation_facets", "node_id", "operations", "node_id"),
        ("operation_facet_status", "node_id", "operations", "node_id"),
        ("operation_text", "node_id", "operations", "node_id"),
        ("operation_vectors", "node_id", "operations", "node_id"),
        ("behaviors", "operation_node_id", "operations", "node_id"),
        ("operations", "brief_id", "briefs", "brief_id"),
    ]
}

/// Generation-local keys and references are checked independently of backend constraints.
/// This includes supports of assertions shared by several briefs (assertion_id is not unique
/// in the rendered assertion relation).
pub fn validate_relations(
    dimensions: i32,
    tables: &BTreeMap<String, Vec<RecordBatch>>,
) -> Result<(), ProjectionError> {
    use std::collections::BTreeSet;
    let files = bundle::files(dimensions);
    if tables.len() != files.len() {
        return Err(corrupt("relation inventory"));
    }
    let total = tables.values().flatten().try_fold(0usize, |n, b| {
        n.checked_add(batch_bytes(b)?)
            .ok_or_else(|| refused("projection byte budget"))
    })?;
    if total > 512 * 1024 * 1024 {
        return Err(refused("projection byte budget"));
    }
    for file in &files {
        let batches = tables
            .get(file.name)
            .ok_or_else(|| corrupt("missing relation"))?;
        for b in batches {
            validate_batch(file.name, dimensions, b)?;
        }
        let rows = batches
            .iter()
            .try_fold(0usize, |n, b| n.checked_add(b.num_rows()))
            .ok_or_else(|| refused("relation row budget"))?;
        let bytes = batches.iter().try_fold(0usize, |n, b| {
            n.checked_add(batch_bytes(b)?)
                .ok_or_else(|| refused("relation byte budget"))
        })?;
        if rows > MAX_RELATION_ROWS || bytes > MAX_RELATION_BYTES {
            return Err(refused("relation aggregate budget"));
        }
        if let Some(key) = unique_key(file.name) {
            let values = keys(batches, key)?;
            if values.iter().any(Option::is_none)
                || values.iter().collect::<BTreeSet<_>>().len() != values.len()
            {
                return Err(corrupt(format!("{}: duplicate/null domain key", file.name)));
            }
        }
    }
    let mut links = foreign_keys();
    links.push(("supports", "assertion_id", "assertions", "assertion_id"));
    for (table, field, parent, target) in links {
        let parents: BTreeSet<_> = keys(&tables[parent], target)?
            .into_iter()
            .flatten()
            .collect();
        if keys(&tables[table], field)?
            .into_iter()
            .flatten()
            .any(|k| !parents.contains(&k))
        {
            return Err(corrupt(format!(
                "{table}: missing generation-local {parent}"
            )));
        }
    }
    crate::serving_support::validate(tables)?;
    let specs = &tables["embedding_spec"];
    let count = specs.iter().map(RecordBatch::num_rows).sum::<usize>();
    if count != usize::from(dimensions > 0) {
        return Err(corrupt("embedding specification count"));
    }
    for b in specs {
        for row in 0..b.num_rows() {
            let text = b
                .column_by_name("spec")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(row);
            let spec = crate::embedding_spec::Spec::parse(text).map_err(corrupt)?;
            let hash = b
                .column_by_name("spec_hash")
                .unwrap()
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap()
                .value(row);
            if spec.dimensions as i32 != dimensions
                || hash != spec.hash().0
                || spec.canonical_json() != text
            {
                return Err(corrupt("embedding specification identity"));
            }
        }
    }
    Ok(())
}
fn keys(batches: &[RecordBatch], fields: &str) -> Result<Vec<Option<Vec<u8>>>, ProjectionError> {
    let mut result = Vec::new();
    for b in batches {
        for row in 0..b.num_rows() {
            let mut bytes = Vec::new();
            let mut null = false;
            for f in fields.split(',') {
                let a = b
                    .column_by_name(f)
                    .ok_or_else(|| corrupt("undeclared key column"))?;
                if a.is_null(row) {
                    null = true;
                    break;
                }
                let value = match a.data_type() {
                    DataType::FixedSizeBinary(_) => a
                        .as_any()
                        .downcast_ref::<FixedSizeBinaryArray>()
                        .unwrap()
                        .value(row)
                        .to_vec(),
                    DataType::Utf8 => a
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .unwrap()
                        .value(row)
                        .as_bytes()
                        .to_vec(),
                    DataType::Int64 => a
                        .as_any()
                        .downcast_ref::<Int64Array>()
                        .unwrap()
                        .value(row)
                        .to_le_bytes()
                        .to_vec(),
                    _ => return Err(corrupt("unsupported key type")),
                };
                bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
                bytes.extend_from_slice(&value);
            }
            result.push((!null).then_some(bytes));
        }
    }
    Ok(result)
}

/// Rank transport is a query result, not a persisted relation or truncated exact substitute.
pub fn rank_schema() -> arrow_schema::SchemaRef {
    use arrow_schema::{DataType, Field, Schema};
    std::sync::Arc::new(Schema::new(vec![
        Field::new("entity_id", DataType::FixedSizeBinary(16), false),
        Field::new("view", DataType::Utf8, false),
        Field::new("rank", DataType::UInt32, false),
        Field::new("score", DataType::Float64, false),
    ]))
}
