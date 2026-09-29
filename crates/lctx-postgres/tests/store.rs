//! The canonical store kernel on a real PostgreSQL 18 (cutover plan WP0.6): the generation
//! lifecycle, invisibility of unpublished attempts, database refusals, privileges, pinned readers
//! and the DDL digest.

use arrow_array::RecordBatch;
use lctx_model::ddl::{self, Install, TableSpec};
use lctx_model::decl::codebook::CodebookEntry;
use lctx_model::decl::column::Blob;
use lctx_model::decl::relation::Relation;
use lctx_model::id::{Digest, Id, IdKind};

/// Sample recipes for the retained `recipe!` machinery; the production recipe catalog is retired
/// (the typed `domain` owns identity).
mod recipes {
    use lctx_model::id::Id;
    lctx_model::recipe!(occurrence, OCCURRENCE = Occurrence { module: Id, start: i64, end: i64, syntax_kind: &str });
    lctx_model::recipe!(symbol_key, SYMBOL_KEY = SymbolKey { distribution: &str, qualified_path: &str, descriptor: &str });
}
use lctx_model::{model, relation};
use lctx_postgres::serving::TEST_IMAGE;
use lctx_postgres::store::{self, CANONICAL, GenerationDigests, RelationReceipt, Writer};
use lctx_postgres::{Config, Error, MigrationStore};
use sqlx::PgPool;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};

relation! {
    /// Entities.
    Entities, EntitiesRow = "entities" {
        layer: L1, family: "store_test", stage: "normalize",
        fidelity: Resolved, polarity: Exact, coverage: "release",
        key: [entity_id],
    }
    row { entity_id: Id, name: String }
}

relation! {
    /// Every legacy column type, with checks, a role and a codebook.
    Samples, SamplesRow = "samples" {
        layer: L0, family: "store_test", stage: "extract",
        fidelity: Extracted, polarity: Exact, coverage: "module",
        key: [sample_id],
        checks: [("span_ordered", "start_byte <= end_byte")],
    }
    row {
        sample_id: Id,
        owner_id: Id [ref entities.entity_id],
        digest: Digest,
        start_byte: i64,
        end_byte: i64,
        kind: IdKind,
        label: Option<String>,
        flag: bool,
        maybe_flag: Option<bool>,
        score: f64,
        maybe_count: Option<i64>,
        raw: Blob,
        tags: Vec<String>,
        history: Vec<f64>,
        vector: Vec<f32>,
    }
}

model! { Entities, Samples }

fn install() -> Install {
    let tables = DECLS
        .iter()
        .map(|d| TableSpec::from_decl(d).unwrap())
        .collect();
    ddl::install(tables, &[CodebookEntry::of::<IdKind>()], &CANONICAL).unwrap()
}

struct Db {
    _container: ContainerAsync<Postgres>,
    owner: MigrationStore,
    writer: Writer,
    writer_pool: PgPool,
    reader: PgPool,
}

async fn start() -> Db {
    let (name, tag) = TEST_IMAGE.trim().split_once(':').unwrap();
    let container = Postgres::default()
        .with_name(name)
        .with_tag(tag)
        .start()
        .await
        .expect("Docker and the pinned pgvector PG18 image are required");
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = |role: &str, password: &str| {
        format!("postgres://{role}:{password}@127.0.0.1:{port}/postgres")
    };
    let admin = PgPool::connect(&url("postgres", "postgres")).await.unwrap();
    lctx_postgres::bootstrap::disposable_cluster(&admin, "test").await.unwrap();
    let config = Config {
        application_url: url("lctx_app", "test"),
        migration_url: url("lctx_migrator", "test"),
        migration_config: None,
        max_connections: 2,
        acquire_timeout_seconds: 5,
        statement_timeout_seconds: 60,
        lock_timeout_seconds: 5,
        max_receipt_bytes: 268435456,
    };
    let owner = config.connect_migrator().await.unwrap();
    owner.migrate().await.unwrap();
    owner.install(&install()).await.unwrap();
    let writer_pool = PgPool::connect(&url("lctx_importer", "test")).await.unwrap();
    let reader = PgPool::connect(&url("lctx_serving", "test")).await.unwrap();
    Db {
        _container: container,
        owner,
        writer: Writer::from_pool(writer_pool.clone()),
        writer_pool,
        reader,
    }
}

