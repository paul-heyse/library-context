//! Semantic graph contracts. The finite graph vocabulary selects semantic entities and
//! assertions; it is not the internal Arrow relation registry or a published execution ledger.
use super::{ContentHash, Id, Key, KeySink, ModelError, Record};
use serde::{Deserialize, Serialize};
mod analysisvalue;
mod claimvalue;
mod membershipvalue;
mod nativevalue;
mod provenancevalue;
mod supportvalue;
pub use analysisvalue::AnalysisValue;
pub use claimvalue::ClaimValue;
pub use membershipvalue::MembershipValue;
pub use nativevalue::NativeValue;
pub use provenancevalue::ProvenanceValue;
pub use supportvalue::SupportValue;
macro_rules! nominal_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name(pub ContentHash);
        impl Key for $name {
            fn encode(&self, sink: &mut KeySink) {
                self.0.encode(sink);
            }
        }
    };
}
nominal_id!(EntityId);
nominal_id!(AssertionId);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u16)]
pub enum EntityKind {
    Release = 0,
    Capture = 1,
    Source = 2,
    Module = 3,
    Occurrence = 4,
    Provider = 5,
    Context = 6,
    Run = 7,
    Declaration = 8,
    Exposure = 9,
    InvocationVariant = 10,
    Scope = 11,
    Parameter = 12,
    Field = 13,
    NativeSymbol = 14,
    Type = 15,
    Literal = 16,
    Place = 17,
    Predicate = 18,
    EvaluationAtom = 19,
    Condition = 20,
    AssumptionUniverse = 21,
    Assumption = 22,
    CatalogOption = 23,
    Scenario = 24,
    RetrievalText = 25,
    RetrievalUnit = 26,
    AnalysisDefinition = 27,
    EmbeddingSpecification = 28,
    Package = 29,
    AcquisitionOrigin = 30,
    NativeModule = 31,
    NativeCallable = 32,
    ParameterShape = 33,
    TypeVariable = 34,
    TypeSequence = 35,
    CallableParameterList = 36,
    CallableParameter = 37,
    TypedDictFieldList = 38,
    TypedDictField = 39,
    PlaceRoot = 40,
    PathSegment = 41,
    AccessPath = 42,
    LiteralSet = 43,
    ConditionNode = 44,
    AssumptionSet = 45,
    Qualification = 46,
    Evidence = 47,
    EvidenceSpan = 48,
    CatalogMember = 49,
    Original = 50,
    RetrievalOrigin = 51,
    SymbolSequence = 101,
    EntityReference = 102,
    Argument = 103,
    AnalysisRun = 104,
    AnalysisParent = 105,
    AnalysisPremise = 106,
    CoverageSource = 107,
    ProviderSurface = 100,
    MethodParameters = 52,
    Subject = 108,
    AnalyticFrame = 109,
    AnchorSource = 110,
    AssertionTemplate = 111,
    BindingEvent = 112,
    CallChannel = 113,
    CallDestination = 114,
    CallOrigin = 115,
    CallReceiver = 116,
    CatalogCallable = 117,
    CatalogCandidate = 118,
    CatalogClass = 119,
    CatalogDefault = 120,
    CatalogExposure = 121,
    CatalogOptionEvidence = 122,
    CatalogOptionSubject = 123,
    ConceptScope = 124,
    DocumentAttributeValue = 125,
    DocumentNode = 126,
    ExportOrigin = 127,
    FlowCallPath = 128,
    FlowCaptureInventory = 129,
    FlowDefinition = 130,
    FlowReachingTarget = 131,
    FlowUse = 132,
    LexicalScope = 133,
    LexicalTarget = 134,
    RecordOptions = 135,
    RecordTransformDefaults = 136,
    RetrievalSubject = 137,
    ScenarioSource = 138,
    SignatureSlot = 139,
    SignatureTypeSubject = 140,
    StructuralFrame = 141,
    SyntaxDetail = 142,
    TaskReport = 143,
    Transfer = 144,
    TypeDomain = 145,
    AuthoredModel = 146,
    ModelCatalog = 147,
    ModeledOperation = 148,
    Outcome = 149,
    Definition = 150,
    SynthesisFrame = 151,
    AnalyticAttribute = 152,
    RetrievalFragment = 153,
    RetrievalDefinition = 154,
}
/// A nominal current semantic key, independent of physical family names or Arrow layout.
fn entity_key(kind: EntityKind, domain: &str, key: &[u8; 16]) -> EntityId {
    let mut sink = KeySink::new("graph-entity-key/v1");
    sink.part(b"kind", &(kind as u16).to_le_bytes());
    sink.part(b"semantic-type", domain.as_bytes());
    sink.part(b"key", key);
    EntityId(sink.finish())
}
impl EntityId {
    pub fn of<R: GraphEntityRecord>(id: Id<R>) -> Self {
        entity_key(R::GRAPH_KIND, R::NAME, id.bytes())
    }
}
impl AssertionId {
    pub fn of<R: Record>(id: Id<R>) -> Self {
        Self::from_key(R::NAME, id.bytes())
    }
    fn from_key(domain: &str, key: &[u8; 16]) -> Self {
        let mut sink = KeySink::new("graph-assertion-key/v1");
        sink.part(b"semantic-type", domain.as_bytes());
        sink.part(b"key", key);
        Self(sink.finish())
    }
}
/// Only explicitly selected intrinsic declarations have an entity lowering.
pub trait GraphEntityRecord: Record {
    const GRAPH_KIND: EntityKind;
    fn into_graph(self) -> Entity;
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Entity {
    CatalogEvidenceRoot(super::catalog::evidence::EvidenceRoot),
    CatalogEvidenceRootSubject(super::catalog::evidence::RootSubject),
    RetrievalFragment(super::retrieval::Fragment),
    RetrievalDefinition(super::retrieval::RetrievalDefinition),
    AnalyticAttribute(super::analytics::Attribute),

    AuthoredModelTargets(super::models::AuthoredTarget),
    AnalyticTextDefinitions(super::embedding::text::TextDefinition),
    AnalyticTextSubjects(super::embedding::text::TextSubject),
    SynthesisFrames(super::synthesis::frames::Frame),
    ContextExecutionItems(super::execution::context_execution::ContextItem),
    CapturedValueSources(super::execution::capture_bridge::CapturedValueSource),
    ClassFieldDefaults(super::normalized::callable_aspects::FieldDefault),

    LocalTypeDomainValues(super::local_theory::DomainValue),
    ReportValues(super::deployment::ReportValue),
    ReportCollections(super::deployment::ReportCollection),
    ReportedEnvironments(super::deployment::ReportedEnvironment),
    ModelCatalogs(super::models::ModelCatalog),
    AuthoredModels(super::models::AuthoredModel),
    AuthoredContextProtocols(super::models::AuthoredContextProtocol),
    FlowCaptureTargets(super::flow_capture::FlowCaptureTarget),
    SynthesisSummaryFacets(super::synthesis::summary::SummaryFacet),
    BaseCompletionOutcomes(super::execution::completion_records::CompletionOutcome),
    ModelValuePaths(super::execution::model_rules::ModelValuePath),
    ModeledOperations(super::execution::model_rules::ModeledOperation),
    ExecutionOutcomes(super::execution::enriched_records::ExecutionOutcome),
    SummaryExceptionOutcomes(super::execution::summary_exceptions::SummaryExceptionOutcome),
    AnalyticsConfigurations(super::analysis::settings::AnalyticsConfiguration),
    NormalizationComputations(super::normalized::coverage::NormalizationComputation),

    TaskReports(super::deployment::TaskReport),
    CallChannels(super::calls::CallChannel),
    CallDestinations(super::calls::CallDestination),
    CallReceivers(super::calls::Receiver),
    CallOrigins(super::calls::CallOrigin),
    RecordOptions(super::class_metadata::RecordOptions),
    RecordTransformDefaults(super::class_metadata::RecordTransformDefaults),
    LexicalScopes(super::lexical::LexicalScope),
    BindingEvents(super::lexical::BindingEvent),
    LexicalTargets(super::lexical::LexicalTarget),
    ExportOrigins(super::symbols::ExportOrigin),
    SyntaxDetails(super::syntax::SyntaxDetail),
    FlowCaptureInventories(super::flow_capture::FlowCaptureInventory),
    FlowUses(super::flow::FlowUse),
    FlowDefinitions(super::flow::FlowDefinition),
    ReachingDefinitions(super::flow::ReachingDefinition),
    FlowCallPaths(super::flow::FlowCallPath),
    DocumentNodes(super::documents::DocumentNode),
    DocumentAttributeValues(super::documents::DocumentAttributeValue),
    CatalogExposures(super::catalog::CatalogExposure),
    CatalogCandidates(super::catalog::CatalogCandidate),
    CatalogCallables(super::catalog::CatalogCallable),
    CatalogClasses(super::catalog::CatalogClass),
    CatalogOptionSubjects(super::catalog::CatalogOptionSubject),
    CatalogDefaults(super::catalog::CatalogDefault),
    CatalogOptionEvidence(super::catalog::CatalogOptionEvidence),
    SynthesisAssertionTemplates(super::synthesis::assertions::AssertionTemplate),
    StructuralFrames(super::structural::StructuralFrame),
    RetrievalSubjects(super::retrieval::Subject),
    RetrievalAnchorSources(super::retrieval::AnchorSource),
    AnalyticFrames(super::analytics::AnalyticFrame),
    AnalyticConceptScopes(super::analytics::ConceptScope),
    SignatureSlots(super::normalized::callables::SignatureSlot),
    SignatureTypeSubjects(super::types::SignatureTypeSubject),
    CatalogScenarioSources(super::catalog::evidence::ScenarioSource),
    LocalTransferKeys(super::transfer::local::TransferKey),
    ModelTransferKeys(super::transfer::model::TransferKey),
    SummaryTransferKeys(super::transfer::summary::TransferKey),

    NativePremise(super::analysis::native::NativeAssertionPremise),
    LocalSubject(super::analysis::local::ObligationSubject),
    BaseEvaluationSubject(super::analysis::base_evaluation::ObligationSubject),
    BaseCompletionSubject(super::analysis::base_completion::ObligationSubject),
    SourceCallSubject(super::analysis::source_call::ObligationSubject),
    EnrichedExecutionSubject(super::analysis::enriched_execution::ObligationSubject),
    ModelSubject(super::analysis::model::ObligationSubject),
    SummarySubject(super::analysis::summary::ObligationSubject),
    StructuralSubject(super::analysis::structural::ObligationSubject),
    AnalyticEmbeddingSubject(super::analysis::analytic_embedding::ObligationSubject),
    AnalyticSubject(super::analysis::analytic::ObligationSubject),
    CatalogCoreSubject(super::analysis::catalog_core::ObligationSubject),
    CatalogEvidenceSubject(super::analysis::catalog_evidence::ObligationSubject),
    SelectionSubject(super::analysis::selection::ObligationSubject),
    SynthesisSubject(super::analysis::synthesis::ObligationSubject),
    RetrievalSubject(super::analysis::retrieval::ObligationSubject),
    LocalObligationSource(super::analysis::local::ObligationSource),
    BaseEvaluationObligationSource(super::analysis::base_evaluation::ObligationSource),
    BaseCompletionObligationSource(super::analysis::base_completion::ObligationSource),
    SourceCallObligationSource(super::analysis::source_call::ObligationSource),
    EnrichedExecutionObligationSource(super::analysis::enriched_execution::ObligationSource),
    ModelObligationSource(super::analysis::model::ObligationSource),
    SummaryObligationSource(super::analysis::summary::ObligationSource),
    StructuralObligationSource(super::analysis::structural::ObligationSource),
    AnalyticEmbeddingObligationSource(super::analysis::analytic_embedding::ObligationSource),
    AnalyticObligationSource(super::analysis::analytic::ObligationSource),
    CatalogCoreObligationSource(super::analysis::catalog_core::ObligationSource),
    CatalogEvidenceObligationSource(super::analysis::catalog_evidence::ObligationSource),
    SelectionObligationSource(super::analysis::selection::ObligationSource),
    SynthesisObligationSource(super::analysis::synthesis::ObligationSource),
    RetrievalObligationSource(super::analysis::retrieval::ObligationSource),

    ProviderSurface(super::assertion::ProviderSurface),
    Package(super::input::Package),
    Release(super::input::Release),
    Capture(super::input::InputRevision),
    AcquisitionOrigin(super::input::InputOrigin),
    Source(super::source::SourceArtifact),
    Module(super::source::Module),
    Occurrence(super::source::Occurrence),
    Provider(super::attribution::Provider),
    Context(super::attribution::AnalysisContext),
    Run(super::attribution::ProviderRun),
    Scope(super::source::CoverageScope),
    Declaration(super::normalized::entities::CallableEntity),
    ClassDeclaration(super::normalized::entities::ClassEntity),
    Parameter(super::normalized::entities::ParameterEntity),
    Field(super::normalized::entities::FieldEntity),
    Exposure(super::normalized::entities::PublicExposure),
    InvocationVariant(super::normalized::callables::SignatureVariant),
    NativeModule(super::calls::ProviderModule),
    NativeSymbol(super::calls::ProviderSymbol),
    NativeCallable(super::calls::ProviderCallable),
    SignatureParameter(super::calls::SignatureParameter),
    ParameterShape(super::calls::ParameterShape),
    Type(super::types::TypeTerm),
    TypeVariable(super::types::TypeVariable),
    TypeSequence(super::types::TypeSequence),
    CallableParameterList(super::types::CallableParameterList),
    CallableParameter(super::types::CallableParameter),
    TypedDictFieldList(super::types::TypedDictFieldList),
    TypedDictField(super::types::TypedDictField),
    Literal(super::value::Literal),
    PlaceRoot(super::value::PlaceRoot),
    PathSegment(super::value::PathSegment),
    AccessPath(super::value::AccessPath),
    Place(super::value::Place),
    Predicate(super::value::Predicate),
    LiteralSet(super::value::LiteralSet),
    EvaluationAtom(super::conditions::EvaluationAtom),
    Condition(super::conditions::Condition),
    ConditionNode(super::conditions::ConditionNode),
    AssumptionUniverse(super::assumptions::AssumptionUniverse),
    Assumption(super::assumptions::Assumption),
    AssumptionSet(super::assumptions::AssumptionSet),
    Qualification(super::assertion::AssertionQualification),
    Evidence(super::assertion::Evidence),
    CatalogMember(super::catalog::CatalogMember),
    CatalogOption(super::catalog::CatalogOption),
    Scenario(super::catalog::evidence::CatalogScenario),
    Original(super::catalog::evidence::OriginalSource),
    RetrievalText(super::retrieval::CorpusText),
    RetrievalUnit(super::retrieval::Unit),
    RetrievalOrigin(super::retrieval::Origin),
    AnalysisDefinition(super::analysis::AnalysisDefinition),
    MethodParameters(super::analysis::MethodParameters),
    EmbeddingSpecification(super::embedding::EmbeddingSpec),
    SymbolSequence(super::symbols::SymbolSequence),
    EntityReference(super::normalized::entities::EntityRef),
    Argument(super::calls::CallArgument),
    LocalRun(super::analysis::local::AnalysisInvocation),
    LocalParent(super::analysis::local::InvocationSource),
    LocalPremise(super::analysis::local::SupportSource),
    LocalCoverageSource(super::analysis::local::CoverageSource),
    BaseEvaluationRun(super::analysis::base_evaluation::AnalysisInvocation),
    BaseEvaluationParent(super::analysis::base_evaluation::InvocationSource),
    BaseEvaluationPremise(super::analysis::base_evaluation::SupportSource),
    BaseEvaluationCoverageSource(super::analysis::base_evaluation::CoverageSource),
    BaseCompletionRun(super::analysis::base_completion::AnalysisInvocation),
    BaseCompletionParent(super::analysis::base_completion::InvocationSource),
    BaseCompletionPremise(super::analysis::base_completion::SupportSource),
    BaseCompletionCoverageSource(super::analysis::base_completion::CoverageSource),
    SourceCallRun(super::analysis::source_call::AnalysisInvocation),
    SourceCallParent(super::analysis::source_call::InvocationSource),
    SourceCallPremise(super::analysis::source_call::SupportSource),
    SourceCallCoverageSource(super::analysis::source_call::CoverageSource),
    EnrichedExecutionRun(super::analysis::enriched_execution::AnalysisInvocation),
    EnrichedExecutionParent(super::analysis::enriched_execution::InvocationSource),
    EnrichedExecutionPremise(super::analysis::enriched_execution::SupportSource),
    EnrichedExecutionCoverageSource(super::analysis::enriched_execution::CoverageSource),
    ModelRun(super::analysis::model::AnalysisInvocation),
    ModelParent(super::analysis::model::InvocationSource),
    ModelPremise(super::analysis::model::SupportSource),
    ModelCoverageSource(super::analysis::model::CoverageSource),
    SummaryRun(super::analysis::summary::AnalysisInvocation),
    SummaryParent(super::analysis::summary::InvocationSource),
    SummaryPremise(super::analysis::summary::SupportSource),
    SummaryCoverageSource(super::analysis::summary::CoverageSource),
    StructuralRun(super::analysis::structural::AnalysisInvocation),
    StructuralParent(super::analysis::structural::InvocationSource),
    StructuralPremise(super::analysis::structural::SupportSource),
    StructuralCoverageSource(super::analysis::structural::CoverageSource),
    AnalyticEmbeddingRun(super::analysis::analytic_embedding::AnalysisInvocation),
    AnalyticEmbeddingParent(super::analysis::analytic_embedding::InvocationSource),
    AnalyticEmbeddingPremise(super::analysis::analytic_embedding::SupportSource),
    AnalyticEmbeddingCoverageSource(super::analysis::analytic_embedding::CoverageSource),
    AnalyticRun(super::analysis::analytic::AnalysisInvocation),
    AnalyticParent(super::analysis::analytic::InvocationSource),
    AnalyticPremise(super::analysis::analytic::SupportSource),
    AnalyticCoverageSource(super::analysis::analytic::CoverageSource),
    CatalogCoreRun(super::analysis::catalog_core::AnalysisInvocation),
    CatalogCoreParent(super::analysis::catalog_core::InvocationSource),
    CatalogCorePremise(super::analysis::catalog_core::SupportSource),
    CatalogCoreCoverageSource(super::analysis::catalog_core::CoverageSource),
    CatalogEvidenceRun(super::analysis::catalog_evidence::AnalysisInvocation),
    CatalogEvidenceParent(super::analysis::catalog_evidence::InvocationSource),
    CatalogEvidencePremise(super::analysis::catalog_evidence::SupportSource),
    CatalogEvidenceCoverageSource(super::analysis::catalog_evidence::CoverageSource),
    SelectionRun(super::analysis::selection::AnalysisInvocation),
    SelectionParent(super::analysis::selection::InvocationSource),
    SelectionPremise(super::analysis::selection::SupportSource),
    SelectionCoverageSource(super::analysis::selection::CoverageSource),
    SynthesisRun(super::analysis::synthesis::AnalysisInvocation),
    SynthesisParent(super::analysis::synthesis::InvocationSource),
    SynthesisPremise(super::analysis::synthesis::SupportSource),
    SynthesisCoverageSource(super::analysis::synthesis::CoverageSource),
    RetrievalRun(super::analysis::retrieval::AnalysisInvocation),
    RetrievalParent(super::analysis::retrieval::InvocationSource),
    RetrievalPremise(super::analysis::retrieval::SupportSource),
    RetrievalCoverageSource(super::analysis::retrieval::CoverageSource),
}
macro_rules! graph_entities {($consumer:ident;$($variant:ident:$kind:ident=>$ty:ty,)*)=>{
    $(impl GraphEntityRecord for $ty{const GRAPH_KIND:EntityKind=EntityKind::$kind;fn into_graph(self)->Entity{Entity::$variant(self)}}
      impl From<$ty> for Entity{fn from(value:$ty)->Self{Self::$variant(value)}})*
    impl Entity{
      pub fn subtype(&self)->Option<i16>{match self{$(Self::$variant(row)=>row.sum_tag(),)*}}
      pub fn reference_requirements(&self)->Result<Vec<ReferenceRequirement>,ModelError>{let refs=match self{$(Self::$variant(row)=>row.references(),)*};refs.iter().map(reference_requirement).collect()}
      pub fn kind(&self)->EntityKind{match self{$(Self::$variant(_)=>EntityKind::$kind,)*}}
      pub fn id(&self)->EntityId{match self{$(Self::$variant(row)=>EntityId::of(row.id()),)*}}
      pub fn content(&self)->ContentHash{let mut sink=KeySink::new("graph-entity-content/v1");sink.part(b"kind",&(self.kind() as u16).to_le_bytes());match self{$(Self::$variant(row)=>row.content_digest().encode(&mut sink),)*};sink.finish()}
      pub fn validate(&self)->Result<(),ModelError>{match self{$(Self::$variant(row)=>row.validate(),)*}}
      pub fn derivation(&self)->Result<Option<GraphDerivation>,ModelError>{match self{$(Self::$variant(row)=>graph_derivation(row.proof()),)*}}
      pub fn references(&self)->Result<Vec<(Target,Option<EntityKind>)>,ModelError>{let source=match self{$(Self::$variant(row)=>row.references(),)*};source.into_iter().map(|reference|reference_target(&reference)).collect()}
    }
};}
// One finite typed declaration owns intrinsic lowering and emitted record inventory. The
// exhaustive Entity matches below refuse any new variant without a selected semantic owner.
#[doc(hidden)]
#[macro_export]
macro_rules! graph_entity_declarations {($apply:path,$consumer:ident)=>{$apply!{$consumer;
    CatalogEvidenceRoot:Scope=>$crate::domain::catalog::evidence::EvidenceRoot,
    CatalogEvidenceRootSubject:Subject=>$crate::domain::catalog::evidence::RootSubject,
    RetrievalFragment:RetrievalFragment=>$crate::domain::retrieval::Fragment,
    RetrievalDefinition:RetrievalDefinition=>$crate::domain::retrieval::RetrievalDefinition,
    AnalyticAttribute:AnalyticAttribute=>$crate::domain::analytics::Attribute,


    AuthoredModelTargets:Subject=>$crate::domain::models::AuthoredTarget,
    AnalyticTextDefinitions:Definition=>$crate::domain::embedding::text::TextDefinition,
    AnalyticTextSubjects:Subject=>$crate::domain::embedding::text::TextSubject,
    SynthesisFrames:SynthesisFrame=>$crate::domain::synthesis::frames::Frame,
    ContextExecutionItems:Subject=>$crate::domain::execution::context_execution::ContextItem,
    CapturedValueSources:Subject=>$crate::domain::execution::capture_bridge::CapturedValueSource,
    ClassFieldDefaults:Literal=>$crate::domain::normalized::callable_aspects::FieldDefault,

    LocalTypeDomainValues:Literal=>$crate::domain::local_theory::DomainValue,
    ReportValues:Literal=>$crate::domain::deployment::ReportValue,
    ReportCollections:TaskReport=>$crate::domain::deployment::ReportCollection,
    ReportedEnvironments:Context=>$crate::domain::deployment::ReportedEnvironment,
    ModelCatalogs:ModelCatalog=>$crate::domain::models::ModelCatalog,
    AuthoredModels:AuthoredModel=>$crate::domain::models::AuthoredModel,
    AuthoredContextProtocols:ModeledOperation=>$crate::domain::models::AuthoredContextProtocol,
    FlowCaptureTargets:Subject=>$crate::domain::flow_capture::FlowCaptureTarget,
    SynthesisSummaryFacets:Subject=>$crate::domain::synthesis::summary::SummaryFacet,
    BaseCompletionOutcomes:Outcome=>$crate::domain::execution::completion_records::CompletionOutcome,
    ModelValuePaths:Place=>$crate::domain::execution::model_rules::ModelValuePath,
    ModeledOperations:ModeledOperation=>$crate::domain::execution::model_rules::ModeledOperation,
    ExecutionOutcomes:Outcome=>$crate::domain::execution::enriched_records::ExecutionOutcome,
    SummaryExceptionOutcomes:Outcome=>$crate::domain::execution::summary_exceptions::SummaryExceptionOutcome,
    AnalyticsConfigurations:MethodParameters=>$crate::domain::analysis::settings::AnalyticsConfiguration,
    NormalizationComputations:AnalysisRun=>$crate::domain::normalized::coverage::NormalizationComputation,

    TaskReports:TaskReport=>$crate::domain::deployment::TaskReport,
    CallChannels:CallChannel=>$crate::domain::calls::CallChannel,
    CallDestinations:CallDestination=>$crate::domain::calls::CallDestination,
    CallReceivers:CallReceiver=>$crate::domain::calls::Receiver,
    CallOrigins:CallOrigin=>$crate::domain::calls::CallOrigin,
    RecordOptions:RecordOptions=>$crate::domain::class_metadata::RecordOptions,
    RecordTransformDefaults:RecordTransformDefaults=>$crate::domain::class_metadata::RecordTransformDefaults,
    LexicalScopes:LexicalScope=>$crate::domain::lexical::LexicalScope,
    BindingEvents:BindingEvent=>$crate::domain::lexical::BindingEvent,
    LexicalTargets:LexicalTarget=>$crate::domain::lexical::LexicalTarget,
    ExportOrigins:ExportOrigin=>$crate::domain::symbols::ExportOrigin,
    SyntaxDetails:SyntaxDetail=>$crate::domain::syntax::SyntaxDetail,
    FlowCaptureInventories:FlowCaptureInventory=>$crate::domain::flow_capture::FlowCaptureInventory,
    FlowUses:FlowUse=>$crate::domain::flow::FlowUse,
    FlowDefinitions:FlowDefinition=>$crate::domain::flow::FlowDefinition,
    ReachingDefinitions:FlowReachingTarget=>$crate::domain::flow::ReachingDefinition,
    FlowCallPaths:FlowCallPath=>$crate::domain::flow::FlowCallPath,
    DocumentNodes:DocumentNode=>$crate::domain::documents::DocumentNode,
    DocumentAttributeValues:DocumentAttributeValue=>$crate::domain::documents::DocumentAttributeValue,
    CatalogExposures:CatalogExposure=>$crate::domain::catalog::CatalogExposure,
    CatalogCandidates:CatalogCandidate=>$crate::domain::catalog::CatalogCandidate,
    CatalogCallables:CatalogCallable=>$crate::domain::catalog::CatalogCallable,
    CatalogClasses:CatalogClass=>$crate::domain::catalog::CatalogClass,
    CatalogOptionSubjects:CatalogOptionSubject=>$crate::domain::catalog::CatalogOptionSubject,
    CatalogDefaults:CatalogDefault=>$crate::domain::catalog::CatalogDefault,
    CatalogOptionEvidence:CatalogOptionEvidence=>$crate::domain::catalog::CatalogOptionEvidence,
    SynthesisAssertionTemplates:AssertionTemplate=>$crate::domain::synthesis::assertions::AssertionTemplate,
    StructuralFrames:StructuralFrame=>$crate::domain::structural::StructuralFrame,
    RetrievalSubjects:RetrievalSubject=>$crate::domain::retrieval::Subject,
    RetrievalAnchorSources:AnchorSource=>$crate::domain::retrieval::AnchorSource,
    AnalyticFrames:AnalyticFrame=>$crate::domain::analytics::AnalyticFrame,
    AnalyticConceptScopes:ConceptScope=>$crate::domain::analytics::ConceptScope,
    SignatureSlots:SignatureSlot=>$crate::domain::normalized::callables::SignatureSlot,
    SignatureTypeSubjects:SignatureTypeSubject=>$crate::domain::types::SignatureTypeSubject,
    CatalogScenarioSources:ScenarioSource=>$crate::domain::catalog::evidence::ScenarioSource,
    LocalTransferKeys:Transfer=>$crate::domain::transfer::local::TransferKey,
    ModelTransferKeys:Transfer=>$crate::domain::transfer::model::TransferKey,
    SummaryTransferKeys:Transfer=>$crate::domain::transfer::summary::TransferKey,

    NativePremise:AnalysisPremise=>$crate::domain::analysis::native::NativeAssertionPremise,
    LocalSubject:Subject=>$crate::domain::analysis::local::ObligationSubject,
    BaseEvaluationSubject:Subject=>$crate::domain::analysis::base_evaluation::ObligationSubject,
    BaseCompletionSubject:Subject=>$crate::domain::analysis::base_completion::ObligationSubject,
    SourceCallSubject:Subject=>$crate::domain::analysis::source_call::ObligationSubject,
    EnrichedExecutionSubject:Subject=>$crate::domain::analysis::enriched_execution::ObligationSubject,
    ModelSubject:Subject=>$crate::domain::analysis::model::ObligationSubject,
    SummarySubject:Subject=>$crate::domain::analysis::summary::ObligationSubject,
    StructuralSubject:Subject=>$crate::domain::analysis::structural::ObligationSubject,
    AnalyticEmbeddingSubject:Subject=>$crate::domain::analysis::analytic_embedding::ObligationSubject,
    AnalyticSubject:Subject=>$crate::domain::analysis::analytic::ObligationSubject,
    CatalogCoreSubject:Subject=>$crate::domain::analysis::catalog_core::ObligationSubject,
    CatalogEvidenceSubject:Subject=>$crate::domain::analysis::catalog_evidence::ObligationSubject,
    SelectionSubject:Subject=>$crate::domain::analysis::selection::ObligationSubject,
    SynthesisSubject:Subject=>$crate::domain::analysis::synthesis::ObligationSubject,
    RetrievalSubject:Subject=>$crate::domain::analysis::retrieval::ObligationSubject,
    LocalObligationSource:AnalysisPremise=>$crate::domain::analysis::local::ObligationSource,
    BaseEvaluationObligationSource:AnalysisPremise=>$crate::domain::analysis::base_evaluation::ObligationSource,
    BaseCompletionObligationSource:AnalysisPremise=>$crate::domain::analysis::base_completion::ObligationSource,
    SourceCallObligationSource:AnalysisPremise=>$crate::domain::analysis::source_call::ObligationSource,
    EnrichedExecutionObligationSource:AnalysisPremise=>$crate::domain::analysis::enriched_execution::ObligationSource,
    ModelObligationSource:AnalysisPremise=>$crate::domain::analysis::model::ObligationSource,
    SummaryObligationSource:AnalysisPremise=>$crate::domain::analysis::summary::ObligationSource,
    StructuralObligationSource:AnalysisPremise=>$crate::domain::analysis::structural::ObligationSource,
    AnalyticEmbeddingObligationSource:AnalysisPremise=>$crate::domain::analysis::analytic_embedding::ObligationSource,
    AnalyticObligationSource:AnalysisPremise=>$crate::domain::analysis::analytic::ObligationSource,
    CatalogCoreObligationSource:AnalysisPremise=>$crate::domain::analysis::catalog_core::ObligationSource,
    CatalogEvidenceObligationSource:AnalysisPremise=>$crate::domain::analysis::catalog_evidence::ObligationSource,
    SelectionObligationSource:AnalysisPremise=>$crate::domain::analysis::selection::ObligationSource,
    SynthesisObligationSource:AnalysisPremise=>$crate::domain::analysis::synthesis::ObligationSource,
    RetrievalObligationSource:AnalysisPremise=>$crate::domain::analysis::retrieval::ObligationSource,

    SymbolSequence:SymbolSequence=>$crate::domain::symbols::SymbolSequence,
    EntityReference:EntityReference=>$crate::domain::normalized::entities::EntityRef,
    Argument:Argument=>$crate::domain::calls::CallArgument,
    LocalRun:AnalysisRun=>$crate::domain::analysis::local::AnalysisInvocation,
    LocalParent:AnalysisParent=>$crate::domain::analysis::local::InvocationSource,
    LocalPremise:AnalysisPremise=>$crate::domain::analysis::local::SupportSource,
    LocalCoverageSource:CoverageSource=>$crate::domain::analysis::local::CoverageSource,
    BaseEvaluationRun:AnalysisRun=>$crate::domain::analysis::base_evaluation::AnalysisInvocation,
    BaseEvaluationParent:AnalysisParent=>$crate::domain::analysis::base_evaluation::InvocationSource,
    BaseEvaluationPremise:AnalysisPremise=>$crate::domain::analysis::base_evaluation::SupportSource,
    BaseEvaluationCoverageSource:CoverageSource=>$crate::domain::analysis::base_evaluation::CoverageSource,
    BaseCompletionRun:AnalysisRun=>$crate::domain::analysis::base_completion::AnalysisInvocation,
    BaseCompletionParent:AnalysisParent=>$crate::domain::analysis::base_completion::InvocationSource,
    BaseCompletionPremise:AnalysisPremise=>$crate::domain::analysis::base_completion::SupportSource,
    BaseCompletionCoverageSource:CoverageSource=>$crate::domain::analysis::base_completion::CoverageSource,
    SourceCallRun:AnalysisRun=>$crate::domain::analysis::source_call::AnalysisInvocation,
    SourceCallParent:AnalysisParent=>$crate::domain::analysis::source_call::InvocationSource,
    SourceCallPremise:AnalysisPremise=>$crate::domain::analysis::source_call::SupportSource,
    SourceCallCoverageSource:CoverageSource=>$crate::domain::analysis::source_call::CoverageSource,
    EnrichedExecutionRun:AnalysisRun=>$crate::domain::analysis::enriched_execution::AnalysisInvocation,
    EnrichedExecutionParent:AnalysisParent=>$crate::domain::analysis::enriched_execution::InvocationSource,
    EnrichedExecutionPremise:AnalysisPremise=>$crate::domain::analysis::enriched_execution::SupportSource,
    EnrichedExecutionCoverageSource:CoverageSource=>$crate::domain::analysis::enriched_execution::CoverageSource,
    ModelRun:AnalysisRun=>$crate::domain::analysis::model::AnalysisInvocation,
    ModelParent:AnalysisParent=>$crate::domain::analysis::model::InvocationSource,
    ModelPremise:AnalysisPremise=>$crate::domain::analysis::model::SupportSource,
    ModelCoverageSource:CoverageSource=>$crate::domain::analysis::model::CoverageSource,
    SummaryRun:AnalysisRun=>$crate::domain::analysis::summary::AnalysisInvocation,
    SummaryParent:AnalysisParent=>$crate::domain::analysis::summary::InvocationSource,
    SummaryPremise:AnalysisPremise=>$crate::domain::analysis::summary::SupportSource,
    SummaryCoverageSource:CoverageSource=>$crate::domain::analysis::summary::CoverageSource,
    StructuralRun:AnalysisRun=>$crate::domain::analysis::structural::AnalysisInvocation,
    StructuralParent:AnalysisParent=>$crate::domain::analysis::structural::InvocationSource,
    StructuralPremise:AnalysisPremise=>$crate::domain::analysis::structural::SupportSource,
    StructuralCoverageSource:CoverageSource=>$crate::domain::analysis::structural::CoverageSource,
    AnalyticEmbeddingRun:AnalysisRun=>$crate::domain::analysis::analytic_embedding::AnalysisInvocation,
    AnalyticEmbeddingParent:AnalysisParent=>$crate::domain::analysis::analytic_embedding::InvocationSource,
    AnalyticEmbeddingPremise:AnalysisPremise=>$crate::domain::analysis::analytic_embedding::SupportSource,
    AnalyticEmbeddingCoverageSource:CoverageSource=>$crate::domain::analysis::analytic_embedding::CoverageSource,
    AnalyticRun:AnalysisRun=>$crate::domain::analysis::analytic::AnalysisInvocation,
    AnalyticParent:AnalysisParent=>$crate::domain::analysis::analytic::InvocationSource,
    AnalyticPremise:AnalysisPremise=>$crate::domain::analysis::analytic::SupportSource,
    AnalyticCoverageSource:CoverageSource=>$crate::domain::analysis::analytic::CoverageSource,
    CatalogCoreRun:AnalysisRun=>$crate::domain::analysis::catalog_core::AnalysisInvocation,
    CatalogCoreParent:AnalysisParent=>$crate::domain::analysis::catalog_core::InvocationSource,
    CatalogCorePremise:AnalysisPremise=>$crate::domain::analysis::catalog_core::SupportSource,
    CatalogCoreCoverageSource:CoverageSource=>$crate::domain::analysis::catalog_core::CoverageSource,
    CatalogEvidenceRun:AnalysisRun=>$crate::domain::analysis::catalog_evidence::AnalysisInvocation,
    CatalogEvidenceParent:AnalysisParent=>$crate::domain::analysis::catalog_evidence::InvocationSource,
    CatalogEvidencePremise:AnalysisPremise=>$crate::domain::analysis::catalog_evidence::SupportSource,
    CatalogEvidenceCoverageSource:CoverageSource=>$crate::domain::analysis::catalog_evidence::CoverageSource,
    SelectionRun:AnalysisRun=>$crate::domain::analysis::selection::AnalysisInvocation,
    SelectionParent:AnalysisParent=>$crate::domain::analysis::selection::InvocationSource,
    SelectionPremise:AnalysisPremise=>$crate::domain::analysis::selection::SupportSource,
    SelectionCoverageSource:CoverageSource=>$crate::domain::analysis::selection::CoverageSource,
    SynthesisRun:AnalysisRun=>$crate::domain::analysis::synthesis::AnalysisInvocation,
    SynthesisParent:AnalysisParent=>$crate::domain::analysis::synthesis::InvocationSource,
    SynthesisPremise:AnalysisPremise=>$crate::domain::analysis::synthesis::SupportSource,
    SynthesisCoverageSource:CoverageSource=>$crate::domain::analysis::synthesis::CoverageSource,
    RetrievalRun:AnalysisRun=>$crate::domain::analysis::retrieval::AnalysisInvocation,
    RetrievalParent:AnalysisParent=>$crate::domain::analysis::retrieval::InvocationSource,
    RetrievalPremise:AnalysisPremise=>$crate::domain::analysis::retrieval::SupportSource,
    RetrievalCoverageSource:CoverageSource=>$crate::domain::analysis::retrieval::CoverageSource,
    ProviderSurface:ProviderSurface=>$crate::domain::assertion::ProviderSurface,
    Package:Package=>$crate::domain::input::Package,
    Release:Release=>$crate::domain::input::Release,
    Capture:Capture=>$crate::domain::input::InputRevision,
    AcquisitionOrigin:AcquisitionOrigin=>$crate::domain::input::InputOrigin,
    Source:Source=>$crate::domain::source::SourceArtifact,
    Module:Module=>$crate::domain::source::Module,
    Occurrence:Occurrence=>$crate::domain::source::Occurrence,
    Provider:Provider=>$crate::domain::attribution::Provider,
    Context:Context=>$crate::domain::attribution::AnalysisContext,
    Run:Run=>$crate::domain::attribution::ProviderRun,
    Scope:Scope=>$crate::domain::source::CoverageScope,
    Declaration:Declaration=>$crate::domain::normalized::entities::CallableEntity,
    ClassDeclaration:Declaration=>$crate::domain::normalized::entities::ClassEntity,
    Parameter:Parameter=>$crate::domain::normalized::entities::ParameterEntity,
    Field:Field=>$crate::domain::normalized::entities::FieldEntity,
    Exposure:Exposure=>$crate::domain::normalized::entities::PublicExposure,
    InvocationVariant:InvocationVariant=>$crate::domain::normalized::callables::SignatureVariant,
    NativeModule:NativeModule=>$crate::domain::calls::ProviderModule,
    NativeSymbol:NativeSymbol=>$crate::domain::calls::ProviderSymbol,
    NativeCallable:NativeCallable=>$crate::domain::calls::ProviderCallable,
    SignatureParameter:Parameter=>$crate::domain::calls::SignatureParameter,
    ParameterShape:ParameterShape=>$crate::domain::calls::ParameterShape,
    Type:Type=>$crate::domain::types::TypeTerm,
    TypeVariable:TypeVariable=>$crate::domain::types::TypeVariable,
    TypeSequence:TypeSequence=>$crate::domain::types::TypeSequence,
    CallableParameterList:CallableParameterList=>$crate::domain::types::CallableParameterList,
    CallableParameter:CallableParameter=>$crate::domain::types::CallableParameter,
    TypedDictFieldList:TypedDictFieldList=>$crate::domain::types::TypedDictFieldList,
    TypedDictField:TypedDictField=>$crate::domain::types::TypedDictField,
    Literal:Literal=>$crate::domain::value::Literal,
    PlaceRoot:PlaceRoot=>$crate::domain::value::PlaceRoot,
    PathSegment:PathSegment=>$crate::domain::value::PathSegment,
    AccessPath:AccessPath=>$crate::domain::value::AccessPath,
    Place:Place=>$crate::domain::value::Place,
    Predicate:Predicate=>$crate::domain::value::Predicate,
    LiteralSet:LiteralSet=>$crate::domain::value::LiteralSet,
    EvaluationAtom:EvaluationAtom=>$crate::domain::conditions::EvaluationAtom,
    Condition:Condition=>$crate::domain::conditions::Condition,
    ConditionNode:ConditionNode=>$crate::domain::conditions::ConditionNode,
    AssumptionUniverse:AssumptionUniverse=>$crate::domain::assumptions::AssumptionUniverse,
    Assumption:Assumption=>$crate::domain::assumptions::Assumption,
    AssumptionSet:AssumptionSet=>$crate::domain::assumptions::AssumptionSet,
    Qualification:Qualification=>$crate::domain::assertion::AssertionQualification,
    Evidence:Evidence=>$crate::domain::assertion::Evidence,
    CatalogMember:CatalogMember=>$crate::domain::catalog::CatalogMember,
    CatalogOption:CatalogOption=>$crate::domain::catalog::CatalogOption,
    Scenario:Scenario=>$crate::domain::catalog::evidence::CatalogScenario,
    Original:Original=>$crate::domain::catalog::evidence::OriginalSource,
    RetrievalText:RetrievalText=>$crate::domain::retrieval::CorpusText,
    RetrievalUnit:RetrievalUnit=>$crate::domain::retrieval::Unit,
    RetrievalOrigin:RetrievalOrigin=>$crate::domain::retrieval::Origin,
    AnalysisDefinition:AnalysisDefinition=>$crate::domain::analysis::AnalysisDefinition,
    MethodParameters:MethodParameters=>$crate::domain::analysis::MethodParameters,
    EmbeddingSpecification:EmbeddingSpecification=>$crate::domain::embedding::EmbeddingSpec,
}};}
crate::graph_entity_declarations!(graph_entities, intrinsic);

/// Mechanical nominal mapping for the selected graph vocabulary. Unsupported internal compiler
/// bookkeeping is deliberately not an entity; its producer must fold it into its semantic owner.
/// Nominal membership and optional sum-arm requirements for a bulk closure join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRequirement {
    pub target: Target,
    pub kind: Option<EntityKind>,
    pub subtype: Option<i16>,
}
fn reference_requirement(
    reference: &super::SemanticReference,
) -> Result<ReferenceRequirement, ModelError> {
    let (target, kind) = reference_target(reference)?;
    Ok(ReferenceRequirement {
        target,
        kind,
        subtype: reference.subtype,
    })
}
/// Resolve a typed proof row through the same finite graph vocabulary as semantic references.
pub fn target_for_row(row: super::derivation::RowRef) -> Result<Target, ModelError> {
    reference_target(&super::SemanticReference {
        field: "source",
        target: row.relation(),
        key: *row.bytes(),
        subtype: None,
    })
    .map(|(target, _)| target)
}
pub fn reference_target(
    reference: &super::SemanticReference,
) -> Result<(Target, Option<EntityKind>), ModelError> {
    match reference.target {
    <super::catalog::evidence::ReleaseDeployment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::ScenarioSpan as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::conditions::stability::GuardSubstitution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::DiagnosticAnnotation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::diagnostics::DiagnosticSubject as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_capture::SummaryCaptureContribution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow_inventory::FlowUseCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow_inventory::FlowUseInventoryMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callables::EffectiveCallableEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callables::EffectiveCallablePremise as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callables::EffectiveDecoratorMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::SymbolEntityEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::SymbolEntityPremise as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::CallAlternativeEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::CallEventResolution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::CallEventResolutionEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::EventPhaseTarget as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::FlowCallEventLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::DeclarationNativeCharacterization as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::ImportModuleAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::ImportModuleCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::MentionSymbolCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::PlaceEntityLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::TestOperandCoverage as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::TypeBinderAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::TypeBinderCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::TypeBinderPremise as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::TypeEntityLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::overload_association::OverloadVariantAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::overload_association::OverloadVariantCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::receiver::ReceiverEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::receiver::ReceiverPremise as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::assertions::AssertionSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::assertions::ProgrammaticAssertionSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::briefs::BriefAssertion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::briefs::BriefDocument as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::briefs::BriefSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),

    <super::normalized::links::ReferenceBindingCharacterization as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::SourceFieldLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callables::SignatureSlotType as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callables::SignatureReturnType as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::FieldLocationLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::ConstructorCandidateLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::FieldAccessAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_fields::FieldLocation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_fields::FieldLocationCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::selection::Context as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::selection::Witness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::selection::DomainContext as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::selection::DomainClosure as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::selection::DomainEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::CallEventSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::CallEventSourceEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::SourceCharacterization as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::SourceCharacterizationScenario as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::SourceUsage as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::DiagnosticUseAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::DiagnosticUseLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::DiagnosticUsePath as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::DiagnosticUseTarget as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::evidence::EvidenceRoot as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Scope,reference.target,&reference.key)),Some(EntityKind::Scope))),
    <super::catalog::evidence::RootSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::retrieval::Fragment as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalFragment,reference.target,&reference.key)),Some(EntityKind::RetrievalFragment))),
    <super::retrieval::RetrievalDefinition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalDefinition,reference.target,&reference.key)),Some(EntityKind::RetrievalDefinition))),
    <super::retrieval::consumption::RetrievalEmbeddingUse as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::retrieval::UnitRoot as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::local::ControlSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::local::Selection as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),

    <super::calls::CallResolutionMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::SignatureEnumerationMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::CommunityLabelMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::bindings::BindingSetMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::bindings::BindingSetCoverage as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::dispatch::DispatchPremise as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::dispatch::DispatchEvidence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::summary::ControlInfluence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::summary::ControlSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::summary::Selection as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),

    <super::models::AuthoredTarget as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::embedding::text::TextDefinition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Definition,reference.target,&reference.key)),Some(EntityKind::Definition))),
    <super::embedding::text::TextSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::synthesis::frames::Frame as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::SynthesisFrame,reference.target,&reference.key)),Some(EntityKind::SynthesisFrame))),
    <super::execution::context_execution::ContextItem as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::execution::capture_bridge::CapturedValueSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::normalized::callable_aspects::FieldDefault as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Literal,reference.target,&reference.key)),Some(EntityKind::Literal))),
    <super::assumptions_universe::AssumptionUniverseSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::embedding::text::TextAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::seeds::ConfiguredSeedDecision as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::automatic::Decision as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::Reach as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::controls::UnfollowedArgument as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::handoffs::ValueSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::handoffs::Handoff as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_consequences::ClaimConclusion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::symbolic_fields::SourceFieldClass as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::symbolic_fields::SourceFieldStore as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::symbolic_fields::SourceFieldReader as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::symbolic_fields::SourceFieldAssociation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::EventAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),

    <super::local_theory::DomainValue as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Literal,reference.target,&reference.key)),Some(EntityKind::Literal))),
    <super::deployment::ReportValue as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Literal,reference.target,&reference.key)),Some(EntityKind::Literal))),
    <super::deployment::ReportCollection as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TaskReport,reference.target,&reference.key)),Some(EntityKind::TaskReport))),
    <super::deployment::ReportedEnvironment as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Context,reference.target,&reference.key)),Some(EntityKind::Context))),
    <super::models::ModelCatalog as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ModelCatalog,reference.target,&reference.key)),Some(EntityKind::ModelCatalog))),
    <super::models::AuthoredModel as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AuthoredModel,reference.target,&reference.key)),Some(EntityKind::AuthoredModel))),
    <super::models::AuthoredContextProtocol as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ModeledOperation,reference.target,&reference.key)),Some(EntityKind::ModeledOperation))),
    <super::flow_capture::FlowCaptureTarget as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::synthesis::summary::SummaryFacet as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::execution::completion_records::CompletionOutcome as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Outcome,reference.target,&reference.key)),Some(EntityKind::Outcome))),
    <super::execution::model_rules::ModelValuePath as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Place,reference.target,&reference.key)),Some(EntityKind::Place))),
    <super::execution::model_rules::ModeledOperation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ModeledOperation,reference.target,&reference.key)),Some(EntityKind::ModeledOperation))),
    <super::execution::enriched_records::ExecutionOutcome as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Outcome,reference.target,&reference.key)),Some(EntityKind::Outcome))),
    <super::execution::summary_exceptions::SummaryExceptionOutcome as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Outcome,reference.target,&reference.key)),Some(EntityKind::Outcome))),
    <super::analysis::settings::AnalyticsConfiguration as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::MethodParameters,reference.target,&reference.key)),Some(EntityKind::MethodParameters))),
    <super::normalized::coverage::NormalizationComputation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::local_theory::TypeDomainAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_theory::TypeDomainMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::deployment::ReportEntry as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::CallOriginStep as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::local_symbolic::SymbolicFieldStore as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow_capture::FlowCaptureCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::atom_decision::AtomDecision as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::flow::FlowCallStep as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::projection::ProjectionSourceAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::embedding::text::TextWindow as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::seeds::SeedPlan as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::seeds::SelectedSeedSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::patterns::AuthoredCodeSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::documentary::ProseSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::conditions::entry::EntryAccessSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::PublicCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::Traversal as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::Path as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::UnresolvedEvent as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::controls::LiteralArgument as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::controls::ControlPath as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::controls::ConditionalRaise as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::controls::UnfollowedPath as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::handoffs::Group as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::closed_targets::ClosedTargetAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::model_protocol::ContextResource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::capture_bridge::CapturedEntryBinding as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::DocumentNeighbour as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::CommunityLabel as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::symbolic_fields::SourceFieldReaderLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callable_aspects::FieldDefaultAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::ReferenceEntityTarget as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::ReferenceEntityAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::AncestryEntityAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::MentionEntityAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::TestOperandTypeAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::SymbolEntityCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::ParameterEntityLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::FieldEntityLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::FieldDeclarationLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::PublicExposureCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::CallAlternativeSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callables::SignatureSlotEntity as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::dispatch::DispatchAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::local::TransferAlternative as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::local::TransferSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::model::TransferAlternative as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::model::TransferSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::summary::TransferAlternative as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::summary::TransferSupport as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),

    <super::local_theory::TypeDomain as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::deployment::TaskReport as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TaskReport,reference.target,&reference.key)),Some(EntityKind::TaskReport))),
    <super::calls::CallChannel as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallChannel,reference.target,&reference.key)),Some(EntityKind::CallChannel))),
    <super::calls::CallDestination as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallDestination,reference.target,&reference.key)),Some(EntityKind::CallDestination))),
    <super::calls::Receiver as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallReceiver,reference.target,&reference.key)),Some(EntityKind::CallReceiver))),
    <super::calls::CallOrigin as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallOrigin,reference.target,&reference.key)),Some(EntityKind::CallOrigin))),
    <super::class_metadata::RecordOptions as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RecordOptions,reference.target,&reference.key)),Some(EntityKind::RecordOptions))),
    <super::class_metadata::RecordTransformDefaults as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RecordTransformDefaults,reference.target,&reference.key)),Some(EntityKind::RecordTransformDefaults))),
    <super::lexical::LexicalScope as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::LexicalScope,reference.target,&reference.key)),Some(EntityKind::LexicalScope))),
    <super::lexical::BindingEvent as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::BindingEvent,reference.target,&reference.key)),Some(EntityKind::BindingEvent))),
    <super::lexical::LexicalTarget as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::LexicalTarget,reference.target,&reference.key)),Some(EntityKind::LexicalTarget))),
    <super::symbols::ExportOrigin as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ExportOrigin,reference.target,&reference.key)),Some(EntityKind::ExportOrigin))),
    <super::syntax::SyntaxDetail as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::SyntaxDetail,reference.target,&reference.key)),Some(EntityKind::SyntaxDetail))),
    <super::flow_capture::FlowCaptureInventory as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::FlowCaptureInventory,reference.target,&reference.key)),Some(EntityKind::FlowCaptureInventory))),
    <super::flow::FlowUse as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::FlowUse,reference.target,&reference.key)),Some(EntityKind::FlowUse))),
    <super::flow::FlowDefinition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::FlowDefinition,reference.target,&reference.key)),Some(EntityKind::FlowDefinition))),
    <super::flow::ReachingDefinition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::FlowReachingTarget,reference.target,&reference.key)),Some(EntityKind::FlowReachingTarget))),
    <super::flow::FlowCallPath as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::FlowCallPath,reference.target,&reference.key)),Some(EntityKind::FlowCallPath))),
    <super::documents::DocumentNode as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::DocumentNode,reference.target,&reference.key)),Some(EntityKind::DocumentNode))),
    <super::documents::DocumentAttributeValue as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::DocumentAttributeValue,reference.target,&reference.key)),Some(EntityKind::DocumentAttributeValue))),
    <super::catalog::CatalogExposure as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogExposure,reference.target,&reference.key)),Some(EntityKind::CatalogExposure))),
    <super::catalog::CatalogCandidate as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogCandidate,reference.target,&reference.key)),Some(EntityKind::CatalogCandidate))),
    <super::catalog::CatalogCallable as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogCallable,reference.target,&reference.key)),Some(EntityKind::CatalogCallable))),
    <super::catalog::CatalogClass as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogClass,reference.target,&reference.key)),Some(EntityKind::CatalogClass))),
    <super::catalog::CatalogOptionSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogOptionSubject,reference.target,&reference.key)),Some(EntityKind::CatalogOptionSubject))),
    <super::catalog::CatalogDefault as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogDefault,reference.target,&reference.key)),Some(EntityKind::CatalogDefault))),
    <super::catalog::CatalogOptionEvidence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogOptionEvidence,reference.target,&reference.key)),Some(EntityKind::CatalogOptionEvidence))),
    <super::synthesis::assertions::AssertionTemplate as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AssertionTemplate,reference.target,&reference.key)),Some(EntityKind::AssertionTemplate))),
    <super::structural::StructuralFrame as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::StructuralFrame,reference.target,&reference.key)),Some(EntityKind::StructuralFrame))),
    <super::retrieval::Subject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalSubject,reference.target,&reference.key)),Some(EntityKind::RetrievalSubject))),
    <super::retrieval::AnchorSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnchorSource,reference.target,&reference.key)),Some(EntityKind::AnchorSource))),
    <super::analytics::AnalyticFrame as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalyticFrame,reference.target,&reference.key)),Some(EntityKind::AnalyticFrame))),
    <super::analytics::ConceptScope as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ConceptScope,reference.target,&reference.key)),Some(EntityKind::ConceptScope))),
    <super::normalized::callables::SignatureSlot as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::SignatureSlot,reference.target,&reference.key)),Some(EntityKind::SignatureSlot))),
    <super::types::SignatureTypeSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::SignatureTypeSubject,reference.target,&reference.key)),Some(EntityKind::SignatureTypeSubject))),
    <super::catalog::evidence::ScenarioSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ScenarioSource,reference.target,&reference.key)),Some(EntityKind::ScenarioSource))),
    <super::transfer::local::TransferKey as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Transfer,reference.target,&reference.key)),Some(EntityKind::Transfer))),
    <super::transfer::model::TransferKey as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Transfer,reference.target,&reference.key)),Some(EntityKind::Transfer))),
    <super::transfer::summary::TransferKey as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Transfer,reference.target,&reference.key)),Some(EntityKind::Transfer))),
    <super::local_theory::BuiltinOperandWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::BindingSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::calls::BindingProjection as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::atom_decision::AtomRestriction as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::embedding::analytic::AnalysisEmbeddingUse as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::catalog::CatalogMemberInvocation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::seeds::SelectedSeed as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::patterns::AuthoredCodeConclusion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::documentary::ProseSlice as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::synthesis::documentary::DocumentarySource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::conditions::entry::EntryValueWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::conditions::stability::StabilityWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::structural::ConclusionSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_terminal::SummaryTerminalWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::completion_records::StatementCompletion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::modeled_call::ModeledCallEvaluation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_path::SummaryPathRoute as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::model_production::ModelApplication as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_capture::SummaryCaptureWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::body_records::SourceBodyCompletion as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::protocol_interpretation::ConditionalTerminalFrontier as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::protocol_interpretation::NormalContinuationRestriction as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_consequences::ClaimProof as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_control::SummaryControlWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::definition::DefinitionEvaluation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::model_rules::AppliedRule as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::context_execution::ContextExecution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::enriched_records::StatementExecution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::enriched_records::BodyExecution as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::enriched_records::SourceExecutionInvocation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::context_binding::ContextEntryBinding as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::summary_symbolic::SymbolicFieldAlternative as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::model_context_transfer::ContextTransferWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::source_call_records::SourceCallOutcome as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::execution::source_call_records::SourceFrameRelease as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::summary::SummaryPremise as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::summary::SummaryWitness as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analytics::ConclusionSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::coverage::NormalizationCoverage as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::ReferenceEntityCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::AncestryEntityMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::MentionEntityCandidate as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::links::TestOperandTypeLink as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::entities::OccurrenceOwnership as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::NormalizedCallEvent as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::events::NormalizedCallAlternative as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::callables::EffectiveCallableAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::bindings::BindingSetAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::receiver::ReceiverAssessment as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::normalized::dispatch::DispatchMember as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::transfer::local::ControlInfluence as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),

    <super::analysis::native::NativeAssertionPremise as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::local::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::base_evaluation::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::base_completion::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::source_call::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::enriched_execution::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::model::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::summary::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::structural::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::analytic_embedding::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::analytic::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::catalog_core::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::catalog_evidence::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::selection::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::synthesis::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::retrieval::ObligationSubject as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Subject,reference.target,&reference.key)),Some(EntityKind::Subject))),
    <super::analysis::local::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::base_evaluation::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::base_completion::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::source_call::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::enriched_execution::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::model::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::summary::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::structural::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::analytic_embedding::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::analytic::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::catalog_core::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::catalog_evidence::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::selection::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::synthesis::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::retrieval::ObligationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::native::NativeQualification as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::local::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::local::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::local::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::base_evaluation::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::base_evaluation::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::base_evaluation::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::base_completion::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::base_completion::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::base_completion::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::source_call::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::source_call::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::source_call::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::enriched_execution::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::enriched_execution::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::enriched_execution::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::model::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::model::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::model::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::summary::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::summary::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::summary::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::structural::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::structural::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::structural::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::analytic_embedding::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::analytic_embedding::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::analytic_embedding::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::analytic::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::analytic::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::analytic::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::catalog_core::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::catalog_core::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::catalog_core::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::catalog_evidence::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::catalog_evidence::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::catalog_evidence::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::selection::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::selection::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::selection::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::synthesis::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::synthesis::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::synthesis::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::retrieval::AnalysisObligation as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::retrieval::CoverageRequirement as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),
    <super::analysis::retrieval::CoverageRequiredSource as Record>::NAME=>Ok((Target::Assertion(AssertionId::from_key(reference.target,&reference.key)),None)),

    <super::symbols::SymbolSequence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::SymbolSequence,reference.target,&reference.key)),Some(EntityKind::SymbolSequence))),
    <super::normalized::entities::EntityRef as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::EntityReference,reference.target,&reference.key)),Some(EntityKind::EntityReference))),
    <super::calls::CallArgument as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Argument,reference.target,&reference.key)),Some(EntityKind::Argument))),
    <super::analysis::local::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::local::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::local::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::local::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::base_evaluation::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::base_evaluation::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::base_evaluation::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::base_evaluation::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::base_completion::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::base_completion::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::base_completion::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::base_completion::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::source_call::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::source_call::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::source_call::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::source_call::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::enriched_execution::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::enriched_execution::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::enriched_execution::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::enriched_execution::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::model::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::model::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::model::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::model::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::summary::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::summary::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::summary::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::summary::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::structural::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::structural::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::structural::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::structural::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::analytic_embedding::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::analytic_embedding::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::analytic_embedding::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::analytic_embedding::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::analytic::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::analytic::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::analytic::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::analytic::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::catalog_core::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::catalog_core::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::catalog_core::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::catalog_core::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::catalog_evidence::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::catalog_evidence::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::catalog_evidence::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::catalog_evidence::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::selection::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::selection::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::selection::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::selection::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::synthesis::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::synthesis::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::synthesis::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::synthesis::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::analysis::retrieval::AnalysisInvocation as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisRun,reference.target,&reference.key)),Some(EntityKind::AnalysisRun))),
    <super::analysis::retrieval::InvocationSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisParent,reference.target,&reference.key)),Some(EntityKind::AnalysisParent))),
    <super::analysis::retrieval::SupportSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisPremise,reference.target,&reference.key)),Some(EntityKind::AnalysisPremise))),
    <super::analysis::retrieval::CoverageSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CoverageSource,reference.target,&reference.key)),Some(EntityKind::CoverageSource))),
    <super::assertion::ProviderSurface as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ProviderSurface,reference.target,&reference.key)),Some(EntityKind::ProviderSurface))),
    <super::input::Package as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Package,reference.target,&reference.key)),Some(EntityKind::Package))),
    <super::input::Release as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Release,reference.target,&reference.key)),Some(EntityKind::Release))),
    <super::input::InputRevision as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Capture,reference.target,&reference.key)),Some(EntityKind::Capture))),
    <super::input::InputOrigin as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AcquisitionOrigin,reference.target,&reference.key)),Some(EntityKind::AcquisitionOrigin))),
    <super::source::SourceArtifact as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Source,reference.target,&reference.key)),Some(EntityKind::Source))),
    <super::source::Module as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Module,reference.target,&reference.key)),Some(EntityKind::Module))),
    <super::source::Occurrence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Occurrence,reference.target,&reference.key)),Some(EntityKind::Occurrence))),
    <super::attribution::Provider as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Provider,reference.target,&reference.key)),Some(EntityKind::Provider))),
    <super::attribution::AnalysisContext as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Context,reference.target,&reference.key)),Some(EntityKind::Context))),
    <super::attribution::ProviderRun as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Run,reference.target,&reference.key)),Some(EntityKind::Run))),
    <super::source::CoverageScope as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Scope,reference.target,&reference.key)),Some(EntityKind::Scope))),
    <super::normalized::entities::CallableEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Declaration,reference.target,&reference.key)),Some(EntityKind::Declaration))),
    <super::normalized::entities::ClassEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Declaration,reference.target,&reference.key)),Some(EntityKind::Declaration))),
    <super::normalized::entities::ParameterEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Parameter,reference.target,&reference.key)),Some(EntityKind::Parameter))),
    <super::normalized::entities::FieldEntity as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Field,reference.target,&reference.key)),Some(EntityKind::Field))),
    <super::normalized::entities::PublicExposure as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Exposure,reference.target,&reference.key)),Some(EntityKind::Exposure))),
    <super::normalized::callables::SignatureVariant as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::InvocationVariant,reference.target,&reference.key)),Some(EntityKind::InvocationVariant))),
    <super::calls::ProviderModule as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::NativeModule,reference.target,&reference.key)),Some(EntityKind::NativeModule))),
    <super::calls::ProviderSymbol as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::NativeSymbol,reference.target,&reference.key)),Some(EntityKind::NativeSymbol))),
    <super::calls::ProviderCallable as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::NativeCallable,reference.target,&reference.key)),Some(EntityKind::NativeCallable))),
    <super::calls::SignatureParameter as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Parameter,reference.target,&reference.key)),Some(EntityKind::Parameter))),
    <super::calls::ParameterShape as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ParameterShape,reference.target,&reference.key)),Some(EntityKind::ParameterShape))),
    <super::types::TypeTerm as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Type,reference.target,&reference.key)),Some(EntityKind::Type))),
    <super::types::TypeVariable as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypeVariable,reference.target,&reference.key)),Some(EntityKind::TypeVariable))),
    <super::types::TypeSequence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypeSequence,reference.target,&reference.key)),Some(EntityKind::TypeSequence))),
    <super::types::CallableParameterList as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallableParameterList,reference.target,&reference.key)),Some(EntityKind::CallableParameterList))),
    <super::types::CallableParameter as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CallableParameter,reference.target,&reference.key)),Some(EntityKind::CallableParameter))),
    <super::types::TypedDictFieldList as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypedDictFieldList,reference.target,&reference.key)),Some(EntityKind::TypedDictFieldList))),
    <super::types::TypedDictField as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::TypedDictField,reference.target,&reference.key)),Some(EntityKind::TypedDictField))),
    <super::value::Literal as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Literal,reference.target,&reference.key)),Some(EntityKind::Literal))),
    <super::value::PlaceRoot as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::PlaceRoot,reference.target,&reference.key)),Some(EntityKind::PlaceRoot))),
    <super::value::PathSegment as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::PathSegment,reference.target,&reference.key)),Some(EntityKind::PathSegment))),
    <super::value::AccessPath as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AccessPath,reference.target,&reference.key)),Some(EntityKind::AccessPath))),
    <super::value::Place as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Place,reference.target,&reference.key)),Some(EntityKind::Place))),
    <super::value::Predicate as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Predicate,reference.target,&reference.key)),Some(EntityKind::Predicate))),
    <super::value::LiteralSet as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::LiteralSet,reference.target,&reference.key)),Some(EntityKind::LiteralSet))),
    <super::conditions::EvaluationAtom as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::EvaluationAtom,reference.target,&reference.key)),Some(EntityKind::EvaluationAtom))),
    <super::conditions::Condition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Condition,reference.target,&reference.key)),Some(EntityKind::Condition))),
    <super::conditions::ConditionNode as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::ConditionNode,reference.target,&reference.key)),Some(EntityKind::ConditionNode))),
    <super::assumptions::AssumptionUniverse as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AssumptionUniverse,reference.target,&reference.key)),Some(EntityKind::AssumptionUniverse))),
    <super::assumptions::Assumption as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Assumption,reference.target,&reference.key)),Some(EntityKind::Assumption))),
    <super::assumptions::AssumptionSet as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AssumptionSet,reference.target,&reference.key)),Some(EntityKind::AssumptionSet))),
    <super::assertion::AssertionQualification as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Qualification,reference.target,&reference.key)),Some(EntityKind::Qualification))),
    <super::assertion::Evidence as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Evidence,reference.target,&reference.key)),Some(EntityKind::Evidence))),
    <super::catalog::CatalogMember as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogMember,reference.target,&reference.key)),Some(EntityKind::CatalogMember))),
    <super::catalog::CatalogOption as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::CatalogOption,reference.target,&reference.key)),Some(EntityKind::CatalogOption))),
    <super::catalog::evidence::CatalogScenario as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Scenario,reference.target,&reference.key)),Some(EntityKind::Scenario))),
    <super::catalog::evidence::OriginalSource as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::Original,reference.target,&reference.key)),Some(EntityKind::Original))),
    <super::retrieval::CorpusText as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalText,reference.target,&reference.key)),Some(EntityKind::RetrievalText))),
    <super::retrieval::Unit as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalUnit,reference.target,&reference.key)),Some(EntityKind::RetrievalUnit))),
    <super::retrieval::Origin as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::RetrievalOrigin,reference.target,&reference.key)),Some(EntityKind::RetrievalOrigin))),
    <super::analysis::AnalysisDefinition as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalysisDefinition,reference.target,&reference.key)),Some(EntityKind::AnalysisDefinition))),
    <super::analysis::MethodParameters as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::MethodParameters,reference.target,&reference.key)),Some(EntityKind::MethodParameters))),
    <super::embedding::EmbeddingSpec as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::EmbeddingSpecification,reference.target,&reference.key)),Some(EntityKind::EmbeddingSpecification))),
    <super::analytics::Attribute as Record>::NAME=>Ok((Target::Entity(entity_key(EntityKind::AnalyticAttribute,reference.target,&reference.key)),Some(EntityKind::AnalyticAttribute))),
    _=>assertion_reference_target(reference),
    }
}
fn assertion_reference_target(
    reference: &super::SemanticReference,
) -> Result<(Target, Option<EntityKind>), ModelError> {
    match reference.target {
        <super::structural::UsageSite as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::structural::UsageEvidence as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::normalized::events::CallPolicyAssessment as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::normalized::events::CallPolicyAdmission as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),

        <super::analytics::UniverseMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::PublicSelector as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::PartitionMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::CommunityMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::ConceptObject as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::ConceptExtent as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::ConceptIntent as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::ImplicationMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::LayerPair as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::CombinedPair as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::CommunityRun as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::CommunityProfile as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::VectorSelection as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::CommunityLabelAssessment as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::LayerResult as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::LayerNeighbour as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::GraphArc as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::PairSource as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::PairContribution as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::IncidenceSource as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::Incidence as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::TypeMetadataSelection as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::DecoratorSelection as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowUseObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowDefinitionObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowReachingObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowValueObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowRegionObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowTestObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowTestLeafObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::Signature as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::CallTarget as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::ProviderCallSite as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::CallSyntax as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::CallResolution as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::declarations::SymbolDeclaration as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::declarations::ParameterDeclaration as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::deployment::TaskReportObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::deployment::DeploymentObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::PassageObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::CodeBlockObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentLinkObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentMentionObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentComponentObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentAttributeObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowAttributeLoadObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowValuePathObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::LexicalScopeObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::BindingObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::ReferenceObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::LexicalResolution as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::source::SyntaxObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::SymbolObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::FunctionTraitObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ClassTraitObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ClassAncestryObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ParameterAnnotationObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::PublicNameObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ParameterDocObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::SyntaxPlacement as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::SyntaxDetailObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::DeclarationObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::DeclarationDecorator as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::ImportAliasObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::DunderAllObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::ParameterSyntaxObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::ClassFieldSyntaxObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypeObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypePresentation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypeVariableRestriction as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::FunctionBodyObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::RecordFieldObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::class_metadata::ClassMemberObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::class_metadata::ClassMetadataObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::SignatureEnumerationObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::ruff::RuffContextObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowNarrowingObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::protocols::NativeExitObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::protocols::NativeTerminalObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::captures::CaptureObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::protocols::NativeExitDiagnostic as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowSourceViewObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::NativeSignatureObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::SignatureTypeObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypeQueryObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::GenericSpecializationObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow_capture::FlowCaptureTimingObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ExportEnumerationObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::ruff::RuffBindingObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::diagnostics::RuffDiagnosticObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::diagnostics::PyreflyDiagnosticObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::diagnostics::NativeParameterDefinitionObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::ruff::RuffDefinitionObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow_inventory::FlowUseInventoryObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::NativeOverloadObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::NativeOverloadCandidate as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ModuleResolutionObservation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::local_semantics::LocalAssessment as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::local_semantics::LocalContribution as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::local_semantics::LocalGuardContribution as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::local_theory::TheoryWitness as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::execution::records::ExpressionEvaluation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::execution::source_call_records::SourceCallHeader as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::execution::source_call_records::SourceInvocation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::execution::model_transfer::ModelTransferWitness as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::execution::summary_consequences::SummaryClaim as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::execution::summary_path::SummaryPathWitness as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::execution::summary_production::SummaryRun as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::normalized::entities::SymbolEntityResolution as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::normalized::entities::PublicEnumerationAssessment as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::normalized::bindings::CallBindingAttempt as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::normalized::bindings::CallBinding as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::normalized::bindings::BindingVariantAssessment as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::CatalogInvocation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::CatalogPath as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::CatalogAlias as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::CatalogConstructor as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::evidence::ScenarioAssociation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::evidence::DocumentAssociation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::evidence::CatalogDeployment as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::catalog::evidence::ScenarioCheck as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::selection::SelectionDomain as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::structural::Conclusion as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::structural::UsageScore as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::structural::controls::ControlTraversal as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::Conclusion as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::TechniqueResult as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::RankScore as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::Community as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::Neighbour as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::Concept as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analytics::Implication as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::synthesis::documentary::DocumentaryConclusion as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::synthesis::assertions::ProgrammaticAssertion as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::synthesis::briefs::Brief as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::retrieval::OriginalAnchor as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::retrieval::UnitSubject as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowUseSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowDefinitionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowReachingSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowValueSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowRegionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowTestSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowTestLeafSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::SignatureSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::CallTargetSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::ProviderCallSiteSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::CallSyntaxSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::CallResolutionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::declarations::SymbolDeclarationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::declarations::ParameterDeclarationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::deployment::TaskReportSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::deployment::DeploymentSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::PassageSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::CodeBlockSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentLinkSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentMentionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentComponentSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::documents::DocumentAttributeSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowAttributeLoadSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowValuePathSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::LexicalScopeSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::BindingSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::ReferenceSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::lexical::LexicalResolutionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::source::SyntaxSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::SymbolSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::FunctionTraitSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ClassTraitSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ClassAncestrySupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ParameterAnnotationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::PublicNameSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ParameterDocSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::SyntaxPlacementSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::SyntaxDetailSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::DeclarationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::DeclarationDecoratorSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::ImportAliasSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::DunderAllSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::ParameterSyntaxSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::syntax::ClassFieldSyntaxSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypeSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypePresentationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypeRestrictionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::FunctionBodySupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::RecordFieldSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::class_metadata::ClassMemberSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::class_metadata::ClassMetadataSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::calls::SignatureEnumerationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::ruff::RuffContextSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowNarrowingSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::protocols::NativeExitSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::protocols::NativeTerminalSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::captures::CaptureSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::protocols::NativeExitDiagnosticSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow::FlowSourceViewSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::NativeSignatureSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::SignatureTypeSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypeQuerySupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::GenericSpecializationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow_capture::FlowCaptureTimingSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ExportEnumerationSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::ruff::RuffBindingSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::diagnostics::RuffDiagnosticSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::diagnostics::PyreflyDiagnosticSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::diagnostics::NativeParameterDefinitionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::ruff::RuffDefinitionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::flow_inventory::FlowUseInventorySupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::NativeOverloadSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::NativeOverloadCandidateSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::ModuleResolutionSupport as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::InputAcquisition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::CorpusLibrary as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::ArtifactUse as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::ArtifactOwnership as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::UnownedArtifact as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::DerivedArtifact as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::InputDistribution as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::DistributionVerification as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::input::EnvironmentFingerprint as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::attribution::ProviderCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::attribution::RunFamily as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::local::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::local::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::local::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::local::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::local::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::local::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::local::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_evaluation::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_evaluation::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_evaluation::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_evaluation::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_evaluation::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_evaluation::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_evaluation::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_completion::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_completion::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_completion::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_completion::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_completion::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_completion::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::base_completion::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::source_call::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::source_call::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::source_call::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::source_call::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::source_call::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::source_call::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::source_call::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::enriched_execution::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::enriched_execution::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::enriched_execution::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::enriched_execution::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::enriched_execution::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::enriched_execution::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::enriched_execution::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::model::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::model::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::model::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::model::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::model::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::model::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::model::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::summary::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::summary::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::summary::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::summary::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::summary::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::summary::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::summary::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::structural::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::structural::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::structural::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::structural::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::structural::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::structural::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::structural::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic_embedding::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic_embedding::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic_embedding::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic_embedding::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic_embedding::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic_embedding::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic_embedding::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::analytic::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_core::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_core::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_core::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_core::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_core::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_core::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_core::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_evidence::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_evidence::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_evidence::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_evidence::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_evidence::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_evidence::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::catalog_evidence::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::selection::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::selection::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::selection::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::selection::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::selection::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::selection::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::selection::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::synthesis::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::synthesis::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::synthesis::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::synthesis::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::synthesis::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::synthesis::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::synthesis::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::retrieval::AnalysisInput as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::retrieval::AnalysisOutcome as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::retrieval::AnalysisCoverage as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::retrieval::AnalysisCoveragePremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::retrieval::AnalysisProposition as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::retrieval::AnalysisDerivation as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::analysis::retrieval::AnalysisDerivationPremise as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::types::TypeSequenceMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::symbols::SymbolSequenceMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::assumptions::AssumptionSetMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        <super::value::LiteralSetMember as Record>::NAME => Ok((
            Target::Assertion(AssertionId::from_key(reference.target, &reference.key)),
            None,
        )),
        _ => Err(ModelError::Invalid(format!(
            "internal reference {}.{} has no selected semantic graph owner",
            reference.target, reference.field
        ))),
    }
}
impl From<super::flow::FlowUseObservation> for NativeValue {
    fn from(row: super::flow::FlowUseObservation) -> Self {
        Self::Use(row)
    }
}
impl From<super::flow::FlowDefinitionObservation> for NativeValue {
    fn from(row: super::flow::FlowDefinitionObservation) -> Self {
        Self::Definition(row)
    }
}
impl From<super::flow::FlowReachingObservation> for NativeValue {
    fn from(row: super::flow::FlowReachingObservation) -> Self {
        Self::Reaching(row)
    }
}
impl From<super::flow::FlowValueObservation> for NativeValue {
    fn from(row: super::flow::FlowValueObservation) -> Self {
        Self::Value(row)
    }
}
impl From<super::flow::FlowRegionObservation> for NativeValue {
    fn from(row: super::flow::FlowRegionObservation) -> Self {
        Self::Region(row)
    }
}
impl From<super::flow::FlowTestObservation> for NativeValue {
    fn from(row: super::flow::FlowTestObservation) -> Self {
        Self::Test(row)
    }
}
impl From<super::flow::FlowTestLeafObservation> for NativeValue {
    fn from(row: super::flow::FlowTestLeafObservation) -> Self {
        Self::Leaf(row)
    }
}
impl From<super::calls::Signature> for NativeValue {
    fn from(row: super::calls::Signature) -> Self {
        Self::Signature(row)
    }
}
impl From<super::calls::CallTarget> for NativeValue {
    fn from(row: super::calls::CallTarget) -> Self {
        Self::CallTarget(row)
    }
}
impl From<super::calls::ProviderCallSite> for NativeValue {
    fn from(row: super::calls::ProviderCallSite) -> Self {
        Self::ProviderCallSite(row)
    }
}
impl From<super::calls::CallSyntax> for NativeValue {
    fn from(row: super::calls::CallSyntax) -> Self {
        Self::CallSyntax(row)
    }
}
impl From<super::calls::CallResolution> for NativeValue {
    fn from(row: super::calls::CallResolution) -> Self {
        Self::CallResolution(row)
    }
}
impl From<super::declarations::SymbolDeclaration> for NativeValue {
    fn from(row: super::declarations::SymbolDeclaration) -> Self {
        Self::SymbolDeclaration(row)
    }
}
impl From<super::declarations::ParameterDeclaration> for NativeValue {
    fn from(row: super::declarations::ParameterDeclaration) -> Self {
        Self::ParameterDeclaration(row)
    }
}
impl From<super::deployment::TaskReportObservation> for NativeValue {
    fn from(row: super::deployment::TaskReportObservation) -> Self {
        Self::TaskReportObservation(row)
    }
}
impl From<super::deployment::DeploymentObservation> for NativeValue {
    fn from(row: super::deployment::DeploymentObservation) -> Self {
        Self::DeploymentObservation(row)
    }
}
impl From<super::documents::DocumentObservation> for NativeValue {
    fn from(row: super::documents::DocumentObservation) -> Self {
        Self::DocumentObservation(row)
    }
}
impl From<super::documents::PassageObservation> for NativeValue {
    fn from(row: super::documents::PassageObservation) -> Self {
        Self::PassageObservation(row)
    }
}
impl From<super::documents::CodeBlockObservation> for NativeValue {
    fn from(row: super::documents::CodeBlockObservation) -> Self {
        Self::CodeBlockObservation(row)
    }
}
impl From<super::documents::DocumentLinkObservation> for NativeValue {
    fn from(row: super::documents::DocumentLinkObservation) -> Self {
        Self::DocumentLinkObservation(row)
    }
}
impl From<super::documents::DocumentMentionObservation> for NativeValue {
    fn from(row: super::documents::DocumentMentionObservation) -> Self {
        Self::DocumentMentionObservation(row)
    }
}
impl From<super::documents::DocumentComponentObservation> for NativeValue {
    fn from(row: super::documents::DocumentComponentObservation) -> Self {
        Self::DocumentComponentObservation(row)
    }
}
impl From<super::documents::DocumentAttributeObservation> for NativeValue {
    fn from(row: super::documents::DocumentAttributeObservation) -> Self {
        Self::DocumentAttributeObservation(row)
    }
}
impl From<super::flow::FlowAttributeLoadObservation> for NativeValue {
    fn from(row: super::flow::FlowAttributeLoadObservation) -> Self {
        Self::FlowAttributeLoadObservation(row)
    }
}
impl From<super::flow::FlowValuePathObservation> for NativeValue {
    fn from(row: super::flow::FlowValuePathObservation) -> Self {
        Self::FlowValuePathObservation(row)
    }
}
impl From<super::lexical::LexicalScopeObservation> for NativeValue {
    fn from(row: super::lexical::LexicalScopeObservation) -> Self {
        Self::LexicalScopeObservation(row)
    }
}
impl From<super::lexical::BindingObservation> for NativeValue {
    fn from(row: super::lexical::BindingObservation) -> Self {
        Self::BindingObservation(row)
    }
}
impl From<super::lexical::ReferenceObservation> for NativeValue {
    fn from(row: super::lexical::ReferenceObservation) -> Self {
        Self::ReferenceObservation(row)
    }
}
impl From<super::lexical::LexicalResolution> for NativeValue {
    fn from(row: super::lexical::LexicalResolution) -> Self {
        Self::LexicalResolution(row)
    }
}
impl From<super::source::SyntaxObservation> for NativeValue {
    fn from(row: super::source::SyntaxObservation) -> Self {
        Self::SyntaxObservation(row)
    }
}
impl From<super::symbols::SymbolObservation> for NativeValue {
    fn from(row: super::symbols::SymbolObservation) -> Self {
        Self::SymbolObservation(row)
    }
}
impl From<super::symbols::FunctionTraitObservation> for NativeValue {
    fn from(row: super::symbols::FunctionTraitObservation) -> Self {
        Self::FunctionTraitObservation(row)
    }
}
impl From<super::symbols::ClassTraitObservation> for NativeValue {
    fn from(row: super::symbols::ClassTraitObservation) -> Self {
        Self::ClassTraitObservation(row)
    }
}
impl From<super::symbols::ClassAncestryObservation> for NativeValue {
    fn from(row: super::symbols::ClassAncestryObservation) -> Self {
        Self::ClassAncestryObservation(row)
    }
}
impl From<super::symbols::ParameterAnnotationObservation> for NativeValue {
    fn from(row: super::symbols::ParameterAnnotationObservation) -> Self {
        Self::ParameterAnnotationObservation(row)
    }
}
impl From<super::symbols::PublicNameObservation> for NativeValue {
    fn from(row: super::symbols::PublicNameObservation) -> Self {
        Self::PublicNameObservation(row)
    }
}
impl From<super::symbols::ParameterDocObservation> for NativeValue {
    fn from(row: super::symbols::ParameterDocObservation) -> Self {
        Self::ParameterDocObservation(row)
    }
}
impl From<super::syntax::SyntaxPlacement> for NativeValue {
    fn from(row: super::syntax::SyntaxPlacement) -> Self {
        Self::SyntaxPlacement(row)
    }
}
impl From<super::syntax::SyntaxDetailObservation> for NativeValue {
    fn from(row: super::syntax::SyntaxDetailObservation) -> Self {
        Self::SyntaxDetailObservation(row)
    }
}
impl From<super::syntax::DeclarationObservation> for NativeValue {
    fn from(row: super::syntax::DeclarationObservation) -> Self {
        Self::DeclarationObservation(row)
    }
}
impl From<super::syntax::DeclarationDecorator> for NativeValue {
    fn from(row: super::syntax::DeclarationDecorator) -> Self {
        Self::DeclarationDecorator(row)
    }
}
impl From<super::syntax::ImportAliasObservation> for NativeValue {
    fn from(row: super::syntax::ImportAliasObservation) -> Self {
        Self::ImportAliasObservation(row)
    }
}
impl From<super::syntax::DunderAllObservation> for NativeValue {
    fn from(row: super::syntax::DunderAllObservation) -> Self {
        Self::DunderAllObservation(row)
    }
}
impl From<super::syntax::ParameterSyntaxObservation> for NativeValue {
    fn from(row: super::syntax::ParameterSyntaxObservation) -> Self {
        Self::ParameterSyntaxObservation(row)
    }
}
impl From<super::syntax::ClassFieldSyntaxObservation> for NativeValue {
    fn from(row: super::syntax::ClassFieldSyntaxObservation) -> Self {
        Self::ClassFieldSyntaxObservation(row)
    }
}
impl From<super::types::TypeObservation> for NativeValue {
    fn from(row: super::types::TypeObservation) -> Self {
        Self::TypeObservation(row)
    }
}
impl From<super::types::TypePresentation> for NativeValue {
    fn from(row: super::types::TypePresentation) -> Self {
        Self::TypePresentation(row)
    }
}
impl From<super::types::TypeVariableRestriction> for NativeValue {
    fn from(row: super::types::TypeVariableRestriction) -> Self {
        Self::TypeVariableRestriction(row)
    }
}
impl From<super::types::FunctionBodyObservation> for NativeValue {
    fn from(row: super::types::FunctionBodyObservation) -> Self {
        Self::FunctionBodyObservation(row)
    }
}
impl From<super::types::RecordFieldObservation> for NativeValue {
    fn from(row: super::types::RecordFieldObservation) -> Self {
        Self::RecordFieldObservation(row)
    }
}
impl From<super::class_metadata::ClassMemberObservation> for NativeValue {
    fn from(row: super::class_metadata::ClassMemberObservation) -> Self {
        Self::ClassMemberObservation(row)
    }
}
impl From<super::class_metadata::ClassMetadataObservation> for NativeValue {
    fn from(row: super::class_metadata::ClassMetadataObservation) -> Self {
        Self::ClassMetadataObservation(row)
    }
}
impl From<super::calls::SignatureEnumerationObservation> for NativeValue {
    fn from(row: super::calls::SignatureEnumerationObservation) -> Self {
        Self::SignatureEnumerationObservation(row)
    }
}
impl From<super::ruff::RuffContextObservation> for NativeValue {
    fn from(row: super::ruff::RuffContextObservation) -> Self {
        Self::RuffContextObservation(row)
    }
}
impl From<super::flow::FlowNarrowingObservation> for NativeValue {
    fn from(row: super::flow::FlowNarrowingObservation) -> Self {
        Self::FlowNarrowing(row)
    }
}
impl From<super::protocols::NativeExitObservation> for NativeValue {
    fn from(row: super::protocols::NativeExitObservation) -> Self {
        Self::NativeExit(row)
    }
}
impl From<super::protocols::NativeTerminalObservation> for NativeValue {
    fn from(row: super::protocols::NativeTerminalObservation) -> Self {
        Self::NativeTerminal(row)
    }
}
impl From<super::captures::CaptureObservation> for NativeValue {
    fn from(row: super::captures::CaptureObservation) -> Self {
        Self::Capture(row)
    }
}
impl From<super::protocols::NativeExitDiagnostic> for NativeValue {
    fn from(row: super::protocols::NativeExitDiagnostic) -> Self {
        Self::NativeExitDiagnostic(row)
    }
}
impl From<super::flow::FlowSourceViewObservation> for NativeValue {
    fn from(row: super::flow::FlowSourceViewObservation) -> Self {
        Self::FlowSourceView(row)
    }
}
impl From<super::types::NativeSignatureObservation> for NativeValue {
    fn from(row: super::types::NativeSignatureObservation) -> Self {
        Self::NativeSignatureObservation(row)
    }
}
impl From<super::types::SignatureTypeObservation> for NativeValue {
    fn from(row: super::types::SignatureTypeObservation) -> Self {
        Self::SignatureTypeObservation(row)
    }
}
impl From<super::types::TypeQueryObservation> for NativeValue {
    fn from(row: super::types::TypeQueryObservation) -> Self {
        Self::TypeQuery(row)
    }
}
impl From<super::types::GenericSpecializationObservation> for NativeValue {
    fn from(row: super::types::GenericSpecializationObservation) -> Self {
        Self::GenericSpecialization(row)
    }
}
impl From<super::flow_capture::FlowCaptureTimingObservation> for NativeValue {
    fn from(row: super::flow_capture::FlowCaptureTimingObservation) -> Self {
        Self::FlowCaptureTiming(row)
    }
}
impl From<super::symbols::ExportEnumerationObservation> for NativeValue {
    fn from(row: super::symbols::ExportEnumerationObservation) -> Self {
        Self::ExportEnumeration(row)
    }
}
impl From<super::ruff::RuffBindingObservation> for NativeValue {
    fn from(row: super::ruff::RuffBindingObservation) -> Self {
        Self::RuffBindingObservation(row)
    }
}
impl From<super::diagnostics::RuffDiagnosticObservation> for NativeValue {
    fn from(row: super::diagnostics::RuffDiagnosticObservation) -> Self {
        Self::RuffDiagnosticObservation(row)
    }
}
impl From<super::diagnostics::PyreflyDiagnosticObservation> for NativeValue {
    fn from(row: super::diagnostics::PyreflyDiagnosticObservation) -> Self {
        Self::PyreflyDiagnosticObservation(row)
    }
}
impl From<super::diagnostics::NativeParameterDefinitionObservation> for NativeValue {
    fn from(row: super::diagnostics::NativeParameterDefinitionObservation) -> Self {
        Self::NativeParameterDefinitionObservation(row)
    }
}
impl From<super::ruff::RuffDefinitionObservation> for NativeValue {
    fn from(row: super::ruff::RuffDefinitionObservation) -> Self {
        Self::RuffDefinitionObservation(row)
    }
}
impl From<super::flow_inventory::FlowUseInventoryObservation> for NativeValue {
    fn from(row: super::flow_inventory::FlowUseInventoryObservation) -> Self {
        Self::FlowUseInventory(row)
    }
}
impl From<super::types::NativeOverloadObservation> for NativeValue {
    fn from(row: super::types::NativeOverloadObservation) -> Self {
        Self::NativeOverload(row)
    }
}
impl From<super::types::NativeOverloadCandidate> for NativeValue {
    fn from(row: super::types::NativeOverloadCandidate) -> Self {
        Self::NativeOverloadCandidate(row)
    }
}
impl From<super::symbols::ModuleResolutionObservation> for NativeValue {
    fn from(row: super::symbols::ModuleResolutionObservation) -> Self {
        Self::ModuleResolutionObservation(row)
    }
}
impl From<super::local_semantics::LocalAssessment> for AnalysisValue {
    fn from(row: super::local_semantics::LocalAssessment) -> Self {
        Self::LocalAssessment(row)
    }
}
impl From<super::local_semantics::LocalContribution> for AnalysisValue {
    fn from(row: super::local_semantics::LocalContribution) -> Self {
        Self::LocalTransfer(row)
    }
}
impl From<super::local_semantics::LocalGuardContribution> for AnalysisValue {
    fn from(row: super::local_semantics::LocalGuardContribution) -> Self {
        Self::LocalGuard(row)
    }
}
impl From<super::local_theory::TheoryWitness> for AnalysisValue {
    fn from(row: super::local_theory::TheoryWitness) -> Self {
        Self::Theory(row)
    }
}
impl From<super::execution::records::ExpressionEvaluation> for AnalysisValue {
    fn from(row: super::execution::records::ExpressionEvaluation) -> Self {
        Self::ExpressionEvaluation(row)
    }
}
impl From<super::execution::source_call_records::SourceCallHeader> for AnalysisValue {
    fn from(row: super::execution::source_call_records::SourceCallHeader) -> Self {
        Self::SourceCall(row)
    }
}
impl From<super::execution::source_call_records::SourceInvocation> for AnalysisValue {
    fn from(row: super::execution::source_call_records::SourceInvocation) -> Self {
        Self::SourceInvocation(row)
    }
}
impl From<super::execution::model_transfer::ModelTransferWitness> for AnalysisValue {
    fn from(row: super::execution::model_transfer::ModelTransferWitness) -> Self {
        Self::ModelTransfer(row)
    }
}
impl From<super::execution::summary_consequences::SummaryClaim> for AnalysisValue {
    fn from(row: super::execution::summary_consequences::SummaryClaim) -> Self {
        Self::SummaryClaim(row)
    }
}
impl From<super::execution::summary_path::SummaryPathWitness> for AnalysisValue {
    fn from(row: super::execution::summary_path::SummaryPathWitness) -> Self {
        Self::SummaryPath(row)
    }
}
impl From<super::execution::summary_production::SummaryRun> for AnalysisValue {
    fn from(row: super::execution::summary_production::SummaryRun) -> Self {
        Self::SummaryRun(row)
    }
}
impl From<super::normalized::entities::SymbolEntityResolution> for AnalysisValue {
    fn from(row: super::normalized::entities::SymbolEntityResolution) -> Self {
        Self::NormalizedSymbol(row)
    }
}
impl From<super::normalized::entities::PublicEnumerationAssessment> for AnalysisValue {
    fn from(row: super::normalized::entities::PublicEnumerationAssessment) -> Self {
        Self::PublicEnumeration(row)
    }
}
impl From<super::normalized::bindings::CallBindingAttempt> for AnalysisValue {
    fn from(row: super::normalized::bindings::CallBindingAttempt) -> Self {
        Self::BindingAttempt(row)
    }
}
impl From<super::normalized::bindings::CallBinding> for AnalysisValue {
    fn from(row: super::normalized::bindings::CallBinding) -> Self {
        Self::Binding(row)
    }
}
impl From<super::normalized::bindings::BindingVariantAssessment> for AnalysisValue {
    fn from(row: super::normalized::bindings::BindingVariantAssessment) -> Self {
        Self::BindingVariant(row)
    }
}
impl From<super::catalog::CatalogInvocation> for AnalysisValue {
    fn from(row: super::catalog::CatalogInvocation) -> Self {
        Self::CatalogInvocation(row)
    }
}
impl From<super::catalog::CatalogPath> for AnalysisValue {
    fn from(row: super::catalog::CatalogPath) -> Self {
        Self::CatalogPath(row)
    }
}
impl From<super::catalog::CatalogAlias> for AnalysisValue {
    fn from(row: super::catalog::CatalogAlias) -> Self {
        Self::CatalogAlias(row)
    }
}
impl From<super::catalog::CatalogConstructor> for AnalysisValue {
    fn from(row: super::catalog::CatalogConstructor) -> Self {
        Self::CatalogConstructor(row)
    }
}
impl From<super::catalog::evidence::ScenarioAssociation> for AnalysisValue {
    fn from(row: super::catalog::evidence::ScenarioAssociation) -> Self {
        Self::ScenarioAssociation(row)
    }
}
impl From<super::catalog::evidence::DocumentAssociation> for AnalysisValue {
    fn from(row: super::catalog::evidence::DocumentAssociation) -> Self {
        Self::DocumentAssociation(row)
    }
}
impl From<super::catalog::evidence::CatalogDeployment> for AnalysisValue {
    fn from(row: super::catalog::evidence::CatalogDeployment) -> Self {
        Self::Deployment(row)
    }
}
impl From<super::catalog::evidence::ScenarioCheck> for AnalysisValue {
    fn from(row: super::catalog::evidence::ScenarioCheck) -> Self {
        Self::ScenarioCheck(row)
    }
}
impl From<super::selection::SelectionDomain> for AnalysisValue {
    fn from(row: super::selection::SelectionDomain) -> Self {
        Self::SelectionDomain(row)
    }
}
impl From<super::structural::Conclusion> for AnalysisValue {
    fn from(row: super::structural::Conclusion) -> Self {
        Self::StructuralConclusion(row)
    }
}
impl From<super::structural::UsageScore> for AnalysisValue {
    fn from(row: super::structural::UsageScore) -> Self {
        Self::StructuralUsage(row)
    }
}
impl From<super::structural::controls::ControlTraversal> for AnalysisValue {
    fn from(row: super::structural::controls::ControlTraversal) -> Self {
        Self::StructuralControl(row)
    }
}
impl From<super::analytics::Conclusion> for AnalysisValue {
    fn from(row: super::analytics::Conclusion) -> Self {
        Self::AnalyticConclusion(row)
    }
}
impl From<super::analytics::TechniqueResult> for AnalysisValue {
    fn from(row: super::analytics::TechniqueResult) -> Self {
        Self::AnalyticTechnique(row)
    }
}
impl From<super::analytics::RankScore> for AnalysisValue {
    fn from(row: super::analytics::RankScore) -> Self {
        Self::Rank(row)
    }
}
impl From<super::analytics::Community> for AnalysisValue {
    fn from(row: super::analytics::Community) -> Self {
        Self::Community(row)
    }
}
impl From<super::analytics::Neighbour> for AnalysisValue {
    fn from(row: super::analytics::Neighbour) -> Self {
        Self::Neighbour(row)
    }
}
impl From<super::analytics::Concept> for AnalysisValue {
    fn from(row: super::analytics::Concept) -> Self {
        Self::Concept(row)
    }
}
impl From<super::analytics::Implication> for AnalysisValue {
    fn from(row: super::analytics::Implication) -> Self {
        Self::Implication(row)
    }
}
impl From<super::synthesis::documentary::DocumentaryConclusion> for AnalysisValue {
    fn from(row: super::synthesis::documentary::DocumentaryConclusion) -> Self {
        Self::DocumentaryConclusion(row)
    }
}
impl From<super::synthesis::assertions::ProgrammaticAssertion> for AnalysisValue {
    fn from(row: super::synthesis::assertions::ProgrammaticAssertion) -> Self {
        Self::ProgrammaticAssertion(row)
    }
}
impl From<super::synthesis::briefs::Brief> for AnalysisValue {
    fn from(row: super::synthesis::briefs::Brief) -> Self {
        Self::Brief(row)
    }
}
impl From<super::retrieval::OriginalAnchor> for AnalysisValue {
    fn from(row: super::retrieval::OriginalAnchor) -> Self {
        Self::RetrievalAnchor(row)
    }
}
impl From<super::retrieval::UnitSubject> for AnalysisValue {
    fn from(row: super::retrieval::UnitSubject) -> Self {
        Self::RetrievalOccurrence(row)
    }
}
/// Unresolved external Python targets retain their provider/context and reason explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Target {
    Entity(EntityId),
    Assertion(AssertionId),
    External {
        provider: EntityId,
        context: EntityId,
        name: String,
        reason: super::normalized::entities::EntityReason,
    },
}
impl Key for Target {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::Entity(id) => {
                sink.part(b"target", &[0]);
                id.encode(sink);
            }
            Self::Assertion(id) => {
                sink.part(b"target", &[1]);
                id.encode(sink);
            }
            Self::External {
                provider,
                context,
                name,
                reason,
            } => {
                sink.part(b"target", &[2]);
                provider.encode(sink);
                context.encode(sink);
                name.encode(sink);
                reason.encode(sink);
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum ParticipantRole {
    Subject = 0,
    Object = 1,
    Caller = 2,
    Callee = 3,
    Argument = 4,
    Parameter = 5,
    Exposure = 6,
    Declaration = 7,
    Evidence = 8,
    Context = 9,
    Qualification = 10,
    Provider = 11,
    Scope = 12,
    Condition = 13,
    Assumption = 14,
    Run = 15,
    Source = 16,
    Parent = 17,
    Premise = 18,
    Member = 19,
    Owner = 20,
    Invocation = 21,
    Definition = 22,
    Variant = 23,
    Type = 24,
    Literal = 25,
    Place = 26,
    Claim = 27,
    Field = 28,
    Attribute = 29,
    Left = 30,
    Right = 31,
    Result = 32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participant {
    pub role: ParticipantRole,
    pub field: Option<String>,
    pub position: Option<u32>,
    pub target: Target,
}
impl Key for Participant {
    fn encode(&self, sink: &mut KeySink) {
        sink.part(b"role", &(self.role as u16).to_le_bytes());
        self.field.encode(sink);
        match self.position {
            Some(n) => {
                sink.part(b"position", &[1]);
                sink.part(b"u32", &n.to_le_bytes());
            }
            None => sink.part(b"position", &[0]),
        };
        self.target.encode(sink);
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InlineQualification {
    pub context: EntityId,
    pub scope: EntityId,
    pub condition: EntityId,
    pub modality: super::attribution::Modality,
    pub approximation: super::assertion::Approximation,
    pub assumptions: Vec<EntityId>,
}
impl Key for InlineQualification {
    fn encode(&self, sink: &mut KeySink) {
        self.context.encode(sink);
        self.scope.encode(sink);
        self.condition.encode(sink);
        self.modality.encode(sink);
        self.approximation.encode(sink);
        self.assumptions.encode(sink);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Qualification {
    Ref(EntityId),
    Inline(InlineQualification),
    Payload,
}
impl Key for Qualification {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::Ref(id) => {
                sink.part(b"qualification", &[0]);
                id.encode(sink);
            }
            Self::Inline(value) => {
                sink.part(b"qualification", &[1]);
                value.encode(sink);
            }
            Self::Payload => sink.part(b"qualification", &[2]),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticKey {
    domain: String,
    key: [u8; 16],
}
impl SemanticKey {
    pub fn domain(&self) -> &str {
        &self.domain
    }
    pub fn bytes(&self) -> &[u8; 16] {
        &self.key
    }
    pub fn of<R: Record>(id: Id<R>) -> Self {
        Self {
            domain: R::NAME.into(),
            key: *id.bytes(),
        }
    }
}
impl Key for SemanticKey {
    fn encode(&self, sink: &mut KeySink) {
        self.domain.encode(sink);
        sink.part(b"key", &self.key);
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum AssertionKind {
    SyntaxSpelling = 0,
    DeclarationCorrespondence = 1,
    ExposureTarget = 2,
    InvocationSignature = 3,
    ParameterBinding = 4,
    EvidenceAssociation = 5,
    SelectionAssessment = 6,
    DerivedConclusion = 7,
    NativeObservation = 8,
    ValueTransfer = 9,
    Requirement = 10,
    PredicateDomain = 11,
    OptionContract = 12,
    ScenarioAssociation = 13,
    DeploymentAssociation = 14,
    SummaryTransfer = 15,
    StructuralMembership = 16,
    StructuralOrder = 17,
    ConceptMembership = 18,
    EmbeddingWitness = 19,
    RetrievalOccurrence = 20,
    RetrievalFamilyMembership = 21,
    DocumentaryEvidence = 22,
    SynthesisClaim = 23,
    InputAcquisition = 24,
    InputDistribution = 25,
    CorpusMembership = 26,
    ArtifactUse = 27,
}
/// A finite payload vocabulary; provider answers remain separate attributed assertions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AssertionValue {
    None,
    Text(String),
    Applicability(super::normalized::entities::ResolutionStatus),
    Verdict(super::obligation::Verdict),
    Order(i64),
    Domain {
        members: Vec<EntityId>,
        closed: bool,
    },
    Signature {
        parameters: Vec<EntityId>,
        returns: Option<EntityId>,
    },
    Predicate {
        predicate: EntityId,
    },
    Native(NativeValue),
    Analysis(AnalysisValue),
    Support(SupportValue),
    Provenance(ProvenanceValue),
    Membership(MembershipValue),
    Claim(ClaimValue),
    Acquisition(super::input::InputOrigin),
}
impl Key for AssertionValue {
    fn encode(&self, sink: &mut KeySink) {
        match self {
            Self::Claim(value) => {
                sink.part(b"value", &[14]);
                value.encode(sink);
            }
            Self::None => sink.part(b"value", &[0]),
            Self::Text(v) => {
                sink.part(b"value", &[1]);
                v.encode(sink);
            }
            Self::Applicability(v) => {
                sink.part(b"value", &[2]);
                v.encode(sink);
            }
            Self::Verdict(v) => {
                sink.part(b"value", &[3]);
                v.encode(sink);
            }
            Self::Order(v) => {
                sink.part(b"value", &[4]);
                v.encode(sink);
            }
            Self::Domain { members, closed } => {
                sink.part(b"value", &[5]);
                members.encode(sink);
                closed.encode(sink);
            }
            Self::Signature {
                parameters,
                returns,
            } => {
                sink.part(b"value", &[6]);
                parameters.encode(sink);
                returns.encode(sink);
            }
            Self::Predicate { predicate } => {
                sink.part(b"value", &[7]);
                predicate.encode(sink);
            }
            Self::Native(value) => {
                sink.part(b"value", &[8]);
                value.encode(sink);
            }
            Self::Analysis(value) => {
                sink.part(b"value", &[9]);
                value.encode(sink);
            }
            Self::Acquisition(value) => {
                sink.part(b"value", &[10]);
                value.write_key(sink);
            }
            Self::Support(value) => {
                sink.part(b"value", &[11]);
                value.encode(sink);
            }
            Self::Membership(value) => {
                sink.part(b"value", &[13]);
                value.encode(sink);
            }
            Self::Provenance(value) => {
                sink.part(b"value", &[12]);
                value.encode(sink);
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum OutcomeKind {
    Complete = 0,
    Partial = 1,
    Unavailable = 2,
    Failed = 3,
    Refused = 4,
    NotRequested = 5,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derivation {
    pub rule: String,
    pub revision: u32,
    pub conclusion: Option<Target>,
    pub premises: Vec<Target>,
    pub assumptions: Vec<EntityId>,
    pub outcome: OutcomeKind,
}
impl Key for Derivation {
    fn encode(&self, sink: &mut KeySink) {
        self.rule.encode(sink);
        sink.part(b"u32", &self.revision.to_le_bytes());
        self.conclusion.encode(sink);
        self.premises.encode(sink);
        self.assumptions.encode(sink);
        sink.part(b"outcome", &(self.outcome as u16).to_le_bytes());
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    pub source: Option<SemanticKey>,
    pub kind: AssertionKind,
    pub participants: Vec<Participant>,
    pub qualification: Qualification,
    pub run: Option<EntityId>,
    pub evidence: Vec<EntityId>,
    pub value: AssertionValue,
    pub derivation: Option<Derivation>,
}
/// Only the finite selected semantic assertion inventory may be lowered.
/// Payload conversion is private to the declaration; use `Assertion::from_record` to retain
/// its role-labelled nominal references and ordered domain derivation.
pub trait GraphAssertionRecord: Record {
    const GRAPH_KIND: AssertionKind;
    fn graph_payload(row: Self) -> Assertion;
}
fn participant_role(field: &str) -> ParticipantRole {
    use ParticipantRole as R;
    match field {
        "qualification" => R::Qualification,
        "provider" => R::Provider,
        "context" => R::Context,
        "scope" => R::Scope,
        "condition" => R::Condition,
        "assumption" | "assumptions" | "universe" => R::Assumption,
        "run" => R::Run,
        "source" | "artifact" | "input" | "revision" => R::Source,
        "parent" => R::Parent,
        "premise" | "support" => R::Premise,
        "member" => R::Member,
        "owner" | "package" | "distribution" => R::Owner,
        "invocation" => R::Invocation,
        "definition" => R::Definition,
        "variant" | "callable" => R::Variant,
        "type" | "ty" | "returns" => R::Type,
        "literal" => R::Literal,
        "place" => R::Place,
        "assertion" | "proposition" | "derivation" | "conclusion" => R::Claim,
        "caller" => R::Caller,
        "callee" | "target" => R::Callee,
        "argument" => R::Argument,
        "parameter" | "formal" | "slot" => R::Parameter,
        "exposure" => R::Exposure,
        "declaration" => R::Declaration,
        "evidence" | "observation" => R::Evidence,
        "field" => R::Field,
        "subject" | "occurrence" | "entity" | "representative" => R::Subject,
        "attribute" => R::Attribute,
        "left" => R::Left,
        "right" => R::Right,
        "result" => R::Result,
        _ => R::Object,
    }
}
impl Assertion {
    pub fn from_record<R: GraphAssertionRecord>(row: R) -> Result<Self, ModelError> {
        let references = row.references();
        let proof = row.proof();
        let declaration = R::derivation();
        let participants = references
            .iter()
            .enumerate()
            .map(|(position, reference)| {
                Ok(Participant {
                    role: participant_role(reference.field),
                    field: Some(reference.field.into()),
                    position: Some(
                        u32::try_from(position)
                            .map_err(|_| invalid("participant ordinal overflow"))?,
                    ),
                    target: reference_target(reference)?.0,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let derivation = match (proof, declaration) {
            (Some(proof), Some(declaration)) => {
                let premises = proof
                    .premises
                    .iter()
                    .map(|premise| {
                        reference_target(&super::SemanticReference {
                            field: "premise",
                            target: premise.relation(),
                            key: *premise.bytes(),
                            subtype: None,
                        })
                        .map(|(target, _)| target)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some(Derivation {
                    rule: declaration.rule.into(),
                    revision: 1,
                    conclusion: if proof.source == proof.conclusion {
                        None
                    } else {
                        Some(
                            reference_target(&super::SemanticReference {
                                field: "conclusion",
                                target: proof.conclusion.relation(),
                                key: *proof.conclusion.bytes(),
                                subtype: None,
                            })?
                            .0,
                        )
                    },
                    premises,
                    assumptions: vec![],
                    outcome: OutcomeKind::Complete,
                })
            }
            (None, None) => None,
            _ => return Err(invalid("derivation declaration and row disagree")),
        };
        let mut assertion = R::graph_payload(row);
        assertion.participants = participants;
        assertion.derivation = derivation;
        assertion.validate()?;
        Ok(assertion)
    }
    pub fn declared_derivation(&self) -> Option<GraphDerivation> {
        self.derivation.as_ref().map(|derivation| GraphDerivation {
            source: Target::Assertion(self.id()),
            conclusion: derivation
                .conclusion
                .clone()
                .unwrap_or(Target::Assertion(self.id())),
            premises: derivation.premises.clone(),
        })
    }
    pub fn id(&self) -> AssertionId {
        if let Some(source) = &self.source {
            return AssertionId::from_key(&source.domain, &source.key);
        }
        let mut sink = KeySink::new("graph-assertion/v1");
        self.encode(&mut sink);
        AssertionId(sink.finish())
    }
    pub fn content(&self) -> ContentHash {
        let mut sink = KeySink::new("graph-assertion-content/v1");
        self.encode(&mut sink);
        sink.finish()
    }
    /// Nominal reference obligations for bounded bulk closure over completed families. The
    /// compiler emits these as a compact stream and joins against membership once.
    pub fn references(&self) -> Result<Vec<(Target, Option<EntityKind>)>, ModelError> {
        use EntityKind as K;
        let mut refs = Vec::new();
        match &self.qualification {
            Qualification::Ref(id) => refs.push((Target::Entity(*id), Some(K::Qualification))),
            Qualification::Inline(q) => {
                refs.extend([
                    (Target::Entity(q.context), Some(K::Context)),
                    (Target::Entity(q.scope), Some(K::Scope)),
                    (Target::Entity(q.condition), Some(K::Condition)),
                ]);
                refs.extend(
                    q.assumptions
                        .iter()
                        .map(|id| (Target::Entity(*id), Some(K::Assumption))),
                );
            }
            Qualification::Payload => {}
        }
        refs.extend(self.run.map(|id| (Target::Entity(id), Some(K::Run))));
        refs.extend(
            self.evidence
                .iter()
                .map(|id| (Target::Entity(*id), Some(K::Evidence))),
        );
        refs.extend(self.participants.iter().map(|p| {
            let kind = if p.field.is_some() {
                None
            } else {
                match p.role {
                    ParticipantRole::Declaration => Some(K::Declaration),
                    ParticipantRole::Exposure => Some(K::Exposure),
                    ParticipantRole::Context => Some(K::Context),
                    ParticipantRole::Evidence => Some(K::Evidence),
                    ParticipantRole::Argument => Some(K::Occurrence),
                    ParticipantRole::Parameter => Some(K::Parameter),
                    _ => None,
                }
            };
            (p.target.clone(), kind)
        }));
        let payload = match &self.value {
            AssertionValue::Claim(value) => value.references(),
            AssertionValue::Native(value) => value.references(),
            AssertionValue::Analysis(value) => value.references(),
            AssertionValue::Support(value) => value.references(),
            AssertionValue::Membership(value) => value.references(),
            AssertionValue::Provenance(value) => value.references(),
            _ => vec![],
        };
        for reference in payload {
            refs.push(reference_target(&reference)?);
        }
        match &self.value {
            AssertionValue::Domain { members, .. } => {
                refs.extend(members.iter().map(|id| (Target::Entity(*id), None)))
            }
            AssertionValue::Signature {
                parameters,
                returns,
            } => {
                refs.extend(
                    parameters
                        .iter()
                        .map(|id| (Target::Entity(*id), Some(K::Parameter))),
                );
                refs.extend(returns.map(|id| (Target::Entity(id), Some(K::Type))));
            }
            AssertionValue::Predicate { predicate } => {
                refs.push((Target::Entity(*predicate), Some(K::Predicate)))
            }
            _ => {}
        }
        if let Some(d) = &self.derivation {
            refs.extend(d.conclusion.iter().map(|target| (target.clone(), None)));
            refs.extend(d.premises.iter().map(|target| (target.clone(), None)));
            refs.extend(
                d.assumptions
                    .iter()
                    .map(|id| (Target::Entity(*id), Some(K::Assumption))),
            );
        }
        Ok(refs)
    }
    pub fn reference_requirements(&self) -> Result<Vec<ReferenceRequirement>, ModelError> {
        let mut refs = self
            .references()?
            .into_iter()
            .map(|(target, kind)| ReferenceRequirement {
                target,
                kind,
                subtype: None,
            })
            .collect::<Vec<_>>();
        let payload = match &self.value {
            AssertionValue::Claim(v) => v.references(),
            AssertionValue::Native(v) => v.references(),
            AssertionValue::Analysis(v) => v.references(),
            AssertionValue::Support(v) => v.references(),
            AssertionValue::Membership(v) => v.references(),
            AssertionValue::Provenance(v) => v.references(),
            _ => vec![],
        };
        for reference in payload
            .iter()
            .filter(|reference| reference.subtype.is_some())
        {
            refs.push(reference_requirement(reference)?);
        }
        Ok(refs)
    }
    fn encode(&self, sink: &mut KeySink) {
        self.source.encode(sink);
        sink.part(b"kind", &(self.kind as u16).to_le_bytes());
        self.participants.encode(sink);
        self.qualification.encode(sink);
        self.run.encode(sink);
        self.evidence.encode(sink);
        self.value.encode(sink);
        self.derivation.encode(sink);
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        match &self.value {
            AssertionValue::Claim(v) => v.validate()?,
            AssertionValue::Native(v) => v.validate()?,
            AssertionValue::Analysis(v) => v.validate()?,
            AssertionValue::Acquisition(v) => v.validate()?,
            AssertionValue::Support(v) => v.validate()?,
            AssertionValue::Membership(v) => v.validate()?,
            AssertionValue::Provenance(v) => v.validate()?,
            _ => {}
        }
        let typed_references = match &self.value {
            AssertionValue::Claim(v) => Some(v.references()),
            AssertionValue::Native(v) => Some(v.references()),
            AssertionValue::Analysis(v) => Some(v.references()),
            AssertionValue::Support(v) => Some(v.references()),
            AssertionValue::Membership(v) => Some(v.references()),
            AssertionValue::Provenance(v) => Some(v.references()),
            _ => None,
        };
        if let Some(references) = typed_references {
            let expected = references
                .iter()
                .enumerate()
                .map(|(position, reference)| {
                    Ok(Participant {
                        role: participant_role(reference.field),
                        field: Some(reference.field.into()),
                        position: Some(
                            u32::try_from(position)
                                .map_err(|_| invalid("participant ordinal overflow"))?,
                        ),
                        target: reference_target(reference)?.0,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            if self.participants != expected {
                return Err(invalid(
                    "assertion participants differ from its typed role declaration",
                ));
            }
        }
        let expected_source = match &self.value {
            AssertionValue::Acquisition(v) => Some(SemanticKey::of(v.id())),
            AssertionValue::Claim(v) => Some(v.semantic_key()),
            AssertionValue::Native(v) => Some(v.semantic_key()),
            AssertionValue::Analysis(v) => Some(v.semantic_key()),
            AssertionValue::Support(v) => Some(v.semantic_key()),
            AssertionValue::Membership(v) => Some(v.semantic_key()),
            AssertionValue::Provenance(v) => Some(v.semantic_key()),
            _ => None,
        };
        if expected_source.is_some() && self.source != expected_source {
            return Err(invalid(
                "assertion key differs from its typed semantic payload",
            ));
        }
        if matches!(self.qualification, Qualification::Payload)
            && !matches!(
                self.value,
                AssertionValue::Claim(_)
                    | AssertionValue::Native(_)
                    | AssertionValue::Analysis(_)
                    | AssertionValue::Support(_)
                    | AssertionValue::Membership(_)
                    | AssertionValue::Provenance(_)
            )
        {
            return Err(invalid(
                "qualification payload needs a typed contextual owner",
            ));
        }
        if self.participants.is_empty()
            && !matches!(
                self.value,
                AssertionValue::Claim(_)
                    | AssertionValue::Native(_)
                    | AssertionValue::Analysis(_)
                    | AssertionValue::Support(_)
                    | AssertionValue::Membership(_)
                    | AssertionValue::Provenance(_)
            )
        {
            return Err(invalid("assertion needs participants"));
        }
        if matches!(&self.qualification,Qualification::Inline(q) if !strictly_ordered(&q.assumptions))
            || !strictly_ordered(&self.evidence)
        {
            return Err(invalid("assertion sets must be sorted and unique"));
        }
        if let Some(derivation) = &self.derivation
            && (derivation.rule.is_empty()
                || derivation.revision == 0
                || !strictly_ordered(&derivation.assumptions))
        {
            return Err(invalid(
                "derivation needs rule revision, ordered premises and canonical assumptions",
            ));
        }
        // Premises and participants are sequences: never sort them to manufacture canonicality.
        Ok(())
    }
}
fn strictly_ordered<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|w| w[0] < w[1])
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

/// Membership lookup is supplied by immutable completed local indexes. It is not a store grant.
pub trait GraphLookup {
    fn entity_kind(&self, id: EntityId) -> Result<Option<EntityKind>, ModelError>;
    fn entity_subtype(&self, _id: EntityId) -> Result<Option<i16>, ModelError> {
        Ok(None)
    }
    fn assertion_exists(&self, id: AssertionId) -> Result<bool, ModelError>;
    fn source_length(&self, id: EntityId) -> Result<Option<u64>, ModelError>;
}
pub fn admit_entity(entity: &Entity, lookup: &impl GraphLookup) -> Result<(), ModelError> {
    entity.validate()?;
    for reference in entity.reference_requirements()? {
        admit_requirement(reference, lookup)?;
    }
    if let Entity::Occurrence(row) = entity
        && lookup
            .source_length(EntityId::of(row.source))?
            .is_none_or(|length| u64::try_from(row.end).ok().is_none_or(|end| end > length))
    {
        return Err(invalid("occurrence extends beyond captured source"));
    }
    if let Entity::Evidence(super::assertion::Evidence::SourceSpan { source, end, .. }) = entity
        && lookup
            .source_length(EntityId::of(*source))?
            .is_none_or(|length| u64::try_from(*end).ok().is_none_or(|end| end > length))
    {
        return Err(invalid("evidence extends beyond captured source"));
    }
    Ok(())
}
fn require_kind(
    lookup: &impl GraphLookup,
    id: EntityId,
    kind: EntityKind,
) -> Result<(), ModelError> {
    if lookup.entity_kind(id)? != Some(kind) {
        return Err(invalid("missing internal entity or wrong nominal kind"));
    }
    Ok(())
}
fn admit_requirement(
    reference: ReferenceRequirement,
    lookup: &impl GraphLookup,
) -> Result<(), ModelError> {
    if let Some(subtype) = reference.subtype {
        match reference.target {
            Target::Entity(id) => {
                if lookup.entity_subtype(id)? != Some(subtype) {
                    return Err(invalid("internal entity has wrong nominal sum arm"));
                }
            }
            _ => return Err(invalid("sum arm requires an internal entity")),
        }
    }
    admit_reference(reference.target, reference.kind, lookup)
}
fn admit_reference(
    target: Target,
    kind: Option<EntityKind>,
    lookup: &impl GraphLookup,
) -> Result<(), ModelError> {
    match target {
        Target::Entity(id) => {
            if let Some(kind) = kind {
                require_kind(lookup, id, kind)?;
            } else if lookup.entity_kind(id)?.is_none() {
                return Err(invalid("missing internal participant"));
            }
        }
        Target::Assertion(id) => {
            if kind.is_some() || !lookup.assertion_exists(id)? {
                return Err(invalid(
                    "missing referenced assertion or wrong nominal role",
                ));
            }
        }
        Target::External {
            provider,
            context,
            name,
            ..
        } => {
            if kind.is_some() {
                return Err(invalid(
                    "external target cannot fill an internal nominal role",
                ));
            }
            require_kind(lookup, provider, EntityKind::Provider)?;
            require_kind(lookup, context, EntityKind::Context)?;
            if name.is_empty() {
                return Err(invalid("external target needs name"));
            }
        }
    }
    Ok(())
}
pub fn admit_assertion(assertion: &Assertion, lookup: &impl GraphLookup) -> Result<(), ModelError> {
    assertion.validate()?;
    for reference in assertion.reference_requirements()? {
        admit_requirement(reference, lookup)?;
    }
    Ok(())
}

pub const ARTIFACT_FORMAT_VERSION: u32 = 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u16)]
pub enum GraphFamily {
    Entities = 0,
    Assertions = 1,
    Outcomes = 2,
    Originals = 3,
    Projection = 4,
    EmbeddingValues = 5,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyContent {
    pub family: GraphFamily,
    pub rows: u64,
    pub content: ContentHash,
}
/// Incremental canonical content, independent of IPC framing and transport fragmentation.
/// Ordered equal keys deduplicate only when the complete semantic payload agrees.
pub struct FamilyHasher {
    family: GraphFamily,
    sink: KeySink,
    previous: Option<(ContentHash, ContentHash)>,
    rows: u64,
}
impl FamilyHasher {
    pub fn new(family: GraphFamily) -> Self {
        let mut sink = KeySink::new("graph-family/v1");
        sink.part(b"family", &(family as u16).to_le_bytes());
        Self {
            family,
            sink,
            previous: None,
            rows: 0,
        }
    }
    pub fn push(&mut self, key: ContentHash, content: ContentHash) -> Result<bool, ModelError> {
        if let Some((previous, payload)) = self.previous {
            if key < previous {
                return Err(invalid("graph family must be ordered by canonical key"));
            }
            if key == previous {
                if content != payload {
                    return Err(ModelError::Conflict("graph family"));
                }
                return Ok(false);
            }
        }
        key.encode(&mut self.sink);
        content.encode(&mut self.sink);
        self.previous = Some((key, content));
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or_else(|| invalid("graph family row count overflow"))?;
        Ok(true)
    }
    pub fn finish(self) -> FamilyContent {
        FamilyContent {
            family: self.family,
            rows: self.rows,
            content: self.sink.finish(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerImplementation {
    pub producer: String,
    pub implementation: ContentHash,
    pub configuration: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeKey {
    pub producer: String,
    pub scope: EntityId,
    pub domain: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub key: OutcomeKey,
    pub status: OutcomeKind,
    pub observed: Option<u64>,
    pub detail: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    pub source: EntityId,
    pub content: ContentHash,
    pub byte_len: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionDefinition {
    pub name: String,
    pub definition: ContentHash,
    pub source_membership: ContentHash,
    pub declared_losses: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingConsumption {
    pub specification: ContentHash,
    pub text: ContentHash,
    pub dimension: u32,
    pub values: ContentHash,
}
/// Content declaration of a compiler artifact. Physical indexes, functions and engine versions
/// enter a publisher's realization identity, never this semantic content manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format_version: u32,
    pub frontier: super::admission::Frontier,
    pub profile: super::stages::Profile,
    pub captures: Vec<EntityId>,
    pub semantic_contract: ContentHash,
    pub producers: Vec<ProducerImplementation>,
    pub settings: ContentHash,
    pub families: Vec<FamilyContent>,
    pub required_outcomes: Vec<OutcomeKey>,
    pub outcomes: Vec<Outcome>,
    pub originals: Vec<Original>,
    pub projections: Vec<ProjectionDefinition>,
    pub embeddings: Vec<EmbeddingConsumption>,
}
impl Manifest {
    pub fn content(&self) -> ContentHash {
        let mut sink = KeySink::new("graph-artifact/v1");
        sink.part(b"u32", &self.format_version.to_le_bytes());
        sink.part(b"frontier", self.frontier.name().as_bytes());
        sink.part(b"profile", self.profile.name().as_bytes());
        self.captures.encode(&mut sink);
        self.semantic_contract.encode(&mut sink);
        self.settings.encode(&mut sink);
        sink.part(b"producers", &(self.producers.len() as u64).to_le_bytes());
        for p in &self.producers {
            p.producer.encode(&mut sink);
            p.implementation.encode(&mut sink);
            p.configuration.encode(&mut sink);
        }
        sink.part(b"families", &(self.families.len() as u64).to_le_bytes());
        for family in &self.families {
            sink.part(b"family", &(family.family as u16).to_le_bytes());
            sink.part(b"u64", &family.rows.to_le_bytes());
            family.content.encode(&mut sink);
        }
        sink.part(
            b"required-outcomes",
            &(self.required_outcomes.len() as u64).to_le_bytes(),
        );
        for key in &self.required_outcomes {
            encode_outcome_key(key, &mut sink);
        }
        sink.part(b"outcomes", &(self.outcomes.len() as u64).to_le_bytes());
        for outcome in &self.outcomes {
            encode_outcome_key(&outcome.key, &mut sink);
            sink.part(b"status", &(outcome.status as u16).to_le_bytes());
            match outcome.observed {
                Some(n) => {
                    sink.part(b"observed", &[1]);
                    sink.part(b"u64", &n.to_le_bytes());
                }
                None => sink.part(b"observed", &[0]),
            };
            outcome.detail.encode(&mut sink);
        }
        sink.part(b"originals", &(self.originals.len() as u64).to_le_bytes());
        for original in &self.originals {
            original.source.encode(&mut sink);
            original.content.encode(&mut sink);
            sink.part(b"u64", &original.byte_len.to_le_bytes());
        }
        sink.part(
            b"projections",
            &(self.projections.len() as u64).to_le_bytes(),
        );
        for projection in &self.projections {
            projection.name.encode(&mut sink);
            projection.definition.encode(&mut sink);
            projection.source_membership.encode(&mut sink);
            projection.declared_losses.encode(&mut sink);
        }
        sink.part(b"embeddings", &(self.embeddings.len() as u64).to_le_bytes());
        for vector in &self.embeddings {
            vector.specification.encode(&mut sink);
            vector.text.encode(&mut sink);
            sink.part(b"u32", &vector.dimension.to_le_bytes());
            vector.values.encode(&mut sink);
        }
        sink.finish()
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.format_version != ARTIFACT_FORMAT_VERSION
            || self.frontier == super::admission::Frontier::Conformance
        {
            return Err(invalid("unsupported artifact format or frontier"));
        }
        if !strictly_ordered(&self.captures) || !strictly_ordered(&self.required_outcomes) {
            return Err(invalid(
                "manifest captures and obligations must be ordered and unique",
            ));
        }
        if !self.families.windows(2).all(|w| w[0].family < w[1].family)
            || !self
                .producers
                .windows(2)
                .all(|w| w[0].producer < w[1].producer)
        {
            return Err(invalid(
                "manifest families and producers must be ordered and unique",
            ));
        }
        if self.producers.iter().any(|p| p.producer.is_empty()) {
            return Err(invalid("manifest producer needs name"));
        }
        if self.outcomes.len() != self.required_outcomes.len()
            || self
                .outcomes
                .iter()
                .zip(&self.required_outcomes)
                .any(|(actual, expected)| actual.key != *expected)
        {
            return Err(invalid(
                "manifest outcomes differ from exact required domain",
            ));
        }
        for outcome in &self.outcomes {
            if outcome.key.producer.is_empty()
                || (outcome.status == OutcomeKind::NotRequested
                    && outcome.observed.is_some_and(|observed| observed != 0))
            {
                return Err(invalid("invalid outcome metadata"));
            }
        }
        if !self.originals.windows(2).all(|w| w[0].source < w[1].source)
            || !self.projections.windows(2).all(|w| w[0].name < w[1].name)
            || !self
                .embeddings
                .windows(2)
                .all(|w| (w[0].specification, w[0].text) < (w[1].specification, w[1].text))
        {
            return Err(invalid(
                "manifest originals, projections and embeddings must be ordered and unique",
            ));
        }
        if self.embeddings.iter().any(|v| v.dimension == 0) {
            return Err(invalid("embedding consumption needs dimensions"));
        }
        Ok(())
    }
}
fn encode_outcome_key(key: &OutcomeKey, sink: &mut KeySink) {
    key.producer.encode(sink);
    key.scope.encode(sink);
    key.domain.encode(sink);
}

/// Actual declared semantic dependency, distinct from ordinary graph adjacency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDerivation {
    pub source: Target,
    pub conclusion: Target,
    pub premises: Vec<Target>,
}
fn graph_derivation(
    proof: Option<super::derivation::Proof>,
) -> Result<Option<GraphDerivation>, ModelError> {
    let target = |row: super::derivation::RowRef| {
        reference_target(&super::SemanticReference {
            field: "premise",
            target: row.relation(),
            key: *row.bytes(),
            subtype: None,
        })
        .map(|(target, _)| target)
    };
    proof
        .map(|proof| {
            Ok(GraphDerivation {
                source: target(proof.source)?,
                conclusion: target(proof.conclusion)?,
                premises: proof
                    .premises
                    .into_iter()
                    .map(target)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .transpose()
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Dependency {
    Entity(EntityId),
    Assertion(AssertionId),
}
impl Dependency {
    fn of(target: &Target) -> Result<Self, ModelError> {
        match target {
            Target::Entity(id) => Ok(Self::Entity(*id)),
            Target::Assertion(id) => Ok(Self::Assertion(*id)),
            Target::External { .. } => Err(invalid(
                "external uncertainty cannot be an internal derivation dependency",
            )),
        }
    }
}
/// Compact cycle check across intrinsic evidence and assertions. This does not replay producer
/// checks or treat ordinary graph relationships as proof edges.
pub fn admit_graph_derivations<'a>(
    derivations: impl IntoIterator<Item = &'a GraphDerivation>,
) -> Result<(), ModelError> {
    let mut graph: petgraph::graphmap::DiGraphMap<Dependency, ()> = Default::default();
    for derivation in derivations {
        let source = Dependency::of(&derivation.source)?;
        let conclusion = Dependency::of(&derivation.conclusion)?;
        graph.add_node(source);
        if source != conclusion {
            graph.add_edge(conclusion, source, ());
        }
        for premise in &derivation.premises {
            graph.add_edge(source, Dependency::of(premise)?, ());
        }
    }
    petgraph::algo::toposort(&graph, None)
        .map(|_| ())
        .map_err(|_| invalid("cyclic semantic derivation"))
}

/// Check only the compact derivation dependency topology. Publication does not replay producers.
pub fn admit_derivations<'a>(
    derivations: impl IntoIterator<Item = (AssertionId, &'a [Target])>,
) -> Result<(), ModelError> {
    let mut graph: petgraph::graphmap::DiGraphMap<AssertionId, ()> =
        petgraph::graphmap::DiGraphMap::new();
    for (assertion, premises) in derivations {
        graph.add_node(assertion);
        for premise in premises {
            if let Target::Assertion(id) = premise {
                graph.add_edge(assertion, *id, ());
            }
        }
    }
    petgraph::algo::toposort(&graph, None)
        .map(|_| ())
        .map_err(|_| invalid("cyclic derivation premises"))
}

impl From<super::flow::FlowUseSupport> for SupportValue {
    fn from(row: super::flow::FlowUseSupport) -> Self {
        Self::UseSupport(row)
    }
}
impl From<super::flow::FlowDefinitionSupport> for SupportValue {
    fn from(row: super::flow::FlowDefinitionSupport) -> Self {
        Self::DefinitionSupport(row)
    }
}
impl From<super::flow::FlowReachingSupport> for SupportValue {
    fn from(row: super::flow::FlowReachingSupport) -> Self {
        Self::ReachingSupport(row)
    }
}
impl From<super::flow::FlowValueSupport> for SupportValue {
    fn from(row: super::flow::FlowValueSupport) -> Self {
        Self::ValueSupport(row)
    }
}
impl From<super::flow::FlowRegionSupport> for SupportValue {
    fn from(row: super::flow::FlowRegionSupport) -> Self {
        Self::RegionSupport(row)
    }
}
impl From<super::flow::FlowTestSupport> for SupportValue {
    fn from(row: super::flow::FlowTestSupport) -> Self {
        Self::TestSupport(row)
    }
}
impl From<super::flow::FlowTestLeafSupport> for SupportValue {
    fn from(row: super::flow::FlowTestLeafSupport) -> Self {
        Self::LeafSupport(row)
    }
}
impl From<super::calls::SignatureSupport> for SupportValue {
    fn from(row: super::calls::SignatureSupport) -> Self {
        Self::SignatureSupport(row)
    }
}
impl From<super::calls::CallTargetSupport> for SupportValue {
    fn from(row: super::calls::CallTargetSupport) -> Self {
        Self::CallTargetSupport(row)
    }
}
impl From<super::calls::ProviderCallSiteSupport> for SupportValue {
    fn from(row: super::calls::ProviderCallSiteSupport) -> Self {
        Self::ProviderCallSiteSupport(row)
    }
}
impl From<super::calls::CallSyntaxSupport> for SupportValue {
    fn from(row: super::calls::CallSyntaxSupport) -> Self {
        Self::CallSyntaxSupport(row)
    }
}
impl From<super::calls::CallResolutionSupport> for SupportValue {
    fn from(row: super::calls::CallResolutionSupport) -> Self {
        Self::CallResolutionSupport(row)
    }
}
impl From<super::declarations::SymbolDeclarationSupport> for SupportValue {
    fn from(row: super::declarations::SymbolDeclarationSupport) -> Self {
        Self::SymbolDeclarationSupport(row)
    }
}
impl From<super::declarations::ParameterDeclarationSupport> for SupportValue {
    fn from(row: super::declarations::ParameterDeclarationSupport) -> Self {
        Self::ParameterDeclarationSupport(row)
    }
}
impl From<super::deployment::TaskReportSupport> for SupportValue {
    fn from(row: super::deployment::TaskReportSupport) -> Self {
        Self::TaskReportObservationSupport(row)
    }
}
impl From<super::deployment::DeploymentSupport> for SupportValue {
    fn from(row: super::deployment::DeploymentSupport) -> Self {
        Self::DeploymentObservationSupport(row)
    }
}
impl From<super::documents::DocumentSupport> for SupportValue {
    fn from(row: super::documents::DocumentSupport) -> Self {
        Self::DocumentObservationSupport(row)
    }
}
impl From<super::documents::PassageSupport> for SupportValue {
    fn from(row: super::documents::PassageSupport) -> Self {
        Self::PassageObservationSupport(row)
    }
}
impl From<super::documents::CodeBlockSupport> for SupportValue {
    fn from(row: super::documents::CodeBlockSupport) -> Self {
        Self::CodeBlockObservationSupport(row)
    }
}
impl From<super::documents::DocumentLinkSupport> for SupportValue {
    fn from(row: super::documents::DocumentLinkSupport) -> Self {
        Self::DocumentLinkObservationSupport(row)
    }
}
impl From<super::documents::DocumentMentionSupport> for SupportValue {
    fn from(row: super::documents::DocumentMentionSupport) -> Self {
        Self::DocumentMentionObservationSupport(row)
    }
}
impl From<super::documents::DocumentComponentSupport> for SupportValue {
    fn from(row: super::documents::DocumentComponentSupport) -> Self {
        Self::DocumentComponentObservationSupport(row)
    }
}
impl From<super::documents::DocumentAttributeSupport> for SupportValue {
    fn from(row: super::documents::DocumentAttributeSupport) -> Self {
        Self::DocumentAttributeObservationSupport(row)
    }
}
impl From<super::flow::FlowAttributeLoadSupport> for SupportValue {
    fn from(row: super::flow::FlowAttributeLoadSupport) -> Self {
        Self::FlowAttributeLoadObservationSupport(row)
    }
}
impl From<super::flow::FlowValuePathSupport> for SupportValue {
    fn from(row: super::flow::FlowValuePathSupport) -> Self {
        Self::FlowValuePathObservationSupport(row)
    }
}
impl From<super::lexical::LexicalScopeSupport> for SupportValue {
    fn from(row: super::lexical::LexicalScopeSupport) -> Self {
        Self::LexicalScopeObservationSupport(row)
    }
}
impl From<super::lexical::BindingSupport> for SupportValue {
    fn from(row: super::lexical::BindingSupport) -> Self {
        Self::BindingObservationSupport(row)
    }
}
impl From<super::lexical::ReferenceSupport> for SupportValue {
    fn from(row: super::lexical::ReferenceSupport) -> Self {
        Self::ReferenceObservationSupport(row)
    }
}
impl From<super::lexical::LexicalResolutionSupport> for SupportValue {
    fn from(row: super::lexical::LexicalResolutionSupport) -> Self {
        Self::LexicalResolutionSupport(row)
    }
}
impl From<super::source::SyntaxSupport> for SupportValue {
    fn from(row: super::source::SyntaxSupport) -> Self {
        Self::SyntaxObservationSupport(row)
    }
}
impl From<super::symbols::SymbolSupport> for SupportValue {
    fn from(row: super::symbols::SymbolSupport) -> Self {
        Self::SymbolObservationSupport(row)
    }
}
impl From<super::symbols::FunctionTraitSupport> for SupportValue {
    fn from(row: super::symbols::FunctionTraitSupport) -> Self {
        Self::FunctionTraitObservationSupport(row)
    }
}
impl From<super::symbols::ClassTraitSupport> for SupportValue {
    fn from(row: super::symbols::ClassTraitSupport) -> Self {
        Self::ClassTraitObservationSupport(row)
    }
}
impl From<super::symbols::ClassAncestrySupport> for SupportValue {
    fn from(row: super::symbols::ClassAncestrySupport) -> Self {
        Self::ClassAncestryObservationSupport(row)
    }
}
impl From<super::symbols::ParameterAnnotationSupport> for SupportValue {
    fn from(row: super::symbols::ParameterAnnotationSupport) -> Self {
        Self::ParameterAnnotationObservationSupport(row)
    }
}
impl From<super::symbols::PublicNameSupport> for SupportValue {
    fn from(row: super::symbols::PublicNameSupport) -> Self {
        Self::PublicNameObservationSupport(row)
    }
}
impl From<super::symbols::ParameterDocSupport> for SupportValue {
    fn from(row: super::symbols::ParameterDocSupport) -> Self {
        Self::ParameterDocObservationSupport(row)
    }
}
impl From<super::syntax::SyntaxPlacementSupport> for SupportValue {
    fn from(row: super::syntax::SyntaxPlacementSupport) -> Self {
        Self::SyntaxPlacementSupport(row)
    }
}
impl From<super::syntax::SyntaxDetailSupport> for SupportValue {
    fn from(row: super::syntax::SyntaxDetailSupport) -> Self {
        Self::SyntaxDetailObservationSupport(row)
    }
}
impl From<super::syntax::DeclarationSupport> for SupportValue {
    fn from(row: super::syntax::DeclarationSupport) -> Self {
        Self::DeclarationObservationSupport(row)
    }
}
impl From<super::syntax::DeclarationDecoratorSupport> for SupportValue {
    fn from(row: super::syntax::DeclarationDecoratorSupport) -> Self {
        Self::DeclarationDecoratorSupport(row)
    }
}
impl From<super::syntax::ImportAliasSupport> for SupportValue {
    fn from(row: super::syntax::ImportAliasSupport) -> Self {
        Self::ImportAliasObservationSupport(row)
    }
}
impl From<super::syntax::DunderAllSupport> for SupportValue {
    fn from(row: super::syntax::DunderAllSupport) -> Self {
        Self::DunderAllObservationSupport(row)
    }
}
impl From<super::syntax::ParameterSyntaxSupport> for SupportValue {
    fn from(row: super::syntax::ParameterSyntaxSupport) -> Self {
        Self::ParameterSyntaxObservationSupport(row)
    }
}
impl From<super::syntax::ClassFieldSyntaxSupport> for SupportValue {
    fn from(row: super::syntax::ClassFieldSyntaxSupport) -> Self {
        Self::ClassFieldSyntaxObservationSupport(row)
    }
}
impl From<super::types::TypeSupport> for SupportValue {
    fn from(row: super::types::TypeSupport) -> Self {
        Self::TypeObservationSupport(row)
    }
}
impl From<super::types::TypePresentationSupport> for SupportValue {
    fn from(row: super::types::TypePresentationSupport) -> Self {
        Self::TypePresentationSupport(row)
    }
}
impl From<super::types::TypeRestrictionSupport> for SupportValue {
    fn from(row: super::types::TypeRestrictionSupport) -> Self {
        Self::TypeVariableRestrictionSupport(row)
    }
}
impl From<super::types::FunctionBodySupport> for SupportValue {
    fn from(row: super::types::FunctionBodySupport) -> Self {
        Self::FunctionBodyObservationSupport(row)
    }
}
impl From<super::types::RecordFieldSupport> for SupportValue {
    fn from(row: super::types::RecordFieldSupport) -> Self {
        Self::RecordFieldObservationSupport(row)
    }
}
impl From<super::class_metadata::ClassMemberSupport> for SupportValue {
    fn from(row: super::class_metadata::ClassMemberSupport) -> Self {
        Self::ClassMemberObservationSupport(row)
    }
}
impl From<super::class_metadata::ClassMetadataSupport> for SupportValue {
    fn from(row: super::class_metadata::ClassMetadataSupport) -> Self {
        Self::ClassMetadataObservationSupport(row)
    }
}
impl From<super::calls::SignatureEnumerationSupport> for SupportValue {
    fn from(row: super::calls::SignatureEnumerationSupport) -> Self {
        Self::SignatureEnumerationObservationSupport(row)
    }
}
impl From<super::ruff::RuffContextSupport> for SupportValue {
    fn from(row: super::ruff::RuffContextSupport) -> Self {
        Self::RuffContextObservationSupport(row)
    }
}
impl From<super::flow::FlowNarrowingSupport> for SupportValue {
    fn from(row: super::flow::FlowNarrowingSupport) -> Self {
        Self::FlowNarrowingSupport(row)
    }
}
impl From<super::protocols::NativeExitSupport> for SupportValue {
    fn from(row: super::protocols::NativeExitSupport) -> Self {
        Self::NativeExitSupport(row)
    }
}
impl From<super::protocols::NativeTerminalSupport> for SupportValue {
    fn from(row: super::protocols::NativeTerminalSupport) -> Self {
        Self::NativeTerminalSupport(row)
    }
}
impl From<super::captures::CaptureSupport> for SupportValue {
    fn from(row: super::captures::CaptureSupport) -> Self {
        Self::CaptureSupport(row)
    }
}
impl From<super::protocols::NativeExitDiagnosticSupport> for SupportValue {
    fn from(row: super::protocols::NativeExitDiagnosticSupport) -> Self {
        Self::NativeExitDiagnosticSupport(row)
    }
}
impl From<super::flow::FlowSourceViewSupport> for SupportValue {
    fn from(row: super::flow::FlowSourceViewSupport) -> Self {
        Self::FlowSourceViewSupport(row)
    }
}
impl From<super::types::NativeSignatureSupport> for SupportValue {
    fn from(row: super::types::NativeSignatureSupport) -> Self {
        Self::NativeSignatureObservationSupport(row)
    }
}
impl From<super::types::SignatureTypeSupport> for SupportValue {
    fn from(row: super::types::SignatureTypeSupport) -> Self {
        Self::SignatureTypeObservationSupport(row)
    }
}
impl From<super::types::TypeQuerySupport> for SupportValue {
    fn from(row: super::types::TypeQuerySupport) -> Self {
        Self::TypeQuerySupport(row)
    }
}
impl From<super::types::GenericSpecializationSupport> for SupportValue {
    fn from(row: super::types::GenericSpecializationSupport) -> Self {
        Self::GenericSpecializationSupport(row)
    }
}
impl From<super::flow_capture::FlowCaptureTimingSupport> for SupportValue {
    fn from(row: super::flow_capture::FlowCaptureTimingSupport) -> Self {
        Self::FlowCaptureTimingSupport(row)
    }
}
impl From<super::symbols::ExportEnumerationSupport> for SupportValue {
    fn from(row: super::symbols::ExportEnumerationSupport) -> Self {
        Self::ExportEnumerationSupport(row)
    }
}
impl From<super::ruff::RuffBindingSupport> for SupportValue {
    fn from(row: super::ruff::RuffBindingSupport) -> Self {
        Self::RuffBindingObservationSupport(row)
    }
}
impl From<super::diagnostics::RuffDiagnosticSupport> for SupportValue {
    fn from(row: super::diagnostics::RuffDiagnosticSupport) -> Self {
        Self::RuffDiagnosticObservationSupport(row)
    }
}
impl From<super::diagnostics::PyreflyDiagnosticSupport> for SupportValue {
    fn from(row: super::diagnostics::PyreflyDiagnosticSupport) -> Self {
        Self::PyreflyDiagnosticObservationSupport(row)
    }
}
impl From<super::diagnostics::NativeParameterDefinitionSupport> for SupportValue {
    fn from(row: super::diagnostics::NativeParameterDefinitionSupport) -> Self {
        Self::NativeParameterDefinitionObservationSupport(row)
    }
}
impl From<super::ruff::RuffDefinitionSupport> for SupportValue {
    fn from(row: super::ruff::RuffDefinitionSupport) -> Self {
        Self::RuffDefinitionObservationSupport(row)
    }
}
impl From<super::flow_inventory::FlowUseInventorySupport> for SupportValue {
    fn from(row: super::flow_inventory::FlowUseInventorySupport) -> Self {
        Self::FlowUseInventorySupport(row)
    }
}
impl From<super::types::NativeOverloadSupport> for SupportValue {
    fn from(row: super::types::NativeOverloadSupport) -> Self {
        Self::NativeOverloadSupport(row)
    }
}
impl From<super::types::NativeOverloadCandidateSupport> for SupportValue {
    fn from(row: super::types::NativeOverloadCandidateSupport) -> Self {
        Self::NativeOverloadCandidateSupport(row)
    }
}
impl From<super::symbols::ModuleResolutionSupport> for SupportValue {
    fn from(row: super::symbols::ModuleResolutionSupport) -> Self {
        Self::ModuleResolutionObservationSupport(row)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! graph_entity_inventory_adapter {($apply:ident;$($variant:ident:$kind:ident=>$ty:ty,)*)=>{$apply!{$($variant:$ty,)*}};}
#[macro_export]
macro_rules! graph_entity_records {
    ($apply:ident) => {
        $crate::graph_entity_declarations! {$crate::graph_entity_inventory_adapter, $apply}
    };
}

impl GraphAssertionRecord for super::flow::FlowUseObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowDefinitionObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowReachingObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowValueObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowRegionObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowTestObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowTestLeafObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::Signature {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::CallTarget {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::ProviderCallSite {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::CallSyntax {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::CallResolution {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::declarations::SymbolDeclaration {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::declarations::ParameterDeclaration {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::deployment::TaskReportObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::deployment::DeploymentObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::PassageObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::CodeBlockObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentLinkObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentMentionObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentComponentObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentAttributeObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowAttributeLoadObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowValuePathObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::LexicalScopeObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::BindingObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::ReferenceObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::LexicalResolution {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::source::SyntaxObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::SymbolObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::FunctionTraitObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ClassTraitObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ClassAncestryObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ParameterAnnotationObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::PublicNameObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ParameterDocObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::SyntaxPlacement {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::SyntaxDetailObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::DeclarationObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::DeclarationDecorator {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::ImportAliasObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::DunderAllObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::ParameterSyntaxObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::ClassFieldSyntaxObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypeObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypePresentation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypeVariableRestriction {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::FunctionBodyObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::RecordFieldObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::class_metadata::ClassMemberObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::class_metadata::ClassMetadataObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::SignatureEnumerationObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::ruff::RuffContextObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowNarrowingObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::protocols::NativeExitObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::protocols::NativeTerminalObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::captures::CaptureObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::protocols::NativeExitDiagnostic {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowSourceViewObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::NativeSignatureObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::SignatureTypeObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypeQueryObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::GenericSpecializationObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow_capture::FlowCaptureTimingObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ExportEnumerationObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::ruff::RuffBindingObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::diagnostics::RuffDiagnosticObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::diagnostics::PyreflyDiagnosticObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::diagnostics::NativeParameterDefinitionObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::ruff::RuffDefinitionObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow_inventory::FlowUseInventoryObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::NativeOverloadObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::NativeOverloadCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ModuleResolutionObservation {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::NativeObservation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Native(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::local_semantics::LocalAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::local_semantics::LocalContribution {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::local_semantics::LocalGuardContribution {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::local_theory::TheoryWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::execution::records::ExpressionEvaluation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::execution::source_call_records::SourceCallHeader {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::execution::source_call_records::SourceInvocation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::execution::model_transfer::ModelTransferWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::execution::summary_consequences::SummaryClaim {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::execution::summary_path::SummaryPathWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::execution::summary_production::SummaryRun {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::normalized::entities::SymbolEntityResolution {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::normalized::entities::PublicEnumerationAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::normalized::bindings::CallBindingAttempt {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::normalized::bindings::CallBinding {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::normalized::bindings::BindingVariantAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::CatalogInvocation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::CatalogPath {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::CatalogAlias {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::CatalogConstructor {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::evidence::ScenarioAssociation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::evidence::DocumentAssociation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::evidence::CatalogDeployment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::catalog::evidence::ScenarioCheck {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::selection::SelectionDomain {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::structural::Conclusion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::structural::UsageScore {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::structural::controls::ControlTraversal {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::analytics::Conclusion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::analytics::TechniqueResult {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::analytics::RankScore {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::analytics::Community {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::analytics::Neighbour {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::analytics::Concept {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::analytics::Implication {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::synthesis::documentary::DocumentaryConclusion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::synthesis::assertions::ProgrammaticAssertion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::synthesis::briefs::Brief {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::retrieval::OriginalAnchor {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::retrieval::UnitSubject {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowUseSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowDefinitionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowReachingSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowValueSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowRegionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowTestSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowTestLeafSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::SignatureSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::CallTargetSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::ProviderCallSiteSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::CallSyntaxSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::CallResolutionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::declarations::SymbolDeclarationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::declarations::ParameterDeclarationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::deployment::TaskReportSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::deployment::DeploymentSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::PassageSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::CodeBlockSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentLinkSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentMentionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentComponentSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::documents::DocumentAttributeSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowAttributeLoadSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowValuePathSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::LexicalScopeSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::BindingSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::ReferenceSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::lexical::LexicalResolutionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::source::SyntaxSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::SymbolSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::FunctionTraitSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ClassTraitSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ClassAncestrySupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ParameterAnnotationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::PublicNameSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ParameterDocSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::SyntaxPlacementSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::SyntaxDetailSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::DeclarationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::DeclarationDecoratorSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::ImportAliasSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::DunderAllSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::ParameterSyntaxSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::syntax::ClassFieldSyntaxSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypeSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypePresentationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypeRestrictionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::FunctionBodySupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::RecordFieldSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::class_metadata::ClassMemberSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::class_metadata::ClassMetadataSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::calls::SignatureEnumerationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::ruff::RuffContextSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowNarrowingSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::protocols::NativeExitSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::protocols::NativeTerminalSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::captures::CaptureSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::protocols::NativeExitDiagnosticSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow::FlowSourceViewSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::NativeSignatureSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::SignatureTypeSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::TypeQuerySupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::GenericSpecializationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow_capture::FlowCaptureTimingSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ExportEnumerationSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::ruff::RuffBindingSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::diagnostics::RuffDiagnosticSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::diagnostics::PyreflyDiagnosticSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::diagnostics::NativeParameterDefinitionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::ruff::RuffDefinitionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::flow_inventory::FlowUseInventorySupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::NativeOverloadSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::types::NativeOverloadCandidateSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}
impl GraphAssertionRecord for super::symbols::ModuleResolutionSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        let qualification = row
            .references()
            .into_iter()
            .find(|reference| reference.target == super::assertion::AssertionQualification::NAME)
            .map(|reference| {
                Qualification::Ref(entity_key(
                    EntityKind::Qualification,
                    reference.target,
                    &reference.key,
                ))
            })
            .unwrap_or(Qualification::Payload);
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification,
            run: None,
            evidence: vec![],
            value: AssertionValue::Support(row.into()),
            derivation: None,
        }
    }
}

#[macro_export]
// Selected analytics retain their computation universe, weights, memberships, availability and
// provenance. QualityStep iteration traces have no semantic/serving consumer and remain private.
macro_rules! graph_assertion_records{($apply:ident)=>{$apply! {
    RetainedReleaseDeployment:$crate::domain::catalog::evidence::ReleaseDeployment,
    RetainedScenarioSpan:$crate::domain::catalog::evidence::ScenarioSpan,
    RetainedGuardSubstitution:$crate::domain::conditions::stability::GuardSubstitution,
    NativeDiagnosticAnnotation:$crate::domain::diagnostics::DiagnosticAnnotation,
    NativeDiagnosticSubject:$crate::domain::diagnostics::DiagnosticSubject,
    RetainedSummaryCaptureContribution:$crate::domain::execution::summary_capture::SummaryCaptureContribution,
    NativeFlowUseCandidate:$crate::domain::flow_inventory::FlowUseCandidate,
    NativeFlowUseInventoryMember:$crate::domain::flow_inventory::FlowUseInventoryMember,
    RetainedEffectiveCallableEvidence:$crate::domain::normalized::callables::EffectiveCallableEvidence,
    RetainedEffectiveCallablePremise:$crate::domain::normalized::callables::EffectiveCallablePremise,
    RetainedEffectiveDecoratorMember:$crate::domain::normalized::callables::EffectiveDecoratorMember,
    RetainedSymbolEntityEvidence:$crate::domain::normalized::entities::SymbolEntityEvidence,
    RetainedSymbolEntityPremise:$crate::domain::normalized::entities::SymbolEntityPremise,
    RetainedCallAlternativeEvidence:$crate::domain::normalized::events::CallAlternativeEvidence,
    RetainedCallEventResolution:$crate::domain::normalized::events::CallEventResolution,
    RetainedCallEventResolutionEvidence:$crate::domain::normalized::events::CallEventResolutionEvidence,
    RetainedEventPhaseTarget:$crate::domain::normalized::events::EventPhaseTarget,
    RetainedFlowCallEventLink:$crate::domain::normalized::events::FlowCallEventLink,
    RetainedDeclarationNativeCharacterization:$crate::domain::normalized::links::DeclarationNativeCharacterization,
    RetainedImportModuleAssessment:$crate::domain::normalized::links::ImportModuleAssessment,
    RetainedImportModuleCandidate:$crate::domain::normalized::links::ImportModuleCandidate,
    RetainedMentionSymbolCandidate:$crate::domain::normalized::links::MentionSymbolCandidate,
    RetainedPlaceEntityLink:$crate::domain::normalized::links::PlaceEntityLink,
    RetainedTestOperandCoverage:$crate::domain::normalized::links::TestOperandCoverage,
    RetainedTypeBinderAssessment:$crate::domain::normalized::links::TypeBinderAssessment,
    RetainedTypeBinderCandidate:$crate::domain::normalized::links::TypeBinderCandidate,
    RetainedTypeBinderPremise:$crate::domain::normalized::links::TypeBinderPremise,
    RetainedTypeEntityLink:$crate::domain::normalized::links::TypeEntityLink,
    RetainedOverloadVariantAssessment:$crate::domain::normalized::overload_association::OverloadVariantAssessment,
    RetainedOverloadVariantCandidate:$crate::domain::normalized::overload_association::OverloadVariantCandidate,
    RetainedReceiverEvidence:$crate::domain::normalized::receiver::ReceiverEvidence,
    RetainedReceiverPremise:$crate::domain::normalized::receiver::ReceiverPremise,
    RetainedAssertionSource:$crate::domain::synthesis::assertions::AssertionSource,
    RetainedProgrammaticAssertionSupport:$crate::domain::synthesis::assertions::ProgrammaticAssertionSupport,
    RetainedBriefAssertion:$crate::domain::synthesis::briefs::BriefAssertion,
    RetainedBriefDocument:$crate::domain::synthesis::briefs::BriefDocument,
    RetainedBriefSource:$crate::domain::synthesis::briefs::BriefSource,

    SelectionReferenceBindingCharacterization:$crate::domain::normalized::links::ReferenceBindingCharacterization,
    SelectionSourceFieldLink:$crate::domain::catalog::evidence::SourceFieldLink,
    SignatureSlotType:$crate::domain::normalized::callables::SignatureSlotType,
    SignatureReturnType:$crate::domain::normalized::callables::SignatureReturnType,
    SelectionFieldLocationLink:$crate::domain::catalog::evidence::FieldLocationLink,
    SelectionConstructorCandidateLink:$crate::domain::catalog::evidence::ConstructorCandidateLink,
    SelectionFieldAccessAssessment:$crate::domain::catalog::evidence::FieldAccessAssessment,
    SelectionFieldLocation:$crate::domain::local_fields::FieldLocation,
    SelectionFieldLocationCandidate:$crate::domain::local_fields::FieldLocationCandidate,
    SelectionContext:$crate::domain::selection::Context,
    SelectionWitness:$crate::domain::selection::Witness,
    SelectionDomainContext:$crate::domain::selection::DomainContext,
    SelectionDomainClosure:$crate::domain::selection::DomainClosure,
    SelectionDomainEvidence:$crate::domain::selection::DomainEvidence,
    NormalizedCallEventSource:$crate::domain::normalized::events::CallEventSource,
    NormalizedCallEventSourceEvidence:$crate::domain::normalized::events::CallEventSourceEvidence,
    CatalogSourceCharacterization:$crate::domain::catalog::evidence::SourceCharacterization,
    CatalogSourceCharacterizationScenario:$crate::domain::catalog::evidence::SourceCharacterizationScenario,
    CatalogSourceUsage:$crate::domain::catalog::evidence::SourceUsage,
    CatalogDiagnosticUseAssessment:$crate::domain::catalog::evidence::DiagnosticUseAssessment,
    CatalogDiagnosticUseLink:$crate::domain::catalog::evidence::DiagnosticUseLink,
    CatalogDiagnosticUsePath:$crate::domain::catalog::evidence::DiagnosticUsePath,
    CatalogDiagnosticUseTarget:$crate::domain::catalog::evidence::DiagnosticUseTarget,
    RetrievalEmbeddingUse:$crate::domain::retrieval::consumption::RetrievalEmbeddingUse,
    RetrievalUnitRoot:$crate::domain::retrieval::UnitRoot,
    StructuralUsageSite:$crate::domain::structural::UsageSite,
    StructuralUsageEvidence:$crate::domain::structural::UsageEvidence,
    CallPolicyAssessment:$crate::domain::normalized::events::CallPolicyAssessment,
    CallPolicyAdmission:$crate::domain::normalized::events::CallPolicyAdmission,

    AnalyticUniverseMember:$crate::domain::analytics::UniverseMember,
    AnalyticPublicSelector:$crate::domain::analytics::PublicSelector,
    AnalyticPartitionMember:$crate::domain::analytics::PartitionMember,
    AnalyticCommunityMember:$crate::domain::analytics::CommunityMember,
    AnalyticConceptObject:$crate::domain::analytics::ConceptObject,
    AnalyticConceptExtent:$crate::domain::analytics::ConceptExtent,
    AnalyticConceptIntent:$crate::domain::analytics::ConceptIntent,
    AnalyticImplicationMember:$crate::domain::analytics::ImplicationMember,
    AnalyticLayerPair:$crate::domain::analytics::LayerPair,
    AnalyticCombinedPair:$crate::domain::analytics::CombinedPair,
    AnalyticCommunityRun:$crate::domain::analytics::CommunityRun,
    AnalyticCommunityProfile:$crate::domain::analytics::CommunityProfile,
    AnalyticVectorSelection:$crate::domain::analytics::VectorSelection,
    AnalyticCommunityLabelAssessment:$crate::domain::analytics::CommunityLabelAssessment,
    AnalyticLayerResult:$crate::domain::analytics::LayerResult,
    AnalyticLayerNeighbour:$crate::domain::analytics::LayerNeighbour,
    AnalyticGraphArc:$crate::domain::analytics::GraphArc,
    AnalyticPairSource:$crate::domain::analytics::PairSource,
    AnalyticPairContribution:$crate::domain::analytics::PairContribution,
    AnalyticIncidenceSource:$crate::domain::analytics::IncidenceSource,
    AnalyticIncidence:$crate::domain::analytics::Incidence,
    AnalyticTypeMetadataSelection:$crate::domain::analytics::TypeMetadataSelection,
    AnalyticDecoratorSelection:$crate::domain::analytics::DecoratorSelection,
    LocalControlSupports:$crate::domain::transfer::local::ControlSupport,
    LocalTransferSelections:$crate::domain::transfer::local::Selection,

    CallResolutionMembers:$crate::domain::calls::CallResolutionMember,
    SignatureEnumerationMembers:$crate::domain::calls::SignatureEnumerationMember,
    AnalyticCommunityLabelMembers:$crate::domain::analytics::CommunityLabelMember,
    BindingSetMembers:$crate::domain::normalized::bindings::BindingSetMember,
    BindingSetCoverage:$crate::domain::normalized::bindings::BindingSetCoverage,
    NormalizedDispatchPremises:$crate::domain::normalized::dispatch::DispatchPremise,
    NormalizedDispatchEvidence:$crate::domain::normalized::dispatch::DispatchEvidence,
    SummaryControlInfluences:$crate::domain::transfer::summary::ControlInfluence,
    SummaryControlSupports:$crate::domain::transfer::summary::ControlSupport,
    SummaryTransferSelections:$crate::domain::transfer::summary::Selection,

    AssumptionUniverseSupports:$crate::domain::assumptions_universe::AssumptionUniverseSupport,
    AnalyticTextAssessments:$crate::domain::embedding::text::TextAssessment,
    SynthesisConfiguredSeedDecisions:$crate::domain::synthesis::seeds::ConfiguredSeedDecision,
    SynthesisAutomaticSeedDecisions:$crate::domain::synthesis::automatic::Decision,
    StructuralReaches:$crate::domain::structural::Reach,
    StructuralUnfollowedArguments:$crate::domain::structural::controls::UnfollowedArgument,
    StructuralHandoffValues:$crate::domain::structural::handoffs::ValueSource,
    StructuralHandoffOccurrences:$crate::domain::structural::handoffs::Handoff,
    SummaryBehavioralConclusions:$crate::domain::execution::summary_consequences::ClaimConclusion,
    SourceFieldClassAssessments:$crate::domain::normalized::symbolic_fields::SourceFieldClass,
    SourceFieldStores:$crate::domain::normalized::symbolic_fields::SourceFieldStore,
    SourceFieldReaders:$crate::domain::normalized::symbolic_fields::SourceFieldReader,
    SourceFieldAssociations:$crate::domain::normalized::symbolic_fields::SourceFieldAssociation,
    CallEventAssessments:$crate::domain::normalized::events::EventAssessment,

    LocalTypeDomains:$crate::domain::local_theory::TypeDomain,
    LocalTypeDomainAssessments:$crate::domain::local_theory::TypeDomainAssessment,
    LocalTypeDomainMembers:$crate::domain::local_theory::TypeDomainMember,
    ReportEntries:$crate::domain::deployment::ReportEntry,
    CallOriginSteps:$crate::domain::calls::CallOriginStep,
    LocalSymbolicFieldStores:$crate::domain::local_symbolic::SymbolicFieldStore,
    FlowCaptureCandidates:$crate::domain::flow_capture::FlowCaptureCandidate,
    LocalAtomDecisions:$crate::domain::atom_decision::AtomDecision,
    FlowCallSteps:$crate::domain::flow::FlowCallStep,
    ProjectionSourceAssessments:$crate::domain::projection::ProjectionSourceAssessment,
    AnalyticTextWindows:$crate::domain::embedding::text::TextWindow,
    SynthesisSeedPlans:$crate::domain::synthesis::seeds::SeedPlan,
    SynthesisSelectedSeedSources:$crate::domain::synthesis::seeds::SelectedSeedSource,
    SynthesisAuthoredCodeSources:$crate::domain::synthesis::patterns::AuthoredCodeSource,
    SynthesisProseSources:$crate::domain::synthesis::documentary::ProseSource,
    EntryAccessSources:$crate::domain::conditions::entry::EntryAccessSource,
    StructuralPublicCandidates:$crate::domain::structural::PublicCandidate,
    StructuralTraversals:$crate::domain::structural::Traversal,
    StructuralPaths:$crate::domain::structural::Path,
    StructuralUnresolvedEvents:$crate::domain::structural::UnresolvedEvent,
    StructuralLiteralArguments:$crate::domain::structural::controls::LiteralArgument,
    StructuralControlPaths:$crate::domain::structural::controls::ControlPath,
    StructuralConditionalRaises:$crate::domain::structural::controls::ConditionalRaise,
    StructuralUnfollowedPaths:$crate::domain::structural::controls::UnfollowedPath,
    StructuralHandoffGroups:$crate::domain::structural::handoffs::Group,
    ClosedTargetAssessments:$crate::domain::execution::closed_targets::ClosedTargetAssessment,
    ModelContextResources:$crate::domain::execution::model_protocol::ContextResource,
    CapturedEntryBindings:$crate::domain::execution::capture_bridge::CapturedEntryBinding,
    AnalyticDocumentNeighbours:$crate::domain::analytics::DocumentNeighbour,
    AnalyticCommunityLabels:$crate::domain::analytics::CommunityLabel,
    SourceFieldReaderLinks:$crate::domain::normalized::symbolic_fields::SourceFieldReaderLink,
    ClassFieldDefaultAssessments:$crate::domain::normalized::callable_aspects::FieldDefaultAssessment,
    ReferenceEntityTargets:$crate::domain::normalized::links::ReferenceEntityTarget,
    ReferenceEntityAssessments:$crate::domain::normalized::links::ReferenceEntityAssessment,
    AncestryEntityAssessments:$crate::domain::normalized::links::AncestryEntityAssessment,
    MentionEntityAssessments:$crate::domain::normalized::links::MentionEntityAssessment,
    TestOperandTypeAssessments:$crate::domain::normalized::links::TestOperandTypeAssessment,
    SymbolEntityCandidates:$crate::domain::normalized::entities::SymbolEntityCandidate,
    ParameterEntityLinks:$crate::domain::normalized::entities::ParameterEntityLink,
    FieldEntityLinks:$crate::domain::normalized::entities::FieldEntityLink,
    FieldDeclarationLinks:$crate::domain::normalized::entities::FieldDeclarationLink,
    PublicExposureCandidates:$crate::domain::normalized::entities::PublicExposureCandidate,
    NormalizedCallAlternativeSources:$crate::domain::normalized::events::CallAlternativeSource,
    SignatureSlotEntities:$crate::domain::normalized::callables::SignatureSlotEntity,
    NormalizedDispatchAssessments:$crate::domain::normalized::dispatch::DispatchAssessment,
    LocalTransferAlternatives:$crate::domain::transfer::local::TransferAlternative,
    LocalTransferSupports:$crate::domain::transfer::local::TransferSupport,
    ModelTransferAlternatives:$crate::domain::transfer::model::TransferAlternative,
    ModelTransferSupports:$crate::domain::transfer::model::TransferSupport,
    SummaryTransferAlternatives:$crate::domain::transfer::summary::TransferAlternative,
    SummaryTransferSupports:$crate::domain::transfer::summary::TransferSupport,

    LocalBuiltinOperandWitnesses:$crate::domain::local_theory::BuiltinOperandWitness,
    BindingSources:$crate::domain::calls::BindingSource,
    BindingProjections:$crate::domain::calls::BindingProjection,
    LocalAtomRestrictions:$crate::domain::atom_decision::AtomRestriction,
    AnalysisEmbeddingUses:$crate::domain::embedding::analytic::AnalysisEmbeddingUse,
    CatalogMemberInvocations:$crate::domain::catalog::CatalogMemberInvocation,
    SynthesisSelectedSeeds:$crate::domain::synthesis::seeds::SelectedSeed,
    SynthesisAuthoredCodeConclusions:$crate::domain::synthesis::patterns::AuthoredCodeConclusion,
    SynthesisProseSlices:$crate::domain::synthesis::documentary::ProseSlice,
    SynthesisDocumentarySources:$crate::domain::synthesis::documentary::DocumentarySource,
    EntryValueWitnesses:$crate::domain::conditions::entry::EntryValueWitness,
    StabilityWitnesses:$crate::domain::conditions::stability::StabilityWitness,
    StructuralConclusionSources:$crate::domain::structural::ConclusionSource,
    SummaryTerminalWitnesses:$crate::domain::execution::summary_terminal::SummaryTerminalWitness,
    BaseStatementCompletions:$crate::domain::execution::completion_records::StatementCompletion,
    ModeledCallEvaluations:$crate::domain::execution::modeled_call::ModeledCallEvaluation,
    SummaryPathRoutes:$crate::domain::execution::summary_path::SummaryPathRoute,
    ModelApplications:$crate::domain::execution::model_production::ModelApplication,
    SummaryCaptureWitnesses:$crate::domain::execution::summary_capture::SummaryCaptureWitness,
    BaseSourceBodyCompletions:$crate::domain::execution::body_records::SourceBodyCompletion,
    ConditionalTerminalFrontiers:$crate::domain::execution::protocol_interpretation::ConditionalTerminalFrontier,
    NormalContinuationRestrictions:$crate::domain::execution::protocol_interpretation::NormalContinuationRestriction,
    SummaryClaimProofs:$crate::domain::execution::summary_consequences::ClaimProof,
    SummaryControlWitnesses:$crate::domain::execution::summary_control::SummaryControlWitness,
    DefinitionEvaluations:$crate::domain::execution::definition::DefinitionEvaluation,
    ModelAppliedRules:$crate::domain::execution::model_rules::AppliedRule,
    ContextExecutions:$crate::domain::execution::context_execution::ContextExecution,
    StatementExecutions:$crate::domain::execution::enriched_records::StatementExecution,
    BodyExecutions:$crate::domain::execution::enriched_records::BodyExecution,
    SourceExecutionInvocations:$crate::domain::execution::enriched_records::SourceExecutionInvocation,
    ContextEntryBindings:$crate::domain::execution::context_binding::ContextEntryBinding,
    SummarySymbolicFieldAlternatives:$crate::domain::execution::summary_symbolic::SymbolicFieldAlternative,
    ModelContextTransferWitnesses:$crate::domain::execution::model_context_transfer::ContextTransferWitness,
    SourceCallOutcomes:$crate::domain::execution::source_call_records::SourceCallOutcome,
    SourceCallFrameReleases:$crate::domain::execution::source_call_records::SourceFrameRelease,
    SummaryTransferPremises:$crate::domain::transfer::summary::SummaryPremise,
    SummaryTransferWitnesses:$crate::domain::transfer::summary::SummaryWitness,
    AnalyticConclusionSources:$crate::domain::analytics::ConclusionSource,
    NormalizationCoverage:$crate::domain::normalized::coverage::NormalizationCoverage,
    ReferenceEntityCandidates:$crate::domain::normalized::links::ReferenceEntityCandidate,
    AncestryEntityMembers:$crate::domain::normalized::links::AncestryEntityMember,
    MentionEntityCandidates:$crate::domain::normalized::links::MentionEntityCandidate,
    TestOperandTypeLinks:$crate::domain::normalized::links::TestOperandTypeLink,
    OccurrenceOwnership:$crate::domain::normalized::entities::OccurrenceOwnership,
    NormalizedCallEvents:$crate::domain::normalized::events::NormalizedCallEvent,
    NormalizedCallAlternatives:$crate::domain::normalized::events::NormalizedCallAlternative,
    EffectiveCallableAssessments:$crate::domain::normalized::callables::EffectiveCallableAssessment,
    BindingSetAssessments:$crate::domain::normalized::bindings::BindingSetAssessment,
    ReceiverAssessments:$crate::domain::normalized::receiver::ReceiverAssessment,
    NormalizedDispatchMembers:$crate::domain::normalized::dispatch::DispatchMember,
    LocalControlInfluences:$crate::domain::transfer::local::ControlInfluence,

    NativeQualification:$crate::domain::analysis::native::NativeQualification,
    LocalAnalysisObligation:$crate::domain::analysis::local::AnalysisObligation,
    LocalCoverageRequirement:$crate::domain::analysis::local::CoverageRequirement,
    LocalCoverageRequiredSource:$crate::domain::analysis::local::CoverageRequiredSource,
    BaseEvaluationAnalysisObligation:$crate::domain::analysis::base_evaluation::AnalysisObligation,
    BaseEvaluationCoverageRequirement:$crate::domain::analysis::base_evaluation::CoverageRequirement,
    BaseEvaluationCoverageRequiredSource:$crate::domain::analysis::base_evaluation::CoverageRequiredSource,
    BaseCompletionAnalysisObligation:$crate::domain::analysis::base_completion::AnalysisObligation,
    BaseCompletionCoverageRequirement:$crate::domain::analysis::base_completion::CoverageRequirement,
    BaseCompletionCoverageRequiredSource:$crate::domain::analysis::base_completion::CoverageRequiredSource,
    SourceCallAnalysisObligation:$crate::domain::analysis::source_call::AnalysisObligation,
    SourceCallCoverageRequirement:$crate::domain::analysis::source_call::CoverageRequirement,
    SourceCallCoverageRequiredSource:$crate::domain::analysis::source_call::CoverageRequiredSource,
    EnrichedExecutionAnalysisObligation:$crate::domain::analysis::enriched_execution::AnalysisObligation,
    EnrichedExecutionCoverageRequirement:$crate::domain::analysis::enriched_execution::CoverageRequirement,
    EnrichedExecutionCoverageRequiredSource:$crate::domain::analysis::enriched_execution::CoverageRequiredSource,
    ModelAnalysisObligation:$crate::domain::analysis::model::AnalysisObligation,
    ModelCoverageRequirement:$crate::domain::analysis::model::CoverageRequirement,
    ModelCoverageRequiredSource:$crate::domain::analysis::model::CoverageRequiredSource,
    SummaryAnalysisObligation:$crate::domain::analysis::summary::AnalysisObligation,
    SummaryCoverageRequirement:$crate::domain::analysis::summary::CoverageRequirement,
    SummaryCoverageRequiredSource:$crate::domain::analysis::summary::CoverageRequiredSource,
    StructuralAnalysisObligation:$crate::domain::analysis::structural::AnalysisObligation,
    StructuralCoverageRequirement:$crate::domain::analysis::structural::CoverageRequirement,
    StructuralCoverageRequiredSource:$crate::domain::analysis::structural::CoverageRequiredSource,
    AnalyticEmbeddingAnalysisObligation:$crate::domain::analysis::analytic_embedding::AnalysisObligation,
    AnalyticEmbeddingCoverageRequirement:$crate::domain::analysis::analytic_embedding::CoverageRequirement,
    AnalyticEmbeddingCoverageRequiredSource:$crate::domain::analysis::analytic_embedding::CoverageRequiredSource,
    AnalyticAnalysisObligation:$crate::domain::analysis::analytic::AnalysisObligation,
    AnalyticCoverageRequirement:$crate::domain::analysis::analytic::CoverageRequirement,
    AnalyticCoverageRequiredSource:$crate::domain::analysis::analytic::CoverageRequiredSource,
    CatalogCoreAnalysisObligation:$crate::domain::analysis::catalog_core::AnalysisObligation,
    CatalogCoreCoverageRequirement:$crate::domain::analysis::catalog_core::CoverageRequirement,
    CatalogCoreCoverageRequiredSource:$crate::domain::analysis::catalog_core::CoverageRequiredSource,
    CatalogEvidenceAnalysisObligation:$crate::domain::analysis::catalog_evidence::AnalysisObligation,
    CatalogEvidenceCoverageRequirement:$crate::domain::analysis::catalog_evidence::CoverageRequirement,
    CatalogEvidenceCoverageRequiredSource:$crate::domain::analysis::catalog_evidence::CoverageRequiredSource,
    SelectionAnalysisObligation:$crate::domain::analysis::selection::AnalysisObligation,
    SelectionCoverageRequirement:$crate::domain::analysis::selection::CoverageRequirement,
    SelectionCoverageRequiredSource:$crate::domain::analysis::selection::CoverageRequiredSource,
    SynthesisAnalysisObligation:$crate::domain::analysis::synthesis::AnalysisObligation,
    SynthesisCoverageRequirement:$crate::domain::analysis::synthesis::CoverageRequirement,
    SynthesisCoverageRequiredSource:$crate::domain::analysis::synthesis::CoverageRequiredSource,
    RetrievalAnalysisObligation:$crate::domain::analysis::retrieval::AnalysisObligation,
    RetrievalCoverageRequirement:$crate::domain::analysis::retrieval::CoverageRequirement,
    RetrievalCoverageRequiredSource:$crate::domain::analysis::retrieval::CoverageRequiredSource,

    TypeSequenceMember:$crate::domain::types::TypeSequenceMember,
    SymbolSequenceMember:$crate::domain::symbols::SymbolSequenceMember,
    AssumptionMember:$crate::domain::assumptions::AssumptionSetMember,
    LiteralMember:$crate::domain::value::LiteralSetMember,
    Acquisition:$crate::domain::input::InputAcquisition,
    CorpusMembership:$crate::domain::input::CorpusLibrary,
    ArtifactUse:$crate::domain::input::ArtifactUse,
    ArtifactOwnership:$crate::domain::input::ArtifactOwnership,
    UnownedArtifact:$crate::domain::input::UnownedArtifact,
    DerivedArtifact:$crate::domain::input::DerivedArtifact,
    Distribution:$crate::domain::input::InputDistribution,
    DistributionVerification:$crate::domain::input::DistributionVerification,
    Environment:$crate::domain::input::EnvironmentFingerprint,
    Coverage:$crate::domain::attribution::ProviderCoverage,
    RunFamily:$crate::domain::attribution::RunFamily,
    LocalAnalysisInput:$crate::domain::analysis::local::AnalysisInput,
    LocalAnalysisOutcome:$crate::domain::analysis::local::AnalysisOutcome,
    LocalAnalysisCoverage:$crate::domain::analysis::local::AnalysisCoverage,
    LocalAnalysisCoveragePremise:$crate::domain::analysis::local::AnalysisCoveragePremise,
    LocalAnalysisProposition:$crate::domain::analysis::local::AnalysisProposition,
    LocalAnalysisDerivation:$crate::domain::analysis::local::AnalysisDerivation,
    LocalAnalysisDerivationPremise:$crate::domain::analysis::local::AnalysisDerivationPremise,
    BaseEvaluationAnalysisInput:$crate::domain::analysis::base_evaluation::AnalysisInput,
    BaseEvaluationAnalysisOutcome:$crate::domain::analysis::base_evaluation::AnalysisOutcome,
    BaseEvaluationAnalysisCoverage:$crate::domain::analysis::base_evaluation::AnalysisCoverage,
    BaseEvaluationAnalysisCoveragePremise:$crate::domain::analysis::base_evaluation::AnalysisCoveragePremise,
    BaseEvaluationAnalysisProposition:$crate::domain::analysis::base_evaluation::AnalysisProposition,
    BaseEvaluationAnalysisDerivation:$crate::domain::analysis::base_evaluation::AnalysisDerivation,
    BaseEvaluationAnalysisDerivationPremise:$crate::domain::analysis::base_evaluation::AnalysisDerivationPremise,
    BaseCompletionAnalysisInput:$crate::domain::analysis::base_completion::AnalysisInput,
    BaseCompletionAnalysisOutcome:$crate::domain::analysis::base_completion::AnalysisOutcome,
    BaseCompletionAnalysisCoverage:$crate::domain::analysis::base_completion::AnalysisCoverage,
    BaseCompletionAnalysisCoveragePremise:$crate::domain::analysis::base_completion::AnalysisCoveragePremise,
    BaseCompletionAnalysisProposition:$crate::domain::analysis::base_completion::AnalysisProposition,
    BaseCompletionAnalysisDerivation:$crate::domain::analysis::base_completion::AnalysisDerivation,
    BaseCompletionAnalysisDerivationPremise:$crate::domain::analysis::base_completion::AnalysisDerivationPremise,
    SourceCallAnalysisInput:$crate::domain::analysis::source_call::AnalysisInput,
    SourceCallAnalysisOutcome:$crate::domain::analysis::source_call::AnalysisOutcome,
    SourceCallAnalysisCoverage:$crate::domain::analysis::source_call::AnalysisCoverage,
    SourceCallAnalysisCoveragePremise:$crate::domain::analysis::source_call::AnalysisCoveragePremise,
    SourceCallAnalysisProposition:$crate::domain::analysis::source_call::AnalysisProposition,
    SourceCallAnalysisDerivation:$crate::domain::analysis::source_call::AnalysisDerivation,
    SourceCallAnalysisDerivationPremise:$crate::domain::analysis::source_call::AnalysisDerivationPremise,
    EnrichedExecutionAnalysisInput:$crate::domain::analysis::enriched_execution::AnalysisInput,
    EnrichedExecutionAnalysisOutcome:$crate::domain::analysis::enriched_execution::AnalysisOutcome,
    EnrichedExecutionAnalysisCoverage:$crate::domain::analysis::enriched_execution::AnalysisCoverage,
    EnrichedExecutionAnalysisCoveragePremise:$crate::domain::analysis::enriched_execution::AnalysisCoveragePremise,
    EnrichedExecutionAnalysisProposition:$crate::domain::analysis::enriched_execution::AnalysisProposition,
    EnrichedExecutionAnalysisDerivation:$crate::domain::analysis::enriched_execution::AnalysisDerivation,
    EnrichedExecutionAnalysisDerivationPremise:$crate::domain::analysis::enriched_execution::AnalysisDerivationPremise,
    ModelAnalysisInput:$crate::domain::analysis::model::AnalysisInput,
    ModelAnalysisOutcome:$crate::domain::analysis::model::AnalysisOutcome,
    ModelAnalysisCoverage:$crate::domain::analysis::model::AnalysisCoverage,
    ModelAnalysisCoveragePremise:$crate::domain::analysis::model::AnalysisCoveragePremise,
    ModelAnalysisProposition:$crate::domain::analysis::model::AnalysisProposition,
    ModelAnalysisDerivation:$crate::domain::analysis::model::AnalysisDerivation,
    ModelAnalysisDerivationPremise:$crate::domain::analysis::model::AnalysisDerivationPremise,
    SummaryAnalysisInput:$crate::domain::analysis::summary::AnalysisInput,
    SummaryAnalysisOutcome:$crate::domain::analysis::summary::AnalysisOutcome,
    SummaryAnalysisCoverage:$crate::domain::analysis::summary::AnalysisCoverage,
    SummaryAnalysisCoveragePremise:$crate::domain::analysis::summary::AnalysisCoveragePremise,
    SummaryAnalysisProposition:$crate::domain::analysis::summary::AnalysisProposition,
    SummaryAnalysisDerivation:$crate::domain::analysis::summary::AnalysisDerivation,
    SummaryAnalysisDerivationPremise:$crate::domain::analysis::summary::AnalysisDerivationPremise,
    StructuralAnalysisInput:$crate::domain::analysis::structural::AnalysisInput,
    StructuralAnalysisOutcome:$crate::domain::analysis::structural::AnalysisOutcome,
    StructuralAnalysisCoverage:$crate::domain::analysis::structural::AnalysisCoverage,
    StructuralAnalysisCoveragePremise:$crate::domain::analysis::structural::AnalysisCoveragePremise,
    StructuralAnalysisProposition:$crate::domain::analysis::structural::AnalysisProposition,
    StructuralAnalysisDerivation:$crate::domain::analysis::structural::AnalysisDerivation,
    StructuralAnalysisDerivationPremise:$crate::domain::analysis::structural::AnalysisDerivationPremise,
    AnalyticEmbeddingAnalysisInput:$crate::domain::analysis::analytic_embedding::AnalysisInput,
    AnalyticEmbeddingAnalysisOutcome:$crate::domain::analysis::analytic_embedding::AnalysisOutcome,
    AnalyticEmbeddingAnalysisCoverage:$crate::domain::analysis::analytic_embedding::AnalysisCoverage,
    AnalyticEmbeddingAnalysisCoveragePremise:$crate::domain::analysis::analytic_embedding::AnalysisCoveragePremise,
    AnalyticEmbeddingAnalysisProposition:$crate::domain::analysis::analytic_embedding::AnalysisProposition,
    AnalyticEmbeddingAnalysisDerivation:$crate::domain::analysis::analytic_embedding::AnalysisDerivation,
    AnalyticEmbeddingAnalysisDerivationPremise:$crate::domain::analysis::analytic_embedding::AnalysisDerivationPremise,
    AnalyticAnalysisInput:$crate::domain::analysis::analytic::AnalysisInput,
    AnalyticAnalysisOutcome:$crate::domain::analysis::analytic::AnalysisOutcome,
    AnalyticAnalysisCoverage:$crate::domain::analysis::analytic::AnalysisCoverage,
    AnalyticAnalysisCoveragePremise:$crate::domain::analysis::analytic::AnalysisCoveragePremise,
    AnalyticAnalysisProposition:$crate::domain::analysis::analytic::AnalysisProposition,
    AnalyticAnalysisDerivation:$crate::domain::analysis::analytic::AnalysisDerivation,
    AnalyticAnalysisDerivationPremise:$crate::domain::analysis::analytic::AnalysisDerivationPremise,
    CatalogCoreAnalysisInput:$crate::domain::analysis::catalog_core::AnalysisInput,
    CatalogCoreAnalysisOutcome:$crate::domain::analysis::catalog_core::AnalysisOutcome,
    CatalogCoreAnalysisCoverage:$crate::domain::analysis::catalog_core::AnalysisCoverage,
    CatalogCoreAnalysisCoveragePremise:$crate::domain::analysis::catalog_core::AnalysisCoveragePremise,
    CatalogCoreAnalysisProposition:$crate::domain::analysis::catalog_core::AnalysisProposition,
    CatalogCoreAnalysisDerivation:$crate::domain::analysis::catalog_core::AnalysisDerivation,
    CatalogCoreAnalysisDerivationPremise:$crate::domain::analysis::catalog_core::AnalysisDerivationPremise,
    CatalogEvidenceAnalysisInput:$crate::domain::analysis::catalog_evidence::AnalysisInput,
    CatalogEvidenceAnalysisOutcome:$crate::domain::analysis::catalog_evidence::AnalysisOutcome,
    CatalogEvidenceAnalysisCoverage:$crate::domain::analysis::catalog_evidence::AnalysisCoverage,
    CatalogEvidenceAnalysisCoveragePremise:$crate::domain::analysis::catalog_evidence::AnalysisCoveragePremise,
    CatalogEvidenceAnalysisProposition:$crate::domain::analysis::catalog_evidence::AnalysisProposition,
    CatalogEvidenceAnalysisDerivation:$crate::domain::analysis::catalog_evidence::AnalysisDerivation,
    CatalogEvidenceAnalysisDerivationPremise:$crate::domain::analysis::catalog_evidence::AnalysisDerivationPremise,
    SelectionAnalysisInput:$crate::domain::analysis::selection::AnalysisInput,
    SelectionAnalysisOutcome:$crate::domain::analysis::selection::AnalysisOutcome,
    SelectionAnalysisCoverage:$crate::domain::analysis::selection::AnalysisCoverage,
    SelectionAnalysisCoveragePremise:$crate::domain::analysis::selection::AnalysisCoveragePremise,
    SelectionAnalysisProposition:$crate::domain::analysis::selection::AnalysisProposition,
    SelectionAnalysisDerivation:$crate::domain::analysis::selection::AnalysisDerivation,
    SelectionAnalysisDerivationPremise:$crate::domain::analysis::selection::AnalysisDerivationPremise,
    SynthesisAnalysisInput:$crate::domain::analysis::synthesis::AnalysisInput,
    SynthesisAnalysisOutcome:$crate::domain::analysis::synthesis::AnalysisOutcome,
    SynthesisAnalysisCoverage:$crate::domain::analysis::synthesis::AnalysisCoverage,
    SynthesisAnalysisCoveragePremise:$crate::domain::analysis::synthesis::AnalysisCoveragePremise,
    SynthesisAnalysisProposition:$crate::domain::analysis::synthesis::AnalysisProposition,
    SynthesisAnalysisDerivation:$crate::domain::analysis::synthesis::AnalysisDerivation,
    SynthesisAnalysisDerivationPremise:$crate::domain::analysis::synthesis::AnalysisDerivationPremise,
    RetrievalAnalysisInput:$crate::domain::analysis::retrieval::AnalysisInput,
    RetrievalAnalysisOutcome:$crate::domain::analysis::retrieval::AnalysisOutcome,
    RetrievalAnalysisCoverage:$crate::domain::analysis::retrieval::AnalysisCoverage,
    RetrievalAnalysisCoveragePremise:$crate::domain::analysis::retrieval::AnalysisCoveragePremise,
    RetrievalAnalysisProposition:$crate::domain::analysis::retrieval::AnalysisProposition,
    RetrievalAnalysisDerivation:$crate::domain::analysis::retrieval::AnalysisDerivation,
    RetrievalAnalysisDerivationPremise:$crate::domain::analysis::retrieval::AnalysisDerivationPremise,
    Use:$crate::domain::flow::FlowUseObservation,
    Definition:$crate::domain::flow::FlowDefinitionObservation,
    Reaching:$crate::domain::flow::FlowReachingObservation,
    Value:$crate::domain::flow::FlowValueObservation,
    Region:$crate::domain::flow::FlowRegionObservation,
    Test:$crate::domain::flow::FlowTestObservation,
    Leaf:$crate::domain::flow::FlowTestLeafObservation,
    Signature:$crate::domain::calls::Signature,
    CallTarget:$crate::domain::calls::CallTarget,
    ProviderCallSite:$crate::domain::calls::ProviderCallSite,
    CallSyntax:$crate::domain::calls::CallSyntax,
    CallResolution:$crate::domain::calls::CallResolution,
    SymbolDeclaration:$crate::domain::declarations::SymbolDeclaration,
    ParameterDeclaration:$crate::domain::declarations::ParameterDeclaration,
    TaskReportObservation:$crate::domain::deployment::TaskReportObservation,
    DeploymentObservation:$crate::domain::deployment::DeploymentObservation,
    DocumentObservation:$crate::domain::documents::DocumentObservation,
    PassageObservation:$crate::domain::documents::PassageObservation,
    CodeBlockObservation:$crate::domain::documents::CodeBlockObservation,
    DocumentLinkObservation:$crate::domain::documents::DocumentLinkObservation,
    DocumentMentionObservation:$crate::domain::documents::DocumentMentionObservation,
    DocumentComponentObservation:$crate::domain::documents::DocumentComponentObservation,
    DocumentAttributeObservation:$crate::domain::documents::DocumentAttributeObservation,
    FlowAttributeLoadObservation:$crate::domain::flow::FlowAttributeLoadObservation,
    FlowValuePathObservation:$crate::domain::flow::FlowValuePathObservation,
    LexicalScopeObservation:$crate::domain::lexical::LexicalScopeObservation,
    BindingObservation:$crate::domain::lexical::BindingObservation,
    ReferenceObservation:$crate::domain::lexical::ReferenceObservation,
    LexicalResolution:$crate::domain::lexical::LexicalResolution,
    SyntaxObservation:$crate::domain::source::SyntaxObservation,
    SymbolObservation:$crate::domain::symbols::SymbolObservation,
    FunctionTraitObservation:$crate::domain::symbols::FunctionTraitObservation,
    ClassTraitObservation:$crate::domain::symbols::ClassTraitObservation,
    ClassAncestryObservation:$crate::domain::symbols::ClassAncestryObservation,
    ParameterAnnotationObservation:$crate::domain::symbols::ParameterAnnotationObservation,
    PublicNameObservation:$crate::domain::symbols::PublicNameObservation,
    ParameterDocObservation:$crate::domain::symbols::ParameterDocObservation,
    SyntaxPlacement:$crate::domain::syntax::SyntaxPlacement,
    SyntaxDetailObservation:$crate::domain::syntax::SyntaxDetailObservation,
    DeclarationObservation:$crate::domain::syntax::DeclarationObservation,
    DeclarationDecorator:$crate::domain::syntax::DeclarationDecorator,
    ImportAliasObservation:$crate::domain::syntax::ImportAliasObservation,
    DunderAllObservation:$crate::domain::syntax::DunderAllObservation,
    ParameterSyntaxObservation:$crate::domain::syntax::ParameterSyntaxObservation,
    ClassFieldSyntaxObservation:$crate::domain::syntax::ClassFieldSyntaxObservation,
    TypeObservation:$crate::domain::types::TypeObservation,
    TypePresentation:$crate::domain::types::TypePresentation,
    TypeVariableRestriction:$crate::domain::types::TypeVariableRestriction,
    FunctionBodyObservation:$crate::domain::types::FunctionBodyObservation,
    RecordFieldObservation:$crate::domain::types::RecordFieldObservation,
    ClassMemberObservation:$crate::domain::class_metadata::ClassMemberObservation,
    ClassMetadataObservation:$crate::domain::class_metadata::ClassMetadataObservation,
    SignatureEnumerationObservation:$crate::domain::calls::SignatureEnumerationObservation,
    RuffContextObservation:$crate::domain::ruff::RuffContextObservation,
    FlowNarrowing:$crate::domain::flow::FlowNarrowingObservation,
    NativeExit:$crate::domain::protocols::NativeExitObservation,
    NativeTerminal:$crate::domain::protocols::NativeTerminalObservation,
    Capture:$crate::domain::captures::CaptureObservation,
    NativeExitDiagnostic:$crate::domain::protocols::NativeExitDiagnostic,
    FlowSourceView:$crate::domain::flow::FlowSourceViewObservation,
    NativeSignatureObservation:$crate::domain::types::NativeSignatureObservation,
    SignatureTypeObservation:$crate::domain::types::SignatureTypeObservation,
    TypeQuery:$crate::domain::types::TypeQueryObservation,
    GenericSpecialization:$crate::domain::types::GenericSpecializationObservation,
    FlowCaptureTiming:$crate::domain::flow_capture::FlowCaptureTimingObservation,
    ExportEnumeration:$crate::domain::symbols::ExportEnumerationObservation,
    RuffBindingObservation:$crate::domain::ruff::RuffBindingObservation,
    RuffDiagnosticObservation:$crate::domain::diagnostics::RuffDiagnosticObservation,
    PyreflyDiagnosticObservation:$crate::domain::diagnostics::PyreflyDiagnosticObservation,
    NativeParameterDefinitionObservation:$crate::domain::diagnostics::NativeParameterDefinitionObservation,
    RuffDefinitionObservation:$crate::domain::ruff::RuffDefinitionObservation,
    FlowUseInventory:$crate::domain::flow_inventory::FlowUseInventoryObservation,
    NativeOverload:$crate::domain::types::NativeOverloadObservation,
    NativeOverloadCandidate:$crate::domain::types::NativeOverloadCandidate,
    ModuleResolutionObservation:$crate::domain::symbols::ModuleResolutionObservation,
    LocalAssessment:$crate::domain::local_semantics::LocalAssessment,
    LocalTransfer:$crate::domain::local_semantics::LocalContribution,
    LocalGuard:$crate::domain::local_semantics::LocalGuardContribution,
    Theory:$crate::domain::local_theory::TheoryWitness,
    ExpressionEvaluation:$crate::domain::execution::records::ExpressionEvaluation,
    SourceCall:$crate::domain::execution::source_call_records::SourceCallHeader,
    SourceInvocation:$crate::domain::execution::source_call_records::SourceInvocation,
    ModelTransfer:$crate::domain::execution::model_transfer::ModelTransferWitness,
    SummaryClaim:$crate::domain::execution::summary_consequences::SummaryClaim,
    SummaryPath:$crate::domain::execution::summary_path::SummaryPathWitness,
    SummaryRun:$crate::domain::execution::summary_production::SummaryRun,
    NormalizedSymbol:$crate::domain::normalized::entities::SymbolEntityResolution,
    PublicEnumeration:$crate::domain::normalized::entities::PublicEnumerationAssessment,
    BindingAttempt:$crate::domain::normalized::bindings::CallBindingAttempt,
    Binding:$crate::domain::normalized::bindings::CallBinding,
    BindingVariant:$crate::domain::normalized::bindings::BindingVariantAssessment,
    CatalogInvocation:$crate::domain::catalog::CatalogInvocation,
    CatalogPath:$crate::domain::catalog::CatalogPath,
    CatalogAlias:$crate::domain::catalog::CatalogAlias,
    CatalogConstructor:$crate::domain::catalog::CatalogConstructor,
    ScenarioAssociation:$crate::domain::catalog::evidence::ScenarioAssociation,
    DocumentAssociation:$crate::domain::catalog::evidence::DocumentAssociation,
    Deployment:$crate::domain::catalog::evidence::CatalogDeployment,
    ScenarioCheck:$crate::domain::catalog::evidence::ScenarioCheck,
    SelectionDomain:$crate::domain::selection::SelectionDomain,
    StructuralConclusion:$crate::domain::structural::Conclusion,
    StructuralUsage:$crate::domain::structural::UsageScore,
    StructuralControl:$crate::domain::structural::controls::ControlTraversal,
    AnalyticConclusion:$crate::domain::analytics::Conclusion,
    AnalyticTechnique:$crate::domain::analytics::TechniqueResult,
    Rank:$crate::domain::analytics::RankScore,
    Community:$crate::domain::analytics::Community,
    Neighbour:$crate::domain::analytics::Neighbour,
    Concept:$crate::domain::analytics::Concept,
    Implication:$crate::domain::analytics::Implication,
    DocumentaryConclusion:$crate::domain::synthesis::documentary::DocumentaryConclusion,
    ProgrammaticAssertion:$crate::domain::synthesis::assertions::ProgrammaticAssertion,
    Brief:$crate::domain::synthesis::briefs::Brief,
    RetrievalAnchor:$crate::domain::retrieval::OriginalAnchor,
    RetrievalOccurrence:$crate::domain::retrieval::UnitSubject,
    UseSupport:$crate::domain::flow::FlowUseSupport,
    DefinitionSupport:$crate::domain::flow::FlowDefinitionSupport,
    ReachingSupport:$crate::domain::flow::FlowReachingSupport,
    ValueSupport:$crate::domain::flow::FlowValueSupport,
    RegionSupport:$crate::domain::flow::FlowRegionSupport,
    TestSupport:$crate::domain::flow::FlowTestSupport,
    LeafSupport:$crate::domain::flow::FlowTestLeafSupport,
    SignatureSupport:$crate::domain::calls::SignatureSupport,
    CallTargetSupport:$crate::domain::calls::CallTargetSupport,
    ProviderCallSiteSupport:$crate::domain::calls::ProviderCallSiteSupport,
    CallSyntaxSupport:$crate::domain::calls::CallSyntaxSupport,
    CallResolutionSupport:$crate::domain::calls::CallResolutionSupport,
    SymbolDeclarationSupport:$crate::domain::declarations::SymbolDeclarationSupport,
    ParameterDeclarationSupport:$crate::domain::declarations::ParameterDeclarationSupport,
    TaskReportObservationSupport:$crate::domain::deployment::TaskReportSupport,
    DeploymentObservationSupport:$crate::domain::deployment::DeploymentSupport,
    DocumentObservationSupport:$crate::domain::documents::DocumentSupport,
    PassageObservationSupport:$crate::domain::documents::PassageSupport,
    CodeBlockObservationSupport:$crate::domain::documents::CodeBlockSupport,
    DocumentLinkObservationSupport:$crate::domain::documents::DocumentLinkSupport,
    DocumentMentionObservationSupport:$crate::domain::documents::DocumentMentionSupport,
    DocumentComponentObservationSupport:$crate::domain::documents::DocumentComponentSupport,
    DocumentAttributeObservationSupport:$crate::domain::documents::DocumentAttributeSupport,
    FlowAttributeLoadObservationSupport:$crate::domain::flow::FlowAttributeLoadSupport,
    FlowValuePathObservationSupport:$crate::domain::flow::FlowValuePathSupport,
    LexicalScopeObservationSupport:$crate::domain::lexical::LexicalScopeSupport,
    BindingObservationSupport:$crate::domain::lexical::BindingSupport,
    ReferenceObservationSupport:$crate::domain::lexical::ReferenceSupport,
    LexicalResolutionSupport:$crate::domain::lexical::LexicalResolutionSupport,
    SyntaxObservationSupport:$crate::domain::source::SyntaxSupport,
    SymbolObservationSupport:$crate::domain::symbols::SymbolSupport,
    FunctionTraitObservationSupport:$crate::domain::symbols::FunctionTraitSupport,
    ClassTraitObservationSupport:$crate::domain::symbols::ClassTraitSupport,
    ClassAncestryObservationSupport:$crate::domain::symbols::ClassAncestrySupport,
    ParameterAnnotationObservationSupport:$crate::domain::symbols::ParameterAnnotationSupport,
    PublicNameObservationSupport:$crate::domain::symbols::PublicNameSupport,
    ParameterDocObservationSupport:$crate::domain::symbols::ParameterDocSupport,
    SyntaxPlacementSupport:$crate::domain::syntax::SyntaxPlacementSupport,
    SyntaxDetailObservationSupport:$crate::domain::syntax::SyntaxDetailSupport,
    DeclarationObservationSupport:$crate::domain::syntax::DeclarationSupport,
    DeclarationDecoratorSupport:$crate::domain::syntax::DeclarationDecoratorSupport,
    ImportAliasObservationSupport:$crate::domain::syntax::ImportAliasSupport,
    DunderAllObservationSupport:$crate::domain::syntax::DunderAllSupport,
    ParameterSyntaxObservationSupport:$crate::domain::syntax::ParameterSyntaxSupport,
    ClassFieldSyntaxObservationSupport:$crate::domain::syntax::ClassFieldSyntaxSupport,
    TypeObservationSupport:$crate::domain::types::TypeSupport,
    TypePresentationSupport:$crate::domain::types::TypePresentationSupport,
    TypeVariableRestrictionSupport:$crate::domain::types::TypeRestrictionSupport,
    FunctionBodyObservationSupport:$crate::domain::types::FunctionBodySupport,
    RecordFieldObservationSupport:$crate::domain::types::RecordFieldSupport,
    ClassMemberObservationSupport:$crate::domain::class_metadata::ClassMemberSupport,
    ClassMetadataObservationSupport:$crate::domain::class_metadata::ClassMetadataSupport,
    SignatureEnumerationObservationSupport:$crate::domain::calls::SignatureEnumerationSupport,
    RuffContextObservationSupport:$crate::domain::ruff::RuffContextSupport,
    FlowNarrowingSupport:$crate::domain::flow::FlowNarrowingSupport,
    NativeExitSupport:$crate::domain::protocols::NativeExitSupport,
    NativeTerminalSupport:$crate::domain::protocols::NativeTerminalSupport,
    CaptureSupport:$crate::domain::captures::CaptureSupport,
    NativeExitDiagnosticSupport:$crate::domain::protocols::NativeExitDiagnosticSupport,
    FlowSourceViewSupport:$crate::domain::flow::FlowSourceViewSupport,
    NativeSignatureObservationSupport:$crate::domain::types::NativeSignatureSupport,
    SignatureTypeObservationSupport:$crate::domain::types::SignatureTypeSupport,
    TypeQuerySupport:$crate::domain::types::TypeQuerySupport,
    GenericSpecializationSupport:$crate::domain::types::GenericSpecializationSupport,
    FlowCaptureTimingSupport:$crate::domain::flow_capture::FlowCaptureTimingSupport,
    ExportEnumerationSupport:$crate::domain::symbols::ExportEnumerationSupport,
    RuffBindingObservationSupport:$crate::domain::ruff::RuffBindingSupport,
    RuffDiagnosticObservationSupport:$crate::domain::diagnostics::RuffDiagnosticSupport,
    PyreflyDiagnosticObservationSupport:$crate::domain::diagnostics::PyreflyDiagnosticSupport,
    NativeParameterDefinitionObservationSupport:$crate::domain::diagnostics::NativeParameterDefinitionSupport,
    RuffDefinitionObservationSupport:$crate::domain::ruff::RuffDefinitionSupport,
    FlowUseInventorySupport:$crate::domain::flow_inventory::FlowUseInventorySupport,
    NativeOverloadSupport:$crate::domain::types::NativeOverloadSupport,
    NativeOverloadCandidateSupport:$crate::domain::types::NativeOverloadCandidateSupport,
    ModuleResolutionObservationSupport:$crate::domain::symbols::ModuleResolutionSupport,
}};}
impl From<super::input::InputAcquisition> for ProvenanceValue {
    fn from(row: super::input::InputAcquisition) -> Self {
        Self::Acquisition(row)
    }
}
impl GraphAssertionRecord for super::input::InputAcquisition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::CorpusLibrary> for ProvenanceValue {
    fn from(row: super::input::CorpusLibrary) -> Self {
        Self::CorpusMembership(row)
    }
}
impl GraphAssertionRecord for super::input::CorpusLibrary {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::ArtifactUse> for ProvenanceValue {
    fn from(row: super::input::ArtifactUse) -> Self {
        Self::ArtifactUse(row)
    }
}
impl GraphAssertionRecord for super::input::ArtifactUse {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::ArtifactOwnership> for ProvenanceValue {
    fn from(row: super::input::ArtifactOwnership) -> Self {
        Self::ArtifactOwnership(row)
    }
}
impl GraphAssertionRecord for super::input::ArtifactOwnership {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::UnownedArtifact> for ProvenanceValue {
    fn from(row: super::input::UnownedArtifact) -> Self {
        Self::UnownedArtifact(row)
    }
}
impl GraphAssertionRecord for super::input::UnownedArtifact {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::DerivedArtifact> for ProvenanceValue {
    fn from(row: super::input::DerivedArtifact) -> Self {
        Self::DerivedArtifact(row)
    }
}
impl GraphAssertionRecord for super::input::DerivedArtifact {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::InputDistribution> for ProvenanceValue {
    fn from(row: super::input::InputDistribution) -> Self {
        Self::Distribution(row)
    }
}
impl GraphAssertionRecord for super::input::InputDistribution {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::DistributionVerification> for ProvenanceValue {
    fn from(row: super::input::DistributionVerification) -> Self {
        Self::DistributionVerification(row)
    }
}
impl GraphAssertionRecord for super::input::DistributionVerification {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::input::EnvironmentFingerprint> for ProvenanceValue {
    fn from(row: super::input::EnvironmentFingerprint) -> Self {
        Self::Environment(row)
    }
}
impl GraphAssertionRecord for super::input::EnvironmentFingerprint {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::attribution::ProviderCoverage> for ProvenanceValue {
    fn from(row: super::attribution::ProviderCoverage) -> Self {
        Self::Coverage(row)
    }
}
impl GraphAssertionRecord for super::attribution::ProviderCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::attribution::RunFamily> for ProvenanceValue {
    fn from(row: super::attribution::RunFamily) -> Self {
        Self::RunFamily(row)
    }
}
impl GraphAssertionRecord for super::attribution::RunFamily {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisInput) -> Self {
        Self::LocalAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisOutcome) -> Self {
        Self::LocalAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisCoverage) -> Self {
        Self::LocalAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisCoveragePremise) -> Self {
        Self::LocalAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisProposition) -> Self {
        Self::LocalAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisDerivation) -> Self {
        Self::LocalAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisDerivationPremise) -> Self {
        Self::LocalAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisInput) -> Self {
        Self::BaseEvaluationAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisOutcome) -> Self {
        Self::BaseEvaluationAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisCoverage) -> Self {
        Self::BaseEvaluationAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisCoveragePremise) -> Self {
        Self::BaseEvaluationAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisProposition) -> Self {
        Self::BaseEvaluationAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisDerivation) -> Self {
        Self::BaseEvaluationAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisDerivationPremise) -> Self {
        Self::BaseEvaluationAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisInput) -> Self {
        Self::BaseCompletionAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisOutcome) -> Self {
        Self::BaseCompletionAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisCoverage) -> Self {
        Self::BaseCompletionAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisCoveragePremise) -> Self {
        Self::BaseCompletionAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisProposition) -> Self {
        Self::BaseCompletionAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisDerivation) -> Self {
        Self::BaseCompletionAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisDerivationPremise) -> Self {
        Self::BaseCompletionAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisInput) -> Self {
        Self::SourceCallAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisOutcome) -> Self {
        Self::SourceCallAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisCoverage) -> Self {
        Self::SourceCallAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisCoveragePremise) -> Self {
        Self::SourceCallAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisProposition) -> Self {
        Self::SourceCallAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisDerivation) -> Self {
        Self::SourceCallAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisDerivationPremise) -> Self {
        Self::SourceCallAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisInput) -> Self {
        Self::EnrichedExecutionAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisOutcome) -> Self {
        Self::EnrichedExecutionAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisCoverage) -> Self {
        Self::EnrichedExecutionAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisCoveragePremise) -> Self {
        Self::EnrichedExecutionAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisProposition) -> Self {
        Self::EnrichedExecutionAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisDerivation) -> Self {
        Self::EnrichedExecutionAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisDerivationPremise) -> Self {
        Self::EnrichedExecutionAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisInput) -> Self {
        Self::ModelAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisOutcome) -> Self {
        Self::ModelAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisCoverage) -> Self {
        Self::ModelAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisCoveragePremise) -> Self {
        Self::ModelAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisProposition) -> Self {
        Self::ModelAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisDerivation) -> Self {
        Self::ModelAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisDerivationPremise) -> Self {
        Self::ModelAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisInput) -> Self {
        Self::SummaryAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisOutcome) -> Self {
        Self::SummaryAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisCoverage) -> Self {
        Self::SummaryAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisCoveragePremise) -> Self {
        Self::SummaryAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisProposition) -> Self {
        Self::SummaryAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisDerivation) -> Self {
        Self::SummaryAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisDerivationPremise) -> Self {
        Self::SummaryAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisInput) -> Self {
        Self::StructuralAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisOutcome) -> Self {
        Self::StructuralAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisCoverage) -> Self {
        Self::StructuralAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisCoveragePremise) -> Self {
        Self::StructuralAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisProposition) -> Self {
        Self::StructuralAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisDerivation) -> Self {
        Self::StructuralAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisDerivationPremise) -> Self {
        Self::StructuralAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisInput) -> Self {
        Self::AnalyticEmbeddingAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisOutcome) -> Self {
        Self::AnalyticEmbeddingAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisCoverage) -> Self {
        Self::AnalyticEmbeddingAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisCoveragePremise) -> Self {
        Self::AnalyticEmbeddingAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisProposition) -> Self {
        Self::AnalyticEmbeddingAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisDerivation) -> Self {
        Self::AnalyticEmbeddingAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisDerivationPremise) -> Self {
        Self::AnalyticEmbeddingAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisInput) -> Self {
        Self::AnalyticAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisOutcome) -> Self {
        Self::AnalyticAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisCoverage) -> Self {
        Self::AnalyticAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisCoveragePremise) -> Self {
        Self::AnalyticAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisProposition) -> Self {
        Self::AnalyticAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisDerivation) -> Self {
        Self::AnalyticAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisDerivationPremise) -> Self {
        Self::AnalyticAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisInput) -> Self {
        Self::CatalogCoreAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisOutcome) -> Self {
        Self::CatalogCoreAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisCoverage) -> Self {
        Self::CatalogCoreAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisCoveragePremise) -> Self {
        Self::CatalogCoreAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisProposition) -> Self {
        Self::CatalogCoreAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisDerivation) -> Self {
        Self::CatalogCoreAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisDerivationPremise) -> Self {
        Self::CatalogCoreAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisInput) -> Self {
        Self::CatalogEvidenceAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisOutcome) -> Self {
        Self::CatalogEvidenceAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisCoverage) -> Self {
        Self::CatalogEvidenceAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisCoveragePremise) -> Self {
        Self::CatalogEvidenceAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisProposition) -> Self {
        Self::CatalogEvidenceAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisDerivation) -> Self {
        Self::CatalogEvidenceAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisDerivationPremise) -> Self {
        Self::CatalogEvidenceAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisInput) -> Self {
        Self::SelectionAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisOutcome) -> Self {
        Self::SelectionAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisCoverage) -> Self {
        Self::SelectionAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisCoveragePremise) -> Self {
        Self::SelectionAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisProposition) -> Self {
        Self::SelectionAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisDerivation) -> Self {
        Self::SelectionAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisDerivationPremise) -> Self {
        Self::SelectionAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisInput) -> Self {
        Self::SynthesisAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisOutcome) -> Self {
        Self::SynthesisAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisCoverage) -> Self {
        Self::SynthesisAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisCoveragePremise) -> Self {
        Self::SynthesisAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisProposition) -> Self {
        Self::SynthesisAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisDerivation) -> Self {
        Self::SynthesisAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisDerivationPremise) -> Self {
        Self::SynthesisAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisInput> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisInput) -> Self {
        Self::RetrievalAnalysisInput(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisInput {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisOutcome> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisOutcome) -> Self {
        Self::RetrievalAnalysisOutcome(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisCoverage> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisCoverage) -> Self {
        Self::RetrievalAnalysisCoverage(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisCoveragePremise> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisCoveragePremise) -> Self {
        Self::RetrievalAnalysisCoveragePremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisCoveragePremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisProposition> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisProposition) -> Self {
        Self::RetrievalAnalysisProposition(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisProposition {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisDerivation> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisDerivation) -> Self {
        Self::RetrievalAnalysisDerivation(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisDerivation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisDerivationPremise> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisDerivationPremise) -> Self {
        Self::RetrievalAnalysisDerivationPremise(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisDerivationPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::types::TypeSequenceMember> for MembershipValue {
    fn from(row: super::types::TypeSequenceMember) -> Self {
        Self::TypeSequenceMember(row)
    }
}
impl GraphAssertionRecord for super::types::TypeSequenceMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}
impl From<super::symbols::SymbolSequenceMember> for MembershipValue {
    fn from(row: super::symbols::SymbolSequenceMember) -> Self {
        Self::SymbolSequenceMember(row)
    }
}
impl GraphAssertionRecord for super::symbols::SymbolSequenceMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}
impl From<super::assumptions::AssumptionSetMember> for MembershipValue {
    fn from(row: super::assumptions::AssumptionSetMember) -> Self {
        Self::AssumptionMember(row)
    }
}
impl GraphAssertionRecord for super::assumptions::AssumptionSetMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}
impl From<super::value::LiteralSetMember> for MembershipValue {
    fn from(row: super::value::LiteralSetMember) -> Self {
        Self::LiteralMember(row)
    }
}
impl GraphAssertionRecord for super::value::LiteralSetMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analysis::native::NativeQualification> for ProvenanceValue {
    fn from(row: super::analysis::native::NativeQualification) -> Self {
        Self::NativeQualification(row)
    }
}
impl GraphAssertionRecord for super::analysis::native::NativeQualification {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::local::AnalysisObligation) -> Self {
        Self::LocalAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::local::CoverageRequirement) -> Self {
        Self::LocalCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::local::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::local::CoverageRequiredSource) -> Self {
        Self::LocalCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::local::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::AnalysisObligation) -> Self {
        Self::BaseEvaluationAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::CoverageRequirement) -> Self {
        Self::BaseEvaluationCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_evaluation::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::base_evaluation::CoverageRequiredSource) -> Self {
        Self::BaseEvaluationCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_evaluation::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::AnalysisObligation) -> Self {
        Self::BaseCompletionAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::CoverageRequirement) -> Self {
        Self::BaseCompletionCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::base_completion::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::base_completion::CoverageRequiredSource) -> Self {
        Self::BaseCompletionCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::base_completion::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::source_call::AnalysisObligation) -> Self {
        Self::SourceCallAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::source_call::CoverageRequirement) -> Self {
        Self::SourceCallCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::source_call::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::source_call::CoverageRequiredSource) -> Self {
        Self::SourceCallCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::source_call::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::AnalysisObligation) -> Self {
        Self::EnrichedExecutionAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::CoverageRequirement) -> Self {
        Self::EnrichedExecutionCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::enriched_execution::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::enriched_execution::CoverageRequiredSource) -> Self {
        Self::EnrichedExecutionCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::enriched_execution::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::model::AnalysisObligation) -> Self {
        Self::ModelAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::model::CoverageRequirement) -> Self {
        Self::ModelCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::model::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::model::CoverageRequiredSource) -> Self {
        Self::ModelCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::model::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::summary::AnalysisObligation) -> Self {
        Self::SummaryAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::summary::CoverageRequirement) -> Self {
        Self::SummaryCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::summary::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::summary::CoverageRequiredSource) -> Self {
        Self::SummaryCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::summary::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::structural::AnalysisObligation) -> Self {
        Self::StructuralAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::structural::CoverageRequirement) -> Self {
        Self::StructuralCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::structural::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::structural::CoverageRequiredSource) -> Self {
        Self::StructuralCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::structural::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::AnalysisObligation) -> Self {
        Self::AnalyticEmbeddingAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::CoverageRequirement) -> Self {
        Self::AnalyticEmbeddingCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic_embedding::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::analytic_embedding::CoverageRequiredSource) -> Self {
        Self::AnalyticEmbeddingCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic_embedding::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::analytic::AnalysisObligation) -> Self {
        Self::AnalyticAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::analytic::CoverageRequirement) -> Self {
        Self::AnalyticCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::analytic::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::analytic::CoverageRequiredSource) -> Self {
        Self::AnalyticCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::analytic::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::AnalysisObligation) -> Self {
        Self::CatalogCoreAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::CoverageRequirement) -> Self {
        Self::CatalogCoreCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_core::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::catalog_core::CoverageRequiredSource) -> Self {
        Self::CatalogCoreCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_core::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::AnalysisObligation) -> Self {
        Self::CatalogEvidenceAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::CoverageRequirement) -> Self {
        Self::CatalogEvidenceCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::catalog_evidence::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::catalog_evidence::CoverageRequiredSource) -> Self {
        Self::CatalogEvidenceCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::catalog_evidence::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::selection::AnalysisObligation) -> Self {
        Self::SelectionAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::selection::CoverageRequirement) -> Self {
        Self::SelectionCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::selection::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::selection::CoverageRequiredSource) -> Self {
        Self::SelectionCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::selection::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::AnalysisObligation) -> Self {
        Self::SynthesisAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::CoverageRequirement) -> Self {
        Self::SynthesisCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::synthesis::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::synthesis::CoverageRequiredSource) -> Self {
        Self::SynthesisCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::synthesis::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::AnalysisObligation> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::AnalysisObligation) -> Self {
        Self::RetrievalAnalysisObligation(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::AnalysisObligation {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::CoverageRequirement> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::CoverageRequirement) -> Self {
        Self::RetrievalCoverageRequirement(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::CoverageRequirement {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analysis::retrieval::CoverageRequiredSource> for ProvenanceValue {
    fn from(row: super::analysis::retrieval::CoverageRequiredSource) -> Self {
        Self::RetrievalCoverageRequiredSource(row)
    }
}
impl GraphAssertionRecord for super::analysis::retrieval::CoverageRequiredSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

/// Explicit semantic graph policies. A change in role mapping or declared derivation meaning
/// revises these constants; implementation source and artifact codec versions are independent.
pub const GRAPH_ROLE_POLICY_REVISION: u32 = 3;
pub const GRAPH_DERIVATION_POLICY_REVISION: u32 = 1;
pub fn semantic_contract(model: &super::ValidatedModel) -> ContentHash {
    semantic_contract_with_policy(
        model,
        GRAPH_ROLE_POLICY_REVISION,
        GRAPH_DERIVATION_POLICY_REVISION,
    )
}
fn semantic_contract_with_policy(
    model: &super::ValidatedModel,
    roles: u32,
    derivations: u32,
) -> ContentHash {
    let mut inventory = std::collections::BTreeMap::new();
    fn declaration<R: Record>(category: u8, kind: u16) -> ContentHash {
        let mut sink = KeySink::new("graph-record-contract/v1");
        sink.part(b"category", &[category]);
        sink.part(b"kind", &kind.to_le_bytes());
        sink.part(b"name", R::NAME.as_bytes());
        for field in R::fields() {
            field.encode_contract(&mut sink);
        }
        if let Some(sum) = R::sum() {
            sum.encode_contract(&mut sink);
        }
        sink.finish()
    }
    macro_rules! entities{($($variant:ident:$record:ty,)*)=>{$(inventory.insert(<$record>::NAME,declaration::<$record>(0,<$record as GraphEntityRecord>::GRAPH_KIND as u16));)*};}
    macro_rules! assertions{($($variant:ident:$record:ty,)*)=>{$(inventory.insert(<$record>::NAME,declaration::<$record>(1,<$record as GraphAssertionRecord>::GRAPH_KIND as u16));)*};}
    crate::graph_entity_records!(entities);
    crate::graph_assertion_records!(assertions);
    let mut sink = KeySink::new("graph-semantic-contract/v1");
    model.digest().encode(&mut sink);
    sink.part(b"role-policy", &roles.to_le_bytes());
    sink.part(b"derivation-policy", &derivations.to_le_bytes());
    for (name, declaration) in inventory {
        sink.part(b"name", name.as_bytes());
        declaration.encode(&mut sink);
    }
    sink.finish()
}
#[cfg(test)]
mod policy_controls {
    use super::*;
    #[test]
    fn graph_policy_revisions_and_intrinsic_categories_enter_contract() {
        let model = crate::domain::model().unwrap();
        let base = semantic_contract(&model);
        assert_ne!(base, model.digest());
        assert_eq!(base, semantic_contract(&model));
        assert_ne!(
            base,
            semantic_contract_with_policy(
                &model,
                GRAPH_ROLE_POLICY_REVISION + 1,
                GRAPH_DERIVATION_POLICY_REVISION
            )
        );
        assert_ne!(
            base,
            semantic_contract_with_policy(
                &model,
                GRAPH_ROLE_POLICY_REVISION,
                GRAPH_DERIVATION_POLICY_REVISION + 1
            )
        );
    }
}

impl From<super::local_theory::BuiltinOperandWitness> for ClaimValue {
    fn from(row: super::local_theory::BuiltinOperandWitness) -> Self {
        Self::LocalBuiltinOperandWitnesses(row)
    }
}
impl GraphAssertionRecord for super::local_theory::BuiltinOperandWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::calls::BindingSource> for ClaimValue {
    fn from(row: super::calls::BindingSource) -> Self {
        Self::BindingSources(row)
    }
}
impl GraphAssertionRecord for super::calls::BindingSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::calls::BindingProjection> for ClaimValue {
    fn from(row: super::calls::BindingProjection) -> Self {
        Self::BindingProjections(row)
    }
}
impl GraphAssertionRecord for super::calls::BindingProjection {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::atom_decision::AtomRestriction> for ClaimValue {
    fn from(row: super::atom_decision::AtomRestriction) -> Self {
        Self::LocalAtomRestrictions(row)
    }
}
impl GraphAssertionRecord for super::atom_decision::AtomRestriction {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::embedding::analytic::AnalysisEmbeddingUse> for ClaimValue {
    fn from(row: super::embedding::analytic::AnalysisEmbeddingUse) -> Self {
        Self::AnalysisEmbeddingUses(row)
    }
}
impl GraphAssertionRecord for super::embedding::analytic::AnalysisEmbeddingUse {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::catalog::CatalogMemberInvocation> for ClaimValue {
    fn from(row: super::catalog::CatalogMemberInvocation) -> Self {
        Self::CatalogMemberInvocations(row)
    }
}
impl GraphAssertionRecord for super::catalog::CatalogMemberInvocation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::seeds::SelectedSeed> for ClaimValue {
    fn from(row: super::synthesis::seeds::SelectedSeed) -> Self {
        Self::SynthesisSelectedSeeds(row)
    }
}
impl GraphAssertionRecord for super::synthesis::seeds::SelectedSeed {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::patterns::AuthoredCodeConclusion> for ClaimValue {
    fn from(row: super::synthesis::patterns::AuthoredCodeConclusion) -> Self {
        Self::SynthesisAuthoredCodeConclusions(row)
    }
}
impl GraphAssertionRecord for super::synthesis::patterns::AuthoredCodeConclusion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::documentary::ProseSlice> for ClaimValue {
    fn from(row: super::synthesis::documentary::ProseSlice) -> Self {
        Self::SynthesisProseSlices(row)
    }
}
impl GraphAssertionRecord for super::synthesis::documentary::ProseSlice {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::documentary::DocumentarySource> for ClaimValue {
    fn from(row: super::synthesis::documentary::DocumentarySource) -> Self {
        Self::SynthesisDocumentarySources(row)
    }
}
impl GraphAssertionRecord for super::synthesis::documentary::DocumentarySource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::conditions::entry::EntryValueWitness> for ClaimValue {
    fn from(row: super::conditions::entry::EntryValueWitness) -> Self {
        Self::EntryValueWitnesses(row)
    }
}
impl GraphAssertionRecord for super::conditions::entry::EntryValueWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::conditions::stability::StabilityWitness> for ClaimValue {
    fn from(row: super::conditions::stability::StabilityWitness) -> Self {
        Self::StabilityWitnesses(row)
    }
}
impl GraphAssertionRecord for super::conditions::stability::StabilityWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::ConclusionSource> for ClaimValue {
    fn from(row: super::structural::ConclusionSource) -> Self {
        Self::StructuralConclusionSources(row)
    }
}
impl GraphAssertionRecord for super::structural::ConclusionSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::summary_terminal::SummaryTerminalWitness> for ClaimValue {
    fn from(row: super::execution::summary_terminal::SummaryTerminalWitness) -> Self {
        Self::SummaryTerminalWitnesses(row)
    }
}
impl GraphAssertionRecord for super::execution::summary_terminal::SummaryTerminalWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::completion_records::StatementCompletion> for ClaimValue {
    fn from(row: super::execution::completion_records::StatementCompletion) -> Self {
        Self::BaseStatementCompletions(row)
    }
}
impl GraphAssertionRecord for super::execution::completion_records::StatementCompletion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::modeled_call::ModeledCallEvaluation> for ClaimValue {
    fn from(row: super::execution::modeled_call::ModeledCallEvaluation) -> Self {
        Self::ModeledCallEvaluations(row)
    }
}
impl GraphAssertionRecord for super::execution::modeled_call::ModeledCallEvaluation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::summary_path::SummaryPathRoute> for ClaimValue {
    fn from(row: super::execution::summary_path::SummaryPathRoute) -> Self {
        Self::SummaryPathRoutes(row)
    }
}
impl GraphAssertionRecord for super::execution::summary_path::SummaryPathRoute {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::model_production::ModelApplication> for ClaimValue {
    fn from(row: super::execution::model_production::ModelApplication) -> Self {
        Self::ModelApplications(row)
    }
}
impl GraphAssertionRecord for super::execution::model_production::ModelApplication {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::summary_capture::SummaryCaptureWitness> for ClaimValue {
    fn from(row: super::execution::summary_capture::SummaryCaptureWitness) -> Self {
        Self::SummaryCaptureWitnesses(row)
    }
}
impl GraphAssertionRecord for super::execution::summary_capture::SummaryCaptureWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::body_records::SourceBodyCompletion> for ClaimValue {
    fn from(row: super::execution::body_records::SourceBodyCompletion) -> Self {
        Self::BaseSourceBodyCompletions(row)
    }
}
impl GraphAssertionRecord for super::execution::body_records::SourceBodyCompletion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::protocol_interpretation::ConditionalTerminalFrontier> for ClaimValue {
    fn from(row: super::execution::protocol_interpretation::ConditionalTerminalFrontier) -> Self {
        Self::ConditionalTerminalFrontiers(row)
    }
}
impl GraphAssertionRecord
    for super::execution::protocol_interpretation::ConditionalTerminalFrontier
{
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::protocol_interpretation::NormalContinuationRestriction> for ClaimValue {
    fn from(row: super::execution::protocol_interpretation::NormalContinuationRestriction) -> Self {
        Self::NormalContinuationRestrictions(row)
    }
}
impl GraphAssertionRecord
    for super::execution::protocol_interpretation::NormalContinuationRestriction
{
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::summary_consequences::ClaimProof> for ClaimValue {
    fn from(row: super::execution::summary_consequences::ClaimProof) -> Self {
        Self::SummaryClaimProofs(row)
    }
}
impl GraphAssertionRecord for super::execution::summary_consequences::ClaimProof {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::summary_control::SummaryControlWitness> for ClaimValue {
    fn from(row: super::execution::summary_control::SummaryControlWitness) -> Self {
        Self::SummaryControlWitnesses(row)
    }
}
impl GraphAssertionRecord for super::execution::summary_control::SummaryControlWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::definition::DefinitionEvaluation> for ClaimValue {
    fn from(row: super::execution::definition::DefinitionEvaluation) -> Self {
        Self::DefinitionEvaluations(row)
    }
}
impl GraphAssertionRecord for super::execution::definition::DefinitionEvaluation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::model_rules::AppliedRule> for ClaimValue {
    fn from(row: super::execution::model_rules::AppliedRule) -> Self {
        Self::ModelAppliedRules(row)
    }
}
impl GraphAssertionRecord for super::execution::model_rules::AppliedRule {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::context_execution::ContextExecution> for ClaimValue {
    fn from(row: super::execution::context_execution::ContextExecution) -> Self {
        Self::ContextExecutions(row)
    }
}
impl GraphAssertionRecord for super::execution::context_execution::ContextExecution {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::enriched_records::StatementExecution> for ClaimValue {
    fn from(row: super::execution::enriched_records::StatementExecution) -> Self {
        Self::StatementExecutions(row)
    }
}
impl GraphAssertionRecord for super::execution::enriched_records::StatementExecution {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::enriched_records::BodyExecution> for ClaimValue {
    fn from(row: super::execution::enriched_records::BodyExecution) -> Self {
        Self::BodyExecutions(row)
    }
}
impl GraphAssertionRecord for super::execution::enriched_records::BodyExecution {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::enriched_records::SourceExecutionInvocation> for ClaimValue {
    fn from(row: super::execution::enriched_records::SourceExecutionInvocation) -> Self {
        Self::SourceExecutionInvocations(row)
    }
}
impl GraphAssertionRecord for super::execution::enriched_records::SourceExecutionInvocation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::context_binding::ContextEntryBinding> for ClaimValue {
    fn from(row: super::execution::context_binding::ContextEntryBinding) -> Self {
        Self::ContextEntryBindings(row)
    }
}
impl GraphAssertionRecord for super::execution::context_binding::ContextEntryBinding {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::summary_symbolic::SymbolicFieldAlternative> for ClaimValue {
    fn from(row: super::execution::summary_symbolic::SymbolicFieldAlternative) -> Self {
        Self::SummarySymbolicFieldAlternatives(row)
    }
}
impl GraphAssertionRecord for super::execution::summary_symbolic::SymbolicFieldAlternative {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::model_context_transfer::ContextTransferWitness> for ClaimValue {
    fn from(row: super::execution::model_context_transfer::ContextTransferWitness) -> Self {
        Self::ModelContextTransferWitnesses(row)
    }
}
impl GraphAssertionRecord for super::execution::model_context_transfer::ContextTransferWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::source_call_records::SourceCallOutcome> for ClaimValue {
    fn from(row: super::execution::source_call_records::SourceCallOutcome) -> Self {
        Self::SourceCallOutcomes(row)
    }
}
impl GraphAssertionRecord for super::execution::source_call_records::SourceCallOutcome {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::source_call_records::SourceFrameRelease> for ClaimValue {
    fn from(row: super::execution::source_call_records::SourceFrameRelease) -> Self {
        Self::SourceCallFrameReleases(row)
    }
}
impl GraphAssertionRecord for super::execution::source_call_records::SourceFrameRelease {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::summary::SummaryPremise> for ClaimValue {
    fn from(row: super::transfer::summary::SummaryPremise) -> Self {
        Self::SummaryTransferPremises(row)
    }
}
impl GraphAssertionRecord for super::transfer::summary::SummaryPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::summary::SummaryWitness> for ClaimValue {
    fn from(row: super::transfer::summary::SummaryWitness) -> Self {
        Self::SummaryTransferWitnesses(row)
    }
}
impl GraphAssertionRecord for super::transfer::summary::SummaryWitness {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analytics::ConclusionSource> for ClaimValue {
    fn from(row: super::analytics::ConclusionSource) -> Self {
        Self::AnalyticConclusionSources(row)
    }
}
impl GraphAssertionRecord for super::analytics::ConclusionSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::coverage::NormalizationCoverage> for ClaimValue {
    fn from(row: super::normalized::coverage::NormalizationCoverage) -> Self {
        Self::NormalizationCoverage(row)
    }
}
impl GraphAssertionRecord for super::normalized::coverage::NormalizationCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::ReferenceEntityCandidate> for ClaimValue {
    fn from(row: super::normalized::links::ReferenceEntityCandidate) -> Self {
        Self::ReferenceEntityCandidates(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::ReferenceEntityCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::AncestryEntityMember> for ClaimValue {
    fn from(row: super::normalized::links::AncestryEntityMember) -> Self {
        Self::AncestryEntityMembers(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::AncestryEntityMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::MentionEntityCandidate> for ClaimValue {
    fn from(row: super::normalized::links::MentionEntityCandidate) -> Self {
        Self::MentionEntityCandidates(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::MentionEntityCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::TestOperandTypeLink> for ClaimValue {
    fn from(row: super::normalized::links::TestOperandTypeLink) -> Self {
        Self::TestOperandTypeLinks(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::TestOperandTypeLink {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::entities::OccurrenceOwnership> for ClaimValue {
    fn from(row: super::normalized::entities::OccurrenceOwnership) -> Self {
        Self::OccurrenceOwnership(row)
    }
}
impl GraphAssertionRecord for super::normalized::entities::OccurrenceOwnership {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::events::NormalizedCallEvent> for ClaimValue {
    fn from(row: super::normalized::events::NormalizedCallEvent) -> Self {
        Self::NormalizedCallEvents(row)
    }
}
impl GraphAssertionRecord for super::normalized::events::NormalizedCallEvent {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::events::NormalizedCallAlternative> for ClaimValue {
    fn from(row: super::normalized::events::NormalizedCallAlternative) -> Self {
        Self::NormalizedCallAlternatives(row)
    }
}
impl GraphAssertionRecord for super::normalized::events::NormalizedCallAlternative {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::callables::EffectiveCallableAssessment> for ClaimValue {
    fn from(row: super::normalized::callables::EffectiveCallableAssessment) -> Self {
        Self::EffectiveCallableAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::callables::EffectiveCallableAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::bindings::BindingSetAssessment> for ClaimValue {
    fn from(row: super::normalized::bindings::BindingSetAssessment) -> Self {
        Self::BindingSetAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::bindings::BindingSetAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::receiver::ReceiverAssessment> for ClaimValue {
    fn from(row: super::normalized::receiver::ReceiverAssessment) -> Self {
        Self::ReceiverAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::receiver::ReceiverAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::dispatch::DispatchMember> for ClaimValue {
    fn from(row: super::normalized::dispatch::DispatchMember) -> Self {
        Self::NormalizedDispatchMembers(row)
    }
}
impl GraphAssertionRecord for super::normalized::dispatch::DispatchMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::local::ControlInfluence> for ClaimValue {
    fn from(row: super::transfer::local::ControlInfluence) -> Self {
        Self::LocalControlInfluences(row)
    }
}
impl GraphAssertionRecord for super::transfer::local::ControlInfluence {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}

impl From<super::local_theory::TypeDomainAssessment> for ClaimValue {
    fn from(row: super::local_theory::TypeDomainAssessment) -> Self {
        Self::LocalTypeDomainAssessments(row)
    }
}
impl GraphAssertionRecord for super::local_theory::TypeDomainAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::local_theory::TypeDomainMember> for ClaimValue {
    fn from(row: super::local_theory::TypeDomainMember) -> Self {
        Self::LocalTypeDomainMembers(row)
    }
}
impl GraphAssertionRecord for super::local_theory::TypeDomainMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::deployment::ReportEntry> for ClaimValue {
    fn from(row: super::deployment::ReportEntry) -> Self {
        Self::ReportEntries(row)
    }
}
impl GraphAssertionRecord for super::deployment::ReportEntry {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::calls::CallOriginStep> for ClaimValue {
    fn from(row: super::calls::CallOriginStep) -> Self {
        Self::CallOriginSteps(row)
    }
}
impl GraphAssertionRecord for super::calls::CallOriginStep {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::local_symbolic::SymbolicFieldStore> for ClaimValue {
    fn from(row: super::local_symbolic::SymbolicFieldStore) -> Self {
        Self::LocalSymbolicFieldStores(row)
    }
}
impl GraphAssertionRecord for super::local_symbolic::SymbolicFieldStore {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::flow_capture::FlowCaptureCandidate> for ClaimValue {
    fn from(row: super::flow_capture::FlowCaptureCandidate) -> Self {
        Self::FlowCaptureCandidates(row)
    }
}
impl GraphAssertionRecord for super::flow_capture::FlowCaptureCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::atom_decision::AtomDecision> for ClaimValue {
    fn from(row: super::atom_decision::AtomDecision) -> Self {
        Self::LocalAtomDecisions(row)
    }
}
impl GraphAssertionRecord for super::atom_decision::AtomDecision {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::flow::FlowCallStep> for ClaimValue {
    fn from(row: super::flow::FlowCallStep) -> Self {
        Self::FlowCallSteps(row)
    }
}
impl GraphAssertionRecord for super::flow::FlowCallStep {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::projection::ProjectionSourceAssessment> for ClaimValue {
    fn from(row: super::projection::ProjectionSourceAssessment) -> Self {
        Self::ProjectionSourceAssessments(row)
    }
}
impl GraphAssertionRecord for super::projection::ProjectionSourceAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::embedding::text::TextWindow> for ClaimValue {
    fn from(row: super::embedding::text::TextWindow) -> Self {
        Self::AnalyticTextWindows(row)
    }
}
impl GraphAssertionRecord for super::embedding::text::TextWindow {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::seeds::SeedPlan> for ClaimValue {
    fn from(row: super::synthesis::seeds::SeedPlan) -> Self {
        Self::SynthesisSeedPlans(row)
    }
}
impl GraphAssertionRecord for super::synthesis::seeds::SeedPlan {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::seeds::SelectedSeedSource> for ClaimValue {
    fn from(row: super::synthesis::seeds::SelectedSeedSource) -> Self {
        Self::SynthesisSelectedSeedSources(row)
    }
}
impl GraphAssertionRecord for super::synthesis::seeds::SelectedSeedSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::patterns::AuthoredCodeSource> for ClaimValue {
    fn from(row: super::synthesis::patterns::AuthoredCodeSource) -> Self {
        Self::SynthesisAuthoredCodeSources(row)
    }
}
impl GraphAssertionRecord for super::synthesis::patterns::AuthoredCodeSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::documentary::ProseSource> for ClaimValue {
    fn from(row: super::synthesis::documentary::ProseSource) -> Self {
        Self::SynthesisProseSources(row)
    }
}
impl GraphAssertionRecord for super::synthesis::documentary::ProseSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::conditions::entry::EntryAccessSource> for ClaimValue {
    fn from(row: super::conditions::entry::EntryAccessSource) -> Self {
        Self::EntryAccessSources(row)
    }
}
impl GraphAssertionRecord for super::conditions::entry::EntryAccessSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::PublicCandidate> for ClaimValue {
    fn from(row: super::structural::PublicCandidate) -> Self {
        Self::StructuralPublicCandidates(row)
    }
}
impl GraphAssertionRecord for super::structural::PublicCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::Traversal> for ClaimValue {
    fn from(row: super::structural::Traversal) -> Self {
        Self::StructuralTraversals(row)
    }
}
impl GraphAssertionRecord for super::structural::Traversal {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::Path> for ClaimValue {
    fn from(row: super::structural::Path) -> Self {
        Self::StructuralPaths(row)
    }
}
impl GraphAssertionRecord for super::structural::Path {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::UnresolvedEvent> for ClaimValue {
    fn from(row: super::structural::UnresolvedEvent) -> Self {
        Self::StructuralUnresolvedEvents(row)
    }
}
impl GraphAssertionRecord for super::structural::UnresolvedEvent {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::controls::LiteralArgument> for ClaimValue {
    fn from(row: super::structural::controls::LiteralArgument) -> Self {
        Self::StructuralLiteralArguments(row)
    }
}
impl GraphAssertionRecord for super::structural::controls::LiteralArgument {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::controls::ControlPath> for ClaimValue {
    fn from(row: super::structural::controls::ControlPath) -> Self {
        Self::StructuralControlPaths(row)
    }
}
impl GraphAssertionRecord for super::structural::controls::ControlPath {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::controls::ConditionalRaise> for ClaimValue {
    fn from(row: super::structural::controls::ConditionalRaise) -> Self {
        Self::StructuralConditionalRaises(row)
    }
}
impl GraphAssertionRecord for super::structural::controls::ConditionalRaise {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::controls::UnfollowedPath> for ClaimValue {
    fn from(row: super::structural::controls::UnfollowedPath) -> Self {
        Self::StructuralUnfollowedPaths(row)
    }
}
impl GraphAssertionRecord for super::structural::controls::UnfollowedPath {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::handoffs::Group> for ClaimValue {
    fn from(row: super::structural::handoffs::Group) -> Self {
        Self::StructuralHandoffGroups(row)
    }
}
impl GraphAssertionRecord for super::structural::handoffs::Group {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::closed_targets::ClosedTargetAssessment> for ClaimValue {
    fn from(row: super::execution::closed_targets::ClosedTargetAssessment) -> Self {
        Self::ClosedTargetAssessments(row)
    }
}
impl GraphAssertionRecord for super::execution::closed_targets::ClosedTargetAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::model_protocol::ContextResource> for ClaimValue {
    fn from(row: super::execution::model_protocol::ContextResource) -> Self {
        Self::ModelContextResources(row)
    }
}
impl GraphAssertionRecord for super::execution::model_protocol::ContextResource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::capture_bridge::CapturedEntryBinding> for ClaimValue {
    fn from(row: super::execution::capture_bridge::CapturedEntryBinding) -> Self {
        Self::CapturedEntryBindings(row)
    }
}
impl GraphAssertionRecord for super::execution::capture_bridge::CapturedEntryBinding {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analytics::DocumentNeighbour> for ClaimValue {
    fn from(row: super::analytics::DocumentNeighbour) -> Self {
        Self::AnalyticDocumentNeighbours(row)
    }
}
impl GraphAssertionRecord for super::analytics::DocumentNeighbour {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analytics::CommunityLabel> for ClaimValue {
    fn from(row: super::analytics::CommunityLabel) -> Self {
        Self::AnalyticCommunityLabels(row)
    }
}
impl GraphAssertionRecord for super::analytics::CommunityLabel {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::symbolic_fields::SourceFieldReaderLink> for ClaimValue {
    fn from(row: super::normalized::symbolic_fields::SourceFieldReaderLink) -> Self {
        Self::SourceFieldReaderLinks(row)
    }
}
impl GraphAssertionRecord for super::normalized::symbolic_fields::SourceFieldReaderLink {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::callable_aspects::FieldDefaultAssessment> for ClaimValue {
    fn from(row: super::normalized::callable_aspects::FieldDefaultAssessment) -> Self {
        Self::ClassFieldDefaultAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::callable_aspects::FieldDefaultAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::ReferenceEntityTarget> for ClaimValue {
    fn from(row: super::normalized::links::ReferenceEntityTarget) -> Self {
        Self::ReferenceEntityTargets(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::ReferenceEntityTarget {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::ReferenceEntityAssessment> for ClaimValue {
    fn from(row: super::normalized::links::ReferenceEntityAssessment) -> Self {
        Self::ReferenceEntityAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::ReferenceEntityAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::AncestryEntityAssessment> for ClaimValue {
    fn from(row: super::normalized::links::AncestryEntityAssessment) -> Self {
        Self::AncestryEntityAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::AncestryEntityAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::MentionEntityAssessment> for ClaimValue {
    fn from(row: super::normalized::links::MentionEntityAssessment) -> Self {
        Self::MentionEntityAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::MentionEntityAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::links::TestOperandTypeAssessment> for ClaimValue {
    fn from(row: super::normalized::links::TestOperandTypeAssessment) -> Self {
        Self::TestOperandTypeAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::links::TestOperandTypeAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::entities::SymbolEntityCandidate> for ClaimValue {
    fn from(row: super::normalized::entities::SymbolEntityCandidate) -> Self {
        Self::SymbolEntityCandidates(row)
    }
}
impl GraphAssertionRecord for super::normalized::entities::SymbolEntityCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::entities::ParameterEntityLink> for ClaimValue {
    fn from(row: super::normalized::entities::ParameterEntityLink) -> Self {
        Self::ParameterEntityLinks(row)
    }
}
impl GraphAssertionRecord for super::normalized::entities::ParameterEntityLink {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::entities::FieldEntityLink> for ClaimValue {
    fn from(row: super::normalized::entities::FieldEntityLink) -> Self {
        Self::FieldEntityLinks(row)
    }
}
impl GraphAssertionRecord for super::normalized::entities::FieldEntityLink {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::entities::FieldDeclarationLink> for ClaimValue {
    fn from(row: super::normalized::entities::FieldDeclarationLink) -> Self {
        Self::FieldDeclarationLinks(row)
    }
}
impl GraphAssertionRecord for super::normalized::entities::FieldDeclarationLink {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::entities::PublicExposureCandidate> for ClaimValue {
    fn from(row: super::normalized::entities::PublicExposureCandidate) -> Self {
        Self::PublicExposureCandidates(row)
    }
}
impl GraphAssertionRecord for super::normalized::entities::PublicExposureCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::events::CallAlternativeSource> for ClaimValue {
    fn from(row: super::normalized::events::CallAlternativeSource) -> Self {
        Self::NormalizedCallAlternativeSources(row)
    }
}
impl GraphAssertionRecord for super::normalized::events::CallAlternativeSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::callables::SignatureSlotEntity> for ClaimValue {
    fn from(row: super::normalized::callables::SignatureSlotEntity) -> Self {
        Self::SignatureSlotEntities(row)
    }
}
impl GraphAssertionRecord for super::normalized::callables::SignatureSlotEntity {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::dispatch::DispatchAssessment> for ClaimValue {
    fn from(row: super::normalized::dispatch::DispatchAssessment) -> Self {
        Self::NormalizedDispatchAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::dispatch::DispatchAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::local::TransferAlternative> for ClaimValue {
    fn from(row: super::transfer::local::TransferAlternative) -> Self {
        Self::LocalTransferAlternatives(row)
    }
}
impl GraphAssertionRecord for super::transfer::local::TransferAlternative {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::local::TransferSupport> for ClaimValue {
    fn from(row: super::transfer::local::TransferSupport) -> Self {
        Self::LocalTransferSupports(row)
    }
}
impl GraphAssertionRecord for super::transfer::local::TransferSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::model::TransferAlternative> for ClaimValue {
    fn from(row: super::transfer::model::TransferAlternative) -> Self {
        Self::ModelTransferAlternatives(row)
    }
}
impl GraphAssertionRecord for super::transfer::model::TransferAlternative {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::model::TransferSupport> for ClaimValue {
    fn from(row: super::transfer::model::TransferSupport) -> Self {
        Self::ModelTransferSupports(row)
    }
}
impl GraphAssertionRecord for super::transfer::model::TransferSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::summary::TransferAlternative> for ClaimValue {
    fn from(row: super::transfer::summary::TransferAlternative) -> Self {
        Self::SummaryTransferAlternatives(row)
    }
}
impl GraphAssertionRecord for super::transfer::summary::TransferAlternative {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::summary::TransferSupport> for ClaimValue {
    fn from(row: super::transfer::summary::TransferSupport) -> Self {
        Self::SummaryTransferSupports(row)
    }
}
impl GraphAssertionRecord for super::transfer::summary::TransferSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}

impl From<super::local_theory::TypeDomain> for ClaimValue {
    fn from(row: super::local_theory::TypeDomain) -> Self {
        Self::LocalTypeDomains(row)
    }
}
impl GraphAssertionRecord for super::local_theory::TypeDomain {
    const GRAPH_KIND: AssertionKind = AssertionKind::PredicateDomain;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::PredicateDomain,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}

impl From<super::assumptions_universe::AssumptionUniverseSupport> for ClaimValue {
    fn from(row: super::assumptions_universe::AssumptionUniverseSupport) -> Self {
        Self::AssumptionUniverseSupports(row)
    }
}
impl GraphAssertionRecord for super::assumptions_universe::AssumptionUniverseSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::embedding::text::TextAssessment> for ClaimValue {
    fn from(row: super::embedding::text::TextAssessment) -> Self {
        Self::AnalyticTextAssessments(row)
    }
}
impl GraphAssertionRecord for super::embedding::text::TextAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::seeds::ConfiguredSeedDecision> for ClaimValue {
    fn from(row: super::synthesis::seeds::ConfiguredSeedDecision) -> Self {
        Self::SynthesisConfiguredSeedDecisions(row)
    }
}
impl GraphAssertionRecord for super::synthesis::seeds::ConfiguredSeedDecision {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::synthesis::automatic::Decision> for ClaimValue {
    fn from(row: super::synthesis::automatic::Decision) -> Self {
        Self::SynthesisAutomaticSeedDecisions(row)
    }
}
impl GraphAssertionRecord for super::synthesis::automatic::Decision {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::Reach> for ClaimValue {
    fn from(row: super::structural::Reach) -> Self {
        Self::StructuralReaches(row)
    }
}
impl GraphAssertionRecord for super::structural::Reach {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::controls::UnfollowedArgument> for ClaimValue {
    fn from(row: super::structural::controls::UnfollowedArgument) -> Self {
        Self::StructuralUnfollowedArguments(row)
    }
}
impl GraphAssertionRecord for super::structural::controls::UnfollowedArgument {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::handoffs::ValueSource> for ClaimValue {
    fn from(row: super::structural::handoffs::ValueSource) -> Self {
        Self::StructuralHandoffValues(row)
    }
}
impl GraphAssertionRecord for super::structural::handoffs::ValueSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::structural::handoffs::Handoff> for ClaimValue {
    fn from(row: super::structural::handoffs::Handoff) -> Self {
        Self::StructuralHandoffOccurrences(row)
    }
}
impl GraphAssertionRecord for super::structural::handoffs::Handoff {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::execution::summary_consequences::ClaimConclusion> for ClaimValue {
    fn from(row: super::execution::summary_consequences::ClaimConclusion) -> Self {
        Self::SummaryBehavioralConclusions(row)
    }
}
impl GraphAssertionRecord for super::execution::summary_consequences::ClaimConclusion {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::symbolic_fields::SourceFieldClass> for ClaimValue {
    fn from(row: super::normalized::symbolic_fields::SourceFieldClass) -> Self {
        Self::SourceFieldClassAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::symbolic_fields::SourceFieldClass {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::symbolic_fields::SourceFieldStore> for ClaimValue {
    fn from(row: super::normalized::symbolic_fields::SourceFieldStore) -> Self {
        Self::SourceFieldStores(row)
    }
}
impl GraphAssertionRecord for super::normalized::symbolic_fields::SourceFieldStore {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::symbolic_fields::SourceFieldReader> for ClaimValue {
    fn from(row: super::normalized::symbolic_fields::SourceFieldReader) -> Self {
        Self::SourceFieldReaders(row)
    }
}
impl GraphAssertionRecord for super::normalized::symbolic_fields::SourceFieldReader {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::symbolic_fields::SourceFieldAssociation> for ClaimValue {
    fn from(row: super::normalized::symbolic_fields::SourceFieldAssociation) -> Self {
        Self::SourceFieldAssociations(row)
    }
}
impl GraphAssertionRecord for super::normalized::symbolic_fields::SourceFieldAssociation {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::events::EventAssessment> for ClaimValue {
    fn from(row: super::normalized::events::EventAssessment) -> Self {
        Self::CallEventAssessments(row)
    }
}
impl GraphAssertionRecord for super::normalized::events::EventAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}

impl From<super::calls::CallResolutionMember> for ClaimValue {
    fn from(row: super::calls::CallResolutionMember) -> Self {
        Self::CallResolutionMembers(row)
    }
}
impl GraphAssertionRecord for super::calls::CallResolutionMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::calls::SignatureEnumerationMember> for ClaimValue {
    fn from(row: super::calls::SignatureEnumerationMember) -> Self {
        Self::SignatureEnumerationMembers(row)
    }
}
impl GraphAssertionRecord for super::calls::SignatureEnumerationMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::analytics::CommunityLabelMember> for ClaimValue {
    fn from(row: super::analytics::CommunityLabelMember) -> Self {
        Self::AnalyticCommunityLabelMembers(row)
    }
}
impl GraphAssertionRecord for super::analytics::CommunityLabelMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::bindings::BindingSetMember> for ClaimValue {
    fn from(row: super::normalized::bindings::BindingSetMember) -> Self {
        Self::BindingSetMembers(row)
    }
}
impl GraphAssertionRecord for super::normalized::bindings::BindingSetMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::bindings::BindingSetCoverage> for ClaimValue {
    fn from(row: super::normalized::bindings::BindingSetCoverage) -> Self {
        Self::BindingSetCoverage(row)
    }
}
impl GraphAssertionRecord for super::normalized::bindings::BindingSetCoverage {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::dispatch::DispatchPremise> for ClaimValue {
    fn from(row: super::normalized::dispatch::DispatchPremise) -> Self {
        Self::NormalizedDispatchPremises(row)
    }
}
impl GraphAssertionRecord for super::normalized::dispatch::DispatchPremise {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::normalized::dispatch::DispatchEvidence> for ClaimValue {
    fn from(row: super::normalized::dispatch::DispatchEvidence) -> Self {
        Self::NormalizedDispatchEvidence(row)
    }
}
impl GraphAssertionRecord for super::normalized::dispatch::DispatchEvidence {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::summary::ControlInfluence> for ClaimValue {
    fn from(row: super::transfer::summary::ControlInfluence) -> Self {
        Self::SummaryControlInfluences(row)
    }
}
impl GraphAssertionRecord for super::transfer::summary::ControlInfluence {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::summary::ControlSupport> for ClaimValue {
    fn from(row: super::transfer::summary::ControlSupport) -> Self {
        Self::SummaryControlSupports(row)
    }
}
impl GraphAssertionRecord for super::transfer::summary::ControlSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::summary::Selection> for ClaimValue {
    fn from(row: super::transfer::summary::Selection) -> Self {
        Self::SummaryTransferSelections(row)
    }
}
impl GraphAssertionRecord for super::transfer::summary::Selection {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}

impl From<super::transfer::local::ControlSupport> for ClaimValue {
    fn from(row: super::transfer::local::ControlSupport) -> Self {
        Self::LocalControlSupports(row)
    }
}
impl GraphAssertionRecord for super::transfer::local::ControlSupport {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}
impl From<super::transfer::local::Selection> for ClaimValue {
    fn from(row: super::transfer::local::Selection) -> Self {
        Self::LocalTransferSelections(row)
    }
}
impl GraphAssertionRecord for super::transfer::local::Selection {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Claim(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::UniverseMember> for MembershipValue {
    fn from(row: super::analytics::UniverseMember) -> Self {
        Self::AnalyticUniverseMember(row)
    }
}
impl GraphAssertionRecord for super::analytics::UniverseMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::PublicSelector> for MembershipValue {
    fn from(row: super::analytics::PublicSelector) -> Self {
        Self::AnalyticPublicSelector(row)
    }
}
impl GraphAssertionRecord for super::analytics::PublicSelector {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::PartitionMember> for MembershipValue {
    fn from(row: super::analytics::PartitionMember) -> Self {
        Self::AnalyticPartitionMember(row)
    }
}
impl GraphAssertionRecord for super::analytics::PartitionMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::CommunityMember> for MembershipValue {
    fn from(row: super::analytics::CommunityMember) -> Self {
        Self::AnalyticCommunityMember(row)
    }
}
impl GraphAssertionRecord for super::analytics::CommunityMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::ConceptObject> for MembershipValue {
    fn from(row: super::analytics::ConceptObject) -> Self {
        Self::AnalyticConceptObject(row)
    }
}
impl GraphAssertionRecord for super::analytics::ConceptObject {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::ConceptExtent> for MembershipValue {
    fn from(row: super::analytics::ConceptExtent) -> Self {
        Self::AnalyticConceptExtent(row)
    }
}
impl GraphAssertionRecord for super::analytics::ConceptExtent {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::ConceptIntent> for MembershipValue {
    fn from(row: super::analytics::ConceptIntent) -> Self {
        Self::AnalyticConceptIntent(row)
    }
}
impl GraphAssertionRecord for super::analytics::ConceptIntent {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::ImplicationMember> for MembershipValue {
    fn from(row: super::analytics::ImplicationMember) -> Self {
        Self::AnalyticImplicationMember(row)
    }
}
impl GraphAssertionRecord for super::analytics::ImplicationMember {
    const GRAPH_KIND: AssertionKind = AssertionKind::StructuralMembership;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::StructuralMembership,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Membership(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::LayerPair> for AnalysisValue {
    fn from(row: super::analytics::LayerPair) -> Self {
        Self::AnalyticLayerPair(row)
    }
}
impl GraphAssertionRecord for super::analytics::LayerPair {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::CombinedPair> for AnalysisValue {
    fn from(row: super::analytics::CombinedPair) -> Self {
        Self::AnalyticCombinedPair(row)
    }
}
impl GraphAssertionRecord for super::analytics::CombinedPair {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::CommunityRun> for AnalysisValue {
    fn from(row: super::analytics::CommunityRun) -> Self {
        Self::AnalyticCommunityRun(row)
    }
}
impl GraphAssertionRecord for super::analytics::CommunityRun {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::CommunityProfile> for AnalysisValue {
    fn from(row: super::analytics::CommunityProfile) -> Self {
        Self::AnalyticCommunityProfile(row)
    }
}
impl GraphAssertionRecord for super::analytics::CommunityProfile {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::VectorSelection> for AnalysisValue {
    fn from(row: super::analytics::VectorSelection) -> Self {
        Self::AnalyticVectorSelection(row)
    }
}
impl GraphAssertionRecord for super::analytics::VectorSelection {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::CommunityLabelAssessment> for AnalysisValue {
    fn from(row: super::analytics::CommunityLabelAssessment) -> Self {
        Self::AnalyticCommunityLabelAssessment(row)
    }
}
impl GraphAssertionRecord for super::analytics::CommunityLabelAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::LayerResult> for AnalysisValue {
    fn from(row: super::analytics::LayerResult) -> Self {
        Self::AnalyticLayerResult(row)
    }
}
impl GraphAssertionRecord for super::analytics::LayerResult {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::LayerNeighbour> for AnalysisValue {
    fn from(row: super::analytics::LayerNeighbour) -> Self {
        Self::AnalyticLayerNeighbour(row)
    }
}
impl GraphAssertionRecord for super::analytics::LayerNeighbour {
    const GRAPH_KIND: AssertionKind = AssertionKind::DerivedConclusion;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::DerivedConclusion,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Analysis(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::GraphArc> for ProvenanceValue {
    fn from(row: super::analytics::GraphArc) -> Self {
        Self::AnalyticGraphArc(row)
    }
}
impl GraphAssertionRecord for super::analytics::GraphArc {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::PairSource> for ProvenanceValue {
    fn from(row: super::analytics::PairSource) -> Self {
        Self::AnalyticPairSource(row)
    }
}
impl GraphAssertionRecord for super::analytics::PairSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::PairContribution> for ProvenanceValue {
    fn from(row: super::analytics::PairContribution) -> Self {
        Self::AnalyticPairContribution(row)
    }
}
impl GraphAssertionRecord for super::analytics::PairContribution {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::IncidenceSource> for ProvenanceValue {
    fn from(row: super::analytics::IncidenceSource) -> Self {
        Self::AnalyticIncidenceSource(row)
    }
}
impl GraphAssertionRecord for super::analytics::IncidenceSource {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::Incidence> for ProvenanceValue {
    fn from(row: super::analytics::Incidence) -> Self {
        Self::AnalyticIncidence(row)
    }
}
impl GraphAssertionRecord for super::analytics::Incidence {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::TypeMetadataSelection> for ProvenanceValue {
    fn from(row: super::analytics::TypeMetadataSelection) -> Self {
        Self::AnalyticTypeMetadataSelection(row)
    }
}
impl GraphAssertionRecord for super::analytics::TypeMetadataSelection {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::analytics::DecoratorSelection> for ProvenanceValue {
    fn from(row: super::analytics::DecoratorSelection) -> Self {
        Self::AnalyticDecoratorSelection(row)
    }
}
impl GraphAssertionRecord for super::analytics::DecoratorSelection {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

// Exact usage and call-policy support for selected analytic CoUse provenance.
impl From<super::structural::UsageSite> for ProvenanceValue {
    fn from(row: super::structural::UsageSite) -> Self {
        Self::StructuralUsageSite(row)
    }
}
impl GraphAssertionRecord for super::structural::UsageSite {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::structural::UsageEvidence> for ProvenanceValue {
    fn from(row: super::structural::UsageEvidence) -> Self {
        Self::StructuralUsageEvidence(row)
    }
}
impl GraphAssertionRecord for super::structural::UsageEvidence {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::normalized::events::CallPolicyAssessment> for ProvenanceValue {
    fn from(row: super::normalized::events::CallPolicyAssessment) -> Self {
        Self::CallPolicyAssessment(row)
    }
}
impl GraphAssertionRecord for super::normalized::events::CallPolicyAssessment {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

impl From<super::normalized::events::CallPolicyAdmission> for ProvenanceValue {
    fn from(row: super::normalized::events::CallPolicyAdmission) -> Self {
        Self::CallPolicyAdmission(row)
    }
}
impl GraphAssertionRecord for super::normalized::events::CallPolicyAdmission {
    const GRAPH_KIND: AssertionKind = AssertionKind::EvidenceAssociation;
    fn graph_payload(row: Self) -> Assertion {
        let source = SemanticKey::of(row.id());
        Assertion {
            source: Some(source),
            kind: AssertionKind::EvidenceAssociation,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}

macro_rules! retrieval_assertions {
    ($($variant:ident:$ty:ty => $kind:ident),* $(,)?)=>{$(
        impl GraphAssertionRecord for $ty {
            const GRAPH_KIND:AssertionKind=AssertionKind::$kind;
            fn graph_payload(row:Self)->Assertion {
                Assertion { source:Some(SemanticKey::of(row.id())),kind:Self::GRAPH_KIND,
                    participants:vec![],qualification:Qualification::Payload,run:None,evidence:vec![],
                    value:AssertionValue::Analysis(AnalysisValue::$variant(row)),derivation:None }
            }
        }
    )*};
}
retrieval_assertions! {
    RetrievalEmbeddingUse:super::retrieval::consumption::RetrievalEmbeddingUse=>EmbeddingWitness,
    RetrievalUnitRoot:super::retrieval::UnitRoot=>EvidenceAssociation,
}

// Exact native-source characterization and diagnostic correspondence survive admission.
macro_rules! catalog_source_assertions {
 ($($variant:ident:$ty:ty),* $(,)?)=>{$(
  impl From<$ty> for ProvenanceValue{fn from(row:$ty)->Self{Self::$variant(row)}}
  impl GraphAssertionRecord for $ty{
   const GRAPH_KIND:AssertionKind=AssertionKind::EvidenceAssociation;
   fn graph_payload(row:Self)->Assertion{
    let qualification=row.references().into_iter().find(|r|r.target==super::assertion::AssertionQualification::NAME)
      .map(|r|Qualification::Ref(entity_key(EntityKind::Qualification,r.target,&r.key))).unwrap_or(Qualification::Payload);
    Assertion{source:Some(SemanticKey::of(row.id())),kind:Self::GRAPH_KIND,participants:vec![],qualification,
      run:None,evidence:vec![],value:AssertionValue::Provenance(row.into()),derivation:None}
   }
  }
 )*};
}
catalog_source_assertions! {
 NormalizedCallEventSource:super::normalized::events::CallEventSource,
 NormalizedCallEventSourceEvidence:super::normalized::events::CallEventSourceEvidence,
 CatalogSourceCharacterization:super::catalog::evidence::SourceCharacterization,
 CatalogSourceCharacterizationScenario:super::catalog::evidence::SourceCharacterizationScenario,
 CatalogSourceUsage:super::catalog::evidence::SourceUsage,
 CatalogDiagnosticUseAssessment:super::catalog::evidence::DiagnosticUseAssessment,
 CatalogDiagnosticUseLink:super::catalog::evidence::DiagnosticUseLink,
 CatalogDiagnosticUsePath:super::catalog::evidence::DiagnosticUsePath,
 CatalogDiagnosticUseTarget:super::catalog::evidence::DiagnosticUseTarget,
}

// Finite consumer declarations and owned membership are part of the transported domain.
// Digest-only parents cannot reconstruct their local universe without these exact children.
macro_rules! retained_consumer_assertions {
 ($($variant:ident:$ty:ty=>$payload:ident,$family:ident,$kind:ident),* $(,)?)=>{$(
  impl From<$ty> for $payload{fn from(row:$ty)->Self{Self::$variant(row)}}
  impl GraphAssertionRecord for $ty{
   const GRAPH_KIND:AssertionKind=AssertionKind::$kind;
   fn graph_payload(row:Self)->Assertion{
    let qualification=row.references().into_iter().find(|r|r.target==super::assertion::AssertionQualification::NAME)
      .map(|r|Qualification::Ref(entity_key(EntityKind::Qualification,r.target,&r.key))).unwrap_or(Qualification::Payload);
    Assertion{source:Some(SemanticKey::of(row.id())),kind:Self::GRAPH_KIND,participants:vec![],qualification,
      run:None,evidence:vec![],value:AssertionValue::$family(row.into()),derivation:None}
   }
  }
 )*};
}
retained_consumer_assertions! {
 SelectionContext:super::selection::Context=>AnalysisValue,Analysis,PredicateDomain,
 SelectionWitness:super::selection::Witness=>AnalysisValue,Analysis,EvidenceAssociation,
 SelectionDomainContext:super::selection::DomainContext=>MembershipValue,Membership,StructuralMembership,
 SelectionDomainClosure:super::selection::DomainClosure=>MembershipValue,Membership,StructuralMembership,
 SelectionDomainEvidence:super::selection::DomainEvidence=>MembershipValue,Membership,StructuralMembership,
 SelectionFieldLocationLink:super::catalog::evidence::FieldLocationLink=>ProvenanceValue,Provenance,EvidenceAssociation,
 SelectionConstructorCandidateLink:super::catalog::evidence::ConstructorCandidateLink=>ProvenanceValue,Provenance,EvidenceAssociation,
 SelectionFieldAccessAssessment:super::catalog::evidence::FieldAccessAssessment=>ProvenanceValue,Provenance,EvidenceAssociation,
 SelectionFieldLocation:super::local_fields::FieldLocation=>ProvenanceValue,Provenance,DerivedConclusion,
 SelectionFieldLocationCandidate:super::local_fields::FieldLocationCandidate=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 SignatureSlotType:super::normalized::callables::SignatureSlotType=>MembershipValue,Membership,InvocationSignature,
 SignatureReturnType:super::normalized::callables::SignatureReturnType=>MembershipValue,Membership,InvocationSignature,
 SelectionSourceFieldLink:super::catalog::evidence::SourceFieldLink=>ProvenanceValue,Provenance,EvidenceAssociation,
 SelectionReferenceBindingCharacterization:super::normalized::links::ReferenceBindingCharacterization=>ProvenanceValue,Provenance,DeclarationCorrespondence,
}

retained_consumer_assertions! {
 RetainedReleaseDeployment:super::catalog::evidence::ReleaseDeployment=>MembershipValue,Membership,DeploymentAssociation,
 RetainedScenarioSpan:super::catalog::evidence::ScenarioSpan=>MembershipValue,Membership,ScenarioAssociation,
 RetainedGuardSubstitution:super::conditions::stability::GuardSubstitution=>ProvenanceValue,Provenance,ValueTransfer,
 NativeDiagnosticAnnotation:super::diagnostics::DiagnosticAnnotation=>ProvenanceValue,Provenance,NativeObservation,
 NativeDiagnosticSubject:super::diagnostics::DiagnosticSubject=>ProvenanceValue,Provenance,NativeObservation,
 RetainedSummaryCaptureContribution:super::execution::summary_capture::SummaryCaptureContribution=>ProvenanceValue,Provenance,SummaryTransfer,
 NativeFlowUseInventoryMember:super::flow_inventory::FlowUseInventoryMember=>MembershipValue,Membership,StructuralMembership,
 RetainedEffectiveCallableEvidence:super::normalized::callables::EffectiveCallableEvidence=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedEffectiveCallablePremise:super::normalized::callables::EffectiveCallablePremise=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedEffectiveDecoratorMember:super::normalized::callables::EffectiveDecoratorMember=>MembershipValue,Membership,StructuralOrder,
 RetainedSymbolEntityEvidence:super::normalized::entities::SymbolEntityEvidence=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedSymbolEntityPremise:super::normalized::entities::SymbolEntityPremise=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedCallAlternativeEvidence:super::normalized::events::CallAlternativeEvidence=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedCallEventResolution:super::normalized::events::CallEventResolution=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedCallEventResolutionEvidence:super::normalized::events::CallEventResolutionEvidence=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedEventPhaseTarget:super::normalized::events::EventPhaseTarget=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedFlowCallEventLink:super::normalized::events::FlowCallEventLink=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedDeclarationNativeCharacterization:super::normalized::links::DeclarationNativeCharacterization=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedImportModuleAssessment:super::normalized::links::ImportModuleAssessment=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedImportModuleCandidate:super::normalized::links::ImportModuleCandidate=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedMentionSymbolCandidate:super::normalized::links::MentionSymbolCandidate=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedPlaceEntityLink:super::normalized::links::PlaceEntityLink=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedTestOperandCoverage:super::normalized::links::TestOperandCoverage=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedTypeBinderAssessment:super::normalized::links::TypeBinderAssessment=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedTypeBinderCandidate:super::normalized::links::TypeBinderCandidate=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedTypeBinderPremise:super::normalized::links::TypeBinderPremise=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedTypeEntityLink:super::normalized::links::TypeEntityLink=>ProvenanceValue,Provenance,DeclarationCorrespondence,
 RetainedOverloadVariantAssessment:super::normalized::overload_association::OverloadVariantAssessment=>ProvenanceValue,Provenance,InvocationSignature,
 RetainedOverloadVariantCandidate:super::normalized::overload_association::OverloadVariantCandidate=>ProvenanceValue,Provenance,InvocationSignature,
 RetainedReceiverEvidence:super::normalized::receiver::ReceiverEvidence=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedReceiverPremise:super::normalized::receiver::ReceiverPremise=>ProvenanceValue,Provenance,EvidenceAssociation,
 RetainedAssertionSource:super::synthesis::assertions::AssertionSource=>ProvenanceValue,Provenance,SynthesisClaim,
 RetainedProgrammaticAssertionSupport:super::synthesis::assertions::ProgrammaticAssertionSupport=>ProvenanceValue,Provenance,SynthesisClaim,
 RetainedBriefAssertion:super::synthesis::briefs::BriefAssertion=>MembershipValue,Membership,SynthesisClaim,
 RetainedBriefDocument:super::synthesis::briefs::BriefDocument=>MembershipValue,Membership,DocumentaryEvidence,
 RetainedBriefSource:super::synthesis::briefs::BriefSource=>MembershipValue,Membership,DocumentaryEvidence,
}

// Reachability and narrowing qualify distinct candidate properties. Neither is a
// candidate-wide qualifier; both remain exact field-labelled semantic references.
impl From<super::flow_inventory::FlowUseCandidate> for ProvenanceValue {
    fn from(row: super::flow_inventory::FlowUseCandidate) -> Self {
        Self::NativeFlowUseCandidate(row)
    }
}
impl GraphAssertionRecord for super::flow_inventory::FlowUseCandidate {
    const GRAPH_KIND: AssertionKind = AssertionKind::NativeObservation;
    fn graph_payload(row: Self) -> Assertion {
        Assertion {
            source: Some(SemanticKey::of(row.id())),
            kind: Self::GRAPH_KIND,
            participants: vec![],
            qualification: Qualification::Payload,
            run: None,
            evidence: vec![],
            value: AssertionValue::Provenance(row.into()),
            derivation: None,
        }
    }
}
