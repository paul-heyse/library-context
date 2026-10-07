//! Pure evidence lowering over one already-scoped canonical native selection.
use lctx_model::domain::{resources::ResourceBudget, serving::*, *};
use lctx_surrealdb::batches::CanonicalBatches;
use std::{any::Any, collections::BTreeMap};
#[derive(Debug)]
pub enum EvidenceError {
    Contract,
    Codec(String),
    ResourceRefused(&'static str),
    Model(ModelError),
}
impl From<ModelError> for EvidenceError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}
impl From<EvidenceError> for ModelError {
    fn from(value: EvidenceError) -> Self {
        match value {
            EvidenceError::Contract => Self::Schema("native evidence packet contract"),
            EvidenceError::Codec(s) => Self::Invalid(s),
            EvidenceError::ResourceRefused(s) => Self::Invalid(format!("resource_refused: {s}")),
            EvidenceError::Model(e) => e,
        }
    }
}
pub struct PacketRows<R: Record> {
    rows: Vec<R>,
    ids: Vec<([u8;16],usize)>,
    _reservation: Box<dyn lctx_model::domain::resources::Reservation>,
}
impl<R: Record> PacketRows<R> {
    pub fn rows(&self) -> &[R] { &self.rows }
    pub fn get(&self,id:Id<R>)->Option<&R>{
        let first=self.ids.partition_point(|(key,_)|key<id.bytes());
        self.ids.get(first).filter(|(key,_)|key==id.bytes()).map(|(_,position)|&self.rows[*position])
    }
}
type IdPosition=([u8;16],usize);
type ReferencePosition=(&'static str,&'static str,[u8;16],usize);
struct PreparedRows<R:Record>{
    rows:Vec<R>,
    ids:Option<Vec<IdPosition>>,
    references:BTreeMap<&'static str,Vec<ReferencePosition>>,
}
impl<R:Record> PreparedRows<R>{
    fn prepare_index(&mut self,field:Option<&str>,charge:&mut charged::StateCharge)->Result<(),ModelError>{
        if let Some(field)=field && !self.references.contains_key(field){
            let field=R::fields().into_iter().find(|descriptor|descriptor.name()==field).ok_or(ModelError::Schema("native evidence indexed field"))?.name();
            charge.grow(32+size_of::<(&str,Vec<ReferencePosition>)>())?;
            let mut index:Vec<ReferencePosition>=Vec::new();
            for (position,row) in self.rows.iter().enumerate(){
                let values=row.references().into_iter().filter(|reference|reference.field==field).collect::<Vec<_>>();
                let needed=index.len()+values.len();
                if needed>index.capacity(){
                    charge.grow((needed-index.capacity())*std::mem::size_of::<ReferencePosition>())?;
                    index.reserve_exact(needed-index.len());
                }
                index.extend(values.into_iter().map(|reference|(reference.field,reference.target,reference.key,position)));
            }
            index.sort_unstable();index.dedup();self.references.insert(field,index);
        }
        if field.is_none() && self.ids.is_none(){
            charge.grow(self.rows.len()*std::mem::size_of::<IdPosition>())?;
            let mut index=self.rows.iter().enumerate().map(|(position,row)|(*row.id().bytes(),position)).collect::<Vec<_>>();
            index.sort_unstable();self.ids=Some(index);
        }
        Ok(())
    }
    fn id_positions(&self,id:Id<R>)->&[IdPosition]{
        let index=self.ids.as_deref().expect("prepared ID index");
        let start=index.partition_point(|(key,_)|key<id.bytes());
        let end=index.partition_point(|(key,_)|key<=id.bytes());
        &index[start..end]
    }
    fn reference_positions<T:Record>(&self,field:&str,id:Id<T>)->&[ReferencePosition]{
        let index=self.references.get(field).expect("prepared reference index");
        let key=(field,T::NAME,*id.bytes());
        let start=index.partition_point(|(field,target,key_,_)|(*field,*target,*key_)<key);
        let end=index.partition_point(|(field,target,key_,_)|(*field,*target,*key_)<=key);
        &index[start..end]
    }
}
pub struct NativePackets<'a> {
    source: &'a CanonicalBatches,
    pub(crate) budget: ResourceBudget,
    pub(crate) charge: charged::StateCharge,
    cache: BTreeMap<&'static str, Box<dyn Any + Send + Sync>>,
    claims: crate::claims::Claims,
}
impl<'a> NativePackets<'a> {
    pub fn new(source: &'a CanonicalBatches, budget: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            source,
            budget: budget.clone(),
            charge: charged::StateCharge::new(budget, "native-evidence-packets"),
            cache: BTreeMap::new(),
            claims: crate::claims::Claims::new(source, budget)?,
        })
    }
    fn prepared<R:Record>(&mut self,field:Option<&str>)->Result<&PreparedRows<R>,EvidenceError>{
        if !self.cache.contains_key(R::NAME){
            let rows=crate::records::rows::<R>(self.source)?;
            for row in &rows {self.charge.admit(row)?;}
            self.charge.grow(rows.capacity()*std::mem::size_of::<R>())?;
            self.cache.insert(R::NAME,Box::new(PreparedRows{rows,ids:None,references:BTreeMap::new()}));
        }
        let prepared=self.cache.get_mut(R::NAME).and_then(|rows|rows.downcast_mut::<PreparedRows<R>>()).ok_or(EvidenceError::Contract)?;
        prepared.prepare_index(field,&mut self.charge)?;
        Ok(prepared)
    }
    fn selected<R: Record>(&self, rows: Vec<R>) -> Result<PacketRows<R>, EvidenceError> {
        let bytes = rows
            .iter()
            .map(HeapSize::heap_bytes)
            .sum::<usize>()
            .saturating_add(rows.capacity() * std::mem::size_of::<R>())
            .saturating_add(rows.len()*std::mem::size_of::<IdPosition>());
        let reservation=self.budget.reserve("native-evidence-selected-rows",bytes)?;
        let mut ids=rows.iter().enumerate().map(|(position,row)|(*row.id().bytes(),position)).collect::<Vec<_>>();
        ids.sort_unstable();
        Ok(PacketRows{rows,ids,_reservation:reservation})
    }
    pub(crate) async fn read_ids<R:Record>(&mut self,ids:&[Id<R>])->Result<PacketRows<R>,EvidenceError>{
        let mut charge=charged::StateCharge::new(&self.budget,"native-evidence-selection-index");
        let prepared=self.prepared::<R>(None)?;
        let mut selected=charged::ChargedSet::<usize>::default();
        for id in ids {for (_,position) in prepared.id_positions(*id){selected.insert(&mut charge,*position)?;}}
        let rows=selected.iter().map(|position|prepared.rows[*position].clone()).collect();
        self.selected(rows)
    }
    pub(crate) async fn read_for<R:Record,T:Record>(&mut self,field:&str,ids:&[Id<T>])->Result<PacketRows<R>,EvidenceError>{
        if !R::fields().iter().any(|descriptor|descriptor.name()==field){
            return Err(EvidenceError::Model(ModelError::Invalid(format!("native evidence field missing: {}.{field}",R::NAME))));
        }
        let mut charge=charged::StateCharge::new(&self.budget,"native-evidence-selection-index");
        let prepared=self.prepared::<R>(Some(field))?;
        let mut selected=charged::ChargedSet::<usize>::default();
        for id in ids {for (_,_,_,position) in prepared.reference_positions(field,*id){selected.insert(&mut charge,*position)?;}}
        let rows=selected.iter().map(|position|prepared.rows[*position].clone()).collect();
        self.selected(rows)
    }

    pub(crate) async fn claim_basis(
        &mut self,
        q: &assertion::AssertionQualification,
    ) -> Result<ClaimBasisPacket, EvidenceError> {
        Ok(self.claims.basis(q.id())?)
    }
}

