//! Generated install, `store check` and `store reset` against a disposable real PostgreSQL 18
//! (cutover plan P1.6). Every drift is a pre-written mutation with the finding it must produce.
use lctx_model::domain::resources::ResourceBudget;
use lctx_model::domain::{ModelError, stages::Profile};
use lctx_model::{
    Domain,
    domain::{Relation, ValidatedModel, model},
};
use lctx_postgres::{
    Config, OwnerPool,
    generations::{Error, FindingKind, GenerationId, GenerationStore},
    testing::{DisposableDatabase, Harness, step_in_flight},
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "reset_probes", semantic_source = include_bytes!("installation.rs"))]
struct ResetProbe {
    #[model(key)]
    name: String,
}

fn budget() -> ResourceBudget {
    ResourceBudget::fixed(1 << 28).unwrap()
}
fn full() -> Arc<ValidatedModel> {
    Arc::new(model().unwrap())
}
fn extended() -> Arc<ValidatedModel> {
    let mut relations = model().unwrap().relations().to_vec();
    relations.push(Relation::of::<ResetProbe>());
    Arc::new(ValidatedModel::validate(relations).unwrap())
}
/// A provisioned database with the service baseline applied, as production has before install.
async fn provisioned() -> (DisposableDatabase, tempfile::TempDir) {
    let db = DisposableDatabase::start().await;
    let dir = tempfile::tempdir().unwrap();
    db.write_configs(dir.path()).unwrap();
    Config::load(&dir.path().join("postgres.json"))
        .unwrap()
        .connect_migrator()
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    (db, dir)
}
async fn run(pool: &sqlx::PgPool, sql: &str) {
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.to_owned()))
        .execute(pool)
        .await
        .unwrap();
}
async fn harness(store: &GenerationStore, db: &DisposableDatabase) -> Harness {
    Harness::begin_empty_conformance(store, db.writer.clone(), Profile::Catalog, budget(), vec![])
        .await
        .unwrap()
}
async fn published(store: &GenerationStore, db: &DisposableDatabase) -> GenerationId {
    let mut g = harness(store, db).await;
    g.seal().await.unwrap();
    g.validate(&budget()).await.unwrap();
    g.publish().await.unwrap();
    g.generation()
}
async fn schemas(pool: &sqlx::PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT nspname::text FROM pg_namespace WHERE nspname NOT LIKE 'pg\\_%' AND nspname <> 'information_schema' ORDER BY 1")
        .fetch_all(pool).await.unwrap()
}

#[tokio::test]
async fn install_requires_service_owner() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let before = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert_eq!(before.findings.len(), 1, "{:#?}", before.findings);
    assert_eq!(
        (before.findings[0].kind, before.findings[0].subject.as_str()),
        (FindingKind::Installation, "lctx_model_store")
    );
    // A runtime role cannot become the owner, so it cannot install (the type admits only a verified owner).
    assert!(matches!(
        OwnerPool::verify(db.writer.clone()).await,
        Err(lctx_postgres::Error::Owner(_))
    ));
    GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let owner: String = sqlx::query_scalar("SELECT pg_get_userbyid(nspowner)::text FROM pg_namespace WHERE nspname = 'lctx_model_store'")
        .fetch_one(&db.superuser).await.unwrap();
    assert_eq!(owner, "lctx_migrator");
    let after = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(after.clean(), "{:#?}", after.findings);
    assert_eq!(after.generations, 0);
}

#[tokio::test]
async fn idempotent_install_and_digest_mismatch() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let first = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let catalog = schemas(&db.superuser).await;
    let second = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    assert_eq!(
        first.physical_digest(),
        second.physical_digest(),
        "the lowering is deterministic"
    );
    assert_eq!(schemas(&db.superuser).await, catalog);
    let installations: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.installation")
            .fetch_one(&db.superuser)
            .await
            .unwrap();
    assert_eq!(installations, 1);
    // Twin: another model is refused and the installation is unchanged.
    let other = extended();
    assert!(matches!(
        GenerationStore::install(db.owner.clone(), other.clone()).await,
        Err(Error::Contract)
    ));
    let digest: Vec<u8> =
        sqlx::query_scalar("SELECT physical_digest FROM lctx_model_store.installation")
            .fetch_one(&db.superuser)
            .await
            .unwrap();
    assert_eq!(digest, first.physical_digest().0);
    let report = GenerationStore::check(&db.owner, &other).await.unwrap();
    assert_eq!(
        report
            .findings
            .iter()
            .map(|f| (f.kind, f.subject.as_str()))
            .collect::<Vec<_>>(),
        [(FindingKind::Installation, "lctx_model_store")]
    );
    assert!(
        GenerationStore::check(&db.owner, &model)
            .await
            .unwrap()
            .clean()
    );
}

