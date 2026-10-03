//! Typed syntax facts from the one Ruff parse (cutover plan A3; ADR-0089). Occurrences carry
//! identity; these provider-qualified records carry an occurrence's placement in the parse, the
//! detail its bytes do not state, and the syntax of declarations, decorators, imports, `__all__`,
//! parameters and class fields. A text is the source bytes at its occurrence, so a record stores
//! only kinds, ordinals and values the bytes do not say.
//!
//! Subject boundaries disclose what a provider could not state for a scope it covers, and
//! attachment outcomes keep the candidates of a provider event that did not attach exactly: an
//! unknown is never recorded as an absence.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{
    assertion::AssertionQualification,
    attribution::{AnalysisContext, CoverageStatus, FactFamily, Provider, ProviderCoverage},
    calls::ParameterKind,
    lexical::SyntaxField,
    obligation::ObligationKind,
    source::{CoverageScope, Module, Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind},
    value::{Literal, LiteralSet},
    *,
};
use crate::{Assertion, Domain, DomainCode, DomainSum};

fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

/// Where an occurrence sits in the parse: its nearest placed ancestor (none for a module), the
/// ancestor's field that holds it, and its ordinal among that field's placed children.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "syntax_placements", validate = validate_placement, invariants = syntax_invariants)]
#[assertion(support = SyntaxPlacementSupport, name = "syntax_placement_supports", family = FactFamily::Syntax, subjects(occurrence, parent))]
pub struct SyntaxPlacement {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub occurrence: Id<Occurrence>,
    #[model(key)]
    pub parent: Option<Id<Occurrence>>,
    #[model(key)]
    pub field: SyntaxField,
    #[model(key)]
    pub ordinal: i64,
}
fn validate_placement(row: &SyntaxPlacement) -> Result<(), ModelError> {
    if row.ordinal < 0 || row.parent == Some(row.occurrence) {
        return Err(invalid(
            "a placement needs a nonnegative ordinal and another parent",
        ));
    }
    Ok(())
}

