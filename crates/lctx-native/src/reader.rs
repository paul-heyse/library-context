use futures::{FutureExt, StreamExt};
pub(crate) mod ingress;
mod retained;
use lctx_model::domain::{
    Key, KeySink, ModelError, Record,
    graph::{Assertion, Entity, Target},
    serving::SnapshotHandle,
};
pub(crate) use retained::{materialize_native_bytes, native_retained_bytes};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;
use surrealdb::{
    Surreal,
    engine::remote::grpc::{Client, Grpc},
    opt::auth::{Database, Root},
    types::{Bytes, RecordId, SerdeWrapper, SurrealValue, Value, Variables},
};

/// Secrets are supplied by the operator; this type deliberately has no Debug or Serialize.
pub enum Credentials {
    Root { username: String, password: String },
    Database { username: String, password: String },
}
/// Finite inputs supplied by a model-owned operation, never a request-controlled SQL predicate.
pub enum RecordSelection {
    Keys(Vec<[u8; 16]>),
    Scope {
        field: String,
        values: Vec<serde_json::Value>,
    },
    Connected {
        targets: Vec<Target>,
        fields: Vec<String>,
    },
}
#[derive(Clone)]
pub struct NativeReader<Context = SnapshotHandle> {
    operations: Arc<std::sync::Mutex<QueryOperations>>,
    client: Arc<Surreal<Client>>,
    handle: Context,
    derivation: Option<lctx_model::domain::ContentHash>,
    scope: Option<Arc<ReaderScope>>,
    cancellation: Option<Arc<std::sync::atomic::AtomicBool>>,
    budget: Option<lctx_model::domain::resources::ResourceBudget>,
    request_budget: Option<lctx_model::domain::resources::ResourceBudget>,
}
/// Protect a publication pin through unexpected unwind and async finalizer abandonment.
/// This holds pin and session owners without creating another reader/scope borrower.
pub struct ReadFinalizationGuard {
    client: Arc<Surreal<Client>>,
    pin: Option<Arc<crate::control::ReaderPin>>,
    installed: Option<Arc<ExactViewSelection>>,
    closed: bool,
}
impl ReadFinalizationGuard {
    pub async fn close<Context>(
        &mut self,
        reader: &NativeReader<Context>,
    ) -> Result<(), ModelError> {
        if !Arc::ptr_eq(&self.client, &reader.client) {
            return Err(ModelError::Conflict("reader finalization session owner"));
        }
        let actual = reader.scope.as_ref().and_then(|scope| scope._pin.as_ref());
        if !match (&self.pin, actual) {
            (Some(pin), Some(actual)) => Arc::ptr_eq(pin, actual),
            (None, None) => true,
            _ => false,
        } {
            return Err(ModelError::Conflict("reader finalization guard owner"));
        }
        let actual = reader.scope.as_ref().and_then(|scope| scope.exact.as_ref());
        if !match (&self.installed, actual) {
            (Some(expected), Some(actual)) => Arc::ptr_eq(expected, actual),
            (None, None) => true,
            _ => false,
        } {
            return Err(ModelError::Conflict(
                "reader finalization guard installed owner",
            ));
        }
        reader.close().await?;
        self.closed = true;
        Ok(())
    }
}
impl Drop for ReadFinalizationGuard {
    fn drop(&mut self) {
        if !self.closed {
            if let Some(pin) = &self.pin {
                pin.retain_unknown();
            }
            if let Some(exact) = &self.installed {
                for view in &exact._installed {
                    view.retain_unknown();
                }
            }
            std::mem::forget(self.client.clone());
        }
    }
}
struct ExactViewSelection {
    references: Vec<crate::completed_state::InstalledViewRef>,
    // Live/compiler selections retain the capabilities that chose these copies.
    // Published/completed selections instead retain their enclosing pin/page owner.
    _installed: Vec<Arc<crate::installed_view::HeldInstalledView>>,
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
}
/// Actual source ownership for one operation-local reader. This is deliberately
/// separate from ReaderScope so mechanical cached selection cannot retain its scope.
pub(crate) struct BorrowedInstalledGuard {
    client: Arc<Surreal<Client>>,
    pins: Vec<Arc<crate::control::ReaderPin>>,
    context: (lctx_model::domain::ContentHash, i64),
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
}
impl BorrowedInstalledGuard {
    pub(crate) fn ensure_live(&self) -> Result<(), ModelError> {
        // Native validity maintenance requires every pin to be released. Actual
        // setup/terminal context observations therefore remain current beneath
        // these pins; the compiler's admitted lease owns source-close exclusion.
        for pin in &self.pins {
            pin.ensure_live()?;
        }
        Ok(())
    }
    pub(crate) fn retain_unknown(&self) {
        for pin in &self.pins {
            pin.retain_unknown();
        }
    }
    pub(crate) async fn ensure_current(
        &self,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        self.ensure_live()?;
        let _scratch = budget.reserve("borrowed reader current installation", 8192)?;
        let reader = NativeReader::private(self.client.clone()).with_budget(budget);
        let observed = async {
            let rows: Vec<surrealdb::types::Object> = reader
                .query_native(
                    "SELECT generation,validity_epoch FROM native_installation:current",
                    Variables::new(),
                )
                .await?;
            let [row] = rows.as_slice() else {
                return Err(ModelError::Conflict(
                    "borrowed reader installation cardinality",
                ));
            };
            if row.get("generation") != Some(&Value::String(self.context.0.hex()))
                || row.get("validity_epoch")
                    != Some(&Value::Number(surrealdb::types::Number::Int(
                        self.context.1,
                    )))
            {
                return Err(ModelError::Conflict("borrowed reader installation changed"));
            }
            self.ensure_live()
        }
        .await;
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step(
            "borrowed reader context transport close",
            reader.close().await,
        );
        let result = lctx_model::domain::completion::complete(observed, terminal);
        if result
            .as_ref()
            .err()
            .is_some_and(|error| !error.permits_storage_cleanup())
        {
            self.retain_unknown();
        }
        result
    }
}
// Own the receiving session before publication. An abandoned or unconfirmed
// setup retains the actual SDK owner as well as its physical closure; dropping
// the last session handle would otherwise enqueue a best-effort detach.
struct InstalledReaderSetup {
    views: Vec<Arc<crate::installed_view::HeldInstalledView>>,
    pin: Option<Arc<crate::control::ReaderPin>>,
    client: Arc<Surreal<Client>>,
    confirmed: bool,
}
impl InstalledReaderSetup {
    fn permits_cleanup(&self, error: &ModelError) -> bool {
        if error.permits_storage_cleanup() {
            return true;
        }
        self.retain_unknown();
        false
    }
    fn retain_unknown(&self) {
        if let Some(pin) = &self.pin {
            pin.retain_unknown();
        }
        for view in &self.views {
            view.retain_unknown();
        }
    }
}
impl Drop for InstalledReaderSetup {
    fn drop(&mut self) {
        if !self.confirmed {
            self.retain_unknown();
            std::mem::forget(self.client.clone());
        }
    }
}
type DrainCompletion =
    futures::future::Shared<futures::future::BoxFuture<'static, Result<(), Arc<ModelError>>>>;
struct SelectionPreparation {
    budget: lctx_model::domain::resources::ResourceBudget,
    derivation: Option<lctx_model::domain::ContentHash>,
    pending: futures::future::Shared<
        futures::future::BoxFuture<
            'static,
            Result<crate::selection::SelectedPayloads, Arc<ModelError>>,
        >,
    >,
}
struct ReaderScope {
    publication: Option<SnapshotHandle>,
    exact: Option<Arc<ExactViewSelection>>,
    originals: Option<Vec<lctx_model::domain::graph::Original>>,
    views: Vec<lctx_model::domain::ContentHash>,
    _pin: Option<Arc<crate::control::ReaderPin>>,
    release_pin_on_close: bool,
    borrowed: Option<Arc<BorrowedInstalledGuard>>,
    selection: std::sync::Mutex<Option<SelectionPreparation>>,
}
impl ReaderScope {
    fn retain_unknown(&self) {
        if let Some(borrowed) = &self.borrowed {
            borrowed.retain_unknown();
        }
        if let Some(pin) = &self._pin {
            pin.retain_unknown();
        }
        if let Some(exact) = &self.exact {
            for view in &exact._installed {
                view.retain_unknown();
            }
        }
    }
    fn ensure_live(&self) -> Result<(), ModelError> {
        if let Some(borrowed) = &self.borrowed {
            borrowed.ensure_live()?;
        }
        if let Some(pin) = &self._pin {
            pin.ensure_live()?;
        }
        Ok(())
    }
    fn require_canonical_selection(&self) -> Result<(), ModelError> {
        if self.exact.is_none() && self.publication.is_none() {
            return Err(ModelError::Conflict(
                "canonical selection requires protected exact installed views",
            ));
        }
        Ok(())
    }
}
#[derive(Default)]
struct QueryOperations {
    recovery: Option<Arc<crate::control::RecoveryScope>>,
    signals: Option<Arc<crate::signal::SignalRegistration>>,
    factory: Option<crate::control::FactoryLease>,
    owns_session: bool,
    session_closed: bool,
    active: usize,
    drains: Vec<DrainCompletion>,
    closed: bool,
    // Abandoned delivery has a reader-owned terminal receipt, rather than a log-only result.
    abandoned: Vec<Result<(), Arc<ModelError>>>,
}
/// Erase the finalization owner's recursive type graph while retaining the
/// actual query registry. This owner has no source factory lease.
pub(crate) trait RecoveryReadOwner: Send + Sync {
    fn reader(&self) -> NativeReader<()>;
    fn close(&self) -> futures::future::BoxFuture<'_, Result<(), ModelError>>;
    fn terminal(&self) -> Result<bool, ModelError>;
}
struct PrivateRecoveryRead(NativeReader<()>);
impl RecoveryReadOwner for PrivateRecoveryRead {
    fn reader(&self) -> NativeReader<()> {
        self.0.clone()
    }
    fn close(&self) -> futures::future::BoxFuture<'_, Result<(), ModelError>> {
        self.0.close().boxed()
    }
    fn terminal(&self) -> Result<bool, ModelError> {
        let state = self
            .0
            .operations
            .lock()
            .map_err(|_| ModelError::Conflict("native finalization read owner"))?;
        Ok(state.closed && state.active == 0)
    }
}
pub(crate) fn recovery_read_owner(
    client: Arc<Surreal<Client>>,
    budget: &lctx_model::domain::resources::ResourceBudget,
    cancellation: Arc<std::sync::atomic::AtomicBool>,
) -> Arc<dyn RecoveryReadOwner> {
    Arc::new(PrivateRecoveryRead(
        NativeReader::private(client)
            .with_budget(budget)
            .with_read_cancellation(cancellation),
    ))
}
struct QueryLease {
    operations: Arc<std::sync::Mutex<QueryOperations>>,
    _client: Arc<Surreal<Client>>,
    scope: Option<Arc<ReaderScope>>,
    finished: bool,
    confirmed: bool,
}
impl QueryLease {
    fn admit(
        operations: Arc<std::sync::Mutex<QueryOperations>>,
        client: Arc<Surreal<Client>>,
        scope: Option<Arc<ReaderScope>>,
    ) -> Result<Self, ModelError> {
        {
            let mut state = operations
                .lock()
                .map_err(|_| ModelError::Conflict("native query operation owner"))?;
            if state.closed {
                return Err(ModelError::Conflict("native reader closed"));
            }
            state.active += 1;
        }
        Ok(Self {
            operations,
            _client: client,
            scope,
            finished: false,
            confirmed: false,
        })
    }
    fn finish(&mut self, outcome: &Result<(), Arc<ModelError>>, abandoned: bool) {
        if outcome
            .as_ref()
            .is_err_and(|error| !error.permits_storage_cleanup())
            && let Some(scope) = &self.scope
        {
            scope.retain_unknown();
        }
        if let Ok(mut state) = self.operations.lock() {
            if abandoned
                || outcome
                    .as_ref()
                    .is_err_and(|error| !error.permits_storage_cleanup())
            {
                state.abandoned.push(outcome.clone());
            }
            state.active -= 1;
        }
        self.finished = true;
    }
}
impl Drop for QueryLease {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if self.confirmed {
            self.finish(&Ok(()), false);
            return;
        }
        if let Some(scope) = &self.scope {
            scope.retain_unknown();
        }
        // Runtime shutdown/panic cannot establish execution finality. Preserve the actual
        // session and a sticky unknown receipt; no local EOF is manufactured.
        std::mem::forget(self._client.clone());
        if let Ok(mut state) = self.operations.lock() {
            state.active -= 1;
            state
                .abandoned
                .push(Err(Arc::new(ModelError::infrastructure(
                    lctx_model::domain::Infrastructure::Unconfirmed,
                    "native query lost drain owner",
                ))));
        }
    }
}
/// Visible native payload and container slots. This is the admitted representation,
/// not allocator RSS: BTree allocator metadata, shared backing buffers and compiled regex
/// internals are owned by the SDK, as are transport queues before delivery. All value kinds
/// contribute their visible payload; no silent dynamic-kind fallback or guessed row maximum.
struct ChargedNativeValue {
    value: Option<Value>,
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
    _lease: QueryLease,
}
impl ChargedNativeValue {
    fn take_value(&mut self) -> Value {
        self.value.take().expect("native value decoded once")
    }
}
enum NativeValueShape {
    Unset,
    Rows(Vec<Value>),
    Single(Value),
}
struct AcceptedValue {
    stream: futures::stream::BoxStream<
        'static,
        Result<surrealdb::method::AdmittedBatchItem, surrealdb::method::AdmittedBatchFailure>,
    >,
    lease: QueryLease,
    budget: lctx_model::domain::resources::ResourceBudget,
    charge: Box<dyn lctx_model::domain::resources::Reservation>,
    statements: usize,
    selected: usize,
    ended: usize,
    // Shape is checked for every statement, even an ignored preparation statement.
    statement_shape: u8,
    abandoned_delivery: bool,
    value: NativeValueShape,
    payload_bytes: usize,
    primary: Option<ModelError>,
    completion: lctx_model::domain::completion::Completion,
    exhausted: bool,
}
impl AcceptedValue {
    fn fail(&mut self, error: ModelError) {
        if self.primary.is_none() {
            self.primary = Some(error);
        } else {
            self.completion
                .step("native exact query late failure", Err(error));
        }
    }
    fn ingest(
        &mut self,
        item: surrealdb::Result<surrealdb::method::BatchItem>,
    ) -> Result<(), ModelError> {
        use surrealdb::method::BatchItem;
        let item = item.map_err(|error| unconfirmed_stream(sdk_error(error)))?;
        let index = item.index();
        let valid = index == self.ended && index < self.statements;
        // Consume errors even on malformed terminals, so shape/order refusal does not hide
        // a statement's own typed failure.
        if let BatchItem::Finished {
            query_type, error, ..
        } = item
        {
            if !valid {
                self.fail(unconfirmed_stream(ModelError::Schema(
                    "native exact query terminal order",
                )));
            } else {
                self.ended += 1;
                self.statement_shape = 0;
            }
            if query_type != surrealdb::method::BatchQueryType::Other {
                self.fail(ModelError::Schema(
                    "native finite query subscription statement",
                ));
            }
            if let Some(error) = error {
                self.fail(sdk_error(error));
            }
            return Ok(());
        }
        if !valid {
            return Err(unconfirmed_stream(ModelError::Schema(
                "native exact query payload order",
            )));
        }
        let shape = match &item {
            BatchItem::Rows { .. } => 1,
            BatchItem::Value { .. } | BatchItem::EmptyValue { .. } => 2,
            BatchItem::Finished { .. } => unreachable!(),
        };
        if self.statement_shape != 0 && (self.statement_shape != shape || shape == 2) {
            return Err(unconfirmed_stream(ModelError::Schema(
                "native exact query mixed/repeated value shape",
            )));
        }
        self.statement_shape = shape;
        if index != self.selected || self.primary.is_some() || self.abandoned_delivery {
            return Ok(());
        }
        match item {
            BatchItem::Rows { mut values, .. } => {
                // Transfer the qualified retained allocation into the application owner while
                // the SDK framing, conversion and batch reservations still overlap it.
                let bytes = values.iter().try_fold(0usize, |total, value| {
                    total
                        .checked_add(native_retained_bytes(value)?)
                        .ok_or(ModelError::Schema("native retained batch size overflow"))
                })?;
                let slots = values
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Value>());
                let _ingress = self.budget.reserve(
                    "native exact query delivered batch",
                    bytes.saturating_add(slots),
                )?;
                if matches!(self.value, NativeValueShape::Unset) {
                    self.value = NativeValueShape::Rows(Vec::new());
                }
                let NativeValueShape::Rows(rows) = &mut self.value else {
                    return Err(ModelError::Schema("native exact query result shape"));
                };
                let count = rows
                    .len()
                    .checked_add(values.len())
                    .ok_or(ModelError::Schema("native exact query result overflow"))?;
                let capacity = if count > rows.capacity() {
                    count.max(rows.capacity().saturating_mul(2))
                } else {
                    rows.capacity()
                };
                // Amortized vector growth is an explicit charged allocation, not a guessed
                // query allowance. Per-value inline slots also cover the final Array
                // conversion's possible second vector; arbitrary T decoding is caller-owned.
                let target = self
                    .payload_bytes
                    .saturating_add(bytes)
                    .saturating_add(capacity.saturating_mul(std::mem::size_of::<Value>()))
                    .saturating_add(std::mem::size_of::<Value>());
                self.charge.try_resize(target)?;
                // Construct the planned capacity under reservation. Old and new
                // vector allocations coexist and are independently charged during growth.
                if count > rows.capacity() {
                    let _old = self.budget.reserve(
                        "native exact query vector relocation",
                        rows.capacity().saturating_mul(std::mem::size_of::<Value>()),
                    )?;
                    let mut replacement = Vec::with_capacity(capacity);
                    self.charge.try_resize(
                        self.payload_bytes
                            .saturating_add(bytes)
                            .saturating_add(
                                replacement
                                    .capacity()
                                    .saturating_mul(std::mem::size_of::<Value>()),
                            )
                            .saturating_add(std::mem::size_of::<Value>()),
                    )?;
                    replacement.append(rows);
                    *rows = replacement;
                }
                for value in &mut values {
                    materialize_native_bytes(value);
                }
                rows.extend(values);
                self.payload_bytes = self.payload_bytes.saturating_add(bytes);
            }
            BatchItem::Value { mut value, .. } => {
                let bytes = native_retained_bytes(&value)?;
                self.charge.try_resize(bytes)?;
                materialize_native_bytes(&mut value);
                self.value = NativeValueShape::Single(value);
                self.payload_bytes = bytes;
            }
            BatchItem::EmptyValue { .. } => {
                self.value = NativeValueShape::Single(Value::None);
            }
            BatchItem::Finished { .. } => unreachable!(),
        }
        Ok(())
    }
    fn ingest_admitted(
        &mut self,
        item: Result<surrealdb::method::AdmittedBatchItem, surrealdb::method::AdmittedBatchFailure>,
    ) -> Result<(), ModelError> {
        use surrealdb::method::BatchItem;
        let (item, reservations) = item
            .map_err(|failure| {
                let (error, reservations) = failure.into_parts();
                ModelError::Cause(Box::new(ingress::LeasedFailure {
                    source: unconfirmed_stream(sdk_error(error)),
                    _reservations: reservations,
                }))
            })?
            .into_parts();
        match item {
            BatchItem::Finished {
                index,
                query_type,
                time,
                error: Some(error),
            } => {
                // Preserve both malformed order and the statement's own typed
                // failure. Its allocations stay admitted through result ownership.
                let result = self.ingest(Ok(BatchItem::Finished {
                    index,
                    query_type,
                    time,
                    error: None,
                }));
                self.fail(ModelError::Cause(Box::new(ingress::LeasedFailure {
                    source: sdk_error(error),
                    _reservations: reservations,
                })));
                result
            }
            item => {
                let result = self.ingest(Ok(item));
                // Retained values now have the accumulation reservation; keep
                // ingress/native/protobuf overlap live through that transfer.
                drop(reservations);
                result
            }
        }
    }
    async fn drain(&mut self, flag: Option<Arc<std::sync::atomic::AtomicBool>>) {
        let mut cancelled = false;
        while let Some(item) = self.stream.next().await {
            if !cancelled
                && let Err(error) = crate::prepared::check_read_cancellation(flag.as_ref())
            {
                self.fail(error);
                cancelled = true;
            }
            if let Err(error) = self.ingest_admitted(item) {
                self.fail(error);
            }
        }
        if !cancelled && let Err(error) = crate::prepared::check_read_cancellation(flag.as_ref()) {
            self.fail(error);
        }
        self.exhausted = true;
        if self.ended != self.statements {
            self.fail(ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed,
                "native exact query missing terminal",
            ));
        }
    }
    fn outcome(&mut self) -> Result<(), Arc<ModelError>> {
        let result = self.primary.take().map_or(Ok(()), Err);
        lctx_model::domain::completion::complete(result, std::mem::take(&mut self.completion))
            .map_err(Arc::new)
    }
}
struct NativeValueOwner {
    accepted: Option<AcceptedValue>,
}
impl NativeValueOwner {
    #[cfg(test)]
    fn new(
        stream: impl futures::Stream<Item = surrealdb::Result<surrealdb::method::BatchItem>>
        + Send
        + 'static,
        statements: usize,
        selected: usize,
        budget: lctx_model::domain::resources::ResourceBudget,
        charge: Box<dyn lctx_model::domain::resources::Reservation>,
        lease: QueryLease,
    ) -> Self {
        Self::new_admitted(
            stream.map(|item| {
                item.map(|item| surrealdb::method::AdmittedBatchItem::new(item, Vec::new()))
                    .map_err(|error| {
                        surrealdb::method::AdmittedBatchFailure::new(error, Vec::new())
                    })
            }),
            statements,
            selected,
            budget,
            charge,
            lease,
        )
    }
    fn new_admitted(
        stream: impl futures::Stream<
            Item = Result<
                surrealdb::method::AdmittedBatchItem,
                surrealdb::method::AdmittedBatchFailure,
            >,
        > + Send
        + 'static,
        statements: usize,
        selected: usize,
        budget: lctx_model::domain::resources::ResourceBudget,
        charge: Box<dyn lctx_model::domain::resources::Reservation>,
        lease: QueryLease,
    ) -> Self {
        Self {
            accepted: Some(AcceptedValue {
                stream: stream.boxed(),
                lease,
                budget,
                charge,
                statements,
                selected,
                ended: 0,
                statement_shape: 0,
                abandoned_delivery: false,
                value: NativeValueShape::Unset,
                payload_bytes: 0,
                primary: None,
                completion: Default::default(),
                exhausted: false,
            }),
        }
    }
    async fn collect(
        &mut self,
        flag: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<ChargedNativeValue, ModelError> {
        let accepted = self.accepted.as_mut().expect("accepted query owner");
        accepted.drain(flag).await;
        let outcome = accepted.outcome();
        if outcome.is_err() {
            accepted.lease.finish(&outcome, false);
        } else {
            accepted.lease.confirmed = true;
        }
        let accepted = self.accepted.take().expect("accepted query owner");
        outcome.map_err(ModelError::SharedCause)?;
        let value = match accepted.value {
            NativeValueShape::Single(value) => value,
            NativeValueShape::Rows(rows) => Value::Array(surrealdb::types::Array::from(rows)),
            NativeValueShape::Unset => Value::Array(surrealdb::types::Array::new()),
        };
        Ok(ChargedNativeValue {
            value: Some(value),
            _charge: accepted.charge,
            _lease: accepted.lease,
        })
    }
}
impl Drop for NativeValueOwner {
    fn drop(&mut self) {
        let Some(mut accepted) = self.accepted.take() else {
            return;
        };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            if let Some(scope) = &accepted.lease.scope {
                scope.retain_unknown();
            }
            // Preserve the actual stream/session/charge when no executor can drain it.
            std::mem::forget(accepted);
            return;
        };
        // No consumer can observe further selected values. Preserve retained state and its
        // charge through terminality, but don't accumulate new output merely to discard it.
        accepted.abandoned_delivery = true;
        let operations = accepted.lease.operations.clone();
        let task = runtime.spawn(async move {
            accepted.drain(None).await;
            let outcome = accepted.outcome();
            accepted.lease.finish(&outcome, true);
        });
        let task = async move {
            task.await.map_err(|error| {
                Arc::new(ModelError::infrastructure(
                    lctx_model::domain::Infrastructure::Unconfirmed,
                    format!("native query drain task: {error}"),
                ))
            })
        }
        .boxed()
        .shared();
        if let Ok(mut state) = operations.lock() {
            state.drains.push(task);
        }
    }
}
#[cfg(test)]
mod query_wire_controls;

