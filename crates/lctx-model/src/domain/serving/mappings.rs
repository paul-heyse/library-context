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
impl Capability {
    fn name(self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::Briefs => "briefs",
            Self::Native => "native",
        }
    }
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
    let mut sink = KeySink::new("serving-mappings/v2");
    identity_for(&inventory()).0.encode(&mut sink);
    for kind in PacketKind::ALL {
        let binding = kind.binding();
        for child in binding.children {
            sink.part(b"child", child.binding().mapping.name.as_bytes());
        }
        for prepared in binding.prepared {
            sink.part(b"prepared", prepared.name().as_bytes());
        }
        let mapping = binding.lowered();
        sink.part(b"transformation", mapping.name.as_bytes());
        sink.part(b"output", mapping.output_type.as_bytes());
        sink.part(b"revision", &mapping.revision.to_le_bytes());
        sink.part(b"frontier", mapping.minimum_frontier.name().as_bytes());
        for capability in mapping.required_capabilities {
            sink.part(b"capability", capability.name().as_bytes());
        }
        for relation in mapping.sources {
            sink.part(b"source", relation.name().as_bytes());
            for field in relation.fields() {
                field.encode_contract(&mut sink);
            }
            if let Some(sum) = relation.sum() {
                sum.encode_contract(&mut sink);
            }
        }
    }
    MappingIdentity(sink.finish())
}
pub fn identity_for(mappings: &[Mapping]) -> MappingIdentity {
    let mut sink = KeySink::new("serving-mapping/v2");
    let mut canonical = mappings.iter().collect::<Vec<_>>();
    canonical.sort_by_key(|mapping| mapping.name);
    for m in canonical {
        m.name.to_owned().encode(&mut sink);
        m.source.name().to_owned().encode(&mut sink);
        sink.part(b"revision", &m.revision.to_le_bytes());
        sink.part(b"frontier", m.minimum_frontier.name().as_bytes());
        for c in m.required_capabilities {
            sink.part(b"capability", c.name().as_bytes());
        }
        for field in m.fields() {
            field.encode_contract(&mut sink);
        }
        if let Some(sum) = m.source.sum() {
            sum.encode_contract(&mut sink);
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
/// A closed binding couples an actual packet output to direct reads, children and prepared owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedDependency {
    Selection,
    CatalogIdentity,
    Retrieval,
    Native,
    CanonicalProof,
}
impl PreparedDependency {
    fn name(self) -> &'static str {
        match self {
            Self::Selection => "selection",
            Self::CatalogIdentity => "catalog-identity",
            Self::Retrieval => "retrieval",
            Self::Native => "native",
            Self::CanonicalProof => "canonical-proof",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketKind {
    OperationCore,
    CallableComparisonPacket,
    ContextualTypePacket,
    IncomingReferencePacket,
    AccessRoutePacket,
    EvidencePacket,
    ScenarioPacket,
    DeploymentPacket,
    OperationCandidate,
    EvidenceHit,
    CapabilityPacket,
    RelationshipPacket,
    BehaviorPacket,
    NativeAssessmentPacket,
    OriginalRange,
    OperationPacket,
}
#[derive(Debug, Clone)]
pub struct PacketBinding {
    pub kind: PacketKind,
    pub mapping: PacketMapping,
    pub children: &'static [PacketKind],
    pub prepared: &'static [PreparedDependency],
}
mod sealed {
    pub trait Sealed {}
}
pub trait PacketOutput: sealed::Sealed {
    fn binding() -> &'static PacketBinding;
}
impl PacketKind {
    pub const ALL: [Self; 16] = [
        Self::OperationCore,
        Self::CallableComparisonPacket,
        Self::ContextualTypePacket,
        Self::IncomingReferencePacket,
        Self::AccessRoutePacket,
        Self::EvidencePacket,
        Self::ScenarioPacket,
        Self::DeploymentPacket,
        Self::OperationCandidate,
        Self::EvidenceHit,
        Self::CapabilityPacket,
        Self::RelationshipPacket,
        Self::BehaviorPacket,
        Self::NativeAssessmentPacket,
        Self::OriginalRange,
        Self::OperationPacket,
    ];
    pub fn binding(self) -> &'static PacketBinding {
        match self {
            Self::OperationCore => <super::OperationCore as PacketOutput>::binding(),
            Self::CallableComparisonPacket => {
                <super::CallableComparisonPacket as PacketOutput>::binding()
            }
            Self::ContextualTypePacket => <super::ContextualTypePacket as PacketOutput>::binding(),
            Self::IncomingReferencePacket => {
                <super::IncomingReferencePacket as PacketOutput>::binding()
            }
            Self::AccessRoutePacket => <super::AccessRoutePacket as PacketOutput>::binding(),
            Self::EvidencePacket => <super::EvidencePacket as PacketOutput>::binding(),
            Self::ScenarioPacket => <super::ScenarioPacket as PacketOutput>::binding(),
            Self::DeploymentPacket => <super::DeploymentPacket as PacketOutput>::binding(),
            Self::OperationCandidate => <super::OperationCandidate as PacketOutput>::binding(),
            Self::EvidenceHit => <super::EvidenceHit as PacketOutput>::binding(),
            Self::CapabilityPacket => <super::CapabilityPacket as PacketOutput>::binding(),
            Self::RelationshipPacket => <super::RelationshipPacket as PacketOutput>::binding(),
            Self::BehaviorPacket => <super::BehaviorPacket as PacketOutput>::binding(),
            Self::NativeAssessmentPacket => {
                <super::NativeAssessmentPacket as PacketOutput>::binding()
            }
            Self::OriginalRange => <super::OriginalRange as PacketOutput>::binding(),
            Self::OperationPacket => <super::OperationPacket as PacketOutput>::binding(),
        }
    }
}
impl PacketBinding {
    /// Check before starting a canonical read, including an empty attempted read.
    pub fn permits<R: Record>(&self) -> bool {
        (self.prepared.contains(&PreparedDependency::CanonicalProof)
            && canonical_declarations()
                .iter()
                .any(|r| r.type_id() == std::any::TypeId::of::<R>()))
            || self
                .mapping
                .sources
                .iter()
                .any(|r| r.type_id() == std::any::TypeId::of::<R>())
            || self.children.iter().any(|k| k.binding().permits::<R>())
    }
    pub fn permits_relation(&self, name: &str) -> bool {
        (self.prepared.contains(&PreparedDependency::CanonicalProof)
            && canonical_declarations().iter().any(|r| r.name() == name))
            || self.mapping.sources.iter().any(|r| r.name() == name)
            || self
                .children
                .iter()
                .any(|k| k.binding().permits_relation(name))
    }
    pub fn lowered(&self) -> PacketMapping {
        let mut mapping = self.mapping.clone();
        if self.prepared.contains(&PreparedDependency::CanonicalProof) {
            mapping
                .sources
                .extend(canonical_declarations().iter().cloned());
        }
        for child in self.children {
            mapping.sources.extend(child.binding().lowered().sources);
        }
        mapping.sources.sort_by_key(Relation::name);
        mapping.sources.dedup_by_key(|r| r.name());
        mapping
    }
}
/// Immutable finite schema metadata is shared process-wide; no per-read model construction.
fn canonical_declarations() -> &'static Vec<Relation> {
    static GRAPH: std::sync::OnceLock<Vec<Relation>> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        let mut relations = Vec::new();
        macro_rules! selected {($($variant:ident:$ty:ty,)*)=>{relations.extend(vec![$(Relation::of::<$ty>(),)*]);};}
        crate::graph_entity_records!(selected);
        crate::graph_assertion_records!(selected);
        relations.sort_by_key(Relation::name);
        relations.dedup_by_key(|relation| relation.name());
        relations
    })
}
fn dependency_sources(dependencies: &[PreparedDependency]) -> Vec<Relation> {
    let mut inputs = Vec::new();
    for dependency in dependencies {
        match dependency {
            PreparedDependency::Selection => {
                inputs.extend(domain::selection::classification::ClassificationData::inputs());
                inputs.extend(domain::selection::build::Output::inputs());
                inputs.extend(domain::selection::admission::AdmissionData::inputs());
            }
            PreparedDependency::CatalogIdentity => {
                inputs.extend(domain::serving::LibraryAdmissionData::inputs());
                inputs.extend([
                    domain::ValidationInput::of::<domain::input::InputDistribution>(&["id"]),
                    domain::ValidationInput::of::<domain::input::ArtifactOwnership>(&["id"]),
                    domain::ValidationInput::of::<domain::input::DistributionVerification>(&["id"]),
                ]);
            }
            PreparedDependency::Retrieval => {
                macro_rules! input {($($field:ident:$ty:ty,)*)=>{inputs.extend(vec![$(domain::ValidationInput::of::<$ty>(&["id"]),)*]);};}
                crate::serving_retrieval_inputs!(input);
            }
            PreparedDependency::Native => {
                inputs.extend(domain::native_requests::NativeInventory::inputs());
                inputs.extend(domain::native_requests::PreparationRows::inputs());
                inputs.extend(
                    domain::normalized::binding_normalization::BindingData::validation_inputs(),
                );
                inputs.extend(
                    domain::normalized::binding_normalization::BindingOutput::validation_inputs(),
                );
                for invariant in domain::native_requests::preparation_invariants() {
                    inputs.extend(invariant.inputs);
                }
            }
            PreparedDependency::CanonicalProof => {}
        }
    }
    if inputs.is_empty() {
        return vec![];
    }
    let model = canonical_declarations();
    model
        .iter()
        .filter(|r| inputs.iter().any(|i| i.type_id() == r.type_id()))
        .cloned()
        .collect()
}
macro_rules! sources {($($ty:ty),*$(,)?)=>{vec![$(Relation::of::<$ty>()),*]};}
fn claim_basis_sources() -> Vec<Relation> {
    sources!(
        domain::assumptions::AssumptionSet,
        domain::assumptions::AssumptionSetMember,
        domain::assumptions::Assumption,
        domain::assumptions::AssumptionUniverse,
        domain::assumptions_universe::AssumptionUniverseSupport,
        domain::assertion::AssertionQualification,
        domain::types::TypeObservation,
        domain::types::TypeSupport,
        domain::symbols::ClassTraitObservation,
        domain::symbols::ClassTraitSupport,
        domain::attribution::ProviderRun,
        domain::attribution::Provider,
        domain::attribution::AnalysisContext,
        domain::assertion::ProviderSurface,
        domain::assertion::Evidence,
        domain::source::Occurrence,
        domain::source::SourceArtifact,
        domain::models::ModelCatalog,
        domain::models::AuthoredModel
    )
}
macro_rules! binding {
    ($name:literal,$out:ident,$caps:expr,$sources:expr,$children:expr,$prepared:expr) => {
        impl sealed::Sealed for super::$out {}
        impl PacketOutput for super::$out {
            fn binding() -> &'static PacketBinding {
                static BINDING: std::sync::OnceLock<PacketBinding> = std::sync::OnceLock::new();
                BINDING.get_or_init(|| {
                    let prepared: &'static [PreparedDependency] = $prepared;
                    let mut sources = $sources;
                    if matches!(
                        $name,
                        "operation_core"
                            | "capability"
                            | "behavior"
                            | "native_assessment"
                            | "original_evidence"
                    ) {
                        sources.extend(claim_basis_sources());
                    }
                    sources.extend(dependency_sources(prepared));
                    sources.sort_by_key(Relation::name);
                    sources.dedup_by_key(|r| r.name());
                    PacketBinding {
                        kind: PacketKind::$out,
                        children: $children,
                        prepared,
                        mapping: PacketMapping {
                            name: $name,
                            sources,
                            minimum_frontier: Frontier::Catalog,
                            required_capabilities: $caps,
                            output_type: std::any::type_name::<super::$out>()
                                .rsplit("::")
                                .next()
                                .expect("packet type"),
                            revision: if matches!(
                                $name,
                                "original_evidence"
                                    | "scenario"
                                    | "relationships"
                                    | "operation"
                                    | "callable_comparison"
                                    | "contextual_typing"
                                    | "incoming_references"
                                    | "access_routes"
                            ) {
                                2
                            } else {
                                1
                            },
                        },
                    }
                })
            }
        }
    };
}
use domain::{catalog as c, catalog::evidence as e};
binding!(
    "operation_core",
    OperationCore,
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
        domain::types::TypeSupport,
        domain::types::NativeSignatureObservation,
        domain::types::NativeSignatureSupport,
        domain::types::SignatureTypeSubject,
        domain::types::SignatureTypeObservation,
        domain::types::SignatureTypeSupport,
        domain::normalized::callables::SignatureSlotType,
        domain::normalized::callables::SignatureReturnType,
        domain::types::TypeTerm,
        domain::types::TypePresentation,
        domain::value::Literal,
        domain::syntax::ParameterSyntaxObservation,
        domain::normalized::entities::FieldEntity,
        domain::normalized::entities::FieldDeclarationLink,
        domain::normalized::entities::FieldEntityLink,
        domain::normalized::callable_aspects::FieldDefaultAssessment,
        domain::normalized::callable_aspects::FieldDefault,
        domain::syntax::ClassFieldSyntaxObservation,
        domain::types::RecordFieldObservation,
        domain::source::Occurrence,
        domain::source::SourceArtifact,
        domain::attribution::AnalysisContext,
        domain::assertion::AssertionQualification,
        domain::conditions::Condition,
        domain::conditions::ConditionNode,
        domain::conditions::EvaluationAtom,
        domain::value::Predicate,
        domain::source::Module,
        domain::input::ArtifactOwnership,
        domain::input::DistributionVerification,
        domain::input::InputDistribution,
        domain::input::Release,
        domain::input::Package
    ),
    &[],
    &[
        PreparedDependency::Selection,
        PreparedDependency::CatalogIdentity
    ]
);
binding!(
    "original_evidence",
    EvidencePacket,
    &[Capability::Catalog],
    sources!(
        domain::conditions::entry::EntryValueWitness,
        domain::conditions::entry::EntryAccessSource,
        domain::local_semantics::LocalContribution,
        domain::flow::FlowUseObservation,
        domain::flow::FlowValueObservation,
        domain::flow::FlowTestLeafObservation,
        domain::flow::FlowRegionObservation,
        domain::flow::FlowUse,
        domain::flow::FlowReachingObservation,
        domain::flow::FlowReachingSupport,
        domain::flow::ReachingDefinition,
        domain::flow::FlowDefinition,
        domain::flow::FlowSourceViewObservation,
        domain::flow::FlowSourceViewSupport,
        domain::flow_inventory::FlowUseInventoryObservation,
        domain::flow_inventory::FlowUseInventorySupport,
        domain::flow_inventory::FlowUseCandidate,
        domain::flow_inventory::FlowUseInventoryMember,
        domain::attribution::ProviderCoverage,
        e::OriginalSource,
        e::SourceUsage,
        e::SourceCharacterization,
        domain::normalized::events::NormalizedCallEvent,
        domain::normalized::events::NormalizedCallAlternative,
        domain::normalized::events::CallAlternativeSource,
        domain::normalized::bindings::CallBindingAttempt,
        domain::normalized::bindings::CallBinding,
        domain::normalized::callables::SignatureVariant,
        domain::calls::ProviderCallSite,
        domain::calls::ProviderCallSiteSupport,
        domain::calls::CallTarget,
        domain::calls::CallTargetSupport,
        domain::calls::CallDestination,
        domain::calls::CallChannel,
        domain::calls::Receiver,
        domain::calls::CallSyntax,
        domain::calls::CallSyntaxSupport,
        domain::calls::CallArgument,
        domain::calls::Signature,
        domain::calls::BindingSource,
        domain::types::TypeObservation,
        domain::types::TypeSupport,
        domain::types::NativeOverloadObservation,
        domain::types::NativeOverloadSupport,
        domain::types::NativeOverloadCandidate,
        domain::types::NativeOverloadCandidateSupport,
        domain::normalized::overload_association::OverloadVariantAssessment,
        domain::normalized::overload_association::OverloadVariantCandidate,
        domain::types::NativeSignatureObservation,
        domain::types::NativeSignatureSupport,
        domain::normalized::entities::SymbolEntityResolution,
        domain::calls::ProviderSymbol,
        e::ScenarioAssociation,
        domain::catalog::CatalogMember,
        e::SourceCharacterizationScenario,
        e::DiagnosticUseAssessment,
        e::DiagnosticUseLink,
        e::DiagnosticUsePath,
        e::DiagnosticUseTarget,
        domain::normalized::events::CallEventSource,
        domain::normalized::entities::OccurrenceOwnership,
        domain::syntax::SyntaxPlacement,
        domain::analysis::native::NativeAssertionPremise,
        domain::assertion::AssertionQualification,
        domain::assertion::ProviderSurface,
        domain::attribution::Provider,
        domain::diagnostics::RuffDiagnosticObservation,
        domain::diagnostics::RuffDiagnosticSupport,
        domain::diagnostics::PyreflyDiagnosticObservation,
        domain::diagnostics::PyreflyDiagnosticSupport,
        domain::diagnostics::DiagnosticSubject,
        domain::diagnostics::DiagnosticAnnotation,
        domain::diagnostics::NativeParameterDefinitionObservation,
        domain::diagnostics::NativeParameterDefinitionSupport,
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
    ),
    &[],
    &[PreparedDependency::CanonicalProof]
);
binding!(
    "scenario",
    ScenarioPacket,
    &[Capability::Catalog],
    sources!(
        e::CatalogScenario,
        e::ScenarioSource,
        e::ScenarioSpan,
        e::ScenarioAssociation,
        e::ScenarioCheck,
        e::DiagnosticUseTarget,
        e::DiagnosticUseLink,
        e::DiagnosticUseAssessment
    ),
    &[PacketKind::OriginalRange],
    &[]
);
binding!(
    "deployment",
    DeploymentPacket,
    &[Capability::Catalog],
    sources!(
        e::CatalogDeployment,
        e::ReleaseDeployment,
        domain::deployment::DeploymentObservation
    ),
    &[PacketKind::OriginalRange],
    &[]
);
binding!(
    "selection",
    OperationCandidate,
    &[Capability::Catalog],
    sources!(
        domain::selection::SelectionDomain,
        domain::selection::DomainContext,
        domain::selection::DomainClosure,
        domain::selection::DomainEvidence,
        domain::selection::Context,
        domain::selection::Witness
    ),
    &[],
    &[
        PreparedDependency::Selection,
        PreparedDependency::CatalogIdentity
    ]
);
binding!(
    "retrieval",
    EvidenceHit,
    &[Capability::Catalog],
    sources!(
        domain::retrieval::Unit,
        domain::retrieval::CorpusText,
        domain::retrieval::OriginalAnchor,
        domain::retrieval::AnchorSource,
        domain::retrieval::UnitSubject,
        domain::retrieval::Subject
    ),
    &[PacketKind::EvidencePacket],
    &[PreparedDependency::Retrieval]
);
binding!(
    "capability",
    CapabilityPacket,
    &[Capability::Catalog, Capability::Briefs],
    sources!(
        domain::synthesis::briefs::Brief,
        domain::catalog::CatalogMember,
        domain::catalog::CatalogMemberInvocation,
        domain::synthesis::seeds::SelectedSeed,
        domain::synthesis::documentary::DocumentaryConclusion,
        domain::synthesis::documentary::ProseSlice,
        domain::synthesis::documentary::ProseSource,
        domain::synthesis::briefs::BriefAssertion,
        domain::synthesis::briefs::BriefDocument,
        domain::synthesis::briefs::BriefSource,
        domain::synthesis::assertions::ProgrammaticAssertion,
        domain::synthesis::assertions::ProgrammaticAssertionSupport,
        domain::synthesis::assertions::AssertionSource,
        domain::protocols::NativeTerminalObservation,
        domain::protocols::NativeTerminalSupport,
        domain::syntax::SyntaxPlacementSupport,
        domain::calls::CallTarget,
        domain::calls::CallTargetSupport,
        domain::class_metadata::ClassMetadataObservation,
        domain::class_metadata::ClassMetadataSupport,
        domain::class_metadata::ClassMemberObservation,
        domain::class_metadata::ClassMemberSupport,
        domain::source::CoverageScope,
        domain::source::Module,
        domain::input::CorpusLibrary,
        domain::input::InputDistribution,
        domain::execution::summary_terminal::SummaryTerminalWitness,
        domain::execution::protocol_interpretation::ConditionalTerminalFrontier,
        domain::execution::protocol_interpretation::NormalContinuationRestriction,
        domain::execution::closed_targets::ClosedTargetAssessment,
        domain::symbols::ClassAncestryObservation,
        domain::analysis::model::AnalysisInvocation,
        domain::analysis::summary::AnalysisInvocation,
        domain::execution::summary_consequences::SummaryClaim,
        domain::analysis::summary::ObligationSubject,
        domain::analysis::summary::SupportSource,
        domain::analysis::summary::AnalysisDerivation,
        domain::analysis::summary::AnalysisProposition,
        domain::analysis::summary::AnalysisDerivationPremise,
        domain::analysis::native::NativeAssertionPremise,
        domain::types::TypeObservation,
        domain::source::Occurrence,
        domain::syntax::SyntaxPlacement,
        domain::assertion::AssertionQualification
    ),
    &[PacketKind::OriginalRange],
    &[]
);
binding!(
    "callable_comparison",
    CallableComparisonPacket,
    &[Capability::Catalog],
    vec![],
    &[],
    &[PreparedDependency::Selection]
);
binding!(
    "contextual_typing",
    ContextualTypePacket,
    &[Capability::Catalog],
    vec![],
    &[],
    &[PreparedDependency::Selection]
);
binding!(
    "incoming_references",
    IncomingReferencePacket,
    &[Capability::Catalog],
    vec![],
    &[],
    &[PreparedDependency::Selection]
);
binding!(
    "relationships",
    RelationshipPacket,
    &[Capability::Catalog],
    sources!(
        domain::normalized::events::NormalizedCallEvent,
        domain::normalized::events::CallPolicyAssessment,
        domain::normalized::events::CallPolicyAdmission,
        domain::normalized::events::NormalizedCallAlternative,
        domain::normalized::entities::OccurrenceOwnership,
        domain::catalog::CatalogCallable,
        domain::catalog::CatalogClass
    ),
    &[],
    &[PreparedDependency::Selection]
);
binding!(
    "behavior",
    BehaviorPacket,
    &[Capability::Catalog],
    sources!(
        domain::catalog::CatalogMemberInvocation,
        domain::analysis::summary::AnalysisOutcome,
        domain::synthesis::summary::SummaryFacet,
        domain::execution::summary_consequences::ClaimConclusion,
        domain::execution::summary_consequences::ClaimProof,
        domain::analysis::summary::AnalysisInvocation,
        domain::analysis::AnalysisDefinition,
        domain::analysis::MethodParameters,
        domain::assertion::AssertionQualification,
        domain::conditions::Condition,
        domain::conditions::ConditionNode,
        domain::models::ModelCatalog,
        domain::execution::summary_consequences::SummaryClaim,
        domain::transfer::summary::SummaryPremise,
        domain::transfer::summary::TransferKey,
        domain::transfer::summary::TransferAlternative,
        domain::execution::summary_capture::SummaryCaptureContribution,
        domain::execution::summary_capture::SummaryCaptureWitness,
        domain::execution::capture_bridge::CapturedEntryBinding,
        domain::execution::capture_bridge::CapturedValueSource,
        domain::analysis::enriched_execution::AnalysisInvocation,
        domain::analysis::source_call::AnalysisInvocation,
        domain::execution::source_call_records::SourceCallHeader,
        domain::execution::enriched_records::SourceExecutionInvocation,
        domain::execution::enriched_records::BodyExecution,
        domain::execution::enriched_records::ExecutionOutcome,
        domain::normalized::events::NormalizedCallEvent,
        domain::syntax::SyntaxPlacement,
        domain::source::CoverageScope,
        domain::source::Module,
        domain::flow::FlowDefinitionObservation,
        domain::flow::FlowDefinitionSupport,
        domain::flow::FlowDefinition,
        domain::flow::FlowUse,
        domain::lexical::LexicalScope,
        domain::captures::CaptureObservation,
        domain::captures::CaptureSupport,
        domain::flow_capture::FlowCaptureTimingObservation,
        domain::flow_capture::FlowCaptureTimingSupport,
        domain::flow_capture::FlowCaptureInventory,
        domain::flow_capture::FlowCaptureCandidate,
        domain::flow_capture::FlowCaptureTarget,
        domain::attribution::RunFamily,
        domain::normalized::entities::ParameterEntity,
        domain::normalized::entities::ParameterEntityLink,
        domain::calls::SignatureParameter,
        domain::calls::Signature,
        domain::calls::ParameterShape,
        domain::declarations::ParameterDeclaration,
        domain::declarations::ParameterDeclarationSupport,
        domain::calls::ProviderSymbol,
        domain::value::Literal
    ),
    &[],
    &[]
);
binding!(
    "native_assessment",
    NativeAssessmentPacket,
    &[Capability::Catalog, Capability::Native],
    sources!(
        domain::catalog::CatalogMember,
        domain::catalog::CatalogCallable,
        domain::catalog::CatalogInvocation,
        domain::normalized::entities::ParameterEntityLink,
        domain::conditions::Condition,
        domain::conditions::EvaluationAtom
    ),
    &[],
    &[PreparedDependency::Native]
);
binding!(
    "original_range",
    OriginalRange,
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
    ),
    &[],
    &[]
);
binding!(
    "operation",
    OperationPacket,
    &[Capability::Catalog],
    vec![],
    &[
        PacketKind::OperationCore,
        PacketKind::CallableComparisonPacket,
        PacketKind::ContextualTypePacket,
        PacketKind::IncomingReferencePacket,
        PacketKind::AccessRoutePacket,
        PacketKind::ScenarioPacket,
        PacketKind::DeploymentPacket,
        PacketKind::RelationshipPacket,
        PacketKind::OperationCandidate,
        PacketKind::CapabilityPacket,
        PacketKind::BehaviorPacket
    ],
    &[]
);
pub fn packet_inventory() -> Vec<PacketMapping> {
    PacketKind::ALL
        .into_iter()
        .map(|k| k.binding().lowered())
        .collect()
}
pub fn prepared_binding(dependency: PreparedDependency) -> &'static PacketBinding {
    static BINDINGS: std::sync::OnceLock<[PacketBinding; 5]> = std::sync::OnceLock::new();
    let all = BINDINGS.get_or_init(|| {
        [
            PreparedDependency::Selection,
            PreparedDependency::CatalogIdentity,
            PreparedDependency::Retrieval,
            PreparedDependency::Native,
            PreparedDependency::CanonicalProof,
        ]
        .map(|dependency| PacketBinding {
            kind: PacketKind::OperationCandidate,
            children: &[],
            prepared: match dependency {
                PreparedDependency::Selection => &[PreparedDependency::Selection],
                PreparedDependency::CatalogIdentity => &[PreparedDependency::CatalogIdentity],
                PreparedDependency::Retrieval => &[PreparedDependency::Retrieval],
                PreparedDependency::Native => &[PreparedDependency::Native],
                PreparedDependency::CanonicalProof => &[PreparedDependency::CanonicalProof],
            },
            mapping: PacketMapping {
                name: "prepared_dependency",
                sources: dependency_sources(&[dependency]),
                minimum_frontier: Frontier::Catalog,
                required_capabilities: &[Capability::Catalog],
                output_type: "prepared owner",
                revision: 1,
            },
        })
    });
    &all[match dependency {
        PreparedDependency::Selection => 0,
        PreparedDependency::CatalogIdentity => 1,
        PreparedDependency::Retrieval => 2,
        PreparedDependency::Native => 3,
        PreparedDependency::CanonicalProof => 4,
    }]
}

