//! One fixed native realization, bounded admission, and closed Rust operation transport.
use lctx_model::domain::{*,resources::ResourceBudget,serving::*};
use lctx_surrealdb::{NativeReader,RecordSelection};
use serde::{Deserialize,Serialize};
use std::{sync::Arc,time::Duration};
use tokio::sync::Semaphore;
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryVector {pub spec:ContentHash,pub input:ContentHash,pub vector:Vec<f32>}
pub struct NativeService {reader:NativeReader,limits:ResourceLimits,shared:ResourceBudget,queries:Arc<Semaphore>,cpu:Arc<Semaphore>}
impl NativeService {
 pub fn new(reader:NativeReader,limits:ResourceLimits)->Result<Self,ModelError>{
  limits.validate()?;
  Ok(Self{shared:ResourceBudget::fixed(limits.shared_bytes as usize)?,queries:Arc::new(Semaphore::new(limits.query_connections as usize)),cpu:Arc::new(Semaphore::new(limits.cpu_jobs as usize)),reader,limits})
 }
 pub fn handle(&self)->&SnapshotHandle{self.reader.handle()}
 pub async fn execute(&self,tool:&str,raw:&str)->Result<String,WireError>{self.run(tool,raw,None,false,self.limits.request_deadline_ms).await}
 pub async fn execute_unavailable(&self,tool:&str,raw:&str)->Result<String,WireError>{self.run(tool,raw,None,true,self.limits.request_deadline_ms).await}
 pub async fn execute_with_vector(&self,tool:&str,raw:&str,vector:Option<QueryVector>)->Result<String,WireError>{self.run(tool,raw,vector,false,self.limits.request_deadline_ms).await}
 /// Execute within the transport's remaining request deadline, including native admission.
 pub async fn execute_for(&self,tool:&str,raw:&str,vector:Option<QueryVector>,unavailable:bool,remaining_ms:u64)->Result<String,WireError>{self.run(tool,raw,vector,unavailable,remaining_ms.min(self.limits.request_deadline_ms)).await}
 async fn run(&self,tool:&str,raw:&str,vector:Option<QueryVector>,unavailable:bool,remaining_ms:u64)->Result<String,WireError>{
  if remaining_ms==0{return Err(WireError::ResourceRefused("request deadline".into()))}
  let request=decode_request(tool,raw,&self.limits)?;
  let deadline=tokio::time::Instant::now()+Duration::from_millis(remaining_ms);
  let admission=(tokio::time::Instant::now()+Duration::from_millis(self.limits.admission_wait_ms)).min(deadline);
  let _query=tokio::time::timeout_at(admission,self.queries.acquire()).await.map_err(|_|WireError::ResourceRefused("query admission".into()))?.map_err(|_|WireError::ResourceRefused("query service closed".into()))?;
  let _cpu=tokio::time::timeout_at(admission,self.cpu.acquire()).await.map_err(|_|WireError::ResourceRefused("CPU admission".into()))?.map_err(|_|WireError::ResourceRefused("CPU service closed".into()))?;
  let budget=ResourceBudget::scoped(&self.shared,self.limits.request_bytes as usize).map_err(failure)?;
  let _request_charge=budget.reserve("native-request-wire",raw.len().saturating_mul(2)).map_err(failure)?;
  let result=tokio::time::timeout_at(deadline,async{
   let query=match &request{Request::SearchOperations(r)=>Some(r.query.as_str()),Request::SearchEvidence(r)=>Some(r.query.as_str()),Request::SearchCapabilities(r)=>Some(r.query.as_str()),_=>None};
   if query.is_none() && vector.is_some(){return Err(WireError::Invalid("query vector supplied to a non-search operation".into()))}
   let _vector_charge=budget.reserve("native-query-vector",vector.as_ref().map_or(0,|v|v.vector.len()*4)).map_err(failure)?;
   let vector_state=if let Some(v)=&vector{
    let specs=self.reader.records::<embedding::EmbeddingSpec>(RecordSelection::Keys(vec![*Id::<embedding::EmbeddingSpec>::of(&embedding::EmbeddingSpecKey{service_hash:v.spec}).bytes()])).await.map_err(failure)?;
    let spec=specs.first().ok_or_else(||WireError::Invalid("query embedding specification is not admitted".into()))?.configuration().map_err(failure)?;
    embedding::check_vector(&v.vector,spec.dimensions).map_err(WireError::Invalid)?;
    if spec.dimensions!=1024 || embedding::value::input_hash(&spec.query_text(query.expect("validated search vector")))!=v.input{return Err(WireError::Invalid("query embedding specification or rendered input mismatch".into()))}
    VectorChannel::Available{spec:v.spec,query_vector:embedding::value::value_digest(&v.vector)}
   }else if unavailable && query.is_some(){VectorChannel::Degraded{reason:Name::new("embedding_service_unavailable")?}}else{VectorChannel::Disabled{}};
   let channels=ChannelState{lexical:query.is_some(),vector:vector_state};
   crate::pagination::validate(&request,self.handle(),&channels)?;
   let response=crate::operations::dispatch(&self.reader,&request,&channels,vector.as_ref(),&self.limits,&budget).await.map_err(failure)?;
   let len=response.json_len()?;
   if len as u64>self.limits.response_bytes(request.page().expanded){return Err(WireError::ResourceRefused("complete structured response bytes".into()))}
   let _response_charge=budget.reserve("native-complete-response",len.saturating_mul(2)).map_err(failure)?;
   let bytes=response.to_json()?;
   if tokio::time::Instant::now()>=deadline{return Err(WireError::ResourceRefused("request deadline".into()))}
   Ok(bytes)
  }).await.map_err(|_|WireError::ResourceRefused("request deadline".into()))?;
  result
 }
}
fn failure(error:ModelError)->WireError{match error{ModelError::Resource{..}|ModelError::Limit{..}=>WireError::ResourceRefused(error.to_string()),_=>WireError::Invalid(error.to_string())}}
