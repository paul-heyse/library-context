//! Transfer-bounded typed batch writing. Rows are admitted against the attempt budget before
//! they are held; a flushed batch takes over the reservation of the rows it contains.
use std::collections::HashMap;
use super::{Batch, ContentHash, Id, ModelError, Record, ValidatedModel};
use super::record::fixed_width;
use super::resources::{Reservation, ResourceBudget, MAX_ROW_BYTES, TRANSFER_BYTES, TRANSFER_ROWS};

/// Row and byte targets for one transfer batch, and the largest admissible single row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferLimits { pub rows: usize, pub bytes: usize, pub max_row: usize }
impl Default for TransferLimits {
    fn default() -> Self { Self { rows: TRANSFER_ROWS, bytes: TRANSFER_BYTES, max_row: MAX_ROW_BYTES } }
}

/// Accumulates one relation's rows into transfer-sized batches. Equal rows repeated across flushes
/// are emitted once; the same identity with a different payload is a conflict. An admitted row
/// larger than the byte target travels alone; one above `max_row` is refused.
#[derive(Debug)]
pub struct BatchWriter<R: Record> {
    budget: ResourceBudget, limits: TransferLimits, fixed: usize,
    pending: Vec<R>, pending_bytes: usize, pending_heap: usize, held: Box<dyn Reservation>,
    seen: HashMap<Id<R>, ContentHash>, seen_held: Box<dyn Reservation>,
}
impl<R: Record> BatchWriter<R> {
    pub fn new(budget: &ResourceBudget, limits: TransferLimits) -> Result<Self, ModelError> {
        if limits.rows == 0 || limits.bytes == 0 || limits.bytes > limits.max_row {
            return Err(ModelError::Invalid("transfer limits need positive targets within the row limit".into()));
        }
        Ok(Self { budget: budget.clone(), limits, fixed: fixed_width::<R>(),
            pending: Vec::new(), pending_bytes: 0, pending_heap: 0, held: budget.reserve(R::NAME, 0)?,
            seen: HashMap::new(), seen_held: budget.reserve(R::NAME, 0)? })
    }
    pub fn budget(&self) -> &ResourceBudget { &self.budget }
    /// Admit one row. Returns the previously pending batch when this row would exceed a target.
    pub fn push(&mut self, model: &ValidatedModel, row: R) -> Result<Option<Batch<R>>, ModelError> {
        model.require::<R>()?;
        row.validate()?;
        let heap = row.heap_bytes();
        let encoded = self.fixed.checked_add(heap).ok_or_else(|| overflow::<R>())?;
        if encoded > self.limits.max_row {
            return Err(ModelError::Invalid(format!("{} row of {encoded} bytes exceeds the {} byte row limit", R::NAME, self.limits.max_row)));
        }
        let id = row.id();
        let digest = row.content_digest();
        match self.seen.get(&id) {
            Some(existing) if *existing == digest => return Ok(None),
            Some(_) => return Err(ModelError::Conflict(R::NAME)),
            None => {},
        }
        let flushed = if !self.pending.is_empty()
            && (self.pending.len() >= self.limits.rows || self.pending_bytes.saturating_add(encoded) > self.limits.bytes) {
            Some(self.flush(model)?)
        } else { None };
        if self.seen.len() == self.seen.capacity() {
            // Hash tables round buckets up and add control bytes; charge twice the entry payload.
            let capacity = self.seen.capacity().saturating_mul(2).max(64);
            let entry = size_of::<(Id<R>, ContentHash)>() + 1;
            self.seen_held.try_resize(capacity.checked_mul(entry).and_then(|b| b.checked_mul(2)).ok_or_else(|| overflow::<R>())?)?;
            self.seen.reserve(capacity - self.seen.len());
        }
        let capacity = if self.pending.len() == self.pending.capacity() {
            self.pending.capacity().saturating_mul(2).max(16).min(self.limits.rows)
        } else { self.pending.capacity() };
        let heap_total = self.pending_heap.checked_add(heap).ok_or_else(|| overflow::<R>())?;
        self.held.try_resize(capacity.checked_mul(size_of::<R>()).and_then(|b| b.checked_add(heap_total)).ok_or_else(|| overflow::<R>())?)?;
        self.pending.reserve_exact(capacity - self.pending.len());
        self.seen.insert(id, digest);
        self.pending.push(row);
        self.pending_heap = heap_total;
        self.pending_bytes += encoded;
        Ok(flushed)
    }
    /// Emit the remaining rows, if any. The duplicate index and its reservation end here.
    pub fn finish(mut self, model: &ValidatedModel) -> Result<Option<Batch<R>>, ModelError> {
        if self.pending.is_empty() { return Ok(None); }
        self.flush(model).map(Some)
    }
    fn flush(&mut self, model: &ValidatedModel) -> Result<Batch<R>, ModelError> {
        let rows = std::mem::take(&mut self.pending);
        let held = std::mem::replace(&mut self.held, self.budget.reserve(R::NAME, 0)?);
        self.pending_bytes = 0;
        self.pending_heap = 0;
        Batch::with_reservation(model, rows, held)
    }
}
fn overflow<R: Record>() -> ModelError { ModelError::Invalid(format!("{} batch size overflow", R::NAME)) }
