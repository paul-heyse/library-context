#[path="fixtures/transfer_composition.rs"]
mod fixture;
#[path="fixtures/source_execution.rs"]
mod source_fixture;
use lctx_model::domain::{*,execution::{source_call::*,evaluation::EvaluationData},conditions::entry::EntryData,analysis::native::NativeInventory,source::*,obligation::ObligationKind};
#[tokio::test]
async fn fresh_binding_is_independent_of_body_and_exact_to_admitted_event(){
 let f=fixture::native().await;let mut data=EvaluationData::new(&f.budget);let mut flow=EntryData::new(&f.budget);let mut inventory=NativeInventory::new(&f.budget);let inputs=NativeInventory::inputs();
 for(name,batch)in f.tables.lock().unwrap().iter(){data.visit(name,batch).unwrap();flow.visit(name,batch).unwrap();if inputs.iter().any(|input|input.name()==*name){inventory.visit(name,batch).unwrap();}}
 macro_rules! normalized{($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap();data.visit(<$ty>::NAME,&batch).unwrap();flow.visit(<$ty>::NAME,&batch).unwrap();)*};}
 lctx_model::normalized_binding_inputs!(normalized);
 let inventory=inventory.collect().unwrap();data.premises=inventory.premises;data.native=inventory.qualifications;
 let id=f.attempt("fresh()");let checked=f.verified.bound(id).unwrap();let admission=f.verified.composition(id).unwrap();let site=f.data.occurrences.get(checked.bound().site()).unwrap();let request=SourceCallRequest{input:f.data.artifacts.get(site.source).unwrap().input,context:checked.context(),event:checked.event()};
 let result=CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,request,&f.budget).unwrap();assert!(result.is_ok(),"fresh: {:?}",result.as_ref().err());let token=result.ok().unwrap();assert!(!token.premises().is_empty());assert_eq!(token.callee(),admission.callee());
 // A body that calls an unknown operation does not invalidate independently proven fresh binding.
 let body=f.attempt("fresh_effect()");let body_checked=f.verified.bound(body).unwrap();let body_admission=f.verified.composition(body).unwrap();assert!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,body_checked,body_admission,SourceCallRequest{event:body_checked.event(),..request},&f.budget).unwrap().is_ok());
 let metadata=f.attempt("metadata_inner()");let m=f.verified.bound(metadata).unwrap();assert!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,m,f.verified.composition(metadata).unwrap(),SourceCallRequest{event:m.event(),..request},&f.budget).unwrap().is_ok());
 for(text,reason)in [("fresh_default()",ObligationKind::DefaultUnavailable),("fresh_async()",ObligationKind::NoSourceDeclaration),("captured_inner()",ObligationKind::CapturedStateUnavailable),("intervening_inner()",ObligationKind::EntryValueUnknown)]{let a=f.attempt(text);let b=f.verified.bound(a).unwrap();let c=f.verified.composition(a).unwrap();assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,b,c,SourceCallRequest{event:b.event(),..request},&f.budget).unwrap(),Err(r) if r==reason),"{text}");}
 let coverage=std::mem::replace(&mut flow.coverage,normalized::Rows::new(&f.budget));assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,request,&f.budget).unwrap(),Err(ObligationKind::IncompleteCoverage)));flow.coverage=coverage;
 let native=std::mem::replace(&mut data.native,normalized::Rows::new(&f.budget));assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,request,&f.budget).unwrap(),Err(ObligationKind::MissingEvidence)));data.native=native;
 let other=f.attempt("identity(seed)");let foreign=f.verified.bound(other).unwrap();assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,SourceCallRequest{event:foreign.event(),..request},&f.budget).unwrap(),Err(ObligationKind::IncompatibleContexts)));
 let tiny=resources::ResourceBudget::fixed(1).unwrap();assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,request,&tiny),Err(ModelError::Resource{..})));
 drop(token);
 // Pure model replay uses explicit fixture invocations; it does not claim publication authority.

 let mut source_data=execution::source_call_records::SourceCallData::new(&f.budget);source_data.evaluation=data;source_data.flow=flow;source_data.bindings=f.data;source_data.output=f.output;
 macro_rules! raw_data{($($field:ident:$ty:ty,)*)=>{$(source_data.completed.visit(<$ty>::NAME,&<$ty as Record>::encode(&source_data.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}lctx_model::execution_evaluation_inputs!(raw_data);
 macro_rules! entry_data{($($field:ident:$ty:ty,)*)=>{$(source_data.completed.visit(<$ty>::NAME,&<$ty as Record>::encode(&source_data.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}lctx_model::entry_value_inputs!(entry_data);
 let(_,evaluation_definition)=execution::configuration::base_evaluation();let(evaluation_invocation,_)=analysis::base_evaluation::AnalysisInvocation::new(request.input,request.context,evaluation_definition.id(),None,[]);
 let empty_entries=normalized::Rows::new(&f.budget);let empty_sources=normalized::Rows::new(&f.budget);let evaluations=execution::production::evaluate_all(&source_data.evaluation,&source_data.flow,&empty_entries,&empty_sources,&evaluation_invocation,&evaluation_definition,stages::Profile::Behavioral,&f.budget).unwrap();
 macro_rules! earlier{($ty:ty,$rows:expr)=>{source_data.visit(<$ty>::NAME,&<$ty as Record>::encode(&$rows).unwrap()).unwrap()};}
 earlier!(analysis::base_evaluation::AnalysisInvocation,vec![evaluation_invocation]);earlier!(analysis::AnalysisDefinition,vec![evaluation_definition]);
 earlier!(execution::records::ExpressionEvaluation,evaluations.evaluations.iter().cloned().collect::<Vec<_>>());earlier!(execution::records::EvaluationSource,evaluations.sources.iter().cloned().collect::<Vec<_>>());earlier!(execution::records::EvaluationMember,evaluations.members.iter().cloned().collect::<Vec<_>>());earlier!(execution::records::EvaluationOperand,evaluations.operands.iter().cloned().collect::<Vec<_>>());
 let(_,base_definition)=execution::configuration::base_completion();let(base,_)=analysis::base_completion::AnalysisInvocation::new(request.input,request.context,base_definition.id(),None,[]);let bodies=execution::completion_production::complete_all(&source_data.completed,&base,&base_definition,stages::Profile::Behavioral,&f.budget).unwrap();earlier!(analysis::base_completion::AnalysisInvocation,vec![base.clone()]);earlier!(analysis::AnalysisDefinition,vec![base_definition.clone()]);earlier!(execution::body_records::SourceBodyCompletion,bodies.bodies.iter().cloned().collect::<Vec<_>>());
 let data=source_data;
 let (_,definition)=execution::configuration::source_calls();let parent=analysis::source_call::InvocationSource::BaseCompletion{invocation:base.id()};let(invocation,_)=analysis::source_call::AnalysisInvocation::new(request.input,request.context,definition.id(),None,[parent.id()]);
 let records=execution::source_call_records::prepare_all(&data,&invocation,&definition,stages::Profile::Behavioral,&f.budget).unwrap();assert!(records.headers.len()>=2);assert!(!records.boundaries.is_empty());assert!(!records.invocations.is_empty());assert!(!records.invocation_boundaries.is_empty());
 for mutation in 0..5{
  let relation=Relation::of::<execution::source_call_records::SourceCallRun>();let invariant=&relation.invariants()[0];let mut check=(invariant.create)(&f.budget);
  for input in execution::source_call_records::SourceCallData::inputs(){macro_rules! feed{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME{let batch=<$ty as Record>::encode(&data.bindings.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::normalized_binding_inputs!(feed);
   macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME&&!normalized::binding_normalization::BindingData::validation_inputs().iter().any(|i|i.name()==input.name()){let batch=<$ty as Record>::encode(&data.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::execution_evaluation_inputs!(raw);
   macro_rules! flow{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME&&!normalized::binding_normalization::BindingData::validation_inputs().iter().any(|i|i.name()==input.name())&&!execution::evaluation::EvaluationData::validation_inputs().iter().any(|i|i.name()==input.name()){let batch=<$ty as Record>::encode(&data.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::entry_value_inputs!(flow);
   macro_rules! outputs{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME{let batch=<$ty as Record>::encode(&data.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::normalized_binding_outputs!(outputs);
  }
  macro_rules! put{($ty:ty,$rows:expr)=>{check.visit(<$ty>::NAME,&<$ty as Record>::encode(&$rows).unwrap()).unwrap()};}
  put!(analysis::source_call::AnalysisInvocation,vec![invocation.clone()]);put!(analysis::base_completion::AnalysisInvocation,vec![base.clone()]);put!(analysis::AnalysisDefinition,vec![definition.clone()]);put!(analysis::source_call::AnalysisOutcome,vec![records.outcome.clone()]);put!(execution::source_call_records::SourceCallBoundary,records.boundaries.iter().cloned().collect::<Vec<_>>());
  let mut invocations=records.invocations.iter().cloned().collect::<Vec<_>>();let mut outcomes=records.call_outcomes.iter().cloned().collect::<Vec<_>>();if mutation==3{invocations[0].status=analysis::policy::EvidenceStatus::Documented;}if mutation==4{let outcome=execution::source_call_records::SourceCallOutcome::Raised{site:records.headers.iter().next().unwrap().declaration,exception:execution::ExactRuntimeException::TypeError};invocations[0].outcome=outcome.id();outcomes.push(outcome);}put!(execution::source_call_records::SourceInvocation,invocations);put!(execution::source_call_records::SourceFrameRelease,records.releases.iter().cloned().collect::<Vec<_>>());put!(execution::source_call_records::SourceCallOutcome,outcomes);put!(execution::source_call_records::InvocationBoundary,records.invocation_boundaries.iter().cloned().collect::<Vec<_>>());
  put!(analysis::base_evaluation::AnalysisInvocation,vec![evaluations.run.invocation].into_iter().map(|id|analysis::base_evaluation::AnalysisInvocation::new(request.input,request.context,execution::configuration::base_evaluation().1.id(),None,[]).0).collect::<Vec<_>>());
  put!(analysis::AnalysisDefinition,vec![execution::configuration::base_evaluation().1,base_definition.clone()]);
  put!(execution::records::ExpressionEvaluation,evaluations.evaluations.iter().cloned().collect::<Vec<_>>());put!(execution::records::EvaluationSource,evaluations.sources.iter().cloned().collect::<Vec<_>>());put!(execution::records::EvaluationMember,evaluations.members.iter().cloned().collect::<Vec<_>>());put!(execution::records::EvaluationOperand,evaluations.operands.iter().cloned().collect::<Vec<_>>());put!(execution::body_records::SourceBodyCompletion,bodies.bodies.iter().cloned().collect::<Vec<_>>());
  let mut run=records.run.clone();let mut headers=records.headers.iter().cloned().collect::<Vec<_>>();let mut members=records.members.iter().cloned().collect::<Vec<_>>();
  if mutation==1{let old=headers[0].id();headers[0].status=analysis::policy::EvidenceStatus::Documented;let new=headers[0].id();for member in &mut members{if member.header==old{member.header=new;}}}
  if mutation==2{let removed=headers.pop().unwrap();members.retain(|m|m.header!=removed.id());run.bound-=1;}
  put!(execution::source_call_records::SourceCallRun,vec![run]);put!(execution::source_call_records::SourceCallHeader,headers);put!(execution::source_call_records::HeaderMember,members);let result=check.finish();assert_eq!(result.is_ok(),mutation==0,"mutation={mutation}: {result:?}");
 }

}

#[tokio::test]
async fn retained_source_shapes_preserve_invocation_and_frame_boundaries(){
 let f=fixture::native_from("source_body_shapes").await;let mut data=source_fixture::data(&f);
 let input=f.rows::<input::InputRevision>()[0].id();let context=f.data.event_events.iter().next().unwrap().context;
 let base=source_fixture::base(&mut data,input,context,&f.budget);
 let(_,definition)=execution::configuration::source_calls();let parent=analysis::source_call::InvocationSource::BaseCompletion{invocation:base.id()};let(invocation,_)=analysis::source_call::AnalysisInvocation::new(input,context,definition.id(),None,[parent.id()]);
 let records=execution::source_call_records::prepare_all(&data,&invocation,&definition,stages::Profile::Behavioral,&f.budget).unwrap();
 let name=|owner:Id<normalized::entities::EntityRef>|{let normalized::entities::EntityRef::Callable{callable}=data.bindings.refs.get(owner).unwrap()else{panic!("source caller")};let normalized::entities::CallableEntity::Source{declaration,..}=data.bindings.callables.get(*callable).unwrap()else{panic!("source caller")};let name=data.bindings.declarations.iter().find(|row|row.declaration==*declaration).unwrap().name;data.evaluation.spellings.iter().find(|row|row.occurrence==name).unwrap().spelling.clone()};
 for caller in ["call_literal","call_fallthrough","call_return_finally"]{
  let headers=records.headers.iter().filter(|row|name(row.owner)==caller).collect::<Vec<_>>();assert_eq!(headers.len(),1,"{caller}: fresh header");
  let release=records.releases.iter().find(|row|row.header==headers[0].id()).unwrap_or_else(||panic!("{caller}: frame unavailable"));let call=records.invocations.iter().find(|row|row.release==release.id()).unwrap();assert_eq!(records.call_outcomes.get(call.outcome),Some(&execution::source_call_records::SourceCallOutcome::Normal),"{caller}");
 }
 let raised=records.headers.iter().find(|row|name(row.owner)=="call_raises").unwrap();let release=records.releases.iter().find(|row|row.header==raised.id()).unwrap();let call=records.invocations.iter().find(|row|row.release==release.id()).unwrap();assert!(matches!(records.call_outcomes.get(call.outcome),Some(execution::source_call_records::SourceCallOutcome::Raised{exception:execution::ExactRuntimeException::TypeError,..})));
 for caller in ["call_local_read","call_default","call_captured","call_intervening","call_alias","call_failed_header","call_unknown_body","call_generator","call_unreachable_yield"]{
  assert!(!records.invocations.iter().any(|row|records.call_outcomes.get(row.outcome)==Some(&execution::source_call_records::SourceCallOutcome::Normal)&&records.releases.get(row.release).and_then(|release|records.headers.get(release.header)).is_some_and(|header|name(header.owner)==caller)),"{caller}: fabricated normal invocation");
 }
 let(_,replayed)=execution::enriched::with_frame(&data,&invocation,&definition,&f.budget,|frame|{
  for caller in ["call_literal","call_fallthrough","call_return_finally","call_raises"]{
   let header=records.headers.iter().find(|row|name(row.owner)==caller).unwrap();let event=data.bindings.event_events.get(header.event).unwrap();
   let mut current=event.site;let statement=loop{let placement=data.evaluation.placements.iter().find(|row|row.occurrence==current).unwrap();if data.evaluation.occurrences.get(current).unwrap().syntax_kind==source::SyntaxKind::StmtExpr{break current;}current=placement.parent.unwrap();};
   let result=frame.complete(execution::completion::CompletionRequest{input,context,owner:header.owner,statement})?.unwrap();
   if caller=="call_raises"{assert!(matches!(result.outcome(),execution::outcome::PendingOutcome::Raise{exception:execution::ExactRuntimeException::TypeError,..}));}else{assert_eq!(result.outcome(),execution::outcome::PendingOutcome::Normal);}
   assert!(result.emit_base(&base,&execution::configuration::base_completion().1,&[],&f.budget).is_err(),"enriched evidence laundered into Base");
  }
  Ok(())
 }).unwrap();assert!(records.invocations.same(&replayed.invocations));
 // Authored call_modeled needs the earlier applicability operation, separate from source-only
 // invocation. Caller reach and Summary witnesses are qualified by downstream controls.
}
