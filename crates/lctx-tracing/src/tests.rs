use super::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tracing_subscriber::{
    Layer, Registry, layer::Context as LayerContext, prelude::*, registry::LookupSpan,
};

#[derive(Default)]
struct Observed {
    polls: AtomicUsize,
    drops: AtomicUsize,
    closed: Mutex<Vec<&'static str>>,
}
#[derive(Clone)]
struct Observer(Arc<Observed>);
impl<S: tracing::Subscriber + for<'a> LookupSpan<'a>> Layer<S> for Observer {
    fn on_close(&self, id: tracing::Id, context: LayerContext<'_, S>) {
        self.0
            .closed
            .lock()
            .unwrap()
            .push(context.span(&id).unwrap().metadata().name());
    }
}
fn dispatch(observed: &Arc<Observed>) -> Dispatch {
    Dispatch::new(Registry::default().with(Observer(observed.clone())))
}
fn current_is(expected: &Arc<Observed>) -> bool {
    tracing::dispatcher::get_default(|dispatch| {
        dispatch
            .downcast_ref::<Observer>()
            .is_some_and(|actual| Arc::ptr_eq(&actual.0, expected))
    })
}
struct Probe {
    observed: Arc<Observed>,
    ready: bool,
    _span: Span,
}
impl Future for Probe {
    type Output = usize;
    fn poll(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<usize> {
        assert!(
            current_is(&self.observed),
            "poll uses its originating subscriber"
        );
        self.observed.polls.fetch_add(1, Ordering::Relaxed);
        if self.ready {
            Poll::Ready(42)
        } else {
            Poll::Pending
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        assert!(
            current_is(&self.observed),
            "destruction uses its originating subscriber"
        );
        self.observed.drops.fetch_add(1, Ordering::Relaxed);
    }
}
struct ProbeStream {
    probe: Probe,
    yielded: bool,
}
impl Stream for ProbeStream {
    type Item = usize;
    fn poll_next(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<Option<usize>> {
        let this = self.get_mut();
        assert!(current_is(&this.probe.observed));
        this.probe.observed.polls.fetch_add(1, Ordering::Relaxed);
        if this.yielded {
            Poll::Ready(None)
        } else {
            this.yielded = true;
            Poll::Ready(Some(42))
        }
    }
}
fn captured(
    origin: &Dispatch,
    observed: &Arc<Observed>,
    ready: bool,
) -> (Probe, Context, Vec<tracing::Id>) {
    tracing::dispatcher::with_default(origin, || {
        let parent = tracing::info_span!("parent");
        let child = parent.in_scope(|| tracing::info_span!("child"));
        let context = Context::from_span(child);
        let inner = context.scope(|| tracing::info_span!("inner"));
        let ids = vec![
            parent.id().unwrap(),
            context.span().id().unwrap(),
            inner.id().unwrap(),
        ];
        (
            Probe {
                observed: observed.clone(),
                ready,
                _span: inner,
            },
            context,
            ids,
        )
    })
}
fn closed(origin: &Dispatch, observed: &Observed, ids: &[tracing::Id]) {
    assert_eq!(observed.drops.load(Ordering::Relaxed), 1);
    let mut names = observed.closed.lock().unwrap().clone();
    names.sort_unstable();
    assert_eq!(names, ["child", "inner", "parent"]);
    let registry = origin.downcast_ref::<Registry>().unwrap();
    assert!(
        ids.iter().all(|id| registry.span(id).is_none()),
        "all original span references release"
    );
}
#[test]
fn span_origin_wins_over_foreign_constructor_ambient_and_unstarted_callback_drop() {
    let observed = Arc::new(Observed::default());
    let origin = dispatch(&observed);
    let foreign_observed = Arc::new(Observed::default());
    let foreign = dispatch(&foreign_observed);
    let (span, ids) = tracing::dispatcher::with_default(&origin, || {
        let parent = tracing::info_span!("constructor_parent");
        let child = parent.in_scope(|| tracing::info_span!("constructor_child"));
        let ids = vec![parent.id().unwrap(), child.id().unwrap()];
        (child, ids)
    });
    tracing::dispatcher::with_default(&foreign, || {
        let context = Context::from_span(span);
        context.scope(|| assert!(current_is(&observed)));
        let callback_observed = observed.clone();
        let callback = move || context.scope(|| assert!(current_is(&callback_observed)));
        // A blocking executor may discard an owned callback before starting it.
        drop(callback);
    });
    let mut names = observed.closed.lock().unwrap().clone();
    names.sort_unstable();
    assert_eq!(names, ["constructor_child", "constructor_parent"]);
    let registry = origin.downcast_ref::<Registry>().unwrap();
    assert!(ids.iter().all(|id| registry.span(id).is_none()));
    assert!(foreign_observed.closed.lock().unwrap().is_empty());
}
#[test]
fn future_ready_pending_cancellation_and_unpolled_drop_keep_origin_and_release_spans() {
    for polls in [None, Some(false), Some(true)] {
        let observed = Arc::new(Observed::default());
        let origin = dispatch(&observed);
        let foreign_observed = Arc::new(Observed::default());
        let foreign = dispatch(&foreign_observed);
        let (probe, context, ids) = captured(&origin, &observed, polls == Some(true));
        let mut future = Box::pin(probe.with_tracing_context(context));
        tracing::dispatcher::with_default(&foreign, || {
            if let Some(ready) = polls {
                let result = future
                    .as_mut()
                    .poll(&mut TaskContext::from_waker(std::task::Waker::noop()));
                assert_eq!(result.is_ready(), ready);
            }
            drop(future);
        });
        closed(&origin, &observed, &ids);
        assert_eq!(
            observed.polls.load(Ordering::Relaxed),
            usize::from(polls.is_some())
        );
        assert!(foreign_observed.closed.lock().unwrap().is_empty());
    }
}
#[test]
fn stream_completion_prefix_cancellation_and_unpolled_drop_keep_origin_and_release_spans() {
    for polls in [0, 1, 2] {
        let observed = Arc::new(Observed::default());
        let origin = dispatch(&observed);
        let foreign_observed = Arc::new(Observed::default());
        let foreign = dispatch(&foreign_observed);
        let (probe, context, ids) = captured(&origin, &observed, true);
        let mut stream = Box::pin(
            ProbeStream {
                probe,
                yielded: false,
            }
            .with_tracing_context(context),
        );
        tracing::dispatcher::with_default(&foreign, || {
            let mut cx = TaskContext::from_waker(std::task::Waker::noop());
            if polls > 0 {
                assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(Some(42)));
            }
            if polls > 1 {
                assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(None));
            }
            drop(stream);
        });
        closed(&origin, &observed, &ids);
        assert_eq!(observed.polls.load(Ordering::Relaxed), polls);
        assert!(foreign_observed.closed.lock().unwrap().is_empty());
    }
}

#[derive(Clone, Copy, Debug)]
enum BufferCase {
    Success,
    QueuedCancellation,
    PendingCancellation,
    PendingWorkerShutdown,
    WorkerShutdown,
    ReceiverGoneAfterReservation,
    ResponseSendRefusal,
    ReadinessFailure,
}
type CancelResponse = Arc<Mutex<Option<Box<dyn FnOnce() + Send>>>>;
struct BufferProbe {
    origin: Arc<Observed>,
    checks: Arc<Mutex<Vec<bool>>>,
    pending: bool,
    fail: bool,
    cancel_response: CancelResponse,
}
impl tower::Service<()> for BufferProbe {
    type Response = ();
    type Error = std::io::Error;
    type Future = std::future::Ready<Result<(), std::io::Error>>;
    fn poll_ready(&mut self, _: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.checks.lock().unwrap().push(current_is(&self.origin));
        if self.pending {
            Poll::Pending
        } else if self.fail {
            Poll::Ready(Err(std::io::Error::other("refused")))
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn call(&mut self, (): ()) -> Self::Future {
        self.checks.lock().unwrap().push(current_is(&self.origin));
        if let Some(cancel) = self.cancel_response.lock().unwrap().take() {
            cancel();
        }
        std::future::ready(Ok(()))
    }
}
#[test]
fn tower_buffer_request_origin_survives_foreign_worker_and_all_terminal_paths() {
    use tower::Service;
    for case in [
        BufferCase::Success,
        BufferCase::QueuedCancellation,
        BufferCase::PendingCancellation,
        BufferCase::PendingWorkerShutdown,
        BufferCase::WorkerShutdown,
        BufferCase::ReceiverGoneAfterReservation,
        BufferCase::ResponseSendRefusal,
        BufferCase::ReadinessFailure,
    ] {
        let observed = Arc::new(Observed::default());
        let origin = dispatch(&observed);
        let foreign_observed = Arc::new(Observed::default());
        let foreign = dispatch(&foreign_observed);
        let checks = Arc::new(Mutex::new(Vec::new()));
        let cancel_response: CancelResponse = Arc::new(Mutex::new(None));
        let service = BufferProbe {
            origin: observed.clone(),
            checks: checks.clone(),
            pending: matches!(
                case,
                BufferCase::PendingCancellation | BufferCase::PendingWorkerShutdown
            ),
            fail: matches!(case, BufferCase::ReadinessFailure),
            cancel_response: cancel_response.clone(),
        };
        let (mut buffer, worker) = tower::buffer::Buffer::pair(service, 2);
        let mut worker = Some(Box::pin(worker));
        let mut sentinel = Some(tracing::dispatcher::with_default(&foreign, || {
            tracing::info_span!("foreign_sentinel")
        }));
        let sentinel_id = sentinel.as_ref().unwrap().id().unwrap();
        let mut cx = TaskContext::from_waker(std::task::Waker::noop());
        assert!(buffer.poll_ready(&mut cx).is_ready());
        if matches!(case, BufferCase::ReceiverGoneAfterReservation) {
            tracing::dispatcher::with_default(&foreign, || drop(worker.take()));
        }
        let (response, ids) = tracing::dispatcher::with_default(&origin, || {
            let parent = tracing::info_span!("buffer_parent");
            let child = parent.in_scope(|| tracing::info_span!("buffer_child"));
            let ids = [parent.id().unwrap(), child.id().unwrap()];
            let response = child.in_scope(|| buffer.call(()));
            (response, ids)
        });
        let mut response = Some(Box::pin(response));
        if matches!(case, BufferCase::ResponseSendRefusal) {
            let response = response.take();
            *cancel_response.lock().unwrap() = Some(Box::new(move || drop(response)));
        }
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tracing::dispatcher::with_default(&foreign, || {
                if matches!(case, BufferCase::QueuedCancellation) {
                    drop(response.take());
                }
                if !matches!(
                    case,
                    BufferCase::WorkerShutdown | BufferCase::ReceiverGoneAfterReservation
                ) {
                    let result = worker.as_mut().unwrap().as_mut().poll(&mut cx);
                    assert_eq!(
                        result.is_ready(),
                        matches!(case, BufferCase::ReadinessFailure)
                    );
                }
                if matches!(case, BufferCase::PendingCancellation) {
                    drop(response.take());
                    assert!(worker.as_mut().unwrap().as_mut().poll(&mut cx).is_pending());
                }
                if matches!(case, BufferCase::Success | BufferCase::ReadinessFailure) {
                    assert!(response.as_mut().unwrap().as_mut().poll(&mut cx).is_ready());
                }
                drop(response.take());
                drop(worker.take());
                drop(buffer);
            });
        }));
        let registry = origin.downcast_ref::<Registry>().unwrap();
        let released = ids.iter().all(|id| registry.span(id).is_none());
        let foreign_registry = foreign.downcast_ref::<Registry>().unwrap();
        let sentinel_intact = foreign_registry.span(&sentinel_id).is_some()
            && foreign_observed.closed.lock().unwrap().is_empty();
        // If stock Tower has already closed the foreign ID, avoid a second diagnostic panic
        // while reporting the original regression. The patched path always releases normally.
        if foreign_registry.span(&sentinel_id).is_some() {
            tracing::dispatcher::with_default(&foreign, || drop(sentinel.take()));
        } else {
            std::mem::forget(sentinel.take());
        }
        assert!(
            outcome.is_ok(),
            "{case:?}: worker must not corrupt a foreign registry"
        );
        assert!(
            released,
            "{case:?}: originating parent and request spans release"
        );
        assert!(
            sentinel_intact,
            "{case:?}: foreign sentinel remains untouched"
        );
        assert!(
            checks.lock().unwrap().iter().all(|check| *check),
            "{case:?}: readiness and call use request origin"
        );
        let mut names = observed.closed.lock().unwrap().clone();
        names.sort_unstable();
        assert_eq!(names, ["buffer_child", "buffer_parent"], "{case:?}");
    }
}

