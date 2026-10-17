//! A0 persists C0-owned public candidates, canonical graph witnesses and exact official usage.
use crate::producer_operations::{self, Declaration};
use crate::{
    analysis_graphs::PreparedGraphs,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::future::BoxFuture;
use lctx_model::domain::{
    analysis::{self, structural as owner},
    stages::*,
    structural::{self as semantic, build, frames},
    *,
};
use std::sync::Arc;
// These types provide decoder reachability; the semantic owner selects their exact input views.
macro_rules! decoder_inputs {
    ($apply:ident, $profile:expr) => {
        lctx_model::projection_inputs!($apply);
        lctx_model::structural_handoff_inputs!($apply);
        if $profile == Profile::Behavioral {
            lctx_model::entry_value_inputs!($apply);
            lctx_model::structural_control_inputs!($apply);
        }
        lctx_model::normalized_event_outputs!($apply);
        // PreparedGraphs retains topology; frame metadata needs assessment IDs only.
        $apply! {
            projection_assessments:projection::ProjectionSourceAssessment,
            uses:input::ArtifactUse,
            members:catalog::CatalogMember,
            callables:catalog::CatalogCallable,
            core_links:catalog::CatalogMemberInvocation,
            core_invocations:analysis::catalog_core::Invocation,
            callable_assessments:normalized::callables::EffectiveCallableAssessment,
            settings:analysis::settings::AnalyticsConfiguration,
            definitions:analysis::AnalysisDefinition,
            parameters:analysis::MethodParameters,
            native_premises:analysis::native::NativeAssertionPremise,
            local:analysis::local::Invocation,
            local_outcomes:analysis::local::AnalysisOutcome,
            local_coverage:analysis::local::AnalysisCoverage,
            assumption_sets:assumptions::AssumptionSet,
            assumption_members:assumptions::AssumptionSetMember,
            assumptions:assumptions::Assumption,
            universes:assumptions::AssumptionUniverse,
            libraries:input::CorpusLibrary,
            distributions:input::InputDistribution,
        }
    };
}
// Initial metadata and expected-domain admission have a distinct decoder inventory from
// the per-parent structural grain. Share the dispatch with its reachability control.
macro_rules! initial_decoder_inputs {
    ($apply:ident) => {
        $apply! {
            runs:attribution::ProviderRun,
            core:analysis::catalog_core::Invocation,
            settings:analysis::settings::AnalyticsConfiguration,
            definitions:analysis::AnalysisDefinition,
            parameters:analysis::MethodParameters,
        }
        lctx_model::expected_domain_inputs!($apply);
    };
}
fn initial_inputs() -> Vec<ValidationInput> {
    let mut inputs = vec![
        ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"]),
        ValidationInput::of::<analysis::settings::AnalyticsConfiguration>(&["id"]),
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ValidationInput::of::<analysis::MethodParameters>(&["id"]),
    ];
    for method in build::methods() {
        inputs.extend(analysis::expected::inputs(method));
    }
    inputs
}
fn load<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    data: &'a mut build::Data,
    context: &'a mut frames::Context,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
                .await?
            {
                continue;
            }
            crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
                if [
                    std::any::TypeId::of::<attribution::ProviderRun>(),
                    std::any::TypeId::of::<analysis::catalog_core::Invocation>(),
                    std::any::TypeId::of::<analysis::settings::AnalyticsConfiguration>(),
                    std::any::TypeId::of::<analysis::AnalysisDefinition>(),
                    std::any::TypeId::of::<analysis::MethodParameters>(),
                ]
                .contains(&input.type_id())
                {
                    data.visit_input(&input, batch)?;
                    context.visit(input.name(), batch)?;
                }
                admission.visit_if_expected(permit, batch)?;
                Ok(())
            })
            .await?;
        }
        Ok(())
    })
}

pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Arc<Workspace>,
    model: &Arc<ValidatedModel>,
    graphs: &Arc<PreparedGraphs>,
) -> Result<(), ModelError> {
    let sources = crate::owned_work::Shared::new(
        analysis::sources::CapturedSources::capture(
            access.profile(),
            access.snapshots(),
            runtime.budget(),
        )?,
        runtime.budget(),
    )?;
    let mut admission = runtime.coverage_admission(&access, &sources).await?;
    let session = access.session(runtime).await?;
    let (data, context) = load_initial(&access, &session, runtime, &mut admission).await?;

    let settings =
        crate::owned_work::Shared::new(context.configuration()?.clone(), runtime.budget())?;
    let parents = frames::parents(&data, runtime.budget())?;
    declare_outputs(&output).await?;
    let scopes = crate::analytical_scopes::FrameScopes::prepare(
        &access,
        &session,
        model,
        build::Data::consumed_inputs(access.profile()),
        crate::analytical_scopes::Kind::Structural,
        runtime.budget(),
    )
    .await?;
    drop(session);
    let definitions = context.definitions;
    drop(data);
    let cancellation = runtime.cancellation();
    let mut readers = Vec::new();
    macro_rules! window_readers {($($field:ident:$ty:ty,)*)=>{$(readers.push(crate::analytical_scopes::window_reader::<$ty>());)*};}
    decoder_inputs!(window_readers, access.profile());
    {
        let mut remaining = parents.iter().peekable();
        while remaining.peek().is_some() {
            let _selected = runtime.budget().reserve(
                "structural-window-roots",
                crate::analytical_scopes::WINDOW_ROOTS
                    * (size_of::<&analysis::catalog_core::Invocation>()
                        + size_of::<Id<analysis::catalog_core::Invocation>>()),
            )?;
            let selected = remaining
                .by_ref()
                .take(crate::analytical_scopes::WINDOW_ROOTS)
                .collect::<Vec<_>>();
            let ids = selected.iter().map(|core| core.id()).collect::<Vec<_>>();
            let window = scopes
                .window(&access, &ids, &readers, runtime.budget(), &cancellation)
                .await?;
            let job_count = selected.len();
            let mut jobs = Vec::new();
            let mut jobs_charge =
                charged::StateCharge::new(runtime.budget(), "prepared work transport slots");
            for (partition, core) in selected.into_iter().enumerate() {
                let (data, mut context) = load_grain(&window, &access, partition, runtime).await?;
                if let Some(domain) = admission.uniform_analysis(
                    analysis::owner::AnalysisOwner::Local,
                    core.input,
                    core.context,
                    &definitions,
                    runtime.budget(),
                )? {
                    context.admit_compact_local(domain)?;
                }
                let core = core.clone();
                let settings = settings.clone();
                let graphs = graphs.clone();
                let access = access.clone();
                let runtime = runtime.clone();
                let sources = sources.clone();
                let job = async move {
                    let (results, receipts, projections) =
                        crate::stage_runtime::borrowed_cpu(access.name(), || {
                            derive_parent(
                                &data,
                                &mut context,
                                &core,
                                &settings,
                                (&graphs, &access),
                                &runtime,
                                &sources,
                            )
                        })?;
                    Ok((context, results, receipts, projections))
                };
                if jobs.is_empty() {
                    jobs_charge.grow(job_count.saturating_mul(std::mem::size_of_val(&job)))?;
                    jobs.reserve_exact(job_count);
                }
                jobs.push(job);
            }
            for (context, results, receipts, projections) in
                crate::owned_work::ordered(runtime, jobs).await?
            {
                emit_results(&results, &output).await?;
                emit_links(&context, &receipts, &projections, &output).await?;

                emit_coverage(&context, &results, &admission, runtime, &output).await?;
                drop(receipts);
                drop(projections);
                drop(results);
                drop(context);
                tokio::task::yield_now().await;
            }
        }
    }
    drop(parents);
    drop(scopes);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}

type InventoryLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a mut build::Data,
    &'a mut frames::Context,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut crate::consumed_rows::ConsumedInputs,
) -> BoxFuture<'a, Result<(), ModelError>>;

fn load_initial<'a, 'sources>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    runtime: &'a Workspace,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
) -> BoxFuture<'a, Result<(build::Data, frames::Context), ModelError>> {
    Box::pin(async move {
        let mut data = build::Data::new(runtime.budget());
        let mut context = frames::Context::new(runtime.budget());
        let mut consumed =
            crate::consumed_rows::ConsumedInputs::new(initial_inputs(), runtime.budget())?;
        macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{{
        const LOADERS: &[InventoryLoader] = &[$(load::<$ty>,)*];
        for loader in LOADERS { loader(access,session,&mut data,&mut context,admission,&mut consumed).await?; }
    }};}
        initial_decoder_inputs!(inventory);
        consumed.finish(access.name())?;

        Ok((data, context))
    })
}

fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! common_publication {($($record:ident,)*)=>{fn common_type(type_id: std::any::TypeId)->bool {false $(||type_id==std::any::TypeId::of::<owner::$record>())*} const COMMON: &[Declaration]=&[$(producer_operations::declare::<owner::$record>,)*]; producer_operations::declare_ordered(output,COMMON).await?;};}
        lctx_model::analysis_publication!(common_publication);
        macro_rules! declare {($($field:ident:$ty:ty,)*)=>{{
        let declarations: Vec<Declaration> = vec![$(producer_operations::declare::<$ty>,)*];
        let kinds = [$(std::any::TypeId::of::<$ty>(),)*];
        for (kind, declaration) in kinds.into_iter().zip(declarations) {
            if !common_type(kind) { declaration(output).await?; }
        }
    }};}
        lctx_model::structural_outputs!(declare);
        producer_operations::declare::<assertion::AssertionQualification>(output).await?;
        producer_operations::declare::<conditions::Condition>(output).await?;
        producer_operations::declare::<conditions::ConditionNode>(output).await?;
        producer_operations::declare::<assumptions::AssumptionSet>(output).await?;
        producer_operations::declare::<assumptions::AssumptionSetMember>(output).await?;
        Ok(())
    })
}

type ScopedLoader = for<'a> fn(
    &'a crate::analytical_scopes::FrameWindow<'a>,
    &'a CompletedInputs,
    usize,
    &'a mut build::Data,
    &'a mut frames::Context,
) -> BoxFuture<'a, Result<(), ModelError>>;

fn load_scoped<'a, R: Record>(
    scopes: &'a crate::analytical_scopes::FrameWindow<'a>,
    _access: &'a CompletedInputs,
    grain: usize,
    data: &'a mut build::Data,
    context: &'a mut frames::Context,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        scopes.read::<R>(grain, move |input, batch| {
            data.visit_input(input, batch)?;
            context.visit(input.name(), batch)?;
            Ok(())
        })
    })
}

fn load_grain<'a>(
    scopes: &'a crate::analytical_scopes::FrameWindow<'a>,
    access: &'a CompletedInputs,
    grain: usize,
    runtime: &'a Workspace,
) -> BoxFuture<'a, Result<(build::Data, frames::Context), ModelError>> {
    Box::pin(async move {
        let mut data = build::Data::new(runtime.budget());
        let mut context = frames::Context::new(runtime.budget());
        let mut groups: Vec<&[ScopedLoader]> = Vec::new();
        macro_rules! scoped {($($field:ident:$ty:ty,)*)=>{{
            const LOADERS: &[ScopedLoader]=&[$(load_scoped::<$ty>,)*];groups.push(LOADERS);
        }};}
        decoder_inputs!(scoped, access.profile());
        for group in groups {
            for loader in group {
                loader(scopes, access, grain, &mut data, &mut context).await?;
            }
        }
        Ok((data, context))
    })
}

type ParentProducts = (
    semantic::Output,
    normalized::Rows<owner::SourceReceipt>,
    normalized::Rows<owner::ProjectionInput>,
);

fn derive_parent(
    data: &build::Data,
    context: &mut frames::Context,
    core: &analysis::catalog_core::Invocation,
    settings: &analysis::settings::AnalyticsConfiguration,
    graph_inputs: (&PreparedGraphs, &CompletedInputs),
    runtime: &Workspace,
    sources: &analysis::sources::CapturedSources,
) -> Result<ParentProducts, ModelError> {
    let (graphs, access) = graph_inputs;
    let mut results = semantic::Output::new(runtime.budget());
    let mut receipts = normalized::Rows::new(runtime.budget());
    let mut projections = normalized::Rows::new(runtime.budget());
    let local = context.local_source(core)?;
    for method in build::methods() {
        let (parameters, definition) = build::definition(settings, method)?;
        if context.definitions.get(definition.id()) != Some(&definition)
            || context.parameters.get(parameters.id()) != Some(&parameters)
        {
            return Err(ModelError::Invalid(
                "structural canonical definition absent".into(),
            ));
        }
        let parents = [
            local.clone(),
            owner::InvocationSource::CatalogCore {
                invocation: core.id(),
            },
        ];
        let mut projection_ids = Vec::new();
        if method == analysis::AnalysisMethod::Delegation {
            projection_ids.extend(
                [
                    projection::ProjectionName::CallableInvocation,
                    projection::ProjectionName::DefinitionContainment,
                ]
                .map(|name| analysis::ProjectionDefinition::builtin(name).id()),
            );
        }
        let (invocation, inputs, source_rows, projection_rows) = owner::Invocation::admitted(
            core.input,
            core.context,
            definition.id(),
            None,
            parents.iter().map(Record::id),
            sources,
            projection_ids,
            runtime.budget(),
        )?;
        for parent in parents {
            context.sources.insert(parent)?;
        }
        for row in inputs {
            context.inputs.insert(row)?;
        }
        for row in source_rows {
            receipts.insert(row)?;
        }
        for row in projection_rows {
            projections.insert(row)?;
        }
        context.invocations.insert(invocation)?;
    }
    let frame = frames::frame(context, core)?;
    let key = |name| projection::normalization::ProjectionKey {
        input: core.input,
        context: core.context,
        name,
    };
    let call = graphs.graph(
        access,
        runtime,
        key(projection::ProjectionName::CallableInvocation),
    )?;
    let definition = graphs.graph(
        access,
        runtime,
        key(projection::ProjectionName::DefinitionContainment),
    )?;
    results.extend(crate::stage_runtime::borrowed_cpu(access.name(), || {
        build::produce(
            data,
            &frame,
            context.invocations.get(frame.invocation).unwrap(),
            settings,
            call,
            definition,
            runtime.budget(),
        )
    })?)?;
    let (condition, nodes) = conditions::Diagram::always().records();
    results.flow_conditions.insert(condition)?;
    for node in nodes {
        results.flow_condition_nodes.insert(node)?;
    }
    Ok((results, receipts, projections))
}

