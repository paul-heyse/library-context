//! Model-owned analysis over completed native/normalized/Enriched inputs.
use crate::producer_operations;
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use arrow_array::Array;
use futures::{TryStreamExt, future::BoxFuture};
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, model as owner, sources::CapturedSources},
    execution::{model_production::*, model_rules::*},
    normalized::Rows,
    stages::*,
    *,
};
use std::sync::Arc;
mod scope;
fn load<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut CoverageAdmission<'sources>,
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

fn load_selected_catalog<'a>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    data: &'a mut ModelData,
    selected: Id<models::ModelCatalog>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<models::ModelCatalog>(access)? {
            crate::consumed_rows::stream_where_at(
                &permit,
                &input,
                access,
                session,
                Some(&format!("id={}", scope::hex(selected))),
                |_, batch| data.visit_input(&input, batch),
            )
            .await?;
        }
        Ok(())
    })
}

type InputReader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut CoverageAdmission<'sources>,
    &'a mut ModelData,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn read_metadata<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut CoverageAdmission<'sources>,
    data: &'a mut ModelData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    load::<R>(access, session, consumed, admission, |input, _, batch| {
        data.visit_input(input, batch)
    })
}
fn read_expected<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut CoverageAdmission<'sources>,
    _data: &'a mut ModelData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    load::<R>(access, session, consumed, admission, |_, _, _| Ok(()))
}
fn load_inputs<'a, 'sources>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    definition: &'a analysis::AnalysisDefinition,
    admission: &'a mut CoverageAdmission<'sources>,
    budget: &'a resources::ResourceBudget,
) -> BoxFuture<'a, Result<ModelData, ModelError>> {
    Box::pin(async move {
        let mut data = ModelData::new(budget);
        // The only resident global domain is finite configuration plus compact publication frames.
        let mut declarations = vec![
            ValidationInput::of::<models::ModelCatalog>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),
            ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ];
        declarations.extend(analysis::expected::inputs(definition.method));
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, budget)?;
        macro_rules! read {($($ty:ty),*)=>{{
        const READERS: &[InputReader] = &[$(read_metadata::<$ty>,)*];
        for read in READERS { read(access, session, &mut consumed, admission, &mut data).await?; }
    }};}
        read!(
            analysis::MethodParameters,
            analysis::AnalysisDefinition,
            analysis::enriched_execution::AnalysisInvocation,
            analysis::source_call::AnalysisInvocation,
            analysis::local::AnalysisInvocation,
            attribution::ProviderRun
        );
        let selected = data
            .parameters
            .get(definition.parameters)
            .and_then(|row| row.model_catalog)
            .ok_or_else(|| ModelError::Invalid("Models selected catalog absent".into()))?;
        load_selected_catalog(access, session, &mut consumed, &mut data, selected).await?;
        if data.definitions.get(definition.id()) != Some(definition) {
            return Err(ModelError::Invalid(
                "Model definition absent from confirmed configuration".into(),
            ));
        }
        macro_rules! expected {($($field:ident:$ty:ty,)*)=>{{
        const READERS: &[InputReader] = &[$(read_expected::<$ty>,)*];
        for read in READERS { read(access, session, &mut consumed, admission, &mut data).await?; }
    }};}
        lctx_model::expected_domain_inputs!(expected);
        consumed.finish(access.name())?;
        Ok(data)
    })
}

fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! declare {($($ty:ty),*)=>{{
        const DECLARATIONS: &[producer_operations::Declaration] = &[$(producer_operations::declare::<$ty>,)*];
        producer_operations::declare_ordered(output, DECLARATIONS).await?;
    }};}
        macro_rules! common_publication {($($record:ident,)*)=>{declare!($(owner::$record),*);};}
        lctx_model::analysis_publication!(common_publication);
        declare!(
            execution::closed_targets::ClosedTargetAssessment,
            execution::protocol_interpretation::ProtocolActionAssessment,
            execution::protocol_interpretation::TerminalFrontierAssessment,
            execution::protocol_interpretation::ConditionalTerminalFrontier,
            execution::protocol_interpretation::NormalContinuationRestriction,
            execution::protocol_interpretation::NativeExitCharacterization,
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
        Ok(())
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "Descriptor streams, workspace, selected configuration and binding, Base and Local owners are independently validated."
)]
pub async fn apply(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    bindings: Option<&crate::analysis_bindings::PreparedBindings>,
    evaluations: Option<
        &crate::semantic_execution::Produced<execution::production::ProducedEvaluations>,
    >,
    local: Option<&crate::local_semantics::PreparedLocal>,
) -> Result<(), ModelError> {
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let data = load_inputs(&access, &session, definition, &mut admission, budget).await?;
    let selected = data
        .parameters
        .get(definition.parameters)
        .and_then(|row| row.model_catalog)
        .ok_or_else(|| ModelError::Invalid("Models selected catalog absent".into()))?;
    let parsed = SelectedCatalog::read(
        data.catalogs
            .get(selected)
            .ok_or(ModelError::Schema(models::ModelCatalog::NAME))?,
        budget,
    )?;
    let actual = if profile == Profile::Behavioral {
        Some(ActualInputs {
            evaluations: evaluations
                .ok_or_else(|| ModelError::Invalid("Models actual Base values absent".into()))?
                .borrow(&access, runtime)?,
            local: local
                .ok_or_else(|| ModelError::Invalid("Models actual Local values absent".into()))?
                .entries(&access, runtime)?,
        })
    } else {
        None
    };
    let verified = if profile == Profile::Behavioral {
        Some(
            bindings
                .ok_or_else(|| ModelError::Invalid("Models actual binding values absent".into()))?
                .application(&access, runtime)?,
        )
    } else {
        None
    };
    let scopes = if profile == Profile::Behavioral {
        Some(scope::ModelScopes::prepare(&access, &session, _model, &parsed, budget).await?)
    } else {
        None
    };
    declare_outputs(&output).await?;
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
        let mut records = if let Some(scopes) = &scopes {
            let window = scopes
                .window(
                    &access,
                    &[ProductionScope::Frame],
                    frame.id(),
                    budget,
                    &runtime.cancellation(),
                )
                .await?;
            let selected = window.data(0, budget, &runtime.cancellation())?;
            apply_selected(
                &selected,
                &invocation,
                definition,
                profile,
                &parsed,
                verified,
                ProductionScope::Frame,
                actual.as_ref(),
                budget,
            )?
        } else {
            apply_selected(
                &data,
                &invocation,
                definition,
                profile,
                &parsed,
                None,
                ProductionScope::Frame,
                None,
                budget,
            )?
        };
        if let Some(scopes) = &scopes {
            let cancellation = runtime.cancellation();
            let application = SelectedApplication {
                access: &access,
                output: &output,
                scopes,
                invocation: &invocation,
                definition,
                profile,
                catalog: &parsed,
                verified,
                actual: actual.as_ref(),
                frame: frame.id(),
                budget,
                cancellation: &cancellation,
            };
            for targets in parsed.catalog().models().chunks(32) {
                let _keys = budget.reserve(
                    "model-target-window",
                    targets.len() * size_of::<ProductionScope>(),
                )?;
                let kinds = targets
                    .iter()
                    .map(|compiled| ProductionScope::Target(compiled.declaration().id()))
                    .collect::<Vec<_>>();
                let window = scopes
                    .window(&access, &kinds, frame.id(), budget, &runtime.cancellation())
                    .await?;
                for (partition, kind) in kinds.iter().enumerate() {
                    let selected = window.data(partition, budget, &runtime.cancellation())?;
                    let produced = apply_data(&application, &selected, *kind)?;
                    merge_run(&mut records, &produced)?;
                    publish_records(&output, &produced).await?;
                }
            }
            let parent = data
                .enriched
                .iter()
                .find(|p| {
                    (p.input, p.context) == (frame.input, frame.context) && p.subject.is_none()
                })
                .ok_or(ModelError::Schema(
                    analysis::enriched_execution::AnalysisInvocation::NAME,
                ))?;
            let selector = scopes;
            macro_rules! roots {
                ($ty:ty,$inventory:expr,$kind:expr) => {{
                    let (query, _query_charge) = selector.root_sql(
                        $inventory,
                        parent.id(),
                        frame.input,
                        frame.context,
                        budget,
                    )?;
                    apply_roots::<$ty>(&application, &session, query, $kind, &mut records).await?;
                }};
            }
            use compiler_scope_program::ModelRootInventory as R;
            roots!(
                execution::context_execution::ContextExecution,
                R::Context,
                ProductionScope::Context
            );
            roots!(
                normalized::events::NormalizedCallEvent,
                R::Event,
                ProductionScope::Event
            );
            roots!(
                protocols::NativeTerminalObservation,
                R::Terminal,
                ProductionScope::Terminal
            );
            roots!(
                protocols::NativeExitObservation,
                R::Exit,
                ProductionScope::Exit
            );
        }
        publish_frame(
            &output,
            &coverage,
            records,
            invocation,
            parents,
            inputs,
            receipts,
            projections,
            budget,
        )
        .await?;
    }
    drop(data);
    output.finish(ProviderOutcome::Complete).await
}

