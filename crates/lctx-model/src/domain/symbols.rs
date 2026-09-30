//! Provider-attributed facts about native symbols (cutover plan A6):
//! - that a provider defines a symbol, and the class or function it nests in;
//! - a function's and a class's native traits;
//! - a class's declared bases and its method resolution order;
//! - a signature parameter's annotation as the provider displays it;
//! - a module's public names and where each one traces;
//! - docstring parameter documentation;
//! - how a provider resolved a module it did not analyze.
//!
//! Symbols are the provider's own (`ProviderSymbol`): every symbol a record names shares the
//! provider and context of the record's subject, and nothing here equates symbols across
//! providers. Qualified names, signature counts and a distribution's version are derived from these
//! records and the acquisition records; they are never restated.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{
    assertion::{AssertionQualification, Evidence, EvidenceSourceSpanId},
    attribution::{FactFamily, Fidelity},
    calls::{ProviderModule, ProviderSymbol, Signature, SignatureParameter, SymbolKind},
    source::{CoverageScope, Module, Occurrence, SyntaxKind},
    *,
};
use crate::{Assertion, Domain, DomainCode, DomainSum};

fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
/// The deepest symbol nesting accepted; a parent chain longer than this is refused as a cycle.
pub const MAX_SYMBOL_NESTING: usize = 256;
/// The most symbols one sequence holds.
pub const MAX_SEQUENCE_SYMBOLS: usize = 4096;

/// An ordered sequence of one provider's symbols (a class's bases or its MRO), identified by its
/// content; the empty sequence is a positive, complete statement.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "symbol_sequences", invariants = sequence_invariants)]
pub struct SymbolSequence {
    #[model(key)]
    pub members: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "symbol_sequence_members", validate = validate_member)]
pub struct SymbolSequenceMember {
    #[model(key)]
    pub sequence: Id<SymbolSequence>,
    #[model(key)]
    pub ordinal: i64,
    pub symbol: Id<ProviderSymbol>,
}
fn validate_member(row: &SymbolSequenceMember) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("negative symbol-sequence ordinal"));
    }
    Ok(())
}
fn sequence_digest(symbols: &[Id<ProviderSymbol>]) -> ContentHash {
    let mut sink = KeySink::new("symbol-sequence");
    for (ordinal, symbol) in symbols.iter().enumerate() {
        (ordinal as i64).encode(&mut sink);
        symbol.encode(&mut sink);
    }
    (symbols.len() as i64).encode(&mut sink);
    sink.finish()
}
impl SymbolSequence {
    pub fn new(
        symbols: &[Id<ProviderSymbol>],
    ) -> Result<(Self, Vec<SymbolSequenceMember>), ModelError> {
        if symbols.len() > MAX_SEQUENCE_SYMBOLS {
            return Err(invalid("symbol-sequence work limit"));
        }
        let row = Self {
            members: sequence_digest(symbols),
        };
        let members = symbols
            .iter()
            .enumerate()
            .map(|(ordinal, symbol)| SymbolSequenceMember {
                sequence: row.id(),
                ordinal: ordinal as i64,
                symbol: *symbol,
            })
            .collect();
        Ok((row, members))
    }
    /// The identity of the empty sequence.
    pub fn empty() -> Id<SymbolSequence> {
        Self {
            members: sequence_digest(&[]),
        }
        .id()
    }
}

/// A provider asserts that it defines `symbol`: at its module's top level (`parent` none), or
/// directly inside the class or function `parent` names.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "symbol_observations", invariants = symbol_invariants)]
#[assertion(support = SymbolSupport, name = "symbol_supports", family = FactFamily::Signatures, subjects(symbol))]
pub struct SymbolObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    pub parent: Option<Id<ProviderSymbol>>,
}
/// A provider's native traits of a function or method it defines. `defining_class` is the class
/// whose body defines a method; `overrides` is the base-class method it directly overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FunctionOrigin {
    DefStatement = 0,
    Synthesized = 1,
    CallableField = 2,
    Unavailable = 3,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "function_trait_observations", validate = validate_function_traits)]
