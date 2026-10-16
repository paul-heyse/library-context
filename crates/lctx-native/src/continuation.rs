//! Budgeted type erasure for the few large native operation continuations.
use futures::future::BoxFuture;
use lctx_model::domain::{
    ModelError,
    resources::{Reservation, ResourceBudget},
};
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Erased admitted continuation, compatible with ordinary native future APIs.
pub type Continuation<'a, T> = BoxFuture<'a, T>;

pin_project! {
    // The pinned future is destroyed before its reservation, including before
    // the first poll. Declaration order is the contract; keep the grant last.
    struct ChargedFuture<F, H> {
        #[pin]
        future: F,
        _hold: H,
    }
}

impl<F: Future, H> Future for ChargedFuture<F, H> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().future.poll(context)
    }
}

/// Retain an existing admission until the owned future has been destroyed.
/// This composes inline without allocating or charging the payload again.
pub fn retain<F: Future, H>(future: F, hold: H) -> impl Future<Output = F::Output> {
    ChargedFuture {
        future,
        _hold: hold,
    }
}

/// Admit the concrete future before allocating or polling it. The returned
/// carrier erases its type without another async frame or a second allocation.
#[inline(never)]
pub fn admit<'a, F>(
    budget: &ResourceBudget,
    owner: &'static str,
    future: F,
) -> Result<Continuation<'a, F::Output>, ModelError>
where
    F: Future + Send + 'a,
    F::Output: 'a,
{
    let padding = std::mem::align_of::<ChargedFuture<F, Box<dyn Reservation>>>()
        .checked_mul(2)
        .ok_or(ModelError::Schema(
            "native continuation allocation alignment",
        ))?;
    let bytes = std::mem::size_of::<ChargedFuture<F, Box<dyn Reservation>>>()
        .checked_add(padding)
        .ok_or(ModelError::Schema("native continuation allocation size"))?;
    let grant = budget.reserve(owner, bytes)?;
    Ok(Box::pin(retain(future, grant)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        marker::PhantomPinned,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    struct ObservedFuture {
        budget: ResourceBudget,
        expected: Arc<AtomicUsize>,
        dropped: Arc<AtomicUsize>,
        polls: Arc<AtomicUsize>,
        ready: bool,
        _pinned: PhantomPinned,
    }
    impl Future for ObservedFuture {
        type Output = usize;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<usize> {
            let this = self.as_ref().get_ref();
            this.polls.fetch_add(1, Ordering::Relaxed);
            if this.ready {
                Poll::Ready(17)
            } else {
                Poll::Pending
            }
        }
    }
    impl Drop for ObservedFuture {
        fn drop(&mut self) {
            let expected = self.expected.load(Ordering::Relaxed);
            assert!(expected > 0);
            assert_eq!(
                self.budget.reserved(),
                expected,
                "inner destruction remains charged"
            );
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn ready_pending_and_unpolled_destruction_keep_charge_until_inner_drop() {
        for ready in [None, Some(false), Some(true)] {
            let budget = ResourceBudget::fixed(4096).unwrap();
            let expected = Arc::new(AtomicUsize::new(0));
            let dropped = Arc::new(AtomicUsize::new(0));
            let polls = Arc::new(AtomicUsize::new(0));
            let mut continuation = admit(
                &budget,
                "continuation drop control",
                ObservedFuture {
                    budget: budget.clone(),
                    expected: expected.clone(),
                    dropped: dropped.clone(),
                    polls: polls.clone(),
                    ready: ready == Some(true),
                    _pinned: PhantomPinned,
                },
            )
            .unwrap();
            expected.store(budget.reserved(), Ordering::Relaxed);
            if let Some(ready) = ready {
                let result = Pin::new(&mut continuation)
                    .poll(&mut Context::from_waker(std::task::Waker::noop()));
                assert_eq!(result.is_ready(), ready);
            }
            drop(continuation);
            assert_eq!(polls.load(Ordering::Relaxed), usize::from(ready.is_some()));
            assert_eq!(dropped.load(Ordering::Relaxed), 1);
            assert_eq!(budget.reserved(), 0);
        }
    }
}
