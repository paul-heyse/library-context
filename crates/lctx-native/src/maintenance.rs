//! Closed current-format maintenance sessions.
use std::sync::Arc;
use surrealdb::{Surreal,engine::remote::grpc::Client,types::Object};
use lctx_model::domain::ModelError;
use crate::{RuntimeConfig,AuthenticationScope};
fn native(value:bool)->surrealdb::types::Value {surrealdb::types::Value::Bool(value)}
async fn marker(client:&Surreal<Client>)->Result<Object,ModelError>{
    let mut response=client.query("SELECT * FROM native_installation:current").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;let [row]=rows.as_slice()else{return Err(ModelError::Conflict("native maintenance installation cardinality"));};Ok(row.clone())
}
pub async fn maintenance_client(config:&RuntimeConfig)->Result<Arc<Surreal<Client>>,ModelError>{
    if config.authentication!=AuthenticationScope::Root{return Err(ModelError::Conflict("native maintenance installer authority"));}
    let client=crate::installation::check_installation(config).await?;
    let result=async{if marker(&client).await?.get("admission_open")!=Some(&native(false)){return Err(ModelError::Conflict("native maintenance closed admission"));}crate::control::check_pending_effects(&client).await}.await;
    if let Err(error)=result{let mut completion=lctx_model::domain::completion::Completion::default();completion.step("failed maintenance session invalidation",client.invalidate().await.map_err(ModelError::codec));return lctx_model::domain::completion::complete(Err(error),completion);}Ok(client)
}