#[assertion(support = FunctionTraitSupport, name = "function_trait_supports", family = FactFamily::Signatures, subjects(symbol))]
pub struct FunctionTraitObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    pub overload: bool,
    pub staticmethod: bool,
    pub classmethod: bool,
    pub property_getter: bool,
    pub property_setter: bool,
    /// The provider's stub: a `...` body, or a trivial one (`pass` or a docstring) where an
    /// implementation may be missing (a protocol, `TYPE_CHECKING`, an abstract method, an overload).
    pub stub: bool,
    /// Native origin preserves synthesized methods separately from callable-valued source fields.
    pub origin: FunctionOrigin,
    pub defining_class: Option<Id<ProviderSymbol>>,
    pub overrides: Option<Id<ProviderSymbol>>,
}
fn validate_function_traits(row: &FunctionTraitObservation) -> Result<(), ModelError> {
    if row.staticmethod && row.classmethod {
        return Err(invalid(
            "a function is not both a static and a class method",
        ));
    }
    if row.property_getter && row.property_setter {
        return Err(invalid(
            "a function is not both a property getter and setter",
        ));
    }
    if row.defining_class.is_none()
        && (row.staticmethod
            || row.classmethod
            || row.property_getter
            || row.property_setter
            || row.overrides.is_some())
    {
        return Err(invalid("method traits need a defining class"));
    }
    if row.overrides == Some(row.symbol) {
        return Err(invalid("a method does not override itself"));
    }
    Ok(())
}
/// A provider's native traits of a class it defines.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "class_trait_observations")]
#[assertion(support = ClassTraitSupport, name = "class_trait_supports", family = FactFamily::Signatures, subjects(symbol))]
pub struct ClassTraitObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    /// Synthesized (a functional `namedtuple`), not a `class` statement.
    pub synthesized: bool,
    pub dataclass: bool,
    pub named_tuple: bool,
    pub typed_dict: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum AncestryRelation {
    Bases = 0,
    Mro = 1,
}
/// How a reported MRO stands: the provider's complete C3 linearization, a recovery prefix of it,
/// or none at all because the hierarchy is cyclic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum Linearization {
    Complete = 0,
    Prefix = 1,
    Cyclic = 2,
}
/// A class's declared bases, or its MRO as the provider reports it: ancestors exclude the class
/// itself and `object`. A cyclic MRO states the empty sequence; only a complete one supports a
/// negative hierarchy decision.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "class_ancestry_observations", validate = validate_ancestry)]
#[assertion(support = ClassAncestrySupport, name = "class_ancestry_supports", family = FactFamily::Signatures, subjects(class))]
pub struct ClassAncestryObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub class: Id<ProviderSymbol>,
    #[model(key)]
    pub relation: AncestryRelation,
    pub ancestors: Id<SymbolSequence>,
    /// Present exactly for an MRO.
    pub linearization: Option<Linearization>,
}
fn validate_ancestry(row: &ClassAncestryObservation) -> Result<(), ModelError> {
    match (row.relation, row.linearization) {
        (AncestryRelation::Bases, None)
        | (AncestryRelation::Mro, Some(Linearization::Complete | Linearization::Prefix)) => Ok(()),
        (AncestryRelation::Mro, Some(Linearization::Cyclic))
            if row.ancestors == SymbolSequence::empty() =>
        {
            Ok(())
        }
        (AncestryRelation::Mro, Some(Linearization::Cyclic)) => {
            Err(invalid("a cyclic MRO states no ancestors"))
        }
        _ => Err(invalid(
            "a linearization belongs to an MRO, and every MRO has one",
        )),
    }
}
/// A signature parameter's annotation as the provider displays it. The display is a rendering:
/// every support is display-only, and no structure is established from it (structural types are
/// type observations).
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "parameter_annotation_observations", validate = validate_annotation)]
#[assertion(support = ParameterAnnotationSupport, name = "parameter_annotation_supports", family = FactFamily::Signatures, fidelity = Fidelity::DisplayOnly, subjects(scope))]
pub struct ParameterAnnotationObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub parameter: Id<SignatureParameter>,
    pub display: String,
}
fn validate_annotation(row: &ParameterAnnotationObservation) -> Result<(), ModelError> {
    if row.display.is_empty() {
        return Err(invalid("an annotation display is not empty"));
    }
    Ok(())
}
/// Pyrefly's kind for the definition an export traces to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ExportKind {
    Module = 0,
    Attribute = 1,
    Variable = 2,
    Constant = 3,
    Parameter = 4,
    TypeParameter = 5,
    TypeAlias = 6,
    Function = 7,
    Method = 8,
    Class = 9,
}
/// Where a public name is defined: the module and name the provider traced it to (with the kind
/// that module records, if any), or untraced when the trace fails. Untraced never equals a trace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "export_origins", validate = validate_origin)]
pub enum ExportOrigin {
    #[model(code = 0)]
    Traced {
        module: Id<ProviderModule>,
        name: String,
        kind: Option<ExportKind>,
    },
    #[model(code = 1)]
    Untraced,
}
fn validate_origin(row: &ExportOrigin) -> Result<(), ModelError> {
    if matches!(row, ExportOrigin::Traced { name, .. } if name.is_empty()) {
        return Err(invalid("a traced export names its origin"));
    }
    Ok(())
}
/// A provider asserts that `name` is public in module `access`, listed by `__all__` or by its
/// definition of public, and defined where `origin` says. One public name has one origin. The
/// claim is about the access module; the origin is a reference, which may lie in another module.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "public_name_observations", validate = validate_public_name)]
#[assertion(support = PublicNameSupport, name = "public_name_supports", family = FactFamily::Exports, subjects(access), referents(origin))]
pub struct PublicNameObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub access: Id<Module>,
    #[model(key)]
    pub name: String,
    pub via_dunder_all: bool,
    pub origin: Id<ExportOrigin>,
}
fn validate_public_name(row: &PublicNameObservation) -> Result<(), ModelError> {
    if row.name.is_empty() {
        return Err(invalid("a public name is not empty"));
    }
    Ok(())
}
/// A parameter a function's docstring documents: the name as written, the provider's normalized
/// text, and the description's verbatim bytes inside the `def`. The name need not be a parameter.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "parameter_doc_observations", validate = validate_doc)]
#[assertion(support = ParameterDocSupport, name = "parameter_doc_supports", family = FactFamily::Signatures, subjects(declaration, description))]
pub struct ParameterDocObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub declaration: Id<Occurrence>,
    #[model(key)]
    pub name: String,
    pub text: String,
    pub description: EvidenceSourceSpanId,
}
fn validate_doc(row: &ParameterDocObservation) -> Result<(), ModelError> {
    if row.name.is_empty() {
        return Err(invalid("a documented parameter has a name"));
    }
    Ok(())
}
/// How a provider resolved a module it references but does not analyze. A bundled stub's
/// location is its path within the bundle and a namespace package's its directory within its
/// search root; an acquired module is located by its artifact and an unresolved one nowhere.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "dependency_module_observations")]
#[assertion(support = DependencyModuleSupport, name = "dependency_module_supports", family = FactFamily::Signatures, subjects(module))]
pub struct DependencyModuleObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub module: Id<ProviderModule>,
    pub location: Option<String>,
}