fn entities(ids: &[u8]) -> RecordBatch {
    Entities::to_batch(
        &ids.iter()
            .map(|n| EntitiesRow {
                entity_id: Id([*n; 16]),
                name: format!("e{n}"),
            })
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

fn sample(n: u8, owner: u8) -> SamplesRow {
    SamplesRow {
        sample_id: recipes::occurrence(Id([n; 16]), 0, i64::from(n), "call"),
        owner_id: Id([owner; 16]),
        digest: Digest([n; 32]),
        start_byte: 0,
        end_byte: i64::from(n),
        kind: IdKind::Transfer,
        label: (n % 2 == 0).then(|| format!("label {n}")),
        flag: n % 3 == 0,
        maybe_flag: None,
        score: -0.0,
        maybe_count: Some(-7),
        raw: Blob(vec![0, 255, n]),
        tags: vec!["a".into(), String::new()],
        history: vec![f64::NAN, 1.5],
        vector: vec![0.25, -1.0],
    }
}

/// Load, validate and publish one generation.
async fn publish(db: &Db, g: Id, entity_ids: &[u8], samples: &[SamplesRow]) -> Result<(), Error> {
    let digest = install().digest;
    db.writer.create(g, "lib", "catalog", digest).await?;
    load(db, g, entity_ids, samples).await?;
    db.writer
        .mark_validated(
            g,
            GenerationDigests {
                compiler: Digest([1; 32]),
                producer: Digest([2; 32]),
                content: Digest([3; 32]),
            },
            &serde_json::json!({}),
        )
        .await?;
    db.writer.publish(g).await
}

async fn load(db: &Db, g: Id, entity_ids: &[u8], samples: &[SamplesRow]) -> Result<(), Error> {
    let relations = db.writer.relations().await?;
    let batches = [entities(entity_ids), Samples::to_batch(samples).unwrap()];
    for (relation, batch) in relations.iter().zip(&batches) {
        let rows = db.writer.copy(g, relation, batch).await?;
        db.writer
            .validate_relation(
                g,
                &RelationReceipt {
                    relation: relation.name.clone(),
                    row_count: rows,
                    schema_digest: Digest([9; 32]),
                },
            )
            .await?;
    }
    Ok(())
}

async fn reader_count(db: &Db, table: &str, g: Id) -> i64 {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM lctx.{table} WHERE generation_id = $1"
    )))
    .bind(g.0.as_slice())
    .fetch_one(&db.reader)
    .await
    .unwrap()
}

