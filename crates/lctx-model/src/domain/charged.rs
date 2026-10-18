//! State admitted before growth. Moves preserve accounting until the last actual owner drops.
use super::resources::{Reservation, ResourceBudget};
use super::{HeapSize, ModelError};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
const NODE_ALLOWANCE: usize = 32;
#[derive(Debug)]
struct ChargeBacking {
    reservation: Box<dyn Reservation>,
    metadata: Box<dyn Reservation>,
}
#[derive(Debug)]
struct ChargePart {
    backing: Arc<Mutex<ChargeBacking>>,
    bytes: usize,
}
impl Drop for ChargePart {
    fn drop(&mut self) {
        let mut backing = self.backing.lock().expect("charge owner");
        let size = backing.reservation.size();
        backing
            .reservation
            .try_resize(size - self.bytes)
            .expect("reservation shrink");
    }
}
/// An owner's share of the attempt pool. Payload reconciliation excludes separately owned
/// accounting metadata; `accounted_bytes` includes that metadata for lifetime observations.
#[derive(Debug, Default)]
pub struct StateCharge {
    budget: Option<ResourceBudget>,
    owner: &'static str,
    parts: Vec<ChargePart>,
    metadata: Option<Box<dyn Reservation>>,
}
impl StateCharge {
    pub fn new(budget: &ResourceBudget, owner: &'static str) -> Self {
        Self {
            budget: Some(budget.clone()),
            owner,
            parts: Vec::new(),
            metadata: None,
        }
    }
    pub fn reserved(&self) -> usize {
        self.parts.iter().map(|p| p.bytes).sum()
    }
    /// Includes this owner's vector and shared backing headers. A shared header is not additive
    /// across independent borrowers; the actual pool charges that header once until its last drop.
    pub fn accounted_bytes(&self) -> usize {
        self.reserved()
            .saturating_add(self.metadata.as_ref().map_or(0, |r| r.size()))
            .saturating_add(
                self.parts
                    .iter()
                    .map(|part| part.backing.lock().expect("charge owner").metadata.size())
                    .sum::<usize>(),
            )
    }
    pub fn budget(&self) -> Option<&ResourceBudget> {
        self.budget.as_ref()
    }
    pub fn owner(&self) -> &'static str {
        self.owner
    }
    fn prepare_slots(&mut self, slots: usize) -> Result<(), ModelError> {
        if self.parts.capacity() >= slots {
            return Ok(());
        }
        let budget = self
            .budget
            .as_ref()
            .ok_or_else(|| ModelError::Invalid("validator state has no attempt budget".into()))?;
        let capacity = slots.max(self.parts.capacity().saturating_mul(2)).max(4);
        let bytes = capacity
            .checked_mul(size_of::<ChargePart>())
            .ok_or_else(|| overflow(self.owner))?;
        match &mut self.metadata {
            Some(reservation) => reservation.try_resize(bytes)?,
            None => self.metadata = Some(budget.reserve("state charge fragment vector", bytes)?),
        }
        self.parts.reserve_exact(capacity - self.parts.len());
        Ok(())
    }
    pub fn grow(&mut self, bytes: usize) -> Result<(), ModelError> {
        if self.budget.is_none() {
            return Err(ModelError::Invalid(
                "validator state has no attempt budget".into(),
            ));
        }
        if bytes == 0 {
            return Ok(());
        }
        if let Some(part) = self.parts.first_mut() {
            let mut backing = part.backing.lock().expect("charge owner");
            let size = backing
                .reservation
                .size()
                .checked_add(bytes)
                .ok_or_else(|| overflow(self.owner))?;
            backing.reservation.try_resize(size)?;
            part.bytes += bytes;
        } else {
            self.prepare_slots(1)?;
            let budget = self.budget.as_ref().expect("bound owner");
            let reservation = budget.reserve(self.owner, bytes)?;
            let metadata = budget.reserve(
                "state charge shared backing",
                size_of::<Mutex<ChargeBacking>>() + 2 * size_of::<usize>(),
            )?;
            self.parts.push(ChargePart {
                backing: Arc::new(Mutex::new(ChargeBacking {
                    reservation,
                    metadata,
                })),
                bytes,
            });
        }
        Ok(())
    }
    pub fn release(&mut self, mut bytes: usize) {
        for part in self.parts.iter_mut().rev() {
            let take = bytes.min(part.bytes);
            let mut backing = part.backing.lock().expect("charge owner");
            let size = backing.reservation.size();
            backing
                .reservation
                .try_resize(size - take)
                .expect("reservation shrink");
            part.bytes -= take;
            bytes -= take;
            if bytes == 0 {
                break;
            }
        }
        self.parts.retain(|p| p.bytes != 0);
    }
    fn split(&mut self, mut bytes: usize) -> Result<Self, ModelError> {
        assert!(bytes <= self.reserved(), "transferred backing was admitted");
        let mut result = Self {
            budget: self.budget.clone(),
            owner: self.owner,
            parts: Vec::new(),
            metadata: None,
        };
        let mut needed = bytes;
        let slots = self
            .parts
            .iter()
            .rev()
            .take_while(|part| {
                if needed == 0 {
                    return false;
                }
                needed = needed.saturating_sub(part.bytes);
                true
            })
            .count();
        result.prepare_slots(slots)?;
        for part in self.parts.iter_mut().rev() {
            let take = bytes.min(part.bytes);
            if take != 0 {
                result.parts.push(ChargePart {
                    backing: part.backing.clone(),
                    bytes: take,
                });
                part.bytes -= take;
                bytes -= take;
            }
            if bytes == 0 {
                break;
            }
        }
        self.parts.retain(|p| p.bytes != 0);
        Ok(result)
    }
    fn incoming_parts(&self, other: &Self) -> usize {
        other
            .parts
            .iter()
            .filter(|incoming| {
                !self
                    .parts
                    .iter()
                    .any(|part| Arc::ptr_eq(&part.backing, &incoming.backing))
            })
            .count()
    }
    /// Admit accounting capacity before a native effect or backing move. Actual retained capacity
    /// remains charged if later work fails. This method moves no payload reservation.
    pub fn prepare_absorb(&mut self, other: &Self) -> Result<(), ModelError> {
        if !self
            .budget
            .as_ref()
            .zip(other.budget.as_ref())
            .is_some_and(|(a, b)| a.shares_pool(b))
        {
            return Err(ModelError::Conflict("charged value foreign receiving pool"));
        }
        self.prepare_slots(
            self.parts
                .len()
                .checked_add(self.incoming_parts(other))
                .ok_or_else(|| overflow(self.owner))?,
        )
    }
    /// Move pre-admitted accounting without allocation or a reserve/release gap. Call
    /// `prepare_absorb` before mandatory effects; capacity mismatch refuses before any move.
    pub fn absorb(&mut self, mut other: Self) -> Result<(), ModelError> {
        if !self
            .budget
            .as_ref()
            .zip(other.budget.as_ref())
            .is_some_and(|(a, b)| a.shares_pool(b))
        {
            return Err(ModelError::Conflict("charged value foreign receiving pool"));
        }
        if self.parts.capacity() < self.parts.len() + self.incoming_parts(&other) {
            return Err(ModelError::Conflict(
                "charged transfer metadata was not prepared",
            ));
        }
        for mut incoming in other.parts.drain(..) {
            if let Some(existing) = self
                .parts
                .iter_mut()
                .find(|part| Arc::ptr_eq(&part.backing, &incoming.backing))
            {
                existing.bytes += incoming.bytes;
                incoming.bytes = 0;
            } else {
                self.parts.push(incoming);
            }
        }
        Ok(())
    }
    pub fn admit<T: HeapSize>(&mut self, value: &T) -> Result<usize, ModelError> {
        let bytes = size_of::<T>()
            .checked_add(value.heap_bytes())
            .ok_or_else(|| overflow(self.owner))?;
        self.grow(bytes).map(|()| bytes)
    }
}
impl Reservation for StateCharge {
    fn size(&self) -> usize {
        self.reserved()
    }
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
        let current = self.reserved();
        if bytes > current {
            self.grow(bytes - current)
        } else {
            self.release(current - bytes);
            Ok(())
        }
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
/// Removed/replaced backing stays admitted until dropped or transferred to another owner.
#[derive(Debug)]
pub struct ChargedValue<T> {
    value: Option<T>,
    charge: StateCharge,
}
impl<T> Deref for ChargedValue<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value.as_ref().expect("owned value")
    }
}
impl<T: PartialEq> PartialEq<T> for ChargedValue<T> {
    fn eq(&self, other: &T) -> bool {
        **self == *other
    }
}
impl<T> ChargedValue<T> {
    pub fn accounted_bytes(&self) -> usize {
        self.charge.accounted_bytes()
    }
    pub fn into_inner(mut self, receiving: &mut StateCharge) -> Result<T, ModelError> {
        if !receiving
            .budget()
            .zip(self.charge.budget())
            .is_some_and(|(a, b)| a.shares_pool(b))
        {
            return Err(ModelError::Conflict("charged value foreign receiving pool"));
        }
        receiving.prepare_absorb(&self.charge)?;
        receiving.absorb(std::mem::take(&mut self.charge))?;
        Ok(self.value.take().expect("owned value"))
    }
}
/// Fixed-layout state whose mutations cannot allocate owned heap.
pub(crate) mod inline_state {
    pub trait Sealed {}
}
pub trait InlineState: inline_state::Sealed + HeapSize + Default {}
impl inline_state::Sealed for usize {}
impl inline_state::Sealed for i64 {}
impl inline_state::Sealed for (usize, usize) {}
impl InlineState for usize {}
impl InlineState for i64 {}
impl InlineState for (usize, usize) {}
pub(crate) mod empty_state {
    pub trait Sealed {}
}
/// Audited default states with no owned allocation. External mutation implementations cannot
/// introduce an allocating default before an entry's reservation is admitted.
pub trait EmptyState: empty_state::Sealed + HeapSize + Default {}
impl<T> empty_state::Sealed for Vec<T> {}
impl<T: HeapSize> EmptyState for Vec<T> {}
impl<T> empty_state::Sealed for BTreeSet<T> {}
impl<T: HeapSize> EmptyState for BTreeSet<T> {}
impl<K, V> empty_state::Sealed for BTreeMap<K, V> {}
impl<K: HeapSize, V: HeapSize> EmptyState for BTreeMap<K, V> {}
/// A typed mutation computes its complete growth allowance before changing any state.
/// `additional` must not allocate owned backing; `apply` stays within that allowance.
/// Only copied outcomes escape: removed owned backing uses `ChargedValue` instead.
pub trait AdmittedMutation<V> {
    type Output: Copy;
    fn additional(&self, value: &V) -> Result<usize, ModelError>;
    fn apply(self, value: &mut V) -> Self::Output;
}
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
        c: &mut StateCharge,
        k: K,
        v: V,
    ) -> Result<Option<ChargedValue<V>>, ModelError> {
        if let Some(previous) = self.0.get(&k) {
            let old = previous.heap_bytes();
            let old_charge = c.split(old)?;
            if let Err(error) = c.grow(v.heap_bytes()) {
                c.prepare_absorb(&old_charge)?;
                c.absorb(old_charge)?;
                return Err(error);
            }
            let previous = self.0.insert(k, v).expect("existing value");
            return Ok(Some(ChargedValue {
                value: Some(previous),
                charge: old_charge,
            }));
        }
        c.grow(entry(&k, &v))?;
        self.0.insert(k, v);
        Ok(None)
    }
    pub fn insert_owned(
        &mut self,
        c: &mut StateCharge,
        k: K,
        mut v: ChargedValue<V>,
    ) -> Result<Option<ChargedValue<V>>, ModelError> {
        if !c
            .budget()
            .zip(v.charge.budget())
            .is_some_and(|(a, b)| a.shares_pool(b))
        {
            return Err(ModelError::Conflict("charged value foreign receiving pool"));
        }
        let base = if self.0.contains_key(&k) {
            0
        } else {
            size_of::<K>() + size_of::<V>() + NODE_ALLOWANCE + k.heap_bytes()
        };
        c.prepare_slots(
            c.parts.len()
                + c.incoming_parts(&v.charge)
                + usize::from(c.parts.is_empty() && base > 0),
        )?;
        c.grow(base)?;
        let old_charge = self
            .0
            .get(&k)
            .map(|value| c.split(value.heap_bytes()))
            .transpose()?;
        c.absorb(std::mem::take(&mut v.charge))?;
        let previous = self.0.insert(k, v.value.take().expect("owned value"));
        Ok(previous.map(|value| ChargedValue {
            value: Some(value),
            charge: old_charge.expect("admitted replaced backing"),
        }))
    }
    pub fn remove(
        &mut self,
        c: &mut StateCharge,
        k: &K,
    ) -> Result<Option<ChargedValue<V>>, ModelError> {
        let Some(value) = self.0.get(k) else {
            return Ok(None);
        };
        let owned = c.split(value.heap_bytes())?;
        let (k, v) = self.0.remove_entry(k).expect("observed value");
        c.release(size_of::<K>() + size_of::<V>() + NODE_ALLOWANCE + k.heap_bytes());
        Ok(Some(ChargedValue {
            value: Some(v),
            charge: owned,
        }))
    }
    pub fn mutate<M: AdmittedMutation<V>>(
        &mut self,
        c: &mut StateCharge,
        k: K,
        m: M,
    ) -> Result<M::Output, ModelError>
    where
        V: EmptyState,
    {
        let default;
        let (growth, missing) = match self.0.get(&k) {
            Some(v) => (m.additional(v)?, false),
            None => {
                default = V::default();
                (m.additional(&default)?, true)
            }
        };
        let base = if missing { entry(&k, &V::default()) } else { 0 };
        c.grow(
            base.checked_add(growth)
                .ok_or_else(|| overflow(c.owner()))?,
        )?;
        let v = self.0.entry(k).or_default();
        let before = v.heap_bytes();
        let output = m.apply(v);
        let after = v.heap_bytes();
        assert!(
            after <= before.saturating_add(growth),
            "typed mutation exceeded its pre-admitted footprint"
        );
        c.release(before.saturating_add(growth).saturating_sub(after));
        Ok(output)
    }
    pub fn update_inline<T>(
        &mut self,
        c: &mut StateCharge,
        k: K,
        m: impl FnOnce(&mut V) -> T,
    ) -> Result<T, ModelError>
    where
        V: InlineState,
    {
        if !self.0.contains_key(&k) {
            c.grow(entry(&k, &V::default()))?;
        }
        Ok(m(self.0.entry(k).or_default()))
    }
}
struct Truncate(usize);
impl<T: HeapSize> AdmittedMutation<Vec<T>> for Truncate {
    type Output = ();
    fn additional(&self, _: &Vec<T>) -> Result<usize, ModelError> {
        Ok(0)
    }
    fn apply(self, v: &mut Vec<T>) {
        v.truncate(self.0);
    }
}
struct Push<T>(T);
impl<T: HeapSize> AdmittedMutation<Vec<T>> for Push<T> {
    type Output = ();
    fn additional(&self, v: &Vec<T>) -> Result<usize, ModelError> {
        Ok((if v.len() == v.capacity() {
            v.capacity().max(4).saturating_mul(size_of::<T>())
        } else {
            0
        })
        .saturating_add(self.0.heap_bytes()))
    }
    fn apply(self, v: &mut Vec<T>) {
        if v.len() == v.capacity() {
            v.reserve_exact(v.capacity().max(4));
        }
        v.push(self.0);
    }
}
impl<K: Ord + HeapSize, T: HeapSize> ChargedMap<K, Vec<T>> {
    pub fn push(&mut self, c: &mut StateCharge, k: K, v: T) -> Result<(), ModelError> {
        self.mutate(c, k, Push(v))
    }
    pub fn truncate(&mut self, c: &mut StateCharge, k: K, len: usize) -> Result<(), ModelError> {
        self.mutate(c, k, Truncate(len))
    }
    pub fn push_unique(&mut self, c: &mut StateCharge, k: K, v: T) -> Result<(), ModelError>
    where
        T: PartialEq,
    {
        if self.0.get(&k).is_some_and(|rows| rows.contains(&v)) {
            return Ok(());
        }
        self.push(c, k, v)
    }
    pub fn push_ordered<B: Ord + Copy>(
        &mut self,
        c: &mut StateCharge,
        k: K,
        v: T,
        by: impl Fn(&T) -> B,
    ) -> Result<(), ModelError>
    where
        K: Copy,
    {
        let at = self
            .0
            .get(&k)
            .map_or(0, |rows| rows.partition_point(|row| by(row) <= by(&v)));
        self.push(c, k, v)?;
        let rows = self.0.get_mut(&k).expect("admitted sequence");
        rows[at..].rotate_right(1);
        Ok(())
    }
}
struct SetInsert<T>(T);
impl<T: Ord + HeapSize> AdmittedMutation<BTreeSet<T>> for SetInsert<T> {
    type Output = bool;
    fn additional(&self, v: &BTreeSet<T>) -> Result<usize, ModelError> {
        Ok(if v.contains(&self.0) {
            0
        } else {
            size_of::<T>() + 16 + self.0.heap_bytes()
        })
    }
    fn apply(self, v: &mut BTreeSet<T>) -> bool {
        v.insert(self.0)
    }
}
impl<K: Ord + HeapSize, T: Ord + HeapSize> ChargedMap<K, BTreeSet<T>> {
    pub fn insert_set(&mut self, c: &mut StateCharge, k: K, v: T) -> Result<bool, ModelError> {
        self.mutate(c, k, SetInsert(v))
    }
}
impl<K: Ord + HeapSize, A: Ord + HeapSize, B: HeapSize> ChargedMap<K, BTreeMap<A, B>> {
    pub fn insert_map(
        &mut self,
        c: &mut StateCharge,
        k: K,
        a: A,
        b: B,
    ) -> Result<Option<ChargedValue<B>>, ModelError> {
        let base = if self.0.contains_key(&k) {
            0
        } else {
            entry(&k, &BTreeMap::<A, B>::new())
        };
        let previous = self.0.get(&k).and_then(|v| v.get(&a));
        let bytes = if previous.is_some() {
            b.heap_bytes()
        } else {
            size_of::<(A, B)>() + 16 + a.heap_bytes() + b.heap_bytes()
        };
        let old_charge = previous
            .map(|value| c.split(value.heap_bytes()))
            .transpose()?;
        if let Err(error) = c.grow(base.checked_add(bytes).ok_or_else(|| overflow(c.owner()))?) {
            if let Some(old) = old_charge {
                c.prepare_absorb(&old)?;
                c.absorb(old)?;
            }
            return Err(error);
        }
        let previous = self.0.entry(k).or_default().insert(a, b);
        Ok(previous.map(|v| ChargedValue {
            value: Some(v),
            charge: old_charge.expect("admitted replaced backing"),
        }))
    }
}
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
    pub fn insert(&mut self, c: &mut StateCharge, v: T) -> Result<bool, ModelError> {
        if self.0.contains(&v) {
            return Ok(false);
        }
        c.grow(size_of::<T>() + NODE_ALLOWANCE + v.heap_bytes())?;
        Ok(self.0.insert(v))
    }
}
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
    pub fn push(&mut self, c: &mut StateCharge, v: T) -> Result<(), ModelError> {
        let op = Push(v);
        c.grow(op.additional(&self.0)?)?;
        op.apply(&mut self.0);
        Ok(())
    }
    pub fn take_last(
        &mut self,
        c: &mut StateCharge,
    ) -> Result<Option<ChargedValue<T>>, ModelError> {
        let Some(value) = self.0.last() else {
            return Ok(None);
        };
        let charge = c.split(value.heap_bytes())?;
        let value = self.0.pop().expect("observed value");
        Ok(Some(ChargedValue {
            value: Some(value),
            charge,
        }))
    }
    pub fn take(&mut self, c: &mut StateCharge) -> Result<ChargedValue<Vec<T>>, ModelError> {
        let charge = c.split(self.0.heap_bytes())?;
        let value = std::mem::take(&mut self.0);
        Ok(ChargedValue {
            value: Some(value),
            charge,
        })
    }
}
