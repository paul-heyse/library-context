//! The attempt-owned lifecycle against a disposable real PostgreSQL 18 (cutover plan P1.7, T10;
//! review focus #4; P0 exit F02 and F07). Every control states its answer before it runs.
use lctx_model::domain::{*, admission::*, attribution::*, input::*, stages::*, transfer::TransferKey};
use lctx_postgres::generations::{CleanupOutcome, Error, GenerationStore};
use lctx_postgres::testing::DisposableDatabase;
use sqlx::PgPool;

use lctx_postgres::testing::{Harness, fixtures::*};

#[tokio::test]
async fn a_facts_generation_holds_only_facts_relations_and_publishes_with_its_admission() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let facts = Facts::new();
    let store = GenerationStore::install(db.owner.clone(), facts.model.clone()).await.unwrap();
    let (attempt, receipt) = facts.written(&store, db.writer.clone(), true).await;
    let g = attempt.generation();
    // F02: the schema holds exactly the facts relations; nothing above the frontier exists.
    let tables: Vec<String> = sqlx::query_scalar("SELECT relname::text FROM pg_class WHERE relnamespace = $1::regnamespace AND relkind = 'r' ORDER BY 1")
        .bind(g.schema()).fetch_all(&db.superuser).await.unwrap();
    let mut expected: Vec<String> = facts_relations().iter().map(|r| r.name().to_owned()).collect();
    expected.sort();
    assert_eq!(tables, expected);
    assert!(!tables.contains(&TransferKey::NAME.to_owned()));
    let validated = attempt.seal(receipt).await.unwrap().validate().await.unwrap();
    let admission = validated.admission().expect("a facts attempt is admitted").clone();
    use Availability::*;
    let availability: Vec<_> = admission.availability().iter().map(|(family, availability)| (*family, *availability)).collect();
    assert_eq!(availability, [(FactFamily::Artifacts, Complete), (FactFamily::Syntax, NoScope), (FactFamily::Lexical, NoScope), (FactFamily::Signatures, NoScope),
        (FactFamily::Calls, NoScope), (FactFamily::Types, NoScope), (FactFamily::Exports, NoScope), (FactFamily::Docs, NoScope), (FactFamily::Deployment, Complete),
        (FactFamily::Flow, NotRequested)].into_iter().collect::<std::collections::BTreeMap<_, _>>().into_iter().collect::<Vec<_>>());
    assert_eq!(validated.publish().await.unwrap(), g);
    let (contract, content): (Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT contract_digest, content_digest FROM lctx_model_store.admissions WHERE generation_id = decode($1, 'hex')")
        .bind(g.hex()).fetch_one(&db.reader).await.unwrap();
    assert_eq!((contract, content), (facts.contract().digest().0.to_vec(), admission.content().0.to_vec()), "the reader sees the recorded admission");
    let outcomes: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.stage_outcomes").fetch_one(&db.reader).await.unwrap();
    assert_eq!(outcomes, 5);
    // F02: a relation above the frontier is refused before any scan, never read as empty.
    let mut lease = store.pin(&db.reader, g, budget()).await.unwrap();
    assert!(matches!(lease.visit::<TransferKey>(|_| Ok(())).await, Err(Error::Frontier(_))));
    assert_eq!(lease.read::<ProviderCoverage>().await.unwrap().rows().len(), 2);
    lease.release().await.unwrap();
    let above = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}.transfer_keys", g.schema()))).execute(&db.reader).await;
    assert_eq!(above.unwrap_err().as_database_error().and_then(|e| e.code()).as_deref(), Some("42P01"));
    store.select(g).await.unwrap();
    let report = GenerationStore::check(&db.owner, &facts.model).await.unwrap();
    assert!(report.clean(), "{:#?}", report.findings);
    // A facts attempt whose stored profile no longer matches its admission is refused and failed.
    let (attempt, receipt) = facts.written(&store, db.writer.clone(), true).await;
    let tampered = attempt.generation();
    let validated = attempt.seal(receipt).await.unwrap().validate().await.unwrap();
    sqlx::query("UPDATE lctx_model_store.generations SET profile = 'behavioral' WHERE id = decode($1, 'hex')").bind(tampered.hex()).execute(db.owner.pool()).await.unwrap();
    assert!(matches!(validated.publish().await, Err(Error::Contract)));
    assert_eq!(state(&db, tampered).await.as_deref(), Some("failed"));
    assert_eq!(count(&db, "SELECT count(*) FROM lctx_model_store.admissions WHERE generation_id = decode($1, 'hex')", tampered).await, 0);
    store.abort(tampered).await.unwrap();
}

