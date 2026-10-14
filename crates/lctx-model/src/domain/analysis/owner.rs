//! Closed current analysis producers and their actual model-owned declarations.
//! This is declaration meaning, not a capability to read selected physical inputs.
use super::{AnalysisCapability as C, AnalysisDefinition, AnalysisMethod as M};
use crate::domain::{
    completed::{CompletedBinding, CompletedContribution},
    stages::*,
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisOwner {
    Local,
    Base,
    Completion,
    SourceCalls,
    Enriched,
    Models,
    Summary,
    Structural,
    Embedding,
    Analytic,
    CatalogCore,
    CatalogEvidence,
    Selection,
    Synthesis,
    Retrieval,
}
impl AnalysisOwner {
    pub const ALL: [Self; 15] = [
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
        Self::CatalogCore,
        Self::CatalogEvidence,
        Self::Selection,
        Self::Synthesis,
        Self::Retrieval,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Local => "analyze_local",
            Self::Base => "evaluate_base",
            Self::Completion => "complete_base",
            Self::SourceCalls => "prepare_source_calls",
            Self::Enriched => "enrich_execution",
            Self::Models => "apply_models",
            Self::Summary => "analyze_summaries",
            Self::Structural => "analyze_structural",
            Self::Embedding => "analytic_embedding",
            Self::Analytic => "analyze_analytic",
            Self::CatalogCore => "catalog_core",
            Self::CatalogEvidence => "catalog_evidence",
            Self::Selection => "catalog_selection",
            Self::Synthesis => "synthesis",
            Self::Retrieval => "retrieval",
        }
    }
    pub fn resolve(name: &str) -> Result<Self, ModelError> {
        Self::ALL
            .into_iter()
            .find(|owner| owner.name() == name)
            .ok_or(ModelError::Conflict("unknown current analysis owner"))
    }
    pub fn for_method(method: M) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|owner| owner.methods().iter().any(|(actual, _)| *actual == method))
    }
    pub(crate) fn for_family(name: &str) -> Option<Self> {
        Some(match name {
            "local" => Self::Local,
            "base_evaluation" => Self::Base,
            "base_completion" => Self::Completion,
            "source_call" => Self::SourceCalls,
            "enriched_execution" => Self::Enriched,
            "model" => Self::Models,
            "summary" => Self::Summary,
            "structural" => Self::Structural,
            "analytic_embedding" => Self::Embedding,
            "analytic" => Self::Analytic,
            "catalog_core" => Self::CatalogCore,
            "catalog_evidence" => Self::CatalogEvidence,
            "selection" => Self::Selection,
            "synthesis" => Self::Synthesis,
            "retrieval" => Self::Retrieval,
            _ => return None,
        })
    }
    pub fn methods(self) -> &'static [(M, C)] {
        match self {
            Self::Local => &[(M::LocalTransfers, C::Transfers)],
            Self::Base => &[(M::Execution, C::Execution)],
            Self::Completion => &[(M::Completion, C::Completion)],
            Self::SourceCalls => &[(M::SourceCalls, C::Execution)],
            Self::Enriched => &[(M::EnrichedExecution, C::Execution)],
            Self::Models => &[(M::Models, C::Models)],
            Self::Summary => &[(M::Summaries, C::Summaries)],
            Self::Structural => &[
                (M::Delegation, C::Delegation),
                (M::DirectUsage, C::DirectUsage),
                (M::Handoffs, C::Handoffs),
                (M::Controls, C::Controls),
            ],
            Self::Embedding => &[(M::AnalyticEmbedding, C::AnalyticEmbedding)],
            Self::Analytic => &[
                (M::PageRank, C::PageRank),
                (M::Communities, C::Communities),
                (M::Concepts, C::Concepts),
                (M::RelationalConcepts, C::RelationalConcepts),
                (M::Neighbours, C::Neighbours),
            ],
            Self::CatalogCore => &[(M::Catalog, C::Catalog)],
            Self::CatalogEvidence => &[(M::CatalogEvidence, C::CatalogEvidence)],
            Self::Selection => &[(M::CatalogSelection, C::CatalogSelection)],
            Self::Synthesis => &[(M::Synthesis, C::Synthesis)],
            Self::Retrieval => &[(M::Retrieval, C::Retrieval)],
        }
    }
    pub fn boundary(self) -> Option<PublicationBoundary> {
        match self {
            Self::Local => Some(PublicationBoundary::Local),
            Self::Models => Some(PublicationBoundary::Model),
            Self::Base => Some(PublicationBoundary::BaseEvaluation),
            Self::Completion => Some(PublicationBoundary::BaseCompletion),
            Self::SourceCalls => Some(PublicationBoundary::SourceCall),
            Self::Enriched => Some(PublicationBoundary::EnrichedExecution),
            Self::Summary => Some(PublicationBoundary::Summary),
            Self::Structural => Some(PublicationBoundary::Structural),
            Self::Embedding => Some(PublicationBoundary::AnalyticEmbedding),
            Self::Analytic => Some(PublicationBoundary::Analytic),
            Self::Synthesis => Some(PublicationBoundary::Synthesis),
            Self::Retrieval => Some(PublicationBoundary::Retrieval),
            _ => None,
        }
    }
    pub fn catalog_only(self) -> bool {
        matches!(self, Self::Selection | Self::Synthesis | Self::Retrieval)
    }
    pub fn can_be_wholly_unrequested(self) -> bool {
        matches!(
            self,
            Self::Local
                | Self::Base
                | Self::Completion
                | Self::SourceCalls
                | Self::Enriched
                | Self::Models
                | Self::Summary
                | Self::Embedding
                | Self::Analytic
        )
    }
    /// These owners author total NotRequested domains in Catalog, independently of
    /// Behavioral result rows. Optional owners retain their own selection contracts.
    pub fn compact_catalog(self, profile: Profile) -> bool {
        profile == Profile::Catalog
            && matches!(
                self,
                Self::Local
                    | Self::Base
                    | Self::Completion
                    | Self::SourceCalls
                    | Self::Enriched
                    | Self::Models
                    | Self::Summary
            )
    }
    fn uniform_checkpoint(self) -> PublicationBoundary {
        match self {
            Self::Local | Self::CatalogCore => PublicationBoundary::Facts,
            Self::Base => PublicationBoundary::Local,
            Self::Completion => PublicationBoundary::BaseEvaluation,
            Self::SourceCalls => PublicationBoundary::BaseCompletion,
            Self::Enriched | Self::CatalogEvidence | Self::Selection => {
                PublicationBoundary::SourceCall
            }
            Self::Models => PublicationBoundary::EnrichedExecution,
            Self::Summary => PublicationBoundary::Model,
            Self::Structural => PublicationBoundary::Summary,
            Self::Embedding => PublicationBoundary::Structural,
            Self::Analytic => PublicationBoundary::AnalyticEmbedding,
            Self::Synthesis => PublicationBoundary::Analytic,
            Self::Retrieval => PublicationBoundary::Synthesis,
        }
    }
    pub fn declaration(
        self,
        profile: Profile,
        selected: &SelectedDefinitions<'_>,
        model: &ValidatedModel,
        order: &PublicationOrder,
    ) -> Result<Stage, ModelError> {
        let definition = || selected.definition(self.methods()[0].0);
        let mut stage = match self {
            Self::Local => local_semantics::stage(profile, definition()?, model, order),
            Self::Base => execution::production::stage(profile, definition()?, model, order),
            Self::Completion => {
                execution::completion_production::stage(profile, definition()?, model, order)
            }
            Self::SourceCalls => {
                execution::source_call::stage(profile, definition()?, model, order)
            }
            Self::Enriched => {
                execution::enriched_production::stage(profile, definition()?, model, order)
            }
            Self::Models => {
                execution::model_production::stage(profile, definition()?, model, order)
            }
            Self::Summary => execution::summary_replay::stage(profile, definition()?, model, order),
            Self::Structural => {
                structural::build::stage(profile, selected.settings()?, model, order)
            }
            Self::Analytic => analytics::build::stage(profile, selected.settings()?, model, order),
            Self::Embedding => {
                embedding::analytic::stage(profile, selected.text()?.requested, model, order)
            }
            Self::CatalogCore => catalog::build::stage(profile, model, order),
            Self::CatalogEvidence => catalog::evidence::build::stage(profile, model, order),
            Self::Selection => selection::build::stage(profile, model, order),
            Self::Synthesis => synthesis::build::stage(profile, selected.settings()?, model, order),
            Self::Retrieval => {
                retrieval::build::stage(profile, selected.retrieval()?, model, order)
            }
        }?;
        if self.can_be_wholly_unrequested() {
            stage
                .outputs
                .push(RelationUse::of::<uniform_outcome::UniformNotRequested>());
            // Domain reconstruction uses actual captured metadata independently of predecessor
            // result rows, including explicit empty Python domains.
            for input in [
                RelationUse::completed::<input::InputRevision>(),
                RelationUse::completed::<input::ArtifactUse>(),
                RelationUse::completed::<source::SourceArtifact>(),
                RelationUse::completed::<source::CoverageScope>(),
                RelationUse::completed::<attribution::ProviderRun>(),
                RelationUse::completed::<AnalysisDefinition>(),
                RelationUse::completed::<super::MethodParameters>(),
            ] {
                if !stage
                    .inputs
                    .iter()
                    .any(|prior| prior.name() == input.name() && prior.prefix() == input.prefix())
                {
                    stage.inputs.push(input);
                }
            }
            if matches!(self, Self::Enriched | Self::Models | Self::Summary)
                && !stage
                    .inputs
                    .iter()
                    .any(|input| input.name() == models::ModelCatalog::NAME)
            {
                stage
                    .inputs
                    .push(RelationUse::completed::<models::ModelCatalog>());
            }
        }
        if self.compact_catalog(profile) {
            // Keep the complete ordinary output inventory, authored definition and
            // producer implementation, but derive inputs from the actual compact
            // domain rather than the Behavioral kernel or preceding result owner.
            stage.inputs = dependency_closure::DependencyClosure::stage_grants(
                model,
                uniform_outcome::AnalysisUniformInputs::domain_inputs(self),
                &stage.outputs,
                PublicationBoundary::Facts,
                dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
                order,
            )?;
        } else {
            // Ordinary kernels still consume the genuine cumulative compact outcomes
            // selected at their semantic checkpoint.
            stage
                .inputs
                .retain(|input| input.name() != uniform_outcome::UniformNotRequested::NAME);
            stage.inputs.push(
                RelationUse::completed::<uniform_outcome::UniformNotRequested>()
                    .at_epoch(self.uniform_checkpoint()),
            );
        }
        let mut code = KeySink::new("current analysis owner production/v2");
        stage.code.encode(&mut code);
        ContentHash::of(include_bytes!("owner.rs")).encode(&mut code);
        stage.code = code.finish();
        Ok(stage)
    }
}

