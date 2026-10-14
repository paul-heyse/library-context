//! Admitted semantic graph artifacts. IPC is transport; typed canonical content defines identity.
use crate::admission::{AdmissionResources, AdmissionRuntime};
use datafusion::{
    arrow::{
        array::{Array, BinaryArray, FixedSizeBinaryArray, Int16Array, UInt64Array},
        datatypes::{DataType, Field, Schema},
        ipc::writer::FileWriter,
        record_batch::RecordBatch,
    },
    execution::options::ArrowReadOptions,
    prelude::SessionContext,
};
use futures::TryStreamExt;
use lctx_model::domain::{
    ContentHash, HeapSize, Id, ModelError, Record, completed_transport::ArtifactExportReceipt,
    graph::*,
};
use lctx_model::domain::{charged::StateCharge, resources::ResourceBudget};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

fn schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::FixedSizeBinary(32), false),
        Field::new("content", DataType::FixedSizeBinary(32), false),
        Field::new("payload", DataType::Binary, false),
        Field::new("kind", DataType::Int16, false),
        Field::new("source_length", DataType::UInt64, true),
        Field::new("subtype", DataType::Int16, true),
    ]))
}
struct PendingRow {
    id: ContentHash,
    content: ContentHash,
    payload: Vec<u8>,
    kind: i16,
    source_length: Option<u64>,
    subtype: Option<i16>,
}
struct FamilyWriter {
    path: PathBuf,
    writer: FileWriter<File>,
    pending: Vec<PendingRow>,
    bytes: usize,
    charge: StateCharge,
}
impl FamilyWriter {
    fn new(path: PathBuf, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let writer =
            FileWriter::try_new(File::create(&path).map_err(ModelError::codec)?, &schema())
                .map_err(ModelError::codec)?;
        Ok(Self {
            path,
            writer,
            pending: vec![],
            bytes: 0,
            charge: StateCharge::new(budget, "graph-artifact-buffer"),
        })
    }
    fn push(&mut self, row: PendingRow) -> Result<(), ModelError> {
        if row.payload.len() > 64 << 20 {
            return Err(ModelError::Invalid(
                "graph element exceeds artifact transport limit".into(),
            ));
        }
        if !self.pending.is_empty()
            && (self.pending.len() >= 4096
                || self.bytes.saturating_add(row.payload.len()) > 1 << 20)
        {
            self.flush()?;
        }
        self.charge
            .grow(row.payload.len().saturating_add(size_of::<PendingRow>()))?;
        self.bytes = self.bytes.saturating_add(row.payload.len());
        self.pending.push(row);
        Ok(())
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let _encoding = self
            .charge
            .budget()
            .expect("bound artifact budget")
            .reserve(
                "graph-artifact-encoding",
                self.bytes.saturating_add(self.pending.len() * 96),
            )?;
        let ids = FixedSizeBinaryArray::try_from_iter(self.pending.iter().map(|r| r.id.0))
            .map_err(ModelError::codec)?;
        let contents =
            FixedSizeBinaryArray::try_from_iter(self.pending.iter().map(|r| r.content.0))
                .map_err(ModelError::codec)?;
        let payloads =
            BinaryArray::from_iter_values(self.pending.iter().map(|r| r.payload.as_slice()));
        let kinds = Int16Array::from(self.pending.iter().map(|r| r.kind).collect::<Vec<_>>());
        let lengths = UInt64Array::from(
            self.pending
                .iter()
                .map(|r| r.source_length)
                .collect::<Vec<_>>(),
        );
        let subtypes = Int16Array::from(self.pending.iter().map(|r| r.subtype).collect::<Vec<_>>());
        self.writer
            .write(
                &RecordBatch::try_new(
                    schema(),
                    vec![
                        Arc::new(ids),
                        Arc::new(contents),
                        Arc::new(payloads),
                        Arc::new(kinds),
                        Arc::new(lengths),
                        Arc::new(subtypes),
                    ],
                )
                .map_err(ModelError::codec)?,
            )
            .map_err(ModelError::codec)?;
        self.pending = Vec::new();
        self.charge.release(self.charge.reserved());
        self.bytes = 0;
        Ok(())
    }
    fn finish(mut self) -> Result<PathBuf, ModelError> {
        self.flush()?;
        self.writer.finish().map_err(ModelError::codec)?;
        self.writer
            .into_inner()
            .map_err(ModelError::codec)?
            .sync_all()
            .map_err(ModelError::codec)?;
        Ok(self.path)
    }
}
/// Explicit transport validation retains compact reference and proof metadata only.
struct TransportCheck {
    directory: tempfile::TempDir,
    references: ReferenceWriter,
    derivations: Vec<GraphDerivation>,
    derivation_charge: StateCharge,
}
impl TransportCheck {
    fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let references =
            ReferenceWriter::new(&directory.path().join("references-pending.arrow"), budget)?;
        Ok(Self {
            directory,
            references,
            derivations: Vec::new(),
            derivation_charge: StateCharge::new(budget, "graph-derivation-topology"),
        })
    }
    fn derivation(&mut self, value: Option<GraphDerivation>) -> Result<(), ModelError> {
        if let Some(value) = value {
            self.derivation_charge.grow(
                size_of::<GraphDerivation>()
                    .saturating_add(
                        value
                            .premises
                            .len()
                            .saturating_mul(size_of::<Target>() + 128),
                    )
                    .saturating_add(256),
            )?;
            self.derivations.push(value);
        }
        Ok(())
    }
    fn entity_references(&mut self, value: &Entity) -> Result<(), ModelError> {
        value.validate()?;
        self.derivation(value.derivation()?)?;
        for requirement in value.reference_requirements()? {
            self.references.push(
                requirement.target,
                requirement.kind,
                requirement.subtype,
                None,
            )?;
        }
        if let Entity::Occurrence(occurrence) = value {
            self.references.push(
                Target::Entity(EntityId::of(occurrence.source)),
                Some(EntityKind::Source),
                None,
                Some(occurrence.end as u64),
            )?;
        }
        if let Entity::Evidence(lctx_model::domain::assertion::Evidence::SourceSpan {
            source,
            end,
            ..
        }) = value
        {
            self.references.push(
                Target::Entity(EntityId::of(*source)),
                Some(EntityKind::Source),
                None,
                Some(*end as u64),
            )?;
        }
        Ok(())
    }
    fn assertion_references(&mut self, value: &Assertion) -> Result<(), ModelError> {
        value.validate()?;
        self.derivation(value.declared_derivation())?;
        for requirement in value.reference_requirements()? {
            self.references.push(
                requirement.target,
                requirement.kind,
                requirement.subtype,
                None,
            )?;
        }
        Ok(())
    }
}
/// Immutable admitted content. The constructor is private: callers cannot label pending output admitted.
pub struct AdmittedArtifact {
    workspace: Arc<AdmissionRuntime>,
    original_keys: Vec<(Id<lctx_model::domain::source::SourceArtifact>, Original)>,
    manifest: Manifest,
    _manifest_charge: StateCharge,
    _admission: Arc<ArtifactAdmission>,
    _native_admission: lctx_native::completed_state::HeldFrozenCompletedState,
}
impl AdmittedArtifact {
    pub fn budget(&self) -> &ResourceBudget {
        self.workspace.budget()
    }
    /// Join execution before the enclosing owner reconciles or abandons native state.
    pub async fn drain_owned_report(&self) -> lctx_model::domain::completion::Completion {
        self.workspace.drain_owned_report().await
    }
    /// Settle the original signal lifetime using the enclosing cleanup outcome.
    pub async fn complete_signal(
        &self,
        completion: &lctx_model::domain::completion::Completion,
    ) -> Result<(), ModelError> {
        self.workspace.complete_signal(completion).await
    }
    pub async fn drain_report(&self) -> lctx_model::domain::completion::Completion {
        self.workspace.drain_report().await
    }
    pub fn native(&self) -> &Arc<crate::compiler::NativeCompilerStore> {
        self.workspace.native()
    }
    pub async fn entities(
        &self,
    ) -> Result<futures::stream::BoxStream<'static, Result<Entity, ModelError>>, ModelError> {
        crate::native_canonical::entities(self.workspace.clone()).await
    }
    pub async fn assertions(
        &self,
    ) -> Result<futures::stream::BoxStream<'static, Result<Assertion, ModelError>>, ModelError>
    {
        crate::native_canonical::assertions(self.workspace.clone()).await
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    /// Verify a transported copy against this compiler-owned admitted content. This does not
    /// promote arbitrary files to admitted output or rerun semantic producers.
    pub async fn verify_export(
        &self,
        path: &Path,
        resources: &Arc<impl AdmissionResources>,
    ) -> Result<(), ModelError> {
        let workspace = resources.admission_resources()?;
        let expected = ArtifactExportReceipt {
            manifest: self.manifest.content(),
            state: self
                .native()
                .transport_receipt(self.workspace.budget())
                .await?,
        };
        verify_transport(path, &workspace, Some(&self.manifest), &expected).await?;
        Ok(())
    }
    /// Create a fresh destination using private staging on its filesystem. Never overwrite an existing artifact.
    pub async fn export(&self, destination: &Path) -> Result<ArtifactExportReceipt, ModelError> {
        use lctx_model::domain::completion::{Completion, RemoteState, complete};
        if destination.exists() {
            return Err(ModelError::Invalid(
                "artifact destination already exists".into(),
            ));
        }
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(ModelError::codec)?;
        let staged = tempfile::Builder::new()
            .prefix(".lctx-artifact-")
            .tempdir_in(parent)
            .map_err(ModelError::codec)?;
        let mut destination_owned = false;
        let mut manifest_published = false;
        let mut installed_paths = Vec::new();
        let result: Result<ArtifactExportReceipt, ModelError> = async {
            let budget = self.workspace.budget();
            let mut entity_file = FamilyWriter::new(staged.path().join("entities.arrow"), budget)?;
            let mut entities = self.entities().await?;
            while let Some(row) = entities.try_next().await? {
                entity_file.push(PendingRow {
                    id: row.id().0,
                    content: row.content(),
                    payload: serde_json::to_vec(&row).map_err(ModelError::codec)?,
                    kind: row.kind() as i16,
                    source_length: if let Entity::Source(source) = &row {
                        Some(source.byte_len as u64)
                    } else {
                        None
                    },
                    subtype: row.subtype(),
                })?;
            }
            entity_file.finish()?;
            let mut assertion_file =
                FamilyWriter::new(staged.path().join("assertions.arrow"), budget)?;
            let mut assertions = self.assertions().await?;
            while let Some(row) = assertions.try_next().await? {
                assertion_file.push(PendingRow {
                    id: row.id().0,
                    content: row.content(),
                    payload: serde_json::to_vec(&row).map_err(ModelError::codec)?,
                    kind: row.kind as i16,
                    source_length: None,
                    subtype: None,
                })?;
            }
            assertion_file.finish()?;
            use crate::compiler::NativePredicate;
            use lctx_model::domain::{Relation, artifact::ArtifactChunk};
            use lctx_native::surrealdb::types::{Number, Value};
            use std::io::Write;
            let chunk_view = self.workspace.completed::<ArtifactChunk>()?;
            for (artifact, original) in &self.original_keys {
                let mut file = File::create(
                    staged
                        .path()
                        .join(format!("original-{}.bin", original.source.0.hex())),
                )
                .map_err(ModelError::codec)?;
                let value = Value::Array(
                    artifact
                        .bytes()
                        .iter()
                        .map(|byte| Value::Number(Number::Int(i64::from(*byte))))
                        .collect::<Vec<_>>()
                        .into(),
                );
                let predicate = NativePredicate::Field {
                    field: "artifact".into(),
                    values: vec![value],
                };
                let mut stream = self
                    .native()
                    .scan_batches(
                        chunk_view.view(),
                        &Relation::of::<ArtifactChunk>(),
                        None,
                        Some(predicate),
                        budget,
                        128,
                    )
                    .await?;
                let mut ordinal = 0;
                let mut length = 0u64;
                let mut content = lctx_model::domain::ContentHasher::default();
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.workspace.cancellation().check()?;
                    let _decode = budget.reserve(
                        "artifact-original-decode",
                        lctx_model::domain::logical_batch_bytes(&batch)?.saturating_mul(2),
                    )?;
                    // Full nominal keys sort independently of ordinal. Selected body chunks are
                    // positioned by their authored ordinal and integrity is checked per chunk.
                    for row in ArtifactChunk::decode(&batch)? {
                        use std::io::{Seek, SeekFrom};
                        file.seek(SeekFrom::Start(
                            u64::try_from(row.ordinal).map_err(ModelError::codec)?
                                * lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES as u64,
                        ))
                        .map_err(ModelError::codec)?;
                        file.write_all(&row.body.0).map_err(ModelError::codec)?;
                        ordinal += 1;
                    }
                }
                file.sync_all().map_err(ModelError::codec)?;
                let mut input = File::open(
                    staged
                        .path()
                        .join(format!("original-{}.bin", original.source.0.hex())),
                )
                .map_err(ModelError::codec)?;
                let _buffer = budget.reserve("artifact-original-hash", 65536)?;
                let mut buffer = vec![0; 65536];
                use std::io::Read;
                loop {
                    let count = input.read(&mut buffer).map_err(ModelError::codec)?;
                    if count == 0 {
                        break;
                    }
                    content.update(&buffer[..count]);
                    length += count as u64;
                }
                let expected_chunks = original
                    .byte_len
                    .div_ceil(lctx_model::domain::artifact::ARTIFACT_CHUNK_BYTES as u64);
                if length != original.byte_len
                    || content.finish() != original.content
                    || ordinal != expected_chunks
                {
                    return Err(ModelError::Conflict("artifact original bytes"));
                }
            }
            let state = self
                .native()
                .export_state(
                    &staged.path().join("completed-state.jsonl"),
                    self.workspace.budget(),
                )
                .await?;
            if state.semantic != self.manifest.completed_state {
                return Err(ModelError::Conflict("admitted completed state changed"));
            }
            let manifest_file = staged.path().join("manifest.json");
            std::fs::write(
                &manifest_file,
                serde_json::to_vec_pretty(&self.manifest).map_err(ModelError::codec)?,
            )
            .map_err(ModelError::codec)?;
            File::open(&manifest_file)
                .map_err(ModelError::codec)?
                .sync_all()
                .map_err(ModelError::codec)?;
            // rename_noreplace is not portable: create the destination ownership first, then move files.
            // A refused ownership race preserves both the private artifact and the other destination.
            std::fs::create_dir(destination).map_err(ModelError::codec)?;
            destination_owned = true;
            let installed: Result<(), ModelError> = (|| {
                for entry in std::fs::read_dir(staged.path()).map_err(ModelError::codec)? {
                    let entry = entry.map_err(ModelError::codec)?;
                    if entry.file_name() == "manifest.json" {
                        continue;
                    }
                    let path = destination.join(entry.file_name());
                    std::fs::hard_link(entry.path(), &path).map_err(ModelError::codec)?;
                    installed_paths.push(path);
                }
                let marker = destination.join("manifest.json");
                std::fs::hard_link(&manifest_file, &marker).map_err(ModelError::codec)?;
                manifest_published = true;
                installed_paths.push(marker);
                File::open(destination)
                    .map_err(ModelError::codec)?
                    .sync_all()
                    .map_err(ModelError::codec)?;
                Ok(())
            })();
            installed?;
            Ok(ArtifactExportReceipt {
                manifest: self.manifest.content(),
                state,
            })
        }
        .await;
        let mut completion = Completion::default();
        if manifest_published {
            // The manifest is the visibility marker. Once linked, later durability or
            // staging-cleanup failures must retain both the output and its identity.
            completion.committed("published artifact", destination.display().to_string());
            if result.is_err() {
                completion.remote = RemoteState::Unknown;
            }
        } else if destination_owned {
            // Only paths created by this export are eligible for rollback. Preserve any
            // secondary cleanup failures alongside the original installation failure.
            for path in installed_paths.into_iter().rev() {
                completion.cleanup(
                    path.display().to_string(),
                    std::fs::remove_file(&path).map_err(ModelError::codec),
                );
            }
            completion.cleanup(
                destination.display().to_string(),
                std::fs::remove_dir(destination).map_err(ModelError::codec),
            );
        }
        completion.cleanup(
            staged.path().display().to_string(),
            staged.close().map_err(ModelError::codec),
        );
        if manifest_published {
            // Flush the containing directory after staging removal as well as destination
            // publication. Attempt this finalizer even when another finalizer already failed.
            let synced = File::open(parent)
                .map_err(ModelError::codec)
                .and_then(|file| file.sync_all().map_err(ModelError::codec));
            if synced.is_err() {
                completion.remote = RemoteState::Unknown;
            }
            completion.step("artifact published destination parent sync", synced);
        }
        complete(result, completion)
    }
}

