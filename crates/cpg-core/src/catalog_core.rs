//! C0 execution consumes completed native/normalized authorities. No flow or brief producer.
use crate::{generation_read::{AttemptSession,ProviderOptions},model_runtime::{AttemptRuntime,StageSession}};
use futures::TryStreamExt;
use lctx_model::domain::{*,catalog::{self,build::{self,CatalogData}},analysis::{self,catalog_core::*},normalized::Rows,stages::*};
use lctx_postgres::{generations::GenerationAttempt,roles::RoleConfig};
use std::sync::Arc;
async fn load<R:Record>(session:&StageSession,rows:&mut Rows<R>,permit:&ReadPermit<'_,R>,admission:Option<&mut analysis::expected::CoverageAdmission<'_>>)->Result<(),ModelError> {
    let query=session.query(&format!("SELECT * FROM \"{}\"",R::NAME)).await.map_err(ModelError::codec)?;
    let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;
    let mut admission=admission;
    while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {
        if let Some(admission)=admission.as_deref_mut() {admission.visit(permit,&batch)?;}
        rows.decode(&batch)?;
    }
    Ok(())
}
/// Internal stage entry; public catalog frontiers are assembled separately by F0.
pub async fn produce(access:StageAccess<'_, '_>,attempt:&GenerationAttempt,config:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>)->Result<(),ModelError> {
    let sources=analysis::sources::CapturedSources::capture(&access,runtime.budget())?;
    let mut admission=analysis::expected::CoverageAdmission::new(&sources,runtime.budget())?;
    let reader=AttemptSession::open(config,attempt,&access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;
    let session=runtime.session(&access);
    let mut data=CatalogData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(
        let permit=access.read::<$ty>()?;
        session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;
        load(&session,&mut data.$field,&permit,if <$ty>::NAME==source::SourceArtifact::NAME {Some(&mut admission)}else {None}).await?;
    )*};}
    lctx_model::catalog_inputs!(read);
    let mut definitions=Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters=Rows::<analysis::MethodParameters>::new(runtime.budget());
    let mut runs=Rows::<attribution::ProviderRun>::new(runtime.budget());
    macro_rules! meta {($ty:ty,$rows:ident,$admit:expr)=>{{let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut $rows,&permit,$admit).await?;}};}
    meta!(analysis::AnalysisDefinition,definitions,None);meta!(analysis::MethodParameters,parameters,None);meta!(attribution::ProviderRun,runs,None);
    macro_rules! expected {($($ty:ty),*)=>{$({let mut rows=Rows::<$ty>::new(runtime.budget());meta!($ty,rows,Some(&mut admission));})*};}
    expected!(input::InputRevision,input::ArtifactUse,source::CoverageScope,normalized::coverage::NormalizationComputation,normalized::coverage::NormalizationCoverage);
    drop(session);reader.close().await.map_err(ModelError::codec)?;
    let budget=runtime.budget().clone();
    let (data,rows)=tokio::task::spawn_blocking(move || {let rows=build::build(&data,&budget)?;Ok::<_,ModelError>((data,rows))}).await.map_err(ModelError::codec)??;
    let (expected_parameters,definition)=build::definition();
    if definitions.get(definition.id())!=Some(&definition) || parameters.get(expected_parameters.id())!=Some(&expected_parameters) {return Err(ModelError::Invalid("catalog requires its completed authored definition".into()));}
    let mut output=StageOutput::new(access,attempt,model,runtime.budget().clone(),Default::default())?;
    macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$field.iter() {output.push(row.clone()).await?;})*};}
    lctx_model::catalog_outputs!(write);
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    declare!(Invocation,InvocationSource,AnalysisInput,ProjectionInput,SourceReceipt,AnalysisOutcome,AnalysisCoverage,CoverageSource,AnalysisCoveragePremise,CoverageRequirement,CoverageRequiredSource,catalog::CatalogMemberInvocation);
    let mut frames=charged::ChargedSet::default();
    let mut charge=charged::StateCharge::new(runtime.budget(),"catalog-computation-frames");
    for run in runs.iter() {frames.insert(&mut charge,(run.input,run.context))?;}
    let mut invocations=Rows::new(runtime.budget());
    for (input,context) in frames.iter() {
        let (invocation,parents,receipts,projections)=Invocation::admitted(*input,*context,definition.id(),None,[],&sources,[],runtime.budget())?;
        if !parents.is_empty() || !projections.is_empty() {return Err(ModelError::Invalid("catalog core has no parent or projection requirement".into()));}
        for receipt in receipts {output.push(receipt).await?;}
        let admitted=coverage::admit(&invocation,&definition,analysis::AnalysisCapability::Catalog,&admission,runtime.budget())?;
        for scope in admitted.scopes() {
            let (requirement,members)=scope.expectation().records()?;output.push(requirement).await?;
            for member in members {output.push(member).await?;}
            for observed in scope.observations() {output.push(observed.source().clone()).await?;}
            let (coverage,members)=coverage::assess(scope.expectation(),scope.observations(),analysis::AnalysisStatus::Completed,None,runtime.budget())?;
            output.push(coverage).await?;for member in members {output.push(member).await?;}
        }
        output.push(AnalysisOutcome {invocation:invocation.id(),status:analysis::AnalysisStatus::Completed,reason:None}).await?;
        output.push(invocation.clone()).await?;invocations.insert(invocation)?;
    }
    let links=build::invocation_links(&data,&rows,&invocations,runtime.budget())?;
    for link in links.iter() {output.push(link.clone()).await?;}
    drop(links);drop(invocations);drop(rows);drop(data);drop(admission);drop(sources);drop(charge);
    output.finish(ProviderOutcome::Complete).await
}

/// Retained callable/field metadata completed under the normalized authority before C0.
pub async fn aspects(access:StageAccess<'_, '_>,attempt:&GenerationAttempt,config:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>)->Result<(),ModelError> {
    use normalized::callable_aspects::{self,AspectData};
    let reader=AttemptSession::open(config,attempt,&access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;
    let session=runtime.session(&access);let mut data=AspectData::new(runtime.budget());
    macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;load(&session,&mut data.$field,&permit,None).await?;)*};}
    lctx_model::callable_aspect_inputs!(read);
    drop(session);reader.close().await.map_err(ModelError::codec)?;
    let budget=runtime.budget().clone();let rows=tokio::task::spawn_blocking(move ||callable_aspects::normalize(&data,&budget)).await.map_err(ModelError::codec)??;
    let mut output=StageOutput::new(access,attempt,model,runtime.budget().clone(),Default::default())?;
    macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$field.iter() {output.push(row.clone()).await?;})*};}
    lctx_model::callable_aspect_outputs!(write);drop(rows);output.finish(ProviderOutcome::Complete).await
}
