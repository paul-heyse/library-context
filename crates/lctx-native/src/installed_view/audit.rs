//! Independent exact selected mapping and dependency inspection for native installation.
//! This prepares a writer's audit; it does not grant semantic admission or completion.
use super::ContributorRealization;
use crate::{
    NativeReader,
    completed_rows::{canonical_backing_endpoints, validate_state_row},
    control::ExpectedRecord,
};
use futures::{FutureExt, StreamExt};
#[cfg(test)]
use lctx_model::domain::recovery_closure::{RecoveryClosure, RecoveryTraversal};
use lctx_model::domain::{
    self as d, ContentHash, ModelError,
    charged::StateCharge,
    completed::{CompletedContribution, CompletedView},
    recovery_closure::{
        SharedRecoveryContribution, SharedRecoveryIngredient, SharedRecoveryTraversal,
    },
    resources::ResourceBudget,
};
use std::collections::{BTreeMap, BTreeSet};
use surrealdb::types::{RecordId, Value, Variables};
#[cfg(test)]
tokio::task_local! { static LATE_MEMBERSHIP_FAILURE:bool; }
#[cfg(test)]
#[derive(Clone)]
struct BlobObservationBarrier {
    observed: std::sync::Arc<tokio::sync::Barrier>,
    resume: std::sync::Arc<tokio::sync::Barrier>,
}
#[cfg(test)]
tokio::task_local! {static BLOB_OBSERVATION_BARRIER:BlobObservationBarrier;}

/// Operation-local independent observations beneath a real native pin. This is
/// neither portable proof nor semantic admission; each root still earns its own
/// descriptor, aliases, mappings, inventories and final checked native effect.
type WorkFuture = futures::future::Shared<
    futures::future::BoxFuture<'static, Result<(), std::sync::Arc<ModelError>>>,