#[cfg(test)]
mod accepted_query_controls {
    use super::*;
    use lctx_model::domain::resources::ResourceBudget;
    use surrealdb::method::{BatchItem, BatchQueryType};

    fn terminal(index: usize, error: Option<surrealdb::Error>) -> surrealdb::Result<BatchItem> {
        Ok(BatchItem::Finished {
            index,
            time: Default::default(),
            query_type: BatchQueryType::Other,
            error,
        })
    }
    fn owner(
        reader: &NativeReader<()>,
        stream: impl futures::Stream<Item = surrealdb::Result<BatchItem>> + Send + 'static,
        statements: usize,
        selected: usize,
    ) -> NativeValueOwner {
        let budget = reader.scratch_budget().unwrap();
        let charge = budget
            .reserve("accepted query control", std::mem::size_of::<Value>())
            .unwrap();
        NativeValueOwner::new(
            stream,
            statements,
            selected,
            budget,
            charge,
            QueryLease::admit(
                reader.operations.clone(),
                reader.client.clone(),
                reader.scope.clone(),
            )
            .unwrap(),
        )
    }
    fn reader(budget: &ResourceBudget) -> NativeReader<()> {
        NativeReader::private(Arc::new(Surreal::<Client>::init())).with_budget(budget)
    }
    #[tokio::test]
    async fn exact_batch_shapes_and_non_none_preparation_match_buffered_values() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let reader = reader(&budget);
        let values = vec![
            Value::None,
            Value::Null,
            7i64.into_value(),
            surrealdb::types::object! { a: 1i64 }.into_value(),
            vec![Value::Array(vec![Value::None, Value::Null].into())].into_value(),
            RecordId::new("test", "key").into_value(),
            Bytes::from(vec![0u8, 255]).into_value(),
        ];
        for expected in values {
            let input = futures::stream::iter(vec![
                Ok(BatchItem::Value {
                    index: 0,
                    value: 19i64.into_value(),
                }),
                terminal(0, None),
                Ok(BatchItem::Value {
                    index: 1,
                    value: expected.clone(),
                }),
                terminal(1, None),
            ]);
            let mut query = owner(&reader, input, 2, 1);
            let result = query.collect(None).await.unwrap();
            assert_eq!(result.value.as_ref(), Some(&expected));
            assert!(
                budget.reserved() > 0,
                "reservation survives exact value conversion boundary"
            );
            assert_eq!(reader.operations.lock().unwrap().active, 1);
            assert!(
                reader.close().await.is_err(),
                "close waits for final conversion ownership"
            );
            drop(result);
            assert_eq!(budget.reserved(), 0);
        }
        for (item, expected) in [
            (BatchItem::EmptyValue { index: 0 }, Value::None),
            (
                BatchItem::Rows {
                    index: 0,
                    values: vec![],
                },
                Value::Array(Default::default()),
            ),
            (
                BatchItem::Rows {
                    index: 0,
                    values: vec![7i64.into_value()],
                },
                vec![7i64].into_value(),
            ),
        ] {
            let mut query = owner(
                &reader,
                futures::stream::iter(vec![Ok(item), terminal(0, None)]),
                1,
                0,
            );
            assert_eq!(query.collect(None).await.unwrap().take_value(), expected);
        }
        reader.close().await.unwrap();
    }
    #[tokio::test]
    async fn primary_statement_and_late_transport_failures_are_composed() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let reader = reader(&budget);
        let mut query = owner(
            &reader,
            futures::stream::iter(vec![
                terminal(
                    0,
                    Some(surrealdb::Error::internal("first statement".into())),
                ),
                terminal(
                    1,
                    Some(surrealdb::Error::internal("second statement".into())),
                ),
                Err(surrealdb::Error::internal(
                    "late physical EOF failure".into(),
                )),
            ]),
            2,
            1,
        );
        let error = query.collect(None).await.err().unwrap();
        let text = error.to_string();
        assert!(
            text.contains("first statement")
                && text.contains("second statement")
                && text.contains("late physical EOF failure")
        );
        assert!(!error.permits_storage_cleanup());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn refused_growth_keeps_charge_until_drain_and_preserves_late_failure() {
        let budget = ResourceBudget::fixed(512).unwrap();
        let reader = reader(&budget);
        let mut query = owner(
            &reader,
            futures::stream::iter(vec![
                Ok(BatchItem::Rows {
                    index: 0,
                    values: vec!["x".repeat(2048).into_value()],
                }),
                terminal(
                    0,
                    Some(surrealdb::Error::internal("late refusal terminal".into())),
                ),
            ]),
            1,
            0,
        );
        let error = query.collect(None).await.err().unwrap().to_string();
        assert!(error.contains("memory") || error.contains("resource"));
        assert!(error.contains("late refusal terminal"));
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn private_drop_before_first_batch_retains_session_and_terminal_receipt() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let reader = reader(&budget);
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let query = owner(&reader, receiver, 1, 0);
        let original_count = Arc::strong_count(&reader.client);
        drop(query);
        assert_eq!(reader.operations.lock().unwrap().active, 1);
        assert_eq!(Arc::strong_count(&reader.client), original_count);
        assert!(budget.reserved() > 0);
        sender
            .unbounded_send(terminal(
                0,
                Some(surrealdb::Error::internal("abandoned terminal".into())),
            ))
            .unwrap();
        sender
            .unbounded_send(Err(surrealdb::Error::internal(
                "abandoned late transport".into(),
            )))
            .unwrap();
        drop(sender);
        let error = reader.close().await.unwrap_err().to_string();
        assert!(error.contains("abandoned terminal") && error.contains("abandoned late transport"));
        assert_eq!(reader.operations.lock().unwrap().active, 0);
        assert!(
            !reader.operations.lock().unwrap().closed,
            "unknown execution retains its owner"
        );
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn confirmed_abandoned_statement_failure_closes_scoped_reader_and_preserves_error() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut reader =
            NativeReader::for_views(Arc::new(Surreal::init()), vec![]).with_budget(&budget);
        Arc::get_mut(reader.scope.as_mut().unwrap()).unwrap()._pin =
            Some(crate::control::ReaderPin::test_detached(true));
        let query = owner(
            &reader,
            futures::stream::iter(vec![terminal(
                0,
                Some(surrealdb::Error::internal(
                    "confirmed abandoned statement".into(),
                )),
            )]),
            1,
            0,
        );
        drop(query);
        let error = reader.close().await.unwrap_err();
        assert!(error.to_string().contains("confirmed abandoned statement"));
        assert!(error.permits_storage_cleanup());
        assert!(reader.operations.lock().unwrap().closed);
        assert!(reader.check_read_admission().is_err());
        assert_eq!(budget.reserved(), 0);
        assert!(
            reader
                .close()
                .await
                .unwrap_err()
                .to_string()
                .contains("confirmed abandoned statement"),
            "terminal failure remains observable"
        );
    }
    #[tokio::test]
    async fn interrupted_close_retains_drop_drain_join_through_late_physical_error() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let reader = reader(&budget);
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let mut query = owner(&reader, receiver, 1, 0);
        sender.unbounded_send(terminal(0, None)).unwrap();
        let mut delivery = Box::pin(query.collect(None));
        assert!(futures::poll!(delivery.as_mut()).is_pending());
        drop(delivery);
        assert_eq!(query.accepted.as_ref().unwrap().ended, 1);
        drop(query);
        let mut closing = Box::pin(reader.close());
        assert!(futures::poll!(closing.as_mut()).is_pending());
        drop(closing);
        assert_eq!(reader.operations.lock().unwrap().drains.len(), 1);
        sender
            .unbounded_send(Err(surrealdb::Error::internal(
                "physical failure after logical terminal".into(),
            )))
            .unwrap();
        drop(sender);
        let error = reader.close().await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("physical failure after logical terminal")
        );
        assert!(!error.permits_storage_cleanup());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn cancelled_healthy_query_drains_and_healthy_abandonment_keeps_success_receipt() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let reader = reader(&budget);
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let mut query = owner(
            &reader,
            futures::stream::iter(vec![terminal(0, None)]),
            1,
            0,
        );
        let error = query.collect(Some(flag.clone())).await.err().unwrap();
        assert!(error.to_string().contains("cancelled"));
        assert!(error.permits_storage_cleanup());
        let query = owner(
            &reader,
            futures::stream::iter(vec![
                Ok(BatchItem::Rows {
                    index: 0,
                    values: vec!["unconsumed".repeat(8192).into_value()],
                }),
                terminal(0, None),
            ]),
            1,
            0,
        );
        drop(query);
        reader.close().await.unwrap();
        assert_eq!(reader.operations.lock().unwrap().abandoned.len(), 1);
        assert!(reader.operations.lock().unwrap().abandoned[0].is_ok());
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn malformed_inventory_and_physical_eof_cannot_accept_output() {
        for input in [
            vec![terminal(0, None)],
            vec![terminal(1, None), terminal(0, None)],
            vec![terminal(0, None), terminal(1, None), terminal(2, None)],
            vec![
                Ok(BatchItem::Value {
                    index: 1,
                    value: Value::Null,
                }),
                terminal(0, None),
                terminal(1, None),
            ],
            vec![
                Ok(BatchItem::Value {
                    index: 0,
                    value: Value::Null,
                }),
                Ok(BatchItem::Rows {
                    index: 0,
                    values: vec![],
                }),
                terminal(0, None),
                terminal(1, None),
            ],
        ] {
            let budget = ResourceBudget::fixed(4096).unwrap();
            let reader = reader(&budget);
            let mut query = owner(&reader, futures::stream::iter(input), 2, 1);
            let error = query.collect(None).await.err().unwrap();
            assert!(!error.permits_storage_cleanup());
        }
    }
}
struct SelectionLease {
    scope: Arc<ReaderScope>,
    confirmed: bool,
}
impl Drop for SelectionLease {
    fn drop(&mut self) {
        if !self.confirmed {
            self.scope.retain_unknown();
        }
    }
}
impl<Context> NativeReader<Context> {
    pub fn transport_reader(&self) -> NativeReader<()> {
        NativeReader {
            operations: self.operations.clone(),
            client: self.client.clone(),
            handle: (),
            derivation: self.derivation,
            scope: self.scope.clone(),
            cancellation: self.cancellation.clone(),
            budget: self.budget.clone(),
            request_budget: self.request_budget.clone(),
        }
    }
    /// Retained selection and default scratch share the caller's allocation authority.
    pub fn with_budget(mut self, budget: &lctx_model::domain::resources::ResourceBudget) -> Self {
        self.budget = Some(budget.clone());
        self.request_budget = Some(budget.clone());
        self
    }
    pub fn with_request_budget_clone(
        &self,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> NativeReader<()> {
        self.transport_reader().with_request_budget(budget)
    }
    pub fn with_request_budget(
        mut self,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Self {
        self.request_budget = Some(budget.clone());
        self
    }
    pub fn resource_budget(
        &self,
    ) -> Result<lctx_model::domain::resources::ResourceBudget, ModelError> {
        self.budget
            .clone()
            .ok_or(ModelError::Conflict("native reader requires caller budget"))
    }
    pub fn recovery_scope(&self) -> Result<Arc<crate::control::RecoveryScope>, ModelError> {
        let recovery = self
            .operations
            .lock()
            .map_err(|_| ModelError::Conflict("native query operation owner"))?
            .recovery
            .clone()
            .or_else(crate::control::capture_recovery_scope)
            .ok_or(ModelError::Conflict("native reader recovery owner missing"))?;
        if !Arc::ptr_eq(recovery.client(), &self.client) {
            return Err(ModelError::Conflict(
                "native reader recovery session differs",
            ));
        }
        Ok(recovery)
    }
    fn scratch_budget(&self) -> Result<lctx_model::domain::resources::ResourceBudget, ModelError> {
        self.request_budget.clone().ok_or(ModelError::Conflict(
            "native selected read requires caller budget",
        ))
    }
    pub fn protect_terminal_close(&self) -> ReadFinalizationGuard {
        ReadFinalizationGuard {
            client: self.client.clone(),
            pin: self.scope.as_ref().and_then(|scope| scope._pin.clone()),
            installed: self.scope.as_ref().and_then(|scope| scope.exact.clone()),
            closed: false,
        }
    }
    pub fn with_read_cancellation(mut self, flag: Arc<std::sync::atomic::AtomicBool>) -> Self {
        self.cancellation = Some(flag);
        self
    }
    pub fn read_cancellation(&self) -> Option<Arc<std::sync::atomic::AtomicBool>> {
        self.cancellation.clone()
    }
    pub(crate) fn scope_pin(&self) -> Option<Arc<crate::control::ReaderPin>> {
        self.scope.as_ref().and_then(|scope| scope._pin.clone())
    }
    pub(crate) fn ensure_scope_live(&self) -> Result<(), ModelError> {
        self.check_read_admission()?;
        if let Some(scope) = &self.scope {
            scope.ensure_live()?;
        }
        Ok(())
    }
    pub(crate) fn matches_installed_views(
        &self,
        views: &[lctx_model::domain::ContentHash],
    ) -> bool {
        self.scope.as_ref().is_some_and(|scope| {
            scope.exact.is_some()
                && views.iter().all(|view| scope.views.contains(view))
                && scope.views.iter().all(|view| views.contains(view))
        })
    }
    pub(crate) fn require_installed_scope(&self) -> Result<(), ModelError> {
        self.ensure_scope_live()?;
        let scope = self
            .scope
            .as_ref()
            .ok_or(ModelError::Conflict("installed reader scope missing"))?;
        if scope.exact.is_none() {
            return Err(ModelError::Conflict("installed reader exact scope missing"));
        }
        Ok(())
    }
    pub(crate) fn attach_scope(&self, mut rows: NativeRows) -> Result<NativeRows, ModelError> {
        self.ensure_scope_live()?;
        rows.scope = self.scope.clone();
        Ok(rows)
    }
    pub fn check_read_admission(&self) -> Result<(), ModelError> {
        if self
            .operations
            .lock()
            .map_err(|_| ModelError::Conflict("native query operation owner"))?
            .closed
        {
            return Err(ModelError::Conflict("native reader closed"));
        }
        crate::prepared::check_read_cancellation(self.cancellation.as_ref())
    }
    /// The owner drains reader clones and streams before explicit session invalidation.
    pub async fn close(&self) -> Result<(), ModelError> {
        let factory = self
            .operations
            .lock()
            .map_err(|_| ModelError::Conflict("native query operation owner"))?
            .factory
            .as_ref()
            .map(|lease| lease.factory().clone());
        self.close_inner().await.map_err(|error| match factory {
            Some(factory) => factory.handback(error),
            None => error,
        })
    }
    async fn close_inner(&self) -> Result<(), ModelError> {
        let drains = self
            .operations
            .lock()
            .map_err(|_| ModelError::Conflict("native query operation owner"))?
            .drains
            .clone();
        let mut join_errors = Vec::new();
        for task in &drains {
            if let Err(error) = task.clone().await {
                self.retain_unknown();
                join_errors.push(Err(error));
            }
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        {
            let mut operations = self
                .operations
                .lock()
                .map_err(|_| ModelError::Conflict("native query operation owner"))?;
            // Concurrent close can observe the same shared joins. Leave their persistent
            // outcomes in place; removing only the completed prefix is unnecessary for
            // terminality and would race another closer's prefix length.
            operations.abandoned.extend(join_errors);
        }
        if self
            .scope
            .as_ref()
            .is_some_and(|scope| Arc::strong_count(scope) != 1)
        {
            return Err(ModelError::Conflict("native reader clones remain live"));
        }
        {
            let mut operations = self
                .operations
                .lock()
                .map_err(|_| ModelError::Conflict("native query operation owner"))?;
            if operations.active != 0 {
                return Err(ModelError::Conflict("native accepted queries remain live"));
            }
            for error in &operations.abandoned {
                completion.step(
                    "abandoned native query terminal",
                    error.clone().map_err(ModelError::SharedCause),
                );
            }
            // A confirmed statement failure remains observable without keeping a
            // completed reader open. Unknown execution must retain its owners.
            if operations.abandoned.iter().any(|outcome| {
                outcome
                    .as_ref()
                    .is_err_and(|error| !error.permits_storage_cleanup())
            }) {
                return lctx_model::domain::completion::complete(Ok(()), completion);
            }
            operations.closed = true;
        }
        if let Some(scope) = &self.scope {
            if Arc::strong_count(scope) != 1 {
                return lctx_model::domain::completion::complete(
                    Err(ModelError::Conflict("native reader clones remain live")),
                    completion,
                );
            }
            if scope.release_pin_on_close
                && let Some(pin) = &scope._pin
                && let Err(error) = pin.release().await
            {
                return lctx_model::domain::completion::complete(Err(error), completion);
            }
            match scope.selection.lock() {
                Ok(mut selection) => {
                    selection.take();
                }
                Err(_) => {
                    return lctx_model::domain::completion::complete(
                        Err(ModelError::Conflict("reader selection owner")),
                        completion,
                    );
                }
            }
        }
        let owns_session = {
            let state = self
                .operations
                .lock()
                .map_err(|_| ModelError::Conflict("native query operation owner"))?;
            state.owns_session && !state.session_closed
        };
        let (recovery, signals, factory) = {
            let operations = self
                .operations
                .lock()
                .map_err(|_| ModelError::Conflict("native query operation owner"))?;
            (
                operations.recovery.clone(),
                operations.signals.clone(),
                operations.factory.clone(),
            )
        };
        if owns_session
            && let Some(recovery) = &recovery
            && recovery.pending_count()? != 0
        {
            completion.local = lctx_model::domain::completion::LocalState::Outstanding;
            completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
            completion.step(
                "owned reader recovery handback",
                Err(ModelError::Conflict(
                    "native reader has pending recovery requests",
                )),
            );
            return lctx_model::domain::completion::complete(Ok(()), completion);
        }
        if owns_session {
            // Even after confirmed query drainage, session detach/invalidation
            // needs its own acknowledgement. Cancellation must retain the exact
            // owner rather than enqueue a best-effort detach on handle drop.
            let mut session = InstalledReaderSetup {
                views: vec![],
                pin: None,
                client: self.client.clone(),
                confirmed: false,
            };
            let invalidated = if let Some(factory) = &factory {
                factory.close_session().await
            } else {
                self.client.invalidate().await.map_err(sdk_error)
            };
            session.confirmed = invalidated.is_ok();
            if invalidated.is_err() {
                completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
                completion.step("owned reader session invalidation", invalidated);
                return lctx_model::domain::completion::complete(Ok(()), completion);
            }
            self.operations
                .lock()
                .map_err(|_| ModelError::Conflict("native query operation owner"))?
                .session_closed = true;
        }
        if let Some(factory) = factory {
            completion.step(
                "owned reader factory finality",
                factory.complete_with(&completion).await,
            );
        } else if let Some(signals) = signals {
            completion.step(
                "owned reader signal finality",
                signals.complete(&completion).await,
            );
        }
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
    /// Conservative protection when a finalizer cannot establish remote terminality.
    pub fn retain_unknown(&self) {
        if let Some(scope) = &self.scope {
            scope.retain_unknown();
        }
    }
    pub fn authorize_original_ranges(
        &self,
        ranges: &[(lctx_model::domain::graph::EntityId, u64, usize)],
    ) -> Result<(), ModelError> {
        if let Some(originals) = self
            .scope
            .as_ref()
            .and_then(|scope| scope.originals.as_ref())
        {
            for (source, start, length) in ranges {
                let original = originals
                    .iter()
                    .find(|original| original.source == *source)
                    .ok_or(ModelError::Conflict("original source outside publication"))?;
                if start
                    .checked_add(*length as u64)
                    .is_none_or(|end| end > original.byte_len)
                {
                    return Err(ModelError::Conflict("original range outside publication"));
                }
            }
        }
        Ok(())
    }
    pub fn with_derivation(mut self, namespace: lctx_model::domain::ContentHash) -> Self {
        self.derivation = Some(namespace);
        self
    }
    pub fn derivation_namespace(&self) -> Option<lctx_model::domain::ContentHash> {
        self.derivation
    }
    pub fn derivation(&self) -> Result<lctx_model::domain::ContentHash, ModelError> {
        self.derivation.ok_or(ModelError::Conflict(
            "derived read requires exact realization",
        ))
    }
    pub fn view_bindings(&self) -> Variables {
        let mut bindings = Variables::new();
        bindings.insert(
            "lctx_derivation",
            self.derivation
                .map(|value| Value::String(value.hex()))
                .unwrap_or(Value::None),
        );
        if let Some(scope) = &self.scope {
            bindings.insert(
                "lctx_views",
                scope
                    .views
                    .iter()
                    .map(|view| RecordId::new("compiler_view", view.hex()))
                    .collect::<Vec<_>>(),
            );
        }
        bindings
    }
    /// One operation-local immutable, disk-backed selection; this carries no acceptance result.
    pub async fn prepare_selection(
        &self,
    ) -> Result<Option<crate::selection::SelectedPayloads>, ModelError> {
        self.selection_preparation().await
    }
    pub fn selection_preparation(
        &self,
    ) -> futures::future::BoxFuture<
        'static,
        Result<Option<crate::selection::SelectedPayloads>, ModelError>,
    > {
        let scope = self.scope.clone();
        let return_owner = self.transport_reader();
        let derivation = self.derivation;
        let client = self.client.clone();
        let cancellation = self.cancellation.clone();
        let budget = self.budget.clone();
        async move {
            let Some(scope) = scope else {
                return Ok(None);
            };
            let budget = budget.ok_or(ModelError::Conflict(
                "native selection requires caller budget",
            ))?;
            return_owner.ensure_scope_live()?;
            let pending = {
                let mut selection = scope
                    .selection
                    .lock()
                    .map_err(|_| ModelError::Conflict("reader selection owner"))?;
                if let Some(prepared) = selection.as_ref() {
                    if !prepared.budget.shares_pool(&budget) || prepared.derivation != derivation {
                        return Err(ModelError::Conflict(
                            "foreign native selection budget or derivation",
                        ));
                    }
                    prepared.pending.clone()
                } else {
                    let retained_scope = scope.clone();
                    let preparation_cancellation = cancellation.clone();
                    let retained_budget = budget.clone();
                    let task = tokio::spawn(async move {
                        let mut lease = SelectionLease {
                            scope: retained_scope,
                            confirmed: false,
                        };
                        let result = if let Some(exact) = &lease.scope.exact {
                            if exact._installed.is_empty() {
                                crate::selection::SelectedPayloads::for_completed_selectors(
                                    client,
                                    &exact.references,
                                    preparation_cancellation,
                                    &retained_budget,
                                )
                                .await
                            } else if let Some(borrowed) = &lease.scope.borrowed {
                                crate::selection::SelectedPayloads::for_borrowed_installed_session(
                                    client,
                                    &exact._installed,
                                    borrowed.clone(),
                                    preparation_cancellation,
                                    &retained_budget,
                                )
                                .await
                            } else {
                                crate::selection::SelectedPayloads::for_installed_session(
                                    client,
                                    &exact._installed,
                                    preparation_cancellation,
                                    &retained_budget,
                                )
                                .await
                            }
                        } else if let Some(handle) = lease.scope.publication.clone() {
                            let published = NativeReader {
                                operations: Default::default(),
                                client: client.clone(),
                                handle,
                                derivation,
                                cancellation: preparation_cancellation.clone(),
                                budget: Some(retained_budget.clone()),
                                request_budget: Some(retained_budget.clone()),
                                scope: Some(lease.scope.clone()),
                            };
                            let prepared = async {
                                let completed = crate::completed_state::published_completed_state(
                                    &published,
                                    &retained_budget,
                                )
                                .await?;
                                crate::selection::SelectedPayloads::for_published_session(
                                    client,
                                    &completed,
                                    &lease.scope.views,
                                    preparation_cancellation,
                                    &retained_budget,
                                )
                                .await
                            }
                            .await;
                            // Neither the temporary capability nor this reader clone is
                            // retained by the mechanical cached result.
                            drop(published);
                            prepared
                        } else {
                            Err(ModelError::Conflict(
                                "canonical selection requires protected exact installed views",
                            ))
                        }
                        .map(|selected| {
                            if let Some(namespace) = derivation {
                                selected.with_derivation(namespace)
                            } else {
                                selected
                            }
                        });
                        lease.confirmed = result
                            .as_ref()
                            .err()
                            .is_none_or(|error| error.permits_storage_cleanup());
                        drop(lease);
                        result
                    });
                    let pending = async move {
                        task.await
                            .map_err(|error| Arc::new(ModelError::codec(error)))?
                            .map_err(Arc::new)
                    }
                    .boxed()
                    .shared();
                    *selection = Some(SelectionPreparation {
                        budget,
                        derivation,
                        pending: pending.clone(),
                    });
                    pending
                }
            };
            let selected = pending.await.map_err(ModelError::SharedCause)?;
            return_owner.ensure_scope_live()?;
            Ok(Some(
                selected
                    .with_reader_owner(return_owner)?
                    .with_read_cancellation(cancellation),
            ))
        }
        .boxed()
    }
    pub fn selected_payload_rows(
        &self,
        table: &str,
        predicate: &str,
        bindings: Variables,
        preparation: Vec<String>,
        order: &str,
        limit: Option<usize>,
    ) -> Result<NativeRows, ModelError> {
        if self.scope.is_none() {
            let limit = limit
                .map(|limit| format!(" LIMIT {limit}"))
                .unwrap_or_default();
            return self.stream_prepared(crate::prepared::PreparedQuery::new(
                bindings,
                preparation,
                vec![format!(
                    "SELECT * FROM {table} WHERE ({predicate}) ORDER BY {order}{limit}"
                )],
            )?);
        }
        let selection = self.selection_preparation();
        let scope = self.scope.clone().expect("scope");
        let budget = self.scratch_budget()?;
        let table = table.to_owned();
        let predicate = predicate.to_owned();
        let order = order.to_owned();
        let mut rows = NativeRows::owned(budget.clone(), move |sender| async move {
            let selection = selection
                .await?
                .ok_or(ModelError::Conflict("selected reader scope"))?;
            if sender.is_closed() {
                return Ok(());
            }
            let mut rows = selection.rows_with_budget(
                crate::selection::SelectedPayloadRead {
                    table: &table,
                    predicate: &predicate,
                    bindings,
                    preparation,
                    order: &order,
                    limit,
                },
                &budget,
            )?;
            let result = async { loop {
                let row = tokio::select! { biased; _ = sender.closed() => { rows.cancel_delivery(); break; }, row = rows.next() => row? };
                let Some(row) = row else { break; };
                if !sender.send(row).await? { rows.cancel_delivery(); break; }
            } Ok(()) }.await;
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "selected reader payload drainage",
                rows.drain_transport().await,
            );
            lctx_model::domain::completion::complete(result, completion)
        })?;
        rows.scope = Some(scope);
        rows.client = Some(self.client.clone());
        Ok(rows)
    }
    /// A model-owned indexed query returns compact `id` nominations. Exact selected
    /// membership is checked before fetching their complete canonical payloads.
    pub fn candidate_payload_rows(
        &self,
        candidates: crate::prepared::PreparedQuery,
        table: &str,
        order: &str,
    ) -> Result<NativeRows, ModelError> {
        self.check_read_admission()?;
        let budget = self.scratch_budget()?;
        let scope = self.scope.as_ref().ok_or(ModelError::Conflict(
            "candidate read requires exact selected scope",
        ))?;
        scope.require_canonical_selection()?;
        let mut rows = {
            let preparation = self.selection_preparation();
            let table = table.to_owned();
            let order = order.to_owned();
            NativeRows::owned(budget.clone(), move |sender| async move {
                let selected = preparation
                    .await?
                    .ok_or(ModelError::Conflict("candidate exact selection missing"))?;
                let mut rows = selected.candidate_rows(candidates, &table, &order, &budget)?;
                let result = async {
                    while let Some(row) = rows.next().await? {
                        if !sender.send(row).await? {
                            rows.cancel_delivery();
                            break;
                        }
                    }
                    Ok(())
                }
                .await;
                let mut terminal = lctx_model::domain::completion::Completion::default();
                terminal.step(
                    "exact candidate payload drainage",
                    rows.drain_transport().await,
                );
                lctx_model::domain::completion::complete(result, terminal)
            })?
        };
        rows.scope = Some(scope.clone());
        rows.client = Some(self.client.clone());
        Ok(rows)
    }
    /// One answer for each distinct requested physical identity, with exact view and
    /// reverse-alias provenance. Absence is local to that identity, never an any-hit flag.
    pub async fn selected_membership(
        &self,
        requested: &[RecordId],
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<crate::selection::MembershipAnswers, ModelError> {
        self.check_read_admission()?;
        let Some(scope) = self.scope.clone() else {
            return crate::selection::MembershipAnswers::unrestricted(requested, budget);
        };
        scope.require_canonical_selection()?;
        let requested_charge = budget.reserve(
            "selected-membership-task-input",
            requested
                .iter()
                .map(crate::selection::node_bytes)
                .sum::<usize>(),
        )?;
        let requested = requested.to_vec();
        let budget = budget.clone();
        let selection = self.selection_preparation();
        // Caller cancellation detaches delivery, not the admitted physical operation.
        // The task retains its input charge and reader pin until every query is drained.
        tokio::spawn(async move {
            let _requested_charge = requested_charge;
            let mut lease = SelectionLease {
                scope,
                confirmed: false,
            };
            let result = {
                match selection.await {
                    Ok(Some(selected)) => selected.membership_answers(&requested, &budget).await,
                    Ok(None) => Err(ModelError::Conflict("exact membership selection missing")),
                    Err(error) => Err(error),
                }
            };
            lease.confirmed = result
                .as_ref()
                .err()
                .is_none_or(|error| error.permits_storage_cleanup());
            result
        })
        .await
        .map_err(ModelError::codec)?
    }
    pub fn retain_rows(&self, mut rows: NativeRows) -> NativeRows {
        rows.scope = self.scope.clone();
        rows.client = Some(self.client.clone());
        rows
    }
    pub async fn selected_candidate_ids(
        &self,
        requested: &[RecordId],
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<Vec<RecordId>, ModelError> {
        Ok(self
            .selected_membership(requested, budget)
            .await?
            .answers
            .into_iter()
            .filter(|answer| answer.selected)
            .map(|answer| answer.requested)
            .collect())
    }
    pub fn record_stream_candidates<R: Record + DeserializeOwned>(
        &self,
        candidates: crate::prepared::PreparedQuery,
        order: &str,
    ) -> Result<CanonicalRecords<R>, ModelError> {
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
        if table == "compiler_record" {
            return Err(ModelError::Schema("canonical native record family"));
        }
        Ok(CanonicalRecords {
            rows: self.candidate_payload_rows(candidates, table, order)?,
            positions: vec![0],
            physical_kind: true,
            failed: false,
            marker: std::marker::PhantomData,
        })
    }
    /// Release retained immutable preparation after the owning service has drained requests.
    pub async fn release_preparation(&self) -> Result<(), ModelError> {
        if let Some(scope) = &self.scope {
            let pending = scope
                .selection
                .lock()
                .map_err(|_| ModelError::Conflict("reader selection owner"))?
                .take();
            if let Some(prepared) = pending {
                prepared.pending.await.map_err(ModelError::SharedCause)?;
            }
        }
        Ok(())
    }
    pub fn relation_rows(&self, relation: &str) -> Result<NativeRows, ModelError> {
        if self.scope.is_none() {
            return Err(ModelError::Conflict(
                "relation read requires exact published view",
            ));
        }
        let mut vars = Variables::new();
        vars.insert("type", relation.to_owned());
        self.selected_payload_rows(
            crate::schema::ScopeTable::for_relation(relation)?.name(),
            "semantic_type=$type",
            vars,
            vec![],
            "semantic_key",
            None,
        )
    }
    pub fn relation_bodies(&self, relation: &str, limit: usize) -> Result<NativeRows, ModelError> {
        self.check_read_admission()?;
        let budget = self.scratch_budget()?;
        let scope = self
            .scope
            .as_ref()
            .ok_or(ModelError::Conflict(
                "relation read requires exact published view",
            ))?
            .clone();
        let table = crate::schema::ScopeTable::for_relation(relation)?.name();
        scope.require_canonical_selection()?;
        let mut input = {
            let preparation = self.selection_preparation();
            let relation = relation.to_owned();
            let table = table.to_owned();
            let budget = budget.clone();
            NativeRows::owned(budget.clone(), move |sender| async move {
                let selected = preparation
                    .await?
                    .ok_or(ModelError::Conflict("exact relation selection missing"))?;
                if limit == 0 {
                    return Ok(());
                }
                let mut rows = selected.relation_candidates(&relation, &table, &budget)?;
                let result = async {
                    while let Some(row) = rows.next().await? {
                        if !sender.send(row).await? {
                            rows.cancel_delivery();
                            break;
                        }
                    }
                    Ok(())
                }
                .await;
                let mut terminal = lctx_model::domain::completion::Completion::default();
                terminal.step(
                    "exact relation nomination drainage",
                    rows.drain_transport().await,
                );
                lctx_model::domain::completion::complete(result, terminal)
            })?
        };
        let mut rows = NativeRows::owned(budget.clone(), move |sender| async move {
            let result = async {
                let mut emitted = 0usize;
                while let Some(row) = input.next().await? {
                    if sender.is_closed() {
                        input.cancel_delivery();
                        break;
                    }
                    if emitted < limit {
                        if !sender.send(row).await? {
                            input.cancel_delivery();
                            break;
                        }
                        emitted += 1;
                    }
                }
                Ok(())
            }
            .await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step("relation candidate drainage", input.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)
        })?;
        rows.scope = Some(scope);
        rows.client = Some(self.client.clone());
        Ok(rows)
    }
    pub fn query_stream(
        &self,
        sql: impl Into<String>,
        bindings: Variables,
        statements: usize,
    ) -> Result<NativeRows, ModelError> {
        self.stream_prepared(crate::prepared::PreparedQuery::from_sql(
            sql.into(),
            bindings,
            statements,
            (0..statements).collect(),
        )?)
    }
    pub fn stream_prepared(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<NativeRows, ModelError> {
        self.ensure_scope_live()?;
        query
            .with_bindings(self.view_bindings())
            .stream_cancellable(
                &self.client,
                self.cancellation.as_ref(),
                &self.scratch_budget()?,
            )
            .map(|mut rows| {
                rows.scope = self.scope.clone();
                rows
            })
    }
    pub async fn query_prepared<T: Serialize + DeserializeOwned + 'static>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<T, ModelError> {
        let mut value = self.checked_native_value(query)?.await?;
        SerdeWrapper::<T>::from_value(value.take_value())
            .map(|value| value.0)
            .map_err(ModelError::codec)
    }
    /// Prepared SDK-native results retain record IDs and nested Object/Array variants.
    /// Exposure uses the same complete response and terminal checks as typed serde results.
    pub async fn query_prepared_native<T: SurrealValue>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<T, ModelError> {
        let mut value = self.checked_native_value(query)?.await?;
        T::from_value(value.take_value()).map_err(ModelError::codec)
    }
    /// Finite owner-generated SELECT/RETURN reads. This API does not register LIVE
    /// subscriptions and is not a read-only authorization fence for arbitrary SQL.
    fn checked_native_value(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<futures::future::BoxFuture<'_, Result<ChargedNativeValue, ModelError>>, ModelError>
    {
        self.ensure_scope_live()?;
        if query.result_positions().len() != 1 {
            return Err(ModelError::Schema("native single result demand"));
        }
        let selected = query.result_positions()[0];
        let statements = query.expected_terminals();
        let budget = self.scratch_budget()?;
        let body_budget = budget.clone();
        let reader = self.transport_reader();
        let body = async move {
            reader.ensure_scope_live()?;
            let budget = body_budget;
            let charge = budget.reserve(
                "native exact query accumulation",
                std::mem::size_of::<Value>(),
            )?;
            let (sql, bindings) = query.with_bindings(reader.view_bindings()).into_request();
            reader.ensure_scope_live()?;
            let stream = reader
                .client
                .query(sql)
                .bind(bindings)
                .stream_batches_with_options(ingress::options(&budget)?)
                .map_err(sdk_error)?;
            let lease = QueryLease::admit(
                reader.operations.clone(),
                reader.client.clone(),
                reader.scope.clone(),
            )?;
            let mut owner =
                NativeValueOwner::new_admitted(stream, statements, selected, budget, charge, lease);
            let value = owner.collect(reader.cancellation.clone()).await?;
            reader.ensure_scope_live()?;
            Ok(value)
        };
        // Every typed/native scalar query shares this concrete execution seam.
        // Admit the complete continuation before allocation, stream issuance or
        // registry admission; the grant survives collection and terminal checks.
        let bytes = std::mem::size_of_val(&body)
            .checked_add(std::mem::size_of::<
                Box<dyn lctx_model::domain::resources::Reservation>,
            >())
            .and_then(|bytes| bytes.checked_add(2 * std::mem::align_of_val(&body)))
            .ok_or(ModelError::Schema("native value query continuation size"))?;
        let grant = budget.reserve("native value query continuation", bytes)?;
        Ok(Box::pin(async move {
            let _grant = grant;
            body.await
        }))
    }

    /// Finite owner-selected predicates; source payloads are decoded one at a time.
    pub fn record_stream<R: Record + DeserializeOwned>(
        &self,
        predicate: &str,
        bindings: Variables,
        order: &str,
    ) -> Result<CanonicalRecords<R>, ModelError> {
        self.record_stream_prepared(predicate, bindings, vec![], order)
    }
    pub fn record_stream_prepared<R: Record + DeserializeOwned>(
        &self,
        predicate: &str,
        mut bindings: Variables,
        preparation: Vec<String>,
        order: &str,
    ) -> Result<CanonicalRecords<R>, ModelError> {
        bindings.insert("type", R::NAME.to_string());
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
        if table == "compiler_record" {
            return Err(ModelError::Schema("canonical native record family"));
        }
        Ok(CanonicalRecords {
            rows: self.selected_payload_rows(
                table,
                &format!("semantic_type=$type AND ({predicate})"),
                bindings,
                preparation,
                order,
                None,
            )?,
            positions: vec![0],
            physical_kind: true,
            failed: false,
            marker: std::marker::PhantomData,
        })
    }
    pub fn client(&self) -> &Surreal<Client> {
        &self.client
    }
    pub fn shared_client(&self) -> Arc<Surreal<Client>> {
        self.client.clone()
    }
    /// Query results become visible only after the SDK has received the complete checked response.
    pub async fn query<T: Serialize + DeserializeOwned + 'static>(
        &self,
        sql: impl Into<String>,
        bindings: Variables,
    ) -> Result<T, ModelError> {
        self.query_prepared(crate::prepared::PreparedQuery::new(
            bindings,
            vec![],
            vec![sql.into()],
        )?)
        .await
    }
    /// Decode through the SDK's native value contract, preserving nested value variants.
    /// Results become visible only after the complete checked response and terminal inventory.
    pub async fn query_native<T: SurrealValue>(
        &self,
        sql: impl Into<String>,
        bindings: Variables,
    ) -> Result<T, ModelError> {
        let query = crate::prepared::PreparedQuery::new(bindings, vec![], vec![sql.into()])?;
        self.query_prepared_native(query).await
    }
    pub async fn records<R: Record + DeserializeOwned>(
        &self,
        selection: RecordSelection,
    ) -> Result<Vec<R>, ModelError> {
        self.check_read_admission()?;
        if let Some(scope) = &self.scope {
            scope.require_canonical_selection()?;
            self.prepare_selection().await?.ok_or(ModelError::Conflict(
                "typed read exact installed selection missing",
            ))?;
        }
        let mut bindings = Variables::new();
        bindings.insert("type", R::NAME.to_string());
        let mut preparation = Vec::new();
        let mut index = "";
        let mut connected = false;
        let predicate = match selection {
            RecordSelection::Keys(keys) => {
                if keys.is_empty() {
                    return Ok(vec![]);
                }
                index = " WITH INDEX semantic_key";
                bindings.insert("keys", keys.iter().map(hex::encode).collect::<Vec<_>>());
                "semantic_key IN $keys".to_owned()
            }
            RecordSelection::Scope { field, values } => {
                if values.is_empty() {
                    return Ok(vec![]);
                }
                if !R::fields().iter().any(|f| f.name() == field) {
                    return Err(ModelError::Invalid("undeclared native scope field".into()));
                }
                let sparse = crate::schema::atomic_scope_field(R::NAME, field.as_str())
                    && !values.iter().any(serde_json::Value::is_null);
                bindings.insert(
                    "values",
                    crate::loader::json_value(serde_json::Value::Array(values))?,
                );
                if sparse {
                    index = " WITH INDEX by_scope";
                    crate::prepared::prepare_scope(
                        &mut preparation,
                        "record_scope",
                        "$type",
                        &field,
                        "$values",
                    )
                } else {
                    format!("body.`{field}` IN $values")
                }
            }
            RecordSelection::Connected { targets, fields } => {
                let derivation = self.derivation()?;
                bindings.insert("lctx_derivation", derivation.hex());
                connected = true;
                if targets.is_empty() {
                    return Ok(vec![]);
                }
                bindings.insert(
                    "targets",
                    targets.into_iter().map(target_id).collect::<Vec<_>>(),
                );
                bindings.insert("fields", fields);
                "array::len((SELECT VALUE id FROM participant WITH INDEX outgoing WHERE derivation=$lctx_derivation AND in=$parent.id AND out IN $targets AND (array::len($fields)=0 OR field IN $fields) LIMIT 1))>0 OR array::len((SELECT VALUE id FROM reference WITH INDEX outgoing WHERE derivation=$lctx_derivation AND in=$parent.id AND out IN $targets AND (array::len($fields)=0 OR field IN $fields) LIMIT 1))>0".into()
            }
        };
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
        let mut rows = if self.scope.is_none() {
            self.record_stream_prepared::<R>(&predicate, bindings, preparation, "semantic_key")?
        } else {
            let statements = if connected {
                ["participant", "reference"].map(|role| format!("SELECT in AS id FROM {role} WITH INDEX incoming WHERE derivation=$lctx_derivation AND out IN $targets AND (array::len($fields)=0 OR field IN $fields) AND in.semantic_type=$type")).to_vec()
            } else {
                vec![format!(
                    "SELECT id FROM {table}{index} WHERE semantic_type=$type AND ({predicate})"
                )]
            };
            CanonicalRecords {
                rows: self.candidate_payload_rows(
                    crate::prepared::PreparedQuery::new(bindings, preparation, statements)?,
                    table,
                    "semantic_key",
                )?,
                positions: vec![0],
                physical_kind: true,
                failed: false,
                marker: std::marker::PhantomData,
            }
        };
        let result = async {
            let mut result = Vec::new();
            while let Some(row) = rows.next().await? {
                result.push(row);
            }
            Ok(result)
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("native typed record drainage", rows.drain_transport().await);
        lctx_model::domain::completion::complete(result, completion)
    }
    /// Hydrate one ordered window through its exact physical record pointers. A nominal
    /// family predicate is never a substitute for the already selected backing records.
    pub async fn records_from_candidates<R: Record + DeserializeOwned>(
        &self,
        candidates: &[crate::ordered_rows::Candidate],
    ) -> Result<Vec<R>, ModelError> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        if candidates.len() > 128 {
            return Err(ModelError::Limit {
                owner: "native-candidate-hydration",
                limit: "window rows",
                observed: candidates.len(),
                bound: 128,
            });
        }
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?;
        if table == crate::schema::ScopeTable::CompilerRecord {
            return Err(ModelError::Schema("native candidate canonical family"));
        }
        if candidates.iter().any(|candidate| {
            candidate.relation != R::NAME || candidate.node.table.as_str() != table.name()
        }) || candidates.windows(2).any(|pair| pair[0].key >= pair[1].key)
        {
            return Err(ModelError::Conflict("native candidate identity/order"));
        }
        let mut bindings = Variables::new();
        bindings.insert(
            "nodes",
            candidates
                .iter()
                .map(|candidate| candidate.node.clone())
                .collect::<Vec<_>>(),
        );
        let requested = candidates
            .iter()
            .map(|candidate| candidate.node.clone())
            .collect::<Vec<_>>();
        if self
            .selected_candidate_ids(&requested, &self.scratch_budget()?)
            .await?
            .len()
            != requested.len()
        {
            return Err(ModelError::Conflict("native candidate outside exact view"));
        }
        let query = crate::prepared::PreparedQuery::new(
            bindings,
            vec![],
            vec![candidate_records_sql().into()],
        )?;
        let position = query.result_positions()[0];
        let terminals = query.expected_terminals();
        let (sql, bindings) = query.with_bindings(self.view_bindings()).into_request();
        self.check_read_admission()?;
        let mut response = self
            .client
            .query(sql)
            .bind(bindings)
            .await
            .map_err(sdk_error)?
            .check()
            .map_err(sdk_error)?;
        if response.num_statements() != terminals {
            return Err(ModelError::Schema("native candidate response inventory"));
        }
        let rows: Vec<surrealdb::types::Object> = response.take(position).map_err(sdk_error)?;
        if rows.len() != candidates.len() {
            return Err(ModelError::Conflict("native candidate backing missing"));
        }
        let mut result = Vec::with_capacity(rows.len());
        for (projection, candidate) in rows.into_iter().zip(candidates) {
            if projection.get("id") != Some(&Value::RecordId(candidate.node.clone()))
                || projection.get("semantic_type") != Some(&Value::String(R::NAME.into()))
                || projection.get("semantic_key")
                    != Some(&Value::String(hex::encode(candidate.key)))
            {
                return Err(ModelError::Conflict("native candidate backing identity"));
            }
            let Some(Value::Bytes(canonical)) = projection.get("canonical") else {
                return Err(corrupt());
            };
            let (row, node) = match table {
                crate::schema::ScopeTable::Entity => canonical_entity_projection::<R>(canonical)?,
                crate::schema::ScopeTable::Assertion => {
                    canonical_assertion_projection::<R>(canonical)?
                }
                crate::schema::ScopeTable::CompilerRecord => {
                    return Err(ModelError::Schema("native candidate canonical family"));
                }
            };
            if *row.id().bytes() != candidate.key || node != candidate.node {
                return Err(ModelError::Conflict("native candidate canonical identity"));
            }
            result.push(row);
        }
        Ok(result)
    }
}

/// Ordering state is bounded by the caller's physical pointer window, never a family scan.
pub fn candidate_records_sql() -> &'static str {
    "SELECT id,semantic_type,semantic_key,canonical FROM $nodes ORDER BY semantic_key"
}

/// Charged persisted marker proof retained while native completion/admission is checked.
/// Neither a handle nor a namespace setter can manufacture the protected pin.
pub(crate) struct ProtectedPublicationMarker {
    row: surrealdb::types::Object,
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
}
impl ProtectedPublicationMarker {
    pub(crate) fn row(&self) -> &surrealdb::types::Object {
        &self.row
    }
}
impl NativeReader<SnapshotHandle> {
    pub(crate) async fn protected_publication_marker(
        &self,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<ProtectedPublicationMarker, ModelError> {
        let pin = self
            .scope
            .as_ref()
            .and_then(|scope| scope._pin.as_ref())
            .ok_or(ModelError::Conflict(
                "published completion requires retained reader pin",
            ))?;
        pin.ensure_live()?;
        self.check_read_admission()?;
        if self.derivation_namespace() != Some(self.handle.realization) {
            return Err(ModelError::Conflict("published reader derivation changed"));
        }
        let mut charge = budget.reserve(
            "protected publication marker",
            lctx_model::domain::resources::MAX_ROW_BYTES,
        )?;
        let reader = self.transport_reader().with_budget(budget);
        let publication = RecordId::new("publication", self.handle.publication.hex());
        let mut variables = Variables::new();
        variables.insert("publication", publication.clone());
        let rows: Vec<surrealdb::types::Object> = reader
            .query_native("SELECT * FROM $publication", variables.clone())
            .await?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict(
                "protected publication marker cardinality",
            ));
        };
        if row.get("handle")
            != Some(&Value::String(hex::encode(
                serde_json::to_vec(&self.handle).map_err(ModelError::codec)?,
            )))
            || row.get("definition_epoch")
                != Some(&Value::String(self.handle.definition_epoch.hex()))
            || row.get("derivation") != Some(&Value::String(self.handle.realization.hex()))
        {
            return Err(ModelError::Conflict("protected publication marker scope"));
        }
        let record = |field: &str, table: &str| -> Result<RecordId, ModelError> {
            match row.get(field) {
                Some(Value::RecordId(id)) if id.table.as_str() == table => Ok(id.clone()),
                _ => Err(ModelError::Schema("protected publication physical pointer")),
            }
        };
        let integer = |field: &str| -> Result<i64, ModelError> {
            match row.get(field) {
                Some(Value::Number(surrealdb::types::Number::Int(value))) if *value > 0 => {
                    Ok(*value)
                }
                _ => Err(ModelError::Schema(
                    "protected publication physical incarnation/epoch",
                )),
            }
        };
        let root_id = record("staging_root", "native_publication_staging")?;
        let completion_id = record("completion", "native_completed_state")?;
        let admission_id = record("admission", "native_completed_admission")?;
        let validity = integer("validity_epoch")?;
        let root = crate::control::inspect_expected_read(&reader, root_id.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict(
                "protected publication staging root missing",
            ))?;
        let Value::Object(root_row) = root.row() else {
            return Err(ModelError::Schema("protected publication staging root"));
        };
        if root.incarnation() != integer("staging_incarnation")?
            || root_row.get("state") != Some(&Value::String("published".into()))
            || root_row.get("completion") != Some(&Value::RecordId(completion_id.clone()))
            || root_row.get("admission") != Some(&Value::RecordId(admission_id.clone()))
            || root_row.get("derivation") != Some(&Value::String(self.handle.realization.hex()))
            || root_row.get("generation")
                != Some(&Value::String(self.handle.service_generation.hex()))
            || root_row.get("validity_epoch") != Some(&validity.into_value())
        {
            return Err(ModelError::Conflict("protected publication root changed"));
        }
        if !matches!(root_row.get("derived_children"),Some(Value::Number(surrealdb::types::Number::Int(count))) if *count>=2)
            || !matches!(root_row.get("derived_inventory"),Some(Value::String(digest)) if hex::decode(digest).is_ok_and(|bytes|bytes.len()==32))
        {
            return Err(ModelError::Conflict(
                "publication derived closure witness missing",
            ));
        }
        for (id, field) in [
            (completion_id.clone(), "completion_incarnation"),
            (admission_id.clone(), "admission_incarnation"),
        ] {
            let expected = crate::control::inspect_expected_read(&reader, id, budget)
                .await?
                .ok_or(ModelError::Conflict(
                    "protected publication witness missing",
                ))?;
            if expected.incarnation() != integer(field)? {
                return Err(ModelError::Conflict(
                    "protected publication witness incarnation changed",
                ));
            }
        }
        variables.insert("root", root_id);
        variables.insert("completion", completion_id);
        variables.insert("admission", admission_id);
        variables.insert("pin", RecordId::new("native_pin", pin.identity.hex()));
        variables.insert("generation", self.handle.service_generation.hex());
        variables.insert("validity_epoch", validity);
        let valid:bool=reader.query_native("RETURN (SELECT VALUE released FROM ONLY $pin)=false AND (SELECT VALUE generation FROM ONLY $pin)=$generation AND array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$pin AND object=$publication LIMIT 2)=1 AND (SELECT VALUE generation FROM ONLY native_installation:current)=$generation AND (SELECT VALUE validity_epoch FROM ONLY native_installation:current)=$validity_epoch AND array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$publication AND object=$root LIMIT 2)=1 AND array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$root AND object=$completion LIMIT 2)=1 AND array::len(SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$root AND object=$admission LIMIT 2)=1",variables.clone()).await?;
        if !valid {
            return Err(ModelError::Conflict(
                "protected publication reachability or validity changed",
            ));
        }
        let repeated: Vec<surrealdb::types::Object> = reader
            .query_native("SELECT * FROM $publication", variables)
            .await?;
        if repeated != rows {
            return Err(ModelError::Conflict(
                "protected publication marker changed during proof",
            ));
        }
        self.check_read_admission()?;
        // Keep the original pin borrowed for the whole proof. The resulting checked
        // completion capability additionally retains a transport-reader scope clone.
        pin.ensure_live()?;
        charge.try_resize(
            crate::loader::native_bytes(&Value::Object(row.clone()))
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        Ok(ProtectedPublicationMarker {
            row: rows.into_iter().next().unwrap(),
            _charge: charge,
        })
    }
}

impl NativeReader<SnapshotHandle> {
    pub async fn open(
        client: Arc<Surreal<Client>>,
        handle: SnapshotHandle,
    ) -> Result<Self, ModelError> {
        let mut setup = InstalledReaderSetup {
            views: vec![],
            pin: None,
            client: client.clone(),
            confirmed: false,
        };
        let result = async {
            crate::control::check_installation(&client, handle.service_generation).await?;
            let mut bindings = Variables::new();
            bindings.insert(
                "publication",
                RecordId::new("publication", handle.publication.hex()),
            );
            let mut response = client
                .query("SELECT * FROM $publication")
                .bind(bindings.clone())
                .await
                .map_err(sdk_error)?
                .check()
                .map_err(sdk_error)?;
            let rows: Vec<surrealdb::types::Object> = response.take(0).map_err(sdk_error)?;
            let [row] = rows.as_slice() else {
                return Err(ModelError::Conflict("snapshot publication missing"));
            };
            let expected = hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?);
            if row.get("handle") != Some(&Value::String(expected))
                || row.get("definition_epoch")
                    != Some(&Value::String(handle.definition_epoch.hex()))
            {
                return Err(ModelError::Conflict("snapshot publication handle"));
            }
            let Some(Value::Bytes(manifest_bytes)) = row.get("manifest") else {
                return Err(ModelError::Schema("publication manifest"));
            };
            let manifest = lctx_model::domain::graph::Manifest::decode(manifest_bytes)?;
            if manifest.content() != handle.semantic {
                return Err(ModelError::Conflict("publication semantic manifest"));
            }
            handle.validate_identity()?;
            let Some(Value::Bytes(bytes)) = row.get("views") else {
                return Err(ModelError::Schema("publication exact views"));
            };
            let completed: Vec<lctx_model::domain::completed::CompletedBinding> =
                serde_json::from_slice(bytes).map_err(ModelError::codec)?;
            if lctx_model::domain::completed::binding_inventory_identity(&completed)? != handle.view
            {
                return Err(ModelError::Conflict("publication exact view inventory"));
            }
            let mut all_views = completed
                .iter()
                .map(|binding| binding.view.identity)
                .collect::<Vec<_>>();
            all_views.sort();
            all_views.dedup();
            let mut views = completed
                .into_iter()
                .filter(|binding| binding.boundary.is_none())
                .map(|binding| binding.view.identity)
                .collect::<Vec<_>>();
            views.sort();
            views.dedup();
            let pin = crate::control::ReaderPin::acquire(client.clone(), &all_views).await?;
            setup.pin = Some(pin.clone());
            pin.protect(RecordId::new("publication", handle.publication.hex()))
                .await?;
            let protected = async {
                let mut response = client
                    .query("SELECT * FROM $publication")
                    .bind(bindings)
                    .await
                    .map_err(sdk_error)?
                    .check()
                    .map_err(sdk_error)?;
                let current: Vec<surrealdb::types::Object> = response.take(0).map_err(sdk_error)?;
                if current != rows {
                    return Err(ModelError::Conflict(
                        "publication changed before reader protection",
                    ));
                }
                Ok(())
            }
            .await;
            protected?;
            let publication_scope = handle.clone();
            Ok(Self {
                operations: Arc::new(std::sync::Mutex::new(QueryOperations {
                    recovery: crate::control::capture_recovery_scope(),
                    ..Default::default()
                })),
                client: client.clone(),
                derivation: Some(handle.realization),
                handle,
                cancellation: None,
                budget: None,
                request_budget: None,
                scope: Some(Arc::new(ReaderScope {
                    publication: Some(publication_scope),
                    exact: None,
                    originals: Some(manifest.originals),
                    views,
                    _pin: Some(pin),
                    release_pin_on_close: true,
                    borrowed: None,
                    selection: std::sync::Mutex::new(None),
                })),
            })
        }
        .await;
        match result {
            Ok(reader) => {
                setup.confirmed = true;
                Ok(reader)
            }
            Err(error) => {
                if !setup.permits_cleanup(&error) {
                    return Err(error);
                }
                let mut terminal = lctx_model::domain::completion::Completion::default();
                if let Some(pin) = &setup.pin {
                    let released = pin.release().await;
                    setup.confirmed = released.is_ok();
                    if released.is_err() {
                        setup.retain_unknown();
                        terminal.remote = lctx_model::domain::completion::RemoteState::Unknown;
                    }
                    terminal.step("failed publication reader setup pin release", released);
                } else {
                    setup.confirmed = true;
                }
                lctx_model::domain::completion::complete(Err(error), terminal)
            }
        }
    }
    /// Caller retains the factory before await and drains it on construction failure.
    pub async fn connect(
        endpoint: &str,
        credentials: &Credentials,
        handle: SnapshotHandle,
        budget: &lctx_model::domain::resources::ResourceBudget,
        factory: &Arc<crate::control::RecoveryFactoryOwner>,
    ) -> Result<Self, ModelError> {
        if factory.binding().generation != handle.service_generation
            || !factory.budget().shares_pool(budget)
        {
            return Err(ModelError::Conflict(
                "publication reader factory budget/generation",
            ));
        }
        let cancellation = crate::control::RetryCancellation::default();
        let stop = cancellation.clone();
        let signals = Arc::new(crate::signal::SignalRegistration::with_receipt(
            "publication native reader",
            move || stop.cancel(),
            factory.binding().clone(),
        )?);
        let client = Arc::new(Surreal::init());
        let recovery = crate::control::RecoveryScope::new(
            client.clone(),
            budget.clone(),
            cancellation,
            signals.pending_recipient()?,
            crate::control::RecoveryTarget {
                generation: handle.service_generation,
                namespace: handle.database.namespace.as_str().into(),
                database: handle.database.database.as_str().into(),
                endpoint: Some(endpoint.into()),
            },
        )?;
        let lease = factory.attach(recovery.clone(), signals.clone())?;
        let result = crate::control::with_recovery_scope(recovery.clone(), async {
            connect_with_client(
                client.clone(),
                endpoint,
                credentials,
                handle.database.namespace.as_str(),
                handle.database.database.as_str(),
                Some(budget),
                false,
            )
            .await?;
            Self::open(client.clone(), handle).await
        })
        .await;
        match result {
            Ok(reader) => {
                let reader = reader.with_budget(budget);
                {
                    let mut operations = reader
                        .operations
                        .lock()
                        .map_err(|_| ModelError::Conflict("publication reader session owner"))?;
                    operations.owns_session = true;
                    operations.recovery = Some(recovery);
                    operations.signals = Some(signals);
                    operations.factory = Some(lease);
                }
                Ok(reader)
            }
            Err(error) => {
                if !error.permits_storage_cleanup() || recovery.pending_count()? != 0 {
                    return Err(factory.handback(error));
                }
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.step(
                    "failed publication reader owner close",
                    lease.complete().await,
                );
                lctx_model::domain::completion::complete(Err(error), completion)
                    .map_err(|error| factory.handback(error))
            }
        }
    }
    pub fn handle(&self) -> &SnapshotHandle {
        &self.handle
    }
}
impl NativeReader<()> {
    /// Borrow internal selectors only while their actual operation remains live.
    /// The caller must drain this reader beneath its source operation's read lease;
    /// closing it never releases source pins or invalidates the shared SDK session.
    pub async fn borrow_installed_views(
        views: &[Arc<crate::installed_view::HeldInstalledView>],
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let first = views.first().ok_or(ModelError::Conflict(
            "borrowed reader requires explicit completed views",
        ))?;
        let client = first.validation_client();
        let charge = budget.reserve(
            "borrowed reader exact capability inventory",
            views
                .len()
                .saturating_mul(
                    std::mem::size_of::<Arc<crate::installed_view::HeldInstalledView>>()
                        + std::mem::size_of::<lctx_model::domain::ContentHash>(),
                )
                .saturating_add(1024),
        )?;
        let guard_charge = budget.reserve(
            "borrowed reader actual source pin handles",
            views
                .len()
                .saturating_mul(std::mem::size_of::<Arc<crate::control::ReaderPin>>())
                .saturating_add(std::mem::size_of::<BorrowedInstalledGuard>())
                .saturating_add(512),
        )?;
        let mut pins = Vec::with_capacity(views.len());
        let mut logical = Vec::with_capacity(views.len());
        let mut context = None;
        for view in views {
            if !Arc::ptr_eq(&client, &view.validation_client()) {
                return Err(ModelError::Conflict(
                    "borrowed reader foreign source session",
                ));
            }
            let source = view.earned_source().ok_or(ModelError::Conflict(
                "borrowed reader requires internal earned selectors",
            ))?;
            source.pin().ensure_live()?;
            if context.is_some_and(|previous| previous != source.context()) {
                return Err(ModelError::Conflict(
                    "borrowed reader source context contradiction",
                ));
            }
            context = Some(source.context());
            if !pins.iter().any(|pin| Arc::ptr_eq(pin, source.pin())) {
                pins.push(source.pin().clone());
            }
            let identity = view.completion().view.identity;
            if logical.contains(&identity) {
                return Err(ModelError::Conflict(
                    "borrowed reader duplicate logical view",
                ));
            }
            logical.push(identity);
        }
        let borrowed = Arc::new(BorrowedInstalledGuard {
            client: client.clone(),
            pins,
            context: context.expect("nonempty earned sources"),
            _charge: guard_charge,
        });
        let mut setup = InstalledReaderSetup {
            views: views.to_vec(),
            pin: None,
            client: client.clone(),
            confirmed: false,
        };
        let result = borrowed.ensure_current(budget).await;
        if let Err(error) = result {
            setup.confirmed = error.permits_storage_cleanup();
            return Err(error);
        }
        let installed = std::mem::take(&mut setup.views);
        setup.confirmed = true;
        Ok(Self {
            operations: Default::default(),
            client,
            handle: (),
            derivation: None,
            cancellation: None,
            budget: Some(budget.clone()),
            request_budget: Some(budget.clone()),
            scope: Some(Arc::new(ReaderScope {
                publication: None,
                originals: None,
                views: logical,
                exact: Some(Arc::new(ExactViewSelection {
                    references: vec![],
                    _installed: installed,
                    _charge: charge,
                })),
                _pin: None,
                release_pin_on_close: false,
                borrowed: Some(borrowed),
                selection: std::sync::Mutex::new(None),
            })),
        })
    }
    /// Select only the exact namespaces already earned by native view factories.
    /// These capabilities stay in the reader scope through every clone and drain.
    pub async fn for_installed_views(
        client: Arc<Surreal<Client>>,
        views: &[Arc<crate::installed_view::HeldInstalledView>],
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        if views.is_empty() {
            return Err(ModelError::Conflict(
                "installed reader requires explicit completed views",
            ));
        }
        let charge = budget.reserve(
            "installed reader exact capability inventory",
            views
                .len()
                .saturating_mul(
                    std::mem::size_of::<crate::completed_state::InstalledViewRef>()
                        + std::mem::size_of::<Arc<crate::installed_view::HeldInstalledView>>()
                        + std::mem::size_of::<lctx_model::domain::ContentHash>(),
                )
                .saturating_add(1024),
        )?;
        let mut logical = Vec::with_capacity(views.len());
        for view in views {
            let identity = view.completion().view.identity;
            if logical.contains(&identity) {
                return Err(ModelError::Conflict(
                    "installed reader duplicate logical view",
                ));
            }
            logical.push(identity);
        }
        // SDK Surreal::clone creates an independently attached authenticated session.
        // Arc::clone would share the source store's invalidation lifetime.
        let parent = crate::control::capture_recovery_scope().ok_or(ModelError::Conflict(
            "independent reader requires source recovery owner",
        ))?;
        if !Arc::ptr_eq(parent.client(), &client) {
            return Err(ModelError::Conflict(
                "independent reader source session differs",
            ));
        }
        let factory = parent.factory()?;
        let (recovery, signals, lease) = parent.fork_session(budget)?;
        let result = crate::control::with_recovery_scope(recovery.clone(), async {
            let client = recovery.client().clone();
            let mut setup = InstalledReaderSetup {
                views: views.to_vec(),
                pin: None,
                client: client.clone(),
                confirmed: false,
            };
            let pin = match crate::control::ReaderPin::acquire(client.clone(), &logical).await {
                Ok(pin) => pin,
                Err(error) => {
                    if !setup.permits_cleanup(&error) {
                        return Err(error);
                    }
                    let mut terminal = lctx_model::domain::completion::Completion::default();
                    let invalidated = lease.close_session().await;
                    if invalidated.is_err() {
                        terminal.remote = lctx_model::domain::completion::RemoteState::Unknown;
                    }
                    terminal.step("failed installed reader session invalidation", invalidated);
                    let result = lctx_model::domain::completion::complete(Err(error), terminal);
                    setup.confirmed = result
                        .as_ref()
                        .err()
                        .is_none_or(|error| error.permits_storage_cleanup());
                    return result;
                }
            };
            setup.pin = Some(pin.clone());
            let reader = Self::private(client.clone()).with_budget(budget);
            let result = async {
                crate::installed_view::protect_receiving_views(views, &pin, &reader, budget)
                    .await?;
                // The exact native capabilities own these selectors already. Mechanical
                // selection borrows them; no duplicate compound descriptor is retained.
                Ok(Vec::<crate::completed_state::InstalledViewRef>::new())
            }
            .await;
            // Mark uncertainty before any cleanup can release the pin or drop a
            // recovery view. Still close the private transport to join its drains.
            if let Err(error) = &result {
                setup.permits_cleanup(error);
            }
            let closed = reader.close().await;
            if let Err(error) = &closed {
                setup.permits_cleanup(error);
            }
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step("installed reader setup private transport close", closed);
            let result = lctx_model::domain::completion::complete(result, terminal);
            let references = match result {
                Ok(references) => references,
                Err(error) => {
                    if !setup.permits_cleanup(&error) {
                        return Err(error);
                    }
                    let released = pin.release().await;
                    let may_invalidate = released
                        .as_ref()
                        .err()
                        .is_none_or(|error| setup.permits_cleanup(error));
                    let mut terminal = lctx_model::domain::completion::Completion::default();
                    terminal.step("failed installed reader setup pin release", released);
                    if may_invalidate {
                        let invalidated = lease.close_session().await;
                        if invalidated.is_err() {
                            terminal.remote = lctx_model::domain::completion::RemoteState::Unknown;
                        }
                        terminal.step("failed installed reader session invalidation", invalidated);
                    }
                    let result = lctx_model::domain::completion::complete(Err(error), terminal);
                    setup.confirmed = result
                        .as_ref()
                        .err()
                        .is_none_or(|error| error.permits_storage_cleanup());
                    return result;
                }
            };
            let installed = std::mem::take(&mut setup.views);
            let pin = setup.pin.take().expect("earned installed reader pin");
            setup.confirmed = true;
            Ok(Self {
                operations: Arc::new(std::sync::Mutex::new(QueryOperations {
                    owns_session: true,
                    recovery: Some(recovery.clone()),
                    signals: Some(signals.clone()),
                    factory: Some(lease.clone()),
                    ..Default::default()
                })),
                client,
                handle: (),
                derivation: None,
                cancellation: None,
                budget: Some(budget.clone()),
                request_budget: Some(budget.clone()),
                scope: Some(Arc::new(ReaderScope {
                    publication: None,
                    originals: None,
                    views: logical,
                    exact: Some(Arc::new(ExactViewSelection {
                        references,
                        _installed: installed,
                        _charge: charge,
                    })),
                    _pin: Some(pin),
                    release_pin_on_close: true,
                    borrowed: None,
                    selection: std::sync::Mutex::new(None),
                })),
            })
        })
        .await;
        if result
            .as_ref()
            .err()
            .is_some_and(ModelError::permits_storage_cleanup)
            && recovery.pending_count()? == 0
        {
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step(
                "failed independent reader signal finality",
                lease.complete_with(&terminal).await,
            );
            return lctx_model::domain::completion::complete(result, terminal)
                .map_err(|error| factory.handback(error));
        }
        result.map_err(|error| factory.handback(error))
    }
    /// Earn an exact physical view scope under a temporary pin. Every requested logical
    /// view is resolved through this completed state's inventory, never global discovery.
    pub async fn for_completed_views(
        client: Arc<Surreal<Client>>,
        completed: &crate::completed_state::HeldFrozenCompletedState,
        views: &[lctx_model::domain::ContentHash],
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        if views.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ModelError::Conflict(
                "completed reader requested view order/duplicate",
            ));
        }
        let data = completed.completion();
        let identity = data.identity()?;
        // Completed readers may outlive their staging/source session. Own a real
        // independent SDK session before the first effect, including setup failure.
        let parent = crate::control::capture_recovery_scope().ok_or(ModelError::Conflict(
            "independent reader requires source recovery owner",
        ))?;
        if !Arc::ptr_eq(parent.client(), &client) {
            return Err(ModelError::Conflict(
                "independent reader source session differs",
            ));
        }
        let factory = parent.factory()?;
        let (recovery, signals, lease) = parent.fork_session(budget)?;
        let result = crate::control::with_recovery_scope(recovery.clone(), async {
        let client = recovery.client().clone();
        let mut setup = InstalledReaderSetup {
            views: vec![],
            pin: None,
            client: client.clone(),
            confirmed: false,
        };
        let pin = match crate::control::ReaderPin::acquire(client.clone(), views).await {
            Ok(pin) => pin,
            Err(error) => {
                if !setup.permits_cleanup(&error) {
                    return Err(error);
                }
                let invalidated = lease.close_session().await;
                let mut completion = lctx_model::domain::completion::Completion::default();
                if invalidated.is_err() {
                    completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
                }
                completion.step(
                    "completed reader failed acquisition session invalidation",
                    invalidated,
                );
                let result = lctx_model::domain::completion::complete(Err(error), completion);
                setup.confirmed = result
                    .as_ref()
                    .err()
                    .is_none_or(|error| error.permits_storage_cleanup());
                return result;
            }
        };
        setup.pin = Some(pin.clone());
        // Every accepted read retains this pin through logical terminals and
        // physical EOF, even when the factory future itself is cancelled.
        let reader = Self::private_pinned(client.clone(), pin.clone()).with_budget(budget);
        let result=async {
            pin.protect(data.record()?).await?;pin.protect(completed.admission_record().clone()).await?;
            let actual=crate::control::inspect_expected_read(&reader,data.record()?,budget).await?
                .ok_or(ModelError::Conflict("completed reader protected completion missing"))?;
            if actual.incarnation()!=completed.incarnation() || actual.row()!=data.value(true)? {
                return Err(ModelError::Conflict("completed reader completion ABA/bytes"));
            }
            let admission=crate::control::inspect_expected_read(&reader,completed.admission_record().clone(),budget).await?
                .ok_or(ModelError::Conflict("completed reader protected admission missing"))?;
            if admission.incarnation()!=completed.admission_incarnation() {
                return Err(ModelError::Conflict("completed reader admission ABA"));
            }
            let mut charge=budget.reserve("completed reader exact view references",views.len()
                .saturating_mul(std::mem::size_of::<crate::completed_state::InstalledViewRef>()
                    +std::mem::size_of::<lctx_model::domain::ContentHash>()).saturating_add(1024))?;
            let mut references=Vec::with_capacity(views.len());let mut pairs=std::collections::BTreeSet::new();
            let mut copies=std::collections::BTreeMap::new();
            for view in views {
                let reference=completed.inventory().view(*view)?;
                charge.try_resize(charge.size().saturating_add(serde_json::to_vec(reference).map_err(ModelError::codec)?.len().saturating_mul(8).saturating_add(2048)))?;
                pairs.insert((data.record()?,reference.record(identity)));
                for mapping in &reference.selector.mappings {
                    let copy=completed.inventory().copies().get(&mapping.physical).ok_or(ModelError::Conflict("completed reader exact selected contributor copy missing"))?;
                    if copy.incarnation!=mapping.incarnation || copy.descriptor.identity()?!=mapping.logical {return Err(ModelError::Conflict("completed reader selected contributor incidence"));}
                    copies.insert(RecordId::new("compiler_contribution",mapping.physical.hex()),copy);
                }
                references.push(reference.clone());
            }
            let children=references.iter().map(|reference|reference.record(identity)).collect::<Vec<_>>();
            for children in children.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let mut observed=crate::control::inspect_expected_window_read(&reader,children,budget).await?;
                for child in children {
                    let row=observed.take_required(child)?;
                    let reference=references.iter().find(|reference|&reference.record(identity)==child).ok_or(ModelError::Conflict("completed reader child correspondence"))?;
                    if row.row()!=reference.value(identity)? {return Err(ModelError::Conflict("completed reader exact selector full bytes"));}
                }
                observed.finish()?;
            }
            let physical=copies.keys().cloned().collect::<Vec<_>>();
            for physical in physical.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let mut observed=crate::control::inspect_expected_window_read(&reader,physical,budget).await?;
                let copy_ids=physical.iter().map(|owner|copies[owner].record(identity)).collect::<Vec<_>>();
                let mut copy_rows=crate::control::inspect_expected_window_read(&reader,&copy_ids,budget).await?;
                for owner in physical {
                    let copy=copies[owner];let row=observed.take_required(owner)?;
                    crate::completed_rows::validate_state_row("compiler_contribution",row.row())?;
                    if row.incarnation()!=copy.incarnation || crate::installed_view::audit::descriptor::<lctx_model::domain::completed::CompletedContribution>(row.row())?!=copy.descriptor {
                        return Err(ModelError::Conflict("completed reader contributor full descriptor/incarnation"));
                    }
                    let copy_id=copy.record(identity);let copy_row=copy_rows.take_required(&copy_id)?;
                    if copy_row.row()!=copy.value(identity)? {return Err(ModelError::Conflict("completed reader contributor exact compound copy"));}
                    pairs.insert((data.record()?,owner.clone()));pairs.insert((data.record()?,copy_id));
                }
                observed.finish()?;copy_rows.finish()?;
            }
            let pairs=pairs.into_iter().collect::<Vec<_>>();
            for pairs in pairs.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
                let mut observed=crate::control::inspect_hold_window_read(&reader,pairs,budget).await?;
                for (owner,object) in pairs {drop(observed.take_required(owner,object)?);}
                observed.finish()?;
            }
            let mut variables=Variables::new();variables.insert("generation",data.generation.hex());variables.insert("validity_epoch",data.validity_epoch);
            let current:Vec<Value>=reader.query_native("SELECT VALUE id FROM native_installation:current WHERE generation=$generation AND validity_epoch=$validity_epoch",variables).await?;
            if current.len()!=1{return Err(ModelError::Conflict("completed reader validity epoch changed"));}
            charge.try_resize(charge.size().saturating_add(references.capacity().saturating_mul(std::mem::size_of::<crate::completed_state::InstalledViewRef>())))?;
            Ok(Arc::new(ExactViewSelection{references,_installed:vec![],_charge:charge}))
        }.await;
        if let Err(error) = &result {
            setup.permits_cleanup(error);
        }
        let closed = reader.close().await;
        if let Err(error) = &closed {
            setup.permits_cleanup(error);
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("completed reader setup private reader close", closed);
        let result = lctx_model::domain::completion::complete(result, completion);
        let exact = match result {
            Ok(exact) => exact,
            Err(error) => {
                if !setup.permits_cleanup(&error) {
                    return Err(error);
                }
                let released = pin.release().await;
                let may_invalidate = released
                    .as_ref()
                    .err()
                    .is_none_or(|error| setup.permits_cleanup(error));
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.step("completed reader failed setup pin release", released);
                if may_invalidate {
                    let invalidated = lease.close_session().await;
                    if invalidated.is_err() {
                        completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
                    }
                    completion.step(
                        "completed reader failed setup session invalidation",
                        invalidated,
                    );
                }
                let result = lctx_model::domain::completion::complete(Err(error), completion);
                setup.confirmed = result
                    .as_ref()
                    .err()
                    .is_none_or(|error| error.permits_storage_cleanup());
                return result;
            }
        };
        let pin = setup.pin.take().expect("earned completed reader pin");
        setup.confirmed = true;
        Ok(Self {
            operations: Arc::new(std::sync::Mutex::new(QueryOperations {
                owns_session: true,
                recovery: Some(recovery.clone()),
                signals: Some(signals.clone()),
                factory: Some(lease.clone()),
                ..Default::default()
            })),
            client,
            handle: (),
            derivation: None,
            cancellation: None,
            budget: Some(budget.clone()),
            request_budget: Some(budget.clone()),
            scope: Some(Arc::new(ReaderScope {
                publication: None,
                originals: None,
                views: views.to_vec(),
                exact: Some(exact),
                _pin: Some(pin),
                release_pin_on_close: true,
                borrowed: None,
                selection: std::sync::Mutex::new(None),
            })),
        })
        }).await;
        if result
            .as_ref()
            .err()
            .is_some_and(ModelError::permits_storage_cleanup)
            && recovery.pending_count()? == 0
        {
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step(
                "failed independent reader signal finality",
                lease.complete_with(&terminal).await,
            );
            return lctx_model::domain::completion::complete(result, terminal)
                .map_err(|error| factory.handback(error));
        }
        result.map_err(|error| factory.handback(error))
    }
    pub fn for_views(
        client: Arc<Surreal<Client>>,
        views: Vec<lctx_model::domain::ContentHash>,
    ) -> Self {
        Self {
            operations: Default::default(),
            client,
            handle: (),
            derivation: None,
            cancellation: None,
            budget: None,
            request_budget: None,
            scope: Some(Arc::new(ReaderScope {
                publication: None,
                exact: None,
                originals: None,
                views,
                _pin: None,
                release_pin_on_close: true,
                borrowed: None,
                selection: std::sync::Mutex::new(None),
            })),
        }
    }

    /// Owner-supplied private native access, with no published snapshot capability.
    /// The owner must drain/discard its private target after any terminal failure.
    pub fn private(client: Arc<Surreal<Client>>) -> Self {
        Self {
            operations: Default::default(),
            client,
            handle: (),
            derivation: None,
            cancellation: None,
            budget: None,
            request_budget: None,
            scope: None,
        }
    }
    pub(crate) fn private_owner_bytes() -> usize {
        size_of::<std::sync::Mutex<QueryOperations>>()
            + size_of::<PrivateRecoveryRead>()
            + 4 * size_of::<usize>()
    }
    /// Borrow a recovery owner's pin through every accepted operation. Closing this
    /// private transport drains its operations; the recovery owner releases the pin.
    pub(crate) fn private_pinned(
        client: Arc<Surreal<Client>>,
        pin: Arc<crate::control::ReaderPin>,
    ) -> Self {
        Self {
            scope: Some(Arc::new(ReaderScope {
                publication: None,
                exact: None,
                originals: None,
                views: vec![],
                _pin: Some(pin),
                release_pin_on_close: false,
                borrowed: None,
                selection: std::sync::Mutex::new(None),
            })),
            ..Self::private(client)
        }
    }
}

