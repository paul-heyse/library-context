//! Explicit completed-input dispatch for pure kernels. There are no persisted grants or epochs.
use crate::workspace::CompletedInputs;
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::sources::CompletedInput, charged, resources::ResourceBudget, *,
};
use std::{any::TypeId, collections::BTreeSet};
/// Expected coverage needs artifact identity, input ownership and the model's bounded
/// classifier property. Ordinary selected algorithm reads still receive canonical rows.
pub(crate) async fn stream_artifact_admission(
    access: &CompletedInputs,
    input: &ValidationInput,
    session: &SessionContext,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<bool, ModelError> {
    if input.type_id() != TypeId::of::<source::SourceArtifact>() {
        return Ok(false);
    }
    let permit = access.read_at::<source::SourceArtifact>(input.prefix())?;
    let table = access.table_for(input)?;
    let selected = format!(
        "SELECT {} FROM {}",
        analysis::expected::CoverageAdmission::artifact_property_columns(),
        identifier(&table)
    );
    stream_query_at(&permit, input, session, &selected, |permit, batch| {
        admission.visit_artifact_properties(permit, batch)
    })
    .await?;
    Ok(true)
}
pub struct ConsumedInputs {
    declarations: Vec<ValidationInput>,
    dispatched: BTreeSet<usize>,
    _charge: charged::StateCharge,
}
impl ConsumedInputs {
    pub fn new(
        mut declarations: Vec<ValidationInput>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        declarations.sort_by_key(|input| (input.name(), input.prefix(), input.order().to_vec()));
        declarations.dedup_by(|a, b| {
            a.name() == b.name() && a.prefix() == b.prefix() && a.order() == b.order()
        });
        let mut charge = charged::StateCharge::new(budget, "completed-consumer-inputs");
        charge.grow(
            declarations
                .capacity()
                .saturating_mul(size_of::<ValidationInput>()),
        )?;
        Ok(Self {
            declarations,
            dispatched: Default::default(),
            _charge: charge,
        })
    }
    pub fn next<R: Record>(
        &mut self,
        inputs: &CompletedInputs,
    ) -> Result<Option<(ValidationInput, CompletedInput<R>)>, ModelError> {
        let Some((index, declaration)) =
            self.declarations
                .iter()
                .enumerate()
                .find(|(index, declaration)| {
                    declaration.type_id() == TypeId::of::<R>() && !self.dispatched.contains(index)
                })
        else {
            return Ok(None);
        };
        let source = inputs.read_at::<R>(declaration.prefix())?;
        self.dispatched.insert(index);
        Ok(Some((declaration.clone(), source)))
    }
    pub fn finish(self, producer: &str) -> Result<(), ModelError> {
        if let Some((_, input)) = self
            .declarations
            .iter()
            .enumerate()
            .find(|(index, _)| !self.dispatched.contains(index))
        {
            return Err(ModelError::Invalid(format!(
                "producer {producer} has no typed loader for {}",
                input.name()
            )));
        }
        Ok(())
    }
}
pub async fn stream_at<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    inputs: &CompletedInputs,
    session: &SessionContext,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    stream_where_at(input, declaration, inputs, session, None, consume).await
}
/// Read the exact immutable input through an owner-selected predicate before rich decoding.
/// The predicate is compiler SQL, never a provider string or a source of semantic authority.
pub async fn stream_where_at<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    inputs: &CompletedInputs,
    session: &SessionContext,
    predicate: Option<&str>,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    let table = inputs.table_at::<R>(declaration.prefix())?;
    let selected = format!(
        "SELECT * FROM {}{}",
        identifier(&table),
        predicate
            .map(|predicate| format!(" WHERE ({predicate})"))
            .unwrap_or_default()
    );
    stream_query_at(input, declaration, session, &selected, consume).await
}
/// Stream an owner-declared SELECT with the record's exact schema and ordering contract.
/// Joins and closure predicates are supplied by the owning semantic kernel.
pub async fn stream_query_at<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    session: &SessionContext,
    selected: &str,
    mut consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    if declaration.type_id() != TypeId::of::<R>() {
        return Err(ModelError::Invalid(
            "scoped stream record declaration mismatch".into(),
        ));
    }
    let order = declaration
        .order()
        .iter()
        .map(|column| identifier(column))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT * FROM ({selected}) AS scoped_input{}",
        if order.is_empty() {
            String::new()
        } else {
            format!(" ORDER BY {order}")
        }
    );
    let mut batches = crate::sql::query(session, &sql)
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
    while let Some(batch) = batches.try_next().await.map_err(ModelError::codec)? {
        consume(input, &batch)?;
        tokio::task::yield_now().await;
    }
    Ok(())
}
pub(crate) fn identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
#[cfg(test)]
pub(crate) fn assert_decoder_reachability(
    declarations: Vec<ValidationInput>,
    decoders: &BTreeSet<TypeId>,
) {
    let missing: Vec<_> = declarations
        .iter()
        .filter(|input| !decoders.contains(&input.type_id()))
        .map(ValidationInput::name)
        .collect();
    assert!(
        missing.is_empty(),
        "completed kernel input lacks typed decoder: {missing:?}"
    );
}
/// An immutable table binding chosen by the consumer, including its exact vocabulary epoch.
/// The alias is supplied by the admitted completed-input or detached validation session.
#[derive(Clone)]
pub struct ClosureTable {
    pub relation: Relation,
    pub alias: String,
}
/// A semantic owner explicitly selects forward nominal dependencies and reverse memberships.
/// This plan describes row selection, not admission; the selected owner predicates still run.
pub struct NominalClosure {
    tables: Vec<ClosureTable>,
    pairs: Vec<(usize, usize, String)>,
}
impl NominalClosure {
    pub fn new(tables: Vec<ClosureTable>) -> Result<Self, ModelError> {
        if tables.is_empty() || tables.iter().any(|table| table.alias.is_empty()) {
            return Err(ModelError::Invalid(
                "closure requires immutable table bindings".into(),
            ));
        }
        Ok(Self {
            tables,
            pairs: Vec::new(),
        })
    }
    /// Follow a declared nominal field. Target selection is explicit, so two vocabulary epochs
    /// of the same relation cannot silently collapse into a latest-alias dependency.
    pub fn follow(&mut self, source: usize, field: &str, target: usize) -> Result<(), ModelError> {
        let from = self.table(source)?;
        let to = self.table(target)?;
        let descriptor = from
            .relation
            .fields()
            .iter()
            .find(|candidate| candidate.name() == field)
            .ok_or_else(|| ModelError::Invalid("closure field absent".into()))?;
        if descriptor.target().map(|(kind, _)| kind) != Some(to.relation.type_id())
            || descriptor.list()
        {
            return Err(ModelError::Invalid(
                "closure field has another nominal target or list shape".into(),
            ));
        }
        let sql = format!(
            "SELECT id AS source_id, {} AS target_id FROM {} WHERE {} IS NOT NULL",
            identifier(field),
            identifier(&from.alias),
            identifier(field)
        );
        self.pairs.push((source, target, sql));
        Ok(())
    }
    /// Include rows owned through this nominal field, as well as their forward owner reference.
    /// Callers declare this only for actual membership, never for every incoming reference.
    pub fn own(
        &mut self,
        member: usize,
        owner_field: &str,
        owner: usize,
    ) -> Result<(), ModelError> {
        self.follow(member, owner_field, owner)?;
        let from = self.table(member)?;
        let sql = format!(
            "SELECT {} AS source_id, id AS target_id FROM {} WHERE {} IS NOT NULL",
            identifier(owner_field),
            identifier(&from.alias),
            identifier(owner_field)
        );
        self.pairs.push((owner, member, sql));
        Ok(())
    }
    /// Explicit semantic edges whose key includes a non-nominal domain, such as RunFamily.
    /// SELECT must return source_id and target_id (both nominal 16-byte IDs). The owner supplies
    /// the complete family/context predicates; this utility does not infer applicability.
    pub fn pairs(
        &mut self,
        source: usize,
        target: usize,
        select: String,
    ) -> Result<(), ModelError> {
        self.table(source)?;
        self.table(target)?;
        if select.trim().is_empty() {
            return Err(ModelError::Invalid("closure pair SELECT absent".into()));
        }
        self.pairs.push((source, target, select));
        Ok(())
    }
    fn table(&self, index: usize) -> Result<&ClosureTable, ModelError> {
        self.tables
            .get(index)
            .ok_or_else(|| ModelError::Invalid("closure table index absent".into()))
    }
    fn pair_sql(source: usize, target: usize, sql: &str) -> String {
        format!(
            "SELECT CAST({source} AS BIGINT) AS source_kind, source_id, CAST({target} AS BIGINT) AS target_kind, target_id FROM ({sql}) AS pair_rows WHERE source_id IS NOT NULL AND target_id IS NOT NULL"
        )
    }
    /// Project nominal edges once for an immutable dependency set into an external ordered IPC
    /// stream. All subsequent grains reuse these compact edges without rescanning rich inputs.
    pub async fn prepare(
        &self,
        session: &SessionContext,
        budget: &ResourceBudget,
    ) -> Result<PreparedEdges, ModelError> {
        use datafusion::{
            arrow::ipc::writer::FileWriter,
            execution::{options::ArrowReadOptions, session_state::SessionStateBuilder},
            prelude::col,
        };
        use std::{
            fs::File,
            io::{Read, Seek, SeekFrom},
        };
        let state = session.state();
        let catalogs = state.catalog_list().clone();
        let config = state
            .config()
            .clone()
            .with_create_default_catalog_and_schema(false)
            .with_target_partitions(state.config().target_partitions().max(2))
            .with_repartition_joins(true)
            .set_bool("datafusion.optimizer.prefer_hash_join", false);
        let session = SessionContext::new_with_state(
            SessionStateBuilder::new_from_existing(state)
                .with_config(config)
                .with_catalog_list(catalogs)
                .build(),
        );
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let pending = directory.path().join("pending-edges.arrow");
        let path = directory.path().join("nominal-edges.arrow");
        let schema = std::sync::Arc::new(arrow_schema::Schema::new(vec![
            arrow_schema::Field::new("source_kind", arrow_schema::DataType::Int64, false),
            arrow_schema::Field::new(
                "source_id",
                arrow_schema::DataType::FixedSizeBinary(16),
                false,
            ),
            arrow_schema::Field::new("target_kind", arrow_schema::DataType::Int64, false),
            arrow_schema::Field::new(
                "target_id",
                arrow_schema::DataType::FixedSizeBinary(16),
                false,
            ),
        ]));
        // Materialize one pair at a time. ORDER BY over a UNION permits the optimizer to
        // introduce one live sort/merge reservation per branch; an IPC scan is one sort input.
        // The staging envelope covers its writer/footer block descriptors, not per-edge state.
        let mut staging_charge =
            charged::StateCharge::new(budget, "semantic-edge-staging-metadata");
        staging_charge.grow(4096)?;
        // Copy fixed nominal values into one charged storage batch. Upstream sorting/filtering
        // can emit tiny fragments; neither IPC footer nor sparse index may grow per fragment.
        let coalescing_charge = budget.reserve(
            "semantic-edge-coalescing",
            resources::TRANSFER_ROWS
                .saturating_mul(48 * 3)
                .saturating_add(4096),
        )?;
        let mut buffer = EdgeBuffer::new(schema.clone());
        let mut staged =
            FileWriter::try_new(File::create(&pending).map_err(ModelError::codec)?, &schema)
                .map_err(ModelError::codec)?;
        for (source, target, sql) in &self.pairs {
            let frame = crate::sql::query(&session, &Self::pair_sql(*source, *target, sql))
                .await
                .map_err(ModelError::codec)?;
            let mut stream = frame.execute_stream().await.map_err(ModelError::codec)?;
            while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                let _transfer = budget.reserve(
                    "semantic-edge-transfer",
                    logical_batch_bytes(&batch)?
                        .saturating_mul(3)
                        .saturating_add(4096),
                )?;
                validate_edge_batch(&batch)?;
                buffer.push(&batch, |edge_batch| {
                    staging_charge.grow(128)?;
                    staged.write(&edge_batch).map_err(ModelError::codec)
                })?;
                tokio::task::yield_now().await;
            }
        }
        if let Some(batch) = buffer.finish()? {
            staging_charge.grow(128)?;
            staged.write(&batch).map_err(ModelError::codec)?;
        }
        staged.finish().map_err(ModelError::codec)?;
        drop(staged);
        let frame = session
            .read_arrow(
                pending.to_string_lossy().into_owned(),
                ArrowReadOptions::default().schema(schema.as_ref()),
            )
            .await
            .map_err(ModelError::codec)?
            .sort(vec![
                col("source_kind").sort(true, false),
                col("source_id").sort(true, false),
                col("target_kind").sort(true, false),
                col("target_id").sort(true, false),
            ])
            .map_err(ModelError::codec)?;
        let mut stream = frame.execute_stream().await.map_err(ModelError::codec)?;
        let mut batches = charged::ChargedVec::default();
        let mut index_charge = charged::StateCharge::new(budget, "semantic-edge-batch-index");
        let mut writer =
            FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?, &schema)
                .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let _transfer = budget.reserve(
                "semantic-edge-transfer",
                logical_batch_bytes(&batch)?
                    .saturating_mul(3)
                    .saturating_add(4096),
            )?;
            validate_edge_batch(&batch)?;
            buffer.push(&batch, |edge_batch| {
                write_edge_batch(&edge_batch, &mut writer, &mut batches, &mut index_charge)
            })?;
            tokio::task::yield_now().await;
        }
        if let Some(batch) = buffer.finish()? {
            write_edge_batch(&batch, &mut writer, &mut batches, &mut index_charge)?;
        }
        writer.finish().map_err(ModelError::codec)?;
        drop(buffer);
        drop(coalescing_charge);
        drop(writer);
        drop(stream);
        drop(staging_charge);
        std::fs::remove_file(&pending).map_err(ModelError::codec)?;
        // Reserve the reader's footer/block metadata before FileReader allocates it in a grain.
        let mut file = File::open(&path).map_err(ModelError::codec)?;
        file.seek(SeekFrom::End(-10)).map_err(ModelError::codec)?;
        let mut length = [0; 4];
        file.read_exact(&mut length).map_err(ModelError::codec)?;
        let footer = u32::from_le_bytes(length) as usize;
        let reader_allowance = footer
            .saturating_mul(2)
            .saturating_add(batches.len().saturating_mul(64))
            .saturating_add(4096);
        // Edge preparation is a bulk nominal operation. Grain selection instead joins the
        // already bounded keys to a rich source scan; sorting the source before filtering
        // would import unrelated payload into the grain's memory lifetime.
        let state = session.state();
        let catalogs = state.catalog_list().clone();
        let config = state
            .config()
            .clone()
            .with_create_default_catalog_and_schema(false)
            .with_target_partitions(1)
            .with_repartition_joins(false)
            .set_bool("datafusion.optimizer.prefer_hash_join", true);
        let session = SessionContext::new_with_state(
            SessionStateBuilder::new_from_existing(state)
                .with_config(config)
                .with_catalog_list(catalogs)
                .build(),
        );
        Ok(PreparedEdges(std::sync::Arc::new(EdgeSource {
            session,
            path,
            batches,
            reader_allowance,
            tables: self.tables.clone(),
            _index_charge: index_charge,
            _directory: directory,
        })))
    }
}
/// A fixed Arrow builder window copies only nominal values, releasing each input fragment.
/// Arrow's generic BatchCoalescer retains FixedSizeBinary slices until concatenation; that can
/// retain one IPC buffer/header per tiny fragment. Fixed-size builders avoid that physical shape.
struct EdgeBuffer {
    schema: arrow_schema::SchemaRef,
    source_kind: arrow_array::builder::Int64Builder,
    source_id: arrow_array::builder::FixedSizeBinaryBuilder,
    target_kind: arrow_array::builder::Int64Builder,
    target_id: arrow_array::builder::FixedSizeBinaryBuilder,
    rows: usize,
}
impl EdgeBuffer {
    fn new(schema: arrow_schema::SchemaRef) -> Self {
        let rows = resources::TRANSFER_ROWS;
        Self {
            schema,
            source_kind: arrow_array::builder::Int64Builder::with_capacity(rows),
            source_id: arrow_array::builder::FixedSizeBinaryBuilder::with_capacity(rows, 16),
            target_kind: arrow_array::builder::Int64Builder::with_capacity(rows),
            target_id: arrow_array::builder::FixedSizeBinaryBuilder::with_capacity(rows, 16),
            rows: 0,
        }
    }
    fn push(
        &mut self,
        batch: &arrow_array::RecordBatch,
        mut emit: impl FnMut(arrow_array::RecordBatch) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        for row in 0..batch.num_rows() {
            let source = edge_key(batch, "source_kind", "source_id", row)?;
            let target = edge_key(batch, "target_kind", "target_id", row)?;
            self.source_kind.append_value(source.0);
            self.source_id
                .append_value(source.1)
                .map_err(ModelError::codec)?;
            self.target_kind.append_value(target.0);
            self.target_id
                .append_value(target.1)
                .map_err(ModelError::codec)?;
            self.rows += 1;
            if self.rows == resources::TRANSFER_ROWS {
                emit(self.finish()?.expect("full nominal storage batch"))?;
            }
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<Option<arrow_array::RecordBatch>, ModelError> {
        if self.rows == 0 {
            return Ok(None);
        }
        let batch = arrow_array::RecordBatch::try_new(
            self.schema.clone(),
            vec![
                std::sync::Arc::new(self.source_kind.finish()),
                std::sync::Arc::new(self.source_id.finish()),
                std::sync::Arc::new(self.target_kind.finish()),
                std::sync::Arc::new(self.target_id.finish()),
            ],
        )
        .map_err(ModelError::codec)?;
        self.rows = 0;
        Ok(Some(batch))
    }
}
fn write_edge_batch(
    batch: &arrow_array::RecordBatch,
    writer: &mut datafusion::arrow::ipc::writer::FileWriter<std::fs::File>,
    batches: &mut charged::ChargedVec<EdgeBatch>,
    charge: &mut charged::StateCharge,
) -> Result<(), ModelError> {
    use std::io::Seek;
    let first = edge_key(batch, "source_kind", "source_id", 0)?;
    let last = edge_key(batch, "source_kind", "source_id", batch.num_rows() - 1)?;
    if first > last
        || batches
            .last()
            .is_some_and(|previous: &EdgeBatch| previous.last > first)
    {
        return Err(ModelError::Schema(
            "nominal edges are not ordered by complete source key",
        ));
    }
    charge.grow(128)?;
    let before = writer
        .get_mut()
        .stream_position()
        .map_err(ModelError::codec)?;
    writer.write(batch).map_err(ModelError::codec)?;
    let after = writer
        .get_mut()
        .stream_position()
        .map_err(ModelError::codec)?;
    let ipc_bytes = usize::try_from(after - before).map_err(ModelError::codec)?;
    batches.push(
        charge,
        EdgeBatch {
            first,
            last,
            rows: batch.num_rows(),
            ipc_bytes,
        },
    )
}
fn scope_alias(kind: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_SCOPE: AtomicU64 = AtomicU64::new(0);
    format!(
        "__semantic_{kind}_{}",
        NEXT_SCOPE.fetch_add(1, Ordering::Relaxed)
    )
}
type NominalKey = (i64, [u8; 16]);
/// Exactly one entry per IPC record batch. Duplicate source ranges may span arbitrarily many
/// adjacent batches; searching by BOTH first and last includes every such batch.
struct EdgeBatch {
    first: NominalKey,
    last: NominalKey,
    rows: usize,
    ipc_bytes: usize,
}
impl HeapSize for EdgeBatch {}
struct EdgeSource {
    session: SessionContext,
    path: std::path::PathBuf,
    batches: charged::ChargedVec<EdgeBatch>,
    reader_allowance: usize,
    tables: Vec<ClosureTable>,
    _index_charge: charged::StateCharge,
    _directory: tempfile::TempDir,
}
impl EdgeSource {
    fn matching_batches(&self, key: NominalKey) -> std::ops::Range<usize> {
        let start = self.batches.partition_point(|batch| batch.last < key);
        let end = self.batches.partition_point(|batch| batch.first <= key);
        start..end
    }
}
/// Four fixed, non-null nominal columns are the complete edge row shape. Buffer
/// capacities may include other arrays in the same IPC block; logical payload cannot.
fn validate_edge_batch(batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
    use arrow_schema::DataType;
    let fields = [
        ("source_kind", DataType::Int64),
        ("source_id", DataType::FixedSizeBinary(16)),
        ("target_kind", DataType::Int64),
        ("target_id", DataType::FixedSizeBinary(16)),
    ];
    if batch.num_columns() != fields.len()
        || fields.iter().enumerate().any(|(index, (name, kind))| {
            batch.schema().field(index).name().as_str() != *name
                || batch.column(index).data_type() != kind
                || batch.column(index).null_count() != 0
        })
        || logical_batch_bytes(batch)? != batch.num_rows().saturating_mul(48)
    {
        return Err(ModelError::Schema(
            "nominal edge batch requires four non-null fixed nominal columns",
        ));
    }
    Ok(())
}
fn edge_key(
    batch: &arrow_array::RecordBatch,
    kind: &str,
    id: &str,
    row: usize,
) -> Result<NominalKey, ModelError> {
    use arrow_array::{Array, FixedSizeBinaryArray, Int64Array};
    let kinds = batch
        .column_by_name(kind)
        .and_then(|column| column.as_any().downcast_ref::<Int64Array>())
        .ok_or(ModelError::Schema("nominal edge kind must be Int64"))?;
    let ids = batch
        .column_by_name(id)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .filter(|ids| ids.value_length() == 16)
        .ok_or(ModelError::Schema("nominal edge ID must have 16 bytes"))?;
    if row >= batch.num_rows() || kinds.is_null(row) || ids.is_null(row) {
        return Err(ModelError::Schema("nominal edge key absent"));
    }
    Ok((
        kinds.value(row),
        ids.value(row).try_into().map_err(ModelError::codec)?,
    ))
}
/// One FileReader and at most one current batch. Reader/block metadata and the decoded edge
/// batch are charged before allocation; the cache is replaced before reading another batch.
struct EdgeCursor<'a> {
    source: &'a EdgeSource,
    reader: datafusion::arrow::ipc::reader::FileReader<std::fs::File>,
    current: Option<(
        usize,
        arrow_array::RecordBatch,
        Box<dyn resources::Reservation>,
    )>,
    _reader_charge: Box<dyn resources::Reservation>,
}
impl<'a> EdgeCursor<'a> {
    fn new(source: &'a EdgeSource, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = budget.reserve("semantic-edge-reader", source.reader_allowance)?;
        let reader = datafusion::arrow::ipc::reader::FileReader::try_new(
            std::fs::File::open(&source.path).map_err(ModelError::codec)?,
            None,
        )
        .map_err(ModelError::codec)?;
        if reader.num_batches() != source.batches.len() {
            return Err(ModelError::Schema(
                "nominal edge sparse index does not match IPC batches",
            ));
        }
        Ok(Self {
            source,
            reader,
            current: None,
            _reader_charge: charge,
        })
    }
    fn batch(
        &mut self,
        index: usize,
        budget: &ResourceBudget,
    ) -> Result<&arrow_array::RecordBatch, ModelError> {
        if self
            .current
            .as_ref()
            .is_none_or(|(current, _, _)| *current != index)
        {
            self.current = None;
            let descriptor = &self.source.batches[index];
            // FileReader owns one metadata+body block buffer, shared by all four arrays.
            // get_array_memory_size counts that entire capacity once per column and is not
            // the logical shape. The recorded serialized block includes IPC metadata/padding.
            let retained = descriptor.ipc_bytes.saturating_add(4096);
            let mut charge = budget.reserve(
                "semantic-edge-current-batch",
                descriptor.ipc_bytes.saturating_mul(2).saturating_add(4096),
            )?;
            self.reader.set_index(index).map_err(ModelError::codec)?;
            let batch = self
                .reader
                .next()
                .ok_or(ModelError::Schema("nominal edge IPC batch missing"))?
                .map_err(ModelError::codec)?;
            validate_edge_batch(&batch)?;
            if batch.num_rows() != descriptor.rows {
                return Err(ModelError::Schema(
                    "nominal edge batch does not match its sparse descriptor",
                ));
            }
            charge.try_resize(retained)?;
            self.current = Some((index, batch, charge));
        }
        Ok(&self.current.as_ref().expect("current edge batch").1)
    }
    fn targets(
        &mut self,
        key: NominalKey,
        budget: &ResourceBudget,
        mut accept: impl FnMut(NominalKey) -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        for index in self.source.matching_batches(key) {
            let batch = self.batch(index, budget)?;
            // Find the first matching row within this selected batch without scanning its prefix.
            let mut left = 0usize;
            let mut right = batch.num_rows();
            while left < right {
                let middle = left + (right - left) / 2;
                if edge_key(batch, "source_kind", "source_id", middle)? < key {
                    left = middle + 1;
                } else {
                    right = middle;
                }
            }
            for row in left..batch.num_rows() {
                if edge_key(batch, "source_kind", "source_id", row)? != key {
                    break;
                }
                accept(edge_key(batch, "target_kind", "target_id", row)?)?;
            }
        }
        Ok(())
    }
}
/// Dependency-set preparation, reused by source/context/family grains. It selects rows only;
/// all ordinary owner predicates and the independent global reference pass still apply.
#[derive(Clone)]
pub struct PreparedEdges(std::sync::Arc<EdgeSource>);
impl PreparedEdges {
    /// Compute compact keys for one explicitly selected grain. An unsupported root remains
    /// selected, and visited complete nominal keys terminate cycles before another edge read.
    pub async fn grain(
        &self,
        root: usize,
        predicate: &str,
        budget: &ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        self.grain_roots(&[(root, predicate.to_owned())], budget)
            .await
    }
    /// Select an explicit owner-declared union of typed roots. The prepared nominal edges and
    /// traversal are shared with single-root grains; no source/metadata key becomes a new root.
    pub async fn grain_roots(
        &self,
        roots: &[(usize, String)],
        budget: &ResourceBudget,
    ) -> Result<PreparedClosure, ModelError> {
        use arrow_array::{FixedSizeBinaryArray, Int64Array, RecordBatch};
        use arrow_schema::{DataType, Field, Schema};
        use datafusion::datasource::MemTable;
        use std::sync::Arc;
        if roots.is_empty() {
            return Err(ModelError::Invalid("closure root domain absent".into()));
        }
        let mut seeds = Vec::new();
        for (root, predicate) in roots {
            let table = self
                .0
                .tables
                .get(*root)
                .ok_or_else(|| ModelError::Invalid("closure table index absent".into()))?;
            if predicate.trim().is_empty() {
                return Err(ModelError::Invalid("closure root predicate absent".into()));
            }
            seeds.push(format!(
                "SELECT CAST({root} AS BIGINT) AS kind,id FROM {} WHERE ({predicate})",
                identifier(&table.alias)
            ));
        }
        let sql = format!(
            "SELECT kind,id FROM ({}) actual_roots ORDER BY kind,id",
            seeds.join(" UNION ALL ")
        );
        let mut roots = crate::sql::query(&self.0.session, &sql)
            .await
            .map_err(ModelError::codec)?
            .execute_stream()
            .await
            .map_err(ModelError::codec)?;
        let mut visited = charged::ChargedSet::default();
        let mut pending = charged::ChargedVec::default();
        let mut traversal_charge =
            charged::StateCharge::new(budget, "semantic-grain-traversal-keys");
        while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
            let _transfer = budget.reserve(
                "semantic-grain-root-transfer",
                batch.get_array_memory_size(),
            )?;
            for row in 0..batch.num_rows() {
                let key = edge_key(&batch, "kind", "id", row)?;
                if visited.insert(&mut traversal_charge, key)? {
                    pending.push(&mut traversal_charge, key)?;
                }
            }
            tokio::task::yield_now().await;
        }
        drop(roots);
        let mut cursor = EdgeCursor::new(&self.0, budget)?;
        while let Some(key) = pending.take_last(&mut traversal_charge) {
            cursor.targets(key, budget, |target| {
                if target.0 < 0 || target.0 as usize >= self.0.tables.len() {
                    return Err(ModelError::Schema("nominal edge target namespace absent"));
                }
                if visited.insert(&mut traversal_charge, target)? {
                    pending.push(&mut traversal_charge, target)?;
                }
                Ok(())
            })?;
            tokio::task::yield_now().await;
        }
        drop(cursor);
        drop(pending);
        let schema = Arc::new(Schema::new(vec![
            Field::new("kind", DataType::Int64, false),
            Field::new("id", DataType::FixedSizeBinary(16), false),
        ]));
        let mut keys = Vec::new();
        let mut charge = charged::StateCharge::new(budget, "semantic-grain-keys");
        let mut selected = visited.iter();
        loop {
            let count = selected.len().min(resources::TRANSFER_ROWS);
            if count == 0 {
                break;
            }
            let mut transfer = budget.reserve(
                "semantic-grain-key-transfer",
                count.saturating_mul(64).saturating_add(4096),
            )?;
            let values = selected.by_ref().take(count).copied().collect::<Vec<_>>();
            let kinds = Int64Array::from_iter_values(values.iter().map(|key| key.0));
            let ids = FixedSizeBinaryArray::try_from_iter(values.iter().map(|key| key.1))
                .map_err(ModelError::codec)?;
            let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(kinds), Arc::new(ids)])
                .map_err(ModelError::codec)?;
            let additional = if keys.len() == keys.capacity() {
                keys.capacity().max(4)
            } else {
                0
            };
            charge.grow(
                batch
                    .get_array_memory_size()
                    .saturating_add(additional.saturating_mul(size_of::<RecordBatch>())),
            )?;
            keys.reserve_exact(additional);
            keys.push(batch);
            transfer.try_resize(0)?;
        }
        drop(visited);
        drop(traversal_charge);
        let alias = scope_alias("keys");
        let provider = MemTable::try_new(schema, vec![keys]).map_err(ModelError::codec)?;
        self.0
            .session
            .register_table(alias.clone(), Arc::new(provider))
            .map_err(ModelError::codec)?;
        Ok(PreparedClosure {
            edges: self.0.clone(),
            alias,
            _charge: charge,
        })
    }
}
/// A short-lived compact nominal key set for one owner grain. It keeps the external edge source
/// alive; dropping the scope removes its temporary key table without rewriting semantic inputs.
pub struct PreparedClosure {
    edges: std::sync::Arc<EdgeSource>,
    alias: String,
    _charge: charged::StateCharge,
}
impl PreparedClosure {
    pub fn session(&self) -> &SessionContext {
        &self.edges.session
    }
    pub fn select(&self, table: usize) -> Result<String, ModelError> {
        let table_binding = self
            .edges
            .tables
            .get(table)
            .ok_or_else(|| ModelError::Invalid("closure table index absent".into()))?;
        Ok(format!(
            "SELECT source.* FROM {} AS keys JOIN {} AS source ON keys.kind={table} AND source.id=keys.id",
            identifier(&self.alias),
            identifier(&table_binding.alias)
        ))
    }
}
impl Drop for PreparedClosure {
    fn drop(&mut self) {
        let _ = self.edges.session.deregister_table(self.alias.as_str());
    }
}

