//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[allow(dead_code)] // Shared fixture has controls used by the lexical suites.
#[path = "../../lctx-model/tests/fixtures/lexical.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,lexical::*,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres,testcontainers::{ImageExt,runners::AsyncRunner}};

#[tokio::test]
async fn coverage_cannot_claim_a_foreign_input_even_with_matching_provider_and_context() {
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
        if foreign {
            let other = InputRevision::from_entries(vec![]).unwrap();
            let mut inputs = fixture.rows::<InputRevision>(); inputs.push(other.clone()); fixture.put(inputs);
            let origin = fixture.rows::<InputOrigin>().pop().unwrap();
            let mut acquisitions = fixture.rows::<InputAcquisition>(); acquisitions.push(InputAcquisition { input: other.id(),origin: origin.id() }); fixture.put(acquisitions);
            let scope = CoverageScope::Input { input: other.id() };
            let mut scopes = fixture.rows::<CoverageScope>(); scopes.push(scope.clone()); fixture.put(scopes);
            let mut outcomes = fixture.rows::<ProviderCoverage>(); outcomes[0].scope = scope.id(); fixture.put(outcomes);
        }
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(fixture.check(&ProviderCoverage::invariants()[0]).is_ok(), !foreign);
        let generation = store.create_conformance(ContentHash::of(b"coverage-ownership"),"catalog").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer,generation,&Batch::new(&model,fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,
            CoverageScope,ProviderCoverage,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Occurrence,
            LexicalScope,BindingEvent,LexicalTarget,LexicalScopeObservation,LexicalScopeSupport,BindingObservation,BindingSupport,
            ReferenceObservation,ReferenceSupport,LexicalResolution,LexicalResolutionSupport,Evidence);
        store.seal(generation).await.unwrap();
        if foreign {
            assert!(matches!(store.validate(generation).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation).await.unwrap(); store.publish(generation).await.unwrap();
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
