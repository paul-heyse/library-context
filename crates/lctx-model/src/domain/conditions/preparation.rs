//! Checked, invocation-owned condition lowering over one exact immutable node inventory.
use super::{AdmittedDiagram, Condition, ConditionNode, Diagram};
use crate::domain::{
    Id, ModelError, Record,
    resources::{Reservation, ResourceBudget},
};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

pub struct PreparedConditions<'a> {
    nodes: BTreeMap<Id<ConditionNode>, &'a ConditionNode>,
    diagrams: RefCell<BTreeMap<Id<Condition>, Arc<AdmittedDiagram>>>,
    budget: ResourceBudget,
    _reservation: Box<dyn Reservation>,
}
impl<'a> PreparedConditions<'a> {
    pub fn new(
        nodes: impl ExactSizeIterator<Item = &'a ConditionNode>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        if nodes.len() > 100_000 {
            return Err(ModelError::Invalid(
                "condition node admission exceeded".into(),
            ));
        }
        let reservation = budget.reserve(
            "condition-inventory",
            nodes
                .len()
                .checked_mul(256)
                .ok_or_else(|| ModelError::Invalid("condition inventory overflow".into()))?,
        )?;
        let mut catalog = BTreeMap::new();
        for node in nodes {
            node.validate()?;
            if catalog.insert(node.id(), node).is_some() {
                return Err(ModelError::Conflict(ConditionNode::NAME));
            }
        }
        Ok(Self {
            nodes: catalog,
            diagrams: RefCell::new(BTreeMap::new()),
            budget: budget.clone(),
            _reservation: reservation,
        })
    }
    /// The root's Merkle identity fixes its effective support in this validated inventory.
    /// The selected model/kernel is the current binary; this private cache never crosses it.
    pub fn diagram(&self, condition: &Condition) -> Result<Arc<AdmittedDiagram>, ModelError> {
        if let Some(diagram) = self.diagrams.borrow().get(&condition.id()) {
            return Ok(Arc::clone(diagram));
        }
        // Admit closure traversal, raw BDD construction, correspondence and retained map entry
        // before lowering. Shrink only after transient construction state has been dropped.
        let mut reservation = self.budget.reserve(
            "condition-preparation",
            self.nodes
                .len()
                .checked_mul(2048)
                .and_then(|n| n.checked_add(4096))
                .ok_or_else(|| ModelError::Invalid("condition preparation overflow".into()))?,
        )?;
        let diagram = Diagram::from_catalog(condition, &self.nodes)?;
        reservation.try_resize(diagram.allocation_allowance().saturating_add(256))?;
        let value = Arc::new(AdmittedDiagram::from_parts(diagram, reservation));
        self.diagrams
            .borrow_mut()
            .insert(condition.id(), Arc::clone(&value));
        Ok(value)
    }
}
