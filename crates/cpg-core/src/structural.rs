//! A0 persists C0-owned public candidates, canonical graph witnesses and exact official usage.
use crate::{
    analysis_graphs::PreparedGraphs,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
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
fn consumed_inputs(profile: Profile) -> Vec<ValidationInput> {
    let mut declarations = build::Data::consumed_inputs(profile);
    for method in build::methods() {
        declarations.extend(analysis::expected::inputs(method));
    }
    declarations
}
async fn load<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    data: &mut build::Data,
    context: &mut frames::Context,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
    consumed: &mut crate::consumed_rows::ConsumedInputs,
) -> Result<(), ModelError> {
    while let Some((input, permit)) = consumed.next::<R>(access)? {
        crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
            if [std::any::TypeId::of::<attribution::ProviderRun>(),std::any::TypeId::of::<analysis::catalog_core::Invocation>(),std::any::TypeId::of::<analysis::settings::AnalyticsConfiguration>(),std::any::TypeId::of::<analysis::AnalysisDefinition>(),std::any::TypeId::of::<analysis::MethodParameters>()].contains(&input.type_id()){
                data.visit_input(&input, batch)?;
                context.visit(input.name(), batch)?;
            }
            admission.visit_if_expected(permit, batch)?;
            Ok(())
        })
        .await?;
    }
    Ok(())
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    graphs: &PreparedGraphs,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let session = access.session(runtime).await?;
    let mut data = build::Data::new(runtime.budget());
    let mut context = frames::Context::new(runtime.budget());
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {let mut inputs=vec![ValidationInput::of::<attribution::ProviderRun>(&["id"]),ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"]),ValidationInput::of::<analysis::settings::AnalyticsConfiguration>(&["id"]),ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),ValidationInput::of::<analysis::MethodParameters>(&["id"])];for method in build::methods(){inputs.extend(analysis::expected::inputs(method));}inputs},
        runtime.budget(),
    )?;
    macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut data,&mut context,&mut admission,&mut consumed).await?;)*};}
    inventory! {runs:attribution::ProviderRun,core:analysis::catalog_core::Invocation,settings:analysis::settings::AnalyticsConfiguration,definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,}
    lctx_model::expected_domain_inputs!(inventory);
    consumed.finish(access.name())?;

    let settings = context.configuration()?.clone();
    let parents = frames::parents(&data, runtime.budget())?;
    macro_rules! common_publication {($($record:ident,)*)=>{fn common_type(type_id: std::any::TypeId)->bool {false $(||type_id==std::any::TypeId::of::<owner::$record>())*} $(output.declare::<owner::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(if !common_type(std::any::TypeId::of::<$ty>()){output.declare::<$ty>()?;})*};}
    lctx_model::structural_outputs!(declare);
    output.declare::<assertion::AssertionQualification>()?;
    output.declare::<conditions::Condition>()?;
    output.declare::<conditions::ConditionNode>()?;
    output.declare::<assumptions::AssumptionSet>()?;
    output.declare::<assumptions::AssumptionSetMember>()?;
    let scopes=crate::analytical_scopes::FrameScopes::prepare(&access,&session,model,build::Data::consumed_inputs(access.profile()),crate::analytical_scopes::Kind::Structural,runtime.budget()).await?;
    drop(session);
    drop(context);
    drop(data);
    for core in parents.iter() {
        let grain=scopes.grain(core.id(),runtime.budget()).await?;
        let mut data=build::Data::new(runtime.budget());
        let mut context=frames::Context::new(runtime.budget());
        macro_rules! scoped {($($field:ident:$ty:ty,)*)=>{$(scopes.read::<$ty>(&access,&grain,|input,batch|{data.visit_input(input,batch)?;context.visit(input.name(),batch)?;Ok(())}).await?;)*};}
        decoder_inputs!(scoped,access.profile());
        drop(grain);
        let mut results=semantic::Output::new(runtime.budget());
        let mut receipts=normalized::Rows::new(runtime.budget());
        let mut projections=normalized::Rows::new(runtime.budget());
        let local = context.local_parent(core)?.id();
        for method in build::methods() {
            let (parameters, definition) = build::definition(&settings, method)?;
            if context.definitions.get(definition.id()) != Some(&definition)
                || context.parameters.get(parameters.id()) != Some(&parameters)
            {
                return Err(ModelError::Invalid(
                    "structural canonical definition absent".into(),
                ));
            }
            let parents = [
                owner::InvocationSource::Local { invocation: local },
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
                &sources,
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
        let frame = frames::frame(&context, core)?;
        let key = |name| projection::normalization::ProjectionKey {
            input: core.input,
            context: core.context,
            name,
        };
        let call = graphs.graph(
            &access,
            runtime,
            key(projection::ProjectionName::CallableInvocation),
        )?;
        let definition = graphs.graph(
            &access,
            runtime,
            key(projection::ProjectionName::DefinitionContainment),
        )?;
        results.extend(crate::stage_runtime::borrowed_cpu(access.name(), || {
            build::produce(
                &data,
                &frame,
                context.invocations.get(frame.invocation).unwrap(),
                &settings,
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
    macro_rules! write {
        ($ty:ty,$rows:expr) => {{
            for row in $rows.iter() {
                output.push(row.clone()).await?;
            }
        }};
    }
    macro_rules! result {($($f:ident:$ty:ty,)*)=>{$(write!($ty,results.$f);)*};}
    write!(
        assertion::AssertionQualification,
        results.conclusion_qualifications
    );
    write!(conditions::Condition, results.flow_conditions);
    write!(conditions::ConditionNode, results.flow_condition_nodes);
    write!(assumptions::AssumptionSet, results.flow_assumption_sets);
    write!(
        assumptions::AssumptionSetMember,
        results.flow_assumption_members
    );
    lctx_model::structural_outputs!(result);
    write!(owner::Invocation, context.invocations);
    write!(owner::InvocationSource, context.sources);
    write!(owner::AnalysisInput, context.inputs);
    write!(owner::SourceReceipt, receipts);
    write!(owner::ProjectionInput, projections);

    let outcomes = semantic::outcomes::derive(&context, &results, runtime.budget())?;
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
            &admission,
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
    drop(receipts);
    drop(projections);
    drop(results);
    drop(context);
    drop(data);
    tokio::task::yield_now().await;
    }
    drop(parents);
    drop(scopes);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structural_declared_views_have_profile_decoder_reachability() {
        for profile in [Profile::Catalog, Profile::Behavioral] {
            let mut decoders = std::collections::BTreeSet::new();
            macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
            decoder_inputs!(inventory, profile);
            crate::consumed_rows::assert_decoder_reachability(consumed_inputs(profile), &decoders);
        }
    }
}
