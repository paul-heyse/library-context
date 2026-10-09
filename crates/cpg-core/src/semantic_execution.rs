//! Qualified base execution consumes actual completed Local/native/normalized inputs.
//! Behavioral Base, Completion and SourceCall execute fresh. Checking complete current domains
//! and all semantic output fields repeats their kernels; cached row replay would add work.
use crate::producer_operations::{Declaration, declare, declare_ordered, emit};
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use futures::future::BoxFuture;
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
#[path = "base_scope.rs"]
mod base_scope;
#[path = "execution_counts.rs"]
mod execution_counts;
#[path = "execution_scope.rs"]
mod execution_scope;
#[path = "source_call_scope.rs"]
mod source_call_scope;
fn load<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut CoverageAdmission<'_>,
    mut visit: impl FnMut(
        &ValidationInput,
        &analysis::sources::CompletedInput<R>,
        &arrow_array::RecordBatch,
    ) -> Result<(), ModelError>
    + Send
    + 'a,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
                .await?
            {
                continue;
            }
            crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
                admission.visit_if_expected(permit, batch)?;
                visit(&input, permit, batch)
            })
            .await?;
        }
        Ok(())
    })
}

// Inventories select typed adapters synchronously. Only the ordered driver is asynchronous.
type InputLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut CoverageAdmission<'sources>,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_ignored<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut CoverageAdmission<'_>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    load::<R>(access, session, consumed, admission, |_, _, _| Ok(()))
}
fn load_ordered<'a>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut CoverageAdmission<'_>,
    loaders: &'a [InputLoader],
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for load in loaders {
            load(access, session, consumed, admission).await?;
        }
        Ok(())
    })
}
macro_rules! ignored_adapters {($($field:ident:$ty:ty,)*) => { const LOADERS: &[InputLoader] = &[$(load_ignored::<$ty>,)*]; };}
macro_rules! consume_inventory {
    ($inventory:ident, $access:expr, $session:expr, $consumed:expr, $admission:expr) => {{
        lctx_model::$inventory!(ignored_adapters);
        load_ordered($access, $session, $consumed, $admission, LOADERS)
    }};
}
macro_rules! consume_types {
    ($access:expr, $session:expr, $consumed:expr, $admission:expr; $($ty:ty),* $(,)?) => {{
        const LOADERS: &[InputLoader] = &[$(load_ignored::<$ty>,)*];
        load_ordered($access, $session, $consumed, $admission, LOADERS)
    }};
}
macro_rules! rows_emitter {
    ($name:ident, $records:ty; $($field:ident),* $(,)?) => {
        fn $name<'a>(records: &'a $records, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> {
            type Emission = for<'a> fn(&'a $records, &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>>;
            $(fn $field<'a>(records: &'a $records, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&records.$field, output) })*
            const EMISSIONS: &[Emission] = &[$($field,)*];
            Box::pin(async move { for emission in EMISSIONS { emission(records, output).await?; } Ok(()) })
        }
    };
}
rows_emitter!(emit_evaluation, production::EvaluationRecords; evaluations, sources, members, operands, boundaries);
rows_emitter!(emit_completion_body, execution::completion_production::CompletionRecords; bodies, body_sources, body_members, body_releases, body_boundaries);
rows_emitter!(emit_completion_statement, execution::completion_production::CompletionRecords; completions, outcomes, sources, members, entered, boundaries);
rows_emitter!(emit_source_calls, execution::source_call_records::SourceCallRecords; headers, members, boundaries, invocations, releases, arguments, call_outcomes, invocation_boundaries);
rows_emitter!(emit_enriched, execution::enriched_production::ExecutionRecords; modeled_calls, modeled_arguments, modeled_native, fresh_calls, fresh_arguments, captured_entries, captured_values, definition_evaluations, definition_sources, definition_members, contexts, context_items, context_sources, context_members, context_bindings, context_binding_sources, context_binding_members, executions, outcomes, sources, members, entered, boundaries, bodies, body_sources, body_members, releases, body_boundaries);

