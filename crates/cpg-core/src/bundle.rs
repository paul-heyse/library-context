//! Stage G (DESIGN §6.4; ADR-0017, ADR-0019): a serving generation, read from a published
//! snapshot and normalized, so that rebuilding it from the store gives byte-identical files.
//!
//! - **Queries.** Each served file is one query over the published session, sorted by its
//!   declared key (`cpg_schema::bundle`). Codebook values are served as their text.
//! - **Normalization.** Every column is cast to its declared type and rebuilt through a builder.
//!   View types, scan metadata and the bytes under null slots never reach a file.
//! - **Files.** One record batch per file, in the Arrow IPC file format: V5 metadata, 64-byte
//!   alignment, uncompressed.
//! - **`MANIFEST.json`**, with sorted keys:
//!   - the snapshot and its content and compiler digests;
//!   - each file's sha256, rows and serving schema digest;
//!   - the spec hash;
//!   - a coverage summary.
//!
//! The generation key is the first 16 hex digits of the SHA-256 of the manifest without its key,
//! so two generations share a key only when their files and provenance are the same.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{
    BooleanBuilder, FixedSizeBinaryBuilder, FixedSizeListBuilder, Float32Builder, Int64Builder,
    StringBuilder,
};
use arrow_array::cast::AsArray;
use arrow_array::types::{Float32Type, Int64Type};
use arrow_array::{Array, ArrayRef, RecordBatch};
use arrow_cast::{CastOptions, cast_with_options};
use arrow_ipc::MetadataVersion;
use arrow_ipc::reader::FileReader;
use arrow_ipc::writer::{FileWriter, IpcWriteOptions};
use arrow_schema::{DataType, Field, FieldRef, SchemaRef};
use cpg_schema::bundle::{ServingFile, files, schema_digest};
use cpg_schema::codebook::{
    AnalyticMethod, ArcKind, AssertionKind, BehaviorKind, BoundaryReason, Codebook, CoverageStatus,
    DeclarationKind, DischargeDecision, DischargeProofKind, EmbeddingView, EvidenceKind,
    EvidenceStatus, FactFamily, FindingKind, InvocationPhase, MemberRole, Modality, OperationFacet,
    ReviewState, ScopeKind, StopReason, SummaryFlowKind, SummaryFlowStepKind, SupportRole,
    TestValueLinkOrigin, Verdict,
};
use cpg_schema::findings::{ASSERTION_POLICY, SLOT_SECTIONS};
use cpg_schema::id::Id;
use datafusion::prelude::SessionContext;
use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};

use crate::{CoreError, sql};

/// The manifest's format version: bumped when a served file, its schema or the manifest changes.
pub const FORMAT: u64 = 11;
const MAX_SUPPORT_ROWS: usize = 100_000;
const MAX_SUPPORT_FILE_BYTES: usize = 64 * 1024 * 1024;

/// A built generation: its key, directory and manifest.
#[derive(Debug, Clone)]
pub struct Generation {
    pub key: String,
    pub dir: PathBuf,
    pub manifest: Value,
}

