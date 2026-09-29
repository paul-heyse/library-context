//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/documents.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,documents::*,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres,testcontainers::{ImageExt,runners::AsyncRunner}};

#[tokio::test]
async fn document_nodes_and_optional_spans_survive_sealed_postgres_validation() {
    let (image,tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PG18 required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(owner,model.clone()).await.unwrap();
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
            assert!(matches!(store.validate(generation).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation).await.unwrap(); store.publish(generation).await.unwrap();
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
