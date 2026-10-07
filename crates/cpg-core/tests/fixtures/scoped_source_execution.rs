//! Actual scoped producers compared with a finite independent whole-operation oracle.
use super::runtime;
use cpg_core::{
    compilation::{self, PreparedCompilation},
    workspace::{CompletedInputs, Workspace, WorkspaceOptions},
};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    execution::{
        enriched_production::{BodyBoundary, EnrichedData, ExecutionBoundary, enrich_all},
        source_call_records::{SourceCallData, prepare_all},
    },
    normalized::Rows,
    stages::*,
    *,
};
use std::{fmt::Debug, sync::Arc};
fn identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
async fn rows<R: Record>(
    session: &SessionContext,
    table: &str,
    budget: &resources::ResourceBudget,
) -> Rows<R> {
    let mut output = Rows::new(budget);
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
        output.decode(&batch).unwrap();
    }
    output
}
async fn source_data(
    access: &CompletedInputs,
    session: &SessionContext,
    budget: &resources::ResourceBudget,
) -> SourceCallData {
    let mut data = SourceCallData::new(budget);
    for input in SourceCallData::inputs() {
        let mut stream = cpg_core::sql::query(
            session,
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
    data
}
async fn enriched_data(
    access: &CompletedInputs,
    session: &SessionContext,
    budget: &resources::ResourceBudget,
    profile: Profile,
) -> EnrichedData {
    let mut data = EnrichedData::new(budget);
    for input in EnrichedData::consumed_inputs(profile) {
        let mut stream = cpg_core::sql::query(
            session,
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
    data
}
fn extend<R: Record>(target: &mut Rows<R>, source: &Rows<R>) {
    for row in source.iter() {
        target.insert(row.clone()).unwrap();
    }
}
async fn compare<R: Record + PartialEq + Debug>(
    session: &SessionContext,
    expected: &Rows<R>,
    budget: &resources::ResourceBudget,
) {
    let actual = rows::<R>(session, R::NAME, budget).await;
    assert_eq!(
        actual.iter().collect::<Vec<_>>(),
        expected.iter().collect::<Vec<_>>(),
        "{} differs from whole-operation oracle",
        R::NAME
    );
}
async fn run(profile: Profile) {
    let model = Arc::new(model().unwrap());
    let workspace = Workspace::new(
        model.clone(),
        WorkspaceOptions {
            memory_bytes: 1 << 30,
            partitions: 1,
            batch_rows: 7,
        }, crate::native_fixture::store()
)
    .unwrap();
    let captured = runtime::capture("scoped_source_execution", profile, workspace.budget());
    let configuration = ContentHash::of(b"scoped-source-oracle");
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
        .inputs("actual-output", profile, completed.iter().map(|r| r.name()))
        .unwrap();
    let actual_session = actual.session(&workspace).await.unwrap();
    // Bind each input to the same immutable semantic boundary described by the schedule.
    // This is mechanical selection; the oracle executes the ordinary complete model kernels.
    let selected = |name| {
        let mut stage = schedule
            .stages()
            .iter()
            .find(|stage| stage.name == name)
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
        workspace.stage_inputs(&stage, profile).unwrap()
    };
    let access = selected("prepare_source_calls");
    let source_session = access.session(&workspace).await.unwrap();
    let data = if profile == Profile::Behavioral {
        Some(source_data(&access, &source_session, workspace.budget()).await)
    } else {
        None
    };
    let frames = rows::<analysis::source_call::AnalysisInvocation>(
        &actual_session,
        analysis::source_call::AnalysisInvocation::NAME,
        workspace.budget(),
    )
    .await;
    let mut headers = Rows::new(workspace.budget());
    let mut members = Rows::new(workspace.budget());
    let mut boundaries = Rows::new(workspace.budget());
    let mut calls = Rows::new(workspace.budget());
    let mut releases = Rows::new(workspace.budget());
    let mut arguments = Rows::new(workspace.budget());
    let mut outcomes = Rows::new(workspace.budget());
    let mut invocation_boundaries = Rows::new(workspace.budget());
    let mut runs = Rows::new(workspace.budget());
    let mut results = Rows::new(workspace.budget());
    let empty = SourceCallData::new(workspace.budget());
    for frame in frames.iter() {
        let oracle = prepare_all(
            data.as_ref().unwrap_or(&empty),
            frame,
            &execution::configuration::source_calls().1,
            profile,
            workspace.budget(),
        )
        .unwrap_or_else(|error| panic!("whole SourceCall oracle: {error}"));
        macro_rules! merge{($($target:ident:$field:ident),*)=>{$(extend(&mut $target,&oracle.$field);)*};}
        merge!(headers:headers,members:members,boundaries:boundaries,calls:invocations,releases:releases,arguments:arguments,outcomes:call_outcomes,invocation_boundaries:invocation_boundaries);
        runs.insert(oracle.run).unwrap();
        results.insert(oracle.outcome).unwrap();
    }
    macro_rules! check{($($field:ident),*)=>{$(compare(&actual_session,&$field,workspace.budget()).await;)*};}
    check!(
        headers,
        members,
        boundaries,
        calls,
        releases,
        arguments,
        outcomes,
        invocation_boundaries,
        runs,
        results
    );
    if profile == Profile::Behavioral {
        assert!(
            !headers.is_empty() && !calls.is_empty() && !arguments.is_empty(),
            "fixture must exercise actual SourceCall argument values"
        );
        assert!(
            !boundaries.is_empty(),
            "unsupported default must remain explicit"
        );
    } else {
        assert!(headers.is_empty() && calls.is_empty());
        assert!(
            results
                .iter()
                .all(|row| row.status == analysis::AnalysisStatus::NotRequested)
        );
    }
    drop(data);
    drop(empty);
    drop(access);
    drop(source_session);
    let access = selected("enrich_execution");
    let session = access.session(&workspace).await.unwrap();
    let data = enriched_data(&access, &session, workspace.budget(), profile).await;
    let frames = rows::<analysis::enriched_execution::AnalysisInvocation>(
        &actual_session,
        analysis::enriched_execution::AnalysisInvocation::NAME,
        workspace.budget(),
    )
    .await;
    macro_rules! targets{($($field:ident:$ty:ty,)*)=>{$(let mut $field=Rows::<$ty>::new(workspace.budget());)*};}
    targets!(executions:execution::enriched_records::StatementExecution,outcomes:execution::enriched_records::ExecutionOutcome,sources:execution::enriched_records::ExecutionSource,members:execution::enriched_records::ExecutionMember,entered:execution::enriched_records::EnteredStatement,boundaries:ExecutionBoundary,modeled_calls:execution::modeled_call::ModeledCallEvaluation,modeled_arguments:execution::modeled_call::ModeledCallArgument,modeled_native:execution::modeled_call::ModeledCallNative,fresh_calls:execution::enriched_records::SourceExecutionInvocation,fresh_arguments:execution::enriched_records::SourceExecutionArgument,captured_entries:execution::capture_bridge::CapturedEntryBinding,captured_values:execution::capture_bridge::CapturedValueSource,definition_evaluations:execution::definition::DefinitionEvaluation,definition_sources:execution::definition::DefinitionSource,definition_members:execution::definition::DefinitionMember,contexts:execution::context_execution::ContextExecution,context_items:execution::context_execution::ContextItem,context_sources:execution::context_execution::ContextSource,context_members:execution::context_execution::ContextMember,context_bindings:execution::context_binding::ContextEntryBinding,context_binding_sources:execution::context_binding::BindingSource,context_binding_members:execution::context_binding::BindingMember,bodies:execution::enriched_records::BodyExecution,body_sources:execution::enriched_records::BodySource,body_members:execution::enriched_records::BodyMember,releases:execution::enriched_records::BodyReleaseInput,body_boundaries:BodyBoundary,);
    let mut runs = Rows::new(workspace.budget());
    let mut results = Rows::new(workspace.budget());
    for frame in frames.iter() {
        let definition = data.source.definitions.get(frame.definition).unwrap();
        let oracle = enrich_all(&data, frame, definition, profile, workspace.budget()).unwrap();
        macro_rules! merge{($($field:ident),*)=>{$(extend(&mut $field,&oracle.$field);)*};}
        merge!(
            executions,
            outcomes,
            sources,
            members,
            entered,
            boundaries,
            modeled_calls,
            modeled_arguments,
            modeled_native,
            fresh_calls,
            fresh_arguments,
            captured_entries,
            captured_values,
            definition_evaluations,
            definition_sources,
            definition_members,
            contexts,
            context_items,
            context_sources,
            context_members,
            context_bindings,
            context_binding_sources,
            context_binding_members,
            bodies,
            body_sources,
            body_members,
            releases,
            body_boundaries
        );
        runs.insert(oracle.run).unwrap();
        results.insert(oracle.outcome).unwrap();
    }
    check!(
        executions,
        outcomes,
        sources,
        members,
        entered,
        boundaries,
        modeled_calls,
        modeled_arguments,
        modeled_native,
        fresh_calls,
        fresh_arguments,
        captured_entries,
        captured_values,
        definition_evaluations,
        definition_sources,
        definition_members,
        contexts,
        context_items,
        context_sources,
        context_members,
        context_bindings,
        context_binding_sources,
        context_binding_members,
        bodies,
        body_sources,
        body_members,
        releases,
        body_boundaries,
        runs,
        results
    );
    if profile == Profile::Behavioral {
        assert!(
            !fresh_calls.is_empty() && !fresh_arguments.is_empty() && !executions.is_empty(),
            "fixture must exercise actual SourceCall→Enriched values: fresh_calls={}, fresh_arguments={}, executions={}",
            fresh_calls.len(),
            fresh_arguments.len(),
            executions.len()
        );
    } else {
        assert!(fresh_calls.is_empty() && executions.is_empty());
        assert!(
            results
                .iter()
                .all(|row| row.status == analysis::AnalysisStatus::NotRequested)
        );
    }
}
#[tokio::test]
async fn actual_source_calls_and_enriched_match_finite_whole_operation() {
    run(Profile::Behavioral).await;
}
#[tokio::test]
async fn catalog_profile_never_requires_source_values() {
    run(Profile::Catalog).await;
}
