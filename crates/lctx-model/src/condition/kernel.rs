//! The bounded condition kernel (DESIGN §15.7; ADR-0082 carrying ADR-0024's kernel clauses).
//!
//! A [`Diagram`] is a reduced ordered BDD whose variables are atom ids, ordered by id. It is
//! persisted as a condition id, its structural root and the content-addressed nonterminal nodes of
//! its closure ([`Diagram::root_and_nodes`]); no library variable index or pointer is ever an
//! identity. Every operation is preflighted against a pair-work budget and node-capped; a budget
//! hit is a [`KernelBoundary`], which a caller records as a `budget_reached` obligation.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use biodivine_lib_bdd::{Bdd, BddNode, BddPointer, BddVariable, BddVariableSet, op_function};

use crate::id::{Id, recipes};

/// Version of the structural node encoding (v2: atoms are ids, not encoded strings).
pub const KERNEL_FORMAT: u32 = 2;
/// Admission limits for one generation's condition catalog.
pub const MAX_CATALOG_CONDITIONS: usize = 100_000;
pub const MAX_CATALOG_NODES: usize = 100_000;
/// Sum of owned nonterminal nodes across every hydrated condition, including shared closures.
pub const MAX_CATALOG_RETAINED_NODES: usize = 1_000_000;

pub(super) const MAX_ATOMS: usize = 128;
pub(super) const MAX_NODES: usize = 50_000;
pub(super) const MAX_PAIR_WORK: usize = 1_000_000;
/// Nodes a constructed expression may hold before any BDD work.
const MAX_EXPR_NODES: usize = 4_096;

/// Why a condition question cannot be decided within the kernel's budgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KernelBoundary {
    AtomLimit,
    WorkPreflight,
    NodeLimit,
    TransferUnsupported,
}

impl KernelBoundary {
    pub fn code(self) -> &'static str {
        match self {
            Self::AtomLimit => "atom_limit",
            Self::WorkPreflight => "work_preflight",
            Self::NodeLimit => "node_limit",
            Self::TransferUnsupported => "transfer_unsupported",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "atom_limit" => Self::AtomLimit,
            "work_preflight" => Self::WorkPreflight,
            "node_limit" => Self::NodeLimit,
            "transfer_unsupported" => Self::TransferUnsupported,
            _ => return None,
        })
    }
}

/// A content-addressed nonterminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagramNode {
    pub node_id: Id,
    pub atom: Id,
    pub low: Id,
    pub high: Id,
}

/// One persisted condition: its id and structural root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConditionRoot {
    pub condition_id: Id,
    pub root_id: Id,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeValidationError {
    DuplicateId,
    MissingNode,
    Cycle,
    Unordered,
    Unreduced,
    IdentityMismatch,
    Limit,
    UnreachableNode,
    InvalidBdd,
}

pub fn false_terminal() -> Id {
    recipes::condition_terminal(false)
}

pub fn true_terminal() -> Id {
    recipes::condition_terminal(true)
}

fn terminal(id: Id) -> bool {
    id == false_terminal() || id == true_terminal()
}

/// The shared publication and load check of a complete condition catalog.
pub fn hydrate_catalog(
    conditions: &[ConditionRoot],
    nodes: &[DiagramNode],
) -> Result<HashMap<Id, Diagram>, String> {
    hydrate_catalog_with_retained_limit(conditions, nodes, MAX_CATALOG_RETAINED_NODES)
}

