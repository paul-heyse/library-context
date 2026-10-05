//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/guards.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, conditions::*, input::*, source::*, value::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn invoked_guards_retain_typed_origins_and_foreign_source_refusal() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for (foreign, kind, nested) in [
        (false, "none", false),
        (true, "none", false),
        (false, "formal", false),
        (false, "formal", true),
        (false, "receiver", false),
        (false, "receiver", true),
        (false, "local", false),
        (false, "local", true),
    ] {
        let mut fixture = Fixture::new(foreign);
        if kind != "none" {
            fixture.operand_origin(kind, nested);
        }
        let eligible = matches!(kind, "none" | "local");
        let valid = !foreign && eligible;
        assert_eq!(
            fixture.check(&lctx_model::domain::validation::invariants_for::<SyntaxSupport>()[0]).is_ok(),
            !foreign
        );
        assert_eq!(
            fixture.check(&lctx_model::domain::validation::invariants_for::<EvaluationAtom>()[0]).is_ok(),
            eligible
        );
        assert_ne!(fixture.diagram.support()[0], fixture.origin.id());
        assert_eq!(fixture.calls.len(), 2);
        let mut generation_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Catalog,
            budget(),
        )
        .await
        .unwrap();
        let generation = generation_h.generation();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            generation_h.copy(&Batch::new(&model,fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(
            InputRevision,
            InputOrigin,
            InputAcquisition,
            AnalysisContext,
            Provider,
            ProviderRun,
            RunFamily,
            ProviderSurface,
            CoverageScope,
            Condition,
            ConditionNode,
            AssertionQualification,
            SourceArtifact,
            ArtifactChunk,
            Evidence,
            Occurrence,
            PlaceRoot,
            AccessPath,
            Place,
            Predicate,
            EvaluationAtom,
            SyntaxObservation,
            SyntaxSupport
        );
        generation_h.seal().await.unwrap();
        if !valid {
            assert!(matches!(
                generation_h.validate(&budget()).await,
                Err(Error::Model(_))
            ));
            assert!(generation_h.publish().await.is_err());
            generation_h.abort().await.unwrap();
        } else {
            generation_h.validate(&budget()).await.unwrap();
            generation_h.publish().await.unwrap();
            let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
            assert_eq!(
                lease.read::<EvaluationAtom>().await.unwrap().rows(),
                fixture.rows::<EvaluationAtom>()
            );
            assert_eq!(
                lease.read::<Predicate>().await.unwrap().rows(),
                fixture.rows::<Predicate>()
            );
            let other_lease = store.pin(&reader, generation, budget()).await.unwrap();
            lease.release().await.unwrap();
            assert!(matches!(store.retire(generation).await, Err(Error::Busy)));
            other_lease.release().await.unwrap();
            store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
