//! Immutable compiler-private relation runs. IPC is pressure relief, never source authority.
//!
//! Arrow buffers carry their reservation through ordinary Arrow clones and slices. The compact
//! sorted nominal index is independently spillable; unions merge indexes without rewriting payload.
use arrow_array::{RecordBatch, make_array};
use arrow_schema::SchemaRef;
use datafusion::{
    arrow::{
        array::ArrayData,
        buffer::{BooleanBuffer, Buffer, NullBuffer},
        compute::interleave_record_batch,
        ipc::{reader::FileReader, writer::FileWriter},
    },
    catalog::{Session, TableProvider},
    common::{Statistics, stats::Precision},
    execution::TaskContext,
    logical_expr::{Expr, TableType},
    physical_plan::{
        ExecutionPlan, SendableRecordBatchStream,
        stream::RecordBatchStreamAdapter,
        streaming::{PartitionStream, StreamingTableExec},
    },
};
use lctx_model::domain::{
    CheckedCanonicalBatch, ContentHash, ModelError, Relation, RowFingerprint,
    resources::{Reclaimer, Reservation, ResourceBudget, TRANSFER_ROWS},
};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex, RwLock, Weak},
};

fn poisoned() -> ModelError {
    ModelError::Invalid("private source lock poisoned".into())
}
fn df_error(error: ModelError) -> datafusion::error::DataFusionError {
    datafusion::error::DataFusionError::External(Box::new(error))
}

#[derive(Debug)]
struct BufferOwner {
    buffer: Buffer,
    _charge: Arc<dyn Reservation>,
}
impl AsRef<[u8]> for BufferOwner {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}
fn owned_buffer(buffer: &Buffer, charge: &Arc<dyn Reservation>) -> Buffer {
    Buffer::from(bytes::Bytes::from_owner(BufferOwner {
        buffer: buffer.clone(),
        _charge: charge.clone(),
    }))
}
fn owned_data(data: ArrayData, charge: &Arc<dyn Reservation>) -> Result<ArrayData, ModelError> {
    let buffers = data
        .buffers()
        .iter()
        .map(|b| owned_buffer(b, charge))
        .collect();
    let children = data
        .child_data()
        .iter()
        .cloned()
        .map(|d| owned_data(d, charge))
        .collect::<Result<Vec<_>, _>>()?;
    let nulls = data.nulls().map(|n| {
        NullBuffer::new(BooleanBuffer::new(
            owned_buffer(n.inner().inner(), charge),
            n.offset(),
            n.len(),
        ))
    });
    data.into_builder()
        .buffers(buffers)
        .child_data(children)
        .nulls(nulls)
        .build()
        .map_err(ModelError::codec)
}
fn owned_batch(
    batch: RecordBatch,
    mut charge: Box<dyn Reservation>,
) -> Result<RecordBatch, ModelError> {
    fn wrappers(data: &ArrayData) -> usize {
        // Includes Bytes/Arc allocation headers, original Buffer handles, and ArrayData wrappers.
        512usize
            .saturating_add(data.buffers().len().saturating_mul(256))
            .saturating_add(data.child_data().iter().map(wrappers).sum::<usize>())
    }
    let metadata = batch
        .columns()
        .iter()
        .map(|a| wrappers(&a.to_data()))
        .sum::<usize>();
    charge.try_resize(charge.size().saturating_add(metadata))?;
    let charge: Arc<dyn Reservation> = charge.into();
    let arrays = batch
        .columns()
        .iter()
        .map(|a| owned_data(a.to_data(), &charge).map(make_array))
        .collect::<Result<Vec<_>, _>>()?;
    RecordBatch::try_new(batch.schema(), arrays).map_err(ModelError::codec)
}