#[tokio::test]
async fn check_clean_in_every_state() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut staging = harness(&store, &db).await;
    let mut sealed = Harness::begin_empty_conformance(&store, db.writer.clone(), Profile::Behavioral, budget(), vec![])
        .await
        .unwrap();
    sealed.seal().await.unwrap();
    let mut validated = harness(&store, &db).await;
    validated.seal().await.unwrap();
    validated.validate(&budget()).await.unwrap();
    // A generation failed while staging keeps the sealed shape; one failed while sealed, too.
    let mut failed = harness(&store, &db).await;
    failed
        .fail(&ModelError::Invalid("control".into()))
        .await
        .unwrap();
    let published = published(&store, &db).await;
    let lease = store.pin(&db.reader, published, budget()).await.unwrap();
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(report.clean(), "{:#?}", report.findings);
    assert_eq!(report.generations, 5);
    // Prefix reconstruction must match owner-only sealed ACLs and still detect extra grants.
    let prefix: String = sqlx::query_scalar("SELECT c.relname::text FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relkind='v' AND c.relname ~ '^__v[0-9]+_' ORDER BY c.relname LIMIT 1")
        .bind(sealed.generation().schema()).fetch_one(db.owner.pool()).await.unwrap();
    let prefix_relation = format!("{}.{}", sealed.generation().schema(), prefix);
    run(db.owner.pool(), &format!("GRANT SELECT ON {prefix_relation} TO lctx_app")).await;
    let granted = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(granted.findings.iter().any(|finding| finding.kind == FindingKind::Differs
        && finding.subject == format!("{} relation {prefix}", sealed.generation().schema())));
    run(db.owner.pool(), &format!("REVOKE SELECT ON {prefix_relation} FROM lctx_app")).await;
    assert!(GenerationStore::check(&db.owner, &model).await.unwrap().clean());
    assert!(
        !schemas(&db.superuser)
            .await
            .iter()
            .any(|s| s.starts_with("lctx_check_")),
        "shadows never survive a check"
    );
    lease.release().await.unwrap();
    staging.abort().await.unwrap();
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(report.clean(), "{:#?}", report.findings);
    assert_eq!(report.generations, 4);
}

