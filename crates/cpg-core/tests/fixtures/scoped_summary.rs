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
pub(crate) async fn rows<R: Record>(
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
    use lctx_tracing::InContext;
    actual_summary_control("scoped_summary_preparation")
        .with_tracing_dispatch(cpg_extract::logging::dispatch())
        .await;
}
#[tokio::test]
async fn actual_recursive_summary_preserves_scc_delta_answers_and_exhaustion() {
    use lctx_tracing::InContext;
    actual_summary_control("scoped_recursive_summary")
        .with_tracing_dispatch(cpg_extract::logging::dispatch())
        .await;
}
async fn actual_summary_control(case: &str) {
    let profile = Profile::Behavioral;
    let model = Arc::new(model().unwrap());
    // Full-frontier functional feedback also retains an independent Summary oracle.
    // Resource refusal and graph-growth behavior have separate small-budget controls.
    let memory_bytes = 4 << 30;
    let native = crate::native_fixture::store_with_limit(memory_bytes);
    let mut workspace_owner = None;
    let result = std::panic::AssertUnwindSafe(async {
    let workspace = Workspace::new(
        model.clone(),
        WorkspaceOptions {
            memory_bytes,
            ..WorkspaceOptions::default()
        },
        native.clone(),
    )
    .unwrap();
    workspace_owner = Some(workspace.clone());
    let captured = runtime::capture(case, profile, workspace.budget());
    let configuration = ContentHash::of(b"scoped-summary-oracle");
    let prepared = PreparedCompilation::new(
        admission::Frontier::Analysis,
        runtime::settings("cases"),
        captured.config().catalog(),
        None,
        workspace.budget(),
    ).map(std::sync::Arc::new)
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
        Some(prepared.clone()),
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
        if case == "scoped_recursive_summary" {
            let entities = rows::<normalized::entities::EntityRef>(&session, "entity_refs", workspace.budget()).await;
            let callables = rows::<normalized::entities::CallableEntity>(&session, "callable_entities", workspace.budget()).await;
            let occurrences = rows::<source::Occurrence>(&session, "occurrences", workspace.budget()).await;
            let source = std::fs::read_to_string(runtime::root(case).join("cases.py")).unwrap();
            for function in ["recurse", "no_base"] {
                let start = source.find(&format!("def {function}(" )).unwrap() as i64;
                let entity = entities.iter().find(|entity| {
                    let normalized::entities::EntityRef::Callable { callable } = entity else { return false; };
                    let Some(normalized::entities::CallableEntity::Source { declaration, .. }) = callables.get(*callable) else { return false; };
                    occurrences.get(*declaration).is_some_and(|occurrence| occurrence.start == start)
                }).unwrap_or_else(|| panic!("source-identified recursive callable missing: {function}"));
                assert!(graph.arcs().any(|arc| arc.source == entity.id() && arc.target == entity.id()),
                    "actual hydrated CallableInvocation projection must retain {function}'s self edge");
            }
        }
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
        "fixture must publish actual finite Summary proofs: Local contributions={}, alternatives={}, invocations={}; Summary origins={}, alternatives={}, witnesses={}, pairs={}, residuals={}",
        data.local_contributions.len(), data.local_alternatives.len(), data.local_invocations.len(),
        expected.origins.len(), expected.alternatives.len(), expected.witnesses.len(),
        expected.pair_outcomes.len(), expected.residuals.len(),
    );
    let fixture = runtime::Fixture { workspace: workspace.clone(), session };
    if case == "scoped_recursive_summary" {
        crate::summary_publication::assert_recursive_summary_answers(&fixture).await;
    } else {
        crate::summary_publication::assert_repeated_prepared_call_pairs(&fixture).await;
    }
    drop(fixture);
    }).catch_unwind().await;
    runtime::complete_workspace(workspace_owner, native, result).await;
}
