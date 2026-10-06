//! Actual property-only Summary producer compared with the complete finite ordinary kernel.
use super::runtime;
use cpg_core::{
    compilation::{self, PreparedCompilation},
    workspace::{Workspace, WorkspaceOptions},
};
use futures::TryStreamExt;
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
    let profile = Profile::Behavioral;
    let model = Arc::new(model().unwrap());
    let workspace = Workspace::new(
        model.clone(),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            partitions: 1,
            batch_rows: 7,
        },
    )
    .unwrap();
    let captured = runtime::capture("scoped_source_execution", profile, workspace.budget());
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
}
