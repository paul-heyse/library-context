//! Actual native admission closure races a cold view registration and held transport.
use lctx_model::domain::{ContentHash,Record,Relation,input::Package,completed::ContributionSpec,admission::Frontier,stages::{Profile,ProviderOutcome}};
use lctx_surrealdb::{RuntimeConfig,compiler::NativeCompilerStore};
use std::collections::{BTreeMap,BTreeSet};

#[tokio::test(flavor="multi_thread")]
async fn closing_admission_waits_for_cold_registration_and_held_native_rows(){
    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,Frontier::Facts).await.unwrap();
    let relation=Relation::of::<Package>();
    let specification=ContributionSpec{producer:"lifecycle".into(),profile:Profile::Catalog,model:ContentHash::of(b"lifecycle-model"),implementation:ContentHash::of(b"lifecycle-code"),configuration:None,inputs:vec![],outputs:BTreeSet::from([relation.name().into()])};
    let id=store.begin_contribution(specification.clone()).await.unwrap();
    store.write_batch(&id,&relation,&Package::encode(&[Package{name:"retained".into()}]).unwrap()).await.unwrap();
    let views=store.complete_contribution(id,ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new()).await.unwrap();
    let cold=NativeCompilerStore::from_existing(store.shared_client(),store.namespace().clone(),store.database().clone());
    let mut registration=Box::pin(cold.scan_rows(&views[relation.name()],&relation,None,None));
    assert!(futures::poll!(&mut registration).is_pending(),"cold registration reaches its SDK metadata await");
    let final_owner=cold.clone();let finalization=tokio::spawn(async move{final_owner.end_writes().await});
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    assert!(!finalization.is_finished(),"admission was recorded before metadata await");
    assert!(cold.begin_contribution(specification).await.is_err(),"closure refuses new work");
    let rows=registration.await.unwrap();
    assert!(!finalization.is_finished(),"returned transport retains admission through terminal drain");
    drop(rows);
    tokio::time::timeout(std::time::Duration::from_secs(10),finalization).await.unwrap().unwrap().unwrap();
    assert!(cold.check().is_err());
    store.abandon().await.unwrap();
}
