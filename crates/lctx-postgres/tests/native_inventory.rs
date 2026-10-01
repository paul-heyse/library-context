//! Real generation-store validation of the early native evidence inventory, including native
//! attribution validators. This is a typed fixture control, not an extraction qualification.
#[path = "../../lctx-model/tests/fixtures/syntax.rs"]
#[macro_use]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    analysis::{native::*, policy::EvidenceStatus},
    artifact::ArtifactChunk,
    assertion::*,
    attribution::*,
    conditions::*,
    input::*,
    source::*,
    stages::Profile,
    syntax::*,
    value::*,
    *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::{DisposableDatabase, Harness, fixtures::budget};
use std::sync::Arc;

#[tokio::test]
async fn native_inventory_roundtrips_and_refuses_omission_and_forged_status() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(
        ValidatedModel::validate(
            facts_relations()
                .into_iter()
                .chain(analysis::native::relations())
                .collect(),
        )
        .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for mutation in ["none", "omit", "status"] {
        let fixture = Fixture::new();
        let budget = budget();
        let mut native = NativeInventory::new(&budget);
        let input_names = NativeInventory::inputs();
        let mut attempt =
            Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget.clone())
                .await
                .unwrap();
        let generation = attempt.generation();
        macro_rules! copy { ($($ty:ty),+) => { $(
            let batch = Batch::new(&model, fixture.rows::<$ty>(), &budget).unwrap();
            if input_names.iter().any(|input| input.name() == <$ty>::NAME) {
                native.visit(<$ty>::NAME, batch.arrow()).unwrap();
            }
            attempt.copy(&batch, &budget).await.unwrap();
        )+ }; }
        syntax_relations!(copy);
        let output = native.collect().unwrap();
        let mut premises = output.premises.iter().cloned().collect::<Vec<_>>();
        let mut qualifications = output.qualifications.iter().cloned().collect::<Vec<_>>();
        assert!(!premises.is_empty());
        if mutation == "omit" {
            let omitted = premises.remove(0).id();
            qualifications.retain(|q| q.premise != omitted);
        } else if mutation == "status" {
            assert_eq!(
                qualifications[0].status,
                EvidenceStatus::StructurallyObserved
            );
            qualifications[0].status = EvidenceStatus::FixtureChecked;
        }
        attempt
            .copy(&Batch::new(&model, premises, &budget).unwrap(), &budget)
            .await
            .unwrap();
        attempt
            .copy(
                &Batch::new(&model, qualifications.clone(), &budget).unwrap(),
                &budget,
            )
            .await
            .unwrap();
        attempt.seal().await.unwrap();
        if mutation != "none" {
            let refused = attempt.validate(&budget).await;
            assert!(
                matches!(&refused, Err(Error::Model(ModelError::Invalid(message)))
                if message.contains("native inventory differs")),
                "{mutation}: {refused:?}"
            );
            attempt.abort().await.unwrap();
            continue;
        }
        attempt.validate(&budget).await.unwrap();
        attempt.publish().await.unwrap();
        let mut lease = store
            .pin(&db.reader, generation, budget.clone())
            .await
            .unwrap();
        qualifications.sort_by_key(Record::id);
        assert_eq!(
            lease.read::<NativeQualification>().await.unwrap().rows(),
            qualifications.as_slice()
        );
        assert_eq!(
            lease
                .read::<NativeAssertionPremise>()
                .await
                .unwrap()
                .rows()
                .len(),
            qualifications.len()
        );
        lease.release().await.unwrap();
        store.retire(generation).await.unwrap();
    }
}
