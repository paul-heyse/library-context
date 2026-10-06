//! Explicit current-namespace inspection. No registry, history or routine content rehashing.
use lctx_model::domain::{ModelError, graph::{Manifest, semantic_contract}, serving::SnapshotHandle};
use lctx_surrealdb::{RuntimeConfig, NativeReader, Loader, reader};
use lctx_surrealdb::surrealdb::{Surreal, engine::remote::grpc::{Client, Grpc}, opt::auth::Root,
    types::{Bytes, SurrealValue, Value, SerdeWrapper}};
use std::sync::Arc;

async fn metadata(client:&Surreal<Client>,sql:&str)->Result<serde_json::Value,ModelError>{
    let mut result=client.query(sql).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let value:Value=result.take(0).map_err(ModelError::codec)?;
    SerdeWrapper::<serde_json::Value>::from_value(value).map(|v|v.0).map_err(ModelError::codec)
}

#[derive(SurrealValue)]
#[surreal(crate="lctx_surrealdb::surrealdb::types")]
struct Marker {handle:String,manifest:Bytes}

async fn marker(client:&Surreal<Client>)->Result<Option<(SnapshotHandle,Manifest)>,ModelError>{
    let info=metadata(client,"INFO FOR DB").await?;
    if !info.get("tables").and_then(serde_json::Value::as_object)
        .ok_or(ModelError::Schema("database table inventory"))?.contains_key("publication"){return Ok(None)}
    let mut result=client.query("SELECT handle,manifest FROM publication:current").await
        .map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let markers:Vec<Marker>=result.take(0).map_err(ModelError::codec)?;
    if markers.is_empty(){return Ok(None)}
    let row=markers.first().filter(|_|markers.len()==1).ok_or(ModelError::Schema("publication marker inventory"))?;
    let handle:SnapshotHandle=serde_json::from_slice(&hex::decode(&row.handle).map_err(ModelError::codec)?).map_err(ModelError::codec)?;
    if hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?)!=row.handle{return Err(ModelError::Conflict("canonical publication handle"))}
    let manifest:Manifest=serde_json::from_slice(&row.manifest).map_err(ModelError::codec)?;
    manifest.validate()?;
    if manifest.content()!=handle.semantic{return Err(ModelError::Conflict("publication semantic manifest"))}
    Ok(Some((handle,manifest)))
}

/// Enumerate surviving complete publications in the configured namespace, excluding control
/// storage and private attempts without a marker. This is a live listing, not an audit/history.
pub async fn list(config:&RuntimeConfig)->Result<Vec<SnapshotHandle>,ModelError>{
    let client=Surreal::new::<Grpc>(config.endpoint.strip_prefix("grpc://")
        .ok_or(ModelError::Schema("managed gRPC endpoint"))?).await.map_err(ModelError::codec)?;
    client.signin(Root{username:config.username.clone(),password:config.password.clone()}).await.map_err(ModelError::codec)?;
    // Avoid use_db entirely on the enumeration session, and do not create a missing namespace.
    let root=metadata(&client,"INFO FOR ROOT").await?;
    if !root.get("namespaces").and_then(serde_json::Value::as_object)
        .ok_or(ModelError::Schema("namespace inventory"))?.contains_key(config.namespace.as_str()){return Ok(vec![])}
    client.use_ns(config.namespace.as_str()).await.map_err(ModelError::codec)?;
    let namespace=metadata(&client,"INFO FOR NS").await?;
    let names=namespace.get("databases").and_then(serde_json::Value::as_object)
        .ok_or(ModelError::Schema("namespace database inventory"))?;
    let contract=semantic_contract(&lctx_model::domain::model()?);
    let mut handles=Vec::new();
    for database in names.keys().filter(|name|name.starts_with("snapshot_")&&name.as_str()!=config.cache_database.as_str()){
        // Every point has its own session; namespace selection never changes on a reader.
        let point=reader::connect(&config.endpoint,&config.root_credentials(),config.namespace.as_str(),database).await?;
        if let Some((handle,manifest))=marker(&point).await?{
            if handle.database.namespace!=config.namespace||handle.database.database.as_str()!=database{return Err(ModelError::Conflict("listed publication database"))}
            if manifest.semantic_contract!=contract{return Err(ModelError::Conflict("listed publication semantic contract"))}
            NativeReader::connect(&config.endpoint,&config.viewer_credentials(),handle.clone()).await?;
            handles.push(handle);
        }
    }
    Ok(handles)
}

/// A deliberate cold audit reuses the publication authorities. It does not replay semantic
/// producers, select a snapshot, install metadata or mutate canonical/derived content.
/// The shared effective inventory excludes users/access definitions and live subscriptions;
/// SurrealDB 3.3 INFO cannot report STRICT. Those properties are not certified by this audit.
pub async fn audit(config:&RuntimeConfig,handle:&SnapshotHandle,native_definitions:&str)->Result<(),ModelError>{
    if handle.database.namespace!=config.namespace{return Err(ModelError::Conflict("audit namespace"))}
    NativeReader::connect(&config.endpoint,&config.viewer_credentials(),handle.clone()).await?;
    let client=reader::connect(&config.endpoint,&config.root_credentials(),config.namespace.as_str(),handle.database.database.as_str()).await?;
    let (stored,manifest)=marker(&client).await?.ok_or(ModelError::Schema("audit publication marker"))?;
    if &stored!=handle{return Err(ModelError::Conflict("audit publication handle"))}
    if manifest.semantic_contract!=semantic_contract(&lctx_model::domain::model()?){return Err(ModelError::Conflict("audit semantic contract"))}
    let loader=Loader::new(Arc::clone(&client));
    loader.reconcile(&manifest).await?;
    if crate::verify_realization(&loader,native_definitions).await?!=handle.realization{return Err(ModelError::Conflict("audit current realization"))}
    Ok(())
}
