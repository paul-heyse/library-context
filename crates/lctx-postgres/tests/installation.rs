//! Generated install, `store check` and `store reset` against a disposable real PostgreSQL 18
//! (cutover plan P1.6). Every drift is a pre-written mutation with the finding it must produce.
use std::sync::Arc;
use lctx_model::{Domain, domain::{ContentHash, Relation, ValidatedModel, model}};
use lctx_model::domain::resources::ResourceBudget;
use lctx_postgres::{Config, OwnerPool, generations::{Error, FindingKind, GenerationId, GenerationStore}, testing::DisposableDatabase};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "reset_probes", semantic_source = include_bytes!("installation.rs"))]
struct ResetProbe { #[model(key)] name: String }

fn budget() -> ResourceBudget { ResourceBudget::fixed(1 << 28).unwrap() }
fn full() -> Arc<ValidatedModel> { Arc::new(model().unwrap()) }
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
    Config::load(&dir.path().join("postgres.json")).unwrap().connect_migrator().await.unwrap().migrate().await.unwrap();
    (db, dir)
}
async fn run(pool: &sqlx::PgPool, sql: &str) { sqlx::raw_sql(sqlx::AssertSqlSafe(sql.to_owned())).execute(pool).await.unwrap(); }
async fn published(store: &GenerationStore) -> GenerationId {
    let g = store.create_conformance(ContentHash::of(b"installation"), "catalog").await.unwrap();
    store.seal(g).await.unwrap();
    store.validate(g, &budget()).await.unwrap();
    store.publish(g).await.unwrap();
    g
}
async fn schemas(pool: &sqlx::PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT nspname::text FROM pg_namespace WHERE nspname NOT LIKE 'pg\\_%' AND nspname <> 'information_schema' ORDER BY 1")
        .fetch_all(pool).await.unwrap()
}
/// The session-level lock key a lifecycle transaction or lease holds for a generation.
fn lock_key(g: GenerationId) -> i64 {
    let hex = g.hex();
    let bytes: Vec<u8> = (0..8).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
    i64::from_le_bytes(bytes.try_into().unwrap())
}

#[tokio::test]
async fn install_requires_service_owner() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let before = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert_eq!(before.findings.len(), 1, "{:#?}", before.findings);
    assert_eq!((before.findings[0].kind, before.findings[0].subject.as_str()), (FindingKind::Installation, "lctx_model_store"));
    // A runtime role cannot become the owner, so it cannot install (the type admits only a verified owner).
    assert!(matches!(OwnerPool::verify(db.writer.clone()).await, Err(lctx_postgres::Error::Owner(_))));
    GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
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
    let first = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let catalog = schemas(&db.superuser).await;
    let second = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    assert_eq!(first.physical_digest(), second.physical_digest(), "the lowering is deterministic");
    assert_eq!(schemas(&db.superuser).await, catalog);
    let installations: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.installation").fetch_one(&db.superuser).await.unwrap();
    assert_eq!(installations, 1);
    // Twin: another model is refused and the installation is unchanged.
    let other = extended();
    assert!(matches!(GenerationStore::install(db.owner.clone(), other.clone()).await, Err(Error::Contract)));
    let digest: Vec<u8> = sqlx::query_scalar("SELECT physical_digest FROM lctx_model_store.installation").fetch_one(&db.superuser).await.unwrap();
    assert_eq!(digest, first.physical_digest().0);
    let report = GenerationStore::check(&db.owner, &other).await.unwrap();
    assert_eq!(report.findings.iter().map(|f| (f.kind, f.subject.as_str())).collect::<Vec<_>>(), [(FindingKind::Installation, "lctx_model_store")]);
    assert!(GenerationStore::check(&db.owner, &model).await.unwrap().clean());
}

