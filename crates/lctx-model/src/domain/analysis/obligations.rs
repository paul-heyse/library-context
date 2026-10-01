//! Nominal subjects retain the precise unanswered question; discharge never follows from absence.
use super::{AnalysisInvocation, AnalysisMethod, invalid, coverage::AnalysisCoverage, support::{AnalysisDerivation,AnalysisProposition}};
use crate::domain::{
    assertion::AssertionQualification, calls::CallPhase, normalized::entities::EntityRef,
    obligation::ObligationKind, source::Occurrence, transfer::TransferKey, *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AnalysisChannel {
    Value = 0,
    Effect = 1,
    Exception = 2,
    Role = 3,
    Execution = 4,
    Completion = 5,
    Catalog = 6,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "obligation_subjects")]
pub enum ObligationSubject {
    #[model(code = 0)]
    Entity { entity: Id<EntityRef> },
    #[model(code = 1)]
    SourceCall { occurrence: Id<Occurrence> },
    #[model(code = 2)]
    Transfer { transfer: Id<TransferKey> },
    #[model(code = 3)]
    Computation { invocation: Id<AnalysisInvocation> },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_obligations", validate = validate_obligation)]
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
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ObligationSubject>(),
        Relation::of::<AnalysisObligation>(),
        Relation::of::<DischargeEvidence>(),
    ]
}

/// A persisted discharge cites an exact subject proof. There is no producer-selected verdict;
/// publication reconstructs it with the shared conservative verdict operation.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="analysis_discharge_evidence",rule="analysis_discharge",conclusion=obligation,invariants=discharge_invariants)]
pub struct DischargeEvidence {
    #[model(key)] pub obligation:Id<AnalysisObligation>,
    #[model(key,premise)] pub derivation:Id<AnalysisDerivation>,
    #[model(key,premise)] pub coverage:Id<AnalysisCoverage>,
}
pub fn admissible_discharge(obligation:&AnalysisObligation,proposition:&AnalysisProposition,qualification:&AssertionQualification,condition:&conditions::Diagram,coverage:&AnalysisCoverage)->Result<obligation::Conclusion,ModelError> {
    if (obligation.subject,obligation.channel,obligation.phase)!=(proposition.subject,proposition.channel,proposition.phase) || proposition.qualification!=qualification.id() || obligation.qualification!=qualification.id() || condition.id()!=qualification.condition || (coverage.scope,coverage.context)!=(qualification.scope,qualification.context) { return Err(invalid("discharge proof names another question or frame")); }
    use normalized::coverage::EvidenceAvailability as A;
    let status=match coverage.availability { A::Complete=>attribution::CoverageStatus::CompleteUnderStatedModel,A::Partial=>attribution::CoverageStatus::Partial,A::Unavailable=>attribution::CoverageStatus::Unavailable,A::NotRequested=>attribution::CoverageStatus::NotRequested,A::NoScope=>return Err(invalid("empty domain cannot discharge an obligation")) };
    let result=obligation::verdict(obligation::VerdictInput { condition:Some(condition),open:&[],coverage:status,approximation:qualification.approximation,modality:qualification.modality });
    let mut decisions=obligation::Decisions::default();
    decisions.proof(obligation.subject,proposition.id(),result.verdict);
    if !obligation::discharge(&std::collections::BTreeSet::from([obligation.subject]),&decisions).0 { return Err(invalid("proof does not discharge the exact obligation")); }
    Ok(result)
}
fn discharge_invariants()->Vec<Invariant> {
    vec![Invariant {name:"analysis_discharge",inputs:vec![ValidationInput::of::<AnalysisObligation>(&["id"]),ValidationInput::of::<DischargeEvidence>(&["id"]),ValidationInput::of::<AnalysisDerivation>(&["id"]),ValidationInput::of::<AnalysisProposition>(&["id"]),ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<AnalysisCoverage>(&["id"]),ValidationInput::of::<conditions::Condition>(&["id"]),ValidationInput::of::<conditions::ConditionNode>(&["id"])],create:std::sync::Arc::new(|budget|Box::new(DischargeCheck {charge:charged::StateCharge::new(budget,"analysis_discharge"),obligations:Default::default(),evidence:Default::default(),derivations:Default::default(),propositions:Default::default(),qualifications:Default::default(),coverage:Default::default(),conditions:Default::default(),nodes:Default::default()})) }]
}
struct DischargeCheck {
    charge:charged::StateCharge,
    obligations:charged::ChargedMap<Id<AnalysisObligation>,AnalysisObligation>,
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
        macro_rules! insert { ($r:ty,$field:ident)=>{ if relation==<$r>::NAME { for row in <$r>::decode(batch)? { if self.$field.insert(&mut self.charge,row.id(),row)?.is_some() {return Err(ModelError::Conflict(<$r>::NAME));} } return Ok(()); } }; }
        insert!(AnalysisObligation,obligations);insert!(DischargeEvidence,evidence);insert!(AnalysisDerivation,derivations);insert!(AnalysisProposition,propositions);insert!(AssertionQualification,qualifications);insert!(AnalysisCoverage,coverage);insert!(conditions::Condition,conditions);insert!(conditions::ConditionNode,nodes);
        Err(invalid("undeclared discharge input"))
    }
    fn finish(self:Box<Self>)->Result<(),ModelError> {
        let bytes=self.nodes.values().try_fold(0usize,|n,row|n.checked_add(size_of::<conditions::ConditionNode>()+row.heap_bytes()+128).ok_or_else(||invalid("discharge allocation overflow")))?;
        let _reservation=self.charge.budget().ok_or_else(||invalid("discharge budget absent"))?.reserve("analysis_discharge",bytes)?;
        let nodes=self.nodes.values().cloned().collect::<Vec<_>>();
        for (_,evidence) in self.evidence.iter() {
            let obligation=self.obligations.get(&evidence.obligation).ok_or_else(||invalid("discharged obligation absent"))?;
            let derivation=self.derivations.get(&evidence.derivation).ok_or_else(||invalid("discharging derivation absent"))?;
            let proposition=self.propositions.get(&derivation.proposition).ok_or_else(||invalid("discharging proposition absent"))?;
            let qualification=self.qualifications.get(&proposition.qualification).ok_or_else(||invalid("discharging qualification absent"))?;
            let condition=self.conditions.get(&qualification.condition).ok_or_else(||invalid("discharging condition absent"))?;
            let coverage=self.coverage.get(&evidence.coverage).ok_or_else(||invalid("discharging coverage absent"))?;
            if derivation.qualification!=qualification.id() || derivation.invocation!=coverage.invocation {return Err(invalid("discharging coverage differs from proof invocation"));}
            admissible_discharge(obligation,proposition,qualification,&conditions::Diagram::from_records(condition,&nodes)?,coverage)?;
        }
        Ok(())
    }
}
