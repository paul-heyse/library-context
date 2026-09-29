//! Generation-bound provider sessions against a disposable real PostgreSQL 18 (cutover plan
//! P1.10, T13; core review C02; review focus #4). Every control states its answer first.
use std::{sync::Arc, time::Duration};
use cpg_core::generation_read::{GenerationSession, InspectionSession, ProviderOptions, ReadError};
use datafusion::{arrow::array::RecordBatch, datasource::TableProvider, execution::runtime_env::RuntimeEnvBuilder,
    logical_expr::TableProviderFilterPushDown, prelude::{SessionConfig, SessionContext, col, lit}};
use datafusion_table_providers_postgres::{bounded::ChunkLimits, pool::PoolHealth};
use futures::StreamExt;
use lctx_model::domain::{*, artifact::*, attribution::*, input::*, source::*, stages::*, transfer::TransferKey};
use lctx_postgres::generations::{CleanupOutcome, Error as StoreError, GenerationId, GenerationStore};
use lctx_postgres::roles::{Role, RoleConfig};
use lctx_postgres::testing::Harness;
use lctx_postgres::testing::{DisposableDatabase, fixtures::{Facts, budget}};

/// A relation only another model has.
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name = "session_probes", semantic_source = include_bytes!("generation_read.rs"))]
struct Probe { #[model(key)] name: String }

fn serving(db: &DisposableDatabase) -> RoleConfig {
    RoleConfig { format: 1, role: Role::Serving, url: db.url("lctx_serving"), max_connections: 3, provider_connections: 2,
        acquire_timeout_seconds: 5, statement_timeout_seconds: 60, lock_timeout_seconds: 10 }
}
fn options() -> ProviderOptions { ProviderOptions { acquire_timeout: Duration::from_secs(2), ..ProviderOptions::default() } }
async fn collect(table: Arc<dyn TableProvider>) -> datafusion::error::Result<Vec<RecordBatch>> {
    SessionContext::new().read_table(table)?.collect().await
}
fn decode<R: Record>(batches: &[RecordBatch]) -> Vec<R> {
    let mut rows: Vec<R> = batches.iter().flat_map(|b| R::decode(b).unwrap()).collect();
    rows.sort_by_key(Record::id);
    rows
}
async fn provider_backends(db: &DisposableDatabase) -> Vec<i32> {
    sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE application_name = 'lctx-provider' ORDER BY pid").fetch_all(&db.superuser).await.unwrap()
}

/// A published conformance generation holding lists, a sum, fixed binaries, a 65 MiB chunked
/// artifact, three packages and one empty relation.
struct Rich { store: GenerationStore, generation: GenerationId, chunks: Vec<ArtifactChunk>, context: AnalysisContext, scopes: Vec<CoverageScope>, packages: Vec<Package> }
async fn rich(db: &DisposableDatabase) -> Rich {
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let mut g_h = Harness::begin(&store, db.writer.clone(), lctx_model::domain::stages::Profile::Catalog, budget()).await.unwrap(); let g = g_h.generation();
    let bytes: Vec<u8> = (0..65 * ARTIFACT_CHUNK_BYTES + 17).map(|i| ((i / ARTIFACT_CHUNK_BYTES + i) % 256) as u8).collect();
    let input = InputRevision::from_entries(vec![ManifestEntry { path: "evidence.bin".into(), content: ContentHash::of(&bytes), byte_len: bytes.len() as i64 }]).unwrap();
    let artifact = SourceArtifact::from_bytes(input.id(), "evidence.bin".into(), &bytes).unwrap();
    let chunks: Vec<ArtifactChunk> = ArtifactChunk::split(&artifact, &bytes).unwrap().collect();
    let context = AnalysisContext { python_version: "3.14.7".into(), python_platform: "linux".into(), search_path: vec!["$input".into(), "src".into()],
        site_package_path: vec![], config_digest: ContentHash::of(b"cfg"), environment_digest: input.manifest, lock_digest: None };
    let scopes = vec![CoverageScope::Input { input: input.id() }, CoverageScope::Artifact { artifact: artifact.id() }];
    let packages: Vec<Package> = ["alpha", "beta", "gamma"].iter().map(|n| Package { name: (*n).into() }).collect();
    macro_rules! copy { ($rows:expr) => { g_h.copy(&Batch::new(&model, $rows, &budget()).unwrap(), &budget()).await.unwrap() }; }
    copy!(vec![input]); copy!(vec![artifact]); copy!(vec![context.clone()]); copy!(scopes.clone()); copy!(packages.clone());
    for chunk in &chunks { copy!(vec![chunk.clone()]); }
    g_h.seal().await.unwrap();
    g_h.validate(&budget()).await.unwrap();
    g_h.publish().await.unwrap();
    Rich { store, generation: g, chunks, context, scopes, packages }
}

#[tokio::test]
async fn provider_reads_equal_typed_readback() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(&serving(&db), Arc::new(model().unwrap()), fixture.generation, options()).await.unwrap();
    let batches = collect(session.table::<ArtifactChunk>().unwrap()).await.unwrap();
    assert!(batches.len() > 1, "65 MiB arrives in byte-bounded batches");
    let mut chunks = fixture.chunks.clone(); chunks.sort_by_key(Record::id);
    assert_eq!(decode::<ArtifactChunk>(&batches), chunks);
    assert_eq!(decode::<AnalysisContext>(&collect(session.table::<AnalysisContext>().unwrap()).await.unwrap()), [fixture.context.clone()], "lists round-trip");
    let mut scopes = fixture.scopes.clone(); scopes.sort_by_key(Record::id);
    assert_eq!(decode::<CoverageScope>(&collect(session.table::<CoverageScope>().unwrap()).await.unwrap()), scopes, "sums round-trip");
    let empty = collect(session.table::<Release>().unwrap()).await.unwrap();
    assert_eq!(empty.iter().map(RecordBatch::num_rows).sum::<usize>(), 0);
    // The typed SQLx readback of the same generation agrees.
    let mut lease = fixture.store.pin(&db.reader, fixture.generation, budget()).await.unwrap();
    let mut typed = Vec::new();
    lease.visit::<ArtifactChunk>(|batch| { typed.extend(batch.rows().iter().cloned()); Ok(()) }).await.unwrap();
    typed.sort_by_key(Record::id);
    assert_eq!(typed, chunks);
    lease.release().await.unwrap();
    session.close().await.unwrap();
}

