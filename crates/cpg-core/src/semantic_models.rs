//! Model-owned analysis over completed native/normalized/Enriched inputs.
use crate::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, StageSession},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, model as owner, sources::CapturedSources},
    execution::{model_production::*, model_rules::*},
    normalized::Rows,
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;
async fn load<R: Record>(
    access: &StageAccess<'_, '_>,
    reader: &AttemptSession,
    session: &StageSession,
    registered: &mut charged::ChargedSet<&'static str>,
    charge: &mut charged::StateCharge,
    admission: &mut CoverageAdmission<'_>,
    mut visit: impl FnMut(&ReadPermit<'_, R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let permit = access.read::<R>()?;
    if registered.insert(charge, permit.relation())? {
        session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
    }
    let query = session
        .query(&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        admission.visit_if_expected(&permit, &batch)?;
        visit(&permit, &batch)?;
    }
    Ok(())
}
pub async fn apply(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    roles: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<(), ModelError> {
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(&access, budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let reader = AttemptSession::open(
        roles,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut registered = charged::ChargedSet::default();
    let mut registration = charged::StateCharge::new(budget, "model_registration");
    let mut data = ModelData::new(budget);
    macro_rules! inputs{($($field:ident:$ty:ty,)*)=>{$(if !registered.contains(<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,&mut admission,|_,batch|data.visit(<$ty>::NAME,batch)).await?;})*};}
    if profile == Profile::Behavioral {
        lctx_model::normalized_binding_inputs!(inputs);
        lctx_model::normalized_binding_outputs!(inputs);
        lctx_model::model_pin_inputs!(inputs);
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
    }
    macro_rules! read{($($ty:ty),*)=>{$(if !registered.contains(<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,&mut admission,|_,batch|data.visit(<$ty>::NAME,batch)).await?;})*};}
    read!(
        models::ModelCatalog,
        analysis::MethodParameters,
        analysis::AnalysisDefinition,
        analysis::enriched_execution::AnalysisInvocation,
        analysis::source_call::AnalysisInvocation,
        analysis::local::AnalysisInvocation,
        attribution::ProviderRun
    );
    if profile == Profile::Behavioral {
        read!(
            analysis::native::NativeQualification,
            analysis::native::NativeAssertionPremise
        );
    }
    if profile == Profile::Behavioral {
        read!(
            conditions::entry::EntryValueWitness,
            conditions::entry::EntryAccessSource,
            execution::records::ExpressionEvaluation,
            execution::records::EvaluationSource,
            execution::records::EvaluationMember,
            execution::records::EvaluationOperand,
            analysis::base_evaluation::AnalysisInvocation,
            execution::body_records::SourceBodyCompletion,
            analysis::base_completion::AnalysisInvocation,
            execution::source_call_records::SourceCallHeader,
            execution::source_call_records::HeaderMember,
            execution::source_call_records::SourceCallBoundary,
            execution::source_call_records::SourceCallRun,
            execution::source_call_records::SourceInvocation,
            execution::source_call_records::SourceFrameRelease,
            execution::source_call_records::SourceFrameArgument,
            syntax::ParameterSyntaxObservation,
            execution::source_call_records::SourceCallOutcome,
            execution::source_call_records::InvocationBoundary,
            analysis::source_call::AnalysisOutcome,
            execution::modeled_call::ModeledCallEvaluation,
            execution::modeled_call::ModeledCallArgument,
            execution::modeled_call::ModeledCallNative,
            execution::context_binding::ContextEntryBinding,
            execution::context_binding::BindingSource,
            execution::context_binding::BindingMember,
            execution::context_execution::ContextExecution,
            execution::context_execution::ContextItem,
            execution::context_execution::ContextSource,
            execution::context_execution::ContextMember,
            execution::enriched_records::ExecutionOutcome
        );
    }
    if data.definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "Model definition absent from confirmed configuration".into(),
        ));
    }
    macro_rules! expected{($($field:ident:$ty:ty,)*)=>{$(if access.stage().reads::<$ty>() && !registered.contains(<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,&mut admission,|_,_|Ok(())).await?;})*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let mut output = StageOutput::new(access, attempt, model, budget.clone(), Default::default())?;
    macro_rules! declare{($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    declare!(
        owner::AnalysisInvocation,
        owner::AnalysisInput,
        owner::SourceReceipt,
        owner::ProjectionInput,
        owner::InvocationSource,
        owner::AnalysisOutcome,
        owner::AnalysisCoverage,
        owner::CoverageRequirement,
        owner::CoverageRequiredSource,
        owner::AnalysisCoveragePremise,
        owner::CoverageSource,
        ModelRun,
        assumptions_universe::AssumptionUniverseSupport,
        assumptions::AssumptionSet,
        assumptions::AssumptionSetMember,
        assumptions::Assumption,
        assumptions::AssumptionUniverse,
        ModelApplication,
        ApplicationPremise,
        ApplicationBoundary,
        TargetAssessment,
        AppliedRule,
        ChannelAssessment,
        ModeledOperation,
        ResourceIdentity,
        ModelValuePath,
        ActionAssessment,
        ActionSource,
        ActionPostcondition,
        execution::model_protocol::ContextResource,
        execution::model_protocol::ContextEntryValue,
        execution::model_protocol::ContextPostcondition,
        execution::model_context_transfer::ContextTransferWitness,
        execution::model_transfer::ModelTransferWitness,
        transfer::model::TransferKey,
        transfer::model::TransferAlternative,
        transfer::model::TransferSupport,
        value::PlaceRoot,
        value::Place,
        assertion::AssertionQualification,
        conditions::Condition,
        conditions::ConditionNode,
        owner::ObligationSubject,
        owner::SupportSource,
        owner::AnalysisDerivation,
        owner::AnalysisProposition,
        owner::AnalysisDerivationPremise
    );
    let mut frames = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(budget, "model_frames");
    for frame in data.early.bindings.runs.iter() {
        if !frames.insert(&mut charge, (frame.input, frame.context))? {
            continue;
        }
        if data
            .enriched
            .iter()
            .filter(|p| (p.input, p.context) == (frame.input, frame.context) && p.subject.is_none())
            .count()
            != 1
        {
            return Err(ModelError::Invalid(
                "Model requires exact independently captured Enriched frame".into(),
            ));
        }
        let mut parents = Rows::new(budget);
        for p in data
            .enriched
            .iter()
            .filter(|p| (p.input, p.context) == (frame.input, frame.context))
        {
            parents.insert(owner::InvocationSource::EnrichedExecution { invocation: p.id() })?;
        }
        for p in data
            .source_calls
            .iter()
            .filter(|p| (p.input, p.context) == (frame.input, frame.context))
        {
            parents.insert(owner::InvocationSource::SourceCallAnalysis { invocation: p.id() })?;
        }
        for p in data
            .local
            .iter()
            .filter(|p| (p.input, p.context) == (frame.input, frame.context))
        {
            parents.insert(owner::InvocationSource::Local { invocation: p.id() })?;
        }
        let (invocation, inputs, receipts, projections) = owner::AnalysisInvocation::admitted(
            frame.input,
            frame.context,
            definition.id(),
            None,
            parents.iter().map(Record::id),
            &sources,
            [],
            budget,
        )?;
        let bytes = receipts
            .iter()
            .try_fold(0usize, |n, r| {
                n.checked_add(size_of::<owner::SourceReceipt>())?
                    .checked_add(r.heap_bytes())
            })
            .and_then(|n| {
                n.checked_add(
                    inputs
                        .len()
                        .checked_mul(size_of::<owner::AnalysisInput>())?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    projections
                        .len()
                        .checked_mul(size_of::<owner::ProjectionInput>())?,
                )
            })
            .ok_or_else(|| ModelError::Invalid("Model invocation lowering overflow".into()))?;
        let _buffers = budget.reserve("model_invocation_lowering", bytes)?;
        let coverage = owner::coverage::admit(
            &invocation,
            definition,
            analysis::AnalysisCapability::Models,
            &admission,
            budget,
        )?;
        let records = apply_all(&data, &invocation, definition, profile, budget)?;
        macro_rules! write{($($field:ident,)*)=>{$(for row in records.$field.iter(){output.push(row.clone()).await?;})*};}
        write!(
            assumption_sets,
            assumption_members,
            assumptions,
            assumption_universes,
            universe_supports,
            applications,
            application_premises,
            boundaries,
            targets,
            rules,
            channels,
            operations,
            resources,
            paths,
            action_assessments,
            action_sources,
            postconditions,
            context_transfers,
            context_resources,
            context_values,
            context_postconditions,
            transfer_witnesses,
            transfer_keys,
            transfer_alternatives,
            transfer_supports,
            transfer_roots,
            transfer_places,
            qualifications,
            conditions,
            condition_nodes,
            subjects,
            support_sources,
            derivations,
            propositions,
            derivation_premises,
        );
        for scope in coverage.scopes() {
            let (requirement, required) = scope.expectation().records()?;
            output.push(requirement).await?;
            for row in required {
                output.push(row).await?;
            }
            for row in scope.observations() {
                output.push(row.source().clone()).await?;
            }
            let (row, premises) = owner::coverage::assess(
                scope.expectation(),
                scope.observations(),
                records.outcome.status,
                records.outcome.reason,
                budget,
            )?;
            output.push(row).await?;
            for row in premises {
                output.push(row).await?;
            }
        }
        output.push(records.run).await?;
        output.push(records.outcome).await?;
        output.push(invocation).await?;
        for row in parents.iter() {
            output.push(row.clone()).await?;
        }
        for row in inputs {
            output.push(row).await?;
        }
        for row in receipts {
            output.push(row).await?;
        }
        for row in projections {
            output.push(row).await?;
        }
    }
    drop(data);
    output.finish(ProviderOutcome::Complete).await
}
