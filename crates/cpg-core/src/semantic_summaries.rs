//! One nominal Summary publication over immutable predecessors and borrowed stored topology.
use crate::{
    analysis_graphs::PreparedGraphs,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, sources::CapturedSources, summary as owner},
    execution::{summary_production::*, summary_replay},
    stages::*,
    *,
};
use std::sync::Arc;
mod scope;
// Decoder reachability is separate from the model-owned consumed source inventory.
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::normalized_binding_inputs!($apply);
        lctx_model::normalized_binding_outputs!($apply);
        lctx_model::entry_value_inputs!($apply);
        lctx_model::summary_path_inputs!($apply);
        lctx_model::summary_owned_inputs!($apply);
        lctx_model::summary_vocabulary!($apply);
        lctx_model::summary_evidence_inputs!($apply);
        lctx_model::expected_domain_inputs!($apply);
    };
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
    graphs: Option<&PreparedGraphs>,
    bindings: Option<&crate::analysis_bindings::PreparedBindings>,
    local: Option<&crate::local_semantics::PreparedLocal>,
) -> Result<(), ModelError> {
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
    let actual = if access.profile() == Profile::Behavioral {
        Some(
            local
                .ok_or_else(|| ModelError::Invalid("Summary actual Local owner absent".into()))?
                .guards(&access, runtime)?,
        )
    } else {
        None
    };
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut coverage = CoverageAdmission::new(&sources, budget)?;
    let mut data = SummaryData::new(budget);
    let session = access.session(runtime).await?;
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {
            let mut inputs = summary_replay::production_inputs(Profile::Catalog);
            inputs.extend(analysis::expected::inputs(
                analysis::AnalysisMethod::Summaries,
            ));
            inputs
        },
        budget,
    )?;
    macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$(while let Some((input,permit))=consumed.next::<$ty>(&access)?{
        if crate::consumed_rows::stream_artifact_admission(&access,&input,&session,&mut coverage).await? {continue;}
        crate::consumed_rows::stream_at(&permit,&input,&access,&session,|permit,batch|{
            coverage.visit_if_expected(permit,batch)?;
            if [attribution::ProviderRun::NAME,analysis::MethodParameters::NAME,analysis::AnalysisDefinition::NAME,analysis::local::AnalysisInvocation::NAME,analysis::model::AnalysisInvocation::NAME,analysis::enriched_execution::AnalysisInvocation::NAME,analysis::source_call::AnalysisInvocation::NAME,analysis::local::AnalysisOutcome::NAME,analysis::model::AnalysisOutcome::NAME,analysis::enriched_execution::AnalysisOutcome::NAME,analysis::source_call::AnalysisOutcome::NAME].contains(&input.name()){data.visit_input(&input,batch)?;}
            Ok(())
        }).await?;
    })*};}
    decoder_inputs!(inputs);
    consumed.finish(access.name())?;
    let scopes = if access.profile() == Profile::Behavioral {
        Some(scope::SummaryScopes::prepare(&access, &session, _model, budget).await?)
    } else {
        None
    };
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<owner::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);

    macro_rules! declarations{($($f:ident:$t:ty,)*)=>{$(output.declare::<$t>()?;)*};}
    lctx_model::summary_outputs!(declarations);
    lctx_model::summary_vocabulary!(declarations);
    let mut frames = charged::ChargedSet::default();
    let mut frame_charge = charged::StateCharge::new(budget, "summary-output-frames");
    for run in data.entry.runs.iter() {
        if !frames.insert(&mut frame_charge, (run.input, run.context))? {
            continue;
        }
        let parents = summary_replay::parents(&data, run.input, run.context, budget)?;
        let key = projection::normalization::ProjectionKey {
            input: run.input,
            context: run.context,
            name: projection::ProjectionName::CallableInvocation,
        };
        let graph = if access.profile() == Profile::Behavioral {
            Some(
                graphs
                    .ok_or_else(|| ModelError::Invalid("requested Summary graph absent".into()))?
                    .graph(&access, runtime, key)?,
            )
        } else {
            None
        };
        let (invocation, inputs, receipts, projections) = owner::AnalysisInvocation::admitted(
            run.input,
            run.context,
            definition.id(),
            None,
            parents.iter().map(Record::id),
            &sources,
            graph.map(|_| {
                analysis::ProjectionDefinition::builtin(
                    projection::ProjectionName::CallableInvocation,
                )
                .id()
            }),
            budget,
        )?;
        let _frame = budget.reserve(
            "summary-output-frame",
            4096 + receipts
                .iter()
                .map(|row| size_of::<owner::SourceReceipt>() + row.heap_bytes())
                .sum::<usize>()
                + inputs.len() * size_of::<owner::AnalysisInput>()
                + projections.len() * size_of::<owner::ProjectionInput>(),
        )?;
        let coverage = owner::coverage::admit(
            &invocation,
            definition,
            analysis::AnalysisCapability::Summaries,
            &coverage,
            budget,
        )?;
        let grain = if let Some(scopes) = &scopes {
            Some(scopes.frame(run.id(), budget).await?)
        } else {
            None
        };
        let selected = if let (Some(scopes), Some(grain)) = (&scopes, &grain) {
            Some(scopes.load(&access, grain, budget).await?)
        } else {
            None
        };
        let frame_data = match access.profile() {
            Profile::Behavioral => selected
                .as_ref()
                .ok_or_else(|| ModelError::Invalid("Summary selected frame absent".into()))?,
            Profile::Catalog => &data,
        };
        let mut result = crate::stage_runtime::borrowed_cpu(access.name(), || {
            execution::summary_production::produce_prepared(
                frame_data,
                &invocation,
                definition,
                access.profile(),
                graph,
                budget,
                application,
                actual,
            )
        })?;
        drop(selected);
        drop(grain);
        tokio::task::yield_now().await;
        let mut actual_coverage = normalized::Rows::new(budget);
        let mut actual_premises = normalized::Rows::new(budget);
        for scope in coverage.scopes() {
            let (row, premises) = owner::coverage::assess(
                scope.expectation(),
                scope.observations(),
                result.outcome.status,
                result.outcome.reason,
                budget,
            )?;
            actual_coverage.insert(row)?;
            for row in premises {
                actual_premises.insert(row)?;
            }
        }
        result.discharge(&actual_coverage)?;
        macro_rules! write{($($f:ident:$t:ty,)*)=>{$(for row in result.$f.iter(){output.push(row.clone()).await?;})*};}
        lctx_model::summary_outputs!(write);
        macro_rules! vocabulary{($($f:ident:$t:ty,)*)=>{$(for row in result.vocabulary.$f.values(){output.push(row.clone()).await?;})*};}
        lctx_model::summary_vocabulary!(vocabulary);
        for scope in coverage.scopes() {
            let (requirement, required) = scope.expectation().records()?;
            output.push(requirement).await?;
            for row in required {
                output.push(row).await?;
            }
            for row in scope.observations() {
                output.push(row.source().clone()).await?;
            }
        }
        for row in actual_coverage.iter() {
            output.push(row.clone()).await?;
        }
        for row in actual_premises.iter() {
            output.push(row.clone()).await?;
        }
        output.push(result.outcome).await?;
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

#[cfg(test)]
mod decoder_tests {
    use super::*;
    #[test]
    fn declared_sources_have_decoder_reachability_in_both_profiles() {
        let mut decoders = std::collections::BTreeSet::new();
        macro_rules! collect {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
        decoder_inputs!(collect);
        lctx_model::summary_projection_inputs!(collect);
        decoders.insert(std::any::TypeId::of::<
            analysis::native::NativeAssertionPremise,
        >());
        for profile in Profile::ALL {
            crate::consumed_rows::assert_decoder_reachability(
                SummaryData::consumed_inputs(profile),
                &decoders,
            );
        }
    }
}
