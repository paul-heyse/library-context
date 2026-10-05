//! Provider-qualified symbols, complete signature alternatives, and one call-shape binder.
//! No normalized/effective entity is presumed at the raw-facts boundary.
use super::charged::{ChargedMap, StateCharge};
use super::{assertion::*, attribution::*, source::*, *};
use crate::{Assertion, Domain, DomainCode, DomainSum};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod signature_enumeration;
pub use signature_enumeration::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
/// Codes 6–8 are reserved legacy allocations. Current implicit callables use ProviderCallable.
pub enum SymbolKind {
    Function = 0,
    Method = 1,
    Class = 2,
    Variable = 3,
    Module = 4,
    Unknown = 5,
    ModuleBody = 6,
    ClassBody = 7,
    DecoratorApplication = 8,
}
/// The stub bundles a provider ships: the standard library's typeshed, typeshed's third-party
/// stubs, and the provider's own third-party stubs. One name in two bundles is two modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ModuleBundle {
    Typeshed = 0,
    TypeshedThirdParty = 1,
    ThirdParty = 2,
}
/// The module a provider places a symbol or type variable in. An acquired module is the typed
/// module over captured bytes. A provider-bundled stub is scoped to its provider and bundle. A
/// namespace package (directories without an `__init__`) has no bytes and is scoped to its
/// provider and context, as is an unresolved module, which keeps the provider's spelling and never
/// equals a resolved one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "provider_modules", validate = validate_provider_module, invariant_refs = provider_module_invariants_refs)]
pub enum ProviderModule {
    #[model(code = 0)]
    Acquired { module: Id<Module> },
    #[model(code = 1)]
    Bundled {
        provider: Id<Provider>,
        bundle: ModuleBundle,
        name: String,
    },
    #[model(code = 2)]
    Unresolved {
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
        name: String,
    },
    #[model(code = 3)]
    Namespace {
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
        name: String,
    },
}
fn validate_provider_module(row: &ProviderModule) -> Result<(), ModelError> {
    match row {
        ProviderModule::Bundled { name, .. }
        | ProviderModule::Unresolved { name, .. }
        | ProviderModule::Namespace { name, .. }
            if name.is_empty() =>
        {
            Err(invalid("provider module needs a name"))
        }
        _ => Ok(()),
    }
}
/// A provider's native symbol key, qualified by its pinned provider and analysis context.
/// Cross-provider equivalence is a later attributed relationship, never a spelling join.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "provider_symbols", validate = validate_symbol, invariant_refs = native_support_invariants_refs)]
pub struct ProviderSymbol {
    #[model(key, provenance)]
    pub provider: Id<Provider>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub module: Id<ProviderModule>,
    #[model(key)]
    pub native_key: String,
    pub name: String,
    pub kind: SymbolKind,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_symbol(row: &ProviderSymbol) -> Result<(), ModelError> {
    if row.native_key.is_empty() {
        return Err(invalid("provider symbol needs a native key"));
    }
    if matches!(
        row.kind,
        SymbolKind::ModuleBody | SymbolKind::ClassBody | SymbolKind::DecoratorApplication
    ) {
        return Err(invalid("implicit callables are not provider symbols"));
    }
    Ok(())
}
/// Native graph ownership, distinct from the semantic occurrence owner. A module is provider
/// neutral, so its body must explicitly retain the analyzer and invocation context.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "provider_callables")]
pub enum ProviderCallable {
    #[model(code = 0)]
    Symbol { symbol: Id<ProviderSymbol> },
    #[model(code = 1)]
    ModuleBody {
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
        module: Id<ProviderModule>,
    },
    #[model(code = 2)]
    ClassBody { class: Id<ProviderSymbol> },
    #[model(code = 3)]
    DecoratorApplication { function: Id<ProviderSymbol> },
}
impl ProviderCallable {
    /// Resolve the owner's module while checking the native symbol subtype. Shared support
    /// authorization separately proves provider/context and acquisition ownership.
    pub(crate) fn module(
        &self,
        symbol: impl Fn(Id<ProviderSymbol>) -> Result<ProviderSymbol, ModelError>,
    ) -> Result<Id<ProviderModule>, ModelError> {
        let (id, class) = match self {
            Self::ModuleBody { module, .. } => return Ok(*module),
            Self::Symbol { symbol } | Self::DecoratorApplication { function: symbol } => {
                (*symbol, false)
            }
            Self::ClassBody { class } => (*class, true),
        };
        let symbol = symbol(id)?;
        if if class {
            symbol.kind != SymbolKind::Class
        } else {
            !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method)
        } {
            return Err(invalid(
                "a call-site caller is a callable of the declared native kind",
            ));
        }
        Ok(symbol.module)
    }
}
pub(crate) fn provider_module_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "provider_module_owners",
        inputs: vec![
            ValidationInput::of::<ProviderModule>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<super::types::TypeVariable>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(ModuleOwners {
                charge: StateCharge::new(budget, "provider_module_owners"),
                ..Default::default()
            })
        }),
    }]
}
/// A bundled module belongs to one provider and an unresolved one to one provider and context.
#[derive(Default)]
struct ModuleOwners {
    charge: StateCharge,
    modules: ChargedMap<Id<ProviderModule>, ProviderModule>,
}
impl ModuleOwners {
    fn owns(
        &self,
        module: Id<ProviderModule>,
        provider: Id<Provider>,
        context: Id<AnalysisContext>,
    ) -> Result<(), ModelError> {
        match self
            .modules
            .get(&module)
            .ok_or_else(|| invalid("provider module absent"))?
        {
            ProviderModule::Acquired { .. } => Ok(()),
            ProviderModule::Bundled {
                provider: owner, ..
            } if *owner == provider => Ok(()),
            ProviderModule::Unresolved {
                provider: owner,
                context: scope,
                ..
            }
            | ProviderModule::Namespace {
                provider: owner,
                context: scope,
                ..
            } if (*owner, *scope) == (provider, context) => Ok(()),
            _ => Err(invalid(
                "provider module belongs to another provider or context",
            )),
        }
    }
}
impl InvariantCheck for ModuleOwners {
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
                self.owns(row.module, row.provider, row.context)?;
            }
        } else if relation == super::types::TypeVariable::NAME {
            for row in super::types::TypeVariable::decode(batch)? {
                self.owns(row.module, row.provider, row.context)?;
            }
        } else {
            return Err(invalid("undeclared provider module input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ParameterKind {
    PositionalOnly = 0,
    PositionalOrKeyword = 1,
    VarPositional = 2,
    KeywordOnly = 3,
    VarKeyword = 4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SignatureForm {
    List = 0,
    Ellipsis = 1,
    ParamSpec = 2,
    /// Ordered native slots whose shape cannot be interpreted as a bindable declaration.
    NativeUnavailable = 3,
}
/// A parameter's structural shape. Its position/owner is SignatureParameter, not this value.
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "parameter_shapes", validate = validate_parameter)]
pub struct ParameterShape {
    #[model(key)]
    pub name: Option<super::Utf8Text>,
    #[model(key)]
    pub kind: ParameterKind,
    #[model(key)]
    pub required: bool,
}
fn validate_parameter(row: &ParameterShape) -> Result<(), ModelError> {
    if (row.name.is_none()
        && matches!(
            row.kind,
            ParameterKind::PositionalOrKeyword | ParameterKind::KeywordOnly
        ))
        || (row.required
            && matches!(
                row.kind,
                ParameterKind::VarPositional | ParameterKind::VarKeyword
            ))
    {
        return Err(invalid("invalid parameter shape"));
    }
    Ok(())
}
/// Native typing roles never grant runtime body or call-transfer authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, DomainCode)]
#[repr(i16)]
pub enum SignatureRole {
    Source = 0,
    EffectiveTyped = 1,
    Synthesized = 2,
    Stub = 3,
    Specialized = 4,
}
impl SignatureRole {
    pub fn runtime_source(self) -> bool {
        matches!(self, Self::Source | Self::Stub)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "signature_observations", invariant_refs = signature_invariants_refs)]
#[assertion(support = SignatureSupport, name = "signature_supports", family = FactFamily::Signatures, subjects(scope))]
pub struct Signature {
    #[model(key)]
    pub role: SignatureRole,
    #[model(key)]
    pub native: Option<Id<super::types::TypeTerm>>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub symbol: Id<ProviderSymbol>,
    #[model(key)]
    pub variant: i64,
    #[model(key)]
    pub form: SignatureForm,
    #[model(key)]
    pub parameters: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "signature_parameters", validate = validate_member)]
