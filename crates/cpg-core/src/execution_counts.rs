//! Canonical counts over external primitive output IDs; rich result rows never stay here.
use arrow_array::{FixedSizeBinaryArray, Int64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use datafusion::{
    arrow::ipc::writer::FileWriter, execution::options::ArrowReadOptions, prelude::SessionContext,
};
use futures::TryStreamExt;
use lctx_model::domain::*;
use std::{fs::File, sync::Arc};
const ROWS: usize = 1024;
pub(super) struct ExecutionCounts {
    directory: tempfile::TempDir,
    writer: FileWriter<File>,
    rows: Vec<(i64, [u8; 16])>,
    charge: charged::StateCharge,
    schema: Arc<Schema>,
}
impl ExecutionCounts {
    pub(super) fn new(budget: &resources::ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(budget, "execution-canonical-count-index");
        charge.grow(ROWS * size_of::<(i64, [u8; 16])>() + 4096)?;
        let schema = Arc::new(Schema::new(vec![
            Field::new("kind", DataType::Int64, false),
            Field::new("id", DataType::FixedSizeBinary(16), false),
        ]));
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let writer = FileWriter::try_new(
            File::create(directory.path().join("output-ids.arrow")).map_err(ModelError::codec)?,
            schema.as_ref(),
        )
        .map_err(ModelError::codec)?;
        Ok(Self {
            directory,
            writer,
            rows: Vec::with_capacity(ROWS),
            charge,
            schema,
        })
    }
    pub(super) fn push<R: Record>(&mut self, kind: i64, row: &R) -> Result<(), ModelError> {
        self.rows.push((kind, *row.id().bytes()));
        if self.rows.len() == ROWS {
            self.flush()?;
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if self.rows.is_empty() {
            return Ok(());
        }
        let _transfer = self.charge.budget().expect("bound count index").reserve(
            "execution-count-index-transfer",
            self.rows.len() * 48 + 4096,
        )?;
        let kinds = Int64Array::from(self.rows.iter().map(|row| row.0).collect::<Vec<_>>());
        let ids = FixedSizeBinaryArray::try_from_iter(self.rows.iter().map(|row| row.1.as_slice()))
            .map_err(ModelError::codec)?;
        let batch = RecordBatch::try_new(self.schema.clone(), vec![Arc::new(kinds), Arc::new(ids)])
            .map_err(ModelError::codec)?;
        self.charge.grow(128)?;
        self.writer.write(&batch).map_err(ModelError::codec)?;
        self.rows.clear();
        Ok(())
    }
    pub(super) async fn finish(mut self, session: &SessionContext) -> Result<[i64; 4], ModelError> {
        self.flush()?;
        self.writer.finish().map_err(ModelError::codec)?;
        drop(self.writer);
        // A view local to this derived session avoids exposing incomplete producer outputs.
        let state = datafusion::execution::session_state::SessionStateBuilder::new_from_existing(
            session.state(),
        )
        .with_config(
            session
                .state()
                .config()
                .clone()
                .with_create_default_catalog_and_schema(true),
        )
        .with_catalog_list(Arc::new(
            datafusion::catalog::MemoryCatalogProviderList::new(),
        ))
        .build();
        let context = SessionContext::new_with_state(state);
        let frame = context
            .read_arrow(
                self.directory
                    .path()
                    .join("output-ids.arrow")
                    .to_string_lossy()
                    .as_ref(),
                ArrowReadOptions::default().schema(self.schema.as_ref()),
            )
            .await
            .map_err(ModelError::codec)?;
        context
            .register_table("execution_output_ids", frame.into_view())
            .map_err(ModelError::codec)?;
        let mut stream = crate::sql::query(
            &context,
            "SELECT kind,id FROM execution_output_ids ORDER BY kind,id",
        )
        .await
        .map_err(ModelError::codec)?
        .execute_stream()
        .await
        .map_err(ModelError::codec)?;
        let mut counts = [0i64; 4];
        let mut previous = None;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            let _transfer = self.charge.budget().expect("bound count index").reserve(
                "execution-count-sorted-transfer",
                lctx_model::domain::logical_batch_bytes(&batch)?,
            )?;
            let kinds = batch
                .column_by_name("kind")
                .and_then(|column| column.as_any().downcast_ref::<Int64Array>())
                .ok_or(ModelError::Schema("execution count kind"))?;
            for row in 0..batch.num_rows() {
                let kind = usize::try_from(kinds.value(row)).map_err(ModelError::codec)?;
                let key = (
                    kind,
                    crate::scoped_admission::column(&batch, "id", row)?
                        .ok_or(ModelError::Schema("execution count ID"))?,
                );
                if previous != Some(key) {
                    let count = counts
                        .get_mut(kind)
                        .ok_or(ModelError::Schema("execution count output family"))?;
                    *count = count
                        .checked_add(1)
                        .ok_or(ModelError::Conflict("execution canonical count overflow"))?;
                    previous = Some(key);
                }
            }
            tokio::task::yield_now().await;
        }
        Ok(counts)
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    #[tokio::test]
    async fn repeated_dependency_ids_count_once_across_external_batches_and_release() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut counts = ExecutionCounts::new(&budget).unwrap();
        for n in 0u64..2500 {
            let mut bytes = [0u8; 16];
            bytes[..8].copy_from_slice(&n.to_be_bytes());
            let artifact = source::SourceArtifact::from_bytes(
                super::super::execution_scope::nominal(&bytes).unwrap(),
                format!("{n}.py"),
                b"pass",
            )
            .unwrap();
            counts.push(0, &artifact).unwrap();
            counts.push(0, &artifact).unwrap();
            counts.push(1, &artifact).unwrap();
        }
        let result = counts.finish(&SessionContext::new()).await.unwrap();
        assert_eq!(result, [2500, 2500, 0, 0]);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn empty_external_index_counts_zero_and_releases() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let counts = ExecutionCounts::new(&budget).unwrap();
        assert_eq!(counts.finish(&SessionContext::new()).await.unwrap(), [0; 4]);
        assert_eq!(budget.reserved(), 0);
    }
}
