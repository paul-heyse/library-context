//! Model-owned analysis over completed native/normalized/Enriched inputs.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, model as owner, sources::CapturedSources},
    execution::{model_production::*, model_rules::*},
    normalized::Rows,
    stages::*,
    *,
};
use std::sync::Arc;
use futures::TryStreamExt;
use arrow_array::Array;
mod scope;
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
pub async fn apply(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    bindings:Option<&crate::analysis_bindings::PreparedBindings>,
    evaluations:Option<&crate::semantic_execution::Produced<execution::production::ProducedEvaluations>>,
    local:Option<&crate::local_semantics::PreparedLocal>,
) -> Result<(), ModelError> {
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut data = ModelData::new(budget);
    // The only resident global domain is finite configuration plus compact publication frames.
    let mut declarations=vec![ValidationInput::of::<models::ModelCatalog>(&["id"]),ValidationInput::of::<analysis::MethodParameters>(&["id"]),ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),ValidationInput::of::<analysis::enriched_execution::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::source_call::AnalysisInvocation>(&["id"]),ValidationInput::of::<analysis::local::AnalysisInvocation>(&["id"]),ValidationInput::of::<attribution::ProviderRun>(&["id"])];
    declarations.extend(analysis::expected::inputs(definition.method));
    let mut consumed=crate::consumed_rows::ConsumedInputs::new(declarations,budget)?;
    macro_rules! read{($($ty:ty),*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,_,batch|data.visit_input(input,batch)).await?;)*};}
    read!(models::ModelCatalog,analysis::MethodParameters,analysis::AnalysisDefinition,analysis::enriched_execution::AnalysisInvocation,analysis::source_call::AnalysisInvocation,analysis::local::AnalysisInvocation,attribution::ProviderRun);
    if data.definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "Model definition absent from confirmed configuration".into(),
        ));
    }
    macro_rules! expected{($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|_,_,_|Ok(())).await?;)*};}
    lctx_model::expected_domain_inputs!(expected);
    consumed.finish(access.name())?;
    let selected=data.parameters.get(definition.parameters).and_then(|row|row.model_catalog).ok_or_else(||ModelError::Invalid("Models selected catalog absent".into()))?;
    let parsed=SelectedCatalog::read(data.catalogs.get(selected).ok_or(ModelError::Schema(models::ModelCatalog::NAME))?,budget)?;
    let actual=if profile==Profile::Behavioral{Some(ActualInputs{
        evaluations:evaluations.ok_or_else(||ModelError::Invalid("Models actual Base values absent".into()))?.borrow(&access,runtime)?,
        local:local.ok_or_else(||ModelError::Invalid("Models actual Local values absent".into()))?.entries(&access,runtime)?,
    })}else{None};
    let verified=if profile==Profile::Behavioral{Some(bindings.ok_or_else(||ModelError::Invalid("Models actual binding values absent".into()))?.application(&access,runtime)?)}else{None};
    let scopes=if profile==Profile::Behavioral{Some(scope::ModelScopes::prepare(&access,&session,_model,&parsed,budget).await?)}else{None};
    macro_rules! declare{($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<owner::$record>()?;)*};}
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
        let mut records=if let Some(scopes)=&scopes{let grain=scopes.frame(frame.id(),budget).await?;let selected=scopes.load(&access,&grain,budget).await?;apply_selected(&selected,&invocation,definition,profile,&parsed,verified,ProductionScope::Frame,actual.as_ref(),budget)?}else{apply_selected(&data,&invocation,definition,profile,&parsed,None,ProductionScope::Frame,None,budget)?};
        if let Some(scopes)=&scopes{
            for compiled in parsed.catalog().models(){
                let kind=ProductionScope::Target(compiled.declaration().id());
                let grain=scopes.selected(kind,frame.id(),budget).await?;let selected=scopes.load(&access,&grain,budget).await?;
                let produced=apply_selected(&selected,&invocation,definition,profile,&parsed,verified,kind,actual.as_ref(),budget)?;
                merge_run(&mut records,&produced)?;publish_records(&output,&produced).await?;
            }
            // Root coordinates are externally ordered before rich rows are decoded.
            let occurrences=crate::consumed_rows::identifier(&access.table_for(&ValidationInput::of::<source::Occurrence>(&["id"]))?);
            let artifacts=crate::consumed_rows::identifier(&access.table_for(&ValidationInput::of::<source::SourceArtifact>(&["id"]))?);
            let qualifications=crate::consumed_rows::identifier(&access.table_for(&ValidationInput::of::<assertion::AssertionQualification>(&["id"]).at_epoch(PublicationBoundary::Facts))?);
            macro_rules! roots{($ty:ty,$predicate:expr,$kind:expr)=>{{
                let alias=crate::consumed_rows::identifier(&access.table_for(&ValidationInput::of::<$ty>(&["id"]))?);
                let sql=format!("SELECT r.id FROM {alias} r {} ORDER BY r.id",$predicate);
                let mut stream=crate::sql::query(&session,&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{
                    let ids=batch.column(0).as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>().ok_or(ModelError::Schema(<$ty>::NAME))?;
                    for i in 0..ids.len(){let id:Id<$ty>=serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new(ids.value(i).iter().copied())).map_err(ModelError::codec)?;let kind=($kind)(id);
                        let grain=scopes.selected(kind,frame.id(),budget).await?;let selected=scopes.load(&access,&grain,budget).await?;
                        let produced=apply_selected(&selected,&invocation,definition,profile,&parsed,verified,kind,actual.as_ref(),budget)?;
                        merge_run(&mut records,&produced)?;publish_records(&output,&produced).await?;
                    }
                }
            }};}
            let parent=data.enriched.iter().find(|p|(p.input,p.context)==(frame.input,frame.context)&&p.subject.is_none()).ok_or(ModelError::Schema(analysis::enriched_execution::AnalysisInvocation::NAME))?;
            roots!(execution::context_execution::ContextExecution,format!("WHERE r.invocation={}",scope::hex(parent.id())),ProductionScope::Context);
            roots!(normalized::events::NormalizedCallEvent,format!("JOIN {occurrences} o ON o.id=r.site JOIN {artifacts} a ON a.id=o.source WHERE a.input={} AND r.context={}",scope::hex(frame.input),scope::hex(frame.context)),ProductionScope::Event);
            let events=crate::consumed_rows::identifier(&access.table_for(&ValidationInput::of::<normalized::events::NormalizedCallEvent>(&["id"]))?);
            roots!(protocols::NativeTerminalObservation,format!("JOIN {qualifications} q ON q.id=r.qualification JOIN {occurrences} o ON o.id=r.subject JOIN {artifacts} a ON a.id=o.source WHERE a.input={} AND q.context={} AND NOT EXISTS (SELECT 1 FROM {events} e WHERE e.site=r.subject AND e.context=q.context AND e.origin={})",scope::hex(frame.input),scope::hex(frame.context),scope::hex(calls::CallOrigin::explicit())),ProductionScope::Terminal);
            roots!(protocols::NativeExitObservation,format!("JOIN {qualifications} q ON q.id=r.qualification JOIN {occurrences} o ON o.id=r.subject JOIN {artifacts} a ON a.id=o.source WHERE a.input={} AND q.context={}",scope::hex(frame.input),scope::hex(frame.context)),ProductionScope::Exit);
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
    output.finish(ProviderOutcome::Complete).await
}

async fn publish_records(output:&ProducerOutput,records:&ModelRecords)->Result<(),ModelError>{
    macro_rules! write{($($field:ident,)*)=>{$(for row in records.$field.iter(){output.push(row.clone()).await?;})*};}
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
}

fn merge_run(total:&mut ModelRecords,part:&ModelRecords)->Result<(),ModelError>{
 total.run.applied=total.run.applied.checked_add(part.run.applied).ok_or_else(||ModelError::Invalid("Model applied count overflow".into()))?;
 total.run.refused=total.run.refused.checked_add(part.run.refused).ok_or_else(||ModelError::Invalid("Model refused count overflow".into()))?;
 if part.outcome.status==analysis::AnalysisStatus::Partial{total.outcome.status=part.outcome.status;total.outcome.reason=part.outcome.reason;}
 Ok(())
}