fn bad(message: impl Into<String>) -> CoreError {
    CoreError::Bundle(message.into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// A codebook column as its text.
pub(crate) fn text_of<C: Codebook>(expr: &str) -> String {
    let arms: String = C::all()
        .iter()
        .map(|c| format!(" WHEN {} THEN '{}'", c.code(), c.text()))
        .collect();
    format!("CASE {expr}{arms} END")
}

/// An assertion kind's section, as text, from the kind policy.
fn section_of(expr: &str) -> String {
    let arms: String = ASSERTION_POLICY
        .iter()
        .map(|(k, s, _)| format!(" WHEN {} THEN '{}'", k.code(), s.text()))
        .collect();
    format!("CASE {expr}{arms} END")
}

/// Each served file's query, sorted by its key; `lexical_text` is computed after the others.
fn query(name: &str) -> Option<String> {
    Some(match name {
        "briefs" => format!(
            "SELECT b.brief_id, b.seed_node_id, b.access_path, b.title, b.applicable_case, \
                    b.documentation_only, {review} AS review_state, o.text AS outcome, \
                    COALESCE({status}, 'unresolved') AS outcome_status \
             FROM briefs b LEFT JOIN ( \
               SELECT ba.brief_id, a.text, a.evidence_status FROM brief_assertions ba \
               JOIN assertions a ON a.assertion_id = ba.assertion_id \
               WHERE a.assertion_kind = {outcome}) o ON o.brief_id = b.brief_id \
             ORDER BY b.brief_id",
            review = text_of::<ReviewState>("b.review_state"),
            status = text_of::<EvidenceStatus>("o.evidence_status"),
            outcome = AssertionKind::Outcome.code(),
        ),
        "assertions" => format!(
            "SELECT ba.brief_id, ba.ordinal, a.assertion_id, {kind} AS kind, {section} AS section, \
                    {status} AS status, a.text, a.applicable_case, a.conditions, a.limitations, \
                    a.template_version \
             FROM brief_assertions ba JOIN assertions a ON a.assertion_id = ba.assertion_id \
             ORDER BY ba.brief_id, ba.ordinal",
            kind = text_of::<AssertionKind>("a.assertion_kind"),
            section = section_of("a.assertion_kind"),
            status = text_of::<EvidenceStatus>("a.evidence_status"),
        ),
        "supports" => format!(
            "SELECT s.assertion_id, {role} AS role, s.ordinal, s.finding_id, \
                    {kind} AS finding_kind, s.evidence_id \
             FROM assertion_support s LEFT JOIN findings f ON f.finding_id = s.finding_id \
             ORDER BY s.assertion_id, role, s.ordinal",
            role = text_of::<SupportRole>("s.role"),
            kind = text_of::<FindingKind>("f.finding_kind"),
        ),
        "support_findings" => format!(
            "WITH cited AS (SELECT DISTINCT s.finding_id FROM assertion_support s \
                JOIN brief_assertions ba ON ba.assertion_id = s.assertion_id \
                WHERE s.finding_id IS NOT NULL) \
             SELECT f.finding_id, {kind} AS finding_kind, {status} AS evidence_status, \
                    f.subject_node_id, f.related_node_id, f.invocation_id, \
                    i.model_id, {method} AS method, i.parameters, \
                    {completion} AS completion, {stop} AS stop_reason, f.witnesses_omitted \
             FROM cited c JOIN findings f ON f.finding_id = c.finding_id \
             JOIN analysis_invocations i ON i.invocation_id = f.invocation_id \
             ORDER BY f.finding_id LIMIT {limit}",
            kind = text_of::<FindingKind>("f.finding_kind"),
            status = text_of::<EvidenceStatus>("f.evidence_status"),
            method = text_of::<AnalyticMethod>("i.method"),
            completion = text_of::<CoverageStatus>("i.completion"),
            stop = text_of::<StopReason>("i.stop_reason"),
            limit = MAX_SUPPORT_ROWS + 1,
        ),
        "support_witnesses" => format!(
            "WITH cited AS (SELECT DISTINCT s.finding_id FROM assertion_support s \
                JOIN brief_assertions ba ON ba.assertion_id = s.assertion_id \
                WHERE s.finding_id IS NOT NULL), \
             declarations_one AS (SELECT node_id, module_node_id, start_byte, end_byte, fact_id \
                FROM (SELECT node_id, module_node_id, start_byte, end_byte, fact_id, \
                    row_number() OVER (PARTITION BY node_id ORDER BY start_byte, end_byte, fact_id) AS rn \
                    FROM declarations) WHERE rn = 1), \
             source_paths AS (SELECT module_node_id, min(path) AS path FROM ({files}) \
                GROUP BY module_node_id) \
             SELECT w.finding_id, w.path, w.step, w.caller_node_id, w.call_site_node_id, \
                    w.callee_node_id, {arc} AS arc_kind, {modality} AS modality, \
                    {phase} AS phase, COALESCE(cs.fact_id, sn.fact_id, d.fact_id) AS source_fact_id, \
                    sp.path AS source_path, COALESCE(cs.start_byte, sn.start_byte, d.start_byte) AS start_byte, \
                    COALESCE(cs.end_byte, sn.end_byte, d.end_byte) AS end_byte \
             FROM cited c JOIN witnesses w ON w.finding_id = c.finding_id \
             LEFT JOIN call_syntax cs ON cs.node_id = w.call_site_node_id \
             LEFT JOIN syntax_nodes sn ON sn.node_id = w.call_site_node_id \
             LEFT JOIN declarations_one d ON d.node_id = w.call_site_node_id \
             LEFT JOIN source_paths sp ON sp.module_node_id = COALESCE(cs.module_node_id, sn.module_node_id, d.module_node_id) \
             ORDER BY w.finding_id, w.path, w.step LIMIT {limit}",
            arc = text_of::<ArcKind>("w.arc_kind"),
            modality = text_of::<Modality>("w.modality"),
            phase = text_of::<InvocationPhase>("w.phase"),
            files = cpg_schema::flows::display_files_sql(),
            limit = MAX_SUPPORT_ROWS + 1,
        ),
        "return_completion_certificates" => format!(
            "WITH candidates AS (SELECT c.*, f.condition_id AS selected_condition, \
                count(*) OVER (PARTITION BY f.summary_id) AS candidate_count \
                FROM return_completion_certificates c JOIN summary_flows f \
                ON f.function_node_id=c.function_node_id AND f.return_site_fact_id=c.return_site_fact_id) \
             SELECT DISTINCT certificate_id,function_node_id,return_site_fact_id,entry_condition_id,exit_condition_id, \
                entry_count,exit_count,entry_digest,exit_digest FROM candidates \
             WHERE entry_condition_id=selected_condition OR candidate_count=1 ORDER BY certificate_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1
        ),
        "model_context_protocols" => format!(
            "SELECT p.model_id, p.revision, p.class_node_id, p.class_fact_id, p.class_module_fact_id, p.allocation_node_id, p.allocation_fact_id, p.allocation_module_fact_id, p.initialization_node_id, p.initialization_fact_id, p.initialization_module_fact_id, {entry} AS entry, p.entry_formal, {exit} AS exit, p.exception_formal, {origin} AS origin FROM model_context_protocols p WHERE EXISTS (SELECT 1 FROM source_context_sites site JOIN summary_flow_steps step ON step.evidence_id=site.site_id AND step.kind IN (27,28,29) WHERE site.model_id=p.model_id AND site.class_node_id=p.class_node_id) ORDER BY p.model_id, p.class_node_id LIMIT {limit}",
            entry = text_of::<cpg_schema::codebook::ContextEntryKind>("p.entry"),
            exit = text_of::<cpg_schema::codebook::ContextExitKind>("p.exit"),
            origin = text_of::<cpg_schema::codebook::Origin>("p.origin"),
            limit = MAX_SUPPORT_ROWS + 1
        ),
        "source_context_sites" => format!(
            "SELECT p.site_id, p.function_node_id, p.with_node_id, p.with_fact_id, p.item_node_id, p.item_fact_id, p.item_ordinal, p.call_node_id, p.call_fact_id, p.expression_fact_id, p.protocol_id, p.model_id, p.class_node_id, p.reference_fact_id, p.resolution_fact_id, p.import_binding_fact_id, p.import_region_fact_id, p.import_condition_id, p.export_fact_id, p.allocation_call_fact_id, p.initialization_call_fact_id, p.constructor_valid, p.entry_argument_fact_id FROM source_context_sites p WHERE EXISTS (SELECT 1 FROM summary_flow_steps step WHERE step.evidence_id=p.site_id AND step.kind IN (27,28,29)) ORDER BY p.site_id LIMIT {limit}",
            limit = MAX_SUPPORT_ROWS + 1
        ),
        "source_context_arguments" => format!(
            "SELECT p.site_id, p.ordinal, p.argument_fact_id, p.expression_fact_id, p.parameter_fact_id, p.exception_class_node_id, p.exception_class_fact_id, p.exception_module_fact_id, p.reference_fact_id, p.resolution_fact_id FROM source_context_arguments p WHERE EXISTS (SELECT 1 FROM summary_flow_steps step WHERE step.evidence_id=p.site_id AND step.kind IN (27,28,29)) ORDER BY p.site_id, p.ordinal LIMIT {limit}",
            limit = MAX_SUPPORT_ROWS + 1
        ),
        "source_context_value_identities" => format!(
            "SELECT identity_id, function_node_id, parameter_node_id, parameter_name, source_flow_fact_id, source_origin_id, condition_id, return_site_fact_id, return_region_fact_id, return_condition_id, return_start_byte, context_site_id, argument_fact_id, argument_expression_fact_id, argument_reference_fact_id, argument_resolution_fact_id, parameter_binding_fact_id, parameter_fact_id, target_binding_fact_id, expression_fact_id, reference_fact_id, resolution_fact_id, scope_fact_id, module_node_id, start_byte, end_byte FROM source_context_value_identities ORDER BY identity_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1
        ),
        "source_body_completions" => format!(
            "SELECT body_id, function_node_id, declaration_fact_id, syntax_fact_id, {kind} AS kind, terminal_fact_id, {exception} AS exception, {reason} AS reason, {release_reason} AS release_reason, function_retainer_required, runtime_statement_count, step_count, steps_digest, release_count, releases_digest, work FROM source_body_completions WHERE body_id IN (SELECT body_id FROM source_call_normals) ORDER BY body_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1,
            kind = text_of::<cpg_schema::codebook::CompletionKind>("kind"),
            exception = text_of::<cpg_schema::codebook::ExactRuntimeException>("exception"),
            reason = text_of::<cpg_schema::codebook::BoundaryReason>("reason"),
            release_reason = text_of::<cpg_schema::codebook::BoundaryReason>("release_reason")
        ),
        "source_body_steps" => format!(
            "SELECT body_id, ordinal, {kind} AS kind, evidence_id FROM source_body_steps WHERE body_id IN (SELECT body_id FROM source_call_normals) ORDER BY body_id, ordinal LIMIT {}",
            MAX_SUPPORT_ROWS + 1,
            kind = text_of::<cpg_schema::codebook::SummaryFlowStepKind>("kind")
        ),
        "source_body_release_inputs" => format!(
            "SELECT body_id, ordinal, syntax_fact_id, {safety} AS safety, proof_offset, proof_count, proof_digest, evaluation_evidence_id FROM source_body_release_inputs WHERE body_id IN (SELECT body_id FROM source_call_normals) ORDER BY body_id, ordinal LIMIT {}",
            MAX_SUPPORT_ROWS + 1,
            safety = text_of::<cpg_schema::codebook::ReleaseSafety>("safety")
        ),
        "source_call_bindings" => format!(
            "SELECT binding_id, function_node_id, call_node_id, call_fact_id, syntax_fact_id, callee_node_id, pysa_fact_id, signature_fact_id, declaration_fact_id, header_fact_id, statement_fact_id, binding_fact_id, reference_fact_id, resolution_fact_id, header_count, header_digest FROM source_call_bindings WHERE binding_id IN (SELECT binding_id FROM source_call_normals) ORDER BY binding_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1
        ),
        "source_call_normals" => format!(
            "SELECT certificate_id, binding_id, body_id, body_count, {body_kind} AS body_kind FROM source_call_normals ORDER BY certificate_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1,
            body_kind = text_of::<cpg_schema::codebook::CompletionKind>("body_kind")
        ),
        "source_call_header_steps" => format!(
            "SELECT binding_id, ordinal, {kind} AS kind, evidence_id FROM source_call_header_steps WHERE binding_id IN (SELECT binding_id FROM source_call_normals) ORDER BY binding_id, ordinal LIMIT {}",
            MAX_SUPPORT_ROWS + 1,
            kind = text_of::<cpg_schema::codebook::SummaryFlowStepKind>("kind")
        ),
        "model_frame_exits" => format!(
            "SELECT frame_exit_id, function_node_id, call_node_id, call_fact_id, syntax_fact_id, target_node_id, pysa_fact_id, model_id, return_parameter, return_argument_fact_id, argument_count, signature_count, arguments_digest, invocation_count, invocation_digest FROM model_frame_exits ORDER BY frame_exit_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1
        ),
        "model_frame_exit_arguments" => format!(
            "SELECT frame_exit_id, ordinal, argument_fact_id, expression_fact_id, {safety} AS safety, parameter_name, expression_offset, expression_count, expression_digest, parameters_digest FROM model_frame_exit_arguments ORDER BY frame_exit_id, ordinal LIMIT {}",
            MAX_SUPPORT_ROWS + 1,
            safety = text_of::<cpg_schema::codebook::ReleaseSafety>("safety")
        ),
        "model_frame_exit_steps" => format!(
            "SELECT frame_exit_id, ordinal, operand_fact_id, evidence_id, {status} AS status, {kind} AS kind FROM model_frame_exit_steps ORDER BY frame_exit_id, ordinal LIMIT {}",
            MAX_SUPPORT_ROWS + 1,
            status = text_of::<cpg_schema::codebook::ModeledArgumentEvaluationStatus>("status"),
            kind = text_of::<cpg_schema::codebook::SummaryFlowStepKind>("kind")
        ),
        "source_modeled_identities" => format!(
            "SELECT identity_id, function_node_id, parameter_node_id, source_flow_fact_id, source_origin_id, condition_id, return_site_fact_id, call_fact_id, call_expression_fact_id, source_argument_fact_id, pysa_fact_id, model_id, rule_id, callee_resolution_fact_id, expression_fact_id, reference_fact_id, resolution_fact_id, binding_fact_id, parameter_fact_id, scope_fact_id, module_node_id, start_byte, end_byte, model_proof_count, model_proof_digest FROM source_modeled_identities ORDER BY identity_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1
        ),
        "source_parameter_identities" => format!(
            "SELECT identity_id, function_node_id, parameter_node_id, source_flow_fact_id, \
             source_origin_id, condition_id, return_site_fact_id, expression_fact_id, reference_fact_id, \
             resolution_fact_id, binding_fact_id, parameter_fact_id, module_node_id, start_byte, end_byte \
             FROM source_parameter_identities ORDER BY identity_id LIMIT {}",
            MAX_SUPPORT_ROWS + 1
        ),
        "support_members" => format!(
            "WITH cited AS (SELECT DISTINCT s.finding_id FROM assertion_support s \
                JOIN brief_assertions ba ON ba.assertion_id = s.assertion_id \
                WHERE s.finding_id IS NOT NULL) \
             SELECT m.finding_id, {role} AS role, m.ordinal, m.node_id, m.cited_fact_id, m.attribute_id, \
                    f.table_name AS fact_table, f.model_id AS fact_model_id, m.label \
             FROM cited c JOIN finding_members m ON m.finding_id = c.finding_id \
             LEFT JOIN facts f ON f.fact_id = m.cited_fact_id \
             ORDER BY m.finding_id, role, m.ordinal LIMIT {limit}",
            role = text_of::<MemberRole>("m.role"),
            limit = MAX_SUPPORT_ROWS + 1,
        ),
        "support_attributes" => format!(
            "SELECT DISTINCT a.attribute_id,{kind} AS kind,a.symbol,{parameter} AS parameter_kind,a.type_term_id, \
                a.class_module,a.class_key,a.target_node_id,{modality} AS modality,{phase} AS phase, \
                {producer_modality} AS producer_modality,{producer_phase} AS producer_phase,a.display \
             FROM assertion_support s JOIN brief_assertions ba ON ba.assertion_id=s.assertion_id \
             JOIN finding_members m ON m.finding_id=s.finding_id JOIN concept_attributes a ON a.attribute_id=m.attribute_id \
             ORDER BY a.attribute_id LIMIT {limit}",
            kind = text_of::<cpg_schema::codebook::ConceptAttributeKind>("a.kind"),
            parameter = text_of::<cpg_schema::codebook::ParameterKind>("a.parameter_kind"),
            modality = text_of::<Modality>("a.modality"),
            phase = text_of::<InvocationPhase>("a.phase"),
            producer_modality = text_of::<Modality>("a.producer_modality"),
            producer_phase = text_of::<InvocationPhase>("a.producer_phase"),
            limit = MAX_SUPPORT_ROWS + 1
        ),
        "support_attribute_incidences" => format!(
            "WITH cited AS (SELECT DISTINCT s.finding_id FROM assertion_support s \
                JOIN brief_assertions ba ON ba.assertion_id=s.assertion_id WHERE s.finding_id IS NOT NULL), \
             links AS ({links}) \
             SELECT k.finding_id,i.incidence_id,i.attribute_id,i.object_node_id, \
                COALESCE(i.source_fact_id,e.evidence_fact_id) AS source_fact_id,f.table_name AS fact_table,f.model_id AS fact_model_id, \
                i.site_node_id,i.edge_id,i.other_site_node_id,i.other_edge_id,i.consumer_formal_id, \
                other.evidence_fact_id AS other_fact_id,of.table_name AS other_fact_table,of.model_id AS other_fact_model_id \
             FROM cited c JOIN links k ON k.finding_id=c.finding_id JOIN concept_incidences i ON i.incidence_id=k.incidence_id \
             LEFT JOIN edges e ON e.edge_id=i.edge_id LEFT JOIN facts f ON f.fact_id=COALESCE(i.source_fact_id,e.evidence_fact_id) \
             LEFT JOIN edges other ON other.edge_id=i.other_edge_id LEFT JOIN facts of ON of.fact_id=other.evidence_fact_id \
             ORDER BY k.finding_id,i.incidence_id LIMIT {limit}",
            links = cpg_schema::concept_attributes::finding_incidence_keys_sql(),
            limit = MAX_SUPPORT_ROWS + 1
        ),
        "evidence" => format!(
            "SELECT e.evidence_id, {kind} AS kind, e.node_id, COALESCE(sf.path, d.path) AS path, \
                    e.start_byte, e.end_byte, e.text \
             FROM evidence e \
             LEFT JOIN (SELECT module_node_id, min(path) AS path FROM ({files}) \
                        GROUP BY module_node_id) sf ON sf.module_node_id = e.module_node_id \
             LEFT JOIN (SELECT node_id, min(path) AS path FROM documents GROUP BY node_id) d \
               ON d.node_id = e.module_node_id \
             ORDER BY e.evidence_id",
            kind = text_of::<EvidenceKind>("e.evidence_kind"),
            files = cpg_schema::flows::display_files_sql(),
        ),
        // FORMAT 2 (the holistic assessment's A1): every public spelling of each brief's seed,
        // own and inherited; `own` says which the export declares.
        "brief_members" => {
            "SELECT brief_id, access_path, export_node_id, declaration_node_id, own \
                            FROM brief_members ORDER BY brief_id, access_path"
                .to_owned()
        }
        "symbol_map" => "SELECT DISTINCT access_path AS symbol, brief_id FROM brief_members \
                         ORDER BY symbol, brief_id"
            .to_owned(),
        "public_paths" => format!(
            "SELECT node_id, access_path, {kind} AS kind, own, preferred FROM public_paths \
             ORDER BY node_id, access_path",
            kind = text_of::<DeclarationKind>("kind"),
        ),
        "embedding_spec" => {
            "SELECT spec_hash, spec FROM embedding_specs ORDER BY spec_hash".to_owned()
        }
        "operations" => format!(
            "SELECT o.node_id, o.access_path, {kind} AS kind, o.is_method, o.qualified_name, \
                    o.module, o.docstring_summary, {status} AS behavior_status, \
                    {reason} AS boundary_reason, o.status_reason, b.brief_id \
             FROM operations o \
             LEFT JOIN (SELECT seed_node_id, min(brief_id) AS brief_id FROM briefs \
                        GROUP BY seed_node_id) b ON b.seed_node_id = o.node_id \
             ORDER BY o.node_id",
            kind = text_of::<DeclarationKind>("o.kind"),
            status = text_of::<Verdict>("o.behavior_status"),
            reason = text_of::<BoundaryReason>("o.boundary_reason"),
        ),
        "operation_facets" => format!(
            "SELECT node_id, {facet} AS facet, value, {verdict} AS verdict \
             FROM operation_facets ORDER BY node_id, facet, value",
            facet = text_of::<OperationFacet>("operation_facets.facet"),
            verdict = text_of::<Verdict>("operation_facets.verdict"),
        ),
        "operation_facet_status" => format!(
            "SELECT node_id, {facet} AS facet, {verdict} AS verdict, reason \
             FROM operation_facet_status ORDER BY node_id, facet",
            facet = text_of::<OperationFacet>("operation_facet_status.facet"),
            verdict = text_of::<Verdict>("operation_facet_status.verdict"),
        ),
        "behaviors" => format!(
            "SELECT b.behavior_id, b.operation_node_id, {kind} AS kind, \
                    {transfer} AS transfer, b.condition_scope_node_id, b.parameter_name, \
                    b.callee_node_id, COALESCE(o.access_path, d.qualified_name) AS callee, \
                    b.target_name, b.value, b.depth, b.conditional, {verdict} AS verdict, \
                    {reason} AS boundary_reason, b.condition, b.callee_text, {phase} AS phase, \
                    b.premise_key, b.occurrences, df.path, b.site_line AS line, b.site_text \
             FROM behaviors b \
             LEFT JOIN operations o ON o.node_id = b.callee_node_id \
             LEFT JOIN (SELECT node_id, min(qualified_name) AS qualified_name FROM declarations \
                        GROUP BY node_id) d ON d.node_id = b.callee_node_id \
             LEFT JOIN (SELECT module_node_id, min(path) AS path FROM ({files}) \
                        GROUP BY module_node_id) df ON df.module_node_id = b.site_module_node_id \
             ORDER BY b.behavior_id",
            kind = text_of::<BehaviorKind>("b.kind"),
            transfer = text_of::<cpg_schema::codebook::FlowTransfer>("b.transfer"),
            verdict = text_of::<Verdict>("b.verdict"),
            reason = text_of::<BoundaryReason>("b.boundary_reason"),
            phase = text_of::<cpg_schema::codebook::ReadPhase>("b.phase"),
            files = cpg_schema::flows::display_files_sql(),
        ),
        "conditions" => "SELECT condition_id, root_id, boundary_reason FROM conditions \
                         ORDER BY condition_id"
            .to_owned(),
        "condition_nodes" => "SELECT node_id, atom, low_id, high_id FROM condition_nodes \
                              ORDER BY node_id"
            .to_owned(),
        "analysis_conditions" => "SELECT condition_id, root_id, boundary_reason \
                                  FROM analysis_conditions ORDER BY condition_id"
            .to_owned(),
        "analysis_condition_nodes" => "SELECT node_id, atom, low_id, high_id \
                                       FROM analysis_condition_nodes ORDER BY node_id"
            .to_owned(),
        "callable_parameters" => format!(
            "SELECT d.node_id AS function_node_id, p.node_id AS formal_node_id, p.name \
             FROM declarations d JOIN parameter_syntax p ON p.function_node_id=d.node_id \
               AND p.snapshot_id=d.snapshot_id WHERE d.kind IN ({}, {}) \
             ORDER BY function_node_id, formal_node_id",
            DeclarationKind::Function.code(),
            DeclarationKind::AsyncFunction.code()
        ),
        "summary_flows" => format!(
            "SELECT summary_id, function_node_id, parameter_node_id, input_path, output_path, \
                    {kind} AS kind, condition_id, {verdict} AS verdict, \
                    {reason} AS boundary_reason, source_flow_fact_id, source_origin_id, return_site_fact_id, \
                    return_region_fact_id, approximated, path_depth \
             FROM summary_flows ORDER BY summary_id",
            kind = text_of::<SummaryFlowKind>("kind"),
            verdict = text_of::<Verdict>("verdict"),
            reason = text_of::<BoundaryReason>("boundary_reason"),
        ),
        "summary_flow_steps" => format!(
            "SELECT summary_id, ordinal, {kind} AS kind, evidence_id, condition_id \
             FROM summary_flow_steps ORDER BY summary_id, ordinal",
            kind = text_of::<SummaryFlowStepKind>("kind"),
        ),
        "summary_boundaries" => format!(
            "SELECT function_node_id, parameter_node_id, source_flow_fact_id, source_origin_id, condition_id, \
                    {reason} AS reason, local_through_call, upstream_through_call, \
                    raw_approximated FROM summary_boundaries \
             ORDER BY function_node_id, parameter_node_id, source_origin_id, condition_id",
            reason = text_of::<BoundaryReason>("reason"),
        ),
        "behavior_discharges" => format!(
            "SELECT behavior_id, origin_id, {kind} AS proof_kind, {decision} AS decision, \
                    summary_id, {reason} AS reason \
             FROM behavior_discharges ORDER BY behavior_id, origin_id",
            kind = text_of::<DischargeProofKind>("proof_kind"),
            decision = text_of::<DischargeDecision>("decision"),
            reason = text_of::<BoundaryReason>("reason"),
        ),
        "flow_test_leaves" => format!(
            "SELECT l.fact_id, l.module_node_id, l.condition_id, l.atom_id, l.atom, \
                    f.path, l.leaf_start_byte, l.leaf_end_byte \
             FROM flow_test_leaves l LEFT JOIN ( \
               SELECT module_node_id, min(path) AS path FROM ({files}) GROUP BY module_node_id \
             ) f ON f.module_node_id = l.module_node_id ORDER BY l.fact_id",
            files = cpg_schema::flows::display_files_sql(),
        ),
        "flow_test_value_links" => format!(
            "SELECT l.link_id, l.operation_node_id, l.formal_node_id, l.module_node_id, \
                    l.leaf_fact_id, l.atom_id, l.condition_id, l.place, \
                    {origin} AS origin, l.effect_model_digest, l.use_id, l.use_fact_id, \
                    l.reaching_fact_id, l.definition_fact_id, l.stability_origin_id, \
                    l.stability_condition_id, f.path, l.operand_start_byte, l.operand_end_byte \
             FROM flow_test_value_links l LEFT JOIN ( \
               SELECT module_node_id, min(path) AS path FROM ({files}) GROUP BY module_node_id \
             ) f ON f.module_node_id = l.module_node_id ORDER BY l.link_id",
            origin = text_of::<TestValueLinkOrigin>("l.origin"),
            files = cpg_schema::flows::display_files_sql(),
        ),
        "singletons" => "SELECT global, class_node_id FROM singletons ORDER BY global".to_owned(),
        "ambient_reads" => format!(
            "SELECT a.global, a.field, a.reader_node_id, \
                    COALESCE(o.access_path, d.qualified_name) AS reader, {phase} AS phase, \
                    df.path, a.line, a.start_byte, a.spelled, \
                    CASE WHEN a.condition = 'true' THEN NULL ELSE a.condition END AS condition \
             FROM ambient_reads a \
             LEFT JOIN operations o ON o.node_id = a.reader_node_id \
             LEFT JOIN (SELECT node_id, min(qualified_name) AS qualified_name FROM declarations \
                        GROUP BY node_id) d ON d.node_id = a.reader_node_id \
             LEFT JOIN (SELECT module_node_id, min(path) AS path FROM ({files}) \
                        GROUP BY module_node_id) df ON df.module_node_id = a.module_node_id \
             ORDER BY a.global, a.field, df.path, a.start_byte",
            phase = text_of::<cpg_schema::codebook::ReadPhase>("a.phase"),
            files = cpg_schema::flows::display_files_sql(),
        ),
        "place_claims" => format!(
            "SELECT place_key, {kind} AS kind, subject_node_id, holds, \
                    {reason} AS boundary_reason, reason \
             FROM negative_premises WHERE kind IN ({field}, {global}) ORDER BY place_key",
            kind = text_of::<cpg_schema::codebook::PremiseKind>("kind"),
            reason = text_of::<BoundaryReason>("boundary_reason"),
            field = cpg_schema::codebook::PremiseKind::Field.code(),
            global = cpg_schema::codebook::PremiseKind::Global.code(),
        ),
        "operation_vectors" => format!(
            "SELECT d.node_id, {view} AS embedding_view, d.chunk, d.input_hash, c.vector \
             FROM operation_documents d JOIN used_embeddings c \
               ON c.spec_hash = d.spec_hash AND c.input_hash = d.input_hash \
             ORDER BY d.node_id, embedding_view, d.chunk",
            view = text_of::<EmbeddingView>("d.embedding_view"),
        ),
        "vectors" => "SELECT d.brief_id, d.chunk, d.input_hash, c.vector FROM brief_documents d \
                      JOIN used_embeddings c \
                        ON c.spec_hash = d.spec_hash AND c.input_hash = d.input_hash \
                      ORDER BY d.brief_id, d.chunk"
            .to_owned(),
        _ => return None,
    })
}

const STRICT: CastOptions<'static> = CastOptions {
    safe: false,
    format_options: arrow_cast::display::FormatOptions::new(),
};

/// A column cast toward its declared type: a fixed-size binary through `Binary`, a fixed-size
/// list with its child left nullable (the builder below declares it).
fn cast_to(a: &ArrayRef, field: &Field) -> Result<ArrayRef, CoreError> {
    Ok(match field.data_type() {
        DataType::FixedSizeBinary(_) => {
            let binary = cast_with_options(a, &DataType::Binary, &STRICT)?;
            cast_with_options(&binary, field.data_type(), &STRICT)?
        }
        DataType::FixedSizeList(child, n) => {
            let loose = DataType::FixedSizeList(
                Arc::new(Field::new(child.name(), child.data_type().clone(), true)),
                *n,
            );
            cast_with_options(a, &loose, &STRICT)?
        }
        other => cast_with_options(a, other, &STRICT)?,
    })
}

/// A column rebuilt through a builder of its declared type: fresh buffers, null slots zeroed.
fn rebuild(a: &ArrayRef, field: &FieldRef) -> Result<ArrayRef, CoreError> {
    let a = cast_to(a, field)?;
    let n = a.len();
    let null = |i: usize| a.is_null(i);
    let built: ArrayRef = match field.data_type() {
        DataType::FixedSizeBinary(width) => {
            let v = a.as_fixed_size_binary();
            let mut b = FixedSizeBinaryBuilder::with_capacity(n, *width);
            for i in 0..n {
                if null(i) {
                    b.append_null();
                } else {
                    b.append_value(v.value(i))?;
                }
            }
            Arc::new(b.finish())
        }
        DataType::Utf8 => {
            let v = a.as_string::<i32>();
            let mut b = StringBuilder::with_capacity(n, v.value_data().len());
            for i in 0..n {
                if null(i) {
                    b.append_null();
                } else {
                    b.append_value(v.value(i));
                }
            }
            Arc::new(b.finish())
        }
        DataType::Boolean => {
            let v = a.as_boolean();
            let mut b = BooleanBuilder::with_capacity(n);
            for i in 0..n {
                b.append_option((!null(i)).then(|| v.value(i)));
            }
            Arc::new(b.finish())
        }
        DataType::Int64 => {
            let v = a.as_primitive::<Int64Type>();
            let mut b = Int64Builder::with_capacity(n);
            for i in 0..n {
                b.append_option((!null(i)).then(|| v.value(i)));
            }
            Arc::new(b.finish())
        }
        DataType::FixedSizeList(child, size) => {
            let v = a.as_fixed_size_list();
            let mut b = FixedSizeListBuilder::with_capacity(
                Float32Builder::with_capacity(n * *size as usize),
                *size,
                n,
            )
            .with_field(child.clone());
            for i in 0..n {
                if null(i) {
                    return Err(bad(format!("{}: a null vector", field.name())));
                }
                let item = v.value(i);
                let floats = item.as_primitive::<Float32Type>();
                if floats.null_count() > 0 {
                    return Err(bad(format!("{}: a null vector component", field.name())));
                }
                b.values().append_slice(floats.values());
                b.append(true);
            }
            Arc::new(b.finish())
        }
        other => return Err(bad(format!("{other} is not a served column type"))),
    };
    if !field.is_nullable() && built.null_count() > 0 {
        return Err(bad(format!("{}: nulls in a non-null column", field.name())));
    }
    Ok(built)
}

/// A query's rows as one normalized batch of `schema`.
async fn normalized(
    ctx: &SessionContext,
    statement: &str,
    schema: &SchemaRef,
) -> Result<RecordBatch, CoreError> {
    let batches = sql::query(ctx, statement).await?.collect().await?;
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let parts: Vec<ArrayRef> = batches
            .iter()
            .map(|b| {
                b.column_by_name(field.name())
                    .cloned()
                    .ok_or_else(|| bad(format!("the query lacks {}", field.name())))
            })
            .collect::<Result<_, _>>()?;
        let whole: ArrayRef = if parts.is_empty() {
            arrow_array::new_empty_array(field.data_type())
        } else {
            let refs: Vec<&dyn Array> = parts.iter().map(|p| p.as_ref()).collect();
            arrow_select::concat::concat(&refs)?
        };
        columns.push(rebuild(&whole, field)?);
    }
    Ok(RecordBatch::try_new(schema.clone(), columns)?)
}

