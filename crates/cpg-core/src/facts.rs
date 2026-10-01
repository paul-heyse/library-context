//! The facts driver (cutover plan A0). Every scheduled stage runs, in schedule order, through the
//! provider that declares it, into one attempt's sink. The providers are exactly the schedule's
//! stages for its profile, and each declaration equals its scheduled stage; anything else is
//! refused before any stage runs. Ambient analyzer configuration is refused first.
use crate::stage_runtime::{StageMeasurement, run_declared_stage_with_resources};
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use lctx_model::domain::{
    ModelError, ValidatedModel,
    batching::TransferLimits,
    resources::ResourceBudget,
    stages::{Execution, ExecutionReceipt, StageSink},
};
use std::sync::Arc;

pub async fn compile_facts<S: StageSink + 'static>(
    execution: Execution<'_>,
    providers: Vec<Box<dyn ProviderStage<S>>>,
    sink: &S,
    model: &Arc<ValidatedModel>,
    captured: &Arc<CapturedInputs>,
    budget: &ResourceBudget,
) -> Result<ExecutionReceipt, ModelError> {
    compile_facts_measured(execution, providers, sink, model, captured, budget, |_| {}).await
}
pub async fn compile_facts_measured<S: StageSink + 'static>(
    mut execution: Execution<'_>,
    providers: Vec<Box<dyn ProviderStage<S>>>,
    sink: &S,
    model: &Arc<ValidatedModel>,
    captured: &Arc<CapturedInputs>,
    budget: &ResourceBudget,
    mut observe: impl FnMut(StageMeasurement),
) -> Result<ExecutionReceipt, ModelError> {
    bundle::refuse_ambient(std::env::vars_os())?;
    let profile = execution.schedule().profile();
    captured.config().check_profile(profile)?;
    captured.config().check_budget(&budget)?;
    let mut offered: Vec<_> = providers
        .into_iter()
        .map(|provider| (provider.declaration(profile), provider))
        .filter(|(declaration, _)| declaration.profiles.contains(&profile))
        .collect();
    let mut ordered = Vec::new();
    for stage in execution.schedule().stages() {
        let position = offered
            .iter()
            .position(|(declaration, _)| declaration.name == stage.name)
            .ok_or_else(|| {
                ModelError::Invalid(format!(
                    "no provider declares the scheduled stage {}",
                    stage.name
                ))
            })?;
        let (declaration, provider) = offered.swap_remove(position);
        if declaration.digest() != stage.digest() {
            return Err(ModelError::Invalid(format!(
                "the {} provider's declaration differs from its scheduled stage",
                stage.name
            )));
        }
        ordered.push((stage.name, provider));
    }
    if let Some((declaration, _)) = offered.first() {
        return Err(ModelError::Invalid(format!(
            "the {} provider is not scheduled for the {} profile",
            declaration.name,
            profile.name()
        )));
    }
    for (_name, provider) in ordered {
        let declaration = provider.declaration(profile);
        run_declared_stage_with_resources(
            &mut execution,
            &declaration,
            budget,
            async move |access| {
                bundle::run_stage(
                    provider,
                    access,
                    sink,
                    model,
                    captured,
                    budget,
                    TransferLimits::default(),
                )
                .await
            },
            &mut observe,
        )
        .await?;
    }
    execution.finish()
}

/// The current facts frontier. Each provider declares its profile and relations once; schedule
/// preflight chooses the available stages before an attempt can write a sink.
pub fn providers<S: StageSink + 'static>(
    configuration: lctx_model::domain::ContentHash,
) -> Vec<Box<dyn ProviderStage<S>>> {
    vec![
        Box::new(cpg_extract::acquisition::Acquire::new(configuration)),
        Box::new(cpg_extract::pyrefly_stage::Pyrefly::new(
            cpg_extract::typed_syntax::SyntaxLimits::default(),
        )),
        Box::new(cpg_extract::ty_flow::TyFlow::default()),
        Box::new(cpg_extract::document_parser::Documents),
        Box::new(cpg_extract::deployment::Deployment),
        Box::new(cpg_extract::assembly::Assemble),
    ]
}
/// Bounded developer inspection over the real fact producers and their shared validators. The
/// product facts compile uses a generation-store attempt rather than this in-memory helper.
pub async fn inspect(
    captured: Arc<CapturedInputs>,
    budget: ResourceBudget,
    profile: lctx_model::domain::stages::Profile,
) -> Result<
    (
        Arc<ValidatedModel>,
        lctx_model::domain::memory::MemoryGeneration,
        lctx_model::domain::ContentHash,
    ),
    ModelError,