/// Compiler lifetime bound around an opaque value minted by the actual model producer.
pub struct Produced<T> {
    premises: CompletedInputs,
    outputs: CompletedInputs,
    value: T,
    source_payloads: Option<source_call_scope::SourcePayloadSpool>,
}
impl<T> Produced<T> {
    pub(crate) fn borrow(
        &self,
        access: &CompletedInputs,
        runtime: &Workspace,
    ) -> Result<&T, ModelError> {
        self.premises.require_subset(runtime, access)?;
        self.outputs.require_subset(runtime, access)?;
        Ok(&self.value)
    }
}
fn produced_premises(
    access: &CompletedInputs,
    inputs: Vec<ValidationInput>,
) -> Result<CompletedInputs, ModelError> {
    access.select(
        &inputs
            .into_iter()
            .filter(|input| access.table_for(input).is_ok())
            .collect::<Vec<_>>(),
    )
}
pub async fn evaluate_base(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    actual_local: Option<&crate::local_semantics::PreparedLocal>,
) -> Result<Option<Produced<production::ProducedEvaluations>>, ModelError> {
    if *definition != execution::configuration::base_evaluation().1 {
        return Err(ModelError::Invalid(
            "base execution definition is not bound".into(),
        ));
    }
    let actual_local = actual_local
        .map(|actual| actual.entries(&access, runtime))
        .transpose()?;
    let mut produced: Option<production::ProducedEvaluations> = None;
    let premises = produced_premises(
        &access,
        execution::records::base_invariants().remove(0).inputs,
    )?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let data = EvaluationData::new(budget);
    let entry = EntryData::new(budget);
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
    if profile == Profile::Behavioral {
        consume_inventory!(
            execution_evaluation_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            entry_value_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
    }
    let entries = Rows::<EntryValueWitness>::new(budget);
    let entry_sources = Rows::<EntryAccessSource>::new(budget);
    let mut local = Rows::<analysis::local::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    if profile == Profile::Behavioral {
        load::<EntryValueWitness>(
            &access,
            &session,
            &mut consumed,
            &mut admission,
            |_, _, _| Ok(()),
        )
        .await?;
        load::<EntryAccessSource>(
            &access,
            &session,
            &mut consumed,
            &mut admission,
            |_, _, _| Ok(()),
        )
        .await?;
    }
    load::<analysis::local::AnalysisInvocation>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |_, _, batch| local.decode(batch),
    )
    .await?;
    load::<analysis::AnalysisDefinition>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |_, _, batch| definitions.decode(batch),
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "base execution definition absent from confirmed configuration".into(),
        ));
    }
    consume_inventory!(
        expected_domain_inputs,
        &access,
        &session,
        &mut consumed,
        &mut admission
    )
    .await?;
    consumed.finish(access.name())?;
    let scopes = if profile == Profile::Behavioral {
        Some(base_scope::BaseScopes::prepare(&access, &session, model, budget).await?)
    } else {
        None
    };
    let mut output_declarations: Vec<Declaration> = Vec::new();
    macro_rules! declare_outputs {($($ty:ty),*) => {$(output_declarations.push(declare::<$ty>);)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output_declarations.push(declare::<publication::$record>);)*};}
    lctx_model::analysis_publication!(common_publication);
    declare_outputs!(
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
    declare_ordered(&output, &output_declarations).await?;
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
        let (records, owner) = if profile == Profile::Behavioral {
            production::evaluate_all_with_local(
                &data,
                &entry,
                &entries,
                &entry_sources,
                &invocation,
                definition,
                profile,
                budget,
                actual_local.ok_or(ModelError::Conflict("actual Local entry owner absent"))?,
            )?
        } else {
            production::evaluate_all_produced(
                &data,
                &entry,
                &entries,
                &entry_sources,
                &invocation,
                definition,
                profile,
                budget,
            )?
        };
        match produced.as_mut() {
            Some(value) => value.append(owner)?,
            None => produced = Some(owner),
        };
        let mut run = records.run;
        let mut outcome = records.outcome;
        if let Some(scopes) = &scopes {
            use futures::TryStreamExt;
            let actual =
                actual_local.ok_or(ModelError::Conflict("actual Local entry owner absent"))?;
            let (mut reads, metadata) = scopes.metadata(&session, &invocation, budget).await?;
            let mut roots = crate::sql::query(
                &session,
                &scopes.roots(&invocation, base_scope::Kernel::Expression)?,
            )
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
            while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                let _transfer = budget.reserve(
                    "Base-expression-root-transfer",
                    batch.get_array_memory_size(),
                )?;
                for row in 0..batch.num_rows() {
                    runtime.cancellation().check()?;
                    let key = crate::scoped_admission::column(&batch, "id", row)?
                        .ok_or(ModelError::Schema("Base expression root ID"))?;
                    let scope = scopes
                        .scope(base_scope::Kernel::Expression, key, budget)
                        .await?;
                    let selected = scopes.data(&access, &scope, budget, true).await?;
                    let (records, owner) = production::evaluate_expression_scoped(
                        &selected.data,
                        &selected.entry,
                        &selected.witnesses,
                        &selected.sources,
                        &invocation,
                        definition,
                        profile,
                        budget,
                        execution_scope::nominal(&key)?,
                        actual,
                        &metadata,
                    )?;
                    produced
                        .as_mut()
                        .expect("actual Base frame owner")
                        .append(owner)?;
                    let add = |left: i64, right: i64| {
                        left.checked_add(right).ok_or_else(|| {
                            ModelError::Invalid("Base expression count overflow".into())
                        })
                    };
                    run.evaluated = add(run.evaluated, records.run.evaluated)?;
                    run.refused = add(run.refused, records.run.refused)?;
                    if records.run.refused != 0 {
                        outcome.status = analysis::AnalysisStatus::Partial;
                        outcome.reason = Some(obligation::ObligationKind::UnsupportedControlFlow);
                    }
                    emit_evaluation(&records, &output).await?;
                    drop(records);
                    drop(selected);
                    drop(scope);
                    tokio::task::yield_now().await;
                }
            }
            scopes
                .reads(
                    &access,
                    &session,
                    runtime,
                    &output,
                    &invocation,
                    actual,
                    &mut reads,
                )
                .await?;
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
                outcome.status,
                outcome.reason,
                budget,
            )?;
            output.push(row).await?;
            for row in premises {
                output.push(row).await?;
            }
        }
        output.push(run).await?;
        output.push(outcome).await?;
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
    if profile != Profile::Behavioral {
        return Ok(None);
    }
    let outputs = runtime.inputs(
        "actual-produced-values",
        profile,
        [
            publication::AnalysisInvocation::NAME,
            ExpressionEvaluation::NAME,
            EvaluationSource::NAME,
            EvaluationMember::NAME,
            EvaluationOperand::NAME,
        ],
    )?;
    Ok(produced.map(|value| Produced {
        premises,
        outputs,
        value,
        source_payloads: None,
    }))
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
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    evaluations: Option<&Produced<production::ProducedEvaluations>>,
) -> Result<Option<Produced<execution::completion_production::ProducedBodies>>, ModelError> {
    if *definition != execution::configuration::base_completion().1 {
        return Err(ModelError::Invalid(
            "base completion definition is not bound".into(),
        ));
    }
    let evaluations = evaluations
        .map(|owner| owner.borrow(&access, runtime))
        .transpose()?;
    let mut produced: Option<execution::completion_production::ProducedBodies> = None;
    let premises = produced_premises(
        &access,
        execution::records::base_invariants().remove(0).inputs,
    )?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let data = execution::completion_production::CompletedEvaluations::new(budget);
    let scopes = if profile == Profile::Behavioral {
        Some(execution_scope::CompletionScopes::prepare(&access, &session, model, budget).await?)
    } else {
        None
    };
    let mut declarations =
        execution::completion_production::CompletedEvaluations::consumed_inputs(profile);
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
    if profile == Profile::Behavioral {
        consume_inventory!(
            execution_evaluation_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            entry_value_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
    }
    let mut base = Rows::<analysis::base_evaluation::AnalysisInvocation>::new(budget);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(budget);
    if profile == Profile::Behavioral {
        consume_types!(&access, &session, &mut consumed, &mut admission;
            EntryValueWitness,
            EntryAccessSource,
            ExpressionEvaluation,
            EvaluationSource,
            EvaluationMember,
            EvaluationOperand
        )
        .await?;
    }
    load::<analysis::base_evaluation::AnalysisInvocation>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| {
            base.decode(batch)?;
            let _ = input;
            Ok(())
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
            let _ = input;
            Ok(())
        },
    )
    .await?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "base completion definition absent from confirmed configuration".into(),
        ));
    }
    consume_inventory!(
        expected_domain_inputs,
        &access,
        &session,
        &mut consumed,
        &mut admission
    )
    .await?;
    consumed.finish(access.name())?;
    let mut output_declarations: Vec<Declaration> = Vec::new();
    macro_rules! declare_outputs {($($ty:ty),*) => {$(output_declarations.push(declare::<$ty>);)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output_declarations.push(declare::<completion_publication::$record>);)*};}
    lctx_model::analysis_publication!(common_publication);
    declare_outputs!(
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
    declare_ordered(&output, &output_declarations).await?;
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
        match produced.as_mut() {
            Some(value) => value.append(owner)?,
            None => produced = Some(owner),
        };
        let mut run = records.run;
        let mut outcome = records.outcome;
        if let Some(scopes) = &scopes {
            use futures::TryStreamExt;
            let actual = evaluations.ok_or(ModelError::Conflict(
                "base evaluation owner authority absent",
            ))?;
            for body in [false, true] {
                let mut roots = crate::sql::query(&session, &scopes.roots(frame.input, body)?)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                    let _root_transfer = budget
                        .reserve("completion-root-transfer", batch.get_array_memory_size())?;
                    for row in 0..batch.num_rows() {
                        runtime.cancellation().check()?;
                        let id = crate::scoped_admission::column(&batch, "id", row)?
                            .ok_or(ModelError::Schema("completion root absent"))?;
                        let records = if body {
                            let callable: Id<normalized::entities::CallableEntity> =
                                execution_scope::nominal(&id)?;
                            let scope = scopes.body(callable, budget).await?;
                            let selected = scopes.data(&access, &scope, budget).await?;
                            let owner = normalized::entities::EntityRef::Callable { callable }.id();
                            let (records, owner) =
                                execution::completion_production::complete_body_produced(
                                    &selected,
                                    &invocation,
                                    definition,
                                    profile,
                                    budget,
                                    actual,
                                    owner,
                                )?;
                            produced
                                .as_mut()
                                .expect("actual frame owner")
                                .append(owner)?;
                            records
                        } else {
                            let statement: Id<source::Occurrence> = execution_scope::nominal(&id)?;
                            let scope = scopes.statement(statement, budget).await?;
                            let selected = scopes.data(&access, &scope, budget).await?;
                            let (records, _) =
                                execution::completion_production::complete_statement_produced(
                                    &selected,
                                    &invocation,
                                    definition,
                                    profile,
                                    budget,
                                    actual,
                                    statement,
                                )?;
                            records
                        };
                        let add = |left: i64, right: i64| {
                            left.checked_add(right).ok_or_else(|| {
                                ModelError::Invalid("completion inventory count overflow".into())
                            })
                        };
                        if body {
                            run.bodied = add(run.bodied, records.run.bodied)?;
                            run.body_refused = add(run.body_refused, records.run.body_refused)?;
                            emit_completion_body(&records, &output).await?;
                            for row in records.outcomes.iter().filter(|row| {
                                records.bodies.iter().any(|body| body.outcome == row.id())
                            }) {
                                output.push(row.clone()).await?;
                            }
                            if records.run.body_refused != 0 {
                                outcome.status = analysis::AnalysisStatus::Partial;
                                outcome.reason =
                                    Some(obligation::ObligationKind::UnsupportedControlFlow);
                            }
                        } else {
                            run.completed = add(run.completed, records.run.completed)?;
                            run.refused = add(run.refused, records.run.refused)?;
                            emit_completion_statement(&records, &output).await?;
                            if records.run.refused != 0 {
                                outcome.status = analysis::AnalysisStatus::Partial;
                                outcome.reason =
                                    Some(obligation::ObligationKind::UnsupportedControlFlow);
                            }
                        }
                        tokio::task::yield_now().await;
                    }
                }
            }
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
            let (row, premises) = completion_publication::coverage::assess(
                scope.expectation(),
                scope.observations(),
                outcome.status,
                outcome.reason,
                budget,
            )?;
            output.push(row).await?;
            for row in premises {
                output.push(row).await?;
            }
        }
        output.push(run).await?;
        output.push(outcome).await?;
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
    if profile != Profile::Behavioral {
        return Ok(None);
    }
    let outputs = runtime.inputs(
        "actual-produced-values",
        profile,
        [
            completion_publication::AnalysisInvocation::NAME,
            SourceBodyCompletion::NAME,
            BodySource::NAME,
            BodyMember::NAME,
            BodyReleaseInput::NAME,
            StatementCompletion::NAME,
        ],
    )?;
    Ok(produced.map(|value| Produced {
        premises,
        outputs,
        value,
        source_payloads: None,
    }))
}

