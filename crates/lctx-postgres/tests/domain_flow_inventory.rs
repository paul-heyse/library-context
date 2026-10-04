//! Actual PostgreSQL generation publication exercises the shared per-use replay contract.
#[path = "../../lctx-model/tests/fixtures/flow_inventory.rs"]
mod fixture;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, conditions::*, flow::*, flow_inventory::*, input::*,
    lexical::*, source::*, value::*, *,
};
use lctx_postgres::{
    generations::{Error, GenerationStore},
    testing::{DisposableDatabase, Harness, fixtures::budget},
};
use std::sync::Arc;
#[tokio::test]
async fn per_use_inventory_publishes_truthful_limits_and_refuses_hidden_omission() {
    let db = DisposableDatabase::start().await;
    let model = Arc::new(ValidatedModel::validate(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for case in ["complete", "incomplete", "bound_unattached", "formula", "hidden", "member", "digest"] {
        let mut fixture = if case == "bound_unattached" || case == "formula" {
            fixture::Fixture::bound_unattached()
        } else { fixture::Fixture::new(case != "complete") };
        if case == "formula" {
            let mut state = fixture.candidates[0].state();
            state.reachability = Some(serde_json::from_value(serde_json::to_value([25u8; 16]).unwrap()).unwrap());
            fixture.replace_inventory(&[state], &[]);
        }
        if case == "hidden" {
            fixture.flow.base.put(fixture.candidates[..1].to_vec());
        }
        if case == "member" {
            fixture.flow.base.put(Vec::<FlowUseInventoryMember>::new());
        }
        if case == "digest" {
            let mut wrong = fixture.inventory.clone();
            wrong.candidate_digest = ContentHash::of(b"forged");
            let id = wrong.id();
            let candidates = fixture
                .candidates
                .iter()
                .cloned()
                .map(|mut c| {
                    c.inventory = id;
                    c
                })
                .collect::<Vec<_>>();
            let members = fixture
                .members
                .iter()
                .cloned()
                .map(|mut m| {
                    m.inventory = id;
                    m
                })
                .collect::<Vec<_>>();
            let mut support = fixture.flow.base.rows::<FlowUseInventorySupport>()[0].clone();
            support.assertion = id;
            fixture.flow.base.put(vec![wrong]);
            fixture.flow.base.put(candidates);
            fixture.flow.base.put(members);
            fixture.flow.base.put(vec![support]);
        }
        let fixture = fixture.flow;
        let mut generation_h = Harness::begin(
            &store,
            db.writer.clone(),
            lctx_model::domain::stages::Profile::Catalog,
            budget(),
        )
        .await
        .unwrap();
        let generation = generation_h.generation();
        macro_rules! copy { ($($ty:ty),+ $(,)?) => { $(generation_h.copy(&Batch::new(&model,fixture.base.rows::<$ty>(),&budget()).unwrap(),&budget()).await.unwrap();)+ }; }
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
            lctx_model::domain::assumptions::AssumptionSet,
            lctx_model::domain::assumptions::AssumptionSetMember,
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
            lctx_model::domain::syntax::SubjectBoundary,
            Module,
            PlaceRoot,
            Place,
            AccessPath,
            FlowUseInventoryObservation,
            FlowUseInventorySupport,
            FlowUseCandidate,
            FlowUseInventoryMember,
            FlowSourceViewObservation,
            FlowSourceViewSupport,
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
        if matches!(case, "complete" | "incomplete" | "bound_unattached") {
            generation_h.validate(&budget()).await.unwrap();
            generation_h.publish().await.unwrap();
            let mut lease = store.pin(&db.reader, generation, budget()).await.unwrap();
            let stored = lease.read::<FlowUseInventoryObservation>().await.unwrap();
            assert_eq!(
                stored.rows(),
                fixture.base.rows::<FlowUseInventoryObservation>()
            );
            let candidates = lease.read::<FlowUseCandidate>().await.unwrap();
            assert_eq!(candidates.rows(), fixture.base.rows::<FlowUseCandidate>());
            if case == "bound_unattached" {
                assert_eq!(candidates.rows().len(), 1);
                let candidate = &candidates.rows()[0];
                assert_eq!(candidate.kind, FlowCandidateKind::Bound);
                assert!(candidate.unattached);
                assert_eq!(candidate.mapped_count, 0);
                assert!(candidate.reachability.is_some() && candidate.narrowing.is_some());
                let qualifications = lease.read::<AssertionQualification>().await.unwrap();
                assert!(qualifications.rows().iter().any(|q| Some(q.id()) == candidate.reachability));
                assert!(lease.read::<FlowReachingObservation>().await.unwrap().rows().is_empty());
                assert!(!stored.rows()[0].complete);
            }
            lease.release().await.unwrap();
            store.retire(generation).await.unwrap();
        } else {
            assert!(
                matches!(generation_h.validate(&budget()).await, Err(Error::Model(_))),
                "{case}"
            );
            assert!(generation_h.publish().await.is_err());
            generation_h.abort().await.unwrap();
        }
    }
}
