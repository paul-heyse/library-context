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
fn decode(row: Marker) -> Result<(SnapshotHandle, Manifest, Vec<CompletedBinding>), ModelError> {
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
    let mut vars = Variables::new();
    vars.insert("logical", logical.into_iter().collect::<Vec<_>>());
    let mut response=reader.client().query("SELECT VALUE descriptor FROM compiler_contribution WHERE completed=true AND logical IN $logical ORDER BY logical").bind(vars).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let descriptors: Vec<Bytes> = response.take(0).map_err(ModelError::codec)?;
    let mut exact = std::collections::BTreeMap::new();
    for bytes in descriptors {
        let contribution: CompletedContribution =
            serde_json::from_slice(&bytes).map_err(ModelError::codec)?;
        let identity = contribution.identity()?;
        if let Some(previous) = exact.insert(identity, (bytes.clone(), contribution)) {
            if previous.0 != bytes {
                return Err(ModelError::Conflict(
                    "logical contribution descriptor collision",
                ));
            }
        }
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
    let reader = NativeReader::connect(
        &config.endpoint,
        &config.writer_credentials(),
        handle.clone(),
    )
    .await?;
    let result = async {
        let (manifest, bindings) = marker(reader.client(), handle).await?;
        if manifest.semantic_contract != semantic_contract(&lctx_model::domain::model()?) {
            return Err(ModelError::Conflict("audit semantic contract"));
        }
        let compiler = lctx_surrealdb::compiler::NativeCompilerStore::from_publication(
            reader.shared_client(),
            config.namespace.clone(),
            config.database.clone(),
            bindings.clone(),
        )
        .await?;
        compiler.verify_state().await?;
        if compiler.completed_state().await? != manifest.completed_state {
            return Err(ModelError::Conflict("audit exact completed state"));
        }
        let loader = Loader::for_views(
            reader.shared_client(),
            bindings
                .iter()
                .filter(|binding| binding.boundary.is_none())
                .map(|binding| binding.view.identity)
                .collect(),
        );
        loader.reconcile(&manifest).await?;
        crate::search::reconcile_search(&loader).await?;
        if crate::verify_realization(&loader, native_definitions).await? != handle.realization {
            return Err(ModelError::Conflict("audit pinned realization"));
        }
        Ok(())
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step("audit reader pin release", reader.close().await);
    completion.step(
        "audit session invalidation",
        reader
            .client()
            .invalidate()
            .await
            .map_err(ModelError::codec),
    );
    lctx_model::domain::completion::complete(result, completion)
}
