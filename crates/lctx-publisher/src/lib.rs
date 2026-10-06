//! Publication consumes trusted verified compiler exports, without replaying producers.
use cpg_core::artifact::VerifiedExport;
use lctx_model::domain::{Key,KeySink,ModelError,serving::{SnapshotHandle,DatabaseIdentity,Name}};
use lctx_surrealdb::{RuntimeConfig,Loader,NativeReader,reader};
use lctx_surrealdb::surrealdb::types::{Object,Variables,Bytes,RecordId};
pub async fn publish(export:&VerifiedExport,config:&RuntimeConfig,native_definitions:&str)->Result<SnapshotHandle,ModelError>{
    let mut identity=KeySink::new("native-database-attempt/v1");identity.part(b"clock",&std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(ModelError::codec)?.as_nanos().to_le_bytes());identity.part(b"process",&std::process::id().to_le_bytes());
    let database=format!("snapshot_{}",identity.finish().hex());
    let client=reader::connect(&config.endpoint,&config.root_credentials(),config.namespace.as_str(),&database).await?;
    let version=client.version().await.map_err(ModelError::codec)?.to_string();
    if !version.starts_with("3.3."){return Err(ModelError::Invalid("native realization requires reviewed SurrealDB 3.3 engine".into()))}
    client.query(format!("DEFINE NAMESPACE IF NOT EXISTS `{}`; DEFINE DATABASE OVERWRITE `{database}` STRICT;",config.namespace.as_str())).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let loader=Loader::new(client.clone());
    let result=load(export,&loader,native_definitions).await;
    if let Err(error)=result{
        // All ordinary submitted writes have completed. Uncertain transport failure still leaves
        // only an unreferenced private database; a successful cleanup is not a publication.
        let _=client.query(format!("REMOVE DATABASE `{database}`")).await;return Err(error)
    }
    let mut realized=KeySink::new("native-realization/v1");realized.part(b"engine",version.as_bytes());lctx_surrealdb::schema::realization_identity(native_definitions).encode(&mut realized);
    let handle=SnapshotHandle{semantic:export.manifest().content(),realization:realized.finish(),database:DatabaseIdentity{namespace:config.namespace.clone(),database:Name::new(&database).map_err(ModelError::codec)?}};
    // Loader has no outstanding tasks and every statement has a successful final response.
    // A distinct database VIEWER is the only credential emitted for serving.
    let mut bindings=Variables::new();bindings.insert("password",config.viewer_password.clone());
    client.query(format!("DEFINE USER `{}` ON DATABASE PASSWORD $password ROLES VIEWER",config.viewer_username)).bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let mut marker=Object::new();marker.insert("id",RecordId::new("publication","current"));marker.insert("handle",hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?));marker.insert("manifest",Bytes::from(serde_json::to_vec(export.manifest()).map_err(ModelError::codec)?));let mut bindings=Variables::new();bindings.insert("marker",marker);
    client.query("INSERT INTO publication $marker RETURN NONE").bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    client.invalidate().await.map_err(ModelError::codec)?;drop(loader);drop(client);
    NativeReader::connect(&config.endpoint,&config.viewer_credentials(),handle.clone()).await?;
    Ok(handle)
}
async fn load(export:&VerifiedExport,loader:&Loader,native_definitions:&str)->Result<(),ModelError>{
    loader.install(native_definitions).await?;
    let mut entities=Vec::new();let mut bytes=0;
    for row in export.entities()?{let row=row?;bytes+=serde_json::to_vec(&row).map_err(ModelError::codec)?.len();entities.push(row);if entities.len()>=128||bytes>=4<<20{loader.entities(&entities).await?;entities.clear();bytes=0;}}loader.entities(&entities).await?;
    let mut assertions=Vec::new();bytes=0;
    for row in export.assertions()?{let row=row?;bytes+=serde_json::to_vec(&row).map_err(ModelError::codec)?.len();assertions.push(row);if assertions.len()>=128||bytes>=4<<20{loader.assertions(&assertions).await?;assertions.clear();bytes=0;}}loader.assertions(&assertions).await?;
    entities.clear();for row in export.entities()?{entities.push(row?);if entities.len()>=128{loader.entity_references(&entities).await?;entities.clear();}}loader.entity_references(&entities).await?;
    assertions.clear();for row in export.assertions()?{assertions.push(row?);if assertions.len()>=128{loader.assertion_references(&assertions).await?;assertions.clear();}}loader.assertion_references(&assertions).await?;
    for original in export.originals(){let(original,mut file)=original?;loader.original_stream(original.source.0,original.content,original.byte_len,&mut file).await?;}
    crate::materialize_search(loader).await?;
    loader.reconcile(export.manifest()).await
}
mod search;
pub use search::materialize_search;
