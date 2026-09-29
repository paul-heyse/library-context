//! Whole-call transfer composition (DESIGN §15.6, core review C05/C06/C07).
//!
//! A caller transfer that delivers a value into an argument, the call's binding and a callee
//! transfer compose, matched by call site, into a `Composed` transfer owned by the caller. Roots
//! map through the binder and the parameter declaration links; paths compose only through
//! identity; the condition is the caller's AND the callee's restated at the call, where formal
//! guards need a stability witness. Anything that cannot be justified is an obligation, never an
//! unconditional flow.
use std::collections::{BTreeMap, BTreeSet};
use crate::Domain;
use super::charged::{ChargedMap, StateCharge};
use super::{*, assertion::{Approximation, AssertionQualification}, attribution::{AnalysisContext, Modality, ObligationKind},
    calls::{BindingProjection, BindingSource, BoundCall, CallArgument, CallDestination, CallTarget, ProviderSymbol, Signature},
    conditions::{Condition, ConditionNode, EvaluationAtom, rebase::{GuardCatalog, RebasedGuards, RootBinding, substitute_call_guards},
        stability::{GuardSubstitution, StabilityWitness}},
    declarations::{ParameterDeclaration, SymbolDeclaration}, place_composition::{self, ComposedPaths, PathCatalog},
    source::Occurrence, transfer::{ProvenanceClass, TransferAlternative, TransferBranch, TransferKey, compose_kinds},
    value::{AccessPath, Literal, PathSegment, Place, PlaceRoot, Predicate}};

/// The derivation of one composed alternative from its caller and callee alternatives.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "call_compositions", rule = "compose_through_call", conclusion = composed, invariants = composition_invariants)]
pub struct CallCompositionStep {
    #[model(key)] pub composed: Id<TransferAlternative>,
    #[model(key, premise)] pub caller: Id<TransferAlternative>,
    #[model(key, premise)] pub callee: Id<TransferAlternative>,
    #[model(key, premise)] pub target: Id<CallTarget>,
    #[model(key, premise)] pub signature: Id<Signature>,
    #[model(key, premise)] pub callee_declaration: Id<SymbolDeclaration>,
    #[model(key)] pub bindings: ContentHash,
}

/// One admitted call target at a site with its complete binding.
pub struct CallFrame<'a> {
    pub site: &'a Occurrence, pub target: &'a CallTarget, pub destination: &'a CallDestination, pub bound: &'a BoundCall,
    pub arguments: &'a [CallArgument],
    /// The site's Summary policy admits this target and exactly one signature variant binds.
    pub summary_admitted: bool, pub unique_variant: bool,
}
/// The caller's declaration, which must be the owner of the call site (the owner rule).
pub struct CallerFrame<'a> { pub declaration: &'a SymbolDeclaration, pub site_owner: Id<Occurrence> }
/// The callee's declaration links; witnesses are `None` when flow facts were not requested.
pub struct CalleeFrame<'a> {
    pub symbol: &'a ProviderSymbol, pub declaration: &'a SymbolDeclaration, pub parameters: &'a [ParameterDeclaration],
    pub witnesses: Option<&'a BTreeMap<Id<EvaluationAtom>, StabilityWitness>>,
}
/// Existing rows the composer reads; lookups verify identity.
pub struct CompositionCatalog<'a> {
    pub guards: GuardCatalog<'a>, pub paths: &'a BTreeMap<Id<AccessPath>, AccessPath>, pub segments: PathCatalog<'a>,
}
/// The rows a composition adds, including restated guards and the derivation step.
#[derive(Debug, Default)]
pub struct CompositionRecords {
    pub literals: Vec<Literal>, pub segments: Vec<PathSegment>, pub paths: Vec<AccessPath>, pub roots: Vec<PlaceRoot>, pub places: Vec<Place>,
    pub atoms: Vec<EvaluationAtom>, pub predicates: Vec<Predicate>, pub substitutions: Vec<GuardSubstitution>,
    pub conditions: Vec<Condition>, pub nodes: Vec<ConditionNode>, pub qualification: Option<AssertionQualification>,
    pub key: Option<TransferKey>, pub alternative: Option<TransferAlternative>, pub step: Option<CallCompositionStep>,
}
#[derive(Debug)]
pub enum CallComposition { Transfer(Box<ComposedTransfer>), Disjoint, Subsumed, Obligation(ObligationKind) }
#[derive(Debug)]
pub struct ComposedTransfer { pub branch: TransferBranch, pub records: CompositionRecords }

