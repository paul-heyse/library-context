//! Persistent native lowering/cold comparison. Synthetic rows isolate physical realization;
//! graph admission and the complete publication journey are separate owner controls.
use lctx_model::domain::{
    catalog::CatalogMember,
    embedding::{EmbeddingSpec, Spec, analytic::VectorAvailability, value},
    graph::{Assertion, Entity, Target},
    retrieval::{consumption::RetrievalEmbeddingUse, *},
    *,
};
use lctx_publisher::{materialize_search, reconcile_search};
use lctx_surrealdb::surrealdb::types::{Object, RecordId, Value, Variables};
use lctx_surrealdb::{Credentials, Loader, NativeReader, reader};
fn id<R: Record>(byte: u8) -> Id<R> {
    serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
}
async fn execute(loader: &Loader, sql: impl Into<String>, bindings: Variables) {
    loader
        .client()
        .query(sql.into())
        .bind(bindings)
        .await
        .unwrap()
        .check()
        .unwrap();
}
async fn rows(loader: &Loader, table: &str) -> Vec<Value> {
    let mut response = loader
        .client()
        .query(format!("SELECT * FROM {table} ORDER BY id"))
        .await
        .unwrap()
        .check()
        .unwrap();
    response.take(0).unwrap()
}
async fn restore(loader: &Loader, row: &Value, relation: bool) {
    let mut bind = Variables::new();
    bind.insert("rows", vec![row.clone()]);
    execute(
        loader,
        format!(
            "INSERT {}INTO {} $rows",
            if relation { "RELATION " } else { "" },
            row.as_object()
                .unwrap()
                .get("id")
                .unwrap()
                .as_record()
                .unwrap()
                .table
        ),
        bind,
    )
    .await;
}
#[tokio::test]
async fn streamed_witnesses_and_complete_read_only_cold_audit() {
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
    let db = format!("search_{}", std::process::id());
    let client = reader::connect(
        cfg["grpc_endpoint"].as_str().unwrap(),
        &credentials,
        ns,
        &db,
    )
    .await
    .unwrap();
    let loader = Loader::new(client.clone());
    execute(
        &loader,
        format!("DEFINE NAMESPACE IF NOT EXISTS {ns}; DEFINE DATABASE OVERWRITE {db} STRICT;"),
        Variables::new(),
    )
    .await;
    loader
        .install(&lctx_surrealdb::materialization::native_definitions())
        .await
        .unwrap();
    let spec: Spec = serde_json::from_slice(include_bytes!(
        "../../../specs/embedding/qwen3-embedding-8b.json"
    ))
    .unwrap();
    let mut second_spec = spec.clone();
    second_spec.query_task = "independent fixture task".into();
    let specifications = [
        EmbeddingSpec::new(&spec).unwrap(),
        EmbeddingSpec::new(&second_spec).unwrap(),
    ];
    let text = "discover exact native evidence";
    let digest = ContentHash::of(text.as_bytes());
    let mut entities = specifications
        .iter()
        .cloned()
        .map(Entity::from)
        .collect::<Vec<_>>();
    let mut assertions = Vec::new();
    let mut members = Vec::new();
    for name in ["first", "second"] {
        let member = CatalogMember {
            input: id(1),
            access: id(2),
            path: vec![name.into()],
            name: name.into(),
        };
        let subject = Subject::Member {
            member: member.id(),
        };
        members.push((member.id(), subject.id()));
        entities.extend([Entity::from(member), Entity::from(subject)]);
    }
    let mut vector = vec![0f32; 1024];
    vector[0] = 1.;
    vector[1] = -0.;
    let mut units = Vec::new();
    let mut api_fragments = Vec::new();
    let mut api_anchors = Vec::new();
    for family in [
        Family::ApiOptions,
        Family::DocumentationDeployment,
        Family::Scenario,
        Family::Source,
    ] {
        let corpus = CorpusText {
            family,
            rendering_version: RENDER_VERSION,
            digest,
            text: text.into(),
        };
        entities.push(Entity::from(corpus.clone()));
        for context in [3, 4] {
            let unit = Unit {
                input: id(1),
                context: id(context),
                family,
                origin: id(5),
                corpus: corpus.id(),
                title: "fixture".into(),
            };
            entities.push(Entity::from(unit.clone()));
            units.push(unit.clone());
            if family == Family::ApiOptions && context == 3 {
                for (_, subject) in &members {
                    assertions.push(
                        Assertion::from_record(UnitSubject {
                            unit: unit.id(),
                            subject: *subject,
                        })
                        .unwrap(),
                    );
                }
                for ordinal in 0..70 {
                    let anchor = OriginalAnchor {
                        unit: unit.id(),
                        ordinal,
                        original: id(6),
                    };
                    api_anchors.push(anchor.id());
                    assertions.push(Assertion::from_record(anchor).unwrap());
                }
            }
        }
        for ordinal in 0..2 {
            let fragment = Fragment {
                definition: id(7),
                fragment_bytes: 4096,
                corpus: corpus.id(),
                ordinal,
                start: 0,
                end: text.len() as i64,
                digest,
                text: text.into(),
            };
            entities.push(Entity::from(fragment.clone()));
            if family == Family::ApiOptions {
                api_fragments.push(fragment.id());
            }
            for (index, specification) in specifications.iter().enumerate() {
                assertions.push(
                    Assertion::from_record(RetrievalEmbeddingUse {
                        invocation: id(10 + index as u8),
                        fragment: fragment.id(),
                        specification: specification.id(),
                        input: value::input_hash(text),
                        availability: VectorAvailability::Available,
                        admitted_tokens: Some(5),
                        codec: Some(value::VALUE_CODEC),
                        value_digest: Some(value::value_digest(&vector)),
                        bytes: Some(EvidenceBytes(value::encode_vector(&vector))),
                    })
                    .unwrap(),
                );
            }
            if family == Family::ApiOptions && ordinal == 0 {
                // Duplicate use of one immutable winner must not duplicate a contextual witness.
                let mut duplicate: RetrievalEmbeddingUse =
                    lctx_surrealdb::codec::assertion_record(assertions.last().unwrap()).unwrap();
                duplicate.invocation = id(12);
                assertions.push(Assertion::from_record(duplicate).unwrap());
            }
        }
    }
    loader.entities(&entities).await.unwrap();
    loader.assertions(&assertions).await.unwrap();
    // Only the endpoints participating in this lowering are needed in this isolated fixture.
    let mut links = Vec::new();
    for assertion in &assertions {
        for participant in &assertion.participants {
            if participant.field.as_deref().is_some_and(|field| {
                ["unit", "subject", "fragment", "specification"].contains(&field)
            }) {
                let mut link = Object::new();
                link.insert(
                    "id",
                    RecordId::new(
                        "participant",
                        format!(
                            "{}_{}",
                            assertion.id().0.hex(),
                            participant.field.as_deref().unwrap()
                        ),
                    ),
                );
                link.insert("in", reader::target_id(Target::Assertion(assertion.id())));
                link.insert("out", reader::target_id(participant.target.clone()));
                link.insert("field", participant.field.as_deref().unwrap());
                link.insert("role", participant.role as i64);
                link.insert("position", Value::Null);
                links.push(Value::Object(link));
            }
        }
    }
    let mut bind = Variables::new();
    bind.insert("rows", links);
    execute(&loader, "INSERT RELATION INTO participant $rows", bind).await;
    let native = NativeReader::new(
        client.clone(),
        serving::SnapshotHandle {
            semantic: ContentHash::of(b"fixture"),
            realization: ContentHash::of(b"fixture"),
            database: serving::DatabaseIdentity {
                namespace: serving::Name::new(ns).unwrap(),
                database: serving::Name::new(&db).unwrap(),
            },
        },
    );
    let mut unit_stream = native
        .record_stream::<Unit>("true", Variables::new(), "semantic_key")
        .unwrap();
    let mut unit_count = 0;
    while unit_stream
        .next()
        .await
        .expect("native canonical unit projection")
        .is_some()
    {
        unit_count += 1;
    }
    assert_eq!(unit_count, 8);
    materialize_search(&loader).await.unwrap();
    let tables = [
        "search_api_options",
        "search_documentation_deployment",
        "search_scenario",
        "search_source",
        "vector",
        "lex_occurs",
        "vec_occurs",
    ];
    let lexical = rows(&loader, "lex_occurs").await;
    assert_eq!(lexical.len(), 294); // 2 fragments × 2 members × 70 anchors, plus 14 fallbacks.
    let vectors = rows(&loader, "vec_occurs").await;
    assert_eq!(vectors.len(), 588); // every exact witness retained for both specifications.
    assert_eq!(rows(&loader, "vector").await.len(), 2);
    for table in &tables[..4] {
        assert_eq!(rows(&loader, table).await.len(), 1);
    }
    assert_eq!(
        lexical
            .iter()
            .filter(|row| row.as_object().unwrap().get("anchor") != Some(&Value::Null))
            .count(),
        280
    );
    assert_eq!(
        lexical
            .iter()
            .filter(|row| row.as_object().unwrap().get("member") == Some(&Value::Null))
            .count(),
        14
    );
    let physical = |value| lctx_surrealdb::loader::json_value(value).unwrap();
    let first_unit = physical(serde_json::to_value(units[0].id()).unwrap());
    let context = physical(serde_json::to_value(units[0].context).unwrap());
    for fragment in &api_fragments {
        for (member, _) in &members {
            for anchor in &api_anchors {
                let fragment = physical(serde_json::to_value(fragment).unwrap());
                let member = physical(serde_json::to_value(member).unwrap());
                let anchor = physical(serde_json::to_value(anchor).unwrap());
                let witnesses = lexical
                    .iter()
                    .filter(|row| {
                        let row = row.as_object().unwrap();
                        row.get("unit") == Some(&first_unit)
                            && row.get("fragment") == Some(&fragment)
                            && row.get("member") == Some(&member)
                            && row.get("anchor") == Some(&anchor)
                            && row.get("context") == Some(&context)
                    })
                    .count();
                assert_eq!(
                    witnesses, 1,
                    "each member/anchor/fragment/context witness is preserved exactly once"
                );
            }
        }
    }
    reconcile_search(&loader).await.unwrap();
    // Full rows, not just counts/IDs: every family refuses deletion, extra row and changed payload.
    for table in tables {
        let family_rows = rows(&loader, table).await;
        let saved = if table.ends_with("occurs") {
            family_rows
                .iter()
                .find(|row| {
                    let row = row.as_object().unwrap();
                    row.get("member") != Some(&Value::Null)
                        && row.get("anchor") != Some(&Value::Null)
                })
                .unwrap()
                .clone()
        } else {
            family_rows[0].clone()
        };
        let object = saved.as_object().unwrap();
        let mut bind = Variables::new();
        bind.insert("id", object.get("id").unwrap().clone());
        execute(&loader, "DELETE $id", bind.clone()).await;
        assert!(reconcile_search(&loader).await.is_err(), "missing {table}");
        restore(&loader, &saved, table.ends_with("occurs")).await;
        // Endpoint deletion may remove connected relations; restore those exact saved witnesses.
        if !table.ends_with("occurs") {
            for (occurrence_table, saved_rows) in
                [("lex_occurs", &lexical), ("vec_occurs", &vectors)]
            {
                let mut restore_bind = Variables::new();
                restore_bind.insert("rows", saved_rows.clone());
                execute(
                    &loader,
                    format!("INSERT RELATION IGNORE INTO {occurrence_table} $rows"),
                    restore_bind,
                )
                .await;
            }
        }
        let mut extra = object.clone();
        extra.insert("id", RecordId::new(table, "extra"));
        if table.starts_with("search_") {
            extra.insert("digest", vec![0i64; 32]);
        } else if table == "vector" {
            extra.insert("input", vec![0i64; 32]);
        } else {
            extra.insert("occurrence_key", "extra");
        }
        let extra = Value::Object(extra);
        restore(&loader, &extra, table.ends_with("occurs")).await;
        assert!(reconcile_search(&loader).await.is_err(), "extra {table}");
        let mut extra_bind = Variables::new();
        extra_bind.insert("id", extra.as_object().unwrap().get("id").unwrap().clone());
        execute(&loader, "DELETE $id", extra_bind).await;
        let changes = if table.starts_with("search_") {
            vec!["text='changed'"]
        } else if table == "vector" {
            vec![
                "digest=array::repeat(0,32)",
                "bytes=b\"00\"",
                "embedding[0]=0.0,embedding[2]=1.0",
                "embedding[1]=0.0",
            ]
        } else {
            vec![
                "eligible=false",
                "family=3",
                "input=array::repeat(0,16)",
                "context=array::repeat(0,16)",
                "member=NULL",
                "anchor=NULL",
                "unit=array::repeat(0,16)",
                "fragment=array::repeat(0,16)",
                "occurrence_key='changed'",
            ]
        };
        for change in changes {
            execute(&loader, format!("UPDATE $id SET {change}"), bind.clone()).await;
            assert!(
                reconcile_search(&loader).await.is_err(),
                "changed {table}: {change}"
            );
            let after = rows(&loader, table).await;
            // Refusal is read-only: a second audit observes the same exact persisted row set.
            assert!(reconcile_search(&loader).await.is_err());
            assert_eq!(after, rows(&loader, table).await);
            let mut original = bind.clone();
            original.insert("saved", saved.clone());
            execute(&loader, "UPDATE $id CONTENT $saved", original).await;
        }
        let (scope_field, source_field) = if table.starts_with("search_") {
            ("scope_digest", "digest")
        } else if table == "vector" {
            ("scope_input", "input")
        } else {
            ("scope_context", "context")
        };
        execute(&loader, format!("DEFINE FIELD OVERWRITE {scope_field} ON {table} TYPE string; UPDATE $id SET {scope_field}='wrong persisted scope'; DEFINE FIELD OVERWRITE {scope_field} ON {table} TYPE string VALUE <string>{source_field};"), bind.clone()).await;
        assert!(
            reconcile_search(&loader).await.is_err(),
            "persisted scope drift in {table}"
        );
        let invalid = rows(&loader, table).await;
        assert!(reconcile_search(&loader).await.is_err());
        assert_eq!(invalid, rows(&loader, table).await);
        let mut original = bind.clone();
        original.insert("saved", saved.clone());
        execute(&loader, "UPDATE $id CONTENT $saved", original).await;
        reconcile_search(&loader).await.unwrap();
    }
    execute(&loader, format!("REMOVE DATABASE {db}"), Variables::new()).await;
}
