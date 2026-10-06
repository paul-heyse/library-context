//! One cumulative attempt through analysis/catalog. Semantic owners supply declarations and proofs.
use crate::{
    analysis_graphs::PreparedGraphs,
    embedding_service::Embedder,
    facts,
    normalize::pipeline::Normalization,
    workspace::{ProducerOutput, Workspace},
};
use cpg_extract::bundle::{CapturedInputs, ProviderStage};
use lctx_model::domain::{
    admission::Frontier,
    analysis::{
        AnalysisDefinition, AnalysisMethod, preparation::Configuration,
        settings::AnalyticsConfiguration,
    },
    stages::*,
    *,
};
use std::sync::Arc;
/// Closed executable upper routes. Metadata and runner dispatch use the same finite type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpperStage {
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
    pub(crate) fn name(self) -> &'static str {
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
    pub(crate) fn for_outcome_relation(name: &str) -> Option<Self> {
        macro_rules! owners { ($($owner:ident => $variant:ident),* $(,)?) => {
            match name { $(analysis::$owner::AnalysisInvocation::NAME => Some(Self::$variant),)* _ => None }
        }; }
        owners!(local => Local, base_evaluation => Base, base_completion => Completion,
            source_call => SourceCalls, enriched_execution => Enriched, model => Models,
            summary => Summary, structural => Structural, analytic_embedding => Embedding,
            analytic => Analytic, catalog_core => CatalogCore, catalog_evidence => CatalogEvidence,
            selection => Selection, synthesis => Synthesis, retrieval => Retrieval)
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
    fn graphs(self, profile: Profile) -> &'static [projection::ProjectionName] {
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
            Self::Summary if profile == Profile::Behavioral => &[projection::ProjectionName::CallableInvocation],
            Self::Summary => &[],
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
            UpperStage::Native => analysis::preparation::native_stage(profile, model, order)?,
            UpperStage::EmbeddingConfiguration => {
                embedding::configuration::stage(self.embedding.as_ref())
            }
            UpperStage::Text => embedding::text::stage(profile, &self.text, model, order)?,
            UpperStage::Embedding => {
                embedding::analytic::stage(profile, self.text.requested, model, order)?
            }
            UpperStage::CatalogCore => catalog::build::stage(profile, model, order)?,
            UpperStage::CatalogEvidence => catalog::evidence::build::stage(profile, model, order)?,
            UpperStage::Local => local_semantics::stage(
                profile,
                self.definition(AnalysisMethod::LocalTransfers)?,
                model,
                order,
            )?,
            UpperStage::Base => execution::production::stage(
                profile,
                self.definition(AnalysisMethod::Execution)?,
                model,
                order,
            )?,
            UpperStage::Completion => execution::completion_production::stage(
                profile,
                self.definition(AnalysisMethod::Completion)?,
                model,
                order,
            )?,
            UpperStage::SourceCalls => execution::source_call::stage(
                profile,
                self.definition(AnalysisMethod::SourceCalls)?,
                model,
                order,
            )?,
            UpperStage::Enriched => execution::enriched_production::stage(
                profile,
                self.definition(AnalysisMethod::EnrichedExecution)?,
                model,
                order,
            )?,
            UpperStage::Models => execution::model_production::stage(
                profile,
                self.definition(AnalysisMethod::Models)?,
                model,
                order,
            )?,
            UpperStage::Summary => execution::summary_replay::stage(
                profile,
                self.definition(AnalysisMethod::Summaries)?,
                model,
                order,
            )?,
            UpperStage::Structural => {
                structural::build::stage(profile, self.settings(), model, order)?
            }
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
                order,
            )?,
            UpperStage::AnalysisFrontier => analysis::frontier::stage(
                profile,
                analysis::frontier::Target::Analysis,
                model,
                order,
            )?,
            UpperStage::CatalogFrontier => analysis::frontier::stage(
                profile,
                analysis::frontier::Target::Catalog,
                model,
                order,
            )?,
        })
    }
    pub fn schedule(
        &self,
        model: &ValidatedModel,
        providers: &[Box<dyn ProviderStage<ProducerOutput>>],
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

/// Resolve the declaration's immutable predecessor view using static semantic dependencies.
/// An unspecified shared-vocabulary input binds its first producer group, matching the model
/// scheduler contract. Ordinary relations retain their completed-stream selection.
fn completed_input_declaration(schedule: &Schedule, declaration: &Stage) -> Stage {
    let mut selected = declaration.clone();
    for input in &mut selected.inputs {
        if input.prefix().is_some() || !is_vocabulary(input.name()) {
            continue;
        }
        if let Some(group) = schedule.publication_groups().iter().find(|group| {
            schedule.stages().iter().any(|producer| {
                group.stages.contains(&producer.name)
                    && producer
                        .outputs
                        .iter()
                        .any(|output| output.name() == input.name())
            })
        }) {
            *input = input.at_epoch(group.epoch);
        }
    }
    selected
}
/// Freeze only completed vocabulary descriptors, once all statically named producers for a
/// boundary finish. No stage grant, receipt, mutable publication epoch or store is involved.
fn freeze_completed_inputs(
    workspace: &Workspace,
    schedule: &Schedule,
    completed: &std::collections::BTreeSet<&'static str>,
    frozen: &mut std::collections::BTreeSet<PublicationBoundary>,
) -> Result<(), ModelError> {
    for group in schedule.publication_groups() {
        if frozen.contains(&group.epoch) {
            continue;
        }
        if !group.stages.iter().all(|name| completed.contains(name)) {
            break;
        }
        workspace.freeze_inputs(group.epoch)?;
        frozen.insert(group.epoch);
    }
    Ok(())
}

/// Execute all selected compiler owners into completed local streams. Publication is a separate
/// consumer of the admitted graph artifact and is intentionally absent from this API.
#[allow(
    clippy::too_many_arguments,
    reason = "The compiler entry point keeps capture, profile, frontier, prepared configuration and optional embedding dependencies explicit"
)]
pub async fn compile(
    workspace: &Arc<Workspace>,
    captured: Arc<CapturedInputs>,
    profile: Profile,
    configuration: ContentHash,
    frontier: Frontier,
    prepared: Option<&PreparedCompilation>,
    embedder: Option<&dyn Embedder>,
    cache: Option<Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>>,
) -> Result<(), ModelError> {
    let model = workspace.model();
    let capture_identity = workspace.captures(&captured)?;
    let providers = facts::providers(configuration);
    // Plan every declaration before running native effects. The schedule is static dependency
    // metadata only; completed streams, not execution grants, supply runtime inputs.
    let schedule = if let Some(prepared) = prepared {
        if prepared.frontier != frontier {
            return Err(ModelError::Invalid("prepared frontier differs".into()));
        }
        prepared.configuration.check_budget(workspace.budget())?;
        prepared.schedule(model, &providers, profile)?
    } else {
        if matches!(frontier, Frontier::Analysis | Frontier::Catalog) {
            return Err(ModelError::Invalid(
                "upper compilation needs prepared configuration".into(),
            ));
        }
        let mut declarations: Vec<_> = providers.iter().map(|p| p.declaration(profile)).collect();
        if frontier == Frontier::Normalized {
            declarations.extend(Normalization::ALL.map(|s| s.declaration(profile)));
        }
        Schedule::build(model, declarations, &[], profile)?
    };
    let fact_names: std::collections::BTreeSet<_> = providers
        .iter()
        .map(|p| p.declaration(profile).name)
        .collect();
    facts::compile_facts(workspace, &captured, profile, providers, Default::default()).await?;
    drop(captured);
    workspace.facts_availability(profile)?;
    let mut completed = schedule
        .stages()
        .iter()
        .filter(|stage| fact_names.contains(stage.name))
        .map(|stage| stage.name)
        .collect();
    let mut frozen = Default::default();
    freeze_completed_inputs(workspace, &schedule, &completed, &mut frozen)?;
    let mut binding_application = None;
    let mut receiver_authority = None;
    let mut event_authority = None;
    for declaration in schedule.stages() {
        let Some(normalization) = Normalization::ALL
            .into_iter()
            .find(|n| n.declaration(profile).name == declaration.name)
        else {
            continue;
        };
        let selected = completed_input_declaration(&schedule, declaration);
        let access = workspace.stage_inputs_selected(declaration, &selected, profile)?;
        let output = workspace.producer(declaration, profile, access.clone());
        match normalization {
            Normalization::Receivers => receiver_authority = Some(crate::normalize::receivers_produced(access, output, workspace, model).await?),
            Normalization::Events => event_authority = Some(crate::normalize::events_produced(access, output, workspace, model,
                receiver_authority.as_ref().ok_or_else(|| ModelError::Invalid("receiver owner authority absent".into()))?).await?),
            Normalization::Bindings => binding_application = Some(crate::normalize::bindings_prepared(access, output, workspace, model,
                receiver_authority.as_ref().ok_or_else(|| ModelError::Invalid("receiver owner authority absent".into()))?,
                event_authority.as_ref().ok_or_else(|| ModelError::Invalid("event owner authority absent".into()))?).await?),
            _ => normalization.run(access, output, workspace, model).await?,
        }
        completed.insert(declaration.name);
        freeze_completed_inputs(workspace, &schedule, &completed, &mut frozen)?;
    }
    drop(event_authority);
    drop(receiver_authority);
    // Source owners are admitted once while the normalized dependency set is immutable.
    // Prepared consumer authorities project this lifetime; upper stages do not replay owners.
    let normalized_authority = if prepared.is_some() {
        Some(workspace.admit_semantics(profile).await?)
    } else { None };
    let bindings = match (binding_application, normalized_authority.as_ref()) {
        (Some(application), Some(authority)) if profile == Profile::Behavioral => Some(crate::analysis_bindings::PreparedBindings::new(application, authority)?),
        _ => None,
    };
    let graph_needs = schedule
        .stages()
        .iter()
        .filter_map(|s| UpperStage::resolve(s.name).ok())
        .flat_map(|s| s.graphs(profile).iter().copied())
        .collect();
    let mut graphs = None;
    let mut local = None;
    let mut evaluations = None;
    let mut completed_bodies = None;
    let mut source_calls = None;
    for declaration in schedule.stages() {
        if fact_names.contains(declaration.name) {
            continue;
        }
        if Normalization::ALL
            .into_iter()
            .any(|n| n.declaration(profile).name == declaration.name)
        {
            continue;
        }
        let selected = completed_input_declaration(&schedule, declaration);
        let access = workspace.stage_inputs_selected(declaration, &selected, profile)?;
        let output = workspace.producer(declaration, profile, access.clone());
        let binding = UpperStage::resolve(declaration.name)?;
        let prepared =
            prepared.ok_or_else(|| ModelError::Invalid("missing upper configuration".into()))?;
        if !binding.graphs(profile).is_empty() && graphs.is_none() {
            graphs = Some(PreparedGraphs::load(&access, workspace, normalized_authority.as_ref().expect("normalized authority"), model, &graph_needs).await?);
        }
        match binding {
            UpperStage::Configuration => {
                crate::analysis_prepare::configuration(
                    access,
                    output,
                    model,
                    workspace,
                    &prepared.configuration,
                )
                .await?
            }
            UpperStage::Native => {
                crate::analysis_prepare::native_inventory(access, output, workspace, model).await?
            }
            UpperStage::EmbeddingConfiguration => {
                crate::analysis_prepare::embedding_configuration(
                    access,
                    output,
                    model,
                    workspace,
                    prepared.embedding.as_ref(),
                )
                .await?
            }
            UpperStage::Text => {
                crate::analytic_text::publish(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.text.clone(),
                )
                .await?
            }
            UpperStage::Embedding => {
                crate::analytic_embedding::produce(
                    access,
                    output,
                    workspace,
                    model,
                    embedder,
                    cache.clone(),
                )
                .await?
            }
            UpperStage::CatalogCore => {
                crate::catalog_core::produce(access, output, workspace, model).await?
            }
            UpperStage::CatalogEvidence => {
                crate::catalog_evidence::produce(access, output, workspace, model).await?
            }
            UpperStage::Selection => {
                crate::catalog_selection::produce(access, output, workspace, model).await?
            }
            UpperStage::Local => {
                local = crate::local_semantics::run(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.definition(AnalysisMethod::LocalTransfers)?,
                )
                .await?;
            }
            UpperStage::Base => {
                evaluations = crate::semantic_execution::evaluate_base(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.definition(AnalysisMethod::Execution)?,
                    local.as_ref(),
                )
                .await?;
            }
            UpperStage::Completion => {
                completed_bodies = crate::semantic_execution::complete_base(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.definition(AnalysisMethod::Completion)?,
                    evaluations.as_ref(),
                )
                .await?;
            }
            UpperStage::SourceCalls => {
                source_calls = crate::semantic_execution::prepare_source_calls(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.definition(AnalysisMethod::SourceCalls)?,
                    bindings.as_ref(),
                    evaluations.as_ref(),
                    completed_bodies.as_ref(),
                )
                .await?;
                completed_bodies = None;
            }
            UpperStage::Enriched => {
                crate::semantic_execution::enrich(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.definition(AnalysisMethod::EnrichedExecution)?,
                    bindings.as_ref(),
                    evaluations.as_ref(),
                    source_calls.as_ref(),
                )
                .await?;
                source_calls = None;
            }
            UpperStage::Models => {
                crate::semantic_models::apply(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.definition(AnalysisMethod::Models)?,
                    bindings.as_ref(),
                    evaluations.as_ref(),
                    local.as_ref(),
                )
                .await?;
                evaluations = None;
            }
            UpperStage::Summary => {
                crate::semantic_summaries::produce(
                    access,
                    output,
                    workspace,
                    model,
                    prepared.definition(AnalysisMethod::Summaries)?,
                    graphs.as_ref(),
                    bindings.as_ref(),
                    local.as_ref(),
                )
                .await?;
                local = None;
            }
            UpperStage::Structural => {
                crate::structural::produce(
                    access,
                    output,
                    workspace,
                    model,
                    graphs.as_ref().expect("prepared selected graphs"),
                )
                .await?
            }
            UpperStage::Analytic => {
                crate::analytic::produce(
                    access,
                    output,
                    workspace,
                    model,
                    graphs.as_ref().expect("prepared selected graphs"),
                )
                .await?
            }
            UpperStage::AnalysisFrontier => {
                crate::final_coverage::produce(
                    access,
                    output,
                    workspace,
                    model,
                    analysis::frontier::Target::Analysis,
                )
                .await?
            }
            UpperStage::CatalogFrontier => {
                crate::final_coverage::produce(
                    access,
                    output,
                    workspace,
                    model,
                    analysis::frontier::Target::Catalog,
                )
                .await?
            }
            UpperStage::Synthesis => {
                crate::synthesis::produce(access, output, workspace, model).await?
            }
            UpperStage::Retrieval => {
                crate::retrieval::produce(access, output, workspace, model, embedder, cache.clone())
                    .await?
            }
        }
        completed.insert(declaration.name);
        freeze_completed_inputs(workspace, &schedule, &completed, &mut frozen)?;
    }
    workspace
        .finish_compilation(capture_identity, frontier, profile, configuration)
        .await
}