/// Preserve recognized work ceilings and the original typed SDK cause.
pub fn sdk_error(error: surrealdb::Error) -> ModelError {
    if matches!(
        error.query_details(),
        Some(surrealdb::types::QueryError::TimedOut { .. })
    ) {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "native SDK refusal",
            Err(ModelError::Cause(Box::new(error))),
        );
        lctx_model::domain::completion::complete::<()>(
            Err(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::ResourceRefused,
            )),
            completion,
        )
        .unwrap_err()
    } else {
        ModelError::Cause(Box::new(error))
    }
}
fn unconfirmed_stream(error: ModelError) -> ModelError {
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
    lctx_model::domain::completion::complete::<()>(Err(error), completion).unwrap_err()
}

// A queued value owns its destination allocation before the producer releases
// its source batch. next() transfers this lease to the cursor until acknowledgment.
struct OwnedRow {
    value: Value,
    reservation: Arc<dyn surrealdb::method::IngressReservation>,
}
/// Admitted delivery for native preparation drivers. A refused allocation is an
/// error; only receiver cancellation is reported as false.
pub struct NativeRowSender {
    sender: tokio::sync::mpsc::Sender<OwnedRow>,
    owner: Arc<dyn surrealdb::method::IngressAdmission>,
    _queue_storage: Arc<dyn surrealdb::method::IngressReservation>,
}
impl NativeRowSender {
    pub async fn send(&self, mut value: Value) -> Result<bool, ModelError> {
        if self.sender.is_closed() {
            return Ok(false);
        }
        let reservation = self
            .owner
            .reserve(
                surrealdb::method::IngressStage::ValueConversion,
                native_retained_bytes(&value)?,
            )
            .map_err(sdk_error)?;
        // Bytes can share an entire source frame. Replace that backing while
        // both the source owner and destination reservation remain live.
        materialize_native_bytes(&mut value);
        Ok(self
            .sender
            .send(OwnedRow { value, reservation })
            .await
            .is_ok())
    }
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
    pub async fn closed(&self) {
        self.sender.closed().await;
    }
}
fn owned_row_queue_bytes() -> usize {
    use std::mem::{align_of, size_of};
    // Tokio 1.53.2 mpsc::block: four-word header plus BLOCK_CAP values.
    // Capacity one permits one write at a time: two retained list blocks plus
    // one candidate during grow/reuse. Include alignment at each boundary.
    let block_cap = if usize::BITS == 64 { 32 } else { 16 };
    let blocks =
        3 * (4 * size_of::<usize>() + block_cap * size_of::<OwnedRow>() + align_of::<OwnedRow>());
    // chan::Chan has two CachePadded fields, Notify, semaphore, counters and
    // RxFields; include final and Arc-header 128-byte alignment separately.
    let channel = 4 * 128
        + size_of::<tokio::sync::Notify>()
        + size_of::<tokio::sync::Semaphore>()
        + 7 * size_of::<usize>();
    // oneshot::Inner contains state, Option<T>, two Task/Waker slots and Arc
    // counters. The shared terminal future/task is independently runtime-owned.
    let terminal = size_of::<Option<Result<(), ModelError>>>()
        + 3 * size_of::<usize>()
        + 2 * size_of::<std::task::Waker>()
        + 4 * align_of::<Option<Result<(), ModelError>>>();
    blocks + channel + terminal
}

