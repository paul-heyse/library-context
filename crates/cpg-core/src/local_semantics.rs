//! Local analysis uses confirmed inputs, one attempt budget and the domain's shared replay.
use crate::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::AttemptRuntime,
};
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, local as publication, sources::CapturedSources},
    local_semantics::{self, LocalData},
    obligation::ObligationKind,
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;
pub async fn run(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<(), ModelError> {
    local_semantics::check_definition(definition)?;
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
    let mut consumed =
        crate::consumed_rows::ConsumedInputs::new(LocalData::consumed_inputs(profile), budget)?;
    let mut data = LocalData::new(budget);
    let mut inputs = normalized::Rows::<input::InputRevision>::new(budget);
    let mut definitions = normalized::Rows::<analysis::AnalysisDefinition>::new(budget);
    macro_rules! load {($($field:ident:$ty:ty,)*)=>{$(while let Some((_,permit))=consumed.next::<$ty>(&access)?{
        let session=runtime.session(&access);
        crate::consumed_rows::stream(&permit,&reader,&session,|permit,batch|{
            admission.visit_if_expected(permit,batch)?;
            data.visit_consumed(profile,<$ty>::NAME,batch)?;
            if <$ty>::NAME==input::InputRevision::NAME{inputs.decode(batch)?;}
            if <$ty>::NAME==analysis::AnalysisDefinition::NAME{definitions.decode(batch)?;}
            Ok(())
        }).await?;
    })*};}
    lctx_model::entry_value_inputs!(load);
    lctx_model::local_semantic_inputs!(load);
    lctx_model::local_theory_inputs!(load);
    lctx_model::local_field_inputs!(load);
    lctx_model::expected_domain_inputs!(load);
    load! {definitions:analysis::AnalysisDefinition,}
    consumed.finish()?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "Local selected definition is absent from confirmed configuration".into(),
        ));
    }
    drop(definitions);
    reader.close().await.map_err(ModelError::codec)?;
    let mut output = StageOutput::new(access, attempt, model, budget.clone(), Default::default())?;
    macro_rules! declare_publication {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    declare_publication!(
        publication::AnalysisInvocation,
        publication::AnalysisInput,
        publication::SourceReceipt,
        publication::ProjectionInput,
        publication::AnalysisOutcome,
        publication::AnalysisDiagnostic,
        publication::InvocationSource,
        publication::ObligationSource,
        publication::AnalysisObligation,
        publication::DischargeEvidence,
        publication::AnalysisCoverage,
        publication::CoverageRequirement,
        publication::CoverageRequiredSource,
        publication::AnalysisCoveragePremise,
        publication::CoverageSource
    );
    macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;)*};}
    lctx_model::local_semantic_outputs!(declare);
    lctx_model::local_theory_outputs!(declare);
    lctx_model::local_field_outputs!(declare);
    let mut frames = charged::ChargedSet::default();
    let mut frame_charge = charged::StateCharge::new(budget, "local_invocation_frames");
    for run in data.entry.runs.iter().filter(|run| {
        profile == Profile::Catalog
            || data
                .entry
                .providers
                .get(run.provider)
                .is_some_and(|p| p.tool == "ty")
    }) {
        if !frames.insert(&mut frame_charge, (run.input, run.context))? {
            continue;
        }
        if inputs.get(run.input).is_none() {
            return Err(ModelError::Invalid("Local flow input absent".into()));
        }
        let (invocation, parents, receipts, projections) =
            publication::AnalysisInvocation::admitted(
                run.input,
                run.context,
                definition.id(),
                None,
                [],
                &sources,
                [],
                budget,
            )?;
        let coverage = publication::coverage::admit(
            &invocation,
            definition,
            analysis::AnalysisCapability::Transfers,
            &admission,
            budget,
        )?;
        let rows = if profile == Profile::Behavioral {
            local_semantics::produce(&data, &invocation, definition, budget)?
        } else {
            local_semantics::LocalRecords::new(budget)
        };
        macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter(){output.push(row.clone()).await?;})*};}
        lctx_model::local_semantic_outputs!(write);
        macro_rules! theory_write{($($field:ident:$ty:ty,)*)=>{$(for row in rows.theory.$field.iter(){output.push(row.clone()).await?;})*};}
        lctx_model::local_theory_outputs!(theory_write);
        macro_rules! fields_write{($($field:ident:$ty:ty,)*)=>{$(for row in rows.fields.$field.iter(){output.push(row.clone()).await?;})*};}
        lctx_model::local_field_outputs!(fields_write);
        let outcome = publication::AnalysisOutcome {
            invocation: invocation.id(),
            status: if profile == Profile::Behavioral {
                analysis::AnalysisStatus::Partial
            } else {
                analysis::AnalysisStatus::NotRequested
            },
            reason: Some(if profile == Profile::Behavioral {
                ObligationKind::IncompleteDomain
            } else {
                ObligationKind::NotRequested
            }),
        };
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
        output.push(invocation).await?;
        for row in parents {
            output.push(row).await?;
        }
        for row in receipts {
            output.push(row).await?;
        }
        for row in projections {
            output.push(row).await?;
        }
        output.push(outcome).await?;
    }
    output.finish(ProviderOutcome::Complete).await
}
