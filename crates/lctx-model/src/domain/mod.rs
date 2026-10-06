//! Typed domain authority. Physical layouts are lowerings of these definitions (ADR-0085).
pub mod admission;
pub mod analysis;
pub mod analytics;
pub mod artifact;
pub mod assertion;
pub mod assumptions;
pub mod assumptions_universe;
pub mod atom_decision;
pub mod attachment;
pub mod attribution;
pub mod batching;
pub mod calls;
pub mod captures;
pub mod catalog;
pub mod charged;
pub mod class_metadata;
pub mod composition;
pub mod conditions;
pub mod declarations;
pub mod deployment;
pub mod derivation;
pub mod diagnostics;
pub mod documents;
pub mod embedding;
pub mod execution;
pub(crate) mod finite;
pub mod flow;
pub mod flow_capture;
pub mod flow_inventory;
pub(crate) mod identity;
pub mod input;
pub mod lexical;
pub mod local_fields;
pub mod local_semantics;
pub mod local_symbolic;
pub mod local_theory;
pub(crate) mod model;
pub mod models;
pub mod normalized;
pub mod obligation;
pub mod occurrence_owner;
pub(crate) mod ownership;
pub mod place_composition;
pub mod projection;
pub mod protocols;
pub(crate) mod record;
pub mod resources;
pub mod retrieval;
pub mod ruff;
pub mod selection;
pub mod source;
pub mod stages;
pub mod structural;
pub mod symbols;
pub mod syntax;
pub mod synthesis;
pub mod transfer;
pub mod types;
pub mod validation;
pub mod value;

