//! Whole-call transfer composition (DESIGN §15.6, core review C05/C06/C07).
//!
//! A caller transfer that delivers a value into an argument, the call's binding and a callee
//! transfer compose, matched by call site, into a `Composed` transfer owned by the caller. Roots
//! map through the binder and the parameter declaration links; paths compose only through
//! identity; the condition is the caller's AND the callee's restated at the call, where formal
//! guards need a stability witness. Anything that cannot be justified is an obligation, never an
//! unconditional flow.
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::conditions::stability::CheckedGuardBinding;
use super::derivation::RowRef;
use super::normalized::{
    binding_normalization::{
        BindingData, BindingOutput, CompositionAdmission, SourceBodySignatureClosure,
        ValidatedBoundCall, VerifiedBindings,
    },
    bindings::CallBindingAttempt,
    events::NormalizedCallEvent,
};
use super::resources::{Reservation, ResourceBudget};
use super::{
    assertion::{Approximation, AssertionQualification},
    attribution::{Modality, ObligationKind},
    calls::{
        BindingProjection, BindingSource, BoundCall, CallArgument, CallDestination, CallSyntax,
        CallTarget, ProviderSymbol, Receiver, Signature, SignatureEnumerationObservation,
        SignatureEnumerationSupport, SignatureParameter,
    },
    conditions::{
        Condition, ConditionNode, Diagram, EvaluationAtom,
        rebase::{GuardCatalog, RebasedGuards, RootBinding, substitute_call_guards},
        stability::{CheckedStability, GuardSubstitution},
    },
    declarations::{ParameterDeclaration, SymbolDeclaration},
    place_composition::{self, ComposedPaths, PathCatalog},
    source::Occurrence,
    transfer::{
        ProvenanceClass, TransferBranch, TransferDescriptor, TransferKeyRecord, TransferKind,
        compose_kinds,
        summary::{SummaryContribution, SummaryPremise, SummaryWitness},
    },
    value::{AccessPath, Literal, PathSegment, Place, PlaceRoot, Predicate},
    *,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Only replayed normalized bindings can enter the composer. The borrowed inputs are compared
/// to the private receipt and used to derive checked guard bindings; no admission booleans exist.
pub struct CallBindingFrame<'a> {
    checked: &'a ValidatedBoundCall,
    admission: &'a CompositionAdmission,
    attempt: &'a CallBindingAttempt,
    event: &'a NormalizedCallEvent,
    data: &'a BindingData,
    output: &'a BindingOutput,
}
impl<'a> CallBindingFrame<'a> {
    pub fn new(
        verified: &'a VerifiedBindings,
        attempt: Id<CallBindingAttempt>,
        data: &'a BindingData,
        output: &'a BindingOutput,
    ) -> Result<Self, ObligationKind> {
        let missing = ObligationKind::MissingEvidence;
        let checked = verified.bound(attempt).ok_or(missing)?;
        let admission = verified
            .composition(attempt)
            .ok_or(ObligationKind::NonDefiniteAlternative)?;
        let attempt = output.attempts.get(attempt).ok_or(missing)?;
        let event = data.event_events.get(checked.event()).ok_or(missing)?;
        if checked.attempt() != attempt.id()
            || admission.attempt() != attempt.id()
            || admission.event() != checked.event()
            || event.id() != attempt.event
            || checked.context() != event.context
        {
            return Err(missing);
        }
        Ok(Self {
            checked,
            admission,
            attempt,
            event,
            data,
            output,
        })
    }
    fn guards(
        &self,
        callee: &CalleeFrame<'_>,
        condition: &Diagram,
        catalog: &GuardCatalog<'_>,
    ) -> Result<BTreeMap<Id<PlaceRoot>, RootBinding>, ObligationKind> {
        let mut roots = BTreeMap::new();
        let mut needed = BTreeSet::new();
        for atom in condition.support() {
            let atom = catalog
                .atoms
                .get(atom)
                .ok_or(ObligationKind::MissingEvidence)?;
            if let Some(operand) = atom.operand {
                needed.insert(
                    catalog
                        .places
                        .get(&operand)
                        .ok_or(ObligationKind::MissingEvidence)?
                        .root,
                );
            }
        }

        for row in self
            .output
            .bindings
            .iter()
            .filter(|row| row.attempt == self.attempt.id())
        {
            let slot = self
                .data
                .callable_slots
                .get(row.slot)
                .ok_or(ObligationKind::MissingEvidence)?;
            let link = callee
                .links
                .iter()
                .find(|link| link.parameter == slot.parameter)
                .ok_or(ObligationKind::NoSourceDeclaration)?;
            let source = self
                .output
                .sources
                .get(row.source)
                .ok_or(ObligationKind::MissingEvidence)?;
            let projection = self
                .output
                .projections
                .get(row.projection)
                .ok_or(ObligationKind::MissingEvidence)?;
            let root = PlaceRoot::Formal {
                declaration: link.declaration,
            }
            .id();
            if !needed.contains(&root) {
                continue;
            }
            let binding = match source {
                BindingSource::Actual { .. } | BindingSource::ClassOf { .. } => {
                    RootBinding::Actual(CheckedGuardBinding::derive(
                        self.checked,
                        row,
                        slot,
                        source,
                        projection,
                        self.attempt,
                        self.event,
                    )?)
                }
                BindingSource::Default => RootBinding::Default,
                BindingSource::EmptyVarargs | BindingSource::EmptyKwargs => {
                    RootBinding::EmptyAggregate
                }
            };
            if roots.insert(root, binding).is_some() {
                return Err(ObligationKind::AmbiguousBinding);
            }
        }
        Ok(roots)
    }
}
/// A finite witness input, never a Summary aggregate. Nominal source membership is preserved.
pub trait CompositionOperand {
    fn descriptor(&self) -> TransferDescriptor;
    fn qualification(&self) -> &AssertionQualification;
    fn condition(&self) -> &Diagram;
    fn premise(&self) -> SummaryPremise;
}
macro_rules! operand {
    ($owner:ident,$variant:ident) => {
        impl CompositionOperand for TransferBranch<super::transfer::$owner::TransferKey> {
            fn descriptor(&self) -> TransferDescriptor {
                TransferBranch::descriptor(self)
            }
            fn qualification(&self) -> &AssertionQualification {
                TransferBranch::qualification(self)
            }
            fn condition(&self) -> &Diagram {
                TransferBranch::condition(self)
            }
            fn premise(&self) -> SummaryPremise {
                SummaryPremise::$variant {
                    alternative: self.alternative().id(),
                }
            }
        }
    };
}
operand!(local, Local);
operand!(model, Model);
#[derive(Debug, Clone)]
pub struct CompositionWitness {
    pub caller: SummaryPremise,
    pub callee: SummaryPremise,
    pub target: Id<CallTarget>,
    pub signature: Id<Signature>,
    pub callee_declaration: Id<SymbolDeclaration>,
    pub attempt: Id<CallBindingAttempt>,
    pub selected_signature_enumeration: Option<Id<SignatureEnumerationObservation>>,
    pub selected_signature_enumeration_support: Option<Id<SignatureEnumerationSupport>>,
    pub bindings: ContentHash,
}
fn selected_signature_premises(
    closure: SourceBodySignatureClosure,
) -> (
    Option<Id<SignatureEnumerationObservation>>,
    Option<Id<SignatureEnumerationSupport>>,
) {
    match closure {
        SourceBodySignatureClosure::GlobalCoverage { .. } => (None, None),
        SourceBodySignatureClosure::DeclaredEnumeration {
            enumeration,
            support,
            ..
        } => (Some(enumeration), Some(support)),
    }
}
/// One admitted call target at a site with its complete binding.
pub struct CallFrame<'a> {
    pub site: &'a Occurrence,
    pub target: &'a CallTarget,
    pub qualification: &'a AssertionQualification,
    pub destination: &'a CallDestination,
    pub receiver: &'a Receiver,
    /// The binding of the target's signature variant; `None` only for an unresolved target.
    pub binding: Option<CallBindingFrame<'a>>,
    pub arguments: &'a [CallArgument],
}
/// The caller's declaration, which must be the owner of the call site (the owner rule).
pub struct CallerFrame<'a> {
    pub declaration: &'a SymbolDeclaration,
    pub owner: Id<super::normalized::entities::EntityRef>,
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
    pub witnesses: Option<&'a BTreeMap<Id<EvaluationAtom>, CheckedStability>>,
}
/// Existing rows the composer reads; lookups verify identity.
pub struct CompositionCatalog<'a> {
    pub assumptions: assumptions::AssumptionCatalog<'a>,
    pub guards: GuardCatalog<'a>,
    pub paths: &'a BTreeMap<Id<AccessPath>, AccessPath>,
    pub segments: PathCatalog<'a>,
}
/// The rows a composition adds, including restated guards and the derivation step.
#[derive(Debug, Default)]
pub struct CompositionRecords {
    pub assumption_sets: Vec<assumptions::AssumptionSet>,
    pub assumption_members: Vec<assumptions::AssumptionSetMember>,
    pub literals: Vec<Literal>,
    pub segments: Vec<PathSegment>,
    pub paths: Vec<AccessPath>,
    pub roots: Vec<PlaceRoot>,
    pub places: Vec<Place>,
    pub atoms: Vec<EvaluationAtom>,
    pub predicates: Vec<Predicate>,
    pub substitutions: Vec<GuardSubstitution>,
    pub influences: Vec<super::transfer::summary::ControlInfluence>,
    pub conditions: Vec<Condition>,
    pub nodes: Vec<ConditionNode>,
    pub qualification: Option<AssertionQualification>,
}
#[derive(Debug)]
pub enum CallComposition {
    Transfer(Box<ComposedTransfer>),
    Disjoint,
    Subsumed,
    Obligation(ObligationKind),
}
/// Retained scalar outcomes and the outer vector stay charged through consumption.
#[derive(Debug)]
pub struct CompositionResults {
    rows: Vec<CallComposition>,
    _charge: Box<dyn Reservation>,
}
impl std::ops::Deref for CompositionResults {
    type Target = Vec<CallComposition>;
    fn deref(&self) -> &Self::Target {
        &self.rows
    }
}
impl CompositionResults {
    fn one(row: CallComposition, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let charge = budget.reserve(
            "composition_results",
            size_of::<CallComposition>() + size_of::<Self>(),
        )?;
        Ok(Self {
            rows: vec![row],
            _charge: charge,
        })
    }
    pub fn remove(&mut self, index: usize) -> CallComposition {
        self.rows.remove(index)
    }
}
pub struct CompositionIntoIter {
    rows: std::vec::IntoIter<CallComposition>,
    _charge: Box<dyn Reservation>,
}
impl Iterator for CompositionIntoIter {
    type Item = CallComposition;
    fn next(&mut self) -> Option<Self::Item> {
        self.rows.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rows.size_hint()
    }
}
impl ExactSizeIterator for CompositionIntoIter {}
impl IntoIterator for CompositionResults {
    type Item = CallComposition;
    type IntoIter = CompositionIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        CompositionIntoIter {
            rows: self.rows.into_iter(),
            _charge: self._charge,
        }
    }
}
#[derive(Debug)]
pub struct ComposedTransfer {
    pub descriptor: TransferDescriptor,
    pub qualification: AssertionQualification,
    pub condition: Diagram,
    pub witness: CompositionWitness,
    pub records: CompositionRecords,
    _charge: Box<dyn Reservation>,
    _guard_charge: Arc<super::conditions::rebase::RebaseAdmission>,
}