pub struct SignatureParameter {
    #[model(key)]
    pub signature: Id<Signature>,
    #[model(key)]
    pub ordinal: i64,
    pub shape: Id<ParameterShape>,
}
fn validate_member(row: &SignatureParameter) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("negative parameter ordinal"));
    }
    Ok(())
}
fn parameter_digest(shapes: &[Id<ParameterShape>]) -> ContentHash {
    let mut sink = KeySink::new("signature-parameters");
    for (ordinal, shape) in shapes.iter().enumerate() {
        (ordinal as i64).encode(&mut sink);
        shape.encode(&mut sink);
    }
    (shapes.len() as i64).encode(&mut sink);
    sink.finish()
}
impl Signature {
    pub fn new(
        qualification: &AssertionQualification,
        role: SignatureRole,
        native: Option<Id<super::types::TypeTerm>>,
        symbol: Id<ProviderSymbol>,
        variant: i64,
        form: SignatureForm,
        parameters: &[ParameterShape],
    ) -> Result<(Self, Vec<SignatureParameter>), ModelError> {
        validate_shapes(form, parameters)?;
        if variant < 0 {
            return Err(invalid("negative signature variant"));
        }
        let shapes: Vec<_> = parameters.iter().map(Record::id).collect();
        let row = Self {
            role,
            native,
            qualification: qualification.id(),
            scope: qualification.scope,
            symbol,
            variant,
            form,
            parameters: parameter_digest(&shapes),
        };
        let members = shapes
            .into_iter()
            .enumerate()
            .map(|(i, shape)| SignatureParameter {
                signature: row.id(),
                ordinal: i as i64,
                shape,
            })
            .collect();
        Ok((row, members))
    }
}
fn validate_shapes(form: SignatureForm, parameters: &[ParameterShape]) -> Result<(), ModelError> {
    if parameters.len() > 4096 {
        return Err(ModelError::Limit {
            owner: "signature_parameters",
            limit: "parameter count",
            observed: parameters.len(),
            bound: 4096,
        });
    }
    if !matches!(form, SignatureForm::List | SignatureForm::NativeUnavailable)
        && !parameters.is_empty()
    {
        return Err(invalid(
            "unsupported signature shape or parameter work limit",
        ));
    }
    let mut names = BTreeSet::new();
    let mut previous = 0;
    let mut optional_positional = false;
    let mut varargs = false;
    let mut kwargs = false;
    for parameter in parameters {
        parameter.validate()?;
        if form == SignatureForm::NativeUnavailable {
            continue;
        }
        if parameter.name.as_ref().is_some_and(|name| name.is_empty()) {
            return Err(invalid("empty bindable parameter name"));
        }
        if parameter
            .name
            .as_ref()
            .is_some_and(|name| !names.insert(name))
        {
            return Err(invalid("duplicate parameter name"));
        }
        // Native callable reports can mix positional kinds (legacy positional-only methods).
        // Ordinals preserve provider order; codebook numbers are not syntax grammar ranks.
        let kind = match parameter.kind {
            ParameterKind::PositionalOnly | ParameterKind::PositionalOrKeyword => 0,
            ParameterKind::VarPositional => 1,
            ParameterKind::KeywordOnly => 2,
            ParameterKind::VarKeyword => 3,
        };
        if kind < previous {
            return Err(invalid("parameter kinds out of declaration order"));
        }
        previous = kind;
        match parameter.kind {
            ParameterKind::VarPositional if std::mem::replace(&mut varargs, true) => {
                return Err(invalid("duplicate varargs"));
            }
            ParameterKind::VarKeyword if std::mem::replace(&mut kwargs, true) => {
                return Err(invalid("duplicate kwargs"));
            }
            ParameterKind::PositionalOnly | ParameterKind::PositionalOrKeyword => {
                if optional_positional && parameter.required {
                    return Err(invalid("required positional follows default"));
                }
                optional_positional |= !parameter.required;
            }
            _ => {}
        }
    }
    Ok(())
}
pub(crate) fn signature_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "complete_signature_membership",
        inputs: vec![
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<ParameterShape>(&["id"]),
            ValidationInput::of::<Signature>(&["id"]),
            ValidationInput::of::<SignatureParameter>(&["signature", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(SignatureCheck {
                charge: StateCharge::new(budget, "complete_signature_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct SignatureCheck {
    charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    symbols: ChargedMap<Id<ProviderSymbol>, ProviderSymbol>,
    shapes: ChargedMap<Id<ParameterShape>, ParameterShape>,
    signatures: ChargedMap<Id<Signature>, Signature>,
    members: ChargedMap<Id<Signature>, Vec<SignatureParameter>>,
}
impl InvariantCheck for SignatureCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications
                    .insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ParameterShape::NAME {
            for row in ParameterShape::decode(batch)? {
                self.shapes.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Signature::NAME {
            for row in Signature::decode(batch)? {
                self.signatures.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == SignatureParameter::NAME {
            for row in SignatureParameter::decode(batch)? {
                if self
                    .members
                    .get(&row.signature)
                    .is_some_and(|members| members.len() >= 4096)
                {
                    return Err(invalid("signature parameter work limit"));
                }
                self.members
                    .update(&mut self.charge, row.signature, |members| members.push(row))?;
            }
        } else {
            return Err(invalid("undeclared signature validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for row in self.signatures.values() {
            let qualification = self
                .qualifications
                .get(&row.qualification)
                .ok_or_else(|| invalid("signature qualification absent"))?;
            let symbol = self
                .symbols
                .get(&row.symbol)
                .ok_or_else(|| invalid("signature symbol absent"))?;
            if qualification.scope != row.scope
                || qualification.context != symbol.context
                || row.variant < 0
                || !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method)
            {
                return Err(invalid("signature scope/context/variant mismatch"));
            }
            let members = self
                .members
                .get(&row.id())
                .map(Vec::as_slice)
                .unwrap_or_default();
            let shapes = resolve_parameters(row, members, &self.shapes)?;
            validate_shapes(row.form, &shapes)?;
        }
        if self
            .members
            .keys()
            .any(|id| !self.signatures.contains_key(id))
        {
            return Err(invalid("signature member has absent owner"));
        }
        Ok(())
    }
}
fn resolve_parameters(
    signature: &Signature,
    members: &[SignatureParameter],
    shapes: &BTreeMap<Id<ParameterShape>, ParameterShape>,
) -> Result<Vec<ParameterShape>, ModelError> {
    if members.len() > 4096 {
        return Err(invalid("signature parameter work limit"));
    }
    for (ordinal, member) in members.iter().enumerate() {
        if member.signature != signature.id() || member.ordinal != ordinal as i64 {
            return Err(invalid(
                "signature parameters missing, repeated or from another variant",
            ));
        }
    }
    if parameter_digest(&members.iter().map(|m| m.shape).collect::<Vec<_>>())
        != signature.parameters
    {
        return Err(invalid("signature parameter set differs from declaration"));
    }
    members
        .iter()
        .map(|m| {
            let shape = shapes
                .get(&m.shape)
                .ok_or_else(|| invalid("parameter shape absent"))?;
            if shape.id() != m.shape {
                return Err(invalid("parameter shape lookup key differs from value"));
            }
            Ok(shape.clone())
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, DomainCode)]
#[repr(i16)]
pub enum CallPhase {
    Call = 0,
    New = 1,
    Init = 2,
    Decorator = 3,
    PropertyGet = 4,
    PropertySet = 5,
    Definition = 6,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "call_channels", validate = validate_channel)]
pub enum CallChannel {
    #[model(code = 0)]
    Direct,
    #[model(code = 1)]
    HigherOrder { argument_index: i64 },
}
fn validate_channel(row: &CallChannel) -> Result<(), ModelError> {
    if matches!(row,CallChannel::HigherOrder { argument_index } if *argument_index < 0) {
        return Err(invalid("negative higher-order argument index"));
    }
    Ok(())
}
/// Pysa's reasons for an unresolved call at the pinned Pyrefly, as it spells them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PysaUnresolvedReason {
    LambdaArgument = 0,
    UnexpectedPyreflyTarget = 1,
    EmptyPyreflyCallTarget = 2,
    UnknownClassField = 3,
    ClassFieldOnlyExistInObject = 4,
    UnsupportedFunctionTarget = 5,
    UnexpectedDefiningClass = 6,
    UnexpectedInitMethod = 7,
    UnexpectedNewMethod = 8,
    UnexpectedCalleeExpression = 9,
    UnresolvedMagicDunderAttr = 10,
    UnresolvedMagicDunderAttrDueToNoBase = 11,
    UnresolvedMagicDunderAttrDueToNoAttribute = 12,
    Mixed = 13,
}
/// Where a call goes: one symbol; a virtual dispatch set, never one callee; or nowhere the provider
/// resolved, with the model's reason and the provider's own. A dispatch set is the named method or
/// any override of it in a class extending the target's receiver class (decided by complete MROs);
/// until a later layer expands it, only the invocation view admits it, as its named member.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "call_destinations", validate = validate_destination)]
pub enum CallDestination {
    #[model(code = 0)]
    Resolved { symbol: Id<ProviderSymbol> },
    #[model(code = 1)]
    Unresolved {
        reason: ObligationKind,
        native: Option<PysaUnresolvedReason>,
    },
    #[model(code = 2)]
    Overrides { symbol: Id<ProviderSymbol> },
    #[model(code = 3)]
    Callable { callable: Id<ProviderCallable> },
    #[model(code = 4)]
    SyntheticFormatting,
}
impl CallDestination {
    /// The symbol a resolved destination or a dispatch set names.
    pub fn symbol(&self) -> Option<Id<ProviderSymbol>> {
        match self {
            Self::Resolved { symbol } | Self::Overrides { symbol } => Some(*symbol),
            Self::Unresolved { .. } | Self::Callable { .. } | Self::SyntheticFormatting => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "call_receivers", validate = validate_receiver)]
pub enum Receiver {
    #[model(code = 0)]
    None,
    #[model(code = 1)]
    Bound { actual: Id<Occurrence> },
    #[model(code = 2)]
    Unknown { reason: ObligationKind },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "call_target_observations", invariant_refs = target_invariants_refs)]
#[assertion(support = CallTargetSupport, name = "call_target_supports", family = FactFamily::Calls, subjects(site), referents(destination, receiver_class))]
pub struct CallTarget {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub site: Id<Occurrence>,
    /// Which call event at the site: the explicit call, or an implicit one the syntax makes.
    #[model(key)]
    pub origin: Id<CallOrigin>,
    #[model(key)]
    pub destination: Id<CallDestination>,
    #[model(key)]
    pub channel: Id<CallChannel>,
    #[model(key)]
    pub phase: CallPhase,
    #[model(key)]
    pub receiver: Id<Receiver>,
    /// An implicit `__call__` of the callee object.
    #[model(key)]
    pub implicit: bool,
    /// The receiver's class the provider resolved the callee through.
    #[model(key)]
    pub receiver_class: Option<Id<ProviderSymbol>>,
    /// The native receiver evidence `receiver` classifies: how an implicit receiver is passed,
    /// and whether the callee is a class or static method.
    #[model(key)]
    pub passing: Option<ReceiverPassing>,
    pub class_method: Option<bool>,
    pub static_method: Option<bool>,
}
/// How a provider passes an implicit receiver: not at all, the class, or the object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ReceiverPassing {
    NotPassed = 0,
    Class = 1,
    Object = 2,
}
/// One step of the desugaring that makes an occurrence call something implicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum OriginStep {
    GetAttrConstantLiteral = 0,
    Comparison = 1,
    GeneratorIter = 2,
    GeneratorNext = 3,
    WithEnter = 4,
    ForDecoratedTarget = 5,
    SubscriptGetItem = 6,
    SubscriptSetItem = 7,
    BinaryOperator = 8,
    AugmentedAssignDunderCall = 9,
    AugmentedAssignRhs = 10,
    AugmentedAssignStatement = 11,
    ForIter = 12,
    ForNext = 13,
    ForAssign = 14,
    ReprCall = 15,
    AbsCall = 16,
    IterCall = 17,
    NextCall = 18,
    StrCallToDunderMethod = 19,
    Slice = 20,
    /// The `index`th target of a chained assignment.
    ChainedAssign = 21,
    /// A format string's implicit calls: its artificial call, and the stringify of an interpolated value.
    FormatStringArtificial = 22,
    FormatStringStringify = 23,
}
/// The most steps one origin holds.
pub const MAX_ORIGIN_STEPS: usize = 64;
/// Which call event at a site: the empty sequence is the explicit call written there; otherwise
/// the ordered desugaring steps of an implicit call (`for` calls `__iter__` then `__next__` at its
/// iterable, two events). Events at one site are distinct calls, never alternatives of one call.
/// Steps run from the outermost context to the implicit operation: Pysa's `Nested { head, tail }`
/// is `tail`'s steps, then `head`'s (the order it displays). The origin is the one authority on
/// whether an event is implicit; a support's origin states only who asserts it.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_origins", invariant_refs = origin_invariants_refs)]
pub struct CallOrigin {
    #[model(key)]
    pub steps: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_origin_steps", validate = validate_origin_step)]
pub struct CallOriginStep {
    #[model(key)]
    pub origin: Id<CallOrigin>,
    #[model(key)]
    pub ordinal: i64,
    pub step: OriginStep,
    /// The chained-assignment target's index, exactly for that step.
    pub index: Option<i64>,
}
fn validate_origin_step(row: &CallOriginStep) -> Result<(), ModelError> {
    if row.ordinal < 0
        || (row.step == OriginStep::ChainedAssign) != row.index.is_some()
        || row.index.is_some_and(|i| i < 0)
    {
        return Err(invalid(
            "an origin step has a nonnegative ordinal, and an index exactly for a chained assignment",
        ));
    }
    Ok(())
}
fn origin_digest(steps: &[(OriginStep, Option<i64>)]) -> ContentHash {
    let mut sink = KeySink::new("call-origin");
    for (ordinal, (step, index)) in steps.iter().enumerate() {
        (ordinal as i64).encode(&mut sink);
        step.encode(&mut sink);
        index.encode(&mut sink);
    }
    (steps.len() as i64).encode(&mut sink);
    sink.finish()
}
impl CallOrigin {
    pub fn new(
        steps: &[(OriginStep, Option<i64>)],
    ) -> Result<(Self, Vec<CallOriginStep>), ModelError> {
        if steps.len() > MAX_ORIGIN_STEPS {
            return Err(invalid("call-origin work limit"));
        }
        let row = Self {
            steps: origin_digest(steps),
        };
        let members: Vec<_> = steps
            .iter()
            .enumerate()
            .map(|(ordinal, (step, index))| CallOriginStep {
                origin: row.id(),
                ordinal: ordinal as i64,
                step: *step,
                index: *index,
            })
            .collect();
        for member in &members {
            member.validate()?;
        }
        Ok((row, members))
    }
    /// The explicit call's origin: no steps.
    pub fn explicit() -> Id<CallOrigin> {
        Self {
            steps: origin_digest(&[]),
        }
        .id()
    }
}
pub(crate) fn origin_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "call_origin_membership",
        inputs: vec![
            ValidationInput::of::<CallOrigin>(&["id"]),
            ValidationInput::of::<CallOriginStep>(&["origin", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(OriginCheck {
                charge: StateCharge::new(budget, "call_origin_membership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct OriginCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<CallOrigin>, ContentHash>,
    current: OriginMembers,
}
impl OriginCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, steps)) = self.current.take()
            && self.expected.remove(&mut self.charge, &id) != Some(origin_digest(&steps))
        {
            return Err(invalid("call origin steps differ"));
        }
        Ok(())
    }
}
impl InvariantCheck for OriginCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == CallOrigin::NAME {
            for row in CallOrigin::decode(batch)? {
                self.expected
                    .insert(&mut self.charge, row.id(), row.steps)?;
            }
        } else if relation == CallOriginStep::NAME {
            for row in CallOriginStep::decode(batch)? {
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(id, _)| *id != row.origin)
                {
                    self.flush()?;
                    self.current = Some((row.origin, Vec::new()));
                }
                let (_, steps) = self.current.as_mut().expect("current origin");
                if row.ordinal != steps.len() as i64 || steps.len() >= MAX_ORIGIN_STEPS {
                    return Err(invalid("call origin gaps, duplicates or work limit"));
                }
                steps.push((row.step, row.index));
            }
        } else {
            return Err(invalid("undeclared call origin input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        if self
            .expected
            .values()
            .any(|digest| *digest != origin_digest(&[]))
        {
            return Err(invalid("call origin has missing steps"));
        }
        Ok(())
    }
}
/// Pysa's identifier kind for a call-graph site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PysaSiteKind {
    Regular = 0,
    ArtificialCall = 1,
    ArtificialAttributeAccess = 2,
    Identifier = 3,
    FormatStringArtificial = 4,
    FormatStringStringify = 5,
}
/// Which Pysa callee record reports a site's callees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum PysaCalleeKind {
    Call = 0,
    Identifier = 1,
    AttributeAccess = 2,
    FormatStringArtificial = 3,
    FormatStringStringify = 4,
}
/// A provider's call-graph site: one call event (site and origin), the callable whose graph
/// reports it, and the native record kinds. The caller is the provider's attribution; the owner
/// rule remains the model's only definition of a caller, and a difference stays visible.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "provider_call_sites", validate = validate_provider_site, invariant_refs = provider_site_invariants_refs)]
#[assertion(support = ProviderCallSiteSupport, name = "provider_call_site_supports", family = FactFamily::Calls, subjects(site, caller))]
pub struct ProviderCallSite {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub site: Id<Occurrence>,
    #[model(key)]
    pub origin: Id<CallOrigin>,
    #[model(key)]
    pub kind: PysaSiteKind,
    #[model(key)]
    pub caller: Id<ProviderCallable>,
    pub callee: PysaCalleeKind,
    /// For an attribute access: some execution reads a plain attribute, so its property calls are
    /// conditional.
    pub is_attribute: Option<bool>,
}
fn validate_provider_site(row: &ProviderCallSite) -> Result<(), ModelError> {
    let artificial = matches!(
        row.kind,
        PysaSiteKind::ArtificialCall
            | PysaSiteKind::ArtificialAttributeAccess
            | PysaSiteKind::FormatStringArtificial
            | PysaSiteKind::FormatStringStringify
    );
    if artificial == (row.origin == CallOrigin::explicit()) {
        return Err(invalid(
            "an artificial site has origin steps, and only such a site",
        ));
    }
    let fits = match row.kind {
        PysaSiteKind::Regular
        | PysaSiteKind::ArtificialCall
        | PysaSiteKind::ArtificialAttributeAccess => matches!(
            row.callee,
            PysaCalleeKind::Call | PysaCalleeKind::AttributeAccess
        ),
        PysaSiteKind::Identifier => row.callee == PysaCalleeKind::Identifier,
        PysaSiteKind::FormatStringArtificial => {
            row.callee == PysaCalleeKind::FormatStringArtificial
        }
        PysaSiteKind::FormatStringStringify => row.callee == PysaCalleeKind::FormatStringStringify,
    };
    if !fits {
        return Err(invalid(
            "a site's callee record does not fit its identifier kind",
        ));
    }
    if (row.callee == PysaCalleeKind::AttributeAccess) != row.is_attribute.is_some() {
        return Err(invalid(
            "only an attribute access states whether it reads an attribute",
        ));
    }
    Ok(())
}
pub(crate) fn provider_site_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "provider_call_site_callers",
        inputs: vec![
            ValidationInput::of::<Module>(&["id"]),
            ValidationInput::of::<ProviderModule>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<ProviderCallable>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<CallOriginStep>(&["origin", "ordinal"]),
            ValidationInput::of::<ProviderCallSite>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(CallerCheck {
                charge: StateCharge::new(budget, "provider_call_site_callers"),
                ..Default::default()
            })
        }),
    }]
}
/// A site's caller is a callable of the site's own module, and a format-string site's origin is
/// exactly its format-string step.
#[derive(Default)]
struct CallerCheck {
    charge: StateCharge,
    modules: ChargedMap<Id<Module>, Id<SourceArtifact>>,
    provider_modules: ChargedMap<Id<ProviderModule>, Option<Id<Module>>>,
    symbols: ChargedMap<Id<ProviderSymbol>, ProviderSymbol>,
    callables: ChargedMap<Id<ProviderCallable>, ProviderCallable>,
    occurrences: ChargedMap<Id<Occurrence>, Id<SourceArtifact>>,
    steps: ChargedMap<Id<CallOrigin>, Vec<OriginStep>>,
}
impl InvariantCheck for CallerCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Module::NAME {
            for row in Module::decode(batch)? {
                self.modules
                    .insert(&mut self.charge, row.id(), row.source)?;
            }
        } else if relation == ProviderModule::NAME {
            for row in ProviderModule::decode(batch)? {
                let module = match &row {
                    ProviderModule::Acquired { module } => Some(*module),
                    _ => None,
                };
                self.provider_modules
                    .insert(&mut self.charge, row.id(), module)?;
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ProviderCallable::NAME {
            for row in ProviderCallable::decode(batch)? {
                self.callables.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences
                    .insert(&mut self.charge, row.id(), row.source)?;
            }
        } else if relation == CallOriginStep::NAME {
            for row in CallOriginStep::decode(batch)? {
                self.steps
                    .update(&mut self.charge, row.origin, |steps| steps.push(row.step))?;
            }
        } else if relation == ProviderCallSite::NAME {
            for row in ProviderCallSite::decode(batch)? {
                let steps = self
                    .steps
                    .get(&row.origin)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let format = steps.iter().any(|s| {
                    matches!(
                        s,
                        OriginStep::FormatStringArtificial | OriginStep::FormatStringStringify
                    )
                });
                let fits = match row.kind {
                    PysaSiteKind::FormatStringArtificial => {
                        steps == [OriginStep::FormatStringArtificial]
                    }
                    PysaSiteKind::FormatStringStringify => {
                        steps == [OriginStep::FormatStringStringify]
                    }
                    PysaSiteKind::Regular
                    | PysaSiteKind::Identifier
                    | PysaSiteKind::ArtificialCall
                    | PysaSiteKind::ArtificialAttributeAccess => !format,
                };
                if !fits {
                    return Err(invalid(
                        "a format-string site's origin is exactly its format-string step",
                    ));
                }
                let callable = self
                    .callables
                    .get(&row.caller)
                    .ok_or_else(|| invalid("call-site caller absent"))?;
                let module = callable.module(|id| {
                    self.symbols
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| invalid("caller symbol absent"))
                })?;
                let source = self
                    .provider_modules
                    .get(&module)
                    .copied()
                    .flatten()
                    .and_then(|module| self.modules.get(&module).copied());
                if source.is_none() || source != self.occurrences.get(&row.site).copied() {
                    return Err(invalid(
                        "a call-site caller is defined in the site's module",
                    ));
                }
            }
        } else {
            return Err(invalid("undeclared call-site caller input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ArgumentKind {
    Positional = 0,
    Starred = 1,
    Keyword = 2,
    DoubleStarred = 3,
    Implicit = 4,
}
/// A producer's view of one argument before it becomes a stored `CallArgument`.
#[derive(Debug, Clone)]
pub struct Actual {
    pub occurrence: Id<Occurrence>,
    pub kind: ArgumentKind,
    pub keyword: Option<String>,
}

/// A provider's syntax for one call site: its callee expression and complete ordered argument list.
/// Arguments are relationship rows; the digest fixes their membership and order.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "call_syntax", invariant_refs = call_syntax_invariants_refs)]
#[assertion(support = CallSyntaxSupport, name = "call_syntax_supports", family = FactFamily::Syntax, subjects(site, callee))]
pub struct CallSyntax {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub site: Id<Occurrence>,
    #[model(key)]
    pub callee: Id<Occurrence>,
    #[model(key)]
    pub arguments: ContentHash,
    #[model(key)]
    pub in_annotation: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "call_arguments", validate = validate_argument)]
pub struct CallArgument {
    #[model(key)]
    pub call: Id<CallSyntax>,
    #[model(key)]
    pub ordinal: i64,
    pub kind: ArgumentKind,
    pub keyword: Option<String>,
    pub value: Id<Occurrence>,
}
fn validate_argument(row: &CallArgument) -> Result<(), ModelError> {
    if row.ordinal < 0
        || (row.kind == ArgumentKind::Keyword) != row.keyword.is_some()
        || row.keyword.as_deref() == Some("")
    {
        return Err(invalid(
            "call argument needs a nonnegative ordinal and a keyword exactly when keyword-passed",
        ));
    }
    Ok(())
}
#[derive(Default)]
struct ArgumentDigest {
    sink: Option<KeySink>,
    count: i64,
}
impl ArgumentDigest {
    fn push(&mut self, kind: ArgumentKind, keyword: &Option<String>, value: Id<Occurrence>) {
        let sink = self
            .sink
            .get_or_insert_with(|| KeySink::new("call-arguments"));
        self.count.encode(sink);
        kind.encode(sink);
        keyword.encode(sink);
        value.encode(sink);
        self.count += 1;
    }
    fn finish(self) -> ContentHash {
        let mut sink = self.sink.unwrap_or_else(|| KeySink::new("call-arguments"));
        self.count.encode(&mut sink);
        sink.finish()
    }
}
impl CallSyntax {
    pub fn new(
        qualification: Id<AssertionQualification>,
        site: Id<Occurrence>,
        callee: Id<Occurrence>,
        in_annotation: bool,
        actuals: &[Actual],
    ) -> Result<(Self, Vec<CallArgument>), ModelError> {
        let mut digest = ArgumentDigest::default();
        for actual in actuals {
            digest.push(actual.kind, &actual.keyword, actual.occurrence);
        }
        let call = Self {
            qualification,
            site,
            callee,
            arguments: digest.finish(),
            in_annotation,
        };
        let arguments = actuals
            .iter()
            .enumerate()
            .map(|(ordinal, actual)| CallArgument {
                call: call.id(),
                ordinal: ordinal as i64,
                kind: actual.kind,
                keyword: actual.keyword.clone(),
                value: actual.occurrence,
            })
            .collect::<Vec<_>>();
        for argument in &arguments {
            argument.validate()?;
        }
        Ok((call, arguments))
    }
    /// The complete ordered arguments of this call, or a refusal if any is missing or foreign.
    pub fn actuals(&self, arguments: &[CallArgument]) -> Result<Vec<Actual>, ModelError> {
        let mut digest = ArgumentDigest::default();
        for (ordinal, argument) in arguments.iter().enumerate() {
            if argument.call != self.id() || argument.ordinal != ordinal as i64 {
                return Err(invalid("call arguments are foreign or out of order"));
            }
            digest.push(argument.kind, &argument.keyword, argument.value);
        }
        if digest.finish() != self.arguments {
            return Err(invalid(
                "call arguments differ from the call's declared membership",
            ));
        }
        Ok(arguments
            .iter()
            .map(|a| Actual {
                occurrence: a.value,
                kind: a.kind,
                keyword: a.keyword.clone(),
            })
            .collect())
    }
}
pub(crate) fn call_syntax_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "call_syntax_membership",
        inputs: vec![
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<CallSyntax>(&["id"]),
            ValidationInput::of::<CallArgument>(&["call", "ordinal"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(CallSyntaxCheck {
                charge: StateCharge::new(budget, "call_syntax_membership"),
                ..Default::default()
            })
        }),
    }]
}
type Span = (Id<SourceArtifact>, i64, i64, bool);
#[derive(Default)]
struct CallSyntaxCheck {
    charge: StateCharge,
    occurrences: ChargedMap<Id<Occurrence>, Span>,
    expected: ChargedMap<Id<CallSyntax>, (ContentHash, Id<Occurrence>)>,
    current: Option<(Id<CallSyntax>, ArgumentDigest)>,
}
impl CallSyntaxCheck {
    fn span(&self, id: Id<Occurrence>) -> Result<Span, ModelError> {
        self.occurrences
            .get(&id)
            .copied()
            .ok_or_else(|| invalid("call syntax occurrence absent"))
    }
    fn inside(&self, child: Id<Occurrence>, site: Id<Occurrence>) -> Result<(), ModelError> {
        let ((source, start, end, _), (site_source, site_start, site_end, _)) =
            (self.span(child)?, self.span(site)?);
        if source != site_source || start < site_start || end > site_end {
            return Err(invalid("call syntax part lies outside its call site"));
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((call, digest)) = self.current.take() {
            let (expected, _) = self
                .expected
                .remove(&mut self.charge, &call)
                .ok_or_else(|| invalid("call arguments name an absent or repeated call"))?;
            if digest.finish() != expected {
                return Err(invalid(
                    "call arguments differ from the call's declared membership",
                ));
            }
        }
        Ok(())
    }
}
impl InvariantCheck for CallSyntaxCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                let call =
                    row.syntax_kind == SyntaxKind::ExprCall || row.role == OccurrenceRole::Call;
                self.occurrences.insert(
                    &mut self.charge,
                    row.id(),
                    (row.source, row.start, row.end, call),
                )?;
            }
        } else if relation == CallSyntax::NAME {
            for row in CallSyntax::decode(batch)? {
                if !self.span(row.site)?.3 {
                    return Err(invalid("call syntax site is not a call occurrence"));
                }
                self.inside(row.callee, row.site)?;
                self.expected
                    .insert(&mut self.charge, row.id(), (row.arguments, row.site))?;
            }
        } else if relation == CallArgument::NAME {
            for row in CallArgument::decode(batch)? {
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(call, _)| *call != row.call)
                {
                    self.flush()?;
                    self.current = Some((row.call, ArgumentDigest::default()));
                }
                let site = self
                    .expected
                    .get(&row.call)
                    .ok_or_else(|| invalid("call argument names an absent call"))?
                    .1;
                self.inside(row.value, site)?;
                let (_, digest) = self.current.as_mut().expect("current call");
                if row.ordinal != digest.count {
                    return Err(invalid("call arguments have gaps or repeats"));
                }
                digest.push(row.kind, &row.keyword, row.value);
            }
        } else {
            return Err(invalid("undeclared call syntax input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        let empty = ArgumentDigest::default().finish();
        if self.expected.values().any(|(digest, _)| *digest != empty) {
            return Err(invalid("call syntax is missing arguments"));
        }
        Ok(())
    }
}
/// Call-shape binding does not evaluate defaults or certify normal completion. A Default source
/// names the definition-time slot; value transfer needs its separately proven value/certificate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "binding_sources")]
pub enum BindingSource {
    #[model(code = 0)]
    Actual { occurrence: Id<Occurrence> },
    #[model(code = 1)]
    Default,
    #[model(code = 2)]
    EmptyVarargs,
    #[model(code = 3)]
    EmptyKwargs,
    /// The runtime class operation applied to the exact receiver expression.
    #[model(code = 4)]
    ClassOf { actual: Id<Occurrence> },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BindingKind {
    Positional = 0,
    Keyword = 1,
    Default = 2,
    Varargs = 3,
    Kwargs = 4,
    Receiver = 5,
    Implicit = 6,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "binding_projections")]
pub enum BindingProjection {
    #[model(code = 0)]
    Whole,
    /// Element position inside the collected *args tuple, including any inserted receiver.
    #[model(code = 1)]
    Positional { index: i64 },
    /// Dictionary key inside the collected **kwargs mapping.
    #[model(code = 2)]
    Keyword { name: Utf8Text },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub formal: Id<SignatureParameter>,
    pub source: BindingSource,
    pub kind: BindingKind,
    pub projection: BindingProjection,
}
#[derive(Debug, Clone)]
pub struct BoundCall {
    site: Id<Occurrence>,
    target: Id<CallTarget>,
    signature: Id<Signature>,
    bindings: Vec<Binding>,
}
impl HeapSize for BoundCall {
    fn heap_bytes(&self) -> usize {
        self.bindings
            .capacity()
            .saturating_mul(size_of::<Binding>())
            .saturating_add(
                self.bindings
                    .iter()
                    .map(|b| b.source.heap_bytes() + b.projection.heap_bytes())
                    .sum::<usize>(),
            )
    }
}
impl BoundCall {
    pub fn site(&self) -> Id<Occurrence> {
        self.site
    }
    pub fn target(&self) -> Id<CallTarget> {
        self.target
    }
    pub fn signature(&self) -> Id<Signature> {
        self.signature
    }
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
}
/// All joins and all parameters of one signature are required. A caller cannot pass an arbitrary
/// subset of successfully bound rows and ask it to certify the whole invocation.
pub struct BindingInput<'a> {
    pub application: &'a super::normalized::signature_applicability::ApplicableSignature<'a>,
    pub parameters: &'a [SignatureParameter],
    pub shapes: &'a BTreeMap<Id<ParameterShape>, ParameterShape>,
    /// The call's syntax and its complete ordered argument set; a partial set refuses.
    pub arguments: &'a [CallArgument],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BindingFailureClass {
    ProvenIncompatible = 0,
    Undetermined = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingFailure {
    pub reason: ObligationKind,
    pub class: BindingFailureClass,
}
impl BindingFailure {
    fn incompatible() -> Self {
        Self {
            reason: ObligationKind::AmbiguousBinding,
            class: BindingFailureClass::ProvenIncompatible,
        }
    }
}
impl From<ObligationKind> for BindingFailure {
    fn from(reason: ObligationKind) -> Self {
        Self {
            reason,
            class: BindingFailureClass::Undetermined,
        }
    }
}
/// The argument algorithm consumes a validated normalized application, never raw symbol equality.
/// An unsupported or missing premise is not proof that Python rejects the invocation.
pub fn bind(input: BindingInput<'_>) -> Result<BoundCall, BindingFailure> {
    let BindingInput {
        application,
        parameters,
        shapes,
        arguments,
    } = input;
    let raw = application.raw();
    let (target, signature, channel, receiver, call) = (
        raw.target,
        raw.signature,
        raw.channel,
        raw.receiver,
        raw.call,
    );
    if !matches!(channel, CallChannel::Direct) {
        return Err(ObligationKind::CallTransfer.into());
    }
    if !matches!(
        target.phase,
        CallPhase::Call | CallPhase::Init | CallPhase::New
    ) {
        return Err(ObligationKind::OutsideProviderModel.into());
    }
    if let Receiver::Unknown { reason } = receiver
        && raw.class_of.is_none()
    {
        return Err((*reason).into());
    }
    let receiver_source = raw
        .class_of
        .map(|actual| BindingSource::ClassOf { actual })
        .or(if let Receiver::Bound { actual } = receiver {
            Some(BindingSource::Actual {
                occurrence: *actual,
            })
        } else {
            None
        });
    let assignments = assign_arguments(
        signature,
        parameters,
        shapes,
        call,
        arguments,
        receiver_source.is_some(),
    )?;
    let bindings = assignments
        .into_iter()
        .map(|assignment| {
            Ok(Binding {
                formal: assignment.formal,
                source: match assignment.source {
                    ArgumentSource::Receiver => receiver_source
                        .clone()
                        .ok_or(ObligationKind::MissingEvidence)?,
                    ArgumentSource::Value(value) => value,
                },
                kind: assignment.kind,
                projection: assignment.projection,
            })
        })
        .collect::<Result<Vec<_>, BindingFailure>>()?;
    Ok(BoundCall {
        site: target.site,
        target: target.id(),
        signature: signature.id(),
        bindings,
    })
}

/// A structural assignment contains no runtime receiver identity or callable authority.
/// The checked caller supplies the meaning of `Receiver`; it cannot become an actual expression
/// merely because the argument algorithm assigned the first formal slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArgumentSource {
    Receiver,
    Value(BindingSource),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArgumentAssignment {
    pub formal: Id<SignatureParameter>,
    pub source: ArgumentSource,
    pub kind: BindingKind,
    pub projection: BindingProjection,
}
/// The sole argument-assignment algorithm. Exact signature/argument membership is replayed here;
/// applicability, native receiver evidence and runtime identity belong to the checked caller.
/// Callers retain the work/output allowance, just as normalized binding does.
pub(crate) fn assign_arguments(
    signature: &Signature,
    parameters: &[SignatureParameter],
    shapes: &BTreeMap<Id<ParameterShape>, ParameterShape>,
    call: &CallSyntax,
    arguments: &[CallArgument],
    receiver: bool,
) -> Result<Vec<ArgumentAssignment>, BindingFailure> {
    let actuals = &call
        .actuals(arguments)
        .map_err(|_| ObligationKind::MissingEvidence)?;
    let formals = resolve_parameters(signature, parameters, shapes)
        .map_err(|_| ObligationKind::MissingEvidence)?;
    validate_shapes(signature.form, &formals).map_err(|_| ObligationKind::AmbiguousBinding)?;
    if signature.form != SignatureForm::List {
        return Err(ObligationKind::OutsideProviderModel.into());
    }
    if actuals.len() > 128 {
        return Err(ObligationKind::InvocationArgumentLimit.into());
    }
    // Unexpanded operands cannot establish a complete binding. Keep the original call evidence;
    // refusing this value transfer is not evidence that Python would reject the invocation.
    if actuals
        .iter()
        .any(|a| matches!(a.kind, ArgumentKind::Starred | ArgumentKind::DoubleStarred))
    {
        return Err(ObligationKind::UnsupportedUnpacking.into());
    }
    let mut out = vec![];
    let mut bound = vec![false; formals.len()];
    let mut positional: Vec<_> = formals
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            matches!(
                f.kind,
                ParameterKind::PositionalOnly | ParameterKind::PositionalOrKeyword
            )
        })
        .map(|(i, _)| i)
        .collect();
    let mut vararg_index = 0i64;
    if receiver {
        let first = positional
            .first()
            .copied()
            .or_else(|| {
                formals
                    .first()
                    .filter(|f| f.kind == ParameterKind::VarPositional)
                    .map(|_| 0)
            })
            .ok_or_else(BindingFailure::incompatible)?;
        if first != 0 {
            return Err(BindingFailure::incompatible());
        }
        let projection = if formals[first].kind == ParameterKind::VarPositional {
            vararg_index += 1;
            BindingProjection::Positional { index: 0 }
        } else {
            BindingProjection::Whole
        };
        out.push(ArgumentAssignment {
            formal: parameters[first].id(),
            source: ArgumentSource::Receiver,
            kind: BindingKind::Receiver,
            projection,
        });
        bound[first] = true;
        if !positional.is_empty() {
            positional.remove(0);
        }
    }
    let varargs = formals
        .iter()
        .position(|f| f.kind == ParameterKind::VarPositional);
    let kwargs = formals
        .iter()
        .position(|f| f.kind == ParameterKind::VarKeyword);
    let mut next = 0;
    let mut names = BTreeSet::new();
    let mut occurrences = BTreeSet::new();
    let mut keyword_seen = false;
    for actual in actuals {
        if !occurrences.insert(actual.occurrence)
            || (matches!(actual.kind, ArgumentKind::Keyword) != actual.keyword.is_some())
        {
            return Err(ObligationKind::MissingEvidence.into());
        }
        let index = match actual.kind {
            ArgumentKind::Positional | ArgumentKind::Implicit => {
                if keyword_seen {
                    return Err(ObligationKind::MissingEvidence.into());
                }
                let found = positional.get(next).copied().or(varargs);
                next += 1;
                found
            }
            ArgumentKind::Keyword => {
                keyword_seen = true;
                let name = actual
                    .keyword
                    .as_deref()
                    .ok_or(ObligationKind::MissingEvidence)?;
                if name.is_empty() {
                    return Err(ObligationKind::MissingEvidence.into());
                }
                if !names.insert(name) {
                    return Err(BindingFailure::incompatible());
                }
                formals
                    .iter()
                    .position(|f| {
                        f.name.as_deref() == Some(name)
                            && matches!(
                                f.kind,
                                ParameterKind::PositionalOrKeyword | ParameterKind::KeywordOnly
                            )
                    })
                    .or(kwargs)
            }
            ArgumentKind::Starred | ArgumentKind::DoubleStarred => unreachable!("refused above"),
        }
        .ok_or_else(BindingFailure::incompatible)?;
        if bound[index] && Some(index) != varargs && Some(index) != kwargs {
            return Err(BindingFailure::incompatible());
        }
        bound[index] = true;
        let (kind, projection) = if Some(index) == varargs {
            let projection = BindingProjection::Positional {
                index: vararg_index,
            };
            vararg_index += 1;
            (
                if actual.kind == ArgumentKind::Implicit {
                    BindingKind::Implicit
                } else {
                    BindingKind::Varargs
                },
                projection,
            )
        } else if Some(index) == kwargs {
            (
                BindingKind::Kwargs,
                BindingProjection::Keyword {
                    name: actual
                        .keyword
                        .clone()
                        .ok_or(ObligationKind::AmbiguousBinding)?
                        .into(),
                },
            )
        } else {
            (
                match actual.kind {
                    ArgumentKind::Keyword => BindingKind::Keyword,
                    ArgumentKind::Implicit => BindingKind::Implicit,
                    _ => BindingKind::Positional,
                },
                BindingProjection::Whole,
            )
        };
        out.push(ArgumentAssignment {
            formal: parameters[index].id(),
            source: ArgumentSource::Value(BindingSource::Actual {
                occurrence: actual.occurrence,
            }),
            kind,
            projection,
        });
    }
    for (index, formal) in formals.iter().enumerate() {
        if bound[index] {
            continue;
        }
        let source = match formal.kind {
            ParameterKind::VarPositional => BindingSource::EmptyVarargs,
            ParameterKind::VarKeyword => BindingSource::EmptyKwargs,
            _ if !formal.required => BindingSource::Default,
            _ => return Err(BindingFailure::incompatible()),
        };
        let kind = match formal.kind {
            ParameterKind::VarPositional => BindingKind::Varargs,
            ParameterKind::VarKeyword => BindingKind::Kwargs,
            _ => BindingKind::Default,
        };
        out.push(ArgumentAssignment {
            formal: parameters[index].id(),
            source: ArgumentSource::Value(source),
            kind,
            projection: BindingProjection::Whole,
        });
    }
    Ok(out)
}

