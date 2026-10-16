//! Physical RN03 controls on the coordinator-bound native target. These exercise
//! exact CAS pages and real ownership, not semantic installed-view admission.
use super::*;
use futures::{FutureExt, future::BoxFuture};
use lctx_model::domain::{
    completion::{self, Completion, LocalState, RemoteState, ThreadPanic},
    resources::ResourceBudget,
};
use std::{collections::BTreeSet, sync::Arc};

struct Fixture {
    client: Arc<Surreal<Client>>,
    observer: Arc<Surreal<Client>>,
    scope: Arc<RecoveryScope>,
    attempt: ContentHash,
    budget: ResourceBudget,
}

async fn run_case(
    label: &'static str,
    run: impl for<'a> FnOnce(&'a Fixture) -> BoxFuture<'a, Result<(), ModelError>>,
) {
    let config = crate::RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
            .expect("coordinator-bound native receiving fixture"),
    ))
    .unwrap();
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    // Retained before asynchronous construction, including failed construction.
    let factory = RecoveryFactoryOwner::new(config.receipt_binding().unwrap(), budget.clone())
        .unwrap();
    let recovery = RetryCancellation::default();
    let stop = recovery.clone();
    let signals = config.signal_owner(label, move || stop.cancel()).unwrap();
    let result = std::panic::AssertUnwindSafe(async {
        let (client, lease) = crate::reader::connect_owned(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
            &factory,
        )
        .await?;
        let (observer, observer_lease) = crate::reader::connect_owned(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
            &factory,
        )
        .await?;
        let scope = lease.recovery_scope().clone();
        let result = with_recovery_scope(scope.clone(), async {
            let issuance = IssuanceEra::capture(&client).await?;
            let allocation = prepare_attempt_allocation(fresh_identity(label)?, issuance)?;
            let attempt = allocation.attempt;
            lease.register_attempt(attempt, allocation.request)?;
            allocation.issue(&client).await?;
            lease.source_ready()?;
            run(&Fixture { client, observer, scope: scope.clone(), attempt, budget }).await
        })
        .await;
        // Both actual session leases stay borrowed until all case futures end.
        drop(observer_lease);
        drop(lease);
        result
    })
    .catch_unwind()
    .await
    .unwrap_or_else(|panic| Err(ModelError::Cause(Box::new(ThreadPanic::new(label, panic)))));
    let mut terminal = Completion::default();
    // Factory finality owns exact reconciliation, pin borrowers, attempt cleanup
    // and session drainage. A panic or cancellation does not imply remote absence.
    terminal.step("receiving control native owner drainage", factory.drain(&recovery).await);
    if !terminal.failures.is_empty() {
        terminal.local = LocalState::Outstanding;
        terminal.remote = RemoteState::Unknown;
    }
    terminal.step("receiving control recovery signal owner", signals.complete(&terminal).await);
    completion::complete(result, terminal).unwrap();
}

fn external(attempt: ContentHash, index: usize) -> (RecordId, Value) {
    let target = lctx_model::domain::graph::Target::External {
        provider: lctx_model::domain::graph::EntityId(ContentHash::of(b"receiving-control-provider")),
        context: lctx_model::domain::graph::EntityId(attempt),
        name: format!("receiving-{index}"),
        reason: lctx_model::domain::normalized::entities::EntityReason::ProviderExternal,
    };
    let id = crate::reader::target_id(target.clone());
    let mut row = Object::new();
    row.insert("id", id.clone());
    row.insert("canonical", surrealdb::types::Bytes::from(serde_json::to_vec(&target).unwrap()));
    (id, Value::Object(row))
}

async fn source_inventory(
    fixture: &Fixture,
    count: usize,
) -> Result<(Arc<ReaderPin>, Vec<Arc<ExpectedRecord>>), ModelError> {
    let (mut ids, rows): (Vec<_>, Vec<_>) = (0..count)
        .map(|index| external(fixture.attempt, index)).unzip();
    ids.sort();
    let owner = RecordId::new("native_attempt", fixture.attempt.hex());
    // Each installed page earns attempt ownership in the same effect as its rows.
    for window in ImmutableWindows::new(rows, Some(fixture.attempt), None, Some(owner))
        .with_budget(Some(&fixture.budget))
    {
        install_window(&fixture.client, Some(fixture.attempt), window?, Some(&fixture.budget)).await?;
    }
    let source = ReaderPin::acquire(fixture.client.clone(), &[]).await?;
    let owner = RecordId::new("native_pin", source.identity.hex());
    let mut records = Vec::with_capacity(count);
    for ids in ids.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
        hold(&fixture.client, None, owner.clone(), ids.to_vec()).await?;
        let mut observed = inspect_expected_window(&fixture.client, ids, &fixture.budget).await?;
        for id in ids {
            records.push(Arc::new(observed.take_required(id)?));
        }
        observed.finish()?;
    }
    Ok((source, records))
}

