//! Bounded synchronous provider access to asynchronous native operations.
//!
//! The actor has its own runtime. A provider may consume a read stream and submit writes
//! while that stream remains open; each request is an owned task rather than a serial actor
//! operation. Shutdown wakes callers and drains submitted operations before releasing the runtime.
use crate::admission::Cancellation;
use crate::dispatcher_completion::{DispatcherReport, OperationResult};
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use lctx_model::domain::ModelError;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tracing::{Instrument, instrument::WithSubscriber};

type Request = BoxFuture<'static, OperationResult>;
type BridgeDrain = Shared<BoxFuture<'static, Result<DispatcherReport, Arc<ModelError>>>>;
enum BridgeEvent {
    Cancelled,
    Request(Option<Request>),
    Completed(Option<Result<OperationResult, tokio::task::JoinError>>),
}
async fn next_event(
    receiver: &mut mpsc::Receiver<Request>,
    tasks: &mut tokio::task::JoinSet<OperationResult>,
    cancellation: &Cancellation,
) -> BridgeEvent {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => BridgeEvent::Cancelled,
        // Reap finished owners and late errors even while input remains continuously ready.
        result = tasks.join_next(), if !tasks.is_empty() => BridgeEvent::Completed(result),
        request = receiver.recv() => BridgeEvent::Request(request),
    }
}

