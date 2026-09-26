//! Bounded Boolean condition questions over evaluation atoms (ADR-0024).
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::condition::{Atom, Condition, Literal, MAX_CONJUNCTIONS, MAX_LITERALS};
use crate::id::{Id, IdHasher};
use biodivine_lib_bdd::{Bdd, BddNode, BddPointer, BddVariable, BddVariableSet, op_function};

/// Version of the structural node encoding persisted in a serving generation.
pub const KERNEL_FORMAT: u32 = 1;
/// Admission limits for one generation's catalog before native hydration.
pub const MAX_CATALOG_CONDITIONS: usize = 100_000;
pub const MAX_CATALOG_NODES: usize = 100_000;
/// Sum of owned nonterminal nodes across every hydrated condition, including shared closures.
pub const MAX_CATALOG_RETAINED_NODES: usize = 1_000_000;

// Stage 2 can retain 16 conjunctions of 8 distinct literals each.
const MAX_ATOMS: usize = 128;
const MAX_NODES: usize = 50_000;
const MAX_PAIR_WORK: usize = 1_000_000;
const MAX_INPUT_LITERALS: usize = 4_096;
const MAX_ATOM_BYTES: usize = 4_096;

/// Why a semantic answer cannot be decided within the declared kernel model/budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelBoundary {
    /// The Stage 2 DNF representation already discarded the original expression.
    SourceOverBudget,
    AtomLimit,
    WorkPreflight,
    NodeLimit,
    TransferUnsupported,
    AtomNameCollision,
}

impl KernelBoundary {
    pub fn code(self) -> &'static str {
        match self {
            Self::SourceOverBudget => "source_over_budget",
            Self::AtomLimit => "atom_limit",
            Self::WorkPreflight => "work_preflight",
            Self::NodeLimit => "node_limit",
            Self::TransferUnsupported => "transfer_unsupported",
            Self::AtomNameCollision => "atom_name_collision",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "source_over_budget" => Self::SourceOverBudget,
            "atom_limit" => Self::AtomLimit,
            "work_preflight" => Self::WorkPreflight,
            "node_limit" => Self::NodeLimit,
            "transfer_unsupported" => Self::TransferUnsupported,
            "atom_name_collision" => Self::AtomNameCollision,
            _ => return None,
        })
    }
}

/// A content-addressed nonterminal. No library-local variable index or pointer is persisted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagramNode {
    pub node_id: Id,
    pub atom: String,
    pub low: Id,
    pub high: Id,
}

/// One persisted condition reference. An absent root carries a named kernel boundary.
#[derive(Clone, Debug)]
pub struct ConditionRoot {
    pub condition_id: Id,
    pub root_id: Option<Id>,
    pub boundary_reason: Option<String>,
}

/// Shared publication and native-load check of a complete condition catalog.
pub fn hydrate_catalog(
    conditions: &[ConditionRoot],
    nodes: &[DiagramNode],
) -> Result<HashMap<Id, Diagram>, String> {
    hydrate_catalog_with_retained_limit(conditions, nodes, MAX_CATALOG_RETAINED_NODES)
}

fn hydrate_catalog_with_retained_limit(
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
        let atom = Atom::parse_encoded(&node.atom)
            .map_err(|e| format!("node {} atom: {e}", node.node_id.hex()))?;
        if atom.encode() != node.atom {
            return Err(format!("node {} has noncanonical atom", node.node_id.hex()));
        }
        if catalog.insert(node.node_id, node.clone()).is_some() {
            return Err(format!("duplicate node {}", node.node_id.hex()));
        }
    }
    // A shared on-disk node can be copied once per root. Refuse before making those copies.
    let mut retained = 0usize;
    for root in conditions.iter().filter_map(|row| row.root_id) {
        let mut seen = HashSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if id == false_terminal() || id == true_terminal() || !seen.insert(id) {
                continue;
            }
            let node = catalog
                .get(&id)
                .ok_or_else(|| format!("condition catalog misses node {}", id.hex()))?;
            retained += 1;
            if retained > retained_limit {
                return Err(format!(
                    "condition catalog retained-node limit: {} stored, over {} expanded",
                    nodes.len(), retained_limit
                ));
            }
            pending.push(node.low);
            pending.push(node.high);
        }
    }
    let mut diagrams = HashMap::new();
    let mut seen_conditions = HashSet::new();
    let mut reached_nodes = HashSet::new();
    for row in conditions {
        if !seen_conditions.insert(row.condition_id) {
            return Err(format!("duplicate condition {}", row.condition_id.hex()));
        }
        match (row.root_id, row.boundary_reason.as_deref()) {
            (Some(root), None) => {
                let diagram = Diagram::from_catalog(root, &catalog)
                    .map_err(|e| format!("invalid root {}: {e:?}", root.hex()))?;
                if diagram.id() != row.condition_id {
                    return Err(format!("condition id differs from root {}", root.hex()));
                }
                reached_nodes.extend(diagram.root_and_nodes().1.into_iter().map(|n| n.node_id));
                diagrams.insert(row.condition_id, diagram);
            }
            (None, Some(code)) if KernelBoundary::from_code(code).is_some() => {}
            _ => {
                return Err(format!(
                    "invalid condition boundary {}",
                    row.condition_id.hex()
                ));
            }
        }
    }
    if reached_nodes.len() != catalog.len() {
        return Err(format!(
            "{} condition nodes are not reachable from any stated root",
            catalog.len().saturating_sub(reached_nodes.len())
        ));
    }
    Ok(diagrams)
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
    AtomNameCollision,
    InvalidBdd,
}

