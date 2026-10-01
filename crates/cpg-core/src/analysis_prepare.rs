//! Store the explicit authored configuration and actual native premise projection once.
use crate::{generation_read::{AttemptSession, ProviderOptions}, model_runtime::{AttemptRuntime, StageSession}};
use futures::TryStreamExt;
use lctx_model::domain::{*, analysis::{*, native::*, preparation::Configuration}, assertion::AssertionQualification,
    models::{ModelCatalog, AuthoredTarget, AuthoredModel, AuthoredContextProtocol}, stages::*};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;

pub async fn configuration(access: StageAccess<'_, '_>, attempt: &GenerationAttempt, model: &ValidatedModel,
    runtime: &AttemptRuntime, configuration: &Configuration) -> Result<(),ModelError> {
    configuration.check_budget(runtime.budget())?;
    let declaration=configuration.declaration();
    if access.stage().name!=declaration.name || access.stage().configuration!=declaration.configuration || access.stage().code!=declaration.code {
        return Err(ModelError::Invalid("authored analysis configuration differs from preflight".into()));
    }
    let mut output=StageOutput::new(access,attempt,model,runtime.budget().clone(),Default::default())?;
    macro_rules! write {($ty:ty,$rows:expr)=>{{output.declare::<$ty>()?;for row in $rows.iter() {output.push(row.clone()).await?;}}};}
    write!(ModelCatalog,configuration.catalogs().catalogs);
    write!(AuthoredTarget,configuration.catalogs().targets);
    write!(AuthoredModel,configuration.catalogs().models);
    write!(AuthoredContextProtocol,configuration.catalogs().protocols);
    write!(MethodParameters,configuration.parameters());
    write!(AnalysisDefinition,configuration.definitions());
    write!(ProjectionDefinition,configuration.projections());
    output.finish(ProviderOutcome::Complete).await
}

async fn native_input<R:Record>(access: &StageAccess<'_, '_>, reader: &AttemptSession, session: &StageSession,
    inventory: &mut NativeInventory) -> Result<(),ModelError> {
    let permit=access.read::<R>()?;
    session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;
    let query=session.query(&format!("SELECT * FROM \"{}\"",R::NAME)).await.map_err(ModelError::codec)?;
    let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {inventory.visit(R::NAME,&batch)?;}
    Ok(())
}
pub async fn native_inventory(access: StageAccess<'_, '_>, attempt: &GenerationAttempt, config: &RoleConfig,
    runtime: &AttemptRuntime, model: &Arc<ValidatedModel>) -> Result<(),ModelError> {
    let reader=AttemptSession::open(config,attempt,&access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;
    let session=runtime.session(&access);
    let mut inventory=NativeInventory::new(runtime.budget());
    native_input::<AssertionQualification>(&access,&reader,&session,&mut inventory).await?;
    macro_rules! read_pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        if access.stage().reads::<$assertion>() {
            native_input::<$assertion>(&access,&reader,&session,&mut inventory).await?;
            native_input::<$support>(&access,&reader,&session,&mut inventory).await?;
        }
    )*};}
    lctx_model::native_analysis_pairs!(read_pairs);
    drop(session);reader.close().await.map_err(ModelError::codec)?;
    let rows=inventory.collect()?;drop(inventory);
    let mut output=StageOutput::new(access,attempt,model,runtime.budget().clone(),Default::default())?;
    output.declare::<NativeAssertionPremise>()?;
    for row in rows.premises.iter() {output.push(row.clone()).await?;}
    output.declare::<NativeQualification>()?;
    for row in rows.qualifications.iter() {output.push(row.clone()).await?;}
    drop(rows);output.finish(ProviderOutcome::Complete).await
}

/// Publish the one selected embedding configuration before analytic or retrieval work.
pub async fn embedding_configuration(access:StageAccess<'_, '_>,attempt:&GenerationAttempt,model:&ValidatedModel,
    runtime:&AttemptRuntime,configuration:Option<&embedding::configuration::Configuration>)->Result<(),ModelError> {
    use embedding::{EmbeddingSpec,configuration::{ServiceConfiguration,stage}};
    if let Some(configuration)=configuration {configuration.check_budget(runtime.budget())?;}
    let declaration=stage(configuration);
    if access.stage().name!=declaration.name || access.stage().configuration!=declaration.configuration || access.stage().code!=declaration.code {
        return Err(ModelError::Invalid("embedding configuration differs from preflight".into()));
    }
    let mut output=StageOutput::new(access,attempt,model,runtime.budget().clone(),Default::default())?;
    output.declare::<EmbeddingSpec>()?;output.declare::<ServiceConfiguration>()?;
    if let Some(configuration)=configuration {output.push(configuration.row().clone()).await?;output.push(configuration.service().clone()).await?;}
    output.finish(ProviderOutcome::Complete).await
}