/// Operators, by their Ruff kinds. Append-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum OperatorKind {
    Add = 0,
    Sub = 1,
    Mult = 2,
    MatMult = 3,
    Div = 4,
    Mod = 5,
    Pow = 6,
    LShift = 7,
    RShift = 8,
    BitOr = 9,
    BitXor = 10,
    BitAnd = 11,
    FloorDiv = 12,
    And = 13,
    Or = 14,
    Not = 15,
    Invert = 16,
    UAdd = 17,
    USub = 18,
    Eq = 19,
    NotEq = 20,
    Lt = 21,
    LtE = 22,
    Gt = 23,
    GtE = 24,
    Is = 25,
    IsNot = 26,
    In = 27,
    NotIn = 28,
}
/// What an occurrence's bytes do not state directly: an operator's kind, or a literal's value
/// (escapes resolved, implicit concatenation joined).
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "syntax_details")]
pub enum SyntaxDetail {
    #[model(code = 0)]
    Operator { operator: OperatorKind },
    #[model(code = 1)]
    Literal { literal: Id<Literal> },
    #[model(code = 2)]
    WithMode { is_async: bool },
}
/// A provider states an occurrence's `ordinal`th detail (a comparison chain has several).
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "syntax_detail_observations", validate = validate_detail)]
#[assertion(support = SyntaxDetailSupport, name = "syntax_detail_supports", family = FactFamily::Syntax, subjects(occurrence))]
pub struct SyntaxDetailObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub occurrence: Id<Occurrence>,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub detail: Id<SyntaxDetail>,
}
fn validate_detail(row: &SyntaxDetailObservation) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("a syntax detail ordinal is nonnegative"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DeclarationKind {
    Function = 0,
    AsyncFunction = 1,
    Class = 2,
}
/// A `def` or `class` statement: its name occurrence, enclosing declaration, `@overload` marker
/// and docstring expression.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "declaration_observations", validate = validate_declaration)]
#[assertion(support = DeclarationSupport, name = "declaration_supports", family = FactFamily::Syntax, subjects(declaration, name, parent, docstring))]
pub struct DeclarationObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub declaration: Id<Occurrence>,
    #[model(key)]
    pub name: Id<Occurrence>,
    #[model(key)]
    pub kind: DeclarationKind,
    #[model(key)]
    pub parent: Option<Id<Occurrence>>,
    #[model(key)]
    pub overload: bool,
    #[model(key)]
    pub docstring: Option<Id<Occurrence>>,
}
fn validate_declaration(row: &DeclarationObservation) -> Result<(), ModelError> {
    if row.name == row.declaration
        || row.parent == Some(row.declaration)
        || row.docstring == Some(row.declaration)
    {
        return Err(invalid(
            "a declaration's name, parent and docstring are other occurrences",
        ));
    }
    Ok(())
}
/// A declaration's decorator, in source order.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "declaration_decorators", validate = validate_decorator)]
#[assertion(support = DeclarationDecoratorSupport, name = "declaration_decorator_supports", family = FactFamily::Syntax, subjects(declaration, decorator))]
pub struct DeclarationDecorator {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub declaration: Id<Occurrence>,
    #[model(key)]
    pub decorator: Id<Occurrence>,
    #[model(key)]
    pub ordinal: i64,
}
fn validate_decorator(row: &DeclarationDecorator) -> Result<(), ModelError> {
    if row.ordinal < 0 || row.decorator == row.declaration {
        return Err(invalid(
            "a decorator has a nonnegative ordinal and its own occurrence",
        ));
    }
    Ok(())
}
/// One alias of an `import` or `from … import` statement. The module, name and alias texts are the
/// statement's and alias's bytes; `resolved_module` is only an absolute source spelling,
/// relative levels resolved against the importing module, when they do not climb past its top.
/// Actual per-alias lookup identity is asserted by `ModuleResolutionObservation`.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "import_alias_observations", validate = validate_import)]
#[assertion(support = ImportAliasSupport, name = "import_alias_supports", family = FactFamily::Exports, subjects(statement, alias))]
pub struct ImportAliasObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    #[model(key)]
    pub alias: Id<Occurrence>,
    #[model(key)]
    pub level: i64,
    #[model(key)]
    pub resolved_module: Option<String>,
}
fn validate_import(row: &ImportAliasObservation) -> Result<(), ModelError> {
    if row.level < 0
        || row.statement == row.alias
        || row.resolved_module.as_ref().is_some_and(|m| m.is_empty())
    {
        return Err(invalid(
            "an import alias needs a nonnegative level, its own occurrence and a nonempty resolved module",
        ));
    }
    Ok(())
}
/// An `__all__` statement: whether it is a literal list or tuple of strings, and then its names.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "dunder_all_observations", validate = validate_dunder_all)]
#[assertion(support = DunderAllSupport, name = "dunder_all_supports", family = FactFamily::Exports, subjects(statement))]
pub struct DunderAllObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    #[model(key)]
    pub literal: bool,
    #[model(key)]
    pub names: Option<Id<LiteralSet>>,
}
fn validate_dunder_all(row: &DunderAllObservation) -> Result<(), ModelError> {
    if row.literal != row.names.is_some() {
        return Err(invalid("exactly a literal __all__ states its names"));
    }
    Ok(())
}
/// A parameter as written: its function (a `def` or `lambda`), ordinal and kind, its default
/// expression and, when that default is a literal, its value, and its annotation.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "parameter_syntax_observations", validate = validate_parameter)]
#[assertion(support = ParameterSyntaxSupport, name = "parameter_syntax_supports", family = FactFamily::Signatures, subjects(function, parameter, default, annotation))]
pub struct ParameterSyntaxObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub function: Id<Occurrence>,
    #[model(key)]
    pub parameter: Id<Occurrence>,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub kind: ParameterKind,
    #[model(key)]
    pub default: Option<Id<Occurrence>>,
    #[model(key)]
    pub default_literal: Option<Id<Literal>>,
    #[model(key)]
    pub annotation: Option<Id<Occurrence>>,
}
fn validate_parameter(row: &ParameterSyntaxObservation) -> Result<(), ModelError> {
    if row.ordinal < 0
        || row.function == row.parameter
        || (row.default_literal.is_some() && row.default.is_none())
        || (row.default.is_some()
            && matches!(
                row.kind,
                ParameterKind::VarPositional | ParameterKind::VarKeyword
            ))
    {
        return Err(invalid(
            "a parameter needs a nonnegative ordinal, and a literal default needs its default; variadics have none",
        ));
    }
    Ok(())
}
/// A class-body field as written: `name: annotation = value`, either part optional.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "class_field_syntax_observations", validate = validate_field)]
#[assertion(support = ClassFieldSyntaxSupport, name = "class_field_syntax_supports", family = FactFamily::Syntax, subjects(class, target, annotation, value))]
pub struct ClassFieldSyntaxObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub class: Id<Occurrence>,
    #[model(key)]
    pub target: Id<Occurrence>,
    #[model(key)]
    pub annotation: Option<Id<Occurrence>>,
    #[model(key)]
    pub value: Option<Id<Occurrence>>,
}
fn validate_field(row: &ClassFieldSyntaxObservation) -> Result<(), ModelError> {
    if row.class == row.target || (row.annotation.is_none() && row.value.is_none()) {
        return Err(invalid(
            "a class field has its own target and an annotation or a value",
        ));
    }
    Ok(())
}

