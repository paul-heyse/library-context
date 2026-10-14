//! Actual property-only Summary producer compared with the complete finite ordinary kernel.
use crate::catalog_runtime as runtime;
use cpg_core::{
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use futures::{FutureExt, TryStreamExt};
use lctx_model::domain::{execution::summary_production::*, normalized::Rows, stages::*, *};
use std::{fmt::Debug, sync::Arc};
fn identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
async fn rows<R: Record>(
    session: &datafusion::prelude::SessionContext,
    table: &str,
    budget: &resources::ResourceBudget,
) -> Rows<R> {
    let mut rows = Rows::new(budget);
    let mut stream = cpg_core::sql::query(
        session,
        &format!("SELECT * FROM {} ORDER BY id", identifier(table)),
    )
    .await
    .unwrap()
    .execute_stream()
    .await
    .unwrap();
    while let Some(batch) = stream.try_next().await.unwrap() {
        rows.decode(&batch).unwrap();
    }
    rows
}
async fn compare<R: Record + PartialEq + Debug>(
    session: &datafusion::prelude::SessionContext,
    expected: &Rows<R>,
    budget: &resources::ResourceBudget,
) {
    let actual = rows::<R>(session, R::NAME, budget).await;
    assert_eq!(
        actual.iter().collect::<Vec<_>>(),
        expected.iter().collect::<Vec<_>>(),
        "{} differs from complete finite Summary oracle",
        R::NAME
    );
}
#[tokio::test]
async fn actual_summary_matches_complete_finite_whole_operation() {
    use tracing::instrument::WithSubscriber;
    actual_summary_control()
        .with_subscriber(cpg_extract::logging::dispatch())
        .await;
}
async fn actual_summary_control() {
    let profile = Profile::Behavioral;
    let model = Arc::new(model().unwrap());
    let native = crate::native_fixture::store();
    let mut workspace_owner = None;
    let result = std::panic::AssertUnwindSafe(async {
    let workspace = Workspace::new(
        model.clone(),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            ..WorkspaceOptions::default()
        },
        native.clone(),
    )
    .unwrap();
    workspace_owner = Some(workspace.clone());
    let captured = runtime::capture("scoped_summary_preparation", profile, workspace.budget());
    let configuration = ContentHash::of(b"scoped-summary-oracle");
    let prepared = PreparedCompilation::new(
        admission::Frontier::Analysis,
        runtime::settings("cases"),
        captured.config().catalog(),
        None,
        workspace.budget(),
    )
    .unwrap();
    let schedule = prepared
        .schedule(&model, &cpg_core::facts::providers(configuration), profile)
        .unwrap();
    compilation::compile(
        &workspace,
        captured,
        profile,
        configuration,
        admission::Frontier::Analysis,
        Some(&prepared),
        None,
        None,
    )
    .await
    .unwrap();
    workspace.admit_semantics(profile).await.unwrap();
    let completed = workspace.completed_relations().unwrap();
    let actual = workspace
        .inputs(
            "summary-output",
            profile,
            completed.iter().map(|row| row.name()),
        )
        .unwrap();
    let session = actual.session(&workspace).await.unwrap();
    let mut stage = schedule
        .stages()
        .iter()
        .find(|stage| stage.name == "summary")
        .or_else(|| {
            schedule.stages().iter().find(|stage| {
                stage
                    .outputs
                    .iter()
                    .any(|output| output.name() == SummaryRun::NAME)
            })
        })
        .unwrap()
        .clone();
    for input in &mut stage.inputs {
        if input.prefix().is_none()
            && is_vocabulary(input.name())
            && let Some(group) = schedule.publication_groups().iter().find(|group| {
                schedule.stages().iter().any(|producer| {
                    group.stages.contains(&producer.name)
                        && producer
                            .outputs
                            .iter()
                            .any(|output| output.name() == input.name())
                })
            })
        {
            *input = input.at_epoch(group.epoch);
        }
    }
    let mut aliases = std::collections::BTreeSet::new();
    stage
        .inputs
        .retain(|input| aliases.insert((input.name(), input.prefix())));
    let access = workspace.stage_inputs(&stage, profile).unwrap();
    let inputs = access.session(&workspace).await.unwrap();
    let mut data = SummaryData::new(workspace.budget());
    for input in SummaryData::inputs() {
        let mut stream = cpg_core::sql::query(
            &inputs,
            &format!(
                "SELECT * FROM {} ORDER BY id",
                identifier(&access.table_for(&input).unwrap())
            ),
        )
        .await
        .unwrap()
        .execute_stream()
        .await
        .unwrap();
        while let Some(batch) = stream.try_next().await.unwrap() {
            data.visit_input(&input, &batch).unwrap();
        }
    }
    let frames = rows::<analysis::summary::AnalysisInvocation>(
        &session,
        analysis::summary::AnalysisInvocation::NAME,
        workspace.budget(),
    )
    .await;
    macro_rules! targets{($($field:ident:$ty:ty,)*)=>{
  struct Expected {$( $field:Rows<$ty>, )*}
  impl Expected {fn new(budget:&resources::ResourceBudget)->Self{Self{$($field:Rows::new(budget),)*}}}
 };}
    lctx_model::summary_outputs!(targets);
    let mut expected = Expected::new(workspace.budget());
    let mut outcomes = Rows::new(workspace.budget());
    let coverage = rows::<analysis::summary::AnalysisCoverage>(
        &session,
        analysis::summary::AnalysisCoverage::NAME,
        workspace.budget(),
    )
    .await;
    for frame in frames.iter() {
        let definition = data.definitions.get(frame.definition).unwrap();
        let assessment = data
            .graphs
            .assessments
            .iter()
            .find(|row| {
                (row.input, row.context, row.projection)
                    == (
                        frame.input,
                        frame.context,
                        projection::ProjectionName::CallableInvocation,
                    )
            })
            .unwrap();
        let snapshot = data
            .graphs
            .snapshots
            .iter()
            .find(|row| row.assessment == assessment.id())
            .unwrap();
        let graph = projection::snapshot::hydrate(
            snapshot,
            assessment,
            &data.graphs.chunks,
            workspace.budget(),
        )
        .unwrap();
        let mut oracle = produce(
            &data,
            frame,
            definition,
            profile,
            Some(&graph),
            workspace.budget(),
        )
        .unwrap();
        oracle.discharge(&coverage).unwrap();
        macro_rules! merge{($($field:ident:$ty:ty,)*)=>{$(for row in oracle.$field.iter(){expected.$field.insert(row.clone()).unwrap();})*};}
        lctx_model::summary_outputs!(merge);
        outcomes.insert(oracle.outcome).unwrap();
    }
    macro_rules! check{($($field:ident:$ty:ty,)*)=>{$(compare(&session,&expected.$field,workspace.budget()).await;)*};}
    lctx_model::summary_outputs!(check);
    compare(&session, &outcomes, workspace.budget()).await;
    assert!(
        !expected.components.is_empty() && !expected.component_members.is_empty(),
        "fixture must traverse actual invocation SCC topology"
    );
    assert!(
        !expected.origins.is_empty() && !expected.alternatives.is_empty(),
        "fixture must publish actual finite Summary proofs"
    );
    crate::summary_publication::assert_repeated_prepared_call_pairs(&runtime::Fixture {
        workspace: workspace.clone(),
        session,
    }).await;
    }).catch_unwind().await;
    // This control owns the original compiler attempt, including its session.
    // Drain actual execution before abandoning it and settle the workspace signal
    // using the combined report, also when compilation or oracle assertions panic.
    use lctx_model::domain::completion::{self, LocalState, RemoteState, StorageState};
    let mut finality = match &workspace_owner {
        Some(workspace) => workspace.drain_owned_report().await,
        None => completion::Completion::default(),
    };
    if finality.local == LocalState::Terminal
        && finality.remote == RemoteState::Confirmed
        && finality
            .failures
            .iter()
            .all(|failure| failure.error.permits_storage_cleanup())
    {
        let mut source = native.abandon_finality_report().await;
        if source.local == LocalState::Outstanding {
            finality.local = LocalState::Outstanding;
        }
        if source.remote == RemoteState::Unknown {
            finality.remote = RemoteState::Unknown;
        }
        finality.storage.append(&mut source.storage);
        finality.committed.append(&mut source.committed);
        finality.failures.append(&mut source.failures);
    } else {
        finality.storage.push(StorageState::Orphan(format!(
            "native_attempt:{}",
            native.attempt().hex()
        )));
    }
    if let Some(workspace) = &workspace_owner {
        let signal = workspace.complete_signal(&finality).await;
        finality.step("scoped Summary workspace signal", signal);
    }
    let mut diagnostics = completion::Completion::default();
    if let Some(workspace) = &workspace_owner {
        workspace.append_operation_failures(&mut diagnostics);
    } else {
        native.append_operation_failures(&mut diagnostics);
    }
    if let Err(panic) = result {
        if let Err(error) = completion::complete(Ok(()), finality) {
            eprintln!("scoped Summary finalization after assertion panic: {error:#?}");
        }
        if !diagnostics.failures.is_empty() {
            eprintln!(
                "scoped Summary operation diagnostics after assertion panic: {diagnostics:#?}"
            );
        }
        std::panic::resume_unwind(panic);
    }
    completion::complete(Ok(()), finality).unwrap();
    completion::complete(Ok(()), diagnostics).unwrap();
}
