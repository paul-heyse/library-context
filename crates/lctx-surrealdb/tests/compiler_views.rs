use futures::TryStreamExt;
use lctx_model::domain::{
    ContentHash, Record, Relation, input::Package,
    completed::ContributionSpec, resources::ResourceBudget,
    stages::{Profile, ProviderOutcome},
};
use lctx_surrealdb::{RuntimeConfig, compiler::{NativeCompilerStore, NativePredicate}};
use std::collections::{BTreeMap, BTreeSet};

fn spec(producer: &str, relation: &Relation) -> ContributionSpec {
    ContributionSpec {
        producer: producer.into(), profile: Profile::Catalog,
        model: ContentHash::of(b"view-fixture-model"),
        implementation: ContentHash::of(b"view-fixture-implementation"),
        configuration: None, inputs: vec![],
        outputs: BTreeSet::from([relation.name().into()]),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cold_backing_rejects_valid_body_changes_and_false_typed_keys() {
    use lctx_model::domain::{FiniteF64, ModelError, analytics::QualityStep};
    use lctx_surrealdb::surrealdb::types::{Number, Value, Variables};
    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let relation=Relation::of::<QualityStep>();
    let row=QualityStep {run:serde_json::from_value(serde_json::json!(vec![3u8;16])).unwrap(),ordinal:0,value:FiniteF64::new(0.5).unwrap()};
    let contribution=store.begin_contribution(spec("quality",&relation)).await.unwrap();
    store.write_batch(&contribution,&relation,&QualityStep::encode(&[row.clone()]).unwrap()).await.unwrap();
    store.complete_contribution(contribution,ProviderOutcome::Complete,&[relation],&BTreeMap::new()).await.unwrap();
    store.verify_state().await.unwrap();
    let file=tempfile::NamedTempFile::new().unwrap();
    let state=store.export_state(file.path()).await.unwrap();
    assert_eq!(state.backing_rows,1);
    let restored=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    restored.import_state(file.path(),&state).await.unwrap();
    restored.abandon().await.unwrap();

    store.client().query("UPDATE compiler_record SET body.value=2.0f").await.unwrap().check().unwrap();
    assert!(matches!(store.verify_state().await,Err(ModelError::Conflict("compiler backing canonical body"))));
    assert!(matches!(store.completed_state().await,Err(ModelError::Conflict("compiler backing canonical body"))));
    store.client().query("UPDATE compiler_record SET body.value=0.5f").await.unwrap().check().unwrap();
    let mut bindings=Variables::new();bindings.insert("key","00".repeat(16));
    store.client().query("UPDATE compiler_record SET semantic_key=$key").bind(bindings).await.unwrap().check().unwrap();
    assert!(matches!(store.verify_state().await,Err(ModelError::Conflict("compiler backing typed identity"))));
    assert!(matches!(store.completed_state().await,Err(ModelError::Conflict("compiler backing typed identity"))));
    let mut bindings=Variables::new();bindings.insert("key",row.id().hex());
    store.client().query("UPDATE compiler_record SET semantic_key=$key").bind(bindings).await.unwrap().check().unwrap();
    assert_eq!(store.completed_state().await.unwrap(),state);

    // External transport must reject the row before trusting even an outer state identity.
    #[derive(serde::Serialize,serde::Deserialize)]
    struct Envelope {table:String,row:Value}
    let text=std::fs::read_to_string(file.path()).unwrap();
    let mut envelopes=text.lines().map(|line|serde_json::from_str::<Envelope>(line).unwrap()).collect::<Vec<_>>();
    for envelope in &mut envelopes {
        if envelope.table=="compiler_record" {
            let Value::Object(object)=&mut envelope.row else{panic!("backing object");};
            let Some(Value::Object(body))=object.get_mut("body") else{panic!("backing body");};
            body.insert("value",Value::Number(Number::Float(2.0)));
        }
    }
    let text=envelopes.iter().map(|row|serde_json::to_string(row).unwrap()+"\n").collect::<String>();
    std::fs::write(file.path(),text).unwrap();
    let altered=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    assert!(matches!(altered.import_state(file.path(),&state).await,Err(ModelError::Conflict("compiler backing canonical body"))));
    altered.abandon().await.unwrap();
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn pending_overlap_frozen_selection_and_state_transport() {
    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config = RuntimeConfig::read(&path).unwrap();
    let store = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let relation = Relation::of::<Package>();
    let first = Package { name: "first".into() };
    let second = Package { name: "second".into() };
    let a = store.begin_contribution(spec("a", &relation)).await.unwrap();
    store.write_batch(&a, &relation, &Package::encode(std::slice::from_ref(&first)).unwrap()).await.unwrap();
    store.write_batch(&a, &relation, &Package::encode(std::slice::from_ref(&first)).unwrap()).await.unwrap();
    let views = store.complete_contribution(a, ProviderOutcome::Complete, std::slice::from_ref(&relation), &BTreeMap::new()).await.unwrap();
    let frozen = views[relation.name()].clone();
    let mut next=spec("b", &relation);
    next.inputs.push(lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(&relation,next.model,&frozen).unwrap());
    let b = store.begin_contribution(next).await.unwrap();
    store.write_batch(&b, &relation, &Package::encode(&[first.clone(), second.clone()]).unwrap()).await.unwrap();
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    let mut rows = store.scan_batches(&frozen, &relation, None, None, &budget, 8).await.unwrap();
    let mut observed = vec![];
    while let Some(batch) = rows.try_next().await.unwrap() { observed.extend(Package::decode(&batch).unwrap()); }
    assert_eq!(observed, vec![first.clone()], "pending values cannot widen a frozen view");
    let current = store.complete_contribution(b, ProviderOutcome::Complete, std::slice::from_ref(&relation), &views).await.unwrap();
    assert_eq!(current[relation.name()].rows, 2, "overlapping membership must deduplicate");
    assert_ne!(current[relation.name()].identity, frozen.identity);
    let mut response=store.client().query("SELECT producer,inputs.relation AS predecessors FROM compiler_contribution WHERE producer='b'").await.unwrap().check().unwrap();
    let projected:Vec<serde_json::Value>=response.take(0).unwrap();
    assert_eq!(projected[0]["predecessors"],serde_json::json!([relation.name()]),"dependency views are natively queryable");
    let mut rows = store.scan_batches(&current[relation.name()], &relation, None,
        Some(NativePredicate::Keys(vec![*second.id().bytes()])), &budget, 8).await.unwrap();
    let selected = rows.try_next().await.unwrap().unwrap();
    assert_eq!(Package::decode(&selected).unwrap(), vec![second]);
    assert!(rows.try_next().await.unwrap().is_none());
    let state = store.completed_state().await.unwrap();
    assert_eq!(state.contributions, 2);
    assert_eq!(state.memberships, 3);
    let file = tempfile::NamedTempFile::new().unwrap();
    assert_eq!(store.export_state(file.path()).await.unwrap(), state);
    let restored = NativeCompilerStore::begin(&config, lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    // Complete state references canonical families; detached import loads those separately.
    lctx_surrealdb::Loader::new(restored.shared_client()).entities(&[
        lctx_model::domain::graph::Entity::from(first.clone()),
        lctx_model::domain::graph::Entity::from(Package { name: "second".into() }),
    ]).await.unwrap();
    restored.import_state(file.path(), &state).await.unwrap();
    assert_eq!(restored.completed_state().await.unwrap(), state);
    // Replay compares complete immutable membership rows, rather than replacing corrupted
    // metadata or silently ignoring an existing key. Published graph payloads remain identical.
    let replay=restored.begin_contribution(spec("replay",&relation)).await.unwrap();
    let batch=Package::encode(std::slice::from_ref(&first)).unwrap();
    restored.write_batch(&replay,&relation,&batch).await.unwrap();
    restored.write_batch(&replay,&relation,&batch).await.unwrap();
    let mut bindings=lctx_surrealdb::surrealdb::types::Variables::new();
    bindings.insert("contribution",lctx_surrealdb::surrealdb::types::RecordId::new("compiler_contribution",replay.hex()));
    bindings.insert("content",ContentHash::of(b"corrupt-membership").hex());
    restored.client().query("UPDATE compiler_membership SET content=$content WHERE contribution=$contribution").bind(bindings).await.unwrap().check().unwrap();
    assert!(matches!(restored.write_batch(&replay,&relation,&batch).await,Err(lctx_model::domain::ModelError::Conflict("native membership same-key payload"))));
    store.abandon().await.unwrap();
    restored.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn opaque_original_chunks_use_one_physical_owner_and_detect_same_key_conflict() {
    use lctx_model::domain::{EvidenceBytes, source::SourceArtifact, artifact::ArtifactChunk, input::InputRevision};
    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let bytes=(0..(1024*1024)).map(|index|(index%256) as u8).collect::<Vec<_>>();
    let source=SourceArtifact::from_bytes(InputRevision::from_entries(vec![]).unwrap().id(),"opaque.bin".into(),&bytes).unwrap();
    let relation=Relation::of::<ArtifactChunk>();
    let source_relation=Relation::of::<SourceArtifact>();
    let mut specification=spec("bytes",&relation);specification.outputs.insert(source_relation.name().into());
    let contribution=store.begin_contribution(specification).await.unwrap();
    store.write_batch(&contribution,&source_relation,&SourceArtifact::encode(std::slice::from_ref(&source)).unwrap()).await.unwrap();
    let row=ArtifactChunk {artifact:source.id(),ordinal:0,body:EvidenceBytes(bytes.clone())};
    store.write_batch(&contribution,&relation,&ArtifactChunk::encode(std::slice::from_ref(&row)).unwrap()).await.unwrap();
    let views=store.complete_contribution(contribution,ProviderOutcome::Complete,&[relation.clone(),source_relation],&BTreeMap::new()).await.unwrap();
    let budget=ResourceBudget::fixed(32<<20).unwrap();
    let mut rows=store.scan_batches(&views[relation.name()],&relation,None,None,&budget,8).await.unwrap();
    assert_eq!(ArtifactChunk::decode(&rows.try_next().await.unwrap().unwrap()).unwrap()[0].body.0,bytes);
    assert!(rows.try_next().await.unwrap().is_none());
    let mut response=store.client().query("SELECT count() AS rows FROM original_chunk GROUP ALL").await.unwrap().check().unwrap();
    let counts:Vec<serde_json::Value>=response.take(0).unwrap();assert_eq!(counts,vec![serde_json::json!({"rows":16})]);
    let mut response=store.client().query("SELECT VALUE body FROM compiler_record").await.unwrap().check().unwrap();
    let metadata:Vec<lctx_surrealdb::surrealdb::types::Value>=response.take(0).unwrap();
    assert!(metadata.iter().all(|body|matches!(body,lctx_surrealdb::surrealdb::types::Value::Object(object) if !object.contains_key("body"))));
    store.verify_state().await.unwrap();
    let state=store.completed_state().await.unwrap();
    store.client().query("UPDATE compiler_record SET body.ordinal=1").await.unwrap().check().unwrap();
    assert!(matches!(store.verify_state().await,Err(lctx_model::domain::ModelError::Conflict("compiler original metadata"))));
    assert!(matches!(store.completed_state().await,Err(lctx_model::domain::ModelError::Conflict("compiler original metadata"))));
    store.client().query("UPDATE compiler_record SET body.ordinal=0").await.unwrap().check().unwrap();
    assert_eq!(store.completed_state().await.unwrap(),state);
    // Metadata is intact; the independently read physical byte owner must still agree.
    let mut response=store.client().query("SELECT * FROM original_chunk ORDER BY start LIMIT 1").await.unwrap().check().unwrap();
    let physical:Vec<lctx_surrealdb::surrealdb::types::Value>=response.take(0).unwrap();
    let lctx_surrealdb::surrealdb::types::Value::Object(chunk)=&physical[0] else{panic!("physical chunk");};
    let Some(lctx_surrealdb::surrealdb::types::Value::Bytes(original))=chunk.get("bytes") else{panic!("physical bytes");};
    let mut changed=original.to_vec();changed[0]^=1;
    let mut bindings=lctx_surrealdb::surrealdb::types::Variables::new();
    bindings.insert("id",chunk.get("id").unwrap().clone());bindings.insert("bytes",lctx_surrealdb::surrealdb::types::Bytes::from(changed.clone()));bindings.insert("content",ContentHash::of(&changed).hex());
    store.client().query("UPDATE $id SET bytes=$bytes,content=$content").bind(bindings).await.unwrap().check().unwrap();
    assert!(matches!(store.verify_state().await,Err(lctx_model::domain::ModelError::Conflict("compiler original digest"))));
    let mut bindings=lctx_surrealdb::surrealdb::types::Variables::new();
    bindings.insert("id",chunk.get("id").unwrap().clone());bindings.insert("bytes",original.clone());bindings.insert("content",ContentHash::of(original).hex());
    store.client().query("UPDATE $id SET bytes=$bytes,content=$content").bind(bindings).await.unwrap().check().unwrap();
    let other=store.begin_contribution(spec("conflict",&relation)).await.unwrap();
    let mut altered=row;altered.body.0[0]^=1;
    assert!(store.write_batch(&other,&relation,&ArtifactChunk::encode(&[altered]).unwrap()).await.is_err());
    assert!(store.check().is_err());
    store.abandon().await.unwrap();
}