#[tokio::test]
async fn a_missing_coverage_row_fails_validation_and_a_subset_schedule_never_begins() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let facts = Facts::new();
    let store = GenerationStore::install(db.owner.clone(), facts.model.clone()).await.unwrap();
    let (attempt, receipt) = facts.written(&store, db.writer.clone(), false).await;
    let g = attempt.generation();
    let refused = attempt.seal(receipt).await.unwrap().validate().await;
    assert!(matches!(&refused, Err(Error::Model(ModelError::Frontier(message))) if message.contains("missing Deployment coverage")), "{:?}", refused.err());
    let failure: (String, String) = sqlx::query_as("SELECT from_state, class FROM lctx_model_store.failures WHERE generation_id = decode($1, 'hex')")
        .bind(g.hex()).fetch_one(&db.superuser).await.unwrap();
    assert_eq!((failure.0.as_str(), failure.1.as_str(), state(&db, g).await.as_deref()), ("sealed", "frontier", Some("failed")));
    let report = GenerationStore::check(&db.owner, &facts.model).await.unwrap();
    assert!(report.clean(), "a failed generation keeps a checkable shape: {:#?}", report.findings);
    store.abort(g).await.unwrap();
    // A schedule that cannot produce the frontier is refused before any store effect.
    let subset = Schedule::build(&facts.model, vec![Stage { name: "acquire", inputs: vec![], outputs: vec![RelationUse::of::<InputRevision>()], contributes: vec![],
        coverage: vec![FactFamily::Artifacts], provider: Some(facts.capture.id()), profiles: vec![Profile::Catalog], effect: Effect::Acquisition,
        code: ContentHash::of(b"subset"), configuration: ContentHash::of(b"cfg") }], &[], Profile::Catalog).unwrap();
    let mut execution = subset.execute();
    assert!(matches!(store.begin(db.writer.clone(), &mut execution, &facts.contract(), budget()).await, Err(Error::Frontier(_))));
    let registered: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.generations").fetch_one(&db.superuser).await.unwrap();
    let schemas: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_namespace WHERE nspname LIKE 'lctx\\_g%'").fetch_one(&db.superuser).await.unwrap();
    assert_eq!((registered, schemas), (0, 0));
}

#[tokio::test]
async fn late_write_vs_seal_race() {
    let db = DisposableDatabase::start().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    // The attempt's own copies cannot overlap its seal: `seal` consumes the attempt. What can race
    // it is a writer-role statement issued directly against the staging schema.
    let late = Package { name: "late".into() };
    let mut outcomes = [0; 2];
    for i in 0..50 {
        let mut harness = Harness::begin(&store, db.writer.clone(), Profile::Catalog, budget()).await.unwrap();
        let g = harness.generation();
        harness.copy(&Batch::new(&small.model, vec![Package { name: "first".into() }], &budget()).unwrap(), &budget()).await.unwrap();
        let insert = format!("INSERT INTO {}.packages(id, name) VALUES ($1, 'late')", g.schema());
        let (write, sealed) = tokio::join!(
            async { stagger(i, true).await; sqlx::query(sqlx::AssertSqlSafe(insert.clone())).bind(late.id().bytes().to_vec()).execute(&db.writer).await },
            async { stagger(i, false).await; harness.seal().await });
        sealed.unwrap();
        let rows = count(&db, &format!("SELECT count(*) FROM {}.packages WHERE $1 IS NOT NULL", g.schema()), g).await;
        match write {
            Ok(_) => { assert_eq!(rows, 2, "iteration {i}: a write that succeeded is sealed"); outcomes[0] += 1; },
            Err(error) => {
                assert_eq!(error.as_database_error().and_then(|e| e.code()).as_deref(), Some("42501"), "iteration {i}: the sealed writer is revoked");
                assert_eq!(rows, 1, "iteration {i}: a refused write left no row");
                outcomes[1] += 1;
            },
        }
        harness.abort().await.unwrap();
    }
    eprintln!("late write vs seal: {} written before the seal, {} refused after it", outcomes[0], outcomes[1]);
    assert!(outcomes.iter().all(|n| *n > 0), "both orderings occur: {outcomes:?}");
}

