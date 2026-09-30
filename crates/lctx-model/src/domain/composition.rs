//! Whole-call transfer composition (DESIGN §15.6, core review C05/C06/C07).
//!
//! A caller transfer that delivers a value into an argument, the call's binding and a callee
//! transfer compose, matched by call site, into a `Composed` transfer owned by the caller. Roots
//! map through the binder and the parameter declaration links; paths compose only through
//! identity; the condition is the caller's AND the callee's restated at the call, where formal
//! guards need a stability witness. Anything that cannot be justified is an obligation, never an
//! unconditional flow.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{
    assertion::{Approximation, AssertionQualification},
    attribution::{Modality, ObligationKind},
    calls::{
        BindingProjection, BindingSource, BoundCall, CallArgument, CallDestination, CallSyntax,
        CallTarget, ProviderSymbol, Receiver, Signature, SignatureParameter,
    },
    conditions::{
        Condition, ConditionNode, Diagram, EvaluationAtom,
        rebase::{GuardCatalog, RebasedGuards, RootBinding, substitute_call_guards},
        stability::{GuardSubstitution, StabilityWitness},
    },
    declarations::{ParameterDeclaration, SymbolDeclaration},
    place_composition::{self, ComposedPaths, PathCatalog},
    source::Occurrence,
    transfer::{
        ProvenanceClass, TransferAlternative, TransferBranch, TransferKey, TransferKind,
        compose_kinds,
    },
    value::{AccessPath, Literal, PathSegment, Place, PlaceRoot, Predicate},
    *,
};
use crate::Domain;
use std::collections::{BTreeMap, BTreeSet};

/// The derivation of one composed alternative from its caller and callee alternatives.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_compositions", rule = "compose_through_call", conclusion = composed, invariants = composition_invariants)]
pub struct CallCompositionStep {
    #[model(key)]
    pub composed: Id<TransferAlternative>,
    #[model(key, premise)]
    pub caller: Id<TransferAlternative>,
    #[model(key, premise)]
    pub callee: Id<TransferAlternative>,
    #[model(key, premise)]
    pub target: Id<CallTarget>,
    #[model(key, premise)]
    pub signature: Id<Signature>,
    #[model(key, premise)]
    pub callee_declaration: Id<SymbolDeclaration>,
    #[model(key)]
    pub bindings: ContentHash,
}

/// One admitted call target at a site with its complete binding.
pub struct CallFrame<'a> {
    pub site: &'a Occurrence,
    pub target: &'a CallTarget,
    pub qualification: &'a AssertionQualification,
    pub destination: &'a CallDestination,
    pub receiver: &'a Receiver,
    /// The binding of the target's signature variant; `None` only for an unresolved target.
    pub bound: Option<&'a BoundCall>,
    pub arguments: &'a [CallArgument],
    /// The site's Summary policy admits this target and exactly one signature variant binds.
    pub summary_admitted: bool,
    pub unique_variant: bool,
}
/// The caller's declaration, which must be the owner of the call site (the owner rule).
pub struct CallerFrame<'a> {
    pub declaration: &'a SymbolDeclaration,
    pub site_owner: Id<Occurrence>,
}
/// The callee's declaration links over the bound signature; witnesses are `None` when flow facts
/// were not requested.
///
/// Callee transfers are port summaries: an end rooted at `Entry` denotes the caller's value, and
/// the producer must justify each such root by entry-value evidence at the access it summarizes.
/// An end rooted at the `Formal` variable does not. A declared callable's receiver is the entry
/// value of its first parameter; a `Receiver` root is refused here.
pub struct CalleeFrame<'a> {
    pub symbol: &'a ProviderSymbol,
    pub declaration: &'a SymbolDeclaration,
    /// The bound signature's parameters in ordinal order, and one declaration link for each.
    pub parameters: &'a [SignatureParameter],
    pub links: &'a [ParameterDeclaration],
    pub witnesses: Option<&'a BTreeMap<Id<EvaluationAtom>, StabilityWitness>>,
}
/// Existing rows the composer reads; lookups verify identity.
pub struct CompositionCatalog<'a> {
    pub guards: GuardCatalog<'a>,
    pub paths: &'a BTreeMap<Id<AccessPath>, AccessPath>,
    pub segments: PathCatalog<'a>,
}
/// The rows a composition adds, including restated guards and the derivation step.
#[derive(Debug, Default)]
pub struct CompositionRecords {
    pub literals: Vec<Literal>,
    pub segments: Vec<PathSegment>,
    pub paths: Vec<AccessPath>,
    pub roots: Vec<PlaceRoot>,
    pub places: Vec<Place>,
    pub atoms: Vec<EvaluationAtom>,
    pub predicates: Vec<Predicate>,
    pub substitutions: Vec<GuardSubstitution>,
    pub conditions: Vec<Condition>,
    pub nodes: Vec<ConditionNode>,
    pub qualification: Option<AssertionQualification>,
    pub key: Option<TransferKey>,
    pub alternative: Option<TransferAlternative>,
    pub step: Option<CallCompositionStep>,
}
#[derive(Debug)]
pub enum CallComposition {
    Transfer(Box<ComposedTransfer>),
    Disjoint,
    Subsumed,
    Obligation(ObligationKind),
}
#[derive(Debug)]
pub struct ComposedTransfer {
    pub branch: TransferBranch,
    pub records: CompositionRecords,
}

