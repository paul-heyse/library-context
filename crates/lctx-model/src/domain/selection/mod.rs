//! Finite catalog requirements and evidence selection; serving only lowers this contract.
pub mod admission;
pub mod algebra;
pub mod build;
pub mod classification;
pub mod evaluate;
mod facets;
pub mod frames;
mod inventory;
mod preparation;
pub mod specialization;
mod structural_facets;
mod vocabulary;
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
#[model(name="catalog_selection_invocations",invariants=frames::invariants,semantic_source=include_bytes!("frames.rs"))]
pub struct SelectionInvocation {
    #[model(key)]
    pub domain: Id<SelectionDomain>,
    #[model(key)]
    pub invocation: Id<analysis::selection::Invocation>,
}

pub mod source_fields;