> {
    let model = Arc::new(lctx_model::domain::model()?);
    let providers = providers::<lctx_model::domain::memory::MemoryGeneration>(
        lctx_model::domain::ContentHash::of(b"facts-inspection"),
    );
    let declarations = providers.iter().map(|p| p.declaration(profile)).collect();
    let schedule = lctx_model::domain::stages::Schedule::build(&model, declarations, &[], profile)?;
    let mut execution = schedule.execute();
    let generation =
        lctx_model::domain::memory::MemoryGeneration::bind(&model, &budget, &mut execution)?;
    let preflight = lctx_model::domain::admission::FrontierContract::facts(&model, profile)?
        .preflight(&schedule)?;
    let receipt = compile_facts(
        execution,
        providers,
        &generation,
        &model,
        &captured,
        &budget,
    )
    .await?;
    let digest = generation
        .validate_facts(&model, &budget, preflight, &receipt)?
        .content();
    Ok((model, generation, digest))
}

/// A published facts generation. Publication leaves selection to the operator.
#[derive(Debug)]
pub struct PublishedGeneration {
    pub generation: lctx_postgres::generations::GenerationId,
    pub content: lctx_model::domain::ContentHash,
    pub availability: std::collections::BTreeMap<
        lctx_model::domain::attribution::FactFamily,
        lctx_model::domain::admission::Availability,
    >,
    pub measurements: Vec<StageMeasurement>,
}
/// Execute the full facts frontier into its owned PostgreSQL attempt. Producer failures remove
/// every generation record; validation/publication failures are acknowledged before cleanup.
pub async fn publish(
    store: &lctx_postgres::generations::GenerationStore,
    writer: sqlx::PgPool,
    captured: Arc<CapturedInputs>,
    budget: ResourceBudget,
    profile: lctx_model::domain::stages::Profile,
    configuration: lctx_model::domain::ContentHash,
) -> Result<PublishedGeneration, ModelError> {
    publish_declared(
        store,
        writer,
        captured,
        budget,
        profile,
        providers::<lctx_postgres::generations::GenerationAttempt>(configuration),
    )
    .await
}
/// The same lifecycle for an explicitly declared complete frontier. Schedule and admission
/// preflight refuse missing or extra capabilities before a generation is created.
pub async fn publish_declared(
    store: &lctx_postgres::generations::GenerationStore,
    writer: sqlx::PgPool,
    captured: Arc<CapturedInputs>,
    budget: ResourceBudget,
    profile: lctx_model::domain::stages::Profile,
    providers: Vec<Box<dyn ProviderStage<lctx_postgres::generations::GenerationAttempt>>>,
) -> Result<PublishedGeneration, ModelError> {
    bundle::refuse_ambient(std::env::vars_os())?;
    captured.config().check_profile(profile)?;
    captured.config().check_budget(&budget)?;
    let model = Arc::new(lctx_model::domain::model()?);
    if store.model().digest() != model.digest() {
        return Err(ModelError::Invalid(
            "facts store model differs from compiler".into(),
        ));
    }
    let schedule = lctx_model::domain::stages::Schedule::build(
        &model,
        providers.iter().map(|p| p.declaration(profile)).collect(),
        &[],
        profile,
    )?;
    let contract = lctx_model::domain::admission::FrontierContract::facts(&model, profile)?;
    contract.preflight(&schedule)?;
    for input in captured.inputs() {
        input
            .captured()
            .verify()
            .map_err(|e| ModelError::Invalid(e.to_string()))?;
    }
    let mut execution = schedule.execute();
    let attempt = store
        .begin(writer, &mut execution, &contract, budget.clone())
        .await
        .map_err(ModelError::from)?;
    let id = attempt.generation();
    let mut measurements = Vec::new();
    let receipt = match compile_facts_measured(
        execution,
        providers,
        &attempt,
        &model,
        &captured,
        &budget,
        |measurement| {
            tracing::info!(
                stage = measurement.stage,
                elapsed_ms = measurement.elapsed_ms as u64,
                sampled_peak_rss_bytes = measurement.sampled_peak_rss_bytes,
                outcome = measurement.outcome,
                "facts stage"
            );
            measurements.push(measurement);
        },
    )
    .await
    {
        Ok(receipt) => receipt,
        Err(error) => {
            if let Err(cleanup) = attempt.abort().await {
                return Err(cleanup_error(
                    id,
                    "provider",
                    &error,
                    &ModelError::from(cleanup),
                ));
            }
            return Err(error);
        }
    };
    finish_publication(store, attempt, receipt, measurements).await
}
pub(crate) async fn finish_publication(
    store: &lctx_postgres::generations::GenerationStore,
    attempt: lctx_postgres::generations::GenerationAttempt,
    receipt: ExecutionReceipt,
    measurements: Vec<StageMeasurement>,
) -> Result<PublishedGeneration, ModelError> {
    let id = attempt.generation();
    let sealed = match attempt.seal(receipt).await {
        Ok(v) => v,
        Err(error) => {
            let error = ModelError::from(error);
            if let Err(cleanup) = store.abort(id).await {
                return Err(cleanup_error(
                    id,
                    "seal",
                    &error,
                    &ModelError::from(cleanup),
                ));
            }
            return Err(error);
        }
    };
    let validated = match sealed.validate().await {
        Ok(v) => v,
        Err(error) => {
            let error = ModelError::from(error);
            if let Err(cleanup) = store.abort(id).await {
                return Err(cleanup_error(
                    id,
                    "validate",
                    &error,
                    &ModelError::from(cleanup),
                ));
            }
            return Err(error);
        }
    };
    let content = validated.content();
    let availability = validated
        .admission()
        .ok_or_else(|| ModelError::Invalid("validated facts have no admission".into()))?
        .availability()
        .clone();
    let generation = match validated.publish().await {
        Ok(id) => id,
        Err(error) => {
            let error = ModelError::from(error);
            if let Err(cleanup) = store.abort(id).await {
                return Err(cleanup_error(
                    id,
                    "publish",
                    &error,
                    &ModelError::from(cleanup),
                ));
            }
            return Err(error);
        }
    };
    Ok(PublishedGeneration {
        generation,
        content,
        availability,
        measurements,
    })
}
/// Full frontier memory control for bounded fixtures, using the same admission and content digest.
pub async fn memory(
    captured: Arc<CapturedInputs>,
    budget: ResourceBudget,
    profile: lctx_model::domain::stages::Profile,
    configuration: lctx_model::domain::ContentHash,
) -> Result<lctx_model::domain::admission::FrontierAdmission, ModelError> {
    let model = Arc::new(lctx_model::domain::model()?);
    let providers = providers::<lctx_model::domain::memory::MemoryGeneration>(configuration);
    let schedule = lctx_model::domain::stages::Schedule::build(
        &model,
        providers.iter().map(|p| p.declaration(profile)).collect(),
        &[],
        profile,
    )?;
    let preflight = lctx_model::domain::admission::FrontierContract::facts(&model, profile)?
        .preflight(&schedule)?;
    let mut execution = schedule.execute();
    let generation =
        lctx_model::domain::memory::MemoryGeneration::bind(&model, &budget, &mut execution)?;
    let receipt = compile_facts(
        execution,
        providers,
        &generation,
        &model,
        &captured,
        &budget,
    )
    .await?;
    generation.validate_facts(&model, &budget, preflight, &receipt)
}

