//! Actual store-selected effect authorization with service-free deterministic contract values.
//! The provider is a fixture; no test here qualifies a live embedding service.
use cpg_core::{analysis_prepare,embedding_service::{Embedder,FakeEmbedder},embedding_realization,model_runtime::{AttemptRuntime,RuntimeOptions}};
use lctx_model::domain::{*,embedding::{EmbeddingSpec,configuration::{Configuration,ServiceConfiguration}},stages::*};
use lctx_postgres::{generations::GenerationStore,roles::{Role,RoleConfig},testing::DisposableDatabase};
use std::sync::Arc;

#[derive(Debug,Clone,PartialEq,Eq,lctx_model::Domain)]
#[model(name="embedding_effect_probes",semantic_source=include_bytes!("embedding_configuration.rs"))]
struct EffectProbe {#[model(key)] selected:bool}
#[derive(Debug,Clone,PartialEq,Eq,lctx_model::Domain)]
#[model(name="embedding_denied_probes",semantic_source=include_bytes!("embedding_configuration.rs"))]
struct DeniedProbe {#[model(key)] selected:bool}

#[tokio::test]
async fn stored_selection_is_early_immutable_and_required_for_embedding_effects() {
    let runtime=AttemptRuntime::new(RuntimeOptions {memory_bytes:1<<24,partitions:2}).unwrap();let budget=runtime.budget();
    let db=DisposableDatabase::start().await;db.migrate().await;
    let roles=RoleConfig {format:1,role:Role::Importer,url:db.url("lctx_importer"),max_connections:4,provider_connections:2,acquire_timeout_seconds:5,statement_timeout_seconds:60,lock_timeout_seconds:10};
    let model=Arc::new(ValidatedModel::validate(embedding::relations().into_iter().chain([Relation::of::<EffectProbe>(),Relation::of::<DeniedProbe>()]).collect()).unwrap());let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let provider=FakeEmbedder::new();let selection=Configuration::new(provider.spec(),provider.endpoint(),budget).unwrap();
    for selected in [Some(&selection),None] {
        let stage=|name,effect,output|Stage {name,inputs:vec![RelationUse::stored::<EmbeddingSpec>(),RelationUse::stored::<ServiceConfiguration>()],outputs:vec![output],contributes:vec![],coverage:vec![],provider:None,profiles:Profile::ALL.to_vec(),effect,code:ContentHash::of(b"embedding effect contract control"),configuration:ContentHash::of(b"selected")};
        let schedule=Schedule::build(&model,vec![embedding::configuration::stage(selected),stage("embedding_consumer",Effect::Embedding,RelationUse::of::<EffectProbe>()),stage("pure_consumer",Effect::Pure,RelationUse::of::<DeniedProbe>())],&[],Profile::Catalog).unwrap();
        let mut execution=schedule.execute();let attempt=store.begin_conformance(db.writer.clone(),&mut execution,budget.clone()).await.unwrap();
        analysis_prepare::embedding_configuration(execution.begin("embedding_configuration").unwrap(),&attempt,&model,&runtime,selected).await.unwrap();
        let access=execution.begin("pure_consumer").unwrap();assert!(embedding_realization::Session::open(&access,&attempt,&roles,&runtime,&model,&provider,None).await.is_err());
        let mut output=StageOutput::new(access,&attempt,&model,budget.clone(),Default::default()).unwrap();output.declare::<DeniedProbe>().unwrap();output.push(DeniedProbe {selected:selected.is_some()}).await.unwrap();output.finish(ProviderOutcome::Complete).await.unwrap();
        let access=execution.begin("embedding_consumer").unwrap();
        let result=embedding_realization::Session::open(&access,&attempt,&roles,&runtime,&model,&provider,None).await;
        if selected.is_some() {
            let mut session=result.unwrap();let spec=session.configuration().specification().clone();let value=session.realize("source declaration").await.unwrap();
            let replay=value.decode(&spec,budget).unwrap();assert_eq!(replay.values().len(),spec.dimensions as usize);drop(replay);drop(session);
        }else {assert!(result.is_err(),"no selected specification is explicit unavailability");}
        let mut output=StageOutput::new(access,&attempt,&model,budget.clone(),Default::default()).unwrap();output.declare::<EffectProbe>().unwrap();output.push(EffectProbe {selected:selected.is_some()}).await.unwrap();output.finish(ProviderOutcome::Complete).await.unwrap();
        let frozen=attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap();frozen.abort().await.unwrap();
    }
    drop(selection);assert_eq!(budget.reserved(),0);
}
