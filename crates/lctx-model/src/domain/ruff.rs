//! Populated lexical/contextual characterization, separate from inferred types and runtime truth.
use super::{*, assertion::AssertionQualification, attribution::FactFamily, source::Occurrence};
use crate::{Assertion, Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ContextPhase { ActiveNode = 0, FinalReference = 1, FinalUnresolved = 2 }

#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "ruff_context_observations", validate = validate_context)]
#[assertion(support = RuffContextSupport, name = "ruff_context_supports", family = FactFamily::Lexical, subjects(subject, final_binding))]
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
    pub typing: Option<bool>,
    #[model(key)]
    pub typing_only_annotation: Option<bool>,
    #[model(key)]
    pub runtime_annotation: Option<bool>,
    #[model(key)]
    pub string_annotation: Option<bool>,
    #[model(key)]
    pub type_checking: Option<bool>,
    #[model(key)]
    pub qualified_name: Option<Vec<String>>,
    /// Final native references retain the exact canonical binding event when located.
    #[model(key)]
    pub final_binding: Option<Id<lexical::BindingEvent>>,
    #[model(key)]
    pub final_binding_location: Option<AttachmentStatus>,
    /// Present only for the native unresolved-reference phase.
    #[model(key)]
    pub unresolved_wildcard: Option<bool>,
    #[model(key)]
    pub unresolved_annotation_binding: Option<bool>,
}

/// Recognition consumes exact populated context, never a decorator's trailing spelling.
/// Missing or contradictory source observations do not establish a resolved name.
pub fn resolved_name(rows: &[RuffContextObservation], qualification: Id<AssertionQualification>, subject: Id<Occurrence>) -> Option<&[String]> {
    let mut matches = rows.iter().filter(|row| row.qualification == qualification && row.subject == subject && row.phase == ContextPhase::ActiveNode);
    let first = matches.next()?.qualified_name.as_deref()?;
    if matches.any(|row| row.qualified_name.as_deref() != Some(first)) { return None; }
    Some(first)
}

/// Correspondence is a located native answer, never a spelling-based identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AttachmentStatus { Located = 0, Unlocated = 1 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum NativeRelationLocation { Absent = 0, Located = 1, Unlocated = 2 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum RuffBindingKind {
    Annotation = 0,
    Argument = 1,
    NamedExprAssignment = 2,
    Assignment = 3,
    TypeParam = 4,
    LoopVar = 5,
    WithItemVar = 6,
    Global = 7,
    Nonlocal = 8,
    Builtin = 9,
    ClassDefinition = 10,
    FunctionDefinition = 11,
    Export = 12,
    FutureImport = 13,
    Import = 14,
    FromImport = 15,
    SubmoduleImport = 16,
    Deletion = 17,
    BoundException = 18,
    UnboundException = 19,
    DunderClassCell = 20,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum RuffDefinitionKind { Class = 0, NestedClass = 1, Function = 2, NestedFunction = 3, Method = 4 }
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "ruff_binding_observations", validate = validate_binding)]
#[assertion(support = RuffBindingSupport, name = "ruff_binding_supports", family = FactFamily::Lexical, subjects(event, scope, shadowed, outer_shadowed, definition_scope))]
pub struct RuffBindingObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub event: Id<lexical::BindingEvent>,
    #[model(key)] pub kind: RuffBindingKind,
    #[model(key)] pub native_name: String,
    #[model(key)] pub scope: Option<Id<lexical::LexicalScope>>,
    #[model(key)] pub scope_location: AttachmentStatus,
    #[model(key)] pub shadowed: Option<Id<lexical::BindingEvent>>,
    #[model(key)] pub shadowed_location: NativeRelationLocation,
    #[model(key)] pub outer_shadowed: Option<Id<lexical::BindingEvent>>,
    #[model(key)] pub outer_shadowed_location: NativeRelationLocation,
    #[model(key)] pub definition_scope: Option<Id<lexical::LexicalScope>>,
    #[model(key)] pub definition_scope_location: NativeRelationLocation,
    #[model(key)] pub typing: bool,
    #[model(key)] pub qualified_name: Option<Vec<String>>,
    #[model(key)] pub explicit_export: bool,
    #[model(key)] pub external: bool,
    #[model(key)] pub alias: bool,
    #[model(key)] pub nonlocal: bool,
    #[model(key)] pub global: bool,
    #[model(key)] pub deleted: bool,
    #[model(key)] pub invalid_all_format: bool,
    #[model(key)] pub invalid_all_object: bool,
    #[model(key)] pub private_declaration: bool,
    #[model(key)] pub unpacked_assignment: bool,
    #[model(key)] pub in_except_handler: bool,
    #[model(key)] pub annotated_type_alias: bool,
    #[model(key)] pub deferred_type_alias: bool,
    #[model(key)] pub in_assert_statement: bool,
    #[model(key)] pub lazy: bool,
}
fn relation_matches<T>(value: Option<Id<T>>, status: NativeRelationLocation) -> bool { value.is_some() == (status == NativeRelationLocation::Located) }
fn validate_binding(row: &RuffBindingObservation) -> Result<(),ModelError> {
    if row.scope.is_some() != (row.scope_location == AttachmentStatus::Located)
        || !relation_matches(row.shadowed,row.shadowed_location)
        || !relation_matches(row.outer_shadowed,row.outer_shadowed_location)
        || !relation_matches(row.definition_scope,row.definition_scope_location) {
        return Err(ModelError::Invalid("native Ruff binding location differs from retained correspondence".into()));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "ruff_definition_observations", validate = validate_definition)]
#[assertion(support = RuffDefinitionSupport, name = "ruff_definition_supports", family = FactFamily::Lexical, subjects(declaration, parent))]
pub struct RuffDefinitionObservation {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub declaration: Id<Occurrence>,
    #[model(key)] pub kind: RuffDefinitionKind,
    #[model(key)] pub name: Option<String>,
    #[model(key)] pub parent: Option<Id<Occurrence>>,
    /// Absent identifies the native module parent, not a missing member lookup.
    #[model(key)] pub parent_location: NativeRelationLocation,
}
fn validate_definition(row: &RuffDefinitionObservation) -> Result<(),ModelError> {
    if !relation_matches(row.parent,row.parent_location) || row.parent == Some(row.declaration) {
        return Err(ModelError::Invalid("native Ruff definition parent correspondence is invalid".into()));
    }
    Ok(())
}

fn validate_context(row: &RuffContextObservation) -> Result<(),ModelError> {
    let unresolved = row.phase == ContextPhase::FinalUnresolved;
    if row.reference_load.is_some() != (row.phase == ContextPhase::FinalReference)
        || row.final_binding_location.is_some() != (row.phase == ContextPhase::FinalReference)
        || row.final_binding.is_some() != (row.final_binding_location == Some(AttachmentStatus::Located))
        || row.unresolved_wildcard.is_some() != unresolved
        || row.unresolved_annotation_binding.is_some() != unresolved {
        return Err(ModelError::Invalid("Ruff context payload disagrees with its native phase".into()));
    }
    if !unresolved && [row.typing,row.typing_only_annotation,row.runtime_annotation,row.string_annotation,row.type_checking].iter().any(Option::is_none) {
        return Err(ModelError::Invalid("located Ruff context lacks native flags".into()));
    }
    Ok(())
}
