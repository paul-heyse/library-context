//! Original contextual evidence references canonical captured coordinates and earlier semantic owners.
pub mod build;
pub mod characterization;
pub mod diagnostic_uses;
pub use characterization::{SourceCharacterization, SourceCharacterizationScenario, SourceUsage};
pub use diagnostic_uses::{
    DiagnosticUseAssessment, DiagnosticUseLink, DiagnosticUsePath, DiagnosticUseStatus,
    DiagnosticUseTarget,
};
mod fields;
pub mod frames;
mod intent;
mod inventory;
pub mod runtime;
mod symbolic;
use crate::domain::{
    assertion::{AssertionQualification, EvidenceSourceSpanId},
    catalog::*,
    deployment::{CheckStatus, DeploymentObservation, TaskReportObservation},
    documents::*,
    input::{ArtifactOwnership, EnvironmentFingerprint, InputRevision, Release, SourceRole},
    normalized::{bindings::*, entities::*, events::*, links::*},
    source::{Occurrence, SourceArtifact},
    syntax::*,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
pub use symbolic::SourceFieldLink;
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "catalog_original_sources")]
pub enum OriginalSource {
    #[model(code = 0)]
    Artifact { artifact: Id<SourceArtifact> },
    #[model(code = 1)]
    Occurrence { occurrence: Id<Occurrence> },
    #[model(code = 2)]
    Span { span: EvidenceSourceSpanId },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "catalog_scenario_sources")]
