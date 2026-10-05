//! Last immutable frontier assessment over actual captured native frames and owner results.
use crate::{
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::frontier::{self, FrontierData, Target},
    stages::*,
    *,
};
use std::sync::Arc;
async fn load<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    data: &mut FrontierData,
) -> Result<(), ModelError> {
    let _permit = access.read::<R>()?;
    
    let query = crate::sql::query(&session,&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        data.visit(R::NAME, &batch)?;
    }
    Ok(())
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
    target: Target,
) -> Result<(), ModelError> {
    let budget = runtime.budget();
    let _inputs = budget.reserve(
        "final-frontier-reader-inputs",
        FrontierData::inputs(target).len().saturating_mul(1024),
    )?;
    let inputs = FrontierData::inputs(target);
    let mut data = FrontierData::new(access.profile(), budget);
    let session = access.session(runtime).await?;
    macro_rules! read{($($ty:ty),* $(,)?)=>{$(if inputs.iter().any(|i|i.name()==<$ty>::NAME){load::<$ty>(&access,&session,&mut data).await?;})*};}
    lctx_model::final_frontier_inputs!(read);
    let records = frontier::derive(&data, target, budget)?;
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
