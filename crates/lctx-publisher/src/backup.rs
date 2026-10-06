//! Trusted local logical transport. Imported metadata never becomes a serving realization.
use crate::{PrivatePublication, begin, abandon, seal};
use lctx_model::domain::{ModelError, graph::{Manifest, Entity, Assertion, semantic_contract}, serving::SnapshotHandle};
use lctx_surrealdb::{RuntimeConfig, NativeReader};
use lctx_surrealdb::surrealdb::{Surreal, engine::remote::http::{Client, Http}, opt::auth::Root,
    types::{Bytes, Variables, RecordId, SurrealValue}};
use std::{path::Path, io::{Write, Seek}};

async fn http(config:&RuntimeConfig,database:&str)->Result<Surreal<Client>,ModelError>{
    // The managed server exposes HTTP and gRPC on the same configured authority.
    let authority=config.endpoint.strip_prefix("grpc://").ok_or(ModelError::Schema("managed gRPC endpoint"))?;
    let client=Surreal::new::<Http>(authority).await.map_err(ModelError::codec)?;
    client.signin(Root{username:config.username.clone(),password:config.password.clone()}).await.map_err(ModelError::codec)?;
    client.use_ns(config.namespace.as_str()).use_db(database).await.map_err(ModelError::codec)?;
    Ok(client)
}

/// Write an exact logical dump only after the SDK's file export has consumed checked EOF.
/// Database users/access credentials and historical versions are excluded.
pub async fn backup(config:&RuntimeConfig,handle:&SnapshotHandle,output:&Path)->Result<(),ModelError>{
    if handle.database.namespace!=config.namespace{return Err(ModelError::Conflict("backup namespace"))}
    NativeReader::connect(&config.endpoint,&config.viewer_credentials(),handle.clone()).await?;
    if output.exists(){return Err(ModelError::Conflict("backup destination already exists"))}
    let parent=output.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let staged=tempfile::NamedTempFile::new_in(parent).map_err(ModelError::codec)?;
    let client=http(config,handle.database.database.as_str()).await?;
    let result=client.export(staged.path()).with_config().users(false).accesses(false).versions(false).await.map_err(ModelError::codec);
    let drained=client.invalidate().await.map_err(ModelError::codec);drop(client);
    result?;drained?;
    staged.as_file().sync_all().map_err(ModelError::codec)?;
    staged.persist_noclobber(output).map_err(|error|ModelError::codec(error.error))?;
    std::fs::File::open(parent).map_err(ModelError::codec)?.sync_all().map_err(ModelError::codec)
}

/// Import a trusted current-format local dump privately, then copy only canonical graph and
/// original bytes into a fresh realization. No imported marker, permission or function is served.
pub async fn restore(config:&RuntimeConfig,input:&Path,native_definitions:&str)->Result<SnapshotHandle,ModelError>{
    let staging=begin(config).await?;
    let imported=import(config,input,&staging).await;
    let result=match imported{
        Err(error)=>Err(error),
        Ok(manifest)=>{
            let fresh=match begin(config).await{Ok(fresh)=>fresh,Err(error)=>{abandon(&staging).await;return Err(error)}};
            match copy(config,&staging,&fresh,&manifest,native_definitions).await{
                Err(error)=>{abandon(&fresh).await;Err(error)},
                Ok(())=>match seal(&fresh,&manifest,config,native_definitions).await{
                    Ok(handle)=>Ok(handle),Err(error)=>{abandon(&fresh).await;Err(error)}
                }
            }
        }
    };
    abandon(&staging).await;
    result
}

async fn import(config:&RuntimeConfig,input:&Path,staging:&PrivatePublication)->Result<Manifest,ModelError>{
    let client=http(config,staging.database.as_str()).await?;
    // The HTTP import API consumes its full response and checks every returned statement.
    let imported=client.import(input).await.map_err(ModelError::codec);
    let drained=client.invalidate().await.map_err(ModelError::codec);drop(client);
    imported?;drained?;
    let reader=NativeReader::new(staging.loader.shared_client(),SnapshotHandle{
        semantic:lctx_model::domain::ContentHash::of(b"private-import"),
        realization:lctx_model::domain::ContentHash::of(b"private-import"),
        database:lctx_model::domain::serving::DatabaseIdentity{namespace:config.namespace.clone(),database:staging.database.clone()},
    });
    let bytes:Vec<Bytes>=reader.query("SELECT VALUE manifest FROM publication:current",Variables::new()).await?;
    let manifest:Manifest=serde_json::from_slice(bytes.first().filter(|_|bytes.len()==1)
        .ok_or(ModelError::Schema("logical dump publication manifest"))?).map_err(ModelError::codec)?;
    manifest.validate()?;
    if manifest.semantic_contract!=semantic_contract(&lctx_model::domain::model()?){return Err(ModelError::Conflict("restore semantic contract"))}
    staging.loader.reconcile(&manifest).await?;
    Ok(manifest)
}

