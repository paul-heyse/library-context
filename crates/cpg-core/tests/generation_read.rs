//! Generation-bound provider sessions against a disposable real PostgreSQL 18 (cutover plan
//! P1.10, T13; core review C02; review focus #4). Every control states its answer first.
use cpg_core::generation_read::{GenerationSession, InspectionSession, ProviderOptions, ReadError};
use datafusion::{
    arrow::array::{Int64Array, RecordBatch, StringArray},
    common::ScalarValue,
    datasource::TableProvider,
    error::DataFusionError,
    execution::runtime_env::RuntimeEnvBuilder,
    logical_expr::{Expr, TableProviderFilterPushDown},
    prelude::{SessionConfig, SessionContext, col, lit},
};
use datafusion_table_providers_postgres::{bounded::ChunkLimits, pool::PoolHealth};
use futures::StreamExt;
use lctx_model::domain::{
    artifact::*, attribution::*, input::*, source::*, stages::*, transfer::local::TransferKey, *,
};
use lctx_postgres::generations::{
    CleanupOutcome, Error as StoreError, GenerationCatalog, GenerationId, GenerationStore,
};
use lctx_postgres::roles::{Role, RoleConfig};
use lctx_postgres::testing::Harness;
use lctx_postgres::testing::{
    DisposableDatabase,
    fixtures::{Facts, budget},
};
use std::{sync::Arc, time::Duration};

/// A relation only another model has.
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "session_probes", semantic_source = include_bytes!("generation_read.rs"))]
struct Probe {
    #[model(key)]
    name: String,
}

