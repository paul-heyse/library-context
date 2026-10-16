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
    pub async fn finish(mut self, owner: &mut InvocationIpc, name: &str) -> Result<(), ModelError> {
        self.cancellation.check()?;
        if !Arc::ptr_eq(&self._directory, &owner.directory)
            || !self.budget.shares_pool(&owner.budget)
        {
            return Err(ModelError::Conflict("invocation IPC owner differs"));
        }
        self.writer.finish().map_err(ModelError::codec)?;
        drop(self.writer);
        owner.charge.grow(
            self.charge
                .reserved()
                .saturating_mul(2)
                .saturating_add(4096),
        )?;
        owner
            .session
            .register_arrow(
                name,
                self.path.to_string_lossy(),
                ArrowReadOptions::default().schema(self.schema.as_ref()),
            )
            .await
            .map_err(crate::sql::model_error)?;
        Ok(())
    }
}
