//! Persistent native lowering/cold comparison. Synthetic rows isolate physical realization;
//! graph admission and the complete publication journey are separate owner controls.
use lctx_model::domain::{
    catalog::CatalogMember,
    embedding::{EmbeddingSpec, Spec, analytic::VectorAvailability, value},
    graph::{Assertion, Entity},
    retrieval::{consumption::RetrievalEmbeddingUse, *},
    *,
};
use lctx_publisher::{materialize_search, reconcile_search};
use lctx_surrealdb::surrealdb::types::{RecordId, Value, Variables};
use lctx_surrealdb::{Credentials, Loader, reader};
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
    second_spec.model = "independent-fixture-encoder".into();
    let specifications = [
        EmbeddingSpec::new(&spec).unwrap(),
        EmbeddingSpec::new(&second_spec).unwrap(),
    ];
    let document = embedding::DocumentRecipe::new(&spec).unwrap();
    let policy = embedding::projection::ProjectionDefinition::initial(&spec);
    let text = "discover exact native evidence";
    let digest = ContentHash::of(text.as_bytes());
    let mut entities = specifications
        .iter()
        .cloned()
        .map(Entity::from)
        .collect::<Vec<_>>();
    entities.extend([Entity::from(document.clone()), Entity::from(policy.clone())]);
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
    let mut vector = vec![0f32; 4096];
    vector[0] = 1.;
    vector[1] = -0.;
    let mut winners = Vec::new();
    let budget = resources::ResourceBudget::fixed(64 * 1024 * 1024).unwrap();
    for (configuration, encoder) in [
        (&spec, &specifications[0]),
        (&second_spec, &specifications[1]),
    ] {
        let admitted = value::AdmittedValue::new(
            configuration,
            &configuration.document_text(text),
            5,
            &vector,
            &budget,
        )
        .unwrap();
        let full = value::FullValue::new(encoder, &admitted).unwrap();
        let projection = embedding::projection::ProjectedValue::new(&full, &policy).unwrap();
        winners.push((full.id(), projection.id()));
        entities.extend([Entity::from(full), Entity::from(projection)]);
    }
    let mut units = Vec::new();
    let mut expected_anchor = Vec::new();
    entities.push(Entity::from(Origin::Api {
        member: members[0].0,
    }));
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
            let origin = Origin::Api {
                member: members[0].0,
            };
            let unit = Unit {
                input: id(1),
                context: id(context),
                family,
                origin: origin.id(),
                corpus: corpus.id(),
                title: "fixture".into(),
            };
            units.push(unit.clone());
            entities.push(Entity::from(unit.clone()));
            // A broad unit attachment must never nominate this sibling.
            assertions.push(
                Assertion::from_record(UnitSubject {
                    unit: unit.id(),
                    subject: members[1].1,
                })
                .unwrap(),
            );
            let supported = OriginalAnchor {
                unit: unit.id(),
                ordinal: 0,
                original: id(6),
            };
            expected_anchor.push(supported.id());
            assertions.push(Assertion::from_record(supported).unwrap());
            for ordinal in 1..70 {
                assertions.push(
                    Assertion::from_record(OriginalAnchor {
                        unit: unit.id(),
                        ordinal,
                        original: id(9),
                    })
                    .unwrap(),
                );
            }
            // More than one 64-window/projection frontier exercises cross-batch winner reuse.
            for ordinal in 0..10 {
                let part = ContentPart {
                    unit: unit.id(),
                    ordinal,
                    purpose: PartPurpose::Primary,
                    scope: Some(members[0].1),
                    qualification: None,
                    digest,
                    text: text.into(),
                };
                let window = SearchWindow {
                    definition: id(7),
                    unit: unit.id(),
                    ordinal,
                    corpus: corpus.id(),
                    digest,
                    text: text.into(),
                    input_text: spec.document_text(text).into(),
                    tokenizer: Some(ContentHash::of(b"fixture tokenizer")),
                    encoded_digest: value::input_hash(&spec.document_text(text)),
                    tokens: Some(5),
                    availability: WindowAvailability::Ready,
                };
                entities.extend([Entity::from(part.clone()), Entity::from(window.clone())]);
                entities.push(Entity::from(WindowPart {
                    window: window.id(),
                    ordinal: 0,
                    part: part.id(),
                    start: 0,
                    end: text.len() as i64,
                }));
                entities.push(Entity::from(WindowBinding {
                    window: window.id(),
                    part: part.id(),
                    subject: members[0].1,
                    basis: BindingBasis::PublicContract,
                    qualification: None,
                }));
                entities.push(Entity::from(WindowSourceMap {
                    window: window.id(),
                    ordinal: 0,
                    start: 0,
                    end: text.len() as i64,
                    part: Some(part.id()),
                    original: Some(id(6)),
                    original_start: Some(0),
                    original_end: Some(text.len() as i64),
                }));
                for (index, encoder) in specifications.iter().enumerate() {
                    assertions.push(
                        Assertion::from_record(RetrievalEmbeddingUse {
                            invocation: id(10 + index as u8),
                            window: window.id(),
                            specification: encoder.id(),
                            document: document.id(),
                            input: value::input_hash(&spec.document_text(text)),
                            availability: VectorAvailability::Available,
                            admitted_tokens: Some(5),
                            value: Some(winners[index].0),
                            projection: Some(winners[index].1),
                        })
                        .unwrap(),
                    );
                }
            }
            // Context-only setup text does not create an applicable search occurrence.
            let setup = "setup only not an applicable primary";
            let part = ContentPart {
                unit: unit.id(),
                ordinal: 10,
                purpose: PartPurpose::Context,
                scope: None,
                qualification: None,
                digest: ContentHash::of(setup.as_bytes()),
                text: setup.into(),
            };
            let window = SearchWindow {
                definition: id(7),
                unit: unit.id(),
                ordinal: 10,
                corpus: corpus.id(),
                digest: part.digest,
                text: setup.into(),
                input_text: spec.document_text(setup).into(),
                tokenizer: None,
                encoded_digest: value::input_hash(&spec.document_text(setup)),
                tokens: None,
                availability: WindowAvailability::TokenizerUnavailable,
            };
            entities.extend([Entity::from(part.clone()), Entity::from(window.clone())]);
            entities.push(Entity::from(WindowPart {
                window: window.id(),
                ordinal: 0,
                part: part.id(),
                start: 0,
                end: setup.len() as i64,
            }));
        }
    }
    loader.entities(&entities).await.unwrap();
    loader.assertions(&assertions).await.unwrap();
    // This physical-lowering fixture has deliberately partial canonical rows.
    // Sparse canonical scope keys support its frontiers; full graph closure is
    // exercised by the separate admitted publication journey.
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
    assert_eq!(lexical.len(), 80);
    let vectors = rows(&loader, "vec_occurs").await;
    assert_eq!(vectors.len(), 160);
    assert_eq!(rows(&loader, "vector").await.len(), 8); // two projections × four input/family cohorts.
    for table in &tables[..4] {
        assert_eq!(rows(&loader, table).await.len(), 1);
    }
    let physical = |value| lctx_surrealdb::loader::json_value(value).unwrap();
    let first_member = physical(serde_json::to_value(members[0].0).unwrap());
    for row in &lexical {
        let row = row.as_object().unwrap();
        assert_eq!(row.get("member"), Some(&first_member));
        assert_ne!(row.get("binding"), Some(&Value::Null));
        assert!(
            expected_anchor
                .iter()
                .any(|a| row.get("anchor") == Some(&physical(serde_json::to_value(a).unwrap())))
        );
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
            extra.insert("projection_key", "extra-projection");
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
                "full_key='changed'",
                "policy_key='changed'",
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
                "window=array::repeat(0,16)",
                "part=array::repeat(0,16)",
                "binding=NULL",
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
            reconcile_search(&loader).await.unwrap();
            continue;
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