fn sqlstate(error: &Error) -> String {
    match error {
        Error::Database { code, .. } => code.clone(),
        other => format!("{other}"),
    }
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn a_generation_publishes_and_is_read_through_its_parents() {
    let db = start().await;
    let g = Id([1; 16]);
    assert_eq!(reader_count(&db, "samples", g).await, 0);
    publish(&db, g, &[7], &[sample(1, 7), sample(2, 7)]).await.unwrap();
    assert_eq!(reader_count(&db, "entities", g).await, 1);
    assert_eq!(reader_count(&db, "samples", g).await, 2);
    db.writer.select(g).await.unwrap();
    let row = db.writer.generation(g).await.unwrap().unwrap();
    assert_eq!((row.state.as_str(), row.library.as_str()), ("published", "lib"));
    assert_eq!(row.content_digest, Some(Digest([3; 32])));
    // Values survive the round trip: floats by bits, nulls, lists, the codebook code.
    let (score, history, kind, label): (f64, Vec<f64>, i16, Option<String>) = sqlx::query_as(
        "SELECT score, history, kind, label FROM lctx.samples WHERE generation_id = $1 ORDER BY end_byte LIMIT 1",
    )
    .bind(g.0.as_slice())
    .fetch_one(&db.reader)
    .await
    .unwrap();
    assert!(score == 0.0 && score.is_sign_negative());
    assert!(history[0].is_nan() && history[1] == 1.5);
    assert_eq!(kind, 9);
    assert_eq!(label, None);
    // The registry and events are readable; the receipts cover every relation.
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT event FROM lctx_store.generation_events WHERE generation_id = $1 ORDER BY ordinal",
    )
    .bind(g.0.as_slice())
    .fetch_all(&db.reader)
    .await
    .unwrap();
    assert_eq!(events, ["created", "validated", "published", "selected"]);
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn an_unpublished_or_failed_attempt_is_invisible_and_a_retry_is_a_new_generation() {
    let db = start().await;
    let g = Id([2; 16]);
    db.writer.create(g, "lib", "catalog", install().digest).await.unwrap();
    load(&db, g, &[7], &[sample(1, 7)]).await.unwrap();
    assert_eq!(reader_count(&db, "samples", g).await, 0, "staging is invisible");
    let schema = ddl::generation_schema(g);
    let denied = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM \"{schema}\".samples")))
        .execute(&db.reader)
        .await;
    assert!(denied.is_err(), "the reader has no access to a generation schema");
    db.writer.fail(g, "validators failed").await.unwrap();
    assert_eq!(db.writer.generation(g).await.unwrap().unwrap().state, "failed");
    // The failed attempt stays inspectable by the writer.
    let staged: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM \"{schema}\".samples")))
        .fetch_one(&db.writer_pool)
        .await
        .unwrap();
    assert_eq!(staged, 1);
    assert!(db.writer.publish(g).await.is_err(), "a failed generation never publishes");
    let retry = db.writer.create(g, "lib", "catalog", install().digest).await.unwrap_err();
    assert_eq!(sqlstate(&retry), "LX002");
    db.owner.retire(g).await.unwrap();
    assert_eq!(db.writer.generation(g).await.unwrap().unwrap().state, "retired");
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn the_database_refuses_key_null_check_codebook_and_reference_violations() {
    let db = start().await;
    let digest = install().digest;
    // Duplicate key: refused when the key index is built.
    let g = Id([3; 16]);
    db.writer.create(g, "lib", "catalog", digest).await.unwrap();
    let relations = db.writer.relations().await.unwrap();
    db.writer.copy(g, &relations[0], &entities(&[7, 7])).await.unwrap();
    let err = db
        .writer
        .validate_relation(g, &RelationReceipt { relation: "entities".into(), row_count: 2, schema_digest: Digest([9; 32]) })
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err), "23505");
    // A copied row count that disagrees with the receipt is refused.
    let g = Id([4; 16]);
    db.writer.create(g, "lib", "catalog", digest).await.unwrap();
    db.writer.copy(g, &relations[0], &entities(&[7])).await.unwrap();
    let err = db
        .writer
        .validate_relation(g, &RelationReceipt { relation: "entities".into(), row_count: 2, schema_digest: Digest([9; 32]) })
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err), "LX005");
    // A CHECK violation is refused at COPY.
    let g = Id([5; 16]);
    db.writer.create(g, "lib", "catalog", digest).await.unwrap();
    let mut bad = sample(3, 7);
    bad.start_byte = 10;
    let err = db
        .writer
        .copy(g, &relations[1], &Samples::to_batch(&[bad]).unwrap())
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err), "23514");
    // A codebook code outside the codebook, and a reference to a missing entity, are refused at
    // publication, and nothing becomes visible.
    for (n, codebook) in [(6u8, true), (8, false)] {
        let g = Id([n; 16]);
        let mut row = sample(n, if codebook { 7 } else { 99 });
        let batch = if codebook {
            let good = Samples::to_batch(&[row.clone()]).unwrap();
            let mut columns = good.columns().to_vec();
            let index = good.schema().index_of("kind").unwrap();
            columns[index] = std::sync::Arc::new(arrow_array::Int16Array::from(vec![99i16]));
            RecordBatch::try_new(good.schema(), columns).unwrap()
        } else {
            row.label = None;
            Samples::to_batch(&[row]).unwrap()
        };
        db.writer.create(g, "lib", "catalog", digest).await.unwrap();
        db.writer.copy(g, &relations[0], &entities(&[7])).await.unwrap();
        db.writer.copy(g, &relations[1], &batch).await.unwrap();
        for (relation, rows) in [("entities", 1), ("samples", 1)] {
            db.writer
                .validate_relation(g, &RelationReceipt { relation: relation.into(), row_count: rows, schema_digest: Digest([9; 32]) })
                .await
                .unwrap();
        }
        db.writer
            .mark_validated(g, GenerationDigests { compiler: Digest([1; 32]), producer: Digest([1; 32]), content: Digest([1; 32]) }, &serde_json::json!({}))
            .await
            .unwrap();
        let err = db.writer.publish(g).await.unwrap_err();
        assert_eq!(sqlstate(&err), "23503", "codebook {codebook}");
        assert_eq!(reader_count(&db, "samples", g).await, 0);
        assert_eq!(reader_count(&db, "entities", g).await, 0, "publication is one transaction");
    }
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn a_published_generation_is_immutable_to_the_writer() {
    let db = start().await;
    let g = Id([10; 16]);
    publish(&db, g, &[7], &[sample(1, 7)]).await.unwrap();
    let schema = ddl::generation_schema(g);
    for sql in [
        format!("INSERT INTO \"{schema}\".entities VALUES ('\\x{0}'::bytea, '\\x{1}'::bytea, 'x')", g.hex(), Id([8; 16]).hex()),
        format!("UPDATE \"{schema}\".entities SET name = 'y'"),
        format!("INSERT INTO lctx.entities VALUES ('\\x{0}'::bytea, '\\x{1}'::bytea, 'x')", g.hex(), Id([8; 16]).hex()),
        "UPDATE lctx.entities SET name = 'y'".to_owned(),
        "DELETE FROM lctx.entities".to_owned(),
    ] {
        let err = sqlx::query(sqlx::AssertSqlSafe(sql.clone())).execute(&db.writer_pool).await;
        assert!(err.is_err(), "{sql}");
    }
    for sql in ["DELETE FROM lctx.entities", "UPDATE lctx_store.generations SET state = 'failed'"] {
        assert!(sqlx::query(sql).execute(&db.reader).await.is_err(), "{sql}");
    }
    assert_eq!(reader_count(&db, "entities", g).await, 1);
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn a_pinned_reader_is_unaffected_by_later_publication_and_retirement() {
    let db = start().await;
    let (a, b) = (Id([11; 16]), Id([12; 16]));
    publish(&db, a, &[7], &[sample(1, 7)]).await.unwrap();
    db.writer.select(a).await.unwrap();
    let before: Vec<Vec<u8>> = sqlx::query_scalar("SELECT sample_id FROM lctx.samples WHERE generation_id = $1 ORDER BY sample_id")
        .bind(a.0.as_slice())
        .fetch_all(&db.reader)
        .await
        .unwrap();
    publish(&db, b, &[7, 8], &[sample(2, 8), sample(3, 7)]).await.unwrap();
    let after: Vec<Vec<u8>> = sqlx::query_scalar("SELECT sample_id FROM lctx.samples WHERE generation_id = $1 ORDER BY sample_id")
        .bind(a.0.as_slice())
        .fetch_all(&db.reader)
        .await
        .unwrap();
    assert_eq!(before, after);
    db.owner.retire(b).await.unwrap();
    assert_eq!(reader_count(&db, "samples", b).await, 0);
    assert_eq!(reader_count(&db, "samples", a).await, 1);
    assert!(db.owner.retire(a).await.is_err(), "a selected generation is not retired");
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn a_different_canonical_schema_is_refused_until_reset() {
    let db = start().await;
    let g = Id([13; 16]);
    publish(&db, g, &[7], &[sample(1, 7)]).await.unwrap();
    let err = db.writer.create(Id([14; 16]), "lib", "catalog", Digest([0; 32])).await.unwrap_err();
    assert_eq!(sqlstate(&err), "LX001");
    // A changed contract: one relation only.
    let changed = ddl::install(
        vec![TableSpec::from_decl(&Entities::DECL).unwrap()],
        &[],
        &CANONICAL,
    )
    .unwrap();
    assert!(matches!(db.owner.install(&changed).await, Err(Error::CanonicalSchema)));
    assert_eq!(db.owner.install(&install()).await.unwrap(), store::InstallOutcome::AlreadyCurrent);
    db.owner.reset(&changed).await.unwrap();
    assert_eq!(db.writer.installed_digest().await.unwrap(), Some(changed.digest));
    assert!(db.writer.generation(g).await.unwrap().is_none(), "reset drops every generation");
    assert!(sqlx::query("SELECT count(*) FROM lctx.samples").execute(&db.reader).await.is_err());
}

/// The generated column types are exactly what the binary COPY encoder writes.
#[test]
fn generated_column_types_equal_the_copy_encoding() {
    for decl in DECLS {
        let spec = TableSpec::from_decl(decl).unwrap();
        let encoded = store::copy_types(&decl.schema()).unwrap();
        for (name, pg) in encoded {
            let column = spec.columns.iter().find(|c| c.name == name).unwrap();
            assert_eq!(column.sql_type.pg_name().to_uppercase(), pg, "{}.{name}", decl.name);
        }
    }
}
