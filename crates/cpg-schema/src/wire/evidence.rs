//! Shared typed evidence transport; byte pagination does not change extraction status.
use super::*;
use crate::evidence::{Alignment, DeploymentDetail, EvidenceRef, ScenarioDetail};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetEvidenceRequest {
    pub snapshot_id: SnapshotId,
    pub evidence: EvidenceRef,
    #[serde(default)]
    pub cursor: Option<Text<0, 2048>>,
    #[serde(default)]
    pub expanded: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceAssociation {
    pub subject: AssociationSubject,
    pub association_id: crate::Id,
    pub evidence: EvidenceRef,
    pub role: String,
    pub basis: String,
    pub intent: crate::evidence::Intent,
    pub site_id: Option<crate::Id>,
    pub support: Vec<crate::evidence::AssociationSupport>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssociationSubject {
    Member { member_id: crate::Id },
    Release { release_id: crate::Id },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct EvidencePage {
    pub items: Vec<EvidenceAssociation>,
    pub total: u64,
    pub next_cursor: Option<String>,
    pub demonstrations: u64,
    pub negative: u64,
    pub context_dependent: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OriginalContent {
    pub span_id: crate::Id,
    pub artifact_id: crate::Id,
    pub release_id: crate::Id,
    pub source_digest: crate::Digest,
    pub path: String,
    pub source_kind: String,
    pub alignment: Alignment,
    pub provenance: String,
    pub span_start: i64,
    pub span_end: i64,
    pub chunk_start: i64,
    pub chunk_end: i64,
    pub text: Option<String>,
    pub bytes_base64: Option<String>,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceResult {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub evidence: EvidenceRef,
    pub scenario: Option<ScenarioDetail>,
    pub deployment: Option<DeploymentDetail>,
    pub metadata_omitted: bool,
    pub content: Vec<OriginalContent>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_kind", rename_all = "snake_case")]
pub enum GetEvidenceResponse {
    Evidence(EvidenceResult),
}