/// Fully checked transport from a trusted local compiler export. This is not provider replay or
/// an authenticity claim for files from an untrusted producer. The operator must keep the export
/// unchanged throughout verification and consumption; native publication reconciles its own copy.
/// The private absolute path prevents later working-directory changes from changing the input.
pub struct VerifiedExport {
    workspace: Arc<AdmissionRuntime>,
    path: PathBuf,
    manifest: Manifest,
    receipt: ArtifactExportReceipt,
}
impl VerifiedExport {
    pub fn receipt(&self) -> &ArtifactExportReceipt {
        &self.receipt
    }
    pub async fn native_entities(
        &self,
    ) -> Result<futures::stream::BoxStream<'static, Result<Entity, ModelError>>, ModelError> {
        crate::native_canonical::entities(self.workspace.clone()).await
    }
    pub async fn native_assertions(
        &self,
    ) -> Result<futures::stream::BoxStream<'static, Result<Assertion, ModelError>>, ModelError>
    {
        crate::native_canonical::assertions(self.workspace.clone()).await
    }
    pub fn budget(&self) -> &ResourceBudget {
        self.workspace.budget()
    }
    pub fn native(&self) -> &Arc<crate::compiler::NativeCompilerStore> {
        self.workspace.native()
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    /// Decode one Arrow batch at a time, retaining no resident graph collection.
    pub fn entities(&self) -> Result<impl Iterator<Item = Result<Entity, ModelError>>, ModelError> {
        Ok(
            records::<Entity>(self.path.join("entities.arrow"))?.map(|value| {
                let value = value?;
                value.validate()?;
                Ok(value)
            }),
        )
    }
    pub fn assertions(
        &self,
    ) -> Result<impl Iterator<Item = Result<Assertion, ModelError>>, ModelError> {
        Ok(
            records::<Assertion>(self.path.join("assertions.arrow"))?.map(|value| {
                let value = value?;
                value.validate()?;
                Ok(value)
            }),
        )
    }
    /// Original bytes remain binary streams; their manifest order is canonical source order.
    pub fn originals(&self) -> impl Iterator<Item = Result<(Original, File), ModelError>> + '_ {
        self.manifest.originals.iter().map(|original| {
            let file = File::open(
                self.path
                    .join(format!("original-{}.bin", original.source.0.hex())),
            )
            .map_err(ModelError::codec)?;
            Ok((original.clone(), file))
        })
    }
}

