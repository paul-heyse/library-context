//! Populated lexical/contextual characterization, separate from inferred types and runtime truth.
use super::{*, assertion::AssertionQualification, attribution::FactFamily, source::Occurrence};
use crate::{Assertion, Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ContextPhase { ActiveNode = 0, FinalReference = 1 }

#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "ruff_context_observations")]
#[assertion(support = RuffContextSupport, name = "ruff_context_supports", family = FactFamily::Lexical, subjects(subject))]
pub struct RuffContextObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub subject: Id<Occurrence>,
    #[model(key)]
    pub phase: ContextPhase,
    /// Active traversal nodes have no finalized reference mode; final reference rows retain it.
    #[model(key)]
    pub reference_load: Option<bool>,
    #[model(key)]
    pub typing: bool,
    #[model(key)]
    pub typing_only_annotation: bool,
    #[model(key)]
    pub runtime_annotation: bool,
    #[model(key)]
    pub string_annotation: bool,
    #[model(key)]
    pub type_checking: bool,
    #[model(key)]
    pub qualified_name: Option<Vec<String>>,
}

/// Recognition consumes exact populated context, never a decorator's trailing spelling.
/// Missing or contradictory source observations do not establish a resolved name.
pub fn resolved_name<'a>(rows: &'a [RuffContextObservation], qualification: Id<AssertionQualification>, subject: Id<Occurrence>) -> Option<&'a [String]> {
    let mut matches = rows.iter().filter(|row| row.qualification == qualification && row.subject == subject && row.phase == ContextPhase::ActiveNode);
    let first = matches.next()?.qualified_name.as_deref()?;
    if matches.any(|row| row.qualified_name.as_deref() != Some(first)) { return None; }
    Some(first)
}
