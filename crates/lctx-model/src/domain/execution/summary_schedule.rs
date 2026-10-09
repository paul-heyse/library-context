//! Canonical callee-first SCC order over a borrowed generation-owned invocation graph.
use crate::domain::{
    Id, ModelError,
    normalized::entities::EntityRef,
    projection::{ProjectionName, snapshot::{MaterializationIdentity, MaterializedGraph}},
    resources::{Reservation, ResourceBudget},
};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::collections::{BTreeMap, BTreeSet};

/// The retained schedule owns its reservation and an exact materialization binding, never
/// graph storage or runtime indices. Equivalent rebuilt topology receives its own schedule.
pub struct SccSchedule {
    components: Vec<Vec<Id<EntityRef>>>,
    materialization: MaterializationIdentity,
    _reservation: Box<dyn Reservation>,
}
impl SccSchedule {
    pub fn require_graph(&self, graph: &MaterializedGraph) -> Result<(), ModelError> {
        if !self.materialization.matches(graph.materialization_identity()) {
            return Err(ModelError::Conflict("SCC schedule materialization binding"));
        }
        Ok(())
    }
    pub fn components(&self) -> &[Vec<Id<EntityRef>>] {
        &self.components
    }
}

fn reserve(graph: &MaterializedGraph, budget: &ResourceBudget) -> Result<Box<dyn Reservation>, ModelError> {
    budget.reserve("invocation-scc-schedule", graph.vertex_count().saturating_mul(256)
        .saturating_add(graph.arc_count().saturating_mul(96)).saturating_add(4096))
}
/// This orders known conservative topology; it does not certify dispatch/coverage completeness.
pub fn invocation_sccs(
    graph: &MaterializedGraph,
    budget: &ResourceBudget,
) -> Result<SccSchedule, ModelError> {
    if graph.key().name != ProjectionName::CallableInvocation {
        return Err(ModelError::Invalid(
            "summary schedule requires the invocation projection".into(),
        ));
    }
    let reservation = reserve(graph, budget)?;
    let components = graph.with_native_graph(|view| {
        let components = petgraph::algo::kosaraju_scc(view)
            .into_iter()
            .map(|component| {
                component
                    .into_iter()
                    .map(|node| view.entity_id(node))
                    .collect()
            })
            .collect();
        canonical_order(
            components,
            view.edge_references()
                .map(|edge| (view.entity_id(edge.source()), view.entity_id(edge.target()))),
        )
    })?;
    Ok(SccSchedule {
        components,
        materialization: graph.materialization_identity().clone(),
        _reservation: reservation,
    })
}

