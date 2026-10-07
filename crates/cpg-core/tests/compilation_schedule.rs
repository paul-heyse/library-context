//! Complete compiler scheduling over the actual declared model, before native effects.
use cpg_core::{compilation::PreparedCompilation, embedding_service::Embedder, facts};
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
