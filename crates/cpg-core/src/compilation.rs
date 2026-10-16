//! One cumulative attempt through analysis/catalog. Semantic owners supply declarations and proofs.
use crate::{
    facts,
    normalize::pipeline::Normalization,
    workspace::{ProducerOutput, Workspace},
};
use cpg_extract::bundle::{CapturedInputs, ProviderStage};
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use lctx_model::domain::embedding::Embedder;
use lctx_model::domain::{
    admission::Frontier,
    analysis::{
        AnalysisDefinition, AnalysisMethod,
        preparation::{Configuration, ConfigurationGroup},
        settings::AnalyticsConfiguration,
    },
    stages::*,
    *,
};
use lctx_tracing::InContext;
use std::sync::Arc;
#[cfg(test)]
mod cache_controls;
mod preparation;
mod reuse;
mod uniform;
/// Closed executable upper routes. Metadata and runner dispatch use the same finite type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpperStage {
    Configuration(ConfigurationGroup),
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
    fn reuse(
        self,
        effect: Effect,
        profile: Profile,
        prepared: &PreparedCompilation,
    ) -> reuse::Eligibility {
        use reuse::Eligibility::*;
        if effect != Effect::Pure {
            return Fresh("external effects retain their existing owner");
        }
        // This owner only reconstructs compact metadata from the current selected inputs.
        // Avoid lookup: a miss would arm independent retention for work cheaper to reproduce.
        if uniform::unrequested(self, profile, prepared) {
            return Fresh("unrequested uniform metadata is produced from current inputs");
        }
        match self {
            Self::Text
            | Self::CatalogCore
            | Self::CatalogEvidence
            | Self::Selection
            | Self::Enriched
            | Self::Models
            | Self::Summary
            | Self::Structural
            | Self::Analytic
            | Self::Synthesis
            | Self::Retrieval => Rows,
            Self::Local | Self::Base | Self::Completion | Self::SourceCalls
                if profile == Profile::Catalog =>
            {
                Rows
            }
            Self::Local => Fresh(
                "Local semantic equivalence requires its complete current transfer/theory/field kernels; replay would add work",
            ),
            Self::Base | Self::Completion | Self::SourceCalls => Fresh(
                "complete current private-owner validation repeats the production kernel; replay would add work",
            ),
            Self::Configuration(_) | Self::Native | Self::EmbeddingConfiguration => {
                Fresh("current configuration and admission metadata must refresh")
            }
            Self::AnalysisFrontier | Self::CatalogFrontier => {
                Fresh("frontier admission observes current completed ownership")
            }
            Self::Embedding => Fresh("embedding value reuse has its own inference cache"),
        }
    }
    const ALL: [Self; 25] = [
        Self::Configuration(ConfigurationGroup::Authored),
        Self::Configuration(ConfigurationGroup::Methods),
        Self::Configuration(ConfigurationGroup::Projections),
        Self::Configuration(ConfigurationGroup::Analytics),
        Self::Configuration(ConfigurationGroup::Retrieval),
        Self::Native,
        Self::EmbeddingConfiguration,
        Self::Text,
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
        Self::Embedding,
        Self::Analytic,
        Self::Selection,
        Self::Synthesis,
        Self::Retrieval,
        Self::AnalysisFrontier,
        Self::CatalogFrontier,
    ];
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Configuration(group) => group.name(),
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
            Self::Configuration(_) => Frontier::Analysis,
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
    #[cfg(test)]
    fn boundary(self) -> Option<PublicationBoundary> {
        analysis::owner::AnalysisOwner::resolve(self.name())
            .ok()
            .and_then(|owner| owner.boundary())
    }
    fn graphs(self, profile: Profile) -> &'static [projection::ProjectionName] {
        match self {
            Self::Configuration(_) => &[],
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
            Self::Summary if profile == Profile::Behavioral => {
                &[projection::ProjectionName::CallableInvocation]
            }
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
    fn bindings(self, profile: Profile) -> bool {
        profile == Profile::Behavioral
            && matches!(
                self,
                Self::SourceCalls | Self::Enriched | Self::Models | Self::Summary
            )
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
        if let Ok(owner) = analysis::owner::AnalysisOwner::resolve(binding.name()) {
            let selected = analysis::owner::SelectedDefinitions {
                definitions: self.configuration.definitions(),
                analytics: self.configuration.analytics().iter().next(),
                text: Some(&self.text),
                retrieval: self.configuration.retrieval().iter().next(),
            };
            return owner.declaration(profile, &selected, model, order);
        }
        Ok(match binding {
            UpperStage::Configuration(group) => self.configuration.declaration(group),
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
        let groups = analysis::owner::publication_groups(facts, self.frontier == Frontier::Catalog);
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
                        ModelError::Invalid("shared producer has no publication boundary".into())
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

/// Validate each request role against the exact selected immutable input. Implicit selectors
/// resolve to the actual epoch; an explicit requested epoch must remain unchanged.
fn selected_product_dependencies(
    workspace: &Workspace,
    declaration: &Stage,
    selected: &Stage,
    request: &compilation_product::ProductRequest,
) -> Result<Vec<compilation_product::DependencyToken>, ModelError> {
    if declaration.inputs.len() != selected.inputs.len()
        || request.dependencies.len() != declaration.inputs.len()
    {
        return Err(ModelError::Conflict(
            "compiler product selected input inventory",
        ));
    }
    request
        .dependencies
        .iter()
        .map(|token| {
            if token.kind != compilation_product::DependencyKind::ExactView {
                return Err(ModelError::Conflict("compiler product selected input kind"));
            }
            let role = token
                .role
                .strip_prefix("input/")
                .and_then(|role| role.parse::<usize>().ok())
                .filter(|role| token.role == format!("input/{role}"))
                .ok_or(ModelError::Conflict("compiler product selected input role"))?;
            let declared = declaration.inputs.get(role).ok_or(ModelError::Conflict(
                "compiler product selected input role range",
            ))?;
            let input = &selected.inputs[role];
            if declared.name() != input.name()
                || token.relation != input.name()
                || token.prefix.as_deref() != input.prefix().map(|prefix| prefix.name())
                || declared
                    .prefix()
                    .is_some_and(|prefix| Some(prefix) != input.prefix())
            {
                return Err(ModelError::Conflict(
                    "compiler product selected input epoch",
                ));
            }
            let relation = workspace
                .model()
                .relation(input.name())
                .ok_or(ModelError::Schema("product input relation"))?;
            let mut validation = ValidationInput::of_relation(relation, &[]);
            if let Some(prefix) = input.prefix() {
                validation = validation.at_epoch(prefix);
            }
            if workspace.input_relation(&validation)?.snapshot().identity() != token.identity {
                return Err(ModelError::Conflict(
                    "compiler product selected input identity",
                ));
            }
            let mut resolved = token.clone();
            resolved.prefix = input.prefix().map(|prefix| prefix.name().to_owned());
            Ok(resolved)
        })
        .collect()
}

/// Freeze only completed shared descriptors, once all statically named producers for a
/// boundary finish. Persist each exact binding before making its in-memory selector visible.
async fn freeze_completed_inputs(
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
        workspace
            .freeze_scheduled_prefix(schedule, group.epoch)
            .await?;
        frozen.insert(group.epoch);
    }
    Ok(())
}

/// Private preparation accompanies the actual terminal owner, never aggregate visibility.
enum StagePreparation {
    None,
    Receivers(
        Arc<crate::normalize::ProducedNormalization<normalized::receiver::VerifiedReceivers>>,
    ),
    Events(
        Arc<
            crate::normalize::ProducedNormalization<
                normalized::event_normalization::VerifiedEvents,
            >,
        >,
    ),
    BindingApplication(Box<normalized::binding_normalization::VerifiedBindings>),
    Local(Arc<crate::local_semantics::PreparedLocal>),
    Evaluations(
        Arc<crate::semantic_execution::Produced<execution::production::ProducedEvaluations>>,
    ),
    Bodies(
        Arc<crate::semantic_execution::Produced<execution::completion_production::ProducedBodies>>,
    ),
    SourceCalls(
        Arc<
            crate::semantic_execution::Produced<
                execution::source_call_records::ProducedSourceCalls,
            >,
        >,
    ),
}
struct StageResult {
    name: &'static str,
    owner: completed::CompletedContribution,
    _charge: Box<dyn resources::Reservation>,
    preparation: StagePreparation,
}
impl StageResult {
    fn completed(
        workspace: &Workspace,
        name: &'static str,
        preparation: StagePreparation,
    ) -> Result<Self, ModelError> {
        let (owner, charge) = workspace.completed_owner(name)?;
        Ok(Self {
            name,
            owner,
            _charge: charge,
            preparation,
        })
    }
}

/// Terminal results remain private until the scheduler accepts them. The schedule
/// bounds this queue; each result retains its original preparation/descriptor charge.
struct ReadyStages<T> {
    rows: std::collections::VecDeque<T>,
    limit: usize,
    _charge: Box<dyn resources::Reservation>,
}
impl<T> ReadyStages<T> {
    fn new(limit: usize, budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        let bytes = limit
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(ModelError::Schema(
                "compiler ready stage allowance overflow",
            ))?;
        let charge = budget.reserve("compiler ready stage results", bytes)?;
        Ok(Self {
            rows: std::collections::VecDeque::with_capacity(limit),
            limit,
            _charge: charge,
        })
    }
    fn push(&mut self, row: T) -> Result<(), ModelError> {
        if self.rows.len() == self.limit {
            return Err(ModelError::Conflict(
                "compiler ready stages exceed schedule",
            ));
        }
        self.rows.push_back(row);
        Ok(())
    }
}

/// Coordinator selection/freezing also collects independently scheduled producer results
/// while awaiting its action. Task joins are retained by the workspace execution owner.
/// After a sibling refusal, finish the already-started action but accept no success.
async fn coordinate_with_pending<T, A>(
    action: impl std::future::Future<Output = Result<A, ModelError>>,
    cancellation: &crate::workspace::Cancellation,
    pending: &mut FuturesUnordered<BoxFuture<'_, Result<T, ModelError>>>,
    ready: &mut ReadyStages<T>,
) -> Result<A, ModelError> {
    tokio::pin!(action);
    let mut failure = None;
    let mut terminal = completion::Completion::default();
    loop {
        tokio::select! {
            biased;
            result = pending.next(), if !pending.is_empty() => {
                let result = result.expect("nonempty pending stage owner");
                let accepted = match result {
                    Ok(row) if failure.is_none() => ready.push(row),
                    Ok(_) => Ok(()),
                    Err(error) => Err(error),
                };
                if let Err(error) = accepted {
                    cancellation.cancel();
                    if failure.is_none() { failure = Some(error); }
                    else { terminal.step("sibling stage during coordinator wait", Err(error)); }
                }
            }
            result = &mut action => {
                return match failure {
                    Some(error) => {
                        if let Err(secondary) = result {
                            terminal.step("coordinator action after sibling refusal", Err(secondary));
                        }
                        completion::complete(Err(error), terminal)
                    }
                    None => result,
                };
            }
        }
    }
}

/// Failure settlement keeps polling every owned task through its cleanup. Successful
/// sibling results are not admitted after failure; their private preparation drops here.
async fn settle_upper_pending<T>(
    cancellation: &crate::workspace::Cancellation,
    pending: &mut FuturesUnordered<BoxFuture<'_, Result<T, ModelError>>>,
    ready: &mut ReadyStages<T>,
    primary: ModelError,
) -> Result<(), ModelError> {
    cancellation.cancel();
    ready.rows.clear();
    let mut terminal = completion::Completion::default();
    while let Some(result) = pending.next().await {
        if let Err(error) = result {
            terminal.step("sibling upper stage completion", Err(error));
        }
    }
    completion::complete(Err(primary), terminal)
}

/// Preparation borrows are owned by the launched task and selected only for its actual consumers.
struct UpperPreparation {
    bindings: Option<Arc<crate::analysis_bindings::PreparedBindings>>,
    local: Option<Arc<crate::local_semantics::PreparedLocal>>,
    evaluations: Option<
        Arc<crate::semantic_execution::Produced<execution::production::ProducedEvaluations>>,
    >,
    completed_bodies: Option<
        Arc<crate::semantic_execution::Produced<execution::completion_production::ProducedBodies>>,
    >,
    source_calls: Option<
        Arc<
            crate::semantic_execution::Produced<
                execution::source_call_records::ProducedSourceCalls,
            >,
        >,
    >,
    graphs: Option<crate::analysis_graphs::SelectedGraphs>,
}
struct UpperTask {
    workspace: Arc<Workspace>,
    model: Arc<ValidatedModel>,
    declaration: Stage,
    binding: UpperStage,
    access: crate::workspace::CompletedInputs,
    output: ProducerOutput,
    profile: Profile,
    prepared: Arc<PreparedCompilation>,
    embedder: Option<Arc<dyn Embedder>>,
    cache: Option<Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>>,
    checked: Option<crate::workspace::CheckedInputs>,
    shared: UpperPreparation,
    selected_dependencies: usize,
}
impl UpperTask {
    async fn run(self) -> Result<StageResult, ModelError> {
        use lctx_native::phase::Phase;
        let Self {
            workspace,
            model,
            declaration,
            binding,
            access,
            output,
            profile,
            prepared,
            embedder,
            cache,
            checked,
            shared,
            selected_dependencies,
        } = self;
        let workspace = &workspace;
        let model = &model;
        let phase = Phase::begin(declaration.name);
        let result = async {
            let eligibility = binding.reuse(declaration.effect, profile, &prepared);
            let hit = if eligibility.reason().is_none() {
                output.reuse_product().await?
            } else {
                None
            };
            tracing::debug!(
                operation = declaration.name,
                hit = hit.is_some(),
                inputs = selected_dependencies,
                reason = eligibility.reason(),
                "compiler product disposition"
            );
            let mut result_preparation = StagePreparation::None;
            if let Some(outcome) = hit {
                output.finish(outcome).await?;
            } else if uniform::unrequested(binding, profile, &prepared) {
                uniform::produce(access, output, workspace, &declaration).await?;
            } else {
                if analysis::owner::AnalysisOwner::resolve(binding.name())
                    .is_ok_and(|owner| owner.can_be_wholly_unrequested())
                {
                    output
                        .declare_async::<uniform_outcome::UniformNotRequested>()
                        .await?;
                }
                let graphs = match &shared.graphs {
                    Some(selection) => Some(
                        selection
                            .load(
                                &access,
                                workspace,
                                checked
                                    .as_ref()
                                    .ok_or(ModelError::Conflict("normalized authority absent"))?,
                                model,
                            )
                            .await?,
                    ),
                    None => None,
                };
                match binding {
                    UpperStage::Configuration(group) => {
                        crate::analysis_prepare::configuration(
                            access,
                            output,
                            model,
                            workspace,
                            &prepared.configuration,
                            group,
                        )
                        .await?
                    }
                    UpperStage::Native => {
                        crate::analysis_prepare::native_inventory(access, output, workspace, model)
                            .await?
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
                            embedder.as_deref(),
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
                        let value = crate::local_semantics::run(
                            access,
                            output,
                            workspace,
                            model,
                            prepared.definition(AnalysisMethod::LocalTransfers)?,
                        )
                        .await?;
                        result_preparation = value
                            .map(Arc::new)
                            .map(StagePreparation::Local)
                            .unwrap_or(StagePreparation::None);
                    }
                    UpperStage::Base => {
                        let value = crate::semantic_execution::evaluate_base(
                            access,
                            output,
                            workspace,
                            model,
                            prepared.definition(AnalysisMethod::Execution)?,
                            shared.local.as_deref(),
                        )
                        .await?;
                        result_preparation = value
                            .map(Arc::new)
                            .map(StagePreparation::Evaluations)
                            .unwrap_or(StagePreparation::None);
                    }
                    UpperStage::Completion => {
                        let value = crate::semantic_execution::complete_base(
                            access,
                            output,
                            workspace,
                            model,
                            prepared.definition(AnalysisMethod::Completion)?,
                            shared.evaluations.as_deref(),
                        )
                        .await?;
                        result_preparation = value
                            .map(Arc::new)
                            .map(StagePreparation::Bodies)
                            .unwrap_or(StagePreparation::None);
                    }
                    UpperStage::SourceCalls => {
                        let value = crate::semantic_execution::prepare_source_calls(
                            access,
                            output,
                            workspace,
                            model,
                            prepared.definition(AnalysisMethod::SourceCalls)?,
                            shared.bindings.as_deref(),
                            shared.evaluations.as_deref(),
                            shared.completed_bodies.as_deref(),
                        )
                        .await?;
                        result_preparation = value
                            .map(Arc::new)
                            .map(StagePreparation::SourceCalls)
                            .unwrap_or(StagePreparation::None);
                    }
                    UpperStage::Enriched => {
                        crate::semantic_execution::enrich(
                            access,
                            output,
                            workspace,
                            model,
                            prepared.definition(AnalysisMethod::EnrichedExecution)?,
                            shared.bindings.as_deref(),
                            shared.evaluations.as_deref(),
                            shared.source_calls.as_deref(),
                        )
                        .await?;
                    }
                    UpperStage::Models => {
                        crate::semantic_models::apply(
                            access,
                            output,
                            workspace,
                            model,
                            prepared.definition(AnalysisMethod::Models)?,
                            shared.bindings.as_deref(),
                            shared.evaluations.as_deref(),
                            shared.local.as_deref(),
                        )
                        .await?;
                    }
                    UpperStage::Summary => {
                        crate::semantic_summaries::produce(
                            access,
                            output,
                            workspace,
                            model,
                            prepared.definition(AnalysisMethod::Summaries)?,
                            graphs.as_ref(),
                            shared.bindings.as_deref(),
                            shared.local.as_deref(),
                        )
                        .await?;
                    }
                    UpperStage::Structural => {
                        crate::structural::produce(
                            access,
                            output,
                            workspace,
                            model,
                            graphs
                                .as_ref()
                                .ok_or(ModelError::Conflict("prepared selected graphs absent"))?,
                        )
                        .await?
                    }
                    UpperStage::Analytic => {
                        crate::analytic::produce(access, output, workspace, model, graphs.as_ref())
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
                        crate::retrieval::produce(
                            access,
                            output,
                            workspace,
                            model,
                            embedder.as_deref(),
                            cache.clone(),
                        )
                        .await?
                    }
                }
            }
            StageResult::completed(workspace, declaration.name, result_preparation)
        }
        .await;
        phase.finish_result(&result);
        result
    }
}

/// Execute all selected compiler owners into exact completed native views. Publication is a separate
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
    prepared: Option<Arc<PreparedCompilation>>,
    embedder: Option<Arc<dyn Embedder>>,
    cache: Option<Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>>,
) -> Result<(), ModelError> {
    let span = tracing::info_span!(target: "lctx_phase", "compilation", attempt=%workspace.native().attempt().hex(), profile = profile.name(), frontier = frontier.name());
    let mut cancellation = CompilerCancellation::new(workspace.cancellation());
    let result = boxed_driver(
        workspace,
        captured,
        profile,
        configuration,
        frontier,
        prepared,
        embedder,
        cache,
    )
    .with_tracing_span(span)
    .await;
    if result.is_ok() { cancellation.disarm(); }
    result
}

/// Dropping a compiler coordinator requests cooperative stop; actual joins stay owned.
struct CompilerCancellation(Option<crate::workspace::Cancellation>);
impl CompilerCancellation {
    fn new(cancellation: crate::workspace::Cancellation) -> Self { Self(Some(cancellation)) }
    fn disarm(&mut self) { self.0 = None; }
}
impl Drop for CompilerCancellation {
    fn drop(&mut self) { if let Some(cancellation) = &self.0 { cancellation.cancel(); } }
}

// Keep construction and polling of the full compiler future inside this library boundary.
#[inline(never)]
#[allow(
    clippy::too_many_arguments,
    reason = "The private driver preserves the explicit compiler entry point dependencies"
)]
fn boxed_driver<'a>(
    workspace: &'a Arc<Workspace>,
    captured: Arc<CapturedInputs>,
    profile: Profile,
    configuration: ContentHash,
    frontier: Frontier,
    prepared: Option<Arc<PreparedCompilation>>,
    embedder: Option<Arc<dyn Embedder>>,
    cache: Option<Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    compile_driver(
        workspace,
        captured,
        profile,
        configuration,
        frontier,
        prepared,
        embedder,
        cache,
        None,
    )
    .boxed()
}