/// `lexical_text`: each brief's documents, then the distinct tokens of its seed's **own** public
/// names, each once however many spellings or splits produce it (§11.2; the holistic assessment's
/// A1(c); ADR-0010's R2 F1 amendment: inherited spellings promote, but name nothing here).
async fn lexical(ctx: &SessionContext, schema: &SchemaRef) -> Result<RecordBatch, CoreError> {
    let docs = normalized(
        ctx,
        "SELECT d.brief_id, d.chunk, d.text, m.access_path FROM brief_documents d \
         LEFT JOIN brief_members m ON m.brief_id = d.brief_id AND m.own \
         ORDER BY d.brief_id, d.chunk, m.access_path",
        &Arc::new(arrow_schema::Schema::new(vec![
            Field::new("brief_id", DataType::FixedSizeBinary(16), false),
            Field::new("chunk", DataType::Int64, false),
            Field::new("text", DataType::Utf8, false),
            Field::new("access_path", DataType::Utf8, true),
        ])),
    )
    .await?;
    let ids = docs.column(0).as_fixed_size_binary();
    let chunks = docs.column(1).as_primitive::<Int64Type>();
    let texts = docs.column(2).as_string::<i32>();
    let paths = docs.column(3).as_string::<i32>();
    /// A brief's document chunks and the tokens of its public names.
    type Lexical = (Vec<(i64, String)>, Vec<String>);
    let mut per: BTreeMap<Vec<u8>, Lexical> = BTreeMap::new();
    for i in 0..docs.num_rows() {
        let entry = per.entry(ids.value(i).to_vec()).or_default();
        if !entry.0.iter().any(|(c, _)| *c == chunks.value(i)) {
            entry.0.push((chunks.value(i), texts.value(i).to_owned()));
        }
        if !paths.is_null(i) {
            cpg_schema::bundle::name_tokens(paths.value(i), &mut entry.1);
        }
    }
    let mut id_b = FixedSizeBinaryBuilder::with_capacity(per.len(), 16);
    let mut text_b = StringBuilder::new();
    for (id, (chunks, words)) in &per {
        id_b.append_value(id)?;
        let body: Vec<&str> = chunks.iter().map(|(_, t)| t.as_str()).collect();
        text_b.append_value(format!("{}\n{}", body.join("\n"), words.join(" ")));
    }
    Ok(RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(id_b.finish()), Arc::new(text_b.finish())],
    )?)
}