pub(super) fn hydrate_catalog_with_retained_limit(
    conditions: &[ConditionRoot],
    nodes: &[DiagramNode],
    retained_limit: usize,
) -> Result<HashMap<Id, Diagram>, String> {
    if conditions.len() > MAX_CATALOG_CONDITIONS || nodes.len() > MAX_CATALOG_NODES {
        return Err(format!(
            "condition catalog exceeds limits: {} conditions, {} nodes",
            conditions.len(),
            nodes.len()
        ));
    }
    let mut catalog = HashMap::new();
    for node in nodes {
        if catalog.insert(node.node_id, node.clone()).is_some() {
            return Err(format!("duplicate node {}", node.node_id.hex()));
        }
    }
    // A shared stored node can be copied once per root. Refuse before making those copies.
    let mut retained = 0usize;
    for root in conditions.iter().map(|row| row.root_id) {
        let mut seen = HashSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if terminal(id) || !seen.insert(id) {
                continue;
            }
            let node = catalog
                .get(&id)
                .ok_or_else(|| format!("condition catalog misses node {}", id.hex()))?;
            retained += 1;
            if retained > retained_limit {
                return Err(format!(
                    "condition catalog retained-node limit: {} stored, over {} expanded",
                    nodes.len(),
                    retained_limit
                ));
            }
            pending.push(node.low);
            pending.push(node.high);
        }
    }
    let mut diagrams = HashMap::new();
    let mut reached = HashSet::new();
    for row in conditions {
        if diagrams.contains_key(&row.condition_id) {
            return Err(format!("duplicate condition {}", row.condition_id.hex()));
        }
        let diagram = Diagram::from_catalog(row.root_id, &catalog)
            .map_err(|e| format!("invalid root {}: {e:?}", row.root_id.hex()))?;
        if diagram.id() != row.condition_id {
            return Err(format!("condition id differs from root {}", row.root_id.hex()));
        }
        reached.extend(diagram.root_and_nodes().1.into_iter().map(|n| n.node_id));
        diagrams.insert(row.condition_id, diagram);
    }
    if reached.len() != catalog.len() {
        return Err(format!(
            "{} condition nodes are not reachable from any stated root",
            catalog.len().saturating_sub(reached.len())
        ));
    }
    Ok(diagrams)
}

/// Validate a persisted root's node closure before publication or hydration.
pub fn validate_nodes(root: Id, nodes: &[DiagramNode]) -> Result<(), NodeValidationError> {
    if nodes.len() > MAX_NODES {
        return Err(NodeValidationError::Limit);
    }
    let mut by_id = BTreeMap::new();
    for node in nodes {
        if by_id.insert(node.node_id, node).is_some() {
            return Err(NodeValidationError::DuplicateId);
        }
    }
    fn visit(
        id: Id,
        by_id: &BTreeMap<Id, &DiagramNode>,
        visiting: &mut HashSet<Id>,
        visited: &mut HashSet<Id>,
        support: &mut BTreeSet<Id>,
        depth: usize,
    ) -> Result<(), NodeValidationError> {
        if terminal(id) || visited.contains(&id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(NodeValidationError::Cycle);
        }
        if depth > MAX_ATOMS {
            return Err(NodeValidationError::Limit);
        }
        let node = by_id.get(&id).ok_or(NodeValidationError::MissingNode)?;
        support.insert(node.atom);
        if support.len() > MAX_ATOMS {
            return Err(NodeValidationError::Limit);
        }
        if node.low == node.high {
            return Err(NodeValidationError::Unreduced);
        }
        if recipes::condition_node(node.atom, node.low, node.high) != id {
            return Err(NodeValidationError::IdentityMismatch);
        }
        for child in [node.low, node.high] {
            if !terminal(child) {
                let next = by_id.get(&child).ok_or(NodeValidationError::MissingNode)?;
                if next.atom <= node.atom {
                    return Err(NodeValidationError::Unordered);
                }
            }
            visit(child, by_id, visiting, visited, support, depth + 1)?;
        }
        visiting.remove(&id);
        visited.insert(id);
        Ok(())
    }
    let mut visited = HashSet::new();
    visit(
        root,
        &by_id,
        &mut HashSet::new(),
        &mut visited,
        &mut BTreeSet::new(),
        1,
    )?;
    if visited.len() != by_id.len() {
        return Err(NodeValidationError::UnreachableNode);
    }
    Ok(())
}

