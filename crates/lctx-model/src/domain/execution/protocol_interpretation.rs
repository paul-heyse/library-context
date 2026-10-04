//! Typed protocol actions and conditional continuation questions. Native query success is not
//! a runtime completion certificate; exact context exits remain owned by ContextExecution.
use super::{closed_targets::{self,ClosedTargetAssessment,TargetBasis,TargetData},model_application::ModelApplicationData,model_production::{ModelData,ModelRecords}};
use crate::domain::{analysis, assumptions::*, assertion::*, attribution::*, calls::*, lexical::SyntaxField, models::Catalog, normalized::{Rows,binding_normalization::VerifiedBindings}, obligation::ObligationKind, protocols::*, resources::ResourceBudget, source::*, syntax::SyntaxPlacement, types::*, *};
use crate::{Domain,DomainCode};

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum ProtocolAction { ExplicitCall=0,AttributeGet=1,Comparison=2,IteratorCreation=3,IteratorNext=4,ContextEntry=5,Decorator=6,SubscriptGet=7,SubscriptSet=8,Binary=9,AugmentedOperator=10,AugmentedRhs=11,AugmentedStatement=12,LoopAssignment=13,Repr=14,Abs=15,StringConversion=16,Slice=17,ChainedAssignment=18,FormatArtificial=19,FormatStringify=20,Allocation=21,Initialization=22,PropertyGet=23,PropertySet=24,Definition=25 }
/// The 24 native origin arms are exhaustive. Iterator creation and next are distinct actions.
pub fn origin_action(step:OriginStep)->ProtocolAction { use OriginStep as S;use ProtocolAction as A; match step {
 S::GetAttrConstantLiteral=>A::AttributeGet,S::Comparison=>A::Comparison,S::GeneratorIter|S::ForIter|S::IterCall=>A::IteratorCreation,S::GeneratorNext|S::ForNext|S::NextCall=>A::IteratorNext,S::WithEnter=>A::ContextEntry,S::ForDecoratedTarget=>A::Decorator,S::SubscriptGetItem=>A::SubscriptGet,S::SubscriptSetItem=>A::SubscriptSet,S::BinaryOperator=>A::Binary,S::AugmentedAssignDunderCall=>A::AugmentedOperator,S::AugmentedAssignRhs=>A::AugmentedRhs,S::AugmentedAssignStatement=>A::AugmentedStatement,S::ForAssign=>A::LoopAssignment,S::ReprCall=>A::Repr,S::AbsCall=>A::Abs,S::StrCallToDunderMethod=>A::StringConversion,S::Slice=>A::Slice,S::ChainedAssign=>A::ChainedAssignment,S::FormatStringArtificial=>A::FormatArtificial,S::FormatStringStringify=>A::FormatStringify } }
fn action(extra:&TargetData,target:&CallTarget)->Result<ProtocolAction,ObligationKind> {
 let mut steps=extra.origin_steps.iter().filter(|s|s.origin==target.origin).collect::<Vec<_>>();steps.sort_by_key(|s|s.ordinal);
 if steps.iter().enumerate().any(|(n,s)|s.ordinal!=n as i64){return Err(ObligationKind::MissingEvidence);}
 let (origin,_)=CallOrigin::new(&steps.iter().map(|s|(s.step,s.index)).collect::<Vec<_>>()).map_err(|_|ObligationKind::MissingEvidence)?;
 if extra.origins.get(target.origin)!=Some(&origin){return Err(ObligationKind::MissingEvidence);}
 Ok(if let Some(step)=steps.last(){origin_action(step.step)}else{match target.phase {CallPhase::Call=>ProtocolAction::ExplicitCall,CallPhase::New=>ProtocolAction::Allocation,CallPhase::Init=>ProtocolAction::Initialization,CallPhase::Decorator=>ProtocolAction::Decorator,CallPhase::PropertyGet=>ProtocolAction::PropertyGet,CallPhase::PropertySet=>ProtocolAction::PropertySet,CallPhase::Definition=>ProtocolAction::Definition}})
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="protocol_action_assessments",rule="native_protocol_action_phase")]
pub struct ProtocolActionAssessment {
 #[model(key,premise)] pub invocation:Id<analysis::model::AnalysisInvocation>,
 #[model(key,premise)] pub target:Id<CallTarget>,
 pub site:Id<Occurrence>,pub origin:Id<CallOrigin>,pub receiver:Id<Receiver>,pub phase:CallPhase,
 pub action:Option<ProtocolAction>,pub reason:Option<ObligationKind>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum InvocationQuestion { GivenInvocationEntered=0 }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="terminal_frontier_assessments",rule="guarded_native_terminal_question")]
