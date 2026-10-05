//! Model-owned topology contracts and generation-local materializations (ADR-0103).
//! Graphs are rebuilt for every collection. Typed canonical relations retain semantic authority.
use super::{
    attribution::{AnalysisContext, FactFamily, ProviderCoverage},
    input::InputRevision,
    normalized::{entities::*, events::*, links::*},
    source::CoverageScope,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
pub(crate) mod inventory;
pub mod native;
pub mod normalization;
pub mod snapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum EndpointRole {
    Invocation = 0,
    Definition = 1,
    Containment = 2,
    Import = 3,
    Reference = 4,
    PublicExposure = 5,
    SourceDefinition = 6,
}
impl EndpointRole {
    pub fn relation(self) -> &'static str {
        match self {
            Self::Invocation | Self::Definition => NormalizedCallAlternative::NAME,
            Self::Containment | Self::SourceDefinition => OccurrenceOwnership::NAME,
            Self::Import => ImportModuleCandidate::NAME,
            Self::Reference => ReferenceEntityCandidate::NAME,
            Self::PublicExposure => PublicExposureCandidate::NAME,
        }
    }
}
pub fn call_roles() -> Vec<EndpointRole> {
    vec![EndpointRole::Invocation, EndpointRole::Definition]
}
pub fn containment_roles() -> Vec<EndpointRole> {
    vec![EndpointRole::Containment, EndpointRole::SourceDefinition]
}
pub fn import_roles() -> Vec<EndpointRole> {
    vec![EndpointRole::Import]
}
pub fn reference_roles() -> Vec<EndpointRole> {
    vec![EndpointRole::Reference]
}
pub fn exposure_roles() -> Vec<EndpointRole> {
    vec![EndpointRole::PublicExposure]
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
#[model(inventory)]
pub enum ProjectionName {
    #[model(wire = "CallableInvocation")]
    CallableInvocation = 0,
    #[model(wire = "DefinitionContainment")]
    DefinitionContainment = 1,
    #[model(wire = "ImportReference")]
    ImportReference = 2,
    #[model(wire = "PublicExposure")]
    PublicExposure = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniversePolicy {
    InputEntitiesAndContextTargets,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplicityPolicy {
    PreserveTypedParallelArcs,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityPolicy {
    RetainEveryRequiredScope,
}
/// Finite named meanings, not flags that turn potential or definition relationships into calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionSpec {
    name: ProjectionName,
}
impl ProjectionSpec {
    pub const VERSION: i32 = 3;
    pub(crate) fn encode_contract(self, sink: &mut KeySink) {
        sink.part(
            b"universe",
            match self.universe() {
                UniversePolicy::InputEntitiesAndContextTargets => {
                    b"input-entities-and-context-targets"
                }
            },
        );
        sink.part(
            b"multiplicity",
            match self.multiplicity() {
                MultiplicityPolicy::PreserveTypedParallelArcs => b"preserve-typed-parallel-arcs",
            },
        );
        sink.part(
            b"availability",
            match self.availability() {
                AvailabilityPolicy::RetainEveryRequiredScope => b"retain-every-required-scope",
            },
        );
        sink.part(b"role-count", &(self.roles().len() as u64).to_le_bytes());
        for role in self.roles() {
            role.encode(sink);
        }
        self.call_policy().encode(sink);
    }
    pub fn builtin(name: ProjectionName) -> Self {
        Self { name }
    }
    pub fn name(self) -> ProjectionName {
        self.name
    }
    pub fn universe(self) -> UniversePolicy {
        UniversePolicy::InputEntitiesAndContextTargets
    }
    pub fn multiplicity(self) -> MultiplicityPolicy {
        MultiplicityPolicy::PreserveTypedParallelArcs
    }
    pub fn availability(self) -> AvailabilityPolicy {
        AvailabilityPolicy::RetainEveryRequiredScope
    }
    pub fn roles(self) -> &'static [EndpointRole] {
        match self.name {
            ProjectionName::CallableInvocation => &[EndpointRole::Invocation],
            ProjectionName::DefinitionContainment => &[
                EndpointRole::Definition,
                EndpointRole::Containment,
                EndpointRole::SourceDefinition,
            ],
            ProjectionName::ImportReference => &[EndpointRole::Import, EndpointRole::Reference],
            ProjectionName::PublicExposure => &[EndpointRole::PublicExposure],
        }
    }
    pub fn call_policy(self) -> Option<CallPolicy> {
        (self.name == ProjectionName::CallableInvocation).then_some(CallPolicy::Invocation)
    }
    pub fn families(self) -> &'static [FactFamily] {
        match self.name {
            ProjectionName::CallableInvocation | ProjectionName::DefinitionContainment => &[
                FactFamily::Artifacts,
                FactFamily::Syntax,
                FactFamily::Signatures,
                FactFamily::Calls,
            ],
            ProjectionName::ImportReference => &[
                FactFamily::Artifacts,
                FactFamily::Syntax,
                FactFamily::Signatures,
                FactFamily::Lexical,
                FactFamily::Types,
                FactFamily::Exports,
            ],
            ProjectionName::PublicExposure => &[
                FactFamily::Artifacts,
                FactFamily::Syntax,
                FactFamily::Signatures,
                FactFamily::Types,
                FactFamily::Lexical,
                FactFamily::Exports,
            ],
        }
    }
    pub fn accepts(self, entity: &EntityRef) -> bool {
        match entity {
            EntityRef::Module { .. } | EntityRef::Callable { .. } | EntityRef::Class { .. } => true,
            EntityRef::Occurrence { .. } => matches!(
                self.name,
                ProjectionName::DefinitionContainment | ProjectionName::ImportReference
            ),
            EntityRef::Parameter { .. } | EntityRef::Field { .. } => matches!(
                self.name,
                ProjectionName::ImportReference | ProjectionName::PublicExposure
            ),
            EntityRef::Type { .. } | EntityRef::Place { .. } => false,
        }
    }
}
/// Arc identity is its typed canonical relationship, without another ID catalog or reuse hash.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum ArcId {
    Invocation(Id<NormalizedCallAlternative>),
    Definition(Id<NormalizedCallAlternative>),
    Containment(Id<OccurrenceOwnership>),
    Import(Id<ImportModuleCandidate>),
    Reference(Id<ReferenceEntityCandidate>),
    PublicExposure(Id<PublicExposureCandidate>),
    /// Exact lexical declaration ownership, distinct from runtime invocation.
    SourceDefinition(Id<OccurrenceOwnership>),
}
impl ArcId {
    pub fn role(self) -> EndpointRole {
        match self {
            Self::Invocation(_) => EndpointRole::Invocation,
            Self::Definition(_) => EndpointRole::Definition,
            Self::Containment(_) => EndpointRole::Containment,
            Self::Import(_) => EndpointRole::Import,
            Self::Reference(_) => EndpointRole::Reference,
            Self::PublicExposure(_) => EndpointRole::PublicExposure,
            Self::SourceDefinition(_) => EndpointRole::SourceDefinition,
        }
    }
}
impl HeapSize for ArcId {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arc {
    pub id: ArcId,
    pub source: Id<EntityRef>,
    pub target: Id<EntityRef>,
}
impl HeapSize for Arc {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ProjectionAvailability {
    CompleteUnderStatedModel = 0,
    Partial = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ProjectionGapReason {
    Unresolved = 0,
    Ambiguous = 1,
    OutsidePolicy = 2,
    OutsideUniverse = 3,
    ExternalModule = 4,
    IncompleteCoverage = 5,
    MissingCoverage = 6,
    NoAlternative = 7,
    Builtin = 8,
    EventUncertainty = 9,
    OverrideDispatch = 10,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "projection_source_assessments", invariant_refs = normalization::invariants_refs)]
pub struct ProjectionSourceAssessment {
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub projection: ProjectionName,
    #[model(key)]
    pub version: i32,
    pub vertices: i64,
    pub arcs: i64,
    pub gaps: i64,
    pub availability: ProjectionAvailability,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "projection_gap_subjects")]
pub enum ProjectionGapSubject {
    #[model(code = 0)]
    Event { event: Id<NormalizedCallEvent> },
    #[model(code = 1)]
    Alternative {
        alternative: Id<NormalizedCallAlternative>,
    },
    #[model(code = 2)]
    Import {
        assessment: Id<ImportModuleAssessment>,
    },
    #[model(code = 3)]
    Reference {
        assessment: Id<ReferenceEntityAssessment>,
    },
    #[model(code = 4)]
    Exposure { exposure: Id<PublicExposure> },
    #[model(code = 5)]
    Coverage { coverage: Id<ProviderCoverage> },
    #[model(code = 7)]
    EventAssessment { assessment: Id<EventAssessment> },
    #[model(code = 6)]
    MissingCoverage {
        scope: Id<CoverageScope>,
        family: FactFamily,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "projection_gaps")]
pub struct ProjectionGap {
    #[model(key)]
    pub assessment: Id<ProjectionSourceAssessment>,
    #[model(key)]
    pub subject: Id<ProjectionGapSubject>,
    #[model(key)]
    pub reason: ProjectionGapReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "projection_source_coverage")]
pub struct ProjectionSourceCoverage {
    #[model(key)]
    pub assessment: Id<ProjectionSourceAssessment>,
    #[model(key)]
    pub coverage: Id<ProviderCoverage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "projection_snapshots", validate = snapshot::validate_header)]
pub struct ProjectionSnapshot {
    #[model(key)]
    pub assessment: Id<ProjectionSourceAssessment>,
    pub format_version: i32,
    pub petgraph_version: String,
    pub codec: String,
    pub bytes: i64,
    pub chunks: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "projection_snapshot_chunks", validate = snapshot::validate_chunk)]
pub struct ProjectionSnapshotChunk {
    #[model(key)]
    pub snapshot: Id<ProjectionSnapshot>,
    #[model(key)]
    pub ordinal: i64,
    pub payload: EvidenceBytes,
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ProjectionSourceAssessment>(),
        Relation::of::<ProjectionGapSubject>(),
        Relation::of::<ProjectionGap>(),
        Relation::of::<ProjectionSourceCoverage>(),
        Relation::of::<ProjectionSnapshot>(),
        Relation::of::<ProjectionSnapshotChunk>(),
    ]
}