/// Fresh source binding consumes acknowledged normalized shapes and earlier completion frames.
#[allow(
    clippy::too_many_arguments,
    reason = "Input and output descriptors, workspace, configuration, binding authority and actual Base and Body owners are independent inputs."
)]
pub async fn prepare_source_calls(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
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
    let evaluations = evaluations
        .map(|owner| owner.borrow(&access, runtime))
        .transpose()?;
    let bodies = bodies
        .map(|owner| owner.borrow(&access, runtime))
        .transpose()?;
    let application = if access.profile() == Profile::Behavioral {
        Some(
            bindings
                .ok_or_else(|| {
                    ModelError::Invalid("normalized application authority absent".into())
                })?
                .application(&access, runtime)?,
        )
    } else {
        None
    };
    let mut produced: Option<execution::source_call_records::ProducedSourceCalls> = None;
    let premises = produced_premises(
        &access,
        execution::source_call_records::SourceCallData::inputs(),
    )?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut data = SourceCallData::new(budget);
    let scopes = if profile == Profile::Behavioral {
        Some(source_call_scope::SourceCallScopes::prepare(&access, &session, model, budget).await?)
    } else {
        None
    };
    let mut private_payloads = source_call_scope::SourcePayloadSpool::new(budget);
    let mut declarations = SourceCallData::consumed_inputs(profile);
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
    if profile == Profile::Behavioral {
        consume_inventory!(
            execution_evaluation_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            entry_value_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            normalized_binding_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            normalized_binding_outputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
    }
    if profile == Profile::Behavioral {
        consume_types!(&access, &session, &mut consumed, &mut admission;
            syntax::ParameterSyntaxObservation,
            EntryValueWitness,
            EntryAccessSource,
            ExpressionEvaluation,
            EvaluationSource,
            EvaluationMember,
            EvaluationOperand,
            analysis::base_evaluation::AnalysisInvocation,
            SourceBodyCompletion,
            BodySource,
            BodyMember,
            BodyReleaseInput,
            StatementCompletion
        )
        .await?;
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
    consume_inventory!(
        expected_domain_inputs,
        &access,
        &session,
        &mut consumed,
        &mut admission
    )
    .await?;
    consumed.finish(access.name())?;
    let mut output_declarations: Vec<Declaration> = Vec::new();
    macro_rules! declare_outputs {($($ty:ty),*) => {$(output_declarations.push(declare::<$ty>);)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output_declarations.push(declare::<owner::$record>);)*};}
    lctx_model::analysis_publication!(common_publication);
    declare_outputs!(
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
    declare_ordered(&output, &output_declarations).await?;
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
        let (records, owner) =
            prepare_empty_produced(&data, &invocation, definition, profile, budget)?;
        match produced.as_mut() {
            Some(value) => value.append(owner)?,
            None => produced = Some(owner),
        };
        let mut run = records.run;
        let mut outcome = records.outcome;
        if let Some(scopes) = &scopes {
            use futures::TryStreamExt;
            let mut roots = crate::sql::query(&session, &scopes.roots(&invocation)?)
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
            while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                let _transfer =
                    budget.reserve("SourceCall-root-transfer", logical_batch_bytes(&batch)?)?;
                for start in (0..batch.num_rows()).step_by(32) {
                    let end = (start + 32).min(batch.num_rows());
                    let _keys = budget.reserve(
                        "SourceCall-root-window",
                        (end - start) * size_of::<[u8; 16]>(),
                    )?;
                    let keys = (start..end)
                        .map(|row| {
                            crate::scoped_admission::column(&batch, "id", row)?
                                .ok_or(ModelError::Schema("SourceCall root ID"))
                        })
                        .collect::<Result<Vec<_>, ModelError>>()?;
                    let window = scopes
                        .event_window(&keys, budget, &runtime.cancellation())
                        .await?;
                    for (partition, key) in keys.iter().enumerate() {
                        runtime.cancellation().check()?;
                        let event = execution_scope::nominal(key)?;
                        let selected = window.data(partition, budget, &runtime.cancellation())?;
                        let (records, owner, payload) = prepare_event_produced(
                            &selected,
                            &invocation,
                            definition,
                            profile,
                            budget,
                            application,
                            evaluations,
                            bodies,
                            event,
                        )?;
                        private_payloads.insert(invocation.id(), event, payload.bytes())?;
                        drop(payload);
                        produced
                            .as_mut()
                            .expect("actual SourceCall frame")
                            .append(owner)?;
                        run.bound = run
                            .bound
                            .checked_add(records.run.bound)
                            .ok_or(ModelError::Conflict("SourceCall bound count overflow"))?;
                        run.refused = run
                            .refused
                            .checked_add(records.run.refused)
                            .ok_or(ModelError::Conflict("SourceCall refused count overflow"))?;
                        if records.outcome.status == analysis::AnalysisStatus::Partial {
                            outcome.status = records.outcome.status;
                            outcome.reason = records.outcome.reason;
                        }
                        emit_source_calls(&records, &output).await?;
                        drop(records);
                        drop(selected);
                        tokio::task::yield_now().await;
                    }
                }
            }
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
                outcome.status,
                outcome.reason,
                budget,
            )?;
            output.push(row).await?;
            for row in premises {
                output.push(row).await?;
            }
        }
        output.push(run).await?;
        output.push(outcome).await?;
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
    if profile != Profile::Behavioral {
        return Ok(None);
    }
    let outputs = runtime.inputs(
        "actual-produced-values",
        profile,
        [
            analysis::source_call::AnalysisInvocation::NAME,
            analysis::source_call::AnalysisOutcome::NAME,
            execution::source_call_records::SourceCallRun::NAME,
            execution::source_call_records::SourceCallHeader::NAME,
            execution::source_call_records::HeaderMember::NAME,
            execution::source_call_records::SourceCallBoundary::NAME,
            execution::source_call_records::SourceInvocation::NAME,
            execution::source_call_records::SourceFrameRelease::NAME,
            execution::source_call_records::SourceFrameArgument::NAME,
            execution::source_call_records::SourceCallOutcome::NAME,
            execution::source_call_records::InvocationBoundary::NAME,
        ],
    )?;
    Ok(produced.map(|value| Produced {
        premises,
        outputs,
        value,
        source_payloads: Some(private_payloads),
    }))
}

