use super::*;
use crate::domain::analysis::obligation_support::{self, ObligationQuestion, Question};
use crate::domain::derivation::RowRef;
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name=owner_table!("analysis_obligations"), validate = validate_obligation)]
pub struct AnalysisObligation {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub subject: Id<ObligationSubject>,
    #[model(key)]
    pub channel: AnalysisChannel,
    #[model(key)]
    pub phase: CallPhase,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub reason: ObligationKind,
    #[model(key)] pub responsible: AnalysisMethod,
}
fn validate_obligation(row: &AnalysisObligation) -> Result<(), ModelError> {
    if row.reason == ObligationKind::ResponseBudget {
        return Err(invalid("response truncation is not an analysis obligation"));
    }
    Ok(())
}
impl ObligationQuestion for AnalysisObligation {
    fn question(&self)->Question {Question {invocation:RowRef::of(self.invocation),subject:RowRef::of(self.subject),channel:self.channel,phase:self.phase,qualification:self.qualification}}
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<AnalysisObligation>(),
        Relation::of::<DischargeEvidence>(),
    ]
}

/// A persisted discharge cites an exact subject proof. There is no producer-selected verdict;
/// publication reconstructs it with the shared conservative verdict operation.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name=owner_table!("analysis_discharge_evidence"),rule="analysis_discharge",invariants=discharge_invariants)]
pub struct DischargeEvidence {
    #[model(key,premise)] pub obligation:Id<ObligationSource>,
    #[model(key,premise)] pub derivation:Id<AnalysisDerivation>,
    #[model(key,premise)] pub coverage:Id<AnalysisCoverage>,
}
pub fn admissible_discharge(obligation:&AnalysisObligation,proposition:&AnalysisProposition,qualification:&AssertionQualification,condition:&conditions::Diagram,coverage:&AnalysisCoverage)->Result<obligation::Conclusion,ModelError> {
    obligation_support::admissible(&obligation.question(),&Question {
        invocation:RowRef::of(obligation.invocation),subject:RowRef::of(proposition.subject),channel:proposition.channel,phase:proposition.phase,qualification:proposition.qualification,
    },RowRef::of(proposition.id()),qualification,condition,(coverage.scope,coverage.context,coverage.availability))
}