pub struct TerminalFrontierAssessment {
 #[model(key,premise)] pub invocation:Id<analysis::model::AnalysisInvocation>,
 #[model(key,premise)] pub observation:Id<NativeTerminalObservation>,
 pub target:Option<Id<ClosedTargetAssessment>>,pub frontier:Option<Id<ConditionalTerminalFrontier>>,pub reason:Option<ObligationKind>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="conditional_terminal_frontiers",rule="given_entry_no_normal_continuation")]
pub struct ConditionalTerminalFrontier {
 #[model(key,premise)] pub invocation:Id<analysis::model::AnalysisInvocation>,
 #[model(key,premise)] pub observation:Id<NativeTerminalObservation>,
 #[model(key,premise)] pub native:Id<analysis::native::NativeAssertionPremise>,
 #[model(key,premise)] pub target:Id<ClosedTargetAssessment>,
 #[model(key,premise)] pub declared_return:Id<TypeObservation>,
 #[model(key,premise)] pub conformance:Id<Assumption>,
 #[model(key)] pub qualification:Id<AssertionQualification>,
 pub owner:Id<normalized::entities::EntityRef>,pub call:Id<Occurrence>,pub statement:Id<Occurrence>,
 pub question:InvocationQuestion,pub effects_unknown:bool,pub exceptions_unknown:bool,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="normal_continuation_restrictions",rule="qualified_direct_suite_continuation")]