#[allow(
    clippy::too_many_arguments,
    reason = "Coverage, selected model result and charged frame receipts have distinct owners and publication order."
)]
fn publish_frame<'a>(
    output: &'a ProducerOutput,
    coverage: &'a owner::coverage::AdmittedCoverage,
    records: ModelRecords,
    invocation: owner::AnalysisInvocation,
    parents: Rows<owner::InvocationSource>,
    inputs: Vec<owner::AnalysisInput>,
    receipts: Vec<owner::SourceReceipt>,
    projections: Vec<owner::ProjectionInput>,
    budget: &'a resources::ResourceBudget,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
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
        Ok(())
    })
}

// Borrow only the selected model application's semantic owners, without copying rich inputs.
struct SelectedApplication<'a, 'actual> {
    access: &'a CompletedInputs,
    output: &'a ProducerOutput,
    scopes: &'a scope::ModelScopes,
    invocation: &'a owner::AnalysisInvocation,
    definition: &'a analysis::AnalysisDefinition,
    profile: Profile,
    catalog: &'a SelectedCatalog,
    verified: Option<&'a normalized::binding_normalization::VerifiedBindings>,
    actual: Option<&'a ActualInputs<'actual>>,
    frame: Id<attribution::ProviderRun>,
    budget: &'a resources::ResourceBudget,
    cancellation: &'a crate::workspace::Cancellation,
}
fn apply_data(
    application: &SelectedApplication<'_, '_>,
    selected: &ModelData,
    kind: ProductionScope,
) -> Result<ModelRecords, ModelError> {
    apply_selected(
        selected,
        application.invocation,
        application.definition,
        application.profile,
        application.catalog,
        application.verified,
        kind,
        application.actual,
        application.budget,
    )
}
fn apply_roots<'a, R: Record>(
    application: &'a SelectedApplication<'_, '_>,
    session: &'a datafusion::prelude::SessionContext,
    sql: String,
    kind: fn(Id<R>) -> ProductionScope,
    records: &'a mut ModelRecords,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let mut stream = crate::sql::query(session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema(R::NAME))?;
            for start in (0..ids.len()).step_by(32) {
                let end = (start + 32).min(ids.len());
                let _keys = application.budget.reserve(
                    "model-root-window",
                    (end - start) * size_of::<ProductionScope>(),
                )?;
                let mut kinds = Vec::with_capacity(end - start);
                for i in start..end {
                    let id: Id<R> =
                        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                            _,
                            serde::de::value::Error,
                        >::new(
                            ids.value(i).iter().copied()
                        ))
                        .map_err(ModelError::codec)?;
                    kinds.push(kind(id));
                }
                let window = application
                    .scopes
                    .window(
                        application.access,
                        &kinds,
                        application.frame,
                        application.budget,
                        application.cancellation,
                    )
                    .await?;
                for (partition, kind) in kinds.iter().enumerate() {
                    let selected =
                        window.data(partition, application.budget, application.cancellation)?;
                    let produced = apply_data(application, &selected, *kind)?;
                    merge_run(records, &produced)?;
                    publish_records(application.output, &produced).await?;
                }
            }
        }
        Ok(())
    })
}

type RecordEmitter =
    for<'a> fn(&'a ModelRecords, &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>>;
fn publish_records<'a>(
    output: &'a ProducerOutput,
    records: &'a ModelRecords,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! write {($($field:ident,)*)=>{{
        const EMITTERS: &[RecordEmitter] = &[$(|records, output| producer_operations::emit(&records.$field, output),)*];
        for emit in EMITTERS { emit(records, output).await?; }
    }};}
        write!(
            closed_targets,
            protocol_actions,
            terminal_assessments,
            terminal_frontiers,
            normal_restrictions,
            exit_characterizations,
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
        Ok(())
    })
}

fn merge_run(total: &mut ModelRecords, part: &ModelRecords) -> Result<(), ModelError> {
    total.run.applied = total
        .run
        .applied
        .checked_add(part.run.applied)
        .ok_or_else(|| ModelError::Invalid("Model applied count overflow".into()))?;
    total.run.refused = total
        .run
        .refused
        .checked_add(part.run.refused)
        .ok_or_else(|| ModelError::Invalid("Model refused count overflow".into()))?;
    if part.outcome.status == analysis::AnalysisStatus::Partial {
        total.outcome.status = part.outcome.status;
        total.outcome.reason = part.outcome.reason;
    }
    Ok(())
}