enum NativeRowsSource {
    // Synthetic drivers and pure controls already own their allocation admission.
    Synthetic(
        futures::stream::BoxStream<'static, surrealdb::Result<surrealdb::method::StreamItem>>,
    ),
    Admitted(
        futures::stream::BoxStream<
            'static,
            Result<surrealdb::method::AdmittedBatchItem, surrealdb::method::AdmittedBatchFailure>,
        >,
    ),
    Owned(tokio::sync::mpsc::Receiver<OwnedRow>),
}
pub struct NativeRows {
    stream: NativeRowsSource,
    pending: Option<(usize, std::vec::IntoIter<Value>)>,
    reservations: Vec<Arc<dyn surrealdb::method::IngressReservation>>,
    results: Option<Vec<usize>>,
    statement_shape: u8,
    // Declaration order matters: dropping the stream signals cancellation before the final
    // session handle can be released. Explicit drainage retains both through completion.
    client: Option<Arc<Surreal<Client>>>,
    transaction: Option<Arc<surrealdb::method::Transaction<Client>>>,
    scope: Option<Arc<ReaderScope>>,
    statements: usize,
    ended: usize,
    exhausted: bool,
    failure: Option<Arc<ModelError>>,
    drain_errors: Vec<Arc<ModelError>>,
    drain_row_order_error: bool,
    row_bytes: usize,
    owned_result: Option<DrainCompletion>,
    current_row_reservation: Option<Arc<dyn surrealdb::method::IngressReservation>>,
    owned_storage: Option<Arc<dyn surrealdb::method::IngressReservation>>,
}
impl NativeRows {
    /// The driver owns all native preparation through its checked terminal. Receiver loss
    /// stops delivery; admitted native reads still drain before the synthetic terminal.
    pub fn owned<F, Fut>(
        budget: lctx_model::domain::resources::ResourceBudget,
        driver: F,
    ) -> Result<Self, ModelError>
    where
        F: FnOnce(NativeRowSender) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), ModelError>> + Send + 'static,
    {
        let owner = ingress::admission(&budget);
        let storage = owner
            .reserve(
                surrealdb::method::IngressStage::ValueConversion,
                owned_row_queue_bytes(),
            )
            .map_err(sdk_error)?;
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (terminal_sender, terminal_receiver) = tokio::sync::oneshot::channel();
        let sender = NativeRowSender {
            sender,
            owner,
            _queue_storage: storage.clone(),
        };
        tokio::spawn(async move {
            let result = driver(sender).await;
            let _ = terminal_sender.send(result);
        });
        let result = async move {
            terminal_receiver
                .await
                .map_err(|error| {
                    Arc::new(ModelError::infrastructure(
                        lctx_model::domain::Infrastructure::Unconfirmed,
                        format!("selected driver lost terminal: {error}"),
                    ))
                })?
                .map_err(Arc::new)
        }
        .boxed()
        .shared();
        let mut rows = Self::new(futures::stream::empty(), 1)?;
        rows.stream = NativeRowsSource::Owned(receiver);
        rows.owned_result = Some(result);
        rows.owned_storage = Some(storage);
        Ok(rows)
    }
    pub fn new(
        stream: impl futures::Stream<Item = surrealdb::Result<surrealdb::method::StreamItem>>
        + Send
        + 'static,
        statements: usize,
    ) -> Result<Self, ModelError> {
        if statements == 0 {
            return Err(ModelError::Schema("native stream statement inventory"));
        }
        Ok(Self {
            stream: NativeRowsSource::Synthetic(stream.boxed()),
            pending: None,
            reservations: Vec::new(),
            results: None,
            statement_shape: 0,
            client: None,
            transaction: None,
            scope: None,
            statements,
            ended: 0,
            exhausted: false,
            failure: None,
            drain_errors: Vec::new(),
            drain_row_order_error: false,
            row_bytes: 1024 * 1024,
            owned_result: None,
            current_row_reservation: None,
            owned_storage: None,
        })
    }
    pub(crate) fn new_admitted(
        stream: impl futures::Stream<
            Item = Result<
                surrealdb::method::AdmittedBatchItem,
                surrealdb::method::AdmittedBatchFailure,
            >,
        > + Send
        + 'static,
        statements: usize,
        results: Vec<usize>,
    ) -> Result<Self, ModelError> {
        if results.is_empty()
            || results.iter().any(|index| *index >= statements)
            || results.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ModelError::Schema("native admitted result inventory"));
        }
        let mut rows = Self::new(futures::stream::empty(), statements)?;
        rows.stream = NativeRowsSource::Admitted(stream.boxed());
        rows.results = Some(results);
        Ok(rows)
    }
    fn leased_error(&mut self, source: ModelError) -> ModelError {
        ModelError::Cause(Box::new(ingress::LeasedFailure {
            source,
            _reservations: std::mem::take(&mut self.reservations),
        }))
    }
    async fn next_item(&mut self) -> Option<Result<surrealdb::method::StreamItem, ModelError>> {
        use surrealdb::method::{BatchItem, StreamItem};
        loop {
            if let Some((statement, values)) = &mut self.pending
                && let Some(value) = values.next()
            {
                let statement = *statement;
                if self
                    .results
                    .as_ref()
                    .is_some_and(|results| !results.contains(&statement))
                {
                    if statement != self.ended || statement >= self.statements {
                        return Some(Err(unconfirmed_stream(ModelError::Schema(
                            "native preparation row order",
                        ))));
                    }
                    if !matches!(value, Value::None) {
                        return Some(Err(ModelError::Schema(
                            "native preparation emitted payload",
                        )));
                    }
                    continue;
                }
                return Some(Ok(StreamItem::Row { statement, value }));
            }
            // Polling again acknowledges that the previously delivered Value was
            // dropped or acquired replacement admission. The iterator's backing
            // allocation is destroyed before its ingress reservations are released.
            self.pending.take();
            self.reservations.clear();
            self.current_row_reservation.take();
            let admitted = match &mut self.stream {
                NativeRowsSource::Synthetic(stream) => {
                    return stream
                        .next()
                        .await
                        .map(|item| item.map_err(|error| unconfirmed_stream(sdk_error(error))));
                }
                NativeRowsSource::Admitted(stream) => stream.next().await?,
                NativeRowsSource::Owned(receiver) => {
                    if let Some(OwnedRow { value, reservation }) = receiver.recv().await {
                        self.current_row_reservation = Some(reservation);
                        return Some(Ok(StreamItem::Row {
                            statement: 0,
                            value,
                        }));
                    }
                    if self.ended == 0 {
                        if let Some(result) = self.owned_result.as_ref().cloned() {
                            let _ = result.await;
                        }
                        return Some(Ok(StreamItem::StatementEnd {
                            statement: 0,
                            stats: Default::default(),
                            result: Ok(()),
                        }));
                    }
                    return None;
                }
            };
            let admitted = match admitted {
                Ok(item) => item,
                Err(failure) => {
                    let (error, reservations) = failure.into_parts();
                    return Some(Err(ModelError::Cause(Box::new(ingress::LeasedFailure {
                        source: unconfirmed_stream(sdk_error(error)),
                        _reservations: reservations,
                    }))));
                }
            };
            let (item, reservations) = admitted.into_parts();
            self.reservations = reservations;
            let index = item.index();
            if !matches!(item, BatchItem::Finished { .. }) {
                if index != self.ended || index >= self.statements {
                    return Some(Err(unconfirmed_stream(ModelError::Schema(
                        "native admitted payload order",
                    ))));
                }
                let shape = if matches!(item, BatchItem::Rows { .. }) {
                    1
                } else {
                    2
                };
                if self.statement_shape != 0 && (shape != self.statement_shape || shape == 2) {
                    return Some(Err(unconfirmed_stream(ModelError::Schema(
                        "native admitted mixed/repeated value shape",
                    ))));
                }
                self.statement_shape = shape;
            }
            match item {
                BatchItem::Rows { index, values } => {
                    self.pending = Some((index, values.into_iter()))
                }
                BatchItem::Value { index, value } => {
                    if self
                        .results
                        .as_ref()
                        .is_some_and(|results| !results.contains(&index))
                    {
                        if matches!(value, Value::None) {
                            continue;
                        }
                        return Some(Err(ModelError::Schema(
                            "native preparation emitted payload",
                        )));
                    }
                    return Some(Ok(StreamItem::Row {
                        statement: index,
                        value,
                    }));
                }
                BatchItem::EmptyValue { .. } => continue,
                BatchItem::Finished {
                    index,
                    time,
                    query_type,
                    error,
                } => {
                    self.statement_shape = 0;
                    return Some(Ok(StreamItem::StatementEnd {
                        statement: index,
                        stats: surrealdb::method::StreamStats::default()
                            .with_execution_time(time)
                            .with_query_type(query_type),
                        result: error.map_or(Ok(()), Err),
                    }));
                }
            }
        }
    }
    /// Close synthetic delivery while keeping its independent driver terminal alive.
    /// Raw transport streams remain owned until checked drainage.
    pub fn cancel_delivery(&mut self) {
        if self.owned_result.is_some() {
            self.stream = NativeRowsSource::Synthetic(futures::stream::empty().boxed());
            self.exhausted = true;
            self.ended = self.statements;
        }
    }
    pub fn with_client(mut self, client: Arc<Surreal<Client>>) -> Self {
        self.client = Some(client);
        self
    }
    pub fn with_transaction(
        mut self,
        transaction: Arc<surrealdb::method::Transaction<Client>>,
    ) -> Self {
        self.transaction = Some(transaction);
        self
    }
    pub fn with_row_bytes(mut self, row_bytes: usize) -> Self {
        self.row_bytes = row_bytes;
        self
    }
    /// Finish the SDK transport after a semantic or envelope failure. The original failure
    /// remains sticky; draining does not turn the failed query into an accepted result.
    pub fn remember_failure(&mut self, error: ModelError) -> ModelError {
        let cause = self.failure.get_or_insert_with(|| Arc::new(error)).clone();
        ModelError::SharedCause(cause)
    }
    /// Consume every remaining statement and the outer transport, retaining late errors.
    /// Interrupted drainage can be retried on this same owner; it never admits failed rows.
    pub async fn drain_transport(&mut self) -> Result<(), ModelError> {
        if !self.exhausted {
            while let Some(item) = self.next_item().await {
                match item {
                    Err(error) => self.drain_errors.push(Arc::new(error)),
                    Ok(surrealdb::method::StreamItem::Row { statement, .. }) => {
                        if (statement != self.ended || statement >= self.statements)
                            && !self.drain_row_order_error
                        {
                            self.drain_row_order_error = true;
                            self.drain_errors.push(Arc::new(unconfirmed_stream(
                                ModelError::Schema("native stream drainage row order"),
                            )));
                        }
                    }
                    Ok(surrealdb::method::StreamItem::StatementEnd {
                        statement, result, ..
                    }) => {
                        if statement != self.ended || statement >= self.statements {
                            self.drain_errors.push(Arc::new(unconfirmed_stream(
                                ModelError::Schema("native stream drainage terminal order"),
                            )));
                        } else {
                            self.ended += 1;
                        }
                        if let Err(error) = result {
                            let error = self.leased_error(sdk_error(error));
                            self.drain_errors.push(Arc::new(error));
                        }
                    }
                }
            }
            self.exhausted = true;
            if self.ended != self.statements {
                self.drain_errors.push(Arc::new(ModelError::infrastructure(
                    lctx_model::domain::Infrastructure::Unconfirmed,
                    "native stream drainage missing terminal",
                )));
            }
        }
        if self.ended != self.statements
            && let Some(scope) = &self.scope
        {
            scope.retain_unknown();
        }
        if let Some(result) = self.owned_result.as_ref().cloned() {
            let result = result.await;
            self.owned_result.take();
            if let Err(error) = result {
                self.drain_errors.push(error);
            }
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        if self.ended != self.statements
            || self
                .failure
                .as_ref()
                .is_some_and(|error| !error.permits_storage_cleanup())
        {
            completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
        }
        for error in &self.drain_errors {
            completion.step(
                "native stream finalization",
                Err(ModelError::SharedCause(error.clone())),
            );
        }
        if completion.remote == lctx_model::domain::completion::RemoteState::Unknown
            && let Some(scope) = &self.scope
        {
            scope.retain_unknown();
        }
        self.scope.take();
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
    /// The returned Value is covered by this owner's current batch leases until
    /// the next poll, drainage or owner drop. Native consumers must drop it or
    /// acquire their own admission before that transfer acknowledgment; arbitrary
    /// escaped Values are not covered after this owner is released.
    pub async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        if let Some(error) = &self.failure {
            return Err(ModelError::SharedCause(error.clone()));
        }
        // Physical EOF removes the stream scope, so retain this small source guard
        // across the await and through delivery of either a row or complete EOF.
        let borrowed = self.scope.as_ref().and_then(|scope| scope.borrowed.clone());
        if let Some(borrowed) = &borrowed
            && let Err(error) = borrowed.ensure_live()
        {
            return Err(self.remember_failure(error));
        }
        match self.read_next().await {
            Ok(row) => {
                if let Some(borrowed) = &borrowed
                    && let Err(error) = borrowed.ensure_live()
                {
                    return Err(self.remember_failure(error));
                }
                Ok(row)
            }
            Err(error) => Err(self.remember_failure(error)),
        }
    }
    async fn read_next(&mut self) -> Result<Option<Value>, ModelError> {
        if self.exhausted && self.owned_result.is_none() {
            return Ok(None);
        }
        while let Some(item) = self.next_item().await {
            match item? {
                surrealdb::method::StreamItem::Row { statement, value } => {
                    if statement != self.ended || statement >= self.statements {
                        return Err(unconfirmed_stream(ModelError::Schema(
                            "native stream row order",
                        )));
                    }
                    // Transport, protobuf and conversion allocations already carry
                    // admission; this is the model's separate per-row contract.
                    let bytes = crate::loader::native_bytes(&value);
                    if bytes > self.row_bytes {
                        return Err(ModelError::Limit {
                            owner: "native-stream",
                            limit: "row bytes",
                            observed: bytes,
                            bound: self.row_bytes,
                        });
                    }
                    return Ok(Some(value));
                }
                surrealdb::method::StreamItem::StatementEnd {
                    statement, result, ..
                } => {
                    if statement != self.ended || statement >= self.statements {
                        return Err(unconfirmed_stream(ModelError::Schema(
                            "native stream terminal order",
                        )));
                    }
                    self.ended += 1;
                    if let Err(error) = result {
                        return Err(self.leased_error(sdk_error(error)));
                    }
                }
            }
        }
        self.exhausted = true;
        if self.ended != self.statements {
            if let Some(scope) = &self.scope {
                scope.retain_unknown();
            }
            return Err(ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed,
                "native stream missing terminal success",
            ));
        }
        if let Some(result) = self.owned_result.as_ref().cloned() {
            let result = result.await;
            self.owned_result.take();
            if let Err(error) = result {
                if !error.permits_storage_cleanup()
                    && let Some(scope) = &self.scope
                {
                    scope.retain_unknown();
                }
                return Err(ModelError::SharedCause(error));
            }
        }
        self.scope.take();
        Ok(None)
    }
}

