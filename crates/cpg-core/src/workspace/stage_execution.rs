//! Actual task finality is independent of coordinator result delivery.
use super::{OwnedTaskJoin, poisoned};
use futures::{FutureExt, future::BoxFuture};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use lctx_tracing::InContext;
use std::{future::Future, sync::Mutex};

pub(super) struct StageExecution(Mutex<Registry>);
struct Registry {
    sealed: bool,
    tasks: Vec<OwnedTaskJoin>,
    charge: StateCharge,
}
impl StageExecution {
    pub(super) fn new(budget: &ResourceBudget) -> Self {
        Self(Mutex::new(Registry {
            sealed: false,
            tasks: Vec::new(),
            charge: StateCharge::new(budget, "compiler producer joins"),
        }))
    }
    pub(super) fn seal(&self) -> Result<Vec<OwnedTaskJoin>, ModelError> {
        let mut registry = self.0.lock().map_err(|_| poisoned())?;
        registry.sealed = true;
        Ok(registry.tasks.clone())
    }
    pub(super) fn spawn<T: Send + 'static>(
        &self,
        cancellation: super::Cancellation,
        work: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> Result<BoxFuture<'static, Result<T, ModelError>>, ModelError> {
        let mut registry = self.0.lock().map_err(|_| poisoned())?;
        if registry.sealed {
            return Err(ModelError::Conflict("compiler producer admission sealed"));
        }
        if registry.tasks.len() == registry.tasks.capacity() {
            let additional = registry.tasks.capacity().max(4);
            registry.charge.grow(additional.saturating_mul(size_of::<OwnedTaskJoin>()))?;
            registry.tasks.reserve_exact(additional);
        }
        // Charge the owned future/result channels before the runtime allocates them.
        // Producer inputs retain their separate semantic reservations.
        let task_charge = registry.charge.budget().expect("stage join budget").reserve(
            "compiler producer task",
            std::mem::size_of_val(&work).saturating_add(size_of::<T>()).saturating_add(256),
        )?;
        let context = lctx_tracing::Context::current();
        let retry = lctx_native::control::capture_retry_cancellation();
        let recovery = lctx_native::control::capture_recovery_scope();
        let (launch, launched) = tokio::sync::oneshot::channel();
        let (result, terminal) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _task_charge = task_charge;
            let run = async move {
                // The actual JoinHandle is registered before useful execution is admitted.
                launched.await.map_err(ModelError::codec)?;
                cancellation.check()?;
                work.await
            };
            let observed = lctx_native::control::with_retry_context(retry, async move {
                match recovery {
                    Some(scope) => lctx_native::control::with_recovery_scope(scope, run).await,
                    None => run.await,
                }
            }).await;
            let _ = result.send(observed);
        }.with_tracing_context(context.clone()));
        let joined = async move {
            task.await.map_err(|error| std::sync::Arc::new(ModelError::Cause(Box::new(error))))
        }.with_tracing_context(context).boxed().shared();
        registry.tasks.push(joined.clone());
        // No await under the registry mutex. A racing seal sees this exact joined owner.
        drop(registry);
        let _ = launch.send(());
        Ok(async move {
            let result = terminal.await;
            joined.await.map_err(ModelError::SharedCause)?;
            result.map_err(ModelError::codec)?
        }.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    #[tokio::test(flavor = "multi_thread")]
    async fn synchronous_producer_does_not_suspend_ready_sibling() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let cancellation = super::super::Cancellation::default();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        let first = tasks.spawn(cancellation.clone(), async move {
            lctx_compiler_store::stage_runtime::borrowed_cpu("first", || {
                entered.send(()).unwrap();
                released.recv().unwrap();
            });
            Ok(1)
        }).unwrap();
        entering.await.unwrap();
        let second = tasks.spawn(cancellation, async move {
            release.send(()).unwrap();
            Ok(2)
        }).unwrap();
        // Producers progress independently of polling either coordinator result future.
        assert_eq!(second.await.unwrap(), 2);
        assert_eq!(first.await.unwrap(), 1);
        for join in tasks.seal().unwrap() { join.await.unwrap(); }
    }

    #[tokio::test]
    async fn current_thread_delivery_and_interrupted_drain_keep_actual_join() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let cancellation = super::super::Cancellation::default();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let completed = Arc::new(AtomicBool::new(false));
        let observed = completed.clone();
        let charge = budget.reserve("producer input", 256).unwrap();
        let first = tasks.spawn(cancellation.clone(), async move {
            entered.send(()).unwrap();
            released.await.map_err(ModelError::codec)?;
            observed.store(true, Ordering::Release);
            drop(charge);
            Ok(1)
        }).unwrap();
        entering.await.unwrap();
        let second = tasks.spawn(cancellation.clone(), async { Ok(2) }).unwrap();
        assert_eq!(second.await.unwrap(), 2);
        drop(first); // Dropped coordinator result ownership does not drop actual task join.
        let joins = tasks.seal().unwrap();
        assert!(tasks.spawn(cancellation, async { Ok(3) }).is_err());
        let mut interrupted = joins[0].clone();
        assert!(futures::poll!(&mut interrupted).is_pending());
        drop(interrupted);
        assert!(!completed.load(Ordering::Acquire));
        assert!(budget.reserved() >= 256);
        release.send(()).unwrap();
        for join in tasks.seal().unwrap() { join.await.unwrap(); }
        assert!(completed.load(Ordering::Acquire));
        drop(tasks);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn panic_keeps_structured_join_error_for_result_and_drain() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let result = tasks.spawn::<()>(super::super::Cancellation::default(), async {
            panic!("producer panic control");
        }).unwrap();
        let error = result.await.unwrap_err();
        let ModelError::SharedCause(cause) = error else { panic!("structured shared join cause absent") };
        assert!(matches!(cause.as_ref(), ModelError::Cause(error) if error.downcast_ref::<tokio::task::JoinError>().is_some_and(tokio::task::JoinError::is_panic)));
        assert!(tasks.seal().unwrap()[0].clone().await.is_err());
    }

    #[tokio::test]
    async fn cancellation_before_first_poll_never_enters_work() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let cancellation = super::super::Cancellation::default();
        cancellation.cancel();
        let entered = Arc::new(AtomicBool::new(false));
        let observed = entered.clone();
        assert!(tasks.spawn(cancellation, async move {
            observed.store(true, Ordering::Release);
            Ok(())
        }).unwrap().await.is_err());
        assert!(!entered.load(Ordering::Acquire));
        for join in tasks.seal().unwrap() { join.await.unwrap(); }
    }

    #[tokio::test]
    async fn task_rebinds_captured_retry_context() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let retry = lctx_native::control::RetryCancellation::default();
        let result = lctx_native::control::with_retry_cancellation(retry.clone(), async {
            tasks.spawn(super::super::Cancellation::default(), async {
                Ok(lctx_native::control::current_retry_cancellation().unwrap())
            })
        }).await.unwrap();
        retry.cancel();
        assert!(result.await.unwrap().stopped());
        for join in tasks.seal().unwrap() { join.await.unwrap(); }
    }
}
