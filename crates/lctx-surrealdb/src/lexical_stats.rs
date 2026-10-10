//! Immutable exact-view lexical statistics. FULLTEXT is nomination, never corpus authority.
use crate::{Loader, NativeReader, derived_search::selected_occurrence_predicate, ordered_rows::{SortedRows,OrderedRows}};
use crate::surrealdb::types::{Value,Object,RecordId,RecordIdKey,SurrealValue};
use lctx_model::domain::{ContentHash,KeySink,ModelError,completion::{Completion,complete},resources::{ResourceBudget,DEFAULT_MEMORY_BYTES}};
use std::collections::BTreeMap;
const TABLES:[&str;4]=["lexical_document","lexical_member","lexical_term","lexical_corpus"];
const FAMILIES:[&str;4]=["search_api_options","search_documentation_deployment","search_scenario","search_source"];
pub fn analyzer_identity()->ContentHash {ContentHash::of(b"lctx-discovery/v2:class,camel;lowercase")}
pub fn scope_identity<Context>(reader:&NativeReader<Context>)->Result<ContentHash,ModelError>{
 let vars=reader.view_bindings();let Some(Value::Array(views))=vars.get("lctx_views") else{return Err(ModelError::Conflict("lexical statistics require exact view scope"));};
 let mut ids=views.iter().map(|v|match v {Value::RecordId(id) if id.table.as_str()=="compiler_view"=>match &id.key {RecordIdKey::String(key)=>Ok(key.clone()),_=>Err(ModelError::Schema("lexical view identity"))},_=>Err(ModelError::Schema("lexical view identity"))}).collect::<Result<Vec<_>,_>>()?;
 ids.sort();ids.dedup();let mut key=KeySink::new("native-lexical-scope/v1;distinct-query-terms;view-family-bm25");key.part(b"analyzer",&analyzer_identity().0);for id in ids{key.part(b"view",id.as_bytes());}Ok(key.finish())
}
pub fn corpus_id(scope:ContentHash,family:i16)->RecordId {RecordId::new("lexical_corpus",format!("{}:{family}",scope.hex()))}
pub fn definitions()->String {
 let mut sql=String::new();
 for (table,fields) in [
 ("lexical_document",vec![("length","int"),("terms","array<object>"),("terms.*.term","string"),("terms.*.tf","int"),("terms.*.dl","int")]),
 ("lexical_member",vec![("scope","string"),("family","int"),("source","record<search_api_options|search_documentation_deployment|search_scenario|search_source>"),("in","record<lexical_document>"),("out","record<lexical_corpus>")]),
 ("lexical_term",vec![("scope","string"),("family","int"),("term","string"),("df","int")]),
 ("lexical_corpus",vec![("scope","string"),("family","int"),("documents","int"),("total_length","int")]),
 ] {sql.push_str(&format!("DEFINE TABLE {table} SCHEMAFULL;\n"));for(name,kind)in fields {sql.push_str(&format!("DEFINE FIELD {name} ON {table} TYPE {kind};\n"));}}
 sql.push_str("DEFINE INDEX scoped_document ON lexical_member FIELDS scope,family,source UNIQUE;\nDEFINE INDEX scoped_term ON lexical_term FIELDS scope,family,term UNIQUE;\nDEFINE INDEX scoped_corpus ON lexical_corpus FIELDS scope,family UNIQUE;\n");sql
}
fn value<T:serde::Serialize>(v:T)->Result<Value,ModelError>{crate::loader::json_value(serde_json::to_value(v).map_err(ModelError::codec)?)}
fn row(id:RecordId)->Object {let mut row=Object::new();row.insert("id",id);row}
fn object(row:Value)->Result<Object,ModelError>{if let Value::Object(row)=row {Ok(row)}else{Err(ModelError::Schema("lexical statistics row"))}}
fn string(row:&Object,key:&str)->Result<String,ModelError>{String::from_value(row.get(key).cloned().ok_or(ModelError::Schema("lexical statistics field"))?).map_err(ModelError::codec)}