#[tokio::test]
async fn closed_filter_pushdown() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(&serving(&db), Arc::new(model().unwrap()), fixture.generation, options()).await.unwrap();
    let packages = session.table::<Package>().unwrap();
    let chunks = session.table::<ArtifactChunk>().unwrap();
    let id = fixture.packages[1].id();
    let exact = [col("name").eq(lit("beta")), col("id").eq(lit(datafusion::common::ScalarValue::FixedSizeBinary(16, Some(id.bytes().to_vec())))),
        col("name").is_not_null()];
    let unsupported = [col("name").lt(lit("beta")), col("name").like(lit("b%"))];
    for filter in &exact { assert_eq!(packages.supports_filters_pushdown(&[filter]).unwrap(), [TableProviderFilterPushDown::Exact], "{filter}"); }
    for filter in &unsupported { assert_eq!(packages.supports_filters_pushdown(&[filter]).unwrap(), [TableProviderFilterPushDown::Unsupported], "{filter}"); }
    assert_eq!(chunks.supports_filters_pushdown(&[&col("ordinal").gt_eq(lit(64i64))]).unwrap(), [TableProviderFilterPushDown::Exact], "integers order");
    let ctx = SessionContext::new();
    ctx.register_table("packages", packages).unwrap();
    ctx.register_table("artifact_chunks", chunks).unwrap();
    for (sql, expected) in [("SELECT name FROM packages WHERE name = 'beta'", vec!["beta"]), ("SELECT name FROM packages WHERE name < 'beta' ORDER BY name", vec!["alpha"]),
        ("SELECT name FROM packages WHERE name LIKE '%a' ORDER BY name", vec!["alpha", "beta", "gamma"])] {
        let batches = ctx.sql(sql).await.unwrap().collect().await.unwrap();
        let names: Vec<String> = batches.iter().flat_map(|b| b.column(0).as_any().downcast_ref::<datafusion::arrow::array::StringArray>().unwrap()
            .iter().map(|v| v.unwrap().to_owned()).collect::<Vec<_>>()).collect();
        assert_eq!(names, expected, "{sql}");
    }
    let tail = ctx.sql("SELECT count(*) FROM artifact_chunks WHERE ordinal >= 64").await.unwrap().collect().await.unwrap();
    assert_eq!(tail[0].column(0).as_any().downcast_ref::<datafusion::arrow::array::Int64Array>().unwrap().value(0), 2);
    let plan = ctx.sql("SELECT name FROM packages WHERE name = 'beta'").await.unwrap().create_physical_plan().await.unwrap();
    let shown = datafusion::physical_plan::displayable(plan.as_ref()).indent(true).to_string();
    assert!(shown.contains("WHERE") && shown.contains("GenerationScan"), "{shown}");
    session.close().await.unwrap();
}