async fn holds(fixture: &Fixture, pin: &ReaderPin) -> Result<BTreeSet<RecordId>, ModelError> {
    // The independent, actually owned session remains usable after production stop.
    let mut response = fixture.observer
        .query("SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$owner ORDER BY object")
        .bind(("owner", RecordId::new("native_pin", pin.identity.hex())))
        .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let owner = RecordId::new("native_pin", pin.identity.hex());
    let mut result = BTreeSet::new();
    for row in rows {
        let Some(Value::RecordId(object)) = row.get("object") else {
            return Err(ModelError::Schema("receiving control actual hold object"));
        };
        assert_eq!(row.get("owner"), Some(&Value::RecordId(owner.clone())));
        assert_eq!(row.get("id"), Some(&Value::RecordId(hold_record_id(&owner, object)?)));
        assert!(result.insert(object.clone()), "duplicate actual receiving hold");
    }
    Ok(result)
}

fn inventory(records: &[Arc<ExpectedRecord>]) -> BTreeSet<RecordId> {
    records.iter().map(|record| record.record().clone()).collect()
}

async fn committed_pages(fixture: &Fixture, transferred: &TransferredExpected) -> Result<(), ModelError> {
    let ids = transferred._pages.iter()
        .map(|page| RecordId::new("native_effect", page.request.operation.hex()))
        .collect::<Vec<_>>();
    let mut response = fixture.observer.query("SELECT * FROM $effects")
        .bind(("effects", ids)).await.map_err(ModelError::codec)?
        .check().map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    assert_eq!(rows.len(), transferred._pages.len());
    for page in &transferred._pages {
        let id = Value::RecordId(RecordId::new("native_effect", page.request.operation.hex()));
        let row = rows.iter().find(|row| row.get("id") == Some(&id))
            .expect("every recorded page has its actual native outcome");
        assert_eq!(row.get("committed"), Some(&Value::Bool(true)));
        assert_eq!(row.get("resolved"), Some(&Value::Bool(true)));
        assert_eq!(authorization::hash_field(row, "request")?, page.request.request);
        assert_eq!(authorization::hash_field(row, "generation")?, page.request.issuance.generation);
        assert_eq!(authorization::int_field(row, "era")?, page.request.issuance.era);
        assert_eq!(row.get("owner"), Some(&Value::RecordId(
            RecordId::new("native_pin", transferred.pin.identity.hex()),
        )));
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn native_receiving_all_pages_keep_source_through_handoff_and_earn_independent_owners() {
    run_case("native receiving complete page inventory", |fixture| async move {
        let (source, records) = source_inventory(fixture, crate::loader::NATIVE_WINDOW_ROWS + 1).await?;
        let exact = inventory(&records);
        assert_eq!(holds(fixture, &source).await?, exact);
        let receiver = ReaderPin::acquire(fixture.client.clone(), &[]).await?;
        assert!(holds(fixture, &receiver).await?.is_empty());
        let transferred = transfer_expected_many(&receiver, records.clone(), &fixture.budget).await?;
        assert!(transferred._pages.len() > 1, "inventory must require multiple actual CAS pages");
        complete_transfer_pages(records.len(), &transferred._pages, transferred._pages[0].request.issuance)?;
        committed_pages(fixture, &transferred).await?;
        assert_eq!(inventory(transferred.records()), exact);
        assert!(transferred.records().iter().zip(&records).all(|(a, b)| Arc::ptr_eq(a, b)));
        assert_eq!(holds(fixture, &receiver).await?, exact);
        assert_eq!(holds(fixture, &source).await?, exact, "source holds survive every page and handoff");
        transferred.ensure_live(&fixture.client)?;
        let foreign = transferred.ensure_live(&fixture.observer).unwrap_err();
        assert!(foreign.to_string().contains("native pin foreign client"), "{foreign}");

        source.release().await?;
        assert!(holds(fixture, &source).await?.is_empty());
        let old_source = Arc::downgrade(&source);
        drop(source);
        assert!(old_source.upgrade().is_none(), "complete receiver cannot retain the source pin borrower");
        transferred.ensure_live(&fixture.client)?;
        assert_eq!(holds(fixture, &receiver).await?, exact);
        // Repeated small borrowers preserve this one complete receiving premise.
        for window in transferred.records().chunks(1) {
            transferred.ensure_live(&fixture.client)?;
            assert_eq!(window.len(), 1);
        }

        let independent = ReaderPin::acquire(fixture.client.clone(), &[]).await?;
        assert_ne!(receiver.identity, independent.identity);
        assert!(holds(fixture, &independent).await?.is_empty(), "new owner inherits no holds");
        let independent_transfer = transfer_expected_many(&independent, records, &fixture.budget).await?;
        assert_eq!(holds(fixture, &independent).await?, exact);
        committed_pages(fixture, &independent_transfer).await?;
        let receiver_borrower = Arc::downgrade(&receiver);
        drop(receiver);
        assert!(receiver_borrower.upgrade().is_some(), "complete outcome retains its actual receiver");
        transferred.pin.release().await?;
        assert!(transferred.ensure_live(&fixture.client).is_err());
        independent_transfer.ensure_live(&fixture.client)?;
        assert_eq!(holds(fixture, &independent).await?, exact, "other owner release cannot revoke independent holds");
        drop(transferred);
        assert!(receiver_borrower.upgrade().is_none());
        Ok(())
    }.boxed()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn native_receiving_late_page_refusal_retains_real_prefix_without_complete_capability() {
    run_case("native receiving late exact CAS refusal", |fixture| async move {
        let (source, mut records) = source_inventory(fixture, crate::loader::NATIVE_WINDOW_ROWS + 1).await?;
        let victim = ReaderPin::acquire(fixture.client.clone(), &[]).await?;
        let victim_id = RecordId::new("native_pin", victim.identity.hex());
        source.protect(victim_id.clone()).await?;
        let expected = inspect_expected(&fixture.client, victim_id.clone(), &fixture.budget).await?
            .ok_or(ModelError::Conflict("receiving control observed live pin missing"))?;
        assert!(records.last().unwrap().record() < expected.record(), "invalidating row must follow the first full page");
        records.push(Arc::new(expected));
        let complete = inventory(&records);
        assert_eq!(holds(fixture, &source).await?, complete);
        // An ordinary owner lifecycle transition invalidates this observed physical
        // row/guard. No canonical row or incarnation is forged or directly edited.
        victim.release().await?;
        let receiver = ReaderPin::acquire(fixture.client.clone(), &[]).await?;
        let refused = match transfer_expected_many(&receiver, records.clone(), &fixture.budget).await {
            Ok(_) => panic!("late stale page cannot yield complete receiving authority"),
            Err(error) => error,
        };
        assert!(refused.to_string().contains("selected reader exact transfer changed"), "{refused}");
        assert!(refused.permits_storage_cleanup(), "acknowledged CAS refusal must retain exact finality: {refused}");
        let prefix = holds(fixture, &receiver).await?;
        assert_eq!(prefix, inventory(&records[..crate::loader::NATIVE_WINDOW_ROWS]),
            "first page stays committed while every hold in the failed last page rolls back");
        assert!(!prefix.contains(&victim_id));
        assert_eq!(holds(fixture, &source).await?, complete, "failure cannot release source obligations");
        assert_eq!(fixture.scope.pending_count()?, 0, "conclusive failed page settles its exact recovery request");
        // Explicit actual pin release, not a failed receiver's Drop, settles prefix holds.
        receiver.release().await?;
        assert!(holds(fixture, &receiver).await?.is_empty());
        assert_eq!(holds(fixture, &source).await?, complete);
        Ok(())
    }.boxed()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn native_receiving_cancelled_issuance_preserves_source_and_actual_pending_owner() {
    run_case("native receiving cancelled issuance", |fixture| async move {
        let (source, records) = source_inventory(fixture, 1).await?;
        let exact = inventory(&records);
        let receiver = ReaderPin::acquire(fixture.client.clone(), &[]).await?;
        fixture.scope.cancellation().cancel();
        let error = match transfer_expected_many(&receiver, records, &fixture.budget).await {
            Ok(_) => panic!("cancelled issuance cannot produce a complete receiver"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("cancelled"), "{error}");
        assert!(fixture.scope.cancellation().stopped());
        let pending = fixture.scope.pending_count()?;
        assert!(holds(fixture, &receiver).await?.is_empty());
        assert_eq!(holds(fixture, &source).await?, exact);
        assert_eq!(fixture.scope.pending_count()?, pending, "observing holds cannot settle pending native actions");
        if pending != 0 {
            assert!(!error.permits_storage_cleanup(), "unsettled original ownership cannot become confirmed cleanup: {error}");
        }
        // No token reset or detached Drop cleanup. The enclosing factory drains
        // under its separately signalled recovery admission after borrowers end.
        Ok(())
    }.boxed()).await;
}