fn increment(n:u64,by:u64)->Result<u64,ModelError>{n.checked_add(by).ok_or(ModelError::Schema("lexical statistics overflow"))}
struct Expected {rows:Vec<OrderedRows>,scope:ContentHash,budget:ResourceBudget}
async fn prepare(loader:&Loader)->Result<Expected,ModelError>{
 let reader=loader.reader();let scope=scope_identity(&reader)?;let budget=ResourceBudget::fixed(DEFAULT_MEMORY_BYTES)?;
 let mut expected=(0..4).map(|_|SortedRows::with_budget(&budget)).collect::<Result<Vec<_>,_>>()?;
 for (family,table) in FAMILIES.iter().enumerate(){
  let family=family as i16;let mut count=0u64;let mut total=0u64;let mut terms=SortedRows::with_budget(&budget)?;
  let sql=format!("SELECT id,text,search::analyze('lctx_discovery',text) AS tokens FROM {table} WHERE id IN (SELECT VALUE in FROM lex_occurs WHERE eligible=true AND {}) ORDER BY id",selected_occurrence_predicate(&reader,"$this"));
  let mut docs=reader.query_stream(sql,reader.view_bindings(),1)?;
  let lowered:Result<(),ModelError>=async {
  while let Some(doc)=docs.next().await? {
   let charge=budget.reserve("lexical statistics document",serde_json::to_vec(&doc).map_err(ModelError::codec)?.len().saturating_mul(12))?;
   let doc=object(doc)?;let id=RecordId::from_value(doc.get("id").cloned().ok_or(ModelError::Schema("lexical document id"))?).map_err(ModelError::codec)?;
   let text=string(&doc,"text")?;let tokens=Vec::<String>::from_value(doc.get("tokens").cloned().ok_or(ModelError::Schema("lexical analyzed tokens"))?).map_err(ModelError::codec)?;
   let mut frequencies=BTreeMap::<String,u64>::new();for token in &tokens {let n=frequencies.entry(token.clone()).or_default();*n=increment(*n,1)?;}
   let mut key=KeySink::new("native-lexical-document/v1");key.part(b"analyzer",&analyzer_identity().0);key.part(b"text",text.as_bytes());let token_id=RecordId::new("lexical_document",key.finish().hex());
   let mut token_row=row(token_id.clone());token_row.insert("length",tokens.len());token_row.insert("terms",value(frequencies.iter().map(|(term,tf)|serde_json::json!({"term":term,"tf":tf,"dl":tokens.len()})).collect::<Vec<_>>())?);expected[0].push(Value::Object(token_row))?;
   let mut member_key=KeySink::new("native-lexical-member/v1");member_key.part(b"scope",&scope.0);member_key.part(b"family",&family.to_le_bytes());member_key.part(b"document",&serde_json::to_vec(&id).map_err(ModelError::codec)?);
   let mut member=row(RecordId::new("lexical_member",member_key.finish().hex()));member.insert("scope",scope.hex());member.insert("family",family);member.insert("source",id.clone());member.insert("in",token_id);member.insert("out",corpus_id(scope,family));expected[1].push(Value::Object(member))?;
   for term in frequencies.keys(){let mut occurrence=row(RecordId::new("lexical_sort",format!("{}:{}",hex::encode(term.as_bytes()),hex::encode(serde_json::to_vec(&id).map_err(ModelError::codec)?))));occurrence.insert("term",term.clone());terms.push(Value::Object(occurrence))?;}
   count=increment(count,1)?;total=increment(total,tokens.len() as u64)?;drop(charge);
  }
  Ok(())
  }.await;
  let mut terminal=Completion::default();terminal.step("lexical analyzed-document stream drainage",docs.drain_transport().await);complete(lowered,terminal)?;
  let mut ordered=terms.finish()?;let mut current:Option<String>=None;let mut df=0u64;
  while let Some(term)=ordered.next_row()?{let term=string(&object(term)?,"term")?;if current.as_ref().is_some_and(|old|old!=&term){emit_term(&mut expected[2],scope,family,current.take().unwrap(),df)?;df=0;}current=Some(term);df=increment(df,1)?;}
  if let Some(term)=current {emit_term(&mut expected[2],scope,family,term,df)?;}
  let mut corpus=row(corpus_id(scope,family));corpus.insert("scope",scope.hex());corpus.insert("family",family);corpus.insert("documents",value(count)?);corpus.insert("total_length",value(total)?);expected[3].push(Value::Object(corpus))?;
 }
 Ok(Expected{rows:expected.into_iter().map(SortedRows::finish).collect::<Result<_,_>>()?,scope,budget})
}
fn emit_term(rows:&mut SortedRows,scope:ContentHash,family:i16,term:String,df:u64)->Result<(),ModelError>{let mut key=KeySink::new("native-lexical-term/v1");key.part(b"scope",&scope.0);key.part(b"family",&family.to_le_bytes());key.part(b"term",term.as_bytes());let mut row=row(RecordId::new("lexical_term",key.finish().hex()));row.insert("scope",scope.hex());row.insert("family",family);row.insert("term",term);row.insert("df",value(df)?);rows.push(Value::Object(row))}
async fn check(loader:&Loader,expected:&mut Expected)->Result<(),ModelError>{
 let reader=loader.reader();
 for(index,table)in TABLES.iter().enumerate(){
  let mut vars=reader.view_bindings();vars.insert("scope",expected.scope.hex());
  let predicate=if index==0{"id IN (SELECT VALUE in FROM lexical_member WHERE scope=$scope)"}else{"scope=$scope"};
  let mut actual=reader.query_stream(format!("SELECT * FROM {table} WHERE {predicate} ORDER BY id"),vars,1)?;
  expected.rows[index].rewind()?;
  let compared=async {loop{
   let a=actual.next().await?;let e=expected.rows[index].next_row()?;
   if a!=e{return Err(ModelError::Conflict("frozen lexical statistics differ"));}
   if a.is_none(){return Ok(());}
  }}.await;
  let mut terminal=Completion::default();terminal.step("lexical actual-statistics stream drainage",actual.drain_transport().await);complete(compared,terminal)?;
 }
 Ok(())
}
/// Construct once for a complete exact view; guarded immutable ingress rejects changed content.
pub async fn materialize(loader:&Loader)->Result<(),ModelError>{
 if loader.attempt_id().is_none(){return Err(ModelError::Conflict("lexical construction requires an owned attempt"));}
 let mut expected=prepare(loader).await?;
 let mut charge=expected.budget.reserve("lexical statistics write window",0)?;
 // Membership retention links require both immutable endpoints to exist.
 for index in [0usize,3,2,1]{let table=TABLES[index];
  let mut batch=Vec::new();let mut bytes=0usize;
  while let Some(row)=expected.rows[index].next_row()?{
   let size=serde_json::to_vec(&row).map_err(ModelError::codec)?.len().saturating_mul(12);
   if !batch.is_empty() && (batch.len()==128 || bytes.saturating_add(size)>8*1024*1024){loader.insert(table,std::mem::take(&mut batch),false).await?;bytes=0;charge.try_resize(0)?;}
   bytes=bytes.checked_add(size).ok_or(ModelError::Schema("lexical statistics write bytes"))?;charge.try_resize(bytes)?;batch.push(row);
  }
  loader.insert(table,batch,false).await?;charge.try_resize(0)?;
 }
 check(loader,&mut expected).await
}
/// Independent native analysis and actual-content comparison, with no writes or marker shortcuts.
pub async fn reconcile(loader:&Loader)->Result<(),ModelError>{let mut expected=prepare(loader).await?;check(loader,&mut expected).await}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn scope_inventory_is_order_independent_and_binds_actual_views(){
  let client=std::sync::Arc::new(crate::surrealdb::Surreal::init());let a=ContentHash::of(b"a");let b=ContentHash::of(b"b");
  let first=NativeReader::for_views(client.clone(),vec![a,b,a]);let reordered=NativeReader::for_views(client.clone(),vec![b,a]);let different=NativeReader::for_views(client,vec![a]);
  assert_eq!(scope_identity(&first).unwrap(),scope_identity(&reordered).unwrap());assert_ne!(scope_identity(&first).unwrap(),scope_identity(&different).unwrap());
 }
}
