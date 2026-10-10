//! Attempt-scoped native compiler authority in one durable shared database. Completed memberships, rather than a publication handle,
//! govern reads. Canonical graph payloads and query fields are immutable mechanical forms.
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
use lctx_model::domain::{
    self as d, admission::Frontier, completed::*, stages::ProviderOutcome, *,
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

pub struct NativeCompilerStore {
    pub(crate) client: Arc<Surreal<Client>>,
    database: d::serving::Name,
    attempt: ContentHash,
    generation: ContentHash,
    published_bindings: Mutex<Option<Vec<CompletedBinding>>>,
    namespace: d::serving::Name,
    endpoint: Option<String>,
    specifications: Mutex<BTreeMap<ContentHash, (ContributionSpec, bool)>>,
    known_views: Mutex<BTreeMap<ContentHash, CompletedView>>,
    known_contributors: Mutex<BTreeMap<ContentHash, ContentHash>>,
    failed: AtomicBool,
    sealed: AtomicBool,
    seal_started: AtomicBool,
    preparation: Mutex<Option<CanonicalPreparation>>,
    membership_preparations:
        Mutex<BTreeMap<ContentHash, (MembershipPreparation, Arc<Box<dyn Reservation>>)>>,
    producing_scopes: Mutex<BTreeMap<ContentHash, Arc<ProducingAdmission>>>,
    admission: Arc<OperationAdmission>,
    runtime: tokio::runtime::Handle,
    frontier: Mutex<Frontier>,
}
pub struct FinalizationInventory {
    pub contributions: Vec<CompletedContribution>,
    pub bindings: Vec<CompletedBinding>,
}
/// One operation's exact binding inventory and immutable completed dependency closure.
struct StateSelection {
    bindings: Vec<CompletedBinding>,
    variables: Variables,
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

pub fn compiler_schema() -> String {
    "DEFINE TABLE compiler_contribution SCHEMAFULL; DEFINE FIELD spec ON compiler_contribution TYPE bytes; DEFINE FIELD descriptor ON compiler_contribution TYPE option<bytes>; DEFINE FIELD completed ON compiler_contribution TYPE bool; DEFINE FIELD logical ON compiler_contribution TYPE option<string>; DEFINE INDEX logical_contribution ON compiler_contribution FIELDS logical; DEFINE FIELD attempt ON compiler_contribution TYPE record<native_attempt>; DEFINE INDEX attempt_contributions ON compiler_contribution FIELDS attempt; DEFINE TABLE compiler_membership SCHEMAFULL; DEFINE FIELD contribution ON compiler_membership TYPE record<compiler_contribution>; DEFINE FIELD relation ON compiler_membership TYPE string; DEFINE FIELD semantic_key ON compiler_membership TYPE string; DEFINE FIELD node ON compiler_membership TYPE record<entity | assertion | compiler_record>; DEFINE FIELD content ON compiler_membership TYPE string; DEFINE INDEX contribution_rows ON compiler_membership FIELDS contribution,relation,semantic_key UNIQUE; DEFINE INDEX member_keys ON compiler_membership FIELDS relation,semantic_key,contribution; DEFINE TABLE compiler_view SCHEMAFULL; DEFINE FIELD descriptor ON compiler_view TYPE bytes; DEFINE TABLE compiler_binding SCHEMAFULL; DEFINE FIELD descriptor ON compiler_binding TYPE bytes; DEFINE FIELD attempt ON compiler_binding TYPE record<native_attempt>; DEFINE TABLE compiler_alias SCHEMAFULL; DEFINE FIELD source ON compiler_alias TYPE record<entity>; DEFINE FIELD target ON compiler_alias TYPE record<entity>; DEFINE INDEX alias_source ON compiler_alias FIELDS source,target UNIQUE;".to_string()+"DEFINE FIELD producer ON compiler_contribution TYPE string; DEFINE FIELD profile ON compiler_contribution TYPE string; DEFINE FIELD model ON compiler_contribution TYPE string; DEFINE FIELD implementation ON compiler_contribution TYPE string; DEFINE FIELD configuration ON compiler_contribution TYPE option<string|null>; DEFINE FIELD inputs ON compiler_contribution TYPE array<record<compiler_view>>; DEFINE FIELD outputs ON compiler_contribution TYPE array<string>; DEFINE FIELD outcome ON compiler_contribution TYPE option<int>; DEFINE INDEX contribution_inputs ON compiler_contribution FIELDS inputs; DEFINE FIELD relation ON compiler_view TYPE string; DEFINE FIELD contributions ON compiler_view TYPE array<string>; DEFINE FIELD rows ON compiler_view TYPE int; DEFINE FIELD relation ON compiler_binding TYPE string; DEFINE FIELD boundary ON compiler_binding TYPE option<string|null>; DEFINE FIELD view ON compiler_binding TYPE record<compiler_view>; DEFINE INDEX binding_view ON compiler_binding FIELDS view;"+&crate::schema::compiler_record_schema()
}

/// Explicit maintenance installation only. Ordinary compiler attachment never defines schema.
pub async fn install_shared(
    config: &RuntimeConfig,
    native_definitions: &str,
) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native installation requires installer authority".into(),
        ));
    }
    let client =
        reader::authenticated(&config.endpoint, &config.writer_credentials(), None).await?;
    client
        .query(format!(
            "DEFINE NAMESPACE IF NOT EXISTS `{}`",
            config.namespace.as_str()
        ))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    client
        .use_ns(config.namespace.as_str())
        .await
        .map_err(ModelError::codec)?;
    client
        .query(format!(
            "DEFINE DATABASE IF NOT EXISTS `{}` STRICT",
            config.database.as_str()
        ))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    client
        .use_db(config.database.as_str())
        .await
        .map_err(ModelError::codec)?;
    let loader = Loader::new(client.clone());
    loader
        .install_declarations(crate::control::schema(), "native durable control schema")
        .await?;
    loader.install(native_definitions).await?;
    loader
        .install_declarations(&compiler_schema(), "compiler shared state schema")
        .await?;
    loader
        .install_declarations(VIEW_SCHEMA, "exact completed view mapping schema")
        .await?;
    let mut bindings = Variables::new();
    bindings.insert("generation", config.service_generation.hex());
    bindings.insert("schema", base_schema_identity().hex());
    client.query("UPSERT native_installation:current SET generation=$generation,schema=$schema,schema_version=3,admission_open=false RETURN NONE").bind(bindings).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
    Ok(())
}
pub async fn check_installation(
    config: &RuntimeConfig,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    let client = reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    let version = client
        .version()
        .await
        .map_err(ModelError::codec)?
        .to_string();
    if !version.starts_with("3.3.0") {
        return Err(ModelError::Invalid(
            "shared native store requires pinned SurrealDB 3.3.0".into(),
        ));
    }
    crate::control::check_installation(&client, config.service_generation).await?;
    Ok(client)
}
pub fn base_schema_identity() -> ContentHash {
    ContentHash::of(
        format!(
            "{}{}{}{}",
            crate::schema::canonical_schema(),
            compiler_schema(),
            crate::control::schema(),
            VIEW_SCHEMA
        )
        .as_bytes(),
    )
}
pub async fn close_admission(config: &RuntimeConfig) -> Result<(), ModelError> {
    maintenance_admission(config, false).await
}
pub async fn open_admission(config: &RuntimeConfig) -> Result<(), ModelError> {
    maintenance_admission(config, true).await
}
async fn maintenance_admission(config: &RuntimeConfig, open: bool) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native maintenance requires installer authority".into(),
        ));
    }
    let client = check_installation(config).await?;
    if open {
        crate::control::check_pending_effects(&client).await?;
    }
    client.query("UPDATE native_installation:current SET admission_open=$open,admission_revision=(admission_revision ?? 0)+1 RETURN NONE").bind(("open",open)).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
    Ok(())
}
pub async fn drain_installation(config: &RuntimeConfig) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native maintenance requires installer authority".into(),
        ));
    }
    let client = check_installation(config).await?;
    let mut response = client
        .query("SELECT VALUE admission_open FROM native_installation:current")
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let open: Vec<bool> = response.take(0).map_err(ModelError::codec)?;
    if open != vec![false] {
        return Err(ModelError::Conflict(
            "maintenance admission must close before drainage",
        ));
    }
    loop {
        let mut response = client
            .query("SELECT VALUE id FROM native_attempt WHERE state='open' LIMIT 128")
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let attempts: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
        if attempts.is_empty() {
            break;
        }
        crate::control::effect(
            &client,
            None,
            "UPDATE $attempts SET state='maintenance_fenced',revision+=1 RETURN NONE",
            {
                let mut bindings = Variables::new();
                bindings.insert("attempts", attempts);
                bindings
            },
        )
        .await?;
    }
    crate::control::drain_effects(&client).await?;
    let mut response=client.query("SELECT VALUE id FROM native_pin WHERE released=false LIMIT 1; SELECT VALUE id FROM native_backup_hold WHERE active=true LIMIT 1").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let pins: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
    let backups: Vec<RecordId> = response.take(1).map_err(ModelError::codec)?;
    if !pins.is_empty() || !backups.is_empty() {
        return Err(ModelError::Conflict(
            "native readers or backup holds remain live",
        ));
    }
    Ok(())
}
/// Explicit recovery of named abandoned clients after the service owner proves actual
/// host drainage. Neither identity age nor an empty view list establishes that proof.
/// Successful per-identity fences are durable if a later identity fails; retry the same list.
pub async fn reconcile_maintenance(
    config: &RuntimeConfig,
    pins: &[ContentHash],
    backup_holds: &[ContentHash],
    readers_stopped: bool,
) -> Result<(), ModelError> {
    if !matches!(config.authentication, crate::AuthenticationScope::Root) {
        return Err(ModelError::Invalid(
            "native maintenance reconciliation requires installer authority".into(),
        ));
    }
    if !readers_stopped {
        return Err(ModelError::Conflict(
            "maintenance reconciliation requires stopped readers",
        ));
    }
    let client = check_installation(config).await?;
    let mut response = client
        .query("SELECT VALUE admission_open FROM native_installation:current")
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let open: Vec<bool> = response.take(0).map_err(ModelError::codec)?;
    if open != vec![false] {
        return Err(ModelError::Conflict(
            "maintenance admission must close before reconciliation",
        ));
    }
    crate::control::drain_effects(&client).await?;
    let mut completion = lctx_model::domain::completion::Completion::default();
    let result = async {
        for pin in pins {
            crate::control::release_pin_checked(&client, *pin, true).await?;
            completion.committed("reconciled native reader pin", pin.hex());
        }
        for hold in backup_holds {
            crate::control::release_backup_hold_checked(&client, *hold, true).await?;
            completion.committed("reconciled native backup hold", hold.hex());
        }
        Ok(())
    }
    .await;
    lctx_model::domain::completion::complete(result, completion)
}
/// Normalize runtime ownership for the portable format. Payload addresses are unchanged.
pub fn normalize_transport_row(
    table: &str,
    value: &Value,
    owners: &BTreeMap<RecordId, ContributionSpec>,
) -> Result<Value, ModelError> {
    let mut row = value_object(value)
        .ok_or(ModelError::Schema("portable state object"))?
        .clone();
    let zero = ContentHash([0; 32]);
    match table {
        "compiler_contribution" => {
            let descriptor: CompletedContribution = decode_descriptor(value)?;
            row.insert(
                "id",
                RecordId::new(
                    table,
                    attempt_contribution(zero, descriptor.spec.identity()?).hex(),
                ),
            );
            row.insert("attempt", RecordId::new("native_attempt", zero.hex()));
        }
        "compiler_binding" => {
            let binding: CompletedBinding = decode_descriptor(value)?;
            row.insert(
                "id",
                RecordId::new(table, attempt_binding(zero, binding.key()).hex()),
            );
            row.insert("attempt", RecordId::new("native_attempt", zero.hex()));
        }
        "compiler_membership" => {
            let Some(Value::RecordId(source)) = row.get("contribution") else {
                return Err(ModelError::Schema("portable membership owner"));
            };
            let spec = owners
                .get(source)
                .ok_or(ModelError::Conflict("portable membership owner"))?;
            let owner = attempt_contribution(zero, spec.identity()?);
            let Some(Value::String(relation)) = row.get("relation") else {
                return Err(ModelError::Schema("portable membership relation"));
            };
            let Some(Value::String(key)) = row.get("semantic_key") else {
                return Err(ModelError::Schema("portable membership key"));
            };
            row.insert(
                "id",
                membership_id(owner, relation, &decode_nominal_key(key)?),
            );
            row.insert(
                "contribution",
                RecordId::new("compiler_contribution", owner.hex()),
            );
        }
        _ => {}
    }
    Ok(Value::Object(row))
}
pub const VIEW_SCHEMA: &str = "DEFINE TABLE compiler_view_member SCHEMAFULL; DEFINE FIELD view ON compiler_view_member TYPE record<compiler_view>; DEFINE FIELD relation ON compiler_view_member TYPE string; DEFINE FIELD semantic_key ON compiler_view_member TYPE string; DEFINE FIELD node ON compiler_view_member TYPE record<entity | assertion | compiler_record>; DEFINE FIELD content ON compiler_view_member TYPE string; DEFINE INDEX view_key ON compiler_view_member FIELDS view,relation,semantic_key UNIQUE; DEFINE INDEX view_nodes ON compiler_view_member FIELDS view,node;";