#[tokio::test]
async fn failed_validation_is_terminal_and_abort_only() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    let dangling = Release { package: Package { name: "absent".into() }.id(), version: "1".into() };
    let (attempt, receipt) = small.written(&store, db.writer.clone(), &["present"], vec![dangling]).await;
    let g = attempt.generation();
    let refused = attempt.seal(receipt).await.unwrap().validate().await;
    assert!(matches!(refused, Err(Error::Database(_))), "the reference fails when validation adds it");
    let failure: (String, String, String) = sqlx::query_as("SELECT from_state, class, detail FROM lctx_model_store.failures WHERE generation_id = decode($1, 'hex')")
        .bind(g.hex()).fetch_one(&db.superuser).await.unwrap();
    assert_eq!((failure.0.as_str(), failure.1.as_str()), ("sealed", "invalid"), "a violated reference is invalid content");
    assert!(failure.2.starts_with("SQLSTATE 23503 constraint ref_") && failure.2.contains("table releases"), "{}", failure.2);
    let report = GenerationStore::check(&db.owner, &small.model).await.unwrap();
    assert!(report.clean(), "{:#?}", report.findings);
    assert!(matches!(store.select(g).await, Err(Error::State)));
    assert!(matches!(store.pin(&db.reader, g, budget()).await, Err(Error::State)));
    assert!(matches!(store.retire(g).await, Err(Error::State)), "a failed generation is aborted, never retired");
    assert!(store.interrupted().await.unwrap().is_empty(), "a failed generation is not an interrupted one");
    assert_eq!(store.abort(g).await.unwrap(), CleanupOutcome::Removed);
    assert_eq!(count(&db, "SELECT count(*) FROM lctx_model_store.failures WHERE generation_id = decode($1, 'hex')", g).await, 0);
}

#[tokio::test]
async fn failed_publication_is_atomic() {
    let db = DisposableDatabase::start().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    let faults = [
        ("a stage receipt missing", "DELETE FROM lctx_model_store.stage_receipts WHERE generation_id = decode($1, 'hex') AND relation_name = 'releases'"),
        ("a relation receipt altered", "UPDATE lctx_model_store.receipts SET content_digest = sha256(content_digest) WHERE generation_id = decode($1, 'hex') AND relation_name = 'packages'"),
        ("the content digest altered", "UPDATE lctx_model_store.generations SET content_digest = sha256(content_digest) WHERE id = decode($1, 'hex')"),
    ];
    for (fault, sql) in faults {
        let (attempt, receipt) = small.written(&store, db.writer.clone(), &["present"], vec![]).await;
        let g = attempt.generation();
        let validated = attempt.seal(receipt).await.unwrap().validate().await.unwrap();
        sqlx::query(sql).bind(g.hex()).execute(db.owner.pool()).await.unwrap();
        assert!(validated.publish().await.is_err(), "{fault}");
        let granted: bool = sqlx::query_scalar("SELECT has_schema_privilege('lctx_serving', $1, 'USAGE')").bind(g.schema()).fetch_one(&db.superuser).await.unwrap();
        let published = count(&db, "SELECT count(*) FROM lctx_model_store.events WHERE generation_id = decode($1, 'hex') AND state = 'published'", g).await;
        assert_eq!((state(&db, g).await.as_deref(), granted, published), (Some("failed"), false, 0), "{fault}: nothing of publication survives");
        store.abort(g).await.unwrap();
    }
}

