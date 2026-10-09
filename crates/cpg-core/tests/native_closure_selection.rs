//! Actual native grains bind compact keys before rich hydration, under the frozen view.
use cpg_core::consumed_rows::{ClosureTable,NominalClosure};
use datafusion::{prelude::SessionContext,common::stats::Precision};
use futures::TryStreamExt;
use lctx_model::domain::{ContentHash,Record,Relation,completed::ContributionSpec,input::{Package,Release},resources::{ResourceBudget,TRANSFER_ROWS},stages::{Profile,ProviderOutcome}};
use lctx_surrealdb::{RuntimeConfig,compiler::NativeCompilerStore};
use std::collections::{BTreeMap,BTreeSet};

#[tokio::test(flavor="multi_thread")]
async fn native_existing_owner_membership_excludes_dangling_keys_at_exact_epochs(){
    async fn persist<R:Record>(store:&std::sync::Arc<NativeCompilerStore>,producer:&str,rows:&[R])->lctx_model::domain::completed::CompletedView {
        let relation=Relation::of::<R>();
        let spec=ContributionSpec{captured_binding: None, producer:producer.into(),profile:Profile::Catalog,model:ContentHash::of(b"existing-owner-model"),implementation:ContentHash::of(b"existing-owner-implementation"),configuration:None,inputs:vec![],outputs:BTreeSet::from([R::NAME.into()])};
        let contribution=store.begin_contribution(spec).await.unwrap();
        store.write_batch(&contribution,&relation,&R::encode(rows).unwrap()).await.unwrap();
        store.complete_contribution(contribution,ProviderOutcome::Complete,&[relation],&BTreeMap::new()).await.unwrap().remove(R::NAME).unwrap()
    }
    let path=std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned disposable compiler fixture");
    let config=RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let known=Package{name:"known-in-first-epoch".into()};
    let missing=Package{name:"absent-from-first-epoch".into()};
    let known_root=Release{package:known.id(),version:"known-root".into()};
    let dangling_root=Release{package:missing.id(),version:"dangling-root".into()};
    let known_child=Release{package:known.id(),version:"known-child".into()};
    let dangling_child=Release{package:missing.id(),version:"dangling-child".into()};
    let roots=persist(&store,"existing-owner-roots",&[known_root.clone(),dangling_root.clone()]).await;
    let first=persist(&store,"existing-owner-first",std::slice::from_ref(&known)).await;
    let second=persist(&store,"existing-owner-second",std::slice::from_ref(&missing)).await;
    let children=persist(&store,"existing-owner-children",&[known_child.clone(),dangling_child.clone()]).await;
    let budget=ResourceBudget::fixed(8<<20).unwrap();let session=SessionContext::new();
    let tables=[("roots",Relation::of::<Release>(),roots),("first_owners",Relation::of::<Package>(),first),("first_children",Relation::of::<Release>(),children.clone()),("second_owners",Relation::of::<Package>(),second),("second_children",Relation::of::<Release>(),children)];
    for (alias,relation,view) in &tables {session.register_table(*alias,store.table_provider(view,relation.clone(),budget.clone(),16).unwrap()).unwrap();}
    let mut plan=NominalClosure::new(tables.iter().map(|(alias,relation,_)|ClosureTable{relation:relation.clone(),alias:(*alias).into()}).collect()).unwrap();
    plan.follow(0,"package",1).unwrap();plan.follow(0,"package",3).unwrap();
    plan.own_existing(2,"package",1).unwrap();plan.own_existing(4,"package",3).unwrap();
    let edges=plan.prepare(&session,&budget).await.unwrap();
    for root in [&known_root,&dangling_root] {
        let scope=edges.grain(0,&format!("id=X'{}'",root.id().hex()),&budget).await.unwrap();
        let read=|table|{let query=scope.select(table).unwrap();let scope=&scope;async move{scope.session().sql(&query).await.unwrap().collect().await.unwrap()}};
        let actual=read(0).await;
        assert_eq!(actual.iter().flat_map(|batch|Release::decode(batch).unwrap()).collect::<Vec<_>>(),vec![(*root).clone()],"the actual root survives even when one exact owner view has no matching row");
        let actual=read(1).await;
        assert_eq!(actual.iter().flat_map(|batch|Package::decode(batch).unwrap()).collect::<Vec<_>>(),if root==&known_root{vec![known.clone()]}else{vec![]});
        let actual=read(2).await;
        assert_eq!(actual.iter().flat_map(|batch|Release::decode(batch).unwrap()).collect::<Vec<_>>(),if root==&known_root{vec![known_child.clone()]}else{vec![]},"a dangling reached key must not admit children without an owner in the first view");
        let actual=read(3).await;
        assert_eq!(actual.iter().flat_map(|batch|Package::decode(batch).unwrap()).collect::<Vec<_>>(),if root==&dangling_root{vec![missing.clone()]}else{vec![]});
        let actual=read(4).await;
        assert_eq!(actual.iter().flat_map(|batch|Release::decode(batch).unwrap()).collect::<Vec<_>>(),if root==&dangling_root{vec![dangling_child.clone()]}else{vec![]},"presence in another exact owner epoch must not substitute for this epoch");
    }
    drop(edges);store.abandon().await.unwrap();
}

