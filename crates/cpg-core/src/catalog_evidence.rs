//! C1 contextual evidence consumes completed C0 and typed original facts, independently of brief seeds.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use datafusion::execution::context::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::stages::ProviderOutcome;
use lctx_model::domain::{
    analysis::{self, catalog_evidence::*},
    catalog::evidence::{
        self,
        build::{self, EvidenceData},
    },
    normalized::Rows,
    *,
};
use std::sync::Arc;
// The semantic owner declares exact views; these macros provide typed decoders.
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::catalog_inputs!($apply);
        lctx_model::catalog_outputs!($apply);
        lctx_model::catalog_evidence_inputs!($apply);
        lctx_model::catalog_runtime_inputs!($apply);
        $apply! {definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,}
        lctx_model::expected_domain_inputs!($apply);
    };
}
fn consumed_inputs(_profile: stages::Profile) -> Vec<ValidationInput> {
    let mut declarations = analysis::expected::inputs(build::definition().1.method);
    declarations.extend([
        ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"]),
        ValidationInput::of::<analysis::local::Invocation>(&["id"]),
        ValidationInput::of::<analysis::local::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<analysis::source_call::Invocation>(&["id"]),
        ValidationInput::of::<analysis::source_call::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ValidationInput::of::<analysis::MethodParameters>(&["id"]),
    ]);
    declarations
}
fn is_frame(name: &str) -> bool {
    [
        attribution::ProviderRun::NAME,
        analysis::catalog_core::Invocation::NAME,
        analysis::local::Invocation::NAME,
        analysis::local::AnalysisOutcome::NAME,
        analysis::source_call::Invocation::NAME,
        analysis::source_call::AnalysisOutcome::NAME,
    ]
    .contains(&name)
}
async fn load<R: Record>(
    access: &CompletedInputs,
    session: &SessionContext,
    consumed: &mut crate::consumed_rows::ConsumedInputs,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
    mut visit: impl FnMut(&ValidationInput, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    while let Some((input, permit)) = consumed.next::<R>(access)? {
        if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
            .await?
        {
            continue;
        }
        crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
            admission.visit_if_expected(permit, batch)?;
            visit(&input, batch)
        })
        .await?;
    }
    Ok(())
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let session = access.session(runtime).await?;
    let mut frames = EvidenceData::new(runtime.budget());
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters = Rows::<analysis::MethodParameters>::new(runtime.budget());
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        consumed_inputs(access.profile()),
        runtime.budget(),
    )?;
    macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(load::<$ty>(&access,&session,&mut consumed,&mut admission,|input,batch|{
        if is_frame(input.name()) { frames.visit_input(input,batch)?; }
        if input.name()==analysis::AnalysisDefinition::NAME {definitions.decode(batch)?;}
        if input.name()==analysis::MethodParameters::NAME {parameters.decode(batch)?;}
        Ok(())
    }).await?;)*};}
    decoder_inputs!(inventory);
    consumed.finish(access.name())?;
    let (expected_parameters, definition) = build::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "C1 requires its completed authored definition".into(),
        ));
    }
    macro_rules! declare_outputs {($($f:ident:$ty:ty,)*)=>{$(output.declare_async::<$ty>().await?;)*};}
    lctx_model::catalog_evidence_outputs!(declare_outputs);
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare_async::<$ty>().await?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare_async::<analysis::catalog_evidence::$record>().await?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(evidence::EvidenceInvocation);
    let mut invocations = Rows::new(runtime.budget());
    let expected_parents = evidence::frames::parents(
        &frames.facts.runs,
        &frames.facts.core_invocations,
        runtime.budget(),
    )?;
    for parent in expected_parents.iter() {
        let parent_sources = evidence::frames::sources(parent, &frames.runtime.lower())?;
        let (invocation, parents, receipts, projections) = Invocation::admitted(
            parent.input,
            parent.context,
            definition.id(),
            None,
            parent_sources.iter().map(Record::id),
            &sources,
            [],
            runtime.budget(),
        )?;
        if !projections.is_empty() {
            return Err(ModelError::Invalid(
                "C1 has no projection requirement".into(),
            ));
        }
        for source in parent_sources {
            output.push(source).await?;
        }
        for row in parents {
            output.push(row).await?;
        }
        for receipt in receipts {
            output.push(receipt).await?;
        }
        let admitted = coverage::admit(
            &invocation,
            &definition,
            analysis::AnalysisCapability::CatalogEvidence,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            let (requirement, members) = scope.expectation().records()?;
            output.push(requirement).await?;
            for member in members {
                output.push(member).await?;
            }
            for observed in scope.observations() {
                output.push(observed.source().clone()).await?;
            }
            let (coverage, members) = coverage::assess(
                scope.expectation(),
                scope.observations(),
                analysis::AnalysisStatus::Completed,
                None,
                runtime.budget(),
            )?;
            output.push(coverage).await?;
            for member in members {
                output.push(member).await?;
            }
        }
        output
            .push(AnalysisOutcome {
                invocation: invocation.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            })
            .await?;
        output.push(invocation.clone()).await?;
        invocations.insert(invocation)?;
    }
    // The native frame domain remains total, including inputs without any evidence roots.
    // Rich premises and outputs have only one original artifact's lifetime.
    drop(expected_parents);
    drop(frames);
    drop(definitions);
    drop(parameters);
    let scopes = crate::catalog_evidence_scope::EvidenceScopes::prepare(
        &access,
        model,
        &session,
        runtime.budget(),
    )
    .await?;
    let artifacts = access.table_for(&ValidationInput::of::<source::SourceArtifact>(&["id"]))?;
    let mut roots = crate::sql::query(
        &session,
        &format!(
            "SELECT id FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&artifacts)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        let ids = batch
            .column_by_name("id")
            .and_then(|column| {
                column
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("C1 artifact root identity"))?;
        for index in 0..batch.num_rows() {
            runtime.cancellation().check()?;
            let predicate = format!(
                "id=X'{}'",
                ids.value(index)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            let scope = scopes
                .edges
                .grain(scopes.root, &predicate, runtime.budget())
                .await?;
            let mut data = EvidenceData::new(runtime.budget());
            let mut consumed =
                crate::consumed_rows::ConsumedInputs::new(scopes.inputs.clone(), runtime.budget())?;
            macro_rules! scoped_inputs {($($field:ident:$ty:ty,)*)=>{$(
                while let Some((input, permit)) = consumed.next::<$ty>(&access)? {
                    let table = scopes.inputs.iter().position(|candidate| candidate.type_id()==input.type_id() && candidate.prefix()==input.prefix())
                        .ok_or(ModelError::Conflict("C1 scoped input declaration"))?;
                    crate::consumed_rows::stream_query_at(&permit,&input,scope.session(),&scope.select(table)?,|_,batch| {
                        data.visit_input(&input,batch)?; Ok(())
                    }).await?;
                }
            )*};}
            decoder_inputs!(scoped_inputs);
            consumed.finish(access.name())?;
            drop(scope);
            let budget = runtime.budget().clone();
            let rows = tokio::task::spawn_blocking(move || build::build(&data, &budget))
                .await
                .map_err(ModelError::codec)??;
            macro_rules! write {($($f:ident:$ty:ty,)*)=>{$(for row in rows.$f.iter() {output.push(row.clone()).await?;})*};}
            lctx_model::catalog_evidence_outputs!(write);
            let links = build::invocation_links(&rows, &invocations, runtime.budget())?;
            for row in links.iter() {
                output.push(row.clone()).await?;
            }
            drop(links);
            drop(rows);
        }
    }
    drop(roots);
    drop(scopes);
    drop(session);
    drop(invocations);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_views_have_profile_decoder_reachability() {
        for profile in [stages::Profile::Catalog, stages::Profile::Behavioral] {
            let mut decoders = std::collections::BTreeSet::new();
            macro_rules! inventory {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
            decoder_inputs!(inventory);
            crate::consumed_rows::assert_decoder_reachability(consumed_inputs(profile), &decoders);
        }
    }
}
