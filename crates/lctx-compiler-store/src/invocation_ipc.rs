//! Private schema-checked IPC preparation. Files preserve data, never source authority.
use crate::admission::Cancellation;
use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use datafusion::{
    arrow::ipc::{
        reader::StreamReader,
        writer::{FileWriter, StreamWriter},
    },
    execution::options::ArrowReadOptions,
    prelude::SessionContext,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    ModelError, charged::StateCharge, logical_batch_bytes, resources::ResourceBudget,
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    sync::Arc,
};

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
/// Native frontier streams have no per-batch file footer. Their one active reader
/// admits only the largest bounded transfer window, independent of total fanout.
pub(crate) struct IpcStreamCapture {
    writer: StreamWriter<File>,
    path: std::path::PathBuf,
    schema: SchemaRef,
    budget: ResourceBudget,
    cancellation: Cancellation,
    charge: StateCharge,
    directory: Arc<tempfile::TempDir>,
    batch_allowance: usize,
}
pub(crate) struct SealedStream {
    path: std::path::PathBuf,
    schema: SchemaRef,
    budget: ResourceBudget,
    cancellation: Cancellation,
    _charge: StateCharge,
    _directory: Arc<tempfile::TempDir>,
    bytes: u64,
    batch_allowance: usize,
}
pub(crate) struct IpcStreamRead {
    reader: StreamReader<File>,
    owner: Arc<SealedStream>,
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
}
impl IpcStreamCapture {
    pub(crate) fn push(&mut self, batch: &RecordBatch) -> Result<(), ModelError> {
        self.cancellation.check()?;
        if batch.schema() != self.schema
            || batch.num_rows() > lctx_model::domain::resources::TRANSFER_ROWS
        {
            return Err(ModelError::Schema("native IPC stream transfer shape"));
        }
        let bytes = logical_batch_bytes(batch)?
            .saturating_mul(3)
            .saturating_add(4096);
        let _transfer = self.budget.reserve("native IPC stream transfer", bytes)?;
        self.batch_allowance = self.batch_allowance.max(bytes);
        self.writer.write(batch).map_err(ModelError::codec)
    }
    pub(crate) fn seal(mut self) -> Result<Arc<SealedStream>, ModelError> {
        self.cancellation.check()?;
        self.writer.finish().map_err(ModelError::codec)?;
        drop(self.writer);
        let bytes = std::fs::metadata(&self.path)
            .map_err(ModelError::codec)?
            .len();
        Ok(Arc::new(SealedStream {
            path: self.path,
            schema: self.schema,
            budget: self.budget,
            cancellation: self.cancellation,
            _charge: self.charge,
            _directory: self.directory,
            bytes,
            batch_allowance: self.batch_allowance,
        }))
    }
}
impl SealedStream {
    pub(crate) fn open(self: &Arc<Self>) -> Result<IpcStreamRead, ModelError> {
        self.cancellation.check()?;
        let charge = self.budget.reserve(
            "native IPC stream reader",
            self.batch_allowance
                + 4096
                + self.schema.fields().iter().map(|f| f.size()).sum::<usize>() * 3,
        )?;
        let mut file = File::open(&self.path).map_err(ModelError::codec)?;
        // StreamReader also accepts physical EOF. Our private sealed carrier instead
        // requires its exact written length and explicit Arrow continuation/EOS marker.
        if file.metadata().map_err(ModelError::codec)?.len() != self.bytes || self.bytes < 8 {
            return Err(ModelError::Schema("native IPC stream sealed length"));
        }
        file.seek(SeekFrom::End(-8)).map_err(ModelError::codec)?;
        let mut eos = [0; 8];
        file.read_exact(&mut eos).map_err(ModelError::codec)?;
        if eos != [255, 255, 255, 255, 0, 0, 0, 0] {
            return Err(ModelError::Schema("native IPC stream sealed EOS"));
        }
        file.seek(SeekFrom::Start(0)).map_err(ModelError::codec)?;
        let reader = StreamReader::try_new(file, None).map_err(ModelError::codec)?;
        if reader.schema() != self.schema {
            return Err(ModelError::Schema("native IPC stream schema"));
        }
        Ok(IpcStreamRead {
            reader,
            owner: self.clone(),
            _charge: charge,
        })
    }
}
impl IpcStreamRead {
    pub(crate) fn next(&mut self) -> Result<Option<RecordBatch>, ModelError> {
        self.owner.cancellation.check()?;
        self.reader.next().transpose().map_err(ModelError::codec)
    }
}
/// Read-only table registrations owned by one physical read stream. Providers retain
/// their exact sealed carriers; this session cannot create files or allocate writer IDs.
pub struct InvocationRead {
    session: SessionContext,
    _charge: StateCharge,
}
impl InvocationRead {
    pub fn session(&self) -> &SessionContext {
        &self.session
    }
}
impl SealedIpc {
    async fn adopt_to(
        &self,
        session: &SessionContext,
        directory: &Arc<tempfile::TempDir>,
        budget: &ResourceBudget,
        cancellation: &Cancellation,
        charge: &mut StateCharge,
        name: &str,
    ) -> Result<(), ModelError> {
        cancellation.check()?;
        if !Arc::ptr_eq(&self.directory, directory) || !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict("invocation IPC owner differs"));
        }
        let bytes = self.reader_allowance;
        charge.grow(bytes)?;
        let result = session
            .register_arrow(
                name,
                self.path.to_string_lossy(),
                ArrowReadOptions::default().schema(self.schema.as_ref()),
            )
            .await
            .map_err(crate::sql::model_error);
        if result.is_err() {
            charge.release(bytes);
        }
        result
    }
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
    pub(crate) fn begin_stream(
        &mut self,
        schema: SchemaRef,
    ) -> Result<IpcStreamCapture, ModelError> {
        self.cancellation.check()?;
        if schema.fields().iter().any(|field| {
            !matches!(
                field.data_type(),
                arrow_schema::DataType::FixedSizeBinary(16) | arrow_schema::DataType::Int64
            )
        }) {
            return Err(ModelError::Schema("native IPC stream nominal schema"));
        }
        let mut charge = StateCharge::new(&self.budget, "native IPC stream metadata");
        charge.grow(4096 + schema.fields().iter().map(|f| f.size()).sum::<usize>() * 3)?;
        let path = self
            .directory
            .path()
            .join(format!("{}.stream.arrow", self.next_file));
        self.next_file += 1;
        let writer =
            StreamWriter::try_new(File::create(&path).map_err(ModelError::codec)?, &schema)
                .map_err(ModelError::codec)?;
        Ok(IpcStreamCapture {
            writer,
            path,
            schema,
            budget: self.budget.clone(),
            cancellation: self.cancellation.clone(),
            charge,
            directory: self.directory.clone(),
            batch_allowance: 4096,
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
    pub fn reader(&self) -> Result<InvocationRead, ModelError> {
        self.cancellation.check()?;
        let mut charge = StateCharge::new(&self.budget, "invocation IPC read session");
        charge.grow(size_of::<InvocationRead>() + 16384 + 4096)?;
        Ok(InvocationRead {
            session: SessionContext::new_with_config_rt(
                self.session
                    .copied_config()
                    .with_create_default_catalog_and_schema(true),
                self.session.runtime_env(),
            ),
            _charge: charge,
        })
    }
    pub async fn adopt(&mut self, sealed: &SealedIpc, name: &str) -> Result<(), ModelError> {
        sealed
            .adopt_to(
                &self.session,
                &self.directory,
                &self.budget,
                &self.cancellation,
                &mut self.charge,
                name,
            )
            .await
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
        let reader_allowance = self
            .charge
            .reserved()
            .saturating_mul(2)
            .saturating_add(4096);
        // The writer/footer allocations ended at sealing. Keep only this file/schema owner;
        // registration separately admits the physical reader's metadata allowance.
        let retained = size_of::<SealedIpc>()
            + self.path.as_os_str().len()
            + self
                .schema
                .fields()
                .iter()
                .map(|field| field.size())
                .sum::<usize>()
            + 512;
        if self.charge.reserved() > retained {
            self.charge.release(self.charge.reserved() - retained);
        }
        Ok(SealedIpc {
            path: self.path,
            schema: self.schema,
            budget: self.budget,
            _charge: self.charge,
            reader_allowance,
            directory: self._directory,
        })
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
                "id",
                arrow_schema::DataType::Int64,
                false,
            )]));
            let input = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(arrow_array::Int64Array::from(vec![7]))],
            )
            .unwrap();
            let mut writer = ipc.begin(schema).unwrap();
            writer.push(&input).unwrap();
            let sealed = writer.seal().unwrap();
            let retained = budget.reserved();
            let pressure = budget
                .reserve(
                    "sealed-local-registration-control",
                    budget.limit() - retained,
                )
                .unwrap();
            assert!(matches!(
                ipc.adopt(&sealed, "selected").await,
                Err(ModelError::Resource { .. })
            ));
            drop(pressure);
            ipc.adopt(&sealed, "selected").await.unwrap();
            let rows = crate::sql::query(ipc.session(), "SELECT id FROM selected")
                .await
                .unwrap()
                .collect()
                .await
                .unwrap();
            assert_eq!(rows.iter().map(RecordBatch::num_rows).sum::<usize>(), 1);
            assert_eq!(
                rows[0]
                    .column(0)
                    .as_any()
                    .downcast_ref::<arrow_array::Int64Array>()
                    .unwrap()
                    .value(0),
                7
            );
            let mut foreign = InvocationIpc::new(&session, &budget, &cancellation, None).unwrap();
            assert!(foreign.adopt(&sealed, "foreign").await.is_err());
        }
        assert_eq!(budget.reserved(), 0);
    }
}
