//! Submitted asynchronous native calls retain task ownership through caller cancellation.
use crate::workspace::Cancellation;
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use lctx_surrealdb::compiler::NativeCompilerStore;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Semaphore, oneshot};
use tracing::{Instrument, instrument::WithSubscriber};

pub(crate) struct NativeCalls {
    native: Arc<NativeCompilerStore>,
    cancellation: Cancellation,
    limit: Arc<Semaphore>,
    tasks: Mutex<(Vec<NativeCallTask>, StateCharge)>,
}
type Request = BoxFuture<'static, Result<(), Arc<ModelError>>>;
struct NativeCallTask {
    joined: Shared<BoxFuture<'static, Result<(), Arc<ModelError>>>>,
    finished: Arc<AtomicBool>,
}
struct CallGuard {
    native: Arc<NativeCompilerStore>,
    cancellation: Cancellation,
    finished: bool,
    done: Option<Arc<AtomicBool>>,
}
impl Drop for CallGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.native.fail();
            self.cancellation.cancel();
        }
        if let Some(done) = &self.done {
            done.store(true, Ordering::Release);
        }
    }
}
impl NativeCalls {
    pub(crate) fn new(
        native: Arc<NativeCompilerStore>,
        cancellation: Cancellation,
        budget: &ResourceBudget,
    ) -> Self {
        Self {
            native,
            cancellation,
            limit: Arc::new(Semaphore::new(8)),
            tasks: Mutex::new((
                Vec::new(),
                StateCharge::new(budget, "native-call-ownership"),
            )),
        }
    }
    /// Erase the complete operation before entering task admission or Tokio scheduling.
    pub(crate) fn call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        self.call_boxed(future.boxed())
    }
    /// Keep the typed acknowledgement adapter independent of the concrete operation future.
    pub(crate) fn call_boxed<T: Send + 'static>(
        &self,
        future: BoxFuture<'static, Result<T, ModelError>>,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        let (answer, acknowledgement) = oneshot::channel();
        let native = self.native.clone();
        let cancellation = self.cancellation.clone();
        // Only the result channel is typed; submitted ownership and scheduler work are shared.
        let request: Request = async move {
            let result = future.await;
            if result.is_err() {
                fail(&native, &cancellation);
            }
            match answer.send(result) {
                Err(Err(error)) => Err(Arc::new(error)),
                _ => Ok(()),
            }
        }
        .in_current_span()
        .with_current_subscriber()
        .boxed();
        async move {
            let mut waiting = self.submit_guarded(request).await?;
            let result = acknowledgement.await.map_err(|_| {
                ModelError::Invalid("native call ended without acknowledgement".into())
            })?;
            waiting.finished = true;
            result
        }
        .boxed()
    }
    // Return the guard to the acknowledgement adapter: dropping the caller after submission
    // must still fail/cancel the attempt while the registered native task runs to terminality.
    async fn submit_guarded(&self, request: Request) -> Result<CallGuard, ModelError> {
        self.cancellation.check()?;
        let waiting = CallGuard {
            native: self.native.clone(),
            cancellation: self.cancellation.clone(),
            finished: false,
            done: None,
        };
        self.submit(request).await?;
        Ok(waiting)
    }
    async fn submit(&self, request: Request) -> Result<(), ModelError> {
        let permit = tokio::select! {
            ()=self.cancellation.cancelled()=>return Err(ModelError::Invalid("compilation cancelled".into())),
            permit=self.limit.clone().acquire_owned()=>permit.map_err(ModelError::codec)?,
        };
        // Serialize registration with drainage. No call is submitted after cancellation.
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|_| ModelError::Conflict("native call ownership"))?;
        self.cancellation.check()?;
        tasks.0.retain(|task| {
            !task.finished.load(Ordering::Acquire)
                || task
                    .joined
                    .clone()
                    .now_or_never()
                    .is_none_or(|result| result.is_err())
        });
        if tasks.0.len() == tasks.0.capacity() {
            tasks.1.grow(256)?;
            tasks.0.reserve_exact(1);
        }
        let native = self.native.clone();
        let cancellation = self.cancellation.clone();
        let finished = Arc::new(AtomicBool::new(false));
        let done = finished.clone();
        let task = tokio::spawn(
            async move {
                let mut submitted = CallGuard {
                    native,
                    cancellation,
                    finished: false,
                    done: Some(done),
                };
                let terminal = request.await;
                submitted.finished = true;
                drop(permit);
                terminal
            }
            .in_current_span()
            .with_current_subscriber(),
        );
        let joined = async move {
            task.await
                .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))?
        }
        .boxed()
        .shared();
        tasks.0.push(NativeCallTask { joined, finished });
        Ok(())
    }
    pub(crate) async fn drain(&self) -> Result<(), ModelError> {
        self.cancellation.cancel();
        // Shared join ownership stays registered if drainage itself is cancelled and retried.
        let tasks = self
            .tasks
            .lock()
            .map_err(|_| unfinished(ModelError::Conflict("native call ownership")))?
            .0
            .iter()
            .map(|task| task.joined.clone())
            .collect::<Vec<_>>();
        let mut completion = lctx_model::domain::completion::Completion::default();
        for task in tasks {
            completion.step(
                "native call join",
                task.await.map_err(ModelError::SharedCause),
            );
        }
        self.tasks
            .lock()
            .map_err(|_| unfinished(ModelError::Conflict("native call ownership")))?
            .0
            .retain(|task| {
                !task.finished.load(Ordering::Acquire)
                    || task
                        .joined
                        .clone()
                        .now_or_never()
                        .is_none_or(|result| result.is_err())
            });
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
}

