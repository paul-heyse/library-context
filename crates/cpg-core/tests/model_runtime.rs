//! Completed input catalogs are isolated and share the compiler allocation pool.
use cpg_core::workspace::{Workspace,WorkspaceOptions};
use lctx_model::domain::{input::Package,stages::{Profile,ProviderOutcome},*};
use std::sync::Arc;
#[tokio::test]
async fn completed_input_sessions_are_isolated_and_detached_queries_retain_sources() {
 let workspace=Workspace::new(Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap()),WorkspaceOptions::default()).unwrap();
 let empty=workspace.inputs("source",Profile::Catalog,[]).unwrap();
 let output=workspace.output("source",Profile::Catalog,ContentHash::of(b"source"),empty.clone());
 output.declare::<Package>().unwrap();output.push(Package{name:"retained".into()}).await.unwrap();output.finish(ProviderOutcome::Complete).await.unwrap();
 assert!(!empty.contains::<Package>());assert!(empty.read::<Package>().is_err());
 let inputs=workspace.inputs("consumer",Profile::Catalog,[Package::NAME]).unwrap();
 let first=inputs.session(&workspace).await.unwrap();let second=empty.session(&workspace).await.unwrap();
 assert!(second.sql("SELECT * FROM packages").await.is_err());
 let query=first.sql("SELECT * FROM packages").await.unwrap();drop(first);drop(inputs);
 let batches=query.collect().await.unwrap();assert_eq!(batches.iter().map(|b|b.num_rows()).sum::<usize>(),1);
}
#[tokio::test]
async fn compute_and_external_reservations_share_one_compiler_pool() {
 let workspace=Workspace::new(Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap()),WorkspaceOptions{memory_bytes:32*1024*1024,partitions:1,..Default::default()}).unwrap();
 let budget=workspace.budget().clone();let empty=workspace.inputs("compute",Profile::Catalog,[]).unwrap();
 let first=empty.session(&workspace).await.unwrap();let second=empty.session(&workspace).await.unwrap();
 let allocation=budget.reserve("producer",budget.limit()).unwrap();assert!(budget.reserve("validator",1).is_err());
 let query="SELECT v FROM generate_series(1, 20000) AS t(v) ORDER BY v DESC";
 assert!(first.sql(query).await.unwrap().collect().await.is_err());drop(allocation);
 let rows=second.sql(query).await.unwrap().collect().await.unwrap();assert_eq!(rows.iter().map(|b|b.num_rows()).sum::<usize>(),20000);
 drop(rows);let retained=budget.reserve("fixture-retained",8*1024*1024).unwrap();
 let rows=second.sql("SELECT v FROM generate_series(1, 2000000) AS t(v) ORDER BY v DESC").await.unwrap().collect().await.unwrap();assert_eq!(rows.iter().map(|b|b.num_rows()).sum::<usize>(),2000000);assert_eq!(budget.reserved(),8*1024*1024,"external reservation survives bounded compute");drop(rows);drop(retained);drop(first);drop(second);drop(empty);drop(workspace);assert_eq!(budget.reserved(),0);
}