#[tokio::test]
async fn digest_and_column_mismatch_rejected_before_scan() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let model = Arc::new(model().unwrap());
    let unpublished_h = Harness::begin(&fixture.store, db.writer.clone(), lctx_model::domain::stages::Profile::Catalog, budget()).await.unwrap(); let unpublished = unpublished_h.generation();
    assert!(matches!(GenerationSession::open(&serving(&db), model.clone(), unpublished, options()).await, Err(ReadError::Store(StoreError::State))));
    let other = { let mut relations = model.relations().to_vec(); relations.push(Relation::of::<Probe>()); Arc::new(ValidatedModel::validate(relations).unwrap()) };
    assert!(matches!(GenerationSession::open(&serving(&db), other, fixture.generation, options()).await, Err(ReadError::Store(StoreError::Contract))),
        "another model's digests are refused");
    sqlx::query(sqlx::AssertSqlSafe(format!("ALTER TABLE {}.packages ADD COLUMN extra integer", fixture.generation.schema()))).execute(db.owner.pool()).await.unwrap();
    assert!(matches!(GenerationSession::open(&serving(&db), model.clone(), fixture.generation, options()).await, Err(ReadError::Store(StoreError::Contract))),
        "live columns that differ from the lowering are refused before any scan");
    assert!(provider_backends(&db).await.is_empty(), "a refused session keeps no connection");
    let held: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_locks WHERE locktype = 'advisory' AND mode = 'ShareLock'").fetch_one(&db.superuser).await.unwrap();
    assert_eq!(held, 0, "and no lease");
}

