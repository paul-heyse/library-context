//! Finite row views. Fields and nominal targets come from their canonical declaration.
use super::identity::MappingIdentity;
use crate::domain::{
    self, ContentHash, Field, Key, KeySink, Record, Relation, Scalar, admission::Frontier,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Catalog,
    Briefs,
    Native,
}
#[derive(Debug, Clone)]
pub struct Mapping {
    pub name: &'static str,
    pub source: Relation,
    pub dependencies: Vec<&'static str>,
    pub minimum_frontier: Frontier,
    pub required_capabilities: &'static [Capability],
    pub output_type: &'static str,
    pub revision: u32,
    /// Canonical field names; the implicit record identity is always included.
    pub lookup_keys: Vec<&'static str>,
}
impl Mapping {
    pub fn of<T: Record>(name: &'static str, required_capabilities: &'static [Capability]) -> Self {
        let source = Relation::of::<T>();
        let mut dependencies = vec![T::NAME];
        dependencies.extend(
            source
                .fields()
                .iter()
                .filter_map(|f| f.target().map(|(_, n)| n)),
        );
        dependencies.sort_unstable();
        dependencies.dedup();
        // ID/content lookups do not index arbitrarily large source text or list keys.
        // Spelling/path navigation uses complete prepared metadata, not a lossy index constraint.
        let lookup_keys = source
            .fields()
            .iter()
            .filter(|f| {
                f.is_key() && !f.list() && !matches!(f.scalar(), Scalar::Text | Scalar::Binary)
            })
            .map(Field::name)
            .collect();
        Self {
            name,
            source,
            dependencies,
            minimum_frontier: Frontier::Catalog,
            required_capabilities,
            output_type: T::NAME,
            revision: 1,
            lookup_keys,
        }
    }
    pub fn fields(&self) -> &[Field] {
        self.source.fields()
    }
}
/// Every view is an identity-preserving canonical row projection, never a copied semantic table.
pub fn inventory() -> Vec<Mapping> {
    use domain::{catalog as c, catalog::evidence as e, retrieval as r};
    macro_rules! core { ($($name:literal:$ty:ty),* $(,)?) => { vec![$(Mapping::of::<$ty>($name,&[Capability::Catalog])),*] }; }
    let mut rows = core! {
        "serving_members":c::CatalogMember,
        "serving_exposures":c::CatalogExposure,
        "serving_candidates":c::CatalogCandidate,
        "serving_callables":c::CatalogCallable,
        "serving_invocations":c::CatalogInvocation,
        "serving_options":c::CatalogOption,
        "serving_defaults":c::CatalogDefault,
        "serving_option_subjects":c::CatalogOptionSubject,
        "serving_option_evidence":c::CatalogOptionEvidence,
        "serving_classes":c::CatalogClass,
        "serving_constructors":c::CatalogConstructor,
        "serving_paths":c::CatalogPath,
        "serving_signatures":domain::calls::Signature,
        "serving_parameters":domain::calls::SignatureParameter,
        "serving_signature_variants":domain::normalized::callables::SignatureVariant,
        "serving_signature_slots":domain::normalized::callables::SignatureSlot,
        "serving_originals":e::OriginalSource,
        "serving_artifacts":domain::source::SourceArtifact,
        "serving_chunks":domain::artifact::ArtifactChunk,
        "serving_scenarios":e::CatalogScenario,
        "serving_scenario_spans":e::ScenarioSpan,
        "serving_scenario_associations":e::ScenarioAssociation,
        "serving_deployments":e::CatalogDeployment,
        "serving_selection_domains":domain::selection::SelectionDomain,
        "serving_selection_contexts":domain::selection::Context,
        "serving_selection_witnesses":domain::selection::Witness,
        "serving_retrieval_units":r::Unit,
        "serving_retrieval_texts":r::CorpusText,
        "serving_retrieval_subjects":r::Subject,
        "serving_retrieval_unit_subjects":r::UnitSubject,
        "serving_retrieval_anchors":r::OriginalAnchor,
        "serving_retrieval_anchor_sources":r::AnchorSource,
        "serving_embedding_uses":r::consumption::RetrievalEmbeddingUse,
    };
    rows.push(Mapping::of::<domain::synthesis::briefs::Brief>(
        "serving_briefs",
        &[Capability::Catalog, Capability::Briefs],
    ));
    rows.push(Mapping::of::<domain::synthesis::briefs::BriefDocument>(
        "serving_brief_documents",
        &[Capability::Catalog, Capability::Briefs],
    ));
    rows
}
pub fn identity() -> MappingIdentity {
    let mut sink = KeySink::new("serving-mappings/v1");
    identity_for(&inventory()).0.encode(&mut sink);
    for mapping in packet_inventory() {
        sink.part(b"transformation", mapping.name.as_bytes());
        sink.part(b"output", mapping.output_type.as_bytes());
        sink.part(b"revision", &mapping.revision.to_le_bytes());
        sink.part(b"frontier", mapping.minimum_frontier.name().as_bytes());
        for capability in mapping.required_capabilities {
            sink.part(b"capability", format!("{capability:?}").as_bytes());
        }
        for relation in mapping.sources {
            sink.part(b"source", relation.name().as_bytes());
            for field in relation.schema().fields() {
                sink.part(b"field", format!("{field:?}").as_bytes());
            }
        }
    }
    MappingIdentity(sink.finish())
}
pub fn identity_for(mappings: &[Mapping]) -> MappingIdentity {
    let mut sink = KeySink::new("serving-mapping/v1");
    for m in mappings {
        m.name.to_owned().encode(&mut sink);
        m.source.name().to_owned().encode(&mut sink);
        sink.part(b"revision", &m.revision.to_le_bytes());
        sink.part(b"frontier", m.minimum_frontier.name().as_bytes());
        for c in m.required_capabilities {
            sink.part(b"capability", format!("{c:?}").as_bytes());
        }
        for field in m.source.schema().fields() {
            sink.part(b"field", format!("{field:?}").as_bytes());
        }
        // Sum arms, nominal targets and codebooks are not all represented by the Arrow datatype.
        for f in m.fields() {
            sink.part(
                b"field-key",
                &[u8::from(f.is_key()), u8::from(f.is_provenance())],
            );
            if let Some((_, name)) = f.target() {
                sink.part(b"target", name.as_bytes());
            }
            if let Some(code) = f.subtype() {
                sink.part(b"subtype", &code.to_le_bytes());
            }
            for (code, name) in f.codes() {
                sink.part(b"code", &code.to_le_bytes());
                sink.part(b"name", name.as_bytes());
            }
        }
        if let Some(sum) = m.source.sum() {
            sink.part(b"sum", format!("{sum:?}").as_bytes());
        }
        for dep in &m.dependencies {
            sink.part(b"dependency", dep.as_bytes());
        }
        sink.part(b"output", m.output_type.as_bytes());
        for key in &m.lookup_keys {
            sink.part(b"lookup", key.as_bytes());
        }
    }
    MappingIdentity(ContentHash(sink.finish().0))
}

