use super::*;

/// Selected typed semantic payloads. Participant references are lowered into graph roles by the
/// compiler; these native/result values retain their owner's exact data and codebook meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AnalysisValue {
    RetrievalEmbeddingUse(crate::domain::retrieval::consumption::RetrievalEmbeddingUse),
    RetrievalUnitRoot(crate::domain::retrieval::UnitRoot),
    LocalAssessment(crate::domain::local_semantics::LocalAssessment),
    LocalTransfer(crate::domain::local_semantics::LocalContribution),
    LocalGuard(crate::domain::local_semantics::LocalGuardContribution),
    Theory(crate::domain::local_theory::TheoryWitness),
    ExpressionEvaluation(crate::domain::execution::records::ExpressionEvaluation),
    SourceCall(crate::domain::execution::source_call_records::SourceCallHeader),
    SourceInvocation(crate::domain::execution::source_call_records::SourceInvocation),
    ModelTransfer(crate::domain::execution::model_transfer::ModelTransferWitness),
    SummaryClaim(crate::domain::execution::summary_consequences::SummaryClaim),
    SummaryPath(crate::domain::execution::summary_path::SummaryPathWitness),
    SummaryRun(crate::domain::execution::summary_production::SummaryRun),
    NormalizedSymbol(crate::domain::normalized::entities::SymbolEntityResolution),
    PublicEnumeration(crate::domain::normalized::entities::PublicEnumerationAssessment),
    BindingAttempt(crate::domain::normalized::bindings::CallBindingAttempt),
    Binding(crate::domain::normalized::bindings::CallBinding),
    BindingVariant(crate::domain::normalized::bindings::BindingVariantAssessment),
    CatalogInvocation(crate::domain::catalog::CatalogInvocation),
    CatalogPath(crate::domain::catalog::CatalogPath),
    CatalogAlias(crate::domain::catalog::CatalogAlias),
    CatalogConstructor(crate::domain::catalog::CatalogConstructor),
    ScenarioAssociation(crate::domain::catalog::evidence::ScenarioAssociation),
    DocumentAssociation(crate::domain::catalog::evidence::DocumentAssociation),
    Deployment(crate::domain::catalog::evidence::CatalogDeployment),
    ScenarioCheck(crate::domain::catalog::evidence::ScenarioCheck),
    SelectionDomain(crate::domain::selection::SelectionDomain),
    StructuralConclusion(crate::domain::structural::Conclusion),
    StructuralUsage(crate::domain::structural::UsageScore),
    StructuralControl(crate::domain::structural::controls::ControlTraversal),
    AnalyticConclusion(crate::domain::analytics::Conclusion),
    AnalyticTechnique(crate::domain::analytics::TechniqueResult),
    Rank(crate::domain::analytics::RankScore),
    Community(crate::domain::analytics::Community),
    Neighbour(crate::domain::analytics::Neighbour),
    Concept(crate::domain::analytics::Concept),
    Implication(crate::domain::analytics::Implication),
    DocumentaryConclusion(crate::domain::synthesis::documentary::DocumentaryConclusion),
    ProgrammaticAssertion(crate::domain::synthesis::assertions::ProgrammaticAssertion),
    Brief(crate::domain::synthesis::briefs::Brief),
    RetrievalAnchor(crate::domain::retrieval::OriginalAnchor),
    RetrievalOccurrence(crate::domain::retrieval::UnitSubject),
    AnalyticLayerPair(crate::domain::analytics::LayerPair),
    AnalyticCombinedPair(crate::domain::analytics::CombinedPair),
    AnalyticCommunityRun(crate::domain::analytics::CommunityRun),
    AnalyticCommunityProfile(crate::domain::analytics::CommunityProfile),
    AnalyticVectorSelection(crate::domain::analytics::VectorSelection),
    AnalyticCommunityLabelAssessment(crate::domain::analytics::CommunityLabelAssessment),
    AnalyticLayerResult(crate::domain::analytics::LayerResult),
    AnalyticLayerNeighbour(crate::domain::analytics::LayerNeighbour),
    SelectionContext(crate::domain::selection::Context),
    SelectionWitness(crate::domain::selection::Witness),
}
impl Key for AnalysisValue {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::SelectionContext(row)=>{sink.part(b"variant",&56u16.to_le_bytes());row.content_digest().encode(sink);},
            Self::SelectionWitness(row)=>{sink.part(b"variant",&57u16.to_le_bytes());row.content_digest().encode(sink);},
            Self::RetrievalEmbeddingUse(row)=>{sink.part(b"variant",&54u16.to_le_bytes());row.content_digest().encode(sink);},
            Self::RetrievalUnitRoot(row)=>{sink.part(b"variant",&55u16.to_le_bytes());row.content_digest().encode(sink);},
            Self::LocalAssessment(row) => {
                sink.part(b"variant", &(0u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalTransfer(row) => {
                sink.part(b"variant", &(1u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LocalGuard(row) => {
                sink.part(b"variant", &(2u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Theory(row) => {
                sink.part(b"variant", &(3u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ExpressionEvaluation(row) => {
                sink.part(b"variant", &(4u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceCall(row) => {
                sink.part(b"variant", &(5u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SourceInvocation(row) => {
                sink.part(b"variant", &(6u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ModelTransfer(row) => {
                sink.part(b"variant", &(7u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryClaim(row) => {
                sink.part(b"variant", &(8u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryPath(row) => {
                sink.part(b"variant", &(9u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SummaryRun(row) => {
                sink.part(b"variant", &(10u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NormalizedSymbol(row) => {
                sink.part(b"variant", &(11u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::PublicEnumeration(row) => {
                sink.part(b"variant", &(12u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BindingAttempt(row) => {
                sink.part(b"variant", &(13u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Binding(row) => {
                sink.part(b"variant", &(14u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::BindingVariant(row) => {
                sink.part(b"variant", &(15u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogInvocation(row) => {
                sink.part(b"variant", &(17u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogPath(row) => {
                sink.part(b"variant", &(19u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogAlias(row) => {
                sink.part(b"variant", &(20u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::CatalogConstructor(row) => {
                sink.part(b"variant", &(21u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ScenarioAssociation(row) => {
                sink.part(b"variant", &(23u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentAssociation(row) => {
                sink.part(b"variant", &(24u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Deployment(row) => {
                sink.part(b"variant", &(25u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ScenarioCheck(row) => {
                sink.part(b"variant", &(26u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionDomain(row) => {
                sink.part(b"variant", &(27u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralConclusion(row) => {
                sink.part(b"variant", &(30u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralUsage(row) => {
                sink.part(b"variant", &(31u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::StructuralControl(row) => {
                sink.part(b"variant", &(32u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticConclusion(row) => {
                sink.part(b"variant", &(33u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticTechnique(row) => {
                sink.part(b"variant", &(34u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Rank(row) => {
                sink.part(b"variant", &(35u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Community(row) => {
                sink.part(b"variant", &(36u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Neighbour(row) => {
                sink.part(b"variant", &(37u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Concept(row) => {
                sink.part(b"variant", &(38u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Implication(row) => {
                sink.part(b"variant", &(39u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::DocumentaryConclusion(row) => {
                sink.part(b"variant", &(40u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::ProgrammaticAssertion(row) => {
                sink.part(b"variant", &(41u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::Brief(row) => {
                sink.part(b"variant", &(42u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalAnchor(row) => {
                sink.part(b"variant", &(44u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetrievalOccurrence(row) => {
                sink.part(b"variant", &(45u16).to_le_bytes());
                row.content_digest().encode(sink);
            }

            Self::AnalyticLayerPair(row) => {
                sink.part(b"variant", &(46u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticCombinedPair(row) => {
                sink.part(b"variant", &(47u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticCommunityRun(row) => {
                sink.part(b"variant", &(48u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticCommunityProfile(row) => {
                sink.part(b"variant", &(49u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticVectorSelection(row) => {
                sink.part(b"variant", &(50u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticCommunityLabelAssessment(row) => {
                sink.part(b"variant", &(51u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticLayerResult(row) => {
                sink.part(b"variant", &(52u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticLayerNeighbour(row) => {
                sink.part(b"variant", &(53u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
        }
    }
}
impl AnalysisValue {
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Self::SelectionContext(row)=>row.validate(),
            Self::SelectionWitness(row)=>row.validate(),
            Self::RetrievalEmbeddingUse(row)=>row.validate(),
            Self::RetrievalUnitRoot(row)=>row.validate(),
            Self::LocalAssessment(row) => row.validate(),
            Self::LocalTransfer(row) => row.validate(),
            Self::LocalGuard(row) => row.validate(),
            Self::Theory(row) => row.validate(),
            Self::ExpressionEvaluation(row) => row.validate(),
            Self::SourceCall(row) => row.validate(),
            Self::SourceInvocation(row) => row.validate(),
            Self::ModelTransfer(row) => row.validate(),
            Self::SummaryClaim(row) => row.validate(),
            Self::SummaryPath(row) => row.validate(),
            Self::SummaryRun(row) => row.validate(),
            Self::NormalizedSymbol(row) => row.validate(),
            Self::PublicEnumeration(row) => row.validate(),
            Self::BindingAttempt(row) => row.validate(),
            Self::Binding(row) => row.validate(),
            Self::BindingVariant(row) => row.validate(),
            Self::CatalogInvocation(row) => row.validate(),
            Self::CatalogPath(row) => row.validate(),
            Self::CatalogAlias(row) => row.validate(),
            Self::CatalogConstructor(row) => row.validate(),
            Self::ScenarioAssociation(row) => row.validate(),
            Self::DocumentAssociation(row) => row.validate(),
            Self::Deployment(row) => row.validate(),
            Self::ScenarioCheck(row) => row.validate(),
            Self::SelectionDomain(row) => row.validate(),
            Self::StructuralConclusion(row) => row.validate(),
            Self::StructuralUsage(row) => row.validate(),
            Self::StructuralControl(row) => row.validate(),
            Self::AnalyticConclusion(row) => row.validate(),
            Self::AnalyticTechnique(row) => row.validate(),
            Self::Rank(row) => row.validate(),
            Self::Community(row) => row.validate(),
            Self::Neighbour(row) => row.validate(),
            Self::Concept(row) => row.validate(),
            Self::Implication(row) => row.validate(),
            Self::DocumentaryConclusion(row) => row.validate(),
            Self::ProgrammaticAssertion(row) => row.validate(),
            Self::Brief(row) => row.validate(),
            Self::RetrievalAnchor(row) => row.validate(),
            Self::RetrievalOccurrence(row) => row.validate(),

            Self::AnalyticLayerPair(row) => row.validate(),
            Self::AnalyticCombinedPair(row) => row.validate(),
            Self::AnalyticCommunityRun(row) => row.validate(),
            Self::AnalyticCommunityProfile(row) => row.validate(),
            Self::AnalyticVectorSelection(row) => row.validate(),
            Self::AnalyticCommunityLabelAssessment(row) => row.validate(),
            Self::AnalyticLayerResult(row) => row.validate(),
            Self::AnalyticLayerNeighbour(row) => row.validate(),
        }
    }
}

impl AnalysisValue {
    pub fn references(&self) -> Vec<super::super::SemanticReference> {
        match self {
            Self::SelectionContext(row)=>row.references(),
            Self::SelectionWitness(row)=>row.references(),
            Self::RetrievalEmbeddingUse(row)=>row.references(),
            Self::RetrievalUnitRoot(row)=>row.references(),
            Self::LocalAssessment(row) => row.references(),
            Self::LocalTransfer(row) => row.references(),
            Self::LocalGuard(row) => row.references(),
            Self::Theory(row) => row.references(),
            Self::ExpressionEvaluation(row) => row.references(),
            Self::SourceCall(row) => row.references(),
            Self::SourceInvocation(row) => row.references(),
            Self::ModelTransfer(row) => row.references(),
            Self::SummaryClaim(row) => row.references(),
            Self::SummaryPath(row) => row.references(),
            Self::SummaryRun(row) => row.references(),
            Self::NormalizedSymbol(row) => row.references(),
            Self::PublicEnumeration(row) => row.references(),
            Self::BindingAttempt(row) => row.references(),
            Self::Binding(row) => row.references(),
            Self::BindingVariant(row) => row.references(),
            Self::CatalogInvocation(row) => row.references(),
            Self::CatalogPath(row) => row.references(),
            Self::CatalogAlias(row) => row.references(),
            Self::CatalogConstructor(row) => row.references(),
            Self::ScenarioAssociation(row) => row.references(),
            Self::DocumentAssociation(row) => row.references(),
            Self::Deployment(row) => row.references(),
            Self::ScenarioCheck(row) => row.references(),
            Self::SelectionDomain(row) => row.references(),
            Self::StructuralConclusion(row) => row.references(),
            Self::StructuralUsage(row) => row.references(),
            Self::StructuralControl(row) => row.references(),
            Self::AnalyticConclusion(row) => row.references(),
            Self::AnalyticTechnique(row) => row.references(),
            Self::Rank(row) => row.references(),
            Self::Community(row) => row.references(),
            Self::Neighbour(row) => row.references(),
            Self::Concept(row) => row.references(),
            Self::Implication(row) => row.references(),
            Self::DocumentaryConclusion(row) => row.references(),
            Self::ProgrammaticAssertion(row) => row.references(),
            Self::Brief(row) => row.references(),
            Self::RetrievalAnchor(row) => row.references(),
            Self::RetrievalOccurrence(row) => row.references(),

            Self::AnalyticLayerPair(row) => row.references(),
            Self::AnalyticCombinedPair(row) => row.references(),
            Self::AnalyticCommunityRun(row) => row.references(),
            Self::AnalyticCommunityProfile(row) => row.references(),
            Self::AnalyticVectorSelection(row) => row.references(),
            Self::AnalyticCommunityLabelAssessment(row) => row.references(),
            Self::AnalyticLayerResult(row) => row.references(),
            Self::AnalyticLayerNeighbour(row) => row.references(),
        }
    }
}

impl AnalysisValue {
    pub fn semantic_key(&self) -> super::SemanticKey {
        match self {
            Self::SelectionContext(row)=>SemanticKey::of(row.id()),
            Self::SelectionWitness(row)=>SemanticKey::of(row.id()),
            Self::RetrievalEmbeddingUse(row)=>super::SemanticKey::of(row.id()),
            Self::RetrievalUnitRoot(row)=>super::SemanticKey::of(row.id()),
            Self::LocalAssessment(row) => super::SemanticKey::of(row.id()),
            Self::LocalTransfer(row) => super::SemanticKey::of(row.id()),
            Self::LocalGuard(row) => super::SemanticKey::of(row.id()),
            Self::Theory(row) => super::SemanticKey::of(row.id()),
            Self::ExpressionEvaluation(row) => super::SemanticKey::of(row.id()),
            Self::SourceCall(row) => super::SemanticKey::of(row.id()),
            Self::SourceInvocation(row) => super::SemanticKey::of(row.id()),
            Self::ModelTransfer(row) => super::SemanticKey::of(row.id()),
            Self::SummaryClaim(row) => super::SemanticKey::of(row.id()),
            Self::SummaryPath(row) => super::SemanticKey::of(row.id()),
            Self::SummaryRun(row) => super::SemanticKey::of(row.id()),
            Self::NormalizedSymbol(row) => super::SemanticKey::of(row.id()),
            Self::PublicEnumeration(row) => super::SemanticKey::of(row.id()),
            Self::BindingAttempt(row) => super::SemanticKey::of(row.id()),
            Self::Binding(row) => super::SemanticKey::of(row.id()),
            Self::BindingVariant(row) => super::SemanticKey::of(row.id()),
            Self::CatalogInvocation(row) => super::SemanticKey::of(row.id()),
            Self::CatalogPath(row) => super::SemanticKey::of(row.id()),
            Self::CatalogAlias(row) => super::SemanticKey::of(row.id()),
            Self::CatalogConstructor(row) => super::SemanticKey::of(row.id()),
            Self::ScenarioAssociation(row) => super::SemanticKey::of(row.id()),
            Self::DocumentAssociation(row) => super::SemanticKey::of(row.id()),
            Self::Deployment(row) => super::SemanticKey::of(row.id()),
            Self::ScenarioCheck(row) => super::SemanticKey::of(row.id()),
            Self::SelectionDomain(row) => super::SemanticKey::of(row.id()),
            Self::StructuralConclusion(row) => super::SemanticKey::of(row.id()),
            Self::StructuralUsage(row) => super::SemanticKey::of(row.id()),
            Self::StructuralControl(row) => super::SemanticKey::of(row.id()),
            Self::AnalyticConclusion(row) => super::SemanticKey::of(row.id()),
            Self::AnalyticTechnique(row) => super::SemanticKey::of(row.id()),
            Self::Rank(row) => super::SemanticKey::of(row.id()),
            Self::Community(row) => super::SemanticKey::of(row.id()),
            Self::Neighbour(row) => super::SemanticKey::of(row.id()),
            Self::Concept(row) => super::SemanticKey::of(row.id()),
            Self::Implication(row) => super::SemanticKey::of(row.id()),
            Self::DocumentaryConclusion(row) => super::SemanticKey::of(row.id()),
            Self::ProgrammaticAssertion(row) => super::SemanticKey::of(row.id()),
            Self::Brief(row) => super::SemanticKey::of(row.id()),
            Self::RetrievalAnchor(row) => super::SemanticKey::of(row.id()),
            Self::RetrievalOccurrence(row) => super::SemanticKey::of(row.id()),

            Self::AnalyticLayerPair(row) => SemanticKey::of(row.id()),
            Self::AnalyticCombinedPair(row) => SemanticKey::of(row.id()),
            Self::AnalyticCommunityRun(row) => SemanticKey::of(row.id()),
            Self::AnalyticCommunityProfile(row) => SemanticKey::of(row.id()),
            Self::AnalyticVectorSelection(row) => SemanticKey::of(row.id()),
            Self::AnalyticCommunityLabelAssessment(row) => SemanticKey::of(row.id()),
            Self::AnalyticLayerResult(row) => SemanticKey::of(row.id()),
            Self::AnalyticLayerNeighbour(row) => SemanticKey::of(row.id()),
        }
    }
}