pub use finite::FiniteF64;
pub use identity::{ArmId, ContentHash, ContentHasher, EvidenceBytes, Id, Key, KeySink, Utf8Text};
pub use model::{
    Invariant, InvariantCheck, InvariantPurpose, PublicationCheck, PublicationInvariant, Relation,
    RelationContent, SEMANTIC_POLICY_REVISION, ValidatedModel, ValidationDefinitions,
    ValidationIdentity, ValidationInput, ValidationKind, implementation_digest,
};
pub use record::{
    Arm, ArmField, Batch, Codebook, Field, FieldValue, FlatValue, HeapSize, Record, Scalar,
    SemanticReference, Sum, SumRecord,
};

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// A recognized consumer cause. Its public meaning never contains internal error text.
    #[error("{0}")]
    Serving(serving::FailureKind),
    #[error(
        "{owner} memory reservation refused: requested {requested} bytes with {used}/{limit} reserved"
    )]
    Resource {
        owner: &'static str,
        requested: usize,
        used: usize,
        limit: usize,
    },
    #[error("invalid model: {0}")]
    Invalid(String),
    /// A declared operational ceiling refused an input: a row, read or transfer larger than its
    /// bound. Producers turn it into `ResourceRefused` coverage; it is never an invalid model.
    #[error("{owner} {limit} limit refused {observed} (bound {bound})")]
    Limit {
        owner: &'static str,
        limit: &'static str,
        observed: usize,
        bound: usize,
    },
    /// A request outside the generation's frontier, or a schedule or coverage that cannot be
    /// admitted to it.
    #[error("frontier: {0}")]
    Frontier(String),
    #[error("wrong schema for {0}")]
    Schema(&'static str),
    #[error("identity does not match semantic key for {0}")]
    Identity(&'static str),
    #[error("equal semantic key has conflicting payload in {0}")]
    Conflict(&'static str),
    #[error("codec: {0}")]
    Codec(String),
    /// A store or capture effect failed for a reason outside the model (P0 exit F07). The class
    /// is what a caller acts on: a transport loss or an unconfirmed commit interrupts the attempt,
    /// a refusal or conflict is reported, never retried as if it were a codec defect.
    #[error("{class:?} infrastructure failure: {detail}")]
    Infrastructure {
        class: Infrastructure,
        detail: String,
    },
}
impl ModelError {
    pub fn codec(error: impl std::fmt::Display) -> Self {
        Self::Codec(error.to_string())
    }
    pub fn infrastructure(class: Infrastructure, error: impl std::fmt::Display) -> Self {
        Self::Infrastructure {
            class,
            detail: error.to_string(),
        }
    }
}
/// The class of an infrastructure failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Infrastructure {
    /// The connection or transport was lost; nothing after the last confirmed step happened.
    Transport,
    /// A commit, rollback or COPY abort was sent without confirmation; the outcome is unknown.
    Unconfirmed,
    /// The server refused a statement: a privilege or protocol refusal outside the content.
    Refused,
    /// A lock or statement timeout, a deadlock or a serialization conflict: another holder was
    /// in the way.
    Contention,
    /// A generation's state or lock excluded the operation.
    State,
    /// The stored model, lowering or registry differs from this binary's.
    Contract,
    /// Local input could not be read.
    Io,
    /// The server exhausted a resource: disk, memory or a program limit (SQLSTATE 53, 54).
    Exhausted,
}

#[doc(hidden)]
pub mod __private {
    pub use arrow_array::RecordBatch;
    pub use serde;
    pub use serde_arrow;
}

/// Declaration controls are paired: a reference collection must be a relationship record.
///
/// ```compile_fail
/// use lctx_model::{Domain, domain::{Id, input::Package}};
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "bad_collection")]
/// struct Collection { #[model(key)] name: String, members: Vec<Id<Package>> }
/// ```
///
/// ```
/// use lctx_model::{Domain, domain::{Id, input::Package}};
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "package_memberships")]
/// struct Membership { #[model(key)] owner: Id<Package>, #[model(key)] member: Id<Package> }
/// ```
///
/// ```compile_fail
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "missing_key")]
/// struct MissingKey { name: String }
/// ```
///
/// ```
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "has_key")]
/// struct HasKey { #[model(key)] name: String }
/// ```
///
/// ```compile_fail
/// use lctx_model::DomainCode;
/// #[derive(DomainCode)]
/// enum ImplicitCode { One, Two }
/// ```
///
/// ```
/// use lctx_model::DomainCode;
/// #[derive(DomainCode)]
/// #[repr(i16)]
/// enum ExplicitCode { One = 0, Two = 1 }
/// ```
///
/// ```compile_fail
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "nested_optional")]
/// struct Nested { #[model(key)] name: String, value: Option<Option<String>> }
/// ```
///
/// ```
/// use lctx_model::Domain;
/// #[derive(Debug, Clone, PartialEq, Eq, Domain)]
/// #[model(name = "ordinary_optional")]
/// struct Ordinary { #[model(key)] name: String, value: Option<String> }
/// ```
pub mod declaration_controls {}

