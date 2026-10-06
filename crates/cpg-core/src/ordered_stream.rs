//! Canonical ordering sorts nominal coordinates, then gathers bounded original Arrow rows.
//! Original payloads never enter the external sorter; duplicate payloads are compared exactly.
use crate::workspace::Cancellation;
use arrow_array::{
    Array, FixedSizeBinaryArray, RecordBatch, UInt32Array, UInt64Array,
    builder::{FixedSizeBinaryBuilder, UInt64Builder},
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use datafusion::{
    arrow::{
        compute::{concat_batches, take},
        ipc::{reader::FileReader, writer::FileWriter},
    },
    execution::options::ArrowReadOptions,
    prelude::{SessionContext, col},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    ContentHash, ModelError, Relation, RelationContent, logical_batch_bytes,
    resources::{Reservation, ResourceBudget, TRANSFER_ROWS},
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
};

pub(crate) struct Source {
    pub path: PathBuf,
    pub blocks: Arc<[usize]>,
}
struct Loaded {
    file: usize,
    block: usize,
    batch: RecordBatch,
    _charge: Box<dyn Reservation>,
}
struct Reader {
    reader: FileReader<File>,
    _charge: Box<dyn Reservation>,
}
fn reader(path: &Path, budget: &ResourceBudget) -> Result<Reader, ModelError> {
    let mut file = File::open(path).map_err(ModelError::codec)?;
    file.seek(SeekFrom::End(-10)).map_err(ModelError::codec)?;
    let mut length = [0; 4];
    file.read_exact(&mut length).map_err(ModelError::codec)?;
    let charge = budget.reserve(
        "workspace-order-reader",
        (u32::from_le_bytes(length) as usize)
            .saturating_mul(2)
            .saturating_add(4096),
    )?;
    file.seek(SeekFrom::Start(0)).map_err(ModelError::codec)?;
    Ok(Reader {
        reader: FileReader::try_new(file, None).map_err(ModelError::codec)?,
        _charge: charge,
    })
}
fn load(
    readers: &mut [Reader],
    sources: &[Source],
    file: usize,
    block: usize,
    budget: &ResourceBudget,
) -> Result<Loaded, ModelError> {
    let bytes = *sources
        .get(file)
        .and_then(|source| source.blocks.get(block))
        .ok_or(ModelError::Schema("ordered row coordinate"))?;
    let charge = budget.reserve("workspace-order-source-block", bytes.saturating_add(4096))?;
    let source = readers
        .get_mut(file)
        .ok_or(ModelError::Schema("ordered row source"))?;
    source.reader.set_index(block).map_err(ModelError::codec)?;
    let batch = source
        .reader
        .next()
        .ok_or(ModelError::Schema("ordered row block"))?
        .map_err(ModelError::codec)?;
    Ok(Loaded {
        file,
        block,
        batch,
        _charge: charge,
    })
}
struct Coordinates {
    schema: SchemaRef,
    ids: FixedSizeBinaryBuilder,
    files: UInt64Builder,
    blocks: UInt64Builder,
    rows: UInt64Builder,
    bytes: UInt64Builder,
    len: usize,
}
impl Coordinates {
    fn new(schema: SchemaRef) -> Self {
        Self {
            schema,
            ids: FixedSizeBinaryBuilder::with_capacity(TRANSFER_ROWS, 16),
            files: UInt64Builder::with_capacity(TRANSFER_ROWS),
            blocks: UInt64Builder::with_capacity(TRANSFER_ROWS),
            rows: UInt64Builder::with_capacity(TRANSFER_ROWS),
            bytes: UInt64Builder::with_capacity(TRANSFER_ROWS),
            len: 0,
        }
    }
    fn push(
        &mut self,
        id: &[u8],
        file: usize,
        block: usize,
        row: usize,
        bytes: usize,
    ) -> Result<(), ModelError> {
        self.ids.append_value(id).map_err(ModelError::codec)?;
        self.files.append_value(file as u64);
        self.blocks.append_value(block as u64);
        self.rows.append_value(row as u64);
        self.bytes.append_value(bytes as u64);
        self.len += 1;
        Ok(())
    }
    fn write(&mut self, writer: &mut FileWriter<File>) -> Result<(), ModelError> {
        if self.len == 0 {
            return Ok(());
        }
        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(self.ids.finish()),
                Arc::new(self.files.finish()),
                Arc::new(self.blocks.finish()),
                Arc::new(self.rows.finish()),
                Arc::new(self.bytes.finish()),
            ],
        )
        .map_err(ModelError::codec)?;
        writer.write(&batch).map_err(ModelError::codec)?;
        self.len = 0;
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Coordinate {
    id: [u8; 16],
    file: usize,
    block: usize,
    row: usize,
    bytes: usize,
}
#[allow(
    clippy::too_many_arguments,
    reason = "Bounded coordinate gathering keeps reader state, row selection, scratch arrays and budget separate without another state owner."
)]
fn window(
    coordinates: &mut Vec<Coordinate>,
    previous: &mut Option<Coordinate>,
    readers: &mut [Reader],
    sources: &[Source],
    relation: &Relation,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
    writer: &mut FileWriter<File>,
    content: &mut RelationContent,
    blocks: &mut Vec<usize>,
) -> Result<(), ModelError> {
    if coordinates.is_empty() {
        return Ok(());
    }
    let bytes = coordinates
        .iter()
        .map(|coordinate| coordinate.bytes)
        .sum::<usize>();
    // Gather one admitted output window. Source blocks are visited in physical order once
    // within this window, then dropped; nominal output order is retained by the slot positions.
    let _charge = budget.reserve(
        "workspace-order-gather",
        bytes
            .saturating_mul(2)
            .saturating_add(
                coordinates
                    .len()
                    .saturating_mul(size_of::<Option<RecordBatch>>() + size_of::<usize>()),
            )
            .saturating_add(4096),
    )?;
    let mut positions = (0..coordinates.len()).collect::<Vec<_>>();
    positions.sort_unstable_by_key(|position| {
        (coordinates[*position].file, coordinates[*position].block)
    });
    let mut rows = vec![None; coordinates.len()];
    let mut cache: Option<Loaded> = None;
    for position in positions {
        cancellation.check()?;
        let coordinate = coordinates[position];
        if cache
            .as_ref()
            .is_none_or(|cached| cached.file != coordinate.file || cached.block != coordinate.block)
        {
            drop(cache.take());
            cache = Some(load(
                readers,
                sources,
                coordinate.file,
                coordinate.block,
                budget,
            )?);
        }
        let loaded = cache.as_ref().expect("selected block");
        if coordinate.row >= loaded.batch.num_rows() {
            return Err(ModelError::Schema("ordered row position"));
        }
        let singleton = loaded.batch.slice(coordinate.row, 1);
        let actual = singleton
            .column_by_name("id")
            .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
            .ok_or(ModelError::Schema(relation.name()))?;
        if actual.value(0) != coordinate.id {
            return Err(ModelError::Conflict("ordered row identity"));
        }
        // `take` copies this actual row instead of retaining the entire original IPC buffer.
        let index = UInt32Array::from(vec![0]);
        let columns = singleton
            .columns()
            .iter()
            .map(|column| take(column.as_ref(), &index, None).map_err(ModelError::codec))
            .collect::<Result<Vec<_>, _>>()?;
        rows[position] = Some(
            RecordBatch::try_new(relation.schema().clone(), columns).map_err(ModelError::codec)?,
        );
    }
    drop(cache);
    let mut keep = Vec::new();
    for (position, coordinate) in coordinates.iter().enumerate() {
        if let Some(before) = previous.as_ref()
            && before.id == coordinate.id
        {
            let equal = if position > 0 {
                rows[position] == rows[position - 1]
            } else {
                let loaded = load(readers, sources, before.file, before.block, budget)?;
                rows[position].as_ref() == Some(&loaded.batch.slice(before.row, 1))
            };
            if !equal {
                return Err(ModelError::Conflict(relation.name()));
            }
        } else {
            keep.push(position);
        }
        *previous = Some(*coordinate);
    }
    let selected = keep
        .into_iter()
        .map(|position| rows[position].take().expect("gathered row"))
        .collect::<Vec<_>>();
    drop(rows);
    if !selected.is_empty() {
        let batch =
            concat_batches(relation.schema(), selected.iter()).map_err(ModelError::codec)?;
        drop(selected);
        relation.hash_rows(&batch, content)?;
        let before = writer
            .get_mut()
            .stream_position()
            .map_err(ModelError::codec)?;
        writer.write(&batch).map_err(ModelError::codec)?;
        let after = writer
            .get_mut()
            .stream_position()
            .map_err(ModelError::codec)?;
        blocks.push(usize::try_from(after - before).map_err(ModelError::codec)?);
    }
    coordinates.clear();
    Ok(())
}
#[allow(
    clippy::too_many_arguments,
    reason = "Relation, immutable source streams, session, memory budget, cancellation and output limits belong to separate owners."
)]
pub(crate) async fn order(
    relation: &Relation,
    sources: Vec<Source>,
    session: &SessionContext,
    budget: &ResourceBudget,
    cancellation: &Cancellation,
    index_path: &Path,
    output_path: &Path,
    batch_rows: usize,
) -> Result<(u64, ContentHash, Arc<[usize]>), ModelError> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::FixedSizeBinary(16), false),
        Field::new("file", DataType::UInt64, false),
        Field::new("block", DataType::UInt64, false),
        Field::new("row", DataType::UInt64, false),
        Field::new("bytes", DataType::UInt64, false),
    ]));
    let _coordinate_charge = budget.reserve(
        "workspace-order-coordinate-window",
        TRANSFER_ROWS * 48 * 2 + 4096,
    )?;
    let mut coordinates = Coordinates::new(schema.clone());
    let mut index = FileWriter::try_new(
        File::create(index_path).map_err(ModelError::codec)?,
        &schema,
    )
    .map_err(ModelError::codec)?;
    let mut readers = sources
        .iter()
        .map(|source| reader(&source.path, budget))
        .collect::<Result<Vec<_>, _>>()?;
    for (file, source) in sources.iter().enumerate() {
        for block in 0..source.blocks.len() {
            cancellation.check()?;
            let loaded = load(&mut readers, &sources, file, block, budget)?;
            let ids = loaded
                .batch
                .column_by_name("id")
                .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
                .ok_or(ModelError::Schema(relation.name()))?;
            for row in 0..ids.len() {
                coordinates.push(
                    ids.value(row),
                    file,
                    block,
                    row,
                    logical_batch_bytes(&loaded.batch.slice(row, 1))?
                        .saturating_add(loaded.batch.num_columns().saturating_mul(256)),
                )?;
                if coordinates.len == TRANSFER_ROWS {
                    coordinates.write(&mut index)?;
                }
            }
        }
    }
    coordinates.write(&mut index)?;
    index.finish().map_err(ModelError::codec)?;
    drop((index, coordinates, _coordinate_charge));
    let frame = session
        .read_arrow(
            index_path.to_string_lossy().into_owned(),
            ArrowReadOptions::default().schema(schema.as_ref()),
        )
        .await
        .map_err(ModelError::codec)?
        .sort(vec![
            col("id").sort(true, false),
            col("file").sort(true, false),
            col("block").sort(true, false),
            col("row").sort(true, false),
        ])
        .map_err(ModelError::codec)?;
    let mut stream = frame.execute_stream().await.map_err(ModelError::codec)?;
    let mut writer = FileWriter::try_new(
        File::create(output_path).map_err(ModelError::codec)?,
        relation.schema().as_ref(),
    )
    .map_err(ModelError::codec)?;
    let mut content = relation.content();
    let mut blocks = Vec::new();
    let mut previous = None;
    let limit = batch_rows.clamp(1, TRANSFER_ROWS);
    let target = (budget.limit() / 8).max(1);
    let _window_charge = budget.reserve(
        "workspace-order-coordinate-read",
        limit.saturating_mul(size_of::<Coordinate>()),
    )?;
    let mut selected = Vec::with_capacity(limit);
    let mut selected_bytes = 0usize;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or(ModelError::Schema("ordered identities"))?;
        let number = |column: usize| {
            batch
                .column(column)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or(ModelError::Schema("ordered coordinates"))
        };
        let files = number(1)?;
        let block_ids = number(2)?;
        let rows = number(3)?;
        let bytes = number(4)?;
        for position in 0..batch.num_rows() {
            let coordinate = Coordinate {
                id: ids.value(position).try_into().map_err(ModelError::codec)?,
                file: usize::try_from(files.value(position)).map_err(ModelError::codec)?,
                block: usize::try_from(block_ids.value(position)).map_err(ModelError::codec)?,
                row: usize::try_from(rows.value(position)).map_err(ModelError::codec)?,
                bytes: usize::try_from(bytes.value(position)).map_err(ModelError::codec)?,
            };
            if !selected.is_empty()
                && (selected.len() == limit
                    || selected_bytes.saturating_add(coordinate.bytes) > target)
            {
                window(
                    &mut selected,
                    &mut previous,
                    &mut readers,
                    &sources,
                    relation,
                    budget,
                    cancellation,
                    &mut writer,
                    &mut content,
                    &mut blocks,
                )?;
                selected_bytes = 0;
            }
            selected.push(coordinate);
            selected_bytes = selected_bytes.saturating_add(coordinate.bytes);
        }
        tokio::task::yield_now().await;
    }
    drop(stream);
    window(
        &mut selected,
        &mut previous,
        &mut readers,
        &sources,
        relation,
        budget,
        cancellation,
        &mut writer,
        &mut content,
        &mut blocks,
    )?;
    drop(readers);
    writer.finish().map_err(ModelError::codec)?;
    writer
        .into_inner()
        .map_err(ModelError::codec)?
        .sync_all()
        .map_err(ModelError::codec)?;
    std::fs::remove_file(index_path).map_err(ModelError::codec)?;
    let (rows, content) = content.finish();
    Ok((rows, content, blocks.into()))
}
