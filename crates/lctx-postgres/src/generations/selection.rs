//! One opaque, generation-bound admitted classification owner; no legacy serving projections.
use super::{Error,GenerationId,GenerationLease,GenerationReader,validation_views,visit_named};
use lctx_model::domain::{*,admission::Frontier, resources::ResourceBudget, selection::{self, admission::AdmissionData,build::{Data,Output},classification::ClassificationData,evaluate::{Prepared,Selected},algebra::Classified}, stages::{Profile,is_vocabulary}};
use futures::TryStreamExt;
use sqlx::Row;
use std::sync::Arc;
/// Initial envelope, injected explicitly by the caller; never increased or pruned automatically.
pub const SELECTION_PREPARATION_BYTES:usize=128*1024*1024;
struct Guard {lease:GenerationLease, lost:bool}
struct State {prepared:Prepared, guard:tokio::sync::Mutex<Guard>, _charge:Box<dyn resources::Reservation>}
/// Only canonical receipt admission constructs this capability. Clones retain one original guard
/// and the same charged rows/indexes until the last consumer releases them.
#[derive(Clone)]
pub struct AdmittedSelection {state:Arc<State>}
impl GenerationReader {
    pub async fn prepare_selection(&self,g:GenerationId,budget:ResourceBudget)->Result<AdmittedSelection,Error> {
        let charge=budget.reserve("selection-admitted-owner",size_of::<State>()+64)?;
        let mut lease=self.pin(g,budget).await?;
        let profile=lease.selection_envelope().await?;
        let mut data=ClassificationData::new(&lease.budget);let mut output=Output::new(&lease.budget);let mut admission=AdmissionData::new(&lease.budget);
        for input in ClassificationData::inputs() {lease.selection_read(&input,|batch|{if !data.visit(input.name(),&batch)? {return Err(Error::Contract);}Ok(())}).await?;}
        for input in Output::inputs() {lease.selection_read(&input,|batch|{if !output.visit(input.name(),&batch)? {return Err(Error::Contract);}Ok(())}).await?;}
        for input in AdmissionData::inputs() {lease.selection_read(&input,|batch|{if !admission.visit(input.name(),&batch)? {return Err(Error::Contract);}Ok(())}).await?;}
        admission.validate(&data,&output,&lease.budget)?;
        lease.verify_selection_sources(&admission,profile).await?;
        let prepared=Prepared::from_local_rows(data,output,&lease.budget)?;
        lease.selection_live().await?;
        Ok(AdmittedSelection{state:Arc::new(State{prepared,guard:tokio::sync::Mutex::new(Guard{lease,lost:false}),_charge:charge})})
    }
}
impl Guard {
    async fn check(&mut self)->Result<(),Error> {
        if self.lost {return Err(Error::State);}
        if let Err(error)=self.lease.selection_live().await {self.lost=true;return Err(error);}
        Ok(())
    }
}
impl AdmittedSelection {
    pub async fn generation(&self)->GenerationId {self.state.guard.lock().await.lease.generation()}
    pub async fn classify(&self,member:Id<catalog::CatalogMember>,analysis:Id<attribution::AnalysisContext>,requirement:&selection::Requirement,budget:&ResourceBudget)->Result<Classified,Error> {
        let mut guard=self.state.guard.lock().await;guard.check().await?;
        let result=self.state.prepared.classify(member,analysis,requirement,budget);
        guard.check().await?;Ok(result?)
    }
    pub async fn select(&self,selection:&selection::Selection,budget:&ResourceBudget)->Result<Selected,Error> {
        let mut guard=self.state.guard.lock().await;guard.check().await?;
        let result=self.state.prepared.select(selection,budget);
        guard.check().await?;Ok(result?)
    }
    /// Acknowledged release requires sole ownership. Shared consumers keep the original lease.
    pub async fn release(self)->Result<(),Error> {
        let state=Arc::try_unwrap(self.state).map_err(|_|Error::Busy)?;
        state.guard.into_inner().lease.release().await
    }
}
impl GenerationLease {
    async fn selection_live(&mut self)->Result<(),Error> {
        let (high,low)=super::locks::halves(self.generation().lock());
        let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND granted AND mode='ShareLock' AND pid=pg_backend_pid() AND objsubid=1 AND classid::bigint=$1 AND objid::bigint=$2) AND EXISTS(SELECT 1 FROM lctx_model_store.generations WHERE id=$3 AND state='published')")
            .bind(high).bind(low).bind(self.generation().0.to_vec()).fetch_one(&mut *self.connection).await?;
        if !live {return Err(Error::State);}Ok(())
    }
    /// Explicit diagnostic: broad C2 replay from this same canonical lease, no admission capability.
    pub async fn prepare_selection_strict(&mut self)->Result<Prepared,Error> {
        self.selection_live().await?;
        let mut data=Data::new(&self.budget);let mut output=Output::new(&self.budget);
        for input in Data::inputs() {self.selection_read(&input,|batch|{if !data.visit(input.name(),&batch)? {return Err(Error::Contract);}Ok(())}).await?;}
        for input in Output::inputs() {self.selection_read(&input,|batch|{if !output.visit(input.name(),&batch)? {return Err(Error::Contract);}Ok(())}).await?;}
        let prepared=Prepared::new(&data,&output,&self.budget)?;
        self.selection_live().await?;Ok(prepared)
    }
    async fn selection_read(&mut self,input:&ValidationInput,mut consume:impl FnMut(arrow_array::RecordBatch)->Result<(),Error>)->Result<(),Error> {
        if !self.relations.contains(input.name()) {return Err(Error::Frontier("selection input outside canonical frontier".into()));}
        let relation=self.model.relations().iter().find(|r|r.name()==input.name() && r.type_id()==input.type_id()).ok_or(Error::Contract)?;
        let physical=validation_views::physical(&mut self.connection,self.contract.generation(),input,relation,relation.name(),validation_views::Scope{upper:None,candidate:None},&self.budget).await?;
        let expected:Option<(i64,Vec<u8>)>=if input.prefix().is_none() {
            sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.receipts WHERE generation_id=$1 AND relation_name=$2").bind(self.contract.generation().0.to_vec()).bind(input.name()).fetch_optional(&mut *self.connection).await?
        }else{
            let order=super::vocabulary::publication_order(&mut self.connection,self.contract.generation()).await?;let prefix=order.resolve(input.prefix().expect("declared prefix"))?;
            sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.epoch_receipts WHERE generation_id=$1 AND epoch=$2 AND relation_name=$3").bind(self.contract.generation().0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_|Error::Contract)?).bind(input.name()).fetch_optional(&mut *self.connection).await?
        };
        let expected=expected.ok_or(Error::Contract)?;
        let mut hash=relation.content();
        visit_named(&mut self.connection,self.contract.generation(),relation,&physical,input.order(),&self.budget,|batch|{relation.hash_rows(&batch,&mut hash)?;consume(batch)}).await?;
        let (rows,hash)=hash.finish();if u64::try_from(expected.0).ok()!=Some(rows) || expected.1!=hash.0 {return Err(Error::Contract);}Ok(())
    }
    async fn selection_envelope(&mut self)->Result<Profile,Error> {
        if self.frontier!=Frontier::Catalog {return Err(Error::Frontier("selection admission requires canonical Catalog".into()));}
        self.selection_live().await?;
        let (model,physical,schedule,content,profile):(Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>,String)=sqlx::query_as("SELECT model_digest,physical_digest,schedule_digest,content_digest,profile FROM lctx_model_store.generations WHERE id=$1 AND state='published' AND frontier='catalog'").bind(self.generation().0.to_vec()).fetch_one(&mut *self.connection).await?;
        if model!=self.model.digest().0 {return Err(Error::Contract);}
        let admitted:Option<bool>=sqlx::query_scalar("SELECT model_digest=$2 AND schedule_digest=$3 AND content_digest=$4 AND profile=$5 FROM lctx_model_store.admissions WHERE generation_id=$1").bind(self.generation().0.to_vec()).bind(&model).bind(&schedule).bind(&content).bind(&profile).fetch_optional(&mut *self.connection).await?;
        if admitted!=Some(true) {return Err(Error::Contract);}
        for name in selection::admission::VALIDATORS {
            let valid:Option<bool>=sqlx::query_scalar("SELECT model_digest=$3 AND physical_digest=$4 AND content_digest=$5 FROM lctx_model_store.validation_receipts WHERE generation_id=$1 AND validator_name=$2").bind(self.generation().0.to_vec()).bind(name).bind(&model).bind(&physical).bind(&content).fetch_optional(&mut *self.connection).await?;
            if valid!=Some(true) {return Err(Error::Contract);}
        }
        // Rebind the generation's receipt inventory without hydrating unrelated canonical bodies.
        let mut expected=self.relations.iter();let mut sink=KeySink::new("generation-content");
        let _receipt_charge=self.budget.reserve("selection-receipt-metadata",4096)?;
        {
            let mut stream=sqlx::query("SELECT relation_name,content_digest FROM lctx_model_store.receipts WHERE generation_id=$1 ORDER BY relation_name COLLATE \"C\"").bind(self.contract.generation().0.to_vec()).fetch(&mut *self.connection);
            while let Some(row)=stream.try_next().await? {
                let name:String=row.try_get(0)?;let bytes:Vec<u8>=row.try_get(1)?;
                if expected.next().copied()!=Some(name.as_str()) {return Err(Error::Contract);}sink.part(name.as_bytes(),&bytes);
            }
        }
        if expected.next().is_some() || content!=sink.finish().0 {return Err(Error::Contract);}
        Profile::ALL.into_iter().find(|p|p.name()==profile).ok_or(Error::Contract)
    }
    async fn verify_selection_sources(&mut self,a:&AdmissionData,profile:Profile)->Result<(),Error> {
        let stage=selection::build::stage(profile,&self.model)?;
        let _declarations=self.budget.reserve("selection-source-declarations",stage.inputs.len().saturating_mul(size_of::<stages::RelationUse>()+256))?;
        let order=super::vocabulary::publication_order(&mut self.connection,self.contract.generation()).await?;
        let consumed=ClassificationData::inputs();
        for invocation in a.invocations.iter() {
            let bytes=a.sources.iter().filter(|r|r.invocation==invocation.id()).try_fold(0usize,|n,r|n.checked_add(r.heap_bytes().saturating_mul(2)+size_of::<analysis::sources::SourceSnapshot>())).ok_or(Error::Contract)?;
            let _source_charge=self.budget.reserve("selection-source-lookup",bytes)?;
            let sources=a.sources.iter().filter(|r|r.invocation==invocation.id()).map(|r|r.source()).collect::<Vec<_>>();
            if sources.len()!=stage.inputs.len() {return Err(Error::Contract);}
            for declaration in &stage.inputs {
                let mut matching=sources.iter().filter(|r|r.relation()==declaration.name());
                let stored=matching.next().ok_or(Error::Contract)?;if matching.next().is_some() {return Err(Error::Contract);}
                if stored.model()!=self.model.digest() || stored.schedule()!=order.decode(0)?.schedule() {return Err(Error::Contract);}
                let relation=self.model.relations().iter().find(|r|r.name()==declaration.name()).ok_or(Error::Contract)?;
                let source:Option<(i64,Vec<u8>,Vec<u8>)>=sqlx::query_as("SELECT row_count,content_digest,schedule_digest FROM lctx_model_store.stage_receipts WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(self.generation().0.to_vec()).bind(stored.producer()).bind(stored.relation()).fetch_optional(&mut *self.connection).await?;
                if source.is_none_or(|(rows,content,schedule)|rows!=stored.rows() || content!=stored.content().0 || schedule!=stored.schedule().0) {return Err(Error::Contract);}
                let prefix=if let Some(boundary)=declaration.prefix() {Some(order.resolve(boundary)?)}else{
                    let epoch:Option<i16>=sqlx::query_scalar("SELECT epoch FROM lctx_model_store.publication_outputs WHERE generation_id=$1 AND stage_name=$2 AND relation_name=$3").bind(self.generation().0.to_vec()).bind(stored.producer()).bind(stored.relation()).fetch_optional(&mut *self.connection).await?;
                    if let Some(epoch)=epoch {
                        Some(order.decode(u16::try_from(epoch).map_err(|_|Error::Contract)?)?)
                    }else{
                        // Ordinary outputs inherit the maximum acknowledged-input prefix at
                        // completion. Their verified canonical capture is the durable metadata;
                        // they have no explicit publication_outputs group row.
                        if is_vocabulary(stored.relation()) {return Err(Error::Contract);}
                        stored.prefix().map(|name| {
                            let boundary=stages::PublicationBoundary::ALL.into_iter().find(|b|format!("{b:?}")==name).ok_or(Error::Contract)?;
                            Ok::<_,Error>(order.resolve(boundary)?)
                        }).transpose()?
                    }
                };
                let expected_prefix=prefix.map(|p|format!("{:?}",p.boundary()));
                let physical=if is_vocabulary(stored.relation()) {prefix.map_or_else(||stored.relation().to_owned(),|p|p.view(stored.relation()))}else{stored.relation().to_owned()};
                if stored.prefix()!=expected_prefix.as_deref() || stored.physical()!=physical {return Err(Error::Contract);}
                if let Some(prefix)=prefix {
                    let closed:Option<bool>=sqlx::query_scalar("SELECT closed FROM lctx_model_store.publication_groups WHERE generation_id=$1 AND epoch=$2").bind(self.generation().0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_|Error::Contract)?).fetch_optional(&mut *self.connection).await?;
                    if closed!=Some(true) {return Err(Error::Contract);}
                }
                if is_vocabulary(stored.relation()) {
                    let prefix=prefix.ok_or(Error::Contract)?;
                    let expected:Option<(i64,Vec<u8>)>=sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.epoch_receipts WHERE generation_id=$1 AND epoch=$2 AND relation_name=$3").bind(self.generation().0.to_vec()).bind(i16::try_from(prefix.ordinal()).map_err(|_|Error::Contract)?).bind(stored.relation()).fetch_optional(&mut *self.connection).await?;
                    if expected.is_none_or(|(rows,content)| rows!=stored.rows() || content!=stored.content().0) {return Err(Error::Contract);}
                }
                // Only classifier-consumed metadata is rehashed; producer reference closure is
                // bound by its canonical capture and validators, never hydrated as request data.
                if consumed.iter().any(|i|i.name()==stored.relation()) {
                    let actual=super::vocabulary::receipt(&mut self.connection,self.contract.generation(),relation,&physical,&self.budget).await?;
                    if i64::try_from(actual.rows).ok()!=Some(stored.rows()) || actual.content!=stored.content() {return Err(Error::Contract);}
                }
            }
        }
        Ok(())
    }
}
