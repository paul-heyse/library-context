use super::FacetValue;
use crate::domain::{
    attribution::AnalysisContext,
    catalog::evidence::{
        CatalogDeployment, CatalogScenario, ConstructorCandidateLink, DocumentAssociation,
        FieldAccessAssessment, FieldLocationLink, Intent, OriginalSource, ScenarioAssociation,
    },
    catalog::*,
    input::Release,
    normalized::{
        callables::SignatureSlot,
        entities::{ClassEntity, EntityRef},
    },
    *,
};
use crate::{Domain, DomainCode, DomainSum};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Quantifier {
    AnyApplicable = 0,
    AllApplicable = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Outcome {
    Supported = 0,
    Contradicted = 1,
    Unresolved = 2,
    Conflicting = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Mode {
    Discovery = 0,
    Strict = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum JointPolicy {
    IndependentRecords = 0,
    RequireCompatible = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum JointApplicability {
    IndependentRecords = 0,
    CompatibleModeledContext = 1,
    DemonstratedCombination = 2,
    ContradictoryModeledContext = 3,
    NotEstablished = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Reason {
    Witness = 0,
    Counterexample = 1,
    ClosedAbsence = 2,
    NoApplicableDomain = 3,
    IncompleteDomain = 4,
    ComparableConflict = 5,
    IncompatibleContexts = 6,
    MissingContext = 7,
    ResourceRefused = 8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Facet {
    Parameter = 0,
    ParameterType = 1,
    Returns = 2,
    Raises = 3,
    Decorator = 4,
    Async = 5,
    DelegatesTo = 6,
    ForwardsTo = 7,
    HandsOffTo = 8,
    TakesFrom = 9,
    Module = 10,
    Kind = 11,
    ReadsSetting = 12,
    ClassMetadata = 13,
    Deprecation = 14,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum EvidenceBasis {
    SourceDeclaration = 0,
    ProviderDeclaration = 1,
    BoundedModel = 2,
    ObservedScenario = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum DomainKind {
    PublicExposures = 0,
    SignatureVariants = 1,
    ConfigurationFields = 2,
    Relationships = 3,
    Scenarios = 4,
    SourceArtifacts = 5,
    ReleaseDeclarations = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum ConfigurationScope {
    Object = 0,
    PerCall = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum MemberKind {
    Function = 0,
    Method = 1,
    Class = 2,
    Property = 3,
    Module = 4,
    Variable = 5,
    Unknown = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum InvocationForm {
    Function = 0,
    Method = 1,
    Class = 2,
    Static = 3,
    Property = 4,
    ContextManager = 5,
    AsyncContextManager = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum DefaultState {
    Absent = 0,
    LiteralNone = 1,
    Literal = 2,
    SourceExpression = 3,
    OptionalExpressionUnavailable = 4,
    Unknown = 5,
    FactoryExpression = 6,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum CheckAxis {
    Parse = 0,
    Binding = 1,
    Environment = 2,
    Execution = 3,
    Extraction = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum FieldRelationship {
    DeclaredParameter = 0,
    ExactStorage = 1,
    ExactReader = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum DeploymentField {
    RequiresDist = 0,
    ProvidesExtra = 1,
    RequiresPython = 2,
    EntryPoint = 3,
    Launch = 4,
    Configuration = 5,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum RelationRole {
    Declares = 0,
    Surface = 1,
    Configuration = 2,
    Reader = 3,
    Documents = 4,
    Demonstrates = 5,
    Invokes = 6,
    TestsFailure = 7,
    Suggests = 8,
    Observes = 9,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum Fidelity {
    SourceFact = 0,
    ResolvedTarget = 1,
    CandidateTargets = 2,
    ExactTextualReference = 3,
    AmbiguousTextualMention = 4,
    SameDocumentPassage = 5,
    OwningDistribution = 6,
    TaskObservation = 7,
    ReleaseDistribution = 8,
    ExplicitConfigReference = 9,
}
/// Declaration identities have no runtime wildcards. Runtime algebra is a separate pure value.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    DomainSum,
    serde::Serialize,
    serde::Deserialize,
)]
#[model(name = "selection_contexts")]
pub enum Context {
    #[model(code = 0)]
    Member {
        member: Id<CatalogMember>,
        analysis: Id<AnalysisContext>,
    },
    #[model(code = 1)]
    Binding {
        member: Id<CatalogMember>,
        candidate: Id<CatalogCandidate>,
        analysis: Id<AnalysisContext>,
    },
    #[model(code = 2)]
    Signature {
        member: Id<CatalogMember>,
        candidate: Id<CatalogCandidate>,
        invocation: Id<CatalogInvocation>,
        analysis: Id<AnalysisContext>,
    },
    #[model(code = 3)]
    Configuration {
        member: Id<CatalogMember>,
        owner: Id<ClassEntity>,
        scope: ConfigurationScope,
        analysis: Id<AnalysisContext>,
    },
    #[model(code = 4)]
    Scenario {
        member: Id<CatalogMember>,
        scenario: Id<CatalogScenario>,
        analysis: Id<AnalysisContext>,
    },
    #[model(code = 5)]
    Source {
        member: Id<CatalogMember>,
        source: Id<OriginalSource>,
        analysis: Id<AnalysisContext>,
    },
    #[model(code = 6)]
    Release {
        release: Id<Release>,
        analysis: Id<AnalysisContext>,
    },
}
impl Context {
    pub fn member(&self) -> Option<Id<CatalogMember>> {
        match self {
            Self::Member { member, .. }
            | Self::Binding { member, .. }
            | Self::Signature { member, .. }
            | Self::Configuration { member, .. }
            | Self::Scenario { member, .. }
            | Self::Source { member, .. } => Some(*member),
            Self::Release { .. } => None,
        }
    }
    pub fn analysis(&self) -> Id<AnalysisContext> {
        match self {
            Self::Member { analysis, .. }
            | Self::Binding { analysis, .. }
            | Self::Signature { analysis, .. }
            | Self::Configuration { analysis, .. }
            | Self::Scenario { analysis, .. }
            | Self::Source { analysis, .. }
            | Self::Release { analysis, .. } => *analysis,
        }
    }
}
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    DomainSum,
    serde::Serialize,
    serde::Deserialize,
)]
#[model(name = "selection_witnesses")]
pub enum Witness {
    #[model(code = 0)]
    Qualification {
        qualification: Id<assertion::AssertionQualification>,
    },
    #[model(code = 1)]
    Member { member: Id<CatalogMember> },
    #[model(code = 2)]
    Candidate { candidate: Id<CatalogCandidate> },
    #[model(code = 3)]
    Invocation { invocation: Id<CatalogInvocation> },
    #[model(code = 4)]
    Option { option: Id<CatalogOption> },
    #[model(code = 5)]
    FieldAccess {
        assessment: Id<FieldAccessAssessment>,
    },
    #[model(code = 6)]
    Association {
        association: Id<ScenarioAssociation>,
    },
    #[model(code = 7)]
    Document {
        association: Id<DocumentAssociation>,
    },
    #[model(code = 8)]
    Original { source: Id<OriginalSource> },
    #[model(code = 9)]
    Deployment { deployment: Id<CatalogDeployment> },
    #[model(code = 10)]
    Domain { domain: Id<SelectionDomain> },
    #[model(code = 11)]
    CoreCoverage {
        coverage: Id<analysis::catalog_core::AnalysisCoverage>,
    },
    #[model(code = 12)]
    EvidenceCoverage {
        coverage: Id<analysis::catalog_evidence::AnalysisCoverage>,
    },
    #[model(code = 13)]
    TypeObservation {
        observation: Id<types::TypeObservation>,
    },
    #[model(code = 14)]
    SignatureSlot { slot: Id<SignatureSlot> },
    #[model(code = 15)]
    NativeField {
        observation: Id<types::RecordFieldObservation>,
    },
    #[model(code = 16)]
    ReceiverLocation { link: Id<FieldLocationLink> },
    #[model(code = 17)]
    ConstructorCandidate { link: Id<ConstructorCandidateLink> },
    #[model(code = 18)]
    SignatureTypeObservation {
        observation: Id<types::SignatureTypeObservation>,
    },
    #[model(code = 19)]
    SummaryException {
        outcome: Id<execution::summary_exceptions::SummaryExceptionOutcome>,
    },
    #[model(code = 20)]
    GenericSpecialization {
        observation: Id<types::GenericSpecializationObservation>,
    },
    #[model(code = 21)]
    SourceCharacterization {
        observation: Id<syntax::DeclarationObservation>,
        support: Id<syntax::DeclarationSupport>,
    },
    #[model(code = 22)]
    ClassMetadata {
        observation: Id<class_metadata::ClassMetadataObservation>,
        support: Id<class_metadata::ClassMetadataSupport>,
    },
    #[model(code = 23)]
    NativeCallableMetadata {
        observation: Id<types::NativeSignatureObservation>,
        support: Id<types::NativeSignatureSupport>,
    },
    #[model(code = 24)]
    RaisedType {
        observation: Id<types::TypeObservation>,
        support: Id<types::TypeSupport>,
    },
    #[model(code = 25)]
    ResolvedDecorator {
        observation: Id<syntax::DeclarationDecorator>,
        support: Id<syntax::DeclarationDecoratorSupport>,
        assessment: Id<normalized::links::ReferenceEntityAssessment>,
        candidate: Id<normalized::links::ReferenceEntityCandidate>,
    },
    #[model(code = 26)]
    LexicalDefinition {
        observation: Id<lexical::BindingObservation>,
        support: Id<lexical::BindingSupport>,
    },
    #[model(code = 27)]
    NativeTypingCoverage {
        coverage: Id<attribution::ProviderCoverage>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="catalog_selection_domains",invariant_refs=super::build::invariants_refs)]
pub struct SelectionDomain {
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub analysis: Id<AnalysisContext>,
    #[model(key)]
    pub kind: DomainKind,
    pub corpus_complete: bool,
    pub analyzer_complete: bool,
    pub contexts: ContentHash,
    pub closure: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "catalog_selection_domain_contexts")]
pub struct DomainContext {
    #[model(key)]
    pub domain: Id<SelectionDomain>,
    #[model(key)]
    pub context: Id<Context>,
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "catalog_selection_domain_closure")]
pub struct DomainClosure {
    #[model(key)]
    pub domain: Id<SelectionDomain>,
    #[model(key)]
    pub evidence: Id<Witness>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "catalog_selection_domain_evidence")]
pub struct DomainEvidence {
    #[model(key)]
    pub domain: Id<SelectionDomain>,
    #[model(key)]
    pub context: Id<Context>,
    #[model(key)]
    pub witness: Id<Witness>,
}
/// Query terms are evaluated against the finite published domains, never exhaustively persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuralType {
    CanonicalTerm { term: Id<types::TypeTerm> },
    Category { kind: i16 },
    NominalIdentity { module: String, name: String },
    DeclaredUnionMember { term: Id<types::TypeTerm> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationTarget {
    Declaration { entity: Id<EntityRef> },
    Member { member: Id<CatalogMember> },
    Original { source: Id<OriginalSource> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldTarget {
    Declaration { entity: Id<EntityRef> },
    Parameter { slot: Id<SignatureSlot> },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Predicate {
    /// Exact finite body behavior under source function entry, using runtime model evidence.
    BehavioralRaises {
        exception: execution::ExactRuntimeException,
    },
    FacetMembership {
        facet: Facet,
        value: FacetValue,
    },
    PublicPath {
        path: Vec<String>,
    },
    PublicModule {
        module: String,
    },
    ClassOwner {
        path: Vec<String>,
    },
    MemberKind {
        kind: MemberKind,
    },
    InvocationForm {
        form: InvocationForm,
    },
    DeclaresParameter {
        name: String,
    },
    ParameterKind {
        name: String,
        kind: calls::ParameterKind,
    },
    ParameterRequired {
        name: String,
        required: bool,
    },
    ParameterDefaultState {
        name: String,
        state: DefaultState,
    },
    ParameterDefault {
        name: String,
        value: Id<value::Literal>,
    },
    ParameterType {
        name: String,
        r#type: StructuralType,
    },
    VariantParameterType {
        role: calls::SignatureRole,
        name: String,
        r#type: StructuralType,
    },
    VariantReturnType {
        role: calls::SignatureRole,
        r#type: StructuralType,
    },
    SpecializedType {
        site: Id<source::Occurrence>,
        declaration: Id<types::NativeSignatureObservation>,
        subject: Id<types::SignatureTypeSubject>,
        r#type: StructuralType,
    },
    DeclaresConfigurationField {
        name: String,
    },
    ConfigurationOwner {
        path: Vec<String>,
    },
    ConfigurationScope {
        scope: ConfigurationScope,
    },
    ConfigurationRecordKind {
        kind: types::RecordKind,
    },
    ConfigurationDefault {
        name: String,
        value: Id<value::Literal>,
    },
    ConfigurationLiteral {
        name: String,
        value: Id<value::Literal>,
    },
    ConfigurationRelationship {
        name: String,
        kind: FieldRelationship,
        target: FieldTarget,
    },
    Relationship {
        role: RelationRole,
        target: RelationTarget,
        fidelity: Fidelity,
    },
    ScenarioIntent {
        intent: Intent,
    },
    ScenarioCheck {
        check: CheckAxis,
        status: deployment::CheckStatus,
    },
    SourceAlignment {
        exact: bool,
    },
    ReleaseVersion {
        distribution: String,
        version: String,
    },
    DeploymentDeclaration {
        field: DeploymentField,
        name: String,
    },
}
impl Predicate {
    pub fn domain(&self) -> DomainKind {
        match self {
            Self::BehavioralRaises { .. }
            | Self::PublicPath { .. }
            | Self::PublicModule { .. }
            | Self::ClassOwner { .. }
            | Self::MemberKind { .. }
            | Self::InvocationForm { .. } => DomainKind::PublicExposures,
            Self::DeclaresParameter { .. }
            | Self::ParameterKind { .. }
            | Self::ParameterRequired { .. }
            | Self::ParameterDefaultState { .. }
            | Self::ParameterDefault { .. }
            | Self::ParameterType { .. }
            | Self::VariantParameterType { .. }
            | Self::VariantReturnType { .. }
            | Self::SpecializedType { .. } => DomainKind::SignatureVariants,
            Self::DeclaresConfigurationField { .. }
            | Self::ConfigurationOwner { .. }
            | Self::ConfigurationScope { .. }
            | Self::ConfigurationRecordKind { .. }
            | Self::ConfigurationDefault { .. }
            | Self::ConfigurationLiteral { .. }
            | Self::ConfigurationRelationship { .. } => DomainKind::ConfigurationFields,
            Self::Relationship { .. } => DomainKind::Relationships,
            Self::FacetMembership { value, .. } => value.domain(),
            Self::ScenarioIntent { .. } | Self::ScenarioCheck { .. } => DomainKind::Scenarios,
            Self::SourceAlignment { .. }
            | Self::DeploymentDeclaration {
                field: DeploymentField::Launch | DeploymentField::Configuration,
                ..
            } => DomainKind::SourceArtifacts,
            Self::ReleaseVersion { .. } | Self::DeploymentDeclaration { .. } => {
                DomainKind::ReleaseDeclarations
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    pub predicate: Predicate,
    pub quantifier: Quantifier,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub requirements: Vec<Requirement>,
    pub mode: Mode,
    pub joint: JointPolicy,
}
impl HeapSize for StructuralType {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::NominalIdentity { module, name } => module.heap_bytes() + name.heap_bytes(),
            _ => 0,
        }
    }
}
impl HeapSize for Predicate {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::FacetMembership { value, .. } => value.heap_bytes(),
            Self::PublicPath { path }
            | Self::ClassOwner { path }
            | Self::ConfigurationOwner { path } => path.heap_bytes(),
            Self::PublicModule { module } => module.heap_bytes(),
            Self::DeclaresParameter { name }
            | Self::ParameterKind { name, .. }
            | Self::ParameterRequired { name, .. }
            | Self::ParameterDefaultState { name, .. }
            | Self::DeclaresConfigurationField { name }
            | Self::ConfigurationRelationship { name, .. }
            | Self::DeploymentDeclaration { name, .. } => name.heap_bytes(),
            Self::ParameterDefault { name, value }
            | Self::ConfigurationDefault { name, value }
            | Self::ConfigurationLiteral { name, value } => name.heap_bytes() + value.heap_bytes(),
            Self::ParameterType { name, r#type }
            | Self::VariantParameterType { name, r#type, .. } => {
                name.heap_bytes() + r#type.heap_bytes()
            }
            Self::VariantReturnType { r#type, .. } | Self::SpecializedType { r#type, .. } => {
                r#type.heap_bytes()
            }
            Self::ReleaseVersion {
                distribution,
                version,
            } => distribution.heap_bytes() + version.heap_bytes(),
            _ => 0,
        }
    }
}
impl HeapSize for Requirement {
    fn heap_bytes(&self) -> usize {
        self.predicate.heap_bytes()
    }
}

impl Predicate {
    pub fn validate(&self) -> Result<(), ModelError> {
        fn text(s: &str) -> Result<(), ModelError> {
            if s.is_empty() || s.len() > 1024 || s.chars().any(char::is_control) {
                Err(ModelError::Invalid(
                    "selection term text is empty, oversized or contains controls".into(),
                ))
            } else {
                Ok(())
            }
        }
        fn path(v: &[String]) -> Result<(), ModelError> {
            if v.is_empty() || v.len() > 32 {
                return Err(ModelError::Invalid(
                    "selection path exceeds its finite bounds".into(),
                ));
            }
            for s in v {
                text(s)?;
                if s.contains('.') {
                    return Err(ModelError::Invalid(
                        "selection path segment contains separator".into(),
                    ));
                }
            }
            Ok(())
        }
        if matches!(
            self,
            Self::VariantParameterType {
                role: calls::SignatureRole::Specialized,
                ..
            } | Self::VariantReturnType {
                role: calls::SignatureRole::Specialized,
                ..
            }
        ) {
            return Err(ModelError::Invalid(
                "specialized type requires a located site and native declaration".into(),
            ));
        }
        match self {
            Self::PublicPath { path: p }
            | Self::ClassOwner { path: p }
            | Self::ConfigurationOwner { path: p } => path(p),
            Self::BehavioralRaises { .. } => Ok(()),
            Self::FacetMembership { facet, value } => value.validate(*facet),
            Self::PublicModule { module } => text(module),
            Self::DeclaresParameter { name }
            | Self::ParameterKind { name, .. }
            | Self::ParameterRequired { name, .. }
            | Self::ParameterDefaultState { name, .. }
            | Self::ParameterDefault { name, .. }
            | Self::DeclaresConfigurationField { name }
            | Self::ConfigurationDefault { name, .. }
            | Self::ConfigurationLiteral { name, .. }
            | Self::ConfigurationRelationship { name, .. }
            | Self::DeploymentDeclaration { name, .. } => text(name),
            Self::ParameterType { name, r#type }
            | Self::VariantParameterType { name, r#type, .. } => {
                text(name)?;
                match r#type {
                    StructuralType::NominalIdentity { module, name } => {
                        text(module)?;
                        text(name)
                    }
                    StructuralType::Category { kind } if !(0..=34).contains(kind) => {
                        Err(ModelError::Invalid(
                            "selection type category is outside the canonical codebook".into(),
                        ))
                    }
                    _ => Ok(()),
                }
            }
            Self::VariantReturnType { r#type, .. } | Self::SpecializedType { r#type, .. } => {
                // Reuse the same canonical structural pattern validation.
                Self::ParameterType {
                    name: "return".into(),
                    r#type: r#type.clone(),
                }
                .validate()
            }
            Self::ReleaseVersion {
                distribution,
                version,
            } => {
                text(distribution)?;
                text(version)
            }
            _ => Ok(()),
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            requirements: vec![],
            mode: Mode::Discovery,
            joint: JointPolicy::IndependentRecords,
        }
    }
}
