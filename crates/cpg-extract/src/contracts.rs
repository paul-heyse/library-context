//! The ordinary provider declaration composition, shared by extraction and cold admission.
use crate::bundle::CapturedInputs;
use lctx_model::domain::{ModelError, Record, producer_contract::CapturedSourceBinding};

use lctx_model::domain::producer_contract::facts::FactsProducer;
fn executable<S:crate::bundle::ProviderSink+'static>(producer:FactsProducer,configuration:lctx_model::domain::ContentHash)->Box<dyn crate::bundle::ProviderStage<S>> {
    match producer {FactsProducer::Acquisition=>Box::new(crate::acquisition::Acquire::new(configuration)),FactsProducer::Pyrefly=>Box::new(crate::pyrefly_stage::Pyrefly::new(crate::typed_syntax::SyntaxLimits::default())),FactsProducer::TyFlow=>Box::new(crate::ty_flow::TyFlow::default()),FactsProducer::Documents=>Box::new(crate::document_parser::Documents),FactsProducer::Deployment=>Box::new(crate::deployment::Deployment),FactsProducer::Assembly=>Box::new(crate::assembly::Assemble)}
}
pub fn executables<S:crate::bundle::ProviderSink+'static>(configuration:lctx_model::domain::ContentHash)->Vec<Box<dyn crate::bundle::ProviderStage<S>>> {FactsProducer::ALL.into_iter().map(|producer|executable(producer, configuration)).collect()}
/// Configuration and source authority come from the captured inputs, before provider execution.
pub fn captured_sources(captured:&CapturedInputs)->Result<Vec<CapturedSourceBinding>,ModelError> {
    let mut sources=Vec::new();
    for input in captured.inputs() {
        let library=match input.acquisition(){crate::acquisition::Acquisition::Corpus{library,..}=>Some(captured.inputs().get(*library).ok_or(ModelError::Schema("captured corpus library"))?),_=>None};
        let context=crate::pyrefly_stage::analysis_context(input,library,captured.config())?;
        sources.push(CapturedSourceBinding{input:input.captured().revision().id(),context:context.id(),configuration:context.config_digest,acquisition:input.descriptor(library)?.into()});
    }
    Ok(sources)
}
