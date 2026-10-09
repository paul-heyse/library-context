//! Bounded native canonical graph reads and compact admission keys.
use crate::workspace::Workspace;
use lctx_model::domain::{self as d, graph::*, charged::StateCharge, ModelError};
use lctx_surrealdb::{compiler::CompilerRows,surrealdb::types::{Value,Number,Object,RecordId,RecordIdKey}};
use std::{collections::{BTreeMap,BTreeSet},sync::Arc};
use futures::stream::BoxStream;
use d::resources::Reservation;

pub(crate) fn object(value:Value)->Result<Object,ModelError>{if let Value::Object(value)=value{Ok(value)}else{Err(ModelError::Schema("native canonical object"))}}
fn hash(value:&str)->Result<d::ContentHash,ModelError>{let bytes=hex::decode(value).map_err(ModelError::codec)?;Ok(d::ContentHash(bytes.try_into().map_err(|_|ModelError::Schema("native graph hash width"))?))}
fn integer(value:Option<&Value>)->Result<Option<i64>,ModelError>{match value{Some(Value::Number(Number::Int(value)))=>Ok(Some(*value)),None|Some(Value::None|Value::Null)=>Ok(None),_=>Err(ModelError::Schema("native graph integer"))}}
fn payload(row:&Object)->Result<&[u8],ModelError>{match row.get("canonical"){Some(Value::Bytes(bytes))=>Ok(bytes),_=>Err(ModelError::Schema("native canonical bytes"))}}
fn verify(row:&Object,table:&str,id:d::ContentHash,content:d::ContentHash,kind:i64,subtype:Option<i16>)->Result<(),ModelError>{
    if row.get("id")!=Some(&Value::RecordId(RecordId::new(table,id.hex()))) || row.get("content")!=Some(&Value::String(content.hex())) || integer(row.get("kind"))?!=Some(kind) || integer(row.get("subtype"))?!=subtype.map(i64::from){return Err(ModelError::Conflict("native canonical graph metadata"));}Ok(())
}
fn entity(row:&Object)->Result<Entity,ModelError>{let entity:Entity=serde_json::from_slice(payload(row)?).map_err(ModelError::codec)?;verify(row,"entity",entity.id().0,entity.content(),i64::from(entity.kind() as u16),entity.subtype())?;entity.validate()?;Ok(entity)}
fn assertion(row:&Object)->Result<Assertion,ModelError>{let assertion:Assertion=serde_json::from_slice(payload(row)?).map_err(ModelError::codec)?;verify(row,"assertion",assertion.id().0,assertion.content(),i64::from(assertion.kind as u16),None)?;assertion.validate()?;Ok(assertion)}
fn stream<R:Send+'static>(rows:CompilerRows,workspace:Arc<Workspace>,decode:fn(&Object)->Result<R,ModelError>)->BoxStream<'static,Result<R,ModelError>>{
    Box::pin(futures::stream::try_unfold((rows,workspace,None::<Box<dyn Reservation>>),move |(mut rows,workspace,charge)|async move{
        drop(charge);workspace.cancellation().check()?;
        let Some(row)=rows.next().await? else{return Ok(None);};let row=object(row)?;
        let charge=workspace.budget().reserve("native-canonical-decode",payload(&row)?.len().saturating_mul(4).saturating_add(512))?;
        let record=decode(&row)?;Ok(Some((record,(rows,workspace,Some(charge)))))
    }))
}
pub async fn entities(workspace:Arc<Workspace>)->Result<BoxStream<'static,Result<Entity,ModelError>>,ModelError>{Ok(stream(workspace.native().scan_canonical(true,workspace.budget()).await?,workspace,entity))}
pub async fn assertions(workspace:Arc<Workspace>)->Result<BoxStream<'static,Result<Assertion,ModelError>>,ModelError>{Ok(stream(workspace.native().scan_canonical(false,workspace.budget()).await?,workspace,assertion))}
macro_rules! kinds {($unused:ident;$($variant:ident:$kind:ident=>$ty:ty,)*)=>{
    fn native_kind(value:i64)->Result<EntityKind,ModelError>{[$(EntityKind::$kind,)*].into_iter().find(|kind|i64::from(*kind as u16)==value).ok_or(ModelError::Schema("native graph entity kind"))}
};}
lctx_model::graph_entity_declarations!(kinds,unused);
struct Header{kind:EntityKind,subtype:Option<i16>,length:Option<u64>}
pub(crate) struct Lookup{entities:BTreeMap<EntityId,Header>,assertions:BTreeSet<AssertionId>,_charge:StateCharge}
impl Lookup{
    pub(crate) async fn load(workspace:&Arc<Workspace>)->Result<Self,ModelError>{
        let mut lookup=Self{entities:BTreeMap::new(),assertions:BTreeSet::new(),_charge:StateCharge::new(workspace.budget(),"native-final-graph-keys")};
        for entities in [true,false]{
            let mut rows=workspace.native().scan_graph_headers(entities,workspace.budget()).await?;
            while let Some(row)=rows.next().await?{
                workspace.cancellation().check()?;let row=object(row)?;
                let Some(Value::RecordId(id))=row.get("id") else{return Err(ModelError::Schema("native graph header ID"));};
                let RecordIdKey::String(key)=&id.key else{return Err(ModelError::Schema("native graph full key"));};let key=hash(key)?;
                lookup._charge.grow(192)?;
                if entities{let kind=native_kind(integer(row.get("kind"))?.ok_or(ModelError::Schema("native graph kind"))?)?;
                    let subtype=integer(row.get("subtype"))?.map(i16::try_from).transpose().map_err(ModelError::codec)?;
                    let length=if kind==EntityKind::Source{integer(row.get("source_length"))?.map(u64::try_from).transpose().map_err(ModelError::codec)?}else{None};
                    if lookup.entities.insert(EntityId(key),Header{kind,subtype,length}).is_some(){return Err(ModelError::Conflict("duplicate native graph header"));}
                }else if !lookup.assertions.insert(AssertionId(key)){return Err(ModelError::Conflict("duplicate native assertion header"));}
            }
        }
        Ok(lookup)
    }
}
impl GraphLookup for Lookup{
    fn entity_kind(&self,id:EntityId)->Result<Option<EntityKind>,ModelError>{Ok(self.entities.get(&id).map(|row|row.kind))}
    fn entity_subtype(&self,id:EntityId)->Result<Option<i16>,ModelError>{Ok(self.entities.get(&id).and_then(|row|row.subtype))}
    fn assertion_exists(&self,id:AssertionId)->Result<bool,ModelError>{Ok(self.assertions.contains(&id))}
    fn source_length(&self,id:EntityId)->Result<Option<u64>,ModelError>{Ok(self.entities.get(&id).and_then(|row|row.length))}
}
