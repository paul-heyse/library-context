//! Shared typed emitter and status policy. Statistical evidence remains navigation evidence;
//! an extractive documentary statement is never converted into a behavioral proof.
use super::{invalid,AnalysisDefinition,AnalysisInvocation,Interpretation,support::*,coverage::AnalysisCoverage,obligations::ObligationSubject};
use crate::domain::{assertion::{Assertion,AssertionQualification,Support},attribution::*,normalized::coverage::EvidenceAvailability,*};
use crate::{Domain,DomainCode};
use crate::domain::flow::*;
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
pub fn finding_policy(kind:FindingKind,status:EvidenceStatus)->Result<(),ModelError> {
    let navigation=matches!(kind,FindingKind::Community|FindingKind::Centrality|FindingKind::DocLink|FindingKind::CommunityLabel);
    let allowed=if navigation {EvidenceStatus::StatisticallyDerived} else {EvidenceStatus::StructurallyObserved};
    if status!=allowed && status!=EvidenceStatus::Unresolved {return Err(invalid("finding kind cannot use this evidence interpretation"));}
    Ok(())
}
/// Evidence is constructed from actual typed premises. The stored validator recomputes this
/// status recursively, so neither the emitter nor a producer accepts a desired status.
pub struct FindingEvidence<'a> {
    pub role:SupportRole,
    premise:QualifiedPremise<'a>,
    status:EvidenceStatus,
    heuristic:bool,
}
impl<'a> FindingEvidence<'a> {
    pub fn native<S:Support>(role:SupportRole,premise:&NativeAssertionPremise,source:&'a SupportSource,assertion:&S::Assertion,support:&S,qualification:&'a AssertionQualification,condition:&'a conditions::Diagram)->Result<Self,ModelError> {
        if source!=&(SupportSource::NativeAssertion {premise:premise.id()}) || premise.assertion_and_support()!=(derivation::RowRef::of(assertion.id()),derivation::RowRef::of(support.id())) || support.assertion()!=assertion.id() || assertion.qualification()!=qualification.id() || condition.id()!=qualification.condition {return Err(invalid("finding native evidence changes its nominal premise"));}
        let attribution=support.attribution().ok_or_else(||invalid("native evidence has no attribution"))?;
        let status=native_status(<S::Assertion as Assertion>::FAMILY,attribution.fidelity);
        Ok(Self {role,premise:QualifiedPremise {source,qualification,condition},status,heuristic:false})
    }
    pub fn derived(role:SupportRole,source:&'a SupportSource,derivation:&AnalysisDerivation,invocation:&AnalysisInvocation,definition:&AnalysisDefinition,qualification:&'a AssertionQualification,condition:&'a conditions::Diagram,supports:&[FindingEvidence<'_>],budget:&resources::ResourceBudget)->Result<Self,ModelError> {
        if source!=&(SupportSource::AnalysisDerivation {derivation:derivation.id()}) || derivation.invocation!=invocation.id() || invocation.definition!=definition.id() || derivation.qualification!=qualification.id() {return Err(invalid("finding derivation evidence changes invocation or qualification"));}
        let mut charge=charged::StateCharge::new(budget,"finding_evidence");let mut sources=charged::ChargedSet::default();
        for support in supports {if !sources.insert(&mut charge,support.premise.source.id())? {return Err(invalid("duplicate finding derivation premise"));}}
        if super::support::input_digest(&sources)!=derivation.inputs {return Err(invalid("finding derivation changes exact input membership"));}
        let _reservation=budget.reserve("finding_evidence",supports.len().checked_mul(size_of::<QualifiedPremise<'_>>()+size_of::<(SupportRole,EvidenceStatus)>()).ok_or_else(||invalid("finding evidence allocation overflow"))?)?;
        let premises=supports.iter().map(|s|s.premise).collect::<Vec<_>>();
        let result=qualify(derivation.operation,&premises)?;
        if result.qualification!=*qualification || condition.id()!=qualification.condition {return Err(invalid("finding derivation strengthens source evidence"));}
        let status=interpreted_status(definition.interpretation,derive_status(&supports.iter().map(|s|(SupportRole::Support,s.status)).collect::<Vec<_>>()));
        Ok(Self {role,premise:QualifiedPremise {source,qualification,condition},status,heuristic:definition.interpretation==Interpretation::Heuristic || supports.iter().any(|s|s.heuristic)})
    }
}
fn native_status(family:FactFamily,fidelity:Fidelity)->EvidenceStatus {
    if fidelity==Fidelity::DisplayOnly {EvidenceStatus::Unresolved} else if family==FactFamily::Docs {EvidenceStatus::Documented} else {EvidenceStatus::StructurallyObserved}
}
fn interpreted_status(interpretation:Interpretation,lower:EvidenceStatus)->EvidenceStatus {
    if lower==EvidenceStatus::Unresolved {lower} else if interpretation==Interpretation::Heuristic {EvidenceStatus::StatisticallyDerived} else {lower}
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="findings",invariants=finding_invariants)]
pub struct Finding {
    #[model(key)] pub invocation:Id<AnalysisInvocation>,
    #[model(key)] pub kind:FindingKind,
    #[model(key)] pub subject:Id<ObligationSubject>,
    #[model(key)] pub qualification:Id<AssertionQualification>,
    #[model(key)] pub coverage:Id<AnalysisCoverage>,
    #[model(key)] pub members:ContentHash,
    #[model(key)] pub supports:ContentHash,
    pub status:EvidenceStatus,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="finding_members")]
