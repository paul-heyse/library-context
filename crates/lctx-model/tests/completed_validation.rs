use lctx_model::domain::{
    artifact::ArtifactChunk, input::*, resources::ResourceBudget, source::*, *,
};
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 30).unwrap()
}
fn frame<R: Record>(rows: &[R]) -> (&'static str, arrow_array::RecordBatch) {
    (R::NAME, R::encode(rows).unwrap())
}
#[test]
fn absent_nominal_references_and_repeated_keys_refuse() {
    let model =
        ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
            .unwrap();
    let p = Package { name: "p".into() };
    let release = Release {
        package: p.id(),
        version: "1".into(),
    };
    assert!(
        validation::replay::replay(&model, &[frame(std::slice::from_ref(&release))], &budget())
            .is_err()
    );
    assert!(matches!(
        validation::replay::replay(
            &model,
            &[
                frame(std::slice::from_ref(&p)),
                frame(std::slice::from_ref(&p))
            ],
            &budget()
        ),
        Err(ModelError::Conflict(_))
    ));
    let q = Package { name: "q".into() };
    let first = validation::replay::replay(
        &model,
        &[
            frame(&[p.clone(), q.clone()]),
            frame(std::slice::from_ref(&release)),
        ],
        &budget(),
    )
    .unwrap();
    let second = validation::replay::replay(
        &model,
        &[frame(&[q]), frame(&[release]), frame(&[p])],
        &budget(),
    )
    .unwrap();
    assert_eq!(first, second);
}
#[test]
fn shared_model_invariants_run_over_declared_input_order() {
    let model = ValidatedModel::declared(vec![
        Relation::of::<InputRevision>(),
        Relation::of::<SourceArtifact>(),
        Relation::of::<ArtifactChunk>(),
        Relation::of::<Occurrence>(),
    ])
    .unwrap();
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
    assert!(
        validation::replay::replay(
            &model,
            &[
                frame(&[outside]),
                frame(&[source]),
                frame(&chunks),
                frame(&[input])
            ],
            &budget()
        )
        .is_err()
    );
}