#[derive(Debug)]
struct SealedSegment {
    file: tempfile::NamedTempFile,
    _directory: Arc<tempfile::TempDir>,
    allowance: usize,
    bytes: u64,
}
#[derive(Debug)]
enum SegmentBacking {
    Resident(RecordBatch),
    Ipc(Arc<SealedSegment>),
}
#[derive(Debug)]
struct Segment {
    backing: RwLock<SegmentBacking>,
    budget: ResourceBudget,
    directory: Arc<tempfile::TempDir>,
    schema: SchemaRef,
    _metadata: Box<dyn Reservation>,
}
impl Segment {
    fn read(&self) -> Result<RecordBatch, ModelError> {
        let file = {
            let backing = self.backing.read().map_err(|_| poisoned())?;
            match &*backing {
                SegmentBacking::Resident(batch) => return Ok(batch.clone()),
                SegmentBacking::Ipc(file) => file.clone(),
            }
        };
        let charge = self.budget.reserve("private IPC reader", file.allowance)?;
        let input = file.file.reopen().map_err(ModelError::codec)?;
        if input.metadata().map_err(ModelError::codec)?.len() != file.bytes {
            return Err(ModelError::Schema("private IPC sealed length"));
        }
        let mut reader = FileReader::try_new(input, None).map_err(ModelError::codec)?;
        if reader.schema() != self.schema || reader.num_batches() != 1 {
            return Err(ModelError::Schema("private IPC sealed shape"));
        }
        let batch = reader
            .next()
            .transpose()
            .map_err(ModelError::codec)?
            .ok_or(ModelError::Schema("private IPC missing batch"))?;
        owned_batch(batch, charge)
    }
    fn spill(&self) -> Result<(), ModelError> {
        let mut backing = self.backing.write().map_err(|_| poisoned())?;
        let SegmentBacking::Resident(batch) = &*backing else {
            return Ok(());
        };
        let allowance = batch
            .get_array_memory_size()
            .saturating_mul(3)
            .saturating_add(16 * 1024);
        let _work = self.budget.reserve("private IPC encoding", allowance)?;
        self.seal(&mut backing)
    }
    fn try_spill(&self, allowance: usize) -> Result<(), ModelError> {
        let Ok(mut backing) = self.backing.try_write() else {
            return Ok(());
        };
        let SegmentBacking::Resident(batch) = &*backing else {
            return Ok(());
        };
        if batch
            .get_array_memory_size()
            .saturating_mul(3)
            .saturating_add(16 * 1024)
            > allowance
        {
            return Ok(());
        }
        self.seal(&mut backing)
    }
    fn seal(&self, backing: &mut SegmentBacking) -> Result<(), ModelError> {
        let SegmentBacking::Resident(batch) = backing else {
            return Ok(());
        };
        let allowance = batch
            .get_array_memory_size()
            .saturating_mul(3)
            .saturating_add(16 * 1024);
        let mut file =
            tempfile::NamedTempFile::new_in(self.directory.path()).map_err(ModelError::codec)?;
        let mut writer = FileWriter::try_new(file.as_file_mut(), self.schema.as_ref())
            .map_err(ModelError::codec)?;
        writer.write(batch).map_err(ModelError::codec)?;
        writer.finish().map_err(ModelError::codec)?;
        drop(writer);
        let bytes = file.as_file().metadata().map_err(ModelError::codec)?.len();
        *backing = SegmentBacking::Ipc(Arc::new(SealedSegment {
            file,
            _directory: self.directory.clone(),
            allowance,
            bytes,
        }));
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    key: [u8; 16],
    content: ContentHash,
    segment: usize,
    row: usize,
}
const ENTRY_BYTES: usize = 64;
impl Entry {
    fn encode(self) -> Result<[u8; ENTRY_BYTES], ModelError> {
        let mut bytes = [0; ENTRY_BYTES];
        bytes[..16].copy_from_slice(&self.key);
        bytes[16..48].copy_from_slice(&self.content.0);
        bytes[48..56].copy_from_slice(
            &u64::try_from(self.segment)
                .map_err(ModelError::codec)?
                .to_le_bytes(),
        );
        bytes[56..64].copy_from_slice(
            &u64::try_from(self.row)
                .map_err(ModelError::codec)?
                .to_le_bytes(),
        );
        Ok(bytes)
    }
    fn decode(bytes: [u8; ENTRY_BYTES]) -> Result<Self, ModelError> {
        Ok(Self {
            key: bytes[..16].try_into().map_err(ModelError::codec)?,
            content: ContentHash(bytes[16..48].try_into().map_err(ModelError::codec)?),
            segment: usize::try_from(u64::from_le_bytes(
                bytes[48..56].try_into().map_err(ModelError::codec)?,
            ))
            .map_err(ModelError::codec)?,
            row: usize::try_from(u64::from_le_bytes(
                bytes[56..64].try_into().map_err(ModelError::codec)?,
            ))
            .map_err(ModelError::codec)?,
        })
    }
}
#[derive(Debug)]
enum IndexBacking {
    Resident {
        entries: Arc<Vec<Entry>>,
        _charge: Arc<dyn Reservation>,
    },
    Disk {
        file: Arc<tempfile::NamedTempFile>,
        _directory: Arc<tempfile::TempDir>,
    },
}
#[derive(Debug)]
struct Index {
    backing: RwLock<IndexBacking>,
    len: usize,
    budget: ResourceBudget,
    directory: Arc<tempfile::TempDir>,
    _metadata: Box<dyn Reservation>,
}
impl Index {
    fn cursor(self: &Arc<Self>) -> Result<IndexCursor, ModelError> {
        self.cursor_at(0)
    }
    fn cursor_at(self: &Arc<Self>, position: usize) -> Result<IndexCursor, ModelError> {
        if position > self.len {
            return Err(ModelError::Schema("private index cursor position"));
        }
        let backing = self.backing.read().map_err(|_| poisoned())?;
        let source = match &*backing {
            IndexBacking::Resident { entries, _charge } => CursorBacking::Resident {
                entries: entries.clone(),
                _charge: _charge.clone(),
            },
            IndexBacking::Disk { file, .. } => {
                let charge = self
                    .budget
                    .reserve("private index reader", 8192 + ENTRY_BYTES)?;
                let mut input = file.reopen().map_err(ModelError::codec)?;
                if input.metadata().map_err(ModelError::codec)?.len()
                    != (self.len as u64).saturating_mul(ENTRY_BYTES as u64)
                {
                    return Err(ModelError::Schema("private index sealed length"));
                }
                input
                    .seek(SeekFrom::Start(
                        (position as u64).saturating_mul(ENTRY_BYTES as u64),
                    ))
                    .map_err(ModelError::codec)?;
                CursorBacking::Disk {
                    reader: BufReader::new(input),
                    _file: file.clone(),
                    _charge: charge,
                }
            }
        };
        Ok(IndexCursor {
            source,
            index: position,
            len: self.len,
            _owner: self.clone(),
        })
    }
    fn spill(&self) -> Result<(), ModelError> {
        let mut backing = self.backing.write().map_err(|_| poisoned())?;
        let _work = self.budget.reserve("private index spill", 8192)?;
        self.seal(&mut backing)
    }
    fn try_spill(&self) -> Result<(), ModelError> {
        let Ok(mut backing) = self.backing.try_write() else {
            return Ok(());
        };
        self.seal(&mut backing)
    }
    fn seal(&self, backing: &mut IndexBacking) -> Result<(), ModelError> {
        let IndexBacking::Resident { entries, .. } = backing else {
            return Ok(());
        };
        let file =
            tempfile::NamedTempFile::new_in(self.directory.path()).map_err(ModelError::codec)?;
        let mut writer = BufWriter::new(file.as_file());
        for entry in entries.iter() {
            writer
                .write_all(&entry.encode()?)
                .map_err(ModelError::codec)?;
        }
        writer.flush().map_err(ModelError::codec)?;
        drop(writer);
        *backing = IndexBacking::Disk {
            file: Arc::new(file),
            _directory: self.directory.clone(),
        };
        Ok(())
    }
}

/// Pressure relief for actual immutable backing, attached to the existing shared budget.
/// Weak registrations do not retain obsolete sources. One pre-admitted encoding workspace
/// makes reclamation possible even when a new provider/compute allocation exhausts the pool.
#[derive(Debug)]
pub struct PrivateSpill {
    base: ResourceBudget,
    workspace: Mutex<Box<dyn Reservation>>,
    registry: Mutex<SpillRegistry>,
}
#[derive(Debug)]
struct SpillRegistry {
    segments: Vec<Weak<Segment>>,
    indexes: Vec<Weak<Index>>,
    charge: Box<dyn Reservation>,
}
impl PrivateSpill {
    pub fn new(base: ResourceBudget) -> Result<Arc<Self>, ModelError> {
        let workspace = base.reserve("private spill encoding workspace", 16 * 1024)?;
        let charge = base.reserve("private spill weak registry", size_of::<Self>() + 512)?;
        Ok(Arc::new(Self {
            base,
            workspace: Mutex::new(workspace),
            registry: Mutex::new(SpillRegistry {
                segments: Vec::new(),
                indexes: Vec::new(),
                charge,
            }),
        }))
    }
    fn prepare(&self, bytes: usize) -> Result<(), ModelError> {
        {
            let mut workspace = self.workspace.lock().map_err(|_| poisoned())?;
            if workspace.size() >= bytes {
                return Ok(());
            }
            if workspace.try_resize(bytes).is_ok() {
                return Ok(());
            }
        }
        self.reclaim(bytes)?;
        self.workspace
            .lock()
            .map_err(|_| poisoned())?
            .try_resize(bytes)
    }
    fn register(
        &self,
        segment: Option<&Arc<Segment>>,
        index: &Arc<Index>,
    ) -> Result<(), ModelError> {
        let mut registry = self.registry.lock().map_err(|_| poisoned())?;
        registry
            .segments
            .retain(|source| source.strong_count() != 0);
        registry.indexes.retain(|source| source.strong_count() != 0);
        let segment_slots =
            if segment.is_some() && registry.segments.len() == registry.segments.capacity() {
                registry.segments.capacity().max(4)
            } else {
                0
            };
        let index_slots = if registry.indexes.len() == registry.indexes.capacity() {
            registry.indexes.capacity().max(4)
        } else {
            0
        };
        let bytes = registry.charge.size().saturating_add(
            segment_slots * size_of::<Weak<Segment>>() + index_slots * size_of::<Weak<Index>>(),
        );
        registry.charge.try_resize(bytes)?;
        registry.segments.reserve_exact(segment_slots);
        registry.indexes.reserve_exact(index_slots);
        if let Some(segment) = segment {
            registry.segments.push(Arc::downgrade(segment));
        }
        registry.indexes.push(Arc::downgrade(index));
        Ok(())
    }
}
impl Reclaimer for PrivateSpill {
    fn reclaim(&self, bytes: usize) -> Result<(), ModelError> {
        let Ok(workspace) = self.workspace.try_lock() else {
            return Ok(());
        };
        let Ok(registry) = self.registry.try_lock() else {
            return Ok(());
        };
        for index in &registry.indexes {
            if self.base.limit().saturating_sub(self.base.reserved()) >= bytes {
                return Ok(());
            }
            if let Some(index) = index.upgrade() {
                index.try_spill()?;
            }
        }
        for segment in &registry.segments {
            if self.base.limit().saturating_sub(self.base.reserved()) >= bytes {
                return Ok(());
            }
            if let Some(segment) = segment.upgrade() {
                segment.try_spill(workspace.size())?;
            }
        }
        Ok(())
    }
}
enum CursorBacking {
    Resident {
        entries: Arc<Vec<Entry>>,
        _charge: Arc<dyn Reservation>,
    },
    Disk {
        reader: BufReader<File>,
        _file: Arc<tempfile::NamedTempFile>,
        _charge: Box<dyn Reservation>,
    },
}
struct IndexCursor {
    source: CursorBacking,
    index: usize,
    len: usize,
    _owner: Arc<Index>,
}
impl Iterator for IndexCursor {
    type Item = Result<Entry, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }
        let row = self.index;
        self.index += 1;
        Some(match &mut self.source {
            CursorBacking::Resident { entries, .. } => Ok(entries[row]),
            CursorBacking::Disk { reader, .. } => {
                let mut bytes = [0; ENTRY_BYTES];
                reader
                    .read_exact(&mut bytes)
                    .map_err(ModelError::codec)
                    .and_then(|()| Entry::decode(bytes))
            }
        })
    }
}
enum BuilderBacking {
    Resident {
        entries: Vec<Entry>,
        charge: Box<dyn Reservation>,
    },
    Disk {
        writer: BufWriter<File>,
        file: tempfile::NamedTempFile,
        _charge: Box<dyn Reservation>,
    },
}
struct IndexBuilder {
    backing: BuilderBacking,
    len: usize,
    budget: ResourceBudget,
    directory: Arc<tempfile::TempDir>,
    metadata: Box<dyn Reservation>,
}
impl IndexBuilder {
    fn new(
        upper: usize,
        budget: &ResourceBudget,
        directory: &Arc<tempfile::TempDir>,
    ) -> Result<Self, ModelError> {
        let metadata = budget.reserve("private index metadata", size_of::<Index>() + 512)?;
        let bytes = upper
            .checked_mul(size_of::<Entry>())
            .ok_or_else(|| ModelError::Invalid("private index size overflow".into()))?;
        let backing = match budget.reserve("private nominal index", bytes) {
            Ok(charge) => BuilderBacking::Resident {
                entries: Vec::with_capacity(upper),
                charge,
            },
            Err(ModelError::Resource { .. }) => {
                let charge = budget.reserve("private nominal index writer", 8192)?;
                let file =
                    tempfile::NamedTempFile::new_in(directory.path()).map_err(ModelError::codec)?;
                let writer = BufWriter::new(file.reopen().map_err(ModelError::codec)?);
                BuilderBacking::Disk {
                    writer,
                    file,
                    _charge: charge,
                }
            }
            Err(error) => return Err(error),
        };
        Ok(Self {
            backing,
            len: 0,
            budget: budget.clone(),
            directory: directory.clone(),
            metadata,
        })
    }
    fn push(&mut self, entry: Entry) -> Result<(), ModelError> {
        match &mut self.backing {
            BuilderBacking::Resident { entries, .. } => entries.push(entry),
            BuilderBacking::Disk { writer, .. } => writer
                .write_all(&entry.encode()?)
                .map_err(ModelError::codec)?,
        }
        self.len += 1;
        Ok(())
    }
    fn finish(self) -> Result<Arc<Index>, ModelError> {
        let backing = match self.backing {
            BuilderBacking::Resident { entries, charge } => IndexBacking::Resident {
                entries: Arc::new(entries),
                _charge: charge.into(),
            },
            BuilderBacking::Disk {
                mut writer, file, ..
            } => {
                writer.flush().map_err(ModelError::codec)?;
                drop(writer);
                IndexBacking::Disk {
                    file: Arc::new(file),
                    _directory: self.directory.clone(),
                }
            }
        };
        Ok(Arc::new(Index {
            backing: RwLock::new(backing),
            len: self.len,
            budget: self.budget,
            directory: self.directory,
            _metadata: self.metadata,
        }))
    }
}

