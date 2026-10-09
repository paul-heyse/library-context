//! Bounded synchronous provider access to asynchronous native operations.
//!
//! The actor has its own runtime. A provider may consume a read stream and submit writes
//! while that stream remains open; each request is an owned task rather than a serial actor
//! operation. Shutdown wakes callers and drains submitted operations before releasing the runtime.
use crate::workspace::Cancellation;
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use lctx_model::domain::ModelError;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use tracing::{Instrument, instrument::WithSubscriber};

type Request = BoxFuture<'static, Result<(), ModelError>>;
type BridgeDrain = Shared<BoxFuture<'static, Result<(), Arc<ModelError>>>>;
const WAIT: Duration = Duration::from_millis(20);

pub(crate) struct NativeBridge {
    requests: mpsc::Sender<Request>,
    cancellation: Cancellation,
    thread: Mutex<BridgeThread>,
}
struct BridgeThread {
    thread: Option<std::thread::JoinHandle<Result<(), ModelError>>>,
    joined: Option<BridgeDrain>,
}
impl NativeBridge {
    pub(crate) fn new(cancellation: Cancellation) -> Result<Self, ModelError> {
        let (requests, mut receiver) = mpsc::channel::<Request>(2);
        let actor_cancel = cancellation.clone();
        let thread = std::thread::Builder::new()
            .name("lctx-native-provider-bridge".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("native bridge runtime");
                runtime.block_on(async move {
                    let mut tasks = tokio::task::JoinSet::new();
                    let mut completion=lctx_model::domain::completion::Completion::default();
                    loop {
                        tokio::select! {
                            biased;
                            () = actor_cancel.cancelled() => break,
                            request = receiver.recv(), if tasks.len() < 8 => match request {
                                Some(request) => { tasks.spawn(request); }
                                None => break,
                            },
                            result = tasks.join_next(), if !tasks.is_empty() => {
                                if let Some(result)=result {completion.step("native bridge task join",result.map_err(|error|ModelError::Cause(Box::new(error))).and_then(|result|result));}
                            },
                        }
                    }
                    receiver.close();
                    // Queued work has not reached native ownership. Already submitted tasks
                    // retain their acknowledgements even when the synchronous caller cancelled.
                    while receiver.try_recv().is_ok() {}
                    while let Some(result)=tasks.join_next().await {completion.step("native bridge task join",result.map_err(|error|ModelError::Cause(Box::new(error))).and_then(|result|result));}
                    lctx_model::domain::completion::complete(Ok(()),completion)
                })
            })
            .map_err(ModelError::codec)?;
        Ok(Self {
            requests,
            cancellation,
            thread: Mutex::new(BridgeThread {
                thread: Some(thread),
                joined: None,
            }),
        })
    }
    /// The future owns its bounded, budget-charged arguments until acknowledgement.
    pub(crate) fn call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> Result<T, ModelError> {
        self.call_boxed(future.boxed())
    }
    pub(crate) fn call_boxed<T: Send + 'static>(
        &self,
        future: BoxFuture<'static, Result<T, ModelError>>,
    ) -> Result<T, ModelError> {
        self.cancellation.check()?;
        let (answer, mut result) = oneshot::channel();
        // Capture this request's origin before it crosses the actor queue. The actor runtime
        // polls each request under its own dispatcher, independently of the actor thread.
        let request: Request = async move {
            let value = future.await;
            match answer.send(value) {
                Err(Err(error)) => Err(error),
                _ => Ok(()),
            }
        }
        .in_current_span()
        .with_current_subscriber()
        .boxed();
        self.submit(request)?;
        loop {
            self.cancellation.check()?;
            match result.try_recv() {
                Ok(value) => return value,
                Err(oneshot::error::TryRecvError::Empty) => std::thread::sleep(WAIT),
                Err(oneshot::error::TryRecvError::Closed) => return Err(closed()),
            }
        }
    }
    pub(crate) fn launch(&self, request: BoxFuture<'static, ()>) -> Result<(), ModelError> {
        self.submit(
            async move {
                request.await;
                Ok(())
            }
            .in_current_span()
            .with_current_subscriber()
            .boxed(),
        )
    }
    fn submit(&self, mut request: Request) -> Result<(), ModelError> {
        loop {
            self.cancellation.check()?;
            match self.requests.try_send(request) {
                Ok(()) => return Ok(()),
                Err(mpsc::error::TrySendError::Full(returned)) => {
                    request = returned;
                    std::thread::sleep(WAIT);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => return Err(closed()),
            }
        }
    }
    /// No runtime worker waits inline for an actor thread. Caller invokes this before cleanup.
    pub(crate) async fn drain(&self) -> Result<(), ModelError> {
        self.cancellation.cancel();
        let joined = {
            let mut owned = self.thread.lock().map_err(|_| {
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.local = lctx_model::domain::completion::LocalState::Outstanding;
                lctx_model::domain::completion::complete::<()>(Err(closed()), completion)
                    .unwrap_err()
            })?;
            if let Some(thread) = owned.thread.take() {
                let dispatch = tracing::dispatcher::get_default(Clone::clone);
                let span = tracing::Span::current();
                let task = tokio::task::spawn_blocking(move || {
                    tracing::dispatcher::with_default(&dispatch, || span.in_scope(|| thread.join()))
                });
                owned.joined = Some(
                    async move {
                        task.await
                            .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))?
                            .map_err(|payload| {
                                Arc::new(ModelError::Cause(Box::new(
                                    lctx_model::domain::completion::ThreadPanic::new(
                                        "native bridge",
                                        payload,
                                    ),
                                )))
                            })?
                            .map_err(Arc::new)
                    }
                    .boxed()
                    .shared(),
                );
            }
            owned.joined.clone()
        };
        if let Some(joined) = joined {
            joined.await.map_err(ModelError::SharedCause)?;
        }
        Ok(())
    }
}
impl Drop for NativeBridge {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Ok(thread) = self.thread.get_mut()
            && let Some(thread) = thread.thread.take()
        {
            // Explicit drainage is required by the attempt owner. This fallback keeps an
            // interrupted owner from synchronously blocking a runtime worker in its destructor.
            let dispatch = tracing::dispatcher::get_default(Clone::clone);
            let span = tracing::Span::current();
            let join = move || {
                tracing::dispatcher::with_default(&dispatch, || {
                    span.in_scope(|| {
                        let _ = thread.join();
                    })
                })
            };
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn_blocking(join);
            } else {
                join();
            }
        }
    }
}
fn closed() -> ModelError {
    ModelError::Invalid("native provider bridge closed before acknowledgement".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_bridge_caller_retains_late_failure_in_completion() {
        let cancellation = Cancellation::default();
        let bridge = Arc::new(NativeBridge::new(cancellation.clone()).unwrap());
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = bridge.clone();
        let running = entered.clone();
        let finish = release.clone();
        let caller = std::thread::spawn(move || {
            owner.call(async move {
                running.notify_one();
                finish.notified().await;
                Err::<(), _>(ModelError::Schema("late bridge failure"))
            })
        });
        entered.notified().await;
        cancellation.cancel();
        assert!(caller.join().unwrap().is_err());
        release.notify_one();
        let error = bridge.drain().await.unwrap_err();
        let ModelError::SharedCause(cause) = error else {
            panic!()
        };
        let ModelError::Completion(outcome) = cause.as_ref() else {
            panic!()
        };
        assert_eq!(
            outcome.completion.local,
            lctx_model::domain::completion::LocalState::Terminal
        );
        assert!(matches!(
            &outcome.completion.failures[0].error,
            ModelError::Schema("late bridge failure")
        ));
    }
    #[tokio::test(flavor = "current_thread")]
    async fn completed_failed_bridge_join_is_terminal_and_retained_for_retry() {
        let bridge = NativeBridge::new(Cancellation::default()).unwrap();
        let entered = Arc::new(tokio::sync::Notify::new());
        let running = entered.clone();
        bridge
            .launch(Box::pin(async move {
                running.notify_one();
                panic!("injected task panic")
            }))
            .unwrap();
        entered.notified().await;
        for _ in 0..2 {
            let error = bridge.drain().await.unwrap_err();
            assert!(error.permits_storage_cleanup());
            let ModelError::SharedCause(cause) = error else {
                panic!()
            };
            let ModelError::Completion(outcome) = cause.as_ref() else {
                panic!()
            };
            assert_eq!(
                outcome.completion.local,
                lctx_model::domain::completion::LocalState::Terminal
            );
            assert_eq!(outcome.completion.failures.len(), 1);
            assert!(
                matches!(&outcome.completion.failures[0].error,ModelError::Cause(error) if error.downcast_ref::<tokio::task::JoinError>().is_some_and(tokio::task::JoinError::is_panic))
            );
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn actor_does_not_need_the_callers_runtime_to_acknowledge() {
        let bridge = NativeBridge::new(Cancellation::default()).unwrap();
        assert_eq!(bridge.call(async { Ok(17) }).unwrap(), 17);
        bridge.drain().await.unwrap();
    }
    #[tokio::test(flavor = "current_thread")]
    async fn interrupted_drain_retains_actor_join_for_retry() {
        let bridge = Arc::new(NativeBridge::new(Cancellation::default()).unwrap());
        let entered = Arc::new(tokio::sync::Notify::new());
        let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let call_owner = bridge.clone();
        let call_entered = entered.clone();
        let call_completed = completed.clone();
        let request = std::thread::spawn(move || {
            call_owner.call(async move {
                call_entered.notify_one();
                tokio::time::sleep(Duration::from_millis(80)).await;
                call_completed.store(true, std::sync::atomic::Ordering::Release);
                Ok::<(), ModelError>(())
            })
        });
        entered.notified().await;
        let first_owner = bridge.clone();
        let first = tokio::spawn(async move { first_owner.drain().await });
        tokio::time::sleep(Duration::from_millis(5)).await;
        first.abort();
        assert!(first.await.is_err());
        bridge.drain().await.unwrap();
        assert!(completed.load(std::sync::atomic::Ordering::Acquire));
        assert!(request.join().unwrap().is_err());
    }
    #[tokio::test(flavor = "current_thread")]
    async fn cancellation_wakes_a_blocked_method_and_drains_its_future() {
        let cancellation = Cancellation::default();
        let bridge = Arc::new(NativeBridge::new(cancellation.clone()).unwrap());
        let entered = Arc::new(tokio::sync::Notify::new());
        let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let request_completed = completed.clone();
        let request_bridge = bridge.clone();
        let request_entered = entered.clone();
        let request = std::thread::spawn(move || {
            request_bridge.call(async move {
                request_entered.notify_one();
                tokio::time::sleep(Duration::from_millis(80)).await;
                request_completed.store(true, std::sync::atomic::Ordering::Release);
                Ok::<(), ModelError>(())
            })
        });
        entered.notified().await;
        cancellation.cancel();
        bridge.drain().await.unwrap();
        assert!(request.join().unwrap().is_err());
        assert!(completed.load(std::sync::atomic::Ordering::Acquire));
    }
}
