//! The ordinary provider declaration composition, shared by extraction and cold admission.
use crate::bundle::CapturedInputs;
use lctx_model::domain::{ModelError, Record, attribution::FactFamily, producer_contract::{CapturedSourceBinding, ConfigurationBinding, ProducerContract, SupplierRole}};

pub fn supplier(name:&'static str,tool:&'static str,analyzer_revision:&'static str,families:Vec<FactFamily>)->SupplierRole {
    SupplierRole{name,tool,analyzer_revision,semantic_revision:1,families,configuration:ConfigurationBinding::CapturedAnalysisContext}
}
#[derive(Clone,Copy)]
enum FactsProducer { Acquisition, Pyrefly, TyFlow, Documents, Deployment, Assembly }
fn composition()->[FactsProducer;6] {[FactsProducer::Acquisition,FactsProducer::Pyrefly,FactsProducer::TyFlow,FactsProducer::Documents,FactsProducer::Deployment,FactsProducer::Assembly]}
impl FactsProducer {
    fn contract(self)->ProducerContract {match self {Self::Acquisition=>crate::acquisition::contract(),Self::Pyrefly=>crate::pyrefly_stage::contract(),Self::TyFlow=>crate::ty_flow::contract(),Self::Documents=>crate::document_parser::contract(),Self::Deployment=>crate::deployment::contract(),Self::Assembly=>crate::assembly::contract()}}
    fn executable<S:crate::bundle::ProviderSink+'static>(self,configuration:lctx_model::domain::ContentHash)->Box<dyn crate::bundle::ProviderStage<S>> {
        match self {Self::Acquisition=>Box::new(crate::acquisition::Acquire::new(configuration)),Self::Pyrefly=>Box::new(crate::pyrefly_stage::Pyrefly::new(crate::typed_syntax::SyntaxLimits::default())),Self::TyFlow=>Box::new(crate::ty_flow::TyFlow::default()),Self::Documents=>Box::new(crate::document_parser::Documents),Self::Deployment=>Box::new(crate::deployment::Deployment),Self::Assembly=>Box::new(crate::assembly::Assemble)}
    }
}
pub fn facts()->Vec<ProducerContract> {composition().into_iter().map(FactsProducer::contract).collect()}
pub fn executables<S:crate::bundle::ProviderSink+'static>(configuration:lctx_model::domain::ContentHash)->Vec<Box<dyn crate::bundle::ProviderStage<S>>> {composition().into_iter().map(|producer|producer.executable(configuration)).collect()}
/// Configuration and source authority come from the captured inputs, before provider execution.
pub fn captured_sources(captured:&CapturedInputs)->Result<Vec<CapturedSourceBinding>,ModelError> {
    let mut sources=Vec::new();
    for input in captured.inputs() {
        let library=match input.acquisition(){crate::acquisition::Acquisition::Corpus{library,..}=>Some(captured.inputs().get(*library).ok_or(ModelError::Schema("captured corpus library"))?),_=>None};
        let context=crate::pyrefly_stage::analysis_context(input,library,captured.config())?;
        sources.push(CapturedSourceBinding{input:input.captured().revision().id(),context:context.id(),configuration:context.config_digest});
    }
    sources.sort_by_key(|source|source.input);
    if sources.windows(2).any(|pair|pair[0].input==pair[1].input){return Err(ModelError::Conflict("captured source binding"));}
    Ok(sources)
}