#[tokio::test]
async fn a_publication_fault_after_its_effects_undoes_them_all() {
    let db = DisposableDatabase::start().await;
    let facts = Facts::new();
    let store = GenerationStore::install(db.owner.clone(), facts.model.clone()).await.unwrap();
    // The fault fires on the last write of publication, after the admission rows and the grant.
    sqlx::raw_sql("CREATE FUNCTION public.refuse_publication() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN \
        IF NEW.state = 'published' THEN RAISE EXCEPTION 'injected publication fault'; END IF; RETURN NEW; END $$; \
        CREATE TRIGGER refuse_publication BEFORE INSERT ON lctx_model_store.events FOR EACH ROW EXECUTE FUNCTION public.refuse_publication()")
        .execute(&db.superuser).await.unwrap();
    let (attempt, receipt) = facts.written(&store, db.writer.clone(), true).await;
    let g = attempt.generation();
    let validated = attempt.seal(receipt).await.unwrap().validate().await.unwrap();
    assert!(matches!(validated.publish().await, Err(Error::Database(_))));
    let granted: bool = sqlx::query_scalar("SELECT has_schema_privilege('lctx_serving', $1, 'USAGE')").bind(g.schema()).fetch_one(&db.superuser).await.unwrap();
    let admitted = count(&db, "SELECT count(*) FROM lctx_model_store.admissions WHERE generation_id = decode($1, 'hex')", g).await
        + count(&db, "SELECT count(*) FROM lctx_model_store.admission_families WHERE generation_id = decode($1, 'hex')", g).await;
    assert_eq!((state(&db, g).await.as_deref(), granted, admitted), (Some("failed"), false, 0), "grant, admission and state were one transaction");
    sqlx::raw_sql("DROP TRIGGER refuse_publication ON lctx_model_store.events; DROP FUNCTION public.refuse_publication()").execute(&db.superuser).await.unwrap();
    store.abort(g).await.unwrap();
}

#[tokio::test]
async fn interrupted_attempts_are_listed_never_published_and_failures_keep_their_class() {
    let db = DisposableDatabase::start().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    let (attempt, receipt) = small.written(&store, db.writer.clone(), &["present"], vec![]).await;
    let g = attempt.generation();
    assert!(store.interrupted().await.unwrap().is_empty(), "a live attempt is not interrupted");
    // The lifecycle connection is lost: its lock goes with it, and sealing reports a transport failure.
    sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE usename = 'lctx_migrator' AND pid <> pg_backend_pid()")
        .execute(&db.superuser).await.unwrap();
    let error = attempt.seal(receipt).await.err().expect("the lifecycle connection is gone");
    assert_eq!(error.class(), Infrastructure::Transport, "{error}");
    assert_eq!(store.interrupted().await.unwrap(), [g]);
    assert_eq!(state(&db, g).await.as_deref(), Some("staging"));
    store.abort(g).await.unwrap();
    // A copy whose writer transport is gone keeps its class through the stage sink.
    let writer = PgPool::connect(&db.url("lctx_importer")).await.unwrap();
    let mut execution = small.schedule.execute();
    let attempt = store.begin_conformance(writer.clone(), &mut execution, budget()).await.unwrap();
    writer.close().await;
    let mut access = execution.begin("packages").unwrap();
    let batch = Batch::new(&small.model, vec![Package { name: "lost".into() }], &budget()).unwrap();
    let copied = access.write::<Package, _>(async |permit| attempt.copy(permit, &batch).await).await;
    assert!(matches!(copied, Err(ModelError::Infrastructure { class: Infrastructure::Transport, .. })), "{copied:?}");
    drop(access);
    let id = attempt.fail(&copied.unwrap_err()).await.unwrap();
    assert_eq!(state(&db, id).await.as_deref(), Some("failed"));
    let class: String = sqlx::query_scalar("SELECT class FROM lctx_model_store.failures WHERE generation_id = decode($1, 'hex')").bind(id.hex())
        .fetch_one(&db.superuser).await.unwrap();
    assert_eq!(class, "transport", "the producer's failure keeps the class the stage saw");
    store.abort(id).await.unwrap();
    // A content violation the server finds in a stage COPY is invalid content, with its SQLSTATE and
    // constraint (P1.10 review F03), not a refusal.
    let mut execution = small.schedule.execute();
    let attempt = store.begin_conformance(db.writer.clone(), &mut execution, budget()).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("ALTER TABLE {}.packages ADD CONSTRAINT probe CHECK (name <> 'refused')", attempt.generation().schema())))
        .execute(db.owner.pool()).await.unwrap();
    let mut access = execution.begin("packages").unwrap();
    let batch = Batch::new(&small.model, vec![Package { name: "refused".into() }], &budget()).unwrap();
    let copied = access.write::<Package, _>(async |permit| attempt.copy(permit, &batch).await).await;
    assert!(matches!(&copied, Err(ModelError::Invalid(detail)) if detail.contains("SQLSTATE 23514 constraint probe")), "{copied:?}");
    drop(access);
    let id = attempt.fail(&copied.unwrap_err()).await.unwrap();
    let (class, detail): (String, String) = sqlx::query_as("SELECT class, detail FROM lctx_model_store.failures WHERE generation_id = decode($1, 'hex')")
        .bind(id.hex()).fetch_one(&db.superuser).await.unwrap();
    assert_eq!(class, "invalid", "{detail}");
    assert!(detail.contains("SQLSTATE 23514 constraint probe") && detail.contains("table packages"), "{detail}");
    store.abort(id).await.unwrap();
    assert_eq!(Error::Commit(sqlx::Error::PoolClosed).class(), Infrastructure::Unconfirmed, "an unconfirmed commit is its own class");
}

