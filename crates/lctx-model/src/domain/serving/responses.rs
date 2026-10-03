//! Complete structured results. Required packet data is never silently truncated.
use super::*;
use crate::domain::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
macro_rules! response {($name:ident {$($(#[$attr:meta])* $field:ident:$ty:ty),*$(,)?})=>{
    #[derive(Debug,Clone,PartialEq,Serialize,Deserialize,JsonSchema)]
    #[serde(deny_unknown_fields)] pub struct $name {pub generation:GenerationKey,$($(#[$attr])* pub $field:$ty,)*}
};}
response!(SearchOperationsResponse {results:SectionPage<OperationCandidate>,extent:SelectionExtent,channels:ChannelState,ranking:Vec<ranking::RankedHit>});
response!(FindOperationsResponse {supported:SectionPage<OperationCandidate>,unresolved:SectionPage<OperationCandidate>,conflicting:SectionPage<OperationCandidate>,extent:SelectionExtent});
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "resolution", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    clippy::large_enum_variant,
    reason = "Inline finite packets retain the value-size accounting used by request reservations"
)]
pub enum OperationResolution {
    Unique { packet: OperationPacket },
    Ambiguous { candidates: Vec<OperationCandidate> },
    Missing { coverage: Availability },
}
response!(GetOperationResponse {
    operation: OperationResolution
});
response!(BrowseLibraryResponse {scope:BrowseScope,view:BrowseView,entries:SectionPage<BrowseEntry>,extent:SelectionExtent,unknown_ownership:u64});
response!(GetEvidenceResponse {
    evidence: EvidencePacket
});
response!(SearchEvidenceResponse {results:SectionPage<EvidenceHit>,channels:ChannelState,extent:SelectionExtent,ranking:Vec<ranking::RankedHit>});
response!(CompareOperationsResponse {operations:Vec<ComparisonEntry>});
response!(SearchCapabilitiesResponse {results:SectionPage<CapabilityPacket>,channels:ChannelState,extent:SelectionExtent,ranking:Vec<ranking::RankedHit>});
response!(GetCapabilityResponse {
    capability: CapabilityPacket
});
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeAssessmentPacket {
    pub claim_basis:ClaimBasisPacket,
    pub path: ProofReference,
    /// Original stored condition identity; request restriction has a separate result identity.
    pub original_condition: Id<conditions::Condition>,
    pub restricted_result: Nullable<ContentHash>,
    /// Model-qualified verdict, never proof of unrestricted runtime behavior.
    pub verdict: obligation::Verdict,
    pub exact: native_requests::ExactOutcome,
    /// Canonical evidence or derivation basis for this result, rather than a confidence score.
    pub basis: native_requests::Basis,
    /// Explicit finite assumptions for exact scalar restriction; no Python is executed.
    pub assumptions: native_requests::Assumptions,
    pub proof: Vec<ProofReference>,
    pub work: native_requests::Work,
    /// Canonical reason for this result; when absent there is no additional diagnosed refusal.
    pub reason: Nullable<obligation::ObligationKind>,
    pub unexamined: u64,
    pub presentation: Nullable<RenderedConditionPacket>,
    pub presentation_truncated: bool,
    pub proof_truncated: bool,
}
response!(InspectValuePathsResponse {member:Id<catalog::CatalogMember>,paths:SectionPage<NativeAssessmentPacket>});
impl NativeAssessmentPacket {
    pub fn from_canonical(value: &native_requests::Assessment, claim_basis:ClaimBasisPacket) -> Self {
        Self {
            claim_basis,
            path: ProofReference::from_canonical(value.path),
            original_condition: value.original_condition,
            restricted_result: Nullable(value.restricted_result.map(|id| id.0)),
            verdict: value.verdict,
            exact: value.exact,
            basis: value.basis,
            assumptions: value.assumptions,
            proof: value
                .proof
                .iter()
                .copied()
                .map(ProofReference::from_canonical)
                .collect(),
            work: value.work,
            reason: Nullable(value.reason),
            unexamined: value.unexamined as u64,
            presentation: Nullable(
                value
                    .rendered
                    .as_ref()
                    .map(RenderedConditionPacket::from_canonical),
            ),
            presentation_truncated: value.presentation_truncated,
            proof_truncated: value.proof_truncated,
        }
    }
}
