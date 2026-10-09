//! Private native compiler authority. Completed memberships, rather than a publication handle,
//! govern reads. Canonical graph payloads and query fields are immutable mechanical forms.
use crate::{
    Loader, RuntimeConfig,
    reader::{self, NativeRows},
};
use crate::{
    acknowledged_candidates::{
        AsyncCandidateSort, AsyncOrderedCandidates, AsyncOrderedRows, AsyncPhysicalSort,
        BlockingTerminal,
    },
    ordered_rows::Candidate,
};
use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use d::resources::{Reservation, ResourceBudget};
use futures::{FutureExt, future::BoxFuture};
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
    types::{Bytes, Object, RecordId, SerdeWrapper, SurrealValue, ToSql, Value, Variables},
};
use tokio::sync::Notify;

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
    client: Arc<Surreal<Client>>,
    database: d::serving::Name,
    namespace: d::serving::Name,
    specifications: Mutex<BTreeMap<ContentHash, (ContributionSpec, bool)>>,
    known_views: Mutex<BTreeMap<ContentHash, CompletedView>>,
    known_contributors: Mutex<BTreeMap<ContentHash, ContentHash>>,
    failed: AtomicBool,
    sealed: AtomicBool,
    admission: Arc<OperationAdmission>,
    runtime: tokio::runtime::Handle,
    frontier: Mutex<Frontier>,
}
#[derive(Default)]
struct OperationState {
    closed: bool,
    active: usize,
    scans: usize,
    failed: bool,
    uncertain: bool,
    failures: Vec<NativeFailure>,
}
struct NativeFailure {
    operation: &'static str,
    error: Arc<ModelError>,
}
impl OperationState {
    fn refusal(&self, detail: &'static str) -> ModelError {
        let refusal = ModelError::infrastructure(d::Infrastructure::State, detail);
        let Some(first) = self.failures.first() else {
            return refusal;
        };
        let mut completion = d::completion::Completion::default();
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
struct OperationLease {
    owner: Arc<OperationAdmission>,
    scan: bool,
    finished: bool,
    operation: &'static str,
    poison_on_error: bool,
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
    fn finish_with<T>(self, result: Result<T, ModelError>) -> Result<T, ModelError> {
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
        if let Ok(mut state) = self.owner.state.lock() {
            state.active -= 1;
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
    "DEFINE TABLE compiler_contribution SCHEMAFULL; DEFINE FIELD spec ON compiler_contribution TYPE bytes; DEFINE FIELD descriptor ON compiler_contribution TYPE option<bytes>; DEFINE FIELD completed ON compiler_contribution TYPE bool; DEFINE FIELD logical ON compiler_contribution TYPE option<string>; DEFINE INDEX logical_contribution ON compiler_contribution FIELDS logical UNIQUE; DEFINE TABLE compiler_membership SCHEMAFULL; DEFINE FIELD contribution ON compiler_membership TYPE record<compiler_contribution>; DEFINE FIELD relation ON compiler_membership TYPE string; DEFINE FIELD semantic_key ON compiler_membership TYPE string; DEFINE FIELD node ON compiler_membership TYPE record<entity | assertion | compiler_record>; DEFINE FIELD content ON compiler_membership TYPE string; DEFINE INDEX contribution_rows ON compiler_membership FIELDS contribution,relation,semantic_key UNIQUE; DEFINE INDEX member_keys ON compiler_membership FIELDS relation,semantic_key,contribution; DEFINE TABLE compiler_view SCHEMAFULL; DEFINE FIELD descriptor ON compiler_view TYPE bytes; DEFINE TABLE compiler_binding SCHEMAFULL; DEFINE FIELD descriptor ON compiler_binding TYPE bytes; DEFINE TABLE compiler_alias SCHEMAFULL; DEFINE FIELD source ON compiler_alias TYPE record<entity>; DEFINE FIELD target ON compiler_alias TYPE record<entity>; DEFINE INDEX alias_source ON compiler_alias FIELDS source,target UNIQUE;".to_string()+"DEFINE FIELD producer ON compiler_contribution TYPE string; DEFINE FIELD profile ON compiler_contribution TYPE string; DEFINE FIELD model ON compiler_contribution TYPE string; DEFINE FIELD implementation ON compiler_contribution TYPE string; DEFINE FIELD configuration ON compiler_contribution TYPE option<string|null>; DEFINE FIELD inputs ON compiler_contribution TYPE array<record<compiler_view>>; DEFINE FIELD outputs ON compiler_contribution TYPE array<string>; DEFINE FIELD outcome ON compiler_contribution TYPE option<int>; DEFINE INDEX contribution_inputs ON compiler_contribution FIELDS inputs; DEFINE FIELD relation ON compiler_view TYPE string; DEFINE FIELD contributions ON compiler_view TYPE array<string>; DEFINE FIELD rows ON compiler_view TYPE int; DEFINE FIELD relation ON compiler_binding TYPE string; DEFINE FIELD boundary ON compiler_binding TYPE option<string|null>; DEFINE FIELD view ON compiler_binding TYPE record<compiler_view>; DEFINE INDEX binding_view ON compiler_binding FIELDS view;"+&crate::schema::compiler_record_schema()
}

impl NativeCompilerStore {
    pub async fn begin(
        config: &RuntimeConfig,
        frontier: Frontier,
    ) -> Result<Arc<Self>, ModelError> {
        let mut sink = KeySink::new("private-native-attempt/v1");
        sink.part(
            b"clock",
            &std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(ModelError::codec)?
                .as_nanos()
                .to_le_bytes(),
        );
        sink.part(b"process", &std::process::id().to_le_bytes());
        let database = d::serving::Name::new(format!("snapshot_{}", sink.finish().hex()))
            .map_err(ModelError::codec)?;
        let client =
            reader::authenticated(&config.endpoint, &config.root_credentials(), None).await?;
        let mut creation_attempted = false;
        let setup = async {
            let version = client
                .version()
                .await
                .map_err(ModelError::codec)?
                .to_string();
            if !version.starts_with("3.3.") {
                return Err(ModelError::Invalid(
                    "native compiler requires reviewed SurrealDB3.3".into(),
                ));
            }
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
            creation_attempted = true;
            client
                .query(format!("DEFINE DATABASE `{}` STRICT", database.as_str()))
                .await
                .map_err(crate::loader::write_failure)?
                .check()
                .map_err(|error| {
                    ModelError::codec(format!("native compiler database creation: {error}"))
                })?;
            client
                .use_db(database.as_str())
                .await
                .map_err(ModelError::codec)?;
            Loader::new(client.clone()).install("").await?;
            Loader::new(client.clone())
                .install_declarations(&compiler_schema(), "compiler state schema")
                .await?;
            Ok::<(), ModelError>(())
        }
        .await;
        if let Err(error) = setup {
            let mut completion = d::completion::Completion::default();
            if creation_attempted && !error.permits_storage_cleanup() {
                completion.remote = d::completion::RemoteState::Unknown;
                completion
                    .storage
                    .push(d::completion::StorageState::Orphan(format!(
                        "{}/{}",
                        config.namespace.as_str(),
                        database.as_str()
                    )));
            } else if creation_attempted {
                let cleanup = async {
                    client
                        .query(format!("REMOVE DATABASE IF EXISTS `{}`", database.as_str()))
                        .await
                        .map_err(crate::loader::write_failure)?
                        .check()
                        .map_err(|error| ModelError::Cause(Box::new(error)))?;
                    Ok::<(), ModelError>(())
                }
                .await;
                completion.cleanup(
                    format!("{}/{}", config.namespace.as_str(), database.as_str()),
                    cleanup,
                );
            }
            completion.step(
                "setup session invalidation",
                client
                    .invalidate()
                    .await
                    .map_err(|error| ModelError::Cause(Box::new(error))),
            );
            return d::completion::complete(Err(error), completion);
        }
        Ok(Arc::new(Self {
            client,
            database,
            namespace: config.namespace.clone(),
            specifications: Mutex::default(),
            known_views: Mutex::default(),
            known_contributors: Mutex::default(),
            failed: AtomicBool::new(false),
            sealed: AtomicBool::new(false),
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
            namespace,
            specifications: Mutex::default(),
            known_views: Mutex::default(),
            known_contributors: Mutex::default(),
            failed: AtomicBool::new(false),
            sealed: AtomicBool::new(false),
            admission: Arc::new(OperationAdmission::default()),
            runtime: tokio::runtime::Handle::current(),
            frontier: Mutex::new(Frontier::Facts),
        })
    }
    pub async fn install_state_schema(self: &Arc<Self>) -> Result<(), ModelError> {
        let lease = self.admit(false, "install_state_schema", true)?;
        let result = self.install_state_schema_inner().await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
        result
    }
    async fn install_state_schema_inner(self: &Arc<Self>) -> Result<(), ModelError> {
        Loader::new(self.client.clone())
            .install_declarations(&compiler_schema(), "compiler state schema")
            .await
    }
    pub fn set_frontier(&self, frontier: Frontier) -> Result<(), ModelError> {
        let lease = self.admit(false, "set_frontier", false)?;
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
    pub async fn contributions(self: &Arc<Self>) -> Result<Vec<CompletedContribution>, ModelError> {
        let lease = self.admit(false, "contributions", false)?;
        let result = self.contributions_inner().await;
        lease.finish_with(result)
    }
    async fn contributions_inner(
        self: &Arc<Self>,
    ) -> Result<Vec<CompletedContribution>, ModelError> {
        let mut rows = self.track_rows(NativeRows::new(
            self.client
                .query("SELECT * FROM compiler_contribution WHERE completed=true ORDER BY logical")
                .stream_items()
                .map_err(ModelError::codec)?,
            1,
        )?)?;
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
            "id,content,canonical,kind,subtype,semantic_type,semantic_key",
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
            "id,kind,subtype,body.byte_len AS source_length",
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
        let mut owners = self.track_rows(NativeRows::new(
            self.client
                .query("SELECT VALUE id FROM compiler_contribution WHERE completed=true")
                .stream_items()
                .map_err(ModelError::codec)?,
            1,
        )?)?;
        while let Some(owner) = owners.next_native().await? {
            let Value::RecordId(owner) = owner else {
                return Err(ModelError::Schema("canonical contribution owner"));
            };
            let mut bindings = Variables::new();
            bindings.insert("owner", owner);
            let mut members=self.track_rows(NativeRows::new(self.client.query("SELECT node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner").bind(bindings).stream_items().map_err(ModelError::codec)?,1)?)?;
            while let Some(member) = members.next_native().await? {
                let Some(Value::RecordId(node)) =
                    value_object(&member).and_then(|row| row.get("node"))
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
        }
        if !frontier.is_empty() {
            self.expand_graph_frontier(&mut sorted, &frontier, entities)
                .await?;
        }
        let ordered = sorted.finish().await?;
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
            rows: None,
            selection: None,
            graph: Some(Box::new(graph)),
            store: self.clone(),
            lease: Some(lease),
            setup: None,
        })
    }
    async fn expand_graph_frontier(
        self: &Arc<Self>,
        sorted: &mut AsyncPhysicalSort,
        frontier: &[RecordId],
        entities: bool,
    ) -> Result<(), ModelError> {
        for node in frontier {
            sorted.push(pointer_row(node.clone())).await?;
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
                sorted.push(pointer_row(alias)).await?;
            }
        }
        Ok(())
    }
    pub fn client(&self) -> &Surreal<Client> {
        &self.client
    }
    pub fn shared_client(&self) -> Arc<Surreal<Client>> {
        self.client.clone()
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
        let mut state = self
            .admission
            .state
            .lock()
            .map_err(|_| ModelError::Conflict("native operation admission"))?;
        if state.closed
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
        if scan {
            state.scans += 1;
        }
        Ok(OperationLease {
            owner: self.admission.clone(),
            scan,
            finished: false,
            operation,
            poison_on_error,
        })
    }
    // An already admitted parent can start its transport even after final closure begins.
    fn track_rows(self: &Arc<Self>, rows: NativeRows) -> Result<CompilerRows, ModelError> {
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
        let lease = OperationLease {
            owner: self.admission.clone(),
            scan: true,
            finished: false,
            operation: "native stream",
            poison_on_error: true,
        };
        Ok(CompilerRows {
            rows: Some(rows),
            selection: None,
            graph: None,
            store: self.clone(),
            lease: Some(lease),
            setup: None,
        })
    }
    fn retained_scan(&self) -> Result<OperationLease, ModelError> {
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
            finished: false,
            operation: "native stream",
            poison_on_error: true,
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
        self.runtime.spawn(async move {
            let result = lease.observe_result(setup.await);
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
        });
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
        Ok(move |terminal| {
            let runtime = store.runtime.clone();
            runtime.spawn(async move {
                if let Err(error) = terminal.await {
                    store.record_background_failure(error);
                }
                lease.finish();
            });
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
        let owners = self.view_owners(view)?;
        let transfer = budget.reserve(
            "compiler-selected-window",
            owners
                .len()
                .saturating_mul(64)
                .saturating_add(crate::loader::NATIVE_WINDOW_ROWS * 2048),
        )?;
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
        let identity = format!("{}/{}", self.namespace.as_str(), self.database.as_str());
        if completion.local == d::completion::LocalState::Terminal
            && completion.remote == d::completion::RemoteState::Confirmed
        {
            let cleanup = async {
                self.client
                    .query(format!(
                        "REMOVE DATABASE IF EXISTS `{}`",
                        self.database.as_str()
                    ))
                    .await
                    .map_err(crate::loader::write_failure)?
                    .check()
                    .map_err(|error| ModelError::Cause(Box::new(error)))?;
                Ok(())
            }
            .await;
            completion.cleanup(identity, cleanup);
        } else {
            completion
                .storage
                .push(d::completion::StorageState::Orphan(identity));
        }
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
        let lease = self.admit(false, "begin_contribution", true)?;
        let result = self.begin_contribution_inner(spec).await;
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
        let id = spec.identity()?;
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
        for (field, value) in contribution_projection(&spec) {
            row.insert(field, value);
        }
        let mut b = Variables::new();
        b.insert("row", row);
        let write = async {
            self.client
                .query("INSERT INTO compiler_contribution $row RETURN NONE")
                .bind(b)
                .await
                .map_err(crate::loader::write_failure)?
                .check()
                .map_err(ModelError::codec)?;
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
            let lease = self.admit(false, "write_batch", true)?;
            let result = self.write_batch_inner(contribution, relation, batch).await;
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
        let batch = relation.canonical(batch)?;
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("native nominal ID column"))?;
        let mut bodies = crate::codec::batch_bodies(relation, &batch)?;
        if relation.name() == d::artifact::ArtifactChunk::NAME {
            for (body, row) in bodies
                .iter_mut()
                .zip(d::artifact::ArtifactChunk::decode(&batch)?)
            {
                self.persist_original_chunk(&row).await?;
                let start = u64::try_from(row.ordinal)
                    .map_err(ModelError::codec)?
                    .checked_mul(d::artifact::ARTIFACT_CHUNK_BYTES as u64)
                    .ok_or(ModelError::Schema("original chunk start"))?;
                let mut metadata = Object::new();
                metadata.insert("__type", relation.name().to_string());
                metadata.insert(
                    "artifact",
                    Value::from_t(
                        row.artifact
                            .bytes()
                            .iter()
                            .map(|byte| i64::from(*byte))
                            .collect::<Vec<_>>(),
                    ),
                );
                metadata.insert("ordinal", row.ordinal);
                metadata.insert(
                    "original",
                    RecordId::new("original", d::graph::EntityId::of(row.artifact).0.hex()),
                );
                metadata.insert("start", start);
                metadata.insert("len", row.body.0.len() as u64);
                metadata.insert("digest", ContentHash::of(&row.body.0).hex());
                *body = Value::Object(metadata);
            }
        }
        if relation.name() == d::source::SourceArtifact::NAME {
            let rows = d::source::SourceArtifact::decode(&batch)?
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
                .collect();
            self.ensure_immutable_rows("original", rows, "original same-key payload")
                .await?;
        }
        let mut graph = vec![None; batch.num_rows()];
        macro_rules! entities {($($variant:ident:$ty:ty,)*)=>{$(if relation.name()==<$ty>::NAME {for (i,row) in <$ty>::decode(&batch)?.into_iter().enumerate() {graph[i]=Some(GraphRow::Entity(d::graph::Entity::from(row)));}})*};}
        lctx_model::graph_entity_records!(entities);
        macro_rules! assertions {($($variant:ident:$ty:ty,)*)=>{$(if relation.name()==<$ty>::NAME {for (i,row) in <$ty>::decode(&batch)?.into_iter().enumerate() {graph[i]=Some(GraphRow::Assertion(d::graph::Assertion::from_record(row)?));}})*};}
        lctx_model::graph_assertion_records!(assertions);
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
                        source
                            .canonical_place_endpoint()
                            .map(|target| (source.id(), target))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut nodes = BTreeMap::<String, Value>::new();
        let mut members = BTreeMap::<String, Value>::new();
        for (i, (body, graph)) in bodies.into_iter().zip(graph).enumerate() {
            let key = hex::encode(ids.value(i));
            let (node, content, canonical, kind, subtype) = match graph {
                Some(GraphRow::Entity(row)) => (
                    RecordId::new("entity", row.id().0.hex()),
                    row.content(),
                    serde_json::to_vec(&row).map_err(ModelError::codec)?,
                    Some(row.kind() as i64),
                    row.subtype(),
                ),
                Some(GraphRow::Assertion(row)) => (
                    RecordId::new("assertion", row.id().0.hex()),
                    row.content(),
                    serde_json::to_vec(&row).map_err(ModelError::codec)?,
                    Some(row.kind as i64),
                    None,
                ),
                None => {
                    if !crate::schema::compiler_relations()
                        .iter()
                        .any(|declared| declared.name() == relation.name())
                    {
                        return Err(ModelError::Schema("undeclared compiler backing type"));
                    }
                    let bytes = serde_json::to_vec(&body).map_err(ModelError::codec)?;
                    let mut sink = KeySink::new("compiler-backing-key/v1");
                    relation.name().to_string().encode(&mut sink);
                    sink.part(b"key", ids.value(i));
                    (
                        RecordId::new("compiler_record", sink.finish().hex()),
                        ContentHash::of(&bytes),
                        bytes,
                        None,
                        None,
                    )
                }
            };
            let mut physical = Object::new();
            physical.insert("id", node.clone());
            physical.insert("semantic_type", relation.name().to_string());
            physical.insert("semantic_key", key.clone());
            physical.insert("content", content.hex());
            physical.insert("canonical", Bytes::from(canonical));
            crate::reconciliation::add_scope_fields(
                &mut physical,
                &body,
                relation.name(),
                crate::schema::ScopeTable::from_name(node.table.as_str())?,
            )?;
            physical.insert("body", body);
            if let Some(kind) = kind {
                physical.insert("kind", kind);
                physical.insert("subtype", subtype.map(Value::from_t).unwrap_or(Value::Null));
            }
            let physical = Value::Object(physical);
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
            let node = RecordId::new("entity", target.id().0.hex());
            let mut physical = Object::new();
            physical.insert("id", node.clone());
            physical.insert("semantic_type", view.semantic_type.clone());
            physical.insert("semantic_key", view.semantic_key);
            physical.insert("kind", target.kind() as i64);
            physical.insert(
                "subtype",
                target.subtype().map(Value::from_t).unwrap_or(Value::Null),
            );
            physical.insert("content", target.content().hex());
            physical.insert(
                "canonical",
                Bytes::from(serde_json::to_vec(&target).map_err(ModelError::codec)?),
            );
            crate::reconciliation::add_scope_fields(
                &mut physical,
                &view.body,
                &view.semantic_type,
                crate::schema::ScopeTable::Entity,
            )?;
            physical.insert("body", view.body);
            let physical = Value::Object(physical);
            if let Some(previous) = nodes.insert(node.to_sql(), physical.clone())
                && previous != physical
            {
                return Err(ModelError::Conflict("native alias payload"));
            }
            let mut sink = KeySink::new("native-place-alias/v1");
            source.0.encode(&mut sink);
            target.id().0.encode(&mut sink);
            let mut alias = Object::new();
            alias.insert("id", RecordId::new("compiler_alias", sink.finish().hex()));
            alias.insert("source", RecordId::new("entity", source.0.hex()));
            alias.insert("target", node);
            aliases.push(Value::Object(alias));
        }
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
        // Read only bounded candidate physical IDs; compare full canonical payloads, not digest
        // alone. Derived schema/index fields are not a second editable semantic payload.
        for window in crate::loader::NativeWindows::new(rows) {
            let mut nodes = BTreeMap::new();
            for row in window? {
                let id = value_object(&row)
                    .and_then(|object| object.get("id"))
                    .ok_or(ModelError::Schema("native payload id"))?
                    .to_sql();
                nodes.insert(id, row);
            }
            let mut b = Variables::new();
            b.insert(
                "ids",
                nodes
                    .values()
                    .map(|v| {
                        value_object(v)
                            .and_then(|o| o.get("id"))
                            .cloned()
                            .ok_or(ModelError::Schema("native payload id"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
            let mut response = self
                .client
                .query("SELECT * FROM $ids")
                .bind(b)
                .await
                .map_err(|error| {
                    ModelError::codec(format!("compiler physical candidate read: {error}"))
                })?
                .check()
                .map_err(|error| {
                    ModelError::codec(format!("compiler physical candidate read: {error}"))
                })?;
            let existing: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
            for stored in existing {
                let object =
                    value_object(&stored).ok_or(ModelError::Schema("stored native row"))?;
                let id = object
                    .get("id")
                    .ok_or(ModelError::Schema("stored native row id"))?
                    .to_sql();
                if let Some(expected) = nodes.remove(&id) {
                    let expected =
                        value_object(&expected).ok_or(ModelError::Schema("expected native row"))?;
                    for field in [
                        "semantic_type",
                        "semantic_key",
                        "canonical",
                        "body",
                        "kind",
                        "subtype",
                        "content",
                    ] {
                        if expected.get(field) != object.get(field) {
                            return Err(ModelError::Conflict("native same-key payload"));
                        }
                    }
                }
            }
            for table in ["entity", "assertion", "compiler_record"] {
                let rows = nodes
                    .values()
                    .filter(|v| {
                        value_object(v)
                            .and_then(|o| o.get("id"))
                            .is_some_and(|id| id.to_sql().starts_with(&format!("{table}:")))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                Loader::new(self.client.clone())
                    .insert(table, rows, false)
                    .await?;
            }
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
        if rows.is_empty() {
            return Ok(());
        }
        let mut candidates = BTreeMap::new();
        for row in rows {
            let id = value_object(&row)
                .and_then(|row| row.get("id"))
                .ok_or(ModelError::Schema("immutable row key"))?
                .to_sql();
            if let Some(previous) = candidates.insert(id, row.clone())
                && previous != row
            {
                return Err(ModelError::Conflict(conflict));
            }
        }
        for window in crate::loader::NativeWindows::new(candidates.into_values().collect()) {
            let mut candidates = BTreeMap::new();
            for row in window? {
                let id = value_object(&row)
                    .and_then(|row| row.get("id"))
                    .ok_or(ModelError::Schema("immutable row key"))?
                    .to_sql();
                candidates.insert(id, row);
            }
            let ids = candidates
                .values()
                .map(|row| {
                    value_object(row)
                        .expect("checked object")
                        .get("id")
                        .expect("checked key")
                        .clone()
                })
                .collect::<Vec<_>>();
            let mut b = Variables::new();
            b.insert("ids", ids);
            let mut result = self
                .client
                .query("SELECT * FROM $ids")
                .bind(b)
                .await
                .map_err(|error| {
                    ModelError::codec(format!(
                        "compiler immutable {table} candidate read: {error}"
                    ))
                })?
                .check()
                .map_err(|error| {
                    ModelError::codec(format!(
                        "compiler immutable {table} candidate read: {error}"
                    ))
                })?;
            let actual: Vec<Value> = result.take(0).map_err(ModelError::codec)?;
            for row in actual {
                let id = value_object(&row)
                    .and_then(|row| row.get("id"))
                    .ok_or(ModelError::Schema("stored immutable row key"))?
                    .to_sql();
                if candidates.remove(&id) != Some(row) {
                    return Err(ModelError::Conflict(conflict));
                }
            }
            Loader::new(self.client.clone())
                .insert(table, candidates.into_values().collect(), false)
                .await?;
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
        let lease = self.admit(false, "complete_contribution", true)?;
        let result = self
            .complete_contribution_inner(id, outcome, outputs, previous)
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
        self.wait_scans().await?;
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
        if completed {
            return Err(ModelError::Conflict("duplicate contribution completion"));
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
        let response=self.client.query("UPDATE $id SET descriptor=$descriptor,logical=$logical,outcome=$outcome,completed=true WHERE completed=false RETURN VALUE completed").bind(b).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
        let mut response = response;
        let success: Vec<bool> = response.take(0).map_err(ModelError::codec)?;
        if success != vec![true] {
            self.fail();
            return Err(ModelError::Conflict("native completion transition"));
        }
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
            let mut row = Object::new();
            row.insert("id", RecordId::new("compiler_view", view.identity.hex()));
            row.insert(
                "descriptor",
                Bytes::from(serde_json::to_vec(&view).map_err(ModelError::codec)?),
            );
            for (field, value) in view_projection(&view)? {
                row.insert(field, value);
            }
            let mut b = Variables::new();
            b.insert("row", row);
            self.client
                .query("UPSERT $row.id CONTENT $row RETURN NONE")
                .bind(b)
                .await
                .map_err(crate::loader::write_failure)?
                .check()
                .map_err(ModelError::codec)?;
            self.remember_view(&view)?;
            views.insert(relation.name().to_string(), view);
        }
        Ok(views)
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
        let result = self.verify_state_inner().await;
        lease.finish_with(result)
    }
    async fn verify_state_inner(self: &Arc<Self>) -> Result<(), ModelError> {
        // Cold audit/import must reconcile the fixed non-graph backing itself, before
        // comparing membership claims. Ordinary completed-view reads carry their validity.
        let mut backing = self.track_rows(
            NativeRows::new(
                self.client
                    .query("SELECT * FROM compiler_record ORDER BY id")
                    .stream_items()
                    .map_err(ModelError::codec)?,
                1,
            )?
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
        let mut rows=self.track_rows(NativeRows::new(self.client.query("SELECT VALUE id FROM compiler_membership WHERE node.semantic_type IS NONE OR node.semantic_type != relation OR node.semantic_key != semantic_key OR node.content != content OR contribution.completed != true LIMIT 1").stream_items().map_err(ModelError::codec)?,1)?)?;
        if rows.next().await?.is_some() {
            return Err(ModelError::Conflict(
                "completed membership backing/visibility",
            ));
        }
        let mut descriptors = self.track_rows(NativeRows::new(
            self.client
                .query("SELECT * FROM compiler_contribution ORDER BY id")
                .stream_items()
                .map_err(ModelError::codec)?,
            1,
        )?)?;
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
                RecordId::new("compiler_contribution", contribution.spec.identity()?.hex()),
            );
            let mut members=self.track_rows(NativeRows::new(self.client.query("SELECT id,contribution,relation,semantic_key,node,content FROM compiler_membership WHERE contribution=$contribution ORDER BY relation,semantic_key").bind(variables).stream_items().map_err(ModelError::codec)?,1)?)?;
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
            let table = crate::schema::ScopeTable::for_relation(&view.relation)?;
            let relation = table
                .relations()
                .iter()
                .find(|relation| relation.name() == view.relation)
                .ok_or(ModelError::Schema("completed view relation"))?;
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
        let mut response = self
            .client
            .query(
                "SELECT * FROM compiler_contribution WHERE completed=true AND logical IN $logical",
            )
            .bind(bindings)
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
        let mut completed = BTreeMap::new();
        for row in rows {
            validate_state_row("compiler_contribution", &row)?;
            let descriptor = decode_descriptor::<CompletedContribution>(&row)?;
            if completed
                .insert(descriptor.identity()?, descriptor)
                .is_some()
            {
                return Err(ModelError::Conflict("duplicate completed view contributor"));
            }
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
        for (logical, descriptor) in completed {
            self.remember_contributor(logical, descriptor.spec.identity()?)?;
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
        if let Some(old) = known.insert(logical, physical)
            && old != physical
        {
            return Err(ModelError::Conflict(
                "completed contributor identity collision",
            ));
        }
        Ok(())
    }
    fn view_owners(&self, view: &CompletedView) -> Result<Vec<ContentHash>, ModelError> {
        let known = self
            .known_contributors
            .lock()
            .map_err(|_| ModelError::Conflict("native contributor owner"))?;
        view.contributions
            .iter()
            .map(|logical| {
                known
                    .get(logical)
                    .copied()
                    .ok_or(ModelError::Conflict("missing verified contributor"))
            })
            .collect()
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
        let lease = self.admit(false, "bind", true)?;
        let result = self.bind_inner(binding).await;
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
        row.insert("id", RecordId::new("compiler_binding", binding.key().hex()));
        row.insert(
            "descriptor",
            Bytes::from(serde_json::to_vec(&binding).map_err(ModelError::codec)?),
        );
        for (field, value) in binding_projection(&binding) {
            row.insert(field, value);
        }
        let mut b = Variables::new();
        b.insert("row", row);
        if binding.boundary.is_some() {
            let mut lookup = Variables::new();
            lookup.insert("id", RecordId::new("compiler_binding", binding.key().hex()));
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
        self.client
            .query("UPSERT $row.id CONTENT $row RETURN NONE")
            .bind(b)
            .await
            .map_err(crate::loader::write_failure)?
            .check()
            .map_err(ModelError::codec)?;
        Ok(())
    }
    pub async fn bindings(self: &Arc<Self>) -> Result<Vec<CompletedBinding>, ModelError> {
        let lease = self.admit(false, "bindings", false)?;
        let result = self.bindings_inner().await;
        lease.finish_with(result)
    }
    async fn bindings_inner(self: &Arc<Self>) -> Result<Vec<CompletedBinding>, ModelError> {
        let mut stream = self.track_rows(NativeRows::new(
            self.client
                .query("SELECT * FROM compiler_binding ORDER BY id")
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
        let result = self.views_inner().await;
        lease.finish_with(result)
    }
    async fn views_inner(self: &Arc<Self>) -> Result<Vec<CompletedView>, ModelError> {
        let mut rows = self.track_rows(NativeRows::new(
            self.client
                .query("SELECT * FROM compiler_view ORDER BY id")
                .stream_items()
                .map_err(ModelError::codec)?,
            1,
        )?)?;
        let mut views = Vec::new();
        while let Some(row) = rows.next().await? {
            validate_state_row("compiler_view", &row)?;
            let object = value_object(&row).ok_or(ModelError::Schema("native completed view"))?;
            let Some(Value::Bytes(bytes)) = object.get("descriptor") else {
                return Err(ModelError::Schema("native completed view descriptor"));
            };
            let view: CompletedView = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            view.validate()?;
            views.push(view);
        }
        Ok(views)
    }
    pub async fn completed_state(self: &Arc<Self>) -> Result<CompletedStateIdentity, ModelError> {
        let lease = self.admit(false, "completed_state", false)?;
        let result = self.completed_state_inner().await;
        lease.finish_with(result)
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
        let mut sink = KeySink::new("native-completed-state/v2");
        let mut counts = [0u64; 6];
        for (i, table) in STATE_TABLES.iter().enumerate() {
            table.to_string().encode(&mut sink);
            let mut rows = self.track_rows(
                NativeRows::new(
                    self.client
                        .query(format!("SELECT * FROM {table} ORDER BY id"))
                        .stream_items()
                        .map_err(ModelError::codec)?,
                    1,
                )?
                .with_row_bytes(d::resources::MAX_ROW_BYTES),
            )?;
            let mut pending = Vec::new();
            let mut pending_bytes = 0;
            while let Some(row) = rows.next().await? {
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
        let result = self.export_state_inner(path).await;
        lease.finish_with(result)
    }
    async fn export_state_inner(
        self: &Arc<Self>,
        path: &std::path::Path,
    ) -> Result<CompletedStateIdentity, ModelError> {
        use std::io::Write;
        let expected = self.completed_state_inner().await?;
        let mut file =
            std::io::BufWriter::new(std::fs::File::create(path).map_err(ModelError::codec)?);
        serde_json::to_writer(&mut file, &CompletedStateHeader::current())
            .map_err(ModelError::codec)?;
        file.write_all(b"\n").map_err(ModelError::codec)?;
        for table in STATE_TABLES {
            let mut rows = self.track_rows(
                NativeRows::new(
                    self.client
                        .query(format!("SELECT * FROM {table} ORDER BY id"))
                        .stream_items()
                        .map_err(ModelError::codec)?,
                    1,
                )?
                .with_row_bytes(d::resources::MAX_ROW_BYTES),
            )?;
            while let Some(row) = rows.next().await? {
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
        let lease = self.admit(false, "import_state", true)?;
        let result = self.import_state_inner(path, expected).await;
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
        self.verify_state_inner().await?;
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
        Loader::new(self.client.clone())
            .insert(table, rows, false)
            .await
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
        let mut sorted = AsyncCandidateSort::new_registered(
            budget,
            self.blocking_owner()?,
            self.blocking_observer()?,
        )
        .await?;
        if let Some((field, values)) = atomic_field {
            let table = crate::schema::ScopeTable::for_relation(relation.name())?.name();
            for value in values {
                let mut bindings = Variables::new();
                bindings.insert("relation", relation.name().to_string());
                bindings.insert("value", value);
                let constant = crate::prepared::scope_constant("$relation", &field, "$value");
                let sql = format!(
                    "LET $__compiler_scope = {constant}; SELECT semantic_type AS relation,semantic_key,id AS node FROM {table} WITH INDEX by_scope WHERE scope_keys CONTAINS $__compiler_scope AND semantic_type=$relation"
                );
                let mut rows = self.track_rows(prepared_rows(&self.client, sql, bindings, 2)?)?;
                while let Some(row) = rows.next_native().await? {
                    sorted.push(decode_candidate(row, relation.name())?).await?;
                }
            }
        } else {
            for owner in self.view_owners(view)? {
                let mut bindings = Variables::new();
                bindings.insert("relation", relation.name().to_string());
                bindings.insert("owner", RecordId::new("compiler_contribution", owner.hex()));
                let mut rows=self.track_rows(NativeRows::new(self.client.query("SELECT relation,semantic_key,node FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation").bind(bindings).stream_items().map_err(ModelError::codec)?,1)?)?;
                while let Some(row) = rows.next_native().await? {
                    sorted.push(decode_candidate(row, relation.name())?).await?;
                }
            }
        }
        self.track_selection(
            SelectionKeys::Ordered(sorted.finish().await?),
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
    client: &Surreal<Client>,
    sql: String,
    bindings: Variables,
    statements: usize,
) -> Result<NativeRows, ModelError> {
    crate::prepared::PreparedQuery::from_sql(sql, bindings, statements, vec![statements - 1])?
        .stream(client)
}

type ReadSetupCharge = Arc<Box<dyn Reservation>>;
pub struct CompilerRows {
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
            self.store.runtime.spawn(async move {
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
            });
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
/// One bounded nominal-key window. Physical owner × key IDs are generated and read in
///128-record windows, with exact pointer deduplication before any rich payload query.
struct MembershipLookup {
    store: Arc<NativeCompilerStore>,
    relation: String,
    keys: Vec<[u8; 16]>,
    owners: Vec<ContentHash>,
    offset: usize,
    current: Option<CompilerRows>,
    nodes: BTreeMap<String, RecordId>,
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
    async fn collect(&mut self) -> Result<BTreeMap<String, RecordId>, ModelError> {
        loop {
            if let Some(rows) = &mut self.current {
                while let Some(row) = rows.next_native().await? {
                    let object = value_object(&row)
                        .ok_or(ModelError::Schema("selected membership pointer"))?;
                    let Some(Value::String(key)) = object.get("semantic_key") else {
                        return Err(ModelError::Schema("selected nominal key"));
                    };
                    if self.keys.binary_search(&decode_nominal_key(key)?).is_err() {
                        return Err(ModelError::Conflict("selected membership key"));
                    }
                    let Some(Value::RecordId(node)) = object.get("node") else {
                        return Err(ModelError::Schema("selected physical pointer"));
                    };
                    if let Some(old) = self.nodes.insert(key.clone(), node.clone())
                        && old != *node
                    {
                        return Err(ModelError::Conflict("selected nominal backing collision"));
                    }
                }
                self.current.take();
            }
            let total = self
                .keys
                .len()
                .checked_mul(self.owners.len())
                .ok_or(ModelError::Schema("native membership demand"))?;
            if self.offset == total {
                return Ok(std::mem::take(&mut self.nodes));
            }
            let end = self
                .offset
                .saturating_add(crate::loader::NATIVE_WINDOW_ROWS)
                .min(total);
            let ids = (self.offset..end)
                .map(|position| {
                    membership_id(
                        self.owners[position / self.keys.len()],
                        &self.relation,
                        &self.keys[position % self.keys.len()],
                    )
                })
                .collect::<Vec<_>>();
            let mut bindings = Variables::new();
            bindings.insert("ids", ids);
            bindings.insert("relation", self.relation.clone());
            let stream=self.store.client.query("SELECT semantic_key,node FROM $ids WHERE relation=$relation ORDER BY semantic_key").bind(bindings).stream_items().map_err(ModelError::codec)?;
            self.current = Some(
                self.store
                    .track_rows(NativeRows::new(stream, 1)?)?
                    .with_setup(self.setup.clone()),
            );
            self.offset = end;
        }
    }
}
enum SelectionKeys {
    Known(std::vec::IntoIter<[u8; 16]>),
    Ordered(AsyncOrderedCandidates),
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
    async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        loop {
            let window_rows = (crate::loader::NATIVE_WINDOW_ROWS / self.owners.len().max(1)).max(1);
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
                        && expected != node
                    {
                        return Err(ModelError::Conflict("selected candidate backing collision"));
                    }
                }
                self.pointers.clear();
                if nodes.is_empty() {
                    continue;
                }
                let mut bindings = self.bindings.clone();
                bindings.insert("__compiler_nodes", nodes.into_values().collect::<Vec<_>>());
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
                continue;
            }
            match &mut self.keys {
                SelectionKeys::Known(source) => {
                    self.pending_keys.extend(source.by_ref().take(window_rows))
                }
                SelectionKeys::Ordered(source) => {
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
                let Some(Value::RecordId(node)) =
                    value_object(&pointer).and_then(|row| row.get("id"))
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
                                .query(format!("SELECT {} FROM $nodes ORDER BY id", self.fields))
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
fn pointer_row(node: RecordId) -> Value {
    let mut row = Object::new();
    row.insert("id", node);
    Value::Object(row)
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
        relation: actual.clone(),
        key: decode_nominal_key(key)?,
        node: node.clone(),
    })
}

#[derive(Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "Homogeneous canonical-record batches remain contiguous; boxing adds a per-row allocation"
)]
enum GraphRow {
    Entity(d::graph::Entity),
    Assertion(d::graph::Assertion),
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
            || id.key != surrealdb::types::RecordIdKey::String(descriptor.spec.identity()?.hex())
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
        if id.key != surrealdb::types::RecordIdKey::String(descriptor.key().hex()) {
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
        let relation = crate::schema::compiler_relations()
            .iter()
            .find(|relation| relation.name() == name)
            .ok_or(ModelError::Schema("undeclared compiler backing type"))?;
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
    let mut sink = KeySink::new("compiler-backing-key/v1");
    name.encode(&mut sink);
    sink.part(b"key", &key);
    if row.get("semantic_type") != Some(&Value::String(name.clone()))
        || row.get("semantic_key") != Some(&Value::String(hex::encode(key)))
        || *id != RecordId::new("compiler_record", sink.finish().hex())
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
    let mut expected = Object::new();
    crate::reconciliation::add_scope_fields(
        &mut expected,
        &canonical_body,
        name,
        crate::schema::ScopeTable::CompilerRecord,
    )?;
    let actual = row
        .iter()
        .filter(|(field, _)| field.starts_with("scope_"))
        .map(|(field, value)| (field.clone(), value.clone()))
        .collect::<Object>();
    if actual != expected {
        return Err(ModelError::Conflict("compiler backing derived scope"));
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
        let relation = crate::schema::compiler_relations()
            .iter()
            .find(|relation| relation.name() == name)
            .ok_or(ModelError::Schema("undeclared compiler backing type"))?;
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
            if row.get("semantic_key") != Some(&Value::String(hex::encode(ids.value(index)))) {
                return Err(ModelError::Conflict("compiler backing typed identity"));
            }
            let Some(Value::Bytes(canonical)) = row.get("canonical") else {
                return Err(ModelError::Schema("compiler backing canonical bytes"));
            };
            if canonical.as_ref()
                != serde_json::to_vec(&body)
                    .map_err(ModelError::codec)?
                    .as_slice()
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
}

#[cfg(test)]
mod compiler_scope_tests {
    use super::*;
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
        let mut sink = KeySink::new("compiler-backing-key/v1");
        relation.name().to_string().encode(&mut sink);
        sink.part(b"key", record.id().bytes());
        let id = RecordId::new("compiler_record", sink.finish().hex());
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
        row.insert("scope_run", "retired scalar");
        assert!(validate_compiler_backing(&row, &id).is_err());
        row.remove("scope_run");
        row.insert("scope_keys", vec!["imported wrong key"]);
        assert!(validate_compiler_backing(&row, &id).is_err());
    }
}