/// The sole production relation membership manifest; physical inventories are derived. It is the
/// facts relations plus the analysis relations derived from them.
pub fn normalized_relations() -> Vec<Relation> {
    let mut relations = facts_relations();
    relations.extend(normalized::relations());
    relations
}
pub fn model() -> Result<ValidatedModel, ModelError> {
    ValidatedModel::declared(catalog_frontier_relations())
}
/// Cumulative analysis ownership. Activation still requires the complete producer envelope.
pub fn analysis_frontier_relations() -> Vec<Relation> {
    let mut relations = normalized_relations();
    relations.extend(pre_catalog_analysis_relations());
    relations
}
/// Cumulative catalog ownership, including all lower analysis relations.
pub fn catalog_frontier_relations() -> Vec<Relation> {
    let mut relations = normalized_relations();
    relations.extend(analysis_relations());
    relations
}
/// Relations later layers derive from facts: transfers, control selections, stability witnesses,
/// guard substitutions and call compositions. A facts generation never writes them.
fn pre_catalog_analysis_relations() -> Vec<Relation> {
    let mut relations = vec![
        Relation::of::<transfer::local::ControlInfluence>(),
        Relation::of::<transfer::local::ControlSupport>(),
        Relation::of::<transfer::local::Selection>(),
        Relation::of::<conditions::entry::EntryValueWitness>(),
        Relation::of::<conditions::entry::EntryAccessSource>(),
        Relation::of::<conditions::stability::StabilityWitness>(),
        Relation::of::<conditions::stability::GuardSubstitution>(),
        Relation::of::<transfer::summary::ControlInfluence>(),
        Relation::of::<transfer::summary::ControlSupport>(),
        Relation::of::<transfer::summary::Selection>(),
        Relation::of::<transfer::summary::SummaryPremise>(),
        Relation::of::<transfer::summary::SummaryWitness>(),
        Relation::of::<transfer::summary::SummaryContribution>(),
    ];
    relations.extend(transfer::local::relations());
    relations.extend(transfer::model::relations());
    relations.extend(transfer::summary::relations());
    relations.extend(analysis::pre_catalog_relations());
    relations.extend(catalog::relations());
    relations.extend(structural::relations());
    relations.extend(analytics::relations());
    relations.extend(embedding::relations());
    relations.extend(execution::relations());
    relations.extend(local_semantics::relations());
    relations.extend(local_theory::relations());
    relations.extend(local_fields::relations());
    relations
}
/// All upper relations for the complete declared model and conformance consumers.
pub fn analysis_relations() -> Vec<Relation> {
    let mut relations = pre_catalog_analysis_relations();
    relations.extend(analysis::catalog_publication_relations());
    relations.extend(selection::relations());
    relations.extend(synthesis::relations());
    relations.extend(retrieval::relations());
    relations
}
/// The canonical typed facts inventory, in publication relation order. Consumers supply a
/// callback accepting a comma-separated list of record types; lowerings derive from this list.
#[macro_export]
macro_rules! facts_records {
    ($apply:ident) => {
        $apply!(
            $crate::domain::assumptions::Assumption,
            $crate::domain::assumptions::AssumptionUniverse,
            $crate::domain::assumptions::AssumptionSet,
            $crate::domain::assumptions::AssumptionSetMember,
            $crate::domain::class_metadata::RecordOptions,
            $crate::domain::class_metadata::RecordTransformDefaults,
            $crate::domain::class_metadata::RecordTransformFieldSpecifier,
            $crate::domain::class_metadata::ClassMetadataObservation,
            $crate::domain::class_metadata::ClassMetadataSupport,
            $crate::domain::class_metadata::ClassMemberObservation,
            $crate::domain::class_metadata::ClassMemberSupport,
            $crate::domain::deployment::ReportCollection,
            $crate::domain::deployment::ReportValue,
            $crate::domain::deployment::ReportEntry,
            $crate::domain::deployment::ReportedEnvironment,
            $crate::domain::deployment::TaskReport,
            $crate::domain::deployment::TaskReportObservation,
            $crate::domain::deployment::TaskReportSupport,
            $crate::domain::deployment::DeploymentObservation,
            $crate::domain::deployment::DeploymentSupport,
            $crate::domain::types::TypeVariable,
            $crate::domain::captures::CaptureObservation,
            $crate::domain::captures::CaptureSupport,
            $crate::domain::protocols::NativeExitObservation,
            $crate::domain::protocols::NativeExitSupport,
            $crate::domain::protocols::NativeTerminalObservation,
            $crate::domain::protocols::NativeTerminalSupport,
            $crate::domain::protocols::NativeExitDiagnostic,
            $crate::domain::protocols::NativeExitDiagnosticSupport,
            $crate::domain::types::TypeTerm,
            $crate::domain::types::TypeSequence,
            $crate::domain::types::TypeSequenceMember,
            $crate::domain::types::CallableParameterList,
            $crate::domain::types::CallableParameter,
            $crate::domain::types::TypedDictFieldList,
            $crate::domain::types::TypedDictField,
            $crate::domain::types::FunctionBodyObservation,
            $crate::domain::types::FunctionBodySupport,
            $crate::domain::types::RecordFieldObservation,
            $crate::domain::types::RecordFieldSupport,
            $crate::domain::types::GenericSpecializationObservation,
            $crate::domain::types::GenericSpecializationSupport,
            $crate::domain::types::NativeSignatureObservation,
            $crate::domain::types::NativeSignatureSupport,
            $crate::domain::types::NativeOverloadObservation,
            $crate::domain::types::NativeOverloadSupport,
            $crate::domain::types::NativeOverloadCandidate,
            $crate::domain::types::NativeOverloadCandidateSupport,
            $crate::domain::types::SignatureTypeSubject,
            $crate::domain::types::SignatureTypeObservation,
            $crate::domain::types::SignatureTypeSupport,
            $crate::domain::types::TypeObservation,
            $crate::domain::types::TypeQueryObservation,
            $crate::domain::types::TypeQuerySupport,
            $crate::domain::types::TypeSupport,
            $crate::domain::types::TypePresentation,
            $crate::domain::types::TypePresentationSupport,
            $crate::domain::types::TypeVariableRestriction,
            $crate::domain::types::TypeRestrictionSupport,
            $crate::domain::flow::FlowUse,
            $crate::domain::flow::FlowDefinition,
            $crate::domain::flow::ReachingDefinition,
            $crate::domain::flow::FlowUseObservation,
            $crate::domain::flow::FlowUseSupport,
            $crate::domain::flow::FlowDefinitionObservation,
            $crate::domain::flow::FlowDefinitionSupport,
            $crate::domain::flow::FlowReachingObservation,
            $crate::domain::flow::FlowReachingSupport,
            $crate::domain::flow::FlowNarrowingObservation,
            $crate::domain::flow::FlowNarrowingSupport,
            $crate::domain::flow::FlowSourceViewObservation,
            $crate::domain::flow::FlowSourceViewSupport,
            $crate::domain::flow_inventory::FlowUseInventoryObservation,
            $crate::domain::flow_inventory::FlowUseInventorySupport,
            $crate::domain::flow_inventory::FlowUseCandidate,
            $crate::domain::flow_inventory::FlowUseInventoryMember,
            $crate::domain::flow_capture::FlowCaptureTarget,
            $crate::domain::flow_capture::FlowCaptureInventory,
            $crate::domain::flow_capture::FlowCaptureCandidate,
            $crate::domain::flow_capture::FlowCaptureTimingObservation,
            $crate::domain::flow_capture::FlowCaptureTimingSupport,
            $crate::domain::flow::FlowValueObservation,
            $crate::domain::flow::FlowValueSupport,
            $crate::domain::flow::FlowRegionObservation,
            $crate::domain::flow::FlowRegionSupport,
            $crate::domain::flow::FlowTestObservation,
            $crate::domain::flow::FlowTestSupport,
            $crate::domain::flow::FlowTestLeafObservation,
            $crate::domain::flow::FlowTestLeafSupport,
            $crate::domain::flow::FlowAttributeLoadObservation,
            $crate::domain::flow::FlowAttributeLoadSupport,
            $crate::domain::flow::FlowCallPath,
            $crate::domain::flow::FlowCallStep,
            $crate::domain::flow::FlowValuePathObservation,
            $crate::domain::flow::FlowValuePathSupport,
            $crate::domain::documents::DocumentNode,
            $crate::domain::documents::DocumentAttributeValue,
            $crate::domain::documents::DocumentObservation,
            $crate::domain::documents::DocumentSupport,
            $crate::domain::documents::PassageObservation,
            $crate::domain::documents::PassageSupport,
            $crate::domain::documents::CodeBlockObservation,
            $crate::domain::documents::CodeBlockSupport,
            $crate::domain::documents::DocumentLinkObservation,
            $crate::domain::documents::DocumentLinkSupport,
            $crate::domain::documents::DocumentMentionObservation,
            $crate::domain::documents::DocumentMentionSupport,
            $crate::domain::documents::DocumentComponentObservation,
            $crate::domain::documents::DocumentComponentSupport,
            $crate::domain::documents::DocumentAttributeObservation,
            $crate::domain::documents::DocumentAttributeSupport,
            $crate::domain::lexical::LexicalScope,
            $crate::domain::lexical::BindingEvent,
            $crate::domain::lexical::LexicalTarget,
            $crate::domain::lexical::LexicalScopeObservation,
            $crate::domain::lexical::LexicalScopeSupport,
            $crate::domain::lexical::BindingObservation,
            $crate::domain::lexical::BindingSupport,
            $crate::domain::lexical::ReferenceObservation,
            $crate::domain::lexical::ReferenceSupport,
            $crate::domain::lexical::LexicalResolution,
            $crate::domain::lexical::LexicalResolutionSupport,
            $crate::domain::calls::ProviderModule,
            $crate::domain::calls::ProviderCallable,
            $crate::domain::calls::ProviderSymbol,
            $crate::domain::calls::ParameterShape,
            $crate::domain::calls::Signature,
            $crate::domain::calls::SignatureParameter,
            $crate::domain::calls::SignatureSupport,
            $crate::domain::calls::SignatureEnumerationObservation,
            $crate::domain::calls::SignatureEnumerationMember,
            $crate::domain::calls::SignatureEnumerationSupport,
            $crate::domain::calls::CallChannel,
            $crate::domain::calls::CallDestination,
            $crate::domain::calls::Receiver,
            $crate::domain::calls::CallOrigin,
            $crate::domain::calls::CallOriginStep,
            $crate::domain::calls::ProviderCallSite,
            $crate::domain::calls::ProviderCallSiteSupport,
            $crate::domain::calls::CallTarget,
            $crate::domain::calls::CallTargetSupport,
            $crate::domain::calls::CallResolution,
            $crate::domain::calls::CallResolutionMember,
            $crate::domain::calls::CallResolutionSupport,
            $crate::domain::calls::CallSyntax,
            $crate::domain::calls::CallSyntaxSupport,
            $crate::domain::calls::CallArgument,
            $crate::domain::diagnostics::RuffDiagnosticObservation,
            $crate::domain::diagnostics::RuffDiagnosticSupport,
            $crate::domain::diagnostics::PyreflyDiagnosticObservation,
            $crate::domain::diagnostics::PyreflyDiagnosticSupport,
            $crate::domain::diagnostics::DiagnosticSubject,
            $crate::domain::diagnostics::DiagnosticAnnotation,
            $crate::domain::diagnostics::NativeParameterDefinitionObservation,
            $crate::domain::diagnostics::NativeParameterDefinitionSupport,
            $crate::domain::ruff::RuffContextObservation,
            $crate::domain::ruff::RuffContextSupport,
            $crate::domain::ruff::RuffBindingObservation,
            $crate::domain::ruff::RuffBindingSupport,
            $crate::domain::ruff::RuffDefinitionObservation,
            $crate::domain::ruff::RuffDefinitionSupport,
            $crate::domain::syntax::SyntaxPlacement,
            $crate::domain::syntax::SyntaxPlacementSupport,
            $crate::domain::syntax::SyntaxDetail,
            $crate::domain::syntax::SyntaxDetailObservation,
            $crate::domain::syntax::SyntaxDetailSupport,
            $crate::domain::syntax::DeclarationObservation,
            $crate::domain::syntax::DeclarationSupport,
            $crate::domain::syntax::DeclarationDecorator,
            $crate::domain::syntax::DeclarationDecoratorSupport,
            $crate::domain::syntax::ImportAliasObservation,
            $crate::domain::syntax::ImportAliasSupport,
            $crate::domain::syntax::DunderAllObservation,
            $crate::domain::syntax::DunderAllSupport,
            $crate::domain::syntax::ParameterSyntaxObservation,
            $crate::domain::syntax::ParameterSyntaxSupport,
            $crate::domain::syntax::ClassFieldSyntaxObservation,
            $crate::domain::syntax::ClassFieldSyntaxSupport,
            $crate::domain::syntax::SubjectBoundary,
            $crate::domain::syntax::AttachmentOutcome,
            $crate::domain::syntax::AttachmentCandidate,
            $crate::domain::declarations::SymbolDeclaration,
            $crate::domain::declarations::SymbolDeclarationSupport,
            $crate::domain::declarations::ParameterDeclaration,
            $crate::domain::declarations::ParameterDeclarationSupport,
            $crate::domain::symbols::SymbolSequence,
            $crate::domain::symbols::SymbolSequenceMember,
            $crate::domain::symbols::SymbolObservation,
            $crate::domain::symbols::SymbolSupport,
            $crate::domain::symbols::FunctionTraitObservation,
            $crate::domain::symbols::FunctionTraitSupport,
            $crate::domain::symbols::ClassTraitObservation,
            $crate::domain::symbols::ClassTraitSupport,
            $crate::domain::symbols::ClassAncestryObservation,
            $crate::domain::symbols::ClassAncestrySupport,
            $crate::domain::symbols::ParameterAnnotationObservation,
            $crate::domain::symbols::ParameterAnnotationSupport,
            $crate::domain::symbols::ExportOrigin,
            $crate::domain::symbols::PublicNameObservation,
            $crate::domain::symbols::ExportEnumerationObservation,
            $crate::domain::symbols::ExportEnumerationSupport,
            $crate::domain::symbols::PublicNameSupport,
            $crate::domain::symbols::ParameterDocObservation,
            $crate::domain::symbols::ParameterDocSupport,
            $crate::domain::symbols::ModuleResolutionObservation,
            $crate::domain::symbols::ModuleResolutionSupport,
            $crate::domain::assertion::AssertionQualification,
            $crate::domain::assertion::ProviderSurface,
            $crate::domain::assertion::Evidence,
            $crate::domain::value::Literal,
            $crate::domain::value::LiteralSet,
            $crate::domain::value::LiteralSetMember,
            $crate::domain::value::PlaceRoot,
            $crate::domain::value::PathSegment,
            $crate::domain::value::AccessPath,
            $crate::domain::value::Place,
            $crate::domain::value::Predicate,
            $crate::domain::conditions::EvaluationAtom,
            $crate::domain::conditions::ConditionNode,
            $crate::domain::conditions::Condition,
            $crate::domain::input::Package,
            $crate::domain::input::Release,
            $crate::domain::input::InputRevision,
            $crate::domain::input::InputOrigin,
            $crate::domain::input::InputAcquisition,
            $crate::domain::input::CorpusLibrary,
            $crate::domain::input::InputDistribution,
            $crate::domain::input::DistributionVerification,
            $crate::domain::input::ArtifactOwnership,
            $crate::domain::input::UnownedArtifact,
            $crate::domain::input::DerivedArtifact,
            $crate::domain::input::EnvironmentFingerprint,
            $crate::domain::input::ArtifactUse,
            $crate::domain::source::SourceArtifact,
            $crate::domain::artifact::ArtifactChunk,
            $crate::domain::source::Module,
            $crate::domain::source::Occurrence,
            $crate::domain::attribution::Provider,
            $crate::domain::attribution::AnalysisContext,
            $crate::domain::attribution::ProviderRun,
            $crate::domain::attribution::RunFamily,
            $crate::domain::source::SyntaxObservation,
            $crate::domain::source::SyntaxSupport,
            $crate::domain::source::CoverageScope,
            $crate::domain::attribution::ProviderCoverage,
        )
    };
}

/// The relations a facts generation publishes: inputs, attribution and coverage, provider
/// observations and the vocabulary they use (cutover phases 0–2).
pub fn facts_relations() -> Vec<Relation> {
    macro_rules! relations {
        ($($record:ty),* $(,)?) => { vec![$(Relation::of::<$record>()),*] };
    }
    crate::facts_records!(relations)
}

pub mod serving;

pub mod native_requests;

pub mod dependency_closure;

pub mod graph;

pub use record::{decode_allowance, logical_batch_bytes};