impl Modality {
    /// The weaker of two modalities: a composition is no stronger than any of its parts.
    pub fn weakest(self, other: Self) -> Self { self.max(other) }
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
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }

/// Where a callee root lands on the caller side.
enum Mapped { Root { root: PlaceRoot, prefix: Option<PathSegment> }, Disjoint, Obligation(ObligationKind) }

struct Composer<'a> { call: &'a CallFrame<'a>, callee: &'a CalleeFrame<'a>, records: CompositionRecords }
impl<'a> Composer<'a> {
    fn parameter(&self, declaration: Id<Occurrence>) -> Option<Id<super::calls::SignatureParameter>> {
        self.callee.parameters.iter().find(|p| p.declaration == declaration).map(|p| p.parameter)
    }
    fn projection(&mut self, projection: &BindingProjection) -> Option<PathSegment> {
        let literal = match projection {
            BindingProjection::Whole => return None,
            BindingProjection::Positional { index } => Literal::Integer { decimal: index.to_string() },
            BindingProjection::Keyword { name } => Literal::String { value: name.clone() },
        };
        let segment = PathSegment::Item { key: literal.id() };
        self.records.literals.push(literal); self.records.segments.push(segment.clone());
        Some(segment)
    }
    /// The binding of a callee root: formals through their declaration link, the receiver through
    /// the receiver binding. `actual` selects the binding of one caller actual when given.
    fn bindings(&self, root: &PlaceRoot) -> Result<Vec<&'a super::calls::Binding>, ObligationKind> {
        let bindings = self.call.bound.bindings();
        match root {
            PlaceRoot::Formal { declaration } => {
                let parameter = self.parameter(*declaration).ok_or(ObligationKind::NoSourceDeclaration)?;
                Ok(bindings.iter().filter(|b| b.formal == parameter).collect())
            },
            PlaceRoot::Receiver { callable } if *callable == self.callee.declaration.declaration =>
                Ok(bindings.iter().filter(|b| b.kind == super::calls::BindingKind::Receiver).collect()),
            _ => Err(ObligationKind::MissingEvidence),
        }
    }
    fn map_output(&mut self, root: &PlaceRoot) -> Result<Vec<Mapped>, ModelError> {
        Ok(match root {
            PlaceRoot::Return { callable } if *callable == self.callee.declaration.declaration =>
                vec![Mapped::Root { root: PlaceRoot::Occurrence { occurrence: self.call.site.id() }, prefix: None }],
            PlaceRoot::Yield { .. } | PlaceRoot::Raise { .. } => vec![Mapped::Obligation(ObligationKind::UnsupportedControlFlow)],
            PlaceRoot::Field { .. } | PlaceRoot::Global { .. } => vec![Mapped::Root { root: root.clone(), prefix: None }],
            PlaceRoot::Formal { .. } | PlaceRoot::Receiver { .. } => match self.bindings(root) {
                Err(kind) => vec![Mapped::Obligation(kind)],
                Ok(bindings) => bindings.into_iter().map(|binding| match &binding.source {
                    BindingSource::Actual(actual) => { let prefix = self.projection(&binding.projection);
                        Mapped::Root { root: PlaceRoot::Occurrence { occurrence: *actual }, prefix } },
                    BindingSource::Default => Mapped::Obligation(ObligationKind::DefaultUnavailable),
                    BindingSource::EmptyVarargs | BindingSource::EmptyKwargs => Mapped::Disjoint,
                }).collect(),
            },
            _ => return Err(invalid("a callee transfer's output root is local to the callee or names another callable")),
        })
    }
    /// Guards on the callee's formals are restated over the call's argument rows.
    fn root_bindings(&self) -> BTreeMap<Id<PlaceRoot>, RootBinding> {
        let mut roots = BTreeMap::new();
        for link in self.callee.parameters {
            let formal = PlaceRoot::Formal { declaration: link.declaration }.id();
            let bound: Vec<_> = self.call.bound.bindings().iter().filter(|b| b.formal == link.parameter).collect();
            let binding = match bound.as_slice() {
                [only] => match (&only.source, &only.projection) {
                    (BindingSource::Actual(actual), BindingProjection::Whole) => self.call.arguments.iter()
                        .find(|argument| argument.value == *actual).cloned().map(RootBinding::Actual),
                    (BindingSource::Default, _) => Some(RootBinding::Default),
                    _ => Some(RootBinding::EmptyAggregate),
                },
                _ => Some(RootBinding::EmptyAggregate),
            };
            if let Some(binding) = binding { roots.insert(formal, binding); }
        }
        roots
    }
}

