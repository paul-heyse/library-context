use lctx_model::{
    Domain,
    domain::{
        input::Package, memory::MemoryGeneration, resources::ResourceBudget, stages::*,
        value::Literal, *,
    },
};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="epoch_results", semantic_source=include_bytes!("vocabulary_epochs.rs"))]
struct ResultRow {
    #[model(key)]
    value: Id<Literal>,
}
fn ready<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("pure future pending"),
    }
}
fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
    Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"epoch test"),
        configuration: ContentHash::of(b"test"),
    }
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(16 << 20).unwrap()
}
#[test]
fn vocabulary_groups_bind_epoch_reads_and_keep_unfinished_results_private() {
    let model = ValidatedModel::validate(vec![
        Relation::of::<Literal>(),
        Relation::of::<ResultRow>(),
        Relation::of::<Package>(),
    ])
    .unwrap();
    let stages = vec![
        stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
        stage(
            "v1",
            vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Facts)],
            vec![RelationUse::of::<Literal>()],
        ),
        stage("result", vec![], vec![RelationUse::of::<ResultRow>()]),
        stage(
            "reader",
            vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Facts)],
            vec![RelationUse::of::<Package>()],
        ),
    ];
    let groups = vec![
        PublicationGroup::new(VocabularyEpoch::Facts, vec!["v0"]),
        PublicationGroup::new(VocabularyEpoch::Dispatch, vec!["v1", "result"]),
    ];
    let schedule =
        Schedule::build_with_publications(&model, stages, &[], Profile::Catalog, groups).unwrap();
    let mut execution = schedule.execute();
    let budget = budget();
    let sink = MemoryGeneration::bind(&model, &budget, &mut execution).unwrap();
    let first = Literal::None;
    let second = Literal::Bool { value: true };
    let initial = Batch::new(&model, vec![first.clone()], &budget).unwrap();
    let mut v0 = execution.begin("v0").unwrap();
    ready(v0.write::<Literal, _>(async |p| sink.copy(p, &initial).await)).unwrap();
    ready(v0.complete(&sink, ProviderOutcome::Complete)).unwrap();
    let initial_source;
    {
        let mut v1 = execution.begin("v1").unwrap();
        initial_source = v1.read::<Literal>().unwrap().source().unwrap().clone();
        let later = Batch::new(&model, vec![first, second.clone()], &budget).unwrap();
        ready(v1.write::<Literal, _>(async |p| sink.copy(p, &later).await)).unwrap();
        ready(v1.complete(&sink, ProviderOutcome::Complete)).unwrap();
    }
    assert_eq!(
        sink.read::<Literal>(&model, &budget).unwrap().rows().len(),
        1,
        "sealed computation must not expose its delta"
    );
    let mut result = execution.begin("result").unwrap();
    let rows = Batch::new(&model, vec![ResultRow { value: second.id() }], &budget).unwrap();
    ready(result.write::<ResultRow, _>(async |p| sink.copy(p, &rows).await)).unwrap();
    ready(result.complete(&sink, ProviderOutcome::Complete)).unwrap();
    assert_eq!(
        sink.read::<Literal>(&model, &budget).unwrap().rows().len(),
        2
    );
    let mut reader = execution.begin("reader").unwrap();
    assert_eq!(
        reader.read::<Literal>().unwrap().source(),
        Some(&initial_source)
    );
    assert_eq!(initial_source.prefix(), Some(VocabularyEpoch::Facts));
    assert_eq!(initial_source.receipt().rows, 1);
    let empty = Batch::<Package>::new(&model, vec![], &budget).unwrap();
    ready(reader.write::<Package, _>(async |p| sink.copy(p, &empty).await)).unwrap();
    ready(reader.complete(&sink, ProviderOutcome::Complete)).unwrap();
    let receipt = execution.finish().unwrap();
    assert_eq!(
        receipt
            .sources()
            .iter()
            .find(|s| s.relation() == Literal::NAME)
            .unwrap()
            .receipt()
            .rows,
        2
    );
}
#[test]
fn epoch_schedule_refuses_unbounded_reads_ordinary_multiwriters_and_self_group_dependencies() {
    let model =
        ValidatedModel::validate(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
            .unwrap();
    let groups = vec![
        PublicationGroup::new(VocabularyEpoch::Facts, vec!["v0"]),
        PublicationGroup::new(VocabularyEpoch::Dispatch, vec!["v1"]),
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
                VocabularyEpoch::Facts,
                vec!["a", "b"]
            )]
        )
        .is_err()
    );
    let self_read = vec![
        stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
        stage(
            "r",
            vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Facts)],
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
                VocabularyEpoch::Facts,
                vec!["v0", "r"]
            )]
        )
        .is_err()
    );
}
#[test]
fn epoch_schedule_filters_inactive_writers_and_refuses_two_writers_in_a_later_epoch() {
    let model =
        ValidatedModel::validate(vec![Relation::of::<Literal>(), Relation::of::<Package>()])
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
        PublicationGroup::new(VocabularyEpoch::Facts, vec!["v0"]),
        PublicationGroup::new(VocabularyEpoch::Dispatch, vec!["v1", "v2"]),
    ];
    assert!(
        Schedule::build_with_publications(&model, stages, &[], Profile::Catalog, groups).is_err()
    );
}
#[test]
fn later_groups_issue_inherited_prefix_sources_without_changing_the_producer() {
    let model = ValidatedModel::validate(vec![
        Relation::of::<Literal>(),
        Relation::of::<ResultRow>(),
        Relation::of::<Package>(),
    ])
    .unwrap();
    let stages = vec![
        stage("v0", vec![], vec![RelationUse::of::<Literal>()]),
        stage(
            "result",
            vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Facts)],
            vec![RelationUse::of::<ResultRow>()],
        ),
        stage(
            "reader",
            vec![RelationUse::stored::<Literal>().at_epoch(VocabularyEpoch::Dispatch)],
            vec![RelationUse::of::<Package>()],
        ),
    ];
    let schedule = Schedule::build_with_publications(
        &model,
        stages,
        &[],
        Profile::Catalog,
        vec![
            PublicationGroup::new(VocabularyEpoch::Facts, vec!["v0"]),
            PublicationGroup::new(VocabularyEpoch::Dispatch, vec!["result"]),
        ],
    )
    .unwrap();
    let mut execution = schedule.execute();
    let budget = budget();
    let sink = MemoryGeneration::bind(&model, &budget, &mut execution).unwrap();
    let literal = Literal::None;
    let rows = Batch::new(&model, vec![literal.clone()], &budget).unwrap();
    let mut v0 = execution.begin("v0").unwrap();
    ready(v0.write::<Literal, _>(async |p| sink.copy(p, &rows).await)).unwrap();
    ready(v0.complete(&sink, ProviderOutcome::Complete)).unwrap();
    let mut result = execution.begin("result").unwrap();
    let old = result.read::<Literal>().unwrap().source().unwrap().clone();
    let rows = Batch::new(
        &model,
        vec![ResultRow {
            value: literal.id(),
        }],
        &budget,
    )
    .unwrap();
    ready(result.write::<ResultRow, _>(async |p| sink.copy(p, &rows).await)).unwrap();
    ready(result.complete(&sink, ProviderOutcome::Complete)).unwrap();
    let mut reader = execution.begin("reader").unwrap();
    let inherited = reader.read::<Literal>().unwrap().source().unwrap().clone();
    assert_eq!(inherited.prefix(), Some(VocabularyEpoch::Dispatch));
    assert_eq!(inherited.producer(), "v0");
    assert_eq!(inherited.receipt(), old.receipt());
    assert_eq!(old.prefix(), Some(VocabularyEpoch::Facts));
    let empty = Batch::<Package>::new(&model, vec![], &budget).unwrap();
    ready(reader.write::<Package, _>(async |p| sink.copy(p, &empty).await)).unwrap();
    ready(reader.complete(&sink, ProviderOutcome::Complete)).unwrap();
    execution.finish().unwrap();
}
