//! Structural provider type observations. Renderings do not define modeled type identity.
//! Variable restrictions are qualified relationships, so recursive bounds need no recursive key.
use super::charged::{ChargedMap, ChargedSet, ChargedVec, StateCharge};
use super::{
    assertion::AssertionQualification,
    attribution::{AnalysisContext, FactFamily, Fidelity, Provider, ProviderRun},
    calls::{ProviderSymbol, SymbolKind},
    obligation::ObligationKind,
    source::{CoverageScope, Occurrence},
    value::Literal,
    *,
};
use crate::{Assertion, Domain, DomainCode, DomainSum};
pub mod locations;
mod signatures;
mod generics;
pub use generics::*;
pub use signatures::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AnyFlavor {
    Explicit = 0,
    Implicit = 1,
    Error = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum NeverFlavor {
    NoReturn = 0,
    Never = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TypeVariableKind {
    TypeVar = 0,
    ParamSpec = 1,
    TypeVarTuple = 2,
    IntVar = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TypeVariableOrigin {
    ScopedLegacy = 0,
    Pep695 = 1,
    Synthetic = 2,
    SyntheticSelf = 3,
    MapIntTuples = 4,
    NormalizedMapIntTuples = 5,
}
/// Native quantified identity is qualified by provider/context. The range is in the provider's
/// module coordinate space; it does not assert that a captured declaration was found there.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "type_variables", validate = validate_variable)]
pub struct TypeVariable {
    #[model(key, provenance)]
    pub provider: Id<Provider>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub module: Id<super::calls::ProviderModule>,
    #[model(key)]
    pub anchor_start: i64,
    #[model(key)]
    pub anchor_end: i64,
    #[model(key)]
    pub slot: i64,
    #[model(key)]
    pub origin: TypeVariableOrigin,
    #[model(key)]
    pub kind: TypeVariableKind,
    pub name: String,
    /// Native declared variance; inference is a separate optional result.
    pub declared_variance: Option<TypeVariance>,
    pub inferred_variance: Option<TypeVariance>,
}
/// The provider's supported typing special forms. Codes are append-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TypingForm {
    Annotated = 0,
    Callable = 1,
    ClassVar = 2,
    Concatenate = 3,
    Final = 4,
    Generic = 5,
    Literal = 6,
    LiteralString = 7,
    Never = 8,
    NoReturn = 9,
    NotRequired = 10,
    Optional = 11,
    Protocol = 12,
    ReadOnly = 13,
    Required = 14,
    SelfType = 15,
    Tuple = 16,
    Type = 17,
    TypeAlias = 18,
    TypeForm = 19,
    TypeGuard = 20,
    TypeIs = 21,
    TypedDict = 22,
    Union = 23,
    Unpack = 24,
    Ellipsis = 25,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_term(row: &TypeTerm) -> Result<(), ModelError> {
    let named = match row {
        TypeTerm::TypeAlias { name, .. }
        | TypeTerm::TypeAliasReference { name, .. }
        | TypeTerm::EnumLiteral { member: name, .. } => !name.is_empty(),
        _ => true,
    };
    if !named {
        return Err(invalid("a named type form is named"));
    }
    Ok(())
}
fn validate_variable(row: &TypeVariable) -> Result<(), ModelError> {
    if row.name.is_empty()
        || row.anchor_start < 0
        || row.anchor_end < row.anchor_start
        || row.slot < 0
    {
        return Err(invalid("invalid native type-variable identity"));
    }
    Ok(())
}
/// Pyrefly's structural type vocabulary. Codes retain the existing type-kind codes; forms the
/// old kinds told apart only by display text have their own appended codes. Renderings, alias
/// display names included, are presentations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "type_terms", validate = validate_term, invariants = type_invariants)]
pub enum TypeTerm {
    #[model(code = 0)]
    ClassInstance {
        class: Id<ProviderSymbol>,
        arguments: Id<TypeSequence>,
    },
    #[model(code = 1)]
    ClassObject { class: Id<ProviderSymbol> },
    #[model(code = 2)]
    TypeOf { target: Id<TypeTerm> },
    /// A `TypedDict` class instance; `partial` is its update form.
    #[model(code = 3)]
    TypedDict {
        class: Id<ProviderSymbol>,
        arguments: Id<TypeSequence>,
        partial: bool,
    },
    #[model(code = 4)]
    Union { members: Id<TypeSequence> },
    #[model(code = 5)]
    Intersection { members: Id<TypeSequence> },
    /// A callable signature; a `def`'s type names its function.
    #[model(code = 6)]
    Callable {
        function: Option<Id<ProviderSymbol>>,
        form: CallableForm,
        parameters: Id<CallableParameterList>,
        param_spec: Option<Id<TypeTerm>>,
        returns: Id<TypeTerm>,
    },
    #[model(code = 7)]
    Overload {
        function: Id<ProviderSymbol>,
        signatures: Id<TypeSequence>,
    },
    #[model(code = 8)]
    BoundMethod {
        receiver: Id<TypeTerm>,
        function: Id<TypeTerm>,
    },
    /// A callable or alias quantified over its own type parameters.
    #[model(code = 9)]
    Generic {
        parameters: Id<TypeSequence>,
        body: Id<TypeTerm>,
    },
    #[model(code = 10)]
    Tuple { elements: Id<TypeSequence> },
    #[model(code = 11)]
    Literal { value: Id<Literal> },
    #[model(code = 12)]
    TypeVar { variable: Id<TypeVariable> },
    #[model(code = 13)]
    ParamSpec { variable: Id<TypeVariable> },
    #[model(code = 14)]
    TypeVarTuple { variable: Id<TypeVariable> },
    /// A module as a value.
    #[model(code = 15)]
    Module {
        module: Id<super::calls::ProviderModule>,
    },
    #[model(code = 16)]
    Any { flavor: AnyFlavor },
    #[model(code = 17)]
    Never { flavor: NeverFlavor },
    #[model(code = 18)]
    None,
    /// A type alias with its value.
    #[model(code = 19)]
    TypeAlias {
        name: String,
        untyped: bool,
        target: Id<TypeTerm>,
    },
    #[model(code = 20)]
    SelfType {
        class: Id<ProviderSymbol>,
        arguments: Id<TypeSequence>,
    },
    #[model(code = 21)]
    Annotated { target: Id<TypeTerm> },
    #[model(code = 22)]
    Unpack { target: Id<TypeTerm> },
    #[model(code = 23)]
    TypeGuard {
        form: GuardForm,
        target: Id<TypeTerm>,
    },
    /// A parameter list standing alone: a `ParamSpec`'s value, or `Concatenate[..., P]` with its
    /// `ParamSpec`.
    #[model(code = 24)]
    ParamList {
        parameters: Id<CallableParameterList>,
        param_spec: Option<Id<TypeTerm>>,
    },
    /// A special form, or a type-variable declaration used as a value, by the name it is written.
    #[model(code = 25)]
    SpecialForm { form: TypingForm },
    #[model(code = 26)]
    Other {
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
        variant: String,
        display: String,
    },
    #[model(code = 27)]
    Truncated {
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
        reason: ObligationKind,
        display: String,
    },
    /// A `TypedDict` without a class: mapping keys have their own field vocabulary.
    #[model(code = 28)]
    AnonymousTypedDict {
        fields: Id<TypedDictFieldList>,
        partial: bool,
    },
    /// A recursive alias reference: its name and arguments, never an expansion.
    #[model(code = 29)]
    TypeAliasReference {
        module: Id<super::calls::ProviderModule>,
        name: String,
        untyped: bool,
        arguments: Id<TypeSequence>,
    },
    /// A form of a type variable: `P.args`, `P.kwargs`, their values, an element of a
    /// `TypeVarTuple`, or the variable used as a value.
    #[model(code = 30)]
    VariableForm {
        variable: Id<TypeVariable>,
        form: VariableFormKind,
    },
    /// An enum member literal: its class keeps same-named enums apart.
    #[model(code = 31)]
    EnumLiteral {
        class: Id<ProviderSymbol>,
        member: String,
    },
    #[model(code = 32)]
    LiteralString,
    #[model(code = 33)]
    TypeForm { target: Id<TypeTerm> },
    /// Native overloaded result values, retaining alternatives without a fabricated function.
    #[model(code = 34)]
    Overloaded { alternatives: Id<TypeSequence> },
}
/// A callable's parameter form: a (possibly partial) list, `...`, a materialization, or a prefix
/// followed by a `ParamSpec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CallableForm {
    List = 0,
    Partial = 1,
    Ellipsis = 2,
    Materialization = 3,
    ParamSpec = 4,
    /// Native expanded slots retained for evidence, unavailable as a callable interpretation.
    NativeUnavailable = 5,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum GuardForm {
    TypeGuard = 0,
    TypeIs = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum VariableFormKind {
    Value = 0,
    Args = 1,
    Kwargs = 2,
    ArgsValue = 3,
    KwargsValue = 4,
    Element = 5,
}
/// An ordered list of named, kinded slots with their types: a callable's parameters, a parameter
/// list, or an anonymous `TypedDict`'s fields. Identified by its content.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "callable_parameter_lists", invariants = parameter_list_invariants)]
pub struct CallableParameterList {
    #[model(key)]
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "callable_parameters", validate = validate_callable_parameter)]
pub struct CallableParameter {
    #[model(key)]
    pub list: Id<CallableParameterList>,
    #[model(key)]
    pub ordinal: i64,
    pub name: Option<super::Utf8Text>,
    pub kind: super::calls::ParameterKind,
    /// Absent exactly for a variadic slot.
    pub required: Option<bool>,
    pub term: Id<TypeTerm>,
}
fn validate_callable_parameter(row: &CallableParameter) -> Result<(), ModelError> {
    use super::calls::ParameterKind as K;
    let variadic = matches!(row.kind, K::VarPositional | K::VarKeyword);
    if row.ordinal < 0
        || variadic == row.required.is_some()
        || (row.name.is_none() && matches!(row.kind, K::PositionalOrKeyword | K::KeywordOnly))
    {
        return Err(invalid(
            "a callable parameter has a nonnegative ordinal, a name when passable by keyword, and requiredness exactly when not variadic",
        ));
    }
    Ok(())
}
/// One callable parameter before it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub name: Option<super::Utf8Text>,
    pub kind: super::calls::ParameterKind,
    pub required: Option<bool>,
    pub term: Id<TypeTerm>,
}
fn parameter_list_digest(slots: &[Slot]) -> ContentHash {
    let mut sink = KeySink::new("callable-parameters");
    for (ordinal, slot) in slots.iter().enumerate() {
        (ordinal as i64).encode(&mut sink);
        slot.name.encode(&mut sink);
        slot.kind.encode(&mut sink);
        slot.required.encode(&mut sink);
        slot.term.encode(&mut sink);
    }
    (slots.len() as i64).encode(&mut sink);
    sink.finish()
}
impl CallableParameterList {
    pub fn new(slots: &[Slot]) -> Result<(Self, Vec<CallableParameter>), ModelError> {
        if slots.len() > 4096 {
            return Err(invalid("callable-parameter work limit"));
        }
        let row = Self {
            members: parameter_list_digest(slots),
        };
        let members: Vec<_> = slots
            .iter()
            .enumerate()
            .map(|(ordinal, slot)| CallableParameter {
                list: row.id(),
                ordinal: ordinal as i64,
                name: slot.name.clone(),
                kind: slot.kind,
                required: slot.required,
                term: slot.term,
            })
            .collect();
        for member in &members {
            member.validate()?;
        }
        Ok((row, members))
    }
}
fn parameter_list_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "callable_parameter_membership",
        inputs: vec![
            ValidationInput::of::<CallableParameterList>(&["id"]),
            ValidationInput::of::<CallableParameter>(&["list", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(ParameterListCheck {
                charge: StateCharge::new(budget, "callable_parameter_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct ParameterListCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<CallableParameterList>, ContentHash>,
    current: Option<(Id<CallableParameterList>, Vec<Slot>)>,
}
impl ParameterListCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, slots)) = self.current.take()
            && self.expected.remove(&mut self.charge, &id) != Some(parameter_list_digest(&slots))
        {
            return Err(invalid("callable parameter membership differs"));
        }
        Ok(())
    }
}
impl InvariantCheck for ParameterListCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == CallableParameterList::NAME {
            for row in CallableParameterList::decode(batch)? {
                self.expected
                    .insert(&mut self.charge, row.id(), row.members)?;
            }
        } else if relation == CallableParameter::NAME {
            for row in CallableParameter::decode(batch)? {
                if self.current.as_ref().is_none_or(|(id, _)| *id != row.list) {
                    self.flush()?;
                    self.current = Some((row.list, Vec::new()));
                }
                let (_, slots) = self.current.as_mut().expect("current list");
                if row.ordinal != slots.len() as i64 || slots.len() >= 4096 {
                    return Err(invalid("callable parameter gaps, duplicates or work limit"));
                }
                slots.push(Slot {
                    name: row.name,
                    kind: row.kind,
                    required: row.required,
                    term: row.term,
                });
            }
        } else {
            return Err(invalid("undeclared callable parameter input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        if self
            .expected
            .values()
            .any(|digest| *digest != parameter_list_digest(&[]))
        {
            return Err(invalid("callable parameter list has missing members"));
        }
        Ok(())
    }
}
/// Ordered structural dictionary fields. Keys are strings, including empty/non-identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "typed_dict_field_lists", invariants = dict_field_invariants)]
pub struct TypedDictFieldList {
    #[model(key)]
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "typed_dict_fields", validate = validate_dict_field)]
pub struct TypedDictField {
    #[model(key)]
    pub list: Id<TypedDictFieldList>,
    #[model(key)]
    pub ordinal: i64,
    pub name: super::Utf8Text,
    pub required: bool,
    pub term: Id<TypeTerm>,
}
pub type TypedDictSlot = (super::Utf8Text, bool, Id<TypeTerm>);
fn validate_dict_field(row: &TypedDictField) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("negative typed dictionary field ordinal"));
    }
    Ok(())
}
fn dict_field_digest(slots: &[TypedDictSlot]) -> ContentHash {
    let mut sink = KeySink::new("typed-dictionary-fields");
    for (ordinal, (name, required, term)) in slots.iter().enumerate() {
        (ordinal as i64).encode(&mut sink);
        name.encode(&mut sink);
        required.encode(&mut sink);
        term.encode(&mut sink);
    }
    (slots.len() as i64).encode(&mut sink);
    sink.finish()
}
impl TypedDictFieldList {
    pub fn new(slots: &[TypedDictSlot]) -> Result<(Self, Vec<TypedDictField>), ModelError> {
        if slots.len() > 4096 {
            return Err(ModelError::Limit {
                owner: "typed_dictionary_fields",
                limit: "field count",
                observed: slots.len(),
                bound: 4096,
            });
        }
        let mut names = std::collections::BTreeSet::new();
        if slots.iter().any(|(name, _, _)| !names.insert(name)) {
            return Err(invalid("duplicate typed dictionary key"));
        }
        let row = Self {
            members: dict_field_digest(slots),
        };
        let members = slots
            .iter()
            .enumerate()
            .map(|(ordinal, (name, required, term))| TypedDictField {
                list: row.id(),
                ordinal: ordinal as i64,
                name: name.clone(),
                required: *required,
                term: *term,
            })
            .collect();
        Ok((row, members))
    }
}
fn dict_field_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "typed_dictionary_membership",
        inputs: vec![
            ValidationInput::of::<TypedDictFieldList>(&["id"]),
            ValidationInput::of::<TypedDictField>(&["list", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(DictFieldCheck {
                charge: StateCharge::new(budget, "typed_dictionary_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct DictFieldCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<TypedDictFieldList>, ContentHash>,
    current: Option<(Id<TypedDictFieldList>, ChargedVec<TypedDictSlot>)>,
}
impl DictFieldCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, mut slots)) = self.current.take() {
            if self.expected.remove(&mut self.charge, &id) != Some(dict_field_digest(&slots)) {
                return Err(invalid("typed dictionary membership differs"));
            }
            let _scratch = self
                .charge
                .budget()
                .expect("bound dictionary validator")
                .reserve("typed_dictionary_names", slots.len().saturating_mul(64))?;
            let mut names = std::collections::BTreeSet::new();
            if slots.iter().any(|(name, _, _)| !names.insert(name)) {
                return Err(invalid("duplicate typed dictionary key"));
            }
            drop(names);
            drop(slots.take(&mut self.charge));
        }
        Ok(())
    }
}
impl InvariantCheck for DictFieldCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == TypedDictFieldList::NAME {
            for row in TypedDictFieldList::decode(batch)? {
                self.expected
                    .insert(&mut self.charge, row.id(), row.members)?;
            }
        } else if relation == TypedDictField::NAME {
            for row in TypedDictField::decode(batch)? {
                if self.current.as_ref().is_none_or(|(id, _)| *id != row.list) {
                    self.flush()?;
                    self.current = Some((row.list, ChargedVec::default()));
                }
                let (_, slots) = self.current.as_mut().expect("current field list");
                if row.ordinal != slots.len() as i64 || slots.len() >= 4096 {
                    return Err(invalid(
                        "typed dictionary field gaps, duplicates or work limit",
                    ));
                }
                slots.push(&mut self.charge, (row.name, row.required, row.term))?;
            }
        } else {
            return Err(invalid("undeclared typed dictionary input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        if self
            .expected
            .values()
            .any(|digest| *digest != dict_field_digest(&[]))
        {
            return Err(invalid("typed dictionary list has missing fields"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TypeChildRole {
    Argument = 0,
    Member = 1,
    Element = 4,
    Variadic = 5,
    Signature = 6,
    TypeParameter = 9,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "type_sequences", invariants = sequence_invariants)]
pub struct TypeSequence {
    #[model(key)]
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "type_sequence_members", validate = validate_member)]
pub struct TypeSequenceMember {
    #[model(key)]
    pub sequence: Id<TypeSequence>,
    #[model(key)]
    pub ordinal: i64,
    pub role: TypeChildRole,
    pub child: Id<TypeTerm>,
}
fn validate_member(row: &TypeSequenceMember) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("negative type-child ordinal"));
    }
    Ok(())
}
fn sequence_digest(items: &[(TypeChildRole, Id<TypeTerm>)]) -> ContentHash {
    let mut sink = KeySink::new("type-sequence");
    for (ordinal, (role, child)) in items.iter().enumerate() {
        (ordinal as i64).encode(&mut sink);
        role.encode(&mut sink);
        child.encode(&mut sink);
    }
    (items.len() as i64).encode(&mut sink);
    sink.finish()
}
impl TypeSequence {
    pub fn new(
        items: &[(TypeChildRole, Id<TypeTerm>)],
    ) -> Result<(Self, Vec<TypeSequenceMember>), ModelError> {
        if items.len() > 4096 {
            return Err(invalid("type-sequence work limit"));
        }
        let row = Self {
            members: sequence_digest(items),
        };
        let members = items
            .iter()
            .enumerate()
            .map(|(ordinal, (role, child))| TypeSequenceMember {
                sequence: row.id(),
                ordinal: ordinal as i64,
                role: *role,
                child: *child,
            })
            .collect();
        Ok((row, members))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
/// What a type observation types. `TestOperand` is a load in a test position (a condition's
/// operand), typed by the provider that owns occurrences.
pub enum TypeRole {
    Parameter = 0,
    Return = 1,
    CallResult = 2,
    Argument = 3,
    Raised = 4,
    TestOperand = 5,
    AttributeBase = 8,
    AssignmentValue = 9,
    ReturnExpression = 10,
    Expected = 11,
    ChosenOverload = 6,
    OverloadCandidates = 7,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "type_observations", invariants = locations::invariants)]
#[assertion(support = TypeSupport, name = "type_supports", family = FactFamily::Types, subjects(subject, term))]
pub struct TypeObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub subject: Id<Occurrence>,
    #[model(key)]
    pub role: TypeRole,
    #[model(key)]
    pub declared: bool,
    #[model(key)]
    pub term: Id<TypeTerm>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "type_presentations")]
#[assertion(support = TypePresentationSupport, name = "type_presentation_supports", family = FactFamily::Types, subjects(scope, term))]
pub struct TypePresentation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub term: Id<TypeTerm>,
    #[model(key)]
    pub display: String,
    #[model(key)]
    pub detail: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TypeRestrictionKind {
    Bound = 12,
    Constraint = 13,
    Default = 14,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "type_variable_restrictions", validate = validate_restriction)]