pub struct FindingMember {
    #[model(key)] pub finding:Id<Finding>,
    #[model(key)] pub role:MemberRole,
    #[model(key)] pub subject:Id<ObligationSubject>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="finding_supports",rule="finding_support",conclusion=finding)]
pub struct FindingSupport {
    #[model(key)] pub finding:Id<Finding>,
    #[model(key)] pub role:SupportRole,
    #[model(key,premise)] pub source:Id<SupportSource>,
}
fn member_digest(members:&std::collections::BTreeSet<(MemberRole,Id<ObligationSubject>)>)->ContentHash {
    let mut sink=KeySink::new("finding-members");for (role,subject) in members {role.encode(&mut sink);subject.encode(&mut sink);}sink.finish()
}
fn support_digest(supports:&std::collections::BTreeSet<(SupportRole,Id<SupportSource>)>)->ContentHash {
    let mut sink=KeySink::new("finding-supports");for (role,source) in supports {role.encode(&mut sink);source.encode(&mut sink);}sink.finish()
}
/// The one construction operation. Conditions and frame come from the shared qualification
/// join; completeness remains the coverage contract. Repeated evidence does not strengthen it.
pub fn emit(invocation:Id<AnalysisInvocation>,kind:FindingKind,subject:Id<ObligationSubject>,members:&[(MemberRole,Id<ObligationSubject>)],coverage:&AnalysisCoverage,evidence:&[FindingEvidence<'_>],budget:&resources::ResourceBudget)->Result<(Finding,Vec<FindingMember>,Vec<FindingSupport>,QualifiedResult),ModelError> {
    let mut charge=charged::StateCharge::new(budget,"finding_emitter");let mut member_set=charged::ChargedSet::default();let mut source_set=charged::ChargedSet::default();
    for member in members {member_set.insert(&mut charge,*member)?;}
    for support in evidence {if !source_set.insert(&mut charge,(support.role,support.premise.source.id()))? {return Err(invalid("duplicate finding evidence"));}}
    let _reservation=budget.reserve("finding_emitter",(members.len()+evidence.len()).checked_mul(size_of::<FindingMember>()+size_of::<FindingSupport>()+size_of::<QualifiedPremise<'_>>()+size_of::<(SupportRole,EvidenceStatus)>()).ok_or_else(||invalid("finding allocation overflow"))?)?;
    let qualification=qualify(QualificationOperation::Conjunction,&evidence.iter().map(|e|e.premise).collect::<Vec<_>>())?;
    if (coverage.invocation,coverage.scope,coverage.context)!=(invocation,qualification.qualification.scope,qualification.qualification.context) {return Err(invalid("finding coverage changes invocation or frame"));}
    let status=if matches!(coverage.availability,EvidenceAvailability::Unavailable|EvidenceAvailability::NotRequested|EvidenceAvailability::NoScope) {EvidenceStatus::Unresolved} else {derive_status(&evidence.iter().map(|e|(e.role,e.status)).collect::<Vec<_>>())};
    if evidence.iter().any(|e|e.heuristic) {finding_policy(kind,EvidenceStatus::StatisticallyDerived)?;}
    finding_policy(kind,status)?;
    let row=Finding {invocation,kind,subject,qualification:qualification.qualification.id(),coverage:coverage.id(),members:member_digest(&member_set),supports:support_digest(&source_set),status};
    let members=member_set.iter().map(|(role,subject)|FindingMember {finding:row.id(),role:*role,subject:*subject}).collect();
    let supports=source_set.iter().map(|(role,source)|FindingSupport {finding:row.id(),role:*role,source:*source}).collect();
    Ok((row,members,supports,qualification))
}

use crate::domain::calls::{Signature,SignatureSupport};
use crate::domain::calls::{CallTarget,CallTargetSupport};
use crate::domain::calls::{ProviderCallSite,ProviderCallSiteSupport};
use crate::domain::calls::{CallSyntax,CallSyntaxSupport};
use crate::domain::calls::{CallResolution,CallResolutionSupport};
use crate::domain::declarations::{SymbolDeclaration,SymbolDeclarationSupport};
use crate::domain::declarations::{ParameterDeclaration,ParameterDeclarationSupport};
use crate::domain::deployment::{TaskReportObservation,TaskReportSupport};
use crate::domain::deployment::{DeploymentObservation,DeploymentSupport};
use crate::domain::documents::{DocumentObservation,DocumentSupport};
use crate::domain::documents::{PassageObservation,PassageSupport};
use crate::domain::documents::{CodeBlockObservation,CodeBlockSupport};
use crate::domain::documents::{DocumentLinkObservation,DocumentLinkSupport};
use crate::domain::documents::{DocumentMentionObservation,DocumentMentionSupport};
use crate::domain::documents::{DocumentComponentObservation,DocumentComponentSupport};
use crate::domain::documents::{DocumentAttributeObservation,DocumentAttributeSupport};
use crate::domain::flow::{FlowAttributeLoadObservation,FlowAttributeLoadSupport};
use crate::domain::flow::{FlowValuePathObservation,FlowValuePathSupport};
use crate::domain::lexical::{LexicalScopeObservation,LexicalScopeSupport};
use crate::domain::lexical::{BindingObservation,BindingSupport};
use crate::domain::lexical::{ReferenceObservation,ReferenceSupport};
use crate::domain::lexical::{LexicalResolution,LexicalResolutionSupport};
use crate::domain::source::{SyntaxObservation,SyntaxSupport};
use crate::domain::symbols::{SymbolObservation,SymbolSupport};
use crate::domain::symbols::{FunctionTraitObservation,FunctionTraitSupport};
use crate::domain::symbols::{ClassTraitObservation,ClassTraitSupport};
use crate::domain::symbols::{ClassAncestryObservation,ClassAncestrySupport};
use crate::domain::symbols::{ParameterAnnotationObservation,ParameterAnnotationSupport};
use crate::domain::symbols::{PublicNameObservation,PublicNameSupport};
use crate::domain::symbols::{ParameterDocObservation,ParameterDocSupport};
use crate::domain::symbols::{DependencyModuleObservation,DependencyModuleSupport};
use crate::domain::syntax::{SyntaxPlacement,SyntaxPlacementSupport};
use crate::domain::syntax::{SyntaxDetailObservation,SyntaxDetailSupport};
use crate::domain::syntax::{DeclarationObservation,DeclarationSupport};
use crate::domain::syntax::{DeclarationDecorator,DeclarationDecoratorSupport};
use crate::domain::syntax::{ImportAliasObservation,ImportAliasSupport};
use crate::domain::syntax::{DunderAllObservation,DunderAllSupport};
use crate::domain::syntax::{ParameterSyntaxObservation,ParameterSyntaxSupport};
use crate::domain::syntax::{ClassFieldSyntaxObservation,ClassFieldSyntaxSupport};
use crate::domain::types::{TypeObservation,TypeSupport};
use crate::domain::types::{TypePresentation,TypePresentationSupport};
use crate::domain::types::{TypeVariableRestriction,TypeRestrictionSupport};
use crate::domain::types::{FunctionBodyObservation,FunctionBodySupport};
use crate::domain::types::{RecordFieldObservation,RecordFieldSupport};

fn finding_invariants()->Vec<Invariant> {
    let mut inputs=vec![ValidationInput::of::<Finding>(&["id"]),ValidationInput::of::<FindingMember>(&["id"]),ValidationInput::of::<FindingSupport>(&["id"]),ValidationInput::of::<AnalysisDefinition>(&["id"]),ValidationInput::of::<AnalysisInvocation>(&["id"]),ValidationInput::of::<AnalysisCoverage>(&["id"]),ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<conditions::Condition>(&["id"]),ValidationInput::of::<conditions::ConditionNode>(&["id"]),ValidationInput::of::<SupportSource>(&["id"]),ValidationInput::of::<NativeAssertionPremise>(&["id"]),ValidationInput::of::<AnalysisDerivation>(&["id"]),ValidationInput::of::<AnalysisDerivationPremise>(&["id"])];
    macro_rules! native_inputs { ($($a:ty=>$s:ty),*)=> {$(inputs.extend([ValidationInput::of::<$a>(&["id"]),ValidationInput::of::<$s>(&["id"])]);)*}; }
    native_inputs!(FlowUseObservation=>FlowUseSupport,FlowDefinitionObservation=>FlowDefinitionSupport,FlowReachingObservation=>FlowReachingSupport,FlowValueObservation=>FlowValueSupport,FlowRegionObservation=>FlowRegionSupport,FlowTestObservation=>FlowTestSupport,FlowTestLeafObservation=>FlowTestLeafSupport,Signature=>SignatureSupport,CallTarget=>CallTargetSupport,ProviderCallSite=>ProviderCallSiteSupport,CallSyntax=>CallSyntaxSupport,CallResolution=>CallResolutionSupport,SymbolDeclaration=>SymbolDeclarationSupport,ParameterDeclaration=>ParameterDeclarationSupport,TaskReportObservation=>TaskReportSupport,DeploymentObservation=>DeploymentSupport,DocumentObservation=>DocumentSupport,PassageObservation=>PassageSupport,CodeBlockObservation=>CodeBlockSupport,DocumentLinkObservation=>DocumentLinkSupport,DocumentMentionObservation=>DocumentMentionSupport,DocumentComponentObservation=>DocumentComponentSupport,DocumentAttributeObservation=>DocumentAttributeSupport,FlowAttributeLoadObservation=>FlowAttributeLoadSupport,FlowValuePathObservation=>FlowValuePathSupport,LexicalScopeObservation=>LexicalScopeSupport,BindingObservation=>BindingSupport,ReferenceObservation=>ReferenceSupport,LexicalResolution=>LexicalResolutionSupport,SyntaxObservation=>SyntaxSupport,SymbolObservation=>SymbolSupport,FunctionTraitObservation=>FunctionTraitSupport,ClassTraitObservation=>ClassTraitSupport,ClassAncestryObservation=>ClassAncestrySupport,ParameterAnnotationObservation=>ParameterAnnotationSupport,PublicNameObservation=>PublicNameSupport,ParameterDocObservation=>ParameterDocSupport,DependencyModuleObservation=>DependencyModuleSupport,SyntaxPlacement=>SyntaxPlacementSupport,SyntaxDetailObservation=>SyntaxDetailSupport,DeclarationObservation=>DeclarationSupport,DeclarationDecorator=>DeclarationDecoratorSupport,ImportAliasObservation=>ImportAliasSupport,DunderAllObservation=>DunderAllSupport,ParameterSyntaxObservation=>ParameterSyntaxSupport,ClassFieldSyntaxObservation=>ClassFieldSyntaxSupport,TypeObservation=>TypeSupport,TypePresentation=>TypePresentationSupport,TypeVariableRestriction=>TypeRestrictionSupport,FunctionBodyObservation=>FunctionBodySupport,RecordFieldObservation=>RecordFieldSupport);

    vec![Invariant {name:"finding_emitter",inputs,create:std::sync::Arc::new(|budget|Box::new(FindingCheck {charge:charged::StateCharge::new(budget,"finding_emitter"),findings:Default::default(),members:Default::default(),supports:Default::default(),definitions:Default::default(),invocations:Default::default(),coverage:Default::default(),qualifications:Default::default(),conditions:Default::default(),nodes:Default::default(),sources:Default::default(),native_premises:Default::default(),derivations:Default::default(),derivation_members:Default::default(),native:Default::default(),attributions:Default::default()}))}]
}
struct FindingCheck {
    charge:charged::StateCharge,
    findings:charged::ChargedMap<Id<Finding>,Finding>,
    members:charged::ChargedMap<Id<Finding>,std::collections::BTreeSet<(MemberRole,Id<ObligationSubject>)>>,
    supports:charged::ChargedMap<Id<Finding>,std::collections::BTreeSet<(SupportRole,Id<SupportSource>)>>,
    definitions:charged::ChargedMap<Id<AnalysisDefinition>,AnalysisDefinition>,
    invocations:charged::ChargedMap<Id<AnalysisInvocation>,AnalysisInvocation>,
    coverage:charged::ChargedMap<Id<AnalysisCoverage>,AnalysisCoverage>,
    qualifications:charged::ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    conditions:charged::ChargedMap<Id<conditions::Condition>,conditions::Condition>,
    nodes:charged::ChargedMap<Id<conditions::ConditionNode>,conditions::ConditionNode>,
    sources:charged::ChargedMap<Id<SupportSource>,SupportSource>,
    native_premises:charged::ChargedMap<Id<NativeAssertionPremise>,NativeAssertionPremise>,
    derivations:charged::ChargedMap<Id<AnalysisDerivation>,AnalysisDerivation>,
    derivation_members:charged::ChargedMap<Id<AnalysisDerivation>,std::collections::BTreeSet<Id<SupportSource>>>,
    native:charged::ChargedMap<derivation::RowRef,Id<AssertionQualification>>,
    attributions:charged::ChargedMap<derivation::RowRef,Fidelity>,
}
impl InvariantCheck for FindingCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        macro_rules! insert {($r:ty,$field:ident)=>{if relation==<$r>::NAME {for row in <$r>::decode(batch)? {if self.$field.insert(&mut self.charge,row.id(),row)?.is_some() {return Err(ModelError::Conflict(<$r>::NAME));}} return Ok(());}};}
        insert!(Finding,findings);insert!(AnalysisDefinition,definitions);insert!(AnalysisInvocation,invocations);insert!(AnalysisCoverage,coverage);insert!(AssertionQualification,qualifications);insert!(conditions::Condition,conditions);insert!(conditions::ConditionNode,nodes);insert!(SupportSource,sources);insert!(NativeAssertionPremise,native_premises);insert!(AnalysisDerivation,derivations);
        if relation==FindingMember::NAME {for row in FindingMember::decode(batch)? {if !self.members.update(&mut self.charge,row.finding,|m|m.insert((row.role,row.subject)))? {return Err(invalid("duplicate finding member"));}}return Ok(());}
        if relation==FindingSupport::NAME {for row in FindingSupport::decode(batch)? {if !self.supports.update(&mut self.charge,row.finding,|m|m.insert((row.role,row.source)))? {return Err(invalid("duplicate finding support"));}}return Ok(());}
        if relation==AnalysisDerivationPremise::NAME {for row in AnalysisDerivationPremise::decode(batch)? {self.derivation_members.update(&mut self.charge,row.derivation,|m|m.insert(row.source))?;}return Ok(());}
        macro_rules! visit_native {($($a:ty=>$s:ty),*)=>{$(
            if relation==<$a>::NAME {for row in <$a>::decode(batch)? {self.native.insert(&mut self.charge,derivation::RowRef::of(row.id()),row.qualification())?;}return Ok(());}
            if relation==<$s>::NAME {for row in <$s>::decode(batch)? {self.attributions.insert(&mut self.charge,derivation::RowRef::of(row.id()),row.attribution().ok_or_else(||invalid("native finding evidence has no attribution"))?.fidelity)?;}return Ok(());}
        )*};}
        visit_native!(FlowUseObservation=>FlowUseSupport,FlowDefinitionObservation=>FlowDefinitionSupport,FlowReachingObservation=>FlowReachingSupport,FlowValueObservation=>FlowValueSupport,FlowRegionObservation=>FlowRegionSupport,FlowTestObservation=>FlowTestSupport,FlowTestLeafObservation=>FlowTestLeafSupport,Signature=>SignatureSupport,CallTarget=>CallTargetSupport,ProviderCallSite=>ProviderCallSiteSupport,CallSyntax=>CallSyntaxSupport,CallResolution=>CallResolutionSupport,SymbolDeclaration=>SymbolDeclarationSupport,ParameterDeclaration=>ParameterDeclarationSupport,TaskReportObservation=>TaskReportSupport,DeploymentObservation=>DeploymentSupport,DocumentObservation=>DocumentSupport,PassageObservation=>PassageSupport,CodeBlockObservation=>CodeBlockSupport,DocumentLinkObservation=>DocumentLinkSupport,DocumentMentionObservation=>DocumentMentionSupport,DocumentComponentObservation=>DocumentComponentSupport,DocumentAttributeObservation=>DocumentAttributeSupport,FlowAttributeLoadObservation=>FlowAttributeLoadSupport,FlowValuePathObservation=>FlowValuePathSupport,LexicalScopeObservation=>LexicalScopeSupport,BindingObservation=>BindingSupport,ReferenceObservation=>ReferenceSupport,LexicalResolution=>LexicalResolutionSupport,SyntaxObservation=>SyntaxSupport,SymbolObservation=>SymbolSupport,FunctionTraitObservation=>FunctionTraitSupport,ClassTraitObservation=>ClassTraitSupport,ClassAncestryObservation=>ClassAncestrySupport,ParameterAnnotationObservation=>ParameterAnnotationSupport,PublicNameObservation=>PublicNameSupport,ParameterDocObservation=>ParameterDocSupport,DependencyModuleObservation=>DependencyModuleSupport,SyntaxPlacement=>SyntaxPlacementSupport,SyntaxDetailObservation=>SyntaxDetailSupport,DeclarationObservation=>DeclarationSupport,DeclarationDecorator=>DeclarationDecoratorSupport,ImportAliasObservation=>ImportAliasSupport,DunderAllObservation=>DunderAllSupport,ParameterSyntaxObservation=>ParameterSyntaxSupport,ClassFieldSyntaxObservation=>ClassFieldSyntaxSupport,TypeObservation=>TypeSupport,TypePresentation=>TypePresentationSupport,TypeVariableRestriction=>TypeRestrictionSupport,FunctionBodyObservation=>FunctionBodySupport,RecordFieldObservation=>RecordFieldSupport);

        Err(invalid("undeclared finding emitter input"))
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        if self.findings.is_empty() {if !self.members.is_empty() || !self.supports.is_empty() {return Err(invalid("orphan finding membership"));}return Ok(());}
        let budget=self.charge.budget().ok_or_else(||invalid("finding budget absent"))?;
        let mut charge=charged::StateCharge::new(budget,"finding_source_interpretation");
        let mut interpreted=charged::ChargedMap::<Id<SupportSource>,(Id<AssertionQualification>,EvidenceStatus,bool)>::default();
        loop {
            let before=interpreted.len();
            for (id,source) in self.sources.iter() {
                if interpreted.contains_key(id) {continue;}
                let value=match source {
                    SupportSource::NativeAssertion {premise}=>{
                        let premise=self.native_premises.get(premise).ok_or_else(||invalid("finding native premise absent"))?;
                        let (assertion,support)=premise.assertion_and_support();
                        let q=*self.native.get(&assertion).ok_or_else(||invalid("finding native assertion absent"))?;
                        let fidelity=*self.attributions.get(&support).ok_or_else(||invalid("finding native support absent"))?;
                        Some((q,native_status(premise.family(),fidelity),false))
                    },
                    SupportSource::AnalysisDerivation {derivation}=>{
                        let derivation=self.derivations.get(derivation).ok_or_else(||invalid("finding analysis derivation absent"))?;
                        let members=self.derivation_members.get(&derivation.id()).ok_or_else(||invalid("finding derivation membership absent"))?;
                        if !members.iter().all(|m|interpreted.contains_key(m)) {None} else {
                            let invocation=self.invocations.get(&derivation.invocation).ok_or_else(||invalid("finding source invocation absent"))?;
                            let definition=self.definitions.get(&invocation.definition).ok_or_else(||invalid("finding source definition absent"))?;
                            let mut status=EvidenceStatus::StructurallyObserved;
                            for member in members {
                                let lower=interpreted[member].1;
                                status=derive_status(&[(SupportRole::Support,status),(SupportRole::Support,lower)]);
                            }
                            Some((derivation.qualification,interpreted_status(definition.interpretation,status),definition.interpretation==Interpretation::Heuristic || members.iter().any(|m|interpreted[m].2)))
                        }
                    },
                };
                if let Some(value)=value {interpreted.insert(&mut charge,*id,value)?;}
            }
            if interpreted.len()==self.sources.len() {break;}
            if interpreted.len()==before {return Err(invalid("finding interpretation has a cyclic or missing source"));}
        }
        let node_bytes=self.nodes.values().try_fold(0usize,|n,row|n.checked_add(size_of::<conditions::ConditionNode>()+row.heap_bytes()+128).ok_or_else(||invalid("finding node allocation overflow")))?;
        let empty=std::collections::BTreeSet::new();
        for (id,row) in self.findings.iter() {
            let members=self.members.get(id).unwrap_or(&empty);
            let supports=self.supports.get(id).ok_or_else(||invalid("finding evidence absent"))?;
            if member_digest(members)!=row.members || support_digest(supports)!=row.supports {return Err(invalid("finding exact membership differs"));}
            let bytes=node_bytes.checked_mul(supports.len()+1).and_then(|n|n.checked_add(supports.len().checked_mul(size_of::<QualifiedPremise<'_>>()+size_of::<FindingEvidence<'_>>()+size_of::<conditions::Diagram>())?)).ok_or_else(||invalid("finding allocation overflow"))?;
            let _reservation=budget.reserve("finding_emitter_recheck",bytes)?;
            let nodes=self.nodes.values().cloned().collect::<Vec<_>>();let mut diagrams=Vec::with_capacity(supports.len());let mut sources=Vec::with_capacity(supports.len());
            for (role,source) in supports {
                let (q,status,heuristic)=*interpreted.get(source).ok_or_else(||invalid("finding source interpretation absent"))?;
                let q=self.qualifications.get(&q).ok_or_else(||invalid("finding source qualification absent"))?;
                let condition=self.conditions.get(&q.condition).ok_or_else(||invalid("finding source condition absent"))?;
                diagrams.push(conditions::Diagram::from_records(condition,&nodes)?);
                sources.push((*role,self.sources.get(source).unwrap(),q,status,heuristic));
            }
            let evidence=sources.iter().zip(&diagrams).map(|((role,source,q,status,heuristic),diagram)|FindingEvidence {role:*role,premise:QualifiedPremise {source,qualification:q,condition:diagram},status:*status,heuristic:*heuristic}).collect::<Vec<_>>();
            let coverage=self.coverage.get(&row.coverage).ok_or_else(||invalid("finding coverage absent"))?;
            let emitted=emit(row.invocation,row.kind,row.subject,&members.iter().copied().collect::<Vec<_>>(),coverage,&evidence,budget)?.0;
            if emitted!=*row {return Err(invalid("finding producer selected status or qualification differs"));}
        }
        for (id,_) in self.members.iter() {if !self.findings.contains_key(id) {return Err(invalid("orphan finding member"));}}
        for (id,_) in self.supports.iter() {if !self.findings.contains_key(id) {return Err(invalid("orphan finding support"));}}
        Ok(())
    }
}
pub fn relations()->Vec<Relation> {vec![Relation::of::<Finding>(),Relation::of::<FindingMember>(),Relation::of::<FindingSupport>()]}
