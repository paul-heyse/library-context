//! Charged finite NextClosure/Duquenne–Guigues kernel. Nominal object/attribute identities enter
//! and leave the boundary; dense indices and equal-length bitsets remain private.
use fixedbitset::FixedBitSet;
use lctx_model::domain::{ModelError,charged::StateCharge,resources::{ResourceBudget,Reservation}};
fn invalid(message:&str)->ModelError {ModelError::Invalid(message.into())}
// 0.5.7 allocates SIMD blocks; a 64-byte tail allowance covers its supported block widths.
fn bit_bytes(bits:usize)->Result<usize,ModelError> {bits.div_ceil(8).checked_add(64+size_of::<FixedBitSet>()).ok_or_else(||invalid("concept bitset size overflow"))}
pub struct Context<O,A> {
    objects:Vec<O>,attributes:Vec<A>,rows:Vec<FixedBitSet>,cols:Vec<FixedBitSet>,
    budget:ResourceBudget,_reservation:Box<dyn Reservation>,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Concept<O,A> {pub extent:Vec<O>,pub intent:Vec<A>}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Implication<A> {pub premise:Vec<A>,pub conclusion:Vec<A>,pub support:usize}
pub struct Lattice<O,A> {concepts:Vec<Concept<O,A>>,implications:Vec<Implication<A>>,examined:usize,budget_reached:bool,_charge:StateCharge}
impl<O,A> Lattice<O,A> {
    pub fn concepts(&self)->&[Concept<O,A>] {&self.concepts}
    pub fn implications(&self)->&[Implication<A>] {&self.implications}
    pub fn examined(&self)->usize {self.examined}
    pub fn budget_reached(&self)->bool {self.budget_reached}
}
impl<O:Copy+Ord,A:Copy+Ord> Context<O,A> {
    pub fn new(objects:&[O],attributes:&[A],incidence:&[(O,A)],budget:&ResourceBudget)->Result<Self,ModelError> {
        let row=bit_bytes(attributes.len())?;let col=bit_bytes(objects.len())?;
        let bytes=objects.len().checked_mul(size_of::<O>()+row).and_then(|n|n.checked_add(attributes.len().checked_mul(size_of::<A>()+col)?)).and_then(|n|n.checked_add(size_of::<Self>()+4096)).ok_or_else(||invalid("concept context size overflow"))?;
        let reservation=budget.reserve("native-concept-context",bytes)?;
        let mut objects=objects.to_vec();let mut attributes=attributes.to_vec();objects.sort_unstable();attributes.sort_unstable();
        if objects.windows(2).any(|p|p[0]==p[1]) || attributes.windows(2).any(|p|p[0]==p[1]) {return Err(invalid("duplicate concept object or attribute"));}
        let mut rows=(0..objects.len()).map(|_|FixedBitSet::with_capacity(attributes.len())).collect::<Vec<_>>();
        let mut cols=(0..attributes.len()).map(|_|FixedBitSet::with_capacity(objects.len())).collect::<Vec<_>>();
        for (object,attribute) in incidence {
            let g=objects.binary_search(object).map_err(|_|invalid("concept incidence has foreign object"))?;
            let m=attributes.binary_search(attribute).map_err(|_|invalid("concept incidence has foreign attribute"))?;
            rows[g].insert(m);cols[m].insert(g);
        }
        Ok(Self {objects,attributes,rows,cols,budget:budget.clone(),_reservation:reservation})
    }
    pub fn objects(&self)->&[O] {&self.objects}
    pub fn attributes(&self)->&[A] {&self.attributes}
    fn extent(&self,intent:&FixedBitSet)->FixedBitSet {
        debug_assert_eq!(intent.len(),self.attributes.len());let mut out=FixedBitSet::with_capacity(self.objects.len());out.insert_range(..);for m in intent.ones() {out.intersect_with(&self.cols[m]);}out
    }
    fn intent(&self,extent:&FixedBitSet)->FixedBitSet {
        debug_assert_eq!(extent.len(),self.objects.len());let mut out=FixedBitSet::with_capacity(self.attributes.len());out.insert_range(..);for g in extent.ones() {out.intersect_with(&self.rows[g]);}out
    }
    /// The retained frequent-context enumeration, bounded by examined concepts plus pseudo-intents.
    /// A cap preserves already derived exact concepts/implications and marks enumeration incomplete.
    pub fn analyse(&self,min_support:usize,max_examined:usize)->Result<Lattice<O,A>,ModelError> {
        let m=self.attributes.len();let g=self.objects.len();let mb=bit_bytes(m)?;let gb=bit_bytes(g)?;
        let scratch=mb.checked_mul(8).and_then(|n|n.checked_add(gb.checked_mul(4)?)).and_then(|n|n.checked_add(4096)).ok_or_else(||invalid("concept scratch overflow"))?;
        let _scratch=self.budget.reserve("native-concept-scratch",scratch)?;
        let mut basis_charge=StateCharge::new(&self.budget,"native-concept-basis");
        let mut out=Lattice {concepts:vec![],implications:vec![],examined:0,budget_reached:false,_charge:StateCharge::new(&self.budget,"native-concept-results")};
        out._charge.grow(size_of::<Lattice<O,A>>())?;
        let mut basis:Vec<(FixedBitSet,FixedBitSet)>=vec![];
        let mut current=implication_closure(&FixedBitSet::with_capacity(m),&basis);
        if self.extent(&current).count_ones(..)<min_support {return Ok(out);}
        loop {
            if out.examined>=max_examined {out.budget_reached=true;return Ok(out);}
            let extent=self.extent(&current);let closed=self.intent(&extent);out.examined+=1;
            if closed==current {
                let bytes=extent.count_ones(..).checked_mul(2*size_of::<O>()).and_then(|n|n.checked_add(current.count_ones(..).checked_mul(2*size_of::<A>())?)).and_then(|n|n.checked_add(2*size_of::<Concept<O,A>>())).ok_or_else(||invalid("concept result size overflow"))?;
                out._charge.grow(bytes)?;
                out.concepts.push(Concept {extent:extent.ones().map(|i|self.objects[i]).collect(),intent:current.ones().map(|i|self.attributes[i]).collect()});
            }else {
                let mut conclusion=closed.clone();conclusion.difference_with(&current);
                let bytes=current.count_ones(..).checked_add(conclusion.count_ones(..)).and_then(|n|n.checked_mul(2*size_of::<A>())).and_then(|n|n.checked_add(2*size_of::<Implication<A>>())).ok_or_else(||invalid("implication result size overflow"))?;
                out._charge.grow(bytes)?;basis_charge.grow(mb.checked_mul(2).and_then(|n|n.checked_add(2*size_of::<(FixedBitSet,FixedBitSet)>())).ok_or_else(||invalid("concept basis size overflow"))?)?;
                out.implications.push(Implication {premise:current.ones().map(|i|self.attributes[i]).collect(),conclusion:conclusion.ones().map(|i|self.attributes[i]).collect(),support:extent.count_ones(..)});
                basis.push((current.clone(),closed));
            }
            let mut next=None;
            for i in (0..m).rev() {
                if current.contains(i) {continue;}
                let mut prefix=current.clone();prefix.remove_range(i..);prefix.insert(i);
                let candidate=implication_closure(&prefix,&basis);
                if (0..i).all(|j|candidate.contains(j)==current.contains(j)) && self.extent(&candidate).count_ones(..)>=min_support {next=Some(candidate);break;}
            }
            match next {Some(candidate)=>current=candidate,None=>return Ok(out)}
        }
    }
}
fn implication_closure(set:&FixedBitSet,basis:&[(FixedBitSet,FixedBitSet)])->FixedBitSet {
    let mut current=set.clone();loop {let mut changed=false;for (premise,closed) in basis {debug_assert_eq!(premise.len(),current.len());debug_assert_eq!(closed.len(),current.len());if premise.is_subset(&current) && premise.count_ones(..)<current.count_ones(..) && !closed.is_subset(&current) {current.union_with(closed);changed=true;}}if !changed {return current;}}
}
