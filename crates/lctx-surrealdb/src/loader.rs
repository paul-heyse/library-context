//! Checked bulk writes into a private database. This loader never publishes a handle.
use crate::{codec, reader::target_id};
use lctx_model::domain::{
    ContentHash, Key, KeySink, ModelError,
    graph::{Assertion, Entity, Target},
};
use std::sync::Arc;
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Bytes, Object, RecordId, SurrealValue, ToSql, Value, Variables},
};

pub struct Loader {
    client: Arc<Surreal<Client>>,
}
impl Loader {
    pub fn new(client: Arc<Surreal<Client>>) -> Self {
        Self { client }
    }
    pub fn client(&self) -> &Surreal<Client> {
        &self.client
    }
    pub fn shared_client(&self) -> Arc<Surreal<Client>> {
        self.client.clone()
    }
    pub async fn install(&self, native_definitions: &str) -> Result<(), ModelError> {
        // Canonical DDL contains only finite declarations. Coarse checked batches avoid one
        // large setup transaction; executable function bodies remain intact in the final query.
        let schema = crate::schema::canonical_schema();
        let statements = schema
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>();
        for chunk in statements.chunks(32) {
            self.client
                .query(chunk.join(";") + ";")
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
        }
        self.client
            .query(native_definitions)
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        Ok(())
    }
    pub(crate) async fn insert(
        &self,
        table: &str,
        rows: Vec<Value>,
        relation: bool,
    ) -> Result<(), ModelError> {
        if rows.is_empty() {
            return Ok(());
        }
        for rows in NativeWindows::new(rows) {
        let mut bindings = Variables::new();
        bindings.insert("rows", rows?);
        let sql = format!(
            "INSERT {}INTO {table} $rows RETURN NONE",
            if relation { "RELATION " } else { "" }
        );
        self.client
            .query(sql)
            .bind(bindings)
            .await
            .map_err(|error|ModelError::codec(format!("native bulk {table} insert: {error}")))?
            .check()
            .map_err(|error|ModelError::codec(format!("native bulk {table} insert: {error}")))?;
        }
        Ok(())
    }
    pub async fn entities(&self, rows: &[Entity]) -> Result<(), ModelError> {
        for window in CanonicalWindows::new(rows) {
            let (chunk,canonical)=window?;
            self.insert("entity", entity_values(chunk,canonical)?, false).await?;
        }
        Ok(())
    }
    pub async fn assertions(&self, rows: &[Assertion]) -> Result<(), ModelError> {
        for window in CanonicalWindows::new(rows) {
            let (chunk,canonical)=window?;
            self.insert("assertion", assertion_values(chunk,canonical)?, false).await?;
        }
        Ok(())
    }
    /// Direct sealing reuses identical canonical rows and strictly inserts missing aliases.
    pub async fn ensure_entities(&self, rows:&[Entity])->Result<(),ModelError>{
        for window in CanonicalWindows::new(rows){let (chunk,canonical)=window?;self.ensure("entity",entity_values(chunk,canonical)?).await?;}
        Ok(())
    }
    pub async fn ensure_assertions(&self, rows:&[Assertion])->Result<(),ModelError>{
        for window in CanonicalWindows::new(rows){let (chunk,canonical)=window?;self.ensure("assertion",assertion_values(chunk,canonical)?).await?;}
        Ok(())
    }
    async fn ensure(&self,table:&str,rows:Vec<Value>)->Result<(),ModelError>{
        for window in NativeWindows::new(rows) {self.ensure_window(table,window?).await?;}
        Ok(())
    }
    async fn ensure_window(&self,table:&str,rows:Vec<Value>)->Result<(),ModelError>{
        let mut candidates=std::collections::BTreeMap::<String,Value>::new();
        for row in rows {
            let Value::Object(object)=&row else{return Err(ModelError::Schema("native candidate object"));};
            let id=object.get("id").ok_or(ModelError::Schema("native candidate ID"))?.to_sql();
            if let Some(previous)=candidates.insert(id,row.clone()) && !same_native_payload(&previous,&row)?{return Err(ModelError::Conflict("native same-key payload"));}
        }
        if candidates.is_empty(){return Ok(());}
        let mut bindings=Variables::new();
        bindings.insert("ids",candidates.values().map(|row|match row{Value::Object(object)=>object.get("id").cloned().expect("checked candidate ID"),_=>unreachable!()}).collect::<Vec<_>>());
        let mut response=self.client.query("SELECT * FROM $ids").bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let stored:Vec<Value>=response.take(0).map_err(ModelError::codec)?;
        for row in stored {
            let Value::Object(object)=&row else{return Err(ModelError::Schema("native existing object"));};
            let id=object.get("id").ok_or(ModelError::Schema("native existing ID"))?.to_sql();
            let expected=candidates.remove(&id).ok_or(ModelError::Conflict("unexpected native candidate"))?;
            if !same_native_payload(&expected,&row)?{return Err(ModelError::Conflict("native same-key payload"));}
        }
        self.insert(table,candidates.into_values().collect(),false).await
    }
    /// Load only after every canonical endpoint is present. Parallel roles keep distinct IDs.
    pub async fn entity_references(&self, rows: &[Entity]) -> Result<(), ModelError> {
        let mut pending=ReferenceWrites::default();
        for row in rows {
            let source=target_id(Target::Entity(row.id()));
            for (position,reference) in codec::entity_references(row).into_iter().enumerate() {
                let target=lctx_model::domain::graph::reference_target(&reference)?.0;
                let endpoint=external_value(&target)?;
                let value=edge("reference",source.clone(),target,reference.field,0,Some(u32::try_from(position).map_err(ModelError::codec)?))?;
                self.reference_write("reference",&mut pending,value,endpoint).await?;
            }
        }
        self.flush_references("reference",&mut pending).await
    }
    pub async fn assertion_references(&self, rows: &[Assertion]) -> Result<(), ModelError> {
        let mut pending=ReferenceWrites::default();
        for row in rows {
            let source=target_id(Target::Assertion(row.id()));
            for participant in &row.participants {
                let endpoint=external_value(&participant.target)?;
                let value=edge("participant",source.clone(),participant.target.clone(),participant.field.as_deref().unwrap_or(""),participant.role as i64,participant.position)?;
                self.reference_write("participant",&mut pending,value,endpoint).await?;
            }
            // Qualification/run/evidence/derivation references retain their native adjacency.
            for (position,(target,_)) in row.references()?.into_iter().enumerate() {
                if row.participants.iter().any(|participant|participant.target==target){continue;}
                let endpoint=external_value(&target)?;
                let value=edge("participant",source.clone(),target,"__reference",-1,Some(u32::try_from(position).map_err(ModelError::codec)?))?;
                self.reference_write("participant",&mut pending,value,endpoint).await?;
            }
        }
        self.flush_references("participant",&mut pending).await
    }
    async fn reference_write(&self,table:&str,pending:&mut ReferenceWrites,value:Value,endpoint:Option<Value>)->Result<(),ModelError>{
        let bytes=native_bytes(&value).saturating_add(endpoint.as_ref().map_or(0,native_bytes));
        let limits=lctx_model::domain::batching::TransferLimits::default();
        require_row(bytes,limits)?;
        if !pending.edges.is_empty()&&(pending.edges.len()>=limits.rows||pending.bytes.saturating_add(bytes)>limits.bytes){self.flush_references(table,pending).await?;}
        if let Some(endpoint)=endpoint{pending.endpoints.push(endpoint);}
        pending.edges.push(value);pending.bytes=pending.bytes.saturating_add(bytes);
        Ok(())
    }
    async fn flush_references(&self,table:&str,pending:&mut ReferenceWrites)->Result<(),ModelError>{
        // Bulk strict endpoint admission precedes every enforced relation batch.
        self.ensure("external",std::mem::take(&mut pending.endpoints)).await?;
        self.insert(table,std::mem::take(&mut pending.edges),true).await?;
        pending.bytes=0;Ok(())
    }
    pub async fn original_stream(&self,source:ContentHash,content:ContentHash,length:u64,input:&mut impl std::io::Read)->Result<(),ModelError>{
        self.original_stream_inner(source,content,length,input,false).await
    }
    /// Direct sealing compares exact existing original bytes and inserts missing chunks only.
    pub async fn ensure_original_stream(&self,source:ContentHash,content:ContentHash,length:u64,input:&mut impl std::io::Read)->Result<(),ModelError>{
        self.original_stream_inner(source,content,length,input,true).await
    }
    async fn original_stream_inner(

        &self,
        source: ContentHash,
        content: ContentHash,
        length: u64,
        input: &mut impl std::io::Read,
        reuse: bool,
    ) -> Result<(), ModelError> {
        let source_key = source.hex();
        let source = RecordId::new("original", source_key.clone());
        let mut header = Object::new();
        header.insert("id", source.clone());
        header.insert("content", content.hex());
        header.insert(
            "byte_len",
            i64::try_from(length).map_err(ModelError::codec)?,
        );
        self.original_rows("original",vec![Value::Object(header)],reuse).await?;
        let mut hasher = lctx_model::domain::ContentHasher::default();
        let mut position = 0u64;
        let mut buffer = vec![0u8; 65536];
        let mut pending = Vec::new();
        loop {
            let mut count = 0;
            while count < buffer.len() {
                let next = input
                    .read(&mut buffer[count..])
                    .map_err(ModelError::codec)?;
                if next == 0 {
                    break;
                }
                count += next;
            }
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
            let mut row = Object::new();
            row.insert(
                "id",
                RecordId::new("original_chunk", format!("{source_key}_{position}")),
            );
            row.insert("source", source.clone());
            row.insert("start", i64::try_from(position).map_err(ModelError::codec)?);
            row.insert("bytes", Bytes::from(buffer[..count].to_vec()));
            row.insert("content", ContentHash::of(&buffer[..count]).hex());
            pending.push(Value::Object(row));
            if pending.len() == 64 {
                self.original_rows("original_chunk",std::mem::take(&mut pending),reuse).await?;
            }
            position += count as u64;
        }
        self.original_rows("original_chunk",pending,reuse).await?;
        if position != length || hasher.finish() != content {
            return Err(ModelError::Conflict("original bytes"));
        }
        Ok(())
    }
    async fn original_rows(&self,table:&str,rows:Vec<Value>,reuse:bool)->Result<(),ModelError>{
        if reuse {self.ensure(table,rows).await}else{self.insert(table,rows,false).await}
    }

}
fn external_value(target:&Target)->Result<Option<Value>,ModelError>{
    if !matches!(target,Target::External{..}){return Ok(None);}
    let mut object=Object::new();object.insert("id",target_id(target.clone()));
    object.insert("canonical",Bytes::from(serde_json::to_vec(target).map_err(ModelError::codec)?));
    Ok(Some(Value::Object(object)))
}
fn graph_value(table:&str,id:ContentHash,content:ContentHash,kind:i64,subtype:Option<i16>,canonical:Vec<u8>,view:codec::RecordView)->Result<Value,ModelError>{
    let mut object=Object::new();
    object.insert("id",RecordId::new(table,id.hex()));
    object.insert("semantic_type",view.semantic_type.clone());object.insert("semantic_key",view.semantic_key);
    object.insert("kind",kind);object.insert("subtype",subtype.map(Value::from_t).unwrap_or(Value::Null));
    object.insert("content",content.hex());object.insert("canonical",Bytes::from(canonical));
    crate::reconciliation::add_scope_fields(&mut object,&view.body,&view.semantic_type)?;
    object.insert("body",view.body);Ok(Value::Object(object))
}
fn entity_values(rows:&[Entity],canonical:Vec<Vec<u8>>)->Result<Vec<Value>,ModelError>{
    rows.iter().zip(codec::entity_views(rows)?).zip(canonical).map(|((row,view),canonical)|{
        row.validate()?;
        graph_value("entity",row.id().0,row.content(),row.kind() as i64,row.subtype(),canonical,view)
    }).collect()
}
fn assertion_values(rows:&[Assertion],canonical:Vec<Vec<u8>>)->Result<Vec<Value>,ModelError>{
    rows.iter().zip(codec::assertion_views(rows)?).zip(canonical).map(|((row,view),canonical)|{
        row.validate()?;
        graph_value("assertion",row.id().0,row.content(),row.kind as i64,None,canonical,view)
    }).collect()
}
pub(crate) fn edge(
    table: &str,
    source: RecordId,
    target: Target,
    field: &str,
    role: i64,
    position: Option<u32>,
) -> Result<Value, ModelError> {
    let mut sink = KeySink::new("native-graph-role/v1");
    table.to_string().encode(&mut sink);
    sink.part(
        b"source",
        &serde_json::to_vec(&source).map_err(ModelError::codec)?,
    );
    target.encode(&mut sink);
    field.to_string().encode(&mut sink);
    role.encode(&mut sink);
    position.map(i64::from).encode(&mut sink);
    let mut obj = Object::new();
    obj.insert("id", RecordId::new(table, sink.finish().hex()));
    obj.insert("in", source);
    obj.insert("out", target_id(target));
    obj.insert("field", field.to_string());
    obj.insert("role", role);
    obj.insert(
        "position",
        position.map(Value::from_t).unwrap_or(Value::Null),
    );
    Ok(Value::Object(obj))
}
pub fn json_value(value: serde_json::Value) -> Result<Value, ModelError> {
    Ok(match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(v) => v.into_value(),
        serde_json::Value::String(v) => v.into_value(),
        serde_json::Value::Number(v) => {
            if let Some(v) = v.as_i64() {
                v.into_value()
            } else if let Some(v) = v.as_u64() {
                v.into_value()
            } else {
                v.as_f64()
                    .ok_or(ModelError::Schema("native float"))?
                    .into_value()
            }
        }
        serde_json::Value::Array(v) => v
            .into_iter()
            .map(json_value)
            .collect::<Result<Vec<_>, _>>()?
            .into_value(),
        serde_json::Value::Object(v) => {
            let mut obj = Object::new();
            for (k, v) in v {
                obj.insert(k, json_value(v)?);
            }
            Value::Object(obj)
        }
    })
}

