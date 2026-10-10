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
pub(crate) fn physical_work_bytes(value: &Value, frame: usize) -> usize {
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
struct Compact;
impl Adapter for Compact {
    type Key = (String, [u8; 16]);
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
    Ok(Some(Entry {
        key,
        bytes,
        work_bytes,
    }))
}
fn reserve(
    budget: Option<&ResourceBudget>,
    bytes: usize,
) -> Result<Option<Box<dyn Reservation>>, ModelError> {
    budget
        .map(|budget| budget.reserve(OWNER, bytes))
        .transpose()
}
fn merge<A: Adapter>(
    directory: &Path,
    a: NamedTempFile,
    b: NamedTempFile,
    budget: Option<&ResourceBudget>,
    bounds: FrameBounds,
) -> Result<NamedTempFile, ModelError> {
    // Two live heads and one transient decode, sized from admitted compact frames,
    // plus fixed reader/writer buffers. Small candidates do not pay the full-row bound.
    let _work = reserve(budget, 3 * bounds.work + 3 * IO_BYTES)?;
    let output = NamedTempFile::new_in(directory).map_err(ModelError::codec)?;
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
    row_bytes: usize,
    // Binary carry merging: logarithmic descriptors and two live heads, never a seen set.
    runs: Vec<Option<NamedTempFile>>,
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
            directory: tempfile::tempdir().map_err(ModelError::codec)?,
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
        size_of::<Self>()
            + self.directory.path().as_os_str().len()
            + capacity
                * (size_of::<Option<NamedTempFile>>()
                    + self.directory.path().as_os_str().len()
                    + 64)
    }
    fn charge(&mut self, pending: usize, run_capacity: usize) -> Result<(), ModelError> {
        let bytes = pending.saturating_add(self.metadata_bytes(run_capacity));
        if let Some(retained) = &mut self.retained {
            retained.try_resize(bytes)?;
        }
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
        if projected > self.limit {
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
        if pending_bytes > self.limit {
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
        let mut run = NamedTempFile::new_in(self.directory.path()).map_err(ModelError::codec)?;
        {
            let _work = reserve(self.budget.as_ref(), IO_BYTES)?;
            let mut writer = BufWriter::with_capacity(IO_BYTES, run.as_file_mut());
            write(&mut writer, &row)?;
            writer.flush().map_err(ModelError::codec)?;
        }
        drop(row);
        self.charge(0, self.runs.capacity())?;
        self.carry_run(run)
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        // Unstable sorting is in-place: no second result-sized sort allocation.
        self.pending.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        let mut run = NamedTempFile::new_in(self.directory.path()).map_err(ModelError::codec)?;
        {
            let _work = reserve(self.budget.as_ref(), IO_BYTES)?;
            let mut writer = BufWriter::with_capacity(IO_BYTES, run.as_file_mut());
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
        self.pending = Vec::new();
        self.payload_bytes = 0;
        self.charge(0, self.runs.capacity())?;
        self.carry_run(run)
    }
    fn carry_run(&mut self, mut run: NamedTempFile) -> Result<(), ModelError> {
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
                        self.directory.path(),
                        previous,
                        run,
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
        self.flush()?;
        let mut final_run = None;
        for run in self.runs.into_iter().flatten() {
            final_run = Some(match final_run {
                Some(previous) => merge::<A>(
                    self.directory.path(),
                    previous,
                    run,
                    self.budget.as_ref(),
                    self.bounds,
                )?,
                None => run,
            });
        }
        let retained = reserve(
            self.budget.as_ref(),
            size_of::<Run<A>>() + self.directory.path().as_os_str().len(),
        )?;
        let run = Arc::new(Run {
            file: final_run,
            _directory: self.directory,
            _retained: retained,
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
    file: Option<NamedTempFile>,
    _directory: TempDir,
    _retained: Option<Box<dyn Reservation>>,
    bounds: FrameBounds,
    budget: Option<ResourceBudget>,
    _adapter: PhantomData<A>,
}
struct Ordered<A: Adapter> {
    reader: Option<BufReader<File>>,
    run: Arc<Run<A>>,
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
            _reader_charge: retained,
        })
    }
    fn rewind(&mut self) -> Result<(), ModelError> {
        if let Some(reader) = &mut self.reader {
            reader.seek(SeekFrom::Start(0)).map_err(ModelError::codec)?;
        }
        Ok(())
    }
    fn next(&mut self) -> Result<Option<A::Output>, ModelError> {
        let Some(reader) = &mut self.reader else {
            return Ok(None);
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
    pub fn finish(self) -> Result<OrderedRows, ModelError> {
        self.0.finish().map(OrderedRows)
    }
}
pub struct OrderedRows(Ordered<Physical>);
/// Reusable completed physical run; contains no mutable cursor and no native scan lease.
#[derive(Clone)]
pub struct PreparedRows(Arc<Run<Physical>>);
impl PreparedRows {
    /// Build once over the admitted immutable run. Only offsets grow with row count,
    /// on disk; readers binary-seek exact native keys with constant resident work.
    pub fn point_index(&self, budget: &ResourceBudget) -> Result<PreparedPointRows, ModelError> {
        let mut offsets = None;
        let mut count = 0u64;
        if let Some(file) = &self.0.file {
            let _work = budget.reserve(OWNER, self.0.bounds.work + 2 * IO_BYTES)?;
            let mut input = BufReader::with_capacity(IO_BYTES, file.reopen().map_err(ModelError::codec)?);
            let output = NamedTempFile::new_in(self.0._directory.path()).map_err(ModelError::codec)?;
            let mut writer = BufWriter::with_capacity(IO_BYTES, output.reopen().map_err(ModelError::codec)?);
            loop {
                let offset = input.stream_position().map_err(ModelError::codec)?;
                let Some(_row) = read::<Physical>(&mut input, self.0.bounds)? else { break; };
                writer.write_all(&offset.to_le_bytes()).map_err(ModelError::codec)?;
                count = count.checked_add(1).ok_or(ModelError::Schema("point index length"))?;
            }
            writer.flush().map_err(ModelError::codec)?;
            offsets = Some(output);
        }
        let retained = budget.reserve(OWNER, size_of::<PointIndex>() + self.0._directory.path().as_os_str().len())?;
        Ok(PreparedPointRows(Arc::new(PointIndex { run: self.0.clone(), offsets, count, _retained: retained })))
    }
    pub fn cursor(&self) -> Result<OrderedRows, ModelError> {
        Ordered::open(self.0.clone()).map(OrderedRows)
    }
    pub fn cursor_with_budget(&self, budget: &ResourceBudget) -> Result<OrderedRows, ModelError> {
        Ordered::open_in(self.0.clone(), Some(budget.clone())).map(OrderedRows)
    }
}
struct PointIndex {
    run: Arc<Run<Physical>>,
    offsets: Option<NamedTempFile>,
    count: u64,
    _retained: Box<dyn Reservation>,
}
/// Exact lookup owner over a collision-checked sorted physical run.
#[derive(Clone)]
pub struct PreparedPointRows(Arc<PointIndex>);
pub struct PointRows {
    index: Arc<PointIndex>,
    offsets: Option<File>,
    rows: Option<File>,
    _retained: Box<dyn Reservation>,
    #[cfg(test)]
    probes: usize,
}
impl PreparedPointRows {
    pub fn cursor(&self, budget: &ResourceBudget) -> Result<PointRows, ModelError> {
        let retained = budget.reserve(OWNER, size_of::<PointRows>() + self.0.run.bounds.work)?;
        Ok(PointRows {
            index: self.0.clone(),
            offsets: self.0.offsets.as_ref().map(NamedTempFile::reopen).transpose().map_err(ModelError::codec)?,
            rows: self.0.run.file.as_ref().map(NamedTempFile::reopen).transpose().map_err(ModelError::codec)?,
            _retained: retained,
            #[cfg(test)]
            probes: 0,
        })
    }
}
impl PointRows {
    pub fn contains(&mut self, wanted: &RecordId) -> Result<bool, ModelError> {
        let (Some(offsets), Some(rows)) = (&mut self.offsets, &mut self.rows) else { return Ok(false); };
        let mut low = 0u64;
        let mut high = self.index.count;
        while low < high {
            let middle = low + (high - low) / 2;
            offsets.seek(SeekFrom::Start(middle.checked_mul(8).ok_or(ModelError::Schema("point index offset"))?)).map_err(ModelError::codec)?;
            let mut offset = [0; 8]; offsets.read_exact(&mut offset).map_err(ModelError::codec)?;
            rows.seek(SeekFrom::Start(u64::from_le_bytes(offset))).map_err(ModelError::codec)?;
            let row = read::<Physical>(rows, self.index.run.bounds)?.ok_or(ModelError::Schema("point index missing row"))?;
            #[cfg(test)]
            { self.probes += 1; }
            match row.key.cmp(wanted) {
                std::cmp::Ordering::Equal => return Ok(true),
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
            }
        }
        Ok(false)
    }
}
impl OrderedRows {
    pub fn into_prepared(self) -> PreparedRows {
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
        let result = async { loop {
            match (self.next_row()?, actual.next().await?) {
                (None, None) => return Ok(()),
                (Some(expected), Some(actual))
                    if serde_json::to_vec(&expected).map_err(ModelError::codec)?
                        == serde_json::to_vec(&actual).map_err(ModelError::codec)? => {}
                _ => {
                    return Err(ModelError::Serving(
                        lctx_model::domain::serving::FailureKind::Corrupt,
                    ));
                }
            }
        } }.await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("ordered actual comparison drainage", actual.drain_transport().await);
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
    pub fn cursor_with_budget(
        &self,
        budget: &ResourceBudget,
    ) -> Result<OrderedCandidates, ModelError> {
        Ordered::open_in(self.0.clone(), Some(budget.clone())).map(OrderedCandidates)
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
            let directory = sorted.0.directory.path().to_owned();
            sorted.push(candidate("entity", 9, "first")).unwrap();
            let result = sorted.push(candidate("entity", 9, "different"));
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
        assert!(matches!(
            sorted.push(candidate("entity", 2, "two")),
            Err(ModelError::Resource { .. })
        ));
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
    fn prepared_physical_cursors_have_independent_offsets_charges_and_scratch() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut sort = SortedRows::with_budget(&budget).unwrap();
        let directory = sort.0.directory.path().to_owned();
        sort.push(row("a", Value::Null)).unwrap();
        sort.push(row("b", Value::Bytes(Bytes::from(vec![0, 255]))))
            .unwrap();
        let prepared = sort.finish().unwrap().into_prepared();
        let owner_bytes = budget.reserved();
        let mut first = prepared.cursor().unwrap();
        let first_bytes = budget.reserved();
        let mut second = prepared.cursor().unwrap();
        assert!(first_bytes > owner_bytes && budget.reserved() > first_bytes);
        assert_eq!(first.next_row().unwrap(), second.next_row().unwrap());
        drop(first);
        drop(prepared);
        assert!(directory.exists());
        assert_eq!(
            second.next_row().unwrap(),
            Some(row("b", Value::Bytes(Bytes::from(vec![0, 255]))))
        );
        assert!(second.next_row().unwrap().is_none());
        drop(second);
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.exists());
    }
    #[test]
    fn disk_point_index_bounds_each_repeated_probe_and_retains_its_run() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut sort = SortedRows::with_budget(&budget).unwrap();
        let directory = sort.0.directory.path().to_owned();
        for n in (0..4096).rev() { sort.push(row(&format!("{n:04}"), Value::Null)).unwrap(); }
        let prepared = sort.finish().unwrap().into_prepared();
        let index = prepared.point_index(&budget).unwrap();
        let retained = budget.reserved();
        assert!(retained < 4096, "resident owner metadata must not contain a key per row");
        let mut points = index.cursor(&budget).unwrap();
        drop(prepared); drop(index);
        assert!(directory.exists());
        for key in ["4095", "0000", "2048", "4096", "-absent", "0000", "4095"] {
            let before = points.probes;
            let expected = !matches!(key, "4096" | "-absent");
            assert_eq!(points.contains(&RecordId::new("test", key)).unwrap(), expected);
            assert!(points.probes - before <= 13, "each window must seek, never scan preceding rows");
        }
        assert!(!points.contains(&RecordId::new("other", "0000")).unwrap());
        drop(points);
        assert!(!directory.exists()); assert_eq!(budget.reserved(), 0);
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
        let directory = sorted.0.directory.path().to_owned();
        let before = row("before", Value::Null);
        let after = row("z-after", Value::Bytes(Bytes::from(vec![0, 255])));
        sorted.push(before.clone()).unwrap();
        sorted.push(boundary.clone()).unwrap();
        assert!(sorted.0.pending.is_empty());
        assert_eq!(sorted.0.payload_bytes, 0);
        sorted.push(boundary.clone()).unwrap();
        sorted.push(after.clone()).unwrap();
        let mut ordered = sorted.finish().unwrap();
        for expected in [before, boundary, after] {
            assert_eq!(ordered.next_row().unwrap(), Some(expected));
        }
        assert!(ordered.next_row().unwrap().is_none());
        drop(ordered);
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.exists());
    }

    #[test]
    fn legal_singleton_conflicts_with_a_different_full_value_across_runs() {
        let budget = ResourceBudget::fixed(128 << 20).unwrap();
        let mut sorted = SortedRows::with_budget(&budget).unwrap();
        let directory = sorted.0.directory.path().to_owned();
        sorted.push(row("before", Value::Null)).unwrap();
        sorted.push(ordinary_boundary_row("middle", 'x')).unwrap();
        sorted.push(ordinary_boundary_row("middle", 'y')).unwrap();
        assert!(matches!(
            sorted.finish(),
            Err(ModelError::Conflict("conflicting physical row identity"))
        ));
        assert_eq!(budget.reserved(), 0);
        assert!(!directory.exists());
    }

    #[test]
    fn legal_singleton_still_requires_its_composed_reservation() {
        let budget = ResourceBudget::fixed(RUN_BYTES).unwrap();
        let outer = budget.reserve("retained-input", 1024).unwrap();
        let mut sorted = SortedRows::with_budget(&budget).unwrap();
        let directory = sorted.0.directory.path().to_owned();
        assert!(matches!(
            sorted.push(ordinary_boundary_row("middle", 'x')),
            Err(ModelError::Resource { .. })
        ));
        assert!(matches!(
            sorted.finish(),
            Err(ModelError::Conflict("failed external ordering"))
        ));
        assert!(!directory.exists());
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
}