fn serving(db: &DisposableDatabase) -> RoleConfig {
    RoleConfig {
        format: 1,
        role: Role::Serving,
        url: db.url("lctx_serving"),
        max_connections: 3,
        provider_connections: 2,
        acquire_timeout_seconds: 5,
        statement_timeout_seconds: 60,
        lock_timeout_seconds: 10,
    }
}
fn options() -> ProviderOptions {
    ProviderOptions {
        acquire_timeout: Duration::from_secs(2),
        ..ProviderOptions::default()
    }
}
async fn collect(table: Arc<dyn TableProvider>) -> datafusion::error::Result<Vec<RecordBatch>> {
    SessionContext::new().read_table(table)?.collect().await
}
fn decode<R: Record>(batches: &[RecordBatch]) -> Vec<R> {
    let mut rows: Vec<R> = batches.iter().flat_map(|b| R::decode(b).unwrap()).collect();
    rows.sort_by_key(Record::id);
    rows
}
/// The typed read error at the root of a DataFusion error, if any.
fn read_error(error: &DataFusionError) -> Option<&ReadError> {
    match error.find_root() {
        DataFusionError::External(inner) => inner.downcast_ref::<ReadError>(),
        _ => None,
    }
}
fn strings(batches: &[RecordBatch]) -> Vec<String> {
    batches
        .iter()
        .flat_map(|b| {
            b.column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .iter()
                .map(|v| v.unwrap().to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}
fn count(batches: &[RecordBatch]) -> i64 {
    batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap()
        .value(0)
}
async fn provider_backends(db: &DisposableDatabase) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT pid FROM pg_stat_activity WHERE application_name = 'lctx-provider' ORDER BY pid",
    )
    .fetch_all(&db.superuser)
    .await
    .unwrap()
}

/// A published conformance generation holding lists, a sum, fixed binaries, a 65 MiB chunked
/// artifact, three packages, releases whose versions differ in case, contexts whose optional lock
/// digest is null, `d1` or `d2`, and one empty relation.
struct Rich {
    store: GenerationStore,
    generation: GenerationId,
    chunks: Vec<ArtifactChunk>,
    contexts: Vec<AnalysisContext>,
    scopes: Vec<CoverageScope>,
    packages: Vec<Package>,
    locks: [ContentHash; 2],
}
async fn rich(db: &DisposableDatabase) -> Rich {
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut g_h = Harness::begin(
        &store,
        db.writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let g = g_h.generation();
    let bytes: Vec<u8> = (0..65 * ARTIFACT_CHUNK_BYTES + 17)
        .map(|i| ((i / ARTIFACT_CHUNK_BYTES + i) % 256) as u8)
        .collect();
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "evidence.bin".into(),
        content: ContentHash::of(&bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let artifact = SourceArtifact::from_bytes(input.id(), "evidence.bin".into(), &bytes).unwrap();
    let chunks: Vec<ArtifactChunk> = ArtifactChunk::split(&artifact, &bytes).unwrap().collect();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec!["$input".into(), "src".into()],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"cfg"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let locks = [ContentHash::of(b"d1"), ContentHash::of(b"d2")];
    let contexts: Vec<AnalysisContext> = [None, Some(locks[0]), Some(locks[1])]
        .into_iter()
        .map(|lock_digest| AnalysisContext {
            lock_digest,
            ..context.clone()
        })
        .collect();
    let scopes = vec![
        CoverageScope::Input { input: input.id() },
        CoverageScope::Artifact {
            artifact: artifact.id(),
        },
    ];
    let packages: Vec<Package> = ["alpha", "beta", "gamma"]
        .iter()
        .map(|n| Package { name: (*n).into() })
        .collect();
    macro_rules! copy {
        ($rows:expr) => {
            g_h.copy(&Batch::new(&model, $rows, &budget()).unwrap(), &budget())
                .await
                .unwrap()
        };
    }
    let releases = ["Beta", "alpha"]
        .iter()
        .map(|v| Release {
            package: packages[0].id(),
            version: (*v).into(),
        })
        .collect();
    copy!(vec![input]);
    copy!(vec![artifact]);
    copy!(contexts.clone());
    copy!(scopes.clone());
    copy!(packages.clone());
    copy!(releases);
    for chunk in &chunks {
        copy!(vec![chunk.clone()]);
    }
    g_h.seal().await.unwrap();
    g_h.validate(&budget()).await.unwrap();
    g_h.publish().await.unwrap();
    Rich {
        store,
        generation: g,
        chunks,
        contexts,
        scopes,
        packages,
        locks,
    }
}

#[tokio::test]
async fn provider_reads_equal_typed_readback() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(
        &serving(&db),
        Arc::new(model().unwrap()),
        fixture.generation,
        options(),
    )
    .await
    .unwrap();
    let batches = collect(session.table::<ArtifactChunk>().unwrap())
        .await
        .unwrap();
    assert!(batches.len() > 1, "65 MiB arrives in byte-bounded batches");
    let mut chunks = fixture.chunks.clone();
    chunks.sort_by_key(Record::id);
    assert_eq!(decode::<ArtifactChunk>(&batches), chunks);
    let mut contexts = fixture.contexts.clone();
    contexts.sort_by_key(Record::id);
    assert_eq!(
        decode::<AnalysisContext>(
            &collect(session.table::<AnalysisContext>().unwrap())
                .await
                .unwrap()
        ),
        contexts,
        "lists and nulls round-trip"
    );
    let mut scopes = fixture.scopes.clone();
    scopes.sort_by_key(Record::id);
    assert_eq!(
        decode::<CoverageScope>(
            &collect(session.table::<CoverageScope>().unwrap())
                .await
                .unwrap()
        ),
        scopes,
        "sums round-trip"
    );
    let empty = collect(session.table::<CorpusLibrary>().unwrap())
        .await
        .unwrap();
    assert_eq!(empty.iter().map(RecordBatch::num_rows).sum::<usize>(), 0);
    // The typed SQLx readback of the same generation agrees.
    let mut lease = fixture
        .store
        .pin(&db.reader, fixture.generation, budget())
        .await
        .unwrap();
    let mut typed = Vec::new();
    lease
        .visit::<ArtifactChunk>(|batch| {
            typed.extend(batch.rows().iter().cloned());
            Ok(())
        })
        .await
        .unwrap();
    typed.sort_by_key(Record::id);
    assert_eq!(typed, chunks);
    lease.release().await.unwrap();
    session.close().await.unwrap();
}

#[tokio::test]
async fn closed_filter_pushdown() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(
        &serving(&db),
        Arc::new(model().unwrap()),
        fixture.generation,
        options(),
    )
    .await
    .unwrap();
    let packages = session.table::<Package>().unwrap();
    let chunks = session.table::<ArtifactChunk>().unwrap();
    let id = fixture.packages[1].id();
    let exact = [
        col("name").eq(lit("beta")),
        col("id").eq(lit(datafusion::common::ScalarValue::FixedSizeBinary(
            16,
            Some(id.bytes().to_vec()),
        ))),
        col("name").is_not_null(),
    ];
    let unsupported = [col("name").lt(lit("beta")), col("name").like(lit("b%"))];
    for filter in &exact {
        assert_eq!(
            packages.supports_filters_pushdown(&[filter]).unwrap(),
            [TableProviderFilterPushDown::Exact],
            "{filter}"
        );
    }
    for filter in &unsupported {
        assert_eq!(
            packages.supports_filters_pushdown(&[filter]).unwrap(),
            [TableProviderFilterPushDown::Unsupported],
            "{filter}"
        );
    }
    assert_eq!(
        chunks
            .supports_filters_pushdown(&[&col("ordinal").gt_eq(lit(64i64))])
            .unwrap(),
        [TableProviderFilterPushDown::Exact],
        "integers order"
    );
    let ctx = SessionContext::new();
    ctx.register_table("packages", packages).unwrap();
    ctx.register_table("artifact_chunks", chunks).unwrap();
    for (sql, expected) in [
        (
            "SELECT name FROM packages WHERE name = 'beta'",
            vec!["beta"],
        ),
        (
            "SELECT name FROM packages WHERE name < 'beta' ORDER BY name",
            vec!["alpha"],
        ),
        (
            "SELECT name FROM packages WHERE name LIKE '%a' ORDER BY name",
            vec!["alpha", "beta", "gamma"],
        ),
    ] {
        let batches = ctx.sql(sql).await.unwrap().collect().await.unwrap();
        let names: Vec<String> = batches
            .iter()
            .flat_map(|b| {
                b.column(0)
                    .as_any()
                    .downcast_ref::<datafusion::arrow::array::StringArray>()
                    .unwrap()
                    .iter()
                    .map(|v| v.unwrap().to_owned())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(names, expected, "{sql}");
    }
    let tail = ctx
        .sql("SELECT count(*) FROM artifact_chunks WHERE ordinal >= 64")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(count(&tail), 2);
    // Executed known answers for the pushed-down bytea rewrite and three-valued NULL logic
    // (P1.10 review F02). Each filter is pushed down whole.
    let beta = ctx
        .table("packages")
        .await
        .unwrap()
        .filter(exact[1].clone())
        .unwrap()
        .select_columns(&["name"])
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(
        strings(&beta),
        ["beta"],
        "an identity equality selects exactly its row"
    );
    let contexts = session.table::<AnalysisContext>().unwrap();
    ctx.register_table("analysis_contexts", contexts.clone())
        .unwrap();
    let d1 = || {
        lit(ScalarValue::FixedSizeBinary(
            32,
            Some(fixture.locks[0].0.to_vec()),
        ))
    };
    let null_logic = [
        (col("lock_digest").not_eq(d1()), 1),
        (Expr::Not(Box::new(col("lock_digest").eq(d1()))), 1),
        (col("lock_digest").eq(d1()), 1),
        (col("lock_digest").is_null(), 1),
        (
            col("lock_digest")
                .is_not_null()
                .and(col("lock_digest").not_eq(d1())),
            1,
        ),
    ];
    for (filter, expected) in null_logic {
        assert_eq!(
            contexts.supports_filters_pushdown(&[&filter]).unwrap(),
            [TableProviderFilterPushDown::Exact],
            "{filter}"
        );
        let rows = ctx
            .table("analysis_contexts")
            .await
            .unwrap()
            .filter(filter.clone())
            .unwrap()
            .count()
            .await
            .unwrap();
        assert_eq!(
            rows, expected,
            "a null lock digest is neither equal nor unequal: {filter}"
        );
    }
    // Text orders by bytes, as DataFusion does: `Beta` sorts before `alpha`. A linguistic
    // collation, which the server offers, orders them the other way.
    ctx.register_table("releases", session.table::<Release>().unwrap())
        .unwrap();
    let before = ctx
        .sql("SELECT version FROM releases WHERE version < 'alpha' ORDER BY version")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(strings(&before), ["Beta"]);
    let linguistic: bool = sqlx::query_scalar("SELECT 'Beta' < ('alpha' COLLATE \"und-x-icu\")")
        .fetch_one(&db.superuser)
        .await
        .unwrap();
    assert!(
        !linguistic,
        "the negative twin: a pushed-down linguistic order would return nothing"
    );
    let plan = ctx
        .sql("SELECT name FROM packages WHERE name = 'beta'")
        .await
        .unwrap()
        .create_physical_plan()
        .await
        .unwrap();
    let shown = datafusion::physical_plan::displayable(plan.as_ref())
        .indent(true)
        .to_string();
    assert!(
        shown.contains("WHERE") && shown.contains("GenerationScan"),
        "{shown}"
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn digest_and_column_mismatch_rejected_before_scan() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let model = Arc::new(model().unwrap());
    let unpublished_h = Harness::begin(
        &fixture.store,
        db.writer.clone(),
        lctx_model::domain::stages::Profile::Catalog,
        budget(),
    )
    .await
    .unwrap();
    let unpublished = unpublished_h.generation();
    assert!(matches!(
        GenerationSession::open(&serving(&db), model.clone(), unpublished, options()).await,
        Err(ReadError::Store(StoreError::State))
    ));
    let other = {
        let mut relations = model.relations().to_vec();
        relations.push(Relation::of::<Probe>());
        Arc::new(ValidatedModel::validate(relations).unwrap())
    };
    assert!(
        matches!(
            GenerationSession::open(&serving(&db), other, fixture.generation, options()).await,
            Err(ReadError::Store(StoreError::Contract))
        ),
        "another model's digests are refused"
    );
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "ALTER TABLE {}.packages ADD COLUMN extra integer",
        fixture.generation.schema()
    )))
    .execute(db.owner.pool())
    .await
    .unwrap();
    assert!(
        matches!(
            GenerationSession::open(&serving(&db), model.clone(), fixture.generation, options())
                .await,
            Err(ReadError::Store(StoreError::Contract))
        ),
        "live columns that differ from the lowering are refused before any scan"
    );
    assert!(
        provider_backends(&db).await.is_empty(),
        "a refused session keeps no connection"
    );
    let held: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_locks WHERE locktype = 'advisory' AND mode = 'ShareLock'",
    )
    .fetch_one(&db.superuser)
    .await
    .unwrap();
    assert_eq!(held, 0, "and no lease");
}

