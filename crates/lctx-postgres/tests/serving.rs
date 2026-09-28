//! Targeted, real PG18 controls for the PG10/11 service contracts.
use arrow_array::{FixedSizeBinaryArray, RecordBatch, StringArray};
use lctx_postgres::{
    Config, projection,
    serving::{Role, RoleConfig, TEST_IMAGE},
};
use std::sync::Arc;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};
struct Fixture {
    _db: ContainerAsync<Postgres>,
    admin: sqlx::PgPool,
    importer: sqlx::PgPool,
    serving: sqlx::PgPool,
    config: RoleConfig,
}
impl Fixture {
    async fn start() -> Self {
        let (name, tag) = TEST_IMAGE.trim().split_once(':').unwrap();
        let db = Postgres::default()
            .with_fsync_enabled()
            .with_name(name)
            .with_tag(tag)
            .start()
            .await
            .expect("Docker/pinned pgvector PG18 image required");
        let port = db.get_host_port_ipv4(5432).await.unwrap();
        let url = |role: &str| format!("postgres://{role}:postgres@127.0.0.1:{port}/postgres");
        let admin = sqlx::PgPool::connect(&url("postgres")).await.unwrap();
        sqlx::raw_sql("CREATE ROLE lctx_app LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_migrator LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_importer LOGIN PASSWORD 'postgres'; CREATE ROLE lctx_serving LOGIN PASSWORD 'postgres'; GRANT CREATE ON DATABASE postgres TO lctx_migrator; GRANT CREATE ON SCHEMA public TO lctx_migrator; REVOKE CREATE ON SCHEMA public FROM PUBLIC; REVOKE TEMP ON DATABASE postgres FROM PUBLIC; GRANT TEMP ON DATABASE postgres TO lctx_importer; CREATE SCHEMA lctx_ext; REVOKE ALL ON SCHEMA lctx_ext FROM PUBLIC; CREATE EXTENSION vector WITH SCHEMA lctx_ext VERSION '0.8.6'; GRANT USAGE ON SCHEMA lctx_ext TO lctx_app,lctx_migrator,lctx_importer,lctx_serving;").execute(&admin).await.unwrap();
        let c = Config {
            application_url: url("lctx_app"),
            migration_url: url("lctx_migrator"),
            migration_config: None,
            max_connections: 2,
            acquire_timeout_seconds: 1,
            statement_timeout_seconds: 2,
            lock_timeout_seconds: 1,
            max_receipt_bytes: 268435456,
        };
        let migrator = c.connect_migrator().await.unwrap();
        migrator.migrate().await.unwrap();
        migrator.check().await.unwrap();
        migrator.migrate().await.unwrap();
        migrator.close().await;
        let importer = sqlx::PgPool::connect(&url("lctx_importer")).await.unwrap();
        let serving = sqlx::PgPool::connect(&url("lctx_serving")).await.unwrap();
        let config = RoleConfig {
            format: 1,
            role: Role::Serving,
            url: url("lctx_serving"),
            max_connections: 2,
            provider_connections: 0,
            acquire_timeout_seconds: 1,
            statement_timeout_seconds: 1,
            lock_timeout_seconds: 1,
        };
        Self {
            _db: db,
            admin,
            importer,
            serving,
            config,
        }
    }
    async fn generation(&self, n: u8) -> Vec<u8> {
        let manifest =
            format!("{{\"format\":1,\"bundle_format\":11,\"dimensions\":1024,\"test\":{n}}}");
        sqlx::query_scalar("INSERT INTO lctx_serving.generations(generation_digest,canonical_manifest) VALUES (sha256(convert_to($1,'UTF8')),$1) RETURNING generation_digest").bind(manifest).fetch_one(&self.importer).await.unwrap()
    }
}
#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn generation_freeze_roles_and_cross_generation_keys() {
    let f = Fixture::start().await;
    let gen1 = f.generation(1).await;
    let gen2 = f.generation(2).await;
    let store = f.config.open_serving().await.unwrap();
    assert_eq!(store.check().await.unwrap().extension, "0.8.6");
    store.close().await;
    for sql in [
        "CREATE TABLE lctx_serving.forbidden(x int)",
        "CREATE TEMP TABLE forbidden(x int)",
        "INSERT INTO lctx_cache.specs VALUES ('x','x',1)",
        "UPDATE lctx_serving.generations SET state='ready'",
        "CREATE TABLE lctx_ext.forbidden(x int)",
    ] {
        assert!(sqlx::query(sql).execute(&f.serving).await.is_err(), "{sql}");
    }
    let insert = "INSERT INTO lctx_serving.evidence(generation_digest,row_ordinal,evidence_id,kind) VALUES($1,$2,$3,$4)";
    // Choose a declared evidence kind from the owner rather than inventing fixture codes.
    let kind = cpg_schema::codebook::registry()
        .into_iter()
        .find(|b| b.name == "evidence_kind")
        .unwrap()
        .values[0]
        .1;
    sqlx::query(insert)
        .bind(&gen1)
        .bind(0i64)
        .bind([1u8; 16].as_slice())
        .bind(kind)
        .execute(&f.importer)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.evidence")
            .fetch_one(&f.serving)
            .await
            .unwrap(),
        0
    );
    // A FK includes generation; a matching entity in another generation cannot satisfy it.
    let role = cpg_schema::codebook::registry()
        .into_iter()
        .find(|b| b.name == "support_role")
        .unwrap()
        .values[0]
        .1;
    assert!(sqlx::query("INSERT INTO lctx_serving.supports(generation_digest,row_ordinal,assertion_id,role,ordinal,evidence_id) VALUES($1,0,$2,$3,0,$2)").bind(&gen2).bind([1u8;16].as_slice()).bind(role).execute(&f.importer).await.is_err());
    let mut writer = f.importer.begin().await.unwrap();
    sqlx::query(insert)
        .bind(&gen1)
        .bind(1i64)
        .bind([2u8; 16].as_slice())
        .bind(kind)
        .execute(&mut *writer)
        .await
        .unwrap();
    let freeze = sqlx::query("SELECT lctx_serving.freeze_generation($1)")
        .bind(&gen1)
        .execute(&f.importer);
    tokio::pin!(freeze);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut freeze)
            .await
            .is_err()
    );
    writer.commit().await.unwrap();
    freeze.await.unwrap();
    assert!(
        sqlx::query(insert)
            .bind(&gen1)
            .bind(2i64)
            .bind([3u8; 16].as_slice())
            .bind(kind)
            .execute(&f.importer)
            .await
            .is_err()
    );
    sqlx::query("UPDATE lctx_serving.generations SET state='ready' WHERE generation_digest=$1")
        .bind(&gen1)
        .execute(&f.admin)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.evidence")
            .fetch_one(&f.serving)
            .await
            .unwrap(),
        2
    );
    assert!(
        sqlx::query("UPDATE lctx_serving.evidence SET kind=kind WHERE generation_digest=$1")
            .bind(&gen1)
            .execute(&f.admin)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM lctx_serving.generations WHERE generation_digest=$1")
            .bind(&gen1)
            .execute(&f.admin)
            .await
            .is_err()
    );
}
#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn copy_codec_pgvector_and_empty_declared_schemas() {
    let f = Fixture::start().await;
    let schema = cpg_schema::bundle::files(1024)
        .into_iter()
        .find(|f| f.name == "lexical_text")
        .unwrap()
        .schema;
    let ids = FixedSizeBinaryArray::try_from_iter([[1u8; 16], [2u8; 16]].into_iter()).unwrap();
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(StringArray::from(vec!["", "non-ascii λ"])),
        ],
    )
    .unwrap();
    let mut conn = f.importer.acquire().await.unwrap();
    sqlx::query("CREATE TEMP TABLE stage (brief_id bytea,text text)")
        .execute(&mut *conn)
        .await
        .unwrap();
    let mut copy = conn
        .copy_in_raw("COPY stage FROM STDIN BINARY")
        .await
        .unwrap();
    copy.send(
        projection::copy_bytes("lexical_text", 1024, &batch)
            .unwrap()
            .freeze(),
    )
    .await
    .unwrap();
    assert_eq!(copy.finish().await.unwrap(), 2);
    let rows = sqlx::query("SELECT brief_id,text FROM stage ORDER BY brief_id")
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        batch,
        projection::decode_rows("lexical_text", 1024, &rows).unwrap()
    );
    let empty = projection::decode_rows("lexical_text", 1024, &[]).unwrap();
    assert_eq!(empty.schema(), batch.schema());
    assert_eq!(empty.num_rows(), 0);
    // A non-embedded generation retains its declared empty width-zero vector schema.
    for name in ["vectors", "operation_vectors"] {
        let empty = projection::decode_rows(name, 0, &[]).unwrap();
        let schema = cpg_schema::bundle::files(0)
            .into_iter()
            .find(|f| f.name == name)
            .unwrap()
            .schema;
        assert_eq!(empty.schema(), schema);
        assert_eq!(empty.num_rows(), 0);
        assert!(projection::copy_bytes(name, 0, &empty).is_ok());
    }
    let mut vector = vec![0f32; 1024];
    vector[0] = 1.0;
    vector[1] = -0.0;
    sqlx::query("SET search_path=pg_catalog,lctx_ext")
        .execute(&mut *conn)
        .await
        .unwrap();
    let returned: pgvector::Vector = sqlx::query_scalar("SELECT $1::lctx_ext.vector(1024)")
        .bind(pgvector::Vector::from(vector.clone()))
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        returned
            .as_slice()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        vector.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert!(
        sqlx::query("SELECT $1::lctx_ext.vector(1024)")
            .bind(pgvector::Vector::from(vec![1.0; 2]))
            .execute(&mut *conn)
            .await
            .is_err()
    );
    let schema = cpg_schema::bundle::files(1024)
        .into_iter()
        .find(|f| f.name == "vectors")
        .unwrap()
        .schema;
    let mut builder = arrow_array::builder::FixedSizeListBuilder::new(
        arrow_array::builder::Float32Builder::new(),
        1024,
    )
    .with_field(std::sync::Arc::new(arrow_schema::Field::new(
        "item",
        arrow_schema::DataType::Float32,
        false,
    )));
    for _ in 0..2 {
        builder.values().append_slice(&vector);
        builder.append(true);
    }
    let ids = FixedSizeBinaryArray::try_from_iter([[1u8; 16], [1u8; 16]].into_iter()).unwrap();
    let hashes = FixedSizeBinaryArray::try_from_iter([[2u8; 32], [3u8; 32]].into_iter()).unwrap();
    let vectors = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(arrow_array::Int64Array::from(vec![0, 1])),
            Arc::new(hashes),
            Arc::new(builder.finish()),
        ],
    )
    .unwrap();
    sqlx::query("CREATE TEMP TABLE vector_stage(brief_id bytea,chunk bigint,input_hash bytea,vector real[])").execute(&mut *conn).await.unwrap();
    let mut copy = conn
        .copy_in_raw("COPY vector_stage FROM STDIN BINARY")
        .await
        .unwrap();
    copy.send(
        projection::copy_bytes("vectors", 1024, &vectors)
            .unwrap()
            .freeze(),
    )
    .await
    .unwrap();
    assert_eq!(copy.finish().await.unwrap(), 2);
    let rows=sqlx::query("SELECT brief_id,chunk,input_hash,vector::lctx_ext.vector(1024) vector FROM vector_stage ORDER BY chunk").fetch_all(&mut *conn).await.unwrap();
    let reconstructed = projection::decode_rows("vectors", 1024, &rows).unwrap();
    assert_eq!(
        cpg_schema::serving_projection::receipt("vectors", 1024, &[vectors]).unwrap(),
        cpg_schema::serving_projection::receipt("vectors", 1024, &[reconstructed]).unwrap()
    );
    let wrong = sqlx::query("SELECT 'x'::bytea brief_id,''::text text")
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    assert!(projection::decode_rows("lexical_text", 1024, &wrong).is_err());
}