/// Source dependencies are derived from the sole packet mapping declaration.
pub fn inputs() -> Vec<ValidationInput> {
    use serving::mappings::PacketOutput;
    EvidencePacket::binding()
        .lowered()
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect()
}
/// Proof dependencies permit outgoing reads, not incoming ownership of every graph family.
pub fn owner_inputs() -> Vec<ValidationInput> {
    use serving::mappings::PacketOutput;
    EvidencePacket::binding()
        .mapping
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect()
}
/// Lower all indivisible selected records before independently paging either section.
pub async fn sections(
    source: &CanonicalBatches,
    range: &OriginalRange,
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
    budget: &ResourceBudget,
) -> Result<
    (
        SectionPage<FlowInventoryPacket>,
        SectionPage<SourceCharacterizationPacket>,
    ),
    ModelError,
> {
    let mut packets = NativePackets::new(source, budget)?;
    let flows = packets
        .flow_inventory(range, usize::MAX)
        .await
        .map_err(ModelError::from)?;
    let characterizations = packets
        .source_characterization(range, usize::MAX)
        .await
        .map_err(ModelError::from)?;
    let flow_keys = flows
        .items
        .into_iter()
        .map(|p| Ok((key(derivation::RowRef::of(p.inventory))?, p)))
        .collect::<Result<Vec<_>, ModelError>>()?;
    let characterization_keys = characterizations
        .items
        .into_iter()
        .map(|p| Ok((key(derivation::RowRef::of(p.characterization))?, p)))
        .collect::<Result<Vec<_>, ModelError>>()?;
    let flow = crate::pagination::page(
        flow_keys,
        request,
        snapshot,
        channels,
        request.tool().name(),
        "flow_inventory",
        None,
        flows.availability,
    )
    .map_err(crate::records::wire)?;
    let characterization = crate::pagination::page(
        characterization_keys,
        request,
        snapshot,
        channels,
        request.tool().name(),
        "source_characterization",
        None,
        characterizations.availability,
    )
    .map_err(crate::records::wire)?;
    Ok((flow, characterization))
}

