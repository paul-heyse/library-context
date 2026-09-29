//! Bounded agent journeys; this is the sole transport contract owner (ADR-0081).
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationSection {
    Behavior,
    Relationships,
    Evidence,
    Fields,
    Facets,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationView {
    #[default]
    Packet,
    Section {
        section: OperationSection,
        #[serde(default)]
        cursor: Option<Text<0, 2048>>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SectionState {
    Available,
    NotRequested,
    Unavailable,
    EmptyUnderCoverage,
    OmittedBudget,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SectionDirectoryEntry {
    pub section: OperationSection,
    pub state: SectionState,
    pub total: Option<u64>,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationPacket {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub library: String,
    pub requirement: String,
    pub member_id: PublicMemberId,
    pub operation_id: Option<OperationId>,
    pub access_path: String,
    pub resolution: String,
    pub own_paths: Vec<String>,
    pub inherited_paths: Vec<String>,
    pub capabilities: crate::catalog::Capabilities,
    pub catalog: CatalogRecord,
    pub behavior_status: String,
    pub boundary_reason: Option<String>,
    pub status_reason: Option<String>,
    pub capability_id: Option<CapabilityId>,
    pub incomplete_facets: BTreeMap<String, String>,
    pub sections: Vec<SectionDirectoryEntry>,
    pub demonstrations: Vec<crate::evidence::EvidenceRef>,
    pub relationships: Vec<Fate>,
    pub evidence: RetrievalTarget,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationSectionRecord {
    Fate {
        record: Box<Fate>,
    },
    Association {
        record: EvidenceAssociation,
    },
    Field {
        record: FieldRecord,
    },
    Facet {
        name: String,
        value: String,
        verdict: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OperationSectionPage {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub member_id: PublicMemberId,
    pub section: OperationSection,
    pub state: SectionState,
    pub reason: Option<String>,
    pub items: Vec<OperationSectionRecord>,
    pub total: u64,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowseScope {
    #[default]
    Library,
    Module {
        name: Text<1, 500>,
    },
    Class {
        name: Text<1, 500>,
    },
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowseView {
    #[default]
    Outline,
    Vocabulary,
    FacetValues {
        facet: FacetName,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrowseLibraryRequest {
    pub library: Text<1, 500>,
    #[serde(default)]
    pub scope: BrowseScope,
    #[serde(default)]
    pub view: BrowseView,
    #[serde(default)]
    pub limit: Limit<100, 20>,
    #[serde(default)]
    pub cursor: Option<Text<0, 2048>>,
    #[serde(default)]
    pub expanded: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowseEntry {
    Module {
        name: String,
        members: u64,
    },
    Class {
        name: String,
        members: u64,
    },
    Member {
        member: CatalogMember,
        module: Option<String>,
        class_owner: Option<String>,
    },
    FacetValue {
        value: String,
        members: u64,
        verdicts: BTreeMap<String, u64>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryVocabulary {
    /// Generated from the authoritative Requirement contract, including referenced definitions.
    pub requirement_schema: serde_json::Value,
    pub facets: Vec<String>,
    pub retrieval_families: Vec<crate::retrieval::Family>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowseLibraryResponse {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub scope: BrowseScope,
    pub view: BrowseView,
    pub state: SectionState,
    pub reason: Option<String>,
    pub members: u64,
    pub unknown_ownership: u64,
    pub capabilities: crate::catalog::Capabilities,
    pub items: Vec<BrowseEntry>,
    pub vocabulary: Option<QueryVocabulary>,
    pub total: u64,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct EvidenceFamilies(Vec<crate::retrieval::Family>);
impl std::ops::Deref for EvidenceFamilies {
    type Target = [crate::retrieval::Family];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'de> Deserialize<'de> for EvidenceFamilies {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let values = Vec::<crate::retrieval::Family>::deserialize(d)?;
        let unique = values
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if values.is_empty() || values.len() > 4 || unique.len() != values.len() {
            return Err(serde::de::Error::custom(
                "one to four distinct evidence families required",
            ));
        }
        Ok(Self(values))
    }
}
impl JsonSchema for EvidenceFamilies {
    fn schema_name() -> Cow<'static, str> {
        "EvidenceFamilies".into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        let item = g.subschema_for::<crate::retrieval::Family>();
        json_schema!({"type":"array","items":item,"minItems":1,"maxItems":4,"uniqueItems":true})
    }
}
fn evidence_families() -> EvidenceFamilies {
    EvidenceFamilies(vec![
        crate::retrieval::Family::DocumentationDeployment,
        crate::retrieval::Family::Scenario,
    ])
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchEvidenceRequest {
    pub library: Text<1, 500>,
    pub query: Text<1, 4000>,
    #[serde(default = "evidence_families")]
    pub families: EvidenceFamilies,
    #[serde(default)]
    pub intent: Option<crate::evidence::Intent>,
    #[serde(default)]
    pub subject: Option<crate::retrieval::Subject>,
    #[serde(default)]
    pub limit: Limit<100, 20>,
    #[serde(default)]
    pub cursor: Option<Text<0, 2048>>,
    #[serde(default)]
    pub expanded: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioSummary {
    pub context: crate::evidence::ContextStatus,
    pub intent: crate::evidence::Intent,
    pub checks: crate::evidence::Checks,
    pub extraction: String,
    pub requirements: u64,
    pub option_bindings: u64,
    pub omitted_options: u64,
    pub omitted_requirements: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceHit {
    pub unit: RetrievalUnitHeader,
    pub intent: Option<crate::evidence::Intent>,
    pub scenario: Option<ScenarioSummary>,
    pub winners: Vec<crate::retrieval::UnitWinner>,
    pub score: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchEvidenceResponse {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub items: Vec<EvidenceHit>,
    pub total: u64,
    pub next_cursor: Option<String>,
    pub retrieval: RetrievalMetadata,
}
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct ComparisonCandidates(pub Vec<Text<1, 500>>);
impl<'de> Deserialize<'de> for ComparisonCandidates {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let values = Vec::<Text<1, 500>>::deserialize(d)?;
        if !(1..=5).contains(&values.len()) {
            return Err(serde::de::Error::custom(
                "compare requires 1..=5 candidates",
            ));
        }
        Ok(Self(values))
    }
}
impl JsonSchema for ComparisonCandidates {
    fn schema_name() -> Cow<'static, str> {
        "ComparisonCandidates".into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        let item = g.subschema_for::<Text<1, 500>>();
        json_schema!({"type":"array", "items":item, "minItems":1, "maxItems":5})
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareOperationsRequest {
    pub library: Text<1, 500>,
    pub candidates: ComparisonCandidates,
    pub selection: Selection,
    #[serde(default)]
    pub expanded: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "resolution", rename_all = "snake_case")]
pub enum ComparedOperation {
    Resolved {
        requested: String,
        selection: Box<CandidateSelection>,
        signatures: Vec<CatalogSignature>,
        packet: GetOperationRequest,
    },
    Ambiguous {
        requested: String,
        choices: Vec<CatalogMember>,
    },
    NotFound {
        requested: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompareOperationsResponse {
    pub snapshot_id: SnapshotId,
    pub generation: GenerationDigest,
    pub candidates: Vec<ComparedOperation>,
}
