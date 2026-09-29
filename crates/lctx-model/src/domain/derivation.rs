//! Typed proof declarations and cycle checking. Targets come from nominal reference fields;
//! producers never supply relation-name strings to reinterpret an arbitrary identifier.
use std::any::TypeId;
use petgraph::{graphmap::DiGraphMap,algo::toposort};
use super::{Id,ModelError,Record,FieldValue};

#[derive(Debug,Clone)]
pub struct ReferenceColumn { name: &'static str, target: (TypeId,&'static str), nullable: bool }
impl ReferenceColumn {
    pub fn of<T: DerivationReference>(name: &'static str) -> Self {
        Self { name,target: T::target().expect("nominal reference"),nullable: T::NULLABLE }
    }
    pub fn name(&self) -> &'static str { self.name }
    pub fn target(&self) -> (TypeId,&'static str) { self.target }
    pub fn nullable(&self) -> bool { self.nullable }
}
#[derive(Debug,Clone)]
pub struct Derivation {
    pub rule: &'static str,
    /// None means that the proof row itself is its conclusion.
    pub conclusion: Option<ReferenceColumn>,
    /// The declared field name is the premise's role; the field's type owns its target.
    pub premises: Vec<ReferenceColumn>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct RowRef { relation: &'static str, id: [u8;16] }
impl RowRef {
    pub fn of<R: Record>(id: Id<R>) -> Self { Self { relation: R::NAME,id: *id.bytes() } }
    pub fn relation(&self) -> &'static str { self.relation }
    pub fn bytes(&self) -> &[u8;16] { &self.id }
}
pub trait DerivationReference: FieldValue { fn row_ref(&self) -> Option<RowRef>; }
impl<R: Record> DerivationReference for Id<R> { fn row_ref(&self) -> Option<RowRef> { Some(RowRef::of(*self)) } }
impl<R: Record> DerivationReference for Option<Id<R>> { fn row_ref(&self) -> Option<RowRef> { self.map(RowRef::of) } }
#[derive(Debug,Clone)]
pub struct Proof { pub source: RowRef,pub conclusion: RowRef,pub premises: Vec<RowRef> }
impl super::HeapSize for RowRef {}
impl super::HeapSize for Proof { fn heap_bytes(&self) -> usize { self.premises.heap_bytes() } }
/// Graph-map bookkeeping admitted per proof node or edge before the cycle check allocates it.
const GRAPH_ENTRY_BYTES: usize = 64;

/// Validates the finite, relation-qualified dependency graph of an assembled proof set.
/// This is a semantic work bound, not coordinated allocation accounting. The effect owner must
/// reserve the admitted proof buffers before calling it. Hitting the bound never certifies a DAG.
pub fn acyclic(proofs: &[Proof],max_work: usize) -> Result<(),ModelError> {
    // Admission precedes graph allocation; duplicate proof rows still count toward work.
    let mut count = 0usize;
    for proof in proofs {
        count = proof.premises.len().checked_add(1+usize::from(proof.source != proof.conclusion)).and_then(|size| count.checked_add(size))
            .filter(|work| *work <= max_work).ok_or_else(|| ModelError::Invalid("derivation work bound exceeded".into()))?;
    }
    let mut graph = DiGraphMap::<RowRef,()>::new();
    for proof in proofs {
        graph.add_node(proof.source);
        if proof.source != proof.conclusion { graph.add_edge(proof.conclusion,proof.source,()); }
        for premise in &proof.premises { graph.add_edge(proof.source,*premise,()); }
    }
    // Pinned petgraph toposort is iterative, unlike is_cyclic_directed. No recursive-stack limit
    // or graph-local numeric index becomes semantic identity; only the success/refusal is used.
    toposort(&graph,None).map(|_| ()).map_err(|_| ModelError::Invalid("cyclic derivation premises".into()))
}

/// The model constructs one common check across every declared proof relation. A cycle spanning
/// two step types is therefore checked together, rather than separately per producer.
pub(super) struct Check { sources: Vec<super::Relation>,charge: super::charged::StateCharge,proofs: super::charged::ChargedVec<Proof>,entries: usize }
impl Check {
    pub fn new(sources: Vec<super::Relation>,budget: &super::resources::ResourceBudget) -> Self {
        Self { sources,charge: super::charged::StateCharge::new(budget,"derivation_acyclic"),proofs: Default::default(),entries: 0 }
    }
}
impl super::InvariantCheck for Check {
    fn visit(&mut self,relation: &str,batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        let source = self.sources.iter().find(|s| s.name() == relation).ok_or_else(|| ModelError::Invalid("undeclared proof input".into()))?;
        for proof in source.proofs(batch)? {
            self.entries = self.entries.saturating_add(proof.premises.len()+2);
            self.proofs.push(&mut self.charge,proof)?;
        }
        Ok(())
    }
    /// The stored check is bounded by admitted memory: proof rows are already charged, and the
    /// cycle graph is admitted before it is built.
    fn finish(mut self: Box<Self>) -> Result<(),ModelError> {
        self.charge.grow(self.entries.saturating_mul(GRAPH_ENTRY_BYTES))?;
        acyclic(&self.proofs,usize::MAX)
    }
}

/// Derivation field annotations must name nominal reference types.
///
/// ```compile_fail
/// use lctx_model::Domain;
/// #[derive(Debug,Clone,PartialEq,Eq,Domain)]
/// #[model(name = "bad_proof",rule = "claims")]
/// struct Bad { #[model(key)] name: String, #[model(premise)] untyped_id: String }
/// ```
///
/// ```
/// use lctx_model::{Domain,domain::Id};
/// #[derive(Debug,Clone,PartialEq,Eq,Domain)]
/// #[model(name = "proof",rule = "follows")]
/// struct Proof { #[model(key)] name: String, #[model(premise)] prior: Option<Id<Proof>> }
/// ```
pub mod declaration_controls {}
