//! Computed normalization stages over admitted completed-stage inputs.
use crate::{generation_read::{AttemptSession, ProviderOptions}, model_runtime::{AttemptRuntime, StageSession}};
use futures::TryStreamExt;
use lctx_model::domain::{normalized::{Rows, entity_normalization::{self, EntityData}}, stages::*, *};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;

/// One transfer-bounded stream at a time. The typed collector admits every retained row; the
/// source-bound session and PreparedQuery own remote scan admission and stream lifetime.
async fn load<R: Record>(session: &StageSession, rows: &mut Rows<R>) -> Result<(), ModelError> {
    let query = session.sql(&format!("SELECT * FROM \"{}\"", R::NAME)).await.map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? { rows.decode(&batch)?; }
    Ok(())
}
pub async fn entities(
    access: StageAccess<'_, '_>, attempt: &GenerationAttempt, config: &RoleConfig,
    runtime: &AttemptRuntime, model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let reader = AttemptSession::open(config, attempt, &access, model.clone(), ProviderOptions::default()).await.map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = EntityData::new(runtime.budget());
    macro_rules! read_inputs { ($($field:ident: $ty:ty => $family:ident,)*) => { $(
        let permit = access.read::<$ty>()?;
        session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session, &mut data.$field).await?;
    )* }; }
    lctx_model::normalized_entity_inputs!(read_inputs);
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let rows = entity_normalization::normalize(data.inputs(), runtime.budget())?;
    drop(data);
    let mut output = StageOutput::new(access, attempt, model, runtime.budget().clone(), Default::default())?;
    macro_rules! write_outputs { ($($field:ident: $ty:ty,)*) => { $(
        output.declare::<$ty>()?;
        for row in rows.$field.iter() { output.push(row.clone()).await?; }
    )* }; }
    lctx_model::normalized_entity_outputs!(write_outputs);
    drop(rows);
    output.finish(ProviderOutcome::Complete).await
}
