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
use lctx_surrealdb::{Loader, RuntimeConfig};
#[path = "../../lctx-serving/tests/fixtures/scoped.rs"]
mod scoped;
struct FixtureIdentity {
    nonce: String,
}
impl FixtureIdentity {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self {
            nonce: format!(
                "{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
    fn id<R: Record>(&self, byte: u8) -> Id<R> {
        let digest = ContentHash::of(format!("{}-{byte}", self.nonce).as_bytes());
        serde_json::from_value(serde_json::json!(&digest.0[..16])).unwrap()
    }
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
    let reader = loader.reader();
    let occurrence_table = if table == "vector" || table == "vec_occurs" { "vec_occurs" } else { "lex_occurs" };
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(64 << 20).unwrap();
    let mut stream = lctx_surrealdb::derived_search::selected_occurrences(&reader, occurrence_table, "true", Variables::new(), &budget).unwrap();
    let mut rows = Vec::new(); let mut sources = std::collections::BTreeSet::new();
    while let Some(row) = stream.next().await.unwrap() {
        if table == occurrence_table { rows.push(row); } else { sources.insert(row.as_object().unwrap().get("in").unwrap().as_record().unwrap().clone()); }
    }
    stream.drain_transport().await.unwrap();
    let sources = sources.into_iter().collect::<Vec<_>>();
    for sources in sources.chunks(128) {
        let mut vars = Variables::new(); vars.insert("sources", sources.to_vec()); vars.insert("table", table.to_owned());
        let mut stream = reader.query_stream("SELECT * FROM $sources WHERE record::table(id)=$table ORDER BY id", vars, 1).unwrap();
        while let Some(row) = stream.next().await.unwrap() { rows.push(row); }
        stream.drain_transport().await.unwrap();
    }
    rows.sort_by_key(|row| row.as_object().unwrap().get("id").unwrap().clone()); rows
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
    streamed_control(AuditTarget::Baseline).await;
}
#[derive(Clone, Copy, Debug)]
enum AuditTarget {
    Baseline,
    Lexical(&'static str),
    Physical(&'static str),
    Definitions,
}
macro_rules! audit_controls {
    ($($name:ident: $kind:ident($table:literal),)*) => {$(
        #[tokio::test]
        async fn $name() { streamed_control(AuditTarget::$kind($table)).await; }
    )*};
}
audit_controls! {
    streamed_lexical_corpus_corruption: Lexical("lexical_corpus"),
    streamed_lexical_term_corruption: Lexical("lexical_term"),
    streamed_lexical_document_corruption: Lexical("lexical_document"),
    streamed_lexical_member_corruption: Lexical("lexical_member"),
    streamed_api_options_corruption: Physical("search_api_options"),
    streamed_documentation_deployment_corruption: Physical("search_documentation_deployment"),
    streamed_scenario_corruption: Physical("search_scenario"),
    streamed_source_corruption: Physical("search_source"),
    streamed_vector_corruption: Physical("vector"),
    streamed_lexical_occurrence_corruption: Physical("lex_occurs"),
    streamed_vector_occurrence_corruption: Physical("vec_occurs"),
}
#[tokio::test]
#[ignore = "requires explicit exclusive maintenance admission"]
async fn installed_scope_definition_drift_is_detected_read_only() {
    assert!(
        std::env::var_os("LCTX_SURREAL_MAINTENANCE_TOKEN").is_some(),
        "run through just service maintenance --native-clients"
    );
    streamed_control(AuditTarget::Definitions).await;
}
async fn streamed_control(target: AuditTarget) {
    let native_read_budget = lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
    let identity = FixtureIdentity::new();
    let maintenance = matches!(target, AuditTarget::Definitions);
    let started = std::time::Instant::now();
    let progress =
        |phase: &str| eprintln!("stream_search {target:?} {:?}: {phase}", started.elapsed());
    progress("fixture construction");
    let config = scoped::config();
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
    let text_owner = format!("discover exact native evidence {}", identity.nonce);
    let text = text_owner.as_str();
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
            input: identity.id(1),
            access: identity.id(2),
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
                input: identity.id(1),
                context: identity.id(context),
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
                original: identity.id(6),
            };
            expected_anchor.push(supported.id());
            assertions.push(Assertion::from_record(supported).unwrap());
            for ordinal in 1..70 {
                assertions.push(
                    Assertion::from_record(OriginalAnchor {
                        unit: unit.id(),
                        ordinal,
                        original: identity.id(9),
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
                    definition: identity.id(7),
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
                    original: Some(identity.id(6)),
                    original_start: Some(0),
                    original_end: Some(text.len() as i64),
                }));
                for (index, encoder) in specifications.iter().enumerate() {
                    assertions.push(
                        Assertion::from_record(RetrievalEmbeddingUse {
                            invocation: identity.id(10 + index as u8),
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
                definition: identity.id(7),
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
    let fixture = scoped::reader(&config, &entities, &assertions)
        .await
        .unwrap();
    let vars = fixture.reader.view_bindings();
    let Some(Value::Array(selected)) = vars.get("lctx_views") else {
        panic!("explicit fixture views")
    };
    let views = selected
        .iter()
        .map(|view| {
            let Value::RecordId(view) = view else {
                panic!("view ID")
            };
            let lctx_surrealdb::surrealdb::types::RecordIdKey::String(hash) = &view.key else {
                panic!("view hash")
            };
            ContentHash(hex::decode(hash).unwrap().try_into().unwrap())
        })
        .collect::<Vec<_>>();
    let loader = Loader::for_attempt_views(
        fixture.reader.shared_client(),
        fixture.store.attempt(),
        views.clone(),
    ).with_budget(&native_read_budget);
    let privileged = if maintenance {
        let installer = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
                .expect("maintenance installer config"),
        ))
        .unwrap();
        let client = lctx_surrealdb::reader::connect(
            &installer.endpoint,
            &installer.writer_credentials(),
            installer.namespace.as_str(),
            installer.database.as_str(),
        )
        .await
        .unwrap();
        Some(Loader::for_views(client, views).with_budget(&native_read_budget))
    } else {
        None
    };
    // This physical-lowering fixture has deliberately partial canonical rows.
    // Sparse canonical scope keys support its frontiers; full graph closure is
    // exercised by the separate admitted publication journey.
    progress("materialize search");
    materialize_search(&loader).await.unwrap();
    progress("read materialized witnesses");
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
    progress("initial cold audit");
    reconcile_search(&loader).await.unwrap();
    progress("initial cold audit passed");
    // Frozen lexical statistics are reconciled by actual payload, without repairing corruption.
    for (table, change) in [
        ("lexical_corpus", "documents=documents+1"),
        ("lexical_term", "df=df+1"),
        ("lexical_document", "length=length+1"),
        ("lexical_member", "family=99"),
    ] {
        if !matches!(target, AuditTarget::Lexical(selected) if selected == table) {
            continue;
        }
        progress(&format!("lexical corruption {table}"));
        let scope = lctx_surrealdb::lexical_stats::scope_identity(&loader.reader()).unwrap();
        let mut bind = Variables::new();
        bind.insert("scope", scope.hex());
        let predicate = if table == "lexical_document" {
            "id IN (SELECT VALUE in FROM lexical_member WHERE scope=$scope)"
        } else {
            "scope=$scope"
        };
        let saved: Vec<Value> = loader
            .reader()
            .query_native(
                format!("SELECT * FROM {table} WHERE {predicate} ORDER BY id LIMIT 1"),
                bind,
            )
            .await
            .unwrap();
        let saved = saved[0].clone();
        let mut vars = Variables::new();
        vars.insert("id", saved.as_object().unwrap().get("id").unwrap().clone());
        vars.insert("saved", saved.clone());
        execute(&loader, format!("UPDATE $id SET {change}"), vars.clone()).await;
        assert!(
            reconcile_search(&loader).await.is_err(),
            "changed immutable {table}"
        );
        let before: Value = loader
            .reader()
            .query_native("SELECT * FROM ONLY $id", vars.clone())
            .await
            .unwrap();
        assert!(reconcile_search(&loader).await.is_err());
        let after: Value = loader
            .reader()
            .query_native("SELECT * FROM ONLY $id", vars.clone())
            .await
            .unwrap();
        assert_eq!(before, after, "cold refusal stays read-only");
        execute(&loader, "UPDATE $id CONTENT $saved", vars).await;
        reconcile_search(&loader).await.unwrap();
    }
    // Full rows, not just counts/IDs: every family refuses deletion, extra row and changed payload.
    for table in tables {
        if !maintenance && !matches!(target, AuditTarget::Physical(selected) if selected == table) {
            continue;
        }
        progress(&format!("family {table}: read witnesses"));
        let family_rows = rows(&loader, table).await;
        let saved = if table.ends_with("occurs") {
            family_rows
                .iter()
                .find(|row| {
                    let row = row.as_object().unwrap();
                    ["member", "anchor", "binding"]
                        .into_iter()
                        .all(|field| row.get(field).is_some_and(|value| value != &Value::Null))
                })
                .unwrap()
                .clone()
        } else {
            family_rows[0].clone()
        };
        let object = saved.as_object().unwrap();
        let mut bind = Variables::new();
        bind.insert("id", object.get("id").unwrap().clone());
        if !maintenance {
            progress(&format!("family {table}: missing row audit"));
            execute(&loader, "DELETE $id", bind.clone()).await;
            let missing_refusal = reconcile_search(&loader).await;
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
            assert!(missing_refusal.is_err(), "missing {table}");
            progress(&format!("family {table}: extra row audit"));
            let mut extra = object.clone();
            extra.insert(
                "id",
                RecordId::new(table, format!("{}-extra", identity.nonce)),
            );
            if table.starts_with("search_") {
                extra.insert(
                    "digest",
                    physical(
                        serde_json::to_value(ContentHash::of(
                            format!("{}-extra-{table}", identity.nonce).as_bytes(),
                        ))
                        .unwrap(),
                    ),
                );
            } else if table == "vector" {
                extra.insert("projection_key", "extra-projection");
            } else {
                extra.insert(
                    "occurrence_key",
                    format!("{}-extra-{table}", identity.nonce),
                );
            }
            let extra = Value::Object(extra);
            restore(&loader, &extra, table.ends_with("occurs")).await;
            let extra_edge = if !table.ends_with("occurs") {
                let template = if table == "vector" {
                    &vectors[0]
                } else {
                    &lexical[0]
                };
                let mut edge = template.as_object().unwrap().clone();
                let name = if table == "vector" {
                    "vec_occurs"
                } else {
                    "lex_occurs"
                };
                let id = RecordId::new(name, format!("{}-extra-{table}", identity.nonce));
                edge.insert("id", id.clone());
                edge.insert("in", extra.as_object().unwrap().get("id").unwrap().clone());
                edge.insert(
                    "occurrence_key",
                    format!("{}-extra-{table}", identity.nonce),
                );
                restore(&loader, &Value::Object(edge), true).await;
                Some(id)
            } else {
                None
            };
            let extra_refusal = reconcile_search(&loader).await;
            let mut extra_bind = Variables::new();
            extra_bind.insert("id", extra.as_object().unwrap().get("id").unwrap().clone());
            if let Some(id) = extra_edge {
                let mut vars = Variables::new();
                vars.insert("id", id);
                execute(&loader, "DELETE $id", vars).await;
            }
            execute(&loader, "DELETE $id", extra_bind).await;
            assert!(extra_refusal.is_err(), "extra {table}");
            let mut mutation_values = Variables::new();
            mutation_values.insert(
                "changed_occurrence_key",
                format!("{}-changed-{table}", identity.nonce),
            );
            if table.ends_with("occurs") {
                let Some(Value::Number(lctx_surrealdb::surrealdb::types::Number::Int(family))) =
                    object.get("family")
                else {
                    panic!("occurrence family integer");
                };
                assert!((0..4).contains(family), "fixture family code");
                let changed_family = (family + 1) % 4;
                assert_ne!(
                    changed_family, *family,
                    "family corruption must change the row"
                );
                mutation_values.insert("changed_family", changed_family);
                assert_eq!(object.get("eligible"), Some(&Value::Bool(true)));
                let zero_identity = physical(serde_json::json!(vec![0; 16]));
                for field in ["input", "context", "unit", "window", "part"] {
                    assert_ne!(
                        object.get(field),
                        Some(&zero_identity),
                        "nonzero fixture {field}"
                    );
                }
                for field in ["member", "anchor", "binding"] {
                    assert!(
                        object.get(field).is_some_and(|value| value != &Value::Null),
                        "nonnull fixture {field}"
                    );
                }
                assert_ne!(
                    object.get("occurrence_key"),
                    mutation_values.get("changed_occurrence_key")
                );
            }
            let changes = if table.starts_with("search_") {
                vec!["text='changed'"]
            } else if table == "vector" {
                vec![
                    "full_key='changed'",
                    "policy_key='changed'",
                    "embedding[0]=0.0,embedding[2]=1.0",
                    "embedding[0]=0.5",
                ]
            } else {
                vec![
                    "eligible=false",
                    "family=$changed_family",
                    "input=array::repeat(0,16)",
                    "context=array::repeat(0,16)",
                    "member=NULL",
                    "anchor=NULL",
                    "unit=array::repeat(0,16)",
                    "window=array::repeat(0,16)",
                    "part=array::repeat(0,16)",
                    "binding=NULL",
                    "occurrence_key=$changed_occurrence_key",
                ]
            };
            for change in changes {
                progress(&format!("family {table}: changed {change}"));
                let mut mutation = bind.clone();
                mutation.extend(mutation_values.clone());
                execute(&loader, format!("UPDATE $id SET {change}"), mutation).await;
                let changed = rows(&loader, table).await;
                let first_refusal = reconcile_search(&loader).await;
                let after = rows(&loader, table).await;
                // Refusal is read-only: a second audit observes the same exact persisted row set.
                let second_refusal = reconcile_search(&loader).await;
                let after_second = rows(&loader, table).await;
                let mut original = bind.clone();
                original.insert("saved", saved.clone());
                execute(&loader, "UPDATE $id CONTENT $saved", original).await;
                // Restore before negative assertions so a test failure retains no corrupt witness.
                assert_ne!(
                    changed, family_rows,
                    "physical mutation must change {table}: {change}"
                );
                assert!(first_refusal.is_err(), "changed {table}: {change}");
                assert!(second_refusal.is_err(), "second refusal {table}: {change}");
                assert_eq!(changed, after, "first audit is read-only");
                assert_eq!(after, after_second, "second audit is read-only");
            }
            progress(&format!("family {table}: restored audit"));
            reconcile_search(&loader).await.unwrap();
            continue;
        }
        let (scope_field, source_field) = if table.starts_with("search_") {
            ("scope_digest", "digest")
        } else if table == "vector" {
            reconcile_search(&loader).await.unwrap();
            continue;
        } else {
            ("scope_context", "context")
        };
        execute(privileged.as_ref().unwrap(), format!("DEFINE FIELD OVERWRITE {scope_field} ON {table} TYPE string; UPDATE $id SET {scope_field}='wrong persisted scope'; DEFINE FIELD OVERWRITE {scope_field} ON {table} TYPE string VALUE <string>{source_field};"), bind.clone()).await;
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
    if let Some(privileged) = privileged {
        privileged.client().invalidate().await.unwrap();
    }
    progress("fixture cleanup");
    fixture.close().await.unwrap();
    loader.client().invalidate().await.unwrap();
    progress("complete");
}
