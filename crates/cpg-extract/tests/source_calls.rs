#[path="fixtures/transfer_composition.rs"]
mod fixture;
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
 for(text,reason)in [("fresh_default()",ObligationKind::DefaultUnavailable),("fresh_async()",ObligationKind::NoSourceDeclaration)]{let a=f.attempt(text);let b=f.verified.bound(a).unwrap();let c=f.verified.composition(a).unwrap();assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,b,c,SourceCallRequest{event:b.event(),..request},&f.budget).unwrap(),Err(r) if r==reason),"{text}");}
 let coverage=std::mem::replace(&mut flow.coverage,normalized::Rows::new(&f.budget));assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,request,&f.budget).unwrap(),Err(ObligationKind::IncompleteCoverage)));flow.coverage=coverage;
 let native=std::mem::replace(&mut data.native,normalized::Rows::new(&f.budget));assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,request,&f.budget).unwrap(),Err(ObligationKind::MissingEvidence)));data.native=native;
 let other=f.attempt("identity(seed)");let foreign=f.verified.bound(other).unwrap();assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,SourceCallRequest{event:foreign.event(),..request},&f.budget).unwrap(),Err(ObligationKind::IncompatibleContexts)));
 let tiny=resources::ResourceBudget::fixed(1).unwrap();assert!(matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,checked,admission,request,&tiny),Err(ModelError::Resource{..})));
 drop(token);
 // Pure model replay uses explicit fixture invocations; it does not claim publication authority.
 let data=execution::source_call_records::SourceCallData{evaluation:data,flow,bindings:f.data,output:f.output};
 let (_,definition)=execution::configuration::source_calls();let (_,base_definition)=execution::configuration::base_completion();let (base,_)=analysis::base_completion::AnalysisInvocation::new(request.input,request.context,base_definition.id(),None,[]);let parent=analysis::source_call::InvocationSource::BaseCompletion{invocation:base.id()};let(invocation,_)=analysis::source_call::AnalysisInvocation::new(request.input,request.context,definition.id(),None,[parent.id()]);
 let records=execution::source_call_records::prepare_all(&data,&invocation,&definition,stages::Profile::Behavioral,&f.budget).unwrap();assert!(records.headers.len()>=2);assert!(!records.boundaries.is_empty());
 for mutation in 0..3{
  let relation=Relation::of::<execution::source_call_records::SourceCallRun>();let invariant=&relation.invariants()[0];let mut check=(invariant.create)(&f.budget);
  for input in execution::source_call_records::SourceCallData::inputs(){macro_rules! feed{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME{let batch=<$ty as Record>::encode(&data.bindings.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::normalized_binding_inputs!(feed);
   macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME&&!normalized::binding_normalization::BindingData::validation_inputs().iter().any(|i|i.name()==input.name()){let batch=<$ty as Record>::encode(&data.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::execution_evaluation_inputs!(raw);
   macro_rules! flow{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME&&!normalized::binding_normalization::BindingData::validation_inputs().iter().any(|i|i.name()==input.name())&&!execution::evaluation::EvaluationData::validation_inputs().iter().any(|i|i.name()==input.name()){let batch=<$ty as Record>::encode(&data.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::entry_value_inputs!(flow);
   macro_rules! outputs{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME{let batch=<$ty as Record>::encode(&data.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}lctx_model::normalized_binding_outputs!(outputs);
  }
  macro_rules! put{($ty:ty,$rows:expr)=>{check.visit(<$ty>::NAME,&<$ty as Record>::encode(&$rows).unwrap()).unwrap()};}
  put!(analysis::source_call::AnalysisInvocation,vec![invocation.clone()]);put!(analysis::base_completion::AnalysisInvocation,vec![base.clone()]);put!(analysis::AnalysisDefinition,vec![definition.clone()]);put!(analysis::source_call::AnalysisOutcome,vec![records.outcome.clone()]);put!(execution::source_call_records::SourceCallBoundary,records.boundaries.iter().cloned().collect::<Vec<_>>());
  let mut run=records.run.clone();let mut headers=records.headers.iter().cloned().collect::<Vec<_>>();let mut members=records.members.iter().cloned().collect::<Vec<_>>();
  if mutation==1{let old=headers[0].id();headers[0].status=analysis::policy::EvidenceStatus::Documented;let new=headers[0].id();for member in &mut members{if member.header==old{member.header=new;}}}
  if mutation==2{let removed=headers.pop().unwrap();members.retain(|m|m.header!=removed.id());run.bound-=1;}
  put!(execution::source_call_records::SourceCallRun,vec![run]);put!(execution::source_call_records::SourceCallHeader,headers);put!(execution::source_call_records::HeaderMember,members);let result=check.finish();assert_eq!(result.is_ok(),mutation==0,"mutation={mutation}: {result:?}");
 }

}