#[tokio::test]
async fn check_clean_in_every_state() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let staging = store.create_conformance(ContentHash::of(b"staging"), "catalog").await.unwrap();
    let sealed = store.create_conformance(ContentHash::of(b"sealed"), "behavioral").await.unwrap();
    store.seal(sealed).await.unwrap();
    let validated = store.create_conformance(ContentHash::of(b"validated"), "catalog").await.unwrap();
    store.seal(validated).await.unwrap();
    store.validate(validated, &budget()).await.unwrap();
    let published = published(&store).await;
    let lease = store.pin(&db.reader, published, budget()).await.unwrap();
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(report.clean(), "{:#?}", report.findings);
    assert_eq!(report.generations, 4);
    assert!(!schemas(&db.superuser).await.iter().any(|s| s.starts_with("lctx_check_")), "shadows never survive a check");
    lease.release().await.unwrap();
    store.abort(staging).await.unwrap();
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(report.clean(), "{:#?}", report.findings);
    assert_eq!(report.generations, 3);
}

#[tokio::test]
async fn check_detects_each_drift() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let owner = db.owner.pool().clone();
    let expect = async |kind: FindingKind, subject: &str, case: &str| {
        let report = GenerationStore::check(&db.owner, &model).await.unwrap();
        assert!(report.findings.iter().any(|f| f.kind == kind && f.subject.contains(subject)),
            "{case}: expected {kind:?} {subject}, found {:#?}", report.findings);
    };
    // Drifts inside a generation schema: each on a disposable generation, removed afterwards.
    let staging_cases: [(&str, &str, FindingKind, &str); 8] = [
        ("column type", "ALTER TABLE {g}.packages ALTER COLUMN name TYPE varchar(64)", FindingKind::Differs, "column packages.name"),
        ("NOT NULL", "ALTER TABLE {g}.packages ALTER COLUMN name DROP NOT NULL", FindingKind::Differs, "column packages.name"),
        ("dropped CHECK", "ALTER TABLE {g}.packages DROP CONSTRAINT {check}", FindingKind::Missing, "constraint packages."),
        ("dropped index", "ALTER TABLE {g}.{indexed} DROP CONSTRAINT {index}", FindingKind::Missing, "index {index}"),
        ("extra index", "CREATE INDEX drift_extra ON {g}.packages(name)", FindingKind::Unexpected, "index drift_extra"),
        ("extra grant", "GRANT SELECT ON {g}.packages TO lctx_app", FindingKind::Differs, "relation packages"),
        ("owner change", "ALTER TABLE {g}.packages OWNER TO postgres", FindingKind::Differs, "relation packages"),
        ("extra table", "CREATE TABLE {g}.drift_extra(id int)", FindingKind::Unexpected, "relation drift_extra"),
    ];
    for (case, sql, kind, subject) in staging_cases {
        let g = store.create_conformance(ContentHash::of(case.as_bytes()), "catalog").await.unwrap();
        let schema = g.schema();
        let check: String = sqlx::query_scalar("SELECT conname::text FROM pg_constraint WHERE conrelid = ($1 || '.packages')::regclass AND contype = 'c' ORDER BY 1 LIMIT 1")
            .bind(&schema).fetch_one(&owner).await.unwrap();
        // A sum relation's tag index backs its unique constraint; dropping the constraint drops it.
        let (index, indexed): (String, String) = sqlx::query_as("SELECT c.relname::text, t.relname::text FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid \
            JOIN pg_class t ON t.oid = i.indrelid WHERE c.relnamespace = $1::regnamespace AND NOT i.indisprimary ORDER BY 1 LIMIT 1").bind(&schema).fetch_one(&owner).await.unwrap();
        let fill = |text: &str| text.replace("{g}", &schema).replace("{check}", &check).replace("{index}", &index).replace("{indexed}", &indexed);
        let (sql, subject) = (fill(sql), fill(subject));
        run(if case == "owner change" { &db.superuser } else { &owner }, &sql).await;
        expect(kind, &format!("{schema} {subject}"), case).await;
        store.abort(g).await.unwrap();
    }
    for (case, sql, subject) in [
        ("dropped FK", "ALTER TABLE {g}.{table} DROP CONSTRAINT {fk}", "constraint {table}.{fk}"),
        ("revoked grant", "REVOKE SELECT ON {g}.packages FROM lctx_serving", "relation packages"),
    ] {
        let g = published(&store).await;
        let schema = g.schema();
        let (table, fk): (String, String) = sqlx::query_as("SELECT c.relname::text, k.conname::text FROM pg_constraint k JOIN pg_class c ON c.oid = k.conrelid \
            WHERE c.relnamespace = $1::regnamespace AND k.contype = 'f' ORDER BY 1, 2 LIMIT 1").bind(&schema).fetch_one(&owner).await.unwrap();
        let fill = |text: &str| text.replace("{g}", &schema).replace("{table}", &table).replace("{fk}", &fk);
        run(&owner, &fill(sql)).await;
        let kind = if case == "dropped FK" { FindingKind::Missing } else { FindingKind::Differs };
        expect(kind, &format!("{schema} {}", fill(subject)), case).await;
        store.retire(g).await.unwrap();
    }
    // Orphans in both directions.
    let orphan = GenerationId::from_schema("lctx_g0123456789abcdef0123456789abcdef").unwrap();
    run(&owner, &format!("CREATE SCHEMA {}", orphan.schema())).await;
    expect(FindingKind::Orphan, &orphan.schema(), "orphan schema").await;
    run(&owner, &format!("DROP SCHEMA {}", orphan.schema())).await;
    let registered = store.create_conformance(ContentHash::of(b"orphan registry"), "catalog").await.unwrap();
    run(&owner, &format!("DROP SCHEMA {} CASCADE", registered.schema())).await;
    expect(FindingKind::Orphan, &registered.schema(), "orphan registry").await;
    store.repair_orphan(registered).await.unwrap();
    // Store-wide drifts: applied, detected, then reverted.
    let state_check: String = sqlx::query_scalar("SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conname = 'generations_state_check'")
        .fetch_one(&owner).await.unwrap();
    let checksum: Vec<u8> = sqlx::query_scalar("SELECT checksum FROM public._sqlx_migrations").fetch_one(&owner).await.unwrap();
    let global = [
        ("control CHECK", &owner, "ALTER TABLE lctx_model_store.generations DROP CONSTRAINT generations_state_check".to_owned(),
            format!("ALTER TABLE lctx_model_store.generations ADD CONSTRAINT generations_state_check {state_check}"),
            FindingKind::Missing, "lctx_model_store constraint generations.generations_state_check"),
        ("migrator-owned lctx_serving", &owner, "CREATE SCHEMA lctx_serving".into(), "DROP SCHEMA lctx_serving".into(), FindingKind::Unexpected, "lctx_serving"),
        ("public object", &owner, "CREATE TABLE public.drift(id int)".into(), "DROP TABLE public.drift".into(), FindingKind::Unexpected, "public.drift"),
        ("service history", &owner, "UPDATE public._sqlx_migrations SET checksum = '\\x00'".into(),
            format!("UPDATE public._sqlx_migrations SET checksum = '\\x{}'", checksum.iter().map(|b| format!("{b:02x}")).collect::<String>()),
            FindingKind::Installation, "service baseline"),
        ("role attribute", &db.superuser, "ALTER ROLE lctx_serving CREATEDB".into(), "ALTER ROLE lctx_serving NOCREATEDB".into(), FindingKind::Role, "lctx_serving"),
        ("membership", &db.superuser, "GRANT lctx_importer TO lctx_serving".into(), "REVOKE lctx_importer FROM lctx_serving".into(), FindingKind::Role, "lctx_serving -> lctx_importer"),
        ("reader not read-only", &db.superuser, "ALTER ROLE lctx_serving RESET default_transaction_read_only".into(),
            "ALTER ROLE lctx_serving SET default_transaction_read_only = on".into(), FindingKind::Role, "lctx_serving"),
        ("database privilege", &db.superuser, "GRANT CREATE ON DATABASE lctx TO lctx_app".into(), "REVOKE CREATE ON DATABASE lctx FROM lctx_app".into(),
            FindingKind::Database, "lctx_app CREATE"),
    ];
    for (case, pool, apply, revert, kind, subject) in global {
        run(pool, &apply).await;
        expect(kind, subject, case).await;
        run(pool, &revert).await;
    }
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert!(report.clean(), "every drift was reverted or removed: {:#?}", report.findings);
}

