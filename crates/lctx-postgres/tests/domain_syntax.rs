//! Typed syntax contracts (cutover plan A3) through the permanent PostgreSQL lowering: the fixture
//! with an ambiguous attachment publishes and reads back; a decorator outside its declaration is
//! refused at validation. Not a producer test.
#[path = "../../lctx-model/tests/fixtures/syntax.rs"]
#[macro_use]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::ArtifactChunk, assertion::*, attribution::*, conditions::*, input::*, source::*,
    stages::Profile, syntax::*, value::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::{DisposableDatabase, Harness, fixtures::budget};
use std::sync::Arc;

#[tokio::test]
async fn typed_syntax_publishes_and_refuses_misplaced_syntax() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut valid = Fixture::new();
    valid.attachment(AttachmentKind::Ambiguous, &["f.name", "f"]);
    let mut misplaced = Fixture::new();
    misplaced.decorators[0].declaration = misplaced.occ["C"].id();
    misplaced.sync();
    for (fixture, admitted) in [(valid, true), (misplaced, false)] {
        let mut attempt = Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget())
            .await
            .unwrap();
        let generation = attempt.generation();
        macro_rules! copy { ($($ty:ty),+) => { $( attempt.copy(&Batch::new(&model, fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap(); )+ }; }
        syntax_relations!(copy);
        attempt.seal().await.unwrap();
        if !admitted {
            let refused = attempt.validate(&budget()).await;
            assert!(
                matches!(&refused, Err(Error::Model(ModelError::Invalid(message))) if message.contains("a decorator lies outside")),
                "{refused:?}"
            );
            attempt.abort().await.unwrap();
            continue;
        }
        attempt.validate(&budget()).await.unwrap();
        attempt.publish().await.unwrap();
        let mut lease = store.pin(&db.reader, generation, budget()).await.unwrap();
        let sorted = |mut rows: Vec<DeclarationObservation>| {
            rows.sort_by_key(Record::id);
            rows
        };
        assert_eq!(
            lease.read::<DeclarationObservation>().await.unwrap().rows(),
            sorted(fixture.declarations.clone()).as_slice()
        );
        assert_eq!(
            lease
                .read::<AttachmentCandidate>()
                .await
                .unwrap()
                .rows()
                .len(),
            2
        );
        assert_eq!(
            lease.read::<SubjectBoundary>().await.unwrap().rows(),
            fixture.boundaries.as_slice()
        );
        assert_eq!(
            lease
                .read::<ParameterSyntaxObservation>()
                .await
                .unwrap()
                .rows()
                .len(),
            3
        );
        lease.release().await.unwrap();
        store.retire(generation).await.unwrap();
    }
}
