//! Private publication workflow over nominal semantic admission owners.
use cpg_core::artifact::{AdmittedArtifact, RestoredAdmission, VerifiedExport};
use lctx_model::domain::{
    ModelError,
    completion::{Completion, complete},
    graph::{Assertion, Entity, Manifest},
    serving::{DatabaseIdentity, Name, SnapshotHandle},
};
use lctx_surrealdb::surrealdb::types::{Bytes, Object, RecordId, ToSql, Value, Variables};
use lctx_surrealdb::{
    Loader, NativeReader, RuntimeConfig, compiler::NativeCompilerStore, phase::Phase,
};
use std::sync::Arc;

/// Only semantic admission owners can enter the private publication workflow.
pub(crate) enum Admission<'a> {
    Completed(&'a AdmittedArtifact),
    Export(&'a VerifiedExport),
    Restored(&'a RestoredAdmission),
}
impl Admission<'_> {
    fn native(&self) -> &Arc<NativeCompilerStore> {
        match self {
            Self::Completed(owner) => owner.native(),
            Self::Export(owner) => owner.native(),
            Self::Restored(owner) => owner.native(),
        }
    }
    fn manifest(&self) -> &Manifest {
        match self {
            Self::Completed(owner) => owner.manifest(),
            Self::Export(owner) => owner.manifest(),
            Self::Restored(owner) => owner.manifest(),
        }
    }
}

/// One private authenticated session, scoped to a nominally admitted owner. No client escapes.
pub(crate) struct Publication<'a> {
    admission: Admission<'a>,
    loader: Loader,
}
impl<'a> Publication<'a> {
    pub(crate) async fn new(
        admission: Admission<'a>,
        config: &RuntimeConfig,
    ) -> Result<Self, ModelError> {
        admission.native().check_publication_target(config)?;
        let lease = admission
            .native()
            .begin_derived_operation("publication session")?;
        let connected = lctx_surrealdb::reader::connect(
            &config.endpoint,
            &config.root_credentials(),
            admission.native().namespace().as_str(),
            admission.native().database().as_str(),
        )
        .await;
        let client = lease.finish_with(connected)?;
        Ok(Self {
            admission,
            loader: Loader::new(client),
        })
    }
    fn store(&self) -> &Arc<NativeCompilerStore> {
        self.admission.native()
    }
    pub(crate) async fn invalidate(&self) -> Result<(), ModelError> {
        self.loader
            .client()
            .invalidate()
            .await
            .map_err(|error| ModelError::Cause(Box::new(error)))
    }

    /// Install the selected executable blueprint without reopening canonical content writes.
    pub(crate) async fn install_definitions(&self, definitions: &str) -> Result<(), ModelError> {
        lctx_surrealdb::materialization::validate_native_definitions(definitions)?;
        let lease = self
            .store()
            .begin_derived_operation("publication definitions")?;
        let phase = Phase::begin("publication_definitions");
        let result = self
            .loader
            .client()
            .query(definitions)
            .await
            .map_err(lctx_surrealdb::loader::write_failure)
            .and_then(|response| response.check().map(|_| ()).map_err(ModelError::codec));
        let result = lease.finish_with(result);
        phase.finish_result(&result);
        result
    }

    pub(crate) async fn entity_references(&self, entities: &[Entity]) -> Result<(), ModelError> {
        let lease = self
            .store()
            .begin_derived_operation("publication entity references")?;
        let result = self.loader.entity_references(entities).await;
        lease.finish_with(result)
    }

    pub(crate) async fn assertion_references(
        &self,
        assertions: &[Assertion],
    ) -> Result<(), ModelError> {
        let lease = self
            .store()
            .begin_derived_operation("publication assertion references")?;
        let result = self.loader.assertion_references(assertions).await;
        lease.finish_with(result)
    }

    pub(crate) async fn materialize_search(&self) -> Result<(), ModelError> {
        let lease = self.store().begin_derived_operation("publication search")?;
        let phase = Phase::begin("publication_search");
        let result = lctx_surrealdb::derived_search::materialize_search(&self.loader).await;
        let result = lease.finish_with(result);
        phase.finish_result(&result);
        result
    }

