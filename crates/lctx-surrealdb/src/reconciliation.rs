//! Full canonical and physical readback before a database can become a published snapshot.
use crate::{Loader,codec,loader::json_value};
use lctx_model::domain::{graph::{Entity,Assertion,Target,GraphFamily,FamilyHasher,Manifest},ContentHash,ContentHasher,ModelError};
use surrealdb::types::{SurrealValue,RecordId,Value,Bytes,Variables};
#[derive(SurrealValue)]
#[surreal(crate="surrealdb::types")]
struct Node {id:RecordId,semantic_type:String,semantic_key:String,kind:i64,subtype:Value,content:String,canonical:Bytes,body:Value}
impl Loader{
    #[allow(clippy::mutable_key_type, reason = "SDK RecordId includes regex caches; these generated string IDs are never mutated") ]
    pub async fn reconcile(&self,manifest:&Manifest)->Result<(),ModelError>{
        manifest.validate()?;
        let mut external=std::collections::BTreeMap::new();let mut edge_counts=[0u64;2];
        for(table,family)in[("entity",GraphFamily::Entities),("assertion",GraphFamily::Assertions)]{
            let mut after=RecordId::new(table,"");let mut hasher=FamilyHasher::new(family);
            loop{
                let mut bind=Variables::new();bind.insert("after",after.clone());
                let mut response=self.client().query(format!("SELECT * FROM {table} WHERE id > $after ORDER BY id LIMIT 128")).bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let rows:Vec<Node>=response.take(0).map_err(ModelError::codec)?;if rows.is_empty(){break}
                let mut sources=Vec::new();let mut expected=Vec::new();
                for row in rows{
                    sources.push(row.id.clone());
                    let(key,content,kind,subtype,view,canonical)=if family==GraphFamily::Entities{
                        let entity:Entity=serde_json::from_slice(&row.canonical).map_err(ModelError::codec)?;entity.validate()?;
                        for(position,reference)in codec::entity_references(&entity).into_iter().enumerate(){
                            let target=lctx_model::domain::graph::reference_target(&reference)?.0;
                            remember_external(&mut external,&target)?;
                            expected.push(crate::loader::edge("reference",row.id.clone(),target,reference.field,0,Some(position as u32))?);
                        }
                        (entity.id().0,entity.content(),entity.kind()as i64,entity.subtype(),codec::entity_view(&entity)?,serde_json::to_vec(&entity).map_err(ModelError::codec)?)
                    }else{
                        let assertion:Assertion=serde_json::from_slice(&row.canonical).map_err(ModelError::codec)?;assertion.validate()?;
                        for p in &assertion.participants{remember_external(&mut external,&p.target)?;expected.push(crate::loader::edge("participant",row.id.clone(),p.target.clone(),p.field.as_deref().unwrap_or(""),p.role as i64,p.position)?);}
                        for(position,(target,_))in assertion.references()?.into_iter().enumerate(){if assertion.participants.iter().any(|p|p.target==target){continue}remember_external(&mut external,&target)?;expected.push(crate::loader::edge("participant",row.id.clone(),target,"__reference",-1,Some(position as u32))?);}
                        (assertion.id().0,assertion.content(),assertion.kind as i64,None,codec::assertion_view(&assertion)?,serde_json::to_vec(&assertion).map_err(ModelError::codec)?)
                    };
                    if row.id!=RecordId::new(table,key.hex())||row.semantic_type!=view.semantic_type||row.semantic_key!=view.semantic_key||row.kind!=kind||!match subtype{Some(value)=>row.subtype==Value::from_t(value),None=>matches!(row.subtype,Value::Null|Value::None)}||row.content!=content.hex()||row.body!=json_value(view.body)?||row.canonical.as_ref()!=canonical{
                        return Err(ModelError::Conflict("native canonical realization readback"))
                    }
                    if !hasher.push(key,content)?{return Err(ModelError::Conflict("duplicate native canonical graph element"))}after=row.id;
                }
                let edge_table=if family==GraphFamily::Entities{"reference"}else{"participant"};
                let mut bind=Variables::new();bind.insert("sources",sources);
                let mut response=self.client().query(format!("SELECT * FROM {edge_table} WHERE in IN $sources ORDER BY id")).bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let actual:Vec<Value>=response.take(0).map_err(ModelError::codec)?;
                expected.sort_by_key(value_id);
                if actual!=expected{return Err(ModelError::Conflict("native role adjacency readback"))}
                edge_counts[usize::from(family==GraphFamily::Assertions)]+=actual.len()as u64;
            }
            if !manifest.families.contains(&hasher.finish()){return Err(ModelError::Conflict("native canonical family reconciliation"))}
        }
        for(table,count)in[("reference",edge_counts[0]),("participant",edge_counts[1]),("external",external.len()as u64),("original",manifest.originals.len()as u64)]{if table_count(self,table).await?!=count{return Err(ModelError::Conflict("native physical inventory"))}}
        for chunk in external.into_iter().collect::<Vec<_>>().chunks(128){
            let mut bind=Variables::new();bind.insert("keys",chunk.iter().map(|(id,_)|id.clone()).collect::<Vec<_>>());
            let mut response=self.client().query("SELECT id,canonical FROM external WHERE id IN $keys ORDER BY id").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let actual:Vec<External>=response.take(0).map_err(ModelError::codec)?;
            if actual.len()!=chunk.len()||actual.iter().zip(chunk).any(|(a,(id,bytes))|&a.id!=id||a.canonical.as_ref()!=bytes){return Err(ModelError::Conflict("native external endpoint readback"))}
        }
        let mut original_chunks=0;
        for original in &manifest.originals{
            let source=RecordId::new("original",original.source.0.hex());let mut bind=Variables::new();bind.insert("source",source.clone());
            let mut response=self.client().query("SELECT * FROM $source").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let headers:Vec<Header>=response.take(0).map_err(ModelError::codec)?;
            if headers.len()!=1||headers[0].content!=original.content.hex()||headers[0].byte_len!=original.byte_len{return Err(ModelError::Conflict("native original header"))}
            let mut hash=ContentHasher::default();let mut position=0u64;
            loop{
                let mut bind=Variables::new();bind.insert("source",source.clone());bind.insert("start",position);
                let mut response=self.client().query("SELECT * FROM original_chunk WHERE source=$source AND start >= $start ORDER BY start LIMIT 128").bind(bind).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let chunks:Vec<Chunk>=response.take(0).map_err(ModelError::codec)?;if chunks.is_empty(){break}
                for chunk in chunks{original_chunks+=1;
                if chunk.start!=position||chunk.bytes.is_empty()||chunk.bytes.len()>65536||ContentHash::of(&chunk.bytes).hex()!=chunk.content{return Err(ModelError::Conflict("native original chunk"))}
                hash.update(&chunk.bytes);position+=chunk.bytes.len()as u64;
                }
            }
            if position!=original.byte_len||hash.finish()!=original.content{return Err(ModelError::Conflict("native original readback"))}
        }
        if table_count(self,"original_chunk").await?!=original_chunks{return Err(ModelError::Conflict("native original chunk inventory"))}Ok(())
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

#[allow(clippy::mutable_key_type, reason = "Generated string RecordIds have immutable ordering and contain no regex keys") ]
fn remember_external(rows:&mut std::collections::BTreeMap<RecordId,Vec<u8>>,target:&Target)->Result<(),ModelError>{if matches!(target,Target::External{..}){rows.insert(crate::reader::target_id(target.clone()),serde_json::to_vec(target).map_err(ModelError::codec)?);}Ok(())}
fn value_id(value:&Value)->RecordId{match value{Value::Object(object)=>RecordId::from_value(object.get("id").expect("generated role id").clone()).expect("generated record id"),_=>unreachable!("generated role object")}}
pub async fn table_count(loader:&Loader,table:&str)->Result<u64,ModelError>{
    let mut response=loader.client().query(format!("SELECT count() AS total FROM {table} GROUP ALL")).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let counts:Vec<InventoryCount>=response.take(0).map_err(ModelError::codec)?;Ok(counts.first().map_or(0,|row|row.total))
}
#[derive(SurrealValue)]
#[surreal(crate="surrealdb::types")]
struct External{id:RecordId,canonical:Bytes}

#[derive(SurrealValue)]
#[surreal(crate="surrealdb::types")]
struct InventoryCount{total:u64}
