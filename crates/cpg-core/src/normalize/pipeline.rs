//! One captured input, one shared runtime, a private facts checkpoint, and one publication.
use super::*;
use crate::{
    facts,
    stage_runtime::{StageMeasurement, run_declared_stage_with_resources},
};
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use lctx_model::domain::{
    admission::{Frontier, FrontierContract},
    normalized::{
        binding_normalization, callable_normalization, entity_normalization, event_normalization,
        relation_normalization,
    },
    projection,
};
use lctx_postgres::generations::GenerationStore;
#[derive(Clone, Copy)]
enum Normalization {
    Entities,
    Relations,
    Callables,
    Events,
    Bindings,
    Projections,
    Coverage,
}
impl Normalization {
    const ALL: [Self; 7] = [
        Self::Entities,
        Self::Relations,
        Self::Callables,
        Self::Events,
        Self::Bindings,
        Self::Projections,
        Self::Coverage,
    ];
    fn declaration(self, profile: Profile) -> Stage {
        match self {
            Self::Entities => entity_normalization::stage(),
            Self::Relations => relation_normalization::stage(profile),
            Self::Callables => callable_normalization::stage(profile),
            Self::Events => event_normalization::stage(profile),
            Self::Bindings => binding_normalization::stage(profile),
            Self::Projections => projection::normalization::stage(profile),
            Self::Coverage => lctx_model::domain::normalized::coverage::stage(profile),
        }
    }
    async fn run(
        self,
        access: StageAccess<'_, '_>,
        attempt: &GenerationAttempt,
        config: &RoleConfig,
        runtime: &AttemptRuntime,
        model: &Arc<ValidatedModel>,
    ) -> Result<(), ModelError> {
        match self {
            Self::Entities => super::entities(access, attempt, config, runtime, model).await,
            Self::Relations => super::relations(access, attempt, config, runtime, model).await,
            Self::Callables => super::callables(access, attempt, config, runtime, model).await,
            Self::Events => super::events(access, attempt, config, runtime, model).await,
            Self::Bindings => super::bindings(access, attempt, config, runtime, model).await,
            Self::Projections => super::projections(access, attempt, config, runtime, model).await,
            Self::Coverage => super::coverage(access, attempt, config, runtime, model).await,
        }
    }
}
pub fn schedule(
    model: &ValidatedModel,
    providers: &[Box<dyn ProviderStage<GenerationAttempt>>],
    profile: Profile,
) -> Result<Schedule, ModelError> {
    let mut declarations: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
    declarations.extend(Normalization::ALL.map(|s| s.declaration(profile)));
    let schedule = Schedule::build(model, declarations, &[], profile)?;
    FrontierContract::for_frontier(model, profile, Frontier::Normalized)?.preflight(&schedule)?;
    FrontierContract::facts(model, profile)?.checkpoint_preflight(&schedule)?;
    Ok(schedule)
}
/// The runtime begins before acquisition and remains shared through native extraction, normalization,
/// source validation and publication. No intermediate generation is published or selected.
pub async fn publish(
    store: &GenerationStore,
    config: &RoleConfig,
    writer: sqlx::PgPool,
    captured: Arc<CapturedInputs>,
    runtime: &AttemptRuntime,
    profile: Profile,
    configuration: ContentHash,
) -> Result<facts::PublishedGeneration, ModelError> {
    bundle::refuse_ambient(std::env::vars_os())?;
    config.validate().map_err(ModelError::codec)?;
    if config.role != lctx_postgres::roles::Role::Importer {
        return Err(ModelError::Invalid(
            "normalized compilation requires importer credentials".into(),
        ));
    }
    config
        .check_capacity(&writer)
        .await
        .map_err(ModelError::codec)?;
    let model = Arc::new(lctx_model::domain::model()?);
    if model.digest() != store.model().digest() {
        return Err(ModelError::Invalid(
            "normalized store model differs from compiler".into(),
        ));
    }
    let mut providers = facts::providers::<GenerationAttempt>(configuration);
    let schedule = schedule(&model, &providers, profile)?;
    for input in captured.inputs() {
        input.captured().verify().map_err(ModelError::codec)?;
    }
    let contract = FrontierContract::for_frontier(&model, profile, Frontier::Normalized)?;
    let mut execution = schedule.execute();
    let attempt = store
        .begin(writer, &mut execution, &contract, runtime.budget().clone())
        .await
        .map_err(ModelError::from)?;
    let id = attempt.generation();
    let mut measurements = Vec::new();
    let mut observe = |measurement: StageMeasurement| {
        tracing::info!(
            stage = measurement.stage,
            elapsed_ms = measurement.elapsed_ms as u64,
            outcome = measurement.outcome,
            "normalized compilation stage"
        );
        measurements.push(measurement);
    };
    let work = async {
        // Complete all facts producers in their dependency order before freezing the lower layer.
        // Execution enforces actual relation dependencies; the checkpoint establishes the hard layer.
        for stage in schedule.stages() {
            let Some(position) = providers
                .iter()
                .position(|p| p.declaration(profile).name == stage.name)
            else {
                continue;
            };
            let provider = providers.swap_remove(position);
            let declaration = provider.declaration(profile);
            run_declared_stage_with_resources(
                &mut execution,
                &declaration,
                runtime.budget(),
                async |access| {
                    bundle::run_stage(
                        provider,
                        access,
                        &attempt,
                        &model,
                        &captured,
                        runtime.budget(),
                        Default::default(),
                    )
                    .await
                },
                &mut observe,
            )
            .await?;
        }
        if providers
            .iter()
            .any(|p| p.declaration(profile).profiles.contains(&profile))
        {
            return Err(ModelError::Invalid("unscheduled fact provider".into()));
        }
        drop(captured);
        attempt
            .checkpoint(&execution, &FrontierContract::facts(&model, profile)?)
            .await?;
        for stage in schedule.stages() {
            let Some(normalization) = Normalization::ALL
                .into_iter()
                .find(|n| n.declaration(profile).name == stage.name)
            else {
                continue;
            };
            let declaration = normalization.declaration(profile);
            run_declared_stage_with_resources(
                &mut execution,
                &declaration,
                runtime.budget(),
                async |access| {
                    normalization
                        .run(access, &attempt, config, runtime, &model)
                        .await
                },
                &mut observe,
            )
            .await?;
        }
        execution.finish()
    }
    .await;
    let receipt = match work {
        Ok(receipt) => receipt,
        Err(error) => {
            if let Err(cleanup) = attempt.abort().await {
                return Err(facts::cleanup_error(
                    id,
                    "normalized compilation",
                    &error,
                    &ModelError::from(cleanup),
                ));
            }
            return Err(error);
        }
    };
    facts::finish_publication(store, attempt, receipt, measurements).await
}
