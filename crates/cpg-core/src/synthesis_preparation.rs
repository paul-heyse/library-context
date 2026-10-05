//! Pure S0 documentary preparation through completed lower-owner grants.
//! The single final S0 producer uses this operation; it does not create a second analysis owner.
use datafusion::execution::context::SessionContext;
use crate::{
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    normalized::Rows,
    stages::*,
    synthesis::documentary::{Data, Output},
    *,
};
use std::sync::Arc;
async fn load<R: Record>(session: &SessionContext, rows: &mut Rows<R>) -> Result<(), ModelError> {
    let query = session
        .sql(&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        rows.decode(&batch)?;
    }
    Ok(())
}
pub async fn documentary(
    access: &CompletedInputs,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(Data, Output), ModelError> {
    let session = runtime.session(access);
    let mut data = Data::new(runtime.budget());
    macro_rules! read{($($field:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;load(&session,&mut data.$field).await?;)*};}
    lctx_model::synthesis_documentary_inputs!(read);
    drop(session);
    let budget = runtime.budget().clone();
    tokio::task::spawn_blocking(move || {
        let output = lctx_model::domain::synthesis::documentary::build(&data, &budget)?;
        Ok((data, output))
    })
    .await
    .map_err(ModelError::codec)?
}
/// The final S0 source-code operation reads actual earlier code blocks through the same grants.
pub async fn code_blocks(
    access: &CompletedInputs,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<Rows<documents::CodeBlockObservation>, ModelError> {
    let session = runtime.session(access);
    let permit = access.read::<documents::CodeBlockObservation>()?;
    
    let mut rows = Rows::new(runtime.budget());
    load(&session, &mut rows).await?;
    drop(session);
    Ok(rows)
}
pub async fn source_setup(
    access: &CompletedInputs,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<synthesis::source_setup::Data, ModelError> {
    let session = runtime.session(access);
    let mut rows = synthesis::source_setup::Data::new(runtime.budget());
    macro_rules! read{($($field:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;load(&session,&mut rows.$field).await?;)*};}
    lctx_model::synthesis_setup_inputs!(read);
    drop(session);
    Ok(rows)
}
pub async fn publish_documentary(
    output: &mut ProducerOutput,
    rows: &Output,
) -> Result<(), ModelError> {
    output.declare::<synthesis::documentary::DocumentaryConclusion>()?;
    output.declare::<synthesis::documentary::DocumentaryBoundary>()?;
    output.declare::<synthesis::documentary_templates::ComponentBoundary>()?;
    output.declare::<synthesis::documentary::ProseSlice>()?;
    output.declare::<synthesis::documentary::ProseSource>()?;
    output.declare::<synthesis::documentary::DocumentarySource>()?;
    output.declare::<assertion::AssertionQualification>()?;
    for row in rows.sources.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.prose_sources.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.slices.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.qualifications.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.conclusions.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.component_boundaries.iter() {
        output.push(row.clone()).await?;
    }
    for row in rows.boundaries.iter() {
        output.push(row.clone()).await?;
    }
    Ok(())
}