    /// Close and drain every ordinary operation, then retain the one private seal's terminality.
    pub(crate) async fn seal(
        &self,
        config: &RuntimeConfig,
        definitions: &str,
    ) -> Result<SnapshotHandle, ModelError> {
        lctx_surrealdb::materialization::validate_native_definitions(definitions)?;
        let manifest = self.admission.manifest();
        manifest.validate()?;
        self.store().check_publication_target(config)?;
        // Drainage can be resumed if its waiter is interrupted. Once issued, the private
        // seal lease owns all final state/graph/search reads and marker effects.
        let lease = self.store().begin_finalization().await?;
        let phase = Phase::begin("publication_seal");
        let result = async {
            if self.store().completed_state_after_closure(&lease).await? != manifest.completed_state
            {
                return Err(ModelError::Conflict("sealed completed state"));
            }
            let loader = &self.loader;
            let reconciliation_phase = Phase::begin("publication_final_reconciliation");
            let reconciled = async {
                loader.reconcile(manifest).await?;
                lctx_surrealdb::derived_search::reconcile_search(loader).await
            }
            .await;
            reconciliation_phase.finish_result(&reconciled);
            reconciled?;
            seal_staging(
                loader,
                self.store().database(),
                manifest,
                config,
                definitions,
                &lease,
            )
            .await
        }
        .await;
        let result = lease.finish_with(result);
        phase.finish_result(&result);
        result
    }
}

/// Marker authority stays inside this admitted workflow and its retained finalization lease.
async fn seal_staging(
    loader: &Loader,
    database: &Name,
    manifest: &Manifest,
    config: &RuntimeConfig,
    native_definitions: &str,
    lease: &lctx_surrealdb::compiler::OperationLease,
) -> Result<SnapshotHandle, ModelError> {
    let client = loader.client();
    let realization_phase = Phase::begin("publication_realization");
    let realization =
        lctx_surrealdb::realization::verify_realization(loader, native_definitions).await;
    realization_phase.finish_result(&realization);
    let realization = realization?;
    let handle = SnapshotHandle {
        semantic: manifest.content(),
        realization,
        database: DatabaseIdentity {
            namespace: config.namespace.clone(),
            database: database.clone(),
        },
    };
    let username = Name::new(config.viewer_username.clone()).map_err(ModelError::codec)?;
    // DEFINE USER requires a literal strand at 3.3; preserve the SDK SQL string codec.
    let password = Value::String(config.viewer_password.clone()).to_sql();
    let viewer_phase = Phase::begin("publication_viewer");
    let viewer_result = client
        .query(format!(
            "DEFINE USER `{}` ON DATABASE PASSWORD {password} ROLES VIEWER",
            username.as_str()
        ))
        .await
        .map_err(lctx_surrealdb::loader::write_failure)
        .and_then(|response| {
            response
                .check()
                .map(|_| ())
                .map_err(|_| ModelError::Invalid("native viewer definition failed".into()))
        });
    viewer_phase.finish_result(&viewer_result);
    viewer_result?;
    let mut marker = Object::new();
    marker.insert("id", RecordId::new("publication", "current"));
    marker.insert(
        "handle",
        hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?),
    );
    marker.insert(
        "manifest",
        Bytes::from(serde_json::to_vec(manifest).map_err(ModelError::codec)?),
    );
    let mut bindings = Variables::new();
    bindings.insert("marker", marker);
    // Prepare the committed identity before the effect, so later local encoding cannot lose it.
    let committed_identity = serde_json::to_string(&handle).map_err(ModelError::codec)?;
    let marker_phase = Phase::begin("publication_marker");
    let marking = client
        .query("INSERT INTO publication $marker RETURN NONE")
        .bind(bindings)
        .await
        .map_err(lctx_surrealdb::loader::write_failure)
        .and_then(|response| response.check().map(|_| ()).map_err(ModelError::codec));
    marker_phase.finish_result(&marking);
    marking?;
    lease.record_committed(committed_identity.clone());
    // The marker is committed. Session/readback failures must retain that effect exactly.
    let mut completion = Completion::default();
    completion.committed("sealed unselected database", committed_identity);
    let readback_phase = Phase::begin("publication_readback");
    completion.step(
        "publication session invalidation",
        client
            .invalidate()
            .await
            .map_err(|error| ModelError::Cause(Box::new(error))),
    );
    let readback = NativeReader::connect(
        &config.endpoint,
        &config.viewer_credentials(),
        handle.clone(),
    )
    .await
    .map(|_| handle);
    let result = complete(readback, completion);
    readback_phase.finish_result(&result);
    result
}
