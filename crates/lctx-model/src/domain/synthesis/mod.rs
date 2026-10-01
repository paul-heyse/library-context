//! Programmatic synthesis consumes typed qualified conclusions.
pub mod build;

pub mod documentary;
pub fn relations()->Vec<crate::domain::Relation>{documentary::relations()}