#[cfg(test)]
mod nominal_closure_controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use input::{Package, Release};
    use std::sync::Arc;

    fn register<R: Record>(session: &SessionContext, name: &str, rows: &[R]) {
        let batch = R::encode(rows).unwrap();
        session
            .register_table(
                name,
                Arc::new(MemTable::try_new(batch.schema(), vec![vec![batch]]).unwrap()),
            )
            .unwrap();
    }
    fn id_predicate<R: Record>(id: Id<R>) -> String {
        let bytes = id
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("id=X'{bytes}'")
    }
    async fn decode<R: Record>(scope: &PreparedClosure, table: usize) -> Vec<R> {
        let mut stream = crate::sql::query(scope.session(), &scope.select(table).unwrap())
            .await
            .unwrap()
            .execute_stream()
            .await
            .unwrap();
        let mut rows = Vec::new();
        while let Some(batch) = stream.try_next().await.unwrap() {
            rows.extend(R::decode(&batch).unwrap());
        }
        rows
    }
    fn fixture() -> (SessionContext, Vec<ClosureTable>, Vec<Release>, Package) {
        let session = SessionContext::new();
        let selected = Package {
            name: "selected".into(),
        };
        let unrelated = Package {
            name: "unrelated".into(),
        };
        let releases = vec![
            Release {
                package: selected.id(),
                version: "1".into(),
            },
            Release {
                package: selected.id(),
                version: "2".into(),
            },
            Release {
                package: unrelated.id(),
                version: "3".into(),
            },
        ];
        register(
            &session,
            "packages_at_attempt",
            &[selected.clone(), unrelated],
        );
        register(&session, "releases_at_attempt", &releases);
        let tables = vec![
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "releases_at_attempt".into(),
            },
            ClosureTable {
                relation: Relation::of::<Package>(),
                alias: "packages_at_attempt".into(),
            },
        ];
        (session, tables, releases, selected)
    }
    #[tokio::test]
    async fn forward_closure_keeps_root_and_excludes_shared_target_neighbors() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let index_usage = budget.reserved();
        assert!(index_usage > 0);
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
        drop(scope);
        assert_eq!(budget.reserved(), index_usage);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn explicit_reverse_membership_closes_cycle_without_unrelated_owners() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.own(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges
            .grain(1, &id_predicate(package.id()), &budget)
            .await
            .unwrap();
        let found = decode::<Release>(&scope, 0).await;
        assert_eq!(found.len(), 2);
        assert!(found.contains(&releases[0]) && found.contains(&releases[1]));
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
    }
    #[tokio::test]
    async fn root_without_support_is_selected_and_wrong_nominal_edge_refuses() {
        let (session, tables, releases, _) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        assert!(plan.follow(0, "package", 0).is_err());
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert!(decode::<Package>(&scope, 1).await.is_empty());
    }
    #[tokio::test]
    async fn sparse_batch_ranges_include_all_split_sources_and_exclude_unrelated_edges() {
        let session = SessionContext::new_with_config(
            datafusion::prelude::SessionConfig::new().with_batch_size(16),
        );
        let mut packages = (0..1000)
            .map(|index| Package {
                name: format!("owner-{index}"),
            })
            .collect::<Vec<_>>();
        packages.sort_by_key(Record::id);
        // With 80 rows per owner, this source range crosses a 4096-row storage boundary
        // after the 80,000 forward edges, with many unrelated full batches on both sides.
        let selected = packages[228].clone();
        let releases = packages
            .iter()
            .flat_map(|package| {
                (0..80).map(move |index| Release {
                    package: package.id(),
                    version: index.to_string(),
                })
            })
            .collect::<Vec<_>>();
        let expected = releases
            .iter()
            .filter(|row| row.package == selected.id())
            .cloned()
            .collect::<Vec<_>>();
        register(&session, "many_packages", &packages);
        register(&session, "many_releases", &releases);
        let mut plan = NominalClosure::new(vec![
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "many_releases".into(),
            },
            ClosureTable {
                relation: Relation::of::<Package>(),
                alias: "many_packages".into(),
            },
        ])
        .unwrap();
        plan.own(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let index_usage = budget.reserved();
        assert!(
            edges.0.batches.len() > 32,
            "fixture requires many unrelated IPC batches"
        );
        let source = (1, *selected.id().bytes());
        let range = edges.0.matching_batches(source);
        assert!(
            range.len() > 1,
            "one source must cross record-batch boundaries"
        );
        assert!(range.len() < edges.0.batches.len());
        assert!(range.start > 0 && range.end < edges.0.batches.len());
        // The direct cursor exercise proves every duplicate source range contributes its keys.
        let mut cursor = EdgeCursor::new(&edges.0, &budget).unwrap();
        let mut targets = std::collections::BTreeSet::new();
        cursor
            .targets(source, &budget, |key| {
                targets.insert(key);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            targets,
            expected
                .iter()
                .map(|row| (0, *row.id().bytes()))
                .collect::<std::collections::BTreeSet<_>>()
        );
        assert_eq!(cursor.current.as_ref().unwrap().0, range.end - 1);
        drop(cursor);
        assert_eq!(budget.reserved(), index_usage);
        let scope = edges
            .grain(1, &id_predicate(selected.id()), &budget)
            .await
            .unwrap();
        let found = decode::<Release>(&scope, 0).await;
        assert_eq!(found.len(), expected.len());
        assert!(expected.iter().all(|row| found.contains(row)));
        assert_eq!(decode::<Package>(&scope, 1).await, vec![selected]);
        drop(scope);
        assert_eq!(budget.reserved(), index_usage);
        // Empty root selection stays empty and does not fabricate a source key.
        let empty = edges.grain(1, "false", &budget).await.unwrap();
        assert!(decode::<Release>(&empty, 0).await.is_empty());
        assert!(decode::<Package>(&empty, 1).await.is_empty());
        drop(empty);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn identical_ids_in_distinct_virtual_namespaces_remain_distinct_keys() {
        let (session, _, releases, package) = fixture();
        register(&session, "releases_in_other_epoch", &releases);
        let mut plan = NominalClosure::new(vec![
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "releases_at_attempt".into(),
            },
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "releases_in_other_epoch".into(),
            },
            ClosureTable {
                relation: Relation::of::<Package>(),
                alias: "packages_at_attempt".into(),
            },
        ])
        .unwrap();
        plan.pairs(
            0,
            1,
            "SELECT id AS source_id,id AS target_id FROM releases_at_attempt".into(),
        )
        .unwrap();
        plan.follow(1, "package", 2).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert_eq!(
            decode::<Release>(&scope, 1).await,
            vec![releases[0].clone()]
        );
        assert_eq!(decode::<Package>(&scope, 2).await, vec![package]);
        drop(edges);
        assert!(
            budget.reserved() > 0,
            "closure retains the charged prepared index"
        );
        drop(scope);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn many_declared_pairs_prepare_without_a_left_deep_union_and_preserve_keys() {
        let (session, tables, releases, package) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        // Duplicate nominal pairs are legal declarations and must all reach external preparation.
        for _ in 0..257 {
            plan.follow(0, "package", 1).unwrap();
        }
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), &budget)
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
        drop(scope);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn ipc_batch_allowance_accounts_for_shared_block_metadata_and_alignment() {
        let (session, tables, _, _) = fixture();
        let mut plan = NominalClosure::new(tables).unwrap();
        plan.follow(0, "package", 1).unwrap();
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let edges = plan.prepare(&session, &budget).await.unwrap();
        let index_usage = budget.reserved();
        let descriptor = &edges.0.batches[0];
        let mut cursor = EdgeCursor::new(&edges.0, &budget).unwrap();
        let batch = cursor.batch(0, &budget).unwrap();
        validate_edge_batch(batch).unwrap();
        assert_eq!(logical_batch_bytes(batch).unwrap(), descriptor.rows * 48);
        assert!(
            descriptor.ipc_bytes > logical_batch_bytes(batch).unwrap(),
            "the IPC block includes metadata and alignment"
        );
        assert!(
            batch.get_array_memory_size() > descriptor.ipc_bytes,
            "columns report the same retained block capacity separately"
        );
        assert_eq!(
            budget.reserved(),
            index_usage + edges.0.reader_allowance + descriptor.ipc_bytes + 4096
        );
        let usage = budget.reserved();
        cursor.batch(0, &budget).unwrap();
        assert_eq!(budget.reserved(), usage);
        drop(cursor);
        assert_eq!(budget.reserved(), index_usage);
        drop(edges);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn many_pair_streams_share_one_external_sort_with_the_workspace_eight_mib_pool() {
        use crate::workspace::{Workspace, WorkspaceOptions};
        use lctx_model::domain::stages::Profile;
        let workspace = Workspace::new(
            Arc::new(model().unwrap()),
            WorkspaceOptions {
                memory_bytes: 8 << 20,
                partitions: 1,
                batch_rows: 16,
            },
        )
        .unwrap();
        let session = workspace
            .inputs("edge-pool-control", Profile::Catalog, std::iter::empty())
            .unwrap()
            .session(&workspace)
            .await
            .unwrap();
        let package = Package {
            name: "bounded-owner".into(),
        };
        let releases = (0..400)
            .map(|index| Release {
                package: package.id(),
                version: index.to_string(),
            })
            .collect::<Vec<_>>();
        register(&session, "bounded_packages", &[package.clone()]);
        register(&session, "bounded_releases", &releases);
        let mut plan = NominalClosure::new(vec![
            ClosureTable {
                relation: Relation::of::<Release>(),
                alias: "bounded_releases".into(),
            },
            ClosureTable {
                relation: Relation::of::<Package>(),
                alias: "bounded_packages".into(),
            },
        ])
        .unwrap();
        for _ in 0..257 {
            plan.follow(0, "package", 1).unwrap();
        }
        let edges = plan.prepare(&session, workspace.budget()).await.unwrap();
        let edge_rows = releases.len() * 257;
        assert_eq!(
            edges
                .0
                .batches
                .iter()
                .map(|batch| batch.rows)
                .sum::<usize>(),
            edge_rows,
            "all duplicate declared pair rows survive staging and ordering"
        );
        assert_eq!(
            edges.0.batches.len(),
            edge_rows.div_ceil(resources::TRANSFER_ROWS),
            "index has one entry per full storage batch, independent of upstream tiny fragments"
        );
        assert!(
            edges.0.batches[..edges.0.batches.len() - 1]
                .iter()
                .all(|batch| batch.rows == resources::TRANSFER_ROWS)
        );
        let index_usage = workspace.budget().reserved();
        let scope = edges
            .grain(0, &id_predicate(releases[0].id()), workspace.budget())
            .await
            .unwrap();
        assert_eq!(
            decode::<Release>(&scope, 0).await,
            vec![releases[0].clone()]
        );
        assert_eq!(decode::<Package>(&scope, 1).await, vec![package]);
        drop(scope);
        assert_eq!(workspace.budget().reserved(), index_usage);
        drop(edges);
        assert_eq!(workspace.budget().reserved(), 0);
    }
}
