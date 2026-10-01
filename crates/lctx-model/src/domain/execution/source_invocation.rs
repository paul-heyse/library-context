//! Caller continuation requires independent fresh binding, body outcome and frame release.
//! The first finite operation accepts the proven zero-argument fresh function envelope only.
use crate::domain::{*,analysis::{policy::EvidenceStatus,support::inferred_status,Interpretation},obligation::ObligationKind,resources::ResourceBudget};
use super::{source_call::CheckedSourceBinding,body::CheckedSourceBody,evaluation::{EvaluationData,ReleaseSafety},outcome::PendingOutcome};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum InvocationOutcome{Normal,Raised{site:Id<source::Occurrence>,exception:super::ExactRuntimeException}}
pub struct CheckedSourceInvocation{event:Id<normalized::events::NormalizedCallEvent>,callee:Id<normalized::entities::EntityRef>,qualification:Id<assertion::AssertionQualification>,outcome:InvocationOutcome,status:EvidenceStatus,_charge:charged::StateCharge}
impl CheckedSourceInvocation{
 pub fn event(&self)->Id<normalized::events::NormalizedCallEvent>{self.event}
 pub fn callee(&self)->Id<normalized::entities::EntityRef>{self.callee}
 pub fn qualification(&self)->Id<assertion::AssertionQualification>{self.qualification}
 pub fn outcome(&self)->InvocationOutcome{self.outcome}
 pub fn status(&self)->EvidenceStatus{self.status}
 /// The callable remains held by its exact fresh caller binding. No argument/formal objects
 /// are installed in this zero-slot callee; every entered temporary must have closed release.
 pub fn derive(data:&EvaluationData,header:&CheckedSourceBinding,body:&CheckedSourceBody,budget:&ResourceBudget)->Result<Result<Self,ObligationKind>,ModelError>{
  let mut charge=charged::StateCharge::new(budget,"source_call_frame_release");charge.grow(size_of::<Self>())?;let h=header.request();let b=body.request();if (h.input,h.context,header.callee(),header.declaration())!=(b.input,b.context,b.callee,body.declaration()){return Ok(Err(ObligationKind::IncompatibleContexts));}
  let Some(caller)=data.qualifications.get(header.qualification())else{return Ok(Err(ObligationKind::MissingEvidence))};let Some(callee)=data.qualifications.get(body.qualification())else{return Ok(Err(ObligationKind::MissingEvidence))};if caller!=callee{return Ok(Err(ObligationKind::IncompatibleContexts));}
  if body.release_inputs().iter().any(|(_,safety)|*safety!=ReleaseSafety::Closed){return Ok(Err(ObligationKind::FrameExitCleanup));}
  let outcome=match body.outcome(){PendingOutcome::Normal|PendingOutcome::Return{..}=>InvocationOutcome::Normal,PendingOutcome::Raise{site,exception}=>InvocationOutcome::Raised{site,exception},PendingOutcome::Break{..}|PendingOutcome::Continue{..}=>return Ok(Err(ObligationKind::UnsupportedControlFlow))};
  let status=inferred_status(Interpretation::Structural,[header.status(),body.status()]);Ok(Ok(Self{event:h.event,callee:b.callee,qualification:header.qualification(),outcome,status,_charge:charge}))
 }
}