/// A completed immutable relation. Its data authority comes from checked producer ingress;
/// neither an IPC file nor the content digest is an independently admitted product.
#[derive(Debug)]
pub struct PrivateRelation {
    model: ContentHash,
    relation: Relation,
    segments: Vec<Arc<Segment>>,
    index: Arc<Index>,
    content: ContentHash,
    budget: ResourceBudget,
    directory: Arc<tempfile::TempDir>,
    _metadata: Box<dyn Reservation>,
    provider: Mutex<Option<Weak<dyn TableProvider>>>,
    spill: Option<Arc<PrivateSpill>>,
}
impl PrivateRelation {
    pub fn shares_pool(&self, budget: &ResourceBudget) -> bool {
        self.budget.shares_pool(budget)
    }
    pub fn model(&self) -> ContentHash {
        self.model
    }
    pub fn relation(&self) -> &Relation {
        &self.relation
    }
    pub fn rows(&self) -> u64 {
        self.index.len as u64
    }
    pub fn content(&self) -> ContentHash {
        self.content
    }
    pub fn empty(
        model: ContentHash,
        relation: Relation,
        budget: ResourceBudget,
        directory: Arc<tempfile::TempDir>,
    ) -> Result<Arc<Self>, ModelError> {
        PrivateWriter::new(model, relation, budget, directory)?.finish()
    }
    pub fn provider(self: &Arc<Self>) -> Arc<dyn TableProvider> {
        let mut cached = self.provider.lock().expect("private provider cache");
        if let Some(provider) = cached.as_ref().and_then(Weak::upgrade) {
            return provider;
        }
        let provider: Arc<dyn TableProvider> = Arc::new(PrivateTable(self.clone()));
        *cached = Some(Arc::downgrade(&provider));
        provider
    }
    pub fn read_batches(self: &Arc<Self>) -> Result<PrivateBatches, ModelError> {
        Ok(PrivateBatches {
            source: self.clone(),
            cursor: self.index.cursor()?.peekable(),
            active: None,
            position: 0,
        })
    }
    /// Representation can change while previous zero-copy borrowers retain the resident buffers.
    pub fn spill_resident(&self) -> Result<(), ModelError> {
        self.index.spill()?;
        for segment in &self.segments {
            segment.spill()?;
        }
        Ok(())
    }
    pub fn merge(left: &Arc<Self>, right: &Arc<Self>) -> Result<Arc<Self>, ModelError> {
        if left.model != right.model
            || left.relation.type_id() != right.relation.type_id()
            || !left.budget.shares_pool(&right.budget)
            || !Arc::ptr_eq(&left.directory, &right.directory)
        {
            return Err(ModelError::Conflict("private source identity"));
        }
        if Arc::ptr_eq(left, right) {
            return Ok(left.clone());
        }
        let segment_count = left
            .segments
            .len()
            .checked_add(right.segments.len())
            .ok_or_else(|| ModelError::Invalid("private source segment count overflow".into()))?;
        let metadata = left.budget.reserve(
            "private relation segments",
            size_of::<Self>() + segment_count * size_of::<Arc<Segment>>(),
        )?;
        let mut segments = Vec::with_capacity(segment_count);
        segments.extend(left.segments.iter().cloned());
        let offset = segments.len();
        segments.extend(right.segments.iter().cloned());
        let index = merge_indexes(
            &left.index,
            &right.index,
            offset,
            &segments,
            &left.relation,
            &left.budget,
            &left.directory,
        )?;
        let content = index_content(&left.relation, &index)?;
        let spill = left.spill.clone().or_else(|| right.spill.clone());
        if let Some(spill) = &spill {
            spill.register(None, &index)?;
        }
        Ok(Arc::new(Self {
            model: left.model,
            relation: left.relation.clone(),
            segments,
            index,
            content,
            budget: left.budget.clone(),
            directory: left.directory.clone(),
            _metadata: metadata,
            provider: Mutex::new(None),
            spill,
        }))
    }
}

