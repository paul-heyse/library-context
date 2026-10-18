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

/// Results are delivered in canonical order as soon as that prefix is ready. The
/// registered workspace stages remain the actual join owner if delivery is dropped.
type PendingResult<T> = futures::future::BoxFuture<'static, (usize, Result<T, ModelError>)>;
pub(crate) struct OrderedResults<T> {
    values: Vec<Option<T>>,
    pending: FuturesUnordered<PendingResult<T>>,
    failures: std::collections::BTreeMap<usize, ModelError>,
    next: usize,
    finished: bool,
    cancellation: crate::workspace::Cancellation,
    _charge: Box<dyn Reservation>,
}
impl<T> Drop for OrderedResults<T> {
    fn drop(&mut self) {
        if !self.finished {
            self.cancellation.cancel();
        }
    }
}
impl<T> OrderedResults<T> {
    pub(crate) async fn next(&mut self) -> Result<Option<T>, ModelError> {
        if self.finished {
            return Ok(None);
        }
        loop {
            if self.failures.is_empty() {
                if let Some(value) = self.values.get_mut(self.next).and_then(Option::take) {
                    self.next += 1;
                    if self.next == self.values.len() && self.pending.is_empty() {
                        self.cancellation.check()?;
                        self.finished = true;
                    }
                    return Ok(Some(value));
                }
                if self.next == self.values.len() {
                    self.cancellation.check()?;
                    self.finished = true;
                    return Ok(None);
                }
            }
            match self.pending.next().await {
                Some((ordinal, Ok(value))) if self.failures.is_empty() => {
                    self.values[ordinal] = Some(value)
                }
                Some((_, Ok(_))) => {}
                Some((ordinal, Err(error))) => {
                    self.values.iter_mut().for_each(|value| *value = None);
                    self.failures.insert(ordinal, error);
                    self.cancellation.cancel();
                }
                None => {
                    self.finished = true;
                    if self.failures.is_empty() {
                        return Err(ModelError::Conflict("private work missing ordinal"));
                    }
                    let key = self.failures.iter().find(|(_,error)| !matches!(error,ModelError::Invalid(message) if message=="compilation cancelled"))
                        .or_else(|| self.failures.first_key_value()).map(|(key,_)| *key).expect("private work failure");
                    let primary = self.failures.remove(&key).expect("private work primary");
                    let mut terminal = completion::Completion::default();
                    for (_, error) in std::mem::take(&mut self.failures) {
                        terminal.step("private peer completion", Err(error));
                    }
                    return completion::complete(Err(primary), terminal);
                }
            }
        }
    }
    #[cfg(test)]
    async fn collect(mut self) -> Result<Vec<T>, ModelError> {
        let mut values = Vec::new();
        while let Some(value) = self.next().await? {
            values.push(value);
        }
        Ok(values)
    }
}

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
    use futures::FutureExt;
    let work = work.into_iter();
    let len = work.len();
    let charge = workspace.budget().reserve(
        "private work ordinal results",
        len.checked_mul(size_of::<Option<T>>().saturating_add(128))
            .ok_or(ModelError::Conflict("private work slots overflow"))?,
    )?;
    let mut results = OrderedResults {
        values: std::iter::repeat_with(|| None).take(len).collect(),
        pending: FuturesUnordered::new(),
        failures: Default::default(),
        next: 0,
        finished: false,
        cancellation: workspace.cancellation(),
        _charge: charge,
    };
    for (ordinal, future) in work.enumerate() {
        let task = workspace
            .cancellation()
            .check()
            .and_then(|()| workspace.spawn_stage(future));
        match task {
            Ok(task) => results
                .pending
                .push(async move { (ordinal, task.await) }.boxed()),
            Err(error) => {
                results.failures.insert(ordinal, error);
                results.cancellation.cancel();
                break;
            }
        }
    }
    Ok(results)
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::resources::ResourceBudget;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    fn workspace() -> Arc<Workspace> {
        Workspace::private(
            Arc::new(lctx_model::domain::model().unwrap()),
            Default::default(),
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
        let results = ordered(&workspace, work)
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();
        assert_eq!(results, vec![0, 1, 2]);
        // No native I/O was admitted; actual registered task joins all completed.
    }
    #[tokio::test]
    async fn first_canonical_result_is_delivered_before_a_later_job_finishes() {
        let workspace = workspace();
        let (release, held) = tokio::sync::oneshot::channel();
        let later_done = Arc::new(AtomicUsize::new(0));
        let observed = later_done.clone();
        let work: Vec<futures::future::BoxFuture<'static, Result<usize, ModelError>>> = vec![
            Box::pin(async { Ok(0) }),
            Box::pin(async move {
                held.await.map_err(ModelError::codec)?;
                observed.store(1, Ordering::SeqCst);
                Ok(1)
            }),
        ];
        let mut ready = ordered(&workspace, work).await.unwrap();
        assert_eq!(ready.next().await.unwrap(), Some(0));
        assert_eq!(later_done.load(Ordering::SeqCst), 0);
        release.send(()).unwrap();
        assert_eq!(ready.next().await.unwrap(), Some(1));
        assert_eq!(ready.next().await.unwrap(), None);
        assert_eq!(later_done.load(Ordering::SeqCst), 1);
        drop(ready);
        workspace.cancellation().check().unwrap();
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
        let error = ordered(&workspace, work)
            .await
            .unwrap()
            .collect()
            .await
            .err()
            .unwrap();
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
        let mut ready = ordered(&workspace, work).await.unwrap();
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
        let first = ready.next().await.unwrap().unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        drop(first);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        let operation = ready.next();
        tokio::pin!(operation);
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
        assert!(workspace.budget().reserved() <= retained);
        // The remaining accepted worker is still gated, independently of dead output.
        assert!(senders.next().unwrap().send(Ok(payload())).is_ok());
        let error = operation.await.err().unwrap();
        assert!(format!("{error:?}").contains("gated peer failure"));
        finished.next().unwrap().await.unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
    }
}
