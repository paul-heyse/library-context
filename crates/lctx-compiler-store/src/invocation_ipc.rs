//! Private schema-checked IPC preparation. Files preserve data, never source authority.
use crate::admission::Cancellation;
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use datafusion::{
    arrow::ipc::writer::FileWriter, execution::options::ArrowReadOptions, prelude::SessionContext,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    ModelError, charged::StateCharge, logical_batch_bytes, resources::ResourceBudget,
};
use std::{fs::File, sync::Arc};

pub struct InvocationIpc {
    session: SessionContext,
    budget: ResourceBudget,
    cancellation: Cancellation,
    directory: Arc<tempfile::TempDir>,
    charge: StateCharge,
    next_file: usize,
}
pub struct IpcCapture {
    writer: FileWriter<File>,
    path: std::path::PathBuf,
    schema: SchemaRef,
    budget: ResourceBudget,
    cancellation: Cancellation,
    charge: StateCharge,
    _directory: Arc<tempfile::TempDir>,
}
/// A completely written private input. Local registration can be retried while this exact
/// owner retains its file and metadata; no source acquisition is repeated by adoption.
pub struct SealedIpc {
    path: std::path::PathBuf,
    schema: SchemaRef,
    budget: ResourceBudget,
    _charge: StateCharge,
    reader_allowance: usize,
    directory: Arc<tempfile::TempDir>,
}
impl InvocationIpc {
    pub fn new(
        source: &SessionContext,
        budget: &ResourceBudget,
        cancellation: &Cancellation,
        parent: Option<&std::path::Path>,
    ) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "invocation IPC metadata");
        charge.grow(size_of::<Self>() + 16384 + 4096)?;
        let directory = Arc::new(
            match parent {
                Some(parent) => tempfile::tempdir_in(parent),
                None => tempfile::tempdir(),
            }
            .map_err(ModelError::codec)?,
        );
        Ok(Self {
            session: SessionContext::new_with_config_rt(
                source
                    .copied_config()
                    .with_create_default_catalog_and_schema(true),
                source.runtime_env(),
            ),
            budget: budget.clone(),
            cancellation: cancellation.clone(),
            directory,
            charge,
            next_file: 0,
        })
    }
    pub fn session(&self) -> &SessionContext {
        &self.session
    }
    pub fn begin(&mut self, schema: SchemaRef) -> Result<IpcCapture, ModelError> {
        self.cancellation.check()?;
        let mut charge = StateCharge::new(&self.budget, "invocation IPC footer");
        charge.grow(
            4096 + schema
                .fields()
                .iter()
                .map(|field| field.size())
                .sum::<usize>()
                .saturating_mul(3),
        )?;
        let path = self
            .directory
            .path()
            .join(format!("{}.arrow", self.next_file));
        self.next_file += 1;
        let writer = FileWriter::try_new(
            File::create(&path).map_err(ModelError::codec)?,
            schema.as_ref(),
        )
        .map_err(ModelError::codec)?;
        Ok(IpcCapture {
            writer,
            path,
            schema,
            budget: self.budget.clone(),
            cancellation: self.cancellation.clone(),
            charge,
            _directory: self.directory.clone(),
        })
    }
    /// No file/footer is needed after an exact selected source reaches EOF without rows.
    pub fn empty(&mut self, name: &str, schema: SchemaRef) -> Result<(), ModelError> {
        self.cancellation.check()?;
        self.charge.grow(512 + name.len())?;
        let table = datafusion::datasource::MemTable::try_new(schema, vec![vec![]])
            .map_err(ModelError::codec)?;
        self.session
            .register_table(name, Arc::new(table))
            .map_err(ModelError::codec)?;
        Ok(())
    }
    pub async fn adopt(&mut self, sealed: &SealedIpc, name: &str) -> Result<(), ModelError> {
        self.cancellation.check()?;
        if !Arc::ptr_eq(&sealed.directory, &self.directory) || !sealed.budget.shares_pool(&self.budget) {
            return Err(ModelError::Conflict("invocation IPC owner differs"));
        }
        let bytes = sealed.reader_allowance;
        self.charge.grow(bytes)?;
        let result = self.session.register_arrow(name, sealed.path.to_string_lossy(),
            ArrowReadOptions::default().schema(sealed.schema.as_ref())).await.map_err(crate::sql::model_error);
        if result.is_err() { self.charge.release(bytes); }
        result
    }
    pub async fn materialize(
        &mut self,
        frame: datafusion::dataframe::DataFrame,
        name: &str,
        expected: Option<&SchemaRef>,
    ) -> Result<(), ModelError> {
        self.cancellation.check()?;
        let mut stream = frame
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
        let schema = stream.schema();
        if expected.is_some_and(|expected| expected.as_ref() != schema.as_ref()) {
            return Err(ModelError::Schema(
                "invocation IPC complete Arrow schema differs",
            ));
        }
        let mut capture = self.begin(schema)?;
        while let Some(batch) = stream.try_next().await.map_err(crate::sql::model_error)? {
            capture.push(&batch)?;
            tokio::task::yield_now().await;
        }
        drop(stream);
        capture.finish(self, name).await
    }
}
impl IpcCapture {
    pub fn push(&mut self, batch: &RecordBatch) -> Result<(), ModelError> {
        self.cancellation.check()?;
        if batch.schema().as_ref() != self.schema.as_ref() {
            return Err(ModelError::Schema("invocation IPC batch schema differs"));
        }
        let _transfer = self.budget.reserve(
            "invocation IPC transfer",
            logical_batch_bytes(batch)?
                .saturating_mul(3)
                .saturating_add(4096),
        )?;
        self.charge.grow(256)?;
        self.writer.write(batch).map_err(ModelError::codec)
    }
    pub fn seal(mut self) -> Result<SealedIpc, ModelError> {
        self.cancellation.check()?;
        self.writer.finish().map_err(ModelError::codec)?;
        drop(self.writer);
        let reader_allowance = self.charge.reserved().saturating_mul(2).saturating_add(4096);
        // The writer/footer allocations ended at sealing. Keep only this file/schema owner;
        // registration separately admits the physical reader's metadata allowance.
        let retained = size_of::<SealedIpc>() + self.path.as_os_str().len()
            + self.schema.fields().iter().map(|field| field.size()).sum::<usize>() + 512;
        if self.charge.reserved() > retained { self.charge.release(self.charge.reserved() - retained); }
        Ok(SealedIpc { path: self.path, schema: self.schema, budget: self.budget,
            _charge: self.charge, reader_allowance, directory: self._directory })
    }
    pub async fn finish(self, owner: &mut InvocationIpc, name: &str) -> Result<(), ModelError> {
        let sealed = self.seal()?;
        owner.adopt(&sealed, name).await
    }
}

