//! Static assembled declarations; actual provider/store qualification is separate.
use lctx_model::domain::{analysis, stages::*, *};
#[test]
fn upper_frontiers_have_one_actual_writer_for_every_relation() {
    let model = model().unwrap();
    let b = resources::ResourceBudget::fixed(1 << 28).unwrap();
    let catalog = models::Catalog::committed().unwrap();
    let settings = analysis::settings::AnalyticsConfiguration {
        module_prefixes: vec!["api".into()],
        public_roots: vec!["api".into()],
        configured_seeds: vec![],
        depth: 2,
        vertices: 512,
        arcs: 2048,
        witnesses: 3,
        brief_budget: 0,
        communities: false,
        pagerank: false,
        fca: false,
        rca: false,
        knn: false,
        type_layer: false,
        mention_layer: false,
        knn_layer: false,
    };
    for profile in Profile::ALL {
        for target in [
            analysis::frontier::Target::Analysis,
            analysis::frontier::Target::Catalog,
        ] {
            let mut pairs = vec![
                local_semantics::definition(),
                execution::configuration::base_evaluation(),
                execution::configuration::base_completion(),
                execution::configuration::source_calls(),
                execution::configuration::enriched_execution(catalog.declaration().id()),
                execution::configuration::models(catalog.declaration().id()),
                execution::configuration::summaries(catalog.declaration().id(), Default::default())
                    .unwrap(),
                catalog::build::definition(),
                catalog::evidence::build::definition(),
                embedding::analytic::definition(),
            ];
            for method in structural::build::methods() {
                pairs.push(structural::build::definition(&settings, method).unwrap());
            }
            for method in analytics::build::METHODS {
                pairs.push(analytics::build::definition(&settings, method).unwrap());
            }
            if target == analysis::frontier::Target::Catalog {
                pairs.extend([
                    selection::build::definition(),
                    synthesis::build::definition(),
                    retrieval::build::definition(),
                ]);
            }
            let mut config = analysis::preparation::Configuration::new(&catalog, pairs.clone(), &b)
                .unwrap()
                .with_analytics(settings.clone())
                .unwrap();
            if target == analysis::frontier::Target::Catalog {
                config = config
                    .with_retrieval(retrieval::RetrievalDefinition::builtin(false))
                    .unwrap();
            }
            let facts = Stage {
                name: "static_facts",
                inputs: vec![],
                outputs: facts_relations()
                    .iter()
                    .map(RelationUse::of_relation)
                    .collect(),
                contributes: vec![],
                coverage: vec![],
                profiles: vec![profile],
                effect: Effect::Pure,
                code: ContentHash::of(b"static"),
                configuration: ContentHash::of(b"static"),
            };
            let mut stages = vec![
                facts,
                normalized::entity_normalization::stage(),
                normalized::relation_normalization::stage(profile),
                normalized::callable_normalization::stage(profile),
                normalized::callable_aspects::stage(profile),
                normalized::receiver::stage(profile),
                normalized::event_normalization::stage(profile),
                normalized::binding_normalization::stage(profile),
                projection::normalization::stage(profile),
                normalized::coverage::stage(profile),
                config.declaration(),
                analysis::preparation::native_stage(profile, &model, &fixture_publication_order()).unwrap(),
                local_semantics::stage(profile, &pairs[0].1, &model, &fixture_publication_order()).unwrap(),
                execution::production::stage(profile, &pairs[1].1, &model, &fixture_publication_order()).unwrap(),
                execution::completion_production::stage(profile, &pairs[2].1, &model, &fixture_publication_order()).unwrap(),
                execution::source_call::stage(profile, &pairs[3].1, &model, &fixture_publication_order()).unwrap(),
                execution::enriched_production::stage(profile, &pairs[4].1, &model, &fixture_publication_order()).unwrap(),
                execution::model_production::stage(profile, &pairs[5].1, &model, &fixture_publication_order()).unwrap(),
                execution::summary_replay::stage(profile, &pairs[6].1, &model, &fixture_publication_order()).unwrap(),
                catalog::build::stage(profile, &model, &fixture_publication_order()).unwrap(),
                catalog::evidence::build::stage(profile, &model, &fixture_publication_order())
                    .unwrap(),
                embedding::configuration::stage(None),
                embedding::text::stage(profile, &embedding::text::TextDefinition::builtin(), &model, &fixture_publication_order())
                    .unwrap(),
                embedding::analytic::stage(profile, false, &model, &fixture_publication_order()).unwrap(),
                structural::build::stage(profile, &settings, &model, &fixture_publication_order()).unwrap(),
                analytics::build::stage(profile, &settings, &model, &fixture_publication_order())
                    .unwrap(),
                analysis::frontier::stage(profile, analysis::frontier::Target::Analysis, &model, &fixture_publication_order())
                    .unwrap(),
            ];
            let mut groups = vec![
                PublicationGroup::new(PublicationBoundary::Facts, vec!["static_facts"]),
                PublicationGroup::new(PublicationBoundary::Local, vec!["analyze_local"]),
                PublicationGroup::new(PublicationBoundary::Model, vec!["apply_models"]),
                PublicationGroup::new(PublicationBoundary::Summary, vec!["analyze_summaries"]),
                PublicationGroup::new(PublicationBoundary::Structural, vec!["analyze_structural"]),
                PublicationGroup::new(PublicationBoundary::Analytic, vec!["analyze_analytic"]),
            ];
            if target == analysis::frontier::Target::Catalog {
                stages.extend([
                    selection::build::stage(profile, &model, &fixture_publication_order()).unwrap(),
                    synthesis::build::stage(
                        profile,
                        &settings,
                        &model,
                        &fixture_publication_order(),
                    )
                    .unwrap(),
                    retrieval::build::stage(
                        profile,
                        config.retrieval().iter().next().unwrap(),
                        &model,
                     &fixture_publication_order())
                    .unwrap(),
                    analysis::frontier::stage(profile, target, &model, &fixture_publication_order()).unwrap(),
                ]);
                groups.push(PublicationGroup::new(
                    PublicationBoundary::Synthesis,
                    vec!["synthesis"],
                ));
            }
            let schedule =
                Schedule::build_with_publications(&model, stages, &[], profile, groups).unwrap();
            let actual = schedule
                .stages()
                .iter()
                .flat_map(|s| s.outputs.iter().map(|r| r.name()))
                .collect::<std::collections::BTreeSet<_>>();
            let expected = match target {
                analysis::frontier::Target::Analysis => analysis_frontier_relations(),
                analysis::frontier::Target::Catalog => catalog_frontier_relations(),
            }
            .iter()
            .map(Relation::name)
            .collect::<std::collections::BTreeSet<_>>();
            let missing = expected.difference(&actual).collect::<Vec<_>>();
            let extra = actual.difference(&expected).collect::<Vec<_>>();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "profile={profile:?} target={target:?}; unowned={missing:?}; undeclared={extra:?}"
            );
        }
    }
    assert_eq!(b.reserved(), 0);
}

fn fixture_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    lctx_model::domain::stages::PublicationOrder::registered(
        lctx_model::domain::ContentHash::of(b"fixture publication order"),
        &[
            (0, lctx_model::domain::stages::PublicationBoundary::Facts),
            (1, lctx_model::domain::stages::PublicationBoundary::Local),
            (2, lctx_model::domain::stages::PublicationBoundary::Model),
            (3, lctx_model::domain::stages::PublicationBoundary::Summary),
            (
                4,
                lctx_model::domain::stages::PublicationBoundary::Structural,
            ),
            (5, lctx_model::domain::stages::PublicationBoundary::Analytic),
            (
                6,
                lctx_model::domain::stages::PublicationBoundary::Synthesis,
            ),
        ],
    )
    .unwrap()
}
