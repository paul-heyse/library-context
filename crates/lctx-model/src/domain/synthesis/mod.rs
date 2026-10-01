//! Programmatic synthesis consumes typed qualified conclusions.
pub mod build;
pub mod frames;
pub mod observations;
pub mod patterns;
pub mod production;
pub mod summary;

pub mod assertions;
pub mod automatic;
pub mod briefs;
pub mod documentary;
pub mod documentary_templates;
pub mod seeds;
pub mod source_code;
pub mod source_setup;
pub fn relations() -> Vec<crate::domain::Relation> {
    let mut rows = documentary::relations();
    rows.extend(patterns::relations());
    rows.extend(assertions::relations());
    rows.extend(seeds::relations());
    rows.extend(briefs::relations());
    rows.push(crate::domain::Relation::of::<frames::Frame>());
    rows.push(crate::domain::Relation::of::<summary::SummaryFacet>());
    rows
}
