//! Qualified base execution consumes actual completed Local/native/normalized inputs.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use lctx_model::domain::{
    analysis::{
        self, base_evaluation as publication, expected::CoverageAdmission, sources::CapturedSources,
    },
    conditions::entry::{EntryAccessSource, EntryData, EntryValueWitness},
    execution::{
        evaluation::EvaluationData,
        production::{self, *},
        records::*,
    },
    normalized::Rows,
    stages::*,
    *,
};
use std::sync::Arc;
async fn load<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    consumed: &mut crate::consumed_rows::ConsumedInputs,
    admission: &mut CoverageAdmission<'_>,
    mut visit: impl FnMut(
        &ValidationInput,
        &analysis::sources::CompletedInput<R>,
        &arrow_array::RecordBatch,
    ) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    while let Some((input, permit)) = consumed.next::<R>(access)? {
        crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
            admission.visit_if_expected(permit, batch)?;
            visit(&input, permit, batch)
        })
        .await?;
    }
    Ok(())
}
/// Compiler lifetime bound around an opaque value minted by the actual model producer.
pub struct Produced<T> {
    premises: CompletedInputs,
    outputs: CompletedInputs,
    value: T,
}
impl<T> Produced<T> {
    fn borrow(&self, access: &CompletedInputs, runtime: &Workspace) -> Result<&T, ModelError> {
        self.premises.require_subset(runtime, access)?;
        self.outputs.require_subset(runtime, access)?;
        Ok(&self.value)
    }
}
fn produced_premises(access: &CompletedInputs, inputs: Vec<ValidationInput>) -> Result<CompletedInputs, ModelError> {
    access.select(&inputs.into_iter().filter(|input| access.table_for(input).is_ok()).collect::<Vec<_>>())
}
pub async fn evaluate_base(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<Option<Produced<production::ProducedEvaluations>>, ModelError> {
    if *definition != execution::configuration::base_evaluation().1 {
        return Err(ModelError::Invalid(
            "base execution definition is not bound".into(),
        ));
    }
    let mut produced: Option<production::ProducedEvaluations> = None;
    let premises = produced_premises(&access, execution::records::base_invariants().remove(0).inputs)?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut data = EvaluationData::new(budget);
    let mut entry = EntryData::new(budget);
    let mut declarations = Vec::new();
    if profile == Profile::Behavioral {
        declarations.extend(EvaluationData::consumed_inputs(profile));
        declarations.extend(EntryData::facts_inputs());
        declarations.extend([
            ValidationInput::of::<EntryValueWitness>(&["id"]),
            ValidationInput::of::<EntryAccessSource>(&["id"]),
        ]);
    }
    declarations.extend([
        ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
    ]);
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
    macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$({load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|{data.visit_input(input, batch)?;entry.visit_input(input, batch)?;Ok(())}).await?;})*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
    }
    let mut entries = Rows::<EntryValueWitness>::new(budget);
    let mut entry_sources = Rows::<EntryAccessSource>::new(budget);
    let mut local = Rows::<analysis::local::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|_,_,batch|$field.decode(batch)).await?;)*};}
    if profile == Profile::Behavioral {
        read! {entries:EntryValueWitness,entry_sources:EntryAccessSource,}
    }
    read! {local:analysis::local::AnalysisInvocation,definitions:analysis::AnalysisDefinition,}
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "base execution definition absent from confirmed configuration".into(),
        ));
    }
    macro_rules! expected {($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|_,_,_|Ok(())).await?;)*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    consumed.finish(access.name())?;
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<publication::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(
        EvaluationRun,
        EvaluationBoundary,
        ExpressionEvaluation,
        EvaluationSource,
        EvaluationMember,
        EvaluationOperand,
        execution::read_channels::ReadObservation,
        execution::read_channels::ReadDependency,
        execution::read_channels::AttributeRead,
        execution::read_channels::FormalReadAssessment,
        execution::read_dynamic::DynamicAccessObservation,
        execution::read_dynamic::DynamicAccessPremise,
        execution::read_fields::FieldLocationObservation,
        execution::read_fields::FieldReadAssessment,
        execution::read_fields::GlobalClassInspection,
        execution::read_fields::GlobalFieldReadAssessment
    );
    let mut frames = charged::ChargedSet::default();
    let mut frame_charge = charged::StateCharge::new(budget, "base_execution_frames");
    for frame in local.iter() {
        if !frames.insert(&mut frame_charge, (frame.input, frame.context))? {
            continue;
        }
        let mut parents = Rows::<publication::InvocationSource>::new(budget);
        for row in local
            .iter()
            .filter(|row| (row.input, row.context) == (frame.input, frame.context))
        {
            parents.insert(publication::InvocationSource::Local {
                invocation: row.id(),
            })?;
        }
        let (invocation, inputs, receipts, projections) =
            publication::AnalysisInvocation::admitted(
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
            .try_fold(0usize, |n, row| {
                n.checked_add(size_of::<publication::SourceReceipt>())?
                    .checked_add(row.heap_bytes())
            })
            .and_then(|n| {
                n.checked_add(
                    inputs
                        .len()
                        .checked_mul(size_of::<publication::AnalysisInput>())?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    projections
                        .len()
                        .checked_mul(size_of::<publication::ProjectionInput>())?,
                )
            })
            .ok_or_else(|| {
                ModelError::Invalid("base invocation lowering allowance overflow".into())
            })?;
        let _buffers = budget.reserve("base_invocation_lowering", bytes)?;
        let coverage = publication::coverage::admit(
            &invocation,
            definition,
            analysis::AnalysisCapability::Execution,
            &admission,
            budget,
        )?;
        let (records, owner) = production::evaluate_all_produced(
            &data,
            &entry,
            &entries,
            &entry_sources,
            &invocation,
            definition,
            profile,
            budget,
        )?;
        match produced.as_mut() { Some(value) => value.append(owner)?, None => produced = Some(owner) };
        macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(for row in records.$field.iter(){output.push(row.clone()).await?;})*};}
        write!(evaluations:ExpressionEvaluation,sources:EvaluationSource,members:EvaluationMember,operands:EvaluationOperand,boundaries:EvaluationBoundary,);
        for row in records.reads.fields.locations.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.fields.assessments.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.fields.globals.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.fields.global_assessments.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.dynamic.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.dynamic_premises.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.reads.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.dependencies.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.attributes.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.reads.formals.iter() {
            output.push(row.clone()).await?;
        }
        for scope in coverage.scopes() {
            let (requirement, required) = scope.expectation().records()?;
            output.push(requirement).await?;
            for row in required {
                output.push(row).await?;
            }
            for row in scope.observations() {
                output.push(row.source().clone()).await?;
            }
            let (row, premises) = publication::coverage::assess(
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
    drop(entry);
    drop(entries);
    drop(entry_sources);
    drop(local);
    output.finish(ProviderOutcome::Complete).await?;
    if profile != Profile::Behavioral { return Ok(None); }
    let outputs = runtime.inputs("actual-produced-values", profile, [publication::AnalysisInvocation::NAME, ExpressionEvaluation::NAME, EvaluationSource::NAME, EvaluationMember::NAME, EvaluationOperand::NAME])?;
    Ok(produced.map(|value| Produced { premises, outputs, value }))
}

use lctx_model::domain::{
    analysis::base_completion as completion_publication,
    execution::{
        body_records::*,
        completion_production::{CompletionBoundary, CompletionRun},
        completion_records::*,
    },
};
pub async fn complete_base(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    evaluations: Option<&Produced<production::ProducedEvaluations>>,
) -> Result<Option<Produced<execution::completion_production::ProducedBodies>>, ModelError> {
    if *definition != execution::configuration::base_completion().1 {
        return Err(ModelError::Invalid(
            "base completion definition is not bound".into(),
        ));
    }
    let evaluations = evaluations.map(|owner| owner.borrow(&access, runtime)).transpose()?;
    let mut produced: Option<execution::completion_production::ProducedBodies> = None;
    let premises = produced_premises(&access, execution::records::base_invariants().remove(0).inputs)?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut data = execution::completion_production::CompletedEvaluations::new(budget);
    let mut declarations =
        execution::completion_production::CompletedEvaluations::consumed_inputs(profile);
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
    macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$({load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|{data.visit_input(input, batch)}).await?;})*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
    }
    let mut base = Rows::<analysis::base_evaluation::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    macro_rules! read {($($ty:ty),*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|data.visit_input(input, batch)).await?;)*};}
    if profile == Profile::Behavioral {
        read!(
            EntryValueWitness,
            EntryAccessSource,
            ExpressionEvaluation,
            EvaluationSource,
            EvaluationMember,
            EvaluationOperand
        );
    }
    load::<analysis::base_evaluation::AnalysisInvocation>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| {
            base.decode(batch)?;
            data.visit_input(input, batch)
        },
    )
    .await?;
    load::<analysis::AnalysisDefinition>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| {
            definitions.decode(batch)?;
            data.visit_input(input, batch)
        },
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "base completion definition absent from confirmed configuration".into(),
        ));
    }
    macro_rules! expected {($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|_,_,_|Ok(())).await?;)*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    consumed.finish(access.name())?;
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<completion_publication::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(
        CompletionRun,
        CompletionBoundary,
        StatementCompletion,
        CompletionOutcome,
        CompletionSource,
        CompletionMember,
        EnteredStatement,
        SourceBodyCompletion,
        BodySource,
        BodyMember,
        BodyReleaseInput,
        BodyBoundary
    );
    let mut frames = charged::ChargedSet::default();
    let mut frame_charge = charged::StateCharge::new(budget, "base_completion_frames");
    for frame in base.iter() {
        if !frames.insert(&mut frame_charge, (frame.input, frame.context))? {
            continue;
        }
        let mut parents = Rows::<completion_publication::InvocationSource>::new(budget);
        for row in base
            .iter()
            .filter(|row| (row.input, row.context) == (frame.input, frame.context))
        {
            parents.insert(completion_publication::InvocationSource::BaseEvaluation {
                invocation: row.id(),
            })?;
        }
        let (invocation, inputs, receipts, projections) =
            completion_publication::AnalysisInvocation::admitted(
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
            .try_fold(0usize, |n, row| {
                n.checked_add(size_of::<completion_publication::SourceReceipt>())?
                    .checked_add(row.heap_bytes())
            })
            .and_then(|n| {
                n.checked_add(
                    inputs
                        .len()
                        .checked_mul(size_of::<completion_publication::AnalysisInput>())?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    projections
                        .len()
                        .checked_mul(size_of::<completion_publication::ProjectionInput>())?,
                )
            })
            .ok_or_else(|| {
                ModelError::Invalid("base invocation lowering allowance overflow".into())
            })?;
        let _buffers = budget.reserve("completion_invocation_lowering", bytes)?;
        let coverage = completion_publication::coverage::admit(
            &invocation,
            definition,
            analysis::AnalysisCapability::Completion,
            &admission,
            budget,
        )?;
        let (records, owner) = execution::completion_production::complete_all_produced(
            &data,
            &invocation,
            definition,
            profile,
            budget,
            evaluations,
        )?;
        match produced.as_mut() { Some(value) => value.append(owner)?, None => produced = Some(owner) };
        macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(for row in records.$field.iter(){output.push(row.clone()).await?;})*};}
        write!(completions:StatementCompletion,outcomes:CompletionOutcome,sources:CompletionSource,members:CompletionMember,entered:EnteredStatement,boundaries:CompletionBoundary,bodies:SourceBodyCompletion,body_sources:BodySource,body_members:BodyMember,body_releases:BodyReleaseInput,body_boundaries:BodyBoundary,);
        for scope in coverage.scopes() {
            let (requirement, required) = scope.expectation().records()?;
            output.push(requirement).await?;
            for row in required {
                output.push(row).await?;
            }
            for row in scope.observations() {
                output.push(row.source().clone()).await?;
            }
            let (row, premises) = completion_publication::coverage::assess(
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
    drop(base);
    output.finish(ProviderOutcome::Complete).await?;
    if profile != Profile::Behavioral { return Ok(None); }
    let outputs = runtime.inputs("actual-produced-values", profile, [completion_publication::AnalysisInvocation::NAME, SourceBodyCompletion::NAME])?;
    Ok(produced.map(|value| Produced { premises, outputs, value }))
}

/// Fresh source binding consumes acknowledged normalized shapes and earlier completion frames.
pub async fn prepare_source_calls(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    bindings: Option<&crate::analysis_bindings::PreparedBindings>,
    evaluations: Option<&Produced<production::ProducedEvaluations>>,
    bodies: Option<&Produced<execution::completion_production::ProducedBodies>>,
) -> Result<Option<Produced<execution::source_call_records::ProducedSourceCalls>>, ModelError> {
    use analysis::source_call as owner;
    use execution::source_call_records::*;
    if *definition != execution::configuration::source_calls().1 {
        return Err(ModelError::Invalid(
            "source call definition is unbound".into(),
        ));
    }
    let evaluations = evaluations.map(|owner| owner.borrow(&access, runtime)).transpose()?;
    let bodies = bodies.map(|owner| owner.borrow(&access, runtime)).transpose()?;
    let application = if access.profile() == Profile::Behavioral {
        Some(bindings.ok_or_else(|| ModelError::Invalid("normalized application authority absent".into()))?.application(&access, runtime)?)
    } else { None };
    let mut produced: Option<execution::source_call_records::ProducedSourceCalls> = None;
    let premises = produced_premises(&access, execution::source_call_records::SourceCallData::inputs())?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut data = SourceCallData::new(budget);
    let mut declarations = SourceCallData::consumed_inputs(profile);
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
    macro_rules! inputs{($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|data.visit_input(input, batch)).await?;)*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
        lctx_model::normalized_binding_inputs!(inputs);
        lctx_model::normalized_binding_outputs!(inputs);
    }
    if profile == Profile::Behavioral {
        macro_rules! earlier{($($ty:ty),*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|data.visit_input(input, batch)).await?;)*};}
        earlier!(
            syntax::ParameterSyntaxObservation,
            EntryValueWitness,
            EntryAccessSource,
            ExpressionEvaluation,
            EvaluationSource,
            EvaluationMember,
            EvaluationOperand,
            analysis::base_evaluation::AnalysisInvocation,
            SourceBodyCompletion
        );
    }
    let mut base = Rows::<analysis::base_completion::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    load::<analysis::base_completion::AnalysisInvocation>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| {
            base.decode(batch)?;
            data.visit_input(input, batch)
        },
    )
    .await?;
    load::<analysis::AnalysisDefinition>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| {
            definitions.decode(batch)?;
            data.visit_input(input, batch)
        },
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "source call definition absent from captured configuration".into(),
        ));
    }
    macro_rules! expected{($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|_,_,_|Ok(())).await?;)*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    consumed.finish(access.name())?;
    macro_rules! declare{($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<owner::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(
        SourceCallRun,
        SourceCallHeader,
        HeaderMember,
        SourceCallBoundary,
        SourceInvocation,
        SourceFrameRelease,
        SourceFrameArgument,
        SourceCallOutcome,
        InvocationBoundary
    );
    let mut frames = charged::ChargedSet::default();
    let mut frame_charge = charged::StateCharge::new(budget, "source_call_frames");
    for frame in base.iter() {
        if !frames.insert(&mut frame_charge, (frame.input, frame.context))? {
            continue;
        }
        let mut parents = Rows::<owner::InvocationSource>::new(budget);
        for row in base
            .iter()
            .filter(|b| (b.input, b.context) == (frame.input, frame.context))
        {
            parents.insert(owner::InvocationSource::BaseCompletion {
                invocation: row.id(),
            })?;
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
            .ok_or_else(|| {
                ModelError::Invalid("source invocation lowering allowance overflow".into())
            })?;
        let _buffers = budget.reserve("source_call_invocation_lowering", bytes)?;
        let coverage = owner::coverage::admit(
            &invocation,
            definition,
            analysis::AnalysisCapability::Execution,
            &admission,
            budget,
        )?;
        let (records, owner) = prepare_all_produced(&data, &invocation, definition, profile, budget, application, evaluations, bodies)?;
        match produced.as_mut() { Some(value) => value.append(owner)?, None => produced = Some(owner) };
        for row in records.headers.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.members.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.boundaries.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.invocations.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.releases.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.arguments.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.call_outcomes.iter() {
            output.push(row.clone()).await?;
        }
        for row in records.invocation_boundaries.iter() {
            output.push(row.clone()).await?;
        }
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
    drop(base);
    drop(definitions);
    output.finish(ProviderOutcome::Complete).await?;
    if profile != Profile::Behavioral { return Ok(None); }
    let outputs = runtime.inputs("actual-produced-values", profile, [analysis::source_call::AnalysisInvocation::NAME, analysis::source_call::AnalysisOutcome::NAME, execution::source_call_records::SourceCallRun::NAME, execution::source_call_records::SourceCallHeader::NAME, execution::source_call_records::HeaderMember::NAME, execution::source_call_records::SourceCallBoundary::NAME, execution::source_call_records::SourceInvocation::NAME, execution::source_call_records::SourceFrameRelease::NAME, execution::source_call_records::SourceFrameArgument::NAME, execution::source_call_records::SourceCallOutcome::NAME, execution::source_call_records::InvocationBoundary::NAME])?;
    Ok(produced.map(|value| Produced { premises, outputs, value }))
}

/// Fresh source binding consumes acknowledged normalized shapes and earlier completion frames.
pub async fn enrich(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    bindings: Option<&crate::analysis_bindings::PreparedBindings>,
    evaluations: Option<&Produced<production::ProducedEvaluations>>,
    source_calls: Option<&Produced<execution::source_call_records::ProducedSourceCalls>>,
) -> Result<(), ModelError> {
    use analysis::enriched_execution as owner;
    use execution::{
        context_binding::{
            BindingMember as ContextBindingMember, BindingSource as ContextBindingSource,
            ContextEntryBinding,
        },
        context_execution::*,
        definition::*,
        enriched_production::*,
        enriched_records::*,
        modeled_call::*,
        source_call_records::{
            HeaderMember, InvocationBoundary, SourceCallBoundary, SourceCallHeader,
            SourceCallOutcome, SourceCallRun, SourceFrameArgument, SourceFrameRelease,
            SourceInvocation,
        },
    };
    if !execution::configuration::enriched_kernel(definition) {
        return Err(ModelError::Invalid(
            "enriched execution definition is unbound".into(),
        ));
    }
    let evaluations = evaluations.map(|owner| owner.borrow(&access, runtime)).transpose()?;
    let source_calls = source_calls.map(|owner| owner.borrow(&access, runtime)).transpose()?;
    let application = if access.profile() == Profile::Behavioral {
        Some(bindings.ok_or_else(|| ModelError::Invalid("normalized application authority absent".into()))?.application(&access, runtime)?)
    } else { None };
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut data = EnrichedData::new(budget);
    let mut declarations = EnrichedData::consumed_inputs(profile);
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
    macro_rules! inputs{($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|data.visit_input(input, batch)).await?;)*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
        lctx_model::normalized_binding_inputs!(inputs);
        lctx_model::normalized_binding_outputs!(inputs);
        lctx_model::model_pin_inputs!(inputs);
    }
    if profile == Profile::Behavioral {
        macro_rules! earlier{($($ty:ty),*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|data.visit_input(input, batch)).await?;)*};}
        earlier!(
            syntax::ParameterSyntaxObservation,
            EntryValueWitness,
            EntryAccessSource,
            ExpressionEvaluation,
            EvaluationSource,
            EvaluationMember,
            EvaluationOperand,
            analysis::base_evaluation::AnalysisInvocation,
            SourceBodyCompletion,
            analysis::base_completion::AnalysisInvocation,
            SourceCallHeader,
            HeaderMember,
            SourceCallBoundary,
            SourceCallRun,
            SourceInvocation,
            SourceFrameRelease,
            SourceFrameArgument,
            SourceCallOutcome,
            InvocationBoundary,
            analysis::source_call::AnalysisOutcome
        );
    }
    load::<analysis::MethodParameters>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| data.visit_input(input, batch),
    )
    .await?;
    load::<models::ModelCatalog>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| data.visit_input(input, batch),
    )
    .await?;
    let mut base = Rows::<analysis::source_call::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    load::<analysis::source_call::AnalysisInvocation>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| {
            base.decode(batch)?;
            data.visit_input(input, batch)
        },
    )
    .await?;
    load::<analysis::AnalysisDefinition>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| {
            definitions.decode(batch)?;
            data.visit_input(input, batch)
        },
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "enriched execution definition absent from captured configuration".into(),
        ));
    }
    macro_rules! expected{($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|_,_,_|Ok(())).await?;)*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    consumed.finish(access.name())?;
    macro_rules! declare{($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<owner::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(
        ExecutionRun,
        ExecutionBoundary,
        BodyBoundary,
        StatementExecution,
        ExecutionOutcome,
        ExecutionSource,
        ExecutionMember,
        EnteredStatement,
        BodyExecution,
        BodySource,
        BodyMember,
        BodyReleaseInput,
        ModeledCallEvaluation,
        ModeledCallArgument,
        ModeledCallNative,
        SourceExecutionInvocation,
        SourceExecutionArgument,
        lctx_model::domain::execution::capture_bridge::CapturedEntryBinding,
        lctx_model::domain::execution::capture_bridge::CapturedValueSource,
        DefinitionEvaluation,
        DefinitionSource,
        DefinitionMember,
        ContextExecution,
        ContextItem,
        ContextSource,
        ContextMember,
        ContextEntryBinding,
        ContextBindingSource,
        ContextBindingMember
    );
    let mut frames = charged::ChargedSet::default();
    let mut frame_charge = charged::StateCharge::new(budget, "enriched_execution_frames");
    for frame in base.iter() {
        if !frames.insert(&mut frame_charge, (frame.input, frame.context))? {
            continue;
        }
        let mut parents = Rows::<owner::InvocationSource>::new(budget);
        for row in base
            .iter()
            .filter(|b| (b.input, b.context) == (frame.input, frame.context))
        {
            parents.insert(owner::InvocationSource::SourceCallAnalysis {
                invocation: row.id(),
            })?;
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
            .ok_or_else(|| {
                ModelError::Invalid("source invocation lowering allowance overflow".into())
            })?;
        let _buffers = budget.reserve("enriched_execution_invocation_lowering", bytes)?;
        let coverage = owner::coverage::admit(
            &invocation,
            definition,
            analysis::AnalysisCapability::Execution,
            &admission,
            budget,
        )?;
        let records = enrich_all_produced(&data, &invocation, definition, profile, budget, application, evaluations, source_calls)?;
        macro_rules! write{($($field:ident:$ty:ty,)*)=>{$(for row in records.$field.iter(){output.push(row.clone()).await?;})*};}
        write!(modeled_calls:ModeledCallEvaluation,modeled_arguments:ModeledCallArgument,modeled_native:ModeledCallNative,fresh_calls:SourceExecutionInvocation,fresh_arguments:SourceExecutionArgument,captured_entries:lctx_model::domain::execution::capture_bridge::CapturedEntryBinding,captured_values:lctx_model::domain::execution::capture_bridge::CapturedValueSource,definition_evaluations:DefinitionEvaluation,definition_sources:DefinitionSource,definition_members:DefinitionMember,contexts:ContextExecution,context_items:ContextItem,context_sources:ContextSource,context_members:ContextMember,context_bindings:ContextEntryBinding,context_binding_sources:ContextBindingSource,context_binding_members:ContextBindingMember,executions:StatementExecution,outcomes:ExecutionOutcome,sources:ExecutionSource,members:ExecutionMember,entered:EnteredStatement,boundaries:ExecutionBoundary,bodies:BodyExecution,body_sources:BodySource,body_members:BodyMember,releases:BodyReleaseInput,body_boundaries:BodyBoundary,);
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
    drop(base);
    drop(definitions);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod produced_authority_controls {
    use super::*;
    use crate::workspace::WorkspaceOptions;
    use lctx_model::domain::source::SourceArtifact;
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new([value;16].into_iter())).unwrap()
    }
    async fn publish(runtime: &Arc<Workspace>, name: &'static str, artifact: SourceArtifact) {
        let output = runtime.output(name, Profile::Catalog, ContentHash::of(name.as_bytes()),
            runtime.inputs(name, Profile::Catalog, []).unwrap());
        output.declare::<SourceArtifact>().unwrap();
        output.push(artifact).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
    }
    #[tokio::test]
    async fn producer_borrow_refuses_foreign_attempt_profile_and_changed_descriptor() {
        let model = Arc::new(model().unwrap());
        let runtime = Workspace::new(model.clone(), WorkspaceOptions::default()).unwrap();
        let artifact = SourceArtifact::from_bytes(nominal(1), "source.py".into(), b"x").unwrap();
        publish(&runtime, "first", artifact.clone()).await;
        let selected = runtime.inputs("consumer", Profile::Catalog, [SourceArtifact::NAME]).unwrap();
        let produced = Produced {premises:selected.clone(), outputs:selected.clone(), value:7u8};
        assert_eq!(*produced.borrow(&selected, &runtime).unwrap(), 7);
        let other = Workspace::new(model, WorkspaceOptions::default()).unwrap();
        publish(&other, "same-content", artifact).await;
        let foreign = other.inputs("consumer", Profile::Catalog, [SourceArtifact::NAME]).unwrap();
        assert!(matches!(produced.borrow(&foreign, &other), Err(ModelError::Conflict(_))));
        let changed_profile = runtime.inputs("consumer", Profile::Behavioral, [SourceArtifact::NAME]).unwrap();
        assert!(matches!(produced.borrow(&changed_profile, &runtime), Err(ModelError::Conflict(_))));
        publish(&runtime, "second", SourceArtifact::from_bytes(nominal(1), "second.py".into(), b"y").unwrap()).await;
        let changed = runtime.inputs("consumer", Profile::Catalog, [SourceArtifact::NAME]).unwrap();
        assert!(matches!(produced.borrow(&changed, &runtime), Err(ModelError::Conflict(_))));
        assert_eq!(*produced.borrow(&selected, &runtime).unwrap(), 7);
    }
    #[test]
    fn requested_production_refuses_absent_predecessor_owners() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let invocation = analysis::base_completion::AnalysisInvocation::new(nominal(1), nominal(2),
            execution::configuration::base_completion().1.id(), None, []).0;
        assert!(matches!(execution::completion_production::complete_all_produced(
            &execution::completion_production::CompletedEvaluations::new(&budget), &invocation,
            &execution::configuration::base_completion().1, Profile::Behavioral, &budget, None,
        ), Err(ModelError::Conflict(_))));
        let (records, _) = execution::completion_production::complete_all_produced(
            &execution::completion_production::CompletedEvaluations::new(&budget), &invocation,
            &execution::configuration::base_completion().1, Profile::Catalog, &budget, None,
        ).unwrap();
        assert_eq!(records.outcome.status, analysis::AnalysisStatus::NotRequested);
        let invocation = analysis::source_call::AnalysisInvocation::new(nominal(1), nominal(2),
            execution::configuration::source_calls().1.id(), None, []).0;
        assert!(matches!(execution::source_call_records::prepare_all_produced(
            &execution::source_call_records::SourceCallData::new(&budget), &invocation,
            &execution::configuration::source_calls().1, Profile::Behavioral, &budget, None, None, None,
        ), Err(ModelError::Conflict(_))));
        let (records, _) = execution::source_call_records::prepare_all_produced(
            &execution::source_call_records::SourceCallData::new(&budget), &invocation,
            &execution::configuration::source_calls().1, Profile::Catalog, &budget, None, None, None,
        ).unwrap();
        assert_eq!(records.outcome.status, analysis::AnalysisStatus::NotRequested);

    }
}