>;
#[derive(Clone)]
struct WorkFlight {
    token: u64,
    future: WorkFuture,
    _charge: std::sync::Arc<Box<dyn d::resources::Reservation>>,
}
struct SharedMap<K, V>(std::sync::Arc<std::sync::Mutex<BTreeMap<K, std::sync::Arc<V>>>>);
impl<K, V> Clone for SharedMap<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<K: Ord, V> SharedMap<K, V> {
    fn new() -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(BTreeMap::new())))
    }
    fn get(&self, key: &K) -> Option<std::sync::Arc<V>> {
        self.0.lock().unwrap().get(key).cloned()
    }
    fn insert(&self, key: K, value: V) -> Option<std::sync::Arc<V>> {
        self.0
            .lock()
            .unwrap()
            .insert(key, std::sync::Arc::new(value))
    }
    fn insert_arc(&self, key: K, value: std::sync::Arc<V>) -> Option<std::sync::Arc<V>> {
        self.0.lock().unwrap().insert(key, value)
    }
    fn get_or_try_insert(
        &self,
        key: K,
        make: impl FnOnce() -> Result<V, ModelError>,
    ) -> Result<std::sync::Arc<V>, ModelError> {
        let mut values = self.0.lock().unwrap();
        if let Some(value) = values.get(&key) {
            return Ok(value.clone());
        }
        let value = std::sync::Arc::new(make()?);
        values.insert(key, value.clone());
        Ok(value)
    }
    fn contains_key(&self, key: &K) -> bool {
        self.0.lock().unwrap().contains_key(key)
    }
    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
}
#[derive(Clone)]
struct SharedCharge(std::sync::Arc<std::sync::Mutex<StateCharge>>);
impl SharedCharge {
    fn grow(&self, bytes: usize) -> Result<(), ModelError> {
        self.0.lock().unwrap().grow(bytes)
    }
}
struct WorkOwner {
    flights: std::sync::Mutex<BTreeMap<(u8, ContentHash), WorkFlight>>,
    workers: std::sync::Mutex<Vec<PreparationWorker>>,
    draining: tokio::sync::Mutex<WorkerDrain>,
    next: std::sync::atomic::AtomicU64,
    accepting: std::sync::atomic::AtomicBool,
    close_gate: tokio::sync::Mutex<()>,
    context_checked: std::sync::atomic::AtomicBool,
    context_gate: tokio::sync::Mutex<()>,
    blocking: std::sync::Mutex<
        Vec<(
            crate::acknowledged_candidates::BlockingTerminal,
            Box<dyn d::resources::Reservation>,
        )>,
    >,
}
type PreparationWorker = (
    u64,
    tokio::task::JoinHandle<()>,
    std::sync::Arc<Box<dyn d::resources::Reservation>>,
);
#[derive(Default)]
struct WorkerDrain {
    workers: Vec<PreparationWorker>,
    terminal: d::completion::Completion,
    blocking: Vec<(
        crate::acknowledged_candidates::BlockingTerminal,
        Box<dyn d::resources::Reservation>,
    )>,
}
/// Bounded operation-owned waiters, not another executor. `run_work` retains the
/// actual worker, pin and capture charges if this window's waiter disappears.
/// FuturesOrdered polls peers together but delivers failures in contributor order.
struct ContributorWindow<'a> {
    pending:
        futures::stream::FuturesOrdered<futures::future::BoxFuture<'a, Result<(), ModelError>>>,
    charge: StateCharge,
    budget: ResourceBudget,
}
impl<'a> ContributorWindow<'a> {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            pending: futures::stream::FuturesOrdered::new(),
            charge: StateCharge::new(budget, "contributor window owned waiters"),
            budget: budget.clone(),
        }
    }
    async fn push<F>(&mut self, future: F) -> Result<(), ModelError>
    where
        F: std::future::Future<Output = Result<(), ModelError>> + Send + 'a,
    {
        if self.pending.len() >= crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Limit {
                owner: "contributor window",
                limit: "waiters",
                observed: self.pending.len() + 1,
                bound: crate::loader::NATIVE_WINDOW_ROWS,
            });
        }
        // The concrete future includes its owned closure/captures. ExactWork and
        // prepared rows retain their separate deep charges; this covers the box,
        // ordered delivery node and the small terminal result slot as well.
        let bytes = std::mem::size_of::<F>().saturating_add(1024);
        match self.charge.grow(bytes) {
            Err(ModelError::Resource { .. }) if !self.pending.is_empty() => {
                self.drain().await?;
                self.charge.grow(bytes)?;
            }
            result => result?,
        }
        self.pending.push_back(future.boxed());
        Ok(())
    }
    async fn exact_capture(
        &mut self,
        expected: &ExpectedRecord,
    ) -> Result<(ContentHash, ExactWork), ModelError> {
        match exact_work(expected, &self.budget) {
            Err(ModelError::Resource { .. }) if !self.pending.is_empty() => {
                // Only this known pre-launch capture reservation is retried.
                // Dynamic native/kernel refusals surface through drain unchanged.
                self.drain().await?;
                exact_work(expected, &self.budget)
            }
            result => result,
        }
    }
    async fn drain(&mut self) -> Result<(), ModelError> {
        let mut terminal = d::completion::Completion::default();
        while let Some(result) = self.pending.next().await {
            terminal.step("independent contributor preparation", result);
        }
        self.pending = futures::stream::FuturesOrdered::new();
        self.charge = StateCharge::new(&self.budget, "contributor window owned waiters");
        d::completion::complete(Ok(()), terminal)
    }
    async fn finish(mut self) -> Result<(), ModelError> {
        self.drain().await
    }
}
#[derive(Clone)]
pub struct InstallationAudit {
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    pin: std::sync::Arc<crate::control::ReaderPin>,
    contributors: SharedMap<RecordId, ExpectedRecord>,
    inspected_contributors: SharedMap<ContentHash, ObservedContributor>,
    inspected_views: SharedMap<ContentHash, ObservedView>,
    inspected_holds: SharedMap<(RecordId, RecordId), ()>,
    recovery_contributors:
        SharedMap<ContentHash, SharedRecoveryContribution<ObservedView, ContributionDescriptor>>,
    recovery_views:
        SharedMap<ContentHash, SharedRecoveryIngredient<ObservedView, ContributionDescriptor>>,
    logical_contributors: SharedMap<RecordId, LogicalContributorRows>,
    membership_contributors: SharedMap<RecordId, AuditedContributorMembership>,
    family_roots: SharedMap<(usize, ContentHash), d::completed_identity::CompletedFamilyRoot>,
    selected_members: SharedMap<ContentHash, PreparedSelectedMembers>,
    aliases: SharedMap<(), ScopedAliasRows>,
    local_aliases: SharedMap<
        RecordId,
        (
            crate::ordered_rows::PreparedRows,
            crate::ordered_rows::PreparedRows,
        ),
    >,
    blobs: SharedMap<RecordId, PreparedBlobObservation>,
    budget: ResourceBudget,
    charge: SharedCharge,
    blob_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    contribution_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    logical_contribution_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    alias_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    #[cfg(test)]
    context_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    context: (ContentHash, i64),
    work: std::sync::Arc<WorkOwner>,
    worker: bool,
}
/// A post-pin native context observation tied to its still-open actual source.
/// Copies retain no authority after pin release, source close or foreign reuse.
#[derive(Clone)]
pub(super) struct CheckedContextLifetime {
    pin: std::sync::Weak<crate::control::ReaderPin>,
    client: std::sync::Weak<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    work: std::sync::Weak<WorkOwner>,
    closed: std::sync::Weak<std::sync::atomic::AtomicBool>,
    budget: ResourceBudget,
}
impl CheckedContextLifetime {
    pub(super) fn check(
        &self,
        reader: &NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let pin = self
            .pin
            .upgrade()
            .ok_or(ModelError::Conflict("checked context source released"))?;
        let client = self
            .client
            .upgrade()
            .ok_or(ModelError::Conflict("checked context session released"))?;
        let work = self
            .work
            .upgrade()
            .ok_or(ModelError::Conflict("checked context operation released"))?;
        let closed = self
            .closed
            .upgrade()
            .ok_or(ModelError::Conflict("checked context source released"))?;
        pin.ensure_live()?;
        if !std::ptr::eq(client.as_ref(), reader.client()) {
            return Err(ModelError::Conflict(
                "checked context foreign native session",
            ));
        }
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict(
                "checked context foreign resource owner",
            ));
        }
        if closed.load(std::sync::atomic::Ordering::Acquire)
            || !work.accepting.load(std::sync::atomic::Ordering::Acquire)
            || !work
                .context_checked
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(ModelError::Conflict("checked context source closed"));
        }
        Ok(())
    }
}
/// Borrowed operation source for boundary construction. This cannot outlive its actual audit;
/// serialized descriptors and independently owned audits cannot supply its observations.
pub struct ProtectedInspectionSource<'a> {
    audit: &'a InstallationAudit,
}
impl ProtectedInspectionSource<'_> {
    pub(crate) fn audit(&self) -> &InstallationAudit {
        self.audit
    }
    pub(crate) fn inspection_scope(
        &self,
        reader: &NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<ClosureInspectionScope, ModelError> {
        self.audit.check_source_local(reader, budget)?;
        let mut scope = ClosureInspectionScope::for_operation(self.audit, budget)?;
        scope.source = true;
        Ok(scope)
    }
}
impl InstallationAudit {
    pub async fn new(
        client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let reader = NativeReader::private(client.clone()).with_budget(budget);
        let context = async {
            let rows: Vec<surrealdb::types::Object> = reader
                .query_native(
                    "SELECT generation,validity_epoch FROM native_installation:current",
                    Variables::new(),
                )
                .await?;
            let [row] = rows.as_slice() else {
                return Err(ModelError::Conflict(
                    "installation audit context cardinality",
                ));
            };
            let Some(Value::String(generation)) = row.get("generation") else {
                return Err(ModelError::Schema("installation audit generation"));
            };
            let generation = ContentHash(
                hex::decode(generation)
                    .map_err(ModelError::codec)?
                    .try_into()
                    .map_err(|_| ModelError::Schema("installation audit generation width"))?,
            );
            let Some(Value::Number(surrealdb::types::Number::Int(epoch))) =
                row.get("validity_epoch")
            else {
                return Err(ModelError::Schema("installation audit validity epoch"));
            };
            if *epoch <= 0 {
                return Err(ModelError::Conflict("installation audit inactive epoch"));
            }
            Ok((generation, *epoch))
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "installation audit context capture reader close",
            reader.close().await,
        );
        let context = d::completion::complete(context, terminal)?;
        Ok(Self {
            context,
            pin: crate::control::ReaderPin::acquire(client.clone(), &[]).await?,
            client,
            contributors: SharedMap::new(),
            inspected_contributors: SharedMap::new(),
            inspected_views: SharedMap::new(),
            inspected_holds: SharedMap::new(),
            recovery_contributors: SharedMap::new(),
            recovery_views: SharedMap::new(),
            logical_contributors: SharedMap::new(),
            membership_contributors: SharedMap::new(),
            family_roots: SharedMap::new(),
            selected_members: SharedMap::new(),
            aliases: SharedMap::new(),
            local_aliases: SharedMap::new(),
            blobs: SharedMap::new(),
            budget: budget.clone(),
            charge: SharedCharge(std::sync::Arc::new(std::sync::Mutex::new(
                StateCharge::new(budget, "installation audit retained observations"),
            ))),
            blob_observations: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            contribution_observations: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            logical_contribution_observations: std::sync::Arc::new(
                std::sync::atomic::AtomicU64::new(0),
            ),
            alias_observations: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            #[cfg(test)]
            context_observations: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            closed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            work: std::sync::Arc::new(WorkOwner {
                flights: std::sync::Mutex::new(BTreeMap::new()),
                workers: std::sync::Mutex::new(Vec::new()),
                draining: tokio::sync::Mutex::new(WorkerDrain::default()),
                next: std::sync::atomic::AtomicU64::new(1),
                accepting: std::sync::atomic::AtomicBool::new(true),
                close_gate: tokio::sync::Mutex::new(()),
                context_checked: std::sync::atomic::AtomicBool::new(false),
                context_gate: tokio::sync::Mutex::new(()),
                blocking: std::sync::Mutex::new(Vec::new()),
            }),
            worker: false,
        })
    }
    fn check(&self, reader: &NativeReader<()>, budget: &ResourceBudget) -> Result<(), ModelError> {
        self.pin.ensure_live()?;
        if !std::ptr::eq(self.client.as_ref(), reader.client()) {
            return Err(ModelError::Conflict(
                "installation audit foreign native session",
            ));
        }
        if !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict(
                "installation audit foreign resource owner",
            ));
        }
        if self.closed.load(std::sync::atomic::Ordering::Acquire)
            || (!self.worker
                && !self
                    .work
                    .accepting
                    .load(std::sync::atomic::Ordering::Acquire))
        {
            return Err(ModelError::Conflict("installation audit closed"));
        }
        Ok(())
    }
    fn check_source_local(
        &self,
        reader: &NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check(reader, budget)?;
        if self.closed.load(std::sync::atomic::Ordering::Acquire)
            || !self
                .work
                .accepting
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(ModelError::Conflict("protected inspection source closed"));
        }
        Ok(())
    }
    /// Explicitly borrow live exact observations from this operation's real source pin.
    pub async fn protected_source<'a>(
        &'a self,
        client: &surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>,
        budget: &ResourceBudget,
    ) -> Result<ProtectedInspectionSource<'a>, ModelError> {
        if !std::ptr::eq(self.client.as_ref(), client) {
            return Err(ModelError::Conflict(
                "installation audit foreign native session",
            ));
        }
        let reader = NativeReader::private(self.client.clone()).with_budget(budget);
        let result: Result<ProtectedInspectionSource<'a>, ModelError> = async {
            self.check_source_local(&reader, budget)?;
            self.ensure_context(&reader, budget).await?;
            self.check_source_local(&reader, budget)?;
            Ok(ProtectedInspectionSource { audit: self })
        }
        .await;
        let closed = reader.close().await;
        if closed.is_err()
            || result
                .as_ref()
                .err()
                .is_some_and(|error| !error.permits_storage_cleanup())
        {
            self.retain_unknown();
        }
        let mut terminal = d::completion::Completion::default();
        terminal.step("protected inspection source capture reader close", closed);
        d::completion::complete(result, terminal)
    }
    pub(crate) async fn ensure_context(
        &self,
        reader: &NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check(reader, budget)?;
        if self
            .work
            .context_checked
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(());
        }
        // The constructor observed context BEFORE acquiring its native pin. Only
        // this post-pin observation earns reuse, coalesced across actual workers.
        let _context = self.work.context_gate.lock().await;
        self.check(reader, budget)?;
        if self
            .work
            .context_checked
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(());
        }
        let _charge = budget.reserve("installation audit context check", 4096)?;
        #[cfg(test)]
        self.context_observations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rows: Vec<surrealdb::types::Object> = reader
            .query_native(
                "SELECT generation,validity_epoch FROM native_installation:current",
                Variables::new(),
            )
            .await?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict(
                "installation audit current context cardinality",
            ));
        };
        if row.get("generation") != Some(&Value::String(self.context.0.hex()))
            || row.get("validity_epoch")
                != Some(&Value::Number(surrealdb::types::Number::Int(
                    self.context.1,
                )))
        {
            return Err(ModelError::Conflict(
                "installation audit validity context changed",
            ));
        }
        self.check(reader, budget)?;
        self.work
            .context_checked
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
    pub(super) fn checked_context_lifetime(
        &self,
        reader: &NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<CheckedContextLifetime, ModelError> {
        self.check_source_local(reader, budget)?;
        if !self
            .work
            .context_checked
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(ModelError::Conflict(
                "installation audit context not earned",
            ));
        }
        Ok(CheckedContextLifetime {
            pin: std::sync::Arc::downgrade(&self.pin),
            client: std::sync::Arc::downgrade(&self.client),
            work: std::sync::Arc::downgrade(&self.work),
            closed: std::sync::Arc::downgrade(&self.closed),
            budget: self.budget.clone(),
        })
    }
    async fn run_work(
        &self,
        key: (u8, ContentHash),
        build: impl FnOnce(Self) -> futures::future::BoxFuture<'static, Result<(), ModelError>>
        + Send
        + 'static,
    ) -> Result<(), ModelError> {
        self.run_work_many(vec![key], move |owner, _| build(owner))
            .await
    }
    async fn run_work_many(
        &self,
        keys: Vec<(u8, ContentHash)>,
        build: impl FnOnce(
            Self,
            Vec<(u8, ContentHash)>,
        ) -> futures::future::BoxFuture<'static, Result<(), ModelError>>
        + Send
        + 'static,
    ) -> Result<(), ModelError> {
        self.pin.ensure_live()?;
        // Cancelled waiters leave their independently owned workers behind. Reap only
        // already terminal handles before admitting another bounded preparation.
        self.reap_terminal_workers(Ok(()), &BTreeSet::new()).await?;
        if keys.len() > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Limit {
                owner: "installation preparation",
                limit: "window keys",
                observed: keys.len(),
                bound: crate::loader::NATIVE_WINDOW_ROWS,
            });
        }
        let futures = {
            let mut registry = self.work.flights.lock().unwrap();
            if !self.worker
                && !self
                    .work
                    .accepting
                    .load(std::sync::atomic::Ordering::Acquire)
            {
                return Err(ModelError::Conflict(
                    "installation audit closed to new preparation",
                ));
            }
            let mut futures = BTreeMap::new();
            let mut missing = BTreeSet::new();
            for key in keys {
                if let Some(flight) = registry.get(&key) {
                    futures.insert(flight.token, flight.future.clone());
                } else {
                    missing.insert(key);
                }
            }
            if !missing.is_empty() {
                let charge = std::sync::Arc::new(self.budget.reserve(
                    "installation preparation flight metadata",
                    4096usize.saturating_mul(missing.len()),
                )?);
                {
                    let mut workers = self.work.workers.lock().unwrap();
                    if workers.len() == workers.capacity() {
                        let capacity = workers.capacity().saturating_mul(2).max(1);
                        self.charge.grow(
                            capacity
                                .saturating_sub(workers.capacity())
                                .saturating_mul(std::mem::size_of::<PreparationWorker>()),
                        )?;
                        let additional = capacity - workers.len();
                        workers.reserve_exact(additional);
                    }
                }
                let token = self
                    .work
                    .next
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let (send, receive) =
                    tokio::sync::oneshot::channel::<Result<(), std::sync::Arc<ModelError>>>();
                let pin = self.pin.clone();
                let receiving_charge = charge.clone();
                let future = async move {
                    let _charge = receiving_charge;
                    receive.await.unwrap_or_else(|_| {
                        pin.retain_unknown();
                        Err(std::sync::Arc::new(ModelError::infrastructure(
                            d::Infrastructure::Unconfirmed,
                            "installation preparation worker failed before terminal delivery",
                        )))
                    })
                }
                .boxed()
                .shared();
                for key in &missing {
                    registry.insert(
                        *key,
                        WorkFlight {
                            token,
                            future: future.clone(),
                            _charge: charge.clone(),
                        },
                    );
                }
                futures.insert(token, future.clone());
                let keys = missing.into_iter().collect::<Vec<_>>();
                #[cfg(test)]
                let late = LATE_MEMBERSHIP_FAILURE
                    .try_with(|enabled| *enabled)
                    .unwrap_or(false);
                #[cfg(test)]
                let barrier = BLOB_OBSERVATION_BARRIER.try_with(Clone::clone).ok();
                let mut owner = self.clone();
                owner.worker = true;
                // Tokio task locals do not cross spawn. The preparation belongs
                // to this retained pin, not to whichever waiter observes it.
                let recovery = owner.pin.recovery_scope();
                let worker_charge = charge.clone();
                let work = async move {
                    let _charge = charge;
                    struct Terminal {
                        pin: std::sync::Arc<crate::control::ReaderPin>,
                        confirmed: bool,
                    }
                    impl Drop for Terminal {
                        fn drop(&mut self) {
                            if !self.confirmed {
                                self.pin.retain_unknown();
                            }
                        }
                    }
                    let mut terminal = Terminal {
                        pin: owner.pin.clone(),
                        confirmed: false,
                    };
                    #[cfg(not(test))]
                    let result = build(owner.clone(), keys.clone()).await;
                    #[cfg(test)]
                    let result = LATE_MEMBERSHIP_FAILURE
                        .scope(late, async {
                            if let Some(barrier) = barrier {
                                BLOB_OBSERVATION_BARRIER
                                    .scope(barrier, build(owner.clone(), keys.clone()))
                                    .await
                            } else {
                                build(owner.clone(), keys.clone()).await
                            }
                        })
                        .await;
                    let result = result.map_err(std::sync::Arc::new);
                    terminal.confirmed = result
                        .as_ref()
                        .err()
                        .is_none_or(|error| error.permits_storage_cleanup());
                    if result.is_err() {
                        let mut registry = owner.work.flights.lock().unwrap();
                        for key in &keys {
                            if registry.get(key).is_some_and(|entry| entry.token == token) {
                                registry.remove(key);
                            }
                        }
                    }
                    let _ = send.send(result);
                };
                let task = tokio::spawn(async move {
                    match recovery {
                        Some(scope) => crate::control::with_recovery_scope(scope, work).await,
                        None => work.await,
                    }
                });
                self.work
                    .workers
                    .lock()
                    .unwrap()
                    .push((token, task, worker_charge));
            }
            futures
        };
        if futures.len() == 1 {
            let (token, future) = futures.into_iter().next().unwrap();
            let result = future.await.map_err(ModelError::SharedCause);
            let failed = if result.is_err() {
                BTreeSet::from([token])
            } else {
                BTreeSet::new()
            };
            return self.reap_terminal_workers(result, &failed).await;
        }
        let mut terminal = d::completion::Completion::default();
        let mut failed = BTreeSet::new();
        for (token, future) in futures {
            let result = future.await.map_err(ModelError::SharedCause);
            if result.is_err() {
                failed.insert(token);
            }
            terminal.step("installation preparation set flight", result);
        }
        self.reap_terminal_workers(d::completion::complete(Ok(()), terminal), &failed)
            .await
    }
    async fn reap_terminal_workers(
        &self,
        result: Result<(), ModelError>,
        failed: &BTreeSet<u64>,
    ) -> Result<(), ModelError> {
        let current = tokio::task::try_id();
        let completed = {
            let mut workers = self.work.workers.lock().unwrap();
            let mut completed = Vec::new();
            let mut index = 0;
            while index < workers.len() {
                let (token, worker, _) = &workers[index];
                if current != Some(worker.id()) && (worker.is_finished() || failed.contains(token))
                {
                    completed.push(workers.swap_remove(index));
                } else {
                    index += 1;
                }
            }
            completed
        };
        let mut terminal = d::completion::Completion::default();
        for (_, worker, _charge) in completed {
            if let Err(error) = worker.await {
                self.retain_unknown();
                terminal.remote = d::completion::RemoteState::Unknown;
                terminal.step(
                    "installation terminal worker join",
                    Err(ModelError::infrastructure(
                        d::Infrastructure::Unconfirmed,
                        format!("installation preparation worker did not reach terminal: {error}"),
                    )),
                );
            }
        }
        d::completion::complete(result, terminal)
    }
    pub fn shares_pool(&self, budget: &ResourceBudget) -> bool {
        self.budget.shares_pool(budget)
    }
    pub async fn close_drained(&self) -> Result<(), ModelError> {
        self.close().await
    }
    fn blocking_observer(
        &self,
    ) -> Result<
        impl FnOnce(crate::acknowledged_candidates::BlockingTerminal) + Send + 'static,
        ModelError,
    > {
        self.pin.ensure_live()?;
        let charge = self
            .budget
            .reserve("installation blocking terminal ownership", 1024)?;
        let work = self.work.clone();
        // Retain each admitted record and its charge until close drains the registry;
        // pruning only entries would leave the Vec's retained capacity uncharged.
        Ok(move |terminal| work.blocking.lock().unwrap().push((terminal, charge)))
    }
    async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(&ResourceBudget) -> Result<T, ModelError> + Send + 'static,
    ) -> Result<T, ModelError> {
        // External callers register beneath the close gate. Accepted audit workers
        // already keep close waiting and must not acquire their parent's gate.
        let _admission = if self.worker {
            None
        } else {
            Some(self.work.close_gate.lock().await)
        };
        self.pin.ensure_live()?;
        if self.closed.load(std::sync::atomic::Ordering::Acquire)
            || (!self.worker
                && !self
                    .work
                    .accepting
                    .load(std::sync::atomic::Ordering::Acquire))
        {
            return Err(ModelError::Conflict(
                "installation audit closed to blocking preparation",
            ));
        }
        let input = self.budget.reserve(
            "installation blocking unit inputs",
            std::mem::size_of_val(&work).saturating_add(1024),
        )?;
        crate::acknowledged_candidates::run_registered(
            &self.budget,
            self.clone(),
            self.blocking_observer()?,
            input,
            work,
        )
        .await
    }
    pub(crate) async fn prepare_member_points(
        &self,
        rows: crate::ordered_rows::PreparedRows,
    ) -> Result<crate::ordered_rows::PreparedPointRows, ModelError> {
        self.blocking(move |budget| rows.point_index(budget)).await
    }
    async fn drain_workers(&self) -> Result<(), ModelError> {
        {
            let _registry = self.work.flights.lock().unwrap();
            self.work
                .accepting
                .store(false, std::sync::atomic::Ordering::Release);
        }
        // The queue belongs to WorkOwner, not to this cancellable close future.
        // Await by reference and remove only joined handles, so a later close can
        // resume the exact same workers after its predecessor is dropped.
        let mut pending = self.work.draining.lock().await;
        loop {
            if pending.workers.is_empty() {
                std::mem::swap(
                    &mut pending.workers,
                    &mut *self.work.workers.lock().unwrap(),
                );
            }
            if pending.workers.is_empty() {
                break;
            }
            while let Some((_, worker, _)) = pending.workers.last_mut() {
                let joined = worker.await;
                let _joined = pending.workers.pop();
                if let Err(error) = joined {
                    self.retain_unknown();
                    pending.terminal.remote = d::completion::RemoteState::Unknown;
                    pending.terminal.step(
                        "installation audit worker join",
                        Err(ModelError::infrastructure(
                            d::Infrastructure::Unconfirmed,
                            format!("installation audit worker did not reach terminal: {error}"),
                        )),
                    );
                }
            }
        }
        loop {
            if pending.blocking.is_empty() {
                std::mem::swap(
                    &mut pending.blocking,
                    &mut *self.work.blocking.lock().unwrap(),
                );
            }
            let Some((terminal, _)) = pending.blocking.last_mut() else {
                break;
            };
            let result = terminal.clone().await.map_err(ModelError::SharedCause);
            pending.blocking.pop();
            pending
                .terminal
                .step("installation blocking preparation terminal", result);
        }
        d::completion::complete(Ok(()), std::mem::take(&mut pending.terminal))
    }
    pub async fn close(&self) -> Result<(), ModelError> {
        self.close_releasing(self.pin.release()).await
    }
    async fn close_releasing(
        &self,
        release: impl std::future::Future<Output = Result<(), ModelError>>,
    ) -> Result<(), ModelError> {
        let _close = self.work.close_gate.lock().await;
        let drained = self.drain_workers().await;
        if drained
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            self.retain_unknown();
            return drained;
        }
        let released = release.await;
        if released.is_ok() {
            self.closed
                .store(true, std::sync::atomic::Ordering::Release);
        }
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "installation audit pin release after worker drainage",
            released,
        );
        d::completion::complete(drained, terminal)
    }
    pub(crate) fn borrow_pin(
        &self,
    ) -> Result<std::sync::Arc<crate::control::ReaderPin>, ModelError> {
        self.pin.ensure_live()?;
        Ok(self.pin.clone())
    }
    pub(crate) fn pin_identity(&self) -> ContentHash {
        self.pin.identity
    }
    pub(crate) fn detach_borrowed(&self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Release);
    }
    pub fn retain_unknown(&self) {
        self.pin.retain_unknown();
    }
    /// Structural observation counts; these are not timing or throughput claims.
    pub fn observations(&self) -> (u64, u64) {
        (
            self.contribution_observations
                .load(std::sync::atomic::Ordering::Relaxed),
            self.blob_observations
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }
    /// Full contributor normalization preparations, independent of root hashes.
    pub fn logical_contribution_observations(&self) -> u64 {
        self.logical_contribution_observations
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    /// Complete local inventory beneath the caller's independently protected contributor.
    pub(crate) async fn local_alias_run(
        &self,
        reader: &NativeReader<()>,
        expected: &ExpectedRecord,
        budget: &ResourceBudget,
    ) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
        self.ensure_context(reader, budget).await?;
        if let Some((_, aliases)) =
            prepare_local_aliases(reader, expected, budget, self, false).await?
        {
            return Ok(aliases);
        }
        self.local_aliases
            .get(expected.record())
            .map(|entry| entry.1.clone())
            .ok_or(ModelError::Conflict(
                "complete local alias preparation absent",
            ))
    }
    pub fn alias_observations(&self) -> u64 {
        self.alias_observations
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
#[derive(Clone)]
pub(crate) struct LogicalContributorRows {
    contribution: crate::ordered_rows::PreparedRows,
    // The complete canonical RecordId::Ord run remains the six-family hash input.
    memberships: crate::ordered_rows::PreparedRows,
    backing: crate::ordered_rows::PreparedRows,
}
struct AuditedContributorMembership {
    // Actual fused locators remain protected preparation; logical family runs are separate.
    memberships: crate::ordered_rows::PreparedRows,
    relation_memberships: std::sync::Arc<PreparedMembershipRelations>,
    _charge: StateCharge,
}
/// A bounded admitted transfer feeds the existing acknowledged sorting owner.
struct AuditRows {
    sort: crate::acknowledged_candidates::AsyncPhysicalSort,
    rows: Vec<Value>,
    transfer: Box<dyn d::resources::Reservation>,
    budget: ResourceBudget,
}
impl AuditRows {
    async fn new(
        audit: &InstallationAudit,
        budget: &ResourceBudget,
        row_bytes: usize,
    ) -> Result<Self, ModelError> {
        if !audit.shares_pool(budget) {
            return Err(ModelError::Conflict("audit sort foreign resource owner"));
        }
        let _admission = if audit.worker {
            None
        } else {
            Some(audit.work.close_gate.lock().await)
        };
        audit.pin.ensure_live()?;
        if audit.closed.load(std::sync::atomic::Ordering::Acquire)
            || (!audit.worker
                && !audit
                    .work
                    .accepting
                    .load(std::sync::atomic::Ordering::Acquire))
        {
            return Err(ModelError::Conflict(
                "installation audit closed to sort preparation",
            ));
        }
        let transfer = budget.reserve("audit sort batch", 0)?;
        let sort =
            crate::acknowledged_candidates::AsyncPhysicalSort::new_registered_with_row_bytes(
                budget,
                audit.clone(),
                audit.blocking_observer()?,
                row_bytes,
            )
            .await?;
        Ok(Self {
            sort,
            rows: Vec::new(),
            transfer,
            budget: budget.clone(),
        })
    }
    async fn push(&mut self, row: Value) -> Result<(), ModelError> {
        let bytes = crate::acknowledged_candidates::physical_bytes(&row)?;
        let capacity = if self.rows.len() == self.rows.capacity() {
            self.rows.capacity().saturating_mul(2).max(1)
        } else {
            self.rows.capacity()
        };
        self.transfer.try_resize(
            self.transfer.size().saturating_add(bytes).saturating_add(
                capacity
                    .saturating_sub(self.rows.capacity())
                    .saturating_mul(std::mem::size_of::<Value>()),
            ),
        )?;
        if capacity > self.rows.capacity() {
            self.rows.reserve_exact(capacity - self.rows.len());
        }
        self.rows.push(row);
        if self.rows.len() >= crate::loader::NATIVE_WINDOW_ROWS
            || self.transfer.size() >= d::resources::TRANSFER_BYTES
        {
            self.flush().await?;
        }
        Ok(())
    }
    async fn push_owned(
        &mut self,
        row: (Value, Box<dyn d::resources::Reservation>),
    ) -> Result<(), ModelError> {
        self.flush().await?;
        let handles = self.budget.reserve(
            "audit owned logical row handle",
            std::mem::size_of_val(&row),
        )?;
        self.sort
            .push_owned_batch_reserved(vec![row], handles)
            .await
    }
    async fn flush(&mut self) -> Result<(), ModelError> {
        if self.rows.is_empty() {
            return Ok(());
        }
        let rows = std::mem::take(&mut self.rows);
        let transfer = std::mem::replace(
            &mut self.transfer,
            self.budget.reserve("audit sort batch", 0)?,
        );
        self.sort.push_batch_reserved(rows, transfer).await
    }
    async fn finish(mut self) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
        self.flush().await?;
        self.sort.prepare_terminal().await
    }
}
impl crate::originals::BlobChunkSink for AuditRows {
    fn push(&mut self, row: Value) -> futures::future::BoxFuture<'_, Result<(), ModelError>> {
        AuditRows::push(self, row).boxed()
    }
}
/// A derived access layout beneath the exact contributor preparation. All rows
/// have reached the canonical run's EOF before an absent relation means empty.
/// These ordinary prepared runs share the operation's pool and spill ownership.
struct PreparedMembershipRelations {
    rows: BTreeMap<String, crate::ordered_rows::PreparedRows>,
    _charge: StateCharge,
}
impl PreparedMembershipRelations {
    fn prepare(
        memberships: &crate::ordered_rows::PreparedRows,
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<Self, ModelError> {
        check()?;
        let mut charge = StateCharge::new(budget, "logical membership relation access metadata");
        // Include the Arc header used by the retaining LogicalContributorRows owner.
        charge.grow(std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>())?;
        let mut partitions = BTreeMap::<String, crate::ordered_rows::SortedRows>::new();
        let mut cursor = memberships.cursor_with_budget(budget)?;
        loop {
            check()?;
            let Some(row) = cursor.next_row()? else {
                break;
            };
            let Some(Value::String(relation)) = row.as_object().and_then(|row| row.get("relation"))
            else {
                return Err(ModelError::Schema("prepared logical membership relation"));
            };
            if let Some(partition) = partitions.get_mut(relation.as_str()) {
                partition.push(row)?;
            } else {
                // Admit both provisional and finished map nodes before cloning the key.
                charge.grow(
                    std::mem::size_of::<(String, crate::ordered_rows::SortedRows)>()
                        .saturating_add(std::mem::size_of::<(
                            String,
                            crate::ordered_rows::PreparedRows,
                        )>())
                        .saturating_mul(11)
                        .saturating_add(relation.len().saturating_mul(2))
                        .saturating_add(256),
                )?;
                let relation = relation.clone();
                let mut partition = crate::ordered_rows::SortedRows::with_budget_and_row_bytes(
                    budget,
                    d::resources::MAX_ROW_BYTES,
                )?;
                partition.push(row)?;
                partitions.insert(relation, partition);
            }
        }
        drop(cursor);
        check()?;
        let mut rows = BTreeMap::new();
        let mut retained = std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>();
        for (relation, partition) in partitions {
            check()?;
            let run = partition.finish()?.into_prepared();
            retained = retained.saturating_add(
                std::mem::size_of::<(String, crate::ordered_rows::PreparedRows)>()
                    .saturating_mul(11)
                    .saturating_add(relation.capacity())
                    .saturating_add(128),
            );
            rows.insert(relation, run);
        }
        check()?;
        charge.release(charge.reserved().saturating_sub(retained));
        Ok(Self {
            rows,
            _charge: charge,
        })
    }
    fn relation(&self, relation: &str) -> Option<&crate::ordered_rows::PreparedRows> {
        self.rows.get(relation)
    }
}
struct ScopedAliasRows {
    // Ordered exact physical contributors and positive incarnations. Full native
    // rows remain independently retained by InstallationAudit::contributors.
    owners: Vec<(ContentHash, i64)>,
    rows: crate::ordered_rows::PreparedRows,
    _charge: StateCharge,
}
/// Registered success beneath this audit's live pin. The digest and count belong
/// to the same complete sorted run; neither is a portable completion witness.
struct PreparedSelectedMembers {
    rows: crate::ordered_rows::PreparedRows,
    count: u64,
    content: ContentHash,
}
struct PreparedBlobObservation {
    incarnation: i64,
    chunks: crate::ordered_rows::PreparedPointRows,
}
impl Drop for InstallationAudit {
    fn drop(&mut self) {
        if !self.closed.load(std::sync::atomic::Ordering::Acquire)
            && std::sync::Arc::strong_count(&self.pin) == 1
        {
            self.pin.retain_unknown();
        }
    }
}

struct ExactWork {
    record: RecordId,
    incarnation: i64,
    row: Value,
    _charge: Box<dyn d::resources::Reservation>,
}
fn exact_work(
    expected: &ExpectedRecord,
    budget: &ResourceBudget,
) -> Result<(ContentHash, ExactWork), ModelError> {
    let charge = budget.reserve(
        "exact contribution preparation worker inputs",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    let mut sink = d::KeySink::new("native-exact-protected-preparation/v1");
    sink.part(
        b"record",
        &serde_json::to_vec(expected.record()).map_err(ModelError::codec)?,
    );
    sink.part(b"incarnation", &expected.incarnation().to_le_bytes());
    sink.part(
        b"full-row",
        &serde_json::to_vec(expected.row()).map_err(ModelError::codec)?,
    );
    Ok((
        sink.finish(),
        ExactWork {
            record: expected.record().clone(),
            incarnation: expected.incarnation(),
            row: expected.row().clone(),
            _charge: charge,
        },
    ))
}
async fn worker_expected(
    reader: &NativeReader<()>,
    input: &ExactWork,
    budget: &ResourceBudget,
) -> Result<ExpectedRecord, ModelError> {
    let actual = crate::control::inspect_expected_read(reader, input.record.clone(), budget)
        .await?
        .ok_or(ModelError::Conflict("prepared contributor disappeared"))?;
    if actual.incarnation() != input.incarnation || actual.row() != input.row {
        return Err(ModelError::Conflict(
            "prepared contributor exact bytes/incarnation changed",
        ));
    }
    Ok(actual)
}

/// Exact physical observations remain retained through the final checked installation
/// effect, which must compare these incarnations while touching each contributor.
struct ContributionDescriptor {
    data: CompletedContribution,
    _charge: Box<dyn d::resources::Reservation>,
}
impl AsRef<CompletedContribution> for ContributionDescriptor {
    fn as_ref(&self) -> &CompletedContribution {
        &self.data
    }
}
#[derive(Clone)]
struct ObservedContributor {
    expected: std::sync::Arc<ExpectedRecord>,
    descriptor: std::sync::Arc<ContributionDescriptor>,
    logical: ContentHash,
}
/// A fresh boundary phase shares only actually inspected, exact-CAS-pinned copies.
/// A new phase starts empty even when its operation audit already has preparations.
struct ObservedView {
    expected: std::sync::Arc<ExpectedRecord>,
    data: CompletedView,
    _charge: Box<dyn d::resources::Reservation>,
}
impl AsRef<CompletedView> for ObservedView {
    fn as_ref(&self) -> &CompletedView {
        &self.data
    }
}
impl std::ops::Deref for ObservedView {
    type Target = CompletedView;
    fn deref(&self) -> &CompletedView {
        &self.data
    }
}
#[derive(Clone)]
enum FreshRequest {
    Contributor(ContributorRealization),
    View(ContentHash),
}
impl FreshRequest {
    fn work_key(&self) -> (u8, ContentHash) {
        match self {
            Self::Contributor(mapping) => (6, inspected_contributor_key(mapping)),
            Self::View(identity) => (7, *identity),
        }
    }
}
fn inspected_contributor_key(mapping: &ContributorRealization) -> ContentHash {
    use d::Key;
    let mut sink = d::KeySink::new("native-exact-inspected-contributor/v1");
    mapping.physical.encode(&mut sink);
    mapping.logical.encode(&mut sink);
    sink.part(b"incarnation", &mapping.incarnation.to_le_bytes());
    sink.finish()
}
fn inspected_hold_key(owner: &RecordId, object: &RecordId) -> Result<ContentHash, ModelError> {
    let mut sink = d::KeySink::new("native-exact-inspected-incidence/v1");
    sink.part(
        b"owner",
        &serde_json::to_vec(owner).map_err(ModelError::codec)?,
    );
    sink.part(
        b"object",
        &serde_json::to_vec(object).map_err(ModelError::codec)?,
    );
    Ok(sink.finish())
}
enum FreshObservation {
    Contributor(ContentHash, std::sync::Arc<ObservedContributor>),
    View(ContentHash, std::sync::Arc<ObservedView>),
}
impl FreshObservation {
    fn expected(&self) -> &std::sync::Arc<ExpectedRecord> {
        match self {
            Self::Contributor(_, value) => &value.expected,
            Self::View(_, value) => &value.expected,
        }
    }
}
pub(crate) struct ClosureInspectionScope {
    pin: std::sync::Arc<crate::control::ReaderPin>,
    context: (ContentHash, i64),
    budget: ResourceBudget,
    observations: BTreeMap<ContentHash, std::sync::Arc<ObservedContributor>>,
    descriptors: BTreeMap<ContentHash, std::sync::Arc<ContributionDescriptor>>,
    views: BTreeMap<ContentHash, std::sync::Arc<ObservedView>>,
    holds: std::collections::BTreeSet<(RecordId, RecordId)>,
    // Input incidences belong to the exact physical copy, not each view which
    // selects it. Every copy still earns its own protected endpoints and holds.
    prepared_dependencies: std::collections::BTreeSet<(ContentHash, i64)>,
    protected: std::collections::BTreeSet<RecordId>,
    operation: bool,
    source: bool,
    boundary: bool,
    context_checked: bool,
    charge: StateCharge,
}
impl ClosureInspectionScope {
    pub(crate) fn new(
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        if !audit.shares_pool(budget) {
            return Err(ModelError::Conflict(
                "boundary inspection foreign resource owner",
            ));
        }
        Ok(Self {
            pin: audit.borrow_pin()?,
            context: audit.context,
            budget: budget.clone(),
            observations: BTreeMap::new(),
            descriptors: BTreeMap::new(),
            views: BTreeMap::new(),
            holds: std::collections::BTreeSet::new(),
            prepared_dependencies: std::collections::BTreeSet::new(),
            protected: std::collections::BTreeSet::new(),
            operation: false,
            source: false,
            boundary: false,
            context_checked: false,
            charge: StateCharge::new(budget, "boundary exact observation shared handles"),
        })
    }
    fn for_operation(
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut scope = Self::new(audit, budget)?;
        scope.operation = true;
        Ok(scope)
    }
    /// A compound receiving phase fences the captured context in its final native
    /// effect. Its selector iterations borrow one observation, without changing
    /// the independently checked endpoints or the final receiving context CAS.
    pub(crate) fn for_boundary(mut self) -> Self {
        self.boundary = true;
        self
    }
    pub(crate) fn check_receiving_source(
        &self,
        reader: &NativeReader<()>,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check_local(reader, audit, budget)
    }
    fn check_local(
        &self,
        reader: &NativeReader<()>,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if !std::sync::Arc::ptr_eq(&self.pin, &audit.pin)
            || self.context != audit.context
            || !self.budget.shares_pool(budget)
        {
            return Err(ModelError::Conflict(
                "boundary inspection foreign audit pin/context",
            ));
        }
        if self.source {
            audit.check_source_local(reader, budget)?;
        } else {
            audit.check(reader, budget)?;
        }
        self.pin.ensure_live()
    }
    async fn check(
        &mut self,
        reader: &NativeReader<()>,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check_local(reader, audit, budget)?;
        // A receiving phase retains exact-CAS-pinned endpoints, and its final
        // completion/acquisition effect independently fences captured context.
        // Ordinary scopes consume the same actual post-pin checked lifetime.
        if (self.operation && !self.boundary) || !self.context_checked {
            audit.ensure_context(reader, budget).await?;
            self.check_local(reader, audit, budget)?;
            self.context_checked = true;
        }
        Ok(())
    }
    fn remember(&mut self, record: &RecordId) -> Result<(), ModelError> {
        if !self.protected.contains(record) {
            self.charge
                .grow(crate::selection::node_bytes(record).saturating_add(64))?;
            self.protected.insert(record.clone());
        }
        Ok(())
    }
    pub(crate) fn expected_copy(
        &self,
        physical: &ContentHash,
    ) -> Result<&std::sync::Arc<ExpectedRecord>, ModelError> {
        self.pin.ensure_live()?;
        self.observations
            .get(physical)
            .map(|value| &value.expected)
            .ok_or(ModelError::Conflict(
                "compound exact contributor absent from fresh inspection",
            ))
    }
    /// Fresh destination observations are set reads followed by exact receiving CAS.
    /// Nothing from the operation registry can populate this phase's premises.
    async fn prepare_fresh(
        &mut self,
        reader: &NativeReader<()>,
        root: &CompletedView,
        mappings: &[ContributorRealization],
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.prepare_views(reader, std::iter::once((root, mappings)), audit, budget)
            .await
    }
    pub(crate) async fn prepare_views<'a>(
        &mut self,
        reader: &NativeReader<()>,
        views: impl Iterator<Item = (&'a CompletedView, &'a [ContributorRealization])> + Clone,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check(reader, audit, budget).await?;
        if views.clone().any(|(_, mappings)| {
            mappings
                .windows(2)
                .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
        }) {
            return Err(ModelError::Conflict(
                "installed selected mapping order/duplicate",
            ));
        }
        let mappings = views.clone().flat_map(|(_, mappings)| mappings.iter());
        let mut requests = BTreeMap::new();
        let mut request_charge =
            StateCharge::new(budget, "fresh boundary observation request identities");
        for mapping in mappings.clone() {
            if mapping.incarnation <= 0 {
                return Err(ModelError::Conflict("installed selected incarnation"));
            }
            if let Some(observed) = self.observations.get(&mapping.physical) {
                if observed.logical != mapping.logical
                    || observed.expected.incarnation() != mapping.incarnation
                {
                    return Err(ModelError::Conflict(
                        "boundary selected physical mapping contradiction",
                    ));
                }
                continue;
            }
            let id = RecordId::new("compiler_contribution", mapping.physical.hex());
            if let Some(FreshRequest::Contributor(previous)) = requests.get(&id) {
                if previous.logical != mapping.logical
                    || previous.incarnation != mapping.incarnation
                {
                    return Err(ModelError::Conflict(
                        "boundary selected physical mapping contradiction",
                    ));
                }
            } else {
                request_charge.grow(
                    crate::selection::node_bytes(&id)
                        .saturating_add(std::mem::size_of::<FreshRequest>())
                        .saturating_add(128),
                )?;
                requests.insert(id, FreshRequest::Contributor(mapping.clone()));
            }
        }
        self.observe_fresh_requests(reader, requests, audit, budget)
            .await?;
        let mut dependency_owners = std::collections::BTreeSet::new();
        for mapping in mappings.clone() {
            let owner = (mapping.physical, mapping.incarnation);
            if !self.prepared_dependencies.contains(&owner) && !dependency_owners.contains(&owner) {
                request_charge.grow(std::mem::size_of::<(ContentHash, i64)>() + 32)?;
                dependency_owners.insert(owner);
            }
        }
        let mut requests = BTreeMap::new();
        // Declared inputs include empty dependency views with no mapping entries.
        let identities = views
            .map(|(root, _)| root.identity)
            .chain(mappings.clone().map(|mapping| mapping.view))
            .chain(dependency_owners.iter().flat_map(|(physical, _)| {
                self.observations[physical]
                    .descriptor
                    .data
                    .spec
                    .inputs
                    .iter()
                    .map(|input| input.view())
            }));
        for identity in identities {
            if self.views.contains_key(&identity) {
                continue;
            }
            let id = RecordId::new("compiler_view", identity.hex());
            if let std::collections::btree_map::Entry::Vacant(entry) = requests.entry(id) {
                request_charge.grow(
                    crate::selection::node_bytes(entry.key())
                        .saturating_add(std::mem::size_of::<FreshRequest>())
                        .saturating_add(128),
                )?;
                entry.insert(FreshRequest::View(identity));
            }
        }
        self.observe_fresh_requests(reader, requests, audit, budget)
            .await?;
        let mut pairs = std::collections::BTreeSet::new();
        let mut pair_charge =
            StateCharge::new(budget, "fresh boundary exact incidence request identities");
        let view_pairs = mappings.map(|mapping| {
            let owner = RecordId::new("compiler_view", mapping.view.hex());
            let physical = RecordId::new("compiler_contribution", mapping.physical.hex());
            (owner, physical)
        });
        let input_pairs = dependency_owners.iter().flat_map(|(physical, _)| {
            let inputs = &self.observations[physical].descriptor.data.spec.inputs;
            inputs.iter().map(move |input| {
                (
                    RecordId::new("compiler_contribution", physical.hex()),
                    RecordId::new("compiler_view", input.view().hex()),
                )
            })
        });
        for pair in view_pairs.chain(input_pairs) {
            if !self.holds.contains(&pair) && !pairs.contains(&pair) {
                pair_charge.grow(
                    crate::selection::node_bytes(&pair.0)
                        .saturating_add(crate::selection::node_bytes(&pair.1))
                        .saturating_add(256),
                )?;
                pairs.insert(pair);
            }
        }
        pair_charge.grow(
            pairs
                .len()
                .saturating_mul(std::mem::size_of::<(RecordId, RecordId)>()),
        )?;
        let pairs = pairs.into_iter().collect::<Vec<_>>();
        let mut offset = 0;
        let mut width = crate::loader::NATIVE_WINDOW_ROWS;
        while offset < pairs.len() {
            let count = width.min(pairs.len() - offset);
            let window = &pairs[offset..offset + count];
            self.check_local(reader, audit, budget)?;
            if window.iter().any(|(owner, object)| {
                !self.protected.contains(owner) || !self.protected.contains(object)
            }) {
                return Err(ModelError::Conflict(
                    "inspected incidence requires both exact pinned endpoints",
                ));
            }
            let captured = async {
                if self.operation {
                    self.observe_operation_holds(reader, window, audit, budget)
                        .await
                } else {
                    let mut actual =
                        crate::control::inspect_hold_window_read(reader, window, budget).await?;
                    for (owner, object) in window {
                        actual.take_required(owner, object)?;
                    }
                    actual.finish()
                }
            }
            .await;
            match captured {
                Err(error) if count > 1 && terminal_local_resource(&error) => {
                    width = (count / 2).max(1);
                    continue;
                }
                other => other?,
            }
            for pair in window {
                self.charge.grow(
                    crate::selection::node_bytes(&pair.0)
                        .saturating_add(crate::selection::node_bytes(&pair.1))
                        .saturating_add(64),
                )?;
                self.holds.insert(pair.clone());
            }
            offset += count;
            width = crate::loader::NATIVE_WINDOW_ROWS;
        }
        self.check_local(reader, audit, budget)?;
        // Publish reuse only after the complete dependency observation and hold
        // windows have reached EOF successfully under this scope's live pin.
        for owner in dependency_owners {
            self.charge
                .grow(std::mem::size_of::<(ContentHash, i64)>() + 32)?;
            self.prepared_dependencies.insert(owner);
        }
        Ok(())
    }
    async fn observe_fresh_requests(
        &mut self,
        reader: &NativeReader<()>,
        requests: BTreeMap<RecordId, FreshRequest>,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if self.operation {
            return self
                .observe_operation_requests(reader, requests, audit, budget)
                .await;
        }
        if requests.is_empty() {
            return Ok(());
        }
        let _identities = budget.reserve(
            "fresh boundary observation ordered handles",
            requests
                .len()
                .saturating_mul(std::mem::size_of::<(RecordId, FreshRequest)>())
                .saturating_add(4096),
        )?;
        let requests = requests.into_iter().collect::<Vec<_>>();
        let mut offset = 0;
        while offset < requests.len() {
            let mut count = (requests.len() - offset).min(crate::loader::NATIVE_WINDOW_ROWS);
            loop {
                let page = &requests[offset..offset + count];
                let captured = async {
                    let _page = budget.reserve(
                        "fresh boundary observation page handles",
                        count.saturating_mul(4096).saturating_add(4096),
                    )?;
                    let ids = page.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
                    let mut actual =
                        crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
                    let mut observed = Vec::with_capacity(count);
                    let mut descriptors =
                        BTreeMap::<ContentHash, std::sync::Arc<ContributionDescriptor>>::new();
                    for (id, request) in page {
                        let expected = actual.take_required(id)?;
                        observed.push(match request {
                            FreshRequest::Contributor(mapping) => {
                                let mut value =
                                    decode_selected_contributor(expected, mapping, budget)?;
                                if let Some(previous) = self
                                    .descriptors
                                    .get(&mapping.logical)
                                    .or_else(|| descriptors.get(&mapping.logical))
                                {
                                    if !std::sync::Arc::ptr_eq(previous, &value.descriptor)
                                        && previous.data != value.descriptor.data
                                    {
                                        return Err(ModelError::Conflict(
                                            "installed equivalent copies differ in full descriptor",
                                        ));
                                    }
                                    value = std::sync::Arc::new(ObservedContributor {
                                        expected: value.expected.clone(),
                                        descriptor: previous.clone(),
                                        logical: value.logical,
                                    });
                                }
                                descriptors.insert(mapping.logical, value.descriptor.clone());
                                FreshObservation::Contributor(mapping.physical, value)
                            }
                            FreshRequest::View(identity) => FreshObservation::View(
                                *identity,
                                decode_selected_view(expected, *identity, budget)?,
                            ),
                        });
                    }
                    actual.finish()?;
                    let expected = observed
                        .iter()
                        .map(|value| value.expected().clone())
                        .collect::<Vec<_>>();
                    crate::control::live_view::transfer_expected_many(&self.pin, expected, budget)
                        .await?;
                    self.check_local(reader, audit, budget)?;
                    Ok::<_, ModelError>(observed)
                }
                .await;
                let observed = match captured {
                    Ok(value) => value,
                    Err(ModelError::Resource { .. }) if count > 1 => {
                        count = (count / 2).max(1);
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                for value in observed {
                    self.charge.grow(1024)?;
                    self.remember(value.expected().record())?;
                    match value {
                        FreshObservation::Contributor(physical, value) => {
                            self.descriptors
                                .insert(value.logical, value.descriptor.clone());
                            self.observations.insert(physical, value);
                        }
                        FreshObservation::View(identity, value) => {
                            self.views.insert(identity, value);
                        }
                    }
                }
                offset += count;
                break;
            }
        }
        Ok(())
    }
    async fn observe_operation_requests(
        &mut self,
        reader: &NativeReader<()>,
        requests: BTreeMap<RecordId, FreshRequest>,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let _handles = budget.reserve(
            "operation observation request handles",
            requests
                .len()
                .saturating_mul(std::mem::size_of::<(RecordId, FreshRequest)>()),
        )?;
        let requests = requests.into_iter().collect::<Vec<_>>();
        let mut offset = 0;
        let mut width = crate::loader::NATIVE_WINDOW_ROWS;
        while offset < requests.len() {
            let count = width.min(requests.len() - offset);
            let page = &requests[offset..offset + count];
            let prepared = async {
                let input = budget.reserve("operation observation worker inputs", count.saturating_mul(4096).saturating_add(4096))?;
                let requested = page.iter().map(|(id, request)| (request.work_key(), (id.clone(), request.clone()))).collect::<BTreeMap<_, _>>();
                let keys = requested.keys().copied().collect::<Vec<_>>();
                let worker_budget = budget.clone();
                audit.run_work_many(keys, move |owner, claimed| async move {
                    let _input = input;
                    let reader = NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                    let result = async {
                        let mut missing = BTreeMap::new();
                        for key in claimed {
                            let (id, request) = &requested[&key];
                            let known = match request {
                                FreshRequest::Contributor(mapping) => {
                                    if let Some(value) = owner.inspected_contributors.get(&mapping.physical) {
                                        if value.logical != mapping.logical || value.expected.incarnation() != mapping.incarnation {
                                            return Err(ModelError::Conflict("operation selected physical mapping contradiction"));
                                        }
                                        true
                                    } else { false }
                                }
                                FreshRequest::View(identity) => owner.inspected_views.contains_key(identity),
                            };
                            if !known { missing.insert(id.clone(), request.clone()); }
                        }
                        let mut scope = ClosureInspectionScope::new(&owner, &worker_budget)?;
                        scope.observe_fresh_requests(&reader, missing, &owner, &worker_budget).await?;
                        owner.ensure_context(&reader, &worker_budget).await?;
                        Ok(scope)
                    }.await;
                    let mut terminal = d::completion::Completion::default();
                    terminal.step("operation observation window reader close", reader.close().await);
                    let scope = d::completion::complete(result, terminal)?;
                    owner.pin.ensure_live()?;
                    for (physical, value) in scope.observations {
                        owner.charge.grow(std::mem::size_of::<(ContentHash, std::sync::Arc<ObservedContributor>)>().saturating_add(128))?;
                        owner.inspected_contributors.insert_arc(physical, value);
                    }
                    for (identity, value) in scope.views {
                        owner.charge.grow(std::mem::size_of::<(ContentHash, std::sync::Arc<ObservedView>)>().saturating_add(128))?;
                        owner.inspected_views.insert_arc(identity, value);
                    }
                    Ok(())
                }.boxed()).await
            }.await;
            match prepared {
                Err(error) if count > 1 && terminal_local_resource(&error) => {
                    width = (count / 2).max(1);
                    continue;
                }
                other => other?,
            }
            self.check_local(reader, audit, budget)?;
            for (_, request) in page {
                self.charge.grow(1024)?;
                match request {
                    FreshRequest::Contributor(mapping) => {
                        let value = audit.inspected_contributors.get(&mapping.physical).ok_or(
                            ModelError::Conflict("operation inspected contributor absent"),
                        )?;
                        if value.logical != mapping.logical
                            || value.expected.incarnation() != mapping.incarnation
                        {
                            return Err(ModelError::Conflict(
                                "operation selected physical mapping contradiction",
                            ));
                        }
                        let value = if let Some(previous) = self.descriptors.get(&mapping.logical) {
                            if !std::sync::Arc::ptr_eq(previous, &value.descriptor)
                                && previous.data != value.descriptor.data
                            {
                                return Err(ModelError::Conflict(
                                    "installed equivalent copies differ in full descriptor",
                                ));
                            }
                            std::sync::Arc::new(ObservedContributor {
                                expected: value.expected.clone(),
                                descriptor: previous.clone(),
                                logical: value.logical,
                            })
                        } else {
                            value
                        };
                        self.remember(value.expected.record())?;
                        self.descriptors
                            .insert(mapping.logical, value.descriptor.clone());
                        self.observations.insert(mapping.physical, value);
                    }
                    FreshRequest::View(identity) => {
                        let value = audit
                            .inspected_views
                            .get(identity)
                            .ok_or(ModelError::Conflict("operation inspected view absent"))?;
                        self.remember(value.expected.record())?;
                        self.views.insert(*identity, value);
                    }
                }
            }
            offset += count;
            width = crate::loader::NATIVE_WINDOW_ROWS;
        }
        Ok(())
    }
    async fn observe_operation_holds(
        &self,
        reader: &NativeReader<()>,
        pairs: &[(RecordId, RecordId)],
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let input = budget.reserve(
            "operation incidence worker inputs",
            pairs.len().saturating_mul(4096),
        )?;
        let requested = pairs
            .iter()
            .map(|(owner, object)| {
                Ok((
                    (8, inspected_hold_key(owner, object)?),
                    (owner.clone(), object.clone()),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, ModelError>>()?;
        let keys = requested.keys().copied().collect::<Vec<_>>();
        let worker_budget = budget.clone();
        audit
            .run_work_many(keys, move |owner, claimed| {
                async move {
                    let _input = input;
                    let pairs = claimed
                        .into_iter()
                        .map(|key| requested[&key].clone())
                        .filter(|pair| !owner.inspected_holds.contains_key(pair))
                        .collect::<Vec<_>>();
                    let reader =
                        NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                    let result = async {
                        let mut actual = crate::control::inspect_hold_window_read(
                            &reader,
                            &pairs,
                            &worker_budget,
                        )
                        .await?;
                        for (holder, object) in &pairs {
                            actual.take_required(holder, object)?;
                        }
                        actual.finish()?;
                        owner.ensure_context(&reader, &worker_budget).await
                    }
                    .await;
                    let mut terminal = d::completion::Completion::default();
                    terminal.step(
                        "operation incidence window reader close",
                        reader.close().await,
                    );
                    d::completion::complete(result, terminal)?;
                    owner.pin.ensure_live()?;
                    for pair in pairs {
                        owner.charge.grow(
                            crate::selection::node_bytes(&pair.0)
                                .saturating_add(crate::selection::node_bytes(&pair.1))
                                .saturating_add(128),
                        )?;
                        owner.inspected_holds.insert(pair, ());
                    }
                    Ok(())
                }
                .boxed()
            })
            .await?;
        self.check_local(reader, audit, budget)
    }
    async fn view(
        &mut self,
        reader: &NativeReader<()>,
        identity: ContentHash,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<std::sync::Arc<ObservedView>, ModelError> {
        self.check_local(reader, audit, budget)?;
        if let Some(observed) = self.views.get(&identity).cloned() {
            self.remember(observed.expected.record())?;
            return Ok(observed);
        }
        let observed = if self.operation {
            let worker_budget = budget.clone();
            audit
                .run_work((7, identity), move |owner| {
                    async move {
                        if owner.inspected_views.contains_key(&identity) {
                            return owner.pin.ensure_live();
                        }
                        let reader =
                            NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                        let result = async {
                            let observed =
                                inspect_view_fresh(&reader, identity, &worker_budget).await?;
                            crate::control::live_view::transfer_expected(
                                &owner.pin,
                                &observed.expected,
                                &worker_budget,
                            )
                            .await?;
                            owner.ensure_context(&reader, &worker_budget).await?;
                            Ok(observed)
                        }
                        .await;
                        let mut terminal = d::completion::Completion::default();
                        terminal.step("inspected view worker close", reader.close().await);
                        let observed = d::completion::complete(result, terminal)?;
                        owner.pin.ensure_live()?;
                        owner.charge.grow(
                            std::mem::size_of::<(ContentHash, std::sync::Arc<ObservedView>)>()
                                .saturating_add(128),
                        )?;
                        owner.inspected_views.insert_arc(identity, observed);
                        Ok(())
                    }
                    .boxed()
                })
                .await?;
            audit
                .inspected_views
                .get(&identity)
                .ok_or(ModelError::Conflict("operation inspected view absent"))?
        } else {
            return Err(ModelError::Conflict(
                "fresh boundary exact view not prepared",
            ));
        };
        self.check_local(reader, audit, budget)?;
        self.charge.grow(
            std::mem::size_of::<(ContentHash, std::sync::Arc<ObservedView>)>().saturating_add(32),
        )?;
        self.remember(observed.expected.record())?;
        self.views.insert(identity, observed.clone());
        Ok(observed)
    }
    async fn require_hold(
        &mut self,
        reader: &NativeReader<()>,
        owner: &RecordId,
        object: &RecordId,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check_local(reader, audit, budget)?;
        if !self.protected.contains(owner) || !self.protected.contains(object) {
            return Err(ModelError::Conflict(
                "inspected incidence requires both exact pinned endpoints",
            ));
        }
        let pair = (owner.clone(), object.clone());
        if self.holds.contains(&pair) {
            return Ok(());
        }
        if self.operation {
            let input = budget.reserve("exact inspected incidence worker inputs", 4096)?;
            let key = inspected_hold_key(owner, object)?;
            let worker_pair = pair.clone();
            let worker_budget = budget.clone();
            audit
                .run_work((8, key), move |owner| {
                    async move {
                        let _input = input;
                        if owner.inspected_holds.contains_key(&worker_pair) {
                            return owner.pin.ensure_live();
                        }
                        let reader =
                            NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                        let result = inspect_exact_hold(
                            &reader,
                            &worker_pair.0,
                            &worker_pair.1,
                            &worker_budget,
                        )
                        .await;
                        let mut terminal = d::completion::Completion::default();
                        terminal.step("inspected hold worker close", reader.close().await);
                        d::completion::complete(result, terminal)?;
                        owner.pin.ensure_live()?;
                        owner.charge.grow(
                            crate::selection::node_bytes(&worker_pair.0)
                                .saturating_add(crate::selection::node_bytes(&worker_pair.1))
                                .saturating_add(128),
                        )?;
                        owner.inspected_holds.insert(worker_pair, ());
                        Ok(())
                    }
                    .boxed()
                })
                .await?;
        } else {
            return Err(ModelError::Conflict(
                "fresh boundary exact incidence not prepared",
            ));
        }
        self.charge.grow(
            crate::selection::node_bytes(owner)
                .saturating_add(crate::selection::node_bytes(object))
                .saturating_add(64),
        )?;
        self.holds.insert(pair);
        self.check_local(reader, audit, budget)
    }
}
/// Shared logical metadata retains domain traversal meaning without retaining
/// another full contribution descriptor for every selector that reaches it.
pub(crate) struct InspectedLogicalClosure {
    views: SharedViews,
    contributions: SharedContributions,
}
impl InspectedLogicalClosure {
    pub(crate) fn views(&self) -> &SharedViews {
        &self.views
    }
    pub(crate) fn contributions(&self) -> &SharedContributions {
        &self.contributions
    }
}
pub(crate) struct SharedViews(BTreeMap<ContentHash, std::sync::Arc<ObservedView>>);
impl SharedViews {
    pub(crate) fn keys(&self) -> impl Iterator<Item = &ContentHash> {
        self.0.keys()
    }
    pub(crate) fn contains_key(&self, identity: &ContentHash) -> bool {
        self.0.contains_key(identity)
    }
}
pub(crate) struct SharedViewIter<'a>(
    std::collections::btree_map::Iter<'a, ContentHash, std::sync::Arc<ObservedView>>,
);
impl<'a> Iterator for SharedViewIter<'a> {
    type Item = (&'a ContentHash, &'a CompletedView);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(identity, view)| (identity, &view.data))
    }
}
impl<'a> IntoIterator for &'a SharedViews {
    type Item = (&'a ContentHash, &'a CompletedView);
    type IntoIter = SharedViewIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        SharedViewIter(self.0.iter())
    }
}
pub(crate) struct SharedContributions(
    BTreeMap<ContentHash, std::sync::Arc<ContributionDescriptor>>,
);
impl std::ops::Index<&ContentHash> for SharedContributions {
    type Output = CompletedContribution;
    fn index(&self, id: &ContentHash) -> &Self::Output {
        &self.0[id].data
    }
}
pub(crate) struct SharedContributionIter<'a>(
    std::collections::btree_map::Iter<'a, ContentHash, std::sync::Arc<ContributionDescriptor>>,
);
impl<'a> Iterator for SharedContributionIter<'a> {
    type Item = (&'a ContentHash, &'a CompletedContribution);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(id, descriptor)| (id, &descriptor.data))
    }
}
impl<'a> IntoIterator for &'a SharedContributions {
    type Item = (&'a ContentHash, &'a CompletedContribution);
    type IntoIter = SharedContributionIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        SharedContributionIter(self.0.iter())
    }
}
pub(crate) struct InspectedClosure {
    pub(crate) logical: InspectedLogicalClosure,
    pub(crate) contributors: BTreeMap<ContentHash, std::sync::Arc<ExpectedRecord>>,
    descriptors_by_physical: BTreeMap<ContentHash, std::sync::Arc<ContributionDescriptor>>,
    pub(crate) mappings: Vec<ContributorRealization>,
    _charge: StateCharge,
    _logical_charge: StateCharge,
}
impl InspectedClosure {
    pub(crate) fn descriptor_for_copy(
        &self,
        physical: &ContentHash,
    ) -> Result<&CompletedContribution, ModelError> {
        self.descriptors_by_physical
            .get(physical)
            .map(|descriptor| &descriptor.data)
            .ok_or(ModelError::Conflict(
                "compound physical copy descriptor absent",
            ))
    }
}
fn shared_descriptor(
    data: CompletedContribution,
    budget: &ResourceBudget,
) -> Result<std::sync::Arc<ContributionDescriptor>, ModelError> {
    let charge = budget.reserve(
        "shared inspected contributor typed descriptor",
        std::mem::size_of::<ContributionDescriptor>()
            .saturating_add(32)
            .saturating_add(contribution_heap(&data)),
    )?;
    Ok(std::sync::Arc::new(ContributionDescriptor {
        data,
        _charge: charge,
    }))
}
async fn inspect_selected_contributor(
    reader: &NativeReader<()>,
    mapping: &ContributorRealization,
    budget: &ResourceBudget,
    scope: Option<(&mut ClosureInspectionScope, &InstallationAudit)>,
) -> Result<std::sync::Arc<ObservedContributor>, ModelError> {
    if mapping.incarnation <= 0 {
        return Err(ModelError::Conflict("installed selected incarnation"));
    }
    if let Some((scope, audit)) = scope {
        scope.check_local(reader, audit, budget)?;
        if let Some(observed) = scope.observations.get(&mapping.physical).cloned() {
            if observed.logical != mapping.logical
                || observed.expected.incarnation() != mapping.incarnation
            {
                return Err(ModelError::Conflict(
                    "boundary selected physical mapping contradiction",
                ));
            }
            scope.remember(observed.expected.record())?;
            return Ok(observed);
        }
        let observed = if scope.operation {
            operation_selected_contributor(reader, mapping, budget, audit).await?
        } else {
            return Err(ModelError::Conflict(
                "fresh boundary exact contributor not prepared",
            ));
        };
        let descriptor = if let Some(previous) = scope.descriptors.get(&mapping.logical) {
            if !std::sync::Arc::ptr_eq(previous, &observed.descriptor)
                && previous.data != observed.descriptor.data
            {
                return Err(ModelError::Conflict(
                    "installed equivalent copies differ in full descriptor",
                ));
            }
            previous.clone()
        } else {
            observed.descriptor.clone()
        };
        // Sharing is earned by actual receiving exact-byte/incarnation CAS, not
        // by a cached descriptor or an earlier boundary's successful audit.
        scope.check_local(reader, audit, budget)?;
        scope.charge.grow(
            std::mem::size_of::<ObservedContributor>()
                .saturating_add(std::mem::size_of::<(
                    ContentHash,
                    std::sync::Arc<ObservedContributor>,
                )>())
                .saturating_add(256),
        )?;
        let observed = std::sync::Arc::new(ObservedContributor {
            expected: observed.expected.clone(),
            descriptor: descriptor.clone(),
            logical: mapping.logical,
        });
        scope.remember(observed.expected.record())?;
        scope.descriptors.insert(mapping.logical, descriptor);
        scope
            .observations
            .insert(mapping.physical, observed.clone());
        Ok(observed)
    } else {
        inspect_selected_contributor_fresh(reader, mapping, budget).await
    }
}
async fn inspect_selected_contributor_fresh(
    reader: &NativeReader<()>,
    mapping: &ContributorRealization,
    budget: &ResourceBudget,
) -> Result<std::sync::Arc<ObservedContributor>, ModelError> {
    let expected = crate::control::inspect_expected_read(
        reader,
        RecordId::new("compiler_contribution", mapping.physical.hex()),
        budget,
    )
    .await?
    .ok_or(ModelError::Conflict(
        "installed selected contributor missing",
    ))?;
    decode_selected_contributor(expected, mapping, budget)
}
fn decode_selected_contributor(
    expected: ExpectedRecord,
    mapping: &ContributorRealization,
    budget: &ResourceBudget,
) -> Result<std::sync::Arc<ObservedContributor>, ModelError> {
    if mapping.incarnation <= 0
        || expected.record() != &RecordId::new("compiler_contribution", mapping.physical.hex())
    {
        return Err(ModelError::Conflict(
            "installed selected contributor identity",
        ));
    }
    if expected.incarnation() != mapping.incarnation {
        return Err(ModelError::Conflict(
            "installed selected contributor incarnation",
        ));
    }
    let _decode = budget.reserve(
        "installed selected contributor decode",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    validate_state_row("compiler_contribution", expected.row())?;
    let data: CompletedContribution = descriptor(expected.row())?;
    if data.identity()? != mapping.logical {
        return Err(ModelError::Conflict(
            "installed selected contributor logical identity",
        ));
    }
    Ok(std::sync::Arc::new(ObservedContributor {
        expected: std::sync::Arc::new(expected),
        descriptor: shared_descriptor(data, budget)?,
        logical: mapping.logical,
    }))
}
async fn operation_selected_contributor(
    reader: &NativeReader<()>,
    mapping: &ContributorRealization,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<std::sync::Arc<ObservedContributor>, ModelError> {
    audit.check(reader, budget)?;
    if let Some(observed) = audit.inspected_contributors.get(&mapping.physical) {
        if observed.logical != mapping.logical
            || observed.expected.incarnation() != mapping.incarnation
        {
            return Err(ModelError::Conflict(
                "operation selected physical mapping contradiction",
            ));
        }
        return Ok(observed);
    }
    let input = budget.reserve("exact inspected contributor worker mapping", 4096)?;
    let key = inspected_contributor_key(mapping);
    let mapping = mapping.clone();
    let physical = mapping.physical;
    let logical = mapping.logical;
    let incarnation = mapping.incarnation;
    let worker_budget = budget.clone();
    audit
        .run_work((6, key), move |owner| {
            async move {
                let _input = input;
                if let Some(observed) = owner.inspected_contributors.get(&mapping.physical) {
                    if observed.logical != mapping.logical
                        || observed.expected.incarnation() != mapping.incarnation
                    {
                        return Err(ModelError::Conflict(
                            "operation selected physical mapping contradiction",
                        ));
                    }
                    return owner.pin.ensure_live();
                }
                let reader =
                    NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                let result = async {
                    let observed =
                        inspect_selected_contributor_fresh(&reader, &mapping, &worker_budget)
                            .await?;
                    crate::control::live_view::transfer_expected(
                        &owner.pin,
                        &observed.expected,
                        &worker_budget,
                    )
                    .await?;
                    owner.ensure_context(&reader, &worker_budget).await?;
                    Ok(observed)
                }
                .await;
                let mut terminal = d::completion::Completion::default();
                terminal.step("inspected contributor worker close", reader.close().await);
                let observed = d::completion::complete(result, terminal)?;
                owner.pin.ensure_live()?;
                owner.charge.grow(
                    std::mem::size_of::<(ContentHash, std::sync::Arc<ObservedContributor>)>()
                        .saturating_add(128),
                )?;
                owner
                    .inspected_contributors
                    .insert_arc(mapping.physical, observed);
                Ok(())
            }
            .boxed()
        })
        .await?;
    let observed = audit
        .inspected_contributors
        .get(&physical)
        .ok_or(ModelError::Conflict(
            "operation inspected contributor absent",
        ))?;
    if observed.logical != logical || observed.expected.incarnation() != incarnation {
        return Err(ModelError::Conflict(
            "operation selected physical mapping contradiction",
        ));
    }
    Ok(observed)
}
async fn inspect_view_fresh(
    reader: &NativeReader<()>,
    identity: ContentHash,
    budget: &ResourceBudget,
) -> Result<std::sync::Arc<ObservedView>, ModelError> {
    let expected = crate::control::inspect_expected_read(
        reader,
        RecordId::new("compiler_view", identity.hex()),
        budget,
    )
    .await?
    .ok_or(ModelError::Conflict("installed selected view cardinality"))?;
    decode_selected_view(expected, identity, budget)
}
fn decode_selected_view(
    expected: ExpectedRecord,
    identity: ContentHash,
    budget: &ResourceBudget,
) -> Result<std::sync::Arc<ObservedView>, ModelError> {
    if expected.record() != &RecordId::new("compiler_view", identity.hex()) {
        return Err(ModelError::Conflict("installed exact view identity"));
    }
    let _decode = budget.reserve(
        "installed exact view decode",
        crate::loader::native_bytes(expected.row())
            .saturating_mul(32)
            .saturating_add(65536),
    )?;
    validate_state_row("compiler_view", expected.row())?;
    let data: CompletedView = descriptor(expected.row())?;
    data.validate()?;
    if data.identity != identity {
        return Err(ModelError::Conflict("installed exact view identity"));
    }
    let charge = budget.reserve(
        "shared inspected view descriptor",
        std::mem::size_of::<ObservedView>()
            .saturating_add(32)
            .saturating_add(data.relation.capacity())
            .saturating_add(
                data.contributions
                    .len()
                    .saturating_mul(std::mem::size_of::<ContentHash>() + 32),
            ),
    )?;
    Ok(std::sync::Arc::new(ObservedView {
        expected: std::sync::Arc::new(expected),
        data,
        _charge: charge,
    }))
}
async fn inspect_exact_hold(
    reader: &NativeReader<()>,
    owner: &RecordId,
    object: &RecordId,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut rows = crate::control::inspect_hold_window_read(
        reader,
        &[(owner.clone(), object.clone())],
        budget,
    )
    .await?;
    rows.take_required(owner, object)?;
    rows.finish()
}
/// Heap admission for a retained typed contribution, independently of any
/// ExpectedRecord native row or temporary descriptor serialization.
pub fn contribution_heap(value: &CompletedContribution) -> usize {
    use d::HeapSize;
    value
        .spec
        .producer
        .capacity()
        .saturating_add(value.spec.inputs.heap_bytes())
        .saturating_add(value.spec.outputs.heap_bytes())
        .saturating_add(
            value
                .spec
                .captured_binding
                .as_ref()
                .map_or(0, |binding| binding.heap_bytes()),
        )
        .saturating_add(
            value
                .outputs
                .keys()
                .map(|name| {
                    std::mem::size_of::<(String, d::completed::OutputContent)>()
                        + 32
                        + name.capacity()
                })
                .fold(0, usize::saturating_add),
        )
}
/// Typed view heap admission estimate, including B-tree nodes. This is not RSS.
pub fn view_heap(view: &CompletedView) -> usize {
    view.relation.capacity().saturating_add(
        view.contributions
            .len()
            .saturating_mul(std::mem::size_of::<ContentHash>() + 32),
    )
}
pub(crate) fn descriptor<T: serde::de::DeserializeOwned>(row: &Value) -> Result<T, ModelError> {
    let Some(Value::Bytes(bytes)) = row.as_object().and_then(|row| row.get("descriptor")) else {
        return Err(ModelError::Schema("installed audit descriptor"));
    };
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}
fn exact_mapping_views<'a>(
    views: impl Iterator<Item = &'a CompletedView>,
    mappings: &[ContributorRealization],
) -> Result<(), ModelError> {
    if mappings
        .windows(2)
        .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
        || mappings.iter().any(|mapping| mapping.incarnation <= 0)
        || !views
            .flat_map(|view| {
                view.contributions
                    .iter()
                    .map(move |logical| (view.identity, *logical))
            })
            .eq(mappings
                .iter()
                .map(|mapping| (mapping.view, mapping.logical)))
    {
        return Err(ModelError::Conflict(
            "installed exact dependency realization inventory",
        ));
    }
    let mut physical = BTreeMap::new();
    for mapping in mappings {
        let selected = (mapping.logical, mapping.incarnation);
        if physical
            .insert(mapping.physical, selected)
            .is_some_and(|previous| previous != selected)
        {
            return Err(ModelError::Conflict(
                "installed exact dependency realization inventory",
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
fn exact_mappings(
    closure: &RecoveryClosure,
    mappings: &[ContributorRealization],
) -> Result<(), ModelError> {
    exact_mapping_views(closure.views().values(), mappings)
}
/// Reads only explicitly supplied physical contributors. A matching logical copy elsewhere
/// in the installation cannot satisfy a missing, replaced or malformed selected contributor.
#[cfg(test)]
pub(crate) async fn inspect_closure(
    reader: &NativeReader<()>,
    root: &CompletedView,
    mappings: &[ContributorRealization],
    budget: &ResourceBudget,
) -> Result<InspectedClosure, ModelError> {
    inspect_closure_inner(reader, root, mappings, budget, None).await
}
pub(crate) async fn inspect_closure_scoped(
    reader: &NativeReader<()>,
    root: &CompletedView,
    mappings: &[ContributorRealization],
    budget: &ResourceBudget,
    audit: &InstallationAudit,
    scope: &mut ClosureInspectionScope,
) -> Result<InspectedClosure, ModelError> {
    scope.check(reader, audit, budget).await?;
    scope
        .prepare_fresh(reader, root, mappings, audit, budget)
        .await?;
    inspect_closure_inner(reader, root, mappings, budget, Some((scope, audit))).await
}
pub(crate) async fn inspect_closure_in_audit(
    reader: &NativeReader<()>,
    root: &CompletedView,
    mappings: &[ContributorRealization],
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<InspectedClosure, ModelError> {
    let mut scope = ClosureInspectionScope::for_operation(audit, budget)?;
    inspect_closure_scoped(reader, root, mappings, budget, audit, &mut scope).await
}
async fn inspect_closure_inner(
    reader: &NativeReader<()>,
    root: &CompletedView,
    mappings: &[ContributorRealization],
    budget: &ResourceBudget,
    mut scope: Option<(&mut ClosureInspectionScope, &InstallationAudit)>,
) -> Result<InspectedClosure, ModelError> {
    // Validation constructs an independent CompletedView, including its set clone.
    let _validation =
        budget.reserve("installed closure view validation scratch", view_heap(root))?;
    root.validate()?;
    drop(_validation);
    let mut charge = StateCharge::new(budget, "installed exact closure descriptors");
    charge.grow(
        mappings
            .len()
            .saturating_mul(std::mem::size_of::<ContributorRealization>()),
    )?;
    if mappings
        .windows(2)
        .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
    {
        return Err(ModelError::Conflict(
            "installed selected mapping order/duplicate",
        ));
    }
    let mut descriptor_charge =
        StateCharge::new(budget, "installed local contributor descriptor handles");
    let mut contributors = BTreeMap::new();
    let mut descriptors_by_physical = BTreeMap::new();
    let mut descriptors: BTreeMap<ContentHash, std::sync::Arc<ContributionDescriptor>> =
        BTreeMap::new();
    for mapping in mappings {
        let observed = match scope.as_mut() {
            Some((scope, audit)) => {
                inspect_selected_contributor(reader, mapping, budget, Some((&mut **scope, *audit)))
                    .await?
            }
            None => inspect_selected_contributor(reader, mapping, budget, None).await?,
        };
        if let Some(previous) = descriptors.get(&mapping.logical) {
            if !std::sync::Arc::ptr_eq(previous, &observed.descriptor)
                && previous.data != observed.descriptor.data
            {
                return Err(ModelError::Conflict(
                    "installed equivalent copies differ in full descriptor",
                ));
            }
        } else {
            descriptor_charge.grow(
                std::mem::size_of::<(ContentHash, std::sync::Arc<ContributionDescriptor>)>()
                    .saturating_add(32),
            )?;
            descriptors.insert(mapping.logical, observed.descriptor.clone());
        }
        if !contributors.contains_key(&mapping.physical) {
            charge.grow(
                std::mem::size_of::<(ContentHash, std::sync::Arc<ExpectedRecord>)>()
                    .saturating_add(std::mem::size_of::<(
                        ContentHash,
                        std::sync::Arc<ContributionDescriptor>,
                    )>())
                    .saturating_add(64),
            )?;
        }
        descriptors_by_physical.insert(mapping.physical, descriptors[&mapping.logical].clone());
        if let Some(previous) = contributors.insert(mapping.physical, observed.expected.clone())
            && !std::sync::Arc::ptr_eq(&previous, &observed.expected)
            && (previous.incarnation() != observed.expected.incarnation()
                || previous.row() != observed.expected.row())
        {
            return Err(ModelError::Conflict(
                "installed selected physical observation contradiction",
            ));
        }
    }
    // The adapter still independently observes the exact root and every selected
    // physical copy. Shared ingredients contain logical direct edges only; each
    // root rechecks its mapping and view holds, with copy input holds earned
    // once under the same live inspection scope and final context fence.
    let root_observed = match scope.as_mut() {
        Some((scope, audit)) => scope.view(reader, root.identity, audit, budget).await?,
        None => inspect_view_fresh(reader, root.identity, budget).await?,
    };
    if root_observed.data != *root {
        return Err(ModelError::Conflict("recovery exact dependency view"));
    }
    let mut traversal = SharedRecoveryTraversal::for_view(root_observed, budget)?;
    let mut checked_dependencies = std::collections::BTreeSet::new();
    let mut dependency_check_charge =
        StateCharge::new(budget, "installed exact physical dependency owner handles");
    while let Some(requested) = traversal.next_view()? {
        let observed = match scope.as_mut() {
            Some((scope, audit)) => {
                scope
                    .view(reader, requested.identity, audit, budget)
                    .await?
            }
            None => inspect_view_fresh(reader, requested.identity, budget).await?,
        };
        if !std::sync::Arc::ptr_eq(&observed, &requested) && observed.data != requested.data {
            return Err(ModelError::Conflict("recovery exact dependency view"));
        }
        // A fresh receiving phase earns its own ingredients. Only an actual
        // operation/source scope can borrow its own audit's cached preparation.
        let operation_audit = scope
            .as_ref()
            .and_then(|(scope, audit)| (scope.operation || scope.source).then(|| (*audit).clone()));
        let cached = operation_audit
            .as_ref()
            .and_then(|audit| audit.recovery_views.get(&requested.identity));
        if cached.as_ref().is_some_and(|ingredient| {
            !std::sync::Arc::ptr_eq(ingredient.view(), &observed)
                && ingredient.view().data != observed.data
        }) {
            return Err(ModelError::Conflict("recovery view descriptor collision"));
        }
        let mut local = StateCharge::new(budget, "installed direct ingredient input handles");
        if cached.is_none() {
            local.grow(
                requested
                    .contributions
                    .len()
                    .saturating_mul(std::mem::size_of::<
                        std::sync::Arc<
                            SharedRecoveryContribution<ObservedView, ContributionDescriptor>,
                        >,
                    >()),
            )?;
        }
        let mut selected = Vec::with_capacity(if cached.is_none() {
            requested.contributions.len()
        } else {
            0
        });
        for logical in &requested.contributions {
            let contribution = descriptors
                .get(logical)
                .ok_or(ModelError::Conflict("installed missing dependency mapping"))?;
            let ready = operation_audit
                .as_ref()
                .and_then(|audit| audit.recovery_contributors.get(logical));
            if ready.as_ref().is_some_and(|ingredient| {
                !std::sync::Arc::ptr_eq(ingredient.data(), contribution)
                    && ingredient.data().data != contribution.data
            }) {
                return Err(ModelError::Conflict("recovery contributor collision"));
            }
            let mapping = &mappings[mappings
                .binary_search_by_key(&(requested.identity, *logical), |mapping| {
                    (mapping.view, mapping.logical)
                })
                .map_err(|_| ModelError::Conflict("installed selected mapping disappeared"))?];
            let view_record = RecordId::new("compiler_view", requested.identity.hex());
            let contributor_record = RecordId::new("compiler_contribution", mapping.physical.hex());
            match scope.as_mut() {
                Some((scope, audit)) => {
                    scope
                        .require_hold(reader, &view_record, &contributor_record, audit, budget)
                        .await?
                }
                None => {
                    crate::completed_state::require_hold(reader, &view_record, &contributor_record)
                        .await?
                }
            }
            let mut dependencies = BTreeMap::new();
            let mut dependency_charge =
                StateCharge::new(budget, "installed direct dependency input handles");
            let physical_owner = (mapping.physical, mapping.incarnation);
            let require_inputs = !checked_dependencies.contains(&physical_owner)
                && !scope.as_ref().is_some_and(|(scope, _)| {
                    scope.prepared_dependencies.contains(&physical_owner)
                });
            let build_dependencies = ready.is_none() && cached.is_none();
            // The same copy's inputs were already assured as a whole by the
            // phase, or by this independent traversal. New logical ingredients
            // still resolve every declared input, including empty views.
            if require_inputs || build_dependencies {
                for input in &contribution.data.spec.inputs {
                    if build_dependencies && !dependencies.contains_key(&input.view()) {
                        let dependency = match scope.as_mut() {
                            Some((scope, audit)) => {
                                scope.view(reader, input.view(), audit, budget).await?
                            }
                            None => inspect_view_fresh(reader, input.view(), budget).await?,
                        };
                        dependency_charge.grow(
                            std::mem::size_of::<(ContentHash, std::sync::Arc<ObservedView>)>() + 32,
                        )?;
                        dependencies.insert(input.view(), dependency);
                    }
                    if !require_inputs {
                        continue;
                    }
                    let dependency_record = RecordId::new("compiler_view", input.view().hex());
                    match scope.as_mut() {
                        Some((scope, audit)) => {
                            scope
                                .require_hold(
                                    reader,
                                    &contributor_record,
                                    &dependency_record,
                                    audit,
                                    budget,
                                )
                                .await?
                        }
                        None => {
                            crate::completed_state::require_hold(
                                reader,
                                &contributor_record,
                                &dependency_record,
                            )
                            .await?
                        }
                    }
                }
            }
            if require_inputs {
                dependency_check_charge.grow(std::mem::size_of::<(ContentHash, i64)>() + 32)?;
                checked_dependencies.insert(physical_owner);
            }
            if cached.is_none() {
                let make = || {
                    SharedRecoveryContribution::new(contribution.clone(), budget, |identity| {
                        dependencies
                            .get(&identity)
                            .cloned()
                            .ok_or(ModelError::Conflict("installed undeclared dependency view"))
                    })
                };
                let ingredient = match (ready, operation_audit.as_ref()) {
                    (Some(ingredient), _) => ingredient,
                    (None, Some(audit)) => {
                        audit
                            .recovery_contributors
                            .get_or_try_insert(*logical, || {
                                let value = make()?;
                                audit.charge.grow(
                                    std::mem::size_of::<(
                                        ContentHash,
                                        std::sync::Arc<
                                            SharedRecoveryContribution<
                                                ObservedView,
                                                ContributionDescriptor,
                                            >,
                                        >,
                                    )>() + 32,
                                )?;
                                Ok(value)
                            })?
                    }
                    (None, None) => std::sync::Arc::new(make()?),
                };
                selected.push(ingredient);
            }
        }
        let ingredient = if let Some(ingredient) = cached {
            ingredient
        } else if let Some(audit) = operation_audit.as_ref() {
            audit
                .recovery_views
                .get_or_try_insert(requested.identity, || {
                    let value = SharedRecoveryIngredient::new(observed, selected, budget)?;
                    audit.charge.grow(
                        std::mem::size_of::<(
                            ContentHash,
                            std::sync::Arc<
                                SharedRecoveryIngredient<ObservedView, ContributionDescriptor>,
                            >,
                        )>() + 32,
                    )?;
                    Ok(value)
                })?
        } else {
            std::sync::Arc::new(SharedRecoveryIngredient::new(observed, selected, budget)?)
        };
        traversal.include_view(&requested, &ingredient)?;
    }
    let logical = traversal.finish()?;
    let _mapping_check = budget.reserve(
        "installed exact mapping correspondence",
        mappings
            .len()
            .saturating_mul(std::mem::size_of::<(ContentHash, (ContentHash, i64))>() + 32),
    )?;
    exact_mapping_views(logical.views().values().map(|view| &view.data), mappings)?;
    drop(_mapping_check);
    let (views, contributions, logical_charge) = logical.into_parts();
    let shared_logical = InspectedLogicalClosure {
        views: SharedViews(views),
        contributions: SharedContributions(contributions),
    };
    if let Some((scope, audit)) = scope.as_mut() {
        scope.check(reader, audit, budget).await?;
    }
    Ok(InspectedClosure {
        logical: shared_logical,
        contributors,
        descriptors_by_physical,
        mappings: mappings.to_vec(),
        _charge: charge,
        _logical_charge: logical_charge,
    })
}

/// Borrow the context already observed by this actual fresh boundary phase. Each
/// selector keeps its full independent closure/member/alias proof; native finality
/// still fences installation context after complete inventory EOF.
pub(crate) async fn assure_boundary_closure(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
    scope: &mut ClosureInspectionScope,
    full: bool,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    scope.check(reader, audit, budget).await?;
    audit_memberships_demand(reader, closure, budget, audit, full).await?;
    let aliases = audit_aliases_in_context(reader, closure, budget, audit).await?;
    if full {
        prepare_closure_in_context(reader, closure, budget, audit).await?;
    }
    scope.check_local(reader, audit, budget)?;
    Ok(aliases)
}
pub(crate) async fn audit_memberships_scoped(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<(), ModelError> {
    audit.ensure_context(reader, budget).await?;
    audit_memberships_in_context(reader, closure, budget, audit).await
}
async fn audit_memberships_in_context(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<(), ModelError> {
    audit_memberships_demand(reader, closure, budget, audit, false).await
}
async fn audit_memberships_demand(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
    full: bool,
) -> Result<(), ModelError> {
    audit.check(reader, budget)?;
    let mut contributors = closure.contributors.values().peekable();
    while contributors.peek().is_some() {
        // Keep borrowed premises; exact worker captures retain their own charges.
        // A native window is a maximum, and local admission can end it earlier.
        let mut page = Vec::new();
        let mut page_charge = StateCharge::new(budget, "contributor protection page identities");
        while page.len() < crate::loader::NATIVE_WINDOW_ROWS {
            let Some(expected) = contributors.peek().copied() else {
                break;
            };
            if let Some(cached) = audit.contributors.get(expected.record()) {
                if cached.incarnation() != expected.incarnation() || cached.row() != expected.row()
                {
                    return Err(ModelError::Conflict(
                        "reused installed contributor premise changed",
                    ));
                }
                audit.check(reader, budget)?;
                contributors.next();
                continue;
            }
            // Covers Vec's initial four pointer slots and subsequent growth.
            match page_charge.grow(4 * std::mem::size_of::<&std::sync::Arc<ExpectedRecord>>()) {
                Err(ModelError::Resource { .. }) if !page.is_empty() => break,
                result => result?,
            }
            page.push(expected);
            contributors.next();
        }
        protect_contributor_page(audit, &page, budget).await?;
        let mut window = ContributorWindow::new(budget);
        for expected in page {
            let protected = expected.clone();
            let (key, input) = window.exact_capture(expected).await?;
            let budget = budget.clone();
            window
                .push(async move {
                    audit
                        .run_work((0, key), move |owner| {
                            async move {
                                let reader = NativeReader::private(owner.client.clone())
                                    .with_budget(&budget);
                                let result = async {
                                    let expected =
                                        worker_expected(&reader, &input, &budget).await?;
                                    let descriptor =
                                        descriptor::<CompletedContribution>(expected.row())?;
                                    let fresh = audit_one_protected_contributor(
                                        &reader,
                                        &descriptor,
                                        &expected,
                                        &budget,
                                        &owner,
                                        full,
                                    )
                                    .await?;
                                    Ok(fresh.map(|products| (protected, products)))
                                }
                                .await;
                                let mut terminal = d::completion::Completion::default();
                                terminal.step(
                                    "complete contributor worker reader close",
                                    reader.close().await,
                                );
                                let prepared = d::completion::complete(result, terminal)?;
                                owner.pin.ensure_live()?;
                                if let Some((prepared, (membership, logical))) = prepared {
                                    owner
                                        .membership_contributors
                                        .insert(prepared.record().clone(), membership);
                                    if let Some(logical) = logical {
                                        owner
                                            .logical_contributors
                                            .insert(prepared.record().clone(), logical);
                                        owner
                                            .logical_contribution_observations
                                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    }
                                    owner
                                        .contributors
                                        .insert_arc(prepared.record().clone(), prepared);
                                    owner
                                        .contribution_observations
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }
                                Ok(())
                            }
                            .boxed()
                        })
                        .await
                })
                .await?;
        }
        window.finish().await?;
    }
    Ok(())
}
/// Protect the exact uncached page before any worker is enqueued. The same pin
/// remains owned through every worker's fresh read and final checked installation.
async fn protect_contributor_page(
    audit: &InstallationAudit,
    page: &[&std::sync::Arc<ExpectedRecord>],
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut offset = 0;
    let mut width = crate::loader::NATIVE_WINDOW_ROWS;
    while offset < page.len() {
        let count = width.min(page.len() - offset);
        let bytes = page[offset..offset + count]
            .iter()
            .map(|expected| crate::selection::node_bytes(expected.record()).saturating_mul(2))
            .fold(0usize, usize::saturating_add);
        let _request = match budget.reserve("contributor protection request identities", bytes) {
            Err(ModelError::Resource { .. }) if count > 1 => {
                width = (count / 2).max(1);
                continue;
            }
            result => result?,
        };
        let result = audit
            .pin
            .protect_many(
                page[offset..offset + count]
                    .iter()
                    .map(|expected| expected.record().clone())
                    .collect(),
            )
            .await;
        match result {
            // protect_many's direct Resource is local hold-plan/continuation
            // admission before issuance. Issued native errors carry their request
            // or completion envelope and must never replay as smaller effects.
            Err(error) if count > 1 && terminal_local_resource(&error) => {
                width = (count / 2).max(1);
                continue;
            }
            result => result?,
        }
        offset += count;
        width = crate::loader::NATIVE_WINDOW_ROWS;
    }
    Ok(())
}
/// Private sole caller above has protected the complete page before scheduling.
/// This still independently earns every byte/incarnation and complete EOF audit.
async fn audit_one_protected_contributor(
    reader: &NativeReader<()>,
    descriptor: &CompletedContribution,
    expected: &ExpectedRecord,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
    full: bool,
) -> Result<Option<(AuditedContributorMembership, Option<LogicalContributorRows>)>, ModelError> {
    use d::{Key, KeySink, completed::OutputContent};
    audit.ensure_context(reader, budget).await?;
    let owner = expected.record().clone();
    if let Some(cached) = audit.contributors.get(&owner) {
        let current = crate::control::inspect_expected_read(reader, owner.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict("reused installed contributor missing"))?;
        if current.incarnation() != cached.incarnation()
            || current.row() != cached.row()
            || current.row() != expected.row()
        {
            return Err(ModelError::Conflict(
                "reused installed contributor premise changed",
            ));
        }
        return Ok(None);
    }
    let protected = crate::control::inspect_expected_read(reader, owner.clone(), budget)
        .await?
        .ok_or(ModelError::Conflict(
            "protected installed contributor missing",
        ))?;
    if protected.incarnation() != expected.incarnation() || protected.row() != expected.row() {
        return Err(ModelError::Conflict(
            "protected installed contributor changed",
        ));
    }
    let _outputs = budget.reserve(
        "installed output digest inventory",
        serde_json::to_vec(&descriptor.outputs)
            .map_err(ModelError::codec)?
            .len()
            .saturating_mul(8)
            .saturating_add(1024),
    )?;
    let mut outputs = descriptor
        .outputs
        .keys()
        .map(|name| {
            (
                name.clone(),
                (0u64, KeySink::new("native-contribution-output/v1")),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut after_relation = String::new();
    let mut after_key = String::new();
    let owners = BTreeMap::from([(owner.clone(), descriptor.clone())]);
    // Native preparation retains fused physical locators; logical six-family membership
    // is normalized only for a consumer that actually requests that family.
    let mut memberships = AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES).await?;
    let mut logical_memberships = if full {
        Some(AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES).await?)
    } else {
        None
    };
    let mut backing = if full {
        Some(AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES).await?)
    } else {
        None
    };
    let mut variables = Variables::new();
    variables.insert("owner", owner.clone());
    let statements=vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner ORDER BY relation,semantic_key".into()];
    #[cfg(test)]
    let statements = {
        let mut statements = statements;
        if LATE_MEMBERSHIP_FAILURE
            .try_with(|enabled| *enabled)
            .unwrap_or(false)
        {
            statements.push("THROW 'late independent membership audit control'".into());
        }
        statements
    };
    let mut stream = reader.stream_prepared(crate::prepared::PreparedQuery::new(
        variables,
        vec![],
        statements,
    )?)?;
    let observed = async {
        let mut window = Vec::new();
        let mut charge = StateCharge::new(budget, "installed membership window");
        while let Some(row) = stream.next().await? {
            let bytes = crate::loader::native_bytes(&row)
                .saturating_mul(8)
                .saturating_add(512);
            charge.grow(bytes)?;
            let row = &row;
            crate::compiler_blocks::validate_locator(row)?;
            let object = row
                .as_object()
                .ok_or(ModelError::Schema("installed membership object"))?;
            let text = |field: &str| -> Result<String, ModelError> {
                match object.get(field) {
                    Some(Value::String(value)) => Ok(value.clone()),
                    _ => Err(ModelError::Schema("installed membership text")),
                }
            };
            let relation = text("relation")?;
            let key = text("semantic_key")?;
            let content = text("content")?;
            if object.get("contribution") != Some(&Value::RecordId(owner.clone()))
                || (!after_relation.is_empty()
                    && (relation.as_str(), key.as_str())
                        <= (after_relation.as_str(), after_key.as_str()))
            {
                return Err(ModelError::Conflict(
                    "installed membership owner/order/duplicate",
                ));
            }
            let (count, sink) = outputs.get_mut(&relation).ok_or(ModelError::Conflict(
                "installed undeclared output membership",
            ))?;
            key.encode(sink);
            content.encode(sink);
            *count = count
                .checked_add(1)
                .ok_or(ModelError::Schema("installed membership count"))?;
            let Some(Value::RecordId(node)) = object.get("node") else {
                return Err(ModelError::Schema("installed membership node"));
            };
            let _ = node;
            after_relation = relation;
            after_key = key;
            memberships.push(row.clone()).await?;
            if let Some(logical) = logical_memberships.as_mut() {
                logical
                    .push(crate::completed_logical_rows::normalize(
                        "compiler_membership",
                        row,
                        &owners,
                    )?)
                    .await?;
            }
            window.push(row.clone());
            if window.len() >= crate::loader::NATIVE_WINDOW_ROWS
                || charge.reserved() >= d::resources::TRANSFER_BYTES
            {
                audit_membership_window(reader, &owner, &window, budget, audit, backing.as_mut())
                    .await?;
                window.clear();
                charge = StateCharge::new(budget, "installed membership window");
            }
        }
        if !window.is_empty() {
            audit_membership_window(reader, &owner, &window, budget, audit, backing.as_mut())
                .await?;
        }
        Ok::<_, ModelError>(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "installed membership complete stream drain",
        stream.drain_transport().await,
    );
    d::completion::complete(observed, terminal)?;
    let actual = outputs
        .into_iter()
        .map(|(relation, (rows, sink))| {
            (
                relation,
                OutputContent {
                    rows,
                    content: sink.finish(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    if actual != descriptor.outputs {
        // Keep the refusal contract while identifying the failed independent EOF
        // premise. Only bounded output metadata is recorded, never payload rows.
        for (relation, expected) in &descriptor.outputs {
            if actual.get(relation) != Some(expected) {
                let observed = actual.get(relation);
                tracing::error!(contributor = ?owner,
                    relation = %relation, expected_rows = expected.rows,
                    observed_rows = ?observed.map(|output| output.rows),
                    expected_content = %expected.content.hex(),
                    observed_content = ?observed.map(|output| output.content.hex()),
                    "independent installed contributor output EOF/digest mismatch");
            }
        }
        return Err(ModelError::Conflict("installed complete output EOF/digest"));
    }
    let current = crate::control::inspect_expected_read(reader, owner.clone(), budget)
        .await?
        .ok_or(ModelError::Conflict(
            "completed contributor missing after full EOF",
        ))?;
    if current.incarnation() != expected.incarnation() || current.row() != expected.row() {
        return Err(ModelError::Conflict(
            "completed contributor changed after full EOF",
        ));
    }
    audit.ensure_context(reader, budget).await?;
    audit.charge.grow(
        crate::loader::native_bytes(expected.row())
            .saturating_mul(4)
            .saturating_add(512),
    )?;
    let memberships = memberships.finish().await?;
    let partition_input = memberships.clone();
    let pin = audit.pin.clone();
    let relation_memberships = std::sync::Arc::new(
        audit
            .blocking(move |budget| {
                PreparedMembershipRelations::prepare(&partition_input, budget, || pin.ensure_live())
            })
            .await?,
    );
    let logical = if let Some(backing) = backing {
        let mut contribution = AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES).await?;
        contribution
            .push(crate::completed_logical_rows::normalize(
                "compiler_contribution",
                expected.row(),
                &owners,
            )?)
            .await?;
        Some(LogicalContributorRows {
            contribution: contribution.finish().await?,
            memberships: logical_memberships
                .expect("full family consumer owns logical memberships")
                .finish()
                .await?,
            backing: backing.finish().await?,
        })
    } else {
        None
    };
    let mut metadata = StateCharge::new(budget, "audited contributor membership metadata");
    metadata.grow(
        std::mem::size_of::<(RecordId, AuditedContributorMembership)>()
            .saturating_mul(11)
            .saturating_add(crate::selection::node_bytes(expected.record()))
            .saturating_add(128),
    )?;
    if logical.is_some() {
        audit.charge.grow(
            std::mem::size_of::<(RecordId, LogicalContributorRows)>()
                .saturating_mul(11)
                .saturating_add(1024),
        )?;
    }
    Ok(Some((
        AuditedContributorMembership {
            memberships,
            relation_memberships,
            _charge: metadata,
        },
        logical,
    )))
}
/// Stream exact point sets and require one active guard for every requested key.
/// Canonical payloads remain streamed; only small correspondence metadata is retained.
async fn audit_active_guards(
    reader: &NativeReader<()>,
    ids: &std::collections::BTreeSet<RecordId>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "installed audit guard correspondence");
    let mut guards = std::collections::BTreeSet::new();
    for id in ids {
        let guard = crate::control::guard_id(&Value::RecordId(id.clone()))?;
        charge.grow(
            crate::selection::node_bytes(&guard)
                .saturating_mul(3)
                .saturating_add(128),
        )?;
        guards.insert(guard);
    }
    for window in guards
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .chunks(crate::loader::NATIVE_WINDOW_ROWS)
    {
        let mut variables = Variables::new();
        variables.insert("guards", window.to_vec());
        let mut stream = reader.stream_prepared(crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec!["SELECT id,phase,incarnation FROM $guards".into()],
        )?)?;
        let result=async {
            let mut seen=std::collections::BTreeSet::new();
            while let Some(value)=stream.next().await? {
                let row=value.as_object().ok_or(ModelError::Schema("installed set guard object"))?;
                let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("installed set guard identity"));};
                if !window.contains(id) || !seen.insert(id.clone()) || row.get("phase")!=Some(&Value::String("active".into()))
                    || !matches!(row.get("incarnation"),Some(Value::Number(surrealdb::types::Number::Int(value))) if *value>0) {
                    return Err(ModelError::Conflict("installed set guard missing/duplicate/foreign/inactive"));
                }
            }
            if seen.len()!=window.len(){return Err(ModelError::Conflict("installed set guard missing"));}Ok(())
        }.await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "installed set guard stream drain",
            stream.drain_transport().await,
        );
        d::completion::complete(result, terminal)?;
    }
    Ok(())
}
pub(crate) async fn audit_owned_keys<Context>(
    reader: &NativeReader<Context>,
    owner: &RecordId,
    ids: &std::collections::BTreeSet<RecordId>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let _charge = budget.reserve(
        "installed set hold correspondence",
        ids.iter()
            .map(|id| {
                crate::selection::node_bytes(id)
                    .saturating_mul(6)
                    .saturating_add(256)
            })
            .sum(),
    )?;
    for window in ids
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .chunks(crate::loader::NATIVE_WINDOW_ROWS)
    {
        let mut variables = Variables::new();
        variables.insert("owner", owner.clone());
        variables.insert("objects", window.to_vec());
        let mut stream=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT id,owner,object FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object IN $objects".into()])?)?;
        let result = async {
            let mut seen = std::collections::BTreeSet::new();
            while let Some(value) = stream.next().await? {
                let row = value
                    .as_object()
                    .ok_or(ModelError::Schema("installed set hold object"))?;
                let Some(Value::RecordId(object)) = row.get("object") else {
                    return Err(ModelError::Schema("installed set hold target"));
                };
                if row.get("owner") != Some(&Value::RecordId(owner.clone()))
                    || !window.contains(object)
                    || !seen.insert(object.clone())
                {
                    return Err(ModelError::Conflict(
                        "installed set hold owner/duplicate/foreign",
                    ));
                }
            }
            if seen.len() != window.len() {
                return Err(ModelError::Conflict("installed set hold missing"));
            }
            Ok(())
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "installed set hold stream drain",
            stream.drain_transport().await,
        );
        d::completion::complete(result, terminal)?;
    }
    Ok(())
}
async fn audit_endpoint_window(
    reader: &NativeReader<()>,
    owner: &RecordId,
    expected: &BTreeMap<RecordId, Value>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    if expected.is_empty() {
        return Ok(());
    }
    let ids = expected
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    audit_active_guards(reader, &ids, budget).await?;
    audit_owned_keys(reader, owner, &ids, budget).await?;
    let mut variables = Variables::new();
    variables.insert("ids", ids.into_iter().collect::<Vec<_>>());
    let mut stream = reader
        .stream_prepared(crate::prepared::PreparedQuery::new(
            variables,
            vec![],
            vec!["SELECT * FROM $ids".into()],
        )?)?
        .with_row_bytes(d::resources::MAX_ROW_BYTES);
    let result = async {
        let mut seen = std::collections::BTreeSet::new();
        while let Some(row) = stream.next().await? {
            let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id")) else {
                return Err(ModelError::Schema("installed set endpoint identity"));
            };
            if !seen.insert(id.clone()) || expected.get(id) != Some(&row) {
                return Err(ModelError::Conflict(
                    "installed set endpoint bytes/duplicate/foreign",
                ));
            }
        }
        if seen.len() != expected.len() {
            return Err(ModelError::Conflict(
                "installed canonical endpoint missing before freeze",
            ));
        }
        Ok(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "installed set endpoint stream drain",
        stream.drain_transport().await,
    );
    d::completion::complete(result, terminal)
}
async fn audit_membership_window(
    reader: &NativeReader<()>,
    owner: &RecordId,
    members: &[Value],
    budget: &ResourceBudget,
    audit: &InstallationAudit,
    mut logical_backing: Option<&mut AuditRows>,
) -> Result<(), ModelError> {
    let mut metadata = BTreeMap::new();
    let mut charge = StateCharge::new(budget, "installed set backing correspondence");
    for row in members {
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("installed set member object"))?;
        let Some(Value::RecordId(node)) = object.get("node") else {
            return Err(ModelError::Schema("installed set member backing"));
        };
        charge.grow(
            crate::selection::node_bytes(node)
                .saturating_mul(5)
                .saturating_add(512),
        )?;
        if let Some(previous) = metadata.insert(node.clone(), object)
            && ["relation", "semantic_key", "content"]
                .iter()
                .any(|field| previous.get(*field) != object.get(*field))
        {
            return Err(ModelError::Conflict(
                "installed membership ambiguous backing",
            ));
        }
    }
    let mut actual =
        crate::compiler_blocks::inspect_backing_window_read(reader, members, budget).await?;
    let mut physical = BTreeSet::new();
    for node in metadata.keys() {
        let located = actual.get(node)?;
        if !physical.contains(located.physical_record()) {
            charge.grow(
                crate::selection::node_bytes(located.physical_record()).saturating_add(128),
            )?;
            physical.insert(located.physical_record().clone());
        }
    }
    // The backing reader already independently observed each actual guard/incarnation once.
    // Membership authorizes only its selected slot; holds protect actual shared containers.
    audit_owned_keys(reader, owner, &physical, budget).await?;
    let mut endpoints = BTreeMap::new();
    let mut endpoint_charge = StateCharge::new(budget, "installed endpoint set preparation");
    for (node, member) in metadata {
        let located = actual.take_required(&node)?;
        let row = located.row();
        let backing = row
            .as_object()
            .ok_or(ModelError::Schema("installed set backing object"))?;
        if backing.get("semantic_type") != member.get("relation")
            || backing.get("semantic_key") != member.get("semantic_key")
            || backing.get("content") != member.get("content")
        {
            return Err(ModelError::Conflict(
                "installed membership backing metadata",
            ));
        }
        let crate::completed_rows::CanonicalBackingEndpoints {
            endpoints: expected,
            original,
            reservation: _decode,
        } = canonical_backing_endpoints(row, budget)?;
        if let Some(original) = original {
            audit_original(reader, &original, budget, audit).await?;
        }
        for endpoint in expected {
            let Some(Value::RecordId(id)) = endpoint.as_object().and_then(|row| row.get("id"))
            else {
                return Err(ModelError::Schema("installed canonical endpoint id"));
            };
            let bytes = crate::loader::native_bytes(&endpoint)
                .saturating_mul(8)
                .saturating_add(512);
            if !endpoints.is_empty()
                && (endpoints.len() >= crate::loader::NATIVE_WINDOW_ROWS
                    || endpoint_charge.reserved().saturating_add(bytes)
                        > d::resources::TRANSFER_BYTES)
            {
                audit_endpoint_window(reader, owner, &endpoints, budget).await?;
                endpoints.clear();
                endpoint_charge = StateCharge::new(budget, "installed endpoint set preparation");
            }
            if let Some(previous) = endpoints.get(id) {
                if previous != endpoint {
                    return Err(ModelError::Conflict(
                        "installed endpoint expected collision",
                    ));
                }
            } else {
                endpoint_charge.grow(bytes)?;
                endpoints.insert(id.clone(), endpoint);
            }
        }
        if node.table.as_str() == "compiler_record"
            && let Some(sort) = logical_backing.as_mut()
        {
            sort.push_owned(located.into_owned_row(budget)?).await?;
        }
    }
    audit_endpoint_window(reader, owner, &endpoints, budget).await
}
// Original backing verification is one bounded logical audit operation. Erase its
// execution future at that seam rather than carrying the complete blob/query/recovery
// future graph through contributor preparation. Existing admission and workers remain.
fn audit_original<'a>(
    reader: &'a NativeReader<()>,
    original: &'a crate::completed_rows::OriginalBacking,
    budget: &'a ResourceBudget,
    audit: &'a InstallationAudit,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    async move {
        let source = d::graph::EntityId::of(original.artifact);
        let layout = crate::originals::layout(reader, &crate::originals::layout_id(source)?)
            .await?
            .ok_or(ModelError::Conflict("installed original layout"))?;
        crate::originals::validate_layout(reader, &layout).await?;
        crate::completed_state::require_hold(reader, &layout.id, &layout.blob.id()).await?;
        if !crate::originals::header(reader, &layout.blob).await? {
            return Err(ModelError::Conflict("installed original incomplete blob"));
        }
        let remaining = layout
            .blob
            .byte_len
            .checked_sub(original.start)
            .ok_or(ModelError::Conflict("installed original coverage"))?;
        if original.len as u64 != remaining.min(d::artifact::ARTIFACT_CHUNK_BYTES as u64) {
            return Err(ModelError::Conflict("installed original coverage"));
        }
        let blob = layout.blob.id();
        let premise = crate::originals::active_blob_premise(reader, &layout.blob).await?;
        if audit
            .blobs
            .get(&blob)
            .is_some_and(|observed| observed.incarnation != premise)
        {
            return Err(ModelError::Conflict(
                "installed original protected incarnation changed",
            ));
        }
        let mut key = d::KeySink::new("native-exact-blob-preparation/v1");
        key.part(
            b"descriptor",
            &serde_json::to_vec(&layout.blob.value(true)).map_err(ModelError::codec)?,
        );
        key.part(b"incarnation", &premise.to_le_bytes());
        let key = key.finish();
        let blob_id = blob.clone();
        let blob = layout.blob.clone();
        let worker_budget = budget.clone();
        audit
            .run_work((3, key), move |owner| {
                async move {
                    let reader =
                        NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                    let result = async {
                        if owner
                            .blobs
                            .get(&blob.id())
                            .is_some_and(|observed| observed.incarnation == premise)
                        {
                            return Ok(None);
                        }
                        owner.pin.protect(blob.id()).await?;
                        if crate::originals::active_blob_premise(&reader, &blob).await? != premise {
                            return Err(ModelError::Conflict(
                                "installed blob changed before observation",
                            ));
                        }
                        #[cfg(test)]
                        if let Ok(barrier) = BLOB_OBSERVATION_BARRIER.try_with(Clone::clone) {
                            barrier.observed.wait().await;
                            barrier.resume.wait().await;
                        }
                        let mut scanned =
                            AuditRows::new(&owner, &worker_budget, crate::ordered_rows::ROW_BYTES)
                                .await?;
                        crate::originals::scan_blob_to_sink(
                            &reader,
                            &blob,
                            &worker_budget,
                            &mut scanned,
                        )
                        .await?;
                        let chunks = owner.prepare_member_points(scanned.finish().await?).await?;
                        if crate::originals::active_blob_premise(&reader, &blob).await? != premise {
                            return Err(ModelError::Conflict(
                                "installed original observation premise changed",
                            ));
                        }
                        owner.ensure_context(&reader, &worker_budget).await?;
                        owner.charge.grow(
                            crate::selection::node_bytes(&blob.id())
                                .saturating_mul(4)
                                .saturating_add(128),
                        )?;
                        Ok(Some((blob.id(), chunks)))
                    }
                    .await;
                    let mut terminal = d::completion::Completion::default();
                    terminal.step("complete blob worker reader close", reader.close().await);
                    let prepared = d::completion::complete(result, terminal)?;
                    owner.pin.ensure_live()?;
                    if let Some((blob, chunks)) = prepared {
                        owner.blobs.insert(
                            blob,
                            PreparedBlobObservation {
                                incarnation: premise,
                                chunks,
                            },
                        );
                        owner
                            .blob_observations
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    Ok(())
                }
                .boxed()
            })
            .await?;
        let prepared = audit.blobs.get(&blob_id).ok_or(ModelError::Conflict(
            "installed original blob preparation absent",
        ))?;
        let (start, len, digest) = (original.start, original.len as u64, original.digest);
        audit
            .blocking(move |budget| {
                crate::originals::check_scanned_chunk(&prepared.chunks, start, len, digest, budget)
            })
            .await?;
        Ok(())
    }
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use d::{
        Record, Relation,
        analysis::sources::SourceSnapshot,
        completed::{ContributionSpec, OutputContent, OutputProfile},
        input::Package,
        stages::Profile,
    };
    use surrealdb::types::SurrealValue;

    // The compiler-store dev dependency has a distinct native crate instance in
    // unit builds. Its task locals cannot authorize this crate's inspection pins.
    pub(super) async fn inspection_session(
        budget: &ResourceBudget,
    ) -> (
        std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
        std::sync::Arc<crate::control::RecoveryFactoryOwner>,
        crate::control::FactoryLease,
    ) {
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native campaign"),
        ))
        .unwrap();
        let factory = crate::control::RecoveryFactoryOwner::new(
            config.receipt_binding().unwrap(),
            budget.clone(),
        )
        .unwrap();
        let (client, lease) = crate::reader::connect_owned(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
            &factory,
        )
        .await
        .unwrap();
        (client, factory, lease)
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn completed_blob_observation_survives_peer_hold_and_equal_replay_under_exact_pin() {
        use d::{
            EvidenceBytes, artifact::ArtifactChunk, input::InputRevision, source::SourceArtifact,
        };
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let store = NativeCompilerStore::begin(
            &config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(&config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let (client, observer_factory, observer_lease) = inspection_session(&budget).await;
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let peer = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            crate::control::ReaderPin::acquire(client.clone(), &[]),
        )
        .await
        .unwrap();
        let closing_audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let receiving_audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let result = crate::control::with_recovery_scope(observer_lease.recovery_scope().clone(), std::panic::AssertUnwindSafe(async {
            let sources = Relation::of::<SourceArtifact>();
            let chunks = Relation::of::<ArtifactChunk>();
            let relations = [sources.clone(), chunks.clone()];
            let mut specification = contribution("completed-blob-owner-replay", vec![]).spec;
            specification.outputs = relations
                .iter()
                .map(|relation| relation.name().into())
                .collect();
            let producer = store.begin_contribution(specification).await.unwrap();
            let bytes = [store.attempt().0.as_slice(), b"completed blob peer replay"].concat();
            // More than one 32-row producer page, with two original-bearing
            // families and distinct layouts sharing exactly one small blob.
            let source_rows = (0..37).map(|ordinal| SourceArtifact::from_bytes(
                InputRevision::from_entries(vec![]).unwrap().id(),
                format!("peer-{ordinal}.bin"), &bytes,
            ).unwrap()).collect::<Vec<_>>();
            let source = source_rows[0].clone();
            let chunk_rows = source_rows.iter().map(|source| ArtifactChunk {
                artifact: source.id(), ordinal: 0, body: EvidenceBytes(bytes.clone()),
            }).collect::<Vec<_>>();
            store.write_batch(&producer, &sources, &SourceArtifact::encode(&source_rows).unwrap(), &budget).await.unwrap();
            store.write_batch(&producer, &chunks, &ArtifactChunk::encode(&chunk_rows).unwrap(), &budget).await.unwrap();
            let views = store
                .complete_contribution(
                    producer,
                    d::stages::ProviderOutcome::Complete,
                    &relations,
                    &BTreeMap::new(),
                    &budget,
                )
                .await
                .unwrap();
            assert_eq!(views[sources.name()].rows, 37);
            assert_eq!(views[chunks.name()].rows, 37);
            let view = &views[chunks.name()];
            let held = store.installed_view(view, &budget).await.unwrap();
            let mappings: Vec<ContributorRealization> =
                serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap())
                    .unwrap();
            let closure = inspect_closure(&reader, view, &mappings, &budget)
                .await
                .unwrap();
            let layout = crate::originals::Layout::source(&source).unwrap();
            let barrier = BlobObservationBarrier {
                observed: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
                resume: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
            };
            let replay = async {
                barrier.observed.wait().await;
                peer.protect(layout.blob.id()).await.unwrap();
                for window in crate::control::ImmutableWindows::new(
                    vec![layout.blob.piece(0, &bytes).unwrap()],
                    Some(store.attempt()),
                    None,
                    None,
                )
                .with_budget(Some(&budget))
                {
                    crate::control::install_window(
                        &client,
                        Some(store.attempt()),
                        window.unwrap(),
                        Some(&budget),
                    )
                    .await
                    .unwrap();
                }
                barrier.resume.wait().await;
            };
            let (observed, ()) = tokio::join!(
                BLOB_OBSERVATION_BARRIER.scope(
                    barrier.clone(),
                    audit_memberships_scoped(&reader, &closure, &budget, &audit)
                ),
                replay
            );
            observed.unwrap();
            assert_eq!(audit.observations(), (1, 1));
            assert_eq!(audit.logical_contribution_observations(), 0);
            let selected = selected_prepared_members(&closure, view, &budget, &audit).await.unwrap();
            let mut cursor = selected.cursor_with_budget(&budget).unwrap();
            let mut actual_chunks = BTreeMap::new();
            let mut pointer_charge = StateCharge::new(&budget, "native backing selected locator control");
            pointer_charge.grow(37 * std::mem::size_of::<Value>() + 32 * std::mem::size_of::<RecordId>()) .unwrap();
            let mut pointers = Vec::with_capacity(37);
            let mut expected_backing = BTreeMap::new();

            while let Some(row) = cursor.next_row().unwrap() {
                let row = row.as_object().unwrap();
                let Some(Value::String(key)) = row.get("semantic_key") else { panic!("selected chunk key"); };
                let Some(Value::String(content)) = row.get("content") else { panic!("selected chunk content"); };
                assert!(actual_chunks.insert(key.clone(), content.clone()).is_none());
                let Some(Value::RecordId(node)) = row.get("node") else { panic!("selected chunk backing node"); };
                let pointer = row.get("locator").expect("selected native locator").clone();
                crate::compiler_blocks::validate_locator(&pointer).unwrap();
                pointer_charge.grow(crate::loader::native_bytes(&pointer).saturating_mul(8).saturating_add(key.len() + content.len() + 512)).unwrap();
                pointers.push(pointer);
                assert!(expected_backing.insert(node.clone(), (key.clone(), content.clone())).is_none());

            }
            let expected_chunks = chunk_rows.iter().map(|chunk| {
                // Explicit native canonical declaration from the fixture, independently
                // of the production adapter's metadata projection or typed record hash.
                let mut declared = surrealdb::types::Object::new();
                declared.insert("__type", "artifact_chunks");
                declared.insert("artifact", chunk.artifact.bytes().iter().map(|byte| i64::from(*byte)).collect::<Vec<_>>());
                declared.insert("ordinal", 0i64);
                declared.insert("original", crate::originals::layout_id(d::graph::EntityId::of(chunk.artifact)).unwrap());
                declared.insert("start", 0u64);
                declared.insert("len", bytes.len() as u64);
                declared.insert("digest", ContentHash::of(&bytes).hex());
                let encoded = serde_json::to_vec(&Value::Object(declared)).unwrap();
                (hex::encode(chunk.id().bytes()), ContentHash::of(&encoded).hex())
            }).collect::<BTreeMap<_, _>>();
            assert_eq!(actual_chunks, expected_chunks, "all explicit chunk identities and contents survive cross-page blob reuse");
            // Selected logical rows now share real blocks. The former per-row arena-count
            // premise is obsolete: every window must hydrate only its exact named members,
            // preserving earlier accepted windows. Typed resource-split behavior is exercised
            // independently by BackingPageRetries controls, without assuming physical packing.
            let mut backing = AuditRows::new(&audit, &budget, d::resources::MAX_ROW_BYTES).await.unwrap();
            append_backing_page(&reader, &pointers[..4], &budget, &mut backing).await.unwrap();
            append_backing_page(&reader, &pointers[4..36], &budget, &mut backing).await.unwrap();
            append_backing_page(&reader, &pointers[36..], &budget, &mut backing).await.unwrap();
            let completed = backing.finish().await.unwrap();
            let mut completed = completed.cursor_with_budget(&budget).unwrap();
            let mut actual_backing = BTreeMap::new();
            while let Some(row) = completed.next_row().unwrap() {
                let original = validate_state_row("compiler_record", &row).unwrap().unwrap();
                assert_eq!(original.start, 0);
                assert_eq!(original.len, bytes.len());
                assert_eq!(original.digest, ContentHash::of(&bytes));
                let row = row.as_object().unwrap();
                let Some(Value::RecordId(id)) = row.get("id") else { panic!("completed backing identity"); };
                let Some(Value::String(key)) = row.get("semantic_key") else { panic!("completed backing key"); };
                let Some(Value::String(content)) = row.get("content") else { panic!("completed backing content"); };
                assert!(actual_backing.insert(id.clone(), (key.clone(), content.clone())).is_none());
            }
            assert_eq!(actual_backing, expected_backing, "the accepted four-row prefix, later selected window and final row preserve exactly 37 canonical records");
            assert_eq!(audit.observations(), (1, 1));
            assert_eq!(audit.logical_contribution_observations(), 0, "selected block windows never restart contributor membership assurance");

            audit_memberships_scoped(&reader, &closure, &budget, &receiving_audit).await.unwrap();
            assert_eq!(receiving_audit.observations(), (1, 1),
                "fresh receiving boundary independently scans once despite 37 layouts and both original families");
            assert_eq!(receiving_audit.logical_contribution_observations(), 0);

            audit_memberships_scoped(&reader, &closure, &budget, &audit)
                .await
                .unwrap();
            assert_eq!(
                audit.observations(),
                (1, 1),
                "equal replay neither invalidates immutable bytes nor repeats the whole audit"
            );
            let source = closing_audit
                .protected_source(&client, &budget)
                .await
                .unwrap();
            let checked_context = closing_audit.checked_context_lifetime(&reader, &budget).unwrap();
            let mut scope = source.inspection_scope(&reader, &budget).unwrap();
            let inspected = inspect_closure_scoped(
                &reader,
                view,
                &mappings,
                &budget,
                &closing_audit,
                &mut scope,
            )
            .await
            .unwrap();
            let barrier = BlobObservationBarrier {
                observed: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
                resume: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
            };
            let close = async {
                barrier.observed.wait().await;
                let closing = closing_audit.close();
                tokio::pin!(closing);
                assert!(
                    futures::poll!(closing.as_mut()).is_pending(),
                    "source close must drain the accepted native first-miss blob worker"
                );
                closing_audit.pin.ensure_live().unwrap();
                assert!(checked_context.check(&reader, &budget).is_err(), "checked context refuses closing before native pin release");
                assert!(
                    closing_audit
                        .protected_source(&client, &budget)
                        .await
                        .is_err(),
                    "closing immediately refuses new source borrowers while its actual read drains"
                );
                barrier.resume.wait().await;
                closing.await.unwrap();
            };
            let (read, ()) = tokio::join!(
                BLOB_OBSERVATION_BARRIER.scope(
                    barrier.clone(),
                    assure_boundary_closure(
                        &reader,
                        &inspected,
                        &budget,
                        &closing_audit,
                        &mut scope,
                        true
                    )
                ),
                close,
            );
            assert!(
                read.is_err(),
                "source closure cannot publish a reusable boundary from an in-flight read"
            );
            assert!(closing_audit.work.workers.lock().unwrap().is_empty());
            assert!(closing_audit.work.draining.lock().await.workers.is_empty());
            assert!(source.inspection_scope(&reader, &budget).is_err());
        })
        .catch_unwind()).await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("peer blob reader close", reader.close().await);
        cleanup.step("peer blob audit pin close", audit.close().await);
        cleanup.step(
            "independent receiving blob audit pin close",
            receiving_audit.close().await,
        );
        cleanup.step("closing blob audit pin close", closing_audit.close().await);
        cleanup.step("peer blob other pin close", peer.release().await);
        cleanup.step("peer blob attempt cleanup", store.abandon().await);
        let observer_completion = observer_lease.complete_with(&cleanup).await;
        cleanup.step(
            "native audit observer owner completion",
            observer_completion,
        );
        drop(observer_lease);
        cleanup.step(
            "native audit observer factory drain",
            observer_factory
                .drain(&crate::control::RetryCancellation::default())
                .await,
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("peer blob cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn protected_source_real_inventory_reuses_exact_observations_and_refuses_changed_mapping_and_close()
     {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let store = NativeCompilerStore::begin(
            &config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(&config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let (client, observer_factory, observer_lease) = inspection_session(&budget).await;
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let result = crate::control::with_recovery_scope(observer_lease.recovery_scope().clone(), std::panic::AssertUnwindSafe(async {
            let relation = Relation::of::<Package>();
            let mut specification = contribution("protected-source-inventory", vec![]).spec;
            specification.outputs = [relation.name().into()].into();
            let producer = store.begin_contribution(specification).await.unwrap();
            let row = Package {
                name: format!("protected-source-{}", store.attempt().hex()),
            };
            store
                .write_batch(
                    &producer,
                    &relation,
                    &Package::encode(&[row]).unwrap(),
                    &budget,
                )
                .await
                .unwrap();
            let views = store
                .complete_contribution(
                    producer,
                    d::stages::ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &BTreeMap::new(),
                    &budget,
                )
                .await
                .unwrap();
            let view = &views[relation.name()];
            let held = store.installed_view(view, &budget).await.unwrap();
            let mappings: Vec<ContributorRealization> =
                serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap())
                    .unwrap();
            // Obtain the opaque source through its real context/pin acquisition, never
            // by constructing a nominal source wrapper or populating its registry.
            let source = audit.protected_source(&client, &budget).await.unwrap();
            let mut first = source.inspection_scope(&reader, &budget).unwrap();
            let closure =
                inspect_closure_scoped(&reader, view, &mappings, &budget, &audit, &mut first)
                    .await
                    .unwrap();
            let aliases = assure_boundary_closure(&reader, &closure, &budget, &audit, &mut first, true)
                .await
                .unwrap();
            let roots = logical_roots_scoped(&reader, &closure, &[], &aliases, &budget, &audit)
                .await
                .unwrap();
            let direct = audit.recovery_views.get(&view.identity).unwrap();
            let direct_contributor = audit.recovery_contributors.get(&mappings[0].logical).unwrap();
            let next = audit.work.next.load(std::sync::atomic::Ordering::Relaxed);
            let cold = inspect_closure(&reader, view, &mappings, &budget)
                .await
                .unwrap();
            let cold_aliases = audit_aliases(&reader, &cold, &budget).await.unwrap();
            assert_eq!(
                roots,
                logical_roots(&reader, &cold, &[], &cold_aliases, &budget)
                    .await
                    .unwrap()
            );
            let mut repeated = source.inspection_scope(&reader, &budget).unwrap();
            let again =
                inspect_closure_scoped(&reader, view, &mappings, &budget, &audit, &mut repeated)
                    .await
                    .unwrap();
            assure_boundary_closure(&reader, &again, &budget, &audit, &mut repeated, true)
                .await
                .unwrap();
            assert!(std::sync::Arc::ptr_eq(&direct, &audit.recovery_views.get(&view.identity).unwrap()), "direct view preparation is reused within its actual protected operation");
            assert!(std::sync::Arc::ptr_eq(&direct_contributor, &audit.recovery_contributors.get(&mappings[0].logical).unwrap()), "direct dependency preparation is not rebuilt per root");
            assert!(std::sync::Arc::ptr_eq(&closure.logical.views.0[&view.identity], &again.logical.views.0[&view.identity]), "roots retain one typed view backing through shared handles");
            assert!(!std::sync::Arc::ptr_eq(&closure.logical.views.0[&view.identity], &cold.logical.views.0[&view.identity]), "independent cold inspection earns its own backing");
            for mapping in &mappings {
                assert!(
                    std::sync::Arc::ptr_eq(
                        first.expected_copy(&mapping.physical).unwrap(),
                        repeated.expected_copy(&mapping.physical).unwrap()
                    ),
                    "the exact native physical observation is shared by its actual source owner"
                );
            }
            assert_eq!(
                audit.work.next.load(std::sync::atomic::Ordering::Relaxed),
                next,
                "a repeated exact selection adds no first-miss worker"
            );
            assert_eq!(audit.observations(), (1, 0));

            let mut changed = mappings.clone();
            changed[0].incarnation += 1;
            let mut changed_scope = source.inspection_scope(&reader, &budget).unwrap();
            assert!(
                inspect_closure_scoped(
                    &reader,
                    view,
                    &changed,
                    &budget,
                    &audit,
                    &mut changed_scope
                )
                .await
                .is_err(),
                "same logical descriptor cannot authorize a changed selected physical incarnation"
            );
            let foreign_budget = ResourceBudget::fixed(256 << 20).unwrap();
            assert!(
                audit
                    .protected_source(&client, &foreign_budget)
                    .await
                    .is_err()
            );
            // Earn a genuine source context, then transfer exact contributor
            // protection before independently dropping that source and its pin.
            let handoff_audit = InstallationAudit::new(client.clone(), &budget).await.unwrap();
            let selected = std::sync::Arc::new(crate::installed_view::select_live_view_scoped(
                client.clone(), store.attempt(), view, &mappings, &budget, &handoff_audit,
            ).await.unwrap());
            assert!(selected.earned_source().unwrap().has_checked_context_lifetime(), "actual source selection earned Some checked lifetime");
            let source_pin = std::sync::Arc::downgrade(&handoff_audit.pin);
            let receiver_pin = crate::control::ReaderPin::acquire(client.clone(), &[]).await.unwrap();
            let received = crate::installed_view::receiving::protect_receiving_views(
                std::slice::from_ref(&selected), &receiver_pin, &reader, &budget,
            ).await.unwrap();
            let received_view = received.views()[0].clone();
            assert!(!received_view.earned_source().unwrap().has_checked_context_lifetime(), "receiving pin cannot inherit source context provenance");
            handoff_audit.close().await.unwrap();
            drop(selected);
            drop(handoff_audit);
            assert!(source_pin.upgrade().is_none(), "receiving preparation retains no source pin or context owner");
            received_view.ensure_borrowed_current(&reader, &budget).await.unwrap();
            received_view.ensure_current(&reader, &budget).await.unwrap();
            assert!(received.ensure_for(&received_view, &client, &budget).unwrap());
            drop(received_view);
            drop(received);
            receiver_pin.release().await.unwrap();
            audit.close().await.unwrap();
            assert!(audit.protected_source(&client, &budget).await.is_err());
            assert!(
                inspect_closure_scoped(&reader, view, &mappings, &budget, &audit, &mut repeated)
                    .await
                    .is_err(),
                "already prepared selectors cannot bypass actual source closure"
            );

            store
                .bind(d::completed::CompletedBinding {
                    boundary: None,
                    source: SourceSnapshot::of_completed_view(
                        &relation,
                        closure.logical.contributions()[&mappings[0].logical]
                            .spec
                            .model,
                        view,
                    )
                    .unwrap(),
                    view: view.clone(),
                    configuration: None,
                })
                .await
                .unwrap();
            store.freeze_content(&budget).await.unwrap();
            let physical = store.complete_physical_content(&budget).await.unwrap();
            // The compiler-store dev dependency links a second crate instance in
            // unit builds. Convert only expected completion data, never its owner.
            let completion: crate::completed_state::FrozenCompletion =
                serde_json::from_value(serde_json::to_value(physical.completion()).unwrap())
                    .unwrap();
            crate::completed_state::inspect_inventory(&reader, &completion, &budget)
                .await
                .unwrap();
            let parent = completion.record().unwrap();
            for table in ["native_completed_contributor", "native_completed_view"] {
                let mut variables = Variables::new();
                variables.insert("completed_parent", parent.clone());
                let children: Vec<Value> = reader
                    .query_native(
                        &format!("SELECT * FROM {table} WHERE completion=$completed_parent ORDER BY id"),
                        variables,
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    children.len(),
                    1,
                    "fixture has one independently checked {table} child"
                );
                let id = RecordId::from_value(
                    children[0].as_object().unwrap().get("id").unwrap().clone(),
                )
                .unwrap();
                let hold = crate::control::hold_record_id(&parent, &id).unwrap();
                for (kind, target) in [("guarded inventory child", id), ("exact parent hold", hold)]
                {
                    let mut variables = Variables::new();
                    variables.insert("id", target);
                    let rows: Vec<Value> = reader
                        .query_native("SELECT * FROM $id", variables.clone())
                        .await
                        .unwrap();
                    assert_eq!(rows.len(), 1);
                    client
                        .query("DELETE $id RETURN NONE")
                        .bind(variables.clone())
                        .await
                        .unwrap()
                        .check()
                        .unwrap();
                    let received = crate::completed_state::hold_physical_state_for_attempt(
                        client.clone(),
                        store.attempt(),
                        &budget,
                    )
                    .await;
                    variables.insert("row", rows[0].clone());
                    client
                        .query("UPSERT $id CONTENT $row RETURN NONE")
                        .bind(variables)
                        .await
                        .unwrap()
                        .check()
                        .unwrap();
                    let refused = received.err().expect("missing inventory item must refuse");
                    assert!(
                        !refused.to_string().contains("recovery"),
                        "complete actual {table} inventory must refuse missing {kind}, not its operation owner: {refused:?}"
                    );
                }
            }
            // A successful fresh physical receiver reaches its final exact owner CAS;
            // it earns no semantic admission from the data/descriptor inspected here.
            let received = crate::completed_state::hold_physical_state_for_attempt(
                client.clone(),
                store.attempt(),
                &budget,
            )
            .await
            .unwrap();
            assert_eq!(received.completion(), &completion);
            assert!(received._charge.reserved() <= std::mem::size_of::<crate::completed_state::NativeCompletedState>()
                + crate::selection::node_bytes(received.held.owner()) + 2 * std::mem::size_of::<usize>(),
                "physical return owns its actual retained wrapper, not departed decode/builder scratch");
            let received = std::sync::Arc::new(received);
            let selected = crate::installed_view::hold_physical_view(client.clone(), received.clone(), view.identity, &budget).await.unwrap();
            assert!(std::sync::Arc::ptr_eq(&selected.retained_completion().unwrap().selection, &received.inventory().installed(view.identity).unwrap().selection),
                "retained cap and independently checked inventory share one exact selector payload");
            assert!(crate::installed_view::hold_physical_view(client.clone(), received.clone(), view.identity, &foreign_budget).await.is_err());
            let foreign_client = std::sync::Arc::new(surrealdb::Surreal::init());
            assert!(crate::installed_view::hold_physical_view(foreign_client, received, view.identity, &budget).await.is_err());

        })
        .catch_unwind()).await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step(
            "protected source inventory reader close",
            reader.close().await,
        );
        cleanup.step("protected source inventory pin close", audit.close().await);
        cleanup.step(
            "protected source inventory attempt cleanup",
            store.abandon().await,
        );
        let observer_completion = observer_lease.complete_with(&cleanup).await;
        cleanup.step(
            "native audit observer owner completion",
            observer_completion,
        );
        drop(observer_lease);
        cleanup.step(
            "native audit observer factory drain",
            observer_factory
                .drain(&crate::control::RetryCancellation::default())
                .await,
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("protected source inventory cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn final_parent_incarnation_change_after_inventory_eof_refuses_completion() {
        use crate::completed_state::{
            FINAL_PARENT_OBSERVATION_BARRIER, FinalParentObservationBarrier,
        };
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(
            &config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        // Keep the exact owned guard outside the assertion body so even a failed
        // assertion restores its incarnation before attempt cleanup.
        let mut restore = None;
        let result = std::panic::AssertUnwindSafe(async {
            let relation = Relation::of::<Package>();
            let mut specification = contribution("final-parent-inventory-race", vec![]).spec;
            specification.outputs = [relation.name().into()].into();
            let model = specification.model;
            let producer = store.begin_contribution(specification).await.unwrap();
            store
                .write_batch(
                    &producer,
                    &relation,
                    &Package::encode(&[Package {
                        name: format!("final-parent-{}", store.attempt().hex()),
                    }])
                    .unwrap(),
                    &budget,
                )
                .await
                .unwrap();
            let views = store
                .complete_contribution(
                    producer,
                    d::stages::ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &BTreeMap::new(),
                    &budget,
                )
                .await
                .unwrap();
            let view = &views[relation.name()];
            store
                .bind(d::completed::CompletedBinding {
                    boundary: None,
                    source: SourceSnapshot::of_completed_view(
                        &relation,
                        model,
                        view,
                    )
                    .unwrap(),
                    view: view.clone(),
                    configuration: None,
                })
                .await
                .unwrap();
            store.freeze_content(&budget).await.unwrap();
            let physical = store.complete_physical_content(&budget).await.unwrap();
            // The dev dependency has its own crate instance: transfer only data,
            // and earn the actual receiver's native authority independently.
            let completion: crate::completed_state::FrozenCompletion =
                serde_json::from_value(serde_json::to_value(physical.completion()).unwrap())
                    .unwrap();
            let parent = completion.record().unwrap();
            let guard = crate::control::guard_id(&Value::RecordId(parent.clone())).unwrap();
            let mut variables = Variables::new();
            variables.insert("guard", guard.clone());
            let rows: Vec<surrealdb::types::Object> = reader
                .query_native("SELECT * FROM $guard", variables.clone())
                .await
                .unwrap();
            let [original] = rows.as_slice() else {
                panic!("owned physical parent must have exactly one guard");
            };
            let incarnation = i64::from_value(original.get("incarnation").unwrap().clone())
                .unwrap();
            assert!(incarnation > 0);
            restore = Some((guard.clone(), original.clone()));
            variables.insert("incarnation", incarnation.checked_add(1).unwrap());
            let barrier = FinalParentObservationBarrier {
                observed: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
                resume: std::sync::Arc::new(tokio::sync::Barrier::new(2)),
            };
            let receiving = FINAL_PARENT_OBSERVATION_BARRIER.scope(
                barrier.clone(),
                crate::completed_state::hold_physical_state_for_attempt(
                    client.clone(),
                    store.attempt(),
                    &budget,
                ),
            );
            tokio::pin!(receiving);
            // An unexpected early refusal must reach cleanup rather than leave
            // the mutator waiting forever for a checkpoint the receiver missed.
            tokio::select! {
                early = receiving.as_mut() => match early {
                    Ok(_) => panic!("receiver returned authority before the final parent checkpoint"),
                    Err(error) => panic!("receiver failed before the final parent checkpoint: {error:?}"),
                },
                _ = barrier.observed.wait() => {}
            }
            // The first successful hold legitimately advances the guard revision.
            // Compare the mutation against the checkpoint state, not the state
            // before that hold was earned.
            let checkpoint: Vec<surrealdb::types::Object> = reader
                .query_native("SELECT * FROM $guard", variables.clone())
                .await
                .unwrap();
            let [checkpoint] = checkpoint.as_slice() else {
                panic!("the final parent checkpoint must retain exactly one guard");
            };
            assert_eq!(checkpoint.get("incarnation"), original.get("incarnation"));
            restore = Some((guard.clone(), checkpoint.clone()));
            let mutate = async {
                // This task-local checkpoint is reached only after inventory EOF,
                // exact child/hold checks, and the last parent observation.
                let changed = client
                    .query("UPDATE $guard SET incarnation=$incarnation RETURN NONE")
                    .bind(variables.clone())
                    .await
                    .and_then(|response| response.check());
                // Always release the receiver before reporting mutation failure.
                barrier.resume.wait().await;
                changed
            };
            let (received, changed) = tokio::join!(receiving.as_mut(), mutate);
            changed.unwrap();
            let error = match received {
                Ok(_) => panic!("changed parent incarnation must never return completion authority"),
                Err(error) => error,
            };
            assert!(
                error.to_string().contains("physical completed hold changed"),
                "the final checked native effect must refuse the parent race: {error:?}"
            );
            let actual: Vec<surrealdb::types::Object> = reader
                .query_native("SELECT * FROM $guard", variables)
                .await
                .unwrap();
            let mut expected = checkpoint.clone();
            expected.insert("incarnation", incarnation + 1);
            assert_eq!(actual, vec![expected], "only the owned parent incarnation changed");
        })
        .catch_unwind()
        .await;
        let mut cleanup = d::completion::Completion::default();
        if let Some((guard, original)) = restore {
            let mut variables = Variables::new();
            variables.insert("guard", guard);
            variables.insert("row", original);
            cleanup.step(
                "final parent exact guard restoration",
                async {
                    client
                        .query("UPSERT $guard CONTENT $row RETURN NONE")
                        .bind(variables)
                        .await
                        .map_err(ModelError::codec)?
                        .check()
                        .map_err(ModelError::codec)?;
                    Ok(())
                }
                .await,
            );
        }
        cleanup.step("final parent inventory reader close", reader.close().await);
        cleanup.step(
            "final parent inventory attempt cleanup",
            store.abandon().await,
        );
        cleanup.step(
            "final parent inventory observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("final parent inventory cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn late_membership_terminal_failure_cannot_publish_reusable_observation() {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let store = NativeCompilerStore::begin(
            &config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(&config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let (client, observer_factory, observer_lease) = inspection_session(&budget).await;
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let fused_audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let failed_blocking_audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let result = crate::control::with_recovery_scope(observer_lease.recovery_scope().clone(), std::panic::AssertUnwindSafe(async{
            let relation=Relation::of::<Package>();let mut specification=contribution("late-terminal",vec![]).spec;specification.outputs=[relation.name().into()].into();
            let producer=store.begin_contribution(specification).await.unwrap();
            let rows=(0..129).map(|index|Package{name:format!("terminal-package-{index}")}).collect::<Vec<_>>();
            store.write_batch(&producer,&relation,&Package::encode(&rows).unwrap(),&budget).await.unwrap();
            let views=store.complete_contribution(producer,d::stages::ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new(),&budget).await.unwrap();
            let view=&views[relation.name()];let held=store.installed_view(view,&budget).await.unwrap();
            let mappings:Vec<ContributorRealization>=serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap()).unwrap();
            let closure=inspect_closure(&reader,view,&mappings,&budget).await.unwrap();
            let failed=LATE_MEMBERSHIP_FAILURE.scope(true,audit_memberships_scoped(&reader,&closure,&budget,&audit)).await.unwrap_err();
            assert!(format!("{failed:?}").contains("late independent membership audit control"),"{failed:?}");
            assert_eq!(audit.observations(),(0,0),"valid provisional membership never becomes a complete observation before its physical terminal");
            audit_memberships_scoped(&reader,&closure,&budget,&audit).await.unwrap();assert_eq!(audit.observations(),(1,0));
            let selected = selected_prepared_members(&closure, view, &budget, &audit).await.unwrap();
            let mut selected = selected.cursor_with_budget(&budget).unwrap();
            let mut selected_count = 0;
            while selected.next_row().unwrap().is_some() { selected_count += 1; }
            assert_eq!(selected_count, rows.len());
            assert_eq!(audit.logical_contribution_observations(), 0,
                "complete selected memberships demand no full logical backing preparation");

            let aliases = audit_aliases_scoped(&reader, &closure, &budget, &audit).await.unwrap();
            let cold_aliases = audit_aliases(&reader, &closure, &budget).await.unwrap();
            let cold = logical_roots(&reader, &closure, &[], &cold_aliases, &budget).await.unwrap();
            let mut fused_inspections = ClosureInspectionScope::new(&fused_audit, &budget).unwrap();
            let fused_aliases = assure_boundary_closure(&reader, &closure, &budget, &fused_audit, &mut fused_inspections, true).await.unwrap();
            let fused = logical_roots_scoped(&reader, &closure, &[], &fused_aliases, &budget, &fused_audit).await.unwrap();
            assert_eq!(fused, cold, "fresh full demand preserves independently reconstructed roots");
            assert_eq!(fused_audit.observations(), (1, 0), "fresh full demand fuses contributor assurance once");
            assert_eq!(fused_audit.logical_contribution_observations(), 1);
            assert_eq!(logical_roots_scoped(&reader, &closure, &[], &fused_aliases, &budget, &fused_audit).await.unwrap(), cold);
            assert_eq!(fused_audit.logical_contribution_observations(), 1, "full reuse cannot normalize the contributor twice");

            let failed = LATE_MEMBERSHIP_FAILURE.scope(true,
                logical_roots_scoped(&reader, &closure, &[], &aliases, &budget, &audit),
            ).await.unwrap_err();
            assert!(format!("{failed:?}").contains("late logical backing preparation control"), "{failed:?}");
            assert_eq!(audit.logical_contribution_observations(), 0);
            assert!(audit.logical_contributors.is_empty(), "provisional normalized rows cannot publish a fragment before terminal success");
            let overlapping = vec![&closure; 17];
            assert_eq!(logical_roots_many_scoped(&reader, &overlapping, &[], &aliases, &budget, &audit).await.unwrap(), cold,
                "overlapping selectors preserve the independently reconstructed six-family identity");
            assert_eq!(audit.logical_contribution_observations(), 1);
            let mut streamed = LogicalFamilyInputs::new(&budget).unwrap();
            let mut retained = None;
            for _ in 0..17 {
                let fresh = inspect_closure_in_audit(&reader, view, &mappings, &budget, &audit).await.unwrap();
                let before = budget.reserved();
                let ingredients = streamed.charge.reserved();
                streamed.include_closure(&reader, &fresh, &aliases, &budget, &audit).unwrap();
                if retained.is_some() {
                    assert_eq!(streamed.charge.reserved(), ingredients,
                        "overlapping roots retain each semantic ingredient once");
                    assert_eq!(budget.reserved(), before,
                        "duplicate inclusion borrows the existing payload arenas");
                }
                drop(fresh);
                if let Some(retained) = retained {
                    assert_eq!(budget.reserved(), retained,
                        "each independently inspected root releases its closure before the next");
                } else {
                    retained = Some(budget.reserved());
                }
            }
            assert_eq!(streamed.finish_scoped(&reader, &[], &budget, &audit).await.unwrap(), cold);
            assert_eq!(audit.logical_contribution_observations(), 1);
            assert_eq!(logical_roots_scoped(&reader, &closure, &[], &aliases, &budget, &audit).await.unwrap(), cold);
            assert_eq!(logical_roots_scoped(&reader, &closure, &[], &aliases, &budget, &audit).await.unwrap(), cold);
            assert_eq!(audit.logical_contribution_observations(), 1, "one exact physical contributor is prepared once within its pin");
            let reused = audit_aliases_scoped(&reader, &closure, &budget, &audit).await.unwrap();
            assert_eq!(logical_roots_scoped(&reader, &closure, &[], &reused, &budget, &audit).await.unwrap(), cold);
            assert_eq!(audit.alias_observations(), 1);
            let failure = failed_blocking_audit.blocking(|_| Err::<(), _>(ModelError::Conflict("native blocking semantic control"))).await.unwrap_err();
            assert!(format!("{failure:?}").contains("native blocking semantic control"));
            let closed = failed_blocking_audit.close().await.unwrap_err();
            assert!(format!("{closed:?}").contains("native blocking semantic control"),
                "actual blocking terminal failure remains observable during close");
            assert!(failed_blocking_audit.pin.ensure_live().is_err(),
                "confirmed semantic worker failure still permits authenticated native pin release");

        }).catch_unwind()).await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("terminal audit reader close", reader.close().await);
        cleanup.step("terminal audit pin close", audit.close().await);
        cleanup.step("terminal fused audit pin close", fused_audit.close().await);
        cleanup.step(
            "terminal failed blocking audit final close",
            failed_blocking_audit.close().await,
        );
        cleanup.step("terminal audit attempt cleanup", store.abandon().await);
        let observer_completion = observer_lease.complete_with(&cleanup).await;
        cleanup.step(
            "native audit observer owner completion",
            observer_completion,
        );
        drop(observer_lease);
        cleanup.step(
            "native audit observer factory drain",
            observer_factory
                .drain(&crate::control::RetryCancellation::default())
                .await,
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("terminal audit cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn set_audit_requires_exact_backing_guards_holds_and_endpoint_inventory() {
        use d::input::Release;
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let store = NativeCompilerStore::begin(
            &config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(&config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let (client, observer_factory, observer_lease) = inspection_session(&budget).await;
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let result = crate::control::with_recovery_scope(observer_lease.recovery_scope().clone(), std::panic::AssertUnwindSafe(async{
            let packages=Relation::of::<Package>();let releases=Relation::of::<Release>();let relations=[packages.clone(),releases.clone()];
            let mut specification=contribution("set-correspondence",vec![]).spec;specification.outputs=relations.iter().map(|relation|relation.name().into()).collect();
            let producer=store.begin_contribution(specification).await.unwrap();
            let package=Package{name:format!("set-correspondence-{}",store.attempt().hex())};let release=Release{package:package.id(),version:"1".into()};
            store.write_batch(&producer,&packages,&Package::encode(&[package]).unwrap(),&budget).await.unwrap();store.write_batch(&producer,&releases,&Release::encode(std::slice::from_ref(&release)).unwrap(),&budget).await.unwrap();
            let views=store.complete_contribution(producer,d::stages::ProviderOutcome::Complete,&relations,&BTreeMap::new(),&budget).await.unwrap();
            let view=&views[releases.name()];let held=store.installed_view(view,&budget).await.unwrap();
            let mappings:Vec<ContributorRealization>=serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap()).unwrap();
            let closure=inspect_closure(&reader,view,&mappings,&budget).await.unwrap();let owner=RecordId::new("compiler_contribution",mappings[0].physical.hex());
            let backing=crate::loader::entity_payload_id(&d::graph::Entity::from(release.clone())).unwrap();
            let endpoints=crate::loader::entity_endpoint_values(&d::graph::Entity::from(release)).unwrap();
            let endpoint=RecordId::from_value(endpoints[0].as_object().unwrap().get("id").unwrap().clone()).unwrap();
            let mut ids=vec![("backing",backing.clone()),("backing guard",crate::control::guard_id(&Value::RecordId(backing.clone())).unwrap()),("endpoint",endpoint.clone()),("endpoint guard",crate::control::guard_id(&Value::RecordId(endpoint.clone())).unwrap())];
            for (kind,object) in [("backing hold",backing),("endpoint hold",endpoint)]{
                let mut variables=Variables::new();variables.insert("owner",owner.clone());variables.insert("object",object);
                let rows:Vec<RecordId>=reader.query_native("SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object=$object",variables).await.unwrap();assert_eq!(rows.len(),1);ids.push((kind,rows[0].clone()));
            }
            for (kind,id) in ids{
                let audit=InstallationAudit::new(client.clone(),&budget).await.unwrap();audit.pin.protect(owner.clone()).await.unwrap();
                let mut variables=Variables::new();variables.insert("id",id.clone());let original:Vec<Value>=reader.query_native("SELECT * FROM $id",variables.clone()).await.unwrap();assert_eq!(original.len(),1);
                client.query("DELETE $id RETURN NONE").bind(variables.clone()).await.unwrap().check().unwrap();
                let failed=audit_memberships_scoped(&reader,&closure,&budget,&audit).await;
                variables.insert("row",original[0].clone());client.query("UPSERT $id CONTENT $row RETURN NONE").bind(variables).await.unwrap().check().unwrap();
                let observations=audit.observations();audit.close().await.unwrap();
                let failed=failed.expect_err("missing set correspondence must refuse");
                let cause=match kind {
                    "backing"=>"installed membership backing missing",
                    "backing guard"|"endpoint guard"=>"installed set guard missing",
                    "endpoint"=>"installed canonical endpoint missing before freeze",
                    "backing hold"|"endpoint hold"=>"installed set hold missing",
                    _=>unreachable!("declared set-correspondence defect"),
                };
                assert!(failed.to_string().contains(cause),"missing {kind} must refuse its actual correspondence: {failed:?}");assert_eq!(observations,(0,0),"failed {kind} audit cannot publish reuse");
            }
        }).catch_unwind()).await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("set correspondence reader close", reader.close().await);
        cleanup.step("set correspondence attempt cleanup", store.abandon().await);
        let observer_completion = observer_lease.complete_with(&cleanup).await;
        cleanup.step(
            "native audit observer owner completion",
            observer_completion,
        );
        drop(observer_lease);
        cleanup.step(
            "native audit observer factory drain",
            observer_factory
                .drain(&crate::control::RetryCancellation::default())
                .await,
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("set correspondence cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn streamed_family_union_preserves_selector_local_copies_bindings_and_late_refusal() {
        use lctx_compiler_store::compiler::NativeCompilerStore;
        use std::sync::Arc;
        async fn produce(
            store: &Arc<NativeCompilerStore>,
            name: &str,
            inputs: Vec<SourceSnapshot>,
            budget: &ResourceBudget,
        ) -> (CompletedView, Vec<ContributorRealization>) {
            let relation = Relation::of::<Package>();
            let mut spec = contribution(name, inputs).spec;
            spec.outputs = [relation.name().into()].into();
            let producer = store.begin_contribution(spec).await.unwrap();
            store
                .write_batch(
                    &producer,
                    &relation,
                    &Package::encode(&[Package { name: name.into() }]).unwrap(),
                    budget,
                )
                .await
                .unwrap();
            let views = store
                .complete_contribution(
                    producer,
                    d::stages::ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &BTreeMap::new(),
                    budget,
                )
                .await
                .unwrap();
            let view = views[relation.name()].clone();
            let held = store.installed_view(&view, budget).await.unwrap();
            let mappings =
                serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap())
                    .unwrap();
            (view, mappings)
        }
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let first = NativeCompilerStore::begin(
            &config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(&config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let second = NativeCompilerStore::begin(
            &config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(&config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let (client, observer_factory, observer_lease) = inspection_session(&budget).await;
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let result = crate::control::with_recovery_scope(observer_lease.recovery_scope().clone(), std::panic::AssertUnwindSafe(async {
            let (dependency, dependency_mappings) = produce(&first, "shared-family-dependency", vec![], &budget).await;
            let (other_dependency, other_mappings) = produce(&second, "shared-family-dependency", vec![], &budget).await;
            assert_eq!(dependency, other_dependency);
            assert_eq!(dependency_mappings[0].logical, other_mappings[0].logical);
            assert_ne!(dependency_mappings[0].physical, other_mappings[0].physical,
                "one logical dependency has two actual physical realizations");
            let source = SourceSnapshot::of_completed_view(&Relation::of::<Package>(), ContentHash::of(b"model"), &dependency).unwrap();
            let (a, a_mappings) = produce(&first, "family-root-a", vec![source.clone()], &budget).await;
            let (b, b_mappings) = produce(&second, "family-root-b", vec![source], &budget).await;
            assert_ne!(a_mappings.iter().find(|mapping| mapping.view == dependency.identity).unwrap().physical,
                b_mappings.iter().find(|mapping| mapping.view == dependency.identity).unwrap().physical);
            let mut closures = Vec::new();
            let mut alias_runs = Vec::new();
            for (view, mappings) in [(&a, &a_mappings), (&b, &b_mappings)] {
                let closure = inspect_closure(&reader, view, mappings, &budget).await.unwrap();
                audit_memberships_scoped(&reader, &closure, &budget, &audit).await.unwrap();
                let aliases = audit_aliases_scoped(&reader, &closure, &budget, &audit).await.unwrap();
                prepare_closure_in_context(&reader, &closure, &budget, &audit).await.unwrap();
                alias_runs.push(aliases);
                closures.push(closure);
            }
            let aliases = crate::ordered_rows::PreparedRows::union_prepared(&alias_runs, &budget, || Ok(())).unwrap();
            let cold = logical_roots_many(&reader, &closures, &[], &aliases, &budget).await.unwrap();
            for order in [[0, 1], [1, 0]] {
                let mut streamed = LogicalFamilyInputs::new(&budget).unwrap();
                for index in order {
                    streamed.include_closure(&reader, &closures[index], &alias_runs[index], &budget, &audit).unwrap();
                }
                assert_eq!(streamed.owners.len(), 4, "both physical dependency copies survive normalization");
                assert_eq!(streamed.views.len(), 3, "shared logical dependency view is unique");
                assert_eq!(streamed.finish_scoped(&reader, &[], &budget, &audit).await.unwrap(), cold,
                    "selector decomposition and order preserve independent cold six-family identity");
            }
            let binding = d::completed::CompletedBinding {
                boundary: None,
                source: SourceSnapshot::of_completed_view(&Relation::of::<Package>(), ContentHash::of(b"model"), &a).unwrap(),
                view: a.clone(), configuration: None,
            };
            let expected = logical_roots(&reader, &closures[0], std::slice::from_ref(&binding), &alias_runs[0], &budget).await.unwrap();
            // Both roots and the dependency are physically inventoried, but only
            // the declared root supplies identity-bearing semantic families.
            let mut selections = vec![
                crate::completed_state::ViewSelection { view: a.clone(), mappings: a_mappings.clone() },
                crate::completed_state::ViewSelection { view: b.clone(), mappings: b_mappings.clone() },
                crate::completed_state::ViewSelection { view: dependency.clone(), mappings: dependency_mappings },
            ];
            selections.sort_by_key(|selection| selection.view.identity);
            let retained = crate::completed_state::complete_retained_content_scoped(client.clone(), first.attempt(),
                std::slice::from_ref(&binding), &selections, &budget, &audit).await.unwrap();
            assert_eq!(&retained.completion().roots, &expected);
            assert_eq!(retained.inventory().references().len(), 3);
            assert_eq!(retained.inventory().copies().len(), 4);
            retained.close().await.unwrap();
            // A late contradictory semantic ingredient cannot yield a prefix root.
            let mut failed = LogicalFamilyInputs::new(&budget).unwrap();
            failed.include_closure(&reader, &closures[0], &alias_runs[0], &budget, &audit).unwrap();
            let id = closures[0].contributors.values().next().unwrap().record().clone();
            *failed.incarnations.get_mut(&id).unwrap() += 1;
            assert!(failed.include_closure(&reader, &closures[0], &alias_runs[0], &budget, &audit).is_err());
            assert!(failed.finish_scoped(&reader, &[], &budget, &audit).await.is_err());
            let mut failed = LogicalFamilyInputs::new(&budget).unwrap();
            failed.include_closure(&reader, &closures[0], &alias_runs[0], &budget, &audit).unwrap();
            failed.owners.get_mut(&id).unwrap().outcome = 2;
            assert!(failed.include_closure(&reader, &closures[0], &alias_runs[0], &budget, &audit).is_err());
            assert!(failed.finish_scoped(&reader, &[], &budget, &audit).await.is_err());
            // An unbound selector is still independently checked before replay or
            // publication. Restore deliberately changed fixture bytes before asserting.
            let b_id = RecordId::new("compiler_view", b.identity.hex());
            let mut inventory_vars = Variables::new();
            inventory_vars.insert("attempt", RecordId::new("native_attempt", first.attempt().hex()));
            let before: Vec<Value> = reader.query_native("SELECT object FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt AND record::table(object)='native_completed_state' ORDER BY id", inventory_vars.clone()).await.unwrap();
            let mut vars = Variables::new(); vars.insert("id", b_id);
            let rows: Vec<Value> = reader.query_native("SELECT * FROM $id", vars.clone()).await.unwrap();
            let mut changed = rows[0].clone();
            let Value::Object(changed_row) = &mut changed else { panic!("native view object"); };
            changed_row.insert("descriptor", surrealdb::types::Bytes::from(b"invalid late unbound view".to_vec()));
            vars.insert("row", changed);
            client.query("UPSERT $id CONTENT $row RETURN NONE").bind(vars.clone()).await.unwrap().check().unwrap();
            let mut late_binding = binding.clone();
            late_binding.boundary = Some("late-unbound-refusal".into());
            let refused = crate::completed_state::complete_retained_content_scoped(client.clone(), first.attempt(),
                std::slice::from_ref(&late_binding), &selections, &budget, &audit).await;
            vars.insert("row", rows[0].clone());
            client.query("UPSERT $id CONTENT $row RETURN NONE").bind(vars).await.unwrap().check().unwrap();
            let refused = match refused {
                Err(error) => {
                    assert!(!error.to_string().contains("recovery"), "invalid unbound selector must reach its actual input refusal: {error:?}");
                    true
                },
                Ok(retained) => { retained.close().await.unwrap(); false }
            };
            let after: Vec<Value> = reader.query_native("SELECT object FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt AND record::table(object)='native_completed_state' ORDER BY id", inventory_vars).await.unwrap();
            assert!(refused, "unbound invalid selector cannot be hidden by semantic root filtering");
            assert_eq!(after, before, "late selector refusal publishes no prefix boundary");
        }).catch_unwind()).await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("streamed families reader close", reader.close().await);
        cleanup.step("streamed families audit close", audit.close().await);
        cleanup.step(
            "streamed families first attempt cleanup",
            first.abandon().await,
        );
        cleanup.step(
            "streamed families second attempt cleanup",
            second.abandon().await,
        );
        let observer_completion = observer_lease.complete_with(&cleanup).await;
        cleanup.step(
            "native audit observer owner completion",
            observer_completion,
        );
        drop(observer_lease);
        cleanup.step(
            "native audit observer factory drain",
            observer_factory
                .drain(&crate::control::RetryCancellation::default())
                .await,
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("streamed families cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    fn contribution(name: &str, inputs: Vec<SourceSnapshot>) -> CompletedContribution {
        CompletedContribution {
            assurance:
                lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            spec: ContributionSpec {
                captured_binding: None,
                producer: name.into(),
                profile: OutputProfile::Specific(Profile::Catalog),
                model: ContentHash::of(b"model"),
                implementation: ContentHash::of(b"implementation"),
                configuration: None,
                inputs,
                outputs: ["packages".into()].into(),
            },
            outcome: 0,
            outputs: [(
                "packages".into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty membership EOF"),
                },
            )]
            .into(),
        }
    }
    fn closure() -> (RecoveryClosure, Vec<ContributorRealization>) {
        let dependency = contribution("dependency", vec![]);
        let dep_id = dependency.identity().unwrap();
        let dependency_view = CompletedView::new("packages".into(), [dep_id].into(), 0).unwrap();
        let input = SourceSnapshot::of_completed_view(
            &Relation::of::<Package>(),
            dependency.spec.model,
            &dependency_view,
        )
        .unwrap();
        let root = contribution("root", vec![input]);
        let root_id = root.identity().unwrap();
        let root_view = CompletedView::new("packages".into(), [root_id].into(), 0).unwrap();
        let mut traversal = RecoveryTraversal::for_views(vec![root_view.clone()]).unwrap();
        let requested = traversal.next_view().unwrap().unwrap();
        traversal
            .include_view(&requested, root_view, vec![root], |_| {
                Ok(dependency_view.clone())
            })
            .unwrap();
        let requested = traversal.next_view().unwrap().unwrap();
        traversal
            .include_view(
                &requested,
                dependency_view,
                vec![dependency],
                |_| unreachable!(),
            )
            .unwrap();
        assert!(traversal.next_view().unwrap().is_none());
        let closure = traversal.finish().unwrap();
        let mappings = closure
            .views()
            .values()
            .flat_map(|view| {
                view.contributions
                    .iter()
                    .map(move |logical| (view.identity, *logical))
            })
            .enumerate()
            .map(|(i, (view, logical))| ContributorRealization {
                view,
                logical,
                physical: ContentHash::of(format!("exact-copy-{i}").as_bytes()),
                incarnation: 1,
            })
            .collect();
        (closure, mappings)
    }
    #[test]
    fn exact_dependency_mapping_retains_empty_premise_and_refuses_missing_extra_duplicate_or_rebound_copy()
     {
        let (closure, mappings) = closure();
        assert_eq!(mappings.len(), 2);
        exact_mappings(&closure, &mappings).unwrap();
        assert!(
            closure.views().values().all(|view| view.rows == 0),
            "empty dependencies are still closure authority"
        );
        for defect in [
            "missing",
            "extra",
            "duplicate",
            "physical_alias",
            "incarnation",
        ] {
            let mut changed = mappings.clone();
            match defect {
                "missing" => {
                    changed.pop();
                }
                "extra" => {
                    changed.push(ContributorRealization {
                        view: ContentHash::of(b"unrequested view"),
                        logical: ContentHash::of(b"unrequested logical owner"),
                        physical: ContentHash::of(b"unrequested copy"),
                        incarnation: 1,
                    });
                    changed.sort_by_key(|mapping| (mapping.view, mapping.logical));
                }
                "duplicate" => {
                    changed.push(changed[0].clone());
                    changed.sort_by_key(|mapping| (mapping.view, mapping.logical));
                }
                "physical_alias" => {
                    changed[1].physical = changed[0].physical;
                }
                _ => {
                    changed[0].incarnation = 0;
                }
            }
            assert!(
                exact_mappings(&closure, &changed).is_err(),
                "exact closure rejects {defect}"
            );
        }
        let root = closure
            .views()
            .values()
            .find(|view| {
                view.contributions
                    .iter()
                    .any(|logical| !closure.contributions()[logical].spec.inputs.is_empty())
            })
            .unwrap();
        let before = super::super::realization_id(root, &mappings).unwrap();
        let mut alternate = mappings.clone();
        alternate[0].physical = ContentHash::of(b"another valid copy");
        exact_mappings(&closure, &alternate).unwrap();
        assert_ne!(
            before,
            super::super::realization_id(root, &alternate).unwrap(),
            "two exact valid copies have different physical namespaces"
        );
    }
    #[test]
    fn equivalent_copies_are_selected_per_view_instead_of_collapsing_the_closure() {
        let mut shared = contribution("shared", vec![]);
        shared.spec.outputs.insert("releases".into());
        shared
            .outputs
            .insert("releases".into(), shared.outputs["packages"].clone());
        let logical = shared.identity().unwrap();
        let views = ["packages", "releases"]
            .map(|relation| CompletedView::new(relation.into(), [logical].into(), 0).unwrap());
        let mut traversal = RecoveryTraversal::for_views(views.to_vec()).unwrap();
        while let Some(view) = traversal.next_view().unwrap() {
            traversal
                .include_view(
                    &view,
                    view.clone(),
                    vec![shared.clone()],
                    |_| unreachable!(),
                )
                .unwrap();
        }
        let closure = traversal.finish().unwrap();
        let mut mappings = views
            .iter()
            .enumerate()
            .map(|(index, view)| ContributorRealization {
                view: view.identity,
                logical,
                physical: ContentHash::of(format!("copy-{index}").as_bytes()),
                incarnation: 1,
            })
            .collect::<Vec<_>>();
        mappings.sort_by_key(|mapping| (mapping.view, mapping.logical));
        exact_mappings(&closure, &mappings).unwrap();
        assert_ne!(
            mappings[0].physical, mappings[1].physical,
            "each selected view retains its exact chosen copy"
        );
        mappings[1].physical = mappings[0].physical;
        exact_mappings(&closure, &mappings).unwrap();
        mappings[1].incarnation += 1;
        assert!(
            exact_mappings(&closure, &mappings).is_err(),
            "one physical record cannot claim two current incarnations"
        );
        mappings[1].incarnation = 1;
        mappings[1].view = mappings[0].view;
        assert!(
            exact_mappings(&closure, &mappings).is_err(),
            "duplicate incidence is never an alternate choice"
        );
    }
}

/// Only alias rows owned by the supplied completed contributor set are closure
/// authority. Global aliases with the same source are intentionally never queried.
pub(crate) async fn audit_aliases_scoped(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    audit.ensure_context(reader, budget).await?;
    audit_aliases_in_context(reader, closure, budget, audit).await
}
async fn audit_aliases_in_context(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    audit.check(reader, budget)?;
    let mut charge = StateCharge::new(budget, "installed alias exact contributor set");
    charge.grow(
        closure
            .contributors
            .len()
            .saturating_mul(std::mem::size_of::<(ContentHash, i64)>())
            .saturating_add(std::mem::size_of::<ScopedAliasRows>()),
    )?;
    let mut owners = Vec::with_capacity(closure.contributors.len());
    for (physical, expected) in &closure.contributors {
        let cached = audit
            .contributors
            .get(expected.record())
            .ok_or(ModelError::Conflict(
                "alias reuse requires complete protected contributor audit",
            ))?;
        if cached.incarnation() != expected.incarnation() || cached.row() != expected.row() {
            return Err(ModelError::Conflict(
                "alias exact contributor premise changed",
            ));
        }
        owners.push((*physical, cached.incarnation()));
    }
    if let Some(previous) = audit.aliases.get(&())
        && previous.owners == owners
    {
        // inspect_closure and audit_memberships_scoped supplied current full
        // native rows for every selected physical owner. Exact set equality,
        // rather than a subset or digest, preserves whole-set reachability.
        return Ok(previous.rows.clone());
    }
    let rows = audit_aliases_inner(reader, closure, budget, audit).await?;
    // Publish only after both streams, all alias/backing checks, and whole-set
    // reachability succeeded, with every pinned immutable owner still exact.
    for expected in closure.contributors.values() {
        let current = audit
            .contributors
            .get(expected.record())
            .ok_or(ModelError::Conflict("alias protected contributor missing"))?;
        // Complete local alias runs and their immutable owner remain beneath this
        // exact live pin; composition does not need another copy of every native row.
        if current.incarnation() != expected.incarnation() || current.row() != expected.row() {
            return Err(ModelError::Conflict("alias protected contributor changed"));
        }
    }
    audit.check(reader, budget)?;
    audit.aliases.insert(
        (),
        ScopedAliasRows {
            owners,
            rows: rows.clone(),
            _charge: charge,
        },
    );
    audit
        .alias_observations
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(rows)
}

#[cfg(test)]
pub(crate) async fn audit_aliases(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    let audit = InstallationAudit::new(reader.shared_client(), budget).await?;
    let result = async {
        audit_memberships_scoped(reader, closure, budget, &audit).await?;
        audit_aliases_scoped(reader, closure, budget, &audit).await
    }
    .await;
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        audit.retain_unknown();
    }
    let mut terminal = d::completion::Completion::default();
    terminal.step("owned aliases audit close", audit.close().await);
    d::completion::complete(result, terminal)
}
async fn audit_aliases_inner(
    _reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    use d::completion::{Completion, complete};
    let mut seed_runs = Vec::new();
    let mut alias_runs = Vec::new();
    let mut run_charge = StateCharge::new(budget, "exact contributor local alias run handles");
    let mut contributors = closure.contributors.values().peekable();
    while contributors.peek().is_some() {
        let mut window = ContributorWindow::new(budget);
        for expected in contributors
            .by_ref()
            .take(crate::loader::NATIVE_WINDOW_ROWS)
        {
            if audit.local_aliases.contains_key(expected.record()) {
                audit.check(_reader, budget)?;
                continue;
            }
            let (key, input) = window.exact_capture(expected).await?;
            let worker_budget = budget.clone();
            window
                .push(async move {
                    audit
                        .run_work((2, key), move |owner| {
                            async move {
                                let reader = NativeReader::private(owner.client.clone())
                                    .with_budget(&worker_budget);
                                let result = async {
                                    let expected =
                                        worker_expected(&reader, &input, &worker_budget).await?;
                                    prepare_local_aliases(
                                        &reader,
                                        &expected,
                                        &worker_budget,
                                        &owner,
                                        true,
                                    )
                                    .await
                                }
                                .await;
                                let mut terminal = Completion::default();
                                terminal
                                    .step("local alias worker reader close", reader.close().await);
                                let prepared = complete(result, terminal)?;
                                owner.pin.ensure_live()?;
                                if let Some(prepared) = prepared {
                                    owner.charge.grow(local_alias_cache_bytes())?;
                                    owner.local_aliases.insert(input.record.clone(), prepared);
                                }
                                Ok(())
                            }
                            .boxed()
                        })
                        .await
                })
                .await?;
        }
        window.finish().await?;
    }
    // Completion order does not choose union/reachability input order.
    for expected in closure.contributors.values() {
        let previous = audit
            .local_aliases
            .get(expected.record())
            .ok_or(ModelError::Conflict("prepared local aliases absent"))?;
        run_charge
            .grow(std::mem::size_of::<crate::ordered_rows::PreparedRows>().saturating_mul(4))?;
        seed_runs.push(previous.0.clone());
        alias_runs.push(previous.1.clone());
    }
    let pin = audit.pin.clone();
    let aliases = audit
        .blocking(move |budget| {
            // Run handles and their reservation follow the actual worker, including
            // lost waiters. All union, cursor and graph scratch I/O lives here.
            let _runs = run_charge;
            pin.ensure_live()?;
            let aliases =
                crate::ordered_rows::PreparedRows::union_prepared(&alias_runs, budget, || {
                    pin.ensure_live()
                })?;
            let empty = aliases.frame_cursor(budget)?.next_frame()?.is_none();
            if !empty {
                let seeds =
                    crate::ordered_rows::PreparedRows::union_prepared(&seed_runs, budget, || {
                        pin.ensure_live()
                    })?;
                audit_alias_reachability(seeds, &aliases, budget)?;
            }
            pin.ensure_live()?;
            Ok(aliases)
        })
        .await?;
    audit.check(_reader, budget)?;
    Ok(aliases)
}
async fn prepare_local_aliases(
    reader: &NativeReader<()>,
    expected: &ExpectedRecord,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
    include_seeds: bool,
) -> Result<
    Option<(
        crate::ordered_rows::PreparedRows,
        crate::ordered_rows::PreparedRows,
    )>,
    ModelError,
> {
    prepare_local_aliases_execution(reader, expected, budget, audit, include_seeds)?.await
}
type PreparedLocalAliasRows = Option<(
    crate::ordered_rows::PreparedRows,
    crate::ordered_rows::PreparedRows,
)>;
// One adaptive alias-preparation operation retains its provisional sorters and
// cleanup through all capacity retries. Its consumers receive one erased execution.
fn prepare_local_aliases_execution<'a>(
    reader: &'a NativeReader<()>,
    expected: &'a ExpectedRecord,
    budget: &'a ResourceBudget,
    audit: &'a InstallationAudit,
    include_seeds: bool,
) -> Result<futures::future::BoxFuture<'a, Result<PreparedLocalAliasRows, ModelError>>, ModelError>
{
    let body = async move {
        let mut window = crate::loader::NATIVE_WINDOW_ROWS;
        loop {
            match prepare_local_aliases_window(
                reader,
                expected,
                budget,
                audit,
                include_seeds,
                window,
            )
            .await
            {
                // Every attempt is effect-free and owns its provisional sorters. Only a
                // terminal local capacity refusal permits replay of the same exact source.
                Err(error) if window > 1 && terminal_local_resource(&error) => {
                    window = (window / 2).max(1);
                }
                result => return result,
            }
        }
    };
    crate::continuation::admit(budget, "native local alias preparation continuation", body)
}

async fn prepare_local_aliases_window(
    reader: &NativeReader<()>,
    expected: &ExpectedRecord,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
    include_seeds: bool,
    window: usize,
) -> Result<
    Option<(
        crate::ordered_rows::PreparedRows,
        crate::ordered_rows::PreparedRows,
    )>,
    ModelError,
> {
    use d::completion::{Completion, complete};
    let owner = expected.record().clone();
    if audit.local_aliases.contains_key(&owner) {
        return Ok(None);
    }
    let mut seeds = AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES)
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))?;
    let mut aliases = AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES)
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))?;
    if include_seeds {
        let mut variables = Variables::new();
        variables.insert("owner", owner.clone());
        let mut members=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],
            vec!["SELECT node AS id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
        let result = async {
            while let Some(row) = members.next().await? {
                seeds
                    .push(row)
                    .await
                    .map_err(|error| ModelError::Cause(Box::new(error)))?;
            }
            Ok::<_, ModelError>(())
        }
        .await;
        let mut completion = Completion::default();
        completion.step(
            "installed seed membership drainage",
            members.drain_transport().await,
        );
        complete(result, completion)?;
    }
    let mut variables = Variables::new();
    variables.insert("owner", owner.clone());
    let mut holds = reader.stream_prepared(crate::prepared::PreparedQuery::new(
        variables,
        vec![],
        vec![
            "SELECT object AS id FROM native_hold WITH INDEX owner_holds WHERE owner=$owner".into(),
        ],
    )?)?;
    let result = async {
        loop {
            let mut charge = StateCharge::new(budget, "installed held alias window IDs");
            charge.grow(window.saturating_mul(std::mem::size_of::<RecordId>()))?;
            let mut ids = BTreeSet::new();
            while ids.len() < window {
                let Some(pointer) = holds.next().await? else {
                    break;
                };
                let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("id"))
                else {
                    return Err(ModelError::Schema("installed held alias pointer"));
                };
                if id.table.as_str() == "compiler_alias" && !ids.contains(id) {
                    charge.grow(crate::selection::node_bytes(id).saturating_add(256))?;
                    ids.insert(id.clone());
                }
            }
            if ids.is_empty() {
                break;
            }
            let ids = ids.into_iter().collect::<Vec<_>>();
            audit_local_alias_window(reader, &owner, &ids, budget, &mut aliases).await?;
        }
        Ok::<_, ModelError>(())
    }
    .await;
    let mut completion = Completion::default();
    completion.step(
        "installed alias hold drainage",
        holds.drain_transport().await,
    );
    complete(result, completion)?;
    let seeds = seeds
        .finish()
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))?;
    let aliases = aliases
        .finish()
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))?;
    let current = crate::control::inspect_expected_read(reader, owner.clone(), budget)
        .await?
        .ok_or(ModelError::Conflict(
            "local alias contributor disappeared after EOF",
        ))?;
    if current.incarnation() != expected.incarnation() || current.row() != expected.row() {
        return Err(ModelError::Conflict(
            "local alias contributor changed after EOF",
        ));
    }
    audit.ensure_context(reader, budget).await?;
    Ok(Some((seeds, aliases)))
}
fn local_alias_cache_bytes() -> usize {
    std::mem::size_of::<(
        RecordId,
        (
            crate::ordered_rows::PreparedRows,
            crate::ordered_rows::PreparedRows,
        ),
    )>()
    .saturating_mul(11)
    .saturating_add(1024)
}

struct AliasWindowRequests {
    targets: BTreeSet<RecordId>,
    holds: BTreeSet<(RecordId, RecordId)>,
    charge: StateCharge,
}
impl AliasWindowRequests {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            targets: BTreeSet::new(),
            holds: BTreeSet::new(),
            charge: StateCharge::new(budget, "installed alias target and incidence identities"),
        }
    }
    fn hold(&mut self, owner: &RecordId, object: &RecordId) -> Result<(), ModelError> {
        if !self.holds.contains(&(owner.clone(), object.clone())) {
            self.charge.grow(
                crate::selection::node_bytes(owner)
                    .saturating_add(crate::selection::node_bytes(object))
                    .saturating_add(512),
            )?;
            self.holds.insert((owner.clone(), object.clone()));
        }
        Ok(())
    }
    fn observe(&mut self, row: &Value, owner: &RecordId) -> Result<(), ModelError> {
        use d::KeySink;
        let object = row
            .as_object()
            .ok_or(ModelError::Schema("installed alias object"))?;
        let Some(Value::RecordId(id)) = object.get("id") else {
            return Err(ModelError::Schema("installed alias identity"));
        };
        let Some(Value::RecordId(source)) = object.get("source") else {
            return Err(ModelError::Schema("installed alias source"));
        };
        let Some(Value::RecordId(target)) = object.get("target") else {
            return Err(ModelError::Schema("installed alias target"));
        };
        if source.table.as_str() != "entity" || target.table.as_str() != "entity" {
            return Err(ModelError::Conflict("installed alias entity family"));
        }
        let mut sink = KeySink::new("native-place-alias/v1");
        sink.part(
            b"source",
            &serde_json::to_vec(source).map_err(ModelError::codec)?,
        );
        sink.part(
            b"target",
            &serde_json::to_vec(target).map_err(ModelError::codec)?,
        );
        let mut canonical = surrealdb::types::Object::new();
        canonical.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
        canonical.insert("source", source.clone());
        canonical.insert("target", target.clone());
        if row != Value::Object(canonical) {
            return Err(ModelError::Conflict("installed exact alias envelope"));
        }
        if !self.targets.contains(target) {
            self.charge
                .grow(crate::selection::node_bytes(target).saturating_add(256))?;
            self.targets.insert(target.clone());
        }
        self.hold(id, source)?;
        self.hold(owner, target)
    }
}

async fn audit_local_alias_window(
    reader: &NativeReader<()>,
    owner: &RecordId,
    ids: &[RecordId],
    budget: &ResourceBudget,
    aliases: &mut AuditRows,
) -> Result<(), ModelError> {
    let mut actual = crate::control::inspect_expected_window_read(reader, ids, budget).await?;
    let mut requests = AliasWindowRequests::new(budget);
    let batch_charge = budget.reserve(
        "installed alias owned transfer",
        ids.len().saturating_mul(std::mem::size_of::<(
            Value,
            Box<dyn d::resources::Reservation>,
        )>()),
    )?;
    let mut rows = Vec::with_capacity(ids.len());
    for id in ids {
        let expected = actual.take_required(id)?;
        requests.observe(expected.row(), owner)?;
        let (row, arena) = expected.into_owned_row();
        rows.push((row, arena));
    }
    actual.finish()?;
    // Each target and heterogeneous incidence is observed once per bounded alias page.
    // Keep the request charge through both point sets and endpoint validation.
    requests.charge.grow(
        requests
            .targets
            .len()
            .saturating_mul(std::mem::size_of::<RecordId>())
            .saturating_add(
                requests
                    .holds
                    .len()
                    .saturating_mul(std::mem::size_of::<(RecordId, RecordId)>()),
            ),
    )?;
    let targets = requests.targets.into_iter().collect::<Vec<_>>();
    let holds = requests.holds.into_iter().collect::<Vec<_>>();
    let mut backing =
        crate::control::inspect_expected_window_read(reader, &targets, budget).await?;
    for window in holds.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
        let mut actual = crate::control::inspect_hold_window_read(reader, window, budget).await?;
        for (owner, object) in window {
            actual.take_required(owner, object)?;
        }
        actual.finish()?;
    }
    for target in &targets {
        let expected = backing.take_required(target)?;
        let crate::completed_rows::CanonicalBackingEndpoints {
            endpoints,
            original,
            reservation: _decode,
        } = canonical_backing_endpoints(expected.row(), budget)?;
        if original.is_some() {
            return Err(ModelError::Conflict(
                "installed entity alias has original metadata",
            ));
        }
        for window in endpoints.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            let expected = window
                .iter()
                .map(|row| {
                    let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id"))
                    else {
                        return Err(ModelError::Schema("installed alias endpoint identity"));
                    };
                    Ok((id.clone(), row.clone()))
                })
                .collect::<Result<BTreeMap<_, _>, ModelError>>()?;
            audit_endpoint_window(reader, owner, &expected, budget).await?;
        }
    }
    backing.finish()?;
    // Publish no alias prefix before every source, target, hold and endpoint in
    // this page validates. A failed ordering command is terminal, never a signal
    // to replay a poisoned builder through the alias capacity fallback.
    aliases
        .flush()
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))?;
    aliases
        .sort
        .push_owned_batch_reserved(rows, batch_charge)
        .await
        .map_err(|error| ModelError::Cause(Box::new(error)))
}
fn audit_alias_reachability(
    seeds: crate::ordered_rows::PreparedRows,
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    crate::alias_topology::audit_reachability(seeds, aliases, budget)
}

#[cfg(test)]
pub(crate) async fn logical_roots(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    bindings: &[d::completed::CompletedBinding],
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    logical_roots_many(
        reader,
        std::slice::from_ref(closure),
        bindings,
        aliases,
        budget,
    )
    .await
}

/// Reuse only normalized row fragments beneath this operation's exact protected
/// contributors. Every root still merges and hashes its own six complete families.
#[cfg(test)]
#[cfg(test)]
pub(crate) async fn logical_roots_scoped(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    bindings: &[d::completed::CompletedBinding],
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    audit.check(reader, budget)?;
    logical_roots_many_inner(reader, &[closure], bindings, aliases, budget, Some(audit)).await
}

async fn prepare_logical_contributor(
    reader: &NativeReader<()>,
    expected: &ExpectedRecord,
    owners: &BTreeMap<RecordId, CompletedContribution>,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<LogicalContributorRows, ModelError> {
    let compact =
        audit
            .membership_contributors
            .get(expected.record())
            .ok_or(ModelError::Conflict(
                "logical demand requires audited memberships",
            ))?;
    let mut contribution = AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES).await?;
    contribution
        .push(crate::completed_logical_rows::normalize(
            "compiler_contribution",
            expected.row(),
            owners,
        )?)
        .await?;
    let mut members = crate::acknowledged_candidates::AsyncOrderedRows::new_registered(
        compact.memberships.clone(),
        budget,
        audit.clone(),
        audit.blocking_observer()?,
    )
    .await?;
    let mut memberships = AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES).await?;
    let mut backing = AuditRows::new(audit, budget, d::resources::MAX_ROW_BYTES).await?;
    let result = async {
        loop {
            let batch = members
                .next_batch(crate::loader::NATIVE_WINDOW_ROWS)
                .await?;
            if batch.is_empty() {
                break;
            }
            let mut typed = Vec::new();
            let mut charge = StateCharge::new(budget, "logical selected locator batch");
            for row in batch {
                crate::compiler_blocks::validate_locator(&row)?;
                memberships
                    .push(crate::completed_logical_rows::normalize(
                        "compiler_membership",
                        &row,
                        owners,
                    )?)
                    .await?;
                let Some(Value::RecordId(node)) = row.as_object().and_then(|row| row.get("node"))
                else {
                    return Err(ModelError::Schema("logical backing member node"));
                };
                if node.table.as_str() == "compiler_record" {
                    charge.grow(
                        crate::loader::native_bytes(&row)
                            .saturating_mul(8)
                            .saturating_add(512),
                    )?;
                    typed.push(row);
                }
            }
            append_backing_page(reader, &typed, budget, &mut backing).await?;
        }
        #[cfg(test)]
        if LATE_MEMBERSHIP_FAILURE
            .try_with(|enabled| *enabled)
            .unwrap_or(false)
        {
            return Err(ModelError::Conflict(
                "late logical backing preparation control",
            ));
        }
        Ok::<_, ModelError>(())
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step("logical membership cursor terminal", members.drain().await);
    d::completion::complete(result, terminal)?;
    Ok(LogicalContributorRows {
        contribution: contribution.finish().await?,
        memberships: memberships.finish().await?,
        backing: backing.finish().await?,
    })
}
// Only explicitly typed page-owned refusal can shrink an exact retained slice. No
// accepted page or immutable membership preparation is reconstructed.
async fn append_backing_page(
    reader: &NativeReader<()>,
    pointers: &[Value],
    budget: &ResourceBudget,
    backing: &mut AuditRows,
) -> Result<(), ModelError> {
    use crate::control::held::PageAcquireFailure;
    backing.flush().await?;
    let mut retries = BackingPageRetries::new(pointers.len(), budget)?;
    while let Some((start, end)) = retries.next() {
        let page: Result<(), PageAcquireFailure> = async {
            let selected = &pointers[start..end];
            let mut actual =
                crate::compiler_blocks::inspect_backing_page_read(reader, selected, budget).await?;
            let transfer = budget
                .reserve(
                    "logical backing owned row handles",
                    selected.len().saturating_mul(std::mem::size_of::<(
                        Value,
                        Box<dyn d::resources::Reservation>,
                    )>()),
                )
                .map_err(PageAcquireFailure::Terminal)?;
            let mut rows = Vec::with_capacity(selected.len());
            for pointer in selected {
                let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("node"))
                else {
                    return Err(PageAcquireFailure::Terminal(ModelError::Schema(
                        "logical backing locator node",
                    )));
                };
                let row = actual
                    .take_required(id)
                    .map_err(PageAcquireFailure::Terminal)?;
                validate_state_row("compiler_record", row.row())
                    .map_err(PageAcquireFailure::Terminal)?;
                rows.push(
                    row.into_owned_row(budget)
                        .map_err(PageAcquireFailure::Terminal)?,
                );
            }
            drop(actual);
            backing
                .sort
                .push_owned_batch_reserved(rows, transfer)
                .await
                .map_err(PageAcquireFailure::Terminal)?;
            Ok(())
        }
        .await;
        if let Err(error) = page {
            retries.refuse((start, end), error)?;
        }
    }
    Ok(())
}

struct BackingPageRetries {
    pending: Vec<(usize, usize)>,
    _charge: Box<dyn d::resources::Reservation>,
}
impl BackingPageRetries {
    fn new(rows: usize, budget: &ResourceBudget) -> Result<Self, ModelError> {
        if rows > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Schema("logical backing input page bound"));
        }
        // Depth-first splitting retains at most one sibling per level, not a second
        // full page inventory. Admit that actual stack before allocating it.
        let depth = if rows == 0 {
            0
        } else {
            (usize::BITS - rows.leading_zeros()) as usize + 1
        };
        let charge = budget.reserve(
            "logical backing page split stack",
            depth.saturating_mul(std::mem::size_of::<(usize, usize)>()),
        )?;
        let mut pending = Vec::with_capacity(depth);
        if rows != 0 {
            pending.push((0, rows));
        }
        Ok(Self {
            pending,
            _charge: charge,
        })
    }
    fn refuse(
        &mut self,
        (start, end): (usize, usize),
        error: crate::control::held::PageAcquireFailure,
    ) -> Result<(), ModelError> {
        use crate::control::held::PageAcquireFailure;
        match error {
            PageAcquireFailure::Reducible(error) if end - start > 1 => {
                drop(error);
                let middle = start + (end - start) / 2;
                self.pending.push((middle, end));
                self.pending.push((start, middle));
                Ok(())
            }
            PageAcquireFailure::Reducible(error) | PageAcquireFailure::Terminal(error) => {
                Err(error)
            }
        }
    }
}
impl Iterator for BackingPageRetries {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<Self::Item> {
        self.pending.pop()
    }
}
fn terminal_local_resource(mut error: &ModelError) -> bool {
    while let ModelError::SharedCause(inner) = error {
        error = inner.as_ref();
    }
    matches!(error, ModelError::Resource { .. })
}

#[cfg(test)]
pub(crate) async fn logical_roots_many(
    reader: &NativeReader<()>,
    closures: &[InspectedClosure],
    bindings: &[d::completed::CompletedBinding],
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    logical_roots_many_inner(
        reader,
        &closures.iter().collect::<Vec<_>>(),
        bindings,
        aliases,
        budget,
        None,
    )
    .await
}
/// Operation-local semantic union. Each selector is independently assured before
/// inclusion; only unique physical ingredients survive the selector's closure.
pub(crate) struct LogicalFamilyInputs {
    owners: BTreeMap<RecordId, CompletedContribution>,
    incarnations: BTreeMap<RecordId, i64>,
    views: BTreeMap<ContentHash, CompletedView>,
    fragments: [Vec<crate::ordered_rows::PreparedRows>; 6],
    alias_backings: BTreeSet<usize>,
    families: Vec<crate::ordered_rows::SortedRows>,
    charge: StateCharge,
    failed: bool,
}
impl LogicalFamilyInputs {
    pub(crate) fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "logical family unique semantic ingredients");
        charge.grow(
            std::mem::size_of::<Self>()
                .saturating_add(6 * std::mem::size_of::<crate::ordered_rows::SortedRows>()),
        )?;
        Ok(Self {
            owners: BTreeMap::new(),
            incarnations: BTreeMap::new(),
            views: BTreeMap::new(),
            fragments: std::array::from_fn(|_| Vec::new()),
            alias_backings: BTreeSet::new(),
            families: d::recovery_closure::COMPLETED_FAMILIES
                .map(|_| {
                    crate::ordered_rows::SortedRows::with_budget_and_row_bytes(
                        budget,
                        d::resources::MAX_ROW_BYTES,
                    )
                })
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            charge,
            failed: false,
        })
    }
    pub(crate) fn include_closure(
        &mut self,
        reader: &NativeReader<()>,
        closure: &InspectedClosure,
        aliases: &crate::ordered_rows::PreparedRows,
        budget: &ResourceBudget,
        audit: &InstallationAudit,
    ) -> Result<(), ModelError> {
        if self.failed {
            return Err(ModelError::Conflict("failed logical family ingredients"));
        }
        let result = self.include_inner(reader, closure, aliases, budget, audit);
        self.failed = result.is_err();
        result
    }
    fn include_inner(
        &mut self,
        reader: &NativeReader<()>,
        closure: &InspectedClosure,
        aliases: &crate::ordered_rows::PreparedRows,
        budget: &ResourceBudget,
        audit: &InstallationAudit,
    ) -> Result<(), ModelError> {
        audit.check(reader, budget)?;
        if !self
            .charge
            .budget()
            .is_some_and(|owner| owner.shares_pool(budget))
        {
            return Err(ModelError::Conflict(
                "logical family foreign resource owner",
            ));
        }
        for (physical, expected) in &closure.contributors {
            let descriptor = closure.descriptor_for_copy(physical)?;
            let id = expected.record();
            let cached = audit.contributors.get(id).ok_or(ModelError::Conflict(
                "logical fragment requires complete protected contributor audit",
            ))?;
            if !std::sync::Arc::ptr_eq(&cached, expected)
                && (cached.incarnation() != expected.incarnation()
                    || cached.row() != expected.row())
            {
                return Err(ModelError::Conflict(
                    "logical fragment exact contributor premise changed",
                ));
            }
            if let Some(previous) = self.owners.get(id) {
                if previous != descriptor {
                    return Err(ModelError::Conflict(
                        "installed normalization physical owner descriptor collision",
                    ));
                }
                if self.incarnations.get(id) != Some(&expected.incarnation()) {
                    return Err(ModelError::Conflict(
                        "family exact physical incarnation contradiction",
                    ));
                }
                continue;
            }
            let prepared = audit
                .logical_contributors
                .get(id)
                .ok_or(ModelError::Conflict("prepared contributor absent"))?;
            // Clone the typed descriptor once per physical owner. Prepared payload
            // arenas remain shared with the protected audit and their live borrowers.
            self.charge.grow(
                std::mem::size_of::<(RecordId, CompletedContribution)>()
                    .saturating_add(std::mem::size_of::<(RecordId, i64)>())
                    .saturating_add(crate::selection::node_bytes(id).saturating_mul(2))
                    .saturating_add(contribution_heap(descriptor))
                    .saturating_add(128),
            )?;
            self.owners.insert(id.clone(), descriptor.clone());
            self.incarnations.insert(id.clone(), expected.incarnation());
            for (family, fragment) in [
                (0, &prepared.contribution),
                (1, &prepared.memberships),
                (3, &prepared.backing),
            ] {
                let handles = &mut self.fragments[family];
                if handles.len() == handles.capacity() {
                    let capacity = handles.capacity().saturating_mul(2).max(4);
                    self.charge.grow(
                        capacity.saturating_sub(handles.capacity()).saturating_mul(
                            std::mem::size_of::<crate::ordered_rows::PreparedRows>(),
                        ),
                    )?;
                    handles.reserve_exact(capacity - handles.len());
                }
                self.fragments[family].push(fragment.clone());
            }
        }
        for (identity, view) in closure.logical.views() {
            if let Some(previous) = self.views.get(identity) {
                if previous != view {
                    return Err(ModelError::Conflict(
                        "family exact view descriptor contradiction",
                    ));
                }
            } else {
                self.charge.grow(
                    std::mem::size_of::<(ContentHash, CompletedView)>()
                        .saturating_add(32)
                        .saturating_add(view.relation.capacity())
                        .saturating_add(
                            view.contributions
                                .len()
                                .saturating_mul(std::mem::size_of::<ContentHash>() + 32),
                        ),
                )?;
                self.views.insert(*identity, view.clone());
            }
        }
        self.include_alias_run(aliases)?;
        audit.check(reader, budget)
    }
    fn include_alias_run(
        &mut self,
        aliases: &crate::ordered_rows::PreparedRows,
    ) -> Result<(), ModelError> {
        // Retain every distinct backing, including conflicting/equal-looking runs.
        // The fragment owns the Arc while its address is indexed, preventing ABA.
        let identity = aliases.backing_identity();
        if self.alias_backings.contains(&identity) {
            return Ok(());
        }
        self.charge
            .grow(std::mem::size_of::<usize>().saturating_add(128))?;
        let handles = &mut self.fragments[5];
        if handles.len() == handles.capacity() {
            let capacity = handles.capacity().saturating_mul(2).max(4);
            self.charge.grow(
                capacity
                    .saturating_sub(handles.capacity())
                    .saturating_mul(std::mem::size_of::<crate::ordered_rows::PreparedRows>()),
            )?;
            handles.reserve_exact(capacity - handles.len());
        }
        handles.push(aliases.clone());
        self.alias_backings.insert(identity);
        Ok(())
    }
    pub(crate) async fn finish_scoped(
        self,
        reader: &NativeReader<()>,
        bindings: &[d::completed::CompletedBinding],
        budget: &ResourceBudget,
        audit: &InstallationAudit,
    ) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
        if self.failed {
            return Err(ModelError::Conflict("failed logical family ingredients"));
        }
        audit.ensure_context(reader, budget).await?;
        let bindings_charge = budget.reserve(
            "logical family worker binding copies",
            bindings
                .iter()
                .map(|binding| {
                    std::mem::size_of_val(binding)
                        .saturating_add(binding.boundary.as_ref().map_or(0, String::capacity))
                        .saturating_add(d::HeapSize::heap_bytes(&binding.source))
                        .saturating_add(binding.view.relation.capacity())
                        .saturating_add(
                            binding
                                .view
                                .contributions
                                .len()
                                .saturating_mul(std::mem::size_of::<ContentHash>() + 64),
                        )
                })
                .sum::<usize>(),
        )?;
        let bindings = bindings.to_vec();
        let worker = audit.clone();
        let (inputs, bindings, dependencies, _bindings_charge) = audit
            .blocking(move |budget| {
                let mut inputs = self;
                let _bindings_charge = bindings_charge;
                worker.pin.ensure_live()?;
                d::completed::binding_inventory_identity(&bindings)?;
                for view in inputs.views.values() {
                    worker.pin.ensure_live()?;
                    push_validated_family_view(
                        &mut inputs.families[2],
                        view,
                        &inputs.owners,
                        budget,
                    )?;
                }
                push_family_bindings(
                    &mut inputs.families[4],
                    &bindings,
                    &inputs.views,
                    &inputs.owners,
                    budget,
                )?;
                let mut dependencies = [ContentHash::of(b"unprepared-family"); 6];
                for (index, rows) in std::mem::take(&mut inputs.families).into_iter().enumerate() {
                    worker.pin.ensure_live()?;
                    let handles = &mut inputs.fragments[index];
                    if handles.len() == handles.capacity() {
                        let capacity = handles.capacity().saturating_mul(2).max(4);
                        inputs.charge.grow(
                            capacity.saturating_sub(handles.capacity()).saturating_mul(
                                std::mem::size_of::<crate::ordered_rows::PreparedRows>(),
                            ),
                        )?;
                        handles.reserve_exact(capacity - handles.len());
                    }
                    handles.push(rows.finish()?.into_prepared());
                    dependencies[index] = family_dependency(
                        index,
                        &inputs.owners,
                        &inputs.incarnations,
                        &inputs.views,
                        &bindings,
                        handles,
                        budget,
                    )?;
                }
                worker.pin.ensure_live()?;
                Ok((inputs, bindings, dependencies, _bindings_charge))
            })
            .await?;
        finish_family_roots(
            inputs.families,
            inputs.fragments,
            FamilyRootInputs {
                owners: &inputs.owners,
                incarnations: &inputs.incarnations,
                views: &inputs.views,
                bindings: &bindings,
                budget,
                audit: Some(audit),
                prepared_dependencies: Some(dependencies),
            },
        )
        .await
    }
}

