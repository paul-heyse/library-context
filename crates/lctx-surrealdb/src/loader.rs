//! Checked bulk writes into a private database. This loader never publishes a handle.
use crate::{codec,reader::target_id};
use lctx_model::domain::{graph::{Entity,Assertion,Target},ModelError,ContentHash,Key,KeySink};
use surrealdb::{Surreal,engine::remote::grpc::Client,types::{Value,Variables,Bytes,SurrealValue,Object,RecordId}};
use std::sync::Arc;

pub struct Loader {client:Arc<Surreal<Client>>}
impl Loader {
    pub fn new(client:Arc<Surreal<Client>>)->Self{Self{client}}
    pub fn client(&self)->&Surreal<Client>{&self.client}
    pub fn shared_client(&self)->Arc<Surreal<Client>>{self.client.clone()}
    pub async fn install(&self,native_definitions:&str)->Result<(),ModelError>{
        // Canonical DDL contains only finite declarations. Coarse checked batches avoid one
        // large setup transaction; executable function bodies remain intact in the final query.
        let schema=crate::schema::canonical_schema();let statements=schema.split(';').filter(|s|!s.trim().is_empty()).collect::<Vec<_>>();
        for chunk in statements.chunks(32){self.client.query(chunk.join(";")+";").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;}
        self.client.query(native_definitions).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok(())
    }
    async fn insert(&self,table:&str,rows:Vec<Value>,relation:bool)->Result<(),ModelError>{
        if rows.is_empty(){return Ok(())}
        let mut bindings=Variables::new();bindings.insert("rows",rows);
        let sql=format!("INSERT {}INTO {table} $rows RETURN NONE",if relation{"RELATION "}else{""});
        self.client.query(sql).bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok(())
    }
    pub async fn entities(&self,rows:&[Entity])->Result<(),ModelError>{
        let values=rows.iter().map(|row|{
            row.validate()?;let view=codec::entity_view(row)?;
            let mut obj=Object::new();obj.insert("id",RecordId::new("entity",row.id().0.hex()));obj.insert("semantic_type",view.semantic_type);obj.insert("semantic_key",view.semantic_key);obj.insert("kind",row.kind() as i64);obj.insert("subtype",row.subtype().map(Value::from_t).unwrap_or(Value::Null));obj.insert("content",row.content().hex());obj.insert("canonical",Bytes::from(serde_json::to_vec(row).map_err(ModelError::codec)?));obj.insert("body",json_value(view.body)?);Ok(Value::Object(obj))
        }).collect::<Result<Vec<_>,ModelError>>()?;self.insert("entity",values,false).await
    }
    pub async fn assertions(&self,rows:&[Assertion])->Result<(),ModelError>{
        let values=rows.iter().map(|row|{
            row.validate()?;let view=codec::assertion_view(row)?;
            let mut obj=Object::new();obj.insert("id",RecordId::new("assertion",row.id().0.hex()));obj.insert("semantic_type",view.semantic_type);obj.insert("semantic_key",view.semantic_key);obj.insert("kind",row.kind as i64);obj.insert("subtype",Value::Null);obj.insert("content",row.content().hex());obj.insert("canonical",Bytes::from(serde_json::to_vec(row).map_err(ModelError::codec)?));obj.insert("body",json_value(view.body)?);Ok(Value::Object(obj))
        }).collect::<Result<Vec<_>,ModelError>>()?;self.insert("assertion",values,false).await
    }
    /// Load only after every canonical endpoint is present. Parallel roles keep distinct IDs.
    pub async fn entity_references(&self,rows:&[Entity])->Result<(),ModelError>{
        let mut values=vec![];
        for row in rows{
            let source=target_id(Target::Entity(row.id()));
            let references=codec::entity_references(row);
            for (position,reference) in references.into_iter().enumerate(){
                let target=lctx_model::domain::graph::reference_target(&reference)?.0;
                self.external(&target).await?;
                values.push(edge("reference",source.clone(),target,reference.field,0,Some(position as u32))?);
            }
        }self.insert("reference",values,true).await
    }
    pub async fn assertion_references(&self,rows:&[Assertion])->Result<(),ModelError>{
        let mut values=vec![];
        for row in rows{
            let source=target_id(Target::Assertion(row.id()));
            for participant in &row.participants{
                self.external(&participant.target).await?;
                values.push(edge("participant",source.clone(),participant.target.clone(),participant.field.as_deref().unwrap_or(""),participant.role as i64,participant.position)?);
            }
            // Non-payload qualification/run/evidence/derivation references remain native adjacency too.
            for (position,(target,_)) in row.references()?.into_iter().enumerate(){
                if row.participants.iter().any(|p|p.target==target){continue}
                self.external(&target).await?;values.push(edge("participant",source.clone(),target,"__reference",-1,Some(position as u32))?);
            }
        }self.insert("participant",values,true).await
    }
    async fn external(&self,target:&Target)->Result<(),ModelError>{
        if !matches!(target,Target::External{..}){return Ok(())}
        let mut obj=Object::new();obj.insert("id",target_id(target.clone()));obj.insert("canonical",Bytes::from(serde_json::to_vec(target).map_err(ModelError::codec)?));
        let mut bindings=Variables::new();bindings.insert("row",obj);
        self.client.query("INSERT IGNORE INTO external $row RETURN NONE").bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok(())
    }
    pub async fn original_stream(&self,source:ContentHash,content:ContentHash,length:u64,input:&mut impl std::io::Read)->Result<(),ModelError>{
        let source_key=source.hex();let source=RecordId::new("original",source_key.clone());
        let mut header=Object::new();header.insert("id",source.clone());header.insert("content",content.hex());header.insert("byte_len",i64::try_from(length).map_err(ModelError::codec)?);
        self.insert("original",vec![Value::Object(header)],false).await?;
        let mut hasher=lctx_model::domain::ContentHasher::default();let mut position=0u64;let mut buffer=vec![0u8;65536];let mut pending=Vec::new();
        loop {let mut count=0;while count<buffer.len(){let next=input.read(&mut buffer[count..]).map_err(ModelError::codec)?;if next==0{break}count+=next;}if count==0{break}hasher.update(&buffer[..count]);
            let mut row=Object::new();row.insert("id",RecordId::new("original_chunk",format!("{source_key}_{position}")));row.insert("source",source.clone());row.insert("start",i64::try_from(position).map_err(ModelError::codec)?);row.insert("bytes",Bytes::from(buffer[..count].to_vec()));row.insert("content",ContentHash::of(&buffer[..count]).hex());pending.push(Value::Object(row));if pending.len()==64{self.insert("original_chunk",std::mem::take(&mut pending),false).await?;}position+=count as u64;
        }
        self.insert("original_chunk",pending,false).await?;
        if position!=length||hasher.finish()!=content{return Err(ModelError::Conflict("original bytes"))}Ok(())
    }

}
pub(crate) fn edge(table:&str,source:RecordId,target:Target,field:&str,role:i64,position:Option<u32>)->Result<Value,ModelError>{
    let mut sink=KeySink::new("native-graph-role/v1");table.to_string().encode(&mut sink);sink.part(b"source",&serde_json::to_vec(&source).map_err(ModelError::codec)?);target.encode(&mut sink);field.to_string().encode(&mut sink);role.encode(&mut sink);position.map(i64::from).encode(&mut sink);
    let mut obj=Object::new();obj.insert("id",RecordId::new(table,sink.finish().hex()));obj.insert("in",source);obj.insert("out",target_id(target));obj.insert("field",field.to_string());obj.insert("role",role);obj.insert("position",position.map(Value::from_t).unwrap_or(Value::Null));Ok(Value::Object(obj))
}
pub fn json_value(value:serde_json::Value)->Result<Value,ModelError>{
    Ok(match value{
        serde_json::Value::Null=>Value::Null,
        serde_json::Value::Bool(v)=>v.into_value(),serde_json::Value::String(v)=>v.into_value(),
        serde_json::Value::Number(v)=>{if let Some(v)=v.as_i64(){v.into_value()}else if let Some(v)=v.as_u64(){v.into_value()}else{v.as_f64().ok_or(ModelError::Schema("native float"))?.into_value()}},
        serde_json::Value::Array(v)=>v.into_iter().map(json_value).collect::<Result<Vec<_>,_>>()?.into_value(),
        serde_json::Value::Object(v)=>{let mut obj=Object::new();for(k,v)in v{obj.insert(k,json_value(v)?);}Value::Object(obj)}
    })
}