pub fn false_terminal() -> Id {
    IdHasher::new("bdd-false").finish_id()
}

pub fn true_terminal() -> Id {
    IdHasher::new("bdd-true").finish_id()
}

fn node_id(atom: &str, low: Id, high: Id) -> Id {
    IdHasher::new("bdd-node")
        .str(atom)
        .id(low)
        .id(high)
        .finish_id()
}

/// Validate a persisted root's node closure before publication or native hydration.
pub fn validate_nodes(root: Id, nodes: &[DiagramNode]) -> Result<(), NodeValidationError> {
    if nodes.len() > MAX_NODES || nodes.iter().any(|node| node.atom.len() > MAX_ATOM_BYTES) {
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
        support: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<(), NodeValidationError> {
        if id == false_terminal() || id == true_terminal() || visited.contains(&id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(NodeValidationError::Cycle);
        }
        if depth > MAX_ATOMS {
            return Err(NodeValidationError::Limit);
        }
        let node = by_id.get(&id).ok_or(NodeValidationError::MissingNode)?;
        support.insert(node.atom.clone());
        if support.len() > MAX_ATOMS {
            return Err(NodeValidationError::Limit);
        }
        if node.low == node.high {
            return Err(NodeValidationError::Unreduced);
        }
        if node_id(&node.atom, node.low, node.high) != id {
            return Err(NodeValidationError::IdentityMismatch);
        }
        for child in [node.low, node.high] {
            if child != false_terminal() && child != true_terminal() {
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

fn atom_name(atom: &str) -> String {
    format!(
        "a_{}",
        IdHasher::new("bdd-atom").str(atom).finish_id().hex()
    )
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

/// A condition's support is intrinsic; context positions are library-local.
#[derive(Clone)]
pub struct Diagram {
    support: Vec<String>,
    ctx: BddVariableSet,
    bdd: Bdd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedCondition {
    /// Paths through the diagram, in variable order. This is display, not condition authority.
    pub terms: Vec<Vec<(String, bool)>>,
    pub truncated: bool,
}

#[derive(Clone)]
pub struct FactorResult {
    pub diagram: Diagram,
    pub factored: bool,
}

impl Diagram {
    /// Drop variables the Boolean function no longer depends on. Keeping them would make a
    /// later atom-limit decision depend on how this value was constructed rather than its root.
    fn effective(
        support: Vec<String>,
        ctx: BddVariableSet,
        bdd: Bdd,
    ) -> Result<Self, KernelBoundary> {
        let live = bdd.support_set();
        if live.len() == support.len() {
            return Ok(Self { support, ctx, bdd });
        }
        let support: Vec<String> = support
            .into_iter()
            .enumerate()
            .filter_map(|(index, atom)| {
                live.contains(&biodivine_lib_bdd::BddVariable::from_index(index))
                    .then_some(atom)
            })
            .collect();
        let names: Vec<String> = support.iter().map(|atom| atom_name(atom)).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let reduced = BddVariableSet::new(&refs);
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
            if id == false_terminal() || id == true_terminal() || !seen.insert(id) {
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

    /// Hydrate one exact persisted root closure after the shared publication/load validation.
    pub fn from_root_and_nodes(
        root: Id,
        nodes: &[DiagramNode],
    ) -> Result<Self, NodeValidationError> {
        validate_nodes(root, nodes)?;
        if root == false_terminal() {
            return Ok(Self::never());
        }
        if root == true_terminal() {
            return Ok(Self::always());
        }
        let support: Vec<String> = nodes
            .iter()
            .map(|node| node.atom.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let names: Vec<String> = support.iter().map(|atom| atom_name(atom)).collect();
        if names.iter().collect::<BTreeSet<_>>().len() != names.len() {
            return Err(NodeValidationError::AtomNameCollision);
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ctx = BddVariableSet::new(&refs);
        let count = u16::try_from(support.len()).map_err(|_| NodeValidationError::Limit)?;
        let mut raw = vec![BddNode::mk_zero(count), BddNode::mk_one(count)];
        let by_id: HashMap<Id, &DiagramNode> =
            nodes.iter().map(|node| (node.node_id, node)).collect();
        fn append(
            id: Id,
            by_id: &HashMap<Id, &DiagramNode>,
            support: &[String],
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
            raw.push(BddNode::mk_node(ctx_var(index), low, high));
            done.insert(id, pointer);
            Ok(pointer)
        }
        fn ctx_var(index: usize) -> biodivine_lib_bdd::BddVariable {
            biodivine_lib_bdd::BddVariable::from_index(index)
        }
        append(root, &by_id, &support, &mut raw, &mut HashMap::new())?;
        let bdd = Bdd::from_nodes(&raw).map_err(|_| NodeValidationError::InvalidBdd)?;
        bdd.validate()
            .map_err(|_| NodeValidationError::InvalidBdd)?;
        let hydrated = Self { support, ctx, bdd };
        if hydrated.root_and_nodes().0 != root {
            return Err(NodeValidationError::IdentityMismatch);
        }
        Ok(hydrated)
    }

    /// A constant condition. These constructors do not pass through Stage 2's DNF budget.
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

    /// Construct one evaluation atom directly, before DNF can discard the source expression.
    pub fn from_atom(atom: &Atom) -> Result<Self, KernelBoundary> {
        let encoded = atom.encode();
        if encoded.len() > MAX_ATOM_BYTES {
            return Err(KernelBoundary::WorkPreflight);
        }
        let name = atom_name(&encoded);
        let ctx = BddVariableSet::new(&[name.as_str()]);
        let bdd = ctx.mk_var(ctx.variables()[0]);
        Ok(Self {
            support: vec![encoded],
            ctx,
            bdd,
        })
    }

    pub fn from_condition(condition: &Condition) -> Result<Self, KernelBoundary> {
        let Condition::Dnf(terms) = condition else {
            return Err(KernelBoundary::SourceOverBudget);
        };
        let input_literals = terms
            .iter()
            .try_fold(0usize, |total, term| total.checked_add(term.len()));
        if terms.len() > MAX_INPUT_LITERALS
            || input_literals.is_none_or(|count| count > MAX_INPUT_LITERALS)
        {
            return Err(KernelBoundary::WorkPreflight);
        }
        let mut unique = BTreeSet::new();
        for literal in terms.iter().flatten() {
            let encoded = literal.atom.encode();
            if encoded.len() > MAX_ATOM_BYTES {
                return Err(KernelBoundary::WorkPreflight);
            }
            unique.insert(encoded);
            if unique.len() > MAX_ATOMS {
                return Err(KernelBoundary::AtomLimit);
            }
        }
        let support: Vec<String> = unique.into_iter().collect();
        if support.len() > MAX_ATOMS {
            return Err(KernelBoundary::AtomLimit);
        }
        let names: Vec<String> = support.iter().map(|a| atom_name(a)).collect();
        if names.iter().collect::<BTreeSet<_>>().len() != names.len() {
            return Err(KernelBoundary::AtomNameCollision);
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let ctx = BddVariableSet::new(&refs);
        let vars = ctx.variables();
        let mut bdd = ctx.mk_false();
        let mut remaining_work = MAX_PAIR_WORK;
        for term in terms {
            let mut conjunction = ctx.mk_true();
            for literal in term {
                let index = support
                    .binary_search(&literal.atom.encode())
                    .expect("literal was included in support");
                let part = ctx.mk_literal(vars[index], literal.positive);
                let cost = conjunction.size().saturating_mul(part.size());
                remaining_work = remaining_work
                    .checked_sub(cost)
                    .ok_or(KernelBoundary::WorkPreflight)?;
                conjunction = apply(&conjunction, &part, op_function::and)?;
            }
            let cost = bdd.size().saturating_mul(conjunction.size());
            remaining_work = remaining_work
                .checked_sub(cost)
                .ok_or(KernelBoundary::WorkPreflight)?;
            bdd = apply(&bdd, &conjunction, op_function::or)?;
        }
        Self::effective(support, ctx, bdd)
    }

    pub fn node_count(&self) -> usize {
        self.bdd.size()
    }

    pub fn support(&self) -> &[String] {
        &self.support
    }

    pub fn is_false(&self) -> bool {
        self.bdd.is_false()
    }

    pub fn is_true(&self) -> bool {
        self.bdd.is_true()
    }

    /// Structural root plus its lossless, content-addressed nonterminal closure.
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
            let atom = &diagram.support[diagram.bdd.var_of(pointer).to_index()];
            let low = node(diagram, diagram.bdd.low_link_of(pointer), memo, rows);
            let high = node(diagram, diagram.bdd.high_link_of(pointer), memo, rows);
            let id = node_id(atom, low, high);
            rows.insert(
                id,
                DiagramNode {
                    node_id: id,
                    atom: atom.clone(),
                    low,
                    high,
                },
            );
            memo.insert(pointer, id);
            id
        }
        let mut rows = BTreeMap::new();
        let root = node(
            self,
            self.bdd.root_pointer(),
            &mut HashMap::new(),
            &mut rows,
        );
        (root, rows.into_values().collect())
    }

    /// The condition id is domain-separated from its structural root id.
    pub fn id(&self) -> Id {
        let (root, _) = self.root_and_nodes();
        IdHasher::new("condition-bdd").id(root).finish_id()
    }

    fn in_union(&self, union: &[String]) -> Result<Bdd, KernelBoundary> {
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
        let names: Vec<String> = union.iter().map(|a| atom_name(a)).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        BddVariableSet::new(&refs)
            .transfer_from(&self.bdd, &self.ctx)
            .ok_or(KernelBoundary::TransferUnsupported)
    }

    fn union(
        &self,
        other: &Self,
    ) -> Result<(Vec<String>, BddVariableSet, Bdd, Bdd), KernelBoundary> {
        let support: Vec<String> = self
            .support
            .iter()
            .chain(other.support.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if support.len() > MAX_ATOMS {
            return Err(KernelBoundary::AtomLimit);
        }
        let left = self.in_union(&support)?;
        let right = other.in_union(&support)?;
        let names: Vec<String> = support.iter().map(|a| atom_name(a)).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        Ok((support, BddVariableSet::new(&refs), left, right))
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

    /// Cofactor a diagram under exact assignments to atoms already in its support. The result
    /// is still a condition over any unfixed atoms, not evidence that the assignments hold in
    /// Python. Restriction visits at most the input diagram for each assigned variable.
    pub fn restrict_atoms(&self, assignments: &[(&str, bool)]) -> Result<Self, KernelBoundary> {
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
                .binary_search_by(|candidate| candidate.as_str().cmp(atom))
                .map_err(|_| KernelBoundary::TransferUnsupported)?;
            values.push((BddVariable::from_index(index), value));
        }
        Self::effective(self.support.clone(), self.ctx.clone(), self.bdd.restrict(&values))
    }

    /// A proposed quotient is accepted only when a capped equality check verifies it.
    /// On failure to verify, the original remains authoritative.
    pub fn given(&self, factor: &Self, proposed: &Self) -> FactorResult {
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

    /// Restrict by a factor with one satisfying cube to nominate a quotient. Restriction is
    /// only a candidate: the caller must still prove `factor ∧ quotient == original`.
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

    /// Enumerate only a bounded number of satisfying paths for display.
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
                    .map(|(var, value)| (self.support[var.to_index()].clone(), value))
                    .collect(),
            );
        }
        Ok(RenderedCondition {
            terms,
            truncated: paths.next().is_some(),
        })
    }

    pub fn compatible(&self, other: &Self) -> Result<bool, KernelBoundary> {
        let union: Vec<String> = self
            .support
            .iter()
            .chain(other.support.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let left = self.in_union(&union)?;
        let right = other.in_union(&union)?;
        Bdd::check_binary_op(MAX_PAIR_WORK, &left, &right, op_function::and)
            .map(|(nonempty, _tasks)| nonempty)
            .ok_or(KernelBoundary::WorkPreflight)
    }

    pub fn implies(&self, other: &Self) -> Result<bool, KernelBoundary> {
        let union: Vec<String> = self
            .support
            .iter()
            .chain(other.support.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let left = self.in_union(&union)?;
        let right = other.in_union(&union)?;
        Bdd::check_binary_op(MAX_PAIR_WORK, &left, &right, op_function::and_not)
            .map(|(nonempty, _tasks)| !nonempty)
            .ok_or(KernelBoundary::WorkPreflight)
    }
}

/// A migration-safe condition value: the diagram decides, while Stage 2's normal form remains a
/// bounded readable projection until every consumer uses persisted roots and nodes.
#[derive(Clone)]
pub struct BoundedCondition {
    legacy: Condition,
    diagram: Result<Diagram, KernelBoundary>,
    approximated: bool,
}

impl std::fmt::Debug for BoundedCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedCondition")
            .field("legacy", &self.legacy)
            .field("diagram", &self.diagram.as_ref().map(Diagram::id))
            .field("approximated", &self.approximated)
            .finish()
    }
}

impl BoundedCondition {
    pub fn always() -> Self {
        Self {
            legacy: Condition::always(),
            diagram: Ok(Diagram::always()),
            approximated: false,
        }
    }

    pub fn never() -> Self {
        Self {
            legacy: Condition::never(),
            diagram: Ok(Diagram::never()),
            approximated: false,
        }
    }

    pub fn atom(atom: Atom) -> Self {
        let diagram = Diagram::from_atom(&atom);
        Self {
            legacy: Condition::atom(atom),
            diagram,
            approximated: false,
        }
    }

    pub fn from_legacy(legacy: Condition) -> Self {
        let diagram = Diagram::from_condition(&legacy);
        Self {
            legacy,
            diagram,
            approximated: false,
        }
    }

    pub fn from_parts(legacy: Condition, diagram: Result<Diagram, KernelBoundary>) -> Self {
        Self {
            legacy,
            diagram,
            approximated: false,
        }
    }

    pub fn unknown(reason: KernelBoundary) -> Self {
        Self {
            legacy: Condition::OverBudget,
            diagram: Err(reason),
            approximated: false,
        }
    }

    pub fn and(&self, other: &Self) -> Self {
        let diagram = match (&self.diagram, &other.diagram) {
            (Ok(a), Ok(b)) => a.and(b),
            (Err(e), _) | (_, Err(e)) => Err(*e),
        };
        Self {
            legacy: self.legacy.and(&other.legacy),
            diagram,
            approximated: self.approximated || other.approximated,
        }
    }

    pub fn or(&self, other: &Self) -> Self {
        let diagram = match (&self.diagram, &other.diagram) {
            (Ok(a), Ok(b)) => a.or(b),
            (Err(e), _) | (_, Err(e)) => Err(*e),
        };
        Self {
            legacy: self.legacy.or(&other.legacy),
            diagram,
            approximated: self.approximated || other.approximated,
        }
    }

    pub fn not(&self) -> Self {
        Self {
            legacy: self.legacy.not(),
            diagram: self.diagram.as_ref().map_err(|e| *e).and_then(Diagram::not),
            approximated: self.approximated,
        }
    }

    pub fn given(&self, factor: &Self) -> Self {
        let Ok(original) = &self.diagram else {
            return self.clone();
        };
        let Ok(divisor) = &factor.diagram else {
            return self.clone();
        };
        if let Some(candidate) = original.cube_quotient(divisor)
            && let Ok(rendered) = candidate.render_terms(MAX_CONJUNCTIONS)
            && !rendered.truncated
            && rendered.terms.iter().all(|term| term.len() <= MAX_LITERALS)
        {
            let literals: Option<Vec<Vec<Literal>>> = rendered
                .terms
                .into_iter()
                .map(|term| {
                    term.into_iter()
                        .map(|(encoded, value)| {
                            Some(Literal::new(Atom::parse_encoded(&encoded).ok()?, value))
                        })
                        .collect()
                })
                .collect();
            if let Some(literals) = literals {
                let legacy = Condition::from_dnf(literals);
                if legacy != Condition::OverBudget {
                    return Self {
                        legacy,
                        diagram: Ok(candidate),
                        approximated: self.approximated || factor.approximated,
                    };
                }
            }
        }
        let proposed_legacy = self.legacy.given(&factor.legacy);
        let Ok(proposed) = Diagram::from_condition(&proposed_legacy) else {
            return self.clone();
        };
        let factored = original.given(divisor, &proposed);
        if factored.factored {
            Self {
                legacy: proposed_legacy,
                diagram: Ok(factored.diagram),
                approximated: self.approximated || factor.approximated,
            }
        } else {
            self.clone()
        }
    }

    pub fn is_never(&self) -> bool {
        self.diagram.as_ref().is_ok_and(Diagram::is_false)
    }

    pub fn is_always(&self) -> bool {
        self.diagram.as_ref().is_ok_and(Diagram::is_true)
    }

    pub fn encode(&self) -> String {
        self.legacy.encode()
    }

    pub fn id(&self) -> Id {
        match &self.diagram {
            Ok(diagram) => diagram.id(),
            Err(reason) => IdHasher::new("condition-bdd-boundary")
                .str(&format!("{reason:?}"))
                .str(&self.legacy.encode())
                .finish_id(),
        }
    }

    pub fn legacy(&self) -> &Condition {
        &self.legacy
    }

    pub fn diagram(&self) -> Result<&Diagram, KernelBoundary> {
        self.diagram.as_ref().map_err(|e| *e)
    }

    pub fn boundary(&self) -> Option<KernelBoundary> {
        self.diagram.as_ref().err().copied()
    }

    /// An admitted may-path crossed a provider ambiguity or declared runtime assumption.
    /// This provenance is row-local and does not change the Boolean function's identity.
    pub fn approximated(&self) -> bool {
        self.approximated
    }

    pub fn with_approximation(mut self) -> Self {
        self.approximated = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::Literal;
    fn d(s: &str) -> Diagram {
        Diagram::from_condition(&Condition::parse(s).unwrap()).unwrap()
    }

    #[test]
    fn stage2_known_answers_have_canonical_ids() {
        let pairs = [
            ("true", "true"),
            ("false", "false"),
            (
                "equals(mode,\"http\") & !is_none(x)",
                "!is_none(x) & equals(mode,\"http\")",
            ),
            ("is_none(x) & !is_none(x)", "false"),
            ("truthy(a) | truthy(a) & truthy(b)", "truthy(a)"),
            (
                "truthy(a) | !truthy(a) & truthy(b)",
                "truthy(a) | truthy(b)",
            ),
            (
                "truthy(a) & !truthy(y) | truthy(a) & truthy(y)",
                "truthy(a)",
            ),
            (
                "member_of(t,{\"sse\",\"http\",\"sse\"})",
                "member_of(t,{\"http\",\"sse\"})",
            ),
            (
                "opaque(\"f(x)  and g\") & truthy(y)",
                "opaque(\"f(x) and g\") & truthy(y)",
            ),
        ];
        for (a, b) in pairs {
            assert_eq!(d(a).id(), d(b).id(), "{a}");
        }
    }

    #[test]
    fn union_transfer_and_factoring_are_exact() {
        let f = d("truthy(a) & truthy(c) | truthy(b) & truthy(c)");
        let normal = d("truthy(a) | truthy(b)");
        let quotient = d("truthy(c)");
        assert!(f.implies(&normal) == Ok(true));
        assert!(f.compatible(&d("!truthy(a) & !truthy(b)")) == Ok(false));
        let factored = Condition::parse("truthy(a) & truthy(c) | truthy(b) & truthy(c)")
            .unwrap()
            .given(&Condition::parse("truthy(a) | truthy(b)").unwrap());
        assert_eq!(
            Diagram::from_condition(&factored).unwrap().id(),
            quotient.id()
        );
        assert_eq!(
            f.id(),
            d("truthy(c) & truthy(b) | truthy(c) & truthy(a)").id()
        );
    }

    #[test]
    fn structural_id_ignores_redundant_support_and_input_order() {
        let Condition::Dnf(mut z) = Condition::parse("truthy(z)").unwrap() else {
            unreachable!()
        };
        let Condition::Dnf(mut az) = Condition::parse("truthy(a) & truthy(z)").unwrap() else {
            unreachable!()
        };
        let first =
            Diagram::from_condition(&Condition::Dnf(vec![z.remove(0), az.remove(0)])).unwrap();
        let second = Diagram::from_condition(&Condition::parse("truthy(z)").unwrap()).unwrap();
        assert_eq!(first.id(), second.id());
        assert_eq!(first.support(), second.support());
    }

    #[test]
    fn site_atoms_stay_independent_and_budget_refuses_work() {
        assert_eq!(d("truthy(p) & !truthy(q)").compatible(&d("true")), Ok(true));
        assert_eq!(
            d("truthy(p) & !truthy(p)").compatible(&d("true")),
            Ok(false)
        );
        let ctx = BddVariableSet::new_anonymous(2);
        let vars = ctx.variables();
        let a = ctx.mk_literal(vars[0], true);
        let b = ctx.mk_literal(vars[1], true);
        assert!(Bdd::binary_op_with_limit(1, &a, &b, op_function::and).is_none());
    }

    #[test]
    fn persisted_node_closure_rejects_loss_and_corruption() {
        let (root, rows) = d("truthy(a) & truthy(b)").root_and_nodes();
        assert_eq!(validate_nodes(root, &rows), Ok(()));
        assert_eq!(
            validate_nodes(root, &rows[1..]),
            Err(NodeValidationError::MissingNode)
        );
        let mut corrupt = rows.clone();
        corrupt[0].low = corrupt[0].high;
        assert_eq!(
            validate_nodes(root, &corrupt),
            Err(NodeValidationError::Unreduced)
        );
        let mut duplicate = rows.clone();
        duplicate.push(rows[0].clone());
        assert_eq!(
            validate_nodes(root, &duplicate),
            Err(NodeValidationError::DuplicateId)
        );
    }

    #[test]
    fn shared_catalog_hydration_rejects_orphans_and_duplicate_roots() {
        let a = d("truthy(a)");
        let (root, mut nodes) = a.root_and_nodes();
        let row = ConditionRoot {
            condition_id: a.id(),
            root_id: Some(root),
            boundary_reason: None,
        };
        assert!(hydrate_catalog(std::slice::from_ref(&row), &nodes).is_ok());
        assert!(
            hydrate_catalog(&[row.clone(), row.clone()], &nodes)
                .err()
                .unwrap()
                .contains("duplicate condition")
        );
        nodes.extend(d("truthy(b)").root_and_nodes().1);
        assert!(
            hydrate_catalog(&[row], &nodes)
                .err()
                .unwrap()
                .contains("not reachable")
        );
        let too_many = vec![
            ConditionRoot {
                condition_id: a.id(),
                root_id: Some(root),
                boundary_reason: None,
            };
            MAX_CATALOG_CONDITIONS + 1
        ];
        assert!(
            hydrate_catalog(&too_many, &[])
                .err()
                .unwrap()
                .contains("exceeds limits")
        );
    }

    #[test]
    fn retained_limit_counts_shared_closures_before_hydration() {
        let first = d("truthy(a) & truthy(z)");
        let second = d("truthy(b) & truthy(z)");
        let mut catalog = std::collections::BTreeMap::new();
        let rows: Vec<_> = [&first, &second]
            .into_iter()
            .map(|diagram| {
                let (root, nodes) = diagram.root_and_nodes();
                for node in nodes {
                    catalog.insert(node.node_id, node);
                }
                ConditionRoot {
                    condition_id: diagram.id(),
                    root_id: Some(root),
                    boundary_reason: None,
                }
            })
            .collect();
        let nodes: Vec<_> = catalog.into_values().collect();
        assert_eq!(nodes.len(), 3);
        let refusal = hydrate_catalog_with_retained_limit(&rows, &nodes, 3)
            .err()
            .unwrap();
        assert!(refusal.contains("3 stored, over 3 expanded"), "{refusal}");
        assert_eq!(
            hydrate_catalog_with_retained_limit(&rows, &nodes, 4)
                .unwrap()
                .len(),
            2
        );

        let disjoint = [d("truthy(a)"), d("truthy(b)")];
        let nodes: Vec<_> = disjoint.iter().flat_map(|d| d.root_and_nodes().1).collect();
        let rows: Vec<_> = disjoint
            .iter()
            .map(|d| ConditionRoot {
                condition_id: d.id(),
                root_id: Some(d.root_and_nodes().0),
                boundary_reason: None,
            })
            .collect();
        assert_eq!(
            hydrate_catalog_with_retained_limit(&rows, &nodes, 2)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn bounded_operations_and_rendering_keep_unknown_explicit() {
        let a = d("truthy(a)");
        let b = d("truthy(b)");
        assert_eq!(a.and(&b).unwrap().id(), d("truthy(a) & truthy(b)").id());
        assert_eq!(a.or(&b).unwrap().id(), d("truthy(a) | truthy(b)").id());
        assert_eq!(a.not().unwrap().id(), d("!truthy(a)").id());
        let combined = a.or(&b).unwrap();
        let display = combined.render_terms(1).unwrap();
        assert_eq!(display.terms.len(), 1);
        assert!(display.truncated);
        assert_eq!(
            combined.render_terms(MAX_PAIR_WORK + 1).err(),
            Some(KernelBoundary::WorkPreflight)
        );
        let factor = a.or(&b).unwrap();
        let body = d("truthy(c)");
        let product = factor.and(&body).unwrap();
        let accepted = product.given(&factor, &body);
        assert!(accepted.factored);
        assert_eq!(accepted.diagram.id(), body.id());
        let rejected = product.given(&factor, &a);
        assert!(!rejected.factored);
        assert_eq!(rejected.diagram.id(), product.id());
    }

    #[test]
    fn ten_atom_truth_table_matches_bounded_decisions_and_restriction() {
        let atoms: Vec<Diagram> = (0..10)
            .map(|index| {
                Diagram::from_atom(&Atom::Truthy {
                    place: format!("p{index}"),
                })
                .unwrap()
            })
            .collect();
        let mut left = Diagram::always();
        for pair in 0..5 {
            left = left.and(&atoms[2 * pair].or(&atoms[2 * pair + 1]).unwrap()).unwrap();
        }
        let right = atoms[0].and(&atoms[2]).unwrap();
        let both = left.and(&right).unwrap();
        let either = left.or(&right).unwrap();
        let complement = left.not().unwrap();
        let names: Vec<&str> = atoms.iter().map(|atom| atom.support()[0].as_str()).collect();
        let mut any_both = false;
        let mut counterexample = false;
        for mask in 0..(1 << 10) {
            let values: Vec<bool> = (0..10).map(|index| mask & (1 << index) != 0).collect();
            let assignments: Vec<(&str, bool)> = names
                .iter()
                .zip(&values)
                .map(|(name, value)| (*name, *value))
                .collect();
            let expected_left = (0..5).all(|pair| values[2 * pair] || values[2 * pair + 1]);
            let expected_right = values[0] && values[2];
            let value_of = |diagram: &Diagram| {
                let relevant: Vec<(&str, bool)> = assignments
                    .iter()
                    .copied()
                    .filter(|(name, _)| diagram.support().binary_search(&name.to_string()).is_ok())
                    .collect();
                diagram.restrict_atoms(&relevant).unwrap().is_true()
            };
            assert_eq!(value_of(&left), expected_left, "left at {mask}");
            assert_eq!(value_of(&right), expected_right, "right at {mask}");
            assert_eq!(value_of(&both), expected_left && expected_right, "and at {mask}");
            assert_eq!(value_of(&either), expected_left || expected_right, "or at {mask}");
            assert_eq!(value_of(&complement), !expected_left, "not at {mask}");
            any_both |= expected_left && expected_right;
            counterexample |= expected_left && !expected_right;
        }
        assert_eq!(left.compatible(&right), Ok(any_both));
        assert_eq!(left.implies(&right), Ok(!counterexample));
    }

    #[test]
    fn cube_restriction_factors_when_legacy_dnf_is_over_budget() {
        let factor = BoundedCondition::atom(Atom::Truthy {
            place: "gate".to_owned(),
        });
        let body = BoundedCondition::atom(Atom::Truthy {
            place: "result".to_owned(),
        });
        let product = factor.and(&body);
        let over_budget = BoundedCondition::from_parts(
            Condition::OverBudget,
            Ok(product.diagram().unwrap().clone()),
        );
        let quotient = over_budget.given(&factor);
        assert_eq!(quotient.id(), body.id());
        assert_eq!(quotient.encode(), body.encode());
        assert_eq!(factor.and(&quotient).id(), over_budget.id());

        let incompatible = BoundedCondition::atom(Atom::Truthy {
            place: "other".to_owned(),
        });
        assert_eq!(over_budget.given(&incompatible).id(), over_budget.id());
    }

    #[test]
    fn direct_atoms_keep_a_seventeen_way_disjunction_stated() {
        let mut diagram = Diagram::never();
        for index in 0..17 {
            let atom = crate::condition::Atom::Truthy {
                place: format!("option_{index}"),
            };
            diagram = diagram.or(&Diagram::from_atom(&atom).unwrap()).unwrap();
        }
        assert!(diagram.compatible(&Diagram::always()).unwrap());
        assert_ne!(diagram.id(), Diagram::always().id());
        assert!(diagram.render_terms(16).unwrap().truncated);
        let (root, nodes) = diagram.root_and_nodes();
        validate_nodes(root, &nodes).unwrap();
        let hydrated = Diagram::from_root_and_nodes(root, &nodes).unwrap();
        assert_eq!(hydrated.id(), diagram.id());
        assert_eq!(
            hydrated.render_terms(18).unwrap(),
            diagram.render_terms(18).unwrap()
        );
        let mut with_extra = nodes.clone();
        with_extra.extend(
            Diagram::from_atom(&crate::condition::Atom::Truthy {
                place: "unrelated".to_owned(),
            })
            .unwrap()
            .root_and_nodes()
            .1,
        );
        assert_eq!(
            Diagram::from_root_and_nodes(root, &with_extra).err(),
            Some(NodeValidationError::UnreachableNode)
        );
    }

    #[test]
    fn admits_the_full_stage2_support_and_bounds_direct_input() {
        let terms: Vec<Vec<Literal>> = (0..16)
            .map(|term| {
                (0..8)
                    .map(|literal| {
                        Literal::new(
                            crate::condition::Atom::Truthy {
                                place: format!("p{term}_{literal}"),
                            },
                            true,
                        )
                    })
                    .collect()
            })
            .collect();
        assert!(Diagram::from_condition(&Condition::Dnf(terms.clone())).is_ok());
        assert!(matches!(
            Diagram::from_condition(&Condition::Dnf(vec![
                terms[0].clone();
                MAX_INPUT_LITERALS + 1
            ])),
            Err(KernelBoundary::WorkPreflight)
        ));
        let oversize = Literal::new(
            crate::condition::Atom::Truthy {
                place: "x".repeat(MAX_ATOM_BYTES),
            },
            true,
        );
        assert!(matches!(
            Diagram::from_condition(&Condition::Dnf(vec![vec![oversize]])),
            Err(KernelBoundary::WorkPreflight)
        ));
    }

    #[test]
    fn persisted_node_validator_caps_untrusted_rows() {
        let (root, rows) = d("truthy(a)").root_and_nodes();
        let mut oversized = vec![rows[0].clone(); MAX_NODES + 1];
        assert_eq!(
            validate_nodes(root, &oversized),
            Err(NodeValidationError::Limit)
        );
        oversized.clear();
        let mut wide = rows[0].clone();
        wide.atom = "x".repeat(MAX_ATOM_BYTES + 1);
        assert_eq!(
            validate_nodes(root, &[wide]),
            Err(NodeValidationError::Limit)
        );
        let mut branches = Vec::new();
        let mut left = true_terminal();
        for number in (1..MAX_ATOMS).rev() {
            let atom = format!("a{number:03}");
            let id = node_id(&atom, false_terminal(), left);
            branches.push(DiagramNode {
                node_id: id,
                atom,
                low: false_terminal(),
                high: left,
            });
            left = id;
        }
        let right_atom = format!("a{MAX_ATOMS:03}");
        let right = node_id(&right_atom, false_terminal(), true_terminal());
        branches.push(DiagramNode {
            node_id: right,
            atom: right_atom,
            low: false_terminal(),
            high: true_terminal(),
        });
        let branch_root = node_id("a000", left, right);
        branches.push(DiagramNode {
            node_id: branch_root,
            atom: "a000".to_owned(),
            low: left,
            high: right,
        });
        assert_eq!(
            validate_nodes(branch_root, &branches),
            Err(NodeValidationError::Limit)
        );
    }
}