pub(super) fn var_name(atom: Id) -> String {
    format!("a_{}", atom.hex())
}

pub(super) fn context(support: &[Id]) -> BddVariableSet {
    let names: Vec<String> = support.iter().map(|a| var_name(*a)).collect();
    BddVariableSet::new(&names.iter().map(String::as_str).collect::<Vec<_>>())
}

fn apply(
    left: &Bdd,
    right: &Bdd,
    op: fn(Option<bool>, Option<bool>) -> Option<bool>,
) -> Result<Bdd, KernelBoundary> {
    if left
        .size()
        .checked_mul(right.size())
        .is_none_or(|work| work > MAX_PAIR_WORK)
    {
        return Err(KernelBoundary::WorkPreflight);
    }
    Bdd::binary_op_with_limit(MAX_NODES, left, right, op).ok_or(KernelBoundary::NodeLimit)
}

/// A Boolean expression over atoms, built by a producer directly from a source predicate. A
/// condition is never constructed from a normal form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CondExpr {
    True,
    False,
    Atom(Id),
    Not(Box<CondExpr>),
    And(Vec<CondExpr>),
    Or(Vec<CondExpr>),
}

impl CondExpr {
    fn size(&self) -> usize {
        match self {
            Self::True | Self::False | Self::Atom(_) => 1,
            Self::Not(inner) => 1 + inner.size(),
            Self::And(parts) | Self::Or(parts) => 1 + parts.iter().map(Self::size).sum::<usize>(),
        }
    }

    fn atoms(&self, out: &mut BTreeSet<Id>) {
        match self {
            Self::True | Self::False => {}
            Self::Atom(id) => {
                out.insert(*id);
            }
            Self::Not(inner) => inner.atoms(out),
            Self::And(parts) | Self::Or(parts) => parts.iter().for_each(|p| p.atoms(out)),
        }
    }
}

/// A condition. Its support is intrinsic (the atoms the function depends on); variable positions
/// are library-local.
#[derive(Clone)]
pub struct Diagram {
    pub(super) support: Vec<Id>,
    pub(super) ctx: BddVariableSet,
    pub(super) bdd: Bdd,
}

impl std::fmt::Debug for Diagram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Diagram")
            .field("id", &self.id())
            .field("atoms", &self.support.len())
            .field("nodes", &self.node_count())
            .finish()
    }
}

/// A presentation of a condition: bounded satisfying paths, in variable order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedCondition {
    pub terms: Vec<Vec<(Id, bool)>>,
    /// More paths exist than were rendered. A truncated rendering never claims completeness.
    pub truncated: bool,
}

/// The outcome of `given`: the quotient if verified, otherwise the original.
#[derive(Clone, Debug)]
pub struct FactorResult {
    pub diagram: Diagram,
    pub factored: bool,
}

impl Diagram {
    /// Drop variables the function no longer depends on, so a later atom-limit decision depends
    /// on the function, not on how it was constructed.
    pub(super) fn effective(
        support: Vec<Id>,
        ctx: BddVariableSet,
        bdd: Bdd,
    ) -> Result<Self, KernelBoundary> {
        let live = bdd.support_set();
        if live.len() == support.len() {
            return Ok(Self { support, ctx, bdd });
        }
        let support: Vec<Id> = support
            .into_iter()
            .enumerate()
            .filter_map(|(index, atom)| live.contains(&BddVariable::from_index(index)).then_some(atom))
            .collect();
        let reduced = context(&support);
        let bdd = reduced
            .transfer_from(&bdd, &ctx)
            .ok_or(KernelBoundary::TransferUnsupported)?;
        Ok(Self {
            support,
            ctx: reduced,
            bdd,
        })
    }