impl Drop for NativeRows {
    fn drop(&mut self) {
        let scope = self.scope.take();
        // Private factory readers and owned preparation drivers have no publication
        // scope, but still own accepted transport and a terminal report. Scope absence
        // must not turn receiver cancellation into abandonment of that report.
        if scope.is_none()
            && self.client.is_none()
            && self.transaction.is_none()
            && self.owned_result.is_none()
        {
            return;
        }
        if self.exhausted && self.owned_result.is_none() {
            if (self.ended != self.statements
                || self
                    .failure
                    .as_ref()
                    .is_some_and(|error| !error.permits_storage_cleanup())
                || self
                    .drain_errors
                    .iter()
                    .any(|error| !error.permits_storage_cleanup()))
                && let Some(scope) = &scope
            {
                scope.retain_unknown();
            }
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            if let Some(scope) = &scope {
                scope.retain_unknown();
            }
            // No executor can establish remote finality. Preserve the actual
            // accepted request, session and reservations instead of dropping
            // them and making unresolved execution look like released ownership.
            std::mem::forget((
                std::mem::replace(
                    &mut self.stream,
                    NativeRowsSource::Synthetic(futures::stream::empty().boxed()),
                ),
                self.pending.take(),
                std::mem::take(&mut self.reservations),
                self.client.take(),
                self.transaction.take(),
                scope,
                self.owned_result.take(),
                self.current_row_reservation.take(),
                self.owned_storage.take(),
            ));
            return;
        };
        self.cancel_delivery();
        let mut drain = NativeRows {
            stream: std::mem::replace(
                &mut self.stream,
                NativeRowsSource::Synthetic(futures::stream::empty().boxed()),
            ),
            pending: self.pending.take(),
            reservations: std::mem::take(&mut self.reservations),
            results: self.results.take(),
            statement_shape: self.statement_shape,
            client: self.client.take(),
            transaction: self.transaction.take(),
            scope,
            statements: self.statements,
            ended: self.ended,
            exhausted: self.exhausted,
            failure: self.failure.take(),
            drain_errors: std::mem::take(&mut self.drain_errors),
            drain_row_order_error: self.drain_row_order_error,
            row_bytes: self.row_bytes,
            owned_result: self.owned_result.take(),
            current_row_reservation: self.current_row_reservation.take(),
            owned_storage: self.owned_storage.take(),
        };
        runtime.spawn(async move {
            let result = drain.drain_transport().await;
            if result.is_err() || drain.failure.is_some() {
                tracing::error!(primary = ?drain.failure, completion = ?result, "cancelled native read terminal outcome");
            }
        });
    }
}