#[tokio::test]
async fn pin_survives_selection_change_and_frontier_is_enforced() {
    let db = DisposableDatabase::start().await;
    let facts = Facts::new();
    // The second generation's analysis context differs, so a read tells the two apart.
    let other = Facts {
        context: AnalysisContext {
            python_version: "3.13.9".into(),
            ..facts.context.clone()
        },
        ..Facts::new()
    };
    let store = GenerationStore::install(db.owner.clone(), facts.model.clone())
        .await
        .unwrap();
    let first = facts.published(&store, db.writer.clone()).await;
    let second = other.published(&store, db.writer.clone()).await;
    store.select(first).await.unwrap();
    let session = GenerationSession::open(&serving(&db), facts.model.clone(), first, options())
        .await
        .unwrap();
    store.select(second).await.unwrap();
    let context = decode::<AnalysisContext>(
        &collect(session.table::<AnalysisContext>().unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(
        context,
        std::slice::from_ref(&facts.context),
        "the session still reads its own generation, not the selected one"
    );
    let selected = GenerationSession::open(&serving(&db), facts.model.clone(), second, options())
        .await
        .unwrap();
    assert_eq!(
        decode::<AnalysisContext>(
            &collect(selected.table::<AnalysisContext>().unwrap())
                .await
                .unwrap()
        ),
        std::slice::from_ref(&other.context),
        "the negative twin: the selected generation reads differently"
    );
    selected.close().await.unwrap();
    assert_eq!(session.generation(), first);
    assert!(
        matches!(store.retire(first).await, Err(StoreError::Busy)),
        "the unselected generation is still leased"
    );
    // F02 at the reader: nothing above the facts frontier exists to read as empty.
    assert!(matches!(
        session.table::<TransferKey>(),
        Err(ReadError::Frontier(_))
    ));
    let inspection = InspectionSession::new(session).unwrap();
    // A relation above the frontier plans (P1.11 registers it) and refuses, typed, when scanned.
    let refused = inspection
        .query("SELECT count(*) FROM transfer_keys")
        .await
        .unwrap_err();
    assert!(
        matches!(read_error(&refused), Some(ReadError::Frontier(_))),
        "{refused}"
    );
    let counted = inspection
        .query("SELECT count(*) FROM provider_coverage")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(count(&counted), 3);
    let ddl = inspection
        .query("CREATE TABLE leak AS SELECT * FROM provider_coverage")
        .await
        .unwrap_err();
    assert!(
        matches!(read_error(&ddl), Some(ReadError::ReadOnly(_))),
        "{ddl}"
    );
    inspection.close().await.unwrap();
    assert_eq!(store.retire(first).await.unwrap(), CleanupOutcome::Removed);
}

#[tokio::test]
async fn lease_blocks_retire_close_releases() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(
        &serving(&db),
        Arc::new(model().unwrap()),
        fixture.generation,
        options(),
    )
    .await
    .unwrap();
    assert_eq!(
        provider_backends(&db).await.len(),
        2,
        "two prefilled connections, each leased"
    );
    assert!(matches!(
        fixture.store.retire(fixture.generation).await,
        Err(StoreError::Busy)
    ));
    session.close().await.unwrap();
    assert_eq!(
        fixture.store.retire(fixture.generation).await.unwrap(),
        CleanupOutcome::Removed
    );
}

#[tokio::test]
async fn byte_bounded_batches_and_reservation_refusal() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let narrow = ProviderOptions {
        chunks: ChunkLimits {
            rows: 2,
            ..ChunkLimits::default()
        },
        ..options()
    };
    let session = GenerationSession::open(
        &serving(&db),
        Arc::new(model().unwrap()),
        fixture.generation,
        narrow,
    )
    .await
    .unwrap();
    let batches = collect(session.table::<Package>().unwrap()).await.unwrap();
    assert_eq!(
        batches
            .iter()
            .map(RecordBatch::num_rows)
            .collect::<Vec<_>>(),
        [2, 1],
        "rows are bounded per batch"
    );
    let batches = collect(session.table::<ArtifactChunk>().unwrap())
        .await
        .unwrap();
    assert!(batches.iter().all(|b| b.num_rows() <= 2));
    // Default limits: no batch holds more than 8 MiB of rows (8 whole chunks).
    let session_default = {
        session.close().await.unwrap();
        GenerationSession::open(
            &serving(&db),
            Arc::new(model().unwrap()),
            fixture.generation,
            options(),
        )
        .await
        .unwrap()
    };
    let batches = collect(session_default.table::<ArtifactChunk>().unwrap())
        .await
        .unwrap();
    assert!(
        batches.iter().all(|b| b.num_rows() <= 8),
        "{:?}",
        batches
            .iter()
            .map(RecordBatch::num_rows)
            .collect::<Vec<_>>()
    );
    // A pool too small for one chunk batch refuses, and the session stays healthy.
    let runtime = RuntimeEnvBuilder::new()
        .with_memory_limit(1 << 20, 1.0)
        .build_arc()
        .unwrap();
    let small = SessionContext::new_with_config_rt(SessionConfig::new(), runtime);
    let refused = small
        .read_table(session_default.table::<ArtifactChunk>().unwrap())
        .unwrap()
        .collect()
        .await;
    assert!(
        matches!(&refused, Err(error) if error.to_string().contains("Resources exhausted")),
        "{refused:?}"
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(matches!(
        session_default.health(),
        PoolHealth::Ready { connections: 2, .. }
    ));
    assert_eq!(
        collect(session_default.table::<Package>().unwrap())
            .await
            .unwrap()
            .iter()
            .map(RecordBatch::num_rows)
            .sum::<usize>(),
        3
    );
    session_default.close().await.unwrap();
}

#[tokio::test]
async fn cancellation_mid_stream_drains_and_returns() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(
        &serving(&db),
        Arc::new(model().unwrap()),
        fixture.generation,
        options(),
    )
    .await
    .unwrap();
    let before = provider_backends(&db).await;
    assert_eq!(before.len(), 2);
    for _ in 0..3 {
        let mut stream = SessionContext::new()
            .read_table(session.table::<ArtifactChunk>().unwrap())
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        let first = stream.next().await.unwrap().unwrap();
        assert!(first.num_rows() > 0);
        drop(stream);
    }
    for _ in 0..50 {
        if matches!(session.health(), PoolHealth::Ready { idle: 2, .. }) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        matches!(
            session.health(),
            PoolHealth::Ready {
                connections: 2,
                idle: 2
            }
        ),
        "{:?}",
        session.health()
    );
    assert_eq!(
        provider_backends(&db).await,
        before,
        "drained connections return; none is replaced"
    );
    // A drain keeps each connection's lease: the generation still has two readers and cannot retire.
    let detail = GenerationCatalog::new(db.reader.clone())
        .show(fixture.generation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.summary.readers, 2);
    assert!(matches!(
        fixture.store.retire(fixture.generation).await,
        Err(StoreError::Busy)
    ));
    assert_eq!(
        decode::<ArtifactChunk>(
            &collect(session.table::<ArtifactChunk>().unwrap())
                .await
                .unwrap()
        )
        .len(),
        fixture.chunks.len()
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn transport_loss_is_terminal() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(
        &serving(&db),
        Arc::new(model().unwrap()),
        fixture.generation,
        options(),
    )
    .await
    .unwrap();
    assert!(matches!(session.health(), PoolHealth::Ready { .. }));
    sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name = 'lctx-provider'").execute(&db.superuser).await.unwrap();
    let lost = collect(session.table::<Package>().unwrap())
        .await
        .unwrap_err();
    assert!(
        matches!(read_error(&lost), Some(ReadError::Lost)),
        "a lost session reads nothing: {lost}"
    );
    assert_eq!(session.health(), PoolHealth::Lost);
    let again = collect(session.table::<Package>().unwrap())
        .await
        .unwrap_err();
    assert!(
        matches!(read_error(&again), Some(ReadError::Lost)),
        "and never recovers: {again}"
    );
    assert!(
        provider_backends(&db).await.is_empty(),
        "no connection was opened to replace the lost ones"
    );
    assert!(matches!(session.close().await, Err(ReadError::Lost)));
    assert_eq!(
        fixture.store.retire(fixture.generation).await.unwrap(),
        CleanupOutcome::Removed,
        "the lost leases are gone with their backends"
    );
}

#[tokio::test]
async fn stage_session_reads_only_its_completed_source_and_admits_all_scan_instances() {
    use cpg_core::{
        generation_read::AttemptSession,
        model_runtime::{AttemptRuntime, RuntimeOptions},
    };
    let db = DisposableDatabase::start().await;
    let config_dir = tempfile::tempdir().unwrap();
    db.write_configs(config_dir.path()).unwrap();
    let mut importer = RoleConfig::load(&config_dir.path().join("postgres-importer.json")).unwrap();
    let model = Arc::new(
        ValidatedModel::validate(vec![Relation::of::<Package>(), Relation::of::<Release>()])
            .unwrap(),
    );
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let stage = |name, inputs, outputs| Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![Profile::Catalog],
        effect: Effect::Store,
        code: ContentHash::of(b"stage reader"),
        configuration: ContentHash::of(b"config"),
    };
    let schedule = Schedule::build(
        &model,
        vec![
            stage("source", vec![], vec![RelationUse::of::<Package>()]),
            stage(
                "consumer",
                vec![RelationUse::stored::<Package>()],
                vec![RelationUse::of::<Release>()],
            ),
        ],
        &[],
        Profile::Catalog,
    )
    .unwrap();
    let runtime = AttemptRuntime::new(RuntimeOptions::default()).unwrap();
    let mut execution = schedule.execute();
    let attempt = store
        .begin_conformance(db.writer.clone(), &mut execution, runtime.budget().clone())
        .await
        .unwrap();
    let mut source = StageOutput::new(
        execution.begin("source").unwrap(),
        &attempt,
        &model,
        runtime.budget().clone(),
        Default::default(),
    )
    .unwrap();
    source.declare::<Package>().unwrap();
    for name in ["alpha", "beta", "gamma"] {
        source.push(Package { name: name.into() }).await.unwrap();
    }
    source.finish(ProviderOutcome::Complete).await.unwrap();
    let consumer = execution.begin("consumer").unwrap();
    let read = AttemptSession::open(&importer, &attempt, &consumer, model.clone(), options())
        .await
        .unwrap();
    assert!(
        AttemptSession::open(&importer, &attempt, &consumer, model.clone(), options())
            .await
            .is_err(),
        "one importer provider budget cannot be allocated twice"
    );
    assert!(
        AttemptSession::open(&serving(&db), &attempt, &consumer, model.clone(), options())
            .await
            .is_err(),
        "serving cannot read staging outputs"
    );
    let stage = runtime.session(&consumer);
    let permit = consumer.read::<Package>().unwrap();
    stage
        .register(&permit, read.table(&permit).unwrap())
        .unwrap();
    assert!(
        stage
            .register(&permit, read.table(&permit).unwrap())
            .is_err()
    );
    let sql = "SELECT a.name AS a, b.name AS b, c.name AS c FROM packages a JOIN packages b ON a.id=b.id JOIN packages c ON b.id=c.id";
    let planned = stage.query(sql).await.unwrap();
    assert_eq!(
        planned.scan_demand(),
        3,
        "aliases consume separate physical scans"
    );
    assert_eq!(
        planned
            .collect()
            .await
            .unwrap()
            .iter()
            .map(RecordBatch::num_rows)
            .sum::<usize>(),
        3
    );
    read.close().await.unwrap();
    assert!(
        stage
            .query("SELECT * FROM packages")
            .await
            .unwrap()
            .collect()
            .await
            .is_err(),
        "retained tables cannot read after terminal close"
    );
    importer.provider_connections = 1;
    importer.max_connections = 3;
    let read = AttemptSession::open(&importer, &attempt, &consumer, model.clone(), options())
        .await
        .unwrap();
    let limited = runtime.session(&consumer);
    limited
        .register(&permit, read.table(&permit).unwrap())
        .unwrap();
    assert!(
        matches!(
            limited.query(sql).await,
            Err(datafusion::error::DataFusionError::ResourcesExhausted(_))
        ),
        "whole demand refuses before acquiring one connection"
    );
    read.close().await.unwrap();
    let mut output = StageOutput::new(
        consumer,
        &attempt,
        &model,
        runtime.budget().clone(),
        Default::default(),
    )
    .unwrap();
    output.declare::<Release>().unwrap();
    output.finish(ProviderOutcome::Complete).await.unwrap();
    attempt
        .seal(execution.finish().unwrap())
        .await
        .unwrap()
        .validate()
        .await
        .unwrap()
        .abort()
        .await
        .unwrap();
}

