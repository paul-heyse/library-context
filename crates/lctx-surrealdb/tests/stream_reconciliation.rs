//! Actual persistent gRPC completion and full canonical physical readback controls.
use lctx_model::domain::{
    graph::{Assertion, Entity, FamilyHasher, GraphFamily, Manifest},
    input::{Package, Release},
    serving::{DatabaseIdentity, Name, SnapshotHandle},
    *,
};
use lctx_surrealdb::surrealdb::types::{RecordId, Value, Variables};
use lctx_surrealdb::{Credentials, Loader, NativeReader, reader};

async fn physical_row(reader: &NativeReader, bindings: Variables) -> Value {
    let mut rows = reader
        .query_stream("SELECT * FROM $id", bindings, 1)
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(rows.next().await.unwrap().is_none());
    row
}
#[tokio::test]
async fn terminal_success_is_required_and_sparse_scope_corruption_is_rejected() {
    let cfg: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned persistent fixture"),
        )
        .unwrap(),
    )
    .unwrap();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let ns = "stream_controls";
    let db = format!("reconciliation_{}", std::process::id());
    let client = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        ns,
        &db,
    )
    .await
    .unwrap();
    client
        .query(format!(
            "DEFINE NAMESPACE IF NOT EXISTS {ns}; DEFINE DATABASE OVERWRITE {db} STRICT;"
        ))
        .await
        .unwrap()
        .check()
        .unwrap();
    let loader = Loader::new(client.clone());
    loader
        .install(&lctx_surrealdb::materialization::native_definitions())
        .await
        .unwrap();
    let native = NativeReader::new(
        client.clone(),
        SnapshotHandle {
            semantic: ContentHash::of(b"fixture"),
            realization: ContentHash::of(b"fixture"),
            database: DatabaseIdentity {
                namespace: Name::new(ns).unwrap(),
                database: Name::new(&db).unwrap(),
            },
        },
    );
    let mut rows = native
        .query_stream(
            "RETURN [{id:entity:first}]; THROW 'late-terminal-control';",
            Variables::new(),
            2,
        )
        .unwrap();
    assert!(
        rows.next().await.unwrap().is_some(),
        "earlier rows are provisional"
    );
    assert!(
        rows.next().await.is_err(),
        "later statement failure must reject the query"
    );
    assert!(
        rows.next().await.is_err(),
        "a failed stream cannot subsequently claim completion"
    );
    let mut incomplete = native
        .query_stream("RETURN [{id:entity:first}]", Variables::new(), 2)
        .unwrap();
    assert!(incomplete.next().await.unwrap().is_some());
    assert!(
        incomplete.next().await.is_err(),
        "missing declared terminal is refused"
    );
    // The first provisional row proves the SDK dispatched the real query. Cancellation
    // occurs while its second statement is pending, rather than before polling the stream.
    let (entered, started) = tokio::sync::oneshot::channel();
    let active_reader = native.clone();
    let active = tokio::spawn(async move {
        let mut rows = active_reader
            .query_stream(
                "RETURN [{id:entity:entered}]; SLEEP 30s; RETURN [{id:entity:late}];",
                Variables::new(),
                3,
            )
            .unwrap();
        let row = rows
            .next()
            .await
            .unwrap()
            .expect("actual provisional native row");
        assert_eq!(
            row.as_object().unwrap().get("id"),
            Some(&Value::RecordId(RecordId::new("entity", "entered")))
        );
        entered.send(()).unwrap();
        // There is no second result until the pending native statement completes.
        let _ = rows.next().await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), started)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !active.is_finished(),
        "native operation must still be pending at cancellation"
    );
    active.abort();
    assert!(active.await.unwrap_err().is_cancelled());
    let mut fresh = native
        .query_stream("RETURN [{id:entity:fresh}]", Variables::new(), 1)
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        assert!(fresh.next().await.unwrap().is_some());
        assert!(fresh.next().await.unwrap().is_none());
        assert!(fresh.next().await.unwrap().is_none());
    })
    .await
    .expect("fresh checked query completes after the cancelled task drains");
    let mut oversized = native
        .query_stream(
            "RETURN [{id:entity:large,value:string::repeat('x',1048576)}]",
            Variables::new(),
            1,
        )
        .unwrap();
    assert!(
        matches!(
            oversized.next().await,
            Err(ModelError::Limit {
                owner: "native-stream",
                ..
            })
        ),
        "oversized native values fail before source expansion"
    );

    let package = Package {
        name: "scope-fixture".into(),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
    let bytes=(0..70000).map(|position|(position%251) as u8).collect::<Vec<_>>();
    let input=input::InputRevision::from_entries(vec![input::ManifestEntry{path:"original.py".into(),content:ContentHash::of(&bytes),byte_len:bytes.len() as i64}]).unwrap();
    let artifact=source::SourceArtifact::from_bytes(input.id(),"original.py".into(),&bytes).unwrap();
    let original=graph::Original{source:graph::EntityId::of(artifact.id()),content:artifact.content,byte_len:bytes.len() as u64};
    let assertion=Assertion::from_record(input::InputDistribution{input:input.id(),release:release.id(),role:input::DistributionRole::FirstParty}).unwrap();
    let mut entities=vec![Entity::from(package),Entity::from(release.clone()),Entity::from(input),Entity::from(artifact),Entity::from(value::Literal::Bytes{value:EvidenceBytes(vec![0xff,0,0x80])})];
    entities.sort_by_key(Entity::id);
    loader.entities(&entities[..1]).await.unwrap();
    loader.ensure_entities(&entities).await.unwrap();
    loader.ensure_entities(&entities).await.unwrap();
    loader.entity_references(&entities).await.unwrap();
    loader.ensure_assertions(&[assertion.clone(),assertion.clone()]).await.unwrap();
    loader.ensure_assertions(std::slice::from_ref(&assertion)).await.unwrap();
    loader.assertion_references(std::slice::from_ref(&assertion)).await.unwrap();
    loader.original_stream(original.source.0,original.content,original.byte_len,&mut bytes.as_slice()).await.unwrap();
    loader.ensure_original_stream(original.source.0,original.content,original.byte_len,&mut bytes.as_slice()).await.unwrap();
    let ranges=[(original.source,65534,4),(original.source,7,3),(original.source,65534,4)];
    assert_eq!(native.original_bytes_batch(&ranges).await.unwrap(),vec![bytes[65534..65538].to_vec(),bytes[7..10].to_vec(),bytes[65534..65538].to_vec()]);
    let mut assertion_hasher=FamilyHasher::new(GraphFamily::Assertions);assert!(assertion_hasher.push(assertion.id().0,assertion.content()).unwrap());
    let mut hasher = FamilyHasher::new(GraphFamily::Entities);
    for entity in &entities {
        assert!(hasher.push(entity.id().0, entity.content()).unwrap());
    }
    let manifest = Manifest {
        format_version: graph::ARTIFACT_FORMAT_VERSION,
        completed_state: completed::CompletedStateIdentity {
            format_version: completed::STATE_FORMAT_VERSION,
            contributions:0,memberships:0,backing_rows:0,
            content:ContentHash::of(b"fixture-empty-completed-state"),
        },
        frontier: admission::Frontier::Facts,
        profile: stages::Profile::Catalog,
        captures: vec![],
        semantic_contract: ContentHash::of(b"fixture"),
        producers: vec![],
        settings: ContentHash::of(b"fixture"),
        families: vec![
            hasher.finish(),
            assertion_hasher.finish(),
        ],
        required_outcomes: vec![],
        outcomes: vec![],
        originals: vec![original.clone()],
        projections: vec![],
        embeddings: vec![],
    };
    loader.reconcile(&manifest).await.unwrap();
    // Same-key reuse compares the complete physical payload, not just the canonical digest.
    let mut assertion_bindings=Variables::new();assertion_bindings.insert("id",reader::target_id(graph::Target::Assertion(assertion.id())));
    let saved_assertion=physical_row(&native,assertion_bindings.clone()).await;
    assertion_bindings.insert("saved",saved_assertion.clone());
    client.query("UPDATE $id SET body.role=1").bind(assertion_bindings.clone()).await.unwrap().check().unwrap();
    assert!(loader.ensure_assertions(std::slice::from_ref(&assertion)).await.is_err());
    assert!(loader.reconcile(&manifest).await.is_err());
    client.query("UPDATE $id CONTENT $saved").bind(assertion_bindings).await.unwrap().check().unwrap();
    let mut chunk_bindings=Variables::new();chunk_bindings.insert("id",RecordId::new("original_chunk",format!("{}_0",original.source.0.hex())));
    let saved_chunk=physical_row(&native,chunk_bindings.clone()).await;chunk_bindings.insert("saved",saved_chunk);
    client.query("UPDATE $id SET bytes=b\"00\"").bind(chunk_bindings.clone()).await.unwrap().check().unwrap();
    assert!(loader.ensure_original_stream(original.source.0,original.content,original.byte_len,&mut bytes.as_slice()).await.is_err());
    assert!(native.original_bytes_batch(&ranges).await.is_err());
    assert!(loader.reconcile(&manifest).await.is_err());
    client.query("UPDATE $id CONTENT $saved").bind(chunk_bindings).await.unwrap().check().unwrap();
    loader.reconcile(&manifest).await.unwrap();

    let mut bindings = Variables::new();
    bindings.insert(
        "id",
        reader::target_id(graph::Target::Entity(graph::EntityId::of(release.id()))),
    );
    let before = physical_row(&native, bindings.clone()).await;
    // Simulate persisted drift: restore the declared VALUE expression without rewriting rows.
    client.query("DEFINE FIELD OVERWRITE scope_package ON entity TYPE option<string>; UPDATE $id SET scope_package='wrong'; DEFINE FIELD OVERWRITE scope_package ON entity TYPE option<string> VALUE IF body.package IS NONE THEN NONE ELSE <string>body.package END;").bind(bindings.clone()).await.unwrap().check().unwrap();
    assert!(loader.ensure_entities(&entities).await.is_err());
    assert!(loader.reconcile(&manifest).await.is_err());
    let after = physical_row(&native, bindings.clone()).await;
    assert_eq!(
        before.as_object().unwrap().get("canonical"),
        after.as_object().unwrap().get("canonical")
    );
    // Readback is read-only: the invalid persisted value remains after the refusal.
    assert_ne!(before, after);
    bindings.insert("saved", before.clone());
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    client
        .query("UPDATE $id SET canonical=b\"00\"")
        .bind(bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(matches!(
        native
            .records::<Release>(lctx_surrealdb::RecordSelection::Keys(vec![
                *release.id().bytes()
            ]))
            .await,
        Err(ModelError::Serving(serving::FailureKind::Corrupt))
    ));
    assert!(matches!(
        loader.reconcile(&manifest).await,
        Err(ModelError::Serving(serving::FailureKind::Corrupt))
    ));
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    loader.reconcile(&manifest).await.unwrap();
    client
        .query(format!("REMOVE DATABASE {db}"))
        .await
        .unwrap()
        .check()
        .unwrap();
    client.invalidate().await.unwrap();
}