impl Modality {
    /// The weaker of two modalities: a composition is no stronger than any of its parts.
    pub fn weakest(self, other: Self) -> Self {
        self.max(other)
    }
}
impl Approximation {
    /// Approximations accumulate; opposite directions become mixed, and unknown absorbs.
    pub fn join(self, other: Self) -> Self {
        use Approximation::*;
        match (self, other) {
            (Unknown, _) | (_, Unknown) => Unknown,
            (Exact, x) | (x, Exact) => x,
            (a, b) if a == b => a,
            _ => Mixed,
        }
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

/// How one caller value reaches a callee port: the value's source and its position in the port.
#[derive(Clone)]
struct PortBinding {
    source: BindingSource,
    projection: BindingProjection,
}
/// A callee root at this call: an entry-value port, or the parameter variable.
enum Port {
    Entry(Vec<PortBinding>),
    Variable(Vec<PortBinding>),
}
/// The call's ports, total over the bound signature: every parameter has exactly one declaration
/// link and at least one binding. A bound receiver is a binding of the first parameter.
struct Ports {
    callable: Id<Occurrence>,
    by_declaration: BTreeMap<Id<Occurrence>, Vec<PortBinding>>,
}
impl Ports {
    fn build(
        bound: &BoundCall,
        callee: &CalleeFrame<'_>,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        let mut members = BTreeSet::new();
        for (ordinal, parameter) in callee.parameters.iter().enumerate() {
            if parameter.signature != bound.signature() || parameter.ordinal != ordinal as i64 {
                return Err(invalid(
                    "callee parameters are not the bound signature's, in order",
                ));
            }
            members.insert(parameter.id());
        }
        let mut declared = BTreeMap::new();
        let mut declarations = BTreeSet::new();
        for link in callee.links {
            if !members.contains(&link.parameter) {
                return Err(invalid(
                    "a parameter declaration link belongs to another signature",
                ));
            }
            if declared.insert(link.parameter, link.declaration).is_some()
                || !declarations.insert(link.declaration)
            {
                return Err(invalid("parameter declaration links are not one-to-one"));
            }
        }
        if declared.len() != members.len() {
            return Ok(Err(ObligationKind::NoSourceDeclaration));
        }
        let mut by_declaration = BTreeMap::new();
        for parameter in callee.parameters {
            let own: Vec<_> = bound
                .bindings()
                .iter()
                .filter(|b| b.formal == parameter.id())
                .map(|b| PortBinding {
                    source: b.source.clone(),
                    projection: b.projection.clone(),
                })
                .collect();
            if own.is_empty() {
                return Err(invalid("a bound parameter has no binding"));
            }
            by_declaration.insert(declared[&parameter.id()], own);
        }
        Ok(Ok(Self {
            callable: callee.declaration.declaration,
            by_declaration,
        }))
    }
    /// The port a callee root names; `None` for a root that is not a parameter of this callee,
    /// such as an enclosing function's parameter read through a closure.
    fn port(&self, root: &PlaceRoot) -> Result<Option<Port>, ModelError> {
        Ok(match root {
            PlaceRoot::Entry { declaration } => self
                .by_declaration
                .get(declaration)
                .cloned()
                .map(Port::Entry),
            PlaceRoot::Formal { declaration } => self
                .by_declaration
                .get(declaration)
                .cloned()
                .map(Port::Variable),
            PlaceRoot::Receiver { .. } => {
                return Err(invalid(
                    "a declared callable's receiver is its first parameter's entry value",
                ));
            }
            _ => None,
        })
    }
    /// Guards on a formal are restated over the argument bound to it, only when that is one whole
    /// actual; transfers read the same bindings.
    fn guard_bindings(&self, arguments: &[CallArgument]) -> BTreeMap<Id<PlaceRoot>, RootBinding> {
        self.by_declaration
            .iter()
            .filter_map(|(declaration, bindings)| {
                let binding = match bindings.as_slice() {
                    [
                        PortBinding {
                            source: BindingSource::Actual { occurrence: actual },
                            projection: BindingProjection::Whole,
                        },
                    ] => RootBinding::Actual(
                        arguments
                            .iter()
                            .find(|argument| argument.value == *actual)?
                            .clone(),
                    ),
                    [
                        PortBinding {
                            source: BindingSource::Default,
                            ..
                        },
                    ] => RootBinding::Default,
                    _ => RootBinding::EmptyAggregate,
                };
                Some((
                    PlaceRoot::Formal {
                        declaration: *declaration,
                    }
                    .id(),
                    binding,
                ))
            })
            .collect()
    }
}

/// Where a callee output lands on the caller side.
enum Mapped {
    Root {
        root: PlaceRoot,
        prefix: Option<PathSegment>,
    },
    Disjoint,
    Obligation(ObligationKind),
}

struct Composer<'a> {
    site: &'a Occurrence,
    ports: Ports,
    records: CompositionRecords,
}
impl Composer<'_> {
    fn projection(&mut self, projection: &BindingProjection) -> Option<PathSegment> {
        let literal = match projection {
            BindingProjection::Whole => return None,
            BindingProjection::Positional { index } => Literal::Integer {
                decimal: index.to_string(),
            },
            BindingProjection::Keyword { name } => Literal::String {
                value: name.clone().into(),
            },
        };
        let segment = PathSegment::Item { key: literal.id() };
        self.records.literals.push(literal);
        self.records.segments.push(segment.clone());
        Some(segment)
    }
    /// A callee output root on the caller side. Writing a whole parameter slot (rebinding the
    /// variable, or storing one element of a collected aggregate) never reaches the caller; a write
    /// below an entry value mutates the caller's object; below the variable it is unknown.
    fn map_output(
        &mut self,
        root: &PlaceRoot,
        path: &AccessPath,
    ) -> Result<Vec<Mapped>, ModelError> {
        let callable = self.ports.callable;
        Ok(match root {
            PlaceRoot::Return { callable: c } if *c == callable => vec![Mapped::Root {
                root: PlaceRoot::Occurrence {
                    occurrence: self.site.id(),
                },
                prefix: None,
            }],
            PlaceRoot::Yield { callable: c } | PlaceRoot::Raise { callable: c }
                if *c == callable =>
            {
                vec![Mapped::Obligation(ObligationKind::UnsupportedControlFlow)]
            }
            PlaceRoot::Field { .. } | PlaceRoot::Global { .. } => vec![Mapped::Root {
                root: root.clone(),
                prefix: None,
            }],
            _ => match self.ports.port(root)? {
                Some(Port::Variable(_)) if *path == AccessPath::empty() => vec![Mapped::Disjoint],
                Some(Port::Variable(_)) => {
                    vec![Mapped::Obligation(ObligationKind::EntryValueUnknown)]
                }
                Some(Port::Entry(_)) if *path == AccessPath::empty() => vec![Mapped::Disjoint],
                Some(Port::Entry(bindings)) => bindings
                    .iter()
                    .map(|binding| match &binding.source {
                        BindingSource::Actual { occurrence: actual } => {
                            let prefix = self.projection(&binding.projection);
                            Mapped::Root {
                                root: PlaceRoot::Occurrence {
                                    occurrence: *actual,
                                },
                                prefix,
                            }
                        }
                        BindingSource::Default => {
                            Mapped::Obligation(ObligationKind::DefaultUnavailable)
                        }
                        BindingSource::EmptyVarargs | BindingSource::EmptyKwargs => {
                            Mapped::Disjoint
                        }
                    })
                    .collect(),
                None => {
                    return Err(invalid(
                        "a callee transfer's output root is local to the callee or names another callable",
                    ));
                }
            },
        })
    }
}

