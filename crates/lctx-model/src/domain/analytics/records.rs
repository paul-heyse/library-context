//! Nominal algorithm inputs, selectors and complete result inventories. Labels never own membership.
use crate::domain::{
    analysis::{self, analytic as owner, settings::AnalyticsConfiguration},
    embedding::analytic::AnalysisEmbeddingUse,
    normalized::{entities::EntityRef, events::NormalizedCallAlternative},
    structural::{PublicCandidate, StructuralFrame, UsageEvidence},
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="analytic_frames",invariants=super::frames::invariants)]
pub struct AnalyticFrame {
    #[model(key)]
    pub structural: Id<StructuralFrame>,
    #[model(key)]
    pub configuration: Id<AnalyticsConfiguration>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Stop {
    NotRequested = 0,
    Converged = 1,
    EmptyDomain = 2,
    IterationLimit = 3,
    WorkLimit = 4,
    EnumerationLimit = 5,
    VectorsUnavailable = 6,
    DegeneratePartition = 7,
    InputPartial = 8,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_technique_results")]
pub struct TechniqueResult {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub method: analysis::AnalysisMethod,
    pub invocation: Id<owner::Invocation>,
    pub selected: bool,
    pub status: analysis::AnalysisStatus,
    pub stop: Stop,
    pub iterations: i64,
    pub examined: i64,
    pub residual: Option<FiniteF64>,
    pub input_partial: bool,
}
/// The complete stored graph domain is separate from the release scope and public output selector.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_universe_members")]
pub struct UniverseMember {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub entity: Id<EntityRef>,
    pub graph: bool,
    pub release_scope: bool,
    pub public: bool,
    pub community_touched: bool,
    pub excluded_isolate: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_public_selectors")]
pub struct PublicSelector {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub candidate: Id<PublicCandidate>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_graph_arcs")]
pub struct GraphArc {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub alternative: Id<NormalizedCallAlternative>,
    pub source: Id<EntityRef>,
    pub target: Id<EntityRef>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Layer {
    Invocation = 0,
    CoUse = 1,
    Type = 2,
    Mention = 3,
    Nearest = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "analytic_pair_sources")]
pub enum PairSource {
    #[model(code = 0)]
    Invocation { arc: Id<GraphArc> },
    #[model(code = 1)]
    CoUse {
        scope: Id<source::Occurrence>,
        left: Id<UsageEvidence>,
        right: Id<UsageEvidence>,
    },
    #[model(code = 2)]
    Type {
        left: Id<types::TypeObservation>,
        right: Id<types::TypeObservation>,
        class: Id<EntityRef>,
    },
    #[model(code = 3)]
    Mention {
        left: Id<documents::DocumentMentionObservation>,
        right: Id<documents::DocumentMentionObservation>,
    },
    #[model(code = 4)]
    Nearest { neighbour: Id<Neighbour> },
    #[model(code = 5)]
    NearestLayer { neighbour: Id<LayerNeighbour> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_pair_contributions")]
pub struct PairContribution {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub layer: Layer,
    #[model(key)]
    pub source: Id<PairSource>,
    pub left: Id<EntityRef>,
    pub right: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_layer_pairs")]
pub struct LayerPair {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub layer: Layer,
    #[model(key)]
    pub left: Id<EntityRef>,
    #[model(key)]
    pub right: Id<EntityRef>,
    pub count: i64,
    pub normalized_weight: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_combined_pairs")]
pub struct CombinedPair {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub left: Id<EntityRef>,
    #[model(key)]
    pub right: Id<EntityRef>,
    pub weight: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_rank_scores")]
pub struct RankScore {
    #[model(key)]
    pub result: Id<TechniqueResult>,
    #[model(key)]
    pub target: Id<EntityRef>,
    pub score: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_community_runs")]
pub struct CommunityRun {
    #[model(key)]
    pub result: Id<TechniqueResult>,
    #[model(key)]
    pub resolution: FiniteF64,
    #[model(key)]
    pub seed: i64,
    pub iterations: i64,
    pub converged: bool,
    pub history: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_partition_members")]
pub struct PartitionMember {
    #[model(key)]
    pub run: Id<CommunityRun>,
    #[model(key)]
    pub entity: Id<EntityRef>,
    pub representative: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_community_profiles")]
pub struct CommunityProfile {
    #[model(key)]
    pub result: Id<TechniqueResult>,
    #[model(key)]
    pub resolution: FiniteF64,
    pub mean_ari: FiniteF64,
    pub sd_ari: FiniteF64,
    pub mean_nmi: FiniteF64,
    pub min_nmi: FiniteF64,
    pub largest: i64,
    pub communities: i64,
    pub degenerate: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_communities")]
pub struct Community {
    #[model(key)]
    pub result: Id<TechniqueResult>,
    #[model(key)]
    pub representative: Id<EntityRef>,
    pub agreement: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_community_members")]
pub struct CommunityMember {
    #[model(key)]
    pub community: Id<Community>,
    #[model(key)]
    pub entity: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_neighbours")]
pub struct Neighbour {
    #[model(key)]
    pub result: Id<TechniqueResult>,
    #[model(key)]
    pub query: Id<EntityRef>,
    #[model(key)]
    pub target: Id<EntityRef>,
    pub query_use: Id<AnalysisEmbeddingUse>,
    pub target_use: Id<AnalysisEmbeddingUse>,
    pub score: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_vector_selections")]
pub struct VectorSelection {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    pub entity: Option<Id<EntityRef>>,
    #[model(key)]
    pub subject: Id<embedding::text::TextSubject>,
    pub invocation: Id<analysis::analytic_embedding::Invocation>,
    pub expected_windows: i64,
    pub available_windows: i64,
}
/// A scope is the exact catalog access namespace; namespace strings are catalog-owned paths.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_concept_scopes")]
pub struct ConceptScope {
    #[model(key)]
    pub result: Id<TechniqueResult>,
    #[model(key)]
    pub access: Id<source::Module>,
    #[model(key)]
    pub namespace: Vec<String>,
    pub examined: i64,
    pub partial: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_concept_objects")]
pub struct ConceptObject {
    #[model(key)]
    pub scope: Id<ConceptScope>,
    #[model(key)]
    pub entity: Id<EntityRef>,
    #[model(key)]
    pub candidate: Id<PublicCandidate>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "analytic_attributes")]
pub enum Attribute {
    #[model(code = 0)]
    Parameter {
        name: Utf8Text,
        kind: calls::ParameterKind,
    },
    #[model(code = 1)]
    ParameterType { term: Id<types::TypeTerm> },
    #[model(code = 2)]
    Returns { term: Id<types::TypeTerm> },
    #[model(code = 3)]
    Raises { class: Id<EntityRef> },
    #[model(code = 4)]
    Decorator { expression: Utf8Text },
    #[model(code = 5)]
    Calls {
        target: Id<EntityRef>,
        modality: attribution::Modality,
        phase: calls::CallPhase,
    },
    #[model(code = 6)]
    Handoff {
        takes: bool,
        target: Id<EntityRef>,
        producer_modality: attribution::Modality,
        consumer_modality: attribution::Modality,
        producer_phase: calls::CallPhase,
        consumer_phase: calls::CallPhase,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "analytic_incidence_sources")]
pub enum IncidenceSource {
    #[model(code = 0)]
    Parameter {
        observation: Id<syntax::ParameterSyntaxObservation>,
    },
    #[model(code = 1)]
    Type {
        observation: Id<types::TypeObservation>,
    },
    #[model(code = 2)]
    Decorator {
        observation: Id<syntax::DeclarationDecorator>,
    },
    #[model(code = 3)]
    Call { arc: Id<GraphArc> },
    #[model(code = 4)]
    Handoff {
        occurrence: Id<structural::handoffs::Handoff>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_incidences")]
pub struct Incidence {
    #[model(key)]
    pub scope: Id<ConceptScope>,
    #[model(key)]
    pub entity: Id<EntityRef>,
    #[model(key)]
    pub attribute: Id<Attribute>,
    #[model(key)]
    pub source: Id<IncidenceSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_concepts")]
pub struct Concept {
    #[model(key)]
    pub scope: Id<ConceptScope>,
    #[model(key)]
    pub extent: ContentHash,
    #[model(key)]
    pub intent: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_implications")]
pub struct Implication {
    #[model(key)]
    pub scope: Id<ConceptScope>,
    #[model(key)]
    pub premise: ContentHash,
    #[model(key)]
    pub conclusion: ContentHash,
    pub support: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_quality_steps")]
pub struct QualityStep {
    #[model(key)]
    pub run: Id<CommunityRun>,
    #[model(key)]
    pub ordinal: i64,
    pub value: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_concept_extents")]
pub struct ConceptExtent {
    #[model(key)]
    pub concept: Id<Concept>,
    #[model(key)]
    pub entity: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_concept_intents")]
pub struct ConceptIntent {
    #[model(key)]
    pub concept: Id<Concept>,
    #[model(key)]
    pub attribute: Id<Attribute>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_implication_members")]
pub struct ImplicationMember {
    #[model(key)]
    pub implication: Id<Implication>,
    #[model(key)]
    pub premise: bool,
    #[model(key)]
    pub attribute: Id<Attribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_document_neighbours")]
pub struct DocumentNeighbour {
    #[model(key)]
    pub result: Id<TechniqueResult>,
    #[model(key)]
    pub query: Id<EntityRef>,
    #[model(key)]
    pub subject: Id<embedding::text::TextSubject>,
    pub query_use: Id<AnalysisEmbeddingUse>,
    pub target_use: Id<AnalysisEmbeddingUse>,
    pub score: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_community_labels")]
pub struct CommunityLabel {
    #[model(key)]
    pub community: Id<Community>,
    pub subject: Id<embedding::text::TextSubject>,
    pub target_use: Id<AnalysisEmbeddingUse>,
    pub members: ContentHash,
    pub score: FiniteF64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_community_label_members")]
pub struct CommunityLabelMember {
    #[model(key)]
    pub label: Id<CommunityLabel>,
    #[model(key)]
    pub use_: Id<AnalysisEmbeddingUse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_community_label_assessments")]
pub struct CommunityLabelAssessment {
    #[model(key)]
    pub community: Id<Community>,
    pub status: analysis::AnalysisStatus,
    pub stop: Stop,
}

/// Community layer availability belongs to its actual Communities invocation. It does not
/// activate the separately optional public nearest-neighbour/doc-link technique.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_layer_results")]
pub struct LayerResult {
    #[model(key)]
    pub frame: Id<AnalyticFrame>,
    #[model(key)]
    pub layer: Layer,
    pub invocation: Id<owner::Invocation>,
    pub status: analysis::AnalysisStatus,
    pub stop: Stop,
    pub examined: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analytic_layer_neighbours")]
pub struct LayerNeighbour {
    #[model(key)]
    pub result: Id<LayerResult>,
    #[model(key)]
    pub query: Id<EntityRef>,
    #[model(key)]
    pub target: Id<EntityRef>,
    pub query_use: Id<AnalysisEmbeddingUse>,
    pub target_use: Id<AnalysisEmbeddingUse>,
    pub score: FiniteF64,
}
