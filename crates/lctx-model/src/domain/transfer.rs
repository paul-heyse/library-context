//! Stable transfer keys, qualified alternatives, and explicit control/selection relationships.
//! A transfer key is vocabulary, not an asserted unconditional flow. Supported alternatives own
//! their conditions; aggregation ORs them while retaining every alternative's identity.
use super::charged::{ChargedMap, StateCharge};
use std::collections::{BTreeMap,BTreeSet};
use crate::{Assertion,Domain,DomainCode};
use super::{*, assertion::*, attribution::*, calls::ProviderSymbol, conditions::*, source::*, value::Place};

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum TransferKind { Identity = 0, Derived = 1 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum ProvenanceClass {
    FlowLocal = 0, DerivedSummary = 1, Composed = 2, AuthoredModel = 3,
    ProviderSummary = 4, CatalogFieldLink = 5,
}
/// Stable aggregation identity. The accumulating Boolean condition is deliberately absent.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "transfer_keys", invariants = transfer_invariants)]
pub struct TransferKey {
    #[model(key)] pub owner: Id<ProviderSymbol>,
    #[model(key)] pub input: Id<Place>,
    #[model(key)] pub output: Id<Place>,
    #[model(key)] pub context: Id<AnalysisContext>,
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key)] pub modality: Modality,
    #[model(key)] pub approximation: Approximation,
    #[model(key)] pub kind: TransferKind,
    #[model(key)] pub call_site: Option<Id<Occurrence>>,
    #[model(key)] pub provenance: ProvenanceClass,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "transfer_alternatives")]
#[assertion(support = TransferSupport, name = "transfer_supports", family = FactFamily::Flow, subjects(scope, transfer))]
pub struct TransferAlternative {
    #[model(key)] pub transfer: Id<TransferKey>,
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub scope: Id<CoverageScope>,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name = "control_influences")]
#[assertion(support = ControlSupport, name = "control_supports", family = FactFamily::Flow, subjects(evaluation, input))]
pub struct ControlInfluence {
    #[model(key)] pub qualification: Id<AssertionQualification>,
    #[model(key)] pub input: Id<Place>,
    #[model(key)] pub atom: Id<EvaluationAtom>,
    #[model(key)] pub evaluation: Id<Occurrence>,
}
/// A typed derivation edge: this supported influence reaches an atom guarding this supported
/// alternative of a stable transfer. It is not a value flow from the influencing place.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name = "transfer_selections", rule = "control_selects_transfer")]
pub struct Selection {
    #[model(key, premise)] pub influence: Id<ControlInfluence>,
    #[model(key)] pub atom: Id<EvaluationAtom>,
    #[model(key, premise)] pub alternative: Id<TransferAlternative>,
    #[model(key)] pub transfer: Id<TransferKey>,
}

pub fn compose_kinds(first: TransferKind, then: TransferKind) -> TransferKind {
    match (first,then) {
        (TransferKind::Identity,TransferKind::Identity) => TransferKind::Identity,
        (TransferKind::Identity|TransferKind::Derived,TransferKind::Identity|TransferKind::Derived) => TransferKind::Derived,
    }
}
fn invalid(message: &str) -> ModelError { ModelError::Invalid(message.into()) }
/// Checked semantic input to the pure aggregation kernel. Supports/derivations remain attached
/// to the returned alternative record; additional evidence never changes its qualification.
#[derive(Debug,Clone)]
pub struct TransferBranch {
    key: TransferKey, qualification: AssertionQualification, condition: Diagram,
}
impl TransferBranch {
    pub fn new(key: TransferKey, qualification: AssertionQualification, condition: Diagram) -> Result<Self,ModelError> {
        if qualification.condition != condition.id() || !same_frame(&key,&qualification) {
            return Err(invalid("transfer condition/frame differs from qualification"));
        }
        Ok(Self { key,qualification,condition })
    }
    pub fn key(&self) -> &TransferKey { &self.key }
    pub fn qualification(&self) -> &AssertionQualification { &self.qualification }
    pub fn condition(&self) -> &Diagram { &self.condition }
    pub fn alternative(&self) -> TransferAlternative {
        TransferAlternative { transfer: self.key.id(),qualification: self.qualification.id(),scope: self.qualification.scope }
    }
    pub fn selection(&self, influence: &ControlInfluence, qualification: &AssertionQualification) -> Result<Option<Selection>,ModelError> {
        if influence.qualification != qualification.id() || qualification.context != self.qualification.context
            || qualification.scope != self.qualification.scope {
            return Err(invalid("control influence crosses transfer context/scope"));
        }
        if !self.condition.support().contains(&influence.atom) { return Ok(None); }
        Ok(Some(Selection { influence: influence.id(),atom: influence.atom,alternative: self.alternative().id(),transfer: self.key.id() }))
    }
}
fn same_frame(key: &TransferKey, qualification: &AssertionQualification) -> bool {
    key.context == qualification.context && key.scope == qualification.scope && key.modality == qualification.modality
        && key.approximation == qualification.approximation
}
#[derive(Debug,Clone)]
pub struct MergedTransfer {
    pub key: TransferKey, pub condition: Diagram, pub alternatives: BTreeSet<Id<TransferAlternative>>,
}
/// Equal stable keys merge their conditions. Branch/support IDs stay unchanged; distinct modality,
/// approximation, context, ownership or provenance cannot strengthen one another through merging.
pub fn merge(branches: impl IntoIterator<Item=TransferBranch>) -> Result<Vec<MergedTransfer>,ObligationKind> {
    let mut merged: BTreeMap<Id<TransferKey>,MergedTransfer> = BTreeMap::new();
    let mut count = 0usize;
    for branch in branches {
        count += 1; if count > 100_000 { return Err(ObligationKind::SummaryPairWorkLimit); }
        let id = branch.key.id(); let alternative = branch.alternative().id();
        match merged.get_mut(&id) {
            Some(existing) => {
                if existing.key != branch.key { return Err(ObligationKind::ConflictingProof); }
                existing.condition = existing.condition.or(&branch.condition).map_err(super::obligation::from_kernel)?;
                existing.alternatives.insert(alternative);
            }
            None => { merged.insert(id,MergedTransfer { key: branch.key,condition: branch.condition,alternatives: BTreeSet::from([alternative]) }); }
        }
    }
    Ok(merged.into_values().collect())
}

