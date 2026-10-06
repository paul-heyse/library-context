//! Original text is rendered one admitted source subject at a time, before vector consumption.
use crate::{consumed_rows::{ClosureTable,NominalClosure,ConsumedInputs,stream_query_at},workspace::{CompletedInputs,ProducerOutput,Workspace}};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{embedding::text::{self,TextAssessment,TextData,TextDefinition,TextSubject,TextWindow},normalized::Rows,stages::ProviderOutcome,*,source::*,syntax::*,documents::*,assertion::*,artifact::*,input::*};
use std::{collections::BTreeMap,sync::Arc};
use arrow_array::Array;
fn key_literal(bytes:&[u8])->String {format!("X'{}'",bytes.iter().map(|byte|format!("{byte:02x}")).collect::<String>())}
async fn prepare_edges(access:&CompletedInputs,session:&SessionContext,budget:&resources::ResourceBudget) -> Result<(crate::consumed_rows::PreparedEdges,BTreeMap<&'static str,usize>),ModelError> {
    let mut tables=Vec::new();
    macro_rules! table {($($field:ident:$ty:ty,)*)=>{$(tables.push(ClosureTable {relation:Relation::of::<$ty>(),alias:access.table_at::<$ty>(None)?});)*};}
    lctx_model::analytic_text_inputs!(table);
    let indices:BTreeMap<_,_>=tables.iter().enumerate().map(|(index,table)|(table.relation.name(),index)).collect();
    let mut plan=NominalClosure::new(tables.clone())?;
    for (source,table) in tables.iter().enumerate() {
        for field in table.relation.fields() {
            if !field.list() && let Some((_,target))=field.target() && let Some(target)=indices.get(target) {plan.follow(source,field.name(),*target)?;}
        }
    }
    plan.own(indices[ArtifactUse::NAME],"artifact",indices[SourceArtifact::NAME])?;
    plan.own(indices[Module::NAME],"source",indices[SourceArtifact::NAME])?;
    let q=&tables[indices[AssertionQualification::NAME]].alias;
    let declarations=&tables[indices[DeclarationObservation::NAME]].alias;
    let parameters=&tables[indices[ParameterSyntaxObservation::NAME]].alias;
    let placements=&tables[indices[SyntaxPlacement::NAME]].alias;
    let details=&tables[indices[SyntaxDetailObservation::NAME]].alias;
    let occurrences=&tables[indices[Occurrence::NAME]].alias;
    let chunks=&tables[indices[ArtifactChunk::NAME]].alias;
    plan.pairs(indices[DeclarationObservation::NAME],indices[DeclarationObservation::NAME],format!("SELECT child.id AS source_id,parent.id AS target_id FROM \"{declarations}\" child JOIN \"{q}\" cq ON cq.id=child.qualification JOIN \"{declarations}\" parent ON parent.declaration=child.parent JOIN \"{q}\" pq ON pq.id=parent.qualification WHERE cq.context=pq.context"))?;
    plan.pairs(indices[DeclarationObservation::NAME],indices[ParameterSyntaxObservation::NAME],format!("SELECT declaration.id AS source_id,parameter.id AS target_id FROM \"{declarations}\" declaration JOIN \"{q}\" dq ON dq.id=declaration.qualification JOIN \"{parameters}\" parameter ON parameter.function=declaration.declaration JOIN \"{q}\" pq ON pq.id=parameter.qualification WHERE dq.context=pq.context"))?;
    let value_field=lctx_model::domain::lexical::SyntaxField::Value as i16;
    let placement=format!("SELECT declaration.id AS source_id,placement.id AS target_id FROM \"{declarations}\" declaration JOIN \"{q}\" dq ON dq.id=declaration.qualification JOIN \"{placements}\" placement ON placement.parent=declaration.docstring JOIN \"{q}\" pq ON pq.id=placement.qualification WHERE dq.context=pq.context AND placement.field={value_field} AND placement.ordinal=0");
    plan.pairs(indices[DeclarationObservation::NAME],indices[SyntaxPlacement::NAME],placement)?;
    plan.pairs(indices[SyntaxPlacement::NAME],indices[SyntaxDetailObservation::NAME],format!("SELECT placement.id AS source_id,detail.id AS target_id FROM \"{placements}\" placement JOIN \"{q}\" pq ON pq.id=placement.qualification JOIN \"{details}\" detail ON detail.occurrence=placement.occurrence JOIN \"{q}\" dq ON dq.id=detail.qualification WHERE pq.context=dq.context"))?;
    // Chunk edges exist only for names/parameter spans actually read by the renderer. A whole
    // declaration/body occurrence must never expand into all of its original source chunks.
    for (name,field) in [(DeclarationObservation::NAME,"name"),(ParameterSyntaxObservation::NAME,"parameter")] {
        let table=&tables[indices[name]].alias;
        plan.pairs(indices[name],indices[ArtifactChunk::NAME],format!("SELECT subject.id AS source_id,chunk.id AS target_id FROM \"{table}\" subject JOIN \"{occurrences}\" span ON span.id=subject.\"{field}\" JOIN \"{chunks}\" chunk ON chunk.artifact=span.source AND chunk.ordinal>=span.start/{ARTIFACT_CHUNK_BYTES} AND chunk.ordinal<=(span.end-1)/{ARTIFACT_CHUNK_BYTES} WHERE span.end>span.start"))?;
    }
    let entities=&tables[indices[lctx_model::domain::normalized::entities::EntityRef::NAME]].alias;
    for (name,column,reference) in [(lctx_model::domain::normalized::entities::CallableEntity::NAME,"source_declaration","callable_callable"),(lctx_model::domain::normalized::entities::ClassEntity::NAME,"source_declaration","class_class")] {
        let table=&tables[indices[name]].alias;
        plan.pairs(indices[DeclarationObservation::NAME],indices[lctx_model::domain::normalized::entities::EntityRef::NAME],format!("SELECT declaration.id AS source_id,entity.id AS target_id FROM \"{declarations}\" declaration JOIN \"{table}\" identity ON identity.\"{column}\"=declaration.declaration JOIN \"{entities}\" entity ON entity.\"{reference}\"=identity.id"))?;
    }
    Ok((plan.prepare(session,budget).await?,indices))
}
async fn selected_source(access:&CompletedInputs,session:&SessionContext,source:&str,budget:&resources::ResourceBudget) -> Result<bool,ModelError> {
    let mut artifacts=Rows::<SourceArtifact>::new(budget);let mut uses=Rows::<ArtifactUse>::new(budget);
    macro_rules! read {($ty:ty,$rows:ident,$predicate:expr)=>{{
        let _permit=access.read::<$ty>()?;let table=access.table_at::<$ty>(None)?;
        let mut stream=crate::sql::query(session,&format!("SELECT * FROM \"{table}\" WHERE {}",$predicate)).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {$rows.decode(&batch)?;}
    }};}
    read!(SourceArtifact,artifacts,format!("id={source}"));read!(ArtifactUse,uses,format!("artifact={source}"));
    let _copies=budget.reserve("analytic-text-root-copies",artifacts.iter().map(|row|size_of::<SourceArtifact>()+row.heap_bytes()).sum::<usize>()+uses.len()*size_of::<ArtifactUse>())?;
    Ok(!admission::analysis_roots(&artifacts.iter().cloned().collect::<Vec<_>>(),&uses.iter().cloned().collect::<Vec<_>>())?.is_empty())
}
pub async fn publish(access:CompletedInputs,output:ProducerOutput,runtime:&Workspace,_model:&Arc<ValidatedModel>,definition:TextDefinition)->Result<(),ModelError> {
    definition.validate()?;
    output.declare::<TextDefinition>()?;output.declare::<TextSubject>()?;output.declare::<TextAssessment>()?;output.declare::<TextWindow>()?;
    output.push(definition.clone()).await?;
    if !definition.requested {return output.finish(ProviderOutcome::Complete).await;}
    let session=access.session(runtime).await?;
    let (edges,indices)=prepare_edges(&access,&session,runtime.budget()).await?;
    for (relation,query) in [
        (DeclarationObservation::NAME,format!("SELECT declaration.id,occurrence.source FROM {} declaration JOIN {} occurrence ON occurrence.id=declaration.declaration ORDER BY occurrence.source,declaration.id",DeclarationObservation::NAME,Occurrence::NAME)),
        (PassageObservation::NAME,format!("SELECT passage.id,evidence.source_span_source AS source FROM {} passage JOIN {} node ON node.id=passage.passage JOIN {} evidence ON evidence.id=node.passage_span ORDER BY evidence.source_span_source,passage.id",PassageObservation::NAME,DocumentNode::NAME,Evidence::NAME)),
    ] {
        let mut roots=crate::sql::query(&session,&query).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        let mut current_source:Option<(Vec<u8>,bool)>=None;
        while let Some(batch)=roots.try_next().await.map_err(ModelError::codec)? {
            let ids=batch.column(0).as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>().ok_or(ModelError::Schema("analytic subject ids"))?;
            let sources=batch.column(1).as_any().downcast_ref::<arrow_array::FixedSizeBinaryArray>().ok_or(ModelError::Schema("analytic subject sources"))?;
            for index in 0..ids.len() {
                let source=sources.value(index);
                if current_source.as_ref().is_none_or(|(previous,_)|previous!=source) {current_source=Some((source.to_vec(),selected_source(&access,&session,&key_literal(source),runtime.budget()).await?));}
                if !current_source.as_ref().expect("selected source").1 {continue;}
                let key=key_literal(ids.value(index));let scope=edges.grain(indices[relation],&format!("id={key}"),runtime.budget()).await?;
                let mut data=TextData::new(runtime.budget());
                let mut consumed=ConsumedInputs::new(TextData::inputs(),runtime.budget())?;
                macro_rules! read {($($field:ident:$ty:ty,)*)=>{$(while let Some((declaration,permit))=consumed.next::<$ty>(&access)? {
                    stream_query_at(&permit,&declaration,scope.session(),&scope.select(indices[<$ty>::NAME])?,|_,batch|data.$field.decode(batch)).await?;
                })*};}
                lctx_model::analytic_text_inputs!(read);consumed.finish("analytic_text")?;
                let root=if relation==DeclarationObservation::NAME {TextSubject::Declaration {declaration:data.declarations.iter().find(|row|key_literal(row.id().bytes())==key).ok_or(ModelError::Schema("analytic declaration root"))?.id()}} else {TextSubject::Passage {passage:data.passages.iter().find(|row|key_literal(row.id().bytes())==key).ok_or(ModelError::Schema("analytic passage root"))?.id()}};
                let prepared=text::prepare_subject(&data,&definition,root,runtime.budget())?;
                drop(data);drop(scope);
                if let Some(prepared)=prepared {
                    output.push(prepared.subject.clone()).await?;output.push(prepared.assessment.clone()).await?;
                    if let Some(text)=prepared.text() {
                        for (ordinal,(start,part)) in text::stream_windows(text,definition.window_bytes as usize)?.enumerate() {
                            let _copy=runtime.budget().reserve("analytic-window-transfer",size_of::<TextWindow>()+part.len())?;
                            output.push(TextWindow {assessment:prepared.assessment.id(),ordinal:ordinal as i64,start:start as i64,end:(start+part.len()) as i64,text:part.into(),content:ContentHash::of(part.as_bytes())}).await?;
                        }
                    }
                }
            }
        }
    }
    drop(edges);drop(session);output.finish(ProviderOutcome::Complete).await
}
