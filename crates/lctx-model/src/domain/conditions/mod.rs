//! Occurrence-keyed conditions. The sole persisted truth representation is a reduced ordered BDD.
pub(super) mod kernel;
mod substitution;
pub use kernel::{CondExpr, Diagram, FactorResult, KernelBoundary, RenderedCondition};
use crate::{Domain, DomainSum};
use super::{Id, ModelError, Record};
use super::{source::Occurrence, attribution::AnalysisContext, value::{Place, Predicate}};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "evaluation_atoms")]
pub struct EvaluationAtom {
    #[model(key)] pub evaluation: Id<Occurrence>,
    #[model(key)] pub context: Id<AnalysisContext>,
    #[model(key)] pub predicate: Id<Predicate>,
    #[model(key)] pub operand: Option<Id<Place>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "condition_nodes", validate = validate_node)]
pub enum ConditionNode {
    #[model(code = 0)] False,
    #[model(code = 1)] True,
    #[model(code = 2)] Branch { atom: Id<EvaluationAtom>, low: Id<ConditionNode>, high: Id<ConditionNode> },
}
fn validate_node(row: &ConditionNode) -> Result<(), ModelError> {
    if let ConditionNode::Branch { low, high, .. } = row {
        if low == high { return Err(ModelError::Invalid("condition branch must be reduced".into())); }
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "conditions", invariants = condition_invariants)]
pub struct Condition { #[model(key)] pub root: Id<ConditionNode> }

fn condition_invariants() -> Vec<super::Invariant> {
    vec![super::Invariant { name: "canonical_condition_catalog", inputs: vec![
        super::ValidationInput::of::<ConditionNode>(&["id"]),
        super::ValidationInput::of::<Condition>(&["id"]),
    ], create: || Box::new(CatalogCheck::default()) }]
}
#[derive(Default)]
struct CatalogCheck {
    nodes: std::collections::BTreeMap<Id<ConditionNode>, ConditionNode>,
    reached: std::collections::BTreeSet<Id<ConditionNode>>,
    visits: usize,
    conditions: usize,
}
impl super::InvariantCheck for CatalogCheck {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == ConditionNode::NAME {
            for row in ConditionNode::decode(batch)? {
                if self.nodes.len() >= 100_000 { return Err(ModelError::Invalid("condition catalog node budget exceeded".into())); }
                if self.nodes.insert(row.id(), row).is_some() { return Err(ModelError::Conflict(ConditionNode::NAME)); }
            }
        } else if relation == Condition::NAME {
            for row in Condition::decode(batch)? {
                self.conditions += 1;
                if self.conditions > 100_000 { return Err(ModelError::Invalid("condition catalog root budget exceeded".into())); }
                let closure = kernel::closure(row.root, &self.nodes)?;
                self.visits += closure.len();
                if self.visits > 1_000_000 { return Err(ModelError::Invalid("condition catalog closure budget exceeded".into())); }
                self.reached.extend(closure);
            }
        } else { return Err(ModelError::Invalid("undeclared condition validation input".into())); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.reached.len() != self.nodes.len() { return Err(ModelError::Invalid("condition catalog contains unreferenced nodes".into())); }
        Ok(())
    }
}