/// Compose one caller alternative through one call into one callee alternative. Each actual bound
/// into an output root yields its own result.
pub fn compose_call(caller: &TransferBranch, callee: &TransferBranch, call: &CallFrame<'_>, caller_frame: &CallerFrame<'_>,
    callee_frame: &CalleeFrame<'_>, catalog: &CompositionCatalog<'_>) -> Result<Vec<CallComposition>, ModelError> {
    let (caller_key, callee_key) = (caller.key(), callee.key());
    if caller_key.owner != caller_frame.declaration.symbol || caller_frame.declaration.declaration != caller_frame.site_owner {
        return Err(invalid("the caller transfer's owner does not own the call site"));
    }
    if call.target.site != call.site.id() || call.bound.target() != call.target.id() || call.bound.site() != call.site.id() {
        return Err(invalid("call frame target, binding and site disagree"));
    }
    match call.destination {
        CallDestination::Unresolved { reason } => return Ok(vec![CallComposition::Obligation(*reason)]),
        CallDestination::Resolved { symbol } if *symbol != callee_key.owner || *symbol != callee_frame.symbol.id()
            || callee_frame.declaration.symbol != *symbol => return Err(invalid("callee transfer's owner is not the call's target")),
        CallDestination::Resolved { .. } => {},
    }
    if caller_key.context != callee_key.context { return Ok(vec![CallComposition::Obligation(ObligationKind::IncompatibleContexts)]); }
    let place = |id: Id<Place>| catalog.guards.places.get(&id).ok_or_else(|| invalid("transfer place absent"));
    let root = |id: Id<PlaceRoot>| catalog.guards.roots.get(&id).ok_or_else(|| invalid("transfer place root absent"));
    let path = |id: Id<AccessPath>| catalog.paths.get(&id).ok_or_else(|| invalid("transfer access path absent"));
    let (caller_input, caller_output) = (place(caller_key.input)?, place(caller_key.output)?);
    let (callee_input, callee_output) = (place(callee_key.input)?, place(callee_key.output)?);
    // The caller must deliver its value into an actual of this call.
    let PlaceRoot::Occurrence { occurrence: actual } = root(caller_output.root)? else { return Ok(vec![CallComposition::Disjoint]); };
    let mut composer = Composer { call, callee: callee_frame, records: CompositionRecords::default() };
    let callee_root = root(callee_input.root)?;
    if !matches!(callee_root, PlaceRoot::Formal { .. } | PlaceRoot::Receiver { .. }) { return Ok(vec![CallComposition::Disjoint]); }
    let bindings = match composer.bindings(callee_root) { Ok(bindings) => bindings, Err(kind) => return Ok(vec![CallComposition::Obligation(kind)]) };
    let Some(binding) = bindings.into_iter().find(|b| b.source == BindingSource::Actual(*actual)) else { return Ok(vec![CallComposition::Disjoint]); };
    // In formal coordinates the caller's value sits at the projection, then the caller's path.
    let mut delivered = AccessPath::empty();
    if let Some(segment) = composer.projection(&binding.projection) { delivered = delivered.extend(segment.id()); }
    let delivered = delivered.append(path(caller_output.path)?);
    // Projection items are new rows until stored; the path catalog must see them to compare.
    let mut segments = catalog.segments.segments.clone(); let mut literals = catalog.segments.literals.clone();
    for segment in &composer.records.segments { segments.insert(segment.id(), segment.clone()); }
    for literal in &composer.records.literals { literals.insert(literal.id(), literal.clone()); }
    let paths = PathCatalog { segments: &segments, literals: &literals };
    let composed = place_composition::compose(path(caller_input.path)?, &delivered, path(callee_input.path)?, path(callee_output.path)?,
        caller_key.kind, callee_key.kind, &paths)?;
    let (input_path, output_path, kind) = match composed {
        ComposedPaths::Flow { input, output, kind } => (input, output, kind),
        ComposedPaths::Disjoint => return Ok(vec![CallComposition::Disjoint]),
        ComposedPaths::Obligation(kind) => return Ok(vec![CallComposition::Obligation(kind)]),
    };
    // The callee condition restated at the call; unjustified formal guards refuse the transfer.
    let restated: RebasedGuards = match substitute_call_guards(callee.condition(), call.site, caller_key.context, &catalog.guards,
        &composer.root_bindings(), callee_frame.witnesses) {
        Ok(restated) => restated, Err(kind) => return Ok(vec![CallComposition::Obligation(kind)]),
    };
    let condition = match caller.condition().and(&restated.condition) {
        Ok(condition) => condition, Err(boundary) => return Ok(vec![CallComposition::Obligation(super::obligation::from_kernel(boundary))]),
    };
    let modality = if call.summary_admitted && call.unique_variant { Modality::Definite } else { Modality::Candidate }
        .weakest(caller.qualification().modality).weakest(callee.qualification().modality);
    let approximation = caller.qualification().approximation.join(callee.qualification().approximation);
    let caller_root = root(caller_input.root)?.clone();
    let RebasedGuards { atoms, predicates, roots, places, substitutions, .. } = restated;
    let outputs = composer.map_output(root(callee_output.root)?)?;
    // Projection items (literals and segments) are shared by every output of this call.
    let shared = std::mem::take(&mut composer.records);
    let mut results = Vec::new();
    for mapped in outputs {
        let (out_root, prefix) = match mapped {
            Mapped::Root { root, prefix } => (root, prefix),
            Mapped::Disjoint => { results.push(CallComposition::Disjoint); continue; },
            Mapped::Obligation(kind) => { results.push(CallComposition::Obligation(kind)); continue; },
        };
        // An aggregate formal's element maps onto its actual only below the projection.
        let out_path = match &prefix {
            None => output_path.clone(),
            Some(segment) => match place_composition::relation(&output_path, &AccessPath::empty().extend(segment.id()), &paths)? {
                place_composition::PathRelation::Rest(rest) => rest,
                place_composition::PathRelation::Shorter(_) | place_composition::PathRelation::Disjoint => { results.push(CallComposition::Disjoint); continue; },
                place_composition::PathRelation::Unknown => { results.push(CallComposition::Obligation(ObligationKind::AmbiguousBinding)); continue; },
            },
        };
        let mut records = CompositionRecords { literals: shared.literals.clone(), segments: shared.segments.clone(), ..Default::default() };
        let input = place_row(&mut records, caller_root.clone(), input_path.clone());
        let output = place_row(&mut records, out_root, out_path);
        let qualification = AssertionQualification { context: caller_key.context, scope: caller.qualification().scope, condition: condition.id(), modality, approximation };
        let key = TransferKey { owner: caller_key.owner, input: input.id(), output: output.id(), context: caller_key.context, scope: qualification.scope,
            modality, approximation, kind, call_site: Some(call.site.id()), provenance: ProvenanceClass::Composed };
        let branch = TransferBranch::new(key.clone(), qualification.clone(), condition.clone())?;
        let alternative = branch.alternative();
        if alternative == caller.alternative() || alternative == callee.alternative() { results.push(CallComposition::Subsumed); continue; }
        let step = CallCompositionStep { composed: alternative.id(), caller: caller.alternative().id(), callee: callee.alternative().id(),
            target: call.target.id(), signature: call.bound.signature(), callee_declaration: callee_frame.declaration.id(), bindings: binding_digest(call.bound) };
        let (condition_row, nodes) = condition.records();
        records.atoms.extend(atoms.iter().cloned()); records.predicates.extend(predicates.iter().cloned());
        records.roots.extend(roots.iter().cloned()); records.places.extend(places.iter().cloned()); records.substitutions.extend(substitutions.iter().cloned());
        records.conditions.push(condition_row); records.nodes.extend(nodes);
        records.qualification = Some(qualification); records.key = Some(key); records.alternative = Some(alternative); records.step = Some(step);
        results.push(CallComposition::Transfer(Box::new(ComposedTransfer { branch, records })));
    }
    Ok(results)
}
fn place_row(records: &mut CompositionRecords, root: PlaceRoot, path: AccessPath) -> Place {
    let place = Place { root: root.id(), path: path.id() };
    records.roots.push(root); records.paths.push(path); records.places.push(place.clone());
    place
}
/// The complete binding the composition used, encoded structurally.
fn binding_digest(bound: &BoundCall) -> ContentHash {
    let mut sink = KeySink::new("composed-bindings");
    for binding in bound.bindings() {
        binding.formal.encode(&mut sink);
        match &binding.source {
            BindingSource::Actual(actual) => { 0i16.encode(&mut sink); actual.encode(&mut sink); },
            BindingSource::Default => 1i16.encode(&mut sink),
            BindingSource::EmptyVarargs => 2i16.encode(&mut sink),
            BindingSource::EmptyKwargs => 3i16.encode(&mut sink),
        }
        match &binding.projection {
            BindingProjection::Whole => 0i16.encode(&mut sink),
            BindingProjection::Positional { index } => { 1i16.encode(&mut sink); index.encode(&mut sink); },
            BindingProjection::Keyword { name } => { 2i16.encode(&mut sink); name.encode(&mut sink); },
        }
    }
    (bound.bindings().len() as i64).encode(&mut sink);
    sink.finish()
}

