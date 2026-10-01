//! A0 persists C0-owned public candidates, canonical graph witnesses and exact official usage.
use crate::{analysis_graphs::PreparedGraphs,generation_read::{AttemptSession,ProviderOptions},model_runtime::{AttemptRuntime,StageSession}};
use futures::TryStreamExt;
use lctx_model::domain::{*,analysis::{self,structural as owner},structural::{self as semantic,build,frames},stages::*};
use lctx_postgres::{generations::GenerationAttempt,roles::RoleConfig};
use std::{sync::Arc,collections::BTreeSet};
async fn load<R:Record>(access:&StageAccess<'_, '_>,reader:&AttemptSession,session:&StageSession,data:&mut build::Data,context:&mut frames::Context,admission:&mut analysis::expected::CoverageAdmission<'_>,seen:&mut BTreeSet<&'static str>)->Result<(),ModelError>{
 if !seen.insert(R::NAME){return Ok(());}let permit=access.read::<R>()?;session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;
 let expected=[input::InputRevision::NAME,input::ArtifactUse::NAME,source::SourceArtifact::NAME,source::CoverageScope::NAME,attribution::ProviderCoverage::NAME,normalized::coverage::NormalizationComputation::NAME,normalized::coverage::NormalizationCoverage::NAME].contains(&R::NAME);
 let query=session.query(&format!("SELECT * FROM \"{}\"",R::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;
 while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{data.visit(R::NAME,&batch)?;context.visit(R::NAME,&batch)?;if expected{admission.visit(&permit,&batch)?;}}Ok(())
}
pub async fn produce(access:StageAccess<'_, '_>,attempt:&GenerationAttempt,config:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>,graphs:&PreparedGraphs)->Result<(),ModelError>{
 let sources=analysis::sources::CapturedSources::capture(&access,runtime.budget())?;let mut admission=analysis::expected::CoverageAdmission::new(&sources,runtime.budget())?;
 let reader=AttemptSession::open(config,attempt,&access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;let session=runtime.session(&access);let mut data=build::Data::new(runtime.budget());let mut context=frames::Context::new(runtime.budget());let mut seen=BTreeSet::new();
 macro_rules! read {($($ty:ty),*)=>{$(load::<$ty>(&access,&reader,&session,&mut data,&mut context,&mut admission,&mut seen).await?;)*};}
 macro_rules! inventory {($($f:ident:$ty:ty,)*)=>{read!($($ty),*);};}
 lctx_model::projection_inputs!(inventory);lctx_model::normalized_event_outputs!(inventory);
 // The runtime graph owner retains snapshots; this context needs assessment IDs only.
 read!(projection::ProjectionSourceAssessment,input::ArtifactUse,catalog::CatalogMember,catalog::CatalogCallable,catalog::CatalogMemberInvocation,analysis::catalog_core::Invocation,normalized::callables::EffectiveCallableAssessment,analysis::settings::AnalyticsConfiguration,analysis::AnalysisDefinition,analysis::MethodParameters,analysis::local::Invocation,input::InputRevision,normalized::coverage::NormalizationComputation,normalized::coverage::NormalizationCoverage);
 drop(session);reader.close().await.map_err(ModelError::codec)?;
 let settings=context.configuration()?.clone();let declaration=build::stage(access.profile(),&settings,model)?;
 if declaration.configuration!=access.stage().configuration||declaration.code!=access.stage().code||declaration.name!=access.stage().name{return Err(ModelError::Invalid("structural stage differs from selected settings/rules".into()));}
 let parents=frames::parents(&data,runtime.budget())?;let mut results=semantic::Output::new(runtime.budget());let mut receipts=normalized::Rows::new(runtime.budget());let mut projections=normalized::Rows::new(runtime.budget());
 for core in parents.iter(){
  let local=context.local_parent(core)?.id();
  for method in [analysis::AnalysisMethod::Delegation,analysis::AnalysisMethod::DirectUsage]{
   let (parameters,definition)=build::definition(&settings,method)?;
   if context.definitions.get(definition.id())!=Some(&definition)||context.parameters.get(parameters.id())!=Some(&parameters){return Err(ModelError::Invalid("structural canonical definition absent".into()));}
   let parents=[owner::InvocationSource::Local{invocation:local},owner::InvocationSource::CatalogCore{invocation:core.id()}];
   let mut projection_ids=Vec::new();if method==analysis::AnalysisMethod::Delegation{projection_ids.extend([projection::ProjectionName::CallableInvocation,projection::ProjectionName::DefinitionContainment].map(|name|analysis::ProjectionDefinition::builtin(name).id()));}
   let (invocation,inputs,source_rows,projection_rows)=owner::Invocation::admitted(core.input,core.context,definition.id(),None,parents.iter().map(Record::id),&sources,projection_ids,runtime.budget())?;
   for parent in parents{context.sources.insert(parent)?;}for row in inputs{context.inputs.insert(row)?;}for row in source_rows{receipts.insert(row)?;}for row in projection_rows{projections.insert(row)?;}context.invocations.insert(invocation)?;
  }
  let frame=frames::frame(&context,core)?;
  let key=|name|projection::normalization::ProjectionKey{input:core.input,context:core.context,name};
  let call=graphs.graph(&access,runtime.budget(),key(projection::ProjectionName::CallableInvocation))?;let definition=graphs.graph(&access,runtime.budget(),key(projection::ProjectionName::DefinitionContainment))?;
  results.extend(build::produce(&data,&frame,context.invocations.get(frame.invocation).unwrap(),&settings,call,definition,runtime.budget())?)?;
 }
 let mut output=StageOutput::new(access,attempt,model,runtime.budget().clone(),Default::default())?;
 macro_rules! write {($ty:ty,$rows:expr)=>{{output.declare::<$ty>()?;for row in $rows.iter(){output.push(row.clone()).await?;}}};}
 macro_rules! result {($($f:ident:$ty:ty,)*)=>{$(write!($ty,results.$f);)*};}lctx_model::structural_outputs!(result);
 write!(owner::Invocation,context.invocations);write!(owner::InvocationSource,context.sources);write!(owner::AnalysisInput,context.inputs);write!(owner::SourceReceipt,receipts);write!(owner::ProjectionInput,projections);
 macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}declare!(owner::AnalysisOutcome,owner::AnalysisCoverage,owner::CoverageSource,owner::AnalysisCoveragePremise,owner::CoverageRequirement,owner::CoverageRequiredSource);
 for invocation in context.invocations.iter(){let definition=context.definitions.get(invocation.definition).unwrap();let capability=if definition.method==analysis::AnalysisMethod::Delegation{analysis::AnalysisCapability::Delegation}else{analysis::AnalysisCapability::DirectUsage};
  let bounded=definition.method==analysis::AnalysisMethod::Delegation&&results.frames.iter().filter(|f|f.invocation==invocation.id()).any(|f|results.traversals.iter().any(|t|t.frame==f.id()&&t.stop.is_some()));
  let status=if bounded{analysis::AnalysisStatus::Partial}else{analysis::AnalysisStatus::Completed};let reason=bounded.then_some(obligation::ObligationKind::BudgetReached);
  let domain=owner::coverage::admit(invocation,definition,capability,&admission,runtime.budget())?;
  for scope in domain.scopes(){let (row,members)=scope.expectation().records()?;output.push(row).await?;for row in members{output.push(row).await?;}for row in scope.observations(){output.push(row.source().clone()).await?;}let (row,members)=owner::coverage::assess(scope.expectation(),scope.observations(),status,reason,runtime.budget())?;output.push(row).await?;for row in members{output.push(row).await?;}}
  output.push(owner::AnalysisOutcome{invocation:invocation.id(),status,reason}).await?;
 }
 drop(receipts);drop(projections);drop(parents);drop(results);drop(context);drop(data);drop(admission);drop(sources);output.finish(ProviderOutcome::Complete).await
}