/// Pure nominal lowering. The sink still validates the immutable source and proof frame.
#[derive(Debug)]
pub struct SummaryEmission {
    pub key: super::transfer::summary::TransferKey,
    pub alternative: super::transfer::summary::TransferAlternative,
    pub premises: Vec<SummaryPremise>,
    pub witness: SummaryWitness,
    pub contribution: SummaryContribution,
    _charge: Box<dyn Reservation>,
}
impl ComposedTransfer {
    pub fn emit(
        &self,
        invocation: &super::analysis::summary::AnalysisInvocation,
        evidence: &[super::transfer::summary::TransferEvidence],
        budget: &ResourceBudget,
    ) -> Result<SummaryEmission, ModelError> {
        let charge = budget.reserve(
            "summary_witness_emit",
            4096usize
                .checked_add(
                    evidence
                        .len()
                        .checked_mul(128)
                        .ok_or_else(|| invalid("summary emit allowance overflow"))?,
                )
                .ok_or_else(|| invalid("summary emit allowance overflow"))?,
        )?;
        if invocation.context != self.qualification.context {
            return Err(invalid("summary emission crosses invocation context"));
        }
        let expected = BTreeSet::from([self.witness.caller.id(), self.witness.callee.id()]);
        let actual = evidence
            .iter()
            .map(|e| e.premise.id())
            .collect::<BTreeSet<_>>();
        if actual != expected || evidence.len() != actual.len() {
            return Err(invalid("summary witness evidence membership mismatch"));
        }
        let status = super::analysis::support::inferred_status(
            super::analysis::Interpretation::Structural,
            evidence
                .iter()
                .flat_map(|e| e.facts.iter().map(|f| f.status)),
        );
        let heuristic = evidence.iter().any(|e| e.facts.iter().any(|f| f.heuristic));
        let key = super::transfer::summary::TransferKey::from_descriptor(self.descriptor.clone());
        let alternative = key.alternative(&self.qualification);
        let caller = self.witness.caller.clone();
        let callee = self.witness.callee.clone();
        let witness = SummaryWitness {
            invocation: invocation.id(),
            status,
            heuristic,
            transfer: key.id(),
            qualification: self.qualification.id(),
            caller: caller.id(),
            callee: callee.id(),
            target: self.witness.target,
            signature: self.witness.signature,
            callee_declaration: self.witness.callee_declaration,
            attempt: self.witness.attempt,
            selected_signature_enumeration: self.witness.selected_signature_enumeration,
            selected_signature_enumeration_support: self
                .witness
                .selected_signature_enumeration_support,
            bindings: self.witness.bindings,
        };
        let contribution = SummaryContribution {
            alternative: alternative.id(),
            witness: witness.id(),
        };
        Ok(SummaryEmission {
            key,
            alternative,
            premises: vec![caller, callee],
            witness,
            contribution,
            _charge: charge,
        })
    }
}
fn composition_allowance(
    caller: &dyn CompositionOperand,
    callee: &dyn CompositionOperand,
    call: &CallFrame<'_>,
    catalog: &CompositionCatalog<'_>,
) -> Result<usize, ModelError> {
    let rows = call
        .arguments
        .len()
        .checked_add(caller.condition().support().len())
        .and_then(|n| n.checked_add(callee.condition().support().len()))
        .and_then(|n| {
            n.checked_add(
                call.binding
                    .as_ref()
                    .map_or(0, |frame| frame.checked.bound().bindings().len()),
            )
        })
        .ok_or_else(|| invalid("composition allowance overflow"))?;
    let maps = catalog
        .guards
        .places
        .heap_bytes()
        .checked_add(catalog.guards.roots.heap_bytes())
        .and_then(|n| n.checked_add(catalog.paths.heap_bytes()))
        .and_then(|n| n.checked_add(catalog.segments.segments.heap_bytes()))
        .and_then(|n| n.checked_add(catalog.segments.literals.heap_bytes()))
        .ok_or_else(|| invalid("composition allowance overflow"))?;
    rows.checked_mul(4096)
        .and_then(|n| n.checked_add(maps.checked_mul(4)?))
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(|| invalid("composition allowance overflow"))
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
                value: name.clone(),
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
                        BindingSource::ClassOf { .. } => {
                            Mapped::Obligation(ObligationKind::EntryValueUnknown)
                        }
                        BindingSource::Default => {
                            Mapped::Obligation(ObligationKind::DefaultUnavailable)
                        }
                        BindingSource::EmptyVarargs | BindingSource::EmptyKwargs => {
                            Mapped::Disjoint
                        }
                    })
                    .collect(),
                None => vec![Mapped::Obligation(ObligationKind::CapturedStateUnavailable)],
            },
        })
    }
}

