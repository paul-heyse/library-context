#[path = "../../lctx-model/tests/fixtures/stability.rs"] mod fixture;
use std::sync::Arc;
use fixture::Fixture;
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, calls::*, conditions::{*, stability::*}, flow::*, input::*, lexical::*, source::*, value::*};
use lctx_postgres::generations::{GenerationStore, Error};
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
