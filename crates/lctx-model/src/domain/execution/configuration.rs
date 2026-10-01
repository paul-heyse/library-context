//! The bound base rule sets have fixed limits. Other authored limits/method meanings require
//! their own operation binding before activation; a producer cannot label this kernel arbitrarily.
use crate::domain::{*,analysis::{AnalysisDefinition,MethodParameters,AnalysisMethod,Interpretation}};
fn parameters()->MethodParameters {MethodParameters {depth:None,proof_steps:None,work:None,members:None,seed:None,iterations:None,threshold:None,resolution:None,damping:None,model_catalog:None}}
pub fn base_evaluation()->(MethodParameters,AnalysisDefinition) {
    let parameters=parameters();let mut key=KeySink::new("base-evaluation-rule-set");ContentHash::of(include_bytes!("evaluation.rs")).encode(&mut key);ContentHash::of(include_bytes!("production.rs")).encode(&mut key);let definition=AnalysisDefinition{method:AnalysisMethod::Execution,semantic_version:key.finish(),parameters:parameters.id(),interpretation:Interpretation::Structural};(parameters,definition)
}
pub fn base_completion()->(MethodParameters,AnalysisDefinition) {
    let parameters=parameters();let mut key=KeySink::new("base-completion-rule-set");ContentHash::of(include_bytes!("completion.rs")).encode(&mut key);ContentHash::of(include_bytes!("completion_production.rs")).encode(&mut key);ContentHash::of(include_bytes!("outcome.rs")).encode(&mut key);ContentHash::of(include_bytes!("body.rs")).encode(&mut key);
    let definition=AnalysisDefinition{method:AnalysisMethod::Completion,semantic_version:key.finish(),parameters:parameters.id(),interpretation:Interpretation::Structural};(parameters,definition)
}
