use super::error::ServiceError;
use tokio::sync::oneshot;

/// Message sent over buffer
#[derive(Debug)]
pub(crate) struct Message<Request, Fut> {
    pub(crate) request: Request,
    pub(crate) tx: Tx<Fut>,
    pub(crate) span: RequestSpan,
}

/// Response sender
pub(crate) type Tx<Fut> = oneshot::Sender<Result<Fut, ServiceError>>;

/// Response receiver
pub(crate) type Rx<Fut> = oneshot::Receiver<Result<Fut, ServiceError>>;


/// The request span owns its originating dispatcher through final destruction.
/// The wrapper, rather than `Message`, implements `Drop` so request and sender
/// fields can still be moved independently by the worker.
#[derive(Debug)]
pub(crate) struct RequestSpan {
    span: Option<tracing::Span>,
    dispatch: tracing::Dispatch,
}

impl RequestSpan {
    pub(crate) fn new(span: tracing::Span) -> Self {
        let dispatch = span
            .with_subscriber(|(_, dispatch)| dispatch.clone())
            .unwrap_or_else(|| tracing::dispatcher::get_default(Clone::clone));
        Self { span: Some(span), dispatch }
    }

    pub(crate) fn dispatcher(&self) -> &tracing::Dispatch {
        &self.dispatch
    }

    pub(crate) fn enter(&self) -> tracing::span::Entered<'_> {
        self.span.as_ref().expect("live request span").enter()
    }
}

impl Drop for RequestSpan {
    fn drop(&mut self) {
        let span = &mut self.span;
        tracing::dispatcher::with_default(&self.dispatch, || drop(span.take()));
    }
}
