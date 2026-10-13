//! Independent exact selected mapping and dependency inspection for native installation.
//! This prepares a writer's audit; it does not grant semantic admission or completion.
use super::ContributorRealization;
use crate::{
    NativeReader,
    completed_rows::{canonical_backing_endpoints, validate_state_row},
    control::ExpectedRecord,
};
use futures::FutureExt;
use lctx_model::domain::{
    self as d, ContentHash, ModelError,
    charged::StateCharge,
    completed::{CompletedContribution, CompletedView},
    recovery_closure::{RecoveryClosure, RecoveryTraversal},
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
}
type PreparationWorker = (u64, tokio::task::JoinHandle<()>, std::sync::Arc<Box<dyn d::resources::Reservation>>);
#[derive(Default)]
struct WorkerDrain {
    workers: Vec<PreparationWorker>,
    terminal: d::completion::Completion,
}
#[derive(Clone)]
pub struct InstallationAudit {
    client: std::sync::Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    pin: std::sync::Arc<crate::control::ReaderPin>,
    contributors: SharedMap<RecordId, ExpectedRecord>,
    inspected_contributors: SharedMap<ContentHash, ObservedContributor>,
    inspected_views: SharedMap<ContentHash, ObservedView>,
    inspected_holds: SharedMap<(RecordId, RecordId), ()>,
    logical_contributors: SharedMap<RecordId, LogicalContributorRows>,
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
    blobs: SharedMap<RecordId, i64>,
    budget: ResourceBudget,
    charge: SharedCharge,
    blob_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    contribution_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    logical_contribution_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    alias_observations: std::sync::Arc<std::sync::atomic::AtomicU64>,
    closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    context: (ContentHash, i64),
    work: std::sync::Arc<WorkOwner>,
    worker: bool,
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
            logical_contributors: SharedMap::new(),
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
            closed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            work: std::sync::Arc::new(WorkOwner {
                flights: std::sync::Mutex::new(BTreeMap::new()),
                workers: std::sync::Mutex::new(Vec::new()),
                draining: tokio::sync::Mutex::new(WorkerDrain::default()),
                next: std::sync::atomic::AtomicU64::new(1),
                accepting: std::sync::atomic::AtomicBool::new(true),
                close_gate: tokio::sync::Mutex::new(()),
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
        Ok(())
    }
    pub(crate) async fn ensure_context(
        &self,
        reader: &NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check(reader, budget)?;
        let _charge = budget.reserve("installation audit context check", 4096)?;
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
        self.pin.ensure_live()
    }
    async fn run_work(
        &self,
        key: (u8, ContentHash),
        build: impl FnOnce(Self) -> futures::future::BoxFuture<'static, Result<(), ModelError>>
        + Send
        + 'static,
    ) -> Result<(), ModelError> {
        self.run_work_many(vec![key], move |owner, _| build(owner)).await
    }
    async fn run_work_many(
        &self,
        keys: Vec<(u8, ContentHash)>,
        build: impl FnOnce(Self, Vec<(u8, ContentHash)>) -> futures::future::BoxFuture<'static, Result<(), ModelError>>
        + Send + 'static,
    ) -> Result<(), ModelError> {
        self.pin.ensure_live()?;
        // Cancelled waiters leave their independently owned workers behind. Reap only
        // already terminal handles before admitting another bounded preparation.
        self.reap_terminal_workers(Ok(()), &BTreeSet::new()).await?;
        if keys.len() > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Limit { owner: "installation preparation", limit: "window keys",
                observed: keys.len(), bound: crate::loader::NATIVE_WINDOW_ROWS });
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
                    "installation preparation flight metadata", 4096usize.saturating_mul(missing.len()))?);
                {
                    let mut workers = self.work.workers.lock().unwrap();
                    if workers.len() == workers.capacity() {
                        let capacity = workers.capacity().saturating_mul(2).max(1);
                        self.charge.grow(capacity.saturating_sub(workers.capacity())
                            .saturating_mul(std::mem::size_of::<PreparationWorker>()))?;
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
                for key in &missing { registry.insert(
                    *key,
                    WorkFlight {
                        token,
                        future: future.clone(),
                        _charge: charge.clone(),
                    },
                ); }
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
                let worker_charge = charge.clone();
                let task = tokio::spawn(async move {
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
                });
                self.work.workers.lock().unwrap().push((token, task, worker_charge));
            }
            futures
        };
        if futures.len() == 1 {
            let (token, future) = futures.into_iter().next().unwrap();
            let result = future.await.map_err(ModelError::SharedCause);
            let failed = if result.is_err() { BTreeSet::from([token]) } else { BTreeSet::new() };
            return self.reap_terminal_workers(result, &failed).await;
        }
        let mut terminal = d::completion::Completion::default();
        let mut failed = BTreeSet::new();
        for (token, future) in futures {
            let result = future.await.map_err(ModelError::SharedCause);
            if result.is_err() { failed.insert(token); }
            terminal.step("installation preparation set flight", result);
        }
        self.reap_terminal_workers(d::completion::complete(Ok(()), terminal), &failed).await
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
                if current != Some(worker.id()) && (worker.is_finished() || failed.contains(token)) {
                    completed.push(workers.swap_remove(index));
                } else { index += 1; }
            }
            completed
        };
        let mut terminal = d::completion::Completion::default();
        for (_, worker, _charge) in completed {
            if let Err(error) = worker.await {
                self.retain_unknown();
                terminal.remote = d::completion::RemoteState::Unknown;
                terminal.step("installation terminal worker join", Err(ModelError::infrastructure(
                    d::Infrastructure::Unconfirmed, format!("installation preparation worker did not reach terminal: {error}"))));
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
                std::mem::swap(&mut pending.workers, &mut *self.work.workers.lock().unwrap());
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
        d::completion::complete(Ok(()), std::mem::take(&mut pending.terminal))
    }
    pub async fn close(&self) -> Result<(), ModelError> {
        let _close = self.work.close_gate.lock().await;
        self.drain_workers().await?;
        self.pin.release().await?;
        self.closed
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
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
    memberships: crate::ordered_rows::PreparedRows,
    backing: crate::ordered_rows::PreparedRows,
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
    if actual.incarnation() != input.incarnation || actual.row() != &input.row {
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
    sink.part(b"owner", &serde_json::to_vec(owner).map_err(ModelError::codec)?);
    sink.part(b"object", &serde_json::to_vec(object).map_err(ModelError::codec)?);
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
    protected: std::collections::BTreeSet<RecordId>,
    operation: bool,
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
            protected: std::collections::BTreeSet::new(),
            operation: false,
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
        audit.check(reader, budget)?;
        self.pin.ensure_live()
    }
    async fn check(
        &mut self,
        reader: &NativeReader<()>,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check_local(reader, audit, budget)?;
        // A fresh phase retains exact-CAS-pinned endpoints. Its boundary final CAS
        // or terminal-acquisition check independently fences full installation
        // context; reborrowing these phase-local premises needs no new read.
        if self.operation || !self.context_checked {
            audit.ensure_context(reader, budget).await?;
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
        let mut requests = BTreeMap::new();
        // Declared inputs include empty dependency views with no mapping entries.
        let identities = views
            .map(|(root, _)| root.identity)
            .chain(mappings.clone().map(|mapping| mapping.view))
            .chain(mappings.clone().flat_map(|mapping| {
                self.observations[&mapping.physical]
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
            if !requests.contains_key(&id) {
                request_charge.grow(
                    crate::selection::node_bytes(&id)
                        .saturating_add(std::mem::size_of::<FreshRequest>())
                        .saturating_add(128),
                )?;
                requests.insert(id, FreshRequest::View(identity));
            }
        }
        self.observe_fresh_requests(reader, requests, audit, budget)
            .await?;
        let mut pairs = std::collections::BTreeSet::new();
        let mut pair_charge =
            StateCharge::new(budget, "fresh boundary exact incidence request identities");
        for mapping in mappings {
            let owner = RecordId::new("compiler_view", mapping.view.hex());
            let physical = RecordId::new("compiler_contribution", mapping.physical.hex());
            let inputs = &self.observations[&mapping.physical]
                .descriptor
                .data
                .spec
                .inputs;
            for pair in
                std::iter::once((owner, physical.clone())).chain(inputs.iter().map(|input| {
                    (
                        physical.clone(),
                        RecordId::new("compiler_view", input.view().hex()),
                    )
                }))
            {
                if !self.holds.contains(&pair) && !pairs.contains(&pair) {
                    pair_charge.grow(
                        crate::selection::node_bytes(&pair.0)
                            .saturating_add(crate::selection::node_bytes(&pair.1))
                            .saturating_add(256),
                    )?;
                    pairs.insert(pair);
                }
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
                    self.observe_operation_holds(reader, window, audit, budget).await
                } else {
                    let mut actual = crate::control::inspect_hold_window_read(reader, window, budget).await?;
                    for (owner, object) in window { actual.take_required(owner, object)?; }
                    actual.finish()
                }
            }.await;
            match captured {
                Err(error) if count > 1 && terminal_local_resource(&error) => { width = (count / 2).max(1); continue; }
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
        self.check_local(reader, audit, budget)
    }
    async fn observe_fresh_requests(
        &mut self,
        reader: &NativeReader<()>,
        requests: BTreeMap<RecordId, FreshRequest>,
        audit: &InstallationAudit,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if self.operation {
            return self.observe_operation_requests(reader, requests, audit, budget).await;
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
                                    if previous.data != value.descriptor.data {
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
                    crate::control::live_view::transfer_expected_many(&self.pin, &expected, budget)
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
        let _handles = budget.reserve("operation observation request handles",
            requests.len().saturating_mul(std::mem::size_of::<(RecordId, FreshRequest)>()))?;
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
                    width = (count / 2).max(1); continue;
                }
                other => other?,
            }
            self.check_local(reader, audit, budget)?;
            for (_, request) in page {
                self.charge.grow(1024)?;
                match request {
                    FreshRequest::Contributor(mapping) => {
                        let value = audit.inspected_contributors.get(&mapping.physical)
                            .ok_or(ModelError::Conflict("operation inspected contributor absent"))?;
                        if value.logical != mapping.logical || value.expected.incarnation() != mapping.incarnation {
                            return Err(ModelError::Conflict("operation selected physical mapping contradiction"));
                        }
                        let value = if let Some(previous) = self.descriptors.get(&mapping.logical) {
                            if previous.data != value.descriptor.data {
                                return Err(ModelError::Conflict("installed equivalent copies differ in full descriptor"));
                            }
                            std::sync::Arc::new(ObservedContributor { expected: value.expected.clone(), descriptor: previous.clone(), logical: value.logical })
                        } else { value };
                        self.remember(value.expected.record())?;
                        self.descriptors.insert(mapping.logical, value.descriptor.clone());
                        self.observations.insert(mapping.physical, value);
                    }
                    FreshRequest::View(identity) => {
                        let value = audit.inspected_views.get(identity)
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
        let input = budget.reserve("operation incidence worker inputs", pairs.len().saturating_mul(4096))?;
        let requested = pairs.iter().map(|(owner, object)| Ok(((8, inspected_hold_key(owner, object)?), (owner.clone(), object.clone()))))
            .collect::<Result<BTreeMap<_, _>, ModelError>>()?;
        let keys = requested.keys().copied().collect::<Vec<_>>();
        let worker_budget = budget.clone();
        audit.run_work_many(keys, move |owner, claimed| async move {
            let _input = input;
            let pairs = claimed.into_iter().map(|key| requested[&key].clone())
                .filter(|pair| !owner.inspected_holds.contains_key(pair)).collect::<Vec<_>>();
            let reader = NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
            let result = async {
                let mut actual = crate::control::inspect_hold_window_read(&reader, &pairs, &worker_budget).await?;
                for (holder, object) in &pairs { actual.take_required(holder, object)?; }
                actual.finish()?;
                owner.ensure_context(&reader, &worker_budget).await
            }.await;
            let mut terminal = d::completion::Completion::default();
            terminal.step("operation incidence window reader close", reader.close().await);
            d::completion::complete(result, terminal)?;
            owner.pin.ensure_live()?;
            for pair in pairs {
                owner.charge.grow(crate::selection::node_bytes(&pair.0).saturating_add(crate::selection::node_bytes(&pair.1)).saturating_add(128))?;
                owner.inspected_holds.insert(pair, ());
            }
            Ok(())
        }.boxed()).await?;
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
    views: BTreeMap<ContentHash, CompletedView>,
    contributions: SharedContributions,
}
impl InspectedLogicalClosure {
    pub(crate) fn views(&self) -> &BTreeMap<ContentHash, CompletedView> {
        &self.views
    }
    pub(crate) fn contributions(&self) -> &SharedContributions {
        &self.contributions
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
            if previous.data != observed.descriptor.data {
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
pub(crate) fn contribution_heap(value: &CompletedContribution) -> usize {
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
                .iter()
                .map(|(name, _)| {
                    std::mem::size_of::<(String, d::completed::OutputContent)>()
                        + 32
                        + name.capacity()
                })
                .fold(0, usize::saturating_add),
        )
}
pub(crate) fn descriptor<T: serde::de::DeserializeOwned>(row: &Value) -> Result<T, ModelError> {
    let Some(Value::Bytes(bytes)) = row.as_object().and_then(|row| row.get("descriptor")) else {
        return Err(ModelError::Schema("installed audit descriptor"));
    };
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}
fn exact_mappings(
    closure: &RecoveryClosure,
    mappings: &[ContributorRealization],
) -> Result<(), ModelError> {
    if mappings
        .windows(2)
        .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
        || mappings.iter().any(|mapping| mapping.incarnation <= 0)
        || !closure
            .views()
            .values()
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
    fn view_heap(view: &CompletedView) -> usize {
        view.relation.capacity().saturating_add(
            view.contributions
                .len()
                .saturating_mul(std::mem::size_of::<ContentHash>() + 32),
        )
    }
    fn pending_capacity(count: usize) -> Result<usize, ModelError> {
        count
            .max(4)
            .checked_next_power_of_two()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<CompletedView>()))
            .ok_or_else(|| {
                ModelError::Invalid("installed traversal pending capacity overflow".into())
            })
    }
    // Validation constructs an independent CompletedView, including its set clone.
    let mut validation_bytes = view_heap(root);
    let mut validation = budget.reserve(
        "installed closure view validation scratch",
        validation_bytes,
    )?;
    root.validate()?;
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
    let mut identity_bytes = 0usize;
    for mapping in mappings {
        let observed = match scope.as_mut() {
            Some((scope, audit)) => {
                inspect_selected_contributor(reader, mapping, budget, Some((&mut **scope, *audit)))
                    .await?
            }
            None => inspect_selected_contributor(reader, mapping, budget, None).await?,
        };
        identity_bytes = identity_bytes.max(
            crate::loader::native_bytes(observed.expected.row())
                .saturating_mul(32)
                .saturating_add(65536),
        );
        if let Some(previous) = descriptors.get(&mapping.logical) {
            if previous.data != observed.descriptor.data {
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
        if let Some(previous) = contributors.insert(mapping.physical, observed.expected.clone()) {
            if previous.incarnation() != observed.expected.incarnation()
                || previous.row() != observed.expected.row()
            {
                return Err(ModelError::Conflict(
                    "installed selected physical observation contradiction",
                ));
            }
        }
    }
    let mut traversal_charge = StateCharge::new(
        budget,
        "installed traversal temporary full logical descriptors",
    );
    // RecoveryTraversal retains unique logical descriptors, but its private pending
    // Vec may contain repeated dependencies and keeps capacity after popping. Admit
    // a conservative single-traversal bound for every dependency queued by each new
    // contributor, including the root and power-of-two capacity highwater. This
    // working bound is released at traversal completion, not retained in its result.
    // include_view revalidates contribution identity after decode scratch ends:
    // output-key sets, input identity vectors and captured-binding validation are
    // additional allocations. Preserve the existing descriptor decode/validation
    // bound for the largest selected contributor, independently of retained copies.
    let _identity = budget.reserve(
        "installed closure contributor validation scratch",
        identity_bytes,
    )?;
    let mut working = StateCharge::new(budget, "installed closure traversal working state");
    let mut queued = 1usize;
    let mut pending_bytes = pending_capacity(queued)?;
    working.grow(pending_bytes.saturating_add(view_heap(root)))?;
    let mut scheduled = std::collections::BTreeSet::new();
    let mut traversal = RecoveryTraversal::for_views(vec![root.clone()])?;
    while let Some(requested) = traversal.next_view()? {
        let observed = match scope.as_mut() {
            Some((scope, audit)) => {
                scope
                    .view(reader, requested.identity, *audit, budget)
                    .await?
            }
            None => inspect_view_fresh(reader, requested.identity, budget).await?,
        };
        let actual = observed.data.clone();
        let actual_validation = view_heap(&actual);
        if actual_validation > validation_bytes {
            validation.try_resize(actual_validation)?;
            validation_bytes = actual_validation;
        }
        let mut local = StateCharge::new(budget, "installed closure local selected descriptors");
        local.grow(requested.contributions.len().saturating_mul(
            std::mem::size_of::<CompletedContribution>() + std::mem::size_of::<ContentHash>() + 32,
        ))?;
        let mut selected = Vec::with_capacity(requested.contributions.len());
        let mut dependencies = BTreeMap::new();
        for logical in &requested.contributions {
            let contribution = descriptors
                .get(logical)
                .ok_or(ModelError::Conflict("installed missing dependency mapping"))?;
            let contribution = &contribution.data;
            local.grow(contribution_heap(contribution))?;
            selected.push(contribution.clone());
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
                        .require_hold(reader, &view_record, &contributor_record, *audit, budget)
                        .await?
                }
                None => {
                    crate::completed_state::require_hold(reader, &view_record, &contributor_record)
                        .await?
                }
            }
            for input in &contribution.spec.inputs {
                if !dependencies.contains_key(&input.view()) {
                    let observed = match scope.as_mut() {
                        Some((scope, audit)) => {
                            scope.view(reader, input.view(), *audit, budget).await?
                        }
                        None => inspect_view_fresh(reader, input.view(), budget).await?,
                    };
                    local.grow(
                        std::mem::size_of::<(ContentHash, CompletedView)>()
                            .saturating_add(32)
                            .saturating_add(view_heap(&observed.data)),
                    )?;
                    dependencies.insert(input.view(), observed.data.clone());
                }
                let dependency_record = RecordId::new("compiler_view", input.view().hex());
                match scope.as_mut() {
                    Some((scope, audit)) => {
                        scope
                            .require_hold(
                                reader,
                                &contributor_record,
                                &dependency_record,
                                *audit,
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
            if !scheduled.contains(logical) {
                working.grow(std::mem::size_of::<ContentHash>() + 32)?;
                // include_view queues inputs only for a contributor first entering
                // its logical closure; repeated input roles still own separate clones.
                for input in &contribution.spec.inputs {
                    let dependency = dependencies
                        .get(&input.view())
                        .ok_or(ModelError::Conflict("installed undeclared dependency view"))?;
                    let dependency_validation = view_heap(dependency);
                    if dependency_validation > validation_bytes {
                        validation.try_resize(dependency_validation)?;
                        validation_bytes = dependency_validation;
                    }
                    queued = queued.checked_add(1).ok_or_else(|| {
                        ModelError::Invalid("installed traversal pending count overflow".into())
                    })?;
                    let capacity = pending_capacity(queued)?;
                    working.grow(
                        capacity
                            .saturating_sub(pending_bytes)
                            .saturating_add(view_heap(dependency)),
                    )?;
                    pending_bytes = capacity;
                }
                traversal_charge.grow(
                    std::mem::size_of::<(ContentHash, CompletedContribution)>()
                        + 32
                        + contribution_heap(contribution),
                )?;
                scheduled.insert(*logical);
            }
        }
        charge
            .grow(std::mem::size_of::<(ContentHash, CompletedView)>() + 32 + view_heap(&actual))?;
        traversal.include_view(&requested, actual, selected, |identity| {
            dependencies
                .get(&identity)
                .cloned()
                .ok_or(ModelError::Conflict("installed undeclared dependency view"))
        })?;
    }
    let logical = traversal.finish()?;
    let _mapping_check = budget.reserve(
        "installed exact mapping correspondence",
        mappings
            .len()
            .saturating_mul(std::mem::size_of::<(ContentHash, (ContentHash, i64))>() + 32),
    )?;
    exact_mappings(&logical, mappings)?;
    charge.grow(
        logical.contributions().len().saturating_mul(
            std::mem::size_of::<(ContentHash, std::sync::Arc<ContributionDescriptor>)>()
                .saturating_add(32),
        ),
    )?;
    let shared = SharedContributions(
        logical
            .contributions()
            .keys()
            .map(|id| {
                Ok((
                    *id,
                    descriptors
                        .get(id)
                        .ok_or(ModelError::Conflict(
                            "inspected logical descriptor handle missing",
                        ))?
                        .clone(),
                ))
            })
            .collect::<Result<_, ModelError>>()?,
    );
    let shared_logical = InspectedLogicalClosure {
        views: logical.views().clone(),
        contributions: shared,
    };
    if let Some((scope, audit)) = scope.as_mut() {
        scope.check(reader, *audit, budget).await?;
    }
    drop(logical);
    drop(traversal_charge);
    Ok(InspectedClosure {
        logical: shared_logical,
        contributors,
        descriptors_by_physical,
        mappings: mappings.to_vec(),
        _charge: charge,
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
) -> Result<crate::ordered_rows::PreparedRows, ModelError> {
    scope.check(reader, audit, budget).await?;
    audit_memberships_in_context(reader, closure, budget, audit).await?;
    let aliases = audit_aliases_in_context(reader, closure, budget, audit).await?;
    prepare_closure_in_context(reader, closure, budget, audit).await?;
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
    audit.check(reader, budget)?;
    for expected in closure.contributors.values() {
        if let Some(cached) = audit.contributors.get(expected.record()) {
            if cached.incarnation() != expected.incarnation() || cached.row() != expected.row() {
                return Err(ModelError::Conflict(
                    "reused installed contributor premise changed",
                ));
            }
            audit.check(reader, budget)?;
            continue;
        }
        let protected = expected.clone();
        let (key, input) = exact_work(expected, budget)?;
        let budget = budget.clone();
        audit
            .run_work((0, key), move |owner| {
                async move {
                    let reader = NativeReader::private(owner.client.clone()).with_budget(&budget);
                    let result = async {
                        let expected = worker_expected(&reader, &input, &budget).await?;
                        let descriptor = descriptor::<CompletedContribution>(expected.row())?;
                        let fresh =
                            audit_one_contributor(&reader, &descriptor, &expected, &budget, &owner)
                                .await?;
                        Ok(fresh.then_some(protected))
                    }
                    .await;
                    let mut terminal = d::completion::Completion::default();
                    terminal.step(
                        "complete contributor worker reader close",
                        reader.close().await,
                    );
                    let prepared = d::completion::complete(result, terminal)?;
                    owner.pin.ensure_live()?;
                    if let Some(prepared) = prepared {
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
            .await?;
    }
    Ok(())
}
async fn audit_one_contributor(
    reader: &NativeReader<()>,
    descriptor: &CompletedContribution,
    expected: &ExpectedRecord,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<bool, ModelError> {
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
        return Ok(false);
    }
    audit.pin.protect(owner.clone()).await?;
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
            validate_state_row("compiler_membership", row)?;
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
            window.push(row.clone());
            if window.len() >= crate::loader::NATIVE_WINDOW_ROWS
                || charge.reserved() >= d::resources::TRANSFER_BYTES
            {
                audit_membership_window(reader, &owner, &window, budget, audit).await?;
                window.clear();
                charge = StateCharge::new(budget, "installed membership window");
            }
        }
        if !window.is_empty() {
            audit_membership_window(reader, &owner, &window, budget, audit).await?;
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
    Ok(true)
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
        if let Some(previous) = metadata.insert(node.clone(), object) {
            if ["relation", "semantic_key", "content"]
                .iter()
                .any(|field| previous.get(*field) != object.get(*field))
            {
                return Err(ModelError::Conflict(
                    "installed membership ambiguous backing",
                ));
            }
        }
    }
    let ids = metadata
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
        let mut endpoints = BTreeMap::new();
        let mut endpoint_charge = StateCharge::new(budget, "installed endpoint set preparation");
        while let Some(row) = stream.next().await? {
            let backing = row
                .as_object()
                .ok_or(ModelError::Schema("installed set backing object"))?;
            let Some(Value::RecordId(node)) = backing.get("id") else {
                return Err(ModelError::Schema("installed set backing identity"));
            };
            let member = metadata
                .get(node)
                .ok_or(ModelError::Conflict("installed foreign backing"))?;
            if !seen.insert(node.clone())
                || backing.get("semantic_type") != member.get("relation")
                || backing.get("semantic_key") != member.get("semantic_key")
                || backing.get("content") != member.get("content")
            {
                return Err(ModelError::Conflict(
                    "installed membership backing metadata/duplicate",
                ));
            }
            let (expected, original, _decode) = canonical_backing_endpoints(&row, budget)?;
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
                    endpoint_charge =
                        StateCharge::new(budget, "installed endpoint set preparation");
                }
                if let Some(previous) = endpoints.get(id) {
                    if previous != &endpoint {
                        return Err(ModelError::Conflict(
                            "installed endpoint expected collision",
                        ));
                    }
                } else {
                    endpoint_charge.grow(bytes)?;
                    endpoints.insert(id.clone(), endpoint);
                }
            }
        }
        if seen.len() != metadata.len() {
            return Err(ModelError::Conflict("installed membership backing missing"));
        }
        audit_endpoint_window(reader, owner, &endpoints, budget).await
    }
    .await;
    let mut terminal = d::completion::Completion::default();
    terminal.step(
        "installed set backing stream drain",
        stream.drain_transport().await,
    );
    d::completion::complete(result, terminal)
}
async fn audit_original(
    reader: &NativeReader<()>,
    original: &crate::completed_rows::OriginalBacking,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<(), ModelError> {
    let source = d::graph::EntityId::of(original.artifact);
    let layout = crate::originals::layout(reader, &RecordId::new("original", source.0.hex()))
        .await?
        .ok_or(ModelError::Conflict("installed original layout"))?;
    crate::originals::validate_layout(reader, &layout).await?;
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
    let mut hash = d::ContentHasher::default();
    let mut offset = 0;
    while offset < original.len {
        let length = (original.len - offset).min(256 << 10);
        let bytes = reader
            .original_bytes(source, original.start + offset as u64, length)
            .await?;
        hash.update(&bytes);
        offset += length;
    }
    if hash.finish() != original.digest {
        return Err(ModelError::Conflict("installed original content digest"));
    }
    let blob = layout.blob.id();
    let premise = crate::originals::active_blob_premise(reader, &layout.blob).await?;
    if audit
        .blobs
        .get(&blob)
        .is_some_and(|observed| *observed != premise)
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
                        .is_some_and(|observed| *observed == premise)
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
                    crate::originals::validate_blob(&reader, &blob, &worker_budget).await?;
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
                    Ok(Some(blob.id()))
                }
                .await;
                let mut terminal = d::completion::Completion::default();
                terminal.step("complete blob worker reader close", reader.close().await);
                let prepared = d::completion::complete(result, terminal)?;
                owner.pin.ensure_live()?;
                if let Some(blob) = prepared {
                    owner.blobs.insert(blob, premise);
                    owner
                        .blob_observations
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(())
            }
            .boxed()
        })
        .await?;
    Ok(())
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
        let store = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
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
        let mut audit = InstallationAudit::new(client.clone(), &budget)
            .await
            .unwrap();
        let peer = crate::control::ReaderPin::acquire(client.clone(), &[])
            .await
            .unwrap();
        let result = std::panic::AssertUnwindSafe(async {
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
            let source = SourceArtifact::from_bytes(
                InputRevision::from_entries(vec![]).unwrap().id(),
                "peer.bin".into(),
                &bytes,
            )
            .unwrap();
            let chunk = ArtifactChunk {
                artifact: source.id(),
                ordinal: 0,
                body: EvidenceBytes(bytes.clone()),
            };
            store
                .write_batch(
                    &producer,
                    &sources,
                    &SourceArtifact::encode(std::slice::from_ref(&source)).unwrap(),
                    &budget,
                )
                .await
                .unwrap();
            store
                .write_batch(
                    &producer,
                    &chunks,
                    &ArtifactChunk::encode(&[chunk]).unwrap(),
                    &budget,
                )
                .await
                .unwrap();
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
                    audit_memberships_scoped(&reader, &closure, &budget, &mut audit)
                ),
                replay
            );
            observed.unwrap();
            assert_eq!(audit.observations(), (1, 1));
            audit_memberships_scoped(&reader, &closure, &budget, &mut audit)
                .await
                .unwrap();
            assert_eq!(
                audit.observations(),
                (1, 1),
                "equal replay neither invalidates immutable bytes nor repeats the whole audit"
            );
        })
        .catch_unwind()
        .await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("peer blob reader close", reader.close().await);
        cleanup.step("peer blob audit pin close", audit.close().await);
        cleanup.step("peer blob other pin close", peer.release().await);
        cleanup.step("peer blob attempt cleanup", store.abandon().await);
        cleanup.step(
            "peer blob observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
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
    async fn late_membership_terminal_failure_cannot_publish_reusable_observation() {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
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
        let mut audit = InstallationAudit::new(client.clone(), &budget)
            .await
            .unwrap();
        let result=std::panic::AssertUnwindSafe(async{
            let relation=Relation::of::<Package>();let mut specification=contribution("late-terminal",vec![]).spec;specification.outputs=[relation.name().into()].into();
            let producer=store.begin_contribution(specification).await.unwrap();
            let rows=(0..129).map(|index|Package{name:format!("terminal-package-{index}")}).collect::<Vec<_>>();
            store.write_batch(&producer,&relation,&Package::encode(&rows).unwrap(),&budget).await.unwrap();
            let views=store.complete_contribution(producer,d::stages::ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new(),&budget).await.unwrap();
            let view=&views[relation.name()];let held=store.installed_view(view,&budget).await.unwrap();
            let mappings:Vec<ContributorRealization>=serde_json::from_value(serde_json::to_value(&held.completion().mappings).unwrap()).unwrap();
            let closure=inspect_closure(&reader,view,&mappings,&budget).await.unwrap();
            let failed=LATE_MEMBERSHIP_FAILURE.scope(true,audit_memberships_scoped(&reader,&closure,&budget,&mut audit)).await.unwrap_err();
            assert!(format!("{failed:?}").contains("late independent membership audit control"),"{failed:?}");
            assert_eq!(audit.observations(),(0,0),"valid provisional membership never becomes a complete observation before its physical terminal");
            audit_memberships_scoped(&reader,&closure,&budget,&mut audit).await.unwrap();assert_eq!(audit.observations(),(1,0));
            let aliases = audit_aliases_scoped(&reader, &closure, &budget, &mut audit).await.unwrap();
            let cold_aliases = audit_aliases(&reader, &closure, &budget).await.unwrap();
            let cold = logical_roots(&reader, &closure, &[], &cold_aliases, &budget).await.unwrap();
            let failed = LATE_MEMBERSHIP_FAILURE.scope(true,
                logical_roots_scoped(&reader, &closure, &[], &aliases, &budget, &mut audit),
            ).await.unwrap_err();
            assert!(format!("{failed:?}").contains("late logical contributor fragment control"), "{failed:?}");
            assert_eq!(audit.logical_contribution_observations(), 0);
            assert!(audit.logical_contributors.is_empty(), "provisional normalized rows cannot publish a fragment before terminal success");
            let overlapping = vec![&closure; 17];
            assert_eq!(logical_roots_many_scoped(&reader, &overlapping, &[], &aliases, &budget, &audit).await.unwrap(), cold,
                "overlapping selectors preserve the independently reconstructed six-family identity");
            assert_eq!(audit.logical_contribution_observations(), 1);
            assert_eq!(logical_roots_scoped(&reader, &closure, &[], &aliases, &budget, &mut audit).await.unwrap(), cold);
            assert_eq!(logical_roots_scoped(&reader, &closure, &[], &aliases, &budget, &mut audit).await.unwrap(), cold);
            assert_eq!(audit.logical_contribution_observations(), 1, "one exact physical contributor is prepared once within its pin");
            let reused = audit_aliases_scoped(&reader, &closure, &budget, &mut audit).await.unwrap();
            assert_eq!(logical_roots_scoped(&reader, &closure, &[], &reused, &budget, &mut audit).await.unwrap(), cold);
            assert_eq!(audit.alias_observations(), 1);
        }).catch_unwind().await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("terminal audit reader close", reader.close().await);
        cleanup.step("terminal audit pin close", audit.close().await);
        cleanup.step("terminal audit attempt cleanup", store.abandon().await);
        cleanup.step(
            "terminal audit observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
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
        let store = NativeCompilerStore::begin(&config, d::admission::Frontier::Facts)
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
        let result=std::panic::AssertUnwindSafe(async{
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
                let mut audit=InstallationAudit::new(client.clone(),&budget).await.unwrap();audit.pin.protect(owner.clone()).await.unwrap();
                let mut variables=Variables::new();variables.insert("id",id.clone());let original:Vec<Value>=reader.query_native("SELECT * FROM $id",variables.clone()).await.unwrap();assert_eq!(original.len(),1);
                client.query("DELETE $id RETURN NONE").bind(variables.clone()).await.unwrap().check().unwrap();
                let failed=audit_memberships_scoped(&reader,&closure,&budget,&mut audit).await;
                variables.insert("row",original[0].clone());client.query("UPSERT $id CONTENT $row RETURN NONE").bind(variables).await.unwrap().check().unwrap();
                let observations=audit.observations();audit.close().await.unwrap();
                assert!(failed.is_err(),"missing {kind} cannot be replaced by matching counts or nominal identity");assert_eq!(observations,(0,0),"failed {kind} audit cannot publish reuse");
            }
        }).catch_unwind().await;
        let mut cleanup = d::completion::Completion::default();
        cleanup.step("set correspondence reader close", reader.close().await);
        cleanup.step("set correspondence attempt cleanup", store.abandon().await);
        cleanup.step(
            "set correspondence observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
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
    if let Some(previous) = audit.aliases.get(&()) {
        if previous.owners == owners {
            // inspect_closure and audit_memberships_scoped supplied current full
            // native rows for every selected physical owner. Exact set equality,
            // rather than a subset or digest, preserves whole-set reachability.
            return Ok(previous.rows.clone());
        }
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
    for expected in closure.contributors.values() {
        if let Some(previous) = audit.local_aliases.get(expected.record()) {
            audit.check(_reader, budget)?;
            run_charge
                .grow(std::mem::size_of::<crate::ordered_rows::PreparedRows>().saturating_mul(4))?;
            seed_runs.push(previous.0.clone());
            alias_runs.push(previous.1.clone());
            continue;
        }
        let (key, input) = exact_work(expected, budget)?;
        let worker_budget = budget.clone();
        audit
            .run_work((2, key), move |owner| {
                async move {
                    let reader =
                        NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                    let result = async {
                        let expected = worker_expected(&reader, &input, &worker_budget).await?;
                        prepare_local_aliases(&reader, &expected, &worker_budget, &owner, true)
                            .await
                    }
                    .await;
                    let mut terminal = Completion::default();
                    terminal.step("local alias worker reader close", reader.close().await);
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
            .await?;
        let previous = audit
            .local_aliases
            .get(expected.record())
            .ok_or(ModelError::Conflict("prepared local aliases absent"))?;
        run_charge
            .grow(std::mem::size_of::<crate::ordered_rows::PreparedRows>().saturating_mul(4))?;
        seed_runs.push(previous.0.clone());
        alias_runs.push(previous.1.clone());
    }
    let aliases =
        crate::ordered_rows::PreparedRows::union_prepared(&alias_runs, budget, || Ok(()))?;
    let empty = aliases.frame_cursor(budget)?.next_frame()?.is_none();
    audit.check(_reader, budget)?;
    // Complete empty alias inventories have no reachability obligations. Their
    // native proofs above and the outer exact-owner checks still apply, while
    // composing all membership seeds and visiting them would establish nothing.
    if empty {
        return Ok(aliases);
    }
    let seeds = crate::ordered_rows::PreparedRows::union_prepared(&seed_runs, budget, || Ok(()))?;
    audit_alias_reachability(seeds, &aliases, budget)?;
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
    let mut window = crate::loader::NATIVE_WINDOW_ROWS;
    loop {
        match prepare_local_aliases_window(reader, expected, budget, audit, include_seeds, window)
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
    use crate::ordered_rows::SortedRows;
    use d::completion::{Completion, complete};
    let owner = expected.record().clone();
    if audit.local_aliases.contains_key(&owner) {
        return Ok(None);
    }
    let mut seeds = SortedRows::with_budget(budget)?;
    let mut aliases = SortedRows::with_budget(budget)?;
    if include_seeds {
        let mut variables = Variables::new();
        variables.insert("owner", owner.clone());
        let mut members=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],
            vec!["SELECT node AS id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
        let result = async {
            while let Some(row) = members.next().await? {
                seeds.push(row)?;
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
                let Some(pointer) = holds.next().await? else { break };
                let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("id"))
                else {
                    return Err(ModelError::Schema("installed held alias pointer"));
                };
                if id.table.as_str() == "compiler_alias" && !ids.contains(id) {
                    charge.grow(crate::selection::node_bytes(id).saturating_add(256))?;
                    ids.insert(id.clone());
                }
            }
            if ids.is_empty() { break }
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
    let seeds = seeds.finish()?.into_prepared();
    let aliases = aliases.finish()?.into_prepared();
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
            self.charge.grow(crate::selection::node_bytes(owner)
                .saturating_add(crate::selection::node_bytes(object))
                .saturating_add(512))?;
            self.holds.insert((owner.clone(), object.clone()));
        }
        Ok(())
    }
    fn observe(&mut self, row: &Value, owner: &RecordId) -> Result<(), ModelError> {
        use d::KeySink;
        let object = row.as_object().ok_or(ModelError::Schema("installed alias object"))?;
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
        sink.part(b"source", &serde_json::to_vec(source).map_err(ModelError::codec)?);
        sink.part(b"target", &serde_json::to_vec(target).map_err(ModelError::codec)?);
        let mut canonical = surrealdb::types::Object::new();
        canonical.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
        canonical.insert("source", source.clone());
        canonical.insert("target", target.clone());
        if row != &Value::Object(canonical) {
            return Err(ModelError::Conflict("installed exact alias envelope"));
        }
        if !self.targets.contains(target) {
            self.charge.grow(crate::selection::node_bytes(target).saturating_add(256))?;
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
    aliases: &mut crate::ordered_rows::SortedRows,
) -> Result<(), ModelError> {
    let mut actual = crate::control::inspect_expected_window_read(reader, ids, budget).await?;
    let mut requests = AliasWindowRequests::new(budget);
    for id in ids {
        let expected = actual.take_required(id)?;
        requests.observe(expected.row(), owner)?;
        let (row, arena) = expected.into_owned_row();
        aliases.push_owned(row, arena)?;
    }
    actual.finish()?;
    // Each target and heterogeneous incidence is observed once per bounded alias page.
    // Keep the request charge through both point sets and endpoint validation.
    requests.charge.grow(requests.targets.len().saturating_mul(std::mem::size_of::<RecordId>())
        .saturating_add(requests.holds.len().saturating_mul(std::mem::size_of::<(RecordId, RecordId)>())) )?;
    let targets = requests.targets.into_iter().collect::<Vec<_>>();
    let holds = requests.holds.into_iter().collect::<Vec<_>>();
    let mut backing = crate::control::inspect_expected_window_read(reader, &targets, budget).await?;
    for window in holds.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
        let mut actual = crate::control::inspect_hold_window_read(reader, window, budget).await?;
        for (owner, object) in window {
            actual.take_required(owner, object)?;
        }
        actual.finish()?;
    }
    for target in &targets {
        let expected = backing.take_required(target)?;
        let (endpoints, original, _decode) = canonical_backing_endpoints(expected.row(), budget)?;
        if original.is_some() {
            return Err(ModelError::Conflict("installed entity alias has original metadata"));
        }
        for window in endpoints.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            let expected = window.iter().map(|row| {
                let Some(Value::RecordId(id)) = row.as_object().and_then(|row| row.get("id")) else {
                    return Err(ModelError::Schema("installed alias endpoint identity"));
                };
                Ok((id.clone(), row.clone()))
            }).collect::<Result<BTreeMap<_, _>, ModelError>>()?;
            audit_endpoint_window(reader, owner, &expected, budget).await?;
        }
    }
    backing.finish()
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
) -> Result<LogicalContributorRows, ModelError> {
    let mut window = crate::loader::NATIVE_WINDOW_ROWS;
    loop {
        match prepare_logical_contributor_window(reader, expected, owners, budget, window).await {
            // This entire effect-free attempt has dropped its provisional sorters
            // and observations. A poisoned sorter never participates in retry.
            Err(error) if window > 1 && terminal_local_resource(&error) => {
                window = (window / 2).max(1);
            }
            result => return result,
        }
    }
}
fn terminal_local_resource(mut error: &ModelError) -> bool {
    // Owned streams share their terminal driver error. This wrapper adds no
    // finalization state; Completion and contextual Cause must remain opaque.
    while let ModelError::SharedCause(inner) = error {
        error = inner.as_ref();
    }
    matches!(error, ModelError::Resource { .. })
}
async fn prepare_logical_contributor_window(
    reader: &NativeReader<()>,
    expected: &ExpectedRecord,
    owners: &BTreeMap<RecordId, CompletedContribution>,
    budget: &ResourceBudget,
    window: usize,
) -> Result<LogicalContributorRows, ModelError> {
    use crate::ordered_rows::SortedRows;
    use d::completion::{Completion, complete};
    let mut contribution =
        SortedRows::with_budget_and_row_bytes(budget, d::resources::MAX_ROW_BYTES)?;
    let mut memberships =
        SortedRows::with_budget_and_row_bytes(budget, d::resources::MAX_ROW_BYTES)?;
    let mut backing = SortedRows::with_budget_and_row_bytes(budget, d::resources::MAX_ROW_BYTES)?;
    let mut backing_ids = SortedRows::with_budget(budget)?;
    {
        let _decode = budget.reserve(
            "logical contributor fragment normalization",
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        validate_state_row("compiler_contribution", expected.row())?;
        contribution.push(crate::completed_logical_rows::normalize(
            "compiler_contribution",
            expected.row(),
            owners,
        )?)?;
    }
    let mut variables = Variables::new();
    variables.insert("owner", expected.record().clone());
    let statements = vec![
        "SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner"
            .into(),
    ];
    #[cfg(test)]
    let statements = {
        let mut statements = statements;
        if LATE_MEMBERSHIP_FAILURE
            .try_with(|enabled| *enabled)
            .unwrap_or(false)
        {
            statements.push("THROW 'late logical contributor fragment control'".into());
        }
        statements
    };
    let mut rows = reader.stream_prepared(crate::prepared::PreparedQuery::new(
        variables,
        vec![],
        statements,
    )?)?;
    let result = async {
        while let Some(row) = rows.next().await? {
            validate_state_row("compiler_membership", &row)?;
            if row.as_object().and_then(|row| row.get("contribution"))
                != Some(&Value::RecordId(expected.record().clone()))
            {
                return Err(ModelError::Conflict(
                    "logical fragment foreign physical membership",
                ));
            }
            memberships.push(crate::completed_logical_rows::normalize(
                "compiler_membership",
                &row,
                owners,
            )?)?;
            let Some(Value::RecordId(node)) = row.as_object().and_then(|row| row.get("node"))
            else {
                return Err(ModelError::Schema("logical backing member node"));
            };
            if node.table.as_str() == "compiler_record" {
                let mut pointer = surrealdb::types::Object::new();
                pointer.insert("id", node.clone());
                backing_ids.push(Value::Object(pointer))?;
            }
        }
        Ok::<_, ModelError>(())
    }
    .await;
    let mut terminal = Completion::default();
    terminal.step(
        "installed logical fragment membership drainage",
        rows.drain_transport().await,
    );
    complete(result, terminal)?;
    let mut backing_ids = backing_ids.finish()?;
    if window == 1 {
        // Preserve the original singleton capacity and refusal without a page's
        // sibling reservations or ID-vector overhead.
        while let Some(pointer) = backing_ids.next_row()? {
            let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("id"))
            else {
                return Err(ModelError::Schema("logical backing identity"));
            };
            let actual = crate::control::inspect_expected_read(reader, id.clone(), budget)
                .await?
                .ok_or(ModelError::Conflict("logical backing missing"))?;
            validate_state_row("compiler_record", actual.row())?;
            // compiler_record logical normalization is byte-preserving. Move the
            // validated row and arena instead of cloning and retaining a dead
            // receiving/decode arena through the sorter's merge workspace.
            let (row, arena) = actual.into_owned_row();
            backing.push_owned(row, arena)?;
        }
    } else {
        loop {
            let mut charge = StateCharge::new(budget, "logical backing exact observation IDs");
            charge.grow(window.saturating_mul(std::mem::size_of::<RecordId>()))?;
            let mut ids = Vec::with_capacity(window);
            while ids.len() < window {
                let Some(pointer) = backing_ids.next_row()? else {
                    break;
                };
                let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("id"))
                else {
                    return Err(ModelError::Schema("logical backing identity"));
                };
                charge.grow(crate::selection::node_bytes(id))?;
                ids.push(id.clone());
            }
            if ids.is_empty() {
                break;
            }
            let mut actual =
                crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
            for id in &ids {
                let row = actual.take_required(id)?;
                validate_state_row("compiler_record", row.row())?;
                let (row, arena) = row.into_owned_row();
                backing.push_owned(row, arena)?;
            }
            actual.finish()?;
        }
    }
    Ok(LogicalContributorRows {
        contribution: contribution.finish()?.into_prepared(),
        memberships: memberships.finish()?.into_prepared(),
        backing: backing.finish()?.into_prepared(),
    })
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
        completed_identity::CompletedFamilyHasher,
        completion::{Completion, complete},
        recovery_closure::COMPLETED_FAMILIES,
    };
    d::completed::binding_inventory_identity(bindings)?;
    if let Some(audit) = audit {
        audit.ensure_context(reader, budget).await?;
        for closure in closures {
            prepare_closure_in_context(reader, closure, budget, audit).await?;
        }
    }
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
            if let Some(previous) = incarnations.insert(id.clone(), expected.incarnation()) {
                if previous != expected.incarnation() {
                    return Err(ModelError::Conflict(
                        "family exact physical incarnation contradiction",
                    ));
                }
            }
        }
        for (identity, view) in closure.logical.views() {
            if let Some(previous) = views.insert(*identity, view.clone()) {
                if previous != *view {
                    return Err(ModelError::Conflict(
                        "family exact view descriptor contradiction",
                    ));
                }
            }
        }
    }
    let mut fragments: [Vec<crate::ordered_rows::PreparedRows>; 6] =
        std::array::from_fn(|_| Vec::new());
    let mut fragment_charge = StateCharge::new(budget, "logical family prepared run handles");
    let mut families = COMPLETED_FAMILIES
        .map(|_| SortedRows::with_budget_and_row_bytes(budget, d::resources::MAX_ROW_BYTES))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut backing_ids = SortedRows::with_budget(budget)?;
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
            if let Some(audit) = audit.as_deref() {
                audit.check(reader, budget)?;
                let cached =
                    audit
                        .contributors
                        .get(expected.record())
                        .ok_or(ModelError::Conflict(
                            "logical fragment requires complete protected contributor audit",
                        ))?;
                if cached.incarnation() != expected.incarnation() || cached.row() != expected.row()
                {
                    return Err(ModelError::Conflict(
                        "logical fragment exact contributor premise changed",
                    ));
                }
                let prepared = audit
                    .logical_contributors
                    .get(expected.record())
                    .ok_or(ModelError::Conflict("prepared contributor absent"))?;
                for (family, fragment) in [
                    (0, &prepared.contribution),
                    (1, &prepared.memberships),
                    (3, &prepared.backing),
                ] {
                    fragment_charge.grow(
                        std::mem::size_of::<crate::ordered_rows::PreparedRows>().saturating_mul(2),
                    )?;
                    fragments[family].push(fragment.clone());
                }
                continue;
            }
            families[0].push(crate::completed_logical_rows::normalize(
                "compiler_contribution",
                expected.row(),
                &owners,
            )?)?;
            let mut variables = Variables::new();
            variables.insert("owner", expected.record().clone());
            let mut rows=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner".into()])?)?;
            let result = async {
                while let Some(row) = rows.next().await? {
                    validate_state_row("compiler_membership", &row)?;
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
                        let mut pointer = surrealdb::types::Object::new();
                        pointer.insert("id", node.clone());
                        backing_ids.push(Value::Object(pointer))?;
                    }
                }
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
        if audit.is_some() {
            // inspect_closure already independently read and physically decoded
            // this exact native row, and RecoveryTraversal compared the full
            // typed descriptor with the requested view. Logical lowering uses
            // only that descriptor and its derived identity, never native extras.
            // Admit serialization scratch before constructing its logical row.
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
            families[2].push(crate::completed_logical_rows::normalize(
                "compiler_view",
                &Value::Object(row),
                &owners,
            )?)?;
            continue;
        }
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
    let mut backing_ids = backing_ids.finish()?;
    while let Some(pointer) = backing_ids.next_row()? {
        let Some(Value::RecordId(id)) = pointer.as_object().and_then(|row| row.get("id")) else {
            return Err(ModelError::Schema("logical backing identity"));
        };
        let actual = crate::control::inspect_expected_read(reader, id.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict("logical backing missing"))?;
        validate_state_row("compiler_record", actual.row())?;
        families[3].push(crate::completed_logical_rows::normalize(
            "compiler_record",
            actual.row(),
            &owners,
        )?)?;
    }
    for binding in bindings {
        if !closures.iter().any(|closure| {
            closure.logical.views().get(&binding.view.identity) == Some(&binding.view)
        }) {
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
        families[4].push(crate::completed_logical_rows::normalize(
            "compiler_binding",
            &Value::Object(row),
            &owners,
        )?)?;
    }
    let mut aliases = aliases.cursor_with_budget(budget)?;
    while let Some(row) = aliases.next_row()? {
        families[5].push(crate::completed_logical_rows::normalize(
            "compiler_alias",
            &row,
            &owners,
        )?)?;
    }
    let mut roots = Vec::with_capacity(6);
    for (index, (family, rows)) in COMPLETED_FAMILIES.into_iter().zip(families).enumerate() {
        fragments[index].push(rows.finish()?.into_prepared());
        let dependency = family_dependency(
            index,
            &owners,
            &incarnations,
            &views,
            bindings,
            &fragments[index],
            budget,
        )?;
        if let Some(audit) = audit {
            let runs = std::mem::take(&mut fragments[index]);
            let worker_budget = budget.clone();
            let handles = budget.reserve(
                "family root preparation worker run handles",
                runs.capacity()
                    .saturating_mul(std::mem::size_of::<crate::ordered_rows::PreparedRows>())
                    .saturating_add(1024),
            )?;
            audit
                .run_work((4, dependency), move |owner| {
                    async move {
                        let _handles = handles;
                        if owner.family_roots.contains_key(&(index, dependency)) {
                            return Ok(());
                        }
                        let mut hasher = CompletedFamilyHasher::new(family)?;
                        crate::ordered_rows::PreparedRows::fold_union(
                            &runs,
                            &worker_budget,
                            || owner.pin.ensure_live(),
                            |_, payload| hasher.row(payload),
                        )?;
                        let root = hasher.finish();
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
pub(crate) async fn prepare_closure_scoped(
    reader: &NativeReader<()>,
    closure: &InspectedClosure,
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<Vec<LogicalContributorRows>, ModelError> {
    audit.ensure_context(reader, budget).await?;
    prepare_closure_in_context(reader, closure, budget, audit).await
}
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
    for expected in closure.contributors.values() {
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
        if let Some(prepared) = audit.logical_contributors.get(expected.record()) {
            audit.check(reader, budget)?;
            result.push(prepared.as_ref().clone());
            continue;
        }
        let (key, input) = exact_work(expected, budget)?;
        let worker_budget = budget.clone();
        audit
            .run_work((1, key), move |owner| {
                async move {
                    let reader =
                        NativeReader::private(owner.client.clone()).with_budget(&worker_budget);
                    let result = async {
                        let expected = worker_expected(&reader, &input, &worker_budget).await?;
                        if owner.logical_contributors.contains_key(expected.record()) {
                            return Ok(None);
                        }
                        let descriptor = descriptor::<CompletedContribution>(expected.row())?;
                        let owners = BTreeMap::from([(expected.record().clone(), descriptor)]);
                        let prepared = prepare_logical_contributor(
                            &reader,
                            &expected,
                            &owners,
                            &worker_budget,
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
            .await?;
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
            if let Some(previous) = logical.insert(identity, descriptor) {
                if previous != descriptor {
                    return Err(ModelError::Conflict(
                        "logical contributor dependency collision",
                    ));
                }
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
            if let Some(previous) = ordered.insert(binding.key(), binding) {
                if previous != binding {
                    return Err(ModelError::Conflict("binding family dependency collision"));
                }
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
            .logical_contributors
            .get(&physical)
            .ok_or(ModelError::Conflict("selected prepared contributor absent"))?;
        handles.grow(std::mem::size_of::<crate::ordered_rows::PreparedRows>().saturating_mul(2))?;
        runs.push(prepared.memberships.clone());
    }
    let relation = view.relation.clone();
    let count = view.rows;
    let worker_budget = budget.clone();
    audit
        .run_work((5, selector), move |owner| {
            async move {
                let _handles = handles;
                if owner.selected_members.contains_key(&selector) {
                    return Ok(());
                }
                let mut rows = crate::ordered_rows::SortedRows::with_budget(&worker_budget)?;
                for run in runs {
                    let mut cursor = run.cursor_with_budget(&worker_budget)?;
                    while let Some(row) = cursor.next_row()? {
                        owner.pin.ensure_live()?;
                        if row.as_object().and_then(|row| row.get("relation"))
                            == Some(&Value::String(relation.clone()))
                        {
                            rows.push(crate::completed_state::earning::member(selector, &row)?)?;
                        }
                    }
                }
                let rows = rows.finish()?.into_prepared();
                let content = crate::completed_state::earning::selected_membership_content(
                    &rows,
                    count,
                    &worker_budget,
                )?;
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
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    };
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
        assert_eq!(requests.holds, BTreeSet::from([
            (id(&alias), source), (id(&second), other), (owner, target),
        ]));
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
        assert!(matches!(requests.observe(
            &exact_alias(&RecordId::new("compiler_record", "foreign"), &target), &owner),
            Err(ModelError::Conflict("installed alias entity family"))));
        let mut forged = exact_alias(&source, &target);
        if let Value::Object(object) = &mut forged {
            object.insert("id", RecordId::new("compiler_alias", "wrong"));
        }
        assert!(matches!(requests.observe(&forged, &owner),
            Err(ModelError::Conflict("installed exact alias envelope"))));
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
            logical_contributors: SharedMap::new(),
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
            }),
        }
    }
    async fn join(audit: &InstallationAudit) {
        audit.drain_workers().await.unwrap();
        audit.closed.store(true, Ordering::Release);
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
        let keys = [(6, ContentHash([10; 32])), (7, ContentHash([11; 32])), (8, ContentHash([12; 32]))];
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let seen = Arc::new(std::sync::Mutex::new(BTreeSet::new()));
        let first = {
            let audit = audit.clone(); let entered = entered.clone(); let release = release.clone(); let seen = seen.clone();
            tokio::spawn(async move { audit.run_work_many(keys[..2].to_vec(), move |_, claimed| async move {
                assert_eq!(claimed, keys[..2]);
                seen.lock().unwrap().extend(claimed);
                entered.wait().await;
                let _permit = release.acquire().await.unwrap();
                Err(ModelError::Conflict("late window refusal"))
            }.boxed()).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), entered.wait()).await.unwrap();
        let peer_claimed = Arc::new(tokio::sync::Barrier::new(2));
        let peer = {
            let audit = audit.clone(); let seen = seen.clone(); let peer_claimed = peer_claimed.clone();
            tokio::spawn(async move { audit.run_work_many(keys.to_vec(), move |_, claimed| async move {
                assert_eq!(claimed, vec![keys[2]]);
                seen.lock().unwrap().extend(claimed);
                peer_claimed.wait().await;
                Ok(())
            }.boxed()).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), peer_claimed.wait()).await.unwrap();
        first.abort(); let _ = first.await;
        release.add_permits(1);
        assert!(peer.await.unwrap().unwrap_err().permits_storage_cleanup());
        join(&audit).await;
        assert_eq!(*seen.lock().unwrap(), BTreeSet::from(keys));
        let flights = audit.work.flights.lock().unwrap();
        assert!(!flights.contains_key(&keys[0]) && !flights.contains_key(&keys[1]));
        assert!(flights.contains_key(&keys[2]));
        assert_eq!(audit.budget.reserved(), 4096 + audit.charge.0.lock().unwrap().reserved());
        assert!(!audit.pin.test_uncertain());
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn failed_window_flight_reservations_release_before_repeated_resource_retry() {
        let audit = test_audit(16384);
        let keys = vec![(6, ContentHash([20; 32])), (7, ContentHash([21; 32]))];
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let live = {
            let audit = audit.clone(); let entered = entered.clone(); let release = release.clone();
            tokio::spawn(async move { audit.run_work((8, ContentHash([22; 32])), move |_| async move {
                entered.wait().await;
                let _permit = release.acquire().await.unwrap();
                Ok(())
            }.boxed()).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), entered.wait()).await.unwrap();
        let mut retained = None;
        for _ in 0..4 {
            let retry = audit.run_work_many(keys.clone(), move |_, _| async {
                Err(ModelError::Resource { owner: "window control", requested: 1, used: 8192, limit: 8192 })
            }.boxed());
            let error = tokio::time::timeout(std::time::Duration::from_secs(30), retry).await.unwrap().unwrap_err();
            assert!(terminal_local_resource(&error));
            let flights = audit.work.flights.lock().unwrap();
            assert_eq!(flights.len(), 1);
            assert!(keys.iter().all(|key| !flights.contains_key(key)));
            drop(flights);
            assert!(!live.is_finished());
            let bytes = audit.budget.reserved();
            assert_eq!(bytes, 4096 + audit.charge.0.lock().unwrap().reserved());
            assert_eq!(*retained.get_or_insert(bytes), bytes);
        }
        release.add_permits(1);
        live.await.unwrap().unwrap();
        audit.run_work_many(keys, move |_, _| async { Ok(()) }.boxed()).await.unwrap();
        join(&audit).await;
        assert_eq!(audit.budget.reserved(), 12288 + audit.charge.0.lock().unwrap().reserved());
        assert!(!audit.pin.test_uncertain());
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn nested_preparation_reaps_terminal_child_without_joining_current_worker() {
        let audit = test_audit(1 << 20);
        let result = audit.run_work((6, ContentHash([30; 32])), move |owner| async move {
            let error = owner.run_work((7, ContentHash([31; 32])), move |_| async {
                Err(ModelError::Resource { owner: "nested control", requested: 2, used: 1, limit: 1 })
            }.boxed()).await.unwrap_err();
            assert!(terminal_local_resource(&error));
            Ok(())
        }.boxed());
        tokio::time::timeout(std::time::Duration::from_secs(30), result).await.unwrap().unwrap();
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
            let audit = audit.clone(); let entered = entered.clone(); let release = release.clone();
            tokio::spawn(async move { audit.run_work((6, ContentHash([40; 32])), move |_| async move {
                entered.wait().await;
                let _permit = release.acquire().await.unwrap();
                Ok(())
            }.boxed()).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), entered.wait()).await.unwrap();
        let failed_entered = Arc::new(tokio::sync::Barrier::new(2));
        let fail = Arc::new(tokio::sync::Semaphore::new(0));
        let failed = {
            let audit = audit.clone(); let entered = failed_entered.clone(); let fail = fail.clone();
            tokio::spawn(async move { audit.run_work((7, ContentHash([41; 32])), move |_| async move {
                entered.wait().await;
                let _permit = fail.acquire().await.unwrap();
                panic!("deterministic drain join failure")
            }.boxed()).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), failed_entered.wait()).await.unwrap();
        failed.abort(); let _ = failed.await;
        fail.add_permits(1);
        // Observe terminality without joining/removing the production owner's handle.
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                if audit.work.workers.lock().unwrap().last().unwrap().1.is_finished() { break; }
                tokio::task::yield_now().await;
            }
        }).await.unwrap();
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
        let error = tokio::time::timeout(std::time::Duration::from_secs(30), retry).await.unwrap().unwrap_err();
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
