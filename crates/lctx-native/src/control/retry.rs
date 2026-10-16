//! Cooperative retry stop never aborts an in-flight transaction or its reconciliation.
use super::*;
use std::{
    future::Future,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

#[derive(Default)]
struct State {
    stopped: Arc<AtomicBool>,
    changed: tokio::sync::Notify,
}
#[derive(Clone, Default)]
pub struct RetryCancellation(Arc<State>);
impl RetryCancellation {
    pub fn cancel(&self) {
        self.0.stopped.store(true, Ordering::Release);
        self.0.changed.notify_waiters();
    }
    pub fn stopped(&self) -> bool {
        self.0.stopped.load(Ordering::Acquire)
    }
    /// The same actual stop bit can govern progressive read delivery.
    pub fn flag(&self) -> Arc<AtomicBool> {
        self.0.stopped.clone()
    }
    pub async fn cancelled(&self) {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.stopped() {
                return;
            }
            changed.await;
        }
    }
}
#[derive(Clone, Default)]
pub struct RetryCancellationContext(Vec<RetryCancellation>);
pub(super) fn recovery_admission_context(
    admission: &RetryCancellation,
) -> RetryCancellationContext {
    RetryCancellationContext(vec![admission.clone()])
}
tokio::task_local! { static CANCELLATION: RetryCancellationContext; }
pub fn capture_retry_cancellation() -> RetryCancellationContext {
    CANCELLATION.try_with(Clone::clone).unwrap_or_default()
}
pub fn current_retry_cancellation() -> Option<RetryCancellation> {
    capture_retry_cancellation().0.last().cloned()
}
pub async fn with_retry_context<T>(
    context: RetryCancellationContext,
    future: impl Future<Output = T>,
) -> T {
    CANCELLATION.scope(context, future).await
}
pub async fn with_retry_cancellation<T>(
    cancellation: RetryCancellation,
    future: impl Future<Output = T>,
) -> T {
    let mut context = capture_retry_cancellation();
    if !context
        .0
        .iter()
        .any(|old| Arc::ptr_eq(&old.0, &cancellation.0))
    {
        context.0.push(cancellation);
    }
    with_retry_context(context, future).await
}
fn delay(attempt: u32, sequence: u64) -> Duration {
    let ceiling = (1u64 << attempt.min(8)).min(250);
    // Per-operation sequence decorrelates callers; jitter never changes their request.
    let mixed = sequence
        .wrapping_mul(0x9e3779b97f4a7c15)
        .rotate_left(attempt % 64);
    Duration::from_micros(ceiling * 500 + mixed % (ceiling * 500 + 1))
}
/// Application recapture begins only after an exact recorded no-op outcome.
/// Bound contention using the existing native lifecycle deadline; never abort
/// an issued query/effect to enforce it. Uncontended large audits have no timer.
pub(crate) struct PremiseRetry {
    retries: u32,
    sequence: u64,
    deadline: Option<tokio::time::Instant>,
}
impl PremiseRetry {
    pub(crate) fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        Self {
            retries: 0,
            sequence: SEQUENCE.fetch_add(1, Ordering::Relaxed),
            deadline: None,
        }
    }
    pub(crate) fn check(&self) -> Result<(), ModelError> {
        if capture_retry_cancellation()
            .0
            .iter()
            .any(RetryCancellation::stopped)
        {
            return Err(ModelError::Invalid(
                "original completion recapture cancelled".into(),
            ));
        }
        if self
            .deadline
            .is_some_and(|deadline| tokio::time::Instant::now() >= deadline)
        {
            return Err(ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Contention,
                "original completion audit premise continuously changed",
            ));
        }
        Ok(())
    }
    pub(crate) async fn changed(&mut self) -> Result<(), ModelError> {
        self.deadline.get_or_insert_with(|| {
            tokio::time::Instant::now()
                + Duration::from_millis(
                    lctx_model::domain::serving::ResourceLimits::default().request_deadline_ms,
                )
        });
        self.check()?;
        let context = capture_retry_cancellation();
        let wait = delay(self.retries, self.sequence);
        self.retries = self.retries.saturating_add(1);
        if context.0.is_empty() {
            tokio::time::sleep(wait).await;
        } else {
            let stops = context
                .0
                .iter()
                .map(|stop| Box::pin(stop.cancelled()))
                .collect::<Vec<_>>();
            tokio::select! {_=futures::future::select_all(stops)=>{},()=tokio::time::sleep(wait)=>{}}
        }
        self.check()
    }
}
pub(super) async fn transaction(
    client: &Surreal<Client>,
    sql: &str,
    bindings: Variables,
    recover: bool,
) -> Result<(), ModelError> {
    transaction_named(client, sql, bindings, recover, "control").await
}
pub(super) async fn transaction_named(
    client: &Surreal<Client>,
    sql: &str,
    bindings: Variables,
    recover: bool,
    stage: &'static str,
) -> Result<(), ModelError> {
    // Recovery changes the durable action, never the authority to issue forever.
    // Its owning scope installs an independent stop context and retains any
    // unsettled action before returning. Await the submitted terminal below;
    // only subsequent issuance/backoff observes this capability.
    let cancellation = capture_retry_cancellation();
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    // Identifies generated SQL shape and exact operation/stage without logging
    // SQL, payloads, credentials or caller-provided binding values.
    let query_digest = ContentHash::of(sql.as_bytes()).hex();
    let operation = bindings
        .get("__effect")
        .or_else(|| bindings.get("effect"))
        .and_then(|value| match value {
            Value::RecordId(id) if id.table.as_str() == "native_effect" => match &id.key {
                surrealdb::types::RecordIdKey::String(key) => Some(key.as_str()),
                _ => None,
            },
            _ => None,
        });
    let started = std::time::Instant::now();
    let mut retries = 0u32;
    loop {
        if cancellation.0.iter().any(RetryCancellation::stopped) {
            tracing::info!(target:"lctx_phase", sequence, stage, operation, %query_digest, retries, recovery=recover, elapsed_ms=started.elapsed().as_millis(), status="cancelled_before_issue", "native transaction terminal");
            return Err(ModelError::Invalid(
                "native transaction retry cancelled".into(),
            ));
        }
        let _recovery_owner = if recover {
            let owner = capture_recovery_scope().ok_or(ModelError::Conflict(
                "native recovery requires its actual owning scope",
            ))?;
            if cancellation.0.is_empty() || !std::ptr::eq(owner.client().as_ref(), client) {
                return Err(ModelError::Conflict(
                    "native recovery issuance owner correspondence",
                ));
            }
            Some(owner)
        } else {
            None
        };
        // Await actual response/terminal before observing cancellation. Dropping delivery
        // or a retry wait is never used as evidence that this effect did not commit.
        let result = match client.query(sql.to_owned()).bind(bindings.clone()).await {
            Ok(response) => checked_transaction(response).map(|_| ()),
            Err(error) => Err(crate::loader::write_failure(error)),
        };
        let Err(error) = result else {
            if retries > 0 {
                tracing::info!(target:"lctx_phase", sequence, stage, operation, %query_digest, retries, recovery=recover, elapsed_ms=started.elapsed().as_millis(), status="committed", "native transaction terminal");
            }
            return Ok(());
        };
        if !retryable_transaction(&error) {
            // This error may still reconcile as committed or remote Unknown.
            tracing::info!(target:"lctx_phase", sequence, stage, operation, %query_digest, retries, recovery=recover, elapsed_ms=started.elapsed().as_millis(), status="error_returned", "native transaction terminal");
            return Err(error);
        }
        let wait = delay(retries, sequence);
        retries = retries.saturating_add(1);
        // Logarithmic observations bound conflict logging during sustained contention.
        if retries.is_power_of_two() {
            tracing::info!(target:"lctx_phase", sequence, stage, operation, %query_digest, retries, delay_us=wait.as_micros(), recovery=recover, elapsed_ms=started.elapsed().as_millis(), conflict="transaction_conflict", "native transaction conflict; retaining original request");
        }
        let stopped = if !cancellation.0.is_empty() {
            let stops = cancellation
                .0
                .iter()
                .map(|stop| Box::pin(stop.cancelled()))
                .collect::<Vec<_>>();
            tokio::select! { _=futures::future::select_all(stops)=>true, ()=tokio::time::sleep(wait)=>false }
        } else {
            tokio::time::sleep(wait).await;
            false
        };
        if stopped {
            tracing::info!(target:"lctx_phase", sequence, stage, operation, %query_digest, retries, recovery=recover, elapsed_ms=started.elapsed().as_millis(), status="cancelled_after_conflict", "native transaction terminal");
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "native transaction retry cancellation",
                Err(ModelError::Invalid(
                    "native transaction retry cancelled".into(),
                )),
            );
            return lctx_model::domain::completion::complete(Err(error), completion);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn completion_recapture_obeys_nested_cancellation_and_reports_contention() {
        let outer = RetryCancellation::default();
        let inner = RetryCancellation::default();
        outer.cancel();
        let result = with_retry_cancellation(
            outer,
            with_retry_cancellation(inner, async { PremiseRetry::new().changed().await }),
        )
        .await;
        assert!(
            matches!(result,Err(ModelError::Invalid(message)) if message=="original completion recapture cancelled")
        );
        let mut retry = PremiseRetry::new();
        assert!(
            retry.deadline.is_none(),
            "the initial full audit is not timed as contention"
        );
        retry.changed().await.unwrap();
        assert!(retry.deadline.is_some());
        retry.deadline = Some(tokio::time::Instant::now());
        let failure = retry.changed().await.unwrap_err();
        assert!(
            matches!(
                failure,
                ModelError::Infrastructure {
                    class: lctx_model::domain::Infrastructure::Contention,
                    ..
                }
            ),
            "{failure:?}"
        );
    }
    #[tokio::test]
    async fn already_cancelled_retry_refuses_without_contacting_native() {
        for recovery in [false, true] {
            let cancellation = RetryCancellation::default();
            cancellation.cancel();
            let result = with_retry_cancellation(
                cancellation,
                transaction(&Surreal::init(), "RETURN true", Variables::new(), recovery),
            )
            .await;
            assert!(
                matches!(result, Err(ModelError::Invalid(message)) if message == "native transaction retry cancelled"),
                "recovery must retain its actual owner's stop authority"
            );
        }
    }
    #[tokio::test]
    async fn nested_scopes_and_spawned_context_retain_every_cancellation_owner() {
        let outer = RetryCancellation::default();
        let inner = RetryCancellation::default();
        let stopped = outer.clone();
        let context = with_retry_cancellation(
            outer,
            with_retry_cancellation(inner, async { capture_retry_cancellation() }),
        )
        .await;
        assert_eq!(context.0.len(), 2);
        stopped.cancel();
        let result = tokio::spawn(with_retry_context(context, async {
            transaction(&Surreal::init(), "RETURN true", Variables::new(), false).await
        }))
        .await
        .unwrap();
        assert!(
            matches!(result, Err(ModelError::Invalid(message)) if message == "native transaction retry cancelled")
        );
    }
    #[tokio::test]
    async fn cancellation_interrupts_backoff_without_losing_notification() {
        let cancellation = RetryCancellation::default();
        let waiting = cancellation.clone();
        let task = tokio::spawn(async move { waiting.cancelled().await });
        cancellation.cancel();
        task.await.unwrap();
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
