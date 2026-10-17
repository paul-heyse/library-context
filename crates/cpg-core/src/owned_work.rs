//! Finite private work on the existing workspace join owner, reduced in declared order.
use crate::workspace::Workspace;
use futures::{StreamExt, stream::FuturesUnordered};
use lctx_model::domain::{ModelError, completion, resources::Reservation};
use std::future::Future;

/// One charged Arc allocation; typed values retain their own deep backing charges.
pub(crate) struct Shared<T> {
    value: T,
    _charge: Box<dyn Reservation>,
}
impl<T> Shared<T> {
    pub(crate) fn new(
        value: T,
        budget: &lctx_model::domain::resources::ResourceBudget,
    ) -> Result<std::sync::Arc<Self>, ModelError> {
        let charge = budget.reserve(
            "private shared work owner",
            size_of::<Self>().saturating_add(2 * size_of::<usize>()),
        )?;
        Ok(std::sync::Arc::new(Self {
            value,
            _charge: charge,
        }))
    }
    pub(crate) fn into_inner(owner: std::sync::Arc<Self>) -> Result<T, ModelError> {
        std::sync::Arc::try_unwrap(owner)
            .map(|owner| owner.value)
            .map_err(|_| ModelError::Conflict("private shared owner still borrowed"))
    }
}
impl<T> std::ops::Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

/// The result allocation and all remaining typed payloads stay owned while consumed.
pub(crate) struct OrderedResults<T> {
    values: std::vec::IntoIter<Option<T>>,
    _charge: Box<dyn Reservation>,
}
impl<T> Iterator for OrderedResults<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.values
            .next()
            .map(|value| value.expect("successful owned work ordinal"))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.values.size_hint()
    }
}
impl<T> ExactSizeIterator for OrderedResults<T> {}

