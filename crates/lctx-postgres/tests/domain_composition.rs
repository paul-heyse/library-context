#[path = "../../lctx-model/tests/fixtures/composition.rs"] mod fixture;
use std::sync::Arc;
use fixture::{Fixture, Mutation};
use lctx_model::domain::{*, artifact::*, assertion::*, attribution::*, calls::*, composition::*, conditions::{*, stability::*}, declarations::*,
    flow::*, input::*, lexical::*, source::*, transfer::*, value::*};
use lctx_postgres::generations::{GenerationStore, Error};
use lctx_postgres::testing::DisposableDatabase;

#[tokio::test]
async fn composed_transfers_round_trip_and_mismatched_steps_refuse() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    for (mutation, reason) in [(None, ""), (Some(Mutation::CalleeFromAnotherSymbol), "differ from the target's symbol"), (Some(Mutation::UnrestatedGuard), "restated at the call"), (Some(Mutation::ErasedCondition), "restated at the call"), (Some(Mutation::ForeignOutput), "not a caller-side place of the call")] {
        let mut fixture = Fixture::new();
        if let Some(mutation) = mutation { fixture.mutate(mutation); }
        let generation = store.create_conformance(ContentHash::of(b"composition-contract"), "behavioral").await.unwrap();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            store.copy(&writer, generation, &Batch::new(&model, fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
        )+ }; }
        copy!(InputRevision, InputOrigin, InputAcquisition, AnalysisContext, Provider, ProviderRun, RunFamily, ProviderSurface, CoverageScope, ProviderCoverage,
            Condition, ConditionNode, AssertionQualification, SourceArtifact, ArtifactChunk, Module, ProviderModule, ProviderSymbol, Occurrence, LexicalScope,
            Literal, PathSegment, PlaceRoot, AccessPath, Place, Predicate, EvaluationAtom, FlowUse, FlowDefinition, ReachingDefinition,
            FlowDefinitionObservation, FlowDefinitionSupport, FlowReachingObservation, FlowReachingSupport, CallSyntax, CallSyntaxSupport, CallArgument,
            Evidence, StabilityWitness, GuardSubstitution, SymbolDeclaration, SymbolDeclarationSupport, ParameterShape, Signature, SignatureParameter,
            SignatureSupport, ParameterDeclaration, ParameterDeclarationSupport, CallDestination, CallChannel, Receiver, CallTarget, CallTargetSupport,
            TransferKey, TransferAlternative, TransferSupport, ControlInfluence, ControlSupport, Selection, CallCompositionStep);
        store.seal(generation).await.unwrap();
        if let Some(mutation) = mutation {
            let error = store.validate(generation, &budget()).await.unwrap_err();
            assert!(matches!(error, Error::Model(_)) && error.to_string().contains(reason), "{mutation:?}: {error}");
            store.abort(generation).await.unwrap();
            continue;
        }
        let digest = store.validate(generation, &budget()).await.unwrap();
        assert_eq!(fixture.validate().unwrap(), digest, "in-memory and stored validation agree");
        store.publish(generation).await.unwrap();
        let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
        let step = fixture.composed.records.step.clone().unwrap();
        assert_eq!(lease.read::<CallCompositionStep>().await.unwrap().rows(), &[step.clone()]);
        assert_eq!(lease.read::<Selection>().await.unwrap().rows(), &[fixture.selection.clone()]);
        let mut keys = lease.read::<TransferKey>().await.unwrap().rows().to_vec(); keys.sort_by_key(Record::id);
        let mut expected = vec![fixture.caller.key().clone(), fixture.callee.key().clone(), fixture.composed.branch.key().clone()]; expected.sort_by_key(Record::id);
        assert_eq!(keys, expected);
        let premises: Vec<(String, Vec<u8>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT role, premise_id FROM {}.derivation_premises WHERE source_relation = '{}' ORDER BY role",
            generation.schema(), CallCompositionStep::NAME))).fetch_all(&reader).await.unwrap();
        assert_eq!(premises, vec![("callee".into(), step.callee.bytes().to_vec()), ("callee_declaration".into(), step.callee_declaration.bytes().to_vec()),
            ("caller".into(), step.caller.bytes().to_vec()), ("signature".into(), step.signature.bytes().to_vec()), ("target".into(), step.target.bytes().to_vec())]);
        lease.release().await.unwrap(); store.retire(generation).await.unwrap();
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(lctx_model::domain::resources::DEFAULT_MEMORY_BYTES).unwrap()
}