pub struct NativeBridge {
    requests: mpsc::Sender<Request>,
    cancellation: Cancellation,
    thread: Mutex<BridgeThread>,
}
struct BridgeThread {
    thread: Option<std::thread::JoinHandle<DispatcherReport>>,
    joined: Option<BridgeDrain>,
}
impl NativeBridge {
    pub fn new(cancellation: Cancellation) -> Result<Self, ModelError> {
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
                    let mut completion = DispatcherReport::default();
                    loop {
                        match next_event(&mut receiver, &mut tasks, &actor_cancel).await {
                            BridgeEvent::Cancelled => break,
                            // A retained read driver may wait for a consumer that must submit
                            // writes. Its lifetime cannot occupy a dispatcher admission slot.
                            // Submitted arguments/streams retain their existing byte charges.
                            BridgeEvent::Request(request) => match request {
                                Some(request) => {
                                    tasks.spawn(request);
                                }
                                None => break,
                            },
                            BridgeEvent::Completed(result) => {
                                if let Some(result) = result {
                                    completion.joined(result);
                                }
                            }
                        }
                    }
                    receiver.close();
                    // Queued work has not reached native ownership. Already submitted tasks
                    // retain their acknowledgements even when the synchronous caller cancelled.
                    while receiver.try_recv().is_ok() {}
                    while let Some(result) = tasks.join_next().await {
                        completion.joined(result);
                    }
                    completion
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
    pub fn call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> Result<T, ModelError> {
        self.call_boxed(future.boxed())
    }
    pub fn call_boxed<T: Send + 'static>(
        &self,
        future: BoxFuture<'static, Result<T, ModelError>>,
    ) -> Result<T, ModelError> {
        // Only genuinely synchronous provider callbacks use this entry point. Async
        // compiler consumers await call_async/call_boxed_async instead.
        futures::executor::block_on(self.call_boxed_async(future))
    }
    #[cfg(test)]
    pub fn call_async<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        self.call_boxed_async(future.boxed())
    }
    pub fn call_boxed_async<T: Send + 'static>(
        &self,
        future: BoxFuture<'static, Result<T, ModelError>>,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        async move {
        self.cancellation.check()?;
        let (answer, result) = oneshot::channel();
        // Capture this request's origin before it crosses the actor queue. The actor runtime
        // polls each request under its own dispatcher, independently of the actor thread.
        let request: Request = async move {
            let value = future.await;
            OperationResult::deliver(answer, value)
        }
        .in_current_span()
        .with_current_subscriber()
        .boxed();
        self.submit_async(request).await?;
        tokio::select! {
            biased;
            () = self.cancellation.cancelled() => {
                self.cancellation.check()?;
                Err(closed())
            },
            value = lctx_native::phase::wait("provider_bridge_acknowledgement", result) => value.map_err(|_| closed())?,
        }
        }.boxed()
    }
    pub fn launch(&self, request: BoxFuture<'static, ()>) -> Result<(), ModelError> {
        self.submit(
            async move {
                request.await;
                OperationResult::default()
            }
            .in_current_span()
            .with_current_subscriber()
            .boxed(),
        )
    }
    pub async fn launch_async(&self, request: BoxFuture<'static, ()>) -> Result<(), ModelError> {
        self.submit_async(
            async move {
                request.await;
                OperationResult::default()
            }
            .in_current_span()
            .with_current_subscriber()
            .boxed(),
        )
        .await
    }
    fn submit(&self, request: Request) -> Result<(), ModelError> {
        futures::executor::block_on(self.submit_async(request))
    }
    async fn submit_async(&self, request: Request) -> Result<(), ModelError> {
        self.cancellation.check()?;
        tokio::select! {
            biased;
            () = self.cancellation.cancelled() => {
                self.cancellation.check()?;
                Err(closed())
            },
            result = lctx_native::phase::wait("provider_bridge_admission", self.requests.send(request)) => result.map_err(|_| closed()),
        }
    }
    /// No runtime worker waits inline for an actor thread. Caller invokes this before cleanup.
    pub async fn drain(&self) -> Result<(), ModelError> {
        let mut completion = self.drain_finality_report().await;
        self.append_operation_failures(&mut completion);
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
    pub async fn drain_finality_report(&self) -> lctx_model::domain::completion::Completion {
        use lctx_model::domain::completion::{Completion, LocalState};
        self.cancellation.cancel();
        let mut completion = Completion::default();
        let joined = {
            let mut owned = match self.thread.lock() {
                Ok(owned) => owned,
                Err(_) => {
                    completion.local = LocalState::Outstanding;
                    completion.step("native bridge ownership", Err(closed()));
                    return completion;
                }
            };
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
                            })
                    }
                    .boxed()
                    .shared(),
                );
            }
            owned.joined.clone()
        };
        if let Some(joined) = joined {
            match lctx_native::phase::wait("provider_bridge_join", joined).await {
                Ok(report) => completion.merge(report.finality()),
                Err(error) => completion.step(
                    "native bridge thread join",
                    Err(ModelError::SharedCause(error)),
                ),
            }
        }
        completion
    }
    pub fn append_operation_failures(
        &self,
        completion: &mut lctx_model::domain::completion::Completion,
    ) {
        if let Ok(owned) = self.thread.lock()
            && let Some(joined) = &owned.joined
            && let Some(Ok(report)) = joined.clone().now_or_never()
        {
            report.append_diagnostics(completion);
        }
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
    use std::time::Duration;
    #[tokio::test(flavor = "current_thread")]
    async fn completed_tasks_are_reaped_while_input_remains_ready() {
        let (sender, mut receiver) = mpsc::channel::<Request>(1);
        assert!(sender.try_send(futures::future::pending().boxed()).is_ok());
        let cancellation = Cancellation::default();
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..32 {
            let task = tasks.spawn(async { OperationResult::default() });
            while !task.is_finished() {
                tokio::task::yield_now().await;
            }
            assert!(matches!(
                next_event(&mut receiver, &mut tasks, &cancellation).await,
                BridgeEvent::Completed(Some(Ok(_)))
            ));
            assert_eq!(
                receiver.len(),
                1,
                "ready input cannot starve terminal ownership"
            );
            assert!(tasks.is_empty());
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn retained_read_drivers_do_not_starve_a_write_their_consumers_need() {
        let bridge = NativeBridge::new(Cancellation::default()).unwrap();
        let finish = Arc::new(tokio::sync::Notify::new());
        let mut entered = Vec::new();
        // More drivers than the old dispatcher lifetime limit, all waiting for a write.
        for _ in 0..9 {
            let (ready, receiver) = oneshot::channel();
            let release = finish.clone();
            bridge
                .launch_async(
                    async move {
                        let waiting = release.notified();
                        tokio::pin!(waiting);
                        waiting.as_mut().enable();
                        ready.send(()).unwrap();
                        waiting.await;
                    }
                    .boxed(),
                )
                .await
                .unwrap();
            entered.push(receiver);
        }
        for ready in entered {
            ready.await.unwrap();
        }
        assert_eq!(bridge.call_async(async { Ok(17) }).await.unwrap(), 17);
        finish.notify_waiters();
        bridge.drain().await.unwrap();
    }
    #[tokio::test(flavor = "current_thread")]
    async fn cancellation_wakes_full_queue_admission_without_polling_or_submission() {
        let cancellation = Cancellation::default();
        let (requests, mut receiver) = mpsc::channel::<Request>(1);
        assert!(
            requests
                .try_send(futures::future::pending().boxed())
                .is_ok()
        );
        let bridge = NativeBridge {
            requests,
            cancellation: cancellation.clone(),
            thread: Mutex::new(BridgeThread {
                thread: None,
                joined: None,
            }),
        };
        let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let executed = observed.clone();
        let mut waiting = Box::pin(bridge.call_async(async move {
            executed.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        }));
        assert!(futures::poll!(&mut waiting).is_pending());
        cancellation.cancel();
        assert!(waiting.await.is_err());
        assert!(!observed.load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(
            receiver.len(),
            1,
            "unsubmitted work never entered the queue"
        );
        receiver.close();
    }
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
        let finality = bridge.drain_finality_report().await;
        assert!(
            finality.failures.is_empty(),
            "acknowledged refusal is not failed ownership"
        );
        assert_eq!(
            finality.remote,
            lctx_model::domain::completion::RemoteState::Confirmed
        );
        let error = bridge.drain().await.unwrap_err();
        let ModelError::Completion(outcome) = error else {
            panic!()
        };
        assert_eq!(
            outcome.completion.local,
            lctx_model::domain::completion::LocalState::Terminal
        );
        assert!(matches!(
            outcome.completion.failures[0].error.primary(),
            Some(ModelError::Schema("late bridge failure"))
        ));
    }
    #[tokio::test(flavor = "current_thread")]
    async fn bridge_preserves_nested_owner_finality_with_live_or_dropped_receiver() {
        use lctx_model::domain::completion::{
            self, Completion, LocalState, RemoteState, StorageState,
        };
        for dropped in [false, true] {
            let bridge = Arc::new(NativeBridge::new(Cancellation::default()).unwrap());
            let entered = Arc::new(tokio::sync::Notify::new());
            let release = Arc::new(tokio::sync::Notify::new());
            let owner = bridge.clone();
            let running = entered.clone();
            let finish = release.clone();
            let caller = tokio::spawn(async move {
                owner
                    .call_async(async move {
                        running.notify_one();
                        finish.notified().await;
                        let mut actual = Completion {
                            local: LocalState::Outstanding,
                            remote: RemoteState::Unknown,
                            ..Default::default()
                        };
                        actual
                            .storage
                            .push(StorageState::Orphan("protected".into()));
                        actual.committed("publication", "exact");
                        actual.step("nested cleanup", Err(ModelError::Schema("cleanup failed")));
                        completion::complete::<()>(Err(ModelError::Schema("refused")), actual)
                    })
                    .await
            });
            entered.notified().await;
            let returned = if dropped {
                caller.abort();
                let returned = caller.await;
                release.notify_one();
                returned
            } else {
                release.notify_one();
                caller.await
            };
            if dropped {
                assert!(returned.is_err());
            } else {
                assert!(!returned.unwrap().unwrap_err().permits_storage_cleanup());
            }
            for _ in 0..2 {
                let actual = bridge.drain_finality_report().await;
                assert_eq!(actual.local, LocalState::Outstanding);
                assert_eq!(actual.remote, RemoteState::Unknown);
                assert_eq!(
                    actual.storage,
                    vec![StorageState::Orphan("protected".into())]
                );
                assert_eq!(actual.committed[0].identity, "exact");
                assert_eq!(actual.failures.len(), 1);
                assert_eq!(actual.failures[0].step, "nested cleanup");
                assert!(matches!(
                    actual.failures[0].error.primary(),
                    Some(ModelError::Schema("cleanup failed"))
                ));
                let mut diagnostics = Completion::default();
                bridge.append_operation_failures(&mut diagnostics);
                assert_eq!(diagnostics.failures.len(), usize::from(dropped));
            }
        }
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
            let ModelError::Completion(outcome) = error else {
                panic!()
            };
            assert_eq!(
                outcome.completion.local,
                lctx_model::domain::completion::LocalState::Terminal
            );
            assert_eq!(outcome.completion.failures.len(), 1);
            assert!(
                matches!(&outcome.completion.failures[0].error,ModelError::SharedCause(error) if matches!(error.as_ref(),ModelError::Cause(error) if error.downcast_ref::<tokio::task::JoinError>().is_some_and(tokio::task::JoinError::is_panic)))
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
