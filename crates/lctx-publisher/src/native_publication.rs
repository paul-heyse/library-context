//! Private publication workflow over nominal semantic admission owners.
use lctx_compiler_store::compiler::NativeCompilerStore;
use lctx_compiler_store::artifact::{AdmittedArtifact, RestoredAdmission, VerifiedExport};
use lctx_model::domain::{
    ContentHash, Key, KeySink, ModelError,
    completed::CompletedBinding,
    completion::{Completion, complete},
    graph::{Assertion, Entity, Manifest},
    serving::{DatabaseIdentity, SnapshotHandle},
};
use lctx_native::{
    Loader, NativeReader, RuntimeConfig,  phase::Phase,
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
    fn budget(&self) -> &lctx_model::domain::resources::ResourceBudget {
        match self { Self::Completed(owner) => owner.budget(), Self::Export(owner) => owner.budget(), Self::Restored(owner) => owner.budget() }
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
    unchanged: Option<SnapshotHandle>,
    definition_epoch: ContentHash,
    session:lctx_native::reader::SessionFinalizationGuard,
}
impl<'a> Publication<'a> {
    pub(crate) async fn new(
        admission: Admission<'a>,
        config: &RuntimeConfig,
        definitions:&str,
    ) -> Result<Self, ModelError> {
        admission.native().check_publication_target(config)?;
        let bindings = canonical_bindings(admission.native().bindings().await?)?;
        let lease = admission
            .native()
            .begin_derived_operation("publication session")?;
        let connected = lctx_native::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            admission.native().namespace().as_str(),
            admission.native().database().as_str(),
        )
        .await;
        let client = match connected {Ok(client)=>client,Err(error)=>return lease.finish_with(Err(error))};
        let mut session=Some(lctx_native::reader::SessionFinalizationGuard::new(client.clone()));
        let result:Result<Self,ModelError>=async {
            lctx_native::materialization::validate_native_definitions(definitions)?;
            let attempt=admission.native().attempt();let budget=admission.budget();
            let loader=Loader::for_attempt_views(client.clone(),attempt,bindings.iter()
                .filter(|binding|binding.boundary.is_none()).map(|binding|binding.view.identity).collect()).with_budget(budget);
            let realization=crate::definitions::verify_epoch(&loader,definitions).await?;
            let loader=loader.with_derivation(realization);
            let handle=publication_handle(admission.manifest(),&bindings,admission.native().database(),config,realization,crate::definitions::epoch_identity(definitions))?;
            let unchanged=if matches!(&admission,Admission::Completed(_)) {
                lctx_native::control::staging::find_existing_publication(&loader,&handle,admission.manifest(),&bindings,budget).await?.map(|marker|marker.handle)
            }else{None};
            let loader=if unchanged.is_some(){loader}else{
                let completed=lctx_native::completed_state::hold_admitted_completed_state_for_attempt(client.clone(),attempt,budget).await?;
                let staging=lctx_native::control::staging::DerivedStaging::begin(&client,completed,realization,budget).await?;
                loader.with_staging(Arc::new(staging))?.prepare_completed_reader().await?
            };
            Ok(Self{admission,loader,bindings,unchanged,definition_epoch:crate::definitions::epoch_identity(definitions),session:session.take().expect("publication session owner")})
        }.await;
        if let Err(error)=&result {
            if !error.permits_storage_cleanup(){return lease.finish_with(result);}
            let mut completion=Completion::default();
            completion.step("failed publication setup session invalidation",session.as_ref().expect("failed setup session owner").close().await);
            return lease.finish_with(complete(result,completion));
        }
        lease.finish_with(result)
    }
    pub(crate) fn is_unchanged(&self)->bool{self.unchanged.is_some()}
    pub(crate) fn loader(&self) -> &Loader { &self.loader }
    fn store(&self) -> &Arc<NativeCompilerStore> {
        self.admission.native()
    }
    pub(crate) async fn invalidate(&self,primary:&ModelError)->Result<(),ModelError>{
        if !primary.permits_storage_cleanup(){return Err(ModelError::infrastructure(lctx_model::domain::Infrastructure::Unconfirmed,"publication session retained after uncertain operation"));}
        self.loader.close_reader().await?;
        self.session.close().await
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
        let result = lctx_native::derived_search::materialize_search(&self.loader).await;
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
        lctx_native::materialization::validate_native_definitions(definitions)?;
        if crate::definitions::epoch_identity(definitions)!=self.definition_epoch{return Err(ModelError::Conflict("publication executable epoch changed"));}
        let manifest = self.admission.manifest();
        manifest.validate()?;
        self.store().check_publication_target(config)?;
        // Drainage can be resumed if its waiter is interrupted. Once issued, the private
        // seal lease owns all final state/graph/search reads and marker effects.
        let lease = self.store().begin_finalization().await?;
        let phase = Phase::begin("publication_seal");
        let result = async {
            let expected=publication_handle(manifest,&self.bindings,self.store().database(),config,self.loader.derivation()?,self.definition_epoch)?;
            // Encode before the acknowledgement so a later local error cannot lose it.
            let committed_identity=serde_json::to_string(&expected).map_err(ModelError::codec)?;
            let handle=if let Some(handle)=&self.unchanged {
                if handle!=&expected{return Err(ModelError::Conflict("unchanged publication complete identity"));}
                let checked=lctx_native::control::staging::find_existing_publication(&self.loader,handle,manifest,&self.bindings,self.admission.budget()).await?
                    .ok_or(ModelError::Conflict("unchanged publication disappeared"))?;
                checked.handle
            }else{
                let staging=self.loader.staging().ok_or(ModelError::Conflict("publication staging owner missing"))?;
                staging.publish(&self.loader,&expected,manifest,&self.bindings,self.admission.budget()).await?.handle
            };
            finalize_marker(&self.loader,&handle,self.store().attempt(),config,&lease,committed_identity,&self.session).await
        }
        .await;
        let result = lease.finish_with(result);
        phase.finish_result(&result);
        result
    }
}