    /// Select and validate one root's closure from a shared node catalog.
    pub fn from_catalog(
        root: Id,
        catalog: &HashMap<Id, DiagramNode>,
    ) -> Result<Self, NodeValidationError> {
        fn gather(
            id: Id,
            catalog: &HashMap<Id, DiagramNode>,
            seen: &mut HashSet<Id>,
            out: &mut Vec<DiagramNode>,
            depth: usize,
        ) -> Result<(), NodeValidationError> {
            if terminal(id) || !seen.insert(id) {
                return Ok(());
            }
            if out.len() >= MAX_NODES || depth > MAX_ATOMS {
                return Err(NodeValidationError::Limit);
            }
            let node = catalog.get(&id).ok_or(NodeValidationError::MissingNode)?;
            out.push(node.clone());
            gather(node.low, catalog, seen, out, depth + 1)?;
            gather(node.high, catalog, seen, out, depth + 1)?;
            Ok(())
        }
        let mut closure = Vec::new();
        gather(root, catalog, &mut HashSet::new(), &mut closure, 1)?;
        Self::from_root_and_nodes(root, &closure)
    }

    /// Hydrate one exact persisted root closure after validating it.
    pub fn from_root_and_nodes(root: Id, nodes: &[DiagramNode]) -> Result<Self, NodeValidationError> {
        validate_nodes(root, nodes)?;
        if root == false_terminal() {
            return Ok(Self::never());
        }
        if root == true_terminal() {
            return Ok(Self::always());
        }
        let support: Vec<Id> = nodes
            .iter()
            .map(|node| node.atom)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let ctx = context(&support);
        let count = u16::try_from(support.len()).map_err(|_| NodeValidationError::Limit)?;
        let mut raw = vec![BddNode::mk_zero(count), BddNode::mk_one(count)];
        let by_id: HashMap<Id, &DiagramNode> = nodes.iter().map(|n| (n.node_id, n)).collect();
        fn append(
            id: Id,
            by_id: &HashMap<Id, &DiagramNode>,
            support: &[Id],
            raw: &mut Vec<BddNode>,
            done: &mut HashMap<Id, BddPointer>,
        ) -> Result<BddPointer, NodeValidationError> {
            if id == false_terminal() {
                return Ok(BddPointer::zero());
            }
            if id == true_terminal() {
                return Ok(BddPointer::one());
            }
            if let Some(&pointer) = done.get(&id) {
                return Ok(pointer);
            }
            let node = by_id.get(&id).ok_or(NodeValidationError::MissingNode)?;
            let low = append(node.low, by_id, support, raw, done)?;
            let high = append(node.high, by_id, support, raw, done)?;
            let index = support
                .binary_search(&node.atom)
                .map_err(|_| NodeValidationError::InvalidBdd)?;
            let pointer = BddPointer::from_index(raw.len());
            raw.push(BddNode::mk_node(BddVariable::from_index(index), low, high));
            done.insert(id, pointer);
            Ok(pointer)
        }
        append(root, &by_id, &support, &mut raw, &mut HashMap::new())?;
        let bdd = Bdd::from_nodes(&raw).map_err(|_| NodeValidationError::InvalidBdd)?;
        bdd.validate().map_err(|_| NodeValidationError::InvalidBdd)?;
        let hydrated = Self { support, ctx, bdd };
        if hydrated.root_and_nodes().0 != root {
            return Err(NodeValidationError::IdentityMismatch);
        }
        Ok(hydrated)
    }

    pub fn always() -> Self {
        let ctx = BddVariableSet::new(&[]);
        let bdd = ctx.mk_true();
        Self {
            support: Vec::new(),
            ctx,
            bdd,
        }
    }

    pub fn never() -> Self {
        let ctx = BddVariableSet::new(&[]);
        let bdd = ctx.mk_false();
        Self {
            support: Vec::new(),
            ctx,
            bdd,
        }
    }

    /// One atom's truth.
    pub fn from_atom(atom: Id) -> Self {
        let ctx = context(&[atom]);
        let bdd = ctx.mk_var(ctx.variables()[0]);
        Self {
            support: vec![atom],
            ctx,
            bdd,
        }
    }

