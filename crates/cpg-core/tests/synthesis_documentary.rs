//! Scoped S0 documentary helper qualification with actual native owners and disposable PG18.
use cpg_core::model_runtime::{AttemptRuntime,RuntimeOptions};
use cpg_extract::{acquisition::AcquiredInput,bundle::{CapturedInputs,run_stage},capture::CapturedInput};
use lctx_model::domain::{*,admission::FrontierContract,stages::*,catalog::build,normalized::{entity_normalization,relation_normalization,callable_normalization,binding_normalization,event_normalization}};
use lctx_postgres::{generations::GenerationStore,roles::{Role,RoleConfig},testing::DisposableDatabase};
use std::sync::Arc;
#[tokio::test]
async fn documentary_preparation_preserves_native_literal_spans_and_candidates() {
    let profile=Profile::Catalog;
    let runtime=AttemptRuntime::new(RuntimeOptions {memory_bytes:1<<30,partitions:2}).unwrap();let budget=runtime.budget();
    let db=DisposableDatabase::start().await;db.migrate().await;
    let config=RoleConfig {format:1,role:Role::Importer,url:db.url("lctx_importer"),max_connections:6,provider_connections:4,acquire_timeout_seconds:5,statement_timeout_seconds:60,lock_timeout_seconds:10};
    let mut relations=normalized_relations();
    relations.extend(analysis::early_relations());
    relations.extend(analysis::dispatch::relations());
    relations.extend(analysis::catalog_core::relations());
    relations.extend(catalog::relations());
    relations.extend(analysis::catalog_evidence::relations());
    relations.extend(synthesis::documentary::relations());
    relations.sort_by_key(Relation::name);relations.dedup_by_key(|r|r.name());
    let model=Arc::new(ValidatedModel::validate(relations).unwrap());let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/synthesis_sources");
    let captured=Arc::new(CapturedInputs::new(vec![AcquiredInput::tree(CapturedInput::capture_derived(&root,&["api.py".into(),"guide.mdx".into()],budget,&["guide.mdx".into()],cpg_extract::acquisition::derive_blocks).unwrap(),"C0") ],cpg_extract::native_context::NativeContextConfig::committed(profile,budget).unwrap()));
    let configuration=analysis::preparation::Configuration::new(captured.config().catalog(),[build::definition(),catalog::evidence::build::definition()],budget).unwrap();
    let mut providers=cpg_core::facts::providers(ContentHash::of(b"C0-native-fixture"));
    let mut declarations:Vec<_>=providers.iter().map(|p|p.declaration(profile)).collect();
    declarations.extend([entity_normalization::stage(),relation_normalization::stage(profile),callable_normalization::stage(profile),normalized::receiver::stage(profile),event_normalization::stage(profile),binding_normalization::stage(profile),projection::normalization::stage(profile),normalized::coverage::stage(profile),configuration.declaration(),analysis::preparation::native_stage(profile),normalized::callable_aspects::stage(profile),build::stage(profile),catalog::evidence::build::stage(profile),Stage {name:"documentary_preparation_control",inputs:documentary_inputs(profile),outputs:synthesis::documentary::relations().iter().map(RelationUse::of_relation).chain([RelationUse::of::<assertion::AssertionQualification>()]).collect(),contributes:vec![],coverage:vec![],provider:None,profiles:vec![profile],effect:Effect::Pure,code:ContentHash::of(b"native-mandatory-retrieval-control"),configuration:ContentHash::of(b"disabled-vectors-mandatory-renderer")}]);
    let facts=declarations.iter().filter(|s|s.name!="documentary_preparation_control"&&s.outputs.iter().any(|r|is_vocabulary(r.name()))).map(|s|s.name).collect();
    let schedule=Schedule::build_with_publications(&model,declarations,&[],profile,vec![PublicationGroup::new(PublicationBoundary::Facts,facts),PublicationGroup::new(PublicationBoundary::Synthesis,vec!["documentary_preparation_control"])]).unwrap();
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
        } else if declaration.name=="catalog_evidence" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|cpg_core::catalog_evidence::produce(access,&attempt,&config,&runtime,&model).await,&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else if declaration.name=="documentary_preparation_control" {
            cpg_core::stage_runtime::run_declared_stage(&mut execution,declaration,async |access|{let (data,rows)=cpg_core::synthesis_preparation::documentary(&access,&attempt,&config,&runtime,&model).await?;let blocks=cpg_core::synthesis_preparation::code_blocks(&access,&attempt,&config,&runtime,&model).await?;let mut setup=cpg_core::synthesis_preparation::source_setup(&access,&attempt,&config,&runtime,&model).await?;let mut mapped=0;for block in blocks.iter(){let context=data.qualifications.get(block.qualification).unwrap().context;let mapping=synthesis::source_code::admit_fence(&data,block,context,budget)?.map_err(|reason|ModelError::Invalid(format!("actual simple fence refused: {reason:?}")))?;assert_eq!(mapping.block(),block.id());assert_eq!(mapping.materialized(),block.materialized.unwrap());for occurrence in data.occurrences.iter().filter(|o|o.source==mapping.materialized()&&o.syntax_kind==source::SyntaxKind::StmtAssign){let closure=synthesis::source_setup::close(&data,&setup,None,&[occurrence.id()],context,budget)?.map_err(|reason|ModelError::Invalid(format!("actual imported setup refused: {reason:?}")))?;assert_eq!(closure.statements().len(),2,"exact imported Alias plus assignment form the smallest subset");assert!(closure.dependencies().iter().any(|d|matches!(d,synthesis::source_setup::Dependency::Import{..})));let imports=std::mem::replace(&mut setup.imports,normalized::Rows::new(budget));assert!(matches!(synthesis::source_setup::close(&data,&setup,None,&[occurrence.id()],context,budget)?,Err(synthesis::source_setup::Boundary::UnsupportedSetup)));setup.imports=imports;let original=mapping.occurrence(&data,occurrence,budget)?;assert!(data.artifacts.get(original.artifact).unwrap().path.ends_with("guide.mdx"));assert!(!data.artifacts.get(original.artifact).unwrap().path.contains("_lctx_blocks"));let mut relocated=occurrence.clone();relocated.start+=1;assert!(mapping.occurrence(&data,&relocated,budget).is_err());mapped+=1;}}assert!(mapped>0,"actual native Python statements map to original document spans");drop(blocks);drop(setup);let mut output=StageOutput::new(access,&attempt,&model,budget.clone(),Default::default())?;cpg_core::synthesis_preparation::publish_documentary(&mut output,&rows).await?;drop(rows);drop(data);output.finish(ProviderOutcome::Complete).await},&mut |_|{}).await.unwrap_or_else(|e|panic!("stage {} failed: {e}",declaration.name));
        } else {
            let position=providers.iter().position(|p|p.declaration(profile).name==declaration.name).unwrap();
            run_stage(providers.swap_remove(position),execution.begin(declaration.name).unwrap(),&attempt,&model,&captured,budget,Default::default()).await.unwrap();
        }
    }
    let validated=attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap();
    let s=id.schema();
    let count:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.synthesis_documentary_conclusions"))).fetch_one(db.owner.pool()).await.unwrap();assert!(count>=2,"original plus sourced alias remain independently qualified");
    let missing:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.synthesis_documentary_boundaries WHERE reason IN(1,3)"))).fetch_one(db.owner.pool()).await.unwrap();assert!(missing>=2,"no docstring and unsupported literal mapping stay explicit");
    let forged:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.synthesis_documentary_conclusions c JOIN {s}.assertion_qualifications q ON q.id=c.qualification WHERE q.modality<>1 OR q.approximation<>1"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(forged,0);
    let passages:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.synthesis_documentary_sources WHERE kind=1"))).fetch_one(db.owner.pool()).await.unwrap();assert!(passages>0,"actual C1 mention associations produce original documentary excerpts");
    let mismatched:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {s}.synthesis_documentary_sources d JOIN {s}.synthesis_documentary_conclusions c ON c.source=d.id JOIN {s}.assertion_qualifications native ON native.id=coalesce(d.literal_source_qualification,d.passage_source_qualification) JOIN {s}.assertion_qualifications derived ON derived.id=c.qualification WHERE native.scope=derived.scope OR native.context<>derived.context"))).fetch_one(db.owner.pool()).await.unwrap();assert_eq!(mismatched,0,"native artifact evidence and static release association remain distinct");
    validated.abort().await.unwrap();drop(configuration);drop(captured);assert_eq!(budget.reserved(),0);
}

fn documentary_inputs(profile:Profile)->Vec<RelationUse>{let parent=catalog::evidence::build::stage(profile);let mut uses=parent.inputs;uses.extend(parent.outputs.into_iter().map(|r|r.completed_store()));uses.extend(synthesis::documentary::Data::stage_inputs());uses.push(RelationUse::stored::<documents::CodeBlockObservation>().completed_store());macro_rules! setup{($($f:ident:$ty:ty,)*)=>{$(uses.push(RelationUse::stored::<$ty>().completed_store());)*};}lctx_model::synthesis_setup_inputs!(setup);uses.sort_by_key(|r|r.name());uses.dedup_by_key(|r|r.name());uses}