#[macro_export]
macro_rules! serving_retrieval_inputs {($m:ident)=>{$m!{
 units:$crate::domain::retrieval::Unit,fragments:$crate::domain::retrieval::SearchWindow,parts:$crate::domain::retrieval::ContentPart,part_maps:$crate::domain::retrieval::PartSourceMap,window_parts:$crate::domain::retrieval::WindowPart,window_maps:$crate::domain::retrieval::WindowSourceMap,window_bindings:$crate::domain::retrieval::WindowBinding,subjects:$crate::domain::retrieval::Subject,unit_subjects:$crate::domain::retrieval::UnitSubject,anchors:$crate::domain::retrieval::OriginalAnchor,origins:$crate::domain::retrieval::Origin,uses:$crate::domain::retrieval::consumption::RetrievalEmbeddingUse,parents:$crate::domain::input::CorpusLibrary,
}};}

binding!(
    "access_routes",
    AccessRoutePacket,
    &[Capability::Catalog],
    sources!(
        domain::syntax::ImportAliasObservation,
        domain::normalized::links::ImportModuleAssessment,
        domain::normalized::links::ImportModuleCandidate,
        domain::symbols::ModuleResolutionObservation,
        domain::symbols::ModuleResolutionSupport,
        domain::ruff::RuffBindingObservation,
        domain::ruff::RuffBindingSupport,
        domain::symbols::PublicNameObservation,
        domain::symbols::PublicNameSupport,
        domain::symbols::ExportOrigin,
        domain::normalized::entities::ClassEntity,
        domain::attribution::ProviderRun,
        domain::attribution::Provider,
        domain::assertion::ProviderSurface,
        domain::assertion::Evidence
    ),
    &[],
    &[PreparedDependency::Selection]
);