#[tokio::test]
async fn check_detects_each_drift() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let owner = db.owner.pool().clone();
    let expect = async |kind: FindingKind, subject: &str, case: &str| {
        let report = GenerationStore::check(&db.owner, &model).await.unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == kind && f.subject.contains(subject)),
            "{case}: expected {kind:?} {subject}, found {:#?}",
            report.findings
        );
    };
    // Drifts inside a generation schema: each on a disposable generation, removed afterwards.
    let staging_cases: [(&str, &str, FindingKind, &str); 8] = [
        (
            "column type",
            "ALTER TABLE {g}.packages ALTER COLUMN name TYPE varchar(64)",
            FindingKind::Differs,
            "column packages.name",
        ),
        (
            "NOT NULL",
            "ALTER TABLE {g}.packages ALTER COLUMN name DROP NOT NULL",
            FindingKind::Differs,
            "column packages.name",
        ),
        (
            "dropped CHECK",
            "ALTER TABLE {g}.packages DROP CONSTRAINT {check}",
            FindingKind::Missing,
            "constraint packages.",
        ),
        (
            "dropped index",
            "ALTER TABLE {g}.{indexed} DROP CONSTRAINT {index}",
            FindingKind::Missing,
            "index {index}",
        ),
        (
            "extra index",
            "CREATE INDEX drift_extra ON {g}.packages(name)",
            FindingKind::Unexpected,
            "index drift_extra",
        ),
        (
            "extra grant",
            "GRANT SELECT ON {g}.packages TO lctx_app",
            FindingKind::Differs,
            "relation packages",
        ),
        (
            "owner change",
            "ALTER TABLE {g}.packages OWNER TO postgres",
            FindingKind::Differs,
            "relation packages",
        ),
        (
            "extra table",
            "CREATE TABLE {g}.drift_extra(id int)",
            FindingKind::Unexpected,
            "relation drift_extra",
        ),
    ];
    for (case, sql, kind, subject) in staging_cases {
        let mut g = harness(&store, &db).await;
        let schema = g.generation().schema();
        let check: String = sqlx::query_scalar("SELECT conname::text FROM pg_constraint WHERE conrelid = ($1 || '.packages')::regclass AND contype = 'c' ORDER BY 1 LIMIT 1")
            .bind(&schema).fetch_one(&owner).await.unwrap();
        // A sum relation's tag index backs its unique constraint; dropping the constraint drops it.
        let (index, indexed): (String, String) = sqlx::query_as("SELECT c.relname::text, t.relname::text FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid \
            JOIN pg_class t ON t.oid = i.indrelid WHERE c.relnamespace = $1::regnamespace AND NOT i.indisprimary ORDER BY 1 LIMIT 1").bind(&schema).fetch_one(&owner).await.unwrap();
        let fill = |text: &str| {
            text.replace("{g}", &schema)
                .replace("{check}", &check)
                .replace("{index}", &index)
                .replace("{indexed}", &indexed)
        };
        let (sql, subject) = (fill(sql), fill(subject));
        run(
            if case == "owner change" {
                &db.superuser
            } else {
                &owner
            },
            &sql,
        )
        .await;
        expect(kind, &format!("{schema} {subject}"), case).await;
        g.abort().await.unwrap();
    }
    for (case, sql, subject) in [
        (
            "dropped FK",
            "ALTER TABLE {g}.{table} DROP CONSTRAINT {fk}",
            "constraint {table}.{fk}",
        ),
        (
            "revoked grant",
            "REVOKE SELECT ON {g}.packages FROM lctx_serving",
            "relation packages",
        ),
    ] {
        let g = published(&store, &db).await;
        let schema = g.schema();
        let (table, fk): (String, String) = sqlx::query_as("SELECT c.relname::text, k.conname::text FROM pg_constraint k JOIN pg_class c ON c.oid = k.conrelid \
            WHERE c.relnamespace = $1::regnamespace AND k.contype = 'f' ORDER BY 1, 2 LIMIT 1").bind(&schema).fetch_one(&owner).await.unwrap();
        let fill = |text: &str| {
            text.replace("{g}", &schema)
                .replace("{table}", &table)
                .replace("{fk}", &fk)
        };
        run(&owner, &fill(sql)).await;
        let kind = if case == "dropped FK" {
            FindingKind::Missing
        } else {
            FindingKind::Differs
        };
        expect(kind, &format!("{schema} {}", fill(subject)), case).await;
        store.retire(g).await.unwrap();
    }
    // Orphans in both directions.
    let orphan = GenerationId::from_schema("lctx_g0123456789abcdef0123456789abcdef").unwrap();
    run(&owner, &format!("CREATE SCHEMA {}", orphan.schema())).await;
    expect(FindingKind::Orphan, &orphan.schema(), "orphan schema").await;
    run(&owner, &format!("DROP SCHEMA {}", orphan.schema())).await;
    let mut registered = harness(&store, &db).await;
    run(
        &owner,
        &format!("DROP SCHEMA {} CASCADE", registered.generation().schema()),
    )
    .await;
    expect(
        FindingKind::Orphan,
        &registered.generation().schema(),
        "orphan registry",
    )
    .await;
    // The attempt cannot record a failure without its schema; ending it releases its lock.
    let _ = registered
        .fail(&ModelError::Invalid("orphaned".into()))
        .await;
    store.repair_orphan(registered.generation()).await.unwrap();
    // Store-wide drifts: applied, detected, then reverted.
    let state_check: String = sqlx::query_scalar("SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conname = 'generations_state_check'")
        .fetch_one(&owner).await.unwrap();
    let checksums: Vec<(i64, Vec<u8>)> = sqlx::query_as("SELECT version, checksum FROM public._sqlx_migrations ORDER BY version")
        .fetch_all(&owner)
        .await
        .unwrap();
    let global = [
        (
            "control CHECK",
            &owner,
            "ALTER TABLE lctx_model_store.generations DROP CONSTRAINT generations_state_check"
                .to_owned(),
            format!(
                "ALTER TABLE lctx_model_store.generations ADD CONSTRAINT generations_state_check {state_check}"
            ),
            FindingKind::Missing,
            "lctx_model_store constraint generations.generations_state_check",
        ),
        (
            "migrator-owned lctx_serving",
            &owner,
            "CREATE SCHEMA lctx_serving".into(),
            "DROP SCHEMA lctx_serving".into(),
            FindingKind::Unexpected,
            "lctx_serving",
        ),
        (
            "public object",
            &owner,
            "CREATE TABLE public.drift(id int)".into(),
            "DROP TABLE public.drift".into(),
            FindingKind::Unexpected,
            "public.drift",
        ),
        (
            "service history",
            &owner,
            "UPDATE public._sqlx_migrations SET checksum = '\\x00'".into(),
            checksums.iter().map(|(version, checksum)| format!(
                "UPDATE public._sqlx_migrations SET checksum = '\\x{}' WHERE version = {version}",
                checksum.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )).collect::<Vec<_>>().join(";"),
            FindingKind::Installation,
            "service baseline",
        ),
        (
            "role attribute",
            &db.superuser,
            "ALTER ROLE lctx_serving CREATEDB".into(),
            "ALTER ROLE lctx_serving NOCREATEDB".into(),
            FindingKind::Role,
            "lctx_serving",
        ),
        (
            "membership",
            &db.superuser,
            "GRANT lctx_importer TO lctx_serving".into(),
            "REVOKE lctx_importer FROM lctx_serving".into(),
            FindingKind::Role,
            "lctx_serving -> lctx_importer",
        ),
        (
            "reader not read-only",
            &db.superuser,
            "ALTER ROLE lctx_serving RESET default_transaction_read_only".into(),
            "ALTER ROLE lctx_serving SET default_transaction_read_only = on".into(),
            FindingKind::Role,
            "lctx_serving",
        ),
        (
            "database privilege",
            &db.superuser,
            "GRANT CREATE ON DATABASE lctx TO lctx_app".into(),
            "REVOKE CREATE ON DATABASE lctx FROM lctx_app".into(),
            FindingKind::Database,
            "lctx_app CREATE",
        ),
    ];
    for (case, pool, apply, revert, kind, subject) in global {
        run(pool, &apply).await;
        expect(kind, subject, case).await;
        run(pool, &revert).await;
    }
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(
        report.clean(),
        "every drift was reverted or removed: {:#?}",
        report.findings
    );
}

