//! Store the explicit authored configuration and actual native premise projection once.
use crate::{
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::TryStreamExt;
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
            output.declare::<$ty>()?;
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

async fn native_input<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    inventory: &mut NativeInventory,
) -> Result<(), ModelError> {
    let _permit = access.read::<R>()?;
    
    let query = crate::sql::query(&session,&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        inventory.visit(R::NAME, &batch)?;
    }
    Ok(())
}
pub async fn native_inventory(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let session = access.session(runtime).await?;
    let mut inventory = NativeInventory::new(runtime.budget());
    native_input::<AssertionQualification>(&access, &session, &mut inventory).await?;
    macro_rules! read_pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        if access.contains::<$assertion>() {
            native_input::<$assertion>(&access,&session,&mut inventory).await?;
            native_input::<$support>(&access,&session,&mut inventory).await?;
        }
    )*};}
    lctx_model::native_analysis_pairs!(read_pairs);
    drop(session);
    let rows = inventory.collect()?;
    drop(inventory);
    output.declare::<NativeAssertionPremise>()?;
    for row in rows.premises.iter() {
        output.push(row.clone()).await?;
    }
    output.declare::<NativeQualification>()?;
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
        EmbeddingSpec,
        configuration::ServiceConfiguration,
    };
    if let Some(configuration) = configuration {
        configuration.check_budget(runtime.budget())?;
    }
    output.declare::<EmbeddingSpec>()?;
    output.declare::<ServiceConfiguration>()?;
    if let Some(configuration) = configuration {
        output.push(configuration.row().clone()).await?;
        output.push(configuration.service().clone()).await?;
    }
    output.finish(ProviderOutcome::Complete).await
}
