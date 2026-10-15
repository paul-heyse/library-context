//! Attempt-owned external ordering of physical rows and compact nominal candidates.
use lctx_model::domain::{
    ContentHash, ModelError,
    resources::{Reservation, ResourceBudget},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    mem::size_of,
    path::Path,
    sync::Arc,
};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue, ToSql, Value};
use tempfile::{NamedTempFile, TempDir};

pub const ROW_BYTES: usize = 1024 * 1024;
const RUN_BYTES: usize = 1024 * 1024;
const IO_BYTES: usize = 8192;
const OWNER: &str = "native-ordered-candidates";

/// Encoded frame and decoded native containers coexist during ordering. Byte and
/// string payloads do not expand into one Value allocation per encoded byte.
pub fn physical_work_bytes(value: &Value, frame: usize) -> usize {
    frame
        .saturating_mul(2)
        .saturating_add(crate::loader::native_bytes(value).saturating_mul(2))
        .saturating_add(size_of::<Entry<RecordId>>())
}

/// A complete relation-qualified nominal identity and its actual stored backing pointer.
/// Scope branches and exact contributors may repeat this tuple, but cannot disagree on node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub relation: String,
    pub key: [u8; 16],
    pub node: RecordId,
    /// Present only for a membership read under its exact completed owner.
    pub content: Option<ContentHash>,
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
    type Key: Ord + Clone;
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
        RecordId::from_value(
            object
                .get("id")
                .ok_or(ModelError::Schema("ordered row identity"))?
                .clone(),
        )
        .map_err(ModelError::codec)
    }
    fn encode(value: &Value) -> Result<Vec<u8>, ModelError> {
        serde_json::to_vec(value).map_err(ModelError::codec)
    }
    fn decode(bytes: &[u8]) -> Result<Value, ModelError> {
        serde_json::from_slice(bytes).map_err(ModelError::codec)
    }
    fn key_bytes(key: &RecordId) -> usize {
        let bytes = key.to_sql().len();
        match &key.key {
            RecordIdKey::String(_) | RecordIdKey::Number(_) | RecordIdKey::Uuid(_) => bytes,
            _ => bytes.saturating_mul(2 * size_of::<Value>() + 4),
        }
    }
    fn work_bytes(value: &Value, frame: usize) -> usize {
        physical_work_bytes(value, frame)
    }
}
type NominalCandidateKey = (String, [u8; 16]);
type CandidateFrameRef<'a> = (&'a NominalCandidateKey, &'a [u8]);
struct Compact;
impl Adapter for Compact {
    type Key = NominalCandidateKey;
    type Output = Candidate;
    const CONFLICT: &'static str = "conflicting nominal candidate pointer";
    fn key(value: &Candidate) -> Result<Self::Key, ModelError> {
        if value.relation.is_empty() {
            return Err(ModelError::Schema("candidate relation"));
        }
        Ok((value.relation.clone(), value.key))
    }
    fn encode(value: &Candidate) -> Result<Vec<u8>, ModelError> {
        serde_json::to_vec(value).map_err(ModelError::codec)
    }
    fn decode(bytes: &[u8]) -> Result<Candidate, ModelError> {
        serde_json::from_slice(bytes).map_err(ModelError::codec)
    }
    fn key_bytes(key: &Self::Key) -> usize {
        key.0.capacity()
    }
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
fn entry<A: Adapter>(value: A::Output, row_bytes: usize) -> Result<Entry<A::Key>, ModelError> {
    let key = A::key(&value)?;
    let bytes = A::encode(&value)?;
    if bytes.len() > row_bytes {
        return Err(ModelError::Limit {
            owner: "native-ordered-rows",
            limit: "row bytes",
            observed: bytes.len(),
            bound: row_bytes,
        });
    }
    let work_bytes = A::work_bytes(&value, bytes.len());
    Ok(Entry {
        key,
        bytes,
        work_bytes,
    })
}
fn write<K>(writer: &mut impl Write, entry: &Entry<K>) -> Result<(), ModelError> {
    writer
        .write_all(&(entry.bytes.len() as u64).to_le_bytes())
        .map_err(ModelError::codec)?;
    writer.write_all(&entry.bytes).map_err(ModelError::codec)
}
fn read<A: Adapter>(
    reader: &mut impl Read,
    bounds: FrameBounds,
) -> Result<Option<Entry<A::Key>>, ModelError> {
    read_observed::<A, ()>(reader, bounds, |_| Ok(())).map(|row| row.map(|(entry, ())| entry))
}
type ObservedEntry<K, T> = (Entry<K>, T);
fn read_observed<A: Adapter, T>(
    reader: &mut impl Read,
    bounds: FrameBounds,
    observe: impl FnOnce(A::Output) -> Result<T, ModelError>,
) -> Result<Option<ObservedEntry<A::Key, T>>, ModelError> {
    let mut length = [0; 8];
    if reader.read(&mut length[..1]).map_err(ModelError::codec)? == 0 {
        return Ok(None);
    }
    reader
        .read_exact(&mut length[1..])
        .map_err(ModelError::codec)?;
    let length = usize::try_from(u64::from_le_bytes(length)).map_err(ModelError::codec)?;
    if length > bounds.frame {
        return Err(ModelError::Schema("ordered row frame bound"));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(ModelError::codec)?;
    // Preserve the original frame rather than reserializing it during every merge.
    let value = A::decode(&bytes)?;
    let work_bytes = A::work_bytes(&value, bytes.len());
    if work_bytes > bounds.work {
        return Err(ModelError::Schema("ordered row decode bound"));
    }
    let key = A::key(&value)?;
    let observed = observe(value)?;
    Ok(Some((
        Entry {
            key,
            bytes,
            work_bytes,
        },
        observed,
    )))
}
fn reserve(
    budget: Option<&ResourceBudget>,
    bytes: usize,
) -> Result<Option<Box<dyn Reservation>>, ModelError> {
    budget
        .map(|budget| budget.reserve(OWNER, bytes))
        .transpose()
}
struct CountedRun {
    file: NamedTempFile,
    count: usize,
}
fn merge<A: Adapter>(
    directory: &Path,
    a: NamedTempFile,
    b: NamedTempFile,
    budget: Option<&ResourceBudget>,
    bounds: FrameBounds,
) -> Result<CountedRun, ModelError> {
    // Two live heads and one transient decode, sized from admitted compact frames,
    // plus fixed reader/writer buffers. Small candidates do not pay the full-row bound.
    let _work = reserve(budget, 3 * bounds.work + 3 * IO_BYTES)?;
    let output = NamedTempFile::new_in(directory).map_err(ModelError::codec)?;
    let mut count = 0usize;
    let mut writer =
        BufWriter::with_capacity(IO_BYTES, output.reopen().map_err(ModelError::codec)?);
    let mut ar = BufReader::with_capacity(IO_BYTES, a.reopen().map_err(ModelError::codec)?);
    let mut br = BufReader::with_capacity(IO_BYTES, b.reopen().map_err(ModelError::codec)?);
    let mut av = read::<A>(&mut ar, bounds)?;
    let mut bv = read::<A>(&mut br, bounds)?;
    while av.is_some() || bv.is_some() {
        let (take_a, duplicate) = match (&av, &bv) {
            (Some(a), Some(b)) if a.key == b.key => {
                if a.bytes != b.bytes {
                    return Err(ModelError::Conflict(A::CONFLICT));
                }
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
            count = count
                .checked_add(1)
                .ok_or(ModelError::Schema("ordered run count"))?;
            drop(av.take());
            av = read::<A>(&mut ar, bounds)?;
        } else {
            write(&mut writer, bv.as_ref().expect("merge source"))?;
            count = count
                .checked_add(1)
                .ok_or(ModelError::Schema("ordered run count"))?;
            drop(bv.take());
            bv = read::<A>(&mut br, bounds)?;
        }
    }
    writer.flush().map_err(ModelError::codec)?;
    Ok(CountedRun {
        file: output,
        count,
    })
}
fn merge_tail<A: Adapter>(
    directory: &Path,
    disk: NamedTempFile,
    tail: &[Entry<A::Key>],
    budget: Option<&ResourceBudget>,
    bounds: FrameBounds,
) -> Result<CountedRun, ModelError> {
    let _work = reserve(budget, 2 * bounds.work + 2 * IO_BYTES)?;
    let output = NamedTempFile::new_in(directory).map_err(ModelError::codec)?;
    let mut count = 0usize;
    let mut writer =
        BufWriter::with_capacity(IO_BYTES, output.reopen().map_err(ModelError::codec)?);
    let mut reader = BufReader::with_capacity(IO_BYTES, disk.reopen().map_err(ModelError::codec)?);
    let mut head = read::<A>(&mut reader, bounds)?;
    let mut position = 0;
    while head.is_some() || position < tail.len() {
        let memory = tail.get(position);
        match (head.as_ref(), memory) {
            (Some(a), Some(b)) if a.key == b.key => {
                if a.bytes != b.bytes {
                    return Err(ModelError::Conflict(A::CONFLICT));
                }
                write(&mut writer, a)?;
                count = count
                    .checked_add(1)
                    .ok_or(ModelError::Schema("ordered run count"))?;
                head = read::<A>(&mut reader, bounds)?;
                position += 1;
            }
            (Some(a), Some(b)) if a.key > b.key => {
                write(&mut writer, b)?;
                count = count
                    .checked_add(1)
                    .ok_or(ModelError::Schema("ordered run count"))?;
                position += 1;
            }
            (Some(a), _) => {
                write(&mut writer, a)?;
                count = count
                    .checked_add(1)
                    .ok_or(ModelError::Schema("ordered run count"))?;
                head = read::<A>(&mut reader, bounds)?;
            }
            (None, Some(b)) => {
                write(&mut writer, b)?;
                count = count
                    .checked_add(1)
                    .ok_or(ModelError::Schema("ordered run count"))?;
                position += 1;
            }
            (None, None) => break,
        }
    }
    writer.flush().map_err(ModelError::codec)?;
    Ok(CountedRun {
        file: output,
        count,
    })
}
struct Sorter<A: Adapter> {
    directory: Option<TempDir>,
    pending: Vec<Entry<A::Key>>,
    payload_bytes: usize,
    limit: usize,
    row_bytes: usize,
    // Binary carry merging: logarithmic descriptors and two live heads, never a seen set.
    runs: Vec<Option<CountedRun>>,
    budget: Option<ResourceBudget>,
    retained: Option<Box<dyn Reservation>>,
    poisoned: bool,
    bounds: FrameBounds,
}
impl<A: Adapter> Sorter<A> {
    fn new(limit: usize, budget: Option<ResourceBudget>) -> Result<Self, ModelError> {
        if limit == 0 || limit > RUN_BYTES {
            return Err(ModelError::Schema("external run byte limit"));
        }
        let retained = reserve(budget.as_ref(), size_of::<Self>())?;
        let mut sorter = Self {
            directory: None,
            pending: Vec::new(),
            payload_bytes: 0,
            limit,
            row_bytes: ROW_BYTES,
            runs: Vec::new(),
            budget,
            retained,
            poisoned: false,
            bounds: FrameBounds::default(),
        };
        sorter.charge(0, 0)?;
        Ok(sorter)
    }
    fn metadata_bytes(&self, capacity: usize) -> usize {
        let path_bytes = self
            .directory
            .as_ref()
            .map_or(0, |dir| dir.path().as_os_str().len());
        size_of::<Self>()
            + path_bytes
            + capacity * (size_of::<Option<CountedRun>>() + path_bytes + 64)
    }
    fn charge(&mut self, pending: usize, run_capacity: usize) -> Result<(), ModelError> {
        let bytes = pending.saturating_add(self.metadata_bytes(run_capacity));
        if let Some(retained) = &mut self.retained {
            retained.try_resize(bytes)?;
        }
        Ok(())
    }
    fn ensure_directory(&mut self) -> Result<(), ModelError> {
        if self.directory.is_none() {
            self.directory = Some(tempfile::tempdir().map_err(ModelError::codec)?);
            self.charge(
                self.payload_bytes + self.pending.capacity() * size_of::<Entry<A::Key>>(),
                self.runs.capacity(),
            )?;
        }
        Ok(())
    }
    fn sort_pending(&mut self) -> Result<(), ModelError> {
        self.pending.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        for pair in self.pending.windows(2) {
            if pair[0].key == pair[1].key && pair[0].bytes != pair[1].bytes {
                return Err(ModelError::Conflict(A::CONFLICT));
            }
        }
        self.pending.dedup_by(|a, b| a.key == b.key);
        Ok(())
    }
    fn push(&mut self, value: A::Output) -> Result<(), ModelError> {
        if self.poisoned {
            return Err(ModelError::Conflict("failed external ordering"));
        }
        let result = self.push_inner(value);
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    fn push_inner(&mut self, value: A::Output) -> Result<(), ModelError> {
        let row = entry::<A>(value, self.row_bytes)?;
        self.push_entry(row)
    }
    fn push_owned(
        &mut self,
        value: A::Output,
        mut arena: Box<dyn Reservation>,
    ) -> Result<(), ModelError> {
        if self.poisoned {
            return Err(ModelError::Conflict("failed external ordering"));
        }
        let result = (|| {
            // The transferred arena covers native data and transient encoding.
            // entry consumes that Value. Only its exact encoded frame remains;
            // release dead decode workspace before spill/merge admission, keeping
            // the frame charged until the sorter's existing ownership takes over.
            let row = entry::<A>(value, self.row_bytes)?;
            arena.try_resize(
                row.bytes
                    .capacity()
                    .saturating_add(A::key_bytes(&row.key))
                    .saturating_add(size_of::<Entry<A::Key>>()),
            )?;
            self.push_entry(row)
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    fn push_entry(&mut self, row: Entry<A::Key>) -> Result<(), ModelError> {
        self.bounds.frame = self.bounds.frame.max(row.bytes.len());
        self.bounds.work = self.bounds.work.max(row.work_bytes);
        let row_bytes = row.bytes.capacity() + A::key_bytes(&row.key);
        let capacity = if self.pending.len() == self.pending.capacity() {
            self.pending.capacity().saturating_mul(2).max(1)
        } else {
            self.pending.capacity()
        };
        let projected = self
            .payload_bytes
            .saturating_add(row_bytes)
            .saturating_add(capacity * size_of::<Entry<A::Key>>());
        // Available reservation determines whether another accumulated row fits. The
        // actual reservation still arbitrates races with sibling operations.
        let available = self.budget.as_ref().map_or(usize::MAX, |budget| {
            budget
                .limit()
                .saturating_sub(budget.reserved())
                .saturating_add(self.retained.as_ref().map_or(0, |charge| charge.size()))
                .saturating_sub(self.metadata_bytes(self.runs.capacity()))
        });
        let run_limit = self.limit.min(available);
        if projected > run_limit {
            self.flush()?;
        }
        let capacity = if self.pending.len() == self.pending.capacity() {
            self.pending.capacity().saturating_mul(2).max(1)
        } else {
            self.pending.capacity()
        };
        let pending_bytes = self
            .payload_bytes
            .saturating_add(row_bytes)
            .saturating_add(capacity * size_of::<Entry<A::Key>>());
        // The frame limit admits individual rows; the run limit bounds accumulated
        // sorting state. A legal frame can exceed an empty run after allocation/key
        // metadata, so write it directly as one charged run without accumulating it.
        if pending_bytes > run_limit {
            return self.spill_singleton(row);
        }
        self.charge(pending_bytes, self.runs.capacity())?;
        if capacity > self.pending.capacity() {
            self.pending.reserve_exact(capacity - self.pending.len());
        }
        self.payload_bytes += row_bytes;
        self.pending.push(row);
        if pending_bytes >= self.limit {
            self.flush()?;
        }
        Ok(())
    }
    fn spill_singleton(&mut self, row: Entry<A::Key>) -> Result<(), ModelError> {
        debug_assert!(self.pending.is_empty());
        let bytes = row
            .bytes
            .capacity()
            .saturating_add(A::key_bytes(&row.key))
            .saturating_add(size_of::<Entry<A::Key>>());
        self.charge(bytes, self.runs.capacity())?;
        self.ensure_directory()?;
        let mut run =
            NamedTempFile::new_in(self.directory.as_ref().expect("spill directory").path())
                .map_err(ModelError::codec)?;
        {
            let capacity = self.budget.as_ref().map_or(IO_BYTES, |budget| {
                IO_BYTES.min(budget.limit().saturating_sub(budget.reserved()))
            });
            let _work = reserve(self.budget.as_ref(), capacity)?;
            let mut writer = BufWriter::with_capacity(capacity, run.as_file_mut());
            write(&mut writer, &row)?;
            writer.flush().map_err(ModelError::codec)?;
        }
        drop(row);
        self.charge(0, self.runs.capacity())?;
        self.carry_run(CountedRun {
            file: run,
            count: 1,
        })
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        // Unstable sorting is in-place: no second result-sized sort allocation.
        self.sort_pending()?;
        self.ensure_directory()?;
        let mut run =
            NamedTempFile::new_in(self.directory.as_ref().expect("spill directory").path())
                .map_err(ModelError::codec)?;
        {
            let capacity = self.budget.as_ref().map_or(IO_BYTES, |budget| {
                IO_BYTES.min(budget.limit().saturating_sub(budget.reserved()))
            });
            let _work = reserve(self.budget.as_ref(), capacity)?;
            let mut writer = BufWriter::with_capacity(capacity, run.as_file_mut());
            for (index, row) in self.pending.iter().enumerate() {
                if index > 0 && self.pending[index - 1].key == row.key {
                    if self.pending[index - 1].bytes != row.bytes {
                        return Err(ModelError::Conflict(A::CONFLICT));
                    }
                    continue;
                }
                write(&mut writer, row)?;
            }
            writer.flush().map_err(ModelError::codec)?;
        }
        let count = self.pending.len();
        self.pending = Vec::new();
        self.payload_bytes = 0;
        self.charge(0, self.runs.capacity())?;
        self.carry_run(CountedRun { file: run, count })
    }
    fn carry_run(&mut self, mut run: CountedRun) -> Result<(), ModelError> {
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
                Some(previous) => {
                    run = merge::<A>(
                        self.directory.as_ref().expect("spill directory").path(),
                        previous.file,
                        run.file,
                        self.budget.as_ref(),
                        self.bounds,
                    )?;
                    level += 1;
                }
                None => {
                    self.runs[level] = Some(run);
                    break;
                }
            }
        }
        Ok(())
    }
    fn finish(mut self) -> Result<Ordered<A>, ModelError> {
        if self.poisoned {
            return Err(ModelError::Conflict("failed external ordering"));
        }
        self.sort_pending()?;
        let mut final_run: Option<CountedRun> = None;
        for run in self.runs.into_iter().flatten() {
            final_run = Some(match final_run {
                Some(previous) => merge::<A>(
                    self.directory.as_ref().expect("spill directory").path(),
                    previous.file,
                    run.file,
                    self.budget.as_ref(),
                    self.bounds,
                )?,
                None => run,
            });
        }
        // A fitting result never creates scratch. When earlier runs exist, merge the
        // fitting tail directly into their final run instead of writing a tail run.
        let memory = if let Some(file) = final_run.take() {
            if self.pending.is_empty() {
                final_run = Some(file);
            } else {
                final_run = Some(merge_tail::<A>(
                    self.directory.as_ref().expect("spill directory").path(),
                    file.file,
                    &self.pending,
                    self.budget.as_ref(),
                    self.bounds,
                )?);
            }
            self.pending = Vec::new();
            Vec::new()
        } else {
            self.pending
        };
        let path_bytes = self
            .directory
            .as_ref()
            .map_or(0, |dir| dir.path().as_os_str().len());
        let memory_bytes = memory.capacity() * size_of::<Entry<A::Key>>()
            + memory
                .iter()
                .map(|entry| entry.bytes.capacity() + A::key_bytes(&entry.key))
                .sum::<usize>();
        let retained_bytes = size_of::<Run<A>>() + path_bytes + memory_bytes;
        if let Some(retained) = &mut self.retained {
            retained.try_resize(retained_bytes)?;
        }
        let count = final_run.as_ref().map_or(memory.len(), |run| run.count);
        let run = Arc::new(Run {
            count,
            file: final_run.map(|run| run.file),
            memory,
            _directory: self.directory,
            _retained: self.retained,
            bounds: self.bounds,
            budget: self.budget,
            _adapter: PhantomData,
        });
        Ordered::open(run)
    }
}
/// Immutable final scratch. Every cursor opens a distinct file description and owns its
/// reader/decode reservation. The last run/cursor releases both scratch and owner charge.
struct Run<A: Adapter> {
    count: usize,
    file: Option<NamedTempFile>,
    memory: Vec<Entry<A::Key>>,
    _directory: Option<TempDir>,
    _retained: Option<Box<dyn Reservation>>,
    bounds: FrameBounds,
    budget: Option<ResourceBudget>,
    _adapter: PhantomData<A>,
}
struct Ordered<A: Adapter> {
    reader: Option<BufReader<File>>,
    run: Arc<Run<A>>,
    position: usize,
    _reader_charge: Option<Box<dyn Reservation>>,
}
impl<A: Adapter> Ordered<A> {
    fn open(run: Arc<Run<A>>) -> Result<Self, ModelError> {
        let budget = run.budget.clone();
        Self::open_in(run, budget)
    }
    fn open_in(run: Arc<Run<A>>, budget: Option<ResourceBudget>) -> Result<Self, ModelError> {
        let reader_bytes = if run.file.is_some() {
            run.bounds.work + IO_BYTES
        } else if !run.memory.is_empty() {
            run.bounds.work
        } else {
            0
        };
        let retained = reserve(budget.as_ref(), reader_bytes + size_of::<Self>())?;
        let reader = run
            .file
            .as_ref()
            .map(|file| {
                file.reopen()
                    .map(|file| BufReader::with_capacity(IO_BYTES, file))
            })
            .transpose()
            .map_err(ModelError::codec)?;
        Ok(Self {
            reader,
            run,
            position: 0,
            _reader_charge: retained,
        })
    }
    fn rewind(&mut self) -> Result<(), ModelError> {
        self.position = 0;
        if let Some(reader) = &mut self.reader {
            reader.seek(SeekFrom::Start(0)).map_err(ModelError::codec)?;
        }
        Ok(())
    }
    fn next(&mut self) -> Result<Option<A::Output>, ModelError> {
        let Some(reader) = &mut self.reader else {
            let Some(row) = self.run.memory.get(self.position) else {
                return Ok(None);
            };
            let value = A::decode(&row.bytes)?;
            self.position += 1;
            return Ok(Some(value));
        };
        read::<A>(reader, self.run.bounds)?
            .map(|row| A::decode(&row.bytes))
            .transpose()
    }
}

pub struct SortedRows(Sorter<Physical>);
impl SortedRows {
    #[cfg(test)]
    pub fn new() -> Result<Self, ModelError> {
        Self::with_run_bytes(RUN_BYTES)
    }
    pub fn with_budget(budget: &ResourceBudget) -> Result<Self, ModelError> {
        Sorter::new(RUN_BYTES, Some(budget.clone())).map(Self)
    }
    pub fn with_budget_and_row_bytes(
        budget: &ResourceBudget,
        row_bytes: usize,
    ) -> Result<Self, ModelError> {
        if row_bytes == 0 || row_bytes > lctx_model::domain::resources::MAX_ROW_BYTES {
            return Err(ModelError::Schema("physical ordering row byte limit"));
        }
        let mut sorter = Sorter::new(RUN_BYTES, Some(budget.clone()))?;
        sorter.row_bytes = row_bytes;
        Ok(Self(sorter))
    }
    #[cfg(test)]
    pub fn with_run_bytes(limit: usize) -> Result<Self, ModelError> {
        Sorter::new(limit, None).map(Self)
    }
    pub fn push(&mut self, row: Value) -> Result<(), ModelError> {
        self.0.push(row)
    }
    /// Same-pool data/encoding arena transferred by an internal owned-row caller.
    pub(crate) fn push_owned(
        &mut self,
        row: Value,
        arena: Box<dyn Reservation>,
    ) -> Result<(), ModelError> {
        self.0.push_owned(row, arena)
    }
    pub fn finish(self) -> Result<OrderedRows, ModelError> {
        self.0.finish().map(OrderedRows)
    }
}
pub struct OrderedRows(Ordered<Physical>);
/// Reusable completed physical run; contains no mutable cursor and no native scan lease.
#[derive(Clone)]
pub struct PreparedRows(Arc<Run<Physical>>);

/// Independent cursor over immutable canonical JSON frames. Memory frames are borrowed;
/// disk frames are decoded once to recover their key, never re-encoded.
pub struct PreparedFrameRows {
    run: Arc<Run<Physical>>,
    reader: Option<BufReader<File>>,
    head: Option<Entry<RecordId>>,
    position: usize,
    _retained: Box<dyn Reservation>,
}
impl PreparedFrameRows {
    fn current(&self) -> Option<(&RecordId, &[u8])> {
        if self.reader.is_some() {
            self.head
                .as_ref()
                .map(|row| (&row.key, row.bytes.as_slice()))
        } else {
            self.position
                .checked_sub(1)
                .and_then(|position| self.run.memory.get(position))
                .map(|row| (&row.key, row.bytes.as_slice()))
        }
    }
    /// The payload excludes the spool's eight-byte length prefix. The borrow lasts
    /// until the next cursor advance. EOF is independent for every opened cursor.
    pub fn next_frame(&mut self) -> Result<Option<(&RecordId, &[u8])>, ModelError> {
        if let Some(reader) = &mut self.reader {
            // Release the previous head before allocating/decoding its replacement.
            self.head = None;
            self.head = read::<Physical>(reader, self.run.bounds)?;
        } else {
            self.position = self.position.saturating_add(1);
        }
        Ok(self.current())
    }
}

/// Appends already ordered, conflict-checked frames under the existing run owner.
/// No sorting, logical lowering or JSON encoding occurs here.
struct OrderedAppend<A: Adapter> {
    count: usize,
    file: Option<NamedTempFile>,
    directory: Option<TempDir>,
    writer: Option<BufWriter<File>>,
    memory: Vec<Entry<A::Key>>,
    payload_bytes: usize,
    budget: ResourceBudget,
    retained: Box<dyn Reservation>,
    _work: Box<dyn Reservation>,
    bounds: FrameBounds,
}
impl<A: Adapter> OrderedAppend<A> {
    fn new(budget: &ResourceBudget, bounds: FrameBounds) -> Result<Self, ModelError> {
        let retained = budget.reserve(OWNER, size_of::<Run<A>>() + 2 * size_of::<usize>())?;
        // Keep a small path-creation allowance while accumulating fitting memory.
        // It transfers to the disk owner before creating any scratch, if spilling.
        let work = budget.reserve(OWNER, size_of::<Self>() + 512)?;
        Ok(Self {
            count: 0,
            file: None,
            directory: None,
            writer: None,
            memory: Vec::new(),
            payload_bytes: 0,
            budget: budget.clone(),
            retained,
            _work: work,
            bounds,
        })
    }
    fn spill_memory(&mut self) -> Result<(), ModelError> {
        if self.writer.is_some() {
            return Ok(());
        }
        self._work.try_resize(size_of::<Self>())?;
        self.retained
            .try_resize(self.retained.size().saturating_add(512))?;
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let file = NamedTempFile::new_in(directory.path()).map_err(ModelError::codec)?;
        let path_bytes = directory.path().as_os_str().len() + file.path().as_os_str().len();
        if path_bytes > 512 {
            return Err(ModelError::Schema("ordered append path bound"));
        }
        let capacity = IO_BYTES.min(self.budget.limit().saturating_sub(self.budget.reserved()));
        self._work.try_resize(size_of::<Self>() + capacity)?;
        let mut writer =
            BufWriter::with_capacity(capacity, file.reopen().map_err(ModelError::codec)?);
        for entry in &self.memory {
            write(&mut writer, entry)?;
        }
        self.directory = Some(directory);
        self.file = Some(file);
        self.writer = Some(writer);
        self.memory = Vec::new();
        self.payload_bytes = 0;
        self.retained
            .try_resize(size_of::<Run<A>>() + 2 * size_of::<usize>() + 512)?;
        Ok(())
    }
    fn append(&mut self, key: &A::Key, payload: &[u8]) -> Result<(), ModelError> {
        if payload.len() > self.bounds.frame {
            return Err(ModelError::Schema("ordered append frame bound"));
        }
        if self.writer.is_none() {
            let capacity = if self.memory.len() == self.memory.capacity() {
                self.memory.capacity().saturating_mul(2).max(1)
            } else {
                self.memory.capacity()
            };
            let payload_bytes = self
                .payload_bytes
                .saturating_add(payload.len())
                .saturating_add(A::key_bytes(key));
            let memory_bytes =
                payload_bytes.saturating_add(capacity.saturating_mul(size_of::<Entry<A::Key>>()));
            if memory_bytes <= RUN_BYTES {
                match self
                    .retained
                    .try_resize(size_of::<Run<A>>() + 2 * size_of::<usize>() + memory_bytes)
                {
                    Ok(()) => {
                        if capacity > self.memory.capacity() {
                            self.memory.reserve_exact(capacity - self.memory.len());
                        }
                        self.memory.push(Entry {
                            key: key.clone(),
                            bytes: payload.to_vec(),
                            work_bytes: self.bounds.work,
                        });
                        self.payload_bytes = payload_bytes;
                        self.count = self
                            .count
                            .checked_add(1)
                            .ok_or(ModelError::Schema("ordered append count"))?;
                        return Ok(());
                    }
                    Err(ModelError::Resource { .. }) => {}
                    Err(error) => return Err(error),
                }
            }
            self.spill_memory()?;
        }
        let writer = self.writer.as_mut().expect("ordered append writer");
        writer
            .write_all(&(payload.len() as u64).to_le_bytes())
            .map_err(ModelError::codec)?;
        writer.write_all(payload).map_err(ModelError::codec)?;
        self.count = self
            .count
            .checked_add(1)
            .ok_or(ModelError::Schema("ordered append count"))?;
        Ok(())
    }
    fn finish(mut self) -> Result<Arc<Run<A>>, ModelError> {
        if let Some(writer) = &mut self.writer {
            writer.flush().map_err(ModelError::codec)?;
        }
        drop(self.writer.take());
        Ok(Arc::new(Run {
            count: self.count,
            file: self.file,
            memory: self.memory,
            _directory: self.directory,
            _retained: Some(self.retained),
            bounds: self.bounds,
            budget: Some(self.budget),
            _adapter: PhantomData,
        }))
    }
}

fn prepared_bounds(inputs: &[PreparedRows]) -> FrameBounds {
    inputs
        .iter()
        .fold(FrameBounds::default(), |bounds, input| FrameBounds {
            frame: bounds.frame.max(input.0.bounds.frame),
            work: bounds.work.max(input.0.bounds.work),
        })
}
fn prepared_pools(inputs: &[PreparedRows], budget: &ResourceBudget) -> Result<(), ModelError> {
    if inputs.iter().any(|input| {
        !input
            .0
            .budget
            .as_ref()
            .is_some_and(|owner| owner.shares_pool(budget))
    }) {
        return Err(ModelError::Conflict("ordered union foreign resource pool"));
    }
    Ok(())
}

struct FrameHeads {
    cursors: Vec<PreparedFrameRows>,
    _retained: Box<dyn Reservation>,
}
impl FrameHeads {
    /// Admission is completed before the first read or callback. A refusal may select
    /// bounded stages; read/callback failures must never be retried as admission.
    fn open(inputs: &[PreparedRows], budget: &ResourceBudget) -> Result<Self, ModelError> {
        let retained = budget.reserve(
            OWNER,
            size_of::<Self>()
                .saturating_add(inputs.len().saturating_mul(size_of::<PreparedFrameRows>())),
        )?;
        let mut cursors = Vec::with_capacity(inputs.len());
        for input in inputs {
            cursors.push(input.frame_cursor(budget)?);
        }
        Ok(Self {
            cursors,
            _retained: retained,
        })
    }
    fn fold(
        mut self,
        check: &mut impl FnMut() -> Result<(), ModelError>,
        row: &mut impl FnMut(&RecordId, &[u8]) -> Result<(), ModelError>,
    ) -> Result<usize, ModelError> {
        for cursor in &mut self.cursors {
            check()?;
            cursor.next_frame()?;
        }
        let mut count = 0usize;
        loop {
            check()?;
            let first = self
                .cursors
                .iter()
                .enumerate()
                .filter_map(|(index, cursor)| cursor.current().map(|(key, _)| (index, key)))
                .min_by(|(_, a), (_, b)| a.cmp(b))
                .map(|(index, _)| index);
            let Some(first) = first else {
                return Ok(count);
            };
            let (key, payload) = self.cursors[first].current().expect("selected frame");
            // Check every equal head before exposing this row, including the last input.
            for cursor in &self.cursors {
                if let Some((other_key, other_payload)) = cursor.current()
                    && key == other_key
                    && payload != other_payload
                {
                    return Err(ModelError::Conflict(Physical::CONFLICT));
                }
            }
            row(key, payload)?;
            count = count
                .checked_add(1)
                .ok_or(ModelError::Schema("ordered union count"))?;
            // Keep the selected key borrowed until all other matching heads advance;
            // no extra key or comparison payload allocation is needed.
            let (before, selected_and_after) = self.cursors.split_at_mut(first);
            let (selected, after) = selected_and_after
                .split_first_mut()
                .expect("selected cursor");
            let (key, _) = selected.current().expect("selected frame");
            for cursor in before.iter_mut().chain(after.iter_mut()) {
                if cursor.current().is_some_and(|(other, _)| other == key) {
                    check()?;
                    cursor.next_frame()?;
                }
            }
            check()?;
            selected.next_frame()?;
        }
    }
}

fn balanced_union(
    inputs: &[PreparedRows],
    budget: &ResourceBudget,
    check: &mut impl FnMut() -> Result<(), ModelError>,
) -> Result<PreparedRows, ModelError> {
    check()?;
    // Each recursion owns at most two intermediate handles. Original inputs remain
    // charged to the same pool throughout; intermediate runs release with their last Arc.
    let _handles = budget.reserve(
        OWNER,
        4 * size_of::<PreparedRows>() + size_of::<FrameHeads>(),
    )?;
    if inputs.len() == 1 {
        return Ok(inputs[0].clone());
    }
    let middle = inputs.len() / 2;
    let left = balanced_union(&inputs[..middle], budget, check)?;
    let right = balanced_union(&inputs[middle..], budget, check)?;
    let pair = [left, right];
    let mut output = OrderedAppend::<Physical>::new(budget, prepared_bounds(&pair))?;
    FrameHeads::open(&pair, budget)?.fold(check, &mut |key, bytes| output.append(key, bytes))?;
    check()?;
    let completed = PreparedRows(output.finish()?);
    check()?;
    Ok(completed)
}

impl PreparedRows {
    pub fn frame_cursor(&self, budget: &ResourceBudget) -> Result<PreparedFrameRows, ModelError> {
        prepared_pools(std::slice::from_ref(self), budget)?;
        let bytes = size_of::<PreparedFrameRows>().saturating_add(if self.0.file.is_some() {
            self.0.bounds.work.saturating_add(IO_BYTES)
        } else {
            0
        });
        let retained = budget.reserve(OWNER, bytes)?;
        let reader = self
            .0
            .file
            .as_ref()
            .map(|file| {
                file.reopen()
                    .map(|file| BufReader::with_capacity(IO_BYTES, file))
            })
            .transpose()
            .map_err(ModelError::codec)?;
        Ok(PreparedFrameRows {
            run: self.0.clone(),
            reader,
            head: None,
            position: 0,
            _retained: retained,
        })
    }
    /// Fold the exact set union in RecordId::Ord order, preserving canonical JSON.
    /// Inputs must already be logically normalized. A callback may observe a prefix;
    /// only Ok(count), after every input EOF and final cancellation check, is complete.
    pub fn fold_union(
        inputs: &[PreparedRows],
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
        mut row: impl FnMut(&RecordId, &[u8]) -> Result<(), ModelError>,
    ) -> Result<usize, ModelError> {
        check()?;
        prepared_pools(inputs, budget)?;
        match FrameHeads::open(inputs, budget) {
            Ok(heads) => heads.fold(&mut check, &mut row),
            Err(ModelError::Resource { .. }) if inputs.len() > 2 => {
                let combined = balanced_union(inputs, budget, &mut check)?;
                FrameHeads::open(std::slice::from_ref(&combined), budget)?
                    .fold(&mut check, &mut row)
            }
            Err(error) => Err(error),
        }
    }
    /// Produce a reusable immutable run only after all required EOFs and successful
    /// cancellation checks. Errors leave every input run independently reusable.
    pub fn union_prepared(
        inputs: &[PreparedRows],
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<Self, ModelError> {
        check()?;
        prepared_pools(inputs, budget)?;
        let mut output = OrderedAppend::<Physical>::new(budget, prepared_bounds(inputs))?;
        match FrameHeads::open(inputs, budget) {
            Ok(heads) => {
                heads.fold(&mut check, &mut |key, bytes| output.append(key, bytes))?;
                check()?;
                let completed = PreparedRows(output.finish()?);
                check()?;
                Ok(completed)
            }
            Err(ModelError::Resource { .. }) if inputs.len() > 2 => {
                drop(output);
                balanced_union(inputs, budget, &mut check)
            }
            Err(error) => Err(error),
        }
    }
    /// Build once over the admitted immutable run. Only offsets grow with row count,
    /// on disk; readers binary-seek exact native keys with constant resident work.
    pub fn point_index(&self, budget: &ResourceBudget) -> Result<PreparedPointRows, ModelError> {
        point_index(self.0.clone(), budget, &mut || Ok(())).map(PreparedPointRows)
    }
    pub fn cursor(&self) -> Result<OrderedRows, ModelError> {
        Ordered::open(self.0.clone()).map(OrderedRows)
    }
    pub fn cursor_with_budget(&self, budget: &ResourceBudget) -> Result<OrderedRows, ModelError> {
        Ordered::open_in(self.0.clone(), Some(budget.clone())).map(OrderedRows)
    }
}
struct PointIndex<A: Adapter> {
    run: Arc<Run<A>>,
    offsets: Option<NamedTempFile>,
    count: u64,
    _retained: Box<dyn Reservation>,
}
fn point_index<A: Adapter>(
    run: Arc<Run<A>>,
    budget: &ResourceBudget,
    check: &mut impl FnMut() -> Result<(), ModelError>,
) -> Result<Arc<PointIndex<A>>, ModelError> {
    let mut retained = budget.reserve(
        OWNER,
        size_of::<PointIndex<A>>() + if run.file.is_some() { 4096 } else { 0 },
    )?;
    let mut offsets = None;
    let mut count = run.memory.len() as u64;
    if let Some(file) = &run.file {
        let _work = budget.reserve(OWNER, run.bounds.work + 2 * IO_BYTES)?;
        let mut input =
            BufReader::with_capacity(IO_BYTES, file.reopen().map_err(ModelError::codec)?);
        let output =
            NamedTempFile::new_in(run._directory.as_ref().expect("disk run directory").path())
                .map_err(ModelError::codec)?;
        if output.path().as_os_str().len() > 4096 {
            return Err(ModelError::Schema("point index path bound"));
        }
        let mut writer =
            BufWriter::with_capacity(IO_BYTES, output.reopen().map_err(ModelError::codec)?);
        count = 0;
        loop {
            check()?;
            let offset = input.stream_position().map_err(ModelError::codec)?;
            let Some(_row) = read::<A>(&mut input, run.bounds)? else {
                break;
            };
            writer
                .write_all(&offset.to_le_bytes())
                .map_err(ModelError::codec)?;
            count = count
                .checked_add(1)
                .ok_or(ModelError::Schema("point index length"))?;
        }
        writer.flush().map_err(ModelError::codec)?;
        offsets = Some(output);
    }
    retained.try_resize(
        size_of::<PointIndex<A>>()
            + 2 * size_of::<usize>()
            + offsets
                .as_ref()
                .map_or(0, |file| file.path().as_os_str().len() + 64),
    )?;
    Ok(Arc::new(PointIndex {
        run,
        offsets,
        count,
        _retained: retained,
    }))
}
/// Exact lookup owner over a collision-checked sorted physical run.
#[derive(Clone)]
pub struct PreparedPointRows(Arc<PointIndex<Physical>>);
pub struct PointRows {
    index: Arc<PointIndex<Physical>>,
    offsets: Option<File>,
    rows: Option<File>,
    value: Option<Value>,
    _retained: Box<dyn Reservation>,
    #[cfg(test)]
    probes: usize,
}
impl PreparedPointRows {
    pub fn cursor(&self, budget: &ResourceBudget) -> Result<PointRows, ModelError> {
        let retained = budget.reserve(OWNER, size_of::<PointRows>() + self.0.run.bounds.work)?;
        Ok(PointRows {
            index: self.0.clone(),
            offsets: self
                .0
                .offsets
                .as_ref()
                .map(NamedTempFile::reopen)
                .transpose()
                .map_err(ModelError::codec)?,
            rows: self
                .0
                .run
                .file
                .as_ref()
                .map(NamedTempFile::reopen)
                .transpose()
                .map_err(ModelError::codec)?,
            value: None,
            _retained: retained,
            #[cfg(test)]
            probes: 0,
        })
    }
}
impl PointRows {
    pub fn contains(&mut self, wanted: &RecordId) -> Result<bool, ModelError> {
        self.seek(wanted, false)
    }
    /// One decoded match borrows this cursor's admitted workspace. Consume or
    /// independently admit any retained fields before requesting another row.
    pub fn get(&mut self, wanted: &RecordId) -> Result<Option<&Value>, ModelError> {
        self.seek(wanted, true)?;
        Ok(self.value.as_ref())
    }
    fn seek(&mut self, wanted: &RecordId, decoded: bool) -> Result<bool, ModelError> {
        self.value = None;
        if self.index.run.file.is_none() {
            let Ok(position) = self
                .index
                .run
                .memory
                .binary_search_by(|row| row.key.cmp(wanted))
            else {
                return Ok(false);
            };
            if decoded {
                let value = Physical::decode(&self.index.run.memory[position].bytes)?;
                if Physical::key(&value)? != *wanted {
                    return Err(ModelError::Conflict("point index memory identity"));
                }
                self.value = Some(value);
            }
            return Ok(true);
        }
        let (Some(offsets), Some(rows)) = (&mut self.offsets, &mut self.rows) else {
            return Err(ModelError::Schema("point index missing disk backing"));
        };
        let mut low = 0u64;
        let mut high = self.index.count;
        while low < high {
            let middle = low + (high - low) / 2;
            offsets
                .seek(SeekFrom::Start(
                    middle
                        .checked_mul(8)
                        .ok_or(ModelError::Schema("point index offset"))?,
                ))
                .map_err(ModelError::codec)?;
            let mut offset = [0; 8];
            offsets.read_exact(&mut offset).map_err(ModelError::codec)?;
            rows.seek(SeekFrom::Start(u64::from_le_bytes(offset)))
                .map_err(ModelError::codec)?;
            let (row, value) =
                read_observed::<Physical, _>(rows, self.index.run.bounds, |value| {
                    Ok(decoded.then_some(value))
                })?
                .ok_or(ModelError::Schema("point index missing row"))?;
            #[cfg(test)]
            {
                self.probes += 1;
            }
            match row.key.cmp(wanted) {
                std::cmp::Ordering::Equal => {
                    self.value = value;
                    return Ok(true);
                }
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
            }
        }
        Ok(false)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct SourcePointer {
    source: RecordId,
    row: RecordId,
    // Byte offset for a disk run, ordinal for a memory run.
    position: u64,
}
struct SourceOrder;
impl Adapter for SourceOrder {
    type Key = SourcePointer;
    type Output = SourcePointer;
    const CONFLICT: &'static str = "conflicting source index pointer";
    fn key(value: &SourcePointer) -> Result<SourcePointer, ModelError> {
        Ok(value.clone())
    }
    fn encode(value: &SourcePointer) -> Result<Vec<u8>, ModelError> {
        serde_json::to_vec(value).map_err(ModelError::codec)
    }
    fn decode(bytes: &[u8]) -> Result<SourcePointer, ModelError> {
        serde_json::from_slice(bytes).map_err(ModelError::codec)
    }
    fn key_bytes(key: &SourcePointer) -> usize {
        Physical::key_bytes(&key.source).saturating_add(Physical::key_bytes(&key.row))
    }
    fn work_bytes(value: &SourcePointer, frame: usize) -> usize {
        let scalar = |key: &RecordIdKey| {
            matches!(
                key,
                RecordIdKey::String(_) | RecordIdKey::Number(_) | RecordIdKey::Uuid(_)
            )
        };
        let factor = if scalar(&value.source.key) && scalar(&value.row.key) {
            4
        } else {
            2 * size_of::<Value>() + 4
        };
        frame
            .saturating_mul(factor)
            .saturating_add(size_of::<Entry<SourcePointer>>())
    }
}
fn source_key(value: &Value, field: &'static str) -> Result<RecordId, ModelError> {
    let Value::Object(object) = value else {
        return Err(ModelError::Schema("source index row"));
    };
    RecordId::from_value(
        object
            .get(field)
            .ok_or(ModelError::Schema("source index field"))?
            .clone(),
    )
    .map_err(ModelError::codec)
}
pub(crate) fn record_id_bytes(wanted: &RecordId) -> usize {
    fn key_bytes(key: &RecordIdKey) -> usize {
        use std::ops::Bound;
        size_of::<RecordIdKey>().saturating_add(match key {
            RecordIdKey::String(value) => value.len(),
            RecordIdKey::Number(_) | RecordIdKey::Uuid(_) => 0,
            RecordIdKey::Array(values) => values
                .iter()
                .map(crate::loader::native_bytes)
                .fold(0, usize::saturating_add),
            RecordIdKey::Object(values) => values
                .iter()
                .map(|(key, value)| {
                    key.len()
                        .saturating_add(32)
                        .saturating_add(crate::loader::native_bytes(value))
                })
                .fold(0, usize::saturating_add),
            RecordIdKey::Range(range) => [&range.start, &range.end]
                .into_iter()
                .map(|bound| match bound {
                    Bound::Included(key) | Bound::Excluded(key) => key_bytes(key),
                    Bound::Unbounded => 0,
                })
                .fold(0, usize::saturating_add),
        })
    }
    wanted
        .table
        .as_str()
        .len()
        .saturating_add(key_bytes(&wanted.key).saturating_mul(2))
}
struct SourceIndex {
    original: Arc<Run<Physical>>,
    pointers: Arc<PointIndex<SourceOrder>>,
    _retained: Box<dyn Reservation>,
}
/// Immutable source-key adjacency over exact original row payloads. Compact pointers
/// share the existing external-sort and point-offset owners, including spill.
#[derive(Clone)]
pub struct PreparedSourceRows(Arc<SourceIndex>);
pub struct SourceRows {
    index: Arc<SourceIndex>,
    offsets: Option<File>,
    pointers: Option<File>,
    original: Option<File>,
    wanted: RecordId,
    position: u64,
    head: Option<Entry<RecordId>>,
    memory_position: Option<usize>,
    finished: bool,
    _retained: Box<dyn Reservation>,
    #[cfg(test)]
    probes: usize,
}
impl PreparedRows {
    /// Sort only compact source/id/offset pointers. Original JSON is never re-encoded.
    /// Call once per immutable field projection; source and target are separate owners.
    pub fn source_index(
        &self,
        field: &'static str,
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<PreparedSourceRows, ModelError> {
        check()?;
        prepared_pools(std::slice::from_ref(self), budget)?;
        let retained = budget.reserve(OWNER, size_of::<SourceIndex>() + 2 * size_of::<usize>())?;
        let pointers = {
            // The decoded original, extracted key and pointer construction coexist.
            let _work = budget.reserve(
                OWNER,
                self.0
                    .bounds
                    .work
                    .saturating_mul(3)
                    .saturating_add(IO_BYTES),
            )?;
            let mut sorted = Sorter::<SourceOrder>::new(RUN_BYTES, Some(budget.clone()))?;
            sorted.row_bytes = lctx_model::domain::resources::MAX_ROW_BYTES;
            if let Some(file) = &self.0.file {
                let mut reader =
                    BufReader::with_capacity(IO_BYTES, file.reopen().map_err(ModelError::codec)?);
                loop {
                    check()?;
                    let position = reader.stream_position().map_err(ModelError::codec)?;
                    let Some((row, source)) =
                        read_observed::<Physical, _>(&mut reader, self.0.bounds, |value| {
                            source_key(&value, field)
                        })?
                    else {
                        break;
                    };
                    sorted.push(SourcePointer {
                        source,
                        row: row.key,
                        position,
                    })?;
                }
            } else {
                for (position, row) in self.0.memory.iter().enumerate() {
                    check()?;
                    let source = source_key(&Physical::decode(&row.bytes)?, field)?;
                    sorted.push(SourcePointer {
                        source,
                        row: row.key.clone(),
                        position: position as u64,
                    })?;
                }
            }
            check()?;
            let ordered = sorted.finish()?;
            ordered.run.clone()
        };
        let pointers = point_index(pointers, budget, &mut check)?;
        check()?;
        Ok(PreparedSourceRows(Arc::new(SourceIndex {
            original: self.0.clone(),
            pointers,
            _retained: retained,
        })))
    }
}
impl PreparedSourceRows {
    /// Binary lower bound by native source key, followed by just that source's rows.
    /// Empty ranges are explicit; one source may own any number of distinct row IDs.
    pub fn rows_for(
        &self,
        wanted: &RecordId,
        budget: &ResourceBudget,
    ) -> Result<SourceRows, ModelError> {
        // Like cursor_with_budget, this borrows a retained immutable owner. Its
        // original charges remain live; only new cursor/key/decode allocations use
        // the request's budget. Constructing/reusing a union has stricter pool identity.
        let retained = budget.reserve(
            OWNER,
            size_of::<SourceRows>()
                .saturating_add(record_id_bytes(wanted))
                .saturating_add(self.0.original.bounds.work.saturating_mul(2))
                .saturating_add(self.0.pointers.run.bounds.work.saturating_mul(2)),
        )?;
        let mut rows = SourceRows {
            index: self.0.clone(),
            offsets: self
                .0
                .pointers
                .offsets
                .as_ref()
                .map(NamedTempFile::reopen)
                .transpose()
                .map_err(ModelError::codec)?,
            pointers: self
                .0
                .pointers
                .run
                .file
                .as_ref()
                .map(NamedTempFile::reopen)
                .transpose()
                .map_err(ModelError::codec)?,
            original: self
                .0
                .original
                .file
                .as_ref()
                .map(NamedTempFile::reopen)
                .transpose()
                .map_err(ModelError::codec)?,
            wanted: wanted.clone(),
            position: 0,
            head: None,
            memory_position: None,
            finished: false,
            _retained: retained,
            #[cfg(test)]
            probes: 0,
        };
        let mut low = 0;
        let mut high = self.0.pointers.count;
        while low < high {
            let middle = low + (high - low) / 2;
            let pointer = rows
                .pointer(middle)?
                .ok_or(ModelError::Schema("source index missing pointer"))?;
            if pointer.source < rows.wanted {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        rows.position = low;
        Ok(rows)
    }
}
impl SourceRows {
    fn pointer(&mut self, position: u64) -> Result<Option<SourcePointer>, ModelError> {
        if position >= self.index.pointers.count {
            return Ok(None);
        }
        #[cfg(test)]
        {
            self.probes += 1;
        }
        if self.index.pointers.run.file.is_none() {
            return Ok(self
                .index
                .pointers
                .run
                .memory
                .get(usize::try_from(position).map_err(ModelError::codec)?)
                .map(|entry| entry.key.clone()));
        }
        let (Some(offsets), Some(pointers)) = (&mut self.offsets, &mut self.pointers) else {
            return Err(ModelError::Schema("source index missing backing"));
        };
        offsets
            .seek(SeekFrom::Start(
                position
                    .checked_mul(8)
                    .ok_or(ModelError::Schema("source index offset"))?,
            ))
            .map_err(ModelError::codec)?;
        let mut offset = [0; 8];
        offsets.read_exact(&mut offset).map_err(ModelError::codec)?;
        pointers
            .seek(SeekFrom::Start(u64::from_le_bytes(offset)))
            .map_err(ModelError::codec)?;
        Ok(read::<SourceOrder>(pointers, self.index.pointers.run.bounds)?.map(|entry| entry.key))
    }
    fn advance(&mut self, decoded: bool) -> Result<Option<Value>, ModelError> {
        self.head = None;
        self.memory_position = None;
        if self.finished {
            return Ok(None);
        }
        let Some(pointer) = self.pointer(self.position)? else {
            self.finished = true;
            return Ok(None);
        };
        if pointer.source != self.wanted {
            self.finished = true;
            return Ok(None);
        }
        self.position = self
            .position
            .checked_add(1)
            .ok_or(ModelError::Schema("source range position"))?;
        if let Some(original) = &mut self.original {
            original
                .seek(SeekFrom::Start(pointer.position))
                .map_err(ModelError::codec)?;
            let (entry, value) =
                read_observed::<Physical, _>(original, self.index.original.bounds, |value| {
                    Ok(decoded.then_some(value))
                })?
                .ok_or(ModelError::Schema("source index missing original"))?;
            if entry.key != pointer.row {
                return Err(ModelError::Conflict("source index original identity"));
            }
            self.head = Some(entry);
            Ok(value)
        } else {
            let position = usize::try_from(pointer.position).map_err(ModelError::codec)?;
            let entry = self
                .index
                .original
                .memory
                .get(position)
                .ok_or(ModelError::Schema("source index missing original"))?;
            if entry.key != pointer.row {
                return Err(ModelError::Conflict("source index original identity"));
            }
            self.memory_position = Some(position);
            if decoded {
                Physical::decode(&entry.bytes).map(Some)
            } else {
                Ok(None)
            }
        }
    }
    pub fn next_frame(&mut self) -> Result<Option<(&RecordId, &[u8])>, ModelError> {
        self.advance(false)?;
        let entry = self.head.as_ref().or_else(|| {
            self.memory_position
                .and_then(|position| self.index.original.memory.get(position))
        });
        Ok(entry.map(|entry| (&entry.key, entry.bytes.as_slice())))
    }
    /// Decode once when a typed consumer needs a Value; frame-only consumers avoid it.
    pub fn next_row(&mut self) -> Result<Option<Value>, ModelError> {
        self.advance(true)
    }
}

impl OrderedRows {
    pub fn into_prepared(self) -> PreparedRows {
        PreparedRows(self.0.run.clone())
    }
    pub(crate) fn prepared(&self) -> PreparedRows {
        PreparedRows(self.0.run.clone())
    }
    pub fn rewind(&mut self) -> Result<(), ModelError> {
        self.0.rewind()
    }
    pub fn next_row(&mut self) -> Result<Option<Value>, ModelError> {
        self.0.next()
    }
    /// Complete comparison preserves SDK types, absent/null and floating-point bits.
    pub async fn reconcile(
        &mut self,
        actual: &mut crate::reader::NativeRows,
    ) -> Result<(), ModelError> {
        let result = async {
            loop {
                match (self.next_row()?, actual.next().await?) {
                    (None, None) => return Ok(()),
                    (Some(expected), Some(actual)) => {
                        let expected_bytes = serde_json::to_vec(&expected).map_err(ModelError::codec)?;
                        let actual_bytes = serde_json::to_vec(&actual).map_err(ModelError::codec)?;
                        if expected_bytes == actual_bytes { continue; }
                        let id = |row: &Value| match row {
                            Value::Object(object) => object.get("id").cloned(),
                            _ => None,
                        };
                        tracing::warn!(target: "lctx_phase",
                            expected_id = ?id(&expected), actual_id = ?id(&actual),
                            expected_digest = %lctx_model::domain::ContentHash::of(&expected_bytes).hex(),
                            actual_digest = %lctx_model::domain::ContentHash::of(&actual_bytes).hex(),
                            "ordered native comparison row mismatch");
                        return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt));
                    }
                    (expected, actual) => {
                        tracing::warn!(target: "lctx_phase", expected_present = expected.is_some(),
                            actual_present = actual.is_some(), "ordered native comparison length mismatch");
                        return Err(ModelError::Serving(
                            lctx_model::domain::serving::FailureKind::Corrupt,
                        ));
                    }
                }
            }
        }
        .await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "ordered actual comparison drainage",
            actual.drain_transport().await,
        );
        lctx_model::domain::completion::complete(result, completion)
    }
}

/// Synchronous bounded spill builder. The caller owns acknowledged blocking execution and
/// passes its operation's existing budget; this kernel creates no nested execution runtime.
pub struct SortedCandidates(Sorter<Compact>);
impl SortedCandidates {
    pub fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        Self::with_run_bytes(budget, RUN_BYTES)
    }
    pub fn with_run_bytes(budget: &ResourceBudget, limit: usize) -> Result<Self, ModelError> {
        Sorter::new(limit, Some(budget.clone())).map(Self)
    }
    pub fn push(&mut self, candidate: Candidate) -> Result<(), ModelError> {
        self.0.push(candidate)
    }
    pub fn finish(self) -> Result<OrderedCandidates, ModelError> {
        self.0.finish().map(OrderedCandidates)
    }
}
/// Owns the final scratch run and its reader reservation until terminal consumption/drop.
pub struct OrderedCandidates(Ordered<Compact>);
/// Compact immutable exact-view selection. Cursors own independent charged readers.
#[derive(Clone)]
pub struct PreparedCandidates(Arc<Run<Compact>>);
impl PreparedCandidates {
    /// Earned during sorting/merging through complete input EOF; never a metadata
    /// claim from a registered native descriptor or a second count-only scan.
    pub fn rows(&self) -> usize {
        self.0.count
    }
    /// Merge immutable compact frames in nominal-key order without decoding and
    /// sorting their entire contents again. Every actual changed union is checked
    /// through both EOFs before its sealed count/backing becomes available.
    pub fn union_prepared(
        inputs: &[Self],
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<Self, ModelError> {
        fn union(
            inputs: &[PreparedCandidates],
            budget: &ResourceBudget,
            check: &mut impl FnMut() -> Result<(), ModelError>,
        ) -> Result<PreparedCandidates, ModelError> {
            check()?;
            let _handles = budget.reserve(OWNER, 4 * size_of::<PreparedCandidates>())?;
            if inputs.is_empty() {
                return Ok(SortedCandidates::new(budget)?.finish()?.into_prepared());
            }
            if inputs.len() == 1 {
                return Ok(inputs[0].clone());
            }
            let middle = inputs.len() / 2;
            let left = union(&inputs[..middle], budget, check)?;
            let right = union(&inputs[middle..], budget, check)?;
            let bounds = FrameBounds {
                frame: left.0.bounds.frame.max(right.0.bounds.frame),
                work: left.0.bounds.work.max(right.0.bounds.work),
            };
            let mut a = CompactFrames::open(&left, budget)?;
            let mut b = CompactFrames::open(&right, budget)?;
            let mut output = OrderedAppend::<Compact>::new(budget, bounds)?;
            a.advance()?;
            b.advance()?;
            while a.current().is_some() || b.current().is_some() {
                check()?;
                let (take_a, duplicate) = match (a.current(), b.current()) {
                    (Some((ak, av)), Some((bk, bv))) if ak == bk => {
                        if av != bv {
                            return Err(ModelError::Conflict(Compact::CONFLICT));
                        }
                        (true, true)
                    }
                    (Some((ak, _)), Some((bk, _))) => (ak < bk, false),
                    (Some(_), None) => (true, false),
                    _ => (false, false),
                };
                if take_a {
                    let (key, payload) = a.current().expect("selected compact head");
                    output.append(key, payload)?;
                    a.advance()?;
                    if duplicate {
                        b.advance()?;
                    }
                } else {
                    let (key, payload) = b.current().expect("selected compact head");
                    output.append(key, payload)?;
                    b.advance()?;
                }
            }
            check()?;
            let result = PreparedCandidates(output.finish()?);
            check()?;
            Ok(result)
        }
        if inputs.iter().any(|input| {
            !input
                .0
                .budget
                .as_ref()
                .is_some_and(|owner| owner.shares_pool(budget))
        }) {
            return Err(ModelError::Conflict("ordered union foreign resource pool"));
        }
        union(inputs, budget, &mut check)
    }
    pub fn cursor_with_budget(
        &self,
        budget: &ResourceBudget,
    ) -> Result<OrderedCandidates, ModelError> {
        Ordered::open_in(self.0.clone(), Some(budget.clone())).map(OrderedCandidates)
    }
}
struct CompactFrames {
    run: Arc<Run<Compact>>,
    reader: Option<BufReader<File>>,
    head: Option<Entry<NominalCandidateKey>>,
    position: usize,
    _charge: Box<dyn Reservation>,
}
impl CompactFrames {
    fn open(input: &PreparedCandidates, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            OWNER,
            size_of::<Self>()
                + if input.0.file.is_some() {
                    input.0.bounds.work + IO_BYTES
                } else {
                    0
                },
        )?;
        let reader = input
            .0
            .file
            .as_ref()
            .map(|file| {
                file.reopen()
                    .map(|file| BufReader::with_capacity(IO_BYTES, file))
            })
            .transpose()
            .map_err(ModelError::codec)?;
        Ok(Self {
            run: input.0.clone(),
            reader,
            head: None,
            position: 0,
            _charge: charge,
        })
    }
    fn current(&self) -> Option<CandidateFrameRef<'_>> {
        if self.reader.is_some() {
            self.head
                .as_ref()
                .map(|entry| (&entry.key, entry.bytes.as_slice()))
        } else {
            self.position
                .checked_sub(1)
                .and_then(|position| self.run.memory.get(position))
                .map(|entry| (&entry.key, entry.bytes.as_slice()))
        }
    }
    fn advance(&mut self) -> Result<(), ModelError> {
        if let Some(reader) = &mut self.reader {
            self.head = None;
            self.head = read::<Compact>(reader, self.run.bounds)?;
        } else {
            self.position = self.position.saturating_add(1);
        }
        Ok(())
    }
}
impl OrderedCandidates {
    pub fn into_prepared(self) -> PreparedCandidates {
        PreparedCandidates(self.0.run.clone())
    }
    pub fn rewind(&mut self) -> Result<(), ModelError> {
        self.0.rewind()
    }
    pub fn next_candidate(&mut self) -> Result<Option<Candidate>, ModelError> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::types::{Bytes, Object};
    fn candidate(relation: &str, key: u8, node: &str) -> Candidate {
        let mut nominal = [0; 16];
        nominal[15] = key;
        Candidate {
            content: None,
            relation: relation.into(),
            key: nominal,
            node: RecordId::new("entity", node),
        }
    }
    #[test]
    fn compact_prepared_union_seals_counts_for_memory_and_spill_without_reordering() {
        for limit in [RUN_BYTES, 64] {
            let budget = ResourceBudget::fixed(2 << 20).unwrap();
            let make = |rows: Vec<Candidate>| {
                let mut sorted = SortedCandidates::with_run_bytes(&budget, limit).unwrap();
                for row in rows {
                    sorted.push(row).unwrap();
                }
                sorted.finish().unwrap().into_prepared()
            };
            let a = candidate("packages", 1, "a");
            let b = candidate("packages", 2, "b");
            let c = candidate("packages", 3, "c");
            let left = make(vec![c.clone(), a.clone(), a.clone()]);
            let right = make(vec![c.clone(), b.clone()]);
            assert_eq!(left.rows(), 2);
            assert_eq!(right.rows(), 2);
            let result =
                PreparedCandidates::union_prepared(&[left.clone(), right.clone()], &budget, || {
                    Ok(())
                })
                .unwrap();
            assert_eq!(result.rows(), 3);
            let mut cursor = result.cursor_with_budget(&budget).unwrap();
            assert_eq!(cursor.next_candidate().unwrap(), Some(a));
            assert_eq!(cursor.next_candidate().unwrap(), Some(b));
            assert_eq!(cursor.next_candidate().unwrap(), Some(c));
            assert!(cursor.next_candidate().unwrap().is_none());
            drop(cursor);
            drop(result);
            drop(left);
            drop(right);
            assert_eq!(budget.reserved(), 0);
        }
    }
    #[test]
    fn compact_union_refuses_late_conflict_cancellation_foreign_pool_and_local_capacity() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let make = |rows: Vec<Candidate>| {
            let mut sorted = SortedCandidates::with_run_bytes(&budget, 64).unwrap();
            for row in rows {
                sorted.push(row).unwrap();
            }
            sorted.finish().unwrap().into_prepared()
        };
        let left = make(vec![
            candidate("packages", 1, "a"),
            candidate("packages", 9, "same"),
        ]);
        let conflict = make(vec![
            candidate("packages", 2, "b"),
            candidate("packages", 9, "different"),
        ]);
        let before = budget.reserved();
        assert!(matches!(
            PreparedCandidates::union_prepared(&[left.clone(), conflict.clone()], &budget, || Ok(
                ()
            )),
            Err(ModelError::Conflict(
                "conflicting nominal candidate pointer"
            ))
        ));
        assert_eq!(budget.reserved(), before);
        let mut calls = 0;
        assert!(
            PreparedCandidates::union_prepared(&[left.clone(), left.clone()], &budget, || {
                calls += 1;
                if calls > 3 {
                    Err(ModelError::Conflict("test cancelled"))
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(budget.reserved(), before);
        let foreign = ResourceBudget::fixed(1 << 20).unwrap();
        assert!(matches!(
            PreparedCandidates::union_prepared(std::slice::from_ref(&left), &foreign, || Ok(())),
            Err(ModelError::Conflict("ordered union foreign resource pool"))
        ));
        let held = budget
            .reserve("test fill", budget.limit() - budget.reserved() - 512)
            .unwrap();
        assert!(matches!(
            PreparedCandidates::union_prepared(&[left.clone(), left.clone()], &budget, || Ok(())),
            Err(ModelError::Resource { .. })
        ));
        drop(held);
        assert_eq!(left.rows(), 2);
        let mut cursor = left.cursor_with_budget(&budget).unwrap();
        assert!(cursor.next_candidate().unwrap().is_some());
        drop(cursor);
        drop(left);
        drop(conflict);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn empty_candidate_ordering_needs_no_merge_or_reader_work_buffer() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let sorted = SortedCandidates::new(&budget).unwrap();
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        assert!(directory.is_none());
        let mut ordered = sorted.finish().unwrap();
        assert!(ordered.0.run.file.is_none() && ordered.0.run._directory.is_none());
        assert!(ordered.next_candidate().unwrap().is_none());
        ordered.rewind().unwrap();
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
    }
    #[test]
    fn candidates_merge_overlapping_runs_in_nominal_not_physical_order() {
        let budget = ResourceBudget::fixed(4 * ROW_BYTES).unwrap();
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 256).unwrap();
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        for key in (0..200u8).rev() {
            // Physical pointers have the opposite order to the nominal keys.
            let row = candidate("entity", key, &format!("{:03}", 199 - key));
            sorted.push(row.clone()).unwrap();
            sorted.push(row).unwrap();
        }
        assert!(sorted.0.runs.len() > 1);
        assert!(sorted.0.runs.len() <= 10);
        sorted.push(candidate("assertion", 199, "last")).unwrap();
        assert!(directory.is_none());
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        let mut ordered = sorted.finish().unwrap();
        assert_eq!(
            ordered.next_candidate().unwrap(),
            Some(candidate("assertion", 199, "last"))
        );
        for key in 0..200u8 {
            assert_eq!(
                ordered.next_candidate().unwrap(),
                Some(candidate("entity", key, &format!("{:03}", 199 - key)))
            );
        }
        assert!(ordered.next_candidate().unwrap().is_none());
        assert!(directory.as_ref().is_some_and(|path| path.exists()));
        assert!(budget.reserved() > 0);
        ordered.rewind().unwrap();
        assert!(ordered.next_candidate().unwrap().is_some());
        drop(ordered);
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
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
        for row in [high.clone(), low.clone(), other.clone()] {
            sorted.push(row).unwrap();
        }
        let mut ordered = sorted.finish().unwrap();
        for row in [other, low, high] {
            assert_eq!(ordered.next_candidate().unwrap(), Some(row));
        }
        assert!(ordered.next_candidate().unwrap().is_none());
    }
    #[test]
    fn candidate_conflicts_are_refused_within_runs_and_during_binary_carry() {
        for limit in [1, RUN_BYTES] {
            let budget = ResourceBudget::fixed(4 * ROW_BYTES).unwrap();
            let mut sorted = SortedCandidates::with_run_bytes(&budget, limit).unwrap();
            let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
            sorted.push(candidate("entity", 9, "first")).unwrap();
            let result = sorted.push(candidate("entity", 9, "different"));
            assert!(directory.is_none());
            let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
            let error = if limit == 1 {
                let error = result.unwrap_err();
                drop(sorted);
                error
            } else {
                result.unwrap();
                sorted.finish().err().unwrap()
            };
            assert!(matches!(
                error,
                ModelError::Conflict("conflicting nominal candidate pointer")
            ));
            assert_eq!(budget.reserved(), 0);
            assert!(!directory.as_ref().is_some_and(|path| path.exists()));
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
        assert!(matches!(
            sorted.finish(),
            Err(ModelError::Conflict(
                "conflicting nominal candidate pointer"
            ))
        ));
        assert_eq!(budget.reserved(), 0);
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 1).unwrap();
        sorted.push(candidate("entity", 4, "first")).unwrap();
        assert!(sorted.push(candidate("entity", 4, "other")).is_err());
        assert!(matches!(
            sorted.finish(),
            Err(ModelError::Conflict("failed external ordering"))
        ));
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
        assert!(ordered.0.run.file.is_none() && ordered.0.run._directory.is_none());
        assert!(budget.reserved() > initial);
        assert!(budget.reserved() < 64 * 1024);
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
        // This ceiling admits the in-memory candidate and its first run but refuses
        // a two-head merge, preserving the caller's original reservation.
        let budget = ResourceBudget::fixed(3 * IO_BYTES).unwrap();
        let outer = budget.reserve("scan-input", 1024).unwrap();
        let mut sorted = SortedCandidates::with_run_bytes(&budget, 1).unwrap();
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        sorted.push(candidate("entity", 1, "one")).unwrap();
        assert!(directory.is_none());
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        assert!(matches!(
            sorted.push(candidate("entity", 2, "two")),
            Err(ModelError::Resource { .. })
        ));
        drop(sorted);
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
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
    fn prepared(budget: &ResourceBudget, limit: usize, values: Vec<Value>) -> PreparedRows {
        let mut sorted = Sorter::<Physical>::new(limit, Some(budget.clone()))
            .map(SortedRows)
            .unwrap();
        for value in values {
            sorted.push(value).unwrap();
        }
        sorted.finish().unwrap().into_prepared()
    }
    fn frames(input: &PreparedRows, budget: &ResourceBudget) -> Vec<Vec<u8>> {
        let mut cursor = input.frame_cursor(budget).unwrap();
        let mut rows = Vec::new();
        while let Some((_, bytes)) = cursor.next_frame().unwrap() {
            rows.push(bytes.to_vec());
        }
        rows
    }
    #[test]
    fn prepared_union_preserves_literal_payloads_and_independent_v4_identity() {
        use lctx_model::domain::{
            completed_identity::{CompletedFamilyHasher, CompletedFamilyRoots},
            recovery_closure::COMPLETED_FAMILIES,
        };
        // Literal JSON independently specifies SDK types, field order and signed zero.
        let expected: [&[u8]; 4] = [
            br#"{"Object":{"id":{"RecordId":{"table":"test","key":{"String":"a"}}},"value":"Null"}}"#,
            br#"{"Object":{"id":{"RecordId":{"table":"test","key":{"String":"b"}}},"value":{"Bytes":[0,255]}}}"#,
            br#"{"Object":{"id":{"RecordId":{"table":"test","key":{"String":"c"}}},"value":{"Number":{"Float":-0.0}}}}"#,
            br#"{"Object":{"id":{"RecordId":{"table":"test","key":{"String":"d"}}},"value":"None"}}"#,
        ];
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let values = || {
            vec![
                row("d", Value::None),
                row("c", Value::Number(surrealdb::types::Number::Float(-0.0))),
                row("b", Value::Bytes(Bytes::from(vec![0, 255]))),
                row("a", Value::Null),
            ]
        };
        for limits in [[RUN_BYTES, RUN_BYTES], [1, 1], [RUN_BYTES, 1]] {
            let first = prepared(&budget, limits[0], values());
            let second = prepared(
                &budget,
                limits[1],
                vec![row("a", Value::Null), row("d", Value::None)],
            );
            let empty = prepared(&budget, RUN_BYTES, vec![]);
            for inputs in [
                vec![first.clone(), second.clone(), empty.clone()],
                vec![second.clone(), first.clone(), first.clone()],
            ] {
                let union = PreparedRows::union_prepared(&inputs, &budget, || Ok(())).unwrap();
                assert_eq!(frames(&union, &budget), expected);
                assert_eq!(frames(&first, &budget), expected);
                let mut roots = COMPLETED_FAMILIES
                    .map(|family| CompletedFamilyHasher::new(family).unwrap().finish());
                let mut hash = CompletedFamilyHasher::new(COMPLETED_FAMILIES[0]).unwrap();
                let count = PreparedRows::fold_union(
                    &inputs,
                    &budget,
                    || Ok(()),
                    |_, bytes| hash.row(bytes),
                )
                .unwrap();
                assert_eq!(count, 4);
                roots[0] = hash.finish();
                // Independent framing oracle: no KeySink or CompletedFamilyHasher is
                // used to construct the expected family streams or the v4 state root.
                fn part(hash: &mut lctx_model::domain::ContentHasher, tag: &[u8], bytes: &[u8]) {
                    hash.update(&(tag.len() as u64).to_le_bytes());
                    hash.update(tag);
                    hash.update(&(bytes.len() as u64).to_le_bytes());
                    hash.update(bytes);
                }
                fn domain(namespace: &[u8]) -> lctx_model::domain::ContentHasher {
                    let mut hash = lctx_model::domain::ContentHasher::default();
                    part(&mut hash, b"domain", b"lctx-semantic/v3");
                    part(&mut hash, b"type", namespace);
                    hash
                }
                let mut state = domain(b"native-completed-state/v4");
                for (index, tag) in [
                    b"contributions".as_slice(),
                    b"memberships",
                    b"views",
                    b"typed-backing",
                    b"bindings",
                    b"aliases",
                ]
                .iter()
                .enumerate()
                {
                    let mut family = domain(b"native-completed-family/v1");
                    part(&mut family, b"family", tag);
                    if index == 0 {
                        for payload in expected {
                            part(&mut family, b"row", payload);
                        }
                    }
                    let count = if index == 0 { 4u64 } else { 0 };
                    part(&mut family, b"rows", &count.to_le_bytes());
                    let expected_family = family.finish().0;
                    assert_eq!(roots[index].content.0, expected_family);
                    part(&mut state, b"family", tag);
                    part(&mut state, b"rows", &count.to_le_bytes());
                    part(&mut state, b"digest", &expected_family);
                }
                assert_eq!(
                    CompletedFamilyRoots::new(roots)
                        .unwrap()
                        .identity()
                        .unwrap()
                        .0,
                    state.finish().0
                );
            }
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_union_empty_foreign_pool_and_composed_refusal() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let empty = PreparedRows::union_prepared(&[], &budget, || Ok(())).unwrap();
        assert!(empty.0.file.is_none() && empty.0._directory.is_none());
        assert_eq!(
            PreparedRows::fold_union(&[], &budget, || Ok(()), |_, _| panic!("empty callback"))
                .unwrap(),
            0
        );
        let input = prepared(&budget, 1, vec![row("a", Value::Null)]);
        let foreign = ResourceBudget::fixed(1 << 20).unwrap();
        assert!(matches!(
            input.frame_cursor(&foreign),
            Err(ModelError::Conflict("ordered union foreign resource pool"))
        ));
        assert!(matches!(
            PreparedRows::union_prepared(std::slice::from_ref(&input), &foreign, || Ok(())),
            Err(ModelError::Conflict("ordered union foreign resource pool"))
        ));
        let before = budget.reserved();
        let hold = budget
            .reserve("union refusal", budget.limit() - before - 32)
            .unwrap();
        assert!(matches!(
            PreparedRows::fold_union(
                std::slice::from_ref(&input),
                &budget,
                || Ok(()),
                |_, _| panic!("unadmitted callback")
            ),
            Err(ModelError::Resource { .. })
        ));
        assert!(matches!(
            PreparedRows::union_prepared(std::slice::from_ref(&input), &budget, || Ok(())),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(budget.reserved(), before + hold.size());
        drop(hold);
        assert_eq!(frames(&input, &budget).len(), 1);
        drop(input);
        drop(empty);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_union_retains_fitting_memory_and_spills_ordered_tail_without_encoding() {
        let tiny = ResourceBudget::fixed(4096).unwrap();
        let a = prepared(&tiny, RUN_BYTES, vec![row("a", Value::Null)]);
        let b = prepared(&tiny, RUN_BYTES, vec![row("b", Value::Null)]);
        let union =
            PreparedRows::union_prepared(&[b.clone(), a.clone()], &tiny, || Ok(())).unwrap();
        assert!(union.0.file.is_none() && union.0._directory.is_none());
        assert_eq!(
            frames(&union, &tiny),
            [
                serde_json::to_vec(&row("a", Value::Null)).unwrap(),
                serde_json::to_vec(&row("b", Value::Null)).unwrap()
            ]
        );
        drop(a);
        drop(b);
        drop(union);
        assert_eq!(tiny.reserved(), 0);
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let values = [
            row("a", Value::String("x".repeat(700_000))),
            row("b", Value::String("y".repeat(700_000))),
        ];
        for second_limit in [RUN_BYTES, 1] {
            let a = prepared(&budget, RUN_BYTES, vec![values[0].clone()]);
            let b = prepared(&budget, second_limit, vec![values[1].clone()]);
            let union =
                PreparedRows::union_prepared(&[b.clone(), a.clone()], &budget, || Ok(())).unwrap();
            assert!(union.0.file.is_some() && union.0.memory.is_empty());
            assert_eq!(
                frames(&union, &budget),
                values
                    .iter()
                    .map(|value| serde_json::to_vec(value).unwrap())
                    .collect::<Vec<_>>()
            );
            assert_eq!(frames(&a, &budget).len(), 1);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_union_late_conflict_callback_failure_and_cancellation_preserve_inputs() {
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        for limit in [RUN_BYTES, 1] {
            let first = prepared(
                &budget,
                limit,
                vec![row("a", Value::Null), row("z", Value::Null)],
            );
            let equal = prepared(&budget, limit, vec![row("z", Value::Null)]);
            let conflict = prepared(&budget, limit, vec![row("z", Value::None)]);
            let inputs = [first.clone(), equal, conflict];
            let retained = budget.reserved();
            let mut prefix = Vec::new();
            assert!(matches!(
                PreparedRows::fold_union(
                    &inputs,
                    &budget,
                    || Ok(()),
                    |id, _| {
                        prefix.push(id.clone());
                        Ok(())
                    }
                ),
                Err(ModelError::Conflict("conflicting physical row identity"))
            ));
            assert_eq!(prefix, [RecordId::new("test", "a")]);
            assert!(PreparedRows::union_prepared(&inputs, &budget, || Ok(())).is_err());
            assert_eq!(budget.reserved(), retained);
            let mut seen = 0;
            assert!(matches!(
                PreparedRows::fold_union(
                    std::slice::from_ref(&first),
                    &budget,
                    || Ok(()),
                    |_, _| {
                        seen += 1;
                        if seen == 2 {
                            Err(ModelError::Schema("injected callback failure"))
                        } else {
                            Ok(())
                        }
                    }
                ),
                Err(ModelError::Schema("injected callback failure"))
            ));
            for cancel_after in 0..8 {
                let mut checks = 0;
                let result =
                    PreparedRows::union_prepared(std::slice::from_ref(&first), &budget, || {
                        checks += 1;
                        if checks > cancel_after {
                            Err(ModelError::Schema("injected cancellation"))
                        } else {
                            Ok(())
                        }
                    });
                assert!(matches!(
                    result,
                    Err(ModelError::Schema("injected cancellation"))
                ));
                assert_eq!(budget.reserved(), retained);
            }
            assert_eq!(frames(&first, &budget).len(), 2);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_union_balanced_stages_fit_when_simultaneous_disk_heads_do_not() {
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let inputs: Vec<_> = (0..16)
            .rev()
            .map(|index| {
                prepared(
                    &budget,
                    1,
                    vec![
                        row(&format!("{index:02}"), Value::Null),
                        row("overlap", Value::Null),
                    ],
                )
            })
            .collect();
        let before = budget.reserved();
        let hold = budget
            .reserve(
                "retained sibling input",
                budget.limit() - before - 80 * 1024,
            )
            .unwrap();
        assert!(matches!(
            FrameHeads::open(&inputs, &budget),
            Err(ModelError::Resource { .. })
        ));
        let union = PreparedRows::union_prepared(&inputs, &budget, || Ok(())).unwrap();
        let mut cursor = union.cursor().unwrap();
        for index in 0..16 {
            assert_eq!(
                cursor.next_row().unwrap(),
                Some(row(&format!("{index:02}"), Value::Null))
            );
        }
        assert_eq!(
            cursor.next_row().unwrap(),
            Some(row("overlap", Value::Null))
        );
        assert!(cursor.next_row().unwrap().is_none());
        drop(cursor);
        drop(union);
        let count = PreparedRows::fold_union(&inputs, &budget, || Ok(()), |_, _| Ok(())).unwrap();
        assert_eq!(count, 17);
        assert_eq!(budget.reserved(), before + hold.size());
        for input in &inputs {
            assert_eq!(frames(input, &budget).len(), 2);
        }
        drop(hold);
        drop(inputs);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_union_late_disk_read_failure_never_completes_a_prefix() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let input = prepared(
            &budget,
            1,
            vec![row("a", Value::Null), row("z", Value::Null)],
        );
        let file = input.0.file.as_ref().unwrap();
        let original = std::fs::read(file.path()).unwrap();
        file.as_file().set_len((original.len() - 1) as u64).unwrap();
        let retained = budget.reserved();
        let mut seen = 0;
        assert!(
            PreparedRows::fold_union(
                std::slice::from_ref(&input),
                &budget,
                || Ok(()),
                |_, _| {
                    seen += 1;
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(seen, 1);
        assert!(
            PreparedRows::union_prepared(std::slice::from_ref(&input), &budget, || Ok(())).is_err()
        );
        assert_eq!(budget.reserved(), retained);
        // Repair only the intentionally damaged fixture, proving failure did not consume
        // the backing run or mutate any reusable cursor state.
        std::fs::write(file.path(), original).unwrap();
        assert_eq!(frames(&input, &budget).len(), 2);
        drop(input);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_union_native_numeric_keys_and_signed_zero_conflicts() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let numeric = |key| {
            let mut value = Object::new();
            value.insert("id", RecordId::new("test", key));
            value.insert("value", Value::Null);
            Value::Object(value)
        };
        let input = prepared(
            &budget,
            RUN_BYTES,
            vec![row("0", Value::Null), numeric(10i64), numeric(2i64)],
        );
        let mut keys = Vec::new();
        PreparedRows::fold_union(
            std::slice::from_ref(&input),
            &budget,
            || Ok(()),
            |key, _| {
                keys.push(key.clone());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            keys,
            [
                RecordId::new("test", 2i64),
                RecordId::new("test", 10i64),
                RecordId::new("test", "0")
            ]
        );
        let zero = prepared(
            &budget,
            RUN_BYTES,
            vec![row(
                "zero",
                Value::Number(surrealdb::types::Number::Float(0.0)),
            )],
        );
        let negative = prepared(
            &budget,
            1,
            vec![row(
                "zero",
                Value::Number(surrealdb::types::Number::Float(-0.0)),
            )],
        );
        assert!(matches!(
            PreparedRows::union_prepared(&[zero, negative], &budget, || Ok(())),
            Err(ModelError::Conflict("conflicting physical row identity"))
        ));
        drop(input);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_union_late_output_flush_failure_releases_unpublished_run() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let input = prepared(&budget, RUN_BYTES, vec![row("a", Value::Null)]);
        let before = budget.reserved();
        let mut output = OrderedAppend::<Physical>::new(&budget, input.0.bounds).unwrap();
        PreparedRows::fold_union(
            std::slice::from_ref(&input),
            &budget,
            || Ok(()),
            |key, payload| output.append(key, payload),
        )
        .unwrap();
        output.spill_memory().unwrap();
        let path = output.directory.as_ref().unwrap().path().to_owned();
        // Force the final buffered write to fail, after the complete input fold.
        drop(output.writer.take());
        output.writer = Some(BufWriter::with_capacity(
            IO_BYTES,
            File::open(output.file.as_ref().unwrap().path()).unwrap(),
        ));
        output
            .append(&RecordId::new("test", "a"), &frames(&input, &budget)[0])
            .unwrap();
        assert!(output.finish().is_err());
        assert!(!path.exists());
        assert_eq!(budget.reserved(), before);
        assert_eq!(frames(&input, &budget).len(), 1);
        drop(input);
        assert_eq!(budget.reserved(), 0);
    }
    fn alias(key: &str, source: &str, target: &str) -> Value {
        let mut value = Object::new();
        value.insert("id", RecordId::new("alias", key));
        value.insert("source", RecordId::new("entity", source));
        value.insert("target", RecordId::new("entity", target));
        value.insert("payload", Value::Bytes(Bytes::from(vec![0, 255])));
        Value::Object(value)
    }
    #[test]
    fn prepared_source_ranges_preserve_original_payload_and_native_key_order() {
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        for limit in [RUN_BYTES, 1] {
            let input = prepared(
                &budget,
                limit,
                vec![
                    alias("z", "a", "b"),
                    alias("a", "b", "c"),
                    alias("c", "b", "a"),
                    alias("b", "b", "a"),
                    alias("d", "z", "b"),
                ],
            );
            let sources = input.source_index("source", &budget, || Ok(())).unwrap();
            let targets = input.source_index("target", &budget, || Ok(())).unwrap();
            let mut rows = sources
                .rows_for(&RecordId::new("entity", "b"), &budget)
                .unwrap();
            for key in ["a", "b", "c"] {
                let expected = alias(key, "b", if key == "a" { "c" } else { "a" });
                let (id, payload) = rows.next_frame().unwrap().unwrap();
                assert_eq!(*id, RecordId::new("alias", key));
                assert_eq!(payload, serde_json::to_vec(&expected).unwrap());
            }
            assert!(rows.next_frame().unwrap().is_none());
            assert!(rows.next_frame().unwrap().is_none());
            drop(rows);
            let mut rows = targets
                .rows_for(&RecordId::new("entity", "a"), &budget)
                .unwrap();
            assert_eq!(rows.next_row().unwrap(), Some(alias("b", "b", "a")));
            assert_eq!(rows.next_row().unwrap(), Some(alias("c", "b", "a")));
            assert!(rows.next_row().unwrap().is_none());
            for missing in ["0", "missing", "zz"] {
                assert!(
                    sources
                        .rows_for(&RecordId::new("entity", missing), &budget)
                        .unwrap()
                        .next_frame()
                        .unwrap()
                        .is_none()
                );
            }
            let foreign = ResourceBudget::fixed(1 << 20).unwrap();
            let mut borrowed = sources
                .rows_for(&RecordId::new("entity", "b"), &foreign)
                .unwrap();
            assert!(borrowed.next_frame().unwrap().is_some());
            assert!(foreign.reserved() > 0);
            drop(borrowed);
            assert_eq!(foreign.reserved(), 0);
            let request = ResourceBudget::scoped(&budget, 128 * 1024).unwrap();
            let retained_before = budget.reserved();
            let mut borrowed = sources
                .rows_for(&RecordId::new("entity", "b"), &request)
                .unwrap();
            assert!(borrowed.next_frame().unwrap().is_some());
            assert_eq!(budget.reserved(), retained_before + request.reserved());
            drop(borrowed);
            assert_eq!(request.reserved(), 0);
            assert_eq!(budget.reserved(), retained_before);
            drop(rows);
            drop(sources);
            drop(targets);
            let retained = budget.reserved();
            assert!(matches!(
                input.source_index("absent", &budget, || Ok(())),
                Err(ModelError::Schema("source index field"))
            ));
            assert!(
                input
                    .source_index("source", &budget, || Err(ModelError::Schema(
                        "cancel source index"
                    )))
                    .is_err()
            );
            for stop in [2, 5, 8] {
                let mut checks = 0;
                assert!(
                    input
                        .source_index("source", &budget, || {
                            checks += 1;
                            if checks >= stop {
                                Err(ModelError::Schema("late source cancellation"))
                            } else {
                                Ok(())
                            }
                        })
                        .is_err()
                );
            }
            assert_eq!(budget.reserved(), retained);
            assert_eq!(frames(&input, &budget).len(), 5);
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn prepared_source_ranges_spill_and_seek_without_scanning_unrelated_rows() {
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let input = prepared(
            &budget,
            4096,
            (0..800)
                .rev()
                .map(|index| {
                    alias(
                        &format!("{index:04}"),
                        &format!("{:04}", 799 - index),
                        "shared",
                    )
                })
                .collect(),
        );
        let before = budget.reserved();
        let hold = budget
            .reserve(
                "source index sibling payload",
                budget.limit() - before - 180 * 1024,
            )
            .unwrap();
        let sources = input.source_index("source", &budget, || Ok(())).unwrap();
        assert!(
            sources.0.pointers.run.file.is_some(),
            "pointer ordering must spill under composed allowance"
        );
        let mut rows = sources
            .rows_for(&RecordId::new("entity", "0798"), &budget)
            .unwrap();
        assert_eq!(
            rows.next_row().unwrap(),
            Some(alias("0001", "0798", "shared"))
        );
        assert!(rows.next_row().unwrap().is_none());
        assert!(
            rows.probes <= 12,
            "lower-bound probes plus the requested range only"
        );
        drop(rows);
        drop(sources);
        assert_eq!(budget.reserved(), before + hold.size());
        drop(hold);
        drop(input);
        assert_eq!(budget.reserved(), 0);
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
    fn prepared_physical_cursors_have_independent_offsets_charges_and_scratch() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut sort = SortedRows::with_budget(&budget).unwrap();
        let directory = sort.0.directory.as_ref().map(|dir| dir.path().to_owned());
        sort.push(row("a", Value::Null)).unwrap();
        sort.push(row("b", Value::Bytes(Bytes::from(vec![0, 255]))))
            .unwrap();
        let prepared = sort.finish().unwrap().into_prepared();
        assert!(
            directory.is_none() && prepared.0.file.is_none() && prepared.0._directory.is_none()
        );
        let owner_bytes = budget.reserved();
        let mut first = prepared.cursor().unwrap();
        let first_bytes = budget.reserved();
        let mut second = prepared.cursor().unwrap();
        assert!(first_bytes > owner_bytes && budget.reserved() > first_bytes);
        assert_eq!(first.next_row().unwrap(), second.next_row().unwrap());
        drop(first);
        drop(prepared);
        assert!(directory.is_none());
        assert_eq!(
            second.next_row().unwrap(),
            Some(row("b", Value::Bytes(Bytes::from(vec![0, 255]))))
        );
        assert!(second.next_row().unwrap().is_none());
        drop(second);
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
    }
    #[test]
    fn disk_point_index_bounds_each_repeated_probe_and_retains_its_run() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut sort = Sorter::<Physical>::new(16 * 1024, Some(budget.clone()))
            .map(SortedRows)
            .unwrap();
        let directory = sort.0.directory.as_ref().map(|dir| dir.path().to_owned());
        for n in (0..4096).rev() {
            sort.push(row(&format!("{n:04}"), Value::Null)).unwrap();
        }
        assert!(directory.is_none());
        let directory = sort.0.directory.as_ref().map(|dir| dir.path().to_owned());
        let prepared = sort.finish().unwrap().into_prepared();
        let index = prepared.point_index(&budget).unwrap();
        let retained = budget.reserved();
        assert!(
            retained < 4096,
            "resident owner metadata must not contain a key per row"
        );
        let mut points = index.cursor(&budget).unwrap();
        drop(prepared);
        drop(index);
        assert!(directory.as_ref().is_some_and(|path| path.exists()));
        for key in ["4095", "0000", "2048", "4096", "-absent", "0000", "4095"] {
            let before = points.probes;
            let expected = !matches!(key, "4096" | "-absent");
            assert_eq!(
                points.contains(&RecordId::new("test", key)).unwrap(),
                expected
            );
            assert!(
                points.probes - before <= 13,
                "each window must seek, never scan preceding rows"
            );
        }
        assert!(!points.contains(&RecordId::new("other", "0000")).unwrap());
        drop(points);
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn point_get_returns_exact_memory_and_spill_rows_with_bounded_distinct_consumer_work() {
        for spill in [false, true] {
            let source = ResourceBudget::fixed(4 << 20).unwrap();
            let consumer = ResourceBudget::fixed(64 << 10).unwrap();
            let mut sorted =
                Sorter::<Physical>::new(if spill { 1024 } else { 1 << 20 }, Some(source.clone()))
                    .map(SortedRows)
                    .unwrap();
            for n in (0..257).rev() {
                sorted
                    .push(row(
                        &format!("{:04}", n * 2),
                        Value::String(format!("literal-{n}")),
                    ))
                    .unwrap();
            }
            let prepared = sorted.finish().unwrap().into_prepared();
            assert_eq!(prepared.0.file.is_some(), spill);
            let index = prepared.point_index(&source).unwrap();
            let owned = source.reserved();
            let insufficient = ResourceBudget::fixed(1).unwrap();
            assert!(matches!(
                index.cursor(&insufficient),
                Err(ModelError::Resource { .. })
            ));
            let mut points = index.cursor(&consumer).unwrap();
            let workspace = consumer.reserved();
            drop(prepared);
            drop(index);
            assert_eq!(
                source.reserved(),
                owned,
                "point cursor retains original owner independently"
            );
            for (key, answer) in [
                ("0200", Some("literal-100")),
                ("0000", Some("literal-0")),
                ("0512", Some("literal-256")),
                ("0201", None),
                ("9999", None),
                ("-absent", None),
                ("0200", Some("literal-100")),
            ] {
                let before = points.probes;
                let expected = answer.map(|value| row(key, Value::String(value.into())));
                assert_eq!(
                    points.get(&RecordId::new("test", key)).unwrap(),
                    expected.as_ref()
                );
                assert!(
                    points.probes - before <= 9,
                    "point get must binary-seek without scanning preceding rows"
                );
                assert_eq!(
                    consumer.reserved(),
                    workspace,
                    "repeated get uses one admitted decoded row slot"
                );
                assert_eq!(source.reserved(), owned);
            }
            assert!(
                points
                    .get(&RecordId::new("foreign", "0000"))
                    .unwrap()
                    .is_none()
            );
            drop(points);
            assert_eq!(consumer.reserved(), 0);
            assert_eq!(source.reserved(), 0);
        }
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

    fn ordinary_boundary_row(key: &str, fill: char) -> Value {
        let overhead = serde_json::to_vec(&row(key, Value::String(String::new())))
            .unwrap()
            .len();
        row(
            key,
            Value::String(fill.to_string().repeat(ROW_BYTES - overhead - 1)),
        )
    }

    #[test]
    fn ordinary_legal_frame_spills_when_allocation_and_key_exceed_the_run() {
        let boundary = ordinary_boundary_row("middle", 'x');
        {
            let encoded = entry::<Physical>(boundary.clone(), ROW_BYTES).unwrap();
            assert_eq!(encoded.bytes.len(), ROW_BYTES - 1);
            assert!(
                encoded.bytes.capacity()
                    + Physical::key_bytes(&encoded.key)
                    + size_of::<Entry<RecordId>>()
                    > RUN_BYTES
            );
        }
        let budget = ResourceBudget::fixed(128 << 20).unwrap();
        let mut sorted = SortedRows::with_budget(&budget).unwrap();
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        let before = row("before", Value::Null);
        let after = row("z-after", Value::Bytes(Bytes::from(vec![0, 255])));
        sorted.push(before.clone()).unwrap();
        sorted.push(boundary.clone()).unwrap();
        assert!(sorted.0.pending.is_empty());
        assert_eq!(sorted.0.payload_bytes, 0);
        sorted.push(boundary.clone()).unwrap();
        sorted.push(after.clone()).unwrap();
        assert!(directory.is_none());
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        let mut ordered = sorted.finish().unwrap();
        for expected in [before, boundary, after] {
            assert_eq!(ordered.next_row().unwrap(), Some(expected));
        }
        assert!(ordered.next_row().unwrap().is_none());
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
    }

    #[test]
    fn legal_singleton_conflicts_with_a_different_full_value_across_runs() {
        let budget = ResourceBudget::fixed(128 << 20).unwrap();
        let mut sorted = SortedRows::with_budget(&budget).unwrap();
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        sorted.push(row("before", Value::Null)).unwrap();
        sorted.push(ordinary_boundary_row("middle", 'x')).unwrap();
        sorted.push(ordinary_boundary_row("middle", 'y')).unwrap();
        assert!(directory.is_none());
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        assert!(matches!(
            sorted.finish(),
            Err(ModelError::Conflict("conflicting physical row identity"))
        ));
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
    }

    #[test]
    fn owned_row_releases_decode_arena_before_spill_merge() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut sorted = SortedRows::with_budget(&budget).unwrap();
        sorted.0.limit = 1;
        let rows = [
            row(
                "a",
                Value::Bytes(surrealdb::types::Bytes::from(vec![255; 128 << 10])),
            ),
            row(
                "b",
                Value::Bytes(surrealdb::types::Bytes::from(vec![0; 128 << 10])),
            ),
        ];
        for value in &rows {
            let arena = budget.reserve("owned row decode arena", 7 << 20).unwrap();
            sorted.push_owned(value.clone(), arena).unwrap();
        }
        let mut ordered = sorted.finish().unwrap();
        for value in rows {
            assert_eq!(ordered.next_row().unwrap(), Some(value));
        }
        assert!(ordered.next_row().unwrap().is_none());
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn legal_singleton_still_requires_its_composed_reservation() {
        let budget = ResourceBudget::fixed(RUN_BYTES).unwrap();
        let outer = budget.reserve("retained-input", 1024).unwrap();
        let mut sorted = SortedRows::with_budget(&budget).unwrap();
        let directory = sorted.0.directory.as_ref().map(|dir| dir.path().to_owned());
        assert!(matches!(
            sorted.push(ordinary_boundary_row("middle", 'x')),
            Err(ModelError::Resource { .. })
        ));
        assert!(matches!(
            sorted.finish(),
            Err(ModelError::Conflict("failed external ordering"))
        ));
        assert!(!directory.as_ref().is_some_and(|path| path.exists()));
        assert_eq!(budget.reserved(), outer.size());
        drop(outer);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn portable_large_rows_use_charged_singletons_without_container_amplification() {
        let value = row("a", Value::Bytes(Bytes::from(vec![255; ROW_BYTES])));
        let mut ordinary = SortedRows::new().unwrap();
        assert!(matches!(
            ordinary.push(value.clone()),
            Err(ModelError::Limit { .. })
        ));
        let budget = ResourceBudget::fixed(128 << 20).unwrap();
        let mut sorted = SortedRows::with_budget_and_row_bytes(
            &budget,
            lctx_model::domain::resources::MAX_ROW_BYTES,
        )
        .unwrap();
        let second = row("b", Value::Bytes(Bytes::from(vec![254; ROW_BYTES])));
        // The encoded frame can fit the ordinary frame limit while its key and
        // retained entry metadata put the singleton just over the run limit.
        let empty = row("c", Value::String(String::new()));
        let overhead = serde_json::to_vec(&empty).unwrap().len();
        let boundary = row("c", Value::String("x".repeat(RUN_BYTES - overhead - 1)));
        assert_eq!(serde_json::to_vec(&boundary).unwrap().len(), RUN_BYTES - 1);
        sorted.push(second.clone()).unwrap();
        assert!(
            sorted.0.pending.is_empty(),
            "large row is flushed as a singleton"
        );
        sorted.push(value.clone()).unwrap();
        sorted.push(value.clone()).unwrap();
        sorted.push(boundary.clone()).unwrap();
        assert!(
            sorted.0.pending.is_empty(),
            "metadata-heavy singleton is flushed"
        );
        let mut ordered = sorted.finish().unwrap();
        assert_eq!(ordered.next_row().unwrap(), Some(value));
        assert_eq!(ordered.next_row().unwrap(), Some(second));
        assert_eq!(ordered.next_row().unwrap(), Some(boundary));
        assert!(ordered.next_row().unwrap().is_none());
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
        assert!(
            SortedRows::with_budget_and_row_bytes(
                &budget,
                lctx_model::domain::resources::MAX_ROW_BYTES + 1,
            )
            .is_err()
        );
    }
    #[test]
    fn fitting_memory_run_replays_and_points_without_creating_scratch() {
        let budget = ResourceBudget::fixed(4096).unwrap();
        let mut sorted = SortedRows::with_budget(&budget).unwrap();
        sorted.push(row("b", Value::Null)).unwrap();
        sorted.push(row("a", Value::Null)).unwrap();
        sorted.push(row("a", Value::Null)).unwrap();
        assert!(sorted.0.directory.is_none());
        let prepared = sorted.finish().unwrap().into_prepared();
        assert!(prepared.0.file.is_none() && prepared.0._directory.is_none());
        let mut first = prepared.cursor().unwrap();
        assert_eq!(first.next_row().unwrap(), Some(row("a", Value::Null)));
        first.rewind().unwrap();
        let mut second = prepared.cursor().unwrap();
        assert_eq!(first.next_row().unwrap(), second.next_row().unwrap());
        drop(first);
        drop(second);
        let index = prepared.point_index(&budget).unwrap();
        assert!(index.0.offsets.is_none());
        let mut points = index.cursor(&budget).unwrap();
        assert!(points.contains(&RecordId::new("test", "b")).unwrap());
        assert!(!points.contains(&RecordId::new("test", "absent")).unwrap());
        drop(prepared);
        drop(index);
        drop(points);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn fitting_tail_merges_directly_and_refuses_a_late_full_value_collision() {
        for conflict in [false, true] {
            let budget = ResourceBudget::fixed(1 << 20).unwrap();
            let mut sorted = Sorter::<Physical>::new(4096, Some(budget.clone()))
                .map(SortedRows)
                .unwrap();
            sorted
                .push(row("same", Value::String("x".repeat(400))))
                .unwrap();
            sorted.0.flush().unwrap();
            sorted
                .push(row(
                    "same",
                    Value::String(if conflict {
                        "y".repeat(400)
                    } else {
                        "x".repeat(400)
                    }),
                ))
                .unwrap();
            assert!(!sorted.0.pending.is_empty());
            let path = sorted.0.directory.as_ref().unwrap().path().to_owned();
            let result = sorted.finish();
            if conflict {
                assert!(matches!(
                    result,
                    Err(ModelError::Conflict("conflicting physical row identity"))
                ));
            } else {
                let mut ordered = result.unwrap();
                assert!(ordered.next_row().unwrap().is_some());
                assert!(ordered.next_row().unwrap().is_none());
            }
            assert!(!path.exists());
            assert_eq!(budget.reserved(), 0);
        }
    }
}
