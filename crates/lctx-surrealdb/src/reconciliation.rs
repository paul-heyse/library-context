//! Full canonical and physical readback before a database can become a published snapshot.
use crate::{Loader,codec,loader::json_value};
use lctx_model::domain::{graph::{Entity,Assertion,GraphFamily,FamilyHasher,Manifest},ContentHash,ContentHasher,ModelError};
use surrealdb::types::{SurrealValue,RecordId,Value,Bytes,Variables};
#[derive(SurrealValue)]
#[surreal(crate="surrealdb::types")]
struct Node {id:RecordId,semantic_type:String,semantic_key:String,kind:i64,subtype:Option<i16>,content:String,canonical:Bytes,body:Value}
impl Loader{
    pub async fn reconcile(&self,manifest:&Manifest)->Result<(),ModelError>{
        manifest.validate()?;
        for(table,family)in[("entity",GraphFamily::Entities),("assertion",GraphFamily::Assertions)]{
            let mut after=RecordId::new(table,"");let mut hasher=FamilyHasher::new(family);
            loop{
                let mut bind=Variables::new();bind.insert("after",after.clone());
                let mut response=self.client().query(format!("SELECT * FROM {table} WHERE id > $after ORDER BY id LIMIT 128")).bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let rows:Vec<Node>=response.take(0).map_err(ModelError::codec)?;if rows.is_empty(){break}
                for row in rows{
                    let(key,content,kind,subtype,view,canonical)=if family==GraphFamily::Entities{
                        let entity:Entity=serde_json::from_slice(&row.canonical).map_err(ModelError::codec)?;entity.validate()?;
                        (entity.id().0,entity.content(),entity.kind()as i64,entity.subtype(),codec::entity_view(&entity)?,serde_json::to_vec(&entity).map_err(ModelError::codec)?)
                    }else{
                        let assertion:Assertion=serde_json::from_slice(&row.canonical).map_err(ModelError::codec)?;assertion.validate()?;
                        (assertion.id().0,assertion.content(),assertion.kind as i64,None,codec::assertion_view(&assertion)?,serde_json::to_vec(&assertion).map_err(ModelError::codec)?)
                    };
                    if row.id!=RecordId::new(table,key.hex())||row.semantic_type!=view.semantic_type||row.semantic_key!=view.semantic_key||row.kind!=kind||row.subtype!=subtype||row.content!=content.hex()||row.body!=json_value(view.body)?||row.canonical.as_ref()!=canonical{
                        return Err(ModelError::Conflict("native canonical realization readback"))
                    }
                    if !hasher.push(key,content)?{return Err(ModelError::Conflict("duplicate native canonical graph element"))}after=row.id;
                }
            }
            if !manifest.families.contains(&hasher.finish()){return Err(ModelError::Conflict("native canonical family reconciliation"))}
        }
        for original in &manifest.originals{
            let source=RecordId::new("original",original.source.0.hex());let mut bind=Variables::new();bind.insert("source",source.clone());
            let mut response=self.client().query("SELECT * FROM $source").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let headers:Vec<Header>=response.take(0).map_err(ModelError::codec)?;
            if headers.len()!=1||headers[0].content!=original.content.hex()||headers[0].byte_len!=original.byte_len{return Err(ModelError::Conflict("native original header"))}
            let mut hash=ContentHasher::default();let mut position=0u64;
            loop{
                let mut bind=Variables::new();bind.insert("source",source.clone());bind.insert("start",position);
                let mut response=self.client().query("SELECT * FROM original_chunk WHERE source=$source AND start >= $start ORDER BY start LIMIT 1").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let chunks:Vec<Chunk>=response.take(0).map_err(ModelError::codec)?;if chunks.is_empty(){break}let chunk=&chunks[0];
                if chunk.start!=position||chunk.bytes.is_empty()||chunk.bytes.len()>65536||ContentHash::of(&chunk.bytes).hex()!=chunk.content{return Err(ModelError::Conflict("native original chunk"))}
                hash.update(&chunk.bytes);position+=chunk.bytes.len()as u64;
            }
            if position!=original.byte_len||hash.finish()!=original.content{return Err(ModelError::Conflict("native original readback"))}
        }
        Ok(())
    }
}
#[derive(SurrealValue)]
#[surreal(crate="surrealdb::types")]
struct Header{id:RecordId,content:String,byte_len:u64}
#[derive(SurrealValue)]
#[surreal(crate="surrealdb::types")]
struct Chunk{id:RecordId,source:RecordId,start:u64,content:String,bytes:Bytes}
impl crate::NativeReader{
    pub async fn original_bytes(&self,source:lctx_model::domain::graph::EntityId,start:u64,length:usize)->Result<Vec<u8>,ModelError>{
        if length>256<<10{return Err(ModelError::Invalid("original byte page exceeds response bound".into()))}
        let end=start.checked_add(length as u64).ok_or(ModelError::Schema("original byte range"))?;
        let mut bind=Variables::new();bind.insert("source",RecordId::new("original",source.0.hex()));bind.insert("start",start/65536*65536);bind.insert("end",end);
        let mut response=self.client().query("SELECT * FROM original_chunk WHERE source=$source AND start >= $start AND start < $end ORDER BY start").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let chunks:Vec<Chunk>=response.take(0).map_err(ModelError::codec)?;let mut result=Vec::with_capacity(length);let mut position=start;
        for chunk in chunks{
            if ContentHash::of(&chunk.bytes).hex()!=chunk.content{return Err(ModelError::Conflict("original chunk content"))}
            let offset=position.checked_sub(chunk.start).filter(|v|*v<chunk.bytes.len()as u64).ok_or(ModelError::Schema("original chunk continuity"))?as usize;
            let count=(end-position).min((chunk.bytes.len()-offset)as u64)as usize;result.extend_from_slice(&chunk.bytes[offset..offset+count]);position+=count as u64;
        }
        if position!=end{return Err(ModelError::Invalid("original byte range unavailable".into()))}Ok(result)
    }
}