async fn copy(config:&RuntimeConfig,staging:&PrivatePublication,fresh:&PrivatePublication,manifest:&Manifest,native_definitions:&str)->Result<(),ModelError>{
    fresh.loader.install(native_definitions).await?;
    // Two passes: materialize every endpoint before constructing native role/reference arcs.
    for references in [false,true]{
        let mut after=RecordId::new("entity","");
        loop{
            let (rows,last)=page::<Entity>(staging,"entity",after).await?;
            if rows.is_empty(){break}
            if references{fresh.loader.entity_references(&rows).await?}else{fresh.loader.entities(&rows).await?}
            after=last;
        }
        let mut after=RecordId::new("assertion","");
        loop{
            let (rows,last)=page::<Assertion>(staging,"assertion",after).await?;
            if rows.is_empty(){break}
            if references{fresh.loader.assertion_references(&rows).await?}else{fresh.loader.assertions(&rows).await?}
            after=last;
        }
    }
    let source=NativeReader::new(staging.loader.shared_client(),SnapshotHandle{
        semantic:manifest.content(),realization:lctx_model::domain::ContentHash::of(b"private-import"),
        database:lctx_model::domain::serving::DatabaseIdentity{namespace:config.namespace.clone(),database:staging.database.clone()},
    });
    for original in &manifest.originals{
        let mut file=tempfile::tempfile().map_err(ModelError::codec)?;let mut start=0;
        while start<original.byte_len{
            let length=(original.byte_len-start).min(65536)as usize;
            file.write_all(&source.original_bytes(original.source,start,length).await?).map_err(ModelError::codec)?;
            start+=length as u64;
        }
        file.rewind().map_err(ModelError::codec)?;
        fresh.loader.original_stream(original.source.0,original.content,original.byte_len,&mut file).await?;
    }
    crate::materialize_search(&fresh.loader).await?;
    fresh.loader.reconcile(manifest).await
}

async fn page<T:serde::de::DeserializeOwned>(staging:&PrivatePublication,table:&str,after:RecordId)->Result<(Vec<T>,RecordId),ModelError>{
    let mut vars=Variables::new();vars.insert("after",after.clone());
    let mut response=staging.loader.client().query(format!("SELECT id,canonical FROM {table} WHERE id>$after ORDER BY id LIMIT 128"))
        .bind(vars).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    #[derive(lctx_surrealdb::surrealdb::types::SurrealValue)]
    #[surreal(crate="lctx_surrealdb::surrealdb::types")]
    struct Row{id:RecordId,canonical:Bytes}
    let rows:Vec<Row>=response.take(0).map_err(ModelError::codec)?;
    let last=rows.last().map(|r|r.id.clone()).unwrap_or(after);
    let values=rows.into_iter().map(|r|serde_json::from_slice(&r.canonical).map_err(ModelError::codec)).collect::<Result<_,_>>()?;
    Ok((values,last))
}

/// Retire an explicitly named, unselected published database after the operator has stopped
/// every known reader. There is no automatic lease/history or selected-handle replacement.
pub async fn retire(config:&RuntimeConfig,handle:&SnapshotHandle,readers_stopped:bool)->Result<(),ModelError>{
    if !readers_stopped{return Err(ModelError::Invalid("retirement requires stopped readers".into()))}
    if handle.database.namespace!=config.namespace{return Err(ModelError::Conflict("retirement namespace"))}
    let database=handle.database.database.as_str();
    let suffix=database.strip_prefix("snapshot_").filter(|s|s.len()==64&&s.bytes().all(|c|c.is_ascii_hexdigit()))
        .ok_or(ModelError::Schema("published private database name"))?;
    if config.selection.try_exists().map_err(ModelError::codec)?&&config.selected()?.database==handle.database{return Err(ModelError::Conflict("cannot retire selected snapshot"))}
    let viewer=NativeReader::connect(&config.endpoint,&config.viewer_credentials(),handle.clone()).await?;
    viewer.client().invalidate().await.map_err(ModelError::codec)?;drop(viewer);
    let client=lctx_surrealdb::reader::connect(&config.endpoint,&config.root_credentials(),config.namespace.as_str(),database).await?;
    client.query(format!("REMOVE DATABASE `snapshot_{suffix}`")).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    client.invalidate().await.map_err(ModelError::codec)?;
    Ok(())
}
