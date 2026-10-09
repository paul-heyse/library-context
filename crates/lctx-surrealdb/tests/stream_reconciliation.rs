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
fn assert_compact_index_plan(plan: &str, index: &str) {
    let scans = plan
        .lines()
        .filter(|line| line.trim_start().starts_with("IndexScan "))
        .collect::<Vec<_>>();
    assert_eq!(scans.len(), 1, "one equality driver: {plan}");
    assert!(
        scans[0].contains(&format!("index: {index},")),
        "required driver: {plan}"
    );
    assert!(
        scans[0].contains("access: ["),
        "single equality prefix: {plan}"
    );
    for forbidden in [
        "TableScan",
        "UnionIndexScan",
        "Union",
        "Sort",
        "TopK",
        "Aggregate",
        "Group",
        "Distinct",
        "Collect",
        "Materialize",
    ] {
        assert!(
            !plan
                .lines()
                .any(|line| line.trim_start().starts_with(forbidden)),
            "whole-match operator {forbidden}: {plan}"
        );
    }
}
async fn candidate_plan_controls(native: &NativeReader, release: &Release) {
    use lctx_surrealdb::prepared::{PreparedQuery, scope_constant};
    use lctx_surrealdb::surrealdb::types::{Bytes, Object};
    // Install the actual compiler declaration inventory; these controls exercise physical
    // access only, independently of completed-view admission and provider qualification.
    native
        .client()
        .query(lctx_surrealdb::compiler::compiler_schema())
        .await
        .unwrap()
        .check()
        .unwrap();
    let record = analytics::QualityStep {
        run: serde_json::from_value(serde_json::to_value([7u8; 16]).unwrap()).unwrap(),
        ordinal: 0,
        value: FiniteF64::new(0.5).unwrap(),
    };
    let relation = Relation::of::<analytics::QualityStep>();
    let body = lctx_surrealdb::codec::batch_bodies(
        &relation,
        &analytics::QualityStep::encode(std::slice::from_ref(&record)).unwrap(),
    )
    .unwrap()
    .pop()
    .unwrap();
    let canonical = serde_json::to_vec(&body).unwrap();
    let mut sink = KeySink::new("compiler-backing-key/v1");
    relation.name().to_string().encode(&mut sink);
    sink.part(b"key", record.id().bytes());
    let mut row = Object::new();
    row.insert("id", RecordId::new("compiler_record", sink.finish().hex()));
    row.insert("semantic_type", relation.name().to_string());
    row.insert("semantic_key", hex::encode(record.id().bytes()));
    row.insert("body", body);
    row.insert("canonical", Bytes::from(canonical.clone()));
    row.insert("content", ContentHash::of(&canonical).hex());
    let mut bindings = Variables::new();
    bindings.insert("row", row);
    native
        .client()
        .query("INSERT INTO compiler_record $row RETURN NONE")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    for (table, relation, field, value) in [
        (
            "entity",
            Release::NAME,
            "package",
            lctx_surrealdb::loader::json_value(serde_json::to_value(release.package).unwrap())
                .unwrap(),
        ),
        (
            "assertion",
            input::InputDistribution::NAME,
            "release",
            lctx_surrealdb::loader::json_value(serde_json::to_value(release.id()).unwrap())
                .unwrap(),
        ),
        (
            "compiler_record",
            analytics::QualityStep::NAME,
            "run",
            Value::from_t(vec![7i64; 16]),
        ),
    ] {
        let mut bindings = Variables::new();
        bindings.insert("relation", relation);
        bindings.insert("value", value);
        let preparation = vec![format!(
            "LET $__compiler_scope = {}",
            scope_constant("$relation", field, "$value")
        )];
        let query = format!(
            "SELECT semantic_type AS relation,semantic_key,id AS node FROM {table} WITH INDEX by_scope WHERE scope_keys CONTAINS $__compiler_scope AND semantic_type=$relation"
        );
        let plan: String = native
            .query_prepared(
                PreparedQuery::new(
                    bindings.clone(),
                    preparation.clone(),
                    vec![format!("EXPLAIN {query}")],
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_compact_index_plan(&plan, "by_scope");
        let mut rows = native
            .stream_prepared(PreparedQuery::new(bindings, preparation, vec![query]).unwrap())
            .unwrap();
        assert!(rows.next().await.unwrap().is_some());
        assert!(rows.next().await.unwrap().is_none());
    }
    let owner = RecordId::new("compiler_contribution", "plan_owner");
    let node = reader::target_id(graph::Target::Entity(graph::EntityId::of(release.id())));
    let mut row = Object::new();
    row.insert("id", RecordId::new("compiler_membership", "plan_member"));
    row.insert("contribution", owner.clone());
    row.insert("relation", Release::NAME);
    row.insert("semantic_key", hex::encode(release.id().bytes()));
    row.insert("node", node);
    row.insert("content", ContentHash::of(b"plan membership").hex());
    let mut bindings = Variables::new();
    bindings.insert("row", row);
    native
        .client()
        .query("INSERT INTO compiler_membership $row RETURN NONE")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut bindings = Variables::new();
    bindings.insert("owner", owner);
    bindings.insert("relation", Release::NAME);
    for query in [
        "SELECT relation,semantic_key,node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation",
        "SELECT node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner",
    ] {
        let plan: String = native
            .query(format!("EXPLAIN {query}"), bindings.clone())
            .await
            .unwrap();
        assert_compact_index_plan(&plan, "contribution_rows");
        let mut rows = native.query_stream(query, bindings.clone(), 1).unwrap();
        assert!(rows.next().await.unwrap().is_some());
        assert!(rows.next().await.unwrap().is_none());
    }
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
    let prepared = lctx_surrealdb::prepared::PreparedQuery::new(
        Variables::new(),
        vec!["LET $constant=7".into()],
        vec!["RETURN [$constant]".into()],
    )
    .unwrap();
    assert_eq!(
        native.query_prepared::<Vec<i64>>(prepared).await.unwrap(),
        vec![7]
    );
    assert!(
        native
            .query::<Vec<i64>>("LET $constant=7; RETURN [$constant]", Variables::new())
            .await
            .is_err(),
        "multiple statements require explicit result positions"
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
    let oversized_error = oversized.next().await.unwrap_err();
    assert!(
        matches!(
            oversized_error.primary(),
            Some(ModelError::Limit {
                owner: "native-stream",
                limit: "row bytes",
                observed,
                bound,
            }) if *bound == 1024 * 1024 && observed > bound
        ),
        "oversized native values fail before source expansion: {oversized_error}"
    );
    let repeated_error = oversized.next().await.unwrap_err();
    let (ModelError::SharedCause(first), ModelError::SharedCause(repeated)) =
        (&oversized_error, &repeated_error)
    else {
        panic!("failed reads retain their primary cause: {oversized_error}; {repeated_error}");
    };
    assert!(
        std::sync::Arc::ptr_eq(first, repeated),
        "a failed stream retains the same refusal"
    );

    let package = Package {
        name: "scope-fixture".into(),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
    let bytes = (0..70000)
        .map(|position| (position % 251) as u8)
        .collect::<Vec<_>>();
    let input = input::InputRevision::from_entries(vec![input::ManifestEntry {
        path: "original.py".into(),
        content: ContentHash::of(&bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let artifact =
        source::SourceArtifact::from_bytes(input.id(), "original.py".into(), &bytes).unwrap();
    let original = graph::Original {
        source: graph::EntityId::of(artifact.id()),
        content: artifact.content,
        byte_len: bytes.len() as u64,
    };
    let assertion = Assertion::from_record(input::InputDistribution {
        input: input.id(),
        release: release.id(),
        role: input::DistributionRole::FirstParty,
    })
    .unwrap();
    let mut entities = vec![
        Entity::from(package),
        Entity::from(release.clone()),
        Entity::from(input),
        Entity::from(artifact),
        Entity::from(value::Literal::Bytes {
            value: EvidenceBytes(vec![0xff, 0, 0x80]),
        }),
    ];
    entities.sort_by_key(Entity::id);
    loader.entities(&entities[..1]).await.unwrap();
    loader.ensure_entities(&entities).await.unwrap();
    loader.ensure_entities(&entities).await.unwrap();
    loader.entity_references(&entities).await.unwrap();
    loader
        .ensure_assertions(&[assertion.clone(), assertion.clone()])
        .await
        .unwrap();
    loader
        .ensure_assertions(std::slice::from_ref(&assertion))
        .await
        .unwrap();
    loader
        .assertion_references(std::slice::from_ref(&assertion))
        .await
        .unwrap();
    assert_eq!(
        native
            .records::<Release>(lctx_surrealdb::RecordSelection::Scope {
                field: "package".into(),
                values: vec![serde_json::to_value(release.package).unwrap()]
            })
            .await
            .unwrap(),
        vec![release.clone()]
    );
    candidate_plan_controls(&native, &release).await;

    loader
        .original_stream(
            original.source.0,
            original.content,
            original.byte_len,
            &mut bytes.as_slice(),
        )
        .await
        .unwrap();
    loader
        .ensure_original_stream(
            original.source.0,
            original.content,
            original.byte_len,
            &mut bytes.as_slice(),
        )
        .await
        .unwrap();
    let ranges = [
        (original.source, 65534, 4),
        (original.source, 7, 3),
        (original.source, 65534, 4),
    ];
    assert_eq!(
        native.original_bytes_batch(&ranges).await.unwrap(),
        vec![
            bytes[65534..65538].to_vec(),
            bytes[7..10].to_vec(),
            bytes[65534..65538].to_vec()
        ]
    );
    let mut assertion_hasher = FamilyHasher::new(GraphFamily::Assertions);
    assert!(
        assertion_hasher
            .push(assertion.id().0, assertion.content())
            .unwrap()
    );
    let mut hasher = FamilyHasher::new(GraphFamily::Entities);
    for entity in &entities {
        assert!(hasher.push(entity.id().0, entity.content()).unwrap());
    }
    let manifest = Manifest {
        admission_contract: ContentHash::of(b"fixture admission contract"),
        format_version: graph::ARTIFACT_FORMAT_VERSION,
        completed_state: completed::CompletedStateIdentity {
            format_version: completed::STATE_FORMAT_VERSION,
            contributions: 0,
            memberships: 0,
            backing_rows: 0,
            content: ContentHash::of(b"fixture-empty-completed-state"),
        },
        frontier: admission::Frontier::Facts,
        profile: stages::Profile::Catalog,
        captures: vec![],
        semantic_contract: ContentHash::of(b"fixture"),
        producers: vec![],
        settings: ContentHash::of(b"fixture"),
        families: vec![hasher.finish(), assertion_hasher.finish()],
        required_outcomes: vec![],
        outcomes: vec![],
        originals: vec![original.clone()],
        projections: vec![],
        embeddings: vec![],
    };
    loader.reconcile(&manifest).await.unwrap();
    // Same-key reuse compares the complete physical payload, not just the canonical digest.
    let mut assertion_bindings = Variables::new();
    assertion_bindings.insert(
        "id",
        reader::target_id(graph::Target::Assertion(assertion.id())),
    );
    let saved_assertion = physical_row(&native, assertion_bindings.clone()).await;
    assertion_bindings.insert("saved", saved_assertion.clone());
    client
        .query("UPDATE $id SET body.role=1")
        .bind(assertion_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        loader
            .ensure_assertions(std::slice::from_ref(&assertion))
            .await
            .is_err()
    );
    assert!(loader.reconcile(&manifest).await.is_err());
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(assertion_bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut chunk_bindings = Variables::new();
    chunk_bindings.insert(
        "id",
        RecordId::new("original_chunk", format!("{}_0", original.source.0.hex())),
    );
    let saved_chunk = physical_row(&native, chunk_bindings.clone()).await;
    chunk_bindings.insert("saved", saved_chunk);
    client
        .query("UPDATE $id SET bytes=b\"00\"")
        .bind(chunk_bindings.clone())
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        loader
            .ensure_original_stream(
                original.source.0,
                original.content,
                original.byte_len,
                &mut bytes.as_slice()
            )
            .await
            .is_err()
    );
    assert!(native.original_bytes_batch(&ranges).await.is_err());
    assert!(loader.reconcile(&manifest).await.is_err());
    client
        .query("UPDATE $id CONTENT $saved")
        .bind(chunk_bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    loader.reconcile(&manifest).await.unwrap();

    let mut bindings = Variables::new();
    bindings.insert(
        "id",
        reader::target_id(graph::Target::Entity(graph::EntityId::of(release.id()))),
    );
    let before = physical_row(&native, bindings.clone()).await;
    // Simulate imported derived-field drift by making the stored scope array independently writable.
    client.query("DEFINE FIELD OVERWRITE scope_keys ON entity TYPE array<string>; UPDATE $id SET scope_keys=['wrong'];").bind(bindings.clone()).await.unwrap().check().unwrap();
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
