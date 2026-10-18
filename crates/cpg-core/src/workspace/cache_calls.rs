//! Independent configured cache effects use their actual cache session for native authority.
//! The compiler owns only acceptance, result delivery and the charged join through drainage.
use super::*;
use std::future::Future;

struct CacheWaiter {
    cancellation: Cancellation,
    finished: bool,
}
impl CacheWaiter {
    fn finish(mut self) {
        self.finished = true;
    }
}
impl Drop for CacheWaiter {
    fn drop(&mut self) {
        if !self.finished {
            self.cancellation.cancel();
        }
    }
}
impl Workspace {
    /// The supplied operation retains its genuine NativeProductCache session. No compiler
    /// target, installed view or compiler recovery scope is manufactured for cache effects.
    /// After acceptance, lost waiters cancel compiler admission but cannot discard the join
    /// or turn the operation's original refusal/unknown finality into an optional cache miss.
    #[inline(never)]
    pub(crate) fn cache_call<T: Send + 'static>(
        &self,
        work: impl Future<Output = Result<T, ModelError>> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, ModelError>> {
        // Erase before composing preparation/acknowledgement wrappers, retaining the
        // concrete future's real allocation grant until its actual body is destroyed.
        let charge = match self.budget.reserve(
            "configured cache operation body",
            std::mem::size_of_val(&work),
        ) {
            Ok(charge) => charge,
            Err(error) => return futures::future::ready(Err(error)).boxed(),
        };
        let work = lctx_native::continuation::retain(work.boxed(), charge);
        let cancellation = self.cancellation.clone();
        let body_cancellation = cancellation.clone();
        // The existing preparation owner retains actual stage joins and abandoned errors.
        // A single-consumer cell permits moving T out without imposing Sync on cache outputs.
        let accepted = self.spawn_preparation(async move {
            let result = work.await;
            if result.is_err() {
                body_cancellation.cancel();
            }
            result.map(|value| Mutex::new(Some(value)))
        });
        let waiter = CacheWaiter {
            cancellation,
            finished: accepted.is_err(),
        };
        async move {
            let result = match accepted {
                Ok(accepted) => match accepted.await {
                    Ok(value) => {
                        let mut value = value.lock().map_err(|_| poisoned())?;
                        value.take().ok_or(ModelError::Conflict(
                            "cache operation result already consumed",
                        ))
                    }
                    Err(error) => Err(ModelError::SharedCause(error)),
                },
                Err(error) => Err(error),
            };
            waiter.finish();
            result
        }
        .boxed()
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    #[tokio::test]
    async fn lost_cache_waiter_retains_actual_work_and_original_failure_until_join() {
        // This checks compiler task ownership only, not a fake successful cache operation.
        let workspace = Workspace::private(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 32 << 20,
                ..Default::default()
            },
        )
        .unwrap();
        let held = workspace
            .budget()
            .reserve("owned cache work control", 4096)
            .unwrap();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let terminal = Arc::new(AtomicBool::new(false));
        let observed = terminal.clone();
        let waiter = workspace.cache_call(async move {
            let _held = held;
            entered.send(()).unwrap();
            released.await.map_err(ModelError::codec)?;
            observed.store(true, Ordering::Release);
            Err::<(), _>(ModelError::Conflict("original accepted cache refusal"))
        });
        entering.await.unwrap();
        drop(waiter);
        assert!(workspace.cancellation.check().is_err());
        assert!(!terminal.load(Ordering::Acquire));
        assert!(workspace.budget().reserved() >= 4096);
        release.send(()).unwrap();
        let finality = workspace.drain_owned_report().await;
        assert!(terminal.load(Ordering::Acquire));
        let error = lctx_model::domain::completion::complete::<()>(Ok(()), finality).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("original accepted cache refusal")
        );
        assert!(workspace.native().is_err());
    }
    #[tokio::test]
    async fn lost_successful_cache_waiter_joins_settlement_and_retires_private_result() {
        // Native resources must be settled inside the accepted body. Its result
        // delivery may retain only ordinary private backing that can be dropped.
        struct SettledResult {
            _charge: Box<dyn Reservation>,
            retired: Arc<AtomicBool>,
        }
        impl Drop for SettledResult {
            fn drop(&mut self) {
                self.retired.store(true, Ordering::Release);
            }
        }
        let workspace = Workspace::private(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 32 << 20,
                ..Default::default()
            },
        )
        .unwrap();
        let charge = workspace
            .budget()
            .reserve("undelivered private cache result", 4096)
            .unwrap();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let settled = Arc::new(AtomicBool::new(false));
        let retired = Arc::new(AtomicBool::new(false));
        let observed_settlement = settled.clone();
        let observed_retirement = retired.clone();
        let waiter = workspace.cache_call(async move {
            entered.send(()).unwrap();
            released.await.map_err(ModelError::codec)?;
            observed_settlement.store(true, Ordering::Release);
            Ok(SettledResult {
                _charge: charge,
                retired: observed_retirement,
            })
        });
        entering.await.unwrap();
        drop(waiter);
        assert!(!settled.load(Ordering::Acquire));
        release.send(()).unwrap();
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.drain_owned_report().await,
        )
        .unwrap();
        assert!(settled.load(Ordering::Acquire));
        assert!(retired.load(Ordering::Acquire));
        assert!(workspace.native().is_err());
    }
    #[tokio::test]
    async fn consumed_cache_result_moves_without_retaining_its_payload_until_drain() {
        let workspace = Workspace::private(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 32 << 20,
                ..Default::default()
            },
        )
        .unwrap();
        let held = workspace
            .budget()
            .reserve("delivered cache work control", 4096)
            .unwrap();
        let held = workspace.cache_call(async move { Ok(held) }).await.unwrap();
        let before = workspace.budget().reserved();
        drop(held);
        assert_eq!(workspace.budget().reserved(), before - 4096);
        assert!(workspace.cancellation.check().is_ok());
        lctx_model::domain::completion::complete::<()>(
            Ok(()),
            workspace.drain_owned_report().await,
        )
        .unwrap();
    }
}
