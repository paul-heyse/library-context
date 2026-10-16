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
