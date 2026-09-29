//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/flow.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*,artifact::*,assertion::*,attribution::*,conditions::*,input::*,lexical::*,flow::*,value::*,source::*};
use lctx_postgres::generations::{GenerationStore,Error};
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn raw_flow_and_transitive_place_provenance_survive_sealed_validation() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    for foreign in [false,true] {
        let mut fixture = Fixture::new();
        assert_eq!(fixture.base.rows::<FlowUse>(),vec![fixture.use_.clone()]);
        if foreign { fixture.foreign_place(); }
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(fixture.base.check(&FlowDefinitionSupport::invariants()[0]).is_ok(), !foreign);
        let generation = store.create_conformance(ContentHash::of(b"flow-contract"),"catalog").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer,generation,&Batch::new(&model,fixture.base.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision,InputOrigin,InputAcquisition,AnalysisContext,Provider,ProviderRun,RunFamily,ProviderSurface,
            CoverageScope,ProviderCoverage,Condition,ConditionNode,AssertionQualification,SourceArtifact,ArtifactChunk,Occurrence,
            LexicalScope,BindingEvent,LexicalTarget,LexicalScopeObservation,LexicalScopeSupport,BindingObservation,BindingSupport,
            ReferenceObservation,ReferenceSupport,LexicalResolution,LexicalResolutionSupport,Evidence,Module,PlaceRoot,Place,AccessPath,
            FlowUse,FlowDefinition,ReachingDefinition,FlowUseObservation,FlowUseSupport,FlowDefinitionObservation,FlowDefinitionSupport,
            FlowReachingObservation,FlowReachingSupport,FlowValueObservation,FlowValueSupport,FlowRegionObservation,FlowRegionSupport);
        store.seal(generation).await.unwrap();
        if foreign {
            assert!(matches!(store.validate(generation, &budget()).await, Err(Error::Model(_))));
            assert!(store.publish(generation).await.is_err());
            store.abort(generation).await.unwrap();
        } else {
            store.validate(generation, &budget()).await.unwrap(); store.publish(generation).await.unwrap();
            let mut lease = store.pin(&reader,generation, budget()).await.unwrap();
            assert_eq!(lease.read::<FlowUse>().await.unwrap().rows(),fixture.base.rows::<FlowUse>());
            assert_eq!(lease.read::<FlowReachingObservation>().await.unwrap().rows(),fixture.base.rows::<FlowReachingObservation>());
            assert_eq!(lease.read::<FlowDefinitionObservation>().await.unwrap().rows(),fixture.base.rows::<FlowDefinitionObservation>());
            lease.release().await.unwrap(); store.retire(generation).await.unwrap();
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