fn discharge_inputs()->Vec<ValidationInput> {
    let mut inputs=vec![ValidationInput::of::<AnalysisObligation>(&["id"]),ValidationInput::of::<ObligationSubject>(&["id"]),ValidationInput::of::<ObligationSource>(&["id"]),ValidationInput::of::<AnalysisInvocation>(&["id"]),ValidationInput::of::<DischargeEvidence>(&["id"]),ValidationInput::of::<AnalysisDerivation>(&["id"]),ValidationInput::of::<AnalysisProposition>(&["id"]),ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<AnalysisCoverage>(&["id"]),ValidationInput::of::<conditions::Condition>(&["id"]),ValidationInput::of::<conditions::ConditionNode>(&["id"])];
    predecessor_obligation_inputs(&mut inputs);
    inputs.extend(ownership::ScopeIndex::inputs());
    inputs
}
fn discharge_invariants()->Vec<Invariant> {
    vec![Invariant {name:owner_table!("analysis_discharge"),inputs:discharge_inputs(),create:std::sync::Arc::new(|budget|Box::new(DischargeCheck {
        charge:charged::StateCharge::new(budget,"analysis_discharge"),ownership:ownership::ScopeIndex::new(budget,"analysis_discharge"),
        obligations:Default::default(),subjects:Default::default(),sources:Default::default(),frames:Default::default(),evidence:Default::default(),derivations:Default::default(),propositions:Default::default(),qualifications:Default::default(),coverage:Default::default(),conditions:Default::default(),nodes:Default::default(),
    }))}]
}
struct DischargeCheck {
    charge:charged::StateCharge,
    ownership:ownership::ScopeIndex,
    obligations:charged::ChargedMap<RowRef,Question>,
    subjects:charged::ChargedMap<RowRef,RowRef>,
    sources:charged::ChargedMap<Id<ObligationSource>,ObligationSource>,
    frames:charged::ChargedMap<RowRef,(Id<InputRevision>,Id<AnalysisContext>)>,
    evidence:charged::ChargedMap<Id<DischargeEvidence>,DischargeEvidence>,
    derivations:charged::ChargedMap<Id<AnalysisDerivation>,AnalysisDerivation>,
    propositions:charged::ChargedMap<Id<AnalysisProposition>,AnalysisProposition>,
    qualifications:charged::ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    coverage:charged::ChargedMap<Id<AnalysisCoverage>,AnalysisCoverage>,
    conditions:charged::ChargedMap<Id<conditions::Condition>,conditions::Condition>,
    nodes:charged::ChargedMap<Id<conditions::ConditionNode>,conditions::ConditionNode>,
}
impl InvariantCheck for DischargeCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        if self.ownership.visit(relation,batch)? {return Ok(());}
        if relation==AnalysisObligation::NAME {for row in AnalysisObligation::decode(batch)? {self.obligations.insert(&mut self.charge,RowRef::of(row.id()),row.question())?;}return Ok(());}
        if relation==ObligationSubject::NAME {for row in ObligationSubject::decode(batch)? {self.subjects.insert(&mut self.charge,RowRef::of(row.id()),row.reference())?;}return Ok(());}
        if relation==AnalysisInvocation::NAME {for row in AnalysisInvocation::decode(batch)? {self.frames.insert(&mut self.charge,RowRef::of(row.id()),(row.input,row.context))?;}return Ok(());}
        if visit_predecessor_obligations(relation,batch,&mut self.obligations,&mut self.subjects,&mut self.charge)? || visit_predecessor_invocation(relation,batch,&mut self.frames,&mut self.charge)? {return Ok(());}
        macro_rules! insert { ($r:ty,$field:ident)=>{ if relation==<$r>::NAME { for row in <$r>::decode(batch)? { if self.$field.insert(&mut self.charge,row.id(),row)?.is_some() {return Err(ModelError::Conflict(<$r>::NAME));} } return Ok(()); } }; }
        insert!(ObligationSource,sources);insert!(DischargeEvidence,evidence);insert!(AnalysisDerivation,derivations);insert!(AnalysisProposition,propositions);insert!(AssertionQualification,qualifications);insert!(AnalysisCoverage,coverage);insert!(conditions::Condition,conditions);insert!(conditions::ConditionNode,nodes);
        Err(invalid("undeclared discharge input"))
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        let bytes=self.nodes.values().try_fold(0usize,|n,row|n.checked_add(size_of::<conditions::ConditionNode>()+row.heap_bytes()+128).ok_or_else(||invalid("discharge allocation overflow")))?;
        let _reservation=self.charge.budget().ok_or_else(||invalid("discharge budget absent"))?.reserve("analysis_discharge",bytes)?;
        let nodes=self.nodes.values().cloned().collect::<Vec<_>>();
        for (_,evidence) in self.evidence.iter() {
            let source=self.sources.get(&evidence.obligation).ok_or_else(||invalid("obligation source absent"))?;
            let mut question=*self.obligations.get(&source.reference()).ok_or_else(||invalid("discharged obligation absent"))?;
            question.subject=*self.subjects.get(&question.subject).ok_or_else(||invalid("discharged subject absent"))?;
            let derivation=self.derivations.get(&evidence.derivation).ok_or_else(||invalid("discharging derivation absent"))?;
            let proposition=self.propositions.get(&derivation.proposition).ok_or_else(||invalid("discharging proposition absent"))?;
            let qualification=self.qualifications.get(&proposition.qualification).ok_or_else(||invalid("discharging qualification absent"))?;
            let condition=self.conditions.get(&qualification.condition).ok_or_else(||invalid("discharging condition absent"))?;
            let coverage=self.coverage.get(&evidence.coverage).ok_or_else(||invalid("discharging coverage absent"))?;
            let frame=self.frames.get(&question.invocation).ok_or_else(||invalid("obligation invocation absent"))?;
            let proof_frame=self.frames.get(&RowRef::of(derivation.invocation)).ok_or_else(||invalid("proof invocation absent"))?;
            if frame!=proof_frame || frame.1!=qualification.context || !self.ownership.owns_scope(frame.0,self.ownership.scope(qualification.scope)?)? || derivation.qualification!=qualification.id() || derivation.invocation!=coverage.invocation || derivation.heuristic {return Err(invalid("discharging proof changes invocation frame or uses heuristic evidence"));}
            let proof=Question {invocation:RowRef::of(derivation.invocation),subject:*self.subjects.get(&RowRef::of(proposition.subject)).ok_or_else(||invalid("proof subject absent"))?,channel:proposition.channel,phase:proposition.phase,qualification:proposition.qualification};
            obligation_support::admissible(&question,&proof,RowRef::of(proposition.id()),qualification,&conditions::Diagram::from_records(condition,&nodes)?,(coverage.scope,coverage.context,coverage.availability))?;
        }
        Ok(())
    }
}