#[tokio::test]
async fn pin_survives_selection_change_and_frontier_is_enforced() {
    let db = DisposableDatabase::start().await;
    let facts = Facts::new();
    let store = GenerationStore::install(db.owner.clone(), facts.model.clone()).await.unwrap();
    let first = facts.published(&store, db.writer.clone()).await;
    let second = facts.published(&store, db.writer.clone()).await;
    store.select(first).await.unwrap();
    let session = GenerationSession::open(&serving(&db), facts.model.clone(), first, options()).await.unwrap();
    store.select(second).await.unwrap();
    let coverage = decode::<ProviderCoverage>(&collect(session.table::<ProviderCoverage>().unwrap()).await.unwrap());
    assert_eq!(coverage.len(), 2, "the session still reads its own generation");
    assert_eq!(session.generation(), first);
    assert!(matches!(store.retire(first).await, Err(StoreError::Busy)), "the unselected generation is still leased");
    // F02 at the reader: nothing above the facts frontier exists to read as empty.
    assert!(matches!(session.table::<TransferKey>(), Err(ReadError::Frontier(_))));
    let inspection = InspectionSession::new(session).unwrap();
    assert!(inspection.sql("SELECT count(*) FROM transfer_keys").await.is_err());
    let counted = inspection.sql("SELECT count(*) FROM provider_coverage").await.unwrap().collect().await.unwrap();
    assert_eq!(counted[0].column(0).as_any().downcast_ref::<datafusion::arrow::array::Int64Array>().unwrap().value(0), 2);
    assert!(inspection.sql("CREATE TABLE leak AS SELECT * FROM provider_coverage").await.is_err());
    inspection.close().await.unwrap();
    assert_eq!(store.retire(first).await.unwrap(), CleanupOutcome::Removed);
}

#[tokio::test]
async fn lease_blocks_retire_close_releases() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(&serving(&db), Arc::new(model().unwrap()), fixture.generation, options()).await.unwrap();
    assert_eq!(provider_backends(&db).await.len(), 2, "two prefilled connections, each leased");
    assert!(matches!(fixture.store.retire(fixture.generation).await, Err(StoreError::Busy)));
    session.close().await.unwrap();
    assert_eq!(fixture.store.retire(fixture.generation).await.unwrap(), CleanupOutcome::Removed);
}

#[tokio::test]
async fn byte_bounded_batches_and_reservation_refusal() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let narrow = ProviderOptions { chunks: ChunkLimits { rows: 2, ..ChunkLimits::default() }, ..options() };
    let session = GenerationSession::open(&serving(&db), Arc::new(model().unwrap()), fixture.generation, narrow).await.unwrap();
    let batches = collect(session.table::<Package>().unwrap()).await.unwrap();
    assert_eq!(batches.iter().map(RecordBatch::num_rows).collect::<Vec<_>>(), [2, 1], "rows are bounded per batch");
    let batches = collect(session.table::<ArtifactChunk>().unwrap()).await.unwrap();
    assert!(batches.iter().all(|b| b.num_rows() <= 2));
    // Default limits: no batch holds more than 8 MiB of rows (8 whole chunks).
    let session_default = { session.close().await.unwrap(); GenerationSession::open(&serving(&db), Arc::new(model().unwrap()), fixture.generation, options()).await.unwrap() };
    let batches = collect(session_default.table::<ArtifactChunk>().unwrap()).await.unwrap();
    assert!(batches.iter().all(|b| b.num_rows() <= 8), "{:?}", batches.iter().map(RecordBatch::num_rows).collect::<Vec<_>>());
    // A pool too small for one chunk batch refuses, and the session stays healthy.
    let runtime = RuntimeEnvBuilder::new().with_memory_limit(1 << 20, 1.0).build_arc().unwrap();
    let small = SessionContext::new_with_config_rt(SessionConfig::new(), runtime);
    let refused = small.read_table(session_default.table::<ArtifactChunk>().unwrap()).unwrap().collect().await;
    assert!(matches!(&refused, Err(error) if error.to_string().contains("Resources exhausted")), "{refused:?}");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(matches!(session_default.health(), PoolHealth::Ready { connections: 2, .. }));
    assert_eq!(collect(session_default.table::<Package>().unwrap()).await.unwrap().iter().map(RecordBatch::num_rows).sum::<usize>(), 3);
    session_default.close().await.unwrap();
}

