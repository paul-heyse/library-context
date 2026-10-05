//! Local analysis uses confirmed inputs, one attempt budget and the domain's shared replay.
use crate::{
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, local as publication, sources::CapturedSources},
    local_semantics::{self, LocalData},
    obligation::ObligationKind,
    stages::*,
    *,
};
use std::sync::Arc;
// Decoder reachability is separate from the model-owned consumed source inventory.
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::entry_value_inputs!($apply);
        lctx_model::local_semantic_inputs!($apply);
        lctx_model::local_theory_inputs!($apply);
        lctx_model::local_field_inputs!($apply);
        lctx_model::expected_domain_inputs!($apply);
        $apply! {definitions:analysis::AnalysisDefinition,}
    };
}
pub async fn run(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<(), ModelError> {
    local_semantics::check_definition(definition)?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut consumed =
        crate::consumed_rows::ConsumedInputs::new(LocalData::consumed_inputs(profile), budget)?;
    let mut data = LocalData::new(budget);
    let mut inputs = normalized::Rows::<input::InputRevision>::new(budget);
    let mut definitions = normalized::Rows::<analysis::AnalysisDefinition>::new(budget);
    macro_rules! load {($($field:ident:$ty:ty,)*)=>{$(while let Some((input,permit))=consumed.next::<$ty>(&access)?{
        crate::consumed_rows::stream_at(&permit,&input,&access,&session,|permit,batch|{
            admission.visit_if_expected(permit,batch)?;
            data.visit_consumed(profile,<$ty>::NAME,batch)?;
            if <$ty>::NAME==input::InputRevision::NAME{inputs.decode(batch)?;}
            if <$ty>::NAME==analysis::AnalysisDefinition::NAME{definitions.decode(batch)?;}
            Ok(())
        }).await?;
    })*};}
    decoder_inputs!(load);
    consumed.finish(access.name())?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "Local selected definition is absent from confirmed configuration".into(),
        ));
    }
    drop(definitions);
    macro_rules! declare_publication {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<publication::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare_publication!(
        publication::AnalysisDiagnostic,
        publication::ObligationSource,
        publication::AnalysisObligation,
        publication::DischargeEvidence
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
        let (status, reason) = if profile == Profile::Catalog {
            (
                analysis::AnalysisStatus::NotRequested,
                Some(ObligationKind::NotRequested),
            )
        } else if coverage
            .scopes()
            .iter()
            .all(|scope| scope.expectation().no_scope)
        {
            // The admitted input domain, rather than absence of produced rows, proves emptiness.
            (analysis::AnalysisStatus::Completed, None)
        } else {
            (
                analysis::AnalysisStatus::Partial,
                Some(ObligationKind::IncompleteDomain),
            )
        };
        let outcome = publication::AnalysisOutcome {
            invocation: invocation.id(),
            status,
            reason,
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
                LocalData::consumed_inputs(profile),
                &decoders,
            );
        }
    }
}
