//! Local analysis uses confirmed inputs, one attempt budget and the domain's shared replay.
use crate::producer_operations::{Declaration, declare, declare_ordered, emit};
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use arrow_array::Array;
use futures::{TryStreamExt, future::BoxFuture};
use lctx_model::domain::{
    analysis::{self, expected::CoverageAdmission, local as publication, sources::CapturedSources},
    local_semantics::{self, LocalData},
    obligation::ObligationKind,
    stages::*,
    *,
};
use std::sync::Arc;
mod scope;
/// Actual Local receipts retain exact source and output descriptors across downstream borrows.
pub struct PreparedLocal {
    premises: CompletedInputs,
    entries: CompletedInputs,
    guards: CompletedInputs,
    values: local_semantics::ProducedLocal,
}
impl PreparedLocal {
    pub fn entries<'a>(
        &'a self,
        access: &CompletedInputs,
        runtime: &Workspace,
    ) -> Result<&'a local_semantics::ProducedLocal, ModelError> {
        self.premises.require_subset(runtime, access)?;
        self.entries.require_subset(runtime, access)?;
        Ok(&self.values)
    }
    pub fn guards<'a>(
        &'a self,
        access: &CompletedInputs,
        runtime: &Workspace,
    ) -> Result<&'a local_semantics::ProducedLocal, ModelError> {
        self.guards.require_subset(runtime, access)?;
        self.entries(access, runtime)
    }
}
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
type MetadataLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut CoverageAdmission<'sources>,
    &'a mut LocalData,
    &'a mut normalized::Rows<input::InputRevision>,
    &'a mut normalized::Rows<analysis::AnalysisDefinition>,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn read_metadata<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut CoverageAdmission<'_>,
    data: &'a mut LocalData,
    inputs: &'a mut normalized::Rows<input::InputRevision>,
    definitions: &'a mut normalized::Rows<analysis::AnalysisDefinition>,
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
                if R::NAME == attribution::ProviderRun::NAME
                    || R::NAME == attribution::Provider::NAME
                {
                    data.entry.visit(R::NAME, batch)?;
                }
                if R::NAME == input::InputRevision::NAME {
                    inputs.decode(batch)?;
                }
                if R::NAME == analysis::AnalysisDefinition::NAME {
                    definitions.decode(batch)?;
                }
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
    admission: &'a mut CoverageAdmission<'_>,
    data: &'a mut LocalData,
    inputs: &'a mut normalized::Rows<input::InputRevision>,
    definitions: &'a mut normalized::Rows<analysis::AnalysisDefinition>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    let mut loaders: Vec<MetadataLoader> = Vec::new();
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {$(loaders.push(read_metadata::<$ty>);)*};}
    decoder_inputs!(adapters);
    Box::pin(async move {
        for load in loaders {
            load(
                access,
                session,
                consumed,
                admission,
                data,
                inputs,
                definitions,
            )
            .await?;
        }
        Ok(())
    })
}
fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        let mut declarations: Vec<Declaration> = Vec::new();
        macro_rules! common {($($record:ident,)*) => {$(declarations.push(declare::<publication::$record>);)*};}
        lctx_model::analysis_publication!(common);
        declarations.extend([
            declare::<publication::AnalysisDiagnostic> as Declaration,
            declare::<publication::ObligationSource>,
            declare::<publication::AnalysisObligation>,
            declare::<publication::DischargeEvidence>,
        ]);
        macro_rules! outputs {($($field:ident:$ty:ty,)*) => {$(declarations.push(declare::<$ty>);)*};}
        lctx_model::local_semantic_outputs!(outputs);
        lctx_model::local_theory_outputs!(outputs);
        lctx_model::local_field_outputs!(outputs);
        declare_ordered(output, &declarations).await
    })
}

