//! The service baseline and the verified service owner against a disposable real PostgreSQL 18
//! (cutover plan P1.5, T11/T12). Each control states its answer before running it.
use lctx_model::domain::ContentHash;
use lctx_postgres::operations::AttemptId;
use lctx_model::domain::{
    ContentHash,
    embedding::{Spec, value::encode_vector},
    model,
};
use lctx_postgres::{
    CacheValue, Config, Error, OwnerPool,
    generations::GenerationStore,
    roles::{Role, RoleConfig},
    testing::DisposableDatabase,
};
use std::sync::Arc;

const SPEC: &str = include_str!("../../../specs/embedding/qwen3-embedding-8b.json");

/// Every non-system relation as `schema.name:relkind` with its owner, sorted.
async fn inventory(pool: &sqlx::PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT n.nspname || '.' || c.relname || ':' || c.relkind::text || '@' || pg_get_userbyid(c.relowner) \
        FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
        WHERE n.nspname NOT IN ('pg_catalog', 'information_schema') AND n.nspname NOT LIKE 'pg\\_%' ORDER BY 1")
        .fetch_all(pool).await.unwrap()
}
async fn schemas(pool: &sqlx::PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT nspname::text FROM pg_namespace WHERE nspname <> 'information_schema' AND nspname NOT LIKE 'pg\\_%' ORDER BY 1")
        .fetch_all(pool).await.unwrap()
}
async fn history(pool: &sqlx::PgPool) -> Vec<(i64, Vec<u8>)> {
    sqlx::query_as("SELECT version, checksum FROM public._sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .unwrap()
}
fn code(result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) -> String {
    result
        .expect_err("refused")
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned())
        .unwrap_or_default()
}

struct Services {
    db: DisposableDatabase,
    _dir: tempfile::TempDir,
    config: Config,
}
async fn services() -> Services {
    let db = DisposableDatabase::start().await;
    let dir = tempfile::tempdir().unwrap();
    db.write_configs(dir.path()).unwrap();
    let config = Config::load(&dir.path().join("postgres.json")).unwrap();
    Services {
        db,
        _dir: dir,
        config,
    }
}

#[tokio::test]
async fn fresh_baseline_installs_only_services() {
    let s = services().await;
    let migrator = s.config.connect_migrator().await.unwrap();
    assert!(
        matches!(migrator.check().await, Err(Error::Schema)),
        "nothing is installed before migration"
    );
    migrator.migrate().await.unwrap();
    migrator.check().await.unwrap();
    assert_eq!(
        schemas(&s.db.superuser).await,
        ["lctx_cache", "lctx_ops", "public"]
    );
    assert_eq!(
        inventory(&s.db.superuser).await,
        [
            "lctx_cache.embedding_values:r@lctx_migrator",
            "lctx_cache.embedding_values_pkey:i@lctx_migrator",
            "lctx_cache.specs:r@lctx_migrator",
            "lctx_cache.specs_pkey:i@lctx_migrator",
            "lctx_ops.attempts:r@lctx_migrator",
            "lctx_ops.attempts_pkey:i@lctx_migrator",
            "lctx_ops.events:r@lctx_migrator",
            "lctx_ops.events_order:i@lctx_migrator",
            "lctx_ops.events_pkey:i@lctx_migrator",
            "public._sqlx_migrations:r@lctx_migrator",
            "public._sqlx_migrations_pkey:i@lctx_migrator",
        ]
    );
    let extensions: Vec<String> =
        sqlx::query_scalar("SELECT extname::text FROM pg_extension ORDER BY 1")
            .fetch_all(&s.db.superuser)
            .await
            .unwrap();
    assert_eq!(
        extensions,
        ["plpgsql"],
        "the baseline requires no extension"
    );
    // Twin: the generation store is installed from the typed model, never by a migration.
    GenerationStore::install(migrator.owner().await.unwrap(), Arc::new(model().unwrap()))
        .await
        .unwrap();
    assert_eq!(
        schemas(&s.db.superuser).await,
        ["lctx_cache", "lctx_model_store", "lctx_ops", "public"]
    );
    migrator.check().await.unwrap();
}