/// Each future owns its actual prepared inputs and deep result reservations. This finite
/// coordinator supplies only task placement, charged ordinal slots and joined failure reduction;
/// it neither estimates kernel memory nor retries a failed operation.
pub(crate) async fn ordered<T, F, I>(
    workspace: &Workspace,
    work: I,
) -> Result<OrderedResults<T>, ModelError>
where
    T: Send + 'static,
    F: Future<Output = Result<T, ModelError>> + Send + 'static,
    I: IntoIterator<Item = F>,
    I::IntoIter: ExactSizeIterator,
{
    let work = work.into_iter();
    let len = work.len();
    let charge = workspace.budget().reserve(
        "private work ordinal results",
        len.checked_mul(size_of::<Option<T>>().saturating_add(128))
            .ok_or(ModelError::Conflict("private work slots overflow"))?,
    )?;
    let mut values = std::iter::repeat_with(|| None)
        .take(len)
        .collect::<Vec<_>>();
    let mut pending = FuturesUnordered::new();
    let mut failures = std::collections::BTreeMap::new();
    for (ordinal, future) in work.enumerate() {
        if let Err(error) = workspace.cancellation().check() {
            failures.insert(ordinal, error);
            break;
        }
        match workspace.spawn_stage(future) {
            Ok(task) => pending.push(async move { (ordinal, task.await) }),
            Err(error) => {
                failures.insert(ordinal, error);
                workspace.cancellation().cancel();
                break;
            }
        }
    }
    while let Some((ordinal, result)) = pending.next().await {
        match result {
            Ok(value) if failures.is_empty() => values[ordinal] = Some(value),
            Ok(_) => {}
            Err(error) => {
                // Delivered successful payloads have no consumer after failure. Their
                // workers have already joined; keep only slot backing and peer joins.
                if failures.is_empty() {
                    values.iter_mut().for_each(|value| *value = None);
                }
                failures.insert(ordinal, error);
                workspace.cancellation().cancel();
            }
        }
    }
    if !failures.is_empty() {
        // Cooperative cancellation is secondary to the failure that requested it.
        // Genuine peer failures reduce in declared order, independently of arrival.
        let key=failures.iter().find(|(_,error)|!matches!(error,ModelError::Invalid(message) if message=="compilation cancelled"))
            .or_else(||failures.first_key_value()).map(|(key,_)|*key).expect("private work failure");
        let primary = failures.remove(&key).expect("private work primary");
        let mut terminal = completion::Completion::default();
        for (_, error) in failures {
            terminal.step("private peer completion", Err(error));
        }
        return completion::complete(Err(primary), terminal);
    }
    workspace.cancellation().check()?;
    if values.iter().any(Option::is_none) {
        return Err(ModelError::Conflict("private work missing ordinal"));
    }
    Ok(OrderedResults {
        values: values.into_iter(),
        _charge: charge,
    })
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::{resources::ResourceBudget, serving::Name};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    fn workspace() -> Arc<Workspace> {
        let native = lctx_compiler_store::compiler::NativeCompilerStore::from_existing(
            Arc::new(lctx_native::surrealdb::Surreal::init()),
            Name::new("owned-work-control").unwrap(),
            Name::new("owned-work-control").unwrap(),
        );
        Workspace::new(
            Arc::new(lctx_model::domain::model().unwrap()),
            Default::default(),
            native,
        )
        .unwrap()
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn gated_peers_progress_and_results_reduce_in_declared_order() {
        let workspace = workspace();
        let gate = Arc::new(tokio::sync::Notify::new());
        let work = (0..3).map(|ordinal| {
            let gate = gate.clone();
            async move {
                if ordinal == 0 {
                    gate.notified().await;
                } else if ordinal == 2 {
                    gate.notify_one();
                }
                Ok(ordinal)
            }
        });
        let results = ordered(&workspace, work).await.unwrap().collect::<Vec<_>>();
        assert_eq!(results, vec![0, 1, 2]);
        // No native I/O was admitted; actual registered task joins all completed.
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn reversed_failures_keep_canonical_cause_and_drain_every_started_peer() {
        let workspace = workspace();
        let done = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new(tokio::sync::Notify::new());
        let entered = Arc::new(tokio::sync::Barrier::new(3));
        let work = (0..3).map(|ordinal| {
            let gate = gate.clone();
            let done = done.clone();
            let entered = entered.clone();
            async move {
                entered.wait().await;
                if ordinal == 0 {
                    gate.notified().await;
                } else if ordinal == 2 {
                    gate.notify_one();
                }
                done.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(ModelError::Conflict(if ordinal == 0 {
                    "first declared failure"
                } else {
                    "later failure"
                }))
            }
        });
        let error = ordered(&workspace, work).await.err().unwrap();
        assert!(format!("{error:?}").contains("first declared failure"));
        assert_eq!(done.load(Ordering::SeqCst), 3);
    }
    #[test]
    fn shared_charge_survives_last_borrower_and_refuses_early_unwrap() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let value = Shared::new(17u64, &budget).unwrap();
        let peer = value.clone();
        let reserved = budget.reserved();
        assert!(reserved > 0);
        assert!(Shared::into_inner(value).is_err());
        assert_eq!(budget.reserved(), reserved);
        assert_eq!(**peer, 17);
        assert_eq!(Shared::into_inner(peer).unwrap(), 17);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn failed_window_releases_delivered_payload_before_a_gated_peer_drains() {
        struct Payload {
            _charge: Box<dyn Reservation>,
            dropped: Arc<AtomicUsize>,
        }
        impl Drop for Payload {
            fn drop(&mut self) {
                self.dropped.fetch_add(1, Ordering::SeqCst);
            }
        }
        let workspace = workspace();
        let dropped = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(tokio::sync::Barrier::new(4));
        let mut senders = Vec::new();
        let mut finished = Vec::new();
        let mut work = Vec::new();
        for _ in 0..3 {
            let (send, receive) = tokio::sync::oneshot::channel::<Result<Payload, ModelError>>();
            let (done, observed) = tokio::sync::oneshot::channel();
            senders.push(send);
            finished.push(observed);
            let entered = entered.clone();
            work.push(async move {
                entered.wait().await;
                let value = receive.await.unwrap();
                let _ = done.send(());
                value
            });
        }
        let operation = ordered(&workspace, work);
        tokio::pin!(operation);
        assert!(futures::poll!(&mut operation).is_pending());
        entered.wait().await; // Every peer has passed admission and owns its actual body.
        let mut senders = senders.into_iter();
        let mut finished = finished.into_iter();
        let payload = || Payload {
            _charge: workspace
                .budget()
                .reserve("gated private output", 128)
                .unwrap(),
            dropped: dropped.clone(),
        };
        assert!(senders.next().unwrap().send(Ok(payload())).is_ok());
        finished.next().unwrap().await.unwrap();
        // On this current-thread runtime the sender finishes its task in the same poll;
        // this poll collects that joined success before the failure is made available.
        assert!(futures::poll!(&mut operation).is_pending());
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        let retained = workspace.budget().reserved();
        assert!(
            senders
                .next()
                .unwrap()
                .send(Err(ModelError::Conflict("gated peer failure")))
                .is_ok()
        );
        finished.next().unwrap().await.unwrap();
        assert!(futures::poll!(&mut operation).is_pending());
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert!(workspace.budget().reserved() <= retained - 128);
        // The remaining accepted worker is still gated, independently of dead output.
        assert!(senders.next().unwrap().send(Ok(payload())).is_ok());
        let error = operation.await.err().unwrap();
        assert!(format!("{error:?}").contains("gated peer failure"));
        finished.next().unwrap().await.unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
    }
}
