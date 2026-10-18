//! Submitted asynchronous native calls retain task ownership through caller cancellation.
use crate::admission::Cancellation;
use crate::compiler::NativeCompilerStore;
use crate::dispatcher_completion::OperationResult;
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use lctx_tracing::InContext;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::oneshot;

pub struct NativeCalls {
    native: Arc<NativeCompilerStore>,
    cancellation: Cancellation,
    budget: ResourceBudget,
    tasks: Mutex<(Vec<NativeCallTask>, StateCharge)>,
}
type Request = BoxFuture<'static, OperationResult>;
struct NativeCallTask {
    joined: Shared<BoxFuture<'static, Result<OperationResult, Arc<ModelError>>>>,
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
    pub fn new(
        native: Arc<NativeCompilerStore>,
        cancellation: Cancellation,
        budget: &ResourceBudget,
    ) -> Self {
        Self {
            native,
            cancellation,
            budget: budget.clone(),
            tasks: Mutex::new((
                Vec::new(),
                StateCharge::new(budget, "native-call-ownership"),
            )),
        }
    }
    /// Erase the complete operation before entering task admission or Tokio scheduling.
    pub fn call<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        let future = match lctx_native::continuation::admit(
            &self.budget,
            "native-call operation body",
            future,
        ) {
            Ok(future) => future,
            Err(error) => return futures::future::ready(Err(error)).boxed(),
        };
        self.call_boxed(future)
    }
    /// Keep the typed acknowledgement adapter independent of the concrete operation future.
    pub fn call_boxed<T: Send + 'static>(
        &self,
        future: BoxFuture<'static, Result<T, ModelError>>,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        let (answer, acknowledgement) = oneshot::channel();
        let native = self.native.clone();
        let cancellation = self.cancellation.clone();
        let budget = self.budget.clone();
        let owner = tracing::info_span!(target: "lctx_phase", "native_call", attempt=%native.attempt().hex());
        // Only the result channel is typed; submitted ownership and scheduler work are shared.
        let request: Request = async move {
            let result = match native.bind_recovery_budget(&budget) {
                Ok(recovery) => {
                    lctx_native::control::with_recovery_scope(
                        recovery,
                        lctx_native::control::with_retry_cancellation(
                            cancellation.native_retry(),
                            future,
                        ),
                    )
                    .await
                }
                Err(error) => Err(error),
            };
            if result.is_err() {
                fail(&native, &cancellation);
            }
            OperationResult::deliver(answer, result)
        }
        .with_tracing_span(owner.clone())
        .boxed();
        async move {
            let mut waiting = self.submit_guarded(request).await?;
            let result = lctx_native::phase::wait("native_call_acknowledgement", acknowledgement)
                .await
                .map_err(|_| {
                    ModelError::Invalid("native call ended without acknowledgement".into())
                })?;
            waiting.finished = true;
            result
        }
        .with_tracing_span(owner)
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
        self.cancellation.check()?;
        // Real allocation admission bounds accepted work. A fixed call-count
        // semaphore unnecessarily serialized independent complete producers.
        let accepted = self.budget.reserve("native-call accepted task", 4096)?;
        // Serialize registration with drainage. No call is submitted after cancellation.
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|_| ModelError::Conflict("native call ownership"))?;
        self.cancellation.check()?;
        tasks.0.retain(|task| {
            !task.finished.load(Ordering::Acquire)
                || task.joined.clone().now_or_never().is_none_or(|result| {
                    result.is_err() || result.is_ok_and(|operation| operation.retained())
                })
        });
        if tasks.0.len() == tasks.0.capacity() {
            tasks.1.grow(256)?;
            tasks.0.reserve_exact(1);
        }
        let native = self.native.clone();
        let cancellation = self.cancellation.clone();
        let finished = Arc::new(AtomicBool::new(false));
        let done = finished.clone();
        // A caller may be polling on the provider bridge's short-lived runtime.
        // Registered calls must retain the store owner's executor through drainage.
        let task = self.native.runtime().spawn(
            async move {
                let mut submitted = CallGuard {
                    native,
                    cancellation,
                    finished: false,
                    done: Some(done),
                };
                let terminal = request.await;
                submitted.finished = true;
                drop(accepted);
                terminal
            }
            .in_tracing_context(),
        );
        let joined = async move {
            task.await
                .map_err(|error| Arc::new(ModelError::Cause(Box::new(error))))
        }
        .boxed()
        .shared();
        tasks.0.push(NativeCallTask { joined, finished });
        Ok(())
    }
    pub async fn drain(&self) -> Result<(), ModelError> {
        let mut completion = self.drain_finality_report().await;
        self.append_operation_failures(&mut completion);
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
    /// Actual joins and nested owner outcomes; acknowledged refusals are diagnostics.
    pub async fn drain_finality_report(&self) -> lctx_model::domain::completion::Completion {
        self.cancellation.cancel();
        // Shared join ownership stays registered if drainage itself is cancelled and retried.
        let mut completion = lctx_model::domain::completion::Completion::default();
        let tasks = match self.tasks.lock() {
            Ok(tasks) => tasks
                .0
                .iter()
                .map(|task| task.joined.clone())
                .collect::<Vec<_>>(),
            Err(_) => {
                completion.step(
                    "native call ownership",
                    Err(unfinished(ModelError::Conflict("native call ownership"))),
                );
                return completion;
            }
        };
        for task in tasks {
            match lctx_native::phase::wait("native_call_join", task).await {
                Ok(operation) => operation.finality(&mut completion),
                Err(error) => {
                    completion.step("native call join", Err(ModelError::SharedCause(error)))
                }
            }
        }
        match self.tasks.lock() {
            Ok(mut tasks) => tasks.0.retain(|task| {
                !task.finished.load(Ordering::Acquire)
                    || task.joined.clone().now_or_never().is_none_or(|result| {
                        result.is_err() || result.is_ok_and(|operation| operation.retained())
                    })
            }),
            Err(_) => completion.step(
                "native call ownership",
                Err(unfinished(ModelError::Conflict("native call ownership"))),
            ),
        }
        completion
    }
    pub fn append_operation_failures(
        &self,
        completion: &mut lctx_model::domain::completion::Completion,
    ) {
        if let Ok(tasks) = self.tasks.lock() {
            for task in &tasks.0 {
                if let Some(Ok(operation)) = task.joined.clone().now_or_never() {
                    operation.append_diagnostics("native call operation", completion);
                }
            }
        }
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
    async fn resource_admitted_native_work_does_not_wait_for_a_fixed_call_count() {
        // Isolate actual accepted-task ownership; no native result is fabricated.
        let native = NativeCompilerStore::from_existing(
            Arc::new(surrealdb::Surreal::init()),
            lctx_model::domain::serving::Name::new("call_control").unwrap(),
            lctx_model::domain::serving::Name::new("resource_control").unwrap(),
        );
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let calls = NativeCalls::new(native.clone(), Cancellation::default(), &budget);
        let mut guards = Vec::new();
        let mut releases = Vec::new();
        let mut entries = Vec::new();
        for _ in 0..16 {
            let (entered, entry) = oneshot::channel();
            let (release, released) = oneshot::channel();
            guards.push(
                calls
                    .submit_guarded(
                        async move {
                            entered.send(()).unwrap();
                            released.await.unwrap();
                            OperationResult::default()
                        }
                        .boxed(),
                    )
                    .await
                    .unwrap(),
            );
            entries.push(entry);
            releases.push(release);
        }
        for entry in entries {
            entry.await.unwrap();
        }
        assert!(
            budget.reserved() >= 16 * 4096,
            "accepted bodies retain real allocation grants"
        );
        for release in releases {
            release.send(()).unwrap();
        }
        for guard in &mut guards {
            guard.finished = true;
        }
        drop(guards);
        assert!(calls.drain_finality_report().await.is_satisfied());
        assert!(native.drain_finality_report().await.is_satisfied());
        drop(calls);
        drop(native);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn dropped_waiter_and_submitting_runtime_keep_registered_call_alive() {
        // No native operation: this isolates actual task/executor ownership.
        let native = NativeCompilerStore::from_existing(
            Arc::new(surrealdb::Surreal::init()),
            lctx_model::domain::serving::Name::new("call_control").unwrap(),
            lctx_model::domain::serving::Name::new("owned_control").unwrap(),
        );
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let calls = Arc::new(NativeCalls::new(
            native.clone(),
            Cancellation::default(),
            &budget,
        ));
        let (entered, entering) = oneshot::channel();
        let (release, released) = oneshot::channel();
        let completed = Arc::new(AtomicBool::new(false));
        let observed = completed.clone();
        let retained = budget.reserve("cross-runtime-call", 256).unwrap();
        let request: Request = async move {
            let _retained = retained;
            entered.send(()).unwrap();
            released.await.unwrap();
            observed.store(true, Ordering::Release);
            OperationResult::default()
        }
        .boxed();
        let submitting = calls.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let waiter = submitting.submit_guarded(request).await.unwrap();
                drop(waiter); // Cancels caller admission; already submitted work still owns its join.
            });
            // This private executor ends before the operation is allowed to finish.
        })
        .join()
        .unwrap();
        entering.await.unwrap();
        assert!(!completed.load(Ordering::Acquire));
        assert!(budget.reserved() >= 256);
        release.send(()).unwrap();
        assert!(calls.drain_finality_report().await.is_satisfied());
        assert!(completed.load(Ordering::Acquire));
        assert!(native.drain_finality_report().await.is_satisfied());
        drop(calls);
        drop(native);
        assert_eq!(budget.reserved(), 0);
    }
    async fn close_fixture(calls: &NativeCalls) {
        let finality = calls.native.abandon_finality_report().await;
        assert_eq!(
            finality.local,
            lctx_model::domain::completion::LocalState::Terminal
        );
        assert_eq!(
            finality.remote,
            lctx_model::domain::completion::RemoteState::Confirmed
        );
        assert!(
            finality.failures.is_empty(),
            "actual native fixture must close independently of the injected dispatcher result"
        );
    }
    #[tokio::test]
    async fn typed_acknowledgement_follows_failure_and_success_can_retire() {
        let native = crate::test_native::store();
        let budget = native.recovery_scope().unwrap().budget().clone();
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
        close_fixture(&calls).await;
    }
    #[tokio::test]
    async fn failed_native_join_is_retained_for_repeated_drain() {
        let native = crate::test_native::store();
        let budget = native.recovery_scope().unwrap().budget().clone();
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
        close_fixture(&calls).await;
    }
    #[tokio::test]
    async fn interrupted_native_drain_keeps_submitted_join() {
        let native = crate::test_native::store();
        let budget = native.recovery_scope().unwrap().budget().clone();
        let calls = Arc::new(NativeCalls::new(native, Cancellation::default(), &budget));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = calls.clone();
        let running = entered.clone();
        let finish = release.clone();
        let mut caller = tokio::spawn(async move {
            owner
                .call(async move {
                    running.notify_one();
                    finish.notified().await;
                    Ok(17)
                })
                .await
        });
        tokio::select! {
            () = entered.notified() => {},
            result = &mut caller => panic!("native caller returned before entering the submitted request: {result:?}"),
        }
        caller.abort();
        assert!(caller.await.is_err());
        let mut first = Box::pin(calls.drain());
        assert!(futures::poll!(&mut first).is_pending());
        drop(first);
        release.notify_one();
        calls.drain().await.unwrap();
        assert!(calls.tasks.lock().unwrap().0.is_empty());
        close_fixture(&calls).await;
    }
    #[tokio::test]
    async fn cancelled_async_request_keeps_complete_batch_charged_until_terminality() {
        use lctx_model::domain::{Batch, Record, input::Package};
        let model = lctx_model::domain::model().unwrap();
        let batch_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let native = crate::test_native::store();
        let task_budget = native.recovery_scope().unwrap().budget().clone();
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
            native,
            Cancellation::default(),
            &task_budget,
        ));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = calls.clone();
        let running = entered.clone();
        let finish = release.clone();
        let mut caller = tokio::spawn(async move {
            owner
                .call(async move {
                    running.notify_one();
                    finish.notified().await;
                    assert_eq!(Package::decode(batch.arrow())?[0].name, "retained");
                    Ok(())
                })
                .await
        });
        tokio::select! {
            () = entered.notified() => {},
            result = &mut caller => panic!("native caller returned before entering the submitted request: {result:?}"),
        }
        caller.abort();
        assert!(caller.await.is_err());
        assert_eq!(batch_budget.reserved(), charged);
        release.notify_one();
        calls.drain().await.unwrap();
        assert_eq!(batch_budget.reserved(), 0);
        close_fixture(&calls).await;
    }
    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_sync_request_keeps_complete_batch_charged_until_terminality() {
        use lctx_model::domain::{Batch, Record, input::Package};
        let model = lctx_model::domain::model().unwrap();
        let batch_budget = ResourceBudget::fixed(1 << 20).unwrap();
        let native = crate::test_native::store();
        let task_budget = native.recovery_scope().unwrap().budget().clone();
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
        let calls = Arc::new(NativeCalls::new(native, cancellation.clone(), &task_budget));
        let bridge =
            Arc::new(crate::native_bridge::NativeBridge::new(cancellation.clone()).unwrap());
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = bridge.clone();
        let running = entered.clone();
        let finish = release.clone();
        let call_owner = calls.clone();
        let (returned, mut premature_return) = tokio::sync::oneshot::channel();
        let caller = std::thread::spawn(move || {
            let result = owner.call(async move {
                call_owner
                    .call(async move {
                        running.notify_one();
                        finish.notified().await;
                        assert_eq!(Package::decode(batch.arrow())?[0].name, "retained");
                        Ok(())
                    })
                    .await
            });
            let _ = returned.send(());
            result
        });
        tokio::select! {
            () = entered.notified() => {},
            _ = &mut premature_return => panic!("sync native caller returned before entering the submitted request: {:?}", caller.join()),
        }
        cancellation.cancel();
        assert!(caller.join().unwrap().is_err());
        assert_eq!(batch_budget.reserved(), charged);
        release.notify_one();
        bridge.drain().await.unwrap();
        calls.drain().await.unwrap();
        assert_eq!(batch_budget.reserved(), 0);
        close_fixture(&calls).await;
    }
    #[tokio::test]
    async fn cancelled_caller_retains_late_native_failure_in_completion() {
        let native = crate::test_native::store();
        let budget = native.recovery_scope().unwrap().budget().clone();
        let calls = Arc::new(NativeCalls::new(native, Cancellation::default(), &budget));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let owner = calls.clone();
        let running = entered.clone();
        let finish = release.clone();
        let mut caller = tokio::spawn(async move {
            owner
                .call(async move {
                    running.notify_one();
                    finish.notified().await;
                    Err::<(), _>(ModelError::Schema("late native failure"))
                })
                .await
        });
        tokio::select! {
            () = entered.notified() => {},
            result = &mut caller => panic!("native caller returned before entering the submitted request: {result:?}"),
        }
        caller.abort();
        assert!(caller.await.is_err());
        release.notify_one();
        let finality = calls.drain_finality_report().await;
        assert!(
            finality.failures.is_empty(),
            "late acknowledged refusal is not failed ownership"
        );
        assert_eq!(
            finality.remote,
            lctx_model::domain::completion::RemoteState::Confirmed
        );
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
        close_fixture(&calls).await;
    }
    #[tokio::test]
    async fn cancelled_caller_stops_submissions_but_submitted_native_call_finishes() {
        let native = crate::test_native::store();
        let budget = native.recovery_scope().unwrap().budget().clone();
        let cancellation = Cancellation::default();
        let calls = Arc::new(NativeCalls::new(
            native.clone(),
            cancellation.clone(),
            &budget,
        ));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let completed = Arc::new(AtomicBool::new(false));
        let call_owner = calls.clone();
        let call_entered = entered.clone();
        let call_completed = completed.clone();
        let finish = release.clone();
        let mut caller = tokio::spawn(async move {
            call_owner
                .call(async move {
                    call_entered.notify_one();
                    finish.notified().await;
                    call_completed.store(true, Ordering::Release);
                    Ok(17)
                })
                .await
        });
        tokio::select! {
            () = entered.notified() => {},
            result = &mut caller => panic!("native caller returned before entering the submitted request: {result:?}"),
        }
        caller.abort();
        assert!(caller.await.is_err());
        assert!(!completed.load(Ordering::Acquire));
        release.notify_one();
        calls.drain().await.unwrap();
        assert!(completed.load(Ordering::Acquire));
        assert!(cancellation.check().is_err());
        assert!(native.check().is_err());
        assert!(calls.call(async { Ok(18) }).await.is_err());
        close_fixture(&calls).await;
    }
}
