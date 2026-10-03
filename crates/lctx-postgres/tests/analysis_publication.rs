//! Exercise the actual R1 publication policies at the generation effect boundary.
use lctx_model::domain::{
    analysis::{expected::CoverageAdmission, local, sources::CapturedSources, *},
    attribution::{AnalysisContext, ProviderCoverage},
    input::{ArtifactUse, InputRevision},
    normalized::coverage::{NormalizationComputation, NormalizationCoverage},
    source::{CoverageScope, SourceArtifact},
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationStore, testing::DisposableDatabase};
use std::sync::Arc;

fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
    Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: Profile::ALL.to_vec(),
        effect: Effect::Pure,
        code: ContentHash::of(b"real R1 publication control"),
        configuration: ContentHash::of(b"empty captured input"),
    }
}

#[tokio::test]
async fn admitted_empty_domain_is_explicit_and_raw_coupled_shrink_refuses_before_receipts() {
    for profile in Profile::ALL {
        let db = DisposableDatabase::start().await;
        let model = Arc::new(model().unwrap());
        let store = GenerationStore::install(db.owner.clone(), model.clone())
            .await
            .unwrap();
        let lower = vec![
            RelationUse::of::<InputRevision>(),
            RelationUse::of::<AnalysisContext>(),
            RelationUse::of::<SourceArtifact>(),
            RelationUse::of::<ArtifactUse>(),
            RelationUse::of::<CoverageScope>(),
            RelationUse::of::<ProviderCoverage>(),
            RelationUse::of::<NormalizationComputation>(),
            RelationUse::of::<NormalizationCoverage>(),
            RelationUse::of::<AnalysisDefinition>(),
            RelationUse::of::<MethodParameters>(),
        ];
        let outputs = vec![
            RelationUse::of::<local::Invocation>(),
            RelationUse::of::<local::SourceReceipt>(),
            RelationUse::of::<local::AnalysisInput>(),
            RelationUse::of::<local::ProjectionInput>(),
            RelationUse::of::<local::Outcome>(),
            RelationUse::of::<local::CoverageRequirement>(),
            RelationUse::of::<local::CoverageRequiredSource>(),
            RelationUse::of::<local::Coverage>(),
            RelationUse::of::<local::AnalysisCoveragePremise>(),
            RelationUse::of::<local::CoverageSource>(),
        ];
        let schedule = Schedule::build(
            &model,
            vec![
                stage("captured", vec![], lower.clone()),
                stage(
                    "local",
                    lower
                        .into_iter()
                        .map(RelationUse::completed_store)
                        .collect(),
                    outputs,
                ),
            ],
            &[],
            profile,
        )
        .unwrap();
        let input = InputRevision::from_entries(vec![]).unwrap();
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"cfg"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let scope = CoverageScope::Input { input: input.id() };
        let parameters = MethodParameters {
            depth: None,
            proof_steps: None,
            work: None,
            members: None,
            seed: None,
            iterations: None,
            threshold: None,
            resolution: None,
            damping: None,
            model_catalog: None,
        };
        let definition = AnalysisDefinition {
            method: AnalysisMethod::LocalTransfers,
            semantic_version: ContentHash::of(b"control"),
            parameters: parameters.id(),
            interpretation: Interpretation::Structural,
        };
        for case in 0..3 {
            let budget = resources::ResourceBudget::fixed(128 << 20).unwrap();
            let mut execution = schedule.execute();
            let attempt = store
                .begin_conformance(db.writer.clone(), &mut execution, budget.clone())
                .await
                .unwrap();
            let generation = attempt.generation();
            let mut access = execution.begin("captured").unwrap();
            macro_rules! write {
                ($r:ty, $rows:expr) => {{
                    let batch = Batch::<$r>::new(&model, $rows, &budget).unwrap();
                    access
                        .write::<$r, _>(async |permit| attempt.copy(permit, &batch).await)
                        .await
                        .unwrap();
                }};
            }
            write!(InputRevision, vec![input.clone()]);
            write!(AnalysisContext, vec![context.clone()]);
            write!(SourceArtifact, vec![]);
            write!(ArtifactUse, vec![]);
            write!(CoverageScope, vec![scope.clone()]);
            write!(ProviderCoverage, vec![]);
            write!(NormalizationComputation, vec![]);
            write!(NormalizationCoverage, vec![]);
            write!(AnalysisDefinition, vec![definition.clone()]);
            write!(MethodParameters, vec![parameters.clone()]);
            access
                .complete(&attempt, ProviderOutcome::Complete)
                .await
                .unwrap();
            let mut access = execution.begin("local").unwrap();
            let captured = CapturedSources::capture(&access, &budget).unwrap();
            let mut admission = CoverageAdmission::new(&captured, &budget).unwrap();
            macro_rules! visit {
                ($r:ty, $rows:expr) => {
                    admission
                        .visit(
                            &access.read::<$r>().unwrap(),
                            &<$r as Record>::encode($rows).unwrap(),
                        )
                        .unwrap();
                };
            }
            visit!(InputRevision, std::slice::from_ref(&input));
            visit!(SourceArtifact, &[]);
            visit!(ArtifactUse, &[]);
            visit!(CoverageScope, std::slice::from_ref(&scope));
            visit!(ProviderCoverage, &[]);
            visit!(NormalizationComputation, &[]);
            visit!(NormalizationCoverage, &[]);
            let (invocation, parents, receipts, projections) = local::Invocation::admitted(
                input.id(),
                context.id(),
                definition.id(),
                None,
                [],
                &captured,
                [],
                &budget,
            )
            .unwrap();
            let admitted = local::coverage::admit(
                &invocation,
                &definition,
                AnalysisCapability::Transfers,
                &admission,
                &budget,
            )
            .unwrap();
            assert_eq!(admitted.scopes().len(), 1);
            let domain = &admitted.scopes()[0];
            let (status, reason) = if profile == Profile::Catalog {
                (
                    AnalysisStatus::NotRequested,
                    Some(obligation::ObligationKind::NotRequested),
                )
            } else {
                (AnalysisStatus::Completed, None)
            };
            let (requirement, required) = domain.expectation().records().unwrap();
            let (coverage, premises) = local::coverage::assess(
                domain.expectation(),
                domain.observations(),
                status,
                reason,
                &budget,
            )
            .unwrap();
            assert_eq!(
                coverage.availability,
                if profile == Profile::Catalog {
                    normalized::coverage::EvidenceAvailability::NotRequested
                } else {
                    normalized::coverage::EvidenceAvailability::NoScope
                }
            );
            drop(admission);
            macro_rules! write {
                ($r:ty, $rows:expr) => {{
                    let batch = Batch::<$r>::new(&model, $rows, &budget).unwrap();
                    access
                        .write::<$r, _>(async |permit| attempt.copy(permit, &batch).await)
                        .await
                        .unwrap();
                }};
            }
            write!(local::Invocation, vec![invocation.clone()]);
            write!(local::AnalysisInput, parents);
            write!(local::SourceReceipt, receipts);
            write!(local::ProjectionInput, projections);
            write!(
                local::Outcome,
                vec![local::Outcome {
                    invocation: invocation.id(),
                    status,
                    reason
                }]
            );
            write!(local::CoverageRequirement, vec![requirement]);
            write!(local::CoverageRequiredSource, required);
            write!(local::Coverage, vec![coverage]);
            write!(local::AnalysisCoveragePremise, premises);
            write!(local::CoverageSource, vec![]);
            if case == 1 {
                // Remove the entire expected requirement and result together. Internal consistency
                // alone cannot detect this; admitted captured input membership must be recomputed.
                for relation in [local::CoverageRequirement::NAME, local::Coverage::NAME] {
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "DELETE FROM {}.{relation}",
                        generation.schema()
                    )))
                    .execute(&db.superuser)
                    .await
                    .unwrap();
                }
            } else if case == 2 {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DELETE FROM {}.{} WHERE relation=$1",
                    generation.schema(),
                    local::SourceReceipt::NAME
                )))
                .bind(SourceArtifact::NAME)
                .execute(&db.superuser)
                .await
                .unwrap();
            }
            let result = access.complete(&attempt, ProviderOutcome::Complete).await;
            if case == 0 {
                result.unwrap();
                assert!(execution.finish().is_ok());
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=decode($1,'hex') AND stage_name='local'")
                    .bind(generation.hex()).fetch_one(&db.superuser).await.unwrap();
                assert_eq!(count, 10);
            } else {
                let error = result.unwrap_err().to_string();
                assert!(
                    error.contains(if case == 1 {
                        "expected capability scope requirement"
                    } else {
                        "source receipt membership"
                    }),
                    "{error}"
                );
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_receipts WHERE generation_id=decode($1,'hex') AND stage_name='local'")
                    .bind(generation.hex()).fetch_one(&db.superuser).await.unwrap();
                assert_eq!(count, 0);
                assert!(
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "SELECT * FROM {}.{}",
                        generation.schema(),
                        local::Invocation::NAME
                    )))
                    .fetch_all(&db.writer)
                    .await
                    .is_err()
                );
            }
            attempt.abort().await.unwrap();
        }
    }
}
