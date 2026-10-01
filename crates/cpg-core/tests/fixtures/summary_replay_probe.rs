//! Corrupt only a read view of the actual closed Summary generation; publication remains intact.
use cpg_core::{generation_read::{AttemptSession,ProviderOptions},model_runtime::AttemptRuntime};
use futures::TryStreamExt;
use lctx_model::domain::{*,analysis,execution::{summary_replay,summary_production::*},stages::*};
use lctx_postgres::{generations::GenerationAttempt,roles::RoleConfig};
use std::sync::Arc;
#[derive(Debug,Clone,PartialEq,Eq,lctx_model::Domain)]
#[model(name="test_summary_replay_receipts",semantic_source=include_bytes!("summary_replay_probe.rs"))]
pub struct ProbeReceipt{#[model(key)]pub profile:i64}
pub fn stage(model:&ValidatedModel,profile:Profile)->Stage{
 let mut inputs=std::collections::BTreeMap::new();for i in summary_replay::inputs_for_profile(profile){let relation=model.relations().iter().find(|r|r.name()==i.name()).unwrap();let r=RelationUse::of_relation(relation).completed_store();inputs.insert(i.name(),if is_vocabulary(i.name()){r.at_epoch(PublicationBoundary::Summary)}else{r});}
 // A completed read grants the full non-facts reference and validation closure.
 let facts=facts_relations().iter().map(Relation::name).collect::<std::collections::BTreeSet<_>>();
 let mut pending=inputs.keys().copied().collect::<Vec<_>>();
 while let Some(name)=pending.pop(){let relation=model.relations().iter().find(|r|r.name()==name).unwrap();for required in relation.fields().iter().filter_map(|f|f.target().map(|(_,n)|n)).chain(relation.invariants().iter().flat_map(|i|i.inputs.iter().map(ValidationInput::name))){if !facts.contains(required)&&!inputs.contains_key(required){let relation=model.relations().iter().find(|r|r.name()==required).unwrap();inputs.insert(required,RelationUse::of_relation(relation).completed_store());pending.push(required);}}}
 Stage{name:"summary_replay_probe",inputs:inputs.into_values().collect(),outputs:vec![RelationUse::of::<ProbeReceipt>()],contributes:vec![],coverage:vec![],provider:None,profiles:vec![profile],effect:Effect::Pure,code:ContentHash::of(include_bytes!("summary_replay_probe.rs")),configuration:ContentHash::of(b"actual-summary-controls")}
}
async fn capture<R:Record>(access:&StageAccess<'_,'_>,reader:&AttemptSession,runtime:&AttemptRuntime,input:&ValidationInput,batches:&mut Vec<(ValidationInput,arrow_array::RecordBatch)>,charge:&mut charged::StateCharge)->Result<(),ModelError>{
 let permit=match input.prefix(){Some(e)=>access.read_at_epoch::<R>(e)?,None=>access.read::<R>()?};let session=runtime.session(access);session.register(&permit,reader.table(&permit).map_err(ModelError::codec)?)?;let query=session.query(&format!("SELECT * FROM \"{}\"",R::NAME)).await.map_err(ModelError::codec)?;let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{charge.grow(batch.get_array_memory_size().saturating_add(256))?;batches.push((input.clone(),batch));}Ok(())
}
pub async fn run(access:StageAccess<'_,'_>,attempt:&GenerationAttempt,roles:&RoleConfig,runtime:&AttemptRuntime,model:&Arc<ValidatedModel>)->Result<(),ModelError>{
 let invariant=Relation::of::<SummaryRun>().invariants()[0].clone();let replay_inputs=summary_replay::inputs_for_profile(access.profile());let reader=AttemptSession::open(roles,attempt,&access,model.clone(),ProviderOptions::default()).await.map_err(ModelError::codec)?;let mut batches=Vec::new();let mut charge=charged::StateCharge::new(runtime.budget(),"summary-control-batches");let mut seen=std::collections::BTreeSet::new();
 macro_rules! read{($($t:ty),*)=>{$(for i in replay_inputs.iter().filter(|i|i.name()==<$t>::NAME){if seen.insert((i.name(),i.prefix().map(|e|e as u8))){capture::<$t>(&access,&reader,runtime,i,&mut batches,&mut charge).await?;}})*};}
 macro_rules! fields{($($f:ident:$t:ty,)*)=>{read!($($t),*);};}
 lctx_model::normalized_binding_inputs!(fields);lctx_model::normalized_binding_outputs!(fields);lctx_model::entry_value_inputs!(fields);lctx_model::summary_path_inputs!(fields);lctx_model::summary_owned_inputs!(fields);lctx_model::summary_vocabulary!(fields);lctx_model::summary_outputs!(fields);
 read!(analysis::local::SupportSource,analysis::local::AnalysisDerivation,analysis::local::AnalysisProposition,analysis::model::SupportSource,analysis::model::AnalysisDerivation,analysis::model::AnalysisProposition,local_theory::TheoryWitness,execution::model_production::ModelApplication,execution::model_transfer::ModelTransferWitness,execution::model_context_transfer::ContextTransferWitness,projection::ProjectionSourceAssessment,projection::ProjectionSnapshot,projection::ProjectionSnapshotChunk,analysis::summary::AnalysisInvocation,analysis::summary::AnalysisOutcome,analysis::summary::AnalysisCoverage);
 reader.close().await.map_err(ModelError::codec)?;
 assert!(replay_inputs.iter().all(|i|seen.contains(&(i.name(),i.prefix().map(|e|e as u8)))),"probe must read every declared replay input");
 let behavioral=access.profile()==Profile::Behavioral;
 for mutation in 0..=8 {if !behavioral&&mutation>1{continue;}let mut check=(invariant.create)(runtime.budget());let mut selected=batches.iter().collect::<Vec<_>>();if mutation==1{selected.reverse();}
  let mut changed=false;for(i,batch)in selected{let mut batch=batch.clone();
   if mutation==2&&i.name()==transfer::summary::SummaryWitness::NAME{batch=transfer::summary::SummaryWitness::encode(&[])?;changed=true;}
   if mutation==3&&i.name()==execution::summary_path::SummaryPathWitness::NAME{let mut rows=execution::summary_path::SummaryPathWitness::decode(&batch)?;if let Some(row)=rows.first_mut(){row.status=analysis::policy::EvidenceStatus::Documented;changed=true;}batch=execution::summary_path::SummaryPathWitness::encode(&rows)?;}
   if mutation==4&&i.name()==execution::summary_proof::SummaryProofCost::NAME{let mut rows=execution::summary_proof::SummaryProofCost::decode(&batch)?;if let Some(row)=rows.first_mut(){row.rank+=1;changed=true;}batch=execution::summary_proof::SummaryProofCost::encode(&rows)?;}
   if mutation==5&&i.name()==conditions::stability::GuardSubstitution::NAME{changed|=batch.num_rows()>0;batch=conditions::stability::GuardSubstitution::encode(&[])?;}
   if mutation==6&&(i.name()==SummaryRun::NAME||i.name()==CallMember::NAME){if i.name()==SummaryRun::NAME{let mut rows=SummaryRun::decode(&batch)?;for row in &mut rows{row.proofs=0;row.residuals=0;}batch=SummaryRun::encode(&rows)?;}else{batch=CallMember::encode(&[])?;changed=true;}}
   if mutation==7&&i.name()==execution::summary_consequences::ClaimConclusion::NAME{changed|=batch.num_rows()>0;batch=execution::summary_consequences::ClaimConclusion::encode(&[])?;}
   if mutation==8&&i.name()==analysis::summary::DischargeEvidence::NAME{changed|=batch.num_rows()>0;batch=analysis::summary::DischargeEvidence::encode(&[])?;}
   check.visit_input(i,&batch)?;
  }
  if mutation>1{assert!(changed,"Summary mutation {mutation} needs an actual positive target");}let result=check.finish();assert_eq!(result.is_ok(),mutation<2,"Summary whole-generation replay mutation {mutation}: {result:?}");eprintln!("Summary replay control {mutation} passed");
 }
 drop(batches);let profile=if behavioral{1}else{0};let mut output=StageOutput::new(access,attempt,model,runtime.budget().clone(),Default::default())?;output.declare::<ProbeReceipt>()?;output.push(ProbeReceipt{profile}).await?;output.finish(ProviderOutcome::Complete).await
}
