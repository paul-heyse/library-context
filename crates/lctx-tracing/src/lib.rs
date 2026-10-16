//! Diagnostic context follows owned work through polling and destruction.
//! This carries no semantic, cancellation, recovery or terminality authority.
use futures_core::Stream;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};
use tracing::{Dispatch, Span};

/// The actual span and its originating dispatcher, including its final release.
#[derive(Clone, Debug)]
pub struct Context {
    span: Option<Span>,
    dispatch: Dispatch,
}
impl Context {
    pub fn current() -> Self {
        Self::from_span(Span::current())
    }
    pub fn from_span(span: Span) -> Self {
        Self::from_span_and_dispatch(span, tracing::dispatcher::get_default(Clone::clone))
    }
    pub fn from_span_and_dispatch(span: Span, fallback: Dispatch) -> Self {
        let dispatch = span
            .with_subscriber(|(_, dispatch)| dispatch.clone())
            .unwrap_or(fallback);
        Self {
            span: Some(span),
            dispatch,
        }
    }
    /// Explicit subscriber selection without importing a foreign current span.
    pub fn from_dispatch(dispatch: Dispatch) -> Self {
        Self {
            span: Some(Span::none()),
            dispatch,
        }
    }
    pub fn span(&self) -> &Span {
        self.span.as_ref().expect("live tracing context")
    }
    pub fn dispatcher(&self) -> &Dispatch {
        &self.dispatch
    }
    pub fn scope<T>(&self, action: impl FnOnce() -> T) -> T {
        tracing::dispatcher::with_default(&self.dispatch, || self.span().in_scope(action))
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        tracing::dispatcher::with_default(&self.dispatch, || drop(self.span.take()));
    }
}

pin_project! {
    /// A future or stream whose owned state is destroyed under its tracing origin.
    /// Pin projection adds no allocation and supports non-Unpin local work.
    #[must_use = "wrapped work does nothing until polled"]
    pub struct WithContext<T> {
        #[pin]
        inner: Option<T>,
        context: Context,
    }
    impl<T> PinnedDrop for WithContext<T> {
        fn drop(this: Pin<&mut Self>) {
            let mut this = this.project();
            this.context.scope(|| this.inner.set(None));
        }
    }
}
impl<F: Future> Future for WithContext<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        this.context.scope(|| {
            let result = this
                .inner
                .as_mut()
                .as_pin_mut()
                .expect("live tracing future")
                .poll(cx);
            if result.is_ready() {
                this.inner.set(None);
            }
            result
        })
    }
}
impl<S: Stream> Stream for WithContext<S> {
    type Item = S::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.context.scope(|| {
            let Some(inner) = this.inner.as_mut().as_pin_mut() else {
                return Poll::Ready(None);
            };
            let result = inner.poll_next(cx);
            if matches!(result, Poll::Ready(None)) {
                this.inner.set(None);
            }
            result
        })
    }
}
pub trait InContext: Sized {
    fn with_tracing_context(self, context: Context) -> WithContext<Self> {
        WithContext {
            inner: Some(self),
            context,
        }
    }
    fn in_tracing_context(self) -> WithContext<Self> {
        self.with_tracing_context(Context::current())
    }
    fn with_tracing_span(self, span: Span) -> WithContext<Self> {
        self.with_tracing_context(Context::from_span(span))
    }
    fn with_tracing_dispatch(self, dispatch: Dispatch) -> WithContext<Self> {
        self.with_tracing_context(Context::from_dispatch(dispatch))
    }
}
impl<T> InContext for T {}

#[cfg(test)]
mod tests;
