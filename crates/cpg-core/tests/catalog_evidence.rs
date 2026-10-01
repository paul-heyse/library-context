//! Native C1 producer through disposable PG18; deliberately no flow, brief, seed or vector stage.
#[path="fixtures/catalog_runtime.rs"] mod catalog_runtime;
use cpg_core::model_runtime::{AttemptRuntime,RuntimeOptions};
use cpg_extract::{acquisition::AcquiredInput,bundle::{CapturedInputs,run_stage},capture::CapturedInput};
use lctx_model::domain::{*,admission::FrontierContract,stages::*,catalog::build,normalized::{entity_normalization,relation_normalization,callable_normalization,binding_normalization,event_normalization}};
use lctx_postgres::{generations::GenerationStore,roles::{Role,RoleConfig},testing::DisposableDatabase};
use std::sync::Arc;
#[tokio::test]
async fn contextual_catalog_preserves_original_roots_and_exact_completed_receipts() {
    run(Profile::Catalog).await;
}
#[tokio::test]
async fn behavioral_contextual_catalog_retains_earlier_locations_without_heap_state(){run(Profile::Behavioral).await;}
async fn run(profile:Profile) {
    let runtime=AttemptRuntime::new(RuntimeOptions {memory_bytes:1<<30,partitions:2}).unwrap();let budget=runtime.budget();
    let db=DisposableDatabase::start().await;db.migrate().await;
    let config=RoleConfig {format:1,role:Role::Importer,url:db.url("lctx_importer"),max_connections:6,provider_connections:4,acquire_timeout_seconds:5,statement_timeout_seconds:60,lock_timeout_seconds:10};
    let mut relations=normalized_relations();
    relations.extend(analysis::early_relations());relations.extend(catalog_runtime::relations());
    
    relations.extend(analysis::catalog_core::relations());
    relations.extend(catalog::relations());
    relations.extend(analysis::catalog_evidence::relations());
    relations.sort_by_key(Relation::name);relations.dedup_by_key(|r|r.name());
    let model=Arc::new(ValidatedModel::validate(relations).unwrap());let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/catalog_context");
    let captured=Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(CapturedInput::capture_derived(&root,&["api.py".into(),"guide.mdx".into()],budget,&["guide.mdx".into()],cpg_extract::acquisition::derive_blocks).unwrap(),"C0") ],cpg_extract::native_context::NativeContextConfig::committed(profile,budget).unwrap()));
    let configuration=analysis::preparation::Configuration::new(captured.config().catalog(),catalog_runtime::definitions().into_iter().chain([build::definition(),catalog::evidence::build::definition()]),budget).unwrap();
    let mut providers=cpg_core::facts::providers(ContentHash::of(b"C0-native-fixture"));
    let mut declarations:Vec<_>=providers.iter().map(|p|p.declaration(profile)).collect();
    declarations.extend([entity_normalization::stage(),relation_normalization::stage(profile),callable_normalization::stage(profile),normalized::receiver::stage(profile),event_normalization::stage(profile),binding_normalization::stage(profile),projection::normalization::stage(profile),normalized::coverage::stage(profile),configuration.declaration(),analysis::preparation::native_stage(profile),normalized::callable_aspects::stage(profile),build::stage(profile),catalog::evidence::build::stage(profile,&model).unwrap()]);
    declarations.extend(catalog_runtime::stages(profile,&model));let schedule=catalog_runtime::schedule(&model,declarations,profile);
    assert!(!schedule.stages().iter().any(|s|s.name.contains("synth") || s.name.contains("embed")));assert_eq!(schedule.stages().iter().any(|s|s.name==cpg_extract::ty_flow::TY_FLOW),profile==Profile::Behavioral);
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
        } else if matches!(declaration.name,"analyze_local"|"evaluate_base"|"complete_base"|"prepare_source_calls") {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|catalog_runtime::run(declaration.name,access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap_or_else(|e|panic!("parent {} failed: {e}",declaration.name));
        } else if declaration.name=="catalog_evidence" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::catalog_evidence::produce(access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else {
            let position=providers.iter().position(|p|p.declaration(profile).name==declaration.name).unwrap();
            run_stage(providers.swap_remove(position),execution.begin(declaration.name).unwrap(),&attempt,&model,&captured,budget,Default::default()).await.unwrap();
        }
    }
    let validated=attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap();
    let s=id.schema();
    let runtime_links:Vec<(i16,i16,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT phase,applicability,state FROM {s}.catalog_field_location_links"))).fetch_all(db.owner.pool()).await.unwrap();if profile==Profile::Behavioral {let lower:Vec<(i16,i64)>=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT reason,count(*) FROM {s}.local_field_location_assessments GROUP BY reason"))).fetch_all(db.owner.pool()).await.unwrap();let candidates:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.local_field_location_candidates"))).fetch_one(db.owner.pool()).await.unwrap();assert!(!runtime_links.is_empty(),"native Local field location must reach C1; lower reasons={lower:?}, candidates={candidates}");assert!(runtime_links.iter().all(|r|r.0==0&&r.1==1&&r.2==58));}else{assert!(runtime_links.is_empty());}
    if profile==Profile::Behavioral{let constructor_links:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_constructor_candidate_links"))).fetch_one(db.owner.pool()).await.unwrap();let headers:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.source_call_headers"))).fetch_one(db.owner.pool()).await.unwrap();let boundaries:Vec<(i16,i64)>=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT reason,count(*) FROM {s}.source_call_boundaries GROUP BY reason"))).fetch_all(db.owner.pool()).await.unwrap();eprintln!("SourceCall boundaries={boundaries:?}; headers={headers}");assert!(constructor_links>0,"actual normalized constructor candidate reaches C1; SourceCall boundaries={boundaries:?}; headers={headers}");let changed_slots:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_constructor_candidate_links l JOIN {s}.catalog_scenario_associations a ON a.id=l.association JOIN {s}.catalog_constructors c ON c.id=l.constructor JOIN {s}.catalog_classes cl ON cl.id=c.class JOIN {s}.catalog_callables ca ON ca.id=c.callable WHERE a.member<>cl.member OR a.member<>ca.member OR l.applicability<>1 OR l.state<>58"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(changed_slots,0);}
    let parent_counts:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_evidence_invocation_sources WHERE kind IN (5,6)"))).fetch_one(db.owner.pool()).await.unwrap();assert!(parent_counts>0);
    let original_chunks:Vec<Vec<u8>>=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT c.body FROM {s}.artifact_chunks c JOIN {s}.source_artifacts a ON a.id=c.artifact WHERE a.path='guide.mdx' ORDER BY c.ordinal"))).fetch_all(db.owner.pool()).await.unwrap();assert_eq!(original_chunks.concat(),std::fs::read(root.join("guide.mdx")).unwrap());
    let scenarios:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_scenarios"))).fetch_one(db.owner.pool()).await.unwrap();assert!(scenarios>=5);
    let executed:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_scenarios WHERE execution<>2"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(executed,0);
    let originals:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_original_sources WHERE kind=2"))).fetch_one(db.owner.pool()).await.unwrap();assert!(originals>=2);
    let extracted:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_scenario_spans WHERE role=3"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(extracted,1);
    let associations:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_scenario_associations"))).fetch_one(db.owner.pool()).await.unwrap();assert!(associations>0);
    let fields:Vec<(String,i16,i16)>=sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT convert_from(f.name,'UTF8'),a.basis,a.applicability FROM {s}.catalog_field_access_assessments a JOIN {s}.field_entities f ON f.id=a.field"))).fetch_all(db.owner.pool()).await.unwrap();assert!(fields.iter().any(|r|r.0=="left"));assert!(fields.iter().any(|r|r.0=="right"));assert!(fields.iter().all(|r|r.1==4 && r.2==1));
    let access_spans:Vec<(String,i64,i64)>=sqlx::query_as(sqlx::AssertSqlSafe(format!(r#"SELECT convert_from(f.name,'UTF8'),o.start,o."end" FROM {s}.catalog_field_access_assessments a JOIN {s}.field_entities f ON f.id=a.field JOIN {s}.occurrences o ON o.id=a.occurrence"#))).fetch_all(db.owner.pool()).await.unwrap();let api=std::fs::read_to_string(root.join("api.py")).unwrap();for (name,start,end) in access_spans {assert_eq!(&api[start as usize..end as usize],format!("self.{name}"));}
    let roots:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_evidence_roots"))).fetch_one(db.owner.pool()).await.unwrap();assert!(roots>scenarios);
    let links:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_evidence_invocations"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(links,roots);
    let mismatches:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.catalog_evidence_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows"))).bind(id.hex()).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(mismatches,0);
    validated.abort().await.unwrap();drop(configuration);drop(captured);assert_eq!(budget.reserved(),0);
}
