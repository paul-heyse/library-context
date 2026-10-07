//! Finite input contracts. Structural decoding happens before store effects.
use super::*;
use crate::domain::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    #[serde(default = "default_page_size")]
    /// Maximum requested page rows, default 20; values must remain within the declared resource limits.
    pub size: u32,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    /// Opaque continuation bound to snapshot, request and wire identity; reuse only with the original request.
    pub cursor: Optional<CursorToken>,
    #[serde(default)]
    /// Request the declared expanded response envelope; it remains bounded and does not imply completeness.
    pub expanded: bool,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    /// Desired visible information and exact context. No evaluator expectations or witness lists.
    pub evidence_demand: Optional<EvidenceDemand>,
}
fn default_page_size() -> u32 {
    ResourceLimits::default().default_page_rows
}
impl Default for PageRequest {
    fn default() -> Self {
        Self {
            size: default_page_size(),
            cursor: Optional::default(),
            expanded: false,
            evidence_demand: Optional::default(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationSelector {
    Member { member: Id<catalog::CatalogMember> },
    PublicPath { path: Vec<Name> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowseScope {
    Library {},
    Module { module: Id<source::Module> },
    Class { member: Id<catalog::CatalogMember> },
}
impl Default for BrowseScope {
    fn default() -> Self {
        Self::Library {}
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BrowseView {
    #[default]
    Members,
    Modules,
    Classes,
    Vocabulary,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationSection {
    Scenarios,
    Deployment,
    Relationships,
    Conflicts,
    Briefs,
    Behavior,
    CallableComparison,
    ContextualTyping,
    IncomingReferences,
    AccessRoutes,
}
pub fn default_selection() -> selection::Selection {
    selection::Selection::default()
}
macro_rules! request {($name:ident {$($(#[$attr:meta])* $field:ident:$ty:ty),*$(,)?})=>{
    #[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct $name {$ ($(#[$attr])* pub $field:$ty,)* #[serde(default)] pub page:PageRequest}
};}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchOperationsRequest {
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    /// Omit to search the finite union of admitted snapshot captures.
    pub library: Optional<Name>,
    /// Lexical or vector query text; retrieval rank is navigation evidence rather than a behavioral proof.
    pub query: QueryText,
    #[serde(default)]
    /// Finite declared predicates and admission policy; discovery preserves unresolved evidence by default.
    pub selection: SelectionInput,
    #[serde(default)]
    pub page: PageRequest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindOperationsRequest {
    pub library: Name,
    #[serde(default)]
    /// Finite declared predicates and admission policy; discovery preserves unresolved evidence by default.
    pub selection: SelectionInput,
    #[serde(default)]
    pub page: PageRequest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CallableComparisonRequest {
    pub analysis: Id<attribution::AnalysisContext>,
    pub left: Id<normalized::callables::SignatureVariant>,
    pub right: Id<normalized::callables::SignatureVariant>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetOperationRequest {
    pub library: Name,
    pub operation: OperationSelector,
    /// Required when requesting callable comparison; variants are selected explicitly.
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub comparison: Optional<CallableComparisonRequest>,
    /// Optional exact formal target, admitted only when it belongs to the selected member.
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub reference_parameter: Optional<Id<normalized::entities::ParameterEntity>>,
    #[serde(default)]
    pub sections: Vec<OperationSection>,
    #[serde(default)]
    pub page: PageRequest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrowseLibraryRequest {
    pub library: Name,
    #[serde(default)]
    pub scope: BrowseScope,
    #[serde(default)]
    pub view: BrowseView,
    #[serde(default)]
    /// Finite declared predicates and admission policy; discovery preserves unresolved evidence by default.
    pub selection: SelectionInput,
    #[serde(default)]
    pub page: PageRequest,
}
/// The finite canonical Selection owns all predicate interpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SelectionInput(pub selection::Selection);
impl Default for SelectionInput {
    fn default() -> Self {
        Self(default_selection())
    }
}
request!(GetEvidenceRequest {
    source: OriginalReference
});
request!(SearchEvidenceRequest {#[serde(default, skip_serializing_if = "Optional::is_absent")] #[doc = "Omit to search the finite union of admitted snapshot captures."] library:Optional<Name>,#[doc = "Lexical or vector query text; retrieval rank is navigation evidence rather than a behavioral proof."] query:QueryText,#[doc = "Restrict evidence retrieval to these declared families; an empty list uses the route default."] families:Vec<retrieval::Family>});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareOperationsRequest {
    pub library: Name,
    pub operations: Vec<OperationSelector>,
    #[serde(default)]
    /// Finite declared predicates and admission policy; discovery preserves unresolved evidence by default.
    pub selection: SelectionInput,
    #[serde(default)]
    pub page: PageRequest,
}
request!(SearchCapabilitiesRequest {
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    #[doc = "Omit to search the finite union of admitted snapshot captures."]
    library: Optional<Name>,
    #[doc = "Lexical or vector query text; retrieval rank is navigation evidence rather than a behavioral proof."]
    query: QueryText
});
request!(GetCapabilityRequest {capability:Id<synthesis::briefs::Brief>});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExactInputBinding {
    /// Canonical public formal identity; a valid defaulted formal may still refuse exact scalar assignment.
    pub formal: Id<normalized::entities::ParameterEntity>,
    pub value: native_requests::ExactScalar,
}
request!(InspectValuePathsRequest {member:Id<catalog::CatalogMember>,analysis:Id<attribution::AnalysisContext>,inputs:Vec<ExactInputBinding>,#[doc = "Explicit finite assumptions for exact scalar restriction; no Python is executed."] assumptions:native_requests::Assumptions});

/// Public information facets. Requesting a facet never asserts that its evidence exists.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFacet {
    Declaration,
    Defaults,
    Conditions,
    Setup,
    Originals,
    Scenarios,
    Deployment,
    Relationships,
    Behavior,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct EvidenceContext {
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub release: Optional<Name>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub signature: Optional<Id<calls::Signature>>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub variant: Optional<Id<normalized::callables::SignatureVariant>>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub analysis: Optional<Id<attribution::AnalysisContext>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceDemand {
    pub facets: Vec<EvidenceFacet>,
    #[serde(default)]
    pub context: EvidenceContext,
    /// Bound for suggested public expansions, not permission to execute them automatically.
    #[serde(default)]
    pub maximum_followups: u32,
}