#[test]
fn tower_buffer_one_worker_handles_distinct_request_dispatchers() {
    use std::collections::VecDeque;
    use tower::Service;
    struct Mixed {
        expected: VecDeque<Arc<Observed>>,
        checks: Arc<Mutex<Vec<bool>>>,
    }
    impl Service<()> for Mixed {
        type Response = ();
        type Error = std::io::Error;
        type Future = std::future::Ready<Result<(), Self::Error>>;
        fn poll_ready(&mut self, _: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
            self.checks
                .lock()
                .unwrap()
                .push(current_is(self.expected.front().unwrap()));
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, (): ()) -> Self::Future {
            self.checks
                .lock()
                .unwrap()
                .push(current_is(&self.expected.pop_front().unwrap()));
            std::future::ready(Ok(()))
        }
    }
    let first = Arc::new(Observed::default());
    let second = Arc::new(Observed::default());
    let origins = [dispatch(&first), dispatch(&second)];
    let foreign_observed = Arc::new(Observed::default());
    let foreign = dispatch(&foreign_observed);
    let checks = Arc::new(Mutex::new(Vec::new()));
    let (mut buffer, worker) = tower::buffer::Buffer::pair(
        Mixed {
            expected: VecDeque::from([first.clone(), second.clone()]),
            checks: checks.clone(),
        },
        2,
    );
    let mut worker = Box::pin(worker);
    let mut cx = TaskContext::from_waker(std::task::Waker::noop());
    let mut responses = Vec::new();
    let mut ids = Vec::new();
    for origin in &origins {
        assert!(buffer.poll_ready(&mut cx).is_ready());
        let (response, pair) = tracing::dispatcher::with_default(origin, || {
            let parent = tracing::info_span!("mixed_parent");
            let child = parent.in_scope(|| tracing::info_span!("mixed_child"));
            let pair = [parent.id().unwrap(), child.id().unwrap()];
            (child.in_scope(|| buffer.call(())), pair)
        });
        responses.push(Box::pin(response));
        ids.push(pair);
    }
    tracing::dispatcher::with_default(&foreign, || {
        let sentinel = tracing::info_span!("mixed_foreign_sentinel");
        assert!(worker.as_mut().poll(&mut cx).is_pending());
        assert!(foreign_observed.closed.lock().unwrap().is_empty());
        for response in &mut responses {
            assert!(response.as_mut().poll(&mut cx).is_ready());
        }
        drop(responses);
        drop(buffer);
        assert!(worker.as_mut().poll(&mut cx).is_ready());
        drop(worker);
        drop(sentinel);
    });
    assert_eq!(*checks.lock().unwrap(), [true, true, true, true]);
    for ((origin, observed), pair) in origins.iter().zip([first, second]).zip(ids) {
        let registry = origin.downcast_ref::<Registry>().unwrap();
        assert!(pair.iter().all(|id| registry.span(id).is_none()));
        let mut names = observed.closed.lock().unwrap().clone();
        names.sort_unstable();
        assert_eq!(names, ["mixed_child", "mixed_parent"]);
    }
    assert_eq!(
        *foreign_observed.closed.lock().unwrap(),
        ["mixed_foreign_sentinel"]
    );
}
