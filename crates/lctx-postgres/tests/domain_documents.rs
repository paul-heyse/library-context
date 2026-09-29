//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/documents.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,documents::*,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn document_nodes_and_optional_spans_survive_sealed_postgres_validation() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    for foreign in [false,true] {
        let mut fixture = Fixture::new();
        if foreign { fixture.foreign_inner(); }
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(fixture.check(&DocumentComponentSupport::invariants()[0]).is_ok(), !foreign);
        let generation = store.create_conformance(ContentHash::of(b"document-contract"),"catalog").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer,generation,&Batch::new(&model,fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,
            CoverageScope,ProviderCoverage,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Occurrence,
            DocumentNode,DocumentAttributeValue,DocumentObservation,DocumentSupport,PassageObservation,PassageSupport,
            CodeBlockObservation,CodeBlockSupport,DocumentLinkObservation,DocumentLinkSupport,DocumentMentionObservation,DocumentMentionSupport,
            DocumentComponentObservation,DocumentComponentSupport,DocumentAttributeObservation,DocumentAttributeSupport,Evidence);
        store.seal(generation).await.unwrap();
        if foreign {
            assert!(matches!(store.validate(generation, &budget()).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation, &budget()).await.unwrap(); store.publish(generation).await.unwrap();
            let mut lease = store.pin(&reader,generation, budget()).await.unwrap();
            assert_eq!(lease.read::<DocumentComponentObservation>().await.unwrap().rows(),fixture.rows::<DocumentComponentObservation>());
            assert_eq!(lease.read::<DocumentAttributeValue>().await.unwrap().rows(),fixture.rows::<DocumentAttributeValue>());
            assert_eq!(lease.read::<CodeBlockObservation>().await.unwrap().rows(),fixture.rows::<CodeBlockObservation>());
            lease.release().await.unwrap(); store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