#[test]
fn role_configuration_refuses_ambiguous_tls_and_pool_budgets() {
    let base = RoleConfig {
        format: 1,
        role: Role::Serving,
        url: "postgres://lctx_serving:secret-token@127.0.0.1/lctx".into(),
        max_connections: 6,
        provider_connections: 2,
        acquire_timeout_seconds: 5,
        statement_timeout_seconds: 30,
        lock_timeout_seconds: 5,
    };
    assert!(base.validate().is_ok());
    assert!(!format!("{base:?}").contains("secret-token"));
    for url in [
        "postgres://lctx_serving:secret-token@remote.invalid/lctx?sslmode=disable",
        "postgres://lctx_serving:secret-token@127.0.0.1/lctx?sslmode=disable&sslmode=require",
        "postgres://lctx_serving:secret-token@127.0.0.1/lctx?options=-csearch_path=public",
        "postgres://lctx_migrator:secret-token@127.0.0.1/lctx",
    ] {
        let mut c = base.clone();
        c.url = url.into();
        assert!(c.validate().is_err());
    }
    let mut c = base;
    c.provider_connections = 6;
    assert!(c.validate().is_err());
}

fn empty_source(root: &std::path::Path) -> lctx_postgres::import::Source {
    empty_source_version(root, "empty==1")
}
fn empty_source_version(
    root: &std::path::Path,
    requirement: &str,
) -> lctx_postgres::import::Source {
    use cpg_schema::serving_projection::{
        self as contract, ArtifactReceipt, Manifest, ServingContext,
    };
    use sha2::{Digest as _, Sha256};
    let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    let mut relations = std::collections::BTreeMap::new();
    let mut artifacts = std::collections::BTreeMap::new();
    let mut files = serde_json::Map::new();
    for file in cpg_schema::bundle::files(0) {
        let batch = RecordBatch::new_empty(file.schema.clone());
        let mut raw = Vec::new();
        {
            let mut writer =
                arrow_ipc::writer::FileWriter::try_new(&mut raw, &file.schema).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }
        let name = format!("{}.arrow", file.name);
        let hash = hex(&Sha256::digest(&raw));
        std::fs::write(root.join(&name), &raw).unwrap();
        relations.insert(
            file.name.to_owned(),
            contract::receipt(file.name, 0, &[batch]).unwrap(),
        );
        if contract::artifact_names().contains(&name) {
            artifacts.insert(
                name.clone(),
                ArtifactReceipt {
                    sha256: hash.clone(),
                    bytes: raw.len() as u64,
                    format: 1,
                },
            );
        }
        files.insert(file.name.into(),serde_json::json!({"file":name,"sha256":hash,"rows":0,"schema_digest":relations[file.name].schema_digest}));
    }
    let m = Manifest {
        format: 2,
        bundle_format: 12,
        context: ServingContext {
            library: "empty".into(),
            requirement: requirement.into(),
            summary: Default::default(),
        },
        snapshot_id: "01".repeat(16),
        snapshot_digest: "02".repeat(32),
        compiler_digest: "03".repeat(32),
        projection_digest: contract::definition_digest(),
        catalog_digest: cpg_schema::models::Catalog::committed_digest().hex(),
        kernel_format: 1,
        entry_value_effect_digest: "04".repeat(32),
        spec_hash: None,
        dimensions: 0,
        relations,
        artifacts,
    };
    let outer = serde_json::json!({"format":12,"library":m.context.library,"requirement":m.context.requirement,"summary":m.context.summary,"snapshot_id":m.snapshot_id,"content_digest":m.snapshot_digest,"compiler_digest":m.compiler_digest,"condition_kernel_format":1,"entry_value_effect_digest":m.entry_value_effect_digest,"spec_hash":null,"files":files,"projection_generation":m.generation().unwrap(),"projection":m});
    std::fs::write(
        root.join("MANIFEST.json"),
        serde_json::to_vec(&outer).unwrap(),
    )
    .unwrap();
    lctx_postgres::import::Source::open(root).unwrap()
}
#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn production_import_is_atomic_repeatable_and_selection_is_explicit() {
    let f = Fixture::start().await;
    let input = tempfile::tempdir().unwrap();
    let artifacts = tempfile::tempdir().unwrap();
    let source = empty_source(input.path());
    let mut config = f.config.clone();
    config.role = Role::Importer;
    config.url = config.url.replace("lctx_serving:", "lctx_importer:");
    config.statement_timeout_seconds = 30;
    let store = config.open_importer().await.unwrap();
    let result = store
        .import(source.clone(), artifacts.path().to_owned())
        .await
        .unwrap();
    assert_eq!(result.state, "ready");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.generations")
            .fetch_one(&f.serving)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.selections")
            .fetch_one(&f.serving)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        store
            .import(source.clone(), artifacts.path().to_owned())
            .await
            .unwrap()
            .generation,
        result.generation
    );
    store.reconcile(source.generation()).await.unwrap();
    store
        .select(
            "empty",
            source.generation(),
            cpg_schema::id::Digest::from_hex(&result.profile).unwrap(),
        )
        .await
        .unwrap();
    assert!(
        store
            .select(
                "another-library",
                source.generation(),
                cpg_schema::id::Digest::from_hex(&result.profile).unwrap()
            )
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.selections")
            .fetch_one(&f.serving)
            .await
            .unwrap(),
        1
    );
    let reader = f.config.open_serving().await.unwrap();
    let pin = reader.pin("empty", None, None).await.unwrap();
    let second = tempfile::tempdir().unwrap();
    let second = empty_source_version(second.path(), "empty==2");
    store
        .import(second.clone(), artifacts.path().to_owned())
        .await
        .unwrap();
    store
        .select(
            "empty",
            second.generation(),
            cpg_schema::Digest::from_hex(&result.profile).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pin.generation(), source.generation().hex());
    assert_eq!(
        reader.pin("empty", None, None).await.unwrap().generation(),
        second.generation().hex()
    );
    assert!(store.cleanup(source.generation()).await.is_err());
    reader.close().await;
    for sql in [
        "SELECT lctx_serving.mark_ready(decode(repeat('01',32),'hex'),'{}')",
        "CREATE TABLE lctx_serving.unowned(x int)",
    ] {
        assert!(
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .execute(&f.serving)
                .await
                .is_err()
        );
    }
    let path = artifacts
        .path()
        .join(&source.manifest().artifacts["lexical_text.arrow"].sha256)
        .join("lexical_text.arrow");
    std::fs::write(path, b"corrupt").unwrap();
    assert!(
        store
            .import(source, artifacts.path().to_owned())
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM lctx_serving.generations")
            .fetch_one(&f.admin)
            .await
            .unwrap(),
        "ready"
    );
    store.close().await;
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL and published pilot projection check"]
async fn published_pilot_import_and_repository_queries() {
    let path = std::env::var_os("LCTX_TEST_PROJECTION")
        .expect("LCTX_TEST_PROJECTION must name a verified published bundle");
    let source = lctx_postgres::import::Source::open(std::path::Path::new(&path)).unwrap();
    let f = Fixture::start().await;
    let artifacts = tempfile::tempdir().unwrap();
    let mut config = f.config.clone();
    config.role = Role::Importer;
    config.url = config.url.replace("lctx_serving:", "lctx_importer:");
    config.statement_timeout_seconds = 30;
    let importer = config.open_importer().await.unwrap();
    let imported = importer
        .import(source.clone(), artifacts.path().to_owned())
        .await
        .unwrap();
    assert_eq!(imported.state, "ready");
    assert!(imported.completed_batches > 0);
    let mut read_config = f.config.clone();
    read_config.statement_timeout_seconds = 30;
    let reader = read_config.open_serving().await.unwrap();
    let pinned = reader
        .pin(
            &source.manifest().context.library,
            Some(source.generation()),
            None,
        )
        .await
        .unwrap();
    let page = reader
        .find_operations(&pinned, &Default::default(), 20, None)
        .await
        .unwrap();
    assert_eq!(
        page["total"].as_u64().unwrap(),
        source.manifest().relations["operations"].rows
    );
    for op in page["matches"].as_array().unwrap() {
        let result = reader
            .get_operation(
                &pinned,
                &pinned.manifest().snapshot_id,
                op["access_path"].as_str().unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result["operation_id"], op["operation_id"]);
    }
    let scope = reader
        .search_scope(&pinned, None, "transport", false)
        .await
        .unwrap();
    for id in scope["eligible"].as_array().unwrap() {
        let result = reader
            .get_capability(
                &pinned,
                &pinned.manifest().snapshot_id,
                id.as_str().unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result["capability_id"], *id);
    }
    assert!(
        reader
            .find_operations(&pinned, &Default::default(), 20, Some("not a cursor"))
            .await
            .is_err()
    );
    if let Some(cursor) = page["next_cursor"].as_str() {
        let filter = lctx_postgres::repository::Where {
            path_prefix: Some("different".into()),
            ..Default::default()
        };
        assert!(
            reader
                .find_operations(&pinned, &filter, 20, Some(cursor))
                .await
                .is_err()
        );
    }
    reader.close().await;
    importer.close().await;
}

