//! The generation catalog against a disposable real PostgreSQL 18 (cutover plan P1.8): every
//! state, frontier and writer liveness reported as stored, read by the reader role.
use lctx_model::domain::{
    admission::{Availability, Frontier},
    attribution::FactFamily,
    stages::Profile,
};
use lctx_postgres::generations::{
    FailureClass, GenerationCatalog, GenerationState, GenerationStore, ListFilter, Writer,
};
use lctx_postgres::testing::{DisposableDatabase, Harness};

use lctx_postgres::testing::fixtures::*;

#[tokio::test]
async fn the_catalog_reports_every_state_frontier_and_writer() {
    let db = DisposableDatabase::start().await;
    let facts = Facts::new();
    let store = GenerationStore::install(db.owner.clone(), facts.model.clone())
        .await
        .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    // Live conformance attempts in each unpublished state.
    let staging_h = Harness::begin_empty_conformance(
        &store,
        db.writer.clone(),
        Profile::Behavioral,
        budget(),
        vec![],
    )
    .await
    .unwrap();
    let staging = staging_h.generation();
    let mut sealed_h = Harness::begin_empty_conformance(
        &store,
        db.writer.clone(),
        Profile::Catalog,
        budget(),
        vec![],
    )
    .await
    .unwrap();
    let sealed = sealed_h.generation();
    sealed_h.seal().await.unwrap();
    let mut validated_h = Harness::begin_empty_conformance(
        &store,
        db.writer.clone(),
        Profile::Catalog,
        budget(),
        vec![],
    )
    .await
    .unwrap();
    let validated = validated_h.generation();
    validated_h.seal().await.unwrap();
    validated_h.validate(&budget()).await.unwrap();
    // Facts attempts: one published and selected, one live, one failed.
    let published = facts.published(&store, db.writer.clone()).await;
    store.select(published).await.unwrap();
    let (live, _receipt) = facts.written(&store, db.writer.clone(), true).await;
    let (refused, receipt) = facts.written(&store, db.writer.clone(), false).await;
    let failed = refused.generation();
    assert!(
        refused
            .seal(receipt)
            .await
            .unwrap()
            .validate()
            .await
            .is_err()
    );
    let rows = catalog.list(&ListFilter::default()).await.unwrap();
    let row = |g| rows.iter().find(|r| r.id == g).unwrap();
    use GenerationState::*;
    let expected = [
        (
            staging,
            Staging,
            Frontier::Conformance,
            Profile::Behavioral,
            Writer::Live,
        ),
        (
            sealed,
            Sealed,
            Frontier::Conformance,
            Profile::Catalog,
            Writer::Live,
        ),
        (
            validated,
            Validated,
            Frontier::Conformance,
            Profile::Catalog,
            Writer::Live,
        ),
        (
            published,
            Published,
            Frontier::Facts,
            Profile::Catalog,
            Writer::Ended,
        ),
        (
            live.generation(),
            Staging,
            Frontier::Facts,
            Profile::Catalog,
            Writer::Live,
        ),
        (
            failed,
            Failed,
            Frontier::Facts,
            Profile::Catalog,
            Writer::Ended,
        ),
    ];
    assert_eq!(rows.len(), expected.len());
    for (g, state, frontier, profile, writer) in expected {
        let r = row(g);
        assert_eq!(
            (
                r.state, r.frontier, r.profile, r.writer, r.selected, r.readers
            ),
            (state, frontier, profile, writer, g == published, 0),
            "{}",
            g.hex()
        );
    }
    let filtered = catalog
        .list(&ListFilter {
            state: Some(Staging),
            frontier: Some(Frontier::Facts),
        })
        .await
        .unwrap();
    assert_eq!(
        filtered.iter().map(|r| r.id).collect::<Vec<_>>(),
        [live.generation()]
    );
    // Details carry what each state has established, and nothing it has not.
    let detail = catalog.show(published).await.unwrap().unwrap();
    assert!(detail.content.is_some() && detail.schedule.is_some() && detail.failure.is_none());
    let (contract, availability) = detail
        .admission
        .expect("a published facts generation shows its admission");
    assert_eq!(contract, facts.contract().digest());
    assert!(
        availability.contains(&(FactFamily::Deployment, Availability::Complete))
            && availability.contains(&(FactFamily::Flow, Availability::NotRequested))
    );
    assert!(
        availability.contains(&(FactFamily::Syntax, Availability::NoScope)),
        "requested but scopeless stays distinct: {availability:?}"
    );
    assert_eq!(
        detail
            .relations
            .iter()
            .find(|(name, _)| name == "provider_coverage")
            .map(|(_, n)| *n),
        Some(3)
    );
    let detail = catalog.show(sealed).await.unwrap().unwrap();
    assert!(detail.content.is_none() && detail.relations.is_empty() && detail.admission.is_none());
    let detail = catalog.show(failed).await.unwrap().unwrap();
    let (from, class, message) = detail
        .failure
        .expect("a failed generation shows its failure");
    assert_eq!(
        (from, class),
        (GenerationState::Sealed, FailureClass::Frontier)
    );
    assert!(message.contains("missing Deployment coverage"), "{message}");
    assert!(
        catalog
            .show(
                lctx_postgres::generations::GenerationId::from_schema(
                    "lctx_g00000000000000000000000000000000"
                )
                .unwrap()
            )
            .await
            .unwrap()
            .is_none()
    );
    // The attempt is lost: its generation is interrupted, not live.
    sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE usename = 'lctx_migrator' AND pid <> pg_backend_pid()")
        .execute(&db.superuser).await.unwrap();
    for _ in 0..50 {
        if catalog
            .show(live.generation())
            .await
            .unwrap()
            .unwrap()
            .summary
            .writer
            == Writer::Interrupted
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(
        catalog
            .show(live.generation())
            .await
            .unwrap()
            .unwrap()
            .summary
            .writer,
        Writer::Interrupted
    );
    // Every attempt lost its connection; the catalog and the store agree on which are interrupted.
    let mut lost = vec![staging, sealed, validated, live.generation()];
    lost.sort_by_key(|g| g.hex());
    assert_eq!(store.interrupted().await.unwrap(), lost);
    let listed: Vec<_> = catalog
        .list(&ListFilter::default())
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.writer == Writer::Interrupted)
        .map(|r| r.id)
        .collect();
    assert_eq!(listed.len(), 4);
    drop((staging_h, sealed_h, validated_h));
}