pub enum ScenarioSource {
    #[model(code = 0)]
    Python {
        artifact: Id<SourceArtifact>,
        role: SourceRole,
        context: Id<attribution::AnalysisContext>,
        declaration: Option<Id<DeclarationObservation>>,
    },
    #[model(code = 1)]
    Fence {
        observation: Id<CodeBlockObservation>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Intent {
    Unknown = 0,
    Demonstration = 1,
    AssertionTest = 2,
    ExpectedFailure = 3,
    SkipXfail = 4,
    Mixed = 5,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_scenarios",invariants=build::invariants,semantic_source=include_bytes!("build.rs"))]
pub struct CatalogScenario {
    #[model(key)]
    pub source: Id<ScenarioSource>,
    pub extraction: CheckStatus,
    pub parse: CheckStatus,
    pub binding: CheckStatus,
    pub environment: CheckStatus,
    pub execution: CheckStatus,
    pub intent: Intent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SpanRole {
    Primary = 0,
    EnclosingModule = 1,
    EnclosingPassage = 2,
    ExtractedPython = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_scenario_spans")]
pub struct ScenarioSpan {
    #[model(key)]
    pub scenario: Id<CatalogScenario>,
    #[model(key)]
    pub ordinal: i64,
    pub role: SpanRole,
    pub source: Id<OriginalSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "catalog_setup_dependencies")]
pub enum SetupDependency {
    #[model(code = 0)]
    ModuleContext { artifact: Id<SourceArtifact> },
    #[model(code = 1)]
    RuntimeInputs { artifact: Id<SourceArtifact> },
    #[model(code = 2)]
    Formal {
        syntax: Id<ParameterSyntaxObservation>,
    },
    #[model(code = 3)]
    UnresolvedReference {
        assessment: Id<ReferenceEntityAssessment>,
    },
    #[model(code = 4)]
    WithContext { placement: Id<SyntaxPlacement> },
    #[model(code = 5)]
    PrecedingMutation {
        binding: Id<lexical::BindingObservation>,
    },
    #[model(code = 6)]
    TestEnvironment { artifact: Id<SourceArtifact> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_scenario_dependencies")]
pub struct ScenarioDependency {
    #[model(key)]
    pub scenario: Id<CatalogScenario>,
    #[model(key)]
    pub dependency: Id<SetupDependency>,
    pub status: CheckStatus,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AssociationBasis {
    CandidateTarget = 0,
    ResolvedTarget = 1,
    DocumentCandidate = 2,
    DeclaredField = 3,
    SourceFieldCandidate = 4,
    ExactInitialization = 5,
    ExactRead = 6,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_scenario_associations",semantic_source=include_bytes!("intent.rs"))]
pub struct ScenarioAssociation {
    #[model(key)]
    pub scenario: Id<CatalogScenario>,
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub alternative: Id<NormalizedCallAlternative>,
    pub qualification: Id<AssertionQualification>,
    pub phase: calls::CallPhase,
    pub basis: AssociationBasis,
    pub intent: Intent,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_scenario_bindings")]
pub struct ScenarioBinding {
    #[model(key)]
    pub association: Id<ScenarioAssociation>,
    #[model(key)]
    pub attempt: Id<CallBindingAttempt>,
    pub status: CheckStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_scenario_options")]
pub struct ScenarioOption {
    #[model(key)]
    pub association: Id<ScenarioAssociation>,
    #[model(key)]
    pub binding: Id<CallBinding>,
    #[model(key)]
    pub option: Id<CatalogOption>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_document_associations")]
pub struct DocumentAssociation {
    #[model(key)]
    pub member: Id<CatalogMember>,
    #[model(key)]
    pub candidate: Id<MentionEntityCandidate>,
    pub basis: AssociationBasis,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FieldAccessKind {
    Read = 0,
    Write = 1,
    Delete = 2,
    Augment = 3,
}
/// Source access is retained with unknown receiver applicability; earlier exact witnesses own promotion.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_field_access_assessments",semantic_source=include_bytes!("fields.rs"))]
pub struct FieldAccessAssessment {
    #[model(key)]
    pub option: Id<CatalogOption>,
    #[model(key)]
    pub occurrence: Id<Occurrence>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub owner: Id<OccurrenceOwnership>,
    #[model(key)]
    pub receiver: Option<Id<ReferenceEntityCandidate>>,
    pub field: Id<FieldEntity>,
    pub constructor: Option<Id<CatalogConstructor>>,
    pub kind: FieldAccessKind,
    pub phase: calls::CallPhase,
    pub basis: AssociationBasis,
    pub applicability: Knowledge,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_deployments")]
pub struct CatalogDeployment {
    #[model(key)]
    pub observation: Id<DeploymentObservation>,
}
/// Release association cites captured distribution ownership, never inferred distribution spelling.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_release_deployments")]
pub struct ReleaseDeployment {
    #[model(key)]
    pub deployment: Id<CatalogDeployment>,
    #[model(key)]
    pub ownership: Id<ArtifactOwnership>,
    pub release: Id<Release>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_scenario_checks")]
pub struct ScenarioCheck {
    #[model(key)]
    pub scenario: Id<CatalogScenario>,
    #[model(key)]
    pub observation: Id<TaskReportObservation>,
    pub fingerprint: Option<Id<EnvironmentFingerprint>>,
    pub environment: CheckStatus,
    pub execution: CheckStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "catalog_evidence_root_subjects")]
pub enum RootSubject {
    #[model(code = 0)]
    Member { member: Id<CatalogMember> },
    #[model(code = 1)]
    Option { option: Id<CatalogOption> },
    #[model(code = 2)]
    Scenario { scenario: Id<CatalogScenario> },
    #[model(code = 3)]
    Document {
        observation: Id<DocumentObservation>,
    },
    #[model(code = 4)]
    Deployment { deployment: Id<CatalogDeployment> },
    #[model(code = 5)]
    Release { release: Id<Release> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "catalog_evidence_roots")]
pub struct EvidenceRoot {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<attribution::AnalysisContext>,
    #[model(key)]
    pub subject: Id<RootSubject>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="catalog_evidence_invocations",invariants=build::invocation_invariants,semantic_source=include_bytes!("frames.rs"))]
pub struct EvidenceInvocation {
    #[model(key)]
    pub root: Id<EvidenceRoot>,
    #[model(key)]
    pub invocation: Id<analysis::catalog_evidence::Invocation>,
}
pub fn relations() -> Vec<Relation> {
    macro_rules! declare {($($f:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    let mut rows = crate::catalog_evidence_outputs!(declare);
    rows.push(Relation::of::<EvidenceInvocation>());
    rows
}

pub use runtime::{ConstructorCandidateLink, FieldLocationLink};
