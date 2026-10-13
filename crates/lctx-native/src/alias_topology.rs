//! Immutable directed alias adjacency over exact owned rows. Source-key indexes and
//! visited runs reuse charged external ordering; no whole-heap graph is required.
use crate::ordered_rows::{
    PreparedPointRows, PreparedRows, PreparedSourceRows, SortedRows, SourceRows,
};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};
use surrealdb::types::{Object, RecordId, Value};

fn pointer(id: RecordId) -> Value {
    let mut row = Object::new();
    row.insert("id", id);
    Value::Object(row)
}
fn id(row: &Value) -> Result<&RecordId, ModelError> {
    match row.as_object().and_then(|row| row.get("id")) {
        Some(Value::RecordId(id)) => Ok(id),
        _ => Err(ModelError::Schema("alias traversal node identity")),
    }
}
fn endpoints(row: &Value) -> Result<(&RecordId, &RecordId), ModelError> {
    let row = row
        .as_object()
        .ok_or(ModelError::Schema("alias topology object"))?;
    let (Some(Value::RecordId(source)), Some(Value::RecordId(target))) =
        (row.get("source"), row.get("target"))
    else {
        return Err(ModelError::Schema("alias topology endpoints"));
    };
    if source.table.as_str() != "entity" || target.table.as_str() != "entity" {
        return Err(ModelError::Conflict("alias topology entity family"));
    }
    Ok((source, target))
}
#[derive(Clone)]
pub(crate) struct AliasTopology {
    owned: PreparedRows,
    forward: PreparedSourceRows,
    reverse: PreparedSourceRows,
}
impl AliasTopology {
    pub(crate) fn new(
        owned: PreparedRows,
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<Self, ModelError> {
        let mut rows = owned.cursor_with_budget(budget)?;
        while let Some(row) = rows.next_row()? {
            check()?;
            if id(&row)?.table.as_str() != "compiler_alias" {
                return Err(ModelError::Conflict("alias topology identity family"));
            }
            endpoints(&row)?;
        }
        let forward = owned.source_index("source", budget, &mut check)?;
        let reverse = owned.source_index("target", budget, &mut check)?;
        check()?;
        Ok(Self {
            owned,
            forward,
            reverse,
        })
    }
    pub(crate) fn incoming(
        &self,
        target: &RecordId,
        budget: &ResourceBudget,
    ) -> Result<SourceRows, ModelError> {
        self.reverse.rows_for(target, budget)
    }
    /// Reachability is composed over the supplied contributor set. Audit additionally
    /// requires every owned source to be reachable, including disconnected cycles.
    pub(crate) fn expand(
        &self,
        seeds: PreparedRows,
        require_all: bool,
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<(PreparedRows, PreparedRows), ModelError> {
        self.expand_filtered(seeds, require_all, None, budget, &mut check)
    }
    pub(crate) fn expand_selected(
        &self,
        seeds: PreparedRows,
        eligible: &PreparedPointRows,
        budget: &ResourceBudget,
        check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<(PreparedRows, PreparedRows), ModelError> {
        self.expand_filtered(seeds, false, Some(eligible), budget, check)
    }
    fn expand_filtered(
        &self,
        seeds: PreparedRows,
        require_all: bool,
        eligible: Option<&PreparedPointRows>,
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<(PreparedRows, PreparedRows), ModelError> {
        let mut eligibility = eligible
            .map(|eligible| eligible.cursor(budget))
            .transpose()?;
        let mut visited = VisitedNodes::new(budget)?;
        visited.add(seeds.clone(), budget, &mut check)?;
        let mut frontier = seeds;
        let mut aliases = SortedRows::with_budget(budget)?;
        loop {
            let mut candidates = SortedRows::with_budget(budget)?;
            let mut nodes = frontier.cursor_with_budget(budget)?;
            while let Some(node) = nodes.next_row()? {
                check()?;
                let node = id(&node)?;
                if node.table.as_str() != "entity" {
                    continue;
                }
                let mut outgoing = self.forward.rows_for(node, budget)?;
                while let Some(row) = outgoing.next_row()? {
                    check()?;
                    if let Some(eligible) = &mut eligibility {
                        if !eligible.contains(id(&row)?)? {
                            continue;
                        }
                    }
                    let (source, target) = endpoints(&row)?;
                    if source != node {
                        return Err(ModelError::Conflict("alias topology source range"));
                    }
                    candidates.push(pointer(target.clone()))?;
                    aliases.push(row)?;
                }
            }
            let (next, changed) =
                visited.unseen(candidates.finish()?.into_prepared(), budget, &mut check)?;
            if !changed {
                break;
            }
            visited.add(next.clone(), budget, &mut check)?;
            frontier = next;
        }
        if require_all {
            let mut rows = self.owned.cursor_with_budget(budget)?;
            while let Some(row) = rows.next_row()? {
                check()?;
                if !visited.contains(endpoints(&row)?.0, budget)? {
                    return Err(ModelError::Conflict(
                        "installed held alias outside selected reachable closure",
                    ));
                }
            }
        }
        check()?;
        Ok((
            visited.finish(budget, &mut check)?,
            aliases.finish()?.into_prepared(),
        ))
    }
}

/// Immutable visited runs merge only when two occupied levels meet. Each new
/// frontier is retained once; long chains do not repeatedly copy all prior nodes.
pub(crate) struct VisitedNodes {
    levels: Vec<Option<(PreparedRows, PreparedPointRows)>>,
    _charge: StateCharge,
}
impl VisitedNodes {
    pub(crate) fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "alias visited run directory");
        charge
            .grow((usize::BITS as usize).saturating_mul(std::mem::size_of::<
                Option<(PreparedRows, PreparedPointRows)>,
            >()))?;
        Ok(Self {
            levels: Vec::with_capacity(usize::BITS as usize),
            _charge: charge,
        })
    }
    pub(crate) fn contains(
        &self,
        node: &RecordId,
        budget: &ResourceBudget,
    ) -> Result<bool, ModelError> {
        for (_, index) in self.levels.iter().flatten() {
            if index.cursor(budget)?.contains(node)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn unseen(
        &self,
        candidates: PreparedRows,
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<(PreparedRows, bool), ModelError> {
        let mut output = SortedRows::with_budget(budget)?;
        let mut changed = false;
        let mut candidates = candidates.cursor_with_budget(budget)?;
        while let Some(row) = candidates.next_row()? {
            check()?;
            if !self.contains(id(&row)?, budget)? {
                changed = true;
                output.push(row)?;
            }
        }
        Ok((output.finish()?.into_prepared(), changed))
    }
    pub(crate) fn add(
        &mut self,
        mut rows: PreparedRows,
        budget: &ResourceBudget,
        mut check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<(), ModelError> {
        let mut level = 0;
        loop {
            check()?;
            if level == self.levels.len() {
                self.levels.push(None);
            }
            if let Some((previous, _)) = self.levels[level].take() {
                rows = PreparedRows::union_prepared(&[previous, rows], budget, &mut check)?;
                level += 1;
            } else {
                let index = rows.point_index(budget)?;
                self.levels[level] = Some((rows, index));
                return Ok(());
            }
        }
    }
    pub(crate) fn finish(
        self,
        budget: &ResourceBudget,
        check: impl FnMut() -> Result<(), ModelError>,
    ) -> Result<PreparedRows, ModelError> {
        let rows = self
            .levels
            .into_iter()
            .flatten()
            .map(|(rows, _)| rows)
            .collect::<Vec<_>>();
        PreparedRows::union_prepared(&rows, budget, check)
    }
}

pub(crate) fn audit_reachability(
    seeds: PreparedRows,
    aliases: &PreparedRows,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let topology = AliasTopology::new(aliases.clone(), budget, || Ok(()))?;
    topology.expand(seeds, true, budget, || Ok(())).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    fn node(key: &str) -> RecordId {
        RecordId::new("entity", key)
    }
    fn prepared_seeds(keys: &[&str], budget: &ResourceBudget) -> PreparedRows {
        let mut rows = SortedRows::with_budget(budget).unwrap();
        for key in keys {
            rows.push(pointer(node(key))).unwrap();
        }
        rows.finish().unwrap().into_prepared()
    }
    fn prepared_aliases(edges: &[(&str, &str)], budget: &ResourceBudget) -> PreparedRows {
        let mut rows = SortedRows::with_budget(budget).unwrap();
        for (index, (source, target)) in edges.iter().enumerate() {
            let mut row = Object::new();
            row.insert(
                "id",
                RecordId::new("compiler_alias", format!("{:08}", edges.len() - index)),
            );
            row.insert("source", node(source));
            row.insert("target", node(target));
            rows.push(Value::Object(row)).unwrap();
        }
        rows.finish().unwrap().into_prepared()
    }
    fn identities(rows: &PreparedRows, budget: &ResourceBudget) -> BTreeSet<RecordId> {
        let mut rows = rows.cursor_with_budget(budget).unwrap();
        let mut actual = BTreeSet::new();
        while let Some(row) = rows.next_row().unwrap() {
            actual.insert(id(&row).unwrap().clone());
        }
        actual
    }
    #[test]
    fn directed_frontier_preserves_chains_cycles_repeated_targets_and_empty_seeds() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let owned = prepared_aliases(
            &[
                ("c", "d"),
                ("b", "c"),
                ("seed", "a"),
                ("a", "b"),
                ("seed", "b"),
                ("d", "a"),
                ("foreign", "outside"),
            ],
            &budget,
        );
        let topology = AliasTopology::new(owned, &budget, || Ok(())).unwrap();
        let (nodes, aliases) = topology
            .expand(
                prepared_seeds(&["seed"], &budget),
                false,
                &budget,
                || Ok(()),
            )
            .unwrap();
        assert_eq!(
            identities(&nodes, &budget),
            ["seed", "a", "b", "c", "d"].map(node).into()
        );
        assert_eq!(
            identities(&aliases, &budget).len(),
            6,
            "all distinct reached alias identities survive a shared target and cycle"
        );
        assert!(
            topology
                .expand(prepared_seeds(&["seed"], &budget), true, &budget, || Ok(()))
                .is_err()
        );
        let (nodes, aliases) = topology
            .expand(prepared_seeds(&[], &budget), false, &budget, || Ok(()))
            .unwrap();
        assert!(identities(&nodes, &budget).is_empty());
        assert!(identities(&aliases, &budget).is_empty());
        assert!(
            topology
                .expand(prepared_seeds(&[], &budget), true, &budget, || Ok(()))
                .is_err()
        );
        let mut incoming = topology.incoming(&node("b"), &budget).unwrap();
        let mut sources = BTreeSet::new();
        while let Some(row) = incoming.next_row().unwrap() {
            sources.insert(endpoints(&row).unwrap().0.clone());
        }
        assert_eq!(sources, ["seed", "a"].map(node).into());
    }
    #[test]
    fn composed_contributor_paths_and_changed_set_have_separate_closures() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let left = prepared_aliases(&[("seed", "bridge")], &budget);
        let mut rows = SortedRows::with_budget(&budget).unwrap();
        let mut row = Object::new();
        row.insert("id", RecordId::new("compiler_alias", "other owner"));
        row.insert("source", node("bridge"));
        row.insert("target", node("target"));
        rows.push(Value::Object(row)).unwrap();
        let right = rows.finish().unwrap().into_prepared();
        let union =
            PreparedRows::union_prepared(&[left.clone(), right.clone()], &budget, || Ok(()))
                .unwrap();
        let topology = AliasTopology::new(union, &budget, || Ok(())).unwrap();
        let (nodes, _) = topology
            .expand(prepared_seeds(&["seed"], &budget), true, &budget, || Ok(()))
            .unwrap();
        assert_eq!(
            identities(&nodes, &budget),
            ["seed", "bridge", "target"].map(node).into()
        );
        let changed = AliasTopology::new(left, &budget, || Ok(())).unwrap();
        let (nodes, _) = changed
            .expand(prepared_seeds(&["seed"], &budget), true, &budget, || Ok(()))
            .unwrap();
        assert_eq!(
            identities(&nodes, &budget),
            ["seed", "bridge"].map(node).into()
        );
        let disconnected = AliasTopology::new(right, &budget, || Ok(())).unwrap();
        assert!(
            disconnected
                .expand(prepared_seeds(&["seed"], &budget), true, &budget, || Ok(()))
                .is_err()
        );
    }
    #[test]
    fn spilled_alias_rows_follow_reversed_order_long_chain_without_heap_graph() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let mut rows = SortedRows::with_budget(&budget).unwrap();
        for index in (0..600).rev() {
            let mut row = Object::new();
            row.insert(
                "id",
                RecordId::new("compiler_alias", format!("{:08}", 600 - index)),
            );
            row.insert("source", node(&format!("{index:04}")));
            row.insert("target", node(&format!("{:04}", index + 1)));
            row.insert("retained_full_bytes", "x".repeat(4096));
            rows.push(Value::Object(row)).unwrap();
        }
        let topology =
            AliasTopology::new(rows.finish().unwrap().into_prepared(), &budget, || Ok(())).unwrap();
        let (nodes, aliases) = topology
            .expand(prepared_seeds(&["0000"], &budget), true, &budget, || Ok(()))
            .unwrap();
        assert_eq!(
            identities(&nodes, &budget),
            (0..=600)
                .map(|index| node(&format!("{index:04}")))
                .collect()
        );
        assert_eq!(identities(&aliases, &budget).len(), 600);
    }
    #[test]
    fn topology_cancellation_and_reservation_refuse_without_returning_partial_closure() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let owned = prepared_aliases(&[("a", "b")], &budget);
        assert!(
            AliasTopology::new(owned.clone(), &budget, || Err(ModelError::Conflict(
                "cancelled"
            )))
            .is_err()
        );
        let topology = AliasTopology::new(owned, &budget, || Ok(())).unwrap();
        let mut checks = 0;
        assert!(
            topology
                .expand(prepared_seeds(&["a"], &budget), false, &budget, || {
                    checks += 1;
                    if checks > 2 {
                        Err(ModelError::Conflict("cancelled"))
                    } else {
                        Ok(())
                    }
                })
                .is_err()
        );
        let small = ResourceBudget::scoped(&budget, 1).unwrap();
        assert!(matches!(
            topology.incoming(&node("b"), &small),
            Err(ModelError::Resource { .. })
        ));
    }
}