#[tokio::test]
#[ignore = "explicit captured file-reference parity check"]
async fn captured_reference_parity() {
    use serde_json::Value;
    fn compare(expected: &Value, actual: &Value, path: &str) {
        match expected {
            Value::Object(fields) => {
                for (key, value) in fields {
                    if key == "generation" || key == "note" || key == "next_cursor" {
                        continue;
                    }
                    compare(value, &actual[key], &format!("{path}.{key}"));
                }
            }
            Value::Array(rows) => {
                assert_eq!(
                    rows.len(),
                    actual
                        .as_array()
                        .unwrap_or_else(|| panic!("missing {path}"))
                        .len(),
                    "{path}"
                );
                for (i, r) in rows.iter().enumerate() {
                    compare(r, &actual[i], &format!("{path}[{i}]"));
                }
            }
            _ => assert_eq!(expected, actual, "{path}"),
        }
    }
    let source = lctx_postgres::import::Source::open(std::path::Path::new(
        &std::env::var("LCTX_TEST_PROJECTION").unwrap(),
    ))
    .unwrap();
    let reference: Value = serde_json::from_slice(
        &std::fs::read(std::env::var("LCTX_TEST_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    let f = Fixture::start().await;
    let artifacts = tempfile::tempdir().unwrap();
    let mut config = f.config.clone();
    config.role = Role::Importer;
    config.url = config.url.replace("lctx_serving:", "lctx_importer:");
    config.statement_timeout_seconds = 30;
    let importer = config.open_importer().await.unwrap();
    importer
        .import(source.clone(), artifacts.path().to_owned())
        .await
        .unwrap();
    let mut config = f.config.clone();
    config.statement_timeout_seconds = 30;
    let reader = config.open_serving().await.unwrap();
    let pinned = reader
        .pin(
            &source.manifest().context.library,
            Some(source.generation()),
            None,
        )
        .await
        .unwrap();
    for (spelling, expected) in reference["operations"].as_object().unwrap() {
        let actual = reader
            .get_operation(&pinned, &pinned.manifest().snapshot_id, spelling)
            .await
            .unwrap();
        compare(expected, &actual, spelling);
    }
    for (id, expected) in reference["briefs"].as_object().unwrap() {
        let actual = reader
            .get_capability(&pinned, &pinned.manifest().snapshot_id, id)
            .await
            .unwrap();
        compare(expected, &actual, id);
    }
    for case in reference["find"].as_array().unwrap() {
        let filter = serde_json::from_value(case["filter"].clone()).unwrap();
        let actual = reader
            .find_operations(&pinned, &filter, 20, None)
            .await
            .unwrap();
        compare(&case["result"], &actual, "find");
    }
    reader.close().await;
    importer.close().await;
}

#[tokio::test]
#[ignore = "explicit exact-vector and ANN installation/qualification check"]
async fn exact_ranks_and_ann_profile_isolation() {
    use arrow_array::{Array, FixedSizeListArray, Float32Array};
    let root = std::path::PathBuf::from(std::env::var("LCTX_TEST_PROJECTION").unwrap());
    let source = lctx_postgres::import::Source::open(&root).unwrap();
    let f = Fixture::start().await;
    let artifacts = tempfile::tempdir().unwrap();
    let mut config = f.config.clone();
    config.role = Role::Importer;
    config.url = config.url.replace("lctx_serving:", "lctx_importer:");
    config.statement_timeout_seconds = 30;
    let importer = config.open_importer().await.unwrap();
    importer
        .import(source.clone(), artifacts.path().to_owned())
        .await
        .unwrap();
    let mut config = f.config.clone();
    config.statement_timeout_seconds = 30;
    let reader = config.open_serving().await.unwrap();
    let pinned = reader
        .pin(
            &source.manifest().context.library,
            Some(source.generation()),
            None,
        )
        .await
        .unwrap();
    let mut query = vec![0.0f32; 1024];
    query[0] = 1.0;
    for (operations, relation, key) in [
        (false, "vectors", "brief_id"),
        (true, "operation_vectors", "node_id"),
    ] {
        let ranks = reader
            .vector_ranks(
                &pinned,
                &query,
                pinned.manifest().spec_hash.as_deref().unwrap(),
                operations,
                None,
                10,
            )
            .await
            .unwrap();
        assert!(!ranks.metadata.approximate);
        assert!(ranks.metadata.fallback.is_none());
        let batches = arrow_ipc::reader::FileReader::try_new(
            std::fs::File::open(root.join(format!("{relation}.arrow"))).unwrap(),
            None,
        )
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        let mut expected = std::collections::BTreeMap::<(String, Vec<u8>), f64>::new();
        for batch in batches {
            let ids = batch
                .column_by_name(key)
                .unwrap()
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap();
            let views = batch
                .column_by_name("embedding_view")
                .map(|a| a.as_any().downcast_ref::<StringArray>().unwrap());
            let vectors = batch
                .column_by_name("vector")
                .unwrap()
                .as_any()
                .downcast_ref::<FixedSizeListArray>()
                .unwrap();
            for row in 0..batch.num_rows() {
                let v = vectors.value(row);
                let values = v.as_any().downcast_ref::<Float32Array>().unwrap();
                let norm = values
                    .values()
                    .iter()
                    .map(|v| f64::from(*v).powi(2))
                    .sum::<f64>()
                    .sqrt();
                let cosine = f64::from(values.value(0)) / norm;
                let view = views.map_or("brief", |v| v.value(row)).to_owned();
                let score = expected
                    .entry((view, ids.value(row).to_vec()))
                    .or_insert(-2.0);
                *score = score.max(cosine);
            }
        }
        let mut seen = 0;
        for batch in
            arrow_ipc::reader::StreamReader::try_new(std::io::Cursor::new(ranks.ipc), None).unwrap()
        {
            let batch = batch.unwrap();
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .unwrap();
            let views = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let actual_ranks = batch
                .column(2)
                .as_any()
                .downcast_ref::<arrow_array::UInt32Array>()
                .unwrap();
            let scores = batch
                .column(3)
                .as_any()
                .downcast_ref::<arrow_array::Float64Array>()
                .unwrap();
            for i in 0..batch.num_rows() {
                let want = expected[&(views.value(i).to_owned(), ids.value(i).to_vec())];
                assert!((scores.value(i) - want).abs() <= 1e-5);
                let mut order: Vec<_> = expected
                    .iter()
                    .filter(|((v, _), _)| v == views.value(i))
                    .collect();
                order.sort_by(|a, b| b.1.total_cmp(a.1).then(a.0.1.cmp(&b.0.1)));
                assert_eq!(
                    order[actual_ranks.value(i) as usize - 1].0.1,
                    ids.value(i),
                    "full rank drift cannot be hidden by score tolerance"
                );
                seen += 1;
            }
        }
        assert_eq!(seen, expected.len());
    }
    importer.build_hnsw(source.generation()).await.unwrap();
    importer.build_hnsw(source.generation()).await.unwrap();
    let ann =
        cpg_schema::Digest::from_hex(&lctx_postgres::profiles::Policy::hnsw().digest().unwrap())
            .unwrap();
    assert!(
        reader
            .pin(
                &source.manifest().context.library,
                Some(source.generation()),
                Some(ann)
            )
            .await
            .is_err()
    );
    let bogus = serde_json::json!({"format":1,"generation":source.generation().hex(),"spec":source.manifest().spec_hash,"maximum_ann_p95_ms":1000.0,"cases":[{"name":"foreign","stratum":"selective","operations":true,"vector":query,"eligible":["ff".repeat(16)],"promoted":[],"lexical":[],"reference":{"signature_doc":[],"source_body":[]}}]});
    assert!(
        importer
            .qualify_hnsw(&reader, &serde_json::to_vec(&bogus).unwrap())
            .await
            .is_err()
    );
    assert!(
        reader
            .pin(
                &source.manifest().context.library,
                Some(source.generation()),
                Some(ann)
            )
            .await
            .is_err()
    );
    reader.close().await;
    importer.close().await;
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL physical admission and recovery controls"]
async fn physical_admission_reindex_and_artifact_relocation() {
    use lctx_postgres::profiles::Policy;
    let source = lctx_postgres::import::Source::open(&std::path::PathBuf::from(
        std::env::var("LCTX_TEST_PROJECTION").unwrap(),
    ))
    .unwrap();
    let f = Fixture::start().await;
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let mut config = f.config.clone();
    config.role = Role::Importer;
    config.url = config.url.replace("lctx_serving:", "lctx_importer:");
    config.statement_timeout_seconds = 30;
    let importer = config.open_importer().await.unwrap();
    importer
        .import(source.clone(), first.path().to_owned())
        .await
        .unwrap();
    importer.build_hnsw(source.generation()).await.unwrap();
    let id = source.generation();
    let policy = Policy::mixed(1024, vec!["unfiltered".into()]);
    let profile = cpg_schema::Digest::from_hex(&policy.digest().unwrap()).unwrap();
    let realization: serde_json::Value =
        sqlx::query_scalar("SELECT lctx_serving.index_realization($1)")
            .bind(id.0.as_slice())
            .fetch_one(&f.serving)
            .await
            .unwrap();
    assert_eq!(realization["valid"], true);
    assert!(
        sqlx::query("SELECT lctx_serving.record_mixed_profile($1,$2,$3)")
            .bind(id.0.as_slice())
            .bind(policy.canonical().unwrap())
            .bind(serde_json::json!({"passed":true,"phase":"confirmation"}))
            .execute(&f.importer)
            .await
            .is_err()
    );
    // Synthetic measurement documents challenge the finite SQL transition and lifecycle.
    // These are fixture controls, never evidence of real ANN recall or latency qualification.
    let calibration = serde_json::json!({"fixture":true,"passed":false,"phase":"calibration","pack_sha256":"01".repeat(32),"chosen_policy":policy});
    sqlx::query("SELECT lctx_serving.record_mixed_profile($1,$2,$3)")
        .bind(id.0.as_slice())
        .bind(policy.canonical().unwrap())
        .bind(calibration)
        .execute(&f.importer)
        .await
        .unwrap();
    let run = serde_json::json!({"passed":true,"plans_passed":true,"ann_execution_passed":true,"recall_at_10":1.,"fused_recall_at_10":1.,"ann_p95_ms":1.,"exact_p95_ms":2.});
    let confirmation = serde_json::json!({"fixture":true,"passed":true,"phase":"confirmation","runner":2,"generation":id.hex(),"profile":profile.hex(),"policy":policy,"pack_sha256":"02".repeat(32),"calibration_sha256":"01".repeat(32),"realization":realization,"exact_reference_passed":true,"classes":[{"class":"unfiltered","queries":8,"runs":[run,run]}]});
    sqlx::query("SELECT lctx_serving.record_mixed_profile($1,$2,$3)")
        .bind(id.0.as_slice())
        .bind(policy.canonical().unwrap())
        .bind(confirmation)
        .execute(&f.importer)
        .await
        .unwrap();
    let reader = f.config.open_serving().await.unwrap();
    let lib = &source.manifest().context.library;
    importer.select(lib, id, profile).await.unwrap();
    let pin = reader.pin(lib, Some(id), Some(profile)).await.unwrap();
    assert_eq!(
        reader.diagnostics(id, true).await.unwrap()["generation"]["availability"],
        "available"
    );
    let index = format!("lctx_serving.o_{}_d_ann", &id.hex()[..48]);
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("REINDEX INDEX {index}")))
        .execute(&f.admin)
        .await
        .unwrap();
    let changed: serde_json::Value =
        sqlx::query_scalar("SELECT lctx_serving.index_realization($1)")
            .bind(id.0.as_slice())
            .fetch_one(&f.serving)
            .await
            .unwrap();
    assert_ne!(realization, changed);
    assert!(reader.pin(lib, Some(id), Some(profile)).await.is_err());
    assert!(importer.select(lib, id, profile).await.is_err());
    assert_eq!(pin.generation(), id.hex());
    // A retained reader must recheck physical admission on the actual ANN path.
    // The small default fixture stays exact; the pilot control has >1024 vector entities.
    let population:i64=sqlx::query_scalar("SELECT count(DISTINCT node_id) FROM lctx_serving.operation_vectors WHERE generation_digest=$1").bind(id.0.as_slice()).fetch_one(&f.serving).await.unwrap();
    if population > 1024 {
        let mut query = vec![0.0f32; 1024];
        query[0] = 1.;
        assert!(matches!(
            reader
                .vector_ranks(
                    &pin,
                    &query,
                    source.manifest().spec_hash.as_deref().unwrap(),
                    true,
                    None,
                    10
                )
                .await,
            Err(lctx_postgres::Error::Admission(_))
        ));
    }
    let exact = cpg_schema::Digest::from_hex(&Policy::exact().digest().unwrap()).unwrap();
    importer.select(lib, id, exact).await.unwrap();
    importer
        .import(source.clone(), second.path().to_owned())
        .await
        .unwrap();
    first.close().unwrap();
    importer
        .relocate_artifacts(id, second.path().to_owned())
        .await
        .unwrap();
    let relocated = reader.pin(lib, Some(id), None).await.unwrap();
    assert!(
        relocated
            .artifacts()
            .iter()
            .all(|(_, p)| p.starts_with(second.path()))
    );
    let (name, path) = relocated.artifacts()[0].clone();
    std::fs::write(&path, b"corrupt").unwrap();
    assert_eq!(
        reader.diagnostics(id, true).await.unwrap()["generation"]["availability"],
        "corrupt"
    );
    std::fs::remove_file(path).unwrap();
    assert_eq!(
        reader.diagnostics(id, true).await.unwrap()["generation"]["availability"],
        "missing",
        "{name}"
    );
    assert!(reader.pin(lib, Some(id), None).await.is_err());
    let mut invalid = config.clone();
    let mut url = url::Url::parse(&invalid.url).unwrap();
    url.set_port(Some(1)).unwrap();
    invalid.url = url.to_string();
    assert_eq!(invalid.diagnose(None, false).await.database, "unavailable");
    // Incompatible migrations must still be observable without normal repository admission.
    sqlx::query("UPDATE public._sqlx_migrations SET checksum=decode('00','hex') WHERE version=(SELECT max(version) FROM public._sqlx_migrations)").execute(&f.admin).await.unwrap();
    let diagnostic = config.diagnose(Some(id), false).await;
    assert_eq!(diagnostic.database, "available");
    assert!(!diagnostic.schema_current);
    assert!(diagnostic.failure.is_some());
    reader.close().await;
    importer.close().await;
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL publication fault controls"]
async fn interrupted_import_resumes_without_partial_visibility() {
    let source = lctx_postgres::import::Source::open(std::path::Path::new(
        &std::env::var("LCTX_TEST_PROJECTION").unwrap(),
    ))
    .unwrap();
    let f = Fixture::start().await;
    let artifacts = tempfile::tempdir().unwrap();
    let mut config = f.config.clone();
    config.role = Role::Importer;
    config.url = config.url.replace("lctx_serving:", "lctx_importer:");
    config.statement_timeout_seconds = 30;
    config.lock_timeout_seconds = 30;
    let importer = Arc::new(config.open_importer().await.unwrap());
    let mut lock = f.admin.begin().await.unwrap();
    sqlx::raw_sql("LOCK TABLE lctx_serving.import_batches IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let task = tokio::spawn({
        let store = importer.clone();
        let source = source.clone();
        let artifacts = artifacts.path().to_owned();
        async move { store.import(source, artifacts).await }
    });
    let mut blocked = false;
    for _ in 0..200 {
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%SELECT first_row,row_count,content_digest FROM lctx_serving.import_batches%'").fetch_one(&f.admin).await.unwrap();
        if count > 0 {
            blocked = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        blocked,
        "production import reached the locked batch boundary"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.generations")
            .fetch_one(&f.serving)
            .await
            .unwrap(),
        0
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    lock.rollback().await.unwrap();
    let resumed = importer
        .import(source.clone(), artifacts.path().to_owned())
        .await
        .unwrap();
    assert_eq!(resumed.state, "ready");
    let outcomes: Vec<String> =
        sqlx::query_scalar("SELECT outcome FROM lctx_serving.import_attempts ORDER BY attempt_id")
            .fetch_all(&f.admin)
            .await
            .unwrap();
    assert_eq!(outcomes, vec!["interrupted", "completed"]);
    let (left, right) = tokio::join!(
        importer.import(source.clone(), artifacts.path().to_owned()),
        importer.import(source.clone(), artifacts.path().to_owned())
    );
    assert_eq!(left.unwrap().generation, right.unwrap().generation);
    assert!(importer.cleanup(source.generation()).await.is_err());
    importer.close().await;
}

#[tokio::test]
#[ignore = "explicit conflicting receipt and incomplete validation controls"]
async fn publication_refuses_conflicts_and_incomplete_stored_content() {
    use sha2::Digest as _;
    let root = std::path::PathBuf::from(std::env::var("LCTX_TEST_PROJECTION").unwrap());
    let source = lctx_postgres::import::Source::open(&root).unwrap();
    let envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("MANIFEST.json")).unwrap()).unwrap();
    let files: std::collections::BTreeMap<_, _> = envelope["files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v["sha256"].as_str().unwrap().to_owned()))
        .collect();
    let transport = sha2::Sha256::digest(serde_json::to_vec(&files).unwrap());
    for conflict in [true, false] {
        let f = Fixture::start().await;
        let artifacts = tempfile::tempdir().unwrap();
        let doc = serde_json::to_string(source.manifest()).unwrap();
        sqlx::query("SELECT lctx_serving.prepare_generation($1,$2)")
            .bind(&doc)
            .bind(transport.as_slice())
            .execute(&f.admin)
            .await
            .unwrap();
        assert!(
            sqlx::query("SELECT lctx_serving.prepare_generation($1,$2)")
                .bind(&doc)
                .bind([0u8; 32].as_slice())
                .execute(&f.admin)
                .await
                .is_err(),
            "transport retry conflicts before loading"
        );
        if conflict {
            let name = source
                .manifest()
                .relations
                .iter()
                .find(|(_, r)| r.rows > 0)
                .unwrap()
                .0;
            sqlx::query("INSERT INTO lctx_serving.import_batches VALUES($1,$2,0,0,1,$3)")
                .bind(source.generation().0.as_slice())
                .bind(name)
                .bind([0u8; 32].as_slice())
                .execute(&f.admin)
                .await
                .unwrap();
        } else {
            for (name, receipt) in &source.manifest().artifacts {
                sqlx::query("SELECT lctx_serving.register_artifact($1,$2,$3,$4,$5,$6)")
                    .bind(source.generation().0.as_slice())
                    .bind(name)
                    .bind(
                        cpg_schema::Digest::from_hex(&receipt.sha256)
                            .unwrap()
                            .0
                            .as_slice(),
                    )
                    .bind(receipt.bytes as i64)
                    .bind(receipt.format as i32)
                    .bind(root.join(name).to_str().unwrap())
                    .execute(&f.admin)
                    .await
                    .unwrap();
            }
            sqlx::query("SELECT lctx_serving.freeze_generation($1)")
                .bind(source.generation().0.as_slice())
                .execute(&f.admin)
                .await
                .unwrap();
        }
        let mut config = f.config.clone();
        config.role = Role::Importer;
        config.url = config.url.replace("lctx_serving:", "lctx_importer:");
        config.statement_timeout_seconds = 30;
        let importer = config.open_importer().await.unwrap();
        let error = importer
            .import(source.clone(), artifacts.path().to_owned())
            .await
            .unwrap_err();
        if conflict {
            assert!(
                error.to_string().contains("batch receipt conflict"),
                "{error}"
            );
        }
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT state FROM lctx_serving.generations")
                .fetch_one(&f.admin)
                .await
                .unwrap(),
            "failed",
            "{error:?}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.generations")
                .fetch_one(&f.serving)
                .await
                .unwrap(),
            0
        );
        importer.cleanup(source.generation()).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.import_recipes")
                .fetch_one(&f.admin)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lctx_serving.artifact_locations")
                .fetch_one(&f.admin)
                .await
                .unwrap(),
            0
        );
        assert!(
            importer
                .import(source.clone(), artifacts.path().to_owned())
                .await
                .is_err(),
            "failed generation remains terminal after cleanup"
        );
        importer.close().await;
    }
}