fn fail(native: &NativeCompilerStore, cancellation: &Cancellation) {
    native.fail();
    cancellation.cancel();
}

fn unfinished(error: ModelError) -> ModelError {
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.local = lctx_model::domain::completion::LocalState::Outstanding;
    lctx_model::domain::completion::complete::<()>(Err(error), completion).unwrap_err()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn typed_acknowledgement_follows_failure_and_success_can_retire() {
        let native = crate::test_native::store();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = Cancellation::default();
        let calls = NativeCalls::new(native.clone(), cancellation.clone(), &budget);
        assert_eq!(calls.call(async { Ok(17u64) }).await.unwrap(), 17);
        assert_eq!(
            calls.call(async { Ok("second result") }).await.unwrap(),
            "second result"
        );
        let error = calls
            .call(async { Err::<(), _>(ModelError::Schema("primary operation failure")) })
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ModelError::Schema("primary operation failure")
        ));
        assert!(native.check().is_err());
        assert!(cancellation.check().is_err());
        calls.drain().await.unwrap();
    }
    #[tokio::test]
    async fn failed_native_join_is_retained_for_repeated_drain() {
        let native = crate::test_native::store();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let calls = NativeCalls::new(native.clone(), Cancellation::default(), &budget);
        assert!(
            calls
                .call(async {
                    panic!("injected native task panic");
                    #[allow(
                        unreachable_code,
                        reason = "the deliberate panic must retain a typed Result future"
                    )]
                    Ok::<(), ModelError>(())
                })
                .await
                .is_err()
        );
        assert!(native.check().is_err());
        for _ in 0..2 {
            let ModelError::Completion(outcome) = calls.drain().await.unwrap_err() else {
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
    #[tokio::test]
    async fn interrupted_native_drain_keeps_submitted_join() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let calls = Arc::new(NativeCalls::new(
            crate::test_native::store(),
            Cancellation::default(),
            &budget,
        ));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = calls.clone();
        let running = entered.clone();
        let finish = release.clone();
        let caller = tokio::spawn(async move {
            owner
                .call(async move {
                    running.notify_one();
                    finish.notified().await;
                    Ok(17)
                })
                .await
        });
        entered.notified().await;
        caller.abort();
        assert!(caller.await.is_err());
        let mut first = Box::pin(calls.drain());
        assert!(futures::poll!(&mut first).is_pending());
        drop(first);
        release.notify_one();
        calls.drain().await.unwrap();
        assert!(calls.tasks.lock().unwrap().0.is_empty());
    }
    #[tokio::test]
    async fn cancelled_async_request_keeps_complete_batch_charged_until_terminality() {
        use lctx_model::domain::{Batch, Record, input::Package};
        let model = lctx_model::domain::model().unwrap();
        let batch_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let task_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let batch = Batch::new(
            &model,
            vec![Package {
                name: "retained".into(),
            }],
            &batch_budget,
        )
        .unwrap();
        let charged = batch_budget.reserved();
        assert!(charged > 0);
        let calls = Arc::new(NativeCalls::new(
            crate::test_native::store(),
            Cancellation::default(),
            &task_budget,
        ));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = calls.clone();
        let running = entered.clone();
        let finish = release.clone();
        let caller = tokio::spawn(async move {
            owner
                .call(async move {
                    running.notify_one();
                    finish.notified().await;
                    assert_eq!(Package::decode(batch.arrow())?[0].name, "retained");
                    Ok(())
                })
                .await
        });
        entered.notified().await;
        caller.abort();
        assert!(caller.await.is_err());
        assert_eq!(batch_budget.reserved(), charged);
        release.notify_one();
        calls.drain().await.unwrap();
        assert_eq!(batch_budget.reserved(), 0);
    }
    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_sync_request_keeps_complete_batch_charged_until_terminality() {
        use lctx_model::domain::{Batch, Record, input::Package};
        let model = lctx_model::domain::model().unwrap();
        let batch_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let task_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let batch = Batch::new(
            &model,
            vec![Package {
                name: "retained".into(),
            }],
            &batch_budget,
        )
        .unwrap();
        let charged = batch_budget.reserved();
        assert!(charged > 0);
        let cancellation = Cancellation::default();
        let calls = Arc::new(NativeCalls::new(
            crate::test_native::store(),
            cancellation.clone(),
            &task_budget,
        ));
        let bridge =
            Arc::new(crate::native_bridge::NativeBridge::new(cancellation.clone()).unwrap());
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = bridge.clone();
        let running = entered.clone();
        let finish = release.clone();
        let call_owner = calls.clone();
        let caller = std::thread::spawn(move || {
            owner.call(async move {
                call_owner
                    .call(async move {
                        running.notify_one();
                        finish.notified().await;
                        assert_eq!(Package::decode(batch.arrow())?[0].name, "retained");
                        Ok(())
                    })
                    .await
            })
        });
        entered.notified().await;
        cancellation.cancel();
        assert!(caller.join().unwrap().is_err());
        assert_eq!(batch_budget.reserved(), charged);
        release.notify_one();
        bridge.drain().await.unwrap();
        calls.drain().await.unwrap();
        assert_eq!(batch_budget.reserved(), 0);
    }
    #[tokio::test]
    async fn cancelled_caller_retains_late_native_failure_in_completion() {
        let native = crate::test_native::store();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let calls = Arc::new(NativeCalls::new(native, Cancellation::default(), &budget));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = calls.clone();
        let running = entered.clone();
        let finish = release.clone();
        let caller = tokio::spawn(async move {
            owner
                .call(async move {
                    running.notify_one();
                    finish.notified().await;
                    Err::<(), _>(ModelError::Schema("late native failure"))
                })
                .await
        });
        entered.notified().await;
        caller.abort();
        assert!(caller.await.is_err());
        release.notify_one();
        let error = calls.drain().await.unwrap_err();
        let ModelError::Completion(outcome) = error else {
            panic!()
        };
        assert_eq!(
            outcome.completion.local,
            lctx_model::domain::completion::LocalState::Terminal
        );
        assert!(
            matches!(&outcome.completion.failures[0].error,ModelError::SharedCause(error) if matches!(error.as_ref(),ModelError::Schema("late native failure")))
        );
    }
    #[tokio::test]
    async fn cancelled_caller_stops_submissions_but_submitted_native_call_finishes() {
        let native = crate::test_native::store();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let cancellation = Cancellation::default();
        let calls = Arc::new(NativeCalls::new(
            native.clone(),
            cancellation.clone(),
            &budget,
        ));
        let entered = Arc::new(tokio::sync::Notify::new());
        let completed = Arc::new(AtomicBool::new(false));
        let call_owner = calls.clone();
        let call_entered = entered.clone();
        let call_completed = completed.clone();
        let caller = tokio::spawn(async move {
            call_owner
                .call(async move {
                    call_entered.notify_one();
                    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                    call_completed.store(true, Ordering::Release);
                    Ok(17)
                })
                .await
        });
        entered.notified().await;
        caller.abort();
        assert!(caller.await.is_err());
        calls.drain().await.unwrap();
        assert!(completed.load(Ordering::Acquire));
        assert!(cancellation.check().is_err());
        assert!(native.check().is_err());
        assert!(calls.call(async { Ok(18) }).await.is_err());
    }
}
