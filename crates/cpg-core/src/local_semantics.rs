//! Local analysis uses confirmed inputs, one attempt budget and the domain's shared replay.
use crate::{generation_read::{AttemptSession,ProviderOptions},model_runtime::AttemptRuntime};
use futures::TryStreamExt;
use lctx_model::domain::{*,analysis::{self,local as publication,sources::CapturedSources,expected::CoverageAdmission},local_semantics::{self,LocalData},stages::*,obligation::ObligationKind};
use lctx_postgres::{generations::GenerationAttempt,roles::RoleConfig};
use std::sync::Arc;
pub async fn run(access:StageAccess<'_,'_>,attempt:&GenerationAttempt,config:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>,definition:&analysis::AnalysisDefinition)->Result<(),ModelError>{
 let profile=access.profile();let budget=runtime.budget();let sources=CapturedSources::capture(&access,budget)?;let mut admission=CoverageAdmission::new(&sources,budget)?;
 let reader=AttemptSession::open(config,attempt,&access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;let session=runtime.session(&access);let mut data=LocalData::new(budget);let mut registered=charged::ChargedSet::default();let mut registered_charge=charged::StateCharge::new(budget,"local_registered_inputs");
 macro_rules! load {($($field:ident:$ty:ty,)*)=>{$({let permit=access.read::<$ty>()?;if registered.insert(&mut registered_charge,permit.relation())?{session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;}let query=session.query(&format!("SELECT * FROM \"{}\"",<$ty>::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{data.visit(<$ty>::NAME,&batch)?;}})*};}if profile==Profile::Behavioral{lctx_model::entry_value_inputs!(load);lctx_model::local_semantic_inputs!(load);}else{
  macro_rules! common {()=>{load! {runs:attribution::ProviderRun,providers:attribution::Provider,}};}common!();
 }
 let mut inputs=lctx_model::domain::normalized::Rows::<input::InputRevision>::new(budget);
 macro_rules! expected {($($ty:ty),*)=>{$({let permit=access.read::<$ty>()?;if registered.insert(&mut registered_charge,permit.relation())?{session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;}let query=session.query(&format!("SELECT * FROM \"{}\"",<$ty>::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{admission.visit(&permit,&batch)?;if <$ty>::NAME==input::InputRevision::NAME{inputs.decode(&batch)?;}}})*};}
 expected!(input::InputRevision,source::SourceArtifact,input::ArtifactUse,source::CoverageScope,normalized::coverage::NormalizationComputation,normalized::coverage::NormalizationCoverage,attribution::ProviderCoverage);
 let permit=access.read::<analysis::AnalysisDefinition>()?;if registered.insert(&mut registered_charge,permit.relation())?{session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;}let query=session.query("SELECT * FROM analysis_definitions").await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;let mut definitions=normalized::Rows::<analysis::AnalysisDefinition>::new(budget);while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{definitions.decode(&batch)?;}drop(stream);if definitions.get(definition.id())!=Some(definition){return Err(ModelError::Invalid("Local selected definition is absent from confirmed configuration".into()));}drop(definitions);drop(session);reader.close().await.map_err(ModelError::codec)?;
 let mut output=StageOutput::new(access,attempt,model,budget.clone(),Default::default())?;macro_rules! declare_publication {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
 declare_publication!(publication::AnalysisInvocation,publication::AnalysisInput,publication::SourceReceipt,publication::ProjectionInput,publication::AnalysisOutcome,publication::AnalysisDiagnostic,publication::InvocationSource,publication::ObligationSource,publication::AnalysisObligation,publication::DischargeEvidence,publication::AnalysisCoverage,publication::CoverageRequirement,publication::CoverageRequiredSource,publication::AnalysisCoveragePremise,publication::CoverageSource);
 macro_rules! declare {($($field:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;)*};}lctx_model::local_semantic_outputs!(declare);
 let mut frames=charged::ChargedSet::default();let mut frame_charge=charged::StateCharge::new(budget,"local_invocation_frames");
 for run in data.entry.runs.iter().filter(|run|profile==Profile::Catalog||data.entry.providers.get(run.provider).is_some_and(|p|p.tool=="ty")) {
 if !frames.insert(&mut frame_charge,(run.input,run.context))?{continue;}
 if inputs.get(run.input).is_none(){return Err(ModelError::Invalid("Local flow input absent".into()));}
 let (invocation,parents,receipts,projections)=publication::AnalysisInvocation::admitted(run.input,run.context,definition.id(),None,[],&sources,[],budget)?;
 let coverage=publication::coverage::admit(&invocation,definition,analysis::AnalysisCapability::Transfers,&admission,budget)?;
 let rows=if profile==Profile::Behavioral{local_semantics::produce(&data,&invocation,definition,budget)?}else{local_semantics::LocalRecords::new(budget)};
 macro_rules! write {($($field:ident:$ty:ty,)*)=>{$(for row in rows.$field.iter(){output.push(row.clone()).await?;})*};}lctx_model::local_semantic_outputs!(write);
 let outcome=publication::AnalysisOutcome{invocation:invocation.id(),status:if profile==Profile::Behavioral{analysis::AnalysisStatus::Partial}else{analysis::AnalysisStatus::NotRequested},reason:Some(if profile==Profile::Behavioral{ObligationKind::IncompleteDomain}else{ObligationKind::NotRequested})};
 for scope in coverage.scopes(){let (requirement,required)=scope.expectation().records()?;output.push(requirement).await?;for row in required{output.push(row).await?;}for row in scope.observations(){output.push(row.source().clone()).await?;}let (row,premises)=publication::coverage::assess(scope.expectation(),scope.observations(),outcome.status,outcome.reason,budget)?;output.push(row).await?;for row in premises{output.push(row).await?;}}
 output.push(invocation).await?;for row in parents{output.push(row).await?;}for row in receipts{output.push(row).await?;}for row in projections{output.push(row).await?;}output.push(outcome).await?;
 }
 output.finish(ProviderOutcome::Complete).await
}