/// Actual selected typed rows. Callers must retain their protected selected view capabilities
/// and independently establish counts/EOF; supplying these rows grants no storage authority.
pub struct SelectedDefinitions<'a> {
    pub definitions: &'a normalized::Rows<AnalysisDefinition>,
    pub analytics: Option<&'a super::settings::AnalyticsConfiguration>,
    pub text: Option<&'a embedding::text::TextDefinition>,
    pub retrieval: Option<&'a retrieval::RetrievalDefinition>,
}
impl SelectedDefinitions<'_> {
    pub fn definition(&self, method: M) -> Result<&AnalysisDefinition, ModelError> {
        let mut rows = self.definitions.iter().filter(|row| row.method == method);
        let row = rows.next().ok_or(ModelError::Conflict(
            "analysis owner selected definition absent",
        ))?;
        if rows.next().is_some() {
            return Err(ModelError::Conflict(
                "analysis owner selected definition ambiguous",
            ));
        }
        row.validate()?;
        Ok(row)
    }
    fn settings(&self) -> Result<&super::settings::AnalyticsConfiguration, ModelError> {
        let row = self
            .analytics
            .ok_or(ModelError::Conflict("analysis selected settings absent"))?;
        row.validate()?;
        Ok(row)
    }
    fn text(&self) -> Result<&embedding::text::TextDefinition, ModelError> {
        let row = self.text.ok_or(ModelError::Conflict(
            "analysis selected text definition absent",
        ))?;
        row.validate()?;
        Ok(row)
    }
    fn retrieval(&self) -> Result<&retrieval::RetrievalDefinition, ModelError> {
        let row = self.retrieval.ok_or(ModelError::Conflict(
            "analysis selected retrieval definition absent",
        ))?;
        row.validate()?;
        Ok(row)
    }
}