#[tokio::test]
async fn reset_refuses_live_lease_and_attempt() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let g = published(&store).await;
    let plan = GenerationStore::reset_plan(&db.owner).await.unwrap();
    assert_eq!((plan.database.as_str(), plan.schemas.clone(), plan.control), ("lctx", vec![g.schema()], true));
    let lease = store.pin(&db.reader, g, budget()).await.unwrap();
    assert!(matches!(GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await, Err(Error::Busy)));
    lease.release().await.unwrap();
    assert!(matches!(GenerationStore::reset(db.owner.clone(), model.clone(), "postgres").await, Err(Error::Confirmation)));
    // A lifecycle transaction in flight holds the generation lock (P1.7 adds the attempt lock).
    let mut transition = db.writer.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock_shared($1)").bind(lock_key(g)).execute(&mut *transition).await.unwrap();
    assert!(matches!(GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await, Err(Error::Busy)));
    assert_eq!(GenerationStore::reset_plan(&db.owner).await.unwrap(), plan, "a refused reset changes nothing");
    transition.commit().await.unwrap();
    let (dropped, _) = GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await.unwrap();
    assert_eq!(dropped, plan);
    assert!(!schemas(&db.superuser).await.contains(&g.schema()));
}

#[tokio::test]
async fn reset_drops_only_inventoried_objects() {
    let (db, _dir) = provisioned().await;
    let model = full();
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let owner = db.owner.pool().clone();
    let staging = store.create_conformance(ContentHash::of(b"staging"), "catalog").await.unwrap();
    let published = published(&store).await;
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
    let (dropped, _) = GenerationStore::reset(db.owner.clone(), model.clone(), "lctx").await.unwrap();
    assert_eq!(dropped, plan);
    assert_eq!(schemas(&db.superuser).await, ["lctx_cache", foreign, "lctx_gfoo", "lctx_model_store", "lctx_ops", "public", "scratch"]);
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_ops.attempts WHERE store_path = 'kept'").fetch_one(&owner).await.unwrap();
    let notes: Option<String> = sqlx::query_scalar("SELECT to_regclass('scratch.notes')::text").fetch_one(&owner).await.unwrap();
    let registry: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.generations").fetch_one(&owner).await.unwrap();
    assert_eq!((kept, notes.as_deref(), registry), (1, Some("scratch.notes"), 0));
    let report = GenerationStore::check(&db.owner, &model).await.unwrap();
    assert_eq!(report.findings.iter().map(|f| (f.kind, f.subject.as_str())).collect::<Vec<_>>(),
        [(FindingKind::Unexpected, "lctx_gfoo"), (FindingKind::Unexpected, "scratch")], "only the operator's own schemas remain to report");
}

