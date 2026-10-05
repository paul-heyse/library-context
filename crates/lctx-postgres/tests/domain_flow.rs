//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/flow.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, conditions::*, flow::*, input::*, lexical::*,
    source::*, value::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn raw_flow_and_transitive_place_provenance_survive_sealed_validation() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for case in 0..3 {
        let foreign = case == 1;
        let mut fixture = Fixture::new();
        if case == 2 {
            fixture.nested_path();
        } else {
            fixture.extended();
        }
        assert_eq!(fixture.base.rows::<FlowUse>(), vec![fixture.use_.clone()]);
        if foreign {
            fixture.foreign_place();
        }
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(
            fixture
                .base
                .check(&lctx_model::domain::validation::invariants_for::<FlowDefinitionSupport>()[0])
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
            generation_h.copy(&Batch::new(&model,fixture.base.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
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
            LexicalScope,
            BindingEvent,
            LexicalTarget,
            LexicalScopeObservation,
            LexicalScopeSupport,
            BindingObservation,
            BindingSupport,
            ReferenceObservation,
            ReferenceSupport,
            LexicalResolution,
            LexicalResolutionSupport,
            Evidence,
            Module,
            PlaceRoot,
            Place,
            AccessPath,
            FlowUse,
            FlowDefinition,
            ReachingDefinition,
            FlowUseObservation,
            FlowUseSupport,
            FlowDefinitionObservation,
            FlowDefinitionSupport,
            FlowReachingObservation,
            FlowReachingSupport,
            FlowValueObservation,
            FlowValueSupport,
            FlowRegionObservation,
            FlowRegionSupport,
            lctx_model::domain::calls::CallSyntax,
            lctx_model::domain::calls::CallSyntaxSupport,
            lctx_model::domain::calls::CallArgument,
            Predicate,
            EvaluationAtom,
            FlowTestObservation,
            FlowTestSupport,
            FlowTestLeafObservation,
            FlowTestLeafSupport,
            FlowAttributeLoadObservation,
            FlowAttributeLoadSupport,
            FlowCallPath,
            FlowCallStep,
            FlowValuePathObservation,
            FlowValuePathSupport
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
                lease.read::<FlowUse>().await.unwrap().rows(),
                fixture.base.rows::<FlowUse>()
            );
            assert_eq!(
                lease
                    .read::<FlowReachingObservation>()
                    .await
                    .unwrap()
                    .rows(),
                fixture.base.rows::<FlowReachingObservation>()
            );
            assert_eq!(
                lease
                    .read::<FlowDefinitionObservation>()
                    .await
                    .unwrap()
                    .rows(),
                fixture.base.rows::<FlowDefinitionObservation>()
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