#[derive(Default)]
struct ReferenceWrites {edges:Vec<Value>,endpoints:Vec<Value>,bytes:usize}
/// Conservative retained native payload estimate; transport encoding remains SDK-owned.
fn same_native_payload(left:&Value,right:&Value)->Result<bool,ModelError>{
    Ok(serde_json::to_vec(left).map_err(ModelError::codec)?==serde_json::to_vec(right).map_err(ModelError::codec)?)
}
pub(crate) fn native_bytes(value:&Value)->usize {
    size_of::<Value>().saturating_add(match value {
        Value::String(value)=>value.len(),
        Value::Bytes(value)=>value.len(),
        Value::Array(values)=>values.iter().map(native_bytes).fold(0,usize::saturating_add),
        Value::Object(values)=>values.iter().map(|(key,value)|key.len().saturating_add(32).saturating_add(native_bytes(value))).fold(0,usize::saturating_add),
        Value::RecordId(value)=>value.to_sql().len(),
        _=>0,
    })
}
fn require_row(bytes:usize,limits:lctx_model::domain::batching::TransferLimits)->Result<(),ModelError>{
    if bytes>limits.max_row {return Err(ModelError::Limit{owner:"native transfer",limit:"row bytes",observed:bytes,bound:limits.max_row});}Ok(())
}
/// Indexed native writes have a smaller transaction target than logical Arrow transfers.
/// Compiler, loading and retained-state import share these row/byte/max-row bounds.
pub(crate) const NATIVE_WINDOW_ROWS:usize=128;
pub(crate) struct NativeWindows {rows:std::iter::Peekable<std::vec::IntoIter<Value>>,limits:lctx_model::domain::batching::TransferLimits}
impl NativeWindows {pub(crate) fn new(rows:Vec<Value>)->Self{Self{rows:rows.into_iter().peekable(),limits:lctx_model::domain::batching::TransferLimits{rows:NATIVE_WINDOW_ROWS,..Default::default()}}}}
impl Iterator for NativeWindows {
    type Item=Result<Vec<Value>,ModelError>;
    fn next(&mut self)->Option<Self::Item>{
        let mut output=Vec::new();let mut bytes=0usize;
        while let Some(row)=self.rows.peek(){
            let size=native_bytes(row);
            if let Err(error)=require_row(size,self.limits){self.rows.next();return Some(Err(error));}
            if !output.is_empty()&&(output.len()>=self.limits.rows||bytes.saturating_add(size)>self.limits.bytes){break;}
            bytes=bytes.saturating_add(size);output.push(self.rows.next().expect("peeked row"));
        }
        if output.is_empty(){None}else{Some(Ok(output))}
    }
}
/// Canonical bytes are generated once and reused by the physical row. A single lookahead
/// bounds the next batch before grouped Arrow bodies and native Values are allocated.
fn canonical_native_bytes(bytes:&[u8])->usize{
    let slots=bytes.iter().filter(|&&byte|matches!(byte,b','|b':'|b'['|b'{')).count();
    bytes.len().saturating_mul(4).saturating_add(slots.saturating_mul(128)).saturating_add(1024)
}
struct CanonicalWindows<'a,T> {rows:&'a[T],position:usize,pending:Option<Vec<u8>>,limits:lctx_model::domain::batching::TransferLimits}
impl<'a,T> CanonicalWindows<'a,T>{fn new(rows:&'a[T])->Self{Self{rows,position:0,pending:None,limits:Default::default()}}}
impl<'a,T:serde::Serialize> Iterator for CanonicalWindows<'a,T>{
    type Item=Result<(&'a[T],Vec<Vec<u8>>),ModelError>;
    fn next(&mut self)->Option<Self::Item>{
        if self.position==self.rows.len(){return None;}
        let start=self.position;let mut canonical=Vec::new();let mut bytes=0usize;
        while self.position<self.rows.len(){
            let row=match self.pending.take().map(Ok).unwrap_or_else(||serde_json::to_vec(&self.rows[self.position]).map_err(ModelError::codec)){Ok(row)=>row,Err(error)=>return Some(Err(error))};
            if let Err(error)=require_row(row.len(),self.limits){self.position+=1;return Some(Err(error));}
            // Slots expand compact JSON arrays. Counting delimiters (even in strings)
            // conservatively allows container entries without parsing another rich tree.
            let size=canonical_native_bytes(&row);
            if !canonical.is_empty()&&(canonical.len()>=self.limits.rows||bytes.saturating_add(size)>self.limits.bytes){self.pending=Some(row);break;}
            bytes=bytes.saturating_add(size);canonical.push(row);self.position+=1;
        }
        Some(Ok((&self.rows[start..self.position],canonical)))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn canonical_windows_reuse_bytes_and_flush_before_body_lowering(){
        let rows=vec!["a".repeat(8),"b".repeat(8),"c".repeat(8)];
        let mut windows=CanonicalWindows::new(&rows);windows.limits=lctx_model::domain::batching::TransferLimits{rows:2,bytes:4096,max_row:8192};
        let first=windows.next().unwrap().unwrap();let second=windows.next().unwrap().unwrap();
        assert_eq!(first.0,&rows[..2]);assert_eq!(second.0,&rows[2..]);assert!(windows.next().is_none());
        assert_eq!(first.1,rows[..2].iter().map(|row|serde_json::to_vec(row).unwrap()).collect::<Vec<_>>());
        let mut windows=CanonicalWindows::new(&rows);windows.limits.bytes=1100;
        assert!(windows.all(|window|window.unwrap().0.len()==1));
    }
    #[test]
    fn native_windows_allow_one_large_row_and_refuse_beyond_singleton_limit(){
        let rows=vec![Value::from_t("small"),Value::Bytes(Bytes::from(vec![1;128])),Value::from_t("last")];
        let target=native_bytes(&rows[0]);
        let mut windows=NativeWindows::new(rows);windows.limits=lctx_model::domain::batching::TransferLimits{rows:2,bytes:target,max_row:4096};
        assert_eq!(windows.next().unwrap().unwrap().len(),1);
        assert!(matches!(&windows.next().unwrap().unwrap()[0],Value::Bytes(bytes) if bytes.len()==128));
        assert_eq!(windows.next().unwrap().unwrap().len(),1);assert!(windows.next().is_none());
        let mut windows=NativeWindows::new(vec![Value::Bytes(Bytes::from(vec![1;1025]))]);windows.limits.max_row=1024;
        assert!(matches!(windows.next().unwrap(),Err(ModelError::Limit{..})));
    }
}
