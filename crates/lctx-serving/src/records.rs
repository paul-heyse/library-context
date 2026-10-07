//! Typed request-local access over the canonical batch adapter.
use lctx_model::domain::{Id, ModelError, Record, HeapSize, charged, resources::ResourceBudget};
use std::{any::Any,collections::BTreeMap,sync::{Arc,Mutex}};
use lctx_surrealdb::batches::CanonicalBatches;
fn decode_rows<R: Record>(source: &CanonicalBatches) -> Result<Vec<R>, ModelError> {
    source
        .batches
        .iter()
        .filter(|(name, _)| *name == R::NAME)
        .flat_map(|(_, batch)| {
            R::decode(batch)
                .map(|rows| rows.into_iter().map(Ok).collect::<Vec<_>>())
                .unwrap_or_else(|error| vec![Err(error)])
        })
        .collect()
}
pub fn wire(error: lctx_model::domain::serving::WireError) -> ModelError {
    ModelError::Serving(error.public_failure().kind)
}

/// One request's typed canonical view. Families decode once; handles retain their own charge.
/// The exact source borrow prevents reuse across a different native selection or pinned view.
pub struct Prepared<'a>{
    source:&'a CanonicalBatches,
    budget:ResourceBudget,
    cache:Mutex<BTreeMap<&'static str,Box<dyn Any+Send+Sync>>>,
    claims:Mutex<Option<Arc<crate::claims::Claims>>>,
}
impl<'a> Prepared<'a>{
    pub fn new(source:&'a CanonicalBatches,budget:&ResourceBudget)->Self{Self{source,budget:budget.clone(),cache:Mutex::new(BTreeMap::new()),claims:Mutex::new(None)}}
    pub fn claims(&self)->Result<Arc<crate::claims::Claims>,ModelError>{
        let mut claims=self.claims.lock().map_err(|_|ModelError::Schema("prepared claim cache"))?;
        if claims.is_none(){*claims=Some(Arc::new(crate::claims::Claims::new(self,&self.budget)?));}
        Ok(claims.as_ref().expect("prepared claims").clone())
    }
    pub fn source(&self)->&CanonicalBatches{self.source}
    pub fn budget(&self)->&ResourceBudget{&self.budget}
    pub fn rows<R:Record>(&self)->Result<PacketRows<R>,ModelError>{
        let mut cache=self.cache.lock().map_err(|_|ModelError::Schema("prepared canonical cache"))?;
        if !cache.contains_key(R::NAME){let rows=decode_rows::<R>(self.source)?;cache.insert(R::NAME,Box::new(PacketRows::new(rows,&self.budget)?));}
        cache.get(R::NAME).and_then(|rows|rows.downcast_ref::<PacketRows<R>>()).cloned().ok_or(ModelError::Schema("prepared canonical type"))
    }
}
impl std::ops::Deref for Prepared<'_>{type Target=CanonicalBatches;fn deref(&self)->&Self::Target{self.source}}
type IdPosition=([u8;16],usize);
type ReferencePosition=(&'static str,[u8;16],usize);
struct Positions{
    ids:Option<Vec<IdPosition>>,
    references:BTreeMap<&'static str,Vec<ReferencePosition>>,
    charge:charged::StateCharge,
}
struct FamilyRows<R:Record>{rows:Vec<R>,positions:Mutex<Positions>,budget:ResourceBudget}
#[derive(Clone)]
pub struct PacketRows<R:Record>{inner:Arc<FamilyRows<R>>}
impl<R:Record> PacketRows<R>{
    pub fn new(rows:Vec<R>,budget:&ResourceBudget)->Result<Self,ModelError>{
        let mut charge=charged::StateCharge::new(budget,"prepared-canonical-family");
        charge.grow(size_of::<FamilyRows<R>>().saturating_add(32).saturating_add(rows.capacity().saturating_mul(size_of::<R>())).saturating_add(rows.iter().map(HeapSize::heap_bytes).fold(0usize,usize::saturating_add)))?;
        Ok(Self::admitted(rows,budget,charge))
    }
    fn admitted(rows:Vec<R>,budget:&ResourceBudget,charge:charged::StateCharge)->Self{
        Self{inner:Arc::new(FamilyRows{rows,positions:Mutex::new(Positions{ids:None,references:BTreeMap::new(),charge}),budget:budget.clone()})}
    }
    pub fn rows(&self)->&[R]{&self.inner.rows}
    pub fn get(&self,id:Id<R>)->Result<Option<&R>,ModelError>{self.get_key(id.bytes())}
    pub fn get_key(&self,key:&[u8;16])->Result<Option<&R>,ModelError>{
        let mut positions=self.inner.positions.lock().map_err(|_|ModelError::Schema("prepared ID positions"))?;
        if positions.ids.is_none(){
            positions.charge.grow(self.inner.rows.len().saturating_mul(size_of::<IdPosition>()))?;
            let mut ids=self.inner.rows.iter().enumerate().map(|(position,row)|(*row.id().bytes(),position)).collect::<Vec<_>>();ids.sort_unstable();positions.ids=Some(ids);
        }
        let ids=positions.ids.as_deref().expect("prepared IDs");let first=ids.partition_point(|(candidate,_)|candidate<key);
        Ok(ids.get(first).filter(|(candidate,_)|candidate==key).map(|(_,position)|&self.inner.rows[*position]))
    }
    pub fn require_unique(&self)->Result<(),ModelError>{
        if let Some(row)=self.rows().first(){self.get(row.id())?;}
        let positions=self.inner.positions.lock().map_err(|_|ModelError::Schema("prepared ID positions"))?;
        if positions.ids.as_deref().unwrap_or(&[]).windows(2).any(|pair|pair[0].0==pair[1].0){return Err(ModelError::Conflict("capability companion identity"));}Ok(())
    }
    pub fn select_ids(&self,ids:&[Id<R>])->Result<Self,ModelError>{
        // Construct the lazy index through the same charged path, including empty demand.
        if let Some(row)=self.inner.rows.first(){self.get(row.id())?;}
        let positions=self.inner.positions.lock().map_err(|_|ModelError::Schema("prepared ID positions"))?;
        let index=positions.ids.as_deref().unwrap_or(&[]);
        self.selected(ids.iter().flat_map(|id|{
            let start=index.partition_point(|(key,_)|key<id.bytes());let end=index.partition_point(|(key,_)|key<=id.bytes());
            index[start..end].iter().map(|(_,position)|*position)
        }))
    }
    pub fn select_for<T:Record>(&self,field:&str,ids:&[Id<T>])->Result<Self,ModelError>{
        let field=R::fields().into_iter().find(|descriptor|descriptor.name()==field).ok_or_else(||ModelError::Invalid(format!("native evidence field missing: {}.{field}",R::NAME)))?.name();
        let mut positions=self.inner.positions.lock().map_err(|_|ModelError::Schema("prepared reference positions"))?;
        if !positions.references.contains_key(field){
            positions.charge.grow(32+size_of::<(&str,Vec<ReferencePosition>)>())?;
            let mut index:Vec<ReferencePosition>=Vec::new();
            for (position,row) in self.inner.rows.iter().enumerate(){
                let references=row.references().into_iter().filter(|reference|reference.field==field).collect::<Vec<_>>();
                let needed=index.len()+references.len();
                if needed>index.capacity(){positions.charge.grow((needed-index.capacity()).saturating_mul(size_of::<ReferencePosition>()))?;index.reserve_exact(needed-index.len());}
                index.extend(references.into_iter().map(|reference|(reference.target,reference.key,position)));
            }
            index.sort_unstable();index.dedup();positions.references.insert(field,index);
        }
        let index=&positions.references[field];
        self.selected(ids.iter().flat_map(|id|{
            let key=(T::NAME,*id.bytes());
            let start=index.partition_point(|(target,key_,_)|(*target,*key_)<key);let end=index.partition_point(|(target,key_,_)|(*target,*key_)<=key);
            index[start..end].iter().map(|(_,_,position)|*position)
        }))
    }
    /// Derived nominal keys use the same posting owner. The static consumer name and
    /// noncapturing model key function cannot depend on changing request parameters.
    pub(crate) fn select_derived<T:Record>(&self,owner:&'static str,ids:&[Id<T>],key:fn(&R)->Id<T>)->Result<Self,ModelError>{
        let mut positions=self.inner.positions.lock().map_err(|_|ModelError::Schema("prepared derived positions"))?;
        if !positions.references.contains_key(owner){
            positions.charge.grow(32+size_of::<(&str,Vec<ReferencePosition>)>()+self.rows().len().saturating_mul(size_of::<ReferencePosition>()))?;
            let mut index=self.rows().iter().enumerate().map(|(position,row)|(T::NAME,*key(row).bytes(),position)).collect::<Vec<_>>();index.sort_unstable();positions.references.insert(owner,index);
        }
        let index=&positions.references[owner];
        self.selected(ids.iter().flat_map(|id|{let key=(T::NAME,*id.bytes());let start=index.partition_point(|(target,key_,_)|(*target,*key_)<key);let end=index.partition_point(|(target,key_,_)|(*target,*key_)<=key);index[start..end].iter().map(|(_,_,position)|*position)}))
    }
    pub fn select(&self,mut predicate:impl FnMut(&R)->bool)->Result<Self,ModelError>{
        self.selected(self.rows().iter().enumerate().filter_map(|(position,row)|predicate(row).then_some(position)))
    }
    fn selected(&self,positions:impl IntoIterator<Item=usize>)->Result<Self,ModelError>{
        let mut charge=charged::StateCharge::new(&self.inner.budget,"prepared-canonical-selection");let mut selected=charged::ChargedSet::default();
        for position in positions{selected.insert(&mut charge,position)?;}
        let mut held=charged::StateCharge::new(&self.inner.budget,"prepared-canonical-family");
        let heap=selected.iter().map(|position|self.inner.rows[*position].heap_bytes()).fold(0usize,usize::saturating_add);
        held.grow(size_of::<FamilyRows<R>>().saturating_add(32).saturating_add(selected.len().saturating_mul(size_of::<R>())).saturating_add(heap))?;
        let rows=selected.iter().map(|position|self.inner.rows[*position].clone()).collect();
        Ok(Self::admitted(rows,&self.inner.budget,held))
    }
}
impl<R:Record> std::ops::Deref for PacketRows<R>{type Target=[R];fn deref(&self)->&Self::Target{&self.inner.rows}}
impl<'a,R:Record> IntoIterator for &'a PacketRows<R>{type Item=&'a R;type IntoIter=std::slice::Iter<'a,R>;fn into_iter(self)->Self::IntoIter{self.rows().iter()}}
pub fn rows<R:Record>(source:&Prepared<'_>)->Result<PacketRows<R>,ModelError>{source.rows()}
pub fn need<R:Record>(rows:&PacketRows<R>,id:Id<R>)->Result<&R,ModelError>{
    rows.get(id)?.ok_or_else(||ModelError::Invalid(format!("required native packet row missing: {}",R::NAME)))
}

#[cfg(test)]
mod tests{
    use super::*;
    use lctx_model::domain::input::{Package,Release};
    fn fixture()->Vec<Release>{let a=Package{name:"a".into()};let b=Package{name:"b".into()};vec![Release{package:b.id(),version:"2".into()},Release{package:a.id(),version:"1".into()},Release{package:a.id(),version:"3".into()},Release{package:a.id(),version:"1".into()}]}
    #[test]
    fn typed_indexes_preserve_order_roles_duplicates_and_family_identity(){
        let budget=ResourceBudget::fixed(1<<20).unwrap();let rows=PacketRows::new(fixture(),&budget).unwrap();let initial=budget.reserved();
        assert_eq!(rows.get(rows[1].id()).unwrap(),Some(&rows[1]));let indexed=budget.reserved();assert!(indexed>initial);
        rows.get(rows[1].id()).unwrap();assert_eq!(indexed,budget.reserved());
        let selected=rows.select_ids(&[rows[1].id(),rows[1].id(),rows[0].id()]).unwrap();assert_eq!(selected.rows(),&[rows[0].clone(),rows[1].clone(),rows[3].clone()]);drop(selected);
        let package=Package{name:"a".into()};let selected=rows.select_for("package",&[package.id(),package.id()]).unwrap();assert_eq!(selected.rows(),&rows.rows()[1..]);drop(selected);
        let after_reference=budget.reserved();let selected=rows.select_for("package",&[package.id()]).unwrap();drop(selected);assert_eq!(after_reference,budget.reserved());
        let foreign:Id<Release>=serde_json::from_value(serde_json::to_value(package.id().bytes()).unwrap()).unwrap();assert!(rows.select_for("package",&[foreign]).unwrap().is_empty());
        assert!(rows.select_for("version",&[package.id()]).unwrap().is_empty());assert!(rows.select_for("missing",&[package.id()]).is_err());
        assert!(rows.require_unique().is_err());let clone=rows.clone();drop(rows);assert!(budget.reserved()>0);drop(clone);assert_eq!(budget.reserved(),0);
    }
    #[test]
    fn lazy_index_refusal_preserves_existing_family_charge(){
        let probe=ResourceBudget::fixed(1<<20).unwrap();let rows=PacketRows::new(fixture(),&probe).unwrap();let base=probe.reserved();drop(rows);
        let budget=ResourceBudget::fixed(base+1).unwrap();let rows=PacketRows::new(fixture(),&budget).unwrap();
        assert!(matches!(rows.get(rows[0].id()),Err(ModelError::Resource{..})));assert_eq!(budget.reserved(),base);
        let package=Package{name:"a".into()};assert!(matches!(rows.select_for("package",&[package.id()]),Err(ModelError::Resource{..})));assert_eq!(budget.reserved(),base);
        drop(rows);assert_eq!(budget.reserved(),0);
    }
}
