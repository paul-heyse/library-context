//! Pure evidence lowering over one already-scoped canonical native selection.
use lctx_model::domain::{*,serving::*,resources::ResourceBudget};
use lctx_surrealdb::batches::CanonicalBatches;
use std::{collections::BTreeMap,any::Any,sync::Arc};
#[derive(Debug)]
pub enum EvidenceError {Contract,Codec(String),ResourceRefused(&'static str),Model(ModelError)}
impl From<ModelError> for EvidenceError{fn from(value:ModelError)->Self{Self::Model(value)}}
impl From<EvidenceError> for ModelError{fn from(value:EvidenceError)->Self{match value{EvidenceError::Contract=>Self::Schema("native evidence packet contract"),EvidenceError::Codec(s)=>Self::Invalid(s),EvidenceError::ResourceRefused(s)=>Self::Invalid(format!("resource_refused: {s}")),EvidenceError::Model(e)=>e}}}
pub struct PacketRows<R:Record>{rows:Vec<R>,_reservation:Box<dyn lctx_model::domain::resources::Reservation>}
impl<R:Record> PacketRows<R>{pub fn rows(&self)->&[R]{&self.rows}}
pub struct NativePackets<'a>{
    source:&'a CanonicalBatches,
    pub(crate) budget:ResourceBudget,
    pub(crate) charge:charged::StateCharge,
    cache:BTreeMap<&'static str,Box<dyn Any+Send+Sync>>,
    claims:crate::claims::Claims,
}
impl<'a> NativePackets<'a>{
    pub fn new(source:&'a CanonicalBatches,budget:&ResourceBudget)->Result<Self,ModelError>{Ok(Self{source,budget:budget.clone(),charge:charged::StateCharge::new(budget,"native-evidence-packets"),cache:BTreeMap::new(),claims:crate::claims::Claims::new(source,budget)?})}
    fn rows<R:Record>(&mut self)->Result<Arc<Vec<R>>,EvidenceError>{
        if !self.cache.contains_key(R::NAME){let rows=crate::records::rows::<R>(self.source)?;for row in &rows{self.charge.admit(row)?;}self.charge.grow(rows.capacity()*std::mem::size_of::<R>())?;self.cache.insert(R::NAME,Box::new(Arc::new(rows)));}
        self.cache.get(R::NAME).and_then(|v|v.downcast_ref::<Arc<Vec<R>>>()).cloned().ok_or(EvidenceError::Contract)
    }
    fn selected<R:Record>(&self,rows:Vec<R>)->Result<PacketRows<R>,EvidenceError>{let bytes=rows.iter().map(HeapSize::heap_bytes).sum::<usize>().saturating_add(rows.capacity()*std::mem::size_of::<R>());Ok(PacketRows{rows,_reservation:self.budget.reserve("native-evidence-selected-rows",bytes)?})}
    pub(crate) async fn read_ids<R:Record>(&mut self,ids:&[Id<R>])->Result<PacketRows<R>,EvidenceError>{let all=self.rows::<R>()?;self.selected(all.iter().filter(|r|ids.contains(&r.id())).cloned().collect())}
    pub(crate) async fn read_for<R:Record,T:Record>(&mut self,field:&str,ids:&[Id<T>])->Result<PacketRows<R>,EvidenceError>{
        if !R::fields().iter().any(|f|f.name()==field){return Err(EvidenceError::Model(ModelError::Invalid(format!("native evidence field missing: {}.{field}",R::NAME))))}
        let all=self.rows::<R>()?;self.selected(all.iter().filter(|r|r.references().iter().any(|reference|reference.field==field&&reference.target==T::NAME&&ids.iter().any(|id|id.bytes()==&reference.key))).cloned().collect())
    }
    pub(crate) async fn claim_basis(&mut self,q:&assertion::AssertionQualification)->Result<ClaimBasisPacket,EvidenceError>{Ok(self.claims.basis(q.id())?)}
}

/// Source dependencies are derived from the sole packet mapping declaration.
pub fn inputs()->Vec<ValidationInput>{
    use serving::mappings::PacketOutput;
    EvidencePacket::binding().lowered().sources.iter().map(|r|ValidationInput::of_relation(r,&["id"])).collect()
}
/// Lower all indivisible selected records before independently paging either section.
pub async fn sections(source:&CanonicalBatches,range:&OriginalRange,request:&Request,snapshot:&SnapshotHandle,channels:&ChannelState,budget:&ResourceBudget)->Result<(SectionPage<FlowInventoryPacket>,SectionPage<SourceCharacterizationPacket>),ModelError>{
    let mut packets=NativePackets::new(source,budget)?;
    let flows=packets.flow_inventory(range,usize::MAX).await.map_err(ModelError::from)?;
    let characterizations=packets.source_characterization(range,usize::MAX).await.map_err(ModelError::from)?;
    let flow_keys=flows.items.into_iter().map(|p|Ok((key(derivation::RowRef::of(p.inventory))?,p))).collect::<Result<Vec<_>,ModelError>>()?;
    let characterization_keys=characterizations.items.into_iter().map(|p|Ok((key(derivation::RowRef::of(p.characterization))?,p))).collect::<Result<Vec<_>,ModelError>>()?;
    let flow=crate::pagination::page(flow_keys,request,snapshot,channels,request.tool().name(),"flow_inventory",None,flows.availability).map_err(crate::records::wire)?;
    let characterization=crate::pagination::page(characterization_keys,request,snapshot,channels,request.tool().name(),"source_characterization",None,characterizations.availability).map_err(crate::records::wire)?;
    Ok((flow,characterization))
}

fn key(row:derivation::RowRef)->Result<ContentHash,ModelError>{match graph::target_for_row(row)?{graph::Target::Entity(id)=>Ok(id.0),graph::Target::Assertion(id)=>Ok(id.0),graph::Target::External{..}=>Err(ModelError::Schema("external evidence packet key"))}}