fn canonical_order<N: Copy + Ord>(
    mut components: Vec<Vec<N>>,
    edges: impl Iterator<Item = (N, N)>,
) -> Result<Vec<Vec<N>>, ModelError> {
    let invalid = || ModelError::Invalid("invalid SCC partition or condensation".into());
    let mut membership = BTreeMap::new();
    for (index, component) in components.iter_mut().enumerate() {
        if component.is_empty() {
            return Err(invalid());
        }
        component.sort();
        for node in component.iter() {
            if membership.insert(*node, index).is_some() {
                return Err(invalid());
            }
        }
    }
    let mut callees = vec![BTreeSet::new(); components.len()];
    let mut callers = vec![BTreeSet::new(); components.len()];
    for (source, target) in edges {
        let source = *membership.get(&source).ok_or_else(invalid)?;
        let target = *membership.get(&target).ok_or_else(invalid)?;
        if source != target {
            callees[source].insert(target);
            callers[target].insert(source);
        }
    }
    let mut ready = callees
        .iter()
        .enumerate()
        .filter(|(_, set)| set.is_empty())
        .map(|(index, _)| (components[index][0], index))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(components.len());
    while let Some((_, index)) = ready.pop_first() {
        for &caller in &callers[index] {
            callees[caller].remove(&index);
            if callees[caller].is_empty() {
                ready.insert((components[caller][0], caller));
            }
        }
        order.push(std::mem::take(&mut components[index]));
    }
    if order.len() != components.len() {
        return Err(invalid());
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::canonical_order;
    #[test]
    fn public_schedule_keeps_partial_projection_isolates_and_owns_its_reservation() {
        use super::*;
        use crate::domain::{
            ContentHash, Record,
            attribution::{AnalysisContext, FactFamily, Provider, ProviderRun},
            input::InputRevision,
            projection::normalization::{ProjectionData, ProjectionKey, describe},
            source::{CoverageScope, Module, SourceArtifact},
        };
        let budget = ResourceBudget::fixed(8 * 1024 * 1024).unwrap();
        let input = InputRevision::from_entries(vec![]).unwrap();
        let context = AnalysisContext {
            python_version: "3.14".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"config"),
            environment_digest: ContentHash::of(b"environment"),
            lock_digest: None,
        };
        let provider = Provider {
            tool: "control".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"provider"),
        };
        let (run, _) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            ContentHash::of(b"configuration"),
            [FactFamily::Calls],
        )
        .unwrap();
        let mut data = ProjectionData::new(&budget);
        data.runs.insert(run).unwrap();
        data.scopes
            .insert(CoverageScope::Input { input: input.id() })
            .unwrap();
        let mut expected = Vec::new();
        for name in ["z", "a", "m"] {
            let source = SourceArtifact::from_bytes(input.id(), format!("{name}.py"), b"").unwrap();
            let module = Module {
                source: source.id(),
                qualified_name: name.into(),
            };
            let entity = EntityRef::Module {
                module: module.id(),
            };
            expected.push(vec![entity.id()]);
            data.artifacts.insert(source).unwrap();
            data.modules.insert(module).unwrap();
            data.refs.insert(entity).unwrap();
        }
        expected.sort();
        let key = ProjectionKey {
            input: input.id(),
            context: context.id(),
            name: ProjectionName::CallableInvocation,
        };
        let projection = describe(&data, key, &budget).unwrap();
        let before_graph = budget.reserved();
        let graph = MaterializedGraph::build(&projection, &budget).unwrap();
        let before = budget.reserved();
        let graph_charge = before - before_graph;
        let schedule = invocation_sccs(&graph, &budget).unwrap();
        assert_eq!(schedule.components(), expected);
        assert!(budget.reserved() > before);
        schedule.require_graph(&graph).unwrap();
        let rebuilt = MaterializedGraph::build(&projection, &budget).unwrap();
        assert_eq!(rebuilt.key(), graph.key());
        assert_eq!(rebuilt.entities().collect::<Vec<_>>(), graph.entities().collect::<Vec<_>>());
        assert!(schedule.require_graph(&rebuilt).is_err(),
            "identical nominal topology does not share materialization authority");
        drop(rebuilt);
        let graph = std::hint::black_box(Box::new(graph));
        schedule.require_graph(&graph).unwrap();
        drop(schedule);
        assert_eq!(budget.reserved(), before);
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            invocation_sccs(&graph, &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(tiny.reserved(), 0);
        let wrong = describe(
            &data,
            ProjectionKey {
                name: ProjectionName::DefinitionContainment,
                ..key
            },
            &budget,
        )
        .unwrap();
        let wrong = MaterializedGraph::build(&wrong, &budget).unwrap();
        assert!(matches!(
            invocation_sccs(&wrong, &budget),
            Err(ModelError::Invalid(_))
        ));
        let before_schedule = budget.reserved();
        let schedule = invocation_sccs(&graph, &budget).unwrap();
        let schedule_charge = budget.reserved() - before_schedule;
        let source = SourceArtifact::from_bytes(input.id(), "new.py".into(), b"").unwrap();
        let module = Module { source: source.id(), qualified_name: "new".into() };
        let entity = EntityRef::Module { module: module.id() };
        data.artifacts.insert(source).unwrap();
        data.modules.insert(module).unwrap();
        data.refs.insert(entity).unwrap();
        let changed_projection = describe(&data, key, &budget).unwrap();
        let changed = MaterializedGraph::build(&changed_projection, &budget).unwrap();
        assert_eq!(changed.key(), graph.key());
        assert_eq!(changed.vertex_count(), graph.vertex_count() + 1);
        assert!(schedule.require_graph(&changed).is_err(),
            "the same frame cannot substitute changed topology");
        drop(changed);
        drop(changed_projection);
        let before_graph_drop = budget.reserved();
        drop(graph);
        assert_eq!(budget.reserved(), before_graph_drop - graph_charge,
            "a retained schedule must not retain graph storage");
        assert_eq!(schedule.components(), expected);
        let before_schedule_drop = budget.reserved();
        drop(schedule);
        assert_eq!(budget.reserved(), before_schedule_drop - schedule_charge);
    }
    #[test]
    fn schedule_is_callee_first_with_canonical_ties_and_unchanged_parallel_evidence() {
        let edges = [(8, 4), (8, 4), (8, 7), (7, 6), (6, 7), (4, 2), (2, 2)];
        let expected = vec![vec![1], vec![2], vec![4], vec![6, 7], vec![8]];
        assert_eq!(
            canonical_order(
                vec![vec![8], vec![7, 6], vec![4], vec![2], vec![1]],
                edges.into_iter()
            )
            .unwrap(),
            expected
        );
        assert_eq!(
            canonical_order(
                vec![vec![1], vec![2], vec![4], vec![6, 7], vec![8]],
                edges.into_iter().rev()
            )
            .unwrap(),
            expected
        );
        assert!(
            canonical_order::<u8>(vec![], [].into_iter())
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn malformed_partition_never_becomes_a_partial_schedule() {
        assert!(canonical_order(vec![vec![1], vec![2]], [(1, 2), (2, 1)].into_iter()).is_err());
        assert!(canonical_order(vec![vec![1], vec![1]], [].into_iter()).is_err());
        assert!(canonical_order(vec![vec![1]], [(1, 2)].into_iter()).is_err());
    }
}
