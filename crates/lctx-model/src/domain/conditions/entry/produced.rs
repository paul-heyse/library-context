//! Compact receipts from actual entry production. First access hydrates charged immutable
//! condition topology; repeated consumers borrow it without repeating flow/entry evaluation.
//! A stored witness never becomes an issuer.
use super::*;
use crate::domain::charged::{ChargedMap, ChargedSet, StateCharge};
use std::sync::{Arc, OnceLock};
#[derive(Clone, Debug, PartialEq)]
struct Receipt {
    source: EntryAccessSource,
    witness: EntryValueWitness,
    status: analysis::policy::EvidenceStatus,
    qualification: AssertionQualification,
    condition: Id<super::super::Condition>,
    parameter: Id<calls::SignatureParameter>,
    declaration: Id<Occurrence>,
    place: Place,
}
impl HeapSize for Receipt {}
struct IssuedEntry {
    receipt: Receipt,
    // Lazy, entry-local computational reuse. The value owns its reservation, including when
    // a borrower survives this receipt owner; this is not another source of entry authority.
    hydrated: OnceLock<Arc<DerivedEntryValue>>,
}
impl HeapSize for IssuedEntry {}
pub struct ProducedEntries {
    entries: ChargedMap<Id<EntryValueWitness>, IssuedEntry>,
    conditions: ChargedMap<Id<super::super::Condition>, super::super::Condition>,
    nodes: ChargedMap<Id<super::super::ConditionNode>, super::super::ConditionNode>,
    charge: StateCharge,
}
impl ProducedEntries {
    pub(crate) fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            entries: Default::default(),
            conditions: Default::default(),
            nodes: Default::default(),
            charge: StateCharge::new(budget, "actual-local-entry-receipts"),
        }
    }
    pub(crate) fn capture(&mut self, value: &DerivedEntryValue) -> Result<(), ModelError> {
        let budget = self.charge.budget().expect("bound actual entry receipts");
        let _encode = budget.reserve(
            "actual-entry-condition-encoding",
            value
                .condition()
                .allocation_allowance()
                .saturating_mul(4)
                .saturating_add(4096),
        )?;
        let (condition, nodes) = value.condition().records();
        let PlaceRoot::Entry { declaration } = value.root() else {
            return Err(ModelError::Invalid(
                "actual Local entry root differs from parameter entry".into(),
            ));
        };
        let receipt = Receipt {
            source: value.source().clone(),
            witness: value.witness().clone(),
            status: value.evidence_status(),
            qualification: value.qualification().clone(),
            condition: condition.id(),
            parameter: value.parameter(),
            declaration: *declaration,
            place: value.place().clone(),
        };
        if self
            .entries
            .get(&receipt.witness.id())
            .is_some_and(|previous| previous.receipt != receipt)
        {
            return Err(ModelError::Conflict("actual Local entry receipt"));
        }
        self.conditions
            .insert(&mut self.charge, condition.id(), condition)?;
        for node in nodes {
            self.nodes.insert(&mut self.charge, node.id(), node)?;
        }
        // Recapture of the exact issuer preserves any active immutable hydration.
        if !self.entries.contains_key(&receipt.witness.id()) {
            self.entries.insert(&mut self.charge, receipt.witness.id(), IssuedEntry {
                receipt, hydrated: OnceLock::new(),
            })?;
        }
        Ok(())
    }
    pub fn require(
        &self,
        witness: &EntryValueWitness,
        source: &EntryAccessSource,
        budget: &resources::ResourceBudget,
    ) -> Result<(), ModelError> {
        if !self
            .charge
            .budget()
            .is_some_and(|owner| owner.shares_pool(budget))
        {
            return Err(ModelError::Invalid(
                "actual Local entry belongs to another attempt budget".into(),
            ));
        }
        let issued = self
            .entries
            .get(&witness.id())
            .ok_or_else(|| ModelError::Invalid("entry has no actual Local issuer".into()))?;
        if &issued.receipt.witness != witness || &issued.receipt.source != source {
            return Err(ModelError::Conflict("actual Local entry descriptor"));
        }
        Ok(())
    }
    pub fn get(
        &self,
        witness: &EntryValueWitness,
        source: &EntryAccessSource,
        budget: &resources::ResourceBudget,
    ) -> Result<Arc<DerivedEntryValue>, ModelError> {
        if !self
            .charge
            .budget()
            .is_some_and(|owner| owner.shares_pool(budget))
        {
            return Err(ModelError::Invalid(
                "actual Local entry belongs to another attempt budget".into(),
            ));
        }
        let issued = self
            .entries
            .get(&witness.id())
            .ok_or_else(|| ModelError::Invalid("entry has no actual Local issuer".into()))?;
        let receipt = &issued.receipt;
        if &receipt.witness != witness || &receipt.source != source {
            return Err(ModelError::Conflict("actual Local entry descriptor"));
        }
        if let Some(hydrated) = issued.hydrated.get() {
            return Ok(hydrated.clone());
        }
        let condition = self
            .conditions
            .get(&receipt.condition)
            .ok_or(ModelError::Schema("actual Local entry condition"))?;
        let mut visited = ChargedSet::default();
        let mut selected = Vec::new();
        let mut scratch = StateCharge::new(budget, "actual-local-entry-hydration-scratch");
        let mut pending = vec![condition.root];
        scratch.grow(size_of::<Id<super::super::ConditionNode>>())?;
        while let Some(id) = pending.pop() {
            if !visited.insert(&mut scratch, id)? {
                continue;
            }
            let node = self
                .nodes
                .get(&id)
                .ok_or(ModelError::Schema("actual Local entry condition node"))?;
            if let super::super::ConditionNode::Branch { low, high, .. } = node {
                pending.push(*low);
                pending.push(*high);
                scratch.grow(2 * size_of::<Id<super::super::ConditionNode>>())?;
            }
            scratch.grow(size_of::<super::super::ConditionNode>().saturating_add(2048))?;
            selected.push(node.clone());
        }
        let condition = super::super::Diagram::from_records(condition, &selected)?;
        let mut charge = StateCharge::new(budget, "actual-local-entry-hydration");
        charge.grow(
            condition
                .allocation_allowance()
                .saturating_add(size_of::<DerivedEntryValue>())
                .saturating_add(4 * size_of::<usize>()),
        )?;
        drop(selected);
        drop(visited);
        drop(pending);
        drop(scratch);
        let hydrated = Arc::new(DerivedEntryValue {
            source: receipt.source.clone(),
            witness: receipt.witness.clone(),
            status: receipt.status,
            qualification: receipt.qualification.clone(),
            condition,
            parameter: receipt.parameter,
            root: PlaceRoot::Entry {
                declaration: receipt.declaration,
            },
            place: receipt.place.clone(),
            _charge: std::sync::Arc::new(charge),
        });
        // Concurrent first borrowers can prepare independently. Only the published immutable
        // value survives; a losing preparation releases its own charge before returning.
        let _ = issued.hydrated.set(hydrated);
        Ok(issued.hydrated.get().expect("entry hydration published").clone())
    }
    pub(crate) fn append(&mut self, other: Self) -> Result<(), ModelError> {
        if !self
            .charge
            .budget()
            .zip(other.charge.budget())
            .is_some_and(|(a, b)| a.shares_pool(b))
        {
            return Err(ModelError::Invalid(
                "actual Local entries cross attempt budgets".into(),
            ));
        }
        let Self {
            mut entries,
            mut conditions,
            mut nodes,
            mut charge,
        } = other;
        while let Some(id) = entries.keys().next().copied() {
            let value = entries.remove(&mut charge, &id).expect("entry");
            if self
                .entries
                .get(&id)
                .is_some_and(|previous| previous.receipt != value.receipt)
            {
                return Err(ModelError::Conflict("actual Local entry receipt"));
            }
            if !self.entries.contains_key(&id) {
                self.entries.insert(&mut self.charge, id, value)?;
            }
        }
        while let Some(id) = conditions.keys().next().copied() {
            let value = conditions.remove(&mut charge, &id).expect("condition");
            self.conditions.insert(&mut self.charge, id, value)?;
        }
        while let Some(id) = nodes.keys().next().copied() {
            let value = nodes.remove(&mut charge, &id).expect("node");
            self.nodes.insert(&mut self.charge, id, value)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod actual_entry_controls {
    use super::*;
    fn nominal<R>(value: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    fn actual(budget: &resources::ResourceBudget) -> DerivedEntryValue {
        let source = EntryAccessSource::Use {
            observation: nominal(1),
            support: nominal(2),
        };
        let condition = super::super::super::Diagram::always();
        let qualification = AssertionQualification {
            context: nominal(3),
            scope: nominal(4),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let witness = EntryValueWitness {
            owner: nominal(5),
            formal: nominal(6),
            access: nominal(7),
            context: qualification.context,
            run: nominal(8),
            access_source: source.id(),
            link: nominal(9),
            declaration_support: nominal(10),
            owner_support: nominal(11),
            use_observation: nominal(12),
            use_support: nominal(13),
            inventory: nominal(14),
            inventory_support: nominal(15),
            reaching: nominal(16),
            reaching_support: nominal(17),
            definition: nominal(18),
            definition_support: nominal(19),
            parameter_placement: None,
            parameter_placement_support: None,
            coverage: nominal(20),
        };
        let root = PlaceRoot::Entry {
            declaration: nominal(21),
        };
        let place = Place {
            root: root.id(),
            path: AccessPath::empty().id(),
        };
        let mut charge = StateCharge::new(budget, "actual-entry-control");
        charge.grow(condition.allocation_allowance()).unwrap();
        DerivedEntryValue {
            source,
            witness,
            qualification,
            condition,
            parameter: nominal(22),
            root,
            place,
            status: analysis::policy::EvidenceStatus::StructurallyObserved,
            _charge: std::sync::Arc::new(charge),
        }
    }
    #[test]
    fn actual_entry_receipt_rehydrates_without_flow_and_refuses_foreign_or_changed_claims() {
        let budget = resources::ResourceBudget::fixed(128 << 10).unwrap();
        let foreign = resources::ResourceBudget::fixed(128 << 10).unwrap();
        let value = actual(&budget);
        let witness = value.witness().clone();
        let source = value.source().clone();
        let mut issued = ProducedEntries::new(&budget);
        assert!(issued.get(&witness, &source, &budget).is_err());
        issued.capture(&value).unwrap();
        let condition = value.condition().id();
        let parameter = value.parameter();
        drop(value);
        let hydrated = issued.get(&witness, &source, &budget).unwrap();
        assert_eq!(hydrated.parameter(), parameter);
        assert_eq!(hydrated.condition().id(), condition);
        assert_eq!(hydrated.witness(), &witness);
        assert!(issued.get(&witness, &source, &foreign).is_err());
        let changed = EntryValueWitness {
            coverage: nominal(99),
            ..witness.clone()
        };
        assert!(issued.get(&changed, &source, &budget).is_err());
        let source_changed = EntryAccessSource::Use {
            observation: nominal(1),
            support: nominal(98),
        };
        assert!(issued.get(&witness, &source_changed, &budget).is_err());
        let mut empty = ProducedEntries::new(&foreign);
        assert!(empty.append(issued).is_err());
        drop(hydrated);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn repeated_entry_borrows_share_hydration_and_hold_charge_after_owner_release() {
        let budget = resources::ResourceBudget::fixed(128 << 10).unwrap();
        let value = actual(&budget);
        let witness = value.witness().clone();
        let source = value.source().clone();
        let mut issued = ProducedEntries::new(&budget);
        issued.capture(&value).unwrap();
        let before_access = budget.reserved();
        let first = issued.get(&witness, &source, &budget).unwrap();
        assert!(budget.reserved() > before_access);
        // Exact recapture and append preserve the already admitted immutable entry.
        issued.capture(&value).unwrap();
        let mut duplicate = ProducedEntries::new(&budget);
        duplicate.capture(&value).unwrap();
        issued.append(duplicate).unwrap();
        drop(value);
        let after_first = budget.reserved();
        let second = issued.get(&witness, &source, &budget).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(budget.reserved(), after_first);
        // A repeated consumer needs no new hydration allowance even under pool pressure.
        let pressure = budget.reserve("entry-borrow-control-pressure", budget.limit() - budget.reserved()).unwrap();
        let third = issued.get(&witness, &source, &budget).unwrap();
        assert!(Arc::ptr_eq(&first, &third));
        drop(pressure);
        drop(issued);
        assert!(budget.reserved() > 0, "live borrower still owns decoded state");
        drop(first);
        drop(second);
        assert!(budget.reserved() > 0);
        drop(third);
        assert_eq!(budget.reserved(), 0);
    }

    #[test]
    fn refused_entry_hydration_can_retry_without_a_published_partial_value() {
        let budget = resources::ResourceBudget::fixed(128 << 10).unwrap();
        let value = actual(&budget);
        let witness = value.witness().clone();
        let source = value.source().clone();
        let mut issued = ProducedEntries::new(&budget);
        issued.capture(&value).unwrap();
        drop(value);
        let pressure = budget.reserve("entry-hydration-control-pressure", budget.limit() - budget.reserved()).unwrap();
        assert!(issued.get(&witness, &source, &budget).is_err());
        assert!(issued.entries.get(&witness.id()).unwrap().hydrated.get().is_none());
        drop(pressure);
        let hydrated = issued.get(&witness, &source, &budget).unwrap();
        assert_eq!(hydrated.witness(), &witness);
        drop(issued);
        drop(hydrated);
        assert_eq!(budget.reserved(), 0);
    }
}
