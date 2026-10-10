//! Private publication workflow over nominal semantic admission owners.
use cpg_core::artifact::{AdmittedArtifact, RestoredAdmission, VerifiedExport};
use lctx_model::domain::{
    ContentHash, Key, KeySink, ModelError,
    completed::CompletedBinding,
    completion::{Completion, complete},
    graph::{Assertion, Entity, Manifest},
    serving::{DatabaseIdentity, SnapshotHandle},
};
use lctx_surrealdb::surrealdb::types::{Bytes, Object, RecordId, Value};
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
    bindings: Vec<CompletedBinding>,
}
impl<'a> Publication<'a> {
    pub(crate) async fn new(
        admission: Admission<'a>,
        config: &RuntimeConfig,
    ) -> Result<Self, ModelError> {
        admission.native().check_publication_target(config)?;
        let bindings = admission.native().bindings().await?;
        let lease = admission
            .native()
            .begin_derived_operation("publication session")?;
        let connected = lctx_surrealdb::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            admission.native().namespace().as_str(),
            admission.native().database().as_str(),
        )
        .await;
        let client = lease.finish_with(connected)?;
        let attempt=admission.native().attempt();
        Ok(Self {
            admission,
            loader: Loader::for_attempt_views(client,attempt,bindings.iter().filter(|binding|binding.boundary.is_none()).map(|binding|binding.view.identity).collect()),
            bindings,
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

    /// Verify the selected immutable executable epoch; ordinary publication performs no DDL.
    pub(crate) async fn install_definitions(&self, definitions: &str) -> Result<(), ModelError> {
        lctx_surrealdb::materialization::validate_native_definitions(definitions)?;
        let lease = self.store().begin_derived_operation("publication definitions")?;
        let result = crate::definitions::verify_epoch(&self.loader, definitions).await.map(|_|());
        lease.finish_with(result)
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
                &self.bindings,
                self.store().attempt(),
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
    database: &lctx_model::domain::serving::Name,
    manifest: &Manifest,
    views: &[CompletedBinding],
    attempt: ContentHash,
    config: &RuntimeConfig,
    native_definitions: &str,
    lease: &lctx_surrealdb::compiler::OperationLease,
) -> Result<SnapshotHandle, ModelError> {
    let client = loader.client();
    let realization_phase = Phase::begin("publication_realization");
    let realization =
        crate::definitions::verify_epoch(loader, native_definitions).await;
    realization_phase.finish_result(&realization);
    let realization = realization?;
    let definition_epoch = crate::definitions::epoch_identity(native_definitions);
    let view = view_identity(views)?;
    let mut publication = KeySink::new("native-publication/v2");
    manifest.content().encode(&mut publication);
    realization.encode(&mut publication);
    view.encode(&mut publication);
    config.service_generation.encode(&mut publication);
    definition_epoch.encode(&mut publication);
    let handle = SnapshotHandle {
        publication: publication.finish(),
        view,
        service_generation: config.service_generation,
        definition_epoch,
        semantic: manifest.content(),
        realization,
        database: DatabaseIdentity {
            namespace: config.namespace.clone(),
            database: database.clone(),
        },
    };
    let mut marker = Object::new();
    marker.insert("id", RecordId::new("publication", handle.publication.hex()));
    marker.insert(
        "handle",
        hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?),
    );
    marker.insert(
        "manifest",
        Bytes::from(serde_json::to_vec(manifest).map_err(ModelError::codec)?),
    );
    marker.insert("views", Bytes::from(serde_json::to_vec(views).map_err(ModelError::codec)?));
    marker.insert("definition_epoch", definition_epoch.hex());
    // Prepare the committed identity before the effect, so later local encoding cannot lose it.
    let committed_identity = serde_json::to_string(&handle).map_err(ModelError::codec)?;
    let marker_phase = Phase::begin("publication_marker");
    // Establish guarded reachability before the manifest can become visible. If the
    // acknowledgement is unknown, the durable native operation retains reconciliation work.
    lctx_surrealdb::control::hold(client, Some(attempt), RecordId::new("publication", handle.publication.hex()), views.iter().map(|binding|RecordId::new("compiler_view",binding.view.identity.hex())).collect()).await?;
    // Transfer every retained attempt root before closing its mutable owner. Bounded
    // windows include originals, roles and derived search records, alongside exact views.
    let mut variables=lctx_surrealdb::surrealdb::types::Variables::new();variables.insert("attempt",RecordId::new("native_attempt",attempt.hex()));
    let retained_reader=loader.reader();
    let mut roots=retained_reader.query_stream("SELECT VALUE object FROM native_hold WHERE owner=$attempt ORDER BY id",variables,1)?;
    let transfer=async {
        let mut window=Vec::new();
        while let Some(root)=roots.next().await? {
            let Value::RecordId(root)=root else{return Err(ModelError::Schema("publication retained root"));};
            window.push(root);
            if window.len()==128{lctx_surrealdb::control::hold(client,Some(attempt),RecordId::new("publication",handle.publication.hex()),std::mem::take(&mut window)).await?;}
        }
        lctx_surrealdb::control::hold(client,Some(attempt),RecordId::new("publication",handle.publication.hex()),window).await
    }.await;
    let mut root_finality=Completion::default();root_finality.step("publication retained-root stream drainage",roots.drain_transport().await);
    complete(transfer,root_finality)?;
    let marking = lctx_surrealdb::control::ensure_rows(client, Some(attempt), vec![Value::Object(marker)]).await;
    marker_phase.finish_result(&marking);
    marking?;
    lease.record_committed(committed_identity.clone());
    lctx_surrealdb::control::close_attempt(client, attempt, "closed").await?;
    // The marker is committed. Session/readback failures must retain that effect exactly.
    let mut completion = Completion::default();
    completion.committed("published unselected manifest", committed_identity);
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
        &config.writer_credentials(),
        handle.clone(),
    )
    .await;
    let readback = match readback {
        Ok(reader)=>{
            let closed=reader.close().await;
            let mut finality=Completion::default();finality.step("publication readback session invalidation",reader.client().invalidate().await.map_err(ModelError::codec));
            complete(closed.map(|_|handle),finality)
        },
        Err(error)=>Err(error),
    };
    let result = complete(readback, completion);
    readback_phase.finish_result(&result);
    result
}

/// The publication freezes this exact binding inventory, not the whole database.
pub(crate) fn view_identity(bindings:&[CompletedBinding])->Result<ContentHash,ModelError>{
    lctx_model::domain::completed::binding_inventory_identity(bindings)
}