#[allow(
    clippy::too_many_arguments,
    reason = "The private driver preserves the explicit compiler entry point dependencies"
)]
async fn compile_driver(
    workspace: &Arc<Workspace>,
    captured: Arc<CapturedInputs>,
    profile: Profile,
    configuration: ContentHash,
    frontier: Frontier,
    prepared: Option<Arc<PreparedCompilation>>,
    embedder: Option<Arc<dyn Embedder>>,
    cache: Option<Arc<dyn lctx_model::domain::embedding::cache::EmbeddingCache>>,
    providers: Option<Vec<Box<dyn ProviderStage<ProducerOutput>>>>,
) -> Result<(), ModelError> {
    use lctx_native::phase::Phase;
    let setup_phase = Phase::begin("compiler_setup");
    let setup = (|| {
        workspace.native().set_frontier(frontier)?;
        let model = workspace.model();
        let capture_identity = workspace.captures(&captured)?;
        let providers = providers.unwrap_or_else(|| facts::providers(configuration));
        // Plan every declaration before running native effects. The schedule is static dependency
        // metadata only; completed streams, not execution grants, supply runtime inputs.
        let schedule = if let Some(prepared) = prepared.as_ref() {
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
            let mut declarations: Vec<_> =
                providers.iter().map(|p| p.declaration(profile)).collect();
            if frontier == Frontier::Normalized {
                declarations.extend(Normalization::ALL.map(|s| s.declaration(profile)));
            }
            Schedule::build(model, declarations, &[], profile)?
        };
        let fact_names: std::collections::BTreeSet<_> = providers
            .iter()
            .map(|p| p.declaration(profile).name)
            .collect();
        Ok::<_, ModelError>((model, capture_identity, providers, schedule, fact_names))
    })();
    setup_phase.finish_result(&setup);
    let (model, capture_identity, providers, schedule, fact_names) = setup?;
    let schedule = workspace.install_membership_demand(schedule)?;

    let providers_phase = Phase::begin("providers");
    let providers_result = async {
        facts::compile_facts(workspace, &captured, profile, providers, Default::default()).await?;
        drop(captured);
        workspace.facts_availability_async(profile).await?;
        Ok::<(), ModelError>(())
    }
    .await;
    providers_phase.finish_result(&providers_result);
    providers_result?;

    let mut completed = schedule
        .stages()
        .iter()
        .filter(|stage| fact_names.contains(stage.name))
        .map(|stage| stage.name)
        .collect();
    let mut frozen = Default::default();
    freeze_completed_inputs(workspace, &schedule, &completed, &mut frozen).await?;
    for name in &completed {
        workspace.membership_stage_completed(name)?;
    }
    let mut binding_application = None;
    let mut receiver_authority: Option<
        Arc<crate::normalize::ProducedNormalization<normalized::receiver::VerifiedReceivers>>,
    > = None;
    let mut event_authority: Option<
        Arc<
            crate::normalize::ProducedNormalization<
                normalized::event_normalization::VerifiedEvents,
            >,
        >,
    > = None;
    let mut products = reuse::SelectedBindings::new(workspace.budget())?;
    let mut started = completed.clone();
    let mut pending = FuturesUnordered::new();
    let mut ready = ReadyStages::new(schedule.stages().len(), workspace.budget())?;
    let mut failure = None;
    loop {
        if let Some(error) = failure.take() {
            return settle_upper_pending(
                &workspace.cancellation(),
                &mut pending,
                &mut ready,
                error,
            )
            .await;
        }
        if ready.rows.is_empty() {
            for declaration in schedule.stages() {
                let Some(normalization) = Normalization::ALL
                    .into_iter()
                    .find(|n| n.declaration(profile).name == declaration.name)
                else {
                    continue;
                };
                if started.contains(declaration.name) {
                    continue;
                }
                let launch = coordinate_with_pending(
                    async {
                        if !schedule.ready(declaration.name, &completed)? {
                            return Ok(None);
                        }
                        let selected = workspace
                            .bind_scheduled_inputs(&schedule, declaration)
                            .await?;
                        let access =
                            workspace.stage_inputs_selected(declaration, &selected, profile)?;
                        let output = workspace.producer(declaration, profile, access);
                        let access = output.bound_inputs()?;
                        let product = output.product_request()?;
                        products.bind_selected(
                            &product,
                            &selected_product_dependencies(
                                workspace,
                                declaration,
                                &selected,
                                &product,
                            )?,
                        )?;
                        Ok::<_, ModelError>(Some((access, output)))
                    },
                    &workspace.cancellation(),
                    &mut pending,
                    &mut ready,
                )
                .await;
                let (access, output) = match launch {
                    Ok(Some(value)) => value,
                    Ok(None) => continue,
                    Err(error) => {
                        workspace.cancellation().cancel();
                        failure = Some(error);
                        break;
                    }
                };
                started.insert(declaration.name);
                let receiver = receiver_authority.clone();
                let event = event_authority.clone();
                let stage_owner = workspace.clone();
                let workspace = workspace.clone();
                let model = model.clone();
                let declaration = declaration.clone();
                let producer_span = tracing::info_span!(target: "lctx_phase", "compiler_producer", producer=declaration.name, attempt=%workspace.native().attempt().hex());
                let task = stage_owner.spawn_stage(
                    async move {
                        let phase = Phase::begin(declaration.name);
                        let result = async {
                            let preparation = match normalization {
                                Normalization::Receivers => StagePreparation::Receivers(Arc::new(
                                    crate::normalize::receivers_produced(
                                        access, output, &workspace, &model,
                                    )
                                    .await?,
                                )),
                                Normalization::Events => StagePreparation::Events(Arc::new(
                                    crate::normalize::events_produced(
                                        access,
                                        output,
                                        &workspace,
                                        &model,
                                        receiver.as_deref().ok_or(ModelError::Conflict(
                                            "receiver owner authority absent",
                                        ))?,
                                    )
                                    .await?,
                                )),
                                Normalization::Bindings => {
                                    StagePreparation::BindingApplication(Box::new(
                                        crate::normalize::bindings_prepared(
                                            access,
                                            output,
                                            &workspace,
                                            &model,
                                            receiver.as_deref().ok_or(ModelError::Conflict(
                                                "receiver owner authority absent",
                                            ))?,
                                            event.as_deref().ok_or(ModelError::Conflict(
                                                "event owner authority absent",
                                            ))?,
                                        )
                                        .await?,
                                    ))
                                }
                                _ => {
                                    normalization
                                        .run(access, output, &workspace, &model)
                                        .await?;
                                    StagePreparation::None
                                }
                            };
                            StageResult::completed(&workspace, declaration.name, preparation)
                        }
                        .await;
                        phase.finish_result(&result);
                        result
                    }
                    .with_tracing_span(producer_span),
                );
                match task {
                    Ok(task) => pending.push(task),
                    Err(error) => { failure = Some(error); break; }
                }
            }
        }
        if failure.is_some() {
            continue;
        }
        let result = match ready.rows.pop_front() {
            Some(result) => Ok(result),
            None => match pending.next().await {
                Some(result) => result,
                None => break,
            },
        };
        let accepted = match result {
            Ok(result) if failure.is_none() => {
                async {
                    if result.owner.spec.producer != result.name {
                        return Err(ModelError::Conflict("terminal stage owner differs"));
                    }
                    match result.preparation {
                        StagePreparation::Receivers(value) => receiver_authority = Some(value),
                        StagePreparation::Events(value) => event_authority = Some(value),
                        StagePreparation::BindingApplication(value) => {
                            binding_application = Some(value)
                        }
                        StagePreparation::None => {}
                        _ => {
                            return Err(ModelError::Conflict(
                                "normalization returned upper preparation",
                            ));
                        }
                    }
                    completed.insert(result.name);
                    coordinate_with_pending(
                        freeze_completed_inputs(workspace, &schedule, &completed, &mut frozen),
                        &workspace.cancellation(),
                        &mut pending,
                        &mut ready,
                    )
                    .await?;
                    workspace.membership_stage_completed(result.name)?;
                    Ok(())
                }
                .await
            }
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        };
        if let Err(error) = accepted {
            workspace.cancellation().cancel();
            failure = Some(error);
        }
    }
    if Normalization::ALL
        .into_iter()
        .filter(|n| {
            schedule
                .stages()
                .iter()
                .any(|s| s.name == n.declaration(profile).name)
        })
        .any(|n| !completed.contains(n.declaration(profile).name))
    {
        return Err(ModelError::Conflict(
            "normalization readiness made no progress",
        ));
    }
    drop(event_authority);
    drop(receiver_authority);
    // Source owners are admitted once while the normalized dependency set is immutable.
    // Prepared consumer authorities project this lifetime; upper stages do not replay owners.
    let normalized_authority = if prepared.is_some() {
        let phase = Phase::begin("normalized_admission");
        let authority = workspace.admit_semantics(profile).await;
        phase.finish_result(&authority);
        Some(authority?)
    } else {
        None
    };
    let mut bindings = match (binding_application, normalized_authority.as_ref()) {
        (Some(application), Some(authority)) if profile == Profile::Behavioral => Some(Arc::new(
            crate::analysis_bindings::PreparedBindings::new(*application, authority)?,
        )),
        _ => None,
    };
    let analytics_requested = prepared.as_ref().is_some_and(|p| analytics::build::requested(p.settings()));
    let mut graphs = crate::analysis_graphs::GraphPreparation::new(workspace.budget())?;
    let mut local = None;
    let mut evaluations = None;
    let mut completed_bodies = None;
    let mut source_calls = None;
    let mut pending = FuturesUnordered::new();
    let mut failure = None;
    loop {
        if let Some(error) = failure.take() {
            return settle_upper_pending(
                &workspace.cancellation(),
                &mut pending,
                &mut ready,
                error,
            )
            .await;
        }
        if ready.rows.is_empty() {
            for declaration in schedule.stages() {
                if started.contains(declaration.name) {
                    continue;
                }
                let launch = coordinate_with_pending(
                    async {
                        if !schedule.ready(declaration.name, &completed)? {
                            return Ok(None);
                        }
                        let binding = UpperStage::resolve(declaration.name)?;
                        let prepared =
                            prepared.clone().ok_or(ModelError::Conflict("missing upper configuration"))?;
                        let selected = workspace
                            .bind_scheduled_inputs(&schedule, declaration)
                            .await?;
                        let access =
                            workspace.stage_inputs_selected(declaration, &selected, profile)?;
                        let output = workspace.producer(declaration, profile, access);
                        let access = output.bound_inputs()?;
                        let product = output.product_request()?;
                        let selected_dependencies = selected_product_dependencies(
                            workspace,
                            declaration,
                            &selected,
                            &product,
                        )?;
                        products.bind_selected(&product, &selected_dependencies)?;
                        let needs =
                            preparation::Needs::remaining([binding], profile, analytics_requested);
                        let shared = UpperPreparation {
                            bindings: needs.bindings.then(|| bindings.clone()).flatten(),
                            local: needs.local.then(|| local.clone()).flatten(),
                            evaluations: needs.evaluations.then(|| evaluations.clone()).flatten(),
                            completed_bodies: needs
                                .bodies
                                .then(|| completed_bodies.clone())
                                .flatten(),
                            source_calls: needs
                                .source_calls
                                .then(|| source_calls.clone())
                                .flatten(),
                            graphs: if needs.graphs.is_empty() {
                                None
                            } else {
                                graphs.select(binding.graphs(profile))?
                            },
                        };
                        Ok::<_, ModelError>(Some(UpperTask {
                            workspace: workspace.clone(),
                            model: model.clone(),
                            declaration: declaration.clone(),
                            binding,
                            access,
                            output,
                            profile,
                            prepared,
                            embedder: embedder.clone(),
                            cache: cache.clone(),
                            checked: normalized_authority.clone(),
                            shared,
                            selected_dependencies: selected_dependencies.len(),
                        }))
                    },
                    &workspace.cancellation(),
                    &mut pending,
                    &mut ready,
                )
                .await;
                match launch {
                    Ok(Some(task)) => {
                        started.insert(declaration.name);
                        let producer_span = tracing::info_span!(target: "lctx_phase", "compiler_producer", producer=declaration.name, attempt=%workspace.native().attempt().hex());
                        match workspace.spawn_stage(task.run().with_tracing_span(producer_span)) {
                            Ok(task) => pending.push(task),
                            Err(error) => { failure = Some(error); break; }
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        workspace.cancellation().cancel();
                        failure = Some(error);
                        break;
                    }
                }
            }
        }
        if failure.is_some() {
            continue;
        }
        let result = match ready.rows.pop_front() {
            Some(result) => Ok(result),
            None => match pending.next().await {
                Some(result) => result,
                None => break,
            },
        };
        let accepted = match result {
            Ok(result) if failure.is_none() => {
                async {
                    if result.owner.spec.producer != result.name {
                        return Err(ModelError::Conflict("terminal upper stage owner differs"));
                    }
                    match result.preparation {
                        StagePreparation::Local(value) => local = Some(value),
                        StagePreparation::Evaluations(value) => evaluations = Some(value),
                        StagePreparation::Bodies(value) => completed_bodies = Some(value),
                        StagePreparation::SourceCalls(value) => source_calls = Some(value),
                        StagePreparation::None => {}
                        _ => {
                            return Err(ModelError::Conflict(
                                "upper stage returned normalization preparation",
                            ));
                        }
                    }
                    completed.insert(result.name);
                    coordinate_with_pending(
                        freeze_completed_inputs(workspace, &schedule, &completed, &mut frozen),
                        &workspace.cancellation(),
                        &mut pending,
                        &mut ready,
                    )
                    .await?;
                    workspace.membership_stage_completed(result.name)?;
                    Ok(())
                }
                .await
            }
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        };
        if let Err(error) = accepted {
            workspace.cancellation().cancel();
            failure = Some(error);
        }
        // Completed membership, rather than list position or launch order, retires demand.
        // This includes every running task and every remaining selected cache/optional branch.
        let needs = preparation::Needs::remaining(
            schedule
                .stages()
                .iter()
                .filter(|stage| !completed.contains(stage.name))
                .filter_map(|stage| UpperStage::resolve(stage.name).ok()),
            profile,
            analytics_requested,
        );
        graphs.retain(&needs.graphs);
        preparation::release_unused(&mut bindings, needs.bindings);
        preparation::release_unused(&mut local, needs.local);
        preparation::release_unused(&mut evaluations, needs.evaluations);
        preparation::release_unused(&mut completed_bodies, needs.bodies);
        preparation::release_unused(&mut source_calls, needs.source_calls);
    }
    if schedule
        .stages()
        .iter()
        .any(|stage| !completed.contains(stage.name))
    {
        return Err(ModelError::Conflict(
            "upper stage readiness made no progress",
        ));
    }
    drop((
        graphs,
        bindings,
        local,
        evaluations,
        completed_bodies,
        source_calls,
    ));
    let phase = Phase::begin("compiler_completion");
    let result = workspace
        .finish_compilation(capture_identity, frontier, profile, configuration)
        .await;
    phase.finish_result(&result);
    result
}

#[cfg(test)]
mod upper_completion_controls {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct OwnedPreparation {
        _charge: Box<dyn resources::Reservation>,
        dropped: Arc<AtomicUsize>,
    }
    impl Drop for OwnedPreparation {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn coordinator_drop_signals_stop_and_success_disarms_it() {
        let cancelled = crate::workspace::Cancellation::default();
        drop(CompilerCancellation::new(cancelled.clone()));
        assert!(cancelled.check().is_err());
        let success = crate::workspace::Cancellation::default();
        let mut guard = CompilerCancellation::new(success.clone());
        guard.disarm();
        drop(guard);
        assert!(success.check().is_ok());
    }

    #[tokio::test]
    async fn coordinator_wait_polls_gate_holder_and_fifo_waiter() {
        for queued in [false, true] {
            let budget = resources::ResourceBudget::fixed(4096).unwrap();
            let cancellation = crate::workspace::Cancellation::default();
            let gate = Arc::new(tokio::sync::Mutex::new(()));
            let held = gate.clone().lock_owned().await;
            let (held_by_producer, queued_behind) = if queued {
                (None, Some(held))
            } else {
                (Some(held), None)
            };
            let dropped = Arc::new(AtomicUsize::new(0));
            let preparation = OwnedPreparation {
                _charge: budget.reserve("gate producer preparation", 256).unwrap(),
                dropped: dropped.clone(),
            };
            let producer_gate = gate.clone();
            let (release, released) = tokio::sync::oneshot::channel();
            let mut pending = FuturesUnordered::new();
            pending.push(
                async move {
                    let held = match held_by_producer {
                        Some(held) => held,
                        None => producer_gate.lock_owned().await,
                    };
                    released.await.map_err(ModelError::codec)?;
                    drop(held);
                    Ok(preparation)
                }
                .boxed(),
            );
            let mut ready = ReadyStages::new(1, &budget).unwrap();
            let queue_bytes = budget.reserved() - 256;
            assert!(futures::poll!(pending.next()).is_pending());
            let mut coordinating = Box::pin(coordinate_with_pending(
                async {
                    let _gate = gate.lock().await;
                    Ok(())
                },
                &cancellation,
                &mut pending,
                &mut ready,
            ));
            assert!(futures::poll!(&mut coordinating).is_pending());
            drop(queued_behind);
            release.send(()).unwrap();
            coordinating.await.unwrap();
            assert!(pending.is_empty());
            assert_eq!(
                ready.rows.len(),
                1,
                "terminal producer remains unaccepted during boundary coordination"
            );
            assert_eq!(dropped.load(Ordering::SeqCst), 0);
            assert_eq!(
                budget.reserved(),
                queue_bytes + 256,
                "queued preparation keeps its actual reservation"
            );
            let accepted = ready.rows.pop_front().unwrap();
            drop(accepted);
            assert_eq!(dropped.load(Ordering::SeqCst), 1);
            assert_eq!(
                budget.reserved(),
                queue_bytes,
                "allocated queue remains charged when empty"
            );
            drop(ready);
            assert_eq!(budget.reserved(), 0);
        }
    }

    #[tokio::test]
    async fn coordinator_refusal_finishes_action_and_drains_owned_preparations() {
        let budget = resources::ResourceBudget::fixed(16384).unwrap();
        let cancellation = crate::workspace::Cancellation::default();
        let dropped = Arc::new(AtomicUsize::new(0));
        let owned = || OwnedPreparation {
            _charge: budget.reserve("coordinator preparation", 256).unwrap(),
            dropped: dropped.clone(),
        };
        let mut ready = ReadyStages::new(5, &budget).unwrap();
        let queue_bytes = budget.reserved();
        let mut pending: FuturesUnordered<BoxFuture<'_, Result<OwnedPreparation, ModelError>>> =
            FuturesUnordered::new();
        let first = owned();
        pending.push(async move { Ok(first) }.boxed());
        let (refuse, refusal) = tokio::sync::oneshot::channel();
        pending.push(
            async move {
                refusal.await.map_err(ModelError::codec)?;
                Err(ModelError::Conflict("first sibling refusal"))
            }
            .boxed(),
        );
        let token = cancellation.clone();
        let later = owned();
        pending.push(
            async move {
                token.cancelled().await;
                Ok(later)
            }
            .boxed(),
        );
        let token = cancellation.clone();
        let (secondary, secondary_ready) = tokio::sync::oneshot::channel();
        pending.push(
            async move {
                token.cancelled().await;
                secondary_ready.await.map_err(ModelError::codec)?;
                Err(ModelError::Conflict("second sibling refusal"))
            }
            .boxed(),
        );
        let token = cancellation.clone();
        let tail = owned();
        let (drain, drained) = tokio::sync::oneshot::channel();
        pending.push(
            async move {
                token.cancelled().await;
                drained.await.map_err(ModelError::codec)?;
                drop(tail);
                Err(ModelError::Conflict("drained sibling refusal"))
            }
            .boxed(),
        );
        let action_terminal = Arc::new(AtomicUsize::new(0));
        let observed = action_terminal.clone();
        let token = cancellation.clone();
        let (finish_action, action_finished) = tokio::sync::oneshot::channel();
        let mut coordinating = Box::pin(coordinate_with_pending(
            async move {
                token.cancelled().await;
                action_finished.await.map_err(ModelError::codec)?;
                observed.store(1, Ordering::SeqCst);
                Err::<(), _>(ModelError::Conflict("coordinator action refusal"))
            },
            &cancellation,
            &mut pending,
            &mut ready,
        ));
        assert!(futures::poll!(&mut coordinating).is_pending());
        refuse.send(()).unwrap();
        assert!(futures::poll!(&mut coordinating).is_pending());
        assert!(cancellation.check().is_err());
        assert_eq!(
            dropped.load(Ordering::SeqCst),
            1,
            "success observed after refusal is never accepted or buffered"
        );
        assert_eq!(budget.reserved(), queue_bytes + 512);
        secondary.send(()).unwrap();
        assert!(futures::poll!(&mut coordinating).is_pending());
        finish_action.send(()).unwrap();
        let primary = coordinating.await.unwrap_err();
        assert_eq!(
            action_terminal.load(Ordering::SeqCst),
            1,
            "accepted coordinator work reaches terminal before returning refusal"
        );
        assert!(matches!(
            primary.primary(),
            Some(ModelError::Conflict("first sibling refusal"))
        ));
        let mut settlement = Box::pin(settle_upper_pending(
            &cancellation,
            &mut pending,
            &mut ready,
            primary,
        ));
        assert!(futures::poll!(&mut settlement).is_pending());
        assert_eq!(
            dropped.load(Ordering::SeqCst),
            2,
            "buffered private preparation drops on failure"
        );
        assert_eq!(
            budget.reserved(),
            queue_bytes + 256,
            "drain retains the unfinished sibling's charge"
        );
        drain.send(()).unwrap();
        let error = settlement.await.unwrap_err();
        assert!(pending.is_empty() && ready.rows.is_empty());
        assert_eq!(dropped.load(Ordering::SeqCst), 3);
        assert_eq!(budget.reserved(), queue_bytes);
        assert!(matches!(
            error.primary(),
            Some(ModelError::Conflict("first sibling refusal"))
        ));
        for cause in [
            "second sibling refusal",
            "coordinator action refusal",
            "drained sibling refusal",
        ] {
            assert!(
                error.to_string().contains(cause),
                "secondary refusal lost: {cause}: {error}"
            );
        }
        drop(ready);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn upper_failure_cancels_and_drains_started_tasks_before_releasing_owned_state() {
        let cancellation = crate::workspace::Cancellation::default();
        let budget = resources::ResourceBudget::fixed(4096).unwrap();
        let started = Arc::new(AtomicUsize::new(0));
        let cleaned = Arc::new(AtomicUsize::new(0));
        let mut pending = FuturesUnordered::new();
        let mut release_cleanup = Vec::new();
        let mut owned_bytes = 0;
        for ordinal in 0..3 {
            let token = cancellation.clone();
            let started = started.clone();
            let cleaned = cleaned.clone();
            let mut owned = charged::StateCharge::new(&budget, "upper-owned-task-control");
            owned.grow(256).unwrap();
            owned_bytes += owned.accounted_bytes();
            let (release, cleanup) = tokio::sync::oneshot::channel();
            release_cleanup.push(release);
            pending.push(
                async move {
                    started.fetch_add(1, Ordering::SeqCst);
                    token.cancelled().await;
                    cleanup
                        .await
                        .map_err(|_| ModelError::Conflict("cleanup control lost"))?;
                    cleaned.fetch_add(1, Ordering::SeqCst);
                    drop(owned);
                    if ordinal == 1 {
                        Err(ModelError::Conflict("sibling cleanup failed"))
                    } else {
                        Ok(())
                    }
                }
                .boxed(),
            );
        }
        assert!(futures::poll!(pending.next()).is_pending());
        assert_eq!(started.load(Ordering::SeqCst), 3);
        let mut ready = ReadyStages::<()>::new(3, &budget).unwrap();
        let mut settlement = Box::pin(settle_upper_pending(
            &cancellation,
            &mut pending,
            &mut ready,
            ModelError::Conflict("original upper setup failed"),
        ));
        assert!(futures::poll!(&mut settlement).is_pending());
        assert!(cancellation.check().is_err());
        assert_eq!(cleaned.load(Ordering::SeqCst), 0);
        assert_eq!(
            budget.reserved(),
            owned_bytes,
            "cancellation has not discarded still-owned tasks"
        );
        for release in release_cleanup {
            release.send(()).unwrap();
        }
        let error = settlement.await.unwrap_err();
        assert!(pending.is_empty());
        assert_eq!(cleaned.load(Ordering::SeqCst), 3);
        assert_eq!(budget.reserved(), 0);
        let ModelError::Completion(outcome) = error else {
            panic!("sibling failure must retain completion details");
        };
        assert!(matches!(
            outcome.primary.as_deref(),
            Some(ModelError::Conflict("original upper setup failed"))
        ));
        assert_eq!(outcome.completion.failures.len(), 1);
        assert!(matches!(
            outcome.completion.failures[0].error,
            ModelError::Conflict("sibling cleanup failed")
        ));
    }
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    use crate::{artifact, workspace::WorkspaceOptions};
    use lctx_model::domain as d;

    struct RawSourceSpanProvider {
        provider: Box<dyn ProviderStage<ProducerOutput>>,
        evidence: d::assertion::Evidence,
    }
    impl cpg_extract::bundle::Declared for RawSourceSpanProvider {
        fn declaration(&self, profile: Profile) -> Stage {
            let mut stage = self.provider.declaration(profile);
            let mut code = KeySink::new("raw-source-span-fixture/v1");
            code.part(b"delegate", &stage.code.0);
            code.part(b"fixture", include_str!("compilation.rs").as_bytes());
            stage.code = code.finish();
            stage
        }
    }
    impl ProviderStage<ProducerOutput> for RawSourceSpanProvider {
        fn run(
            &mut self,
            context: &mut cpg_extract::bundle::StageContext<ProducerOutput>,
        ) -> Result<ProviderOutcome, ModelError> {
            let outcome = self.provider.run(context)?;
            let d::assertion::Evidence::SourceSpan { source, .. } = &self.evidence else {
                return Err(ModelError::Invalid(
                    "fixture requires an exact source span".into(),
                ));
            };
            let mut found = false;
            for batch in context.input::<d::source::SourceArtifact>()? {
                found |= batch?.rows().iter().any(|row| row.id() == *source);
            }
            if !found {
                return Err(ModelError::Invalid(
                    "fixture evidence source is not acquired".into(),
                ));
            }
            context.contribute(self.evidence.clone())?;
            Ok(outcome)
        }
    }

    fn transported<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Vec<T> {
        use datafusion::arrow::{array::BinaryArray, ipc::reader::FileReader};
        FileReader::try_new(std::fs::File::open(path).unwrap(), None)
            .unwrap()
            .flat_map(|batch| {
                let batch = batch.unwrap();
                let payload = batch
                    .column_by_name("payload")
                    .unwrap()
                    .as_any()
                    .downcast_ref::<BinaryArray>()
                    .unwrap();
                (0..batch.num_rows())
                    .map(|row| serde_json::from_slice(payload.value(row)).unwrap())
                    .collect::<Vec<T>>()
            })
            .collect()
    }
    async fn completed_rows<R: lctx_model::domain::Record>(workspace: &Workspace) -> Vec<R> {
        let mut batches = workspace
            .completed::<R>()
            .unwrap()
            .batches_async()
            .await
            .unwrap();
        let mut rows = Vec::new();
        while let Some(batch) = batches.next_async().await {
            rows.extend(R::decode(&batch.unwrap()).unwrap());
        }
        rows
    }
    #[tokio::test]
    async fn raw_original_bytes_and_half_open_span_survive_artifact_transport() {
        use cpg_extract::{
            acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput,
        };
        use lctx_model::domain::{
            self as d, Record,
            graph::{Entity, EntityId},
        };
        let workspace = Workspace::new(
            Arc::new(d::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 1 << 30,
                ..Default::default()
            },
            crate::test_native::store_with_limit(1 << 30),
        )
        .unwrap();
        let input = tempfile::tempdir().unwrap();
        let raw = b"a\x00\xff\xfez\r\n";
        std::fs::write(input.path().join("original.bin"), raw).unwrap();
        std::fs::write(input.path().join("api.py"), b"def f(x): return x\n").unwrap();
        let captured = Arc::new(CapturedInputs::new(
            vec![AcquiredInput::tree(
                CapturedInput::capture(
                    input.path(),
                    &["api.py".into(), "original.bin".into()],
                    workspace.budget(),
                )
                .unwrap(),
                "raw-artifact",
            )],
            cpg_extract::native_context::NativeContextConfig::committed(
                Profile::Catalog,
                workspace.budget(),
            )
            .unwrap(),
        ));
        let configuration = ContentHash::of(b"raw-original-artifact");
        let source = captured.inputs()[0]
            .captured()
            .artifacts()
            .iter()
            .find(|row| row.path == "original.bin")
            .unwrap()
            .clone();
        let evidence = d::assertion::Evidence::SourceSpan {
            source: source.id(),
            start: 1,
            end: 4,
        };
        let mut providers = facts::providers(configuration);
        let index = providers
            .iter()
            .position(|provider| {
                provider.declaration(Profile::Catalog).name == cpg_extract::deployment::DEPLOYMENT
            })
            .unwrap();
        let provider = providers.remove(index);
        providers.insert(
            index,
            Box::new(RawSourceSpanProvider {
                provider,
                evidence: evidence.clone(),
            }),
        );
        let schedule = Schedule::build(
            workspace.model(),
            providers
                .iter()
                .map(|provider| provider.declaration(Profile::Catalog))
                .collect(),
            &[],
            Profile::Catalog,
        )
        .unwrap();
        let position = |name| {
            schedule
                .stages()
                .iter()
                .position(|stage| stage.name == name)
                .unwrap()
        };
        assert!(
            position(cpg_extract::acquisition::ACQUIRE)
                < position(cpg_extract::deployment::DEPLOYMENT)
        );
        assert!(
            position(cpg_extract::deployment::DEPLOYMENT)
                < position(cpg_extract::assembly::ASSEMBLE)
        );
        let authored = schedule
            .stages()
            .iter()
            .find(|stage| stage.name == cpg_extract::deployment::DEPLOYMENT)
            .unwrap();
        assert!(authored.reads::<d::source::SourceArtifact>());
        assert!(authored.contributes_to::<d::assertion::Evidence>());
        compile_driver(
            &workspace,
            captured.clone(),
            Profile::Catalog,
            configuration,
            Frontier::Facts,
            None,
            None,
            None,
            Some(providers),
        )
        .boxed()
        .await
        .unwrap();
        let admitted = artifact::admit(
            &workspace,
            &captured,
            Frontier::Facts,
            Profile::Catalog,
            configuration,
        )
        .await
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("raw-graph");
        admitted.export(&destination).await.unwrap();
        admitted
            .verify_export(&destination, &workspace)
            .await
            .unwrap();
        assert!(
            completed_rows::<d::source::SourceArtifact>(&workspace)
                .await
                .contains(&source)
        );
        let source_id = EntityId::of(source.id());
        let originals =
            std::fs::read(destination.join(format!("original-{}.bin", source_id.0.hex()))).unwrap();
        assert_eq!(originals, raw);
        assert!(std::str::from_utf8(&originals).is_err());
        let entities: Vec<Entity> = transported(&destination.join("entities.arrow"));
        assert!(
            entities
                .iter()
                .any(|entity| matches!(entity,Entity::Source(row) if row==&source))
        );
        let restored = entities
            .into_iter()
            .find(|entity| entity == &Entity::from(evidence.clone()))
            .expect("admitted artifact lost its source-span evidence");
        if let Entity::Evidence(d::assertion::Evidence::SourceSpan { source, start, end }) =
            restored
        {
            assert_eq!(EntityId::of(source), source_id);
            assert_eq!(&originals[start as usize..end as usize], b"\x00\xff\xfe");
        } else {
            panic!("source evidence lost its typed span");
        }
    }

    #[test]
    fn value_publication_boundaries_follow_semantic_dependency_order() {
        let groups: Vec<_> = UpperStage::ALL
            .into_iter()
            .filter_map(|stage| stage.boundary())
            .collect();
        let structural = groups
            .iter()
            .position(|boundary| *boundary == PublicationBoundary::Structural)
            .unwrap();
        assert_eq!(
            groups[structural + 1],
            PublicationBoundary::AnalyticEmbedding
        );
        assert_eq!(groups[structural + 2], PublicationBoundary::Analytic);
        assert_eq!(groups.last(), Some(&PublicationBoundary::Retrieval));
        assert!(
            groups
                .iter()
                .position(|boundary| *boundary == PublicationBoundary::Local)
                .unwrap()
                < structural
        );
    }
}