#[cfg(test)]
mod sealed_controls {
    use super::*;
    #[tokio::test]
    async fn sealed_input_survives_local_registration_refusal_without_recapture() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        {
            let session = SessionContext::new();
            let cancellation = Cancellation::default();
            let mut ipc = InvocationIpc::new(&session, &budget, &cancellation, None).unwrap();
            let schema = Arc::new(arrow_schema::Schema::new(vec![arrow_schema::Field::new(
                "id", arrow_schema::DataType::Int64, false)]));
            let input = RecordBatch::try_new(schema.clone(), vec![Arc::new(arrow_array::Int64Array::from(vec![7]))]).unwrap();
            let mut writer = ipc.begin(schema).unwrap();
            writer.push(&input).unwrap();
            let sealed = writer.seal().unwrap();
            let retained = budget.reserved();
            let pressure = budget.reserve("sealed-local-registration-control", budget.limit() - retained).unwrap();
            assert!(matches!(ipc.adopt(&sealed, "selected").await, Err(ModelError::Resource { .. })));
            drop(pressure);
            ipc.adopt(&sealed, "selected").await.unwrap();
            let rows = crate::sql::query(ipc.session(), "SELECT id FROM selected").await.unwrap().collect().await.unwrap();
            assert_eq!(rows.iter().map(RecordBatch::num_rows).sum::<usize>(), 1);
            assert_eq!(rows[0].column(0).as_any().downcast_ref::<arrow_array::Int64Array>().unwrap().value(0), 7);
            let mut foreign = InvocationIpc::new(&session, &budget, &cancellation, None).unwrap();
            assert!(foreign.adopt(&sealed, "foreign").await.is_err());
        }
        assert_eq!(budget.reserved(), 0);
    }
}