#[tokio::test]
async fn a_live_attempt_cannot_be_aborted() {
    let db = DisposableDatabase::start().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    let (attempt, receipt) = small.written(&store, db.writer.clone(), &["present"], vec![]).await;
    let g = attempt.generation();
    assert!(matches!(store.abort(g).await, Err(Error::Busy)));
    let sealed = attempt.seal(receipt).await.unwrap();
    assert!(matches!(store.abort(g).await, Err(Error::Busy)), "a sealed attempt still owns its generation");
    assert!(matches!(GenerationStore::reset(db.owner.clone(), small.model.clone(), "lctx").await, Err(Error::Busy)), "nor can a reset drop it");
    assert_eq!(sealed.fail(&ModelError::Invalid("abandoned".into())).await.unwrap(), g);
    assert_eq!(store.abort(g).await.unwrap(), CleanupOutcome::Removed);
    // T9: an attempt removes its own generation, with no registry row left behind.
    let (attempt, _) = small.written(&store, db.writer.clone(), &["present"], vec![]).await;
    let aborted = attempt.abort().await.unwrap();
    assert_eq!(state(&db, aborted).await, None);
    assert!(matches!(store.pin(&db.reader, aborted, budget()).await, Err(Error::Absent)), "an absent generation is not an unpublished one");
}

#[tokio::test]
async fn lease_vs_retire_race() {
    let db = DisposableDatabase::start().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    let mut outcomes = [0; 2];
    for i in 0..50 {
        let g = small.published(&store, &db.writer, &format!("lease{i}")).await;
        let (lease, retired) = tokio::join!(async { stagger(i, true).await; store.pin(&db.reader, g, budget()).await },
            async { stagger(i, false).await; store.retire(g).await });
        match (lease, retired) {
            (Ok(mut lease), Err(Error::Busy)) => {
                assert_eq!(lease.read::<Package>().await.unwrap().rows().len(), 1, "iteration {i}: a pinned generation stays readable");
                lease.release().await.unwrap();
                store.retire(g).await.unwrap();
                outcomes[0] += 1;
            },
            (Err(Error::Absent), Ok(CleanupOutcome::Removed)) => outcomes[1] += 1,
            (lease, retired) => panic!("iteration {i}: lease {:?} with retire {retired:?}", lease.map(|_| ())),
        }
    }
    eprintln!("lease vs retire: {} leased first, {} retired first", outcomes[0], outcomes[1]);
    assert!(outcomes.iter().all(|n| *n > 0), "both orderings occur: {outcomes:?}");
}