/// One admitted target of a call site with the callee alternatives to compose through it.
pub struct SiteCall<'a> { pub frame: CallFrame<'a>, pub callee: CalleeFrame<'a>, pub branches: &'a [TransferBranch] }
/// Compose every caller alternative through every admitted target, signature variant and callee
/// alternative of one site; alternatives stay separate results, never one merged invocation.
pub fn compose_site(callers: &[TransferBranch], calls: &[SiteCall<'_>], caller_frame: &CallerFrame<'_>, catalog: &CompositionCatalog<'_>)
    -> Result<Vec<CallComposition>, ModelError> {
    let mut results = Vec::new();
    for caller in callers { for call in calls { for callee in call.branches {
        results.extend(compose_call(caller, callee, &call.frame, caller_frame, &call.callee, catalog)?);
    } } }
    Ok(results)
}

fn composition_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "call_composition_frames", inputs: vec![
        ValidationInput::of::<AssertionQualification>(&["id"]), ValidationInput::of::<EvaluationAtom>(&["id"]),
        ValidationInput::of::<ConditionNode>(&["id"]), ValidationInput::of::<Condition>(&["id"]),
        ValidationInput::of::<CallDestination>(&["id"]), ValidationInput::of::<CallTarget>(&["id"]), ValidationInput::of::<Signature>(&["id"]),
        ValidationInput::of::<SymbolDeclaration>(&["id"]), ValidationInput::of::<TransferKey>(&["id"]),
        ValidationInput::of::<TransferAlternative>(&["id"]), ValidationInput::of::<CallCompositionStep>(&["composed"]),
    ], create: std::sync::Arc::new(|budget| Box::new(CompositionCheck { charge: StateCharge::new(budget, "call_composition_frames"), ..Default::default() })) }]
}
/// A stored composition step must connect its premises the way `compose_call` does: the composed
/// transfer is the caller's, at the target's site, with the combined kind and approximation, no
/// stronger modality, and a condition over caller atoms or atoms restated at the call; the callee
/// transfer, its declaration and the signature all belong to the target's resolved symbol.
#[derive(Default)]
struct CompositionCheck {
    charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>, (Id<AnalysisContext>, Id<Condition>)>,
    atoms: ChargedMap<Id<EvaluationAtom>, Id<Occurrence>>, nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    conditions: ChargedMap<Id<Condition>, BTreeSet<Id<EvaluationAtom>>>,
    destinations: ChargedMap<Id<CallDestination>, CallDestination>, targets: ChargedMap<Id<CallTarget>, CallTarget>,
    signatures: ChargedMap<Id<Signature>, (Id<ProviderSymbol>, Id<AssertionQualification>)>,
    declarations: ChargedMap<Id<SymbolDeclaration>, Id<ProviderSymbol>>,
    keys: ChargedMap<Id<TransferKey>, TransferKey>, alternatives: ChargedMap<Id<TransferAlternative>, (Id<TransferKey>, Id<AssertionQualification>)>,
}
impl CompositionCheck {
    fn alternative(&self, id: Id<TransferAlternative>) -> Result<(&TransferKey, Id<Condition>), ModelError> {
        let (key, qualification) = self.alternatives.get(&id).ok_or_else(|| invalid("composition premise alternative absent"))?;
        let key = self.keys.get(key).ok_or_else(|| invalid("composition transfer key absent"))?;
        Ok((key, self.qualifications.get(qualification).ok_or_else(|| invalid("composition qualification absent"))?.1))
    }
    fn atoms(&self, condition: Id<Condition>) -> Result<&BTreeSet<Id<EvaluationAtom>>, ModelError> {
        self.conditions.get(&condition).ok_or_else(|| invalid("composition condition absent"))
    }
    fn context(&self, qualification: Id<AssertionQualification>) -> Option<Id<AnalysisContext>> { self.qualifications.get(&qualification).map(|q| q.0) }
    fn step(&self, row: &CallCompositionStep) -> Result<(), ModelError> {
        let (composed, composed_condition) = self.alternative(row.composed)?;
        let (caller, caller_condition) = self.alternative(row.caller)?;
        let (callee, _) = self.alternative(row.callee)?;
        let target = self.targets.get(&row.target).ok_or_else(|| invalid("composition target absent"))?;
        let Some(CallDestination::Resolved { symbol }) = self.destinations.get(&target.destination) else {
            return Err(invalid("composition through an unresolved or absent destination"));
        };
        let (signature_symbol, signature_qualification) = *self.signatures.get(&row.signature).ok_or_else(|| invalid("composition signature absent"))?;
        let declared = *self.declarations.get(&row.callee_declaration).ok_or_else(|| invalid("composition callee declaration absent"))?;
        if callee.owner != *symbol || signature_symbol != *symbol || declared != *symbol {
            return Err(invalid("composition callee, signature and declaration differ from the target's symbol"));
        }
        let contexts = [Some(caller.context), Some(callee.context), self.context(target.qualification), self.context(signature_qualification)];
        if composed.provenance != ProvenanceClass::Composed || composed.call_site != Some(target.site) || composed.owner != caller.owner
            || contexts.iter().any(|context| *context != Some(composed.context)) || composed.scope != caller.scope {
            return Err(invalid("composed transfer is not the caller's transfer at the target's site"));
        }
        if composed.kind != compose_kinds(caller.kind, callee.kind) || composed.approximation != caller.approximation.join(callee.approximation)
            || composed.modality < caller.modality.weakest(callee.modality) {
            return Err(invalid("composed kind, approximation or modality does not follow its premises"));
        }
        let caller_atoms = self.atoms(caller_condition)?;
        for atom in self.atoms(composed_condition)? {
            if !caller_atoms.contains(atom) && self.atoms.get(atom) != Some(&target.site) {
                return Err(invalid("composed condition uses a callee atom not restated at the call"));
            }
        }
        Ok(())
    }
}
impl InvariantCheck for CompositionCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == AssertionQualification::NAME { for row in AssertionQualification::decode(batch)? { self.qualifications.insert(&mut self.charge, row.id(), (row.context, row.condition))?; } }
        else if relation == EvaluationAtom::NAME { for row in EvaluationAtom::decode(batch)? { self.atoms.insert(&mut self.charge, row.id(), row.evaluation)?; } }
        else if relation == ConditionNode::NAME { for row in ConditionNode::decode(batch)? { self.nodes.insert(&mut self.charge, row.id(), row)?; } }
        else if relation == Condition::NAME { for row in Condition::decode(batch)? {
            // Each root's closure is bounded by the kernel's node and atom limits.
            let closure = super::conditions::kernel::closure(row.root, &self.nodes)?;
            let atoms = closure.into_iter().filter_map(|id| match &self.nodes[&id] { ConditionNode::Branch { atom, .. } => Some(*atom), _ => None }).collect();
            self.conditions.insert(&mut self.charge, row.id(), atoms)?;
        } }
        else if relation == CallDestination::NAME { for row in CallDestination::decode(batch)? { self.destinations.insert(&mut self.charge, row.id(), row)?; } }
        else if relation == CallTarget::NAME { for row in CallTarget::decode(batch)? { self.targets.insert(&mut self.charge, row.id(), row)?; } }
        else if relation == Signature::NAME { for row in Signature::decode(batch)? { self.signatures.insert(&mut self.charge, row.id(), (row.symbol, row.qualification))?; } }
        else if relation == SymbolDeclaration::NAME { for row in SymbolDeclaration::decode(batch)? { self.declarations.insert(&mut self.charge, row.id(), row.symbol)?; } }
        else if relation == TransferKey::NAME { for row in TransferKey::decode(batch)? { self.keys.insert(&mut self.charge, row.id(), row)?; } }
        else if relation == TransferAlternative::NAME { for row in TransferAlternative::decode(batch)? { self.alternatives.insert(&mut self.charge, row.id(), (row.transfer, row.qualification))?; } }
        else if relation == CallCompositionStep::NAME { for row in CallCompositionStep::decode(batch)? { self.step(&row)?; } }
        else { return Err(invalid("undeclared composition validation input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { Ok(()) }
}
