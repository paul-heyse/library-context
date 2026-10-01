//! Canonical authored definition shared by S0 and its downstream retrieval consumer.
use crate::domain::{*,analysis::*};
pub fn definition()->(MethodParameters,AnalysisDefinition){
 let parameters=MethodParameters{depth:None,proof_steps:None,work:None,members:None,seed:None,iterations:None,threshold:None,resolution:None,damping:None,model_catalog:None};
 let mut hash=KeySink::new("programmatic-synthesis-version-v2");parameters.id().encode(&mut hash);
 for code in [include_bytes!("build.rs").as_slice(),include_bytes!("documentary.rs").as_slice(),include_bytes!("source_code.rs").as_slice(),include_bytes!("source_setup.rs").as_slice(),include_bytes!("assertions.rs").as_slice(),include_bytes!("seeds.rs").as_slice(),include_bytes!("briefs.rs").as_slice(),include_bytes!("../analysis/policy.rs").as_slice()]{ContentHash::of(code).encode(&mut hash);}
 let definition=AnalysisDefinition{method:AnalysisMethod::Synthesis,parameters:parameters.id(),semantic_version:hash.finish(),interpretation:Interpretation::Structural};(parameters,definition)
}
