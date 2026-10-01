//! E0 mandatory preparation over actual completed C1 grants; final realization reuses this helper.
use crate::{generation_read::{AttemptSession,ProviderOptions},model_runtime::{AttemptRuntime,StageSession}};
use futures::TryStreamExt;
use lctx_model::domain::{*,retrieval::build::{self,Data,Output},normalized::Rows,stages::*};
use lctx_postgres::{generations::GenerationAttempt,roles::RoleConfig};
use std::sync::Arc;
async fn load<R:Record>(session:&StageSession,rows:&mut Rows<R>)->Result<(),ModelError> {let query=session.query(&format!("SELECT * FROM \"{}\"",R::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {rows.decode(&batch)?;}Ok(())}
/// Read through the declared finite source closure. Original bytes come solely from ArtifactChunk.
pub async fn mandatory(access:&StageAccess<'_, '_>,attempt:&GenerationAttempt,config:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>)->Result<(Data,Output),ModelError> {
 let reader=AttemptSession::open(config,attempt,access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;let session=runtime.session(access);let mut data=Data::new(runtime.budget());
 macro_rules! core {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.source.core.$f).await?;)*};}lctx_model::catalog_inputs!(core);
 macro_rules! catalog {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.source.catalog.$f).await?;)*};}lctx_model::catalog_outputs!(catalog);
 macro_rules! facts {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.source.facts.$f).await?;)*};}lctx_model::catalog_evidence_inputs!(facts);
 macro_rules! evidence {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.evidence.$f).await?;)*};}lctx_model::catalog_evidence_outputs!(evidence);
 macro_rules! extra {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.facts.$f).await?;)*};}lctx_model::retrieval_inputs!(extra);
 macro_rules! lower {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.source.runtime.$f).await?;)*};}lctx_model::catalog_runtime_inputs!(lower);
 drop(session);reader.close().await.map_err(ModelError::codec)?;
 catalog::evidence::frames::verify(&data.source.facts.runs,&data.source.facts.core_invocations,&data.facts.evidence_invocations,&data.facts.evidence_sources,&data.facts.evidence_inputs,&data.source.runtime.lower(),runtime.budget())?;
 if !data.facts.evidence_links.same(&catalog::evidence::build::invocation_links(&data.evidence,&data.facts.evidence_invocations,runtime.budget())?) {return Err(build::invalid("retrieval C1 root invocation closure differs"));}
 let budget=runtime.budget().clone();tokio::task::spawn_blocking(move || {let output=build::build(&data,&budget)?;Ok((data,output))}).await.map_err(ModelError::codec)?
}
/// Same publication operation used by the final E0 producer. The scoped native control declares
/// only this mandatory boundary; it makes no completed S0/realization claim.
pub async fn publish_mandatory<S:StageSink>(output:&mut StageOutput<'_, '_, '_, S>,rows:&Output)->Result<(),ModelError> {macro_rules! write {($($f:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$f.iter() {output.push(row.clone()).await?;})*};}lctx_model::retrieval_outputs!(write);Ok(())}
