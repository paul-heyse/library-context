//! Qualified base execution consumes actual completed Local/native/normalized inputs.
use crate::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, StageSession},
};
use futures::TryStreamExt;
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
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;
async fn load<R: Record>(
    access: &StageAccess<'_, '_>,
    reader: &AttemptSession,
    session: &StageSession,
    registered: &mut charged::ChargedSet<&'static str>,
    charge: &mut charged::StateCharge,
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
        visit(&permit, &batch)?;
    }
    Ok(())
}
pub async fn evaluate_base(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<(), ModelError> {
    if *definition != execution::configuration::base_evaluation().1 {
        return Err(ModelError::Invalid(
            "base execution definition is not bound".into(),
        ));
    }
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(&access, budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut registered = charged::ChargedSet::default();
    let mut registration = charged::StateCharge::new(budget, "base_execution_registration");
    let mut data = EvaluationData::new(budget);
    let mut entry = EntryData::new(budget);
    macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$({if !registered.contains(<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|{data.visit(<$ty>::NAME,batch)?;entry.visit(<$ty>::NAME,batch)?;Ok(())}).await?;}})*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
    }
    let mut entries = Rows::<EntryValueWitness>::new(budget);
    let mut entry_sources = Rows::<EntryAccessSource>::new(budget);
    let mut local = Rows::<analysis::local::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|$field.decode(batch)).await?;)*};}
    if profile == Profile::Behavioral {
        read! {entries:EntryValueWitness,entry_sources:EntryAccessSource,}
    }
    read! {local:analysis::local::AnalysisInvocation,definitions:analysis::AnalysisDefinition,}
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "base execution definition absent from confirmed configuration".into(),
        ));
    }
    macro_rules! expected {($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|permit,batch|admission.visit(permit,batch)).await?;)*};}
    expected!(
        input::InputRevision,
        source::SourceArtifact,
        input::ArtifactUse,
        source::CoverageScope,
        normalized::coverage::NormalizationComputation,
        normalized::coverage::NormalizationCoverage,
        attribution::ProviderCoverage
    );
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let mut output = StageOutput::new(access, attempt, model, budget.clone(), Default::default())?;
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    declare!(
        publication::AnalysisInvocation,
        publication::AnalysisInput,
        publication::SourceReceipt,
        publication::ProjectionInput,
        publication::AnalysisOutcome,
        publication::InvocationSource,
        publication::AnalysisCoverage,
        publication::CoverageRequirement,
        publication::CoverageRequiredSource,
        publication::AnalysisCoveragePremise,
        publication::CoverageSource,
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
        let records = production::evaluate_all(
            &data,
            &entry,
            &entries,
            &entry_sources,
            &invocation,
            definition,
            profile,
            budget,
        )?;
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
    output.finish(ProviderOutcome::Complete).await
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
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<(), ModelError> {
    if *definition != execution::configuration::base_completion().1 {
        return Err(ModelError::Invalid(
            "base completion definition is not bound".into(),
        ));
    }
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(&access, budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut registered = charged::ChargedSet::default();
    let mut registration = charged::StateCharge::new(budget, "base_completion_registration");
    let mut data = execution::completion_production::CompletedEvaluations::new(budget);
    macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$({if !registered.contains(<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|{data.visit(<$ty>::NAME,batch)}).await?;}})*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
    }
    let mut base = Rows::<analysis::base_evaluation::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    macro_rules! read {($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|data.visit(<$ty>::NAME,batch)).await?;)*};}
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
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| {
            base.decode(batch)?;
            data.visit(analysis::base_evaluation::AnalysisInvocation::NAME, batch)
        },
    )
    .await?;
    load::<analysis::AnalysisDefinition>(
        &access,
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| {
            definitions.decode(batch)?;
            data.visit(analysis::AnalysisDefinition::NAME, batch)
        },
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "base completion definition absent from confirmed configuration".into(),
        ));
    }
    macro_rules! expected {($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|permit,batch|admission.visit(permit,batch)).await?;)*};}
    expected!(
        input::InputRevision,
        source::SourceArtifact,
        input::ArtifactUse,
        source::CoverageScope,
        normalized::coverage::NormalizationComputation,
        normalized::coverage::NormalizationCoverage,
        attribution::ProviderCoverage
    );
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let mut output = StageOutput::new(access, attempt, model, budget.clone(), Default::default())?;
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    declare!(
        completion_publication::AnalysisInvocation,
        completion_publication::AnalysisInput,
        completion_publication::SourceReceipt,
        completion_publication::ProjectionInput,
        completion_publication::AnalysisOutcome,
        completion_publication::InvocationSource,
        completion_publication::AnalysisCoverage,
        completion_publication::CoverageRequirement,
        completion_publication::CoverageRequiredSource,
        completion_publication::AnalysisCoveragePremise,
        completion_publication::CoverageSource,
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
        let records = execution::completion_production::complete_all(
            &data,
            &invocation,
            definition,
            profile,
            budget,
        )?;
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
    output.finish(ProviderOutcome::Complete).await
}

