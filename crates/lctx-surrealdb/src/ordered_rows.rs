//! Attempt-owned external ordering of physical rows and compact nominal candidates.
use lctx_model::domain::{ModelError, resources::{Reservation, ResourceBudget}};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    mem::size_of,
    path::Path,
};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue, Value, ToSql};
use tempfile::{NamedTempFile, TempDir};

pub const ROW_BYTES: usize = 1024 * 1024;
const RUN_BYTES: usize = 1024 * 1024;
const IO_BYTES: usize = 8192;
const OWNER: &str = "native-ordered-candidates";

/// A complete relation-qualified nominal identity and its actual stored backing pointer.
/// Scope branches and exact contributors may repeat this tuple, but cannot disagree on node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub relation: String,
    pub key: [u8; 16],
    pub node: RecordId,
}

#[derive(Clone, Copy, Default)]
struct FrameBounds {
    frame: usize,
    work: usize,
}
struct Entry<K> {
    key: K,
    bytes: Vec<u8>,
    work_bytes: usize,
}
trait Adapter {
    type Key: Ord;
    type Output;
    const CONFLICT: &'static str;
    fn key(value: &Self::Output) -> Result<Self::Key, ModelError>;
    fn encode(value: &Self::Output) -> Result<Vec<u8>, ModelError>;
    fn decode(bytes: &[u8]) -> Result<Self::Output, ModelError>;
    fn key_bytes(key: &Self::Key) -> usize;
    fn work_bytes(value: &Self::Output, frame: usize) -> usize;
}
struct Physical;
impl Adapter for Physical {
    type Key = RecordId;
    type Output = Value;
    const CONFLICT: &'static str = "conflicting physical row identity";
    fn key(value: &Value) -> Result<RecordId, ModelError> {
        let Value::Object(object) = value else {
            return Err(ModelError::Schema("ordered physical row"));
        };
        RecordId::from_value(object.get("id").ok_or(ModelError::Schema("ordered row identity"))?.clone())
            .map_err(ModelError::codec)
    }
    fn encode(value: &Value) -> Result<Vec<u8>, ModelError> {
        serde_json::to_vec(value).map_err(ModelError::codec)
    }
    fn decode(bytes: &[u8]) -> Result<Value, ModelError> {
        serde_json::from_slice(bytes).map_err(ModelError::codec)
    }
    fn key_bytes(key: &RecordId) -> usize {
        let bytes=key.to_sql().len();
        match &key.key {RecordIdKey::String(_)|RecordIdKey::Number(_)|RecordIdKey::Uuid(_)=>bytes,_=>bytes.saturating_mul(2*size_of::<Value>()+4)}
    }
    fn work_bytes(_: &Value, frame: usize) -> usize { frame.saturating_mul(2 * size_of::<Value>() + 4) + size_of::<Entry<RecordId>>() }
}
struct Compact;
impl Adapter for Compact {
    type Key = (String, [u8; 16]);
    type Output = Candidate;
    const CONFLICT: &'static str = "conflicting nominal candidate pointer";
    fn key(value: &Candidate) -> Result<Self::Key, ModelError> {
        if value.relation.is_empty() { return Err(ModelError::Schema("candidate relation")); }
        Ok((value.relation.clone(), value.key))
    }
    fn encode(value: &Candidate) -> Result<Vec<u8>, ModelError> {
        serde_json::to_vec(value).map_err(ModelError::codec)
    }
    fn decode(bytes: &[u8]) -> Result<Candidate, ModelError> {
        serde_json::from_slice(bytes).map_err(ModelError::codec)
    }
    fn key_bytes(key: &Self::Key) -> usize { key.0.capacity() }
    fn work_bytes(value: &Candidate, frame: usize) -> usize {
        // Encoded frame, decoded output strings, and the independently retained key.
        // Native scalar/string pointers need no container expansion. For SDK composite
        // pointers, allow geometrically allocated Value containers per serialized byte.
        let factor = match &value.node.key {
            RecordIdKey::String(_) | RecordIdKey::Number(_) | RecordIdKey::Uuid(_) => 4,
            _ => 2 * size_of::<Value>() + 4,
        };
        frame.saturating_mul(factor) + size_of::<Candidate>() + size_of::<Entry<Self::Key>>()
    }
}
fn entry<A: Adapter>(value: A::Output) -> Result<Entry<A::Key>, ModelError> {
    let key = A::key(&value)?;
    let bytes = A::encode(&value)?;
    if bytes.len() > ROW_BYTES {
        return Err(ModelError::Limit {
            owner: "native-ordered-rows", limit: "row bytes", observed: bytes.len(), bound: ROW_BYTES,
        });
    }
    let work_bytes = A::work_bytes(&value, bytes.len());
    Ok(Entry { key, bytes, work_bytes })
}
fn write<K>(writer: &mut impl Write, entry: &Entry<K>) -> Result<(), ModelError> {
    writer.write_all(&(entry.bytes.len() as u64).to_le_bytes()).map_err(ModelError::codec)?;
    writer.write_all(&entry.bytes).map_err(ModelError::codec)
}
fn read<A: Adapter>(reader: &mut impl Read, bounds: FrameBounds) -> Result<Option<Entry<A::Key>>, ModelError> {
    let mut length = [0; 8];
    if reader.read(&mut length[..1]).map_err(ModelError::codec)? == 0 { return Ok(None); }
    reader.read_exact(&mut length[1..]).map_err(ModelError::codec)?;
    let length = usize::try_from(u64::from_le_bytes(length)).map_err(ModelError::codec)?;
    if length > ROW_BYTES || length > bounds.frame { return Err(ModelError::Schema("ordered row frame bound")); }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(ModelError::codec)?;
    // Preserve the original frame rather than reserializing it during every merge.
    let value = A::decode(&bytes)?;
    let work_bytes = A::work_bytes(&value, bytes.len());
    if work_bytes > bounds.work { return Err(ModelError::Schema("ordered row decode bound")); }
    let key = A::key(&value)?;
    Ok(Some(Entry { key, bytes, work_bytes }))
}
fn reserve(budget: Option<&ResourceBudget>, bytes: usize) -> Result<Option<Box<dyn Reservation>>, ModelError> {
    budget.map(|budget| budget.reserve(OWNER, bytes)).transpose()
}
fn merge<A: Adapter>(directory: &Path, a: NamedTempFile, b: NamedTempFile, budget: Option<&ResourceBudget>, bounds: FrameBounds) -> Result<NamedTempFile, ModelError> {
    // Two live heads and one transient decode, sized from admitted compact frames,
    // plus fixed reader/writer buffers. Small candidates do not pay the full-row bound.
    let _work = reserve(budget, 3 * bounds.work + 3 * IO_BYTES)?;
    let output = NamedTempFile::new_in(directory).map_err(ModelError::codec)?;
    let mut writer = BufWriter::with_capacity(IO_BYTES, output.reopen().map_err(ModelError::codec)?);
    let mut ar = BufReader::with_capacity(IO_BYTES, a.reopen().map_err(ModelError::codec)?);
    let mut br = BufReader::with_capacity(IO_BYTES, b.reopen().map_err(ModelError::codec)?);
    let mut av = read::<A>(&mut ar, bounds)?;
    let mut bv = read::<A>(&mut br, bounds)?;
    while av.is_some() || bv.is_some() {
        let (take_a, duplicate) = match (&av, &bv) {
            (Some(a), Some(b)) if a.key == b.key => {
                if a.bytes != b.bytes { return Err(ModelError::Conflict(A::CONFLICT)); }
                (true, true)
            }
            (Some(a), Some(b)) => (a.key < b.key, false),
            (Some(_), None) => (true, false),
            _ => (false, false),
        };
        if duplicate {
            drop(bv.take());
            bv = read::<A>(&mut br, bounds)?;
        }
        if take_a {
            write(&mut writer, av.as_ref().expect("merge source"))?;
            drop(av.take());
            av = read::<A>(&mut ar, bounds)?;
        } else {
            write(&mut writer, bv.as_ref().expect("merge source"))?;
            drop(bv.take());
            bv = read::<A>(&mut br, bounds)?;
        }
    }
    writer.flush().map_err(ModelError::codec)?;
    Ok(output)
}
struct Sorter<A: Adapter> {
    directory: TempDir,
    pending: Vec<Entry<A::Key>>,
    payload_bytes: usize,
    limit: usize,
    // Binary carry merging: logarithmic descriptors and two live heads, never a seen set.
    runs: Vec<Option<NamedTempFile>>,
    budget: Option<ResourceBudget>,
    retained: Option<Box<dyn Reservation>>,
    poisoned: bool,
    bounds: FrameBounds,
}
impl<A: Adapter> Sorter<A> {
    fn new(limit: usize, budget: Option<ResourceBudget>) -> Result<Self, ModelError> {
        if limit == 0 || limit > RUN_BYTES { return Err(ModelError::Schema("external run byte limit")); }
        let retained = reserve(budget.as_ref(), size_of::<Self>())?;
        let mut sorter = Self { directory: tempfile::tempdir().map_err(ModelError::codec)?, pending: Vec::new(), payload_bytes: 0, limit, runs: Vec::new(), budget, retained, poisoned: false, bounds: FrameBounds::default() };
        sorter.charge(0, 0)?;
        Ok(sorter)
    }
    fn metadata_bytes(&self, capacity: usize) -> usize {
        size_of::<Self>() + self.directory.path().as_os_str().len() + capacity * (size_of::<Option<NamedTempFile>>() + self.directory.path().as_os_str().len() + 64)
    }
    fn charge(&mut self, pending: usize, run_capacity: usize) -> Result<(), ModelError> {
        let bytes = pending.saturating_add(self.metadata_bytes(run_capacity));
        if let Some(retained) = &mut self.retained { retained.try_resize(bytes)?; }
        Ok(())
    }
    fn push(&mut self, value: A::Output) -> Result<(), ModelError> {
        if self.poisoned { return Err(ModelError::Conflict("failed external ordering")); }
        let result = self.push_inner(value);
        if result.is_err() { self.poisoned = true; }
        result
    }
    fn push_inner(&mut self, value: A::Output) -> Result<(), ModelError> {
        let row = entry::<A>(value)?;
        self.bounds.frame = self.bounds.frame.max(row.bytes.len());
        self.bounds.work = self.bounds.work.max(row.work_bytes);
        let row_bytes = row.bytes.capacity() + A::key_bytes(&row.key);
        let capacity = if self.pending.len() == self.pending.capacity() { self.pending.capacity().saturating_mul(2).max(1) } else { self.pending.capacity() };
        let projected = self.payload_bytes.saturating_add(row_bytes).saturating_add(capacity * size_of::<Entry<A::Key>>());
        if projected > self.limit { self.flush()?; }
        let capacity = if self.pending.len() == self.pending.capacity() { self.pending.capacity().saturating_mul(2).max(1) } else { self.pending.capacity() };
        let pending_bytes = self.payload_bytes.saturating_add(row_bytes).saturating_add(capacity * size_of::<Entry<A::Key>>());
        // Reduced test run limits may emit a singleton; the production candidate buffer
        // still never exceeds the existing one-MiB run bound.
        if self.budget.is_some() && pending_bytes > RUN_BYTES {
            return Err(ModelError::Limit { owner: OWNER, limit: "candidate run bytes", observed: pending_bytes, bound: RUN_BYTES });
        }
        self.charge(pending_bytes, self.runs.capacity())?;
        if capacity > self.pending.capacity() { self.pending.reserve_exact(capacity - self.pending.len()); }
        self.payload_bytes += row_bytes;
        self.pending.push(row);
        if pending_bytes >= self.limit { self.flush()?; }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if self.pending.is_empty() { return Ok(()); }
        // Unstable sorting is in-place: no second result-sized sort allocation.
        self.pending.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        let mut run = NamedTempFile::new_in(self.directory.path()).map_err(ModelError::codec)?;
        {
            let _work = reserve(self.budget.as_ref(), IO_BYTES)?;
            let mut writer = BufWriter::with_capacity(IO_BYTES, run.as_file_mut());
            for (index, row) in self.pending.iter().enumerate() {
                if index > 0 && self.pending[index - 1].key == row.key {
                    if self.pending[index - 1].bytes != row.bytes { return Err(ModelError::Conflict(A::CONFLICT)); }
                    continue;
                }
                write(&mut writer, row)?;
            }
            writer.flush().map_err(ModelError::codec)?;
        }
        self.pending = Vec::new();
        self.payload_bytes = 0;
        self.charge(0, self.runs.capacity())?;
        let mut level = 0;
        loop {
            if level == self.runs.len() {
                if self.runs.len() == self.runs.capacity() {
                    let capacity = self.runs.capacity().saturating_mul(2).max(1);
                    self.charge(0, capacity)?;
                    self.runs.reserve_exact(capacity - self.runs.len());
                }
                self.runs.push(None);
            }
            match self.runs[level].take() {
                Some(previous) => { run = merge::<A>(self.directory.path(), previous, run, self.budget.as_ref(), self.bounds)?; level += 1; }
                None => { self.runs[level] = Some(run); break; }
            }
        }
        Ok(())
    }
    fn finish(mut self) -> Result<Ordered<A>, ModelError> {
        if self.poisoned { return Err(ModelError::Conflict("failed external ordering")); }
        self.flush()?;
        let mut final_run = None;
        for run in self.runs.into_iter().flatten() {
            final_run = Some(match final_run { Some(previous) => merge::<A>(self.directory.path(), previous, run, self.budget.as_ref(), self.bounds)?, None => run });
        }
        // Reader plus bounded decoded frame/output. Ownership travels with scratch through
        // acknowledged blocking work; no task/runtime is created by this synchronous kernel.
        let reader_bytes = if final_run.is_some() { self.bounds.work + IO_BYTES } else { 0 };
        let retained = reserve(self.budget.as_ref(), reader_bytes + size_of::<Ordered<A>>() + self.directory.path().as_os_str().len())?;
        let reader = final_run.as_ref().map(|run| run.reopen().map(|file| BufReader::with_capacity(IO_BYTES, file))).transpose().map_err(ModelError::codec)?;
        Ok(Ordered { reader, _run: final_run, _directory: self.directory, _retained: retained, bounds: self.bounds, _adapter: PhantomData })
    }
}
struct Ordered<A: Adapter> {
    reader: Option<BufReader<File>>,
    _run: Option<NamedTempFile>,
    _directory: TempDir,
    _retained: Option<Box<dyn Reservation>>,
    bounds: FrameBounds,
    _adapter: PhantomData<A>,
}
impl<A: Adapter> Ordered<A> {
    fn rewind(&mut self) -> Result<(), ModelError> {
        if let Some(reader) = &mut self.reader { reader.seek(SeekFrom::Start(0)).map_err(ModelError::codec)?; }
        Ok(())
    }
    fn next(&mut self) -> Result<Option<A::Output>, ModelError> {
        let Some(reader) = &mut self.reader else { return Ok(None); };
        read::<A>(reader, self.bounds)?.map(|row| A::decode(&row.bytes)).transpose()
    }
}

