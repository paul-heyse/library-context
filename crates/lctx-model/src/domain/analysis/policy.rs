//! One shared evidence floor/ceiling and interpretation policy; documentary prose is not behavioral proof.
use super::invalid;
use crate::domain::{ModelError,attribution::{FactFamily,Fidelity}};
use crate::DomainCode;
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,DomainCode)]
#[repr(i16)]
pub enum EvidenceStatus {
    StructurallyObserved = 0,
    Documented = 1,
    StatisticallyDerived = 2,
    FixtureChecked = 3,
    Unresolved = 4,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,DomainCode)]
#[repr(i16)]
pub enum FindingKind {
    PublicAlias = 0,
    DirectDelegation = 1,
    BoundedDelegationPath = 2,
    ImplementationBoundary = 3,
    IncompleteResolution = 4,
    TraversalStop = 5,
    Forwarding = 6,
    TransformedArgument = 7,
    ConditionalRaise = 8,
    Handoff = 9,
    UnfollowedArgument = 10,
    Community = 11,
    Centrality = 12,
    ApplicableCase = 13,
    Implication = 14,
    DocLink = 15,
    CommunityLabel = 16,
    DirectUsage = 17,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,DomainCode)]
#[repr(i16)]
pub enum MemberRole {
    AccessPath = 0,
    SourceParameter = 1,
    Value = 2,
    Alias = 3,
    Formal = 4,
    ProducerSite = 5,
    ConsumerSite = 6,
    ConditionalCall = 7,
    Reason = 8,
    CommunityMember = 9,
    SupportingSite = 10,
    ExtentMember = 11,
    IntentAttribute = 12,
    Premise = 13,
    Conclusion = 14,
    Label = 15,
    HandoffAttribute = 16,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,DomainCode)]
#[repr(i16)]
pub enum AssertionKind {
    Outcome = 0,
    PublicAccess = 1,
    Coordinates = 2,
    Parameter = 3,
    AnalysisBoundary = 4,
    Related = 5,
    Control = 6,
    TransformedControl = 7,
    Restriction = 8,
    UsagePattern = 9,
    Handoff = 10,
    UnfollowedControl = 11,
    ApplicableCase = 12,
    Implication = 13,
    DocLink = 14,
    SharedSignature = 15,
    DocumentedWarning = 16,
    /// Observed calls in captured sources; never an execution or fixture claim.
    StaticUsageObservation = 17,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,DomainCode)]
#[repr(i16)]
pub enum BriefSection {
    Outcome = 0,
    PublicAccess = 1,
    ApplicableCase = 2,
    Controls = 3,
    UsagePattern = 4,
    Limits = 5,
    Evidence = 6,
    Related = 7,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,DomainCode)]
