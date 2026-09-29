//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/types.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,lexical::*,types::*,calls::{ProviderModule,ProviderSymbol},value::Literal,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn structural_types_and_recursive_variable_restrictions_roundtrip_without_namespace_erasure() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
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
            store.copy(&writer,generation,&Batch::new(&model,fixture.base.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,ProviderModule,
            CoverageScope,ProviderCoverage,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Occurrence,
            LexicalScope,BindingEvent,LexicalTarget,LexicalScopeObservation,LexicalScopeSupport,BindingObservation,BindingSupport,
            ReferenceObservation,ReferenceSupport,LexicalResolution,LexicalResolutionSupport,Evidence,ProviderSymbol,Literal,TypeVariable,TypeTerm,TypeSequence,TypeSequenceMember,
            TypeObservation,TypeSupport,TypePresentation,TypePresentationSupport,TypeVariableRestriction,TypeRestrictionSupport);
        store.seal(generation).await.unwrap();
        if !valid {
            assert!(matches!(store.validate(generation, &budget()).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation, &budget()).await.unwrap(); store.publish(generation).await.unwrap();
            let mut lease = store.pin(&reader,generation, budget()).await.unwrap();
            assert_eq!(lease.read::<TypeObservation>().await.unwrap().rows(),fixture.base.rows::<TypeObservation>());
            assert_eq!(lease.read::<TypeVariableRestriction>().await.unwrap().rows(),fixture.base.rows::<TypeVariableRestriction>());
            assert_eq!(lease.read::<Literal>().await.unwrap().rows(),fixture.base.rows::<Literal>());
            lease.release().await.unwrap(); store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
