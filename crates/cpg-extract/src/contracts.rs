//! The ordinary provider declaration composition, shared by extraction and cold admission.
use crate::bundle::CapturedInputs;
use lctx_model::domain::{ModelError, Record, producer_contract::CapturedSourceBinding};

use lctx_model::domain::producer_contract::facts::FactsProducer;
fn executable<S: crate::bundle::ProviderSink + 'static>(
    producer: FactsProducer,
    configuration: lctx_model::domain::ContentHash,
) -> Box<dyn crate::bundle::ProviderStage<S>> {
    match producer {
        FactsProducer::Acquisition => Box::new(crate::acquisition::Acquire::new(configuration)),
        FactsProducer::Pyrefly => Box::new(crate::pyrefly_stage::Pyrefly::new(
            crate::typed_syntax::SyntaxLimits::default(),
        )),
        FactsProducer::TyFlow => Box::new(crate::ty_flow::TyFlow::default()),
        FactsProducer::Documents => Box::new(crate::document_parser::Documents),
        FactsProducer::Deployment => Box::new(crate::deployment::Deployment),
        FactsProducer::Assembly => Box::new(crate::assembly::Assemble),
    }
}
pub fn executables<S: crate::bundle::ProviderSink + 'static>(
    configuration: lctx_model::domain::ContentHash,
) -> Vec<Box<dyn crate::bundle::ProviderStage<S>>> {
    FactsProducer::ALL
        .into_iter()
        .map(|producer| executable(producer, configuration))
        .collect()
}
/// Configuration and source authority come from the captured inputs, before provider execution.
pub fn captured_sources(
    captured: &CapturedInputs,
) -> Result<Vec<CapturedSourceBinding>, ModelError> {
    let mut sources = Vec::new();
    for input in captured.inputs() {
        let library = match input.acquisition() {
            crate::acquisition::Acquisition::Corpus { library, .. } => Some(
                captured
                    .inputs()
                    .get(*library)
                    .ok_or(ModelError::Schema("captured corpus library"))?,
            ),
            _ => None,
        };
        let context = crate::pyrefly_stage::analysis_context(input, library, captured.config())?;
        sources.push(CapturedSourceBinding {
            input: input.captured().revision().id(),
            context: Some(context.id()),
            configuration: Some(context.config_digest),
            acquisition: input.descriptor(library)?.into(),
        });
    }
    Ok(sources)
}

/// Bind the actual immutable declaration before scheduling and retain it through execution.
/// Acquisition rows carry no analyzer context; Assembly supplies current attribution separately.
pub fn bind_stage(
    stage: lctx_model::domain::stages::Stage,
    captured: &CapturedInputs,
) -> Result<lctx_model::domain::stages::Stage, ModelError> {
    let sources = if stage.name == lctx_model::domain::producer_contract::facts::ACQUIRE {
        captured
            .inputs()
            .iter()
            .map(|input| {
                let library = match input.acquisition() {
                    crate::acquisition::Acquisition::Corpus { library, .. } => Some(
                        captured
                            .inputs()
                            .get(*library)
                            .ok_or(ModelError::Schema("captured corpus library"))?,
                    ),
                    _ => None,
                };
                Ok(CapturedSourceBinding {
                    input: input.captured().revision().id(),
                    context: None,
                    configuration: None,
                    acquisition: input.descriptor(library)?.into(),
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?
    } else {
        captured_sources(captured)?
    };
    bind_stage_with_sources(stage, captured, &sources)
}
/// An ordinary schedule shares the prepared acquisition descriptors across every declaration.
pub fn bind_stage_with_sources(
    mut stage: lctx_model::domain::stages::Stage,
    captured: &CapturedInputs,
    sources: &[CapturedSourceBinding],
) -> Result<lctx_model::domain::stages::Stage, ModelError> {
    use lctx_model::domain::producer_contract::{
        CapturedContextDependency, ContextDependencyPolicy,
    };
    let Some(binding) = stage.captured_binding.as_mut() else {
        return Ok(stage);
    };
    let contract = lctx_model::domain::producer_contract::facts::facts()
        .into_iter()
        .find(|contract| contract.name == stage.name)
        .ok_or(ModelError::Conflict("unknown captured facts declaration"))?;
    if binding.contract != contract.identity()
        || binding.semantic_revision != contract.semantic_revision
    {
        return Err(ModelError::Conflict(
            "captured executable declaration differs from semantic inventory",
        ));
    }
    match contract.context_dependency {
        ContextDependencyPolicy::AcquisitionOnly => {
            binding.context_dependency = CapturedContextDependency::AcquisitionOnly;
            binding.sources = sources
                .iter()
                .cloned()
                .map(|mut source| {
                    source.context = None;
                    source.configuration = None;
                    source
                })
                .collect();
        }
        ContextDependencyPolicy::SemanticContext => {
            binding.context_dependency = CapturedContextDependency::SemanticContext;
            binding.sources = sources.to_vec();
        }
        ContextDependencyPolicy::ModelSelection => {
            let mut requirements = Vec::with_capacity(captured.inputs().len());
            for input in captured.inputs() {
                let library = match input.acquisition() {
                    crate::acquisition::Acquisition::Corpus { library, .. } => Some(
                        captured
                            .inputs()
                            .get(*library)
                            .ok_or(ModelError::Schema("captured corpus library"))?,
                    ),
                    _ => None,
                };
                let context =
                    crate::pyrefly_stage::analysis_context(input, library, captured.config())?;
                requirements.push(
                    captured
                        .config()
                        .requirements(&context.python_version, captured.config().budget())?
                        .map(|requirements| requirements.identity()),
                );
            }
            binding.context_dependency = CapturedContextDependency::ModelSelection {
                catalog: (captured.config().profile()
                    == lctx_model::domain::stages::Profile::Behavioral)
                    .then(|| captured.config().catalog().digest()),
                requirements,
            };
            binding.sources = sources.to_vec();
        }
    }
    binding.validate()?;
    Ok(stage)
}
