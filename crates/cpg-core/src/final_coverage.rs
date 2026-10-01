//! Last immutable frontier assessment over actual captured native frames and owner results.
use crate::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, StageSession},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::frontier::{self, FrontierData, Target},
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;
async fn load<R: Record>(
    access: &StageAccess<'_, '_>,
    reader: &AttemptSession,
    session: &StageSession,
    data: &mut FrontierData,
) -> Result<(), ModelError> {
    let permit = access.read::<R>()?;
    session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
    let query = session
        .query(&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        data.visit(R::NAME, &batch)?;
    }
    Ok(())
}
pub async fn produce(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    roles: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    target: Target,
) -> Result<(), ModelError> {
    let budget = runtime.budget();
    let _inputs = budget.reserve(
        "final-frontier-reader-inputs",
        FrontierData::inputs(target).len().saturating_mul(1024),
    )?;
    let inputs = FrontierData::inputs(target);
    let mut data = FrontierData::new(access.profile(), budget);
    let reader = AttemptSession::open(
        roles,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    macro_rules! read{($($ty:ty),* $(,)?)=>{$(if inputs.iter().any(|i|i.name()==<$ty>::NAME){load::<$ty>(&access,&reader,&session,&mut data).await?;})*};}
    lctx_model::final_frontier_inputs!(read);
    reader.close().await.map_err(ModelError::codec)?;
    let records = frontier::derive(&data, target, budget)?;
    let mut output = StageOutput::new(access, attempt, model, budget.clone(), Default::default())?;
    match target {
        Target::Analysis => {
            output.declare::<frontier::AnalysisAssessment>()?;
            output.declare::<frontier::AnalysisMember>()?;
            for row in records.analysis.iter() {
                output.push(row.clone()).await?;
            }
            for row in records.analysis_members.iter() {
                output.push(row.clone()).await?;
            }
        }
        Target::Catalog => {
            output.declare::<frontier::CatalogAssessment>()?;
            output.declare::<frontier::CatalogMember>()?;
            for row in records.catalog.iter() {
                output.push(row.clone()).await?;
            }
            for row in records.catalog_members.iter() {
                output.push(row.clone()).await?;
            }
        }
    }
    drop(records);
    drop(data);
    output.finish(ProviderOutcome::Complete).await
}
