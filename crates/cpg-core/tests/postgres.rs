//! Real PG18 qualification. Explicitly run by `just test-postgres`; never a mocked DB pass.
use cpg_core::{
    embed::{Embedder, FakeEmbedder, Session, Usage},
    postgres::{CacheValue, Config, Store, TEST_IMAGE_TAG},
};
use cpg_schema::{
    embedding::encode_vector,
    id::{Digest, Id},
};
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};

struct Fixture {
    _db: ContainerAsync<Postgres>,
    admin: sqlx::PgPool,
    app: Store,
    config: Config,
}

impl Fixture {
    async fn start() -> Self {
        cpg_extract::logging::init_logging();
        let db = Postgres::default()
            .with_fsync_enabled()
            .with_tag(TEST_IMAGE_TAG.trim())
            .start()
            .await
            .expect("blocked: Docker and the pinned PostgreSQL image are required");
        let port = db.get_host_port_ipv4(5432).await.unwrap();
        let admin = sqlx::PgPool::connect(&format!(
            "postgres://postgres:postgres@127.0.0.1:{port}/postgres"
        ))
        .await
        .unwrap();
        sqlx::raw_sql("CREATE ROLE lctx_app LOGIN PASSWORD 'fixture-only'; CREATE ROLE lctx_migrator LOGIN PASSWORD 'fixture-only'; GRANT CREATE ON DATABASE postgres TO lctx_migrator; GRANT CREATE ON SCHEMA public TO lctx_migrator;").execute(&admin).await.unwrap();
        let config = Config {
            application_url: format!("postgres://lctx_app:fixture-only@127.0.0.1:{port}/postgres"),
            migration_url: format!(
                "postgres://lctx_migrator:fixture-only@127.0.0.1:{port}/postgres"
            ),
            max_connections: 2,
            acquire_timeout_seconds: 1,
            statement_timeout_seconds: 2,
            lock_timeout_seconds: 1,
            max_receipt_bytes: 268435456,
        };
        let migrator = config.connect(true).await.unwrap();
        assert!(migrator.check().await.is_err());
        let (first, second) = tokio::join!(migrator.migrate(), migrator.migrate());
        first.unwrap();
        second.unwrap();
        migrator.check().await.unwrap();
        migrator.close().await;
        let app = config.connect(false).await.unwrap();
        app.check().await.unwrap();
        assert_eq!(app.health().await.unwrap().server_version, "180006");
        Self {
            _db: db,
            admin,
            app,
            config,
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit PostgreSQL fixture setup; included by just test-postgres"]
async fn pg_cache_race_bytes_privileges_and_corruption() {
    let f = Fixture::start().await;
    let mut spec = FakeEmbedder::new().spec().clone();
    spec.dimensions = 2;
    f.app.ensure_spec(&spec).await.unwrap();
    let a = CacheValue {
        input_hash: Digest([1; 32]),
        vector: vec![1.0, -0.0],
        admitted_tokens: 1,
    };
    let b = CacheValue {
        vector: vec![0.0, 1.0],
        ..a.clone()
    };
    let (left, right) = tokio::join!(
        f.app.admit(&spec, std::slice::from_ref(&a)),
        f.app.admit(&spec, std::slice::from_ref(&b))
    );
    let left = left.unwrap();
    let right = right.unwrap();
    assert_eq!(
        encode_vector(&left[&a.input_hash].vector),
        encode_vector(&right[&a.input_hash].vector)
    );
    let app = sqlx::PgPool::connect(&f.config.application_url)
        .await
        .unwrap();
    for statement in [
        "DELETE FROM lctx_cache.embedding_values",
        "UPDATE lctx_cache.embedding_values SET admitted_tokens=0",
        "CREATE TABLE lctx_cache.forbidden(id int)",
        "CREATE TABLE public.forbidden(id int)",
    ] {
        assert!(
            sqlx::query(statement).execute(&app).await.is_err(),
            "{statement}"
        );
    }
    let bad = CacheValue {
        input_hash: Digest([2; 32]),
        vector: vec![f32::NAN, 0.0],
        admitted_tokens: 1,
    };
    assert!(f.app.admit(&spec, &[bad]).await.is_err());
    let over = CacheValue {
        admitted_tokens: spec.max_document_tokens + 1,
        ..a.clone()
    };
    assert!(f.app.admit(&spec, &[over]).await.is_err());
    sqlx::query("UPDATE lctx_cache.embedding_values SET value_digest=$1")
        .bind(vec![0u8; 32])
        .execute(&f.admin)
        .await
        .unwrap();
    assert!(f.app.cached(&spec, &[a.input_hash]).await.is_err());
    app.close().await;
    f.app.close().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit PostgreSQL fixture setup; included by just test-postgres"]
async fn pg_retained_values_survive_cache_restore_and_disconnect() {
    let f = Fixture::start().await;
    let fake = FakeEmbedder::new();
    let mut session = Session::new(Id([1; 16]), Some(f.app.clone()), 268435456);
    let texts = vec!["operation and E0 overlap".to_owned()];
    let first = session
        .texts(&fake, &texts, Usage::Operation)
        .await
        .unwrap();
    sqlx::query("DELETE FROM lctx_cache.embedding_values")
        .execute(&f.admin)
        .await
        .unwrap();
    let mut replacement = first[0].clone();
    for v in &mut replacement {
        *v = -*v;
    }
    f.app
        .admit(
            fake.spec(),
            &[CacheValue {
                input_hash: cpg_core::embed::input_hash(&texts[0]),
                vector: replacement,
                admitted_tokens: 1,
            }],
        )
        .await
        .unwrap();
    let second = session
        .texts(&fake, &texts, Usage::Analytics)
        .await
        .unwrap();
    assert_eq!(encode_vector(&first[0]), encode_vector(&second[0]));
    f.app.close().await;
    // Finalization has no database dependency; a repeated retained key can also be consumed.
    let third = session.texts(&fake, &texts, Usage::Brief).await.unwrap();
    assert_eq!(encode_vector(&first[0]), encode_vector(&third[0]));
    let receipt = session.finish().unwrap();
    assert_eq!(receipt.uses[0].usage_mask, 7);
    assert_eq!(
        encode_vector(&receipt.values[0].vector),
        encode_vector(&first[0])
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit PostgreSQL fixture setup; included by just test-postgres"]
async fn pg_schema_timeout_cancel_reuse_and_event_idempotency() {
    let f = Fixture::start().await;
    let spec = FakeEmbedder::new().spec().clone();
    f.app.ensure_spec(&spec).await.unwrap();
    let mut lock = f.admin.begin().await.unwrap();
    sqlx::query("LOCK TABLE lctx_cache.embedding_values IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            f.app.cached(&spec, &[Digest([2; 32])])
        )
        .await
        .is_err()
    );
    lock.rollback().await.unwrap();
    assert!(
        f.app
            .cached(&spec, &[Digest([2; 32])])
            .await
            .unwrap()
            .is_empty()
    );
    let id = Id([8; 16]);
    let compiler = Digest([9; 32]);
    f.app
        .start_attempt(id, compiler, "fixture", "/fixture")
        .await
        .unwrap();
    f.app
        .start_attempt(id, compiler, "fixture", "/fixture")
        .await
        .unwrap();
    assert_eq!(
        f.app.runs(Some(id), 1, 0).await.unwrap()[0].outcome,
        "unfinished"
    );
    f.app
        .event(id, "published", "published", "digest")
        .await
        .unwrap();
    f.app
        .event(id, "published", "published", "digest")
        .await
        .unwrap();
    assert!(
        f.app
            .event(id, "published", "failed", "other")
            .await
            .is_err()
    );
    assert_eq!(f.app.events(id, 100, 0).await.unwrap().len(), 2);
    assert_eq!(
        f.app.runs(Some(id), 1, 0).await.unwrap()[0].outcome,
        "published"
    );
    sqlx::query("UPDATE public._sqlx_migrations SET checksum=$1")
        .bind(vec![0u8; 48])
        .execute(&f.admin)
        .await
        .unwrap();
    assert!(f.app.check().await.is_err());
    f.app.close().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit PostgreSQL fixture setup; included by just test-postgres"]
async fn pg_reconciliation_repairs_missing_observations_and_stale_discovery() {
    let f = Fixture::start().await;
    let id = Id([21; 16]);
    let compiler = Digest([22; 32]);
    let content = Digest([23; 32]);
    f.app
        .start_attempt(id, compiler, "fixture", "/store")
        .await
        .unwrap();
    f.app
        .event(id, "failed", "failed", "lost publication acknowledgement")
        .await
        .unwrap();
    f.app
        .observe_publication("/store", id, content, compiler)
        .await
        .unwrap();
    f.app
        .observe_publication("/store", id, content, compiler)
        .await
        .unwrap();
    assert_eq!(
        f.app.runs(Some(id), 1, 0).await.unwrap()[0].outcome,
        "published"
    );
    let recovered = Id([24; 16]);
    f.app
        .observe_publication("/store", recovered, content, compiler)
        .await
        .unwrap();
    let rows = f.app.runs(Some(recovered), 1, 0).await.unwrap();
    assert_eq!(rows[0].registration, "reconciled");
    assert!(rows[0].started_at.is_none() && rows[0].library.is_none());
    f.app
        .record_generation("/generations/old", "/store", "g", id, content)
        .await
        .unwrap();
    let before = f.app.reconciliation_started().await.unwrap();
    f.app
        .record_generation("/generations/new", "/store", "g", id, content)
        .await
        .unwrap();
    f.app
        .finish_reconciliation("/store", "/generations", &before, &[id], &[])
        .await
        .unwrap();
    let generations = f.app.generations(None, 100, 0).await.unwrap();
    assert_eq!(generations.len(), 1);
    assert_eq!(
        generations[0].location, "/generations/new",
        "concurrent newer observation survives"
    );
    assert_eq!(f.app.snapshots(None, 100, 0).await.unwrap().len(), 1);
    assert_eq!(
        f.app.runs(None, 100, 0).await.unwrap().len(),
        2,
        "history retained"
    );
    let before = f.app.reconciliation_started().await.unwrap();
    f.app
        .finish_reconciliation("/store", "/generations", &before, &[], &[])
        .await
        .unwrap();
    assert!(f.app.generations(None, 100, 0).await.unwrap().is_empty());
    assert!(f.app.snapshots(None, 100, 0).await.unwrap().is_empty());
    assert_eq!(f.app.events(id, 100, 0).await.unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit PostgreSQL fixture setup; included by just test-postgres"]
async fn pg_legacy_import_readmission_resume_and_conflict() {
    use cpg_schema::{
        embedding::{EmbeddingCache, EmbeddingCacheRow},
        table::Table,
    };
    let f = Fixture::start().await;
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeEmbedder::new();
    let texts = [
        "legacy admitted request".to_owned(),
        "inactive without original text".to_owned(),
    ];
    let rows: Vec<_> = texts
        .iter()
        .map(|text| EmbeddingCacheRow {
            spec_hash: fake.spec().hash(),
            input_hash: cpg_core::embed::input_hash(text),
            model: fake.spec().model.clone(),
            vector: fake.vector(text),
        })
        .collect();
    let batch = EmbeddingCache::to_batch(&rows).unwrap();
    let table = cpg_core::delta::create::<EmbeddingCache>(dir.path())
        .await
        .unwrap();
    cpg_core::delta::append(table, batch, Id([41; 16]))
        .await
        .unwrap();
    let table = cpg_core::delta::open_verified::<EmbeddingCache>(dir.path())
        .await
        .unwrap();
    let version = table.version().unwrap();
    let first = cpg_core::postgres::legacy::import(&f.app, dir.path(), version, &fake, &texts[..1])
        .await
        .unwrap();
    let retry = cpg_core::postgres::legacy::import(&f.app, dir.path(), version, &fake, &texts[..1])
        .await
        .unwrap();
    assert_eq!(first.imported, 1);
    assert_eq!(first.inactive_without_request, 1);
    assert_eq!(first.import_digest, retry.import_digest);
    sqlx::query("DELETE FROM lctx_cache.embedding_values")
        .execute(&f.admin)
        .await
        .unwrap();
    f.app
        .admit(
            fake.spec(),
            &[CacheValue {
                input_hash: rows[0].input_hash,
                vector: rows[0].vector.iter().map(|v| -v).collect(),
                admitted_tokens: 1,
            }],
        )
        .await
        .unwrap();
    assert!(
        cpg_core::postgres::legacy::import(&f.app, dir.path(), version, &fake, &texts[..1])
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit PostgreSQL fixture setup; included by just test-postgres"]
async fn pg_published_bundle_replays_every_byte_after_database_stops() {
    use cpg_core::{
        analyze::{Analysis, Techniques},
        attempt, bundle,
    };
    let f = Fixture::start().await;
    let dir = tempfile::tempdir().unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/python/analysis_shapes")
        .canonicalize()
        .unwrap();
    let id = Id([31; 16]);
    let out = cpg_extract::extract(&cpg_extract::ExtractInput {
        release: cpg_extract::Release::from_tree(fixture.join("release"), "analysis_shapes")
            .unwrap(),
        venv_root: fixture.clone(),
        site_packages: vec![fixture.join("site")],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: id,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
    .unwrap();
    let config = lctx_analytics::config::AnalyticsConfig::parse(r#"
version = 1
[subsystem]
module_prefixes = ["pkg.server", "pkg.helpers", "pkg.handlers", "pkg.boot", "pkg.shadow", "pkg.controls"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.helper", "pkg.Server.route", "pkg.describe", "pkg.configure"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = 3
[briefs]
budget = 6
"#).unwrap();
    let analysis = Analysis {
        embedding_cache: Some(f.app.clone()),
        config,
        embedder: Some(std::sync::Arc::new(FakeEmbedder::new())),
        techniques: Techniques::parse("+knn").unwrap(),
    };
    let store = dir.path().join("store");
    attempt::compile_analyzed(&store, id, &out.tables, Some(&analysis))
        .await
        .unwrap();
    let first = bundle::bundle(&store, id, &dir.path().join("online"))
        .await
        .unwrap();
    f.app.close().await;
    f.admin.close().await;
    f._db.stop().await.unwrap();
    let second = bundle::bundle(&store, id, &dir.path().join("offline"))
        .await
        .unwrap();
    assert_eq!(first.key, second.key);
    for entry in std::fs::read_dir(&first.dir).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            assert_eq!(
                std::fs::read(entry.path()).unwrap(),
                std::fs::read(second.dir.join(entry.file_name())).unwrap(),
                "{:?}",
                entry.file_name()
            );
        }
    }
    f._db.start().await.unwrap();
    // Docker can allocate a different host port on restart; readiness must be re-established.
    let port = f._db.get_host_port_ipv4(5432).await.unwrap();
    let mut config = f.config.clone();
    let mut url = url::Url::parse(&config.application_url).unwrap();
    url.set_port(Some(port)).unwrap();
    config.application_url = url.to_string();
    config.acquire_timeout_seconds = 5;
    let restored = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if let Ok(store) = config.connect(false).await {
                break store;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("restarted PostgreSQL must become authenticated-ready within 30 seconds");
    restored.check().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_cache.embedding_values")
        .fetch_one(
            &sqlx::PgPool::connect(&config.application_url)
                .await
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(count > 0, "completed cache batches survive restart");
}

/// Preregistered PG0 cache-only workloads. Whole-compile/live-provider costs are separate.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "explicit measured workload; included by just test-postgres"]
async fn pg_cache_cost_workloads() {
    let f = Fixture::start().await;
    let mut config = f.config.clone();
    config.max_connections = 6;
    f.app.close().await;
    let fake = FakeEmbedder::new();
    assert_eq!(fake.spec().dimensions, 4096);
    for count in [2463, 25000] {
        for clients in [1, 4] {
            let mut stores = Vec::new();
            for _ in 0..clients {
                let store = config.connect(false).await.unwrap();
                store.ensure_spec(fake.spec()).await.unwrap();
                stores.push(store);
            }
            let values: Vec<_> = (0..count)
                .map(|i| {
                    let text = format!("PG0 workload {count}/{clients}/{i}");
                    CacheValue {
                        input_hash: cpg_core::embed::input_hash(&text),
                        vector: fake.vector(&text),
                        admitted_tokens: 8,
                    }
                })
                .collect();
            let keys: Vec<_> = values.iter().map(|v| v.input_hash).collect();
            let before: String = sqlx::query_scalar("SELECT pg_current_wal_lsn()::text")
                .fetch_one(&f.admin)
                .await
                .unwrap();
            let started = std::time::Instant::now();
            let results =
                futures::future::join_all(stores.iter().map(|s| s.admit(fake.spec(), &values)))
                    .await;
            let cold = started.elapsed().as_secs_f64();
            for result in results {
                assert_eq!(result.unwrap().len(), count);
            }
            let started = std::time::Instant::now();
            let results =
                futures::future::join_all(stores.iter().map(|s| s.cached(fake.spec(), &keys)))
                    .await;
            let warm = started.elapsed().as_secs_f64();
            for result in results {
                assert_eq!(result.unwrap().len(), count);
            }
            let (wal, database): (i64, i64) = sqlx::query_as("SELECT pg_wal_lsn_diff(pg_current_wal_lsn(),$1::text::pg_lsn)::bigint,pg_database_size(current_database())").bind(before).fetch_one(&f.admin).await.unwrap();
            println!(
                "PG_COST {}",
                serde_json::json!({"keys":count,"dimensions":fake.spec().dimensions,"clients":clients,"pool_per_client":6,"cold_seconds":cold,"warm_seconds":warm,"vector_bytes_per_client":count*4096*4,"wal_bytes":wal,"database_bytes":database,"smaller_workload_budget_met":count!=2463 || (cold<5.0 && warm<2.0)})
            );
            for store in stores {
                store.close().await;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires explicit PostgreSQL fixture setup; included by just test-postgres"]
async fn pg_connection_loss_reconciles_retry_and_pool_exhaustion_is_bounded() {
    let f = Fixture::start().await;
    let mut config = f.config.clone();
    config.max_connections = 1;
    config.statement_timeout_seconds = 10;
    config.lock_timeout_seconds = 10;
    let app = config.connect(false).await.unwrap();
    let fake = FakeEmbedder::new();
    app.ensure_spec(fake.spec()).await.unwrap();
    let value = CacheValue {
        input_hash: Digest([51; 32]),
        vector: fake.vector("disconnect"),
        admitted_tokens: 1,
    };
    let mut lock = f.admin.begin().await.unwrap();
    sqlx::query("LOCK TABLE lctx_cache.embedding_values IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let work = {
        let app = app.clone();
        let spec = fake.spec().clone();
        let value = value.clone();
        tokio::spawn(async move { app.admit(&spec, &[value]).await })
    };
    let pid = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE application_name='lctx' AND wait_event='relation' AND query LIKE 'INSERT INTO lctx_cache.embedding_values%' LIMIT 1").fetch_optional(&f.admin).await.unwrap();
            if let Some(pid) = pid { break pid; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    let timeout = app
        .cached(fake.spec(), &[value.input_hash])
        .await
        .unwrap_err();
    assert!(timeout.to_string().contains("pool acquisition timed out"));
    sqlx::query("SELECT pg_terminate_backend($1)")
        .bind(pid)
        .execute(&f.admin)
        .await
        .unwrap();
    lock.rollback().await.unwrap();
    let winners = work.await.unwrap().unwrap();
    assert_eq!(
        encode_vector(&winners[&value.input_hash].vector),
        encode_vector(&value.vector)
    );
    assert_eq!(
        app.cached(fake.spec(), &[value.input_hash])
            .await
            .unwrap()
            .len(),
        1
    );
}