/// Incomplete receiver evidence is a first-class Unknown, never an implicit plain function.
#[derive(Debug, Clone, Copy)]
pub struct ReceiverEvidence {
    pub passing: Option<ReceiverPassing>,
    pub static_method: Option<bool>,
    pub class_method: Option<bool>,
    pub actual: Option<Id<Occurrence>>,
}
/// The receiver a call binds, following Pysa's implicit-receiver rule (`has_implicit_receiver` in
/// its call graph): a method called on an object, or a class method called on a class, receives the
/// expression it is called on. A class method called on an object receives the object's class,
/// which no actual denotes, so it is Unknown; so is any contradictory or missing evidence.
pub fn classify_receiver(evidence: ReceiverEvidence) -> Receiver {
    let unknown = || Receiver::Unknown {
        reason: ObligationKind::AmbiguousBinding,
    };
    let bound = || {
        evidence
            .actual
            .map(|actual| Receiver::Bound { actual })
            .unwrap_or_else(unknown)
    };
    if evidence.static_method == Some(true) && evidence.class_method == Some(true) {
        return unknown();
    }
    match evidence.passing {
        Some(ReceiverPassing::Class | ReceiverPassing::Object)
            if evidence.static_method == Some(true) =>
        {
            unknown()
        }
        Some(ReceiverPassing::Object) if evidence.class_method == Some(true) => unknown(),
        Some(ReceiverPassing::Class | ReceiverPassing::Object) => bound(),
        Some(ReceiverPassing::NotPassed) if evidence.class_method == Some(true) => unknown(),
        Some(ReceiverPassing::NotPassed) => Receiver::None,
        None if evidence.static_method == Some(true) => Receiver::None,
        None if evidence.class_method == Some(true) => bound(),
        None => unknown(),
    }
}