pub(crate) fn publication_handle(manifest:&Manifest,views:&[CompletedBinding],database:&lctx_model::domain::serving::Name,
    config:&RuntimeConfig,realization:ContentHash,definition_epoch:ContentHash)->Result<SnapshotHandle,ModelError>{
    let view=view_identity(views)?;
    Ok(SnapshotHandle{publication:publication_identity(manifest.content(),realization,view,config.service_generation,definition_epoch),
        view,service_generation:config.service_generation,definition_epoch,semantic:manifest.content(),realization,
        database:DatabaseIdentity{namespace:config.namespace.clone(),database:database.clone()}})
}
/// Marker acknowledgement precedes every fallible finalizer and is retained in both owners.
async fn finalize_marker(loader:&Loader,handle:&SnapshotHandle,attempt:ContentHash,config:&RuntimeConfig,
    lease:&lctx_compiler_store::compiler::OperationLease,committed_identity:String,session:&lctx_native::reader::SessionFinalizationGuard)->Result<SnapshotHandle,ModelError>{
    let client=loader.client();
    lease.record_committed(committed_identity.clone());
    let mut completion = Completion::default();
    completion.committed("published unselected manifest", committed_identity);
    let reader_closed=loader.close_reader().await;
    let may_finalize=reader_closed.is_ok();
    completion.step("publication completed reader close",reader_closed);
    if !may_finalize { return complete(Ok(handle.clone()),completion); }
    if let Err(error) = lctx_native::control::close_attempt(client, attempt, "closed").await {
        return complete(Err(error), completion);
    }
    let readback_phase = Phase::begin("publication_readback");
    completion.step(
        "publication session invalidation",
        session.close().await,
    );
    let readback = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    )
    .await;
    let readback = match readback {
        Ok(reader) => {
            let mut owner=reader.protect_terminal_close();
            owner.close(&reader).await.map(|_|handle.clone())
        }
        Err(error) => Err(error),
    };
    let result = complete(readback, completion);
    readback_phase.finish_result(&result);
    result
}

