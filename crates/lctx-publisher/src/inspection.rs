//! Exact publication inspection. A database can hold many unrelated immutable publications.
use lctx_model::domain::{
    ModelError,
    completed::{CompletedBinding, CompletedContribution, CompletedView},
    graph::{Manifest, semantic_contract},
    serving::SnapshotHandle,
};
use lctx_surrealdb::{
    Loader, NativeReader, RuntimeConfig, reader,
    surrealdb::{
        Surreal,
        engine::remote::grpc::Client,
        types::{Bytes, RecordId, SurrealValue, Variables},
    },
};

#[derive(serde::Serialize)]
pub struct SnapshotDetails {
    pub handle: SnapshotHandle,
    pub manifest: Manifest,
    pub contributions: Vec<CompletedContribution>,
    pub views: Vec<CompletedView>,
    pub bindings: Vec<CompletedBinding>,
}
#[derive(SurrealValue)]
#[surreal(crate = "lctx_surrealdb::surrealdb::types")]
pub(crate) struct Marker {
    pub handle: String,
    pub manifest: Bytes,
    pub views: Bytes,
}
pub(crate) fn decode(row: Marker) -> Result<(SnapshotHandle, Manifest, Vec<CompletedBinding>), ModelError> {
    let handle: SnapshotHandle =
        serde_json::from_slice(&hex::decode(&row.handle).map_err(ModelError::codec)?)
            .map_err(ModelError::codec)?;
    if hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?) != row.handle {
        return Err(ModelError::Conflict("canonical publication handle"));
    }
    handle.validate_identity()?;
    let manifest = Manifest::decode(&row.manifest)?;
    let views: Vec<CompletedBinding> =
        serde_json::from_slice(&row.views).map_err(ModelError::codec)?;
    if manifest.content() != handle.semantic
        || crate::native_publication::view_identity(&views)? != handle.view
    {
        return Err(ModelError::Conflict("publication exact manifest/view"));
    }
    Ok((handle, manifest, views))
}
pub(crate) async fn marker(
    client: &Surreal<Client>,
    handle: &SnapshotHandle,
) -> Result<(Manifest, Vec<CompletedBinding>), ModelError> {
    let mut vars = Variables::new();
    vars.insert(
        "publication",
        RecordId::new("publication", handle.publication.hex()),
    );
    let mut response = client
        .query("SELECT handle,manifest,views FROM $publication")
        .bind(vars)
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let mut rows: Vec<Marker> = response.take(0).map_err(ModelError::codec)?;
    if rows.len() != 1 {
        return Err(ModelError::Schema("exact publication inventory"));
    }
    let (actual, manifest, views) = decode(rows.remove(0))?;
    if &actual != handle {
        return Err(ModelError::Conflict("pinned publication handle"));
    }
    Ok((manifest, views))
}
pub async fn show(reader: &NativeReader) -> Result<SnapshotDetails, ModelError> {
    let handle = reader.handle().clone();
    let (manifest, bindings) = marker(reader.client(), &handle).await?;
    let logical = bindings
        .iter()
        .flat_map(|binding| binding.view.contributions.iter().map(|id| id.hex()))
        .collect::<std::collections::BTreeSet<_>>();
    let mut exact = std::collections::BTreeMap::new();
    for logical in logical {
        let mut vars = Variables::new(); vars.insert("logical", logical);
        let mut rows = reader.stream_prepared(lctx_surrealdb::prepared::PreparedQuery::new(vars, vec![], vec!["SELECT descriptor FROM compiler_contribution WITH INDEX logical_contribution WHERE completed=true AND logical=$logical ORDER BY id".into()])?)?;
        let result = async {
            while let Some(value) = rows.next().await? {
                let lctx_surrealdb::surrealdb::types::Value::Object(row) = value else { return Err(ModelError::Schema("inspection contributor object")); };
                let Some(lctx_surrealdb::surrealdb::types::Value::Bytes(bytes)) = row.get("descriptor") else { return Err(ModelError::Schema("inspection contributor descriptor")); };
                let contribution: CompletedContribution = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
                let identity = contribution.identity()?;
                if let Some(previous) = exact.insert(identity, (bytes.clone(), contribution)) {
                    if previous.0 != *bytes { return Err(ModelError::Conflict("logical contribution descriptor collision")); }
                }
            }
            Ok(())
        }.await;
        let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("inspection contributor drainage", rows.drain_transport().await); lctx_model::domain::completion::complete(result, completion)?;
    }
    let contributions = exact
        .into_values()
        .map(|(_, contribution)| contribution)
        .collect();
    let views = bindings
        .iter()
        .map(|binding| binding.view.clone())
        .collect();
    Ok(SnapshotDetails {
        handle,
        manifest,
        contributions,
        views,
        bindings,
    })
}
/// List immutable manifests inside the installed database; no namespace/database enumeration.
pub async fn list(config: &RuntimeConfig) -> Result<Vec<SnapshotHandle>, ModelError> {
    let client = reader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
    )
    .await?;
    lctx_surrealdb::control::check_installation(&client, config.service_generation).await?;
    let result = async {
        let mut response = client
            .query("SELECT handle,manifest,views FROM publication ORDER BY id")
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Marker> = response.take(0).map_err(ModelError::codec)?;
        let mut result = Vec::with_capacity(rows.len());
        let contract = semantic_contract(&lctx_model::domain::model()?);
        for row in rows {
            let (handle, manifest, _) = decode(row)?;
            if handle.database.namespace != config.namespace
                || handle.database.database != config.database
                || handle.service_generation != config.service_generation
                || manifest.semantic_contract != contract
            {
                return Err(ModelError::Conflict("listed publication realization"));
            }
            result.push(handle);
        }
        Ok(result)
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "publication listing session invalidation",
        client.invalidate().await.map_err(ModelError::codec),
    );
    lctx_model::domain::completion::complete(result, completion)
}
pub async fn audit(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    native_definitions: &str,
) -> Result<(), ModelError> {
    if handle.database.namespace != config.namespace
        || handle.database.database != config.database
        || handle.service_generation != config.service_generation
    {
        return Err(ModelError::Conflict("audit installation identity"));
    }
    // An owned driver retains the pin/session through caller cancellation. Dropping the
    // JoinHandle transfers delivery only; the driver still joins every native tail and reports
    // cleanup uncertainty before releasing authority.
    let config = config.clone();
    let handle = handle.clone();
    let native_definitions = native_definitions.to_owned();
    crate::owned_read::run("publication audit", move |cancel| async move { audit_owned(&config, &handle, &native_definitions, &cancel).await }).await
}
async fn audit_owned(
    config: &RuntimeConfig,
    handle: &SnapshotHandle,
    native_definitions: &str,
    cancel: &crate::owned_read::Cancellation,
) -> Result<(), ModelError> {
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(cpg_core::workspace::WorkspaceOptions::default().memory_bytes)?;
    let reader = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    ).await?.with_budget(&budget).with_read_cancellation(cancel.flag());
    let mut pin_guard = reader.protect_terminal_close();
    let mut owner = None;
    let result = async {
        cancel.check()?;
        let (manifest, bindings) = marker(reader.client(), handle).await?;
        if manifest.semantic_contract != semantic_contract(&lctx_model::domain::model()?) {
            return Err(ModelError::Conflict("audit semantic contract"));
        }
        let compiler = lctx_surrealdb::compiler::NativeCompilerStore::publication_owner(
            reader.shared_client(), config.namespace.clone(), config.database.clone(), bindings.clone(),
        )?;
        compiler.set_read_cancellation(cancel.flag())?;
        owner = Some(compiler.clone());
        let capture = compiler.capture_audit(&budget).await?;
        cancel.check()?;
        compiler.verify_captured(&capture).await?;
        cancel.check()?;
        if compiler.completed_state_captured(&capture).await? != manifest.completed_state {
            return Err(ModelError::Conflict("audit exact completed state"));
        }
        let loader = Loader::for_views(
            reader.shared_client(),
            bindings.iter().filter(|binding| binding.boundary.is_none())
                .map(|binding| binding.view.identity).collect(),
        ).with_budget(&budget).with_read_cancellation(cancel.flag());
        cancel.check()?;
        loader.reconcile(&manifest).await?;
        cancel.check()?;
        crate::search::reconcile_search(&loader).await?;
        cancel.check()?;
        if crate::verify_realization(&loader, native_definitions).await? != handle.realization {
            return Err(ModelError::Conflict("audit pinned realization"));
        }
        Ok(())
    }.await;
    let mut completion = if let Some(owner) = owner {
        owner.drain_report().await
    } else {
        lctx_model::domain::completion::Completion::default()
    };
    if result.as_ref().err().is_some_and(|error| !error.permits_storage_cleanup()) || completion.local != lctx_model::domain::completion::LocalState::Terminal || completion.remote != lctx_model::domain::completion::RemoteState::Confirmed { reader.retain_unknown(); }
    completion.step("audit reader pin release", pin_guard.close(&reader).await);
    completion.step("audit session invalidation", reader.client().invalidate().await.map_err(ModelError::codec));
    lctx_model::domain::completion::complete(result, completion)
}