/// `operation_text`: each public operation's docstring summary, its parameters' names, and the
/// distinct name tokens of all its public spellings (`cpg_schema::bundle::name_tokens`), for
/// `search_operations`' BM25 (ADR-0021).
async fn operation_text(
    ctx: &SessionContext,
    schema: &SchemaRef,
) -> Result<RecordBatch, CoreError> {
    let rows = normalized(
        ctx,
        &format!(
            "SELECT o.node_id, o.docstring_summary, p.access_path, f.value AS parameter \
             FROM operations o \
             LEFT JOIN public_paths p ON p.node_id = o.node_id \
             LEFT JOIN operation_facets f ON f.node_id = o.node_id AND f.facet = {parameter} \
             ORDER BY o.node_id, p.access_path, f.value",
            parameter = OperationFacet::Parameter.code(),
        ),
        &Arc::new(arrow_schema::Schema::new(vec![
            Field::new("node_id", DataType::FixedSizeBinary(16), false),
            Field::new("docstring_summary", DataType::Utf8, true),
            Field::new("access_path", DataType::Utf8, true),
            Field::new("parameter", DataType::Utf8, true),
        ])),
    )
    .await?;
    let ids = rows.column(0).as_fixed_size_binary();
    let summaries = rows.column(1).as_string::<i32>();
    let paths = rows.column(2).as_string::<i32>();
    let parameters = rows.column(3).as_string::<i32>();
    /// An operation's summary, parameter names and name tokens.
    type Text = (Option<String>, Vec<String>, Vec<String>);
    let mut per: BTreeMap<Vec<u8>, Text> = BTreeMap::new();
    for i in 0..rows.num_rows() {
        let e = per.entry(ids.value(i).to_vec()).or_default();
        if e.0.is_none() && !summaries.is_null(i) {
            e.0 = Some(summaries.value(i).to_owned());
        }
        if !parameters.is_null(i) {
            let p = parameters.value(i).trim_start_matches('*').to_owned();
            if !e.1.contains(&p) {
                e.1.push(p);
            }
        }
        if !paths.is_null(i) {
            cpg_schema::bundle::name_tokens(paths.value(i), &mut e.2);
        }
    }
    let mut id_b = FixedSizeBinaryBuilder::with_capacity(per.len(), 16);
    let mut text_b = StringBuilder::new();
    for (id, (summary, parameters, words)) in &per {
        id_b.append_value(id)?;
        text_b.append_value(format!(
            "{}\n{}\n{}",
            summary.as_deref().unwrap_or_default(),
            parameters.join(" "),
            words.join(" ")
        ));
    }
    Ok(RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(id_b.finish()), Arc::new(text_b.finish())],
    )?)
}