/// Fresh source binding consumes acknowledged normalized shapes and earlier completion frames.
pub async fn prepare_source_calls(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<(), ModelError> {
    use analysis::source_call as owner;
    use execution::source_call_records::*;
    if *definition != execution::configuration::source_calls().1 {
        return Err(ModelError::Invalid(
            "source call definition is unbound".into(),
        ));
    }
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(&access, budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut registered = charged::ChargedSet::default();
    let mut registration = charged::StateCharge::new(budget, "source_call_registration");
    let mut data = SourceCallData::new(budget);
    macro_rules! inputs{($($field:ident:$ty:ty,)*)=>{$(if !registered.contains(<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|data.visit(<$ty>::NAME,batch)).await?;})*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
        lctx_model::normalized_binding_inputs!(inputs);
        lctx_model::normalized_binding_outputs!(inputs);
    }
    if profile == Profile::Behavioral {
        macro_rules! earlier{($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|data.visit(<$ty>::NAME,batch)).await?;)*};}
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
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| {
            base.decode(batch)?;
            data.visit(analysis::base_completion::AnalysisInvocation::NAME, batch)
        },
    )
    .await?;
    load::<analysis::AnalysisDefinition>(
        &access,
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| {
            definitions.decode(batch)?;
            data.visit(analysis::AnalysisDefinition::NAME, batch)
        },
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "source call definition absent from captured configuration".into(),
        ));
    }
    macro_rules! expected{($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|permit,batch|admission.visit(permit,batch)).await?;)*};}
    expected!(
        input::InputRevision,
        source::SourceArtifact,
        input::ArtifactUse,
        source::CoverageScope,
        normalized::coverage::NormalizationComputation,
        normalized::coverage::NormalizationCoverage,
        attribution::ProviderCoverage
    );
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
        let records = prepare_all(&data, &invocation, definition, profile, budget)?;
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
    output.finish(ProviderOutcome::Complete).await
}

/// Fresh source binding consumes acknowledged normalized shapes and earlier completion frames.
pub async fn enrich(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
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
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(&access, budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut registered = charged::ChargedSet::default();
    let mut registration = charged::StateCharge::new(budget, "enriched_execution_registration");
    let mut data = EnrichedData::new(budget);
    macro_rules! inputs{($($field:ident:$ty:ty,)*)=>{$(if !registered.contains(<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|data.visit(<$ty>::NAME,batch)).await?;})*};}
    if profile == Profile::Behavioral {
        lctx_model::execution_evaluation_inputs!(inputs);
        lctx_model::entry_value_inputs!(inputs);
        lctx_model::normalized_binding_inputs!(inputs);
        lctx_model::normalized_binding_outputs!(inputs);
        lctx_model::model_pin_inputs!(inputs);
    }
    if profile == Profile::Behavioral {
        macro_rules! earlier{($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|_,batch|data.visit(<$ty>::NAME,batch)).await?;)*};}
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
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| data.visit(analysis::MethodParameters::NAME, batch),
    )
    .await?;
    load::<models::ModelCatalog>(
        &access,
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| data.visit(models::ModelCatalog::NAME, batch),
    )
    .await?;
    let mut base = Rows::<analysis::source_call::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    load::<analysis::source_call::AnalysisInvocation>(
        &access,
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| {
            base.decode(batch)?;
            data.visit(analysis::source_call::AnalysisInvocation::NAME, batch)
        },
    )
    .await?;
    load::<analysis::AnalysisDefinition>(
        &access,
        &reader,
        &session,
        &mut registered,
        &mut registration,
        |_, batch| {
            definitions.decode(batch)?;
            data.visit(analysis::AnalysisDefinition::NAME, batch)
        },
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "enriched execution definition absent from captured configuration".into(),
        ));
    }
    macro_rules! expected{($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut registered,&mut registration,|permit,batch|admission.visit(permit,batch)).await?;)*};}
    expected!(
        input::InputRevision,
        source::SourceArtifact,
        input::ArtifactUse,
        source::CoverageScope,
        normalized::coverage::NormalizationComputation,
        normalized::coverage::NormalizationCoverage,
        attribution::ProviderCoverage
    );
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
        let records = enrich_all(&data, &invocation, definition, profile, budget)?;
        macro_rules! write{($($field:ident:$ty:ty,)*)=>{$(for row in records.$field.iter(){output.push(row.clone()).await?;})*};}
        write!(modeled_calls:ModeledCallEvaluation,modeled_arguments:ModeledCallArgument,modeled_native:ModeledCallNative,fresh_calls:SourceExecutionInvocation,fresh_arguments:SourceExecutionArgument,definition_evaluations:DefinitionEvaluation,definition_sources:DefinitionSource,definition_members:DefinitionMember,contexts:ContextExecution,context_items:ContextItem,context_sources:ContextSource,context_members:ContextMember,context_bindings:ContextEntryBinding,context_binding_sources:ContextBindingSource,context_binding_members:ContextBindingMember,executions:StatementExecution,outcomes:ExecutionOutcome,sources:ExecutionSource,members:ExecutionMember,entered:EnteredStatement,boundaries:ExecutionBoundary,bodies:BodyExecution,body_sources:BodySource,body_members:BodyMember,releases:BodyReleaseInput,body_boundaries:BodyBoundary,);
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
