//! Private stored-value observations for the independent numerical evaluator.
//! No inference, expected answers, public SQL transport or publication mutation.
use lctx_model::domain::{self as d, Record, ModelError, Id, ContentHash,
    embedding::{EmbeddingSpec, value::FullValue, projection::ProjectionDefinition},
    resources::ResourceBudget, serving::ranking::{Occurrence, Target, RankingPolicy}};
use lctx_native::{NativeReader, RecordSelection, config::ViewerConfig};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap,BTreeSet}, io::{Read,Write}, sync::{Arc,atomic::{AtomicBool,Ordering}}};
use surrealdb::types::{Object,RecordId,SerdeWrapper,SurrealValue,Value,Variables,ToSql};

#[derive(Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
struct Cohort { encoder_hash:String, policy_key:String, library_input:String, family:i16 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request { k:usize, cohort:Option<Cohort> }
#[derive(Serialize)]
struct Vector { vector_id:String, full_key:String, projection_key:String, embedding:Vec<f32> }
#[derive(Serialize)]
struct Witness { vector_id:String, occurrence:Occurrence, occurrence_key:String }
fn field(row:&Object,key:&str)->Result<Value,ModelError>{row.get(key).cloned().ok_or(ModelError::Schema("numeric replay field"))}
fn text(row:&Object,key:&str)->Result<String,ModelError>{String::from_value(field(row,key)?).map_err(ModelError::codec)}
fn nominal<T:serde::de::DeserializeOwned+Serialize+'static>(row:&Object,key:&str)->Result<T,ModelError>{Ok(SerdeWrapper::<T>::from_value(field(row,key)?).map_err(ModelError::codec)?.0)}
fn optional<T:serde::de::DeserializeOwned+Serialize+'static>(row:&Object,key:&str)->Result<Option<T>,ModelError>{match field(row,key)?{Value::None|Value::Null=>Ok(None),_=>nominal(row,key).map(Some)}}
fn occurrence(row:&Object,family:d::retrieval::Family)->Result<Occurrence,ModelError>{
    let unit=nominal(row,"unit")?;
    Ok(Occurrence{target:Target::Unit{unit},unit,window:nominal(row,"window")?,part:nominal(row,"part")?,
        binding:optional(row,"binding")?,context:nominal(row,"context")?,anchor:optional(row,"anchor")?,family})
}
fn key(value:&str)->Result<[u8;16],ModelError>{hex::decode(value).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("numeric replay nominal width"))}
fn hash(value:&str)->Result<ContentHash,ModelError>{Ok(ContentHash(hex::decode(value).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("numeric replay hash width"))?))}
async fn document(reader:&NativeReader,id:RecordId)->Result<Object,ModelError>{
    if id.table.as_str()!="vector"{return Err(ModelError::Schema("numeric replay vector source"));}
    let mut vars=Variables::new();vars.insert("document",id);
    let rows:Vec<Object>=reader.query_native("SELECT * FROM $document",vars).await?;
    let [row]=rows.as_slice()else{return Err(ModelError::Schema("numeric replay exact document"));};Ok(row.clone())
}
fn cohort(row:&Object)->Result<Cohort,ModelError>{Ok(Cohort{encoder_hash:text(row,"encoder_hash")?,policy_key:text(row,"policy_key")?,library_input:text(row,"library_input")?,family:i16::from_value(field(row,"family")?).map_err(ModelError::codec)?})}
async fn observe(reader:&NativeReader,request:Request,budget:&ResourceBudget)->Result<serde_json::Value,ModelError>{
    if !(1..=64).contains(&request.k){return Err(ModelError::Invalid("numeric replay k".into()));}
    let mut charge=d::charged::StateCharge::new(budget,"private-numeric-observations");
    let selected=if let Some(cohort)=request.cohort{cohort}else{
        let mut stream=lctx_native::derived_search::selected_occurrences(reader,"vec_occurs","eligible=true",Variables::new(),budget)?;
        let result=async{
            let row=stream.next().await?.ok_or(ModelError::Schema("numeric replay has no selected eligible cohort"))?;
            let row=Object::from_value(row).map_err(ModelError::codec)?;
            document(reader,RecordId::from_value(field(&row,"in")?).map_err(ModelError::codec)?).await.and_then(|row|cohort(&row))
        }.await;
        stream.cancel_delivery();
        let mut terminal=d::completion::Completion::default();terminal.step("numeric cohort discovery drainage",stream.drain_transport().await);
        d::completion::complete(result,terminal)?
    };
    let family=match selected.family{0=>d::retrieval::Family::ApiOptions,1=>d::retrieval::Family::DocumentationDeployment,2=>d::retrieval::Family::Scenario,3=>d::retrieval::Family::Source,_=>return Err(ModelError::Schema("numeric replay family"))};
    let input:[u8;16]=serde_json::from_str(&selected.library_input).map_err(ModelError::codec)?;
    let mut vars=Variables::new();vars.insert("family",selected.family);vars.insert("input_key",selected.library_input.clone());vars.insert("encoder",selected.encoder_hash.clone());vars.insert("policy",selected.policy_key.clone());
    let mut stream=lctx_native::derived_search::selected_occurrences(reader,"vec_occurs",
        "eligible=true AND family=$family AND scope_input=$input_key AND in.encoder_hash=$encoder AND in.policy_key=$policy AND in.family=family AND in.library_input=scope_input",vars,budget)?;
    let mut documents=BTreeMap::new();let mut witnesses=Vec::new();
    let collected=async{
        while let Some(value)=stream.next().await?{
            charge.grow(lctx_native::loader::native_bytes(&value).saturating_mul(3))?;
            if witnesses.len()==4096{return Err(ModelError::Invalid("numeric replay occurrence bound4096".into()));}
            let row=Object::from_value(value).map_err(ModelError::codec)?;
            let id=RecordId::from_value(field(&row,"in")?).map_err(ModelError::codec)?;
            if !documents.contains_key(&id){
                if documents.len()==256{return Err(ModelError::Invalid("numeric replay exhaustive cohort exceeds256".into()));}
                let row=document(reader,id.clone()).await?;
                if cohort(&row)?!=selected{return Err(ModelError::Conflict("numeric replay cohort changed"));}
                charge.grow(lctx_native::loader::native_bytes(&Value::Object(row.clone())).saturating_mul(3))?;
                documents.insert(id,row);
            }
            witnesses.push(Witness{vector_id:id_string(&row)?,occurrence:occurrence(&row,family)?,occurrence_key:text(&row,"occurrence_key")?});
        }Ok(())
    }.await;
    let mut terminal=d::completion::Completion::default();terminal.step("numeric exhaustive occurrence drainage",stream.drain_transport().await);d::completion::complete(collected,terminal)?;
    if documents.is_empty(){return Err(ModelError::Schema("numeric replay empty cohort"));}
    let mut rows=Vec::new();let mut full=BTreeMap::new();let mut encoder=None;
    for (id,row) in documents{
        let full_key=text(&row,"full_key")?;
        if !full.contains_key(&full_key){
            charge.grow(4096*4*4+1024)?;
            let mut values=reader.records::<FullValue>(RecordSelection::Keys(vec![key(&full_key)?])).await?;
            if values.len()!=1{return Err(ModelError::Schema("numeric replay canonical full winner"));}
            let value=values.remove(0);
            if value.dimensions!=4096||value.codec!=1||value.id().hex()!=full_key{return Err(ModelError::Schema("numeric replay full4096 codec"));}
            value.validate()?;
            if encoder.is_none(){
                let mut records=reader.records::<EmbeddingSpec>(RecordSelection::Keys(vec![*value.encoder.bytes()])).await?;
                if records.len()!=1{return Err(ModelError::Schema("numeric replay encoder"));}encoder=Some(records.remove(0));
            }
            value.verify_encoder(encoder.as_ref().ok_or(ModelError::Schema("numeric replay encoder missing"))?)?;
            full.insert(full_key.clone(),value);
        }
        rows.push(Vector{vector_id:id.to_sql(),full_key,projection_key:text(&row,"projection_key")?,embedding:Vec::<f32>::from_value(field(&row,"embedding")?).map_err(ModelError::codec)?});
    }
    rows.sort_by(|a,b|a.vector_id.cmp(&b.vector_id));
    let spec=encoder.ok_or(ModelError::Schema("numeric replay encoder missing"))?;
    if spec.service_hash!=hash(&selected.encoder_hash)?{return Err(ModelError::Conflict("numeric replay encoder cohort"));}
    let projection:Id<ProjectionDefinition>=serde_json::from_value(serde_json::json!(key(&selected.policy_key)?)).map_err(ModelError::codec)?;
    let policies=reader.records::<ProjectionDefinition>(RecordSelection::Keys(vec![*projection.bytes()])).await?;
    if policies.len()!=1||policies[0].dimensions!=1024{return Err(ModelError::Schema("numeric replay projection definition"));}
    let winner=full.get(&rows[0].full_key).ok_or(ModelError::Schema("numeric replay query winner"))?;
    let query=lctx_serving::QueryVector{spec:spec.service_hash,input:winner.input,vector:d::embedding::value::decode_vector(&winner.bytes.0,4096).map_err(ModelError::Invalid)?,recipe:spec.configuration()?.query_recipe(),projection};
    let projected=d::embedding::projection::project_prefix(&query.vector,1024).map_err(ModelError::Invalid)?;
    let policy=RankingPolicy::default();
    let candidates=lctx_serving::search::vector_scoped(reader,&projected,query.spec,d::embedding::value::value_digest(&query.vector),query.recipe.identity(),projection,family,&[input],None,false,lctx_serving::search::UnitScope::All,request.k,&policy,budget).await?;
    let rescored=lctx_serving::search::rescore_union_scoped(reader,&query,&candidates,&policy,budget).await?;
    let mapped=|values:&[lctx_serving::search::ScopedCandidate]|->Result<Vec<serde_json::Value>,ModelError>{values.iter().map(|candidate|{
        let matching=witnesses.iter().filter(|row|row.occurrence==candidate.occurrence).map(|row|row.vector_id.clone()).collect::<BTreeSet<_>>();
        if matching.is_empty(){return Err(ModelError::Conflict("served candidate outside observed selected cohort"));}
        Ok(serde_json::json!({"occurrence":candidate.occurrence,"vector_ids":matching,"score":candidate.score}))
    }).collect()};
    let native_candidates=mapped(&candidates)?;let native_rescored=mapped(&rescored)?;
    let full=full.into_iter().map(|(semantic_key,value)|serde_json::json!({"semantic_key":semantic_key,"dimensions":value.dimensions,"codec":value.codec,"bytes_hex":hex::encode(value.bytes.0)})).collect::<Vec<_>>();
    Ok(serde_json::json!({"schema":1,"snapshot":reader.handle(),"cohort":selected,"rows":rows,"full":full,"occurrences":witnesses,
        "native_candidates":native_candidates,"native_rescored":native_rescored,"inference_started":false}))
}
fn id_string(row:&Object)->Result<String,ModelError>{Ok(RecordId::from_value(field(row,"in")?).map_err(ModelError::codec)?.to_sql())}
#[tokio::main]
async fn main()->Result<(),ModelError>{
    let args=std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len()!=1{return Err(ModelError::Schema("numeric replay requires one viewer configuration"));}
    let config=ViewerConfig::read(std::path::Path::new(&args[0]))?;
    let handle=config.selected()?;
    let limits=config.serving_limits.clone().unwrap_or_default();limits.validate()?;
    let budget=ResourceBudget::fixed(limits.request_bytes as usize)?;
    let mut raw=Vec::new();std::io::stdin().take(64*1024+1).read_to_end(&mut raw).map_err(ModelError::codec)?;
    if raw.len()>64*1024{return Err(ModelError::Schema("numeric replay request bound"));}
    let request:Request=serde_json::from_slice(&raw).map_err(ModelError::codec)?;
    let stopped=Arc::new(AtomicBool::new(false));let flag=stopped.clone();let retry=lctx_native::control::RetryCancellation::default();let cancellation=retry.clone();
    let signal=lctx_native::signal::SignalRegistration::new("private numeric selected replay",move||{flag.store(true,Ordering::Release);cancellation.cancel();})?;
    let reader=lctx_native::control::with_retry_cancellation(retry,NativeReader::connect(&config.endpoint,&config.credentials(),handle)).await?;
    let reader=reader.with_budget(&budget).with_read_cancellation(stopped);
    let mut close=reader.protect_terminal_close();
    let result=observe(&reader,request,&budget).await;
    if result.as_ref().err().is_some_and(|error|!error.permits_storage_cleanup()){reader.retain_unknown();}
    let mut terminal=d::completion::Completion::default();terminal.step("numeric selected reader session close",close.close(&reader).await);
    if terminal.local==d::completion::LocalState::Terminal&&terminal.remote==d::completion::RemoteState::Confirmed&&terminal.failures.is_empty(){terminal.step("numeric signal owner",signal.complete(&terminal).await);}
    let value=d::completion::complete(result,terminal)?;
    let _encoding=budget.reserve("numeric replay output",32<<20)?;
    let encoded=serde_json::to_vec(&value).map_err(ModelError::codec)?;
    if encoded.len()>32<<20{return Err(ModelError::Schema("numeric replay output bound"));}
    std::io::stdout().write_all(&encoded).map_err(ModelError::codec)
}