pub struct SortedRows(Sorter<Physical>);
impl SortedRows {
    pub fn new() -> Result<Self, ModelError> { Self::with_run_bytes(RUN_BYTES) }
    pub fn with_budget(budget: &ResourceBudget) -> Result<Self, ModelError> { Sorter::new(RUN_BYTES, Some(budget.clone())).map(Self) }
    pub fn with_run_bytes(limit: usize) -> Result<Self, ModelError> { Sorter::new(limit, None).map(Self) }
    pub fn push(&mut self, row: Value) -> Result<(), ModelError> { self.0.push(row) }
    pub fn finish(self) -> Result<OrderedRows, ModelError> { self.0.finish().map(OrderedRows) }
}
pub struct OrderedRows(Ordered<Physical>);
impl OrderedRows {
    pub fn rewind(&mut self) -> Result<(), ModelError> { self.0.rewind() }
    pub fn next_row(&mut self) -> Result<Option<Value>, ModelError> { self.0.next() }
    /// Complete comparison preserves SDK types, absent/null and floating-point bits.
    pub async fn reconcile(&mut self, actual: &mut crate::reader::NativeRows) -> Result<(), ModelError> {
        loop {
            match (self.next_row()?, actual.next().await?) {
                (None, None) => return Ok(()),
                (Some(expected), Some(actual)) if serde_json::to_vec(&expected).map_err(ModelError::codec)? == serde_json::to_vec(&actual).map_err(ModelError::codec)? => {},
                _ => return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)),
            }
        }
    }
}

