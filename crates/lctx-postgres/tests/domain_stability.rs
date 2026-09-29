#[path = "../../lctx-model/tests/fixtures/stability.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, calls::*, conditions::{*, stability::*}, flow::*, input::*, lexical::*, source::*, value::*};
use lctx_postgres::generations::{GenerationStore, Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::{ImageExt, runners::AsyncRunner}};

#[tokio::test]
async fn witnessed_substitutions_round_trip_and_unsubstituted_bound_guards_refuse() {
    let (image, tag) = lctx_postgres::serving::TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default().with_name(image).with_tag(tag).start().await.expect("Docker and pinned PG18 image required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
    let owner = PgPool::connect(&url("postgres")).await.unwrap();
    sqlx::raw_sql("CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'").execute(&owner).await.unwrap();
    let writer = PgPool::connect(&url("lctx_importer")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving")).await.unwrap();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(owner, model.clone()).await.unwrap();
    for unsubstituted in [false, true] {
        let mut fixture = Fixture::new();
        if unsubstituted { fixture.put::<GuardSubstitution>(vec![]); }
        let generation = store.create_conformance(ContentHash::of(b"stability-contract"), "behavioral").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer, generation, &Batch::new(&model, fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision, InputOrigin, InputAcquisition, AnalysisContext, Provider, ProviderRun, RunFamily, ProviderSurface, CoverageScope, ProviderCoverage,
            Condition, ConditionNode, AssertionQualification, SourceArtifact, ArtifactChunk, Occurrence, LexicalScope, PlaceRoot, AccessPath, Place, Predicate,
            EvaluationAtom, FlowUse, FlowDefinition, ReachingDefinition, FlowDefinitionObservation, FlowDefinitionSupport, FlowReachingObservation,
            FlowReachingSupport, CallSyntax, CallSyntaxSupport, CallArgument, Evidence, StabilityWitness, GuardSubstitution);
        store.seal(generation).await.unwrap();
        if unsubstituted {
            assert!(matches!(store.validate(generation, &budget()).await, Err(Error::Model(_))));
            store.abort(generation).await.unwrap();
        } else {
            let digest = store.validate(generation, &budget()).await.unwrap();
            assert_eq!(fixture.validate().unwrap(), digest, "in-memory and stored validation agree");
            store.publish(generation).await.unwrap();
            let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
            assert_eq!(lease.read::<GuardSubstitution>().await.unwrap().rows(), fixture.rows::<GuardSubstitution>());
            assert_eq!(lease.read::<StabilityWitness>().await.unwrap().rows(), fixture.rows::<StabilityWitness>());
            lease.release().await.unwrap(); store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