fn push_validated_family_view(
    family: &mut crate::ordered_rows::SortedRows,
    view: &CompletedView,
    owners: &BTreeMap<RecordId, CompletedContribution>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let _decode = budget.reserve(
        "logical validated view normalization",
        view.relation
            .len()
            .saturating_mul(6)
            .saturating_add(view.contributions.len().saturating_mul(512))
            .saturating_add(65536),
    )?;
    let mut row = surrealdb::types::Object::new();
    row.insert("id", RecordId::new("compiler_view", view.identity.hex()));
    row.insert(
        "descriptor",
        surrealdb::types::Bytes::from(serde_json::to_vec(view).map_err(ModelError::codec)?),
    );
    family.push(crate::completed_logical_rows::normalize(
        "compiler_view",
        &Value::Object(row),
        owners,
    )?)
}
fn push_family_bindings(
    family: &mut crate::ordered_rows::SortedRows,
    bindings: &[d::completed::CompletedBinding],
    views: &BTreeMap<ContentHash, CompletedView>,
    owners: &BTreeMap<RecordId, CompletedContribution>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    for binding in bindings {
        if views.get(&binding.view.identity) != Some(&binding.view) {
            return Err(ModelError::Conflict(
                "logical binding outside exact dependency closure",
            ));
        }
        let _decode = budget.reserve(
            "logical binding normalization",
            serde_json::to_vec(binding)
                .map_err(ModelError::codec)?
                .len()
                .saturating_mul(16)
                .saturating_add(65536),
        )?;
        let mut row = surrealdb::types::Object::new();
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?),
        );
        family.push(crate::completed_logical_rows::normalize(
            "compiler_binding",
            &Value::Object(row),
            owners,
        )?)?;
    }
    Ok(())
}

