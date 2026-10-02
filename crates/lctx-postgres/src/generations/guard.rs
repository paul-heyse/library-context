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
        let charge = budget.reserve("generation-guard", std::mem::size_of::<State>())?;
        let mut lease = self.pin(generation, budget).await?;
        lease.serving_shape().await?;
        let guard = GenerationGuard {
            state: Arc::new(State {
                lease: tokio::sync::Mutex::new(Some(lease)),
                generation,
                lost: AtomicBool::new(false),
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
        self.state.lost.store(true, Ordering::Release);
        let lease = self.state.lease.lock().await.take();
        match lease {
            Some(lease) => lease.release().await,
            None => Ok(()),
        }
    }
}
