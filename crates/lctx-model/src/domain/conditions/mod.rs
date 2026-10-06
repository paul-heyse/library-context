//! Occurrence-keyed conditions. The sole persisted truth representation is a reduced ordered BDD.
pub mod graph;
pub(super) mod kernel;
pub(crate) mod substitution;
pub use graph::{CondGraph, CondNode, GraphCondition};
pub mod entry;
pub mod rebase;
pub mod stability;
use super::charged::{ChargedMap, ChargedSet, StateCharge};
use super::{Id, ModelError, Record};
use super::{
    attribution::AnalysisContext,
    source::Occurrence,
    value::{Place, Predicate},
};
use crate::{Domain, DomainSum};
pub use kernel::{
    AdmittedDiagram, BooleanOperation, CondExpr, Diagram, DiagramAdmissionError, FactorResult,
    KernelBoundary, RenderedCondition,
};

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "evaluation_atoms", invariant_refs = rebase::guard_invariants_refs)]
pub struct EvaluationAtom {
    #[model(key)]
    pub evaluation: Id<Occurrence>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    #[model(key)]
    pub predicate: Id<Predicate>,
    #[model(key)]
    pub operand: Option<Id<Place>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "condition_nodes", validate = validate_node)]
pub enum ConditionNode {
    #[model(code = 0)]
    False,
    #[model(code = 1)]
    True,
    #[model(code = 2)]
    Branch {
        atom: Id<EvaluationAtom>,
        low: Id<ConditionNode>,
        high: Id<ConditionNode>,
    },
}
fn validate_node(row: &ConditionNode) -> Result<(), ModelError> {
    if let ConditionNode::Branch { low, high, .. } = row
        && low == high
    {
        return Err(ModelError::Invalid(
            "condition branch must be reduced".into(),
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "conditions", invariant_refs = condition_invariants_refs)]
pub struct Condition {
    #[model(key)]
    pub root: Id<ConditionNode>,
}

pub(crate) fn condition_invariants() -> Vec<super::Invariant> {
    vec![super::Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 1,
        name: "canonical_condition_catalog",
        inputs: vec![
            super::ValidationInput::of::<ConditionNode>(&["id"]),
            super::ValidationInput::of::<Condition>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(CatalogCheck {
                charge: StateCharge::new(budget, "canonical_condition_catalog"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Default)]
struct CatalogCheck {
    charge: StateCharge,
    nodes: ChargedMap<Id<ConditionNode>, ConditionNode>,
    reached: ChargedSet<Id<ConditionNode>>,
}
impl super::InvariantCheck for CatalogCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == ConditionNode::NAME {
            for row in ConditionNode::decode(batch)? {
                if self
                    .nodes
                    .insert(&mut self.charge, row.id(), row)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(ConditionNode::NAME));
                }
            }
        } else if relation == Condition::NAME {
            for row in Condition::decode(batch)? {
                // Each root's closure is bounded by the kernel's node and atom limits.
                for id in kernel::closure(row.root, &self.nodes)? {
                    self.reached.insert(&mut self.charge, id)?;
                }
            }
        } else {
            return Err(ModelError::Invalid(
                "undeclared condition validation input".into(),
            ));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.reached.len() != self.nodes.len() {
            return Err(ModelError::Invalid(
                "condition catalog contains unreferenced nodes".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn condition_invariants_refs() -> Vec<&'static str> {
    vec!["canonical_condition_catalog"]
}
