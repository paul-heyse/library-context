//! Last immutable frontier assessment over actual captured native frames and owner results.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use lctx_model::domain::{
    analysis::frontier::{self, FrontierData, Target},
    stages::*,
    *,
};
use std::sync::Arc;
async fn load<R: Record>(
    access: &CompletedInputs,
    session: &datafusion::prelude::SessionContext,
    declaration: &ValidationInput,
    data: &mut FrontierData,
) -> Result<(), ModelError> {
    let permit = access.read_at::<R>(declaration.prefix())?;
    crate::consumed_rows::stream_at(&permit, declaration, access, session, |_, batch| {
        data.visit(R::NAME, batch)
    }).await
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
    macro_rules! read{($($ty:ty),* $(,)?)=>{$(if let Some(input)=inputs.iter().find(|i|i.type_id()==std::any::TypeId::of::<$ty>()){load::<$ty>(&access,&session,input,&mut data).await?;})*};}
    lctx_model::final_frontier_inputs!(read);
    let records = frontier::derive(&data, target, budget)?;
    match target {
        Target::Analysis => {
            output.declare_async::<frontier::AnalysisAssessment>().await?;
            output.declare_async::<frontier::AnalysisMember>().await?;
            for row in records.analysis.iter() {
                output.push(row.clone()).await?;
            }
            for row in records.analysis_members.iter() {
                output.push(row.clone()).await?;
            }
        }
        Target::Catalog => {
            output.declare_async::<frontier::CatalogAssessment>().await?;
            output.declare_async::<frontier::CatalogMember>().await?;
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