#[tokio::test]
async fn reset_refuses_live_lease_and_attempt() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let g = published(&store, &db).await;
    let plan = GenerationStore::reset_plan(&db.owner).await.unwrap();
    assert_eq!(
        (plan.database.as_str(), plan.schemas.clone(), plan.control),
        ("lctx", vec![g.schema()], true)
    );
    let lease = store.pin(&db.reader, g, budget()).await.unwrap();
    assert!(matches!(
        GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await,
        Err(Error::Busy)
    ));
    lease.release().await.unwrap();
    assert!(matches!(
        GenerationStore::reset(db.owner.clone(), model.clone(), "postgres").await,
        Err(Error::Confirmation)
    ));
    // A live attempt owns its generation until it ends.
    let mut live = harness(&store, &db).await;
    assert!(matches!(
        GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await,
        Err(Error::Busy)
    ));
    live.fail(&ModelError::Invalid("ended".into()))
        .await
        .unwrap();
    // A step in flight holds the installation lock shared: reset refuses promptly, never queues.
    let step = step_in_flight(db.owner.pool()).await;
    let started = std::time::Instant::now();
    assert!(matches!(
        GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await,
        Err(Error::Busy)
    ));
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    step.commit().await.unwrap();
    let mut refused = plan.clone();
    refused.schemas.push(live.generation().schema());
    refused.schemas.sort();
    assert_eq!(
        GenerationStore::reset_plan(&db.owner).await.unwrap(),
        refused,
        "a refused reset changes nothing"
    );
    let (dropped, _) = GenerationStore::reset(db.owner.clone(), model.clone(), "lctx")
        .await
        .unwrap();
    assert_eq!(dropped, refused);
    assert!(!schemas(&db.superuser).await.contains(&g.schema()));
}