fn transfer_invariants() -> Vec<Invariant> {
    vec![Invariant { name: "transfer_frames_and_selections", inputs: vec![
        ValidationInput::of::<AssertionQualification>(&["id"]),ValidationInput::of::<ProviderSymbol>(&["id"]),
        ValidationInput::of::<EvaluationAtom>(&["id"]),ValidationInput::of::<ConditionNode>(&["id"]),ValidationInput::of::<Condition>(&["id"]),
        ValidationInput::of::<TransferKey>(&["id"]),ValidationInput::of::<TransferAlternative>(&["id"]),
        ValidationInput::of::<ControlInfluence>(&["id"]),ValidationInput::of::<Selection>(&["id"]),
    ], create: std::sync::Arc::new(|budget| Box::new(TransferCheck { charge: StateCharge::new(budget,"transfer_frames_and_selections"),..Default::default() })) }]
}
#[derive(Default)]
struct TransferCheck { charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>,AssertionQualification>, symbols: ChargedMap<Id<ProviderSymbol>,ProviderSymbol>,
    atoms: ChargedMap<Id<EvaluationAtom>,EvaluationAtom>, nodes: ChargedMap<Id<ConditionNode>,ConditionNode>,
    conditions: ChargedMap<Id<Condition>,BTreeSet<Id<EvaluationAtom>>>,keys: ChargedMap<Id<TransferKey>,TransferKey>,
    alternatives: ChargedMap<Id<TransferAlternative>,TransferAlternative>, influences: ChargedMap<Id<ControlInfluence>,ControlInfluence>,
}
impl TransferCheck {
    fn qualification(&self,id: Id<AssertionQualification>) -> Result<&AssertionQualification,ModelError> {
        self.qualifications.get(&id).ok_or_else(|| invalid("transfer/control qualification absent"))
    }
}
impl InvariantCheck for TransferCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(),ModelError> {
        if relation == AssertionQualification::NAME { for row in AssertionQualification::decode(batch)? { self.qualifications.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == ProviderSymbol::NAME { for row in ProviderSymbol::decode(batch)? { self.symbols.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == EvaluationAtom::NAME { for row in EvaluationAtom::decode(batch)? { self.atoms.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == ConditionNode::NAME { for row in ConditionNode::decode(batch)? { self.nodes.insert(&mut self.charge, row.id(),row)?; } }
        else if relation == Condition::NAME { for row in Condition::decode(batch)? {
            // Each root's closure is bounded by the kernel's node and atom limits.
            let closure = super::conditions::kernel::closure(row.root,&self.nodes)?;
            let atoms = closure.into_iter().filter_map(|id| match &self.nodes[&id] { ConditionNode::Branch { atom,.. } => Some(*atom),_ => None }).collect();
            self.conditions.insert(&mut self.charge, row.id(),atoms)?;
        } }
        else if relation == TransferKey::NAME { for row in TransferKey::decode(batch)? {
            if self.symbols.get(&row.owner).is_none_or(|symbol| symbol.context != row.context) { return Err(invalid("transfer owner context mismatch")); }
            self.keys.insert(&mut self.charge, row.id(),row)?;
        } }
        else if relation == TransferAlternative::NAME { for row in TransferAlternative::decode(batch)? {
            let key = self.keys.get(&row.transfer).ok_or_else(|| invalid("transfer alternative key absent"))?;
            let qualification = self.qualification(row.qualification)?;
            if !same_frame(key,qualification) || row.scope != key.scope { return Err(invalid("transfer alternative frame mismatch")); }
            self.alternatives.insert(&mut self.charge, row.id(),row)?;
        } }
        else if relation == ControlInfluence::NAME { for row in ControlInfluence::decode(batch)? {
            let atom = self.atoms.get(&row.atom).ok_or_else(|| invalid("influence atom absent"))?;
            if atom.evaluation != row.evaluation || atom.context != self.qualification(row.qualification)?.context {
                return Err(invalid("influence evaluation/context mismatch"));
            }
            self.influences.insert(&mut self.charge, row.id(),row)?;
        } }
        else if relation == Selection::NAME { for row in Selection::decode(batch)? {
            let influence = self.influences.get(&row.influence).ok_or_else(|| invalid("selection influence absent"))?;
            let alternative = self.alternatives.get(&row.alternative).ok_or_else(|| invalid("selection alternative absent"))?;
            let influence_q = self.qualification(influence.qualification)?; let alternative_q = self.qualification(alternative.qualification)?;
            if row.atom != influence.atom || row.transfer != alternative.transfer
                || influence_q.context != alternative_q.context || influence_q.scope != alternative_q.scope
                || self.conditions.get(&alternative_q.condition).is_none_or(|atoms| !atoms.contains(&row.atom)) {
                return Err(invalid("selection does not connect influence to its guarded transfer"));
            }
        } }
        else { return Err(invalid("undeclared transfer validation input")); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(),ModelError> { Ok(()) }
}