/// Compose one caller alternative through one call into one callee alternative. Each actual bound
/// into an output root yields its own result; the result list is never empty.
pub fn compose_call(
    caller: &TransferBranch,
    callee: &TransferBranch,
    call: &CallFrame<'_>,
    caller_frame: &CallerFrame<'_>,
    callee_frame: &CalleeFrame<'_>,
    catalog: &CompositionCatalog<'_>,
) -> Result<Vec<CallComposition>, ModelError> {
    let (caller_key, callee_key) = (caller.key(), callee.key());
    if caller_key.owner != caller_frame.declaration.symbol
        || caller_frame.declaration.declaration != caller_frame.site_owner
    {
        return Err(invalid(
            "the caller transfer's owner does not own the call site",
        ));
    }
    if call.target.site != call.site.id()
        || call.target.destination != call.destination.id()
        || call.target.qualification != call.qualification.id()
        || call.target.receiver != call.receiver.id()
    {
        return Err(invalid(
            "call frame site, destination, receiver or qualification differs from its target",
        ));
    }
    match call.destination {
        CallDestination::Unresolved { reason, .. } => {
            return Ok(vec![CallComposition::Obligation(*reason)]);
        }
        CallDestination::Callable { .. } | CallDestination::SyntheticFormatting => {
            return Ok(vec![CallComposition::Obligation(
                ObligationKind::OutsideProviderModel,
            )]);
        }
        CallDestination::Overrides { .. } => {
            return Ok(vec![CallComposition::Obligation(
                ObligationKind::OverrideDispatch,
            )]);
        }
        CallDestination::Resolved { symbol }
            if *symbol != callee_key.owner
                || *symbol != callee_frame.symbol.id()
                || callee_frame.declaration.symbol != *symbol =>
        {
            return Err(invalid("callee transfer's owner is not the call's target"));
        }
        CallDestination::Resolved { .. } => {}
    }
    let bound = call
        .bound
        .ok_or_else(|| invalid("a resolved target composes only through its binding"))?;
    if bound.target() != call.target.id() || bound.site() != call.site.id() {
        return Err(invalid("call frame binding belongs to another target"));
    }
    if caller_key.context != callee_key.context || call.qualification.context != caller_key.context
    {
        return Ok(vec![CallComposition::Obligation(
            ObligationKind::IncompatibleContexts,
        )]);
    }
    let place = |id: Id<Place>| {
        catalog
            .guards
            .places
            .get(&id)
            .ok_or_else(|| invalid("transfer place absent"))
    };
    let root = |id: Id<PlaceRoot>| {
        catalog
            .guards
            .roots
            .get(&id)
            .ok_or_else(|| invalid("transfer place root absent"))
    };
    let path = |id: Id<AccessPath>| {
        catalog
            .paths
            .get(&id)
            .ok_or_else(|| invalid("transfer access path absent"))
    };
    let (caller_input, caller_output) = (place(caller_key.input)?, place(caller_key.output)?);
    let (callee_input, callee_output) = (place(callee_key.input)?, place(callee_key.output)?);
    // The caller must deliver its value into an actual of this call.
    let PlaceRoot::Occurrence { occurrence: actual } = root(caller_output.root)? else {
        return Ok(vec![CallComposition::Disjoint]);
    };
    let ports = match Ports::build(bound, callee_frame)? {
        Ok(ports) => ports,
        Err(kind) => return Ok(vec![CallComposition::Obligation(kind)]),
    };
    let (bindings, entry) = match ports.port(root(callee_input.root)?)? {
        Some(Port::Entry(bindings)) => (bindings, true),
        Some(Port::Variable(bindings)) => (bindings, false),
        None => return Ok(vec![CallComposition::Disjoint]),
    };
    let Some(binding) = bindings
        .into_iter()
        .find(|b| b.source == BindingSource::Actual { occurrence: *actual })
    else {
        return Ok(vec![CallComposition::Disjoint]);
    };
    // The callee reads its variable, which need not hold the caller's value any more.
    if !entry {
        return Ok(vec![CallComposition::Obligation(
            ObligationKind::EntryValueUnknown,
        )]);
    }
    let guard_bindings = ports.guard_bindings(call.arguments);
    let mut composer = Composer {
        site: call.site,
        ports,
        records: CompositionRecords::default(),
    };
    // In port coordinates the caller's value sits at the projection, then the caller's path.
    let mut delivered = AccessPath::empty();
    if let Some(segment) = composer.projection(&binding.projection) {
        delivered = delivered.extend(segment.id());
    }
    let delivered = delivered.append(path(caller_output.path)?);
    // Projection items are new rows until stored; the path catalog must see them to compare.
    let mut segments = catalog.segments.segments.clone();
    let mut literals = catalog.segments.literals.clone();
    for segment in &composer.records.segments {
        segments.insert(segment.id(), segment.clone());
    }
    for literal in &composer.records.literals {
        literals.insert(literal.id(), literal.clone());
    }
    let paths = PathCatalog {
        segments: &segments,
        literals: &literals,
    };
    let composed = place_composition::compose(
        path(caller_input.path)?,
        &delivered,
        path(callee_input.path)?,
        path(callee_output.path)?,
        caller_key.kind,
        callee_key.kind,
        &paths,
    )?;
    let (input_path, output_path, kind) = match composed {
        ComposedPaths::Flow {
            input,
            output,
            kind,
        } => (input, output, kind),
        ComposedPaths::Disjoint => return Ok(vec![CallComposition::Disjoint]),
        ComposedPaths::Obligation(kind) => return Ok(vec![CallComposition::Obligation(kind)]),
    };
    // The callee condition restated at the call; unjustified formal guards refuse the transfer.
    let restated: RebasedGuards = match substitute_call_guards(
        callee.condition(),
        call.site,
        caller_key.context,
        &catalog.guards,
        &guard_bindings,
        callee_frame.witnesses,
    ) {
        Ok(restated) => restated,
        Err(kind) => return Ok(vec![CallComposition::Obligation(kind)]),
    };
    let condition = match caller.condition().and(&restated.condition) {
        Ok(condition) => condition,
        Err(boundary) => {
            return Ok(vec![CallComposition::Obligation(
                super::obligation::from_kernel(boundary),
            )]);
        }
    };
    let modality = if call.summary_admitted && call.unique_variant {
        Modality::Definite
    } else {
        Modality::Candidate
    }
    .weakest(call.qualification.modality)
    .weakest(caller.qualification().modality)
    .weakest(callee.qualification().modality);
    let approximation = caller
        .qualification()
        .approximation
        .join(callee.qualification().approximation)
        .join(call.qualification.approximation);
    let caller_root = root(caller_input.root)?.clone();
    let RebasedGuards {
        atoms,
        predicates,
        roots,
        places,
        substitutions,
        ..
    } = restated;
    let outputs = composer.map_output(root(callee_output.root)?, &output_path)?;
    // Projection items (literals and segments) are shared by every output of this call.
    let shared = std::mem::take(&mut composer.records);
    let mut results = Vec::new();
    for mapped in outputs {
        let (out_root, prefix) = match mapped {
            Mapped::Root { root, prefix } => (root, prefix),
            Mapped::Disjoint => {
                results.push(CallComposition::Disjoint);
                continue;
            }
            Mapped::Obligation(kind) => {
                results.push(CallComposition::Obligation(kind));
                continue;
            }
        };
        // A collected aggregate's element maps onto its actual only strictly below the element's slot.
        let out_path = match &prefix {
            None => output_path.clone(),
            Some(segment) => match place_composition::relation(
                &output_path,
                &AccessPath::empty().extend(segment.id()),
                &paths,
            )? {
                place_composition::PathRelation::Rest(rest) if rest == AccessPath::empty() => {
                    results.push(CallComposition::Disjoint);
                    continue;
                }
                place_composition::PathRelation::Rest(rest) => rest,
                place_composition::PathRelation::Shorter(_)
                | place_composition::PathRelation::Disjoint => {
                    results.push(CallComposition::Disjoint);
                    continue;
                }
                place_composition::PathRelation::Unknown => {
                    results.push(CallComposition::Obligation(
                        ObligationKind::AmbiguousBinding,
                    ));
                    continue;
                }
            },
        };
        let mut records = CompositionRecords {
            literals: shared.literals.clone(),
            segments: shared.segments.clone(),
            ..Default::default()
        };
        let input = place_row(&mut records, caller_root.clone(), input_path.clone());
        let output = place_row(&mut records, out_root, out_path);
        let qualification = AssertionQualification {
            context: caller_key.context,
            scope: caller.qualification().scope,
            condition: condition.id(),
            modality,
            approximation,
        };
        let key = TransferKey {
            owner: caller_key.owner,
            input: input.id(),
            output: output.id(),
            context: caller_key.context,
            scope: qualification.scope,
            modality,
            approximation,
            kind,
            call_site: Some(call.site.id()),
            provenance: ProvenanceClass::Composed,
        };
        let branch = TransferBranch::new(key.clone(), qualification.clone(), condition.clone())?;
        let alternative = branch.alternative();
        if alternative == caller.alternative() || alternative == callee.alternative() {
            results.push(CallComposition::Subsumed);
            continue;
        }
        let step = CallCompositionStep {
            composed: alternative.id(),
            caller: caller.alternative().id(),
            callee: callee.alternative().id(),
            target: call.target.id(),
            signature: bound.signature(),
            callee_declaration: callee_frame.declaration.id(),
            bindings: binding_digest(bound),
        };
        let (condition_row, nodes) = condition.records();
        records.atoms.extend(atoms.iter().cloned());
        records.predicates.extend(predicates.iter().cloned());
        records.roots.extend(roots.iter().cloned());
        records.places.extend(places.iter().cloned());
        records.substitutions.extend(substitutions.iter().cloned());
        records.conditions.push(condition_row);
        records.nodes.extend(nodes);
        records.qualification = Some(qualification);
        records.key = Some(key);
        records.alternative = Some(alternative);
        records.step = Some(step);
        results.push(CallComposition::Transfer(Box::new(ComposedTransfer {
            branch,
            records,
        })));
    }
    debug_assert!(!results.is_empty(), "every port has at least one binding");
    Ok(results)
}
fn place_row(records: &mut CompositionRecords, root: PlaceRoot, path: AccessPath) -> Place {
    let place = Place {
        root: root.id(),
        path: path.id(),
    };
    records.roots.push(root);
    records.paths.push(path);
    records.places.push(place.clone());
    place
}
/// The complete binding the composition used, encoded structurally.
fn binding_digest(bound: &BoundCall) -> ContentHash {
    let mut sink = KeySink::new("composed-bindings");
    for binding in bound.bindings() {
        binding.formal.encode(&mut sink);
        binding.source.encode(&mut sink);
        binding.kind.encode(&mut sink);
        binding.projection.encode(&mut sink);
    }
    (bound.bindings().len() as i64).encode(&mut sink);
    sink.finish()
}