#[tokio::test]
async fn cancellation_mid_stream_drains_and_returns() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(&serving(&db), Arc::new(model().unwrap()), fixture.generation, options()).await.unwrap();
    let before = provider_backends(&db).await;
    assert_eq!(before.len(), 2);
    for _ in 0..3 {
        let mut stream = SessionContext::new().read_table(session.table::<ArtifactChunk>().unwrap()).unwrap().execute_stream().await.unwrap();
        let first = stream.next().await.unwrap().unwrap();
        assert!(first.num_rows() > 0);
        drop(stream);
    }
    for _ in 0..50 {
        if matches!(session.health(), PoolHealth::Ready { idle: 2, .. }) { break; }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(matches!(session.health(), PoolHealth::Ready { connections: 2, idle: 2 }), "{:?}", session.health());
    assert_eq!(provider_backends(&db).await, before, "drained connections return; none is replaced");
    assert_eq!(decode::<ArtifactChunk>(&collect(session.table::<ArtifactChunk>().unwrap()).await.unwrap()).len(), fixture.chunks.len());
    session.close().await.unwrap();
}

#[tokio::test]
async fn transport_loss_is_terminal() {
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let session = GenerationSession::open(&serving(&db), Arc::new(model().unwrap()), fixture.generation, options()).await.unwrap();
    assert!(matches!(session.health(), PoolHealth::Ready { .. }));
    sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name = 'lctx-provider'").execute(&db.superuser).await.unwrap();
    assert!(collect(session.table::<Package>().unwrap()).await.is_err(), "a lost session reads nothing");
    assert_eq!(session.health(), PoolHealth::Lost);
    assert!(collect(session.table::<Package>().unwrap()).await.is_err(), "and never recovers");
    assert!(provider_backends(&db).await.is_empty(), "no connection was opened to replace the lost ones");
    assert!(matches!(session.close().await, Err(ReadError::Pool(_))));
    assert_eq!(fixture.store.retire(fixture.generation).await.unwrap(), CleanupOutcome::Removed, "the lost leases are gone with their backends");
}

#[tokio::test]
async fn stage_session_registers_generation_table() {
    use cpg_core::model_runtime::{AttemptRuntime, RuntimeOptions};
    let db = DisposableDatabase::start().await;
    let fixture = rich(&db).await;
    let model = Arc::new(model().unwrap());
    let session = GenerationSession::open(&serving(&db), model.clone(), fixture.generation, options()).await.unwrap();
    let schedule = Schedule::build(&model, vec![
        Stage { name: "source", inputs: vec![], outputs: vec![RelationUse::of::<Package>()], contributes: vec![], coverage: vec![], provider: None,
            profiles: vec![Profile::Catalog], effect: Effect::Pure, code: ContentHash::of(b"source"), configuration: ContentHash::of(b"cfg") },
        Stage { name: "consumer", inputs: vec![RelationUse::of::<Package>()], outputs: vec![RelationUse::of::<Release>()], contributes: vec![], coverage: vec![],
            provider: None, profiles: vec![Profile::Catalog], effect: Effect::Pure, code: ContentHash::of(b"consumer"), configuration: ContentHash::of(b"cfg") },
    ], &[], Profile::Catalog).unwrap();
    let runtime = AttemptRuntime::new(RuntimeOptions::default()).unwrap();
    let mut execution = schedule.execute();
    let mut source = execution.begin("source").unwrap();
    source.write::<Package, _>(async |_| Ok(())).await.unwrap();
    source.retain(Arc::new(Batch::<Package>::new(&model, vec![], &budget()).unwrap())).unwrap();
    source.finish(ProviderOutcome::Complete).unwrap();
    let consumer = execution.begin("consumer").unwrap();
    let stage = runtime.session(&consumer);
    stage.register(&consumer.read::<Package>().unwrap(), session.table::<Package>().unwrap()).unwrap();
    let rows = stage.sql("SELECT name FROM packages WHERE name <> 'alpha'").await.unwrap().collect().await.unwrap();
    assert_eq!(rows.iter().map(RecordBatch::num_rows).sum::<usize>(), 2);
    assert!(stage.register(&consumer.read::<Package>().unwrap(), session.table::<Package>().unwrap()).is_err(), "one registration per relation");
    drop(consumer);
    session.close().await.unwrap();
}