#[tokio::test]
async fn reset_reinstalls_new_model() {
    let (db, _dir) = provisioned().await;
    let (old, new) = (full(), extended());
    let before = GenerationStore::install(db.owner.clone(), old.clone()).await.unwrap();
    published(&before).await;
    let (_, after) = GenerationStore::reset(db.owner.clone(), new.clone(), "lctx").await.unwrap();
    assert_ne!(before.physical_digest(), after.physical_digest());
    let digests: (Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT model_digest, physical_digest FROM lctx_model_store.installation").fetch_one(&db.superuser).await.unwrap();
    assert_eq!(digests, (new.digest().0.to_vec(), after.physical_digest().0.to_vec()));
    assert!(matches!(before.create_conformance(ContentHash::of(b"stale"), "catalog").await, Err(Error::Contract)));
    let g = published(&after).await;
    let probes: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text").bind(format!("{}.reset_probes", g.schema())).fetch_one(&db.superuser).await.unwrap();
    assert!(probes.is_some(), "the new model's relation is lowered");
    assert!(GenerationStore::check(&db.owner, &new).await.unwrap().clean());
    let stale = GenerationStore::check(&db.owner, &old).await.unwrap();
    assert!(stale.findings.iter().any(|f| f.kind == FindingKind::Installation && f.subject == "lctx_model_store"));
}
