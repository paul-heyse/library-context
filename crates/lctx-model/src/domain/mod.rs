//! Typed domain authority. Physical layouts are lowerings of these definitions (ADR-0085).
pub mod admission;
pub mod analysis;
pub mod analytics;
pub mod artifact;
pub mod assertion;
pub mod assumptions;
pub mod assumptions_universe;
pub mod attachment;
pub mod attribution;
pub mod batching;
pub mod calls;
pub mod class_metadata;
pub mod catalog;
pub mod captures;
pub mod charged;
pub mod composition;
pub mod conditions;
pub mod declarations;
pub mod deployment;
pub mod derivation;
pub mod documents;
pub mod embedding;
pub mod execution;
mod finite;
pub mod flow;
mod identity;
pub mod input;
pub mod lexical;
pub mod local_fields;
pub mod local_semantics;
pub mod local_symbolic;
pub mod local_theory;
pub mod atom_decision;
pub mod memory;
mod model;
pub mod models;
pub mod normalized;
pub mod obligation;
pub mod occurrence_owner;
mod ownership;
pub mod place_composition;
pub mod projection;
pub mod protocols;
mod record;
pub mod resources;
pub mod retrieval;
pub mod selection;
pub mod source;
pub mod stages;
pub mod structural;
pub mod symbols;
pub mod syntax;
pub mod ruff;
pub mod synthesis;
pub mod transfer;
pub mod types;
pub mod value;

