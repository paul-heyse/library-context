use futures::{FutureExt, StreamExt};
use lctx_model::domain::{
    Key, KeySink, ModelError, Record,
    graph::{Assertion, Entity, Target},
    serving::SnapshotHandle,
};
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
struct ReaderScope {
    publication: Option<SnapshotHandle>,
    exact: Option<Arc<ExactViewSelection>>,
    originals: Option<Vec<lctx_model::domain::graph::Original>>,
    views: Vec<lctx_model::domain::ContentHash>,
    _pin: Option<Arc<crate::control::ReaderPin>>,
    release_pin_on_close: bool,
    borrowed: Option<Arc<BorrowedInstalledGuard>>,
    selection: std::sync::Mutex<
        Option<(
            lctx_model::domain::resources::ResourceBudget,
            Option<lctx_model::domain::ContentHash>,
            futures::future::Shared<
                futures::future::BoxFuture<
                    'static,
                    Result<crate::selection::SelectedPayloads, Arc<ModelError>>,
                >,
            >,
        )>,
    >,
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
    owns_session: bool,
    session_closed: bool,
    active: usize,
    drains: Vec<
        futures::future::Shared<futures::future::BoxFuture<'static, Result<(), Arc<ModelError>>>>,
    >,
    closed: bool,
    // Abandoned delivery has a reader-owned terminal receipt, rather than a log-only result.
    abandoned: Vec<Result<(), Arc<ModelError>>>,
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
        {
            if let Some(scope) = &self.scope {
                scope.retain_unknown();
            }
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
fn native_query_payload_bytes(value: &Value) -> usize {
    use std::ops::Bound;
    use surrealdb::types::{Geometry, RecordIdKey};
    fn sum(values: impl Iterator<Item = usize>) -> usize {
        values.fold(0, usize::saturating_add)
    }
    fn bound(value: &Bound<Value>) -> usize {
        match value {
            Bound::Included(value) | Bound::Excluded(value) => native_query_payload_bytes(value),
            Bound::Unbounded => 0,
        }
    }
    fn key(value: &RecordIdKey) -> usize {
        std::mem::size_of::<RecordIdKey>().saturating_add(match value {
            RecordIdKey::Number(_) | RecordIdKey::Uuid(_) => 0,
            RecordIdKey::String(value) => value.capacity(),
            RecordIdKey::Array(values) => sum(values.iter().map(native_query_payload_bytes))
                .saturating_add(
                    (values.capacity() - values.len()).saturating_mul(std::mem::size_of::<Value>()),
                ),
            RecordIdKey::Object(values) => sum(values.iter().map(|(name, value)| {
                name.capacity()
                    .saturating_add(std::mem::size_of::<String>())
                    .saturating_add(native_query_payload_bytes(value))
            })),
            RecordIdKey::Range(range) => {
                std::mem::size_of_val(range.as_ref()).saturating_add(sum([
                    &range.start,
                    &range.end,
                ]
                .into_iter()
                .map(|value| match value {
                    Bound::Included(value) | Bound::Excluded(value) => key(value),
                    Bound::Unbounded => 0,
                })))
            }
        })
    }
    fn slots<T>(values: &Vec<T>) -> usize {
        values.capacity().saturating_mul(std::mem::size_of::<T>())
    }
    // Geometry containers expose their storage through the pinned public geo fields.
    fn geometry(value: &Geometry) -> usize {
        match value {
            Geometry::Point(_) => 0,
            Geometry::Line(value) => slots(&value.0),
            Geometry::Polygon(value) => slots(&value.exterior().0)
                .saturating_add(std::mem::size_of_val(value.interiors()))
                .saturating_add(sum(value.interiors().iter().map(|line| slots(&line.0)))),
            Geometry::MultiPoint(value) => slots(&value.0),
            Geometry::MultiLine(value) => {
                slots(&value.0).saturating_add(sum(value.0.iter().map(|line| slots(&line.0))))
            }
            Geometry::MultiPolygon(value) => {
                slots(&value.0).saturating_add(sum(value.0.iter().map(|polygon| {
                    slots(&polygon.exterior().0)
                        .saturating_add(std::mem::size_of_val(polygon.interiors()))
                        .saturating_add(sum(polygon.interiors().iter().map(|line| slots(&line.0))))
                })))
            }
            Geometry::Collection(values) => {
                slots(values).saturating_add(sum(values.iter().map(geometry)))
            }
        }
    }
    std::mem::size_of::<Value>().saturating_add(match value {
        Value::None
        | Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Duration(_)
        | Value::Datetime(_)
        | Value::Uuid(_) => 0,
        Value::String(value) => value.capacity(),
        Value::Bytes(value) => value.len(),
        Value::Array(values) => sum(values.iter().map(native_query_payload_bytes)).saturating_add(
            (values.capacity() - values.len()).saturating_mul(std::mem::size_of::<Value>()),
        ),
        Value::Object(values) => sum(values.iter().map(|(name, value)| {
            name.capacity()
                .saturating_add(std::mem::size_of::<String>())
                .saturating_add(native_query_payload_bytes(value))
        })),
        Value::Set(values) => sum(values.iter().map(native_query_payload_bytes)),
        Value::Table(value) => value.capacity(),
        Value::RecordId(value) => value.table.capacity().saturating_add(key(&value.key)),
        Value::File(value) => value.bucket.capacity().saturating_add(value.key.capacity()),
        Value::Range(value) => std::mem::size_of_val(value.as_ref())
            .saturating_add(bound(&value.start))
            .saturating_add(bound(&value.end)),
        Value::Regex(value) => value.as_str().len(),
        Value::Geometry(value) => geometry(value),
    })
}
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
    stream: futures::stream::BoxStream<'static, surrealdb::Result<surrealdb::method::BatchItem>>,
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
            BatchItem::Rows { values, .. } => {
                // Charge the delivered batch before admitting it to retained accumulation.
                // This does not charge SDK/tonic allocations before delivery.
                let bytes = values
                    .iter()
                    .map(native_query_payload_bytes)
                    .fold(0usize, usize::saturating_add);
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
                rows.extend(values);
                self.payload_bytes = self.payload_bytes.saturating_add(bytes);
            }
            BatchItem::Value { value, .. } => {
                let bytes = native_query_payload_bytes(&value);
                self.charge.try_resize(bytes)?;
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
    async fn drain(&mut self, flag: Option<Arc<std::sync::atomic::AtomicBool>>) {
        let mut cancelled = false;
        while let Some(item) = self.stream.next().await {
            if !cancelled {
                if let Err(error) = crate::prepared::check_read_cancellation(flag.as_ref()) {
                    self.fail(error);
                    cancelled = true;
                }
            }
            if let Err(error) = self.ingest(item) {
                self.fail(error);
            }
        }
        if !cancelled {
            if let Err(error) = crate::prepared::check_read_cancellation(flag.as_ref()) {
                self.fail(error);
            }
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
            {
                if let Err(error) = pin.release().await {
                    return lctx_model::domain::completion::complete(Err(error), completion);
                }
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
            let invalidated = self.client.invalidate().await.map_err(sdk_error);
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
                if let Some((owner, namespace, pending)) = selection.as_ref() {
                    if !owner.shares_pool(&budget) || *namespace != derivation {
                        return Err(ModelError::Conflict(
                            "foreign native selection budget or derivation",
                        ));
                    }
                    pending.clone()
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
                    *selection = Some((budget, derivation, pending.clone()));
                    pending
                }
            };
            let selected = pending.await.map_err(ModelError::SharedCause)?;
            return_owner.ensure_scope_live()?;
            Ok(Some(
                selected
                    .with_reader_owner(return_owner)
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
        let mut rows = NativeRows::owned(move |sender| async move {
            let selection = selection
                .await?
                .ok_or(ModelError::Conflict("selected reader scope"))?;
            if sender.is_closed() {
                return Ok(());
            }
            let mut rows = selection.rows_with_budget(
                &table,
                &predicate,
                bindings,
                preparation,
                &order,
                limit,
                &budget,
            )?;
            let result = async { loop {
                let row = tokio::select! { biased; _ = sender.closed() => { rows.cancel_delivery(); break; }, row = rows.next() => row? };
                let Some(row) = row else { break; };
                if sender.send(row).await.is_err() { rows.cancel_delivery(); break; }
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
            NativeRows::owned(move |sender| async move {
                let selected = preparation
                    .await?
                    .ok_or(ModelError::Conflict("candidate exact selection missing"))?;
                let mut rows = selected.candidate_rows(candidates, &table, &order, &budget)?;
                let result = async {
                    while let Some(row) = rows.next().await? {
                        if sender.send(row).await.is_err() {
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
            if let Some((_, _, pending)) = pending {
                pending.await.map_err(ModelError::SharedCause)?;
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
            NativeRows::owned(move |sender| async move {
                let selected = preparation
                    .await?
                    .ok_or(ModelError::Conflict("exact relation selection missing"))?;
                if limit == 0 {
                    return Ok(());
                }
                let mut rows = selected.relation_candidates(&relation, &table, &budget)?;
                let result = async {
                    while let Some(row) = rows.next().await? {
                        if sender.send(row).await.is_err() {
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
        let mut rows = NativeRows::owned(move |sender| async move {
            let result = async {
                let mut emitted = 0usize;
                while let Some(row) = input.next().await? {
                    if sender.is_closed() {
                        input.cancel_delivery();
                        break;
                    }
                    if emitted < limit {
                        if sender.send(row).await.is_err() {
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
            .stream_cancellable(&self.client, self.cancellation.as_ref())
            .map(|mut rows| {
                rows.scope = self.scope.clone();
                rows
            })
    }
    pub async fn query_prepared<T: Serialize + DeserializeOwned + 'static>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<T, ModelError> {
        let mut value = self.checked_native_value(query).await?;
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
        let mut value = self.checked_native_value(query).await?;
        T::from_value(value.take_value()).map_err(ModelError::codec)
    }
    /// Finite owner-generated SELECT/RETURN reads. This API does not register LIVE
    /// subscriptions and is not a read-only authorization fence for arbitrary SQL.
    async fn checked_native_value(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<ChargedNativeValue, ModelError> {
        self.ensure_scope_live()?;
        if query.result_positions().len() != 1 {
            return Err(ModelError::Schema("native single result demand"));
        }
        let selected = query.result_positions()[0];
        let statements = query.expected_terminals();
        let budget = self.scratch_budget()?;
        let charge = budget.reserve(
            "native exact query accumulation",
            std::mem::size_of::<Value>(),
        )?;
        let (sql, bindings) = query.with_bindings(self.view_bindings()).into_request();
        self.ensure_scope_live()?;
        let stream = self
            .client
            .query(sql)
            .bind(bindings)
            .stream_batches()
            .map_err(sdk_error)?;
        let lease = QueryLease::admit(
            self.operations.clone(),
            self.client.clone(),
            self.scope.clone(),
        )?;
        let mut owner = NativeValueOwner::new(stream, statements, selected, budget, charge, lease);
        let value = owner.collect(self.cancellation.clone()).await?;
        self.ensure_scope_live()?;
        Ok(value)
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
                operations: Default::default(),
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
    /// Own a dedicated authenticated session through setup, query drain and close.
    pub async fn connect(
        endpoint: &str,
        credentials: &Credentials,
        handle: SnapshotHandle,
    ) -> Result<Self, ModelError> {
        let client = connect(
            endpoint,
            credentials,
            handle.database.namespace.as_str(),
            handle.database.database.as_str(),
        )
        .await?;
        let mut setup = InstalledReaderSetup {
            views: vec![],
            pin: None,
            client: client.clone(),
            confirmed: false,
        };
        match Self::open(client.clone(), handle).await {
            Ok(reader) => {
                reader
                    .operations
                    .lock()
                    .map_err(|_| ModelError::Conflict("publication reader session owner"))?
                    .owns_session = true;
                setup.confirmed = true;
                Ok(reader)
            }
            Err(error) => {
                if !setup.permits_cleanup(&error) {
                    return Err(error);
                }
                let mut terminal = lctx_model::domain::completion::Completion::default();
                let invalidated = client.invalidate().await.map_err(sdk_error);
                setup.confirmed = invalidated.is_ok();
                if invalidated.is_err() {
                    terminal.remote = lctx_model::domain::completion::RemoteState::Unknown;
                }
                terminal.step(
                    "failed publication reader session invalidation",
                    invalidated,
                );
                lctx_model::domain::completion::complete(Err(error), terminal)
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
        let client = Arc::new(client.as_ref().clone());
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
                let invalidated = client.invalidate().await.map_err(sdk_error);
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
            crate::installed_view::protect_receiving_views(views, &pin, &reader, budget).await?;
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
                    let invalidated = client.invalidate().await.map_err(sdk_error);
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
        let client = Arc::new(client.as_ref().clone());
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
                let invalidated = client.invalidate().await.map_err(sdk_error);
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
            if actual.incarnation()!=completed.incarnation() || actual.row()!=&data.value(true)? {
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
                    if row.row()!=&reference.value(identity)? {return Err(ModelError::Conflict("completed reader exact selector full bytes"));}
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
                    if copy_row.row()!=&copy.value(identity)? {return Err(ModelError::Conflict("completed reader contributor exact compound copy"));}
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
                    let invalidated = client.invalidate().await.map_err(sdk_error);
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

pub struct NativeRows {
    stream: futures::stream::BoxStream<'static, surrealdb::Result<surrealdb::method::StreamItem>>,
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
    owned_result: Option<
        futures::future::Shared<futures::future::BoxFuture<'static, Result<(), Arc<ModelError>>>>,
    >,
}
impl NativeRows {
    /// The driver owns all native preparation through its checked terminal. Receiver loss
    /// stops delivery; admitted native reads still drain before the synthetic terminal.
    pub fn owned<F, Fut>(driver: F) -> Result<Self, ModelError>
    where
        F: FnOnce(tokio::sync::mpsc::Sender<Value>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), ModelError>> + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (terminal_sender, terminal_receiver) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let result = driver(sender).await;
            let _ = terminal_sender.send(result);
        });
        let rows = futures::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.map(|value| {
                (
                    Ok(surrealdb::method::StreamItem::Row {
                        statement: 0,
                        value,
                    }),
                    receiver,
                )
            })
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
        let terminal_result = result.clone();
        let terminal = futures::stream::once(async move {
            let _ = terminal_result.await;
            Ok(surrealdb::method::StreamItem::StatementEnd {
                statement: 0,
                stats: Default::default(),
                result: Ok(()),
            })
        });
        let mut rows = Self::new(rows.chain(terminal), 1)?;
        rows.owned_result = Some(result);
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
            stream: stream.boxed(),
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
        })
    }
    /// Close synthetic delivery while keeping its independent driver terminal alive.
    /// Raw transport streams remain owned until checked drainage.
    pub fn cancel_delivery(&mut self) {
        if self.owned_result.is_some() {
            self.stream = futures::stream::empty().boxed();
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
            while let Some(item) = self.stream.next().await {
                match item {
                    Err(error) => self
                        .drain_errors
                        .push(Arc::new(unconfirmed_stream(sdk_error(error)))),
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
                            self.drain_errors.push(Arc::new(sdk_error(error)));
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
        if self.ended != self.statements {
            if let Some(scope) = &self.scope {
                scope.retain_unknown();
            }
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
        if completion.remote == lctx_model::domain::completion::RemoteState::Unknown {
            if let Some(scope) = &self.scope {
                scope.retain_unknown();
            }
        }
        self.scope.take();
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
    pub async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        if let Some(error) = &self.failure {
            return Err(ModelError::SharedCause(error.clone()));
        }
        // Physical EOF removes the stream scope, so retain this small source guard
        // across the await and through delivery of either a row or complete EOF.
        let borrowed = self.scope.as_ref().and_then(|scope| scope.borrowed.clone());
        if let Some(borrowed) = &borrowed {
            if let Err(error) = borrowed.ensure_live() {
                return Err(self.remember_failure(error));
            }
        }
        match self.read_next().await {
            Ok(row) => {
                if let Some(borrowed) = &borrowed {
                    if let Err(error) = borrowed.ensure_live() {
                        return Err(self.remember_failure(error));
                    }
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
        while let Some(item) = self.stream.next().await {
            match item.map_err(|error| unconfirmed_stream(sdk_error(error)))? {
                surrealdb::method::StreamItem::Row { statement, value } => {
                    if statement != self.ended || statement >= self.statements {
                        return Err(unconfirmed_stream(ModelError::Schema(
                            "native stream row order",
                        )));
                    }
                    // The SDK queue is row bounded, not byte bounded. Refuse an oversized source
                    // value before decoding or expansion; this is not a server RSS guarantee.
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
                    result.map_err(sdk_error)?;
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
                if !error.permits_storage_cleanup() {
                    if let Some(scope) = &self.scope {
                        scope.retain_unknown();
                    }
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
            if self.ended != self.statements
                || self
                    .failure
                    .as_ref()
                    .is_some_and(|error| !error.permits_storage_cleanup())
                || self
                    .drain_errors
                    .iter()
                    .any(|error| !error.permits_storage_cleanup())
            {
                if let Some(scope) = &scope {
                    scope.retain_unknown();
                }
            }
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            if let Some(scope) = &scope {
                scope.retain_unknown();
            }
            return;
        };
        self.cancel_delivery();
        let mut drain = NativeRows {
            stream: std::mem::replace(&mut self.stream, futures::stream::empty().boxed()),
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
        };
        runtime.spawn(async move {
            let result = drain.drain_transport().await;
            if result.is_err() || drain.failure.is_some() {
                tracing::error!(primary = ?drain.failure, completion = ?result, "cancelled native read terminal outcome");
            }
        });
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
    pub(crate) fn transfer(self) {
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
    let client = authenticated(endpoint, credentials, Some((namespace, database))).await?;
    let session = SessionFinalizationGuard::new(client.clone());
    let selected = client
        .use_ns(namespace)
        .use_db(database)
        .await
        .map_err(sdk_error);
    if let Err(error) = selected {
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
    session.transfer();
    Ok(client)
}
/// Authenticate without selecting or implicitly creating a database. Private compiler creation
/// can therefore establish STRICT before selecting its first storage session.
pub async fn authenticated(
    endpoint: &str,
    credentials: &Credentials,
    scope: Option<(&str, &str)>,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    let client = Arc::new(
        Surreal::new::<Grpc>(
            endpoint
                .strip_prefix("grpc://")
                .ok_or_else(|| ModelError::Invalid("native gRPC endpoint required".into()))?,
        )
        .await
        .map_err(sdk_error)?,
    );
    let session = SessionFinalizationGuard::new(client.clone());
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
    if let Err(error) = signed {
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
    session.transfer();
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
        let mut invalidation = lctx_model::domain::completion::Completion::default();
        invalidation.remote = lctx_model::domain::completion::RemoteState::Unknown;
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
