//! Pure S0 documentary preparation through completed lower-owner grants.
//! The single final S0 producer uses this operation; it does not create a second analysis owner.
use crate::{generation_read::{AttemptSession,ProviderOptions},model_runtime::{AttemptRuntime,StageSession}};
use futures::TryStreamExt;
use lctx_model::domain::{*,synthesis::documentary::{Data,Output},normalized::Rows,stages::*};
use lctx_postgres::{generations::GenerationAttempt,roles::RoleConfig};
use std::sync::Arc;
async fn load<R:Record>(session:&StageSession,rows:&mut Rows<R>)->Result<(),ModelError>{let query=session.query(&format!("SELECT * FROM \"{}\"",R::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{rows.decode(&batch)?;}Ok(())}
pub async fn documentary(access:&StageAccess<'_, '_>,attempt:&GenerationAttempt,config:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>)->Result<(Data,Output),ModelError>{
 let reader=AttemptSession::open(config,attempt,access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;let session=runtime.session(access);let mut data=Data::new(runtime.budget());
 macro_rules! read{($($field:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.$field).await?;)*};}lctx_model::synthesis_documentary_inputs!(read);
 drop(session);reader.close().await.map_err(ModelError::codec)?;let budget=runtime.budget().clone();tokio::task::spawn_blocking(move||{let output=lctx_model::domain::synthesis::documentary::build(&data,&budget)?;Ok((data,output))}).await.map_err(ModelError::codec)?
}
pub async fn publish_documentary<S:StageSink>(output:&mut StageOutput<'_, '_, '_,S>,rows:&Output)->Result<(),ModelError>{
 output.declare::<synthesis::documentary::DocumentaryConclusion>()?;output.declare::<synthesis::documentary::DocumentaryBoundary>()?;output.declare::<synthesis::documentary::ProseSlice>()?;output.declare::<assertion::AssertionQualification>()?;
 for row in rows.slices.iter(){output.push(row.clone()).await?;}for row in rows.qualifications.iter(){output.push(row.clone()).await?;}for row in rows.conclusions.iter(){output.push(row.clone()).await?;}for row in rows.boundaries.iter(){output.push(row.clone()).await?;}Ok(())
}