pub use finite::FiniteF64;
pub use identity::{ArmId, ContentHash, ContentHasher, EvidenceBytes, Id, Key, KeySink, Utf8Text};
pub use model::{
    Invariant, InvariantCheck, PublicationCheck, PublicationInvariant, Relation, RelationContent,
    ValidatedModel, ValidationInput,
};
pub use record::{
    Arm, ArmField, Batch, Codebook, Field, FieldValue, FlatValue, HeapSize, Record, Scalar, Sum,
    SumRecord,
};

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
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
    ValidatedModel::validate(catalog_frontier_relations())
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
/// The relations a facts generation publishes: inputs, attribution and coverage, provider
/// observations and the vocabulary they use (cutover phases 0–2).
pub fn facts_relations() -> Vec<Relation> {
    use assertion::*;
    use assumptions::*;
    use attribution::*;
    use calls::*;
    use conditions::*;
    use deployment::*;
    use documents::*;
    use flow::*;
    use input::*;
    use lexical::*;
    use source::*;
    use types::*;
    use value::*;
    vec![
        Relation::of::<Assumption>(),
        Relation::of::<AssumptionUniverse>(),
        Relation::of::<AssumptionSet>(),
        Relation::of::<AssumptionSetMember>(),
        Relation::of::<class_metadata::RecordOptions>(),
        Relation::of::<class_metadata::RecordTransformDefaults>(),
        Relation::of::<class_metadata::RecordTransformFieldSpecifier>(),
        Relation::of::<class_metadata::ClassMetadataObservation>(),
        Relation::of::<class_metadata::ClassMetadataSupport>(),
        Relation::of::<class_metadata::ClassMemberObservation>(),
        Relation::of::<class_metadata::ClassMemberSupport>(),
        Relation::of::<ReportCollection>(),
        Relation::of::<ReportValue>(),
        Relation::of::<ReportEntry>(),
        Relation::of::<ReportedEnvironment>(),
        Relation::of::<TaskReport>(),
        Relation::of::<TaskReportObservation>(),
        Relation::of::<TaskReportSupport>(),
        Relation::of::<DeploymentObservation>(),
        Relation::of::<DeploymentSupport>(),
        Relation::of::<TypeVariable>(),
        Relation::of::<captures::CaptureObservation>(),
        Relation::of::<captures::CaptureSupport>(),
        Relation::of::<protocols::NativeExitObservation>(),
        Relation::of::<protocols::NativeExitSupport>(),
        Relation::of::<protocols::NativeTerminalObservation>(),
        Relation::of::<protocols::NativeTerminalSupport>(),
        Relation::of::<protocols::NativeExitDiagnostic>(),
        Relation::of::<protocols::NativeExitDiagnosticSupport>(),
        Relation::of::<TypeTerm>(),
        Relation::of::<TypeSequence>(),
        Relation::of::<TypeSequenceMember>(),
        Relation::of::<CallableParameterList>(),
        Relation::of::<CallableParameter>(),
        Relation::of::<TypedDictFieldList>(),
        Relation::of::<TypedDictField>(),
        Relation::of::<FunctionBodyObservation>(),
        Relation::of::<FunctionBodySupport>(),
        Relation::of::<RecordFieldObservation>(),
        Relation::of::<RecordFieldSupport>(),
        Relation::of::<GenericSpecializationObservation>(),
        Relation::of::<GenericSpecializationSupport>(),
        Relation::of::<NativeSignatureObservation>(),
        Relation::of::<NativeSignatureSupport>(),
        Relation::of::<SignatureTypeSubject>(),
        Relation::of::<SignatureTypeObservation>(),
        Relation::of::<SignatureTypeSupport>(),
        Relation::of::<TypeObservation>(),
        Relation::of::<types::TypeQueryObservation>(),
        Relation::of::<types::TypeQuerySupport>(),
        Relation::of::<TypeSupport>(),
        Relation::of::<TypePresentation>(),
        Relation::of::<TypePresentationSupport>(),
        Relation::of::<TypeVariableRestriction>(),
        Relation::of::<TypeRestrictionSupport>(),
        Relation::of::<FlowUse>(),
        Relation::of::<FlowDefinition>(),
        Relation::of::<ReachingDefinition>(),
        Relation::of::<FlowUseObservation>(),
        Relation::of::<FlowUseSupport>(),
        Relation::of::<FlowDefinitionObservation>(),
        Relation::of::<FlowDefinitionSupport>(),
        Relation::of::<FlowReachingObservation>(),
        Relation::of::<FlowReachingSupport>(),
        Relation::of::<FlowNarrowingObservation>(),
        Relation::of::<FlowNarrowingSupport>(),
        Relation::of::<FlowSourceViewObservation>(),
        Relation::of::<FlowSourceViewSupport>(),
        Relation::of::<FlowValueObservation>(),
        Relation::of::<FlowValueSupport>(),
        Relation::of::<FlowRegionObservation>(),
        Relation::of::<FlowRegionSupport>(),
        Relation::of::<FlowTestObservation>(),
        Relation::of::<FlowTestSupport>(),
        Relation::of::<FlowTestLeafObservation>(),
        Relation::of::<FlowTestLeafSupport>(),
        Relation::of::<FlowAttributeLoadObservation>(),
        Relation::of::<FlowAttributeLoadSupport>(),
        Relation::of::<FlowCallPath>(),
        Relation::of::<FlowCallStep>(),
        Relation::of::<FlowValuePathObservation>(),
        Relation::of::<FlowValuePathSupport>(),
        Relation::of::<DocumentNode>(),
        Relation::of::<DocumentAttributeValue>(),
        Relation::of::<DocumentObservation>(),
        Relation::of::<DocumentSupport>(),
        Relation::of::<PassageObservation>(),
        Relation::of::<PassageSupport>(),
        Relation::of::<CodeBlockObservation>(),
        Relation::of::<CodeBlockSupport>(),
        Relation::of::<DocumentLinkObservation>(),
        Relation::of::<DocumentLinkSupport>(),
        Relation::of::<DocumentMentionObservation>(),
        Relation::of::<DocumentMentionSupport>(),
        Relation::of::<DocumentComponentObservation>(),
        Relation::of::<DocumentComponentSupport>(),
        Relation::of::<DocumentAttributeObservation>(),
        Relation::of::<DocumentAttributeSupport>(),
        Relation::of::<LexicalScope>(),
        Relation::of::<BindingEvent>(),
        Relation::of::<LexicalTarget>(),
        Relation::of::<LexicalScopeObservation>(),
        Relation::of::<LexicalScopeSupport>(),
        Relation::of::<BindingObservation>(),
        Relation::of::<BindingSupport>(),
        Relation::of::<ReferenceObservation>(),
        Relation::of::<ReferenceSupport>(),
        Relation::of::<LexicalResolution>(),
        Relation::of::<LexicalResolutionSupport>(),
        Relation::of::<ProviderModule>(),
        Relation::of::<ProviderCallable>(),
        Relation::of::<ProviderSymbol>(),
        Relation::of::<ParameterShape>(),
        Relation::of::<Signature>(),
        Relation::of::<SignatureParameter>(),
        Relation::of::<SignatureSupport>(),
        Relation::of::<SignatureEnumerationObservation>(),
        Relation::of::<SignatureEnumerationMember>(),
        Relation::of::<SignatureEnumerationSupport>(),
        Relation::of::<CallChannel>(),
        Relation::of::<CallDestination>(),
        Relation::of::<Receiver>(),
        Relation::of::<CallOrigin>(),
        Relation::of::<CallOriginStep>(),
        Relation::of::<ProviderCallSite>(),
        Relation::of::<ProviderCallSiteSupport>(),
        Relation::of::<CallTarget>(),
        Relation::of::<CallTargetSupport>(),
        Relation::of::<CallResolution>(),
        Relation::of::<CallResolutionMember>(),
        Relation::of::<CallResolutionSupport>(),
        Relation::of::<CallSyntax>(),
        Relation::of::<CallSyntaxSupport>(),
        Relation::of::<CallArgument>(),
        Relation::of::<ruff::RuffContextObservation>(),
        Relation::of::<ruff::RuffContextSupport>(),
        Relation::of::<syntax::SyntaxPlacement>(),
        Relation::of::<syntax::SyntaxPlacementSupport>(),
        Relation::of::<syntax::SyntaxDetail>(),
        Relation::of::<syntax::SyntaxDetailObservation>(),
        Relation::of::<syntax::SyntaxDetailSupport>(),
        Relation::of::<syntax::DeclarationObservation>(),
        Relation::of::<syntax::DeclarationSupport>(),
        Relation::of::<syntax::DeclarationDecorator>(),
        Relation::of::<syntax::DeclarationDecoratorSupport>(),
        Relation::of::<syntax::ImportAliasObservation>(),
        Relation::of::<syntax::ImportAliasSupport>(),
        Relation::of::<syntax::DunderAllObservation>(),
        Relation::of::<syntax::DunderAllSupport>(),
        Relation::of::<syntax::ParameterSyntaxObservation>(),
        Relation::of::<syntax::ParameterSyntaxSupport>(),
        Relation::of::<syntax::ClassFieldSyntaxObservation>(),
        Relation::of::<syntax::ClassFieldSyntaxSupport>(),
        Relation::of::<syntax::SubjectBoundary>(),
        Relation::of::<syntax::AttachmentOutcome>(),
        Relation::of::<syntax::AttachmentCandidate>(),
        Relation::of::<declarations::SymbolDeclaration>(),
        Relation::of::<declarations::SymbolDeclarationSupport>(),
        Relation::of::<declarations::ParameterDeclaration>(),
        Relation::of::<declarations::ParameterDeclarationSupport>(),
        Relation::of::<symbols::SymbolSequence>(),
        Relation::of::<symbols::SymbolSequenceMember>(),
        Relation::of::<symbols::SymbolObservation>(),
        Relation::of::<symbols::SymbolSupport>(),
        Relation::of::<symbols::FunctionTraitObservation>(),
        Relation::of::<symbols::FunctionTraitSupport>(),
        Relation::of::<symbols::ClassTraitObservation>(),
        Relation::of::<symbols::ClassTraitSupport>(),
        Relation::of::<symbols::ClassAncestryObservation>(),
        Relation::of::<symbols::ClassAncestrySupport>(),
        Relation::of::<symbols::ParameterAnnotationObservation>(),
        Relation::of::<symbols::ParameterAnnotationSupport>(),
        Relation::of::<symbols::ExportOrigin>(),
        Relation::of::<symbols::PublicNameObservation>(),
        Relation::of::<symbols::ExportEnumerationObservation>(),
        Relation::of::<symbols::ExportEnumerationSupport>(),
        Relation::of::<symbols::PublicNameSupport>(),
        Relation::of::<symbols::ParameterDocObservation>(),
        Relation::of::<symbols::ParameterDocSupport>(),
        Relation::of::<symbols::ModuleResolutionObservation>(),
        Relation::of::<symbols::ModuleResolutionSupport>(),
        Relation::of::<AssertionQualification>(),
        Relation::of::<ProviderSurface>(),
        Relation::of::<Evidence>(),
        Relation::of::<Literal>(),
        Relation::of::<LiteralSet>(),
        Relation::of::<LiteralSetMember>(),
        Relation::of::<PlaceRoot>(),
        Relation::of::<PathSegment>(),
        Relation::of::<AccessPath>(),
        Relation::of::<Place>(),
        Relation::of::<Predicate>(),
        Relation::of::<EvaluationAtom>(),
        Relation::of::<ConditionNode>(),
        Relation::of::<Condition>(),
        Relation::of::<Package>(),
        Relation::of::<Release>(),
        Relation::of::<InputRevision>(),
        Relation::of::<InputOrigin>(),
        Relation::of::<InputAcquisition>(),
        Relation::of::<CorpusLibrary>(),
        Relation::of::<InputDistribution>(),
        Relation::of::<DistributionVerification>(),
        Relation::of::<ArtifactOwnership>(),
        Relation::of::<UnownedArtifact>(),
        Relation::of::<DerivedArtifact>(),
        Relation::of::<EnvironmentFingerprint>(),
        Relation::of::<ArtifactUse>(),
        Relation::of::<SourceArtifact>(),
        Relation::of::<artifact::ArtifactChunk>(),
        Relation::of::<Module>(),
        Relation::of::<Occurrence>(),
        Relation::of::<Provider>(),
        Relation::of::<AnalysisContext>(),
        Relation::of::<ProviderRun>(),
        Relation::of::<RunFamily>(),
        Relation::of::<SyntaxObservation>(),
        Relation::of::<SyntaxSupport>(),
        Relation::of::<CoverageScope>(),
        Relation::of::<ProviderCoverage>(),
    ]
}

pub mod serving;

pub mod native_requests;

pub mod dependency_closure;
