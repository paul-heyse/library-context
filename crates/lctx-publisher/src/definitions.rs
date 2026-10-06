//! Cold definition inventory. No content scan or INFO request is added to ordinary serving.
use lctx_model::domain::{ContentHash,Key,KeySink,ModelError};
use lctx_surrealdb::{Loader,schema};
use lctx_surrealdb::surrealdb::types::{Object,Value,ToSql};

const DB_GROUPS:&[&str]=&["accesses","analyzers","apis","buckets","configs","functions","models","modules","params","sequences","tables","users"];
const TABLE_GROUPS:&[&str]=&["events","fields","indexes","lives","tables"];
fn groups(object:&Object,expected:&[&str])->Result<(),ModelError>{
    if object.len()!=expected.len()||object.keys().any(|key|!expected.contains(&key.as_str()))||object.values().any(|value|!matches!(value,Value::Object(_))){return Err(ModelError::Schema("native definition inventory groups"))}
    Ok(())
}
async fn info(loader:&Loader,sql:String)->Result<Object,ModelError>{
    let mut response=loader.client().query(sql).await.map_err(|_|ModelError::Schema("native definition inventory response"))?.check().map_err(|_|ModelError::Schema("native definition inventory response"))?;
    response.take(0).map_err(|_|ModelError::Schema("native definition inventory object"))
}
/// Version-scoped effective definitions plus the current owned blueprint. INFO's plain maps
/// preserve rendered expressions; structured INFO includes mutable physical IDs. Neither INFO
/// variant exposes the stored STRICT flag at 3.3: creation uses explicit STRICT, not an inferred flag.
pub(crate) async fn verify_realization(loader:&Loader,native_definitions:&str)->Result<ContentHash,ModelError>{
    let version=loader.client().version().await.map_err(ModelError::codec)?.to_string();
    if !version.starts_with("3.3."){return Err(ModelError::Conflict("reviewed native engine family"))}
    let mut database=info(loader,"INFO FOR DB".into()).await?;groups(&database,DB_GROUPS)?;
    let Some(Value::Object(tables))=database.get("tables")else{return Err(ModelError::Schema("native table inventory"))};
    let names:Vec<_>=tables.keys().cloned().collect();
    database.remove("users");database.remove("accesses");
    let mut inventory=Object::new();inventory.insert("database",database);
    let mut details=Object::new();
    for name in names{
        // Inventory names come from the engine, not a request. Escape quoted identifiers.
        let escaped=name.replace('\\',"\\\\").replace('`',"\\`");
        let mut table=info(loader,format!("INFO FOR TABLE `{escaped}`")).await?;groups(&table,TABLE_GROUPS)?;
        table.remove("lives");details.insert(name,table);
    }
    inventory.insert("table_details",details);
    let mut key=KeySink::new("native-realization/v2");key.part(b"engine",version.as_bytes());
    key.part(b"lowering",b"lctx-native-graph/v3;remote-sdk-3.3;sparse-atomic-id-scopes");
    schema::realization_identity(native_definitions).encode(&mut key);
    key.part(b"effective-definitions/info-maps-v1",inventory.to_sql().as_bytes());
    Ok(key.finish())
}
