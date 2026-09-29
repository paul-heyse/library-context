//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/deployment.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,deployment::*,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres,testcontainers::{ImageExt,runners::AsyncRunner}};

#[tokio::test]
async fn captured_reports_preserve_values_and_reject_incomplete_or_foreign_evidence() {
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
    for (foreign,missing) in [(false,false),(true,false),(false,true)] {
        let mut fixture = Fixture::new(foreign);
        if missing { let mut entries = fixture.rows::<ReportEntry>(); entries.pop(); fixture.put_entries(entries); }
        let valid = !foreign && !missing;
        assert_eq!(fixture.check(&TaskReportSupport::invariants()[0]).is_ok(),!foreign);
        assert_eq!(fixture.check(&ReportCollection::invariants()[0]).is_ok(),!missing);
        let generation = store.create_conformance(ContentHash::of(b"deployment-contract"),"catalog").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer,generation,&Batch::new(&model,fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,Package,Release,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,
            CoverageScope,ProviderCoverage,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Evidence,
            ReportCollection,ReportEntry,ReportValue,ReportedEnvironment,TaskReport,TaskReportObservation,TaskReportSupport,DeploymentObservation,DeploymentSupport);
        store.seal(generation).await.unwrap();
        if !valid {
            assert!(matches!(store.validate(generation).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation).await.unwrap(); store.publish(generation).await.unwrap();
            let mut lease = store.pin(&reader,generation, budget()).await.unwrap();
            assert_eq!(lease.read::<TaskReport>().await.unwrap().rows(),fixture.rows::<TaskReport>());
            assert_eq!(lease.read::<TaskReportObservation>().await.unwrap().rows(),fixture.rows::<TaskReportObservation>());
            assert_eq!(lease.read::<ReportedEnvironment>().await.unwrap().rows(),fixture.rows::<ReportedEnvironment>());
            assert_eq!(lease.read::<ReportValue>().await.unwrap().rows(),fixture.rows::<ReportValue>());
            let other_lease = store.pin(&reader,generation, budget()).await.unwrap();
            lease.release().await.unwrap();
            assert!(matches!(store.retire(generation).await,Err(Error::Busy)));
            other_lease.release().await.unwrap();
            store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
