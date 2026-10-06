//! A1 owns optional nominal results over stored A0 frames and admitted E1 winning bytes.
use crate::{
    analysis_graphs::PreparedGraphs,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use lctx_model::domain::{
    analysis::{self, analytic as owner},
    analytics::{self as semantic, build, frames},
    stages::*,
    *,
};
use std::sync::Arc;
// Decoder reachability is separate from the model-owned consumed source inventory.
macro_rules! decoder_inputs {($apply:ident)=>{
        lctx_model::normalized_binding_inputs!($apply);
        lctx_model::structural_outputs!($apply);
        $apply! {qualifications:assertion::AssertionQualification,conditions:conditions::Condition,nodes:conditions::ConditionNode,sets:assumptions::AssumptionSet,members:assumptions::AssumptionSetMember,}
        lctx_model::analytic_extra_inputs!($apply);
        lctx_model::analytic_consumption_inputs!($apply);
        lctx_model::projection_outputs!($apply);

};}
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
    let mut data = build::Data::new(runtime.budget());
    let session = access.session(runtime).await?;
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {
            let mut inputs = vec![
                ValidationInput::of::<analysis::settings::AnalyticsConfiguration>(&["id"]),
                ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
                ValidationInput::of::<analysis::MethodParameters>(&["id"]),
                ValidationInput::of::<structural::StructuralFrame>(&["id"]),
                ValidationInput::of::<analysis::structural::Invocation>(&["id"]),
            ];
            for method in build::METHODS {
                inputs.extend(analysis::expected::inputs(method));
            }
            inputs
        },
        runtime.budget(),
    )?;
    macro_rules! read{($($t:ty),*)=>{$(while let Some((input,permit))=consumed.next::<$t>(&access)? {
        if crate::consumed_rows::stream_artifact_admission(&access,&input,&session,&mut admission).await? {continue;}
        crate::consumed_rows::stream_at(&permit,&input,&access,&session,|permit,batch| {
            if [std::any::TypeId::of::<analysis::settings::AnalyticsConfiguration>(),std::any::TypeId::of::<analysis::AnalysisDefinition>(),std::any::TypeId::of::<analysis::MethodParameters>(),std::any::TypeId::of::<structural::StructuralFrame>(),std::any::TypeId::of::<analysis::structural::Invocation>()].contains(&input.type_id()){
                data.visit_input(&input,batch)?;
            }
            admission.visit_if_expected(permit,batch)?;
            Ok(())
        }).await?;
    })*};}
    macro_rules! inventory{($($f:ident:$t:ty,)*)=>{read!($($t),*);};}
    inventory! {settings:analysis::settings::AnalyticsConfiguration,definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,frames:structural::StructuralFrame,parents:analysis::structural::Invocation,}
    lctx_model::expected_domain_inputs!(inventory);
    consumed.finish(access.name())?;
    let settings = data.configuration()?.clone();
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<owner::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;)*};}
    lctx_model::analytic_outputs!(declare);
    output.declare::<assertion::AssertionQualification>()?;
    output.declare::<conditions::Condition>()?;
    output.declare::<conditions::ConditionNode>()?;
    let mut roots = normalized::Rows::new(runtime.budget());
    for frame in data.structural.frames.iter() {
        roots.insert(frame.clone())?;
    }
    let scopes = crate::analytical_scopes::FrameScopes::prepare(
        &access,
        &session,
        model,
        build::Data::consumed_inputs(access.profile()),
        crate::analytical_scopes::Kind::Analytic,
        runtime.budget(),
    )
    .await?;
    drop(session);
    drop(data);
    for sf in roots.iter() {
        let grain = scopes.grain(sf.id(), runtime.budget()).await?;
        let mut data = build::Data::new(runtime.budget());
        // Parent is fixed metadata. Native dependencies can name other provider runs; they
        // cannot widen the E1 consumer's expected invocation domain into another frame.
        let mut parent_rows =
            normalized::Rows::<analysis::structural::Invocation>::new(runtime.budget());
        scopes
            .read::<analysis::structural::Invocation>(&access, &grain, |_, batch| {
                parent_rows.decode(batch)?;
                Ok(())
            })
            .await?;
        let frame_parent = parent_rows
            .get(sf.invocation)
            .ok_or(ModelError::Schema("analytic selected parent"))?
            .clone();
        drop(parent_rows);
        macro_rules! scoped {($($field:ident:$ty:ty,)*)=>{$(scopes.read::<$ty>(&access,&grain,|input,batch|data.visit_frame_input(input,batch,frame_parent.input,frame_parent.context)).await?;)*};}
        decoder_inputs!(scoped);
        drop(grain);
        let mut context = frames::Context::new(runtime.budget());
        let mut receipts = normalized::Rows::new(runtime.budget());
        let mut projections = normalized::Rows::new(runtime.budget());
        let mut results = semantic::Output::new(runtime.budget());
        let parent = data
            .structural_invocations
            .get(sf.invocation)
            .ok_or_else(|| ModelError::Invalid("analytic structural parent absent".into()))?;
        for method in build::METHODS {
            let (parameters, definition) = build::definition(&settings, method)?;
            if data.definitions.get(definition.id()) != Some(&definition)
                || data.parameters.get(parameters.id()) != Some(&parameters)
            {
                return Err(ModelError::Invalid(
                    "analytic canonical definition absent".into(),
                ));
            }
            let parents = frames::parents(&data, sf, method)?;
            let (invocation, inputs, source_rows, projection_rows) = owner::Invocation::admitted(
                parent.input,
                parent.context,
                definition.id(),
                None,
                parents.iter().map(Record::id),
                &sources,
                [analysis::ProjectionDefinition::builtin(
                    projection::ProjectionName::CallableInvocation,
                )
                .id()],
                runtime.budget(),
            )?;
            for p in parents {
                context.sources.insert(p)?;
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
        let frame = semantic::AnalyticFrame {
            structural: sf.id(),
            configuration: settings.id(),
        };
        let graph = graphs.graph(
            &access,
            runtime,
            projection::normalization::ProjectionKey {
                input: parent.input,
                context: parent.context,
                name: projection::ProjectionName::CallableInvocation,
            },
        )?;
        results.extend(crate::stage_runtime::borrowed_cpu(access.name(), || {
            build::produce(&data, &frame, &context.invocations, graph, runtime.budget())
        })?)?;

        macro_rules! write {
            ($t:ty,$rows:expr) => {{
                for row in $rows.iter() {
                    output.push(row.clone()).await?;
                }
            }};
        }
        macro_rules! result{($($f:ident:$t:ty,)*)=>{$(write!($t,results.$f);)*};}
        lctx_model::analytic_outputs!(result);
        write!(assertion::AssertionQualification, results.qualifications);
        if !results.qualifications.is_empty() {
            let (c, nodes) = conditions::Diagram::always().records();
            output.push(c).await?;
            for n in nodes {
                output.push(n).await?;
            }
        }
        write!(owner::Invocation, context.invocations);
        write!(owner::InvocationSource, context.sources);
        write!(owner::AnalysisInput, context.inputs);
        write!(owner::SourceReceipt, receipts);
        write!(owner::ProjectionInput, projections);

        for result in results.results.iter() {
            let invocation = context.invocations.get(result.invocation).unwrap();
            let definition = data.definitions.get(invocation.definition).unwrap();
            let outcome = frames::outcome(result);
            let domain = owner::coverage::admit(
                invocation,
                definition,
                build::capability(result.method)?,
                &admission,
                runtime.budget(),
            )?;
            for scope in domain.scopes() {
                let (r, members) = scope.expectation().records()?;
                output.push(r).await?;
                for r in members {
                    output.push(r).await?;
                }
                for r in scope.observations() {
                    output.push(r.source().clone()).await?;
                }
                let (r, members) = owner::coverage::assess(
                    scope.expectation(),
                    scope.observations(),
                    outcome.status,
                    outcome.reason,
                    runtime.budget(),
                )?;
                output.push(r).await?;
                for r in members {
                    output.push(r).await?;
                }
            }
            output.push(outcome).await?;
        }
        drop(results);
        drop(context);
        drop(data);
        drop(receipts);
        drop(projections);
        tokio::task::yield_now().await;
    }
    drop(roots);
    drop(scopes);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod decoder_tests {
    use super::*;
    #[test]
    fn declared_sources_have_decoder_reachability_in_both_profiles() {
        let mut decoders = std::collections::BTreeSet::new();
        macro_rules! collect {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
        decoder_inputs!(collect);
        for profile in Profile::ALL {
            crate::consumed_rows::assert_decoder_reachability(
                build::Data::consumed_inputs(profile),
                &decoders,
            );
        }
    }
}
