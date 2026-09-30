//! The facts driver (cutover plan A0). Every scheduled stage runs, in schedule order, through the
//! provider that declares it, into one attempt's sink. The providers are exactly the schedule's
//! stages for its profile, and each declaration equals its scheduled stage; anything else is
//! refused before any stage runs. Ambient analyzer configuration is refused first.
use std::sync::Arc;
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use lctx_model::domain::{ModelError, ValidatedModel, batching::TransferLimits, resources::ResourceBudget,
    stages::{Execution, ExecutionReceipt, StageSink}};

pub async fn compile_facts<S: StageSink + 'static>(mut execution: Execution<'_>, providers: Vec<Box<dyn ProviderStage<S>>>, sink: &S,
    model: &Arc<ValidatedModel>, captured: &Arc<CapturedInputs>, budget: &ResourceBudget) -> Result<ExecutionReceipt, ModelError> {
    bundle::refuse_ambient(std::env::vars_os())?;
    let profile = execution.schedule().profile();
    let mut offered: Vec<_> = providers.into_iter().map(|provider| (provider.declaration(profile), provider))
        .filter(|(declaration, _)| declaration.profiles.contains(&profile)).collect();
    let mut ordered = Vec::new();
    for stage in execution.schedule().stages() {
        let position = offered.iter().position(|(declaration, _)| declaration.name == stage.name)
            .ok_or_else(|| ModelError::Invalid(format!("no provider declares the scheduled stage {}", stage.name)))?;
        let (declaration, provider) = offered.swap_remove(position);
        if declaration.digest() != stage.digest() {
            return Err(ModelError::Invalid(format!("the {} provider's declaration differs from its scheduled stage", stage.name)));
        }
        ordered.push((stage.name, provider));
    }
    if let Some((declaration, _)) = offered.first() {
        return Err(ModelError::Invalid(format!("the {} provider is not scheduled for the {} profile", declaration.name, profile.name())));
    }
    for (name, provider) in ordered {
        let access = execution.begin(name)?;
        bundle::run_stage(provider, access, sink, model, captured, budget, TransferLimits::default()).await?;
    }
    execution.finish()
}
