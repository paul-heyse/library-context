//! Actual native candidate queries in a private schema fixture, independent of graph admission.
use lctx_model::domain::{
    attribution::AnalysisContext,
    catalog::CatalogMember,
    graph::{Entity, EntityId, Target},
    resources::ResourceBudget,
    retrieval::{ContentPart, Family, SearchWindow, Unit, WindowBinding},
    serving::{ranking::RankingPolicy, *},
    *,
};
use lctx_surrealdb::{Credentials, Loader, NativeReader, loader::json_value, reader};
use surrealdb::types::{RecordId, Value, Variables};
fn id<R: Record>(byte: u8) -> Id<R> {
    serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
}
async fn insert(reader: &NativeReader, table: &str, relation: bool, rows: Vec<Value>) {
    let mut vars = Variables::new();
    vars.insert("rows", rows);
    reader
        .query::<serde_json::Value>(
            format!(
                "INSERT {}INTO {table} $rows RETURN NONE;",
                if relation { "RELATION " } else { "" }
            ),
            vars,
        )
        .await
        .unwrap();
}
fn object(body: serde_json::Value, id: RecordId) -> surrealdb::types::Object {
    let Value::Object(mut obj) = json_value(body).unwrap() else {
        panic!("object")
    };
    obj.insert("id", id);
    obj
}
#[allow(
    clippy::too_many_arguments,
    reason = "The fixture explicitly separates physical endpoints and semantic eligibility witnesses"
)]
fn occurrence(
    table: &str,
    key: &str,
    source: RecordId,
    out: RecordId,
    input: [u8; 16],
    member: Option<Id<CatalogMember>>,
    context: Id<AnalysisContext>,
    unit: Id<Unit>,
) -> Value {
    let mut obj = object(
        serde_json::json!({"family":Family::ApiOptions as i16,"unit":unit,"window":id::<SearchWindow>(7),"part":id::<ContentPart>(8),"binding":if member.is_some(){Some(id::<WindowBinding>(9))}else{None},"exact_name":"connect","exact_path":"pkg.connect","exact_option":"timeout","context":context,"member":member,"anchor":null,"input":input,"eligible":true,"occurrence_key":key}),
        RecordId::new(table, key.to_owned()),
    );
    obj.insert("in", source);
    obj.insert("out", out);
    obj.insert(
        "unit_node",
        reader::target_id(Target::Entity(EntityId::of(unit))),
    );
    Value::Object(obj)
}
#[tokio::test]
async fn native_member_hydration_uses_bounded_physical_windows_without_family_scans() {
    use graph::Assertion;
    use input::{DistributionRole, InputDistribution, InputRevision, Package, Release};
    use lctx_surrealdb::ordered_rows::Candidate;
    use source::{Module, SourceArtifact};
    let cfg: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var("LCTX_SURREAL_TEST_CONFIG").expect("owned disposable native fixture"),
        )
        .unwrap(),
    )
    .unwrap();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let ns = "gn_member_windows";
    let db = format!("members_{}", std::process::id());
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
    let definitions = "";
    let loader = Loader::new(client.clone());
    loader.install(definitions).await.unwrap();
    let input = InputRevision {
        manifest: ContentHash::of(b"selected physical member input"),
    };
    let other = InputRevision {
        manifest: ContentHash::of(b"unrelated physical member input"),
    };
    let package = Package {
        name: "physical-members".into(),
    };
    let unrelated_package = Package {
        name: "other-members".into(),
    };
    let release = Release {
        package: package.id(),
        version: "1".into(),
    };
    let unrelated_release = Release {
        package: unrelated_package.id(),
        version: "1".into(),
    };
    let source =
        SourceArtifact::from_bytes(input.id(), "selected.py".into(), b"selected source").unwrap();
    let other_source =
        SourceArtifact::from_bytes(other.id(), "other.py".into(), b"other source").unwrap();
    let module = Module {
        source: source.id(),
        qualified_name: "selected".into(),
    };
    let other_module = Module {
        source: other_source.id(),
        qualified_name: "other".into(),
    };
    let mut expected = (0..260)
        .rev()
        .map(|index| CatalogMember {
            input: input.id(),
            access: module.id(),
            path: vec![format!("operation_{index:04}")],
            name: format!("operation_{index:04}"),
        })
        .collect::<Vec<_>>();
    let unrelated = (0..320)
        .map(|index| CatalogMember {
            input: other.id(),
            access: other_module.id(),
            path: vec![format!("operation_{index:04}")],
            name: format!("operation_{index:04}"),
        })
        .collect::<Vec<_>>();
    let mut entities = vec![
        Entity::from(input.clone()),
        Entity::from(other.clone()),
        Entity::from(package),
        Entity::from(unrelated_package),
        Entity::from(release.clone()),
        Entity::from(unrelated_release.clone()),
        Entity::from(source),
        Entity::from(other_source),
        Entity::from(module),
        Entity::from(other_module),
    ];
    entities.extend(expected.iter().chain(&unrelated).cloned().map(Entity::from));
    loader.entities(&entities).await.unwrap();
    loader.entity_references(&entities).await.unwrap();
    let assertions = [
        InputDistribution {
            input: input.id(),
            release: release.id(),
            role: DistributionRole::FirstParty,
        },
        InputDistribution {
            input: other.id(),
            release: unrelated_release.id(),
            role: DistributionRole::FirstParty,
        },
    ]
    .into_iter()
    .map(|row| Assertion::from_record(row).unwrap())
    .collect::<Vec<_>>();
    loader.assertions(&assertions).await.unwrap();
    loader.assertion_references(&assertions).await.unwrap();
    let handle = SnapshotHandle {
        semantic: ContentHash::of(b"private physical member fixture"),
        realization: lctx_surrealdb::schema::realization_identity(definitions),
        database: DatabaseIdentity {
            namespace: Name::new(ns).unwrap(),
            database: Name::new(&db).unwrap(),
        },
    };
    let native = NativeReader::new(client.clone(), handle);
    let budget = ResourceBudget::fixed(32 << 20).unwrap();
    expected.sort_by_key(Record::id);
    let members = lctx_serving::selection::members(
        &native,
        Some(&Name::new("physical-members").unwrap()),
        None,
        &budget,
    )
    .await
    .unwrap();
    assert_eq!(
        members.as_slice(),
        expected.as_slice(),
        "all three windows preserve nominal order and exclude unrelated same-family members"
    );
    assert!(
        budget.reserved()
            >= expected
                .iter()
                .map(|row| size_of::<CatalogMember>() + row.heap_bytes())
                .sum::<usize>()
    );
    let candidates = expected
        .iter()
        .map(|row| Candidate {
            relation: CatalogMember::NAME.into(),
            key: *row.id().bytes(),
            node: reader::target_id(Target::Entity(EntityId::of(row.id()))),
            content: None,
        })
        .collect::<Vec<_>>();
    let mut bindings = Variables::new();
    bindings.insert(
        "nodes",
        candidates[..128]
            .iter()
            .map(|candidate| candidate.node.clone())
            .collect::<Vec<_>>(),
    );
    let plan: String = native
        .query(
            format!("EXPLAIN {}", reader::candidate_records_sql()),
            bindings,
        )
        .await
        .unwrap();
    // The pinned engine's SourceExpr resolves the bound RecordId array with a batch
    // point fetch. Sorting that explicit <=128-row source is allowed.
    assert_eq!(
        plan.lines()
            .filter(|line| line.trim_start().starts_with("SourceExpr "))
            .count(),
        1,
        "bounded point source: {plan}"
    );
    for scan in ["TableScan", "IndexScan", "UnionIndexScan", "DynamicScan"] {
        assert!(
            !plan.lines().any(|line| line.trim_start().starts_with(scan)),
            "family scan in physical hydration: {plan}"
        );
    }
    assert_eq!(
        native
            .records_from_candidates::<CatalogMember>(&candidates[..128])
            .await
            .unwrap(),
        expected[..128]
    );
    assert!(matches!(
        native
            .records_from_candidates::<CatalogMember>(&candidates[..129])
            .await,
        Err(ModelError::Limit {
            owner: "native-candidate-hydration",
            ..
        })
    ));
    let mut wrong = candidates[..1].to_vec();
    wrong[0].node = candidates[1].node.clone();
    assert!(matches!(
        native
            .records_from_candidates::<CatalogMember>(&wrong)
            .await,
        Err(ModelError::Conflict("native candidate backing identity"))
    ));
    wrong[0].node = RecordId::new("entity", "missing_physical_member");
    assert!(matches!(
        native
            .records_from_candidates::<CatalogMember>(&wrong)
            .await,
        Err(ModelError::Conflict("native candidate backing missing"))
    ));
    assert!(matches!(
        native
            .records_from_candidates::<CatalogMember>(&[
                candidates[0].clone(),
                candidates[0].clone()
            ])
            .await,
        Err(ModelError::Conflict("native candidate identity/order"))
    ));
    let mut bindings = Variables::new();
    bindings.insert("node", candidates[0].node.clone());
    bindings.insert(
        "canonical",
        surrealdb::types::Bytes::from(
            serde_json::to_vec(&Entity::from(expected[1].clone())).unwrap(),
        ),
    );
    native
        .query::<serde_json::Value>(
            "UPDATE $node SET canonical=$canonical RETURN NONE",
            bindings,
        )
        .await
        .unwrap();
    assert!(matches!(
        native
            .records_from_candidates::<CatalogMember>(&candidates[..1])
            .await,
        Err(ModelError::Conflict("native candidate canonical identity"))
    ));
    drop(members);
    assert_eq!(budget.reserved(), 0);
    client
        .query(format!("REMOVE DATABASE {db}"))
        .await
        .unwrap()
        .check()
        .unwrap();
}
#[tokio::test]
async fn native_channels_admit_exact_context_pairs_and_members_before_candidate_caps() {
    let cfg: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var("LCTX_SURREAL_TEST_CONFIG")
                .expect("owned disposable SurrealDB fixture required"),
        )
        .unwrap(),
    )
    .unwrap();
    let credentials = Credentials::Root {
        username: cfg["admin_user"].as_str().unwrap().into(),
        password: cfg["admin_password"].as_str().unwrap().into(),
    };
    let ns = "gn_search_controls";
    let db = format!("channels_{}", std::process::id());
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
        .install(&lctx_serving::native_definitions())
        .await
        .unwrap();
    let input = id::<input::InputRevision>(1);
    let member = CatalogMember {
        input,
        access: id::<source::Module>(2),
        path: vec!["connect".into()],
        name: "connect".into(),
    };
    let out = reader::target_id(Target::Entity(EntityId::of(member.id())));
    loader
        .entities(&[Entity::from(member.clone())])
        .await
        .unwrap();
    let handle = SnapshotHandle {
        semantic: ContentHash::of(b"private search fixture"),
        realization: lctx_surrealdb::schema::realization_identity(
            &lctx_serving::native_definitions(),
        ),
        database: DatabaseIdentity {
            namespace: Name::new(ns).unwrap(),
            database: Name::new(&db).unwrap(),
        },
    };
    let native = NativeReader::new(client.clone(), handle);
    let mut member_vars = Variables::new();
    member_vars.insert(
        "scope",
        format!(
            "catalog_members|input|{}",
            lctx_surrealdb::prepared::scope_string(
                &json_value(serde_json::to_value(input).unwrap()).unwrap()
            )
        ),
    );
    member_vars.insert("member", Value::Null);
    member_vars.insert("path", Value::Null);
    let member_plan: serde_json::Value = native
        .query(
            format!(
                "{} EXPLAIN",
                lctx_serving::selection::member_candidates_sql().trim_end_matches(';')
            ),
            member_vars,
        )
        .await
        .unwrap();
    let member_plan = member_plan.to_string();
    assert!(member_plan.contains("by_scope"), "{member_plan}");
    for material in ["UnionIndexScan", "Sort", "Aggregate", "Distinct"] {
        assert!(
            !member_plan.contains(material),
            "candidate extraction unexpectedly retains match state: {member_plan}"
        );
    }
    let good = id::<AnalysisContext>(3);
    let other = id::<AnalysisContext>(4);
    let origin = retrieval::Origin::Brief { brief: id(5) };
    let unit_record = Unit {
        input,
        context: good,
        family: Family::ApiOptions,
        origin: origin.id(),
        corpus: id(6),
        title: "brief witness".into(),
    };
    let unit = unit_record.id();
    loader
        .entities(&[Entity::from(origin.clone()), Entity::from(unit_record)])
        .await
        .unwrap();
    let mut origin_edge = surrealdb::types::Object::new();
    origin_edge.insert("id", RecordId::new("reference", "brief_origin"));
    origin_edge.insert("in", reader::target_id(Target::Entity(EntityId::of(unit))));
    origin_edge.insert(
        "out",
        reader::target_id(Target::Entity(EntityId::of(origin.id()))),
    );
    origin_edge.insert("field", "origin");
    origin_edge.insert("role", 0i64);
    origin_edge.insert("position", Value::Null);
    insert(&native, "reference", true, vec![Value::Object(origin_edge)]).await;
    let inputs = [*input.bytes()];
    let pairs = [(*member.id().bytes(), *good.bytes())];
    let mut docs = vec![];
    let mut occurrences = vec![];
    // More than the native document cap of nonmember occurrences cannot crowd the member out.
    for n in 0..102 {
        let key = format!("doc{n:03}");
        let text = if n == 101 {
            "connect eligible member with deliberately longer descriptive text".to_owned()
        } else {
            format!("connect connect connect {n}")
        };
        let doc = RecordId::new("search_api_options", key.clone());
        docs.push(Value::Object(object(
            serde_json::json!({"text":text,"digest":ContentHash::of(text.as_bytes())}),
            doc.clone(),
        )));
        occurrences.push(occurrence(
            "lex_occurs",
            &key,
            doc,
            out.clone(),
            *input.bytes(),
            if n == 101 { Some(member.id()) } else { None },
            good,
            if n == 101 { unit } else { id(55) },
        ));
    }
    let bad = RecordId::new("search_api_options", "other_context");
    docs.push(Value::Object(object(serde_json::json!({"text":"connect connect connect","digest":ContentHash::of(b"connect connect connect")}),bad.clone())));
    occurrences.push(occurrence(
        "lex_occurs",
        "other_context",
        bad,
        out.clone(),
        *input.bytes(),
        Some(member.id()),
        other,
        id(55),
    ));
    // Ubiquitous matched terms yield BM25 zero and still nominate eligible primary witnesses.
    for n in 0..160 {
        let text = format!("connect unrelated background document {n}");
        docs.push(Value::Object(object(
            serde_json::json!({"text":text,"digest":ContentHash::of(text.as_bytes())}),
            RecordId::new("search_api_options", format!("background{n:03}")),
        )));
    }
    insert(&native, "search_api_options", false, docs).await;
    insert(&native, "lex_occurs", true, occurrences).await;
    let policy = RankingPolicy::default();
    let lexical = lctx_serving::search::lexical(
        &native,
        "connect absentterm",
        Family::ApiOptions,
        &inputs,
        Some(&pairs),
        true,
        lctx_serving::search::UnitScope::All,
        100,
        &policy,
    )
    .await
    .unwrap();
    assert_eq!(lexical.len(), 1);
    assert_eq!(lexical[0].occurrence.context, good);
    assert_eq!(lexical[0].score, Some(0.0));
    let selected: embedding::Spec = serde_json::from_slice(include_bytes!(
        "../../../specs/embedding/qwen3-embedding-8b.json"
    ))
    .unwrap();
    let projection = embedding::projection::ProjectionDefinition::initial(&selected).id();
    let encoder = embedding::EmbeddingSpec::new(&selected).unwrap();
    let spec = encoder.service_hash;
    loader
        .entities(&[Entity::from(encoder.clone())])
        .await
        .unwrap();
    let mut vectors = vec![];
    let mut vec_occurrences = vec![];
    let mut query = vec![0f32; 1024];
    query[0] = 1.;
    for (key, context, vector) in [
        ("excluded", other, query.clone()),
        ("eligible", good, {
            let mut v = vec![0f32; 1024];
            v[0] = 0.8;
            v[1] = 0.6;
            v
        }),
        ("secondary", good, {
            let mut v = vec![0f32; 1024];
            v[0] = 0.7;
            v[1] = 0.51f32.sqrt();
            v
        }),
    ] {
        let mut full = vec![0.0; 4096];
        for (index, value) in vector.iter().enumerate() {
            full[index] = *value * 0.5;
        }
        full[2048] = 0.75f32.sqrt();
        let full = embedding::value::FullValue {
            encoder: encoder.id(),
            input: ContentHash::of(key.as_bytes()),
            dimensions: 4096,
            tokens: 5,
            codec: embedding::value::VALUE_CODEC,
            digest: embedding::value::value_digest(&full),
            bytes: EvidenceBytes(embedding::value::encode_vector(&full)),
        };
        loader
            .entities(&[Entity::from(full.clone())])
            .await
            .unwrap();
        let source = RecordId::new("vector", key);

        let obj = object(
            serde_json::json!({"encoder_hash":spec.hex(),"policy_key":projection.hex(),"library_input":lctx_surrealdb::prepared::scope_string(&json_value(serde_json::to_value(input).unwrap()).unwrap()),"family":Family::ApiOptions as i16,"full_key":full.id().hex(),"projection_key":key,"embedding":vector}),
            source.clone(),
        );
        vectors.push(Value::Object(obj));
        let mut witness = occurrence(
            "vec_occurs",
            key,
            source,
            out.clone(),
            *input.bytes(),
            Some(member.id()),
            context,
            if key == "eligible" { unit } else { id(55) },
        );
        if key == "secondary" {
            let Value::Object(ref mut row) = witness else {
                panic!("occurrence")
            };
            row.insert(
                "window",
                json_value(serde_json::to_value(id::<SearchWindow>(17)).unwrap()).unwrap(),
            );
            row.insert(
                "part",
                json_value(serde_json::to_value(id::<ContentPart>(18)).unwrap()).unwrap(),
            );
            row.insert(
                "binding",
                json_value(serde_json::to_value(id::<WindowBinding>(19)).unwrap()).unwrap(),
            );
        }
        vec_occurrences.push(witness);
    }
    // Nearer vectors in a foreign library cohort cannot enter through an otherwise matching edge.
    for n in 0..300 {
        let key = format!("foreign{n}");
        let source = RecordId::new("vector", key.clone());
        vectors.push(Value::Object(object(serde_json::json!({"encoder_hash":spec.hex(),"policy_key":projection.hex(),"library_input":lctx_surrealdb::prepared::scope_string(&json_value(serde_json::to_value(id::<input::InputRevision>(99)).unwrap()).unwrap()),"family":Family::ApiOptions as i16,"full_key":key,"projection_key":key,"embedding":query}),source.clone())));
        vec_occurrences.push(occurrence(
            "vec_occurs",
            &key,
            source,
            out.clone(),
            *input.bytes(),
            Some(member.id()),
            good,
            unit,
        ));
    }
    insert(&native, "vector", false, vectors).await;
    insert(&native, "vec_occurs", true, vec_occurrences).await;
    let vector = lctx_serving::search::vector(
        &native,
        &query,
        spec,
        embedding::value::value_digest(&query),
        selected.query_recipe().identity(),
        projection,
        Family::ApiOptions,
        &inputs,
        Some(&pairs),
        true,
        lctx_serving::search::UnitScope::All,
        100,
        &policy,
    )
    .await
    .unwrap();
    assert_eq!(vector.len(), 1);
    assert_eq!(vector[0].occurrence.context, good);
    assert!(vector[0].score.unwrap() < 1.0);
    // Brief eligibility precedes both channel caps and does not materialize a unit inventory.
    let brief_lexical = lctx_serving::search::lexical(
        &native,
        "connect",
        Family::ApiOptions,
        &inputs,
        None,
        false,
        lctx_serving::search::UnitScope::BriefOrigins,
        1,
        &policy,
    )
    .await
    .unwrap();
    assert_eq!(brief_lexical.len(), 1);
    assert_eq!(brief_lexical[0].occurrence.unit, unit);
    let brief_vector = lctx_serving::search::vector(
        &native,
        &query,
        spec,
        embedding::value::value_digest(&query),
        selected.query_recipe().identity(),
        projection,
        Family::ApiOptions,
        &inputs,
        None,
        false,
        lctx_serving::search::UnitScope::BriefOrigins,
        1,
        &policy,
    )
    .await
    .unwrap();
    assert_eq!(brief_vector.len(), 1);
    assert_eq!(brief_vector[0].occurrence.unit, unit);
    let mut full_query = vec![0.0; 4096];
    full_query[0] = 1.0;
    let query_value = lctx_serving::QueryVector {
        spec,
        input: ContentHash::of(b"query"),
        vector: full_query,
        recipe: selected.query_recipe(),
        projection,
    };
    let rescored = lctx_serving::search::rescore_union(&native, &query_value, &lexical, &policy)
        .await
        .unwrap();
    assert_eq!(rescored.len(), 1);
    assert!(
        (rescored[0].score.unwrap() - 0.4).abs() < 1e-6,
        "full4096, not projected1024, determines rescore"
    );
    let mut explain = Variables::new();
    explain.insert("vector", query);
    explain.insert("encoder_hash", spec.hex());
    explain.insert("policy_key", projection.hex());
    explain.insert("family", Family::ApiOptions as i16);
    explain.insert(
        "input_keys",
        vec![lctx_surrealdb::prepared::scope_string(
            &json_value(serde_json::to_value(input).unwrap()).unwrap(),
        )],
    );
    explain.insert(
        "pairs",
        json_value(serde_json::to_value(pairs).unwrap()).unwrap(),
    );
    explain.insert("member_mode", true);
    explain.insert("brief_origins", false);
    explain.insert("units", Value::Null);
    let plan: serde_json::Value = native
        .query(
            format!(
                "{} EXPLAIN",
                lctx_serving::search::vector_selection_sql(128).unwrap()
            ),
            explain,
        )
        .await
        .unwrap();
    let plan = plan.to_string();
    assert!(plan.contains("KnnScan"), "{plan}");
    assert!(plan.contains("BitmapIndexScan"), "{plan}");
    // Exact spelling is a separate route, including the declared option key.
    let option = lctx_serving::search::lexical(
        &native,
        "timeout",
        Family::ApiOptions,
        &inputs,
        Some(&pairs),
        true,
        lctx_serving::search::UnitScope::All,
        128,
        &policy,
    )
    .await
    .unwrap();
    assert_eq!(option.len(), 1);
    assert_eq!(option[0].occurrence.context, good);
    // One target with more than 1024 windows cannot consume the contextual target quota.
    let beta = (6..=255u8)
        .map(|byte| CatalogMember {
            input,
            access: id(byte),
            path: vec!["pkg".into(), "beta".into()],
            name: "pkg.beta".into(),
        })
        .find(|member| reader::target_id(Target::Entity(EntityId::of(member.id()))) > out)
        .expect("stable later target for quota control");
    let beta_out = reader::target_id(Target::Entity(EntityId::of(beta.id())));
    loader
        .entities(&[Entity::from(beta.clone())])
        .await
        .unwrap();
    let mut crowd_docs = Vec::new();
    let mut crowd_occurrences = Vec::new();
    for n in 0..1400u32 {
        let key = format!("quota{n:04}");
        let text = format!("beta pkg quota_option crowded window {n}");
        let source = RecordId::new("search_api_options", key.clone());
        crowd_docs.push(Value::Object(object(
            serde_json::json!({"text":text,"digest":ContentHash::of(text.as_bytes())}),
            source.clone(),
        )));
        let mut row = occurrence(
            "lex_occurs",
            &key,
            source,
            out.clone(),
            *input.bytes(),
            Some(member.id()),
            good,
            unit,
        );
        let Value::Object(ref mut body) = row else {
            panic!("occurrence")
        };
        let mut window = [7u8; 16];
        window[..4].copy_from_slice(&n.to_le_bytes());
        body.insert(
            "window",
            json_value(serde_json::to_value(window).unwrap()).unwrap(),
        );
        crowd_occurrences.push(row);
    }
    let source = RecordId::new("search_api_options", "quota_exact");
    let text = "independent literal-only route";
    crowd_docs.push(Value::Object(object(
        serde_json::json!({"text":text,"digest":ContentHash::of(text.as_bytes())}),
        source.clone(),
    )));
    let mut row = occurrence(
        "lex_occurs",
        "quota_exact",
        source,
        beta_out,
        *input.bytes(),
        Some(beta.id()),
        good,
        id(25),
    );
    let Value::Object(ref mut body) = row else {
        panic!("occurrence")
    };
    body.insert("exact_name", "beta");
    body.insert("exact_path", "pkg.beta");
    body.insert("exact_option", "quota_option");
    crowd_occurrences.push(row);
    for chunk in crowd_docs.chunks(128) {
        insert(&native, "search_api_options", false, chunk.to_vec()).await;
    }
    for chunk in crowd_occurrences.chunks(128) {
        insert(&native, "lex_occurs", true, chunk.to_vec()).await;
    }
    let quota_pairs = [
        (*member.id().bytes(), *good.bytes()),
        (*beta.id().bytes(), *good.bytes()),
    ];
    for query in ["beta", "pkg.beta", "quota_option"] {
        let admitted = lctx_serving::search::lexical(
            &native,
            query,
            Family::ApiOptions,
            &inputs,
            Some(&quota_pairs),
            true,
            lctx_serving::search::UnitScope::All,
            1024,
            &policy,
        )
        .await
        .unwrap();
        assert_eq!(
            admitted.len(),
            2,
            "window multiplicity must not consume target/context quota"
        );
        assert!(
            matches!(admitted[0].occurrence.target,lctx_model::domain::serving::ranking::Target::Member{member} if member==beta.id()),
            "exact {query} is independently prioritized"
        );
        assert_eq!(admitted[0].occurrence.context, good);
        assert_eq!(admitted[0].score, Some(0.0));
    }
    // A second selected-policy value for the same exact primary witness refuses arbitration.
    let mut response = native
        .client()
        .query("SELECT * FROM vector:excluded")
        .await
        .unwrap()
        .check()
        .unwrap();
    let mut competing: Vec<Value> = response.take(0).unwrap();
    let Value::Object(mut competing) = competing.remove(0) else {
        panic!("vector object")
    };
    let source = RecordId::new("vector", "competing");
    competing.insert("id", source.clone());
    competing.insert("projection_key", "competing");
    insert(&native, "vector", false, vec![Value::Object(competing)]).await;
    insert(
        &native,
        "vec_occurs",
        true,
        vec![occurrence(
            "vec_occurs",
            "competing",
            source,
            out.clone(),
            *input.bytes(),
            Some(member.id()),
            good,
            unit,
        )],
    )
    .await;
    assert!(
        lctx_serving::search::rescore_union(&native, &query_value, &lexical, &policy)
            .await
            .is_err(),
        "competing canonical full winners must refuse"
    );
    client
        .query(
            "DEFINE FUNCTION OVERWRITE fn::lctx_operation_definition() { RETURN 'incompatible'; };",
        )
        .await
        .unwrap()
        .check()
        .unwrap();
    let service = lctx_serving::NativeService::new(native, ResourceLimits::default()).unwrap();
    let refusal = service
        .execute("find_operations", r#"{"library":"unqueried"}"#)
        .await
        .unwrap_err();
    assert_eq!(
        refusal.public_failure(),
        PublicFailure::new(FailureKind::Incompatible)
    );
    client
        .query(format!("REMOVE DATABASE {db}"))
        .await
        .unwrap()
        .check()
        .unwrap();
}
