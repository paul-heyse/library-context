#[path="../../lctx-model/tests/fixtures/validation_views.rs"] mod fixture;
use fixture::*;
use cpg_core::model_runtime::{AttemptRuntime,RuntimeOptions};
use lctx_model::domain::{*,stages::*,value::Literal};
use lctx_postgres::{generations::GenerationStore,testing::DisposableDatabase};
use std::sync::Arc;

#[derive(Debug,Clone,PartialEq,Eq,lctx_model::Domain)]
#[model(name="validation_future_probes",publication_checks=future_check,semantic_source=include_bytes!("validation_views.rs"))]
struct FutureProbe {#[model(key)] marker:bool}
fn future_check()->Vec<PublicationInvariant> {vec![PublicationInvariant {name:"future_view_refusal_control",inputs:vec![ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Dispatch)],create:Arc::new(|_|Box::new(Noop))}]}
struct Noop;
impl PublicationCheck for Noop {
    fn visit(&mut self,_:&str,_:&arrow_array::RecordBatch)->Result<(),ModelError> {Ok(())}
    fn finish(self:Box<Self>,_:&[CompletedRelation],_:Profile)->Result<(),ModelError> {Ok(())}
}

#[tokio::test]
async fn validation_cannot_widen_a_facts_grant_to_later_vocabulary_even_after_it_closes() {
    for close_later in [false,true] {
        let model=Arc::new(ValidatedModel::validate(vec![Relation::of::<Literal>(),Relation::of::<FutureProbe>()]).unwrap());
        let schedule=Schedule::build_with_publications(&model,vec![stage("facts",vec![],vec![RelationUse::of::<Literal>()]),stage("later",vec![],vec![RelationUse::of::<Literal>()]),stage("future",vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Facts)],vec![RelationUse::of::<FutureProbe>()])],&[],Profile::Catalog,vec![PublicationGroup::new(PublicationBoundary::Facts,vec!["facts"]),PublicationGroup::new(PublicationBoundary::Dispatch,vec!["later"])]).unwrap();
        let db=DisposableDatabase::start().await;let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();let runtime=AttemptRuntime::new(RuntimeOptions {memory_bytes:1<<24,partitions:1}).unwrap();let budget=runtime.budget();let mut execution=schedule.execute();let attempt=store.begin_conformance(db.writer.clone(),&mut execution,budget.clone()).await.unwrap();
        for (name,row) in [("facts",Literal::None),("later",Literal::Bool {value:true})] {if name=="later" && !close_later {continue;}let mut output=StageOutput::new(execution.begin(name).unwrap(),&attempt,&model,budget.clone(),Default::default()).unwrap();output.declare::<Literal>().unwrap();output.push(row).await.unwrap();output.finish(ProviderOutcome::Complete).await.unwrap();}
        let mut output=StageOutput::new(execution.begin("future").unwrap(),&attempt,&model,budget.clone(),Default::default()).unwrap();output.declare::<FutureProbe>().unwrap();output.push(FutureProbe {marker:true}).await.unwrap();assert!(output.finish(ProviderOutcome::Complete).await.is_err(),"later close cannot widen this relation's Facts grant");
        attempt.abort().await.unwrap();assert_eq!(budget.reserved(),0);
    }
}

#[tokio::test]
async fn real_publication_and_final_replay_route_facts_and_current_views_independently() {
    let model=Arc::new(ValidatedModel::validate(vec![Relation::of::<Literal>(),Relation::of::<Probe>(),Relation::of::<ReadProbe>()]).unwrap());let schedule=schedule(&model);
    let db=DisposableDatabase::start().await;let store=GenerationStore::install(db.owner.clone(),model.clone()).await.unwrap();
    let runtime=AttemptRuntime::new(RuntimeOptions {memory_bytes:1<<24,partitions:1}).unwrap();let budget=runtime.budget();let mut execution=schedule.execute();
    let attempt=store.begin_conformance(db.writer.clone(),&mut execution,budget.clone()).await.unwrap();
    for (name,row) in [("facts",Literal::None),("later",Literal::Bool {value:true})] {let mut output=StageOutput::new(execution.begin(name).unwrap(),&attempt,&model,budget.clone(),Default::default()).unwrap();output.declare::<Literal>().unwrap();output.push(row).await.unwrap();output.finish(ProviderOutcome::Complete).await.unwrap();}
    let mut output=StageOutput::new(execution.begin("probe").unwrap(),&attempt,&model,budget.clone(),Default::default()).unwrap();output.declare::<Probe>().unwrap();output.push(Probe {marker:true}).await.unwrap();output.finish(ProviderOutcome::Complete).await.unwrap();
    let directory=tempfile::tempdir().unwrap();db.write_configs(directory.path()).unwrap();let roles=lctx_postgres::roles::RoleConfig::load(&directory.path().join("postgres-importer.json")).unwrap();
    let access=execution.begin("reader").unwrap();let reader=cpg_core::generation_read::AttemptSession::open(&roles,&attempt,&access,model.clone(),Default::default()).await.unwrap();reader.close().await.unwrap();
    let mut output=StageOutput::new(access,&attempt,&model,budget.clone(),Default::default()).unwrap();output.declare::<ReadProbe>().unwrap();output.push(ReadProbe {marker:true}).await.unwrap();output.finish(ProviderOutcome::Complete).await.unwrap();
    let generation=attempt.seal(execution.finish().unwrap()).await.unwrap().validate().await.unwrap();generation.abort().await.unwrap();assert_eq!(budget.reserved(),0);
}
