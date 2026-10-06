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
use surrealdb::types::Variables;
fn config() -> serde_json::Value {
    let path = std::env::var("LCTX_SURREAL_TEST_CONFIG")
        .expect("owned disposable SurrealDB configuration is required");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
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
    let schema_error=client.query("CREATE entity:bad CONTENT {semantic_type:'packages',semantic_key:'bad',kind:29,subtype:null,content:'bad',canonical:b\"00\",body:{__type:'packages',name:'bad',extra:1}};").await.unwrap().check();
    assert!(schema_error.is_err());
    let enforced=client.query("RELATE assertion:absent->participant:dangling->entity:absent CONTENT {field:'missing',role:0,position:null};").await.unwrap().check();
    assert!(enforced.is_err());
    let cache = NativeEmbeddingCache::install(client.clone()).await.unwrap();
    let spec: Spec = serde_json::from_slice(include_bytes!(
        "../../../specs/embedding/qwen3-embedding-8b.json"
    ))
    .unwrap();
    let mut vector = vec![0f32; 1024];
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

/// The native body and its closed schema must agree on logical binary-backed text.
#[tokio::test]
async fn native_binary_backed_text_matches_closed_schema() {
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
    let literal_bodies: Vec<serde_json::Value> = native
        .query(
            "SELECT VALUE body FROM entity WHERE semantic_type='literal_values' ORDER BY subtype",
            Variables::new(),
        )
        .await
        .unwrap();
    assert_eq!(literal_bodies.len(), 2);
    assert_eq!(literal_bodies[0]["string_value"], "exact 雪\n\0text");
    assert!(literal_bodies[1]["string_value"].is_null());
    assert!(
        literal_bodies
            .iter()
            .all(|body| body.get("bytes_value").is_none())
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
    assert!(
        client
            .query("UPDATE $id SET body.unexpected=1;")
            .bind(bindings)
            .await
            .unwrap()
            .check()
            .is_err(),
        "logical text must not open the closed object schema"
    );
    client
        .query(format!("REMOVE DATABASE {db}"))
        .await
        .unwrap()
        .check()
        .unwrap();
}
