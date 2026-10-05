//! Stable transfer keys, qualified alternatives, and explicit control/selection relationships.
//! A transfer key is vocabulary, not an asserted unconditional flow. Supported alternatives own
//! their conditions; aggregation ORs them while retaining every alternative's identity.
use super::charged::{ChargedMap, StateCharge};
use super::{assertion::*, attribution::*, conditions::*, source::*, value::Place, *};
use super::{
    derivation::RowRef,
    normalized::entities::EntityRef,
    resources::{Reservation, ResourceBudget},
};
use crate::{Assertion, Domain, DomainCode};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TransferKind {
    Identity = 0,
    Derived = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ProvenanceClass {
    FlowLocal = 0,
    DerivedSummary = 1,
    Composed = 2,
    AuthoredModel = 3,
    ProviderSummary = 4,
    CatalogFieldLink = 5,
}
/// Shared semantic frame, independent of publication identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferDescriptor {
    pub owner: Id<EntityRef>,
    pub input: Id<Place>,
    pub output: Id<Place>,
    pub context: Id<AnalysisContext>,
    pub scope: Id<CoverageScope>,
    pub modality: Modality,
    pub approximation: Approximation,
    pub kind: TransferKind,
    pub call_site: Option<Id<Occurrence>>,
    pub provenance: ProvenanceClass,
}
impl HeapSize for TransferDescriptor {}
impl TransferDescriptor {
    pub fn check(&self, q: &AssertionQualification) -> Result<(), ModelError> {
        if !same_frame(self, q) {
            return Err(invalid("transfer frame differs from qualification"));
        }
        Ok(())
    }
}
/// Each instantiation owns its persistent identity and companion source. No descriptor registry.
macro_rules! transfer_family {
 ($owner:ident,$prefix:literal,$support_name:literal,$source:path,$($controls:tt)*) => {pub mod $owner {
 use super::*;
 #[derive(Debug,Clone,PartialEq,Eq,Domain)]
 #[model(name=concat!($prefix,"_transfer_keys"),invariant_refs=family_invariants_refs)]
 pub struct TransferKey {
    #[model(key)]
    pub owner: Id<EntityRef>,
    #[model(key)]
    pub input: Id<Place>,
    #[model(key)]
    pub output: Id<Place>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub scope: Id<CoverageScope>,
    #[model(key)]
    pub modality: Modality,
    #[model(key)]
    pub approximation: Approximation,
    #[model(key)]
    pub kind: TransferKind,
    #[model(key)]
    pub call_site: Option<Id<Occurrence>>,
    #[model(key)]
    pub provenance: ProvenanceClass,
}

 #[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
 #[model(name=concat!($prefix,"_transfer_alternatives"))]
 #[assertion(support=TransferSupport,name=$support_name,family=FactFamily::Flow,derived,source=$source,subjects(scope,transfer))]
 pub struct TransferAlternative {
 #[model(key)] pub transfer:Id<TransferKey>,
 #[model(key)] pub qualification:Id<AssertionQualification>,
 #[model(key)] pub scope:Id<CoverageScope>,
 }
 impl TransferKeyRecord for TransferKey {
 type Alternative=TransferAlternative;
 fn descriptor(&self)->TransferDescriptor {TransferDescriptor {owner:self.owner,input:self.input,output:self.output,context:self.context,scope:self.scope,modality:self.modality,approximation:self.approximation,kind:self.kind,call_site:self.call_site,provenance:self.provenance}}
 fn from_descriptor(d:TransferDescriptor)->Self {Self {owner:d.owner,input:d.input,output:d.output,context:d.context,scope:d.scope,modality:d.modality,approximation:d.approximation,kind:d.kind,call_site:d.call_site,provenance:d.provenance}}
 fn alternative(&self,q:&AssertionQualification)->TransferAlternative {TransferAlternative {transfer:self.id(),qualification:q.id(),scope:q.scope}}
 }
 impl TransferAlternativeRecord for TransferAlternative {
 type TransferKey=TransferKey;
 fn transfer(&self)->Id<TransferKey> {self.transfer}
 fn qualification(&self)->Id<AssertionQualification> {self.qualification}
 fn scope(&self)->Id<CoverageScope> {self.scope}
 }
 impl assertion::SubjectValue for Id<TransferKey> {
 fn inputs()->Vec<ValidationInput> {let mut rows=<Id<Place> as assertion::SubjectValue>::inputs();rows.push(ValidationInput::of::<TransferKey>(&["id"]));rows}
 fn append_subjects(&self,rows:&mut Vec<assertion::Subject>) {rows.push(assertion::Subject::Transfer(RowRef::of(*self)));}
 }
 pub(crate) fn family_invariants()->Vec<Invariant> {frame_invariants::<TransferKey>()}
 fn family_invariants_refs()->Vec<&'static str> {vec![TransferKey::NAME]}
 pub fn relations()->Vec<Relation> {vec![Relation::of::<TransferKey>(),Relation::of::<TransferAlternative>(),Relation::of::<TransferSupport>()]}
 $($controls)*
 }};
}
macro_rules! control_family {
 ($control_name:literal,$control_support_name:literal,$selection_name:literal,$source:path)=>{
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = $control_name)]
#[assertion(support = ControlSupport, name = $control_support_name, family = FactFamily::Flow, derived, source = $source, subjects(evaluation, input))]
pub struct ControlInfluence {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub input: Id<Place>,
    #[model(key)]
    pub atom: Id<EvaluationAtom>,
    #[model(key)]
    pub evaluation: Id<Occurrence>,
}
/// A typed derivation edge: this supported influence reaches an atom guarding this supported
/// alternative of a stable transfer. It is not a value flow from the influencing place.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = $selection_name, invariant_refs = control_invariants_refs, rule = "control_selects_transfer")]
pub struct Selection {
    #[model(key, premise)]
    pub influence: Id<ControlInfluence>,
    #[model(key)]
    pub atom: Id<EvaluationAtom>,
    #[model(key, premise)]
    pub alternative: Id<TransferAlternative>,
    #[model(key)]
    pub transfer: Id<TransferKey>,
}