/// Compose one caller alternative through one call into one callee alternative. Each actual bound
/// into an output root yields its own result; the result list is never empty.
pub fn compose_call(
    caller: &dyn CompositionOperand,
    callee: &dyn CompositionOperand,
    call: &CallFrame<'_>,
    caller_frame: &CallerFrame<'_>,
    callee_frame: &CalleeFrame<'_>,
    catalog: &CompositionCatalog<'_>,
    budget: &ResourceBudget,
) -> Result<CompositionResults, ModelError> {
    let (caller_key, callee_key) = (caller.descriptor(), callee.descriptor());
    let _work = budget.reserve(
        "call_composition_work",
        composition_allowance(caller, callee, call, catalog)?,
    )?;
    if caller_key.owner != caller_frame.owner {
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
            return CompositionResults::one(CallComposition::Obligation(*reason), budget);
        }
        CallDestination::Callable { .. } | CallDestination::SyntheticFormatting => {
            return CompositionResults::one(
                CallComposition::Obligation(ObligationKind::OutsideProviderModel),
                budget,
            );
        }
        CallDestination::Overrides { .. } => {
            return CompositionResults::one(
                CallComposition::Obligation(ObligationKind::OverrideDispatch),
                budget,
            );
        }
        CallDestination::Resolved { symbol }
            if *symbol != callee_frame.symbol.id()
                || callee_frame.declaration.symbol != *symbol =>
        {
            return Err(invalid("callee transfer's owner is not the call's target"));
        }
        CallDestination::Resolved { .. } => {}
    }
    let Some(binding_frame) = call.binding.as_ref() else {
        return CompositionResults::one(
            CallComposition::Obligation(ObligationKind::NonDefiniteAlternative),
            budget,
        );
    };
    let bound = binding_frame.checked.bound();
    let owner = binding_frame
        .data
        .owners
        .get(binding_frame.admission.owner())
        .ok_or_else(|| invalid("composition admitted site owner absent"))?;
    if owner.entity != binding_frame.admission.owner_entity()
        || owner.owner != binding_frame.admission.owner_declaration()
        || binding_frame.admission.callee() != callee_key.owner
        || owner.entity != caller_key.owner
        || owner.owner != caller_frame.declaration.declaration
        || owner.occurrence != call.site.id()
    {
        return Err(invalid(
            "normalized composition owners differ from transfer owners",
        ));
    }

    if bound.target() != call.target.id() || bound.site() != call.site.id() {
        return Err(invalid("call frame binding belongs to another target"));
    }
    if caller_key.context != callee_key.context || call.qualification.context != caller_key.context
    {
        return CompositionResults::one(
            CallComposition::Obligation(ObligationKind::IncompatibleContexts),
            budget,
        );
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
        return CompositionResults::one(CallComposition::Disjoint, budget);
    };
    let ports = match Ports::build(bound, callee_frame)? {
        Ok(ports) => ports,
        Err(kind) => return CompositionResults::one(CallComposition::Obligation(kind), budget),
    };
    let (bindings, entry) = match ports.port(root(callee_input.root)?)? {
        Some(Port::Entry(bindings)) => (bindings, true),
        Some(Port::Variable(bindings)) => (bindings, false),
        None => return CompositionResults::one(CallComposition::Disjoint, budget),
    };
    let Some(binding) = bindings.into_iter().find(|b| {
        b.source
            == BindingSource::Actual {
                occurrence: *actual,
            }
    }) else {
        return CompositionResults::one(CallComposition::Disjoint, budget);
    };
    // The callee reads its variable, which need not hold the caller's value any more.
    if !entry {
        return CompositionResults::one(
            CallComposition::Obligation(ObligationKind::EntryValueUnknown),
            budget,
        );
    }
    let guard_bindings =
        match binding_frame.guards(callee_frame, callee.condition(), &catalog.guards) {
            Ok(bindings) => bindings,
            Err(reason) => {
                return CompositionResults::one(CallComposition::Obligation(reason), budget);
            }
        };
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
        ComposedPaths::Disjoint => {
            return CompositionResults::one(CallComposition::Disjoint, budget);
        }
        ComposedPaths::Obligation(kind) => {
            return CompositionResults::one(CallComposition::Obligation(kind), budget);
        }
    };
    // The callee condition restated at the call; unjustified formal guards refuse the transfer.
    let restated: RebasedGuards = match substitute_call_guards(
        callee.condition(),
        call.site,
        caller_key.context,
        &catalog.guards,
        &guard_bindings,
        callee_frame.witnesses,
        Some(caller.qualification()),
        budget,
    ) {
        Ok(restated) => restated,
        Err(kind) => return CompositionResults::one(CallComposition::Obligation(kind), budget),
    };
    let admitted = match caller.condition().admitted_binary(
        &restated.condition,
        super::conditions::kernel::BooleanOperation::Conjunction,
        budget,
    ) {
        Ok(condition) => condition,
        Err(super::conditions::kernel::DiagramAdmissionError::Resource(error)) => {
            return Err(error);
        }
        Err(super::conditions::kernel::DiagramAdmissionError::Boundary(boundary)) => {
            return CompositionResults::one(
                CallComposition::Obligation(super::obligation::from_kernel(boundary)),
                budget,
            );
        }
    };
    let (condition, _condition_charge) = admitted.into_parts();
    let modality = Modality::Definite
        .weakest(call.qualification.modality)
        .weakest(caller.qualification().modality)
        .weakest(callee.qualification().modality);
    let approximation = caller
        .qualification()
        .approximation
        .join(callee.qualification().approximation)
        .join(call.qualification.approximation);
    let _basis_scratch = budget.reserve(
        "composition_assumption_union",
        assumptions::MAX_ASSUMPTIONS
            .saturating_mul(4 * size_of::<assumptions::AssumptionSetMember>()),
    )?;
    let basis_ids = BTreeSet::from([
        call.qualification.assumptions,
        caller.qualification().assumptions,
        callee.qualification().assumptions,
    ]);
    let basis = if basis_ids.len() > 1 {
        use assumptions::AssumptionResolver;
        let resolved = basis_ids
            .into_iter()
            .map(|id| catalog.assumptions.resolve(id))
            .collect::<Result<Vec<_>, _>>()?;
        Some(assumptions::ResolvedAssumptions::union(&resolved)?)
    } else {
        None
    };
    let basis_id = basis
        .as_ref()
        .map_or(call.qualification.assumptions, |b| b.set.id());
    let final_q = AssertionQualification {
        assumptions: basis_id,
        context: caller_key.context,
        scope: caller.qualification().scope,
        condition: condition.id(),
        modality,
        approximation,
    };
    let mut restated = match substitute_call_guards(
        callee.condition(),
        call.site,
        caller_key.context,
        &catalog.guards,
        &guard_bindings,
        callee_frame.witnesses,
        Some(&final_q),
        budget,
    ) {
        Ok(rows) => rows,
        Err(reason) => return CompositionResults::one(CallComposition::Obligation(reason), budget),
    };
    let guard_charge = Arc::new(restated.take_admission());
    let caller_root = root(caller_input.root)?.clone();
    let RebasedGuards {
        atoms,
        predicates,
        roots,
        places,
        substitutions,
        influences,
        ..
    } = restated;
    let outputs = composer.map_output(root(callee_output.root)?, &output_path)?;
    // Projection items (literals and segments) are shared by every output of this call.
    let shared = std::mem::take(&mut composer.records);
    let result_charge = budget.reserve(
        "composition_results",
        outputs
            .len()
            .checked_mul(size_of::<CallComposition>())
            .and_then(|n| n.checked_add(size_of::<CompositionResults>()))
            .ok_or_else(|| invalid("composition result allowance overflow"))?,
    )?;
    let mut results = Vec::with_capacity(outputs.len());
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
        if let Some(basis) = &basis {
            records.assumption_sets.push(basis.set.clone());
            records
                .assumption_members
                .extend(basis.members.iter().cloned());
        }
        let input = place_row(&mut records, caller_root.clone(), input_path.clone());
        let output = place_row(&mut records, out_root, out_path);
        let qualification = AssertionQualification {
            assumptions: basis_id,
            context: caller_key.context,
            scope: caller.qualification().scope,
            condition: condition.id(),
            modality,
            approximation,
        };
        let descriptor = TransferDescriptor {
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
        let (selected_signature_enumeration, selected_signature_enumeration_support) =
            selected_signature_premises(binding_frame.admission.signature_closure());
        let witness = CompositionWitness {
            caller: caller.premise(),
            callee: callee.premise(),
            target: call.target.id(),
            signature: bound.signature(),
            callee_declaration: callee_frame.declaration.id(),
            attempt: binding_frame.checked.attempt(),
            selected_signature_enumeration,
            selected_signature_enumeration_support,
            bindings: binding_digest(bound),
        };
        let retained = budget.reserve(
            "call_composition_output",
            composition_allowance(caller, callee, call, catalog)?
                .saturating_add(condition.allocation_allowance())
                .saturating_add(basis.as_ref().map_or(0, |b| {
                    size_of::<assumptions::AssumptionSet>() + b.heap_bytes()
                })),
        )?;
        let (condition_row, nodes) = condition.records();
        records.atoms.extend(atoms.iter().cloned());
        records.predicates.extend(predicates.iter().cloned());
        records.roots.extend(roots.iter().cloned());
        records.places.extend(places.iter().cloned());
        records.substitutions.extend(substitutions.iter().cloned());
        records.influences.extend(influences.iter().map(|row| {
            super::transfer::summary::ControlInfluence {
                qualification: row.qualification,
                input: row.input,
                atom: row.atom,
                evaluation: row.evaluation,
            }
        }));
        records.conditions.push(condition_row);
        records.nodes.extend(nodes);
        records.qualification = Some(qualification);
        results.push(CallComposition::Transfer(Box::new(ComposedTransfer {
            descriptor,
            qualification: final_q.clone(),
            condition: condition.clone(),
            witness,
            records,
            _charge: retained,
            _guard_charge: guard_charge.clone(),
        })));
    }
    debug_assert!(!results.is_empty(), "every port has at least one binding");
    Ok(CompositionResults {
        rows: results,
        _charge: result_charge,
    })
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
    budget: &ResourceBudget,
) -> Result<CompositionResults, ModelError> {
    let pairs = calls
        .iter()
        .try_fold(0usize, |n, call| n.checked_add(call.branches.len().max(1)))
        .and_then(|n| n.checked_mul(callers.len()))
        .ok_or_else(|| invalid("composition pair allowance overflow"))?;
    if pairs > 100_000 {
        return CompositionResults::one(
            CallComposition::Obligation(ObligationKind::SummaryPairWorkLimit),
            budget,
        );
    }
    let rows = calls
        .iter()
        .try_fold(0usize, |n, call| {
            n.checked_add(
                call.branches.len().max(1).checked_mul(
                    call.frame
                        .binding
                        .as_ref()
                        .map_or(1, |b| b.checked.bound().bindings().len().max(1)),
                )?,
            )
        })
        .and_then(|n| n.checked_mul(callers.len()))
        .ok_or_else(|| invalid("composition result allowance overflow"))?;
    let result_charge = budget.reserve(
        "site_composition_results",
        rows.checked_mul(size_of::<CallComposition>())
            .and_then(|n| n.checked_add(size_of::<CompositionResults>()))
            .ok_or_else(|| invalid("composition result allowance overflow"))?,
    )?;
    let mut results = Vec::with_capacity(rows);
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
                            budget,
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
    Ok(CompositionResults {
        rows: results,
        _charge: result_charge,
    })
}
/// The caller alternative delivers its value into one of the call's actuals or its receiver.
fn delivers(
    caller: &dyn CompositionOperand,
    call: &CallFrame<'_>,
    catalog: &CompositionCatalog<'_>,
) -> Result<bool, ModelError> {
    if call.target.receiver != call.receiver.id() {
        return Err(invalid("call frame receiver differs from its target"));
    }
    let output = catalog
        .guards
        .places
        .get(&caller.descriptor().output)
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

pub(crate) fn composition_invariants() -> Vec<Invariant> {
    vec![Invariant {
        name: "call_composition_frames",
        inputs: {
            let mut inputs = vec![
                ValidationInput::of::<AssertionQualification>(&["id"]),
                ValidationInput::of::<Predicate>(&["id"]),
                ValidationInput::of::<EvaluationAtom>(&["id"]),
                ValidationInput::of::<ConditionNode>(&["id"]),
                ValidationInput::of::<Condition>(&["id"]),
                ValidationInput::of::<PlaceRoot>(&["id"]),
                ValidationInput::of::<AccessPath>(&["id"]),
                ValidationInput::of::<Place>(&["id"]),
                ValidationInput::of::<PathSegment>(&["id"]),
                ValidationInput::of::<Literal>(&["id"]),
                ValidationInput::of::<CallSyntax>(&["id"]),
                ValidationInput::of::<CallArgument>(&["call", "ordinal"]),
                ValidationInput::of::<Receiver>(&["id"]),
                ValidationInput::of::<CallDestination>(&["id"]),
                ValidationInput::of::<CallTarget>(&["id"]),
                ValidationInput::of::<Signature>(&["id"]),
                ValidationInput::of::<SymbolDeclaration>(&["id"]),
                ValidationInput::of::<super::transfer::local::TransferKey>(&["id"]),
                ValidationInput::of::<super::transfer::model::TransferKey>(&["id"]),
                ValidationInput::of::<super::transfer::summary::TransferKey>(&["id"]),
                ValidationInput::of::<super::transfer::local::TransferAlternative>(&["id"]),
                ValidationInput::of::<super::transfer::model::TransferAlternative>(&["id"]),
                ValidationInput::of::<super::transfer::summary::TransferAlternative>(&["id"]),
                ValidationInput::of::<SummaryPremise>(&["id"]),
                ValidationInput::of::<SummaryWitness>(&["id"]),
                ValidationInput::of::<super::execution::summary_path::SummaryPathWitness>(&["id"]),
                ValidationInput::of::<super::execution::summary_capture::SummaryCaptureWitness>(&[
                    "id",
                ]),
                ValidationInput::of::<SummaryContribution>(&["id"]),
                ValidationInput::of::<super::transfer::local::TransferSupport>(&["id"]),
                ValidationInput::of::<super::transfer::model::TransferSupport>(&["id"]),
                ValidationInput::of::<super::analysis::summary::AnalysisInvocation>(&["id"]),
                ValidationInput::of::<super::analysis::AnalysisDefinition>(&["id"]),
            ];
            for input in BindingData::validation_inputs().into_iter().chain(BindingOutput::validation_inputs()).chain(super::analysis::local::support::EvidenceIndex::inputs()).chain(super::analysis::model::support::EvidenceIndex::inputs()).chain(<super::analysis::local::SupportSource as assertion::DerivedSupportSource>::inputs()).chain(<super::analysis::model::SupportSource as assertion::DerivedSupportSource>::inputs()).chain(super::ownership::ScopeIndex::inputs()) {if !inputs.iter().any(|old|old.name()==input.name() && old.prefix()==input.prefix()) {inputs.push(input);}}
            inputs
        },
        create: std::sync::Arc::new(|budget| Box::new(CompositionCheck::new(budget))),
    }]
}
/// A stored composition step must be what `compose_call` derives from its premises: the caller's
/// transfer at the target's site, from the caller's input, into a caller-side root, with the
/// combined kind and approximation, no stronger modality than any premise or the target, and the
/// condition `caller ∧ callee` with each callee atom replaced by its unique restatement at the site.
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
    segments: ChargedMap<Id<PathSegment>, PathSegment>,
    literals: ChargedMap<Id<Literal>, Literal>,
    places: ChargedMap<Id<Place>, Place>,
    /// Call syntax sites, and the actual occurrences each site passes.
    syntax: ChargedMap<Id<CallSyntax>, Id<Occurrence>>,
    actuals: ChargedSet<(Id<Occurrence>, Id<Occurrence>)>,
    receivers: ChargedMap<Id<Receiver>, Receiver>,
    destinations: ChargedMap<Id<CallDestination>, CallDestination>,
    targets: ChargedMap<Id<CallTarget>, CallTarget>,
    signatures: ChargedMap<Id<Signature>, (Id<ProviderSymbol>, Id<AssertionQualification>)>,
    declarations: ChargedMap<Id<SymbolDeclaration>, Id<ProviderSymbol>>,
    keys: ChargedMap<RowRef, TransferDescriptor>,
    alternatives: ChargedMap<RowRef, (RowRef, Id<AssertionQualification>)>,
    premises: ChargedMap<Id<SummaryPremise>, SummaryPremise>,
    witnesses: ChargedMap<Id<SummaryWitness>, SummaryWitness>,
    path_witnesses: ChargedMap<
        Id<super::execution::summary_path::SummaryPathWitness>,
        super::execution::summary_path::SummaryPathWitness,
    >,
    capture_witnesses: ChargedMap<
        Id<super::execution::summary_capture::SummaryCaptureWitness>,
        super::execution::summary_capture::SummaryCaptureWitness,
    >,
    contributions: super::charged::ChargedVec<SummaryContribution>,
    binding_data: BindingData,
    binding_output: BindingOutput,
    budget: ResourceBudget,
    local_evidence: super::analysis::local::support::EvidenceIndex,
    local_frames: Box<dyn assertion::DerivedSupportIndex<super::analysis::local::SupportSource>>,
    model_frames: Box<dyn assertion::DerivedSupportIndex<super::analysis::model::SupportSource>>,
    model_evidence: super::analysis::model::support::EvidenceIndex,
    local_supports: ChargedMap<
        Id<super::transfer::local::TransferAlternative>,
        Vec<Id<super::analysis::local::SupportSource>>,
    >,
    model_supports: ChargedMap<
        Id<super::transfer::model::TransferAlternative>,
        Vec<Id<super::analysis::model::SupportSource>>,
    >,
    invocations: ChargedMap<
        Id<super::analysis::summary::AnalysisInvocation>,
        super::analysis::summary::AnalysisInvocation,
    >,
    definitions:
        ChargedMap<Id<super::analysis::AnalysisDefinition>, super::analysis::AnalysisDefinition>,
    ownership: super::ownership::ScopeIndex,
}
impl CompositionCheck {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            charge: StateCharge::new(budget, "call_composition_frames"),
            qualifications: Default::default(),
            predicates: Default::default(),
            restatements: Default::default(),
            nodes: Default::default(),
            conditions: Default::default(),
            roots: Default::default(),
            paths: Default::default(),
            segments: Default::default(),
            literals: Default::default(),
            places: Default::default(),
            syntax: Default::default(),
            actuals: Default::default(),
            receivers: Default::default(),
            destinations: Default::default(),
            targets: Default::default(),
            signatures: Default::default(),
            declarations: Default::default(),
            keys: Default::default(),
            alternatives: Default::default(),
            premises: Default::default(),
            witnesses: Default::default(),
            path_witnesses: Default::default(),
            capture_witnesses: Default::default(),
            contributions: Default::default(),
            binding_data: BindingData::new(budget),
            binding_output: BindingOutput::new(budget),
            budget: budget.clone(),
            local_evidence: super::analysis::local::support::EvidenceIndex::new(budget),
            local_frames:
                <super::analysis::local::SupportSource as assertion::DerivedSupportSource>::index(
                    budget,
                ),
            model_frames:
                <super::analysis::model::SupportSource as assertion::DerivedSupportSource>::index(
                    budget,
                ),
            model_evidence: super::analysis::model::support::EvidenceIndex::new(budget),
            local_supports: Default::default(),
            model_supports: Default::default(),
            invocations: Default::default(),
            definitions: Default::default(),
            ownership: super::ownership::ScopeIndex::new(budget, "summary_witness_scope"),
        }
    }
    fn premise_facts(
        &self,
        id: Id<SummaryPremise>,
        invocation: &super::analysis::summary::AnalysisInvocation,
    ) -> Result<Vec<super::analysis::support::SourceFacts>, ModelError> {
        match Self::get(&self.premises, &id, "summary premise absent")? {
            SummaryPremise::Local { alternative } => Self::get(
                &self.local_supports,
                alternative,
                "summary local premise has no support",
            )?
            .iter()
            .map(|source| {
                let frame = self.local_frames.frame(*source)?;
                if frame.input != invocation.input || frame.context != invocation.context {
                    return Err(invalid(
                        "summary local premise crosses invocation input/context",
                    ));
                }
                self.local_evidence.get(*source).map(|(_, facts)| facts)
            })
            .collect(),
            SummaryPremise::Model { alternative } => Self::get(
                &self.model_supports,
                alternative,
                "summary model premise has no support",
            )?
            .iter()
            .map(|source| {
                let frame = self.model_frames.frame(*source)?;
                if frame.input != invocation.input || frame.context != invocation.context {
                    return Err(invalid(
                        "summary model premise crosses invocation input/context",
                    ));
                }
                self.model_evidence.get(*source).map(|(_, facts)| facts)
            })
            .collect(),
            SummaryPremise::Captured { witness } => {
                let row = Self::get(&self.capture_witnesses, witness, "capture witness absent")?;
                let parent = Self::get(
                    &self.invocations,
                    &row.invocation,
                    "capture summary invocation absent",
                )?;
                if (parent.input, parent.context) != (invocation.input, invocation.context) {
                    return Err(invalid("capture premise crosses invocation frame"));
                }
                Ok(vec![
                    super::analysis::support::DerivedEvidence::source_facts(row),
                ])
            }
            SummaryPremise::Path { witness } => {
                let row = Self::get(
                    &self.path_witnesses,
                    witness,
                    "earlier summary path witness absent",
                )?;
                let previous = Self::get(
                    &self.invocations,
                    &row.invocation,
                    "earlier summary invocation absent",
                )?;
                if (previous.input, previous.context) != (invocation.input, invocation.context) {
                    return Err(invalid("summary path premise crosses invocation frame"));
                }
                Ok(vec![
                    super::analysis::support::DerivedEvidence::source_facts(row),
                ])
            }
            SummaryPremise::Witness { witness } => {
                let row = Self::get(&self.witnesses, witness, "earlier summary witness absent")?;
                let previous = Self::get(
                    &self.invocations,
                    &row.invocation,
                    "earlier summary invocation absent",
                )?;
                if (previous.input, previous.context) != (invocation.input, invocation.context) {
                    return Err(invalid(
                        "earlier summary witness crosses invocation input/context",
                    ));
                }
                Ok(vec![
                    super::analysis::support::DerivedEvidence::source_facts(row),
                ])
            }
        }
    }
    fn witness_frame(
        &self,
        row: &SummaryWitness,
    ) -> Result<(&TransferDescriptor, &AssertionQualification), ModelError> {
        Ok((
            Self::get(
                &self.keys,
                &RowRef::of(row.transfer),
                "summary witness key absent",
            )?,
            Self::get(
                &self.qualifications,
                &row.qualification,
                "summary witness qualification absent",
            )?,
        ))
    }
    fn premise(
        &self,
        id: Id<SummaryPremise>,
    ) -> Result<(&TransferDescriptor, &AssertionQualification), ModelError> {
        let source = Self::get(&self.premises, &id, "summary premise absent")?;
        if let SummaryPremise::Witness { witness } = source {
            return self.witness_frame(Self::get(
                &self.witnesses,
                witness,
                "earlier summary witness absent",
            )?);
        }
        if let SummaryPremise::Captured { witness } = source {
            let row = Self::get(&self.capture_witnesses, witness, "capture witness absent")?;
            return Ok((
                Self::get(
                    &self.keys,
                    &RowRef::of(row.transfer),
                    "capture transfer absent",
                )?,
                Self::get(
                    &self.qualifications,
                    &row.qualification,
                    "capture qualification absent",
                )?,
            ));
        }
        if let SummaryPremise::Path { witness } = source {
            let row = Self::get(&self.path_witnesses, witness, "earlier path proof absent")?;
            return Ok((
                Self::get(
                    &self.keys,
                    &RowRef::of(row.transfer),
                    "path proof transfer absent",
                )?,
                Self::get(
                    &self.qualifications,
                    &row.qualification,
                    "path proof qualification absent",
                )?,
            ));
        }
        self.alternative(source.reference())
    }

    fn get<'a, K: Ord, V>(
        map: &'a ChargedMap<K, V>,
        key: &K,
        what: &str,
    ) -> Result<&'a V, ModelError> {
        map.get(key).ok_or_else(|| invalid(what))
    }
    fn alternative(
        &self,
        id: RowRef,
    ) -> Result<(&TransferDescriptor, &AssertionQualification), ModelError> {
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
    fn step(&self, row: &SummaryWitness, verified: &VerifiedBindings) -> Result<(), ModelError> {
        let (composed, composed_q) = self.witness_frame(row)?;
        let (caller, caller_q) = self.premise(row.caller)?;
        let (callee, callee_q) = self.premise(row.callee)?;
        let invocation = Self::get(
            &self.invocations,
            &row.invocation,
            "summary witness invocation absent",
        )?;
        let definition = Self::get(
            &self.definitions,
            &invocation.definition,
            "summary witness definition absent",
        )?;
        if definition.method != super::analysis::AnalysisMethod::Summaries
            || definition.interpretation == super::analysis::Interpretation::Heuristic
            || invocation.context != composed.context
            || !self
                .ownership
                .owns_scope(invocation.input, self.ownership.scope(composed.scope)?)?
        {
            return Err(invalid("summary witness changes its invocation frame"));
        }
        let _evidence_charge = self.budget.reserve(
            "summary_witness_evidence",
            (self.local_supports.values().map(Vec::len).sum::<usize>()
                + self.model_supports.values().map(Vec::len).sum::<usize>()
                + 2)
            .saturating_mul(size_of::<super::analysis::support::SourceFacts>() + 64),
        )?;
        let caller_facts = self.premise_facts(row.caller, invocation)?;
        let callee_facts = self.premise_facts(row.callee, invocation)?;
        if caller_facts
            .iter()
            .any(|fact| fact.qualification != caller_q.id())
            || callee_facts
                .iter()
                .any(|fact| fact.qualification != callee_q.id())
        {
            return Err(invalid(
                "summary witness support changes premise qualification",
            ));
        }
        let facts = caller_facts.iter().chain(&callee_facts);
        let expected_status = super::analysis::support::inferred_status(
            super::analysis::Interpretation::Structural,
            facts.clone().map(|fact| fact.status),
        );
        if row.status != expected_status
            || row.heuristic != facts.clone().any(|fact| fact.heuristic)
        {
            return Err(invalid("summary witness strengthens evidence lineage"));
        }

        let binding = CallBindingFrame::new(
            verified,
            row.attempt,
            &self.binding_data,
            &self.binding_output,
        )
        .map_err(|reason| invalid(&format!("composition lacks checked admission: {reason:?}")))?;
        if binding.admission.target() != row.target
            || binding_digest(binding.checked.bound()) != row.bindings
            || binding.checked.bound().signature() != row.signature
            || binding.admission.callee() != callee.owner
            || selected_signature_premises(binding.admission.signature_closure())
                != (
                    row.selected_signature_enumeration,
                    row.selected_signature_enumeration_support,
                )
        {
            return Err(invalid(
                "summary witness does not use its exact normalized binding",
            ));
        }
        let owner = self
            .binding_data
            .owners
            .get(binding.admission.owner())
            .ok_or_else(|| invalid("composition owner absent"))?;
        if owner.entity != binding.admission.owner_entity()
            || owner.owner != binding.admission.owner_declaration()
            || owner.entity != caller.owner
        {
            return Err(invalid(
                "composition caller differs from normalized site owner",
            ));
        }
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
        if signature_symbol != *symbol || declared != *symbol {
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
                != caller
                    .modality
                    .weakest(callee.modality)
                    .weakest(target_q.modality)
        {
            return Err(invalid(
                "composed kind, approximation or modality does not follow its premises",
            ));
        }
        // Replay the shared port/path operation, not only root locality. A caller-side field with
        // a forged path must not become evidence for a different transfer proposition.
        let row_bytes = self
            .binding_data
            .parameters
            .iter()
            .try_fold(0usize, |n, row| {
                n.checked_add(size_of::<SignatureParameter>() + row.heap_bytes() + 128)
            })
            .and_then(|n| {
                n.checked_add(
                    self.binding_data
                        .parameter_declarations
                        .len()
                        .checked_mul(size_of::<ParameterDeclaration>() + 128)?,
                )
            })
            .ok_or_else(|| invalid("composition path allowance overflow"))?;
        let map_bytes = self
            .segments
            .values()
            .try_fold(row_bytes, |n, row| {
                n.checked_add(size_of::<PathSegment>() + row.heap_bytes() + 128)
            })
            .and_then(|n| {
                self.literals.values().try_fold(n, |n, row| {
                    n.checked_add(size_of::<Literal>() + row.heap_bytes() + 128)
                })
            })
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(self.places.len().checked_mul(2048)?))
            .and_then(|n| n.checked_add(8192))
            .ok_or_else(|| invalid("composition path allowance overflow"))?;
        let _path_charge = self.budget.reserve("composition_path_replay", map_bytes)?;
        let bound = binding.checked.bound();
        let declaration = self
            .binding_data
            .entity_declarations
            .get(row.callee_declaration)
            .ok_or_else(|| invalid("composition source declaration absent"))?;
        let symbol = self
            .binding_data
            .symbols
            .get(declaration.symbol)
            .ok_or_else(|| invalid("composition callee symbol absent"))?;
        let mut parameters = self
            .binding_data
            .parameters
            .iter()
            .filter(|p| p.signature == bound.signature())
            .cloned()
            .collect::<Vec<_>>();
        parameters.sort_by_key(|p| p.ordinal);
        let links = self
            .binding_data
            .parameter_declarations
            .iter()
            .filter(|p| parameters.iter().any(|m| m.id() == p.parameter))
            .cloned()
            .collect::<Vec<_>>();
        let frame = CalleeFrame {
            symbol,
            declaration,
            parameters: &parameters,
            links: &links,
            witnesses: None,
        };
        let ports = Ports::build(bound, &frame)?
            .map_err(|reason| invalid(&format!("composition port replay refused: {reason:?}")))?;
        let (delivered_root, delivered_path) = self.place(caller.output)?;
        let PlaceRoot::Occurrence { occurrence: actual } = delivered_root else {
            return Err(invalid(
                "composition caller does not deliver into this call",
            ));
        };
        let (callee_root, callee_path) = self.place(callee.input)?;
        let Some(Port::Entry(bindings)) = ports.port(callee_root)? else {
            return Err(invalid("composition callee input is not an entry port"));
        };
        let source_binding = bindings
            .iter()
            .find(|b| {
                b.source
                    == BindingSource::Actual {
                        occurrence: *actual,
                    }
            })
            .ok_or_else(|| invalid("composition value enters a different binding"))?;
        let site = self
            .binding_data
            .occurrences
            .get(target.site)
            .ok_or_else(|| invalid("composition site absent"))?;
        let mut composer = Composer {
            site,
            ports,
            records: Default::default(),
        };
        let mut delivered = AccessPath::empty();
        if let Some(segment) = composer.projection(&source_binding.projection) {
            delivered = delivered.extend(segment.id());
        }
        delivered = delivered.append(delivered_path);
        let mut segments = self
            .segments
            .iter()
            .map(|(id, p)| (*id, p.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut literals = self
            .literals
            .iter()
            .map(|(id, p)| (*id, p.clone()))
            .collect::<BTreeMap<_, _>>();
        for segment in &composer.records.segments {
            segments.insert(segment.id(), segment.clone());
        }
        for literal in &composer.records.literals {
            literals.insert(literal.id(), literal.clone());
        }
        let path_catalog = PathCatalog {
            segments: &segments,
            literals: &literals,
        };
        let (caller_input_root, caller_input_path) = self.place(caller.input)?;
        let (callee_output_root, callee_output_path) = self.place(callee.output)?;
        let ComposedPaths::Flow {
            input: expected_input,
            output: expected_output,
            kind: expected_kind,
        } = place_composition::compose(
            caller_input_path,
            &delivered,
            callee_path,
            callee_output_path,
            caller.kind,
            callee.kind,
            &path_catalog,
        )?
        else {
            return Err(invalid("composition path replay has no transfer"));
        };
        let mut matching_output = false;
        for mapped in composer.map_output(callee_output_root, &expected_output)? {
            if let Mapped::Root { root, prefix } = mapped {
                let output = match prefix {
                    None => expected_output.clone(),
                    Some(segment) => match place_composition::relation(
                        &expected_output,
                        &AccessPath::empty().extend(segment.id()),
                        &path_catalog,
                    )? {
                        place_composition::PathRelation::Rest(rest)
                            if rest != AccessPath::empty() =>
                        {
                            rest
                        }
                        _ => continue,
                    },
                };
                if self.place(composed.output)? == (&root, &output) {
                    matching_output = true;
                }
            }
        }
        if self.place(composed.input)? != (caller_input_root, &expected_input)
            || !matching_output
            || composed.kind != expected_kind
        {
            return Err(invalid(
                "composed ports or paths differ from normalized replay",
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
        let _replay = self.budget.reserve(
            "composition_replay",
            self.nodes
                .len()
                .checked_mul(1024)
                .and_then(|n| n.checked_add(4096))
                .ok_or_else(|| invalid("composition replay allowance overflow"))?,
        )?;
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
        let restated = callee_condition
            .admitted_substitution(&replacements, &self.budget)
            .map_err(|error| {
                invalid(&format!("composed condition substitution refused: {error}"))
            })?;
        let expected = self
            .diagram(caller_q.condition)?
            .admitted_binary(
                &restated,
                super::conditions::kernel::BooleanOperation::Conjunction,
                &self.budget,
            )
            .map_err(|error| invalid(&format!("composed condition replay refused: {error}")))?;
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
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if input.prefix() == Some(super::stages::PublicationBoundary::Facts) {
            if !self.binding_data.visit(input.name(), batch)? {
                return Err(invalid("undeclared early composition binding input"));
            }
            return Ok(());
        }
        self.visit(input.name(), batch)
    }
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        let accepted = (!super::stages::is_vocabulary(relation)
            && self.binding_data.visit(relation, batch)?)
            | self.binding_output.visit(relation, batch)?
            | self.local_evidence.visit(relation, batch)?
            | self.model_evidence.visit(relation, batch)?
            | self.ownership.visit(relation, batch)?
            | self.local_frames.visit(relation, batch)?
            | self.model_frames.visit(relation, batch)?;
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
        } else if relation == PathSegment::NAME {
            for row in PathSegment::decode(batch)? {
                self.segments.insert(c, row.id(), row)?;
            }
        } else if relation == Literal::NAME {
            for row in Literal::decode(batch)? {
                self.literals.insert(c, row.id(), row)?;
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
        } else if let Some(rows) = super::transfer::subject_rows(relation, batch)? {
            for (id, row) in rows {
                self.keys.insert(c, id, row)?;
            }
        } else if relation == super::transfer::local::TransferAlternative::NAME {
            for row in super::transfer::local::TransferAlternative::decode(batch)? {
                self.alternatives.insert(
                    c,
                    RowRef::of(row.id()),
                    (RowRef::of(row.transfer), row.qualification),
                )?;
            }
        } else if relation == super::transfer::model::TransferAlternative::NAME {
            for row in super::transfer::model::TransferAlternative::decode(batch)? {
                self.alternatives.insert(
                    c,
                    RowRef::of(row.id()),
                    (RowRef::of(row.transfer), row.qualification),
                )?;
            }
        } else if relation == super::transfer::summary::TransferAlternative::NAME {
            for row in super::transfer::summary::TransferAlternative::decode(batch)? {
                self.alternatives.insert(
                    c,
                    RowRef::of(row.id()),
                    (RowRef::of(row.transfer), row.qualification),
                )?;
            }
        } else if relation == super::transfer::local::TransferSupport::NAME {
            for row in super::transfer::local::TransferSupport::decode(batch)? {
                self.local_supports
                    .update(c, row.assertion, |sources| sources.push(row.source))?;
            }
        } else if relation == super::transfer::model::TransferSupport::NAME {
            for row in super::transfer::model::TransferSupport::decode(batch)? {
                self.model_supports
                    .update(c, row.assertion, |sources| sources.push(row.source))?;
            }
        } else if relation == super::analysis::summary::AnalysisInvocation::NAME {
            for row in super::analysis::summary::AnalysisInvocation::decode(batch)? {
                self.invocations.insert(c, row.id(), row)?;
            }
        } else if relation == super::analysis::AnalysisDefinition::NAME {
            for row in super::analysis::AnalysisDefinition::decode(batch)? {
                self.definitions.insert(c, row.id(), row)?;
            }
        } else if relation == SummaryPremise::NAME {
            for row in SummaryPremise::decode(batch)? {
                self.premises.insert(c, row.id(), row)?;
            }
        } else if relation == super::execution::summary_path::SummaryPathWitness::NAME {
            for row in super::execution::summary_path::SummaryPathWitness::decode(batch)? {
                self.path_witnesses
                    .insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == super::execution::summary_capture::SummaryCaptureWitness::NAME {
            for row in super::execution::summary_capture::SummaryCaptureWitness::decode(batch)? {
                self.capture_witnesses.insert(c, row.id(), row)?;
            }
        } else if relation == SummaryWitness::NAME {
            for row in SummaryWitness::decode(batch)? {
                self.witnesses.insert(c, row.id(), row)?;
            }
        } else if relation == SummaryContribution::NAME {
            for row in SummaryContribution::decode(batch)? {
                self.contributions.push(c, row)?;
            }
        } else if !accepted {
            return Err(invalid("undeclared composition validation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.witnesses.is_empty() && self.contributions.is_empty() {
            return Ok(());
        }
        let verified = super::normalized::binding_normalization::verify(
            &self.binding_data,
            &self.binding_output,
            &self.budget,
        )?;
        for row in self.witnesses.values() {
            self.step(row, &verified)?;
        }
        for row in self.contributions.iter() {
            let witness = Self::get(
                &self.witnesses,
                &row.witness,
                "summary contribution witness absent",
            )?;
            let (key, q) = self.alternative(RowRef::of(row.alternative))?;
            let (wk, wq) = self.witness_frame(witness)?;
            if key != wk {
                return Err(invalid("summary contribution changes its witness transfer"));
            }
            key.check(q)?;
            wk.check(wq)?;
            // A finite member contributes to an OR aggregate; its condition need not equal
            // that aggregate. The shared derivation validator and whole Summary replay
            // separately check the exact union and its complete witness membership.
            let _condition_memory = self.budget.reserve(
                "summary-contribution-condition",
                self.nodes.len().saturating_mul(192).saturating_add(4096),
            )?;
            let member = self.diagram(wq.condition)?;
            let aggregate = self.diagram(q.condition)?;
            let union = member
                .admitted_binary(
                    &aggregate,
                    super::conditions::kernel::BooleanOperation::Disjunction,
                    &self.budget,
                )
                .map_err(|error| match error {
                    super::conditions::DiagramAdmissionError::Resource(error) => error,
                    super::conditions::DiagramAdmissionError::Boundary(reason) => invalid(
                        &format!("summary contribution implication boundary: {reason:?}"),
                    ),
                })?;
            if union.id() != aggregate.id() {
                return Err(invalid(
                    "summary contribution condition is outside its aggregate",
                ));
            }
        }
        Ok(())
    }
}

type RestatementIndex = ChargedMap<(Id<Occurrence>, Id<EvaluationAtom>), Vec<Id<EvaluationAtom>>>;
