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