#[tokio::test]
async fn leases_are_counted_and_the_reader_cannot_mutate_control() {
    let db = DisposableDatabase::start().await;
    let small = Small::new();
    let store = GenerationStore::install(db.owner.clone(), small.model.clone())
        .await
        .unwrap();
    let catalog = GenerationCatalog::new(db.reader.clone());
    let g = small.published(&store, &db.writer, "leased").await;
    let first = store.pin(&db.reader, g, budget()).await.unwrap();
    let second = store.pin(&db.reader, g, budget()).await.unwrap();
    assert_eq!(catalog.show(g).await.unwrap().unwrap().summary.readers, 2);
    first.release().await.unwrap();
    assert_eq!(catalog.show(g).await.unwrap().unwrap().summary.readers, 1);
    second.release().await.unwrap();
    assert_eq!(catalog.show(g).await.unwrap().unwrap().summary.readers, 0);
    // Even with its read-only default lifted, the reader holds no write privilege on control.
    for statement in [
        "UPDATE lctx_model_store.selection SET generation_id = NULL",
        "DELETE FROM lctx_model_store.events",
        "UPDATE lctx_model_store.generations SET state = 'failed'",
        "INSERT INTO lctx_model_store.failures VALUES ('\\x00', 'sealed', 'state', '')",
    ] {
        let mut tx = db.reader.begin().await.unwrap();
        sqlx::query("SET TRANSACTION READ WRITE")
            .execute(&mut *tx)
            .await
            .unwrap();
        let refused = sqlx::query(statement).execute(&mut *tx).await.unwrap_err();
        assert_eq!(
            refused
                .as_database_error()
                .and_then(|e| e.code())
                .as_deref(),
            Some("42501"),
            "{statement}"
        );
    }
}
