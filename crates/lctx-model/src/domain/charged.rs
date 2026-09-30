//! Validator and index state charged to the attempt budget. Retained entries are admitted before
//! they are stored, and the reservation lives exactly as long as the owning check or index.
//! Cardinality alone never refuses; semantic work bounds remain with their owners.
use super::resources::{Reservation, ResourceBudget};
use super::{HeapSize, ModelError};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

/// Per-entry allowance for B-tree node headers, edges and partially filled nodes.
const NODE_ALLOWANCE: usize = 32;

/// One check's share of the attempt budget. The default is unbound and refuses to admit state,
/// so a check constructed without its budget cannot retain anything silently.
#[derive(Debug, Default)]
pub struct StateCharge {
    budget: Option<ResourceBudget>,
    owner: &'static str,
    reservation: Option<Box<dyn Reservation>>,
}
impl StateCharge {
    pub fn new(budget: &ResourceBudget, owner: &'static str) -> Self {
        Self {
            budget: Some(budget.clone()),
            owner,
            reservation: None,
        }
    }
    pub fn reserved(&self) -> usize {
        self.reservation.as_ref().map_or(0, |r| r.size())
    }
    pub fn budget(&self) -> Option<&ResourceBudget> {
        self.budget.as_ref()
    }
    pub fn owner(&self) -> &'static str {
        self.owner
    }
    pub fn grow(&mut self, bytes: usize) -> Result<(), ModelError> {
        let budget = self
            .budget
            .as_ref()
            .ok_or_else(|| ModelError::Invalid("validator state has no attempt budget".into()))?;
        match &mut self.reservation {
            Some(reservation) => {
                let size = reservation
                    .size()
                    .checked_add(bytes)
                    .ok_or_else(|| overflow(self.owner))?;
                reservation.try_resize(size)
            }
            None => {
                self.reservation = Some(budget.reserve(self.owner, bytes)?);
                Ok(())
            }
        }
    }
    pub fn release(&mut self, bytes: usize) {
        if let Some(reservation) = &mut self.reservation {
            // Shrinking never fails; the pool only refuses growth.
            let _ = reservation.try_resize(reservation.size().saturating_sub(bytes));
        }
    }
    /// Admit a value retained outside a charged container, such as an in-progress sequence.
    pub fn admit<T: HeapSize>(&mut self, value: &T) -> Result<usize, ModelError> {
        let bytes = size_of::<T>()
            .checked_add(value.heap_bytes())
            .ok_or_else(|| overflow(self.owner))?;
        self.grow(bytes).map(|()| bytes)
    }
}
fn overflow(owner: &'static str) -> ModelError {
    ModelError::Invalid(format!("{owner} state size overflow"))
}
fn entry<K: HeapSize, V: HeapSize>(key: &K, value: &V) -> usize {
    (size_of::<K>() + size_of::<V>() + NODE_ALLOWANCE)
        .saturating_add(key.heap_bytes())
        .saturating_add(value.heap_bytes())
}

/// An ordered map whose only growth paths reserve first. Reads go through `Deref`.
#[derive(Debug)]
pub struct ChargedMap<K, V>(BTreeMap<K, V>);
impl<K, V> Default for ChargedMap<K, V> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}
impl<K, V> Deref for ChargedMap<K, V> {
    type Target = BTreeMap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<K: Ord + HeapSize, V: HeapSize> ChargedMap<K, V> {
    pub fn insert(
        &mut self,
        charge: &mut StateCharge,
        key: K,
        value: V,
    ) -> Result<Option<V>, ModelError> {
        let key_heap = key.heap_bytes();
        charge.grow(entry(&key, &value))?;
        let previous = self.0.insert(key, value);
        if let Some(previous) = &previous {
            // The map keeps its original key; the replaced value and duplicate key are released.
            charge.release(
                (size_of::<K>() + size_of::<V>() + NODE_ALLOWANCE)
                    .saturating_add(key_heap)
                    .saturating_add(previous.heap_bytes()),
            );
        }
        Ok(previous)
    }
    /// Mutate the value at `key`, inserting its default first. Nested growth is charged right
    /// after `mutate` adds it; a refused charge fails the check.
    pub fn update<T>(
        &mut self,
        charge: &mut StateCharge,
        key: K,
        mutate: impl FnOnce(&mut V) -> T,
    ) -> Result<T, ModelError>
    where
        K: Clone,
        V: Default,
    {
        if !self.0.contains_key(&key) {
            self.insert(charge, key.clone(), V::default())?;
        }
        let value = self.0.get_mut(&key).expect("inserted above");
        let before = value.heap_bytes();
        let result = mutate(value);
        let after = value.heap_bytes();
        if after > before {
            charge.grow(after - before)?;
        } else {
            charge.release(before - after);
        }
        Ok(result)
    }
    pub fn remove(&mut self, charge: &mut StateCharge, key: &K) -> Option<V> {
        let (key, value) = self.0.remove_entry(key)?;
        charge.release(entry(&key, &value));
        Some(value)
    }
}

/// An ordered set whose only growth path reserves first. Reads go through `Deref`.
#[derive(Debug)]
pub struct ChargedSet<T>(BTreeSet<T>);
impl<T> Default for ChargedSet<T> {
    fn default() -> Self {
        Self(BTreeSet::new())
    }
}
impl<T> Deref for ChargedSet<T> {
    type Target = BTreeSet<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: Ord + HeapSize> ChargedSet<T> {
    pub fn insert(&mut self, charge: &mut StateCharge, value: T) -> Result<bool, ModelError> {
        let bytes = (size_of::<T>() + NODE_ALLOWANCE).saturating_add(value.heap_bytes());
        charge.grow(bytes)?;
        let inserted = self.0.insert(value);
        if !inserted {
            charge.release(bytes);
        }
        Ok(inserted)
    }
}

/// A growable sequence whose only growth path reserves first. Reads go through `Deref`.
#[derive(Debug)]
pub struct ChargedVec<T>(Vec<T>);
impl<T> Default for ChargedVec<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl<T> Deref for ChargedVec<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: HeapSize> ChargedVec<T> {
    pub fn push(&mut self, charge: &mut StateCharge, value: T) -> Result<(), ModelError> {
        // Charge the next capacity step before the vector allocates it.
        let additional = if self.0.len() == self.0.capacity() {
            self.0.capacity().max(4)
        } else {
            0
        };
        charge.grow(
            additional
                .saturating_mul(size_of::<T>())
                .saturating_add(value.heap_bytes()),
        )?;
        self.0.reserve_exact(additional);
        self.0.push(value);
        Ok(())
    }
    /// Remove the last value, releasing its heap charge; the vector keeps its charged capacity.
    pub fn take_last(&mut self, charge: &mut StateCharge) -> Option<T> {
        let value = self.0.pop()?;
        charge.release(value.heap_bytes());
        Some(value)
    }
    /// Move the values out; their charge is released and the caller owns their accounting.
    pub fn take(&mut self, charge: &mut StateCharge) -> Vec<T> {
        let values = std::mem::take(&mut self.0);
        charge.release(values.heap_bytes());
        values
    }
}
