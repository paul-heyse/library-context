//! Native C0 producer through disposable PG18; deliberately no flow, brief, seed or vector stage.
use cpg_core::model_runtime::{AttemptRuntime,RuntimeOptions};
use cpg_extract::{acquisition::AcquiredInput,bundle::{CapturedInputs,run_stage},capture::CapturedInput};
use lctx_model::domain::{*,admission::FrontierContract,stages::*,catalog::build,normalized::{entity_normalization,relation_normalization,callable_normalization,binding_normalization,event_normalization}};
use lctx_postgres::{generations::GenerationStore,roles::{Role,RoleConfig},testing::DisposableDatabase};
use std::sync::Arc;
#[tokio::test]
async fn mandatory_catalog_uses_completed_normalized_contracts_and_exact_receipts() {
    let profile=Profile::Catalog;
    let runtime=AttemptRuntime::new(RuntimeOptions {memory_bytes:1<<30,partitions:2}).unwrap();let budget=runtime.budget();
    let db=DisposableDatabase::start().await;db.migrate().await;
    let config=RoleConfig {format:1,role:Role::Importer,url:db.url("lctx_importer"),max_connections:6,provider_connections:4,acquire_timeout_seconds:5,statement_timeout_seconds:60,lock_timeout_seconds:10};
    let mut relations=normalized_relations();
    relations.extend(analysis::early_relations());
    
    relations.extend(analysis::catalog_core::relations());
    relations.extend(catalog::core_relations());
    relations.sort_by_key(Relation::name);relations.dedup_by_key(|r|r.name());
    let model=Arc::new(ValidatedModel::validate(relations).unwrap());let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/catalog_core");
    let captured=Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(CapturedInput::capture(&root,&["api.py".into()],budget).unwrap(),"C0") ],cpg_extract::native_context::NativeContextConfig::committed(profile,budget).unwrap()));
    let configuration=analysis::preparation::Configuration::new(captured.config().catalog(),[build::definition()],budget).unwrap();
    let mut providers=cpg_core::facts::providers(ContentHash::of(b"C0-native-fixture"));
    let mut declarations:Vec<_>=providers.iter().map(|p|p.declaration(profile)).collect();
    declarations.extend([entity_normalization::stage(),relation_normalization::stage(profile),callable_normalization::stage(profile),normalized::receiver::stage(profile),event_normalization::stage(profile),binding_normalization::stage(profile),projection::normalization::stage(profile),normalized::coverage::stage(profile),configuration.declaration(),analysis::preparation::native_stage(profile),normalized::callable_aspects::stage(profile),build::stage(profile)]);
    let schedule=Schedule::build(&model,declarations,&[],profile).unwrap();
    assert!(!schedule.stages().iter().any(|s|s.name=="flow" || s.name.contains("synth") || s.name.contains("embed")));
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
        } else if declaration.name=="catalog_core" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::catalog_core::produce(access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else {
            let position=providers.iter().position(|p|p.declaration(profile).name==declaration.name).unwrap();
            run_stage(providers.swap_remove(position),execution.begin(declaration.name).unwrap(),&attempt,&model,&captured,budget,Default::default()).await.unwrap();
        }
    }
    let validated=attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap();
    let names:Vec<String>=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT name FROM {}.catalog_members ORDER BY name",id.schema()))).fetch_all(db.owner.pool()).await.unwrap();
    for name in ["choose","alias","wrapped","Base","Child","Config","ChildConfig","Config.read_left","Accessors.value"] {assert!(names.iter().any(|n|n==name),"public slot {name} absent: {names:?}");}
    let aliases:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(DISTINCT c.member) FROM {s}.catalog_callables c JOIN {s}.catalog_members m ON m.id=c.member WHERE m.name IN ('choose','alias')",s=id.schema()))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(aliases,2);
    let alias_basis:Vec<i16>=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT c.basis FROM {s}.catalog_callables c JOIN {s}.catalog_members m ON m.id=c.member WHERE m.name='alias'",s=id.schema()))).fetch_all(db.owner.pool()).await.unwrap();assert!(!alias_basis.is_empty());assert!(alias_basis.iter().all(|v|*v==1));
    let none:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_options o JOIN {s}.catalog_members m ON m.id=o.member JOIN {s}.catalog_defaults d ON d.id=o.\"default\" JOIN {s}.literal_values l ON l.id=d.literal_literal WHERE m.name='wrapped' AND d.kind=3 AND l.kind=0",s=id.schema()))).fetch_one(db.owner.pool()).await.unwrap();assert!(none>0);
    let inherited:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_constructors k JOIN {s}.catalog_classes c ON c.id=k.class JOIN {s}.catalog_members m ON m.id=c.member WHERE m.name='Child' AND k.origin=1 AND k.ancestry IS NOT NULL",s=id.schema()))).fetch_one(db.owner.pool()).await.unwrap();assert!(inherited>0);
    let synthetic:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_constructors k JOIN {s}.catalog_classes c ON c.id=k.class JOIN {s}.catalog_members m ON m.id=c.member WHERE m.name='Config' AND k.origin=2",s=id.schema()))).fetch_one(db.owner.pool()).await.unwrap();assert!(synthetic>0);
    let copied_signatures:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_invocations c JOIN {s}.signature_variants v ON v.id=c.variant JOIN {s}.signature_observations r ON r.id=v.signature JOIN {s}.catalog_callables a ON a.id=c.callable JOIN {s}.catalog_members m ON m.id=a.member WHERE m.name='choose'",s=id.schema()))).fetch_one(db.owner.pool()).await.unwrap();assert!(copied_signatures>=2);
    let mismatches:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_core_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows",s=id.schema()))).bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(mismatches,0);
    let metadata:Vec<i16>=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT DISTINCT a.kind FROM {s}.catalog_callable_aspects c JOIN {s}.callable_aspects a ON a.id=c.aspect",s=id.schema()))).fetch_all(db.owner.pool()).await.unwrap();
    for kind in [1,2,3,5,7] {assert!(metadata.contains(&kind),"normalized metadata kind {kind} absent: {metadata:?}");}
    let factory:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_options o JOIN {s}.catalog_members m ON m.id=o.member JOIN {s}.catalog_defaults d ON d.id=o.\"default\" JOIN {s}.catalog_option_subjects u ON u.id=o.subject JOIN {s}.field_entities f ON f.id=u.field_field WHERE m.name='Config' AND f.name='cache' AND d.kind=5",s=id.schema()))).fetch_one(db.owner.pool()).await.unwrap();assert!(factory>0);
    let flow:Vec<i16>=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT c.availability FROM {s}.normalization_coverage c JOIN {s}.normalization_computations n ON n.id=c.computation WHERE n.capability=10",s=id.schema()))).fetch_all(db.owner.pool()).await.unwrap();assert!(!flow.is_empty());assert!(flow.iter().all(|v|*v==3));
    validated.abort().await.unwrap();drop(configuration);drop(captured);assert_eq!(budget.reserved(),0);
}
