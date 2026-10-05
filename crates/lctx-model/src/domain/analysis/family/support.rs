use super::*;
use crate::domain::analysis::{native::NativeQualification,policy::EvidenceStatus,support::{SourceFacts,inferred_status}};
pub use crate::domain::analysis::support::{QualificationOperation,QualifiedResult,qualify_with_basis};
pub use super::SupportSource;
pub type QualifiedPremise<'a>=crate::domain::analysis::support::QualifiedPremise<'a,SupportSource>;
pub fn qualify(operation:QualificationOperation,premises:&[QualifiedPremise<'_>],budget:&resources::ResourceBudget)->Result<QualifiedResult,ModelError> {crate::domain::analysis::support::qualify(operation,premises,budget)}
#[derive(Debug,Clone,PartialEq,Eq,Domain, serde::Serialize, serde::Deserialize)]
#[model(name=owner_table!("analysis_propositions"))]
pub struct AnalysisProposition {
    #[model(key)] pub subject:Id<ObligationSubject>,
    #[model(key)] pub channel:AnalysisChannel,
    #[model(key)] pub phase:CallPhase,
    #[model(key)] pub qualification:Id<AssertionQualification>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain, serde::Serialize, serde::Deserialize)]
#[model(name=owner_table!("analysis_derivations"),invariant_refs=support_invariants_refs)]
pub struct AnalysisDerivation {
    #[model(key)] pub invocation:Id<AnalysisInvocation>,
    #[model(key)] pub proposition:Id<AnalysisProposition>,
    #[model(key)] pub qualification:Id<AssertionQualification>,
    #[model(key)] pub operation:QualificationOperation,
    #[model(key)] pub inputs:ContentHash,
    /// Recomputed by shared source policy; these payloads are not producer-selected fidelity.
    pub status:EvidenceStatus,
    pub heuristic:bool,
}
impl AnalysisDerivation {pub fn facts(&self)->SourceFacts {SourceFacts {qualification:self.qualification,status:self.status,heuristic:self.heuristic}}}
#[derive(Debug,Clone,PartialEq,Eq,Domain, serde::Serialize, serde::Deserialize)]
#[model(name=owner_table!("analysis_derivation_premises"),rule="qualified_analysis_derivation",conclusion=derivation)]
pub struct AnalysisDerivationPremise {#[model(key)] pub derivation:Id<AnalysisDerivation>,#[model(key,premise)] pub source:Id<SupportSource>}
/// A source's status is obtained only from its typed native projection or immutable proof.
pub struct EvidencePremise<'a> {pub(crate) premise:QualifiedPremise<'a>,pub(crate) facts:SourceFacts}
impl<'a> EvidencePremise<'a> {
    pub fn native(source:&'a SupportSource,native:&NativeQualification,qualification:&'a AssertionQualification,condition:&'a conditions::Diagram)->Result<Self,ModelError> {
        if source!=&(SupportSource::NativeAssertion {premise:native.premise}) {return Err(invalid("evidence changes nominal native pair"));}
        Self::checked(source,SourceFacts {qualification:native.qualification,status:native.status,heuristic:false},qualification,condition)
    }
    pub fn derived<R:crate::domain::analysis::support::DerivedEvidence>(source:&'a SupportSource,row:&R,qualification:&'a AssertionQualification,condition:&'a conditions::Diagram)->Result<Self,ModelError> {
        if source.reference()!=derivation::RowRef::of(row.id()) {return Err(invalid("evidence changes nominal derivation"));}
        Self::checked(source,row.source_facts(),qualification,condition)
    }
    fn checked(source:&'a SupportSource,facts:SourceFacts,qualification:&'a AssertionQualification,condition:&'a conditions::Diagram)->Result<Self,ModelError> {
        if facts.qualification!=qualification.id() || condition.id()!=qualification.condition {return Err(invalid("evidence changes qualification/condition"));}
        Ok(Self {premise:QualifiedPremise {source,qualification,condition},facts})
    }
}
impl crate::domain::analysis::support::sealed::DerivedEvidence for AnalysisDerivation {}
impl crate::domain::analysis::support::DerivedEvidence for AnalysisDerivation {fn source_facts(&self)->SourceFacts {self.facts()}}
pub(super) fn input_digest(sources:&std::collections::BTreeSet<Id<SupportSource>>)->ContentHash {crate::domain::analysis::support::membership_digest("analysis-derivation-premises",sources)}
impl AnalysisDerivation {
    #[allow(clippy::too_many_arguments,reason="Derivation emission binds the invocation, definition, subject, channel, phase, operation and premises")]
    pub fn emit(invocation:&AnalysisInvocation,definition:&AnalysisDefinition,subject:Id<ObligationSubject>,channel:AnalysisChannel,phase:CallPhase,operation:QualificationOperation,premises:&[EvidencePremise<'_>],budget:&resources::ResourceBudget)->Result<(Self,AnalysisProposition,Vec<AnalysisDerivationPremise>,QualifiedResult),ModelError> {
        Self::emit_with_basis(invocation,definition,subject,channel,phase,operation,premises,None,budget)
    }
    #[allow(clippy::too_many_arguments, reason = "Derivation emission keeps the nominal question, evidence and assumption resolver distinct.")]
    pub fn emit_with_basis(invocation:&AnalysisInvocation,definition:&AnalysisDefinition,subject:Id<ObligationSubject>,channel:AnalysisChannel,phase:CallPhase,operation:QualificationOperation,premises:&[EvidencePremise<'_>],basis:Option<&dyn assumptions::AssumptionResolver>,budget:&resources::ResourceBudget)->Result<(Self,AnalysisProposition,Vec<AnalysisDerivationPremise>,QualifiedResult),ModelError> {
        if invocation.definition!=definition.id() {return Err(invalid("derivation changes analysis definition"));}
        let mut charge=charged::StateCharge::new(budget,"analysis_derivation_emit");let mut sources=charged::ChargedSet::default();
        for premise in premises {if !sources.insert(&mut charge,premise.premise.source.id())? {return Err(invalid("duplicate qualified derivation source"));}}
        let _buffer=budget.reserve("analysis_derivation_emit",premises.len().checked_mul(size_of::<QualifiedPremise<'_>>()+size_of::<AnalysisDerivationPremise>()).ok_or_else(||invalid("derivation buffer overflow"))?)?;
        let qualified=premises.iter().map(|p|QualifiedPremise {source:p.premise.source,qualification:p.premise.qualification,condition:p.premise.condition}).collect::<Vec<_>>();
        let result=qualify_with_basis(operation,&qualified,basis,budget)?;
        if result.qualification.context!=invocation.context {return Err(invalid("derivation qualification crosses invocation"));}
        let proposition=AnalysisProposition {subject,channel,phase,qualification:result.qualification.id()};
        let row=Self {invocation:invocation.id(),proposition:proposition.id(),qualification:result.qualification.id(),operation,inputs:input_digest(&sources),status:inferred_status(definition.interpretation,premises.iter().map(|p|p.facts.status)),heuristic:definition.interpretation==Interpretation::Heuristic || premises.iter().any(|p|p.facts.heuristic)};
        let members=sources.iter().map(|source|AnalysisDerivationPremise {derivation:row.id(),source:*source}).collect();
        Ok((row,proposition,members,result))
    }
}
fn support_inputs()->Vec<ValidationInput> {let mut inputs=vec![ValidationInput::of::<AnalysisProposition>(&["id"]),ValidationInput::of::<AnalysisInvocation>(&["id"]),ValidationInput::of::<AnalysisDefinition>(&["id"]),ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<conditions::Condition>(&["id"]),ValidationInput::of::<conditions::ConditionNode>(&["id"]),ValidationInput::of::<NativeQualification>(&["id"]),ValidationInput::of::<AnalysisDerivation>(&["id"]),ValidationInput::of::<SupportSource>(&["id"]),ValidationInput::of::<AnalysisDerivationPremise>(&["id"])];predecessor_support_inputs(&mut inputs);inputs.extend(assumptions::AssumptionIndex::inputs());for input in ownership::ScopeIndex::inputs() {if !inputs.iter().any(|r|r.name()==input.name()) {inputs.push(input);}}inputs}
pub(crate) fn support_invariants()->Vec<Invariant> {vec![Invariant {revision: 1,name:owner_table!("qualified_derivation"),inputs:support_inputs(),create:std::sync::Arc::new(|budget|Box::new(SupportCheck::new(budget)))}]}
struct SupportCheck {
    charge:charged::StateCharge,
    ownership:ownership::ScopeIndex,
    assumptions:assumptions::AssumptionIndex,
    propositions:charged::ChargedMap<Id<AnalysisProposition>,AnalysisProposition>,
    invocations:charged::ChargedMap<Id<AnalysisInvocation>,AnalysisInvocation>,
    definitions:charged::ChargedMap<Id<AnalysisDefinition>,AnalysisDefinition>,
    qualifications:charged::ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    conditions:charged::ChargedMap<Id<conditions::Condition>,conditions::Condition>,
    nodes:charged::ChargedMap<Id<conditions::ConditionNode>,conditions::ConditionNode>,
    facts:charged::ChargedMap<derivation::RowRef,SourceFacts>,
    sources:charged::ChargedMap<Id<SupportSource>,SupportSource>,
    derivations:charged::ChargedMap<Id<AnalysisDerivation>,AnalysisDerivation>,
    members:charged::ChargedMap<Id<AnalysisDerivation>,std::collections::BTreeSet<Id<SupportSource>>>,
}
impl SupportCheck {
    fn new(budget:&resources::ResourceBudget)->Self {Self {charge:charged::StateCharge::new(budget,"qualified_analysis_derivation"),ownership:ownership::ScopeIndex::new(budget,"qualified_analysis_derivation"),assumptions:assumptions::AssumptionIndex::new(budget),propositions:Default::default(),invocations:Default::default(),definitions:Default::default(),qualifications:Default::default(),conditions:Default::default(),nodes:Default::default(),facts:Default::default(),sources:Default::default(),derivations:Default::default(),members:Default::default()}}
    fn source_facts(&self,source:&SupportSource)->Result<SourceFacts,ModelError> {match source {SupportSource::AnalysisDerivation {derivation}=>self.derivations.get(derivation).map(|r|r.facts()),_=>self.facts.get(&source.reference()).copied()}.ok_or_else(||invalid("support source evidence absent"))}
}
impl InvariantCheck for SupportCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        if self.assumptions.visit(relation,batch)? {return Ok(());}
        if self.ownership.visit(relation,batch)? {return Ok(());}
        macro_rules! insert {($r:ty,$field:ident)=>{if relation==<$r>::NAME {for row in <$r>::decode(batch)? {if self.$field.insert(&mut self.charge,row.id(),row)?.is_some() {return Err(ModelError::Conflict(<$r>::NAME));}}return Ok(());}};}
        insert!(AnalysisProposition,propositions);insert!(AnalysisInvocation,invocations);insert!(AnalysisDefinition,definitions);insert!(AssertionQualification,qualifications);insert!(conditions::Condition,conditions);insert!(conditions::ConditionNode,nodes);insert!(SupportSource,sources);insert!(AnalysisDerivation,derivations);
        if relation==NativeQualification::NAME {for row in NativeQualification::decode(batch)? {self.facts.insert(&mut self.charge,derivation::RowRef::of(row.premise),SourceFacts {qualification:row.qualification,status:row.status,heuristic:false})?;}return Ok(());}
        if relation==AnalysisDerivationPremise::NAME {for row in AnalysisDerivationPremise::decode(batch)? {if !self.members.update(&mut self.charge,row.derivation,|m|m.insert(row.source))? {return Err(invalid("duplicate derivation premise"));}}return Ok(());}
        if visit_predecessor_support(relation,batch,&mut self.facts,&mut self.charge)? {return Ok(());}
        Err(invalid("undeclared qualified derivation input"))
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        let budget=self.charge.budget().ok_or_else(||invalid("qualification budget absent"))?;
        for (id,row) in self.derivations.iter() {
            let invocation=self.invocations.get(&row.invocation).ok_or_else(||invalid("derived invocation absent"))?;
            let definition=self.definitions.get(&invocation.definition).ok_or_else(||invalid("derived definition absent"))?;
            let q=self.qualifications.get(&row.qualification).ok_or_else(||invalid("derived qualification absent"))?;
            if self.propositions.get(&row.proposition).ok_or_else(||invalid("derived proposition absent"))?.qualification!=q.id() || q.context!=invocation.context || !self.ownership.owns_scope(invocation.input,self.ownership.scope(q.scope)?)? {return Err(invalid("derived proposition changes qualification/frame"));}
            let members=self.members.get(id).ok_or_else(||invalid("derived membership absent"))?;
            if input_digest(members)!=row.inputs {return Err(invalid("derived membership digest differs"));}
            let node_bytes=self.nodes.values().try_fold(0usize,|n,r|n.checked_add(size_of::<conditions::ConditionNode>()+r.heap_bytes()+128).ok_or_else(||invalid("qualification node buffer overflow")))?;
            let bytes=node_bytes.checked_mul(members.len()+1).and_then(|n|n.checked_add(members.len().checked_mul(size_of::<QualifiedPremise<'_>>()+size_of::<conditions::Diagram>()+size_of::<SourceFacts>())?)).ok_or_else(||invalid("qualification buffer overflow"))?;
            let _buffer=budget.reserve("qualified_analysis_derivation",bytes)?;
            let nodes=self.nodes.values().cloned().collect::<Vec<_>>();let mut diagrams=Vec::with_capacity(members.len());let mut evidence=Vec::with_capacity(members.len());
            for member in members {let source=self.sources.get(member).ok_or_else(||invalid("derivation source absent"))?;let facts=self.source_facts(source)?;let q=self.qualifications.get(&facts.qualification).ok_or_else(||invalid("source qualification absent"))?;let condition=self.conditions.get(&q.condition).ok_or_else(||invalid("source condition absent"))?;diagrams.push(conditions::Diagram::from_records(condition,&nodes)?);evidence.push((source,q,facts));}
            let premises=evidence.iter().zip(&diagrams).map(|((source,q,_),condition)|QualifiedPremise {source,qualification:q,condition}).collect::<Vec<_>>();
            if qualify_with_basis(row.operation,&premises,Some(&self.assumptions),budget)?.qualification!=*q || row.status!=inferred_status(definition.interpretation,evidence.iter().map(|(_,_,f)|f.status)) || row.heuristic!=(definition.interpretation==Interpretation::Heuristic || evidence.iter().any(|(_,_,f)|f.heuristic)) {return Err(invalid("derivation strengthens qualification or evidence lineage"));}
        }
        for id in self.members.keys() {if !self.derivations.contains_key(id) {return Err(invalid("orphan derivation membership"));}}
        for source in self.sources.values() {self.source_facts(source)?;}
        Ok(())
    }
}
pub fn relations()->Vec<Relation> {vec![Relation::of::<AnalysisProposition>(),Relation::of::<SupportSource>(),Relation::of::<AnalysisDerivation>(),Relation::of::<AnalysisDerivationPremise>()]}
/// Only this publication's derivation can support its generated assertions. A predecessor proof
/// first participates in a new local derivation with the owner's interpretation and frame.
impl assertion::DerivedSupportSource for SupportSource {
    fn inputs()->Vec<ValidationInput> {vec![ValidationInput::of::<SupportSource>(&["id"]),ValidationInput::of::<AnalysisDerivation>(&["id"]),ValidationInput::of::<AnalysisInvocation>(&["id"])]}
    fn index(budget:&resources::ResourceBudget)->Box<dyn assertion::DerivedSupportIndex<Self>> {Box::new(CompanionIndex {charge:charged::StateCharge::new(budget,"derived_companion"),sources:Default::default(),derivations:Default::default(),invocations:Default::default()})}
}
struct CompanionIndex {charge:charged::StateCharge,sources:charged::ChargedMap<Id<SupportSource>,SupportSource>,derivations:charged::ChargedMap<Id<AnalysisDerivation>,AnalysisDerivation>,invocations:charged::ChargedMap<Id<AnalysisInvocation>,AnalysisInvocation>}
impl assertion::DerivedSupportIndex<SupportSource> for CompanionIndex {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {
        macro_rules! insert {($r:ty,$field:ident)=>{if relation==<$r>::NAME {for row in <$r>::decode(batch)? {self.$field.insert(&mut self.charge,row.id(),row)?;}return Ok(true);}};}
        insert!(SupportSource,sources);insert!(AnalysisDerivation,derivations);insert!(AnalysisInvocation,invocations);Ok(false)
    }
    fn frame(&self,source:Id<SupportSource>)->Result<assertion::DerivedSupportFrame,ModelError> {
        let source=self.sources.get(&source).ok_or_else(||invalid("derived support source absent"))?;
        let SupportSource::AnalysisDerivation {derivation}=source else {return Err(invalid("generated assertion requires its owner's derivation"));};
        let row=self.derivations.get(derivation).ok_or_else(||invalid("support derivation absent"))?;
        let invocation=self.invocations.get(&row.invocation).ok_or_else(||invalid("support invocation absent"))?;
        Ok(assertion::DerivedSupportFrame {input:invocation.input,context:invocation.context,qualification:row.qualification,evidence:row.facts()})
    }
}
/// Read-only projection from already validated nominal source rows. Each concrete owner supplies
/// its finite predecessor adapters; inference remains the shared qualification/status policy.
pub struct EvidenceIndex {pub assumptions:assumptions::AssumptionIndex,charge:charged::StateCharge,sources:charged::ChargedMap<Id<SupportSource>,SupportSource>,facts:charged::ChargedMap<derivation::RowRef,SourceFacts>}
impl EvidenceIndex {
    pub fn new(budget:&resources::ResourceBudget)->Self {Self {assumptions:assumptions::AssumptionIndex::new(budget),charge:charged::StateCharge::new(budget,"analysis_source_index"),sources:Default::default(),facts:Default::default()}}
    pub fn inputs()->Vec<ValidationInput> {let mut inputs=vec![ValidationInput::of::<SupportSource>(&["id"]),ValidationInput::of::<NativeQualification>(&["id"]),ValidationInput::of::<AnalysisDerivation>(&["id"])];predecessor_support_inputs(&mut inputs);inputs.extend(assumptions::AssumptionIndex::inputs());inputs}
    pub fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {
        if self.assumptions.visit(relation,batch)? {return Ok(true);}
        if relation==SupportSource::NAME {for row in SupportSource::decode(batch)? {self.sources.insert(&mut self.charge,row.id(),row)?;}return Ok(true);}
        if relation==NativeQualification::NAME {for row in NativeQualification::decode(batch)? {self.facts.insert(&mut self.charge,derivation::RowRef::of(row.premise),SourceFacts {qualification:row.qualification,status:row.status,heuristic:false})?;}return Ok(true);}
        if relation==AnalysisDerivation::NAME {for row in AnalysisDerivation::decode(batch)? {self.facts.insert(&mut self.charge,derivation::RowRef::of(row.id()),row.facts())?;}return Ok(true);}
        visit_predecessor_support(relation,batch,&mut self.facts,&mut self.charge)
    }
    pub fn get(&self,id:Id<SupportSource>)->Result<(&SupportSource,SourceFacts),ModelError> {let source=self.sources.get(&id).ok_or_else(||invalid("nominal support source absent"))?;let facts=*self.facts.get(&source.reference()).ok_or_else(||invalid("immutable source evidence absent"))?;Ok((source,facts))}
}

pub(crate) fn support_invariants_refs() -> Vec<&'static str> { vec![owner_table!("qualified_derivation")] }