#[tokio::test(flavor="multi_thread")]
async fn native_forward_fields_preserve_null_shared_targets_and_exact_epochs(){
    use lctx_model::domain::{Id,calls::{CallTarget,CallPhase,ProviderSymbol,SymbolKind},source::{Occurrence,OccurrenceRole,SyntaxKind}};
    // This is a closure-selection control, not semantic admission of the unselected fields.
    fn nominal<T>(byte:u8)->Id<T>{serde_json::from_value(serde_json::to_value(vec![byte;16]).unwrap()).unwrap()}
    async fn persist<R:Record>(store:&std::sync::Arc<NativeCompilerStore>,producer:&str,rows:&[R])->lctx_model::domain::completed::CompletedView {
        let relation=Relation::of::<R>();
        let spec=ContributionSpec{captured_binding: None, producer:producer.into(),profile:Profile::Catalog,model:ContentHash::of(b"forward-closure-model"),implementation:ContentHash::of(b"forward-closure-implementation"),configuration:None,inputs:vec![],outputs:BTreeSet::from([R::NAME.into()])};
        let contribution=store.begin_contribution(spec).await.unwrap();
        store.write_batch(&contribution,&relation,&R::encode(rows).unwrap()).await.unwrap();
        store.complete_contribution(contribution,ProviderOutcome::Complete,&[relation],&BTreeMap::new()).await.unwrap().remove(R::NAME).unwrap()
    }
    let path=std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned disposable compiler fixture");
    let config=RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let occurrence=Occurrence{source:nominal(1),start:0,end:1,syntax_kind:SyntaxKind::ExprCall,role:OccurrenceRole::Call,structural_path:vec![]};
    let other_occurrence=Occurrence{start:2,end:3,..occurrence.clone()};
    let symbol=ProviderSymbol{provider:nominal(2),context:nominal(3),module:nominal(4),native_key:"selected-class".into(),name:"Selected".into(),kind:SymbolKind::Class};
    let other_symbol=ProviderSymbol{native_key:"unrelated-class".into(),name:"Unrelated".into(),..symbol.clone()};
    let target=CallTarget{qualification:nominal(5),site:occurrence.id(),origin:nominal(6),destination:nominal(7),channel:nominal(8),phase:CallPhase::Call,receiver:nominal(9),implicit:false,receiver_class:Some(symbol.id()),passing:None,class_method:None,static_method:None};
    let null_target=CallTarget{receiver_class:None,..target.clone()};
    let unrelated=CallTarget{site:other_occurrence.id(),receiver_class:Some(other_symbol.id()),..target.clone()};
    let roots=persist(&store,"forward-roots",&[target.clone(),null_target.clone(),unrelated]).await;
    let first=persist(&store,"first-occurrence-epoch",std::slice::from_ref(&occurrence)).await;
    let second=persist(&store,"second-occurrence-epoch",std::slice::from_ref(&other_occurrence)).await;
    let symbols=persist(&store,"forward-symbols",&[symbol.clone(),other_symbol]).await;
    let budget=ResourceBudget::fixed(8<<20).unwrap();let session=SessionContext::new();
    let tables=[("roots",Relation::of::<CallTarget>(),roots),("first_occurrences",Relation::of::<Occurrence>(),first),("second_occurrences",Relation::of::<Occurrence>(),second),("symbols",Relation::of::<ProviderSymbol>(),symbols)];
    for (alias,relation,view) in &tables {session.register_table(*alias,store.table_provider(view,relation.clone(),budget.clone(),16).unwrap()).unwrap();}
    let mut plan=NominalClosure::new(tables.iter().map(|(alias,relation,_)|ClosureTable{relation:relation.clone(),alias:(*alias).into()}).collect()).unwrap();
    plan.follow(0,"site",1).unwrap();plan.follow(0,"site",2).unwrap();plan.follow(0,"receiver_class",3).unwrap();
    let edges=plan.prepare(&session,&budget).await.unwrap();
    for root in [&target,&null_target] {
        let scope=edges.grain(0,&format!("id=X'{}'",root.id().hex()),&budget).await.unwrap();
        let read=|table|{let query=scope.select(table).unwrap();let scope=&scope;async move{scope.session().sql(&query).await}};
        let actual=read(0).await.unwrap().collect().await.unwrap();
        assert_eq!(actual.iter().flat_map(|batch|CallTarget::decode(batch).unwrap()).collect::<Vec<_>>(),vec![(*root).clone()]);
        let actual=read(1).await.unwrap().collect().await.unwrap();
        assert_eq!(actual.iter().flat_map(|batch|Occurrence::decode(batch).unwrap()).collect::<Vec<_>>(),vec![occurrence.clone()]);
        let actual=read(2).await.unwrap().collect().await.unwrap();
        assert_eq!(actual.iter().map(|batch|batch.num_rows()).sum::<usize>(),0,"same reference bytes must retain the explicitly selected target epoch");
        let actual=read(3).await.unwrap().collect().await.unwrap();
        let expected=if root.receiver_class.is_some(){vec![symbol.clone()]}else{vec![]};
        assert_eq!(actual.iter().flat_map(|batch|ProviderSymbol::decode(batch).unwrap()).collect::<Vec<_>>(),expected,"nullable links and shared targets exclude unrelated neighbors");
    }
    drop(edges);store.abandon().await.unwrap();
}

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
    let spec=ContributionSpec{captured_binding: None, producer:"native-closure".into(),profile:Profile::Catalog,model:ContentHash::of(b"closure-model"),implementation:ContentHash::of(b"closure-implementation"),configuration:None,inputs:vec![],outputs:BTreeSet::from([Package::NAME.into(),Release::NAME.into()])};
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