/// One batch as an Arrow IPC file's bytes (V5, 64-byte alignment, uncompressed).
fn ipc_bytes(batch: &RecordBatch) -> Result<Vec<u8>, CoreError> {
    let options = IpcWriteOptions::try_new(64, false, MetadataVersion::V5)?;
    let mut out = Vec::new();
    {
        let mut writer = FileWriter::try_new_with_options(&mut out, &batch.schema(), options)?;
        writer.write(batch)?;
        writer.finish()?;
    }
    Ok(out)
}

/// `value` with every object's keys sorted (serde_json keeps insertion order in this build).
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let ordered: BTreeMap<String, Value> =
                map.into_iter().map(|(k, v)| (k, sorted(v))).collect();
            Value::Object(ordered.into_iter().collect::<Map<String, Value>>())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

/// The generation key of a manifest (its `generation` field left out).
fn key_of(manifest: &Value) -> Result<String, CoreError> {
    let mut body = manifest.clone();
    if let Value::Object(map) = &mut body {
        map.remove("generation");
    }
    let canonical = serde_json::to_string(&sorted(body)).map_err(|e| bad(e.to_string()))?;
    Ok(sha256(canonical.as_bytes())[..16].to_owned())
}

/// `(label…, count)` rows of a grouping query whose label columns are text.
async fn counted(
    ctx: &SessionContext,
    statement: &str,
) -> Result<Vec<(Vec<String>, i64)>, CoreError> {
    let mut out = Vec::new();
    for b in sql::query(ctx, statement).await?.collect().await? {
        let n = b.num_columns();
        let labels: Vec<ArrayRef> = (0..n - 1)
            .map(|c| cast_with_options(b.column(c), &DataType::Utf8, &STRICT))
            .collect::<Result<_, _>>()?;
        let counts = cast_with_options(b.column(n - 1), &DataType::Int64, &STRICT)?;
        let counts = counts.as_primitive::<Int64Type>();
        for i in 0..b.num_rows() {
            let row = labels
                .iter()
                .map(|l| {
                    let l = l.as_string::<i32>();
                    if l.is_null(i) {
                        "-".to_owned()
                    } else {
                        l.value(i).to_owned()
                    }
                })
                .collect();
            out.push((row, counts.value(i)));
        }
    }
    Ok(out)
}