type ResultEmitter =
    for<'a> fn(&'a semantic::Output, &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>>;
fn emit_results<'a>(
    results: &'a semantic::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        producer_operations::emit(&results.conclusion_qualifications, output).await?;
        producer_operations::emit(&results.flow_conditions, output).await?;
        producer_operations::emit(&results.flow_condition_nodes, output).await?;
        producer_operations::emit(&results.flow_assumption_sets, output).await?;
        producer_operations::emit(&results.flow_assumption_members, output).await?;
        macro_rules! result {($($f:ident:$ty:ty,)*)=>{{
            const EMITTERS: &[ResultEmitter]=&[$(|results,output| producer_operations::emit(&results.$f,output),)*];
            for emit in EMITTERS { emit(results,output).await?; }
        }};}
        lctx_model::structural_outputs!(result);
        Ok(())
    })
}
fn emit_links<'a>(
    context: &'a frames::Context,
    receipts: &'a normalized::Rows<owner::SourceReceipt>,
    projections: &'a normalized::Rows<owner::ProjectionInput>,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        producer_operations::emit(&context.invocations, output).await?;
        producer_operations::emit(&context.sources, output).await?;
        producer_operations::emit(&context.inputs, output).await?;
        producer_operations::emit(receipts, output).await?;
        producer_operations::emit(projections, output).await?;
        Ok(())
    })
}

fn emit_coverage<'a, 'sources>(
    context: &'a frames::Context,
    results: &'a semantic::Output,
    admission: &'a analysis::expected::CoverageAdmission<'sources>,
    runtime: &'a Workspace,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let outcomes = semantic::outcomes::derive(context, results, runtime.budget())?;
        for outcome in outcomes.iter() {
            let invocation = context.invocations.get(outcome.invocation).unwrap();
            let definition = context.definitions.get(invocation.definition).unwrap();
            let capability = semantic::outcomes::capability(definition.method)?;
            let status = outcome.status;
            let reason = outcome.reason;
            let domain = owner::coverage::admit(
                invocation,
                definition,
                capability,
                admission,
                runtime.budget(),
            )?;
            for scope in domain.scopes() {
                let (row, members) = scope.expectation().records()?;
                output.push(row).await?;
                for row in members {
                    output.push(row).await?;
                }
                for row in scope.observations() {
                    output.push(row.source().clone()).await?;
                }
                let (row, members) = owner::coverage::assess(
                    scope.expectation(),
                    scope.observations(),
                    status,
                    reason,
                    runtime.budget(),
                )?;
                output.push(row).await?;
                for row in members {
                    output.push(row).await?;
                }
            }
            output
                .push(owner::AnalysisOutcome {
                    invocation: invocation.id(),
                    status,
                    reason,
                })
                .await?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structural_initial_views_have_decoder_reachability() {
        let mut decoders = std::collections::BTreeSet::new();
        macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
        initial_decoder_inputs!(inventory);
        crate::consumed_rows::assert_decoder_reachability(initial_inputs(), &decoders);
    }

    #[test]
    fn structural_declared_views_have_profile_decoder_reachability() {
        for profile in [Profile::Catalog, Profile::Behavioral] {
            let mut decoders = std::collections::BTreeSet::new();
            macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
            decoder_inputs!(inventory, profile);
            crate::consumed_rows::assert_decoder_reachability(
                build::Data::consumed_inputs(profile),
                &decoders,
            );
        }
    }
}
