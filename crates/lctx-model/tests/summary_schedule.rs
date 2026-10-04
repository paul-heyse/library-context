//! Static schedule contracts. No provider or store qualification is claimed here.
use lctx_model::domain::{analysis, execution, stages::*, *};
#[test]
fn summary_dependencies_use_only_published_nominal_evidence_routes() {
    let model = model().unwrap();
    let budget = resources::ResourceBudget::fixed(1 << 28).unwrap();
    let catalog = models::Catalog::committed().unwrap();
    for profile in Profile::ALL {
        let pairs = vec![
            local_semantics::definition(),
            execution::configuration::base_evaluation(),
            execution::configuration::base_completion(),
            execution::configuration::source_calls(),
            execution::configuration::enriched_execution(catalog.declaration().id()),
            execution::configuration::models(catalog.declaration().id()),
            execution::configuration::summaries(catalog.declaration().id(), Default::default())
                .unwrap(),
        ];
        let configuration =
            analysis::preparation::Configuration::new(&catalog, pairs.clone(), &budget).unwrap();
        let facts = Stage {
            name: "fixture_facts_declaration",
            inputs: vec![],
            outputs: facts_relations()
                .iter()
                .map(RelationUse::of_relation)
                .collect(),
            contributes: vec![],
            coverage: vec![],
            profiles: vec![profile],
            effect: Effect::Pure,
            code: ContentHash::of(b"static-only"),
            configuration: ContentHash::of(b"static-only"),
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
            configuration.declaration(),
            analysis::preparation::native_stage(profile, &model, &alignment_publication_order()).unwrap(),
        ];
        stages.extend([
            local_semantics::stage(profile, &pairs[0].1, &model, &alignment_publication_order()).unwrap(),
            execution::production::stage(profile, &pairs[1].1, &model, &alignment_publication_order()).unwrap(),
            execution::completion_production::stage(profile, &pairs[2].1, &model, &alignment_publication_order()).unwrap(),
            execution::source_call::stage(profile, &pairs[3].1, &model, &alignment_publication_order()).unwrap(),
            execution::enriched_production::stage(profile, &pairs[4].1, &model, &alignment_publication_order()).unwrap(),
            execution::model_production::stage(profile, &pairs[5].1, &model, &alignment_publication_order()).unwrap(),
        ]);
        let summary = execution::summary_replay::stage(profile, &pairs[6].1, &model, &alignment_publication_order()).unwrap();
        for name in [
            analysis::base_completion::AnalysisDerivation::NAME,
            analysis::base_evaluation::AnalysisDerivation::NAME,
            analysis::enriched_execution::AnalysisDerivation::NAME,
            analysis::source_call::AnalysisDerivation::NAME,
        ] {
            assert!(
                !summary.inputs.iter().any(|i| i.name() == name),
                "invocation membership cannot invent a generic proof producer: {name}"
            );
        }
        // Exact consumed views retain both checkpoints; sufficient grants acknowledge Model.
        let views = execution::summary_production::SummaryData::consumed_inputs(profile);
        for name in [conditions::Condition::NAME, conditions::ConditionNode::NAME, assertion::AssertionQualification::NAME].into_iter().filter(|_| profile == Profile::Behavioral) {
            assert!(views.iter().any(|input| input.name() == name && input.prefix() == Some(PublicationBoundary::Facts)));
            assert!(views.iter().any(|input| input.name() == name && input.prefix() == Some(PublicationBoundary::Model)));
            assert_eq!(summary.inputs.iter().find(|input| input.name() == name).unwrap().prefix(), Some(PublicationBoundary::Model));
        }
        stages.push(summary);
        let schedule = Schedule::build_with_publications(
            &model,
            stages,
            &[],
            profile,
            vec![
                PublicationGroup::new(
                    PublicationBoundary::Facts,
                    vec!["fixture_facts_declaration"],
                ),
                PublicationGroup::new(PublicationBoundary::Local, vec!["analyze_local"]),
                PublicationGroup::new(PublicationBoundary::Model, vec!["apply_models"]),
                PublicationGroup::new(PublicationBoundary::Summary, vec!["analyze_summaries"]),
            ],
        )
        .unwrap();
        assert!(
            schedule
                .stages()
                .iter()
                .position(|s| s.name == "apply_models")
                .unwrap()
                < schedule
                    .stages()
                    .iter()
                    .position(|s| s.name == "analyze_summaries")
                    .unwrap()
        );
    }
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn summary_model_assumptions_hydrate_both_evidence_indexes_without_mixing_facts() {
    use lctx_model::domain::{
        assumptions::*, execution::summary_production::SummaryData, value::*,
    };
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    fn load<R: Record>(data: &mut SummaryData, rows: &[R], prefix: PublicationBoundary) {
        let input = ValidationInput::of::<R>(&["id"]).at_epoch(prefix);
        data.visit_input(&input, &R::encode(rows).unwrap()).unwrap();
    }
    for profile in Profile::ALL {
        assert!(
            SummaryData::consumed_inputs(profile)
                .iter()
                .filter(|i| is_vocabulary(i.name()))
                .all(|i| matches!(
                    i.prefix(),
                    Some(PublicationBoundary::Facts | PublicationBoundary::Model)
                ))
        );
    }
    for input in AssumptionIndex::inputs() {
        let declarations: Vec<_> = SummaryData::inputs()
            .into_iter()
            .filter(|i| i.name() == input.name())
            .collect();
        assert!(!declarations.is_empty());
        assert!(
            declarations
                .iter()
                .all(|i| i.prefix() == Some(PublicationBoundary::Model))
        );
    }
    let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
    let mut data = SummaryData::new(&budget);
    let facts_predicate = Predicate::IsNone;
    let model_predicate = Predicate::IsValue {
        value: Literal::None.id(),
    };
    load(
        &mut data,
        std::slice::from_ref(&facts_predicate),
        PublicationBoundary::Facts,
    );
    load(
        &mut data,
        std::slice::from_ref(&model_predicate),
        PublicationBoundary::Model,
    );
    assert!(data.entry.predicates.get(facts_predicate.id()).is_some());
    assert!(data.entry.predicates.get(model_predicate.id()).is_none());
    assert!(
        data.vocabulary
            .predicates
            .get(&model_predicate.id())
            .is_some()
    );
    assert!(
        data.vocabulary
            .predicates
            .get(&facts_predicate.id())
            .is_none()
    );

    let universe = AssumptionUniverse {
        context: id(1),
        input: id(2),
        environment: ContentHash::of(b"fixture environment"),
        model_definition: ContentHash::of(b"fixture model"),
    };
    let assumption = Assumption::NoExtraOverrides {
        class: id(3),
        support: id(4),
        universe: universe.id(),
    };
    let basis = AssumptionSet::new([assumption.id()]).unwrap();
    load(
        &mut data,
        std::slice::from_ref(&universe),
        PublicationBoundary::Model,
    );
    load(
        &mut data,
        std::slice::from_ref(&assumption),
        PublicationBoundary::Model,
    );
    load(
        &mut data,
        std::slice::from_ref(&basis.set),
        PublicationBoundary::Model,
    );
    assert!(
        data.local_evidence
            .assumptions
            .resolve(basis.set.id())
            .is_err()
    );
    assert!(
        data.model_evidence
            .assumptions
            .resolve(basis.set.id())
            .is_err()
    );
    load(&mut data, &basis.members, PublicationBoundary::Model);
    for resolver in [
        &data.local_evidence.assumptions,
        &data.model_evidence.assumptions,
    ] {
        assert_eq!(resolver.resolve(basis.set.id()).unwrap(), basis);
        assert!(resolver.universes.get(&universe.id()).is_some());
    }
    assert_eq!(
        data.vocabulary.assumption_sets.get(&basis.set.id()),
        Some(&basis.set)
    );
    let batch = AssumptionSet::encode(std::slice::from_ref(&basis.set)).unwrap();
    let unprefixed = ValidationInput::of::<AssumptionSet>(&["id"]);
    assert!(data.visit_input(&unprefixed, &batch).is_err());
    assert!(
        data.visit_input(&unprefixed.at_epoch(PublicationBoundary::Local), &batch)
            .is_err()
    );
    drop(data);
    assert_eq!(budget.reserved(), 0);
}

fn alignment_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    use lctx_model::domain::stages::*;
    PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::Local, vec!["local"]),
        PublicationGroup::new(PublicationBoundary::Model, vec!["model"]),
        PublicationGroup::new(PublicationBoundary::Summary, vec!["summary"]),
        PublicationGroup::new(PublicationBoundary::Structural, vec!["structural"]),
        PublicationGroup::new(PublicationBoundary::Analytic, vec!["analytic"]),
        PublicationGroup::new(PublicationBoundary::Synthesis, vec!["synthesis"]),
    ]).unwrap()
}