fn records<T: serde::de::DeserializeOwned + serde::Serialize>(
    path: PathBuf,
) -> Result<impl Iterator<Item = Result<T, ModelError>>, ModelError> {
    let reader = datafusion::arrow::ipc::reader::FileReader::try_new(
        File::open(path).map_err(ModelError::codec)?,
        None,
    )
    .map_err(ModelError::codec)?;
    if reader.schema() != schema() {
        return Err(ModelError::Schema("graph artifact"));
    }
    // A batch stays alive only until its final row is decoded. Iterator errors are propagated to
    // the loader, which must complete every stream before it can publish a realization.
    let mut reader = reader;
    let mut batch = None::<RecordBatch>;
    let mut row = 0;
    let mut failed = false;
    Ok(std::iter::from_fn(move || {
        if failed {
            return None;
        }
        loop {
            if let Some(current) = &batch
                && row < current.num_rows()
            {
                let payloads = current
                    .column(2)
                    .as_any()
                    .downcast_ref::<BinaryArray>()
                    .expect("checked graph artifact schema");
                let bytes = payloads.value(row);
                let value = serde_json::from_slice::<T>(bytes)
                    .map_err(ModelError::codec)
                    .and_then(|value| {
                        if serde_json::to_vec(&value).map_err(ModelError::codec)? != bytes {
                            return Err(ModelError::Conflict("artifact canonical payload"));
                        }
                        Ok(value)
                    });
                row += 1;
                failed = value.is_err();
                return Some(value);
            }
            match reader.next() {
                Some(Ok(next)) => {
                    batch = Some(next);
                    row = 0;
                }
                Some(Err(error)) => {
                    failed = true;
                    return Some(Err(ModelError::codec(error)));
                }
                None => return None,
            }
        }
    }))
}

/// Verify detached compiler output by transport integrity and necessary semantic re-admission.
/// A self-authored manifest cannot confer the live compiler's checked authority.
pub async fn verify_export(
    path: &Path,
    resources: &Arc<impl AdmissionResources>,
    expected: &ArtifactExportReceipt,
) -> Result<VerifiedExport, ModelError> {
    let workspace = resources.admission_resources()?;
    let CheckedTransport { path, manifest } =
        verify_transport(path, &workspace, None, expected).await?;
    readmit_detached(&path, &manifest, &workspace, expected).await?;
    Ok(VerifiedExport {
        workspace: workspace.clone(),
        path,
        manifest,
        receipt: expected.clone(),
    })
}
struct CheckedTransport {
    path: PathBuf,
    manifest: Manifest,
}