    /// Build a condition from a producer's expression, under the kernel's budgets.
    pub fn from_expr(expr: &CondExpr) -> Result<Self, KernelBoundary> {
        if expr.size() > MAX_EXPR_NODES {
            return Err(KernelBoundary::WorkPreflight);
        }
        let mut atoms = BTreeSet::new();
        expr.atoms(&mut atoms);
        if atoms.len() > MAX_ATOMS {
            return Err(KernelBoundary::AtomLimit);
        }
        let support: Vec<Id> = atoms.into_iter().collect();
        let ctx = context(&support);
        let mut work = MAX_PAIR_WORK;
        fn build(
            expr: &CondExpr,
            support: &[Id],
            ctx: &BddVariableSet,
            work: &mut usize,
        ) -> Result<Bdd, KernelBoundary> {
            let fold = |parts: &[CondExpr],
                            start: Bdd,
                            op: fn(Option<bool>, Option<bool>) -> Option<bool>,
                            work: &mut usize|
             -> Result<Bdd, KernelBoundary> {
                let mut acc = start;
                for part in parts {
                    let next = build(part, support, ctx, work)?;
                    *work = work
                        .checked_sub(acc.size().saturating_mul(next.size()))
                        .ok_or(KernelBoundary::WorkPreflight)?;
                    acc = apply(&acc, &next, op)?;
                }
                Ok(acc)
            };
            match expr {
                CondExpr::True => Ok(ctx.mk_true()),
                CondExpr::False => Ok(ctx.mk_false()),
                CondExpr::Atom(id) => {
                    let index = support.binary_search(id).expect("gathered into the support");
                    Ok(ctx.mk_var(BddVariable::from_index(index)))
                }
                CondExpr::Not(inner) => Ok(build(inner, support, ctx, work)?.not()),
                CondExpr::And(parts) => fold(parts, ctx.mk_true(), op_function::and, work),
                CondExpr::Or(parts) => fold(parts, ctx.mk_false(), op_function::or, work),
            }
        }
        let bdd = build(expr, &support, &ctx, &mut work)?;
        Self::effective(support, ctx, bdd)
    }

    pub fn node_count(&self) -> usize {
        self.bdd.size()
    }

    /// The atoms the condition depends on, in id order.
    pub fn support(&self) -> &[Id] {
        &self.support
    }

    pub fn is_false(&self) -> bool {
        self.bdd.is_false()
    }

    pub fn is_true(&self) -> bool {
        self.bdd.is_true()
    }

    /// The structural root plus its lossless, content-addressed nonterminal closure.
    pub fn root_and_nodes(&self) -> (Id, Vec<DiagramNode>) {
        fn node(
            diagram: &Diagram,
            pointer: BddPointer,
            memo: &mut HashMap<BddPointer, Id>,
            rows: &mut BTreeMap<Id, DiagramNode>,
        ) -> Id {
            if pointer.is_zero() {
                return false_terminal();
            }
            if pointer.is_one() {
                return true_terminal();
            }
            if let Some(id) = memo.get(&pointer) {
                return *id;
            }
            let atom = diagram.support[diagram.bdd.var_of(pointer).to_index()];
            let low = node(diagram, diagram.bdd.low_link_of(pointer), memo, rows);
            let high = node(diagram, diagram.bdd.high_link_of(pointer), memo, rows);
            let id = recipes::condition_node(atom, low, high);
            rows.insert(
                id,
                DiagramNode {
                    node_id: id,
                    atom,
                    low,
                    high,
                },
            );
            memo.insert(pointer, id);
            id
        }
        let mut rows = BTreeMap::new();
        let root = node(self, self.bdd.root_pointer(), &mut HashMap::new(), &mut rows);
        (root, rows.into_values().collect())
    }

    /// The condition id, domain-separated from its structural root.
    pub fn id(&self) -> Id {
        recipes::condition(self.root_and_nodes().0)
    }

