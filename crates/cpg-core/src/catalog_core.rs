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
async fn load<R: Record>(
    session: &SessionContext,
    rows: &mut Rows<R>,
    permit: &analysis::sources::CompletedInput<R>,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<(), ModelError> {
    let query = crate::sql::query(&session, &format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        admission.visit_if_expected(permit, &batch)?;
        rows.decode(&batch)?;
    }
    Ok(())
}
/// Internal stage entry; public catalog frontiers are assembled separately by F0.
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let session = access.session(runtime).await?;
    let mut registered = charged::ChargedSet::default();
    let mut registration =
        charged::StateCharge::new(runtime.budget(), "expected-input-registration");
    let mut data = CatalogData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(
        let permit=access.read::<$ty>()?;
        registered.insert(&mut registration,<$ty>::NAME)?;
        load(&session,&mut data.$field,&permit,&mut admission).await?;
    )*};}
    lctx_model::catalog_inputs!(read);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters = Rows::<analysis::MethodParameters>::new(runtime.budget());
    let mut runs = Rows::<attribution::ProviderRun>::new(runtime.budget());
    macro_rules! meta {
        ($ty:ty,$rows:ident,$admit:expr) => {{
            let permit = access.read::<$ty>()?;

            registered.insert(&mut registration, <$ty>::NAME)?;
            load(&session, &mut $rows, &permit, $admit).await?;
        }};
    }
    meta!(analysis::AnalysisDefinition, definitions, &mut admission);
    meta!(analysis::MethodParameters, parameters, &mut admission);
    meta!(attribution::ProviderRun, runs, &mut admission);
    macro_rules! expected {($($field:ident:$ty:ty,)*)=>{$({if access.contains::<$ty>() && !registered.contains(<$ty>::NAME){let mut rows=Rows::<$ty>::new(runtime.budget());meta!($ty,rows,&mut admission);}})*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    let budget = runtime.budget().clone();
    let (data, rows) = tokio::task::spawn_blocking(move || {
        let rows = build::build(&data, &budget)?;
        Ok::<_, ModelError>((data, rows))
    })
    .await
    .map_err(ModelError::codec)??;
    let (expected_parameters, definition) = build::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "catalog requires its completed authored definition".into(),
        ));
    }
    macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$field.iter() {output.push(row.clone()).await?;})*};}
    lctx_model::catalog_outputs!(write);
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<analysis::catalog_core::$record>()?;)*};}
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
    let links = build::invocation_links(&data, &rows, &invocations, runtime.budget())?;
    for link in links.iter() {
        output.push(link.clone()).await?;
    }
    drop(links);
    drop(invocations);
    drop(rows);
    drop(data);
    drop(admission);
    drop(sources);
    drop(charge);
    output.finish(ProviderOutcome::Complete).await
}

/// Retained callable/field metadata completed under the normalized authority before C0.
pub async fn aspects(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    use normalized::callable_aspects::{self, AspectData};
    let session = access.session(runtime).await?;
    let mut data = AspectData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$({access.read::<$ty>()?;let query=crate::sql::query(&session,&format!("SELECT * FROM \"{}\"",<$ty>::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{data.$field.decode(&batch)?;}})*};}
    lctx_model::callable_aspect_inputs!(read);
    drop(session);
    let budget = runtime.budget().clone();
    let rows = tokio::task::spawn_blocking(move || callable_aspects::normalize(&data, &budget))
        .await
        .map_err(ModelError::codec)??;
    macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$field.iter() {output.push(row.clone()).await?;})*};}
    lctx_model::callable_aspect_outputs!(write);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}
