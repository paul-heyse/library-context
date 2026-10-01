#[path = "fixtures/analysis_support.rs"]
mod stored_analysis;
#[path = "../../lctx-model/tests/fixtures/composition.rs"]
mod fixture;
use fixture::{Fixture, Mutation};
use lctx_model::domain::{
    analysis::{self, local::*},
    artifact::*,
    assertion::*,
    attribution::*,
    calls::*,
    composition::*,
    conditions::{stability::*, *},
    declarations::*,
    flow::*,
    input::*,
    lexical::*,
    source::*,
    transfer::*,
    value::*,
    *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn composed_transfers_round_trip_and_mismatched_steps_refuse() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(stored_analysis::model());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for (mutation, reason) in [
        (None, ""),
        (
            Some(Mutation::CalleeFromAnotherSymbol),
            "differ from the target's symbol",
        ),
        (Some(Mutation::UnrestatedGuard), "restated at the call"),
        (Some(Mutation::ErasedCondition), "restated at the call"),
        (
            Some(Mutation::ForeignOutput),
            "not a caller-side place of the call",
        ),
    ] {
        let mut fixture = Fixture::new();
        if let Some(mutation) = mutation {
            fixture.mutate(mutation);
        }
        let mut generation_h = Harness::begin(
            &store,
            writer.clone(),
            lctx_model::domain::stages::Profile::Behavioral,
            budget(),
        )
        .await
        .unwrap();
        let generation = generation_h.generation();
        let mut native_inventory = analysis::native::NativeInventory::new(&budget());
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(
            stored_analysis::copy(&generation_h, &model, fixture.rows::<$ty>(), &mut native_inventory, &budget()).await.unwrap();
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
            Module,
            ProviderModule,
            ProviderSymbol,
            Occurrence,
            LexicalScope,
            Literal,
            PathSegment,
            PlaceRoot,
            AccessPath,
            Place,
            Predicate,
            EvaluationAtom,
            FlowUse,
            FlowValueObservation,
            FlowValueSupport,
            FlowDefinition,
            ReachingDefinition,
            FlowDefinitionObservation,
            FlowDefinitionSupport,
            FlowReachingObservation,
            FlowReachingSupport,
            CallSyntax,
            CallSyntaxSupport,
            CallArgument,
            Evidence,
            StabilityWitness,
            GuardSubstitution,
            SymbolDeclaration,
            SymbolDeclarationSupport,
            ParameterShape,
            Signature,
            SignatureParameter,
            SignatureSupport,
            ParameterDeclaration,
            ParameterDeclarationSupport,
            CallDestination,
            CallChannel,
            Receiver,
            CallOrigin,
            CallTarget,
            CallTargetSupport,
            TransferKey,
            TransferAlternative,
            TransferSupport,
            ControlInfluence,
            ControlSupport,
            Selection,
            CallCompositionStep,
            analysis::AnalysisDefinition,
            analysis::MethodParameters,
            Invocation,
            ObligationSubject,
            Proposition,
            Derivation,
            AnalysisDerivationPremise,
            SupportSource
        );
        stored_analysis::finish(&generation_h, &model, native_inventory, &budget()).await.unwrap();
        generation_h.seal().await.unwrap();
        if let Some(mutation) = mutation {
            let error = generation_h.validate(&budget()).await.unwrap_err();
            assert!(
                matches!(error, Error::Model(_)) && error.to_string().contains(reason),
                "{mutation:?}: {error}"
            );
            generation_h.abort().await.unwrap();
            continue;
        }
        let digest = generation_h.validate(&budget()).await.unwrap();
        assert_eq!(
            fixture.validate().unwrap(),
            digest,
            "in-memory and stored validation agree"
        );
        generation_h.publish().await.unwrap();
        let mut lease = store.pin(&reader, generation, budget()).await.unwrap();
        let step = fixture.composed.records.step.clone().unwrap();
        assert_eq!(
            lease.read::<CallCompositionStep>().await.unwrap().rows(),
            std::slice::from_ref(&step)
        );
        assert_eq!(
            lease.read::<Selection>().await.unwrap().rows(),
            &[fixture.selection.clone()]
        );
        let mut keys = lease.read::<TransferKey>().await.unwrap().rows().to_vec();
        keys.sort_by_key(Record::id);
        let mut expected = vec![
            fixture.caller.key().clone(),
            fixture.callee.key().clone(),
            fixture.composed.branch.key().clone(),
        ];
        expected.sort_by_key(Record::id);
        assert_eq!(keys, expected);
        let premises: Vec<(String, Vec<u8>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT role, premise_id FROM {}.derivation_premises WHERE source_relation = '{}' ORDER BY role",
            generation.schema(), CallCompositionStep::NAME))).fetch_all(&reader).await.unwrap();
        assert_eq!(
            premises,
            vec![
                ("callee".into(), step.callee.bytes().to_vec()),
                (
                    "callee_declaration".into(),
                    step.callee_declaration.bytes().to_vec()
                ),
                ("caller".into(), step.caller.bytes().to_vec()),
                ("signature".into(), step.signature.bytes().to_vec()),
                ("target".into(), step.target.bytes().to_vec())
            ]
        );
        lease.release().await.unwrap();
        store.retire(generation).await.unwrap();
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