    fn in_union(&self, union: &[Id]) -> Result<Bdd, KernelBoundary> {
        if union.len() > MAX_ATOMS {
            return Err(KernelBoundary::AtomLimit);
        }
        if self
            .node_count()
            .checked_mul(union.len())
            .is_none_or(|work| work > MAX_PAIR_WORK)
        {
            return Err(KernelBoundary::WorkPreflight);
        }
        context(union)
            .transfer_from(&self.bdd, &self.ctx)
            .ok_or(KernelBoundary::TransferUnsupported)
    }

    fn union_support(&self, other: &Self) -> Vec<Id> {
        self.support
            .iter()
            .chain(other.support.iter())
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn union(&self, other: &Self) -> Result<(Vec<Id>, BddVariableSet, Bdd, Bdd), KernelBoundary> {
        let support = self.union_support(other);
        if support.len() > MAX_ATOMS {
            return Err(KernelBoundary::AtomLimit);
        }
        let left = self.in_union(&support)?;
        let right = other.in_union(&support)?;
        let ctx = context(&support);
        Ok((support, ctx, left, right))
    }

    pub fn and(&self, other: &Self) -> Result<Self, KernelBoundary> {
        let (support, ctx, left, right) = self.union(other)?;
        let bdd = apply(&left, &right, op_function::and)?;
        Self::effective(support, ctx, bdd)
    }

    pub fn or(&self, other: &Self) -> Result<Self, KernelBoundary> {
        let (support, ctx, left, right) = self.union(other)?;
        let bdd = apply(&left, &right, op_function::or)?;
        Self::effective(support, ctx, bdd)
    }

    pub fn not(&self) -> Result<Self, KernelBoundary> {
        if self.node_count() > MAX_NODES {
            return Err(KernelBoundary::NodeLimit);
        }
        Self::effective(self.support.clone(), self.ctx.clone(), self.bdd.not())
    }

    /// Cofactor under exact assignments to atoms in the support. The result is still a condition
    /// over the unfixed atoms, not evidence that the assignments hold.
    pub fn restrict_atoms(&self, assignments: &[(Id, bool)]) -> Result<Self, KernelBoundary> {
        if self
            .node_count()
            .checked_mul(assignments.len())
            .is_none_or(|work| work > MAX_PAIR_WORK)
        {
            return Err(KernelBoundary::WorkPreflight);
        }
        let mut values = Vec::with_capacity(assignments.len());
        for &(atom, value) in assignments {
            let index = self
                .support
                .binary_search(&atom)
                .map_err(|_| KernelBoundary::TransferUnsupported)?;
            values.push((BddVariable::from_index(index), value));
        }
        Self::effective(self.support.clone(), self.ctx.clone(), self.bdd.restrict(&values))
    }

    /// Simultaneously replace source atoms by conditions. Replacements are evaluated against the
    /// original diagram (no capture). Unknown or duplicate source atoms are refused. Work, each
    /// result and all retained intermediates are bounded; a refusal is never `false`.
    pub fn substitute_atoms(&self, replacements: &[(Id, &Self)]) -> Result<Self, KernelBoundary> {
        super::substitution::compose(self, replacements)
    }

    /// Existentially eliminate `atoms` (DESIGN §15.6: callee-local atoms at a call boundary): the
    /// result holds exactly when some assignment to them makes this condition hold. Atoms outside
    /// the support are ignored. Each step is one bounded `or` of two cofactors under a shared work
    /// budget.
    pub fn exists(&self, atoms: &[Id]) -> Result<Self, KernelBoundary> {
        let targets: BTreeSet<Id> = atoms
            .iter()
            .copied()
            .filter(|a| self.support.binary_search(a).is_ok())
            .collect();
        let mut work = MAX_PAIR_WORK;
        let mut bdd = self.bdd.clone();
        for atom in targets {
            let index = self.support.binary_search(&atom).expect("filtered to the support");
            let variable = BddVariable::from_index(index);
            let low = bdd.restrict(&[(variable, false)]);
            let high = bdd.restrict(&[(variable, true)]);
            work = work
                .checked_sub(low.size().saturating_mul(high.size()))
                .ok_or(KernelBoundary::WorkPreflight)?;
            bdd = apply(&low, &high, op_function::or)?;
        }
        Self::effective(self.support.clone(), self.ctx.clone(), bdd)
    }

    /// `self` given `factor`: a quotient `q` with `factor ∧ q == self`, verified by a capped
    /// equality check. A factor with one satisfying cube nominates the cube cofactor; otherwise,
    /// or when verification fails, the original is returned unfactored.
    pub fn given(&self, factor: &Self) -> FactorResult {
        match self.cube_quotient(factor) {
            Some(quotient) => FactorResult {
                diagram: quotient,
                factored: true,
            },
            None => FactorResult {
                diagram: self.clone(),
                factored: false,
            },
        }
    }

    /// Verify a caller-proposed quotient: accepted only when `factor ∧ proposed == self`.
    pub fn verify_quotient(&self, factor: &Self, proposed: &Self) -> FactorResult {
        let factored = factor
            .and(proposed)
            .is_ok_and(|combined| combined.id() == self.id());
        FactorResult {
            diagram: if factored {
                proposed.clone()
            } else {
                self.clone()
            },
            factored,
        }
    }

    fn cube_quotient(&self, factor: &Self) -> Option<Self> {
        let (support, ctx, original, divisor) = self.union(factor).ok()?;
        let mut cubes = divisor.sat_clauses();
        let cube = cubes.next()?;
        if cubes.next().is_some() {
            return None;
        }
        let values = cube.to_values();
        if original.size().checked_mul(values.len())? > MAX_PAIR_WORK {
            return None;
        }
        let candidate = Self::effective(support, ctx, original.restrict(&values)).ok()?;
        factor
            .and(&candidate)
            .ok()
            .filter(|product| product.id() == self.id())
            .map(|_| candidate)
    }

    /// At most `max_terms` satisfying paths, for display. `truncated` states that more exist.
    pub fn render_terms(&self, max_terms: usize) -> Result<RenderedCondition, KernelBoundary> {
        if self
            .support
            .len()
            .checked_mul(max_terms.saturating_add(1))
            .is_none_or(|work| work > MAX_PAIR_WORK)
        {
            return Err(KernelBoundary::WorkPreflight);
        }
        let mut paths = self.bdd.sat_clauses();
        let mut terms = Vec::new();
        for _ in 0..max_terms {
            let Some(path) = paths.next() else { break };
            terms.push(
                path.to_values()
                    .into_iter()
                    .map(|(var, value)| (self.support[var.to_index()], value))
                    .collect(),
            );
        }
        Ok(RenderedCondition {
            terms,
            truncated: paths.next().is_some(),
        })
    }

    /// Whether both can hold together. Decided by a budgeted dry run; `Err` is unknown.
    pub fn compatible(&self, other: &Self) -> Result<bool, KernelBoundary> {
        let union = self.union_support(other);
        let left = self.in_union(&union)?;
        let right = other.in_union(&union)?;
        Bdd::check_binary_op(MAX_PAIR_WORK, &left, &right, op_function::and)
            .map(|(nonempty, _tasks)| nonempty)
            .ok_or(KernelBoundary::WorkPreflight)
    }

    /// Whether `self` implies `other` (`self ∧ ¬other` is unsatisfiable). `Err` is unknown.
    pub fn implies(&self, other: &Self) -> Result<bool, KernelBoundary> {
        let union = self.union_support(other);
        let left = self.in_union(&union)?;
        let right = other.in_union(&union)?;
        Bdd::check_binary_op(MAX_PAIR_WORK, &left, &right, op_function::and_not)
            .map(|(nonempty, _tasks)| !nonempty)
            .ok_or(KernelBoundary::WorkPreflight)
    }
}
