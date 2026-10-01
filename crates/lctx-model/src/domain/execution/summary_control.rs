//! Rebased guard influence has its own qualification, separate from the selected value branch.
use crate::Domain;
use crate::domain::{*,analysis::{self,summary as owner,support::{DerivedEvidence,SourceFacts}},assertion::AssertionQualification,conditions::Diagram,transfer::summary::*,resources::ResourceBudget};
use super::summary_production::SummaryRecords;
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="summary_control_witnesses",rule="rebased_summary_guard")]
pub struct SummaryControlWitness{
 #[model(key,premise)]pub witness:Id<SummaryWitness>,
 #[model(key)]pub influence:Id<ControlInfluence>,
 #[model(key)]pub qualification:Id<AssertionQualification>,
 pub status:analysis::policy::EvidenceStatus,pub heuristic:bool,
}
impl analysis::support::sealed::DerivedEvidence for SummaryControlWitness{}
impl DerivedEvidence for SummaryControlWitness{fn source_facts(&self)->SourceFacts{SourceFacts{qualification:self.qualification,status:self.status,heuristic:self.heuristic}}}
pub(super) fn publish(invocation:&owner::AnalysisInvocation,definition:&analysis::AnalysisDefinition,witness:&SummaryWitness,influence:&ControlInfluence,q:&AssertionQualification,condition:&Diagram,out:&mut SummaryRecords,budget:&ResourceBudget)->Result<(),ModelError>{
 if influence.qualification!=q.id()||condition.id()!=q.condition{return Err(ModelError::Invalid("rebased influence qualification differs".into()));}
 let proof=SummaryControlWitness{witness:witness.id(),influence:influence.id(),qualification:q.id(),status:witness.status,heuristic:witness.heuristic};let source=owner::SupportSource::ControlWitness{witness:proof.id()};let subject=owner::ObligationSubject::SourceCall{occurrence:influence.evaluation};
 let evidence=owner::support::EvidencePremise::derived(&source,&proof,q,condition)?;let(derivation,proposition,members,qualified)=owner::AnalysisDerivation::emit(invocation,definition,subject.id(),analysis::AnalysisChannel::Role,calls::CallPhase::Call,analysis::support::QualificationOperation::Conjunction,&[evidence],budget)?;let generated=owner::SupportSource::AnalysisDerivation{derivation:derivation.id()};
 out.control_supports.insert(ControlSupport{assertion:influence.id(),source:generated.id()})?;out.control_witnesses.insert(proof)?;out.sources.insert(source)?;out.sources.insert(generated)?;out.subjects.insert(subject)?;out.propositions.insert(proposition)?;out.derivations.insert(derivation)?;for row in members{out.derivation_premises.insert(row)?;}out.vocabulary.qualified(&qualified.qualification,&qualified.condition)?;out.influences.insert(influence.clone())?;Ok(())
}
