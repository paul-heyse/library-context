//! Bounded synchronous provider access to asynchronous native operations.
//!
//! The actor has its own runtime. A provider may consume a read stream and submit writes
//! while that stream remains open; each request is an owned task rather than a serial actor
//! operation. Shutdown wakes callers and drains submitted operations before releasing the runtime.
use crate::workspace::Cancellation;
use futures::future::BoxFuture;
use lctx_model::domain::ModelError;
use std::{sync::Mutex, time::Duration};
use tokio::sync::{mpsc, oneshot};

type Request = BoxFuture<'static, ()>;
const WAIT: Duration = Duration::from_millis(20);

pub(crate) struct NativeBridge {
    requests: mpsc::Sender<Request>,
    cancellation: Cancellation,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
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
                    loop {
                        tokio::select! {
                            biased;
                            () = actor_cancel.cancelled() => break,
                            request = receiver.recv(), if tasks.len() < 8 => match request {
                                Some(request) => { tasks.spawn(request); }
                                None => break,
                            },
                            _ = tasks.join_next(), if !tasks.is_empty() => {},
                        }
                    }
                    receiver.close();
                    // Queued work has not reached native ownership. Already submitted tasks
                    // retain their acknowledgements even when the synchronous caller cancelled.
                    while receiver.try_recv().is_ok() {}
                    while tasks.join_next().await.is_some() {}
                });
            })
            .map_err(ModelError::codec)?;
        Ok(Self { requests, cancellation, thread: Mutex::new(Some(thread)) })
    }
    /// The future owns its bounded, budget-charged arguments until acknowledgement.
    pub(crate) fn call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> Result<T, ModelError> {
        self.cancellation.check()?;
        let (answer, mut result) = oneshot::channel();
        let request: Request = Box::pin(async move {
            let value=future.await;
            let _=answer.send(value);
        });
        self.launch(request)?;
        loop {
            self.cancellation.check()?;
            match result.try_recv() {
                Ok(value) => return value,
                Err(oneshot::error::TryRecvError::Empty) => std::thread::sleep(WAIT),
                Err(oneshot::error::TryRecvError::Closed) => return Err(closed()),
            }
        }
    }
    pub(crate) fn launch(&self, mut request: Request) -> Result<(), ModelError> {
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
        let thread = self.thread.lock().map_err(|_| closed())?.take();
        if let Some(thread) = thread {
            tokio::task::spawn_blocking(move || thread.join())
                .await.map_err(ModelError::codec)?
                .map_err(|_| closed())?;
        }
        Ok(())
    }
}
impl Drop for NativeBridge {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Ok(thread) = self.thread.get_mut()
            && let Some(thread) = thread.take()
        {
            // Explicit drainage is required by the attempt owner. This fallback keeps an
            // interrupted owner from synchronously blocking a runtime worker in its destructor.
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn_blocking(move || { let _ = thread.join(); });
            } else {
                let _ = thread.join();
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
    async fn actor_does_not_need_the_callers_runtime_to_acknowledge() {
        let bridge = NativeBridge::new(Cancellation::default()).unwrap();
        assert_eq!(bridge.call(async { Ok(17) }).unwrap(), 17);
        bridge.drain().await.unwrap();
    }
    #[tokio::test(flavor = "current_thread")]
    async fn cancellation_wakes_a_blocked_method_and_drains_its_future() {
        let cancellation = Cancellation::default();
        let bridge = Arc::new(NativeBridge::new(cancellation.clone()).unwrap());
        let entered = Arc::new(tokio::sync::Notify::new());
        let completed=Arc::new(std::sync::atomic::AtomicBool::new(false));
        let request_completed=completed.clone();
        let request_bridge = bridge.clone();
        let request_entered = entered.clone();
        let request = std::thread::spawn(move || request_bridge.call(async move {
            request_entered.notify_one();
            tokio::time::sleep(Duration::from_millis(80)).await;
            request_completed.store(true,std::sync::atomic::Ordering::Release);
            Ok::<(),ModelError>(())
        }));
        entered.notified().await;
        cancellation.cancel();
        bridge.drain().await.unwrap();
        assert!(request.join().unwrap().is_err());
        assert!(completed.load(std::sync::atomic::Ordering::Acquire));
    }
}