#[tokio::test]
async fn reset_drops_only_inventoried_objects() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let owner = db.owner.pool().clone();
    let mut staging = harness(&store, &db).await;
    staging
        .fail(&ModelError::Invalid("ended".into()))
        .await
        .unwrap();
    let staging = staging.generation();
    let published = published(&store, &db).await;
    let orphan = GenerationId::from_schema("lctx_gabcdefabcdefabcdefabcdefabcdefab").unwrap();
    let foreign = "lctx_g00000000000000000000000000000001";
    run(&owner, &format!("CREATE SCHEMA {}; CREATE SCHEMA scratch; CREATE TABLE scratch.notes(id int); CREATE SCHEMA lctx_gfoo; \
        INSERT INTO lctx_ops.attempts(attempt_id, compiler_digest, library, store_path) VALUES (decode(repeat('01', 16), 'hex'), decode(repeat('02', 32), 'hex'), 'fastmcp', 'kept')",
        orphan.schema())).await;
    run(&db.superuser, &format!("CREATE SCHEMA {foreign}")).await;
    let mut expected = vec![staging.schema(), published.schema(), orphan.schema()];
    expected.sort();
    let plan = GenerationStore::reset_plan(&db.owner).await.unwrap();
    assert_eq!((plan.schemas.clone(), plan.control), (expected, true));
    let (dropped, _) = GenerationStore::reset(db.owner.clone(), model.clone(), "lctx")
        .await
        .unwrap();
    assert_eq!(dropped, plan);
    assert_eq!(
        schemas(&db.superuser).await,
        [
            "lctx_cache",
            "lctx_ext",
            foreign,
            "lctx_gfoo",
            "lctx_model_store",
            "lctx_ops",
            "public",
            "scratch"
        ]
    );
    let kept: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lctx_ops.attempts WHERE store_path = 'kept'")
            .fetch_one(&owner)
            .await
            .unwrap();
    let notes: Option<String> = sqlx::query_scalar("SELECT to_regclass('scratch.notes')::text")
        .fetch_one(&owner)
        .await
        .unwrap();
    let registry: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.generations")
        .fetch_one(&owner)
        .await
        .unwrap();
    assert_eq!(
        (kept, notes.as_deref(), registry),
        (1, Some("scratch.notes"), 0)
    );
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert_eq!(
        report
            .findings
            .iter()
            .map(|f| (f.kind, f.subject.as_str()))
            .collect::<Vec<_>>(),
        [
            (FindingKind::Unexpected, "lctx_gfoo"),
            (FindingKind::Unexpected, "scratch")
        ],
        "only the operator's own schemas remain to report"
    );
}

