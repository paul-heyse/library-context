//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/types.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,lexical::*,types::*,calls::ProviderSymbol,value::Literal,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres,testcontainers::{ImageExt,runners::AsyncRunner}};

#[tokio::test]
async fn structural_types_and_recursive_variable_restrictions_roundtrip_without_namespace_erasure() {
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
    for (foreign,opaque,fidelity) in [(false,false,Fidelity::NativeStructural),(true,false,Fidelity::NativeStructural),
        (false,true,Fidelity::NativeStructural),(false,true,Fidelity::DisplayOnly)] {
        let mut fixture = Fixture::new(foreign);
        if opaque { fixture.opaque(true,true,fidelity); }
        let valid = !foreign && (!opaque || fidelity == Fidelity::DisplayOnly);
        assert!(fixture.base.rows::<TypeTerm>().iter().any(|t| t.id() == fixture.term.id()));
        assert_eq!(fixture.base.rows::<TypeVariable>(),vec![fixture.variable.clone()]);
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(fixture.base.check(&TypeSupport::invariants()[0]).is_ok(), valid);
        let generation = store.create_conformance(ContentHash::of(b"type-contract"),"catalog").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer,generation,&Batch::new(&model,fixture.base.rows::<$ty>()).unwrap()).await.unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,
            CoverageScope,ProviderCoverage,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Occurrence,
            LexicalScope,BindingEvent,LexicalTarget,LexicalScopeObservation,LexicalScopeSupport,BindingObservation,BindingSupport,
            ReferenceObservation,ReferenceSupport,LexicalResolution,LexicalResolutionSupport,Evidence,ProviderSymbol,Literal,TypeVariable,TypeTerm,TypeSequence,TypeSequenceMember,
            TypeObservation,TypeSupport,TypePresentation,TypePresentationSupport,TypeVariableRestriction,TypeRestrictionSupport);
        store.seal(generation).await.unwrap();
        if !valid {
            assert!(matches!(store.validate(generation).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation).await.unwrap(); store.publish(generation).await.unwrap();
            let mut lease = store.pin(&reader,generation).await.unwrap();
            assert_eq!(lease.read::<TypeObservation>().await.unwrap().rows(),fixture.base.rows::<TypeObservation>());
            assert_eq!(lease.read::<TypeVariableRestriction>().await.unwrap().rows(),fixture.base.rows::<TypeVariableRestriction>());
            assert_eq!(lease.read::<Literal>().await.unwrap().rows(),fixture.base.rows::<Literal>());
            drop(lease); store.retire(generation).await.unwrap();
        }
    }
}
