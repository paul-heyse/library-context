use lctx_model::domain::{graph::{Target, Entity, Assertion}, serving::SnapshotHandle, ModelError, Record,Key,KeySink};
use serde::{Serialize,de::DeserializeOwned};
use std::sync::Arc;
use surrealdb::{Surreal,engine::remote::grpc::{Client,Grpc},opt::auth::{Root,Database},types::{Variables,SerdeWrapper,Bytes,RecordId,Value,SurrealValue}};

/// Secrets are supplied by the operator; this type deliberately has no Debug or Serialize.
pub enum Credentials {
    Root { username: String, password: String },
    Database { username: String, password: String },
}
/// Finite inputs supplied by a model-owned operation, never a request-controlled SQL predicate.
pub enum RecordSelection {
    Keys(Vec<[u8;16]>),
    Scope { field: String, values: Vec<serde_json::Value> },
    Connected { targets: Vec<Target>, fields: Vec<String> },
}
#[derive(Clone)]
pub struct NativeReader { client: Arc<Surreal<Client>>, handle: SnapshotHandle }
impl NativeReader {
    pub fn new(client: Arc<Surreal<Client>>, handle: SnapshotHandle) -> Self { Self {client,handle} }
    pub async fn connect(endpoint:&str, credentials:&Credentials, handle:SnapshotHandle)->Result<Self,ModelError>{
        let client=connect(endpoint,credentials,handle.database.namespace.as_str(),handle.database.database.as_str()).await?;
        let reader=Self::new(client,handle);
        let markers:Vec<String>=reader.query("SELECT VALUE handle FROM publication:current",Variables::new()).await?;
        let expected=hex::encode(serde_json::to_vec(reader.handle()).map_err(ModelError::codec)?);
        if markers != vec![expected] {return Err(ModelError::Conflict("snapshot publication handle"));}
        Ok(reader)
    }
    pub fn handle(&self)->&SnapshotHandle{&self.handle}
    pub fn client(&self)->&Surreal<Client>{&self.client}
    /// Query results become visible only after the SDK has received the complete checked response.
    pub async fn query<T:Serialize+DeserializeOwned+'static>(&self, sql:impl Into<String>, bindings:Variables)->Result<T,ModelError>{
        let mut response=self.client.query(sql.into()).bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let value:Value=response.take(0).map_err(ModelError::codec)?;
        SerdeWrapper::<T>::from_value(value).map(|value|value.0).map_err(ModelError::codec)
    }
    pub async fn records<R:Record+DeserializeOwned>(&self,selection:RecordSelection)->Result<Vec<R>,ModelError>{
        let mut bindings=Variables::new();bindings.insert("type",R::NAME.to_string());
        let predicate=match selection {
            RecordSelection::Keys(keys)=>{if keys.is_empty(){return Ok(vec![])}bindings.insert("keys",keys.iter().map(hex::encode).collect::<Vec<_>>());"semantic_key IN $keys".to_owned()},
            RecordSelection::Scope{field,values}=>{
                if values.is_empty(){return Ok(vec![])}
                if !R::fields().iter().any(|f|f.name()==field){return Err(ModelError::Invalid("undeclared native scope field".into()))}
                bindings.insert("values",SerdeWrapper(values));format!("body.`{field}` IN $values")
            },
            RecordSelection::Connected{targets,fields}=>{
                if targets.is_empty(){return Ok(vec![])}
                bindings.insert("targets",targets.into_iter().map(target_id).collect::<Vec<_>>());bindings.insert("fields",fields);
                "id IN (SELECT VALUE in FROM participant WHERE out IN $targets AND (array::len($fields)=0 OR field IN $fields)) OR id IN (SELECT VALUE in FROM reference WHERE out IN $targets AND (array::len($fields)=0 OR field IN $fields))".into()
            }
        };
        let sql=format!("SELECT VALUE canonical FROM entity WHERE semantic_type=$type AND ({predicate}) ORDER BY semantic_key; SELECT VALUE canonical FROM assertion WHERE semantic_type=$type AND ({predicate}) ORDER BY semantic_key;");
        let mut response=self.client.query(sql).bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let entities:Vec<Bytes>=response.take(0).map_err(ModelError::codec)?;
        let assertions:Vec<Bytes>=response.take(1).map_err(ModelError::codec)?;
        let mut result=Vec::with_capacity(entities.len()+assertions.len());
        for payload in entities { let entity:Entity=serde_json::from_slice(&payload).map_err(ModelError::codec)?;result.push(crate::codec::entity_record::<R>(&entity)?); }
        for payload in assertions { let assertion:Assertion=serde_json::from_slice(&payload).map_err(ModelError::codec)?;result.push(crate::codec::assertion_record::<R>(&assertion)?); }
        Ok(result)
    }
}
pub fn target_id(target:Target)->RecordId {match target{Target::Entity(id)=>RecordId::new("entity",id.0.hex()),Target::Assertion(id)=>RecordId::new("assertion",id.0.hex()),external@Target::External{..}=>{let mut sink=KeySink::new("graph-external-endpoint/v1");external.encode(&mut sink);RecordId::new("external",sink.finish().hex())}}}
pub async fn connect(endpoint:&str,credentials:&Credentials,namespace:&str,database:&str)->Result<Arc<Surreal<Client>>,ModelError>{
    let client=Surreal::new::<Grpc>(endpoint.strip_prefix("grpc://").ok_or_else(||ModelError::Invalid("native gRPC endpoint required".into()))?).await.map_err(ModelError::codec)?;
    match credentials{
        Credentials::Root{username,password}=>{client.signin(Root{username:username.clone(),password:password.clone()}).await.map_err(ModelError::codec)?;},
        Credentials::Database{username,password}=>{client.signin(Database{namespace:namespace.into(),database:database.into(),username:username.clone(),password:password.clone()}).await.map_err(ModelError::codec)?;}
    }
    client.use_ns(namespace).use_db(database).await.map_err(ModelError::codec)?;Ok(Arc::new(client))
}