#[cfg(test)]
async fn logical_roots_many_inner(
    reader: &NativeReader<()>,
    closures: &[&InspectedClosure],
    bindings: &[d::completed::CompletedBinding],
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
    audit: Option<&InstallationAudit>,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    use crate::ordered_rows::SortedRows;
    use d::{
        completion::{Completion, complete},
        recovery_closure::COMPLETED_FAMILIES,
    };
    if let Some(audit) = audit {
        audit.ensure_context(reader, budget).await?;
        let mut inputs = LogicalFamilyInputs::new(budget)?;
        // The caller supplied one already composed semantic alias union.
        let empty = crate::ordered_rows::SortedRows::with_budget(budget)?
            .finish()?
            .into_prepared();
        for closure in closures {
            prepare_closure_in_context(reader, closure, budget, audit).await?;
            inputs.include_closure(reader, closure, &empty, budget, audit)?;
        }
        inputs.include_alias_run(aliases)?;
        return inputs.finish_scoped(reader, bindings, budget, audit).await;
    }
    d::completed::binding_inventory_identity(bindings)?;
    let mut owner_charge =
        StateCharge::new(budget, "installed logical normalization owner descriptors");
    let mut owners = BTreeMap::new();
    for closure in closures {
        for mapping in &closure.mappings {
            let descriptor = &closure.logical.contributions()[&mapping.logical];
            let expected = &closure.contributors[&mapping.physical];
            if let Some(previous) = owners.get(expected.record()) {
                if previous != descriptor {
                    return Err(ModelError::Conflict(
                        "installed normalization physical owner descriptor collision",
                    ));
                }
            } else {
                // This map retains typed clones, not serialized descriptors. Reuse
                // the closure's typed heap admission before cloning; ExpectedRecord
                // independently retains and charges the original native row. Its
                // key is the inspected compiler_contribution record's 64-hex ID.
                owner_charge.grow(
                    std::mem::size_of::<(RecordId, CompletedContribution)>()
                        .saturating_add(32)
                        .saturating_add(expected.record().table.as_str().len())
                        .saturating_add(64)
                        .saturating_add(contribution_heap(descriptor)),
                )?;
                owners.insert(expected.record().clone(), descriptor.clone());
            }
        }
    }
    let mut incarnations = BTreeMap::new();
    let mut views = BTreeMap::new();
    for closure in closures {
        for expected in closure.contributors.values() {
            let id = expected.record();
            if let Some(previous) = incarnations.insert(id.clone(), expected.incarnation())
                && previous != expected.incarnation()
            {
                return Err(ModelError::Conflict(
                    "family exact physical incarnation contradiction",
                ));
            }
        }
        for (identity, view) in closure.logical.views() {
            if let Some(previous) = views.insert(*identity, view.clone())
                && previous != *view
            {
                return Err(ModelError::Conflict(
                    "family exact view descriptor contradiction",
                ));
            }
        }
    }
    let fragments: [Vec<crate::ordered_rows::PreparedRows>; 6] =
        std::array::from_fn(|_| Vec::new());
    let mut families = COMPLETED_FAMILIES
        .map(|_| SortedRows::with_budget_and_row_bytes(budget, d::resources::MAX_ROW_BYTES))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    // Every selector was assured independently above. The semantic families are
    // sets: contribute each exact physical owner's immutable runs once, rather
    // than opening them again for each overlapping selector. Descriptor and
    // incarnation contradictions have already been rejected before this reduction.
    let _selected = budget.reserve(
        "logical union unique physical handles",
        owners.len().saturating_mul(96).saturating_add(1024),
    )?;
    let mut selected = std::collections::BTreeSet::new();
    for closure in closures {
        for mapping in &closure.mappings {
            if !selected.insert(mapping.physical) {
                continue;
            }
            let expected = &closure.contributors[&mapping.physical];
            families[0].push(crate::completed_logical_rows::normalize(
                "compiler_contribution",
                expected.row(),
                &owners,
            )?)?;
            let mut variables = Variables::new();
            variables.insert("owner", expected.record().clone());
            let mut rows=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
            let result = async {
                let mut locators = Vec::new();
                let mut charge = StateCharge::new(budget, "logical native locator window");
                while let Some(row) = rows.next().await? {
                    crate::compiler_blocks::validate_locator(&row)?;
                    families[1].push(crate::completed_logical_rows::normalize(
                        "compiler_membership",
                        &row,
                        &owners,
                    )?)?;
                    let Some(Value::RecordId(node)) =
                        row.as_object().and_then(|row| row.get("node"))
                    else {
                        return Err(ModelError::Schema("logical backing member node"));
                    };
                    if node.table.as_str() == "compiler_record" {
                        charge.grow(
                            crate::loader::native_bytes(&row)
                                .saturating_mul(8)
                                .saturating_add(512),
                        )?;
                        locators.push(row);
                    }
                    if locators.len() >= crate::loader::NATIVE_WINDOW_ROWS
                        || charge.reserved() >= d::resources::TRANSFER_BYTES
                    {
                        append_logical_backing_records(reader, &locators, budget, &mut families[3])
                            .await?;
                        locators.clear();
                        charge = StateCharge::new(budget, "logical native locator window");
                    }
                }
                append_logical_backing_records(reader, &locators, budget, &mut families[3]).await?;
                Ok::<_, ModelError>(())
            }
            .await;
            let mut completion = Completion::default();
            completion.step(
                "installed logical membership drainage",
                rows.drain_transport().await,
            );
            complete(result, completion)?;
        }
    }
    // The exact unique view map above also rejected contradictory descriptors.
    // Normalize each view once before ordering, preserving the same union bytes.
    for view in views.values() {
        let mut variables = Variables::new();
        variables.insert("view", RecordId::new("compiler_view", view.identity.hex()));
        let _read = budget.reserve("logical declared view read", std::mem::size_of::<Value>())?;
        let rows: Vec<Value> = reader
            .query_native("SELECT * FROM $view", variables)
            .await?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("logical declared view cardinality"));
        };
        let _decode = budget.reserve(
            "logical declared view normalization",
            crate::loader::native_bytes(row)
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        validate_state_row("compiler_view", row)?;
        if descriptor::<CompletedView>(row)? != *view {
            return Err(ModelError::Conflict("logical declared view changed"));
        }
        families[2].push(crate::completed_logical_rows::normalize(
            "compiler_view",
            row,
            &owners,
        )?)?;
    }
    push_family_bindings(&mut families[4], bindings, &views, &owners, budget)?;
    let mut aliases = aliases.cursor_with_budget(budget)?;
    while let Some(row) = aliases.next_row()? {
        families[5].push(crate::completed_logical_rows::normalize(
            "compiler_alias",
            &row,
            &owners,
        )?)?;
    }
    finish_family_roots(
        families,
        fragments,
        FamilyRootInputs {
            owners: &owners,
            incarnations: &incarnations,
            views: &views,
            bindings,
            budget,
            audit,
            prepared_dependencies: None,
        },
    )
    .await
}

