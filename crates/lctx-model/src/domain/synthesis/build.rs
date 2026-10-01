//! Canonical authored definition shared by S0 and its downstream retrieval consumer.
use crate::domain::{*,analysis::*};
pub fn definition()->(MethodParameters,AnalysisDefinition){
 let parameters=MethodParameters{depth:None,proof_steps:None,work:None,members:None,seed:None,iterations:None,threshold:None,resolution:None,damping:None,model_catalog:None};
 let definition=AnalysisDefinition{method:AnalysisMethod::Synthesis,parameters:parameters.id(),semantic_version:ContentHash::of(b"programmatic-synthesis-v1"),interpretation:Interpretation::Structural};(parameters,definition)
}
