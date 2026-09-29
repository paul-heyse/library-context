//! Callee-local guards remain conditional at a caller site. Typed origin chains retain repeated
//! instantiations without identifying separate calls or existentially eliminating local guards.
use std::collections::BTreeMap;
use crate::domain::charged::{ChargedMap,StateCharge};
use super::{Diagram,EvaluationAtom};
use crate::domain::{*,attribution::{AnalysisContext,ObligationKind},source::{Occurrence,OccurrenceRole,SyntaxKind},value::{Place,PlaceRoot,Predicate}};
pub const MAX_GUARD_DEPTH: usize = 32;
// Explicit calls and provider-observed implicit call evaluations share the same role contract.
fn is_call(site: &Occurrence) -> bool { site.syntax_kind == SyntaxKind::ExprCall || site.role == OccurrenceRole::Call }
fn local_root(root: &PlaceRoot) -> Result<(),ObligationKind> {
    if matches!(root,PlaceRoot::Formal { .. }|PlaceRoot::Receiver { .. }) { Err(ObligationKind::ConditionTransferUnsupported) } else { Ok(()) }
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }

/// Shared origin traversal for persisted validation and support source authorization.
#[derive(Default)]
pub(crate) struct GuardIndex {
    charge: StateCharge,
    atoms: ChargedMap<Id<EvaluationAtom>,EvaluationAtom>,predicates: ChargedMap<Id<Predicate>,Predicate>,
}
impl GuardIndex {
    pub fn new(budget: &crate::domain::resources::ResourceBudget,owner: &'static str) -> Self { Self { charge: StateCharge::new(budget,owner),..Self::default() } }
    pub fn visit_input(&mut self,relation: &str,batch: &arrow_array::RecordBatch) -> Result<bool,ModelError> {
        if relation == EvaluationAtom::NAME { for row in EvaluationAtom::decode(batch)? { self.atoms.insert(&mut self.charge,row.id(),row)?; } }
        else if relation == Predicate::NAME { for row in Predicate::decode(batch)? { self.predicates.insert(&mut self.charge,row.id(),row)?; } }
        else { return Ok(false); }
        Ok(true)
    }
    /// Each lineage is bounded by `MAX_GUARD_DEPTH`; that is the semantic work limit.
    pub fn lineage(&self,id: Id<EvaluationAtom>) -> Result<Vec<EvaluationAtom>,ModelError> {
        let mut at = id; let mut context = None; let mut rows = Vec::new();
        loop {
            if rows.len() >= MAX_GUARD_DEPTH { return Err(invalid("guard origin depth limit")); }
            let row = self.atoms.get(&at).ok_or_else(|| invalid("guard origin atom absent"))?;
            if context.is_some_and(|c| c != row.context) { return Err(invalid("guard origin crosses analysis contexts")); }
            context = Some(row.context);
            rows.push(row.clone());
            let predicate = self.predicates.get(&row.predicate).ok_or_else(|| invalid("guard predicate absent"))?;
            if let Predicate::InvokedGuard { source } = predicate {
                if row.operand.is_some() { return Err(invalid("opaque invoked guard cannot assert a caller operand")); }
                at = *source;
            } else { return Ok(rows); }
        }
    }
}
pub(super) fn guard_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "invoked_guard_origins",inputs: vec![ValidationInput::of::<Predicate>(&["id"]),ValidationInput::of::<EvaluationAtom>(&["id"]),
        ValidationInput::of::<Occurrence>(&["id"]),ValidationInput::of::<PlaceRoot>(&["id"]),ValidationInput::of::<Place>(&["id"])],create: std::sync::Arc::new(|budget| Box::new(GuardCheck {
            charge: StateCharge::new(budget,"invoked_guard_origins"),index: GuardIndex::new(budget,"invoked_guard_origins"),..Default::default() })) }]
}
#[derive(Default)]
struct GuardCheck { charge: StateCharge,index: GuardIndex,occurrences: ChargedMap<Id<Occurrence>,bool>,
    roots: ChargedMap<Id<PlaceRoot>,PlaceRoot>,places: ChargedMap<Id<Place>,Place> }