#[cfg(test)]
mod selected_catalog_controls {
    use super::*;
    #[tokio::test]
    async fn input_phase_is_lazy_and_missing_metadata_releases_its_charge() {
        let runtime = Workspace::new(
            Arc::new(model().unwrap()),
            crate::workspace::WorkspaceOptions {
                memory_bytes: 128 << 20,
                partitions: 1,
                batch_rows: 16,
            },
            crate::test_native::store(),
        )
        .unwrap();
        let access = runtime
            .inputs("model-phase-laziness", Profile::Catalog, [])
            .unwrap();
        let session = access.session(&runtime).await.unwrap();
        let budget = resources::ResourceBudget::fixed(4 << 20).unwrap();
        let sources =
            CapturedSources::capture(access.profile(), access.snapshots(), &budget).unwrap();
        let mut admission = CoverageAdmission::new(&sources, &budget).unwrap();
        let catalog = models::Catalog::parse(
            "external.toml",
            include_str!("../../lctx-model/models/external.toml"),
        )
        .unwrap();
        let (_, definition) = execution::configuration::models(catalog.declaration().id());
        let before = budget.reserved();
        let operation = load_inputs(&access, &session, &definition, &mut admission, &budget);
        assert_eq!(
            budget.reserved(),
            before,
            "constructing an unpolled phase must not reserve input state"
        );
        drop(operation);
        assert_eq!(budget.reserved(), before);
        let error = match load_inputs(&access, &session, &definition, &mut admission, &budget).await
        {
            Ok(_) => panic!("missing metadata must fail before catalog or expected-domain work"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains(analysis::MethodParameters::NAME),
            "first metadata failure: {error}"
        );
        assert_eq!(
            budget.reserved(),
            before,
            "failed input phases release finite configuration and declaration charges"
        );
    }
    #[tokio::test]
    async fn captured_catalog_is_selected_before_rich_decode() {
        let runtime = Workspace::new(
            Arc::new(model().unwrap()),
            crate::workspace::WorkspaceOptions {
                memory_bytes: 128 << 20,
                partitions: 1,
                batch_rows: 16,
            },
            crate::test_native::store(),
        )
        .unwrap();
        let parsed = models::Catalog::parse(
            "external.toml",
            include_str!("../../lctx-model/models/external.toml"),
        )
        .unwrap();
        let catalog = parsed.declaration();
        let huge_source = format!("{}\n#{}", catalog.source, "x".repeat(6 << 20));
        let unrelated = models::ModelCatalog {
            source_name: "unrelated.toml".into(),
            source: huge_source.clone(),
            content: ContentHash::of(huge_source.as_bytes()),
            ..catalog.clone()
        };
        let access = runtime
            .inputs("catalog-config", Profile::Behavioral, [])
            .unwrap();
        let output = runtime.output(
            "catalog-config",
            Profile::Behavioral,
            ContentHash::of(b"catalog-selection-control"),
            access,
            [<models::ModelCatalog>::NAME],
        );
        output.declare::<models::ModelCatalog>().unwrap();
        output.push(catalog.clone()).await.unwrap();
        output.push(unrelated).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let access = runtime
            .inputs(
                "catalog-consumer",
                Profile::Behavioral,
                [models::ModelCatalog::NAME],
            )
            .unwrap();
        let session = access.session(&runtime).await.unwrap();
        let budget = resources::ResourceBudget::fixed(256 << 10).unwrap();
        let declaration = ValidationInput::of::<models::ModelCatalog>(&["id"]);
        let mut consumed =
            crate::consumed_rows::ConsumedInputs::new(vec![declaration.clone()], &budget).unwrap();
        let mut data = ModelData::new(&budget);
        load_selected_catalog(&access, &session, &mut consumed, &mut data, catalog.id())
            .await
            .unwrap();
        consumed.finish("catalog-consumer").unwrap();
        assert_eq!(data.catalogs.len(), 1);
        assert_eq!(data.catalogs.get(catalog.id()), Some(catalog));
        drop(data);
        assert_eq!(budget.reserved(), 0);
        let permit = access.read::<models::ModelCatalog>().unwrap();
        let mut whole = Rows::<models::ModelCatalog>::new(&budget);
        assert!(
            crate::consumed_rows::stream_at(
                &permit,
                &declaration,
                &access,
                &session,
                |_, batch| whole.decode(batch)
            )
            .await
            .is_err(),
            "unused rich catalog would exceed the selected consumer budget"
        );
        drop(whole);
        assert_eq!(budget.reserved(), 0);
    }
}
