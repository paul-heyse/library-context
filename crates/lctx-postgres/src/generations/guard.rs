//! One original canonical session shared by every prepared consumer. Loss is terminal.
use super::{Error, GenerationId, GenerationLease, GenerationReader};
use lctx_model::domain::resources::ResourceBudget;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub(super) struct State {
    pub(super) lease: tokio::sync::Mutex<Option<GenerationLease>>,
    generation: GenerationId,
    lost: AtomicBool,
    pub(super) cpu_slots: Arc<tokio::sync::Semaphore>,
    startup_jobs: tokio_util::task::TaskTracker,
    startup_admission: std::sync::Mutex<()>,
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
}
#[derive(Clone)]
pub struct GenerationGuard {
    pub(super) state: Arc<State>,
    owners: Arc<()>,
}
impl GenerationReader {
    pub async fn guard(
        &self,
        generation: GenerationId,
        budget: ResourceBudget,
    ) -> Result<GenerationGuard, Error> {
        // The original session has retained state before the first possibly blocked admission
        // query. Cancellation drops this reservation alongside its close-on-drop connection.
        // Conservative retained allowance for the semaphore/tracker Arc backing objects;
        // public handle sizes alone omit their heap metadata. This is not an RSS measurement.
        let charge = budget.reserve("generation-guard", std::mem::size_of::<State>() + 1024)?;
        let mut lease = self.pin(generation, budget).await?;
        lease.serving_shape().await?;
        let guard = GenerationGuard {
            state: Arc::new(State {
                lease: tokio::sync::Mutex::new(Some(lease)),
                generation,
                lost: AtomicBool::new(false),
                cpu_slots: Arc::new(tokio::sync::Semaphore::new(lctx_model::domain::serving::ResourceLimits::default().cpu_jobs as usize)),
                startup_jobs: tokio_util::task::TaskTracker::new(),
                startup_admission: std::sync::Mutex::new(()),
                _charge: charge,
            }),
            owners: Arc::new(()),
        };
        guard.check().await?;
        let weak = Arc::downgrade(&guard.state);
        // Weak supervision neither extends generation lifetime nor creates a replacement session.
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let Some(state) = weak.upgrade() else {
                    break;
                };
                let guard = GenerationGuard {
                    state,
                    owners: Arc::new(()),
                };
                if guard.check().await.is_err() {
                    break;
                }
            }
        });
        Ok(guard)
    }
}
impl GenerationGuard {
    pub fn generation(&self) -> GenerationId {
        self.state.generation
    }
    pub fn is_lost(&self) -> bool {
        self.state.lost.load(Ordering::Acquire)
    }
    pub async fn check(&self) -> Result<(), Error> {
        if self.is_lost() {
            return Err(Error::State);
        }
        let mut lease = self.state.lease.lock().await;
        if self.is_lost() {
            return Err(Error::State);
        }
        let result = match lease.as_mut() {
            Some(lease) => tokio::time::timeout(Duration::from_secs(2), lease.selection_live())
                .await
                .unwrap_or(Err(Error::State)),
            None => Err(Error::State),
        };
        if result.is_err() {
            self.state.lost.store(true, Ordering::Release);
        }
        result
    }
    /// Startup has its own admission wait and no request deadline. Once started, finite pure
    /// work retains its slot, preparation allowance and original guard through cancellation.
    pub async fn prepare_cpu<T: Send + 'static>(
        &self,
        work: impl FnOnce(&ResourceBudget) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        self.check().await?;
        let budget = self.state.lease.lock().await.as_ref().ok_or(Error::State)?.budget.clone();
        let job_charge = budget.reserve("startup-cpu-job", 1024)?;
        let slot = tokio::time::timeout(Duration::from_secs(30), self.state.cpu_slots.clone().acquire_owned())
            .await.map_err(|_| Error::ResourceRefused("startup CPU admission wait"))?
            .map_err(|_| Error::State)?;
        let job = {
            let _registration = self.state.startup_admission.lock().map_err(|_| Error::State)?;
            if self.is_lost() { return Err(Error::State); }
            let retained = self.clone();
            self.state.startup_jobs.spawn_blocking(move || {
                let _slot = slot;
                let _charge = job_charge;
                if retained.is_lost() { return Err(Error::State); }
                let result = work(&budget);
                drop(retained);
                result
            })
        };
        let result = job.await.map_err(|_| Error::State)?;
        self.check().await?;
        result
    }

    /// Sole-owner release remains available to minimal canonical consumers.
    pub async fn release(self) -> Result<(), Error> {
        // The weak supervisor may be checking transiently. Acquire the mutex before checking
        // references, so it cannot retain an idle strong reference while awaiting this lock.
        if Arc::strong_count(&self.owners) > 1 {
            return Err(Error::Busy);
        }
        self.close().await
    }
    /// Runtime shutdown calls this only after stopping admission and draining jobs/query cleanup.
    pub(super) async fn close(&self) -> Result<(), Error> {
        {
            let _registration = self.state.startup_admission.lock().map_err(|_| Error::State)?;
            self.state.lost.store(true, Ordering::Release);
            self.state.cpu_slots.close();
            self.state.startup_jobs.close();
        }
        self.state.startup_jobs.wait().await;
        let lease = self.state.lease.lock().await.take();
        match lease {
            Some(lease) => lease.release().await,
            None => Ok(()),
        }
    }
}
