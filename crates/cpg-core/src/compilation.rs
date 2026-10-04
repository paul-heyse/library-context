//! One cumulative attempt through analysis/catalog. Semantic owners supply declarations and proofs.
use crate::{
    analysis_graphs::PreparedGraphs,
    embedding_service::Embedder,
    facts,
    model_runtime::AttemptRuntime,
    normalize::pipeline::Normalization,
    stage_runtime::{StageMeasurement, run_declared_stage_with_resources},
};
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use lctx_model::domain::{
    admission::{Frontier, FrontierContract},
    analysis::{
        AnalysisDefinition, AnalysisMethod, preparation::Configuration,
        settings::AnalyticsConfiguration,
    },
    stages::*,
    *,
};
use lctx_postgres::{
    generations::{GenerationAttempt, GenerationStore},
    roles::{Role, RoleConfig},
};
use std::sync::Arc;

/// Closed executable upper routes. Metadata and runner dispatch use the same finite type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpperStage {
    Configuration,
    Native,
    EmbeddingConfiguration,
    Text,
    Embedding,
    CatalogCore,
    CatalogEvidence,
    Local,
    Base,
    Completion,
    SourceCalls,
    Enriched,
    Models,
    Summary,
    Structural,
    Analytic,
    Selection,
    Synthesis,
    Retrieval,
    AnalysisFrontier,
    CatalogFrontier,
}
impl UpperStage {
    const ALL: [Self; 21] = [
        Self::Configuration,
        Self::Native,
        Self::EmbeddingConfiguration,
        Self::Text,
        Self::Embedding,
        Self::CatalogCore,
        Self::CatalogEvidence,
        Self::Local,
        Self::Base,
        Self::Completion,
        Self::SourceCalls,
        Self::Enriched,
        Self::Models,
        Self::Summary,
        Self::Structural,
        Self::Analytic,
        Self::Selection,
        Self::Synthesis,
        Self::Retrieval,
        Self::AnalysisFrontier,
        Self::CatalogFrontier,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Configuration => "analysis_configuration",
            Self::Native => "analysis_native_inventory",
            Self::EmbeddingConfiguration => "embedding_configuration",
            Self::Text => "analytic_text",
            Self::Embedding => "analytic_embedding",
            Self::CatalogCore => "catalog_core",
            Self::CatalogEvidence => "catalog_evidence",
            Self::Local => "analyze_local",
            Self::Base => "evaluate_base",
            Self::Completion => "complete_base",
            Self::SourceCalls => "prepare_source_calls",
            Self::Enriched => "enrich_execution",
            Self::Models => "apply_models",
            Self::Summary => "analyze_summaries",
            Self::Structural => "analyze_structural",
            Self::Analytic => "analyze_analytic",
            Self::Selection => "catalog_selection",
            Self::Synthesis => "synthesis",
            Self::Retrieval => "retrieval",
            Self::AnalysisFrontier => "assess_analysis_frontier",
            Self::CatalogFrontier => "assess_catalog_frontier",
        }
    }
    fn phase(self) -> Frontier {
        match self {
            Self::Configuration => Frontier::Analysis,
            Self::Native => Frontier::Analysis,
            Self::EmbeddingConfiguration => Frontier::Analysis,
            Self::Text => Frontier::Analysis,
            Self::Embedding => Frontier::Analysis,
            Self::CatalogCore => Frontier::Analysis,
            Self::CatalogEvidence => Frontier::Analysis,
            Self::Local => Frontier::Analysis,
            Self::Base => Frontier::Analysis,
            Self::Completion => Frontier::Analysis,
            Self::SourceCalls => Frontier::Analysis,
            Self::Enriched => Frontier::Analysis,
            Self::Models => Frontier::Analysis,
            Self::Summary => Frontier::Analysis,
            Self::Structural => Frontier::Analysis,
            Self::Analytic => Frontier::Analysis,
            Self::Selection => Frontier::Catalog,
            Self::Synthesis => Frontier::Catalog,
            Self::Retrieval => Frontier::Catalog,
            Self::AnalysisFrontier => Frontier::Analysis,
            Self::CatalogFrontier => Frontier::Catalog,
        }
    }
    fn boundary(self) -> Option<PublicationBoundary> {
        match self {
            Self::Configuration => None,
            Self::Native => None,
            Self::EmbeddingConfiguration => None,
            Self::Text => None,
            Self::Embedding => None,
            Self::CatalogCore => None,
            Self::CatalogEvidence => None,
            Self::Local => Some(PublicationBoundary::Local),
            Self::Base => None,
            Self::Completion => None,
            Self::SourceCalls => None,
            Self::Enriched => None,
            Self::Models => Some(PublicationBoundary::Model),
            Self::Summary => Some(PublicationBoundary::Summary),
            Self::Structural => Some(PublicationBoundary::Structural),
            Self::Analytic => Some(PublicationBoundary::Analytic),
            Self::Selection => None,
            Self::Synthesis => Some(PublicationBoundary::Synthesis),
            Self::Retrieval => None,
            Self::AnalysisFrontier => None,
            Self::CatalogFrontier => None,
        }
    }
    fn graphs(self) -> &'static [projection::ProjectionName] {
        match self {
            Self::Configuration => &[],
            Self::Native => &[],
            Self::EmbeddingConfiguration => &[],
            Self::Text => &[],
            Self::Embedding => &[],
            Self::CatalogCore => &[],
            Self::CatalogEvidence => &[],
            Self::Local => &[],
            Self::Base => &[],
            Self::Completion => &[],
            Self::SourceCalls => &[],
            Self::Enriched => &[],
            Self::Models => &[],
            Self::Summary => &[projection::ProjectionName::CallableInvocation],
            Self::Structural => &[
                projection::ProjectionName::CallableInvocation,
                projection::ProjectionName::DefinitionContainment,
            ],
            Self::Analytic => &[projection::ProjectionName::CallableInvocation],
            Self::Selection => &[],
            Self::Synthesis => &[],
            Self::Retrieval => &[],
            Self::AnalysisFrontier => &[],
            Self::CatalogFrontier => &[],
        }
    }
    fn resolve(name: &str) -> Result<Self, ModelError> {
        let mut found = Self::ALL.into_iter().filter(|s| s.name() == name);
        let stage = found
            .next()
            .ok_or_else(|| ModelError::Invalid(format!("upper stage has no binding: {name}")))?;
        if found.next().is_some() {
            return Err(ModelError::Invalid("duplicate upper binding".into()));
        }
        Ok(stage)
    }
}