/// P1.10 review F05: inspection queries run in a bounded pool.
#[tokio::test]
async fn inspection_memory_is_bounded() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let small = ProviderOptions {
        inspection_memory: 1 << 20,
        ..options()
    };
    let session = GenerationSession::open(
        &serving(&db),
        Arc::new(model().unwrap()),
        fixture.generation,
        small,
    )
    .await
    .unwrap();
    let inspection = InspectionSession::new(session).unwrap();
    let refused = inspection
        .query("SELECT * FROM artifact_chunks ORDER BY ordinal DESC")
        .await
        .unwrap()
        .collect()
        .await;
    assert!(
        matches!(&refused, Err(error) if error.to_string().contains("Resources exhausted")),
        "{refused:?}"
    );
    for _ in 0..50 {
        if matches!(inspection.health(), PoolHealth::Ready { idle: 2, .. }) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        matches!(
            inspection.health(),
            PoolHealth::Ready { connections: 2, .. }
        ),
        "{:?}",
        inspection.health()
    );
    assert_eq!(
        count(
            &inspection
                .query("SELECT count(*) FROM packages")
                .await
                .unwrap()
                .collect()
                .await
                .unwrap()
        ),
        3,
        "the session stays usable"
    );
    inspection.close().await.unwrap();
}

/// P1.10 review F03: a provider connection classifies SQLSTATEs as the store does. A lock timeout
/// while taking the lease is contention.
#[tokio::test]
async fn a_lease_lock_timeout_is_contention() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let mut holder = db.superuser.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(1279476824, 0)")
        .execute(&mut *holder)
        .await
        .unwrap();
    let impatient = RoleConfig {
        lock_timeout_seconds: 1,
        ..serving(&db)
    };
    let refused = GenerationSession::open(
        &impatient,
        Arc::new(model().unwrap()),
        fixture.generation,
        options(),
    )
    .await;
    assert!(
        matches!(&refused, Err(ReadError::Store(StoreError::Driver { class: lctx_model::domain::Infrastructure::Contention, detail }))
        if detail.contains("55P03")),
        "{:?}",
        refused.as_ref().err()
    );
    sqlx::query("SELECT pg_advisory_unlock(1279476824, 0)")
        .execute(&mut *holder)
        .await
        .unwrap();
    drop(holder);
    GenerationSession::open(
        &impatient,
        Arc::new(model().unwrap()),
        fixture.generation,
        options(),
    )
    .await
    .unwrap()
    .close()
    .await
    .unwrap();
}
