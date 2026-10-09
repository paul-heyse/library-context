//! C0 execution consumes completed native/normalized authorities. No flow or brief producer.
use crate::producer_operations::{self, Declaration};
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use datafusion::execution::context::SessionContext;
use futures::{TryStreamExt, future::BoxFuture};
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
struct Metadata {
    definitions: Rows<analysis::AnalysisDefinition>,
    parameters: Rows<analysis::MethodParameters>,
    runs: Rows<attribution::ProviderRun>,
}
type InventoryLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut Metadata,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn inventory<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    metadata: &'a mut Metadata,
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
                if R::NAME == analysis::AnalysisDefinition::NAME {
                    metadata.definitions.decode(batch)?;
                }
                if R::NAME == analysis::MethodParameters::NAME {
                    metadata.parameters.decode(batch)?;
                }
                if R::NAME == attribution::ProviderRun::NAME {
                    metadata.runs.decode(batch)?;
                }
                Ok(())
            })
            .await?;
        }
        Ok(())
    })
}
fn load_metadata<'a, 'sources>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    runtime: &'a Workspace,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
) -> BoxFuture<'a, Result<Metadata, ModelError>> {
    Box::pin(async move {
        let mut metadata = Metadata {
            definitions: Rows::new(runtime.budget()),
            parameters: Rows::new(runtime.budget()),
            runs: Rows::new(runtime.budget()),
        };
        let mut declarations = CatalogData::validation_inputs();
        declarations.extend(analysis::expected::inputs(build::definition().1.method));
        declarations.extend([
            ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
            ValidationInput::of::<analysis::MethodParameters>(&["id"]),
            ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ]);
        let mut consumed =
            crate::consumed_rows::ConsumedInputs::new(declarations, runtime.budget())?;
        macro_rules! read {($($field:ident:$ty:ty,)*)=>{{
            const LOADERS:&[InventoryLoader]=&[$(inventory::<$ty>,)*];
            for load in LOADERS {load(access,session,&mut consumed,admission,&mut metadata).await?;}
        }};}
        lctx_model::catalog_inputs!(read);
        lctx_model::expected_domain_inputs!(read);
        read! {definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,runs:attribution::ProviderRun,}
        consumed.finish(access.name())?;
        Ok(metadata)
    })
}
fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! declare {($($field:ident:$ty:ty,)*)=>{{const DECLARATIONS:&[Declaration]=&[$(producer_operations::declare::<$ty>,)*];producer_operations::declare_ordered(output,DECLARATIONS).await?;}};}
        lctx_model::catalog_outputs!(declare);
        macro_rules! common {($($record:ident,)*)=>{{const DECLARATIONS:&[Declaration]=&[$(producer_operations::declare::<analysis::catalog_core::$record>,)*];producer_operations::declare_ordered(output,DECLARATIONS).await?;}};}
        lctx_model::analysis_publication!(common);
        producer_operations::declare::<catalog::CatalogMemberInvocation>(output).await
    })
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
    let metadata = load_metadata(&access, &session, runtime, &mut admission).await?;
    let (expected_parameters, definition) = build::definition();
    if metadata.definitions.get(definition.id()) != Some(&definition)
        || metadata.parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "catalog requires its completed authored definition".into(),
        ));
    }
    declare_outputs(&output).await?;
    let mut frames = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(runtime.budget(), "catalog-computation-frames");
    for run in metadata.runs.iter() {
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
    drop(metadata);
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
            let data = load_scope(&scopes, &scope, &access, runtime).await?;
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
            emit_rows(&rows, &output).await?;
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

type ScopedLoader = for<'a> fn(
    &'a crate::catalog_core_scope::CatalogScopes,
    &'a crate::consumed_rows::PreparedClosure,
    &'a CompletedInputs,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut CatalogData,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn load_record<'a, R: Record>(
    scopes: &'a crate::catalog_core_scope::CatalogScopes,
    scope: &'a crate::consumed_rows::PreparedClosure,
    access: &'a CompletedInputs,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    data: &'a mut CatalogData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            let table = scopes
                .inputs
                .iter()
                .position(|candidate| {
                    candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix()
                })
                .ok_or(ModelError::Conflict("C0 scoped input declaration"))?;
            crate::consumed_rows::stream_query_at(
                &permit,
                &input,
                access,
                scope.session(),
                &scope.select(table)?,
                |_, batch| {
                    data.visit(input.name(), batch)?;
                    Ok(())
                },
            )
            .await?;
        }
        Ok(())
    })
}
fn load_scope<'a>(
    scopes: &'a crate::catalog_core_scope::CatalogScopes,
    scope: &'a crate::consumed_rows::PreparedClosure,
    access: &'a CompletedInputs,
    runtime: &'a Workspace,
) -> BoxFuture<'a, Result<CatalogData, ModelError>> {
    Box::pin(async move {
        let mut data = CatalogData::new(runtime.budget());
        let mut consumed =
            crate::consumed_rows::ConsumedInputs::new(scopes.inputs.clone(), runtime.budget())?;
        macro_rules! read {($($field:ident:$ty:ty,)*)=>{{const LOADERS:&[ScopedLoader]=&[$(load_record::<$ty>,)*];for load in LOADERS{load(scopes,scope,access,&mut consumed,&mut data).await?;}}};}
        lctx_model::catalog_inputs!(read);
        consumed.finish(access.name())?;
        Ok(data)
    })
}
type Emitter = for<'a> fn(
    &'a build::CatalogOutput,
    &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn emit_rows<'a>(
    rows: &'a build::CatalogOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! emit {($($field:ident:$ty:ty,)*)=>{{const EMITTERS:&[Emitter]=&[$(|rows,output|producer_operations::emit(&rows.$field,output),)*];for emit in EMITTERS{emit(rows,output).await?;}}};}
        lctx_model::catalog_outputs!(emit);
        Ok(())
    })
}
