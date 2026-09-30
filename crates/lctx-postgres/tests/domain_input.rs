//! The acquisition contracts (cutover plan A1) through the permanent PostgreSQL lowering: a
//! classified installed input publishes and reads back; an artifact with two classes is refused at
//! validation. Not a producer test.
#[path = "../../lctx-model/tests/fixtures/input.rs"]
#[macro_use]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::ArtifactChunk, input::*, source::SourceArtifact, stages::Profile, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::{DisposableDatabase, Harness, fixtures::budget};
use std::sync::Arc;

#[tokio::test]
async fn classified_inputs_publish_and_a_double_class_is_refused() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut doubled = Fixture::new();
    let owned = doubled.artifact("demo/__init__.py").id();
    doubled.unowned.push(UnownedArtifact {
        artifact: owned,
        acquisition: doubled.acquisition.id(),
    });
    for (fixture, valid) in [(Fixture::new(), true), (doubled, false)] {
        let mut attempt = Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget())
            .await
            .unwrap();
        let generation = attempt.generation();
        macro_rules! copy { ($($ty:ty),+) => { $( attempt.copy(&Batch::new(&model, fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap(); )+ }; }
        input_relations!(copy);
        attempt.seal().await.unwrap();
        if !valid {
            let refused = attempt.validate(&budget()).await;
            assert!(
                matches!(&refused, Err(Error::Model(ModelError::Invalid(message))) if message.contains("more than one")),
                "{refused:?}"
            );
            attempt.abort().await.unwrap();
            continue;
        }
        attempt.validate(&budget()).await.unwrap();
        attempt.publish().await.unwrap();
        let mut lease = store.pin(&db.reader, generation, budget()).await.unwrap();
        assert_eq!(
            lease.read::<UnownedArtifact>().await.unwrap().rows(),
            Batch::new(&model, fixture.rows::<UnownedArtifact>(), &budget())
                .unwrap()
                .rows()
        );
        assert_eq!(
            lease.read::<DerivedArtifact>().await.unwrap().rows(),
            fixture.derived.as_slice()
        );
        assert_eq!(
            lease.read::<EnvironmentFingerprint>().await.unwrap().rows(),
            std::slice::from_ref(&fixture.fingerprint)
        );
        lease.release().await.unwrap();
        store.retire(generation).await.unwrap();
    }
}