async fn verify_transport(
    path: &Path,
    workspace: &Arc<AdmissionRuntime>,
    expected: Option<&Manifest>,
    receipt: &ArtifactExportReceipt,
) -> Result<CheckedTransport, ModelError> {
    let path = std::fs::canonicalize(path).map_err(ModelError::codec)?;
    let manifest =
        Manifest::decode(&std::fs::read(path.join("manifest.json")).map_err(ModelError::codec)?)?;
    receipt.validate()?;
    if manifest.content() != receipt.manifest || manifest.completed_state != receipt.state.semantic
    {
        return Err(ModelError::Conflict(
            "artifact independent receipt manifest",
        ));
    }
    lctx_native::completed_transport::read_transport(
        &path.join("completed-state.jsonl"),
        &receipt.state,
        workspace.budget(),
    )?;
    if manifest.semantic_contract != semantic_contract(workspace.model()) {
        return Err(ModelError::Conflict("artifact semantic contract"));
    }
    if expected.is_some_and(|expected| manifest != *expected) {
        return Err(ModelError::Conflict("artifact manifest"));
    }
    if manifest
        .families
        .iter()
        .map(|family| family.family)
        .collect::<Vec<_>>()
        != [GraphFamily::Entities, GraphFamily::Assertions]
    {
        return Err(ModelError::Conflict("artifact graph families"));
    }
    let mut metadata_charge = StateCharge::new(workspace.budget(), "artifact-transport-metadata");
    metadata_charge.grow(
        manifest.originals.len() * (size_of::<EntityId>() + 64)
            + manifest.captures.len() * (size_of::<EntityId>() + 32)
            + manifest.embeddings.len() * (size_of::<(ContentHash, ContentHash)>() + 32),
    )?;
    let mut originals = manifest
        .originals
        .iter()
        .map(|o| (o.source, o))
        .collect::<BTreeMap<_, _>>();
    let mut captures = manifest.captures.iter().copied().collect::<BTreeSet<_>>();
    let mut specifications = BTreeMap::new();
    let mut projections = BTreeSet::new();
    let mut embeddings = manifest
        .embeddings
        .iter()
        .map(|value| (value.specification, value.text))
        .collect::<BTreeSet<_>>();
    let encoder_keys = manifest
        .embeddings
        .iter()
        .map(|v| {
            (
                Id::of(&lctx_model::domain::embedding::EmbeddingSpecKey {
                    service_hash: v.specification,
                }),
                v.specification,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut check = TransportCheck::new(workspace.budget())?;
    for (file, family) in [
        ("entities.arrow", GraphFamily::Entities),
        ("assertions.arrow", GraphFamily::Assertions),
    ] {
        let reader = datafusion::arrow::ipc::reader::FileReader::try_new(
            File::open(path.join(file)).map_err(ModelError::codec)?,
            None,
        )
        .map_err(ModelError::codec)?;
        if reader.schema() != schema() {
            return Err(ModelError::Schema("graph artifact"));
        }
        let mut content = FamilyHasher::new(family);
        for batch in reader {
            workspace.cancellation().check()?;
            let batch = batch.map_err(ModelError::codec)?;
            let _decode = workspace.budget().reserve(
                "graph-artifact-transport-decode",
                lctx_model::domain::logical_batch_bytes(&batch)?.saturating_mul(4),
            )?;
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("graph artifact"))?;
            let hashes = batch
                .column(1)
                .as_any()
                .downcast_ref::<FixedSizeBinaryArray>()
                .ok_or(ModelError::Schema("graph artifact"))?;
            let payloads = batch
                .column(2)
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or(ModelError::Schema("graph artifact"))?;
            let kinds = batch
                .column(3)
                .as_any()
                .downcast_ref::<Int16Array>()
                .ok_or(ModelError::Schema("graph artifact"))?;
            let lengths = batch
                .column(4)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or(ModelError::Schema("graph artifact"))?;
            let subtypes = batch
                .column(5)
                .as_any()
                .downcast_ref::<Int16Array>()
                .ok_or(ModelError::Schema("graph artifact"))?;
            if batch.columns()[..4]
                .iter()
                .any(|column| column.null_count() != 0)
            {
                return Err(ModelError::Schema("graph artifact"));
            }
            for row in 0..batch.num_rows() {
                let bytes = payloads.value(row);
                if bytes.len() > 64 << 20 {
                    return Err(ModelError::Invalid(
                        "graph element exceeds artifact transport limit".into(),
                    ));
                }
                let (id, hash, kind, length, subtype) = if family == GraphFamily::Entities {
                    let value: Entity = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
                    if serde_json::to_vec(&value).map_err(ModelError::codec)? != bytes {
                        return Err(ModelError::Conflict("artifact canonical payload"));
                    }
                    match &value {
                        Entity::Capture(capture) => {
                            if !captures.remove(&EntityId::of(capture.id())) {
                                return Err(ModelError::Conflict("artifact capture membership"));
                            }
                        }
                        Entity::Source(source) => {
                            let original = originals
                                .remove(&EntityId::of(source.id()))
                                .ok_or(ModelError::Conflict("artifact source membership"))?;
                            if original.content != source.content
                                || original.byte_len != source.byte_len as u64
                                || manifest
                                    .captures
                                    .binary_search(&EntityId::of(source.input))
                                    .is_err()
                            {
                                return Err(ModelError::Conflict("artifact source metadata"));
                            }
                        }
                        Entity::EmbeddingFullValue(full) => {
                            full.validate()?;
                            let encoder = encoder_keys
                                .get(&full.encoder)
                                .ok_or(ModelError::Conflict("artifact full winner encoder"))?;
                            let key = (*encoder, full.input);
                            let position = manifest
                                .embeddings
                                .binary_search_by_key(&key, |v| (v.specification, v.text))
                                .map_err(|_| {
                                    ModelError::Conflict("artifact full winner membership")
                                })?;
                            let expected = &manifest.embeddings[position];
                            if i64::from(expected.dimension) != full.dimensions
                                || expected.values != full.digest
                                || !embeddings.remove(&key)
                            {
                                return Err(ModelError::Conflict(
                                    "artifact full winner differs from manifest",
                                ));
                            }
                        }
                        Entity::EmbeddingSpecification(specification) => {
                            let configuration = specification.configuration()?;
                            metadata_charge.grow(
                                size_of::<lctx_model::domain::embedding::Spec>()
                                    + configuration.heap_bytes()
                                    + 64,
                            )?;
                            specifications.insert(specification.id(), configuration);
                        }
                        _ => {}
                    }
                    let length = if let Entity::Source(source) = &value {
                        Some(source.byte_len as u64)
                    } else {
                        None
                    };
                    let metadata = (
                        value.id().0,
                        value.content(),
                        value.kind() as i16,
                        length,
                        value.subtype(),
                    );
                    check.entity_references(&value)?;
                    metadata
                } else {
                    let value: Assertion =
                        serde_json::from_slice(bytes).map_err(ModelError::codec)?;
                    if serde_json::to_vec(&value).map_err(ModelError::codec)? != bytes {
                        return Err(ModelError::Conflict("artifact canonical payload"));
                    }
                    match &value.value {
                        AssertionValue::Claim(ClaimValue::ProjectionSourceAssessments(
                            assessment,
                        )) => {
                            projections.insert(assessment.projection);
                        }
                        AssertionValue::Claim(ClaimValue::AnalysisEmbeddingUses(consumption)) => {
                            consumption.validate()?;
                        }
                        AssertionValue::Analysis(AnalysisValue::RetrievalEmbeddingUse(
                            consumption,
                        )) => {
                            consumption.validate()?;
                        }
                        _ => {}
                    }
                    let metadata = (value.id().0, value.content(), value.kind as i16, None, None);
                    check.assertion_references(&value)?;
                    metadata
                };
                if ids.value(row) != id.0
                    || hashes.value(row) != hash.0
                    || kinds.value(row) != kind
                    || (!lengths.is_null(row)).then(|| lengths.value(row)) != length
                    || (!subtypes.is_null(row)).then(|| subtypes.value(row)) != subtype
                {
                    return Err(ModelError::Conflict("artifact element metadata"));
                }
                if !content.push(id, hash)? {
                    return Err(ModelError::Invalid(
                        "artifact stream contains duplicate element".into(),
                    ));
                }
            }
        }
        let content = content.finish();
        if !manifest.families.contains(&content) {
            return Err(ModelError::Conflict("artifact graph family"));
        }
    }
    if !originals.is_empty() || !captures.is_empty() {
        return Err(ModelError::Conflict(
            "artifact source or capture membership",
        ));
    }
    verify_projections(&projections, &manifest)?;
    if !embeddings.is_empty() {
        return Err(ModelError::Conflict("artifact embedding membership"));
    }
    admit_graph_derivations(&check.derivations)?;
    let TransportCheck {
        directory,
        references,
        ..
    } = check;
    references.finish()?;
    let base = workspace.transport_session();
    let context = SessionContext::new_with_config_rt(
        base.copied_config()
            .set_bool("datafusion.optimizer.prefer_hash_join", false),
        base.runtime_env(),
    );
    reference_closure(
        &context,
        &path.join("entities.arrow"),
        &path.join("assertions.arrow"),
        &directory.path().join("references-pending.arrow"),
    )
    .await?;
    use std::io::Read;
    let _copy = workspace
        .budget()
        .reserve("artifact-original-verification", 65536)?;
    let mut buffer = vec![0u8; 65536];
    for original in &manifest.originals {
        let mut input = File::open(path.join(format!("original-{}.bin", original.source.0.hex())))
            .map_err(ModelError::codec)?;
        let mut content = lctx_model::domain::ContentHasher::default();
        let mut length = 0u64;
        loop {
            workspace.cancellation().check()?;
            let count = input.read(&mut buffer).map_err(ModelError::codec)?;
            if count == 0 {
                break;
            }
            content.update(&buffer[..count]);
            length += count as u64;
        }
        if length != original.byte_len || content.finish() != original.content {
            return Err(ModelError::Conflict("artifact original bytes"));
        }
    }
    Ok(CheckedTransport { path, manifest })
}

async fn readmit_detached(
    path: &Path,
    manifest: &Manifest,
    runtime: &Arc<AdmissionRuntime>,
    receipt: &ArtifactExportReceipt,
) -> Result<(), ModelError> {
    for (file, entities) in [("entities.arrow", true), ("assertions.arrow", false)] {
        let reader = datafusion::arrow::ipc::reader::FileReader::try_new(
            File::open(path.join(file)).map_err(ModelError::codec)?,
            None,
        )
        .map_err(ModelError::codec)?;
        for batch in reader {
            runtime.cancellation().check()?;
            let batch = batch.map_err(ModelError::codec)?;
            let _decode = runtime.budget().reserve(
                "detached-semantic-decode",
                lctx_model::domain::logical_batch_bytes(&batch)?.saturating_mul(4),
            )?;
            let payloads = batch
                .column(2)
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or(ModelError::Schema("graph artifact"))?;
            if entities {
                let rows = (0..batch.num_rows())
                    .map(|index| {
                        serde_json::from_slice::<Entity>(payloads.value(index))
                            .map_err(ModelError::codec)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let native = runtime.native().clone();
                runtime
                    .native_call(async move { native.import_entities(&rows).await })
                    .await?;
            } else {
                let rows = (0..batch.num_rows())
                    .map(|index| {
                        serde_json::from_slice::<Assertion>(payloads.value(index))
                            .map_err(ModelError::codec)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let native = runtime.native().clone();
                runtime
                    .native_call(async move { native.import_assertions(&rows).await })
                    .await?;
            }
        }
    }
    for original in &manifest.originals {
        let mut file = File::open(path.join(format!("original-{}.bin", original.source.0.hex())))
            .map_err(ModelError::codec)?;
        let native = runtime.native().clone();
        let original = original.clone();
        let budget = runtime.budget().clone();
        runtime
            .native_call(async move {
                native
                    .import_original_stream(
                        original.source.0,
                        original.content,
                        original.byte_len,
                        &mut file,
                        &budget,
                    )
                    .await
            })
            .await?;
    }
    let native = runtime.native().clone();
    let path = path.join("completed-state.jsonl");
    let state = receipt.state.clone();
    let budget = runtime.budget().clone();
    runtime
        .native_call(async move { native.import_state(&path, &state, &budget).await })
        .await?;
    runtime.restore(manifest.profile).await?;
    verify_restored(runtime, manifest).await.map(|_| ())
}

/// Independently admitted restored content; physical freeze alone cannot construct this owner.
pub struct RestoredAdmission {
    workspace: Arc<AdmissionRuntime>,
    manifest: Manifest,
    _manifest_charge: StateCharge,
    _admission: Arc<ArtifactAdmission>,
    _native_admission: lctx_native::completed_state::HeldFrozenCompletedState,
}
impl RestoredAdmission {
    pub fn budget(&self) -> &ResourceBudget {
        self.workspace.budget()
    }
    pub fn native(&self) -> &Arc<crate::compiler::NativeCompilerStore> {
        self.workspace.native()
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub async fn entities(
        &self,
    ) -> Result<futures::stream::BoxStream<'static, Result<Entity, ModelError>>, ModelError> {
        crate::native_canonical::entities(self.workspace.clone()).await
    }
    pub async fn assertions(
        &self,
    ) -> Result<futures::stream::BoxStream<'static, Result<Assertion, ModelError>>, ModelError>
    {
        crate::native_canonical::assertions(self.workspace.clone()).await
    }
}

/// Admit a detached native realization against its exact restored completed bindings.
pub async fn verify_restored(
    runtime: &Arc<AdmissionRuntime>,
    manifest: &Manifest,
) -> Result<RestoredAdmission, ModelError> {
    let phase = lctx_native::phase::Phase::begin("restored_artifact_admission");
    let result = async {
        manifest.validate()?;
        runtime.freeze_for_admission().await?;
        let mut charge = StateCharge::new(runtime.budget(), "restored-producer-inventory");
        if crate::artifact_manifest::producer_inventory(runtime, &mut charge).await?
            != manifest.producers
        {
            return Err(ModelError::Conflict(
                "restored contribution producer inventory",
            ));
        }
        if runtime
            .admitted_facts_async(manifest.profile)
            .await?
            .contract
            != manifest.admission_contract
        {
            return Err(ModelError::Conflict("restored facts admission contract"));
        }
        let semantic = runtime.execute_semantics(manifest.profile).await?;
        let frontier = runtime
            .execute_frontier(manifest.frontier, manifest.profile)
            .await?;
        crate::artifact_manifest::verify_actual(runtime, manifest).await?;
        runtime.native().prepare_canonical(runtime.budget()).await?;
        let graph = execute_graph_admission(runtime).await?;
        let (captures, originals, _inventory_charge) = original_inventory(runtime).await?;
        if captures != manifest.captures
            || originals
                .iter()
                .map(|(_, original)| original)
                .ne(manifest.originals.iter())
        {
            return Err(ModelError::Conflict("restored actual captures/originals"));
        }
        // Reserve every cloned vector and nested string before retaining the admission owner.
        let mut manifest_charge = StateCharge::new(runtime.budget(), "restored-admission-manifest");
        manifest_charge.grow(size_of::<Manifest>())?;
        manifest_charge.grow(
            manifest
                .captures
                .len()
                .saturating_mul(size_of::<EntityId>()),
        )?;
        manifest_charge.grow(
            manifest
                .producers
                .len()
                .saturating_mul(size_of::<ProducerImplementation>()),
        )?;
        manifest_charge.grow(
            manifest
                .families
                .len()
                .saturating_mul(size_of::<FamilyContent>()),
        )?;
        manifest_charge.grow(
            manifest
                .required_outcomes
                .len()
                .saturating_mul(size_of::<OutcomeKey>()),
        )?;
        manifest_charge.grow(manifest.outcomes.len().saturating_mul(size_of::<Outcome>()))?;
        manifest_charge.grow(
            manifest
                .originals
                .len()
                .saturating_mul(size_of::<Original>()),
        )?;
        manifest_charge.grow(
            manifest
                .projections
                .len()
                .saturating_mul(size_of::<ProjectionDefinition>()),
        )?;
        manifest_charge.grow(
            manifest
                .embeddings
                .len()
                .saturating_mul(size_of::<EmbeddingConsumption>()),
        )?;
        for producer in &manifest.producers {
            manifest_charge.grow(producer.producer.len())?;
        }
        for outcome in &manifest.required_outcomes {
            manifest_charge.grow(outcome.producer.len())?;
        }
        for outcome in &manifest.outcomes {
            manifest_charge.grow(outcome.key.producer.len())?;
            manifest_charge.grow(outcome.detail.as_ref().map_or(0, String::len))?;
        }
        for projection in &manifest.projections {
            manifest_charge.grow(projection.name.len())?;
            manifest_charge.grow(
                projection
                    .declared_losses
                    .len()
                    .saturating_mul(size_of::<String>()),
            )?;
            for loss in &projection.declared_losses {
                manifest_charge.grow(loss.len())?;
            }
        }
        let admission =
            Arc::new(assemble_admission(runtime, semantic, frontier, graph, manifest).await?);
        let native_admission = runtime
            .native()
            .admit_completed_state(&admission, runtime.budget())
            .await?;
        Ok::<_, ModelError>(RestoredAdmission {
            workspace: runtime.clone(),
            manifest: manifest.clone(),
            _manifest_charge: manifest_charge,
            _admission: admission,
            _native_admission: native_admission,
        })
    }
    .await;
    phase.finish_result(&result);
    result
}

fn verify_projections(
    represented: &BTreeSet<lctx_model::domain::projection::ProjectionName>,
    manifest: &Manifest,
) -> Result<(), ModelError> {
    use lctx_model::domain::{Record, projection::ProjectionSpec};
    if represented.len() != manifest.projections.len() {
        return Err(ModelError::Conflict("artifact projection membership"));
    }
    for name in represented {
        let index = manifest
            .projections
            .binary_search_by(|value| value.name.cmp(&format!("{name:?}")))
            .map_err(|_| ModelError::Conflict("artifact projection membership"))?;
        let projection = &manifest.projections[index];
        let definition = lctx_model::domain::analysis::ProjectionDefinition::builtin(*name);
        let mut excluded = ProjectionSpec::builtin(*name)
            .excluded_categories()
            .map(|category| category.label())
            .collect::<Vec<_>>();
        excluded.sort_unstable();
        let mut losses = vec![format!("ProjectionSpec::accepts excludes {} entities", excluded.join(", ")),
            "topology omits conditions, provider qualifications and source evidence retained by assertions".into()];
        losses.sort();
        if projection.definition != definition.content_digest()
            || projection.declared_losses != losses
        {
            return Err(ModelError::Conflict("artifact projection definition"));
        }
        // source_membership binds compiler-only completed projection inputs. A trusted compiler
        // declares this digest; transport validation does not replay or invent those inputs.
    }
    Ok(())
}

fn reference_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("target", DataType::FixedSizeBinary(32), false),
        Field::new("namespace", DataType::Int16, false),
        Field::new("expected_kind", DataType::Int16, true),
        Field::new("expected_subtype", DataType::Int16, true),
        Field::new("max_end", DataType::UInt64, true),
    ]))
}
struct ReferenceRow {
    target: ContentHash,
    namespace: i16,
    kind: Option<i16>,
    subtype: Option<i16>,
    max_end: Option<u64>,
}
struct ReferenceWriter {
    writer: FileWriter<File>,
    pending: Vec<ReferenceRow>,
    charge: StateCharge,
}
impl ReferenceWriter {
    fn new(path: &Path, budget: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            writer: FileWriter::try_new(
                File::create(path).map_err(ModelError::codec)?,
                &reference_schema(),
            )
            .map_err(ModelError::codec)?,
            pending: vec![],
            charge: StateCharge::new(budget, "graph-artifact-references"),
        })
    }
    fn push(
        &mut self,
        target: Target,
        kind: Option<EntityKind>,
        subtype: Option<i16>,
        max_end: Option<u64>,
    ) -> Result<(), ModelError> {
        self.charge.grow(size_of::<ReferenceRow>())?;
        match target {
            Target::Entity(id) => self.pending.push(ReferenceRow {
                target: id.0,
                namespace: 0,
                kind: kind.map(|k| k as i16),
                subtype,
                max_end,
            }),
            Target::Assertion(id) => self.pending.push(ReferenceRow {
                target: id.0,
                namespace: 1,
                kind: None,
                subtype: None,
                max_end: None,
            }),
            Target::External {
                provider, context, ..
            } => {
                self.charge.release(size_of::<ReferenceRow>());
                self.push(
                    Target::Entity(provider),
                    Some(EntityKind::Provider),
                    None,
                    None,
                )?;
                self.push(
                    Target::Entity(context),
                    Some(EntityKind::Context),
                    None,
                    None,
                )?;
            }
        }
        if self.pending.len() >= 4096 {
            self.flush()?;
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let _encoding = self
            .charge
            .budget()
            .expect("bound artifact budget")
            .reserve("graph-artifact-reference-encoding", self.pending.len() * 64)?;
        let targets = FixedSizeBinaryArray::try_from_iter(self.pending.iter().map(|r| r.target.0))
            .map_err(ModelError::codec)?;
        let namespaces =
            Int16Array::from(self.pending.iter().map(|r| r.namespace).collect::<Vec<_>>());
        let kinds = Int16Array::from(self.pending.iter().map(|r| r.kind).collect::<Vec<_>>());
        let subtypes = Int16Array::from(self.pending.iter().map(|r| r.subtype).collect::<Vec<_>>());
        let ends = UInt64Array::from(self.pending.iter().map(|r| r.max_end).collect::<Vec<_>>());
        self.writer
            .write(
                &RecordBatch::try_new(
                    reference_schema(),
                    vec![
                        Arc::new(targets),
                        Arc::new(namespaces),
                        Arc::new(kinds),
                        Arc::new(subtypes),
                        Arc::new(ends),
                    ],
                )
                .map_err(ModelError::codec)?,
            )
            .map_err(ModelError::codec)?;
        self.pending = Vec::new();
        self.charge.release(self.charge.reserved());
        Ok(())
    }
    fn finish(mut self) -> Result<(), ModelError> {
        self.flush()?;
        self.writer.finish().map_err(ModelError::codec)?;
        Ok(())
    }
}
async fn reference_closure(
    context: &SessionContext,
    entities: &Path,
    assertions: &Path,
    references: &Path,
) -> Result<(), ModelError> {
    context
        .register_arrow(
            "graph_entities",
            entities.to_string_lossy(),
            ArrowReadOptions::default().schema(&schema()),
        )
        .await
        .map_err(ModelError::codec)?;
    context
        .register_arrow(
            "graph_assertions",
            assertions.to_string_lossy(),
            ArrowReadOptions::default().schema(&schema()),
        )
        .await
        .map_err(ModelError::codec)?;
    context
        .register_arrow(
            "graph_references",
            references.to_string_lossy(),
            ArrowReadOptions::default().schema(&reference_schema()),
        )
        .await
        .map_err(ModelError::codec)?;
    let result=async{
        let sql="WITH targets AS (SELECT id, 0 AS namespace, kind, subtype, source_length FROM graph_entities UNION ALL SELECT id, 1 AS namespace, NULL AS kind, NULL AS subtype, NULL AS source_length FROM graph_assertions) SELECT r.target,r.namespace,r.expected_kind,r.expected_subtype,t.kind,t.subtype,r.max_end,t.source_length FROM graph_references r LEFT JOIN targets t ON r.target=t.id AND r.namespace=t.namespace WHERE t.id IS NULL OR (r.expected_kind IS NOT NULL AND (t.kind IS NULL OR r.expected_kind<>t.kind)) OR (r.expected_subtype IS NOT NULL AND (t.subtype IS NULL OR r.expected_subtype<>t.subtype)) OR (r.max_end IS NOT NULL AND (t.source_length IS NULL OR r.max_end>t.source_length)) LIMIT 1";
        let mut stream=crate::sql::query(context,sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows()!=0 {
                let target=batch.column(0).as_any().downcast_ref::<FixedSizeBinaryArray>().ok_or(ModelError::Schema("graph reference"))?;
                let namespace=batch.column(1).as_any().downcast_ref::<Int16Array>().ok_or(ModelError::Schema("graph reference"))?;
                return Err(ModelError::Invalid(format!("graph reference closure, nominal role or original span failed: {} namespace {}, expected kind/subtype {:?}/{:?}, actual {:?}/{:?}, end/length {:?}/{:?}",ContentHash(target.value(0).try_into().map_err(ModelError::codec)?).hex(),namespace.value(0),batch.column(2),batch.column(3),batch.column(4),batch.column(5),batch.column(6),batch.column(7))));
            }
        }
        Ok(())
    }.await;
    for name in ["graph_entities", "graph_assertions", "graph_references"] {
        context.deregister_table(name).map_err(ModelError::codec)?;
    }
    result
}

/// Derive the manifest inventory from exact completed typed content, never caller labels.
async fn original_inventory(
    runtime: &AdmissionRuntime,
) -> Result<
    (
        Vec<EntityId>,
        Vec<(Id<lctx_model::domain::source::SourceArtifact>, Original)>,
        StateCharge,
    ),
    ModelError,
> {
    use lctx_model::domain::{Batch, Relation, input::InputRevision, source::SourceArtifact};
    let mut charge = StateCharge::new(runtime.budget(), "artifact-source-inventory");
    let mut captures = Vec::new();
    let mut originals = Vec::new();
    let source = runtime.completed::<InputRevision>()?;
    let mut stream = runtime
        .native()
        .scan_batches(
            source.view(),
            &Relation::of::<InputRevision>(),
            None,
            None,
            runtime.budget(),
            runtime.options().batch_rows,
        )
        .await?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        runtime.cancellation().check()?;
        let batch = Batch::<InputRevision>::read(runtime.model(), &batch, runtime.budget())?;
        for row in batch.rows() {
            charge.grow(size_of::<EntityId>() * 2)?;
            captures.push(EntityId::of(row.id()));
        }
    }
    let source = runtime.completed::<SourceArtifact>()?;
    let mut stream = runtime
        .native()
        .scan_batches(
            source.view(),
            &Relation::of::<SourceArtifact>(),
            None,
            None,
            runtime.budget(),
            runtime.options().batch_rows,
        )
        .await?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        runtime.cancellation().check()?;
        let batch = Batch::<SourceArtifact>::read(runtime.model(), &batch, runtime.budget())?;
        for row in batch.rows() {
            charge.grow((size_of::<Id<SourceArtifact>>() + size_of::<Original>()) * 2)?;
            originals.push((
                row.id(),
                Original {
                    source: EntityId::of(row.id()),
                    content: row.content,
                    byte_len: u64::try_from(row.byte_len).map_err(ModelError::codec)?,
                },
            ));
        }
    }
    captures.sort();
    captures.dedup();
    originals.sort_by_key(|(_, original)| original.source);
    originals.dedup();
    Ok((captures, originals, charge))
}

