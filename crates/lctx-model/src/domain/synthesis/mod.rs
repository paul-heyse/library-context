//! Programmatic synthesis consumes typed qualified conclusions.
pub mod build;
pub mod frames;
pub mod production;
pub mod observations;
pub mod summary;

pub mod documentary;
pub mod source_code;
pub mod source_setup;
pub mod assertions;
pub mod seeds;
pub mod automatic;
pub mod briefs;
pub fn relations()->Vec<crate::domain::Relation>{let mut rows=documentary::relations();rows.extend(assertions::relations());rows.extend(seeds::relations());rows.extend(briefs::relations());rows.push(crate::domain::Relation::of::<frames::Frame>());rows.push(crate::domain::Relation::of::<summary::SummaryFacet>());rows}
