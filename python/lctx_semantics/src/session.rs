//! Read-only domain session. Retained preparation and owned asynchronous native
//! execution share one Rust admission lifetime; Python never owns native finality.
use lctx_model::domain::ModelError;
use lctx_native::{NativeReader, config::ViewerConfig, control::RetryCancellation};
use lctx_serving::{NativeService, TransportRequest};
use pyo3::prelude::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

struct State {
    service: Option<Arc<NativeService>>,
    active: BTreeMap<u64, RetryCancellation>,
    deliveries: BTreeMap<u64, Delivery>,
    sequence: u64,
    closing: bool,
    closed: Option<Result<(), Arc<ModelError>>>,
}
struct Shared {
    state: Arc<Mutex<State>>,
    drained: tokio::sync::Notify,
    reader: NativeReader,
    finalization: Mutex<Option<lctx_native::reader::ReadFinalizationGuard>>,
    signal: lctx_native::signal::SignalRegistration,
    factory: Arc<lctx_native::control::RecoveryFactoryOwner>,
}
#[pyclass(module = "lctx_semantics", frozen)]
pub struct NativeSession {
    shared: Arc<Shared>,
}
/// An exception retains this concrete Rust recipient when native setup cannot
/// settle. Calling resume explicitly admits a separately signalled recovery turn.
#[pyclass(module = "lctx_semantics", frozen)]
pub struct NativeRecoveryOwner {
    factory: Arc<lctx_native::control::RecoveryFactoryOwner>,
}
#[pymethods]
impl NativeRecoveryOwner {
    fn pending_count(&self) -> PyResult<usize> {
        self.factory.pending_count().map_err(crate::model_error)
    }
    fn resume(&self, py: Python<'_>) -> PyResult<()> {
        let factory = self.factory.clone();
        py.detach(move || {
            let runtime = pyo3_async_runtimes::tokio::get_runtime();
            let cancellation = RetryCancellation::default();
            let stop = cancellation.clone();
            let signal = {
                let _entered = runtime.enter();
                lctx_native::signal::SignalRegistration::with_receipt(
                    "Python native recovery resumption",
                    move || stop.cancel(),
                    factory.binding().clone(),
                )
                .map_err(crate::model_error)?
            };
            let result = runtime.block_on(factory.drain(&cancellation));
            let mut completion = lctx_model::domain::completion::Completion::default();
            if factory.pending_count().map_err(crate::model_error)? != 0 {
                completion.local = lctx_model::domain::completion::LocalState::Outstanding;
            }
            if result
                .as_ref()
                .is_err_and(|error| !error.permits_storage_cleanup())
            {
                completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
            }
            completion.step(
                "Python native recovery owner receipt",
                runtime.block_on(signal.complete(&completion)),
            );
            lctx_model::domain::completion::complete(result, completion)
                .map_err(|error| recovery_error(factory, error))
        })
    }
}
fn recovery_error(
    factory: Arc<lctx_native::control::RecoveryFactoryOwner>,
    error: ModelError,
) -> PyErr {
    let error = crate::model_error(error);
    Python::attach(|py| {
        let recovery = match Py::new(py, NativeRecoveryOwner { factory }) {
            Ok(owner) => owner,
            Err(error) => return error,
        };
        match error.value(py).setattr("recovery", recovery) {
            Ok(()) => error,
            Err(error) => error,
        }
    })
}
/// One accepted request owns preparation bytes before any embedding await. Its
/// Active moves into the Rust task; dropping a Python future cannot release it.
#[pyclass(module = "lctx_semantics", frozen)]
pub struct NativeRequest {
    shared: Arc<Shared>,
    active: Mutex<Option<Active>>,
    cancellation: RetryCancellation,
    wire: Mutex<Option<(String, String)>>,
}
#[pymethods]
impl NativeSession {
    #[new]
    fn new(py: Python<'_>, serving_config: String) -> PyResult<Self> {
        py.detach(move || {
            let config = ViewerConfig::read(std::path::Path::new(&serving_config))
                .map_err(crate::model_error)?;
            let selected = config.selected().map_err(crate::model_error)?;
            let limits = config.serving_limits.clone().unwrap_or_default();
            limits.validate().map_err(crate::model_error)?;
            let budget = lctx_model::domain::resources::ResourceBudget::fixed(
                limits
                    .shared_bytes
                    .try_into()
                    .map_err(|error| crate::model_error(ModelError::codec(error)))?,
            )
            .map_err(crate::model_error)?;
            let factory = lctx_native::control::RecoveryFactoryOwner::new(
                config
                    .receipt_binding(selected.service_generation)
                    .map_err(crate::model_error)?,
                budget.clone(),
            )
            .map_err(crate::model_error)?;
            // The bridge owns this process runtime; requests and close tasks can
            // finish even after their Python delivery future has disappeared.
            let runtime = pyo3_async_runtimes::tokio::get_runtime();
            let state = Arc::new(Mutex::new(State {
                service: None,
                active: BTreeMap::new(),
                deliveries: BTreeMap::new(),
                sequence: 0,
                closing: false,
                closed: None,
            }));
            let cancellation = RetryCancellation::default();
            let (closing, retry) = (state.clone(), cancellation.clone());
            let signal = {
                let _entered = runtime.enter();
                lctx_native::signal::SignalRegistration::new("Python native session", move || {
                    retry.cancel();
                    if let Ok(mut state) = closing.lock() {
                        state.closing = true;
                        for token in state.active.values() {
                            token.cancel();
                        }
                    }
                })
                .map_err(crate::model_error)?
            };
            if cancellation.stopped() {
                runtime
                    .block_on(signal.complete(&Default::default()))
                    .map_err(crate::model_error)?;
                return Err(crate::unavailable());
            }
            let opened = runtime.block_on(lctx_native::control::with_retry_cancellation(
                cancellation.clone(),
                NativeReader::connect(
                    &config.endpoint,
                    &config.credentials(),
                    selected,
                    &budget,
                    &factory,
                ),
            ));
            let reader = match opened {
                Ok(reader) => reader,
                Err(error) => {
                    let error = Arc::new(error);
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step("native session setup", Err(ModelError::SharedCause(error)));
                    completion.step(
                        "native session failed factory drainage",
                        runtime.block_on(factory.drain(&cancellation)),
                    );
                    let observation = runtime.block_on(signal.complete(&completion));
                    completion.step("native signal receipt", observation);
                    let failure =
                        lctx_model::domain::completion::complete::<()>(Ok(()), completion)
                            .unwrap_err();
                    return Err(recovery_error(factory, failure));
                }
            };
            let mut finalization = reader.protect_terminal_close();
            let service = match NativeService::new(reader.clone(), limits) {
                Ok(service) => Arc::new(service),
                Err(error) => {
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step(
                        "native session setup reader close",
                        runtime.block_on(finalization.close(&reader)),
                    );
                    let observation = runtime.block_on(signal.complete(&completion));
                    completion.step("native signal receipt", observation);
                    let failure =
                        lctx_model::domain::completion::complete::<()>(Err(error), completion)
                            .unwrap_err();
                    return Err(recovery_error(factory, failure));
                }
            };
            state.lock().map_err(|_| crate::unavailable())?.service = Some(service);
            Ok(Self {
                shared: Arc::new(Shared {
                    state,
                    drained: tokio::sync::Notify::new(),
                    reader,
                    signal,
                    factory,
                    finalization: Mutex::new(Some(finalization)),
                }),
            })
        })
    }
    fn admit(&self, tool: String, request_json: String) -> PyResult<NativeRequest> {
        let mut state = self.shared.state.lock().map_err(|_| crate::unavailable())?;
        if state.closing {
            return Err(crate::unavailable());
        }
        let service = state
            .service
            .as_ref()
            .ok_or_else(crate::unavailable)?
            .clone();
        let bytes = size_of::<NativeRequest>()
            .saturating_add(size_of::<Active>())
            .saturating_add(size_of::<(u64, RetryCancellation)>())
            .saturating_add(size_of::<(u64, Delivery)>())
            .saturating_add(size_of::<DeliveryCallback>())
            .saturating_add(
                size_of::<ModelError>() + size_of::<lctx_model::domain::serving::WireError>(),
            )
            .saturating_add(tool.capacity())
            .saturating_add(request_json.capacity());
        let transport = service.admit_transport(bytes).map_err(crate::model_error)?;
        let cancellation = RetryCancellation::default();
        let identity = state.sequence;
        state.sequence = state
            .sequence
            .checked_add(1)
            .ok_or_else(crate::unavailable)?;
        state.active.insert(identity, cancellation.clone());
        Ok(NativeRequest {
            shared: self.shared.clone(),
            active: Mutex::new(Some(Active {
                shared: self.shared.clone(),
                service: Some(service),
                transport: Some(transport),
                executing: false,
                completed: false,
                delivery: false,
                identity,
            })),
            cancellation,
            wire: Mutex::new(Some((tool, request_json))),
        })
    }
    #[pyo3(signature=(tool,request_json,query_vector_json=None,remaining_deadline_ms=None))]
    fn execute(
        &self,
        py: Python<'_>,
        tool: String,
        request_json: String,
        query_vector_json: Option<String>,
        remaining_deadline_ms: Option<u64>,
    ) -> PyResult<Py<pyo3::types::PyString>> {
        let request = self.admit(tool, request_json)?;
        request.retain(query_vector_json.as_ref().map_or(0, String::capacity))?;
        let (active, tool, raw) = request.take()?;
        let cancellation = request.cancellation.clone();
        py.detach(move || {
            pyo3_async_runtimes::tokio::get_runtime().block_on(execute_owned(
                active,
                tool,
                raw,
                query_vector_json,
                remaining_deadline_ms,
                cancellation,
            ))
        })
    }
    #[pyo3(signature=(tool,request_json,query_vector_json=None,remaining_deadline_ms=None))]
    fn execute_async<'py>(
        &self,
        py: Python<'py>,
        tool: String,
        request_json: String,
        query_vector_json: Option<String>,
        remaining_deadline_ms: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.admit(tool, request_json)?
            .execute_async(py, query_vector_json, remaining_deadline_ms)
    }
    fn handle_json(&self) -> PyResult<String> {
        serde_json::to_string(self.shared.reader.handle()).map_err(|_| crate::unavailable())
    }
    fn stop_accepting(&self) -> PyResult<()> {
        self.shared
            .state
            .lock()
            .map_err(|_| crate::unavailable())?
            .closing = true;
        Ok(())
    }
    /// Drain all accepted preparation and actual Rust execution, then release the
    /// sole reader pin before invalidating its native client.
    fn close(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| pyo3_async_runtimes::tokio::get_runtime().block_on(self.shared.close()))
            .map_err(|error| {
                recovery_error(self.shared.factory.clone(), ModelError::SharedCause(error))
            })
    }
    fn close_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.stop_accepting()?;
        let shared = self.shared.clone();
        let factory = shared.factory.clone();
        let owner =
            pyo3_async_runtimes::tokio::get_runtime().spawn(async move { shared.close().await });
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            owner
                .await
                .map_err(|error| {
                    recovery_error(factory.clone(), ModelError::Cause(Box::new(error)))
                })?
                .map_err(|error| recovery_error(factory, ModelError::SharedCause(error)))
        })
    }
}
#[pymethods]
impl NativeRequest {
    fn retain(&self, bytes: usize) -> PyResult<()> {
        let active = self.active.lock().map_err(|_| crate::unavailable())?;
        active
            .as_ref()
            .ok_or_else(crate::unavailable)?
            .transport
            .as_ref()
            .ok_or_else(crate::unavailable)?
            .retain(bytes)
            .map_err(crate::model_error)
    }
    fn cancel(&self) {
        self.cancellation.cancel();
    }
    fn check(&self) -> PyResult<()> {
        if self.cancellation.stopped() {
            return Err(crate::unavailable());
        }
        Ok(())
    }
    fn release(&self) -> PyResult<()> {
        self.wire.lock().map_err(|_| crate::unavailable())?.take();
        self.active.lock().map_err(|_| crate::unavailable())?.take();
        Ok(())
    }
    #[pyo3(signature=(query_vector_json=None,remaining_deadline_ms=None))]
    fn execute_async<'py>(
        &self,
        py: Python<'py>,
        query_vector_json: Option<String>,
        remaining_deadline_ms: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.check()?;
        self.retain(query_vector_json.as_ref().map_or(0, String::capacity))?;
        let (active, tool, raw) = self.take()?;
        let identity = active.identity;
        self.shared
            .state
            .lock()
            .map_err(|_| crate::unavailable())?
            .deliveries
            .insert(identity, Delivery::default());
        let mut active = active;
        active.delivery = true;
        let cancellation = self.cancellation.clone();
        // Register the actual execution owner before constructing delivery. The
        // bridge may be cancelled, but close still waits for this Active guard.
        let owner = pyo3_async_runtimes::tokio::get_runtime().spawn(execute_owned(
            active,
            tool,
            raw,
            query_vector_json,
            remaining_deadline_ms,
            cancellation,
        ));
        let future = match pyo3_async_runtimes::tokio::future_into_py(py, async move {
            owner.await.map_err(|_| crate::unavailable())?
        }) {
            Ok(future) => future,
            Err(error) => {
                self.shared.acknowledge(identity, DeliveryState::Abandoned);
                return Err(error);
            }
        };
        let callback = Py::new(
            py,
            DeliveryCallback {
                shared: self.shared.clone(),
                identity,
                called: std::sync::atomic::AtomicBool::new(false),
            },
        )?;
        future.call_method1("add_done_callback", (callback,))?;
        Ok(future)
    }
}
impl NativeRequest {
    fn take(&self) -> PyResult<(Active, String, String)> {
        if self
            .shared
            .state
            .lock()
            .map_err(|_| crate::unavailable())?
            .closing
        {
            return Err(crate::unavailable());
        }
        let mut active = self.active.lock().map_err(|_| crate::unavailable())?;
        let mut wire = self.wire.lock().map_err(|_| crate::unavailable())?;
        if active.is_none() || wire.is_none() {
            return Err(crate::unavailable());
        }
        let (tool, raw) = wire.take().ok_or_else(crate::unavailable)?;
        Ok((active.take().ok_or_else(crate::unavailable)?, tool, raw))
    }
}
impl Drop for NativeRequest {
    fn drop(&mut self) {
        // Free retained input bytes before releasing their admission/charge.
        self.wire
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        self.active
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
    }
}
async fn execute_owned(
    mut active: Active,
    tool: String,
    raw: String,
    vector_json: Option<String>,
    remaining: Option<u64>,
    cancellation: RetryCancellation,
) -> PyResult<Py<pyo3::types::PyString>> {
    active.executing = true;
    let outcome = {
        let execution = async {
            if cancellation.stopped() {
                return Err(TaskFailure::unavailable());
            }
            let service = active
                .service
                .as_ref()
                .ok_or_else(TaskFailure::unavailable)?;
            let unavailable = vector_json.as_deref() == Some("unavailable");
            let vector = if unavailable {
                None
            } else {
                vector_json
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()
                    .map_err(|error| {
                        let retained = size_of::<serde_json::Error>() + error.to_string().len();
                        TaskFailure {
                            public: crate::public_error(
                                lctx_model::domain::serving::PublicFailure::new(
                                    lctx_model::domain::serving::FailureKind::Incompatible,
                                ),
                            ),
                            cause: ModelError::Cause(Box::new(error)),
                            retained,
                        }
                    })?
            };
            let transport = active
                .transport
                .as_ref()
                .ok_or_else(TaskFailure::unavailable)?;
            let encoded = service
                .execute_admitted_encoded_for(
                    transport,
                    cancellation,
                    lctx_serving::ToolInput {
                        tool: &tool,
                        raw: &raw,
                    },
                    vector,
                    unavailable,
                    remaining.unwrap_or(service.request_deadline_ms()),
                )
                .await
                .map_err(TaskFailure::wire)?;
            // Preserve the final-byte reservation through Python's transport copy.
            let result =
                Python::attach(|py| pyo3::types::PyString::new(py, encoded.as_str()).unbind());
            drop(encoded);
            Ok(result)
        };
        tokio::pin!(execution);
        // Preserve the original panic payload before Active publishes drainage. A
        // cancelled Python bridge owns neither this polling future nor its result.
        std::future::poll_fn(|cx| {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                execution.as_mut().poll(cx)
            })) {
                Ok(poll) => poll,
                Err(payload) => std::task::Poll::Ready(Err(TaskFailure::panic(payload))),
            }
        })
        .await
    };
    let outcome = active.publish(outcome);
    active.completed = true;
    outcome
}
impl Shared {
    fn acknowledge(&self, identity: u64, state: DeliveryState) {
        if let Ok(mut owner) = self.state.lock() {
            if state == DeliveryState::Abandoned
                && let Some(token) = owner.active.get(&identity)
            {
                token.cancel();
            }
            if let Some(delivery) = owner.deliveries.get_mut(&identity)
                && delivery.acknowledge(state)
            {
                owner.deliveries.remove(&identity);
            }
        }
    }
    async fn close(self: &Arc<Self>) -> Result<(), Arc<ModelError>> {
        let (service, deliveries) = loop {
            let changed = self.drained.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| Arc::new(ModelError::Conflict("native session owner")))?;
                state.closing = true;
                if let Some(result) = &state.closed {
                    return result.clone();
                }
                for token in state.active.values() {
                    token.cancel();
                }
                if state.active.is_empty()
                    && let Some(service) = state.service.take()
                {
                    break (service, std::mem::take(&mut state.deliveries));
                }
            }
            changed.await;
        };
        let mut finalizer = CloseFinalizer {
            shared: self.clone(),
            complete: false,
        };
        let mut completion = lctx_model::domain::completion::Completion::default();
        for (_, delivery) in deliveries {
            delivery.finish(&mut completion);
        }
        service.close().await;
        drop(service);
        let guard = self
            .finalization
            .lock()
            .map_err(|_| Arc::new(ModelError::Conflict("native reader finalization")))?
            .take();
        match guard {
            Some(mut guard) => completion.step(
                "native session reader close",
                guard.close(&self.reader).await,
            ),
            None => completion.step(
                "native session reader close",
                Err(ModelError::Conflict(
                    "native reader finalization owner lost",
                )),
            ),
        }
        let observation = self.signal.complete(&completion).await;
        completion.step("native signal receipt", observation);
        let result = lctx_model::domain::completion::complete(Ok(()), completion).map_err(Arc::new);
        self.state
            .lock()
            .map_err(|_| Arc::new(ModelError::Conflict("native session owner")))?
            .closed = Some(result.clone());
        finalizer.complete = true;
        self.drained.notify_waiters();
        result
    }
}
struct CloseFinalizer {
    shared: Arc<Shared>,
    complete: bool,
}
impl Drop for CloseFinalizer {
    fn drop(&mut self) {
        if !self.complete {
            self.shared.reader.retain_unknown();
            if let Ok(mut state) = self.shared.state.lock() {
                state.closed = Some(Err(Arc::new(ModelError::infrastructure(
                    lctx_model::domain::Infrastructure::Unconfirmed,
                    "native session finalization owner lost",
                ))));
            }
            self.shared.drained.notify_waiters();
        }
    }
}
struct Active {
    shared: Arc<Shared>,
    service: Option<Arc<NativeService>>,
    transport: Option<TransportRequest>,
    executing: bool,
    completed: bool,
    delivery: bool,
    identity: u64,
}
impl Active {
    fn publish(
        &mut self,
        outcome: Result<Py<pyo3::types::PyString>, TaskFailure>,
    ) -> PyResult<Py<pyo3::types::PyString>> {
        let (outcome, failure) = match outcome {
            Ok(value) => (Ok(value), None),
            Err(failure) => {
                let mut cause = failure.cause;
                if let Some(transport) = &self.transport
                    && let Err(refusal) = transport.retain(failure.retained)
                {
                    let mut completion = lctx_model::domain::completion::Completion::default();
                    completion.step("native terminal error reservation", Err(refusal));
                    cause = lctx_model::domain::completion::complete::<()>(Err(cause), completion)
                        .unwrap_err();
                }
                (Err(failure.public), Some(Arc::new(cause)))
            }
        };
        if self.delivery {
            match self.shared.state.lock() {
                Ok(mut state) => {
                    if let Some(delivery) = state.deliveries.get_mut(&self.identity) {
                        if delivery.publish(failure, self.transport.take()) {
                            state.deliveries.remove(&self.identity);
                        }
                    } else {
                        self.shared.reader.retain_unknown();
                    }
                }
                Err(_) => self.shared.reader.retain_unknown(),
            }
        }
        outcome
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum DeliveryState {
    #[default]
    Pending,
    Healthy,
    Abandoned,
}
#[derive(Default)]
struct Delivery {
    state: DeliveryState,
    terminal: bool,
    failure: Option<Arc<ModelError>>,
    transport: Option<TransportRequest>,
}
impl Delivery {
    fn acknowledge(&mut self, state: DeliveryState) -> bool {
        self.state = state;
        state == DeliveryState::Healthy && self.terminal
    }
    fn publish(
        &mut self,
        failure: Option<Arc<ModelError>>,
        transport: Option<TransportRequest>,
    ) -> bool {
        self.failure = failure;
        self.transport = transport;
        self.terminal = true;
        self.state == DeliveryState::Healthy
    }
    fn finish(self, completion: &mut lctx_model::domain::completion::Completion) {
        if let Some(error) = self.failure {
            completion.step(
                match self.state {
                    DeliveryState::Pending => "native pending delivery failed",
                    DeliveryState::Abandoned => "native abandoned delivery failed",
                    DeliveryState::Healthy => "native healthy delivery failed",
                },
                Err(ModelError::SharedCause(error)),
            );
        }
        // The request lease and charged result metadata survive task completion
        // and are released before service preparation drainage at owner close.
        drop(self.transport);
    }
}
#[pyclass(frozen)]
struct DeliveryCallback {
    shared: Arc<Shared>,
    identity: u64,
    called: std::sync::atomic::AtomicBool,
}
#[pymethods]
impl DeliveryCallback {
    fn __call__(&self, future: &Bound<'_, PyAny>) -> PyResult<()> {
        let cancelled: bool = future.call_method0("cancelled")?.extract()?;
        self.shared.acknowledge(
            self.identity,
            if cancelled {
                DeliveryState::Abandoned
            } else {
                DeliveryState::Healthy
            },
        );
        self.called
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }
}
impl Drop for DeliveryCallback {
    fn drop(&mut self) {
        if !self.called.load(std::sync::atomic::Ordering::Acquire) {
            self.shared
                .acknowledge(self.identity, DeliveryState::Abandoned);
        }
    }
}
struct TaskFailure {
    public: PyErr,
    cause: ModelError,
    retained: usize,
}
impl TaskFailure {
    fn unavailable() -> Self {
        Self {
            public: crate::unavailable(),
            cause: ModelError::Serving(lctx_model::domain::serving::FailureKind::Unavailable),
            retained: 0,
        }
    }
    fn wire(error: lctx_model::domain::serving::WireError) -> Self {
        use lctx_model::domain::serving::WireError;
        let retained = match &error {
            WireError::Invalid(text)
            | WireError::ResourceRefused(text)
            | WireError::UnknownTool(text)
            | WireError::Continuation(text) => text.capacity(),
            WireError::Failure(_) => 0,
        };
        Self {
            public: crate::public_error(error.public_failure()),
            cause: ModelError::Cause(Box::new(error)),
            retained,
        }
    }
    fn panic(payload: Box<dyn std::any::Any + Send>) -> Self {
        let retained = std::mem::size_of_val(payload.as_ref())
            + payload.downcast_ref::<String>().map_or(0, String::capacity)
            + size_of::<lctx_model::domain::completion::ThreadPanic>();
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
        let cause = ModelError::Cause(Box::new(lctx_model::domain::completion::ThreadPanic::new(
            "native accepted execution",
            payload,
        )));
        Self {
            public: crate::unavailable(),
            retained,
            cause: lctx_model::domain::completion::complete::<()>(Err(cause), completion)
                .unwrap_err(),
        }
    }
}
impl Drop for Active {
    fn drop(&mut self) {
        // Publish drainage only after retained request state and reader clone go.
        if self.executing && !self.completed {
            // Panic/runtime loss cannot promote an SDK future's disappearance
            // to terminal proof or release the remote reader authority.
            self.shared.reader.retain_unknown();
        }
        drop(self.transport.take());
        drop(self.service.take());
        if let Ok(mut state) = self.shared.state.lock() {
            state.active.remove(&self.identity);
            self.shared.drained.notify_waiters();
        }
    }
}

#[cfg(test)]
mod delivery_tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn cancelled_delivery_keeps_original_late_error_in_both_race_orders() {
        for error_first in [false, true] {
            let original = Arc::new(ModelError::Conflict("late native failure"));
            let delivery = Arc::new(Mutex::new(Delivery::default()));
            let first = Arc::new(Barrier::new(2));
            let second = Arc::new(Barrier::new(2));
            std::thread::scope(|scope| {
                let (worker_delivery, worker_error, worker_first, worker_second) = (
                    delivery.clone(),
                    original.clone(),
                    first.clone(),
                    second.clone(),
                );
                scope.spawn(move || {
                    if !error_first {
                        worker_first.wait();
                    }
                    assert!(
                        !worker_delivery
                            .lock()
                            .unwrap()
                            .publish(Some(worker_error), None)
                    );
                    if error_first {
                        worker_first.wait();
                    }
                    worker_second.wait();
                });
                if error_first {
                    first.wait();
                }
                assert!(
                    !delivery
                        .lock()
                        .unwrap()
                        .acknowledge(DeliveryState::Abandoned)
                );
                if !error_first {
                    first.wait();
                }
                second.wait();
            });
            let delivery = Arc::try_unwrap(delivery)
                .ok()
                .unwrap()
                .into_inner()
                .unwrap();
            assert!(delivery.terminal);
            assert!(Arc::ptr_eq(delivery.failure.as_ref().unwrap(), &original));
            let mut completion = lctx_model::domain::completion::Completion::default();
            delivery.finish(&mut completion);
            assert_eq!(
                completion.failures[0].step,
                "native abandoned delivery failed"
            );
            assert!(
                matches!(&completion.failures[0].error, ModelError::SharedCause(error) if Arc::ptr_eq(error, &original))
            );
        }
    }
    #[test]
    fn healthy_python_future_transfers_error_and_releases_ledger_owner() {
        let original = Arc::new(ModelError::Conflict("delivered native failure"));
        let mut delivery = Delivery::default();
        assert!(!delivery.publish(Some(original.clone()), None));
        assert_eq!(Arc::strong_count(&original), 2);
        assert!(delivery.acknowledge(DeliveryState::Healthy));
        drop(delivery);
        assert_eq!(Arc::strong_count(&original), 1);
    }
    #[test]
    fn close_retains_pending_error_distinctly_without_claiming_delivery() {
        let original = Arc::new(ModelError::Conflict("not yet delivered"));
        let mut delivery = Delivery::default();
        delivery.publish(Some(original.clone()), None);
        let mut completion = lctx_model::domain::completion::Completion::default();
        delivery.finish(&mut completion);
        assert_eq!(
            completion.failures[0].step,
            "native pending delivery failed"
        );
        assert!(
            matches!(&completion.failures[0].error, ModelError::SharedCause(error) if Arc::ptr_eq(error, &original))
        );
    }
}