struct CheckedGraph {
    families: Vec<FamilyContent>,
    _charge: StateCharge,
}
/// Complete production authority. Native receives no public DTO-to-admitted route;
/// only this private constructor composes executed facts, semantic/frontier, graph and
/// checked manifest evidence over the independently earned exact physical completion.
pub(crate) struct ArtifactAdmission {
    physical: Arc<lctx_native::completed_state::NativeCompletedState>,
    model: Arc<lctx_model::domain::ValidatedModel>,
    evidence: lctx_model::domain::artifact_admission::ArtifactAdmissionEvidence,
    _charges: Vec<StateCharge>,
    _scoped: Vec<Arc<crate::admission::scoped::ExecutedScopedCheck>>,
}
impl ArtifactAdmission {
    pub(crate) fn physical(&self) -> &lctx_native::completed_state::NativeCompletedState {
        &self.physical
    }
    pub(crate) fn model(&self) -> &lctx_model::domain::ValidatedModel {
        &self.model
    }
    pub(crate) fn evidence(
        &self,
    ) -> &lctx_model::domain::artifact_admission::ArtifactAdmissionEvidence {
        &self.evidence
    }
    pub(crate) fn retained_bytes(&self) -> usize {
        self._charges.iter().map(StateCharge::reserved).sum()
    }
}
fn manifest_retention_bytes(manifest: &Manifest) -> usize {
    size_of::<Manifest>()
        .saturating_add(
            manifest
                .captures
                .capacity()
                .saturating_mul(size_of::<EntityId>()),
        )
        .saturating_add(
            manifest
                .producers
                .capacity()
                .saturating_mul(size_of::<ProducerImplementation>()),
        )
        .saturating_add(
            manifest
                .producers
                .iter()
                .map(|producer| producer.producer.capacity())
                .sum::<usize>(),
        )
        .saturating_add(
            manifest
                .families
                .capacity()
                .saturating_mul(size_of::<FamilyContent>()),
        )
        .saturating_add(
            manifest
                .required_outcomes
                .capacity()
                .saturating_mul(size_of::<OutcomeKey>()),
        )
        .saturating_add(
            manifest
                .required_outcomes
                .iter()
                .map(|outcome| outcome.producer.capacity())
                .sum::<usize>(),
        )
        .saturating_add(
            manifest
                .outcomes
                .capacity()
                .saturating_mul(size_of::<Outcome>()),
        )
        .saturating_add(
            manifest
                .outcomes
                .iter()
                .map(|outcome| {
                    outcome
                        .key
                        .producer
                        .capacity()
                        .saturating_add(outcome.detail.as_ref().map_or(0, String::capacity))
                })
                .sum::<usize>(),
        )
        .saturating_add(
            manifest
                .originals
                .capacity()
                .saturating_mul(size_of::<Original>()),
        )
        .saturating_add(
            manifest
                .embeddings
                .capacity()
                .saturating_mul(size_of::<EmbeddingConsumption>()),
        )
        .saturating_add(
            manifest
                .projections
                .capacity()
                .saturating_mul(size_of::<ProjectionDefinition>()),
        )
        .saturating_add(
            manifest
                .projections
                .iter()
                .map(|projection| {
                    projection
                        .name
                        .capacity()
                        .saturating_add(
                            projection
                                .declared_losses
                                .capacity()
                                .saturating_mul(size_of::<String>()),
                        )
                        .saturating_add(
                            projection
                                .declared_losses
                                .iter()
                                .map(String::capacity)
                                .sum::<usize>(),
                        )
                })
                .sum::<usize>(),
        )
}
async fn assemble_admission(
    runtime: &Arc<AdmissionRuntime>,
    semantic: crate::admission::CheckedSemanticScope,
    frontier: crate::admission::CheckedFrontier,
    graph: CheckedGraph,
    manifest: &Manifest,
) -> Result<ArtifactAdmission, ModelError> {
    use lctx_model::domain::{
        artifact_admission::{ArtifactAdmissionEvidence, production_contract},
        completed::binding_inventory_identity,
    };
    manifest.validate()?;
    let physical = runtime
        .native()
        .complete_physical_content(runtime.budget())
        .await?;
    let bindings = physical.inventory().bindings();
    let binding_identity = binding_inventory_identity(bindings)?;
    let facts = runtime.admitted_facts_async(manifest.profile).await?;
    if facts.contract != manifest.admission_contract
        || graph.families != manifest.families
        || manifest.semantic_contract != semantic_contract(runtime.model())
    {
        return Err(ModelError::Conflict(
            "complete artifact facts/graph/manifest gates disagree",
        ));
    }
    let (invariants, semantic_charge, scoped) =
        semantic.into_evidence(runtime.model().digest(), manifest.profile, binding_identity)?;
    let (frontier, frontier_charge) = frontier.into_evidence(
        runtime.model().digest(),
        manifest.profile,
        manifest.frontier,
    )?;
    let mut charge = StateCharge::new(
        runtime.budget(),
        "complete artifact admission retained evidence",
    );
    charge.grow(
        manifest_retention_bytes(manifest)
            .saturating_mul(2)
            .saturating_add(4096),
    )?;
    let evidence = ArtifactAdmissionEvidence {
        version: 1,
        model: runtime.model().digest(),
        bindings: binding_identity,
        manifest: manifest.clone(),
        invariants,
        frontier,
        production_contract: production_contract(),
    };
    evidence.validate_current(
        runtime.model(),
        bindings,
        &physical.completion().roots,
        runtime.budget(),
    )?;
    Ok(ArtifactAdmission {
        physical,
        model: runtime.model().clone(),
        evidence,
        _charges: vec![semantic_charge, frontier_charge, graph._charge, charge],
        _scoped: scoped,
    })
}
/// This constructor executes all canonical entities/assertions and derivations to EOF.
/// Its result cannot be manufactured from manifest family hashes.
async fn execute_graph_admission(
    workspace: &Arc<AdmissionRuntime>,
) -> Result<CheckedGraph, ModelError> {
    let lookup = crate::native_canonical::Lookup::load(workspace).await?;
    let mut derivations = Vec::new();
    let mut proof_charge = StateCharge::new(workspace.budget(), "native-final-derivations");
    let mut family = FamilyHasher::new(GraphFamily::Entities);
    let mut entities = crate::native_canonical::entities(workspace.clone()).await?;
    while let Some(row) = entities.try_next().await? {
        admit_entity(&row, &lookup)?;
        family.push(row.id().0, row.content())?;
        if let Some(proof) = row.derivation()? {
            proof_charge.grow(
                size_of::<GraphDerivation>()
                    + proof.premises.len() * (size_of::<Target>() + 128)
                    + 256,
            )?;
            derivations.push(proof);
        }
    }
    let entities_content = family.finish();
    let mut family = FamilyHasher::new(GraphFamily::Assertions);
    let mut assertions = crate::native_canonical::assertions(workspace.clone()).await?;
    while let Some(row) = assertions.try_next().await? {
        admit_assertion(&row, &lookup)?;
        family.push(row.id().0, row.content())?;
        if let Some(proof) = row.declared_derivation() {
            proof_charge.grow(
                size_of::<GraphDerivation>()
                    + proof.premises.len() * (size_of::<Target>() + 128)
                    + 256,
            )?;
            derivations.push(proof);
        }
    }
    let assertions_content = family.finish();
    admit_graph_derivations(&derivations)?;
    drop(derivations);
    drop(proof_charge);
    drop(lookup);
    let mut charge = StateCharge::new(
        workspace.budget(),
        "executed canonical graph admission receipt",
    );
    charge.grow(2 * size_of::<FamilyContent>() + 128)?;
    Ok(CheckedGraph {
        families: vec![entities_content, assertions_content],
        _charge: charge,
    })
}

