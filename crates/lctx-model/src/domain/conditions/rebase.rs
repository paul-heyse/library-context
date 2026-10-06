//! Callee-local guards remain conditional at a caller site. Typed origin chains retain repeated
//! instantiations without identifying separate calls or existentially eliminating local guards.
use super::{Diagram, EvaluationAtom};
use crate::domain::charged::{ChargedMap, StateCharge};
use crate::domain::{
    attribution::{AnalysisContext, ObligationKind},
    source::{Occurrence, OccurrenceRole, SyntaxKind, properties::OccurrenceView},
    value::{Place, PlaceRoot, Predicate},
    *,
};
use std::collections::BTreeMap;
pub const MAX_GUARD_DEPTH: usize = 32;
// Explicit calls and provider-observed implicit call evaluations share the same role contract.
fn is_call(site: &dyn OccurrenceView) -> bool {
    site.syntax_kind() == SyntaxKind::ExprCall || site.role() == OccurrenceRole::Call
}
fn local_root(root: &PlaceRoot) -> Result<(), ObligationKind> {
    if matches!(
        root,
        PlaceRoot::Formal { .. } | PlaceRoot::Receiver { .. } | PlaceRoot::Entry { .. }
    ) {
        Err(ObligationKind::ConditionTransferUnsupported)
    } else {
        Ok(())
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

/// Shared origin traversal for persisted validation and support source authorization.
#[derive(Default)]
pub(crate) struct GuardIndex {
    charge: StateCharge,
    atoms: ChargedMap<Id<EvaluationAtom>, EvaluationAtom>,
    predicates: ChargedMap<Id<Predicate>, Predicate>,
}
impl GuardIndex {
    pub fn new(budget: &crate::domain::resources::ResourceBudget, owner: &'static str) -> Self {
        Self {
            charge: StateCharge::new(budget, owner),
            ..Self::default()
        }
    }
    pub fn visit_input(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        if relation == EvaluationAtom::NAME {
            for row in EvaluationAtom::decode(batch)? {
                self.atoms.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Predicate::NAME {
            for row in Predicate::decode(batch)? {
                self.predicates.insert(&mut self.charge, row.id(), row)?;
            }
        } else {
            return Ok(false);
        }
        Ok(true)
    }
    /// Each lineage is bounded by `MAX_GUARD_DEPTH`; that is the semantic work limit.
    pub fn lineage(&self, id: Id<EvaluationAtom>) -> Result<Vec<EvaluationAtom>, ModelError> {
        let mut at = id;
        let mut context = None;
        let mut rows = Vec::new();
        loop {
            if rows.len() >= MAX_GUARD_DEPTH {
                return Err(invalid("guard origin depth limit"));
            }
            let row = self
                .atoms
                .get(&at)
                .ok_or_else(|| invalid("guard origin atom absent"))?;
            if context.is_some_and(|c| c != row.context) {
                return Err(invalid("guard origin crosses analysis contexts"));
            }
            context = Some(row.context);
            rows.push(row.clone());
            let predicate = self
                .predicates
                .get(&row.predicate)
                .ok_or_else(|| invalid("guard predicate absent"))?;
            match predicate {
                Predicate::InvokedGuard { source } => {
                    if row.operand.is_some() {
                        return Err(invalid(
                            "opaque invoked guard cannot assert a caller operand",
                        ));
                    }
                    at = *source;
                }
                Predicate::BoundGuard { source } => {
                    if row.operand.is_none() {
                        return Err(invalid("a bound guard tests its bound actual"));
                    }
                    at = *source;
                }
                _ => return Ok(rows),
            }
        }
    }
}
pub(crate) fn guard_invariants() -> Vec<Invariant> {
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 1,
        name: "invoked_guard_origins",
        inputs: vec![
            ValidationInput::of::<Predicate>(&["id"]),
            ValidationInput::of::<EvaluationAtom>(&["id"]),
            ValidationInput::of::<Occurrence>(&["id"]),
            ValidationInput::of::<PlaceRoot>(&["id"]),
            ValidationInput::of::<Place>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(GuardCheck {
                charge: StateCharge::new(budget, "invoked_guard_origins"),
                index: GuardIndex::new(budget, "invoked_guard_origins"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct GuardCheck {
    charge: StateCharge,
    index: GuardIndex,
    occurrences: ChargedMap<Id<Occurrence>, bool>,
    roots: ChargedMap<Id<PlaceRoot>, PlaceRoot>,
    places: ChargedMap<Id<Place>, Place>,
}
impl InvariantCheck for GuardCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if self.index.visit_input(relation, batch)? {
        } else if relation == Occurrence::NAME {
            for row in Occurrence::decode(batch)? {
                self.occurrences
                    .insert(&mut self.charge, row.id(), is_call(&row))?;
            }
        } else if relation == PlaceRoot::NAME {
            for row in PlaceRoot::decode(batch)? {
                self.roots.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Place::NAME {
            for row in Place::decode(batch)? {
                self.places.insert(&mut self.charge, row.id(), row)?;
            }
        } else {
            return Err(invalid("undeclared guard origin input"));
        }
        Ok(())
    }
    /// A formal or receiver operand inside an instantiated lineage is admissible only directly
    /// behind a `BoundGuard`, whose witnessed substitution the stability invariant checks.
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for id in self.index.atoms.keys().copied() {
            let lineage = self.index.lineage(id)?;
            for (index, row) in lineage.iter().enumerate() {
                let wrapped = matches!(
                    self.index.predicates.get(&row.predicate),
                    Some(Predicate::InvokedGuard { .. } | Predicate::BoundGuard { .. })
                );
                if wrapped && self.occurrences.get(&row.evaluation) != Some(&true) {
                    return Err(invalid(
                        "invoked guard evaluation must name a call occurrence",
                    ));
                }
                if index == 0 {
                    continue;
                }
                let Some(operand) = row.operand else {
                    continue;
                };
                let place = self
                    .places
                    .get(&operand)
                    .ok_or_else(|| invalid("invoked guard origin place absent"))?;
                let root = self
                    .roots
                    .get(&place.root)
                    .ok_or_else(|| invalid("invoked guard origin root absent"))?;
                let bound = matches!(
                    self.index.predicates.get(&lineage[index - 1].predicate),
                    Some(Predicate::BoundGuard { .. })
                );
                if !bound {
                    local_root(root).map_err(|_| {
                        invalid(
                            "invoked guard formal/receiver needs binding and stability evidence",
                        )
                    })?;
                }
            }
        }
        Ok(())
    }
}

pub struct GuardCatalog<'a> {
    pub atoms: &'a BTreeMap<Id<EvaluationAtom>, EvaluationAtom>,
    pub predicates: &'a BTreeMap<Id<Predicate>, Predicate>,
    pub places: &'a BTreeMap<Id<Place>, Place>,
    pub roots: &'a BTreeMap<Id<PlaceRoot>, PlaceRoot>,
}
#[derive(Debug)]
pub struct RebasedGuards {
    pub condition: Diagram,
    pub atoms: Vec<EvaluationAtom>,
    pub predicates: Vec<Predicate>,
    pub roots: Vec<PlaceRoot>,
    pub places: Vec<Place>,
    pub substitutions: Vec<super::stability::GuardSubstitution>,
    pub qualifications: Vec<crate::domain::assertion::AssertionQualification>,
    pub influences: Vec<GuardInfluence>,
    admission: RebaseAdmission,
}
/// The actual emitter lowers this description into its own nominal influence relation.
#[derive(Debug, Clone)]
pub struct GuardInfluence {
    pub qualification: Id<crate::domain::assertion::AssertionQualification>,
    pub input: Id<Place>,
    pub atom: Id<EvaluationAtom>,
    pub evaluation: Id<Occurrence>,
}
#[derive(Debug, Default)]
pub struct RebaseAdmission {
    _rows: StateCharge,
    condition: Option<Box<dyn crate::domain::resources::Reservation>>,
}
impl RebasedGuards {
    /// Move the allowance into the output owner before moving the public records.
    pub fn take_admission(&mut self) -> RebaseAdmission {
        std::mem::take(&mut self.admission)
    }
}
/// How a callee's formal or receiver root is bound at one call.
#[derive(Debug, Clone)]
pub enum RootBinding {
    Actual(super::stability::CheckedGuardBinding),
    Default,
    EmptyAggregate,
}
/// Rebase local guards only. A formal/receiver operand requires a separately established binding
/// and stability witness; this operation refuses it rather than inventing substitution evidence.
pub fn rebase_local_guards(
    source: &Diagram,
    site: &dyn OccurrenceView,
    context: Id<AnalysisContext>,
    catalog: &GuardCatalog<'_>,
    budget: &crate::domain::resources::ResourceBudget,
) -> Result<RebasedGuards, ObligationKind> {
    substitute_call_guards(
        source,
        site,
        context,
        catalog,
        &BTreeMap::new(),
        Some(&BTreeMap::new()),
        None,
        budget,
    )
}
/// Restate a callee condition at a call. Local and operand-free guards become opaque
/// `InvokedGuard`s at the call site. A guard on a formal or receiver becomes a `BoundGuard` over the
/// bound actual only when it is substitutable, tests the whole root, the root is bound to an actual
/// and a stability witness exists; otherwise the transfer is refused, never made unconditional.
/// `witnesses` is `None` when flow facts were not requested.
#[expect(
    clippy::too_many_arguments,
    reason = "The restatement binds the callee condition, call site, context, catalogs, root bindings, stability witnesses, caller and budget"
)]
pub fn substitute_call_guards(
    source: &Diagram,
    site: &dyn OccurrenceView,
    context: Id<AnalysisContext>,
    catalog: &GuardCatalog<'_>,
    bindings: &BTreeMap<Id<PlaceRoot>, RootBinding>,
    witnesses: Option<&BTreeMap<Id<EvaluationAtom>, super::stability::CheckedStability>>,
    caller: Option<&crate::domain::assertion::AssertionQualification>,
    budget: &crate::domain::resources::ResourceBudget,
) -> Result<RebasedGuards, ObligationKind> {
    if site.validate_site().is_err() || !is_call(site) {
        return Err(ObligationKind::MissingEvidence);
    }
    let mut rows = StateCharge::new(budget, "guard_restatement");
    rows.grow(
        source
            .support()
            .len()
            .saturating_mul(8192)
            .saturating_add(source.allocation_allowance()),
    )
    .map_err(|_| ObligationKind::ResourceRefused)?;
    let mut out = RebasedGuards {
        condition: Diagram::always(),
        atoms: vec![],
        predicates: vec![],
        roots: vec![],
        places: vec![],
        substitutions: vec![],
        qualifications: vec![],
        influences: vec![],
        admission: RebaseAdmission {
            _rows: rows,
            condition: None,
        },
    };
    let mut replacements = Vec::new();
    let mut work = 0;
    let empty = crate::domain::value::AccessPath::empty().id();
    for id in source.support() {
        let atom = lookup(catalog.atoms, *id)?;
        if atom.context != context {
            return Err(ObligationKind::MissingEvidence);
        }
        let formal = match atom.operand {
            Some(operand) => {
                let place = lookup(catalog.places, operand)?;
                let root = lookup(catalog.roots, place.root)?;
                local_root(root)
                    .is_err()
                    .then_some((place, lookup(catalog.predicates, atom.predicate)?))
            }
            None => None,
        };
        if let Some((place, predicate)) = formal {
            let witnesses = witnesses.ok_or(ObligationKind::NotRequested)?;
            if place.path != empty || !super::stability::substitutable(predicate) {
                return Err(ObligationKind::ConditionTransferUnsupported);
            }
            let binding = match bindings.get(&place.root) {
                Some(RootBinding::Actual(binding)) => binding,
                Some(RootBinding::Default) => return Err(ObligationKind::DefaultStabilityUnknown),
                Some(RootBinding::EmptyAggregate) | None => {
                    return Err(ObligationKind::ConditionTransferUnsupported);
                }
            };
            let witness = witnesses
                .get(id)
                .ok_or(ObligationKind::ConditionTransferUnsupported)?;
            if witness.root() != place.root
                || witness.witness().atom != *id
                || !binding.agrees(witness, site.occurrence_id(), context)
            {
                return Err(ObligationKind::MissingEvidence);
            }
            let caller = caller.ok_or(ObligationKind::MissingEvidence)?;
            if caller.context != context || caller.validate().is_err() {
                return Err(ObligationKind::MissingEvidence);
            }
            let root = binding.root();
            let operand = Place {
                root: root.id(),
                path: empty,
            };
            let predicate = Predicate::BoundGuard { source: *id };
            let bound = EvaluationAtom {
                evaluation: site.occurrence_id(),
                context,
                predicate: predicate.id(),
                operand: Some(operand.id()),
            };
            out.substitutions.push(super::stability::GuardSubstitution {
                atom: bound.id(),
                witness: witness.witness().id(),
                binding: binding.binding(),
                source: binding.source(),
                event: binding.event(),
                source_atom: *id,
                actual_place: operand.id(),
                qualification: caller.id(),
            });
            out.influences.push(GuardInfluence {
                qualification: caller.id(),
                input: operand.id(),
                atom: bound.id(),
                evaluation: site.occurrence_id(),
            });
            if !out.qualifications.contains(caller) {
                out.qualifications.push(caller.clone());
            }
            replacements.push((*id, Diagram::from_atom(bound.id())));
            out.roots.push(root);
            out.places.push(operand);
            out.predicates.push(predicate);
            out.atoms.push(bound);
            continue;
        }
        // Local and operand-free guards: every origin in the invoked lineage must be local too.
        let mut at = *id;
        let mut depth = 0;
        loop {
            depth += 1;
            work += 1;
            if depth >= MAX_GUARD_DEPTH {
                return Err(ObligationKind::SummaryDepthLimit);
            }
            if work > 4096 {
                return Err(ObligationKind::ConditionWorkLimit);
            }
            let row = lookup(catalog.atoms, at)?;
            if row.context != context {
                return Err(ObligationKind::MissingEvidence);
            }
            if let Some(operand) = row.operand {
                let place = lookup(catalog.places, operand)?;
                let root = lookup(catalog.roots, place.root)?;
                local_root(root)?;
            }
            if let Predicate::InvokedGuard { source } = lookup(catalog.predicates, row.predicate)? {
                if row.operand.is_some() {
                    return Err(ObligationKind::MissingEvidence);
                }
                at = *source;
            } else {
                break;
            }
        }
        let predicate = Predicate::InvokedGuard { source: *id };
        let invoked = EvaluationAtom {
            evaluation: site.occurrence_id(),
            context,
            predicate: predicate.id(),
            operand: None,
        };
        replacements.push((*id, Diagram::from_atom(invoked.id())));
        out.predicates.push(predicate);
        out.atoms.push(invoked);
    }
    let replacements: Vec<_> = replacements
        .iter()
        .map(|(id, diagram)| (*id, diagram))
        .collect();
    let admitted = source
        .admitted_substitution(&replacements, budget)
        .map_err(|error| match error {
            super::DiagramAdmissionError::Boundary(boundary) => {
                super::super::obligation::from_kernel(boundary)
            }
            super::DiagramAdmissionError::Resource(_) => ObligationKind::ResourceRefused,
        })?;
    let (condition, reservation) = admitted.into_parts();
    out.condition = condition;
    out.admission.condition = Some(reservation);
    Ok(out)
}
fn lookup<R: Record>(rows: &BTreeMap<Id<R>, R>, id: Id<R>) -> Result<&R, ObligationKind> {
    let row = rows.get(&id).ok_or(ObligationKind::MissingEvidence)?;
    if row.id() != id || row.validate().is_err() {
        return Err(ObligationKind::MissingEvidence);
    }
    Ok(row)
}

pub(crate) fn guard_invariants_refs() -> Vec<&'static str> {
    vec!["invoked_guard_origins"]
}
