//! Attempt-scoped native compiler authority in one durable shared database. Completed memberships, rather than a publication handle,
//! govern reads. Canonical graph payloads and query fields are immutable mechanical forms.
use lctx_native::completed_rows::{
    OriginalBacking, admit_backing_row, backing_integer, binding_projection, check_projection,
    contribution_projection, decode_backing_id, decode_nominal_key, membership_id,
    require_backing_fields, validate_backing_batch, validate_compiler_backing, validate_state_row,
    view_projection,
};
pub use lctx_native::installation::*;
pub use lctx_native::recovery::*;
mod ingress;
use crate::{
    Loader, RuntimeConfig,
    adapter::GraphRow,
    reader::{self, NativeRows},
};
use crate::{
    acknowledged_candidates::{
        AsyncCandidateSort, AsyncOrderedCandidates, AsyncOrderedRows, AsyncPhysicalSort,
        BlockingTerminal,
    },
    ordered_rows::{Candidate, PreparedCandidates, PreparedRows},
};
use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use d::resources::{Reservation, ResourceBudget};
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
pub use ingress::{IngressFailure, IngressSequence, NativeIngress, WriteAcknowledgement};
use lctx_model::domain::{
    self as d, admission::Frontier, completed::*, completed_transport::*, stages::ProviderOutcome,
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{
        Bytes, Object, RecordId, RecordIdKey, SerdeWrapper, SurrealValue, ToSql, Value, Variables,
    },
};
use tokio::sync::Notify;
use tracing::{Instrument, instrument::WithSubscriber};

#[derive(Clone)]
pub enum NativePredicate {
    Keys(Vec<[u8; 16]>),
    KeysSql {
        keys: Vec<[u8; 16]>,
        sql: String,
        bindings: Variables,
        preparation: Vec<String>,
    },
    FieldSql {
        field: String,
        values: Vec<Value>,
        sql: String,
        bindings: Variables,
        preparation: Vec<String>,
    },
    Field {
        field: String,
        values: Vec<Value>,
    },
    /// SQL comes only from the finite operation/physical planner; all values remain bindings.
    Sql {
        sql: String,
        bindings: Variables,
        preparation: Vec<String>,
    },
}
fn read_predicate_bytes(predicate: &NativePredicate) -> usize {
    let values = |values: &Vec<Value>| {
        values.iter().map(crate::loader::native_bytes).fold(
            values
                .capacity()
                .saturating_sub(values.len())
                .saturating_mul(size_of::<Value>()),
            usize::saturating_add,
        )
    };
    let sql = |sql: &String, bindings: &Variables, preparation: &Vec<String>| {
        sql.capacity()
            .saturating_add(
                bindings
                    .iter()
                    .map(|(key, value)| {
                        key.len()
                            .saturating_add(64)
                            .saturating_add(crate::loader::native_bytes(value))
                    })
                    .fold(0, usize::saturating_add),
            )
            .saturating_add(
                preparation
                    .iter()
                    .map(|statement| size_of::<String>().saturating_add(statement.capacity()))
                    .fold(0, usize::saturating_add),
            )
    };
    match predicate {
        NativePredicate::Keys(keys) => keys.capacity().saturating_mul(size_of::<[u8; 16]>()),
        NativePredicate::KeysSql {
            keys,
            sql: query,
            bindings,
            preparation,
        } => keys
            .capacity()
            .saturating_mul(size_of::<[u8; 16]>())
            .saturating_add(sql(query, bindings, preparation)),
        NativePredicate::Field {
            field,
            values: bound,
        } => field.capacity().saturating_add(values(bound)),
        NativePredicate::FieldSql {
            field,
            values: bound,
            sql: query,
            bindings,
            preparation,
        } => field
            .capacity()
            .saturating_add(values(bound))
            .saturating_add(sql(query, bindings, preparation)),
        NativePredicate::Sql {
            sql: query,
            bindings,
            preparation,
        } => sql(query, bindings, preparation),
    }
}

/// Readiness of this attempt's checked contribution attachment. Registration alone
/// grants no producer or completion authority; imported descriptors stay registering
/// until their independent admission succeeds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ContributionOwnership {
    Registering,
    Producing,
    Completed,
}
#[derive(Clone)]
struct RegisteredContribution {
    spec: ContributionSpec,
    ownership: ContributionOwnership,
    _transport_charge: Option<Arc<Box<dyn Reservation>>>,
}
impl RegisteredContribution {
    fn checked(&self) -> Result<(&ContributionSpec, bool), ModelError> {
        match self.ownership {
            ContributionOwnership::Registering => Err(ModelError::Conflict(
                "native contribution ownership not ready",
            )),
            ContributionOwnership::Producing => Ok((&self.spec, false)),
            ContributionOwnership::Completed => Ok((&self.spec, true)),
        }
    }
}

pub struct NativeCompilerStore {
    pub(crate) client: Arc<Surreal<Client>>,
    database: d::serving::Name,
    attempt: ContentHash,
    generation: ContentHash,
    published_bindings: Mutex<Option<Vec<CompletedBinding>>>,
    published_completion: Mutex<Option<lctx_native::completed_state::PublishedCompletedState>>,
    physical_completed: Mutex<Option<Arc<lctx_native::completed_state::NativeCompletedState>>>,
    read_only: AtomicBool,
    read_cancellation: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    signal_stop: crate::control::RetryCancellation,
    signals: Result<lctx_native::signal::SignalRegistration, Arc<ModelError>>,
    owns_session: bool,
    original_ingress: lctx_native::originals::PendingOriginals,
    namespace: d::serving::Name,
    endpoint: Option<String>,
    specifications: Mutex<BTreeMap<ContentHash, RegisteredContribution>>,
    known_views: Mutex<BTreeMap<ContentHash, CompletedView>>,
    view_mappings: Mutex<
        BTreeMap<
            ContentHash,
            (
                Vec<lctx_native::installed_view::ContributorRealization>,
                Arc<Box<dyn Reservation>>,
            ),
        >,
    >,
    installed_views:
        Mutex<BTreeMap<ContentHash, Arc<lctx_native::installed_view::HeldInstalledView>>>,
    view_preparations: Mutex<BTreeMap<ContentHash, ViewPreparation>>,
    product_retention_gates:
        Mutex<BTreeMap<ContentHash, (Arc<tokio::sync::Mutex<()>>, Arc<Box<dyn Reservation>>)>>,
    view_audit: tokio::sync::Mutex<Option<lctx_native::installed_view::InstallationAudit>>,
    known_contributors: Mutex<BTreeMap<ContentHash, ContentHash>>,
    failed: AtomicBool,
    sealed: AtomicBool,
    seal_started: AtomicBool,
    preparation: Mutex<Option<CanonicalPreparation>>,
    membership_preparations: Mutex<
        BTreeMap<
            ContentHash,
            (
                MembershipPreparation,
                Arc<Box<dyn Reservation>>,
                ResourceBudget,
            ),
        >,
    >,
    producing_scopes: Mutex<BTreeMap<ContentHash, Arc<ProducingAdmission>>>,
    admission: Arc<OperationAdmission>,
    runtime: tokio::runtime::Handle,
    frontier: Mutex<Frontier>,
}
type PreparedView = Shared<
    BoxFuture<
        'static,
        Result<Arc<lctx_native::installed_view::HeldInstalledView>, Arc<ModelError>>,
    >,
>;
struct ViewPreparation {
    pending: PreparedView,
    token: Arc<Box<dyn Reservation>>,
    budget: ResourceBudget,
}
/// An exact held native candidate. Inspection holds no receiver view/binding and
/// grants no producer authority; attachment consumes it after receiver preflight.
pub struct HeldCandidate {
    owner: Arc<OperationAdmission>,
    physical: ContentHash,
    descriptor: CompletedContribution,
    expected: crate::control::ExpectedRecord,
    completed: Arc<CandidateCompletion>,
    outputs: BTreeMap<String, Arc<lctx_native::installed_view::HeldInstalledView>>,
    _admission: Option<crate::admission::candidate::CandidateAdmission>,
    _charge: Box<dyn Reservation>,
}
pub(crate) enum CandidateCompletion {
    Admitted(Arc<lctx_native::completed_state::HeldFrozenCompletedState>),
    Terminal(Arc<lctx_native::control::HeldTerminalProduct>),
}
impl CandidateCompletion {
    pub(crate) fn retain_unknown(&self) {
        if let Self::Terminal(owner) = self {
            owner.retain_unknown();
        }
    }
    async fn close(self: Arc<Self>) -> Result<(), ModelError> {
        match Arc::try_unwrap(self) {
            Ok(Self::Terminal(owner)) => owner.close().await,
            Ok(Self::Admitted(_)) => Ok(()),
            Err(owner) => {
                owner.retain_unknown();
                let completion = d::completion::Completion {
                    local: d::completion::LocalState::Outstanding,
                    ..d::completion::Completion::default()
                };
                d::completion::complete(
                    Err(ModelError::Conflict(
                        "candidate completion still has live read owners",
                    )),
                    completion,
                )
            }
        }
    }
    async fn held_view(
        &self,
        client: Arc<Surreal<Client>>,
        view: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<Arc<lctx_native::installed_view::HeldInstalledView>, ModelError> {
        match self {
            Self::Admitted(owner) => Ok(Arc::new(
                lctx_native::installed_view::hold_frozen_view(client, owner.clone(), view, budget)
                    .await?,
            )),
            Self::Terminal(owner) => {
                owner.ensure_current(budget).await?;
                owner
                    .views()
                    .get(&view)
                    .cloned()
                    .ok_or(ModelError::Conflict(
                        "terminal candidate exact selected view",
                    ))
            }
        }
    }
}
impl HeldCandidate {
    pub fn physical(&self) -> ContentHash {
        self.physical
    }
    pub fn descriptor(&self) -> &CompletedContribution {
        &self.descriptor
    }
    /// Explicitly join a candidate rejected by its caller after all scans drain.
    pub async fn close(self) -> Result<(), ModelError> {
        let Self {
            completed, outputs, ..
        } = self;
        drop(outputs);
        completed.close().await
    }
}
// Request discovery commits the entire specification. Equality is eligibility
// data only; exact native completion/incarnation acquisition still follows.
fn require_retained_request_spec(
    actual: &ContributionSpec,
    requested: &ContributionSpec,
) -> Result<(), ModelError> {
    if actual != requested {
        return Err(ModelError::Conflict(
            "equal request with inconsistent contribution specification",
        ));
    }
    Ok(())
}
pub struct FinalizationInventory {
    pub contributions: Vec<CompletedContribution>,
    pub bindings: Vec<CompletedBinding>,
}
/// One operation's exact binding inventory and immutable completed dependency closure.
pub struct AuditCapture {
    owner: Arc<OperationAdmission>,
    bindings: Vec<CompletedBinding>,
    variables: Variables,
    closure: d::recovery_closure::RecoveryClosure,
    payloads: crate::selection::SelectedPayloads,
    budget: ResourceBudget,
    _closure_charge: d::charged::StateCharge,
}
#[derive(Clone)]
struct PreparedCanonical {
    entities: PreparedRows,
    assertions: PreparedRows,
}
type CanonicalPreparation = Shared<BoxFuture<'static, Result<PreparedCanonical, Arc<ModelError>>>>;
type MembershipPreparation =
    Shared<BoxFuture<'static, Result<PreparedCandidates, Arc<ModelError>>>>;
#[derive(Default)]
struct OperationState {
    closed: bool,
    content_closed: bool,
    content_ready: bool,
    mutations: usize,
    active: usize,
    scans: usize,
    failed: bool,
    uncertain: bool,
    failures: Vec<NativeFailure>,
    committed: Option<d::completion::CommittedEffect>,
}
struct NativeFailure {
    operation: &'static str,
    error: Arc<ModelError>,
}
impl OperationState {
    fn refusal(&self, detail: &'static str) -> ModelError {
        let refusal = ModelError::infrastructure(d::Infrastructure::State, detail);
        let mut completion = d::completion::Completion::default();
        completion.committed.extend(self.committed.clone());
        let Some(first) = self.failures.first() else {
            return d::completion::complete::<()>(Err(refusal), completion).unwrap_err();
        };
        if self.active != 0 {
            completion.local = d::completion::LocalState::Outstanding;
        }
        if self.uncertain {
            completion.remote = d::completion::RemoteState::Unknown;
        }
        completion.step(
            "native operation admission",
            Err(ModelError::infrastructure(
                d::Infrastructure::State,
                format!("{detail}; first failed operation: {}", first.operation),
            )),
        );
        d::completion::complete::<()>(
            Err(ModelError::SharedCause(first.error.clone())),
            completion,
        )
        .unwrap_err()
    }
}
#[derive(Default)]
struct OperationAdmission {
    state: Mutex<OperationState>,
    changed: Notify,
    source_finalization: tokio::sync::Mutex<SourceFinalization>,
}
#[derive(Default)]
struct SourceFinalization {
    attempt_closed: bool,
    session_closed: bool,
}
#[derive(Default)]
struct ProducingState {
    closed: bool,
    active: usize,
}
struct ProducingAdmission {
    state: Mutex<ProducingState>,
    changed: Notify,
    _retained: Box<dyn Reservation>,
}
#[derive(Clone)]
struct ProducingBinding {
    owner: Arc<OperationAdmission>,
    scope: Arc<ProducingAdmission>,
}
#[derive(Clone)]
struct DescendantBinding {
    owner: Arc<OperationAdmission>,
    live: Arc<AtomicBool>,
}
tokio::task_local! {
    static PRODUCING_WORK: ProducingBinding;
    static DESCENDANT_WORK: DescendantBinding;
    static WRITING_CONTRIBUTION: ContentHash;
}
/// Native-owned producing work for exactly one store and contribution. This is an
/// execution lifetime, not semantic membership or a transferable completion assertion.
#[derive(Clone)]
pub struct ProducingScope {
    store: Arc<NativeCompilerStore>,
    contribution: ContentHash,
    scope: Arc<ProducingAdmission>,
}
struct ProducingLease(Arc<ProducingAdmission>);
impl ProducingAdmission {
    fn admit(self: &Arc<Self>, descendant: bool) -> Result<ProducingLease, ModelError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native producing scope"))?;
        if state.closed && (!descendant || state.active == 0) {
            return Err(ModelError::Conflict("native producing scope closed"));
        }
        state.active = state
            .active
            .checked_add(1)
            .ok_or(ModelError::Schema("native producing count"))?;
        Ok(ProducingLease(self.clone()))
    }
}
impl Drop for ProducingLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.active -= 1;
        }
        self.0.changed.notify_waiters();
    }
}
impl ProducingScope {
    pub fn belongs_to(&self, store: &Arc<NativeCompilerStore>) -> bool {
        Arc::ptr_eq(&self.store, store)
    }
    fn binding(&self) -> ProducingBinding {
        ProducingBinding {
            owner: self.store.admission.clone(),
            scope: self.scope.clone(),
        }
    }
    fn admitted_descendant(&self) -> bool {
        self.store.has_live_parent()
            && PRODUCING_WORK
                .try_with(|binding| {
                    Arc::ptr_eq(&binding.owner, &self.store.admission)
                        && Arc::ptr_eq(&binding.scope, &self.scope)
                })
                .unwrap_or(false)
    }
    /// Bind a native launch explicitly. Native descendant drivers capture this binding;
    /// Tokio task inheritance is never relied on.
    pub async fn run<T>(
        &self,
        future: impl std::future::Future<Output = Result<T, ModelError>>,
    ) -> Result<T, ModelError> {
        let descendant = self.admitted_descendant();
        let _lease = self.scope.admit(descendant)?;
        let global = BlockingLease(Some(if descendant {
            self.store.admit(false, "native producing work", true)?
        } else {
            self.store
                .admit_root(false, "native producing work", true)?
        }));
        let owner = global.0.as_ref().expect("admitted producing root");
        let result = owner
            .within(PRODUCING_WORK.scope(self.binding(), future))
            .await;
        owner.observe_result(result)
    }
    pub(crate) fn bind_stream<S: futures::Stream + Unpin>(
        &self,
        stream: S,
    ) -> Result<ProducingStream<S>, ModelError> {
        let descendant = self.admitted_descendant();
        let lease = self.scope.admit(descendant)?;
        let global = BlockingLease(Some(if descendant {
            self.store.admit(true, "native producing stream", true)?
        } else {
            self.store
                .admit_root(true, "native producing stream", true)?
        }));
        let descendant = global
            .0
            .as_ref()
            .expect("admitted producing stream")
            .binding();
        Ok(ProducingStream {
            stream,
            binding: self.binding(),
            descendant,
            lease: Some(lease),
            global: Some(global),
        })
    }
    pub async fn close_and_wait(&self) -> Result<(), ModelError> {
        self.scope
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native producing scope"))?
            .closed = true;
        loop {
            let changed = self.scope.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self
                .scope
                .state
                .lock()
                .map_err(|_| ModelError::Conflict("native producing scope"))?
                .active
                == 0
            {
                return self.store.check_failed();
            }
            changed.await;
        }
    }
}
pub(crate) struct ProducingStream<S> {
    stream: S,
    binding: ProducingBinding,
    descendant: DescendantBinding,
    lease: Option<ProducingLease>,
    global: Option<BlockingLease>,
}
impl<S: futures::Stream + Unpin> futures::Stream for ProducingStream<S> {
    type Item = S::Item;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let binding = self.binding.clone();
        let descendant = self.descendant.clone();
        let result = DESCENDANT_WORK.sync_scope(descendant, || {
            PRODUCING_WORK.sync_scope(binding, || {
                std::pin::Pin::new(&mut self.stream).poll_next(cx)
            })
        });
        if matches!(&result, std::task::Poll::Ready(None)) {
            self.lease.take();
            self.global.take();
        }
        result
    }
}
async fn producing_future<T>(
    binding: Option<ProducingBinding>,
    future: impl std::future::Future<Output = T>,
) -> T {
    match binding {
        Some(binding) => PRODUCING_WORK.scope(binding, future).await,
        None => future.await,
    }
}
/// Mechanical operation lifetime; grants no administrator or publication API.
pub struct OperationLease {
    owner: Arc<OperationAdmission>,
    scan: bool,
    mutation: bool,
    finished: bool,
    operation: &'static str,
    poison_on_error: bool,
    finalization: bool,
    _producing: Option<ProducingLease>,
    live: Arc<AtomicBool>,
}
struct ReadSetupResult {
    result: Option<Result<CompilerRows, ModelError>>,
    lease: Option<OperationLease>,
    store: Arc<NativeCompilerStore>,
    operation: &'static str,
}
impl ReadSetupResult {
    fn take(mut self) -> Result<CompilerRows, ModelError> {
        self.result.take().expect("owned read setup result")
    }
}
impl Drop for ReadSetupResult {
    fn drop(&mut self) {
        if let Some(result) = self.result.take() {
            match result {
                Ok(rows) => drop(rows),
                Err(error) => {
                    let error = match error {
                        ModelError::SharedCause(error) => error,
                        other => Arc::new(other),
                    };
                    self.store.fail();
                    if let Ok(mut state) = self.store.admission.state.lock() {
                        state.failed = true;
                        state.failures.push(NativeFailure {
                            operation: self.operation,
                            error,
                        });
                    }
                }
            }
        }
        if let Some(lease) = self.lease.take() {
            lease.finish();
        }
    }
}
impl OperationLease {
    fn binding(&self) -> DescendantBinding {
        DescendantBinding {
            owner: self.owner.clone(),
            live: self.live.clone(),
        }
    }
    async fn within<T>(&self, future: impl std::future::Future<Output = T>) -> T {
        DESCENDANT_WORK.scope(self.binding(), future).await
    }
    fn finish(mut self) {
        self.finished = true;
    }
    fn observe_result<T>(&self, result: Result<T, ModelError>) -> Result<T, ModelError> {
        result.map_err(|error| {
            let uncertain = !error.permits_storage_cleanup();
            if self.poison_on_error || uncertain {
                let error = match error {
                    ModelError::SharedCause(error) => error,
                    other => Arc::new(other),
                };
                if let Ok(mut state) = self.owner.state.lock() {
                    state.failed = true;
                    state.uncertain |= uncertain;
                    state.failures.push(NativeFailure {
                        operation: self.operation,
                        error: error.clone(),
                    });
                }
                ModelError::SharedCause(error)
            } else {
                error
            }
        })
    }
    pub fn record_committed(&self, identity: String) {
        // Marker acknowledgement is retained before another await can cancel the caller.
        let mut state = self
            .owner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.committed = Some(d::completion::CommittedEffect {
            kind: "published unselected manifest",
            identity,
        });
    }
    pub fn finish_with<T>(self, result: Result<T, ModelError>) -> Result<T, ModelError> {
        let result = self.observe_result(result);
        self.finish();
        result
    }
}
// A blocking worker owns a local computation, not a remote acknowledgement. Its
// lease remains live until the worker really exits, including cancellation/drop.
struct BlockingLease(Option<OperationLease>);
impl Drop for BlockingLease {
    fn drop(&mut self) {
        if let Some(lease) = self.0.take() {
            lease.finish();
        }
    }
}
impl Drop for OperationLease {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
        if let Ok(mut state) = self.owner.state.lock() {
            state.active -= 1;
            if self.mutation {
                state.mutations -= 1;
            }
            if self.scan {
                state.scans -= 1;
            }
            if !self.finished {
                state.failed = true;
                state.uncertain = true;
                state.failures.push(NativeFailure {
                    operation: self.operation,
                    error: Arc::new(ModelError::infrastructure(
                        d::Infrastructure::Unconfirmed,
                        format!("{} ended before acknowledgement", self.operation),
                    )),
                });
            }
        }
        self.owner.changed.notify_waiters();
    }
}
impl std::fmt::Debug for NativeCompilerStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeCompilerStore")
            .field("database", &self.database)
            .finish_non_exhaustive()
    }
}

fn native_signal_owners() -> (
    crate::control::RetryCancellation,
    Arc<Mutex<Option<Arc<AtomicBool>>>>,
    Result<lctx_native::signal::SignalRegistration, Arc<ModelError>>,
) {
    let cancellation = crate::control::RetryCancellation::default();
    let reads = Arc::new(Mutex::new(Some(Arc::new(AtomicBool::new(false)))));
    let token = cancellation.clone();
    let read_owner = reads.clone();
    let signals =
        lctx_native::signal::SignalRegistration::new("native compiler store", move || {
            token.cancel();
            if let Ok(flag) = read_owner.lock() {
                if let Some(flag) = &*flag {
                    flag.store(true, Ordering::Release);
                }
            }
        })
        .map_err(Arc::new);
    (cancellation, reads, signals)
}
impl NativeCompilerStore {
    /// Read-only exact publication context. This imports no runtime attempt ownership and
    /// creates no database or schema; pins belong to the published reader owner.
    pub async fn from_publication(
        client: Arc<Surreal<Client>>,
        namespace: d::serving::Name,
        database: d::serving::Name,
        bindings: Vec<CompletedBinding>,
        budget: &ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let store = Self::publication_owner(client, namespace, database, bindings)?;
        // The owner exists before the first asynchronous preparation. A failed constructor
        // joins all admitted tails instead of losing the only route to their terminal report.
        let result = store.capture_audit(budget).await.map(|_| store.clone());
        if result.is_err() {
            return d::completion::complete(result, store.drain_report().await);
        }
        result
    }
    /// Synchronous read ownership, deliberately without native attempt/maintenance authority.
    /// Callers must drain this owner before releasing their publication pin or session.
    pub fn publication_owner(
        client: Arc<Surreal<Client>>,
        namespace: d::serving::Name,
        database: d::serving::Name,
        bindings: Vec<CompletedBinding>,
    ) -> Result<Arc<Self>, ModelError> {
        binding_inventory_identity(&bindings)?;
        let store = Self::from_existing(client, namespace, database);
        store.read_only.store(true, Ordering::Release);
        *store
            .published_bindings
            .lock()
            .map_err(|_| ModelError::Conflict("published bindings"))? = Some(bindings);
        Ok(store)
    }
    pub fn set_read_cancellation(&self, flag: Arc<AtomicBool>) -> Result<(), ModelError> {
        if self.signal_stop.stopped() {
            flag.store(true, Ordering::Release);
        }
        if !self.read_only.load(Ordering::Acquire) {
            return Err(ModelError::Conflict("cancellation requires read owner"));
        }
        *self
            .read_cancellation
            .lock()
            .map_err(|_| ModelError::Conflict("read cancellation owner"))? = Some(flag);
        Ok(())
    }
    fn read_cancellation(&self) -> Result<Option<Arc<AtomicBool>>, ModelError> {
        Ok(self
            .read_cancellation
            .lock()
            .map_err(|_| ModelError::Conflict("read cancellation owner"))?
            .clone())
    }
    fn check_read_admission(&self) -> Result<(), ModelError> {
        self.signals
            .as_ref()
            .map_err(|error| ModelError::SharedCause(error.clone()))?;
        if self.signal_stop.stopped() {
            return Err(ModelError::Invalid("native operation cancelled".into()));
        }
        crate::prepared::check_read_cancellation(self.read_cancellation()?.as_ref())
    }
    fn read_client(&self) -> Result<&Surreal<Client>, ModelError> {
        self.check_read_admission()?;
        Ok(&self.client)
    }
    /// State transport is the dependency closure of the binding inventory, rather than
    /// every intermediate view observed while the attempt was being constructed.
    async fn state_closure(
        self: &Arc<Self>,
    ) -> Result<
        (
            BTreeMap<ContentHash, CompletedView>,
            std::collections::BTreeSet<ContentHash>,
        ),
        ModelError,
    > {
        self.state_closure_from(&self.bindings_inner().await?).await
    }
    async fn state_closure_from(
        self: &Arc<Self>,
        bindings: &[CompletedBinding],
    ) -> Result<
        (
            BTreeMap<ContentHash, CompletedView>,
            std::collections::BTreeSet<ContentHash>,
        ),
        ModelError,
    > {
        let mut views = bindings
            .iter()
            .map(|binding| (binding.view.identity, binding.view.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut owners = std::collections::BTreeSet::new();
        let mut selected_views = std::collections::BTreeSet::new();
        {
            let mappings = self
                .view_mappings
                .lock()
                .map_err(|_| ModelError::Conflict("state root-qualified metadata"))?;
            for binding in bindings {
                let (rows, _) =
                    mappings
                        .get(&binding.view.identity)
                        .ok_or(ModelError::Conflict(
                            "state lacks exact bound-root mappings",
                        ))?;
                for row in rows {
                    owners.insert(row.physical);
                    selected_views.insert(row.view);
                }
            }
        }
        for view in selected_views {
            if views.contains_key(&view) {
                continue;
            }
            let mut stream = self.track_rows(NativeRows::new(
                self.read_client()?
                    .query("SELECT * FROM $view")
                    .bind(("view", RecordId::new("compiler_view", view.hex())))
                    .stream_items()
                    .map_err(ModelError::codec)?,
                1,
            )?)?;
            let row = stream
                .next()
                .await?
                .ok_or(ModelError::Conflict("state dependency view missing"))?;
            validate_state_row("compiler_view", &row)?;
            let descriptor: CompletedView = decode_descriptor(&row)?;
            if descriptor.identity != view || stream.next().await?.is_some() {
                return Err(ModelError::Conflict(
                    "state dependency view exact descriptor",
                ));
            }
            views.insert(view, descriptor);
        }
        Ok((views, owners))
    }
    async fn logical_recovery_closure(
        self: &Arc<Self>,
        bindings: &[CompletedBinding],
        budget: &ResourceBudget,
    ) -> Result<
        (
            d::recovery_closure::RecoveryClosure,
            std::collections::BTreeSet<ContentHash>,
            d::charged::StateCharge,
        ),
        ModelError,
    > {
        let mut charge =
            d::charged::StateCharge::new(budget, "bound root-qualified recovery observations");
        let mut mappings =
            BTreeMap::<(ContentHash, ContentHash), std::collections::BTreeSet<ContentHash>>::new();
        let mut owners = std::collections::BTreeSet::new();
        let mut incarnations = BTreeMap::new();
        {
            let registered = self
                .view_mappings
                .lock()
                .map_err(|_| ModelError::Conflict("recovery root-qualified copies"))?;
            for binding in bindings {
                let (rows, _) =
                    registered
                        .get(&binding.view.identity)
                        .ok_or(ModelError::Conflict(
                            "recovery lacks exact bound-root mappings",
                        ))?;
                for row in rows {
                    charge.grow(512)?;
                    if row.incarnation <= 0
                        || incarnations
                            .insert(row.physical, row.incarnation)
                            .is_some_and(|previous| previous != row.incarnation)
                    {
                        return Err(ModelError::Conflict(
                            "state bound-root copy incarnation collision",
                        ));
                    }
                    mappings
                        .entry((row.view, row.logical))
                        .or_default()
                        .insert(row.physical);
                    owners.insert(row.physical);
                }
            }
        }
        // Observe every selected physical copy once. Equal logical descriptors may
        // share semantic traversal while each original root keeps its exact aliases.
        let mut descriptors = BTreeMap::new();
        let mut logical = BTreeMap::new();
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let result =
            async {
                let physical = owners
                    .iter()
                    .map(|owner| RecordId::new("compiler_contribution", owner.hex()))
                    .collect::<Vec<_>>();
                for window in physical.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                    let observed =
                        lctx_native::control::inspect_expected_window_read(&reader, window, budget)
                            .await?;
                    for record in window {
                        let expected = observed
                            .get(record)?
                            .ok_or(ModelError::Conflict("state selected contributor missing"))?;
                        validate_state_row("compiler_contribution", expected.row())?;
                        charge.grow(
                            crate::loader::native_bytes(expected.row())
                                .saturating_mul(32)
                                .saturating_add(2048),
                        )?;
                        let descriptor: CompletedContribution = decode_descriptor(expected.row())?;
                        let identity = descriptor.identity()?;
                        if let Some(previous) = logical.get(&identity) {
                            if previous != &descriptor {
                                return Err(ModelError::Conflict(
                                    "state logical contributor full descriptor collision",
                                ));
                            }
                        } else {
                            logical.insert(identity, descriptor.clone());
                        }
                        let owner = ContentHash(
                            hex::decode(match &record.key {
                                RecordIdKey::String(key) => key,
                                _ => return Err(ModelError::Schema("state physical owner key")),
                            })
                            .map_err(ModelError::codec)?
                            .try_into()
                            .map_err(|_| ModelError::Schema("state physical owner width"))?,
                        );
                        if incarnations.get(&owner) != Some(&expected.incarnation()) {
                            return Err(ModelError::Conflict(
                                "state selected copy incarnation changed",
                            ));
                        }
                        descriptors.insert(owner, identity);
                    }
                }
                let mut traversal = d::recovery_closure::RecoveryTraversal::new(bindings.to_vec())?;
                let mut views = BTreeMap::<ContentHash, CompletedView>::new();
                while let Some(view) = traversal.next_view()? {
                    let actual = if let Some(actual) = views.get(&view.identity) {
                        actual.clone()
                    } else {
                        let record = RecordId::new("compiler_view", view.identity.hex());
                        let observed = lctx_native::control::inspect_expected_window_read(
                            &reader,
                            std::slice::from_ref(&record),
                            budget,
                        )
                        .await?;
                        let expected = observed
                            .get(&record)?
                            .ok_or(ModelError::Conflict("state dependency view missing"))?;
                        validate_state_row("compiler_view", expected.row())?;
                        charge.grow(
                            crate::loader::native_bytes(expected.row())
                                .saturating_mul(32)
                                .saturating_add(2048),
                        )?;
                        let actual: CompletedView = decode_descriptor(expected.row())?;
                        views.insert(view.identity, actual.clone());
                        actual
                    };
                    if actual != view {
                        return Err(ModelError::Conflict("state exact selected view descriptor"));
                    }
                    let mut selected = vec![];
                    for identity in &view.contributions {
                        let copies = mappings.get(&(view.identity, *identity)).ok_or(
                            ModelError::Conflict("state missing root-qualified contributor slot"),
                        )?;
                        for physical in copies {
                            if descriptors.get(physical) != Some(identity) {
                                return Err(ModelError::Conflict(
                                    "state selected copy logical correspondence",
                                ));
                            }
                        }
                        let descriptor = logical.get(identity).ok_or(ModelError::Conflict(
                            "state selected logical descriptor missing",
                        ))?;
                        for input in &descriptor.spec.inputs {
                            if !views.contains_key(&input.view()) {
                                let record = RecordId::new("compiler_view", input.view().hex());
                                let observed = lctx_native::control::inspect_expected_window_read(
                                    &reader,
                                    std::slice::from_ref(&record),
                                    budget,
                                )
                                .await?;
                                let expected = observed
                                    .get(&record)?
                                    .ok_or(ModelError::Conflict("state dependency view missing"))?;
                                validate_state_row("compiler_view", expected.row())?;
                                charge.grow(
                                    crate::loader::native_bytes(expected.row())
                                        .saturating_mul(32)
                                        .saturating_add(2048),
                                )?;
                                let dependency: CompletedView = decode_descriptor(expected.row())?;
                                if dependency.identity != input.view() {
                                    return Err(ModelError::Conflict(
                                        "state exact dependency descriptor",
                                    ));
                                }
                                views.insert(dependency.identity, dependency);
                            }
                        }
                        selected.push(descriptor.clone());
                    }
                    traversal.include_view(&view, actual, selected, |id| {
                        views
                            .get(&id)
                            .cloned()
                            .ok_or(ModelError::Conflict("state dependency descriptor missing"))
                    })?;
                }
                Ok((traversal.finish()?, owners, charge))
            }
            .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "root-qualified recovery observation reader close",
            reader.close().await,
        );
        d::completion::complete(result, terminal)
    }
    pub async fn capture_audit(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<AuditCapture, ModelError> {
        let lease = self.admit(false, "capture_audit", false)?;
        let result = lease.within(self.capture_state(budget)).await;
        lease.finish_with(result)
    }
    pub async fn verify_captured(
        self: &Arc<Self>,
        selection: &AuditCapture,
    ) -> Result<(), ModelError> {
        if !Arc::ptr_eq(&selection.owner, &self.admission) {
            return Err(ModelError::Conflict("foreign audit capture owner"));
        }
        let lease = self.admit(false, "verify_captured", false)?;
        let result = lease.within(self.verify_state_selected(selection)).await;
        lease.finish_with(result)
    }
    pub async fn completed_state_captured(
        self: &Arc<Self>,
        selection: &AuditCapture,
    ) -> Result<CompletedStateIdentity, ModelError> {
        if !Arc::ptr_eq(&selection.owner, &self.admission) {
            return Err(ModelError::Conflict("foreign audit capture owner"));
        }
        let lease = self.admit(false, "completed_state_captured", false)?;
        let result = lease
            .within(self.state_identity_selected(true, selection))
            .await;
        lease.finish_with(result)
    }
    async fn capture_state(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<AuditCapture, ModelError> {
        let bindings = self.bindings_inner().await?;
        let phase = crate::phase::Phase::begin("native_audit_logical_closure");
        let closure = self.logical_recovery_closure(&bindings, budget).await;
        phase.finish_result(&closure);
        let (closure, owners, closure_charge) = closure?;
        let selection_owners = owners.iter().copied().collect::<Vec<_>>();
        let phase = crate::phase::Phase::begin("native_audit_payload_selection");
        let payloads = async {
            let client = self.client.clone();
            let cancellation = self.read_cancellation()?;
            let lease = self.retained_scan()?;
            let retained_budget = budget.clone();
            let task = self.runtime.spawn(async move {
                let result = lease
                    .within(crate::selection::SelectedPayloads::for_cancellable_owners(
                        client,
                        &selection_owners,
                        cancellation,
                        &retained_budget,
                    ))
                    .await;
                lease.finish_with(result)
            });
            task.await.map_err(ModelError::codec)?
        }
        .await;
        phase.finish_result(&payloads);
        let payloads = payloads?;
        let mut variables = Variables::new();
        variables.insert(
            "state_owners",
            owners
                .into_iter()
                .map(|owner| RecordId::new("compiler_contribution", owner.hex()))
                .collect::<Vec<_>>(),
        );
        variables.insert(
            "state_views",
            closure
                .views()
                .keys()
                .map(|view| RecordId::new("compiler_view", view.hex()))
                .collect::<Vec<_>>(),
        );
        Ok(AuditCapture {
            owner: self.admission.clone(),
            bindings,
            variables,
            closure,
            payloads,
            budget: budget.clone(),
            _closure_charge: closure_charge,
        })
    }
    fn state_source(
        &self,
        table: &str,
        selection: &AuditCapture,
    ) -> Result<NativeRows, ModelError> {
        if table == "compiler_record" {
            return selection
                .payloads
                .rows(table, "true", Variables::new(), vec![], "id", None)
                .map(|rows| rows.with_row_bytes(d::resources::MAX_ROW_BYTES));
        }
        if table == "compiler_alias" {
            let mut cursor = selection.payloads.aliases()?;
            return NativeRows::owned(move |sender| async move {
                while let Some(row) = cursor.next_row()? {
                    if sender.send(row).await.is_err() {
                        break;
                    }
                }
                Ok(())
            })
            .map(|rows| rows.with_row_bytes(d::resources::MAX_ROW_BYTES));
        }
        let key = match table {
            "compiler_contribution" | "compiler_membership" => "state_owners",
            "compiler_view" => "state_views",
            _ => return Err(ModelError::Schema("native state query family")),
        };
        let Value::Array(roots) = selection
            .variables
            .get(key)
            .cloned()
            .ok_or(ModelError::Schema("native state roots"))?
        else {
            return Err(ModelError::Schema("native state roots"));
        };
        let roots = roots.into_iter().collect::<Vec<_>>();
        let client = self.client.clone();
        let table = table.to_owned();
        let cancellation = self.read_cancellation()?;
        let budget = selection.budget.clone();
        NativeRows::owned(move |sender| async move {
            for roots in roots.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                if sender.is_closed() {
                    break;
                }
                let mut charge = d::charged::StateCharge::new(&budget, "state-root-window");
                charge.grow(
                    roots
                        .iter()
                        .map(|root| crate::loader::native_bytes(root).saturating_mul(2))
                        .sum(),
                )?;
                let sql = if table == "compiler_membership" {
                    "SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $roots"
                } else {
                    "SELECT * FROM $roots"
                };
                let mut source = crate::prepared::PreparedQuery::new(
                    Variables::from_iter([("roots".into(), roots.to_vec().into_value())]),
                    vec![],
                    vec![sql.into()],
                )?
                .stream_cancellable(&client, cancellation.as_ref())?
                .with_row_bytes(d::resources::MAX_ROW_BYTES);
                let result = async {
                    while let Some(row) = source.next().await? {
                        if sender.send(row).await.is_err() {
                            break;
                        }
                    }
                    Ok(())
                }
                .await;
                let mut terminal = d::completion::Completion::default();
                terminal.step(
                    "state selected query drainage",
                    source.drain_transport().await,
                );
                d::completion::complete(result, terminal)?;
                if sender.is_closed() {
                    break;
                }
            }
            Ok(())
        }).map(|rows|rows.with_row_bytes(d::resources::MAX_ROW_BYTES))
    }
    #[cfg(test)]
    async fn registered_or_install_views(
        self: &Arc<Self>,
        views: &[&CompletedView],
        budget: &ResourceBudget,
    ) -> Result<(u64, u64, u64, u64), ModelError> {
        let lease = self.admit(false, "exact internal selection batch", true)?;
        let result = lease
            .within(async {
                let mut seen = BTreeMap::<ContentHash, &CompletedView>::new();
                let mut charge = d::charged::StateCharge::new(
                    budget,
                    "internal selection descriptor correspondence",
                );
                for view in views {
                    if let Some(previous) = seen.get(&view.identity) {
                        if *previous != *view {
                            return Err(ModelError::Conflict(
                                "internal selection descriptor collision",
                            ));
                        }
                        continue;
                    }
                    charge.grow(std::mem::size_of::<(ContentHash, &CompletedView)>() + 64)?;
                    seen.insert(view.identity, view);
                    self.installed_view_inner(view, budget).await?;
                }
                let audit = self.view_audit.lock().await;
                Ok(audit.as_ref().map_or((0, 0, 0, 0), |audit| {
                    let (contributors, blobs) = audit.observations();
                    (
                        contributors,
                        blobs,
                        audit.logical_contribution_observations(),
                        audit.alias_observations(),
                    )
                }))
            })
            .await;
        lease.finish_with(result)
    }
    /// Establishes exact physical view authority while unrelated producers remain open.
    /// Candidate discovery never selects a global logical match. Only this store's
    /// explicit production/held attachment/import inventory supplies an initial copy.
    pub async fn installed_view(
        self: &Arc<Self>,
        view: &CompletedView,
        budget: &ResourceBudget,
    ) -> Result<Arc<lctx_native::installed_view::HeldInstalledView>, ModelError> {
        let lease = self.admit(false, "exact installed view acquisition", true)?;
        let result = lease.within(self.installed_view_inner(view, budget)).await;
        lease.finish_with(result)
    }
    /// Reader construction consumes native-held exact copies, never logical view
    /// names. Its scope retains every selected capability through delivery/drain.
    pub async fn reader_for_views(
        self: &Arc<Self>,
        views: &[CompletedView],
        budget: &ResourceBudget,
    ) -> Result<crate::NativeReader<()>, ModelError> {
        let lease = self.admit(false, "exact selected reader acquisition", true)?;
        let result = lease
            .within(async {
                let _charge = budget.reserve(
                    "compiler selected reader capabilities",
                    views
                        .len()
                        .saturating_mul(std::mem::size_of::<
                            Arc<lctx_native::installed_view::HeldInstalledView>,
                        >())
                        .saturating_add(1024),
                )?;
                let mut held: Vec<Arc<lctx_native::installed_view::HeldInstalledView>> =
                    Vec::with_capacity(views.len());
                for view in views {
                    if let Some(previous) = held
                        .iter()
                        .find(|previous| previous.completion().view.identity == view.identity)
                    {
                        if previous.completion().view != *view {
                            return Err(ModelError::Conflict(
                                "selected reader descriptor collision",
                            ));
                        }
                        continue;
                    }
                    held.push(self.installed_view_inner(view, budget).await?);
                }
                let reader =
                    crate::NativeReader::for_installed_views(self.client.clone(), &held, budget)
                        .await?;
                Ok(if let Some(flag) = self.read_cancellation()? {
                    reader.with_read_cancellation(flag)
                } else {
                    reader
                })
            })
            .await;
        lease.finish_with(result)
    }
    async fn canonical_selection(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<(crate::NativeReader<()>, crate::selection::SelectedPayloads), ModelError> {
        let bindings = self.bindings_inner().await?;
        let _charge =
            budget.reserve(
                "compiler canonical selected root descriptors",
                bindings
                    .len()
                    .saturating_mul(std::mem::size_of::<CompletedView>())
                    .saturating_add(
                        bindings
                            .iter()
                            .map(|binding| {
                                binding.view.relation.capacity().saturating_add(
                                    binding.view.contributions.len().saturating_mul(96),
                                )
                            })
                            .sum::<usize>(),
                    ),
            )?;
        let roots = bindings
            .into_iter()
            .filter(|binding| binding.boundary.is_none())
            .map(|binding| binding.view)
            .collect::<Vec<_>>();
        // This private consumer lives beneath the actual setup/scan lease. Public
        // reader_for_views keeps its independent receiving/session lifetime.
        let reader = self.canonical_reader_for_views(&roots, budget).await?;
        let selected = reader.prepare_selection().await.and_then(|selected| {
            selected.ok_or(ModelError::Conflict(
                "compiler canonical installed selection missing",
            ))
        });
        match selected {
            Ok(selected) => Ok((reader, selected)),
            Err(error) => {
                let mut terminal = d::completion::Completion::default();
                terminal.step("failed canonical reader setup close", reader.close().await);
                d::completion::complete(Err(error), terminal)
            }
        }
    }
    async fn canonical_reader_for_views(
        self: &Arc<Self>,
        views: &[CompletedView],
        budget: &ResourceBudget,
    ) -> Result<crate::NativeReader<()>, ModelError> {
        if !self.has_live_parent() {
            return Err(ModelError::Conflict(
                "borrowed canonical reader requires admitted source operation",
            ));
        }
        let _charge = budget.reserve(
            "canonical reader exact source handles",
            views
                .len()
                .saturating_mul(std::mem::size_of::<
                    Arc<lctx_native::installed_view::HeldInstalledView>,
                >())
                .saturating_add(1024),
        )?;
        let mut held: Vec<Arc<lctx_native::installed_view::HeldInstalledView>> =
            Vec::with_capacity(views.len());
        for view in views {
            if let Some(previous) = held
                .iter()
                .find(|previous| previous.completion().view.identity == view.identity)
            {
                if previous.completion().view != *view {
                    return Err(ModelError::Conflict(
                        "canonical reader exact descriptor collision",
                    ));
                }
                continue;
            }
            held.push(self.installed_view_inner(view, budget).await?);
        }
        let internal = {
            let registered = self
                .installed_views
                .lock()
                .map_err(|_| ModelError::Conflict("canonical reader source registry"))?;
            held.iter().all(|cap| {
                cap.is_internal()
                    && registered
                        .get(&cap.completion().view.identity)
                        .is_some_and(|owned| Arc::ptr_eq(owned, cap))
            })
        };
        let reader = if internal {
            crate::NativeReader::borrow_installed_views(&held, budget).await?
        } else {
            crate::NativeReader::for_installed_views(self.client.clone(), &held, budget).await?
        };
        Ok(if let Some(flag) = self.read_cancellation()? {
            reader.with_read_cancellation(flag)
        } else {
            reader
        })
    }
    async fn installed_view_inner(
        self: &Arc<Self>,
        view: &CompletedView,
        budget: &ResourceBudget,
    ) -> Result<Arc<lctx_native::installed_view::HeldInstalledView>, ModelError> {
        self.check_failed()?;
        view.validate()?;
        let cached = self
            .installed_views
            .lock()
            .map_err(|_| ModelError::Conflict("native installed view owner"))?
            .get(&view.identity)
            .cloned();
        if let Some(held) = cached {
            if held.completion().view != *view {
                return Err(ModelError::Conflict("installed view descriptor collision"));
            }
            let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
            let reader = if let Some(flag) = self.read_cancellation()? {
                reader.with_read_cancellation(flag)
            } else {
                reader
            };
            let result = held
                .ensure_borrowed_current(&reader, budget)
                .await
                .map(|_| held);
            let mut terminal = d::completion::Completion::default();
            terminal.step("cached installed view reader close", reader.close().await);
            return d::completion::complete(result, terminal);
        }
        let physical = self
            .physical_completed
            .lock()
            .map_err(|_| ModelError::Conflict("physical installed completion owner"))?
            .clone();
        if let Some(physical) = physical {
            let held = Arc::new(
                lctx_native::installed_view::hold_physical_view(
                    self.client.clone(),
                    physical,
                    view.identity,
                    budget,
                )
                .await?,
            );
            if held.completion().view != *view {
                return Err(ModelError::Conflict(
                    "physical installed view descriptor collision",
                ));
            }
            self.installed_views
                .lock()
                .map_err(|_| ModelError::Conflict("physical installed view owner"))?
                .insert(view.identity, held.clone());
            return Ok(held);
        }
        if self.read_only.load(Ordering::Acquire) {
            let completed = self
                .published_completion
                .lock()
                .map_err(|_| ModelError::Conflict("published exact completion owner"))?
                .clone()
                .ok_or(ModelError::Conflict(
                    "published context lacks protected exact inventory",
                ))?;
            let held = Arc::new(
                lctx_native::installed_view::hold_published_view(&completed, view.identity, budget)
                    .await?,
            );
            if held.completion().view != *view {
                return Err(ModelError::Conflict(
                    "published installed descriptor mismatch",
                ));
            }
            self.installed_views
                .lock()
                .map_err(|_| ModelError::Conflict("published installed view owner"))?
                .insert(view.identity, held.clone());
            return Ok(held);
        }
        // The registry owns an independently spawned worker. Dropping a waiter does
        // not drop its native preparation, and no registry lock crosses an await.
        let (pending, token) = {
            let mut preparations = self
                .view_preparations
                .lock()
                .map_err(|_| ModelError::Conflict("internal selection preparation registry"))?;
            if let Some(entry) = preparations.get(&view.identity) {
                if !entry.budget.shares_pool(budget) {
                    return Err(ModelError::Conflict(
                        "internal selection foreign resource owner",
                    ));
                }
                (entry.pending.clone(), entry.token.clone())
            } else {
                let token = Arc::new(
                    budget.reserve(
                        "internal selection preparation entry",
                        view.relation
                            .len()
                            .saturating_add(view.contributions.len().saturating_mul(64))
                            .saturating_add(2048),
                    )?,
                );
                let lease = self.admit_mutation("earn exact internal selection", true)?;
                let binding = self.producing_binding()?;
                let store = self.clone();
                let selected = view.clone();
                let selected_budget = budget.clone();
                let task = self.runtime.spawn(
                    async move {
                        let result = lease
                            .within(producing_future(binding, async {
                                store.install_view_inner(&selected, &selected_budget).await
                            }))
                            .await;
                        lease.finish_with(result).map_err(Arc::new)
                    }
                    .in_current_span()
                    .with_current_subscriber(),
                );
                let pending = async move {
                    task.await
                        .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))?
                }
                .boxed()
                .shared();
                preparations.insert(
                    view.identity,
                    ViewPreparation {
                        pending: pending.clone(),
                        token: token.clone(),
                        budget: budget.clone(),
                    },
                );
                (pending, token)
            }
        };
        let result = pending.await.map_err(ModelError::SharedCause);
        if result.is_err()
            && let Ok(mut preparations) = self.view_preparations.lock()
            && preparations
                .get(&view.identity)
                .is_some_and(|current| Arc::ptr_eq(&current.token, &token))
        {
            preparations.remove(&view.identity);
        }
        result
    }
    async fn root_gates(
        &self,
        roots: impl IntoIterator<Item = ContentHash>,
        budget: &ResourceBudget,
    ) -> Result<Vec<tokio::sync::OwnedMutexGuard<()>>, ModelError> {
        let roots = roots.into_iter().collect::<std::collections::BTreeSet<_>>();
        let mut gates = Vec::with_capacity(roots.len());
        {
            let mut registry = self
                .product_retention_gates
                .lock()
                .map_err(|_| ModelError::Conflict("root installation gate registry"))?;
            for root in roots {
                let mut sink = KeySink::new("compiler-root-installation-gate/v1");
                root.encode(&mut sink);
                let key = sink.finish();
                if let Some((gate, _)) = registry.get(&key) {
                    gates.push(gate.clone());
                } else {
                    let gate = Arc::new(tokio::sync::Mutex::new(()));
                    let charge = Arc::new(budget.reserve("exact root installation gate", 1024)?);
                    registry.insert(key, (gate.clone(), charge));
                    gates.push(gate);
                }
            }
        }
        let mut held = Vec::with_capacity(gates.len());
        for gate in gates {
            held.push(gate.lock_owned().await);
        }
        Ok(held)
    }
    async fn install_view_inner(
        self: &Arc<Self>,
        view: &CompletedView,
        budget: &ResourceBudget,
    ) -> Result<Arc<lctx_native::installed_view::HeldInstalledView>, ModelError> {
        let _root_gate = self.root_gates([view.identity], budget).await?;
        self.install_view_under_gate(view, budget).await
    }
    async fn install_view_under_gate(
        self: &Arc<Self>,
        view: &CompletedView,
        budget: &ResourceBudget,
    ) -> Result<Arc<lctx_native::installed_view::HeldInstalledView>, ModelError> {
        if let Some(held) = self
            .installed_views
            .lock()
            .map_err(|_| ModelError::Conflict("installed root gate owner"))?
            .get(&view.identity)
            .cloned()
        {
            return Ok(held);
        }
        if self.read_only.load(Ordering::Acquire) {
            return Err(ModelError::Conflict(
                "published context requires protected completed inventory",
            ));
        }
        let (mappings, _mapping_charge) = self.selected_view_mappings(view, budget).await?;
        self.declare_view_inner(view, &mappings, budget).await?;
        let mut scope = self.view_audit.lock().await;
        if scope.is_none() {
            *scope = Some(
                lctx_native::installed_view::InstallationAudit::new(self.client.clone(), budget)
                    .await?,
            );
        }
        let audit = scope
            .as_ref()
            .ok_or(ModelError::Conflict("internal selection preparation owner"))?
            .clone();
        drop(scope);
        let held = Arc::new(
            lctx_native::installed_view::select_live_view_scoped(
                self.client.clone(),
                self.attempt,
                view,
                &mappings,
                budget,
                &audit,
            )
            .await?,
        );
        self.remember_view_mapping(view, &mappings, budget)?;
        self.remember_view(view)?;
        let mut installed = self
            .installed_views
            .lock()
            .map_err(|_| ModelError::Conflict("native installed view owner"))?;
        if let Some(previous) = installed.get(&view.identity) {
            if previous.completion().view != held.completion().view
                || previous.completion().mappings != held.completion().mappings
            {
                return Err(ModelError::Conflict(
                    "concurrent installed view selected different copies",
                ));
            }
            return Ok(previous.clone());
        }
        installed.insert(view.identity, held.clone());
        Ok(held)
    }
    /// Persist only model-owned semantic declaration and dependency incidence.
    /// This grants no earned selector or independently retained physical boundary.
    async fn declare_view_inner(
        &self,
        view: &CompletedView,
        mappings: &[lctx_native::installed_view::ContributorRealization],
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.declare_view_window(&[(view, mappings)], budget).await
    }
    async fn declare_view_window(
        &self,
        views: &[(
            &CompletedView,
            &[lctx_native::installed_view::ContributorRealization],
        )],
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if views.len() > crate::loader::NATIVE_WINDOW_ROWS {
            return Err(ModelError::Conflict(
                "semantic view declaration window bound",
            ));
        }
        let _handles = budget.reserve(
            "semantic view declaration iterator",
            65536usize.saturating_add(views.len().saturating_mul(std::mem::size_of::<Value>())),
        )?;
        let rows = views.iter().map(|(view, _)| {
            let mut input = budget.reserve(
                "semantic view serialized input",
                view.relation
                    .capacity()
                    .saturating_add(view.contributions.len().saturating_mul(256))
                    .saturating_add(1024)
                    .saturating_mul(32)
                    .saturating_add(65536),
            )?;
            view.validate()?;
            let mut row = Object::new();
            row.insert("id", RecordId::new("compiler_view", view.identity.hex()));
            row.insert(
                "descriptor",
                Bytes::from(serde_json::to_vec(view).map_err(ModelError::codec)?),
            );
            for (field, value) in view_projection(view)? {
                row.insert(field, value);
            }
            let row = Value::Object(row);
            input.try_resize(
                crate::loader::native_bytes(&row)
                    .saturating_mul(2)
                    .saturating_add(1024),
            )?;
            Ok((row, Some(input)))
        });
        // The existing immutable owner splits by complete expanded request bytes
        // as well as row count, and retains each serialized input until its effect.
        let windows = crate::control::ImmutableWindows::from_charged_iter(
            rows,
            Some(self.attempt),
            None,
            None,
        )
        .with_budget(Some(budget));
        for window in windows {
            crate::control::install_window(&self.client, Some(self.attempt), window?, Some(budget))
                .await?;
        }
        let _pair_handles = budget.reserve(
            "semantic view hold page handles",
            crate::loader::NATIVE_WINDOW_ROWS
                .saturating_mul(2 * std::mem::size_of::<(RecordId, RecordId)>())
                .saturating_add(4096),
        )?;
        let mut pairs = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS);
        for (view, mappings) in views {
            let owner = RecordId::new("compiler_view", view.identity.hex());
            pairs.push((
                RecordId::new("native_attempt", self.attempt.hex()),
                owner.clone(),
            ));
            if pairs.len() == crate::loader::NATIVE_WINDOW_ROWS {
                self.declare_view_holds(std::mem::take(&mut pairs), budget)
                    .await?;
            }
            for mapping in mappings.iter().filter(|mapping| {
                mapping.view == view.identity && view.contributions.contains(&mapping.logical)
            }) {
                pairs.push((
                    owner.clone(),
                    RecordId::new("compiler_contribution", mapping.physical.hex()),
                ));
                if pairs.len() == crate::loader::NATIVE_WINDOW_ROWS {
                    self.declare_view_holds(std::mem::take(&mut pairs), budget)
                        .await?;
                }
            }
        }
        if !pairs.is_empty() {
            self.declare_view_holds(pairs, budget).await?;
        }
        Ok(())
    }
    async fn declare_view_holds(
        &self,
        pairs: Vec<(RecordId, RecordId)>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        // Keys here are fixed compiler/attempt families with full 64-hex identities.
        // Admit their guarded row/index/request expansion before hold_plan copies it.
        let bytes = pairs
            .iter()
            .map(|(owner, object)| {
                lctx_native::selection::node_bytes(owner)
                    .saturating_add(lctx_native::selection::node_bytes(object))
            })
            .fold(0usize, usize::saturating_add);
        let _construction = budget.reserve(
            "semantic view guarded hold page",
            bytes.saturating_mul(256).saturating_add(131072),
        )?;
        crate::control::hold_many(&self.client, Some(self.attempt), pairs).await
    }
    async fn declare_completed_views(
        self: &Arc<Self>,
        views: &[&CompletedView],
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let reader = if let Some(flag) = self.read_cancellation()? {
            reader.with_read_cancellation(flag)
        } else {
            reader
        };
        let mut observed = BTreeMap::new();
        let result = async {
            for window in views.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let _root_gates = self
                    .root_gates(window.iter().map(|view| view.identity), budget)
                    .await?;
                let _handles = budget.reserve(
                    "semantic declaration mapping window handles",
                    window.len().saturating_mul(1024).saturating_add(4096),
                )?;
                let mut mappings = Vec::with_capacity(window.len());
                for view in window {
                    mappings.push(
                        self.selected_view_mappings_read(
                            &reader,
                            view,
                            budget,
                            &mut observed,
                            false,
                        )
                        .await?,
                    );
                }
                let requested = window
                    .iter()
                    .zip(&mappings)
                    .map(|(view, (mappings, _))| (*view, mappings.as_slice()))
                    .collect::<Vec<_>>();
                self.declare_view_window(&requested, budget).await?;
                for (view, (mappings, _)) in window.iter().zip(&mappings) {
                    self.remember_view_mapping(view, mappings, budget)?;
                    self.remember_view(view)?;
                }
            }
            Ok(())
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "semantic declaration observed mapping reader close",
            reader.close().await,
        );
        d::completion::complete(result, terminal)
    }
    fn remember_view_mapping(
        &self,
        view: &CompletedView,
        mappings: &[lctx_native::installed_view::ContributorRealization],
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if mappings
            .windows(2)
            .any(|pair| (pair[0].view, pair[0].logical) >= (pair[1].view, pair[1].logical))
            || view.contributions.iter().any(|logical| {
                mappings
                    .binary_search_by_key(&(view.identity, *logical), |mapping| {
                        (mapping.view, mapping.logical)
                    })
                    .is_err()
            })
        {
            return Err(ModelError::Conflict(
                "exact per-view mapping order/root inventory",
            ));
        }
        // Preserve the full root-qualified closure; different roots may select
        // different physical copies of the same logical dependency.
        let rows = || mappings.iter();
        let mut known = self
            .view_mappings
            .lock()
            .map_err(|_| ModelError::Conflict("native per-view mapping owner"))?;
        if let Some((previous, _)) = known.get(&view.identity) {
            if !previous.iter().eq(rows()) {
                return Err(ModelError::Conflict(
                    "exact per-view physical mapping changed",
                ));
            }
        } else {
            let count = rows().count();
            let charge = budget.reserve(
                "retained exact per-view mappings",
                count
                    .saturating_mul(std::mem::size_of::<
                        lctx_native::installed_view::ContributorRealization,
                    >())
                    .saturating_add(256),
            )?;
            let mut selected = Vec::with_capacity(count);
            selected.extend(rows().cloned());
            known.insert(view.identity, (selected, Arc::new(charge)));
        }
        Ok(())
    }
    /// Replace live view capabilities with a held independently earned exact frozen
    /// physical inventory. This grants no semantic admission or publication authority.
    pub fn register_physical_inventory(
        &self,
        completed: &Arc<lctx_native::completed_state::NativeCompletedState>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if completed.completion().attempt != self.attempt
            || completed.completion().generation != self.generation
        {
            return Err(ModelError::Conflict(
                "physical inventory foreign receiving attempt",
            ));
        }
        let mut retained = self
            .physical_completed
            .lock()
            .map_err(|_| ModelError::Conflict("physical completed inventory owner"))?;
        if retained
            .as_ref()
            .is_some_and(|previous| previous.completion() != completed.completion())
        {
            return Err(ModelError::Conflict(
                "physical completed inventory replacement",
            ));
        }
        for reference in completed.inventory().references() {
            let installed = completed.inventory().installed(reference.logical_view)?;
            self.remember_view_mapping(&installed.view, &installed.mappings, budget)?;
            self.remember_view(&installed.view)?;
        }
        self.installed_views
            .lock()
            .map_err(|_| ModelError::Conflict("physical live capability replacement"))?
            .clear();
        *retained = Some(completed.clone());
        Ok(())
    }
    /// Read-only publication consumers retain the supplied protected capability for
    /// their full operation. This copies its exact mappings, never a global search.
    pub fn register_published_inventory(
        &self,
        completed: &lctx_native::completed_state::PublishedCompletedState,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if !self.read_only.load(Ordering::Acquire) {
            return Err(ModelError::Conflict(
                "published inventory requires read-only compiler owner",
            ));
        }
        completed.require_session(self.client.as_ref())?;
        let bindings = self
            .published_bindings
            .lock()
            .map_err(|_| ModelError::Conflict("published bindings"))?;
        let bindings = bindings
            .as_ref()
            .ok_or(ModelError::Conflict("published binding inventory absent"))?;
        if bindings.len() != completed.inventory().bindings().len()
            || bindings
                .iter()
                .any(|binding| !completed.inventory().bindings().contains(binding))
        {
            return Err(ModelError::Conflict(
                "published binding inventory differs from protected capability",
            ));
        }
        let mut retained = self
            .published_completion
            .lock()
            .map_err(|_| ModelError::Conflict("published exact completion owner"))?;
        if retained
            .as_ref()
            .is_some_and(|previous| !previous.same_capability(completed))
        {
            return Err(ModelError::Conflict(
                "published completion capability replacement",
            ));
        }
        for reference in completed.inventory().references() {
            let installed = completed.inventory().installed(reference.logical_view)?;
            self.remember_view_mapping(&installed.view, &installed.mappings, budget)?;
            self.remember_view(&installed.view)?;
        }
        *retained = Some(completed.clone());
        Ok(())
    }
    async fn selected_view_mappings(
        self: &Arc<Self>,
        view: &CompletedView,
        budget: &ResourceBudget,
    ) -> Result<
        (
            Vec<lctx_native::installed_view::ContributorRealization>,
            d::charged::StateCharge,
        ),
        ModelError,
    > {
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let reader = if let Some(flag) = self.read_cancellation()? {
            reader.with_read_cancellation(flag)
        } else {
            reader
        };
        let mut observed = BTreeMap::new();
        let result = self
            .selected_view_mappings_read(&reader, view, budget, &mut observed, true)
            .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step("live mapping private reader close", reader.close().await);
        d::completion::complete(result, terminal)
    }
    // Observation reuse is local to one admitted declaration/read operation. This
    // compact metadata is never an earned selector or independent completion grant.
    async fn selected_view_mappings_read(
        self: &Arc<Self>,
        reader: &crate::NativeReader<()>,
        view: &CompletedView,
        budget: &ResourceBudget,
        observed: &mut BTreeMap<
            ContentHash,
            (
                crate::control::ExpectedRecord,
                CompletedContribution,
                Box<dyn Reservation>,
            ),
        >,
        allow_cached_caps: bool,
    ) -> Result<
        (
            Vec<lctx_native::installed_view::ContributorRealization>,
            d::charged::StateCharge,
        ),
        ModelError,
    > {
        use lctx_native::installed_view::ContributorRealization;
        let mut charge =
            d::charged::StateCharge::new(budget, "selected exact live dependency mappings");
        let cached = self
            .installed_views
            .lock()
            .map_err(|_| ModelError::Conflict("selected installed mapping owner"))?
            .get(&view.identity)
            .cloned();
        if let Some(held) = cached.filter(|_| allow_cached_caps) {
            if held.completion().view != *view {
                return Err(ModelError::Conflict(
                    "selected installed mapping descriptor",
                ));
            }
            held.ensure_borrowed_current(&reader, budget).await?;
            charge.grow(
                held.completion()
                    .mappings
                    .len()
                    .saturating_mul(std::mem::size_of::<ContributorRealization>()),
            )?;
            // The root owns its complete chosen dependency incidence. A different
            // root may independently select equivalent copies of those same views.
            return Ok((held.completion().mappings.clone(), charge));
        }
        if let Some((mappings, _)) = self
            .view_mappings
            .lock()
            .map_err(|_| ModelError::Conflict("root-qualified exact mappings"))?
            .get(&view.identity)
        {
            charge.grow(
                mappings
                    .len()
                    .saturating_mul(std::mem::size_of::<ContributorRealization>()),
            )?;
            return Ok((mappings.clone(), charge));
        }
        let view_bytes = |view: &CompletedView| {
            std::mem::size_of::<CompletedView>()
                .saturating_add(view.relation.capacity())
                .saturating_add(
                    view.contributions
                        .len()
                        .saturating_mul(std::mem::size_of::<ContentHash>() + 64),
                )
                .saturating_add(128)
        };
        struct PendingView(CompletedView, Box<dyn d::resources::Reservation>);
        impl d::HeapSize for PendingView {
            // The descriptor's reservation moves with it through processing;
            // ChargedVec separately retains the stack's highwater capacity.
            fn heap_bytes(&self) -> usize {
                0
            }
        }
        let initial = budget.reserve("selected dependency pending view", view_bytes(view))?;
        let mut pending_capacity =
            d::charged::StateCharge::new(budget, "selected dependency pending capacity");
        let mut pending = d::charged::ChargedVec::default();
        pending.push(&mut pending_capacity, PendingView(view.clone(), initial))?;
        let mut seen = std::collections::BTreeSet::new();
        let mut mappings = BTreeMap::new();
        let mut working =
            d::charged::StateCharge::new(budget, "selected dependency traversal state");
        while let Some(PendingView(selected, _selected_charge)) =
            pending.take_last(&mut pending_capacity)
        {
            if seen.contains(&selected.identity) {
                continue;
            }
            working.grow(std::mem::size_of::<ContentHash>() + 64)?;
            seen.insert(selected.identity);
            for logical in &selected.contributions {
                let explicit = self
                    .view_mappings
                    .lock()
                    .map_err(|_| ModelError::Conflict("native exact view mappings"))?
                    .get(&selected.identity)
                    .and_then(|(rows, _)| {
                        rows.iter()
                            .find(|row| row.view == selected.identity && row.logical == *logical)
                    })
                    .cloned();
                let physical = if let Some(mapping) = &explicit {
                    mapping.physical
                } else {
                    *self
                        .known_contributors
                        .lock()
                        .map_err(|_| ModelError::Conflict("native contributor candidates"))?
                        .get(logical)
                        .ok_or(ModelError::Conflict(
                            "dependency has no explicitly selected physical contributor",
                        ))?
                };
                if !observed.contains_key(&physical) {
                    let expected =
                        lctx_native::installed_view::inspect_contributor(reader, physical, budget)
                            .await?
                            .ok_or(ModelError::Conflict(
                                "selected dependency contributor missing",
                            ))?;
                    validate_state_row("compiler_contribution", expected.row())?;
                    let mut retained = budget.reserve(
                        "selected mapping observed full descriptor",
                        crate::loader::native_bytes(expected.row())
                            .saturating_mul(32)
                            .saturating_add(66560),
                    )?;
                    let descriptor: CompletedContribution = decode_descriptor(expected.row())?;
                    retained.try_resize(
                        crate::loader::native_bytes(expected.row())
                            .saturating_mul(8)
                            .saturating_add(1024),
                    )?;
                    observed.insert(physical, (expected, descriptor, retained));
                }
                let (expected, descriptor, _) = observed.get(&physical).ok_or(
                    ModelError::Conflict("selected mapping observed descriptor absent"),
                )?;
                if explicit
                    .as_ref()
                    .is_some_and(|mapping| mapping.incarnation != expected.incarnation())
                {
                    return Err(ModelError::Conflict(
                        "selected dependency incarnation changed",
                    ));
                }

                if descriptor.identity()? != *logical
                    || !descriptor.outputs.contains_key(&selected.relation)
                {
                    return Err(ModelError::Conflict("selected dependency logical output"));
                }
                let mapping = ContributorRealization {
                    view: selected.identity,
                    logical: *logical,
                    physical,
                    incarnation: expected.incarnation(),
                };
                if let Some(previous) = mappings.get(&(selected.identity, *logical)) {
                    if previous != &mapping {
                        return Err(ModelError::Conflict(
                            "one view closure selects incompatible physical copies",
                        ));
                    }
                } else {
                    working.grow(
                        std::mem::size_of::<ContributorRealization>()
                            + std::mem::size_of::<(ContentHash, ContentHash)>()
                            + 64,
                    )?;
                    mappings.insert((selected.identity, *logical), mapping);
                }
                for source in &descriptor.spec.inputs {
                    let known = self
                        .known_views
                        .lock()
                        .map_err(|_| ModelError::Conflict("native declared views"))?;
                    let dependency = known.get(&source.view()).ok_or(ModelError::Conflict(
                        "dependency view requires exact prior attachment/import",
                    ))?;
                    let retained = budget
                        .reserve("selected dependency pending view", view_bytes(dependency))?;
                    pending.push(
                        &mut pending_capacity,
                        PendingView(dependency.clone(), retained),
                    )?;
                }
            }
        }
        charge.grow(
            mappings
                .len()
                .saturating_mul(std::mem::size_of::<ContributorRealization>()),
        )?;
        Ok((mappings.into_values().collect(), charge))
    }

    /// Close durable canonical mutation and independently earn the exact physical
    /// inventory. Retained product roots protect reuse without entering artifact meaning.
    pub async fn complete_physical_content(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<Arc<lctx_native::completed_state::NativeCompletedState>, ModelError> {
        self.check_frozen_content()?;
        if let Some(completed) = self
            .physical_completed
            .lock()
            .map_err(|_| ModelError::Conflict("physical completed owner"))?
            .clone()
        {
            return Ok(completed);
        }
        let bindings = self.bindings_inner().await?;
        let (products, _product_charge) = self.retained_product_views(budget).await?;
        let mut selection_charge =
            d::charged::StateCharge::new(budget, "physical completed selected inventory");
        selection_charge.grow(
            bindings
                .len()
                .saturating_add(products.len())
                .saturating_mul(4096),
        )?;
        let mut pending = bindings
            .iter()
            .map(|binding| binding.view.clone())
            .chain(products)
            .collect::<Vec<_>>();
        let mut selections = BTreeMap::new();
        let mut mapping_charges = Vec::new();
        let mapping_reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let mapping_reader = if let Some(flag) = self.read_cancellation()? {
            mapping_reader.with_read_cancellation(flag)
        } else {
            mapping_reader
        };
        let mut observed = BTreeMap::new();
        let mapping_result = async {
            while let Some(view) = pending.pop() {
                if selections.contains_key(&view.identity) {
                    continue;
                }
                self.registered_view(&view).await?;
                let (mappings, charge) = self
                    .selected_view_mappings_read(
                        &mapping_reader,
                        &view,
                        budget,
                        &mut observed,
                        false,
                    )
                    .await?;
                for mapping in &mappings {
                    if mapping.view != view.identity && !selections.contains_key(&mapping.view) {
                        let known = self.known_views.lock().map_err(|_| {
                            ModelError::Conflict("physical dependency view inventory")
                        })?;
                        let dependency = known.get(&mapping.view).ok_or(ModelError::Conflict(
                            "physical dependency descriptor absent",
                        ))?;
                        selection_charge.grow(
                            dependency
                                .relation
                                .capacity()
                                .saturating_add(dependency.contributions.len().saturating_mul(128))
                                .saturating_add(1024),
                        )?;
                        pending.push(dependency.clone());
                    }
                }
                mapping_charges.push(charge);
                selections.insert(
                    view.identity,
                    lctx_native::completed_state::ViewSelection { view, mappings },
                );
            }
            Ok(())
        }
        .await;
        let mut mapping_terminal = d::completion::Completion::default();
        mapping_terminal.step(
            "physical inventory mapping reader close",
            mapping_reader.close().await,
        );
        d::completion::complete(mapping_result, mapping_terminal)?;
        drop(observed);
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let result = async {
            let mut vars = Variables::new();
            vars.insert(
                "attempt",
                RecordId::new("native_attempt", self.attempt.hex()),
            );
            let owners: Vec<Object> = reader
                .query_native("SELECT state,revision FROM $attempt", vars)
                .await?;
            let [owner] = owners.as_slice() else {
                return Err(ModelError::Conflict("physical freeze owner cardinality"));
            };
            let fence = if owner.get("state") == Some(&Value::String("frozen".into())) {
                crate::control::resume_content_fence(self.client.clone(), self.attempt, budget)
                    .await?
            } else {
                let revision = i64::from_value(
                    owner
                        .get("revision")
                        .cloned()
                        .ok_or(ModelError::Schema("physical freeze revision"))?,
                )
                .map_err(ModelError::codec)?;
                crate::control::freeze_content_native(
                    self.client.clone(),
                    self.attempt,
                    revision,
                    binding_inventory_identity(&bindings)?,
                    budget,
                )
                .await?
            };
            if fence.bindings() != binding_inventory_identity(&bindings)? {
                return Err(ModelError::Conflict(
                    "physical frozen binding inventory differs",
                ));
            }
            let selections = selections.into_values().collect::<Vec<_>>();
            let mut scope = self.view_audit.lock().await;
            if scope.is_none() {
                *scope = Some(
                    lctx_native::installed_view::InstallationAudit::new(
                        self.client.clone(),
                        budget,
                    )
                    .await?,
                );
            }
            let audit = scope
                .as_ref()
                .filter(|audit| audit.shares_pool(budget))
                .cloned();
            drop(scope);
            // A separately budgeted explicit frozen boundary earns its own audit.
            // Never consume another operation pool's cached preparations or raise
            // the earlier scan's bound to satisfy finalization.
            let completed = Arc::new(match audit {
                Some(audit) => {
                    lctx_native::completed_state::complete_frozen_content_scoped(
                        self.client.clone(),
                        &fence,
                        &bindings,
                        &selections,
                        budget,
                        &audit,
                    )
                    .await?
                }
                None => {
                    lctx_native::completed_state::complete_frozen_content(
                        self.client.clone(),
                        &fence,
                        &bindings,
                        &selections,
                        budget,
                    )
                    .await?
                }
            });
            self.register_physical_inventory(&completed, budget)?;
            Ok(completed)
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "physical completion setup reader close",
            reader.close().await,
        );
        d::completion::complete(result, terminal)
    }

    /// The former marker-only route cannot earn current native admission. Keep its
    /// explicit refusal while callers migrate to executed premises; neither host lane
    /// freeze nor successful legacy callbacks substitute for the opaque full ledger.
    pub async fn mark_attempt_admitted(self: &Arc<Self>) -> Result<(), ModelError> {
        Err(ModelError::Conflict(
            "native admission requires exact physical completion and executed full premises",
        ))
    }

    /// Only a private complete production artifact owner can request durable semantic
    /// admission. Native plans and executes the exact physical CAS; a serialized
    /// descriptor or successful model-only validation cannot reach this constructor.
    pub(crate) async fn admit_completed_state(
        self: &Arc<Self>,
        admission: &crate::artifact::ArtifactAdmission,
        budget: &ResourceBudget,
    ) -> Result<lctx_native::completed_state::HeldFrozenCompletedState, ModelError> {
        self.check_frozen_content()?;
        let physical = admission.physical();
        let data = physical.completion();
        if data.attempt != self.attempt || data.generation != self.generation {
            return Err(ModelError::Conflict(
                "artifact admission foreign physical completion",
            ));
        }
        admission.evidence().validate_current(
            admission.model(),
            physical.inventory().bindings(),
            &data.roots,
            budget,
        )?;
        let _copy = budget.reserve(
            "durable complete artifact evidence construction",
            admission
                .retained_bytes()
                .saturating_mul(3)
                .saturating_add(65536),
        )?;
        let persisted = lctx_native::completed_state::CompletedAdmission {
            version: 2,
            completion: data.identity()?,
            completion_incarnation: physical.incarnation(),
            generation: data.generation,
            validity_epoch: data.validity_epoch,
            evidence: admission.evidence().clone(),
        };
        let (sql, variables, _plan_charge) =
            crate::control::completed_admission_plan(physical, &persisted, budget)?;
        crate::control::effect_frozen_attempt(&self.client, self.attempt, &sql, variables).await?;
        lctx_native::completed_state::hold_completed_state_with_model(
            self.client.clone(),
            self.attempt,
            data.identity()?,
            admission.model(),
            budget,
        )
        .await
    }
    pub fn retain_product_identity<'a>(
        self: &'a Arc<Self>,
        request: ContentHash,
        contribution: ContentHash,
        budget: &'a ResourceBudget,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        Box::pin(async move {
            let lease = self.admit_mutation("retain independent product boundary", true)?;
            let phase = crate::phase::Phase::begin("native_product_retention");
            let result = lease
                .within(async {
                    let gate = {
                        let mut gates = self
                            .product_retention_gates
                            .lock()
                            .map_err(|_| ModelError::Conflict("product retention gate registry"))?;
                        if let Some((gate, _)) = gates.get(&request) {
                            gate.clone()
                        } else {
                            let charge =
                                Arc::new(budget.reserve("product retention request gate", 1024)?);
                            let gate = Arc::new(tokio::sync::Mutex::new(()));
                            gates.insert(request, (gate.clone(), charge));
                            gate
                        }
                    };
                    let _same_request = gate.lock().await;
                    self.check_failed()?;
                    if lctx_native::completed_state::check_retained_product_replay(
                        self.client.clone(),
                        self.attempt,
                        request,
                        contribution,
                        budget,
                    )
                    .await?
                    {
                        return Ok(());
                    }
                    let descriptor = self.completed_contribution(contribution).await?;
                    let logical = descriptor.identity()?;
                    let mut charge =
                        d::charged::StateCharge::new(budget, "retained product selector inventory");
                    let mut pending = Vec::new();
                    for (relation, output) in &descriptor.outputs {
                        charge.grow(relation.len().saturating_add(1024))?;
                        pending.push(CompletedView::new(
                            relation.clone(),
                            [logical].into(),
                            output.rows,
                        )?);
                    }
                    let mut selections = BTreeMap::new();
                    let mut mapping_charges = Vec::new();
                    let mapping_reader =
                        crate::NativeReader::private(self.client.clone()).with_budget(budget);
                    let mapping_reader = if let Some(flag) = self.read_cancellation()? {
                        mapping_reader.with_read_cancellation(flag)
                    } else {
                        mapping_reader
                    };
                    let mut observed = BTreeMap::new();
                    let mapping_phase =
                        crate::phase::Phase::begin("native_product_mapping_discovery");
                    let mapping_result = async {
                        while let Some(view) = pending.pop() {
                            if selections.contains_key(&view.identity) {
                                continue;
                            }
                            self.registered_view(&view).await?;
                            let (mappings, mapping_charge) = self
                                .selected_view_mappings_read(
                                    &mapping_reader,
                                    &view,
                                    budget,
                                    &mut observed,
                                    false,
                                )
                                .await?;
                            for mapping in &mappings {
                                if mapping.view != view.identity
                                    && !selections.contains_key(&mapping.view)
                                {
                                    let dependency = self
                                        .known_views
                                        .lock()
                                        .map_err(|_| {
                                            ModelError::Conflict(
                                                "retained product dependency inventory",
                                            )
                                        })?
                                        .get(&mapping.view)
                                        .cloned()
                                        .ok_or(ModelError::Conflict(
                                            "retained product dependency descriptor absent",
                                        ))?;
                                    charge.grow(
                                        dependency
                                            .relation
                                            .len()
                                            .saturating_add(
                                                dependency.contributions.len().saturating_mul(128),
                                            )
                                            .saturating_add(1024),
                                    )?;
                                    pending.push(dependency);
                                }
                            }
                            mapping_charges.push(mapping_charge);
                            charge.grow(2048)?;
                            selections.insert(
                                view.identity,
                                lctx_native::completed_state::ViewSelection { view, mappings },
                            );
                        }
                        Ok(())
                    }
                    .await;
                    mapping_phase.finish_result(&mapping_result);
                    let mut mapping_terminal = d::completion::Completion::default();
                    mapping_terminal.step(
                        "retained product mapping reader close",
                        mapping_reader.close().await,
                    );
                    d::completion::complete(mapping_result, mapping_terminal)?;
                    drop(observed);
                    let selections = selections.into_values().collect::<Vec<_>>();
                    let mut scope = self.view_audit.lock().await;
                    if scope.is_none() {
                        *scope = Some(
                            lctx_native::installed_view::InstallationAudit::new(
                                self.client.clone(),
                                budget,
                            )
                            .await?,
                        );
                    }
                    let audit = scope
                        .as_ref()
                        .filter(|audit| audit.shares_pool(budget))
                        .cloned();
                    drop(scope);
                    let boundary = match audit {
                        Some(audit) => {
                            lctx_native::completed_state::complete_retained_content_scoped(
                                self.client.clone(),
                                self.attempt,
                                &[],
                                &selections,
                                budget,
                                &audit,
                            )
                            .await?
                        }
                        None => {
                            lctx_native::completed_state::complete_retained_content(
                                self.client.clone(),
                                self.attempt,
                                &[],
                                &selections,
                                budget,
                            )
                            .await?
                        }
                    };
                    let product = RecordId::new(
                        "native_product",
                        format!("{}_{}", request.hex(), self.attempt.hex()),
                    );
                    let mut row = Object::new();
                    row.insert("id", product.clone());
                    row.insert("request", request.hex());
                    row.insert("released", false);
                    row.insert(
                        "contribution",
                        RecordId::new("compiler_contribution", contribution.hex()),
                    );
                    crate::control::ensure_rows(
                        &self.client,
                        Some(self.attempt),
                        vec![Value::Object(row)],
                    )
                    .await?;
                    let reader =
                        crate::NativeReader::private(self.client.clone()).with_budget(budget);
                    let result = async {
                        let address = boundary.completion().record()?;
                        let mut observed = crate::control::inspect_expected_window_read(
                            &reader,
                            std::slice::from_ref(&address),
                            budget,
                        )
                        .await?;
                        let expected = observed.take_required(&address)?;
                        observed.finish()?;
                        if expected.incarnation() != boundary.incarnation()
                            || expected.row() != &boundary.completion().value(true)?
                        {
                            return Err(ModelError::Conflict(
                                "retained product completed boundary changed",
                            ));
                        }
                        crate::control::hold_expected(
                            &self.client,
                            self.attempt,
                            product.clone(),
                            expected,
                        )
                        .await?;
                        crate::control::hold_many(
                            &self.client,
                            Some(self.attempt),
                            vec![
                                (
                                    product.clone(),
                                    RecordId::new("compiler_contribution", contribution.hex()),
                                ),
                                (RecordId::new("native_attempt", self.attempt.hex()), product),
                            ],
                        )
                        .await
                    }
                    .await;
                    let mut terminal = d::completion::Completion::default();
                    let closed = reader.close().await;
                    if closed.is_err()
                        || result
                            .as_ref()
                            .err()
                            .is_some_and(|error: &ModelError| !error.permits_storage_cleanup())
                    {
                        boundary.retain_unknown();
                    }
                    terminal.step("retained product receiving reader close", closed);
                    terminal.step(
                        "retained product independent boundary pin close",
                        boundary.close().await,
                    );
                    d::completion::complete(result, terminal)
                })
                .await;
            phase.finish_result(&result);
            lease.finish_with(result)
        })
    }
    /// End the named request's warm retention after its original producer ended.
    /// Nomination is durable before release; this never advances native collection.
    pub async fn release_product_identity(
        self: &Arc<Self>,
        request: ContentHash,
        source_attempt: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<Option<ContentHash>, ModelError> {
        let lease = self.admit_mutation("release retained product", false)?;
        let result = lease
            .within(crate::control::release_retained_product(
                self.client.clone(),
                request,
                source_attempt,
                budget,
            ))
            .await;
        lease.finish_with(result)
    }
    /// Exact currently retained product roots. Enumerate the attempt's indexed hold
    /// incidence one page at a time, never global matching contribution history.
    pub async fn retained_product_views(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<(Vec<CompletedView>, d::charged::StateCharge), ModelError> {
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let result=async {
            let mut charge=d::charged::StateCharge::new(budget,"retained product output root inventory");
            let mut views=BTreeMap::new();let mut after=Value::None;
            loop {
                let mut variables=Variables::new();variables.insert("owner",RecordId::new("native_attempt",self.attempt.hex()));variables.insert("after",after.clone());
                let rows:Vec<Value>=reader.query_native("SELECT id,object.* AS product FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND record::table(object)='native_product' AND ($after=NONE OR id>$after) ORDER BY id LIMIT 1",variables).await?;
                if rows.is_empty(){break;}
                let [row]=rows.as_slice() else{return Err(ModelError::Conflict("retained product inventory page bound"));};
                let row=value_object(row).ok_or(ModelError::Schema("retained product inventory object"))?;after=row.get("id").cloned().ok_or(ModelError::Schema("retained product hold identity"))?;
                let product=row.get("product").and_then(value_object).ok_or(ModelError::Schema("retained product inventory source"))?;
                let Some(Value::RecordId(physical))=product.get("contribution") else{return Err(ModelError::Schema("retained product exact contributor"));};
                let mut variables=Variables::new();variables.insert("contribution",physical.clone());
                let descriptors:Vec<Value>=reader.query_native("SELECT * FROM $contribution",variables).await?;
                let [row]=descriptors.as_slice() else{return Err(ModelError::Conflict("retained product contributor cardinality"));};
                validate_state_row("compiler_contribution",row)?;
                let _decode=budget.reserve("retained product descriptor decode",crate::loader::native_bytes(row).saturating_mul(32).saturating_add(65536))?;
                let descriptor:CompletedContribution=decode_descriptor(row)?;let logical=descriptor.identity()?;
                for (relation,output) in &descriptor.outputs {
                    let view=CompletedView::new(relation.clone(),[logical].into(),output.rows)?;
                    charge.grow(view.relation.capacity().saturating_add(512))?;views.insert(view.identity,view);
                }
            }
            Ok((views.into_values().collect(),charge))
        }.await;
        let mut terminal = d::completion::Completion::default();
        terminal.step("retained product root reader close", reader.close().await);
        d::completion::complete(result, terminal)
    }
    pub async fn attach_retained_product(
        self: &Arc<Self>,
        request: ContentHash,
        spec: &ContributionSpec,
        budget: &ResourceBudget,
    ) -> Result<
        Option<(
            ContentHash,
            BTreeMap<String, CompletedView>,
            CompletedContribution,
        )>,
        ModelError,
    > {
        let Some(candidate) = self.hold_retained_product(request, spec, budget).await? else {
            return Ok(None);
        };
        self.attach_held_product(candidate, budget).await.map(Some)
    }
    pub async fn hold_retained_product(
        self: &Arc<Self>,
        request: ContentHash,
        spec: &ContributionSpec,
        budget: &ResourceBudget,
    ) -> Result<Option<HeldCandidate>, ModelError> {
        let lease = self.admit_mutation("hold retained product candidate", true)?;
        let result = lease
            .within(self.hold_retained_product_inner(request, spec, budget))
            .await;
        lease.finish_with(result)
    }
    async fn hold_retained_product_inner(
        self: &Arc<Self>,
        request: ContentHash,
        spec: &ContributionSpec,
        budget: &ResourceBudget,
    ) -> Result<Option<HeldCandidate>, ModelError> {
        self.check_failed()?;
        let mut lookup = Variables::new();
        lookup.insert("request", request.hex());
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let discovered=async {
            let rows:Vec<Value>=reader.query_native("SELECT contribution.* AS stored FROM native_product WHERE request=$request AND released=false AND contribution.completed=true AND contribution.attempt.admitted=true ORDER BY id LIMIT 1",lookup).await?;
            let charge=budget.reserve("retained candidate descriptor preflight",rows.iter().map(crate::loader::native_bytes).fold(0usize,usize::saturating_add).saturating_mul(32).saturating_add(65536))?;
            Ok::<_,ModelError>((rows,charge))
        }.await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "retained candidate discovery reader close",
            reader.close().await,
        );
        let (rows, _discovery_charge) = d::completion::complete(discovered, terminal)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let source = value_object(row)
            .and_then(|row| row.get("stored"))
            .ok_or(ModelError::Schema("retained admitted contribution"))?;
        validate_state_row("compiler_contribution", source)?;
        let descriptor: CompletedContribution = decode_descriptor(source)?;
        require_retained_request_spec(&descriptor.spec, spec)?;
        let source_attempt = match value_object(source).and_then(|row| row.get("attempt")) {
            Some(Value::RecordId(record)) => record.clone(),
            _ => return Err(ModelError::Schema("retained contribution source attempt")),
        };
        let RecordIdKey::String(attempt_key) = &source_attempt.key else {
            return Err(ModelError::Schema("retained source attempt key"));
        };
        let source_attempt = ContentHash(
            hex::decode(attempt_key)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("retained source attempt width"))?,
        );
        let completed = Arc::new(
            lctx_native::completed_state::hold_admitted_completed_state_for_attempt(
                self.client.clone(),
                source_attempt,
                budget,
            )
            .await?,
        );
        // Read-only source closure inspection precedes every receiving-attempt effect.
        for input in &spec.inputs {
            let installed = completed.inventory().installed(input.view())?;
            if installed.view.relation != input.relation()
                || input.rows() < 0
                || installed.view.rows != input.rows() as u64
            {
                return Err(ModelError::Conflict(
                    "retained dependency exact source metadata",
                ));
            }
        }
        let source_record = match value_object(source).and_then(|row| row.get("id")) {
            Some(Value::RecordId(record)) => record.clone(),
            _ => return Err(ModelError::Schema("retained contribution record")),
        };
        let RecordIdKey::String(key) = &source_record.key else {
            return Err(ModelError::Schema("retained contribution physical key"));
        };
        let physical = ContentHash(
            hex::decode(key)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("retained contribution physical width"))?,
        );
        let expected = crate::control::inspect_expected(&self.client, source_record, budget)
            .await?
            .ok_or(ModelError::Conflict(
                "retained contribution disappeared before hold",
            ))?;
        if expected.row() != source {
            return Err(ModelError::Conflict(
                "retained contribution changed after preflight",
            ));
        }
        // The protected source completion must explicitly retain this exact selected copy.
        let logical = descriptor.identity()?;
        let selected = completed.inventory().references().iter().any(|reference| {
            completed
                .inventory()
                .installed(reference.logical_view)
                .is_ok_and(|installed| {
                    installed.mappings.iter().any(|mapping| {
                        mapping.logical == logical
                            && mapping.physical == physical
                            && mapping.incarnation == expected.incarnation()
                    })
                })
        });
        if !selected {
            return Err(ModelError::Conflict(
                "retained candidate absent from protected physical inventory",
            ));
        }
        let mut outputs = BTreeMap::new();
        for (relation, output) in &descriptor.outputs {
            let view = CompletedView::new(relation.clone(), [logical].into(), output.rows)?;
            let held = Arc::new(
                lctx_native::installed_view::hold_frozen_view(
                    self.client.clone(),
                    completed.clone(),
                    view.identity,
                    budget,
                )
                .await?,
            );
            if held.completion().view != view
                || held
                    .completion()
                    .mappings
                    .iter()
                    .find(|mapping| mapping.view == view.identity && mapping.logical == logical)
                    .is_none_or(|mapping| {
                        mapping.physical != physical
                            || mapping.incarnation != expected.incarnation()
                    })
            {
                return Err(ModelError::Conflict(
                    "retained exact singleton physical output",
                ));
            }
            outputs.insert(relation.clone(), held);
        }
        Ok(Some(HeldCandidate {
            owner: self.admission.clone(),
            physical,
            descriptor,
            expected,
            completed: Arc::new(CandidateCompletion::Admitted(completed)),
            outputs,
            _admission: None,
            _charge: _discovery_charge,
        }))
    }

    /// Acquire and independently admit one exact current terminal product. The
    /// captured request comes from the current producer, not persisted artifact flags.
    pub async fn hold_captured_product(
        self: &Arc<Self>,
        request: &d::producer_contract::request::CapturedRequest,
        model: Arc<ValidatedModel>,
        profile: d::stages::Profile,
        budget: &ResourceBudget,
    ) -> Result<Option<HeldCandidate>, ModelError> {
        let lease = self.admit_mutation("hold captured terminal product", true)?;
        let mut acquired = None;
        let result=lease.within(async {
            self.check_failed()?;
            if model.digest()!=request.contribution().model{return Err(ModelError::Conflict("terminal candidate current model"));}
            request.contribution().validate_profile(profile)?;
            let identity=request.identity()?;
            let reader=crate::NativeReader::private(self.client.clone()).with_budget(budget);
            let mut variables=Variables::new();variables.insert("request",identity.hex());variables.insert("model",model.digest().hex());
            // Current model/terminal discovery is eligibility only. Independent
            // protected contributor and actual request checks establish admission.
            let discovered=reader.query_native::<Vec<Object>>("SELECT id FROM native_product WHERE request=$request AND released=false AND contribution.model=$model AND contribution.completed=true AND contribution.attempt.state IN ['closed','abandoned','maintenance_fenced','frozen'] ORDER BY id LIMIT 1",variables).await;
            let mut terminal=d::completion::Completion::default();terminal.step("terminal candidate discovery reader close",reader.close().await);
            let rows=d::completion::complete(discovered,terminal)?;
            let Some(row)=rows.first()else{return Ok(None);};
            let Some(Value::RecordId(product))=row.get("id")else{return Err(ModelError::Schema("terminal candidate discovered product"));};
            let held=Arc::new(lctx_native::control::hold_terminal_product(self.client.clone(),product.clone(),identity,budget).await?);
            acquired=Some(held.clone());
            let descriptor=held.descriptor();
            require_retained_request_spec(&descriptor.spec,request.contribution())?;
            if descriptor.outcome!=0 || descriptor.spec.producer==d::producer_contract::facts::ASSEMBLE{return Ok(None);}
            let charge=budget.reserve("terminal candidate retained descriptor",serde_json::to_vec(descriptor).map_err(ModelError::codec)?.len().saturating_mul(8).saturating_add(65536))?;
            let admission=crate::admission::candidate::admit(self.client.clone(),self.namespace.clone(),self.database.clone(),held.clone(),request,model,profile,budget).await?;
            let descriptor=held.descriptor().clone();
            let expected=crate::control::inspect_expected(&self.client,held.contribution().record().clone(),budget).await?.ok_or(ModelError::Conflict("terminal admitted contributor disappeared"))?;
            if expected.incarnation()!=held.contribution().incarnation() || expected.row()!=held.contribution().row(){return Err(ModelError::Conflict("terminal admitted contributor changed"));}
            let RecordIdKey::String(key)=&expected.record().key else{return Err(ModelError::Schema("terminal candidate physical key"));};
            let physical=ContentHash(hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("terminal candidate physical width"))?);
            let logical=descriptor.identity()?;let mut outputs=BTreeMap::new();
            for (relation,output) in &descriptor.outputs {
                let view=CompletedView::new(relation.clone(),[logical].into(),output.rows)?;
                outputs.insert(relation.clone(),held.views().get(&view.identity).cloned().ok_or(ModelError::Conflict("terminal candidate output absent"))?);
            }
            Ok(Some(HeldCandidate{owner:self.admission.clone(),physical,descriptor,expected,completed:Arc::new(CandidateCompletion::Terminal(held)),outputs,_admission:Some(admission),_charge:charge}))
        }).await;
        let result = if !matches!(&result, Ok(Some(_))) {
            if let Some(owner) = acquired.take() {
                if result
                    .as_ref()
                    .err()
                    .is_some_and(|error| !error.permits_storage_cleanup())
                {
                    owner.retain_unknown();
                    result
                } else {
                    let mut terminal = d::completion::Completion::default();
                    terminal.step(
                        "rejected terminal candidate source close",
                        owner.close().await,
                    );
                    d::completion::complete(result, terminal)
                }
            } else {
                result
            }
        } else {
            result
        };
        lease.finish_with(result)
    }
    pub(crate) fn terminal_read_owner(
        client: Arc<Surreal<Client>>,
        namespace: d::serving::Name,
        database: d::serving::Name,
        owner: &lctx_native::control::HeldTerminalProduct,
        budget: &ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let mut store = Self::from_existing(client, namespace, database);
        let fresh =
            Arc::get_mut(&mut store).ok_or(ModelError::Conflict("terminal fresh read owner"))?;
        fresh.generation = owner.generation();
        fresh.read_only.store(true, Ordering::Release);
        for source in owner.views().values() {
            let installed = source.completion();
            fresh.remember_view_mapping(&installed.view, &installed.mappings, budget)?;
            fresh.remember_view(&installed.view)?;
            fresh
                .installed_views
                .get_mut()
                .map_err(|_| ModelError::Conflict("terminal exact read owner"))?
                .insert(installed.view.identity, source.clone());
        }
        Ok(store)
    }

    async fn receiver_root_under_gate(
        self: &Arc<Self>,
        held: Arc<lctx_native::installed_view::HeldInstalledView>,
        budget: &ResourceBudget,
    ) -> Result<Arc<lctx_native::installed_view::HeldInstalledView>, ModelError> {
        let view = held.completion().view.identity;
        if self
            .known_views
            .lock()
            .map_err(|_| ModelError::Conflict("receiver known root descriptors"))?
            .get(&view)
            .is_some_and(|known| *known != held.completion().view)
        {
            return Err(ModelError::Conflict(
                "receiver retained root descriptor collision",
            ));
        }
        let existing = self
            .installed_views
            .lock()
            .map_err(|_| ModelError::Conflict("receiver protected root"))?
            .get(&view)
            .cloned();
        if let Some(existing) = existing {
            let incoming = lctx_native::completed_transport::eligibility_selected(
                self.client.clone(),
                std::slice::from_ref(&held),
                budget,
            )
            .await?;
            let receiving = lctx_native::completed_transport::eligibility_selected(
                self.client.clone(),
                std::slice::from_ref(&existing),
                budget,
            )
            .await?;
            if incoming != receiving {
                return Err(ModelError::Conflict(
                    "receiver retained root eligibility collision",
                ));
            }
            return Ok(existing);
        } else {
            if self
                .view_mappings
                .lock()
                .map_err(|_| ModelError::Conflict("receiver exact mappings"))?
                .contains_key(&view)
            {
                let receiving = self
                    .install_view_under_gate(&held.completion().view, budget)
                    .await?;
                let incoming = lctx_native::completed_transport::eligibility_selected(
                    self.client.clone(),
                    std::slice::from_ref(&held),
                    budget,
                )
                .await?;
                let previous = lctx_native::completed_transport::eligibility_selected(
                    self.client.clone(),
                    std::slice::from_ref(&receiving),
                    budget,
                )
                .await?;
                if incoming != previous {
                    return Err(ModelError::Conflict(
                        "receiver retained root eligibility collision",
                    ));
                }
                return Ok(receiving);
            } else {
                return Ok(held);
            }
        }
    }
    pub async fn attach_held_product(
        self: &Arc<Self>,
        candidate: HeldCandidate,
        budget: &ResourceBudget,
    ) -> Result<
        (
            ContentHash,
            BTreeMap<String, CompletedView>,
            CompletedContribution,
        ),
        ModelError,
    > {
        if !Arc::ptr_eq(&candidate.owner, &self.admission) {
            let mut terminal = d::completion::Completion::default();
            terminal.step("wrong receiver candidate close", candidate.close().await);
            return d::completion::complete(
                Err(ModelError::Conflict("retained candidate receiver owner")),
                terminal,
            );
        }
        let lease = match self.admit_mutation("attach held product", true) {
            Ok(lease) => lease,
            Err(error) => {
                if !error.permits_storage_cleanup() {
                    return Err(error);
                }
                let mut terminal = d::completion::Completion::default();
                terminal.step("refused receiver candidate close", candidate.close().await);
                return d::completion::complete(Err(error), terminal);
            }
        };
        let HeldCandidate {
            physical,
            descriptor,
            expected,
            completed,
            outputs,
            _admission,
            _charge: _candidate_charge,
            ..
        } = candidate;
        let closing = completed.clone();
        let attached_committed = AtomicBool::new(false);
        let committed = &attached_committed;
        let result: Result<_, ModelError> = lease
            .within(async move {
                self.check_failed()?;
                if let CandidateCompletion::Terminal(owner) = completed.as_ref() {
                    _admission
                        .as_ref()
                        .ok_or(ModelError::Conflict(
                            "terminal candidate lacks executed admission",
                        ))?
                        .require(&descriptor)?;
                    owner.ensure_current(budget).await?;
                }
                // Source acquisition and every receiver compatibility check precede
                // the single native incoming-reference transaction.
                let mut inventory_charge = d::charged::StateCharge::new(
                    budget,
                    "held product attachment source inventory",
                );
                inventory_charge.grow(
                    outputs
                        .values()
                        .map(|source| source.completion().mappings.len().saturating_add(1))
                        .sum::<usize>()
                        .saturating_mul(256)
                        .saturating_add(1024),
                )?;
                let _root_gates = self
                    .root_gates(
                        outputs
                            .values()
                            .map(|source| source.completion().view.identity),
                        budget,
                    )
                    .await?;
                let mut sources = Vec::with_capacity(outputs.len());
                let mut original_candidate_root = None;
                let candidate_logical = descriptor.identity()?;
                for source in outputs.values() {
                    let view = source.completion().view.identity;
                    let held = completed
                        .held_view(self.client.clone(), view, budget)
                        .await?;
                    if held.completion().mappings.iter().any(|mapping| {
                        mapping.physical == physical
                            && mapping.logical == candidate_logical
                            && mapping.incarnation == expected.incarnation()
                    }) {
                        original_candidate_root = Some(held.clone());
                    }
                    sources.push(self.receiver_root_under_gate(held, budget).await?);
                }
                // Reserve each receiver mapping before the native effect. Existing
                // equivalent selections keep their own exact copies independently.
                let mut pending_mappings = Vec::with_capacity(sources.len());
                for source in &sources {
                    let installed = source.completion();
                    let exists = self
                        .view_mappings
                        .lock()
                        .map_err(|_| ModelError::Conflict("receiver exact mappings"))?
                        .contains_key(&installed.view.identity);
                    if !exists {
                        let rows = installed.mappings.iter();
                        let count = rows.clone().count();
                        let charge = budget.reserve(
                            "retained exact per-view mappings",
                            count
                                .saturating_mul(std::mem::size_of::<
                                    lctx_native::installed_view::ContributorRealization,
                                >())
                                .saturating_add(256),
                        )?;
                        let mut selected = Vec::with_capacity(count);
                        selected.extend(rows.cloned());
                        pending_mappings.push((
                            installed.view.identity,
                            selected,
                            Arc::new(charge),
                        ));
                    }
                }
                let logical = descriptor.identity()?;
                let registered = RegisteredContribution {
                    spec: descriptor.spec.clone(),
                    ownership: ContributionOwnership::Completed,
                    _transport_charge: None,
                };
                let mut views = BTreeMap::new();
                for (relation, output) in &descriptor.outputs {
                    views.insert(
                        relation.clone(),
                        CompletedView::new(relation.clone(), [logical].into(), output.rows)?,
                    );
                }
                // Check every local owner before reference attachment too. These locks
                // are released before awaiting the fixed native transaction.
                drop(
                    self.specifications
                        .lock()
                        .map_err(|_| ModelError::Conflict("native contribution owner"))?,
                );
                drop(
                    self.known_contributors
                        .lock()
                        .map_err(|_| ModelError::Conflict("native contributor candidates"))?,
                );
                drop(
                    self.installed_views
                        .lock()
                        .map_err(|_| ModelError::Conflict("attached exact installed owner"))?,
                );
                let original_candidate_root = original_candidate_root.ok_or(
                    ModelError::Conflict("retained candidate lacks original root correspondence"),
                )?;
                let attached = lctx_native::installed_view::attach_live_views_retaining(
                    self.client.clone(),
                    self.attempt,
                    &sources,
                    vec![(expected, original_candidate_root)],
                    budget,
                )
                .await?;
                committed.store(true, Ordering::Release);
                for (view, rows, charge) in pending_mappings {
                    self.view_mappings
                        .lock()
                        .map_err(|_| ModelError::Conflict("receiver exact mappings"))?
                        .insert(view, (rows, charge));
                }
                for attached in attached {
                    let installed = attached.completion();
                    self.remember_view(&installed.view)?;
                    self.installed_views
                        .lock()
                        .map_err(|_| ModelError::Conflict("attached exact installed owner"))?
                        .insert(installed.view.identity, Arc::new(attached));
                }
                self.specifications
                    .lock()
                    .map_err(|_| ModelError::Conflict("native contribution owner"))?
                    .insert(physical, registered);
                self.remember_contributor(logical, physical)?;
                Ok((physical, views, descriptor))
            })
            .await;
        let result = if result
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            closing.retain_unknown();
            result
        } else {
            let mut terminal = d::completion::Completion::default();
            // The receiver has earned independent holds before source release.
            // A later source close error must retain the committed attach identity.
            if attached_committed.load(Ordering::Acquire) {
                terminal.committed("retained product attached", physical.hex());
            }
            terminal.step("attached candidate source close", closing.close().await);
            d::completion::complete(result, terminal)
        };
        lease.finish_with(result)
    }
    async fn hold_contribution_inputs(
        &self,
        contribution: ContentHash,
        spec: &ContributionSpec,
    ) -> Result<(), ModelError> {
        let owner = RecordId::new("compiler_contribution", contribution.hex());
        let mut references = vec![(
            RecordId::new("native_attempt", self.attempt.hex()),
            owner.clone(),
        )];
        references.extend(spec.inputs.iter().map(|input| {
            (
                owner.clone(),
                RecordId::new("compiler_view", input.view().hex()),
            )
        }));
        crate::control::hold_many(&self.client, Some(self.attempt), references).await
    }
    pub fn producing_scope(
        self: &Arc<Self>,
        specification: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<ProducingScope, ModelError> {
        self.check_failed()?;
        let contribution = attempt_contribution(self.attempt, specification);
        let mut scopes = self
            .producing_scopes
            .lock()
            .map_err(|_| ModelError::Conflict("native producing scopes"))?;
        let scope = match scopes.entry(contribution) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.get().clone(),
            std::collections::btree_map::Entry::Vacant(entry) => entry
                .insert(Arc::new(ProducingAdmission {
                    state: Mutex::default(),
                    changed: Notify::new(),
                    _retained: budget.reserve("native-producing-scope", 1024)?,
                }))
                .clone(),
        };
        Ok(ProducingScope {
            store: self.clone(),
            contribution,
            scope,
        })
    }
    fn producing_binding(&self) -> Result<Option<ProducingBinding>, ModelError> {
        let binding = PRODUCING_WORK.try_with(Clone::clone).ok();
        if binding
            .as_ref()
            .is_some_and(|binding| !Arc::ptr_eq(&binding.owner, &self.admission))
        {
            return Err(ModelError::Conflict("foreign native producing store"));
        }
        Ok(binding)
    }
    fn producing_lease(&self) -> Result<Option<ProducingLease>, ModelError> {
        // Only an explicit run/stream admits a new scope root. Work observed through
        // its binding is a descendant and may finish after root admission closes.
        self.producing_binding()?
            .map(|binding| binding.scope.admit(true))
            .transpose()
    }
    pub async fn begin(
        config: &RuntimeConfig,
        frontier: Frontier,
    ) -> Result<Arc<Self>, ModelError> {
        let phase = crate::phase::Phase::begin("native_setup");
        let result = Self::begin_inner(config, frontier).await;
        phase.finish_result(&result);
        result
    }
    async fn begin_inner(
        config: &RuntimeConfig,
        frontier: Frontier,
    ) -> Result<Arc<Self>, ModelError> {
        let (signal_stop, read_cancellation, signals) = native_signal_owners();
        signals
            .as_ref()
            .map_err(|error| ModelError::SharedCause(error.clone()))?;
        if signal_stop.stopped() {
            return Err(ModelError::Invalid("native operation cancelled".into()));
        }
        let database = config.database.clone();
        let client = check_installation(config).await?;
        let attempt = crate::control::with_retry_cancellation(
            signal_stop.clone(),
            crate::control::issue_attempt(&client, config.service_generation),
        )
        .await?;
        Ok(Arc::new(Self {
            client,
            database,
            attempt,
            generation: config.service_generation,
            published_bindings: Mutex::new(None),
            published_completion: Mutex::new(None),
            physical_completed: Mutex::new(None),
            read_only: AtomicBool::new(false),
            read_cancellation,
            signal_stop,
            signals,
            owns_session: true,
            original_ingress: Default::default(),
            namespace: config.namespace.clone(),
            endpoint: Some(config.endpoint.clone()),
            specifications: Mutex::default(),
            known_views: Mutex::default(),
            view_mappings: Mutex::default(),
            installed_views: Mutex::default(),
            view_preparations: Mutex::default(),
            product_retention_gates: Mutex::default(),
            view_audit: tokio::sync::Mutex::new(None),
            known_contributors: Mutex::default(),
            failed: AtomicBool::new(false),
            sealed: AtomicBool::new(false),
            seal_started: AtomicBool::new(false),
            preparation: Mutex::default(),
            membership_preparations: Mutex::default(),
            producing_scopes: Mutex::default(),
            admission: Arc::new(OperationAdmission::default()),
            runtime: tokio::runtime::Handle::current(),
            frontier: Mutex::new(frontier),
        }))
    }
    /// Bind an explicitly owned database session for cold transport/reconciliation.
    /// This does not create a published handle or infer a current relation view.
    pub fn from_existing(
        client: Arc<Surreal<Client>>,
        namespace: d::serving::Name,
        database: d::serving::Name,
    ) -> Arc<Self> {
        let (signal_stop, read_cancellation, signals) = native_signal_owners();
        Arc::new(Self {
            client,
            database,
            attempt: crate::control::fresh_identity("detached-native-attempt")
                .expect("system clock"),
            generation: ContentHash::of(b"detached-unadmitted"),
            published_bindings: Mutex::new(None),
            published_completion: Mutex::new(None),
            physical_completed: Mutex::new(None),
            read_only: AtomicBool::new(false),
            read_cancellation,
            signal_stop,
            signals,
            owns_session: false,
            original_ingress: Default::default(),
            namespace,
            endpoint: None,
            specifications: Mutex::default(),
            known_views: Mutex::default(),
            view_mappings: Mutex::default(),
            installed_views: Mutex::default(),
            view_preparations: Mutex::default(),
            product_retention_gates: Mutex::default(),
            view_audit: tokio::sync::Mutex::new(None),
            known_contributors: Mutex::default(),
            failed: AtomicBool::new(false),
            sealed: AtomicBool::new(false),
            seal_started: AtomicBool::new(false),
            preparation: Mutex::default(),
            membership_preparations: Mutex::default(),
            producing_scopes: Mutex::default(),
            admission: Arc::new(OperationAdmission::default()),
            runtime: tokio::runtime::Handle::current(),
            frontier: Mutex::new(Frontier::Facts),
        })
    }
    pub fn set_frontier(&self, frontier: Frontier) -> Result<(), ModelError> {
        let lease = self.admit_mutation("set_frontier", false)?;
        let result = (|| {
            if !self
                .specifications
                .lock()
                .map_err(|_| ModelError::Conflict("contribution owner"))?
                .is_empty()
            {
                return Err(ModelError::Conflict("frontier after compiler writes"));
            }
            *self
                .frontier
                .lock()
                .map_err(|_| ModelError::Conflict("frontier owner"))? = frontier;
            Ok(())
        })();
        lease.finish_with(result)
    }
    pub async fn completed_contribution(
        self: &Arc<Self>,
        id: ContentHash,
    ) -> Result<CompletedContribution, ModelError> {
        let lease = self.admit(false, "completed_contribution", false)?;
        let result = lease
            .within(async {
                let mut bindings = Variables::new();
                bindings.insert("owner", RecordId::new("compiler_contribution", id.hex()));
                let mut rows = self.track_rows(NativeRows::new(
                    self.client
                        .query("SELECT * FROM $owner WHERE completed=true")
                        .bind(bindings)
                        .stream_items()
                        .map_err(ModelError::codec)?,
                    1,
                )?)?;
                let row = rows.next().await?.ok_or(ModelError::Conflict(
                    "missing completed native contribution",
                ))?;
                validate_state_row("compiler_contribution", &row)?;
                if rows.next().await?.is_some() {
                    return Err(ModelError::Conflict(
                        "duplicate completed native contribution",
                    ));
                }
                let Some(Value::Bytes(bytes)) =
                    value_object(&row).and_then(|row| row.get("descriptor"))
                else {
                    return Err(ModelError::Schema(
                        "completed native contribution descriptor",
                    ));
                };
                let descriptor: CompletedContribution =
                    serde_json::from_slice(bytes).map_err(ModelError::codec)?;
                if attempt_contribution(self.attempt, descriptor.spec.identity()?) != id
                    && !self
                        .known_contributors
                        .lock()
                        .map_err(|_| ModelError::Conflict("native contributor owner"))?
                        .values()
                        .any(|known| *known == id)
                    && !self
                        .specifications
                        .lock()
                        .map_err(|_| ModelError::Conflict("native copy registration"))?
                        .contains_key(&id)
                {
                    return Err(ModelError::Conflict(
                        "completed native contribution identity",
                    ));
                }
                descriptor.identity()?;
                Ok(descriptor)
            })
            .await;
        lease.finish_with(result)
    }
    /// Permanently close only compiler-content mutation. Ordinary immutable reads remain open.
    pub fn freeze_content<'a>(
        self: &'a Arc<Self>,
        budget: &'a ResourceBudget,
    ) -> futures::future::BoxFuture<'a, Result<FinalizationInventory, ModelError>> {
        Box::pin(async move {
            let lease = self.admit(false, "freeze_content", true)?;
            let phase = crate::phase::Phase::begin("native_content_freeze");
            let result = lease.within(async {
            self.close_content_lane().await?;
            self.check_failed()?;
            let mut pending = self.track_rows(NativeRows::new(
                self.client
                    .query(
                        "SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$attempt AND completed=false LIMIT 1",
                    ).bind(("attempt",RecordId::new("native_attempt",self.attempt.hex())))
                    .stream_items()
                    .map_err(ModelError::codec)?,
                1,
            )?)?;
            let has_pending = pending.next().await?.is_some();
            while pending.next().await?.is_some() {}
            if has_pending {
                return Err(ModelError::Conflict(
                    "pending native content at final freeze",
                ));
            }
            // All content mutation owners are now quiescent, and no contribution remains
            // open. Attempt-global replay cannot impose order on unrelated live producers.
            let already_frozen=self.admission.state.lock().map_err(|_|ModelError::Conflict("native operation admission"))?.content_ready;
            let cancellation=self.read_cancellation()?;
            let reader=crate::NativeReader::private(self.client.clone()).with_budget(budget);
            let reader=if let Some(flag)=cancellation {reader.with_read_cancellation(flag)} else {reader};
            let replay=if already_frozen {
                lctx_native::originals::validate_pending(&reader,self.attempt,budget).await
            } else {
                lctx_native::originals::complete_pending(&reader,self.attempt,budget).await
            };
            let mut completion=d::completion::Completion::default();
            completion.step("original freeze reader close",reader.close().await);
            d::completion::complete(replay,completion)?;
            Ok(FinalizationInventory {
                contributions: self.contributions_inner(budget).await?,
                bindings: self.bindings().await?,
            })
        })
        .await;
            let result = lease.finish_with(result);
            if result.is_ok() {
                self.admission
                    .state
                    .lock()
                    .map_err(|_| ModelError::Conflict("native operation admission"))?
                    .content_ready = true;
            }
            phase.finish_result(&result);
            result
        })
    }
    async fn close_content_lane(&self) -> Result<(), ModelError> {
        self.admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?
            .content_closed = true;
        loop {
            let notification = self.admission.changed.notified();
            tokio::pin!(notification);
            notification.as_mut().enable();
            if self
                .admission
                .state
                .lock()
                .map_err(|_| ModelError::Conflict("native operation admission"))?
                .mutations
                == 0
            {
                return Ok(());
            }
            notification.await;
        }
    }
    fn check_frozen(&self) -> Result<(), ModelError> {
        self.check()?;
        self.check_frozen_content()
    }
    fn check_frozen_content(&self) -> Result<(), ModelError> {
        self.check_failed()?;
        let state = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?;
        if !state.content_ready || state.mutations != 0 {
            return Err(ModelError::Conflict("native content is not frozen"));
        }
        Ok(())
    }
    /// A private effect session must address the actual attempt, not a same-named database
    /// on another server. Cold reader sessions carry no publication location authority.
    pub fn check_publication_target(&self, config: &RuntimeConfig) -> Result<(), ModelError> {
        if self.endpoint.as_deref() != Some(config.endpoint.as_str())
            || self.namespace != config.namespace
            || self.database != config.database
            || self.generation != config.service_generation
        {
            return Err(ModelError::Conflict("native publication target"));
        }
        Ok(())
    }
    pub fn begin_derived_operation(
        &self,
        operation: &'static str,
    ) -> Result<OperationLease, ModelError> {
        self.check_frozen()?;
        self.admit(false, operation, true)
    }
    pub async fn begin_finalization(&self) -> Result<OperationLease, ModelError> {
        self.check_failed()?;
        if !self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?
            .content_ready
        {
            return Err(ModelError::Conflict("native content is not frozen"));
        }
        // Closing/draining is retryable when its waiter is interrupted. Once the private
        // lease is issued, no second seal can run; a dropped lease records uncertainty.
        self.end_writes().await?;
        if self
            .seal_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ModelError::Conflict("native seal already started"));
        }
        let mut state = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?;
        state.active += 1;
        Ok(OperationLease {
            owner: self.admission.clone(),
            scan: false,
            mutation: false,
            finished: false,
            operation: "native final seal",
            poison_on_error: true,
            finalization: true,
            _producing: None,
            live: Arc::new(AtomicBool::new(true)),
        })
    }
    pub async fn import_entities(&self, rows: &[d::graph::Entity]) -> Result<(), ModelError> {
        let lease = self.admit_mutation("import_entities", true)?;
        lease.finish_with(
            Loader::for_attempt_views(self.client.clone(), self.attempt, vec![])
                .entities(rows)
                .await,
        )
    }
    pub async fn import_assertions(&self, rows: &[d::graph::Assertion]) -> Result<(), ModelError> {
        let lease = self.admit_mutation("import_assertions", true)?;
        lease.finish_with(
            Loader::for_attempt_views(self.client.clone(), self.attempt, vec![])
                .assertions(rows)
                .await,
        )
    }
    pub async fn import_original_stream(
        &self,
        source: ContentHash,
        content: ContentHash,
        length: u64,
        input: &mut (impl std::io::Read + Send),
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let lease = self.admit_mutation("import_original_stream", true)?;
        lease.finish_with(
            Loader::for_attempt_views(self.client.clone(), self.attempt, vec![])
                .with_budget(budget)
                .original_stream(source, content, length, input)
                .await,
        )
    }
    pub fn contributions<'a>(
        self: &'a Arc<Self>,
        budget: &'a ResourceBudget,
    ) -> futures::future::BoxFuture<'a, Result<Vec<CompletedContribution>, ModelError>> {
        Box::pin(async move {
            let lease = self.admit(false, "contributions", false)?;
            let result = lease.within(self.contributions_inner(budget)).await;
            lease.finish_with(result)
        })
    }
    async fn contributions_inner(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<Vec<CompletedContribution>, ModelError> {
        let selection = self.capture_state(budget).await?;
        let mut rows = self.track_rows(self.state_source("compiler_contribution", &selection)?)?;
        let mut contributions = Vec::new();
        while let Some(row) = rows.next().await? {
            validate_state_row("compiler_contribution", &row)?;
            let object =
                value_object(&row).ok_or(ModelError::Schema("contribution descriptor row"))?;
            let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
                return Err(ModelError::Schema("contribution descriptor bytes"));
            };
            let descriptor: CompletedContribution =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            descriptor.identity()?;
            contributions.push(descriptor);
        }
        Ok(contributions)
    }
    pub async fn scan_canonical(
        self: &Arc<Self>,
        entities: bool,
        budget: &ResourceBudget,
    ) -> Result<CompilerRows, ModelError> {
        self.scan_graph(
            entities,
            "id,anchor,content,canonical,kind,subtype,semantic_type,semantic_key",
            budget,
        )
        .await
    }
    pub async fn scan_graph_headers(
        self: &Arc<Self>,
        entities: bool,
        budget: &ResourceBudget,
    ) -> Result<CompilerRows, ModelError> {
        self.scan_graph(
            entities,
            "id,anchor,content,semantic_type,semantic_key,kind,subtype,body.byte_len AS source_length",
            budget,
        )
        .await
    }
    async fn scan_graph(
        self: &Arc<Self>,
        entities: bool,
        fields: &str,
        budget: &ResourceBudget,
    ) -> Result<CompilerRows, ModelError> {
        let retained = budget.reserve("compiler-read-setup", fields.len().saturating_add(1024))?;
        let fields = fields.to_owned();
        let store = self.clone();
        let budget = budget.clone();
        self.retain_read_setup(
            "scan_graph",
            Box::pin(async move {
                store
                    .scan_graph_inner(entities, &fields, &budget)
                    .await
                    .map(|rows| rows.with_setup(Some(Arc::new(retained))))
            }),
        )
        .await
    }
    async fn scan_graph_inner(
        self: &Arc<Self>,
        entities: bool,
        fields: &str,
        budget: &ResourceBudget,
    ) -> Result<CompilerRows, ModelError> {
        self.check_failed()?;
        let frozen = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?
            .content_ready;
        let ordered = if frozen {
            let prepared = self.prepared_canonical(budget).await?;
            let run = if entities {
                prepared.entities
            } else {
                prepared.assertions
            };
            AsyncOrderedRows::new_registered(
                run,
                budget,
                self.blocking_owner()?,
                self.blocking_observer()?,
            )
            .await?
        } else {
            let (reader, selected) = self.canonical_selection(budget).await?;
            let result = async {
                self.discover_graph(entities, budget, &selected)
                    .await?
                    .finish()
                    .await
            }
            .await;
            drop(selected);
            let mut terminal = d::completion::Completion::default();
            terminal.step("canonical discovery reader close", reader.close().await);
            d::completion::complete(result, terminal)?
        };
        let transfer = budget.reserve(
            "canonical-pointer-window",
            crate::loader::NATIVE_WINDOW_ROWS * 512,
        )?;
        let lease = self.retained_scan()?;
        let graph = GraphRows {
            ordered,
            payload: None,
            store: self.clone(),
            fields: fields.into(),
            expected: std::collections::VecDeque::new(),
            setup: None,
            _transfer: transfer,
        };
        Ok(CompilerRows {
            descendant: Some(lease.binding()),
            producing: self.producing_binding()?,
            rows: None,
            selection: None,
            graph: Some(Box::new(graph)),
            store: self.clone(),
            lease: Some(lease),
            setup: None,
            installed: None,
            source_hold: None,
        })
    }
    async fn discover_graph(
        self: &Arc<Self>,
        entities: bool,
        budget: &ResourceBudget,
        selected: &crate::selection::SelectedPayloads,
    ) -> Result<AsyncPhysicalSort, ModelError> {
        self.check_failed()?;
        let table = if entities { "entity" } else { "assertion" };
        let mut sorted = AsyncPhysicalSort::new_registered(
            budget,
            self.blocking_owner()?,
            self.blocking_observer()?,
        )
        .await?;
        let mut rows =
            self.track_rows(selected.rows(table, "true", Variables::new(), vec![], "id", None)?)?;
        while let Some(value) = rows.next_native().await? {
            let object =
                value_object(&value).ok_or(ModelError::Schema("canonical selected payload"))?;
            let Some(Value::RecordId(anchor)) = object.get("anchor") else {
                return Err(ModelError::Schema("canonical payload anchor"));
            };
            let Some(Value::RecordId(node)) = object.get("id") else {
                return Err(ModelError::Schema("canonical payload pointer"));
            };
            let mut pointer = Object::new();
            pointer.insert("id", anchor.clone());
            pointer.insert("payload", node.clone());
            sorted.push(Value::Object(pointer)).await?;
        }
        Ok(sorted)
    }
    pub async fn prepare_canonical(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let lease = self.admit(false, "prepare_canonical", true)?;
        let result = lease
            .within(self.prepared_canonical(budget))
            .await
            .map(|_| ());
        lease.finish_with(result)
    }
    async fn prepared_canonical(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<PreparedCanonical, ModelError> {
        // This is a descendant of an already admitted read/preparation. Final closure
        // waits for that parent, rather than refusing its remaining work.
        self.check_frozen_content()?;
        let pending = {
            let mut slot = self
                .preparation
                .lock()
                .map_err(|_| ModelError::Conflict("native canonical preparation owner"))?;
            if let Some(pending) = &*slot {
                pending.clone()
            } else {
                let lease = self.retained_scan()?;
                let binding = self.producing_binding()?;
                let store = self.clone();
                let budget = budget.clone();
                let task = self.runtime.spawn(
                    async move {
                        let phase = crate::phase::Phase::begin("native_canonical_preparation");
                        let result = lease
                            .within(producing_future(binding, async {
                                let (reader, selected) = store.canonical_selection(&budget).await?;
                                let result = async {
                                    let entities = store
                                        .discover_graph(true, &budget, &selected)
                                        .await?
                                        .prepare()
                                        .await?;
                                    let assertions = store
                                        .discover_graph(false, &budget, &selected)
                                        .await?
                                        .prepare()
                                        .await?;
                                    Ok(PreparedCanonical {
                                        entities,
                                        assertions,
                                    })
                                }
                                .await;
                                drop(selected);
                                let mut terminal = d::completion::Completion::default();
                                terminal.step(
                                    "canonical preparation reader close",
                                    reader.close().await,
                                );
                                d::completion::complete(result, terminal)
                            }))
                            .await;
                        let result = lease.finish_with(result);
                        phase.finish_result(&result);
                        result.map_err(Arc::new)
                    }
                    .in_current_span()
                    .with_current_subscriber(),
                );
                let pending = async move {
                    task.await
                        .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))?
                }
                .boxed()
                .shared();
                *slot = Some(pending.clone());
                pending
            }
        };
        pending.await.map_err(ModelError::SharedCause)
    }
    pub fn attempt(&self) -> ContentHash {
        self.attempt
    }
    pub fn generation(&self) -> ContentHash {
        self.generation
    }
    pub fn database(&self) -> &d::serving::Name {
        &self.database
    }
    pub fn namespace(&self) -> &d::serving::Name {
        &self.namespace
    }
    pub fn fail(&self) {
        self.failed.store(true, Ordering::Release);
        self.admission.changed.notify_waiters();
    }
    fn check_failed(&self) -> Result<(), ModelError> {
        self.signals
            .as_ref()
            .map_err(|error| ModelError::SharedCause(error.clone()))?;
        if self.signal_stop.stopped() {
            return Err(ModelError::Invalid("native operation cancelled".into()));
        }
        let state = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?;
        if self.failed.load(Ordering::Acquire) || state.failed {
            return Err(state.refusal("native compiler authority failed"));
        }
        Ok(())
    }
    pub fn check(&self) -> Result<(), ModelError> {
        let state = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?;
        if self.failed.load(Ordering::Acquire)
            || self.sealed.load(Ordering::Acquire)
            || state.closed
            || state.failed
        {
            return Err(state.refusal("native compiler authority closed or failed"));
        }
        Ok(())
    }
    fn admit(
        &self,
        scan: bool,
        operation: &'static str,
        poison_on_error: bool,
    ) -> Result<OperationLease, ModelError> {
        self.check_read_admission()?;
        self.admit_access(scan, false, operation, poison_on_error, true)
    }
    fn admit_root(
        &self,
        scan: bool,
        operation: &'static str,
        poison_on_error: bool,
    ) -> Result<OperationLease, ModelError> {
        self.admit_access(scan, false, operation, poison_on_error, false)
    }
    fn has_live_parent(&self) -> bool {
        DESCENDANT_WORK
            .try_with(|parent| {
                Arc::ptr_eq(&parent.owner, &self.admission) && parent.live.load(Ordering::Acquire)
            })
            .unwrap_or(false)
    }
    fn admit_mutation(
        &self,
        operation: &'static str,
        poison_on_error: bool,
    ) -> Result<OperationLease, ModelError> {
        if self.read_only.load(Ordering::Acquire) {
            return Err(ModelError::Conflict("read owner has no mutation authority"));
        }
        self.admit_access(false, true, operation, poison_on_error, true)
    }
    fn admit_access(
        &self,
        scan: bool,
        mutation: bool,
        operation: &'static str,
        poison_on_error: bool,
        allow_descendant: bool,
    ) -> Result<OperationLease, ModelError> {
        let descendant = allow_descendant && self.has_live_parent();
        let producing = self.producing_lease()?;
        let mut state = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?;
        if (mutation && state.content_closed)
            || (state.closed && !descendant)
            || state.failed
            || self.failed.load(Ordering::Acquire)
            || self.sealed.load(Ordering::Acquire)
        {
            return Err(state.refusal("native compiler authority closed or failed"));
        }
        state.active = state
            .active
            .checked_add(1)
            .ok_or(ModelError::Schema("native operation count"))?;
        if mutation {
            state.mutations += 1;
        }
        if scan {
            state.scans += 1;
        }
        Ok(OperationLease {
            owner: self.admission.clone(),
            scan,
            mutation,
            finished: false,
            operation,
            poison_on_error,
            finalization: false,
            _producing: producing,
            live: Arc::new(AtomicBool::new(true)),
        })
    }
    // Unchecked descendant registration requires a live admitted parent for this store.
    // Otherwise this is a new root and must pass ordinary global admission.
    fn retained_scan(&self) -> Result<OperationLease, ModelError> {
        if !self.has_live_parent() {
            return self.admit_root(true, "native stream", true);
        }
        let producing = self.producing_lease()?;
        let mut state = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?;
        state.active = state
            .active
            .checked_add(1)
            .ok_or(ModelError::Schema("native operation count"))?;
        state.scans += 1;
        Ok(OperationLease {
            owner: self.admission.clone(),
            scan: true,
            mutation: false,
            finished: false,
            operation: "native stream",
            poison_on_error: true,
            finalization: false,
            _producing: producing,
            live: Arc::new(AtomicBool::new(true)),
        })
    }
    fn track_rows(self: &Arc<Self>, rows: NativeRows) -> Result<CompilerRows, ModelError> {
        let lease = self.retained_scan()?;
        Ok(CompilerRows {
            descendant: Some(lease.binding()),
            producing: self.producing_binding()?,
            rows: Some(rows),
            selection: None,
            graph: None,
            store: self.clone(),
            lease: Some(lease),
            setup: None,
            installed: None,
            source_hold: None,
        })
    }
    fn blocking_owner(&self) -> Result<BlockingLease, ModelError> {
        Ok(BlockingLease(Some(self.retained_scan()?)))
    }
    fn record_background_failure(&self, error: Arc<ModelError>) {
        self.fail();
        if let Ok(mut state) = self.admission.state.lock() {
            state.failed = true;
            state.failures.push(NativeFailure {
                operation: "native background work",
                error,
            });
        }
    }
    /// Lazy compute consumers may stop before setup returns a row stream. The admitted
    /// driver, not that consumer's future, owns metadata reads and sort preparation to
    /// their real terminal. Lost delivery drops rows through their existing drainage.
    async fn retain_read_setup(
        self: &Arc<Self>,
        operation: &'static str,
        setup: BoxFuture<'static, Result<CompilerRows, ModelError>>,
    ) -> Result<CompilerRows, ModelError> {
        let lease = self.admit(false, operation, false)?;
        let (deliver, receive) = tokio::sync::oneshot::channel();
        let store = self.clone();
        let binding = self.producing_binding()?;
        self.runtime.spawn(
            async move {
                let result =
                    lease.observe_result(lease.within(producing_future(binding, setup)).await);
                // The buffered result owns the lease until consumption or discard, closing
                // the race between successful send and receiver cancellation. Its Drop
                // retains a lost error / drains rows before releasing admission.
                let result = ReadSetupResult {
                    result: Some(result),
                    lease: Some(lease),
                    store,
                    operation,
                };
                if let Err(undelivered) = deliver.send(result) {
                    drop(undelivered);
                }
            }
            .in_current_span()
            .with_current_subscriber(),
        );
        receive
            .await
            .map_err(|error| {
                ModelError::infrastructure(
                    Infrastructure::Unconfirmed,
                    format!("{operation} setup driver ended without a result: {error}"),
                )
            })?
            .take()
    }
    fn blocking_observer(
        self: &Arc<Self>,
    ) -> Result<impl FnOnce(BlockingTerminal) + Send + 'static, ModelError> {
        let lease = self.retained_scan()?;
        let store = self.clone();
        let dispatch = tracing::dispatcher::get_default(Clone::clone);
        let span = tracing::Span::current();
        Ok(move |terminal| {
            let runtime = store.runtime.clone();
            runtime.spawn(
                async move {
                    if let Err(error) = terminal.await {
                        store.record_background_failure(error);
                    }
                    lease.finish();
                }
                .instrument(span)
                .with_subscriber(dispatch),
            );
        })
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "A selected stream retains its exact view, projected fields, bound predicate and compact preparation"
    )]
    fn track_selection(
        self: &Arc<Self>,
        keys: SelectionKeys,
        view: &CompletedView,
        installed: Arc<lctx_native::installed_view::HeldInstalledView>,
        relation: &Relation,
        fields: String,
        predicate: String,
        bindings: Variables,
        preparation: Vec<String>,
        budget: &ResourceBudget,
    ) -> Result<CompilerRows, ModelError> {
        let direct = matches!(&keys, SelectionKeys::Known(_));
        let transfer = budget.reserve(
            "compiler-selected-window",
            (if direct {
                0
            } else {
                view.contributions.len().saturating_mul(64)
            })
            .saturating_add(crate::loader::NATIVE_WINDOW_ROWS * 2048),
        )?;
        let owners = if direct {
            Vec::new()
        } else {
            self.view_owners(view)?
        };
        let lease = self.retained_scan()?;
        let selection = SelectedRows {
            keys,
            installed,
            direct_members: None,
            direct_position: 0,
            budget: budget.clone(),
            owners,
            relation: relation.name().into(),
            fields,
            predicate,
            bindings,
            preparation,
            store: self.clone(),
            lookup: None,
            payload: None,
            pending_keys: Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS),
            pointers: BTreeMap::new(),
            setup: None,
            _transfer: transfer,
        };
        Ok(CompilerRows {
            descendant: Some(lease.binding()),
            producing: self.producing_binding()?,
            rows: None,
            selection: Some(Box::new(selection)),
            graph: None,
            store: self.clone(),
            lease: Some(lease),
            setup: None,
            installed: None,
            source_hold: None,
        })
    }
    async fn wait_operations(&self, scans_only: bool) -> Result<(), ModelError> {
        loop {
            let notification = self.admission.changed.notified();
            tokio::pin!(notification);
            notification.as_mut().enable();
            {
                let state = self
                    .admission
                    .state
                    .lock()
                    .map_err(|_| ModelError::Conflict("native operation admission"))?;
                let finished = if scans_only {
                    state.scans == 0
                } else {
                    state.active == 0
                };
                if finished {
                    return Ok(());
                }
            }
            notification.await;
        }
    }
    async fn wait_scans(&self) -> Result<(), ModelError> {
        self.wait_operations(true).await
    }
    pub async fn abandon(&self) -> Result<(), ModelError> {
        if self.read_only.load(Ordering::Acquire) {
            return Err(ModelError::Conflict(
                "read owner has no attempt abandonment authority",
            ));
        }
        // Serialize acknowledgements, including callers repeating abandonment after
        // its original operation failed. Never query an already invalidated session.
        let mut source = self.admission.source_finalization.lock().await;
        let session = (!source.session_closed)
            .then(|| reader::SessionFinalizationGuard::new(self.client.clone()));
        let mut completion = if source.attempt_closed {
            self.acknowledged_finality()
        } else {
            self.drain_finality().await
        };
        // Abandon releases this attempt only; shared immutable completed data survives.
        if !source.attempt_closed
            && completion.local == d::completion::LocalState::Terminal
            && completion.remote == d::completion::RemoteState::Confirmed
            && completion.failures.is_empty()
        {
            let closed =
                crate::control::close_attempt(&self.client, self.attempt, "abandoned").await;
            if closed.is_ok() {
                source.attempt_closed = true;
            }
            completion.step("native attempt abandonment", closed);
        }
        if source.attempt_closed
            && completion.local == d::completion::LocalState::Terminal
            && completion.remote == d::completion::RemoteState::Confirmed
            && completion.failures.is_empty()
        {
            if let Some(session) = session {
                let closed = session.close().await;
                if closed.is_ok() {
                    source.session_closed = true;
                }
                completion.step("native session invalidation", closed);
            }
        }
        if !source.session_closed {
            completion.local = d::completion::LocalState::Outstanding;
        }
        self.complete_source_signals(&source, &mut completion).await;
        self.append_operation_failures(&mut completion);
        d::completion::complete(Ok(()), completion)
    }
    async fn complete_source_signals(
        &self,
        source: &SourceFinalization,
        completion: &mut d::completion::Completion,
    ) {
        // Signal completion is immutable. Keep the actual owner Started while
        // finalization can still be retried; an error/drop is not a close ack.
        if source.session_closed
            && completion.local == d::completion::LocalState::Terminal
            && completion.remote == d::completion::RemoteState::Confirmed
            && completion.failures.is_empty()
        {
            if let Ok(signals) = &self.signals {
                let finality = signals.complete(completion).await;
                completion.step("native compiler signal owner", finality);
            }
        }
    }
    /// Local terminality and remote acknowledgement are independent facts. The admission owner
    /// remains registered when this future is interrupted, allowing a later drain to finish.
    pub async fn drain_report(&self) -> d::completion::Completion {
        let source = self.admission.source_finalization.lock().await;
        let mut completion = if source.attempt_closed {
            self.acknowledged_finality()
        } else {
            self.drain_finality().await
        };
        // Attempt cleanup may already be acknowledged while session close is still
        // owned by abandonment. This report covers execution drainage, not that close.
        self.append_operation_failures(&mut completion);
        completion
    }
    fn acknowledged_finality(&self) -> d::completion::Completion {
        let mut completion = d::completion::Completion::default();
        match self.admission.state.lock() {
            Ok(state) => completion.committed.extend(state.committed.clone()),
            Err(_) => {
                completion.local = d::completion::LocalState::Outstanding;
                completion.step(
                    "native finalization acknowledgement state",
                    Err(ModelError::Conflict("native operation admission")),
                );
            }
        }
        completion
    }
    fn append_operation_failures(&self, completion: &mut d::completion::Completion) {
        if let Ok(state) = self.admission.state.lock() {
            for failure in &state.failures {
                completion.step(
                    failure.operation,
                    Err(ModelError::SharedCause(failure.error.clone())),
                );
            }
        }
    }
    // An acknowledged operation failure remains the caller's result, but is not
    // a failure to join that operation or finalize its actual native owner.
    async fn drain_finality(&self) -> d::completion::Completion {
        let mut completion = d::completion::Completion::default();
        match self.admission.state.lock() {
            Ok(mut state) => state.closed = true,
            Err(_) => {
                completion.local = d::completion::LocalState::Outstanding;
                completion.step(
                    "close native admission",
                    Err(ModelError::Conflict("native operation admission")),
                );
                return completion;
            }
        }
        if let Err(error) = self.wait_operations(false).await {
            completion.local = d::completion::LocalState::Outstanding;
            completion.step("native operation drainage", Err(error));
        }
        match self.admission.state.lock() {
            Ok(state) if state.uncertain => {
                completion.remote = d::completion::RemoteState::Unknown;
                completion.step(
                    "native acknowledgement",
                    Err(ModelError::infrastructure(
                        Infrastructure::Unconfirmed,
                        "native operation ended without acknowledgement",
                    )),
                );
            }
            Err(_) => {
                completion.remote = d::completion::RemoteState::Unknown;
                completion.step(
                    "native acknowledgement state",
                    Err(ModelError::Conflict("native operation admission")),
                );
            }
            _ => {}
        }
        if let Ok(state) = self.admission.state.lock() {
            completion.committed.extend(state.committed.clone());
        }
        // Final readback is non-destructive and safe after admission. It borrows the
        // existing session and retains the original ingress caller's budget capability.
        if completion.local == d::completion::LocalState::Terminal {
            let validate_completed = match self.admission.state.lock() {
                Ok(state) => state.content_ready,
                Err(_) => {
                    completion.step(
                        "original ingress completion scope",
                        Err(ModelError::Conflict("native operation admission")),
                    );
                    false
                }
            };
            let originals = self
                .original_ingress
                .close_owned(
                    self.client.clone(),
                    self.attempt,
                    self.read_cancellation(),
                    validate_completed,
                )
                .await;
            completion.step(
                "original ingress final close",
                d::completion::complete(Ok(()), originals),
            );
        }
        // Release protected publication clones only after all accepted compiler reads
        // are terminal; mapped metadata alone never retains selection authority.
        if completion.local == d::completion::LocalState::Terminal {
            match self.installed_views.lock() {
                Ok(mut views) => views.clear(),
                Err(_) => completion.step(
                    "installed view capability release",
                    Err(ModelError::Conflict("installed view owner")),
                ),
            };
            match self.view_preparations.lock() {
                Ok(mut preparations) => preparations.clear(),
                Err(_) => completion.step(
                    "internal preparation registry release",
                    Err(ModelError::Conflict("internal preparation registry")),
                ),
            };
            match self.product_retention_gates.lock() {
                Ok(mut gates) => gates.clear(),
                Err(_) => completion.step(
                    "product retention gate release",
                    Err(ModelError::Conflict("product retention gate registry")),
                ),
            };
            if let Some(audit) = self.view_audit.lock().await.as_mut() {
                if completion.remote == d::completion::RemoteState::Unknown {
                    audit.retain_unknown();
                }
                completion.step(
                    "internal contribution preparation pin close",
                    audit.close_drained().await,
                );
            }
            match self.physical_completed.lock() {
                Ok(mut cap) => {
                    cap.take();
                }
                Err(_) => completion.step(
                    "physical completion capability release",
                    Err(ModelError::Conflict("physical completion owner")),
                ),
            };
            match self.published_completion.lock() {
                Ok(mut cap) => {
                    cap.take();
                }
                Err(_) => completion.step(
                    "published completion capability release",
                    Err(ModelError::Conflict("published completion owner")),
                ),
            };
        }
        // A borrowed store owns its admitted execution, but not the caller's
        // session or reader pin. Its receipt ends after its own work settles;
        // the independent session owner retains invalidation finality.
        if !self.owns_session {
            if let Ok(signals) = &self.signals {
                let finality = signals.complete(&completion).await;
                completion.step("borrowed compiler signal owner", finality);
            }
        }
        completion
    }
    /// Final drainage closes admission before waiting; ordinary completion uses scan quiescence.
    pub async fn drain(&self) -> Result<(), ModelError> {
        d::completion::complete(Ok(()), self.drain_report().await)
    }
    pub async fn end_writes(&self) -> Result<(), ModelError> {
        self.drain().await?;
        self.check_failed()?;
        self.sealed.store(true, Ordering::Release);
        Ok(())
    }
    pub async fn begin_contribution(
        self: &Arc<Self>,
        spec: ContributionSpec,
    ) -> Result<ContentHash, ModelError> {
        let phase = crate::phase::Phase::begin("native_contribution_begin");
        let result = async {
            let lease = self.admit_mutation("begin_contribution", true)?;
            let result = lease.within(self.begin_contribution_inner(spec)).await;
            let result = lease.finish_with(result);
            if result.is_err() {
                self.fail();
            }
            result
        }
        .await;
        phase.finish_result(&result);
        result
    }
    async fn begin_contribution_inner(
        self: &Arc<Self>,
        spec: ContributionSpec,
    ) -> Result<ContentHash, ModelError> {
        self.check_failed()?;
        self.validate_inputs(&spec.inputs).await?;
        let id = attempt_contribution(self.attempt, spec.identity()?);
        {
            let mut specifications = self
                .specifications
                .lock()
                .map_err(|_| ModelError::Conflict("native contribution owner"))?;
            if specifications
                .insert(
                    id,
                    RegisteredContribution {
                        spec: spec.clone(),
                        ownership: ContributionOwnership::Registering,
                        _transport_charge: None,
                    },
                )
                .is_some()
            {
                self.fail();
                return Err(ModelError::Conflict("duplicate native contribution"));
            }
        }
        let mut row = Object::new();
        row.insert("id", RecordId::new("compiler_contribution", id.hex()));
        row.insert(
            "spec",
            Bytes::from(serde_json::to_vec(&spec).map_err(ModelError::codec)?),
        );
        row.insert("completed", false);
        row.insert(
            "attempt",
            RecordId::new("native_attempt", self.attempt.hex()),
        );
        for (field, value) in contribution_projection(&spec) {
            row.insert(field, value);
        }
        let mut b = Variables::new();
        b.insert("row", row);
        let write = async {
            crate::control::guarded_effect(&self.client,Some(self.attempt),vec![RecordId::new("compiler_contribution",id.hex())],"IF $__owner.admitted { THROW 'native attempt canonical content admitted'; }; CREATE $row.id CONTENT $row RETURN NONE",b).await?;
            self.hold_contribution_inputs(id,&spec).await?;
            Ok::<(), ModelError>(())
        }
        .await;
        if let Err(e) = write {
            self.fail();
            return Err(e);
        }
        self.specifications
            .lock()
            .map_err(|_| ModelError::Conflict("native contribution owner"))?
            .get_mut(&id)
            .ok_or(ModelError::Conflict("unknown native contribution"))?
            .ownership = ContributionOwnership::Producing;
        Ok(id)
    }
    pub async fn write_batch(
        self: &Arc<Self>,
        contribution: &ContentHash,
        relation: &Relation,
        batch: &RecordBatch,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.boxed_write_batch(contribution, relation, batch, None, Some(budget))
            .await
    }
    /// Model-owned canonical buffers need physical lowering, not another typed decode/sort/encode.
    /// The contribution still independently checks its model, declared output and mutable lifetime.
    pub async fn write_checked_batch(
        self: &Arc<Self>,
        contribution: &ContentHash,
        batch: &CheckedCanonicalBatch,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.boxed_write_batch(
            contribution,
            batch.relation(),
            batch.arrow(),
            Some(batch.model()),
            Some(budget),
        )
        .await
    }
    // Keep all declared-record lowering and its poll frame inside the native library.
    #[inline(never)]
    fn boxed_write_batch<'a>(
        self: &'a Arc<Self>,
        contribution: &'a ContentHash,
        relation: &'a Relation,
        batch: &'a RecordBatch,
        checked_model: Option<ContentHash>,
        budget: Option<&'a ResourceBudget>,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        crate::control::with_retry_cancellation(self.signal_stop.clone(), async move {
            let phase = crate::phase::Phase::begin("native_batch_write");
            let result = async {
                let lease = self.admit_mutation("write_batch", true)?;
                let result = WRITING_CONTRIBUTION
                    .scope(
                        *contribution,
                        lease.within(self.write_batch_inner(
                            contribution,
                            relation,
                            batch,
                            checked_model,
                            budget,
                        )),
                    )
                    .await;
                let result = lease.finish_with(result);
                if result.is_err() {
                    self.fail();
                }
                result
            }
            .await;
            phase.finish_result(&result);
            result
        })
        .boxed()
    }
    async fn write_batch_inner(
        self: &Arc<Self>,
        contribution: &ContentHash,
        relation: &Relation,
        batch: &RecordBatch,
        checked_model: Option<ContentHash>,
        budget: Option<&ResourceBudget>,
    ) -> Result<(), ModelError> {
        self.check_failed()?;
        {
            let specifications = self
                .specifications
                .lock()
                .map_err(|_| ModelError::Conflict("native contribution owner"))?;
            let (spec, completed) = specifications
                .get(contribution)
                .ok_or(ModelError::Conflict("unknown producer contribution"))?
                .checked()?;
            if completed {
                self.fail();
                return Err(ModelError::Conflict("write after contribution completion"));
            }
            if !spec.outputs.contains(relation.name()) {
                self.fail();
                return Err(ModelError::Conflict("undeclared producer output"));
            }
            if checked_model.is_some_and(|model| model != spec.model) {
                return Err(ModelError::Conflict(
                    "checked ingress model differs from contribution",
                ));
            }
        }
        let adapter = crate::adapter::select(relation.name())?;
        if adapter.relation.type_id() != relation.type_id()
            || adapter.relation.schema() != relation.schema()
        {
            return Err(ModelError::Schema("native relation authority"));
        }
        let batch = match checked_model {
            Some(_) => batch.clone(),
            None => adapter.relation.canonical(batch)?,
        };
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("native nominal ID column"))?;
        // The original body has its own bounded native-piece stream; it must not first
        // expand into a JSON byte array alongside the decoded typed original.
        let original_relation = relation.name() == d::artifact::ArtifactChunk::NAME;
        let _original_decode = if original_relation {
            budget
                .map(|budget| {
                    budget.reserve(
                        "native original typed decode",
                        d::logical_batch_bytes(&batch)?
                            .saturating_mul(2)
                            .saturating_add(batch.num_rows().saturating_mul(1024)),
                    )
                })
                .transpose()?
        } else {
            None
        };
        // Admit all simultaneous physical lowering state before JSON/graph/alias
        // construction. Arrow owns compact values; native containers and canonical
        // JSON coexist with graph decoding, payload/member maps and alias records.
        // Original raw bytes use the separately bounded piece path above.
        let _physical_lowering = if !original_relation {
            budget
                .map(|budget| {
                    budget.reserve(
                        "native typed physical lowering",
                        d::logical_batch_bytes(&batch)?
                            .saturating_mul(32)
                            .saturating_add(batch.num_rows().saturating_mul(
                                batch.num_columns().saturating_mul(512).saturating_add(4096),
                            ))
                            .saturating_add(65536),
                    )
                })
                .transpose()?
        } else {
            None
        };
        let mut bodies = if original_relation {
            Vec::new()
        } else {
            crate::codec::batch_bodies(relation, &batch)?
        };
        let original_chunks = if relation.name() == d::artifact::ArtifactChunk::NAME {
            let rows = d::artifact::ArtifactChunk::decode(&batch)?;
            bodies = rows
                .iter()
                .map(crate::adapter::original_metadata)
                .collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            Vec::new()
        };
        let original_headers = if relation.name() == d::source::SourceArtifact::NAME {
            d::source::SourceArtifact::decode(&batch)?
        } else {
            Vec::new()
        };
        let graph = adapter.graph(&batch)?;
        let native_aliases = if matches!(
            *self
                .frontier
                .lock()
                .map_err(|_| ModelError::Conflict("frontier owner"))?,
            Frontier::Normalized | Frontier::Analysis | Frontier::Catalog
        ) {
            graph
                .iter()
                .filter_map(|row| {
                    if let Some(GraphRow::Entity(source)) = row {
                        source.canonical_place_endpoint().map(|target| {
                            crate::loader::entity_payload_id(source)
                                .map(|physical| (physical, target))
                        })
                    } else {
                        None
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        let mut nodes = BTreeMap::<String, Value>::new();
        let mut members = BTreeMap::<String, Value>::new();
        for (i, (body, graph)) in bodies.into_iter().zip(graph).enumerate() {
            let key = hex::encode(ids.value(i));
            let endpoints = match &graph {
                Some(GraphRow::Entity(row)) => crate::loader::entity_endpoint_values(row)?,
                Some(GraphRow::Assertion(row)) => crate::loader::assertion_endpoint_values(row)?,
                None => Vec::new(),
            };
            for endpoint in endpoints {
                let id = value_object(&endpoint)
                    .and_then(|row| row.get("id"))
                    .ok_or(ModelError::Schema("canonical producer endpoint"))?
                    .to_sql();
                if let Some(previous) = nodes.insert(id, endpoint.clone())
                    && previous != endpoint
                {
                    return Err(ModelError::Conflict(
                        "canonical producer endpoint collision",
                    ));
                }
            }
            let (node, content, canonical, kind, subtype) = match graph {
                Some(GraphRow::Entity(row)) => (
                    crate::loader::entity_payload_id(&row)?,
                    row.content(),
                    serde_json::to_vec(&row).map_err(ModelError::codec)?,
                    Some(row.kind() as i64),
                    row.subtype(),
                ),
                Some(GraphRow::Assertion(row)) => (
                    crate::loader::assertion_payload_id(&row)?,
                    row.content(),
                    serde_json::to_vec(&row).map_err(ModelError::codec)?,
                    Some(row.kind as i64),
                    None,
                ),
                None => {
                    let bytes = serde_json::to_vec(&body).map_err(ModelError::codec)?;
                    (
                        crate::loader::payload_id(
                            "compiler_record",
                            relation.name(),
                            ids.value(i),
                            ContentHash::of(&bytes),
                        )?,
                        ContentHash::of(&bytes),
                        bytes,
                        None,
                        None,
                    )
                }
            };
            let physical = crate::adapter::physical_row(
                node.clone(),
                content,
                canonical,
                kind.map(|kind| (kind, subtype)),
                crate::codec::RecordView {
                    semantic_type: relation.name().into(),
                    semantic_key: key.clone(),
                    scopes: adapter.scopes(&body)?,
                    body,
                },
            )?;
            let name = node.to_sql();
            if let Some(previous) = nodes.insert(name, physical.clone())
                && previous != physical
            {
                return Err(ModelError::Conflict("native same-key payload"));
            }
            let mut member = Object::new();
            member.insert(
                "id",
                membership_id(*contribution, relation.name(), ids.value(i)),
            );
            member.insert(
                "contribution",
                RecordId::new("compiler_contribution", contribution.hex()),
            );
            member.insert("relation", relation.name().to_string());
            member.insert("semantic_key", key);
            member.insert("node", node);
            member.insert("content", content.hex());
            members.insert(sink_key(&member)?, Value::Object(member));
        }
        let alias_values = crate::codec::entity_views(
            &native_aliases
                .iter()
                .map(|(_, target)| target.clone())
                .collect::<Vec<_>>(),
        )?;
        let mut aliases = Vec::new();
        for ((source, target), view) in native_aliases.into_iter().zip(alias_values) {
            for endpoint in crate::loader::entity_endpoint_values(&target)? {
                let id = value_object(&endpoint)
                    .and_then(|row| row.get("id"))
                    .ok_or(ModelError::Schema("canonical alias endpoint"))?
                    .to_sql();
                if let Some(previous) = nodes.insert(id, endpoint.clone())
                    && previous != endpoint
                {
                    return Err(ModelError::Conflict("canonical alias endpoint collision"));
                }
            }
            let node = crate::loader::entity_payload_id(&target)?;
            let physical = crate::adapter::physical_row(
                node.clone(),
                target.content(),
                serde_json::to_vec(&target).map_err(ModelError::codec)?,
                Some((target.kind() as i64, target.subtype())),
                view,
            )?;
            if let Some(previous) = nodes.insert(node.to_sql(), physical.clone())
                && previous != physical
            {
                return Err(ModelError::Conflict("native alias payload"));
            }
            let mut sink = KeySink::new("native-place-alias/v1");
            sink.part(
                b"source",
                &serde_json::to_vec(&source).map_err(ModelError::codec)?,
            );
            sink.part(
                b"target",
                &serde_json::to_vec(&node).map_err(ModelError::codec)?,
            );
            let mut alias = Object::new();
            alias.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
            alias.insert("source", source);
            alias.insert("target", node);
            aliases.push(Value::Object(alias));
        }
        // Every selected record, alias and original range is valid before any window effect.
        self.ensure_physical_rows(nodes.into_values().collect(), budget)
            .await?;
        if !original_headers.is_empty() || !original_chunks.is_empty() {
            let budget = budget.ok_or(ModelError::Conflict(
                "original ingress requires caller budget",
            ))?;
            let contribution = WRITING_CONTRIBUTION.try_with(|id| *id).ok();
            let cancellation = self.read_cancellation()?;
            let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
            let reader = if let Some(flag) = cancellation {
                reader.with_read_cancellation(flag)
            } else {
                reader
            };
            // The adapter owns this reader scope, while the store owns the shared session.
            // Preserve the original failure and any independent local reader finalization.
            let result = async {
                self.original_ingress
                    .sources(
                        &reader,
                        self.attempt,
                        contribution,
                        &original_headers,
                        budget,
                    )
                    .await?;
                self.original_ingress
                    .chunks(
                        &reader,
                        self.attempt,
                        contribution,
                        &original_chunks,
                        budget,
                    )
                    .await
            }
            .await;
            let mut completion = d::completion::Completion::default();
            completion.step("original ingress reader close", reader.close().await);
            d::completion::complete(result, completion)?;
        }
        self.ensure_immutable_rows(
            "compiler_alias",
            aliases,
            "native alias same-key payload",
            budget,
        )
        .await?;
        // Repeated identical writes within this contribution are idempotent, without hiding
        // semantic uniqueness failures. Membership contains no separately editable payload.
        self.ensure_immutable_rows(
            "compiler_membership",
            members.into_values().collect(),
            "native membership same-key payload",
            budget,
        )
        .await?;
        Ok(())
    }

    async fn ensure_physical_rows(
        self: &Arc<Self>,
        rows: Vec<Value>,
        budget: Option<&ResourceBudget>,
    ) -> Result<(), ModelError> {
        let contribution = WRITING_CONTRIBUTION.try_with(|id| *id).ok();
        let owner = contribution
            .map(|id| RecordId::new("compiler_contribution", id.hex()))
            .unwrap_or_else(|| RecordId::new("native_attempt", self.attempt.hex()));
        for window in crate::control::ImmutableWindows::new(
            rows,
            Some(self.attempt),
            contribution,
            Some(owner),
        )
        .with_budget(budget)
        {
            crate::control::install_window(&self.client, Some(self.attempt), window?, budget)
                .await?;
        }
        Ok(())
    }

    // Compare only this bounded batch's complete immutable rows, then insert new rows in one
    // statement. Identical replay is admitted without per-row transactions or silent conflicts.
    async fn ensure_immutable_rows(
        self: &Arc<Self>,
        table: &str,
        rows: Vec<Value>,
        conflict: &'static str,
        budget: Option<&ResourceBudget>,
    ) -> Result<(), ModelError> {
        let _ = (table, conflict);
        self.ensure_physical_rows(rows, budget).await
    }

    pub async fn complete_contribution(
        self: &Arc<Self>,
        id: ContentHash,
        outcome: ProviderOutcome,
        outputs: &[Relation],
        previous: &BTreeMap<String, CompletedView>,
        budget: &ResourceBudget,
    ) -> Result<BTreeMap<String, CompletedView>, ModelError> {
        let phase = crate::phase::Phase::begin("native_contribution_complete");
        let result = async {
            // Independent/manual clients retain the conservative drainage contract. Ordinary
            // producers use the required native-owned producing scope below.
            self.wait_scans().await?;
            let lease = self.admit_mutation("complete_contribution", true)?;
            let result = lease
                .within(self.complete_contribution_inner(id, outcome, outputs, previous, budget))
                .await;
            let result = lease.finish_with(result);
            if result.is_err() {
                self.fail();
            }
            result
        }
        .await;
        phase.finish_result(&result);
        result
    }
    pub async fn complete_contribution_scoped(
        self: &Arc<Self>,
        scope: &ProducingScope,
        id: ContentHash,
        outcome: ProviderOutcome,
        outputs: &[Relation],
        previous: &BTreeMap<String, CompletedView>,
        budget: &ResourceBudget,
    ) -> Result<BTreeMap<String, CompletedView>, ModelError> {
        let phase = crate::phase::Phase::begin("native_contribution_complete_scoped");
        let result = async {
            if !scope.belongs_to(self) || scope.contribution != id {
                return Err(ModelError::Conflict(
                    "native producing contribution binding",
                ));
            }
            scope.close_and_wait().await?;
            let lease = self.admit_mutation("complete_contribution", true)?;
            let result = lease
                .within(self.complete_contribution_inner(id, outcome, outputs, previous, budget))
                .await;
            let result = lease.finish_with(result);
            if result.is_err() {
                self.fail();
            }
            result
        }
        .await;
        phase.finish_result(&result);
        result
    }
    /// Only this contribution's exact original-bearing memberships can earn completion.
    /// Other open producers may still legitimately own staging without source headers.
    async fn complete_contribution_originals(
        &self,
        id: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let cancellation = self.read_cancellation()?;
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let reader = if let Some(flag) = cancellation {
            reader.with_read_cancellation(flag)
        } else {
            reader
        };
        let result=async {
            let owner=RecordId::new("compiler_contribution",id.hex());
            let _page=budget.reserve("original contribution membership page",32*4096)?;
            for relation in [d::source::SourceArtifact::NAME,d::artifact::ArtifactChunk::NAME]{
                let mut vars=Variables::new();vars.insert("owner",owner.clone());vars.insert("relation",relation);vars.insert("after",Value::None);
                let mut after=None::<String>;
                loop{
                    let memberships:Vec<Value>=reader.query_native("SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation AND ($after=NONE OR semantic_key>$after) ORDER BY semantic_key LIMIT 32",vars.clone()).await?;
                    if memberships.is_empty(){break;}
                    let mut sources=Vec::with_capacity(memberships.len());
                    for membership in memberships{
                        validate_state_row("compiler_membership",&membership)?;
                        let member=value_object(&membership).ok_or(ModelError::Schema("original completion membership"))?;
                        let Some(Value::String(key))=member.get("semantic_key") else{return Err(ModelError::Schema("original completion membership key"));};
                        if member.get("contribution")!=Some(&Value::RecordId(owner.clone())) || member.get("relation")!=Some(&Value::String(relation.into())) || after.as_ref().is_some_and(|previous|key<=previous){return Err(ModelError::Conflict("original completion exact membership owner/order"));}
                        let Some(Value::RecordId(node))=member.get("node") else{return Err(ModelError::Schema("original completion payload pointer"));};
                        let mut payload_vars=Variables::new();payload_vars.insert("node",node.clone());
                        let payloads:Vec<Value>=reader.query_native("SELECT * FROM $node",payload_vars).await?;
                        let [payload]=payloads.as_slice() else{return Err(ModelError::Conflict("original completion payload missing"));};
                        let object=value_object(payload).ok_or(ModelError::Schema("original completion payload"))?;
                        if object.get("id")!=Some(&Value::RecordId(node.clone())) || object.get("content")!=member.get("content"){return Err(ModelError::Conflict("original completion membership payload"));}
                        let Some(Value::Bytes(canonical))=object.get("canonical") else{return Err(ModelError::Schema("original completion canonical payload"));};
                        let _decode=budget.reserve("original contribution payload decode",canonical.len().saturating_mul(8).saturating_add(4096))?;
                        let source=if relation==d::source::SourceArtifact::NAME{
                            let entity:d::graph::Entity=serde_json::from_slice(canonical).map_err(ModelError::codec)?;entity.validate()?;
                            if crate::loader::entity_payload_id(&entity)?!=*node || object.get("content")!=Some(&Value::String(entity.content().hex())){return Err(ModelError::Conflict("original completion source payload"));}
                            let d::graph::Entity::Source(source)=entity else{return Err(ModelError::Schema("original completion source kind"));};
                            if hex::encode(source.id().bytes())!=*key{return Err(ModelError::Conflict("original completion source semantic key"));}
                            lctx_native::originals::Layout::source(&source)?.id
                        }else{
                            let backing=validate_state_row("compiler_record",payload)?.ok_or(ModelError::Schema("original completion chunk backing"))?;
                            if object.get("semantic_key")!=Some(&Value::String(key.clone())){return Err(ModelError::Conflict("original completion chunk semantic key"));}
                            RecordId::new("original",d::graph::EntityId::of(backing.artifact).0.hex())
                        };
                        sources.push(source);after=Some(key.clone());
                    }
                    lctx_native::originals::complete_selected(&reader,Some(self.attempt),&sources,budget).await?;
                    vars.insert("after",after.clone().map(Value::String).unwrap_or(Value::None));
                }
            }Ok(())
        }.await;
        let mut completion = d::completion::Completion::default();
        completion.step("original contribution reader close", reader.close().await);
        d::completion::complete(result, completion)
    }
    async fn complete_contribution_inner(
        self: &Arc<Self>,
        id: ContentHash,
        outcome: ProviderOutcome,
        outputs: &[Relation],
        previous: &BTreeMap<String, CompletedView>,
        budget: &ResourceBudget,
    ) -> Result<BTreeMap<String, CompletedView>, ModelError> {
        self.check_failed()?;
        let registration = self
            .specifications
            .lock()
            .map_err(|_| ModelError::Conflict("native contribution owner"))?
            .get(&id)
            .cloned()
            .ok_or(ModelError::Conflict("unknown native contribution"))?;
        let (spec, completed) = registration.checked()?;
        // Begin, retained attachment and independent import establish these immutable
        // input holds before readiness. Completion consumes that checked ownership;
        // replay must not mutate the same dependency guards again.
        let spec = spec.clone();
        for relation in outputs {
            if let Some(prior) = previous.get(relation.name()) {
                if prior.relation != relation.name() {
                    return Err(ModelError::Conflict("previous completed view relation"));
                }
                self.registered_view(prior).await?;
            }
        }
        if completed {
            let descriptor = self.completed_contribution(id).await?;
            if descriptor.outcome != outcome.code() {
                return Err(ModelError::Conflict("retained contribution outcome"));
            }
            let mut views = BTreeMap::new();
            for relation in outputs {
                let singleton = self.contribution_view_inner(id, relation).await?;
                let mut contributions = previous
                    .get(relation.name())
                    .map(|view| view.contributions.clone())
                    .unwrap_or_default();
                contributions.extend(singleton.contributions);
                let candidate = CompletedView::new(relation.name().into(), contributions, 0)?;
                let prepared = self.prepare_memberships(&candidate, budget).await?;
                let mut ordered = AsyncOrderedCandidates::new_registered(
                    prepared,
                    &budget,
                    self.blocking_owner()?,
                    self.blocking_observer()?,
                )
                .await?;
                let mut count = 0u64;
                loop {
                    let rows = ordered
                        .next_batch(crate::loader::NATIVE_WINDOW_ROWS)
                        .await?;
                    if rows.is_empty() {
                        break;
                    }
                    count += rows.len() as u64;
                }
                let view = CompletedView::new(candidate.relation, candidate.contributions, count)?;
                views.insert(relation.name().to_string(), view);
            }
            self.declare_completed_views(&views.values().collect::<Vec<_>>(), budget)
                .await?;
            return Ok(views);
        }
        let prior_ids = outputs
            .iter()
            .filter_map(|relation| previous.get(relation.name()))
            .map(|view| {
                self.view_owners(view)
                    .map(|owners| (view.relation.clone(), owners))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut content = BTreeMap::<String, (u64, KeySink)>::new();
        for output in outputs {
            content.insert(
                output.name().into(),
                (0, KeySink::new("native-contribution-output/v1")),
            );
        }
        let mut added = BTreeMap::<String, u64>::new();
        let mut pending_relation = None::<String>;
        let mut pending = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS);
        let mut b = Variables::new();
        b.insert(
            "contribution",
            RecordId::new("compiler_contribution", id.hex()),
        );
        let mut stream=self.track_rows(NativeRows::new(self.client.query("SELECT relation,semantic_key,content FROM compiler_membership WHERE contribution=$contribution ORDER BY relation,semantic_key").bind(b).stream_items().map_err(ModelError::codec)?,1)?)?;
        while let Some(row) = stream.next().await? {
            let row = SerdeWrapper::<MembershipContent>::from_value(row)
                .map_err(ModelError::codec)?
                .0;
            let (count, sink) = content
                .get_mut(&row.relation)
                .ok_or(ModelError::Conflict("undeclared native output"))?;
            row.semantic_key.encode(sink);
            row.content.encode(sink);
            *count += 1;
            if pending_relation
                .as_ref()
                .is_some_and(|relation| relation != &row.relation)
            {
                let relation = pending_relation.take().expect("nonempty key window");
                let owners = &prior_ids[&relation];
                *added.entry(relation.clone()).or_default() += self
                    .new_membership_keys(&relation, &pending, owners)
                    .await?;
                pending.clear();
            }
            if previous
                .get(&row.relation)
                .is_some_and(|prior| prior.rows > 0)
            {
                pending_relation = Some(row.relation.clone());
                pending.push(row.semantic_key);
                if pending.len() == crate::loader::NATIVE_WINDOW_ROWS {
                    let owners = &prior_ids[&row.relation];
                    *added.entry(row.relation.clone()).or_default() += self
                        .new_membership_keys(&row.relation, &pending, owners)
                        .await?;
                    pending.clear();
                    pending_relation = None;
                }
            } else {
                *added.entry(row.relation).or_default() += 1;
            }
        }
        if let Some(relation) = pending_relation {
            let owners = &prior_ids[&relation];
            *added.entry(relation.clone()).or_default() += self
                .new_membership_keys(&relation, &pending, owners)
                .await?;
        }
        self.complete_contribution_originals(id, budget).await?;
        let descriptor = CompletedContribution {
            assurance:
                lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            spec,
            outcome: outcome.code(),
            outputs: content
                .into_iter()
                .map(|(name, (rows, sink))| {
                    (
                        name,
                        OutputContent {
                            rows,
                            content: sink.finish(),
                        },
                    )
                })
                .collect(),
        };
        let logical = descriptor.identity()?;
        let mut b = Variables::new();
        b.insert("id", RecordId::new("compiler_contribution", id.hex()));
        b.insert(
            "descriptor",
            Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?),
        );
        b.insert("logical", logical.hex());
        b.insert("outcome", outcome.code());
        crate::control::effect(&self.client,Some(self.attempt),"fn::native_touch($id); LET $owner=SELECT * FROM ONLY $id FOR UPDATE; IF $owner.completed OR $owner.attempt != $__attempt { THROW 'native contribution completion fenced'; }; UPDATE $id SET descriptor=$descriptor,logical=$logical,outcome=$outcome,completed=true RETURN NONE",b).await?;
        self.specifications
            .lock()
            .map_err(|_| ModelError::Conflict("native contribution owner"))?
            .get_mut(&id)
            .ok_or(ModelError::Conflict("unknown native contribution"))?
            .ownership = ContributionOwnership::Completed;
        self.remember_contributor(logical, id)?;
        let _singletons_charge = budget.reserve(
            "completion singleton view descriptors",
            descriptor
                .outputs
                .keys()
                .map(|relation| std::mem::size_of::<CompletedView>() + relation.len() + 768)
                .fold(0usize, usize::saturating_add),
        )?;
        let mut singletons = Vec::with_capacity(descriptor.outputs.len());
        for (relation, output) in &descriptor.outputs {
            singletons.push(CompletedView::new(
                relation.clone(),
                [logical].into(),
                output.rows,
            )?);
        }
        let mut views = BTreeMap::new();
        for relation in outputs {
            let mut contributions = previous
                .get(relation.name())
                .map(|v| v.contributions.clone())
                .unwrap_or_default();
            contributions.insert(logical);
            let prior = previous.get(relation.name());
            let new = added.get(relation.name()).copied().unwrap_or(0);
            let count = prior
                .map(|view| view.rows)
                .unwrap_or(0)
                .checked_add(new)
                .ok_or(ModelError::Schema("native union count overflow"))?;
            let view = CompletedView::new(relation.name().into(), contributions, count)?;
            views.insert(relation.name().to_string(), view);
        }
        let _installation_charge = budget.reserve(
            "completion installed view references",
            singletons
                .len()
                .saturating_add(views.len())
                .saturating_mul(std::mem::size_of::<&CompletedView>()),
        )?;
        let mut installation = Vec::with_capacity(singletons.len().saturating_add(views.len()));
        installation.extend(singletons.iter().chain(views.values()));
        self.declare_completed_views(&installation, budget).await?;
        Ok(views)
    }

    /// Read the exact declared output of one completed contribution without registering a
    /// dependency view or adding optional capture metadata to retained compiler state.
    pub async fn scan_contribution_batches(
        self: &Arc<Self>,
        id: ContentHash,
        relation: &Relation,
        budget: &ResourceBudget,
        batch_rows: usize,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        if batch_rows == 0 {
            return Err(ModelError::Schema("native Arrow batch rows"));
        }
        let schema = relation.schema().clone();
        let bytes = schema
            .fields()
            .iter()
            .map(|field| size_of::<String>() + field.name().len())
            .sum::<usize>()
            .saturating_add(relation.name().len())
            .saturating_add(1024);
        let retained = budget.reserve("compiler-contribution-read-setup", bytes)?;
        let columns = schema
            .fields()
            .iter()
            .map(|field| field.name().clone())
            .collect::<Vec<_>>();
        let store = self.clone();
        let selected_relation = relation.clone();
        let selected_budget = budget.clone();
        let rows = self
            .retain_read_setup(
                "scan_contribution_batches",
                Box::pin(async move {
                    // This store reconstructs the private selection from its actual completed
                    // descriptor, checking physical/logical identity and declared output/count.
                    let view = store
                        .contribution_view_inner(id, &selected_relation)
                        .await?;
                    store
                        .scan_bound_rows_inner(
                            &view,
                            &selected_relation,
                            Some(&columns),
                            None,
                            &selected_budget,
                            None,
                        )
                        .await
                        .map(|rows| rows.with_setup(Some(Arc::new(retained))))
                }),
            )
            .await?;
        crate::compiler_provider::batches_from_rows(
            rows,
            relation.clone(),
            schema,
            budget,
            batch_rows,
        )
    }
    /// Receiver preflight reads use the candidate's source completion and exact
    /// contributor, without installing receiver views or selecting a logical match.
    pub async fn scan_held_candidate_batches(
        self: &Arc<Self>,
        candidate: &HeldCandidate,
        relation: &Relation,
        budget: &ResourceBudget,
        batch_rows: usize,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        if !Arc::ptr_eq(&candidate.owner, &self.admission) || batch_rows == 0 {
            return Err(ModelError::Conflict(
                "held candidate read receiver/batch binding",
            ));
        }
        if !candidate.descriptor.outputs.contains_key(relation.name()) {
            return Err(ModelError::Conflict("held candidate undeclared output"));
        }
        let lease = self.admit(false, "held source candidate read", true)?;
        let mut setup_owner = CompilerDrainOwners {
            installed: None,
            source_hold: Some(candidate.completed.clone()),
            confirmed: false,
        };
        let result=lease.within(async {
            let actual=crate::control::inspect_expected(&self.client,candidate.expected.record().clone(),budget).await?
                .ok_or(ModelError::Conflict("held candidate contributor disappeared"))?;
            if actual.incarnation()!=candidate.expected.incarnation() || actual.row()!=candidate.expected.row() {
                return Err(ModelError::Conflict("held candidate source changed"));
            }
            let schema=relation.schema().clone();
            let fields=schema.fields().iter().map(|field|field.name().clone()).collect::<Vec<_>>();
            let fields=projected_fields(relation,&fields)?.join(",").replace("body.","node.body.");
            let mut variables=Variables::new();variables.insert("owner",candidate.expected.record().clone());variables.insert("relation",relation.name().to_owned());
            let query=format!("SELECT {fields} FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation ORDER BY semantic_key");
            let mut rows=self.track_rows(prepared_rows(&self.client,query,variables,1,self.read_cancellation()?.as_ref())?)?;
            rows.source_hold=Some(candidate.completed.clone());
            crate::compiler_provider::batches_from_rows(rows,relation.clone(),schema,budget,batch_rows)
        }).await;
        setup_owner.confirmed = result
            .as_ref()
            .err()
            .is_none_or(|error| error.permits_storage_cleanup());
        lease.finish_with(result)
    }
    async fn contribution_view_inner(
        self: &Arc<Self>,
        id: ContentHash,
        relation: &Relation,
    ) -> Result<CompletedView, ModelError> {
        self.check_failed()?;
        let mut bindings = Variables::new();
        bindings.insert("id", RecordId::new("compiler_contribution", id.hex()));
        let mut response = self
            .read_client()?
            .query("SELECT * FROM $id")
            .bind(bindings)
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("missing completed contribution"));
        };
        validate_state_row("compiler_contribution", row)?;
        let descriptor: CompletedContribution = decode_descriptor(row)?;
        if attempt_contribution(self.attempt, descriptor.spec.identity()?) != id
            && !self
                .known_contributors
                .lock()
                .map_err(|_| ModelError::Conflict("native contributor owner"))?
                .values()
                .any(|known| *known == id)
        {
            return Err(ModelError::Conflict("completed contribution identity"));
        }
        let logical = descriptor.identity()?;
        let output = descriptor
            .outputs
            .get(relation.name())
            .ok_or(ModelError::Conflict("undeclared contribution output"))?;
        self.remember_contributor(logical, id)?;
        let view = CompletedView::new(
            relation.name().into(),
            std::collections::BTreeSet::from([logical]),
            output.rows,
        )?;
        Ok(view)
    }

    /// Reuse the necessary ordered content pass to probe only one bounded window of newly
    /// produced nominal keys. Prior memberships are selected before transfer; neither prior
    /// payloads nor the entire prior key universe are reconstructed for a union count.
    async fn new_membership_keys(
        self: &Arc<Self>,
        relation: &str,
        keys: &[String],
        owners: &[ContentHash],
    ) -> Result<u64, ModelError> {
        if keys.is_empty()
            || keys.len() > crate::loader::NATIVE_WINDOW_ROWS
            || keys.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ModelError::Schema("incremental membership key window"));
        }
        let keys = keys
            .iter()
            .map(|key| decode_nominal_key(key))
            .collect::<Result<Vec<_>, _>>()?;
        let mut lookup =
            MembershipLookup::new(self.clone(), relation.into(), keys, owners.to_vec());
        let nodes = lookup.collect().await?;
        u64::try_from(lookup.keys.len() - nodes.len()).map_err(ModelError::codec)
    }

    async fn validate_inputs(
        self: &Arc<Self>,
        inputs: &[d::analysis::sources::SourceSnapshot],
    ) -> Result<(), ModelError> {
        if inputs.is_empty() {
            return Ok(());
        }
        let mut views = {
            let known = self
                .known_views
                .lock()
                .map_err(|_| ModelError::Conflict("native view owner"))?;
            inputs
                .iter()
                .filter_map(|source| {
                    known
                        .get(&source.view())
                        .map(|view| (view.identity, view.clone()))
                })
                .collect::<BTreeMap<_, _>>()
        };
        let missing = inputs
            .iter()
            .map(|source| source.view())
            .filter(|id| !views.contains_key(id))
            .collect::<std::collections::BTreeSet<_>>();
        if !missing.is_empty() {
            let mut b = Variables::new();
            b.insert(
                "ids",
                missing
                    .iter()
                    .map(|id| RecordId::new("compiler_view", id.hex()))
                    .collect::<Vec<_>>(),
            );
            let mut response = self
                .read_client()?
                .query("SELECT * FROM $ids")
                .bind(b)
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
            let mut loaded = Vec::new();
            for row in rows {
                validate_state_row("compiler_view", &row)?;
                loaded.push(decode_descriptor::<CompletedView>(&row)?);
            }
            self.validate_view_contributions(&loaded).await?;
            for view in loaded {
                self.remember_view(&view)?;
                if let Some(old) = views.insert(view.identity, view.clone())
                    && old != view
                {
                    return Err(ModelError::Conflict("completed view collision"));
                }
            }
        }
        for source in inputs {
            let view = views
                .get(&source.view())
                .ok_or(ModelError::Conflict("missing dependency view"))?;
            if view.relation != source.relation()
                || source.rows() < 0
                || view.rows != source.rows() as u64
            {
                return Err(ModelError::Conflict("dependency view metadata"));
            }
        }
        Ok(())
    }
    /// Independent cold import/audit checks. Compilation carries completed validation instead.
    pub async fn verify_state(self: &Arc<Self>, budget: &ResourceBudget) -> Result<(), ModelError> {
        let lease = self.admit(false, "verify_state", false)?;
        let result = lease.within(self.verify_state_inner(budget)).await;
        lease.finish_with(result)
    }
    async fn verify_state_inner(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let selection = self.capture_state(budget).await?;
        self.verify_state_selected(&selection).await
    }
    async fn verify_state_selected(
        self: &Arc<Self>,
        selection: &AuditCapture,
    ) -> Result<(), ModelError> {
        // Cold audit/import must reconcile the fixed non-graph backing itself, before
        // comparing membership claims. Ordinary completed-view reads carry their validity.
        let mut backing = self.track_rows(
            self.state_source("compiler_record", selection)?
                .with_row_bytes(d::resources::MAX_ROW_BYTES),
        )?;
        let mut pending = Vec::new();
        let mut pending_bytes = 0;
        while let Some(row) = backing.next().await? {
            if let Some(original) = validate_state_row("compiler_record", &row)? {
                self.verify_original_backing(&original, &selection.budget)
                    .await?;
            }
            admit_backing_row(&mut pending, &mut pending_bytes, row, &selection.budget)?;
        }
        validate_backing_batch(&pending, &selection.budget)?;
        let mut memberships =
            self.track_rows(self.state_source("compiler_membership", selection)?)?;
        while let Some(member) = memberships.next_native().await? {
            validate_state_row("compiler_membership", &member)?;
            let object = value_object(&member).ok_or(ModelError::Schema("completed membership"))?;
            let node = object
                .get("node")
                .cloned()
                .ok_or(ModelError::Schema("completed membership node"))?;
            let mut actual = self.track_rows(NativeRows::new(
                self.read_client()?
                    .query("SELECT semantic_type,semantic_key,content FROM $node")
                    .bind(("node", node))
                    .stream_items()
                    .map_err(ModelError::codec)?,
                1,
            )?)?;
            let backing = actual.next_native().await?.ok_or(ModelError::Conflict(
                "completed membership backing/visibility",
            ))?;
            let backing = value_object(&backing).ok_or(ModelError::Schema("membership backing"))?;
            if backing.get("semantic_type") != object.get("relation")
                || backing.get("semantic_key") != object.get("semantic_key")
                || backing.get("content") != object.get("content")
            {
                return Err(ModelError::Conflict(
                    "completed membership backing/visibility",
                ));
            }
            if actual.next_native().await?.is_some() {
                return Err(ModelError::Conflict("duplicate membership backing"));
            }
        }
        let mut descriptors =
            self.track_rows(self.state_source("compiler_contribution", selection)?)?;
        while let Some(row) = descriptors.next().await? {
            validate_state_row("compiler_contribution", &row)?;
            let object =
                value_object(&row).ok_or(ModelError::Schema("contribution descriptor row"))?;
            let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
                return Err(ModelError::Schema("contribution descriptor bytes"));
            };
            let contribution: CompletedContribution =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            contribution.identity()?;
            self.validate_inputs(&contribution.spec.inputs).await?;
            let mut actual = contribution
                .outputs
                .keys()
                .map(|name| {
                    (
                        name.clone(),
                        (0u64, KeySink::new("native-contribution-output/v1")),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let mut variables = Variables::new();
            variables.insert(
                "contribution",
                value_object(&row)
                    .and_then(|row| row.get("id"))
                    .cloned()
                    .ok_or(ModelError::Schema("audited contribution identity"))?,
            );
            let mut members=self.track_rows(NativeRows::new(self.read_client()?.query("SELECT id,contribution,relation,semantic_key,node,content FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$contribution ORDER BY relation,semantic_key").bind(variables).stream_items().map_err(ModelError::codec)?,1)?)?;
            while let Some(row) = members.next().await? {
                validate_state_row("compiler_membership", &row)?;
                let member = SerdeWrapper::<MembershipContent>::from_value(row)
                    .map_err(ModelError::codec)?
                    .0;
                let (count, sink) = actual
                    .get_mut(&member.relation)
                    .ok_or(ModelError::Conflict("undeclared restored output"))?;
                member.semantic_key.encode(sink);
                member.content.encode(sink);
                *count += 1;
            }
            let actual = actual
                .into_iter()
                .map(|(name, (rows, sink))| {
                    (
                        name,
                        OutputContent {
                            rows,
                            content: sink.finish(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if actual != contribution.outputs {
                return Err(ModelError::Conflict(
                    "restored contribution output membership",
                ));
            }
        }
        // Cold audit uses one bounded budget for each complete exact membership scan,
        // rather than a whole-match GROUP array merely to count distinct keys.
        let audit_budget = &selection.budget;
        for view in selection.closure.views().values() {
            let relation = &crate::adapter::select(&view.relation)?.relation;
            let columns = vec!["id".to_string()];
            let mut rows = self
                .scan_rows_inner(&view, relation, Some(&columns), None, &audit_budget, None)
                .await?;
            let mut count = 0u64;
            while rows.next().await?.is_some() {
                count += 1;
            }
            if count != view.rows {
                return Err(ModelError::Conflict("completed view cardinality"));
            }
        }
        for binding in &selection.bindings {
            self.registered_view(&binding.view).await?;
        }
        Ok(())
    }
    async fn verify_original_backing(
        &self,
        original: &OriginalBacking,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let source = d::graph::EntityId::of(original.artifact);
        let cancellation = self.read_cancellation()?;
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let reader = if let Some(flag) = cancellation {
            reader.with_read_cancellation(flag)
        } else {
            reader
        };
        let result = async {
            let layout =
                lctx_native::originals::layout(&reader, &RecordId::new("original", source.0.hex()))
                    .await?
                    .ok_or(ModelError::Conflict("compiler original layout"))?;
            lctx_native::originals::validate_layout(&reader, &layout).await?;
            if !lctx_native::originals::header(&reader, &layout.blob).await? {
                return Err(ModelError::Conflict("compiler original incomplete blob"));
            }
            let length = i64::try_from(layout.blob.byte_len).map_err(ModelError::codec)?;
            let remaining = u64::try_from(length)
                .map_err(ModelError::codec)?
                .checked_sub(original.start)
                .ok_or(ModelError::Conflict("compiler original coverage"))?;
            if original.len as u64 != remaining.min(d::artifact::ARTIFACT_CHUNK_BYTES as u64) {
                return Err(ModelError::Conflict("compiler original coverage"));
            }
            // Reuse the independently checked physical-range reader; retain only one bounded
            // page, and hash it without reconstructing or storing a second raw chunk body.
            let mut hash = ContentHasher::default();
            let mut offset = 0usize;
            while offset < original.len {
                let len = (original.len - offset).min(256 << 10);
                let bytes = reader
                    .original_bytes(source, original.start + offset as u64, len)
                    .await?;
                hash.update(&bytes);
                offset += len;
            }
            if hash.finish() != original.digest {
                return Err(ModelError::Conflict("compiler original digest"));
            }
            Ok(())
        }
        .await;
        let mut completion = d::completion::Completion::default();
        completion.step("original backing reader close", reader.close().await);
        d::completion::complete(result, completion)
    }
    async fn registered_view(self: &Arc<Self>, view: &CompletedView) -> Result<(), ModelError> {
        view.validate()?;
        if let Some(known) = self
            .known_views
            .lock()
            .map_err(|_| ModelError::Conflict("native view owner"))?
            .get(&view.identity)
        {
            return if known == view {
                Ok(())
            } else {
                Err(ModelError::Conflict("completed view descriptor collision"))
            };
        }
        let mut b = Variables::new();
        b.insert("id", RecordId::new("compiler_view", view.identity.hex()));
        let mut response = self
            .read_client()?
            .query("SELECT * FROM $id")
            .bind(b)
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
        if rows.len() != 1 {
            return Err(ModelError::Conflict("unregistered native completed view"));
        }
        validate_state_row("compiler_view", &rows[0])?;
        if decode_descriptor::<CompletedView>(&rows[0])? != *view {
            return Err(ModelError::Conflict("unregistered native completed view"));
        }
        self.validate_view_contributions(std::slice::from_ref(view))
            .await?;
        self.remember_view(view)
    }
    async fn validate_view_contributions(
        self: &Arc<Self>,
        views: &[CompletedView],
    ) -> Result<(), ModelError> {
        // Loading a logical declaration is not attachment authority. Every requested
        // contributor must already have a selected local or exact per-view mapping.
        for view in views {
            self.view_owners(view)?;
        }
        Ok(())
    }

    fn remember_contributor(
        &self,
        logical: ContentHash,
        physical: ContentHash,
    ) -> Result<(), ModelError> {
        let mut known = self
            .known_contributors
            .lock()
            .map_err(|_| ModelError::Conflict("native contributor owner"))?;
        known.insert(logical, physical);
        Ok(())
    }
    fn view_owners(&self, view: &CompletedView) -> Result<Vec<ContentHash>, ModelError> {
        let selected = self
            .view_mappings
            .lock()
            .map_err(|_| ModelError::Conflict("native exact view mapping owner"))?;
        let local = self
            .known_contributors
            .lock()
            .map_err(|_| ModelError::Conflict("native contributor owner"))?;
        let mut owners = Vec::with_capacity(view.contributions.len());
        for logical in &view.contributions {
            let physical = if let Some((mappings, _)) = selected.get(&view.identity) {
                mappings
                    .binary_search_by_key(&(view.identity, *logical), |mapping| {
                        (mapping.view, mapping.logical)
                    })
                    .ok()
                    .map(|index| mappings[index].physical)
            } else {
                local.get(logical).copied()
            }
            .ok_or(ModelError::Conflict(
                "missing explicit selected contributor",
            ))?;
            owners.push(physical);
        }
        owners.sort_unstable();
        if owners.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ModelError::Conflict(
                "duplicate physical selected contributor",
            ));
        }
        Ok(owners)
    }

    fn remember_view(&self, view: &CompletedView) -> Result<(), ModelError> {
        let mut views = self
            .known_views
            .lock()
            .map_err(|_| ModelError::Conflict("native view owner"))?;
        if let Some(known) = views.get(&view.identity) {
            if known != view {
                return Err(ModelError::Conflict("completed view descriptor collision"));
            }
        } else {
            views.insert(view.identity, view.clone());
        }
        Ok(())
    }
    pub async fn bind(self: &Arc<Self>, binding: CompletedBinding) -> Result<(), ModelError> {
        let lease = self.admit_mutation("bind", true)?;
        let result = lease.within(self.bind_inner(binding)).await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
        result
    }
    async fn bind_inner(self: &Arc<Self>, binding: CompletedBinding) -> Result<(), ModelError> {
        self.check_failed()?;
        binding.validate()?;
        self.registered_view(&binding.view).await?;
        let mut row = Object::new();
        row.insert(
            "id",
            RecordId::new(
                "compiler_binding",
                attempt_binding(self.attempt, binding.key()).hex(),
            ),
        );
        row.insert(
            "descriptor",
            Bytes::from(serde_json::to_vec(&binding).map_err(ModelError::codec)?),
        );
        row.insert(
            "attempt",
            RecordId::new("native_attempt", self.attempt.hex()),
        );
        for (field, value) in binding_projection(&binding) {
            row.insert(field, value);
        }
        let mut b = Variables::new();
        b.insert("row", row);
        b.insert("frozen_binding", binding.boundary.is_some());
        if binding.boundary.is_some() {
            let mut lookup = Variables::new();
            lookup.insert(
                "id",
                RecordId::new(
                    "compiler_binding",
                    attempt_binding(self.attempt, binding.key()).hex(),
                ),
            );
            let mut response = self
                .client
                .query("SELECT * FROM $id")
                .bind(lookup)
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let stored: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
            if !stored.is_empty() {
                if stored.len() != 1 {
                    return Err(ModelError::Conflict("frozen native binding replacement"));
                }
                validate_state_row("compiler_binding", &stored[0])?;
                if decode_descriptor::<CompletedBinding>(&stored[0])? != binding {
                    return Err(ModelError::Conflict("frozen native binding replacement"));
                }
                return Ok(());
            }
        }
        let slot = RecordId::new(
            "compiler_binding",
            attempt_binding(self.attempt, binding.key()).hex(),
        );
        let owner = RecordId::new("native_attempt", self.attempt.hex());
        // The replaceable slot has its own terminal lifecycle. Canonical complete
        // descriptors retain the full meaning independently after physical completion.
        crate::control::guarded_effect_with_holds(&self.client,self.attempt,vec![slot.clone()],
            vec![(owner.clone(),slot),(owner,RecordId::new("compiler_view",binding.view.identity.hex()))],
            "IF $__owner.admitted { THROW 'native binding inventory admitted'; }; LET $binding_record=$row.id; LET $existing=SELECT * FROM ONLY $binding_record FOR UPDATE; IF $frozen_binding AND $existing!=NONE AND $existing!=$row { THROW 'frozen native binding replacement'; }; UPSERT $row.id CONTENT $row RETURN NONE; UPDATE $__attempt SET revision+=1 RETURN NONE",b).await?;
        Ok(())
    }
    pub async fn bindings(self: &Arc<Self>) -> Result<Vec<CompletedBinding>, ModelError> {
        let lease = self.admit(false, "bindings", false)?;
        let result = lease.within(self.bindings_inner()).await;
        lease.finish_with(result)
    }
    async fn bindings_inner(self: &Arc<Self>) -> Result<Vec<CompletedBinding>, ModelError> {
        if let Some(bindings) = self
            .published_bindings
            .lock()
            .map_err(|_| ModelError::Conflict("published bindings"))?
            .clone()
        {
            return Ok(bindings);
        }
        let mut stream = self.track_rows(NativeRows::new(
            self.read_client()?
                .query("SELECT * FROM compiler_binding WHERE attempt=$attempt ORDER BY id")
                .bind((
                    "attempt",
                    RecordId::new("native_attempt", self.attempt.hex()),
                ))
                .stream_items()
                .map_err(ModelError::codec)?,
            1,
        )?)?;
        let mut bindings = Vec::new();
        while let Some(row) = stream.next().await? {
            validate_state_row("compiler_binding", &row)?;
            let object = value_object(&row).ok_or(ModelError::Schema("native binding row"))?;
            let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
                return Err(ModelError::Schema("native binding descriptor"));
            };
            let binding: CompletedBinding =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            binding.validate()?;
            bindings.push(binding);
        }
        Ok(bindings)
    }
    /// Compact retained dependency inspection. No mutable scheduling authority is exposed.
    pub fn views<'a>(self: &'a Arc<Self>) -> BoxFuture<'a, Result<Vec<CompletedView>, ModelError>> {
        Box::pin(async move {
            let lease = self.admit(false, "views", false)?;
            let result = lease.within(self.views_inner()).await;
            lease.finish_with(result)
        })
    }
    async fn views_inner(self: &Arc<Self>) -> Result<Vec<CompletedView>, ModelError> {
        Ok(self.state_closure().await?.0.into_values().collect())
    }
    pub fn completed_state<'a>(
        self: &'a Arc<Self>,
        budget: &'a ResourceBudget,
    ) -> futures::future::BoxFuture<'a, Result<CompletedStateIdentity, ModelError>> {
        Box::pin(async move {
            let lease = self.admit(false, "completed_state", false)?;
            let result = lease.within(self.completed_state_inner(budget)).await;
            lease.finish_with(result)
        })
    }
    pub async fn completed_state_after_closure(
        self: &Arc<Self>,
        parent: &OperationLease,
        budget: &ResourceBudget,
    ) -> Result<CompletedStateIdentity, ModelError> {
        // The live borrowed parent covers all awaits. Public labels cannot spoof seal origin,
        // and a guard for a different attempt provides no descendant admission here.
        if !parent.finalization || parent.finished || !Arc::ptr_eq(&parent.owner, &self.admission) {
            return Err(ModelError::Conflict("native finalization parent"));
        }
        parent.within(self.completed_state_inner(budget)).await
    }
    async fn completed_state_inner(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<CompletedStateIdentity, ModelError> {
        // This owner was earned only after mutators drained and every selected
        // physical closure and logical family reached checked EOF. Semantic
        // persistence still rechecks its exact native epoch/incarnation by CAS.
        let completed = self
            .physical_completed
            .lock()
            .map_err(|_| ModelError::Conflict("physical completed identity owner"))?
            .clone();
        if let Some(completed) = completed {
            self.check_frozen_content()?;
            return CompletedStateIdentity::from_roots(&completed.completion().roots);
        }
        if let Some(published) = self
            .published_completion
            .lock()
            .map_err(|_| ModelError::Conflict("published completed identity owner"))?
            .as_ref()
        {
            return CompletedStateIdentity::from_roots(&published.completion().roots);
        }
        self.check_frozen_content()?;
        let completed = self.complete_physical_content(budget).await?;
        CompletedStateIdentity::from_roots(&completed.completion().roots)
    }
    // Import has just independently validated the actual stored type slices. Its checksum
    // pass still checks every framing/canonical row, without repeating model reconstruction.
    async fn state_identity_inner(
        self: &Arc<Self>,
        validate_backing: bool,
        budget: &ResourceBudget,
    ) -> Result<CompletedStateIdentity, ModelError> {
        self.wait_scans().await?;
        let selection = self.capture_state(budget).await?;
        self.state_identity_selected(validate_backing, &selection)
            .await
    }
    async fn state_identity_selected(
        self: &Arc<Self>,
        validate_backing: bool,
        selection: &AuditCapture,
    ) -> Result<CompletedStateIdentity, ModelError> {
        use d::recovery_closure::COMPLETED_FAMILIES;
        // Transport addresses are installation-independent, but spec-only addresses are
        // not the logical owner: the complete outcome and every output digest participate.
        let mut owner_charge =
            d::charged::StateCharge::new(&selection.budget, "completed logical owner mapping");
        let mut owners = BTreeMap::new();
        let mut contributions =
            self.track_rows(self.state_source("compiler_contribution", selection)?)?;
        while let Some(row) = contributions.next_native().await? {
            validate_state_row("compiler_contribution", &row)?;
            let Some(Value::RecordId(id)) = value_object(&row).and_then(|row| row.get("id")) else {
                return Err(ModelError::Schema("completed logical contribution owner"));
            };
            owner_charge.grow(
                crate::loader::native_bytes(&row)
                    .saturating_mul(32)
                    .saturating_add(2048),
            )?;
            let descriptor: CompletedContribution = decode_descriptor(&row)?;
            if owners.insert(id.clone(), descriptor).is_some() {
                return Err(ModelError::Conflict("completed duplicate native owner"));
            }
        }
        let mut roots = Vec::with_capacity(6);
        for (i, table) in STATE_TABLES.iter().enumerate() {
            let mut logical = AsyncPhysicalSort::new_registered_with_row_bytes(
                &selection.budget,
                self.blocking_owner()?,
                self.blocking_observer()?,
                d::resources::MAX_ROW_BYTES,
            )
            .await?;
            let mut pending = Vec::new();
            let mut pending_bytes = 0;
            if *table == "compiler_binding" {
                for binding in &selection.bindings {
                    let mut row = Object::new();
                    row.insert(
                        "descriptor",
                        Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?),
                    );
                    logical
                        .push(lctx_native::completed_logical_rows::normalize(
                            table,
                            &Value::Object(row),
                            &owners,
                        )?)
                        .await?;
                }
            } else {
                let mut rows = self.track_rows(self.state_source(table, selection)?)?;
                while let Some(row) = rows.next_native().await? {
                    validate_state_row(table, &row)?;
                    logical
                        .push(lctx_native::completed_logical_rows::normalize(
                            table, &row, &owners,
                        )?)
                        .await?;
                    if validate_backing && *table == "compiler_record" {
                        admit_backing_row(
                            &mut pending,
                            &mut pending_bytes,
                            row,
                            &selection.budget,
                        )?;
                    }
                }
            }
            if validate_backing && *table == "compiler_record" {
                validate_backing_batch(&pending, &selection.budget)?;
            }
            let prepared = logical.prepare_terminal().await?;
            let root = AsyncPhysicalSort::fold_union_registered(
                COMPLETED_FAMILIES[i],
                vec![prepared],
                &selection.budget,
                self.blocking_owner()?,
                self.blocking_observer()?,
                self.read_cancellation()?,
            )
            .await?;
            roots.push(root);
        }
        CompletedStateIdentity::from_roots(&d::completed_identity::CompletedFamilyRoots::new(
            roots
                .try_into()
                .map_err(|_| ModelError::Schema("completed logical six families"))?,
        )?)
    }
    /// Explicit detached transport only. Ordinary publication seals this database directly.
    async fn prepare_transport_inner(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<lctx_native::completed_transport::PreparedTransport, ModelError> {
        if !self.read_only.load(Ordering::Acquire) {
            self.check_frozen_content()?;
        }
        self.completed_state_inner(budget).await?;
        let physical = self
            .physical_completed
            .lock()
            .map_err(|_| ModelError::Conflict("transport physical owner"))?
            .clone();
        if let Some(physical) = physical {
            return lctx_native::completed_transport::prepare_physical(
                self.client.clone(),
                &physical,
                budget,
            )
            .await;
        }
        let published = self
            .published_completion
            .lock()
            .map_err(|_| ModelError::Conflict("transport publication owner"))?
            .clone()
            .ok_or(ModelError::Conflict(
                "transport requires exact protected completion",
            ))?;
        lctx_native::completed_transport::prepare_published(&published, budget).await
    }
    pub async fn transport_receipt(
        self: &Arc<Self>,
        budget: &ResourceBudget,
    ) -> Result<CompletedTransportReceipt, ModelError> {
        let lease = self.admit(false, "transport_receipt", false)?;
        let result = lease
            .within(async {
                self.prepare_transport_inner(budget)
                    .await?
                    .envelope()
                    .receipt()
            })
            .await;
        lease.finish_with(result)
    }
    pub async fn export_state(
        self: &Arc<Self>,
        path: &std::path::Path,
        budget: &ResourceBudget,
    ) -> Result<CompletedTransportReceipt, ModelError> {
        let lease = self.admit(false, "export_state", false)?;
        let result = lease.within(self.export_state_inner(path, budget)).await;
        lease.finish_with(result)
    }
    async fn export_state_inner(
        self: &Arc<Self>,
        path: &std::path::Path,
        budget: &ResourceBudget,
    ) -> Result<CompletedTransportReceipt, ModelError> {
        if !self.read_only.load(Ordering::Acquire) {
            self.check_frozen_content()?;
        }
        let expected = self.completed_state_inner(budget).await?;
        let transport = self.prepare_transport_inner(budget).await?;
        let bindings = self.bindings_inner().await?;
        let envelope = transport.envelope();
        let views = envelope
            .roots
            .iter()
            .flat_map(|root| std::iter::once(&root.view).chain(root.dependencies.iter()))
            .map(|view| (view.identity, view))
            .collect::<BTreeMap<_, _>>();
        let owners = transport
            .copies()
            .iter()
            .map(|copy| copy.physical)
            .collect::<Vec<_>>();
        let payloads = crate::selection::SelectedPayloads::for_cancellable_owners(
            self.client.clone(),
            &owners,
            self.read_cancellation()?,
            budget,
        )
        .await?;
        let mut prepared = Vec::with_capacity(6);
        let mut roots = Vec::with_capacity(6);
        for (index, table) in STATE_TABLES.iter().enumerate() {
            let run = if index == 1 || index == 5 {
                let runs = transport
                    .copies()
                    .iter()
                    .map(|copy| {
                        if index == 1 {
                            copy.memberships.clone()
                        } else {
                            copy.aliases.clone()
                        }
                    })
                    .collect::<Vec<_>>();
                PreparedRows::union_prepared(&runs, budget, || Ok(()))?
            } else {
                let mut logical = AsyncPhysicalSort::new_registered_with_row_bytes(
                    budget,
                    self.blocking_owner()?,
                    self.blocking_observer()?,
                    d::resources::MAX_ROW_BYTES,
                )
                .await?;
                if index == 0 {
                    for copy in &envelope.copies {
                        let mut row = Object::new();
                        row.insert(
                            "id",
                            RecordId::new(
                                "compiler_contribution",
                                copy.descriptor.identity()?.hex(),
                            ),
                        );
                        row.insert(
                            "descriptor",
                            Bytes::from(
                                serde_json::to_vec(&copy.descriptor).map_err(ModelError::codec)?,
                            ),
                        );
                        logical.push(Value::Object(row)).await?;
                    }
                } else if index == 2 {
                    for view in views.values() {
                        let mut row = Object::new();
                        row.insert("id", RecordId::new("compiler_view", view.identity.hex()));
                        row.insert(
                            "descriptor",
                            Bytes::from(serde_json::to_vec(view).map_err(ModelError::codec)?),
                        );
                        logical.push(Value::Object(row)).await?;
                    }
                } else if index == 4 {
                    for binding in &bindings {
                        let mut row = Object::new();
                        row.insert("id", RecordId::new("compiler_binding", binding.key().hex()));
                        row.insert(
                            "descriptor",
                            Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?),
                        );
                        logical.push(Value::Object(row)).await?;
                    }
                } else {
                    let mut source = self.track_rows(
                        payloads
                            .rows(table, "true", Variables::new(), vec![], "id", None)?
                            .with_row_bytes(d::resources::MAX_ROW_BYTES),
                    )?;
                    while let Some(row) = source.next_native().await? {
                        validate_state_row(table, &row)?;
                        logical.push(row).await?;
                    }
                }
                logical.prepare_terminal().await?
            };
            roots.push(
                AsyncPhysicalSort::fold_union_registered(
                    d::recovery_closure::COMPLETED_FAMILIES[index],
                    vec![run.clone()],
                    budget,
                    self.blocking_owner()?,
                    self.blocking_observer()?,
                    self.read_cancellation()?,
                )
                .await?,
            );
            prepared.push(run);
        }
        let actual =
            CompletedStateIdentity::from_roots(&d::completed_identity::CompletedFamilyRoots::new(
                roots
                    .try_into()
                    .map_err(|_| ModelError::Schema("portable preparation six families"))?,
            )?)?;
        if actual != expected {
            return Err(ModelError::Conflict(
                "actual portable content differs from frozen boundary",
            ));
        }
        let prepared: [PreparedRows; 6] = prepared
            .try_into()
            .map_err(|_| ModelError::Schema("transport six prepared families"))?;
        lctx_native::completed_transport::write_transport(path, &transport, &prepared, budget)
    }
    pub async fn import_state(
        self: &Arc<Self>,
        path: &std::path::Path,
        expected: &CompletedTransportReceipt,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let lease = self.admit_mutation("import_state", true)?;
        let result = lease
            .within(self.import_state_inner(path, expected, budget))
            .await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
        result
    }
    async fn import_state_inner(
        self: &Arc<Self>,
        path: &std::path::Path,
        expected: &CompletedTransportReceipt,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check_failed()?;
        let decoded = lctx_native::completed_transport::read_transport(path, expected, budget)?;
        let envelope = decoded.envelope();
        let _root_gates = self
            .root_gates(envelope.roots.iter().map(|root| root.view.identity), budget)
            .await?;
        if !self
            .known_views
            .lock()
            .map_err(|_| ModelError::Conflict("cold import destination"))?
            .is_empty()
            || !self
                .specifications
                .lock()
                .map_err(|_| ModelError::Conflict("cold import destination"))?
                .is_empty()
        {
            return Err(ModelError::Conflict(
                "cold import requires empty destination content",
            ));
        }
        let _inventory_charge = budget.reserve(
            "import fresh owner correspondence",
            envelope
                .copies
                .len()
                .saturating_mul(512)
                .saturating_add(envelope.roots.len().saturating_mul(128))
                .saturating_add(2048),
        )?;
        let mut owners = BTreeMap::new();
        for copy in &envelope.copies {
            let physical = self.register_transport_copy(copy, expected, budget).await?;
            owners.insert(copy.label, physical);
        }
        // Native backing and declarations are reconstructed from canonical data only.
        for index in [2usize, 3, 4] {
            let mut cursor = decoded.families()[index].cursor_with_budget(budget)?;
            let mut pending = vec![];
            let mut bytes = 0usize;
            let mut pending_charge =
                d::charged::StateCharge::new(budget, "import pending canonical rows");
            while let Some(value) = cursor.next_row()? {
                pending_charge.grow(
                    crate::loader::native_bytes(&value)
                        .saturating_mul(4)
                        .saturating_add(65536),
                )?;
                let value = if index == 2 {
                    let view: CompletedView = decode_descriptor(&value)?;
                    let mut row = Object::new();
                    row.insert("id", RecordId::new("compiler_view", view.identity.hex()));
                    row.insert(
                        "descriptor",
                        Bytes::from(serde_json::to_vec(&view).map_err(ModelError::codec)?),
                    );
                    for (key, value) in view_projection(&view)? {
                        row.insert(key, value);
                    }
                    self.remember_view(&view)?;
                    Value::Object(row)
                } else if index == 4 {
                    let binding: CompletedBinding = decode_descriptor(&value)?;
                    let mut row = Object::new();
                    row.insert(
                        "id",
                        RecordId::new(
                            "compiler_binding",
                            attempt_binding(self.attempt, binding.key()).hex(),
                        ),
                    );
                    row.insert(
                        "attempt",
                        RecordId::new("native_attempt", self.attempt.hex()),
                    );
                    row.insert(
                        "descriptor",
                        Bytes::from(serde_json::to_vec(&binding).map_err(ModelError::codec)?),
                    );
                    for (key, value) in binding_projection(&binding) {
                        row.insert(key, value);
                    }
                    Value::Object(row)
                } else {
                    value
                };
                bytes = bytes.saturating_add(crate::loader::native_bytes(&value));
                pending.push(value);
                if pending.len() >= d::resources::TRANSFER_ROWS
                    || bytes >= d::resources::TRANSFER_BYTES
                {
                    self.insert_transport_rows(std::mem::take(&mut pending), None, budget)
                        .await?;
                    pending_charge.release(pending_charge.reserved());
                    bytes = 0;
                }
            }
            if !pending.is_empty() {
                self.insert_transport_rows(pending, None, budget).await?;
            }
        }
        for copy in &envelope.copies {
            self.hold_contribution_inputs(owners[&copy.label], &copy.descriptor.spec)
                .await?;
        }
        for copy in decoded.copies() {
            let physical = owners[&copy.label];
            let owner = RecordId::new("compiler_contribution", physical.hex());
            let mut cursor = copy.memberships.cursor_with_budget(budget)?;
            let mut pending = vec![];
            let mut bytes = 0usize;
            let mut pending_charge =
                d::charged::StateCharge::new(budget, "import pending local memberships");
            while let Some(value) = cursor.next_row()? {
                pending_charge.grow(
                    crate::loader::native_bytes(&value)
                        .saturating_mul(2)
                        .saturating_add(1024),
                )?;
                let mut row = value
                    .as_object()
                    .ok_or(ModelError::Schema("import copy membership"))?
                    .clone();
                let (Some(Value::String(relation)), Some(Value::String(key))) =
                    (row.get("relation"), row.get("semantic_key"))
                else {
                    return Err(ModelError::Schema("import copy member identity"));
                };
                row.insert(
                    "id",
                    membership_id(physical, relation, &decode_nominal_key(key)?),
                );
                row.insert("contribution", owner.clone());
                let row = Value::Object(row);
                bytes = bytes.saturating_add(crate::loader::native_bytes(&row));
                pending.push(row);
                if pending.len() >= d::resources::TRANSFER_ROWS
                    || bytes >= d::resources::TRANSFER_BYTES
                {
                    self.insert_transport_rows(
                        std::mem::take(&mut pending),
                        Some(physical),
                        budget,
                    )
                    .await?;
                    pending_charge.release(pending_charge.reserved());
                    bytes = 0;
                }
            }
            if !pending.is_empty() {
                self.insert_transport_rows(pending, Some(physical), budget)
                    .await?;
            }
        }
        // Materialize the whole selected endpoint closure before any alias attachment:
        // copy labels are correspondence order, never cross-contributor topology.
        for copy in decoded.copies() {
            let physical = owners[&copy.label];
            let mut cursor = copy.aliases.cursor_with_budget(budget)?;
            let _target_charge = budget.reserve(
                "import alias target window",
                crate::loader::NATIVE_WINDOW_ROWS.saturating_mul(512),
            )?;
            let mut targets = std::collections::BTreeSet::new();
            while let Some(row) = cursor.next_row()? {
                let Some(Value::RecordId(target)) =
                    row.as_object().and_then(|row| row.get("target"))
                else {
                    return Err(ModelError::Schema("import alias target"));
                };
                targets.insert(target.clone());
                if targets.len() == crate::loader::NATIVE_WINDOW_ROWS {
                    self.restore_import_endpoints(
                        physical,
                        std::mem::take(&mut targets).into_iter().collect(),
                        budget,
                    )
                    .await?;
                }
            }
            if !targets.is_empty() {
                self.restore_import_endpoints(physical, targets.into_iter().collect(), budget)
                    .await?;
            }
        }
        for copy in decoded.copies() {
            let physical = owners[&copy.label];
            let owner = RecordId::new("compiler_contribution", physical.hex());
            let mut cursor = copy.aliases.cursor_with_budget(budget)?;
            while let Some(row) = cursor.next_row()? {
                let _alias_charge = budget.reserve(
                    "import local alias effect",
                    crate::loader::native_bytes(&row)
                        .saturating_mul(4)
                        .saturating_add(4096),
                )?;
                let object = row
                    .as_object()
                    .ok_or(ModelError::Schema("import local alias"))?;
                let (
                    Some(Value::RecordId(id)),
                    Some(Value::RecordId(source)),
                    Some(Value::RecordId(target)),
                ) = (object.get("id"), object.get("source"), object.get("target"))
                else {
                    return Err(ModelError::Schema("import local alias endpoints"));
                };
                crate::control::ensure_rows(&self.client, Some(self.attempt), vec![row.clone()])
                    .await?;
                crate::control::hold_many(
                    &self.client,
                    Some(self.attempt),
                    vec![
                        (owner.clone(), id.clone()),
                        (owner.clone(), target.clone()),
                        (id.clone(), source.clone()),
                    ],
                )
                .await?;
            }
        }
        let owner_records = owners
            .values()
            .map(|owner| RecordId::new("compiler_contribution", owner.hex()))
            .collect::<Vec<_>>();
        for window in owner_records.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            let mut vars = Variables::new();
            vars.insert("owners", window.to_vec());
            crate::control::effect(
                &self.client,
                Some(self.attempt),
                "UPDATE $owners SET completed=true RETURN NONE",
                vars,
            )
            .await?;
        }
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let observed = async {
            let mut incarnations = BTreeMap::new();
            for window in owner_records.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let records =
                    lctx_native::control::inspect_expected_window_read(&reader, window, budget)
                        .await?;
                for record in window {
                    let copy = records
                        .get(record)?
                        .ok_or(ModelError::Conflict("fresh imported copy missing"))?;
                    incarnations.insert(record.clone(), copy.incarnation());
                }
            }
            Ok(incarnations)
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step(
            "import unique copy incarnation reader close",
            reader.close().await,
        );
        let incarnations = d::completion::complete(observed, terminal)?;
        for root in &envelope.roots {
            let _mapping_charge = budget.reserve(
                "import root mapping assembly",
                root.slots
                    .len()
                    .saturating_mul(
                        std::mem::size_of::<lctx_native::installed_view::ContributorRealization>()
                            + 256,
                    )
                    .saturating_add(4096),
            )?;
            let mut mappings = Vec::with_capacity(root.slots.len());
            for slot in &root.slots {
                let physical = owners[&slot.copy];
                let record = RecordId::new("compiler_contribution", physical.hex());
                let incarnation = *incarnations.get(&record).ok_or(ModelError::Conflict(
                    "fresh imported copy incarnation missing",
                ))?;
                mappings.push(lctx_native::installed_view::ContributorRealization {
                    view: slot.view,
                    logical: slot.logical,
                    physical,
                    incarnation,
                });
            }
            // Logical view rows may have several exact local copies. Only this root's
            // explicit mappings select eligibility; the hold graph preserves every copy.
            for window in mappings.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                crate::control::hold_many(
                    &self.client,
                    Some(self.attempt),
                    window
                        .iter()
                        .map(|mapping| {
                            (
                                RecordId::new("compiler_view", mapping.view.hex()),
                                RecordId::new("compiler_contribution", mapping.physical.hex()),
                            )
                        })
                        .collect(),
                )
                .await?;
            }
            self.declare_view_inner(&root.view, &mappings, budget)
                .await?;
            self.remember_view_mapping(&root.view, &mappings, budget)?;
        }
        for root in &envelope.roots {
            self.install_view_under_gate(&root.view, budget).await?;
        }
        self.verify_state_inner(budget).await?;
        if self.state_identity_inner(false, budget).await? != expected.semantic {
            return Err(ModelError::Conflict(
                "completed transport independently installed semantic identity",
            ));
        }
        let held = envelope
            .roots
            .iter()
            .map(|root| {
                self.installed_views
                    .lock()
                    .map_err(|_| ModelError::Conflict("import installed roots"))?
                    .get(&root.view.identity)
                    .cloned()
                    .ok_or(ModelError::Conflict("import installed root missing"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if lctx_native::completed_transport::eligibility_selected(
            self.client.clone(),
            &held,
            budget,
        )
        .await?
            != expected.eligibility
        {
            return Err(ModelError::Conflict(
                "completed transport independently installed eligibility",
            ));
        }
        let mut specifications = self
            .specifications
            .lock()
            .map_err(|_| ModelError::Conflict("import copy completion ownership"))?;
        for physical in owners.values() {
            specifications
                .get_mut(physical)
                .ok_or(ModelError::Conflict("import copy unknown registration"))?
                .ownership = ContributionOwnership::Completed;
        }
        Ok(())
    }
    async fn register_transport_copy(
        self: &Arc<Self>,
        copy: &TransportCopy,
        expected: &CompletedTransportReceipt,
        budget: &ResourceBudget,
    ) -> Result<ContentHash, ModelError> {
        let mut sink = KeySink::new("completed-import-copy/v1");
        expected.envelope.encode(&mut sink);
        sink.part(b"label", &copy.label.to_le_bytes());
        let seed = sink.finish();
        let physical = attempt_contribution(self.attempt, seed);
        let descriptor = &copy.descriptor;
        struct EncodedSize(usize);
        impl std::io::Write for EncodedSize {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 = self.0.saturating_add(bytes.len());
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut size = EncodedSize(0);
        serde_json::to_writer(&mut size, descriptor).map_err(ModelError::codec)?;
        let _encoding = budget.reserve(
            "fresh copy descriptor encoding",
            size.0.saturating_mul(32).saturating_add(65536),
        )?;
        let retained = Arc::new(budget.reserve(
            "fresh copy retained specification",
            size.0.saturating_mul(8).saturating_add(2048),
        )?);
        let mut row = Object::new();
        row.insert("id", RecordId::new("compiler_contribution", physical.hex()));
        row.insert(
            "attempt",
            RecordId::new("native_attempt", self.attempt.hex()),
        );
        row.insert("transport_copy", seed.hex());
        row.insert(
            "descriptor",
            Bytes::from(serde_json::to_vec(descriptor).map_err(ModelError::codec)?),
        );
        row.insert(
            "spec",
            Bytes::from(serde_json::to_vec(&descriptor.spec).map_err(ModelError::codec)?),
        );
        row.insert("logical", descriptor.identity()?.hex());
        row.insert("outcome", descriptor.outcome);
        row.insert("completed", false);
        for (key, value) in contribution_projection(&descriptor.spec) {
            row.insert(key, value);
        }
        self.specifications
            .lock()
            .map_err(|_| ModelError::Conflict("import copy ownership"))?
            .insert(
                physical,
                RegisteredContribution {
                    spec: descriptor.spec.clone(),
                    ownership: ContributionOwnership::Registering,
                    _transport_charge: Some(retained),
                },
            );
        crate::control::ensure_rows(&self.client, Some(self.attempt), vec![Value::Object(row)])
            .await?;
        crate::control::hold_many(
            &self.client,
            Some(self.attempt),
            vec![(
                RecordId::new("native_attempt", self.attempt.hex()),
                RecordId::new("compiler_contribution", physical.hex()),
            )],
        )
        .await?;
        self.remember_contributor(descriptor.identity()?, physical)?;
        Ok(physical)
    }
    async fn insert_transport_rows(
        self: &Arc<Self>,
        rows: Vec<Value>,
        owner: Option<ContentHash>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let _metadata_charge = budget.reserve(
            "import bounded endpoint and hold handles",
            rows.len().saturating_mul(2048).saturating_add(4096),
        )?;
        let mut holds = vec![];
        let mut nodes = vec![];
        for row in &rows {
            let object = row
                .as_object()
                .ok_or(ModelError::Schema("import native row"))?;
            let Some(Value::RecordId(id)) = object.get("id") else {
                return Err(ModelError::Schema("import native row id"));
            };
            if let Some(owner) = owner {
                let owner = RecordId::new("compiler_contribution", owner.hex());
                holds.push((owner.clone(), id.clone()));
                let Some(Value::RecordId(node)) = object.get("node") else {
                    return Err(ModelError::Schema("import membership node"));
                };
                nodes.push(node.clone());
                holds.push((owner, node.clone()));
            } else {
                holds.push((
                    RecordId::new("native_attempt", self.attempt.hex()),
                    id.clone(),
                ));
            }
        }
        if rows
            .first()
            .and_then(Value::as_object)
            .and_then(|row| row.get("id"))
            .is_some_and(
                |id| matches!(id,Value::RecordId(id) if id.table.as_str()=="compiler_record"),
            )
        {
            validate_backing_batch(&rows, budget)?;
        }
        crate::control::ensure_rows(&self.client, Some(self.attempt), rows).await?;
        if let Some(owner) = owner {
            nodes.sort();
            nodes.dedup();
            self.restore_import_endpoints(owner, nodes, budget).await?;
        }
        crate::control::hold_many(&self.client, Some(self.attempt), holds).await
    }
    // Portable membership targets can be canonical graph payloads restored by the
    // adjacent graph ingress. Recreate the fresh contributor's exact endpoint holds
    // from validated canonical bytes rather than inheriting source-installation edges.
    async fn restore_import_endpoints(
        self: &Arc<Self>,
        contribution: ContentHash,
        nodes: Vec<RecordId>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        let reader = if let Some(flag) = self.read_cancellation()? {
            reader.with_read_cancellation(flag)
        } else {
            reader
        };
        let result = async {
            let mut variables = Variables::new();
            variables.insert("ids", nodes.clone());
            let mut stream = reader
                .stream_prepared(lctx_native::prepared::PreparedQuery::new(
                    variables,
                    vec![],
                    vec!["SELECT * FROM $ids".into()],
                )?)?
                .with_row_bytes(d::resources::MAX_ROW_BYTES);
            let result = async {
                let mut pending = Vec::new();
                let mut charge =
                    d::charged::StateCharge::new(budget, "import canonical endpoint window");
                let mut seen = std::collections::BTreeSet::new();
                while let Some(row) = stream.next().await? {
                    let Some(Value::RecordId(node)) =
                        value_object(&row).and_then(|row| row.get("id"))
                    else {
                        return Err(ModelError::Schema("import backing identity"));
                    };
                    if nodes.binary_search(node).is_err() || !seen.insert(node.clone()) {
                        return Err(ModelError::Conflict("import backing foreign/duplicate"));
                    }
                    let (endpoints, _, _decode) =
                        lctx_native::completed_rows::canonical_backing_endpoints(&row, budget)?;
                    for endpoint in endpoints {
                        let bytes = crate::loader::native_bytes(&endpoint)
                            .saturating_mul(8)
                            .saturating_add(512);
                        if !pending.is_empty()
                            && (pending.len() >= crate::loader::NATIVE_WINDOW_ROWS
                                || charge.reserved().saturating_add(bytes)
                                    > d::resources::TRANSFER_BYTES)
                        {
                            WRITING_CONTRIBUTION
                                .scope(
                                    contribution,
                                    self.ensure_physical_rows(
                                        std::mem::take(&mut pending),
                                        Some(budget),
                                    ),
                                )
                                .await?;
                            charge = d::charged::StateCharge::new(
                                budget,
                                "import canonical endpoint window",
                            );
                        }
                        charge.grow(bytes)?;
                        pending.push(endpoint);
                    }
                }
                if seen.len() != nodes.len() {
                    return Err(ModelError::Conflict("import backing missing"));
                }
                WRITING_CONTRIBUTION
                    .scope(
                        contribution,
                        self.ensure_physical_rows(pending, Some(budget)),
                    )
                    .await
            }
            .await;
            let mut terminal = d::completion::Completion::default();
            terminal.step(
                "import backing endpoint stream drain",
                stream.drain_transport().await,
            );
            d::completion::complete(result, terminal)
        }
        .await;
        let mut terminal = d::completion::Completion::default();
        terminal.step("import endpoint reader close", reader.close().await);
        d::completion::complete(result, terminal)
    }
    pub async fn scan_rows(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        columns: Option<&[String]>,
        predicate: Option<NativePredicate>,
        budget: &ResourceBudget,
    ) -> Result<CompilerRows, ModelError> {
        self.scan_rows_setup(view, relation, columns, predicate, budget, None)
            .await
    }
    /// A provider borrows the exact opaque capability it already owns. Delayed
    /// execution checks current source authority and retains its capability/lease.
    pub(crate) async fn scan_rows_with_installed(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        columns: Option<&[String]>,
        predicate: Option<NativePredicate>,
        budget: &ResourceBudget,
        installed: Arc<lctx_native::installed_view::HeldInstalledView>,
    ) -> Result<CompilerRows, ModelError> {
        self.scan_rows_setup(view, relation, columns, predicate, budget, Some(installed))
            .await
    }
    async fn scan_rows_setup(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        columns: Option<&[String]>,
        predicate: Option<NativePredicate>,
        budget: &ResourceBudget,
        installed: Option<Arc<lctx_native::installed_view::HeldInstalledView>>,
    ) -> Result<CompilerRows, ModelError> {
        // These owned copies can outlive the consumer that admitted the scan. Retain
        // their charge independently of a DataFusion window's cancellation lifetime.
        let bytes = view
            .contributions
            .len()
            .saturating_mul(64)
            .saturating_add(view.relation.len())
            .saturating_add(columns.map_or(0, |columns| {
                columns
                    .iter()
                    .map(|column| size_of::<String>().saturating_add(column.len()))
                    .fold(0, usize::saturating_add)
            }))
            .saturating_add(predicate.as_ref().map_or(0, read_predicate_bytes))
            .saturating_add(1024);
        let retained = budget.reserve("compiler-read-setup", bytes)?;
        let view = view.clone();
        let relation = relation.clone();
        let columns = columns.map(<[String]>::to_vec);
        let store = self.clone();
        let budget = budget.clone();
        self.retain_read_setup(
            "scan_rows",
            Box::pin(async move {
                store
                    .scan_rows_inner(
                        &view,
                        &relation,
                        columns.as_deref(),
                        predicate,
                        &budget,
                        installed,
                    )
                    .await
                    .map(|rows| rows.with_setup(Some(Arc::new(retained))))
            }),
        )
        .await
    }
    /// Read only primitive membership/content premises under an exact immutable view.
    /// These tokens do not admit payload semantics or establish a selector's completeness.
    pub fn row_tokens<'a>(
        self: &'a Arc<Self>,
        view: &'a CompletedView,
        relation: &'a Relation,
        keys: &'a [[u8; 16]],
        budget: &'a ResourceBudget,
    ) -> BoxFuture<'a, Result<Vec<([u8; 16], ContentHash)>, ModelError>> {
        Box::pin(async move {
            let lease = self.admit(false, "row_tokens", false)?;
            let result = lease
                .within(self.row_tokens_inner(view, relation, keys, budget))
                .await;
            lease.finish_with(result)
        })
    }
    async fn row_tokens_inner(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        keys: &[[u8; 16]],
        budget: &ResourceBudget,
    ) -> Result<Vec<([u8; 16], ContentHash)>, ModelError> {
        self.check_failed()?;
        self.registered_view(view).await?;
        if view.relation != relation.name() || keys.windows(2).any(|w| w[0] >= w[1]) {
            return Err(ModelError::Conflict("selected content-token binding"));
        }
        let installed = self.installed_view(view, budget).await?;
        let _charge = budget.reserve(
            "native-selected-content-tokens",
            keys.len().saturating_mul(256).saturating_add(4096),
        )?;
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        // A demand at least as large as the view may reuse its compact run. Sparse
        // demand retains scalar access even if a broad run already exists.
        let prepared = self
            .membership_preparations
            .lock()
            .map_err(|_| ModelError::Conflict("native membership preparation"))?
            .get(&view.identity)
            .filter(|(_, _, owner)| {
                owner.shares_pool(budget)
                    && u64::try_from(keys.len()).unwrap_or(u64::MAX) >= view.rows
            })
            .map(|(future, _, _)| future.clone());
        if let Some(prepared) = prepared {
            let prepared = prepared.await.map_err(ModelError::SharedCause)?;
            let mut cursor = AsyncOrderedCandidates::new_registered(
                prepared,
                budget,
                self.blocking_owner()?,
                self.blocking_observer()?,
            )
            .await?;
            let mut position = 0;
            loop {
                let rows = cursor.next_batch(crate::loader::NATIVE_WINDOW_ROWS).await?;
                if rows.is_empty() {
                    break;
                }
                for candidate in rows {
                    while position < keys.len() && keys[position] < candidate.key {
                        position += 1;
                    }
                    if position == keys.len() {
                        cursor.drain().await?;
                        return Ok(result);
                    }
                    if keys[position] == candidate.key {
                        result.push((
                            candidate.key,
                            candidate
                                .content
                                .ok_or(ModelError::Schema("verified membership content"))?,
                        ));
                        position += 1;
                    }
                }
            }
        } else {
            // Exact point IDs are in the held physical realization, never a global
            // membership index or one scalar query per possible producer.
            for window in keys.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let members = installed
                    .members(&self.installed_member_reader(budget)?, window, budget)
                    .await?;
                result.extend(
                    members
                        .rows()
                        .iter()
                        .map(|member| (member.key, member.content)),
                );
            }
        }
        Ok(result)
    }
    fn installed_member_reader(
        &self,
        budget: &ResourceBudget,
    ) -> Result<crate::NativeReader<()>, ModelError> {
        self.check_read_admission()?;
        let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
        Ok(match self.read_cancellation()? {
            Some(flag) => reader.with_read_cancellation(flag),
            None => reader,
        })
    }
    async fn prepare_memberships(
        self: &Arc<Self>,
        view: &CompletedView,
        budget: &ResourceBudget,
    ) -> Result<PreparedCandidates, ModelError> {
        let binding = self.producing_binding()?;
        let identity = view.identity;
        let installed_owner = self
            .installed_views
            .lock()
            .map_err(|_| ModelError::Conflict("prepared membership installed owner"))?
            .get(&identity)
            .cloned();
        let (future, retained) = {
            let mut preparations = self
                .membership_preparations
                .lock()
                .map_err(|_| ModelError::Conflict("native membership preparation"))?;
            if preparations
                .get(&view.identity)
                .is_some_and(|(_, _, owner)| !owner.shares_pool(budget))
            {
                // Existing borrowers retain their original immutable run and charge. A new
                // operation cannot use another pool's retained preparation as free capacity.
                preparations.remove(&view.identity);
            }
            if !preparations.contains_key(&view.identity) {
                let retained = Arc::new(
                    budget.reserve(
                        "native-membership-preparation-entry",
                        view.contributions
                            .len()
                            .saturating_mul(64)
                            .saturating_add(2048),
                    )?,
                );
                let store = self.clone();
                let view = view.clone();
                let preparation_pool = budget.clone();
                let budget = budget.clone();
                let lease = self.retained_scan()?;
                let task = self.runtime.spawn(async move {
                    let result = lease.within(producing_future(binding, async {
                        let _charge = budget.reserve("native-membership-preparation", view.contributions.len().saturating_mul(64).saturating_add(1024))?;
                        let owners = store.view_owners(&view)?;
                        let mut sorted = AsyncCandidateSort::new_registered(&budget, store.blocking_owner()?, store.blocking_observer()?).await?;
                        for window in owners.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                            let _window = budget.reserve("native membership owner window", window.len().saturating_mul(512).saturating_add(4096))?;
                            let mut bindings = Variables::new();
                            bindings.insert("relation", view.relation.clone());
                            bindings.insert("owners", window.iter().map(|owner|RecordId::new("compiler_contribution",owner.hex())).collect::<Vec<_>>());
                            let stream = store.read_client()?.query("SELECT id,contribution,relation,semantic_key,node,content FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $owners AND relation=$relation").bind(bindings).stream_items().map_err(ModelError::codec)?;
                            let mut rows = store.track_rows(NativeRows::new(stream, 1)?)?;
                            rows.installed=installed_owner.clone();
                            while let Some(row) = rows.next_native().await? {
                                let candidate = decode_membership(row, &view.relation, window, false)?.ok_or(ModelError::Conflict("exact view membership"))?;
                                sorted.push(candidate).await?;
                            }
                        }
                        sorted.prepare().await
                    })).await;
                    lease.finish_with(result).map_err(Arc::new)
                }.in_current_span().with_current_subscriber());
                let future = async move {
                    task.await
                        .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))?
                }
                .boxed()
                .shared();
                preparations.insert(identity, (future, retained, preparation_pool));
            }
            let (future, retained, _) = &preparations[&identity];
            (future.clone(), retained.clone())
        };
        let result = future.await.map_err(ModelError::SharedCause);
        // A local preparation refusal belongs to this attempt, not to the immutable
        // view. A stale waiter must not evict a newer attempt for the same view.
        // Native failures remain sticky through the existing operation admission.
        if result.is_err()
            && let Ok(mut preparations) = self.membership_preparations.lock()
            && preparations
                .get(&identity)
                .is_some_and(|(_, current, _)| Arc::ptr_eq(current, &retained))
        {
            preparations.remove(&identity);
        }
        result
    }
    async fn scan_rows_inner(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        columns: Option<&[String]>,
        predicate: Option<NativePredicate>,
        budget: &ResourceBudget,
        installed: Option<Arc<lctx_native::installed_view::HeldInstalledView>>,
    ) -> Result<CompilerRows, ModelError> {
        self.check_failed()?;
        self.registered_view(view).await?;
        self.scan_bound_rows_inner(view, relation, columns, predicate, budget, installed)
            .await
    }
    // Callers have either checked a registered view or reconstructed one exact singleton
    // from this store's validated completed descriptor; no public view bypass reaches here.
    async fn scan_bound_rows_inner(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        columns: Option<&[String]>,
        predicate: Option<NativePredicate>,
        budget: &ResourceBudget,
        supplied: Option<Arc<lctx_native::installed_view::HeldInstalledView>>,
    ) -> Result<CompilerRows, ModelError> {
        if view.relation != relation.name() {
            return Err(ModelError::Conflict("native view relation"));
        }
        let mut b = Variables::new();
        b.insert("relation", relation.name().to_string());
        b.insert(
            "contributions",
            view.contributions
                .iter()
                .map(ContentHash::hex)
                .collect::<Vec<_>>(),
        );
        let empty = match &predicate {
            None => view.rows == 0,
            Some(NativePredicate::Keys(keys) | NativePredicate::KeysSql { keys, .. }) => {
                keys.is_empty()
            }
            Some(
                NativePredicate::Field { values, .. } | NativePredicate::FieldSql { values, .. },
            ) => values.is_empty(),
            Some(NativePredicate::Sql { .. }) => false,
        };
        let mut explicit_keys = None;
        let mut atomic_field = None;
        let mut preparation = Vec::new();
        let predicate = match predicate {
            None => "true".into(),
            Some(NativePredicate::Keys(keys)) => {
                explicit_keys = Some(keys);
                "true".into()
            }
            Some(NativePredicate::KeysSql {
                keys,
                sql,
                bindings,
                preparation: prepared,
            }) => {
                explicit_keys = Some(keys);
                b.extend(bindings);
                preparation = prepared;
                sql
            }
            Some(NativePredicate::Field { field, values }) => {
                if !relation.fields().iter().any(|f| f.name() == field) {
                    return Err(ModelError::Conflict("native selected field"));
                }
                if relation.name() == d::artifact::ArtifactChunk::NAME && field == "body" {
                    return Err(ModelError::Conflict(
                        "original bytes are hydrated after native selection",
                    ));
                }
                let atomic = crate::schema::atomic_scope_field(relation.name(), field.as_str())
                    && !values
                        .iter()
                        .any(|value| matches!(value, Value::Null | Value::None));
                if atomic {
                    atomic_field = Some((field.clone(), values.clone()));
                }
                b.insert("values", values);
                if atomic {
                    crate::prepared::prepare_scope(
                        &mut preparation,
                        "__compiler_scope_filter",
                        "$relation",
                        &field,
                        "$values",
                    );
                    "scope_keys CONTAINSANY $__compiler_scope_filter".into()
                } else {
                    format!("body.`{field}` IN $values")
                }
            }
            Some(NativePredicate::FieldSql {
                field,
                values,
                sql,
                bindings,
                preparation: prepared,
            }) => {
                if !relation.fields().iter().any(|f| f.name() == field)
                    || !crate::schema::atomic_scope_field(relation.name(), field.as_str())
                    || values
                        .iter()
                        .any(|value| matches!(value, Value::Null | Value::None))
                {
                    return Err(ModelError::Conflict("native selected atomic field"));
                }
                atomic_field = Some((field.clone(), values.clone()));
                b.extend(bindings);
                preparation = prepared;
                b.insert("values", values);
                crate::prepared::prepare_scope(
                    &mut preparation,
                    "__compiler_scope_filter",
                    "$relation",
                    &field,
                    "$values",
                );
                format!("scope_keys CONTAINSANY $__compiler_scope_filter AND ({sql})")
            }
            Some(NativePredicate::Sql {
                sql,
                bindings,
                preparation: prepared,
            }) => {
                b.extend(bindings);
                preparation = prepared;
                sql
            }
        };
        let fields = columns.map(|c| c.to_vec()).unwrap_or_else(|| {
            relation
                .schema()
                .fields()
                .iter()
                .map(|f| f.name().clone())
                .collect()
        });
        let projections = projected_fields(relation, &fields)?;
        let supplied_cap = supplied.is_some();
        let installed = if let Some(installed) = supplied {
            if installed.completion().view != *view
                || !self
                    .installed_views
                    .lock()
                    .map_err(|_| ModelError::Conflict("provider installed owner"))?
                    .get(&view.identity)
                    .is_some_and(|owned| Arc::ptr_eq(owned, &installed))
            {
                return Err(ModelError::Conflict(
                    "provider requires this store's exact held selector",
                ));
            }
            installed
        } else {
            self.installed_view(view, budget).await?
        };
        let mut setup_owner = CompilerDrainOwners {
            installed: Some(installed.clone()),
            source_hold: None,
            confirmed: false,
        };
        let result=async {
        // Exact empty demand still passes the owner, view and shape checks above. There is
        // no physical stream to hold or drain, and no fabricated transport receipt. Internal
        // SQL predicates keep their ordinary execution/error semantics.
        if supplied_cap {
            // The captured provider owns this source capability, but every delayed
            // execution still checks the actual source pin and current installation.
            // Its immutable closure is already earned beneath that live pin; independent
            // receiving/final boundaries retain full exact recapture and CAS.
            // The provider's captured consumer budget owns this check and setup;
            // the source capability retains its original independent pool charges.
            let reader=crate::NativeReader::private(self.client.clone()).with_budget(budget);
            let result=installed.ensure_borrowed_current(&reader,budget).await;
            let mut terminal=d::completion::Completion::default();
            terminal.step("provider held-selector read close",reader.close().await);
            d::completion::complete(result,terminal)?;
        }
        if empty {
            return Ok(CompilerRows {
                descendant: None,
                producing: self.producing_binding()?,
                rows: None,
                selection: None,
                graph: None,
                store: self.clone(),
                lease: None,
                setup: None,
                installed: None,
            source_hold: None,
            });
        }
        if let Some(mut keys) = explicit_keys {
            keys.sort_unstable();
            keys.dedup();
            return self.track_selection(
                SelectionKeys::Known(keys.into_iter()),
                view,
                installed,
                relation,
                projections.join(","),
                predicate,
                b,
                preparation,
                budget,
            );
        }
        let keys = if let Some((field, values)) = atomic_field {
            let owners = self.view_owners(view)?;
            let mut sorted = AsyncCandidateSort::new_registered(
                budget,
                self.blocking_owner()?,
                self.blocking_observer()?,
            )
            .await?;
            let table = crate::schema::ScopeTable::for_relation(relation.name())?.name();
            // Scope values are bounded, and exact ownership is proved before nominal
            // deduplication so unrelated retained revisions cannot manufacture a collision.
            for owner in &owners {
                for values in values.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                    let mut bindings = Variables::new();
                    bindings.insert("relation", relation.name().to_string());
                    bindings.insert("values", values.to_vec());
                    bindings.insert("owner", RecordId::new("compiler_contribution", owner.hex()));
                    let constants =
                        crate::prepared::scope_constants("$relation", &field, "$values");
                    let sql = format!(
                        "LET $__compiler_scope = {constants}; SELECT semantic_type AS relation,semantic_key,id AS node FROM {table} WITH INDEX by_scope WHERE scope_keys CONTAINSANY $__compiler_scope AND semantic_type=$relation AND array::len((SELECT VALUE id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation AND semantic_key=$parent.semantic_key AND node=$parent.id LIMIT 1))>0"
                    );
                    let mut rows = self.track_rows(prepared_rows(
                        &self.client,
                        sql,
                        bindings,
                        2,
                        self.read_cancellation()?.as_ref(),
                    )?)?;
                    rows.installed=Some(installed.clone());
                    while let Some(row) = rows.next_native().await? {
                        sorted.push(decode_candidate(row, relation.name())?).await?;
                    }
                }
            }
            SelectionKeys::Unverified(sorted.finish().await?)
        } else {
            let prepared = self.prepare_memberships(view, budget).await?;
            SelectionKeys::Verified(
                AsyncOrderedCandidates::new_registered(
                    prepared,
                    budget,
                    self.blocking_owner()?,
                    self.blocking_observer()?,
                )
                .await?,
            )
        };
        self.track_selection(
            keys,
            view,
            installed,
            relation,
            projections.join(","),
            predicate,
            b,
            preparation,
            budget,
        )
        }.await;
        setup_owner.confirmed = result
            .as_ref()
            .err()
            .is_none_or(|error| error.permits_storage_cleanup());
        result
    }
    pub async fn table_provider(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: Relation,
        budget: d::resources::ResourceBudget,
        batch_rows: usize,
    ) -> Result<Arc<dyn datafusion::catalog::TableProvider>, ModelError> {
        crate::compiler_provider::table_provider(
            self.clone(),
            view.clone(),
            relation,
            budget,
            batch_rows,
        )
        .await
    }
    pub async fn scan_batches(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        projection: Option<Vec<usize>>,
        predicate: Option<NativePredicate>,
        budget: &d::resources::ResourceBudget,
        batch_rows: usize,
    ) -> Result<datafusion::physical_plan::SendableRecordBatchStream, ModelError> {
        crate::compiler_provider::scan_batches(
            self.clone(),
            view.clone(),
            relation.clone(),
            projection,
            predicate,
            budget.clone(),
            batch_rows,
        )
        .await
    }
}

fn projected_fields(relation: &Relation, fields: &[String]) -> Result<Vec<String>, ModelError> {
    let mut projections = Vec::new();
    for field in fields {
        if field == "id" {
            projections.push("semantic_key AS id".to_string());
        } else if relation.name() == d::artifact::ArtifactChunk::NAME && field == "body" {
            projections.push("(SELECT * FROM original_chunk WITH INDEX position WHERE blob=$parent.body.original.blob AND start >= $parent.body.start AND start < $parent.body.start+$parent.body.len ORDER BY start) AS __original_chunks, (SELECT * FROM ONLY $parent.body.original) AS __original_layout, (SELECT * FROM ONLY $parent.body.original.blob) AS __original_blob, (SELECT * FROM ONLY $parent.body.original.artifact) AS __original_artifact, body.original AS __original_source_id, body.start AS __original_start, body.len AS __original_len, body.digest AS __original_digest".to_string());
        } else if relation.fields().iter().any(|f| f.name() == field) {
            projections.push(format!("body.`{field}` AS `{field}`"));
        } else {
            return Err(ModelError::Conflict("native projection field"));
        }
    }
    if projections.is_empty() {
        projections.push("semantic_key AS __row".into());
    }
    Ok(projections)
}

fn prepared_rows(
    client: &Arc<Surreal<Client>>,
    sql: String,
    bindings: Variables,
    statements: usize,
    cancellation: Option<&Arc<AtomicBool>>,
) -> Result<NativeRows, ModelError> {
    crate::prepared::PreparedQuery::from_sql(sql, bindings, statements, vec![statements - 1])?
        .stream_cancellable(client, cancellation)
}

type ReadSetupCharge = Arc<Box<dyn Reservation>>;
// These exact sources survive setup and background drains, including interrupted
// tasks. Unconfirmed task destruction keeps terminal recovery pins durable.
struct CompilerDrainOwners {
    installed: Option<Arc<lctx_native::installed_view::HeldInstalledView>>,
    source_hold: Option<Arc<CandidateCompletion>>,
    confirmed: bool,
}
impl Drop for CompilerDrainOwners {
    fn drop(&mut self) {
        if !self.confirmed {
            if let Some(installed) = &self.installed {
                installed.retain_unknown();
            }
            if let Some(source) = &self.source_hold {
                source.retain_unknown();
            }
        }
    }
}
pub struct CompilerRows {
    descendant: Option<DescendantBinding>,
    producing: Option<ProducingBinding>,
    rows: Option<NativeRows>,
    selection: Option<Box<SelectedRows>>,
    graph: Option<Box<GraphRows>>,
    store: Arc<NativeCompilerStore>,
    lease: Option<OperationLease>,
    setup: Option<ReadSetupCharge>,
    installed: Option<Arc<lctx_native::installed_view::HeldInstalledView>>,
    source_hold: Option<Arc<CandidateCompletion>>,
}
impl CompilerRows {
    fn with_setup(mut self, setup: Option<ReadSetupCharge>) -> Self {
        // An exact empty read has no retained query state or transport ownership.
        self.setup = if self.lease.is_some() { setup } else { None };
        if let Some(selection) = &mut self.selection {
            selection.setup = self.setup.clone();
        }
        if let Some(graph) = &mut self.graph {
            graph.setup = self.setup.clone();
        }
        self
    }
    pub async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        let descendant = self.descendant.clone();
        let future = producing_future(self.producing.clone(), self.next_inner());
        match descendant {
            Some(binding) => DESCENDANT_WORK.scope(binding, future).await,
            None => future.await,
        }
    }
    async fn next_inner(&mut self) -> Result<Option<Value>, ModelError> {
        if let Some(graph) = &mut self.graph {
            let result = graph.next().await;
            if result.is_err() {
                self.store.fail();
            }
            if !matches!(&result, Ok(Some(_))) {
                self.graph.take();
                self.setup.take();
                if let Some(lease) = self.lease.take() {
                    return lease.finish_with(result);
                }
            }
            return result;
        }
        if let Some(selection) = &mut self.selection {
            let result = selection.next().await;
            if result
                .as_ref()
                .err()
                .is_some_and(|error| !error.permits_storage_cleanup())
            {
                selection.installed.retain_unknown();
            }
            if result.is_err() {
                self.store.fail();
            }
            if !matches!(&result, Ok(Some(_))) {
                self.selection.take();
                self.setup.take();
                if let Some(lease) = self.lease.take() {
                    return lease.finish_with(result);
                }
            }
            return result;
        }
        self.next_native().await
    }
    async fn next_native(&mut self) -> Result<Option<Value>, ModelError> {
        let Some(rows) = &mut self.rows else {
            return Ok(None);
        };
        let result = rows
            .next()
            .await
            .and_then(|row| row.map(hydrate_original_row).transpose());
        let result = match result {
            Ok(row) => Ok(row),
            Err(error) => {
                self.store.fail();
                let primary = rows.remember_failure(error);
                let mut completion = d::completion::Completion::default();
                completion.step(
                    "native stream transport drainage",
                    rows.drain_transport().await,
                );
                d::completion::complete(Err(primary), completion)
            }
        };
        if !matches!(&result, Ok(Some(_))) {
            self.rows.take();
            self.setup.take();
            if result
                .as_ref()
                .err()
                .is_some_and(|error| !error.permits_storage_cleanup())
            {
                if let Some(installed) = &self.installed {
                    installed.retain_unknown();
                }
                if let Some(source) = &self.source_hold {
                    source.retain_unknown();
                }
            }
            self.installed.take();
            self.source_hold.take();
            if let Some(lease) = self.lease.take() {
                return lease.finish_with(result);
            }
        }
        result
    }
}
impl Drop for CompilerRows {
    fn drop(&mut self) {
        let mut retained = self.setup.take();
        let installed = self.installed.take();
        let source_hold = self.source_hold.take();
        if self.selection.take().is_some() | self.graph.take().is_some() {
            drop(retained.take());
            if let Some(lease) = self.lease.take() {
                lease.finish();
            }
        }
        if let Some(mut rows) = self.rows.take() {
            let mut sources = CompilerDrainOwners {
                installed,
                source_hold,
                confirmed: false,
            };
            let store = self.store.clone();
            let lease = self.lease.take();
            // Cleanup inherits drop-time context. Rows do not retain a separate diagnostic
            // origin when a consumer drops them outside the scoped dispatcher.
            self.store.runtime.spawn(
                async move {
                    loop {
                        match rows.next().await {
                            Ok(Some(_)) => {}
                            Ok(None) => {
                                sources.confirmed = true;
                                break;
                            }
                            Err(error) => {
                                let primary_confirmed = error.permits_storage_cleanup();
                                store.record_background_failure(Arc::new(error));
                                match rows.drain_transport().await {
                                    Ok(()) => sources.confirmed = primary_confirmed,
                                    Err(error) => {
                                        sources.confirmed =
                                            primary_confirmed && error.permits_storage_cleanup();
                                        store.record_background_failure(Arc::new(error));
                                    }
                                }
                                break;
                            }
                        }
                    }
                    drop(rows);
                    drop(retained);
                    drop(sources);
                    if let Some(lease) = lease {
                        lease.finish();
                    }
                }
                .in_current_span()
                .with_current_subscriber(),
            );
        }
    }
}

/// One bounded nominal-key window. Unary equality-prefix branches examine actual
/// memberships; only a single verified owner uses deterministic point IDs.
struct MembershipLookup {
    store: Arc<NativeCompilerStore>,
    relation: String,
    keys: Vec<[u8; 16]>,
    owners: Vec<ContentHash>,
    offset: usize,
    current: Option<CompilerRows>,
    nodes: BTreeMap<String, Candidate>,
    setup: Option<ReadSetupCharge>,
    installed: Option<Arc<lctx_native::installed_view::HeldInstalledView>>,
}
impl MembershipLookup {
    fn new(
        store: Arc<NativeCompilerStore>,
        relation: String,
        keys: Vec<[u8; 16]>,
        owners: Vec<ContentHash>,
    ) -> Self {
        Self {
            store,
            relation,
            keys,
            owners,
            offset: 0,
            current: None,
            nodes: BTreeMap::new(),
            setup: None,
            installed: None,
        }
    }
    async fn collect(&mut self) -> Result<BTreeMap<String, Candidate>, ModelError> {
        loop {
            if let Some(rows) = &mut self.current {
                while let Some(row) = rows.next_native().await? {
                    let Some(candidate) = decode_membership(
                        row,
                        &self.relation,
                        &self.owners,
                        self.owners.len() != 1,
                    )?
                    else {
                        continue;
                    };
                    if self.keys.binary_search(&candidate.key).is_err() {
                        return Err(ModelError::Conflict("selected membership key"));
                    }
                    let key = hex::encode(candidate.key);
                    if let Some(old) = self.nodes.insert(key, candidate.clone())
                        && old != candidate
                    {
                        return Err(ModelError::Conflict("selected nominal backing collision"));
                    }
                }
                self.current.take();
            }
            if self.owners.is_empty() || self.offset == self.keys.len() {
                return Ok(std::mem::take(&mut self.nodes));
            }
            let mut bindings = Variables::new();
            let sql;
            if self.owners.len() == 1 {
                let end = self
                    .offset
                    .saturating_add(crate::loader::NATIVE_WINDOW_ROWS)
                    .min(self.keys.len());
                let ids = self.keys[self.offset..end]
                    .iter()
                    .map(|key| membership_id(self.owners[0], &self.relation, key))
                    .collect::<Vec<_>>();
                bindings.insert("ids", ids);
                sql = "SELECT id,contribution,relation,semantic_key,node,content FROM $ids";
                self.offset = end;
            } else {
                bindings.insert("relation", self.relation.clone());
                bindings.insert("key", hex::encode(self.keys[self.offset]));
                sql = MEMBERSHIP_KEY_QUERY;
                self.offset += 1;
            }
            let stream = self
                .store
                .read_client()?
                .query(sql)
                .bind(bindings)
                .stream_items()
                .map_err(ModelError::codec)?;
            let mut current = self
                .store
                .track_rows(NativeRows::new(stream, 1)?)?
                .with_setup(self.setup.clone());
            current.installed = self.installed.clone();
            self.current = Some(current);
        }
    }
}
/// Scalar equality keeps member_keys on one prefix; no native union/distinct/order state.
const MEMBERSHIP_KEY_QUERY: &str = "SELECT id,contribution,relation,semantic_key,node,content FROM compiler_membership WITH INDEX member_keys WHERE relation=$relation AND semantic_key=$key";
fn decode_membership(
    row: Value,
    relation: &str,
    owners: &[ContentHash],
    allow_foreign: bool,
) -> Result<Option<Candidate>, ModelError> {
    let object = value_object(&row).ok_or(ModelError::Schema("selected membership pointer"))?;
    let Some(Value::RecordId(owner)) = object.get("contribution") else {
        return Err(ModelError::Schema("selected membership owner"));
    };
    if owner.table.as_str() != "compiler_contribution" {
        return Err(ModelError::Conflict("selected membership owner family"));
    }
    let RecordIdKey::String(owner_key) = &owner.key else {
        return Err(ModelError::Schema("selected membership owner key"));
    };
    if owner_key.len() != 64
        || !owner_key
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ModelError::Schema("selected membership canonical owner"));
    }
    let mut owner_bytes = [0; 32];
    hex::decode_to_slice(owner_key, &mut owner_bytes).map_err(ModelError::codec)?;
    let owner = ContentHash(owner_bytes);
    if owners.binary_search(&owner).is_err() {
        if allow_foreign {
            return Ok(None);
        }
        return Err(ModelError::Conflict("foreign selected membership owner"));
    }
    let mut candidate = decode_candidate(row.clone(), relation)?;
    if object.get("id")
        != Some(&Value::RecordId(membership_id(
            owner,
            relation,
            &candidate.key,
        )))
    {
        return Err(ModelError::Conflict(
            "compiler membership physical identity",
        ));
    }
    let Some(Value::String(content)) = object.get("content") else {
        return Err(ModelError::Schema("selected membership content"));
    };
    candidate.content = Some(ContentHash(
        hex::decode(content)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("content token digest width"))?,
    ));
    Ok(Some(candidate))
}
enum SelectionKeys {
    Known(std::vec::IntoIter<[u8; 16]>),
    Unverified(AsyncOrderedCandidates),
    Verified(AsyncOrderedCandidates),
}
struct SelectedRows {
    keys: SelectionKeys,
    installed: Arc<lctx_native::installed_view::HeldInstalledView>,
    direct_members: Option<lctx_native::installed_view::SelectedMembers>,
    direct_position: usize,
    budget: ResourceBudget,
    owners: Vec<ContentHash>,
    relation: String,
    fields: String,
    predicate: String,
    bindings: Variables,
    preparation: Vec<String>,
    store: Arc<NativeCompilerStore>,
    lookup: Option<MembershipLookup>,
    payload: Option<CompilerRows>,
    pending_keys: Vec<[u8; 16]>,
    pointers: BTreeMap<String, RecordId>,
    setup: Option<ReadSetupCharge>,
    _transfer: Box<dyn Reservation>,
}
fn compact_key_field(
    fields: &str,
    predicate: &str,
    preparation: &[String],
) -> Option<&'static str> {
    if predicate != "true" || !preparation.is_empty() {
        return None;
    }
    match fields {
        "semantic_key AS id" => Some("id"),
        "semantic_key AS __row" => Some("__row"),
        _ => None,
    }
}
impl SelectedRows {
    fn start_payload(&mut self, nodes: Vec<RecordId>) -> Result<(), ModelError> {
        let mut bindings = self.bindings.clone();
        bindings.insert("__compiler_nodes", nodes);
        let prelude = self
            .preparation
            .iter()
            .map(|statement| format!("{statement};"))
            .collect::<String>();
        let statements = self.preparation.len() + 1;
        let sql = format!(
            "{prelude}SELECT {} FROM $__compiler_nodes WHERE semantic_type=$relation AND ({}) ORDER BY semantic_key",
            self.fields, self.predicate
        );
        let mut payload = self
            .store
            .track_rows(
                prepared_rows(
                    &self.store.client,
                    sql,
                    bindings,
                    statements,
                    self.store.read_cancellation()?.as_ref(),
                )?
                .with_row_bytes(64 << 20),
            )?
            .with_setup(self.setup.clone());
        payload.installed = Some(self.installed.clone());
        self.payload = Some(payload);
        Ok(())
    }
    async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        loop {
            let window_rows = crate::loader::NATIVE_WINDOW_ROWS;
            if let Some(members) = &self.direct_members {
                if let Some(member) = members.rows().get(self.direct_position) {
                    self.direct_position += 1;
                    let mut row = Object::new();
                    row.insert(
                        compact_key_field(&self.fields, &self.predicate, &self.preparation)
                            .ok_or(ModelError::Conflict("compact member projection"))?,
                        hex::encode(member.key),
                    );
                    return Ok(Some(Value::Object(row)));
                }
                self.direct_members.take();
                self.direct_position = 0;
            }
            if let Some(rows) = &mut self.payload {
                if let Some(row) = rows.next_native().await? {
                    return Ok(Some(row));
                }
                self.payload.take();
            }
            if let Some(lookup) = &mut self.lookup {
                let nodes = lookup.collect().await?;
                self.lookup.take();
                for (key, node) in &nodes {
                    if let Some(expected) = self.pointers.get(key)
                        && expected != &node.node
                    {
                        return Err(ModelError::Conflict("selected candidate backing collision"));
                    }
                }
                self.pointers.clear();
                if nodes.is_empty() {
                    continue;
                }
                self.start_payload(
                    nodes
                        .into_values()
                        .map(|candidate| candidate.node)
                        .collect(),
                )?;
                continue;
            }
            match &mut self.keys {
                SelectionKeys::Known(source) => {
                    self.pending_keys.extend(source.by_ref().take(window_rows))
                }
                SelectionKeys::Unverified(source) | SelectionKeys::Verified(source) => {
                    for candidate in source.next_batch(window_rows).await? {
                        if candidate.relation != self.relation {
                            return Err(ModelError::Conflict("ordered candidate relation"));
                        }
                        self.pointers
                            .insert(hex::encode(candidate.key), candidate.node);
                        self.pending_keys.push(candidate.key);
                    }
                }
            }
            if self.pending_keys.is_empty() {
                return Ok(None);
            }
            if self.pending_keys.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ModelError::Conflict("native selected key order"));
            }
            let keys = std::mem::replace(
                &mut self.pending_keys,
                Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS),
            );
            if matches!(&self.keys, SelectionKeys::Known(_)) {
                let reader = self.store.installed_member_reader(&self.budget)?;
                let members = self.installed.members(&reader, &keys, &self.budget).await?;
                if compact_key_field(&self.fields, &self.predicate, &self.preparation).is_some() {
                    self.direct_members = Some(members);
                    continue;
                }
                let nodes = members
                    .rows()
                    .iter()
                    .map(|member| member.node.clone())
                    .collect();
                if !members.rows().is_empty() {
                    self.start_payload(nodes)?;
                }
                continue;
            }
            if matches!(&self.keys, SelectionKeys::Verified(_)) {
                let nodes = std::mem::take(&mut self.pointers)
                    .into_values()
                    .collect::<Vec<_>>();
                self.start_payload(nodes)?;
                continue;
            }
            let mut lookup = MembershipLookup::new(
                self.store.clone(),
                self.relation.clone(),
                keys,
                self.owners.clone(),
            );
            lookup.setup = self.setup.clone();
            lookup.installed = Some(self.installed.clone());
            self.lookup = Some(lookup);
        }
    }
}
struct GraphRows {
    ordered: AsyncOrderedRows,
    payload: Option<CompilerRows>,
    store: Arc<NativeCompilerStore>,
    fields: String,
    expected: std::collections::VecDeque<RecordId>,
    setup: Option<ReadSetupCharge>,
    _transfer: Box<dyn Reservation>,
}
impl GraphRows {
    async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        loop {
            if let Some(payload) = &mut self.payload {
                if let Some(row) = payload.next_native().await? {
                    let expected = self
                        .expected
                        .pop_front()
                        .ok_or(ModelError::Conflict("unexpected canonical graph row"))?;
                    if value_object(&row).and_then(|row| row.get("id"))
                        != Some(&Value::RecordId(expected))
                    {
                        return Err(ModelError::Conflict("canonical graph pointer order"));
                    }
                    let mut row = row;
                    if let Value::Object(object) = &mut row {
                        object.remove("__graph_order");
                    }
                    return Ok(Some(row));
                }
                if !self.expected.is_empty() {
                    return Err(ModelError::Conflict("missing canonical graph backing"));
                }
                self.payload.take();
            }
            let pointers = self
                .ordered
                .next_batch(crate::loader::NATIVE_WINDOW_ROWS)
                .await?;
            if pointers.is_empty() {
                return Ok(None);
            }
            for pointer in pointers {
                let Some(Value::RecordId(node)) = value_object(&pointer)
                    .and_then(|row| row.get("payload").or_else(|| row.get("id")))
                else {
                    return Err(ModelError::Schema("canonical ordered pointer"));
                };
                self.expected.push_back(node.clone());
            }
            let mut bindings = Variables::new();
            bindings.insert("nodes", self.expected.iter().cloned().collect::<Vec<_>>());
            self.payload = Some(
                self.store
                    .track_rows(
                        NativeRows::new(
                            self.store.read_client()?.query(format!("SELECT {},anchor.nominal AS __graph_order FROM $nodes ORDER BY __graph_order", self.fields))
                                .bind(bindings)
                                .stream_items()
                                .map_err(ModelError::codec)?,
                            1,
                        )?
                        .with_row_bytes(64 << 20),
                    )?
                    .with_setup(self.setup.clone()),
            );
        }
    }
}

fn decode_candidate(row: Value, relation: &str) -> Result<Candidate, ModelError> {
    let object = value_object(&row).ok_or(ModelError::Schema("native compact candidate"))?;
    let Some(Value::String(actual)) = object.get("relation") else {
        return Err(ModelError::Schema("candidate relation"));
    };
    if actual != relation {
        return Err(ModelError::Conflict("native candidate relation"));
    }
    let Some(Value::String(key)) = object.get("semantic_key") else {
        return Err(ModelError::Schema("candidate nominal key"));
    };
    let Some(Value::RecordId(node)) = object.get("node") else {
        return Err(ModelError::Schema("candidate backing pointer"));
    };
    if node.table.as_str() != crate::schema::ScopeTable::for_relation(relation)?.name() {
        return Err(ModelError::Conflict("candidate backing family"));
    }
    Ok(Candidate {
        content: None,
        relation: actual.clone(),
        key: decode_nominal_key(key)?,
        node: node.clone(),
    })
}

#[derive(Serialize, Deserialize)]
struct MembershipContent {
    relation: String,
    semantic_key: String,
    content: String,
}
fn sink_key(row: &Object) -> Result<String, ModelError> {
    Ok(row
        .get("id")
        .ok_or(ModelError::Schema("membership id"))?
        .to_sql())
}

fn value_object(value: &Value) -> Option<&Object> {
    if let Value::Object(object) = value {
        Some(object)
    } else {
        None
    }
}
fn decode_descriptor<T: serde::de::DeserializeOwned>(row: &Value) -> Result<T, ModelError> {
    let Some(Value::Bytes(bytes)) = value_object(row).and_then(|object| object.get("descriptor"))
    else {
        return Err(ModelError::Schema("native descriptor bytes"));
    };
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}

/// Check closed per-row framing. Bounded ordinary type slices independently validate model keys.

fn hydrate_original_row(value: Value) -> Result<Value, ModelError> {
    let Value::Object(mut row) = value else {
        return Ok(value);
    };
    let Some(Value::Array(chunks)) = row.remove("__original_chunks") else {
        return Ok(Value::Object(row));
    };
    let start = match row.remove("__original_start") {
        Some(Value::Number(surrealdb::types::Number::Int(n))) => {
            u64::try_from(n).map_err(ModelError::codec)?
        }
        _ => return Err(ModelError::Schema("original typed chunk start")),
    };
    let len = match row.remove("__original_len") {
        Some(Value::Number(surrealdb::types::Number::Int(n))) => {
            usize::try_from(n).map_err(ModelError::codec)?
        }
        _ => return Err(ModelError::Schema("original typed chunk length")),
    };
    let Some(Value::String(digest)) = row.remove("__original_digest") else {
        return Err(ModelError::Schema("original typed chunk digest"));
    };
    if len > d::artifact::ARTIFACT_CHUNK_BYTES {
        return Err(ModelError::Schema("original typed chunk byte bound"));
    }
    let layout = lctx_native::originals::decode_layout(
        row.remove("__original_layout")
            .ok_or(ModelError::Schema("original projected layout"))?,
    )?;
    let (blob, completed) = lctx_native::originals::decode_blob(
        row.remove("__original_blob")
            .ok_or(ModelError::Schema("original projected blob"))?,
    )?;
    if blob != layout.blob
        || !completed
        || row.remove("__original_source_id") != Some(Value::RecordId(layout.id.clone()))
    {
        return Err(ModelError::Conflict("original projected layout authority"));
    }
    let artifact = row
        .remove("__original_artifact")
        .ok_or(ModelError::Schema("original projected artifact"))?;
    let artifact =
        value_object(&artifact).ok_or(ModelError::Schema("original projected artifact object"))?;
    let Some(Value::Bytes(canonical)) = artifact.get("canonical") else {
        return Err(ModelError::Schema("original projected artifact canonical"));
    };
    let entity: d::graph::Entity = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
    entity.validate()?;
    let d::graph::Entity::Source(source) = entity else {
        return Err(ModelError::Schema("original projected artifact kind"));
    };
    layout.validate_source(&source)?;
    let mut bytes = Vec::with_capacity(len);
    let mut position = start;
    for chunk in chunks {
        let piece = lctx_native::originals::decode_piece(chunk, &blob)?;
        if piece.start != position || piece.bytes.len() > len.saturating_sub(bytes.len()) {
            return Err(ModelError::Conflict("original physical chunk continuity"));
        }
        bytes.extend_from_slice(&piece.bytes);
        position += piece.bytes.len() as u64;
    }
    if bytes.len() != len || ContentHash::of(&bytes).hex() != digest {
        return Err(ModelError::Conflict("original typed chunk integrity"));
    }
    row.insert("body", Bytes::from(bytes));
    Ok(Value::Object(row))
}

/// One bounded cold-state window, grouped through existing model callbacks. Hot writes and
/// completed-view reads carry their validated typed ownership and never take this audit route.

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[tokio::test(flavor = "multi_thread")]
    async fn last_use_finalization_invalidates_original_session_after_closed_attempt() {
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("owned schema8 validation runtime"),
        )).unwrap();
        let store = NativeCompilerStore::begin(&config, Frontier::Facts).await.unwrap();
        let observer = reader::connect(
            &config.endpoint, &config.writer_credentials(),
            store.namespace().as_str(), store.database().as_str(),
        ).await.unwrap();
        let result = async {
            store.client.query("INFO FOR DB").await.map_err(reader::sdk_error)?
                .check().map_err(reader::sdk_error)?;
            // A separate publisher session can close the attempt while the original
            // compiler session is still authenticated. Drain alone does not close it.
            crate::control::close_attempt(&observer, store.attempt(), "closed").await?;
            d::completion::complete(Ok(()), store.drain_report().await)?;
            store.client.query("INFO FOR DB").await.map_err(reader::sdk_error)?
                .check().map_err(reader::sdk_error)?;
            assert!(!store.admission.source_finalization.lock().await.session_closed);
            store.abandon().await?;
            let source = store.admission.source_finalization.lock().await;
            assert!(source.attempt_closed && source.session_closed);
            drop(source);
            let response = store.client.query("INFO FOR DB").await;
            assert!(response.and_then(|response| response.check()).is_err(),
                "original compiler session must no longer authenticate database inspection");
            let mut response = observer.query("SELECT VALUE state FROM $attempt")
                .bind(("attempt", RecordId::new("native_attempt", store.attempt().hex())))
                .await.map_err(reader::sdk_error)?.check().map_err(reader::sdk_error)?;
            let states: Vec<String> = response.take(0).map_err(ModelError::codec)?;
            assert_eq!(states, ["closed"], "last-use finality preserves prior terminal state");
            store.abandon().await?;
            Ok::<_, ModelError>(())
        }.await;
        let mut terminal = d::completion::Completion::default();
        terminal.step("last-use control original store finality", store.abandon().await);
        terminal.step("last-use control observer finality", observer.invalidate().await.map_err(ModelError::codec));
        d::completion::complete(result, terminal).unwrap();
    }
    fn local_store() -> Arc<NativeCompilerStore> {
        NativeCompilerStore::from_existing(
            Arc::new(Surreal::init()),
            d::serving::Name::new("local_control").unwrap(),
            d::serving::Name::new("owned_control").unwrap(),
        )
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn retained_same_root_compares_alias_eligibility_and_keeps_receiver_copy() {
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG")
                .expect("owned schema8 validation runtime"),
        ))
        .unwrap();
        let receiver = NativeCompilerStore::begin(&config, Frontier::Normalized)
            .await
            .unwrap();
        let equal = NativeCompilerStore::begin(&config, Frontier::Normalized)
            .await
            .unwrap();
        let unequal = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let result=async {
            let relation=Relation::of::<d::value::Place>();let nonce=receiver.attempt();
            let place=d::value::Place{root:serde_json::from_value(serde_json::to_value(<[u8;16]>::try_from(&nonce.0[..16]).map_err(ModelError::codec)?).map_err(ModelError::codec)?).map_err(ModelError::codec)?,path:serde_json::from_value(serde_json::to_value(<[u8;16]>::try_from(&nonce.0[16..]).map_err(ModelError::codec)?).map_err(ModelError::codec)?).map_err(ModelError::codec)?};
            let spec=ContributionSpec{captured_binding:None,producer:format!("retained-eligibility-{}",nonce.hex()),profile:OutputProfile::Specific(d::stages::Profile::Catalog),model:d::model()?.digest(),implementation:nonce,configuration:None,inputs:vec![],outputs:[relation.name().into()].into()};
            let mut selected=vec![];
            for store in [&receiver,&equal,&unequal] {
                let contribution=store.begin_contribution(spec.clone()).await?;
                store.write_batch(&contribution,&relation,&d::value::Place::encode(std::slice::from_ref(&place))?,&budget).await?;
                let views=store.complete_contribution(contribution,ProviderOutcome::Complete,std::slice::from_ref(&relation),&BTreeMap::new(),&budget).await?;
                selected.push(store.installed_view(&views[relation.name()],&budget).await?);
            }
            assert_eq!(selected[0].completion().view,selected[1].completion().view);assert_eq!(selected[0].completion().view,selected[2].completion().view);
            assert_ne!(selected[0].completion().mappings,selected[1].completion().mappings);
            let _root_gate=receiver.root_gates([selected[0].completion().view.identity],&budget).await?;
            let existing=receiver.receiver_root_under_gate(selected[1].clone(),&budget).await?;
            assert!(Arc::ptr_eq(&existing,&selected[0]),"equivalent roots retain the receiver's protected physical copies");
            let incoming_physical=selected[1].completion().mappings.iter().find(|mapping|mapping.view==existing.completion().view.identity).ok_or(ModelError::Schema("retained incoming physical"))?.physical;
            let reader=crate::NativeReader::private(receiver.client.clone()).with_budget(&budget);
            let incoming=lctx_native::installed_view::inspect_contributor(&reader,incoming_physical,&budget).await;
            let mut terminal=d::completion::Completion::default();terminal.step("retained incoming record reader close",reader.close().await);let incoming=d::completion::complete(incoming,terminal)?.ok_or(ModelError::Conflict("retained incoming record missing"))?;
            let attached=lctx_native::installed_view::attach_live_views_retaining(receiver.client.clone(),receiver.attempt,std::slice::from_ref(&existing),vec![(incoming,selected[1].clone())],&budget).await?;
            assert_eq!(attached[0].completion().mappings,selected[0].completion().mappings,"actual attachment keeps receiver selectors while retaining original candidate bookkeeping");
            let before=receiver.view_mappings.lock().map_err(|_|ModelError::Conflict("retained test mapping"))?.get(&existing.completion().view.identity).map(|row|row.0.clone()).unwrap();
            let refused=receiver.receiver_root_under_gate(selected[2].clone(),&budget).await;
            assert!(matches!(refused,Err(ModelError::Conflict("receiver retained root eligibility collision"))),"same descriptor with absent local aliases must refuse substitution");
            let after=receiver.view_mappings.lock().map_err(|_|ModelError::Conflict("retained test mapping"))?.get(&existing.completion().view.identity).map(|row|row.0.clone()).unwrap();assert_eq!(before,after);
            assert!(Arc::ptr_eq(receiver.installed_views.lock().map_err(|_|ModelError::Conflict("retained test protected root"))?.get(&existing.completion().view.identity).unwrap(),&selected[0]));
            Ok::<_,ModelError>(())
        }.await;
        let mut terminal = d::completion::Completion::default();
        terminal.step("retained unequal source close", unequal.abandon().await);
        terminal.step("retained equal source close", equal.abandon().await);
        terminal.step("retained receiver close", receiver.abandon().await);
        d::completion::complete(result, terminal).unwrap();
    }
    #[tokio::test]
    async fn receiver_view_mapping_metadata_preserves_independent_equivalent_dependency_choices() {
        use lctx_native::installed_view::ContributorRealization;
        let store = local_store();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let base = ContentHash::of(b"equivalent dependency");
        let derived = ContentHash::of(b"derived output");
        let dependency = CompletedView::new("packages".into(), [base].into(), 0).unwrap();
        let root = CompletedView::new("releases".into(), [derived].into(), 0).unwrap();
        let chosen = ContributorRealization {
            view: dependency.identity,
            logical: base,
            physical: ContentHash::of(b"receiver dependency copy"),
            incarnation: 1,
        };
        store
            .remember_view_mapping(&dependency, std::slice::from_ref(&chosen), &budget)
            .unwrap();
        let alternate = ContributorRealization {
            physical: ContentHash::of(b"source dependency copy"),
            ..chosen.clone()
        };
        let root_copy = ContributorRealization {
            view: root.identity,
            logical: derived,
            physical: ContentHash::of(b"source output copy"),
            incarnation: 1,
        };
        let mut source_closure = vec![alternate, root_copy.clone()];
        source_closure.sort_by_key(|mapping| (mapping.view, mapping.logical));
        store
            .remember_view_mapping(&root, &source_closure, &budget)
            .unwrap();
        let known = store.view_mappings.lock().unwrap();
        assert_eq!(
            known[&dependency.identity].0,
            vec![chosen],
            "the receiver preserves its own exact equivalent dependency selection"
        );
        assert_eq!(
            known[&root.identity].0, source_closure,
            "root retains its own full closure without replacing the dependency root selection"
        );
        assert!(
            budget.reserved() > 0,
            "mapping allocations retain their budget owner"
        );
        drop(known);
        store.view_mappings.lock().unwrap().clear();
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn detached_transport_preserves_distinct_completions_of_one_specification() {
        let spec = ContributionSpec {
            captured_binding: None,
            producer: "transport-owner".into(),
            profile: OutputProfile::Specific(d::stages::Profile::Catalog),
            model: ContentHash::of(b"transport-model"),
            implementation: ContentHash::of(b"implementation"),
            configuration: None,
            inputs: vec![],
            outputs: ["packages".into()].into(),
        };
        let first = CompletedContribution {
            assurance: d::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            spec,
            outcome: 0,
            outputs: [(
                "packages".into(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"complete empty"),
                },
            )]
            .into(),
        };
        let mut second = first.clone();
        second.outcome = 2;
        require_retained_request_spec(&first.spec, &first.spec).unwrap();
        let mut changed = first.spec.clone();
        changed.implementation = ContentHash::of(b"different implementation");
        assert!(
            matches!(
                require_retained_request_spec(&first.spec, &changed),
                Err(ModelError::Conflict(
                    "equal request with inconsistent contribution specification"
                ))
            ),
            "an equal request cannot quietly miss after its committed specification changes"
        );
        let attempt = ContentHash::of(b"producing attempt");
        let physical = RecordId::new(
            "compiler_contribution",
            attempt_contribution(attempt, first.spec.identity().unwrap()).hex(),
        );
        let encode = |descriptor: &CompletedContribution| {
            let mut row = Object::new();
            row.insert("id", physical.clone());
            row.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
            row.insert("completed", true);
            row.insert("logical", descriptor.identity().unwrap().hex());
            row.insert(
                "descriptor",
                Bytes::from(serde_json::to_vec(descriptor).unwrap()),
            );
            row.insert(
                "spec",
                Bytes::from(serde_json::to_vec(&descriptor.spec).unwrap()),
            );
            row.insert("outcome", Value::from_t(descriptor.outcome));
            for (key, value) in contribution_projection(&descriptor.spec) {
                row.insert(key, value);
            }
            Value::Object(row)
        };
        let one = lctx_native::completed_logical_rows::normalize(
            "compiler_contribution",
            &encode(&first),
            &BTreeMap::new(),
        )
        .unwrap();
        let two = lctx_native::completed_logical_rows::normalize(
            "compiler_contribution",
            &encode(&second),
            &BTreeMap::new(),
        )
        .unwrap();
        lctx_native::completed_transport::validate_logical_row("compiler_contribution", &one)
            .unwrap();
        lctx_native::completed_transport::validate_logical_row("compiler_contribution", &two)
            .unwrap();
        assert_ne!(
            value_object(&one).unwrap().get("id"),
            value_object(&two).unwrap().get("id")
        );
        let mut bad = value_object(&one).unwrap().clone();
        bad.insert("id", RecordId::new("compiler_contribution", "unrelated"));
        assert!(
            lctx_native::completed_transport::validate_logical_row(
                "compiler_contribution",
                &Value::Object(bad)
            )
            .is_err()
        );
        assert_ne!(
            lctx_native::completed_logical_rows::normalize(
                "compiler_contribution",
                &one,
                &BTreeMap::new()
            )
            .unwrap(),
            lctx_native::completed_logical_rows::normalize(
                "compiler_contribution",
                &two,
                &BTreeMap::new()
            )
            .unwrap()
        );
    }
    #[tokio::test]
    async fn import_view_missing_exact_contributor_refuses_before_native_installation() {
        // This store has no connected native transport: a missing selected input must
        // refuse locally, rather than installing an empty or globally matched view.
        let store = local_store();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let view = CompletedView::new(
            "packages".into(),
            [ContentHash::of(b"unimported contributor")].into(),
            0,
        )
        .unwrap();
        let error = store.view_owners(&view).unwrap_err();
        assert!(matches!(
            error,
            ModelError::Conflict("missing explicit selected contributor")
        ));
    }
    #[tokio::test]
    async fn registering_contribution_cannot_complete_or_write_before_checked_attachment() {
        // An unconnected store makes any accidental native work fail differently. The
        // local registering owner must refuse before querying even previous views.
        let store = local_store();
        let relation = Relation::of::<d::analytics::QualityStep>();
        let spec = ContributionSpec {
            captured_binding: None,
            producer: "registering-owner".into(),
            profile: lctx_model::domain::completed::OutputProfile::Specific(
                d::stages::Profile::Catalog,
            ),
            model: ContentHash::of(b"registering-model"),
            implementation: ContentHash::of(b"registering-code"),
            configuration: None,
            inputs: vec![],
            outputs: [relation.name().to_string()].into(),
        };
        let id = attempt_contribution(store.attempt, spec.identity().unwrap());
        store.specifications.lock().unwrap().insert(
            id,
            RegisteredContribution {
                spec,
                ownership: ContributionOwnership::Registering,
                _transport_charge: None,
            },
        );
        let previous = CompletedView::new(
            relation.name().into(),
            [ContentHash::of(b"unregistered-prior")].into(),
            0,
        )
        .unwrap();
        let budget = ResourceBudget::fixed(1024).unwrap();
        let result = store
            .complete_contribution_inner(
                id,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &[(relation.name().to_string(), previous)].into(),
                &budget,
            )
            .await;
        assert!(matches!(
            result,
            Err(ModelError::Conflict(
                "native contribution ownership not ready"
            ))
        ));
        let batch = d::analytics::QualityStep::encode(&[]).unwrap();
        let result = store
            .write_batch_inner(&id, &relation, &batch, None, None)
            .await;
        assert!(matches!(
            result,
            Err(ModelError::Conflict(
                "native contribution ownership not ready"
            ))
        ));
        assert_eq!(budget.reserved(), 0);
        assert!(store.drain_report().await.failures.is_empty());
    }

    async fn empty_prepared_rows(
        store: &Arc<NativeCompilerStore>,
        budget: &ResourceBudget,
    ) -> PreparedRows {
        let mut sorter = AsyncPhysicalSort::new_registered(
            budget,
            store.blocking_owner().unwrap(),
            store.blocking_observer().unwrap(),
        )
        .await
        .unwrap();
        sorter.prepare().await.unwrap()
    }
    fn empty_compiler_rows(store: Arc<NativeCompilerStore>) -> CompilerRows {
        CompilerRows {
            descendant: None,
            producing: None,
            rows: None,
            selection: None,
            graph: None,
            store,
            lease: None,
            setup: None,
            installed: None,
            source_hold: None,
        }
    }
    #[tokio::test]
    async fn producing_root_delayed_before_first_native_call_holds_global_drain() {
        let store = local_store();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"delayed-producing-root"), &budget)
            .unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let owner = store.clone();
        let work_scope = scope.clone();
        let caller = tokio::spawn(async move {
            work_scope
                .run(async move {
                    started.send(()).unwrap();
                    gate.await.unwrap();
                    // This is a descendant of the already globally admitted root.
                    owner
                        .admit(false, "delayed native descendant", false)?
                        .finish();
                    let child_scope = owner.producing_scope(
                        ContentHash::of(b"delayed-producing-root"),
                        &ResourceBudget::fixed(4096)?,
                    )?;
                    let mut child = child_scope.bind_stream(futures::stream::iter([1]))?;
                    assert_eq!(futures::StreamExt::next(&mut child).await, Some(1));
                    assert_eq!(futures::StreamExt::next(&mut child).await, None);
                    Ok(())
                })
                .await
        });
        ready.await.unwrap();
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        assert!(
            scope.run(async { Ok(()) }).await.is_err(),
            "closure refuses a new producing root even while the old root is admitted"
        );
        release.send(()).unwrap();
        caller.await.unwrap().unwrap();
        let report = drain.await;
        assert_eq!(report.local, d::completion::LocalState::Terminal);
        assert_eq!(report.remote, d::completion::RemoteState::Confirmed);
        assert!(report.failures.is_empty());
    }
    #[tokio::test]
    async fn cancelled_producing_root_before_native_call_has_no_uncertain_remote_effect() {
        let store = local_store();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"cancelled-local-root"), &budget)
            .unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let caller = tokio::spawn(async move {
            scope
                .run(async move {
                    started.send(()).unwrap();
                    futures::future::pending::<Result<(), ModelError>>().await
                })
                .await
        });
        ready.await.unwrap();
        caller.abort();
        assert!(caller.await.err().unwrap().is_cancelled());
        let report = store.drain_report().await;
        assert_eq!(report.local, d::completion::LocalState::Terminal);
        assert_eq!(report.remote, d::completion::RemoteState::Confirmed);
        assert!(report.failures.is_empty());
    }
    #[tokio::test]
    async fn caught_producing_root_error_prevents_scope_completion() {
        let store = local_store();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"caught-producing-error"), &budget)
            .unwrap();
        assert!(
            scope
                .run(async { Err::<(), _>(ModelError::Conflict("own producing failure")) })
                .await
                .is_err()
        );
        assert!(scope.close_and_wait().await.is_err());
        let report = store.drain_report().await;
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.step == "native producing work")
        );
        assert_eq!(report.remote, d::completion::RemoteState::Confirmed);
    }
    #[tokio::test]
    async fn lazy_producing_provider_execution_after_global_close_is_refused() {
        use datafusion::execution::context::SessionContext;
        use futures::TryStreamExt;
        let store = crate::test_native::store();
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"lazy-producing-provider"), &budget)
            .unwrap();
        let relation = Relation::of::<d::input::Package>();
        let id = store
            .begin_contribution(ContributionSpec {
                captured_binding: None,
                producer: "lazy-provider".into(),
                profile: d::completed::OutputProfile::Specific(d::stages::Profile::Catalog),
                model: ContentHash::of(b"lazy-model"),
                implementation: ContentHash::of(b"lazy-implementation"),
                configuration: None,
                inputs: vec![],
                outputs: [relation.name().into()].into(),
            })
            .await
            .unwrap();
        let views = store
            .complete_contribution(
                id,
                ProviderOutcome::Complete,
                std::slice::from_ref(&relation),
                &BTreeMap::new(),
                &budget,
            )
            .await
            .unwrap();
        let view = views[relation.name()].clone();
        let provider = store
            .table_provider(&view, relation, budget, 16)
            .await
            .unwrap();
        let provider = crate::compiler_provider::bind_producing(&provider, &scope).unwrap();
        let session = SessionContext::new();
        let plan = provider
            .scan(&session.state(), None, &[], None)
            .await
            .unwrap();
        store.drain().await.unwrap();
        let mut stream = plan.execute(0, session.task_ctx()).unwrap();
        let error = stream.try_next().await.unwrap_err();
        assert!(format!("{error:?}").contains("native compiler authority closed or failed"));
        assert_eq!(store.admission.state.lock().unwrap().active, 0);
        assert_eq!(scope.scope.state.lock().unwrap().active, 0);
        store.abandon().await.unwrap();
    }
    #[test]
    fn membership_owner_lookup_decodes_canonical_hash_and_searches_sorted_physical_owners() {
        let mut owners = (0..64)
            .map(|index| ContentHash::of(&[index]))
            .collect::<Vec<_>>();
        owners.sort_unstable();
        let owner = owners[37];
        let key = [7; 16];
        let relation = d::input::Package::NAME;
        let mut row = Object::new();
        row.insert("id", membership_id(owner, relation, &key));
        row.insert(
            "contribution",
            RecordId::new("compiler_contribution", owner.hex()),
        );
        row.insert("relation", relation.to_owned());
        row.insert("semantic_key", hex::encode(key));
        row.insert("node", RecordId::new("entity", "owner-lookup-node"));
        row.insert("content", ContentHash::of(b"membership-content").hex());
        assert_eq!(
            decode_membership(Value::Object(row.clone()), relation, &owners, false)
                .unwrap()
                .unwrap()
                .key,
            key
        );
        row.insert(
            "contribution",
            RecordId::new(
                "compiler_contribution",
                ContentHash::of(b"foreign-owner").hex(),
            ),
        );
        assert!(
            decode_membership(Value::Object(row.clone()), relation, &owners, true)
                .unwrap()
                .is_none()
        );
        assert!(decode_membership(Value::Object(row.clone()), relation, &owners, false).is_err());
        row.insert(
            "contribution",
            RecordId::new("compiler_contribution", owner.hex().to_uppercase()),
        );
        assert!(matches!(
            decode_membership(Value::Object(row), relation, &owners, true),
            Err(ModelError::Schema("selected membership canonical owner"))
        ));
    }
    #[tokio::test]
    async fn producing_scope_waits_only_for_owned_read_and_final_drain_is_global() {
        let store = local_store();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"own-producing-work"), &budget)
            .unwrap();
        let unrelated = store.retained_scan().unwrap();
        let own = scope.run(async { store.retained_scan() }).await.unwrap();
        let mut completion = Box::pin(scope.close_and_wait());
        assert!(futures::poll!(&mut completion).is_pending());
        own.finish();
        completion.await.unwrap();
        assert!(
            scope.run(async { Ok(()) }).await.is_err(),
            "closed scope cannot admit a new root"
        );
        let mut final_drain = Box::pin(store.drain());
        assert!(
            futures::poll!(&mut final_drain).is_pending(),
            "unrelated native work still belongs to final closure"
        );
        unrelated.finish();
        final_drain.await.unwrap();
    }
    #[tokio::test]
    async fn cancelled_producing_setup_retains_scope_until_actual_terminal() {
        let store = local_store();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"cancelled-producing-setup"), &budget)
            .unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let owner = store.clone();
        let work_scope = scope.clone();
        let caller = tokio::spawn(async move {
            work_scope
                .run(owner.retain_read_setup(
                    "producing gated setup",
                    Box::pin(async move {
                        started.send(()).unwrap();
                        gate.await.unwrap();
                        Err(ModelError::Conflict("late producing setup error"))
                    }),
                ))
                .await
        });
        ready.await.unwrap();
        caller.abort();
        assert!(caller.await.err().unwrap().is_cancelled());
        let mut completion = Box::pin(scope.close_and_wait());
        assert!(
            futures::poll!(&mut completion).is_pending(),
            "cancelled waiter is not terminal producing work"
        );
        release.send(()).unwrap();
        assert!(completion.await.is_err());
        let report = store.drain_report().await;
        assert_eq!(report.local, d::completion::LocalState::Terminal);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.step == "producing gated setup")
        );
    }
    #[tokio::test]
    async fn unrelated_late_read_error_still_poison_final_admission() {
        let store = local_store();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"independent-producing-work"), &budget)
            .unwrap();
        let unrelated = store.retained_scan().unwrap();
        scope.close_and_wait().await.unwrap();
        assert!(
            unrelated
                .finish_with::<()>(Err(ModelError::Conflict("unrelated late read")))
                .is_err()
        );
        assert!(store.drain().await.is_err());
        assert!(store.check_failed().is_err());
    }
    #[tokio::test]
    async fn content_lane_drains_mutators_without_waiting_for_readers() {
        let store = local_store();
        let mutation = store.admit_mutation("held mutation", true).unwrap();
        let read = store.admit(false, "held read", false).unwrap();
        let mut freeze = Box::pin(store.close_content_lane());
        assert!(futures::poll!(&mut freeze).is_pending());
        assert!(store.admit_mutation("late mutation", false).is_err());
        store
            .admit(false, "new frozen read", false)
            .unwrap()
            .finish();
        mutation.finish();
        freeze.await.unwrap();
        assert_eq!(store.admission.state.lock().unwrap().active, 1);
        read.finish();
        store.drain().await.unwrap();
    }
    #[tokio::test]
    async fn interrupted_seal_drain_can_resume_before_the_private_lease_is_issued() {
        let store = local_store();
        store.close_content_lane().await.unwrap();
        store.admission.state.lock().unwrap().content_ready = true;
        let read = store.admit(false, "held read", false).unwrap();
        let mut sealing = Box::pin(store.begin_finalization());
        assert!(futures::poll!(&mut sealing).is_pending());
        drop(sealing);
        assert!(!store.seal_started.load(Ordering::Acquire));
        read.finish();
        let private = store.begin_finalization().await.unwrap();
        assert!(store.seal_started.load(Ordering::Acquire));
        assert!(store.admit(false, "ordinary after seal", false).is_err());
        private.finish();
        assert!(store.begin_finalization().await.is_err());
    }
    #[tokio::test]
    async fn cancelled_read_setup_retains_ownership_until_real_completion() {
        let store = local_store();
        let budget = ResourceBudget::fixed(4096).unwrap();
        let retained = budget.reserve("cancelled-read-setup", 1024).unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let owner = store.clone();
        let rows_owner = store.clone();
        let caller = tokio::spawn(async move {
            owner
                .retain_read_setup(
                    "gated read setup",
                    Box::pin(async move {
                        started.send(()).unwrap();
                        gate.await.unwrap();
                        drop(retained);
                        Ok(CompilerRows {
                            descendant: None,
                            producing: None,
                            rows: None,
                            selection: None,
                            graph: None,
                            store: rows_owner,
                            lease: None,
                            setup: None,
                            installed: None,
                            source_hold: None,
                        })
                    }),
                )
                .await
        });
        ready.await.unwrap();
        caller.abort();
        assert!(
            caller
                .await
                .err()
                .expect("cancelled consumer")
                .is_cancelled()
        );
        assert!(store.check().is_ok());
        assert_eq!(budget.reserved(), 1024);
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        release.send(()).unwrap();
        let completion = drain.await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.is_empty());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn cancelled_read_setup_drains_undelivered_native_rows() {
        use futures::StreamExt;
        let store = local_store();
        let budget = ResourceBudget::fixed(4096).unwrap();
        let retained = Arc::new(budget.reserve("retained-read-binding", 1024).unwrap());
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let (tail_release, tail_gate) = tokio::sync::oneshot::channel();
        let (dropped, did_drop) = tokio::sync::oneshot::channel();
        let owner = store.clone();
        let rows_owner = store.clone();
        let caller = tokio::spawn(async move {
            owner
                .retain_read_setup(
                    "gated native setup",
                    Box::pin(async move {
                        started.send(()).unwrap();
                        gate.await.unwrap();
                        let tail = futures::stream::once(async move {
                            tail_gate.await.unwrap();
                            Ok(surrealdb::method::StreamItem::StatementEnd {
                                statement: 0,
                                stats: Default::default(),
                                result: Ok(()),
                            })
                        });
                        let rows = rows_owner
                            .track_rows(NativeRows::new(tail.boxed(), 1).unwrap())
                            .map(|rows| rows.with_setup(Some(retained)));
                        dropped.send(()).unwrap();
                        rows
                    }),
                )
                .await
        });
        ready.await.unwrap();
        caller.abort();
        assert!(
            caller
                .await
                .err()
                .expect("cancelled consumer")
                .is_cancelled()
        );
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        release.send(()).unwrap();
        did_drop.await.unwrap();
        assert!(futures::poll!(&mut drain).is_pending());
        assert_eq!(budget.reserved(), 1024);
        tail_release.send(()).unwrap();
        let completion = drain.await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.is_empty());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn delivered_scan_retains_large_moved_bindings_until_release() {
        let store = local_store();
        let budget = ResourceBudget::fixed(2 << 20).unwrap();
        let relation = Relation::of::<d::input::InputRevision>();
        let contributor = ContentHash::of(b"binding control contributor");
        let view = CompletedView::new(relation.name().into(), [contributor].into(), 1).unwrap();
        store
            .known_views
            .lock()
            .unwrap()
            .insert(view.identity, view.clone());
        store
            .known_contributors
            .lock()
            .unwrap()
            .insert(contributor, contributor);
        let mut bindings = Variables::new();
        bindings.insert("large_bound_value", "x".repeat(300_000));
        let predicate = NativePredicate::KeysSql {
            keys: vec![[1; 16]],
            sql: "true".into(),
            bindings,
            preparation: vec![],
        };
        let rows = store
            .scan_rows(&view, &relation, None, Some(predicate), &budget)
            .await
            .unwrap();
        assert!(
            budget.reserved() >= 300_000,
            "delivered binding lost its sized owner"
        );
        drop(rows);
        let completion = store.drain_report().await;
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.is_empty());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn cancelled_read_setup_retains_a_late_typed_failure_before_quiescence() {
        let store = local_store();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let owner = store.clone();
        let caller = tokio::spawn(async move {
            owner
                .retain_read_setup(
                    "gated refused setup",
                    Box::pin(async move {
                        started.send(()).unwrap();
                        gate.await.unwrap();
                        Err(ModelError::Schema("late acknowledged read refusal"))
                    }),
                )
                .await
        });
        ready.await.unwrap();
        caller.abort();
        assert!(
            caller
                .await
                .err()
                .expect("cancelled consumer")
                .is_cancelled()
        );
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        release.send(()).unwrap();
        let completion = drain.await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(
            completion
                .failures
                .iter()
                .any(|failure| failure.step == "gated refused setup"
                    && matches!(
                        failure.error.primary(),
                        Some(ModelError::Schema("late acknowledged read refusal"))
                    ))
        );
    }
    #[tokio::test]
    async fn discarded_buffered_read_failure_retains_admission_until_observed() {
        let store = local_store();
        let (ready, complete) = tokio::sync::oneshot::channel();
        let mut pending = Box::pin(store.retain_read_setup(
            "buffered read refusal",
            Box::pin(async move {
                ready.send(()).unwrap();
                Err(ModelError::Schema("buffered acknowledged read refusal"))
            }),
        ));
        assert!(futures::poll!(&mut pending).is_pending());
        complete.await.unwrap();
        assert_eq!(
            store.admission.state.lock().unwrap().active,
            1,
            "buffered response lost its admitted owner"
        );
        drop(pending);
        let completion = store.drain_report().await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(
            completion
                .failures
                .iter()
                .any(|failure| failure.step == "buffered read refusal"
                    && matches!(
                        failure.error.primary(),
                        Some(ModelError::Schema("buffered acknowledged read refusal"))
                    ))
        );
    }
    #[tokio::test]
    async fn failed_write_transport_retains_cause_and_unknown_acknowledgement() {
        let store = local_store();
        let lease = store.admit(false, "control write", true).unwrap();
        let error = crate::loader::write_failure(surrealdb::Error::internal(
            "injected transport failure".into(),
        ));
        assert!(
            matches!(error.primary(),Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some())
        );
        assert!(!error.permits_storage_cleanup());
        let result = Err::<(), _>(error);
        let result = lease.finish_with(result);
        assert!(result.is_err());
        let completion = store.drain_report().await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Unknown);
    }
    #[tokio::test]
    async fn admission_refusal_preserves_the_first_operation_cause() {
        let store = local_store();
        let first = store.admit(false, "begin contribution", true).unwrap();
        let failed = first.finish_with(Err::<(), _>(ModelError::Schema(
            "original contribution refusal",
        )));
        assert!(matches!(
            failed.unwrap_err().primary(),
            Some(ModelError::Schema("original contribution refusal"))
        ));
        let refused = match store.admit(false, "following write", true) {
            Ok(_) => panic!("failed owner admitted write"),
            Err(error) => error,
        };
        assert!(matches!(
            refused.primary(),
            Some(ModelError::Schema("original contribution refusal"))
        ));
        assert!(refused.to_string().contains("State infrastructure failure"));
        assert!(!refused.to_string().contains("equal semantic key"));
        let completion = store.drain_report().await;
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(
            completion
                .failures
                .iter()
                .any(|failure| failure.step == "begin contribution"
                    && matches!(
                        failure.error.primary(),
                        Some(ModelError::Schema("original contribution refusal"))
                    ))
        );
    }
    #[tokio::test]
    async fn definite_read_request_refusal_does_not_poison_the_owner() {
        let store = local_store();
        let lease = store
            .admit(false, "selected read validation", false)
            .unwrap();
        let result = lease.finish_with(Err::<(), _>(ModelError::Schema("foreign selected view")));
        assert!(result.is_err());
        assert!(store.check().is_ok());
        let completion = store.drain_report().await;
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.is_empty());
    }
    #[tokio::test]
    async fn acknowledged_operation_failure_is_returned_separately_from_owner_finality() {
        let store = local_store();
        let lease = store
            .admit(false, "acknowledged control write", true)
            .unwrap();
        let error = lease
            .finish_with(Err::<(), _>(ModelError::Conflict(
                "acknowledged immutable refusal",
            )))
            .unwrap_err();
        let finality = store.drain_finality().await;
        assert_eq!(finality.local, d::completion::LocalState::Terminal);
        assert_eq!(finality.remote, d::completion::RemoteState::Confirmed);
        assert!(
            finality.failures.is_empty(),
            "joined acknowledged refusal is not failed owner cleanup"
        );
        let caller = store.drain_report().await;
        assert_eq!(caller.failures.len(), 1);
        assert_eq!(caller.failures[0].step, "acknowledged control write");
        let (ModelError::SharedCause(actual), ModelError::SharedCause(expected)) =
            (&caller.failures[0].error, error)
        else {
            panic!("exact recorded cause")
        };
        assert!(Arc::ptr_eq(actual, &expected));
    }
    #[tokio::test]
    async fn source_signal_finality_remains_pending_until_acknowledged_retry() {
        for retry_acknowledged in [false, true] {
            let abandoned = Arc::new(AtomicBool::new(false));
            let observed = abandoned.clone();
            let mut store = local_store();
            // Use the actual listener/OnceCell protocol without fixture receipt
            // environment mutation or native effects. Drop cancels only an owner
            // whose completion has not been published.
            Arc::get_mut(&mut store).unwrap().signals =
                Ok(lctx_native::signal::SignalRegistration::process(move || {
                    observed.store(true, Ordering::Release);
                })
                .unwrap());
            let mut source = SourceFinalization {
                attempt_closed: true,
                session_closed: false,
            };
            let mut unknown = d::completion::Completion::default();
            unknown.local = d::completion::LocalState::Outstanding;
            unknown.remote = d::completion::RemoteState::Unknown;
            store.complete_source_signals(&source, &mut unknown).await;
            assert_eq!(unknown.remote, d::completion::RemoteState::Unknown);
            assert_eq!(unknown.local, d::completion::LocalState::Outstanding);
            if retry_acknowledged {
                source.session_closed = true;
                let mut confirmed = d::completion::Completion::default();
                store.complete_source_signals(&source, &mut confirmed).await;
                assert!(confirmed.failures.is_empty());
            }
            drop(store);
            assert_eq!(
                abandoned.load(Ordering::Acquire),
                !retry_acknowledged,
                "an unconfirmed first attempt must not freeze SignalRegistration completion before its acknowledged retry"
            );
        }
    }
    #[tokio::test]
    async fn acknowledged_abandonment_replays_original_failure_without_reopening_session() {
        let store = local_store();
        let lease = store
            .admit(false, "acknowledged control write", true)
            .unwrap();
        let error = lease
            .finish_with(Err::<(), _>(ModelError::Conflict(
                "original acknowledged refusal",
            )))
            .unwrap_err();
        // Model the two already received native acknowledgements. The unconnected
        // SDK below makes any accidental repeated remote work fail this control.
        {
            let mut source = store.admission.source_finalization.lock().await;
            assert!(!source.attempt_closed && !source.session_closed);
            source.attempt_closed = true;
            assert!(
                !source.session_closed,
                "attempt acknowledgement alone cannot cache session finality"
            );
            source.session_closed = true;
        }
        for _ in 0..2 {
            let ModelError::Completion(outcome) = store.abandon().await.unwrap_err() else {
                panic!("original operation failure remains returned")
            };
            assert_eq!(
                outcome.completion.local,
                d::completion::LocalState::Terminal
            );
            assert_eq!(
                outcome.completion.remote,
                d::completion::RemoteState::Confirmed
            );
            assert_eq!(outcome.completion.failures.len(), 1);
            assert_eq!(
                outcome.completion.failures[0].step,
                "acknowledged control write"
            );
            let (ModelError::SharedCause(actual), ModelError::SharedCause(expected)) =
                (&outcome.completion.failures[0].error, &error)
            else {
                panic!("exact recorded cause")
            };
            assert!(Arc::ptr_eq(actual, expected));
        }
        let drained = store.drain_report().await;
        assert_eq!(drained.failures.len(), 1);
        assert_eq!(drained.remote, d::completion::RemoteState::Confirmed);
    }
    #[tokio::test]
    async fn local_terminal_drain_retains_unknown_native_acknowledgement() {
        let store = local_store();
        let lease = store.admit(false, "control write", true).unwrap();
        drop(lease);
        let finality = store.drain_finality().await;
        assert_eq!(finality.remote, d::completion::RemoteState::Unknown);
        assert!(
            finality
                .failures
                .iter()
                .any(|failure| failure.step == "native acknowledgement"),
            "unconfirmed ownership cannot become a successful cleanup receipt"
        );
        let completion = store.drain_report().await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Unknown);
        assert!(
            completion
                .failures
                .iter()
                .any(|failure| failure.step == "control write")
        );
        assert!(store.end_writes().await.is_err());
    }
    #[tokio::test]
    async fn interrupted_native_drain_keeps_admission_for_acknowledged_retry() {
        let store = local_store();
        let lease = store.admit(false, "control write", true).unwrap();
        let owner = store.clone();
        let first = tokio::spawn(async move { owner.drain_report().await });
        tokio::task::yield_now().await;
        assert!(!first.is_finished());
        first.abort();
        assert!(first.await.is_err());
        lease.finish();
        let completion = store.drain_report().await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.is_empty());
    }
    #[tokio::test]
    async fn local_sort_worker_failure_remains_typed_in_compiler_finalization() {
        let store = local_store();
        let budget = ResourceBudget::fixed(1024).unwrap();
        assert!(
            AsyncCandidateSort::new_registered(
                &budget,
                store.blocking_owner().unwrap(),
                store.blocking_observer().unwrap()
            )
            .await
            .is_err()
        );
        let completion = store.drain_report().await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.iter().any(|failure| matches!(
            failure.error.primary(),
            Some(ModelError::Resource {
                owner: "ordered-command-owner",
                ..
            })
        )));
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn publication_read_owner_failed_preparation_joins_delayed_tail_without_write_authority()
    {
        let native_operation_budget =
            lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let contribution = CompletedContribution {
            assurance:
                lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
            spec: ContributionSpec {
                captured_binding: None,
                producer: "audit-read-owner".into(),
                profile: lctx_model::domain::completed::OutputProfile::Specific(
                    d::stages::Profile::Catalog,
                ),
                model: ContentHash::of(b"audit-model"),
                implementation: ContentHash::of(b"audit-code"),
                configuration: None,
                inputs: vec![],
                outputs: [d::analytics::QualityStep::NAME.to_string()].into(),
            },
            outcome: 0,
            outputs: [(
                d::analytics::QualityStep::NAME.to_string(),
                OutputContent {
                    rows: 0,
                    content: ContentHash::of(b"empty"),
                },
            )]
            .into(),
        };
        let view = CompletedView::new(
            d::analytics::QualityStep::NAME.into(),
            [contribution.identity().unwrap()].into(),
            0,
        )
        .unwrap();
        let source = d::analysis::sources::SourceSnapshot::of_completed_view(
            &Relation::of::<d::analytics::QualityStep>(),
            contribution.spec.model,
            &view,
        )
        .unwrap();
        let store = NativeCompilerStore::publication_owner(
            Arc::new(Surreal::init()),
            d::serving::Name::new("audit").unwrap(),
            d::serving::Name::new("read_owner").unwrap(),
            vec![CompletedBinding {
                boundary: None,
                source,
                view,
                configuration: None,
            }],
        )
        .unwrap();
        assert!(store.set_frontier(Frontier::Catalog).is_err());
        assert!(
            store.abandon().await.is_err(),
            "a read owner cannot abandon an attempt or invalidate its reader session"
        );
        let (release, gate) = tokio::sync::oneshot::channel();
        let stream = futures::stream::once(async move {
            gate.await.unwrap();
            Ok(surrealdb::method::StreamItem::StatementEnd {
                statement: 0,
                stats: Default::default(),
                result: Err(surrealdb::Error::query("late audit tail".into(), None)),
            })
        });
        let rows = store
            .track_rows(NativeRows::new(stream, 1).unwrap())
            .unwrap();
        drop(rows);
        let primary = store
            .capture_audit(&native_operation_budget)
            .await
            .err()
            .expect("unconnected native preparation must fail");
        let mut finalizer = Box::pin(store.drain_report());
        assert!(
            futures::poll!(&mut finalizer).is_pending(),
            "reader pin/session release must wait for admitted tail"
        );
        release.send(()).unwrap();
        let completion = finalizer.await;
        let failure = d::completion::complete::<()>(Err(primary), completion).unwrap_err();
        assert!(failure.to_string().contains("late audit tail"));
        assert_eq!(store.admission.state.lock().unwrap().scans, 0);
    }
    #[tokio::test]
    async fn audit_capture_is_borrowable_only_by_its_original_read_owner() {
        let native_operation_budget =
            lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let owner = || {
            NativeCompilerStore::publication_owner(
                Arc::new(Surreal::init()),
                d::serving::Name::new("audit").unwrap(),
                d::serving::Name::new("capture_owner").unwrap(),
                vec![],
            )
            .unwrap()
        };
        let first = owner();
        let second = owner();
        let capture = first.capture_audit(&native_operation_budget).await.unwrap();
        assert!(second.verify_captured(&capture).await.is_err());
        assert!(second.completed_state_captured(&capture).await.is_err());
        assert!(first.drain_report().await.failures.is_empty());
        assert!(second.drain_report().await.failures.is_empty());
    }
    #[tokio::test]
    async fn cancelled_native_failure_drain_retains_typed_primary_and_late_error() {
        use futures::StreamExt;
        let store = local_store();
        let (release, gate) = tokio::sync::oneshot::channel::<()>();
        let initial = futures::stream::iter(vec![
            Ok(surrealdb::method::StreamItem::StatementEnd {
                statement: 0,
                stats: Default::default(),
                result: Err(surrealdb::Error::query(
                    "owned primary refusal".into(),
                    None,
                )),
            }),
            Err(surrealdb::Error::internal(
                "owned late transport error".into(),
            )),
        ]);
        let tail = futures::stream::once(async move {
            gate.await.unwrap();
            Ok(surrealdb::method::StreamItem::StatementEnd {
                statement: 1,
                stats: Default::default(),
                result: Ok(()),
            })
        });
        let mut rows = store
            .track_rows(NativeRows::new(initial.chain(tail), 2).unwrap())
            .unwrap();
        let mut next = Box::pin(rows.next());
        assert!(futures::poll!(&mut next).is_pending());
        drop(next);
        drop(rows);
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        release.send(()).unwrap();
        let completion = drain.await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.iter().any(|failure|matches!(failure.error.primary(),Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some() && cause.to_string().contains("owned primary refusal"))));
        assert!(completion.failures.iter().any(|failure| {
            failure
                .error
                .to_string()
                .contains("owned late transport error")
        }));
    }

    #[tokio::test]
    async fn retained_read_setup_can_finish_cached_preparation_after_global_close() {
        let store = local_store();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let prepared = PreparedCanonical {
            entities: empty_prepared_rows(&store, &budget).await,
            assertions: empty_prepared_rows(&store, &budget).await,
        };
        let (cache_started, cache_entered) = tokio::sync::oneshot::channel();
        let (release_cache, cache_gate) = tokio::sync::oneshot::channel();
        let pending: Shared<BoxFuture<'static, Result<PreparedCanonical, Arc<ModelError>>>> =
            async move {
                cache_started.send(()).unwrap();
                cache_gate.await.unwrap();
                Ok(prepared)
            }
            .boxed()
            .shared();
        *store.preparation.lock().unwrap() = Some(pending);

        let (setup_started, setup_entered) = tokio::sync::oneshot::channel();
        let (release_setup, setup_gate) = tokio::sync::oneshot::channel();
        let (prepared_tx, prepared_rx) = tokio::sync::oneshot::channel();
        let (release_result, result_gate) = tokio::sync::oneshot::channel();
        let setup_store = store.clone();
        let setup_budget = budget.clone();
        let owner = store.clone();
        let setup = tokio::spawn(async move {
            owner
                .retain_read_setup(
                    "cached canonical setup",
                    Box::pin(async move {
                        setup_started.send(()).unwrap();
                        setup_gate.await.unwrap();
                        let _prepared = setup_store.prepared_canonical(&setup_budget).await?;
                        prepared_tx.send(()).unwrap();
                        result_gate.await.unwrap();
                        Ok(empty_compiler_rows(setup_store))
                    }),
                )
                .await
        });
        setup_entered.await.unwrap();

        store.close_content_lane().await.unwrap();
        store.admission.state.lock().unwrap().content_ready = true;
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        assert!(store.prepare_canonical(&budget).await.is_err());

        release_setup.send(()).unwrap();
        cache_entered.await.unwrap();
        release_cache.send(()).unwrap();
        prepared_rx.await.unwrap();
        assert!(futures::poll!(&mut drain).is_pending());
        release_result.send(()).unwrap();
        drop(setup.await.unwrap().unwrap());

        let completion = drain.await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.is_empty());
        drop(store);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn first_preparation_owner_can_start_its_registered_worker_under_parent_after_close() {
        let store = local_store();
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        store.close_content_lane().await.unwrap();
        store.admission.state.lock().unwrap().content_ready = true;

        let (setup_started, setup_entered) = tokio::sync::oneshot::channel();
        let (release_setup, setup_gate) = tokio::sync::oneshot::channel();
        let (worker_done, worker_finished) = tokio::sync::oneshot::channel();
        let (release_result, result_gate) = tokio::sync::oneshot::channel();
        let setup_store = store.clone();
        let setup_budget = budget.clone();
        let owner = store.clone();
        let setup = tokio::spawn(async move {
            owner
                .retain_read_setup(
                    "first canonical setup",
                    Box::pin(async move {
                        setup_started.send(()).unwrap();
                        setup_gate.await.unwrap();
                        // This is the first-owner branch's shared lease/registered-worker
                        // acquisition, using an empty sorter to avoid a native DB query.
                        let preparation_owner = setup_store.retained_scan()?;
                        let prepared = empty_prepared_rows(&setup_store, &setup_budget).await;
                        preparation_owner.finish();
                        assert!(prepared.cursor()?.next_row()?.is_none());
                        worker_done.send(()).unwrap();
                        result_gate.await.unwrap();
                        Ok(empty_compiler_rows(setup_store))
                    }),
                )
                .await
        });
        setup_entered.await.unwrap();
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        assert!(store.prepare_canonical(&budget).await.is_err());

        release_setup.send(()).unwrap();
        worker_finished.await.unwrap();
        assert!(futures::poll!(&mut drain).is_pending());
        release_result.send(()).unwrap();
        drop(setup.await.unwrap().unwrap());

        let completion = drain.await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Confirmed);
        assert!(completion.failures.is_empty());
        drop(store);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn post_closure_state_requires_a_live_same_store_finalization_parent() {
        let native_operation_budget =
            lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let store = local_store();
        store.admission.state.lock().unwrap().content_ready = true;
        let ordinary = store.admit(false, "native final seal", false).unwrap();
        assert!(
            store
                .completed_state_after_closure(&ordinary, &native_operation_budget)
                .await
                .is_err()
        );
        ordinary.finish();
        let other = local_store();
        other.admission.state.lock().unwrap().content_ready = true;
        let unrelated = other.begin_finalization().await.unwrap();
        assert!(
            store
                .completed_state_after_closure(&unrelated, &native_operation_budget)
                .await
                .is_err()
        );
        unrelated.finish();
        let parent = store.begin_finalization().await.unwrap();
        let scan = store.retained_scan().unwrap();
        let mut read =
            Box::pin(store.completed_state_after_closure(&parent, &native_operation_budget));
        assert!(futures::poll!(&mut read).is_pending());
        let mut drain = Box::pin(store.drain_report());
        assert!(futures::poll!(&mut drain).is_pending());
        drop(read);
        scan.finish();
        assert!(futures::poll!(&mut drain).is_pending());
        parent.finish();
        assert!(drain.await.failures.is_empty());
    }

    #[tokio::test]
    async fn interrupted_final_seal_retains_acknowledged_commit_identity() {
        let store = local_store();
        store.admission.state.lock().unwrap().content_ready = true;
        let lease = store.begin_finalization().await.unwrap();
        let identity = "local_control/owned_control/exact-handle".to_owned();
        lease.record_committed(identity.clone());
        drop(lease);

        let completion = store.drain_report().await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Unknown);
        assert_eq!(
            completion.committed,
            vec![d::completion::CommittedEffect {
                kind: "published unselected manifest",
                identity,
            }]
        );
    }

    #[tokio::test]
    async fn interrupted_final_seal_before_commit_retains_unknown_without_identity() {
        let store = local_store();
        store.admission.state.lock().unwrap().content_ready = true;
        let lease = store.begin_finalization().await.unwrap();
        drop(lease);

        let completion = store.drain_report().await;
        assert_eq!(completion.local, d::completion::LocalState::Terminal);
        assert_eq!(completion.remote, d::completion::RemoteState::Unknown);
        assert!(completion.committed.is_empty());
    }
}

#[cfg(test)]
mod compiler_scope_tests {
    use super::*;
    async fn input_hold_snapshot(
        reader: &crate::NativeReader<()>,
        contribution: ContentHash,
        input: ContentHash,
    ) -> Result<(Vec<Value>, Vec<Value>), ModelError> {
        let object = RecordId::new("compiler_view", input.hex());
        let guard = RecordId::new(
            "native_guard",
            ContentHash::of(
                &serde_json::to_vec(&Value::RecordId(object.clone())).map_err(ModelError::codec)?,
            )
            .hex(),
        );
        let mut vars = Variables::new();
        vars.insert(
            "owner",
            RecordId::new("compiler_contribution", contribution.hex()),
        );
        vars.insert("object", object);
        vars.insert("guard", guard);
        let holds = reader.query_native(
            "SELECT * FROM native_hold WITH INDEX owner_holds WHERE owner=$owner AND object=$object ORDER BY id", vars.clone(),
        ).await?;
        let guards = reader.query_native("SELECT * FROM $guard", vars).await?;
        Ok((holds, guards))
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn batched_empty_views_verify_membership_and_retain_dependency_ownership() {
        let native_operation_budget =
            lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let native = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let client = check_installation(&config).await.unwrap();
        let reader =
            crate::NativeReader::private(client.clone()).with_budget(&native_operation_budget);
        let result = async {
            let nonce = crate::control::fresh_identity("batched-empty-views")?;
            let relation = Relation::of::<d::analytics::QualityStep>();
            let model = d::model()?.digest();
            let base = native
                .begin_contribution(ContributionSpec {
                    captured_binding: None,
                    producer: format!("base-{}", nonce.hex()),
                    profile: OutputProfile::Specific(d::stages::Profile::Catalog),
                    model,
                    implementation: nonce,
                    configuration: None,
                    inputs: vec![],
                    outputs: [relation.name().to_string()].into(),
                })
                .await?;
            let row = d::analytics::QualityStep {
                run: serde_json::from_value(
                    serde_json::to_value(
                        <[u8; 16]>::try_from(&nonce.0[..16]).map_err(ModelError::codec)?,
                    )
                    .map_err(ModelError::codec)?,
                )
                .map_err(ModelError::codec)?,
                ordinal: 0,
                value: d::FiniteF64::new(0.75)?,
            };
            native
                .write_batch(
                    &base,
                    &relation,
                    &d::analytics::QualityStep::encode(&[row])?,
                    &native_operation_budget,
                )
                .await?;
            let base_views = native
                .complete_contribution(
                    base,
                    ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &BTreeMap::new(),
                    &native_operation_budget,
                )
                .await?;
            let base_view = &base_views[relation.name()];
            assert!(
                native.installed_views.lock().unwrap().is_empty(),
                "semantic completion does not demand unused physical selectors"
            );
            let outputs = [
                Relation::of::<d::analytics::RankScore>(),
                Relation::of::<d::analytics::CommunityRun>(),
            ];
            let empty = native
                .begin_contribution(ContributionSpec {
                    captured_binding: None,
                    producer: format!("empty-{}", nonce.hex()),
                    profile: OutputProfile::Specific(d::stages::Profile::Catalog),
                    model,
                    implementation: nonce,
                    configuration: None,
                    inputs: vec![d::analysis::sources::SourceSnapshot::of_completed_view(
                        &relation, model, base_view,
                    )?],
                    outputs: outputs
                        .iter()
                        .map(|relation| relation.name().to_owned())
                        .collect(),
                })
                .await?;
            let before = input_hold_snapshot(&reader, empty, base_view.identity).await?;
            let views = native
                .complete_contribution(
                    empty,
                    ProviderOutcome::Complete,
                    &outputs,
                    &BTreeMap::new(),
                    &native_operation_budget,
                )
                .await?;
            let after = input_hold_snapshot(&reader, empty, base_view.identity).await?;
            let references = views.values().collect::<Vec<_>>();
            let observations = native
                .registered_or_install_views(&references, &native_operation_budget)
                .await?;
            assert_eq!(
                observations.0, 2,
                "one full audit per exact physical contributor"
            );
            assert_eq!(observations.1, 0, "no originals in this case");
            assert_eq!(
                observations.2, 2,
                "normalized contributor fragments reused across selectors"
            );
            for view in views.values() {
                let held = native
                    .installed_view(view, &native_operation_budget)
                    .await?;
                assert!(held.is_internal());
                assert!(held.retained_completion().is_err());
                assert_eq!(held.completion().view, *view);
                let selected = held
                    .completion()
                    .mappings
                    .iter()
                    .map(|mapping| (mapping.view, mapping.logical, mapping.physical))
                    .collect::<std::collections::BTreeSet<_>>();
                let expected = view
                    .contributions
                    .iter()
                    .map(|logical| (view.identity, *logical, empty))
                    .chain(
                        base_view
                            .contributions
                            .iter()
                            .map(|logical| (base_view.identity, *logical, base)),
                    )
                    .collect();
                assert_eq!(
                    selected, expected,
                    "empty output retains exact nonempty dependency closure"
                );
                assert!(
                    held.members(&reader, &[[17; 16]], &native_operation_budget)
                        .await?
                        .rows()
                        .is_empty()
                );
            }
            let prepared = input_hold_snapshot(&reader, empty, base_view.identity).await?;
            assert_eq!(
                after.0, prepared.0,
                "required input incidence remains exact through first metadata-pin acquisition"
            );
            let replay = native
                .complete_contribution(
                    empty,
                    ProviderOutcome::Complete,
                    &outputs,
                    &BTreeMap::new(),
                    &native_operation_budget,
                )
                .await?;
            let replayed = input_hold_snapshot(&reader, empty, base_view.identity).await?;
            assert_eq!(before, after);
            assert_eq!(prepared, replayed);
            assert_eq!(views, replay);
            for object in [
                RecordId::new("compiler_contribution", base.hex()),
                RecordId::new("compiler_contribution", empty.hex()),
                RecordId::new("compiler_view", base_view.identity.hex()),
            ] {
                let eligibility = crate::control::retirement_eligibility(&client, object).await?;
                assert!(
                    !eligibility.eligible,
                    "actual operation/dependency ownership prevents retirement"
                );
            }
            let forged = CompletedView::new(
                base_view.relation.clone(),
                base_view.contributions.clone(),
                0,
            )?;
            let refusal = native
                .registered_or_install_views(&[&forged], &native_operation_budget)
                .await
                .expect_err("nonempty membership defeats forged completed-empty selector");
            assert!(
                !native
                    .installed_views
                    .lock()
                    .unwrap()
                    .contains_key(&forged.identity),
                "failed selector grants no cached capability"
            );
            Ok::<_, ModelError>((views, refusal))
        }
        .await;
        let original = result.as_ref().ok().map(|(_, error)| error);
        fn same_shared_cause(actual: &ModelError, expected: &ModelError) -> bool {
            let mut actual = actual;
            while let ModelError::SharedCause(source) = actual {
                let mut expected = expected;
                while let ModelError::SharedCause(other) = expected {
                    if Arc::ptr_eq(source, other) {
                        return true;
                    }
                    expected = other.as_ref();
                }
                actual = source.as_ref();
            }
            false
        }
        let abandoned = match native.abandon().await {
            Ok(()) if original.is_none() => Ok(()),
            Err(ModelError::Completion(outcome))
                if original.is_some()
                    && outcome.primary.is_none()
                    && outcome.completion.local == d::completion::LocalState::Terminal
                    && outcome.completion.remote == d::completion::RemoteState::Confirmed
                    && outcome.completion.storage.is_empty()
                    && !outcome.completion.failures.is_empty()
                    && outcome.completion.failures.iter().all(|failure| {
                        original.is_some_and(|expected| same_shared_cause(&failure.error, expected))
                    }) =>
            {
                Ok(())
            }
            Err(error) => Err(error),
            Ok(()) => Err(ModelError::Conflict(
                "acknowledged selector refusal lost at finalization",
            )),
        };
        let mut completion = d::completion::Completion::default();
        completion.step("earned empty selection attempt close", abandoned);
        if completion.failures.is_empty() {
            let mut variables = Variables::new();
            variables.insert(
                "attempt",
                RecordId::new("native_attempt", native.attempt().hex()),
            );
            let states: Vec<String> = reader
                .query_native("SELECT VALUE state FROM $attempt", variables)
                .await
                .unwrap();
            assert_eq!(states, ["abandoned"]);
        }
        completion.step("earned empty selection reader close", reader.close().await);
        completion.step(
            "earned empty selection session close",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let (views, _) = d::completion::complete(result, completion).unwrap();
        assert_eq!(views.len(), 2);
        assert!(views.values().all(|view| view.rows == 0));
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn imported_membership_pages_retain_contributor_reachability_without_duplicate_attempt_roots()
     {
        let native_operation_budget =
            lctx_model::domain::resources::ResourceBudget::fixed(256 << 20).unwrap();
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let source = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let target = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let client = check_installation(&config).await.unwrap();
        let reader = crate::NativeReader::private(client.clone());
        let result = async {
            let nonce = crate::control::fresh_identity("partial-membership-import")?;
            let row = d::analytics::QualityStep {
                run: serde_json::from_value(serde_json::to_value(<[u8;16]>::try_from(&nonce.0[..16]).map_err(ModelError::codec)?).map_err(ModelError::codec)?).map_err(ModelError::codec)?,
                ordinal: 0, value: d::FiniteF64::new(0.75)?,
            };
            let relation = Relation::of::<d::analytics::QualityStep>();
            let spec = ContributionSpec {
                captured_binding: None, producer: nonce.hex(), profile: lctx_model::domain::completed::OutputProfile::Specific(d::stages::Profile::Catalog),
                model: d::model()?.digest(), implementation: nonce, configuration: None,
                inputs: vec![], outputs: std::collections::BTreeSet::from([relation.name().to_string()]),
            };
            let contribution = source.begin_contribution(spec.clone()).await?;
            source.write_batch(&contribution, &relation, &d::analytics::QualityStep::encode(&[row])?, &native_operation_budget).await?;
            let views = source.complete_contribution(contribution, ProviderOutcome::Complete, std::slice::from_ref(&relation), &BTreeMap::new(), &native_operation_budget).await?;
            let view = views[relation.name()].clone();
            source.bind(CompletedBinding {
                boundary: None,
                source: d::analysis::sources::SourceSnapshot::of_completed_view(&relation, spec.model, &view)?,
                view, configuration: None,
            }).await?;
            source.retain_product_identity(nonce, contribution, &native_operation_budget).await?;
            source.freeze_content(&native_operation_budget).await?;
            let directory = tempfile::tempdir().map_err(ModelError::codec)?;
            let path = directory.path().join("completed.jsonl");
            let expected = source.export_state(&path, &native_operation_budget).await?;
            let decoded=lctx_native::completed_transport::read_transport(&path,&expected,&native_operation_budget)?;
            let [copy]=decoded.envelope().copies.as_slice() else {return Err(ModelError::Schema("single copy partial import fixture"));};
            let imported_id=target.register_transport_copy(copy,&expected,&native_operation_budget).await?;
            let mut cursor=decoded.copies()[0].memberships.cursor_with_budget(&native_operation_budget)?;
            let value=cursor.next_row()?.ok_or(ModelError::Schema("single row partial import fixture"))?;
            if cursor.next_row()?.is_some(){return Err(ModelError::Schema("single row partial import fixture"));}
            let mut row=value.as_object().ok_or(ModelError::Schema("partial membership object"))?.clone();
            let (Some(Value::String(member_relation)),Some(Value::String(key)))=(row.get("relation"),row.get("semantic_key")) else {return Err(ModelError::Schema("partial membership identity"));};
            row.insert("id",membership_id(imported_id,member_relation,&decode_nominal_key(key)?));row.insert("contribution",RecordId::new("compiler_contribution",imported_id.hex()));
            let value=Value::Object(row);
            crate::control::ensure_rows(&target.client,Some(target.attempt),vec![value.clone()]).await?;
            let Some(Value::RecordId(member))=value.as_object().and_then(|row|row.get("id")) else{return Err(ModelError::Schema("partial membership id"));};
            crate::control::hold_many(&target.client,Some(target.attempt),vec![(RecordId::new("compiler_contribution",imported_id.hex()),member.clone())]).await?;
            let premature = target.complete_contribution_inner(imported_id, ProviderOutcome::Complete,
                std::slice::from_ref(&relation), &BTreeMap::new(), &native_operation_budget).await;
            if !matches!(premature, Err(ModelError::Conflict("native contribution ownership not ready"))) {
                return Err(ModelError::Conflict("partial import granted completion authority"));
            }
            let mut vars = Variables::new();
            vars.insert("attempt", RecordId::new("native_attempt", target.attempt.hex()));
            let contributors: Vec<RecordId> = reader.query_native("SELECT VALUE id FROM compiler_contribution WITH INDEX attempt_contributions WHERE attempt=$attempt", vars.clone()).await?;
            vars.insert("contributors", contributors.clone());
            let members: Vec<Object> = reader.query_native("SELECT id,node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $contributors", vars.clone()).await?;
            let [member] = members.as_slice() else { return Err(ModelError::Schema("partial import membership")); };
            let (Some(Value::RecordId(membership)), Some(Value::RecordId(node))) = (member.get("id"), member.get("node")) else { return Err(ModelError::Schema("partial import pointers")); };
            let membership = membership.clone();
            vars.insert("node", node.clone());
            let original: Vec<Value> = reader.query_native("SELECT * FROM $node", vars.clone()).await?;
            let attempt_roots: Vec<RecordId> = reader.query_native("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt", vars.clone()).await?;
            let contributor_roots: Vec<RecordId> = reader.query_native("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner IN $contributors", vars.clone()).await?;
            let completed: Vec<bool> = reader.query("SELECT VALUE completed FROM $contributors", vars.clone()).await?;
            let eligibility = crate::control::retirement_eligibility(&client, membership.clone()).await?;
            target.drain().await?;
            crate::control::close_attempt(&client, target.attempt, "abandoned").await?;
            let terminal_roots: Vec<RecordId> = reader.query_native("SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt", vars.clone()).await?;
            let mut retired = crate::control::retire_reachable(&client, contributors.clone(), 16).await?;
            // A bounded advancement need not revisit children inserted before its
            // current item cursor, even when its effect allowance is not exhausted.
            while !retired.remaining.is_empty() {
                if !retired.retained.is_empty() {
                    return Err(ModelError::Conflict("partial import cleanup remains protected"));
                }
                let next = crate::control::resume_retirement(&client, retired.identity, 16).await?;
                if next.remaining == retired.remaining && next.retired == retired.retired && next.examined == retired.examined {
                    return Err(ModelError::Conflict("partial import cleanup made no progress"));
                }
                retired = next;
            }
            let remaining_members: Vec<RecordId> = reader.query_native("SELECT VALUE id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $contributors", vars.clone()).await?;
            let retained: Vec<Value> = reader.query_native("SELECT * FROM $node", vars).await?;
            source.verify_state(&native_operation_budget).await?;
            let state_after_cleanup = source.completed_state(&native_operation_budget).await?;
            Ok::<_, ModelError>((contributors, membership, attempt_roots, contributor_roots, completed, eligibility, terminal_roots, retired, remaining_members, original, retained, expected, state_after_cleanup))
        }.await;
        let mut completion = d::completion::Completion::default();
        completion.step(
            "partial membership target finalization",
            target.abandon().await,
        );
        completion.step(
            "partial membership source finalization",
            source.abandon().await,
        );
        completion.step("partial membership reader close", reader.close().await);
        completion.step(
            "partial membership session close",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let (
            contributors,
            membership,
            attempt_roots,
            contributor_roots,
            completed,
            eligibility,
            terminal_roots,
            retired,
            remaining_members,
            original,
            retained,
            expected,
            state_after_cleanup,
        ) = d::completion::complete(result, completion).unwrap();
        assert_eq!(contributors.len(), 1);
        assert_eq!(
            attempt_roots, contributors,
            "only the contributor is an attempt root"
        );
        assert_eq!(
            contributor_roots,
            vec![membership],
            "the contributor protects its imported membership page"
        );
        assert_eq!(
            completed,
            vec![false],
            "later import completion did not run"
        );
        assert!(
            !eligibility.eligible
                && eligibility
                    .reasons
                    .iter()
                    .any(|reason| reason == "reachable through native owner hold")
        );
        assert!(terminal_roots.is_empty());
        assert_eq!(
            retired.retired, 2,
            "only the abandoned contributor and its membership retire"
        );
        assert!(
            retired.remaining.is_empty()
                && retired.retained.is_empty()
                && remaining_members.is_empty()
        );
        assert_eq!(original.len(), 1);
        assert_eq!(
            retained, original,
            "admitted shared backing content remains exact"
        );
        assert_eq!(state_after_cleanup, expected.semantic);
    }
    #[test]
    fn canonical_compiler_backing_rejects_imported_scope_and_retired_scalars() {
        let native_operation_budget = ResourceBudget::fixed(256 << 20).unwrap();
        let record = d::analytics::QualityStep {
            run: serde_json::from_value(serde_json::to_value([7u8; 16]).unwrap()).unwrap(),
            ordinal: 0,
            value: d::FiniteF64::new(0.5).unwrap(),
        };
        let relation = Relation::of::<d::analytics::QualityStep>();
        let body = crate::codec::batch_bodies(
            &relation,
            &d::analytics::QualityStep::encode(std::slice::from_ref(&record)).unwrap(),
        )
        .unwrap()
        .pop()
        .unwrap();
        let canonical = serde_json::to_vec(&body).unwrap();
        let id = crate::loader::payload_id(
            "compiler_record",
            relation.name(),
            record.id().bytes(),
            ContentHash::of(&canonical),
        )
        .unwrap();
        let mut row = Object::new();
        row.insert("id", id.clone());
        row.insert("semantic_type", relation.name().to_string());
        row.insert("semantic_key", hex::encode(record.id().bytes()));
        row.insert("body", body);
        row.insert("canonical", Bytes::from(canonical.clone()));
        row.insert("content", ContentHash::of(&canonical).hex());
        let expected = format!(
            "{}|run|{}",
            relation.name(),
            Value::from_t(vec![7i64; 16]).to_sql()
        );
        row.insert("scope_keys", vec![expected]);
        assert!(validate_compiler_backing(&row, &id).unwrap().is_none());
        validate_backing_batch(&[Value::Object(row.clone())], &native_operation_budget).unwrap();
        for field in ["unexpected", "kind", "subtype", "scope_context"] {
            let mut altered = row.clone();
            altered.insert(field, Value::Null);
            assert!(
                validate_compiler_backing(&altered, &id).is_err(),
                "extra envelope field {field}"
            );
            assert!(
                validate_backing_batch(&[Value::Object(altered)], &native_operation_budget)
                    .is_err(),
                "reconstructed envelope field {field}"
            );
        }
        for field in [
            "id",
            "semantic_type",
            "semantic_key",
            "content",
            "canonical",
            "scope_keys",
        ] {
            let mut altered = row.clone();
            altered.remove(field);
            assert!(
                validate_compiler_backing(&altered, &id).is_err(),
                "missing envelope field {field}"
            );
        }
        row.insert("scope_run", "retired scalar");
        assert!(validate_compiler_backing(&row, &id).is_err());
        row.remove("scope_run");
        row.insert("scope_keys", vec!["imported wrong key"]);
        assert!(validate_compiler_backing(&row, &id).is_err());
    }
}

#[cfg(test)]
mod c04_projection_tests {
    use super::compact_key_field;
    #[test]
    fn compact_projection_never_bypasses_a_value_predicate_or_required_payload() {
        assert_eq!(
            compact_key_field("semantic_key AS id", "true", &[]),
            Some("id")
        );
        assert_eq!(
            compact_key_field("semantic_key AS __row", "true", &[]),
            Some("__row")
        );
        assert_eq!(
            compact_key_field("semantic_key AS id", "body.`version` = $v0", &[]),
            None
        );
        assert_eq!(
            compact_key_field(
                "semantic_key AS id,body.`version` AS `version`",
                "true",
                &[]
            ),
            None
        );
        assert_eq!(
            compact_key_field("semantic_key AS id", "true", &["LET $scope = 1".into()]),
            None
        );
    }
}