/// A stop a provider disclosed in a scope it covers: the family, the subject when one exists, the
/// reason and a bounded detail. Its coverage of that scope is never complete.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "subject_boundaries", validate = validate_boundary, invariants = boundary_invariants)]
pub struct SubjectBoundary {
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key, provenance)]
    pub provider: Id<Provider>,
    #[model(key, provenance)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub family: FactFamily,
    #[model(key)]
    pub subject: Option<Id<Occurrence>>,
    #[model(key)]
    pub reason: ObligationKind,
    #[model(key)]
    pub detail: Option<String>,
}
fn validate_boundary(row: &SubjectBoundary) -> Result<(), ModelError> {
    if row.detail.as_ref().is_some_and(|d| d.len() > 4096) {
        return Err(invalid("a boundary detail is at most 4096 bytes"));
    }
    Ok(())
}
/// Why a provider event did not attach to an occurrence (ADR-0089: only an exact source, span,
/// kind and role match attaches).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AttachmentKind {
    Innermost = 0,
    Ambiguous = 1,
    Unmatched = 2,
    BudgetExceeded = 3,
}
impl AttachmentKind {
    /// The boundary reason that discloses this outcome.
    pub fn reason(self) -> ObligationKind {
        match self {
            Self::Ambiguous => ObligationKind::AttachmentAmbiguous,
            Self::Innermost | Self::Unmatched => ObligationKind::AttachmentUnmatched,
            Self::BudgetExceeded => ObligationKind::BudgetReached,
        }
    }
}
/// A provider event that did not attach: its coordinates and outcome, under its boundary.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "attachment_outcomes", validate = validate_outcome)]
pub struct AttachmentOutcome {
    #[model(key)]
    pub boundary: Id<SubjectBoundary>,
    #[model(key)]
    pub source: Id<SourceArtifact>,
    #[model(key)]
    pub start: i64,
    #[model(key)]
    pub end: i64,
    #[model(key)]
    pub syntax_kind: Option<SyntaxKind>,
    #[model(key)]
    pub role: Option<OccurrenceRole>,
    #[model(key)]
    pub outcome: AttachmentKind,
}
fn validate_outcome(row: &AttachmentOutcome) -> Result<(), ModelError> {
    if row.start < 0 || row.end < row.start {
        return Err(invalid("an attachment outcome needs an ordered span"));
    }
    Ok(())
}
/// A candidate occurrence of an unattached event, kept rather than chosen.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "attachment_candidates")]
pub struct AttachmentCandidate {
    #[model(key)]
    pub outcome: Id<AttachmentOutcome>,
    #[model(key)]
    pub candidate: Id<Occurrence>,
}

/// One occurrence's source, span and syntax kind.
type Placed = (Id<SourceArtifact>, i64, i64, i16);
fn contains(outer: &Placed, inner: &Placed) -> bool {
    outer.0 == inner.0 && outer.1 <= inner.1 && inner.2 <= outer.2
}
fn kind(placed: &Placed, kinds: &[SyntaxKind]) -> bool {
    kinds.iter().any(|k| *k as i16 == placed.3)
}