/// Nested objects of counts, one level per label.
fn nest(rows: Vec<(Vec<String>, i64)>) -> Value {
    fn insert(map: &mut Map<String, Value>, labels: &[String], n: i64) {
        match labels {
            [] => {}
            [last] => {
                map.insert(last.clone(), json!(n));
            }
            [first, rest @ ..] => {
                let child = map
                    .entry(first.clone())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(child) = child {
                    insert(child, rest, n);
                }
            }
        }
    }
    let mut root = Map::new();
    for (labels, n) in rows {
        insert(&mut root, &labels, n);
    }
    Value::Object(root)
}

/// The coverage summary (§6.4): coverage by scope, family and status; boundaries by reason;
/// analysis invocations by completion; briefs by review state; unresolved slots by section (the
/// §B11 gap metric).
async fn coverage(ctx: &SessionContext) -> Result<Value, CoreError> {
    let unresolved = EvidenceStatus::Unresolved.code();
    Ok(json!({
        "coverage": nest(counted(ctx, &format!(
            "SELECT {}, {}, {}, count(*) FROM coverage GROUP BY 1, 2, 3 ORDER BY 1, 2, 3",
            text_of::<ScopeKind>("scope_kind"),
            text_of::<FactFamily>("fact_family"),
            text_of::<CoverageStatus>("status"),
        )).await?),
        "boundaries": nest(counted(ctx, &format!(
            "SELECT {}, count(*) FROM boundaries GROUP BY 1 ORDER BY 1",
            text_of::<BoundaryReason>("reason"),
        )).await?),
        "invocations": nest(counted(ctx, &format!(
            "SELECT {}, count(*) FROM analysis_invocations GROUP BY 1 ORDER BY 1",
            text_of::<CoverageStatus>("completion"),
        )).await?),
        "briefs": nest(counted(ctx, &format!(
            "SELECT {}, CASE WHEN documentation_only THEN 'documentation_only' \
                    ELSE 'analysis_backed' END, count(*) FROM briefs GROUP BY 1, 2 ORDER BY 1, 2",
            text_of::<ReviewState>("review_state"),
        )).await?),
        "unresolved_slots": nest(counted(ctx, &format!(
            "SELECT {}, count(*) FROM assertions WHERE evidence_status = {unresolved} \
             GROUP BY 1 ORDER BY 1",
            section_of("assertion_kind"),
        )).await?),
        // Slot sections a brief has no assertion in (increment-1 deep review F4).
        "absent_slots": nest(counted(ctx, &format!(
            "SELECT s.section, count(*) FROM briefs b CROSS JOIN (VALUES {slots}) AS s(section) \
             LEFT ANTI JOIN (SELECT ba.brief_id, {section} AS section FROM brief_assertions ba \
                             JOIN assertions a ON a.assertion_id = ba.assertion_id) p \
               ON p.brief_id = b.brief_id AND p.section = s.section \
             GROUP BY 1 ORDER BY 1",
            slots = SLOT_SECTIONS
                .iter()
                .map(|s| format!("('{}')", s.text()))
                .collect::<Vec<_>>()
                .join(", "),
            section = section_of("a.assertion_kind"),
        )).await?),
        "slot_sections": SLOT_SECTIONS.iter().map(|s| s.text()).collect::<Vec<_>>(),
    }))
}

