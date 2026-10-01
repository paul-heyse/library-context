//! Programmatic synthesis consumes typed qualified conclusions.
pub mod build;

pub mod documentary;
pub mod assertions;
pub fn relations()->Vec<crate::domain::Relation>{let mut rows=documentary::relations();rows.extend(assertions::relations());rows}