/// The production upper inventory's fixed boundary order, shared by declaration preparation
/// and lower reconstruction. Facts suppliers are supplied by the actual provider schedule.
pub fn publication_groups(
    facts: Vec<&'static str>,
    include_catalog: bool,
) -> Vec<PublicationGroup> {
    let mut groups = vec![PublicationGroup::new(PublicationBoundary::Facts, facts)];
    for owner in AnalysisOwner::ALL {
        if owner.catalog_only() && !include_catalog {
            continue;
        }
        if let Some(boundary) = owner.boundary() {
            groups.push(PublicationGroup::new(boundary, vec![owner.name()]));
        }
    }
    groups
}

/// Check every ordered role against actual protected completed bindings. Equal view hashes
/// do not excuse a missing epoch; unspecified vocabulary roles use the real first boundary.
pub fn validate_selected_roles(
    stage: &Stage,
    owner: &CompletedContribution,
    bindings: &[CompletedBinding],
    model: &ValidatedModel,
) -> Result<(), ModelError> {
    if owner.spec.model != model.digest()
        || owner.spec.producer != stage.name
        || owner.spec.configuration != Some(stage.configuration)
        || owner.spec.captured_binding.is_some()
        || owner.spec.inputs.len() != stage.inputs.len()
        || owner.spec.outputs
            != stage
                .outputs
                .iter()
                .chain(&stage.contributes)
                .map(|input| input.name().to_owned())
                .collect()
        || owner.outputs.keys().ne(owner.spec.outputs.iter())
    {
        return Err(ModelError::Conflict(
            "analysis owner actual recipe inventory differs",
        ));
    }
    let order = PublicationOrder::planning(&publication_groups(Vec::new(), true))?;
    for (role, snapshot) in stage.inputs.iter().zip(&owner.spec.inputs) {
        if role.name() != snapshot.relation() || snapshot.model() != model.digest() {
            return Err(ModelError::Conflict(
                "analysis owner ordered input role differs",
            ));
        }
        let prefix = if role.prefix().is_none() && is_epoch_shared(role.name()) {
            let mut first = None;
            for binding in bindings
                .iter()
                .filter(|binding| binding.source.relation() == role.name())
            {
                let Some(name) = binding.boundary.as_deref() else {
                    continue;
                };
                let boundary = PublicationBoundary::from_name(name).ok_or(ModelError::Conflict(
                    "analysis owner unknown selected boundary",
                ))?;
                let ordinal = order.resolve(boundary)?.ordinal();
                if first.is_none_or(|(prior, _)| ordinal < prior) {
                    first = Some((ordinal, boundary));
                }
            }
            Some(
                first
                    .ok_or(ModelError::Conflict(
                        "analysis owner shared input has no real boundary",
                    ))?
                    .1,
            )
        } else {
            role.prefix()
        };
        let mut matching = bindings.iter().filter(|binding| {
            binding.source == *snapshot
                && match prefix {
                    Some(prefix) => binding.boundary.as_deref() == Some(prefix.name()),
                    None => binding.boundary.is_none(),
                }
        });
        let binding = matching.next().ok_or(ModelError::Conflict(
            "analysis owner selected input role absent",
        ))?;
        binding.validate()?;
        // With no declared boundary, copies of one exact immutable snapshot are harmless;
        // distinct sources cannot match. Explicit boundaries must have one actual binding.
        for duplicate in matching {
            duplicate.validate()?;
            if prefix.is_some() || duplicate.view != binding.view {
                return Err(ModelError::Conflict(
                    "analysis owner selected input role ambiguous",
                ));
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod controls {
    use super::*;
    #[test]
    fn compact_catalog_inputs_retain_domain_and_config_without_execution_predecessors() {
        let budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
        let model = crate::domain::model().unwrap();
        let catalog = models::Catalog::committed().unwrap();
        let mut definitions = normalized::Rows::new(&budget);
        for (_, definition) in [
            local_semantics::definition(),
            execution::configuration::base_evaluation(),
            execution::configuration::base_completion(),
            execution::configuration::source_calls(),
            execution::configuration::enriched_execution(catalog.declaration().id()),
            execution::configuration::models(catalog.declaration().id()),
            execution::configuration::summaries(catalog.declaration().id(), Default::default())
                .unwrap(),
        ] {
            definitions.insert(definition).unwrap();
        }
        let selected = SelectedDefinitions {
            definitions: &definitions,
            analytics: None,
            text: None,
            retrieval: None,
        };
        let order = PublicationOrder::planning(&publication_groups(Vec::new(), true)).unwrap();
        let owners = [
            AnalysisOwner::Local,
            AnalysisOwner::Base,
            AnalysisOwner::Completion,
            AnalysisOwner::SourceCalls,
            AnalysisOwner::Enriched,
            AnalysisOwner::Models,
            AnalysisOwner::Summary,
        ];
        let catalog_stages = owners.map(|owner| {
            owner
                .declaration(Profile::Catalog, &selected, &model, &order)
                .unwrap()
        });
        let ordinary_outputs = catalog_stages
            .iter()
            .flat_map(|stage| &stage.outputs)
            .filter(|role| role.name() != uniform_outcome::UniformNotRequested::NAME)
            .map(|role| role.name())
            .collect::<std::collections::BTreeSet<_>>();
        for (owner, stage) in owners.into_iter().zip(catalog_stages.iter()) {
            for role in &stage.inputs {
                assert!(
                    !ordinary_outputs.contains(role.name()),
                    "{} depends on Behavioral output {}",
                    stage.name,
                    role.name()
                );
                assert_ne!(role.name(), uniform_outcome::UniformNotRequested::NAME);
                assert!(
                    role.prefix()
                        .is_none_or(|prefix| prefix == PublicationBoundary::Facts)
                );
            }
            // Domain membership needs admitted captured metadata, not original byte chunks.
            // Original completeness stays with the retained normalization/final admission gates.
            for expected in [
                input::InputRevision::NAME,
                input::ArtifactUse::NAME,
                source::SourceArtifact::NAME,
                source::CoverageScope::NAME,
                attribution::ProviderRun::NAME,
                attribution::AnalysisContext::NAME,
                AnalysisDefinition::NAME,
                super::super::MethodParameters::NAME,
            ] {
                assert!(
                    stage.inputs.iter().any(|role| role.name() == expected),
                    "{} omitted {expected}",
                    stage.name
                );
            }
            let behavioral = owner
                .declaration(Profile::Behavioral, &selected, &model, &order)
                .unwrap();
            assert_eq!(stage.configuration, behavioral.configuration);
            assert_eq!(
                stage
                    .outputs
                    .iter()
                    .map(|role| role.name())
                    .collect::<Vec<_>>(),
                behavioral
                    .outputs
                    .iter()
                    .map(|role| role.name())
                    .collect::<Vec<_>>()
            );
            assert!(behavioral.inputs.iter().any(|role| role.name()
                == uniform_outcome::UniformNotRequested::NAME
                && role.prefix() == Some(owner.uniform_checkpoint())));
            if matches!(
                owner,
                AnalysisOwner::Enriched | AnalysisOwner::Models | AnalysisOwner::Summary
            ) {
                assert!(
                    stage
                        .inputs
                        .iter()
                        .any(|role| role.name() == models::ModelCatalog::NAME)
                );
            }
        }
    }
    #[test]
    fn compact_owner_boundaries_follow_real_dependencies_and_keep_stable_codes() {
        let groups = publication_groups(vec![producer_contract::facts::ASSEMBLE], true);
        let expected = [
            PublicationBoundary::Facts,
            PublicationBoundary::Local,
            PublicationBoundary::BaseEvaluation,
            PublicationBoundary::BaseCompletion,
            PublicationBoundary::SourceCall,
            PublicationBoundary::EnrichedExecution,
            PublicationBoundary::Model,
            PublicationBoundary::Summary,
            PublicationBoundary::Structural,
            PublicationBoundary::AnalyticEmbedding,
            PublicationBoundary::Analytic,
            PublicationBoundary::Synthesis,
            PublicationBoundary::Retrieval,
        ];
        assert_eq!(
            groups.iter().map(|group| group.epoch).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(PublicationBoundary::BaseEvaluation.code(), 7);
        assert_eq!(PublicationBoundary::BaseCompletion.code(), 8);
        assert_eq!(PublicationBoundary::SourceCall.code(), 9);
        assert_eq!(PublicationBoundary::EnrichedExecution.code(), 10);
        let names = AnalysisOwner::ALL
            .into_iter()
            .filter(|owner| owner.can_be_wholly_unrequested())
            .map(AnalysisOwner::name)
            .collect::<Vec<_>>();
        assert_eq!(names.len(), 9);
        assert!(!AnalysisOwner::Structural.can_be_wholly_unrequested());
        assert!(stages::is_epoch_shared(
            uniform_outcome::UniformNotRequested::NAME
        ));
        assert!(!stages::is_vocabulary(
            uniform_outcome::UniformNotRequested::NAME
        ));
    }
}
