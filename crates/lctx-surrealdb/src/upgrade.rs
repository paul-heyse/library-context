//! Explicit, resumable migration of an installed control format under closed maintenance.
//! Ordinary attachment and installation never reinterpret an older marker.
use crate::{AuthenticationScope, Loader, RuntimeConfig, reader};
use lctx_model::domain::{ContentHash, ModelError};
use surrealdb::{Surreal, engine::remote::grpc::Client, types::{Object, Value, Variables}};

pub const LEGACY_SCHEMA: &str = "464537d364bfabdc43acd2ca3f05037d6f9c29558ff7d72bdfdcea23f5870580";
const JOURNAL_SCHEMA: &str = "DEFINE TABLE native_upgrade SCHEMAFULL; DEFINE FIELD source ON native_upgrade TYPE string; DEFINE FIELD target ON native_upgrade TYPE string; DEFINE FIELD generation ON native_upgrade TYPE string; DEFINE FIELD phase ON native_upgrade TYPE string;";

fn check_source(row: &Object, config: &RuntimeConfig, expected: ContentHash) -> Result<(), ModelError> {
    if expected.hex() != LEGACY_SCHEMA
        || row.get("generation") != Some(&Value::String(config.service_generation.hex()))
        || row.get("admission_open") != Some(&Value::Bool(false))
        || row.get("schema") != Some(&Value::String(expected.hex()))
        || row.get("schema_version") != Some(&Value::Number(surrealdb::types::Number::Int(3))) {
        return Err(ModelError::Conflict("explicit native upgrade source/generation/closed admission"));
    }
    Ok(())
}
async fn marker(client: &Surreal<Client>) -> Result<Object, ModelError> {
    let mut response=client.query("SELECT * FROM native_installation:current").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [row]=rows.as_slice() else { return Err(ModelError::Conflict("native upgrade installation marker")); };
    Ok(row.clone())
}
/// The host maintenance owner supplies the actual borrower-drain proof; this checks
/// the native half of that capability and never opens admission on the caller's behalf.
pub async fn maintenance_client(config: &RuntimeConfig) -> Result<std::sync::Arc<Surreal<Client>>, ModelError> {
    if config.authentication != AuthenticationScope::Root {
        return Err(ModelError::Conflict("native lifecycle maintenance requires installer authority"));
    }
    let client=crate::compiler::check_installation(config).await?;
    if marker(&client).await?.get("admission_open")!=Some(&Value::Bool(false)) {
        return Err(ModelError::Conflict("native lifecycle maintenance requires closed admission"));
    }
    crate::control::check_pending_effects(&client).await?;
    Ok(client)
}
async fn checkpoint(client:&Surreal<Client>, operation:ContentHash, phase:&str)->Result<(),ModelError>{
    client.query("UPDATE $journal SET phase=$phase RETURN NONE")
        .bind(("journal",surrealdb::types::RecordId::new("native_upgrade",operation.hex())))
        .bind(("phase",phase.to_owned())).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
    Ok(())
}
/// Caller owns host borrower drainage, daemon restart and credential/session replacement.
/// This transaction journal owns the exact native migration identity and publishes last.
pub async fn upgrade(config:&RuntimeConfig, expected:ContentHash, operation:ContentHash, native_definitions:&str)->Result<(),ModelError>{
    if config.authentication!=AuthenticationScope::Root { return Err(ModelError::Conflict("native upgrade requires installer authority")); }
    let client=reader::connect(&config.endpoint,&config.writer_credentials(),config.namespace.as_str(),config.database.as_str()).await?;
    let target=crate::compiler::base_schema_identity();
    let installed=marker(&client).await?;
    let published=installed.get("schema_version")==Some(&Value::Number(surrealdb::types::Number::Int(crate::control::SCHEMA_VERSION)))
        && installed.get("schema")==Some(&Value::String(target.hex()));
    if !published { check_source(&installed,config,expected)?; }
    else if expected.hex()!=LEGACY_SCHEMA || installed.get("generation")!=Some(&Value::String(config.service_generation.hex())) || installed.get("admission_open")!=Some(&Value::Bool(false)) {
        return Err(ModelError::Conflict("native upgrade published installation identity"));
    }
    let loader=Loader::new(client.clone());
    if !published { loader.install_declarations(JOURNAL_SCHEMA,"native upgrade identity journal").await?; }
    let mut bindings=Variables::new();
    bindings.insert("journal",surrealdb::types::RecordId::new("native_upgrade",operation.hex()));
    bindings.insert("source",expected.hex()); bindings.insert("target",target.hex()); bindings.insert("generation",config.service_generation.hex());
    if !published {
        client.query("BEGIN; LET $old=SELECT * FROM ONLY $journal FOR UPDATE; IF $old=NONE { CREATE $journal SET source=$source,target=$target,generation=$generation,phase='intent' RETURN NONE; } ELSE { IF $old.source!=$source OR $old.target!=$target OR $old.generation!=$generation { THROW 'native upgrade operation collision'; }; }; COMMIT;")
            .bind(bindings.clone()).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
    }
    let mut response=client.query("SELECT * FROM $journal").bind(bindings.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let rows:Vec<Object>=response.take(0).map_err(ModelError::codec)?;
    let [journal]=rows.as_slice() else { return Err(ModelError::Conflict("native upgrade journal missing")); };
    if journal.get("source")!=Some(&Value::String(expected.hex())) || journal.get("target")!=Some(&Value::String(target.hex())) || journal.get("generation")!=Some(&Value::String(config.service_generation.hex())) {
        return Err(ModelError::Conflict("native upgrade journal identity"));
    }
    if published {
        if journal.get("phase")!=Some(&Value::String("published".into())) { return Err(ModelError::Conflict("native upgrade terminal journal")); }
        return crate::control::check_installation(&client,config.service_generation).await;
    }
    loader.install_declarations(crate::control::schema(),"native control format upgrade").await?;
    loader.install(native_definitions).await?;
    loader.install_declarations(&crate::compiler::compiler_schema(),"compiler format upgrade").await?;
    loader.install_declarations(crate::compiler::VIEW_SCHEMA,"completed view format upgrade").await?;
    checkpoint(&client,operation,"declarations").await?;
    crate::control::migrate_legacy_state(&client,operation).await?;
    checkpoint(&client,operation,"translated").await?;
    // The installation marker and journal become visible together only after all verified work.
    bindings.insert("version",crate::control::SCHEMA_VERSION);
    client.query("BEGIN; LET $installed=SELECT * FROM ONLY native_installation:current FOR UPDATE; IF $installed.generation!=$generation OR $installed.schema!=$source OR $installed.schema_version!=3 OR $installed.admission_open OR $installed.era!=1 OR $installed.closed_through!=0 { THROW 'native upgrade publication precondition'; }; UPDATE native_installation:current SET schema=$target,schema_version=$version RETURN NONE; UPDATE $journal SET phase='published' RETURN NONE; COMMIT;")
        .bind(bindings).await.map_err(crate::loader::write_failure)?.check().map_err(ModelError::codec)?;
    crate::control::check_installation(&client,config.service_generation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_upgrade_accepts_only_exact_closed_source() {
        let generation=ContentHash::of(b"generation");
        let expected=ContentHash(hex::decode(LEGACY_SCHEMA).unwrap().try_into().unwrap());
        let config=RuntimeConfig { reuse:None,serving_limits:None,endpoint:"grpc://127.0.0.1:1".into(),username:"root".into(),password:"secret".into(),viewer_username:"viewer".into(),viewer_password:"secret".into(),namespace:lctx_model::domain::serving::Name::new("test").unwrap(),database:lctx_model::domain::serving::Name::new("validation").unwrap(),service_generation:generation,authentication:AuthenticationScope::Root,cache_database:lctx_model::domain::serving::Name::new("validation").unwrap(),selection:"/tmp/unused-selection".into() };
        let mut row=Object::new(); row.insert("generation",generation.hex()); row.insert("schema",expected.hex()); row.insert("schema_version",3i64); row.insert("admission_open",false);
        assert!(check_source(&row,&config,expected).is_ok());
        row.insert("admission_open",true); assert!(check_source(&row,&config,expected).is_err());
        row.insert("admission_open",false); row.insert("schema_version",4i64); assert!(check_source(&row,&config,expected).is_err());
        row.insert("schema_version",3i64); row.insert("generation",ContentHash::of(b"foreign").hex()); assert!(check_source(&row,&config,expected).is_err());
    }
}