/// Synchronous bounded spill builder. The caller owns acknowledged blocking execution and
/// passes its operation's existing budget; this kernel creates no nested execution runtime.
pub struct SortedCandidates(Sorter<Compact>);
impl SortedCandidates {
    pub fn new(budget: &ResourceBudget) -> Result<Self, ModelError> { Self::with_run_bytes(budget, RUN_BYTES) }
    pub fn with_run_bytes(budget: &ResourceBudget, limit: usize) -> Result<Self, ModelError> { Sorter::new(limit, Some(budget.clone())).map(Self) }
    pub fn push(&mut self, candidate: Candidate) -> Result<(), ModelError> { self.0.push(candidate) }
    pub fn finish(self) -> Result<OrderedCandidates, ModelError> { self.0.finish().map(OrderedCandidates) }
}
/// Owns the final scratch run and its reader reservation until terminal consumption/drop.
pub struct OrderedCandidates(Ordered<Compact>);
impl OrderedCandidates {
    pub fn rewind(&mut self) -> Result<(), ModelError> { self.0.rewind() }
    pub fn next_candidate(&mut self) -> Result<Option<Candidate>, ModelError> { self.0.next() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::types::{Bytes, Object};
    fn candidate(relation: &str, key: u8, node: &str) -> Candidate {
        let mut nominal = [0; 16];
        nominal[15] = key;
        Candidate { relation: relation.into(), key: nominal, node: RecordId::new("entity", node) }
    }
    #[test]
    fn empty_candidate_ordering_needs_no_merge_or_reader_work_buffer() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let sorted = SortedCandidates::new(&budget).unwrap();
        let directory = sorted.0.directory.path().to_owned();
        let mut ordered = sorted.finish().unwrap();
        assert!(ordered.next_candidate().unwrap().is_none());
        ordered.rewind().unwrap();
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.exists());
    }
    #[test]
    fn candidates_merge_overlapping_runs_in_nominal_not_physical_order() {
        let budget = ResourceBudget::fixed(4 * ROW_BYTES).unwrap();
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 256).unwrap();
        let directory = sorted.0.directory.path().to_owned();
        for key in (0..200u8).rev() {
            // Physical pointers have the opposite order to the nominal keys.
            let row = candidate("entity", key, &format!("{:03}", 199 - key));
            sorted.push(row.clone()).unwrap();
            sorted.push(row).unwrap();
        }
        assert!(sorted.0.runs.len() > 1);
        assert!(sorted.0.runs.len() <= 10);
        sorted.push(candidate("assertion", 199, "last")).unwrap();
        let mut ordered = sorted.finish().unwrap();
        assert_eq!(ordered.next_candidate().unwrap(), Some(candidate("assertion", 199, "last")));
        for key in 0..200u8 {
            assert_eq!(ordered.next_candidate().unwrap(), Some(candidate("entity", key, &format!("{:03}", 199 - key))));
        }
        assert!(ordered.next_candidate().unwrap().is_none());
        assert!(directory.exists());
        assert!(budget.reserved() > 0);
        ordered.rewind().unwrap();
        assert!(ordered.next_candidate().unwrap().is_some());
        drop(ordered);
        assert!(!directory.exists());
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn candidate_keys_use_all_nominal_bytes_and_qualify_the_relation() {
        let budget = ResourceBudget::fixed(4 * ROW_BYTES).unwrap();
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 1).unwrap();
        let mut high = candidate("entity", 0, "first-physical");
        high.key[0] = 1;
        let low = candidate("entity", 255, "last-physical");
        let other = candidate("assertion", 255, "other");
        for row in [high.clone(), low.clone(), other.clone()] { sorted.push(row).unwrap(); }
        let mut ordered = sorted.finish().unwrap();
        for row in [other, low, high] { assert_eq!(ordered.next_candidate().unwrap(), Some(row)); }
        assert!(ordered.next_candidate().unwrap().is_none());
    }
    #[test]
    fn candidate_conflicts_are_refused_within_runs_and_during_binary_carry() {
        for limit in [1, RUN_BYTES] {
            let budget = ResourceBudget::fixed(4 * ROW_BYTES).unwrap();
            let mut sorted = SortedCandidates::with_run_bytes(&budget, limit).unwrap();
            let directory = sorted.0.directory.path().to_owned();
            sorted.push(candidate("entity", 9, "first")).unwrap();
            let result = sorted.push(candidate("entity", 9, "different"));
            let error = if limit == 1 { let error = result.unwrap_err(); drop(sorted); error } else { result.unwrap(); sorted.finish().err().unwrap() };
            assert!(matches!(error, ModelError::Conflict("conflicting nominal candidate pointer")));
            assert_eq!(budget.reserved(), 0);
            assert!(!directory.exists());
        }
    }
    #[test]
    fn candidates_conflict_in_final_merge_and_failed_sort_cannot_yield_partial_success() {
        let budget = ResourceBudget::fixed(4 * ROW_BYTES).unwrap();
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 1).unwrap();
        // Three singleton runs leave two levels; the conflicting keys meet at finish.
        sorted.push(candidate("entity", 4, "first")).unwrap();
        sorted.push(candidate("entity", 8, "unrelated")).unwrap();
        sorted.push(candidate("entity", 4, "other")).unwrap();
        assert!(matches!(sorted.finish(), Err(ModelError::Conflict("conflicting nominal candidate pointer"))));
        assert_eq!(budget.reserved(), 0);
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 1).unwrap();
        sorted.push(candidate("entity", 4, "first")).unwrap();
        assert!(sorted.push(candidate("entity", 4, "other")).is_err());
        assert!(matches!(sorted.finish(), Err(ModelError::Conflict("failed external ordering"))));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn candidate_budget_accounts_pending_metadata_merge_and_owned_reader() {
        let budget = ResourceBudget::fixed(4 * ROW_BYTES).unwrap();
        let mut sorted = SortedCandidates::with_run_bytes(&budget, RUN_BYTES).unwrap();
        let initial = budget.reserved();
        sorted.push(candidate("entity", 1, "one")).unwrap();
        assert!(budget.reserved() > initial);
        let ordered = sorted.finish().unwrap();
        assert!(budget.reserved() >= IO_BYTES);
        assert!(budget.reserved() < 64 * 1024);
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
        // This ceiling admits the in-memory candidate and its first run but refuses
        // a two-head merge, preserving the caller's original reservation.
        let budget = ResourceBudget::fixed(3 * IO_BYTES).unwrap();
        let outer = budget.reserve("scan-input", 1024).unwrap();
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 1).unwrap();
        let directory = sorted.0.directory.path().to_owned();
        sorted.push(candidate("entity", 1, "one")).unwrap();
        assert!(matches!(sorted.push(candidate("entity", 2, "two")), Err(ModelError::Resource { .. })));
        drop(sorted);
        assert!(!directory.exists());
        assert_eq!(budget.reserved(), outer.size());
        drop(outer);
        assert_eq!(budget.reserved(), 0);
    }
    fn row(key: &str, value: Value) -> Value {
        let mut row = Object::new();
        row.insert("id", RecordId::new("test", key));
        row.insert("value", value);
        Value::Object(row)
    }
    #[test]
    fn external_runs_merge_full_values_and_deduplicate_across_runs() {
        let mut rows = SortedRows::with_run_bytes(128).unwrap();
        for n in (0..200).rev() {
            let value = row(
                &format!("{n:04}"),
                Value::Bytes(Bytes::from(vec![n as u8; 100])),
            );
            rows.push(value.clone()).unwrap();
            rows.push(value).unwrap();
        }
        let mut ordered = rows.finish().unwrap();
        for n in 0..200 {
            assert_eq!(
                ordered.next_row().unwrap(),
                Some(row(
                    &format!("{n:04}"),
                    Value::Bytes(Bytes::from(vec![n as u8; 100]))
                ))
            );
        }
        assert!(ordered.next_row().unwrap().is_none());
        ordered.rewind().unwrap();
        assert!(ordered.next_row().unwrap().is_some());
    }
    #[test]
    fn sdk_serde_preserves_binary_record_ids_null_absence_and_signed_zero() {
        for value in [
            Value::None,
            Value::Null,
            Value::from_t(-0.0f64),
            Value::RecordId(RecordId::new("entity", "a")),
            Value::Bytes(Bytes::from(vec![0, 255])),
        ] {
            let original = row("key", value);
            let bytes = serde_json::to_vec(&original).unwrap();
            let mut sorted = SortedRows::with_run_bytes(1).unwrap();
            sorted.push(original).unwrap();
            assert_eq!(
                serde_json::to_vec(&sorted.finish().unwrap().next_row().unwrap().unwrap()).unwrap(),
                bytes
            );
        }
        let mut sorted = SortedRows::with_run_bytes(1).unwrap();
        sorted.push(row("same", Value::from_t(-0.0f64))).unwrap();
        assert!(sorted.push(row("same", Value::from_t(0.0f64))).is_err());
    }
    #[test]
    fn conflicting_duplicate_rows_and_oversized_values_are_refused() {
        let mut sorted = SortedRows::with_run_bytes(1).unwrap();
        sorted
            .push(row("same", Value::String("first".into())))
            .unwrap();
        assert!(
            sorted
                .push(row("same", Value::String("second".into())))
                .is_err()
        );
        let mut sorted = SortedRows::new().unwrap();
        assert!(matches!(
            sorted.push(row("large", Value::String("x".repeat(ROW_BYTES)))),
            Err(ModelError::Limit { .. })
        ));
    }
}
