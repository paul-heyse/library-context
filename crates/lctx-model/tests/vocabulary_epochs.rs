use lctx_model::{
    Domain,
    domain::{
        input::Package,   stages::*,
        value::Literal, *,
    },
};
fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
    Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"epoch test"),
        configuration: ContentHash::of(b"test"),
    }
}
#[test]
fn epoch_schedule_refuses_unbounded_reads_ordinary_multiwriters_and_self_group_dependencies() {
    let model =
        ValidatedModel::declared(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
            .unwrap();
    let groups = vec![
        PublicationGroup::new(PublicationBoundary::Facts, vec!["v0"]),
        PublicationGroup::new(PublicationBoundary::Dispatch, vec!["v1"]),
    ];
    let unbounded = vec![
        stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
        stage(
            "v1",
            vec![RelationUse::stored::<Literal>()],
            vec![RelationUse::of::<Literal>()],
        ),
    ];
    assert!(
        Schedule::build_with_publications(&model, unbounded, &[], Profile::Catalog, groups)
            .is_err()
    );
    let ordinary = vec![
        stage("a", vec![], vec![RelationUse::of::<Package>()]),
        stage("b", vec![], vec![RelationUse::of::<Package>()]),
    ];
    assert!(
        Schedule::build_with_publications(
            &model,
            ordinary,
            &[],
            Profile::Catalog,
            vec![PublicationGroup::new(
                PublicationBoundary::Facts,
                vec!["a", "b"]
            )]
        )
        .is_err()
    );
    let self_read = vec![
        stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
        stage(
            "r",
            vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Facts)],
            vec![RelationUse::of::<Package>()],
        ),
    ];
    assert!(
        Schedule::build_with_publications(
            &model,
            self_read,
            &[],
            Profile::Catalog,
            vec![PublicationGroup::new(
                PublicationBoundary::Facts,
                vec!["v0", "r"]
            )]
        )
        .is_err()
    );
}
#[test]
fn epoch_schedule_filters_inactive_writers_and_refuses_two_writers_in_a_later_epoch() {
    let model =
        ValidatedModel::declared(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
            .unwrap();
    let mut behavioral = stage("behavioral", vec![], vec![RelationUse::of::<Literal>()]);
    behavioral.profiles = vec![Profile::Behavioral];
    assert!(
        Schedule::build(
            &model,
            vec![
                behavioral,
                stage("catalog", vec![], vec![RelationUse::of::<Package>()])
            ],
            &[],
            Profile::Catalog
        )
        .is_ok()
    );
    let stages = ["v0", "v1", "v2"]
        .into_iter()
        .map(|n| stage(n, vec![], vec![RelationUse::of::<Literal>()]))
        .collect();
    let groups = vec![
        PublicationGroup::new(PublicationBoundary::Facts, vec!["v0"]),
        PublicationGroup::new(PublicationBoundary::Dispatch, vec!["v1", "v2"]),
    ];
    assert!(
        Schedule::build_with_publications(&model, stages, &[], Profile::Catalog, groups).is_err()
    );
}
#[test]
fn named_boundary_codes_are_not_publication_positions() {
    let model =
        ValidatedModel::declared(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
            .unwrap();
    let boundaries = [
        PublicationBoundary::Facts,
        PublicationBoundary::BaseEvaluation,
        PublicationBoundary::BaseCompletion,
        PublicationBoundary::SourceCall,
        PublicationBoundary::EnrichedExecution,
        PublicationBoundary::ExecutionModel,
        PublicationBoundary::Summary,
    ];
    let names = [
        "facts",
        "evaluation",
        "completion",
        "source_call",
        "enriched",
        "execution",
        "summary",
    ];
    let mut stages: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            stage(
                name,
                if i == 0 {
                    vec![]
                } else {
                    vec![RelationUse::stored::<Literal>().at_epoch(boundaries[i - 1])]
                },
                vec![RelationUse::of::<Literal>()],
            )
        })
        .collect();
    stages.push(stage(
        "old_reader",
        vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::BaseEvaluation)],
        vec![RelationUse::of::<Package>()],
    ));
    let groups = boundaries
        .iter()
        .zip(names)
        .map(|(b, n)| PublicationGroup::new(*b, vec![n]))
        .collect();
    let schedule =
        Schedule::build_with_publications(&model, stages, &[], Profile::Catalog, groups).unwrap();
    assert_eq!(PublicationBoundary::ExecutionModel.code(), 3);
    assert_eq!(
        schedule
            .prefix_for(PublicationBoundary::ExecutionModel)
            .unwrap()
            .ordinal(),
        5
    );
    assert_eq!(
        schedule
            .prefix_for(PublicationBoundary::Summary)
            .unwrap()
            .ordinal(),
        6
    );
    assert!(
        schedule
            .prefix_for(PublicationBoundary::CatalogCore)
            .is_err()
    );
}
#[test]
fn registered_order_refuses_duplicates_gaps_unknown_and_foreign_prefixes() {
    let digest = ContentHash::of(b"order");
    let entries = [
        (0, PublicationBoundary::Facts),
        (1, PublicationBoundary::BaseCompletion),
        (2, PublicationBoundary::Summary),
    ];
    let order = PublicationOrder::registered(digest, &entries).unwrap();
    assert!(
        PublicationOrder::registered(
            digest,
            &[
                (0, PublicationBoundary::Facts),
                (1, PublicationBoundary::Facts)
            ]
        )
        .is_err()
    );
    assert!(
        PublicationOrder::registered(
            digest,
            &[
                (0, PublicationBoundary::Facts),
                (2, PublicationBoundary::Summary)
            ]
        )
        .is_err()
    );
    assert!(PublicationOrder::registered(digest, &[(0, PublicationBoundary::Summary)]).is_err());
    assert!(order.decode(3).is_err());
    let foreign = PublicationOrder::registered(ContentHash::of(b"foreign"), &entries)
        .unwrap()
        .decode(1)
        .unwrap();
    assert!(order.validate(foreign).is_err());
    assert!(order.decode(1).unwrap().earlier(foreign).is_err());
    let model = ValidatedModel::declared(vec![Relation::of::<Literal>()]).unwrap();
    let stages = vec![
        stage("a", vec![], vec![RelationUse::of::<Literal>()]),
        stage("b", vec![], vec![RelationUse::of::<Literal>()]),
    ];
    assert!(
        Schedule::build_with_publications(
            &model,
            stages,
            &[],
            Profile::Catalog,
            vec![
                PublicationGroup::new(PublicationBoundary::Facts, vec!["a"]),
                PublicationGroup::new(PublicationBoundary::Facts, vec!["b"])
            ]
        )
        .is_err()
    );
}
