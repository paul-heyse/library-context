//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/guards.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,value::*,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres,testcontainers::{ImageExt,runners::AsyncRunner}};

#[tokio::test]
async fn invoked_guards_retain_typed_origins_and_foreign_source_refusal() {
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
    for (foreign,kind,nested) in [(false,"none",false),(true,"none",false),(false,"formal",false),(false,"formal",true),
        (false,"receiver",false),(false,"receiver",true),(false,"local",false),(false,"local",true)] {
        let mut fixture = Fixture::new(foreign);
        if kind != "none" { fixture.operand_origin(kind,nested); }
        let eligible = matches!(kind,"none"|"local");
        let valid = !foreign && eligible;
        assert_eq!(fixture.check(&SyntaxSupport::invariants()[0]).is_ok(),!foreign);
        assert_eq!(fixture.check(&EvaluationAtom::invariants()[0]).is_ok(),eligible);
        assert_ne!(fixture.diagram.support()[0],fixture.origin.id());
        assert_eq!(fixture.calls.len(),2);
        let generation = store.create_conformance(ContentHash::of(b"guard-contract"),"catalog").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer,generation,&Batch::new(&model,fixture.rows::<$ty>()).unwrap()).await.unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,
            CoverageScope,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Evidence,Occurrence,
            PlaceRoot,AccessPath,Place,Predicate,EvaluationAtom,SyntaxObservation,SyntaxSupport);
        store.seal(generation).await.unwrap();
        if !valid {
            assert!(matches!(store.validate(generation).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation).await.unwrap(); store.publish(generation).await.unwrap();
            let mut lease = store.pin(&reader,generation).await.unwrap();
            assert_eq!(lease.read::<EvaluationAtom>().await.unwrap().rows(),fixture.rows::<EvaluationAtom>());
            assert_eq!(lease.read::<Predicate>().await.unwrap().rows(),fixture.rows::<Predicate>());
            let other_lease = store.pin(&reader,generation).await.unwrap();
            lease.release().await.unwrap();
            assert!(matches!(store.retire(generation).await,Err(Error::Busy)));
            other_lease.release().await.unwrap();
            store.retire(generation).await.unwrap();
        }
    }
}