pub(crate) fn cleanup_error(
    id: lctx_postgres::generations::GenerationId,
    phase: &str,
    primary: &ModelError,
    cleanup: &ModelError,
) -> ModelError {
    ModelError::infrastructure(
        lctx_model::domain::Infrastructure::Unconfirmed,
        format!(
            "generation {} {phase} failed: {primary}; cleanup unconfirmed: {cleanup}; inspect generation show before recovery",
            id.hex()
        ),
    )
}
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    #[test]
    fn cleanup_diagnostics_keep_generation_primary_and_secondary_causes() {
        let id = lctx_postgres::generations::GenerationId::from_hex(&"01".repeat(16)).unwrap();
        for primary in [
            ModelError::Invalid("required producer refused".into()),
            ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed,
                "publication commit acknowledgement lost",
            ),
        ] {
            let cleanup = ModelError::infrastructure(
                lctx_model::domain::Infrastructure::State,
                "abort refuses published generation",
            );
            let combined = cleanup_error(id, "publish", &primary, &cleanup);
            assert!(matches!(
                combined,
                ModelError::Infrastructure {
                    class: lctx_model::domain::Infrastructure::Unconfirmed,
                    ..
                }
            ));
            let detail = combined.to_string();
            assert!(
                detail.contains(&id.hex())
                    && detail.contains(&primary.to_string())
                    && detail.contains(&cleanup.to_string())
            );
        }
    }
}
