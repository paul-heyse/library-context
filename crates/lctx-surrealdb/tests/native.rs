use lctx_model::domain::{
    embedding::{
        Spec,
        cache::{CacheValue, EmbeddingCache},
    },
    graph::{Entity, EntityId, Target},
    input::{Package, Release},
    *,
};
use lctx_surrealdb::{
    Credentials, Loader, NativeEmbeddingCache, NativeReader, RecordSelection, reader,
};
use surrealdb::types::{Bytes, Value, Variables};
#[path = "fixtures/scoped.rs"]
mod scoped;
fn config() -> serde_json::Value {
    let path = std::env::var("LCTX_SURREAL_TEST_CONFIG")
        .expect("installed validation configuration is required");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
#[tokio::test]
async fn exact_relation_reads_include_ingress_aliases_and_exclude_foreign_revisions() {
    use lctx_surrealdb::compiler::NativeCompilerStore;
    use std::collections::{BTreeMap, BTreeSet};
    let cfg = scoped::config();
    let store = NativeCompilerStore::begin(&cfg, admission::Frontier::Normalized)
        .await
        .unwrap();
    let nonce = lctx_surrealdb::control::fresh_identity("exact-relation-control").unwrap();
    let key = |byte: u8| {
        let mut key = [byte; 16];
        key[..8].copy_from_slice(&nonce.0[..8]);
        key
    };
    let place = value::Place {
        root: serde_json::from_value(serde_json::json!(key(1))).unwrap(),
        path: serde_json::from_value(serde_json::json!(key(2))).unwrap(),
    };
    let mut foreign_place = place.clone();
    foreign_place.root = serde_json::from_value(serde_json::json!(key(3))).unwrap();
    let quality = analytics::QualityStep {
        run: serde_json::from_value(serde_json::json!(key(4))).unwrap(),
        ordinal: 0,
        value: FiniteF64::new(0.5).unwrap(),
    };
    let mut foreign_quality = quality.clone();
    foreign_quality.value = FiniteF64::new(0.75).unwrap();
    let relations = [
        Relation::of::<value::Place>(),
        Relation::of::<analytics::QualityStep>(),
    ];
    let mut selected_views = vec![];
    for (producer, place, quality) in [
        ("selected", place.clone(), quality),
        ("foreign", foreign_place, foreign_quality),
    ] {
        let owner = store
            .begin_contribution(completed::ContributionSpec {
                captured_binding: None,
                producer: format!("{producer}-{}", nonce.hex()),
                profile: stages::Profile::Catalog,
                model: model().unwrap().digest(),
                implementation: ContentHash::of(b"exact-relation-control"),
                configuration: None,
                inputs: vec![],
                outputs: relations
                    .iter()
                    .map(|relation| relation.name().into())
                    .collect::<BTreeSet<_>>(),
            })
            .await
            .unwrap();
        store
            .write_batch(
                &owner,
                &relations[0],
                &value::Place::encode(&[place]).unwrap(),
            )
            .await
            .unwrap();
        store
            .write_batch(
                &owner,
                &relations[1],
                &analytics::QualityStep::encode(&[quality]).unwrap(),
            )
            .await
            .unwrap();
        let views = store
            .complete_contribution(
                owner,
                stages::ProviderOutcome::Complete,
                &relations,
                &BTreeMap::new(),
            )
            .await
            .unwrap();
        if producer == "selected" {
            selected_views = views.values().map(|view| view.identity).collect();
        }
    }
    let native = NativeReader::for_views(
        lctx_surrealdb::compiler::check_installation(&cfg)
            .await
            .unwrap(),
        selected_views,
    );
    let alias = normalized::entities::EntityRef::Place { place: place.id() };
    let result = async {
        let aliases = native
            .records::<normalized::entities::EntityRef>(RecordSelection::Keys(vec![
                *alias.id().bytes(),
            ]))
            .await?;
        let mut streamed = native.record_stream_prepared::<normalized::entities::EntityRef>(
            "true",
            Variables::new(),
            vec!["LET $alias_control = true".into()],
            "semantic_key",
        )?;
        let stream_alias = streamed.next().await?;
        let stream_end = streamed.next().await?;
        drop(streamed);
        let mut alias_rows = native.relation_rows(normalized::entities::EntityRef::NAME)?;
        let mut alias_values = vec![];
        while let Some(row) = alias_rows.next().await? {
            alias_values.push(row);
        }
        alias_rows.drain_transport().await?;
        drop(alias_rows);
        let mut canonical = native.relation_rows(analytics::QualityStep::NAME)?;
        let mut canonical_values = vec![];
        while let Some(row) = canonical.next().await? {
            canonical_values.push(row);
        }
        canonical.drain_transport().await?;
        drop(canonical);
        let mut bodies = native.relation_bodies(analytics::QualityStep::NAME, 1)?;
        let mut body_values = vec![];
        while let Some(row) = bodies.next().await? {
            body_values.push(row);
        }
        bodies.drain_transport().await?;
        drop(bodies);
        Ok::<_, ModelError>((
            aliases,
            stream_alias,
            stream_end,
            alias_values,
            canonical_values,
            body_values,
        ))
    }
    .await;
    let mut completion = completion::Completion::default();
    completion.step("exact relation reader close", native.close().await);
    completion.step("exact relation attempt close", store.abandon().await);
    let (aliases, stream_alias, stream_end, alias_values, canonical_values, body_values) =
        completion::complete(result, completion).unwrap();
    assert_eq!(aliases, vec![alias.clone()]);
    assert_eq!(stream_alias, Some(alias.clone()));
    assert!(stream_end.is_none());
    assert_eq!(alias_values.len(), 1);
    assert_eq!(canonical_values.len(), 1);
    assert_eq!(body_values.len(), 1);
    let Value::Object(alias_row) = &alias_values[0] else {
        panic!("alias envelope")
    };
    let Some(Value::Bytes(bytes)) = alias_row.get("canonical") else {
        panic!("alias canonical")
    };
    assert_eq!(
        serde_json::from_slice::<Entity>(bytes).unwrap(),
        Entity::from(alias)
    );
    let Value::Object(canonical) = &canonical_values[0] else {
        panic!("canonical envelope")
    };
    let Some(Value::Bytes(bytes)) = canonical.get("canonical") else {
        panic!("canonical bytes")
    };
    let Value::Object(body) = &body_values[0] else {
        panic!("body envelope")
    };
    let Some(Value::Object(decoded)) = body.get("body") else {
        panic!("compiler body")
    };
    assert_eq!(
        bytes.as_ref(),
        serde_json::to_vec(body.get("body").unwrap())
            .unwrap()
            .as_slice()
    );
    assert_eq!(decoded.get("value"), Some(&Value::from_t(0.5f64)));
}
#[tokio::test]
async fn prepared_streams_share_authenticated_session_and_keep_it_alive_through_drainage() {
    let cfg = config();
    let credentials = Credentials::Database {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let client = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        cfg["namespace"].as_str().unwrap(),
        cfg["database"].as_str().unwrap(),
    )
    .await
    .unwrap();
    // session::id is the server-assigned RPC session UUID, not the authenticated user.
    let sql = "RETURN { id: session::id(), ns: session::ns(), db: session::db() }";
    let native = NativeReader::private(client.clone());
    let expected: Value = native.query_native(sql, Variables::new()).await.unwrap();
    let Value::Object(session) = &expected else {
        panic!("session descriptor must be an object")
    };
    assert!(matches!(session.get("id"), Some(Value::Uuid(_))));
    assert_eq!(
        session.get("ns"),
        Some(&Value::String(
            cfg["namespace"].as_str().unwrap().to_owned()
        ))
    );
    assert_eq!(
        session.get("db"),
        Some(&Value::String(cfg["database"].as_str().unwrap().to_owned()))
    );
    // Revealing contrast: cloning Surreal itself recreates a distinct authenticated session.
    let independent = NativeReader::private(std::sync::Arc::new(client.as_ref().clone()));
    let other: Value = independent
        .query_native(sql, Variables::new())
        .await
        .unwrap();
    let Value::Object(other) = other else {
        panic!("independent session descriptor")
    };
    assert_ne!(session.get("id"), other.get("id"));
    drop(independent);

    let make = || {
        lctx_surrealdb::prepared::PreparedQuery::new(
            Variables::new(),
            vec![],
            vec![sql.into(), sql.into()],
        )
        .unwrap()
    };
    let first = native.stream_prepared(make()).unwrap();
    let second = native.stream_prepared(make()).unwrap();
    let mut late = native
        .stream_prepared(
            lctx_surrealdb::prepared::PreparedQuery::from_sql(
                format!("{sql}; THROW 'prepared late failure';"),
                Variables::new(),
                2,
                vec![0],
            )
            .unwrap(),
        )
        .unwrap();
    let retained = std::sync::Arc::downgrade(&client);
    drop(native);
    drop(client);
    assert!(
        retained.upgrade().is_some(),
        "streams retain their original session after reader drop"
    );
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
    assert!(
        retained.upgrade().is_none(),
        "drained streams release the session handle"
    );
}
#[tokio::test]
async fn native_codec_graph_search_and_immutable_winners() {
    let config = scoped::config();
    let nonce = tempfile::NamedTempFile::new().unwrap();
    let scope = nonce.path().display().to_string();
    let package = Package {
        name: format!("native-fixture-{scope}"),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
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
    let fixture = scoped::reader(
        &config,
        &[Entity::from(package.clone()), Entity::from(release.clone())],
        std::slice::from_ref(&assertion),
    )
    .await
    .unwrap();
    let reader = &fixture.reader;
    let client = reader.shared_client();
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
        .query(
            format!(
                "SELECT VALUE in.semantic_type FROM reference WHERE out=$key AND ({})",
                reader.selected_node_predicate("in")
            ),
            b,
        )
        .await
        .unwrap();
    assert_eq!(adjacency, vec!["releases"]);
    // Raw SQL preserves flexible bodies; independent admission owns semantic closure.
    let mut bindings = Variables::new();
    bindings.insert(
        "id",
        surrealdb::types::RecordId::new(
            "entity",
            format!("malformed_{}", ContentHash::of(scope.as_bytes()).hex()),
        ),
    );
    bindings.insert(
        "anchor",
        reader::target_id(Target::Entity(EntityId::of(package.id()))),
    );
    client.query("CREATE $id CONTENT {anchor:$anchor,semantic_type:'packages',semantic_key:'bad',kind:29,subtype:null,content:'bad',canonical:b\"00\",scope_keys:[],body:{__type:'packages',name:'bad',extra:1}};").bind(bindings.clone()).await.unwrap().check().unwrap();
    client
        .query("DELETE $id")
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut bindings = Variables::new();
    bindings.insert(
        "source",
        surrealdb::types::RecordId::new(
            "assertion",
            format!("absent_{}", ContentHash::of(scope.as_bytes()).hex()),
        ),
    );
    bindings.insert(
        "target",
        surrealdb::types::RecordId::new(
            "entity_anchor",
            format!("absent_{}", ContentHash::of(scope.as_bytes()).hex()),
        ),
    );
    let enforced = client
        .query(
            "RELATE $source->participant->$target CONTENT {field:'missing',role:0,position:null};",
        )
        .bind(bindings)
        .await
        .unwrap()
        .check();
    assert!(enforced.is_err());
    let cache = NativeEmbeddingCache::connect(client.clone()).await.unwrap();
    let spec: Spec = serde_json::from_slice(include_bytes!(
        "../../../specs/embedding/qwen3-embedding-8b.json"
    ))
    .unwrap();
    let mut vector = vec![0f32; usize::try_from(spec.dimensions).unwrap()];
    vector[0] = 1.;
    let first = CacheValue {
        input_hash: ContentHash::of(scope.as_bytes()),
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
    drop(cache);
    fixture.close().await.unwrap();
}

/// Flexible native bodies preserve logical text, bytes and explicit inactive NULL fields.
#[tokio::test]
async fn native_binary_backed_text_preserves_flexible_bodies() {
    use lctx_model::domain::{
        retrieval::{CorpusText, Family, RENDER_VERSION, Unit},
        value::Literal,
    };
    let config = scoped::config();
    let nonce = tempfile::NamedTempFile::new().unwrap();
    let key = ContentHash::of(nonce.path().to_string_lossy().as_bytes());
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
        origin: serde_json::from_value(serde_json::to_value(&key.0[..16]).unwrap()).unwrap(),
        corpus: corpus.id(),
        title: "Usage scenario 雪".into(),
    };
    let string = Literal::String {
        value: "exact 雪\n\0text".into(),
    };
    let opaque = Literal::Bytes {
        value: EvidenceBytes(vec![0xff, 0, 0x80]),
    };
    let fixture = scoped::reader(
        &config,
        &[
            Entity::from(corpus.clone()),
            Entity::from(unit.clone()),
            Entity::from(string.clone()),
            Entity::from(opaque.clone()),
        ],
        &[],
    )
    .await
    .unwrap();
    let native = &fixture.reader;
    let client = native.shared_client();
    let loader = Loader::new(client.clone());
    let texts: Vec<String> = native
        .query(
            format!("SELECT VALUE body.text FROM entity WHERE semantic_type='retrieval_corpus_texts' AND ({})",native.selected_node_predicate("id")),
            Variables::new(),
        )
        .await
        .unwrap();
    assert_eq!(texts, vec![text.to_owned()]);
    let titles: Vec<String> = native
        .query(
            format!("SELECT VALUE body.title FROM entity WHERE semantic_type='retrieval_units' AND ({})",native.selected_node_predicate("id")),
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
            format!("SELECT VALUE body FROM entity WHERE semantic_type='literal_values' AND ({}) ORDER BY subtype",native.selected_node_predicate("id")),
        )
        .bind(native.view_bindings())
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
        lctx_surrealdb::loader::entity_payload_id(&Entity::from(unit.clone())).unwrap(),
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
    fixture.close().await.unwrap();
}
