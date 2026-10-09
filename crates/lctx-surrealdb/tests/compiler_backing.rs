//! Actual native ownership for private model-declared projection backing.
use lctx_model::domain::{ContentHash, EvidenceBytes, Id, Record, Relation, ModelError,
    admission::Frontier, completed::ContributionSpec, projection::{ProjectionSnapshot,ProjectionSnapshotChunk,ProjectionSourceAssessment,snapshot},
    stages::{Profile,ProviderOutcome}};
use lctx_surrealdb::{RuntimeConfig,compiler::NativeCompilerStore};
use std::collections::{BTreeMap,BTreeSet};

#[tokio::test(flavor="multi_thread")]
async fn projection_backing_preserves_typed_keys_payloads_and_cold_state(){
    let path=std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config=RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store=NativeCompilerStore::begin(&config,Frontier::Normalized).await.unwrap();
    let assessment:Id<ProjectionSourceAssessment>=serde_json::from_value(serde_json::json!(vec![9u8;16])).unwrap();
    let header=snapshot::header(assessment,snapshot::CHUNK_BYTES).unwrap();
    let mut payload=vec![0xff;snapshot::CHUNK_BYTES];payload[..3].copy_from_slice(&[0xff,0,0x80]);
    let chunk=ProjectionSnapshotChunk{snapshot:header.id(),ordinal:0,payload:EvidenceBytes(payload)};
    let header_relation=Relation::of::<ProjectionSnapshot>();let chunk_relation=Relation::of::<ProjectionSnapshotChunk>();
    let spec=ContributionSpec{captured_binding: None, producer:"projection-backing".into(),profile:Profile::Catalog,model:ContentHash::of(b"projection-backing-model"),implementation:ContentHash::of(b"projection-backing-code"),configuration:None,inputs:vec![],outputs:BTreeSet::from([ProjectionSnapshot::NAME.into(),ProjectionSnapshotChunk::NAME.into()])};
    let contribution=store.begin_contribution(spec.clone()).await.unwrap();
    store.write_batch(&contribution,&header_relation,&ProjectionSnapshot::encode(std::slice::from_ref(&header)).unwrap()).await.unwrap();
    store.write_batch(&contribution,&chunk_relation,&ProjectionSnapshotChunk::encode(std::slice::from_ref(&chunk)).unwrap()).await.unwrap();
    let views=store.complete_contribution(contribution,ProviderOutcome::Complete,&[header_relation.clone(),chunk_relation.clone()],&BTreeMap::new()).await.unwrap();
    let budget=lctx_model::domain::resources::ResourceBudget::fixed(8<<20).unwrap();
    use futures::TryStreamExt;
    let batches=store.scan_batches(&views[ProjectionSnapshotChunk::NAME],&chunk_relation,None,None,&budget,32).await.unwrap().try_collect::<Vec<_>>().await.unwrap();
    let actual=batches.iter().flat_map(|batch|ProjectionSnapshotChunk::decode(batch).unwrap()).collect::<Vec<_>>();
    assert_eq!(actual,vec![chunk.clone()]);
    let file=tempfile::NamedTempFile::new().unwrap();let state=store.export_state(file.path()).await.unwrap();assert_eq!(state.backing_rows,2);
    assert!(std::fs::read_to_string(file.path()).unwrap().lines().any(|line|line.len()>lctx_model::domain::resources::TRANSFER_BYTES),"maximum declared opaque chunk requires one oversized transport envelope");
    let restored=NativeCompilerStore::begin(&config,Frontier::Normalized).await.unwrap();
    restored.import_state(file.path(),&state).await.unwrap();
    assert_eq!(restored.completed_state().await.unwrap(),state);
    let batches=restored.scan_batches(&views[ProjectionSnapshot::NAME],&header_relation,None,None,&budget,32).await.unwrap().try_collect::<Vec<_>>().await.unwrap();
    assert_eq!(batches.iter().flat_map(|batch|ProjectionSnapshot::decode(batch).unwrap()).collect::<Vec<_>>(),vec![header]);
    restored.abandon().await.unwrap();
    use lctx_surrealdb::surrealdb::types::{Bytes,Number,Value,Variables};
    let mut response=store.client().query("SELECT * FROM compiler_record WHERE semantic_type='projection_snapshot_chunks'").await.unwrap().check().unwrap();
    let mut rows:Vec<Value>=response.take(0).unwrap();let original=rows.pop().unwrap();assert!(rows.is_empty());
    let Value::Object(original)=original else{panic!("native backing row");};
    let id=original.get("id").unwrap().clone();
    let mut altered=original.clone();
    let Some(Value::Object(body))=altered.get_mut("body") else{panic!("native backing body");};
    body.insert("ordinal",Value::Number(Number::Int(1)));
    let canonical=serde_json::to_vec(&Value::Object(body.clone())).unwrap();
    altered.insert("canonical",Bytes::from(canonical.clone()));altered.insert("content",ContentHash::of(&canonical).hex());
    let mut bindings=Variables::new();bindings.insert("id",id.clone());bindings.insert("row",Value::Object(altered));
    store.client().query("UPDATE $id CONTENT $row RETURN NONE").bind(bindings).await.unwrap().check().unwrap();
    assert!(matches!(store.verify_state().await,Err(ModelError::Conflict("compiler backing typed identity"))),"coherent canonical/body/content changes cannot bypass full nominal-key validation");
    let mut bindings=Variables::new();bindings.insert("id",id.clone());bindings.insert("row",Value::Object(original.clone()));
    store.client().query("UPDATE $id CONTENT $row RETURN NONE").bind(bindings).await.unwrap().check().unwrap();
    let mut bindings=Variables::new();bindings.insert("id",id.clone());bindings.insert("key","00".repeat(16));
    store.client().query("UPDATE $id SET semantic_key=$key RETURN NONE").bind(bindings).await.unwrap().check().unwrap();
    assert!(matches!(store.verify_state().await,Err(ModelError::Conflict("compiler backing typed identity"))),"stored nominal keys are independently recomputed");
    let mut bindings=Variables::new();bindings.insert("id",id);bindings.insert("row",Value::Object(original));
    store.client().query("UPDATE $id CONTENT $row RETURN NONE").bind(bindings).await.unwrap().check().unwrap();
    assert_eq!(store.completed_state().await.unwrap(),state);
    let mut conflicting=chunk;conflicting.payload=EvidenceBytes(vec![1,2,3]);
    let mut next=spec;next.producer="conflicting-projection-backing".into();
    let contribution=store.begin_contribution(next).await.unwrap();
    assert!(matches!(store.write_batch(&contribution,&chunk_relation,&ProjectionSnapshotChunk::encode(&[conflicting]).unwrap()).await,Err(ModelError::Conflict("native same-key payload"))));
    store.abandon().await.unwrap();
}

