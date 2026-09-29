//! The Arrow → binary COPY → provider type matrix (cutover plan WP0.6): every column type the
//! contracts use round-trips through a published generation as an identical batch, and the pin
//! admits only what each role may read.

use arrow_array::RecordBatch;
use cpg_core::session;
use cpg_core::store_read::{CanonicalReader, ReadRelation};
use lctx_model::ddl::{self, TableSpec};
use lctx_model::decl::codebook::{Codebook, CodebookEntry};
use lctx_model::decl::column::Blob;
use lctx_model::decl::relation::Relation;
use lctx_model::id::{Digest, Id, IdKind};
use lctx_model::{model, relation};
use lctx_postgres::serving::{Role, RoleConfig, TEST_IMAGE};
use lctx_postgres::store::{CANONICAL, GenerationDigests, RelationReceipt, Writer};
use lctx_postgres::{Config, MigrationStore};
use sqlx::PgPool;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};

relation! {
    /// Every contract column type, required and optional.
    Matrix, MatrixRow = "matrix" {
        layer: L0, family: "type_matrix", stage: "extract",
        fidelity: Extracted, polarity: Exact, coverage: "module",
        key: [row_id],
    }
    row {
        row_id: Id,
        maybe_id: Option<Id>,
        digest: Digest,
        maybe_digest: Option<Digest>,
        text: String,
        maybe_text: Option<String>,
        count: i64,
        maybe_count: Option<i64>,
        flag: bool,
        maybe_flag: Option<bool>,
        kind: IdKind,
        maybe_kind: Option<IdKind>,
        score: f64,
        maybe_score: Option<f64>,
        raw: Blob,
        tags: Vec<String>,
        history: Vec<f64>,
        vector: Vec<f32>,
    }
}

model! { Matrix }

fn rows() -> Vec<MatrixRow> {
    (0u8..6)
        .map(|n| MatrixRow {
            row_id: Id([n; 16]),
            maybe_id: (n % 2 == 0).then_some(Id([n + 100; 16])),
            digest: Digest([n; 32]),
            maybe_digest: (n % 3 == 0).then_some(Digest([n + 1; 32])),
            text: ["", "ascii", "ünïcödé", "tab\tnewline\n", "quote'\"", "\\x00"][n as usize].to_owned(),
            maybe_text: (n % 2 == 1).then(|| format!("t{n}")),
            count: [i64::MIN, -1, 0, 1, i64::MAX, 42][n as usize],
            maybe_count: (n % 2 == 0).then_some(-7),
            flag: n % 2 == 0,
            maybe_flag: [None, Some(true), Some(false)][n as usize % 3],
            kind: IdKind::all()[n as usize],
            maybe_kind: (n % 2 == 0).then(|| IdKind::all()[n as usize + 1]),
            score: [0.0, -0.0, f64::NAN, f64::INFINITY, 1e-300, -2.5][n as usize],
            maybe_score: (n % 3 == 1).then_some(f64::NEG_INFINITY),
            raw: Blob(vec![n, 0, 255]),
            tags: (0..n).map(|i| format!("tag{i}")).collect(),
            history: (0..n).map(|i| f64::from(i) / 3.0).collect(),
            vector: (0..n).map(|i| f32::from(i) * 0.5).collect(),
        })
        .collect()
}

struct Db {
    _container: ContainerAsync<Postgres>,
    _owner: MigrationStore,
    writer: Writer,
    reader: RoleConfig,
    writer_config: RoleConfig,
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
    let url = |role: &str, password: &str| format!("postgres://{role}:{password}@127.0.0.1:{port}/postgres");
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
    let tables = DECLS.iter().map(|d| TableSpec::from_decl(d).unwrap()).collect();
    owner
        .install(&ddl::install(tables, &[CodebookEntry::of::<IdKind>()], &CANONICAL).unwrap())
        .await
        .unwrap();
    let role = |role: Role| RoleConfig {
        format: 1,
        role,
        url: url(role.name(), "test"),
        max_connections: 2,
        provider_connections: 0,
        acquire_timeout_seconds: 5,
        statement_timeout_seconds: 60,
        lock_timeout_seconds: 5,
    };
    let writer_config = role(Role::Importer);
    Db {
        _container: container,
        _owner: owner,
        writer: Writer::open(&writer_config).await.unwrap(),
        reader: role(Role::Serving),
        writer_config,
    }
}

async fn stage(db: &Db, g: Id, batch: &RecordBatch) {
    let digest = db.writer.installed_digest().await.unwrap().unwrap();
    db.writer.create(g, "lib", "catalog", digest).await.unwrap();
    let relation = &db.writer.relations().await.unwrap()[0];
    let rows = db.writer.copy(g, relation, batch).await.unwrap();
    db.writer
        .validate_relation(g, &RelationReceipt { relation: "matrix".into(), row_count: rows, schema_digest: Digest([9; 32]) })
        .await
        .unwrap();
}

async fn read(config: &RoleConfig, g: Id) -> datafusion::error::Result<Vec<RecordBatch>> {
    let reader = CanonicalReader::open(config, 1).await.unwrap();
    let ctx = session::session();
    reader
        .register(
            &ctx,
            g,
            &[ReadRelation {
                name: "matrix".into(),
                declared: Matrix::DECL.schema(),
                supplied_partition: Some("generation_id".into()),
            }],
        )
        .await?;
    ctx.sql("SELECT * FROM matrix ORDER BY row_id").await?.collect().await
}

/// Floats compare by bits, so NaN and -0.0 are checked exactly.
fn same(a: &[MatrixRow], b: &[MatrixRow]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert_eq!(x.score.to_bits(), y.score.to_bits(), "score of {:?}", x.row_id);
        let strip = |r: &MatrixRow| MatrixRow { score: 0.0, ..r.clone() };
        assert_eq!(strip(x), strip(y));
    }
}

#[tokio::test]
#[ignore = "explicit real PostgreSQL functional check"]
async fn every_column_type_round_trips_through_a_published_generation() {
    let db = start().await;
    let g = Id([1; 16]);
    let original = rows();
    stage(&db, g, &Matrix::to_batch(&original).unwrap()).await;
    // Unpublished: only the writer inspects it; the reader is refused, never answered empty.
    assert!(read(&db.reader, g).await.is_err());
    let staged: Vec<MatrixRow> = read(&db.writer_config, g)
        .await
        .unwrap()
        .iter()
        .flat_map(|b| Matrix::from_batch(b).unwrap())
        .collect();
    same(&staged, &original);
    db.writer
        .mark_validated(g, GenerationDigests { compiler: Digest([1; 32]), producer: Digest([1; 32]), content: Digest([1; 32]) }, &serde_json::json!({}))
        .await
        .unwrap();
    db.writer.publish(g).await.unwrap();
    let batches = read(&db.reader, g).await.unwrap();
    let schema = batches[0].schema();
    for (field, declared) in schema.fields().iter().zip(Matrix::DECL.schema().fields()) {
        assert_eq!(field.data_type(), declared.data_type(), "{}", field.name());
    }
    let published: Vec<MatrixRow> = batches.iter().flat_map(|b| Matrix::from_batch(b).unwrap()).collect();
    same(&published, &original);
    // A generation that does not exist is refused.
    assert!(read(&db.reader, Id([2; 16])).await.is_err());
}