/// Syntax records describe occurrences of one source, nested as the parse nests them.
fn syntax_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "syntax_geometry",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<SyntaxPlacement>(&["id"]),
            ValidationInput::of::<DeclarationObservation>(&["id"]),
            ValidationInput::of::<DeclarationDecorator>(&["id"]),
            ValidationInput::of::<ImportAliasObservation>(&["id"]),
            ValidationInput::of::<DunderAllObservation>(&["id"]),
            ValidationInput::of::<ParameterSyntaxObservation>(&["id"]),
            ValidationInput::of::<ClassFieldSyntaxObservation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(SyntaxGeometry {
                charge: StateCharge::new(budget, "syntax_geometry"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct SyntaxGeometry {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, Placed>,
    placed: ChargedSet<(Id<AssertionQualification>, Id<Occurrence>)>,
}
impl SyntaxGeometry {
    fn get(&self, id: Id<Occurrence>) -> Result<Placed, ModelError> {
        self.occurrences
            .get(&id)
            .copied()
            .ok_or_else(|| invalid("a syntax subject is absent"))
    }
    fn within(
        &self,
        outer: Id<Occurrence>,
        inner: Option<Id<Occurrence>>,
        what: &str,
    ) -> Result<(), ModelError> {
        let Some(inner) = inner else {
            return Ok(());
        };
        if !contains(&self.get(outer)?, &self.get(inner)?) {
            return Err(ModelError::Invalid(format!(
                "{what} lies outside its occurrence"
            )));
        }
        Ok(())
    }
}
impl InvariantCheck for SyntaxGeometry {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        use SyntaxKind::*;
        if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences.insert(
                    &mut self.charge,
                    row.id(),
                    (row.source, row.start, row.end, row.syntax_kind as i16),
                )?;
            }
        } else if relation == SyntaxPlacement::NAME {
            for row in SyntaxPlacement::decode(batch)? {
                if !self
                    .placed
                    .insert(&mut self.charge, (row.qualification, row.occurrence))?
                {
                    return Err(invalid("an occurrence has one placement per qualification"));
                }
                match row.parent {
                    Some(parent) => self.within(parent, Some(row.occurrence), "a placed child")?,
                    None if !kind(&self.get(row.occurrence)?, &[ModModule, ModExpression]) => {
                        return Err(invalid("only a module is placed without a parent"));
                    }
                    None => {}
                }
            }
        } else if relation == DeclarationObservation::NAME {
            for row in DeclarationObservation::decode(batch)? {
                let declaration = self.get(row.declaration)?;
                let expected: &[SyntaxKind] = if row.kind == DeclarationKind::Class {
                    &[StmtClassDef]
                } else {
                    &[StmtFunctionDef]
                };
                if !kind(&declaration, expected) || !kind(&self.get(row.name)?, &[Identifier]) {
                    return Err(invalid(
                        "a declaration's statement and name have the declared kinds",
                    ));
                }
                self.within(row.declaration, Some(row.name), "a declaration's name")?;
                self.within(row.declaration, row.docstring, "a docstring")?;
                if let Some(parent) = row.parent {
                    if !kind(&self.get(parent)?, &[StmtFunctionDef, StmtClassDef]) {
                        return Err(invalid("a declaration's parent is a def or class"));
                    }
                    self.within(parent, Some(row.declaration), "a nested declaration")?;
                }
            }
        } else if relation == DeclarationDecorator::NAME {
            for row in DeclarationDecorator::decode(batch)? {
                if !kind(&self.get(row.decorator)?, &[Decorator])
                    || !kind(
                        &self.get(row.declaration)?,
                        &[StmtFunctionDef, StmtClassDef],
                    )
                {
                    return Err(invalid("a decorator decorates a def or class"));
                }
                self.within(row.declaration, Some(row.decorator), "a decorator")?;
            }
        } else if relation == ImportAliasObservation::NAME {
            for row in ImportAliasObservation::decode(batch)? {
                if !kind(&self.get(row.statement)?, &[StmtImport, StmtImportFrom])
                    || !kind(&self.get(row.alias)?, &[Alias])
                {
                    return Err(invalid(
                        "an import alias is an alias of an import statement",
                    ));
                }
                if kind(&self.get(row.statement)?, &[StmtImport]) && row.level != 0 {
                    return Err(invalid("a plain import is absolute"));
                }
                self.within(row.statement, Some(row.alias), "an import alias")?;
            }
        } else if relation == DunderAllObservation::NAME {
            for row in DunderAllObservation::decode(batch)? {
                if !kind(
                    &self.get(row.statement)?,
                    &[StmtAssign, StmtAugAssign, StmtAnnAssign, StmtExpr],
                ) {
                    return Err(invalid("__all__ is stated by a statement"));
                }
            }
        } else if relation == ParameterSyntaxObservation::NAME {
            for row in ParameterSyntaxObservation::decode(batch)? {
                if !kind(&self.get(row.function)?, &[StmtFunctionDef, ExprLambda])
                    || !kind(
                        &self.get(row.parameter)?,
                        &[Parameter, ParameterWithDefault],
                    )
                {
                    return Err(invalid("a parameter belongs to a def or lambda"));
                }
                for (part, what) in [
                    (Some(row.parameter), "a parameter"),
                    (row.default, "a default"),
                    (row.annotation, "an annotation"),
                ] {
                    self.within(row.function, part, what)?;
                }
            }
        } else if relation == ClassFieldSyntaxObservation::NAME {
            for row in ClassFieldSyntaxObservation::decode(batch)? {
                if !kind(&self.get(row.class)?, &[StmtClassDef]) {
                    return Err(invalid("a class field belongs to a class"));
                }
                for (part, what) in [
                    (Some(row.target), "a field target"),
                    (row.annotation, "a field annotation"),
                    (row.value, "a field value"),
                ] {
                    self.within(row.class, part, what)?;
                }
            }
        } else {
            return Err(invalid("undeclared syntax geometry input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

/// A boundary sits under its provider's non-complete coverage of its scope, and its subject lies in
/// that scope. An attachment outcome is disclosed by the matching reason, in its boundary's scope,
/// and keeps its candidates: one for an innermost container, at least two when ambiguous.
fn boundary_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "subject_boundaries",
        inputs: vec![
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<Module>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<CoverageScope>(&["id"]),
            ValidationInput::of::<ProviderCoverage>(&["id"]),
            ValidationInput::of::<SubjectBoundary>(&["id"]),
            ValidationInput::of::<AttachmentOutcome>(&["id"]),
            ValidationInput::of::<AttachmentCandidate>(&["outcome"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(BoundaryCheck {
                charge: StateCharge::new(budget, "subject_boundaries"),
                ..Default::default()
            })
        }),
    }]
}
/// A scope's reach: an artifact, a module's source, an input, or a release (checked no further).
#[derive(Debug, Clone, Copy)]
enum Reach {
    Artifact(Id<SourceArtifact>),
    Input(Id<input::InputRevision>),
    Release,
}
impl HeapSize for Reach {}
#[derive(Default)]
struct BoundaryCheck {
    charge: StateCharge,
    artifacts: ChargedMap<Id<SourceArtifact>, Id<input::InputRevision>>,
    modules: ChargedMap<Id<Module>, Id<SourceArtifact>>,
    occurrences: ChargedMap<Id<Occurrence>, Id<SourceArtifact>>,
    scopes: ChargedMap<Id<CoverageScope>, Reach>,
    /// Scopes a provider covers without completeness, per context and family.
    incomplete: IncompleteScopes,
    boundaries: ChargedMap<Id<SubjectBoundary>, (Id<CoverageScope>, i16)>,
    outcomes: ChargedMap<Id<AttachmentOutcome>, (Id<SourceArtifact>, i16)>,
    candidates: ChargedMap<Id<AttachmentOutcome>, u64>,
}
impl BoundaryCheck {
    fn reaches(
        &self,
        scope: Id<CoverageScope>,
        source: Id<SourceArtifact>,
    ) -> Result<bool, ModelError> {
        Ok(
            match self
                .scopes
                .get(&scope)
                .ok_or_else(|| invalid("a boundary's scope is absent"))?
            {
                Reach::Artifact(artifact) => *artifact == source,
                Reach::Input(input) => self.artifacts.get(&source) == Some(input),
                Reach::Release => true,
            },
        )
    }
}
impl InvariantCheck for BoundaryCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                self.artifacts
                    .insert(&mut self.charge, row.id(), row.input)?;
            }
        } else if relation == Module::NAME {
            for row in Module::decode(batch)? {
                self.modules
                    .insert(&mut self.charge, row.id(), row.source)?;
            }
        } else if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences
                    .insert(&mut self.charge, row.id(), row.source)?;
            }
        } else if relation == CoverageScope::NAME {
            for row in CoverageScope::decode(batch)? {
                let reach = match row {
                    CoverageScope::Artifact { artifact } => Reach::Artifact(artifact),
                    CoverageScope::Module { module } => Reach::Artifact(
                        *self
                            .modules
                            .get(&module)
                            .ok_or_else(|| invalid("a module scope's module is absent"))?,
                    ),
                    CoverageScope::Input { input } => Reach::Input(input),
                    CoverageScope::Release { .. } => Reach::Release,
                };
                self.scopes.insert(&mut self.charge, row.id(), reach)?;
            }
        } else if relation == ProviderCoverage::NAME {
            for row in ProviderCoverage::decode(batch)? {
                if matches!(
                    row.status,
                    CoverageStatus::Partial | CoverageStatus::Unavailable | CoverageStatus::Failed
                ) {
                    self.incomplete.insert(
                        &mut self.charge,
                        (
                            row.scope,
                            row.provider
                                .ok_or_else(|| invalid("attempted coverage needs its provider"))?,
                            row.context,
                            row.family.code(),
                        ),
                    )?;
                }
            }
        } else if relation == SubjectBoundary::NAME {
            for row in SubjectBoundary::decode(batch)? {
                if !self.incomplete.contains(&(
                    row.scope,
                    row.provider,
                    row.context,
                    row.family.code(),
                )) {
                    return Err(invalid(
                        "a boundary needs its provider's partial, unavailable or failed coverage of its scope and family",
                    ));
                }
                if let Some(subject) = row.subject {
                    let source = *self
                        .occurrences
                        .get(&subject)
                        .ok_or_else(|| invalid("a boundary's subject is absent"))?;
                    if !self.reaches(row.scope, source)? {
                        return Err(invalid("a boundary's subject lies outside its scope"));
                    }
                }
                self.boundaries.insert(
                    &mut self.charge,
                    row.id(),
                    (row.scope, row.reason.code()),
                )?;
            }
        } else if relation == AttachmentOutcome::NAME {
            for row in AttachmentOutcome::decode(batch)? {
                let (scope, reason) = *self
                    .boundaries
                    .get(&row.boundary)
                    .ok_or_else(|| invalid("an attachment outcome's boundary is absent"))?;
                if reason != row.outcome.reason().code() {
                    return Err(invalid(
                        "an attachment outcome is disclosed by its matching reason",
                    ));
                }
                if !self.reaches(scope, row.source)? {
                    return Err(invalid(
                        "an attachment outcome lies outside its boundary's scope",
                    ));
                }
                self.outcomes.insert(
                    &mut self.charge,
                    row.id(),
                    (row.source, row.outcome.code()),
                )?;
            }
        } else if relation == AttachmentCandidate::NAME {
            for row in AttachmentCandidate::decode(batch)? {
                let (source, _) = *self
                    .outcomes
                    .get(&row.outcome)
                    .ok_or_else(|| invalid("a candidate's outcome is absent"))?;
                if self.occurrences.get(&row.candidate) != Some(&source) {
                    return Err(invalid("a candidate lies in its event's source"));
                }
                let count = self.candidates.get(&row.outcome).copied().unwrap_or(0);
                self.candidates
                    .insert(&mut self.charge, row.outcome, count + 1)?;
            }
        } else {
            return Err(invalid("undeclared boundary input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for (outcome, (_, kind)) in self.outcomes.iter() {
            let count = self.candidates.get(outcome).copied().unwrap_or(0);
            let admitted = match AttachmentKind::from_code(*kind) {
                Some(AttachmentKind::Innermost) => count == 1,
                Some(AttachmentKind::Ambiguous) => count >= 2,
                Some(AttachmentKind::Unmatched) => count == 0,
                Some(AttachmentKind::BudgetExceeded) => true,
                None => false,
            };
            if !admitted {
                return Err(invalid(
                    "an attachment outcome keeps exactly its candidates: one innermost, two or more ambiguous, none unmatched",
                ));
            }
        }
        Ok(())
    }
}

type IncompleteScopes = ChargedSet<(Id<CoverageScope>, Id<Provider>, Id<AnalysisContext>, i16)>;
