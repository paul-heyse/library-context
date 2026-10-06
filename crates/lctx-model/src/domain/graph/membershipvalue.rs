use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MembershipValue {
    RetainedReleaseDeployment(crate::domain::catalog::evidence::ReleaseDeployment),
    RetainedScenarioSpan(crate::domain::catalog::evidence::ScenarioSpan),
    NativeFlowUseInventoryMember(crate::domain::flow_inventory::FlowUseInventoryMember),
    RetainedEffectiveDecoratorMember(
        crate::domain::normalized::callables::EffectiveDecoratorMember,
    ),
    RetainedBriefAssertion(crate::domain::synthesis::briefs::BriefAssertion),
    RetainedBriefDocument(crate::domain::synthesis::briefs::BriefDocument),
    RetainedBriefSource(crate::domain::synthesis::briefs::BriefSource),

    TypeSequenceMember(crate::domain::types::TypeSequenceMember),
    SymbolSequenceMember(crate::domain::symbols::SymbolSequenceMember),
    AssumptionMember(crate::domain::assumptions::AssumptionSetMember),
    LiteralMember(crate::domain::value::LiteralSetMember),
    AnalyticUniverseMember(crate::domain::analytics::UniverseMember),
    AnalyticPublicSelector(crate::domain::analytics::PublicSelector),
    AnalyticPartitionMember(crate::domain::analytics::PartitionMember),
    AnalyticCommunityMember(crate::domain::analytics::CommunityMember),
    AnalyticConceptObject(crate::domain::analytics::ConceptObject),
    AnalyticConceptExtent(crate::domain::analytics::ConceptExtent),
    AnalyticConceptIntent(crate::domain::analytics::ConceptIntent),
    AnalyticImplicationMember(crate::domain::analytics::ImplicationMember),
    SelectionDomainContext(crate::domain::selection::DomainContext),
    SelectionDomainClosure(crate::domain::selection::DomainClosure),
    SelectionDomainEvidence(crate::domain::selection::DomainEvidence),
    SignatureSlotType(crate::domain::normalized::callables::SignatureSlotType),
    SignatureReturnType(crate::domain::normalized::callables::SignatureReturnType),
}
impl Key for MembershipValue {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::RetainedReleaseDeployment(row) => {
                sink.part(b"variant", &17u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedScenarioSpan(row) => {
                sink.part(b"variant", &18u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::NativeFlowUseInventoryMember(row) => {
                sink.part(b"variant", &19u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedEffectiveDecoratorMember(row) => {
                sink.part(b"variant", &20u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedBriefAssertion(row) => {
                sink.part(b"variant", &21u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedBriefDocument(row) => {
                sink.part(b"variant", &22u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::RetainedBriefSource(row) => {
                sink.part(b"variant", &23u16.to_le_bytes());
                row.content_digest().encode(sink);
            }

            Self::SignatureSlotType(row) => {
                sink.part(b"variant", &15u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SignatureReturnType(row) => {
                sink.part(b"variant", &16u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionDomainContext(row) => {
                sink.part(b"variant", &12u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionDomainClosure(row) => {
                sink.part(b"variant", &13u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SelectionDomainEvidence(row) => {
                sink.part(b"variant", &14u16.to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::TypeSequenceMember(row) => {
                sink.part(b"variant", &(0u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::SymbolSequenceMember(row) => {
                sink.part(b"variant", &(1u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AssumptionMember(row) => {
                sink.part(b"variant", &(2u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::LiteralMember(row) => {
                sink.part(b"variant", &(3u16).to_le_bytes());
                row.content_digest().encode(sink);
            }

            Self::AnalyticUniverseMember(row) => {
                sink.part(b"variant", &(4u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticPublicSelector(row) => {
                sink.part(b"variant", &(5u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticPartitionMember(row) => {
                sink.part(b"variant", &(6u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticCommunityMember(row) => {
                sink.part(b"variant", &(7u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticConceptObject(row) => {
                sink.part(b"variant", &(8u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticConceptExtent(row) => {
                sink.part(b"variant", &(9u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticConceptIntent(row) => {
                sink.part(b"variant", &(10u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
            Self::AnalyticImplicationMember(row) => {
                sink.part(b"variant", &(11u16).to_le_bytes());
                row.content_digest().encode(sink);
            }
        }
    }
}
impl MembershipValue {
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Self::RetainedReleaseDeployment(row) => row.validate(),
            Self::RetainedScenarioSpan(row) => row.validate(),
            Self::NativeFlowUseInventoryMember(row) => row.validate(),
            Self::RetainedEffectiveDecoratorMember(row) => row.validate(),
            Self::RetainedBriefAssertion(row) => row.validate(),
            Self::RetainedBriefDocument(row) => row.validate(),
            Self::RetainedBriefSource(row) => row.validate(),

            Self::SignatureSlotType(row) => row.validate(),
            Self::SignatureReturnType(row) => row.validate(),
            Self::SelectionDomainContext(row) => row.validate(),
            Self::SelectionDomainClosure(row) => row.validate(),
            Self::SelectionDomainEvidence(row) => row.validate(),
            Self::TypeSequenceMember(row) => row.validate(),
            Self::SymbolSequenceMember(row) => row.validate(),
            Self::AssumptionMember(row) => row.validate(),
            Self::LiteralMember(row) => row.validate(),

            Self::AnalyticUniverseMember(row) => row.validate(),
            Self::AnalyticPublicSelector(row) => row.validate(),
            Self::AnalyticPartitionMember(row) => row.validate(),
            Self::AnalyticCommunityMember(row) => row.validate(),
            Self::AnalyticConceptObject(row) => row.validate(),
            Self::AnalyticConceptExtent(row) => row.validate(),
            Self::AnalyticConceptIntent(row) => row.validate(),
            Self::AnalyticImplicationMember(row) => row.validate(),
        }
    }
    pub fn references(&self) -> Vec<super::super::SemanticReference> {
        match self {
            Self::RetainedReleaseDeployment(row) => row.references(),
            Self::RetainedScenarioSpan(row) => row.references(),
            Self::NativeFlowUseInventoryMember(row) => row.references(),
            Self::RetainedEffectiveDecoratorMember(row) => row.references(),
            Self::RetainedBriefAssertion(row) => row.references(),
            Self::RetainedBriefDocument(row) => row.references(),
            Self::RetainedBriefSource(row) => row.references(),

            Self::SignatureSlotType(row) => row.references(),
            Self::SignatureReturnType(row) => row.references(),
            Self::SelectionDomainContext(row) => row.references(),
            Self::SelectionDomainClosure(row) => row.references(),
            Self::SelectionDomainEvidence(row) => row.references(),
            Self::TypeSequenceMember(row) => row.references(),
            Self::SymbolSequenceMember(row) => row.references(),
            Self::AssumptionMember(row) => row.references(),
            Self::LiteralMember(row) => row.references(),

            Self::AnalyticUniverseMember(row) => row.references(),
            Self::AnalyticPublicSelector(row) => row.references(),
            Self::AnalyticPartitionMember(row) => row.references(),
            Self::AnalyticCommunityMember(row) => row.references(),
            Self::AnalyticConceptObject(row) => row.references(),
            Self::AnalyticConceptExtent(row) => row.references(),
            Self::AnalyticConceptIntent(row) => row.references(),
            Self::AnalyticImplicationMember(row) => row.references(),
        }
    }
    pub fn semantic_key(&self) -> SemanticKey {
        match self {
            Self::RetainedReleaseDeployment(row) => SemanticKey::of(row.id()),
            Self::RetainedScenarioSpan(row) => SemanticKey::of(row.id()),
            Self::NativeFlowUseInventoryMember(row) => SemanticKey::of(row.id()),
            Self::RetainedEffectiveDecoratorMember(row) => SemanticKey::of(row.id()),
            Self::RetainedBriefAssertion(row) => SemanticKey::of(row.id()),
            Self::RetainedBriefDocument(row) => SemanticKey::of(row.id()),
            Self::RetainedBriefSource(row) => SemanticKey::of(row.id()),

            Self::SignatureSlotType(row) => SemanticKey::of(row.id()),
            Self::SignatureReturnType(row) => SemanticKey::of(row.id()),
            Self::SelectionDomainContext(row) => SemanticKey::of(row.id()),
            Self::SelectionDomainClosure(row) => SemanticKey::of(row.id()),
            Self::SelectionDomainEvidence(row) => SemanticKey::of(row.id()),
            Self::TypeSequenceMember(row) => SemanticKey::of(row.id()),
            Self::SymbolSequenceMember(row) => SemanticKey::of(row.id()),
            Self::AssumptionMember(row) => SemanticKey::of(row.id()),
            Self::LiteralMember(row) => SemanticKey::of(row.id()),

            Self::AnalyticUniverseMember(row) => SemanticKey::of(row.id()),
            Self::AnalyticPublicSelector(row) => SemanticKey::of(row.id()),
            Self::AnalyticPartitionMember(row) => SemanticKey::of(row.id()),
            Self::AnalyticCommunityMember(row) => SemanticKey::of(row.id()),
            Self::AnalyticConceptObject(row) => SemanticKey::of(row.id()),
            Self::AnalyticConceptExtent(row) => SemanticKey::of(row.id()),
            Self::AnalyticConceptIntent(row) => SemanticKey::of(row.id()),
            Self::AnalyticImplicationMember(row) => SemanticKey::of(row.id()),
        }
    }
}