/// Build a generation from a published session into `out/<key>/`. An existing generation of the
/// same key must hold the same bytes; it is then left as it is.
pub async fn build(ctx: &SessionContext, out: &Path) -> Result<Generation, CoreError> {
    let snapshot = counted(
        ctx,
        "SELECT encode(CAST(snapshot_id AS BYTEA), 'hex'), \
                encode(CAST(content_digest AS BYTEA), 'hex'), \
                encode(CAST(compiler_digest AS BYTEA), 'hex'), count(*) \
         FROM snapshots GROUP BY 1, 2, 3",
    )
    .await?;
    // The library the generation serves: an acquired library's name, else a source tree's label.
    let release = counted(
        ctx,
        "SELECT COALESCE(min(library), min(label)), COALESCE(min(requirement), ''), count(*) \
         FROM releases",
    )
    .await?;
    let [(release, _)] = release.as_slice() else {
        return Err(bad("the snapshot has no release"));
    };
    let [(ids, _)] = snapshot.as_slice() else {
        return Err(bad(format!(
            "the session holds {} snapshot identities, not one",
            snapshot.len()
        )));
    };
    // One spec, or none (lexical only); never two vector spaces in one generation.
    let specs = counted(
        ctx,
        "SELECT encode(CAST(spec_hash AS BYTEA), 'hex'), spec, count(*) FROM embedding_specs \
         GROUP BY 1, 2 ORDER BY 1",
    )
    .await?;
    let documents = counted(
        ctx,
        "SELECT encode(CAST(spec_hash AS BYTEA), 'hex'), count(*) FROM ( \
           SELECT spec_hash FROM brief_documents UNION ALL \
           SELECT spec_hash FROM operation_documents) \
         WHERE spec_hash IS NOT NULL GROUP BY 1 ORDER BY 1",
    )
    .await?;
    if specs.len() > 1 || documents.len() > specs.len() {
        return Err(bad(format!(
            "mixed embedding specs: {} declared, {} used by documents",
            specs.len(),
            documents.len()
        )));
    }
    let (spec_hash, dimensions) = match specs.first() {
        Some((labels, _)) => {
            let spec: crate::embed::Spec =
                crate::embed::Spec::parse(&labels[1]).map_err(|e| bad(format!("the spec: {e}")))?;
            if documents.first().is_some_and(|(d, _)| d[0] != labels[0]) {
                return Err(bad("the documents' spec is not the snapshot's"));
            }
            (Some(labels[0].clone()), spec.dimensions as i32)
        }
        None => (None, 0),
    };

    let served: Vec<ServingFile> = files(dimensions);
    let mut projection_batches = BTreeMap::new();
    let mut projection_relations = BTreeMap::new();
    let mut projection_artifacts = BTreeMap::new();
    let mut built: Vec<(&'static str, Vec<u8>, usize, String)> = Vec::new();
    for file in &served {
        let batch = match (query(file.name), file.name) {
            (Some(q), _) => normalized(ctx, &q, &file.schema).await?,
            (None, "operation_text") => operation_text(ctx, &file.schema).await?,
            (None, _) => lexical(ctx, &file.schema).await?,
        };
        let bounded_support = file.name.starts_with("support_")
            || matches!(
                file.name,
                "return_completion_certificates"
                    | "model_context_protocols"
                    | "source_context_sites"
                    | "source_context_arguments"
                    | "source_context_value_identities"
                    | "source_parameter_identities"
                    | "source_modeled_identities"
                    | "model_frame_exits"
                    | "model_frame_exit_arguments"
                    | "model_frame_exit_steps"
                    | "source_body_completions"
                    | "source_body_steps"
                    | "source_body_release_inputs"
                    | "source_call_bindings"
                    | "source_call_normals"
                    | "source_call_header_steps"
            );
        if bounded_support && batch.num_rows() > MAX_SUPPORT_ROWS {
            return Err(bad(format!(
                "{} exceeds the support projection row limit",
                file.name
            )));
        }
        cpg_schema::serving_projection::validate_batch(file.name, dimensions, &batch)
            .map_err(|e| bad(e.to_string()))?;
        projection_batches.insert(file.name.to_owned(), vec![batch.clone()]);
        let digest = schema_digest(&file.schema).map_err(bad)?;
        let bytes = ipc_bytes(&batch)?;
        if bounded_support && bytes.len() > MAX_SUPPORT_FILE_BYTES {
            return Err(bad(format!(
                "{} exceeds the support projection byte limit",
                file.name
            )));
        }
        projection_relations.insert(
            file.name.to_owned(),
            cpg_schema::serving_projection::receipt(
                file.name,
                dimensions,
                std::slice::from_ref(&batch),
            )
            .map_err(|e| bad(e.to_string()))?,
        );
        let artifact_name = format!("{}.arrow", file.name);
        if cpg_schema::serving_projection::artifact_names().contains(&artifact_name) {
            projection_artifacts.insert(
                artifact_name,
                cpg_schema::serving_projection::ArtifactReceipt {
                    sha256: sha256(&bytes),
                    bytes: bytes.len() as u64,
                    format: 1,
                },
            );
        }
        built.push((file.name, bytes, batch.num_rows(), digest));
    }
    cpg_schema::serving_projection::validate_relations(dimensions, &projection_batches)
        .map_err(|e| bad(e.to_string()))?;
    drop(projection_batches);
    let embedded = counted(
        ctx,
        "SELECT 'documents', count(*) FROM brief_documents WHERE input_hash IS NOT NULL",
    )
    .await?;
    let vectors = built.iter().find(|b| b.0 == "vectors").map_or(0, |b| b.2);
    if embedded.first().map_or(0, |e| e.1) as usize != vectors {
        return Err(bad(format!(
            "{vectors} vectors for {} embedded documents",
            embedded.first().map_or(0, |e| e.1)
        )));
    }

    let mut entries = Map::new();
    for (name, bytes, rows, digest) in &built {
        entries.insert(
            (*name).to_owned(),
            json!({
                "file": format!("{name}.arrow"),
                "rows": rows,
                "sha256": sha256(bytes),
                "schema_digest": digest,
            }),
        );
    }
    let projection = cpg_schema::serving_projection::Manifest {
        format: cpg_schema::serving_projection::FORMAT,
        bundle_format: FORMAT as u32,
        snapshot_id: ids[0].clone(),
        snapshot_digest: ids[1].clone(),
        compiler_digest: ids[2].clone(),
        projection_digest: cpg_schema::serving_projection::definition_digest(),
        catalog_digest: cpg_schema::models::Catalog::committed_digest().hex(),
        kernel_format: cpg_schema::condition_kernel::KERNEL_FORMAT,
        entry_value_effect_digest: crate::entry_links::digest().hex(),
        spec_hash: spec_hash.clone(),
        dimensions,
        relations: projection_relations,
        artifacts: projection_artifacts,
    };
    let projection_generation = projection.generation().map_err(|e| bad(e.to_string()))?;
    let mut manifest = json!({
        "projection": projection,
        "projection_generation": projection_generation,
        "format": FORMAT,
        "condition_kernel_format": cpg_schema::condition_kernel::KERNEL_FORMAT,
        "entry_value_effect_digest": crate::entry_links::digest().hex(),
        "library": release[0],
        "requirement": release[1],
        "snapshot_id": ids[0],
        "content_digest": ids[1],
        "compiler_digest": ids[2],
        "spec_hash": spec_hash,
        "files": Value::Object(entries),
        "summary": coverage(ctx).await?,
    });
    let key = key_of(&manifest)?;
    manifest["generation"] = json!(key);
    let manifest = sorted(manifest);
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| bad(e.to_string()))? + "\n";

    let dir = out.join(&key);
    fs_err::create_dir_all(out)?;
    let staging = out.join(format!(".building-{key}-{}", std::process::id()));
    if staging.exists() {
        fs_err::remove_dir_all(&staging)?;
    }
    fs_err::create_dir_all(&staging)?;
    for (name, bytes, _, _) in &built {
        fs_err::write(staging.join(format!("{name}.arrow")), bytes)?;
    }
    fs_err::write(staging.join("MANIFEST.json"), &text)?;
    if dir.exists() {
        for entry in fs_err::read_dir(&staging)? {
            let path = entry?.path();
            let name = path.file_name().expect("a file name");
            if fs_err::read(&path)? != fs_err::read(dir.join(name))? {
                fs_err::remove_dir_all(&staging)?;
                return Err(bad(format!(
                    "generation {key} exists with other bytes in {}",
                    name.to_string_lossy()
                )));
            }
        }
        fs_err::remove_dir_all(&staging)?;
    } else {
        fs_err::rename(&staging, &dir)?;
    }
    Ok(Generation { key, dir, manifest })
}