/// Mutable ingress belongs only to its producer. Successful finish publishes one immutable source.
pub struct PrivateWriter {
    model: ContentHash,
    relation: Relation,
    budget: ResourceBudget,
    directory: Arc<tempfile::TempDir>,
    segments: Vec<Arc<Segment>>,
    runs: VecDeque<Arc<Index>>,
    metadata: Box<dyn Reservation>,
    spill: Option<Arc<PrivateSpill>>,
    failed: bool,
}
impl PrivateWriter {
    pub fn new(
        model: ContentHash,
        relation: Relation,
        budget: ResourceBudget,
        directory: Arc<tempfile::TempDir>,
    ) -> Result<Self, ModelError> {
        Self::new_with_spill(model, relation, budget, directory, None)
    }
    pub fn new_with_spill(
        model: ContentHash,
        relation: Relation,
        budget: ResourceBudget,
        directory: Arc<tempfile::TempDir>,
        spill: Option<Arc<PrivateSpill>>,
    ) -> Result<Self, ModelError> {
        let metadata = budget.reserve("private writer metadata", size_of::<Self>())?;
        Ok(Self {
            model,
            relation,
            budget,
            directory,
            segments: Vec::new(),
            runs: VecDeque::new(),
            metadata,
            spill,
            failed: false,
        })
    }
    pub fn push(&mut self, batch: CheckedCanonicalBatch) -> Result<(), ModelError> {
        if self.failed {
            return Err(ModelError::Conflict("private writer previously failed"));
        }
        let result = self.push_inner(batch);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn push_inner(&mut self, batch: CheckedCanonicalBatch) -> Result<(), ModelError> {
        if !batch.shares_pool(&self.budget) {
            return Err(ModelError::Conflict("private batch allocation pool"));
        }
        if batch.model() != self.model || batch.relation().type_id() != self.relation.type_id() {
            return Err(ModelError::Conflict("private producer batch identity"));
        }
        if batch.arrow().num_rows() == 0 {
            return Ok(());
        }
        let fingerprints = batch.fingerprints();
        if fingerprints
            .windows(2)
            .any(|p| p[0].nominal >= p[1].nominal)
        {
            return Err(ModelError::Schema("private checked canonical order"));
        }
        let mut index = match IndexBuilder::new(fingerprints.len(), &self.budget, &self.directory) {
            Ok(index) => index,
            Err(ModelError::Resource { .. }) => {
                self.spill()?;
                IndexBuilder::new(fingerprints.len(), &self.budget, &self.directory)?
            }
            Err(error) => return Err(error),
        };
        let segment = self.segments.len();
        for (row, fingerprint) in fingerprints.iter().enumerate() {
            index.push(Entry {
                key: fingerprint.nominal,
                content: fingerprint.content,
                segment,
                row,
            })?;
        }
        let bytes = size_of::<Self>()
            + (self.segments.len() + 1) * size_of::<Arc<Segment>>() * 2
            + (self.runs.len() + 1) * size_of::<Arc<Index>>() * 2;
        self.metadata.try_resize(bytes)?;
        let segment_metadata = self
            .budget
            .reserve("private segment metadata", size_of::<Segment>() + 1024)?;
        if let Some(spill) = &self.spill {
            spill.prepare(
                batch
                    .arrow()
                    .get_array_memory_size()
                    .saturating_mul(3)
                    .saturating_add(32 * 1024),
            )?;
        }
        let (_, _, arrow, charge) = batch.into_parts();
        let arrow = owned_batch(arrow, charge)?;
        self.segments.push(Arc::new(Segment {
            backing: RwLock::new(SegmentBacking::Resident(arrow)),
            budget: self.budget.clone(),
            directory: self.directory.clone(),
            schema: self.relation.schema().clone(),
            _metadata: segment_metadata,
        }));
        let index = index.finish()?;
        if let Some(spill) = &self.spill {
            spill.register(self.segments.last(), &index)?;
        }
        self.runs.push_back(index);
        Ok(())
    }
    fn spill(&self) -> Result<(), ModelError> {
        for run in &self.runs {
            run.spill()?;
        }
        for segment in &self.segments {
            segment.spill()?;
        }
        Ok(())
    }
    pub fn finish(mut self) -> Result<Arc<PrivateRelation>, ModelError> {
        if self.failed {
            return Err(ModelError::Conflict(
                "failed private writer cannot complete",
            ));
        }
        while self.runs.len() > 1 {
            let a = self.runs.pop_front().expect("two runs");
            let b = self.runs.pop_front().expect("two runs");
            self.runs.push_back(merge_indexes(
                &a,
                &b,
                0,
                &self.segments,
                &self.relation,
                &self.budget,
                &self.directory,
            )?);
        }
        let index = match self.runs.pop_front() {
            Some(index) => index,
            None => IndexBuilder::new(0, &self.budget, &self.directory)?.finish()?,
        };
        let content = index_content(&self.relation, &index)?;
        if let Some(spill) = &self.spill {
            spill.register(None, &index)?;
        }
        Ok(Arc::new(PrivateRelation {
            model: self.model,
            relation: self.relation,
            segments: self.segments,
            index,
            content,
            budget: self.budget,
            directory: self.directory,
            _metadata: self.metadata,
            provider: Mutex::new(None),
            spill: self.spill,
        }))
    }
}
fn index_content(relation: &Relation, index: &Arc<Index>) -> Result<ContentHash, ModelError> {
    let mut content = relation.content();
    for entry in index.cursor()? {
        let entry = entry?;
        content.push_fingerprint(RowFingerprint {
            nominal: entry.key,
            content: entry.content,
        })?;
    }
    Ok(content.finish().1)
}
fn merge_indexes(
    a: &Arc<Index>,
    b: &Arc<Index>,
    offset: usize,
    segments: &[Arc<Segment>],
    relation: &Relation,
    budget: &ResourceBudget,
    directory: &Arc<tempfile::TempDir>,
) -> Result<Arc<Index>, ModelError> {
    let mut output = IndexBuilder::new(a.len.saturating_add(b.len), budget, directory)?;
    let mut a = a.cursor()?.peekable();
    let mut b = b
        .cursor()?
        .map(|entry| {
            entry.map(|mut e| {
                e.segment += offset;
                e
            })
        })
        .peekable();
    let mut compared = [None::<(usize, RecordBatch)>, None];
    loop {
        let ak = a.peek().map(|e| e.as_ref().map(|e| e.key).map_err(|_| ()));
        let bk = b.peek().map(|e| e.as_ref().map(|e| e.key).map_err(|_| ()));
        let entry = match (ak, bk) {
            (None, None) => break,
            (Some(Err(())), _) => {
                return Err(a.next().expect("peeked entry").expect_err("peeked error"));
            }
            (_, Some(Err(()))) => {
                return Err(b.next().expect("peeked entry").expect_err("peeked error"));
            }
            (Some(Ok(ak)), Some(Ok(bk))) if ak == bk => {
                let left = a.next().expect("peeked entry")?;
                let right = b.next().expect("peeked entry")?;
                if left.content != right.content
                    || !equal_rows(left, right, segments, &mut compared)?
                {
                    return Err(ModelError::Conflict(relation.name()));
                }
                left
            }
            (Some(Ok(ak)), Some(Ok(bk))) if ak < bk => a.next().expect("peeked entry")?,
            (Some(_), None) => a.next().expect("peeked entry")?,
            _ => b.next().expect("peeked entry")?,
        };
        output.push(entry)?;
    }
    output.finish()
}
fn equal_rows(
    a: Entry,
    b: Entry,
    segments: &[Arc<Segment>],
    cache: &mut [Option<(usize, RecordBatch)>; 2],
) -> Result<bool, ModelError> {
    if a.segment == b.segment && a.row == b.row {
        return Ok(true);
    }
    if cache[0]
        .as_ref()
        .is_none_or(|(segment, _)| *segment != a.segment)
    {
        cache[0] = Some((a.segment, segments[a.segment].read()?));
    }
    if cache[1]
        .as_ref()
        .is_none_or(|(segment, _)| *segment != b.segment)
    {
        cache[1] = Some((b.segment, segments[b.segment].read()?));
    }
    let a_batch = &cache[0].as_ref().expect("left compared segment").1;
    let b_batch = &cache[1].as_ref().expect("right compared segment").1;
    Ok(a_batch
        .columns()
        .iter()
        .zip(b_batch.columns())
        .all(|(left, right)| left.slice(a.row, 1).to_data() == right.slice(b.row, 1).to_data()))
}

/// Canonical ordered zero-copy windows. Returned batches independently retain backing charges.
pub struct PrivateBatches {
    source: Arc<PrivateRelation>,
    cursor: std::iter::Peekable<IndexCursor>,
    active: Option<(usize, RecordBatch)>,
    position: usize,
}
impl Iterator for PrivateBatches {
    type Item = Result<RecordBatch, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        let first = match self.cursor.next()? {
            Ok(entry) => entry,
            Err(error) => return Some(Err(error)),
        };
        let mut count = 1;
        while count < TRANSFER_ROWS
            && self.cursor.peek().is_some_and(|entry| {
                entry
                    .as_ref()
                    .is_ok_and(|e| e.segment == first.segment && e.row == first.row + count)
            })
        {
            let _ = self.cursor.next();
            count += 1;
        }
        if self
            .active
            .as_ref()
            .is_none_or(|(segment, _)| *segment != first.segment)
        {
            match self.source.segments[first.segment].read() {
                Ok(batch) => self.active = Some((first.segment, batch)),
                Err(error) => return Some(Err(error)),
            }
        }
        let original_count = count;
        while count > 1
            && lctx_model::domain::logical_batch_bytes(
                &self
                    .active
                    .as_ref()
                    .expect("active batch")
                    .1
                    .slice(first.row, count),
            )
            .is_ok_and(|bytes| bytes > lctx_model::domain::resources::TRANSFER_BYTES)
        {
            count = count.div_ceil(2);
        }
        if count != original_count {
            match self.source.index.cursor_at(self.position + count) {
                Ok(cursor) => self.cursor = cursor.peekable(),
                Err(error) => return Some(Err(error)),
            }
        }
        self.position += count;
        if count != original_count || count == TRANSFER_ROWS || self.cursor.peek().is_none() {
            return Some(Ok(self
                .active
                .as_ref()
                .expect("active batch")
                .1
                .slice(first.row, count)));
        }
        Some(self.gather(first, count))
    }
}
impl PrivateBatches {
    fn gather(&mut self, first: Entry, count: usize) -> Result<RecordBatch, ModelError> {
        use lctx_model::domain::{logical_batch_bytes, resources::TRANSFER_BYTES};
        let fields = self.source.relation.schema().fields().len();
        // Index slots, input handles and Arrow's interleave source metadata are bounded by
        // the same transfer window. Payload reservations stay on the actual source arrays.
        let _metadata = self.source.budget.reserve(
            "private canonical gather metadata",
            TRANSFER_ROWS.saturating_mul(256 + fields.saturating_mul(64)),
        )?;
        let mut output_charge = self
            .source
            .budget
            .reserve("private canonical gather buffers", 0)?;
        let first_batch = self.active.as_ref().expect("active batch").1.clone();
        let mut bytes = logical_batch_bytes(&first_batch.slice(first.row, count))?;
        output_charge.try_resize(bytes.saturating_mul(3).saturating_add(4096))?;
        let mut batches = vec![first_batch];
        let mut positions = BTreeMap::from([(first.segment, 0usize)]);
        let mut indices = (first.row..first.row + count)
            .map(|row| (0usize, row))
            .collect::<Vec<_>>();
        while indices.len() < TRANSFER_ROWS {
            let Some(next) = self.cursor.peek() else {
                break;
            };
            let next = match next {
                Ok(entry) => *entry,
                Err(_) => {
                    return Err(self
                        .cursor
                        .next()
                        .expect("peeked entry")
                        .expect_err("peeked error"));
                }
            };
            let position = match positions.get(&next.segment) {
                Some(position) => *position,
                None => {
                    let batch = match self.source.segments[next.segment].read() {
                        Ok(batch) => batch,
                        Err(ModelError::Resource { .. }) if !indices.is_empty() => break,
                        Err(error) => return Err(error),
                    };
                    let position = batches.len();
                    batches.push(batch);
                    positions.insert(next.segment, position);
                    position
                }
            };
            let row_bytes = logical_batch_bytes(&batches[position].slice(next.row, 1))?;
            if bytes.saturating_add(row_bytes) > TRANSFER_BYTES {
                break;
            }
            output_charge.try_resize(
                bytes
                    .saturating_add(row_bytes)
                    .saturating_mul(3)
                    .saturating_add(4096),
            )?;
            bytes += row_bytes;
            indices.push((position, next.row));
            let _ = self.cursor.next();
            self.position += 1;
        }
        let refs = batches.iter().collect::<Vec<_>>();
        let batch = interleave_record_batch(&refs, &indices).map_err(ModelError::codec)?;
        owned_batch(batch, output_charge)
    }
}
#[derive(Debug)]
struct PrivateTable(Arc<PrivateRelation>);
#[async_trait::async_trait]
impl TableProvider for PrivateTable {
    fn schema(&self) -> SchemaRef {
        self.0.relation.schema().clone()
    }
    fn table_type(&self) -> TableType {
        TableType::Base
    }
    fn statistics(&self) -> Option<Statistics> {
        let mut stats = Statistics::new_unknown(self.schema().as_ref());
        stats.num_rows = Precision::Exact(self.0.index.len);
        Some(stats)
    }
    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        _filters: &[Expr],
        limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(StreamingTableExec::try_new(
            self.schema(),
            vec![Arc::new(PrivatePartition(self.0.clone()))],
            projection,
            [],
            false,
            limit,
        )?))
    }
}
#[derive(Debug)]
struct PrivatePartition(Arc<PrivateRelation>);
impl PartitionStream for PrivatePartition {
    fn schema(&self) -> &SchemaRef {
        self.0.relation.schema()
    }
    fn execute(&self, _context: Arc<TaskContext>) -> SendableRecordBatchStream {
        let batches = self.0.read_batches();
        let stream = futures::stream::try_unfold(batches, |state| async move {
            let mut batches = state.map_err(df_error)?;
            match batches.next() {
                Some(batch) => {
                    tokio::task::yield_now().await;
                    Ok(Some((batch.map_err(df_error)?, Ok(batches))))
                }
                None => Ok(None),
            }
        });
        Box::pin(RecordBatchStreamAdapter::new(
            self.0.relation.schema().clone(),
            stream,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::{
        Domain,
        domain::{Batch, Record, ValidatedModel},
    };

    #[derive(Clone, Debug, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
    #[model(name = "private_source_control_rows")]
    struct Row {
        #[model(key)]
        key: String,
        payload: String,
    }
    fn setup() -> (ValidatedModel, ResourceBudget, Arc<tempfile::TempDir>) {
        (
            ValidatedModel::declared(vec![Relation::of::<Row>()]).unwrap(),
            ResourceBudget::fixed(32 << 20).unwrap(),
            Arc::new(tempfile::tempdir().unwrap()),
        )
    }
    fn row(key: &str, payload: &str) -> Row {
        Row {
            key: key.into(),
            payload: payload.into(),
        }
    }
    fn source(
        model: &ValidatedModel,
        budget: &ResourceBudget,
        directory: &Arc<tempfile::TempDir>,
        rows: Vec<Row>,
    ) -> Arc<PrivateRelation> {
        let mut writer = PrivateWriter::new(
            model.digest(),
            Relation::of::<Row>(),
            budget.clone(),
            directory.clone(),
        )
        .unwrap();
        writer
            .push(
                Batch::new(model, rows, budget)
                    .unwrap()
                    .into_checked()
                    .unwrap(),
            )
            .unwrap();
        writer.finish().unwrap()
    }
    fn collected(source: &Arc<PrivateRelation>) -> Vec<Row> {
        source
            .read_batches()
            .unwrap()
            .flat_map(|batch| Row::decode(&batch.unwrap()).unwrap())
            .collect()
    }
    #[test]
    fn resident_union_is_exact_canonical_without_rewriting_payload() {
        let (model, budget, directory) = setup();
        let a = source(
            &model,
            &budget,
            &directory,
            vec![row("a", "one"), row("b", "two")],
        );
        let b = source(
            &model,
            &budget,
            &directory,
            vec![row("a", "one"), row("c", "three")],
        );
        let combined = PrivateRelation::merge(&a, &b).unwrap();
        assert_eq!(combined.rows(), 3);
        assert!(Arc::ptr_eq(&combined.segments[0], &a.segments[0]));
        let mut expected = vec![row("a", "one"), row("b", "two"), row("c", "three")];
        expected.sort_by_key(Record::id);
        assert_eq!(collected(&combined), expected);
        let mut content = combined.relation.content();
        for batch in combined.read_batches().unwrap() {
            combined
                .relation
                .hash_rows(&batch.unwrap(), &mut content)
                .unwrap();
        }
        assert_eq!(content.finish(), (combined.rows(), combined.content()));
        assert_eq!(
            std::fs::read_dir(directory.path()).unwrap().count(),
            0,
            "ordinary handoff must not create IPC or index files"
        );
    }
    #[test]
    fn unequal_duplicate_refuses_and_equal_cross_batch_duplicate_collapses() {
        let (model, budget, directory) = setup();
        let a = source(&model, &budget, &directory, vec![row("same", "first")]);
        let b = source(&model, &budget, &directory, vec![row("same", "changed")]);
        assert!(matches!(
            PrivateRelation::merge(&a, &b),
            Err(ModelError::Conflict(_))
        ));
        // Deliberately simulate a fingerprint collision: nominal/content hashing is a useful
        // rejection index, never permission to discard a distinct actual typed payload.
        {
            let original = a.index.cursor().unwrap().next().unwrap().unwrap();
            let mut backing = b.index.backing.write().unwrap();
            let IndexBacking::Resident { entries, .. } = &mut *backing else {
                panic!("resident control");
            };
            Arc::get_mut(entries).unwrap()[0].content = original.content;
        }
        assert!(matches!(
            PrivateRelation::merge(&a, &b),
            Err(ModelError::Conflict(_))
        ));
        let mut writer = PrivateWriter::new(
            model.digest(),
            Relation::of::<Row>(),
            budget.clone(),
            directory,
        )
        .unwrap();
        for _ in 0..2 {
            writer
                .push(
                    Batch::new(&model, vec![row("same", "first")], &budget)
                        .unwrap()
                        .into_checked()
                        .unwrap(),
                )
                .unwrap();
        }
        assert_eq!(writer.finish().unwrap().rows(), 1);
    }
    #[test]
    fn explicit_empty_and_foreign_model_are_distinct() {
        let (model, budget, directory) = setup();
        let empty = PrivateRelation::empty(
            model.digest(),
            Relation::of::<Row>(),
            budget.clone(),
            directory.clone(),
        )
        .unwrap();
        assert_eq!(empty.rows(), 0);
        assert!(empty.read_batches().unwrap().next().is_none());
        assert_eq!(empty.content(), Relation::of::<Row>().content().finish().1);
        let foreign = ValidatedModel::declared(vec![
            Relation::of::<Row>(),
            Relation::of::<lctx_model::domain::input::Package>(),
        ])
        .unwrap();
        let mut writer = PrivateWriter::new(
            model.digest(),
            Relation::of::<Row>(),
            budget.clone(),
            directory,
        )
        .unwrap();
        assert!(matches!(
            writer.push(
                Batch::new(&foreign, vec![row("a", "one")], &budget)
                    .unwrap()
                    .into_checked()
                    .unwrap()
            ),
            Err(ModelError::Conflict(_))
        ));
    }
    #[test]
    fn foreign_allocation_cannot_be_relabelled_as_this_operation() {
        let (model, budget, directory) = setup();
        let foreign = ResourceBudget::fixed(budget.limit()).unwrap();
        let mut writer =
            PrivateWriter::new(model.digest(), Relation::of::<Row>(), budget, directory).unwrap();
        assert!(matches!(
            writer.push(
                Batch::new(&model, vec![row("a", "one")], &foreign)
                    .unwrap()
                    .into_checked()
                    .unwrap()
            ),
            Err(ModelError::Conflict("private batch allocation pool"))
        ));
        assert_eq!(foreign.reserved(), 0);
    }
    #[test]
    fn canonical_hash_interleaving_coalesces_into_a_bounded_transfer() {
        let (model, budget, directory) = setup();
        let mut writer =
            PrivateWriter::new(model.digest(), Relation::of::<Row>(), budget, directory).unwrap();
        for part in 0..8 {
            writer
                .push(
                    Batch::new(
                        &model,
                        (0..128)
                            .map(|i| row(&format!("part-{part}-key-{i}"), "payload"))
                            .collect(),
                        &writer.budget,
                    )
                    .unwrap()
                    .into_checked()
                    .unwrap(),
                )
                .unwrap();
        }
        let source = writer.finish().unwrap();
        let batches = source
            .read_batches()
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            batches.len(),
            1,
            "independently sorted hash runs must not become singleton transfers"
        );
        assert_eq!(batches[0].num_rows(), 1024);
        let mut content = source.relation.content();
        source
            .relation
            .hash_rows(&batches[0], &mut content)
            .unwrap();
        assert_eq!(content.finish(), (source.rows(), source.content()));
    }
    #[test]
    fn ordinary_pool_pressure_reclaims_completed_payload_with_reserved_workspace() {
        let (model, base, directory) = setup();
        let spill = PrivateSpill::new(base.clone()).unwrap();
        let budget = base.with_reclaimer(spill.clone());
        let mut writer = PrivateWriter::new_with_spill(
            model.digest(),
            Relation::of::<Row>(),
            budget.clone(),
            directory,
            Some(spill.clone()),
        )
        .unwrap();
        writer
            .push(
                Batch::new(&model, vec![row("large", &"x".repeat(2 << 20))], &budget)
                    .unwrap()
                    .into_checked()
                    .unwrap(),
            )
            .unwrap();
        let source = writer.finish().unwrap();
        assert!(matches!(
            *source.segments[0].backing.read().unwrap(),
            SegmentBacking::Resident(_)
        ));
        let pressure = base
            .reserve(
                "private automatic spill control pressure",
                base.limit() - base.reserved() - 1024,
            )
            .unwrap();
        let new_provider = budget.reserve("new provider transfer", 128 * 1024).unwrap();
        assert!(
            matches!(
                *source.segments[0].backing.read().unwrap(),
                SegmentBacking::Ipc(_)
            ),
            "actual shared pool allocation triggers spill without a direct spill call"
        );
        drop((new_provider, pressure));
        assert_eq!(collected(&source)[0].payload.len(), 2 << 20);
        drop((source, budget, spill));
        assert_eq!(base.reserved(), 0);
    }
    #[test]
    fn spill_preserves_borrowers_and_last_array_owns_full_reservation() {
        let (model, budget, directory) = setup();
        let source = source(
            &model,
            &budget,
            &directory,
            vec![row("a", &"large".repeat(4000))],
        );
        let mut borrowed = source.read_batches().unwrap();
        let batch = borrowed.next().unwrap().unwrap();
        let column = batch.column(1).slice(0, 1);
        drop(batch);
        drop(borrowed);
        let held = budget.reserved();
        let pressure = budget
            .reserve(
                "private payload spill control pressure",
                budget.limit() - budget.reserved() - 256 * 1024,
            )
            .unwrap();
        source.spill_resident().unwrap();
        assert!(matches!(
            *source.segments[0].backing.read().unwrap(),
            SegmentBacking::Ipc(_)
        ));
        assert!(!collected(&source).is_empty());
        drop(pressure);
        drop(source);
        assert!(
            budget.reserved() > 0,
            "plain Arrow column slices must retain original reservation"
        );
        assert!(
            budget.reserved() < held,
            "index and relation metadata were released"
        );
        drop(column);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn index_pressure_selects_disk_and_releases_after_source_drops() {
        let (model, budget, directory) = setup();
        let a = source(
            &model,
            &budget,
            &directory,
            (0..300)
                .map(|i| row(&format!("key-{i}"), "value"))
                .collect(),
        );
        let pressure = budget
            .reserve(
                "private spill control pressure",
                budget.limit() - budget.reserved() - 10 * 1024,
            )
            .unwrap();
        // Force the compact cumulative index to its disk form with one bounded reader window.
        let mut builder = IndexBuilder::new(a.index.len, &budget, &directory).unwrap();
        for entry in a.index.cursor().unwrap() {
            builder.push(entry.unwrap()).unwrap();
        }
        let index = builder.finish().unwrap();
        assert!(matches!(
            *index.backing.read().unwrap(),
            IndexBacking::Disk { .. }
        ));
        drop(pressure);
        assert_eq!(index.cursor().unwrap().count(), 300);
        drop(index);
        drop(a);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn provider_and_output_outlive_registry_and_support_projection() {
        use futures::TryStreamExt;
        let (model, budget, directory) = setup();
        let source = source(
            &model,
            &budget,
            &directory,
            vec![row("a", "one"), row("b", "two")],
        );
        let context = datafusion::prelude::SessionContext::new();
        context
            .register_table("private_rows", source.provider())
            .unwrap();
        drop(source);
        let data = context
            .sql("SELECT payload FROM private_rows")
            .await
            .unwrap();
        let batches = data
            .execute_stream()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(batches.iter().map(RecordBatch::num_rows).sum::<usize>(), 2);
        drop(context);
        assert!(budget.reserved() > 0);
        drop(batches);
        assert_eq!(budget.reserved(), 0);
    }
}
