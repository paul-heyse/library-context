//! Bounded condition checks for source-to-summary paths (DESIGN §9.9).
//!
//! These facts decide only compatibility under the declared evaluation atoms. They neither
//! choose a reaching definition nor promote a modeled call to a completed transfer.

use std::collections::{BTreeSet, HashMap};

use cpg_schema::behavior::{
    ValueFlowPredecessorCandidatesRow, ValueFlowPredecessorCompatibilityRow,
};
use cpg_schema::codebook::BoundaryReason;
use cpg_schema::condition_kernel::Diagram;
use cpg_schema::id::Id;
use petgraph::Directed;
use petgraph::algo::tarjan_scc;
use petgraph::graph::{Graph, NodeIndex};

use crate::AnalyticsError;

/// One component of an attributed caller→callee graph, scheduled after its callees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallComponent {
    pub members: Vec<Id>,
    pub recursive: bool,
}

/// Canonical callee-first SCC schedule. Petgraph owns the SCC algorithm; this wrapper sorts
/// members and chooses ties by canonical id instead of relying on graph insertion order.
pub fn call_components(
    functions: &[Id],
    calls: &[(Id, Id)],
) -> Result<Vec<CallComponent>, AnalyticsError> {
    let mut ids = functions.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let mut edges = calls.to_vec();
    edges.sort_unstable();
    edges.dedup();
    let mut graph: Graph<(), (), Directed, u32> = Graph::with_capacity(ids.len(), edges.len());
    for _ in &ids {
        graph.try_add_node(()).map_err(|e| AnalyticsError::Graph(e.to_string()))?;
    }
    for &(caller, callee) in &edges {
        let src = ids.binary_search(&caller)
            .map_err(|_| AnalyticsError::UnknownVertex(caller.hex()))?;
        let dst = ids.binary_search(&callee)
            .map_err(|_| AnalyticsError::UnknownVertex(callee.hex()))?;
        graph.try_add_edge(NodeIndex::new(src), NodeIndex::new(dst), ())
            .map_err(|e| AnalyticsError::Graph(e.to_string()))?;
    }
    let mut components: Vec<CallComponent> = tarjan_scc(&graph)
        .into_iter()
        .map(|component| {
            let mut members: Vec<_> = component.into_iter().map(|node| ids[node.index()]).collect();
            members.sort_unstable();
            CallComponent { recursive: members.len() > 1, members }
        })
        .collect();
    let mut component_of = vec![0_usize; ids.len()];
    for (index, component) in components.iter().enumerate() {
        for member in &component.members {
            let vertex = ids.binary_search(member)
                .map_err(|_| AnalyticsError::UnknownVertex(member.hex()))?;
            component_of[vertex] = index;
        }
    }
    let mut downstream: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); components.len()];
    let mut upstream: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); components.len()];
    for &(caller, callee) in &edges {
        let src = ids.binary_search(&caller)
            .map_err(|_| AnalyticsError::UnknownVertex(caller.hex()))?;
        let dst = ids.binary_search(&callee)
            .map_err(|_| AnalyticsError::UnknownVertex(callee.hex()))?;
        let a = component_of[src];
        let b = component_of[dst];
        if a == b {
            if caller == callee {
                components[a].recursive = true;
            }
        } else if downstream[a].insert(b) {
            upstream[b].insert(a);
        }
    }
    let mut ready: BTreeSet<(Id, usize)> = components.iter().enumerate()
        .filter(|(index, _)| downstream[*index].is_empty())
        .map(|(index, component)| (component.members[0], index))
        .collect();
    let mut ordered = Vec::with_capacity(components.len());
    while let Some((_, index)) = ready.pop_first() {
        ordered.push(components[index].clone());
        for &caller in &upstream[index] {
            downstream[caller].remove(&index);
            if downstream[caller].is_empty() {
                ready.insert((components[caller].members[0], caller));
            }
        }
    }
    if ordered.len() != components.len() {
        return Err(AnalyticsError::Graph("SCC condensation is cyclic".to_owned()));
    }
    Ok(ordered)
}

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
    fn call_sccs_are_callee_first_and_input_order_independent() {
        let [a, b, c, d, e] = [1, 2, 3, 4, 5].map(|n| Id([n; 16]));
        let functions = [e, c, a, d, b];
        let calls = [(a, b), (b, a), (a, c), (c, d), (a, c), (e, e)];
        let actual = call_components(&functions, &calls).unwrap();
        assert_eq!(actual, vec![
            CallComponent { members: vec![d], recursive: false },
            CallComponent { members: vec![c], recursive: false },
            CallComponent { members: vec![a, b], recursive: true },
            CallComponent { members: vec![e], recursive: true },
        ]);
        let mut reversed = calls;
        reversed.reverse();
        assert_eq!(call_components(&[a, b, c, d, e], &reversed).unwrap(), actual);
        assert!(matches!(
            call_components(&[a], &[(a, b)]),
            Err(AnalyticsError::UnknownVertex(_))
        ));
    }

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
