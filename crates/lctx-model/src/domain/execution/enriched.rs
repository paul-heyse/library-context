//! Ordered caller execution consumes independent earlier source-invocation proofs. The
//! SourceCall operation retains private header/body/frame evidence through this callback.
use crate::domain::{*,analysis,resources::ResourceBudget,obligation::ObligationKind};
use super::{source_call_records::{SourceCallData,SourceCallHeader,SourceInvocation},source_call::CheckedSourceBinding,evaluation::{CheckedEvaluation,ExpressionRequest},completion::{CheckedCompletion,CompletionRequest}};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum EvaluationPremise {Base(Id<super::records::ExpressionEvaluation>),Source(Id<SourceInvocation>)}
pub struct EnrichedFrame<'a>{data:&'a SourceCallData,input:Id<input::InputRevision>,context:Id<attribution::AnalysisContext>,evaluations:Vec<(CheckedEvaluation,EvaluationPremise)>,headers:Vec<(&'a CheckedSourceBinding,&'a SourceCallHeader)>,budget:&'a ResourceBudget,_charge:charged::StateCharge}
impl EnrichedFrame<'_>{
 pub fn complete(&self,request:CompletionRequest)->Result<Result<CheckedCompletion,ObligationKind>,ModelError>{
  if(request.input,request.context)!=(self.input,self.context){return Ok(Err(ObligationKind::IncompatibleContexts));}
  let _scratch=self.budget.reserve("enriched_completion_operands",self.evaluations.len().checked_mul(size_of::<&CheckedEvaluation>()*2).ok_or_else(||ModelError::Invalid("enriched operand allowance overflow".into()))?)?;
  let proofs=self.evaluations.iter().filter(|(proof,_)|proof.request().owner==request.owner).map(|(proof,_)|proof).collect::<Vec<_>>();
  super::completion::complete_with_headers(&self.data.evaluation,request,&proofs,&self.headers,self.budget)
 }
 pub(crate) fn evaluation_premises(&self,proof:&CheckedCompletion)->Result<Vec<EvaluationPremise>,ModelError>{
  let mut results=Vec::new();
  for facts in proof.evaluation_facts(){let mut matching=self.evaluations.iter().filter(|(checked,_)|super::records::EvaluationFacts::of(checked)==*facts);let(_,premise)=matching.next().ok_or_else(||ModelError::Invalid("enriched completion evaluation mapping absent".into()))?;if matching.next().is_some(){return Err(ModelError::Invalid("enriched completion evaluation mapping ambiguous".into()));}results.push(*premise);}
  Ok(results)
 }
}
/// A callback over one independently replayed frame avoids a second trusted record route.
/// Persisted SourceInvocation inputs are compared by the publishing owner against the rows
/// produced by this exact replay; source snapshot authority remains the publication check.
pub fn with_frame<T>(data:&SourceCallData,invocation:&analysis::source_call::AnalysisInvocation,definition:&analysis::AnalysisDefinition,budget:&ResourceBudget,visit:impl FnOnce(&EnrichedFrame<'_>)->Result<T,ModelError>)->Result<(T,super::source_call_records::SourceCallRecords),ModelError>{
 let mut visit=Some(visit);let mut result=None;
 let records=super::source_call_records::prepare_all_with(data,invocation,definition,stages::Profile::Behavioral,budget,&mut|headers,calls|{
  let mut frame=EnrichedFrame{data,input:invocation.input,context:invocation.context,evaluations:Vec::new(),headers:Vec::new(),budget,_charge:charged::StateCharge::new(budget,"enriched_private_evidence")};
  let earlier=data.completed.earlier();
  for row in earlier.evaluations.iter(){let parent=earlier.invocations.get(row.invocation).ok_or_else(||ModelError::Invalid("enriched base invocation absent".into()))?;if(parent.input,parent.context)!=(invocation.input,invocation.context){continue;}frame._charge.grow(size_of::<(CheckedEvaluation,EvaluationPremise)>()*2)?;frame.evaluations.push((earlier.replay(row)?,EvaluationPremise::Base(row.id())));}
  for(header,row)in headers{frame._charge.grow(size_of::<(&CheckedSourceBinding,&SourceCallHeader)>()*2)?;frame.headers.push((header,row));}
  for(proof,row)in calls{
   let event=data.bindings.event_events.get(proof.event()).ok_or_else(||ModelError::Invalid("enriched source event absent".into()))?;let mut matching=headers.iter().filter(|(_,h)|h.event==event.id());let(_,header)=matching.next().ok_or_else(||ModelError::Invalid("enriched source header absent".into()))?;if matching.next().is_some(){return Err(ModelError::Invalid("enriched source header ambiguous".into()));}
   let request=ExpressionRequest{input:invocation.input,context:invocation.context,owner:header.owner,expression:event.site};
   let evaluation=super::evaluation::source_call_evaluation(&data.evaluation,request,proof,row.id(),budget)?.map_err(|_|ModelError::Invalid("enriched source evaluation refused".into()))?;
   frame._charge.grow(size_of::<(CheckedEvaluation,EvaluationPremise)>()*2)?;frame.evaluations.push((evaluation,EvaluationPremise::Source(row.id())));
  }
  result=Some(visit.take().ok_or_else(||ModelError::Invalid("enriched frame callback repeated".into()))?(&frame)?);Ok(())
 })?;
 Ok((result.ok_or_else(||ModelError::Invalid("enriched frame callback absent".into()))?,records))
}
