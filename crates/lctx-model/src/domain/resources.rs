//! Shared, library-neutral allocation reservations. These account admitted buffers, not process RSS.
use super::ModelError;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
pub const DEFAULT_MEMORY_BYTES: usize = 64 * 1024 * 1024 * 1024;
pub const DEFAULT_PARTITIONS: usize = 8;
pub const TRANSFER_ROWS: usize = 4096;
pub const TRANSFER_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ROW_BYTES: usize = 64 * 1024 * 1024;

/// An execution library may supply its own pool. Reservations must be returned on drop;
/// a failed resize leaves the previous reservation intact.
pub trait ResourcePool: std::fmt::Debug + Send + Sync {
    fn reserve(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn Reservation>, ModelError>;
    fn reserved(&self) -> usize;
    fn limit(&self) -> usize;
    /// The highest reservation total so far, where the pool tracks it.
    fn peak(&self) -> Option<usize> {
        None
    }
    fn reset_stage_peak(&self) {}
    fn stage_peak(&self) -> Option<usize> {
        None
    }
}
pub trait Reservation: std::fmt::Debug + Send + Sync {
    fn size(&self) -> usize;
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError>;
}
#[derive(Debug, Clone)]
pub struct ResourceBudget(Arc<dyn ResourcePool>);
impl ResourceBudget {
    /// A local ceiling whose every retained byte is also charged to the parent pool.
    /// Preparation and request budgets cannot independently promise the process allowance.
    pub fn scoped(parent: &Self, limit: usize) -> Result<Self, ModelError> {
        let local = Self::fixed(limit)?;
        Self::from_pool(Arc::new(ScopedPool { parent: parent.clone(), local }))
    }
    pub fn fixed(limit: usize) -> Result<Self, ModelError> {
        if limit == 0 {
            return Err(ModelError::Invalid("memory limit must be positive".into()));
        }
        Ok(Self(Arc::new(FixedPool(Arc::new(FixedState {
            limit,
            used: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        })))))
    }
    pub fn from_pool(pool: Arc<dyn ResourcePool>) -> Result<Self, ModelError> {
        if pool.limit() == 0 {
            return Err(ModelError::Invalid("memory limit must be positive".into()));
        }
        Ok(Self(pool))
    }
    pub fn reserve(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn Reservation>, ModelError> {
        self.0.reserve(owner, bytes)
    }
    pub fn reserved(&self) -> usize {
        self.0.reserved()
    }
    pub fn limit(&self) -> usize {
        self.0.limit()
    }
    pub fn peak(&self) -> Option<usize> {
        self.0.peak()
    }
    pub fn reset_stage_peak(&self) {
        self.0.reset_stage_peak();
    }
    pub fn stage_peak(&self) -> Option<usize> {
        self.0.stage_peak()
    }
    pub fn shares_pool(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
#[derive(Debug)]
struct ScopedPool {
    parent: ResourceBudget,
    local: ResourceBudget,
}
#[derive(Debug)]
struct ScopedReservation {
    parent: Box<dyn Reservation>,
    local: Box<dyn Reservation>,
}
impl ResourcePool for ScopedPool {
    fn reserve(&self, owner: &'static str, bytes: usize) -> Result<Box<dyn Reservation>, ModelError> {
        let mut reservation = ScopedReservation {
            parent: self.parent.reserve(owner, 0)?,
            local: self.local.reserve(owner, 0)?,
        };
        reservation.try_resize(bytes)?;
        Ok(Box::new(reservation))
    }
    fn reserved(&self) -> usize { self.local.reserved() }
    fn limit(&self) -> usize { self.local.limit() }
    fn peak(&self) -> Option<usize> { self.local.peak() }
}
impl Reservation for ScopedReservation {
    fn size(&self) -> usize { self.local.size() }
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
        let old = self.local.size();
        self.local.try_resize(bytes)?;
        if let Err(error) = self.parent.try_resize(bytes) {
            // Returning a just-acquired local increment always fits; shrink is infallible.
            self.local.try_resize(old).expect("restore local reservation after parent refusal");
            return Err(error);
        }
        Ok(())
    }
}
#[derive(Debug)]
struct FixedState {
    limit: usize,
    used: AtomicUsize,
    peak: AtomicUsize,
}
#[derive(Debug)]
struct FixedPool(Arc<FixedState>);
#[derive(Debug)]
struct FixedReservation {
    state: Arc<FixedState>,
    size: usize,
    owner: &'static str,
}
impl ResourcePool for FixedPool {
    fn reserve(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn Reservation>, ModelError> {
        let mut reservation = FixedReservation {
            state: self.0.clone(),
            size: 0,
            owner,
        };
        reservation.try_resize(bytes)?;
        Ok(Box::new(reservation))
    }
    fn reserved(&self) -> usize {
        self.0.used.load(Ordering::Relaxed)
    }
    fn limit(&self) -> usize {
        self.0.limit
    }
    fn peak(&self) -> Option<usize> {
        Some(self.0.peak.load(Ordering::Relaxed))
    }
}
impl Reservation for FixedReservation {
    fn size(&self) -> usize {
        self.size
    }
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
        if bytes > self.size {
            let additional = bytes - self.size;
            let previous = self
                .state
                .used
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                    used.checked_add(additional)
                        .filter(|new| *new <= self.state.limit)
                })
                .map_err(|used| ModelError::Resource {
                    owner: self.owner,
                    requested: additional,
                    used,
                    limit: self.state.limit,
                })?;
            self.state
                .peak
                .fetch_max(previous + additional, Ordering::Relaxed);
        } else {
            self.state
                .used
                .fetch_sub(self.size - bytes, Ordering::Relaxed);
        }
        self.size = bytes;
        Ok(())
    }
}
impl Drop for FixedReservation {
    fn drop(&mut self) {
        self.state.used.fetch_sub(self.size, Ordering::Relaxed);
    }
}
