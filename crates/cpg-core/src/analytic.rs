//! A1 owns optional nominal results over stored A0 frames and admitted E1 winning bytes.
use crate::producer_operations::{Declaration, declare, declare_ordered, emit};
use crate::{
    analysis_graphs::PreparedGraphs,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::future::BoxFuture;
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
        $apply! {full_values:embedding::value::FullValue,projected_values:embedding::projection::ProjectedValue,}
        lctx_model::projection_outputs!($apply);

};}
type MetadataLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut build::Data,
) -> BoxFuture<'a, Result<(), ModelError>>;

fn read_metadata<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'_>,
    data: &'a mut build::Data,
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
                    std::any::TypeId::of::<analysis::settings::AnalyticsConfiguration>(),
                    std::any::TypeId::of::<analysis::AnalysisDefinition>(),
                    std::any::TypeId::of::<analysis::MethodParameters>(),
                    std::any::TypeId::of::<structural::StructuralFrame>(),
                    std::any::TypeId::of::<analysis::structural::Invocation>(),
                ]
                .contains(&input.type_id())
                {
                    data.visit_input(&input, batch)?;
                }
                admission.visit_if_expected(permit, batch)?;
                Ok(())
            })
            .await?;
        }
        Ok(())
    })
}
fn load_metadata<'a>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'_>,
    data: &'a mut build::Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        const METADATA: &[MetadataLoader] = &[
            read_metadata::<analysis::settings::AnalyticsConfiguration>,
            read_metadata::<analysis::AnalysisDefinition>,
            read_metadata::<analysis::MethodParameters>,
            read_metadata::<structural::StructuralFrame>,
            read_metadata::<analysis::structural::Invocation>,
        ];
        macro_rules! inventory {($($field:ident:$ty:ty,)*) => { const EXPECTED: &[MetadataLoader] = &[$(read_metadata::<$ty>,)*]; };}
        lctx_model::expected_domain_inputs!(inventory);
        for load in METADATA.iter().chain(EXPECTED) {
            load(access, session, consumed, admission, data).await?;
        }
        Ok(())
    })
}
fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        let mut declarations: Vec<Declaration> = Vec::new();
        macro_rules! common {($($record:ident,)*) => { $(declarations.push(declare::<owner::$record>);)* };}
        lctx_model::analysis_publication!(common);
        macro_rules! outputs {($($field:ident:$ty:ty,)*) => { $(declarations.push(declare::<$ty>);)* };}
        lctx_model::analytic_outputs!(outputs);
        declarations.extend([
            declare::<assertion::AssertionQualification> as Declaration,
            declare::<conditions::Condition>,
            declare::<conditions::ConditionNode>,
        ]);
        declare_ordered(output, &declarations).await
    })
}
fn load_frame<'a>(
    scopes: &'a crate::analytical_scopes::FrameScopes,
    access: &'a CompletedInputs,
    grain: &'a crate::consumed_rows::PreparedClosure,
    data: &'a mut build::Data,
    input: Id<input::InputRevision>,
    context: Id<attribution::AnalysisContext>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Loader = for<'a> fn(
        &'a crate::analytical_scopes::FrameScopes,
        &'a CompletedInputs,
        &'a crate::consumed_rows::PreparedClosure,
        &'a mut build::Data,
        Id<input::InputRevision>,
        Id<attribution::AnalysisContext>,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    fn read_frame<'a, R: Record>(
        scopes: &'a crate::analytical_scopes::FrameScopes,
        access: &'a CompletedInputs,
        grain: &'a crate::consumed_rows::PreparedClosure,
        data: &'a mut build::Data,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        scopes.read::<R>(access, grain, move |binding, batch| {
            data.visit_frame_input(binding, batch, input, context)
        })
    }
    let mut loaders: Vec<Loader> = Vec::new();
    macro_rules! inventory {($($field:ident:$ty:ty,)*) => { $(loaders.push(read_frame::<$ty>);)* };}
    decoder_inputs!(inventory);
    Box::pin(async move {
        for load in loaders {
            load(scopes, access, grain, data, input, context).await?;
        }
        Ok(())
    })
}
fn emit_results<'a>(
    rows: &'a semantic::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a semantic::Output,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a semantic::Output, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[Emission] = &[$($field,)*];
    };}
    lctx_model::analytic_outputs!(adapters);
    Box::pin(async move {
        for emission in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}
fn emit_context<'a>(
    rows: &'a frames::Context,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a frames::Context,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident),*) => {
        $(fn $field<'a>(rows: &'a frames::Context, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[Emission] = &[$($field,)*];
    };}
    adapters!(invocations, sources, inputs);
    Box::pin(async move {
        for emission in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}

pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    graphs: Option<&PreparedGraphs>,
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
    load_metadata(&access, &session, &mut consumed, &mut admission, &mut data).await?;
    consumed.finish(access.name())?;
    let settings = data.configuration()?.clone();
    declare_outputs(&output).await?;
    let mut roots = normalized::Rows::new(runtime.budget());
    for frame in data.structural.frames.iter() {
        roots.insert(frame.clone())?;
    }
    let requested = build::requested(&settings);
    let scopes = if requested {
        Some(
            crate::analytical_scopes::FrameScopes::prepare(
                &access,
                &session,
                model,
                build::Data::demanded_inputs(access.profile(), &settings),
                crate::analytical_scopes::Kind::Analytic,
                runtime.budget(),
            )
            .await?,
        )
    } else {
        None
    };
    drop(session);
    let metadata = data;
    for sf in roots.iter() {
        let mut data = build::Data::new(runtime.budget());
        if let Some(scopes) = &scopes {
            let grain = scopes.grain(sf.id(), runtime.budget()).await?;
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
            load_frame(
                scopes,
                &access,
                &grain,
                &mut data,
                frame_parent.input,
                frame_parent.context,
            )
            .await?;
            drop(grain);
        } else {
            data.structural.frames.insert(sf.clone())?;
            macro_rules! metadata {($($field:ident),*)=>{$(for row in metadata.$field.iter(){data.$field.insert(row.clone())?;})*};}
            metadata!(settings, definitions, parameters);
            data.structural_invocations.insert(
                metadata
                    .structural_invocations
                    .get(sf.invocation)
                    .ok_or(ModelError::Schema("analytic selected parent"))?
                    .clone(),
            )?;
        }
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
        let produced = if requested {
            let graph = graphs
                .ok_or(ModelError::Schema("requested analytic graph owner"))?
                .graph(
                    &access,
                    runtime,
                    projection::normalization::ProjectionKey {
                        input: parent.input,
                        context: parent.context,
                        name: projection::ProjectionName::CallableInvocation,
                    },
                )?;
            crate::stage_runtime::borrowed_cpu(access.name(), || {
                build::produce(&data, &frame, &context.invocations, graph, runtime.budget())
            })?
        } else {
            build::not_requested(
                &data,
                &frame,
                &context.invocations,
                runtime.budget(),
                semantic::policy::RETAINED.attributes,
            )?
        };
        results.extend(produced)?;

        emit_results(&results, &output).await?;
        emit(&results.qualifications, &output).await?;
        if !results.qualifications.is_empty() {
            let (c, nodes) = conditions::Diagram::always().records();
            output.push(c).await?;
            for n in nodes {
                output.push(n).await?;
            }
        }
        emit_context(&context, &output).await?;
        emit(&receipts, &output).await?;
        emit(&projections, &output).await?;

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
