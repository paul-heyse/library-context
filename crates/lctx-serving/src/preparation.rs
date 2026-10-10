//! Viewer-owned exact preparation. Requests own only their wait and fresh validators.
use lctx_model::domain::{
    ModelError, ValidationInput,
    resources::{Reservation, ResourceBudget},
    serving::{ResourceLimits, SnapshotHandle},
};
use lctx_surrealdb::{batches::CanonicalBatches, scope::PreparedServingScope};
use moka::future::Cache;
use std::{
    collections::HashMap,
    future::Future,
    hash::{Hash, Hasher},
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
struct ExactPreparedKey {
    bytes: Arc<Vec<u8>>,
    _charge: Arc<Box<dyn Reservation>>,
}
impl PartialEq for ExactPreparedKey {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
impl Eq for ExactPreparedKey {}
impl Hash for ExactPreparedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}
fn weight_units(bytes: usize) -> Option<u32> {
    u32::try_from(bytes.div_ceil(1024)).ok()
}

type Initialization =
    Pin<Box<dyn Future<Output = Result<PreparedData, ModelError>> + Send + 'static>>;
type Initializer = Box<dyn Fn(ResourceBudget) -> Initialization + Send + Sync + 'static>;

pub(crate) enum PreparedData {
    Layout(Arc<PreparedServingScope>),
    Batches(Arc<CanonicalBatches>),
    #[cfg(test)]
    Test {
        _charge: Box<dyn Reservation>,
    },
}
struct ChargedPreparedValue {
    data: PreparedData,
    budget: ResourceBudget,
    _key: ExactPreparedKey,
    _lease: ValueLease,
}
type Retention = Cache<ExactPreparedKey, Arc<ChargedPreparedValue>>;
type Terminal = Result<Arc<ChargedPreparedValue>, Arc<ModelError>>;
struct Flight {
    terminal: Mutex<Option<Terminal>>,
    changed: Notify,
    _charge: Box<dyn Reservation>,
}
impl Flight {
    async fn wait(&self) -> Result<Arc<ChargedPreparedValue>, ModelError> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if let Some(result) = self.terminal.lock().expect("preparation terminal").clone() {
                return result.map_err(ModelError::SharedCause);
            }
            changed.await;
        }
    }
}
#[derive(Default)]
struct State {
    closing: bool,
    owners: usize,
    values: usize,
    requests: usize,
    retention: Option<Retention>,
    generation: u64,
    publication: u64,
    flights: HashMap<ExactPreparedKey, Arc<Flight>>,
}
#[derive(Default)]
struct Lifecycle {
    state: Mutex<State>,
    changed: Notify,
}
pub(crate) struct RequestLease(Arc<Lifecycle>);
impl Drop for RequestLease {
    fn drop(&mut self) {
        let mut s = self.0.state.lock().expect("preparation lifecycle");
        s.requests -= 1;
        self.0.changed.notify_waiters();
    }
}
struct InsertionOwner {
    lifecycle: Arc<Lifecycle>,
    key: ExactPreparedKey,
    flight: Arc<Flight>,
}
impl Drop for InsertionOwner {
    fn drop(&mut self) {
        // Also make an unwinding loader terminal, so cancellation/panic cannot strand waiters.
        let mut terminal = self.flight.terminal.lock().expect("preparation terminal");
        if terminal.is_none() {
            *terminal = Some(Err(Arc::new(ModelError::codec(
                "preparation loader ended without a terminal result",
            ))));
        }
        drop(terminal);
        self.flight.changed.notify_waiters();
        let removed = {
            let mut s = self.lifecycle.state.lock().expect("preparation lifecycle");
            s.owners -= 1;
            if s.flights
                .get(&self.key)
                .is_some_and(|flight| Arc::ptr_eq(flight, &self.flight))
            {
                s.publication = s.publication.wrapping_add(1);
                let removed = s.flights.remove(&self.key);
                // Do not retain an uncharged high-water allocation after flight charges leave.
                if s.flights.is_empty() {
                    s.flights = HashMap::new();
                } else if s.flights.capacity() > s.flights.len().saturating_mul(4) {
                    s.flights.shrink_to_fit();
                }
                removed
            } else {
                None
            }
        };
        drop(removed);
        self.lifecycle.changed.notify_waiters();
    }
}
#[derive(Clone, PartialEq, serde::Serialize)]
enum PreparationIdentity {
    Publication(SnapshotHandle),
    #[cfg(test)]
    CompilerViews(surrealdb::types::Variables),
}
struct ValueLease {
    lifecycle: Arc<Lifecycle>,
    _pin: PreparationIdentity,
}
impl Drop for ValueLease {
    fn drop(&mut self) {
        let mut s = self.lifecycle.state.lock().expect("preparation lifecycle");
        s.values -= 1;
        self.lifecycle.changed.notify_waiters();
    }
}
/// One viewer and one immutable handle. Eviction never releases a borrower's charge.
pub(crate) struct PreparedCache {
    pin: PreparationIdentity,
    capacity: u64,
    lifecycle: Arc<Lifecycle>,
    budget: ResourceBudget,
    queries: Arc<Semaphore>,
    cpu: Arc<Semaphore>,
    #[cfg(test)]
    after_miss: Mutex<Option<(Arc<Notify>, Arc<Semaphore>)>>,
}
impl PreparedCache {
    pub(crate) fn new(
        pin: SnapshotHandle,
        shared: &ResourceBudget,
        limits: &ResourceLimits,
        queries: Arc<Semaphore>,
        cpu: Arc<Semaphore>,
    ) -> Result<Arc<Self>, ModelError> {
        Self::with_identity(
            PreparationIdentity::Publication(pin),
            shared,
            limits,
            queries,
            cpu,
        )
    }
    #[cfg(test)]
    pub(crate) fn for_scope(
        reader: &lctx_surrealdb::NativeReader<()>,
        shared: &ResourceBudget,
        limits: &ResourceLimits,
        queries: Arc<Semaphore>,
        cpu: Arc<Semaphore>,
    ) -> Result<Arc<Self>, ModelError> {
        Self::with_identity(
            PreparationIdentity::CompilerViews(reader.view_bindings()),
            shared,
            limits,
            queries,
            cpu,
        )
    }
    fn with_identity(
        pin: PreparationIdentity,
        shared: &ResourceBudget,
        limits: &ResourceLimits,
        queries: Arc<Semaphore>,
        cpu: Arc<Semaphore>,
    ) -> Result<Arc<Self>, ModelError> {
        let budget = ResourceBudget::scoped(shared, limits.preparation_bytes as usize)?;
        let capacity = limits
            .preparation_bytes
            .div_ceil(1024)
            .min(u64::from(u32::MAX) - 1);
        let lifecycle = Arc::new(Lifecycle {
            state: Mutex::new(State {
                retention: Some(Self::empty_cache(capacity)),
                ..Default::default()
            }),
            changed: Notify::new(),
        });
        Ok(Arc::new(Self {
            pin,
            capacity,
            lifecycle,
            budget,
            queries,
            cpu,
            #[cfg(test)]
            after_miss: Mutex::new(None),
        }))
    }
    fn empty_cache(capacity: u64) -> Cache<ExactPreparedKey, Arc<ChargedPreparedValue>> {
        Cache::builder()
            .max_capacity(capacity)
            .weigher(|_: &ExactPreparedKey, v: &Arc<ChargedPreparedValue>| {
                weight_units(v.budget.reserved().saturating_add(v._key._charge.size()))
                    .unwrap_or(u32::MAX)
            })
            .build()
    }
    fn release_optional_retention(&self) {
        // Moka logical invalidation defers payload destruction. Replace its replaceable
        // retention owner instead: once active initializers release their clones, dropping
        // the old cache destroys its unborrowed values. External Arc borrowers remain live.
        let old = {
            let mut s = self.lifecycle.state.lock().expect("preparation lifecycle");
            s.generation = s.generation.wrapping_add(1);
            s.retention.replace(Self::empty_cache(self.capacity))
        };
        drop(old);
    }
    fn reserve_capture(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn Reservation>, ModelError> {
        match self.budget.reserve(owner, bytes) {
            Err(ModelError::Resource { .. } | ModelError::Limit { .. }) => {
                self.release_optional_retention();
                self.budget.reserve(owner, bytes)
            }
            result => result,
        }
    }
    pub(crate) fn admit_request(&self) -> Result<RequestLease, ModelError> {
        let mut s = self.lifecycle.state.lock().expect("preparation lifecycle");
        if s.closing {
            return Err(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::Unavailable,
            ));
        }
        s.requests += 1;
        Ok(RequestLease(self.lifecycle.clone()))
    }
    fn key(
        &self,
        kind: &str,
        roots: &[surrealdb::types::RecordId],
        inputs: &[ValidationInput],
        incoming: &[ValidationInput],
        fields: &[&str],
    ) -> Result<ExactPreparedKey, ModelError> {
        // Conservative full-contract bytes; digests and physical aliases never replace equality.
        let estimated = 2048usize
            .saturating_add(roots.len().saturating_mul(512))
            .saturating_add(
                inputs
                    .iter()
                    .chain(incoming)
                    .map(|i| {
                        512 + i.name().len() + i.order().iter().map(|s| s.len() + 32).sum::<usize>()
                    })
                    .sum::<usize>(),
            )
            .saturating_add(fields.iter().map(|s| s.len() + 32).sum::<usize>());
        let mut charge = self
            .budget
            .reserve("viewer-prepared-exact-key", estimated)?;
        fn inventory(values: &[ValidationInput]) -> Vec<(&str, Option<u8>, &[&str])> {
            values
                .iter()
                .map(|i| (i.name(), i.prefix().map(|p| p.code()), i.order()))
                .collect()
        }
        let mut roots = roots
            .iter()
            .map(|r| serde_json::to_vec(r).map_err(ModelError::codec))
            .collect::<Result<Vec<_>, _>>()?;
        roots.sort();
        roots.dedup();
        let bytes = serde_json::to_vec(&(
            "viewer-preparation/v1",
            &self.pin,
            crate::operation_definition(),
            kind,
            roots,
            inventory(inputs),
            inventory(incoming),
            fields,
        ))
        .map_err(ModelError::codec)?;
        charge.try_resize(bytes.capacity() + size_of::<ExactPreparedKey>() + 256)?;
        Ok(ExactPreparedKey {
            bytes: Arc::new(bytes),
            _charge: Arc::new(charge),
        })
    }
    async fn available_key(
        &self,
        kind: &str,
        roots: &[surrealdb::types::RecordId],
        inputs: &[ValidationInput],
        incoming: &[ValidationInput],
        fields: &[&str],
    ) -> Result<ExactPreparedKey, ModelError> {
        match self.key(kind, roots, inputs, incoming, fields) {
            Err(error) if matches!(error.primary(), Some(ModelError::Resource { .. })) => {
                self.release_optional_retention();
                self.key(kind, roots, inputs, incoming, fields)
            }
            result => result,
        }
    }
    async fn load(
        self: &Arc<Self>,
        key: ExactPreparedKey,
        initialize: Initializer,
    ) -> Result<Arc<ChargedPreparedValue>, ModelError> {
        // A waiter owns only its independent wait; the flight owns the loader to terminality.
        if self
            .lifecycle
            .state
            .lock()
            .expect("preparation lifecycle")
            .closing
        {
            return Err(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::Unavailable,
            ));
        }
        let _waiter = self.reserve_capture("viewer-preparation-waiter", 1024)?;
        let mut candidate = None;
        let mut initialize = Some(initialize);
        let flight = loop {
            let (retention, generation, publication) = {
                let s = self.lifecycle.state.lock().expect("preparation lifecycle");
                if s.closing {
                    return Err(ModelError::Serving(
                        lctx_model::domain::serving::FailureKind::Unavailable,
                    ));
                }
                if let Some(flight) = s.flights.get(&key) {
                    break flight.clone();
                }
                (
                    s.retention.as_ref().expect("viewer retention").clone(),
                    s.generation,
                    s.publication,
                )
            };
            let cached = retention.get(&key).await;
            drop(retention);
            if let Some(value) = cached {
                if self
                    .lifecycle
                    .state
                    .lock()
                    .expect("preparation lifecycle")
                    .closing
                {
                    return Err(ModelError::Serving(
                        lctx_model::domain::serving::FailureKind::Unavailable,
                    ));
                }
                return Ok(value);
            }
            #[cfg(test)]
            {
                let pause = self.after_miss.lock().unwrap().take();
                if let Some((reached, gate)) = pause {
                    reached.notify_one();
                    let _permit = gate.acquire().await.unwrap();
                }
            }
            // Reserve before locking: reclaiming retention can destroy a ValueLease.
            if candidate.is_none() {
                candidate = Some(Arc::new(Flight {
                    terminal: Mutex::new(None),
                    changed: Notify::new(),
                    _charge: self.reserve_capture("viewer-preparation-flight", 4096)?,
                }));
            }
            let owner = {
                let mut s = self.lifecycle.state.lock().expect("preparation lifecycle");
                if s.closing {
                    return Err(ModelError::Serving(
                        lctx_model::domain::serving::FailureKind::Unavailable,
                    ));
                }
                if let Some(flight) = s.flights.get(&key) {
                    break flight.clone();
                }
                // Completion and replacement use this same lock. A stale asynchronous miss
                // cannot install after another loader published and removed its flight.
                if s.generation != generation || s.publication != publication {
                    continue;
                }
                let flight = candidate.take().expect("charged flight");
                s.flights.insert(key.clone(), flight.clone());
                s.owners += 1;
                InsertionOwner {
                    lifecycle: self.lifecycle.clone(),
                    key: key.clone(),
                    flight,
                }
            };
            let flight = owner.flight.clone();
            let this = self.clone();
            let initialize = initialize.take().expect("flight initializer");
            let loader_key = key.clone();
            tokio::spawn(async move {
                let owner = owner;
                let result = this
                    .initialize(&loader_key, &initialize)
                    .await
                    .map_err(Arc::new);
                // Keep the flight joinable during optional publication. Waiters own their
                // terminal Arc independently, and never depend on deferred Moka maintenance.
                *owner.flight.terminal.lock().expect("preparation terminal") = Some(result.clone());
                if let Ok(value) = &result {
                    let retention = {
                        let s = this.lifecycle.state.lock().expect("preparation lifecycle");
                        if !s.closing
                            && weight_units(
                                value
                                    .budget
                                    .reserved()
                                    .saturating_add(value._key._charge.size()),
                            )
                            .is_some()
                        {
                            s.retention.clone()
                        } else {
                            None
                        }
                    };
                    // Replacement while insert is pending may forgo retention entirely.
                    if let Some(retention) = retention {
                        retention.insert(loader_key.clone(), value.clone()).await;
                    }
                }
                // Drain reader/layout captures before advertising loader drainage to close.
                drop(initialize);
                drop(loader_key);
                drop(result);
                drop(owner); // Exact identity removal and publication sequence advance.
            });
            break flight;
        };
        // A joiner never retains a second initializer's charged captures while waiting.
        drop(candidate);
        drop(initialize);
        drop(key);
        flight.wait().await
    }
    async fn initialize(
        &self,
        key: &ExactPreparedKey,
        initialize: &Initializer,
    ) -> Result<Arc<ChargedPreparedValue>, ModelError> {
        for retry in 0..2 {
            // Close fences retries as well as initial installation and queued admission.
            if self
                .lifecycle
                .state
                .lock()
                .expect("preparation lifecycle")
                .closing
            {
                return Err(ModelError::Serving(
                    lctx_model::domain::serving::FailureKind::Unavailable,
                ));
            }
            let result = async {
                let _query = self.queries.clone().acquire_owned().await.map_err(|_| {
                    ModelError::Serving(lctx_model::domain::serving::FailureKind::Unavailable)
                })?;
                let _cpu = self.cpu.clone().acquire_owned().await.map_err(|_| {
                    ModelError::Serving(lctx_model::domain::serving::FailureKind::Unavailable)
                })?;
                if self
                    .lifecycle
                    .state
                    .lock()
                    .expect("preparation lifecycle")
                    .closing
                {
                    return Err(ModelError::Serving(
                        lctx_model::domain::serving::FailureKind::Unavailable,
                    ));
                }
                let budget = ResourceBudget::scoped(&self.budget, self.budget.limit())?;
                let data = initialize(budget.clone()).await?;
                let lease = {
                    let mut s = self.lifecycle.state.lock().expect("preparation lifecycle");
                    s.values += 1;
                    ValueLease {
                        lifecycle: self.lifecycle.clone(),
                        _pin: self.pin.clone(),
                    }
                };
                Ok(Arc::new(ChargedPreparedValue {
                    data,
                    budget,
                    _key: key.clone(),
                    _lease: lease,
                }))
            }
            .await;
            if retry == 0
                && result.as_ref().is_err_and(|e| {
                    matches!(
                        e.primary(),
                        Some(ModelError::Resource { .. } | ModelError::Limit { .. })
                    )
                })
            {
                self.release_optional_retention();
                continue;
            }
            return result;
        }
        unreachable!("preparation retry returns its terminal result")
    }
    pub(crate) async fn close(&self) {
        {
            self.lifecycle
                .state
                .lock()
                .expect("preparation lifecycle")
                .closing = true;
        }
        self.queries.close();
        self.cpu.close(); // Cancel queued owners; submitted work still drains.
        loop {
            let changed = self.lifecycle.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if {
                let s = self.lifecycle.state.lock().expect("preparation lifecycle");
                s.owners == 0 && s.requests == 0
            } {
                break;
            }
            changed.await;
        }
        self.release_optional_retention();
        loop {
            let changed = self.lifecycle.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self
                .lifecycle
                .state
                .lock()
                .expect("preparation lifecycle")
                .values
                == 0
            {
                break;
            }
            changed.await;
        }
    }
}
/// Request admission is surrendered while waiting for viewer preparation; the initializer uses
/// these same permits, so one waiting request cannot occupy the last loader slot.
pub(crate) struct RequestAdmission {
    queries: Arc<Semaphore>,
    cpu: Arc<Semaphore>,
    held: tokio::sync::Mutex<Option<(OwnedSemaphorePermit, OwnedSemaphorePermit)>>,
    deadline: tokio::time::Instant,
    admission_wait: std::time::Duration,
}
impl RequestAdmission {
    pub(crate) async fn new(
        queries: Arc<Semaphore>,
        cpu: Arc<Semaphore>,
        deadline: tokio::time::Instant,
        admission_wait: std::time::Duration,
    ) -> Result<Self, ModelError> {
        let this = Self {
            queries,
            cpu,
            held: tokio::sync::Mutex::new(None),
            deadline,
            admission_wait,
        };
        this.resume().await?;
        Ok(this)
    }
    async fn suspend(&self) {
        self.held.lock().await.take();
    }
    async fn resume(&self) -> Result<(), ModelError> {
        let permits = tokio::time::timeout_at(
            self.deadline
                .min(tokio::time::Instant::now() + self.admission_wait),
            async {
                let q = self.queries.clone().acquire_owned().await.map_err(|_| {
                    ModelError::Serving(lctx_model::domain::serving::FailureKind::Unavailable)
                })?;
                let c = self.cpu.clone().acquire_owned().await.map_err(|_| {
                    ModelError::Serving(lctx_model::domain::serving::FailureKind::Unavailable)
                })?;
                Ok::<_, ModelError>((q, c))
            },
        )
        .await
        .map_err(|_| {
            ModelError::Serving(lctx_model::domain::serving::FailureKind::ResourceRefused)
        })??;
        *self.held.lock().await = Some(permits);
        Ok(())
    }
}
pub(crate) struct Preparation<'a> {
    pub(crate) cache: &'a Arc<PreparedCache>,
    pub(crate) admission: &'a RequestAdmission,
}
impl Preparation<'_> {
    pub(crate) async fn hydrate(
        &self,
        reader: &lctx_surrealdb::NativeReader,
        roots: Vec<surrealdb::types::RecordId>,
        inputs: &[ValidationInput],
        incoming: &[ValidationInput],
        fields: &[&str],
    ) -> Result<PreparedBatches, ModelError> {
        if self.cache.pin != PreparationIdentity::Publication(reader.handle().clone()) {
            return Err(ModelError::Conflict("viewer preparation pin"));
        }
        self.hydrate_exact(reader, roots, inputs, incoming, fields)
            .await
    }
    #[cfg(test)]
    pub(crate) async fn hydrate_scope(
        &self,
        reader: &lctx_surrealdb::NativeReader<()>,
        roots: Vec<surrealdb::types::RecordId>,
        inputs: &[ValidationInput],
        incoming: &[ValidationInput],
        fields: &[&str],
    ) -> Result<PreparedBatches, ModelError> {
        if self.cache.pin != PreparationIdentity::CompilerViews(reader.view_bindings()) {
            return Err(ModelError::Conflict("compiler preparation exact views"));
        }
        self.hydrate_exact(reader, roots, inputs, incoming, fields)
            .await
    }
    async fn hydrate_exact<Context: Clone + Send + Sync + 'static>(
        &self,
        reader: &lctx_surrealdb::NativeReader<Context>,
        roots: Vec<surrealdb::types::RecordId>,
        inputs: &[ValidationInput],
        incoming: &[ValidationInput],
        fields: &[&str],
    ) -> Result<PreparedBatches, ModelError> {
        self.admission.suspend().await;
        let result = async {
            let layoutkey = self
                .cache
                .available_key("layout", &[], inputs, incoming, fields)
                .await?;
            let capture_charge = Arc::new(self.cache.reserve_capture(
                "viewer-preparation-layout-capture",
                1024 + (inputs.len() + incoming.len()) * 512
                    + fields.iter().map(|s| s.len() + 64).sum::<usize>(),
            )?);
            let owned = Arc::new((
                inputs.to_vec(),
                incoming.to_vec(),
                fields.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            ));
            let layout = self
                .cache
                .load(
                    layoutkey,
                    Box::new(move |budget| {
                        let owned = owned.clone();
                        let capture_charge = capture_charge.clone();
                        Box::pin(async move {
                            let _capture_charge = capture_charge;
                            let fields = owned.2.iter().map(String::as_str).collect::<Vec<_>>();
                            let program =
                                lctx_model::domain::serving_scope::ServingScopeProgram::new(
                                    &owned.0, &owned.1, &fields, &budget,
                                )?;
                            Ok(PreparedData::Layout(PreparedServingScope::new(
                                program, &budget,
                            )?))
                        })
                    }),
                )
                .await?;
            let key = self
                .cache
                .available_key("closure", &roots, inputs, incoming, fields)
                .await?;
            let reader = reader.clone();
            let capture_charge = Arc::new(self.cache.reserve_capture(
                "viewer-preparation-closure-capture",
                1024 + roots.len() * 384,
            )?);
            let roots = Arc::new(roots);
            let lifecycle = self.cache.lifecycle.clone();
            self.cache
                .load(
                    key,
                    Box::new(move |budget| {
                        let reader = reader.clone();
                        let roots = roots.clone();
                        let layout = layout.clone();
                        let capture_charge = capture_charge.clone();
                        let lifecycle = lifecycle.clone();
                        Box::pin(async move {
                            let _capture_charge = capture_charge;
                            let _root_copy = budget
                                .reserve("viewer-preparation-root-copy", roots.len() * 384)?;
                            let PreparedData::Layout(prepared) = &layout.data else {
                                return Err(ModelError::Schema("viewer preparation layout kind"));
                            };
                            let cancelled = || {
                                lifecycle
                                    .state
                                    .lock()
                                    .expect("preparation lifecycle")
                                    .closing
                            };
                            Ok(PreparedData::Batches(Arc::new(
                                crate::scope::hydrate_prepared_layout(
                                    &reader,
                                    roots.as_ref().clone(),
                                    prepared,
                                    &budget,
                                    Some(&cancelled),
                                )
                                .await?,
                            )))
                        })
                    }),
                )
                .await
                .map(PreparedBatches)
        }
        .await;
        // A request whose deadline expired while waiting fails independently of the initializer.
        let resumed = self.admission.resume().await;
        match result {
            Ok(v) => {
                resumed?;
                Ok(v)
            }
            Err(e) => Err(e),
        }
    }
}
pub(crate) struct PreparedBatches(Arc<ChargedPreparedValue>);
#[cfg(test)]
impl PreparedBatches {
    pub(crate) fn shares_value(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl std::ops::Deref for PreparedBatches {
    type Target = CanonicalBatches;
    fn deref(&self) -> &Self::Target {
        let PreparedData::Batches(b) = &self.0.data else {
            unreachable!("typed prepared batch owner")
        };
        b
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{
        ContentHash,
        serving::{DatabaseIdentity, Name},
        source,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    fn make_cache() -> (Arc<PreparedCache>, ResourceBudget) {
        let shared = ResourceBudget::fixed(1 << 20).unwrap();
        let limits = ResourceLimits {
            shared_bytes: 1 << 20,
            preparation_bytes: 1 << 19,
            request_bytes: 1 << 18,
            ..Default::default()
        };
        let pin = SnapshotHandle {
            publication: ContentHash::of(b"publication"),
            view: ContentHash::of(b"view"),
            service_generation: ContentHash::of(b"generation"),
            definition_epoch: ContentHash::of(b"epoch"),
            semantic: ContentHash::of(b"semantic"),
            realization: ContentHash::of(b"realization"),
            database: DatabaseIdentity {
                namespace: Name::new("ns").unwrap(),
                database: Name::new("snapshot").unwrap(),
            },
        };
        (
            PreparedCache::new(
                pin,
                &shared,
                &limits,
                Arc::new(Semaphore::new(1)),
                Arc::new(Semaphore::new(1)),
            )
            .unwrap(),
            shared,
        )
    }
    fn key(cache: &PreparedCache) -> ExactPreparedKey {
        cache.key("test", &[], &[], &[], &[]).unwrap()
    }
    #[test]
    fn exact_prepared_key_binds_physical_pin_ordered_roles_columns_and_closure() {
        let (cache, _) = make_cache();
        let inputs = [ValidationInput::of::<source::SourceArtifact>(&["id"])];
        let reordered = [ValidationInput::of::<source::SourceArtifact>(&[
            "input", "id",
        ])];
        let root = surrealdb::types::RecordId::new("entity", "a");
        let a = cache
            .key(
                "closure",
                std::slice::from_ref(&root),
                &inputs,
                &inputs,
                &["member"],
            )
            .unwrap();
        let same = cache
            .key(
                "closure",
                &[root.clone(), root.clone()],
                &inputs,
                &inputs,
                &["member"],
            )
            .unwrap();
        assert!(a == same);
        for other in [
            cache
                .key("closure", &[root.clone()], &reordered, &inputs, &["member"])
                .unwrap(),
            cache
                .key("closure", &[root.clone()], &inputs, &[], &["member"])
                .unwrap(),
            cache
                .key("closure", &[root.clone()], &inputs, &inputs, &["owner"])
                .unwrap(),
            cache
                .key("closure", &[], &inputs, &inputs, &["member"])
                .unwrap(),
            cache
                .key("layout", &[root], &inputs, &inputs, &["member"])
                .unwrap(),
        ] {
            assert!(a != other);
        }
        let (mut foreign, _) = make_cache();
        let PreparationIdentity::Publication(pin) = &mut Arc::get_mut(&mut foreign).unwrap().pin
        else {
            panic!("publication key")
        };
        pin.database.database = Name::new("other").unwrap();
        assert!(
            a != foreign
                .key(
                    "closure",
                    &[surrealdb::types::RecordId::new("entity", "a")],
                    &inputs,
                    &inputs,
                    &["member"]
                )
                .unwrap()
        );
    }
    #[tokio::test]
    async fn cancelled_request_does_not_cancel_coalesced_initializer_and_eviction_keeps_borrow() {
        let (cache, shared) = make_cache();
        let gate = Arc::new(Semaphore::new(0));
        let started = Arc::new(Notify::new());
        let count = Arc::new(AtomicUsize::new(0));
        let first = {
            let cache = cache.clone();
            let gate = gate.clone();
            let started = started.clone();
            let count = count.clone();
            tokio::spawn(async move {
                cache
                    .load(
                        key(&cache),
                        Box::new(move |budget| {
                            let gate = gate.clone();
                            let started = started.clone();
                            let count = count.clone();
                            Box::pin(async move {
                                count.fetch_add(1, Ordering::SeqCst);
                                let charge = budget.reserve("test-payload", 4096)?;
                                started.notify_one();
                                let _permit = gate.acquire().await.unwrap();
                                Ok(PreparedData::Test { _charge: charge })
                            })
                        }),
                    )
                    .await
            })
        };
        started.notified().await;
        first.abort();
        assert!(first.await.is_err());
        cache.release_optional_retention();
        let second = {
            let cache = cache.clone();
            let count = count.clone();
            tokio::spawn(async move {
                cache
                    .load(
                        key(&cache),
                        Box::new(move |budget| {
                            count.fetch_add(1, Ordering::SeqCst);
                            Box::pin(async move {
                                Ok(PreparedData::Test {
                                    _charge: budget.reserve("test-payload", 4096)?,
                                })
                            })
                        }),
                    )
                    .await
            })
        };
        while Arc::strong_count(
            cache
                .lifecycle
                .state
                .lock()
                .unwrap()
                .flights
                .values()
                .next()
                .unwrap(),
        ) < 3
        {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            cache.lifecycle.state.lock().unwrap().owners,
            1,
            "pressure preserves the one admitted loader"
        );
        gate.add_permits(1);
        let held = second.await.unwrap().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        cache.release_optional_retention();
        assert!(shared.reserved() > 0);
        assert_eq!(cache.lifecycle.state.lock().unwrap().values, 1);
        let closing = {
            let cache = cache.clone();
            tokio::spawn(async move { cache.close().await })
        };
        tokio::task::yield_now().await;
        assert!(!closing.is_finished());
        drop(held);
        closing.await.unwrap();
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[tokio::test]
    async fn failed_initialization_is_uncached_and_success_racing_retirement_drains() {
        let (cache, shared) = make_cache();
        let failed = cache
            .load(
                key(&cache),
                Box::new(|_| Box::pin(async { Err(ModelError::Schema("test failure")) })),
            )
            .await;
        assert!(matches!(
            failed.err().unwrap().primary(),
            Some(ModelError::Schema("test failure"))
        ));
        let gate = Arc::new(Semaphore::new(0));
        let started = Arc::new(Notify::new());
        let request = {
            let cache = cache.clone();
            let gate = gate.clone();
            let started = started.clone();
            tokio::spawn(async move {
                cache
                    .load(
                        key(&cache),
                        Box::new(move |budget| {
                            let gate = gate.clone();
                            let started = started.clone();
                            Box::pin(async move {
                                started.notify_one();
                                let _permit = gate.acquire().await.unwrap();
                                Ok(PreparedData::Test {
                                    _charge: budget.reserve("test-payload", 4096)?,
                                })
                            })
                        }),
                    )
                    .await
            })
        };
        started.notified().await;
        request.abort();
        let closing = {
            let cache = cache.clone();
            tokio::spawn(async move { cache.close().await })
        };
        while !cache.lifecycle.state.lock().unwrap().closing {
            tokio::task::yield_now().await;
        }
        assert!(cache.admit_request().is_err());
        assert!(
            cache
                .load(
                    key(&cache),
                    Box::new(|budget| Box::pin(async move {
                        Ok(PreparedData::Test {
                            _charge: budget.reserve("test-payload", 4096)?,
                        })
                    }))
                )
                .await
                .is_err()
        );
        gate.add_permits(1);
        closing.await.unwrap();
        assert_eq!(cache.lifecycle.state.lock().unwrap().values, 0);
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[test]
    fn weight_units_use_checked_ceiling_without_truncating_large_payloads() {
        assert_eq!(weight_units(1), Some(1));
        assert_eq!(weight_units(1024), Some(1));
        assert_eq!(weight_units(1025), Some(2));
        assert_eq!(weight_units(usize::MAX), None);
    }
    #[tokio::test]
    async fn stale_retention_miss_rechecks_completed_publication_before_installing() {
        let (cache, shared) = make_cache();
        let reached = Arc::new(Notify::new());
        let gate = Arc::new(Semaphore::new(0));
        *cache.after_miss.lock().unwrap() = Some((reached.clone(), gate.clone()));
        let count = Arc::new(AtomicUsize::new(0));
        let stale = {
            let cache = cache.clone();
            let count = count.clone();
            tokio::spawn(async move {
                cache
                    .load(
                        key(&cache),
                        Box::new(move |budget| {
                            count.fetch_add(1, Ordering::SeqCst);
                            Box::pin(async move {
                                Ok(PreparedData::Test {
                                    _charge: budget.reserve("stale", 4096)?,
                                })
                            })
                        }),
                    )
                    .await
            })
        };
        reached.notified().await;
        let completed = cache
            .load(
                key(&cache),
                Box::new({
                    let count = count.clone();
                    move |budget| {
                        count.fetch_add(1, Ordering::SeqCst);
                        Box::pin(async move {
                            Ok(PreparedData::Test {
                                _charge: budget.reserve("completed", 4096)?,
                            })
                        })
                    }
                }),
            )
            .await
            .unwrap();
        while cache.lifecycle.state.lock().unwrap().owners != 0 {
            tokio::task::yield_now().await;
        }
        assert!(cache.lifecycle.state.lock().unwrap().flights.is_empty());
        gate.add_permits(1);
        let reused = stale.await.unwrap().unwrap();
        assert!(Arc::ptr_eq(&completed, &reused));
        assert_eq!(count.load(Ordering::SeqCst), 1);
        drop(completed);
        drop(reused);
        cache.close().await;
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[tokio::test]
    async fn stale_owner_cannot_remove_a_replacement_flight() {
        let (cache, shared) = make_cache();
        let exact = key(&cache);
        let flight = || {
            Arc::new(Flight {
                terminal: Mutex::new(None),
                changed: Notify::new(),
                _charge: cache.budget.reserve("test-flight", 4096).unwrap(),
            })
        };
        let old = flight();
        let current = flight();
        {
            let mut s = cache.lifecycle.state.lock().unwrap();
            s.owners = 1;
            s.flights.insert(exact.clone(), current.clone());
        }
        drop(InsertionOwner {
            lifecycle: cache.lifecycle.clone(),
            key: exact.clone(),
            flight: old.clone(),
        });
        assert!(old.terminal.lock().unwrap().as_ref().unwrap().is_err());
        {
            let s = cache.lifecycle.state.lock().unwrap();
            assert!(Arc::ptr_eq(s.flights.get(&exact).unwrap(), &current));
            assert_eq!(s.publication, 0);
        }
        let removed = cache.lifecycle.state.lock().unwrap().flights.remove(&exact);
        drop(removed);
        drop(old);
        drop(current);
        drop(exact);
        cache.close().await;
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[tokio::test]
    async fn close_fences_resource_retry_and_queued_loader_admission() {
        let (cache, shared) = make_cache();
        let gate = Arc::new(Semaphore::new(0));
        let started = Arc::new(Notify::new());
        let count = Arc::new(AtomicUsize::new(0));
        let request = {
            let cache = cache.clone();
            let gate = gate.clone();
            let started = started.clone();
            let count = count.clone();
            tokio::spawn(async move {
                cache
                    .load(
                        key(&cache),
                        Box::new(move |budget| {
                            let gate = gate.clone();
                            let started = started.clone();
                            let count = count.clone();
                            Box::pin(async move {
                                count.fetch_add(1, Ordering::SeqCst);
                                started.notify_one();
                                let _permit = gate.acquire().await.unwrap();
                                Ok(PreparedData::Test {
                                    _charge: budget
                                        .reserve("resource-refusal", budget.limit() + 1)?,
                                })
                            })
                        }),
                    )
                    .await
            })
        };
        started.notified().await;
        let queued = {
            let cache = cache.clone();
            tokio::spawn(async move {
                cache
                    .load(
                        cache.key("queued", &[], &[], &[], &[]).unwrap(),
                        Box::new(|_| {
                            Box::pin(async { panic!("closed queued loader must not initialize") })
                        }),
                    )
                    .await
            })
        };
        while cache.lifecycle.state.lock().unwrap().owners != 2 {
            tokio::task::yield_now().await;
        }
        let closing = {
            let cache = cache.clone();
            tokio::spawn(async move { cache.close().await })
        };
        while !cache.lifecycle.state.lock().unwrap().closing {
            tokio::task::yield_now().await;
        }
        gate.add_permits(1);
        for result in [request.await.unwrap(), queued.await.unwrap()] {
            assert!(matches!(
                result.err().unwrap().primary(),
                Some(ModelError::Serving(
                    lctx_model::domain::serving::FailureKind::Unavailable
                ))
            ));
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
        closing.await.unwrap();
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[tokio::test]
    async fn close_fences_a_suspended_retention_miss() {
        let (cache, shared) = make_cache();
        let reached = Arc::new(Notify::new());
        let gate = Arc::new(Semaphore::new(0));
        *cache.after_miss.lock().unwrap() = Some((reached.clone(), gate.clone()));
        let request = {
            let cache = cache.clone();
            let lease = cache.admit_request().unwrap();
            tokio::spawn(async move {
                let _lease = lease;
                cache
                    .load(
                        key(&cache),
                        Box::new(|_| Box::pin(async { panic!("closed miss must not initialize") })),
                    )
                    .await
            })
        };
        reached.notified().await;
        let closing = {
            let cache = cache.clone();
            tokio::spawn(async move { cache.close().await })
        };
        while !cache.lifecycle.state.lock().unwrap().closing {
            tokio::task::yield_now().await;
        }
        assert!(!closing.is_finished());
        gate.add_permits(1);
        assert!(matches!(
            request.await.unwrap().err().unwrap().primary(),
            Some(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::Unavailable
            ))
        ));
        closing.await.unwrap();
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[tokio::test]
    async fn optional_retention_yields_to_required_preparation_without_releasing_borrowers() {
        let (cache, shared) = make_cache();
        let first = cache
            .load(
                key(&cache),
                Box::new(|budget| {
                    Box::pin(async move {
                        Ok(PreparedData::Test {
                            _charge: budget.reserve("test-payload", 256 * 1024)?,
                        })
                    })
                }),
            )
            .await
            .unwrap();
        drop(first);
        let count = Arc::new(AtomicUsize::new(0));
        let secondkey = cache.key("other", &[], &[], &[], &[]).unwrap();
        let second = cache
            .load(
                secondkey,
                Box::new({
                    let count = count.clone();
                    move |budget| {
                        count.fetch_add(1, Ordering::SeqCst);
                        Box::pin(async move {
                            Ok(PreparedData::Test {
                                _charge: budget.reserve("test-payload", 300 * 1024)?,
                            })
                        })
                    }
                }),
            )
            .await
            .unwrap();
        assert_eq!(
            count.load(Ordering::SeqCst),
            2,
            "exact resource refusal evicts only optional cache retention and retries"
        );
        assert!(shared.reserved() >= 300 * 1024);
        drop(second);
        cache.close().await;
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[tokio::test]
    async fn suspended_request_releases_the_only_initializer_admission_slot() {
        let (cache, _) = make_cache();
        let admission = RequestAdmission::new(
            cache.queries.clone(),
            cache.cpu.clone(),
            tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        )
        .await
        .unwrap();
        assert_eq!(cache.queries.available_permits(), 0);
        admission.suspend().await;
        let value = cache
            .load(
                key(&cache),
                Box::new(|budget| {
                    Box::pin(async move {
                        Ok(PreparedData::Test {
                            _charge: budget.reserve("test-payload", 4096)?,
                        })
                    })
                }),
            )
            .await
            .unwrap();
        admission.resume().await.unwrap();
        assert_eq!(cache.queries.available_permits(), 0);
        drop(value);
        drop(admission);
        cache.close().await;
    }
    #[tokio::test]
    async fn preloader_capture_evicts_optional_retention_before_refusing_fresh_work() {
        let (cache, shared) = make_cache();
        let old = cache
            .load(
                key(&cache),
                Box::new(|budget| {
                    Box::pin(async move {
                        Ok(PreparedData::Test {
                            _charge: budget.reserve("old", 200 * 1024)?,
                        })
                    })
                }),
            )
            .await
            .unwrap();
        drop(old);
        let pressure = cache
            .budget
            .reserve(
                "required-live-consumer",
                cache.budget.limit() - cache.budget.reserved() - 2048,
            )
            .unwrap();
        let capture = cache
            .reserve_capture("viewer-preparation-closure-capture", 8192)
            .unwrap();
        assert_eq!(
            cache.lifecycle.state.lock().unwrap().values,
            0,
            "capture admission releases only optional retention"
        );
        drop(capture);
        drop(pressure);
        cache.close().await;
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
    #[tokio::test]
    async fn insertion_admission_evicts_optional_retention_before_refusing_fresh_work() {
        let (cache, shared) = make_cache();
        let old = cache
            .load(
                key(&cache),
                Box::new(|budget| {
                    Box::pin(async move {
                        Ok(PreparedData::Test {
                            _charge: budget.reserve("old", 200 * 1024)?,
                        })
                    })
                }),
            )
            .await
            .unwrap();
        drop(old);
        let next = cache.key("new", &[], &[], &[], &[]).unwrap();
        let pressure = cache
            .budget
            .reserve(
                "required-live-consumer",
                cache.budget.limit() - cache.budget.reserved() - 2048,
            )
            .unwrap();
        let fresh = cache
            .load(
                next,
                Box::new(|budget| {
                    Box::pin(async move {
                        Ok(PreparedData::Test {
                            _charge: budget.reserve("fresh", 64)?,
                        })
                    })
                }),
            )
            .await
            .unwrap();
        assert_eq!(
            cache.lifecycle.state.lock().unwrap().values,
            1,
            "unborrowed old value was released for insertion admission"
        );
        drop(fresh);
        drop(pressure);
        cache.close().await;
        drop(cache);
        assert_eq!(shared.reserved(), 0);
    }
}