impl NativeCompilerStore {
    /// Read-only exact publication context. This imports no runtime attempt ownership and
    /// creates no database or schema; pins belong to the published reader owner.
    pub async fn from_publication(
        client: Arc<Surreal<Client>>,
        namespace: d::serving::Name,
        database: d::serving::Name,
        bindings: Vec<CompletedBinding>,
    ) -> Result<Arc<Self>, ModelError> {
        binding_inventory_identity(&bindings)?;
        let store = Self::from_existing(client, namespace, database);
        *store
            .published_bindings
            .lock()
            .map_err(|_| ModelError::Conflict("published bindings"))? = Some(bindings.clone());
        store.state_closure().await?;
        Ok(store)
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
        let mut queue = bindings
            .iter()
            .map(|binding| binding.view.clone())
            .collect::<Vec<_>>();
        let mut views = BTreeMap::new();
        let mut owners = std::collections::BTreeSet::new();
        while let Some(view) = queue.pop() {
            if let Some(old) = views.get(&view.identity) {
                if old != &view {
                    return Err(ModelError::Conflict("state view descriptor collision"));
                }
                continue;
            }
            self.registered_view(&view).await?;
            for owner in self.view_owners(&view)? {
                if !owners.insert(owner) {
                    continue;
                }
                let mut stream = self.track_rows(NativeRows::new(
                    self.client
                        .query("SELECT * FROM $owner WHERE completed=true")
                        .bind(("owner", RecordId::new("compiler_contribution", owner.hex())))
                        .stream_items()
                        .map_err(ModelError::codec)?,
                    1,
                )?)?;
                let row = stream
                    .next()
                    .await?
                    .ok_or(ModelError::Conflict("state dependency contributor"))?;
                validate_state_row("compiler_contribution", &row)?;
                if stream.next().await?.is_some() {
                    return Err(ModelError::Conflict(
                        "duplicate state dependency contributor",
                    ));
                }
                let descriptor: CompletedContribution = decode_descriptor(&row)?;
                if !view.contributions.contains(&descriptor.identity()?) {
                    return Err(ModelError::Conflict(
                        "state dependency contributor identity",
                    ));
                }
                for input in descriptor.spec.inputs {
                    let mut stream = self.track_rows(NativeRows::new(
                        self.client
                            .query("SELECT * FROM $view")
                            .bind(("view", RecordId::new("compiler_view", input.view().hex())))
                            .stream_items()
                            .map_err(ModelError::codec)?,
                        1,
                    )?)?;
                    let row = stream
                        .next()
                        .await?
                        .ok_or(ModelError::Conflict("state dependency view"))?;
                    validate_state_row("compiler_view", &row)?;
                    if stream.next().await?.is_some() {
                        return Err(ModelError::Conflict("duplicate state dependency view"));
                    }
                    let dependency: CompletedView = decode_descriptor(&row)?;
                    if dependency.identity != input.view()
                        || dependency.relation != input.relation()
                        || input.rows() < 0
                        || dependency.rows != input.rows() as u64
                    {
                        return Err(ModelError::Conflict("state dependency view metadata"));
                    }
                    queue.push(dependency);
                }
            }
            views.insert(view.identity, view);
        }
        Ok((views, owners))
    }
    async fn capture_state(self: &Arc<Self>) -> Result<StateSelection, ModelError> {
        let bindings = self.bindings_inner().await?;
        let (views, owners) = self.state_closure_from(&bindings).await?;
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
            views
                .into_keys()
                .map(|view| RecordId::new("compiler_view", view.hex()))
                .collect::<Vec<_>>(),
        );
        Ok(StateSelection {
            bindings,
            variables,
        })
    }
    fn state_query(
        &self,
        table: &str,
        bindings: Variables,
    ) -> Result<crate::prepared::PreparedQuery, ModelError> {
        let mut preparation = Vec::new();
        if matches!(table, "compiler_record" | "compiler_alias") {
            let family = if table == "compiler_record" {
                "compiler_record"
            } else {
                "entity"
            };
            preparation.push(format!("LET $state_nodes = array::distinct(SELECT VALUE node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $state_owners AND contribution.completed=true AND record::table(node)='{family}')"));
        }
        let result=match table {
            "compiler_contribution"=>"SELECT * FROM $state_owners WHERE completed=true ORDER BY id".to_string(),
            "compiler_membership"=>"SELECT * FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $state_owners AND contribution.completed=true ORDER BY id".into(),
            "compiler_view"=>"SELECT * FROM $state_views ORDER BY id".into(),
            "compiler_record"=>"SELECT * FROM $state_nodes WHERE record::table(id)='compiler_record' ORDER BY id".into(),
            "compiler_alias"=>{
                preparation.push("LET $state_aliases = SELECT VALUE id FROM compiler_alias WITH INDEX alias_source WHERE source IN $state_nodes".into());
                "SELECT * FROM $state_aliases ORDER BY id".into()
            },
            // Binding transport is generated from the exact logical inventory below.
            _=>return Err(ModelError::Schema("native state query family")),
        };
        crate::prepared::PreparedQuery::new(bindings, preparation, vec![result])
    }
    fn state_source(
        &self,
        table: &str,
        selection: &StateSelection,
    ) -> Result<NativeRows, ModelError> {
        self.state_query(table, selection.variables.clone())?
            .stream(&self.client)
    }
    async fn portable_state_rows(
        self: &Arc<Self>,
        table: &str,
        selection: &StateSelection,
    ) -> Result<AsyncOrderedRows, ModelError> {
        let budget = crate::ordered_rows::portable_ordering_budget()?;
        let mut sorted = AsyncPhysicalSort::new_registered_with_row_bytes(
            &budget,
            self.blocking_owner()?,
            self.blocking_observer()?,
            d::resources::MAX_ROW_BYTES,
        )
        .await?;
        if table == "compiler_binding" {
            for binding in &selection.bindings {
                let mut row = Object::new();
                row.insert(
                    "id",
                    RecordId::new(
                        "compiler_binding",
                        attempt_binding(ContentHash([0; 32]), binding.key()).hex(),
                    ),
                );
                row.insert(
                    "attempt",
                    RecordId::new("native_attempt", ContentHash([0; 32]).hex()),
                );
                row.insert(
                    "descriptor",
                    Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?),
                );
                for (key, value) in binding_projection(binding) {
                    row.insert(key, value);
                }
                sorted.push(Value::Object(row)).await?;
            }
        } else {
            let mut portable_owners = BTreeMap::new();
            if table == "compiler_membership" {
                let owners: Vec<Value> = crate::NativeReader::private(self.client.clone())
                    .query_prepared_native(
                        self.state_query("compiler_contribution", selection.variables.clone())?,
                    )
                    .await?;
                for value in owners {
                    let row = value_object(&value).ok_or(ModelError::Schema("portable owner"))?;
                    let Some(Value::RecordId(id)) = row.get("id") else {
                        return Err(ModelError::Schema("portable owner identity"));
                    };
                    let descriptor: CompletedContribution = decode_descriptor(&value)?;
                    portable_owners.insert(id.clone(), descriptor.spec);
                }
            }
            let mut source = self.track_rows(
                self.state_source(table, selection)?
                    .with_row_bytes(d::resources::MAX_ROW_BYTES),
            )?;
            while let Some(value) = source.next_native().await? {
                let mut row = value_object(&value)
                    .ok_or(ModelError::Schema("portable state row"))?
                    .clone();
                row = value_object(&normalize_transport_row(
                    table,
                    &Value::Object(row),
                    &portable_owners,
                )?)
                .ok_or(ModelError::Schema("portable normalized row"))?
                .clone();
                sorted.push(Value::Object(row)).await?;
            }
        }
        sorted.finish().await
    }
    async fn registered_or_install_views(
        self: &Arc<Self>,
        views: &[&CompletedView],
    ) -> Result<(), ModelError> {
        // Membership preparation depends on verified contributor mappings, not early
        // known-view registration. Remember a view only after checked EOF and ownership.
        let window_budget = ResourceBudget::fixed(d::resources::MAX_ROW_BYTES.saturating_mul(4))?;
        let mut descriptor_charge = window_budget.reserve("native-view-descriptor-window", 0)?;
        let mut reference_charge = window_budget.reserve("native-view-retention-window", 0)?;
        let mut start = 0;
        while start < views.len() {
            let mut end = start;
            let mut rows = Vec::new();
            let mut bytes = 0usize;
            while end < views.len() {
                let view = views[end];
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
                let weight = crate::loader::native_bytes(&row);
                if !rows.is_empty()
                    && (rows.len() == crate::loader::NATIVE_WINDOW_ROWS
                        || bytes.saturating_add(weight) > d::resources::TRANSFER_BYTES)
                {
                    break;
                }
                bytes = bytes.saturating_add(weight);
                descriptor_charge.try_resize(bytes.saturating_mul(4))?;
                rows.push(row);
                end += 1;
            }
            crate::control::ensure_rows(&self.client, Some(self.attempt), rows).await?;
            descriptor_charge.try_resize(0)?;
            let window = &views[start..end];
            let mut references = Vec::new();
            let mut reference_bytes = 0usize;
            for view in window {
                // Preserve the existing per-view preparation budget and retained cache ownership.
                let budget = ResourceBudget::fixed(d::resources::MAX_ROW_BYTES.saturating_mul(4))?;
                let prepared = self.prepare_memberships(view, &budget).await?;
                let mut ordered = AsyncOrderedCandidates::new_registered(
                    prepared,
                    &budget,
                    self.blocking_owner()?,
                    self.blocking_observer()?,
                )
                .await?;
                let mut count = 0u64;
                loop {
                    let candidates = ordered
                        .next_batch(crate::loader::NATIVE_WINDOW_ROWS)
                        .await?;
                    if candidates.is_empty() {
                        break;
                    }
                    let mut rows = Vec::with_capacity(candidates.len());
                    let mut nodes = Vec::with_capacity(candidates.len());
                    for candidate in candidates {
                        let mut sink = KeySink::new("native-view-member/v1");
                        view.identity.encode(&mut sink);
                        candidate.relation.encode(&mut sink);
                        sink.part(b"key", &candidate.key);
                        let mut row = Object::new();
                        row.insert(
                            "id",
                            RecordId::new("compiler_view_member", sink.finish().hex()),
                        );
                        row.insert("view", RecordId::new("compiler_view", view.identity.hex()));
                        row.insert("relation", candidate.relation);
                        row.insert("semantic_key", hex::encode(candidate.key));
                        row.insert("node", candidate.node.clone());
                        row.insert(
                            "content",
                            candidate
                                .content
                                .ok_or(ModelError::Schema("exact view content"))?
                                .hex(),
                        );
                        nodes.push(candidate.node);
                        if let Some(Value::RecordId(id)) = row.get("id") {
                            nodes.push(id.clone());
                        }
                        rows.push(Value::Object(row));
                        count += 1;
                    }
                    crate::control::ensure_rows(&self.client, Some(self.attempt), rows).await?;
                    crate::control::hold(
                        &self.client,
                        Some(self.attempt),
                        RecordId::new("compiler_view", view.identity.hex()),
                        nodes,
                    )
                    .await?;
                }
                if count != view.rows {
                    return Err(ModelError::Conflict("exact view cardinality"));
                }
                let owner = RecordId::new("compiler_view", view.identity.hex());
                let contributors = self.view_owners(view)?;
                let edges = contributors
                    .into_iter()
                    .map(|id| {
                        (
                            owner.clone(),
                            RecordId::new("compiler_contribution", id.hex()),
                        )
                    })
                    .chain(std::iter::once((
                        RecordId::new("native_attempt", self.attempt.hex()),
                        owner.clone(),
                    )));
                for (owner, object) in edges {
                    let weight = crate::loader::native_bytes(&Value::RecordId(owner.clone()))
                        .saturating_add(crate::loader::native_bytes(&Value::RecordId(
                            object.clone(),
                        )));
                    if !references.is_empty()
                        && (references.len() == crate::loader::NATIVE_WINDOW_ROWS
                            || reference_bytes.saturating_add(weight)
                                > d::resources::TRANSFER_BYTES)
                    {
                        crate::control::hold_many(
                            &self.client,
                            Some(self.attempt),
                            std::mem::take(&mut references),
                        )
                        .await?;
                        reference_bytes = 0;
                        reference_charge.try_resize(0)?;
                    }
                    reference_bytes = reference_bytes.saturating_add(weight);
                    reference_charge.try_resize(reference_bytes.saturating_mul(4))?;
                    references.push((owner, object));
                }
            }
            crate::control::hold_many(&self.client, Some(self.attempt), references).await?;
            reference_charge.try_resize(0)?;
            for view in window {
                self.remember_view(view)?;
            }
            start = end;
        }
        Ok(())
    }

    /// Only admission of the complete current semantic boundary makes retained products eligible.
    pub async fn mark_attempt_admitted(self: &Arc<Self>) -> Result<(), ModelError> {
        let mut revision = self
            .client
            .query("SELECT VALUE revision FROM $attempt")
            .bind((
                "attempt",
                RecordId::new("native_attempt", self.attempt.hex()),
            ))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let revision: Vec<i64> = revision.take(0).map_err(ModelError::codec)?;
        let [revision] = revision.as_slice() else {
            return Err(ModelError::Conflict("native admission attempt"));
        };
        let bindings = self.bindings().await?;
        let inventory = binding_inventory_identity(&bindings)?;
        let mut bindings = Variables::new();
        bindings.insert(
            "attempt",
            RecordId::new("native_attempt", self.attempt.hex()),
        );
        bindings.insert("view", inventory.hex());
        bindings.insert("revision", *revision);
        crate::control::effect(&self.client,None,"LET $owner=SELECT * FROM ONLY $attempt FOR UPDATE; IF $owner.state!='open' OR $owner.revision!=$revision { THROW 'native admission inventory raced'; }; IF $owner.admitted AND $owner.admitted_view!=$view { THROW 'native admission inventory changed'; }; UPDATE $attempt SET admitted=true,admitted_view=$view,revision+=1 RETURN NONE",bindings).await
    }
    pub async fn retain_product_identity(
        self: &Arc<Self>,
        request: ContentHash,
        contribution: ContentHash,
    ) -> Result<(), ModelError> {
        self.completed_contribution(contribution).await?;
        let mut row = Object::new();
        row.insert(
            "id",
            RecordId::new(
                "native_product",
                format!("{}_{}", request.hex(), self.attempt.hex()),
            ),
        );
        row.insert("request", request.hex());
        row.insert(
            "contribution",
            RecordId::new("compiler_contribution", contribution.hex()),
        );
        let mut bindings = Variables::new();
        bindings.insert("row", row);
        crate::control::effect(
            &self.client,
            Some(self.attempt),
            "UPSERT $row.id CONTENT $row RETURN NONE",
            bindings,
        )
        .await?;
        crate::control::hold(
            &self.client,
            Some(self.attempt),
            RecordId::new(
                "native_product",
                format!("{}_{}", request.hex(), self.attempt.hex()),
            ),
            vec![RecordId::new("compiler_contribution", contribution.hex())],
        )
        .await
    }
    pub async fn attach_retained_product(
        self: &Arc<Self>,
        request: ContentHash,
        spec: &ContributionSpec,
    ) -> Result<
        Option<(
            ContentHash,
            BTreeMap<String, CompletedView>,
            CompletedContribution,
        )>,
        ModelError,
    > {
        let mut lookup = Variables::new();
        lookup.insert("request", request.hex());
        let mut response=self.client.query("SELECT contribution.* AS stored FROM native_product WHERE request=$request AND contribution.completed=true AND contribution.attempt.admitted=true ORDER BY id LIMIT 1").bind(lookup).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let source = value_object(row)
            .and_then(|row| row.get("stored"))
            .ok_or(ModelError::Schema("retained admitted contribution"))?;
        validate_state_row("compiler_contribution", source)?;
        let descriptor: CompletedContribution = decode_descriptor(source)?;
        if &descriptor.spec != spec {
            return Ok(None);
        }
        self.validate_inputs(&spec.inputs).await?;
        let physical = attempt_contribution(self.attempt, spec.identity()?);
        let source_id = value_object(source)
            .and_then(|row| row.get("id"))
            .cloned()
            .ok_or(ModelError::Schema("retained contribution identity"))?;
        let Value::RecordId(source_record) = &source_id else {
            return Err(ModelError::Schema("retained contribution record"));
        };
        crate::control::hold(
            &self.client,
            Some(self.attempt),
            RecordId::new("native_attempt", self.attempt.hex()),
            vec![source_record.clone()],
        )
        .await?;
        let mut bindings = Variables::new();
        bindings.insert("source", source_id.clone());
        bindings.insert(
            "target",
            RecordId::new("compiler_contribution", physical.hex()),
        );
        bindings.insert(
            "spec",
            Bytes::from(serde_json::to_vec(spec).map_err(ModelError::codec)?),
        );
        // Attach the immutable descriptor and membership pointers, never canonical row replay.
        crate::control::effect(&self.client,Some(self.attempt),"LET $source=SELECT * FROM ONLY $source FOR UPDATE; LET $source_attempt_id=$source.attempt; LET $source_attempt=SELECT * FROM ONLY $source_attempt_id FOR UPDATE; IF !$source.completed OR !$source_attempt.admitted OR $source.spec!=$spec { THROW 'retained admission changed'; }; CREATE $target CONTENT object::extend($source,{id:$target,attempt:$__attempt,completed:false}) RETURN NONE",bindings).await?;
        self.hold_contribution_inputs(physical, spec).await?;
        // Normalize copied memberships to the deterministic exact-owner identities in bounded
        // windows. This transfer copies compact pointers only; payloads remain shared.
        let mut select = Variables::new();
        select.insert("source", source_id);
        let mut members=NativeRows::new(self.client.query("SELECT relation,semantic_key,node,content FROM compiler_membership WHERE contribution=$source ORDER BY relation,semantic_key").bind(select).stream_items().map_err(ModelError::codec)?,1)?;
        let mut pending = Vec::new();
        while let Some(value) = members.next().await? {
            let mut row = value_object(&value)
                .ok_or(ModelError::Schema("retained membership"))?
                .clone();
            let Some(Value::String(relation)) = row.get("relation") else {
                return Err(ModelError::Schema("retained membership relation"));
            };
            let Some(Value::String(key)) = row.get("semantic_key") else {
                return Err(ModelError::Schema("retained membership key"));
            };
            row.insert(
                "id",
                membership_id(physical, relation, &decode_nominal_key(key)?),
            );
            row.insert(
                "contribution",
                RecordId::new("compiler_contribution", physical.hex()),
            );
            pending.push(Value::Object(row));
            if pending.len() == crate::loader::NATIVE_WINDOW_ROWS {
                self.retain_attached_members(physical, std::mem::take(&mut pending))
                    .await?;
            }
        }
        self.retain_attached_members(physical, pending).await?;
        let mut finish = Variables::new();
        finish.insert(
            "target",
            RecordId::new("compiler_contribution", physical.hex()),
        );
        crate::control::effect(
            &self.client,
            Some(self.attempt),
            "UPDATE $target SET completed=true RETURN NONE",
            finish,
        )
        .await?;
        self.specifications
            .lock()
            .map_err(|_| ModelError::Conflict("native contribution owner"))?
            .insert(physical, (spec.clone(), true));
        self.remember_contributor(descriptor.identity()?, physical)?;
        let mut views = BTreeMap::new();
        for (relation, output) in &descriptor.outputs {
            let view = CompletedView::new(
                relation.clone(),
                std::collections::BTreeSet::from([descriptor.identity()?]),
                output.rows,
            )?;
            views.insert(relation.clone(), view);
        }
        self.registered_or_install_views(&views.values().collect::<Vec<_>>())
            .await?;
        Ok(Some((physical, views, descriptor)))
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
    async fn retain_attached_members(
        &self,
        contribution: ContentHash,
        rows: Vec<Value>,
    ) -> Result<(), ModelError> {
        let owner = RecordId::new("compiler_contribution", contribution.hex());
        let mut references = Vec::new();
        for row in &rows {
            let row = value_object(row).ok_or(ModelError::Schema("attached member"))?;
            for field in ["id", "node"] {
                if let Some(Value::RecordId(id)) = row.get(field) {
                    references.push((owner.clone(), id.clone()));
                }
            }
        }
        crate::control::ensure_rows_owned(
            &self.client,
            Some(self.attempt),
            Some(contribution),
            rows,
        )
        .await?;
        crate::control::hold_many(&self.client, Some(self.attempt), references).await
    }
    /// Producers identify their specification before registration. The native scope binds
    /// that identity to this attempt's physical contribution, just like begin_contribution.
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
        let database = config.database.clone();
        let client = check_installation(config).await?;
        let attempt = crate::control::fresh_identity("compiler-attempt")?;
        crate::control::begin_attempt(&client, attempt, config.service_generation).await?;
        Ok(Arc::new(Self {
            client,
            database,
            attempt,
            generation: config.service_generation,
            published_bindings: Mutex::new(None),
            namespace: config.namespace.clone(),
            endpoint: Some(config.endpoint.clone()),
            specifications: Mutex::default(),
            known_views: Mutex::default(),
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
        Arc::new(Self {
            client,
            database,
            attempt: crate::control::fresh_identity("detached-native-attempt")
                .expect("system clock"),
            generation: ContentHash::of(b"detached-unadmitted"),
            published_bindings: Mutex::new(None),
            namespace,
            endpoint: None,
            specifications: Mutex::default(),
            known_views: Mutex::default(),
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
    pub async fn freeze_content(self: &Arc<Self>) -> Result<FinalizationInventory, ModelError> {
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
            Ok(FinalizationInventory {
                contributions: self.contributions_inner().await?,
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
    ) -> Result<(), ModelError> {
        let lease = self.admit_mutation("import_original_stream", true)?;
        lease.finish_with(
            Loader::for_attempt_views(self.client.clone(), self.attempt, vec![])
                .original_stream(source, content, length, input)
                .await,
        )
    }
    pub async fn contributions(self: &Arc<Self>) -> Result<Vec<CompletedContribution>, ModelError> {
        let lease = self.admit(false, "contributions", false)?;
        let result = lease.within(self.contributions_inner()).await;
        lease.finish_with(result)
    }
    async fn contributions_inner(
        self: &Arc<Self>,
    ) -> Result<Vec<CompletedContribution>, ModelError> {
        let selection = self.capture_state().await?;
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
            self.discover_graph(entities, budget)
                .await?
                .finish()
                .await?
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
        })
    }
    async fn discover_graph(
        self: &Arc<Self>,
        entities: bool,
        budget: &ResourceBudget,
    ) -> Result<AsyncPhysicalSort, ModelError> {
        self.check_failed()?;
        let table = if entities { "entity" } else { "assertion" };
        let mut sorted = AsyncPhysicalSort::new_registered(
            budget,
            self.blocking_owner()?,
            self.blocking_observer()?,
        )
        .await?;
        let _frontier_charge = budget.reserve(
            "canonical-alias-frontier",
            crate::loader::NATIVE_WINDOW_ROWS * 512,
        )?;
        let mut frontier = Vec::with_capacity(crate::loader::NATIVE_WINDOW_ROWS);
        let roots = self
            .bindings_inner()
            .await?
            .into_iter()
            .filter(|binding| binding.boundary.is_none())
            .map(|binding| RecordId::new("compiler_view", binding.view.identity.hex()))
            .collect::<Vec<_>>();
        let mut bindings = Variables::new();
        bindings.insert("views", roots);
        let mut members=self.track_rows(NativeRows::new(self.client.query("SELECT node FROM compiler_view_member WITH INDEX view_nodes WHERE view IN $views ORDER BY node").bind(bindings).stream_items().map_err(ModelError::codec)?,1)?)?;
        while let Some(member) = members.next_native().await? {
            let Some(Value::RecordId(node)) = value_object(&member).and_then(|row| row.get("node"))
            else {
                return Err(ModelError::Schema("canonical membership pointer"));
            };
            if node.table.as_str() != table {
                continue;
            }
            frontier.push(node.clone());
            if frontier.len() == crate::loader::NATIVE_WINDOW_ROWS {
                self.expand_graph_frontier(&mut sorted, &frontier, entities)
                    .await?;
                frontier.clear();
            }
        }
        if !frontier.is_empty() {
            self.expand_graph_frontier(&mut sorted, &frontier, entities)
                .await?;
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
                                let entities =
                                    store.discover_graph(true, &budget).await?.prepare().await?;
                                let assertions = store
                                    .discover_graph(false, &budget)
                                    .await?
                                    .prepare()
                                    .await?;
                                Ok(PreparedCanonical {
                                    entities,
                                    assertions,
                                })
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
    async fn expand_graph_frontier(
        self: &Arc<Self>,
        sorted: &mut AsyncPhysicalSort,
        frontier: &[RecordId],
        entities: bool,
    ) -> Result<(), ModelError> {
        let mut bindings = Variables::new();
        bindings.insert("nodes", frontier.to_vec());
        let mut rows = self.track_rows(NativeRows::new(
            self.client
                .query("SELECT id,anchor FROM $nodes")
                .bind(bindings)
                .stream_items()
                .map_err(ModelError::codec)?,
            1,
        )?)?;
        while let Some(value) = rows.next_native().await? {
            let object =
                value_object(&value).ok_or(ModelError::Schema("canonical payload anchor"))?;
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
        if entities {
            let mut bindings = Variables::new();
            bindings.insert("sources", frontier.to_vec());
            // Bounded exact sources expand one hop. Alias targets never feed this frontier.
            let mut aliases=self.track_rows(NativeRows::new(self.client.query("SELECT VALUE target FROM compiler_alias WITH INDEX alias_source WHERE source IN $sources").bind(bindings).stream_items().map_err(ModelError::codec)?,1)?)?;
            while let Some(alias) = aliases.next_native().await? {
                let Value::RecordId(alias) = alias else {
                    return Err(ModelError::Schema("canonical alias pointer"));
                };
                if alias.table.as_str() != "entity" {
                    return Err(ModelError::Conflict("canonical alias family"));
                }
                let mut variables = Variables::new();
                variables.insert("node", alias.clone());
                let mut response = self
                    .client
                    .query("SELECT VALUE anchor FROM $node")
                    .bind(variables)
                    .await
                    .map_err(ModelError::codec)?
                    .check()
                    .map_err(ModelError::codec)?;
                let anchors: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
                let [anchor] = anchors.as_slice() else {
                    return Err(ModelError::Schema("canonical alias anchor"));
                };
                let mut pointer = Object::new();
                pointer.insert("id", anchor.clone());
                pointer.insert("payload", alias);
                sorted.push(Value::Object(pointer)).await?;
            }
        }
        Ok(())
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
        relation: &Relation,
        fields: String,
        predicate: String,
        bindings: Variables,
        preparation: Vec<String>,
        budget: &ResourceBudget,
    ) -> Result<CompilerRows, ModelError> {
        let transfer = budget.reserve(
            "compiler-selected-window",
            view.contributions
                .len()
                .saturating_mul(64)
                .saturating_add(crate::loader::NATIVE_WINDOW_ROWS * 2048),
        )?;
        let owners = self.view_owners(view)?;
        let lease = self.retained_scan()?;
        let selection = SelectedRows {
            keys,
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
        let mut completion = self.drain_report().await;
        // Abandon releases this attempt only; shared immutable completed data survives.
        completion.step(
            "native attempt abandonment",
            crate::control::close_attempt(&self.client, self.attempt, "abandoned").await,
        );
        completion.step(
            "native session invalidation",
            self.client
                .invalidate()
                .await
                .map_err(|error| ModelError::Cause(Box::new(error))),
        );
        d::completion::complete(Ok(()), completion)
    }
    /// Local terminality and remote acknowledgement are independent facts. The admission owner
    /// remains registered when this future is interrupted, allowing a later drain to finish.
    pub async fn drain_report(&self) -> d::completion::Completion {
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
            for failure in &state.failures {
                completion.step(
                    failure.operation,
                    Err(ModelError::SharedCause(failure.error.clone())),
                );
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
        let lease = self.admit_mutation("begin_contribution", true)?;
        let result = lease.within(self.begin_contribution_inner(spec)).await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
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
            if specifications.insert(id, (spec.clone(), false)).is_some() {
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
            crate::control::effect(&self.client,Some(self.attempt),"IF $__owner.admitted { THROW 'native attempt canonical content admitted'; }; CREATE $row.id CONTENT $row RETURN NONE",b).await?;
            self.hold_contribution_inputs(id,&spec).await?;
            Ok::<(), ModelError>(())
        }
        .await;
        if let Err(e) = write {
            self.fail();
            return Err(e);
        }
        Ok(id)
    }
    pub async fn write_batch(
        self: &Arc<Self>,
        contribution: &ContentHash,
        relation: &Relation,
        batch: &RecordBatch,
    ) -> Result<(), ModelError> {
        self.boxed_write_batch(contribution, relation, batch).await
    }
    // Keep all declared-record lowering and its poll frame inside the native library.
    #[inline(never)]
    fn boxed_write_batch<'a>(
        self: &'a Arc<Self>,
        contribution: &'a ContentHash,
        relation: &'a Relation,
        batch: &'a RecordBatch,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        async move {
            let lease = self.admit_mutation("write_batch", true)?;
            let result = WRITING_CONTRIBUTION
                .scope(
                    *contribution,
                    lease.within(self.write_batch_inner(contribution, relation, batch)),
                )
                .await;
            let result = lease.finish_with(result);
            if result.is_err() {
                self.fail();
            }
            result
        }
        .boxed()
    }
    async fn write_batch_inner(
        self: &Arc<Self>,
        contribution: &ContentHash,
        relation: &Relation,
        batch: &RecordBatch,
    ) -> Result<(), ModelError> {
        self.check_failed()?;
        {
            let specifications = self
                .specifications
                .lock()
                .map_err(|_| ModelError::Conflict("native contribution owner"))?;
            let (spec, completed) = specifications
                .get(contribution)
                .ok_or(ModelError::Conflict("unknown producer contribution"))?;
            if *completed {
                self.fail();
                return Err(ModelError::Conflict("write after contribution completion"));
            }
            if !spec.outputs.contains(relation.name()) {
                self.fail();
                return Err(ModelError::Conflict("undeclared producer output"));
            }
        }
        let adapter = crate::adapter::select(relation.name())?;
        if adapter.relation.type_id() != relation.type_id()
            || adapter.relation.schema() != relation.schema()
        {
            return Err(ModelError::Schema("native relation authority"));
        }
        let batch = adapter.relation.canonical(batch)?;
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("native nominal ID column"))?;
        let mut bodies = crate::codec::batch_bodies(relation, &batch)?;
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
                .into_iter()
                .map(|row| {
                    let mut header = Object::new();
                    header.insert(
                        "id",
                        RecordId::new("original", d::graph::EntityId::of(row.id()).0.hex()),
                    );
                    header.insert("byte_len", row.byte_len);
                    header.insert("content", row.content.hex());
                    Value::Object(header)
                })
                .collect()
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
        for row in &original_chunks {
            self.persist_original_chunk(row).await?;
        }
        self.ensure_immutable_rows("original", original_headers, "original same-key payload")
            .await?;
        self.ensure_physical_rows(nodes.into_values().collect())
            .await?;
        self.ensure_immutable_rows("compiler_alias", aliases, "native alias same-key payload")
            .await?;
        // Repeated identical writes within this contribution are idempotent, without hiding
        // semantic uniqueness failures. Membership contains no separately editable payload.
        self.ensure_immutable_rows(
            "compiler_membership",
            members.into_values().collect(),
            "native membership same-key payload",
        )
        .await?;
        Ok(())
    }

    async fn ensure_physical_rows(self: &Arc<Self>, rows: Vec<Value>) -> Result<(), ModelError> {
        for window in crate::loader::NativeWindows::new(rows) {
            let rows = window?;
            let anchors = rows
                .iter()
                .filter_map(|row| value_object(row).and_then(|row| row.get("anchor")))
                .filter_map(|value| {
                    if let Value::RecordId(id) = value {
                        let mut anchor = Object::new();
                        anchor.insert("id", id.clone());
                        anchor.insert(
                            "family",
                            id.table.as_str().trim_end_matches("_anchor").to_string(),
                        );
                        if let RecordIdKey::String(key) = &id.key {
                            anchor.insert("nominal", key.clone());
                            Some(Value::Object(anchor))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            crate::control::ensure_rows_owned(
                &self.client,
                Some(self.attempt),
                WRITING_CONTRIBUTION.try_with(|id| *id).ok(),
                anchors,
            )
            .await?;
            crate::control::ensure_rows_owned(
                &self.client,
                Some(self.attempt),
                WRITING_CONTRIBUTION.try_with(|id| *id).ok(),
                rows.clone(),
            )
            .await?;
            let ids = rows
                .iter()
                .filter_map(|row| {
                    value_object(row)
                        .and_then(|row| row.get("id"))
                        .and_then(|id| {
                            if let Value::RecordId(id) = id {
                                Some(id.clone())
                            } else {
                                None
                            }
                        })
                })
                .collect::<Vec<_>>();
            let owner = WRITING_CONTRIBUTION
                .try_with(|id| RecordId::new("compiler_contribution", id.hex()))
                .unwrap_or_else(|_| RecordId::new("native_attempt", self.attempt.hex()));
            crate::control::hold(&self.client, Some(self.attempt), owner, ids).await?;
            let references = rows
                .iter()
                .filter_map(value_object)
                .flat_map(|row| {
                    let owner = row.get("id").and_then(|id| {
                        if let Value::RecordId(id) = id {
                            Some(id.clone())
                        } else {
                            None
                        }
                    });
                    ["anchor", "in", "out", "source"]
                        .into_iter()
                        .filter_map(move |field| {
                            let owner = owner.clone()?;
                            let Value::RecordId(object) = row.get(field)? else {
                                return None;
                            };
                            Some((owner, object.clone()))
                        })
                })
                .collect();
            crate::control::hold_many(&self.client, Some(self.attempt), references).await?;
        }
        Ok(())
    }

    async fn persist_original_chunk(
        self: &Arc<Self>,
        row: &d::artifact::ArtifactChunk,
    ) -> Result<(), ModelError> {
        let source = RecordId::new("original", d::graph::EntityId::of(row.artifact).0.hex());
        let base = u64::try_from(row.ordinal)
            .map_err(ModelError::codec)?
            .checked_mul(d::artifact::ARTIFACT_CHUNK_BYTES as u64)
            .ok_or(ModelError::Schema("original chunk start"))?;
        let rows = row
            .body
            .0
            .chunks(65536)
            .enumerate()
            .map(|(index, bytes)| {
                let start = base + (index * 65536) as u64;
                let mut chunk = Object::new();
                chunk.insert(
                    "id",
                    RecordId::new(
                        "original_chunk",
                        format!("{}_{}", d::graph::EntityId::of(row.artifact).0.hex(), start),
                    ),
                );
                chunk.insert("source", source.clone());
                chunk.insert("start", start);
                chunk.insert("bytes", Bytes::from(bytes.to_vec()));
                chunk.insert("content", ContentHash::of(bytes).hex());
                Value::Object(chunk)
            })
            .collect();
        self.ensure_immutable_rows("original_chunk", rows, "original same-key payload")
            .await
    }
    // Compare only this bounded batch's complete immutable rows, then insert new rows in one
    // statement. Identical replay is admitted without per-row transactions or silent conflicts.
    async fn ensure_immutable_rows(
        self: &Arc<Self>,
        table: &str,
        rows: Vec<Value>,
        conflict: &'static str,
    ) -> Result<(), ModelError> {
        let _ = (table, conflict);
        for window in crate::loader::NativeWindows::new(rows) {
            let rows = window?;
            crate::control::ensure_rows_owned(
                &self.client,
                Some(self.attempt),
                WRITING_CONTRIBUTION.try_with(|id| *id).ok(),
                rows.clone(),
            )
            .await?;
            let owner = WRITING_CONTRIBUTION
                .try_with(|id| RecordId::new("compiler_contribution", id.hex()))
                .unwrap_or_else(|_| RecordId::new("native_attempt", self.attempt.hex()));
            let ids = rows
                .iter()
                .filter_map(|row| value_object(row).and_then(|row| row.get("id")))
                .filter_map(|value| {
                    if let Value::RecordId(id) = value {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();
            crate::control::hold(&self.client, Some(self.attempt), owner, ids).await?;
            if table == "original_chunk" {
                let references = rows
                    .iter()
                    .filter_map(value_object)
                    .filter_map(|row| match (row.get("id"), row.get("source")) {
                        (Some(Value::RecordId(id)), Some(Value::RecordId(source))) => {
                            Some((source.clone(), id.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                crate::control::hold_many(&self.client, Some(self.attempt), references).await?;
            }
        }
        Ok(())
    }

    pub async fn complete_contribution(
        self: &Arc<Self>,
        id: ContentHash,
        outcome: ProviderOutcome,
        outputs: &[Relation],
        previous: &BTreeMap<String, CompletedView>,
    ) -> Result<BTreeMap<String, CompletedView>, ModelError> {
        // Independent/manual clients retain the conservative drainage contract. Ordinary
        // producers use the required native-owned producing scope below.
        self.wait_scans().await?;
        let lease = self.admit_mutation("complete_contribution", true)?;
        let result = lease
            .within(self.complete_contribution_inner(id, outcome, outputs, previous))
            .await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
        result
    }
    pub async fn complete_contribution_scoped(
        self: &Arc<Self>,
        scope: &ProducingScope,
        id: ContentHash,
        outcome: ProviderOutcome,
        outputs: &[Relation],
        previous: &BTreeMap<String, CompletedView>,
    ) -> Result<BTreeMap<String, CompletedView>, ModelError> {
        if !scope.belongs_to(self) || scope.contribution != id {
            return Err(ModelError::Conflict(
                "native producing contribution binding",
            ));
        }
        scope.close_and_wait().await?;
        let lease = self.admit_mutation("complete_contribution", true)?;
        let result = lease
            .within(self.complete_contribution_inner(id, outcome, outputs, previous))
            .await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
        result
    }
    async fn complete_contribution_inner(
        self: &Arc<Self>,
        id: ContentHash,
        outcome: ProviderOutcome,
        outputs: &[Relation],
        previous: &BTreeMap<String, CompletedView>,
    ) -> Result<BTreeMap<String, CompletedView>, ModelError> {
        self.check_failed()?;
        for relation in outputs {
            if let Some(prior) = previous.get(relation.name()) {
                if prior.relation != relation.name() {
                    return Err(ModelError::Conflict("previous completed view relation"));
                }
                self.registered_view(prior).await?;
            }
        }
        let (spec, completed) = self
            .specifications
            .lock()
            .map_err(|_| ModelError::Conflict("native contribution owner"))?
            .get(&id)
            .cloned()
            .ok_or(ModelError::Conflict("unknown native contribution"))?;
        self.hold_contribution_inputs(id, &spec).await?;
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
                let budget = ResourceBudget::fixed(d::resources::MAX_ROW_BYTES.saturating_mul(4))?;
                let prepared = self.prepare_memberships(&candidate, &budget).await?;
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
            self.registered_or_install_views(&views.values().collect::<Vec<_>>())
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
        let descriptor = CompletedContribution {
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
        crate::control::effect(&self.client,Some(self.attempt),"LET $owner=SELECT * FROM ONLY $id FOR UPDATE; IF $owner.completed OR $owner.attempt != $__attempt { THROW 'native contribution completion fenced'; }; UPDATE $id SET descriptor=$descriptor,logical=$logical,outcome=$outcome,completed=true RETURN NONE",b).await?;
        self.specifications
            .lock()
            .map_err(|_| ModelError::Conflict("native contribution owner"))?
            .get_mut(&id)
            .ok_or(ModelError::Conflict("unknown native contribution"))?
            .1 = true;
        self.remember_contributor(logical, id)?;
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
        self.registered_or_install_views(&views.values().collect::<Vec<_>>())
            .await?;
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
    async fn contribution_view_inner(
        self: &Arc<Self>,
        id: ContentHash,
        relation: &Relation,
    ) -> Result<CompletedView, ModelError> {
        self.check_failed()?;
        let mut bindings = Variables::new();
        bindings.insert("id", RecordId::new("compiler_contribution", id.hex()));
        let mut response = self
            .client
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
                .client
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
    pub async fn verify_state(self: &Arc<Self>) -> Result<(), ModelError> {
        let lease = self.admit(false, "verify_state", false)?;
        let result = lease.within(self.verify_state_inner()).await;
        lease.finish_with(result)
    }
    async fn verify_state_inner(self: &Arc<Self>) -> Result<(), ModelError> {
        let selection = self.capture_state().await?;
        // Cold audit/import must reconcile the fixed non-graph backing itself, before
        // comparing membership claims. Ordinary completed-view reads carry their validity.
        let mut backing = self.track_rows(
            self.state_source("compiler_record", &selection)?
                .with_row_bytes(d::resources::MAX_ROW_BYTES),
        )?;
        let mut pending = Vec::new();
        let mut pending_bytes = 0;
        while let Some(row) = backing.next().await? {
            if let Some(original) = validate_state_row("compiler_record", &row)? {
                self.verify_original_backing(&original).await?;
            }
            admit_backing_row(&mut pending, &mut pending_bytes, row)?;
        }
        validate_backing_batch(&pending)?;
        let mut rows=self.track_rows(NativeRows::new(self.client.query("SELECT VALUE id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $state_owners AND (node.semantic_type IS NONE OR node.semantic_type != relation OR node.semantic_key != semantic_key OR node.content != content OR contribution.completed != true) LIMIT 1").bind(selection.variables.clone()).stream_items().map_err(ModelError::codec)?,1)?)?;
        if rows.next().await?.is_some() {
            return Err(ModelError::Conflict(
                "completed membership backing/visibility",
            ));
        }
        let mut descriptors =
            self.track_rows(self.state_source("compiler_contribution", &selection)?)?;
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
            let mut members=self.track_rows(NativeRows::new(self.client.query("SELECT id,contribution,relation,semantic_key,node,content FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$contribution ORDER BY relation,semantic_key").bind(variables).stream_items().map_err(ModelError::codec)?,1)?)?;
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
        let audit_budget = ResourceBudget::fixed(d::resources::MAX_ROW_BYTES.saturating_mul(4))?;
        for view in self.views_inner().await? {
            let relation = &crate::adapter::select(&view.relation)?.relation;
            let columns = vec!["id".to_string()];
            let mut rows = self
                .scan_rows_inner(&view, relation, Some(&columns), None, &audit_budget)
                .await?;
            let mut count = 0u64;
            while rows.next().await?.is_some() {
                count += 1;
            }
            if count != view.rows {
                return Err(ModelError::Conflict("completed view cardinality"));
            }
        }
        for binding in self.bindings_inner().await? {
            self.registered_view(&binding.view).await?;
        }
        Ok(())
    }
    async fn verify_original_backing(&self, original: &OriginalBacking) -> Result<(), ModelError> {
        let source = d::graph::EntityId::of(original.artifact);
        let mut bindings = Variables::new();
        bindings.insert("id", RecordId::new("original", source.0.hex()));
        let mut response = self
            .client
            .query("SELECT VALUE byte_len FROM $id")
            .bind(bindings)
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let lengths: Vec<i64> = response.take(0).map_err(ModelError::codec)?;
        let [length] = lengths.as_slice() else {
            return Err(ModelError::Conflict("compiler original header"));
        };
        let remaining = u64::try_from(*length)
            .map_err(ModelError::codec)?
            .checked_sub(original.start)
            .ok_or(ModelError::Conflict("compiler original coverage"))?;
        if original.len as u64 != remaining.min(d::artifact::ARTIFACT_CHUNK_BYTES as u64) {
            return Err(ModelError::Conflict("compiler original coverage"));
        }
        // Reuse the independently checked physical-range reader; retain only one bounded
        // page, and hash it without reconstructing or storing a second raw chunk body.
        let reader = crate::NativeReader::private(self.client.clone());
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
            .client
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
        let logical = views
            .iter()
            .flat_map(|view| view.contributions.iter().copied())
            .collect::<std::collections::BTreeSet<_>>();
        if logical.is_empty() {
            return Ok(());
        }
        let mut bindings = Variables::new();
        bindings.insert(
            "logical",
            logical.iter().map(ContentHash::hex).collect::<Vec<_>>(),
        );
        // Repeated compatible attempts may retain many physical copies of one logical
        // descriptor. Validate every copy without materializing that entire result set.
        let mut rows=self.track_rows(NativeRows::new(self.client.query(
            "SELECT * FROM compiler_contribution WHERE completed=true AND logical IN $logical",
        ).bind(bindings).stream_items().map_err(ModelError::codec)?,1)?)?;
        let mut completed = BTreeMap::new();
        while let Some(row) = rows.next().await? {
            validate_state_row("compiler_contribution", &row)?;
            let descriptor = decode_descriptor::<CompletedContribution>(&row)?;
            let logical = descriptor.identity()?;
            let physical = value_object(&row)
                .and_then(|row| row.get("id"))
                .and_then(|id| {
                    if let Value::RecordId(id) = id {
                        if let RecordIdKey::String(key) = &id.key {
                            hex::decode(key)
                                .ok()
                                .and_then(|bytes| bytes.try_into().ok())
                                .map(ContentHash)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .ok_or(ModelError::Schema("completed contributor identity"))?;
            if let Some(old) = completed.get(&logical) {
                if old != &descriptor {
                    return Err(ModelError::Conflict(
                        "completed contributor identity collision",
                    ));
                }
            } else {
                completed.insert(logical, descriptor);
            }
            self.known_contributors
                .lock()
                .map_err(|_| ModelError::Conflict("native contributor owner"))?
                .entry(logical)
                .or_insert(physical);
        }
        if completed.len() != logical.len() || logical.iter().any(|id| !completed.contains_key(id))
        {
            return Err(ModelError::Conflict(
                "completed view contribution membership",
            ));
        }
        for view in views {
            for id in &view.contributions {
                if !completed[id].outputs.contains_key(&view.relation) {
                    return Err(ModelError::Conflict("completed view output ownership"));
                }
            }
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
        let known = self
            .known_contributors
            .lock()
            .map_err(|_| ModelError::Conflict("native contributor owner"))?;
        let mut owners = view
            .contributions
            .iter()
            .map(|logical| {
                known
                    .get(logical)
                    .copied()
                    .ok_or(ModelError::Conflict("missing verified contributor"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        owners.sort_unstable();
        if owners.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ModelError::Conflict(
                "duplicate physical completed contributor",
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
        crate::control::effect(&self.client,Some(self.attempt),"IF $__owner.admitted { THROW 'native binding inventory admitted'; }; UPSERT $row.id CONTENT $row RETURN NONE; UPDATE $__attempt SET revision+=1 RETURN NONE",b).await?;
        crate::control::hold(
            &self.client,
            Some(self.attempt),
            RecordId::new("native_attempt", self.attempt.hex()),
            vec![RecordId::new("compiler_view", binding.view.identity.hex())],
        )
        .await?;
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
            self.client
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
    pub async fn views(self: &Arc<Self>) -> Result<Vec<CompletedView>, ModelError> {
        let lease = self.admit(false, "views", false)?;
        let result = lease.within(self.views_inner()).await;
        lease.finish_with(result)
    }
    async fn views_inner(self: &Arc<Self>) -> Result<Vec<CompletedView>, ModelError> {
        Ok(self.state_closure().await?.0.into_values().collect())
    }
    pub async fn completed_state(self: &Arc<Self>) -> Result<CompletedStateIdentity, ModelError> {
        let lease = self.admit(false, "completed_state", false)?;
        let result = lease.within(self.completed_state_inner()).await;
        lease.finish_with(result)
    }
    pub async fn completed_state_after_closure(
        self: &Arc<Self>,
        parent: &OperationLease,
    ) -> Result<CompletedStateIdentity, ModelError> {
        // The live borrowed parent covers all awaits. Public labels cannot spoof seal origin,
        // and a guard for a different attempt provides no descendant admission here.
        if !parent.finalization || parent.finished || !Arc::ptr_eq(&parent.owner, &self.admission) {
            return Err(ModelError::Conflict("native finalization parent"));
        }
        parent.within(self.completed_state_inner()).await
    }
    async fn completed_state_inner(self: &Arc<Self>) -> Result<CompletedStateIdentity, ModelError> {
        self.state_identity_inner(true).await
    }
    // Import has just independently validated the actual stored type slices. Its checksum
    // pass still checks every framing/canonical row, without repeating model reconstruction.
    async fn state_identity_inner(
        self: &Arc<Self>,
        validate_backing: bool,
    ) -> Result<CompletedStateIdentity, ModelError> {
        self.wait_scans().await?;
        let selection = self.capture_state().await?;
        self.state_identity_selected(validate_backing, &selection)
            .await
    }
    async fn state_identity_selected(
        self: &Arc<Self>,
        validate_backing: bool,
        selection: &StateSelection,
    ) -> Result<CompletedStateIdentity, ModelError> {
        let mut sink = KeySink::new("native-completed-state/v3");
        let mut counts = [0u64; 6];
        for (i, table) in STATE_TABLES.iter().enumerate() {
            table.to_string().encode(&mut sink);
            let mut rows = self.portable_state_rows(table, selection).await?;
            let mut pending = Vec::new();
            let mut pending_bytes = 0;
            loop {
                let window = rows.next_batch(1).await?;
                let Some(row) = window.into_iter().next() else {
                    break;
                };
                validate_state_row(table, &row)?;
                sink.part(
                    b"row",
                    &serde_json::to_vec(&row).map_err(ModelError::codec)?,
                );
                counts[i] = counts[i]
                    .checked_add(1)
                    .ok_or(ModelError::Schema("completed state row count"))?;
                if validate_backing && *table == "compiler_record" {
                    admit_backing_row(&mut pending, &mut pending_bytes, row)?;
                }
            }
            if validate_backing && *table == "compiler_record" {
                validate_backing_batch(&pending)?;
            }
        }
        Ok(CompletedStateIdentity {
            format_version: STATE_FORMAT_VERSION,
            contributions: counts[0],
            memberships: counts[1],
            backing_rows: counts[3],
            content: sink.finish(),
        })
    }
    /// Explicit detached transport only. Ordinary publication seals this database directly.
    pub async fn export_state(
        self: &Arc<Self>,
        path: &std::path::Path,
    ) -> Result<CompletedStateIdentity, ModelError> {
        let lease = self.admit(false, "export_state", false)?;
        let result = lease.within(self.export_state_inner(path)).await;
        lease.finish_with(result)
    }
    async fn export_state_inner(
        self: &Arc<Self>,
        path: &std::path::Path,
    ) -> Result<CompletedStateIdentity, ModelError> {
        use std::io::Write;
        self.wait_scans().await?;
        let selection = self.capture_state().await?;
        let expected = self.state_identity_selected(true, &selection).await?;
        let mut file =
            std::io::BufWriter::new(std::fs::File::create(path).map_err(ModelError::codec)?);
        serde_json::to_writer(&mut file, &CompletedStateHeader::current())
            .map_err(ModelError::codec)?;
        file.write_all(b"\n").map_err(ModelError::codec)?;
        for table in STATE_TABLES {
            let mut rows = self.portable_state_rows(table, &selection).await?;
            loop {
                let window = rows.next_batch(1).await?;
                let Some(row) = window.into_iter().next() else {
                    break;
                };
                serde_json::to_writer(
                    &mut file,
                    &StateRow {
                        table: table.into(),
                        row,
                    },
                )
                .map_err(ModelError::codec)?;
                file.write_all(b"\n").map_err(ModelError::codec)?;
            }
        }
        file.flush().map_err(ModelError::codec)?;
        file.get_ref().sync_all().map_err(ModelError::codec)?;
        Ok(expected)
    }
    pub async fn import_state(
        self: &Arc<Self>,
        path: &std::path::Path,
        expected: &CompletedStateIdentity,
    ) -> Result<(), ModelError> {
        let lease = self.admit_mutation("import_state", true)?;
        let result = lease.within(self.import_state_inner(path, expected)).await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
        result
    }
    async fn import_state_inner(
        self: &Arc<Self>,
        path: &std::path::Path,
        expected: &CompletedStateIdentity,
    ) -> Result<(), ModelError> {
        use std::io::{BufRead, Read};
        self.check_failed()?;
        expected.validate()?;
        // Each envelope owns one bounded row; only fixed generated table names are admitted.
        let mut file =
            std::io::BufReader::new(std::fs::File::open(path).map_err(ModelError::codec)?);
        let mut bytes = Vec::new();
        let mut previous: Option<(usize, String)> = None;
        let header_bytes = file
            .by_ref()
            .take(d::resources::MAX_ROW_BYTES as u64 + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(ModelError::codec)?;
        if header_bytes == 0 || header_bytes > d::resources::MAX_ROW_BYTES {
            return Err(ModelError::Schema("completed state transport header"));
        }
        CompletedStateHeader::decode(&bytes)?;
        let mut pending = Vec::new();
        let mut pending_table = None;
        let mut pending_bytes = 0usize;
        loop {
            bytes.clear();
            let read = file
                .by_ref()
                .take(d::resources::MAX_ROW_BYTES as u64 + 1)
                .read_until(b'\n', &mut bytes)
                .map_err(ModelError::codec)?;
            if read == 0 {
                break;
            }
            if read > d::resources::MAX_ROW_BYTES {
                self.fail();
                return Err(ModelError::Schema("completed state transport row bound"));
            }
            let row: StateRow = serde_json::from_slice(&bytes).map_err(ModelError::codec)?;
            let table = STATE_TABLES
                .iter()
                .position(|name| *name == row.table)
                .ok_or(ModelError::Schema("completed state transport table"))?;
            validate_state_row(&row.table, &row.row)?;
            let id = value_object(&row.row)
                .and_then(|object| object.get("id"))
                .ok_or(ModelError::Schema("completed state transport key"))?
                .to_sql();
            if previous
                .as_ref()
                .is_some_and(|previous| previous >= &(table, id.clone()))
            {
                return Err(ModelError::Conflict("completed state transport order"));
            }
            previous = Some((table, id));
            let weight = read.max(crate::loader::native_bytes(&row.row));
            if weight > d::resources::MAX_ROW_BYTES {
                return Err(ModelError::Limit {
                    owner: "completed-state-import",
                    limit: "row bytes",
                    observed: weight,
                    bound: d::resources::MAX_ROW_BYTES,
                });
            }
            if !pending.is_empty()
                && (pending_table != Some(table)
                    || pending.len() >= d::resources::TRANSFER_ROWS
                    || pending_bytes.saturating_add(weight) > d::resources::TRANSFER_BYTES)
            {
                self.insert_state_batch(
                    pending_table.expect("nonempty state batch"),
                    std::mem::take(&mut pending),
                )
                .await?;
                pending_bytes = 0;
            }
            pending_table = Some(table);
            pending_bytes += weight;
            pending.push(row.row);
            if pending_bytes >= d::resources::TRANSFER_BYTES {
                self.insert_state_batch(table, std::mem::take(&mut pending))
                    .await?;
                pending_bytes = 0;
            }
        }
        if !pending.is_empty() {
            self.insert_state_batch(pending_table.expect("nonempty state batch"), pending)
                .await?;
        }
        let imported = self
            .specifications
            .lock()
            .map_err(|_| ModelError::Conflict("import contribution ownership"))?
            .iter()
            .map(|(id, (spec, _))| (*id, spec.clone()))
            .collect::<Vec<_>>();
        // Targets may occur later in the portable table order. All inserted rows already
        // have attempt holds; establish exact dependency and payload links once present.
        for (id, spec) in &imported {
            self.hold_contribution_inputs(*id, spec).await?;
            let owner = RecordId::new("compiler_contribution", id.hex());
            let mut rows=self.track_rows(NativeRows::new(self.client.query("SELECT VALUE node FROM compiler_membership WHERE contribution=$owner ORDER BY id").bind(("owner",owner.clone())).stream_items().map_err(ModelError::codec)?,1)?)?;
            let mut refs = Vec::new();
            while let Some(value) = rows.next().await? {
                let Value::RecordId(node) = value else {
                    return Err(ModelError::Schema("import membership target"));
                };
                refs.push((owner.clone(), node));
                if refs.len() == crate::loader::NATIVE_WINDOW_ROWS {
                    crate::control::hold_many(
                        &self.client,
                        Some(self.attempt),
                        std::mem::take(&mut refs),
                    )
                    .await?;
                }
            }
            crate::control::hold_many(&self.client, Some(self.attempt), refs).await?;
        }
        let imported = imported.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
        for window in imported.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            let mut bindings = Variables::new();
            bindings.insert(
                "owners",
                window
                    .iter()
                    .map(|id| RecordId::new("compiler_contribution", id.hex()))
                    .collect::<Vec<_>>(),
            );
            crate::control::effect(
                &self.client,
                Some(self.attempt),
                "UPDATE $owners SET completed=true RETURN NONE",
                bindings,
            )
            .await?;
        }
        self.verify_state_inner().await?;
        let views = self.views_inner().await?;
        self.registered_or_install_views(&views.iter().collect::<Vec<_>>())
            .await?;
        if &self.state_identity_inner(false).await? != expected {
            self.fail();
            return Err(ModelError::Conflict("completed state transport identity"));
        }
        Ok(())
    }
    async fn insert_state_batch(
        self: &Arc<Self>,
        table: usize,
        rows: Vec<Value>,
    ) -> Result<(), ModelError> {
        let table = STATE_TABLES
            .get(table)
            .ok_or(ModelError::Schema("completed state batch table"))?;
        if *table == "compiler_record" {
            validate_backing_batch(&rows)?;
        }
        let mut rebound = Vec::with_capacity(rows.len());
        for value in rows {
            let mut row = value_object(&value)
                .ok_or(ModelError::Schema("import state object"))?
                .clone();
            if *table == "compiler_contribution" {
                let descriptor: CompletedContribution = decode_descriptor(&value)?;
                let id = attempt_contribution(self.attempt, descriptor.spec.identity()?);
                row.insert("id", RecordId::new(*table, id.hex()));
                row.insert(
                    "attempt",
                    RecordId::new("native_attempt", self.attempt.hex()),
                );
                row.insert("completed", false);
                self.specifications
                    .lock()
                    .map_err(|_| ModelError::Conflict("import contribution ownership"))?
                    .insert(id, (descriptor.spec.clone(), true));
                self.remember_contributor(descriptor.identity()?, id)?;
            } else if *table == "compiler_membership" {
                let Some(Value::RecordId(owner)) = row.get("contribution") else {
                    return Err(ModelError::Schema("import membership owner"));
                };
                let target = self
                    .specifications
                    .lock()
                    .map_err(|_| ModelError::Conflict("import contribution ownership"))?
                    .iter()
                    .find_map(|(id, (spec, _))| {
                        let portable =
                            attempt_contribution(ContentHash([0; 32]), spec.identity().ok()?);
                        if owner.key == RecordIdKey::String(portable.hex()) {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .ok_or(ModelError::Conflict("import membership owner"))?;
                let Some(Value::String(relation)) = row.get("relation") else {
                    return Err(ModelError::Schema("import membership relation"));
                };
                let Some(Value::String(key)) = row.get("semantic_key") else {
                    return Err(ModelError::Schema("import membership key"));
                };
                row.insert(
                    "id",
                    membership_id(target, relation, &decode_nominal_key(key)?),
                );
                row.insert(
                    "contribution",
                    RecordId::new("compiler_contribution", target.hex()),
                );
            } else if *table == "compiler_binding" {
                let binding: CompletedBinding = decode_descriptor(&value)?;
                row.insert(
                    "id",
                    RecordId::new(*table, attempt_binding(self.attempt, binding.key()).hex()),
                );
                row.insert(
                    "attempt",
                    RecordId::new("native_attempt", self.attempt.hex()),
                );
            }
            if *table == "compiler_view" {
                let view: CompletedView = decode_descriptor(&value)?;
                self.remember_view(&view)?;
            }
            rebound.push(Value::Object(row));
        }
        let attempt = RecordId::new("native_attempt", self.attempt.hex());
        let mut holds = Vec::new();
        for row in &rebound {
            let object = value_object(row).ok_or(ModelError::Schema("import state object"))?;
            let Some(Value::RecordId(id)) = object.get("id") else {
                return Err(ModelError::Schema("import state identity"));
            };
            // Imported contributions precede memberships and are already attempt-rooted.
            // Keep membership ownership on that contributor, including partial ingress,
            // rather than duplicating every membership as a transient attempt root.
            if *table != "compiler_membership" {
                holds.push((attempt.clone(), id.clone()));
            }
            if *table == "compiler_membership" {
                let (Some(Value::RecordId(owner)), Some(Value::RecordId(node))) =
                    (object.get("contribution"), object.get("node"))
                else {
                    return Err(ModelError::Schema("import state membership"));
                };
                holds.push((owner.clone(), id.clone()));
                let _ = node;
            }
            if *table == "compiler_alias" {
                let (Some(Value::RecordId(source)), Some(Value::RecordId(target))) =
                    (object.get("source"), object.get("target"))
                else {
                    return Err(ModelError::Schema("import state alias"));
                };
                holds.push((source.clone(), id.clone()));
                holds.push((id.clone(), target.clone()));
            }
        }
        crate::control::ensure_rows(&self.client, Some(self.attempt), rebound).await?;
        crate::control::hold_many(&self.client, Some(self.attempt), holds).await
    }
    pub async fn scan_rows(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        columns: Option<&[String]>,
        predicate: Option<NativePredicate>,
        budget: &ResourceBudget,
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
                    .scan_rows_inner(&view, &relation, columns.as_deref(), predicate, &budget)
                    .await
                    .map(|rows| rows.with_setup(Some(Arc::new(retained))))
            }),
        )
        .await
    }
    /// Read only primitive membership/content premises under an exact immutable view.
    /// These tokens do not admit payload semantics or establish a selector's completeness.
    pub async fn row_tokens(
        self: &Arc<Self>,
        view: &CompletedView,
        relation: &Relation,
        keys: &[[u8; 16]],
        budget: &ResourceBudget,
    ) -> Result<Vec<([u8; 16], ContentHash)>, ModelError> {
        let lease = self.admit(false, "row_tokens", false)?;
        let result = lease
            .within(self.row_tokens_inner(view, relation, keys, budget))
            .await;
        lease.finish_with(result)
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
        let _owners = budget.reserve(
            "native-selected-membership-owners",
            view.contributions
                .len()
                .saturating_mul(size_of::<ContentHash>()),
        )?;
        let owners = self.view_owners(view)?;
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
            .filter(|_| u64::try_from(keys.len()).unwrap_or(u64::MAX) >= view.rows)
            .map(|(future, _)| future.clone());
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
            let _window = budget.reserve(
                "native-selected-membership-window",
                owners
                    .len()
                    .saturating_mul(64)
                    .saturating_add(crate::loader::NATIVE_WINDOW_ROWS * 2048),
            )?;
            for window in keys.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let mut lookup = MembershipLookup::new(
                    self.clone(),
                    relation.name().into(),
                    window.to_vec(),
                    owners.clone(),
                );
                for candidate in lookup.collect().await?.into_values() {
                    result.push((
                        candidate.key,
                        candidate
                            .content
                            .ok_or(ModelError::Schema("verified membership content"))?,
                    ));
                }
            }
        }
        Ok(result)
    }
    async fn prepare_memberships(
        self: &Arc<Self>,
        view: &CompletedView,
        budget: &ResourceBudget,
    ) -> Result<PreparedCandidates, ModelError> {
        let binding = self.producing_binding()?;
        let identity = view.identity;
        let (future, retained) = {
            let mut preparations = self
                .membership_preparations
                .lock()
                .map_err(|_| ModelError::Conflict("native membership preparation"))?;
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
                let budget = budget.clone();
                let future = producing_future(binding, async move {
                    let _charge = budget.reserve("native-membership-preparation", view.contributions.len().saturating_mul(64).saturating_add(1024))?;
                    let owners = store.view_owners(&view)?;
                    let mut sorted = AsyncCandidateSort::new_registered(&budget, store.blocking_owner()?, store.blocking_observer()?).await?;
                    for owner in owners {
                        let mut bindings = Variables::new();
                        bindings.insert("relation", view.relation.clone());
                        bindings.insert("owner", RecordId::new("compiler_contribution", owner.hex()));
                        let stream = store.client.query("SELECT id,contribution,relation,semantic_key,node,content FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation").bind(bindings).stream_items().map_err(ModelError::codec)?;
                        let mut rows = store.track_rows(NativeRows::new(stream, 1)?)?;
                        while let Some(row) = rows.next_native().await? {
                            let candidate = decode_membership(row, &view.relation, &[owner], false)?.ok_or(ModelError::Conflict("exact view membership"))?;
                            sorted.push(candidate).await?;
                        }
                    }
                    sorted.prepare().await
                }).map(|result: Result<PreparedCandidates, ModelError>| result.map_err(Arc::new)).boxed().shared();
                preparations.insert(identity, (future, retained));
            }
            let (future, retained) = &preparations[&identity];
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
                .is_some_and(|(_, current)| Arc::ptr_eq(current, &retained))
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
    ) -> Result<CompilerRows, ModelError> {
        self.check_failed()?;
        self.registered_view(view).await?;
        self.scan_bound_rows_inner(view, relation, columns, predicate, budget)
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
    ) -> Result<CompilerRows, ModelError> {
        let owners = self.view_owners(view)?;
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
        let mut projections = Vec::new();
        for field in &fields {
            if field == "id" {
                projections.push("semantic_key AS id".to_string());
            } else if relation.name() == d::artifact::ArtifactChunk::NAME && field == "body" {
                projections.push("(SELECT start,bytes,content FROM original_chunk WHERE source=$parent.body.original AND start >= $parent.body.start AND start < $parent.body.start+$parent.body.len ORDER BY start) AS __original_chunks, body.start AS __original_start, body.len AS __original_len, body.digest AS __original_digest".to_string());
            } else if relation.fields().iter().any(|f| f.name() == field) {
                projections.push(format!("body.`{field}` AS `{field}`"));
            } else {
                return Err(ModelError::Conflict("native projection field"));
            }
        }
        if projections.is_empty() {
            projections.push("semantic_key AS __row".into());
        }
        // Exact empty demand still passes the owner, view and shape checks above. There is
        // no physical stream to hold or drain, and no fabricated transport receipt. Internal
        // SQL predicates keep their ordinary execution/error semantics.
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
            });
        }
        if let Some(mut keys) = explicit_keys {
            keys.sort_unstable();
            keys.dedup();
            return self.track_selection(
                SelectionKeys::Known(keys.into_iter()),
                view,
                relation,
                projections.join(","),
                predicate,
                b,
                preparation,
                budget,
            );
        }
        let keys = if let Some((field, values)) = atomic_field {
            let mut sorted = AsyncCandidateSort::new_registered(
                budget,
                self.blocking_owner()?,
                self.blocking_observer()?,
            )
            .await?;
            let table = crate::schema::ScopeTable::for_relation(relation.name())?.name();
            let mut bindings = Variables::new();
            bindings.insert("relation", relation.name().to_string());
            bindings.insert("values", values);
            bindings.insert(
                "owners",
                owners
                    .iter()
                    .map(|owner| RecordId::new("compiler_contribution", owner.hex()))
                    .collect::<Vec<_>>(),
            );
            let constants = crate::prepared::scope_constants("$relation", &field, "$values");
            // Nominal revisions coexist globally. Select exact physical members before
            // nominal sorting; foreign pointers cannot conflict with this view's pointer.
            // The later membership lookup still verifies each nominated backing.
            let sql = format!(
                "LET $__compiler_nodes = array::distinct(SELECT VALUE node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $owners AND relation=$relation); LET $__compiler_scope = {constants}; SELECT semantic_type AS relation,semantic_key,id AS node FROM $__compiler_nodes WHERE record::table(id)='{table}' AND scope_keys CONTAINSANY $__compiler_scope AND semantic_type=$relation"
            );
            let mut rows = self.track_rows(prepared_rows(&self.client, sql, bindings, 3)?)?;
            while let Some(row) = rows.next_native().await? {
                sorted.push(decode_candidate(row, relation.name())?).await?;
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
            relation,
            projections.join(","),
            predicate,
            b,
            preparation,
            budget,
        )
    }
    pub fn table_provider(
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

fn prepared_rows(
    client: &Arc<Surreal<Client>>,
    sql: String,
    bindings: Variables,
    statements: usize,
) -> Result<NativeRows, ModelError> {
    crate::prepared::PreparedQuery::from_sql(sql, bindings, statements, vec![statements - 1])?
        .stream(client)
}

type ReadSetupCharge = Arc<Box<dyn Reservation>>;
pub struct CompilerRows {
    descendant: Option<DescendantBinding>,
    producing: Option<ProducingBinding>,
    rows: Option<NativeRows>,
    selection: Option<Box<SelectedRows>>,
    graph: Option<Box<GraphRows>>,
    store: Arc<NativeCompilerStore>,
    lease: Option<OperationLease>,
    setup: Option<ReadSetupCharge>,
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
        if self.selection.take().is_some() | self.graph.take().is_some() {
            drop(retained.take());
            if let Some(lease) = self.lease.take() {
                lease.finish();
            }
        }
        if let Some(mut rows) = self.rows.take() {
            let store = self.store.clone();
            let lease = self.lease.take();
            // Cleanup inherits drop-time context. Rows do not retain a separate diagnostic
            // origin when a consumer drops them outside the scoped dispatcher.
            self.store.runtime.spawn(
                async move {
                    loop {
                        match rows.next().await {
                            Ok(Some(_)) => {}
                            Ok(None) => break,
                            Err(error) => {
                                store.record_background_failure(Arc::new(error));
                                if let Err(error) = rows.drain_transport().await {
                                    store.record_background_failure(Arc::new(error));
                                }
                                break;
                            }
                        }
                    }
                    drop(rows);
                    drop(retained);
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
fn membership_id(contribution: ContentHash, relation: &str, key: &[u8]) -> RecordId {
    let mut sink = KeySink::new("compiler-membership/v1");
    contribution.encode(&mut sink);
    relation.to_string().encode(&mut sink);
    sink.part(b"key", key);
    RecordId::new("compiler_membership", sink.finish().hex())
}
fn decode_nominal_key(key: &str) -> Result<[u8; 16], ModelError> {
    hex::decode(key)
        .map_err(ModelError::codec)?
        .try_into()
        .map_err(|_| ModelError::Schema("native nominal key width"))
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
                .client
                .query(sql)
                .bind(bindings)
                .stream_items()
                .map_err(ModelError::codec)?;
            self.current = Some(
                self.store
                    .track_rows(NativeRows::new(stream, 1)?)?
                    .with_setup(self.setup.clone()),
            );
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
        self.payload = Some(
            self.store
                .track_rows(
                    prepared_rows(&self.store.client, sql, bindings, statements)?
                        .with_row_bytes(64 << 20),
                )?
                .with_setup(self.setup.clone()),
        );
        Ok(())
    }
    async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        loop {
            let window_rows = crate::loader::NATIVE_WINDOW_ROWS;
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
                            self.store
                                .client
                                .query(format!("SELECT {},anchor.nominal AS __graph_order FROM $nodes ORDER BY __graph_order", self.fields))
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

const STATE_TABLES: [&str; 6] = [
    "compiler_contribution",
    "compiler_membership",
    "compiler_view",
    "compiler_record",
    "compiler_binding",
    "compiler_alias",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateRow {
    table: String,
    row: Value,
}
fn validate_state_row(table: &str, row: &Value) -> Result<Option<OriginalBacking>, ModelError> {
    let object = value_object(row).ok_or(ModelError::Schema("completed state object"))?;
    let Some(Value::RecordId(id)) = object.get("id") else {
        return Err(ModelError::Schema("completed state key"));
    };
    if id.table.as_str() != table {
        return Err(ModelError::Schema("completed state table/key"));
    }
    if table == "compiler_record" {
        return validate_compiler_backing(object, id);
    } else if table == "compiler_contribution" {
        if object.get("completed") != Some(&Value::Bool(true)) {
            return Err(ModelError::Conflict(
                "pending contribution cannot be sealed",
            ));
        }
        let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
            return Err(ModelError::Schema("completed contribution descriptor"));
        };
        let descriptor: CompletedContribution =
            serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if object.get("logical") != Some(&Value::String(descriptor.identity()?.hex()))
            || object
                .get("attempt")
                .and_then(|value| {
                    if let Value::RecordId(attempt) = value {
                        if let RecordIdKey::String(key) = &attempt.key {
                            hex::decode(key)
                                .ok()
                                .and_then(|bytes| bytes.try_into().ok())
                                .map(ContentHash)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .is_none_or(|attempt| {
                    id.key
                        != RecordIdKey::String(
                            attempt_contribution(
                                attempt,
                                descriptor.spec.identity().expect("validated specification"),
                            )
                            .hex(),
                        )
                })
        {
            return Err(ModelError::Conflict("completed contribution identity"));
        }
        let Some(Value::Bytes(spec)) = object.get("spec") else {
            return Err(ModelError::Schema("completed contribution specification"));
        };
        if serde_json::from_slice::<ContributionSpec>(spec).map_err(ModelError::codec)?
            != descriptor.spec
        {
            return Err(ModelError::Conflict("completed contribution specification"));
        }
        check_projection(object, contribution_projection(&descriptor.spec))?;
        if object.get("outcome") != Some(&Value::from_t(descriptor.outcome)) {
            return Err(ModelError::Conflict(
                "completed contribution outcome projection",
            ));
        }
    } else if table == "compiler_membership" {
        let Some(Value::RecordId(contribution)) = object.get("contribution") else {
            return Err(ModelError::Schema("membership contribution"));
        };
        let surrealdb::types::RecordIdKey::String(physical) = &contribution.key else {
            return Err(ModelError::Schema("membership physical owner"));
        };
        let physical = ContentHash(
            hex::decode(physical)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("membership owner width"))?,
        );
        let Some(Value::String(relation)) = object.get("relation") else {
            return Err(ModelError::Schema("membership relation"));
        };
        let Some(Value::String(key)) = object.get("semantic_key") else {
            return Err(ModelError::Schema("membership nominal key"));
        };
        if contribution.table.as_str() != "compiler_contribution"
            || *id != membership_id(physical, relation, &decode_nominal_key(key)?)
        {
            return Err(ModelError::Conflict(
                "compiler membership physical identity",
            ));
        }
    } else if table == "compiler_view" {
        let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
            return Err(ModelError::Schema("completed view descriptor"));
        };
        let descriptor: CompletedView = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        descriptor.validate()?;
        if id.key != surrealdb::types::RecordIdKey::String(descriptor.identity.hex()) {
            return Err(ModelError::Conflict("completed view key"));
        }
        check_projection(object, view_projection(&descriptor)?)?;
    } else if table == "compiler_binding" {
        let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
            return Err(ModelError::Schema("completed binding descriptor"));
        };
        let descriptor: CompletedBinding =
            serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        descriptor.validate()?;
        let attempt = object
            .get("attempt")
            .and_then(|value| {
                if let Value::RecordId(attempt) = value {
                    if let RecordIdKey::String(key) = &attempt.key {
                        hex::decode(key)
                            .ok()
                            .and_then(|bytes| bytes.try_into().ok())
                            .map(ContentHash)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or(ModelError::Schema("native binding attempt"))?;
        if id.key
            != surrealdb::types::RecordIdKey::String(
                attempt_binding(attempt, descriptor.key()).hex(),
            )
        {
            return Err(ModelError::Conflict("completed binding key"));
        }
        check_projection(object, binding_projection(&descriptor))?;
    }
    Ok(None)
}

struct OriginalBacking {
    artifact: Id<d::source::SourceArtifact>,
    start: u64,
    len: usize,
    digest: ContentHash,
}
/// Check closed per-row framing. Bounded ordinary type slices independently validate model keys.
fn validate_compiler_backing(
    row: &Object,
    id: &RecordId,
) -> Result<Option<OriginalBacking>, ModelError> {
    let Some(Value::Object(body)) = row.get("body") else {
        return Err(ModelError::Schema("compiler backing body"));
    };
    let Some(Value::String(name)) = body.get("__type") else {
        return Err(ModelError::Schema("compiler backing type"));
    };
    let (key, original) = if name == d::artifact::ArtifactChunk::NAME {
        require_backing_fields(
            body,
            &[
                "__type", "artifact", "ordinal", "original", "start", "len", "digest",
            ],
        )?;
        let artifact: Id<d::source::SourceArtifact> = decode_backing_id(body, "artifact")?;
        let ordinal = backing_integer(body, "ordinal")?;
        let start = u64::try_from(backing_integer(body, "start")?).map_err(ModelError::codec)?;
        let len = usize::try_from(backing_integer(body, "len")?).map_err(ModelError::codec)?;
        let Some(Value::String(digest)) = body.get("digest") else {
            return Err(ModelError::Schema("compiler original digest"));
        };
        let digest_bytes: [u8; 32] = hex::decode(digest)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("compiler original digest width"))?;
        let digest_hash = ContentHash(digest_bytes);
        if u64::try_from(ordinal)
            .ok()
            .and_then(|ordinal| ordinal.checked_mul(d::artifact::ARTIFACT_CHUNK_BYTES as u64))
            != Some(start)
            || len == 0
            || len > d::artifact::ARTIFACT_CHUNK_BYTES
            || start
                .checked_add(len as u64)
                .is_none_or(|end| end > i64::MAX as u64)
            || digest_hash.hex() != *digest
            || body.get("original")
                != Some(&Value::RecordId(RecordId::new(
                    "original",
                    d::graph::EntityId::of(artifact).0.hex(),
                )))
        {
            return Err(ModelError::Conflict("compiler original metadata"));
        }
        let key = Id::<d::artifact::ArtifactChunk>::of(&d::artifact::ArtifactChunkKey {
            artifact,
            ordinal,
        });
        (
            *key.bytes(),
            Some(OriginalBacking {
                artifact,
                start,
                len,
                digest: digest_hash,
            }),
        )
    } else {
        let adapter = crate::adapter::select(name)?;
        if adapter.table != crate::schema::ScopeTable::CompilerRecord {
            return Err(ModelError::Schema("undeclared compiler backing type"));
        }
        let relation = &adapter.relation;
        let fields = std::iter::once("__type")
            .chain(relation.fields().iter().map(|field| field.name()))
            .collect::<Vec<_>>();
        require_backing_fields(body, &fields)?;
        let Some(Value::String(key)) = row.get("semantic_key") else {
            return Err(ModelError::Schema("compiler backing semantic key"));
        };
        let key: [u8; 16] = hex::decode(key)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("compiler backing semantic key width"))?;
        // This checks physical framing only. The bounded type slice below independently
        // recomputes ordinary-record nominal keys through the model callback.
        (key, None)
    };
    if row.get("semantic_type") != Some(&Value::String(name.clone()))
        || row.get("semantic_key") != Some(&Value::String(hex::encode(key)))
        || *id
            != crate::loader::payload_id(
                "compiler_record",
                name,
                &key,
                ContentHash::of(
                    serde_json::to_vec(&Value::Object(body.clone()))
                        .map_err(ModelError::codec)?
                        .as_slice(),
                ),
            )?
    {
        return Err(ModelError::Conflict("compiler backing typed identity"));
    }
    let Some(Value::Bytes(canonical)) = row.get("canonical") else {
        return Err(ModelError::Schema("compiler backing canonical bytes"));
    };
    let canonical_body: Value = serde_json::from_slice(canonical).map_err(ModelError::codec)?;
    if canonical_body != Value::Object(body.clone())
        || canonical.as_ref()
            != serde_json::to_vec(&Value::Object(body.clone()))
                .map_err(ModelError::codec)?
                .as_slice()
    {
        return Err(ModelError::Conflict("compiler backing canonical body"));
    }
    if row.get("content") != Some(&Value::String(ContentHash::of(canonical).hex())) {
        return Err(ModelError::Conflict("compiler backing content"));
    }
    let adapter = crate::adapter::select(name)?;
    if adapter.table != crate::schema::ScopeTable::CompilerRecord {
        return Err(ModelError::Schema("undeclared compiler backing type"));
    }
    let expected = crate::adapter::physical_row(
        id.clone(),
        ContentHash::of(canonical),
        canonical.to_vec(),
        None,
        crate::codec::RecordView {
            semantic_type: name.clone(),
            semantic_key: hex::encode(key),
            scopes: adapter.scopes(&canonical_body)?,
            body: canonical_body,
        },
    )?;
    if serde_json::to_vec(&Value::Object(row.clone())).map_err(ModelError::codec)?
        != serde_json::to_vec(&expected).map_err(ModelError::codec)?
    {
        return Err(ModelError::Conflict("compiler backing complete envelope"));
    }
    Ok(original)
}
fn require_backing_fields(body: &Object, fields: &[&str]) -> Result<(), ModelError> {
    if body.len() != fields.len() || fields.iter().any(|field| !body.contains_key(*field)) {
        return Err(ModelError::Schema("compiler backing closed body"));
    }
    Ok(())
}
fn backing_integer(body: &Object, field: &str) -> Result<i64, ModelError> {
    match body.get(field) {
        Some(Value::Number(surrealdb::types::Number::Int(value))) => Ok(*value),
        _ => Err(ModelError::Schema("compiler backing declared integer")),
    }
}
fn decode_backing_id<T: Record>(body: &Object, field: &str) -> Result<Id<T>, ModelError> {
    let Some(Value::Array(values)) = body.get(field) else {
        return Err(ModelError::Schema("compiler backing nominal bytes"));
    };
    if values.len()!=16 || values.iter().any(|value|!matches!(value,Value::Number(surrealdb::types::Number::Int(value)) if u8::try_from(*value).is_ok())) {return Err(ModelError::Schema("compiler backing nominal bytes"));}
    Ok(
        SerdeWrapper::<Id<T>>::from_value(Value::Array(values.clone()))
            .map_err(ModelError::codec)?
            .0,
    )
}

fn contribution_projection(spec: &ContributionSpec) -> Object {
    let mut fields = Object::new();
    fields.insert("producer", spec.producer.clone());
    fields.insert("profile", spec.profile.name().to_string());
    fields.insert("model", spec.model.hex());
    fields.insert("implementation", spec.implementation.hex());
    fields.insert(
        "configuration",
        spec.configuration
            .map(|hash| Value::String(hash.hex()))
            .unwrap_or(Value::Null),
    );
    let views = spec
        .inputs
        .iter()
        .map(|source| source.view())
        .collect::<std::collections::BTreeSet<_>>();
    fields.insert(
        "inputs",
        views
            .iter()
            .map(|view| RecordId::new("compiler_view", view.hex()))
            .collect::<Vec<_>>(),
    );
    fields.insert("outputs", spec.outputs.iter().cloned().collect::<Vec<_>>());
    fields
}
fn view_projection(view: &CompletedView) -> Result<Object, ModelError> {
    let mut fields = Object::new();
    fields.insert("relation", view.relation.clone());
    fields.insert(
        "contributions",
        view.contributions
            .iter()
            .map(ContentHash::hex)
            .collect::<Vec<_>>(),
    );
    fields.insert("rows", i64::try_from(view.rows).map_err(ModelError::codec)?);
    Ok(fields)
}
fn binding_projection(binding: &CompletedBinding) -> Object {
    let mut fields = Object::new();
    fields.insert("relation", binding.view.relation.clone());
    fields.insert(
        "boundary",
        binding
            .boundary
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    fields.insert(
        "view",
        RecordId::new("compiler_view", binding.view.identity.hex()),
    );
    fields
}
fn check_projection(actual: &Object, expected: Object) -> Result<(), ModelError> {
    for (field, value) in expected {
        if actual.get(&field) != Some(&value) {
            return Err(ModelError::Conflict("completed metadata projection"));
        }
    }
    Ok(())
}

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
    let mut bytes = Vec::with_capacity(len);
    let mut position = start;
    for chunk in chunks {
        let Some(chunk) = value_object(&chunk) else {
            return Err(ModelError::Schema("original physical chunk"));
        };
        let Some(Value::Number(surrealdb::types::Number::Int(at))) = chunk.get("start") else {
            return Err(ModelError::Schema("original physical chunk position"));
        };
        let Some(Value::Bytes(body)) = chunk.get("bytes") else {
            return Err(ModelError::Schema("original physical chunk bytes"));
        };
        if u64::try_from(*at).map_err(ModelError::codec)? != position
            || body.is_empty()
            || body.len() > 65536
            || body.len() > len.saturating_sub(bytes.len())
            || chunk.get("content") != Some(&Value::String(ContentHash::of(body).hex()))
        {
            return Err(ModelError::Conflict("original physical chunk integrity"));
        }
        bytes.extend_from_slice(body);
        position += body.len() as u64;
    }
    if bytes.len() != len || ContentHash::of(&bytes).hex() != digest {
        return Err(ModelError::Conflict("original typed chunk integrity"));
    }
    row.insert("body", Bytes::from(bytes));
    Ok(Value::Object(row))
}

/// One bounded cold-state window, grouped through existing model callbacks. Hot writes and
/// completed-view reads carry their validated typed ownership and never take this audit route.
fn validate_backing_batch(rows: &[Value]) -> Result<(), ModelError> {
    let mut groups = BTreeMap::<&str, Vec<&Object>>::new();
    for row in rows {
        let object = value_object(row).ok_or(ModelError::Schema("compiler backing row"))?;
        let Some(Value::String(name)) = object.get("semantic_type") else {
            return Err(ModelError::Schema("compiler backing semantic type"));
        };
        if name != d::artifact::ArtifactChunk::NAME {
            groups.entry(name).or_default().push(object);
        }
    }
    let budget =
        d::resources::ResourceBudget::fixed(d::resources::MAX_ROW_BYTES.saturating_mul(4))?;
    for (name, mut rows) in groups {
        let adapter = crate::adapter::select(name)?;
        if adapter.table != crate::schema::ScopeTable::CompilerRecord {
            return Err(ModelError::Schema("undeclared compiler backing type"));
        }
        let relation = &adapter.relation;
        rows.sort_by_key(|row| match row.get("semantic_key") {
            Some(Value::String(key)) => key.as_str(),
            _ => "",
        });
        let mut builder = crate::projected_arrow::ProjectedBuilder::new(
            relation.clone(),
            relation.schema().clone(),
            &budget,
        )?;
        for row in &rows {
            let Some(Value::Object(body)) = row.get("body") else {
                return Err(ModelError::Schema("compiler backing body"));
            };
            let mut fields = body.clone();
            fields.insert(
                "id",
                row.get("semantic_key")
                    .ok_or(ModelError::Schema("compiler backing semantic key"))?
                    .clone(),
            );
            builder.push(Value::Object(fields))?;
        }
        let batch = relation
            .canonical(&builder.finish()?)
            .map_err(|_| ModelError::Conflict("compiler backing typed identity"))?;
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("compiler backing nominal column"))?;
        let bodies = crate::codec::batch_bodies(relation, &batch)?;
        if batch.num_rows() != rows.len() {
            return Err(ModelError::Conflict("compiler backing type slice"));
        }
        for (index, (row, body)) in rows.iter().zip(bodies).enumerate() {
            let key = hex::encode(ids.value(index));
            if row.get("semantic_key") != Some(&Value::String(key.clone())) {
                return Err(ModelError::Conflict("compiler backing typed identity"));
            }
            let canonical = serde_json::to_vec(&body).map_err(ModelError::codec)?;
            let expected = crate::adapter::physical_row(
                crate::loader::payload_id(
                    "compiler_record",
                    name,
                    ids.value(index),
                    ContentHash::of(&canonical),
                )?,
                ContentHash::of(&canonical),
                canonical,
                None,
                crate::codec::RecordView {
                    semantic_type: name.into(),
                    semantic_key: key,
                    scopes: adapter.scopes(&body)?,
                    body,
                },
            )?;
            if serde_json::to_vec(&Value::Object((*row).clone())).map_err(ModelError::codec)?
                != serde_json::to_vec(&expected).map_err(ModelError::codec)?
            {
                return Err(ModelError::Conflict("compiler backing declared body"));
            }
        }
    }
    Ok(())
}
fn admit_backing_row(
    pending: &mut Vec<Value>,
    bytes: &mut usize,
    row: Value,
) -> Result<(), ModelError> {
    let weight = crate::loader::native_bytes(&row);
    if weight > d::resources::MAX_ROW_BYTES {
        return Err(ModelError::Schema("compiler backing row bound"));
    }
    if !pending.is_empty()
        && (pending.len() >= d::resources::TRANSFER_ROWS
            || bytes.saturating_add(weight) > d::resources::TRANSFER_BYTES)
    {
        validate_backing_batch(pending)?;
        pending.clear();
        *bytes = 0;
    }
    *bytes = bytes.saturating_add(weight);
    pending.push(row);
    Ok(())
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    fn local_store() -> Arc<NativeCompilerStore> {
        NativeCompilerStore::from_existing(
            Arc::new(Surreal::init()),
            d::serving::Name::new("local_control").unwrap(),
            d::serving::Name::new("owned_control").unwrap(),
        )
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
        let store = local_store();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let scope = store
            .producing_scope(ContentHash::of(b"lazy-producing-provider"), &budget)
            .unwrap();
        let relation = Relation::of::<d::input::Package>();
        let view = CompletedView::new(
            relation.name().into(),
            std::collections::BTreeSet::from([ContentHash::of(b"lazy-empty-completed-owner")]),
            0,
        )
        .unwrap();
        let provider = store.table_provider(&view, relation, budget, 16).unwrap();
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
    async fn local_terminal_drain_retains_unknown_native_acknowledgement() {
        let store = local_store();
        let lease = store.admit(false, "control write", true).unwrap();
        drop(lease);
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
        let store = local_store();
        store.admission.state.lock().unwrap().content_ready = true;
        let ordinary = store.admit(false, "native final seal", false).unwrap();
        assert!(
            store
                .completed_state_after_closure(&ordinary)
                .await
                .is_err()
        );
        ordinary.finish();
        let other = local_store();
        other.admission.state.lock().unwrap().content_ready = true;
        let unrelated = other.begin_finalization().await.unwrap();
        assert!(
            store
                .completed_state_after_closure(&unrelated)
                .await
                .is_err()
        );
        unrelated.finish();
        let parent = store.begin_finalization().await.unwrap();
        let scan = store.retained_scan().unwrap();
        let mut read = Box::pin(store.completed_state_after_closure(&parent));
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
    #[tokio::test(flavor = "multi_thread")]
    async fn batched_empty_views_verify_membership_and_retain_dependency_ownership() {
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("stable validation runtime"),
        ))
        .unwrap();
        let native = NativeCompilerStore::begin(&config, Frontier::Facts)
            .await
            .unwrap();
        let client = check_installation(&config).await.unwrap();
        let reader = crate::NativeReader::private(client.clone());
        let mut forged_id = None;
        let result = async {
            let nonce = crate::control::fresh_identity("batched-empty-views")?;
            let relation = Relation::of::<d::analytics::QualityStep>();
            let model = d::model()?.digest();
            let spec = ContributionSpec {
                captured_binding: None, producer: format!("base-{}", nonce.hex()),
                profile: d::stages::Profile::Catalog, model, implementation: nonce,
                configuration: None, inputs: vec![],
                outputs: std::collections::BTreeSet::from([relation.name().to_string()]),
            };
            let base = native.begin_contribution(spec).await?;
            let row = d::analytics::QualityStep {
                run: serde_json::from_value(serde_json::to_value(
                    <[u8;16]>::try_from(&nonce.0[..16]).map_err(ModelError::codec)?,
                ).map_err(ModelError::codec)?).map_err(ModelError::codec)?,
                ordinal: 0, value: d::FiniteF64::new(0.75)?,
            };
            native.write_batch(&base, &relation, &d::analytics::QualityStep::encode(&[row])?).await?;
            let base_views = native.complete_contribution(base, ProviderOutcome::Complete,
                std::slice::from_ref(&relation), &BTreeMap::new()).await?;
            let base_view = base_views[relation.name()].clone();
            let outputs = [Relation::of::<d::analytics::RankScore>(),
                Relation::of::<d::analytics::CommunityRun>()];
            let spec = ContributionSpec {
                captured_binding: None, producer: format!("empty-{}", nonce.hex()),
                profile: d::stages::Profile::Catalog, model, implementation: nonce,
                configuration: None,
                inputs: vec![d::analysis::sources::SourceSnapshot::of_completed_view(
                    &relation, model, &base_view,
                )?],
                outputs: outputs.iter().map(|relation| relation.name().to_string()).collect(),
            };
            let empty = native.begin_contribution(spec).await?;
            let views = native.complete_contribution(empty, ProviderOutcome::Complete,
                &outputs, &BTreeMap::new()).await?;
            let empty_ids = views.values().map(|view| RecordId::new(
                "compiler_view", view.identity.hex(),
            )).collect::<Vec<_>>();
            let attempt = RecordId::new("native_attempt", native.attempt().hex());
            let contributor = RecordId::new("compiler_contribution", empty.hex());
            let base_id = RecordId::new("compiler_view", base_view.identity.hex());
            let base_contributor = RecordId::new("compiler_contribution", base.hex());
            let mut expected = std::collections::BTreeSet::from([
                (contributor.clone(), base_id.clone()),
                (base_id.clone(), base_contributor.clone()),
                (attempt.clone(), base_id.clone()),
                (attempt.clone(), base_contributor.clone()),
                (attempt.clone(), contributor.clone()),
            ]);
            for view in &empty_ids {
                expected.insert((attempt.clone(), view.clone()));
                expected.insert((view.clone(), contributor.clone()));
            }
            let mut vars = Variables::new();
            vars.insert("views", empty_ids.clone());
            vars.insert("owners", expected.iter().map(|(owner,_)| owner.clone()).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>());
            vars.insert("objects", expected.iter().map(|(_,object)| object.clone()).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>());
            let members: Vec<RecordId> = reader.query_native(
                "SELECT VALUE id FROM compiler_view_member WHERE view IN $views", vars.clone(),
            ).await?;
            let rows: Vec<Object> = reader.query_native(
                "SELECT owner,object FROM native_hold WITH INDEX owner_holds WHERE owner IN $owners AND object IN $objects", vars,
            ).await?;
            let edges = rows.into_iter().map(|row| match (row.get("owner"),row.get("object")) {
                (Some(Value::RecordId(owner)),Some(Value::RecordId(object))) => Ok((owner.clone(),object.clone())),
                _ => Err(ModelError::Schema("empty view retention edge")),
            }).collect::<Result<std::collections::BTreeSet<_>,_>>()?;
            let mut eligibility = Vec::new();
            for object in [contributor, base_id, base_contributor] {
                eligibility.push(crate::control::retirement_eligibility(&client, object).await?);
            }
            let forged = CompletedView::new(base_view.relation.clone(),
                base_view.contributions.clone(), 0)?;
            forged_id = Some(RecordId::new("compiler_view", forged.identity.hex()));
            // A genuine membership must defeat a forged zero even after its descriptor
            // is ensured. The normal member scan and cardinality check own this refusal.
            let refusal = native.registered_or_install_views(&[&forged]).await;
            let remembered = native.known_views.lock()
                .map_err(|_| ModelError::Conflict("native view owner"))?
                .contains_key(&forged.identity);
            Ok::<_, ModelError>((views, members, edges, expected, eligibility, refusal, remembered))
        }.await;
        let mut completion = d::completion::Completion::default();
        let abandoned = native.abandon().await;
        let can_retire = abandoned.is_ok();
        completion.step("batched view attempt finalization", abandoned);
        if let Some(id) = forged_id {
            if can_retire {
                let retired = async {
                    crate::control::retire_reachable(&client, vec![id.clone()], 4096).await?;
                    let mut vars = Variables::new();
                    vars.insert("view", id.clone());
                    let view: Vec<RecordId> = reader.query_native(
                        "SELECT VALUE id FROM $view", vars.clone(),
                    ).await?;
                    let members: Vec<RecordId> = reader.query_native(
                        "SELECT VALUE id FROM compiler_view_member WHERE view=$view", vars.clone(),
                    ).await?;
                    let holds: Vec<RecordId> = reader.query_native(
                        "SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$view", vars,
                    ).await?;
                    if !view.is_empty() || !members.is_empty() || !holds.is_empty() {
                        return Err(ModelError::Conflict("forged view cleanup remains reachable"));
                    }
                    Ok(())
                }.await;
                completion.step("exact forged view retirement", retired);
            } else {
                completion
                    .storage
                    .push(d::completion::StorageState::Orphan(id.to_sql()));
            }
        }
        completion.step("batched view reader close", reader.close().await);
        completion.step(
            "batched view session close",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let (views, members, edges, expected, eligibility, refusal, remembered) =
            d::completion::complete(result, completion).unwrap();
        assert_eq!(views.len(), 2);
        assert!(views.values().all(|view| view.rows == 0));
        assert!(
            members.is_empty(),
            "both installed empty views have actual zero memberships"
        );
        assert_eq!(
            edges, expected,
            "both ownership edge kinds and the exact prerequisite chain are retained"
        );
        assert!(
            eligibility.iter().all(|item| !item.eligible
                && item
                    .reasons
                    .iter()
                    .any(|reason| reason == "reachable through native owner hold",)),
            "contributors and their source view remain protected"
        );
        assert!(matches!(
            refusal,
            Err(ModelError::Conflict("exact view cardinality"))
        ));
        assert!(
            !remembered,
            "a descriptor cannot grant registration before actual membership and ownership"
        );
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn imported_membership_pages_retain_contributor_reachability_without_duplicate_attempt_roots()
     {
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
                captured_binding: None, producer: nonce.hex(), profile: d::stages::Profile::Catalog,
                model: d::model()?.digest(), implementation: nonce, configuration: None,
                inputs: vec![], outputs: std::collections::BTreeSet::from([relation.name().to_string()]),
            };
            let contribution = source.begin_contribution(spec.clone()).await?;
            source.write_batch(&contribution, &relation, &d::analytics::QualityStep::encode(&[row])?).await?;
            let views = source.complete_contribution(contribution, ProviderOutcome::Complete, std::slice::from_ref(&relation), &BTreeMap::new()).await?;
            let view = views[relation.name()].clone();
            source.bind(CompletedBinding {
                boundary: None,
                source: d::analysis::sources::SourceSnapshot::of_completed_view(&relation, spec.model, &view)?,
                view, configuration: None,
            }).await?;
            source.retain_product_identity(nonce, contribution).await?;
            source.mark_attempt_admitted().await?;
            let directory = tempfile::tempdir().map_err(ModelError::codec)?;
            let path = directory.path().join("completed.jsonl");
            let expected = source.export_state(&path).await?;
            let transport = std::fs::read_to_string(&path).map_err(ModelError::codec)?;
            let mut pages = [Vec::new(), Vec::new()];
            for line in transport.lines().skip(1) {
                let row: StateRow = serde_json::from_str(line).map_err(ModelError::codec)?;
                match row.table.as_str() {
                    "compiler_contribution" => pages[0].push(row.row),
                    "compiler_membership" => pages[1].push(row.row),
                    _ => break,
                }
            }
            if pages.iter().any(|page| page.len()!=1) {
                return Err(ModelError::Invalid("single-row partial import fixture".into()));
            }
            // Execute the actual confirmed import pages, then stop before views, backing,
            // bindings, dependency attachment or completion. No public fault hook is needed.
            target.insert_state_batch(0, std::mem::take(&mut pages[0])).await?;
            target.insert_state_batch(1, std::mem::take(&mut pages[1])).await?;
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
            let retired = crate::control::retire_reachable(&client, contributors.clone(), 16).await?;
            let remaining_members: Vec<RecordId> = reader.query_native("SELECT VALUE id FROM compiler_membership WITH INDEX contribution_rows WHERE contribution IN $contributors", vars.clone()).await?;
            let retained: Vec<Value> = reader.query_native("SELECT * FROM $node", vars).await?;
            source.verify_state().await?;
            let state_after_cleanup = source.completed_state().await?;
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
        assert_eq!(state_after_cleanup, expected);
    }
    #[test]
    fn canonical_compiler_backing_rejects_imported_scope_and_retired_scalars() {
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
        validate_backing_batch(&[Value::Object(row.clone())]).unwrap();
        for field in ["unexpected", "kind", "subtype", "scope_context"] {
            let mut altered = row.clone();
            altered.insert(field, Value::Null);
            assert!(
                validate_compiler_backing(&altered, &id).is_err(),
                "extra envelope field {field}"
            );
            assert!(
                validate_backing_batch(&[Value::Object(altered)]).is_err(),
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
