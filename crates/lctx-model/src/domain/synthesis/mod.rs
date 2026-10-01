//! Programmatic synthesis consumes typed qualified conclusions.
pub mod build;

pub mod documentary;
pub mod assertions;
pub mod seeds;
pub mod briefs;
pub fn relations()->Vec<crate::domain::Relation>{let mut rows=documentary::relations();rows.extend(assertions::relations());rows.extend(seeds::relations());rows.extend(briefs::relations());rows}
