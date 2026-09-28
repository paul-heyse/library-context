//! Shared existing response contracts. Python is a transport and rendering consumer.
use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum DischargeDecision {
    #[serde(rename = "proved")]
    Proved,
    #[serde(rename = "open")]
    Open,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum FateTransfer {
    #[serde(rename = "identity")]
    Identity,
    #[serde(rename = "derived")]
    Derived,
    #[serde(rename = "call")]
    Call,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AmbiguousOperationResolution {
    #[serde(rename = "ambiguous")]
    Ambiguous,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OperationHitsMode {
    #[serde(rename = "hybrid")]
    Hybrid,
    #[serde(rename = "lexical-only")]
    LexicalOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CapabilityUnavailableStatus {
    #[serde(rename = "unavailable")]
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CapabilityUnavailableReason {
    #[serde(rename = "not_requested")]
    NotRequested,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SearchResultMode {
    #[serde(rename = "hybrid")]
    Hybrid,
    #[serde(rename = "lexical-only")]
    LexicalOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum FindingSupportSourceResolution {
    #[serde(rename = "source_span")]
    SourceSpan,
    #[serde(rename = "fact_only")]
    FactOnly,
    #[serde(rename = "unavailable")]
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RetrievalMetadataRequestedRoute {
    #[serde(rename = "exact")]
    Exact,
    #[serde(rename = "hnsw")]
    Hnsw,
    #[serde(rename = "mixed")]
    Mixed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RetrievalMetadataActualRoute {
    #[serde(rename = "exact")]
    Exact,
    #[serde(rename = "hnsw")]
    Hnsw,
    #[serde(rename = "lexical-only")]
    LexicalOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ValuePathExactInputResult {
    #[serde(rename = "refuted_under_model")]
    RefutedUnderModel,
    #[serde(rename = "compatible_under_model")]
    CompatibleUnderModel,
    #[serde(rename = "unknown")]
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Discharge {
    pub origin_id: String,
    pub proof_kind: String,
    pub decision: DischargeDecision,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub summary_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Fate {
    pub kind: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<FateTransfer>")]
    pub transfer: Option<FateTransfer>,
    pub condition_scope_id: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub parameter: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub callee: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub target: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub value: Option<String>,
    pub depth: i64,
    pub conditional: bool,
    pub verdict: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub boundary_reason: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub condition: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub callee_text: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub phase: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub premise_key: Option<String>,
    pub occurrences: i64,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub path: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<i64>")]
    pub line: Option<i64>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub site_text: Option<String>,
    #[serde(default)]
    pub discharges: Vec<Discharge>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ParameterRecord {
    pub name: String,
    pub fates: Vec<Fate>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub note: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SettingRead {
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub reader: Option<String>,
    pub phase: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub path: Option<String>,
    pub line: i64,
    pub spelled: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub condition: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FieldRecord {
    pub name: String,
    pub reads: Vec<SettingRead>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub never_read: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FacetValue {
    pub value: String,
    pub verdict: String,
}
pub type CatalogMember = crate::catalog::CatalogMembersRow;
pub type CatalogBinding = crate::catalog::CatalogBindingsRow;
pub type CatalogParameter = crate::catalog::CatalogParametersRow;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CatalogSignature {
    #[serde(flatten)]
    pub contract: crate::catalog::CatalogSignaturesRow,
    pub parameters: Vec<CatalogParameter>,
}
pub type CatalogEvidence = crate::catalog::CatalogEvidenceRow;
pub type CatalogType = crate::catalog::CatalogTypesRow;
pub type CatalogTypeArgument = crate::catalog::CatalogTypeArgsRow;
pub type CatalogTypeObservation = crate::catalog::CatalogTypeObservationsRow;
pub type CatalogConstructor = crate::catalog::CatalogConstructorsRow;
pub type CatalogSurface = crate::catalog::CatalogSurfacesRow;
pub type CatalogConfiguration = crate::catalog::CatalogConfigurationsRow;
pub type CatalogFieldLink = crate::catalog::CatalogFieldLinksRow;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CatalogRecord {
    pub evidence_page: super::EvidencePage,
    pub demonstrations: Vec<super::EvidenceResult>,
    pub surfaces: Vec<CatalogSurface>,
    pub configurations: Vec<CatalogConfiguration>,
    pub field_links: Vec<CatalogFieldLink>,
    pub member: CatalogMember,
    pub constructors: Vec<CatalogConstructor>,
    pub bindings: Vec<CatalogBinding>,
    pub signatures: Vec<CatalogSignature>,
    pub type_observations: Vec<CatalogTypeObservation>,
    pub types: Vec<CatalogType>,
    pub type_arguments: Vec<CatalogTypeArgument>,
    pub effective_surface: String,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AmbiguousOperation {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub resolution: AmbiguousOperationResolution,
    pub requested: String,
    pub choices: Vec<CatalogMember>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Operation {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<OperationId>")]
    pub operation_id: Option<OperationId>,
    #[serde(default)]
    pub member_id: Option<PublicMemberId>,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub capabilities: BTreeMap<String, bool>,
    #[serde(default)]
    pub catalog: Option<CatalogRecord>,
    pub access_path: String,
    pub own_paths: Vec<String>,
    pub inherited_paths: Vec<String>,
    pub kind: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<bool>")]
    pub is_method: Option<bool>,
    pub qualified_name: String,
    pub module: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub docstring_summary: Option<String>,
    pub behavior_status: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub boundary_reason: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub status_reason: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<CapabilityId>")]
    pub capability_id: Option<CapabilityId>,
    pub facets: BTreeMap<String, Vec<FacetValue>>,
    pub incomplete_facets: BTreeMap<String, String>,
    pub parameters: Vec<ParameterRecord>,
    #[serde(default)]
    pub unbound_parameter_fates: Vec<Fate>,
    pub delegates: Vec<Fate>,
    pub handoffs: Vec<Fate>,
    #[serde(default)]
    pub reads: Vec<Fate>,
    #[serde(default)]
    pub constructor: Option<Box<Operation>>,
    #[serde(default)]
    pub singleton_of: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldRecord>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationRef {
    pub operation_id: OperationId,
    pub access_path: String,
    pub kind: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub docstring_summary: Option<String>,
    pub behavior_status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationSet {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub matches: Vec<OperationRef>,
    pub total: i64,
    pub complete: bool,
    pub unknown: Vec<OperationRef>,
    pub unknown_total: i64,
    pub unknown_truncated: bool,
    pub truncated: bool,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub next_cursor: Option<String>,
    pub note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationHit {
    pub operation_id: OperationId,
    pub access_path: String,
    pub kind: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub docstring_summary: Option<String>,
    pub relevance: f64,
    pub rank_source: String,
    pub promoted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationHits {
    pub retrieval: RetrievalMetadata,
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub mode: OperationHitsMode,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub degraded_reason: Option<String>,
    pub ranked_discovery: bool,
    pub hits: Vec<OperationHit>,
    pub note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Hit {
    pub capability_id: CapabilityId,
    pub title: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub outcome: Option<String>,
    pub outcome_status: String,
    pub relevance: f64,
    pub rank_source: RankSource,
    pub promoted: bool,
}
fn capabilityunavailable_status_default() -> CapabilityUnavailableStatus {
    CapabilityUnavailableStatus::Unavailable
}
fn capabilityunavailable_reason_default() -> CapabilityUnavailableReason {
    CapabilityUnavailableReason::NotRequested
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityUnavailable {
    #[serde(default = "capabilityunavailable_status_default")]
    pub status: CapabilityUnavailableStatus,
    #[serde(default = "capabilityunavailable_reason_default")]
    pub reason: CapabilityUnavailableReason,
    pub capability: String,
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub retrieval: RetrievalMetadata,
    pub library: String,
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub mode: SearchResultMode,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub degraded_reason: Option<String>,
    pub coverage: crate::serving_projection::CoverageSummary,
    pub note: String,
    pub hits: Vec<Hit>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Evidence {
    pub evidence_id: EvidenceId,
    pub kind: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub path: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<i64>")]
    pub start_byte: Option<i64>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<i64>")]
    pub end_byte: Option<i64>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub text: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindingWitness {
    pub path: i64,
    pub step: i64,
    pub call_site_node_id: String,
    pub callee_node_id: String,
    pub arc_kind: String,
    pub modality: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub phase: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub source_fact_id: Option<String>,
    pub source_path: String,
    pub start_byte: i64,
    pub end_byte: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConceptAttribute {
    pub attribute_id: String,
    pub kind: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub symbol: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub parameter_kind: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<TypeTermId>")]
    pub type_term_id: Option<TypeTermId>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub class_module: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub class_key: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub target_node_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub modality: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub phase: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub producer_modality: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub producer_phase: Option<String>,
    pub display: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttributeIncidence {
    pub finding_id: String,
    pub incidence_id: String,
    pub attribute_id: String,
    pub object_node_id: String,
    pub source_fact_id: String,
    pub fact_table: String,
    pub fact_model_id: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub site_node_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub edge_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub other_site_node_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub other_edge_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub consumer_formal_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub other_fact_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub other_fact_table: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub other_fact_model_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindingMember {
    pub role: String,
    pub ordinal: i64,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub node_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub cited_fact_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub attribute_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub fact_table: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub fact_model_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub label: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindingSupport {
    pub finding_id: String,
    pub kind: String,
    pub evidence_status: String,
    pub subject_node_id: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub related_node_id: Option<String>,
    pub invocation_id: String,
    pub model_id: String,
    pub method: String,
    pub parameters: String,
    pub completion: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub stop_reason: Option<String>,
    pub witnesses_omitted: bool,
    pub witnesses: Vec<FindingWitness>,
    pub members: Vec<FindingMember>,
    pub attributes: Vec<ConceptAttribute>,
    pub attribute_incidences: Vec<AttributeIncidence>,
    pub source_resolution: FindingSupportSourceResolution,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Support {
    pub role: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub finding_id: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub finding_kind: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<EvidenceId>")]
    pub evidence_id: Option<EvidenceId>,
    #[serde(default)]
    pub finding: Option<FindingSupport>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Assertion {
    pub assertion_id: String,
    pub kind: String,
    pub section: String,
    pub status: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub text: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub applicable_case: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub conditions: Option<String>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub limitations: Option<String>,
    pub supports: Vec<Support>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Capability {
    pub library: String,
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub capability_id: CapabilityId,
    pub title: String,
    pub access_path: String,
    pub public_paths: Vec<String>,
    pub documentation_only: bool,
    pub review_state: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub outcome: Option<String>,
    pub outcome_status: String,
    pub assertions: Vec<Assertion>,
    pub evidence: Vec<Evidence>,
    pub sections_absent: Vec<String>,
}
fn retrievalmetadata_candidate_depth_default() -> i64 {
    0
}
fn retrievalmetadata_approximate_default() -> bool {
    false
}
fn retrievalmetadata_routing_reason_default() -> String {
    "explicit_profile".to_owned()
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrievalMetadata {
    pub profile: String,
    pub requested_route: RetrievalMetadataRequestedRoute,
    pub actual_route: RetrievalMetadataActualRoute,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default = "retrievalmetadata_candidate_depth_default")]
    pub candidate_depth: i64,
    #[serde(default = "retrievalmetadata_approximate_default")]
    pub approximate: bool,
    #[serde(default = "retrievalmetadata_routing_reason_default")]
    pub routing_reason: String,
    #[serde(default)]
    pub admission: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProofStep {
    pub kind: String,
    pub evidence_id: EvidenceId,
    pub condition_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValueLinkEvidence {
    pub link_id: String,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub path: Option<String>,
    pub start_byte: i64,
    pub end_byte: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TheoryWork {
    pub links_examined: i64,
    pub assignments_applied: i64,
    pub bdd_preflight_pairs: i64,
    pub peak_bdd_nodes: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValuePath {
    pub summary_id: String,
    pub source_flow_fact_id: String,
    pub source_origin_id: String,
    pub source_verdict: String,
    pub condition_id: String,
    pub steps: Vec<ProofStep>,
    pub exact_input_result: ValuePathExactInputResult,
    pub value_links: Vec<ValueLinkEvidence>,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub boundary_reason: Option<String>,
    pub theory_work: TheoryWork,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpenBoundary {
    pub source_flow_fact_id: String,
    pub source_origin_id: String,
    pub condition_id: String,
    pub reason: String,
}
fn valuepathpage_note_default() -> String {
    "A refutation applies only to its cited summary path under the exact input model. Compatibility means only a satisfiable model after checked value links, not a concrete execution. Unknown and absent paths do not establish absence.".to_owned()
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValuePathPage {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub operation: String,
    pub formal: String,
    pub exact_input: ExactPrimitive,
    pub standard_builtins: bool,
    pub paths: Vec<ValuePath>,
    pub boundaries: Vec<OpenBoundary>,
    pub total_rows: i64,
    pub examined_rows: i64,
    pub theory_work: TheoryWork,
    pub truncated: bool,
    #[serde(deserialize_with = "super::required_nullable")]
    #[schemars(with = "super::Nullable<String>")]
    pub next_cursor: Option<String>,
    #[serde(default = "valuepathpage_note_default")]
    pub note: String,
}