pub struct PreparedCompilation {
    frontier: Frontier,
    configuration: Configuration,
    embedding: Option<embedding::configuration::Configuration>,
    text: embedding::text::TextDefinition,
}
impl PreparedCompilation {
    pub fn new(
        frontier: Frontier,
        settings: AnalyticsConfiguration,
        catalog: &models::Catalog,
        embedder: Option<&dyn Embedder>,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        if !matches!(frontier, Frontier::Analysis | Frontier::Catalog) {
            return Err(ModelError::Invalid(
                "upper compilation requires analysis or catalog".into(),
            ));
        }
        settings.validate()?;
        let mut definitions = vec![
            local_semantics::definition(),
            execution::configuration::base_evaluation(),
            execution::configuration::base_completion(),
            execution::configuration::source_calls(),
            execution::configuration::enriched_execution(catalog.declaration().id()),
            execution::configuration::models(catalog.declaration().id()),
            execution::configuration::summaries(
                catalog.declaration().id(),
                execution::configuration::SummaryLimits::default(),
            )?,
            catalog::build::definition(),
            catalog::evidence::build::definition(),
            embedding::analytic::definition(),
        ];
        for method in structural::build::methods() {
            definitions.push(structural::build::definition(&settings, method)?);
        }
        for method in analytics::build::METHODS {
            definitions.push(analytics::build::definition(&settings, method)?);
        }
        if frontier == Frontier::Catalog {
            definitions.extend([
                selection::build::definition(),
                synthesis::build::definition(),
                retrieval::build::definition(),
            ]);
        }
        let mut configuration =
            Configuration::new(catalog, definitions, budget)?.with_analytics(settings.clone())?;
        if frontier == Frontier::Catalog {
            configuration = configuration
                .with_retrieval(retrieval::RetrievalDefinition::builtin(embedder.is_some()))?;
        }
        let embedding = embedder
            .map(|e| embedding::configuration::Configuration::new(e.spec(), e.endpoint(), budget))
            .transpose()?;
        let text = embedding::text::TextDefinition {
            requested: embedder.is_some() && (settings.knn || settings.knn_layer),
            ..embedding::text::TextDefinition::builtin()
        };
        Ok(Self {
            frontier,
            configuration,
            embedding,
            text,
        })
    }
    /// The admitted configuration also supplies fixtures with the exact production declarations.
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }
    fn definition(&self, method: AnalysisMethod) -> Result<&AnalysisDefinition, ModelError> {
        self.configuration
            .definitions()
            .iter()
            .find(|d| d.method == method)
            .ok_or_else(|| ModelError::Invalid("required compilation definition missing".into()))
    }
    fn settings(&self) -> &AnalyticsConfiguration {
        self.configuration
            .analytics()
            .iter()
            .next()
            .expect("admitted configuration")
    }
    fn upper_declaration(
        &self,
        binding: UpperStage,
        profile: Profile,
        model: &ValidatedModel,
        order: &PublicationOrder,
    ) -> Result<Stage, ModelError> {
        Ok(match binding {
            UpperStage::Configuration => self.configuration.declaration(),
            UpperStage::Native => analysis::preparation::native_stage(profile),
            UpperStage::EmbeddingConfiguration => {
                embedding::configuration::stage(self.embedding.as_ref())
            }
            UpperStage::Text => embedding::text::stage(profile, &self.text)?,
            UpperStage::Embedding => embedding::analytic::stage(profile, self.text.requested),
            UpperStage::CatalogCore => catalog::build::stage(profile),
            UpperStage::CatalogEvidence => catalog::evidence::build::stage(profile, model, order)?,
            UpperStage::Local => local_semantics::stage(
                profile,
                self.definition(AnalysisMethod::LocalTransfers)?,
                model,
            ),
            UpperStage::Base => execution::production::stage(
                profile,
                self.definition(AnalysisMethod::Execution)?,
                model,
            )?,
            UpperStage::Completion => execution::completion_production::stage(
                profile,
                self.definition(AnalysisMethod::Completion)?,
                model,
            )?,
            UpperStage::SourceCalls => execution::source_call::stage(
                profile,
                self.definition(AnalysisMethod::SourceCalls)?,
                model,
            )?,
            UpperStage::Enriched => execution::enriched_production::stage(
                profile,
                self.definition(AnalysisMethod::EnrichedExecution)?,
                model,
            )?,
            UpperStage::Models => execution::model_production::stage(
                profile,
                self.definition(AnalysisMethod::Models)?,
                model,
            )?,
            UpperStage::Summary => execution::summary_replay::stage(
                profile,
                self.definition(AnalysisMethod::Summaries)?,
                model,
            )?,
            UpperStage::Structural => structural::build::stage(profile, self.settings(), model)?,
            UpperStage::Analytic => {
                analytics::build::stage(profile, self.settings(), model, order)?
            }
            UpperStage::Selection => selection::build::stage(profile, model, order)?,
            UpperStage::Synthesis => {
                synthesis::build::stage(profile, self.settings(), model, order)?
            }
            UpperStage::Retrieval => retrieval::build::stage(
                profile,
                self.configuration
                    .retrieval()
                    .iter()
                    .next()
                    .expect("catalog retrieval definition"),
                model,
            )?,
            UpperStage::AnalysisFrontier => {
                analysis::frontier::stage(profile, analysis::frontier::Target::Analysis, model)?
            }
            UpperStage::CatalogFrontier => {
                analysis::frontier::stage(profile, analysis::frontier::Target::Catalog, model)?
            }
        })
    }
    pub fn schedule(
        &self,
        model: &ValidatedModel,
        providers: &[Box<dyn ProviderStage<GenerationAttempt>>],
        profile: Profile,
    ) -> Result<Schedule, ModelError> {
        let mut declarations: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
        declarations.extend(Normalization::ALL.map(|s| s.declaration(profile)));
        let bindings: Vec<_> = UpperStage::ALL
            .into_iter()
            .filter(|s| s.phase() == Frontier::Analysis || self.frontier == Frontier::Catalog)
            .collect();
        let facts = declarations
            .iter()
            .filter(|s| {
                providers
                    .iter()
                    .any(|p| p.declaration(profile).name == s.name)
                    && s.profiles.contains(&profile)
                    && s.outputs.iter().any(|r| is_vocabulary(r.name()))
            })
            .map(|s| s.name)
            .collect();
        let mut groups = vec![PublicationGroup::new(PublicationBoundary::Facts, facts)];
        for binding in &bindings {
            if let Some(boundary) = binding.boundary() {
                groups.push(PublicationGroup::new(boundary, vec![binding.name()]));
            }
        }
        let order = PublicationOrder::planning(&groups)?;
        for binding in &bindings {
            let declaration = self.upper_declaration(*binding, profile, model, &order)?;
            if declaration.name != binding.name() {
                return Err(ModelError::Invalid(
                    "upper declaration differs from binding".into(),
                ));
            }
            declarations.push(declaration);
        }
        let schedule =
            Schedule::build_with_publications(model, declarations, &[], profile, groups)?;
        validate_upper_dependencies(&schedule)?;
        FrontierContract::for_frontier(model, profile, self.frontier)?.preflight(&schedule)?;
        for checkpoint in self.frontier.descriptor().checkpoints() {
            FrontierContract::for_frontier(model, profile, *checkpoint)?
                .checkpoint_preflight(&schedule)?;
        }
        Ok(schedule)
    }
}

