//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[allow(
    dead_code,
    reason = "Shared generated contracts and fixtures require this scoped exception"
)] // Shared fixture has controls used by the lexical suites.
#[path = "../../lctx-model/tests/fixtures/lexical.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, conditions::*, input::*, lexical::*, source::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn coverage_cannot_claim_a_foreign_input_even_with_matching_provider_and_context() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for foreign in [false, true] {
        let mut fixture = Fixture::new();
        if foreign {
            let other = InputRevision::from_entries(vec![]).unwrap();
            let mut inputs = fixture.rows::<InputRevision>();
            inputs.push(other.clone());
            fixture.put(inputs);
            let origin = fixture.rows::<InputOrigin>().pop().unwrap();
            let mut acquisitions = fixture.rows::<InputAcquisition>();
            acquisitions.push(InputAcquisition {
                input: other.id(),
                origin: origin.id(),
            });
            fixture.put(acquisitions);
            let scope = CoverageScope::Input { input: other.id() };
            let mut scopes = fixture.rows::<CoverageScope>();
            scopes.push(scope.clone());
            fixture.put(scopes);
            let mut outcomes = fixture.rows::<ProviderCoverage>();
            outcomes[0].scope = scope.id();
            fixture.put(outcomes);
        }
        // Exercise the same in-memory check before the independent persisted-content execution.
        assert_eq!(
            fixture
                .check(&lctx_model::domain::validation::invariants_for::<ProviderCoverage>()[0])
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
            generation_h.copy(&Batch::new(&model,fixture.rows::<$ty>(), &budget()).unwrap(), &budget()).await.unwrap();
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
            Evidence
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
                lease.read::<BindingObservation>().await.unwrap().rows(),
                fixture.rows::<BindingObservation>()
            );
            assert_eq!(
                lease.read::<LexicalResolution>().await.unwrap().rows(),
                fixture.rows::<LexicalResolution>()
            );
            assert_eq!(
                lease
                    .read::<LexicalScopeObservation>()
                    .await
                    .unwrap()
                    .rows(),
                fixture.rows::<LexicalScopeObservation>()
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
