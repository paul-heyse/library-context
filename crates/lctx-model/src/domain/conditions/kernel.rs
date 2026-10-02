//! Bounded biodivine operations over nominal evaluation atoms and typed persisted nodes.
use super::{Condition, ConditionNode, EvaluationAtom};
use crate::domain::{
    Id, ModelError, Record,
    resources::{Reservation, ResourceBudget},
};
use biodivine_lib_bdd::{Bdd, BddNode, BddPointer, BddVariable, BddVariableSet, op_function};
use std::collections::{BTreeMap, BTreeSet, HashMap};
type AtomId = Id<EvaluationAtom>;
pub(super) const MAX_ATOMS: usize = 128;
pub(super) const MAX_NODES: usize = 50_000;
pub(super) const MAX_PAIR_WORK: usize = 1_000_000;
const MAX_EXPR_NODES: usize = 4_096;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KernelBoundary {
    AtomLimit,
    WorkPreflight,
    NodeLimit,
    TransferUnsupported,
    MalformedGraph,
}
#[derive(Clone)]
pub struct Diagram {
    pub(super) support: Vec<AtomId>,
    pub(super) ctx: BddVariableSet,
    pub(super) bdd: Bdd,
}
/// Library allocation admission is separate from semantic BDD work/node limits. The allowance
/// covers bounded vectors, memo tables, variable names and canonical-record lowering; it does
/// not claim process RSS or allocator-internal accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOperation {
    Conjunction,
    Disjunction,
}
#[derive(Debug)]
pub enum DiagramAdmissionError {
    Boundary(KernelBoundary),
    Resource(ModelError),
}
impl std::fmt::Display for DiagramAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boundary(boundary) => write!(f, "condition boundary: {boundary:?}"),
            Self::Resource(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for DiagramAdmissionError {}
/// The reservation follows the returned diagram. Consumers may borrow it or explicitly transfer
/// both parts into their charged output owner; dropping the value releases its allowance.
#[derive(Debug)]
pub struct AdmittedDiagram {
    diagram: Diagram,
    reservation: Box<dyn Reservation>,
}
impl std::ops::Deref for AdmittedDiagram {
    type Target = Diagram;
    fn deref(&self) -> &Diagram {
        &self.diagram
    }
}
impl AdmittedDiagram {
    pub fn reserved_bytes(&self) -> usize {
        self.reservation.size()
    }
    pub fn into_parts(self) -> (Diagram, Box<dyn Reservation>) {
        (self.diagram, self.reservation)
    }
}
// Hash/vector allowances include spare capacity and bucket headers. The pinned engine's apply
// retains one task memo and DFS stack per node pair, and one unique-node map/result vector.
const PAIR_ALLOCATION_ALLOWANCE: usize = 128;
const NODE_ALLOCATION_ALLOWANCE: usize = 512;
const ATOM_ALLOCATION_ALLOWANCE: usize = 1024;
impl std::fmt::Debug for Diagram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Diagram")
            .field("atoms", &self.support.len())
            .field("nodes", &self.bdd.size())
            .finish()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedCondition {
    pub terms: Vec<Vec<(AtomId, bool)>>,
    pub truncated: bool,
}
#[derive(Clone, Debug)]
pub struct FactorResult {
    pub diagram: Diagram,
    pub factored: bool,
}
pub(super) fn var_name(atom: AtomId) -> String {
    format!("a_{}", atom.hex())
}

pub(super) fn context(support: &[AtomId]) -> BddVariableSet {
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
    Atom(AtomId),
    Not(Box<CondExpr>),
    And(Vec<CondExpr>),
    Or(Vec<CondExpr>),
}

impl CondExpr {
    fn size(&self) -> usize {
        let mut pending = vec![(self, 0usize)];
        let mut count = 0;
        while let Some((expr, depth)) = pending.pop() {
            count += 1;
            if count > MAX_EXPR_NODES || depth > MAX_ATOMS {
                return MAX_EXPR_NODES + 1;
            }
            match expr {
                Self::Not(inner) => pending.push((inner, depth + 1)),
                Self::And(parts) | Self::Or(parts) => {
                    if parts.len() > MAX_EXPR_NODES {
                        return MAX_EXPR_NODES + 1;
                    }
                    pending.extend(parts.iter().map(|p| (p, depth + 1)));
                }
                _ => {}
            }
        }
        count
    }

    fn atoms(&self, out: &mut BTreeSet<AtomId>) {
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

impl Diagram {
    pub(super) fn effective(
        support: Vec<AtomId>,
        ctx: BddVariableSet,
        bdd: Bdd,
    ) -> Result<Self, KernelBoundary> {
        let live = bdd.support_set();
        if live.len() == support.len() {
            return Ok(Self { support, ctx, bdd });
        }
        let support: Vec<AtomId> = support
            .into_iter()
            .enumerate()
            .filter_map(|(index, atom)| {
                live.contains(&BddVariable::from_index(index))
                    .then_some(atom)
            })
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
    pub fn from_atom(atom: AtomId) -> Self {
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
        let support: Vec<AtomId> = atoms.into_iter().collect();
        let ctx = context(&support);
        let mut work = MAX_PAIR_WORK;
        fn build(
            expr: &CondExpr,
            support: &[AtomId],
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
                    let index = support
                        .binary_search(id)
                        .expect("gathered into the support");
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

    /// Retained diagram plus its canonical-ID lowering scratch. Consumers that copy or lower a
    /// borrowed diagram reserve this amount before materializing that copy.
    pub fn allocation_allowance(&self) -> usize {
        self.node_count()
            .saturating_mul(NODE_ALLOCATION_ALLOWANCE)
            .saturating_add(self.support.len().saturating_mul(ATOM_ALLOCATION_ALLOWANCE))
            .saturating_add(size_of::<Self>())
    }
    /// Deterministic preflight for the exact shared apply operation. Union cardinality is counted
    /// without allocating. Pair work bounds the task cache even when the reduced result is tiny;
    /// the output limit counts terminals and a refused apply may create one excess node first.
    pub fn binary_allocation_allowance(&self, other: &Self) -> Result<usize, KernelBoundary> {
        let mut left = self.support.iter().peekable();
        let mut right = other.support.iter().peekable();
        let mut atoms = 0usize;
        while left.peek().is_some() || right.peek().is_some() {
            match (left.peek(), right.peek()) {
                (Some(a), Some(b)) => match a.cmp(b) {
                    std::cmp::Ordering::Less => {
                        left.next();
                    }
                    std::cmp::Ordering::Greater => {
                        right.next();
                    }
                    std::cmp::Ordering::Equal => {
                        left.next();
                        right.next();
                    }
                },
                (Some(_), None) => {
                    left.next();
                }
                (None, Some(_)) => {
                    right.next();
                }
                (None, None) => break,
            }
            atoms += 1;
        }
        if atoms > MAX_ATOMS {
            return Err(KernelBoundary::AtomLimit);
        }
        let pairs = self
            .node_count()
            .checked_mul(other.node_count())
            .ok_or(KernelBoundary::WorkPreflight)?;
        if pairs > MAX_PAIR_WORK
            || self
                .node_count()
                .checked_mul(atoms)
                .is_none_or(|n| n > MAX_PAIR_WORK)
            || other
                .node_count()
                .checked_mul(atoms)
                .is_none_or(|n| n > MAX_PAIR_WORK)
        {
            return Err(KernelBoundary::WorkPreflight);
        }
        let output = pairs.saturating_add(2).min(MAX_NODES + 1);
        pairs
            .checked_mul(PAIR_ALLOCATION_ALLOWANCE)
            .and_then(|n| {
                n.checked_add(
                    (self.node_count() + other.node_count() + output)
                        .checked_mul(NODE_ALLOCATION_ALLOWANCE)?,
                )
            })
            .and_then(|n| n.checked_add(atoms.checked_mul(ATOM_ALLOCATION_ALLOWANCE)?))
            .and_then(|n| n.checked_add(size_of::<Self>() * 4))
            .ok_or(KernelBoundary::WorkPreflight)
    }
    pub fn admitted_binary(
        &self,
        other: &Self,
        operation: BooleanOperation,
        budget: &ResourceBudget,
    ) -> Result<AdmittedDiagram, DiagramAdmissionError> {
        let bytes = self
            .binary_allocation_allowance(other)
            .map_err(DiagramAdmissionError::Boundary)?;
        let mut reservation = budget
            .reserve("condition_binary", bytes)
            .map_err(DiagramAdmissionError::Resource)?;
        let diagram = match operation {
            BooleanOperation::Conjunction => self.and(other),
            BooleanOperation::Disjunction => self.or(other),
        }
        .map_err(DiagramAdmissionError::Boundary)?;
        // Apply's task tables have been dropped. Keep the result and its canonical lowering
        // admitted for as long as the consumer owns the diagram.
        reservation
            .try_resize(diagram.allocation_allowance())
            .map_err(DiagramAdmissionError::Resource)?;
        Ok(AdmittedDiagram {
            diagram,
            reservation,
        })
    }

    /// The atoms the condition depends on, in id order.
    pub fn support(&self) -> &[AtomId] {
        &self.support
    }

    pub fn is_false(&self) -> bool {
        self.bdd.is_false()
    }

    pub fn is_true(&self) -> bool {
        self.bdd.is_true()
    }

    pub fn records(&self) -> (Condition, Vec<ConditionNode>) {
        fn visit(
            diagram: &Diagram,
            pointer: BddPointer,
            memo: &mut HashMap<BddPointer, Id<ConditionNode>>,
            rows: &mut BTreeMap<Id<ConditionNode>, ConditionNode>,
        ) -> Id<ConditionNode> {
            if let Some(id) = memo.get(&pointer) {
                return *id;
            }
            let row = if pointer.is_zero() {
                ConditionNode::False
            } else if pointer.is_one() {
                ConditionNode::True
            } else {
                let atom = diagram.support[diagram.bdd.var_of(pointer).to_index()];
                let low = visit(diagram, diagram.bdd.low_link_of(pointer), memo, rows);
                let high = visit(diagram, diagram.bdd.high_link_of(pointer), memo, rows);
                ConditionNode::Branch { atom, low, high }
            };
            let id = row.id();
            rows.insert(id, row);
            memo.insert(pointer, id);
            id
        }
        let mut rows = BTreeMap::new();
        let root = visit(
            self,
            self.bdd.root_pointer(),
            &mut HashMap::new(),
            &mut rows,
        );
        (Condition { root }, rows.into_values().collect())
    }
    pub fn id(&self) -> Id<Condition> {
        self.records().0.id()
    }
    pub fn from_records(
        condition: &Condition,
        nodes: &[ConditionNode],
    ) -> Result<Self, ModelError> {
        if nodes.len() > 100_000 {
            return Err(ModelError::Invalid(
                "condition node admission exceeded".into(),
            ));
        }
        let mut catalog = BTreeMap::new();
        for node in nodes {
            node.validate()?;
            if catalog.insert(node.id(), node.clone()).is_some() {
                return Err(ModelError::Conflict(ConditionNode::NAME));
            }
        }
        let selected = closure(condition.root, &catalog)?;
        let support: Vec<_> = selected
            .iter()
            .filter_map(|id| match &catalog[id] {
                ConditionNode::Branch { atom, .. } => Some(*atom),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let ctx = context(&support);
        let mut raw = vec![
            BddNode::mk_zero(support.len() as u16),
            BddNode::mk_one(support.len() as u16),
        ];
        fn append(
            id: Id<ConditionNode>,
            catalog: &BTreeMap<Id<ConditionNode>, ConditionNode>,
            support: &[AtomId],
            raw: &mut Vec<BddNode>,
            memo: &mut BTreeMap<Id<ConditionNode>, BddPointer>,
        ) -> BddPointer {
            if let Some(pointer) = memo.get(&id) {
                return *pointer;
            }
            let pointer = match &catalog[&id] {
                ConditionNode::False => BddPointer::zero(),
                ConditionNode::True => BddPointer::one(),
                ConditionNode::Branch { atom, low, high } => {
                    let low = append(*low, catalog, support, raw, memo);
                    let high = append(*high, catalog, support, raw, memo);
                    let pointer = BddPointer::from_index(raw.len());
                    raw.push(BddNode::mk_node(
                        BddVariable::from_index(support.binary_search(atom).expect("closure atom")),
                        low,
                        high,
                    ));
                    pointer
                }
            };
            memo.insert(id, pointer);
            pointer
        }
        let root = append(
            condition.root,
            &catalog,
            &support,
            &mut raw,
            &mut BTreeMap::new(),
        );
        if root.is_zero() {
            return Ok(Self::never());
        }
        if root.is_one() {
            return Ok(Self::always());
        }
        let bdd = Bdd::from_nodes(&raw).map_err(ModelError::codec)?;
        bdd.validate().map_err(ModelError::codec)?;
        let result = Self { support, ctx, bdd };
        if result.id() != condition.id() {
            return Err(ModelError::Identity(Condition::NAME));
        }
        Ok(result)
    }
    fn in_union(&self, union: &[AtomId]) -> Result<Bdd, KernelBoundary> {
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

    fn union_support(&self, other: &Self) -> Vec<AtomId> {
        self.support
            .iter()
            .chain(other.support.iter())
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn union(
        &self,
        other: &Self,
    ) -> Result<(Vec<AtomId>, BddVariableSet, Bdd, Bdd), KernelBoundary> {
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
    pub fn restrict_atoms(&self, assignments: &[(AtomId, bool)]) -> Result<Self, KernelBoundary> {
        if self
            .node_count()
            .checked_mul(assignments.len())
            .is_none_or(|work| work > MAX_PAIR_WORK)
        {
            return Err(KernelBoundary::WorkPreflight);
        }
        let mut values = Vec::with_capacity(assignments.len());
        let mut seen = BTreeSet::new();
        for &(atom, value) in assignments {
            if !seen.insert(atom) {
                return Err(KernelBoundary::TransferUnsupported);
            }
            let index = self
                .support
                .binary_search(&atom)
                .map_err(|_| KernelBoundary::TransferUnsupported)?;
            values.push((BddVariable::from_index(index), value));
        }
        Self::effective(
            self.support.clone(),
            self.ctx.clone(),
            self.bdd.restrict(&values),
        )
    }

    /// Simultaneously replace source atoms by conditions. Replacements are evaluated against the
    /// original diagram (no capture). Unknown or duplicate source atoms are refused. Work, each
    /// result and all retained intermediates are bounded; a refusal is never `false`.
    pub fn substitute_atoms(
        &self,
        replacements: &[(AtomId, &Self)],
    ) -> Result<Self, KernelBoundary> {
        super::substitution::compose(self, replacements)
    }
    /// Admit all bounded memoized subdiagrams and apply scratch before substitution starts.
    pub fn admitted_substitution(
        &self,
        replacements: &[(AtomId, &Self)],
        budget: &ResourceBudget,
    ) -> Result<AdmittedDiagram, DiagramAdmissionError> {
        // Validate bindings before allocating even the preflight vocabulary. Canonical contexts
        // order nominal atom ids, so transferring a replacement into the union preserves its
        // relative variable order and cannot grow its node count.
        for (index, (atom, _)) in replacements.iter().enumerate() {
            if self.support.binary_search(atom).is_err()
                || replacements[..index].iter().any(|(previous, _)| previous == atom)
            {
                return Err(DiagramAdmissionError::Boundary(KernelBoundary::TransferUnsupported));
            }
        }
        let vocabulary = self.support.len().checked_add(replacements.iter().map(|(_, d)| d.support.len()).sum::<usize>())
            .and_then(|n| n.checked_mul(128)).and_then(|n| n.checked_add(8192))
            .ok_or(DiagramAdmissionError::Boundary(KernelBoundary::WorkPreflight))?;
        let mut reservation = budget.reserve("condition_substitution", vocabulary)
            .map_err(DiagramAdmissionError::Resource)?;
        let support: BTreeSet<_> = self.support.iter().filter(|atom| !replacements.iter().any(|(bound, _)| bound == *atom))
            .chain(replacements.iter().flat_map(|(_, d)| d.support.iter())).copied().collect();
        if support.len() > MAX_ATOMS {
            return Err(DiagramAdmissionError::Boundary(KernelBoundary::AtomLimit));
        }
        // A reduced ordered BDD over n variables has at most the full decision tree plus
        // terminals. The deliberately looser 2^(n+1) also covers the constant case.
        let possible_nodes = 1usize.checked_shl((support.len() + 1) as u32).unwrap_or(MAX_NODES).min(MAX_NODES);
        let pairs = possible_nodes.checked_mul(possible_nodes).unwrap_or(MAX_PAIR_WORK).min(MAX_PAIR_WORK);
        // At most two edge visits per source node clone a memo/terminal result. Each first
        // visit also owns predicate/not, three apply outputs and the memo output clone;
        // the final two allowances cover normalization. Initial replacement transfers keep
        // their node counts because canonical variable order is unchanged.
        let retained = self.bdd.size().checked_mul(8).and_then(|n| n.checked_add(2))
            .and_then(|n| n.checked_mul(possible_nodes))
            .and_then(|n| replacements.iter().try_fold(n, |n, (_, d)| n.checked_add(d.bdd.size())))
            .unwrap_or(super::substitution::MAX_RETAINED_NODES).min(super::substitution::MAX_RETAINED_NODES);
        let bytes = pairs
            .checked_mul(PAIR_ALLOCATION_ALLOWANCE)
            .and_then(|n| {
                n.checked_add(retained.checked_mul(size_of::<BddNode>().saturating_mul(4))?)
            })
            .and_then(|n| n.checked_add(possible_nodes.checked_mul(NODE_ALLOCATION_ALLOWANCE)?))
            .and_then(|n| n.checked_add(self.allocation_allowance()))
            .and_then(|n| {
                replacements
                    .iter()
                    .try_fold(n, |n, (_, d)| n.checked_add(d.allocation_allowance()))
            })
            .ok_or(DiagramAdmissionError::Boundary(
                KernelBoundary::WorkPreflight,
            ))?;
        reservation.try_resize(bytes.checked_add(vocabulary).ok_or(DiagramAdmissionError::Boundary(KernelBoundary::WorkPreflight))?)
            .map_err(DiagramAdmissionError::Resource)?;
        drop(support);
        let diagram = self
            .substitute_atoms(replacements)
            .map_err(DiagramAdmissionError::Boundary)?;
        reservation
            .try_resize(diagram.allocation_allowance())
            .map_err(DiagramAdmissionError::Resource)?;
        Ok(AdmittedDiagram {
            diagram,
            reservation,
        })
    }

    /// Existentially eliminate explicitly selected `atoms`: the
    /// result holds exactly when some assignment to them makes this condition hold. Atoms outside
    /// the support are ignored. Each step is one bounded `or` of two cofactors under a shared work
    /// budget. This algebraic operation is not a call-boundary policy: composition must retain
    /// local opaque guards unless an explicit semantic proof permits their elimination.
    pub fn exists(&self, atoms: &[AtomId]) -> Result<Self, KernelBoundary> {
        let targets: BTreeSet<AtomId> = atoms
            .iter()
            .copied()
            .filter(|a| self.support.binary_search(a).is_ok())
            .collect();
        let mut work = MAX_PAIR_WORK;
        let mut bdd = self.bdd.clone();
        for atom in targets {
            let index = self
                .support
                .binary_search(&atom)
                .expect("filtered to the support");
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

pub(crate) fn closure(
    root: Id<ConditionNode>,
    nodes: &BTreeMap<Id<ConditionNode>, ConditionNode>,
) -> Result<BTreeSet<Id<ConditionNode>>, ModelError> {
    let mut found = BTreeSet::new();
    let mut atoms = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if !found.insert(id) {
            continue;
        }
        if found.len() > MAX_NODES {
            return Err(ModelError::Invalid("condition node budget exceeded".into()));
        }
        let node = nodes
            .get(&id)
            .ok_or_else(|| ModelError::Invalid("condition child absent".into()))?;
        node.validate()?;
        if let ConditionNode::Branch { atom, low, high } = node {
            atoms.insert(*atom);
            if atoms.len() > MAX_ATOMS {
                return Err(ModelError::Invalid("condition atom budget exceeded".into()));
            }
            for child in [low, high] {
                let row = nodes
                    .get(child)
                    .ok_or_else(|| ModelError::Invalid("condition child absent".into()))?;
                if let ConditionNode::Branch {
                    atom: child_atom, ..
                } = row
                    && child_atom <= atom
                {
                    return Err(ModelError::Invalid(
                        "condition atoms are unordered or cyclic".into(),
                    ));
                }
                pending.push(*child);
            }
        }
    }
    Ok(found)
}