#[cfg(test)]
mod admitted_native_rows_controls {
    use super::*;
    use lctx_model::domain::resources::ResourceBudget;
    use surrealdb::method::{
        AdmittedBatchFailure, AdmittedBatchItem, BatchItem, BatchQueryType, IngressStage,
    };

    fn end(index: usize, error: Option<surrealdb::Error>) -> BatchItem {
        BatchItem::Finished {
            index,
            time: Default::default(),
            query_type: BatchQueryType::Other,
            error,
        }
    }
    fn charged(item: BatchItem, budget: &ResourceBudget, bytes: usize) -> AdmittedBatchItem {
        AdmittedBatchItem::new(
            item,
            vec![
                ingress::admission(budget)
                    .reserve(IngressStage::ValueConversion, bytes)
                    .unwrap(),
            ],
        )
    }
    #[tokio::test]
    async fn owned_queue_refuses_before_driver_and_reports_row_refusal() {
        let refused = ResourceBudget::fixed(owned_row_queue_bytes() - 1).unwrap();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let marker = called.clone();
        let result = NativeRows::owned(refused.clone(), move |_| async move {
            marker.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        });
        assert!(result.is_err());
        assert!(!called.load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(refused.reserved(), 0);

        let budget = ResourceBudget::fixed(owned_row_queue_bytes()).unwrap();
        let mut rows = NativeRows::owned(budget.clone(), |sender| async move {
            sender.send(Value::from_t(7)).await?;
            Ok(())
        })
        .unwrap();
        assert!(
            rows.next().await.is_err(),
            "quota refusal must not become successful missing rows"
        );
        let _ = rows.drain_transport().await;
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn owned_queue_retains_each_destination_until_acknowledgment() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let value_bytes = native_retained_bytes(&Value::from_t(1)).unwrap();
        let mut rows = NativeRows::owned(budget.clone(), |sender| async move {
            for number in 0..97 {
                if !sender.send(Value::from_t(number)).await? {
                    break;
                }
            }
            Ok(())
        })
        .unwrap();
        for number in 0..97 {
            let value = rows.next().await.unwrap().unwrap();
            assert_eq!(value, Value::from_t(number));
            assert!(budget.reserved() >= owned_row_queue_bytes() + value_bytes);
            let replacement = budget.reserve("row transfer control", value_bytes).unwrap();
            drop(value);
            drop(replacement);
        }
        assert!(rows.next().await.unwrap().is_none());
        drop(rows);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn owned_queue_detaches_shared_bytes_under_destination_admission() {
        let budget = ResourceBudget::fixed(1024 * 1024).unwrap();
        let backing = prost::bytes::Bytes::from(vec![7; 128 * 1024]);
        let slice = backing.slice(3..7);
        let old_pointer = slice.as_ptr() as usize;
        let mut rows = NativeRows::owned(budget.clone(), move |sender| async move {
            sender.send(Value::Bytes(slice.into())).await?;
            Ok(())
        })
        .unwrap();
        let value = rows.next().await.unwrap().unwrap();
        let Value::Bytes(bytes) = &value else {
            panic!("byte row");
        };
        assert_eq!(bytes.as_ref(), &[7; 4]);
        assert_ne!(
            bytes.as_ptr() as usize,
            old_pointer,
            "shared full-frame backing must not escape its original owner"
        );
        assert!(
            budget.reserved() >= owned_row_queue_bytes() + native_retained_bytes(&value).unwrap()
        );
        drop(value);
        assert!(rows.next().await.unwrap().is_none());
        drop(rows);
        assert_eq!(budget.reserved(), 0);
        drop(backing);
    }
    #[tokio::test]
    async fn one_batch_retains_leases_through_each_row_transfer() {
        let budget = ResourceBudget::fixed(1024).unwrap();
        let item = charged(
            BatchItem::Rows {
                index: 0,
                values: vec![Value::from_t(1), Value::from_t(2)],
            },
            &budget,
            256,
        );
        let mut rows = NativeRows::new_admitted(
            futures::stream::iter(vec![
                Ok(item),
                Ok(AdmittedBatchItem::new(end(0, None), vec![])),
            ]),
            1,
            vec![0],
        )
        .unwrap();
        let first = rows.next().await.unwrap().unwrap();
        assert_eq!(budget.reserved(), 256);
        let retained = budget.reserve("test consumer transfer", 128).unwrap();
        assert_eq!(
            budget.reserved(),
            384,
            "replacement and ingress overlap before acknowledgment"
        );
        drop(first);
        let second = rows.next().await.unwrap().unwrap();
        assert_eq!(
            budget.reserved(),
            384,
            "remaining batch cannot release early"
        );
        drop(second);
        assert!(rows.next().await.unwrap().is_none());
        assert_eq!(budget.reserved(), 128);
        drop(retained);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn terminal_failure_owns_its_leases_after_stream_owner_drop() {
        let budget = ResourceBudget::fixed(1024).unwrap();
        let item = charged(
            end(
                0,
                Some(surrealdb::Error::internal("typed terminal details".into())),
            ),
            &budget,
            256,
        );
        let mut rows =
            NativeRows::new_admitted(futures::stream::iter(vec![Ok(item)]), 1, vec![0]).unwrap();
        let failure = rows.next().await.unwrap_err();
        rows.drain_transport().await.unwrap();
        drop(rows);
        assert_eq!(budget.reserved(), 256);
        drop(failure);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn physically_drained_stream_failure_retains_details() {
        let budget = ResourceBudget::fixed(1024).unwrap();
        let lease = ingress::admission(&budget)
            .reserve(IngressStage::ValueConversion, 256)
            .unwrap();
        let mut rows = NativeRows::new_admitted(
            futures::stream::iter(vec![
                Ok(AdmittedBatchItem::new(end(0, None), vec![])),
                Err(AdmittedBatchFailure::new(
                    surrealdb::Error::internal("late status".into()),
                    vec![lease],
                )),
            ]),
            1,
            vec![0],
        )
        .unwrap();
        let failure = rows.next().await.unwrap_err();
        let completion = rows.drain_transport().await.unwrap_err();
        assert!(
            !completion.permits_storage_cleanup(),
            "physical EOF does not prove remote executor completion"
        );
        drop(completion);
        drop(rows);
        assert_eq!(budget.reserved(), 256);
        drop(failure);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn preparation_filter_preserves_empty_single_and_every_terminal() {
        let mut rows = NativeRows::new_admitted(
            futures::stream::iter(vec![
                Ok(AdmittedBatchItem::new(
                    BatchItem::Value {
                        index: 0,
                        value: Value::None,
                    },
                    vec![],
                )),
                Ok(AdmittedBatchItem::new(end(0, None), vec![])),
                Ok(AdmittedBatchItem::new(
                    BatchItem::EmptyValue { index: 1 },
                    vec![],
                )),
                Ok(AdmittedBatchItem::new(end(1, None), vec![])),
            ]),
            2,
            vec![1],
        )
        .unwrap();
        assert!(rows.next().await.unwrap().is_none());
        let mut rows = NativeRows::new_admitted(
            futures::stream::iter(vec![
                Ok(AdmittedBatchItem::new(
                    BatchItem::Rows {
                        index: 0,
                        values: vec![],
                    },
                    vec![],
                )),
                Ok(AdmittedBatchItem::new(
                    BatchItem::EmptyValue { index: 0 },
                    vec![],
                )),
            ]),
            1,
            vec![0],
        )
        .unwrap();
        assert!(
            rows.next().await.is_err(),
            "mixed shape must fail even without payload rows"
        );
    }
}

pub struct CanonicalRecords<R> {
    rows: NativeRows,
    positions: Vec<usize>,
    physical_kind: bool,
    failed: bool,
    marker: std::marker::PhantomData<R>,
}
impl<R: Record + DeserializeOwned> CanonicalRecords<R> {
    pub fn from_physical_rows(rows: NativeRows) -> Self {
        Self {
            rows,
            positions: vec![0],
            physical_kind: true,
            failed: false,
            marker: std::marker::PhantomData,
        }
    }
    /// Drain the owning transport without accepting a previously failed typed stream.
    /// Late statement/transport errors remain structured finalization failures.
    pub async fn drain_transport(&mut self) -> Result<(), ModelError> {
        self.rows.drain_transport().await
    }
    pub async fn next(&mut self) -> Result<Option<R>, ModelError> {
        if self.failed {
            return Err(ModelError::Conflict("failed canonical stream"));
        }
        let result = self.read_next().await;
        self.failed |= result.is_err();
        result
    }
    async fn read_next(&mut self) -> Result<Option<R>, ModelError> {
        let Some(value) = self.rows.next().await? else {
            return Ok(None);
        };
        let Value::Object(projection) = value else {
            return Err(corrupt());
        };
        let canonical = Bytes::from_value(projection.get("canonical").ok_or_else(corrupt)?.clone())
            .map_err(|_| corrupt())?;
        // Statement identity is supplied by the checked SDK envelope, not a string projection.
        // ORDER BY fields are selected for SurrealQL but never interpreted as canonical authority.
        if self.physical_kind {
            let id = RecordId::from_value(projection.get("id").cloned().ok_or_else(corrupt)?)
                .map_err(|_| corrupt())?;
            return match id.table.as_str() {
                "entity" => canonical_entity::<R>(&canonical).map(Some),
                "assertion" => canonical_assertion::<R>(&canonical).map(Some),
                _ => Err(ModelError::Schema("native canonical family")),
            };
        }
        let row = match self
            .positions
            .iter()
            .position(|position| *position == self.rows.ended)
        {
            Some(0) => canonical_entity::<R>(&canonical)?,
            Some(1) => canonical_assertion::<R>(&canonical)?,
            _ => return Err(ModelError::Schema("native stream canonical statement")),
        };
        Ok(Some(row))
    }
}
fn corrupt() -> ModelError {
    ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)
}
pub fn canonical_error(error: ModelError) -> ModelError {
    match error {
        refusal @ (ModelError::Resource { .. }
        | ModelError::Limit { .. }
        | ModelError::Serving(_)
        | ModelError::Completion(_)
        | ModelError::SharedCause(_)
        | ModelError::Cause(_)) => refusal,
        _ => corrupt(),
    }
}
fn canonical_entity<R: Record + DeserializeOwned>(payload: &[u8]) -> Result<R, ModelError> {
    canonical_entity_projection(payload).map(|(row, _)| row)
}
fn canonical_entity_projection<R: Record + DeserializeOwned>(
    payload: &[u8],
) -> Result<(R, RecordId), ModelError> {
    let entity: Entity = serde_json::from_slice(payload).map_err(|_| corrupt())?;
    entity.validate().map_err(canonical_error)?;
    Ok((
        crate::codec::entity_record::<R>(&entity).map_err(canonical_error)?,
        crate::loader::entity_payload_id(&entity)?,
    ))
}
fn canonical_assertion<R: Record + DeserializeOwned>(payload: &[u8]) -> Result<R, ModelError> {
    canonical_assertion_projection(payload).map(|(row, _)| row)
}
fn canonical_assertion_projection<R: Record + DeserializeOwned>(
    payload: &[u8],
) -> Result<(R, RecordId), ModelError> {
    let assertion: Assertion = serde_json::from_slice(payload).map_err(|_| corrupt())?;
    assertion.validate().map_err(canonical_error)?;
    Ok((
        crate::codec::assertion_record::<R>(&assertion).map_err(canonical_error)?,
        crate::loader::assertion_payload_id(&assertion)?,
    ))
}
pub fn target_id(target: Target) -> RecordId {
    match target {
        Target::Entity(id) => RecordId::new("entity_anchor", id.0.hex()),
        Target::Assertion(id) => RecordId::new("assertion_anchor", id.0.hex()),
        external @ Target::External { .. } => {
            let mut sink = KeySink::new("graph-external-endpoint/v1");
            external.encode(&mut sink);
            RecordId::new("external", sink.finish().hex())
        }
    }
}
/// An actual authenticated session owner retained through asynchronous finalization.
/// Drop before an acknowledged close preserves its handle for recovery.
pub struct SessionFinalizationGuard {
    client: Arc<Surreal<Client>>,
    closed: std::sync::atomic::AtomicBool,
}
impl SessionFinalizationGuard {
    pub fn new(client: Arc<Surreal<Client>>) -> Self {
        Self {
            client,
            closed: std::sync::atomic::AtomicBool::new(false),
        }
    }
    /// Transfer the close obligation to the returned owner; this is no terminal acknowledgement.
    pub fn transfer(self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Release);
    }
    pub async fn close(&self) -> Result<(), ModelError> {
        if self.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        let invalidated = self.client.invalidate().await.map_err(sdk_error);
        if invalidated.is_ok() {
            self.closed
                .store(true, std::sync::atomic::Ordering::Release);
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        if invalidated.is_err() {
            completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
        }
        completion.step("owned session invalidation", invalidated);
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
}
impl Drop for SessionFinalizationGuard {
    fn drop(&mut self) {
        if !self.closed.load(std::sync::atomic::Ordering::Acquire) {
            std::mem::forget(self.client.clone());
        }
    }
}
pub async fn connect(
    endpoint: &str,
    credentials: &Credentials,
    namespace: &str,
    database: &str,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    connect_inner(endpoint, credentials, namespace, database, None).await
}
pub async fn connect_admitted(
    endpoint: &str,
    credentials: &Credentials,
    namespace: &str,
    database: &str,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    connect_inner(endpoint, credentials, namespace, database, Some(budget)).await
}
/// Caller retains the factory before this await. Any interrupted construction
/// leaves a concrete child in that factory, recoverable without resetting stop.
pub async fn connect_owned(
    endpoint: &str,
    credentials: &Credentials,
    namespace: &str,
    database: &str,
    factory: &Arc<crate::control::RecoveryFactoryOwner>,
) -> Result<(Arc<Surreal<Client>>, crate::control::FactoryLease), ModelError> {
    let cancellation = crate::control::RetryCancellation::default();
    let stop = cancellation.clone();
    let signals = Arc::new(crate::signal::SignalRegistration::with_receipt(
        "owned native session",
        move || stop.cancel(),
        factory.binding().clone(),
    )?);
    let client = Arc::new(Surreal::init());
    let recovery = crate::control::RecoveryScope::new(
        client.clone(),
        factory.budget().clone(),
        cancellation,
        signals.pending_recipient()?,
        crate::control::RecoveryTarget {
            generation: factory.binding().generation,
            namespace: namespace.into(),
            database: database.into(),
            endpoint: Some(endpoint.into()),
        },
    )?;
    let lease = factory.attach(recovery.clone(), signals)?;
    let connected = crate::control::with_recovery_scope(
        recovery,
        connect_with_client(
            client.clone(),
            endpoint,
            credentials,
            namespace,
            database,
            Some(factory.budget()),
            false,
        ),
    )
    .await;
    match connected {
        Ok(client) => Ok((client, lease)),
        Err(error) => Err(factory.handback(error)),
    }
}
async fn connect_inner(
    endpoint: &str,
    credentials: &Credentials,
    namespace: &str,
    database: &str,
    budget: Option<&lctx_model::domain::resources::ResourceBudget>,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    connect_with_client(
        Arc::new(Surreal::init()),
        endpoint,
        credentials,
        namespace,
        database,
        budget,
        true,
    )
    .await
}
async fn connect_with_client(
    client: Arc<Surreal<Client>>,
    endpoint: &str,
    credentials: &Credentials,
    namespace: &str,
    database: &str,
    budget: Option<&lctx_model::domain::resources::ResourceBudget>,
    standalone: bool,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    let client = authenticated_client(
        client,
        endpoint,
        credentials,
        Some((namespace, database)),
        budget,
        standalone,
    )
    .await?;
    let session = standalone.then(|| SessionFinalizationGuard::new(client.clone()));
    let selected = client
        .use_ns(namespace)
        .use_db(database)
        .await
        .map_err(sdk_error);
    if let Err(error) = selected {
        let Some(session) = session else {
            return Err(error);
        };
        if !error.permits_storage_cleanup() {
            return Err(error);
        }
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step(
            "failed native selection session close",
            session.close().await,
        );
        return lctx_model::domain::completion::complete(Err(error), terminal);
    }
    if let Some(session) = session {
        session.transfer();
    }
    Ok(client)
}
/// Authenticate without selecting or implicitly creating a database. Private compiler creation
/// can therefore establish STRICT before selecting its first storage session.
pub async fn authenticated(
    endpoint: &str,
    credentials: &Credentials,
    scope: Option<(&str, &str)>,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    authenticated_inner(endpoint, credentials, scope, None).await
}
pub async fn authenticated_admitted(
    endpoint: &str,
    credentials: &Credentials,
    scope: Option<(&str, &str)>,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    authenticated_inner(endpoint, credentials, scope, Some(budget)).await
}
async fn authenticated_inner(
    endpoint: &str,
    credentials: &Credentials,
    scope: Option<(&str, &str)>,
    budget: Option<&lctx_model::domain::resources::ResourceBudget>,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    authenticated_client(
        Arc::new(Surreal::init()),
        endpoint,
        credentials,
        scope,
        budget,
        true,
    )
    .await
}
async fn authenticated_client(
    client: Arc<Surreal<Client>>,
    endpoint: &str,
    credentials: &Credentials,
    scope: Option<(&str, &str)>,
    budget: Option<&lctx_model::domain::resources::ResourceBudget>,
    standalone: bool,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    let mut config = surrealdb::opt::Config::new();
    if let Some(budget) = budget {
        config = config
            .grpc(
                surrealdb::opt::GrpcConfig::default().ingress_admission(ingress::admission(budget)),
            )
            .map_err(sdk_error)?;
    }
    let address = endpoint
        .strip_prefix("grpc://")
        .ok_or_else(|| ModelError::Invalid("native gRPC endpoint required".into()))?;
    let phase = crate::phase::Phase::begin("native_connection_bootstrap");
    let connected = client
        .connect::<Grpc>((address, config))
        .await
        .map_err(sdk_error);
    phase.finish_result(&connected);
    connected?;
    let session = standalone.then(|| SessionFinalizationGuard::new(client.clone()));
    let phase = crate::phase::Phase::begin("native_connection_signin");
    let signed: Result<(), ModelError> = async {
        match credentials {
            Credentials::Root { username, password } => {
                client
                    .signin(Root {
                        username: username.clone(),
                        password: password.clone(),
                    })
                    .await
                    .map_err(sdk_error)?;
            }
            Credentials::Database { username, password } => {
                let (namespace, database) =
                    scope.ok_or(ModelError::Schema("database authentication scope"))?;
                client
                    .signin(Database {
                        namespace: namespace.into(),
                        database: database.into(),
                        username: username.clone(),
                        password: password.clone(),
                    })
                    .await
                    .map_err(sdk_error)?;
            }
        }
        Ok(())
    }
    .await;
    phase.finish_result(&signed);
    if let Err(error) = signed {
        let Some(session) = session else {
            return Err(error);
        };
        if !error.permits_storage_cleanup() {
            return Err(error);
        }
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step(
            "failed native authentication session close",
            session.close().await,
        );
        return lctx_model::domain::completion::complete(Err(error), terminal);
    }
    if let Some(session) = session {
        session.transfer();
    }
    Ok(client)
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    use surrealdb::method::StreamItem;
    #[tokio::test]
    async fn installed_reader_setup_marks_unknown_before_pin_release_and_retains_session_on_abandonment()
     {
        let client = Arc::new(Surreal::init());
        let pin = crate::control::ReaderPin::test_detached(false);
        let setup = InstalledReaderSetup {
            views: vec![],
            pin: Some(pin.clone()),
            client: client.clone(),
            confirmed: false,
        };
        let unknown = ModelError::infrastructure(
            lctx_model::domain::Infrastructure::Unconfirmed,
            "setup readback has no remote terminal",
        );
        assert!(!setup.permits_cleanup(&unknown));
        assert!(pin.test_uncertain());
        // The guard marks the real pin before a release could make native effects.
        assert!(!pin.release().await.unwrap_err().permits_storage_cleanup());
        drop(setup);
        assert_eq!(
            Arc::strong_count(&client),
            2,
            "unconfirmed setup retains the actual SDK session owner"
        );

        let cancelled_client = Arc::new(Surreal::init());
        let cancelled_pin = crate::control::ReaderPin::test_detached(false);
        let cancelled = InstalledReaderSetup {
            views: vec![],
            pin: Some(cancelled_pin.clone()),
            client: cancelled_client.clone(),
            confirmed: false,
        };
        drop(cancelled);
        assert!(cancelled_pin.test_uncertain());
        assert_eq!(
            Arc::strong_count(&cancelled_client),
            2,
            "cancellation before publication also retains the session"
        );

        let confirmed_client = Arc::new(Surreal::init());
        let confirmed = InstalledReaderSetup {
            views: vec![],
            pin: None,
            client: confirmed_client.clone(),
            confirmed: true,
        };
        assert!(confirmed.permits_cleanup(&ModelError::Conflict("confirmed semantic refusal")));
        drop(confirmed);
        assert_eq!(Arc::strong_count(&confirmed_client), 1);
        let mut invalidation = lctx_model::domain::completion::Completion {
            remote: lctx_model::domain::completion::RemoteState::Unknown,
            ..Default::default()
        };
        invalidation.step(
            "session invalidation",
            Err(sdk_error(surrealdb::Error::internal(
                "lost invalidation acknowledgement".into(),
            ))),
        );
        let failure = lctx_model::domain::completion::complete(Ok(()), invalidation).unwrap_err();
        assert!(
            !failure.permits_storage_cleanup(),
            "an SDK cause cannot turn absent invalidation acknowledgement into confirmed finality"
        );
    }
    #[tokio::test]
    async fn logical_view_scope_refuses_canonical_reads_including_zero_demand_without_native_queries()
     {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap();
        let reader =
            NativeReader::for_views(Arc::new(Surreal::init()), vec![]).with_budget(&budget);
        assert!(reader.relation_bodies("packages", 0).is_err());
        assert!(reader.prepare_selection().await.is_err());
        assert!(reader.selected_membership(&[], &budget).await.is_err());
        let nonempty = NativeReader::for_views(
            Arc::new(Surreal::init()),
            vec![lctx_model::domain::ContentHash::of(b"zero-limit-view")],
        )
        .with_budget(&budget);
        assert!(nonempty.relation_bodies("packages", 0).is_err());
        assert!(
            nonempty.relation_bodies("not-a-model-relation", 0).is_err(),
            "zero demand still validates the relation schema"
        );
        nonempty.close().await.unwrap();
        assert_eq!(budget.reserved(), 0);
        let cancelled =
            reader.with_read_cancellation(Arc::new(std::sync::atomic::AtomicBool::new(true)));
        assert!(cancelled.relation_bodies("packages", 0).is_err());
        let unadmitted = NativeReader::for_views(Arc::new(Surreal::init()), vec![]);
        assert!(unadmitted.relation_bodies("packages", 0).is_err());
    }
    #[tokio::test]
    async fn owned_session_missing_invalidation_ack_retains_actual_client_and_unknown_outcome() {
        let client = Arc::new(Surreal::init());
        let weak = Arc::downgrade(&client);
        let owner = SessionFinalizationGuard::new(client.clone());
        let error = owner.close().await.unwrap_err();
        assert!(
            !error.permits_storage_cleanup(),
            "an SDK failure cannot establish acknowledged session closure"
        );
        drop(client);
        drop(owner);
        assert!(
            weak.upgrade().is_some(),
            "unknown finality retains the exact SDK owner"
        );
    }
    #[tokio::test]
    async fn completed_setup_private_transport_retains_pin_through_query_owner_loss() {
        let client = Arc::new(Surreal::init());
        let pin = crate::control::ReaderPin::test_detached(false);
        let setup = InstalledReaderSetup {
            views: vec![],
            pin: Some(pin.clone()),
            client: client.clone(),
            confirmed: false,
        };
        let reader = NativeReader::private_pinned(client.clone(), pin.clone());
        let lease = QueryLease::admit(
            reader.operations.clone(),
            client.clone(),
            reader.scope.clone(),
        )
        .unwrap();
        assert_eq!(reader.operations.lock().unwrap().active, 1);
        assert!(
            reader.close().await.is_err(),
            "an accepted setup query retains the pin-backed scope"
        );
        drop(setup);
        assert!(
            pin.test_uncertain(),
            "factory abandonment precedes any possible pin release"
        );
        drop(lease);
        assert_eq!(reader.operations.lock().unwrap().active, 0);
        assert!(
            !reader.close().await.unwrap_err().permits_storage_cleanup(),
            "lost query terminal remains unknown through close"
        );
        assert!(!pin.release().await.unwrap_err().permits_storage_cleanup());
    }
    #[tokio::test]
    async fn borrowed_recovery_pin_survives_private_reader_close_and_guard_close() {
        let pin = crate::control::ReaderPin::test_detached(false);
        let reader = NativeReader::private_pinned(Arc::new(Surreal::init()), pin.clone());
        let mut guard = reader.protect_terminal_close();
        guard.close(&reader).await.unwrap();
        drop(guard);
        pin.ensure_live().unwrap();
        assert!(!pin.test_uncertain());
        // Synthetic pins have no service attachment to release.
        pin.retain_unknown();
    }
    #[tokio::test]
    async fn reader_terminal_guard_retains_actual_pin_on_unwind_without_an_extra_scope_borrower() {
        use futures::FutureExt;
        let pin = crate::control::ReaderPin::test_detached(false);
        let mut reader = NativeReader::for_views(Arc::new(Surreal::init()), vec![]);
        Arc::get_mut(reader.scope.as_mut().unwrap()).unwrap()._pin = Some(pin.clone());
        let scope_borrowers = Arc::strong_count(reader.scope.as_ref().unwrap());
        let guard = reader.protect_terminal_close();
        assert_eq!(
            Arc::strong_count(reader.scope.as_ref().unwrap()),
            scope_borrowers
        );
        let panic = std::panic::AssertUnwindSafe(async move {
            let _guard = guard;
            panic!("read owner unwind before terminal close");
        })
        .catch_unwind()
        .await;
        assert!(panic.is_err());
        assert!(pin.test_uncertain());
        assert!(!reader.close().await.unwrap_err().permits_storage_cleanup());
        let pin = crate::control::ReaderPin::test_detached(true);
        Arc::get_mut(reader.scope.as_mut().unwrap()).unwrap()._pin = Some(pin.clone());
        let mut guard = reader.protect_terminal_close();
        guard.close(&reader).await.unwrap();
        drop(guard);
        assert!(
            !pin.test_uncertain(),
            "successful explicit close disarms protection"
        );
    }
    #[tokio::test]
    async fn cancelled_read_owner_fences_reader_loader_and_selected_native_admission() {
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let reader =
            NativeReader::private(Arc::new(Surreal::init())).with_read_cancellation(flag.clone());
        let error = reader
            .query_native::<Value>("RETURN 1", Variables::new())
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("delivery cancelled"),
            "must refuse before unconnected SDK client: {error}"
        );
        let loader =
            crate::Loader::for_views(reader.shared_client(), vec![]).with_read_cancellation(flag);
        assert!(loader.check_read_admission().is_err());
        let error = match loader
            .reader()
            .query_stream("RETURN 1", Variables::new(), 1)
        {
            Err(error) => error,
            Ok(_) => panic!("cancelled stream admitted"),
        };
        assert!(error.to_string().contains("delivery cancelled"));
    }
    #[test]
    fn typed_sdk_timeout_is_resource_refused_without_message_classification() {
        let error = surrealdb::Error::query(
            "opaque engine reason".into(),
            surrealdb::types::QueryError::TimedOut {
                duration: std::time::Duration::from_secs(10),
            },
        );
        assert!(matches!(
            sdk_error(error).primary(),
            Some(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::ResourceRefused
            ))
        ));
    }
    #[test]
    fn other_sdk_errors_preserve_typed_payload_even_with_timeout_wording() {
        for error in [
            surrealdb::Error::query("timeout without typed details".into(), None),
            surrealdb::Error::query("cancelled".into(), surrealdb::types::QueryError::Cancelled),
            surrealdb::Error::query(
                "conflict".into(),
                surrealdb::types::QueryError::TransactionConflict,
            ),
            surrealdb::Error::internal("exceeded the timeout: 10s".into()),
            surrealdb::Error::serialization("serialization failure".into(), None),
        ] {
            let expected = error.to_string();
            assert!(
                matches!(sdk_error(error).primary(), Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some() && cause.to_string()==expected)
            );
        }
    }
    fn row() -> StreamItem {
        let mut value = surrealdb::types::Object::new();
        value.insert("id", RecordId::new("entity", "row"));
        StreamItem::Row {
            statement: 0,
            value: Value::Object(value),
        }
    }
    fn end(result: surrealdb::Result<()>) -> StreamItem {
        StreamItem::StatementEnd {
            statement: 0,
            stats: Default::default(),
            result,
        }
    }
    fn failure() -> surrealdb::Error {
        surrealdb::Error::internal("injected late stream failure".into())
    }
    #[tokio::test]
    async fn borrowed_pending_row_and_eof_refuse_source_authority_lost_during_await() {
        for deliver_row in [true, false] {
            let budget = lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap();
            let pin = crate::control::ReaderPin::test_detached(false);
            let client = Arc::new(Surreal::init());
            let guard = Arc::new(BorrowedInstalledGuard {
                client: client.clone(),
                pins: vec![pin.clone()],
                context: (lctx_model::domain::ContentHash([1; 32]), 1),
                _charge: budget
                    .reserve("pending borrowed source guard", 1024)
                    .unwrap(),
            });
            let mut reader = NativeReader::for_views(client, vec![]);
            Arc::get_mut(reader.scope.as_mut().unwrap())
                .unwrap()
                .borrowed = Some(guard.clone());
            let (sender, receiver) = futures::channel::mpsc::unbounded();
            let mut rows = NativeRows::new(receiver, 1).unwrap();
            rows.scope = reader.scope.clone();
            let mut pending = Box::pin(rows.next());
            assert!(futures::poll!(pending.as_mut()).is_pending());
            pin.retain_unknown();
            if deliver_row {
                sender.unbounded_send(Ok(row())).unwrap();
            }
            sender.unbounded_send(Ok(end(Ok(())))).unwrap();
            drop(sender);
            let error = pending.await.unwrap_err();
            assert!(
                matches!(
                    error.primary(),
                    Some(ModelError::Conflict(
                        "native reader pin is released or uncertain"
                    ))
                ),
                "neither a queued row nor physical EOF restores lost source authority"
            );
            // Logical delivery refusal and physical drainage are separate outcomes.
            rows.drain_transport().await.unwrap();
            assert!(
                rows.next().await.is_err(),
                "drain cannot erase the original refusal"
            );
            assert!(pin.test_uncertain());
            assert!(
                !pin.release().await.unwrap_err().permits_storage_cleanup(),
                "confirmed local drainage does not settle the original source uncertainty"
            );
            reader.close().await.unwrap();
            drop(rows);
            drop(reader);
            drop(guard);
            drop(pin);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[tokio::test]
    async fn missing_terminal_and_late_physical_failure_forbid_pin_cleanup() {
        let mut missing = NativeRows::new(futures::stream::iter([Ok(row())]), 1).unwrap();
        assert!(missing.next().await.unwrap().is_some());
        let primary = missing.next().await.unwrap_err();
        assert!(!primary.permits_storage_cleanup());
        assert!(
            !missing
                .drain_transport()
                .await
                .unwrap_err()
                .permits_storage_cleanup()
        );
        let mut late =
            NativeRows::new(futures::stream::iter([Ok(end(Ok(()))), Err(failure())]), 1).unwrap();
        let primary = late.next().await.unwrap_err();
        assert!(
            matches!(primary.primary(), Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some())
        );
        assert!(!primary.permits_storage_cleanup());
        assert!(
            !late
                .drain_transport()
                .await
                .unwrap_err()
                .permits_storage_cleanup()
        );
        let mut semantic = NativeRows::new(
            futures::stream::iter([Ok(end(Err(surrealdb::Error::query(
                "confirmed statement failure".into(),
                None,
            ))))]),
            1,
        )
        .unwrap();
        assert!(semantic.next().await.unwrap_err().permits_storage_cleanup());
        semantic.drain_transport().await.unwrap();
    }
    #[tokio::test]
    async fn interrupted_transport_drain_retains_primary_and_both_late_errors() {
        let (release, gate) = tokio::sync::oneshot::channel::<()>();
        let initial = futures::stream::iter(vec![
            Ok(end(Err(surrealdb::Error::query(
                "primary statement failure".into(),
                None,
            )))),
            Err(surrealdb::Error::internal("late transport failure".into())),
        ]);
        let tail = futures::stream::once(async move {
            gate.await.unwrap();
            Ok(StreamItem::StatementEnd {
                statement: 1,
                stats: Default::default(),
                result: Err(surrealdb::Error::query(
                    "later statement failure".into(),
                    None,
                )),
            })
        });
        let mut rows = NativeRows::new(initial.chain(tail), 2).unwrap();
        let primary = rows.next().await.unwrap_err();
        assert!(
            matches!(primary.primary(),Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some() && cause.to_string().contains("primary statement failure"))
        );
        let original = rows.failure.as_ref().unwrap().clone();
        let mut interrupted = Box::pin(rows.drain_transport());
        assert!(futures::poll!(&mut interrupted).is_pending());
        drop(interrupted);
        assert_eq!(rows.drain_errors.len(), 1);
        assert!(Arc::ptr_eq(&original, rows.failure.as_ref().unwrap()));
        assert_eq!(
            rows.next().await.unwrap_err().to_string(),
            primary.to_string()
        );
        release.send(()).unwrap();
        let error = rows.drain_transport().await.unwrap_err();
        assert!(rows.exhausted);
        assert_eq!(rows.drain_errors.len(), 2);
        let ModelError::Completion(outcome) = error else {
            panic!("structured drain outcome");
        };
        assert!(outcome.primary.is_none());
        assert_eq!(outcome.completion.failures.len(), 2);
        for failure in outcome.completion.failures {
            assert!(
                matches!(failure.error.primary(),Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some())
            );
        }
        assert!(Arc::ptr_eq(&original, rows.failure.as_ref().unwrap()));
        assert!(rows.drain_transport().await.is_err());
    }
    #[tokio::test]
    async fn canonical_projection_decodes_one_valid_record_then_checks_completion() {
        let package = lctx_model::domain::input::Package {
            name: "projection".into(),
        };
        let mut node = surrealdb::types::Object::new();
        node.insert("node_kind", "entity");
        node.insert(
            "canonical",
            Bytes::from(serde_json::to_vec(&Entity::from(package.clone())).unwrap()),
        );
        let rows = NativeRows::new(
            futures::stream::iter(vec![
                Ok(StreamItem::Row {
                    statement: 0,
                    value: Value::Object(node),
                }),
                Ok(end(Ok(()))),
            ]),
            1,
        )
        .unwrap();
        let mut records = CanonicalRecords::<lctx_model::domain::input::Package> {
            rows,
            positions: vec![0],
            physical_kind: false,
            failed: false,
            marker: std::marker::PhantomData,
        };
        assert_eq!(records.next().await.unwrap(), Some(package));
        assert!(records.next().await.unwrap().is_none());
    }
    #[tokio::test]
    async fn malformed_canonical_stream_cannot_later_report_completion() {
        let mut node = surrealdb::types::Object::new();
        node.insert("node_kind", "entity");
        node.insert("canonical", Bytes::from(b"malformed".to_vec()));
        let rows = NativeRows::new(
            futures::stream::iter(vec![
                Ok(StreamItem::Row {
                    statement: 0,
                    value: Value::Object(node),
                }),
                Ok(end(Ok(()))),
            ]),
            1,
        )
        .unwrap();
        let mut records = CanonicalRecords::<lctx_model::domain::input::Package> {
            rows,
            positions: vec![0],
            physical_kind: false,
            failed: false,
            marker: std::marker::PhantomData,
        };
        assert!(matches!(
            records.next().await,
            Err(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::Corrupt
            ))
        ));
        assert!(records.next().await.is_err());
    }
    #[tokio::test]
    async fn private_stream_drop_keeps_client_through_outer_terminal_drain() {
        let client: Arc<Surreal<Client>> = Arc::new(Surreal::init());
        let weak = Arc::downgrade(&client);
        let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let complete = Arc::new(tokio::sync::Notify::new());
        let receipt = complete.clone();
        let retained = observed.clone();
        let stream = futures::stream::unfold(0, move |step| {
            let weak = weak.clone();
            let receipt = receipt.clone();
            let retained = retained.clone();
            async move {
                if step == 0 {
                    Some((Ok(end(Ok(()))), 1))
                } else {
                    retained.store(
                        weak.upgrade().is_some(),
                        std::sync::atomic::Ordering::Release,
                    );
                    receipt.notify_one();
                    None
                }
            }
        });
        let rows = NativeRows::new(stream, 1).unwrap().with_client(client);
        drop(rows);
        tokio::time::timeout(std::time::Duration::from_secs(5), complete.notified())
            .await
            .unwrap();
        assert!(
            observed.load(std::sync::atomic::Ordering::Acquire),
            "private scope absence cannot release the native session before outer EOF"
        );
    }
    #[tokio::test]
    async fn stream_rows_are_provisional_until_statement_and_outer_completion() {
        for items in [
            vec![Ok(row()), Ok(end(Err(failure())))],
            vec![Ok(row()), Ok(end(Ok(()))), Err(failure())],
            vec![Ok(row())],
        ] {
            let mut rows = NativeRows::new(futures::stream::iter(items), 1).unwrap();
            assert!(rows.next().await.unwrap().is_some());
            assert!(rows.next().await.is_err());
            assert!(rows.next().await.is_err());
        }
        let mut actual = NativeRows::new(
            futures::stream::iter(vec![Ok(row()), Ok(end(Ok(()))), Err(failure())]),
            1,
        )
        .unwrap();
        let StreamItem::Row { value, .. } = row() else {
            unreachable!()
        };
        let mut expected = crate::ordered_rows::SortedRows::new().unwrap();
        expected.push(value).unwrap();
        assert!(
            expected
                .finish()
                .unwrap()
                .reconcile(&mut actual)
                .await
                .is_err(),
            "full expected row equality cannot hide an outer completion error"
        );
    }
}
