//! One nominal Summary publication over immutable predecessors and borrowed stored topology.
use crate::{analysis_graphs::PreparedGraphs,generation_read::{AttemptSession,ProviderOptions},model_runtime::AttemptRuntime};
use futures::TryStreamExt;
use lctx_model::domain::{*,analysis::{self,summary as owner,sources::CapturedSources,expected::CoverageAdmission},execution::{summary_production::*,summary_replay},stages::*};
use lctx_postgres::{generations::GenerationAttempt,roles::RoleConfig};
use std::{collections::BTreeSet,sync::Arc};
async fn load<R:Record>(access:&StageAccess<'_,'_>,reader:&AttemptSession,runtime:&AttemptRuntime,epoch:Option<PublicationBoundary>,mut visit:impl FnMut(&ReadPermit<'_,R>,&arrow_array::RecordBatch)->Result<(),ModelError>)->Result<(),ModelError>{
 if access.profile()==Profile::Catalog&&!access.stage().inputs.iter().any(|i|i.name()==R::NAME){return Ok(());}
 let permit=match epoch{Some(e)=>access.read_at_epoch::<R>(e)?,None=>access.read::<R>()?};let session=runtime.session(access);session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;let mut stream=session.query(&format!("SELECT * FROM \"{}\"",R::NAME)).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{visit(&permit,&batch)?;}Ok(())
}
pub async fn produce(access:StageAccess<'_,'_>,attempt:&GenerationAttempt,roles:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>,definition:&analysis::AnalysisDefinition,graphs:&PreparedGraphs)->Result<(),ModelError>{
 let budget=runtime.budget();let sources=CapturedSources::capture(&access,budget)?;let mut coverage=CoverageAdmission::new(&sources,budget)?;let reader=AttemptSession::open(roles,attempt,&access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;let mut data=SummaryData::new(budget);let mut seen=charged::ChargedSet::default();let mut charge=charged::StateCharge::new(budget,"summary-input-registration");
 macro_rules! inputs{($($f:ident:$t:ty,)*)=>{$(for input in SummaryData::inputs().iter().filter(|i|i.name()==<$t>::NAME){if seen.insert(&mut charge,(input.name(),input.prefix().map(|e|e as u8)))?{load::<$t>(&access,&reader,runtime,input.prefix(),|_,batch|data.visit_input(input,batch)).await?;}})*};}
 lctx_model::normalized_binding_inputs!(inputs);lctx_model::normalized_binding_outputs!(inputs);lctx_model::entry_value_inputs!(inputs);lctx_model::summary_path_inputs!(inputs);lctx_model::summary_owned_inputs!(inputs);lctx_model::summary_vocabulary!(inputs);
 macro_rules! extra{($($t:ty),*)=>{$(let input=ValidationInput::of::<$t>(&["id"]);if seen.insert(&mut charge,(input.name(),input.prefix().map(|e|e as u8)))?{load::<$t>(&access,&reader,runtime,None,|_,batch|data.visit_input(&input,batch)).await?;})*};}
 extra!(analysis::local::SupportSource,analysis::local::AnalysisDerivation,analysis::local::AnalysisProposition,analysis::local::AnalysisInvocation,analysis::model::SupportSource,analysis::model::AnalysisDerivation,analysis::model::AnalysisProposition,analysis::model::AnalysisInvocation,local_theory::TheoryWitness,execution::model_production::ModelApplication,execution::model_transfer::ModelTransferWitness,execution::model_context_transfer::ContextTransferWitness,projection::ProjectionSourceAssessment,projection::ProjectionSnapshot,projection::ProjectionSnapshotChunk);
 macro_rules! expected{($($t:ty),*)=>{$(load::<$t>(&access,&reader,runtime,None,|permit,batch|coverage.visit(permit,batch)).await?;)*};}
 expected!(input::InputRevision,source::SourceArtifact,input::ArtifactUse,source::CoverageScope,normalized::coverage::NormalizationComputation,normalized::coverage::NormalizationCoverage,attribution::ProviderCoverage);
 reader.close().await.map_err(ModelError::codec)?;
 let mut records=Vec::new();let mut frames=charged::ChargedSet::default();let mut frame_charge=charged::StateCharge::new(budget,"summary-output-frames");
 for run in data.entry.runs.iter(){if !frames.insert(&mut frame_charge,(run.input,run.context))?{continue;}let parents=summary_replay::parents(&data,run.input,run.context,budget)?;let key=projection::normalization::ProjectionKey{input:run.input,context:run.context,name:projection::ProjectionName::CallableInvocation};let graph=graphs.graph(&access,budget,key)?;
 let(invocation,inputs,receipts,projections)=owner::AnalysisInvocation::admitted(run.input,run.context,definition.id(),None,parents.iter().map(Record::id),&sources,[analysis::ProjectionDefinition::builtin(projection::ProjectionName::CallableInvocation).id()],budget)?;
 let admitted=owner::coverage::admit(&invocation,definition,analysis::AnalysisCapability::Summaries,&coverage,budget)?;let result=execution::summary_production::produce(&data,&invocation,definition,access.profile(),graph,budget)?;frame_charge.grow(size_of::<(owner::AnalysisInvocation,SummaryRecords)>()+4096+receipts.len().saturating_mul(512))?;records.push((invocation,parents,inputs,receipts,projections,admitted,result));
 }
 let mut output=StageOutput::new(access,attempt,model,budget.clone(),Default::default())?;
 macro_rules! declare{($($t:ty),*)=>{$(output.declare::<$t>()?;)*};}
 declare!(owner::AnalysisInvocation,owner::AnalysisInput,owner::SourceReceipt,owner::ProjectionInput,owner::InvocationSource,owner::AnalysisOutcome,owner::AnalysisCoverage,owner::CoverageRequirement,owner::CoverageRequiredSource,owner::AnalysisCoveragePremise,owner::CoverageSource);
 macro_rules! declarations{($($f:ident:$t:ty,)*)=>{$(output.declare::<$t>()?;)*};}lctx_model::summary_outputs!(declarations);lctx_model::summary_vocabulary!(declarations);
 for(invocation,parents,inputs,receipts,projections,coverage,mut result)in records{
  let mut actual_coverage=normalized::Rows::new(budget);let mut actual_premises=normalized::Rows::new(budget);
  for scope in coverage.scopes(){let(row,premises)=owner::coverage::assess(scope.expectation(),scope.observations(),result.outcome.status,result.outcome.reason,budget)?;actual_coverage.insert(row)?;for row in premises{actual_premises.insert(row)?;}}
  result.discharge(&actual_coverage)?;
  macro_rules! write{($($f:ident:$t:ty,)*)=>{$(for row in result.$f.iter(){output.push(row.clone()).await?;})*};}lctx_model::summary_outputs!(write);
  macro_rules! vocabulary{($($f:ident:$t:ty,)*)=>{$(for row in result.vocabulary.$f.values(){output.push(row.clone()).await?;})*};}lctx_model::summary_vocabulary!(vocabulary);
  for scope in coverage.scopes(){let(requirement,required)=scope.expectation().records()?;output.push(requirement).await?;for row in required{output.push(row).await?;}for row in scope.observations(){output.push(row.source().clone()).await?;}}
  for row in actual_coverage.iter(){output.push(row.clone()).await?;}for row in actual_premises.iter(){output.push(row.clone()).await?;}
  output.push(result.outcome).await?;output.push(invocation).await?;for row in parents.iter(){output.push(row.clone()).await?;}for row in inputs{output.push(row).await?;}for row in receipts{output.push(row).await?;}for row in projections{output.push(row).await?;}
 }
 drop(data);output.finish(ProviderOutcome::Complete).await
}
