//! Cooperative retry stop never aborts an in-flight transaction or its reconciliation.
use super::*;
use std::{future::Future, sync::atomic::{AtomicBool, AtomicU64, Ordering}, time::Duration};

#[derive(Default)]
struct State { stopped: AtomicBool, changed: tokio::sync::Notify }
#[derive(Clone, Default)]
pub struct RetryCancellation(Arc<State>);
impl RetryCancellation {
    pub fn cancel(&self) {
        self.0.stopped.store(true, Ordering::Release);
        self.0.changed.notify_waiters();
    }
    pub fn stopped(&self) -> bool { self.0.stopped.load(Ordering::Acquire) }
    pub async fn cancelled(&self) {
        loop {
            let changed = self.0.changed.notified(); tokio::pin!(changed); changed.as_mut().enable();
            if self.stopped() { return; }
            changed.await;
        }
    }
}
#[derive(Clone, Default)]
pub struct RetryCancellationContext(Vec<RetryCancellation>);
tokio::task_local! { static CANCELLATION: RetryCancellationContext; }
pub fn capture_retry_cancellation() -> RetryCancellationContext { CANCELLATION.try_with(Clone::clone).unwrap_or_default() }
pub fn current_retry_cancellation() -> Option<RetryCancellation> { capture_retry_cancellation().0.last().cloned() }
pub async fn with_retry_context<T>(context: RetryCancellationContext, future: impl Future<Output=T>) -> T {
    CANCELLATION.scope(context, future).await
}
pub async fn with_retry_cancellation<T>(cancellation: RetryCancellation, future: impl Future<Output=T>) -> T {
    let mut context = capture_retry_cancellation();
    if !context.0.iter().any(|old| Arc::ptr_eq(&old.0, &cancellation.0)) { context.0.push(cancellation); }
    with_retry_context(context, future).await
}
fn delay(attempt: u32, sequence: u64) -> Duration {
    let ceiling = (1u64 << attempt.min(8)).min(250);
    // Per-operation sequence decorrelates callers; jitter never changes their request.
    let mixed = sequence.wrapping_mul(0x9e3779b97f4a7c15).rotate_left(attempt % 64);
    Duration::from_micros(ceiling * 500 + mixed % (ceiling * 500 + 1))
}
pub(super) async fn transaction(client: &Surreal<Client>, sql: &str, bindings: Variables,
    recover: bool) -> Result<(), ModelError> {
    let cancellation = if recover { RetryCancellationContext::default() } else { capture_retry_cancellation() };
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut retries = 0u32;
    loop {
        if cancellation.0.iter().any(RetryCancellation::stopped) {
            return Err(ModelError::Invalid("native transaction retry cancelled".into()));
        }
        // Await actual response/terminal before observing cancellation. Dropping delivery
        // or a retry wait is never used as evidence that this effect did not commit.
        let result = match client.query(sql.to_owned()).bind(bindings.clone()).await {
            Ok(response) => checked_transaction(response).map(|_| ()),
            Err(error) => Err(crate::loader::write_failure(error)),
        };
        let Err(error) = result else { return Ok(()); };
        if !retryable_transaction(&error) { return Err(error); }
        let wait = delay(retries, sequence);
        retries = retries.saturating_add(1);
        tracing::debug!(target:"lctx_phase", retries, delay_us=wait.as_micros(), recovery=recover, "native transaction conflict; retaining original request");
        let stopped = if !cancellation.0.is_empty() {
            let stops = cancellation.0.iter().map(|stop| Box::pin(stop.cancelled())).collect::<Vec<_>>();
            tokio::select! { _=futures::future::select_all(stops)=>true, ()=tokio::time::sleep(wait)=>false }
        } else { tokio::time::sleep(wait).await; false };
        if stopped {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step("native transaction retry cancellation", Err(ModelError::Invalid("native transaction retry cancelled".into())));
            return lctx_model::domain::completion::complete(Err(error), completion);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn already_cancelled_retry_refuses_without_contacting_native() {
        let cancellation = RetryCancellation::default(); cancellation.cancel();
        let result = with_retry_cancellation(cancellation,
            transaction(&Surreal::init(), "RETURN true", Variables::new(), false)).await;
        assert!(matches!(result, Err(ModelError::Invalid(message)) if message == "native transaction retry cancelled"));
    }
    #[tokio::test]
    async fn nested_scopes_and_spawned_context_retain_every_cancellation_owner() {
        let outer = RetryCancellation::default();
        let inner = RetryCancellation::default();
        let stopped = outer.clone();
        let context = with_retry_cancellation(outer, with_retry_cancellation(inner,
            async { capture_retry_cancellation() })).await;
        assert_eq!(context.0.len(), 2);
        stopped.cancel();
        let result = tokio::spawn(with_retry_context(context, async {
            transaction(&Surreal::init(), "RETURN true", Variables::new(), false).await
        })).await.unwrap();
        assert!(matches!(result, Err(ModelError::Invalid(message)) if message == "native transaction retry cancelled"));
    }
    #[tokio::test]
    async fn cancellation_interrupts_backoff_without_losing_notification() {
        let cancellation = RetryCancellation::default();
        let waiting = cancellation.clone();
        let task = tokio::spawn(async move { waiting.cancelled().await });
        cancellation.cancel(); task.await.unwrap();
        cancellation.cancelled().await;
    }
    #[test]
    fn backoff_is_nonzero_jittered_and_bounded_without_an_attempt_limit() {
        for attempt in [0, 1, 8, 128, u32::MAX] {
            for sequence in 0..32 {
                let wait = delay(attempt, sequence);
                assert!(wait >= Duration::from_micros(500));
                assert!(wait <= Duration::from_millis(250));
            }
        }
        assert_ne!(delay(4, 1), delay(4, 2));
    }
}