/// Fresh source binding consumes acknowledged normalized shapes and earlier completion frames.
#[allow(
    clippy::too_many_arguments,
    reason = "Selected descriptor streams, workspace, configuration and independent binding, Base and SourceCall authorities stay explicit."
)]
pub async fn enrich(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
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
    let evaluations = evaluations
        .map(|owner| owner.borrow(&access, runtime))
        .transpose()?;
    let source_values = source_calls
        .map(|owner| owner.borrow(&access, runtime))
        .transpose()?;
    let application = if access.profile() == Profile::Behavioral {
        Some(
            bindings
                .ok_or_else(|| {
                    ModelError::Invalid("normalized application authority absent".into())
                })?
                .application(&access, runtime)?,
        )
    } else {
        None
    };
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut data = EnrichedData::new(budget);
    let scopes = if profile == Profile::Behavioral {
        Some(
            source_call_scope::SourceCallScopes::prepare_enriched(&access, &session, model, budget)
                .await?,
        )
    } else {
        None
    };
    let mut declarations = EnrichedData::consumed_inputs(profile);
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
    if profile == Profile::Behavioral {
        consume_inventory!(
            execution_evaluation_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            entry_value_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            normalized_binding_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            normalized_binding_outputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
        consume_inventory!(
            model_pin_inputs,
            &access,
            &session,
            &mut consumed,
            &mut admission
        )
        .await?;
    }
    if profile == Profile::Behavioral {
        consume_types!(&access, &session, &mut consumed, &mut admission;
            syntax::ParameterSyntaxObservation,
            EntryValueWitness,
            EntryAccessSource,
            ExpressionEvaluation,
            EvaluationSource,
            EvaluationMember,
            EvaluationOperand,
            analysis::base_evaluation::AnalysisInvocation,
            SourceBodyCompletion,
            execution::body_records::BodySource,
            execution::body_records::BodyMember,
            execution::body_records::BodyReleaseInput,
            StatementCompletion,
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
        )
        .await?;
    }
    load::<analysis::MethodParameters>(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        |input, _, batch| data.visit_input(input, batch),
    )
    .await?;
    let catalog = data
        .parameters
        .get(definition.parameters)
        .and_then(|row| row.model_catalog)
        .ok_or(ModelError::Conflict(
            "Enriched selected model catalog absent",
        ))?;
    while let Some((input, permit)) = consumed.next::<models::ModelCatalog>(&access)? {
        crate::consumed_rows::stream_where_at(
            &permit,
            &input,
            &access,
            &session,
            Some(&execution_scope::predicate(catalog)),
            |_, batch| data.visit_input(&input, batch),
        )
        .await?;
    }
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
    consume_inventory!(
        expected_domain_inputs,
        &access,
        &session,
        &mut consumed,
        &mut admission
    )
    .await?;
    consumed.finish(access.name())?;
    let mut output_declarations: Vec<Declaration> = Vec::new();
    macro_rules! declare_outputs {($($ty:ty),*) => {$(output_declarations.push(declare::<$ty>);)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output_declarations.push(declare::<owner::$record>);)*};}
    lctx_model::analysis_publication!(common_publication);
    declare_outputs!(
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
    declare_ordered(&output, &output_declarations).await?;
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
        let mut work = EnrichedWork::new(&invocation, budget)?;
        let mut parent = base
            .iter()
            .filter(|row| (row.input, row.context) == (frame.input, frame.context));
        let parent = parent.next().ok_or(ModelError::Conflict(
            "Enriched actual SourceCall frame absent",
        ))?;
        if base
            .iter()
            .filter(|row| (row.input, row.context) == (frame.input, frame.context))
            .count()
            != 1
        {
            return Err(ModelError::Conflict(
                "Enriched actual SourceCall frame ambiguous",
            ));
        }
        let empty_values = source_values
            .map(|values| values.hydrate_empty(parent, budget))
            .transpose()?;
        let records = enrich_empty_produced(
            &data,
            &invocation,
            definition,
            profile,
            budget,
            empty_values.as_ref(),
            &mut work,
        )?;
        let mut run = records.run;
        let mut outcome = records.outcome;
        drop(empty_values);
        if let Some(scopes) = &scopes {
            use futures::TryStreamExt;
            let values = source_values.ok_or(ModelError::Conflict(
                "Enriched actual SourceCall owner absent",
            ))?;
            let spool = source_calls
                .and_then(|owner| owner.source_payloads.as_ref())
                .ok_or(ModelError::Conflict(
                    "Enriched private SourceCall payload owner absent",
                ))?;
            let mut counts = execution_counts::ExecutionCounts::new(budget)?;
            for unowned in [false, true] {
                let mut roots =
                    crate::sql::query(&session, &scopes.owner_roots(&invocation, unowned)?)
                        .await
                        .map_err(ModelError::codec)?
                        .execute_stream()
                        .await
                        .map_err(ModelError::codec)?;
                while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                    let _transfer =
                        budget.reserve("Enriched-root-transfer", logical_batch_bytes(&batch)?)?;
                    for start in (0..batch.num_rows()).step_by(32) {
                        let end = (start + 32).min(batch.num_rows());
                        let _keys = budget.reserve(
                            "Enriched-root-window",
                            (end - start) * size_of::<[u8; 16]>(),
                        )?;
                        let keys = (start..end)
                            .map(|row| {
                                crate::scoped_admission::column(&batch, "id", row)?
                                    .ok_or(ModelError::Schema("Enriched root ID"))
                            })
                            .collect::<Result<Vec<_>, ModelError>>()?;
                        let window = scopes
                            .owner_window(&keys, unowned, budget, &runtime.cancellation())
                            .await?;
                        for (partition, key) in keys.iter().enumerate() {
                            runtime.cancellation().check()?;
                            let mut selected =
                                window.enriched_data(partition, budget, &runtime.cancellation())?;
                            source_call_scope::enriched_configuration(&data, &mut selected)?;
                            let records = if unowned {
                                enrich_unowned_statement(
                                    &selected,
                                    &invocation,
                                    definition,
                                    profile,
                                    budget,
                                    execution_scope::nominal(key)?,
                                    &mut work,
                                )?
                            } else {
                                let hydrated = source_call_scope::hydrate_selected(
                                    values, spool, parent, &selected, budget,
                                )?;
                                enrich_owner_produced(
                                    &selected,
                                    &invocation,
                                    definition,
                                    profile,
                                    budget,
                                    application,
                                    evaluations,
                                    Some(&hydrated),
                                    execution_scope::nominal(key)?,
                                    &mut work,
                                )?
                            };
                            for row in records.executions.iter() {
                                counts.push(0, row)?;
                            }
                            for row in records.boundaries.iter() {
                                counts.push(1, row)?;
                            }
                            for row in records.bodies.iter() {
                                counts.push(2, row)?;
                            }
                            for row in records.body_boundaries.iter() {
                                counts.push(3, row)?;
                            }
                            emit_enriched(&records, &output).await?;
                            drop(records);
                            drop(selected);
                            tokio::task::yield_now().await;
                        }
                    }
                }
            }
            let [executed, refused, bodied, body_refused] = counts.finish(&session).await?;
            run.executed = executed;
            run.refused = refused;
            run.bodied = bodied;
            run.body_refused = body_refused;
            if refused != 0 || body_refused != 0 {
                outcome.status = analysis::AnalysisStatus::Partial;
                outcome.reason = Some(obligation::ObligationKind::UnsupportedControlFlow);
            }
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
                outcome.status,
                outcome.reason,
                budget,
            )?;
            output.push(row).await?;
            for row in premises {
                output.push(row).await?;
            }
        }
        output.push(run).await?;
        output.push(outcome).await?;
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
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    async fn publish(runtime: &Arc<Workspace>, name: &'static str, artifact: SourceArtifact) {
        let output = runtime.output(
            name,
            Profile::Catalog,
            ContentHash::of(name.as_bytes()),
            runtime.inputs(name, Profile::Catalog, []).unwrap(),
            [<SourceArtifact>::NAME],
        );
        output.declare::<SourceArtifact>().unwrap();
        output.push(artifact).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
    }
    #[tokio::test]
    async fn producer_borrow_refuses_foreign_attempt_profile_and_changed_descriptor() {
        let model = Arc::new(model().unwrap());
        let runtime = Workspace::new(
            model.clone(),
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        let artifact = SourceArtifact::from_bytes(nominal(1), "source.py".into(), b"x").unwrap();
        publish(&runtime, "first", artifact.clone()).await;
        let selected = runtime
            .inputs("consumer", Profile::Catalog, [SourceArtifact::NAME])
            .unwrap();
        {
            let session = datafusion::prelude::SessionContext::new();
            let sources = CapturedSources::capture(
                selected.profile(),
                selected.snapshots(),
                runtime.budget(),
            )
            .unwrap();
            let mut admission = CoverageAdmission::new(&sources, runtime.budget()).unwrap();
            let mut unpolled = crate::consumed_rows::ConsumedInputs::new(
                vec![ValidationInput::of::<SourceArtifact>(&["id"])],
                runtime.budget(),
            )
            .unwrap();
            drop(load_ignored::<SourceArtifact>(
                &selected,
                &session,
                &mut unpolled,
                &mut admission,
            ));
            assert!(
                unpolled.finish("unpolled-control").is_err(),
                "an unpolled adapter must not dispatch its declaration"
            );

            let first_error = selected
                .read::<analysis::AnalysisDefinition>()
                .err()
                .unwrap()
                .to_string();
            let mut ordered = crate::consumed_rows::ConsumedInputs::new(
                vec![
                    ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
                    ValidationInput::of::<SourceArtifact>(&["id"]),
                ],
                runtime.budget(),
            )
            .unwrap();
            let loaders: &[InputLoader] = &[
                load_ignored::<analysis::AnalysisDefinition>,
                load_ignored::<SourceArtifact>,
            ];
            let error = load_ordered(&selected, &session, &mut ordered, &mut admission, loaders)
                .await
                .unwrap_err();
            assert_eq!(
                error.to_string(),
                first_error,
                "the first missing typed input must retain error precedence"
            );
            assert!(
                ordered.next::<SourceArtifact>(&selected).unwrap().is_some(),
                "a later declaration must remain undispatched after refusal"
            );
        }
        let produced = Produced {
            premises: selected.clone(),
            outputs: selected.clone(),
            value: 7u8,
            source_payloads: None,
        };
        assert_eq!(*produced.borrow(&selected, &runtime).unwrap(), 7);
        let other = Workspace::new(
            model,
            WorkspaceOptions::default(),
            crate::test_native::store(),
        )
        .unwrap();
        publish(&other, "same-content", artifact).await;
        let foreign = other
            .inputs("consumer", Profile::Catalog, [SourceArtifact::NAME])
            .unwrap();
        assert!(matches!(
            produced.borrow(&foreign, &other),
            Err(ModelError::Conflict(_))
        ));
        let changed_profile = runtime
            .inputs("consumer", Profile::Behavioral, [SourceArtifact::NAME])
            .unwrap();
        assert!(matches!(
            produced.borrow(&changed_profile, &runtime),
            Err(ModelError::Conflict(_))
        ));
        let contribution = runtime.output(
            "second",
            Profile::Catalog,
            ContentHash::of(b"second"),
            runtime.inputs("second", Profile::Catalog, []).unwrap(),
            [<SourceArtifact>::NAME],
        );
        let batch = Batch::new(
            runtime.model(),
            vec![SourceArtifact::from_bytes(nominal(1), "second.py".into(), b"y").unwrap()],
            runtime.budget(),
        )
        .unwrap();
        contribution.contribute(batch).unwrap();
        contribution
            .finish(ProviderOutcome::Complete)
            .await
            .unwrap();
        let changed = runtime
            .inputs("consumer", Profile::Catalog, [SourceArtifact::NAME])
            .unwrap();
        assert!(matches!(
            produced.borrow(&changed, &runtime),
            Err(ModelError::Conflict(_))
        ));
        assert_eq!(*produced.borrow(&selected, &runtime).unwrap(), 7);
    }
    #[test]
    fn requested_production_refuses_absent_predecessor_owners() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let invocation = analysis::base_completion::AnalysisInvocation::new(
            nominal(1),
            nominal(2),
            execution::configuration::base_completion().1.id(),
            None,
            [],
        )
        .0;
        assert!(matches!(
            execution::completion_production::complete_all_produced(
                &execution::completion_production::CompletedEvaluations::new(&budget),
                &invocation,
                &execution::configuration::base_completion().1,
                Profile::Behavioral,
                &budget,
                None,
            ),
            Err(ModelError::Conflict(_))
        ));
        let (records, _) = execution::completion_production::complete_all_produced(
            &execution::completion_production::CompletedEvaluations::new(&budget),
            &invocation,
            &execution::configuration::base_completion().1,
            Profile::Catalog,
            &budget,
            None,
        )
        .unwrap();
        assert_eq!(
            records.outcome.status,
            analysis::AnalysisStatus::NotRequested
        );
        let invocation = analysis::source_call::AnalysisInvocation::new(
            nominal(1),
            nominal(2),
            execution::configuration::source_calls().1.id(),
            None,
            [],
        )
        .0;
        assert!(matches!(
            execution::source_call_records::prepare_all_produced(
                &execution::source_call_records::SourceCallData::new(&budget),
                &invocation,
                &execution::configuration::source_calls().1,
                Profile::Behavioral,
                &budget,
                None,
                None,
                None,
            ),
            Err(ModelError::Conflict(_))
        ));
        let (records, _) = execution::source_call_records::prepare_all_produced(
            &execution::source_call_records::SourceCallData::new(&budget),
            &invocation,
            &execution::configuration::source_calls().1,
            Profile::Catalog,
            &budget,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            records.outcome.status,
            analysis::AnalysisStatus::NotRequested
        );
    }
}
