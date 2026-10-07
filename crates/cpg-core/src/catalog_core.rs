//! C0 execution consumes completed native/normalized authorities. No flow or brief producer.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use datafusion::execution::context::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::stages::ProviderOutcome;
use lctx_model::domain::{
    analysis::{self, catalog_core::*},
    catalog::{
        self,
        build::{self, CatalogData},
    },
    normalized::Rows,
    *,
};
use std::sync::Arc;
async fn inventory<R: Record>(
    access: &CompletedInputs,
    session: &SessionContext,
    consumed: &mut crate::consumed_rows::ConsumedInputs,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
    mut visit: impl FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    while let Some((input, permit)) = consumed.next::<R>(access)? {
        if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
            .await?
        {
            continue;
        }
        crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
            admission.visit_if_expected(permit, batch)?;
            visit(batch)
        })
        .await?;
    }
    Ok(())
}
/// Internal stage entry; public catalog frontiers are assembled separately by F0.
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
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters = Rows::<analysis::MethodParameters>::new(runtime.budget());
    let mut runs = Rows::<attribution::ProviderRun>::new(runtime.budget());
    let mut declarations = CatalogData::validation_inputs();
    declarations.extend(analysis::expected::inputs(build::definition().1.method));
    declarations.extend([
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ValidationInput::of::<analysis::MethodParameters>(&["id"]),
        ValidationInput::of::<attribution::ProviderRun>(&["id"]),
    ]);
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations, runtime.budget())?;
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(inventory::<$ty>(&access,&session,&mut consumed,&mut admission,|batch|{
        if <$ty>::NAME==analysis::AnalysisDefinition::NAME{definitions.decode(batch)?;}
        if <$ty>::NAME==analysis::MethodParameters::NAME{parameters.decode(batch)?;}
        if <$ty>::NAME==attribution::ProviderRun::NAME{runs.decode(batch)?;}
        Ok(())
    }).await?;)*};}
    lctx_model::catalog_inputs!(read);
    lctx_model::expected_domain_inputs!(read);
    read! {definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,runs:attribution::ProviderRun,}
    consumed.finish(access.name())?;
    let (expected_parameters, definition) = build::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "catalog requires its completed authored definition".into(),
        ));
    }
    macro_rules! declare_outputs {($($field:ident:$ty:ty,)*)=>{$(output.declare_async::<$ty>().await?;)*};}
    lctx_model::catalog_outputs!(declare_outputs);
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare_async::<$ty>().await?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare_async::<analysis::catalog_core::$record>().await?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(catalog::CatalogMemberInvocation);
    let mut frames = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(runtime.budget(), "catalog-computation-frames");
    for run in runs.iter() {
        frames.insert(&mut charge, (run.input, run.context))?;
    }
    let mut invocations = Rows::new(runtime.budget());
    for (input, context) in frames.iter() {
        let (invocation, parents, receipts, projections) = Invocation::admitted(
            *input,
            *context,
            definition.id(),
            None,
            [],
            &sources,
            [],
            runtime.budget(),
        )?;
        if !parents.is_empty() || !projections.is_empty() {
            return Err(ModelError::Invalid(
                "catalog core has no parent or projection requirement".into(),
            ));
        }
        for receipt in receipts {
            output.push(receipt).await?;
        }
        let admitted = coverage::admit(
            &invocation,
            &definition,
            analysis::AnalysisCapability::Catalog,
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
    drop(frames);
    drop(runs);
    drop(definitions);
    drop(parameters);
    drop(charge);
    let scopes = crate::catalog_core_scope::CatalogScopes::prepare(
        &access,
        model,
        &session,
        runtime.budget(),
    )
    .await?;
    let names = access.table_for(&ValidationInput::of::<symbols::PublicNameObservation>(&[
        "id",
    ]))?;
    let mut roots = crate::sql::query(
        &session,
        &format!(
            "SELECT id FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&names)
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
            .ok_or(ModelError::Schema("C0 slot root identity"))?;
        for start in (0..batch.num_rows()).step_by(32) {
            runtime.cancellation().check()?;
            let keys = (start..(start + 32).min(batch.num_rows()))
                .map(|index| {
                    format!(
                        "X'{}'",
                        ids.value(index)
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    )
                })
                .collect::<Vec<_>>();
            let predicate = format!("id IN ({})", keys.join(","));
            let scope = scopes
                .edges
                .grain(scopes.root, &predicate, runtime.budget())
                .await?;
            let mut data = CatalogData::new(runtime.budget());
            let mut consumed =
                crate::consumed_rows::ConsumedInputs::new(scopes.inputs.clone(), runtime.budget())?;
            macro_rules! scoped_inputs {($($field:ident:$ty:ty,)*)=>{$(
                while let Some((input,permit))=consumed.next::<$ty>(&access)? {
                    let table=scopes.inputs.iter().position(|candidate|candidate.type_id()==input.type_id()&&candidate.prefix()==input.prefix())
                        .ok_or(ModelError::Conflict("C0 scoped input declaration"))?;
                    crate::consumed_rows::stream_query_at(&permit,&input,scope.session(),&scope.select(table)?,|_,batch|{data.visit(input.name(),batch)?;Ok(())}).await?;
                }
            )*};}
            lctx_model::catalog_inputs!(scoped_inputs);
            consumed.finish(access.name())?;
            drop(scope);
            let budget = runtime.budget().clone();
            let (data, rows) = tokio::task::spawn_blocking(move || {
                let rows = build::build(&data, &budget)?;
                Ok::<_, ModelError>((data, rows))
            })
            .await
            .map_err(ModelError::codec)??;
            let links = build::invocation_links(&data, &rows, &invocations, runtime.budget())?;
            drop(data);
            macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter(){output.push(row.clone()).await?;})*};}
            lctx_model::catalog_outputs!(write);
            for link in links.iter() {
                output.push(link.clone()).await?;
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
