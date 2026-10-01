//! Finite catalog requirements and evidence selection; serving only lowers this contract.
pub mod algebra;
pub mod build;
pub mod evaluate;
pub mod frames;
mod inventory;
mod vocabulary;
use crate::domain::*;
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
