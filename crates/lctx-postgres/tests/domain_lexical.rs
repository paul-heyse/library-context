//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/lexical.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,lexical::*,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn lexical_support_and_optional_subjects_survive_sealed_postgres_validation() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    for foreign in [false,true] {
        let mut fixture = Fixture::new();
        if foreign { fixture.foreign_value(); }
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(fixture.check(&BindingSupport::invariants()[0]).is_ok(), !foreign);
        let generation = store.create_conformance(ContentHash::of(b"lexical-contract"),"catalog").await.unwrap();
        // The same rows also go to an in-memory generation, whose validation must agree with the store.
        let memory = lctx_model::domain::memory::MemoryGeneration::conformance(&model, &budget());
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            let batch = Batch::new(&model,fixture.rows::<$ty>(), &budget()).unwrap();
            store.copy(&writer,generation,&batch, &budget()).await.unwrap();
            memory.put(&batch).unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,
            CoverageScope,ProviderCoverage,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Occurrence,
            LexicalScope,BindingEvent,LexicalTarget,LexicalScopeObservation,LexicalScopeSupport,BindingObservation,BindingSupport,
            ReferenceObservation,ReferenceSupport,LexicalResolution,LexicalResolutionSupport,Evidence);
        store.seal(generation).await.unwrap();
        if foreign {
            assert!(matches!(store.validate(generation, &budget()).await, Err(Error::Model(_))));
            assert!(memory.validate(&model, &budget()).is_err(), "the in-memory generation refuses the same contents");
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            let digest = store.validate(generation, &budget()).await.unwrap(); store.publish(generation).await.unwrap();
            assert_eq!(memory.validate(&model, &budget()).unwrap(), digest, "in-memory and stored content digests agree");
            let mut lease = store.pin(&reader,generation, budget()).await.unwrap();
            assert_eq!(lease.read::<BindingObservation>().await.unwrap().rows(),fixture.rows::<BindingObservation>());
            assert_eq!(lease.read::<LexicalResolution>().await.unwrap().rows(),fixture.rows::<LexicalResolution>());
            assert_eq!(lease.read::<LexicalScopeObservation>().await.unwrap().rows(),fixture.rows::<LexicalScopeObservation>());
            lease.release().await.unwrap(); store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
