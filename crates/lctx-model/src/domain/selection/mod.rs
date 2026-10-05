//! Finite catalog requirements and evidence selection; serving only lowers this contract.
pub mod admission;
pub mod algebra;
pub mod build;
pub mod classification;
pub mod evaluate;
pub(crate) mod facets;
pub mod frames;
pub(crate) mod inventory;
pub(crate) mod preparation;
pub mod specialization;
pub(crate) mod structural_facets;
pub(crate) mod vocabulary;
use crate::domain::*;
pub use facets::*;
pub use vocabulary::*;
pub fn relations() -> Vec<Relation> {
    macro_rules! rows {($($f:ident:$ty:ty,)*)=>{vec![$(Relation::of::<$ty>()),*]};}
    let mut rows = crate::catalog_selection_outputs!(rows);
    rows.push(Relation::of::<SelectionInvocation>());
    rows
}
#[derive(Debug, Clone, PartialEq, Eq, crate::Domain)]
#[model(name="catalog_selection_invocations",invariant_refs=frames::invariants_refs,semantic_source=include_bytes!("frames.rs"))]
pub struct SelectionInvocation {
    #[model(key)]
    pub domain: Id<SelectionDomain>,
    #[model(key)]
    pub invocation: Id<analysis::selection::Invocation>,
}

pub mod source_fields;