#[repr(i16)]
pub enum SupportRole {
    Support = 0,
    Scope = 1,
}
pub const ASSERTION_POLICY: &[(AssertionKind, BriefSection, &[EvidenceStatus])] = &[
    (AssertionKind::StaticUsageObservation, BriefSection::UsagePattern, &[EvidenceStatus::StructurallyObserved]),
    (
        AssertionKind::Outcome,
        BriefSection::Outcome,
        &[EvidenceStatus::Documented, EvidenceStatus::Unresolved],
    ),
    (
        AssertionKind::PublicAccess,
        BriefSection::PublicAccess,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::Coordinates,
        BriefSection::PublicAccess,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::Parameter,
        BriefSection::Controls,
        &[
            EvidenceStatus::StructurallyObserved,
            EvidenceStatus::Documented,
        ],
    ),
    (
        AssertionKind::AnalysisBoundary,
        BriefSection::Limits,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::Related,
        BriefSection::Related,
        &[EvidenceStatus::StatisticallyDerived],
    ),
    // Pass B (§9.2): a forwarded parameter is observed. Nothing documents one yet (D18), so the
    // policy permits nothing more (slice 2.1 review F7); 3.4 widens it with its rule.
    (
        AssertionKind::Control,
        BriefSection::Controls,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::TransformedControl,
        BriefSection::Controls,
        &[EvidenceStatus::StructurallyObserved],
    ),
    // An implementation raise is observed. A public precondition needs documented support, which
    // no recognizer gives yet (D18; review F7): 3.4 widens this with its rule.
    (
        AssertionKind::Restriction,
        BriefSection::Limits,
        &[EvidenceStatus::StructurallyObserved],
    ),
    // A value Pass B does not follow is an observed limit of the analysis (review F4).
    (
        AssertionKind::UnfollowedControl,
        BriefSection::Limits,
        &[EvidenceStatus::StructurallyObserved],
    ),
    // A doc's own warning (slice 3.4): documentation, stated with where it is.
    (
        AssertionKind::DocumentedWarning,
        BriefSection::Limits,
        &[EvidenceStatus::Documented],
    ),
    // Reserved for an input or mode source (the increment-2 review's U2): nothing fills it in v1.
    (
        AssertionKind::ApplicableCase,
        BriefSection::ApplicableCase,
        &[EvidenceStatus::StructurallyObserved],
    ),
    // FCA (§9.6): exact over the scope's extracted attributes, the scope stated.
    (
        AssertionKind::SharedSignature,
        BriefSection::Related,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::Implication,
        BriefSection::Controls,
        &[EvidenceStatus::StructurallyObserved],
    ),
    // kNN (§9.7): a link to documentation, statistical like Related.
    (
        AssertionKind::DocLink,
        BriefSection::Related,
        &[EvidenceStatus::StatisticallyDerived],
    ),
    // Pass C and §10.5: a pattern is official usage code (documented), or an executed fixture.
    (
        AssertionKind::UsagePattern,
        BriefSection::UsagePattern,
        &[EvidenceStatus::Documented, EvidenceStatus::FixtureChecked],
    ),
    (
        AssertionKind::Handoff,
        BriefSection::UsagePattern,
        &[EvidenceStatus::StructurallyObserved],
    ),
];


/// Unresolved support cannot be strengthened by siblings. Statistical scope evidence poisons
/// the aggregate even with documentary or fixture evidence; scope never raises its strength.
pub fn derive_status(supports:&[(SupportRole,EvidenceStatus)])->EvidenceStatus {
    use EvidenceStatus as S;
    let supporting=||supports.iter().filter(|(role,_)|*role==SupportRole::Support).map(|(_,status)|*status);
    if supporting().next().is_none() || supporting().any(|s|s==S::Unresolved) {return S::Unresolved;}
    if supports.iter().any(|(_,s)|*s==S::StatisticallyDerived) {return S::StatisticallyDerived;}
    supporting().max_by_key(|status|match status {S::StructurallyObserved=>0,S::Documented=>1,S::FixtureChecked=>2,_=>0}).unwrap()
}
pub fn assertion_policy(kind:AssertionKind,status:EvidenceStatus)->Result<BriefSection,ModelError> {
    let (_,section,allowed)=ASSERTION_POLICY.iter().find(|(k,_,_)|*k==kind).ok_or_else(||invalid("assertion kind has no policy"))?;
    if !allowed.contains(&status) {return Err(invalid("assertion evidence falls outside its floor or ceiling"));}
    Ok(*section)
}
/// Behavioral proof consumers share the Control evidence contract. Extractive prose and
/// statistical lineage cannot establish execution or control behavior.
pub fn behavioral_support(status:EvidenceStatus,heuristic:bool)->Result<(),ModelError> {
    if heuristic {return Err(invalid("heuristic evidence cannot establish behavioral support"));}
    assertion_policy(AssertionKind::Control,status).map(|_|())
}
pub fn finding_policy(kind:FindingKind,status:EvidenceStatus)->Result<(),ModelError> {
    let navigation=matches!(kind,FindingKind::Community|FindingKind::Centrality|FindingKind::DocLink|FindingKind::CommunityLabel);
    let allowed=if navigation {EvidenceStatus::StatisticallyDerived} else {EvidenceStatus::StructurallyObserved};
    if status!=allowed && status!=EvidenceStatus::Unresolved {return Err(invalid("finding kind cannot use this evidence interpretation"));}
    Ok(())
}
pub fn native_status(family:FactFamily,fidelity:Fidelity)->EvidenceStatus {
    if fidelity==Fidelity::DisplayOnly {EvidenceStatus::Unresolved} else if family==FactFamily::Docs {EvidenceStatus::Documented} else {EvidenceStatus::StructurallyObserved}
}
