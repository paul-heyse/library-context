use lctx_model::domain::{
    artifact::ArtifactChunk, input::*, memory::MemoryGeneration, resources::ResourceBudget,
    source::*, stages::*, *,
};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn ready<T>(future: impl Future<Output = T>) -> T {
    match std::pin::pin!(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("in-memory sink unexpectedly pending"),
    }
}
fn package_release_model() -> ValidatedModel {
    ValidatedModel::validate(vec![Relation::of::<Package>(), Relation::of::<Release>()]).unwrap()
}
fn input_order_model() -> ValidatedModel {
    ValidatedModel::validate(vec![
        Relation::of::<InputRevision>(),
        Relation::of::<SourceArtifact>(),
        Relation::of::<ArtifactChunk>(),
        Relation::of::<Occurrence>(),
    ])
    .unwrap()
}
fn package_model() -> ValidatedModel {
    ValidatedModel::validate(vec![Relation::of::<Package>()]).unwrap()
}
fn generation<R: Record>(model: &ValidatedModel, batches: Vec<Vec<R>>) -> MemoryGeneration {
    let generation = MemoryGeneration::conformance(model, &budget());
    for rows in batches {
        generation
            .put(&Batch::new(model, rows, &budget()).unwrap())
            .unwrap();
    }
    generation
}

#[test]
fn absent_references_and_repeated_keys_refuse_like_store_keys() {
    let model = package_release_model();
    let (p, q) = (Package { name: "p".into() }, Package { name: "q".into() });
    let release = Release {
        package: p.id(),
        version: "1".into(),
    };
    let orphan = MemoryGeneration::conformance(&model, &budget());
    orphan
        .put(&Batch::new(&model, vec![release.clone()], &budget()).unwrap())
        .unwrap();
    assert!(
        orphan.validate(&model, &budget()).is_err(),
        "a release needs its package"
    );
    let repeated = generation(&model, vec![vec![p.clone()], vec![p.clone()]]);
    assert!(
        matches!(
            repeated.validate(&model, &budget()),
            Err(ModelError::Conflict(_))
        ),
        "one identity across batches is a key conflict"
    );
    let complete = |order: bool| {
        let g = if order {
            generation(&model, vec![vec![p.clone(), q.clone()]])
        } else {
            generation(&model, vec![vec![q.clone()], vec![p.clone()]])
        };
        g.put(&Batch::new(&model, vec![release.clone()], &budget()).unwrap())
            .unwrap();
        g.validate(&model, &budget()).unwrap()
    };
    assert_eq!(
        complete(true),
        complete(false),
        "batch split and arrival order do not change content"
    );
}

#[test]
fn model_invariants_run_over_declared_input_order() {
    let model = input_order_model();
    let bytes = b"x = 1";
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "m.py".into(),
        content: ContentHash::of(bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "m.py".into(), bytes).unwrap();
    let chunks = ArtifactChunk::split(&source, bytes)
        .unwrap()
        .collect::<Vec<_>>();
    let outside = Occurrence {
        source: source.id(),
        start: 0,
        end: 99,
        syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Read,
        structural_path: vec![0],
    };
    let g = MemoryGeneration::conformance(&model, &budget());
    g.put(&Batch::new(&model, vec![input], &budget()).unwrap())
        .unwrap();
    g.put(&Batch::new(&model, vec![source], &budget()).unwrap())
        .unwrap();
    g.put(&Batch::new(&model, chunks, &budget()).unwrap())
        .unwrap();
    g.put(&Batch::new(&model, vec![outside], &budget()).unwrap())
        .unwrap();
    assert!(
        g.validate(&model, &budget()).is_err(),
        "an occurrence beyond its source bytes fails the stored invariant"
    );
}

#[test]
fn a_bound_generation_accepts_only_its_execution_and_charges_what_it_stores() {
    let model = package_model();
    let stages = vec![Stage {
        name: "packages",
        inputs: vec![],
        outputs: vec![RelationUse::of::<Package>()],
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"p"),
        configuration: ContentHash::of(b"c"),
    }];
    let schedule = Schedule::build(&model, stages, &[], Profile::Catalog).unwrap();
    let store = budget();
    let (mut first, mut second) = (schedule.execute(), schedule.execute());
    let sink = MemoryGeneration::bind(&model, &store, &mut first).unwrap();
    let batch = Batch::new(&model, vec![Package { name: "p".into() }], &budget()).unwrap();
    assert!(
        sink.put(&batch).is_err(),
        "a stage-bound generation refuses unpermitted writes"
    );
    let mut foreign = second.begin("packages").unwrap();
    assert!(
        ready(foreign.write::<Package, _>(async |permit| sink.copy(permit, &batch).await)).is_err()
    );
    drop(foreign);
    let mut stage = first.begin("packages").unwrap();
    ready(stage.write::<Package, _>(async |permit| sink.copy(permit, &batch).await)).unwrap();
    ready(stage.complete(&sink, ProviderOutcome::Complete)).unwrap();
    first.finish().unwrap();
    assert!(
        store.reserved() >= batch.arrow().get_array_memory_size(),
        "stored batches are charged to the generation"
    );
    sink.validate(&model, &budget()).unwrap();
    drop(sink);
    assert_eq!(store.reserved(), 0);
}