/// One admitted target of a call site with the callee alternatives to compose through it; an
/// unresolved target has none.
pub struct SiteCall<'a> {
    pub frame: CallFrame<'a>,
    pub callee: Option<CalleeFrame<'a>>,
    pub branches: &'a [TransferBranch],
}
/// Compose every caller alternative through every admitted target, signature variant and callee
/// alternative of one site; alternatives stay separate results, never one merged invocation. A
/// caller value delivered into the call of an unresolved target is that target's obligation.
pub fn compose_site(
    callers: &[TransferBranch],
    calls: &[SiteCall<'_>],
    caller_frame: &CallerFrame<'_>,
    catalog: &CompositionCatalog<'_>,
) -> Result<Vec<CallComposition>, ModelError> {
    let mut results = Vec::new();
    for caller in callers {
        for call in calls {
            match (call.frame.destination, &call.callee) {
                (CallDestination::Unresolved { reason, .. }, _) => {
                    if delivers(caller, &call.frame, catalog)? {
                        results.push(CallComposition::Obligation(*reason));
                    }
                }
                (CallDestination::Callable { .. } | CallDestination::SyntheticFormatting, _) => {
                    if delivers(caller, &call.frame, catalog)? {
                        results.push(CallComposition::Obligation(
                            ObligationKind::OutsideProviderModel,
                        ));
                    }
                }
                // An override dispatch set is expanded by a later layer.
                (CallDestination::Overrides { .. }, _) => {
                    if delivers(caller, &call.frame, catalog)? {
                        results.push(CallComposition::Obligation(
                            ObligationKind::OverrideDispatch,
                        ));
                    }
                }
                (CallDestination::Resolved { .. }, Some(callee)) => {
                    for branch in call.branches {
                        results.extend(compose_call(
                            caller,
                            branch,
                            &call.frame,
                            caller_frame,
                            callee,
                            catalog,
                        )?);
                    }
                }
                (CallDestination::Resolved { .. }, None) => {
                    return Err(invalid(
                        "a resolved target composes only through its callee frame",
                    ));
                }
            }
        }
    }
    Ok(results)
}
/// The caller alternative delivers its value into one of the call's actuals or its receiver.
fn delivers(
    caller: &TransferBranch,
    call: &CallFrame<'_>,
    catalog: &CompositionCatalog<'_>,
) -> Result<bool, ModelError> {
    if call.target.receiver != call.receiver.id() {
        return Err(invalid("call frame receiver differs from its target"));
    }
    let output = catalog
        .guards
        .places
        .get(&caller.key().output)
        .ok_or_else(|| invalid("transfer place absent"))?;
    Ok(
        match catalog
            .guards
            .roots
            .get(&output.root)
            .ok_or_else(|| invalid("transfer place root absent"))?
        {
            PlaceRoot::Occurrence { occurrence } => {
                call.arguments
                    .iter()
                    .any(|argument| argument.value == *occurrence)
                    || matches!(call.receiver, Receiver::Bound { actual } if actual == occurrence)
            }
            _ => false,
        },
    )
}

fn composition_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "call_composition_frames",
        inputs: vec![
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<Predicate>(&["id"]),
            ValidationInput::of::<EvaluationAtom>(&["id"]),
            ValidationInput::of::<ConditionNode>(&["id"]),
            ValidationInput::of::<Condition>(&["id"]),
            ValidationInput::of::<PlaceRoot>(&["id"]),
            ValidationInput::of::<AccessPath>(&["id"]),
            ValidationInput::of::<Place>(&["id"]),
            ValidationInput::of::<CallSyntax>(&["id"]),
            ValidationInput::of::<CallArgument>(&["call", "ordinal"]),
            ValidationInput::of::<Receiver>(&["id"]),
            ValidationInput::of::<CallDestination>(&["id"]),
            ValidationInput::of::<CallTarget>(&["id"]),
            ValidationInput::of::<Signature>(&["id"]),
            ValidationInput::of::<SymbolDeclaration>(&["id"]),
            ValidationInput::of::<TransferKey>(&["id"]),
            ValidationInput::of::<TransferAlternative>(&["id"]),
            ValidationInput::of::<CallCompositionStep>(&["composed"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(CompositionCheck {
                charge: StateCharge::new(budget, "call_composition_frames"),
                ..Default::default()
            })
        }),
    }]
}
/// A stored composition step must be what `compose_call` derives from its premises: the caller's
/// transfer at the target's site, from the caller's input, into a caller-side root, with the
/// combined kind and approximation, no stronger modality than any premise or the target, and the
/// condition `caller ∧ callee` with each callee atom replaced by its unique restatement at the site.
#[derive(Default)]
struct CompositionCheck {
    charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    predicates: ChargedMap<Id<Predicate>, Predicate>,
    /// Restated atoms per (call site, restated callee atom).
    restatements: RestatementIndex,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    conditions: ChargedMap<Id<Condition>, Condition>,
    roots: ChargedMap<Id<PlaceRoot>, PlaceRoot>,
    paths: ChargedMap<Id<AccessPath>, AccessPath>,
    places: ChargedMap<Id<Place>, Place>,
    /// Call syntax sites, and the actual occurrences each site passes.
    syntax: ChargedMap<Id<CallSyntax>, Id<Occurrence>>,
    actuals: ChargedSet<(Id<Occurrence>, Id<Occurrence>)>,
    receivers: ChargedMap<Id<Receiver>, Receiver>,
    destinations: ChargedMap<Id<CallDestination>, CallDestination>,
    targets: ChargedMap<Id<CallTarget>, CallTarget>,
    signatures: ChargedMap<Id<Signature>, (Id<ProviderSymbol>, Id<AssertionQualification>)>,
    declarations: ChargedMap<Id<SymbolDeclaration>, Id<ProviderSymbol>>,
    keys: ChargedMap<Id<TransferKey>, TransferKey>,
    alternatives:
        ChargedMap<Id<TransferAlternative>, (Id<TransferKey>, Id<AssertionQualification>)>,
}
impl CompositionCheck {
    fn get<'a, K: Ord, V>(
        map: &'a ChargedMap<K, V>,
        key: &K,
        what: &str,
    ) -> Result<&'a V, ModelError> {
        map.get(key).ok_or_else(|| invalid(what))
    }
    fn alternative(
        &self,
        id: Id<TransferAlternative>,
    ) -> Result<(&TransferKey, &AssertionQualification), ModelError> {
        let (key, qualification) = Self::get(
            &self.alternatives,
            &id,
            "composition premise alternative absent",
        )?;
        Ok((
            Self::get(&self.keys, key, "composition transfer key absent")?,
            Self::get(
                &self.qualifications,
                qualification,
                "composition qualification absent",
            )?,
        ))
    }
    fn diagram(&self, id: Id<Condition>) -> Result<Diagram, ModelError> {
        let condition = Self::get(&self.conditions, &id, "composition condition absent")?;
        let closure = super::conditions::kernel::closure(condition.root, &self.nodes)?;
        Diagram::from_records(
            condition,
            &closure
                .into_iter()
                .map(|node| self.nodes[&node].clone())
                .collect::<Vec<_>>(),
        )
    }
    fn place(&self, id: Id<Place>) -> Result<(&PlaceRoot, &AccessPath), ModelError> {
        let place = Self::get(&self.places, &id, "composition place absent")?;
        Ok((
            Self::get(&self.roots, &place.root, "composition place root absent")?,
            Self::get(&self.paths, &place.path, "composition access path absent")?,
        ))
    }
    fn step(&self, row: &CallCompositionStep) -> Result<(), ModelError> {
        let (composed, composed_q) = self.alternative(row.composed)?;
        let (caller, caller_q) = self.alternative(row.caller)?;
        let (callee, callee_q) = self.alternative(row.callee)?;
        let target = Self::get(&self.targets, &row.target, "composition target absent")?;
        let target_q = Self::get(
            &self.qualifications,
            &target.qualification,
            "composition target qualification absent",
        )?;
        let Some(CallDestination::Resolved { symbol }) = self.destinations.get(&target.destination)
        else {
            return Err(invalid(
                "composition through an unresolved or absent destination",
            ));
        };
        let (signature_symbol, signature_q) = *Self::get(
            &self.signatures,
            &row.signature,
            "composition signature absent",
        )?;
        let declared = *Self::get(
            &self.declarations,
            &row.callee_declaration,
            "composition callee declaration absent",
        )?;
        if callee.owner != *symbol || signature_symbol != *symbol || declared != *symbol {
            return Err(invalid(
                "composition callee, signature and declaration differ from the target's symbol",
            ));
        }
        let contexts = [
            Some(caller.context),
            Some(callee.context),
            Some(target_q.context),
            self.qualifications.get(&signature_q).map(|q| q.context),
        ];
        if composed.provenance != ProvenanceClass::Composed
            || composed.call_site != Some(target.site)
            || composed.owner != caller.owner
            || contexts
                .iter()
                .any(|context| *context != Some(composed.context))
            || composed.scope != caller.scope
        {
            return Err(invalid(
                "composed transfer is not the caller's transfer at the target's site",
            ));
        }
        if composed.kind != compose_kinds(caller.kind, callee.kind)
            || composed.approximation
                != caller
                    .approximation
                    .join(callee.approximation)
                    .join(target_q.approximation)
            || composed.modality
                < caller
                    .modality
                    .weakest(callee.modality)
                    .weakest(target_q.modality)
        {
            return Err(invalid(
                "composed kind, approximation or modality does not follow its premises",
            ));
        }
        // The value enters at the caller's input, extended only through identity, and lands in a
        // caller-side root.
        let ((input_root, input_path), (caller_root, caller_path)) =
            (self.place(composed.input)?, self.place(caller.input)?);
        let extends = if caller.kind == TransferKind::Identity {
            is_prefix(caller_path, input_path)
        } else {
            caller_path == input_path
        };
        if input_root != caller_root || !extends {
            return Err(invalid("composed input is not the caller's input"));
        }
        let receiver = match self.receivers.get(&target.receiver) {
            Some(Receiver::Bound { actual }) => Some(*actual),
            _ => None,
        };
        let caller_side = match self.place(composed.output)?.0 {
            PlaceRoot::Occurrence { occurrence } => {
                *occurrence == target.site
                    || receiver == Some(*occurrence)
                    || self.actuals.contains(&(target.site, *occurrence))
            }
            PlaceRoot::Field { .. } | PlaceRoot::Global { .. } => true,
            _ => false,
        };
        if !caller_side {
            return Err(invalid(
                "composed output is not a caller-side place of the call",
            ));
        }
        let callee_condition = self.diagram(callee_q.condition)?;
        let composed_support: BTreeSet<_> = self
            .diagram(composed_q.condition)?
            .support()
            .iter()
            .copied()
            .collect();
        let mut replacements = Vec::new();
        for atom in callee_condition.support() {
            // Signature variants binding different actuals restate one guard differently; the
            // composed condition names the restatement this step used.
            let candidates = self
                .restatements
                .get(&(target.site, *atom))
                .map(Vec::as_slice)
                .unwrap_or_default();
            let used: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|id| composed_support.contains(id))
                .collect();
            match (candidates, used.as_slice()) {
                ([restated], _) | (_, [restated]) => {
                    replacements.push((*atom, Diagram::from_atom(*restated)))
                }
                _ => {
                    return Err(invalid(
                        "a callee atom has no unique restatement at the call",
                    ));
                }
            }
        }
        let replacements: Vec<_> = replacements
            .iter()
            .map(|(atom, diagram)| (*atom, diagram))
            .collect();
        let expected = callee_condition
            .substitute_atoms(&replacements)
            .and_then(|restated| {
                self.diagram(caller_q.condition)
                    .map_err(|_| super::conditions::KernelBoundary::TransferUnsupported)?
                    .and(&restated)
            })
            .map_err(|_| {
                invalid("composed condition cannot be recomputed within the condition limits")
            })?;
        if expected.id() != composed_q.condition {
            return Err(invalid(
                "composed condition is not the caller's and the callee's restated at the call",
            ));
        }
        Ok(())
    }
}
/// `prefix`'s explicit segments begin `path`, and a saturated prefix admits no extension.
fn is_prefix(prefix: &AccessPath, path: &AccessPath) -> bool {
    if prefix.unknown_suffix {
        return prefix == path;
    }
    let (p, q): (Vec<_>, Vec<_>) = (
        [prefix.first, prefix.second]
            .into_iter()
            .flatten()
            .collect(),
        [path.first, path.second].into_iter().flatten().collect(),
    );
    p.len() <= q.len() && p[..] == q[..p.len()]
}
impl InvariantCheck for CompositionCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let c = &mut self.charge;
        if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications.insert(c, row.id(), row)?;
            }
        } else if relation == Predicate::NAME {
            for row in Predicate::decode(batch)? {
                self.predicates.insert(c, row.id(), row)?;
            }
        } else if relation == EvaluationAtom::NAME {
            for row in EvaluationAtom::decode(batch)? {
                if let Some(Predicate::InvokedGuard { source } | Predicate::BoundGuard { source }) =
                    self.predicates.get(&row.predicate)
                {
                    let id = row.id();
                    self.restatements
                        .update(c, (row.evaluation, *source), |restated| restated.push(id))?;
                }
            }
        } else if relation == ConditionNode::NAME {
            for row in ConditionNode::decode(batch)? {
                self.nodes.insert(c, row.id(), row)?;
            }
        } else if relation == Condition::NAME {
            for row in Condition::decode(batch)? {
                self.conditions.insert(c, row.id(), row)?;
            }
        } else if relation == PlaceRoot::NAME {
            for row in PlaceRoot::decode(batch)? {
                self.roots.insert(c, row.id(), row)?;
            }
        } else if relation == AccessPath::NAME {
            for row in AccessPath::decode(batch)? {
                self.paths.insert(c, row.id(), row)?;
            }
        } else if relation == Place::NAME {
            for row in Place::decode(batch)? {
                self.places.insert(c, row.id(), row)?;
            }
        } else if relation == CallSyntax::NAME {
            for row in CallSyntax::decode(batch)? {
                self.syntax.insert(c, row.id(), row.site)?;
            }
        } else if relation == CallArgument::NAME {
            for row in CallArgument::decode(batch)? {
                let site = *Self::get(&self.syntax, &row.call, "composition call syntax absent")?;
                self.actuals.insert(c, (site, row.value))?;
            }
        } else if relation == Receiver::NAME {
            for row in Receiver::decode(batch)? {
                self.receivers.insert(c, row.id(), row)?;
            }
        } else if relation == CallDestination::NAME {
            for row in CallDestination::decode(batch)? {
                self.destinations.insert(c, row.id(), row)?;
            }
        } else if relation == CallTarget::NAME {
            for row in CallTarget::decode(batch)? {
                self.targets.insert(c, row.id(), row)?;
            }
        } else if relation == Signature::NAME {
            for row in Signature::decode(batch)? {
                self.signatures
                    .insert(c, row.id(), (row.symbol, row.qualification))?;
            }
        } else if relation == SymbolDeclaration::NAME {
            for row in SymbolDeclaration::decode(batch)? {
                self.declarations.insert(c, row.id(), row.symbol)?;
            }
        } else if relation == TransferKey::NAME {
            for row in TransferKey::decode(batch)? {
                self.keys.insert(c, row.id(), row)?;
            }
        } else if relation == TransferAlternative::NAME {
            for row in TransferAlternative::decode(batch)? {
                self.alternatives
                    .insert(c, row.id(), (row.transfer, row.qualification))?;
            }
        } else if relation == CallCompositionStep::NAME {
            for row in CallCompositionStep::decode(batch)? {
                self.step(&row)?;
            }
        } else {
            return Err(invalid("undeclared composition validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

type RestatementIndex = ChargedMap<(Id<Occurrence>, Id<EvaluationAtom>), Vec<Id<EvaluationAtom>>>;