#[tokio::test]
async fn select_vs_retire_race() {
    let db = DisposableDatabase::start().await;
    let facts = Facts::new();
    let store = GenerationStore::install(db.owner.clone(), facts.model.clone()).await.unwrap();
    let mut outcomes = [0; 2];
    for i in 0..10 {
        let g = facts.published(&store, db.writer.clone()).await;
        let (selected, retired) = tokio::join!(async { stagger(i, true).await; store.select(g).await },
            async { stagger(i, false).await; store.retire(g).await });
        let pointer: Option<Vec<u8>> = sqlx::query_scalar("SELECT generation_id FROM lctx_model_store.selection").fetch_one(&db.superuser).await.unwrap();
        match (selected, retired) {
            (Ok(()), Err(Error::Busy)) => {
                assert_eq!(pointer.map(hex), Some(g.hex()), "iteration {i}");
                store.clear_selection().await.unwrap();
                store.retire(g).await.unwrap();
                outcomes[0] += 1;
            },
            (Err(Error::Absent), Ok(CleanupOutcome::Removed)) => { assert_eq!(pointer, None, "iteration {i}: nothing selects a retired generation"); outcomes[1] += 1; },
            (selected, retired) => panic!("iteration {i}: select {selected:?} with retire {retired:?}"),
        }
    }
    eprintln!("select vs retire: {} selected first, {} retired first", outcomes[0], outcomes[1]);
    assert!(outcomes.iter().all(|n| *n > 0), "both orderings occur: {outcomes:?}");
}
/// Alternate which side of a race leads, by a varying head start, so both orderings occur.
async fn stagger(iteration: usize, first: bool) {
    if (iteration % 2 == 0) != first { tokio::time::sleep(std::time::Duration::from_millis(2 * (iteration % 5) as u64 + 2)).await; }
}
fn hex(bytes: Vec<u8>) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }

#[tokio::test]
async fn retire_is_atomic_under_fault() {
    let db = DisposableDatabase::start().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    let g = small.published(&store, &db.writer, "kept").await;
    sqlx::raw_sql("CREATE FUNCTION public.refuse_delete() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected retirement fault'; END $$; \
        CREATE TRIGGER refuse_delete BEFORE DELETE ON lctx_model_store.events FOR EACH ROW EXECUTE FUNCTION public.refuse_delete()")
        .execute(&db.superuser).await.unwrap();
    assert!(matches!(store.retire(g).await, Err(Error::Database(_))));
    let schema: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_namespace WHERE nspname = $1)").bind(g.schema()).fetch_one(&db.superuser).await.unwrap();
    assert_eq!((schema, state(&db, g).await.as_deref()), (true, Some("published")), "a failed retirement removes nothing");
    let mut lease = store.pin(&db.reader, g, budget()).await.unwrap();
    assert_eq!(lease.read::<Package>().await.unwrap().rows().len(), 1);
    lease.release().await.unwrap();
    sqlx::raw_sql("DROP TRIGGER refuse_delete ON lctx_model_store.events; DROP FUNCTION public.refuse_delete()").execute(&db.superuser).await.unwrap();
    assert_eq!(store.retire(g).await.unwrap(), CleanupOutcome::Removed);
    assert_eq!(state(&db, g).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mixed_lifecycle_does_not_deadlock() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone()).await.unwrap();
    let attempts = (0..4).map(|i| {
        let (store, small, writer) = (store.clone(), &small, db.writer.clone());
        async move {
            let (attempt, receipt) = small.written(&store, writer, &[&format!("attempt{i}")], vec![]).await;
            let g = attempt.seal(receipt).await?.validate().await?.publish().await?;
            store.retire(g).await.map(drop)
        }
    });
    let readers = (0..3).map(|i| {
        let (store, small, db) = (store.clone(), &small, &db);
        async move {
            let g = small.published(&store, &db.writer, &format!("reader{i}")).await;
            let mut lease = store.pin(&db.reader, g, budget()).await?;
            lease.read::<Package>().await?;
            lease.release().await?;
            store.retire(g).await.map(drop)
        }
    });
    let observers = async {
        for _ in 0..5 { store.interrupted().await?; GenerationStore::check(&db.owner, &small.model).await?; }
        Ok::<_, Error>(())
    };
    let all = async {
        let (attempts, readers, observed) = tokio::join!(futures::future::join_all(attempts), futures::future::join_all(readers), observers);
        (attempts, readers, observed)
    };
    let (attempts, readers, observed) = tokio::time::timeout(std::time::Duration::from_secs(120), all).await.expect("no deadlock within two minutes");
    for result in attempts.into_iter().chain(readers) { result.unwrap(); }
    observed.unwrap();
    let registered: i64 = sqlx::query_scalar("SELECT count(*) FROM lctx_model_store.generations").fetch_one(&db.superuser).await.unwrap();
    assert_eq!(registered, 0, "every generation was retired");
}
