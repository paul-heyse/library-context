//! Canonical serving admission, shared CPU capacity and cancellation-safe ownership.
use super::{
    AdmittedSelection, Error, GenerationGuard, GenerationId, GenerationLease, GenerationReader,
};
use crate::roles::{Role, RoleConfig};
use lctx_model::domain::{
    Batch, Record, ValidatedModel, resources::ResourceBudget, serving::ResourceLimits,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Notify, OwnedSemaphorePermit, Semaphore},
    time::Instant,
};
struct State {
    reader: GenerationReader,
    guard: GenerationGuard,
    selection: AdmittedSelection,
    memory: ResourceBudget,
    preparation: ResourceBudget,
    slots: Arc<Semaphore>,
    queries: Arc<Semaphore>,
    stopped: AtomicBool,
    admission: std::sync::Mutex<()>,
    active: AtomicUsize,
    drained: Notify,
    limits: ResourceLimits,
}
#[derive(Clone)]
pub struct GenerationService {
    state: Arc<State>,
}
/// Service-owned library buffers remain charged after their initializing request has ended.
pub struct PreparedReservation {
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
    _guard: GenerationGuard,
}
struct Execution {
    state: Arc<State>,
    _slot: OwnedSemaphorePermit,
    budget: ResourceBudget,
    deadline: Instant,
    retained: std::sync::Mutex<Vec<Box<dyn lctx_model::domain::resources::Reservation>>>,
}
/// One execution grant crosses native and numerical Python phases. Clones share its slot;
/// cancelling the caller cannot free it while a spawned worker retains the grant.
#[derive(Clone)]
pub struct RequestExecution {
    inner: Arc<Execution>,
}
impl Drop for Execution {
    fn drop(&mut self) {
        self.state.active.fetch_sub(1, Ordering::AcqRel);
        self.state.drained.notify_waiters();
    }
}
fn refused(owner: &'static str) -> Error {
    Error::ResourceRefused(owner)
}
impl GenerationService {
    /// Reads operator selection once if no explicit generation is supplied. No writes or repair.
    pub async fn admit(
        model: Arc<ValidatedModel>,
        config: &RoleConfig,
        generation: Option<GenerationId>,
    ) -> Result<Self, Error> {
        if config.role != Role::Serving || config.max_connections < 3 {
            return Err(Error::Contract);
        }
        let limits = ResourceLimits::default();
        limits.validate()?;
        // Runtime has no compute providers; its combined role capacity is guard + queries.
        let mut runtime_config = config.clone();
        runtime_config.provider_connections = 0;
        runtime_config.max_connections = limits.query_connections + 1;
        let reader = GenerationReader::connect(model, &runtime_config).await?;
        reader
            .check_serving_capacity(limits.query_connections + 1)
            .await?;
        let generation = match generation {
            Some(g) => g,
            None => reader.selected().await?,
        };
        let memory = ResourceBudget::fixed(limits.shared_bytes as usize)?;
        let preparation = ResourceBudget::scoped(&memory, limits.preparation_bytes as usize)?;
        let guard = reader.guard(generation, preparation.clone()).await?;
        let selection = guard.prepare_selection().await?;
        Ok(Self {
            state: Arc::new(State {
                reader,
                guard,
                selection,
                memory,
                preparation,
                slots: Arc::new(Semaphore::new(limits.cpu_jobs as usize)),
                queries: Arc::new(Semaphore::new(limits.query_connections as usize)),
                stopped: AtomicBool::new(false),
                active: AtomicUsize::new(0),
                admission: std::sync::Mutex::new(()),
                drained: Notify::new(),
                limits,
            }),
        })
    }
    pub fn generation(&self) -> GenerationId {
        self.state.guard.generation()
    }
    pub fn guard(&self) -> GenerationGuard {
        self.state.guard.clone()
    }
    pub(crate) fn selection(&self) -> AdmittedSelection {
        self.state.selection.clone()
    }
    pub fn memory_reserved(&self) -> usize {
        self.state.memory.reserved()
    }
    pub async fn reserve_prepared(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<PreparedReservation, Error> {
        if self.state.stopped.load(Ordering::Acquire) {
            return Err(Error::State);
        }
        self.state.guard.check().await?;
        Ok(PreparedReservation {
            _charge: self.state.preparation.reserve(owner, bytes)?,
            _guard: self.state.guard.clone(),
        })
    }
    pub async fn execution(&self) -> Result<RequestExecution, Error> {
        let deadline =
            Instant::now() + Duration::from_millis(self.state.limits.request_deadline_ms);
        if self.state.stopped.load(Ordering::Acquire) {
            return Err(Error::State);
        }
        self.state.guard.check().await?;
        let slot = tokio::time::timeout_at(
            deadline
                .min(Instant::now() + Duration::from_millis(self.state.limits.admission_wait_ms)),
            self.state.slots.clone().acquire_owned(),
        )
        .await
        .map_err(|_| refused("CPU admission wait"))?
        .map_err(|_| Error::State)?;
        let budget =
            ResourceBudget::scoped(&self.state.memory, self.state.limits.request_bytes as usize)?;
        let _registration = self.state.admission.lock().map_err(|_| Error::State)?;
        if self.state.stopped.load(Ordering::Acquire) {
            return Err(Error::State);
        }
        self.state.active.fetch_add(1, Ordering::AcqRel);
        Ok(RequestExecution {
            inner: Arc::new(Execution {
                state: self.state.clone(),
                _slot: slot,
                budget,
                deadline,
                retained: std::sync::Mutex::new(Vec::new()),
            }),
        })
    }
    /// Stop new work, drain actual jobs and query cleanup, then acknowledge original guard release.
    pub async fn shutdown(&self) -> Result<(), Error> {
        {
            let _registration = self.state.admission.lock().map_err(|_| Error::State)?;
            self.state.stopped.store(true, Ordering::Release);
            self.state.slots.close();
        }
        loop {
            let notified = self.state.drained.notified();
            if self.state.active.load(Ordering::Acquire) == 0 {
                break;
            }
            notified.await;
        }
        let released = self.state.guard.close().await;
        self.state.reader.close().await;
        released
    }
}
impl RequestExecution {
    pub(super) fn shares_guard(&self, guard: &GenerationGuard) -> bool {
        Arc::ptr_eq(&self.inner.state.guard.state, &guard.state)
    }
    /// Buffers handed across phases remain charged until the actual execution grant ends.
    pub fn retain(&self, owner: &'static str, bytes: usize) -> Result<(), Error> {
        let charge = self.budget().reserve(
            owner,
            bytes
                .checked_add(64)
                .ok_or(Error::ResourceRefused("retained buffer size"))?,
        )?;
        self.inner
            .retained
            .lock()
            .map_err(|_| Error::State)?
            .push(charge);
        Ok(())
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.inner.budget
    }
    pub fn remaining(&self) -> Result<Duration, Error> {
        self.inner
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| refused("request deadline"))
    }
    pub fn generation(&self) -> GenerationId {
        self.inner.state.guard.generation()
    }
    pub async fn confirm(&self) -> Result<(), Error> {
        self.remaining()?;
        if self.inner.state.stopped.load(Ordering::Acquire) {
            return Err(Error::State);
        }
        tokio::time::timeout_at(self.inner.deadline, self.inner.state.guard.check())
            .await
            .map_err(|_| refused("request deadline"))?
    }
    pub async fn cpu<T: Send + 'static>(
        &self,
        work: impl FnOnce(&ResourceBudget) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        self.confirm().await?;
        let retained = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            retained.remaining()?;
            work(retained.budget())
        });
        // Dropping/timeout of this wait leaves the blocking worker, grant and reservations intact.
        let result = tokio::time::timeout_at(self.inner.deadline, result)
            .await
            .map_err(|_| refused("request deadline"))?
            .map_err(|_| Error::State)?;
        self.confirm().await?;
        result
    }
    /// Same-generation query connection is independently admitted and drained before return.
    pub async fn read<R: Record>(&self) -> Result<Batch<R>, Error> {
        self.query(|lease| Box::pin(async move { lease.read::<R>().await }))
            .await
    }
    pub async fn query<T: Send + 'static>(
        &self,
        work: impl for<'a> FnOnce(
            &'a mut GenerationLease,
        ) -> futures::future::BoxFuture<'a, Result<T, Error>>
        + Send
        + 'static,
    ) -> Result<T, Error> {
        self.confirm().await?;
        let retained = self.clone();
        let deadline = self.inner.deadline;
        let permit =
            tokio::time::timeout_at(deadline, self.inner.state.queries.clone().acquire_owned())
                .await
                .map_err(|_| refused("query admission deadline"))?
                .map_err(|_| Error::State)?;
        // The task owns query capacity and execution until actual cleanup, even if the caller
        // cancels or stops awaiting. A failed query is never transparently replayed.
        let query = tokio::spawn(async move {
            let _permit = permit;
            let mut lease = tokio::time::timeout_at(
                deadline,
                retained
                    .inner
                    .state
                    .reader
                    .pin(retained.generation(), retained.budget().clone()),
            )
            .await
            .map_err(|_| refused("query acquisition deadline"))??;
            lease.request_deadline(retained.remaining()?).await?;
            let result = tokio::time::timeout_at(deadline, work(&mut lease))
                .await
                .map_err(|_| refused("query deadline"));
            let cleanup = lease.release().await;
            cleanup?;
            retained.confirm().await?;
            result?
        });
        let result = query.await.map_err(|_| Error::State)?;
        self.confirm().await?;
        result
    }
}
impl GenerationLease {
    async fn request_deadline(&mut self, remaining: Duration) -> Result<(), Error> {
        let millis = remaining.as_millis().clamp(1, 30_000).to_string();
        sqlx::query("SELECT set_config('statement_timeout',$1,false)")
            .bind(millis)
            .execute(&mut *self.connection)
            .await?;
        Ok(())
    }
}
