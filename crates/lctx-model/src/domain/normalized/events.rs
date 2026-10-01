//! Complete normalized events precede the five named consumer policies.
use crate::domain::{
    attribution::AnalysisContext,
    calls::*,
    normalized::{entities::*, links::LinkReason},
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum PhaseGroup {
    Call = 0,
    Construct = 1,
    Decorator = 2,
    Property = 3,
    Definition = 4,
}
impl PhaseGroup {
    pub fn of(phase: CallPhase) -> Self {
        match phase {
            CallPhase::Call => Self::Call,
            CallPhase::New | CallPhase::Init => Self::Construct,
            CallPhase::Decorator => Self::Decorator,
            CallPhase::PropertyGet | CallPhase::PropertySet => Self::Property,
            CallPhase::Definition => Self::Definition,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum CallPolicy {
    Invocation = 0,
    Dataflow = 1,
    Summary = 2,
    Usage = 3,
    Association = 4,
}
impl CallPolicy {
    pub const ALL: [Self; 5] = [
        Self::Invocation,
        Self::Dataflow,
        Self::Summary,
        Self::Usage,
        Self::Association,
    ];
    pub fn view_name(self) -> &'static str {
        match self {
            Self::Invocation => "invocation_calls",
            Self::Dataflow => "dataflow_calls",
            Self::Summary => "summary_calls",
            Self::Usage => "usage_calls",
            Self::Association => "association_calls",
        }
    }
    /// The SQL lowering selects already validated memberships. It contains no semantic predicate.
    /// `qualify` supplies physical quoting only; the same selection is used by both query engines.
    pub fn select_sql(self, qualify: impl Fn(&str) -> String) -> String {
        format!(
            "SELECT a.*, m.id AS admission_id, p.id AS policy_assessment_id FROM {} m JOIN {} p ON p.id=m.assessment JOIN {} a ON a.id=m.alternative WHERE p.policy={}",
            qualify(CallPolicyAdmission::NAME),
            qualify(CallPolicyAssessment::NAME),
            qualify(NormalizedCallAlternative::NAME),
            self.code()
        )
    }
    /// Include the invariant owner even though its rows are not selected by the SQL view.
    pub fn view_relations() -> [&'static str; 4] {
        [
            NormalizedCallEvent::NAME,
            CallPolicyAdmission::NAME,
            CallPolicyAssessment::NAME,
            NormalizedCallAlternative::NAME,
        ]
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum EventReason {
    CompleteUnique = 0,
    NoDirectResolution = 1,
    OpenResolution = 2,
    MissingCorrespondence = 3,
    Dispatched = 4,
    ConflictingDestinations = 5,
    MultiplePhaseGroups = 6,
    QualifiedUncertainty = 7,
    UnknownReceiver = 8,
    UnreportedTarget = 9,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PolicyReason {
    Admitted = 0,
    NoAlternative = 1,
    OutsidePolicy = 2,
    IncompleteEvent = 3,
    NonUniqueEvent = 4,
    UncertainEvent = 5,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_call_events", invariants = super::event_normalization::invariants)]
pub struct NormalizedCallEvent {
    #[model(key)]
    pub site: Id<source::Occurrence>,
    #[model(key)]
    pub origin: Id<CallOrigin>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    pub owner: Id<OccurrenceOwnership>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_call_event_sources")]
pub struct CallEventSource {
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub observation: Id<ProviderCallSite>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_call_source_evidence")]
pub struct CallEventSourceEvidence {
    #[model(key)]
    pub source: Id<CallEventSource>,
    #[model(key)]
    pub support: Id<ProviderCallSiteSupport>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_call_event_resolutions")]
pub struct CallEventResolution {
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub resolution: Id<CallResolution>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_call_resolution_evidence")]
pub struct CallEventResolutionEvidence {
    #[model(key)]
    pub resolution: Id<CallEventResolution>,
    #[model(key)]
    pub support: Id<CallResolutionSupport>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_call_alternatives", projection_roles = crate::domain::projection::call_roles)]
pub struct NormalizedCallAlternative {
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub source: Id<CallAlternativeSource>,
    #[model(key)]
    pub resolution: Option<Id<CallResolution>>,
    pub correspondence: Option<Id<SymbolEntityResolution>>,
    pub entity: Option<Id<EntityRef>>,
    pub status: ResolutionStatus,
    pub reason: LinkReason,
}
/// A derived member retains its original native premise; it never impersonates a raw target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "normalized_call_alternative_sources")]
pub enum CallAlternativeSource {
    #[model(code = 0)]
    Native { target: Id<CallTarget> },
    #[model(code = 1)]
    DerivedDispatch { target: Id<CallTarget>, member: Id<super::dispatch::DispatchMember> },
}
impl CallAlternativeSource {
    pub fn target(&self) -> Id<CallTarget> { match self { Self::Native { target } | Self::DerivedDispatch { target, .. } => *target } }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "normalized_call_alternative_evidence")]
pub struct CallAlternativeEvidence {
    #[model(key)]
    pub alternative: Id<NormalizedCallAlternative>,
    #[model(key)]
    pub support: Id<CallTargetSupport>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_event_assessments")]
pub struct EventAssessment {
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub policy: ContentHash,
    pub members: ContentHash,
    /// Completeness/uniqueness refer to the declared direct-channel phase group. Higher-order
    /// evidence remains in `members` but never becomes a direct invocation target.
    pub complete: bool,
    pub unique: bool,
    pub exact: bool,
    pub known_receivers: bool,
    pub group: Option<PhaseGroup>,
    pub unresolved: bool,
    pub dispatch: bool,
    pub disagreement: bool,
    pub reason: EventReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_event_phase_targets")]
pub struct EventPhaseTarget {
    #[model(key)]
    pub assessment: Id<EventAssessment>,
    #[model(key)]
    pub phase: CallPhase,
    #[model(key)]
    pub entity: Id<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_policy_assessments")]
pub struct CallPolicyAssessment {
    #[model(key)]
    pub event: Id<NormalizedCallEvent>,
    #[model(key)]
    pub policy: CallPolicy,
    pub event_assessment: Id<EventAssessment>,
    pub admitted: i64,
    pub members: ContentHash,
    pub reason: PolicyReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_policy_admissions")]
pub struct CallPolicyAdmission {
    #[model(key)]
    pub assessment: Id<CallPolicyAssessment>,
    #[model(key)]
    pub alternative: Id<NormalizedCallAlternative>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "flow_call_event_links")]
pub struct FlowCallEventLink {
    #[model(key)]
    pub observation: Id<flow::FlowValuePathObservation>,
    #[model(key)]
    pub step: Id<flow::FlowCallStep>,
    pub event: Option<Id<NormalizedCallEvent>>,
    pub status: ResolutionStatus,
    pub reason: FlowEventReason,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FlowEventReason {
    ExactEvent = 0,
    NoReportedEvent = 1,
    AmbiguousEvent = 2,
}
pub fn relations() -> Vec<Relation> {
    macro_rules! declare { ($($field:ident: $ty:ty,)*) => { vec![$(Relation::of::<$ty>()),*] }; }
    crate::normalized_event_outputs!(declare)
}