/// The declaration of the complete alternative set is independent of which policy consumes it.
/// Its digest includes unresolved and otherwise inadmissible alternatives. Filtering first cannot
/// manufacture uniqueness. Completeness remains an attributed provider claim.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "call_resolutions", invariant_refs = resolution_invariants_refs)]
#[assertion(support = CallResolutionSupport, name = "call_resolution_supports", family = FactFamily::Calls, subjects(site))]
pub struct CallResolution {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub site: Id<Occurrence>,
    #[model(key)]
    pub origin: Id<CallOrigin>,
    #[model(key)]
    pub channel: Id<CallChannel>,
    #[model(key)]
    pub phase: CallPhase,
    #[model(key)]
    pub alternatives: ContentHash,
    #[model(key)]
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_resolution_members")]
pub struct CallResolutionMember {
    #[model(key)]
    pub resolution: Id<CallResolution>,
    #[model(key)]
    pub target: Id<CallTarget>,
}
fn alternative_digest(alternatives: &BTreeSet<Id<CallTarget>>) -> ContentHash {
    let mut sink = KeySink::new("call-alternatives");
    for alternative in alternatives {
        alternative.encode(&mut sink);
    }
    (alternatives.len() as i64).encode(&mut sink);
    sink.finish()
}
impl CallResolution {
    pub fn new(
        qualification: &AssertionQualification,
        site: Id<Occurrence>,
        origin: Id<CallOrigin>,
        channel: Id<CallChannel>,
        phase: CallPhase,
        complete: bool,
        targets: &[CallTarget],
    ) -> Result<(Self, Vec<CallResolutionMember>), ModelError> {
        if targets.len() > 4096
            || targets.iter().any(|target| {
                target.site != site
                    || target.origin != origin
                    || target.channel != channel
                    || target.phase != phase
            })
        {
            return Err(invalid(
                "call alternatives differ in site/origin/channel/phase or exceed work limit",
            ));
        }
        let targets: BTreeSet<_> = targets.iter().map(Record::id).collect();
        let row = Self {
            qualification: qualification.id(),
            site,
            origin,
            channel,
            phase,
            complete,
            alternatives: alternative_digest(&targets),
        };
        let members = targets
            .into_iter()
            .map(|target| CallResolutionMember {
                resolution: row.id(),
                target,
            })
            .collect();
        Ok((row, members))
    }
}
pub(crate) fn resolution_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "complete_call_alternatives",
        inputs: vec![
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<CallTarget>(&["id"]),
            ValidationInput::of::<CallResolution>(&["id"]),
            ValidationInput::of::<CallResolutionMember>(&["resolution", "target"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(ResolutionCheck {
                charge: StateCharge::new(budget, "complete_call_alternatives"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct ResolutionCheck {
    charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    targets: ChargedMap<Id<CallTarget>, CallTarget>,
    resolutions: ChargedMap<Id<CallResolution>, CallResolution>,
    members: ChargedMap<Id<CallResolution>, BTreeSet<Id<CallTarget>>>,
}
impl InvariantCheck for ResolutionCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications
                    .insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == CallTarget::NAME {
            for row in CallTarget::decode(batch)? {
                self.targets.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == CallResolution::NAME {
            for row in CallResolution::decode(batch)? {
                self.resolutions.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == CallResolutionMember::NAME {
            for row in CallResolutionMember::decode(batch)? {
                if self
                    .members
                    .get(&row.resolution)
                    .is_some_and(|members| members.len() >= 4096)
                    || !self
                        .members
                        .update(&mut self.charge, row.resolution, |members| {
                            members.insert(row.target)
                        })?
                {
                    return Err(invalid("call alternatives repeated or work limit exceeded"));
                }
            }
        } else {
            return Err(invalid("undeclared call resolution validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for resolution in self.resolutions.values() {
            let empty = BTreeSet::new();
            let members = self.members.get(&resolution.id()).unwrap_or(&empty);
            if alternative_digest(members) != resolution.alternatives {
                return Err(invalid(
                    "call resolution alternatives differ from declaration",
                ));
            }
            let qualification = self
                .qualifications
                .get(&resolution.qualification)
                .ok_or_else(|| invalid("resolution qualification absent"))?;
            for id in members {
                let target = self
                    .targets
                    .get(id)
                    .ok_or_else(|| invalid("call resolution target absent"))?;
                let target_qualification = self
                    .qualifications
                    .get(&target.qualification)
                    .ok_or_else(|| invalid("target qualification absent"))?;
                if target.site != resolution.site
                    || target.origin != resolution.origin
                    || target.channel != resolution.channel
                    || target.phase != resolution.phase
                    || qualification.context != target_qualification.context
                    || qualification.scope != target_qualification.scope
                {
                    return Err(invalid(
                        "call alternative outside resolution site/origin/channel/phase/context/scope",
                    ));
                }
            }
        }
        if self
            .members
            .keys()
            .any(|id| !self.resolutions.contains_key(id))
        {
            return Err(invalid("call resolution member owner absent"));
        }
        Ok(())
    }
}

/// One provider-native callee report for a call site, before normalization.
#[derive(Debug, Clone)]
pub struct NativeCallee {
    pub phase: CallPhase,
    pub channel: CallChannel,
    pub destination: CallDestination,
    pub receiver: ReceiverEvidence,
    pub implicit: bool,
    pub modality: Modality,
    pub receiver_class: Option<Id<ProviderSymbol>>,
}
/// The typed rows one site normalizes to: one resolution per (channel, phase) with its complete
/// alternative membership.
#[derive(Debug, Default)]
pub struct NormalizedSite {
    pub qualifications: Vec<AssertionQualification>,
    pub channels: Vec<CallChannel>,
    pub destinations: Vec<CallDestination>,
    pub receivers: Vec<Receiver>,
    pub targets: Vec<CallTarget>,
    pub resolutions: Vec<CallResolution>,
    pub members: Vec<CallResolutionMember>,
}
/// Normalize one provider's callee reports for one call event (`site`, `origin`). Receivers are classified only by
/// `classify_receiver`. A (channel, phase) group is complete only when the provider says so in
/// `complete`; absence of further callees never implies completeness. An empty report, or an
/// explicit `unresolved` remainder, adds an unresolved direct call alternative.
pub fn normalize_site(
    base: &AssertionQualification,
    site: Id<Occurrence>,
    origin: Id<CallOrigin>,
    callees: &[NativeCallee],
    complete: &BTreeSet<(Id<CallChannel>, CallPhase)>,
    unresolved: Option<(ObligationKind, Option<PysaUnresolvedReason>)>,
) -> Result<NormalizedSite, ModelError> {
    let mut groups: BTreeMap<(i64, CallPhase), (CallChannel, Vec<CallTarget>)> = BTreeMap::new();
    let mut site_rows = NormalizedSite::default();
    let mut add = |channel: &CallChannel,
                   phase: CallPhase,
                   destination: CallDestination,
                   evidence: Option<(ReceiverEvidence, Option<Id<ProviderSymbol>>)>,
                   implicit: bool,
                   modality: Modality,
                   rows: &mut NormalizedSite| {
        let qualification = AssertionQualification {
            modality,
            ..base.clone()
        };
        let receiver = evidence.map_or(
            Receiver::Unknown {
                reason: ObligationKind::AmbiguousBinding,
            },
            |(evidence, _)| classify_receiver(evidence),
        );
        let target = CallTarget {
            qualification: qualification.id(),
            site,
            origin,
            destination: destination.id(),
            channel: channel.id(),
            phase,
            receiver: receiver.id(),
            implicit,
            receiver_class: evidence.and_then(|(_, class)| class),
            passing: evidence.and_then(|(e, _)| e.passing),
            class_method: evidence.and_then(|(e, _)| e.class_method),
            static_method: evidence.and_then(|(e, _)| e.static_method),
        };
        let order = match channel {
            CallChannel::Direct => -1,
            CallChannel::HigherOrder { argument_index } => *argument_index,
        };
        groups
            .entry((order, phase))
            .or_insert_with(|| (channel.clone(), Vec::new()))
            .1
            .push(target);
        if !rows.qualifications.contains(&qualification) {
            rows.qualifications.push(qualification);
        }
        if !rows.channels.contains(channel) {
            rows.channels.push(channel.clone());
        }
        if !rows.destinations.contains(&destination) {
            rows.destinations.push(destination);
        }
        if !rows.receivers.contains(&receiver) {
            rows.receivers.push(receiver);
        }
    };
    for callee in callees {
        add(
            &callee.channel,
            callee.phase,
            callee.destination.clone(),
            Some((callee.receiver, callee.receiver_class)),
            callee.implicit,
            callee.modality,
            &mut site_rows,
        );
    }
    if callees.is_empty() || unresolved.is_some() {
        let (reason, native) = unresolved.unwrap_or((ObligationKind::UnresolvedTarget, None));
        add(
            &CallChannel::Direct,
            CallPhase::Call,
            CallDestination::Unresolved { reason, native },
            None,
            false,
            base.modality,
            &mut site_rows,
        );
    }
    if !site_rows.qualifications.contains(base) {
        site_rows.qualifications.push(base.clone());
    }
    for (_, (channel, targets)) in groups {
        let phase = targets[0].phase;
        let has_unresolved = targets.iter().any(|t| {
            site_rows
                .destinations
                .iter()
                .any(|d| d.id() == t.destination && matches!(d, CallDestination::Unresolved { .. }))
        });
        let claimed = complete.contains(&(channel.id(), phase)) && !has_unresolved;
        let (resolution, members) =
            CallResolution::new(base, site, origin, channel.id(), phase, claimed, &targets)?;
        site_rows.targets.extend(targets);
        site_rows.resolutions.push(resolution);
        site_rows.members.extend(members);
    }
    Ok(site_rows)
}

fn validate_destination(row: &CallDestination) -> Result<(), ModelError> {
    if matches!(
        row,
        CallDestination::Unresolved {
            reason: ObligationKind::ResponseBudget | ObligationKind::NotRequested,
            ..
        }
    ) {
        return Err(invalid(
            "presentation/not-requested is not an unresolved call cause",
        ));
    }
    Ok(())
}
fn validate_receiver(row: &Receiver) -> Result<(), ModelError> {
    if matches!(
        row,
        Receiver::Unknown {
            reason: ObligationKind::ResponseBudget | ObligationKind::NotRequested
        }
    ) {
        return Err(invalid(
            "presentation/not-requested is not an unknown receiver cause",
        ));
    }
    Ok(())
}
pub(crate) fn target_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "call_target_ownership",
        inputs: vec![
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<Receiver>(&["id"]),
            ValidationInput::of::<CallDestination>(&["id"]),
            ValidationInput::of::<CallTarget>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(TargetCheck {
                charge: StateCharge::new(budget, "call_target_ownership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct TargetCheck {
    charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    symbols: ChargedMap<Id<ProviderSymbol>, ProviderSymbol>,
    occurrences: ChargedMap<Id<Occurrence>, Occurrence>,
    receivers: ChargedMap<Id<Receiver>, Receiver>,
    destinations: ChargedMap<Id<CallDestination>, CallDestination>,
}
impl InvariantCheck for TargetCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications
                    .insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Receiver::NAME {
            for row in Receiver::decode(batch)? {
                self.receivers.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == CallDestination::NAME {
            for row in CallDestination::decode(batch)? {
                self.destinations.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == CallTarget::NAME {
            for row in CallTarget::decode(batch)? {
                let qualification = self
                    .qualifications
                    .get(&row.qualification)
                    .ok_or_else(|| invalid("call qualification absent"))?;
                let destination = self
                    .destinations
                    .get(&row.destination)
                    .ok_or_else(|| invalid("call destination absent"))?;
                if let Some(symbol) = destination.symbol() {
                    let symbol = self
                        .symbols
                        .get(&symbol)
                        .ok_or_else(|| invalid("call symbol absent"))?;
                    if symbol.context != qualification.context {
                        return Err(invalid("call symbol context differs from assertion"));
                    }
                    if matches!(destination, CallDestination::Overrides { .. })
                        && (symbol.kind != SymbolKind::Method || row.receiver_class.is_none())
                    {
                        return Err(invalid(
                            "an override dispatch set names a method and its receiver class",
                        ));
                    }
                }
                if let Some(class) = row.receiver_class {
                    let class = self
                        .symbols
                        .get(&class)
                        .ok_or_else(|| invalid("receiver class absent"))?;
                    if class.kind != SymbolKind::Class || class.context != qualification.context {
                        return Err(invalid(
                            "a receiver class is a class of the assertion's context",
                        ));
                    }
                }
                let receiver = self
                    .receivers
                    .get(&row.receiver)
                    .ok_or_else(|| invalid("call receiver absent"))?;
                // The stored receiver is the one its stored native evidence classifies.
                let evidence = |actual| ReceiverEvidence {
                    passing: row.passing,
                    static_method: row.static_method,
                    class_method: row.class_method,
                    actual,
                };
                let consistent = match receiver {
                    Receiver::Bound { actual } => {
                        classify_receiver(evidence(Some(*actual))) == *receiver
                    }
                    Receiver::None => classify_receiver(evidence(None)) == Receiver::None,
                    Receiver::Unknown { .. } => {
                        matches!(classify_receiver(evidence(None)), Receiver::Unknown { .. })
                    }
                };
                if !consistent {
                    return Err(invalid(
                        "a call's receiver differs from what its native evidence classifies",
                    ));
                }
                if let Receiver::Bound { actual } = receiver {
                    let site = self
                        .occurrences
                        .get(&row.site)
                        .ok_or_else(|| invalid("call site absent"))?;
                    let actual = self
                        .occurrences
                        .get(actual)
                        .ok_or_else(|| invalid("call receiver occurrence absent"))?;
                    if site.source != actual.source
                        || site.start > actual.start
                        || site.end < actual.end
                    {
                        return Err(invalid("bound receiver lies outside call occurrence"));
                    }
                }
            }
        } else {
            return Err(invalid("undeclared call ownership input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

/// A native provider key is meaningful only in that provider's namespace. Supporting an
/// equivalent symbol from another provider requires a later explicit equivalence relationship.
pub(crate) fn native_support_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "native_symbol_support_ownership",
        inputs: vec![
            ValidationInput::of::<ProviderSymbol>(&["id"]),
            ValidationInput::of::<ProviderRun>(&["id"]),
            ValidationInput::of::<CallDestination>(&["id"]),
            ValidationInput::of::<Signature>(&["id"]),
            ValidationInput::of::<CallTarget>(&["id"]),
            ValidationInput::of::<SignatureSupport>(&["id"]),
            ValidationInput::of::<CallTargetSupport>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(NativeSupportCheck {
                charge: StateCharge::new(budget, "native_symbol_support_ownership"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct NativeSupportCheck {
    charge: StateCharge,
    symbols: ChargedMap<Id<ProviderSymbol>, Id<Provider>>,
    runs: ChargedMap<Id<ProviderRun>, Id<Provider>>,
    destinations: ChargedMap<Id<CallDestination>, CallDestination>,
    signatures: ChargedMap<Id<Signature>, Id<ProviderSymbol>>,
    targets: NativeTargetIndex,
}
impl NativeSupportCheck {
    fn check(&self, symbol: Id<ProviderSymbol>, run: Id<ProviderRun>) -> Result<(), ModelError> {
        let provider = self
            .symbols
            .get(&symbol)
            .ok_or_else(|| invalid("native symbol absent"))?;
        if self.runs.get(&run) != Some(provider) {
            return Err(invalid(
                "native symbol support belongs to a different provider",
            ));
        }
        Ok(())
    }
}
impl InvariantCheck for NativeSupportCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == ProviderSymbol::NAME {
            for row in ProviderSymbol::decode(batch)? {
                self.symbols
                    .insert(&mut self.charge, row.id(), row.provider)?;
            }
        } else if relation == ProviderRun::NAME {
            for row in ProviderRun::decode(batch)? {
                self.runs.insert(&mut self.charge, row.id(), row.provider)?;
            }
        } else if relation == CallDestination::NAME {
            for row in CallDestination::decode(batch)? {
                self.destinations.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Signature::NAME {
            for row in Signature::decode(batch)? {
                self.signatures
                    .insert(&mut self.charge, row.id(), row.symbol)?;
            }
        } else if relation == CallTarget::NAME {
            for row in CallTarget::decode(batch)? {
                self.targets.insert(
                    &mut self.charge,
                    row.id(),
                    (row.destination, row.receiver_class),
                )?;
            }
        } else if relation == SignatureSupport::NAME {
            for row in SignatureSupport::decode(batch)? {
                self.check(
                    *self
                        .signatures
                        .get(&row.assertion)
                        .ok_or_else(|| invalid("supported signature absent"))?,
                    row.run,
                )?;
            }
        } else if relation == CallTargetSupport::NAME {
            for row in CallTargetSupport::decode(batch)? {
                let (destination, receiver_class) = self
                    .targets
                    .get(&row.assertion)
                    .ok_or_else(|| invalid("supported call target absent"))?;
                if let Some(symbol) = self
                    .destinations
                    .get(destination)
                    .ok_or_else(|| invalid("supported call destination absent"))?
                    .symbol()
                {
                    self.check(symbol, row.run)?;
                }
                if let Some(class) = receiver_class {
                    self.check(*class, row.run)?;
                }
            }
        } else {
            return Err(invalid("undeclared native symbol support input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

type OriginMembers = Option<(Id<CallOrigin>, Vec<(OriginStep, Option<i64>)>)>;

type NativeTargetIndex =
    ChargedMap<Id<CallTarget>, (Id<CallDestination>, Option<Id<ProviderSymbol>>)>;

pub(crate) fn provider_module_invariants_refs() -> Vec<&'static str> {
    vec!["provider_module_owners"]
}
pub(crate) fn signature_invariants_refs() -> Vec<&'static str> {
    vec!["complete_signature_membership"]
}
pub(crate) fn origin_invariants_refs() -> Vec<&'static str> {
    vec!["call_origin_membership"]
}
pub(crate) fn provider_site_invariants_refs() -> Vec<&'static str> {
    vec!["provider_call_site_callers"]
}
pub(crate) fn call_syntax_invariants_refs() -> Vec<&'static str> {
    vec!["call_syntax_membership"]
}
pub(crate) fn resolution_invariants_refs() -> Vec<&'static str> {
    vec!["complete_call_alternatives"]
}
pub(crate) fn target_invariants_refs() -> Vec<&'static str> {
    vec!["call_target_ownership"]
}
pub(crate) fn native_support_invariants_refs() -> Vec<&'static str> {
    vec!["native_symbol_support_ownership"]
}
