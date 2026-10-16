//! Receiving-invocation preparation. Local IPC is data, never validity or owner authority.
use super::{AdmissionRuntime, ValidationSession};
use arrow_schema::SchemaRef;
use datafusion::{
    arrow::ipc::writer::FileWriter, execution::options::ArrowReadOptions, prelude::SessionContext,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    ContentHash, ModelError, Relation, charged::StateCharge, logical_batch_bytes,
};
use std::{collections::BTreeMap, fs::File};

struct PreparedInput {
    schema: SchemaRef,
    table: String,
}
struct PreparedFamily {
    schema: SchemaRef,
    table: String,
}
/// Original provider/session ownership outlives all local files and queries. Every demanded
/// input is drained once, preserving its full typed rows, before any local comparison uses it.
/// The caller still performs actual current observations and failure-atomic premise publication.
pub(super) struct PreparedReferenceTargets<'a> {
    runtime: &'a AdmissionRuntime,
    original: &'a SessionContext,
    _owner: Option<&'a ValidationSession>,
    local: Option<SessionContext>,
    inputs: BTreeMap<ContentHash, PreparedInput>,
    families: BTreeMap<Vec<ContentHash>, PreparedFamily>,
    charge: StateCharge,
    // The private context drops its file providers before the directory is removed.
    directory: Option<tempfile::TempDir>,
    next_file: usize,
}
impl<'a> PreparedReferenceTargets<'a> {
    pub(super) fn new(
        runtime: &'a AdmissionRuntime,
        original: &'a SessionContext,
        owner: Option<&'a ValidationSession>,
    ) -> Self {
        Self {
            runtime,
            original,
            _owner: owner,
            local: None,
            inputs: BTreeMap::new(),
            families: BTreeMap::new(),
            charge: StateCharge::new(runtime.budget(), "receiving reference preparation metadata"),
            directory: None,
            next_file: 0,
        }
    }
    pub(super) fn session(&self) -> &SessionContext {
        self.local
            .as_ref()
            .expect("reference input has been prepared")
    }
    fn initialize(&mut self) -> Result<(), ModelError> {
        if self.local.is_none() {
            self.charge
                .grow(std::mem::size_of::<Self>().saturating_add(16384))?;
            self.local = Some(SessionContext::new_with_config_rt(
                self.runtime.context.copied_config(),
                self.runtime.context.runtime_env(),
            ));
        }
        Ok(())
    }
    pub(super) async fn input(
        &mut self,
        identity: ContentHash,
        relation: &Relation,
        table: &str,
    ) -> Result<String, ModelError> {
        self.runtime.cancellation.check()?;
        if let Some(prepared) = self.inputs.get(&identity) {
            if prepared.schema.as_ref() != relation.schema().as_ref() {
                return Err(ModelError::Schema(
                    "reference preparation input schema changed",
                ));
            }
            return Ok(prepared.table.clone());
        }
        self.initialize()?;
        self.charge
            .grow(1024usize.saturating_add(relation.schema().fields().len().saturating_mul(128)))?;
        let name = format!("_reference_input_{}", identity.hex());
        let sql = format!("SELECT * FROM {}", crate::consumed_rows::identifier(table));
        let frame = crate::sql::query(self.original, &sql)
            .await
            .map_err(crate::sql::model_error)?;
        self.materialize(frame, &name, Some(relation.schema()))
            .await?;
        self.inputs.insert(
            identity,
            PreparedInput {
                schema: relation.schema().clone(),
                table: name.clone(),
            },
        );
        Ok(name)
    }
    /// The complete ordered target set and its full schema determine this private family.
    /// Payload conflicts have already been checked before its canonical ID/tag projection.
    pub(super) async fn family(
        &mut self,
        targets: &[(&ContentHash, &(Relation, String))],
    ) -> Result<String, ModelError> {
        let (_, (relation, _)) = targets
            .first()
            .ok_or(ModelError::Schema("empty prepared target family"))?;
        let _key = self.runtime.budget().reserve(
            "reference target family lookup",
            targets
                .len()
                .saturating_mul(std::mem::size_of::<ContentHash>())
                .saturating_add(512),
        )?;
        let identities = targets
            .iter()
            .map(|(identity, _)| **identity)
            .collect::<Vec<_>>();
        if targets
            .iter()
            .any(|(_, (other, _))| other.schema().as_ref() != relation.schema().as_ref())
        {
            return Err(ModelError::Schema("reference target family schema differs"));
        }
        if let Some(prepared) = self.families.get(&identities) {
            if prepared.schema.as_ref() != relation.schema().as_ref() {
                return Err(ModelError::Schema("reference target family schema changed"));
            }
            return Ok(prepared.table.clone());
        }
        self.charge.grow(
            targets
                .len()
                .saturating_mul(
                    std::mem::size_of::<ContentHash>() + std::mem::size_of::<String>() + 128,
                )
                .saturating_add(512),
        )?;
        let mut tables = Vec::with_capacity(targets.len());
        for (identity, (relation, table)) in targets {
            tables.push(self.input(**identity, relation, table).await?);
        }
        let quote = crate::consumed_rows::identifier;
        let columns = match relation.sum() {
            Some(sum) => format!("{}, {}", quote("id"), quote(sum.tag)),
            None => quote("id"),
        };
        self.charge.grow(
            tables
                .iter()
                .map(|table| table.len().saturating_add(columns.len()).saturating_add(64))
                .sum(),
        )?;
        let union = tables
            .iter()
            .map(|table| format!("SELECT {columns} FROM {}", quote(table)))
            .collect::<Vec<_>>()
            .join(" UNION ALL ");
        let sql = format!("SELECT DISTINCT {columns} FROM ({union}) AS reference_targets");
        let frame = crate::sql::query(self.session(), &sql)
            .await
            .map_err(crate::sql::model_error)?;
        let name = format!("_reference_family_{}", self.families.len());
        self.materialize(frame, &name, None).await?;
        self.families.insert(
            identities,
            PreparedFamily {
                schema: relation.schema().clone(),
                table: name.clone(),
            },
        );
        Ok(name)
    }
    async fn materialize(
        &mut self,
        frame: datafusion::dataframe::DataFrame,
        name: &str,
        expected: Option<&SchemaRef>,
    ) -> Result<(), ModelError> {
        self.runtime.cancellation.check()?;
        let mut stream = frame
            .execute_stream()
            .await
            .map_err(crate::sql::model_error)?;
        let schema = stream.schema();
        if expected.is_some_and(|expected| expected.as_ref() != schema.as_ref()) {
            return Err(ModelError::Schema(
                "reference input complete Arrow schema differs",
            ));
        }
        if self.directory.is_none() {
            self.charge.grow(4096)?;
            self.directory = Some(
                match &self.runtime._directory {
                    Some(parent) => tempfile::tempdir_in(parent.path()),
                    None => tempfile::tempdir(),
                }
                .map_err(ModelError::codec)?,
            );
        }
        let path = self
            .directory
            .as_ref()
            .expect("reference directory prepared")
            .path()
            .join(format!("{}.arrow", self.next_file));
        self.next_file += 1;
        let mut footer_charge =
            StateCharge::new(self.runtime.budget(), "receiving reference IPC footer");
        footer_charge.grow(
            4096usize.saturating_add(
                schema
                    .fields()
                    .iter()
                    .map(|field| field.size())
                    .sum::<usize>()
                    .saturating_mul(3),
            ),
        )?;
        let mut writer = FileWriter::try_new(
            File::create(&path).map_err(ModelError::codec)?,
            schema.as_ref(),
        )
        .map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(crate::sql::model_error)? {
            self.runtime.cancellation.check()?;
            // Source stream ownership remains live. Admission covers IPC's simultaneous
            // serialized buffers and the writer's growing block/footer descriptors.
            let _transfer = self.runtime.budget().reserve(
                "receiving reference IPC transfer",
                logical_batch_bytes(&batch)?
                    .saturating_mul(3)
                    .saturating_add(4096),
            )?;
            footer_charge.grow(256)?;
            writer.write(&batch).map_err(ModelError::codec)?;
            tokio::task::yield_now().await;
        }
        self.runtime.cancellation.check()?;
        writer.finish().map_err(ModelError::codec)?;
        drop(writer);
        drop(stream);
        // Retain metadata admission for the listing provider/footer until this entire
        // receiving invocation ends; local query batches use the shared memory pool.
        self.charge.grow(
            footer_charge
                .reserved()
                .saturating_mul(2)
                .saturating_add(4096),
        )?;
        self.session()
            .register_arrow(
                name,
                path.to_string_lossy(),
                ArrowReadOptions::default().schema(schema.as_ref()),
            )
            .await
            .map_err(crate::sql::model_error)?;
        Ok(())
    }
}
