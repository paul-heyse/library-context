//! Provider-qualified symbols, complete signature alternatives, and one call-shape binder.
//! No normalized/effective entity is presumed at the raw-facts boundary.
use super::charged::{ChargedMap, StateCharge};
use std::collections::{BTreeMap, BTreeSet};
use crate::{Assertion, Domain, DomainCode, DomainSum};
use super::{*, assertion::*, attribution::*, source::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SymbolKind { Function = 0, Method = 1, Class = 2, Variable = 3, Module = 4, Unknown = 5 }
/// The module a provider places a symbol or type variable in. An acquired module is the typed
/// module over captured bytes. A provider-bundled stub (such as vendored typeshed) is scoped to
/// its provider; an unresolved module keeps the provider's spelling and never equals a resolved one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "provider_modules", validate = validate_provider_module, invariants = provider_module_invariants)]
pub enum ProviderModule {
    #[model(code = 0)] Acquired { module: Id<Module> },
    #[model(code = 1)] Bundled { provider: Id<Provider>, name: String },
    #[model(code = 2)] Unresolved { provider: Id<Provider>, context: Id<AnalysisContext>, name: String },
}
fn validate_provider_module(row: &ProviderModule) -> Result<(),ModelError> {
    match row {
        ProviderModule::Bundled { name, .. } | ProviderModule::Unresolved { name, .. } if name.is_empty() => Err(invalid("provider module needs a name")),
        _ => Ok(()),
    }
}
/// A provider's native symbol key, qualified by its pinned provider and analysis context.
/// Cross-provider equivalence is a later attributed relationship, never a spelling join.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "provider_symbols", validate = validate_symbol, invariants = native_support_invariants)]
pub struct ProviderSymbol {
    #[model(key, provenance)] pub provider: Id<Provider>,
    #[model(key)] pub context: Id<AnalysisContext>,
    #[model(key)] pub module: Id<ProviderModule>,
    #[model(key)] pub native_key: String,
    pub name: String,
    pub kind: SymbolKind,
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
fn validate_symbol(row: &ProviderSymbol) -> Result<(),ModelError> {
    if row.native_key.is_empty() { return Err(invalid("provider symbol needs a native key")); }
    Ok(())
}
fn provider_module_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "provider_module_owners", inputs: vec![ValidationInput::of::<ProviderModule>(&["id"]),
        ValidationInput::of::<ProviderSymbol>(&["id"]), ValidationInput::of::<super::types::TypeVariable>(&["id"])],
        create: std::sync::Arc::new(|budget| Box::new(ModuleOwners { charge: StateCharge::new(budget, "provider_module_owners"), ..Default::default() })) }]
}
/// A bundled module belongs to one provider and an unresolved one to one provider and context.
#[derive(Default)]
struct ModuleOwners { charge: StateCharge, modules: ChargedMap<Id<ProviderModule>, ProviderModule> }
impl ModuleOwners {
    fn owns(&self, module: Id<ProviderModule>, provider: Id<Provider>, context: Id<AnalysisContext>) -> Result<(),ModelError> {
        match self.modules.get(&module).ok_or_else(|| invalid("provider module absent"))? {
            ProviderModule::Acquired { .. } => Ok(()),
            ProviderModule::Bundled { provider: owner, .. } if *owner == provider => Ok(()),
            ProviderModule::Unresolved { provider: owner, context: scope, .. } if (*owner, *scope) == (provider, context) => Ok(()),
            _ => Err(invalid("provider module belongs to another provider or context")),
        }
    }
}
impl InvariantCheck for ModuleOwners {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == ProviderModule::NAME { for row in ProviderModule::decode(batch)? { self.modules.insert(&mut self.charge, row.id(), row)?; } }
        else if relation == ProviderSymbol::NAME { for row in ProviderSymbol::decode(batch)? { self.owns(row.module, row.provider, row.context)?; } }
        else if relation == super::types::TypeVariable::NAME { for row in super::types::TypeVariable::decode(batch)? { self.owns(row.module, row.provider, row.context)?; } }
        else { return Err(invalid("undeclared provider module input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { Ok(()) }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ParameterKind { PositionalOnly = 0, PositionalOrKeyword = 1, VarPositional = 2, KeywordOnly = 3, VarKeyword = 4 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SignatureForm { List = 0, Ellipsis = 1, ParamSpec = 2 }
/// A parameter's structural shape. Its position/owner is SignatureParameter, not this value.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "parameter_shapes", validate = validate_parameter)]
pub struct ParameterShape {
    #[model(key)] pub name: Option<String>,
    #[model(key)] pub kind: ParameterKind,
    #[model(key)] pub required: bool,
}
fn validate_parameter(row: &ParameterShape) -> Result<(),ModelError> {
    if row.name.as_ref().is_some_and(String::is_empty)
        || (row.name.is_none() && matches!(row.kind,ParameterKind::PositionalOrKeyword|ParameterKind::KeywordOnly))
        || (row.required && matches!(row.kind,ParameterKind::VarPositional|ParameterKind::VarKeyword)) {
        return Err(invalid("invalid parameter shape"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "signature_observations", invariants = signature_invariants)]
#[assertion(support = SignatureSupport, name = "signature_supports", family = FactFamily::Signatures, subjects(scope))]
pub struct Signature {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key)] pub symbol: Id<ProviderSymbol>,
    #[model(key)] pub variant: i64,
    #[model(key)] pub form: SignatureForm,
    #[model(key)] pub parameters: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "signature_parameters", validate = validate_member)]
pub struct SignatureParameter {
    #[model(key)] pub signature: Id<Signature>,
    #[model(key)] pub ordinal: i64,
    pub shape: Id<ParameterShape>,
}
fn validate_member(row: &SignatureParameter) -> Result<(),ModelError> {
    if row.ordinal < 0 { return Err(invalid("negative parameter ordinal")); } Ok(())
}
fn parameter_digest(shapes: &[Id<ParameterShape>]) -> ContentHash {
    let mut sink = KeySink::new("signature-parameters");
    for (ordinal,shape) in shapes.iter().enumerate() { (ordinal as i64).encode(&mut sink); shape.encode(&mut sink); }
    (shapes.len() as i64).encode(&mut sink); sink.finish()
}
impl Signature {
    pub fn new(qualification: &AssertionQualification, symbol: Id<ProviderSymbol>, variant: i64,
        form: SignatureForm, parameters: &[ParameterShape]) -> Result<(Self,Vec<SignatureParameter>),ModelError> {
        validate_shapes(form,parameters)?;
        if variant < 0 { return Err(invalid("negative signature variant")); }
        let shapes: Vec<_> = parameters.iter().map(Record::id).collect();
        let row = Self { qualification: qualification.id(),scope: qualification.scope,symbol,variant,form,parameters: parameter_digest(&shapes) };
        let members = shapes.into_iter().enumerate().map(|(i,shape)| SignatureParameter {
            signature: row.id(),ordinal: i as i64,shape,
        }).collect();
        Ok((row,members))
    }
}
fn validate_shapes(form: SignatureForm, parameters: &[ParameterShape]) -> Result<(),ModelError> {
    if parameters.len() > 4096 || (form != SignatureForm::List && !parameters.is_empty()) {
        return Err(invalid("unsupported signature shape or parameter work limit"));
    }
    let mut names = BTreeSet::new(); let mut previous = 0; let mut optional_positional = false;
    let mut varargs = false; let mut kwargs = false;
    for parameter in parameters {
        parameter.validate()?;
        if parameter.name.as_ref().is_some_and(|name| !names.insert(name)) { return Err(invalid("duplicate parameter name")); }
        let kind = parameter.kind as i16;
        if kind < previous { return Err(invalid("parameter kinds out of declaration order")); } previous = kind;
        match parameter.kind {
            ParameterKind::VarPositional if std::mem::replace(&mut varargs,true) => return Err(invalid("duplicate varargs")),
            ParameterKind::VarKeyword if std::mem::replace(&mut kwargs,true) => return Err(invalid("duplicate kwargs")),
            ParameterKind::PositionalOnly | ParameterKind::PositionalOrKeyword => {
                if optional_positional && parameter.required { return Err(invalid("required positional follows default")); }
                optional_positional |= !parameter.required;
            }
            _ => {},
        }
    }
    Ok(())
}
fn signature_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "complete_signature_membership", inputs: vec![
        ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<ProviderSymbol>(&["id"]),
        ValidationInput::of::<ParameterShape>(&["id"]),ValidationInput::of::<Signature>(&["id"]),
        ValidationInput::of::<SignatureParameter>(&["signature","ordinal"]),
    ], create: std::sync::Arc::new(|budget| Box::new(SignatureCheck { charge: StateCharge::new(budget,"complete_signature_membership"),..Default::default() })) }]
}
#[derive(Default)]
struct SignatureCheck { charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    symbols: ChargedMap<Id<ProviderSymbol>,ProviderSymbol>, shapes: ChargedMap<Id<ParameterShape>,ParameterShape>,
    signatures: ChargedMap<Id<Signature>,Signature>, members: ChargedMap<Id<Signature>,Vec<SignatureParameter>>,
}
impl InvariantCheck for SignatureCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == AssertionQualification::NAME { for row in AssertionQualification::decode(batch)? { self.qualifications.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == ProviderSymbol::NAME { for row in ProviderSymbol::decode(batch)? { self.symbols.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == ParameterShape::NAME { for row in ParameterShape::decode(batch)? { self.shapes.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == Signature::NAME { for row in Signature::decode(batch)? { self.signatures.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == SignatureParameter::NAME {
            for row in SignatureParameter::decode(batch)? {
                if self.members.get(&row.signature).is_some_and(|members| members.len() >= 4096) { return Err(invalid("signature parameter work limit")); }
                self.members.update(&mut self.charge,row.signature,|members| members.push(row))?;
            }
        } else { return Err(invalid("undeclared signature validation input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> {
        for row in self.signatures.values() {
            let qualification = self.qualifications.get(&row.qualification).ok_or_else(|| invalid("signature qualification absent"))?;
            let symbol = self.symbols.get(&row.symbol).ok_or_else(|| invalid("signature symbol absent"))?;
            if qualification.scope != row.scope || qualification.context != symbol.context || row.variant < 0 {
                return Err(invalid("signature scope/context/variant mismatch"));
            }
            let members = self.members.get(&row.id()).map(Vec::as_slice).unwrap_or_default();
            let shapes = resolve_parameters(row,members,&self.shapes)?;
            validate_shapes(row.form,&shapes)?;
        }
        if self.members.keys().any(|id| !self.signatures.contains_key(id)) { return Err(invalid("signature member has absent owner")); }
        Ok(())
    }
}
fn resolve_parameters(signature: &Signature, members: &[SignatureParameter], shapes: &BTreeMap<Id<ParameterShape>,ParameterShape>) -> Result<Vec<ParameterShape>,ModelError> {
    if members.len() > 4096 { return Err(invalid("signature parameter work limit")); }
    for (ordinal,member) in members.iter().enumerate() {
        if member.signature != signature.id() || member.ordinal != ordinal as i64 { return Err(invalid("signature parameters missing, repeated or from another variant")); }
    }
    if parameter_digest(&members.iter().map(|m| m.shape).collect::<Vec<_>>()) != signature.parameters {
        return Err(invalid("signature parameter set differs from declaration"));
    }
    members.iter().map(|m| {
        let shape = shapes.get(&m.shape).ok_or_else(|| invalid("parameter shape absent"))?;
        if shape.id() != m.shape { return Err(invalid("parameter shape lookup key differs from value")); }
        Ok(shape.clone())
    }).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, DomainCode)]
#[repr(i16)]
pub enum CallPhase { Call = 0, New = 1, Init = 2, Decorator = 3, PropertyGet = 4, PropertySet = 5, Definition = 6 }
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "call_channels", validate = validate_channel)]
pub enum CallChannel {
    #[model(code = 0)] Direct,
    #[model(code = 1)] HigherOrder { argument_index: i64 },
}
fn validate_channel(row: &CallChannel) -> Result<(),ModelError> {
    if matches!(row,CallChannel::HigherOrder { argument_index } if *argument_index < 0) { return Err(invalid("negative higher-order argument index")); } Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "call_destinations", validate = validate_destination)]
pub enum CallDestination {
    #[model(code = 0)] Resolved { symbol: Id<ProviderSymbol> },
    #[model(code = 1)] Unresolved { reason: ObligationKind },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "call_receivers", validate = validate_receiver)]
pub enum Receiver {
    #[model(code = 0)] None,
    #[model(code = 1)] Bound { actual: Id<Occurrence> },
    #[model(code = 2)] Unknown { reason: ObligationKind },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "call_target_observations", invariants = target_invariants)]
#[assertion(support = CallTargetSupport, name = "call_target_supports", family = FactFamily::Calls, subjects(site))]
pub struct CallTarget {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub site: Id<Occurrence>,
    #[model(key)] pub destination: Id<CallDestination>,
    #[model(key)] pub channel: Id<CallChannel>,
    #[model(key)] pub phase: CallPhase,
    #[model(key)] pub receiver: Id<Receiver>,
    #[model(key)] pub implicit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ArgumentKind { Positional = 0, Starred = 1, Keyword = 2, DoubleStarred = 3, Implicit = 4 }
/// A producer's view of one argument before it becomes a stored `CallArgument`.
#[derive(Debug, Clone)]
pub struct Actual { pub occurrence: Id<Occurrence>,pub kind: ArgumentKind,pub keyword: Option<String> }

/// A provider's syntax for one call site: its callee expression and complete ordered argument list.
/// Arguments are relationship rows; the digest fixes their membership and order.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "call_syntax", invariants = call_syntax_invariants)]
#[assertion(support = CallSyntaxSupport, name = "call_syntax_supports", family = FactFamily::Syntax, subjects(site, callee))]
pub struct CallSyntax {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub site: Id<Occurrence>,
    #[model(key)] pub callee: Id<Occurrence>,
    #[model(key)] pub arguments: ContentHash,
    #[model(key)] pub in_annotation: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_arguments", validate = validate_argument)]
pub struct CallArgument {
    #[model(key)] pub call: Id<CallSyntax>,
    #[model(key)] pub ordinal: i64,
    pub kind: ArgumentKind,
    pub keyword: Option<String>,
    pub value: Id<Occurrence>,
}
fn validate_argument(row: &CallArgument) -> Result<(),ModelError> {
    if row.ordinal < 0 || (row.kind == ArgumentKind::Keyword) != row.keyword.is_some() || row.keyword.as_deref() == Some("") {
        return Err(invalid("call argument needs a nonnegative ordinal and a keyword exactly when keyword-passed"));
    }
    Ok(())
}
#[derive(Default)]
struct ArgumentDigest { sink: Option<KeySink>, count: i64 }
impl ArgumentDigest {
    fn push(&mut self, kind: ArgumentKind, keyword: &Option<String>, value: Id<Occurrence>) {
        let sink = self.sink.get_or_insert_with(|| KeySink::new("call-arguments"));
        self.count.encode(sink); kind.encode(sink); keyword.encode(sink); value.encode(sink);
        self.count += 1;
    }
    fn finish(self) -> ContentHash {
        let mut sink = self.sink.unwrap_or_else(|| KeySink::new("call-arguments"));
        self.count.encode(&mut sink); sink.finish()
    }
}
impl CallSyntax {
    pub fn new(qualification: Id<AssertionQualification>, site: Id<Occurrence>, callee: Id<Occurrence>, in_annotation: bool, actuals: &[Actual])
        -> Result<(Self, Vec<CallArgument>),ModelError> {
        let mut digest = ArgumentDigest::default();
        for actual in actuals { digest.push(actual.kind, &actual.keyword, actual.occurrence); }
        let call = Self { qualification, site, callee, arguments: digest.finish(), in_annotation };
        let arguments = actuals.iter().enumerate().map(|(ordinal, actual)| CallArgument { call: call.id(), ordinal: ordinal as i64,
            kind: actual.kind, keyword: actual.keyword.clone(), value: actual.occurrence }).collect::<Vec<_>>();
        for argument in &arguments { argument.validate()?; }
        Ok((call, arguments))
    }
    /// The complete ordered arguments of this call, or a refusal if any is missing or foreign.
    pub fn actuals(&self, arguments: &[CallArgument]) -> Result<Vec<Actual>,ModelError> {
        let mut digest = ArgumentDigest::default();
        for (ordinal, argument) in arguments.iter().enumerate() {
            if argument.call != self.id() || argument.ordinal != ordinal as i64 { return Err(invalid("call arguments are foreign or out of order")); }
            digest.push(argument.kind, &argument.keyword, argument.value);
        }
        if digest.finish() != self.arguments { return Err(invalid("call arguments differ from the call's declared membership")); }
        Ok(arguments.iter().map(|a| Actual { occurrence: a.value, kind: a.kind, keyword: a.keyword.clone() }).collect())
    }
}
fn call_syntax_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "call_syntax_membership", inputs: vec![ValidationInput::of::<Occurrence>(&["id"]),
        ValidationInput::of::<CallSyntax>(&["id"]), ValidationInput::of::<CallArgument>(&["call","ordinal"])],
        create: std::sync::Arc::new(|budget| Box::new(CallSyntaxCheck { charge: StateCharge::new(budget,"call_syntax_membership"),..Default::default() })) }]
}
type Span = (Id<SourceArtifact>, i64, i64, bool);
#[derive(Default)]
struct CallSyntaxCheck {
    charge: StateCharge, occurrences: ChargedMap<Id<Occurrence>, Span>,
    expected: ChargedMap<Id<CallSyntax>, (ContentHash, Id<Occurrence>)>, current: Option<(Id<CallSyntax>, ArgumentDigest)>,
}
impl CallSyntaxCheck {
    fn span(&self, id: Id<Occurrence>) -> Result<Span,ModelError> { self.occurrences.get(&id).copied().ok_or_else(|| invalid("call syntax occurrence absent")) }
    fn inside(&self, child: Id<Occurrence>, site: Id<Occurrence>) -> Result<(),ModelError> {
        let ((source, start, end, _), (site_source, site_start, site_end, _)) = (self.span(child)?, self.span(site)?);
        if source != site_source || start < site_start || end > site_end { return Err(invalid("call syntax part lies outside its call site")); }
        Ok(())
    }
    fn flush(&mut self) -> Result<(),ModelError> {
        if let Some((call, digest)) = self.current.take() {
            let (expected, _) = self.expected.remove(&mut self.charge, &call).ok_or_else(|| invalid("call arguments name an absent or repeated call"))?;
            if digest.finish() != expected { return Err(invalid("call arguments differ from the call's declared membership")); }
        }
        Ok(())
    }
}
impl InvariantCheck for CallSyntaxCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == Occurrence::NAME { for row in Occurrence::decode(batch)? {
            let call = row.syntax_kind == SyntaxKind::ExprCall || row.role == OccurrenceRole::Call;
            self.occurrences.insert(&mut self.charge, row.id(), (row.source, row.start, row.end, call))?;
        } }
        else if relation == CallSyntax::NAME { for row in CallSyntax::decode(batch)? {
            if !self.span(row.site)?.3 { return Err(invalid("call syntax site is not a call occurrence")); }
            self.inside(row.callee, row.site)?;
            self.expected.insert(&mut self.charge, row.id(), (row.arguments, row.site))?;
        } }
        else if relation == CallArgument::NAME { for row in CallArgument::decode(batch)? {
            if self.current.as_ref().is_none_or(|(call, _)| *call != row.call) { self.flush()?; self.current = Some((row.call, ArgumentDigest::default())); }
            let site = self.expected.get(&row.call).ok_or_else(|| invalid("call argument names an absent call"))?.1;
            self.inside(row.value, site)?;
            let (_, digest) = self.current.as_mut().expect("current call");
            if row.ordinal != digest.count { return Err(invalid("call arguments have gaps or repeats")); }
            digest.push(row.kind, &row.keyword, row.value);
        } }
        else { return Err(invalid("undeclared call syntax input")); }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(),ModelError> {
        self.flush()?;
        let empty = ArgumentDigest::default().finish();
        if self.expected.values().any(|(digest, _)| *digest != empty) { return Err(invalid("call syntax is missing arguments")); }
        Ok(())
    }
}
/// Call-shape binding does not evaluate defaults or certify normal completion. A Default source
/// names the definition-time slot; value transfer needs its separately proven value/certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSource {
    Actual(Id<Occurrence>), Default, EmptyVarargs, EmptyKwargs,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingKind { Positional, Keyword, Default, Varargs, Kwargs, Receiver, Implicit }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingProjection {
    Whole,
    /// Element position inside the collected *args tuple, including any inserted receiver.
    Positional { index: i64 },
    /// Dictionary key inside the collected **kwargs mapping.
    Keyword { name: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub formal: Id<SignatureParameter>,pub source: BindingSource,
    pub kind: BindingKind,pub projection: BindingProjection,
}
#[derive(Debug, Clone)]
pub struct BoundCall {
    site: Id<Occurrence>, target: Id<CallTarget>, signature: Id<Signature>, bindings: Vec<Binding>,
}
impl BoundCall {
    pub fn site(&self) -> Id<Occurrence> { self.site }
    pub fn target(&self) -> Id<CallTarget> { self.target }
    pub fn signature(&self) -> Id<Signature> { self.signature }
    pub fn bindings(&self) -> &[Binding] { &self.bindings }
}
/// All joins and all parameters of one signature are required. A caller cannot pass an arbitrary
/// subset of successfully bound rows and ask it to certify the whole invocation.
pub struct BindingInput<'a> {
    pub target: &'a CallTarget, pub qualification: &'a AssertionQualification,
    pub signature_qualification: &'a AssertionQualification, pub destination: &'a CallDestination, pub channel: &'a CallChannel,
    pub receiver: &'a Receiver, pub signature: &'a Signature,
    pub parameters: &'a [SignatureParameter], pub shapes: &'a BTreeMap<Id<ParameterShape>,ParameterShape>,
    /// The call's syntax and its complete ordered argument set; a partial set refuses.
    pub call: &'a CallSyntax, pub arguments: &'a [CallArgument],
}
pub fn bind(input: BindingInput<'_>) -> Result<BoundCall,ObligationKind> {
    let BindingInput { target,qualification,signature_qualification,destination,channel,receiver,signature,parameters,shapes,call,arguments } = input;
    if call.site != target.site { return Err(ObligationKind::MissingEvidence); }
    let actuals = &call.actuals(arguments).map_err(|_| ObligationKind::MissingEvidence)?;
    if target.qualification != qualification.id() || signature.qualification != signature_qualification.id()
        || qualification.context != signature_qualification.context
        || target.destination != destination.id() || target.channel != channel.id() || target.receiver != receiver.id()
        || !matches!(destination,CallDestination::Resolved { symbol } if *symbol == signature.symbol) {
        return Err(ObligationKind::MissingEvidence);
    }
    if !matches!(channel,CallChannel::Direct) { return Err(ObligationKind::CallTransfer); }
    if !matches!(target.phase,CallPhase::Call|CallPhase::Init|CallPhase::New) { return Err(ObligationKind::OutsideProviderModel); }
    if let Receiver::Unknown { reason } = receiver { return Err(*reason); }
    let formals = resolve_parameters(signature,parameters,shapes).map_err(|_| ObligationKind::MissingEvidence)?;
    validate_shapes(signature.form,&formals).map_err(|_| ObligationKind::AmbiguousBinding)?;
    if signature.form != SignatureForm::List { return Err(ObligationKind::OutsideProviderModel); }
    if actuals.len() > 128 { return Err(ObligationKind::InvocationArgumentLimit); }
    // Unexpanded operands cannot establish a complete binding. Keep the original call evidence;
    // refusing this value transfer is not evidence that Python would reject the invocation.
    if actuals.iter().any(|a| matches!(a.kind,ArgumentKind::Starred|ArgumentKind::DoubleStarred)) {
        return Err(ObligationKind::UnsupportedUnpacking);
    }
    let mut out = vec![]; let mut bound = vec![false;formals.len()];
    let mut positional: Vec<_> = formals.iter().enumerate().filter(|(_,f)| matches!(f.kind,ParameterKind::PositionalOnly|ParameterKind::PositionalOrKeyword)).map(|(i,_)| i).collect();
    let mut vararg_index = 0i64;
    if let Receiver::Bound { actual } = receiver {
        let first = positional.first().copied().or_else(|| formals.first().filter(|f| f.kind == ParameterKind::VarPositional).map(|_| 0))
            .ok_or(ObligationKind::AmbiguousBinding)?;
        if first != 0 { return Err(ObligationKind::AmbiguousBinding); }
        let projection = if formals[first].kind == ParameterKind::VarPositional {
            vararg_index += 1; BindingProjection::Positional { index: 0 }
        } else { BindingProjection::Whole };
        out.push(Binding { formal: parameters[first].id(),source: BindingSource::Actual(*actual),kind: BindingKind::Receiver,projection });
        bound[first] = true; if !positional.is_empty() { positional.remove(0); }
    }
    let varargs = formals.iter().position(|f| f.kind == ParameterKind::VarPositional);
    let kwargs = formals.iter().position(|f| f.kind == ParameterKind::VarKeyword);
    let mut next = 0; let mut names = BTreeSet::new(); let mut occurrences = BTreeSet::new(); let mut keyword_seen = false;
    for actual in actuals {
        if !occurrences.insert(actual.occurrence) || (matches!(actual.kind,ArgumentKind::Keyword) != actual.keyword.is_some()) {
            return Err(ObligationKind::AmbiguousBinding);
        }
        let index = match actual.kind {
            ArgumentKind::Positional|ArgumentKind::Implicit => {
                if keyword_seen { return Err(ObligationKind::AmbiguousBinding); }
                let found = positional.get(next).copied().or(varargs); next += 1; found
            }
            ArgumentKind::Keyword => {
                keyword_seen = true;
                let name = actual.keyword.as_deref().ok_or(ObligationKind::AmbiguousBinding)?;
                if name.is_empty() || !names.insert(name) { return Err(ObligationKind::AmbiguousBinding); }
                formals.iter().position(|f| f.name.as_deref() == Some(name)
                    && matches!(f.kind,ParameterKind::PositionalOrKeyword|ParameterKind::KeywordOnly)).or(kwargs)
            }
            ArgumentKind::Starred|ArgumentKind::DoubleStarred => unreachable!("refused above"),
        }.ok_or(ObligationKind::AmbiguousBinding)?;
        if bound[index] && Some(index) != varargs && Some(index) != kwargs { return Err(ObligationKind::AmbiguousBinding); }
        bound[index] = true;
        let (kind,projection) = if Some(index) == varargs {
            let projection = BindingProjection::Positional { index: vararg_index }; vararg_index += 1;
            (if actual.kind == ArgumentKind::Implicit { BindingKind::Implicit } else { BindingKind::Varargs },projection)
        } else if Some(index) == kwargs {
            (BindingKind::Kwargs,BindingProjection::Keyword { name: actual.keyword.clone().ok_or(ObligationKind::AmbiguousBinding)? })
        } else {
            (match actual.kind { ArgumentKind::Keyword => BindingKind::Keyword,ArgumentKind::Implicit => BindingKind::Implicit,_ => BindingKind::Positional },BindingProjection::Whole)
        };
        out.push(Binding { formal: parameters[index].id(),source: BindingSource::Actual(actual.occurrence),kind,projection });
    }
    for (index,formal) in formals.iter().enumerate() {
        if bound[index] { continue; }
        let source = match formal.kind {
            ParameterKind::VarPositional => BindingSource::EmptyVarargs,
            ParameterKind::VarKeyword => BindingSource::EmptyKwargs,
            _ if !formal.required => BindingSource::Default,
            _ => return Err(ObligationKind::AmbiguousBinding),
        };
        let kind = match formal.kind { ParameterKind::VarPositional => BindingKind::Varargs,
            ParameterKind::VarKeyword => BindingKind::Kwargs,_ => BindingKind::Default };
        out.push(Binding { formal: parameters[index].id(),source,kind,projection: BindingProjection::Whole });
    }
    Ok(BoundCall { site: target.site,target: target.id(),signature: signature.id(),bindings: out })
}

/// Incomplete receiver evidence is a first-class Unknown, never an implicit plain function.
#[derive(Debug, Clone, Copy)]
pub struct ReceiverEvidence {
    pub implicit_receiver: Option<bool>, pub static_method: Option<bool>,
    pub class_method: Option<bool>, pub attribute_access: bool, pub actual: Option<Id<Occurrence>>,
}
pub fn classify_receiver(evidence: ReceiverEvidence) -> Receiver {
    let unknown = || Receiver::Unknown { reason: ObligationKind::AmbiguousBinding };
    if evidence.static_method == Some(true) && evidence.class_method == Some(true) { return unknown(); }
    match evidence.implicit_receiver {
        Some(true) if evidence.static_method == Some(true) => unknown(),
        Some(true) => evidence.actual.map(|actual| Receiver::Bound { actual }).unwrap_or_else(unknown),
        Some(false) if evidence.class_method == Some(true) => unknown(),
        Some(false) => Receiver::None,
        None if evidence.static_method == Some(true) => Receiver::None,
        None if evidence.class_method == Some(true) => evidence.actual.map(|actual| Receiver::Bound { actual }).unwrap_or_else(unknown),
        None if evidence.attribute_access => unknown(),
        None => unknown(),
    }
}

/// The declaration of the complete alternative set is independent of which policy consumes it.
/// Its digest includes unresolved and otherwise inadmissible alternatives. Filtering first cannot
/// manufacture uniqueness. Completeness remains an attributed provider claim.
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "call_resolutions", invariants = resolution_invariants)]
#[assertion(support = CallResolutionSupport, name = "call_resolution_supports", family = FactFamily::Calls, subjects(site))]
pub struct CallResolution {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub site: Id<Occurrence>,
    #[model(key)] pub channel: Id<CallChannel>,
    #[model(key)] pub phase: CallPhase,
    #[model(key)] pub alternatives: ContentHash,
    #[model(key)] pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_resolution_members")]
pub struct CallResolutionMember {
    #[model(key)] pub resolution: Id<CallResolution>,
    #[model(key)] pub target: Id<CallTarget>,
}
fn alternative_digest(alternatives: &BTreeSet<Id<CallTarget>>) -> ContentHash {
    let mut sink = KeySink::new("call-alternatives");
    for alternative in alternatives { alternative.encode(&mut sink); }
    (alternatives.len() as i64).encode(&mut sink); sink.finish()
}
impl CallResolution {
    pub fn new(qualification: &AssertionQualification, site: Id<Occurrence>, channel: Id<CallChannel>,
        phase: CallPhase, complete: bool, targets: &[CallTarget]) -> Result<(Self,Vec<CallResolutionMember>),ModelError> {
        if targets.len() > 4096 || targets.iter().any(|target| target.site != site || target.channel != channel || target.phase != phase) {
            return Err(invalid("call alternatives differ in site/channel/phase or exceed work limit"));
        }
        let targets: BTreeSet<_> = targets.iter().map(Record::id).collect();
        let row = Self { qualification: qualification.id(),site,channel,phase,complete,alternatives: alternative_digest(&targets) };
        let members = targets.into_iter().map(|target| CallResolutionMember { resolution: row.id(),target }).collect();
        Ok((row,members))
    }
}
fn resolution_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "complete_call_alternatives", inputs: vec![
        ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<CallTarget>(&["id"]),
        ValidationInput::of::<CallResolution>(&["id"]),ValidationInput::of::<CallResolutionMember>(&["resolution","target"]),
    ], create: std::sync::Arc::new(|budget| Box::new(ResolutionCheck { charge: StateCharge::new(budget,"complete_call_alternatives"),..Default::default() })) }]
}
#[derive(Default)]
struct ResolutionCheck { charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    targets: ChargedMap<Id<CallTarget>,CallTarget>,resolutions: ChargedMap<Id<CallResolution>,CallResolution>,
    members: ChargedMap<Id<CallResolution>,BTreeSet<Id<CallTarget>>>,
}
impl InvariantCheck for ResolutionCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == AssertionQualification::NAME { for row in AssertionQualification::decode(batch)? { self.qualifications.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == CallTarget::NAME { for row in CallTarget::decode(batch)? { self.targets.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == CallResolution::NAME { for row in CallResolution::decode(batch)? { self.resolutions.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == CallResolutionMember::NAME { for row in CallResolutionMember::decode(batch)? {
            if self.members.get(&row.resolution).is_some_and(|members| members.len() >= 4096)
                || !self.members.update(&mut self.charge,row.resolution,|members| members.insert(row.target))? {
                return Err(invalid("call alternatives repeated or work limit exceeded"));
            }
        } } else { return Err(invalid("undeclared call resolution validation input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> {
        for resolution in self.resolutions.values() {
            let empty = BTreeSet::new(); let members = self.members.get(&resolution.id()).unwrap_or(&empty);
            if alternative_digest(members) != resolution.alternatives { return Err(invalid("call resolution alternatives differ from declaration")); }
            let qualification = self.qualifications.get(&resolution.qualification).ok_or_else(|| invalid("resolution qualification absent"))?;
            for id in members {
                let target = self.targets.get(id).ok_or_else(|| invalid("call resolution target absent"))?;
                let target_qualification = self.qualifications.get(&target.qualification).ok_or_else(|| invalid("target qualification absent"))?;
                if target.site != resolution.site || target.channel != resolution.channel || target.phase != resolution.phase
                    || qualification.context != target_qualification.context || qualification.scope != target_qualification.scope {
                    return Err(invalid("call alternative outside resolution site/channel/phase/context/scope"));
                }
            }
        }
        if self.members.keys().any(|id| !self.resolutions.contains_key(id)) { return Err(invalid("call resolution member owner absent")); }
        Ok(())
    }
}

pub struct CallCandidate<'a> {
    pub target: &'a CallTarget, pub qualification: &'a AssertionQualification,
    pub destination: &'a CallDestination, pub symbol: Option<&'a ProviderSymbol>,
    pub receiver: &'a Receiver, pub supports: &'a [CallTargetSupport],
}
pub struct TargetSet<'a> {
    resolution: &'a CallResolution, qualification: &'a AssertionQualification,
    channel: &'a CallChannel, candidates: Vec<CallCandidate<'a>>,
}
impl<'a> TargetSet<'a> {
    pub fn new(resolution: &'a CallResolution, qualification: &'a AssertionQualification,
        channel: &'a CallChannel, candidates: Vec<CallCandidate<'a>>) -> Result<Self,ModelError> {
        if candidates.len() > 4096 || resolution.qualification != qualification.id() || resolution.channel != channel.id() {
            return Err(invalid("call resolution qualification/channel mismatch or alternative work limit"));
        }
        let mut ids = BTreeSet::new();
        for candidate in &candidates {
            let target = candidate.target;
            if !ids.insert(target.id()) || target.site != resolution.site || target.phase != resolution.phase
                || target.channel != channel.id() || target.qualification != candidate.qualification.id()
                || candidate.qualification.context != qualification.context || candidate.qualification.scope != qualification.scope
                || target.destination != candidate.destination.id() || target.receiver != candidate.receiver.id()
                || candidate.supports.is_empty() || candidate.supports.iter().any(|support| support.assertion != target.id()) {
                return Err(invalid("incomplete or crossed call candidate"));
            }
            match (candidate.destination,candidate.symbol) {
                (CallDestination::Resolved { symbol },Some(row)) if *symbol == row.id() && row.context == qualification.context => {},
                (CallDestination::Unresolved { .. },None) => {},
                _ => return Err(invalid("call destination symbol mismatch")),
            }
        }
        if alternative_digest(&ids) != resolution.alternatives { return Err(invalid("policy input omits call alternatives")); }
        Ok(Self { resolution,qualification,channel,candidates })
    }
}

/// Phases that together form one invocation event at a site: constructing `C()` runs `__new__`
/// and then `__init__`. Every other phase is its own group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhaseGroup { Call, Construct, Decorator, Property, Definition }
impl PhaseGroup {
    pub fn of(phase: CallPhase) -> Self {
        match phase {
            CallPhase::Call => Self::Call, CallPhase::New | CallPhase::Init => Self::Construct,
            CallPhase::Decorator => Self::Decorator, CallPhase::PropertyGet | CallPhase::PropertySet => Self::Property,
            CallPhase::Definition => Self::Definition,
        }
    }
}
/// What every resolution at one site jointly says (C04). Only direct, non-potential alternatives
/// count. The site is unique when one phase group is present, each of its phases names one symbol
/// and no alternative is unresolved; differing symbols are a disagreement (cross-provider
/// equivalence is a later relationship). It is complete only if every direct resolution is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteFacts {
    pub targets: BTreeMap<CallPhase, BTreeSet<Id<ProviderSymbol>>>, pub group: Option<PhaseGroup>,
    pub unique: bool, pub complete: bool, pub disagreement: bool, pub unresolved: bool,
}
pub fn site_facts(sets: &[TargetSet<'_>]) -> SiteFacts {
    let mut targets: BTreeMap<CallPhase, BTreeSet<Id<ProviderSymbol>>> = BTreeMap::new();
    let (mut unresolved, mut complete) = (false, true);
    for set in sets.iter().filter(|set| matches!(set.channel, CallChannel::Direct)) {
        complete &= set.resolution.complete;
        for candidate in set.candidates.iter().filter(|c| c.qualification.modality != Modality::Potential) {
            match candidate.destination {
                CallDestination::Resolved { symbol } => { targets.entry(candidate.target.phase).or_default().insert(*symbol); },
                CallDestination::Unresolved { .. } => unresolved = true,
            }
        }
    }
    let groups: BTreeSet<_> = targets.keys().map(|phase| PhaseGroup::of(*phase)).collect();
    let disagreement = targets.values().any(|symbols| symbols.len() > 1);
    SiteFacts { group: (groups.len() == 1).then(|| *groups.first().expect("one group")),
        unique: groups.len() == 1 && !disagreement && !unresolved, complete: complete && !targets.is_empty(), disagreement, unresolved, targets }
}
/// Every resolution at one site. Policies are asked here, never of one resolution alone, so
/// uniqueness cannot be decided from a filtered or partial view of the site.
pub struct SiteTargets<'a> { sets: Vec<TargetSet<'a>>, facts: SiteFacts }
impl<'a> SiteTargets<'a> {
    pub fn new(sets: Vec<TargetSet<'a>>) -> Result<Self,ModelError> {
        let first = sets.first().ok_or_else(|| invalid("a call site needs at least one resolution"))?;
        let (site, context) = (first.resolution.site, first.qualification.context);
        if sets.iter().any(|set| set.resolution.site != site || set.qualification.context != context) {
            return Err(invalid("site targets mix call sites or analysis contexts"));
        }
        let facts = site_facts(&sets);
        Ok(Self { sets, facts })
    }
    pub fn facts(&self) -> &SiteFacts { &self.facts }
    pub fn admitted(&self, policy: CallPolicy) -> Vec<Id<CallTarget>> {
        self.sets.iter().flat_map(|set| set.candidates.iter().filter(move |candidate| policy.admits(set, candidate, &self.facts)).map(|c| c.target.id())).collect()
    }
}

/// One provider-native callee report for a call site, before normalization.
#[derive(Debug, Clone)]
pub struct NativeCallee {
    pub phase: CallPhase, pub channel: CallChannel, pub destination: CallDestination,
    pub receiver: ReceiverEvidence, pub implicit: bool, pub modality: Modality,
}
/// The typed rows one site normalizes to: one resolution per (channel, phase) with its complete
/// alternative membership.
#[derive(Debug, Default)]
pub struct NormalizedSite {
    pub qualifications: Vec<AssertionQualification>, pub channels: Vec<CallChannel>, pub destinations: Vec<CallDestination>,
    pub receivers: Vec<Receiver>, pub targets: Vec<CallTarget>, pub resolutions: Vec<CallResolution>, pub members: Vec<CallResolutionMember>,
}
/// Normalize one provider's callee reports for a site. Receivers are classified only by
/// `classify_receiver`. A (channel, phase) group is complete only when the provider says so in
/// `complete`; absence of further callees never implies completeness. An empty report, or an
/// explicit `unresolved` remainder, adds an unresolved direct call alternative.
pub fn normalize_site(base: &AssertionQualification, site: Id<Occurrence>, callees: &[NativeCallee],
    complete: &BTreeSet<(Id<CallChannel>, CallPhase)>, unresolved: Option<ObligationKind>) -> Result<NormalizedSite,ModelError> {
    let mut groups: BTreeMap<(i64, CallPhase), (CallChannel, Vec<CallTarget>)> = BTreeMap::new();
    let mut site_rows = NormalizedSite::default();
    let mut add = |channel: &CallChannel, phase: CallPhase, destination: CallDestination, receiver: Receiver, implicit: bool, modality: Modality, rows: &mut NormalizedSite| {
        let qualification = AssertionQualification { modality, ..base.clone() };
        let target = CallTarget { qualification: qualification.id(), site, destination: destination.id(), channel: channel.id(), phase, receiver: receiver.id(), implicit };
        let order = match channel { CallChannel::Direct => -1, CallChannel::HigherOrder { argument_index } => *argument_index };
        groups.entry((order, phase)).or_insert_with(|| (channel.clone(), Vec::new())).1.push(target);
        if !rows.qualifications.contains(&qualification) { rows.qualifications.push(qualification); }
        if !rows.channels.contains(channel) { rows.channels.push(channel.clone()); }
        if !rows.destinations.contains(&destination) { rows.destinations.push(destination); }
        if !rows.receivers.contains(&receiver) { rows.receivers.push(receiver); }
    };
    for callee in callees {
        add(&callee.channel, callee.phase, callee.destination.clone(), classify_receiver(callee.receiver), callee.implicit, callee.modality, &mut site_rows);
    }
    if callees.is_empty() || unresolved.is_some() {
        let reason = unresolved.unwrap_or(ObligationKind::UnresolvedTarget);
        add(&CallChannel::Direct, CallPhase::Call, CallDestination::Unresolved { reason }, Receiver::Unknown { reason: ObligationKind::AmbiguousBinding },
            false, base.modality, &mut site_rows);
    }
    if !site_rows.qualifications.contains(base) { site_rows.qualifications.push(base.clone()); }
    for (_, (channel, targets)) in groups {
        let phase = targets[0].phase;
        let has_unresolved = targets.iter().any(|t| site_rows.destinations.iter().any(|d| d.id() == t.destination && matches!(d, CallDestination::Unresolved { .. })));
        let claimed = complete.contains(&(channel.id(), phase)) && !has_unresolved;
        let (resolution, members) = CallResolution::new(base, site, channel.id(), phase, claimed, &targets)?;
        site_rows.targets.extend(targets); site_rows.resolutions.push(resolution); site_rows.members.extend(members);
    }
    Ok(site_rows)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPolicy { Invocation, Dataflow, Summary, Usage, Association }
impl CallPolicy {
    fn admits(self, set: &TargetSet<'_>, candidate: &CallCandidate<'_>, site: &SiteFacts) -> bool {
        let target = candidate.target; let qualified = candidate.qualification;
        let ordinary_modality = matches!(qualified.modality,Modality::Definite|Modality::Candidate);
        let origin = |wanted| candidate.supports.iter().any(|support| support.origin == wanted);
        let direct = matches!(set.channel,CallChannel::Direct);
        let callable = candidate.symbol.is_some_and(|symbol| matches!(symbol.kind,SymbolKind::Function|SymbolKind::Method));
        let flow_phase = matches!(target.phase,CallPhase::Call|CallPhase::Init);
        match self {
            Self::Invocation => direct && ordinary_modality && origin(Origin::AnalyzerAssertion)
                && matches!(target.phase,CallPhase::Call|CallPhase::PropertyGet|CallPhase::PropertySet),
            Self::Dataflow => direct && ordinary_modality && callable && flow_phase,
            Self::Summary => direct && callable && matches!(target.phase,CallPhase::Call|CallPhase::New|CallPhase::Init)
                && site.unique && site.complete
                && set.qualification.modality == Modality::Definite && set.qualification.approximation == Approximation::Exact
                && qualified.modality == Modality::Definite && qualified.approximation == Approximation::Exact
                && !matches!(candidate.receiver,Receiver::Unknown { .. }),
            Self::Usage => ordinary_modality && (origin(Origin::AnalyzerAssertion)||origin(Origin::SourceObservation)),
            Self::Association => true,
        }
    }
}

fn validate_destination(row: &CallDestination) -> Result<(),ModelError> {
    if matches!(row,CallDestination::Unresolved { reason: ObligationKind::ResponseBudget|ObligationKind::NotRequested }) {
        return Err(invalid("presentation/not-requested is not an unresolved call cause"));
    }
    Ok(())
}
fn validate_receiver(row: &Receiver) -> Result<(),ModelError> {
    if matches!(row,Receiver::Unknown { reason: ObligationKind::ResponseBudget|ObligationKind::NotRequested }) {
        return Err(invalid("presentation/not-requested is not an unknown receiver cause"));
    }
    Ok(())
}
fn target_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "call_target_ownership", inputs: vec![
        ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<ProviderSymbol>(&["id"]),
        ValidationInput::of::<Occurrence>(&["id"]),ValidationInput::of::<Receiver>(&["id"]),
        ValidationInput::of::<CallDestination>(&["id"]),ValidationInput::of::<CallTarget>(&["id"]),
    ], create: std::sync::Arc::new(|budget| Box::new(TargetCheck { charge: StateCharge::new(budget,"call_target_ownership"),..Default::default() })) }]
}
#[derive(Default)]
struct TargetCheck { charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>,AssertionQualification>,
    symbols: ChargedMap<Id<ProviderSymbol>,ProviderSymbol>,occurrences: ChargedMap<Id<Occurrence>,Occurrence>,
    receivers: ChargedMap<Id<Receiver>,Receiver>,destinations: ChargedMap<Id<CallDestination>,CallDestination>,
}
impl InvariantCheck for TargetCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == AssertionQualification::NAME { for row in AssertionQualification::decode(batch)? { self.qualifications.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == ProviderSymbol::NAME { for row in ProviderSymbol::decode(batch)? { self.symbols.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == Occurrence::NAME { for row in Occurrence::decode(batch)? { self.occurrences.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == Receiver::NAME { for row in Receiver::decode(batch)? { self.receivers.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == CallDestination::NAME { for row in CallDestination::decode(batch)? { self.destinations.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == CallTarget::NAME {
            for row in CallTarget::decode(batch)? {
                let qualification = self.qualifications.get(&row.qualification).ok_or_else(|| invalid("call qualification absent"))?;
                let destination = self.destinations.get(&row.destination).ok_or_else(|| invalid("call destination absent"))?;
                if let CallDestination::Resolved { symbol } = destination {
                    let symbol = self.symbols.get(symbol).ok_or_else(|| invalid("call symbol absent"))?;
                    if symbol.context != qualification.context { return Err(invalid("call symbol context differs from assertion")); }
                }
                let receiver = self.receivers.get(&row.receiver).ok_or_else(|| invalid("call receiver absent"))?;
                if let Receiver::Bound { actual } = receiver {
                    let site = self.occurrences.get(&row.site).ok_or_else(|| invalid("call site absent"))?;
                    let actual = self.occurrences.get(actual).ok_or_else(|| invalid("call receiver occurrence absent"))?;
                    if site.source != actual.source || site.start > actual.start || site.end < actual.end {
                        return Err(invalid("bound receiver lies outside call occurrence"));
                    }
                }
            }
        } else { return Err(invalid("undeclared call ownership input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { Ok(()) }
}

/// A native provider key is meaningful only in that provider's namespace. Supporting an
/// equivalent symbol from another provider requires a later explicit equivalence relationship.
fn native_support_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "native_symbol_support_ownership", inputs: vec![
        ValidationInput::of::<ProviderSymbol>(&["id"]),ValidationInput::of::<ProviderRun>(&["id"]),
        ValidationInput::of::<CallDestination>(&["id"]),ValidationInput::of::<Signature>(&["id"]),
        ValidationInput::of::<CallTarget>(&["id"]),ValidationInput::of::<SignatureSupport>(&["id"]),
        ValidationInput::of::<CallTargetSupport>(&["id"]),
    ], create: std::sync::Arc::new(|budget| Box::new(NativeSupportCheck { charge: StateCharge::new(budget,"native_symbol_support_ownership"),..Default::default() })) }]
}
#[derive(Default)]
struct NativeSupportCheck { charge: StateCharge,
    symbols: ChargedMap<Id<ProviderSymbol>,Id<Provider>>,runs: ChargedMap<Id<ProviderRun>,Id<Provider>>,
    destinations: ChargedMap<Id<CallDestination>,CallDestination>,
    signatures: ChargedMap<Id<Signature>,Id<ProviderSymbol>>,targets: ChargedMap<Id<CallTarget>,Id<CallDestination>>,
}
impl NativeSupportCheck {
    fn check(&self, symbol: Id<ProviderSymbol>,run: Id<ProviderRun>) -> Result<(),ModelError> {
        let provider = self.symbols.get(&symbol).ok_or_else(|| invalid("native symbol absent"))?;
        if self.runs.get(&run) != Some(provider) { return Err(invalid("native symbol support belongs to a different provider")); }
        Ok(())
    }
}
impl InvariantCheck for NativeSupportCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == ProviderSymbol::NAME { for row in ProviderSymbol::decode(batch)? { self.symbols.insert(&mut self.charge, row.id(),row.provider)?; } }
        else if relation == ProviderRun::NAME { for row in ProviderRun::decode(batch)? { self.runs.insert(&mut self.charge, row.id(),row.provider)?; } }
        else if relation == CallDestination::NAME { for row in CallDestination::decode(batch)? { self.destinations.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == Signature::NAME { for row in Signature::decode(batch)? { self.signatures.insert(&mut self.charge, row.id(),row.symbol)?; } }
        else if relation == CallTarget::NAME { for row in CallTarget::decode(batch)? { self.targets.insert(&mut self.charge, row.id(),row.destination)?; } }
        else if relation == SignatureSupport::NAME { for row in SignatureSupport::decode(batch)? {
            self.check(*self.signatures.get(&row.assertion).ok_or_else(|| invalid("supported signature absent"))?,row.run)?;
        } }
        else if relation == CallTargetSupport::NAME { for row in CallTargetSupport::decode(batch)? {
            let destination = self.targets.get(&row.assertion).ok_or_else(|| invalid("supported call target absent"))?;
            if let CallDestination::Resolved { symbol } = self.destinations.get(destination).ok_or_else(|| invalid("supported call destination absent"))? {
                self.check(*symbol,row.run)?;
            }
        } } else { return Err(invalid("undeclared native symbol support input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { Ok(()) }
}