#[tokio::test(flavor="multi_thread")]
async fn cold_backing_rejects_coherent_negative_zero_before_membership_checks(){
    use lctx_model::domain::{FiniteF64,analytics::QualityStep};
    use lctx_surrealdb::surrealdb::types::{Bytes,Number,Value,Variables};
    let path=std::env::var("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture");
    let config=RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
    let store=NativeCompilerStore::begin(&config,Frontier::Facts).await.unwrap();
    let relation=Relation::of::<QualityStep>();
    let row=QualityStep{run:serde_json::from_value(serde_json::json!(vec![3u8;16])).unwrap(),ordinal:0,value:FiniteF64::new(0.0).unwrap()};
    let spec=ContributionSpec{captured_binding: None, producer:"quality-zero".into(),profile:Profile::Catalog,model:ContentHash::of(b"quality-zero-model"),implementation:ContentHash::of(b"quality-zero-code"),configuration:None,inputs:vec![],outputs:BTreeSet::from([QualityStep::NAME.into()])};
    let contribution=store.begin_contribution(spec).await.unwrap();
    store.write_batch(&contribution,&relation,&QualityStep::encode(&[row]).unwrap()).await.unwrap();
    store.complete_contribution(contribution,ProviderOutcome::Complete,&[relation],&BTreeMap::new()).await.unwrap();
    let mut response=store.client().query("SELECT * FROM compiler_record").await.unwrap().check().unwrap();
    let mut rows:Vec<Value>=response.take(0).unwrap();let Value::Object(mut altered)=rows.pop().unwrap() else{panic!("quality backing row");};assert!(rows.is_empty());
    let id=altered.get("id").unwrap().clone();
    let Some(Value::Object(body))=altered.get_mut("body") else{panic!("quality backing body");};
    body.insert("value",Value::Number(Number::Float(-0.0)));
    let canonical=serde_json::to_vec(&Value::Object(body.clone())).unwrap();
    altered.insert("canonical",Bytes::from(canonical.clone()));altered.insert("content",ContentHash::of(&canonical).hex());
    let mut bindings=Variables::new();bindings.insert("id",id);bindings.insert("row",Value::Object(altered));
    store.client().query("UPDATE $id CONTENT $row RETURN NONE").bind(bindings).await.unwrap().check().unwrap();
    assert!(matches!(store.verify_state().await,Err(ModelError::Conflict("compiler backing declared body"))),"model-normalized numeric bytes must be exact before membership checking");
    assert!(matches!(store.completed_state().await,Err(ModelError::Conflict("compiler backing declared body"))));
    store.abandon().await.unwrap();
}
