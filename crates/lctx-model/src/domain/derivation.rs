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
pub(super) struct Check { sources: Vec<super::Relation>,proofs: Vec<Proof>,work: usize }
impl Check { pub fn new(sources: Vec<super::Relation>) -> Self { Self { sources,proofs: Vec::new(),work: 0 } } }
impl super::InvariantCheck for Check {
    fn visit(&mut self,relation: &str,batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        let source = self.sources.iter().find(|s| s.name() == relation).ok_or_else(|| ModelError::Invalid("undeclared proof input".into()))?;
        let arity = source.derivation().expect("declared proof source").premises.len();
        let added = arity.checked_add(2).and_then(|width| width.checked_mul(batch.num_rows()));
        self.work = added.and_then(|added| self.work.checked_add(added)).filter(|work| *work <= 1_000_000)
            .ok_or_else(|| ModelError::Invalid("derivation work bound exceeded".into()))?;
        self.proofs.extend(source.proofs(batch)?); Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { acyclic(&self.proofs,1_000_000) }
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
