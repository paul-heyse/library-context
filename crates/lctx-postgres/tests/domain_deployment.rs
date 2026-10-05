//! Shared semantic contract fixtures through permanent PostgreSQL lowering; not a producer test.
#[path = "../../lctx-model/tests/fixtures/deployment.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    artifact::*, assertion::*, attribution::*, conditions::*, deployment::*, input::*, source::*, *,
};
use lctx_postgres::generations::{Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use lctx_postgres::testing::Harness;
use std::sync::Arc;

#[tokio::test]
async fn captured_reports_preserve_values_and_reject_incomplete_or_foreign_evidence() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(ValidatedModel::declared(facts_relations()).unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    for (foreign, missing) in [(false, false), (true, false), (false, true)] {
        let mut fixture = Fixture::new(foreign);
        if missing {
            let mut entries = fixture.rows::<ReportEntry>();
            entries.pop();
            fixture.put_entries(entries);
        }
        let valid = !foreign && !missing;
        assert_eq!(
            fixture.check(&lctx_model::domain::validation::invariants_for::<TaskReportSupport>()[0]).is_ok(),
            !foreign
        );
        assert_eq!(
            fixture.check(&lctx_model::domain::validation::invariants_for::<ReportCollection>()[0]).is_ok(),
            !missing
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
            Package,
            Release,
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
            Evidence,
            ReportCollection,
            ReportEntry,
            ReportValue,
            ReportedEnvironment,
            TaskReport,
            TaskReportObservation,
            TaskReportSupport,
            DeploymentObservation,
            DeploymentSupport
        );
        generation_h.seal().await.unwrap();
        if !valid {
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
                lease.read::<TaskReport>().await.unwrap().rows(),
                fixture.rows::<TaskReport>()
            );
            assert_eq!(
                lease.read::<TaskReportObservation>().await.unwrap().rows(),
                fixture.rows::<TaskReportObservation>()
            );
            assert_eq!(
                lease.read::<ReportedEnvironment>().await.unwrap().rows(),
                fixture.rows::<ReportedEnvironment>()
            );
            assert_eq!(
                lease.read::<ReportValue>().await.unwrap().rows(),
                fixture.rows::<ReportValue>()
            );
            let other_lease = store.pin(&reader, generation, budget()).await.unwrap();
            lease.release().await.unwrap();
            assert!(matches!(store.retire(generation).await, Err(Error::Busy)));
            other_lease.release().await.unwrap();
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
