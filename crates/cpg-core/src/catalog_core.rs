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
        let declarations = build::metadata_validation_inputs();
        let mut consumed =
            crate::consumed_rows::ConsumedInputs::new(declarations, runtime.budget())?;
        macro_rules! read {($($field:ident:$ty:ty,)*)=>{{
            const LOADERS:&[InventoryLoader]=&[$(inventory::<$ty>,)*];
            for load in LOADERS {load(access,session,&mut consumed,admission,&mut metadata).await?;}
        }};}
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
    runtime: &Arc<Workspace>,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = runtime.coverage_admission(&access, &sources).await?;
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
            let end = (start + 32).min(batch.num_rows());
            let _keys = runtime.budget().reserve(
                "C0 owned public window keys",
                (end - start) * size_of::<crate::consumed_rows::PreparedRoot>(),
            )?;
            let mut keys = Vec::with_capacity(end - start);
            for index in start..end {
                keys.push(crate::consumed_rows::PreparedRoot {
                    table: scopes.root,
                    key: ids.value(index).try_into().map_err(ModelError::codec)?,
                    kind: crate::consumed_rows::PreparedRootKind::Physical,
                });
            }
            let selected = scopes
                .edges
                .batch_with_cancellation(&keys, runtime.budget(), &runtime.cancellation())
                .await?;
            let mut union = CatalogData::new(runtime.budget());
            crate::scoped_batch::hydrate_union(
                &selected,
                &scopes.inputs,
                runtime.budget(),
                &runtime.cancellation(),
                &mut |_, input, batch| union.visit(input.name(), batch).map(|_| ()),
            )
            .await?;
            let mut _jobs = None;
            let mut work = Vec::new();
            for (partition, key) in keys.iter().enumerate() {
                let data = selected_partition(
                    &union,
                    &scopes.inputs,
                    &selected,
                    partition,
                    runtime.budget(),
                )?;
                let worker = runtime.clone();
                let implementation = output.implementation_identity();
                let key = key.key;
                let future = async move {
                    let mut domain = KeySink::new("catalog-public-root-domain/v2");
                    domain.part(b"root", &key);
                    let mut product = crate::catalog_products::ProductWindow::new(
                        &data,
                        implementation,
                        domain.finish(),
                        &worker,
                    )?;
                    let hit = product.lookup(&worker).await?;
                    let reused = hit.is_some();
                    let rows = match hit {
                        Some(rows) => rows,
                        None => product.compute(|| {
                            crate::stage_runtime::borrowed_cpu("catalog_core", || {
                                build::build(&data, worker.budget())
                            })
                        })?,
                    };
                    Ok((data, rows, product, reused))
                };
                if work.is_empty() {
                    _jobs = Some(runtime.budget().reserve(
                        "C0 owned window task list",
                        keys.len() * size_of_val(&future),
                    )?);
                    work.reserve_exact(keys.len());
                }
                work.push(future);
            }
            drop(selected);
            drop(union);
            let mut results = crate::owned_work::ordered(runtime, work).await?;
            while let Some((data, mut rows, mut product, reused)) = results.next().await? {
                let mut links = product.compute(|| {
                    build::invocation_links(&data, &rows, &invocations, runtime.budget())
                })?;
                drop(data);
                product.prepare_emission(&rows, &links, runtime.budget())?;
                if !reused {
                    product.retain(&rows, runtime).await?;
                }
                emit_rows(&mut rows, &output).await?;
                producer_operations::emit_owned(&mut links, &output).await?;
            }
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

/// Copy a single exact public-root partition from the once-hydrated owning window.
pub(super) fn selected_partition(
    union: &CatalogData,
    inputs: &[ValidationInput],
    selected: &crate::consumed_rows::PreparedRootBatch,
    partition: usize,
    budget: &resources::ResourceBudget,
) -> Result<CatalogData, ModelError> {
    union.selected_copy(inputs, &mut |table| selected.keys(partition, table), budget)
}
type Emitter = for<'a> fn(
    &'a mut build::CatalogOutput,
    &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn emit_rows<'a>(
    rows: &'a mut build::CatalogOutput,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! emit {($($field:ident:$ty:ty,)*)=>{{const EMITTERS:&[Emitter]=&[$(|rows,output|producer_operations::emit_owned(&mut rows.$field,output),)*];for emit in EMITTERS{emit(rows,output).await?;}}};}
        lctx_model::catalog_outputs!(emit);
        Ok(())
    })
}

