//! Complete compiler scheduling over the actual declared model, before native effects.
use cpg_core::{compilation::PreparedCompilation, facts};
use lctx_model::domain::embedding::Embedder;
use lctx_model::domain::{
    admission::Frontier, analysis, analysis::settings::AnalyticsConfiguration, stages::*, *,
};

#[test]
fn prepared_schedules_bind_canonical_values_to_completed_producer_epochs() {
    use analysis::settings::Techniques;
    use cpg_core::embedding_service::FakeEmbedder;
    let model = model().unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 27).unwrap();
    let catalog = cpg_extract::native_context::CatalogSelection::committed(&budget).unwrap();
    let fake = FakeEmbedder::new();
    let providers = facts::providers(ContentHash::of(b"canonical-schedule-control"));
    for profile in Profile::ALL {
        for (selected, knn) in [(false, false), (true, false), (true, true)] {
            let mut settings = AnalyticsConfiguration::parse(
                include_str!("../../../libraries/fastmcp/analytics.toml"),
                Techniques::default(),
            )
            .unwrap();
            settings.knn = knn;
            let embedder = selected.then_some(&fake as &dyn Embedder);
            let prepared = PreparedCompilation::new(
                Frontier::Catalog,
                settings,
                catalog.catalog(),
                embedder,
                &budget,
            )
            .unwrap();
            assert_eq!(
                prepared
                    .configuration()
                    .retrieval()
                    .iter()
                    .next()
                    .unwrap()
                    .embedding_requested,
                selected
            );
            let schedule = prepared.schedule(&model, &providers, profile).unwrap();
            for (name, checkpoint) in [
                ("analyze_local", PublicationBoundary::Facts),
                ("evaluate_base", PublicationBoundary::Local),
                ("complete_base", PublicationBoundary::BaseEvaluation),
                ("prepare_source_calls", PublicationBoundary::BaseCompletion),
                ("enrich_execution", PublicationBoundary::SourceCall),
                ("apply_models", PublicationBoundary::EnrichedExecution),
                ("analyze_summaries", PublicationBoundary::Model),
                ("analyze_structural", PublicationBoundary::Summary),
                ("analytic_embedding", PublicationBoundary::Structural),
                ("analyze_analytic", PublicationBoundary::AnalyticEmbedding),
                ("catalog_evidence", PublicationBoundary::SourceCall),
                ("catalog_selection", PublicationBoundary::SourceCall),
                ("synthesis", PublicationBoundary::Analytic),
                ("retrieval", PublicationBoundary::Synthesis),
            ] {
                let stage = schedule
                    .stages()
                    .iter()
                    .find(|stage| stage.name == name)
                    .unwrap();
                let uniform: Vec<_> = stage
                    .inputs
                    .iter()
                    .filter(|input| input.name() == uniform_outcome::UniformNotRequested::NAME)
                    .collect();
                let owner = analysis::owner::AnalysisOwner::resolve(name).unwrap();
                if owner.compact_catalog(profile) {
                    assert!(
                        uniform.is_empty(),
                        "{name} authors its Catalog domain independently of compact predecessors"
                    );
                } else {
                    assert_eq!(
                        uniform.len(),
                        1,
                        "{name} must select one cumulative compact predecessor inventory"
                    );
                    assert_eq!(
                        uniform[0].prefix(),
                        Some(checkpoint),
                        "{name} must select its real predecessor boundary"
                    );
                }
                assert_eq!(
                    stage
                        .outputs
                        .iter()
                        .any(|output| output.name() == uniform_outcome::UniformNotRequested::NAME),
                    owner.can_be_wholly_unrequested()
                );
                assert!(
                    !stage
                        .contributes
                        .iter()
                        .any(|output| output.name() == uniform_outcome::UniformNotRequested::NAME)
                );
            }
            if profile == Profile::Catalog {
                let local = schedule.prerequisites("analyze_local").unwrap();
                let base = schedule.prerequisites("evaluate_base").unwrap();
                assert!(!base.contains("analyze_local"));
                assert!(!local.contains("evaluate_base"));
                // This is the actual normalized barrier plus authored configuration,
                // with no compact upper owner marked complete.
                let mut complete = providers
                    .iter()
                    .map(|provider| provider.declaration(profile).name)
                    .collect::<std::collections::BTreeSet<_>>();
                complete.extend(
                    [
                        normalized::entity_normalization::stage(),
                        normalized::relation_normalization::stage(profile),
                        normalized::callable_normalization::stage(profile),
                        normalized::receiver::stage(profile),
                        normalized::callable_aspects::stage(profile),
                        normalized::event_normalization::stage(profile),
                        normalized::binding_normalization::stage(profile),
                        projection::normalization::stage(profile),
                        normalized::coverage::stage(profile),
                    ]
                    .into_iter()
                    .map(|stage| stage.name),
                );
                complete.extend(
                    prepared
                        .configuration()
                        .declarations()
                        .iter()
                        .map(|stage| stage.name),
                );
                assert!(schedule.ready("analyze_local", &complete).unwrap());
                assert!(schedule.ready("evaluate_base", &complete).unwrap());
                let simultaneously_ready = schedule
                    .stages()
                    .iter()
                    .filter(|stage| {
                        !complete.contains(stage.name)
                            && schedule.ready(stage.name, &complete).unwrap()
                    })
                    .map(|stage| stage.name)
                    .collect::<std::collections::BTreeSet<_>>();
                for peer in [
                    "analyze_local",
                    "evaluate_base",
                    "complete_base",
                    "prepare_source_calls",
                    "enrich_execution",
                    "apply_models",
                    "analyze_summaries",
                ] {
                    assert!(
                        simultaneously_ready.contains(peer),
                        "Catalog metadata owner {peer} should be independently ready"
                    );
                }
                // C0 retains its actual owner prerequisites rather than a broad upper-stage barrier.
                let core = schedule.prerequisites("catalog_core").unwrap();
                assert!(!core.contains("evaluate_base"));
                assert!(!core.contains("complete_base"));
                assert!(schedule.ready("catalog_core", core).unwrap());
                let mut missing = core.clone();
                let prerequisite = *missing
                    .first()
                    .expect("C0 has real normalized/configuration inputs");
                missing.remove(prerequisite);
                assert!(!schedule.ready("catalog_core", &missing).unwrap());
            }
            for name in [
                embedding::value::FullValue::NAME,
                embedding::projection::ProjectedValue::NAME,
            ] {
                for stage_name in ["analyze_analytic", "retrieval"] {
                    let stage = schedule
                        .stages()
                        .iter()
                        .find(|s| s.name == stage_name)
                        .unwrap();
                    assert!(stage.inputs.iter().any(|r| r.name() == name
                        && r.prefix() == Some(PublicationBoundary::AnalyticEmbedding)));
                    assert!(
                        stage
                            .inputs
                            .iter()
                            .filter(|r| r.name() == name)
                            .all(|r| r.prefix() == Some(PublicationBoundary::AnalyticEmbedding))
                    );
                }
                for (stage_name, epoch) in [
                    ("analytic_embedding", PublicationBoundary::AnalyticEmbedding),
                    ("retrieval", PublicationBoundary::Retrieval),
                ] {
                    let stage = schedule
                        .stages()
                        .iter()
                        .find(|s| s.name == stage_name)
                        .unwrap();
                    assert!(stage.outputs.iter().any(|r| r.name() == name));
                    assert_eq!(schedule.epoch_for(stage_name), Some(epoch));
                }
            }
            let e1 = schedule
                .stages()
                .iter()
                .find(|s| s.name == "analytic_embedding")
                .unwrap();
            assert_eq!(
                e1.effect,
                if selected && knn {
                    Effect::Embedding
                } else {
                    Effect::Pure
                }
            );
        }
    }
}

#[path = "fixtures/native.rs"]
mod native_fixture;