fn sequence_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "symbol_sequence_membership",
        inputs: vec![
            ValidationInput::of::<SymbolSequence>(&["id"]),
            ValidationInput::of::<SymbolSequenceMember>(&["sequence", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(SequenceCheck {
                charge: StateCharge::new(budget, "symbol_sequence_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct SequenceCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<SymbolSequence>, ContentHash>,
    current: Option<(Id<SymbolSequence>, Vec<Id<ProviderSymbol>>)>,
}
impl SequenceCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, members)) = self.current.take()
            && self.expected.remove(&mut self.charge, &id) != Some(sequence_digest(&members))
        {
            return Err(invalid("symbol sequence membership differs"));
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
        if relation == SymbolSequence::NAME {
            for row in SymbolSequence::decode(batch)? {
                self.expected
                    .insert(&mut self.charge, row.id(), row.members)?;
            }
        } else if relation == SymbolSequenceMember::NAME {
            for row in SymbolSequenceMember::decode(batch)? {
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(id, _)| *id != row.sequence)
                {
                    self.flush()?;
                    self.current = Some((row.sequence, Vec::new()));
                }
                let (_, members) = self.current.as_mut().expect("current sequence");
                if row.ordinal != members.len() as i64 || members.len() >= MAX_SEQUENCE_SYMBOLS {
                    return Err(invalid("symbol sequence gaps, duplicates or work limit"));
                }
                members.push(row.symbol);
            }
        } else {
            return Err(invalid("undeclared symbol sequence input"));
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
            return Err(invalid("symbol sequence has missing members"));
        }
        Ok(())
    }
}

fn symbol_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "symbol_structure",
        inputs: vec![
            ValidationInput::of::<ProviderModule>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<Evidence>(&["id"]),
            ValidationInput::of::<Signature>(&["id"]),
            ValidationInput::of::<SignatureParameter>(&["id"]),
            ValidationInput::of::<SymbolSequenceMember>(&["sequence", "ordinal"]),
            ValidationInput::of::<SymbolObservation>(&["id"]),
            ValidationInput::of::<FunctionTraitObservation>(&["id"]),
            ValidationInput::of::<ClassTraitObservation>(&["id"]),
            ValidationInput::of::<ClassAncestryObservation>(&["id"]),
            ValidationInput::of::<ParameterAnnotationObservation>(&["id"]),
            ValidationInput::of::<ParameterDocObservation>(&["id"]),
            ValidationInput::of::<DependencyModuleObservation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(SymbolCheck {
                charge: StateCharge::new(budget, "symbol_structure"),
                ..Default::default()
            })
        }),
    }]
}
/// Every record's symbols belong to its subject's provider, context and (for nesting) module; a
/// trait or ancestry names an observed symbol of the right kind; a parent chain ends at a module's
/// top level; a documented description lies inside its `def`.
#[derive(Default)]
struct SymbolCheck {
    charge: StateCharge,
    modules: ChargedMap<Id<ProviderModule>, ProviderModule>,
    symbols: ChargedMap<Id<ProviderSymbol>, ProviderSymbol>,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    spans: ChargedMap<Id<Evidence>, (Id<source::SourceArtifact>, i64, i64)>,
    signatures: ChargedMap<Id<Signature>, Id<AssertionQualification>>,
    parameters: ChargedMap<Id<SignatureParameter>, Id<Signature>>,
    sequences: ChargedMap<Id<SymbolSequence>, Vec<Id<ProviderSymbol>>>,
    /// Observed symbols and their parents, per qualification.
    observed: ObservedSymbolParents,
    /// Symbols whose parent chain is verified, per qualification.
    rooted: ChargedSet<(Id<AssertionQualification>, Id<ProviderSymbol>)>,
}
impl SymbolCheck {
    fn symbol(&self, id: Id<ProviderSymbol>) -> Result<&ProviderSymbol, ModelError> {
        self.symbols
            .get(&id)
            .ok_or_else(|| invalid("provider symbol absent"))
    }
    /// `other` belongs to the provider and context of `owner`, and has one of `kinds`.
    fn related(
        &self,
        owner: Id<ProviderSymbol>,
        other: Id<ProviderSymbol>,
        kinds: &[SymbolKind],
        what: &str,
    ) -> Result<(), ModelError> {
        let (owner, other) = (self.symbol(owner)?, self.symbol(other)?);
        if (owner.provider, owner.context) != (other.provider, other.context) {
            return Err(invalid(&format!(
                "{what} belongs to another provider or context"
            )));
        }
        if !kinds.contains(&other.kind) {
            return Err(invalid(&format!("{what} has another kind")));
        }
        Ok(())
    }
    fn kind(
        &self,
        symbol: Id<ProviderSymbol>,
        kinds: &[SymbolKind],
        what: &str,
    ) -> Result<(), ModelError> {
        if !kinds.contains(&self.symbol(symbol)?.kind) {
            return Err(invalid(&format!("{what} has another kind")));
        }
        Ok(())
    }
    fn parent(
        &self,
        qualification: Id<AssertionQualification>,
        symbol: Id<ProviderSymbol>,
    ) -> Result<Option<Id<ProviderSymbol>>, ModelError> {
        self.observed
            .get(&(qualification, symbol))
            .copied()
            .ok_or_else(|| invalid("a symbol's traits need its observation"))
    }
    /// Walk the parent chain to the top level, once per symbol and qualification.
    fn root(
        &mut self,
        qualification: Id<AssertionQualification>,
        symbol: Id<ProviderSymbol>,
    ) -> Result<(), ModelError> {
        let mut chain = Vec::new();
        let mut current = symbol;
        while !self.rooted.contains(&(qualification, current)) {
            if chain.len() >= MAX_SYMBOL_NESTING {
                return Err(invalid("symbol nesting is cyclic or beyond its limit"));
            }
            chain.push(current);
            let Some(parent) = self
                .observed
                .get(&(qualification, current))
                .copied()
                .ok_or_else(|| invalid("a symbol's parent is not observed"))?
            else {
                break;
            };
            let (child, of) = (self.symbol(current)?, self.symbol(parent)?);
            if (child.provider, child.context, child.module) != (of.provider, of.context, of.module)
            {
                return Err(invalid("a symbol nests in another module"));
            }
            if !matches!(
                of.kind,
                SymbolKind::Class | SymbolKind::Function | SymbolKind::Method
            ) {
                return Err(invalid("a symbol nests only in a class or function"));
            }
            current = parent;
        }
        for symbol in chain {
            self.rooted
                .insert(&mut self.charge, (qualification, symbol))?;
        }
        Ok(())
    }
}
impl InvariantCheck for SymbolCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == ProviderModule::NAME {
            for row in ProviderModule::decode(batch)? {
                self.modules.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Evidence::NAME {
            for row in Evidence::decode(batch)? {
                if let Evidence::SourceSpan { source, start, end } = row {
                    self.spans
                        .insert(&mut self.charge, row.id(), (source, start, end))?;
                }
            }
        } else if relation == Signature::NAME {
            for row in Signature::decode(batch)? {
                self.signatures
                    .insert(&mut self.charge, row.id(), row.qualification)?;
            }
        } else if relation == SignatureParameter::NAME {
            for row in SignatureParameter::decode(batch)? {
                self.parameters
                    .insert(&mut self.charge, row.id(), row.signature)?;
            }
        } else if relation == SymbolSequenceMember::NAME {
            for row in SymbolSequenceMember::decode(batch)? {
                self.sequences
                    .update(&mut self.charge, row.sequence, |members| {
                        members.push(row.symbol)
                    })?;
            }
        } else if relation == SymbolObservation::NAME {
            let rows = SymbolObservation::decode(batch)?;
            for row in &rows {
                if let Some(parent) = row.parent {
                    self.related(
                        row.symbol,
                        parent,
                        &[SymbolKind::Class, SymbolKind::Function, SymbolKind::Method],
                        "a symbol's parent",
                    )?;
                }
                self.observed.insert(
                    &mut self.charge,
                    (row.qualification, row.symbol),
                    row.parent,
                )?;
            }
        } else if relation == FunctionTraitObservation::NAME {
            for row in FunctionTraitObservation::decode(batch)? {
                let method = row.defining_class.is_some();
                self.kind(
                    row.symbol,
                    if method {
                        &[SymbolKind::Method]
                    } else {
                        &[SymbolKind::Function]
                    },
                    "a function with its traits (a method exactly when it has a defining class)",
                )?;
                let parent = self.parent(row.qualification, row.symbol)?;
                if let Some(class) = row.defining_class {
                    self.related(row.symbol, class, &[SymbolKind::Class], "a defining class")?;
                    if parent != Some(class) {
                        return Err(invalid(
                            "a method's defining class is the class it nests in",
                        ));
                    }
                }
                if let Some(base) = row.overrides {
                    self.related(
                        row.symbol,
                        base,
                        &[SymbolKind::Method],
                        "an overridden method",
                    )?;
                }
            }
        } else if relation == ClassTraitObservation::NAME {
            for row in ClassTraitObservation::decode(batch)? {
                self.kind(
                    row.symbol,
                    &[SymbolKind::Class],
                    "a class with class traits",
                )?;
                self.parent(row.qualification, row.symbol)?;
            }
        } else if relation == ClassAncestryObservation::NAME {
            for row in ClassAncestryObservation::decode(batch)? {
                self.kind(row.class, &[SymbolKind::Class], "a class with ancestry")?;
                self.parent(row.qualification, row.class)?;
                let ancestors = self
                    .sequences
                    .get(&row.ancestors)
                    .cloned()
                    .unwrap_or_default();
                let mut seen = std::collections::BTreeSet::new();
                for ancestor in &ancestors {
                    if *ancestor == row.class {
                        return Err(invalid("a class is not its own ancestor"));
                    }
                    self.related(row.class, *ancestor, &[SymbolKind::Class], "an ancestor")?;
                    if row.relation == AncestryRelation::Mro && !seen.insert(*ancestor) {
                        return Err(invalid("an MRO lists each ancestor once"));
                    }
                }
            }
        } else if relation == ParameterAnnotationObservation::NAME {
            for row in ParameterAnnotationObservation::decode(batch)? {
                let signature = self
                    .parameters
                    .get(&row.parameter)
                    .ok_or_else(|| invalid("annotated parameter absent"))?;
                if self.signatures.get(signature) != Some(&row.qualification) {
                    return Err(invalid("an annotation is stated with its signature"));
                }
            }
        } else if relation == ParameterDocObservation::NAME {
            for row in ParameterDocObservation::decode(batch)? {
                let declaration = self
                    .occurrences
                    .get(&row.declaration)
                    .ok_or_else(|| invalid("documented declaration absent"))?;
                if declaration.syntax_kind != SyntaxKind::StmtFunctionDef {
                    return Err(invalid("parameter documentation belongs to a def"));
                }
                let (source, start, end) = *self
                    .spans
                    .get(&row.description.id())
                    .ok_or_else(|| invalid("parameter description span absent"))?;
                if source != declaration.source
                    || start < declaration.start
                    || end > declaration.end
                    || end <= start
                {
                    return Err(invalid("a parameter description lies inside its def"));
                }
            }
        } else if relation == DependencyModuleObservation::NAME {
            for row in DependencyModuleObservation::decode(batch)? {
                let located = match self
                    .modules
                    .get(&row.module)
                    .ok_or_else(|| invalid("resolved module absent"))?
                {
                    ProviderModule::Bundled { .. } | ProviderModule::Namespace { .. } => true,
                    ProviderModule::Acquired { .. } | ProviderModule::Unresolved { .. } => false,
                };
                match &row.location {
                    Some(path)
                        if located
                            && !path.is_empty()
                            && !path.starts_with('/')
                            && !path.split('/').any(|part| part == "..") => {}
                    None if !located => {}
                    _ => {
                        return Err(invalid(
                            "a bundled or namespace module is located by a relative path, and only such a module",
                        ));
                    }
                }
            }
        } else {
            return Err(invalid("undeclared symbol structure input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        let observed: Vec<_> = self.observed.keys().copied().collect();
        for (qualification, symbol) in observed {
            self.root(qualification, symbol)?;
        }
        Ok(())
    }
}

type ObservedSymbolParents =
    ChargedMap<(Id<AssertionQualification>, Id<ProviderSymbol>), Option<Id<ProviderSymbol>>>;