/// Reject a Catalog-only producer for an Analysis binding before any attempt is opened.
fn validate_upper_dependencies(schedule: &Schedule) -> Result<(), ModelError> {
    for stage in schedule.stages() {
        if !UpperStage::resolve(stage.name).is_ok_and(|s| s.phase() == Frontier::Analysis) {
            continue;
        }
        for input in &stage.inputs {
            let limit = input.prefix().map(|e| schedule.prefix_for(e)).transpose()?;
            for producer in schedule.stages() {
                if !UpperStage::resolve(producer.name).is_ok_and(|s| s.phase() == Frontier::Catalog)
                    || !producer.outputs.iter().any(|o| o.name() == input.name())
                {
                    continue;
                }
                if let Some(limit) = limit {
                    let boundary = schedule.epoch_for(producer.name).ok_or_else(|| {
                        ModelError::Invalid(
                            "vocabulary producer has no publication boundary".into(),
                        )
                    })?;
                    if schedule.prefix_for(boundary)?.ordinal() > limit.ordinal() {
                        continue;
                    }
                }
                return Err(ModelError::Invalid(
                    "analysis route consumes catalog-only output".into(),
                ));
            }
        }
    }
    Ok(())
}

/// A single attempt owns every collection and lease. Lower checkpoints never publish or select it.
#[expect(
    clippy::too_many_arguments,
    reason = "Publication receives explicit independent store, native-input, runtime, profile, prepared configuration and embedding effect capabilities"
)]
pub async fn publish(
    store: &GenerationStore,
    roles: &RoleConfig,
    writer: sqlx::PgPool,
    captured: Arc<CapturedInputs>,
    runtime: &AttemptRuntime,
    profile: Profile,
    configuration: ContentHash,
    prepared: &PreparedCompilation,
    embedder: Option<&dyn Embedder>,
    cache: Option<lctx_postgres::Store>,
) -> Result<facts::PublishedGeneration, ModelError> {
    bundle::refuse_ambient(std::env::vars_os())?;
    captured.config().check_profile(profile)?;
    captured.config().check_budget(runtime.budget())?;
    prepared.configuration.check_budget(runtime.budget())?;
    if let Some(selected) = &prepared.embedding {
        selected.check_budget(runtime.budget())?;
    }
    roles.validate().map_err(ModelError::codec)?;
    if roles.role != Role::Importer {
        return Err(ModelError::Invalid(
            "upper compilation requires importer credentials".into(),
        ));
    }
    roles
        .check_capacity(&writer)
        .await
        .map_err(ModelError::codec)?;
    let model = Arc::new(lctx_model::domain::model()?);
    if model.digest() != store.model().digest() {
        return Err(ModelError::Invalid(
            "compilation and store model differ".into(),
        ));
    }
    let mut providers = facts::providers::<GenerationAttempt>(configuration);
    let schedule = prepared.schedule(&model, &providers, profile)?;
    let fact_names: std::collections::BTreeSet<_> = providers
        .iter()
        .map(|p| p.declaration(profile).name)
        .collect();
    for input in captured.inputs() {
        input.captured().verify().map_err(ModelError::codec)?;
    }
    let contract = FrontierContract::for_frontier(&model, profile, prepared.frontier)?;
    let mut execution = schedule.execute();
    let attempt = store
        .begin(writer, &mut execution, &contract, runtime.budget().clone())
        .await
        .map_err(ModelError::from)?;
    let id = attempt.generation();
    let mut measurements = vec![];
    let mut observe = |measurement: StageMeasurement| {
        tracing::info!(
            stage = measurement.stage,
            elapsed_ms = measurement.elapsed_ms as u64,
            outcome = measurement.outcome,
            "cumulative compilation stage"
        );
        measurements.push(measurement);
    };
    let work = async {
        for stage in schedule.stages() {
            let Some(index) = providers
                .iter()
                .position(|p| p.declaration(profile).name == stage.name)
            else {
                continue;
            };
            let provider = providers.swap_remove(index);
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
            return Err(ModelError::Invalid("unscheduled facts provider".into()));
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
            run_declared_stage_with_resources(
                &mut execution,
                stage,
                runtime.budget(),
                async |access| {
                    normalization
                        .run(access, &attempt, roles, runtime, &model)
                        .await
                },
                &mut observe,
            )
            .await?;
        }
        attempt
            .checkpoint(
                &execution,
                &FrontierContract::for_frontier(&model, profile, Frontier::Normalized)?,
            )
            .await?;
        let graph_needs = schedule
            .stages()
            .iter()
            .filter_map(|s| UpperStage::resolve(s.name).ok())
            .flat_map(|b| b.graphs().iter().copied())
            .collect();
        let mut graphs = None;
        for catalog_phase in [false, true] {
            if catalog_phase {
                if prepared.frontier != Frontier::Catalog {
                    break;
                }
                attempt
                    .checkpoint(
                        &execution,
                        &FrontierContract::for_frontier(&model, profile, Frontier::Analysis)?,
                    )
                    .await?;
            }
            for stage in schedule.stages() {
                if Normalization::ALL
                    .into_iter()
                    .any(|n| n.declaration(profile).name == stage.name)
                    || fact_names.contains(stage.name)
                {
                    continue;
                }
                let binding = UpperStage::resolve(stage.name)?;
                if catalog_phase != (binding.phase() == Frontier::Catalog) {
                    continue;
                }
                run_declared_stage_with_resources(
                    &mut execution,
                    stage,
                    runtime.budget(),
                    async |access| {
                        if !binding.graphs().is_empty() && graphs.is_none() {
                            graphs = Some(
                                PreparedGraphs::load(
                                    &access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    &graph_needs,
                                )
                                .await?,
                            );
                        }
                        match binding {
                            UpperStage::Configuration => {
                                crate::analysis_prepare::configuration(
                                    access,
                                    &attempt,
                                    &model,
                                    runtime,
                                    &prepared.configuration,
                                )
                                .await
                            }
                            UpperStage::Native => {
                                crate::analysis_prepare::native_inventory(
                                    access, &attempt, roles, runtime, &model,
                                )
                                .await
                            }
                            UpperStage::EmbeddingConfiguration => {
                                crate::analysis_prepare::embedding_configuration(
                                    access,
                                    &attempt,
                                    &model,
                                    runtime,
                                    prepared.embedding.as_ref(),
                                )
                                .await
                            }
                            UpperStage::Text => {
                                crate::analytic_text::publish(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.text.clone(),
                                )
                                .await
                            }
                            UpperStage::Embedding => {
                                crate::analytic_embedding::produce(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    embedder,
                                    cache.clone(),
                                )
                                .await
                            }
                            UpperStage::CatalogCore => {
                                crate::catalog_core::produce(
                                    access, &attempt, roles, runtime, &model,
                                )
                                .await
                            }
                            UpperStage::CatalogEvidence => {
                                crate::catalog_evidence::produce(
                                    access, &attempt, roles, runtime, &model,
                                )
                                .await
                            }
                            UpperStage::Selection => {
                                crate::catalog_selection::produce(
                                    access, &attempt, roles, runtime, &model,
                                )
                                .await
                            }
                            UpperStage::Local => {
                                crate::local_semantics::run(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.definition(AnalysisMethod::LocalTransfers)?,
                                )
                                .await
                            }
                            UpperStage::Base => {
                                crate::semantic_execution::evaluate_base(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.definition(AnalysisMethod::Execution)?,
                                )
                                .await
                            }
                            UpperStage::Completion => {
                                crate::semantic_execution::complete_base(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.definition(AnalysisMethod::Completion)?,
                                )
                                .await
                            }
                            UpperStage::SourceCalls => {
                                crate::semantic_execution::prepare_source_calls(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.definition(AnalysisMethod::SourceCalls)?,
                                )
                                .await
                            }
                            UpperStage::Enriched => {
                                crate::semantic_execution::enrich(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.definition(AnalysisMethod::EnrichedExecution)?,
                                )
                                .await
                            }
                            UpperStage::Models => {
                                crate::semantic_models::apply(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.definition(AnalysisMethod::Models)?,
                                )
                                .await
                            }
                            UpperStage::Summary => {
                                crate::semantic_summaries::produce(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    prepared.definition(AnalysisMethod::Summaries)?,
                                    graphs.as_ref().expect("loaded above"),
                                )
                                .await
                            }
                            UpperStage::Structural => {
                                crate::structural::produce(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    graphs.as_ref().expect("loaded above"),
                                )
                                .await
                            }
                            UpperStage::Analytic => {
                                crate::analytic::produce(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    graphs.as_ref().expect("loaded above"),
                                )
                                .await
                            }
                            UpperStage::AnalysisFrontier => {
                                crate::final_coverage::produce(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    analysis::frontier::Target::Analysis,
                                )
                                .await
                            }
                            UpperStage::CatalogFrontier => {
                                crate::final_coverage::produce(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    analysis::frontier::Target::Catalog,
                                )
                                .await
                            }
                            UpperStage::Synthesis => {
                                crate::synthesis::produce(access, &attempt, roles, runtime, &model)
                                    .await
                            }
                            UpperStage::Retrieval => {
                                crate::retrieval::produce(
                                    access,
                                    &attempt,
                                    roles,
                                    runtime,
                                    &model,
                                    embedder,
                                    cache.clone(),
                                )
                                .await
                            }
                        }
                    },
                    &mut observe,
                )
                .await?;
            }
        }
        drop(graphs);
        execution.finish()
    }
    .await;
    let receipt = match work {
        Ok(receipt) => receipt,
        Err(error) => {
            if let Err(cleanup) = attempt.abort().await {
                return Err(facts::cleanup_error(
                    id,
                    "cumulative compilation",
                    &error,
                    &ModelError::from(cleanup),
                ));
            }
            return Err(error);
        }
    };
    facts::finish_publication(store, attempt, receipt, measurements).await
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    use lctx_model::domain::input::{Package, Release};
    #[test]
    fn missing_route_and_analysis_dependency_on_catalog_refuse_before_effects() {
        assert!(UpperStage::resolve("unknown_upper_route").is_err());
        let names: std::collections::BTreeSet<_> =
            UpperStage::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(names.len(), UpperStage::ALL.len());
        let model =
            ValidatedModel::validate(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap();
        let stage = |name, inputs, outputs| Stage {
            name,
            inputs,
            outputs,
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog],
            effect: lctx_model::domain::stages::Effect::Pure,
            code: ContentHash::of(b"phase control"),
            configuration: ContentHash::of(b"fixture"),
        };
        let schedule = Schedule::build(
            &model,
            vec![
                stage("retrieval", vec![], vec![RelationUse::of::<Package>()]),
                stage(
                    "analyze_local",
                    vec![RelationUse::stored::<Package>()],
                    vec![RelationUse::of::<Release>()],
                ),
            ],
            &[],
            Profile::Catalog,
        )
        .unwrap();
        assert!(validate_upper_dependencies(&schedule).is_err());
        let schedule = Schedule::build(
            &model,
            vec![
                stage("catalog_core", vec![], vec![RelationUse::of::<Package>()]),
                stage(
                    "analyze_local",
                    vec![RelationUse::stored::<Package>()],
                    vec![RelationUse::of::<Release>()],
                ),
            ],
            &[],
            Profile::Catalog,
        )
        .unwrap();
        validate_upper_dependencies(&schedule).unwrap();
    }
    #[test]
    fn earlier_vocabulary_prefix_does_not_depend_on_later_catalog_extension() {
        use lctx_model::domain::value::Literal;
        let model =
            ValidatedModel::validate(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
                .unwrap();
        let stage = |name, inputs, outputs| Stage {
            name,
            inputs,
            outputs,
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog],
            effect: lctx_model::domain::stages::Effect::Pure,
            code: ContentHash::of(b"epoch phase control"),
            configuration: ContentHash::of(b"fixture"),
        };
        let schedule = |epoch| {
            Schedule::build_with_publications(
                &model,
                vec![
                    stage("facts", vec![], vec![RelationUse::of::<Literal>()]),
                    stage(
                        "analyze_local",
                        vec![RelationUse::stored::<Literal>().at_epoch(epoch)],
                        vec![RelationUse::of::<Package>()],
                    ),
                    stage("synthesis", vec![], vec![RelationUse::of::<Literal>()]),
                ],
                &[],
                Profile::Catalog,
                vec![
                    PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
                    PublicationGroup::new(PublicationBoundary::Synthesis, vec!["synthesis"]),
                ],
            )
            .unwrap()
        };
        validate_upper_dependencies(&schedule(PublicationBoundary::Facts)).unwrap();
        assert!(
            validate_upper_dependencies(&schedule(PublicationBoundary::Synthesis)).is_err(),
            "a visible Catalog vocabulary contributor must refuse an Analysis route"
        );
    }
}
