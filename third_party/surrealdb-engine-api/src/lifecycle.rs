//! Lifecycle acknowledgments and connection-task observations, with no client ownership.
use std::sync::{Arc, Mutex};
use surrealdb_rpc::IngressReservation;
use surrealdb_types::Error;
use tokio::sync::Notify;
// Fresh local diagnostics are bounded literals. Remote causes keep their original
// ingress resources; sharing them admits only the additional Arc/cause wrapper.
fn session_local_error_bytes() -> usize {
    "Session lifecycle owner is unavailable"
        .len()
        .max("Session close already registered".len())
        .max("Explicit session close lost its ready session".len())
}
fn connection_local_error_bytes() -> usize {
    "Explicit session detachment was not acknowledged"
        .len()
        .max("Dropped session retains unconfirmed detachment".len())
}
#[derive(Debug)]
struct UnacknowledgedLifecycle;
impl std::fmt::Display for UnacknowledgedLifecycle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Lifecycle owner ended without acknowledgment")
    }
}
impl std::error::Error for UnacknowledgedLifecycle {}

/// Shares the original typed terminal cause and its admission grant without deep cloning.
#[derive(Clone, Debug)]
pub struct LifecycleError {
    error: Arc<dyn std::error::Error + Send + Sync>,
    _charge: Option<Arc<dyn IngressReservation>>,
}
impl std::fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.error.as_ref(), formatter)
    }
}
impl std::error::Error for LifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.error.as_ref())
    }
}
impl LifecycleError {
    fn admitted(
        error: impl std::error::Error + Send + Sync + 'static,
        charge: Option<Arc<dyn IngressReservation>>,
    ) -> Self {
        Self {
            error: Arc::new(error),
            _charge: charge,
        }
    }
}
impl From<Error> for LifecycleError {
    fn from(error: Error) -> Self {
        Self::admitted(error, None)
    }
}
#[derive(Debug)]
struct State {
    result: Mutex<Option<Result<(), LifecycleError>>>,
    ready: Notify,
    charge: Option<Arc<dyn IngressReservation>>,
}
/// Observes one ordered session close; owns no session, client, router or engine.
#[derive(Clone, Debug)]
pub struct LifecycleCompletion(Arc<State>);
/// Producer capability retained by the actual queued lifecycle operation.
#[derive(Clone, Debug)]
pub struct LifecycleAcknowledgment(Arc<Publisher>);
#[derive(Debug)]
struct Publisher(Arc<State>);
impl LifecycleCompletion {
    /// Shared state, publisher, Arc headers and one bounded terminal-cause wrapper.
    pub fn allocation_bytes() -> usize {
        size_of::<State>()
            + size_of::<Publisher>()
            + size_of::<Error>().max(size_of::<tokio::task::JoinError>())
            + 6 * size_of::<usize>()
            + session_local_error_bytes()
    }
    /// The backing grant must precede creation and survives every escaped shared error.
    pub fn channel(charge: Option<Arc<dyn IngressReservation>>) -> (Self, LifecycleAcknowledgment) {
        let state = Arc::new(State {
            result: Mutex::new(None),
            ready: Notify::new(),
            charge,
        });
        (
            Self(state.clone()),
            LifecycleAcknowledgment(Arc::new(Publisher(state))),
        )
    }
    pub async fn wait(&self) -> Result<(), LifecycleError> {
        loop {
            let ready = self.0.ready.notified();
            if let Some(result) = self
                .0
                .result
                .lock()
                .expect("lifecycle completion poisoned")
                .clone()
            {
                return result;
            }
            ready.await;
        }
    }
}
impl LifecycleAcknowledgment {
    pub fn complete(&self, result: Result<(), Error>) {
        let mut slot = self
            .0
            .0
            .result
            .lock()
            .expect("lifecycle completion poisoned");
        if slot.is_none() {
            *slot = Some(
                result.map_err(|error| LifecycleError::admitted(error, self.0.0.charge.clone())),
            );
        }
        drop(slot);
        self.0.0.ready.notify_waiters();
    }
}
impl Drop for Publisher {
    fn drop(&mut self) {
        let mut slot = self.0.result.lock().expect("lifecycle completion poisoned");
        if slot.is_none() {
            *slot = Some(Err(LifecycleError::admitted(
                UnacknowledgedLifecycle,
                self.0.charge.clone(),
            )));
        }
        drop(slot);
        self.0.ready.notify_waiters();
    }
}
/// Joins the SDK session-lifecycle task and its admitted engine backing after all handles drop.
/// This does not qualify H2/TLS/runtime workspace or remote query/executor terminality.
#[derive(Clone, Debug)]
pub struct ConnectionCompletion(Arc<ConnectionState>);
#[derive(Debug)]
struct ConnectionState {
    joined: tokio::sync::Mutex<JoinState>,
    charge: Option<Arc<dyn IngressReservation>>,
}
#[derive(Debug)]
enum JoinState {
    Running(tokio::task::JoinHandle<Result<(), Error>>),
    Complete(Result<(), LifecycleError>),
}
impl ConnectionCompletion {
    pub fn allocation_bytes() -> usize {
        size_of::<ConnectionState>()
            + size_of::<Error>().max(size_of::<tokio::task::JoinError>())
            + 4 * size_of::<usize>()
            + connection_local_error_bytes()
    }
    /// Admit shared backing before allocation. The join handle owns no client/router handle.
    pub fn new(
        task: tokio::task::JoinHandle<Result<(), Error>>,
        charge: Option<Arc<dyn IngressReservation>>,
    ) -> Self {
        Self(Arc::new(ConnectionState {
            joined: tokio::sync::Mutex::new(JoinState::Running(task)),
            charge,
        }))
    }
    /// Cancellation leaves the sole actual join handle registered for the next observer.
    pub async fn wait(&self) -> Result<(), LifecycleError> {
        let mut joined = self.0.joined.lock().await;
        if let JoinState::Running(task) = &mut *joined {
            let result = match task.await {
                Ok(result) => {
                    result.map_err(|error| LifecycleError::admitted(error, self.0.charge.clone()))
                }
                Err(error) => Err(LifecycleError::admitted(error, self.0.charge.clone())),
            };
            *joined = JoinState::Complete(result);
        }
        match &*joined {
            JoinState::Complete(result) => result.clone(),
            JoinState::Running(_) => unreachable!("joined lifecycle task"),
        }
    }
}