#[cfg(test)]
async fn append_logical_backing_records(
    reader: &NativeReader<()>,
    locators: &[Value],
    budget: &ResourceBudget,
    sort: &mut crate::ordered_rows::SortedRows,
) -> Result<(), ModelError> {
    let mut actual =
        crate::compiler_blocks::inspect_backing_window_read(reader, locators, budget).await?;
    for locator in locators {
        let Some(Value::RecordId(node)) = locator.as_object().and_then(|row| row.get("node"))
        else {
            return Err(ModelError::Schema("logical backing locator node"));
        };
        let row = actual.take_required(node)?;
        validate_state_row("compiler_record", row.row())?;
        let (row, charge) = row.into_owned_row(budget)?;
        sort.push_owned(row, charge)?;
    }
    Ok(())
}

struct FamilyRootInputs<'a> {
    owners: &'a BTreeMap<RecordId, CompletedContribution>,
    incarnations: &'a BTreeMap<RecordId, i64>,
    views: &'a BTreeMap<ContentHash, CompletedView>,
    bindings: &'a [d::completed::CompletedBinding],
    budget: &'a ResourceBudget,
    audit: Option<&'a InstallationAudit>,
    prepared_dependencies: Option<[ContentHash; 6]>,
}
/// One canonical family dependency and fold implementation serves both independent
/// physical reconstruction and streamed protected semantic ingredients.
async fn finish_family_roots(
    families: Vec<crate::ordered_rows::SortedRows>,
    mut fragments: [Vec<crate::ordered_rows::PreparedRows>; 6],
    inputs: FamilyRootInputs<'_>,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    use d::{completed_identity::CompletedFamilyHasher, recovery_closure::COMPLETED_FAMILIES};
    let FamilyRootInputs {
        owners,
        incarnations,
        views,
        bindings,
        budget,
        audit,
        prepared_dependencies,
    } = inputs;
    let mut final_handles = StateCharge::new(budget, "logical family final run handles");
    let mut roots = Vec::with_capacity(6);
    let mut families = families.into_iter();
    for (index, family) in COMPLETED_FAMILIES.into_iter().enumerate() {
        let dependency = if let Some(dependencies) = &prepared_dependencies {
            dependencies[index]
        } else {
            if fragments[index].len() == fragments[index].capacity() {
                let capacity = fragments[index].capacity().saturating_mul(2).max(4);
                final_handles.grow(
                    capacity
                        .saturating_sub(fragments[index].capacity())
                        .saturating_mul(std::mem::size_of::<crate::ordered_rows::PreparedRows>()),
                )?;
                fragments[index].reserve_exact(capacity - fragments[index].len());
            }
            fragments[index].push(
                families
                    .next()
                    .ok_or(ModelError::Schema("logical completed family cardinality"))?
                    .finish()?
                    .into_prepared(),
            );
            family_dependency(
                index,
                owners,
                incarnations,
                views,
                bindings,
                &fragments[index],
                budget,
            )?
        };
        if let Some(audit) = audit {
            let runs = std::mem::take(&mut fragments[index]);
            let handles = budget.reserve(
                "family root preparation worker run handles",
                runs.capacity()
                    .saturating_mul(std::mem::size_of::<crate::ordered_rows::PreparedRows>())
                    .saturating_add(1024),
            )?;
            audit
                .run_work((4, dependency), move |owner| {
                    async move {
                        if owner.family_roots.contains_key(&(index, dependency)) {
                            return Ok(());
                        }
                        owner.pin.ensure_live()?;
                        let worker = owner.clone();
                        let root = owner
                            .blocking(move |budget| {
                                let _handles = handles;
                                let mut hasher = CompletedFamilyHasher::new(family)?;
                                crate::ordered_rows::PreparedRows::fold_union(
                                    &runs,
                                    budget,
                                    || worker.pin.ensure_live(),
                                    |_, payload| hasher.row(payload),
                                )?;
                                worker.pin.ensure_live()?;
                                Ok(hasher.finish())
                            })
                            .await?;
                        owner.pin.ensure_live()?;
                        owner.charge.grow(
                            std::mem::size_of::<(
                                (usize, ContentHash),
                                d::completed_identity::CompletedFamilyRoot,
                            )>()
                            .saturating_mul(11)
                            .saturating_add(256),
                        )?;
                        owner.family_roots.insert((index, dependency), root);
                        Ok(())
                    }
                    .boxed()
                })
                .await?;
            roots.push(
                audit
                    .family_roots
                    .get(&(index, dependency))
                    .ok_or(ModelError::Conflict("prepared family root absent"))?
                    .as_ref()
                    .clone(),
            );
        } else {
            let mut hasher = CompletedFamilyHasher::new(family)?;
            crate::ordered_rows::PreparedRows::fold_union(
                &fragments[index],
                budget,
                || Ok(()),
                |_, payload| hasher.row(payload),
            )?;
            roots.push(hasher.finish());
        }
    }
    d::completed_identity::CompletedFamilyRoots::new(
        roots
            .try_into()
            .map_err(|_| ModelError::Schema("logical completed family cardinality"))?,
    )
}