impl ControlRecord for ControlInfluence {fn atom(&self)->Id<EvaluationAtom> {self.atom}fn evaluation(&self)->Id<Occurrence> {self.evaluation}}
impl SelectionRecord for Selection {type Transfer=TransferKey;type Control=ControlInfluence;fn influence(&self)->Id<ControlInfluence> {self.influence}fn atom(&self)->Id<EvaluationAtom> {self.atom}fn alternative(&self)->Id<TransferAlternative> {self.alternative}fn transfer(&self)->Id<TransferKey> {self.transfer}}
pub(crate) fn control_invariants()->Vec<Invariant> {super::control_invariants::<TransferKey,ControlInfluence,Selection>()}
 fn control_invariants_refs()->Vec<&'static str> {vec![Selection::NAME]}

 };
}
transfer_family!(local,"local","local_transfer_supports",crate::domain::analysis::local::SupportSource,
control_family!("local_control_influences","local_control_supports","local_transfer_selections",crate::domain::analysis::local::SupportSource);
);
transfer_family!(
    model,
    "model",
    "model_transfer_supports",
    crate::domain::analysis::model::SupportSource,
);
transfer_family!(summary,"summary","summary_transfer_supports",crate::domain::analysis::summary::SupportSource,
control_family!("summary_control_influences","summary_control_supports","summary_transfer_selections",crate::domain::analysis::summary::SupportSource);
pub use super::witness::{SummaryPremise,SummaryWitness,SummaryContribution,WitnessBranch,TransferEvidence};
);
pub(crate) mod witness;
/// Read-only typed projection used by the shared support validator. Membership stays nominal.
pub(crate) fn subject_rows(
    relation: &str,
    batch: &arrow_array::RecordBatch,
) -> Result<Option<Vec<(RowRef, TransferDescriptor)>>, ModelError> {
    macro_rules! decode {
        ($ty:ty) => {
            if relation == <$ty>::NAME {
                return Ok(Some(
                    <$ty>::decode(batch)?
                        .into_iter()
                        .map(|row| (RowRef::of(row.id()), row.descriptor()))
                        .collect(),
                ));
            }
        };
    }
    decode!(local::TransferKey);
    decode!(model::TransferKey);
    decode!(summary::TransferKey);
    Ok(None)
}
pub trait TransferKeyRecord: Record + Clone + std::fmt::Debug + PartialEq + Eq {
    type Alternative: TransferAlternativeRecord<TransferKey = Self>;
    fn descriptor(&self) -> TransferDescriptor;
    fn from_descriptor(d: TransferDescriptor) -> Self;
    fn alternative(&self, q: &AssertionQualification) -> Self::Alternative;
}
pub trait TransferAlternativeRecord: assertion::Assertion {
    type TransferKey: TransferKeyRecord<Alternative = Self>;
    fn transfer(&self) -> Id<Self::TransferKey>;
    fn qualification(&self) -> Id<AssertionQualification>;
    fn scope(&self) -> Id<CoverageScope>;
}
pub fn compose_kinds(first: TransferKind, then: TransferKind) -> TransferKind {
    match (first, then) {
        (TransferKind::Identity, TransferKind::Identity) => TransferKind::Identity,
        (
            TransferKind::Identity | TransferKind::Derived,
            TransferKind::Identity | TransferKind::Derived,
        ) => TransferKind::Derived,
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
/// Shared branch admission retains its BDD allowance across cheap Arc clones.
#[derive(Debug, Clone)]
pub struct TransferBranch<K: TransferKeyRecord = local::TransferKey>(Arc<Branch<K>>);
#[derive(Debug)]
struct Branch<K: TransferKeyRecord> {
    key: K,
    qualification: AssertionQualification,
    condition: Diagram,
    _charge: Box<dyn Reservation>,
}
impl<K: TransferKeyRecord> TransferBranch<K> {
    pub fn new(
        key: K,
        qualification: AssertionQualification,
        condition: Diagram,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        key.descriptor().check(&qualification)?;
        if qualification.condition != condition.id() {
            return Err(invalid("transfer condition differs from qualification"));
        }
        let charge = budget.reserve(
            "transfer_branch",
            condition.allocation_allowance() + size_of::<Branch<K>>() + key.heap_bytes(),
        )?;
        Ok(Self(Arc::new(Branch {
            key,
            qualification,
            condition,
            _charge: charge,
        })))
    }
    pub fn key(&self) -> &K {
        &self.0.key
    }
    pub fn descriptor(&self) -> TransferDescriptor {
        self.key().descriptor()
    }
    pub fn qualification(&self) -> &AssertionQualification {
        &self.0.qualification
    }
    pub fn condition(&self) -> &Diagram {
        &self.0.condition
    }
    pub fn alternative(&self) -> K::Alternative {
        self.key().alternative(self.qualification())
    }
    pub fn reference(&self) -> RowRef {
        RowRef::of(self.alternative().id())
    }
}
impl TransferBranch<local::TransferKey> {
    pub fn selection(
        &self,
        influence: &local::ControlInfluence,
        q: &AssertionQualification,
    ) -> Result<Option<local::Selection>, ModelError> {
        if influence.qualification != q.id()
            || q.context != self.qualification().context
            || q.scope != self.qualification().scope
            || q.assumptions != self.qualification().assumptions
        {
            return Err(invalid("control influence crosses transfer context/scope"));
        }
        Ok(self
            .condition()
            .support()
            .contains(&influence.atom)
            .then(|| local::Selection {
                influence: influence.id(),
                atom: influence.atom,
                alternative: self.alternative().id(),
                transfer: self.key().id(),
            }))
    }
}
fn same_frame(key: &TransferDescriptor, q: &AssertionQualification) -> bool {
    key.context == q.context
        && key.scope == q.scope
        && key.modality == q.modality
        && key.approximation == q.approximation
}
#[derive(Debug)]
pub struct MergedTransfer<K: TransferKeyRecord = local::TransferKey> {
    pub key: K,
    pub assumptions: Id<assumptions::AssumptionSet>,
    pub condition: Diagram,
    pub alternatives: BTreeSet<Id<K::Alternative>>,
    _condition_charge: Box<dyn Reservation>,
}
/// Reservation includes map/vector capacity and every retained alternative; it follows output.
#[derive(Debug)]
pub struct TransferAggregation<K: TransferKeyRecord> {
    rows: Vec<MergedTransfer<K>>,
    _charge: StateCharge,
}
impl<K: TransferKeyRecord> std::ops::Deref for TransferAggregation<K> {
    type Target = [MergedTransfer<K>];
    fn deref(&self) -> &Self::Target {
        &self.rows
    }
}
#[derive(Debug)]
pub enum TransferError {
    Obligation(ObligationKind),
    Resource(ModelError),
}
impl From<ModelError> for TransferError {
    fn from(error: ModelError) -> Self {
        Self::Resource(error)
    }
}
pub fn merge<K: TransferKeyRecord>(
    branches: impl IntoIterator<Item = TransferBranch<K>>,
    budget: &ResourceBudget,
) -> Result<TransferAggregation<K>, TransferError> {
    let mut merged: BTreeMap<(Id<K>, ContentHash), MergedTransfer<K>> = BTreeMap::new();
    let mut charge = StateCharge::new(budget, "transfer_aggregation");
    let mut count = 0usize;
    for branch in branches {
        count += 1;
        if count > 100_000 {
            return Err(TransferError::Obligation(
                ObligationKind::SummaryPairWorkLimit,
            ));
        }
        let id = (
            branch.key().id(),
            analysis::support::alternative_basis(branch.qualification()),
        );
        let alternative = branch.alternative().id();
        match merged.get_mut(&id) {
            Some(existing) => {
                if existing.key != *branch.key() {
                    return Err(TransferError::Obligation(ObligationKind::ConflictingProof));
                }
                if !existing.alternatives.contains(&alternative) {
                    charge.grow(size_of::<Id<K::Alternative>>() + 64)?;
                }
                let admitted = existing
                    .condition
                    .admitted_binary(
                        branch.condition(),
                        kernel::BooleanOperation::Disjunction,
                        budget,
                    )
                    .map_err(|e| match e {
                        kernel::DiagramAdmissionError::Boundary(e) => {
                            TransferError::Obligation(super::obligation::from_kernel(e))
                        }
                        kernel::DiagramAdmissionError::Resource(e) => TransferError::Resource(e),
                    })?;
                let (condition, reservation) = admitted.into_parts();
                existing.condition = condition;
                existing._condition_charge = reservation;
                existing.alternatives.insert(alternative);
            }
            None => {
                charge.grow(
                    size_of::<MergedTransfer<K>>()
                        + size_of::<Id<K::Alternative>>()
                        + 128
                        + branch.key().heap_bytes(),
                )?;
                let reservation = budget.reserve(
                    "transfer_condition",
                    branch.condition().allocation_allowance(),
                )?;
                merged.insert(
                    id,
                    MergedTransfer {
                        key: branch.key().clone(),
                        assumptions: branch.qualification().assumptions,
                        condition: branch.condition().clone(),
                        alternatives: BTreeSet::from([alternative]),
                        _condition_charge: reservation,
                    },
                );
            }
        }
    }
    charge.grow(merged.len() * size_of::<MergedTransfer<K>>())?;
    Ok(TransferAggregation {
        rows: merged.into_values().collect(),
        _charge: charge,
    })
}
fn frame_invariants<K: TransferKeyRecord>() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: K::NAME,
        inputs: vec![
            ValidationInput::of::<EntityRef>(&["id"]),
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<K>(&["id"]),
            ValidationInput::of::<K::Alternative>(&["id"]),
        ],
        create: Arc::new(|budget| {
            Box::new(FrameCheck::<K> {
                charge: StateCharge::new(budget, "transfer_frames"),
                entities: Default::default(),
                qualifications: Default::default(),
                keys: Default::default(),
            })
        }),
    }]
}
struct FrameCheck<K: TransferKeyRecord> {
    charge: StateCharge,
    entities: super::charged::ChargedSet<Id<EntityRef>>,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    keys: ChargedMap<Id<K>, K>,
}
impl<K: TransferKeyRecord> InvariantCheck for FrameCheck<K> {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == EntityRef::NAME {
            for row in EntityRef::decode(batch)? {
                if matches!(
                    row,
                    EntityRef::Module { .. } | EntityRef::Callable { .. } | EntityRef::Class { .. }
                ) {
                    self.entities.insert(&mut self.charge, row.id())?;
                }
            }
        } else if relation == AssertionQualification::NAME {
            for row in AssertionQualification::decode(batch)? {
                self.qualifications
                    .insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == K::NAME {
            for row in K::decode(batch)? {
                if !self.entities.contains(&row.descriptor().owner) {
                    return Err(invalid(
                        "transfer owner is not an admitted normalized execution entity",
                    ));
                }
                self.keys.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == K::Alternative::NAME {
            for row in K::Alternative::decode(batch)? {
                let key = self
                    .keys
                    .get(&row.transfer())
                    .ok_or_else(|| invalid("transfer alternative key absent"))?;
                let q = self
                    .qualifications
                    .get(&TransferAlternativeRecord::qualification(&row))
                    .ok_or_else(|| invalid("transfer qualification absent"))?;
                key.descriptor().check(q)?;
                if row.scope() != q.scope {
                    return Err(invalid("transfer alternative scope mismatch"));
                }
            }
        } else {
            return Err(invalid("undeclared transfer frame input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

pub trait ControlRecord: assertion::Assertion {
    fn atom(&self) -> Id<EvaluationAtom>;
    fn evaluation(&self) -> Id<Occurrence>;
}
pub trait SelectionRecord: Record {
    type Transfer: TransferKeyRecord;
    type Control: ControlRecord;
    fn influence(&self) -> Id<Self::Control>;
    fn atom(&self) -> Id<EvaluationAtom>;
    fn alternative(&self) -> Id<<Self::Transfer as TransferKeyRecord>::Alternative>;
    fn transfer(&self) -> Id<Self::Transfer>;
}
fn control_invariants<
    K: TransferKeyRecord,
    I: ControlRecord,
    S: SelectionRecord<Transfer = K, Control = I>,
>() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: S::NAME,
        inputs: vec![
            ValidationInput::of::<AssertionQualification>(&["id"]),
            ValidationInput::of::<EvaluationAtom>(&["id"]),
            ValidationInput::of::<ConditionNode>(&["id"]),
            ValidationInput::of::<Condition>(&["id"]),
            ValidationInput::of::<K>(&["id"]),
            ValidationInput::of::<K::Alternative>(&["id"]),
            ValidationInput::of::<I>(&["id"]),
            ValidationInput::of::<S>(&["id"]),
        ],
        create: Arc::new(|budget| {
            Box::new(ControlCheck::<K, I, S> {
                charge: StateCharge::new(budget, "control_selection"),
                qualifications: Default::default(),
                atoms: Default::default(),
                nodes: Default::default(),
                conditions: Default::default(),
                keys: Default::default(),
                alternatives: Default::default(),
                influences: Default::default(),
                _selection: std::marker::PhantomData,
            })
        }),
    }]
}
struct ControlCheck<
    K: TransferKeyRecord,
    I: ControlRecord,
    S: SelectionRecord<Transfer = K, Control = I>,
> {
    charge: StateCharge,
    qualifications: ChargedMap<Id<AssertionQualification>, AssertionQualification>,
    atoms: ChargedMap<Id<EvaluationAtom>, EvaluationAtom>,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    conditions: ChargedMap<Id<Condition>, BTreeSet<Id<EvaluationAtom>>>,
    keys: ChargedMap<Id<K>, K>,
    alternatives: ChargedMap<Id<K::Alternative>, K::Alternative>,
    influences: ChargedMap<Id<I>, I>,
    _selection: std::marker::PhantomData<S>,
}
impl<K: TransferKeyRecord, I: ControlRecord, S: SelectionRecord<Transfer = K, Control = I>>
    InvariantCheck for ControlCheck<K, I, S>
{
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
        } else if relation == EvaluationAtom::NAME {
            for row in EvaluationAtom::decode(batch)? {
                self.atoms.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == ConditionNode::NAME {
            for row in ConditionNode::decode(batch)? {
                self.nodes.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Condition::NAME {
            for row in Condition::decode(batch)? {
                let closure = kernel::closure(row.root, &self.nodes)?;
                let atoms = closure
                    .into_iter()
                    .filter_map(|id| match self.nodes[&id] {
                        ConditionNode::Branch { atom, .. } => Some(atom),
                        _ => None,
                    })
                    .collect();
                self.conditions.insert(&mut self.charge, row.id(), atoms)?;
            }
        } else if relation == K::NAME {
            for row in K::decode(batch)? {
                self.keys.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == K::Alternative::NAME {
            for row in K::Alternative::decode(batch)? {
                self.alternatives.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == I::NAME {
            for row in I::decode(batch)? {
                let q = self
                    .qualifications
                    .get(&row.qualification())
                    .ok_or_else(|| invalid("influence qualification absent"))?;
                let atom = self
                    .atoms
                    .get(&row.atom())
                    .ok_or_else(|| invalid("influence atom absent"))?;
                if atom.evaluation != row.evaluation() || atom.context != q.context {
                    return Err(invalid("influence evaluation/context mismatch"));
                }
                self.influences.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == S::NAME {
            for row in S::decode(batch)? {
                let i = self
                    .influences
                    .get(&row.influence())
                    .ok_or_else(|| invalid("selection influence absent"))?;
                let a = self
                    .alternatives
                    .get(&row.alternative())
                    .ok_or_else(|| invalid("selection alternative absent"))?;
                let iq = self
                    .qualifications
                    .get(&i.qualification())
                    .ok_or_else(|| invalid("selection influence qualification absent"))?;
                let aq = self
                    .qualifications
                    .get(&TransferAlternativeRecord::qualification(a))
                    .ok_or_else(|| invalid("selection alternative qualification absent"))?;
                if row.atom() != i.atom()
                    || row.transfer() != a.transfer()
                    || iq.context != aq.context
                    || iq.scope != aq.scope
                    || self
                        .conditions
                        .get(&aq.condition)
                        .is_none_or(|atoms| !atoms.contains(&row.atom()))
                {
                    return Err(invalid(
                        "selection does not connect influence to its guarded transfer",
                    ));
                }
            }
        } else {
            return Err(invalid("undeclared control input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        Ok(())
    }
}

