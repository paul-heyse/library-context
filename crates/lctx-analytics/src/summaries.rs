//! Bounded condition checks for source-to-summary paths (DESIGN §9.9).
//!
//! These facts decide only compatibility under the declared evaluation atoms. They neither
//! choose a reaching definition nor promote a modeled call to a completed transfer.

use std::collections::HashMap;

use cpg_schema::behavior::{
    ValueFlowPredecessorCandidatesRow, ValueFlowPredecessorCompatibilityRow,
};
use cpg_schema::codebook::BoundaryReason;
use cpg_schema::condition_kernel::Diagram;
use cpg_schema::id::Id;

pub fn predecessor_compatibility(
    candidates: &[ValueFlowPredecessorCandidatesRow],
    conditions: &HashMap<Id, Diagram>,
    boundaries: &HashMap<Id, BoundaryReason>,
) -> Vec<ValueFlowPredecessorCompatibilityRow> {
    candidates
        .iter()
        .map(|edge| {
            let decision = if edge.loop_carried {
                Err(BoundaryReason::UnsupportedControlFlow)
            } else {
                let roots = [
                    edge.predecessor_condition_id,
                    edge.reaching_condition_id,
                    edge.successor_condition_id,
                ];
                if let [Some(a), Some(b), Some(c)] = roots.map(|id| conditions.get(&id)) {
                    a.and(b)
                        .and_then(|ab| ab.and(c))
                        .map(|all| !all.is_false())
                        .map_err(|_| BoundaryReason::BudgetReached)
                } else {
                    Err(roots
                        .iter()
                        .filter_map(|id| boundaries.get(id).copied())
                        .find(|reason| *reason == BoundaryReason::BudgetReached)
                        .or_else(|| roots.iter().find_map(|id| boundaries.get(id).copied()))
                        .unwrap_or(BoundaryReason::MissingEvidence))
                }
            };
            ValueFlowPredecessorCompatibilityRow {
                snapshot_id: edge.snapshot_id,
                successor_fact_id: edge.successor_fact_id,
                predecessor_fact_id: edge.predecessor_fact_id,
                source_key: edge.source_key.clone(),
                reaching_fact_id: edge.reaching_fact_id,
                predecessor_condition_id: edge.predecessor_condition_id,
                reaching_condition_id: edge.reaching_condition_id,
                successor_condition_id: edge.successor_condition_id,
                compatible_under_atoms: decision.as_ref().ok().copied(),
                boundary_reason: decision.err(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpg_schema::condition::Condition;

    #[test]
    fn a_contradictory_reaching_path_is_refuted_only_within_one_iteration() {
        let positive = Diagram::from_condition(&Condition::parse("truthy(z)").unwrap()).unwrap();
        let negative = positive.not().unwrap();
        let always = Diagram::always();
        let edge = ValueFlowPredecessorCandidatesRow {
            snapshot_id: Id([1; 16]),
            successor_fact_id: Id([2; 16]),
            predecessor_fact_id: Id([3; 16]),
            source_key: "Parameter[x]".to_owned(),
            parameter_node_id: Id([4; 16]),
            function_node_id: Id([5; 16]),
            successor_use_id: Id([6; 16]),
            predecessor_use_id: Id([7; 16]),
            reaching_fact_id: Id([8; 16]),
            reaching_condition_id: negative.id(),
            reaching_approximated: false,
            loop_carried: false,
            definition_id: Id([9; 16]),
            definition_fact_id: Id([10; 16]),
            successor_condition_id: always.id(),
            predecessor_condition_id: positive.id(),
            successor_raw_approximated: false,
            predecessor_raw_approximated: false,
            predecessor_local_through_call: true,
            predecessor_upstream_through_call: false,
        };
        let conditions = HashMap::from([
            (positive.id(), positive),
            (negative.id(), negative),
            (always.id(), always),
        ]);
        let result = predecessor_compatibility(std::slice::from_ref(&edge), &conditions, &HashMap::new());
        assert_eq!(result[0].compatible_under_atoms, Some(false));
        assert_eq!(result[0].boundary_reason, None);

        let mut capped_conditions = conditions.clone();
        capped_conditions.remove(&edge.predecessor_condition_id);
        let capped = HashMap::from([(edge.predecessor_condition_id, BoundaryReason::BudgetReached)]);
        let result = predecessor_compatibility(std::slice::from_ref(&edge), &capped_conditions, &capped);
        assert_eq!(result[0].compatible_under_atoms, None);
        assert_eq!(result[0].boundary_reason, Some(BoundaryReason::BudgetReached));

        let carried = ValueFlowPredecessorCandidatesRow { loop_carried: true, ..edge };
        let result = predecessor_compatibility(&[carried], &conditions, &HashMap::new());
        assert_eq!(result[0].compatible_under_atoms, None);
        assert_eq!(result[0].boundary_reason, Some(BoundaryReason::UnsupportedControlFlow));
    }
}
