//! Actual native grains bind compact keys before rich hydration, under the frozen view.
use cpg_core::consumed_rows::{ClosureTable,NominalClosure};
use datafusion::{prelude::SessionContext,common::stats::Precision};
use futures::TryStreamExt;
use lctx_model::domain::{ContentHash,Record,Relation,completed::ContributionSpec,input::{Package,Release},resources::{ResourceBudget,TRANSFER_ROWS},stages::{Profile,ProviderOutcome}};
use lctx_surrealdb::{RuntimeConfig,compiler::NativeCompilerStore};
use std::collections::{BTreeMap,BTreeSet};

#[tokio::test(flavor="multi_thread")]
async fn native_closure_windows_projection_pin_and_stream_lifetime(){
    let path=std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned disposable compiler fixture");
    let config=RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let package=Package{name:"selected".into()};let unrelated=Package{name:"unrelated".into()};
    let mut releases=(0..TRANSFER_ROWS+3).map(|index|Release{package:package.id(),version:format!("version-{index}")}).collect::<Vec<_>>();
    releases.sort_by_key(Record::id);
    let huge=Release{package:unrelated.id(),version:"x".repeat(4<<20)};
    let package_relation=Relation::of::<Package>();let release_relation=Relation::of::<Release>();
    let spec=ContributionSpec{producer:"native-closure".into(),profile:Profile::Catalog,model:ContentHash::of(b"closure-model"),implementation:ContentHash::of(b"closure-implementation"),configuration:None,inputs:vec![],outputs:BTreeSet::from([Package::NAME.into(),Release::NAME.into()])};
    let contribution=store.begin_contribution(spec.clone()).await.unwrap();
    store.write_batch(&contribution,&package_relation,&Package::encode(&[package.clone(),unrelated.clone()]).unwrap()).await.unwrap();
    store.write_batch(&contribution,&release_relation,&Release::encode(&releases).unwrap()).await.unwrap();
    store.write_batch(&contribution,&release_relation,&Release::encode(std::slice::from_ref(&huge)).unwrap()).await.unwrap();
    let views=store.complete_contribution(contribution,ProviderOutcome::Complete,&[package_relation.clone(),release_relation.clone()],&BTreeMap::new()).await.unwrap();
    let pending=Release{package:package.id(),version:"pending".into()};let mut next=spec;next.producer="pending".into();
    let pending_contribution=store.begin_contribution(next).await.unwrap();
    store.write_batch(&pending_contribution,&release_relation,&Release::encode(std::slice::from_ref(&pending)).unwrap()).await.unwrap();
    let native_budget=ResourceBudget::fixed(4<<20).unwrap();let closure_budget=ResourceBudget::fixed(32<<20).unwrap();
    let session=SessionContext::new();
    let packages=store.table_provider(&views[Package::NAME],package_relation.clone(),native_budget.clone(),64).unwrap();
    let release_provider=store.table_provider(&views[Release::NAME],release_relation.clone(),native_budget.clone(),128).unwrap();
    session.register_table("packages",packages).unwrap();session.register_table("releases",release_provider.clone()).unwrap();
    let state=session.state();
    let unfiltered=release_provider.scan(&state,None,&[],None).await.unwrap();
    #[allow(deprecated,reason="The physical join optimizer currently consumes partition statistics")]
    let statistics=unfiltered.partition_statistics(None).unwrap();
    assert_eq!(statistics.num_rows,Precision::Exact(releases.len()+1));
    let mut plan=NominalClosure::new(vec![ClosureTable{relation:release_relation,alias:"releases".into()},ClosureTable{relation:package_relation,alias:"packages".into()}]).unwrap();
    plan.follow(0,"package",1).unwrap();plan.own(0,"package",1).unwrap();plan.own(0,"package",1).unwrap();
    plan.pairs(0,1,"SELECT id AS source_id,package AS target_id FROM releases WHERE version='version-0'".into()).unwrap();
    // Corrupt an unrelated physical reference with a schema-valid negative integer. A bulk
    // native topology scan would try to decode it; the selected owner frontier must not.
    let mut vars=lctx_surrealdb::surrealdb::types::Variables::new();vars.insert("key",huge.id().hex());vars.insert("wrong",vec![-1i64;16]);
    store.client().query("UPDATE entity SET body.package=$wrong WHERE semantic_type='releases' AND semantic_key=$key RETURN NONE").bind(vars).await.unwrap().check().unwrap();
    let edges=plan.prepare(&session,&closure_budget).await.unwrap();
    let scope=edges.grain(1,&format!("id=X'{}'",package.id().hex()),&closure_budget).await.unwrap();
    let selected=scope.select(0).unwrap();assert_eq!(scope.select(0).unwrap(),selected);
    let filtered=scope.session().sql(&format!("SELECT id FROM ({selected}) selected WHERE version='version-7'")).await.unwrap().collect().await.unwrap();
    assert_eq!(filtered.iter().map(|batch|batch.num_rows()).sum::<usize>(),1,"static filters remain inside selected native windows");
    let frame=scope.session().sql(&selected).await.unwrap();
    let physical=frame.clone().create_physical_plan().await.unwrap();
    #[allow(deprecated,reason="The physical join optimizer currently consumes partition statistics")]
    let selected_statistics=physical.partition_statistics(None).unwrap();
    assert_eq!(selected_statistics.num_rows,Precision::Inexact(releases.len()));
    let mut stream=frame.execute_stream().await.unwrap();drop(physical);drop(scope);drop(edges);
    assert!(closure_budget.reserved()>0,"active native streams retain their nominal key owner");
    let mut actual=Vec::new();while let Some(batch)=stream.try_next().await.unwrap(){actual.extend(Release::decode(&batch).unwrap());}
    assert_eq!(actual,releases,"selected windows preserve semantic key order and exclude unrelated and pending payloads");
    drop(stream);assert_eq!(closure_budget.reserved(),0);
    let mut vars=lctx_surrealdb::surrealdb::types::Variables::new();vars.insert("key",huge.id().hex());vars.insert("package",huge.package.bytes().to_vec());
    store.client().query("UPDATE entity SET body.package=$package WHERE semantic_type='releases' AND semantic_key=$key RETURN NONE").bind(vars).await.unwrap().check().unwrap();
    let projected_scope=plan.prepare(&session,&closure_budget).await.unwrap().grain(1,&format!("id=X'{}'",unrelated.id().hex()),&closure_budget).await.unwrap();
    let batches=projected_scope.session().sql(&format!("SELECT id FROM ({}) selected",projected_scope.select(0).unwrap())).await.unwrap().collect().await.unwrap();
    assert_eq!(batches.iter().map(|batch|batch.num_rows()).sum::<usize>(),1);
    let ids=batches.iter().flat_map(|batch|{assert_eq!(batch.num_columns(),1);let ids=batch.column(0).as_any().downcast_ref::<datafusion::arrow::array::FixedSizeBinaryArray>().unwrap();(0..batch.num_rows()).map(|row|ids.value(row).to_vec()).collect::<Vec<_>>()}).collect::<Vec<_>>();
    assert_eq!(ids,vec![huge.id().bytes().to_vec()],"projected native reads omit a body larger than the native transfer budget");
    drop(projected_scope);drop(unfiltered);store.abandon().await.unwrap();
}