/// Named nested transformations declare hydration dependencies separately from row-view lowering.
/// The output's Serde/Schemars declaration owns nesting; this is not a projection DSL.
#[derive(Debug, Clone)]
pub struct PacketMapping {
    pub name: &'static str,
    pub sources: Vec<Relation>,
    pub minimum_frontier: Frontier,
    pub required_capabilities: &'static [Capability],
    pub output_type: &'static str,
    pub revision: u32,
}
pub fn packet_inventory() -> Vec<PacketMapping> {
    use domain::{catalog as c, catalog::evidence as e};
    macro_rules! sources {($($ty:ty),*$(,)?)=>{vec![$(Relation::of::<$ty>()),*]};}
    macro_rules! mapping {
        ($name:literal,$out:literal,$caps:expr,$sources:expr) => {
            PacketMapping {
                name: $name,
                sources: $sources,
                minimum_frontier: Frontier::Catalog,
                required_capabilities: $caps,
                output_type: $out,
                revision: 1,
            }
        };
    }
    vec![
        mapping!(
            "operation_core",
            "OperationCore",
            &[Capability::Catalog],
            sources!(
                c::CatalogMember,
                c::CatalogExposure,
                c::CatalogCandidate,
                c::CatalogCallable,
                c::CatalogInvocation,
                c::CatalogOption,
                c::CatalogOptionSubject,
                c::CatalogDefault,
                c::CatalogOptionEvidence,
                domain::normalized::callables::EffectiveCallableAssessment,
                domain::normalized::callables::SignatureVariant,
                domain::normalized::callables::SignatureSlot,
                domain::normalized::callables::SignatureSlotEntity,
                domain::normalized::entities::ParameterEntityLink,
                domain::calls::Signature,
                domain::calls::SignatureParameter,
                domain::calls::ParameterShape,
                domain::types::TypeObservation,
                domain::types::TypeTerm,
                domain::types::TypePresentation,
                domain::value::Literal,
                domain::source::Module,
                domain::input::ArtifactOwnership,
                domain::input::DistributionVerification,
                domain::input::InputDistribution,
                domain::input::Release,
                domain::input::Package
            )
        ),
        mapping!(
            "original_evidence",
            "EvidencePacket",
            &[Capability::Catalog],
            sources!(
                e::OriginalSource,
                domain::source::Occurrence,
                domain::source::SourceArtifact,
                domain::artifact::ArtifactChunk,
                domain::input::ArtifactOwnership,
                domain::input::DistributionVerification,
                domain::input::InputDistribution,
                domain::input::CorpusLibrary,
                domain::input::Release,
                domain::attribution::AnalysisContext,
                domain::attribution::ProviderRun,
                domain::assertion::Evidence,
                domain::retrieval::OriginalAnchor,
                domain::retrieval::AnchorSource,
                domain::synthesis::documentary::ProseSlice,
                domain::synthesis::documentary::ProseSource
            )
        ),
        mapping!(
            "scenario",
            "ScenarioPacket",
            &[Capability::Catalog],
            sources!(
                e::CatalogScenario,
                e::ScenarioSource,
                e::ScenarioSpan,
                e::ScenarioAssociation,
                e::ScenarioCheck
            )
        ),
        mapping!(
            "deployment",
            "DeploymentPacket",
            &[Capability::Catalog],
            sources!(
                e::CatalogDeployment,
                e::ReleaseDeployment,
                domain::deployment::DeploymentObservation
            )
        ),
        mapping!(
            "selection",
            "OperationCandidate",
            &[Capability::Catalog],
            sources!(
                domain::selection::SelectionDomain,
                domain::selection::DomainContext,
                domain::selection::DomainClosure,
                domain::selection::DomainEvidence,
                domain::selection::Context,
                domain::selection::Witness
            )
        ),
        mapping!(
            "retrieval",
            "EvidenceHit",
            &[Capability::Catalog],
            sources!(
                domain::retrieval::Unit,
                domain::retrieval::CorpusText,
                domain::retrieval::OriginalAnchor,
                domain::retrieval::AnchorSource,
                domain::retrieval::UnitSubject,
                domain::retrieval::Subject
            )
        ),
        mapping!(
            "capability",
            "CapabilityPacket",
            &[Capability::Catalog, Capability::Briefs],
            sources!(
                domain::synthesis::briefs::Brief,
                domain::synthesis::briefs::BriefAssertion,
                domain::synthesis::briefs::BriefDocument,
                domain::synthesis::briefs::BriefSource,
                domain::synthesis::assertions::ProgrammaticAssertion,
                domain::synthesis::assertions::ProgrammaticAssertionSupport,
                domain::synthesis::assertions::AssertionSource,
                domain::assertion::AssertionQualification
            )
        ),
        mapping!(
            "relationships",
            "RelationshipPacket",
            &[Capability::Catalog],
            sources!(
                domain::normalized::events::NormalizedCallEvent,
                domain::normalized::events::CallPolicyAssessment,
                domain::normalized::events::CallPolicyAdmission,
                domain::normalized::events::NormalizedCallAlternative,
                domain::normalized::entities::OccurrenceOwnership,
                domain::catalog::CatalogCallable,
                domain::catalog::CatalogClass
            )
        ),
        mapping!(
            "behavior",
            "BehaviorPacket",
            &[Capability::Catalog],
            sources!(
                domain::synthesis::summary::SummaryFacet,
                domain::execution::summary_consequences::ClaimConclusion,
                domain::execution::summary_consequences::ClaimProof,
                domain::analysis::summary::AnalysisInvocation,
                domain::analysis::AnalysisDefinition,
                domain::analysis::MethodParameters,
                domain::assertion::AssertionQualification,
                domain::conditions::Condition,
                domain::conditions::ConditionNode,
                domain::models::ModelCatalog
            )
        ),
        mapping!(
            "native_assessment",
            "NativeAssessmentPacket",
            &[Capability::Catalog, Capability::Native],
            sources!(
                domain::conditions::Condition,
                domain::conditions::EvaluationAtom
            )
        ),
    ]
}