#[tokio::test]
async fn repeated_migrate_is_noop() {
    let s = services().await;
    let migrator = s.config.connect_migrator().await.unwrap();
    let (first, second) = tokio::join!(migrator.migrate(), migrator.migrate());
    first.unwrap();
    second.unwrap();
    let before = (
        inventory(&s.db.superuser).await,
        history(&s.db.superuser).await,
    );
    assert_eq!(before.1.len(), 1, "one baseline migration");
    migrator.migrate().await.unwrap();
    assert_eq!(
        (
            inventory(&s.db.superuser).await,
            history(&s.db.superuser).await
        ),
        before
    );
    // Twin: a changed checksum is detected by check and refused by migrate.
    sqlx::query("UPDATE public._sqlx_migrations SET checksum = '\\x00'")
        .execute(s.db.owner.pool())
        .await
        .unwrap();
    assert!(matches!(migrator.check().await, Err(Error::Schema)));
    assert!(matches!(migrator.migrate().await, Err(Error::Migration)));
}

#[tokio::test]
async fn legacy_history_refused_without_change() {
    let s = services().await;
    sqlx::raw_sql("CREATE TABLE public._sqlx_migrations (version BIGINT PRIMARY KEY, description TEXT NOT NULL, \
        installed_on TIMESTAMPTZ NOT NULL DEFAULT now(), success BOOLEAN NOT NULL, checksum BYTEA NOT NULL, execution_time BIGINT NOT NULL); \
        INSERT INTO public._sqlx_migrations(version, description, success, checksum, execution_time) VALUES (202609270001, 'services', true, '\\x00', 1); \
        CREATE SCHEMA lctx_serving")
        .execute(s.db.owner.pool()).await.unwrap();
    let before = (
        schemas(&s.db.superuser).await,
        inventory(&s.db.superuser).await,
        history(&s.db.superuser).await,
    );
    let migrator = s.config.connect_migrator().await.unwrap();
    assert!(matches!(
        migrator.migrate().await,
        Err(Error::LegacyHistory)
    ));
    assert_eq!(
        (
            schemas(&s.db.superuser).await,
            inventory(&s.db.superuser).await,
            history(&s.db.superuser).await
        ),
        before
    );
    assert!(!before.0.contains(&"lctx_cache".to_owned()));
    // Twin: an empty history is not legacy.
    sqlx::query("DELETE FROM public._sqlx_migrations")
        .execute(s.db.owner.pool())
        .await
        .unwrap();
    migrator.migrate().await.unwrap();
    migrator.check().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cache_roundtrip_and_spec_conflict() {
    let s = services().await;
    let migrator = s.config.connect_migrator().await.unwrap();
    migrator.migrate().await.unwrap();
    let app = s.config.connect_application().await.unwrap();
    app.check().await.unwrap();
    assert_eq!(app.health().await.unwrap().role, "lctx_app");
    let spec = Spec::parse(SPEC).unwrap();
    app.ensure_spec(&spec).await.unwrap();
    app.ensure_spec(&spec).await.unwrap();
    let unit = |axis: usize| {
        let mut v = vec![0.0f32; spec.dimensions as usize];
        v[axis] = 1.0;
        v
    };
    let a = CacheValue {
        input_hash: ContentHash([1; 32]),
        vector: unit(0),
        admitted_tokens: 3,
    };
    let b = CacheValue {
        vector: unit(1),
        ..a.clone()
    };
    let (left, right) = tokio::join!(
        app.admit(&spec, std::slice::from_ref(&a)),
        app.admit(&spec, std::slice::from_ref(&b))
    );
    let (left, right) = (left.unwrap(), right.unwrap());
    let winner = encode_vector(&left[&a.input_hash].vector);
    assert_eq!(
        winner,
        encode_vector(&right[&a.input_hash].vector),
        "one committed value per key"
    );
    assert!(winner == encode_vector(&a.vector) || winner == encode_vector(&b.vector));
    let found = app
        .cached(&spec, &[a.input_hash, ContentHash([9; 32])])
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(
        (
            encode_vector(&found[&a.input_hash].vector),
            found[&a.input_hash].admitted_tokens
        ),
        (winner, 3)
    );
    // Refusals before any write.
    let nan = CacheValue {
        input_hash: ContentHash([2; 32]),
        vector: {
            let mut v = unit(0);
            v[1] = f32::NAN;
            v
        },
        admitted_tokens: 1,
    };
    assert!(matches!(
        app.admit(&spec, &[nan]).await,
        Err(Error::Integrity(_))
    ));
    let over = CacheValue {
        input_hash: ContentHash([3; 32]),
        vector: unit(0),
        admitted_tokens: spec.max_document_tokens + 1,
    };
    assert!(matches!(
        app.admit(&spec, &[over]).await,
        Err(Error::Integrity(_))
    ));
    assert_eq!(
        app.cached(&spec, &[ContentHash([2; 32]), ContentHash([3; 32])])
            .await
            .unwrap()
            .len(),
        0
    );
    // A stored value whose digest no longer matches is refused on read.
    sqlx::query("UPDATE lctx_cache.embedding_values SET value_digest = $1")
        .bind(vec![0u8; 32])
        .execute(&s.db.superuser)
        .await
        .unwrap();
    assert!(matches!(
        app.cached(&spec, &[a.input_hash]).await,
        Err(Error::Integrity("cached value digest"))
    ));
    // A different canonical text under the same spec hash is a conflict, never silently accepted.
    sqlx::query("UPDATE lctx_cache.specs SET canonical_spec = canonical_spec || ' '")
        .execute(&s.db.superuser)
        .await
        .unwrap();
    assert!(matches!(
        app.ensure_spec(&spec).await,
        Err(Error::Integrity("canonical spec conflict"))
    ));
    app.close().await;
}

#[tokio::test]
async fn attempts_events_runs_mark_interrupted() {
    let s = services().await;
    s.config
        .connect_migrator()
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    let app = s.config.connect_application().await.unwrap();
    let (first, second) = (AttemptId::from_bytes([1; 16]), AttemptId::from_bytes([2; 16]));
    app.start_attempt(first, ContentHash([7; 32]), "fastmcp", "build/store")
        .await
        .unwrap();
    app.start_attempt(first, ContentHash([7; 32]), "fastmcp", "build/store")
        .await
        .unwrap();
    assert!(matches!(
        app.start_attempt(first, ContentHash([7; 32]), "other", "build/store")
            .await,
        Err(Error::Integrity(_))
    ));
    let runs = app.runs(Some(first), 10, 0).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (
            runs[0].library.as_deref(),
            runs[0].registration.as_str(),
            runs[0].outcome.as_str()
        ),
        (Some("fastmcp"), "started", "unfinished")
    );
    assert!(
        runs[0].started_at.is_some(),
        "a started attempt records its start"
    );
    app.event(
        first,
        "operator/interrupted",
        "interrupted",
        "operator reconciliation",
    )
    .await
    .unwrap();
    app.event(
        first,
        "operator/interrupted",
        "interrupted",
        "operator reconciliation",
    )
    .await
    .unwrap();
    assert!(matches!(
        app.event(first, "operator/interrupted", "interrupted", "changed")
            .await,
        Err(Error::Integrity(_))
    ));
    assert!(matches!(
        app.event(first, "bogus", "bogus", "").await,
        Err(Error::Database { .. })
    ));
    assert_eq!(
        app.runs(Some(first), 10, 0).await.unwrap()[0].outcome,
        "interrupted"
    );
    let keys: Vec<_> = app
        .events(first, 10, 0)
        .await
        .unwrap()
        .into_iter()
        .map(|e| (e.event_key, e.kind))
        .collect();
    assert_eq!(
        keys,
        [
            ("started".to_owned(), "started".to_owned()),
            ("operator/interrupted".to_owned(), "interrupted".to_owned())
        ]
    );
    // Twin: a published attempt reports published, and an attempt without events is not invented.
    app.start_attempt(second, ContentHash([7; 32]), "fastmcp", "build/store")
        .await
        .unwrap();
    app.event(second, "publish", "published", "").await.unwrap();
    assert_eq!(
        app.runs(Some(second), 10, 0).await.unwrap()[0].outcome,
        "published"
    );
    assert!(app.runs(Some(AttemptId::from_bytes([3; 16])), 10, 0).await.unwrap().is_empty());
    assert!(
        matches!(
            app.event(AttemptId::from_bytes([3; 16]), "started", "started", "").await,
            Err(Error::Database { .. })
        ),
        "an event needs its attempt"
    );
}