/// Prepare each full exact contributor once. Cache publication follows EOF and a
/// final current-incarnation/full-row check under the actual observation pin.
async fn prepare_closure_in_context(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<Vec<LogicalContributorRows>, ModelError> {
    audit.check(reader, budget)?;
    let mut charge = StateCharge::new(budget, "prepared contributor borrowed run handles");
    charge.grow(
        closure
            .contributors
            .len()
            .saturating_mul(std::mem::size_of::<LogicalContributorRows>()),
    )?;
    let mut result = Vec::with_capacity(closure.contributors.len());
    let mut contributors = closure.contributors.values().peekable();
    while contributors.peek().is_some() {
        let mut window = ContributorWindow::new(budget);
        for expected in contributors
            .by_ref()
            .take(crate::loader::NATIVE_WINDOW_ROWS)
        {
            let cached = audit
                .contributors
                .get(expected.record())
                .ok_or(ModelError::Conflict(
                    "preparation requires protected completed contributor",
                ))?;
            if cached.incarnation() != expected.incarnation() || cached.row() != expected.row() {
                return Err(ModelError::Conflict(
                    "preparation exact contributor premise",
                ));
            }
            if audit.logical_contributors.contains_key(expected.record()) {
                audit.check(reader, budget)?;
                continue;
            }
            let (key, input) = window.exact_capture(expected).await?;
            let worker_budget = budget.clone();
            window
                .push(async move {
                    audit
                        .run_work((1, key), move |owner| {
                            async move {
                                let reader = NativeReader::private(owner.client.clone())
                                    .with_budget(&worker_budget);
                                let result = async {
                                    let expected =
                                        worker_expected(&reader, &input, &worker_budget).await?;
                                    if owner.logical_contributors.contains_key(expected.record()) {
                                        return Ok(None);
                                    }
                                    let descriptor =
                                        descriptor::<CompletedContribution>(expected.row())?;
                                    let owners =
                                        BTreeMap::from([(expected.record().clone(), descriptor)]);
                                    let prepared = prepare_logical_contributor(
                                        &reader,
                                        &expected,
                                        &owners,
                                        &worker_budget,
                                        &owner,
                                    )
                                    .await?;
                                    worker_expected(&reader, &input, &worker_budget).await?;
                                    owner.ensure_context(&reader, &worker_budget).await?;
                                    owner.charge.grow(
                                        std::mem::size_of::<(RecordId, LogicalContributorRows)>()
                                            .saturating_mul(11)
                                            .saturating_add(1024),
                                    )?;
                                    Ok(Some((expected.record().clone(), prepared)))
                                }
                                .await;
                                let mut terminal = d::completion::Completion::default();
                                terminal.step(
                                    "logical contributor worker reader close",
                                    reader.close().await,
                                );
                                let prepared = d::completion::complete(result, terminal)?;
                                owner.pin.ensure_live()?;
                                if let Some((record, prepared)) = prepared {
                                    owner.logical_contributors.insert(record, prepared);
                                    owner
                                        .logical_contribution_observations
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }
                                Ok(())
                            }
                            .boxed()
                        })
                        .await
                })
                .await?;
        }
        window.finish().await?;
    }
    // Canonical contributor order, independent of actual worker completion.
    for expected in closure.contributors.values() {
        result.push(
            audit
                .logical_contributors
                .get(expected.record())
                .ok_or(ModelError::Conflict("prepared contributor absent"))?
                .as_ref()
                .clone(),
        );
    }
    Ok(result)
}

#[cfg(test)]
pub(crate) async fn logical_roots_many_scoped(
    reader: &NativeReader<()>,
    closures: &[&InspectedClosure],
    bindings: &[d::completed::CompletedBinding],
    aliases: &crate::ordered_rows::PreparedRows,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<d::completed_identity::CompletedFamilyRoots, ModelError> {
    audit.check(reader, budget)?;
    logical_roots_many_inner(reader, closures, bindings, aliases, budget, Some(audit)).await
}

/// Complete family dependencies, canonicalized independently of selector traversal
/// decomposition. Full normalized descriptor identity owns normalized membership IDs.
fn family_dependency(
    index: usize,
    owners: &BTreeMap<RecordId, CompletedContribution>,
    incarnations: &BTreeMap<RecordId, i64>,
    views: &BTreeMap<ContentHash, CompletedView>,
    bindings: &[d::completed::CompletedBinding],
    aliases: &[crate::ordered_rows::PreparedRows],
    budget: &ResourceBudget,
) -> Result<ContentHash, ModelError> {
    use d::Key;
    let mut sink = d::KeySink::new("native-prepared-family-dependencies/v2");
    sink.part(b"family", &index.to_le_bytes());
    if index == 0 {
        let mut logical = BTreeMap::new();
        let mut charge = StateCharge::new(budget, "canonical logical contributor dependency keys");
        for descriptor in owners.values() {
            charge.grow(512)?;
            let identity = descriptor.identity()?;
            if let Some(previous) = logical.insert(identity, descriptor)
                && previous != descriptor
            {
                return Err(ModelError::Conflict(
                    "logical contributor dependency collision",
                ));
            }
        }
        for (identity, descriptor) in logical {
            identity.encode(&mut sink);
            let _scratch = budget.reserve(
                "family contributor descriptor dependency",
                contribution_heap(descriptor)
                    .saturating_mul(32)
                    .saturating_add(65536),
            )?;
            sink.part(
                b"descriptor",
                &serde_json::to_vec(descriptor).map_err(ModelError::codec)?,
            );
        }
    }
    if matches!(index, 1 | 3) {
        for (physical, descriptor) in owners {
            let incarnation = *incarnations.get(physical).ok_or(ModelError::Conflict(
                "family dependency missing exact incarnation",
            ))?;
            if incarnation <= 0 {
                return Err(ModelError::Conflict(
                    "family dependency inactive incarnation",
                ));
            }
            sink.part(
                b"physical",
                &serde_json::to_vec(physical).map_err(ModelError::codec)?,
            );
            sink.part(b"incarnation", &incarnation.to_le_bytes());
            descriptor.identity()?.encode(&mut sink);
        }
    }
    if index == 2 {
        for view in views.values() {
            let _scratch = budget.reserve(
                "family view dependency serialization",
                view.relation
                    .len()
                    .saturating_mul(6)
                    .saturating_add(view.contributions.len().saturating_mul(512))
                    .saturating_add(65536),
            )?;
            sink.part(
                b"view",
                &serde_json::to_vec(view).map_err(ModelError::codec)?,
            );
        }
    }
    if index == 4 {
        let mut ordered = BTreeMap::new();
        let mut charge = StateCharge::new(budget, "canonical binding dependency keys");
        for binding in bindings {
            charge.grow(512)?;
            if let Some(previous) = ordered.insert(binding.key(), binding)
                && previous != binding
            {
                return Err(ModelError::Conflict("binding family dependency collision"));
            }
        }
        for binding in ordered.values() {
            let _scratch = budget.reserve(
                "family binding dependency serialization",
                65536usize
                    .saturating_add(binding.view.relation.len().saturating_mul(6))
                    .saturating_add(binding.view.contributions.len().saturating_mul(512)),
            )?;
            sink.part(
                b"binding",
                &serde_json::to_vec(binding).map_err(ModelError::codec)?,
            );
        }
    }
    if index == 5 {
        crate::ordered_rows::PreparedRows::fold_union(
            aliases,
            budget,
            || Ok(()),
            |_, payload| {
                sink.part(b"alias", payload);
                Ok(())
            },
        )?;
    }
    Ok(sink.finish())
}

#[cfg(test)]
mod family_dependency_controls {
    use super::*;
    #[test]
    fn alias_fragment_reuse_is_backing_exact_and_keeps_distinct_conflicts() {
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let run = |payload: i64| {
            let mut sorted = crate::ordered_rows::SortedRows::with_budget(&budget).unwrap();
            let mut row = surrealdb::types::Object::new();
            row.insert("id", RecordId::new("compiler_alias", "same"));
            row.insert("payload", payload);
            sorted.push(Value::Object(row)).unwrap();
            sorted.finish().unwrap().into_prepared()
        };
        let first = run(1);
        let mut inputs = LogicalFamilyInputs::new(&budget).unwrap();
        inputs.include_alias_run(&first).unwrap();
        let retained = inputs.charge.reserved();
        for _ in 0..17 {
            inputs.include_alias_run(&first.clone()).unwrap();
        }
        assert_eq!(
            inputs.charge.reserved(),
            retained,
            "repeated exact backing retains no extra workspace"
        );
        inputs.include_alias_run(&run(2)).unwrap();
        assert!(
            crate::ordered_rows::PreparedRows::fold_union(
                &inputs.fragments[5],
                &budget,
                || Ok(()),
                |_, _| Ok(())
            )
            .is_err(),
            "distinct conflicting backings remain visible to agreement"
        );
        drop(inputs);
        drop(first);
        assert_eq!(
            budget.reserved(),
            0,
            "identity index and retained backings release together"
        );
    }
    fn descriptor() -> CompletedContribution {
        CompletedContribution {
            spec: d::completed::ContributionSpec {
                captured_binding: None,
                producer: "p".into(),
                profile: d::completed::OutputProfile::Specific(d::stages::Profile::Catalog),
                model: ContentHash([1; 32]),
                implementation: ContentHash([2; 32]),
                configuration: None,
                inputs: vec![],
                outputs: ["packages".into()].into(),
            },
            assurance: d::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            outcome: 0,
            outputs: [(
                "packages".into(),
                d::completed::OutputContent {
                    rows: 0,
                    content: ContentHash([3; 32]),
                },
            )]
            .into(),
        }
    }
    #[test]
    fn family_keys_follow_exact_full_dependencies_and_ignore_unrelated_families() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let one = descriptor();
        let logical = one.identity().unwrap();
        let a = RecordId::new("compiler_contribution", "a");
        let b = RecordId::new("compiler_contribution", "b");
        let owners = BTreeMap::from([(a.clone(), one.clone())]);
        let incarnations = BTreeMap::from([(a.clone(), 1)]);
        let view = CompletedView::new("packages".into(), [logical].into(), 0).unwrap();
        let views = BTreeMap::from([(view.identity, view.clone())]);
        let key = |index,
                   owners: &BTreeMap<RecordId, CompletedContribution>,
                   incarnations: &BTreeMap<RecordId, i64>,
                   views: &BTreeMap<ContentHash, CompletedView>| {
            family_dependency(index, owners, incarnations, views, &[], &[], &budget).unwrap()
        };
        let copies = BTreeMap::from([(a.clone(), one.clone()), (b.clone(), one.clone())]);
        let copy_incarnations = BTreeMap::from([(a.clone(), 1), (b.clone(), 2)]);
        assert_eq!(
            key(0, &owners, &incarnations, &views),
            key(0, &copies, &copy_incarnations, &views),
            "same complete logical descriptors ignore copy decomposition"
        );
        for index in [1, 3] {
            assert_ne!(
                key(index, &owners, &incarnations, &views),
                key(index, &copies, &copy_incarnations, &views),
                "selected physical copy inventory is a complete dependency"
            );
            let aba = BTreeMap::from([(a.clone(), 2)]);
            assert_ne!(
                key(index, &owners, &incarnations, &views),
                key(index, &owners, &aba, &views)
            );
        }
        let mut outcome = one.clone();
        outcome.outcome = 2;
        let changed = BTreeMap::from([(a.clone(), outcome)]);
        for index in [0, 1, 3] {
            assert_ne!(
                key(index, &owners, &incarnations, &views),
                key(index, &changed, &incarnations, &views),
                "full completed identity owns logical membership normalization"
            );
        }
        let empty_views = BTreeMap::new();
        for index in [0, 1, 3, 4, 5] {
            assert_eq!(
                key(index, &owners, &incarnations, &views),
                key(index, &owners, &incarnations, &empty_views),
                "unrelated view inventory must not invalidate this family"
            );
        }
        assert_ne!(
            key(2, &owners, &incarnations, &views),
            key(2, &owners, &incarnations, &empty_views)
        );
        let inactive = BTreeMap::from([(a, 0)]);
        assert!(family_dependency(1, &owners, &inactive, &views, &[], &[], &budget).is_err());
    }
}

/// Prepared relation runs retain exact native locators. Nominal agreement compares only
/// logical metadata, while one deterministic actual placement accompanies each selected member.
fn compose_selected_member_runs(
    selector: ContentHash,
    relation: &str,
    runs: &[crate::ordered_rows::PreparedRows],
    budget: &ResourceBudget,
    mut check: impl FnMut() -> Result<(), ModelError>,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    check()?;
    // Physical placements can differ for the same selected logical member. Sort the small
    // projection by nominal member THEN source locator, compare logical metadata only, and
    // retain one deterministic placement. Neither placement nor packing enters content identity.
    let mut placements = crate::ordered_rows::SortedRows::with_budget_and_row_bytes(
        budget,
        d::resources::MAX_ROW_BYTES,
    )?;
    for run in runs {
        let mut cursor = run.cursor_with_budget(budget)?;
        loop {
            check()?;
            let Some(row) = cursor.next_row()? else {
                break;
            };
            if !matches!(row.as_object().and_then(|row| row.get("relation")),
                Some(Value::String(actual)) if actual == relation)
            {
                return Err(ModelError::Conflict("selected prepared relation partition"));
            }
            crate::compiler_blocks::validate_locator(&row)?;
            let member = crate::completed_state::earning::member(selector, &row)?;
            let member_id = member
                .as_object()
                .and_then(|row| row.get("id"))
                .cloned()
                .ok_or(ModelError::Schema("selected projected member ID"))?;
            let locator_id = row
                .as_object()
                .and_then(|row| row.get("id"))
                .cloned()
                .ok_or(ModelError::Schema("selected projected locator ID"))?;
            let mut projection = surrealdb::types::Object::new();
            projection.insert(
                "id",
                RecordId::new(
                    "prepared_member_placement",
                    surrealdb::types::RecordIdKey::Array(vec![member_id, locator_id].into()),
                ),
            );
            projection.insert("member", member);
            projection.insert("locator", row);
            placements.push(Value::Object(projection))?;
        }
    }
    let mut placements = placements.finish()?;
    let mut rows = crate::ordered_rows::SortedRows::with_budget_and_row_bytes(
        budget,
        d::resources::MAX_ROW_BYTES,
    )?;
    let mut pending: Option<(Value, Value)> = None;
    let mut charge = StateCharge::new(budget, "selected nominal placement group");
    loop {
        check()?;
        let Some(projection) = placements.next_row()? else {
            break;
        };
        let Value::Object(mut projection) = projection else {
            return Err(ModelError::Schema("selected placement object"));
        };
        let member = projection
            .remove("member")
            .ok_or(ModelError::Schema("selected placement member"))?;
        let locator = projection
            .remove("locator")
            .ok_or(ModelError::Schema("selected placement locator"))?;
        if let Some((previous, _)) = pending.as_ref() {
            if previous.as_object().and_then(|row| row.get("id"))
                == member.as_object().and_then(|row| row.get("id"))
            {
                if previous != member {
                    return Err(ModelError::Conflict("conflicting physical row identity"));
                }
                continue;
            }
            let (previous, locator) = pending.take().expect("current nominal group");
            rows.push(selected_member_with_locator(previous, locator)?)?;
            charge = StateCharge::new(budget, "selected nominal placement group");
        }
        charge.grow(
            crate::loader::native_bytes(&member)
                .saturating_add(crate::loader::native_bytes(&locator))
                .saturating_mul(8)
                .saturating_add(1024),
        )?;
        pending = Some((member, locator));
    }
    if let Some((member, locator)) = pending {
        rows.push(selected_member_with_locator(member, locator)?)?;
    }
    check()?;
    let rows = rows.finish()?.into_prepared();
    check()?;
    Ok(rows)
}
fn selected_member_with_locator(member: Value, locator: Value) -> Result<Value, ModelError> {
    let Value::Object(mut member) = member else {
        return Err(ModelError::Schema("selected member object"));
    };
    member.insert("locator", locator);
    Ok(Value::Object(member))
}

pub(crate) async fn selected_prepared_members(
    closure: &InspectedClosure,
    view: &CompletedView,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    Ok(
        selected_prepared_member_result(closure, view, budget, audit)
            .await?
            .rows
            .clone(),
    )
}
pub(crate) async fn selected_prepared_members_with_content(
    closure: &InspectedClosure,
    view: &CompletedView,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<(crate::ordered_rows::PreparedRows, ContentHash), ModelError> {
    let prepared = selected_prepared_member_result(closure, view, budget, audit).await?;
    Ok((prepared.rows.clone(), prepared.content))
}
async fn selected_prepared_member_result(
    closure: &InspectedClosure,
    view: &CompletedView,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<std::sync::Arc<PreparedSelectedMembers>, ModelError> {
    audit.pin.ensure_live()?;
    let selector = super::realization_id(view, &closure.mappings)?;
    let mut handles = StateCharge::new(budget, "selector preparation input handles");
    let mut runs = Vec::new();
    for mapping in closure
        .mappings
        .iter()
        .filter(|mapping| mapping.view == view.identity)
    {
        let physical = RecordId::new("compiler_contribution", mapping.physical.hex());
        let prepared = audit
            .membership_contributors
            .get(&physical)
            .ok_or(ModelError::Conflict("selected prepared contributor absent"))?;
        if let Some(run) = prepared.relation_memberships.relation(&view.relation) {
            handles
                .grow(std::mem::size_of::<crate::ordered_rows::PreparedRows>().saturating_mul(2))?;
            runs.push(run.clone());
        }
    }
    handles.grow(view.relation.len().saturating_add(128))?;
    let relation = view.relation.clone();
    let count = view.rows;
    audit
        .run_work((5, selector), move |owner| {
            async move {
                if owner.selected_members.contains_key(&selector) {
                    return Ok(());
                }
                let pin = owner.pin.clone();
                let (rows, content) = owner
                    .blocking(move |budget| {
                        let _handles = handles;
                        let rows = compose_selected_member_runs(
                            selector,
                            &relation,
                            &runs,
                            budget,
                            || pin.ensure_live(),
                        )?;
                        let content = crate::completed_state::earning::selected_membership_content(
                            &rows, count, budget,
                        )?;
                        Ok((rows, content))
                    })
                    .await?;
                owner.pin.ensure_live()?;
                owner.charge.grow(
                    std::mem::size_of::<(ContentHash, PreparedSelectedMembers)>()
                        .saturating_mul(11)
                        .saturating_add(256),
                )?;
                owner.selected_members.insert(
                    selector,
                    PreparedSelectedMembers {
                        rows,
                        count,
                        content,
                    },
                );
                Ok(())
            }
            .boxed()
        })
        .await?;
    audit.pin.ensure_live()?;
    let prepared = audit
        .selected_members
        .get(&selector)
        .ok_or(ModelError::Conflict(
            "prepared exact selector members absent",
        ))?;
    if prepared.count != view.rows {
        return Err(ModelError::Conflict(
            "prepared exact selector membership count changed",
        ));
    }
    Ok(prepared)
}

#[cfg(test)]
mod shared_preparation_controls {
    use super::*;
    #[test]
    fn backing_page_retry_preserves_accepted_prefix_and_splits_only_failed_slice() {
        use crate::control::held::PageAcquireFailure;
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut accepted = Vec::new();
        let mut attempts = Vec::new();
        for (offset, rows) in [(0, 2), (2, 4)] {
            let mut retries = BackingPageRetries::new(rows, &budget).unwrap();
            while let Some((start, end)) = retries.next() {
                attempts.push((start + offset, end + offset));
                if end - start > 2 {
                    retries
                        .refuse(
                            (start, end),
                            PageAcquireFailure::reducible_resource(
                                budget.reserve("test-page", budget.limit() + 1).unwrap_err(),
                            ),
                        )
                        .unwrap();
                } else {
                    accepted.extend(start + offset..end + offset);
                }
            }
        }
        assert_eq!(attempts, [(0, 2), (2, 6), (2, 4), (4, 6)]);
        assert_eq!(accepted, (0..6).collect::<Vec<_>>());
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn backing_page_retry_keeps_singleton_and_sorter_refusals_terminal() {
        use crate::control::held::PageAcquireFailure;
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut retries = BackingPageRetries::new(1, &budget).unwrap();
        let range = retries.next().unwrap();
        let error = retries
            .refuse(
                range,
                PageAcquireFailure::reducible_resource(
                    budget
                        .reserve("test-singleton", budget.limit() + 1)
                        .unwrap_err(),
                ),
            )
            .unwrap_err();
        assert!(retries.next().is_none());
        assert!(matches!(
            error,
            ModelError::Resource {
                owner: "test-singleton",
                ..
            }
        ));
        drop(retries);
        let mut retries = BackingPageRetries::new(8, &budget).unwrap();
        let range = retries.next().unwrap();
        let error = retries
            .refuse(
                range,
                PageAcquireFailure::Terminal(
                    budget
                        .reserve("test-sorter", budget.limit() + 1)
                        .unwrap_err(),
                ),
            )
            .unwrap_err();
        assert!(
            retries.next().is_none(),
            "terminal sorter refusal must not split or replay"
        );
        assert!(matches!(
            error,
            ModelError::Resource {
                owner: "test-sorter",
                ..
            }
        ));
        drop(retries);
        assert_eq!(budget.reserved(), 0);
    }
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    };
    fn relation_member(owner: &str, relation: &str, key: u8, content: &str) -> Value {
        let logical = ContentHash::of(owner.as_bytes());
        let key = [key; 16];
        let mut row = surrealdb::types::Object::new();
        row.insert(
            "id",
            crate::completed_rows::membership_id(logical, relation, &key),
        );
        row.insert(
            "contribution",
            RecordId::new("compiler_contribution", logical.hex()),
        );
        row.insert("relation", relation);
        row.insert("semantic_key", hex::encode(key));
        row.insert("node", RecordId::new("entity", hex::encode(key)));
        row.insert("content", ContentHash::of(content.as_bytes()).hex());
        row.insert("storage", "graph");
        row.insert("scope_keys", Vec::<Value>::new());
        row.insert("scope_lookup", Vec::<Value>::new());
        Value::Object(row)
    }
    fn membership_run(
        rows: impl IntoIterator<Item = Value>,
        budget: &ResourceBudget,
    ) -> crate::ordered_rows::PreparedRows {
        let mut sorted = crate::ordered_rows::SortedRows::with_budget_and_row_bytes(
            budget,
            d::resources::MAX_ROW_BYTES,
        )
        .unwrap();
        for row in rows {
            sorted.push(row).unwrap();
        }
        sorted.finish().unwrap().into_prepared()
    }
    fn membership_root(
        runs: &[crate::ordered_rows::PreparedRows],
        budget: &ResourceBudget,
    ) -> d::completed_identity::CompletedFamilyRoot {
        let mut hash = d::completed_identity::CompletedFamilyHasher::new(
            d::recovery_closure::RecoveryFamily::Memberships,
        )
        .unwrap();
        crate::ordered_rows::PreparedRows::fold_union(
            runs,
            budget,
            || Ok(()),
            |_, bytes| hash.row(bytes),
        )
        .unwrap();
        hash.finish()
    }
    #[test]
    fn relation_preparation_selects_only_own_rows_preserves_full_hash_and_large_frames() {
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let mut large = relation_member("same-logical-copy", "unrelated", 9, "large");
        // The derived layout must retain the canonical source's admitted frame bound.
        if let Value::Object(row) = &mut large {
            row.insert(
                "node",
                RecordId::new("entity", "x".repeat(crate::ordered_rows::ROW_BYTES + 512)),
            );
        }
        let full = membership_run(
            [
                large,
                relation_member("same-logical-copy", "packages", 2, "second"),
                relation_member("same-logical-copy", "releases", 3, "release"),
                relation_member("same-logical-copy", "packages", 1, "first"),
            ],
            &budget,
        );
        let before = membership_root(std::slice::from_ref(&full), &budget);
        let access =
            Arc::new(PreparedMembershipRelations::prepare(&full, &budget, || Ok(())).unwrap());
        assert!(access.relation("explicit_empty").is_none());
        let selected = access.relation("packages").unwrap().clone();
        let all = access.rows.values().cloned().collect::<Vec<_>>();
        assert_eq!(membership_root(&all, &budget), before);
        assert_eq!(
            membership_root(std::slice::from_ref(&full), &budget),
            before
        );
        let selector = ContentHash::of(b"selective packages");
        let mut checks = 0;
        let members = compose_selected_member_runs(
            selector,
            "packages",
            std::slice::from_ref(&selected),
            &budget,
            || {
                checks += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            checks, 9,
            "two requested rows, placement grouping, both EOFs and final checks"
        );
        crate::completed_state::earning::selected_membership_content(&members, 2, &budget).unwrap();
        let empty =
            compose_selected_member_runs(selector, "explicit_empty", &[], &budget, || Ok(()))
                .unwrap();
        crate::completed_state::earning::selected_membership_content(&empty, 0, &budget).unwrap();
        drop(empty);
        drop(members);
        drop(all);
        drop(access);
        drop(full);
        let mut cursor = selected.cursor_with_budget(&budget).unwrap();
        drop(selected);
        let mut count = 0;
        while let Some(row) = cursor.next_row().unwrap() {
            assert_eq!(
                row.as_object().unwrap().get("relation"),
                Some(&Value::String("packages".into()))
            );
            count += 1;
        }
        assert_eq!(
            count, 2,
            "a last borrower keeps only the requested prepared run alive"
        );
        drop(cursor);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn relation_selector_preserves_overlap_dedup_and_unequal_collision_refusal() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let first_full = membership_run(
            [relation_member("logical-one", "packages", 1, "equal")],
            &budget,
        );
        let second_full = membership_run(
            [relation_member("logical-two", "packages", 1, "equal")],
            &budget,
        );
        let unequal_full = membership_run(
            [relation_member("logical-two", "packages", 1, "unequal")],
            &budget,
        );
        let first = PreparedMembershipRelations::prepare(&first_full, &budget, || Ok(())).unwrap();
        let second =
            PreparedMembershipRelations::prepare(&second_full, &budget, || Ok(())).unwrap();
        let unequal =
            PreparedMembershipRelations::prepare(&unequal_full, &budget, || Ok(())).unwrap();
        drop(first_full);
        drop(second_full);
        drop(unequal_full);
        let selector = ContentHash::of(b"overlap selector");
        let equal = compose_selected_member_runs(
            selector,
            "packages",
            &[
                first.relation("packages").unwrap().clone(),
                first.relation("packages").unwrap().clone(),
                second.relation("packages").unwrap().clone(),
            ],
            &budget,
            || Ok(()),
        )
        .unwrap();
        crate::completed_state::earning::selected_membership_content(&equal, 1, &budget).unwrap();
        let retained = budget.reserved();
        assert!(matches!(
            compose_selected_member_runs(
                selector,
                "packages",
                &[
                    first.relation("packages").unwrap().clone(),
                    unequal.relation("packages").unwrap().clone(),
                ],
                &budget,
                || Ok(())
            ),
            Err(ModelError::Conflict("conflicting physical row identity"))
        ));
        assert_eq!(
            budget.reserved(),
            retained,
            "failed projection releases its provisional run"
        );
        assert!(
            compose_selected_member_runs(
                selector,
                "releases",
                &[first.relation("packages").unwrap().clone(),],
                &budget,
                || Ok(())
            )
            .is_err()
        );
        drop(equal);
        drop(unequal);
        drop(second);
        drop(first);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn selector_chooses_one_native_locator_without_changing_logical_membership_content() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let make = |owner, packing: &[u8], slot| {
            let mut row = relation_member(owner, "packages", 1, "equal");
            let Value::Object(ref mut object) = row else {
                unreachable!()
            };
            object.insert(
                "node",
                RecordId::new("compiler_record", ContentHash::of(b"logical row").hex()),
            );
            object.insert("storage", "block");
            object.insert(
                "block",
                RecordId::new("compiler_block", ContentHash::of(packing).hex()),
            );
            object.insert("slot", slot);
            row
        };
        let one = make("one", b"first packing", 3i64);
        let two = make("two", b"second packing", 7i64);
        let first = membership_run([one.clone()], &budget);
        let second = membership_run([two.clone()], &budget);
        let selector = ContentHash::of(b"different packing selector");
        let a = compose_selected_member_runs(
            selector,
            "packages",
            &[first.clone(), second.clone()],
            &budget,
            || Ok(()),
        )
        .unwrap();
        let b = compose_selected_member_runs(
            selector,
            "packages",
            &[second, first],
            &budget,
            || Ok(()),
        )
        .unwrap();
        let mut ac = a.cursor_with_budget(&budget).unwrap();
        let mut bc = b.cursor_with_budget(&budget).unwrap();
        let actual = ac.next_row().unwrap().unwrap();
        assert_eq!(Some(actual.clone()), bc.next_row().unwrap());
        assert!(ac.next_row().unwrap().is_none());
        assert!(bc.next_row().unwrap().is_none());
        let object = actual.as_object().unwrap();
        assert_eq!(object.get("node"), one.as_object().unwrap().get("node"));
        let chosen = object.get("locator").unwrap();
        let chosen_id = chosen.as_object().unwrap().get("id").unwrap();
        assert_eq!(
            chosen_id,
            std::cmp::min(
                one.as_object().unwrap().get("id").unwrap(),
                two.as_object().unwrap().get("id").unwrap()
            )
        );
        let logical = crate::completed_state::earning::member(selector, &one).unwrap();
        let mut expected = d::KeySink::new("native-selected-membership/v3");
        expected.part(b"member", &serde_json::to_vec(&logical).unwrap());
        expected.part(b"rows", &1u64.to_le_bytes());
        assert_eq!(
            crate::completed_state::earning::selected_membership_content(&a, 1, &budget).unwrap(),
            expected.finish(),
            "selected membership identity excludes physical packing and locator choice"
        );
    }
    #[test]
    fn relation_preparation_refuses_late_cancellation_and_resource_without_publishing() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let full = membership_run(
            [
                relation_member("logical", "packages", 1, "one"),
                relation_member("logical", "releases", 2, "two"),
            ],
            &budget,
        );
        let retained = budget.reserved();
        let flag = Arc::new(AtomicBool::new(false));
        let mut checks = 0;
        let refused = PreparedMembershipRelations::prepare(&full, &budget, || {
            checks += 1;
            if checks == 5 {
                flag.store(true, Ordering::Release);
            }
            crate::prepared::check_read_cancellation(Some(&flag))
        });
        assert!(
            refused.is_err(),
            "cancellation after canonical EOF publishes no access layout"
        );
        assert_eq!(budget.reserved(), retained);
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            PreparedMembershipRelations::prepare(&full, &tiny, || Ok(())),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(tiny.reserved(), 0);
        let access = PreparedMembershipRelations::prepare(&full, &budget, || Ok(())).unwrap();
        let retained = budget.reserved();
        let flag = Arc::new(AtomicBool::new(false));
        let mut checks = 0;
        assert!(
            compose_selected_member_runs(
                ContentHash::of(b"late selective cancellation"),
                "packages",
                std::slice::from_ref(access.relation("packages").unwrap()),
                &budget,
                || {
                    checks += 1;
                    if checks == 5 {
                        flag.store(true, Ordering::Release);
                    }
                    crate::prepared::check_read_cancellation(Some(&flag))
                },
            )
            .is_err()
        );
        assert_eq!(
            budget.reserved(),
            retained,
            "late refusal drops the completed provisional selector"
        );
        drop(access);
        drop(full);
        assert_eq!(budget.reserved(), 0);
    }
    fn exact_alias(source: &RecordId, target: &RecordId) -> Value {
        let mut sink = d::KeySink::new("native-place-alias/v1");
        sink.part(b"source", &serde_json::to_vec(source).unwrap());
        sink.part(b"target", &serde_json::to_vec(target).unwrap());
        let mut object = surrealdb::types::Object::new();
        object.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
        object.insert("source", source.clone());
        object.insert("target", target.clone());
        Value::Object(object)
    }
    #[test]
    fn alias_window_requests_deduplicate_targets_and_keep_exact_heterogeneous_holds() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let owner = RecordId::new("compiler_contribution", "owner");
        let source = RecordId::new("entity", "source");
        let other = RecordId::new("entity", "other");
        let target = RecordId::new("entity", "target");
        let alias = exact_alias(&source, &target);
        let second = exact_alias(&other, &target);
        let mut requests = AliasWindowRequests::new(&budget);
        requests.observe(&alias, &owner).unwrap();
        let retained = budget.reserved();
        requests.observe(&alias, &owner).unwrap();
        assert_eq!(budget.reserved(), retained);
        requests.observe(&second, &owner).unwrap();
        assert_eq!(requests.targets, BTreeSet::from([target.clone()]));
        let id = |row: &Value| match row.as_object().unwrap().get("id").unwrap() {
            Value::RecordId(id) => id.clone(),
            _ => panic!("alias ID"),
        };
        assert_eq!(
            requests.holds,
            BTreeSet::from([(id(&alias), source), (id(&second), other), (owner, target),])
        );
        drop(requests);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn alias_window_requests_refuse_foreign_endpoints_and_noncanonical_envelopes() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let owner = RecordId::new("compiler_contribution", "owner");
        let source = RecordId::new("entity", "source");
        let target = RecordId::new("entity", "target");
        let mut requests = AliasWindowRequests::new(&budget);
        assert!(matches!(
            requests.observe(
                &exact_alias(&RecordId::new("compiler_record", "foreign"), &target),
                &owner
            ),
            Err(ModelError::Conflict("installed alias entity family"))
        ));
        let mut forged = exact_alias(&source, &target);
        if let Value::Object(object) = &mut forged {
            object.insert("id", RecordId::new("compiler_alias", "wrong"));
        }
        assert!(matches!(
            requests.observe(&forged, &owner),
            Err(ModelError::Conflict("installed exact alias envelope"))
        ));
        assert!(requests.targets.is_empty());
        assert!(requests.holds.is_empty());
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn backing_window_resource_retry_never_erases_completion_or_context() {
        let resource = || ModelError::Resource {
            owner: "backing retry control",
            requested: 2,
            used: 1,
            limit: 1,
        };
        let shared =
            ModelError::SharedCause(Arc::new(ModelError::SharedCause(Arc::new(resource()))));
        assert!(terminal_local_resource(&shared));
        assert!(!terminal_local_resource(&ModelError::Cause(Box::new(
            resource()
        ))));
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "failed backing drainage",
            Err(ModelError::Conflict("terminal refusal")),
        );
        let failed = d::completion::complete::<()>(Err(resource()), terminal).unwrap_err();
        assert!(!terminal_local_resource(&failed));
        assert!(!terminal_local_resource(&ModelError::SharedCause(
            Arc::new(failed)
        )));
        assert!(!terminal_local_resource(&ModelError::SharedCause(
            Arc::new(ModelError::Conflict("changed backing"))
        )));
    }
    pub(super) fn test_audit(bytes: usize) -> InstallationAudit {
        let budget = ResourceBudget::fixed(bytes).unwrap();
        InstallationAudit {
            client: Arc::new(surrealdb::Surreal::init()),
            pin: crate::control::ReaderPin::test_detached(false),
            contributors: SharedMap::new(),
            inspected_contributors: SharedMap::new(),
            inspected_views: SharedMap::new(),
            inspected_holds: SharedMap::new(),
            recovery_contributors: SharedMap::new(),
            recovery_views: SharedMap::new(),
            logical_contributors: SharedMap::new(),
            membership_contributors: SharedMap::new(),
            family_roots: SharedMap::new(),
            selected_members: SharedMap::new(),
            aliases: SharedMap::new(),
            local_aliases: SharedMap::new(),
            blobs: SharedMap::new(),
            budget: budget.clone(),
            charge: SharedCharge(Arc::new(std::sync::Mutex::new(StateCharge::new(
                &budget,
                "pure shared preparation control",
            )))),
            blob_observations: Arc::new(AtomicU64::new(0)),
            contribution_observations: Arc::new(AtomicU64::new(0)),
            logical_contribution_observations: Arc::new(AtomicU64::new(0)),
            alias_observations: Arc::new(AtomicU64::new(0)),
            context_observations: Arc::new(AtomicU64::new(0)),
            closed: Arc::new(AtomicBool::new(false)),
            context: (ContentHash([1; 32]), 1),
            worker: false,
            work: Arc::new(WorkOwner {
                flights: std::sync::Mutex::new(BTreeMap::new()),
                workers: std::sync::Mutex::new(Vec::new()),
                draining: tokio::sync::Mutex::new(WorkerDrain::default()),
                next: AtomicU64::new(1),
                accepting: AtomicBool::new(true),
                close_gate: tokio::sync::Mutex::new(()),
                context_checked: std::sync::atomic::AtomicBool::new(false),
                context_gate: tokio::sync::Mutex::new(()),
                blocking: std::sync::Mutex::new(Vec::new()),
            }),
        }
    }
    #[tokio::test]
    async fn lost_blocking_waiter_close_joins_late_semantic_failure_before_releasing() {
        let audit = test_audit(1 << 20);
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let worker_audit = audit.clone();
        let waiter = tokio::spawn(async move {
            worker_audit
                .blocking::<()>(move |_| {
                    let _ = entered_tx.send(());
                    gate.recv().map_err(ModelError::codec)?;
                    Err(ModelError::Conflict("late blocking semantic refusal"))
                })
                .await
        });
        entered.await.unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        let released = Arc::new(AtomicBool::new(false));
        let release_observed = released.clone();
        let mut close = Box::pin(audit.close_releasing(async move {
            release_observed.store(true, Ordering::Release);
            Ok(())
        }));
        assert!(futures::poll!(&mut close).is_pending());
        assert!(!released.load(Ordering::Acquire));
        assert!(!audit.closed.load(Ordering::Acquire));
        assert!(audit.budget.reserved() > 0);
        // Independent async work progresses while close owns the paused worker's join.
        assert_eq!(tokio::spawn(async { 19 }).await.unwrap(), 19);
        release.send(()).unwrap();
        let error = close.await.unwrap_err();
        assert!(error.to_string().contains("late blocking semantic refusal"));
        assert!(error.permits_storage_cleanup());
        assert!(released.load(Ordering::Acquire));
        assert!(audit.closed.load(Ordering::Acquire));
        assert!(!audit.pin.test_uncertain());
        assert_eq!(
            audit.budget.reserved(),
            audit.charge.0.lock().unwrap().reserved()
        );
    }
    #[tokio::test]
    async fn unconfirmed_blocking_panic_close_retains_pin_without_polling_release() {
        let audit = test_audit(1 << 20);
        let error = audit
            .blocking::<()>(|_| panic!("controlled blocking unit panic"))
            .await
            .unwrap_err();
        assert!(!error.permits_storage_cleanup());
        let released = Arc::new(AtomicBool::new(false));
        let release_observed = released.clone();
        let error = audit
            .close_releasing(async move {
                release_observed.store(true, Ordering::Release);
                Ok(())
            })
            .await
            .unwrap_err();
        assert!(!error.permits_storage_cleanup());
        assert!(!released.load(Ordering::Acquire));
        assert!(!audit.closed.load(Ordering::Acquire));
        assert!(audit.pin.test_uncertain());
        assert_eq!(
            audit.budget.reserved(),
            audit.charge.0.lock().unwrap().reserved()
        );
    }
    async fn join(audit: &InstallationAudit) {
        audit.drain_workers().await.unwrap();
        audit.closed.store(true, Ordering::Release);
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn contributor_window_polls_blocked_peers_and_reports_reverse_failures_in_source_order() {
        let audit = test_audit(1 << 20);
        let first_release = Arc::new(tokio::sync::Semaphore::new(0));
        let (second_done, observed) = tokio::sync::oneshot::channel();
        let mut window = ContributorWindow::new(&audit.budget);
        let first = audit.clone();
        let release = first_release.clone();
        window
            .push(async move {
                first
                    .run_work((0, ContentHash([61; 32])), move |_| {
                        async move {
                            let _permit = release.acquire().await.unwrap();
                            Err(ModelError::Conflict("first contributor terminal refusal"))
                        }
                        .boxed()
                    })
                    .await
            })
            .await
            .unwrap();
        let second = audit.clone();
        window
            .push(async move {
                second
                    .run_work((0, ContentHash([62; 32])), move |_| {
                        async move {
                            let _ = second_done.send(());
                            Err(ModelError::Conflict("second contributor terminal refusal"))
                        }
                        .boxed()
                    })
                    .await
            })
            .await
            .unwrap();
        let finishing = tokio::spawn(async move { window.finish().await });
        tokio::time::timeout(std::time::Duration::from_secs(30), observed)
            .await
            .unwrap()
            .unwrap();
        assert!(
            !finishing.is_finished(),
            "late peer failure cannot abandon accepted first worker"
        );
        first_release.add_permits(1);
        let error = finishing.await.unwrap().unwrap_err();
        let text = error.to_string();
        assert!(
            text.find("first contributor terminal refusal").unwrap()
                < text.find("second contributor terminal refusal").unwrap()
        );
        assert!(error.permits_storage_cleanup());
        assert!(audit.work.workers.lock().unwrap().is_empty());
        assert!(
            audit.work.flights.lock().unwrap().is_empty(),
            "failed observations are never reusable"
        );
        join(&audit).await;
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn contributor_window_cancelled_waiter_retains_actual_capture_until_worker_drain() {
        let audit = test_audit(1 << 20);
        let (started, observed) = tokio::sync::oneshot::channel();
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let mut window = ContributorWindow::new(&audit.budget);
        let owner = audit.clone();
        let gate = release.clone();
        let capture = audit
            .budget
            .reserve("contributor lifetime control", 16384)
            .unwrap();
        window
            .push(async move {
                owner
                    .run_work((0, ContentHash([63; 32])), move |_| {
                        async move {
                            let _capture = capture;
                            let _ = started.send(());
                            let _permit = gate.acquire().await.unwrap();
                            Ok(())
                        }
                        .boxed()
                    })
                    .await
            })
            .await
            .unwrap();
        let waiter = tokio::spawn(async move { window.finish().await });
        observed.await.unwrap();
        waiter.abort();
        let _ = waiter.await;
        assert!(
            audit.budget.reserved() >= 16384,
            "capture follows actual accepted worker"
        );
        let closing = {
            let audit = audit.clone();
            tokio::spawn(async move { audit.drain_workers().await })
        };
        tokio::task::yield_now().await;
        assert!(!closing.is_finished());
        release.add_permits(1);
        closing.await.unwrap().unwrap();
        assert!(!audit.pin.test_uncertain());
        audit.closed.store(true, Ordering::Release);
        let budget = audit.budget.clone();
        drop(audit);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn contributor_window_admission_refusal_runs_no_unadmitted_work() {
        let audit = test_audit(1);
        let calls = Arc::new(AtomicU64::new(0));
        let seen = calls.clone();
        let mut window = ContributorWindow::new(&audit.budget);
        assert!(matches!(
            window
                .push(async move {
                    seen.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                })
                .await,
            Err(ModelError::Resource { .. })
        ));
        window.finish().await.unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert!(audit.work.workers.lock().unwrap().is_empty());
        join(&audit).await;
        assert_eq!(audit.budget.reserved(), 0);
    }
    #[tokio::test]
    async fn contributor_window_known_slot_refusal_progresses_only_through_owned_results() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let count = Arc::new(AtomicU64::new(0));
        let mut window = ContributorWindow::new(&budget);
        let first = count.clone();
        window
            .push(async move {
                first.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
            .await
            .unwrap();
        let second = count.clone();
        let capture = [7_u8; 2048];
        window
            .push(async move {
                std::hint::black_box(capture);
                second.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            count.load(Ordering::Relaxed),
            1,
            "owned predecessor releases its waiter slot before admission retries"
        );
        window.finish().await.unwrap();
        assert_eq!(count.load(Ordering::Relaxed), 2);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn contributor_window_dynamic_refusal_is_terminal_without_kernel_replay() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let count = Arc::new(AtomicU64::new(0));
        let mut window = ContributorWindow::new(&budget);
        let first = count.clone();
        window
            .push(async move {
                first.fetch_add(1, Ordering::Relaxed);
                Err(ModelError::Resource {
                    owner: "dynamic contributor kernel",
                    requested: 8192,
                    used: 0,
                    limit: 4096,
                })
            })
            .await
            .unwrap();
        let second = count.clone();
        let capture = [7_u8; 2048];
        let result = window
            .push(async move {
                std::hint::black_box(capture);
                second.fetch_add(100, Ordering::Relaxed);
                Ok(())
            })
            .await;
        assert!(result.is_err());
        assert_eq!(
            count.load(Ordering::Relaxed),
            1,
            "kernel refused once; unadmitted successor did not run"
        );
        window.finish().await.unwrap();
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn shared_exact_worker_survives_waiter_cancellation_and_distinct_keys_run_independently()
    {
        let audit = test_audit(1 << 20);
        let entered = Arc::new(tokio::sync::Barrier::new(3));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let count = Arc::new(AtomicU64::new(0));
        let spawn = |key: ContentHash| {
            let audit = audit.clone();
            let entered = entered.clone();
            let release = release.clone();
            let count = count.clone();
            tokio::spawn(async move {
                audit
                    .run_work((0, key), move |_| {
                        async move {
                            count.fetch_add(1, Ordering::Relaxed);
                            entered.wait().await;
                            let _permit = release.acquire().await.unwrap();
                            Ok(())
                        }
                        .boxed()
                    })
                    .await
            })
        };
        let cancelled = spawn(ContentHash([2; 32]));
        let independent = spawn(ContentHash([3; 32]));
        tokio::time::timeout(std::time::Duration::from_secs(30), entered.wait())
            .await
            .unwrap();
        let peer = {
            let audit = audit.clone();
            tokio::spawn(async move {
                audit
                    .run_work((0, ContentHash([2; 32])), move |_| {
                        async {
                            panic!("same exact input must reuse its independently owned worker")
                        }
                        .boxed()
                    })
                    .await
            })
        };
        cancelled.abort();
        let _ = cancelled.await;
        release.add_permits(2);
        independent.await.unwrap().unwrap();
        peer.await.unwrap().unwrap();
        assert_eq!(count.load(Ordering::Relaxed), 2);
        join(&audit).await;
        assert!(!audit.pin.test_uncertain());
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn failed_exact_work_is_evicted_before_retry_and_admission_refusal_spawns_nothing() {
        let audit = test_audit(1 << 20);
        let key = (1, ContentHash([4; 32]));
        let failed = audit
            .run_work(key, move |_| {
                async { Err(ModelError::Conflict("late complete-input refusal")) }.boxed()
            })
            .await
            .unwrap_err();
        assert!(failed.permits_storage_cleanup());
        assert!(!audit.work.flights.lock().unwrap().contains_key(&key));
        audit
            .run_work(key, move |_| async { Ok(()) }.boxed())
            .await
            .unwrap();
        assert!(audit.work.flights.lock().unwrap().contains_key(&key));
        audit.work.accepting.store(false, Ordering::Release);
        assert!(
            audit
                .run_work((1, ContentHash([5; 32])), move |_| async {
                    panic!("closed admission cannot spawn")
                }
                .boxed())
                .await
                .is_err()
        );
        join(&audit).await;
        let insufficient = test_audit(4095);
        assert!(matches!(
            insufficient
                .run_work((1, ContentHash([6; 32])), move |_| async {
                    panic!("unreserved work cannot spawn")
                }
                .boxed())
                .await,
            Err(ModelError::Resource { .. })
        ));
        assert!(insufficient.work.flights.lock().unwrap().is_empty());
        assert!(insufficient.work.workers.lock().unwrap().is_empty());
        join(&insufficient).await;
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn overlapping_windows_share_item_flights_and_deliver_late_failure_after_cancellation() {
        let audit = test_audit(1 << 20);
        let keys = [
            (6, ContentHash([10; 32])),
            (7, ContentHash([11; 32])),
            (8, ContentHash([12; 32])),
        ];
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let seen = Arc::new(std::sync::Mutex::new(BTreeSet::new()));
        let first = {
            let audit = audit.clone();
            let entered = entered.clone();
            let release = release.clone();
            let seen = seen.clone();
            tokio::spawn(async move {
                audit
                    .run_work_many(keys[..2].to_vec(), move |_, claimed| {
                        async move {
                            assert_eq!(claimed, keys[..2]);
                            seen.lock().unwrap().extend(claimed);
                            entered.wait().await;
                            let _permit = release.acquire().await.unwrap();
                            Err(ModelError::Conflict("late window refusal"))
                        }
                        .boxed()
                    })
                    .await
            })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), entered.wait())
            .await
            .unwrap();
        let peer_claimed = Arc::new(tokio::sync::Barrier::new(2));
        let peer = {
            let audit = audit.clone();
            let seen = seen.clone();
            let peer_claimed = peer_claimed.clone();
            tokio::spawn(async move {
                audit
                    .run_work_many(keys.to_vec(), move |_, claimed| {
                        async move {
                            assert_eq!(claimed, vec![keys[2]]);
                            seen.lock().unwrap().extend(claimed);
                            peer_claimed.wait().await;
                            Ok(())
                        }
                        .boxed()
                    })
                    .await
            })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), peer_claimed.wait())
            .await
            .unwrap();
        first.abort();
        let _ = first.await;
        release.add_permits(1);
        assert!(peer.await.unwrap().unwrap_err().permits_storage_cleanup());
        join(&audit).await;
        assert_eq!(*seen.lock().unwrap(), BTreeSet::from(keys));
        let flights = audit.work.flights.lock().unwrap();
        assert!(!flights.contains_key(&keys[0]) && !flights.contains_key(&keys[1]));
        assert!(flights.contains_key(&keys[2]));
        assert_eq!(
            audit.budget.reserved(),
            4096 + audit.charge.0.lock().unwrap().accounted_bytes()
        );
        assert!(!audit.pin.test_uncertain());
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn failed_window_flight_reservations_release_before_repeated_resource_retry() {
        let audit = test_audit(16384);
        let keys = vec![(6, ContentHash([20; 32])), (7, ContentHash([21; 32]))];
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let live = {
            let audit = audit.clone();
            let entered = entered.clone();
            let release = release.clone();
            tokio::spawn(async move {
                audit
                    .run_work((8, ContentHash([22; 32])), move |_| {
                        async move {
                            entered.wait().await;
                            let _permit = release.acquire().await.unwrap();
                            Ok(())
                        }
                        .boxed()
                    })
                    .await
            })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), entered.wait())
            .await
            .unwrap();
        let mut retained = None;
        for _ in 0..4 {
            let retry = audit.run_work_many(keys.clone(), move |_, _| {
                async {
                    Err(ModelError::Resource {
                        owner: "window control",
                        requested: 1,
                        used: 8192,
                        limit: 8192,
                    })
                }
                .boxed()
            });
            let error = tokio::time::timeout(std::time::Duration::from_secs(30), retry)
                .await
                .unwrap()
                .unwrap_err();
            assert!(terminal_local_resource(&error));
            let flights = audit.work.flights.lock().unwrap();
            assert_eq!(flights.len(), 1);
            assert!(keys.iter().all(|key| !flights.contains_key(key)));
            drop(flights);
            assert!(!live.is_finished());
            let bytes = audit.budget.reserved();
            assert_eq!(
                bytes,
                4096 + audit.charge.0.lock().unwrap().accounted_bytes()
            );
            assert_eq!(*retained.get_or_insert(bytes), bytes);
        }
        release.add_permits(1);
        live.await.unwrap().unwrap();
        audit
            .run_work_many(keys, move |_, _| async { Ok(()) }.boxed())
            .await
            .unwrap();
        join(&audit).await;
        assert_eq!(
            audit.budget.reserved(),
            12288 + audit.charge.0.lock().unwrap().accounted_bytes()
        );
        assert!(!audit.pin.test_uncertain());
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn nested_preparation_reaps_terminal_child_without_joining_current_worker() {
        let audit = test_audit(1 << 20);
        let result = audit.run_work((6, ContentHash([30; 32])), move |owner| {
            async move {
                let error = owner
                    .run_work((7, ContentHash([31; 32])), move |_| {
                        async {
                            Err(ModelError::Resource {
                                owner: "nested control",
                                requested: 2,
                                used: 1,
                                limit: 1,
                            })
                        }
                        .boxed()
                    })
                    .await
                    .unwrap_err();
                assert!(terminal_local_resource(&error));
                Ok(())
            }
            .boxed()
        });
        tokio::time::timeout(std::time::Duration::from_secs(30), result)
            .await
            .unwrap()
            .unwrap();
        join(&audit).await;
        assert_eq!(audit.work.flights.lock().unwrap().len(), 1);
        assert!(!audit.pin.test_uncertain());
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn cancelled_close_retains_pending_worker_and_prior_join_failure_until_retry_drains() {
        let audit = test_audit(1 << 20);
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let worker = {
            let audit = audit.clone();
            let entered = entered.clone();
            let release = release.clone();
            tokio::spawn(async move {
                audit
                    .run_work((6, ContentHash([40; 32])), move |_| {
                        async move {
                            entered.wait().await;
                            let _permit = release.acquire().await.unwrap();
                            Ok(())
                        }
                        .boxed()
                    })
                    .await
            })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), entered.wait())
            .await
            .unwrap();
        let failed_entered = Arc::new(tokio::sync::Barrier::new(2));
        let fail = Arc::new(tokio::sync::Semaphore::new(0));
        let failed = {
            let audit = audit.clone();
            let entered = failed_entered.clone();
            let fail = fail.clone();
            tokio::spawn(async move {
                audit
                    .run_work((7, ContentHash([41; 32])), move |_| {
                        async move {
                            entered.wait().await;
                            let _permit = fail.acquire().await.unwrap();
                            panic!("deterministic drain join failure")
                        }
                        .boxed()
                    })
                    .await
            })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), failed_entered.wait())
            .await
            .unwrap();
        failed.abort();
        let _ = failed.await;
        fail.add_permits(1);
        // Observe terminality without joining/removing the production owner's handle.
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                if audit
                    .work
                    .workers
                    .lock()
                    .unwrap()
                    .last()
                    .unwrap()
                    .1
                    .is_finished()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let mut first = Box::pin(audit.close_drained());
        assert!(futures::poll!(first.as_mut()).is_pending());
        drop(first);
        {
            let pending = audit.work.draining.lock().await;
            assert_eq!(pending.workers.len(), 1);
            assert_eq!(pending.terminal.failures.len(), 1);
            assert_eq!(pending.terminal.remote, d::completion::RemoteState::Unknown);
        }
        let mut retry = Box::pin(audit.close_drained());
        assert!(futures::poll!(retry.as_mut()).is_pending());
        assert!(!worker.is_finished());
        release.add_permits(1);
        worker.await.unwrap().unwrap();
        let error = tokio::time::timeout(std::time::Duration::from_secs(30), retry)
            .await
            .unwrap()
            .unwrap_err();
        assert!(!error.permits_storage_cleanup());
        assert!(format!("{error:?}").contains("installation audit worker join"));
        assert!(audit.work.draining.lock().await.workers.is_empty());
        assert!(audit.pin.test_uncertain());
        // This pure control qualifies worker drainage and retained failure, not native
        // pin release. Unknown prevents the detached pin's release from being attempted.
        audit.closed.store(true, Ordering::Release);
    }
}

/// Share the independently retained exact native row instead of copying every
/// cumulative selector's full contributor descriptors and their reservations.
pub(crate) fn protected_contributors(
    closure: &InspectedClosure,
    audit: &InstallationAudit,
) -> Result<BTreeMap<ContentHash, std::sync::Arc<ExpectedRecord>>, ModelError> {
    audit.pin.ensure_live()?;
    closure
        .contributors
        .iter()
        .map(|(physical, expected)| {
            let protected =
                audit
                    .contributors
                    .get(expected.record())
                    .ok_or(ModelError::Conflict(
                        "earned exact contributor not prepared",
                    ))?;
            if protected.incarnation() != expected.incarnation()
                || protected.row() != expected.row()
            {
                return Err(ModelError::Conflict(
                    "earned exact contributor premise changed",
                ));
            }
            Ok((*physical, protected))
        })
        .collect()
}

#[cfg(test)]
mod closure_metadata_controls {
    use super::*;
    use std::sync::Arc;
    fn descriptor() -> CompletedContribution {
        CompletedContribution {
            spec: d::completed::ContributionSpec {
                captured_binding: None,
                producer: "wide".repeat(16384),
                profile: d::completed::OutputProfile::Specific(d::stages::Profile::Catalog),
                model: ContentHash([1; 32]),
                implementation: ContentHash([2; 32]),
                configuration: None,
                inputs: Vec::new(),
                outputs: ["packages".into()].into(),
            },
            assurance: d::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            outcome: 0,
            outputs: [(
                "packages".into(),
                d::completed::OutputContent {
                    rows: 0,
                    content: ContentHash([3; 32]),
                },
            )]
            .into(),
        }
    }
    fn require_context_campaign_maintenance(config: &crate::RuntimeConfig) {
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let proof = std::process::Command::new(repository.join(".venv/bin/python"))
            .env("PYTHONPATH", repository.join("scripts"))
            .args(["-c", r#"
import os
from pathlib import Path
import sys
import surrealdb_service as service
installation = service.Installation.load(ready=False)
record = installation.record
assert Path(os.environ["LCTX_SURREAL_SERVICE_CONFIG"]).resolve() == (installation.directory / "installation.json").resolve()
assert record.get("kind") == "campaign" and record.get("purpose") == "surrealdb-lifecycle"
assert installation.id == os.environ["LCTX_SURREAL_INSTALLATION_ID"]
endpoint, namespace, generation, database = sys.argv[1:]
assert installation.grpc_endpoint == endpoint and record["namespace"] == namespace
assert bytes(record["service_generation"]).hex() == generation == os.environ["LCTX_SURREAL_GENERATION"]
assert database in record["databases"]
assert Path(os.environ["LCTX_SURREAL_INSTALLER_CONFIG"]).resolve() == installation.runtime_path(database, installer=True).resolve()
assert service.maintenance_owned(installation), "exclusive owned campaign maintenance required"
"#])
            .arg(&config.endpoint).arg(config.namespace.as_str())
            .arg(config.service_generation.hex()).arg(config.database.as_str())
            .output().unwrap();
        assert!(
            proof.status.success(),
            "context drift maintenance ownership refused: {}",
            String::from_utf8_lossy(&proof.stderr)
        );
    }
    async fn boundary_context_control(drift: bool) {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native campaign"),
        ))
        .unwrap();
        // The destructive case proves maintenance before creating even an attempt.
        if drift {
            require_context_campaign_maintenance(&config);
        }
        // The dev dependency uses the ordinary crate instance. Read its typed
        // configuration independently rather than treating unit-test types as authority.
        let store_config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native campaign"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(64 << 20).unwrap();
        let store = NativeCompilerStore::begin(
            &store_config,
            d::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(&store_config, budget.clone())
                .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let (client, observer_factory, observer_lease) = tests::inspection_session(&budget).await;
        let reader = NativeReader::private(client.clone()).with_budget(&budget);
        let audit = crate::control::with_recovery_scope(
            observer_lease.recovery_scope().clone(),
            InstallationAudit::new(client.clone(), &budget),
        )
        .await
        .unwrap();
        let mut admin = None;
        let mut admin_lease = None;
        let mut restore_epoch = None;
        let mut boundary_owner = None;
        let result = crate::control::with_recovery_scope(observer_lease.recovery_scope().clone(), std::panic::AssertUnwindSafe(async {
            let mut stale = InstallationAudit::new(client.clone(), &budget).await.unwrap();
            // Model an observation captured before pin issuance which no longer
            // matches the actual post-pin context, without changing shared state.
            stale.context.1 = stale.context.1.checked_add(1).unwrap();
            let mismatch = stale.ensure_context(&reader, &budget).await.unwrap_err();
            assert!(mismatch.to_string().contains("installation audit validity context changed"));
            assert_eq!(stale.context_observations.load(std::sync::atomic::Ordering::Relaxed), 1);
            assert!(!stale.work.context_checked.load(std::sync::atomic::Ordering::Acquire));
            stale.close().await.unwrap();
            let source = audit.protected_source(&client, &budget).await.unwrap();
            let mut scope = source.inspection_scope(&reader, &budget).unwrap().for_boundary();
            let before = audit.context_observations.load(std::sync::atomic::Ordering::Relaxed);
            assert_eq!(before, 1, "protected source earns exactly one actual post-pin context observation");
            scope.check(&reader, &audit, &budget).await.unwrap();
            let first = audit.context_observations.load(std::sync::atomic::Ordering::Relaxed);
            assert_eq!(first, before, "phase entry consumes the already checked actual pin lifetime");
            for _ in 0..64 { scope.check(&reader, &audit, &budget).await.unwrap(); }
            assert_eq!(audit.context_observations.load(std::sync::atomic::Ordering::Relaxed), first,
                "selector reuse keeps pin/client/pool/source checks without repeated context queries");
            let mut ordinary = source.inspection_scope(&reader, &budget).unwrap();
            ordinary.check(&reader, &audit, &budget).await.unwrap();
            ordinary.check(&reader, &audit, &budget).await.unwrap();
            assert_eq!(audit.context_observations.load(std::sync::atomic::Ordering::Relaxed), first,
                "ordinary operation scopes retain local checks without repeating protected context reads");
            let lifetime = audit.checked_context_lifetime(&reader, &budget).unwrap();
            let foreign_budget = ResourceBudget::fixed(64 << 20).unwrap();
            assert!(lifetime.check(&reader, &foreign_budget).is_err());
            let foreign_reader = NativeReader::private(std::sync::Arc::new(surrealdb::Surreal::init())).with_budget(&budget);
            assert!(lifetime.check(&foreign_reader, &budget).is_err());
            foreign_reader.close().await.unwrap();
            let own_pin = RecordId::new("native_pin", audit.pin.identity.hex());
            let mut live = client.query("SELECT VALUE released FROM ONLY $pin").bind(("pin", own_pin.clone())).await.unwrap().check().unwrap();
            assert_eq!(live.take::<Value>(0).unwrap(), Value::Bool(false), "this audit owns an actual live native pin");
            let guard = format!("BEGIN; IF {} {{ THROW 'native installation live pins remain'; }}; CANCEL;", crate::control::live_pin_drain_predicate_for_control());
            let blocked = client.query(guard).await.unwrap().check().unwrap_err();
            assert!(blocked.to_string().contains("native installation live pins remain"), "production live-pin clause itself must exclude the checked lifetime, independently of attempts: {blocked}");
            audit.pin.ensure_live().unwrap();
            if !drift {
                audit.close().await.unwrap();
                let mut released = client.query("SELECT VALUE released FROM ONLY $pin").bind(("pin", own_pin)).await.unwrap().check().unwrap();
                assert_eq!(released.take::<Value>(0).unwrap(), Value::Bool(true), "this audit's exact pin is released regardless of other parallel pins");
                assert!(lifetime.check(&reader, &budget).is_err(), "released pin cannot retain checked context");
                assert!(ordinary.check(&reader, &audit, &budget).await.is_err());
                return;
            }
            // Earn a genuine completed parent first; stale local context is never
            // substituted for a receiving parent or incarnation capability.
            let boundary = crate::completed_state::complete_retained_content_from_source(
                client.clone(), store.attempt(), &[], &[], &budget, &source,
            ).await.unwrap();
            boundary_owner = Some(boundary);
            let boundary = boundary_owner.as_ref().unwrap();
            let completion = boundary.completion().clone();
            let expected = crate::control::inspect_expected_read(&reader, completion.record().unwrap(), &budget)
                .await.unwrap().unwrap();
            let live = crate::control::live_view::capture(&reader, store.attempt(), &budget).await.unwrap();
            let installer = crate::RuntimeConfig::read(std::path::Path::new(
                &std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("exact owned campaign installer"),
            )).unwrap();
            assert_eq!(installer.endpoint, config.endpoint);
            assert_eq!(installer.namespace, config.namespace);
            assert_eq!(installer.database, config.database);
            assert_eq!(installer.service_generation, config.service_generation);
            assert_eq!(installer.authentication, crate::AuthenticationScope::Root);
            let (connected, lease) = crate::reader::connect_owned(&installer.endpoint, &installer.writer_credentials(),
                store.namespace().as_str(), store.database().as_str(), &observer_factory).await.unwrap();
            admin = Some(connected);
            admin_lease = Some(lease);
            let admin = admin.as_ref().unwrap();
            let old = audit.context.1;
            let new = old.checked_add(1).unwrap();
            // Deliberate test tamper and exact restoration are restricted to this
            // disposable exclusive target; this is not a production epoch rollback.
            restore_epoch = Some(old);
            let mut variables = Variables::new(); variables.insert("epoch", new);
            admin.query("UPDATE native_installation:current SET validity_epoch=$epoch RETURN NONE")
                .bind(variables).await.unwrap().check().unwrap();
            let before_reuse = audit.context_observations.load(std::sync::atomic::Ordering::Relaxed);
            scope.check(&reader, &audit, &budget).await.unwrap();
            assert_eq!(audit.context_observations.load(std::sync::atomic::Ordering::Relaxed), before_reuse,
                "cached phase context defers remote drift detection to its receiving boundary");
            scope.check_receiving_source(&reader, &audit, &budget).unwrap();
            let error = match crate::control::held::hold_live_boundary_expected(
                &client, &live, &completion, expected, &budget,
            ).await {
                Ok(_) => panic!("late validity drift must not return independently held boundary authority"),
                Err(error) => error,
            };
            assert!(error.to_string().contains("live installed view owner changed"),
                "final live acquisition must compare the captured context atomically: {error:?}");
            ordinary.check(&reader, &audit, &budget).await.unwrap();
            assert_eq!(audit.context_observations.load(std::sync::atomic::Ordering::Relaxed), before_reuse,
                "raw privileged tampering is outside controlled-write source reuse; final receiving CAS still refuses it");
        }).catch_unwind()).await;
        let mut cleanup = d::completion::Completion::default();
        if let (Some(admin), Some(epoch)) = (admin.as_ref(), restore_epoch) {
            let mut variables = Variables::new();
            variables.insert("epoch", epoch);
            cleanup.step("context drift exact test epoch restoration", async {
                admin.query("UPDATE native_installation:current SET validity_epoch=$epoch RETURN NONE")
                    .bind(variables).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                Ok(())
            }.await);
        }
        if let Some(boundary) = boundary_owner {
            cleanup.step(
                "context drift retained boundary close",
                boundary.close().await,
            );
        }
        cleanup.step("boundary context reader close", reader.close().await);
        cleanup.step("boundary context audit close", audit.close().await);
        cleanup.step("boundary context attempt cleanup", store.abandon().await);
        let observer_completion = observer_lease.complete_with(&cleanup).await;
        cleanup.step(
            "native audit observer owner completion",
            observer_completion,
        );
        drop(observer_lease);
        if let Some(lease) = admin_lease {
            let completion = lease.complete_with(&cleanup).await;
            cleanup.step("context drift installer owner completion", completion);
            drop(lease);
        }
        cleanup.step(
            "native audit observer factory drain",
            observer_factory
                .drain(&crate::control::RetryCancellation::default())
                .await,
        );
        let cleaned = d::completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("boundary context finalization after assertion panic: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn boundary_context_reuse_requires_checked_live_pin_and_refuses_closing_source() {
        boundary_context_control(false).await;
    }
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "requires exclusive coordinator-owned disposable maintenance campaign; mutates actual validity epoch"]
    async fn boundary_final_live_hold_refuses_epoch_drift_after_local_context_reuse() {
        boundary_context_control(true).await;
    }
    #[tokio::test]
    async fn protected_source_scope_reuses_operation_holds_and_refuses_unobserved_incidence() {
        let audit = shared_preparation_controls::test_audit(1 << 20);
        let budget = audit.budget.clone();
        let reader = NativeReader::private(audit.client.clone()).with_budget(&budget);
        // This pure control tests routing under modeled pinned endpoints. The client is
        // disconnected, so a missing incidence cannot acquire fabricated native success.
        let source = ProtectedInspectionSource { audit: &audit };
        let mut scope = source.inspection_scope(&reader, &budget).unwrap();
        let owner = RecordId::new("compiler_view", "one");
        let object = RecordId::new("compiler_contribution", "two");
        scope.remember(&owner).unwrap();
        scope.remember(&object).unwrap();
        audit
            .inspected_holds
            .insert((owner.clone(), object.clone()), ());
        assert!(scope.source && scope.operation);
        scope
            .require_hold(&reader, &owner, &object, &audit, &budget)
            .await
            .unwrap();

        let mut fresh = ClosureInspectionScope::new(&audit, &budget).unwrap();
        fresh.remember(&owner).unwrap();
        fresh.remember(&object).unwrap();
        assert!(
            fresh
                .require_hold(&reader, &owner, &object, &audit, &budget)
                .await
                .is_err()
        );
        assert!(
            fresh.holds.is_empty(),
            "fresh construction cannot borrow source discovery"
        );

        let missing = RecordId::new("compiler_contribution", "missing");
        scope.remember(&missing).unwrap();
        let before = audit.work.next.load(std::sync::atomic::Ordering::Relaxed);
        assert!(
            scope
                .require_hold(&reader, &owner, &missing, &audit, &budget)
                .await
                .is_err()
        );
        assert!(
            audit.work.next.load(std::sync::atomic::Ordering::Relaxed) > before,
            "first miss must reach the actual inspection worker"
        );
        assert!(!audit.inspected_holds.contains_key(&(owner, missing)));
        audit.drain_workers().await.unwrap();
        reader.close().await.unwrap();
        audit.retain_unknown();
        audit
            .closed
            .store(true, std::sync::atomic::Ordering::Release);
    }
    #[tokio::test]
    async fn protected_source_scope_refuses_foreign_session_pool_pin_context_and_closed_source() {
        let audit = shared_preparation_controls::test_audit(1 << 20);
        let budget = audit.budget.clone();
        let reader = NativeReader::private(audit.client.clone()).with_budget(&budget);
        let source = ProtectedInspectionSource { audit: &audit };
        let scope = source.inspection_scope(&reader, &budget).unwrap();
        let foreign_client = Arc::new(surrealdb::Surreal::init());
        let foreign_reader = NativeReader::private(foreign_client.clone()).with_budget(&budget);
        assert!(source.inspection_scope(&foreign_reader, &budget).is_err());
        assert!(
            audit
                .protected_source(&foreign_client, &budget)
                .await
                .is_err()
        );
        let foreign_budget = ResourceBudget::fixed(1 << 20).unwrap();
        assert!(source.inspection_scope(&reader, &foreign_budget).is_err());
        assert!(
            audit
                .protected_source(&audit.client, &foreign_budget)
                .await
                .is_err()
        );
        let mut changed = audit.clone();
        changed.context.1 += 1;
        assert!(scope.check_local(&reader, &changed, &budget).is_err());
        let mut foreign_pin = audit.clone();
        foreign_pin.pin = crate::control::ReaderPin::test_detached(false);
        assert!(scope.check_local(&reader, &foreign_pin, &budget).is_err());
        foreign_pin.retain_unknown();
        audit
            .work
            .accepting
            .store(false, std::sync::atomic::Ordering::Release);
        assert!(scope.check_local(&reader, &audit, &budget).is_err());
        assert!(
            audit
                .protected_source(&audit.client, &budget)
                .await
                .is_err()
        );
        audit
            .work
            .accepting
            .store(true, std::sync::atomic::Ordering::Release);
        audit
            .closed
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(source.inspection_scope(&reader, &budget).is_err());
        audit.retain_unknown();
        assert!(source.inspection_scope(&reader, &budget).is_err());
        reader.close().await.unwrap();
        foreign_reader.close().await.unwrap();
    }
    #[test]
    fn many_selector_maps_share_one_charged_full_descriptor_and_keep_exact_logical_values() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let data = descriptor();
        let identity = data.identity().unwrap();
        let descriptor = shared_descriptor(data.clone(), &budget).unwrap();
        let retained = budget.reserved();
        let selectors = (0..256)
            .map(|_| SharedContributions(BTreeMap::from([(identity, descriptor.clone())])))
            .collect::<Vec<_>>();
        assert_eq!(
            budget.reserved(),
            retained,
            "selector handles must not reserve another full descriptor"
        );
        for selected in &selectors {
            assert!(Arc::ptr_eq(&selected.0[&identity], &descriptor));
            assert_eq!(selected[&identity], data);
            let values = selected.into_iter().collect::<Vec<_>>();
            let [(selected_id, selected_data)] = values.as_slice() else {
                panic!("single exact selected contributor")
            };
            assert_eq!(**selected_id, identity);
            assert_eq!(**selected_data, data);
        }
        drop(descriptor);
        assert_eq!(budget.reserved(), retained);
        drop(selectors);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn independent_boundary_scope_starts_fresh_and_refuses_another_real_pin_context() {
        let audit = shared_preparation_controls::test_audit(1 << 20);
        let budget = audit.budget.clone();
        let mut operation = ClosureInspectionScope::for_operation(&audit, &budget).unwrap();
        let boundary = ClosureInspectionScope::new(&audit, &budget).unwrap();
        operation.holds.insert((
            RecordId::new("compiler_view", "one"),
            RecordId::new("compiler_contribution", "two"),
        ));
        assert!(operation.operation);
        assert!(!boundary.operation);
        assert!(
            boundary.observations.is_empty()
                && boundary.descriptors.is_empty()
                && boundary.views.is_empty()
                && boundary.holds.is_empty()
        );
        let reader = NativeReader::private(audit.client.clone()).with_budget(&budget);
        boundary.check_local(&reader, &audit, &budget).unwrap();
        let foreign = shared_preparation_controls::test_audit(1 << 20);
        assert!(boundary.check_local(&reader, &foreign, &budget).is_err());
        audit.retain_unknown();
        foreign.retain_unknown();
    }
}
