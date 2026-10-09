//! Store the explicit authored configuration and actual native premise projection once.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use futures::future::BoxFuture;
use lctx_model::domain::{
    analysis::{native::*, preparation::Configuration, *},
    assertion::AssertionQualification,
    models::{AuthoredContextProtocol, AuthoredModel, AuthoredTarget, ModelCatalog},
    stages::*,
    *,
};
use std::sync::Arc;

pub async fn configuration(
    _access: CompletedInputs,
    output: ProducerOutput,
    _model: &ValidatedModel,
    runtime: &Workspace,
    configuration: &Configuration,
) -> Result<(), ModelError> {
    configuration.check_budget(runtime.budget())?;
    macro_rules! write {
        ($ty:ty,$rows:expr) => {{
            output.declare_async::<$ty>().await?;
            for row in $rows.iter() {
                output.push(row.clone()).await?;
            }
        }};
    }
    write!(ModelCatalog, configuration.catalogs().catalogs);
    write!(AuthoredTarget, configuration.catalogs().targets);
    write!(AuthoredModel, configuration.catalogs().models);
    write!(AuthoredContextProtocol, configuration.catalogs().protocols);
    write!(MethodParameters, configuration.parameters());
    write!(AnalysisDefinition, configuration.definitions());
    write!(ProjectionDefinition, configuration.projections());
    write!(settings::AnalyticsConfiguration, configuration.analytics());
    write!(
        lctx_model::domain::retrieval::RetrievalDefinition,
        configuration.retrieval()
    );
    output.finish(ProviderOutcome::Complete).await
}

fn native_input<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    declaration: &'a ValidationInput,
    inventory: &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let permit = access.read_at::<R>(declaration.prefix())?;
        crate::consumed_rows::stream_at(&permit, declaration, access, session, |_, batch| {
            inventory.visit(R::NAME, batch)
        })
        .await
    })
}
fn native_declaration<R: Record>(
    declarations: &[ValidationInput],
) -> Result<&ValidationInput, ModelError> {
    declarations
        .iter()
        .find(|input| input.type_id() == std::any::TypeId::of::<R>())
        .ok_or(ModelError::Schema(
            "native inventory typed input declaration",
        ))
}
type PairLoader = for<'a> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a [ValidationInput],
    &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>>;
fn native_pair<'a, A: Record, S: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    declarations: &'a [ValidationInput],
    inventory: &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        // Presence is the existing assertion-family guard. A skipped pair neither resolves
        // its declarations nor acquires a support permit. Assertion failure precedes support.
        if access.contains::<A>() {
            native_input::<A>(
                access,
                session,
                native_declaration::<A>(declarations)?,
                inventory,
            )
            .await?;
            native_input::<S>(
                access,
                session,
                native_declaration::<S>(declarations)?,
                inventory,
            )
            .await?;
        }
        Ok(())
    })
}
fn load_native_inputs<'a>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    declarations: &'a [ValidationInput],
    inventory: &'a mut NativeInventory,
) -> BoxFuture<'a, Result<(), ModelError>> {
    macro_rules! pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*) => {
        const PAIRS: &[PairLoader] = &[$(native_pair::<$assertion, $support>,)*];
    };}
    lctx_model::native_analysis_pairs!(pairs);
    Box::pin(async move {
        native_input::<AssertionQualification>(
            access,
            session,
            native_declaration::<AssertionQualification>(declarations)?,
            inventory,
        )
        .await?;
        for load in PAIRS {
            load(access, session, declarations, inventory).await?;
        }
        Ok(())
    })
}
pub async fn native_inventory(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let session = access.session(runtime).await?;
    let mut inventory = NativeInventory::new(runtime.budget());
    let declarations = NativeInventory::inputs();
    load_native_inputs(&access, &session, &declarations, &mut inventory).await?;
    drop(session);
    let rows = inventory.collect()?;
    drop(inventory);
    output.declare_async::<NativeAssertionPremise>().await?;
    for row in rows.premises.iter() {
        output.push(row.clone()).await?;
    }
    output.declare_async::<NativeQualification>().await?;
    for row in rows.qualifications.iter() {
        output.push(row.clone()).await?;
    }
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}

/// Publish the one selected embedding configuration before analytic or retrieval work.
pub async fn embedding_configuration(
    _access: CompletedInputs,
    output: ProducerOutput,
    _model: &ValidatedModel,
    runtime: &Workspace,
    configuration: Option<&embedding::configuration::Configuration>,
) -> Result<(), ModelError> {
    use embedding::{
        DocumentRecipe, EmbeddingSpec, configuration::ServiceConfiguration,
        projection::ProjectionDefinition,
    };
    if let Some(configuration) = configuration {
        configuration.check_budget(runtime.budget())?;
    }
    output.declare_async::<EmbeddingSpec>().await?;
    output.declare_async::<ServiceConfiguration>().await?;
    output.declare_async::<DocumentRecipe>().await?;
    output.declare_async::<ProjectionDefinition>().await?;
    if let Some(configuration) = configuration {
        output.push(configuration.row().clone()).await?;
        output.push(configuration.service().clone()).await?;
        output.push(configuration.document().clone()).await?;
        output.push(configuration.projection().clone()).await?;
    }
    output.finish(ProviderOutcome::Complete).await
}