pub struct NormalContinuationRestriction {
 #[model(key,premise)] pub frontier:Id<ConditionalTerminalFrontier>,
 #[model(key,premise)] pub statement:Id<SyntaxPlacement>,
 #[model(key,premise)] pub following:Id<SyntaxPlacement>,
 #[model(key,premise)] pub statement_native:Id<analysis::native::NativeAssertionPremise>,
 #[model(key,premise)] pub following_native:Id<analysis::native::NativeAssertionPremise>,
 pub owner:Id<normalized::entities::EntityRef>,pub from:Id<Occurrence>,pub to:Id<Occurrence>,pub qualification:Id<AssertionQualification>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum ExitCharacterization { Suppress=0,DoNotSuppress=1,Unknown=2 }
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="native_exit_characterizations",rule="qualified_exit_typing_characterization")]
pub struct NativeExitCharacterization {
 #[model(key,premise)] pub invocation:Id<analysis::model::AnalysisInvocation>,
 #[model(key,premise)] pub observation:Id<NativeExitObservation>,
 #[model(key)] pub exceptional:bool,
 pub result:Id<TypeTerm>,pub characterization:ExitCharacterization,pub qualification:Id<AssertionQualification>,
 pub conformance:Option<Id<Assumption>>,pub reason:Option<ObligationKind>,
 /// A typing literal never authorizes suppression, effects or cleanup completion.
 pub runtime_completion_admitted:bool,
}
pub struct ProtocolData {pub target:TargetData,pub terminals:Rows<NativeTerminalObservation>,pub exits:Rows<NativeExitObservation>,pub literals:Rows<value::Literal>}
impl ProtocolData {
 pub fn new(b:&ResourceBudget)->Self{Self{target:TargetData::new(b),terminals:Rows::new(b),exits:Rows::new(b),literals:Rows::new(b)}}
 pub fn inputs()->Vec<ValidationInput>{let mut r=TargetData::inputs();r.extend([ValidationInput::of::<NativeTerminalObservation>(&["id"]),ValidationInput::of::<NativeExitObservation>(&["id"]),ValidationInput::of::<value::Literal>(&["id"])]);r}
 pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{self.target.visit(n,b)?;if n==NativeTerminalObservation::NAME{self.terminals.decode(b)?;}if n==NativeExitObservation::NAME{self.exits.decode(b)?;}if n==value::Literal::NAME{self.literals.decode(b)?;}Ok(())}
}
fn add_basis(out:&mut ModelRecords,defs:impl IntoIterator<Item=Assumption>)->Result<ResolvedAssumptions,ModelError>{let mut ids=Vec::new();for a in defs{ids.push(a.id());out.assumptions.insert(a)?;}let basis=AssumptionSet::new(ids)?;out.assumption_sets.insert(basis.set.clone())?;for m in &basis.members{out.assumption_members.insert(m.clone())?;}Ok(basis)}
fn direct_statement(data:&ModelApplicationData,call:Id<Occurrence>)->Result<&SyntaxPlacement,ObligationKind>{let b=&data.bindings;let p=closed_targets::one(b.placements.iter().filter(|p|p.occurrence==call && p.field==SyntaxField::Value))?;let stmt=p.parent.and_then(|s|b.occurrences.get(s)).ok_or(ObligationKind::MissingEvidence)?;if stmt.syntax_kind!=SyntaxKind::StmtExpr{return Err(ObligationKind::UnsupportedControlFlow);}closed_targets::one(b.placements.iter().filter(|p|p.occurrence==stmt.id()))}
fn terminal_return<'a>(data:&'a ModelApplicationData,obs:&NativeTerminalObservation,signature:Id<Signature>,input:Id<input::InputRevision>,context:Id<AnalysisContext>)->Result<(&'a TypeObservation,Assumption),ObligationKind>{
 if obs.decision!=TerminalDecision::DeclaredOrInferredDivergence || obs.return_is_inferred!=Some(false){return Err(ObligationKind::OutsideProviderModel);}
 let b=&data.bindings;let signature=b.signatures.get(signature).ok_or(ObligationKind::MissingEvidence)?;
 if !signature.role.runtime_source() || closed_targets::function_identity(data,obs.callee)!=Some(signature.symbol){return Err(ObligationKind::OutsideProviderModel);}
 let declaration=closed_targets::one(b.entity_declarations.iter().filter(|d|d.symbol==signature.symbol && b.qualifications.get(d.qualification).is_some_and(|q|q.context==context)))?;
 let row=closed_targets::one(b.type_observations.iter().filter(|r|r.subject==declaration.declaration && r.role==TypeRole::Return && r.declared && Some(r.term)==obs.return_type && b.qualifications.get(r.qualification).is_some_and(|q|q.context==context)))?;
 if !matches!(b.terms.get(row.term),Some(TypeTerm::Never{flavor:NeverFlavor::NoReturn})){return Err(ObligationKind::OutsideProviderModel);}
 let a=closed_targets::conformance(data,row,input,context)?;Ok((row,a))
}
fn restrict(data:&ModelApplicationData,f:&ConditionalTerminalFrontier,owner_declaration:Id<Occurrence>,input:Id<input::InputRevision>)->Result<NormalContinuationRestriction,ObligationKind>{
 let b=&data.bindings;let p=direct_statement(data,f.call)?;let parent=p.parent.and_then(|s|b.occurrences.get(s)).ok_or(ObligationKind::MissingEvidence)?;
 let declaration=b.occurrences.get(owner_declaration).ok_or(ObligationKind::MissingEvidence)?;
 if p.field!=SyntaxField::Body || parent.source!=declaration.source || parent.start!=declaration.start || parent.end!=declaration.end || parent.syntax_kind!=SyntaxKind::StmtFunctionDef{return Err(ObligationKind::UnsupportedControlFlow);}
 let next=closed_targets::one(b.placements.iter().filter(|s|s.parent==p.parent && s.field==p.field && s.ordinal==p.ordinal+1))?;
 let q=b.qualifications.get(f.qualification).or_else(||b.qualifications.get(p.qualification)).ok_or(ObligationKind::MissingEvidence)?;
 let a=closed_targets::native(data,derivation::RowRef::of(p.id()),p.qualification,input,q.context)?;
 let c=closed_targets::native(data,derivation::RowRef::of(next.id()),next.qualification,input,q.context)?;
 Ok(NormalContinuationRestriction{frontier:f.id(),statement:p.id(),following:next.id(),statement_native:a.id(),following_native:c.id(),owner:f.owner,from:p.occurrence,to:next.occurrence,qualification:f.qualification})
}
pub(super) fn emit(data:&ModelData,catalog:&Catalog,verified:&VerifiedBindings,invocation:&analysis::model::AnalysisInvocation,out:&mut ModelRecords,budget:&ResourceBudget)->Result<(),ModelError>{
 let b=&data.early.bindings;let extra=&data.protocol.target;
 for t in b.targets.iter().filter(|t|b.qualifications.get(t.qualification).is_some_and(|q|q.context==invocation.context) && b.occurrences.get(t.site).and_then(|o|b.artifacts.get(o.source)).is_some_and(|a|a.input==invocation.input)) {let a=action(extra,t);out.protocol_actions.insert(ProtocolActionAssessment{invocation:invocation.id(),target:t.id(),site:t.site,origin:t.origin,receiver:t.receiver,phase:t.phase,action:a.as_ref().ok().copied(),reason:a.err()})?;}
 for attempt in data.bindings.attempts.iter().filter(|a|b.event_events.get(a.event).is_some_and(|e|e.context==invocation.context && b.occurrences.get(e.site).and_then(|o|b.artifacts.get(o.source)).is_some_and(|a|a.input==invocation.input))) {
  let mut assessment=ClosedTargetAssessment{invocation:invocation.id(),attempt:attempt.id(),event:attempt.event,target:None,original_qualification:None,target_native:None,receiver_observation:None,receiver_qualification:None,basis:TargetBasis::Open,qualification:None,reason:Some(ObligationKind::MissingEvidence),runtime_receiver:None,ancestry:None,receiver_conformance:None,no_extra_overrides:None,universe_support:None,final_metadata:None,final_member:None,final_native:None,member_native:None};
  if let Some(bound)=verified.bound(attempt.id()) {
   if let Some(shape)=verified.shape(attempt.id()) {
    assessment.target=Some(shape.target());assessment.original_qualification=b.targets.get(shape.target()).map(|t|t.qualification);assessment.qualification=assessment.original_qualification;
    match closed_targets::runtime_identity(catalog,&data.early,bound,shape,verified.effective_invocation(attempt.id()),budget)? {
     Ok(receiver)=>{assessment.basis=TargetBasis::ExactRuntime;assessment.reason=None;if let Some((r,mro))=receiver{assessment.runtime_receiver=Some(r);assessment.ancestry=Some(mro);}}
     Err(reason)=>assessment.reason=Some(reason),
    }
   }
   if assessment.basis==TargetBasis::Open && matches!(attempt.authority_reason,normalized::signature_applicability::AuthorityReason::DispatchOpen|normalized::signature_applicability::AuthorityReason::Established) && let Some(shape)=verified.source_shape(attempt.id()) {
    assessment.target=Some(shape.target());assessment.original_qualification=b.targets.get(shape.target()).map(|t|t.qualification);assessment.qualification=assessment.original_qualification;
    match closed_targets::typing_target(catalog,&data.early,extra,bound,shape,attempt,budget)? {
     Ok(p)=>{assessment.basis=TargetBasis::TypingConditional;assessment.reason=None;assessment.target_native=Some(p.target_native);assessment.receiver_observation=Some(p.receiver_observation);assessment.receiver_qualification=Some(p.question_base);assessment.ancestry=Some(p.ancestry);assessment.receiver_conformance=Some(p.receiver.id());assessment.no_extra_overrides=Some(p.overrides.id());assessment.universe_support=Some(p.support.id());assessment.final_metadata=Some(p.metadata);assessment.final_member=p.member;assessment.final_native=Some(p.metadata_native);assessment.member_native=Some(p.member_native);out.assumption_universes.insert(p.universe)?;out.universe_supports.insert(p.support)?;let basis=add_basis(out,[p.receiver,p.overrides])?;let q=closed_targets::question_qualification(b.qualifications.get(p.question_base).unwrap(),&basis)?;out.qualifications.insert(q.clone())?;assessment.qualification=Some(q.id());let _=p.basis;},
     Err(r)=>assessment.reason=Some(r),
    }
   }
  }
  out.closed_targets.insert(assessment)?;
 }
 for obs in data.protocol.terminals.iter().filter(|o|b.qualifications.get(o.qualification).is_some_and(|q|q.context==invocation.context) && b.occurrences.get(o.subject).and_then(|o|b.artifacts.get(o.source)).is_some_and(|a|a.input==invocation.input)) {
  let mut assessment=TerminalFrontierAssessment{invocation:invocation.id(),observation:obs.id(),target:None,frontier:None,reason:Some(ObligationKind::MissingEvidence)};
  let result=(||{
   let native=closed_targets::native(&data.early,derivation::RowRef::of(obs.id()),obs.qualification,invocation.input,invocation.context)?;
   let event=closed_targets::one(b.event_events.iter().filter(|e|e.site==obs.subject && e.context==invocation.context && e.origin==CallOrigin::explicit()))?;
   let callee=closed_targets::function_identity(&data.early,obs.callee).ok_or(ObligationKind::MissingEvidence)?;
   // Select this checked native callee question under its own basis. Open runtime alternatives
   // remain stored; the typing promise does not erase them or certify event execution.
   let target=closed_targets::one(out.closed_targets.iter().filter(|t|t.event==event.id() && t.basis!=TargetBasis::Open && verified.bound(t.attempt).and_then(|v|b.signatures.get(v.bound().signature())).is_some_and(|s|s.symbol==callee)))?;
   let shape=verified.source_shape(target.attempt).ok_or(ObligationKind::MissingEvidence)?;let bound=verified.bound(target.attempt).ok_or(ObligationKind::MissingEvidence)?;
   let attempt=data.bindings.attempts.get(target.attempt).ok_or(ObligationKind::MissingEvidence)?;
   let effective=attempt.effective.and_then(|id|b.callable_assessments.get(id)).ok_or(ObligationKind::MissingEvidence)?;
   if effective.asynchronous!=Some(false) || effective.generator!=Some(false){return Err(ObligationKind::UnsupportedControlFlow);}
   let t=b.targets.get(shape.target()).ok_or(ObligationKind::MissingEvidence)?;
   if shape.phase()!=CallPhase::Call || action(extra,t)?!=ProtocolAction::ExplicitCall{return Err(ObligationKind::CallTransfer);}
   let (row,a)=terminal_return(&data.early,obs,bound.bound().signature(),invocation.input,invocation.context)?;
   let stmt=direct_statement(&data.early,obs.subject)?;
   Ok((native.id(),target.clone(),shape.owner_entity(),shape.owner_declaration(),row.id(),a,stmt.occurrence))
  })();
  match result {Err(r)=>assessment.reason=Some(r),Ok((native,target,owner,declaration,row,a,statement))=>{
   let mut defs=vec![a.clone()];for id in [target.receiver_conformance,target.no_extra_overrides].into_iter().flatten(){defs.push(out.assumptions.get(id).ok_or_else(||ModelError::Invalid("closed target basis absent".into()))?.clone());}
   let basis=add_basis(out,defs)?;
   let mut q=b.qualifications.get(obs.qualification).unwrap().clone();q.assumptions=basis.set.id();out.qualifications.insert(q.clone())?;
   let frontier=ConditionalTerminalFrontier{invocation:invocation.id(),observation:obs.id(),native,target:target.id(),declared_return:row,conformance:a.id(),qualification:q.id(),owner,call:obs.subject,statement,question:InvocationQuestion::GivenInvocationEntered,effects_unknown:true,exceptions_unknown:true};
   match restrict(&data.early,&frontier,declaration,invocation.input){Ok(edge)=>{assessment.target=Some(target.id());assessment.frontier=Some(frontier.id());assessment.reason=None;out.normal_restrictions.insert(edge)?;out.terminal_frontiers.insert(frontier)?;},Err(r)=>assessment.reason=Some(r)}
  }}
  out.terminal_assessments.insert(assessment)?;
 }
 emit_exits(data,invocation,out)?;Ok(())
}
fn emit_exits(data:&ModelData,invocation:&analysis::model::AnalysisInvocation,out:&mut ModelRecords)->Result<(),ModelError>{
 let b=&data.early.bindings;
 for obs in data.protocol.exits.iter().filter(|o|b.qualifications.get(o.qualification).is_some_and(|q|q.context==invocation.context) && b.occurrences.get(o.subject).and_then(|o|b.artifacts.get(o.source)).is_some_and(|a|a.input==invocation.input)) {
  for (exceptional,result,status) in [(false,obs.normal_result,obs.normal_status),(true,obs.exceptional_result,obs.exceptional_status)] {
   let mut row=NativeExitCharacterization{invocation:invocation.id(),observation:obs.id(),exceptional,result,characterization:ExitCharacterization::Unknown,qualification:obs.qualification,conformance:None,reason:Some(ObligationKind::MissingEvidence),runtime_completion_admitted:false};
   let proof=(||{
    if obs.asynchronous || status!=NativeCallStatus::NoHardDiagnostics{return Err(ObligationKind::UnsupportedControlFlow);}
    closed_targets::native(&data.early,derivation::RowRef::of(obs.id()),obs.qualification,invocation.input,invocation.context)?;
    let receiver=closed_targets::one(b.type_observations.iter().filter(|r|r.subject==obs.subject && r.term==obs.receiver && b.qualifications.get(r.qualification).is_some_and(|q|q.context==invocation.context)))?;
    let class=match b.terms.get(receiver.term){Some(TypeTerm::ClassInstance{class,..})=>*class,_=>return Err(ObligationKind::MissingEvidence)};
    let member=closed_targets::one(b.traits.iter().filter(|t|t.defining_class==Some(class) && b.symbols.get(t.symbol).is_some_and(|s|s.name==obs.member_name)))?;
    let declaration=closed_targets::one(b.entity_declarations.iter().filter(|d|d.symbol==member.symbol))?;
    let declared=closed_targets::one(b.type_observations.iter().filter(|r|r.subject==declaration.declaration && r.role==TypeRole::Return && r.declared && r.term==result))?;
    let characterization=match b.terms.get(result){Some(TypeTerm::Literal{value})=>match data.protocol.literals.get(*value){Some(value::Literal::Bool{value:true})=>ExitCharacterization::Suppress,Some(value::Literal::Bool{value:false})=>ExitCharacterization::DoNotSuppress,_=>return Err(ObligationKind::MissingEvidence)},Some(TypeTerm::None)=>ExitCharacterization::DoNotSuppress,_=>return Err(ObligationKind::MissingEvidence)};
    Ok((characterization,closed_targets::conformance(&data.early,receiver,invocation.input,invocation.context)?,closed_targets::conformance(&data.early,declared,invocation.input,invocation.context)?))
   })();
   match proof{Err(r)=>row.reason=Some(r),Ok((kind,receiver,declared))=>{row.conformance=Some(declared.id());let basis=add_basis(out,[receiver,declared])?;let mut q=b.qualifications.get(obs.qualification).unwrap().clone();q.assumptions=basis.set.id();out.qualifications.insert(q.clone())?;row.qualification=q.id();row.characterization=kind;row.reason=None;}}
   out.exit_characterizations.insert(row)?;
  }
 }
 Ok(())
}
pub fn relations()->Vec<Relation>{vec![Relation::of::<ProtocolActionAssessment>(),Relation::of::<TerminalFrontierAssessment>(),Relation::of::<ConditionalTerminalFrontier>(),Relation::of::<NormalContinuationRestriction>(),Relation::of::<NativeExitCharacterization>()]}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn protocol_origin_audit_preserves_all_twenty_four_native_arms_and_phases() {
  use OriginStep as S;use ProtocolAction as A;
  // Independent native ordinal oracle: all original Pysa arms, including the extra stringify arm.
  let expected=[(S::GetAttrConstantLiteral,A::AttributeGet),(S::Comparison,A::Comparison),(S::GeneratorIter,A::IteratorCreation),(S::GeneratorNext,A::IteratorNext),(S::WithEnter,A::ContextEntry),(S::ForDecoratedTarget,A::Decorator),(S::SubscriptGetItem,A::SubscriptGet),(S::SubscriptSetItem,A::SubscriptSet),(S::BinaryOperator,A::Binary),(S::AugmentedAssignDunderCall,A::AugmentedOperator),(S::AugmentedAssignRhs,A::AugmentedRhs),(S::AugmentedAssignStatement,A::AugmentedStatement),(S::ForIter,A::IteratorCreation),(S::ForNext,A::IteratorNext),(S::ForAssign,A::LoopAssignment),(S::ReprCall,A::Repr),(S::AbsCall,A::Abs),(S::IterCall,A::IteratorCreation),(S::NextCall,A::IteratorNext),(S::StrCallToDunderMethod,A::StringConversion),(S::Slice,A::Slice),(S::ChainedAssign,A::ChainedAssignment),(S::FormatStringArtificial,A::FormatArtificial),(S::FormatStringStringify,A::FormatStringify)];
  for (code,(step,meaning)) in expected.into_iter().enumerate(){assert_eq!(step as i16,code as i16);assert_eq!(origin_action(step),meaning);}
  assert_ne!(origin_action(S::GeneratorIter),origin_action(S::GeneratorNext));
  assert_ne!(origin_action(S::ForIter),origin_action(S::ForNext));
 }
 #[test]
 fn native_terminal_negative_twins_do_not_acquire_a_declared_return_promise() {
  fn id<R>(n:u8)->Id<R>{serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
  let budget=ResourceBudget::fixed(1<<20).unwrap();let data=ModelApplicationData::new(&budget);
  for decision in [TerminalDecision::NarrowedNeverCallee,TerminalDecision::InferredMethodReturn,TerminalDecision::NotImplementedBody,TerminalDecision::NonDivergentCallable,TerminalDecision::NoNativeCallableSignature] {
   let row=NativeTerminalObservation{qualification:id(1),subject:id(2),callee:id(3),return_type:Some(id(4)),return_is_inferred:Some(false),is_bound_method:true,decision};
   assert!(matches!(terminal_return(&data,&row,id(5),id(6),id(7)),Err(ObligationKind::OutsideProviderModel)));
  }
  for inferred in [Some(true),None] {let row=NativeTerminalObservation{qualification:id(1),subject:id(2),callee:id(3),return_type:Some(id(4)),return_is_inferred:inferred,is_bound_method:false,decision:TerminalDecision::DeclaredOrInferredDivergence};assert!(matches!(terminal_return(&data,&row,id(5),id(6),id(7)),Err(ObligationKind::OutsideProviderModel)));}
 }
}
