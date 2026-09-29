#[path = "../../lctx-model/tests/fixtures/stability.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, calls::*, conditions::{*, stability::*}, flow::*, input::*, lexical::*, source::*, value::*};
use lctx_postgres::generations::{GenerationStore, Error};
use lctx_postgres::testing::Harness;
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn witnessed_substitutions_round_trip_and_unsubstituted_bound_guards_refuse() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    for unsubstituted in [false, true] {
        let mut fixture = Fixture::new();
        if unsubstituted { fixture.put::<GuardSubstitution>(vec![]); }
        let mut generation_h = Harness::begin(&store, writer.clone(), lctx_model::domain::stages::Profile::Behavioral, budget()).await.unwrap(); let generation = generation_h.generation();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            generation_h.copy(&Batch::new(&model, fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision, InputOrigin, InputAcquisition, AnalysisContext, Provider, ProviderRun, RunFamily, ProviderSurface, CoverageScope, ProviderCoverage,
            Condition, ConditionNode, AssertionQualification, SourceArtifact, ArtifactChunk, Occurrence, LexicalScope, PlaceRoot, AccessPath, Place, Predicate,
            EvaluationAtom, FlowUse, FlowDefinition, ReachingDefinition, FlowDefinitionObservation, FlowDefinitionSupport, FlowReachingObservation,
            FlowReachingSupport, CallSyntax, CallSyntaxSupport, CallArgument, Evidence, StabilityWitness, GuardSubstitution);
        generation_h.seal().await.unwrap();
        if unsubstituted {
            assert!(matches!(generation_h.validate(&budget()).await, Err(Error::Model(_))));
            generation_h.abort().await.unwrap();
        } else {
            let digest = generation_h.validate(&budget()).await.unwrap();
            assert_eq!(fixture.validate().unwrap(), digest, "in-memory and stored validation agree");
            generation_h.publish().await.unwrap();
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
