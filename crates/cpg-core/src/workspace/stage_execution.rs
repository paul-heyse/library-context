//! Actual task finality is independent of coordinator result delivery.
use super::{OwnedTaskJoin, poisoned};
use futures::{FutureExt, future::BoxFuture};
use lctx_model::domain::{
    ModelError,
    charged::StateCharge,
    resources::{Reservation, ResourceBudget},
};
use lctx_tracing::InContext;
use std::{
    future::Future,
    sync::{Arc, Mutex},
};

pub(super) struct StageExecution(Arc<Mutex<Registry>>);
struct Registry {
    sealed: bool,
    tasks: Vec<(u64, OwnedTaskJoin)>,
    next_id: u64,
    charge: StateCharge,
}
/// A snapshot keeps the allocation charge until its final joined owner is released.
pub(super) struct StageDrainSnapshot {
    tasks: std::vec::IntoIter<OwnedTaskJoin>,
    _charge: Box<dyn Reservation>,
}
impl Iterator for StageDrainSnapshot {
    type Item = OwnedTaskJoin;
    fn next(&mut self) -> Option<Self::Item> {
        self.tasks.next()
    }
}
impl std::ops::Deref for StageDrainSnapshot {
    type Target = [OwnedTaskJoin];
    fn deref(&self) -> &Self::Target {
        self.tasks.as_slice()
    }
}
impl StageExecution {
    pub(super) fn new(budget: &ResourceBudget) -> Self {
        Self(Arc::new(Mutex::new(Registry {
            sealed: false,
            tasks: Vec::new(),
            next_id: 0,
            charge: StateCharge::new(budget, "compiler producer joins"),
        })))
    }
    pub(super) fn seal(&self) -> Result<StageDrainSnapshot, ModelError> {
        let mut registry = self.0.lock().map_err(|_| poisoned())?;
        registry.sealed = true;
        let charge = registry
            .charge
            .budget()
            .expect("stage join budget")
            .reserve(
                "compiler producer join snapshot",
                registry
                    .tasks
                    .len()
                    .saturating_mul(size_of::<OwnedTaskJoin>()),
            )?;
        Ok(StageDrainSnapshot {
            tasks: registry
                .tasks
                .iter()
                .map(|(_, join)| join.clone())
                .collect::<Vec<_>>()
                .into_iter(),
            _charge: charge,
        })
    }
    #[inline(never)]
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
            registry
                .charge
                .grow(additional.saturating_mul(size_of::<(u64, OwnedTaskJoin)>()))?;
            registry.tasks.reserve_exact(additional);
        }
        let registration = registry.next_id;
        registry.next_id = registration
            .checked_add(1)
            .ok_or(ModelError::Conflict("producer registration overflow"))?;
        // Charge the owned future/result channels before the runtime allocates them.
        // Producer inputs retain their separate semantic reservations.
        let task_charge = registry
            .charge
            .budget()
            .expect("stage join budget")
            .reserve(
                "compiler producer task",
                std::mem::size_of_val(&work).saturating_add(256),
            )?;
        let delivery_charge = Arc::new(
            registry
                .charge
                .budget()
                .expect("stage join budget")
                .reserve(
                    "compiler producer result delivery",
                    size_of::<T>().saturating_add(256),
                )?,
        );
        let receiver_charge = delivery_charge.clone();
        // Retain the concrete body allowance above, then erase before composing task-local
        // recovery/retry, tracing and runtime futures. Those wrappers otherwise move the
        // complete producer state repeatedly on the coordinator's stack at launch.
        let work: BoxFuture<'static, Result<T, ModelError>> = work.boxed();
        let context = lctx_tracing::Context::current();
        let retry = lctx_native::control::capture_retry_cancellation();
        let recovery = lctx_native::control::capture_recovery_scope();
        let (launch, launched) = tokio::sync::oneshot::channel();
        let (result, terminal) = tokio::sync::oneshot::channel();
        let task = async move {
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
            })
            .await;
            let _ = result.send(observed);
        }
        .with_tracing_context(context.clone());
        // Destroy the entire task (including an unpolled body and sender) before
        // releasing either grant; async capture/local ordering is not this contract.
        let task = tokio::spawn(lctx_native::continuation::retain(
            task,
            (task_charge, delivery_charge),
        ));
        let joined = async move {
            task.await
                .map_err(|error| std::sync::Arc::new(ModelError::Cause(Box::new(error))))
        }
        .with_tracing_context(context)
        .boxed()
        .shared();
        registry.tasks.push((registration, joined.clone()));
        let acknowledged_registry = self.0.clone();
        // No await under the registry mutex. A racing seal sees this exact joined owner.
        drop(registry);
        let _ = launch.send(());
        Ok(async move {
            // The actual queue/result reservation outlives both sender and receiver.
            // Once returned, the coordinator's already charged ReadyStages slot owns T.
            let _delivery_charge = receiver_charge;
            let result = terminal.await;
            let joined = joined.await.map_err(ModelError::SharedCause);
            // A consumed result plus successful actual join acknowledges this exact
            // registration. Lost waiters and failed joins remain available to drain.
            // Capacity follows peak live work, rather than all historical grain launches.
            let retired = if joined.is_ok() {
                acknowledged_registry
                    .lock()
                    .map_err(|_| poisoned())
                    .map(|mut registry| {
                        if let Some(index) = registry
                            .tasks
                            .iter()
                            .position(|(id, _)| *id == registration)
                        {
                            drop(registry.tasks.remove(index));
                        }
                    })
            } else {
                Ok(())
            };
            match result {
                Ok(observed) => {
                    let mut finality = lctx_model::domain::completion::Completion::default();
                    finality.step("producer task join", joined);
                    finality.step("producer acknowledged registration retirement", retired);
                    lctx_model::domain::completion::complete(observed, finality)
                }
                Err(error) => {
                    // A panic before delivery has no body outcome. Keep its structured
                    // join error primary, rather than the incidental channel refusal.
                    joined?;
                    retired?;
                    Err(ModelError::codec(error))
                }
            }
        }
        .boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[tokio::test]
    async fn acknowledged_windows_retire_join_metadata_without_losing_unobserved_work() {
        let budget = ResourceBudget::fixed(8192).unwrap();
        let tasks = StageExecution::new(&budget);
        let cancellation = super::super::Cancellation::default();
        let mut peak = None;
        for _ in 0..40 {
            let first = tasks.spawn(cancellation.clone(), async { Ok(1) }).unwrap();
            let second = tasks.spawn(cancellation.clone(), async { Ok(2) }).unwrap();
            assert_eq!(first.await.unwrap(), 1);
            assert_eq!(second.await.unwrap(), 2);
            assert!(tasks.0.lock().unwrap().tasks.is_empty());
            let bytes = budget.reserved();
            if let Some(expected) = peak {
                assert_eq!(bytes, expected);
            } else {
                peak = Some(bytes);
            }
        }
        let (release, released) = tokio::sync::oneshot::channel();
        let waiter = tasks
            .spawn(cancellation, async {
                released.await.map_err(ModelError::codec)?;
                Ok(())
            })
            .unwrap();
        drop(waiter);
        assert_eq!(tasks.0.lock().unwrap().tasks.len(), 1);
        release.send(()).unwrap();
        for joined in tasks.seal().unwrap() {
            joined.await.unwrap();
        }
        drop(tasks);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn synchronous_producer_does_not_suspend_ready_sibling() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let cancellation = super::super::Cancellation::default();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        let first = tasks
            .spawn(cancellation.clone(), async move {
                lctx_compiler_store::stage_runtime::borrowed_cpu("first", || {
                    entered.send(()).unwrap();
                    released.recv().unwrap();
                });
                Ok(1)
            })
            .unwrap();
        entering.await.unwrap();
        let second = tasks
            .spawn(cancellation, async move {
                release.send(()).unwrap();
                Ok(2)
            })
            .unwrap();
        // Producers progress independently of polling either coordinator result future.
        assert_eq!(second.await.unwrap(), 2);
        assert_eq!(first.await.unwrap(), 1);
        for join in tasks.seal().unwrap() {
            join.await.unwrap();
        }
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
        let first = tasks
            .spawn(cancellation.clone(), async move {
                entered.send(()).unwrap();
                released.await.map_err(ModelError::codec)?;
                observed.store(true, Ordering::Release);
                drop(charge);
                Ok(1)
            })
            .unwrap();
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
        for join in tasks.seal().unwrap() {
            join.await.unwrap();
        }
        assert!(completed.load(Ordering::Acquire));
        drop(joins);
        drop(tasks);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn queued_terminal_result_keeps_delivery_charge_after_actual_join() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let result = tasks
            .spawn(super::super::Cancellation::default(), async {
                Ok([0_u8; 512])
            })
            .unwrap();
        for join in tasks.seal().unwrap() {
            join.await.unwrap();
        }
        let registry_bytes = tasks.0.lock().unwrap().charge.accounted_bytes();
        assert_eq!(
            budget.reserved(),
            registry_bytes + 512 + 256,
            "actual task exit cannot release the unconsumed channel/result reservation"
        );
        drop(result);
        assert_eq!(budget.reserved(), registry_bytes);
        drop(tasks);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn panic_keeps_structured_join_error_for_result_and_drain() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let result = tasks
            .spawn::<()>(super::super::Cancellation::default(), async {
                panic!("producer panic control");
            })
            .unwrap();
        let error = result.await.unwrap_err();
        let ModelError::SharedCause(cause) = error else {
            panic!("structured shared join cause absent")
        };
        assert!(
            matches!(cause.as_ref(), ModelError::Cause(error) if error.downcast_ref::<tokio::task::JoinError>().is_some_and(tokio::task::JoinError::is_panic))
        );
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
        assert!(
            tasks
                .spawn(cancellation, async move {
                    observed.store(true, Ordering::Release);
                    Ok(())
                })
                .unwrap()
                .await
                .is_err()
        );
        assert!(!entered.load(Ordering::Acquire));
        for join in tasks.seal().unwrap() {
            join.await.unwrap();
        }
    }

    #[tokio::test]
    async fn large_producer_body_is_refused_before_launch() {
        let budget = ResourceBudget::fixed(48 << 10).unwrap();
        let tasks = StageExecution::new(&budget);
        let entered = Arc::new(AtomicBool::new(false));
        let observed = entered.clone();
        let payload = std::hint::black_box([7_u8; 64 << 10]);
        let result = tasks.spawn(super::super::Cancellation::default(), async move {
            observed.store(true, Ordering::Release);
            tokio::task::yield_now().await;
            Ok(std::hint::black_box(payload)
                .iter()
                .map(|byte| u64::from(*byte))
                .sum::<u64>())
        });
        assert!(matches!(
            result,
            Err(ModelError::Resource {
                owner: "compiler producer task",
                ..
            })
        ));
        assert!(!entered.load(Ordering::Acquire));
        assert!(tasks.seal().unwrap().is_empty());
        drop(tasks);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn large_producer_body_stays_charged_through_owned_cleanup() {
        const BODY_BYTES: usize = 128 << 10;
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let tasks = StageExecution::new(&budget);
        let (entered, entering) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let (cleanup, cleaning) = tokio::sync::oneshot::channel();
        let (finish, finished) = tokio::sync::oneshot::channel();
        let completed = Arc::new(AtomicBool::new(false));
        let observed = completed.clone();
        let payload = std::hint::black_box([7_u8; BODY_BYTES]);
        let result = tasks
            .spawn(super::super::Cancellation::default(), async move {
                entered.send(()).unwrap();
                released.await.map_err(ModelError::codec)?;
                cleanup.send(()).unwrap();
                finished.await.map_err(ModelError::codec)?;
                let checksum = std::hint::black_box(payload)
                    .iter()
                    .map(|byte| u64::from(*byte))
                    .sum::<u64>();
                assert_eq!(checksum, 7 * BODY_BYTES as u64);
                observed.store(true, Ordering::Release);
                Ok(checksum)
            })
            .unwrap();
        entering.await.unwrap();
        assert!(budget.reserved() >= BODY_BYTES);
        drop(result);
        release.send(()).unwrap();
        cleaning.await.unwrap();
        assert!(!completed.load(Ordering::Acquire));
        assert!(
            budget.reserved() >= BODY_BYTES,
            "dropping coordinator delivery cannot release a producer still cleaning up"
        );
        finish.send(()).unwrap();
        for join in tasks.seal().unwrap() {
            join.await.unwrap();
        }
        assert!(completed.load(Ordering::Acquire));
        assert!(
            budget.reserved() < BODY_BYTES,
            "the concrete body allocation is released after actual task finality"
        );
        drop(tasks);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn unpolled_runtime_shutdown_destroys_body_before_releasing_its_grant() {
        use std::sync::atomic::AtomicUsize;
        struct ObserveDrop {
            budget: ResourceBudget,
            observed: Arc<AtomicUsize>,
        }
        impl Drop for ObserveDrop {
            fn drop(&mut self) {
                self.observed
                    .store(self.budget.reserved(), Ordering::Release);
            }
        }
        let budget = ResourceBudget::fixed(4096).unwrap();
        let tasks = StageExecution::new(&budget);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let observed = Arc::new(AtomicUsize::new(0));
        let polled = Arc::new(AtomicBool::new(false));
        let entered = polled.clone();
        let payload = ObserveDrop {
            budget: budget.clone(),
            observed: observed.clone(),
        };
        let result = {
            let _runtime = runtime.enter();
            tasks
                .spawn(super::super::Cancellation::default(), async move {
                    entered.store(true, Ordering::Release);
                    drop(payload);
                    Ok(())
                })
                .unwrap()
        };
        let retained = budget.reserved();
        drop(runtime);
        assert!(!polled.load(Ordering::Acquire));
        assert_eq!(
            observed.load(Ordering::Acquire),
            retained,
            "an unpolled producer's destructor must still have its full task reservation"
        );
        assert!(futures::executor::block_on(result).is_err());
        for join in tasks.seal().unwrap() {
            assert!(futures::executor::block_on(join).is_err());
        }
        drop(tasks);
        assert_eq!(budget.reserved(), 0);
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
        })
        .await
        .unwrap();
        retry.cancel();
        assert!(result.await.unwrap().stopped());
        for join in tasks.seal().unwrap() {
            join.await.unwrap();
        }
    }
}