#[assertion(support = TypeRestrictionSupport, name = "type_restriction_supports", family = FactFamily::Types, subjects(scope, variable, term))]
pub struct TypeVariableRestriction {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub variable: Id<TypeVariable>,
    #[model(key)]
    pub kind: TypeRestrictionKind,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub term: Id<TypeTerm>,
}
fn validate_restriction(row: &TypeVariableRestriction) -> Result<(), ModelError> {
    if row.ordinal < 0 || (row.kind != TypeRestrictionKind::Constraint && row.ordinal != 0) {
        return Err(invalid("invalid type restriction ordinal"));
    }
    Ok(())
}

fn sequence_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "type_sequence_membership",
        inputs: vec![
            ValidationInput::of::<TypeSequence>(&["id"]),
            ValidationInput::of::<TypeSequenceMember>(&["sequence", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(SequenceCheck {
                charge: StateCharge::new(budget, "type_sequence_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct SequenceCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<TypeSequence>, ContentHash>,
    current: SequenceMembers,
}
impl SequenceCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, members)) = self.current.take()
            && self.expected.remove(&mut self.charge, &id) != Some(sequence_digest(&members))
        {
            return Err(invalid("type sequence membership differs"));
        }
        Ok(())
    }
}
impl InvariantCheck for SequenceCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == TypeSequence::NAME {
            for row in TypeSequence::decode(batch)? {
                self.expected
                    .insert(&mut self.charge, row.id(), row.members)?;
            }
        } else if relation == TypeSequenceMember::NAME {
            for row in TypeSequenceMember::decode(batch)? {
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(id, _)| *id != row.sequence)
                {
                    self.flush()?;
                    self.current = Some((row.sequence, Vec::new()));
                }
                let (_, members) = self.current.as_mut().expect("current sequence");
                if row.ordinal != members.len() as i64 || members.len() >= 4096 {
                    return Err(invalid("type sequence gaps, duplicates or work limit"));
                }
                members.push((row.role, row.child));
            }
        } else {
            return Err(invalid("undeclared type sequence input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        if self
            .expected
            .values()
            .any(|digest| *digest != sequence_digest(&[]))
        {
            return Err(invalid("type sequence has missing members"));
        }
        Ok(())
    }
}

fn type_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "structural_type_shapes",
        inputs: TypeIndex::inputs(),
        create: std::sync::Arc::new(|budget| {
            Box::new(TypeIndex::new(budget, "structural_type_shapes"))
        }),
    }]
}
/// The same native-owner closure is used by shared support validation and structural checks.
#[derive(Default)]
pub(crate) struct TypeIndex {
    charge: StateCharge,
    modules: ChargedMap<Id<super::calls::ProviderModule>, super::calls::ProviderModule>,
    symbols: ChargedMap<Id<ProviderSymbol>, ProviderSymbol>,
    variables: ChargedMap<Id<TypeVariable>, TypeVariable>,
    terms: ChargedMap<Id<TypeTerm>, TypeTerm>,
    sequences: ChargedSet<Id<TypeSequence>>,
    members: ChargedMap<Id<TypeSequence>, Vec<TypeSequenceMember>>,
    dict_lists: ChargedSet<Id<TypedDictFieldList>>,
    dict_fields: ChargedMap<Id<TypedDictFieldList>, Vec<TypedDictField>>,
    lists: ChargedSet<Id<CallableParameterList>>,
    slots: ChargedMap<Id<CallableParameterList>, Vec<CallableParameter>>,
    /// Terms whose closure is verified for (provider, context, display-only support); a shared
    /// term is walked once per key, so total work is linear in the term graph.
    verified: VerifiedTypeOwners,
}
impl TypeIndex {
    pub fn new(budget: &super::resources::ResourceBudget, owner: &'static str) -> Self {
        Self {
            charge: StateCharge::new(budget, owner),
            ..Self::default()
        }
    }
    pub fn inputs() -> Vec<ValidationInput> {
        vec![
            ValidationInput::of::<super::calls::ProviderModule>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<TypeVariable>(&["id"]),
            ValidationInput::of::<TypeSequence>(&["id"]),
            ValidationInput::of::<TypeSequenceMember>(&["sequence", "ordinal"]),
            ValidationInput::of::<TypedDictFieldList>(&["id"]),
            ValidationInput::of::<TypedDictField>(&["list", "ordinal"]),
            ValidationInput::of::<CallableParameterList>(&["id"]),
            ValidationInput::of::<CallableParameter>(&["list", "ordinal"]),
            ValidationInput::of::<TypeTerm>(&["id"]),
        ]
    }
    pub fn visit_input(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        if relation == super::calls::ProviderModule::NAME {
            for row in super::calls::ProviderModule::decode(batch)? {
                self.modules.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == TypedDictFieldList::NAME {
            for row in TypedDictFieldList::decode(batch)? {
                self.dict_lists.insert(&mut self.charge, row.id())?;
            }
        } else if relation == TypedDictField::NAME {
            for row in TypedDictField::decode(batch)? {
                self.dict_fields
                    .update(&mut self.charge, row.list, |fields| fields.push(row))?;
            }
        } else if relation == CallableParameterList::NAME {
            for row in CallableParameterList::decode(batch)? {
                self.lists.insert(&mut self.charge, row.id())?;
            }
        } else if relation == CallableParameter::NAME {
            for row in CallableParameter::decode(batch)? {
                self.slots
                    .update(&mut self.charge, row.list, |slots| slots.push(row))?;
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == TypeVariable::NAME {
            for row in TypeVariable::decode(batch)? {
                self.variables.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == TypeSequence::NAME {
            for row in TypeSequence::decode(batch)? {
                self.sequences.insert(&mut self.charge, row.id())?;
            }
        } else if relation == TypeSequenceMember::NAME {
            for row in TypeSequenceMember::decode(batch)? {
                self.members
                    .update(&mut self.charge, row.sequence, |members| members.push(row))?;
            }
        } else if relation == TypeTerm::NAME {
            for row in TypeTerm::decode(batch)? {
                self.terms.insert(&mut self.charge, row.id(), row)?;
            }
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    fn dict(&self, id: Id<TypedDictFieldList>) -> Result<&[TypedDictField], ModelError> {
        if !self.dict_lists.contains(&id) {
            return Err(invalid("typed dictionary field list absent"));
        }
        Ok(self
            .dict_fields
            .get(&id)
            .map(Vec::as_slice)
            .unwrap_or_default())
    }
    fn list(&self, id: Id<CallableParameterList>) -> Result<&[CallableParameter], ModelError> {
        if !self.lists.contains(&id) {
            return Err(invalid("callable parameter list absent"));
        }
        Ok(self.slots.get(&id).map(Vec::as_slice).unwrap_or_default())
    }
    pub fn module(
        &self,
        id: Id<super::calls::ProviderModule>,
    ) -> Result<&super::calls::ProviderModule, ModelError> {
        self.modules
            .get(&id)
            .ok_or_else(|| invalid("provider module absent"))
    }
    /// A module a provider names is acquired, or its own (bundled), or its own in its context.
    fn module_owner(
        &self,
        id: Id<super::calls::ProviderModule>,
        run: &ProviderRun,
    ) -> Result<(), ModelError> {
        use super::calls::ProviderModule as M;
        match self.module(id)? {
            M::Acquired { .. } => Ok(()),
            M::Bundled { provider, .. } if *provider == run.provider => Ok(()),
            M::Namespace {
                provider, context, ..
            }
            | M::Unresolved {
                provider, context, ..
            } if (*provider, *context) == (run.provider, run.context) => Ok(()),
            _ => Err(invalid("type module belongs to another provider/context")),
        }
    }
    fn sequence(&self, id: Id<TypeSequence>) -> Result<&[TypeSequenceMember], ModelError> {
        if !self.sequences.contains(&id) {
            return Err(invalid("type sequence absent"));
        }
        Ok(self.members.get(&id).map(Vec::as_slice).unwrap_or_default())
    }
    pub fn variable_owner(
        &self,
        id: Id<TypeVariable>,
        run: &ProviderRun,
    ) -> Result<(), ModelError> {
        let v = self
            .variables
            .get(&id)
            .ok_or_else(|| invalid("type variable absent"))?;
        if (v.provider, v.context) != (run.provider, run.context) {
            return Err(invalid("type variable belongs to another provider/context"));
        }
        Ok(())
    }
    pub fn term_support(
        &mut self,
        id: Id<TypeTerm>,
        run: &ProviderRun,
        fidelity: Fidelity,
    ) -> Result<(), ModelError> {
        let key = |term: Id<TypeTerm>| {
            (
                term,
                run.provider,
                run.context,
                fidelity == Fidelity::DisplayOnly,
            )
        };
        if self.verified.contains(&key(id)) {
            return Ok(());
        }
        // The walk's scratch is charged to the same budget and released when the walk ends.
        let mut scratch = StateCharge::new(
            self.charge
                .budget()
                .ok_or_else(|| invalid("type closure needs a validation budget"))?,
            "type_closure",
        );
        let mut pending = ChargedVec::default();
        let mut seen = ChargedSet::default();
        pending.push(&mut scratch, id)?;
        while let Some(id) = pending.take_last(&mut scratch) {
            if self.verified.contains(&key(id)) || !seen.insert(&mut scratch, id)? {
                continue;
            }
            let mut children = Vec::new();
            let term = self
                .terms
                .get(&id)
                .ok_or_else(|| invalid("type term absent"))?;
            match term {
                TypeTerm::ClassInstance { class, arguments }
                | TypeTerm::TypedDict {
                    class, arguments, ..
                }
                | TypeTerm::SelfType { class, arguments } => {
                    self.class_owner(*class, run)?;
                    children.extend(self.sequence(*arguments)?.iter().map(|m| m.child));
                }
                TypeTerm::ClassObject { class } | TypeTerm::EnumLiteral { class, .. } => {
                    self.class_owner(*class, run)?
                }
                TypeTerm::TypeOf { target }
                | TypeTerm::TypeForm { target }
                | TypeTerm::Annotated { target }
                | TypeTerm::Unpack { target }
                | TypeTerm::TypeGuard { target, .. }
                | TypeTerm::TypeAlias { target, .. } => children.push(*target),
                TypeTerm::Overloaded { alternatives: members } | TypeTerm::Union { members } | TypeTerm::Intersection { members } => {
                    children.extend(self.sequence(*members)?.iter().map(|m| m.child))
                }
                TypeTerm::Tuple { elements } => {
                    children.extend(self.sequence(*elements)?.iter().map(|m| m.child))
                }
                TypeTerm::Overload {
                    function,
                    signatures,
                } => {
                    self.function_owner(*function, run)?;
                    children.extend(self.sequence(*signatures)?.iter().map(|m| m.child));
                }
                TypeTerm::Generic { parameters, body } => {
                    children.extend(self.sequence(*parameters)?.iter().map(|m| m.child));
                    children.push(*body);
                }
                TypeTerm::BoundMethod { receiver, function } => {
                    children.extend([*receiver, *function])
                }
                TypeTerm::Callable {
                    function,
                    parameters,
                    param_spec,
                    returns,
                    form,
                } => {
                    if *form == CallableForm::NativeUnavailable && fidelity != Fidelity::DisplayOnly
                    {
                        return Err(invalid(
                            "unavailable callable closure requires display-only support",
                        ));
                    }
                    if let Some(function) = function {
                        self.function_owner(*function, run)?;
                    }
                    children.extend(self.list(*parameters)?.iter().map(|slot| slot.term));
                    children.extend(*param_spec);
                    children.push(*returns);
                }
                TypeTerm::ParamList {
                    parameters,
                    param_spec,
                } => {
                    children.extend(self.list(*parameters)?.iter().map(|slot| slot.term));
                    children.extend(*param_spec);
                }
                TypeTerm::AnonymousTypedDict { fields, .. } => {
                    children.extend(self.dict(*fields)?.iter().map(|slot| slot.term))
                }
                TypeTerm::Module { module } => self.module_owner(*module, run)?,
                TypeTerm::TypeAliasReference {
                    module, arguments, ..
                } => {
                    self.module_owner(*module, run)?;
                    children.extend(self.sequence(*arguments)?.iter().map(|m| m.child));
                }
                TypeTerm::TypeVar { variable }
                | TypeTerm::ParamSpec { variable }
                | TypeTerm::TypeVarTuple { variable }
                | TypeTerm::VariableForm { variable, .. } => self.variable_owner(*variable, run)?,
                TypeTerm::Other {
                    provider, context, ..
                }
                | TypeTerm::Truncated {
                    provider, context, ..
                } => {
                    if (*provider, *context) != (run.provider, run.context) {
                        return Err(invalid("opaque type belongs to another provider/context"));
                    }
                    if fidelity != Fidelity::DisplayOnly {
                        return Err(invalid("opaque type closure requires display-only support"));
                    }
                }
                TypeTerm::Literal { .. }
                | TypeTerm::Any { .. }
                | TypeTerm::Never { .. }
                | TypeTerm::None
                | TypeTerm::LiteralString
                | TypeTerm::SpecialForm { .. } => {}
            }
            for child in children {
                pending.push(&mut scratch, child)?;
            }
        }
        // Every term the walk reached has its own closure inside this verified one.
        for term in seen.iter() {
            self.verified.insert(&mut self.charge, key(*term))?;
        }
        Ok(())
    }
    pub fn symbol(&self, id: Id<ProviderSymbol>) -> Result<&ProviderSymbol, ModelError> {
        self.symbols
            .get(&id)
            .ok_or_else(|| invalid("provider symbol absent"))
    }
    fn function_owner(&self, id: Id<ProviderSymbol>, run: &ProviderRun) -> Result<(), ModelError> {
        let symbol = self.symbol(id)?;
        if (symbol.provider, symbol.context) != (run.provider, run.context)
            || !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method)
        {
            return Err(invalid(
                "named callable belongs to another provider/context or is not a function",
            ));
        }
        Ok(())
    }
    fn class_owner(&self, id: Id<ProviderSymbol>, run: &ProviderRun) -> Result<(), ModelError> {
        let symbol = self
            .symbols
            .get(&id)
            .ok_or_else(|| invalid("type class symbol absent"))?;
        if (symbol.provider, symbol.context) != (run.provider, run.context) {
            return Err(invalid("type class belongs to another provider/context"));
        }
        Ok(())
    }
}
impl InvariantCheck for TypeIndex {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.visit_input(relation, batch)? {
            return Err(invalid("undeclared type shape input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for term in self.terms.values() {
            let class = match term {
                TypeTerm::ClassInstance { class, .. }
                | TypeTerm::ClassObject { class }
                | TypeTerm::TypedDict { class, .. }
                | TypeTerm::SelfType { class, .. }
                | TypeTerm::EnumLiteral { class, .. } => Some(class),
                _ => None,
            };
            if let Some(class) = class
                && self
                    .symbols
                    .get(class)
                    .is_none_or(|s| s.kind != SymbolKind::Class)
            {
                return Err(invalid("class term lacks a native class symbol"));
            }
            let function = match term {
                TypeTerm::Callable { function, .. } => *function,
                TypeTerm::Overload { function, .. } => Some(*function),
                _ => None,
            };
            if let Some(function) = function
                && self
                    .symbols
                    .get(&function)
                    .is_none_or(|s| !matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            {
                return Err(invalid("named callable lacks a native function symbol"));
            }
            let arm = |id: &Id<TypeTerm>| {
                self.terms
                    .get(id)
                    .ok_or_else(|| invalid("type term child absent"))
            };
            let roles = |sequence: &Id<TypeSequence>,
                         role: TypeChildRole|
             -> Result<&[TypeSequenceMember], ModelError> {
                let items = self.sequence(*sequence)?;
                if items.iter().any(|m| m.role != role) {
                    return Err(invalid("type sequence children have another role"));
                }
                Ok(items)
            };
            match term {
                TypeTerm::ClassInstance { arguments, .. }
                | TypeTerm::TypedDict { arguments, .. }
                | TypeTerm::SelfType { arguments, .. }
                | TypeTerm::TypeAliasReference { arguments, .. } => {
                    if self
                        .sequence(*arguments)?
                        .iter()
                        .any(|m| m.role != TypeChildRole::Argument)
                    {
                        return Err(invalid("class type children must be arguments"));
                    }
                }
                TypeTerm::Overload { signatures, .. } => {
                    let items = roles(signatures, TypeChildRole::Signature)?;
                    if items.is_empty()
                        || items.iter().try_fold(false, |bad, m| {
                            Ok::<_, ModelError>(
                                bad || !matches!(
                                    arm(&m.child)?,
                                    TypeTerm::Callable { .. }
                                        | TypeTerm::Generic { .. }
                                        | TypeTerm::Other { .. }
                                        | TypeTerm::Truncated { .. }
                                ),
                            )
                        })?
                    {
                        return Err(invalid(
                            "an overload names its function and has callable signatures",
                        ));
                    }
                }
                TypeTerm::Overloaded { alternatives } => {
                    if roles(alternatives, TypeChildRole::Member)?.is_empty() {return Err(invalid("overloaded values retain nonempty ordered type alternatives"));}
                }
                TypeTerm::BoundMethod { function, .. } => {
                    if !matches!(
                        arm(function)?,
                        TypeTerm::Callable { .. }
                            | TypeTerm::Generic { .. }
                            | TypeTerm::Overload { .. }
                            | TypeTerm::Overloaded { .. }
                            | TypeTerm::Other { .. }
                            | TypeTerm::Truncated { .. }
                    ) {
                        return Err(invalid("a bound method binds a callable"));
                    }
                }
                TypeTerm::Generic { parameters, body } => {
                    let items = roles(parameters, TypeChildRole::TypeParameter)?;
                    if items.is_empty()
                        || items.iter().try_fold(false, |bad, m| {
                            Ok::<_, ModelError>(
                                bad || !matches!(
                                    arm(&m.child)?,
                                    TypeTerm::TypeVar { .. }
                                        | TypeTerm::ParamSpec { .. }
                                        | TypeTerm::TypeVarTuple { .. }
                                        | TypeTerm::Other { .. }
                                        | TypeTerm::Truncated { .. }
                                ),
                            )
                        })?
                        || !matches!(
                            arm(body)?,
                            TypeTerm::Callable { .. }
                                | TypeTerm::TypeAlias { .. }
                                | TypeTerm::Other { .. }
                                | TypeTerm::Truncated { .. }
                        )
                    {
                        return Err(invalid(
                            "a generic binds type variables over a callable or alias",
                        ));
                    }
                }
                TypeTerm::Callable {
                    form,
                    parameters,
                    param_spec,
                    ..
                } => {
                    let slots = self.list(*parameters)?;
                    let prefix = slots.iter().all(|s| {
                        matches!(
                            s.kind,
                            super::calls::ParameterKind::PositionalOnly
                                | super::calls::ParameterKind::PositionalOrKeyword
                        )
                    });
                    let valid = match form {
                        CallableForm::List | CallableForm::Partial => {
                            param_spec.is_none()
                                && slots
                                    .iter()
                                    .all(|s| s.name.as_ref().is_none_or(|name| !name.is_empty()))
                        }
                        CallableForm::NativeUnavailable => param_spec.is_none(),
                        CallableForm::Ellipsis | CallableForm::Materialization => {
                            param_spec.is_none() && slots.is_empty()
                        }
                        CallableForm::ParamSpec => param_spec.is_some() && prefix,
                    };
                    if !valid {
                        return Err(invalid("a callable's parameters differ from its form"));
                    }
                }
                TypeTerm::ParamList {
                    parameters,
                    param_spec,
                } => {
                    let prefix = self.list(*parameters)?.iter().all(|s| {
                        matches!(
                            s.kind,
                            super::calls::ParameterKind::PositionalOnly
                                | super::calls::ParameterKind::PositionalOrKeyword
                        )
                    });
                    if param_spec.is_some() && !prefix {
                        return Err(invalid("a concatenation prefixes positional parameters"));
                    }
                }
                TypeTerm::AnonymousTypedDict { fields, .. } => {
                    self.dict(*fields)?;
                }
                TypeTerm::VariableForm { variable, form } => {
                    let v = self
                        .variables
                        .get(variable)
                        .ok_or_else(|| invalid("type variable absent"))?;
                    let valid = match form {
                        VariableFormKind::Args
                        | VariableFormKind::Kwargs
                        | VariableFormKind::ArgsValue
                        | VariableFormKind::KwargsValue => v.kind == TypeVariableKind::ParamSpec,
                        VariableFormKind::Element => v.kind == TypeVariableKind::TypeVarTuple,
                        VariableFormKind::Value => true,
                    };
                    if !valid {
                        return Err(invalid("a variable form needs its variable's kind"));
                    }
                }
                TypeTerm::Union { members } | TypeTerm::Intersection { members } => {
                    let items = self.sequence(*members)?;
                    if items.is_empty() || items.iter().any(|m| m.role != TypeChildRole::Member) {
                        return Err(invalid("union/intersection needs members"));
                    }
                }
                TypeTerm::Tuple { elements } => {
                    let items = self.sequence(*elements)?;
                    if items.iter().any(|m| {
                        !matches!(m.role, TypeChildRole::Element | TypeChildRole::Variadic)
                    }) || items
                        .iter()
                        .filter(|m| m.role == TypeChildRole::Variadic)
                        .count()
                        > 1
                    {
                        return Err(invalid(
                            "tuple needs fixed elements and at most one variadic segment",
                        ));
                    }
                }
                TypeTerm::TypeVar { variable }
                | TypeTerm::ParamSpec { variable }
                | TypeTerm::TypeVarTuple { variable } => {
                    let v = self
                        .variables
                        .get(variable)
                        .ok_or_else(|| invalid("type variable absent"))?;
                    let valid = match term {
                        TypeTerm::TypeVar { .. } => {
                            matches!(v.kind, TypeVariableKind::TypeVar | TypeVariableKind::IntVar)
                        }
                        TypeTerm::ParamSpec { .. } => v.kind == TypeVariableKind::ParamSpec,
                        TypeTerm::TypeVarTuple { .. } => v.kind == TypeVariableKind::TypeVarTuple,
                        _ => false,
                    };
                    if !valid {
                        return Err(invalid("type variable kind differs from term"));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Pyrefly's resolved classification of a function body, not a source-text guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FunctionBodyKind {
    RaiseNotImplementedError = 0,
    ReturnNotImplemented = 1,
    Ellipsis = 2,
    Trivial = 3,
    Other = 4,
}
/// What a provider resolved about a `def`'s body and the declarations around it: whether callers
/// run a real body (an abstract method, a protocol member, a stub, an overload) or its override's.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "function_body_observations", invariants = body_invariants)]
#[assertion(support = FunctionBodySupport, name = "function_body_supports", family = FactFamily::Types, subjects(declaration))]
pub struct FunctionBodyObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub declaration: Id<Occurrence>,
    pub body: FunctionBodyKind,
    pub abstract_method: bool,
    pub in_protocol_class: bool,
    pub in_type_checking_block: bool,
    pub overload: bool,
}
fn body_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "function_body_declarations",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<FunctionBodyObservation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(BodyCheck {
                charge: StateCharge::new(budget, "function_body_declarations"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct BodyCheck {
    charge: StateCharge,
    kinds: ChargedMap<Id<Occurrence>, super::source::SyntaxKind>,
}
impl InvariantCheck for BodyCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.kinds
                    .insert(&mut self.charge, row.id(), row.syntax_kind)?;
            }
        } else if relation == FunctionBodyObservation::NAME {
            for row in FunctionBodyObservation::decode(batch)? {
                if self.kinds.get(&row.declaration)
                    != Some(&super::source::SyntaxKind::StmtFunctionDef)
                {
                    return Err(invalid("a function body belongs to a def"));
                }
            }
        } else {
            return Err(invalid("undeclared function body input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}
/// Which record model a class's fields come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum RecordKind {
    Dataclass = 0,
    Attrs = 1,
    Pydantic = 2,
    TypedDict = 3,
    NamedTuple = 4,
}
/// One field of a record class in the provider's field order (inherited fields first), with its
/// type and the flags its record model defines: dataclass-like fields state a default and whether
/// `__init__` takes them (with an alias and `kw_only` when set); named-tuple fields state a default;
/// `TypedDict` fields state requiredness and read-only. The claim is about the class; an inherited
/// field's declaration is a reference inside the class that declares it, possibly another module's.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "record_field_observations", validate = validate_record_field, invariants = record_invariants)]
#[assertion(support = RecordFieldSupport, name = "record_field_supports", family = FactFamily::Types, subjects(class, term, default_term), referents(declaration))]
pub struct RecordFieldObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub class: Id<ProviderSymbol>,
    #[model(key)]
    pub name: super::Utf8Text,
    pub record: RecordKind,
    pub ordinal: i64,
    pub term: Id<TypeTerm>,
    /// The field has an explicit annotation.
    pub declared: bool,
    pub declaration: Option<Id<Occurrence>>,
    pub has_default: Option<bool>,
    /// Native effective default type; default/factory source origin remains independent.
    #[model(key)]
    pub default_term: Option<Id<TypeTerm>>,
    pub init: Option<bool>,
    pub alias: Option<String>,
    pub kw_only: Option<bool>,
    pub required: Option<bool>,
    pub read_only: Option<bool>,
}
fn validate_record_field(row: &RecordFieldObservation) -> Result<(), ModelError> {
    let dataclass_like = matches!(
        row.record,
        RecordKind::Dataclass | RecordKind::Attrs | RecordKind::Pydantic
    );
    let typed_dict = row.record == RecordKind::TypedDict;
    let valid = row.ordinal >= 0
        && (typed_dict || !row.name.is_empty())
        && row.alias.as_ref().is_none_or(|alias| !alias.is_empty())
        && row.has_default.is_some() == !typed_dict
        && (row.default_term.is_none() || (dataclass_like && row.has_default == Some(true)))
        && row.init.is_some() == dataclass_like
        && (dataclass_like || (row.alias.is_none() && row.kw_only.is_none()))
        && row.required.is_some() == typed_dict
        && row.read_only.is_some() == typed_dict;
    if !valid {
        return Err(invalid(
            "a record field states exactly the flags its record model defines",
        ));
    }
    Ok(())
}
fn record_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "record_field_order",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<RecordFieldObservation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(RecordCheck {
                charge: StateCharge::new(budget, "record_field_order"),
                ..Default::default()
            })
        }),
    }]
}
/// A class's fields under one qualification come from one record model, at ordinals 0..n; a field's
/// declaration lies inside a class statement.
#[derive(Default)]
struct RecordCheck {
    charge: StateCharge,
    classes: ChargedSet<Id<ProviderSymbol>>,
    fields: RecordFieldIndex,
    spans: ChargedMap<Id<Occurrence>, (Id<super::source::SourceArtifact>, i64, i64)>,
    class_statements: ChargedMap<Id<super::source::SourceArtifact>, Vec<(i64, i64)>>,
}
impl InvariantCheck for RecordCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.spans
                    .insert(&mut self.charge, row.id(), (row.source, row.start, row.end))?;
                if row.syntax_kind == super::source::SyntaxKind::StmtClassDef {
                    self.class_statements
                        .update(&mut self.charge, row.source, |spans| {
                            spans.push((row.start, row.end))
                        })?;
                }
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                if row.kind == SymbolKind::Class {
                    self.classes.insert(&mut self.charge, row.id())?;
                }
            }
        } else if relation == RecordFieldObservation::NAME {
            for row in RecordFieldObservation::decode(batch)? {
                if !self.classes.contains(&row.class) {
                    return Err(invalid("a record field belongs to a class"));
                }
                if let Some(declaration) = row.declaration {
                    let (source, start, end) = *self
                        .spans
                        .get(&declaration)
                        .ok_or_else(|| invalid("record field declaration absent"))?;
                    let classes = self
                        .class_statements
                        .get(&source)
                        .map(Vec::as_slice)
                        .unwrap_or_default();
                    if !classes.iter().any(|(s, e)| *s <= start && end <= *e) {
                        return Err(invalid(
                            "a record field's declaration lies inside a class statement",
                        ));
                    }
                }
                self.fields
                    .update(&mut self.charge, (row.qualification, row.class), |fields| {
                        fields.push((row.ordinal, row.record))
                    })?;
            }
        } else {
            return Err(invalid("undeclared record field input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for fields in self.fields.values() {
            let mut ordinals: Vec<i64> = fields.iter().map(|(ordinal, _)| *ordinal).collect();
            ordinals.sort_unstable();
            if ordinals
                .iter()
                .enumerate()
                .any(|(i, ordinal)| *ordinal != i as i64)
                || fields.iter().any(|(_, kind)| *kind != fields[0].1)
            {
                return Err(invalid(
                    "a class's record fields have one record model and ordinals 0..n",
                ));
            }
        }
        Ok(())
    }
}

type SequenceMembers = Option<(Id<TypeSequence>, Vec<(TypeChildRole, Id<TypeTerm>)>)>;

type VerifiedTypeOwners = ChargedSet<(Id<TypeTerm>, Id<Provider>, Id<AnalysisContext>, bool)>;

type RecordFieldIndex =
    ChargedMap<(Id<AssertionQualification>, Id<ProviderSymbol>), Vec<(i64, RecordKind)>>;

pub mod queries;
pub use queries::{TypeQueryObservation, TypeQuerySupport, TypeQueryStatus};
