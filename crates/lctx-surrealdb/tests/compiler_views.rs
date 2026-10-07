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
    store.write_batch(&contribution,&relation,&QualityStep::encode(std::slice::from_ref(&row)).unwrap()).await.unwrap();
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
async fn cold_backing_rejects_coherently_renamed_membership_identity() {
    use lctx_model::domain::ModelError;
    use lctx_surrealdb::surrealdb::types::{RecordId, Value, Variables};

    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let relation=Relation::of::<Package>();
    let rows=vec![Package{name:"membership-first".into()},Package{name:"membership-second".into()}];
    let contribution=store.begin_contribution(spec("membership-identity",&relation)).await.unwrap();
    store.write_batch(&contribution,&relation,&Package::encode(&rows).unwrap()).await.unwrap();
    store.complete_contribution(contribution,ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new()).await.unwrap();
    store.verify_state().await.unwrap();

    let mut response=store.client().query("SELECT * FROM compiler_membership ORDER BY semantic_key").await.unwrap().check().unwrap();
    let memberships:Vec<Value>=response.take(0).unwrap();
    assert_eq!(memberships.len(),2);
    let Value::Object(mut payload)=memberships[0].clone() else{panic!("native membership object");};
    let original=payload.remove("id").unwrap();
    let renamed=RecordId::new("compiler_membership",ContentHash::of(b"coherently-renamed-membership").hex());
    assert_ne!(original,Value::RecordId(renamed.clone()));
    let mut bindings=Variables::new();
    bindings.insert("original",original);
    bindings.insert("renamed",renamed.clone());
    bindings.insert("payload",Value::Object(payload.clone()));
    // Delete before create preserves the unique owner/relation/key index. The immutable
    // payload and contribution claims remain intact; only the physical lookup ID changes.
    store.client().query("BEGIN TRANSACTION; DELETE $original; CREATE $renamed CONTENT $payload; COMMIT TRANSACTION;").bind(bindings).await.unwrap().check().unwrap();
    let mut response=store.client().query("SELECT * FROM compiler_membership ORDER BY semantic_key").await.unwrap().check().unwrap();
    let memberships:Vec<Value>=response.take(0).unwrap();
    assert_eq!(memberships.len(),2);
    let Value::Object(mut observed)=memberships[0].clone() else{panic!("native membership object");};
    assert_eq!(observed.remove("id"),Some(Value::RecordId(renamed)));
    assert_eq!(observed,payload,"renaming preserves every membership claim and backing pointer");
    assert!(matches!(store.verify_state().await,Err(ModelError::Conflict("compiler membership physical identity"))));
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

#[tokio::test(flavor = "multi_thread")]
async fn overlapping_membership_windows_count_distinct_keys_across_contributors() {
    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let relation=Relation::of::<Package>();
    let mut views=BTreeMap::new();
    let mut frozen=Vec::new();
    for (producer,range,expected) in [("a",0..200,200),("b",120..400,400),("c",150..450,450),("empty",0..0,450)] {
        let mut specification=spec(producer,&relation);
        if let Some(view)=views.get(relation.name()) {
            specification.inputs.push(lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(&relation,specification.model,view).unwrap());
        }
        let contribution=store.begin_contribution(specification).await.unwrap();
        let rows=range.rev().map(|index|Package{name:format!("window-{index:04}")}).collect::<Vec<_>>();
        if !rows.is_empty(){store.write_batch(&contribution,&relation,&Package::encode(&rows).unwrap()).await.unwrap();}
        views=store.complete_contribution(contribution,ProviderOutcome::Complete,std::slice::from_ref(&relation),&views).await.unwrap();
        let view=views[relation.name()].clone();
        assert_eq!(view.rows,expected,"union cardinality for {producer}");
        frozen.push(view);
    }
    let budget=ResourceBudget::fixed(32<<20).unwrap();
    for (view,expected) in frozen.iter().zip([200,400,450,450]) {
        let mut stream=store.scan_batches(view,&relation,None,None,&budget,37).await.unwrap();
        let mut observed=BTreeSet::new();
        while let Some(batch)=stream.try_next().await.unwrap(){
            for row in Package::decode(&batch).unwrap(){assert!(observed.insert(row.name));}
        }
        let expected=(0..expected).map(|index|format!("window-{index:04}")).collect::<BTreeSet<_>>();
        assert_eq!(observed,expected,"frozen membership retains its own exact union");
    }
    store.verify_state().await.unwrap();
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn large_exact_key_and_atomic_field_selections_preserve_frozen_membership() {
    use arrow_array::{FixedSizeBinaryArray, StringArray};
    use lctx_model::domain::{graph::Entity, input::Release};
    use lctx_surrealdb::compiler_provider::{select_field_table, select_table};
    use lctx_surrealdb::surrealdb::types::{Number, Value};
    use std::sync::Arc;

    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let relation=Relation::of::<Release>();
    let packages=(0..180).map(|index|Package{name:format!("selection-{index:04}")}).collect::<Vec<_>>();
    lctx_surrealdb::Loader::new(store.shared_client()).entities(&packages.iter().cloned().map(Entity::from).collect::<Vec<_>>()).await.unwrap();
    let releases=(0..720).map(|index|Release{package:packages[index/4].id(),version:(index%4).to_string()}).collect::<Vec<_>>();
    let mut views=BTreeMap::new();
    for (producer,range) in [("selection-a",0..360),("selection-b",240..600)] {
        let mut specification=spec(producer,&relation);
        if let Some(view)=views.get(relation.name()) {
            specification.inputs.push(lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(&relation,specification.model,view).unwrap());
        }
        let contribution=store.begin_contribution(specification).await.unwrap();
        let rows=range.rev().map(|index|releases[index].clone()).collect::<Vec<_>>();
        store.write_batch(&contribution,&relation,&Release::encode(&rows).unwrap()).await.unwrap();
        views=store.complete_contribution(contribution,ProviderOutcome::Complete,std::slice::from_ref(&relation),&views).await.unwrap();
    }
    let frozen=views[relation.name()].clone();
    assert_eq!(frozen.rows,600,"identical releases from both completed owners count once");
    let mut specification=spec("selection-newer",&relation);
    specification.inputs.push(lctx_model::domain::analysis::sources::SourceSnapshot::of_completed_view(&relation,specification.model,&frozen).unwrap());
    let newer=store.begin_contribution(specification).await.unwrap();
    store.write_batch(&newer,&relation,&Release::encode(&releases[480..]).unwrap()).await.unwrap();

    let mut nominal_keys=(30..680).rev().filter(|index|index%5!=0).map(|index|*releases[index].id().bytes()).collect::<Vec<_>>();
    nominal_keys.extend((100..160).filter(|index|index%5!=0).map(|index|*releases[index].id().bytes()));
    nominal_keys.push(*Release{package:packages[0].id(),version:"missing".into()}.id().bytes());
    let missing_package=Package{name:"selection-missing".into()};
    let mut field_keys=packages[10..175].iter().map(|package|*package.id().bytes()).collect::<Vec<_>>();
    field_keys.push(*packages[179].id().bytes());
    field_keys.push(*missing_package.id().bytes());
    field_keys.sort_unstable();
    let mut field_values=field_keys.iter().rev().chain(field_keys[..40].iter()).map(|key|Value::Array(key.iter().map(|byte|Value::Number(Number::Int(i64::from(*byte)))).collect())).collect::<Vec<_>>();
    // The repeated values are separated in the demand, and several real values only belong
    // to the newer owner. Neither repetition nor physical presence changes frozen membership.
    field_values.reverse();
    let mut expected_keys=(30..600).filter(|index|index%5!=0).map(|index|releases[index].clone()).collect::<Vec<_>>();
    expected_keys.sort_by_key(|row|*row.id().bytes());
    let mut expected_fields=releases[40..600].to_vec();
    expected_fields.sort_by_key(|row|*row.id().bytes());
    let budget=ResourceBudget::fixed(32<<20).unwrap();
    for completed in [false,true] {
        if completed {
            views=store.complete_contribution(newer,ProviderOutcome::Complete,std::slice::from_ref(&relation),&views).await.unwrap();
            assert_eq!(views[relation.name()].rows,720);
        }
        for (predicate,expected) in [
            (NativePredicate::Keys(nominal_keys.clone()),&expected_keys),
            (NativePredicate::Field{field:"package".into(),values:field_values.clone()},&expected_fields),
        ] {
            let mut stream=store.scan_batches(&frozen,&relation,None,Some(predicate),&budget,37).await.unwrap();
            let mut observed=Vec::new();
            while let Some(batch)=stream.try_next().await.unwrap(){observed.extend(Release::decode(&batch).unwrap());}
            assert_eq!(&observed,expected,"selected records remain sorted and exact with newer owner completed={completed}");
        }
    }

    nominal_keys.sort_unstable();
    nominal_keys.dedup();
    for (table_name,field_selection) in [("nominal_selected",false),("field_selected",true)] {
        let provider=store.table_provider(&frozen,relation.clone(),budget.clone(),37).unwrap();
        let charge=Arc::new(lctx_model::domain::charged::StateCharge::new(&budget,"selected-release-fixture"));
        let provider=if field_selection {
            select_field_table(&provider,"package",Arc::new(field_keys.clone()),charge).unwrap().unwrap()
        } else {
            select_table(&provider,Arc::new(nominal_keys.clone()),charge).unwrap().unwrap()
        };
        let session=datafusion::prelude::SessionContext::new();
        session.register_table(table_name,provider).unwrap();
        let batches=session.sql(&format!("SELECT id, version FROM {table_name} WHERE version = '2'")).await.unwrap().collect().await.unwrap();
        let mut observed=Vec::new();
        for batch in batches {
            assert_eq!(batch.schema().fields().iter().map(|field|field.name().as_str()).collect::<Vec<_>>(),vec!["id","version"]);
            let ids=batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().unwrap();
            let versions=batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
            for index in 0..batch.num_rows(){observed.push((ids.value(index).to_vec(),versions.value(index).to_owned()));}
        }
        let expected=if field_selection {&expected_fields}else{&expected_keys}.iter().filter(|row|row.version=="2").map(|row|(row.id().bytes().to_vec(),row.version.clone())).collect::<Vec<_>>();
        assert_eq!(observed,expected,"projected {table_name} retains exact membership, static filtering and unique sorted output");
    }
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn canonical_graph_scans_select_completed_families_and_one_hop_aliases() {
    use lctx_model::domain::{FiniteF64,analytics::QualityStep,graph::Entity};
    use lctx_surrealdb::surrealdb::types::{Bytes,RecordId,SurrealValue,Value,Variables};
    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Normalized).await.unwrap();
    let relation=Relation::of::<Package>();
    let first=Package{name:"completed".into()};
    let alias=Package{name:"one-hop-alias".into()};
    let pending=Package{name:"pending-unselected".into()};
    let quality=QualityStep{run:serde_json::from_value(serde_json::json!(vec![8u8;16])).unwrap(),ordinal:0,value:FiniteF64::new(0.25).unwrap()};
    let metadata=Relation::of::<QualityStep>();
    let mut specification=spec("complete",&relation);specification.outputs.insert(metadata.name().into());
    let complete=store.begin_contribution(specification).await.unwrap();
    store.write_batch(&complete,&relation,&Package::encode(std::slice::from_ref(&first)).unwrap()).await.unwrap();
    store.write_batch(&complete,&metadata,&QualityStep::encode(&[quality]).unwrap()).await.unwrap();
    store.complete_contribution(complete,ProviderOutcome::Complete,&[relation.clone(),metadata],&BTreeMap::new()).await.unwrap();
    let uncompleted=store.begin_contribution(spec("pending",&relation)).await.unwrap();
    store.write_batch(&uncompleted,&relation,&Package::encode(&[alias.clone(),pending]).unwrap()).await.unwrap();
    // Mechanical one-hop selection: only a completed source can admit its intrinsic target.
    let first_id=lctx_model::domain::graph::EntityId::of(first.id());
    let alias_id=lctx_model::domain::graph::EntityId::of(alias.id());
    let mut variables=Variables::new();variables.insert("source",RecordId::new("entity",first_id.0.hex()));variables.insert("target",RecordId::new("entity",alias_id.0.hex()));
    store.client().query("CREATE compiler_alias:selection SET source=$source,target=$target").bind(variables).await.unwrap().check().unwrap();
    let mut rows=store.scan_canonical(true).await.unwrap();
    let mut names=BTreeSet::new();
    while let Some(row)=rows.next().await.unwrap(){
        let Value::Object(object)=row else{panic!("canonical graph record");};
        let bytes=Bytes::from_value(object.get("canonical").unwrap().clone()).unwrap();
        let entity:Entity=serde_json::from_slice(&bytes).unwrap();
        let Entity::Package(package)=entity else{panic!("only selected packages");};
        names.insert(package.name);
    }
    assert_eq!(names,BTreeSet::from([first.name,alias.name]));
    let mut headers=store.scan_graph_headers(true).await.unwrap();
    let mut count=0;
    while headers.next().await.unwrap().is_some(){count+=1;}
    assert_eq!(count,2,"nongraph backing and unrelated pending entities stay excluded");
    let mut assertions=store.scan_canonical(false).await.unwrap();
    assert!(assertions.next().await.unwrap().is_none());
    store.abandon().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn exact_empty_reads_retain_view_shape_and_lifecycle_checks() {
    use lctx_model::domain::completed::CompletedView;
    let path=std::path::PathBuf::from(std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned persistent native fixture"));
    let config=RuntimeConfig::read(&path).unwrap();
    let store=NativeCompilerStore::begin(&config,lctx_model::domain::admission::Frontier::Facts).await.unwrap();
    let relation=Relation::of::<Package>();
    let empty=store.begin_contribution(spec("empty-read",&relation)).await.unwrap();
    let views=store.complete_contribution(empty,ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new()).await.unwrap();
    let frozen=views[relation.name()].clone();
    assert_eq!(frozen.rows,0);
    assert!(store.scan_rows(&frozen,&relation,None,None).await.unwrap().next().await.unwrap().is_none());
    let invalid=vec!["missing_field".to_owned()];
    assert!(store.scan_rows(&frozen,&relation,Some(&invalid),None).await.is_err());
    let invalid=NativePredicate::Field{field:"missing_field".into(),values:vec![]};
    assert!(store.scan_rows(&frozen,&relation,None,Some(invalid)).await.is_err());
    let forged=CompletedView::new(relation.name().into(),frozen.contributions.clone(),1).unwrap();
    assert!(store.scan_rows(&forged,&relation,None,Some(NativePredicate::Keys(vec![]))).await.is_err());
    let next=store.begin_contribution(spec("populated-read",&relation)).await.unwrap();
    store.write_batch(&next,&relation,&Package::encode(&[Package{name:"present".into()}]).unwrap()).await.unwrap();
    let current=store.complete_contribution(next,ProviderOutcome::Complete,std::slice::from_ref(&relation),&views).await.unwrap();
    let view=&current[relation.name()];
    assert_eq!(view.rows,1);
    for predicate in [NativePredicate::Keys(vec![]),NativePredicate::Field{field:"name".into(),values:vec![]}] {
        assert!(store.scan_rows(view,&relation,None,Some(predicate)).await.unwrap().next().await.unwrap().is_none());
    }
    // An empty physical demand is also preserved through the shared selected provider.
    let budget=ResourceBudget::fixed(32<<20).unwrap();
    let provider=store.table_provider(view,relation.clone(),budget.clone(),8).unwrap();
    let charge=std::sync::Arc::new(lctx_model::domain::charged::StateCharge::new(&budget,"empty-key-owner"));
    let provider=lctx_surrealdb::compiler_provider::select_table(&provider,std::sync::Arc::new(vec![]),charge).unwrap().unwrap();
    let session=datafusion::prelude::SessionContext::new();
    session.register_table("selected",provider).unwrap();
    let batches=session.sql("SELECT * FROM selected").await.unwrap().collect().await.unwrap();
    assert_eq!(batches.iter().map(|batch|batch.num_rows()).sum::<usize>(),0);
    // Newer members do not widen the exact completed empty view.
    assert!(store.scan_rows(&frozen,&relation,None,None).await.unwrap().next().await.unwrap().is_none());
    store.end_writes().await.unwrap();
    assert!(store.scan_rows(&frozen,&relation,None,None).await.is_err());
    assert!(store.scan_rows(view,&relation,None,Some(NativePredicate::Keys(vec![]))).await.is_err());
    store.abandon().await.unwrap();
}
