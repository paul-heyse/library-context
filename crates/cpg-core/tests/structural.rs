//! Actual structural publication over native C0/Local and borrowed normalized graphs.
use cpg_core::model_runtime::{AttemptRuntime,RuntimeOptions};
use cpg_extract::{acquisition::AcquiredInput,bundle::{CapturedInputs,run_stage},capture::CapturedInput};
use lctx_model::domain::{*,admission::FrontierContract,stages::*,catalog::build,normalized::{entity_normalization,relation_normalization,callable_normalization,binding_normalization,event_normalization}};
use lctx_postgres::{generations::GenerationStore,roles::{Role,RoleConfig},testing::DisposableDatabase};
use std::sync::Arc;
#[tokio::test]
async fn structural_candidates_paths_and_usage_publish_in_both_profiles() {
    for profile in Profile::ALL {
    let runtime=AttemptRuntime::new(RuntimeOptions {memory_bytes:1<<30,partitions:2}).unwrap();let budget=runtime.budget();
    let db=DisposableDatabase::start().await;db.migrate().await;
    let config=RoleConfig {format:1,role:Role::Importer,url:db.url("lctx_importer"),max_connections:6,provider_connections:4,acquire_timeout_seconds:5,statement_timeout_seconds:60,lock_timeout_seconds:10};
    let mut relations=normalized_relations();
    relations.extend(analysis::early_relations());
    relations.extend(analysis::dispatch::relations());
    relations.extend(analysis::catalog_core::relations());
    relations.extend(catalog::relations());
    relations.extend(analysis::catalog_evidence::relations());
    relations.extend(analysis::selection::relations());
    relations.extend(selection::relations());
    relations.extend(analysis::local::relations());relations.extend(transfer::local::relations());relations.extend(local_semantics::relations());relations.extend(local_theory::relations());relations.extend(local_fields::relations());
    macro_rules! declared {($($field:ident:$ty:ty,)*)=>{$(relations.push(Relation::of::<$ty>());)*};}lctx_model::local_semantic_outputs!(declared);
    relations.extend(analysis::structural::relations());relations.extend(structural::relations());
    relations.sort_by_key(Relation::name);relations.dedup_by_key(|r|r.name());
    let model=Arc::new(ValidatedModel::validate(relations).unwrap());let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/catalog_context");
    let captured=Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(CapturedInput::capture_derived(&root,&["api.py".into(),"guide.mdx".into()],budget,&["guide.mdx".into()],cpg_extract::acquisition::derive_blocks).unwrap(),"C0") ],cpg_extract::native_context::NativeContextConfig::committed(profile,budget).unwrap()));
    let settings=analysis::settings::AnalyticsConfiguration{module_prefixes:vec!["api".into()],public_roots:vec!["api".into()],configured_seeds:vec!["api.Client".into(),"api.missing".into()],depth:2,vertices:512,arcs:2048,witnesses:3,brief_budget:8,communities:false,pagerank:false,fca:false,rca:false,knn:false,type_layer:false,mention_layer:false,knn_layer:false};
    let (parameters,local)=lctx_model::domain::local_semantics::definition();
    let configuration=analysis::preparation::Configuration::new(captured.config().catalog(),[build::definition(),catalog::evidence::build::definition(),selection::build::definition(),(parameters,local.clone()),structural::build::definition(&settings,analysis::AnalysisMethod::Delegation).unwrap(),structural::build::definition(&settings,analysis::AnalysisMethod::DirectUsage).unwrap()],budget).unwrap().with_analytics(settings.clone()).unwrap();
    let mut providers=cpg_core::facts::providers(ContentHash::of(b"C0-native-fixture"));
    let mut declarations:Vec<_>=providers.iter().map(|p|p.declaration(profile)).collect();
    declarations.extend([entity_normalization::stage(),relation_normalization::stage(profile),callable_normalization::stage(profile),normalized::receiver::stage(profile),event_normalization::stage(profile),binding_normalization::stage(profile),projection::normalization::stage(profile),normalized::coverage::stage(profile),configuration.declaration(),analysis::preparation::native_stage(profile),normalized::callable_aspects::stage(profile),build::stage(profile),catalog::evidence::build::stage(profile),selection::build::stage(profile)]);
    declarations.extend([local_semantics::stage(profile,&local,&model),structural::build::stage(profile,&settings,&model).unwrap()]);
    let facts_members=declarations.iter().filter(|s|s.name!="analyze_local"&&s.name!="analyze_structural"&&s.outputs.iter().any(|r|is_vocabulary(r.name()))).map(|s|s.name).collect();
    let schedule=Schedule::build_with_publications(&model,declarations,&[],profile,vec![PublicationGroup::new(PublicationBoundary::Facts,facts_members),PublicationGroup::new(PublicationBoundary::Local,vec!["analyze_local"]),PublicationGroup::new(PublicationBoundary::Structural,vec!["analyze_structural"])]).unwrap();
    assert!(!schedule.stages().iter().any(|s|s.name.contains("synth") || s.name.contains("embed")));
    let mut execution=schedule.execute();let attempt=store.begin_conformance(db.writer.clone(),&mut execution,budget.clone()).await.unwrap();let id=attempt.generation();
    for declaration in schedule.stages() {
        let installation:bool=sqlx::query_scalar("SELECT model_digest=$1 AND physical_digest=$2 FROM lctx_model_store.installation WHERE singleton").bind(model.digest().0.to_vec()).bind(store.physical_digest().0.to_vec()).fetch_one(db.owner.pool()).await.unwrap();assert!(installation,"installation digest changed before {}",declaration.name);
        let normalization=match declaration.name {"normalize_entities"=>Some(0),"normalize_relations"=>Some(1),"normalize_callables"=>Some(2),"normalize_events"=>Some(3),"normalize_bindings"=>Some(4),"normalize_projections"=>Some(5),"normalize_coverage"=>Some(6),"normalize_receivers"=>Some(7),_=>None};
        if let Some(which)=normalization {
            if which==0 {attempt.checkpoint(&execution,&FrontierContract::facts(&model,profile).unwrap()).await.unwrap();}
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access| {match which {
                0=>cpg_core::normalize::entities(access,&attempt,&config,&runtime,&model).await,
                1=>cpg_core::normalize::relations(access,&attempt,&config,&runtime,&model).await,
                2=>cpg_core::normalize::callables(access,&attempt,&config,&runtime,&model).await,
                3=>cpg_core::normalize::events(access,&attempt,&config,&runtime,&model).await,
                4=>cpg_core::normalize::bindings(access,&attempt,&config,&runtime,&model).await,
                5=>cpg_core::normalize::projections(access,&attempt,&config,&runtime,&model).await,
                7=>cpg_core::normalize::receivers(access,&attempt,&config,&runtime,&model).await,
                _=>cpg_core::normalize::coverage(access,&attempt,&config,&runtime,&model).await,
            }},&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else if declaration.name=="analysis_configuration" {
            cpg_core::analysis_prepare::configuration(execution.begin(declaration.name).unwrap(),&attempt,&model,&runtime,&configuration).await.unwrap();
        } else if declaration.name=="analysis_native_inventory" {
            cpg_core::analysis_prepare::native_inventory(execution.begin(declaration.name).unwrap(),&attempt,&config,&runtime,&model).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else if declaration.name=="normalize_callable_aspects" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::catalog_core::aspects(access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap();
        } else if declaration.name=="analyze_local" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::local_semantics::run(access,&attempt,&config,&runtime,&model,&local).await,&mut |_|{}).await.unwrap();
        } else if declaration.name=="analyze_structural" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|{
                let graphs=cpg_core::analysis_graphs::PreparedGraphs::load(&access,&attempt,&config,&runtime,&model,&[projection::ProjectionName::CallableInvocation,projection::ProjectionName::DefinitionContainment].into_iter().collect()).await?;
                cpg_core::structural::produce(access,&attempt,&config,&runtime,&model,&graphs).await
            },&mut |_|{}).await.unwrap();
        } else if declaration.name=="catalog_core" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::catalog_core::produce(access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else if declaration.name=="catalog_evidence" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::catalog_evidence::produce(access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else if declaration.name=="catalog_selection" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::catalog_selection::produce(access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else {
            let position=providers.iter().position(|p|p.declaration(profile).name==declaration.name).unwrap();
            run_stage(providers.swap_remove(position),execution.begin(declaration.name).unwrap(),&attempt,&model,&captured,budget,Default::default()).await.unwrap();
        }
    }
    let validated=attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap();
    let s=id.schema();
    let frames:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_frames"))).fetch_one(db.owner.pool()).await.unwrap();assert!(frames>0);
    let invocations:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_analysis_invocations"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(invocations,frames*2);
    let public:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_public_candidates"))).fetch_one(db.owner.pool()).await.unwrap();assert!(public>0);
    let missing:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_configured_seeds WHERE path='api.missing' AND candidates=0"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(missing,frames);
    let mismatches:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.structural_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows"))).bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(mismatches,0);
    validated.abort().await.unwrap();drop(configuration);drop(captured);assert_eq!(budget.reserved(),0);
}
}
