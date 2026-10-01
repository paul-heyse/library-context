//! Caller continuation requires independent fresh binding, ordered actual evaluation,
//! callee outcome and exact outside holders during frame release.
use crate::domain::{*,analysis::{policy::EvidenceStatus,support::inferred_status,Interpretation},obligation::ObligationKind,resources::ResourceBudget};
use super::{source_call::CheckedSourceBinding,body::CheckedSourceBody,evaluation::{EvaluationData,ReleaseSafety},outcome::PendingOutcome,completion_production::CompletedEvaluations};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum InvocationOutcome{Normal,Raised{site:Id<source::Occurrence>,exception:super::ExactRuntimeException}}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct InvocationArgument{pub formal:Id<calls::SignatureParameter>,pub actual:Id<source::Occurrence>,pub evaluation:Id<super::records::ExpressionEvaluation>}
pub struct CheckedSourceInvocation{event:Id<normalized::events::NormalizedCallEvent>,callee:Id<normalized::entities::EntityRef>,qualification:Id<assertion::AssertionQualification>,outcome:InvocationOutcome,status:EvidenceStatus,arguments:Vec<InvocationArgument>,release:ReleaseSafety,_charge:charged::StateCharge}
impl CheckedSourceInvocation{
 pub fn event(&self)->Id<normalized::events::NormalizedCallEvent>{self.event}
 pub fn callee(&self)->Id<normalized::entities::EntityRef>{self.callee}
 pub fn qualification(&self)->Id<assertion::AssertionQualification>{self.qualification}
 pub fn outcome(&self)->InvocationOutcome{self.outcome}
 pub fn status(&self)->EvidenceStatus{self.status}
 pub fn arguments(&self)->&[InvocationArgument]{&self.arguments}
 pub fn release(&self)->ReleaseSafety{self.release}
 pub fn derive(data:&EvaluationData,header:&CheckedSourceBinding,body:&CheckedSourceBody,earlier:&CompletedEvaluations,budget:&ResourceBudget)->Result<Result<Self,ObligationKind>,ModelError>{
  let mut charge=charged::StateCharge::new(budget,"source_call_frame_release");charge.grow(size_of::<Self>()+header.arguments().len()*size_of::<InvocationArgument>()*2)?;let h=header.request();let b=body.request();if (h.input,h.context,header.callee(),header.declaration())!=(b.input,b.context,b.callee,body.declaration()){return Ok(Err(ObligationKind::IncompatibleContexts));}
  let Some(caller)=data.qualifications.get(header.qualification())else{return Ok(Err(ObligationKind::MissingEvidence))};let Some(callee)=data.qualifications.get(body.qualification())else{return Ok(Err(ObligationKind::MissingEvidence))};if caller!=callee{return Ok(Err(ObligationKind::IncompatibleContexts));}
  let base=earlier.earlier();let mut arguments=Vec::new();let mut status=inferred_status(Interpretation::Structural,[header.status(),body.status()]);
  for(formal,actual)in header.arguments(){
   let mut rows=base.evaluations.iter().filter(|row|row.expression==*actual&&row.owner==header.caller()&&base.invocations.get(row.invocation).is_some_and(|frame|(frame.input,frame.context)==(h.input,h.context)));let Some(row)=rows.next()else{return Ok(Err(ObligationKind::MissingEvidence));};if rows.next().is_some(){return Ok(Err(ObligationKind::AmbiguousBinding));}
   let checked=base.replay(row)?;if checked.exception().is_some(){return Ok(Err(ObligationKind::CallTransfer));}
   if checked.release()!=ReleaseSafety::Closed&&!base.caller_holds_argument(row)?&&!matches!(super::builtin_read::CheckedBuiltinRead::derive(data,checked.request(),budget)?,Ok(_)){return Ok(Err(ObligationKind::FrameExitCleanup));}
   if data.qualifications.get(checked.qualification())!=Some(caller){return Ok(Err(ObligationKind::IncompatibleContexts));}
   status=inferred_status(Interpretation::Structural,[status,checked.status()]);arguments.push(InvocationArgument{formal:*formal,actual:*actual,evaluation:row.id()});
  }
  for(site,safety)in body.release_inputs(){if *safety==ReleaseSafety::Closed{continue;}if *safety!=ReleaseSafety::CallerRetained{return Ok(Err(ObligationKind::FrameExitCleanup));}
   let mut rows=base.evaluations.iter().filter(|row|row.expression==*site&&row.owner==header.callee()&&base.invocations.get(row.invocation).is_some_and(|frame|(frame.input,frame.context)==(h.input,h.context)));let Some(row)=rows.next()else{return Ok(Err(ObligationKind::FrameExitCleanup));};if rows.next().is_some(){return Ok(Err(ObligationKind::AmbiguousBinding));}
   let Some(formal)=base.held_formal(row)?else{return Ok(Err(ObligationKind::FrameExitCleanup));};if arguments.iter().filter(|argument|argument.formal==formal).count()!=1{return Ok(Err(ObligationKind::FrameExitCleanup));}
  }
  let outcome=match body.outcome(){PendingOutcome::Normal|PendingOutcome::Return{..}=>InvocationOutcome::Normal,PendingOutcome::Raise{site,exception}=>InvocationOutcome::Raised{site,exception},PendingOutcome::Break{..}|PendingOutcome::Continue{..}=>return Ok(Err(ObligationKind::UnsupportedControlFlow))};
  let mut release=ReleaseSafety::Closed;if let PendingOutcome::Return{site}=body.outcome(){let mut values=data.placements.iter().filter(|p|p.parent==Some(site)&&p.field==lexical::SyntaxField::Value);if let Some(value)=values.next(){if values.next().is_some(){return Ok(Err(ObligationKind::MissingEvidence));}let Some((_,safety))=body.release_inputs().iter().find(|(expression,_)|*expression==value.occurrence)else{return Ok(Err(ObligationKind::MissingEvidence));};release=*safety;if release==ReleaseSafety::CallerRetained{let row=base.evaluations.iter().find(|row|row.expression==value.occurrence&&row.owner==header.callee()&&base.invocations.get(row.invocation).is_some_and(|frame|(frame.input,frame.context)==(h.input,h.context))).ok_or_else(||ModelError::Invalid("returned formal evaluation absent".into()))?;let formal=base.held_formal(row)?.ok_or_else(||ModelError::Invalid("returned formal holder absent".into()))?;let argument=arguments.iter().find(|argument|argument.formal==formal).ok_or_else(||ModelError::Invalid("returned formal actual absent".into()))?;let row=base.evaluations.get(argument.evaluation).ok_or_else(||ModelError::Invalid("returned actual proof absent".into()))?;release=base.replay(row)?.release();}}}
  Ok(Ok(Self{event:h.event,callee:b.callee,qualification:header.qualification(),outcome,status,arguments,release,_charge:charge}))
 }
}
