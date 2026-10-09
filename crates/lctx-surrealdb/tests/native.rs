use lctx_model::domain::{
    embedding::{
        Spec,
        cache::{CacheValue, EmbeddingCache},
    },
    graph::{Entity, EntityId, Target},
    input::{Package, Release},
    serving::{DatabaseIdentity, Name, SnapshotHandle},
    *,
};
use lctx_surrealdb::{
    Credentials, Loader, NativeEmbeddingCache, NativeReader, RecordSelection, reader,
};
use surrealdb::types::{Bytes, Value, Variables};
fn config() -> serde_json::Value {
    let path = std::env::var("LCTX_SURREAL_TEST_CONFIG")
        .expect("owned disposable SurrealDB configuration is required");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
#[tokio::test]
async fn prepared_streams_share_authenticated_session_and_keep_it_alive_through_drainage() {
    let cfg = config();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let client = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        cfg["namespace"].as_str().unwrap(),
        cfg["database"].as_str().unwrap(),
    ).await.unwrap();
    // session::id is the server-assigned RPC session UUID, not the authenticated user.
    let sql = "RETURN { id: session::id(), ns: session::ns(), db: session::db() }";
    let native = NativeReader::private(client.clone());
    let expected: Value = native.query_native(sql, Variables::new()).await.unwrap();
    let Value::Object(session) = &expected else { panic!("session descriptor must be an object") };
    assert!(matches!(session.get("id"), Some(Value::Uuid(_))));
    assert_eq!(session.get("ns"), Some(&Value::String(cfg["namespace"].as_str().unwrap().to_owned())));
    assert_eq!(session.get("db"), Some(&Value::String(cfg["database"].as_str().unwrap().to_owned())));
    // Revealing contrast: cloning Surreal itself recreates a distinct authenticated session.
    let independent = NativeReader::private(std::sync::Arc::new(client.as_ref().clone()));
    let other: Value = independent.query_native(sql, Variables::new()).await.unwrap();
    let Value::Object(other) = other else { panic!("independent session descriptor") };
    assert_ne!(session.get("id"), other.get("id"));
    drop(independent);

    let make = || lctx_surrealdb::prepared::PreparedQuery::new(
        Variables::new(), vec![], vec![sql.into(), sql.into()],
    ).unwrap();
    let first = native.stream_prepared(make()).unwrap();
    let second = native.stream_prepared(make()).unwrap();
    let mut late = native.stream_prepared(lctx_surrealdb::prepared::PreparedQuery::from_sql(
        format!("{sql}; THROW 'prepared late failure';"), Variables::new(), 2, vec![0],
    ).unwrap()).unwrap();
    let retained = std::sync::Arc::downgrade(&client);
    drop(native);
    drop(client);
    assert!(retained.upgrade().is_some(), "streams retain their original session after reader drop");
    async fn consume(mut rows: lctx_surrealdb::reader::NativeRows, expected: &Value) {
        assert_eq!(rows.next().await.unwrap().as_ref(), Some(expected));
        assert_eq!(rows.next().await.unwrap().as_ref(), Some(expected));
        assert!(rows.next().await.unwrap().is_none());
        rows.drain_transport().await.unwrap();
    }
    tokio::join!(consume(first, &expected), consume(second, &expected));
    assert_eq!(late.next().await.unwrap().as_ref(), Some(&expected));
    let error = late.next().await.unwrap_err();
    assert!(format!("{error:?}").contains("prepared late failure"));
    assert!(late.next().await.is_err(), "a late failure remains sticky");
    late.drain_transport().await.unwrap();
    drop(late);
    assert!(retained.upgrade().is_none(), "drained streams release the session handle");
}
#[tokio::test]
async fn native_codec_graph_search_and_immutable_winners() {
    let cfg = config();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let ns = "gn_controls";
    let db = format!("native_{}", std::process::id());
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
    let package = Package {
        name: "native-fixture".into(),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
    loader
        .entities(&[Entity::from(package.clone()), Entity::from(release.clone())])
        .await
        .unwrap();
    loader
        .entity_references(&[Entity::from(release.clone())])
        .await
        .unwrap();
    let handle = SnapshotHandle {
        semantic: ContentHash::of(b"fixture"),
        realization: lctx_surrealdb::schema::realization_identity(
            &lctx_surrealdb::materialization::native_definitions(),
        ),
        database: DatabaseIdentity {
            namespace: Name::new(ns).unwrap(),
            database: Name::new(&db).unwrap(),
        },
    };
    let reader = NativeReader::new(client.clone(), handle);
    // This graph-capable proof owner is outside mandatory Catalog publication.
    // Nominal endpoints are codec inputs; this control asserts no semantic admission.
    let premise = analysis::analytic::AnalysisDerivationPremise {
        derivation: serde_json::from_value(serde_json::json!(vec![31u8; 16])).unwrap(),
        source: serde_json::from_value(serde_json::json!(vec![37u8; 16])).unwrap(),
    };
    assert!(
        !catalog_frontier_relations()
            .iter()
            .any(|relation| relation.name() == analysis::analytic::AnalysisDerivationPremise::NAME)
    );
    let assertion = graph::Assertion::from_record(premise.clone()).unwrap();
    assert_eq!(
        lctx_surrealdb::codec::assertion_record::<analysis::analytic::AnalysisDerivationPremise>(
            &assertion
        )
        .unwrap(),
        premise
    );
    let native_view =
        lctx_surrealdb::codec::assertion_views(std::slice::from_ref(&assertion)).unwrap();
    assert_eq!(
        native_view[0].semantic_type,
        analysis::analytic::AnalysisDerivationPremise::NAME
    );
    assert_eq!(
        native_view[0].semantic_key,
        hex::encode(premise.id().bytes())
    );
    loader
        .assertions(std::slice::from_ref(&assertion))
        .await
        .unwrap();
    assert_eq!(
        reader
            .records::<analysis::analytic::AnalysisDerivationPremise>(RecordSelection::Keys(vec![
                *premise.id().bytes()
            ]))
            .await
            .unwrap(),
        vec![premise]
    );
    assert_eq!(
        reader
            .records::<Release>(RecordSelection::Keys(vec![*release.id().bytes()]))
            .await
            .unwrap(),
        vec![release.clone()]
    );
    assert_eq!(
        reader
            .records::<Release>(RecordSelection::Scope {
                field: "package".into(),
                values: vec![serde_json::to_value(package.id()).unwrap()]
            })
            .await
            .unwrap(),
        vec![release]
    );
    let mut b = Variables::new();
    b.insert(
        "key",
        reader::target_id(Target::Entity(EntityId::of(package.id()))),
    );
    let adjacency: Vec<String> = reader
        .query("SELECT VALUE semantic_type FROM $key<-reference<-entity", b)
        .await
        .unwrap();
    assert_eq!(adjacency, vec!["releases"]);
    // Raw SQL preserves flexible bodies; independent admission owns semantic closure.
    client.query("CREATE entity:bad CONTENT {semantic_type:'packages',semantic_key:'bad',kind:29,subtype:null,content:'bad',canonical:b\"00\",scope_keys:[],body:{__type:'packages',name:'bad',extra:1}};").await.unwrap().check().unwrap();
    client
        .query("DELETE entity:bad")
        .await
        .unwrap()
        .check()
        .unwrap();
    let enforced=client.query("RELATE assertion:absent->participant:dangling->entity:absent CONTENT {field:'missing',role:0,position:null};").await.unwrap().check();
    assert!(enforced.is_err());
    let cache = NativeEmbeddingCache::install(client.clone()).await.unwrap();
    let spec: Spec = serde_json::from_slice(include_bytes!(
        "../../../specs/embedding/qwen3-embedding-8b.json"
    ))
    .unwrap();
    let mut vector = vec![0f32; usize::try_from(spec.dimensions).unwrap()];
    vector[0] = 1.;
    let first = CacheValue {
        input_hash: ContentHash::of(b"text"),
        vector: vector.clone(),
        admitted_tokens: 3,
    };
    let winner = cache
        .admit(&spec, std::slice::from_ref(&first))
        .await
        .unwrap();
    assert_eq!(winner[&first.input_hash].vector, vector);
    vector[0] = 0.;
    vector[1] = 1.;
    let later = CacheValue {
        vector,
        admitted_tokens: 4,
        ..first.clone()
    };
    assert!(
        cache
            .admit(&spec, std::slice::from_ref(&later))
            .await
            .is_err()
    );
    let winner = cache.cached(&spec, &[first.input_hash]).await.unwrap();
    assert_eq!(winner[&first.input_hash].vector, first.vector);
    assert_eq!(winner[&first.input_hash].admitted_tokens, 3);
    let winner = cache
        .admit(
            &spec,
            &[CacheValue {
                admitted_tokens: 3,
                ..later
            }],
        )
        .await
        .unwrap();
    assert_eq!(winner[&first.input_hash].vector, first.vector);
    client
        .query(format!("REMOVE DATABASE {db}"))
        .await
        .unwrap()
        .check()
        .unwrap();
}

/// Flexible native bodies preserve logical text, bytes and explicit inactive NULL fields.
#[tokio::test]
async fn native_binary_backed_text_preserves_flexible_bodies() {
    use lctx_model::domain::{
        retrieval::{CorpusText, Family, RENDER_VERSION, Unit},
        value::Literal,
    };
    let cfg = config();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let ns = "gn_controls";
    let db = format!("text_codec_{}", std::process::id());
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
    loader.install("").await.unwrap();
    let text = "Scenario intent=Demonstration\n```python\nconnect('雪')\n```\n";
    let corpus = CorpusText {
        family: Family::Scenario,
        rendering_version: RENDER_VERSION,
        digest: ContentHash::of(text.as_bytes()),
        text: text.into(),
    };
    // These nominal keys are codec inputs; no graph endpoint/semantic admission is asserted.
    let unit = Unit {
        input: serde_json::from_value(serde_json::to_value([1u8; 16]).unwrap()).unwrap(),
        context: serde_json::from_value(serde_json::to_value([2u8; 16]).unwrap()).unwrap(),
        family: Family::Scenario,
        origin: serde_json::from_value(serde_json::to_value([3u8; 16]).unwrap()).unwrap(),
        corpus: corpus.id(),
        title: "Usage scenario 雪".into(),
    };
    let string = Literal::String {
        value: "exact 雪\n\0text".into(),
    };
    let opaque = Literal::Bytes {
        value: EvidenceBytes(vec![0xff, 0, 0x80]),
    };
    loader
        .entities(&[
            Entity::from(corpus.clone()),
            Entity::from(unit.clone()),
            Entity::from(string.clone()),
            Entity::from(opaque.clone()),
        ])
        .await
        .unwrap();
    let native = NativeReader::new(
        client.clone(),
        SnapshotHandle {
            semantic: ContentHash::of(b"text-codec-control"),
            realization: lctx_surrealdb::schema::realization_identity(""),
            database: DatabaseIdentity {
                namespace: Name::new(ns).unwrap(),
                database: Name::new(&db).unwrap(),
            },
        },
    );
    let texts: Vec<String> = native
        .query(
            "SELECT VALUE body.text FROM entity WHERE semantic_type='retrieval_corpus_texts'",
            Variables::new(),
        )
        .await
        .unwrap();
    assert_eq!(texts, vec![text.to_owned()]);
    let titles: Vec<String> = native
        .query(
            "SELECT VALUE body.title FROM entity WHERE semantic_type='retrieval_units'",
            Variables::new(),
        )
        .await
        .unwrap();
    assert_eq!(titles, vec!["Usage scenario 雪".to_owned()]);
    // Raw SDK values retain bytes/null tags; the reader's SerdeWrapper route serves ordinary
    // serde models and cannot decode Value's separately tagged serde representation.
    let mut response = native
        .client()
        .query(
            "SELECT VALUE body FROM entity WHERE semantic_type='literal_values' ORDER BY subtype",
        )
        .await
        .unwrap()
        .check()
        .unwrap();
    let literal_bodies: Vec<Value> = response.take(0).unwrap();
    assert_eq!(literal_bodies.len(), 2);
    let textual = literal_bodies[0].as_object().unwrap();
    let opaque_body = literal_bodies[1].as_object().unwrap();
    assert_eq!(
        textual.get("string_value"),
        Some(&Value::from_t("exact 雪\n\0text"))
    );
    assert_eq!(textual.get("bytes_value"), Some(&Value::Null));
    assert_eq!(opaque_body.get("string_value"), Some(&Value::Null));
    assert_eq!(
        opaque_body.get("bytes_value"),
        Some(&Value::Bytes(Bytes::from(vec![0xff, 0, 0x80])))
    );
    assert_eq!(
        native
            .records::<CorpusText>(RecordSelection::Keys(vec![*corpus.id().bytes()]))
            .await
            .unwrap(),
        vec![corpus]
    );
    assert_eq!(
        native
            .records::<Unit>(RecordSelection::Keys(vec![*unit.id().bytes()]))
            .await
            .unwrap(),
        vec![unit.clone()]
    );
    for literal in [string, opaque] {
        assert_eq!(
            native
                .records::<Literal>(RecordSelection::Keys(vec![*literal.id().bytes()]))
                .await
                .unwrap(),
            vec![literal]
        );
    }
    let mut bindings = Variables::new();
    bindings.insert(
        "id",
        reader::target_id(Target::Entity(EntityId::of(unit.id()))),
    );
    client
        .query("UPDATE $id SET body.unexpected=1;")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    assert!(
        loader.ensure_entities(&[Entity::from(unit)]).await.is_err(),
        "complete envelope comparison rejects raw flexible body drift"
    );
    client
        .query(format!("REMOVE DATABASE {db}"))
        .await
        .unwrap()
        .check()
        .unwrap();
}