pub async fn run(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    definition: &analysis::AnalysisDefinition,
) -> Result<Option<PreparedLocal>, ModelError> {
    local_semantics::check_definition(definition)?;
    let profile = access.profile();
    let budget = runtime.budget();
    let sources = CapturedSources::capture(access.profile(), access.snapshots(), budget)?;
    let mut admission = CoverageAdmission::new(&sources, budget)?;
    let session = access.session(runtime).await?;
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {
            let mut inputs = vec![
                ValidationInput::of::<attribution::ProviderRun>(&["id"]),
                ValidationInput::of::<attribution::Provider>(&["id"]),
                ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ];
            inputs.extend(analysis::expected::inputs(definition.method));
            inputs
        },
        budget,
    )?;
    let mut data = LocalData::new(budget);
    let mut inputs = normalized::Rows::<input::InputRevision>::new(budget);
    let mut definitions = normalized::Rows::<analysis::AnalysisDefinition>::new(budget);
    load_metadata(
        &access,
        &session,
        &mut consumed,
        &mut admission,
        &mut data,
        &mut inputs,
        &mut definitions,
    )
    .await?;
    consumed.finish(access.name())?;
    if definitions.get(definition.id()) != Some(definition) {
        return Err(ModelError::Invalid(
            "Local selected definition is absent from confirmed configuration".into(),
        ));
    }
    drop(definitions);
    let scopes = if profile == Profile::Behavioral {
        Some(scope::LocalScopes::prepare(&access, &session, _model, budget).await?)
    } else {
        None
    };
    declare_outputs(&output).await?;
    let mut actual = local_semantics::ProducedLocal::empty(budget);
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
        if let Some(scopes) = &scopes {
            let mut composition = local_semantics::composition::Composition::new(budget);
            let mut roots =
                crate::sql::query(&session, &scopes.root_sql(&access, run.input, run.context)?)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
            while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                let ids = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                    .ok_or(ModelError::Schema(source::SourceArtifact::NAME))?;
                for i in 0..ids.len() {
                    let source: Id<source::SourceArtifact> =
                        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                            _,
                            serde::de::value::Error,
                        >::new(
                            ids.value(i).iter().copied()
                        ))
                        .map_err(ModelError::codec)?;
                    let grain = scopes.source(source, run.context, budget).await?;
                    let selected = scopes.load(&access, &grain, budget).await?;
                    let rows = local_semantics::produce_source(
                        &selected,
                        &invocation,
                        definition,
                        source,
                        budget,
                    )?;
                    composition.observe(&selected, &rows, budget)?;
                    publish_records(&output, &rows).await?;
                    actual.append(rows.actual)?;
                }
            }
            for selection in composition.selections() {
                output.push(selection).await?;
            }
        }
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
    output.finish(ProviderOutcome::Complete).await?;
    if profile == Profile::Catalog {
        return Ok(None);
    }
    let premises = access.select(&conditions::entry::EntryData::facts_inputs())?;
    let entries = runtime.inputs(
        "actual-local-entry-outputs",
        profile,
        [
            conditions::entry::EntryValueWitness::NAME,
            conditions::entry::EntryAccessSource::NAME,
        ],
    )?;
    let guards = runtime.inputs(
        "actual-local-guard-outputs",
        profile,
        [conditions::stability::StabilityWitness::NAME],
    )?;
    Ok(Some(PreparedLocal {
        premises,
        entries,
        guards,
        values: actual,
    }))
}

fn publish_records<'a>(
    output: &'a ProducerOutput,
    rows: &'a local_semantics::LocalRecords,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a ProducerOutput,
        &'a local_semantics::LocalRecords,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    let semantic = {
        macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
            $(fn $field<'a>(output: &'a ProducerOutput, rows: &'a local_semantics::LocalRecords) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
            const EMISSIONS: &[Emission] = &[$($field,)*];
        };}
        lctx_model::local_semantic_outputs!(adapters);
        EMISSIONS
    };
    let theory = {
        macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
            $(fn $field<'a>(output: &'a ProducerOutput, rows: &'a local_semantics::LocalRecords) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.theory.$field, output) })*
            const EMISSIONS: &[Emission] = &[$($field,)*];
        };}
        lctx_model::local_theory_outputs!(adapters);
        EMISSIONS
    };
    let fields = {
        macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
            $(fn $field<'a>(output: &'a ProducerOutput, rows: &'a local_semantics::LocalRecords) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.fields.$field, output) })*
            const EMISSIONS: &[Emission] = &[$($field,)*];
        };}
        lctx_model::local_field_outputs!(adapters);
        EMISSIONS
    };
    Box::pin(async move {
        for emission in semantic.iter().chain(theory).chain(fields) {
            emission(output, rows).await?;
        }
        Ok(())
    })
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