impl InvariantCheck for GuardCheck {
    fn visit(&mut self,relation: &str,batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if self.index.visit_input(relation,batch)? {}
        else if relation == Occurrence::NAME { for row in Occurrence::decode(batch)? { self.occurrences.insert(&mut self.charge,row.id(),is_call(&row))?; } }
        else if relation == PlaceRoot::NAME { for row in PlaceRoot::decode(batch)? { self.roots.insert(&mut self.charge,row.id(),row)?; } }
        else if relation == Place::NAME { for row in Place::decode(batch)? { self.places.insert(&mut self.charge,row.id(),row)?; } }
        else { return Err(invalid("undeclared guard origin input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> {
        for id in self.index.atoms.keys().copied() {
            let lineage = self.index.lineage(id)?;
            let invoked = lineage.len() > 1;
            for row in lineage {
            if invoked { if let Some(operand) = row.operand {
                let place = self.places.get(&operand).ok_or_else(|| invalid("invoked guard origin place absent"))?;
                let root = self.roots.get(&place.root).ok_or_else(|| invalid("invoked guard origin root absent"))?;
                local_root(root).map_err(|_| invalid("invoked guard formal/receiver needs binding and stability evidence"))?;
            } }
            if matches!(self.index.predicates.get(&row.predicate),Some(Predicate::InvokedGuard { .. }))
                && self.occurrences.get(&row.evaluation) != Some(&true) {
                return Err(invalid("invoked guard evaluation must name a call occurrence"));
            }
        } } Ok(())
    }
}

pub struct GuardCatalog<'a> {
    pub atoms: &'a BTreeMap<Id<EvaluationAtom>,EvaluationAtom>,
    pub predicates: &'a BTreeMap<Id<Predicate>,Predicate>,
    pub places: &'a BTreeMap<Id<Place>,Place>,
    pub roots: &'a BTreeMap<Id<PlaceRoot>,PlaceRoot>,
}
#[derive(Debug)]
pub struct RebasedGuards { pub condition: Diagram,pub atoms: Vec<EvaluationAtom>,pub predicates: Vec<Predicate> }
/// Rebase local guards only. A formal/receiver operand requires a separately established binding
/// and stability witness; this operation refuses it rather than inventing substitution evidence.
/// A caller composing a whole call must first discharge those operands through the binding owner.
pub fn rebase_local_guards(source: &Diagram,site: &Occurrence,context: Id<AnalysisContext>,catalog: &GuardCatalog<'_>) -> Result<RebasedGuards,ObligationKind> {
    if site.validate().is_err() || !is_call(site) { return Err(ObligationKind::MissingEvidence); }
    let mut atoms = Vec::new(); let mut predicates = Vec::new(); let mut replacements = Vec::new(); let mut work = 0;
    for id in source.support() {
        let mut at = *id; let mut depth = 0;
        loop {
            depth += 1; work += 1;
            if depth >= MAX_GUARD_DEPTH { return Err(ObligationKind::SummaryDepthLimit); }
            if work > 4096 { return Err(ObligationKind::ConditionWorkLimit); }
            let atom = lookup(catalog.atoms,at)?;
            if atom.context != context { return Err(ObligationKind::MissingEvidence); }
            if let Some(operand) = atom.operand {
                let place = lookup(catalog.places,operand)?; let root = lookup(catalog.roots,place.root)?;
                local_root(root)?;
            }
            if let Predicate::InvokedGuard { source } = lookup(catalog.predicates,atom.predicate)? {
                if atom.operand.is_some() { return Err(ObligationKind::MissingEvidence); }
                at = *source;
            } else { break; }
        }
        let predicate = Predicate::InvokedGuard { source: *id };
        let atom = EvaluationAtom { evaluation: site.id(),context,predicate: predicate.id(),operand: None };
        replacements.push((*id,Diagram::from_atom(atom.id()))); predicates.push(predicate); atoms.push(atom);
    }
    let replacements: Vec<_> = replacements.iter().map(|(id,diagram)| (*id,diagram)).collect();
    let condition = source.substitute_atoms(&replacements).map_err(super::super::obligation::from_kernel)?;
    Ok(RebasedGuards { condition,atoms,predicates })
}
fn lookup<R: Record>(rows: &BTreeMap<Id<R>,R>,id: Id<R>) -> Result<&R,ObligationKind> {
    let row = rows.get(&id).ok_or(ObligationKind::MissingEvidence)?;
    if row.id() != id || row.validate().is_err() { return Err(ObligationKind::MissingEvidence); } Ok(row)
}
