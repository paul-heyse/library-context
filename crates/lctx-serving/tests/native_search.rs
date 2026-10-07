//! Actual native candidate queries in a private schema fixture, independent of graph admission.
use lctx_model::domain::{
    attribution::AnalysisContext,
    catalog::CatalogMember,
    graph::{Entity, EntityId, Target},
    retrieval::{Family, SearchWindow, ContentPart, WindowBinding, Unit},
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
    Value::Object(obj)
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
    let good = id::<AnalysisContext>(3);
    let other = id::<AnalysisContext>(4);
    let unit = id::<Unit>(5);
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
            unit,
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
        unit,
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
        None,
        100,
        &policy,
    )
    .await
    .unwrap();
    assert_eq!(lexical.len(), 1);
    assert_eq!(lexical[0].occurrence.context, good);
    assert_eq!(lexical[0].score,Some(0.0));
    let selected:embedding::Spec=serde_json::from_slice(include_bytes!("../../../specs/embedding/qwen3-embedding-8b.json")).unwrap();
    let projection=embedding::projection::ProjectionDefinition::initial(&selected).id();
    let encoder=embedding::EmbeddingSpec::new(&selected).unwrap();
    let spec=encoder.service_hash;
    loader.entities(&[Entity::from(encoder.clone())]).await.unwrap();
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
    ] {
        let mut full=vec![0.0;4096];
        for (index,value) in vector.iter().enumerate(){full[index]=*value*0.5;}
        full[2048]=0.75f32.sqrt();
        let full=embedding::value::FullValue{encoder:encoder.id(),input:ContentHash::of(key.as_bytes()),dimensions:4096,tokens:5,codec:embedding::value::VALUE_CODEC,digest:embedding::value::value_digest(&full),bytes:EvidenceBytes(embedding::value::encode_vector(&full))};
        loader.entities(&[Entity::from(full.clone())]).await.unwrap();
        let source = RecordId::new("vector", key);

        let obj = object(
            serde_json::json!({"encoder_hash":spec.hex(),"policy_key":projection.hex(),"library_input":lctx_surrealdb::reconciliation::scope_string(&json_value(serde_json::to_value(input).unwrap()).unwrap()),"family":Family::ApiOptions as i16,"full_key":full.id().hex(),"projection_key":key,"embedding":vector}),
            source.clone(),
        );
        vectors.push(Value::Object(obj));
        vec_occurrences.push(occurrence(
            "vec_occurs",
            key,
            source,
            out.clone(),
            *input.bytes(),
            Some(member.id()),
            context,
            unit,
        ));
    }
    // Nearer vectors in a foreign library cohort cannot enter through an otherwise matching edge.
    for n in 0..300 {
        let key=format!("foreign{n}");let source=RecordId::new("vector",key.clone());
        vectors.push(Value::Object(object(serde_json::json!({"encoder_hash":spec.hex(),"policy_key":projection.hex(),"library_input":lctx_surrealdb::reconciliation::scope_string(&json_value(serde_json::to_value(id::<input::InputRevision>(99)).unwrap()).unwrap()),"family":Family::ApiOptions as i16,"full_key":key,"projection_key":key,"embedding":query}),source.clone())));
        vec_occurrences.push(occurrence("vec_occurs",&key,source,out.clone(),*input.bytes(),Some(member.id()),good,unit));
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
        None,
        100,
        &policy,
    )
    .await
    .unwrap();
    assert_eq!(vector.len(), 1);
    assert_eq!(vector[0].occurrence.context, good);
    assert!(vector[0].score.unwrap() < 1.0);
    let mut full_query=vec![0.0;4096];full_query[0]=1.0;
    let query_value=lctx_serving::QueryVector{spec,input:ContentHash::of(b"query"),vector:full_query,recipe:selected.query_recipe(),projection};
    let rescored=lctx_serving::search::rescore_union(&native,&query_value,&lexical,&policy).await.unwrap();
    assert_eq!(rescored.len(),1);
    assert!((rescored[0].score.unwrap()-0.4).abs()<1e-6,"full4096, not projected1024, determines rescore");
    let mut explain=Variables::new();
    explain.insert("vector",query);explain.insert("encoder_hash",spec.hex());explain.insert("policy_key",projection.hex());explain.insert("family",Family::ApiOptions as i16);
    explain.insert("input_keys",vec![lctx_surrealdb::reconciliation::scope_string(&json_value(serde_json::to_value(input).unwrap()).unwrap())]);
    explain.insert("pairs",json_value(serde_json::to_value(pairs).unwrap()).unwrap());explain.insert("member_mode",true);explain.insert("units",Value::Null);
    let plan:serde_json::Value=native.query(format!("{} EXPLAIN",lctx_serving::search::vector_selection_sql(128).unwrap()),explain).await.unwrap();
    let plan=plan.to_string();
    assert!(plan.contains("KnnScan"),"{plan}");
    assert!(plan.contains("BitmapIndexScan"),"{plan}");
    // Exact spelling is a separate route, including the declared option key.
    let option=lctx_serving::search::lexical(&native,"timeout",Family::ApiOptions,&inputs,Some(&pairs),true,None,128,&policy).await.unwrap();
    assert_eq!(option.len(),1);assert_eq!(option[0].occurrence.context,good);
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