fn key(row: derivation::RowRef) -> Result<ContentHash, ModelError> {
    match graph::target_for_row(row)? {
        graph::Target::Entity(id) => Ok(id.0),
        graph::Target::Assertion(id) => Ok(id.0),
        graph::Target::External { .. } => Err(ModelError::Schema("external evidence packet key")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::input::{Package,Release};
    fn rows()->PreparedRows<Release>{
        let a=Package{name:"a".into()};let b=Package{name:"b".into()};
        PreparedRows{rows:vec![Release{package:b.id(),version:"2".into()},Release{package:a.id(),version:"1".into()},Release{package:a.id(),version:"3".into()},Release{package:a.id(),version:"1".into()}],ids:None,references:BTreeMap::new()}
    }
    #[test]
    fn typed_indexes_keep_source_order_duplicate_rows_and_reference_target_type(){
        let budget=ResourceBudget::fixed(1<<20).unwrap();
        let mut charge=charged::StateCharge::new(&budget,"index-control");
        let mut prepared=rows();prepared.prepare_index(None,&mut charge).unwrap();prepared.prepare_index(Some("package"),&mut charge).unwrap();
        let reserved=budget.reserved();assert!(reserved>0);
        prepared.prepare_index(None,&mut charge).unwrap();prepared.prepare_index(Some("package"),&mut charge).unwrap();assert_eq!(reserved,budget.reserved(),"repeat reads reuse their charged indexes");
        assert_eq!(prepared.id_positions(prepared.rows[1].id()).iter().map(|(_,position)|*position).collect::<Vec<_>>(),[1,3]);
        let package=Package{name:"a".into()};
        assert_eq!(prepared.reference_positions("package",package.id()).iter().map(|(_,_,_,position)|*position).collect::<Vec<_>>(),[1,2,3]);
        prepared.prepare_index(Some("version"),&mut charge).unwrap();assert!(prepared.reference_positions("version",package.id()).is_empty());
        let foreign:Id<Release>=serde_json::from_value(serde_json::to_value(package.id().bytes()).unwrap()).unwrap();
        assert!(prepared.reference_positions("package",foreign).is_empty(),"equal bytes from another relation do not match");
        let missing=Release{package:package.id(),version:"missing".into()};assert!(prepared.id_positions(missing.id()).is_empty());
        drop(prepared);drop(charge);assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn index_refusal_happens_before_retaining_a_partial_index(){
        let budget=ResourceBudget::fixed(1).unwrap();let mut charge=charged::StateCharge::new(&budget,"index-control");let mut prepared=rows();
        assert!(matches!(prepared.prepare_index(None,&mut charge),Err(ModelError::Resource{..})));assert!(prepared.ids.is_none());assert_eq!(budget.reserved(),0);
        assert!(matches!(prepared.prepare_index(Some("package"),&mut charge),Err(ModelError::Resource{..})));assert!(prepared.references.is_empty());assert_eq!(budget.reserved(),0);
    }
}
