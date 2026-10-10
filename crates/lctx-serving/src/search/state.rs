//! Request-owned search buffers. A returned frontier keeps its reservation with its rows.
use super::*;
use lctx_model::domain::{charged::StateCharge,resources::ResourceBudget};

#[derive(Debug)]
pub struct SearchRows<T> { pub(super) rows:Vec<T>,pub(super) charge:StateCharge }
impl<T> std::ops::Deref for SearchRows<T>{type Target=[T];fn deref(&self)->&[T]{&self.rows}}
impl<'a,T> IntoIterator for &'a SearchRows<T>{type Item=&'a T;type IntoIter=std::slice::Iter<'a,T>;fn into_iter(self)->Self::IntoIter{self.rows.iter()}}
impl<T> SearchRows<T>{
    pub(crate) fn new(budget:&ResourceBudget)->Self{Self{rows:Vec::new(),charge:StateCharge::new(budget,"native-search-frontier")}}
    pub(super) fn push(&mut self,row:T,heap:usize)->Result<(),ModelError>{
        let additional=if self.rows.len()==self.rows.capacity(){self.rows.capacity().max(4)}else{0};
        self.charge.grow(additional.checked_mul(size_of::<T>()).and_then(|n|n.checked_add(heap)).ok_or(ModelError::Schema("search frontier allocation overflow"))?)?;
        self.rows.reserve_exact(additional);self.rows.push(row);Ok(())
    }
    pub(crate) fn append(&mut self,mut other:Self)->Result<(),ModelError>{
        self.charge.grow(other.charge.reserved().checked_add(other.rows.len().checked_mul(size_of::<T>()).ok_or(ModelError::Schema("search aggregate overflow"))?).ok_or(ModelError::Schema("search aggregate overflow"))?)?;
        self.rows.reserve_exact(other.rows.len());self.rows.append(&mut other.rows);Ok(())
    }
    pub(crate) fn retain(&mut self,keep:impl FnMut(&T)->bool){self.rows.retain(keep);}
    pub(super) fn map<U>(self,f:impl FnMut(T)->Result<U,ModelError>)->Result<SearchRows<U>,ModelError>{self.map_with_heap(0,f)}
    // The mapping declares newly allocated per-row heap before any row is cloned.
    pub(super) fn map_with_heap<U>(mut self,heap_per_row:usize,mut f:impl FnMut(T)->Result<U,ModelError>)->Result<SearchRows<U>,ModelError>{
        self.charge.grow(size_of::<U>().checked_add(heap_per_row).and_then(|bytes|self.rows.len().checked_mul(bytes)).ok_or(ModelError::Schema("search mapped frontier overflow"))?)?;
        let mut rows=Vec::with_capacity(self.rows.len());for row in self.rows{rows.push(f(row)?);}
        Ok(SearchRows{rows,charge:self.charge})
    }
}
pub(super) fn id_heap(id:&RecordId)->Result<usize,ModelError>{
    let key=match &id.key{surrealdb::types::RecordIdKey::String(value)=>value.len(),surrealdb::types::RecordIdKey::Number(_)|surrealdb::types::RecordIdKey::Uuid(_)=>0,_=>return Err(ModelError::Schema("search scalar physical identity"))};
    Ok(id.table.len()+key)
}
pub(super) fn native_heap(value:&Value)->Result<usize,ModelError>{Ok(match value{
    Value::String(value)=>value.capacity(),Value::Bytes(value)=>value.len(),Value::RecordId(value)=>id_heap(value)?,
    Value::Array(values)=>values.iter().try_fold(values.capacity()*size_of::<Value>(),|bytes,value|native_heap(value).map(|heap|bytes.saturating_add(heap)))?,
    Value::Object(values)=>object_heap(values)?,_=>0,
})}
pub(super) fn object_heap(values:&Object)->Result<usize,ModelError>{values.iter().try_fold(0usize,|bytes,(key,value)|native_heap(value).map(|heap|bytes.saturating_add(size_of::<String>()+size_of::<Value>()+32+key.capacity()+heap)))}
pub(super) fn admit_object(charge:&mut StateCharge,row:&Object)->Result<(),ModelError>{charge.grow(size_of::<Object>()+object_heap(row)?)}
pub(super) async fn values<C>(reader:&NativeReader<C>,query:lctx_surrealdb::prepared::PreparedQuery,budget:&ResourceBudget)->Result<SearchRows<Object>,ModelError>{
    let mut stream=reader.stream_prepared(query)?;
    let outcome=async{let mut rows=SearchRows::new(budget);while let Some(row)=stream.next().await?{let row=object(row)?;let heap=object_heap(&row)?;rows.push(row,heap)?;}Ok(rows)}.await;
    let mut terminal=lctx_model::domain::completion::Completion::default();terminal.step("search point result drainage",stream.drain_transport().await);lctx_model::domain::completion::complete(outcome,terminal)
}

/// Resolve only exact selected nominal keys, then use the charged canonical conversion.
pub(super) async fn records<R:Record+serde::de::DeserializeOwned,C>(reader:&NativeReader<C>,keys:Vec<[u8;16]>,budget:&ResourceBudget)->Result<SearchRows<R>,ModelError>{
    let mut output=SearchRows::new(budget);
    for keys in keys.chunks(POINT_BATCH){
        let mut scratch=StateCharge::new(budget,"native-search-record-keys");scratch.grow(keys.len()*(size_of::<String>()+32))?;
        let mut vars=Variables::new();vars.insert("type",R::NAME.to_owned());vars.insert("keys",keys.iter().map(hex::encode).collect::<Vec<_>>());
        let reader = reader.with_request_budget_clone(budget);
        let mut selected=reader.record_stream_candidates::<R>(lctx_surrealdb::prepared::PreparedQuery::new(vars,vec![],vec!["SELECT id FROM entity WITH INDEX semantic_key WHERE semantic_type=$type AND semantic_key IN $keys".into()])?,"semantic_key")?;
        let result=async{
            while let Some(row)=selected.next().await? { output.charge.grow(row.heap_bytes())?; output.push(row,0)?; }
            Ok(())
        }.await;
        let mut terminal=lctx_model::domain::completion::Completion::default();terminal.step("search exact record drainage",selected.drain_transport().await);lctx_model::domain::completion::complete(result,terminal)?;
    }Ok(output)
}