#[cfg(test)]
mod encoding_controls {
    use super::*;
    #[test]
    fn mapping_bytes_match_an_independently_framed_simple_contract() {
        let mapping =
            Mapping::of::<domain::input::Package>("package_control", &[Capability::Catalog]);
        let mut raw = blake3::Hasher::new();
        let mut frame = |tag: &[u8], value: &[u8]| {
            raw.update(&(tag.len() as u64).to_le_bytes());
            raw.update(tag);
            raw.update(&(value.len() as u64).to_le_bytes());
            raw.update(value);
        };
        frame(b"domain", b"lctx-semantic/v3");
        frame(b"type", b"serving-mapping/v2");
        frame(b"text", b"package_control");
        frame(b"text", b"packages");
        frame(b"revision", &1u32.to_le_bytes());
        frame(b"frontier", b"catalog");
        frame(b"capability", b"catalog");
        frame(b"field", b"name");
        frame(b"scalar", b"text");
        frame(b"roles", &[1, 0, 0, 0]);
        frame(b"target", b"");
        frame(b"option", &[0]);
        frame(b"code-count", &0u64.to_le_bytes());
        frame(b"dependency", b"packages");
        frame(b"output", b"packages");
        assert_eq!(
            identity_for(std::slice::from_ref(&mapping)).0.0,
            *raw.finalize().as_bytes()
        );
        let mut revision = mapping.clone();
        revision.revision = 2;
        assert_ne!(
            identity_for(std::slice::from_ref(&mapping)),
            identity_for(&[revision])
        );
        let mut other = mapping.clone();
        other.name = "other";
        assert_eq!(
            identity_for(&[mapping.clone(), other.clone()]),
            identity_for(&[other, mapping])
        );
    }
}
