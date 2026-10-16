//! Exact private source-key -> capture-window routing, using the existing ordered nominal
//! IPC buffer, sparse batch descriptors and bounded reader. Binary-level merging bounds the
//! number of immutable runs without retaining an in-memory entry for every captured key.
use super::{EdgeBuffer, EdgeCursor, NominalKey, PreparedAdjacency, edge_key, write_edge_batch};
use crate::admission::Cancellation;
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::arrow::ipc::writer::FileWriter;
use lctx_model::domain::{ModelError, charged, resources::{self, ResourceBudget}};
use std::{fs::File, io::{Read, Seek, SeekFrom}, sync::Arc};

pub(crate) struct CoverageIndex { source: PreparedAdjacency }
struct Builder {
    directory: tempfile::TempDir,
    path: std::path::PathBuf,
    writer: FileWriter<File>,
    buffer: EdgeBuffer,
    batches: charged::ChargedVec<super::EdgeBatch>,
    charge: charged::StateCharge,
    _buffer: Box<dyn resources::Reservation>,
}
fn schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("source_kind", DataType::Int64, false),
        Field::new("source_id", DataType::FixedSizeBinary(16), false),
        Field::new("target_kind", DataType::Int64, false),
        Field::new("target_id", DataType::FixedSizeBinary(16), false),
    ]))
}
impl Builder {
    fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        let buffer_charge = budget.reserve("native-coverage-index-buffer", resources::TRANSFER_ROWS * 48 * 3 + 4096)?;
        let mut charge = charged::StateCharge::new(budget, "native-coverage-index");
        charge.grow(4096)?;
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let path = directory.path().join("coverage.arrow");
        let schema = schema();
        let writer = FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?, &schema).map_err(ModelError::codec)?;
        Ok(Self { directory, path, writer, buffer: EdgeBuffer::new(schema), batches: Default::default(),
            charge, _buffer: buffer_charge })
    }
    fn push(&mut self, source: NominalKey, target: NominalKey) -> Result<(), ModelError> {
        self.buffer.push_entry(source, target, |batch| write_edge_batch(&batch, &mut self.writer, &mut self.batches, &mut self.charge))
    }
    fn finish(mut self) -> Result<CoverageIndex, ModelError> {
        if let Some(batch) = self.buffer.finish()? { write_edge_batch(&batch, &mut self.writer, &mut self.batches, &mut self.charge)?; }
        self.writer.finish().map_err(ModelError::codec)?;
        drop(self.writer);
        drop(self.buffer);
        let mut file = File::open(&self.path).map_err(ModelError::codec)?;
        file.seek(SeekFrom::End(-10)).map_err(ModelError::codec)?;
        let mut footer = [0;4]; file.read_exact(&mut footer).map_err(ModelError::codec)?;
        let reader_allowance = (u32::from_le_bytes(footer) as usize).saturating_mul(2)
            .saturating_add(self.batches.len() * 64).saturating_add(4096);
        Ok(CoverageIndex { source: PreparedAdjacency { path: Some(self.path), batches: self.batches,
            reader_allowance, native_edges: Vec::new(), _index_charge: self.charge, _directory: Some(self.directory) } })
    }
}
struct Entries<'a> { source: &'a PreparedAdjacency, cursor: EdgeCursor<'a>, batch: usize, row: usize }
impl<'a> Entries<'a> {
    fn new(index: &'a CoverageIndex, budget: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self { source: &index.source, cursor: EdgeCursor::new(&index.source, budget)?, batch: 0, row: 0 })
    }
    fn next(&mut self, budget: &ResourceBudget) -> Result<Option<(NominalKey, NominalKey)>, ModelError> {
        if self.batch >= self.source.batches.len() { return Ok(None); }
        let batch = self.cursor.batch(self.batch, budget)?;
        let value = (edge_key(batch, "source_kind", "source_id", self.row)?, edge_key(batch, "target_kind", "target_id", self.row)?);
        self.row += 1;
        if self.row == batch.num_rows() { self.batch += 1; self.row = 0; }
        Ok(Some(value))
    }
}
impl CoverageIndex {
    pub(crate) fn new(keys: &[[u8;16]], window: usize, budget: &ResourceBudget) -> Result<Self, ModelError> {
        if keys.is_empty() || keys.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ModelError::Schema("native coverage index keys"));
        }
        let ordinal = i64::try_from(window).map_err(ModelError::codec)?;
        let mut builder = Builder::new(budget)?;
        for key in keys { builder.push((0,*key), (ordinal,[0;16]))?; }
        builder.finish()
    }
    /// Two immutable successful inputs remain owned until the merged replacement is complete.
    /// A local refusal cannot erase either run or require source acquisition to be repeated.
    pub(crate) async fn merge(&self, other: &Self, budget: &ResourceBudget, cancellation: &Cancellation) -> Result<Self, ModelError> {
        let mut out = Builder::new(budget)?;
        let mut a = Entries::new(self, budget)?;
        let mut b = Entries::new(other, budget)?;
        let mut left = a.next(budget)?;
        let mut right = b.next(budget)?;
        let mut count = 0usize;
        while left.is_some() || right.is_some() {
            cancellation.check()?;
            let value = match (left, right) {
                (Some(x), Some(y)) if x.0 == y.0 => return Err(ModelError::Conflict("native capture coverage was reissued")),
                (Some(x), Some(y)) if x.0 < y.0 => { left = a.next(budget)?; x }
                (Some(_), Some(y)) => { right = b.next(budget)?; y }
                (Some(x), None) => { left = a.next(budget)?; x }
                (None, Some(y)) => { right = b.next(budget)?; y }
                (None, None) => unreachable!(),
            };
            out.push(value.0, value.1)?;
            count += 1;
            if count.is_multiple_of(resources::TRANSFER_ROWS) { tokio::task::yield_now().await; }
        }
        out.finish()
    }
    pub(crate) fn lookup(&self, keys: &[[u8;16]], budget: &ResourceBudget, mut consume: impl FnMut([u8;16], usize) -> Result<(), ModelError>) -> Result<(), ModelError> {
        let mut cursor = EdgeCursor::new(&self.source, budget)?;
        for key in keys {
            cursor.targets((0,*key), budget, |target| {
                let ordinal = usize::try_from(target.0).map_err(ModelError::codec)?;
                if target.1 != [0;16] { return Err(ModelError::Schema("native coverage index value")); }
                consume(*key, ordinal)
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn merged_routes_keep_exact_ordinals_and_absence_under_bounded_readers() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        {
            let left = CoverageIndex::new(&[[1;16],[3;16],[5;16]], 7, &budget).unwrap();
            let right = CoverageIndex::new(&[[2;16],[4;16],[6;16]], 9, &budget).unwrap();
            let merged = left.merge(&right, &budget, &Cancellation::default()).await.unwrap();
            let mut actual = Vec::new();
            merged.lookup(&[[1;16],[2;16],[5;16],[6;16],[8;16]], &budget, |key, ordinal| {
                actual.push((key,ordinal)); Ok(())
            }).unwrap();
            assert_eq!(actual, [([1;16],7),([2;16],9),([5;16],7),([6;16],9)]);
            let duplicate = CoverageIndex::new(&[[1;16]], 11, &budget).unwrap();
            assert!(matches!(merged.merge(&duplicate, &budget, &Cancellation::default()).await,
                Err(ModelError::Conflict("native capture coverage was reissued"))));
        }
        assert_eq!(budget.reserved(), 0);
    }
}
