//! Necessary E0 admission over exact unit grains and one externally ordered request group.
use crate::{consumed_rows::{ClosureTable,NominalClosure,identifier},scoped_admission::field_target,workspace::Cancellation};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{retrieval::{self,build::{self,Data,Output},consumption::{self,ConsumptionData,RetrievalEmbeddingUse}},normalized::Rows,resources::ResourceBudget,*};
use std::any::TypeId;
fn index<R:Record>(inputs:&[ValidationInput])->Result<usize,ModelError>{
    let mut matches=inputs.iter().enumerate().filter(|(_,input)|input.type_id()==TypeId::of::<R>());
    let first=matches.next().ok_or(ModelError::Schema("E0 admission declaration absent"))?;
    if matches.next().is_none(){return Ok(first.0);}
    inputs.iter().position(|input|input.type_id()==TypeId::of::<R>() && input.prefix()==Some(stages::PublicationBoundary::Facts)).ok_or(ModelError::Conflict("E0 admission immutable epoch"))
}
fn alias<R:Record>(inputs:&[ValidationInput],tables:&[ClosureTable])->Result<String,ModelError>{Ok(identifier(&tables[index::<R>(inputs)?].alias))}
fn nominal<R:serde::de::DeserializeOwned>(bytes:&[u8])->Result<R,ModelError>{serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new(bytes.iter().copied())).map_err(ModelError::codec)}
fn bytes<'a>(batch:&'a arrow_array::RecordBatch,name:&str,row:usize)->Result<&'a [u8],ModelError>{
    use arrow_array::Array;
    let array=batch.column_by_name(name).ok_or(ModelError::Schema("E0 admission identity column"))?;
    if array.is_null(row){return Err(ModelError::Schema("E0 admission null identity"));}
    array.as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>().map(|array|array.value(row)).ok_or(ModelError::Schema("E0 admission identity shape"))
}
async fn stream(session:&SessionContext,sql:&str,cancellation:&Cancellation,mut visit:impl FnMut(&arrow_array::RecordBatch)->Result<(),ModelError>)->Result<(),ModelError>{
    let mut rows=crate::sql::query(session,sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch)=rows.try_next().await.map_err(ModelError::codec)?{cancellation.check()?;visit(&batch)?;}Ok(())
}
async fn fetch<R:Record>(inputs:&[ValidationInput],tables:&[ClosureTable],session:&SessionContext,id:Id<R>,budget:&ResourceBudget,cancellation:&Cancellation)->Result<R,ModelError>{
    let mut rows=Rows::new(budget);let sql=format!("SELECT * FROM {} WHERE id=X'{}'",alias::<R>(inputs,tables)?,id.hex());
    stream(session,&sql,cancellation,|batch|{rows.decode(batch)?;Ok(())}).await?;Ok(build::need(&rows,id)?.clone())
}
async fn refuse_rows(session:&SessionContext,sql:String,cancellation:&Cancellation,message:&'static str)->Result<(),ModelError>{
    stream(session,&sql,cancellation,|batch|{if batch.num_rows()!=0{return Err(build::invalid(message));}Ok(())}).await
}
fn plan(inputs:&[ValidationInput],tables:&[ClosureTable])->Result<NominalClosure,ModelError>{
    let mut plan=NominalClosure::new(tables.to_vec())?;
    for(source,table)in tables.iter().enumerate(){for field in table.relation.fields(){
        let Some((target,_))=field.target()else{continue;};let Some(target)=field_target(inputs,source,target)?else{continue;};
        if field.list(){plan.pairs(source,target,format!("SELECT id AS source_id,UNNEST({}) AS target_id FROM {}",identifier(field.name()),identifier(&table.alias)))?;}else{plan.follow(source,field.name(),target)?;}
    }}
    let unit=index::<retrieval::Unit>(inputs)?;let corpus=index::<retrieval::CorpusText>(inputs)?;
    for(member,field,owner)in[(index::<retrieval::UnitRoot>(inputs)?,"unit",unit),(index::<retrieval::OriginalAnchor>(inputs)?,"unit",unit),(index::<retrieval::UnitSubject>(inputs)?,"unit",unit),(index::<retrieval::Fragment>(inputs)?,"corpus",corpus)]{plan.own(member,field,owner)?;}
    Ok(plan)
}
/// Root integrates this declared scope arm in Workspace::validate_scope. Frozen aliases and
/// all catalogs are supplied by that existing admission authority; no input is reacquired.
pub(crate) async fn validate_retrieval(invariant:&Invariant,_scope:consumption::Scope,tables:Vec<ClosureTable>,session:&SessionContext,budget:&ResourceBudget,cancellation:&Cancellation)->Result<(),ModelError>{
    let inputs=&invariant.inputs;let mut metadata=ConsumptionData::new(budget);
    let frames=Data::frame_inputs();let mut invocations=Rows::<analysis::retrieval::Invocation>::new(budget);let mut outcomes=Rows::<analysis::retrieval::AnalysisOutcome>::new(budget);
    for(index,input)in inputs.iter().enumerate(){
        if frames.iter().any(|candidate|candidate.type_id()==input.type_id() && candidate.prefix()==input.prefix()) || [TypeId::of::<embedding::EmbeddingSpec>(),TypeId::of::<embedding::configuration::ServiceConfiguration>(),TypeId::of::<analysis::retrieval::InvocationSource>(),TypeId::of::<analysis::retrieval::AnalysisInput>()].contains(&input.type_id()){
            stream(session,&format!("SELECT * FROM {}",identifier(&tables[index].alias)),cancellation,|batch|{metadata.visit_input(input,batch)?;Ok(())}).await?;
        }else if input.type_id()==TypeId::of::<analysis::retrieval::Invocation>(){stream(session,&format!("SELECT * FROM {}",identifier(&tables[index].alias)),cancellation,|batch|{invocations.decode(batch)?;Ok(())}).await?;
        }else if input.type_id()==TypeId::of::<analysis::retrieval::AnalysisOutcome>(){stream(session,&format!("SELECT * FROM {}",identifier(&tables[index].alias)),cancellation,|batch|{outcomes.decode(batch)?;Ok(())}).await?;}
    }
    let selected=metadata.render.selected()?.embedding_requested;if selected{metadata.selected_spec()?;}
    let unit=alias::<retrieval::Unit>(inputs,&tables)?;let corpus=alias::<retrieval::CorpusText>(inputs,&tables)?;let fragment=alias::<retrieval::Fragment>(inputs,&tables)?;let uses=alias::<RetrievalEmbeddingUse>(inputs,&tables)?;let invocation=alias::<analysis::retrieval::Invocation>(inputs,&tables)?;
    refuse_rows(session,format!("SELECT c.id FROM {corpus} c WHERE NOT EXISTS (SELECT 1 FROM {unit} u WHERE u.corpus=c.id) LIMIT 1"),cancellation,"retrieval corpus has no contextual unit").await?;
    refuse_rows(session,format!("SELECT v.id FROM {uses} v JOIN {invocation} i ON v.invocation=i.id JOIN {fragment} f ON v.fragment=f.id WHERE NOT EXISTS (SELECT 1 FROM {unit} u WHERE u.corpus=f.corpus AND u.input=i.input AND u.context=i.context) LIMIT 1"),cancellation,"retrieval use has no exact owning native frame").await?;
    if !selected{refuse_rows(session,format!("SELECT id FROM {uses} LIMIT 1"),cancellation,"retrieval has unrequested vector uses").await?;}
    let prepared=plan(inputs,&tables)?.prepare(session,budget).await?;let unit_index=index::<retrieval::Unit>(inputs)?;let completion=Data::completion_types();let outputs=Output::inputs();
    let mut roots=crate::sql::query(session,&format!("SELECT id FROM {unit} ORDER BY id")).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch)=roots.try_next().await.map_err(ModelError::codec)?{
        cancellation.check()?;for row in 0..batch.num_rows(){
            let id:Id<retrieval::Unit>=nominal(bytes(&batch,"id",row)?)?;let closure=prepared.grain(unit_index,&format!("id=X'{}'",id.hex()),budget).await?;
            let mut data=Data::new(budget);data.facts.definitions.insert(metadata.render.selected()?.clone())?;let mut output=Output::new(budget);
            for(index,input)in inputs.iter().enumerate(){
                if !completion.contains(&input.type_id()) && !outputs.iter().any(|candidate|candidate.type_id()==input.type_id()){continue;}
                if input.type_id()==TypeId::of::<retrieval::RetrievalDefinition>(){continue;}
                let selected=closure.select(index)?;
                let sql=if input.type_id()==TypeId::of::<catalog::CatalogMember>(){format!("SELECT id,input,access FROM ({selected}) ownership")}else if input.type_id()==TypeId::of::<source::Module>(){format!("SELECT id,source FROM ({selected}) ownership")}else if input.type_id()==TypeId::of::<documents::DocumentObservation>(){format!("SELECT id,source FROM ({selected}) ownership")}else if input.type_id()==TypeId::of::<synthesis::briefs::Brief>(){format!("SELECT id,seed FROM ({selected}) ownership")}else if input.type_id()==TypeId::of::<source::SourceArtifact>(){format!("SELECT id,input,byte_len FROM ({selected}) ownership")}else{selected};
                stream(closure.session(),&sql,cancellation,|batch|{data.completion_visit(input,batch)?;output.visit(input.name(),batch)?;Ok(())}).await?;
            }
            output.verify_completion(&data,budget)?;
            let owner=build::need(&output.units,id)?;let mut parents=invocations.iter().filter(|invocation|invocation.input==owner.input && invocation.context==owner.context);
            let parent=parents.next().ok_or_else(||build::invalid("retrieval unit native frame absent"))?;if parents.next().is_some(){return Err(build::invalid("retrieval unit native frame ambiguous"));}
            let mut unit_uses=Rows::new(budget);let sql=format!("SELECT v.* FROM {uses} v JOIN {fragment} f ON v.fragment=f.id WHERE f.corpus=X'{}' AND v.invocation=X'{}'",owner.corpus.hex(),parent.id().hex());
            stream(session,&sql,cancellation,|batch|{unit_uses.decode(batch)?;Ok(())}).await?;
            consumption::verify_uses(&output,parent,if selected{Some(metadata.selected_spec()?)}else{None},&unit_uses,budget)?;
        }
    }
    drop(roots);drop(prepared);
    // Outcome reasons depend on all units in one native frame, but only availability is needed.
    let mut expected=Rows::new(budget);
    for parent in invocations.iter(){
        let mut disposition=consumption::Disposition::default();let sql=format!("SELECT DISTINCT availability FROM {uses} WHERE invocation=X'{}'",parent.id().hex());
        stream(session,&sql,cancellation,|batch|{let values=batch.column_by_name("availability").and_then(|array|array.as_any().downcast_ref::<arrow_array::Int16Array>()).ok_or(ModelError::Schema("retrieval availability shape"))?;for value in values.iter(){disposition.observe_availability(match value{Some(0)=>embedding::analytic::VectorAvailability::Available,Some(1)=>embedding::analytic::VectorAvailability::ServiceUnavailable,Some(2)=>embedding::analytic::VectorAvailability::TokenLimit,_=>return Err(ModelError::Schema("retrieval availability code"))});}Ok(())}).await?;
        expected.insert(disposition.outcome(parent))?;
    }
    metadata.verify_frames(&invocations,&outcomes,&expected,budget)?;
    verify_winners(inputs,&tables,session,budget,cancellation,&metadata.specifications).await
}
async fn verify_winners(inputs:&[ValidationInput],tables:&[ClosureTable],session:&SessionContext,budget:&ResourceBudget,cancellation:&Cancellation,specifications:&Rows<embedding::EmbeddingSpec>)->Result<(),ModelError>{
    let analytic=alias::<embedding::analytic::AnalysisEmbeddingUse>(inputs,tables)?;let retrieval=alias::<RetrievalEmbeddingUse>(inputs,tables)?;
    let sql=format!("SELECT specification,input,kind,id FROM (SELECT specification,input,0 AS kind,id FROM {analytic} WHERE availability=0 UNION ALL SELECT specification,input,1 AS kind,id FROM {retrieval} WHERE availability=0) request_keys ORDER BY specification,input,kind,id");
    let mut keys=crate::sql::query(session,&sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
    let mut group=None;let mut winners=embedding::consumption::Winners::new(budget);let mut tokens=None;let _group=budget.reserve("retrieval-current-winner-key",128)?;
    while let Some(batch)=keys.try_next().await.map_err(ModelError::codec)?{
        cancellation.check()?;let kinds=batch.column_by_name("kind").and_then(|array|array.as_any().downcast_ref::<arrow_array::Int64Array>()).ok_or(ModelError::Schema("retrieval winner kind"))?;
        for row in 0..batch.num_rows(){
            let specification:Id<embedding::EmbeddingSpec>=nominal(bytes(&batch,"specification",row)?)?;let input:ContentHash=nominal(bytes(&batch,"input",row)?)?;
            if group!=Some((specification,input)){drop(winners);winners=embedding::consumption::Winners::new(budget);group=Some((specification,input));tokens=None;}
            let spec=build::need(specifications,specification)?.configuration()?;
            let admitted=if kinds.value(row)==0{
                let use_=fetch::<embedding::analytic::AnalysisEmbeddingUse>(inputs,tables,session,nominal(bytes(&batch,"id",row)?)?,budget,cancellation).await?;
                let window=fetch::<embedding::text::TextWindow>(inputs,tables,session,use_.window,budget,cancellation).await?;let receipt=use_.receipt()?;let admitted=receipt.admitted_tokens;drop(winners.replay(&spec,window.text.as_str(),receipt)?);admitted
            }else if kinds.value(row)==1{
                let use_=fetch::<RetrievalEmbeddingUse>(inputs,tables,session,nominal(bytes(&batch,"id",row)?)?,budget,cancellation).await?;
                let fragment=fetch::<retrieval::Fragment>(inputs,tables,session,use_.fragment,budget,cancellation).await?;let receipt=use_.receipt()?;let admitted=receipt.admitted_tokens;drop(winners.replay(&spec,fragment.text.as_str(),receipt)?);admitted
            }else{return Err(ModelError::Schema("retrieval winner kind code"));};
            if tokens.is_some_and(|previous|previous!=admitted){return Err(build::invalid("embedding consumers disagree on exact winning tokens"));}tokens=Some(admitted);
        }
    }Ok(())
}

#[cfg(test)]
mod controls {
    use super::*;
    use datafusion::datasource::MemTable;
    use std::sync::Arc;
    fn id<R>(byte:u8)->Id<R>{nominal(&[byte;16]).unwrap()}
    fn fixture()->(SessionContext,Vec<ValidationInput>,Vec<ClosureTable>){
        let model=lctx_model::domain::model().unwrap();let inputs=retrieval::consumption::invariants().into_iter().find(|invariant|invariant.name=="retrieval_embedding_consumption_and_winners").unwrap().inputs;let session=SessionContext::new();
        let tables=inputs.iter().enumerate().map(|(index,input)|{let relation=model.relation(input.name()).unwrap().clone();let alias=format!("winner_fixture_{index}");let batch=arrow_array::RecordBatch::new_empty(relation.schema().clone());session.register_table(alias.as_str(),Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();ClosureTable{relation,alias}}).collect();(session,inputs,tables)
    }
    fn install<R:Record>(session:&SessionContext,inputs:&[ValidationInput],tables:&[ClosureTable],rows:&[R]){let index=index::<R>(inputs).unwrap();let batch=R::encode(rows).unwrap();session.deregister_table(tables[index].alias.as_str()).unwrap();session.register_table(tables[index].alias.as_str(),Arc::new(MemTable::try_new(batch.schema(),vec![vec![batch]]).unwrap())).unwrap();}
    fn spec()->embedding::Spec{let mut spec=embedding::Spec::parse(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../specs/embedding/qwen3-embedding-8b.json"))).unwrap();spec.reduction="none".into();spec.admission=None;spec.source_dimensions=32768;spec.dimensions=32768;spec.document_template="prefix: {text}".into();spec}
    #[tokio::test]
    async fn externally_ordered_request_groups_release_winners_and_refuse_changed_tokens_bytes_and_request(){
        let budget=ResourceBudget::fixed(2<<20).unwrap();let(session,inputs,tables)=fixture();let spec=spec();let specification=embedding::EmbeddingSpec::new(&spec).unwrap();let mut specifications=Rows::new(&budget);specifications.insert(specification.clone()).unwrap();
        let mut vector=vec![0.0;32768];vector[0]=1.0;vector[2]=-0.0;
        let mut windows=Vec::new();let mut fragments=Vec::new();let mut analytic=Vec::new();let mut retrieval=Vec::new();
        for ordinal in 0..64{
            let text=format!("exact request {ordinal}");let value=embedding::value::AdmittedValue::new(&spec,&spec.document_text(&text),7,&vector,&budget).unwrap();
            let window=embedding::text::TextWindow{assessment:id(1),ordinal,start:0,end:text.len() as i64,text:text.clone().into(),content:ContentHash::of(text.as_bytes())};
            let fragment=retrieval::Fragment{definition:retrieval::Definition::builtin(true).id(),fragment_bytes:4096,corpus:id(2),ordinal,start:0,end:text.len() as i64,digest:ContentHash::of(text.as_bytes()),text:text.into()};
            analytic.push(embedding::analytic::AnalysisEmbeddingUse{invocation:id(3),window:window.id(),specification:specification.id(),input:value.input(),availability:embedding::analytic::VectorAvailability::Available,admitted_tokens:Some(7),codec:Some(embedding::value::VALUE_CODEC),value_digest:Some(value.digest()),bytes:Some(EvidenceBytes(value.bytes().to_vec()))});
            retrieval.push(RetrievalEmbeddingUse{invocation:id(4),fragment:fragment.id(),specification:specification.id(),input:value.input(),availability:embedding::analytic::VectorAvailability::Available,admitted_tokens:Some(7),codec:Some(embedding::value::VALUE_CODEC),value_digest:Some(value.digest()),bytes:Some(EvidenceBytes(value.bytes().to_vec()))});windows.push(window);fragments.push(fragment);
        }
        install(&session,&inputs,&tables,&windows);install(&session,&inputs,&tables,&fragments);install(&session,&inputs,&tables,&analytic);install(&session,&inputs,&tables,&retrieval);
        let cancellation=Cancellation::default();verify_winners(&inputs,&tables,&session,&budget,&cancellation,&specifications).await.unwrap();
        // Same valid vector and exact request, with only admitted token evidence changed.
        retrieval[0].admitted_tokens=Some(8);install(&session,&inputs,&tables,&retrieval);
        assert!(verify_winners(&inputs,&tables,&session,&budget,&cancellation,&specifications).await.unwrap_err().to_string().contains("winning tokens"));retrieval[0].admitted_tokens=Some(7);
        let changed=embedding::value::AdmittedValue::new(&spec,&spec.document_text(fragments[0].text.as_str()),7,&{let mut values=vector.clone();values[2]=0.0;values},&budget).unwrap();retrieval[0].bytes=Some(EvidenceBytes(changed.bytes().to_vec()));retrieval[0].value_digest=Some(changed.digest());drop(changed);install(&session,&inputs,&tables,&retrieval);
        assert!(verify_winners(&inputs,&tables,&session,&budget,&cancellation,&specifications).await.unwrap_err().to_string().contains("winning bytes"));retrieval[0].bytes=analytic[0].bytes.clone();retrieval[0].value_digest=analytic[0].value_digest;
        retrieval[0].input=ContentHash::of(b"different exact request");install(&session,&inputs,&tables,&retrieval);assert!(verify_winners(&inputs,&tables,&session,&budget,&cancellation,&specifications).await.is_err());
        drop(specifications);assert_eq!(budget.reserved(),0);
    }
}