struct PreparedArtifactAdmission {
    original_keys: Vec<(Id<lctx_model::domain::source::SourceArtifact>, Original)>,
    manifest: Manifest,
    manifest_charge: StateCharge,
    admission: Arc<ArtifactAdmission>,
}
/// Shared production gates. Neither an ordinary caller nor maintenance may
/// replace these executions with descriptor hashes or caller-authored labels.
async fn prepare_full_admission(
    workspace: &Arc<AdmissionRuntime>,
    frontier: lctx_model::domain::admission::Frontier,
    profile: lctx_model::domain::stages::Profile,
    settings: crate::artifact_manifest::SettingsOrigin,
) -> Result<PreparedArtifactAdmission, ModelError> {
    async {
        workspace.cancellation().check()?;
        workspace.freeze_for_admission().await?;
        let semantic_phase = lctx_native::phase::Phase::begin("artifact_semantic_admission");
        let semantic_result = async {
            let facts_phase = lctx_native::phase::Phase::begin("artifact_facts_admission");
            let facts = workspace.facts_availability_async(profile).await;
            facts_phase.finish_result(&facts);
            facts?;
            let invariants_phase = lctx_native::phase::Phase::begin("artifact_invariant_admission");
            let invariants = workspace.execute_semantics(profile).await;
            invariants_phase.finish_result(&invariants);
            let invariants = invariants?;
            let frontier_phase = lctx_native::phase::Phase::begin("artifact_frontier_admission");
            let admitted_frontier = workspace.execute_frontier(frontier, profile).await;
            frontier_phase.finish_result(&admitted_frontier);
            let admitted_frontier = admitted_frontier?;
            Ok::<_, ModelError>((invariants, admitted_frontier))
        }
        .await;
        semantic_phase.finish_result(&semantic_result);
        let (semantic, frontier_evidence) = semantic_result?;
        workspace
            .native()
            .prepare_canonical(workspace.budget())
            .await?;
        let graph_phase = lctx_native::phase::Phase::begin("artifact_graph_admission");
        let graph_result = execute_graph_admission(workspace).await;
        graph_phase.finish_result(&graph_result);
        let checked_graph = graph_result?;
        let manifest_phase = lctx_native::phase::Phase::begin("artifact_manifest");
        let manifest_result = async {
            let (captures, original_keys, _inventory_charge) =
                original_inventory(workspace).await?;
            let originals = original_keys
                .iter()
                .map(|(_, original)| original.clone())
                .collect();
            let (manifest, mut manifest_charge) = crate::artifact_manifest::populate_with_origin(
                workspace,
                frontier,
                profile,
                settings,
                captures,
                originals,
                checked_graph.families.clone(),
            )
            .await?;
            manifest.validate()?;
            manifest_charge.grow(
                original_keys.len()
                    * (size_of::<Id<lctx_model::domain::source::SourceArtifact>>()
                        + size_of::<Original>()),
            )?;
            let admission = Arc::new(
                assemble_admission(
                    workspace,
                    semantic,
                    frontier_evidence,
                    checked_graph,
                    &manifest,
                )
                .await?,
            );
            let artifact = PreparedArtifactAdmission {
                original_keys,
                manifest,
                manifest_charge,
                admission,
            };
            Ok::<_, ModelError>(artifact)
        }
        .await;
        manifest_phase.finish_result(&manifest_result);
        manifest_result
    }
    .await
}

/// Admit one completed native authority. Portable files are produced only by explicit export.
pub async fn admit(
    workspace: &Arc<AdmissionRuntime>,
    frontier: lctx_model::domain::admission::Frontier,
    profile: lctx_model::domain::stages::Profile,
    settings: ContentHash,
) -> Result<AdmittedArtifact, ModelError> {
    let phase = lctx_native::phase::Phase::begin("artifact_admission");
    let result = async {
        let prepared = prepare_full_admission(
            workspace,
            frontier,
            profile,
            crate::artifact_manifest::SettingsOrigin::Acquisition(settings),
        )
        .await?;
        let native_admission = workspace
            .native()
            .admit_completed_state(&prepared.admission, workspace.budget())
            .await?;
        Ok::<_, ModelError>(AdmittedArtifact {
            workspace: workspace.clone(),
            original_keys: prepared.original_keys,
            manifest: prepared.manifest,
            _manifest_charge: prepared.manifest_charge,
            _admission: prepared.admission,
            _native_admission: native_admission,
        })
    }
    .await;
    phase.finish_result(&result);
    result
}