#[cfg(test)]
mod metadata_controls {
    use super::*;
    use futures::FutureExt;
    use lctx_model::domain::stages::Profile;
    #[tokio::test]
    async fn catalog_metadata_does_not_acquire_irrelevant_rich_rows_but_rich_adapter_refuses_malformed_rows()
     {
        let budget = resources::ResourceBudget::fixed(128 << 20).unwrap();
        let workspace = Workspace::with_budget(
            Arc::new(lctx_model::domain::model().unwrap()),
            crate::workspace::WorkspaceOptions {
                memory_bytes: 128 << 20,
                ..Default::default()
            },
            budget.clone(),
            crate::test_native::store_with_budget(budget.clone()),
        )
        .unwrap();
        let result = std::panic::AssertUnwindSafe(async {
            let mut names = build::metadata_validation_inputs().iter().map(ValidationInput::name).collect::<Vec<_>>();
            names.push(calls::ProviderSymbol::NAME);
            let output = workspace.output("catalog-metadata-control-source", Profile::Catalog, ContentHash::of(b"catalog-metadata-control"),
                workspace.inputs("catalog-metadata-control-source", Profile::Catalog, []).unwrap(), names.clone());
            macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(if names.contains(&<$ty>::NAME) {output.declare_async::<$ty>().await.unwrap();})*};}
            lctx_model::expected_domain_inputs!(declare);
            declare! {definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,runs:attribution::ProviderRun,symbols:calls::ProviderSymbol,}
            let (parameters, definition) = build::definition();
            output.push(parameters.clone()).await.unwrap(); output.push(definition.clone()).await.unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            // The rich relation is genuinely completed, but absent from metadata's
            // captured ports. Any attempted rich acquisition must therefore refuse.
            let access = workspace.inputs("catalog-metadata-control-consumer", Profile::Catalog,
                names.iter().copied().filter(|name| *name != calls::ProviderSymbol::NAME)).unwrap();
            assert!(access.read::<calls::ProviderSymbol>().is_err());
            let sources = analysis::sources::CapturedSources::capture(Profile::Catalog, access.snapshots(), &budget).unwrap();
            let mut admission = analysis::expected::CoverageAdmission::new(&sources, &budget).unwrap();
            let session = access.session(&workspace).await.unwrap();
            let metadata = load_metadata(&access, &session, &workspace, &mut admission).await.unwrap();
            assert_eq!(metadata.definitions.get(definition.id()), Some(&definition));
            assert_eq!(metadata.parameters.get(parameters.id()), Some(&parameters));
            // Independent finite adapter evidence only: selected rich production
            // still uses CatalogData's typed visitor, which refuses a wrong schema.
            // This does not mutate captured aliases or assert native corruption.
            let malformed = input::Package::encode(&[input::Package { name:"not-a-symbol".into() }]).unwrap();
            let mut selected = CatalogData::new(&budget);
            assert!(selected.visit(calls::ProviderSymbol::NAME, &malformed).is_err(),
                "the rich typed adapter still refuses a malformed selected premise");
        }).catch_unwind().await;
        let mut finality = workspace.drain_owned_report().await;
        if finality.local == completion::LocalState::Terminal
            && finality.remote == completion::RemoteState::Confirmed
            && finality
                .failures
                .iter()
                .all(|failure| failure.error.permits_storage_cleanup())
        {
            finality.merge(workspace.native().abandon_finality_report().await);
        } else {
            finality
                .storage
                .push(completion::StorageState::Orphan(format!(
                    "native_attempt:{}",
                    workspace.native().attempt().hex()
                )));
        }
        let signal = workspace.complete_signal(&finality).await;
        finality.step("catalog metadata control workspace signal", signal);
        if let Err(panic) = result {
            if let Err(error) = completion::complete::<()>(Ok(()), finality) {
                eprintln!("catalog metadata cleanup after panic: {error:#?}");
            }
            std::panic::resume_unwind(panic);
        }
        completion::complete::<()>(Ok(()), finality).unwrap();
    }
}
