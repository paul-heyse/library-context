//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/documents.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, conditions::*, documents::*, input::*, source::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn document_nodes_and_optional_spans_survive_sealed_postgres_validation() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for foreign in [false, true] {
        let mut fixture = Fixture::new();
        if foreign {
            fixture.foreign_inner();
        }
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(
            fixture
                .check(&DocumentComponentSupport::invariants()[0])
                .is_ok(),
            !foreign
        );
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
            ProviderCoverage,
            Condition,
            ConditionNode,
            AssertionQualification,
            SourceArtifact,
            ArtifactChunk,
            Occurrence,
            DocumentNode,
            DocumentAttributeValue,
            DocumentObservation,
            DocumentSupport,
            PassageObservation,
            PassageSupport,
            CodeBlockObservation,
            CodeBlockSupport,
            DocumentLinkObservation,
            DocumentLinkSupport,
            DocumentMentionObservation,
            DocumentMentionSupport,
            DocumentComponentObservation,
            DocumentComponentSupport,
            DocumentAttributeObservation,
            DocumentAttributeSupport,
            Evidence
        );
        generation_h.seal().await.unwrap();
        if foreign {
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
                lease
                    .read::<DocumentComponentObservation>()
                    .await
                    .unwrap()
                    .rows(),
                fixture.rows::<DocumentComponentObservation>()
            );
            assert_eq!(
                lease.read::<DocumentAttributeValue>().await.unwrap().rows(),
                fixture.rows::<DocumentAttributeValue>()
            );
            assert_eq!(
                lease.read::<CodeBlockObservation>().await.unwrap().rows(),
                fixture.rows::<CodeBlockObservation>()
            );
            lease.release().await.unwrap();
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
