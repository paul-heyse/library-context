//! Delivery cancellation fences later reads while the operation retains its native finalizer.
use lctx_model::domain::ModelError;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
#[derive(Clone, Default)]
pub(crate) struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub(crate) fn check(&self) -> Result<(), ModelError> {
        if self.0.load(Ordering::Acquire) { Err(ModelError::Cause(Box::new(std::io::Error::new(std::io::ErrorKind::Interrupted, "native read delivery cancelled")))) } else { Ok(()) }
    }
    pub(crate) fn flag(&self) -> Arc<AtomicBool> { self.0.clone() }
}
struct Delivery(Option<Cancellation>);
impl Drop for Delivery { fn drop(&mut self) { if let Some(cancellation) = &self.0 { cancellation.0.store(true, Ordering::Release); } } }
pub(crate) async fn run<T: Send + 'static, F, Fut>(owner: &'static str, operation: F) -> Result<T, ModelError>
where F: FnOnce(Cancellation) -> Fut + Send + 'static, Fut: std::future::Future<Output=Result<T, ModelError>> + Send + 'static {
    run_observed(operation, move |result| match result {
        Ok(_) => tracing::info!(owner, "native read finalization completed after delivery loss"),
        Err(error) => tracing::error!(owner, error = ?error, "native read terminal failure after delivery loss"),
    }).await
}
async fn run_observed<T: Send + 'static, F, Fut, R>(operation: F, report: R) -> Result<T, ModelError>
where F: FnOnce(Cancellation) -> Fut + Send + 'static, Fut: std::future::Future<Output=Result<T, ModelError>> + Send + 'static,
      R: FnOnce(Result<(), Arc<ModelError>>) + Send + 'static {
    let cancellation = Cancellation::default(); let mut delivery = Delivery(Some(cancellation.clone()));
    let operation = tokio::spawn(operation(cancellation));
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let (acknowledge, acknowledged) = tokio::sync::oneshot::channel();
    // This supervisor has no discarded Result. It observes task panics as well as
    // typed finalization failures, including failures discovered after caller abort.
    tokio::spawn(async move {
        let result = operation.await.map_err(|error| {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
            lctx_model::domain::completion::complete::<()>(Err(ModelError::Cause(Box::new(error))), completion).unwrap_err()
        }).and_then(|result| result);
        let (result, receipt) = match result {
            Ok(value) => (Ok(value), Ok(())),
            Err(error) => { let error = Arc::new(error); (Err(ModelError::SharedCause(error.clone())), Err(error)) }
        };
        // A successful send alone is not observation: cancellation can discard the
        // queued result before the waiter receives it. Retain the typed receipt until
        // the waiter acknowledges synchronous receipt, or report delivery loss.
        if sender.send(result).is_err() || acknowledged.await.is_err() { report(receipt); }
    });
    let result = receiver.await.map_err(ModelError::codec)?;
    let _ = acknowledge.send(());
    delivery.0 = None; result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancellation_fences_new_work_and_retains_finalizer_through_delayed_tail() {
        let (started, ready) = tokio::sync::oneshot::channel(); let (tail, drained) = tokio::sync::oneshot::channel(); let (finished, terminal) = tokio::sync::oneshot::channel();
        let released = Arc::new(AtomicBool::new(false)); let owner_released = released.clone();
        let waiter = tokio::spawn(run("cancellation control", move |cancel| async move {
            started.send(()).unwrap(); drained.await.unwrap();
            let result = cancel.check(); // No subsequent read is admitted after delivery cancellation.
            owner_released.store(true, Ordering::Release); finished.send(result.is_err()).unwrap(); result
        }));
        ready.await.unwrap(); waiter.abort(); assert!(waiter.await.unwrap_err().is_cancelled());
        assert!(!released.load(Ordering::Acquire)); tail.send(()).unwrap();
        assert!(terminal.await.unwrap()); assert!(released.load(Ordering::Acquire));
    }
    #[tokio::test]
    async fn cancelled_delivery_reports_delayed_cleanup_failure_and_join_panic() {
        for panic in [false, true] {
            let (started, ready) = tokio::sync::oneshot::channel();
            let (tail, drained) = tokio::sync::oneshot::channel();
            let (reported, observation) = tokio::sync::oneshot::channel();
            let waiter = tokio::spawn(run_observed(move |_cancel| async move {
                started.send(()).unwrap(); drained.await.unwrap();
                if panic { panic!("owned finalizer panic control"); }
                let mut completion = lctx_model::domain::completion::Completion::default();
                completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
                completion.step("delayed cleanup", Err(ModelError::Conflict("late cleanup failure")));
                lctx_model::domain::completion::complete(Err::<(), _>(ModelError::Conflict("primary read failure")), completion)
            }, move |result| { reported.send(format!("{result:?}")).unwrap(); }));
            ready.await.unwrap(); waiter.abort(); assert!(waiter.await.unwrap_err().is_cancelled());
            tail.send(()).unwrap();
            let report = observation.await.unwrap();
            if panic { assert!(report.contains("JoinError") && report.contains("owned finalizer panic control"), "{report}"); }
            else { for fragment in ["primary read failure", "late cleanup failure", "Unknown"] { assert!(report.contains(fragment), "{report}"); } }
        }
    }
}
