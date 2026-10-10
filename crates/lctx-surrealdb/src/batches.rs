//! Finite canonical graph records back into request-scoped model kernel batches.
use crate::reader::{NativeReader, canonical_error};
use arrow_array::RecordBatch;
use lctx_model::domain::{
    ModelError, Record, Batch,
    graph::{Assertion, Entity},
    resources::{Reservation, ResourceBudget},
};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Bytes, Variables};
mod charged_json;
use std::io::Write;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalNode {
    pub node_kind: String,
    pub canonical: Bytes,
}
/// Charges both canonical transport and encoded kernel rows until request scope is dropped.
pub struct CanonicalBatches {
    pub batches: Vec<(&'static str, RecordBatch)>,
    _charges: Vec<Box<dyn Reservation>>,
    _metadata: Box<dyn Reservation>,
}
impl CanonicalBatches {
    fn new(budget:&ResourceBudget)->Result<Self,ModelError>{Ok(Self {batches:Vec::new(),_charges:Vec::new(),_metadata:budget.reserve("native-canonical-batch-metadata",0)?})}
    fn grow_metadata(&mut self,count:usize)->Result<(),ModelError>{
        self._metadata.try_resize(count.max(1).next_power_of_two().max(4).saturating_mul(size_of::<(&'static str,RecordBatch)>()+size_of::<Box<dyn Reservation>>()))
    }
    fn append(&mut self,mut other:Self)->Result<(),ModelError>{
        self.grow_metadata(self.batches.len().saturating_add(other.batches.len()))?;
        self.batches.append(&mut other.batches);self._charges.append(&mut other._charges);Ok(())
    }
}
impl<Context> NativeReader<Context> {
    /// Hydrate already admitted exact physical IDs with finite streamed point reads.
    pub async fn canonical_point_batches(&self, ids: &[surrealdb::types::RecordId], types: &[String], budget: &ResourceBudget) -> Result<CanonicalBatches, ModelError> {
        let mut combined=CanonicalBatches::new(budget)?;
        for window in ids.chunks(128) {
            let pointer_bytes=window.iter().try_fold(0usize,|bytes,id| {
                let key=match &id.key {surrealdb::types::RecordIdKey::String(key)=>key.len(),surrealdb::types::RecordIdKey::Number(_)|surrealdb::types::RecordIdKey::Uuid(_)=>0,_=>return Err(ModelError::Schema("canonical exact scalar identity"))};
                bytes.checked_add(size_of::<surrealdb::types::Value>()+size_of::<surrealdb::types::RecordId>()+id.table.len()+key).ok_or(ModelError::Schema("canonical pointer bytes overflow"))
            })?;
            let type_bytes=types.iter().try_fold(0usize,|bytes,name|bytes.checked_add(size_of::<surrealdb::types::Value>()+size_of::<String>()+name.len()).ok_or(ModelError::Schema("canonical type bytes overflow")))?;
            let _bindings=budget.reserve("native-canonical-point-bindings",pointer_bytes.checked_add(type_bytes).and_then(|n|n.checked_add(10+2*(32+size_of::<surrealdb::types::Value>()))).ok_or(ModelError::Schema("canonical binding bytes overflow"))?)?;
            let mut variables=Variables::new();variables.insert("nodes",window.to_vec());variables.insert("types",types.to_vec());
            let rows=self.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec!["SELECT record::table(id) AS node_kind,canonical FROM $nodes WHERE semantic_type IN $types".into()])?)?.with_row_bytes(lctx_model::domain::resources::MAX_ROW_BYTES);
            let decoded=Self::canonical_row_batches(rows,budget).await?;
            combined.append(decoded)?;
        }
        combined.batches.sort_by_key(|(name,_)|*name);
        Ok(combined)
    }
    /// Transfer checked canonical rows into typed kernel batches without a complete server array.
    /// The caller supplies the exact selected source; this decoder does not choose its scope.
    pub async fn canonical_row_batches(mut rows:crate::reader::NativeRows,budget:&ResourceBudget)->Result<CanonicalBatches,ModelError> {
        use surrealdb::types::{SurrealValue,Value};
        let result=async {
            let mut combined=CanonicalBatches::new(budget)?;
            loop {
                let mut input=budget.reserve("native-canonical-stream-window",128*size_of::<CanonicalNode>())?;
                let mut nodes=Vec::with_capacity(128);
                while nodes.len()<128 {
                    let Some(row)=rows.next().await? else{break;};
                    input.try_resize(input.size().saturating_add(crate::loader::native_bytes(&row)))?;
                    let Value::Object(mut row)=row else{return Err(ModelError::Schema("canonical native object"));};
                    let node_kind=String::from_value(row.remove("node_kind").ok_or(ModelError::Schema("canonical node kind"))?).map_err(ModelError::codec)?;
                    let canonical=Bytes::from_value(row.remove("canonical").ok_or(ModelError::Schema("canonical node bytes"))?).map_err(ModelError::codec)?;
                    if !row.is_empty(){return Err(ModelError::Schema("canonical native unknown field"));}
                    nodes.push(CanonicalNode {node_kind,canonical});
                }
                if nodes.is_empty(){break;}
                let decoded=Self::canonical_nodes(nodes,budget)?;
                combined.append(decoded)?;
                tokio::task::yield_now().await;
            }
            combined.batches.sort_by_key(|(name,_)|*name);Ok(combined)
        }.await;
        let mut terminal=lctx_model::domain::completion::Completion::default();terminal.step("canonical stream drainage",rows.drain_transport().await);
        lctx_model::domain::completion::complete(result,terminal)
    }
    fn canonical_nodes(nodes: Vec<CanonicalNode>, budget: &ResourceBudget) -> Result<CanonicalBatches, ModelError> {
        let entity_count=nodes.iter().filter(|node|node.node_kind=="entity").count();
        let assertion_count=nodes.iter().filter(|node|node.node_kind=="assertion").count();
        let _inline=budget.reserve("native-canonical-typed-vectors",entity_count.checked_mul(size_of::<Entity>()).and_then(|bytes|assertion_count.checked_mul(size_of::<Assertion>()).and_then(|n|bytes.checked_add(n))).ok_or(ModelError::Schema("canonical typed vector overflow"))?)?;
        let mut entities=Vec::with_capacity(entity_count);
        let mut assertions=Vec::with_capacity(assertion_count);
        let _metadata=budget.reserve("native-canonical-charge-metadata",nodes.len().saturating_mul(size_of::<Box<dyn Reservation>>()))?;
        let mut decoded_charges=Vec::with_capacity(nodes.len());
        for node in nodes {
            match node.node_kind.as_str() {
                "entity"=> {let (row,charge)=charged_json::from_slice::<Entity>(&node.canonical,budget).map_err(canonical_error)?;row.validate().map_err(canonical_error)?;entities.push(row);decoded_charges.push(charge);},
                "assertion"=> {let (row,charge)=charged_json::from_slice::<Assertion>(&node.canonical,budget).map_err(canonical_error)?;row.validate().map_err(canonical_error)?;assertions.push(row);decoded_charges.push(charge);},
                _=>return Err(ModelError::Schema("native canonical node kind")),
            }
        }
        let model=lctx_model::domain::model()?;
        let mut output=CanonicalBatches::new(budget)?;
        macro_rules! entity_batches {($($variant:ident:$ty:ty,)*)=>{$({
            let count=entities.iter().filter(|entity|matches!(entity,Entity::$variant(_))).count();
            if count>0 {
                let held=entities.iter().filter_map(|entity|match entity {Entity::$variant(row)=>Some(row.row_bytes()),_=>None}).try_fold(0usize,|bytes,row|bytes.checked_add(row)).ok_or(ModelError::Schema("canonical record clone overflow"))?;
                let charge=budget.reserve("native-canonical-record-clone",held)?;
                let mut rows=Vec::with_capacity(count);
                for entity in &entities {if let Entity::$variant(row)=entity {rows.push(row.clone());}}
                let (batch,charge)=Batch::<$ty>::with_reservation(&model,rows,charge)?.into_arrow()?;
                push_batch(&mut output,<$ty>::NAME,batch,charge)?;
            }
        })*};}
        lctx_model::graph_entity_records!(entity_batches);
        macro_rules! assertion_batches {($($variant:ident:$ty:ty,)*)=>{$({
            let count=assertions.iter().filter(|row|row.source.as_ref().is_some_and(|source|source.domain()==<$ty>::NAME)).count();
            if count>0 {
                let mut charge=budget.reserve("native-canonical-record-decode",count.saturating_mul(size_of::<$ty>()))?;
                let mut rows=Vec::with_capacity(count);
                for assertion in &assertions {if assertion.source.as_ref().is_some_and(|source|source.domain()==<$ty>::NAME) {
                    let (row,next)=assertion_row::<$ty>(assertion,budget,charge)?;charge=next;rows.push(row);
                }}
                let held=rows.iter().try_fold(count.saturating_mul(size_of::<$ty>()),|bytes,row|bytes.checked_add(lctx_model::domain::HeapSize::heap_bytes(row))).ok_or(ModelError::Schema("canonical record decoded overflow"))?;
                charge.try_resize(held)?;
                let (batch,charge)=Batch::<$ty>::with_reservation(&model,rows,charge)?.into_arrow()?;
                push_batch(&mut output,<$ty>::NAME,batch,charge)?;
            }
        })*};}
        lctx_model::graph_assertion_records!(assertion_batches);
        if output.batches.iter().map(|(_,batch)|batch.num_rows()).sum::<usize>()!=entities.len()+assertions.len() {return Err(ModelError::Schema("native selected semantic batch inventory"));}
        output.batches.sort_by_key(|(name,_)|*name);
        Ok(output)
    }
}
fn push_batch(output:&mut CanonicalBatches,name:&'static str,batch:RecordBatch,charge:Box<dyn Reservation>)->Result<(),ModelError>{
    output.grow_metadata(output.batches.len()+1)?;
    output.batches.push((name,batch));output._charges.push(charge);Ok(())
}
struct JsonWriter { bytes:Vec<u8>, charge:Box<dyn Reservation>, failure:Option<ModelError> }
impl Write for JsonWriter {
    fn write(&mut self,raw:&[u8])->std::io::Result<usize>{
        let end=self.bytes.len().checked_add(raw.len()).ok_or_else(||std::io::Error::other("canonical JSON encoding overflow"))?;
        if end>self.bytes.capacity(){let old=self.bytes.capacity();let capacity=end.max(old.saturating_add(old.clamp(64,8192)));if let Err(error)=self.charge.try_resize(capacity){self.failure=Some(error);return Err(std::io::Error::other("canonical JSON allocation admission"));}self.bytes.try_reserve_exact(capacity-self.bytes.len()).map_err(std::io::Error::other)?;}
        self.bytes.extend_from_slice(raw);Ok(raw.len())
    }
    fn flush(&mut self)->std::io::Result<()> {Ok(())}
}
fn assertion_row<R:Record+serde::de::DeserializeOwned>(assertion:&Assertion,budget:&ResourceBudget,charge:Box<dyn Reservation>)->Result<(R,Box<dyn Reservation>),ModelError>{
    use lctx_model::domain::graph::AssertionValue;
    let source=assertion.source.as_ref().ok_or(ModelError::Schema("assertion typed source"))?;
    if source.domain()!=R::NAME {return Err(ModelError::Schema("assertion typed source domain"));}
    let mut encoded=JsonWriter {bytes:Vec::new(),charge:budget.reserve("native-canonical-assertion-json",0)?,failure:None};
    if let Err(error)=serde_json::to_writer(&mut encoded,&assertion.value) {return Err(encoded.failure.take().unwrap_or_else(||ModelError::codec(error)));}
    let depth=match &assertion.value {
        AssertionValue::Acquisition(_)=>1,
        AssertionValue::Native(_)|AssertionValue::Analysis(_)|AssertionValue::Support(_)|AssertionValue::Provenance(_)|AssertionValue::Membership(_)|AssertionValue::Claim(_)=>2,
        _=>return Err(ModelError::Schema("assertion has no typed record payload")),
    };
    let (row,charge)=charged_json::from_wrapped_slice::<R>(&encoded.bytes,depth,budget,charge).map_err(canonical_error)?;
    drop(encoded);
    row.validate().map_err(canonical_error)?;
    if row.id().bytes()!=source.bytes(){return Err(ModelError::Conflict("canonical assertion record key"));}
    Ok((row,charge))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_batches_retain_only_encoded_arrow_owner() {
        use lctx_model::domain::{ContentHash,input::InputRevision};
        let budget=ResourceBudget::fixed(16*1024*1024).unwrap();
        let entity=Entity::Capture(InputRevision {manifest:ContentHash::of(b"canonical owner")});
        let canonical=serde_json::to_vec(&entity).unwrap();
        let batches=NativeReader::<()>::canonical_nodes(vec![CanonicalNode {node_kind:"entity".into(),canonical:canonical.into()}],&budget).unwrap();
        assert_eq!(batches.batches.len(),1);
        assert_eq!(batches.batches[0].0,InputRevision::NAME);
        assert_eq!(batches.batches[0].1.num_rows(),1);
        let arrow=batches.batches.iter().map(|(_,batch)|batch.get_array_memory_size()).sum::<usize>();
        assert_eq!(batches._charges.iter().map(|charge|charge.size()).sum::<usize>(),arrow);
        assert_eq!(budget.reserved(),arrow+batches._metadata.size());
        drop(batches);assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn canonical_json_writer_preserves_resource_refusal() {
        let budget=ResourceBudget::fixed(1).unwrap();
        let mut writer=JsonWriter {bytes:Vec::new(),charge:budget.reserve("test",0).unwrap(),failure:None};
        assert!(serde_json::to_writer(&mut writer,&"a").is_err());
        assert!(matches!(writer.failure,Some(ModelError::Resource {..})));
        assert!(writer.bytes.is_empty());
    }
}
