//! Real native/normalized inputs and actual base operation outputs for retained source controls.
use lctx_model::domain::{*,analysis,execution,normalized::Rows};
use super::fixture::NativeFixture;
pub fn data(f:&NativeFixture)->execution::source_call_records::SourceCallData {
 let mut data=execution::source_call_records::SourceCallData::new(&f.budget);
 for(name,batch)in f.tables.lock().unwrap().iter(){data.visit(name,batch).unwrap();}
 macro_rules! native{($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}lctx_model::normalized_binding_inputs!(native);
 macro_rules! bound{($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}lctx_model::normalized_binding_outputs!(bound);
 let mut inventory=analysis::native::NativeInventory::new(&f.budget);
 for(name,batch)in f.tables.lock().unwrap().iter(){if analysis::native::NativeInventory::inputs().iter().any(|input|input.name()==*name){inventory.visit(name,batch).unwrap();}}
 let inventory=inventory.collect().unwrap();
 data.visit(analysis::native::NativeAssertionPremise::NAME,&<analysis::native::NativeAssertionPremise as Record>::encode(&inventory.premises.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();
 data.visit(analysis::native::NativeQualification::NAME,&analysis::native::NativeQualification::encode(&inventory.qualifications.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();
 data
}
/// Explicit pure fixture frame; this helper grants no store publication authority.
pub fn base(data:&mut execution::source_call_records::SourceCallData,input:Id<input::InputRevision>,context:Id<attribution::AnalysisContext>,budget:&resources::ResourceBudget)->analysis::base_completion::AnalysisInvocation {
 let(_,definition)=execution::configuration::base_evaluation();let(invocation,_)=analysis::base_evaluation::AnalysisInvocation::new(input,context,definition.id(),None,[]);
 let output=execution::production::evaluate_all(&data.evaluation,&data.flow,&Rows::new(budget),&Rows::new(budget),&invocation,&definition,stages::Profile::Behavioral,budget).unwrap();
 macro_rules! put{($ty:ty,$rows:expr)=>{data.visit(<$ty>::NAME,&<$ty as Record>::encode(&$rows).unwrap()).unwrap()};}
 put!(analysis::base_evaluation::AnalysisInvocation,vec![invocation]);put!(analysis::AnalysisDefinition,vec![definition]);
 put!(execution::records::ExpressionEvaluation,output.evaluations.iter().cloned().collect::<Vec<_>>());put!(execution::records::EvaluationSource,output.sources.iter().cloned().collect::<Vec<_>>());put!(execution::records::EvaluationMember,output.members.iter().cloned().collect::<Vec<_>>());put!(execution::records::EvaluationOperand,output.operands.iter().cloned().collect::<Vec<_>>());
 let(_,definition)=execution::configuration::base_completion();let(invocation,_)=analysis::base_completion::AnalysisInvocation::new(input,context,definition.id(),None,[]);
 let output=execution::completion_production::complete_all(&data.completed,&invocation,&definition,stages::Profile::Behavioral,budget).unwrap();
 put!(analysis::base_completion::AnalysisInvocation,vec![invocation.clone()]);put!(analysis::AnalysisDefinition,vec![definition]);put!(execution::body_records::SourceBodyCompletion,output.bodies.iter().cloned().collect::<Vec<_>>());
 invocation
}