/// The publication freezes this exact binding inventory, not the whole database.
pub(crate) fn view_identity(bindings: &[CompletedBinding]) -> Result<ContentHash, ModelError> {
    lctx_model::domain::completed::binding_inventory_identity(bindings)
}

/// Logical binding order owns both the immutable marker bytes and its scoped loader inputs.
/// Attempt-qualified physical binding addresses are transport details and may reorder on restore.
fn canonical_bindings(
    mut bindings: Vec<CompletedBinding>,
) -> Result<Vec<CompletedBinding>, ModelError> {
    bindings.sort_by_key(CompletedBinding::key);
    view_identity(&bindings)?;
    Ok(bindings)
}

fn publication_identity(
    semantic: ContentHash,
    realization: ContentHash,
    view: ContentHash,
    generation: ContentHash,
    definition_epoch: ContentHash,
) -> ContentHash {
    let mut publication = KeySink::new("native-publication/v2");
    semantic.encode(&mut publication);
    realization.encode(&mut publication);
    view.encode(&mut publication);
    generation.encode(&mut publication);
    definition_epoch.encode(&mut publication);
    publication.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::{
        Record, Relation, analysis::sources::SourceSnapshot, completed::CompletedView,
        input::Package,
    };
    use std::collections::BTreeSet;

    fn binding(boundary: Option<&str>) -> CompletedBinding {
        let relation = Relation::of::<Package>();
        let view = CompletedView::new(
            Package::NAME.into(),
            BTreeSet::from([ContentHash::of(b"contributor")]),
            0,
        )
        .unwrap();
        CompletedBinding {
            boundary: boundary.map(str::to_owned),
            source: SourceSnapshot::of_completed_view(&relation, ContentHash::of(b"model"), &view)
                .unwrap(),
            view,
            configuration: None,
        }
    }

    #[test]
    fn restored_binding_order_preserves_immutable_marker_bytes_and_handle() {
        let first = binding(None);
        let second = binding(Some("facts"));
        let original = vec![first.clone(), second.clone()];
        let reversed = vec![second, first];
        assert_ne!(
            serde_json::to_vec(&original).unwrap(),
            serde_json::to_vec(&reversed).unwrap()
        );
        assert_eq!(
            view_identity(&original).unwrap(),
            view_identity(&reversed).unwrap()
        );
        let original = canonical_bindings(original).unwrap();
        let restored = canonical_bindings(reversed).unwrap();
        // These are the exact bytes persisted in publication.views by seal_staging.
        assert_eq!(
            serde_json::to_vec(&original).unwrap(),
            serde_json::to_vec(&restored).unwrap()
        );
        let make_handle = |bindings: &[CompletedBinding]| {
            let semantic = ContentHash::of(b"semantic");
            let realization = ContentHash::of(b"realization");
            let service_generation = ContentHash::of(b"generation");
            let definition_epoch = ContentHash::of(b"epoch");
            let view = view_identity(bindings).unwrap();
            SnapshotHandle {
                publication: publication_identity(
                    semantic,
                    realization,
                    view,
                    service_generation,
                    definition_epoch,
                ),
                view,
                semantic,
                realization,
                service_generation,
                definition_epoch,
                database: DatabaseIdentity {
                    namespace: lctx_model::domain::serving::Name::new("library_context").unwrap(),
                    database: lctx_model::domain::serving::Name::new("validation").unwrap(),
                },
            }
        };
        assert_eq!(make_handle(&original), make_handle(&restored));
    }

    #[test]
    fn canonical_binding_capture_refuses_duplicate_or_invalid_inventory() {
        let valid = binding(None);
        assert!(matches!(
            canonical_bindings(vec![valid.clone(), valid.clone()]),
            Err(ModelError::Conflict("duplicate publication binding")),
        ));
        let mut invalid = valid;
        invalid.view.rows += 1;
        assert!(canonical_bindings(vec![invalid]).is_err());
    }
}