#[tokio::test]
async fn service_grant_matrix() {
    let s = services().await;
    s.config
        .connect_migrator()
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    let tables = [
        "lctx_cache.specs",
        "lctx_cache.embedding_values",
        "lctx_ops.attempts",
        "lctx_ops.events",
    ];
    for table in tables.iter().chain(&["public._sqlx_migrations"]) {
        sqlx::query(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .execute(&s.db.app)
            .await
            .unwrap();
    }
    for table in tables {
        for statement in [
            format!(
                "UPDATE {table} SET {} = {0}",
                if table.ends_with("specs") || table.ends_with("values") {
                    "spec_hash"
                } else {
                    "attempt_id"
                }
            ),
            format!("DELETE FROM {table}"),
            format!("TRUNCATE {table}"),
        ] {
            assert_eq!(
                code(
                    sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
                        .execute(&s.db.app)
                        .await
                ),
                "42501",
                "{statement}"
            );
        }
        for runtime in [&s.db.writer, &s.db.reader] {
            assert_eq!(
                code(
                    sqlx::query(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                        .execute(runtime)
                        .await
                ),
                "42501",
                "{table}"
            );
        }
    }
    for schema in ["lctx_cache", "lctx_ops", "public"] {
        assert_eq!(
            code(
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE TABLE {schema}.forbidden(id int)"
                )))
                .execute(&s.db.app)
                .await
            ),
            "42501",
            "{schema}"
        );
        assert_eq!(
            code(
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE TABLE {schema}.forbidden(id int)"
                )))
                .execute(&s.db.writer)
                .await
            ),
            "42501",
            "{schema}"
        );
    }
    assert_eq!(
        code(
            sqlx::query("CREATE TEMP TABLE scratch(id int)")
                .execute(&s.db.app)
                .await
        ),
        "42501",
        "only the importer may use TEMP"
    );
    sqlx::query("CREATE TEMP TABLE scratch(id int)")
        .execute(&s.db.writer)
        .await
        .unwrap();
    assert_eq!(
        code(
            sqlx::query("CREATE TEMP TABLE scratch(id int)")
                .execute(&s.db.reader)
                .await
        ),
        "25006",
        "the reader is read-only"
    );
    // Twin: the owner holds every service table.
    for table in tables {
        sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
            .execute(s.db.owner.pool())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn superuser_and_elevated_owners_refused() {
    let s = services().await;
    assert!(matches!(
        OwnerPool::verify(s.db.superuser.clone()).await,
        Err(Error::Owner("a superuser may not own the store"))
    ));
    let superuser = Config {
        migration_url: s.db.url("postgres"),
        ..s.config.clone()
    };
    let migrator = superuser.connect_migrator().await.unwrap();
    assert!(matches!(migrator.migrate().await, Err(Error::Owner(_))));
    assert_eq!(
        schemas(&s.db.superuser).await,
        ["public"],
        "a refused install changes nothing"
    );
    assert!(inventory(&s.db.superuser).await.is_empty());
    sqlx::raw_sql("CREATE ROLE lctx_creator LOGIN CREATEROLE PASSWORD 'postgres'; CREATE ROLE lctx_plain LOGIN PASSWORD 'postgres'; \
        GRANT CONNECT, CREATE ON DATABASE lctx TO lctx_creator; GRANT CONNECT ON DATABASE lctx TO lctx_plain")
        .execute(&s.db.superuser).await.unwrap();
    let creator = sqlx::PgPool::connect(&s.db.url("lctx_creator"))
        .await
        .unwrap();
    assert!(matches!(
        OwnerPool::verify(creator).await,
        Err(Error::Owner(
            "the owner may not hold CREATEROLE or BYPASSRLS"
        ))
    ));
    let plain = sqlx::PgPool::connect(&s.db.url("lctx_plain"))
        .await
        .unwrap();
    assert!(matches!(
        OwnerPool::verify(plain).await,
        Err(Error::Owner("the owner lacks CREATE on its database"))
    ));
    for (grant, revoke) in [
        (
            "GRANT lctx_app TO lctx_migrator",
            "REVOKE lctx_app FROM lctx_migrator",
        ),
        (
            "GRANT lctx_migrator TO lctx_serving",
            "REVOKE lctx_migrator FROM lctx_serving",
        ),
    ] {
        sqlx::query(grant).execute(&s.db.superuser).await.unwrap();
        assert!(
            matches!(
                OwnerPool::verify(s.db.owner.pool().clone()).await,
                Err(Error::Owner(
                    "the owner shares a membership edge with a runtime role"
                ))
            ),
            "{grant}"
        );
        sqlx::query(revoke).execute(&s.db.superuser).await.unwrap();
    }
    // Twin: the provisioned owner verifies and installs.
    OwnerPool::verify(s.db.owner.pool().clone()).await.unwrap();
    s.config
        .connect_migrator()
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
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