#[tokio::test]
async fn reset_reinstalls_new_model() {
    let (db, _dir) = provisioned().await;
    let (old, new) = (full(), extended());
    let before = GenerationStore::install(db.owner.clone(), old.clone())
        .await
        .unwrap();
    published(&before, &db).await;
    // Simulate the preceding control-schema revision: reset must retire generations even
    // when that installation predates private stage reads and checkpoints.
    run(
        &db.superuser,
        "DROP TABLE lctx_model_store.stage_read_checks; DROP TABLE lctx_model_store.checkpoints",
    )
    .await;
    let (_, after) = GenerationStore::reset(db.owner.clone(), new.clone(), "lctx")
        .await
        .unwrap();
    assert_ne!(before.physical_digest(), after.physical_digest());
    let digests: (Vec<u8>, Vec<u8>) =
        sqlx::query_as("SELECT model_digest, physical_digest FROM lctx_model_store.installation")
            .fetch_one(&db.superuser)
            .await
            .unwrap();
    assert_eq!(
        digests,
        (new.digest().0.to_vec(), after.physical_digest().0.to_vec())
    );
    assert!(matches!(
        Harness::begin(&before, db.writer.clone(), Profile::Catalog, budget()).await,
        Err(Error::Contract)
    ));
    let g = published(&after, &db).await;
    let probes: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(format!("{}.reset_probes", g.schema()))
        .fetch_one(&db.superuser)
        .await
        .unwrap();
    assert!(probes.is_some(), "the new model's relation is lowered");
    assert!(
        GenerationStore::check(&db.owner, &new)
            .await
            .unwrap()
            .clean()
    );
    let stale = GenerationStore::check(&db.owner, &old).await.unwrap();
    assert!(
        stale
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::Installation && f.subject == "lctx_model_store")
    );
}

#[tokio::test]
async fn reset_is_phased_and_resumable() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let mut generations = Vec::new();
    for _ in 0..8 {
        generations.push(published(&store, &db).await);
    }
    // An injected fault stops the reset between generations: at the last one in id order, which
    // is the order reset removes them.
    let stuck = *generations.iter().max_by_key(|g| g.hex()).unwrap();
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE FUNCTION public.refuse_reset() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN \
        IF OLD.generation_id = decode('{}', 'hex') THEN RAISE EXCEPTION 'injected reset fault'; END IF; RETURN OLD; END $$; \
        CREATE TRIGGER refuse_reset BEFORE DELETE ON lctx_model_store.events FOR EACH ROW EXECUTE FUNCTION public.refuse_reset()", stuck.hex())))
        .execute(&db.superuser).await.unwrap();
    assert!(matches!(
        GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await,
        Err(Error::Database(_))
    ));
    assert!(
        matches!(harness_attempt(&store, &db).await, Err(Error::Contract)),
        "a withdrawn installation refuses every step"
    );
    let remaining = GenerationStore::reset_plan(&db.owner).await.unwrap();
    assert_eq!(
        remaining.schemas,
        [stuck.schema()],
        "every generation before the fault is gone"
    );
    run(
        &db.superuser,
        "DROP TRIGGER refuse_reset ON lctx_model_store.events; DROP FUNCTION public.refuse_reset()",
    )
    .await;
    // A rerun finishes, eight full-model generations within the server's default lock table.
    GenerationStore::reset(db.owner.clone(), model.clone(), "lctx")
        .await
        .unwrap();
    assert!(
        GenerationStore::reset_plan(&db.owner)
            .await
            .unwrap()
            .schemas
            .is_empty()
    );
    assert!(
        GenerationStore::check(&db.owner, &model)
            .await
            .unwrap()
            .clean()
    );
}
async fn harness_attempt(
    store: &GenerationStore,
    db: &DisposableDatabase,
) -> Result<Harness, Error> {
    Harness::begin(store, db.writer.clone(), Profile::Catalog, budget()).await
}

#[tokio::test]
async fn check_and_reset_refuse_busy_without_stalling_the_store() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let g = published(&store, &db).await;
    let step = step_in_flight(db.owner.pool()).await;
    // A reader with production's short lock timeout pins while check waits to try again.
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(
            db.url("lctx_serving")
                .parse::<sqlx::postgres::PgConnectOptions>()
                .unwrap()
                .options([("lock_timeout", "1s")]),
        )
        .await
        .unwrap();
    let (checked, lease) = tokio::join!(GenerationStore::check(&db.owner, &model), async {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        store.pin(&reader, g, budget()).await
    });
    assert!(matches!(checked, Err(Error::Busy)));
    let lease = lease.expect("a compatible shared request never queues behind check");
    lease.release().await.unwrap();
    step.commit().await.unwrap();
    assert!(
        GenerationStore::check(&db.owner, &model)
            .await
            .unwrap()
            .clean()
    );
}