/// Build the generation of a published snapshot.
pub async fn bundle(root: &Path, snapshot_id: Id, out: &Path) -> Result<Generation, CoreError> {
    let (_, ctx) = crate::snapshot::published(root, snapshot_id)
        .await?
        .ok_or_else(|| bad(format!("snapshot {} is not published", snapshot_id.hex())))?;
    build(&ctx, out).await
}

/// Check a generation against its manifest: each file's sha256, row count and serving schema
/// digest, and the key, which must also name the directory.
pub fn verify(dir: &Path) -> Result<Value, CoreError> {
    let manifest: Value = serde_json::from_str(&fs_err::read_to_string(dir.join("MANIFEST.json"))?)
        .map_err(|e| bad(format!("MANIFEST.json: {e}")))?;
    if manifest["format"].as_u64() != Some(FORMAT)
        || manifest["condition_kernel_format"].as_u64()
            != Some(u64::from(cpg_schema::condition_kernel::KERNEL_FORMAT))
    {
        return Err(bad("manifest or condition kernel format mismatch"));
    }
    let projection: cpg_schema::serving_projection::Manifest =
        serde_json::from_value(manifest["projection"].clone()).map_err(|e| bad(e.to_string()))?;
    projection
        .validate_envelope(&manifest)
        .map_err(|e| bad(e.to_string()))?;
    let projection_key = projection.generation().map_err(|e| bad(e.to_string()))?;
    if manifest["projection_generation"].as_str() != Some(&projection_key)
        || projection.projection_digest != cpg_schema::serving_projection::definition_digest()
    {
        return Err(bad("projection identity/definition mismatch"));
    }
    let mut projection_batches = BTreeMap::new();
    let files = manifest["files"]
        .as_object()
        .ok_or_else(|| bad("MANIFEST.json lists no files"))?;
    for (name, entry) in files {
        let file = entry["file"]
            .as_str()
            .ok_or_else(|| bad(format!("{name}: no file")))?;
        if file != format!("{name}.arrow") {
            return Err(bad(format!("{name}: unexpected served file path")));
        }
        if dir.join(file).is_symlink() {
            return Err(bad(format!("{file}: a served file cannot be a symlink")));
        }
        let bytes = fs_err::read(dir.join(file))?;
        if Some(sha256(&bytes).as_str()) != entry["sha256"].as_str() {
            return Err(bad(format!("{file}: its sha256 differs from the manifest")));
        }
        projection
            .verify_artifact(file, &bytes)
            .map_err(|e| bad(e.to_string()))?;
        let reader = FileReader::try_new(std::io::Cursor::new(bytes), None)?;
        let digest = schema_digest(&reader.schema()).map_err(bad)?;
        if Some(digest.as_str()) != entry["schema_digest"].as_str() {
            return Err(bad(format!(
                "{file}: its schema digest differs from the manifest"
            )));
        }
        let batches = reader.collect::<Result<Vec<_>, _>>()?;
        let rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
        projection_batches.insert(name.to_owned(), batches);
        if Some(rows as u64) != entry["rows"].as_u64() {
            return Err(bad(format!("{file}: {rows} rows, not the manifest's")));
        }
    }
    projection
        .validate_relations(&projection_batches)
        .map_err(|e| bad(e.to_string()))?;
    let key = key_of(&manifest)?;
    let named = dir.file_name().map(|n| n.to_string_lossy().into_owned());
    if manifest["generation"].as_str() != Some(key.as_str()) || named.as_deref() != Some(&key) {
        return Err(bad(format!(
            "the generation key is {key}, not the manifest's or the directory's"
        )));
    }
    Ok(manifest)
}
