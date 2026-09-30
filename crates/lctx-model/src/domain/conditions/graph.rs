//! Transient provider expressions. No provider index or alternate truth encoding is persisted.
use super::{Diagram, EvaluationAtom, KernelBoundary};
use crate::domain::Id;

/// Canonical truth together with the provider's uncertainty; callers must carry both.
#[derive(Debug, Clone)]
pub struct GraphCondition {
    pub diagram: Diagram,
    pub approximation: crate::domain::assertion::Approximation,
}

pub const MAX_GRAPH_NODES: usize = 4096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CondNode<L> {
    False,
    True,
    Leaf(L),
    Not(usize),
    And(usize, usize),
    Or(usize, usize),
    Refused,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CondGraph<L> {
    pub nodes: Vec<CondNode<L>>,
    pub root: usize,
    pub approximate: bool,
}
impl<L: Clone + PartialEq> CondGraph<L> {
    pub fn always() -> Self {
        Self {
            nodes: vec![CondNode::True],
            root: 0,
            approximate: false,
        }
    }
    pub fn never() -> Self {
        Self {
            nodes: vec![CondNode::False],
            root: 0,
            approximate: false,
        }
    }
    pub fn atom(leaf: L) -> Self {
        Self {
            nodes: vec![CondNode::Leaf(leaf)],
            root: 0,
            approximate: false,
        }
    }
    pub fn refused() -> Self {
        Self {
            nodes: vec![CondNode::Refused],
            root: 0,
            approximate: true,
        }
    }
    pub fn is_never(&self) -> bool {
        self.nodes.get(self.root) == Some(&CondNode::False)
    }
    pub fn is_always(&self) -> bool {
        self.nodes.get(self.root) == Some(&CondNode::True)
    }
    pub fn approximated(&self) -> bool {
        self.approximate
    }
    pub fn with_approximation(mut self) -> Self {
        self.approximate = true;
        self
    }
    fn has_refusal(&self) -> bool {
        self.nodes.iter().any(|n| matches!(n, CondNode::Refused))
    }
    pub fn not(&self) -> Self {
        if self.has_refusal() {
            return Self::refused();
        }
        if self.is_never() {
            return Self::always().with_approximation_if(self.approximate);
        }
        if self.is_always() {
            return Self::never().with_approximation_if(self.approximate);
        }
        if self.nodes.len() >= MAX_GRAPH_NODES {
            return Self::refused();
        }
        let mut out = self.clone();
        out.nodes.push(CondNode::Not(out.root));
        out.root = out.nodes.len() - 1;
        out
    }
    fn with_approximation_if(mut self, value: bool) -> Self {
        self.approximate = value;
        self
    }
    fn join(&self, other: &Self, and: bool) -> Self {
        if self.has_refusal() || other.has_refusal() {
            return Self::refused();
        }
        let approximate = self.approximate || other.approximate;
        if and && (self.is_never() || other.is_never()) {
            return Self::never().with_approximation_if(approximate);
        }
        if !and && (self.is_always() || other.is_always()) {
            return Self::always().with_approximation_if(approximate);
        }
        if (and && self.is_always()) || (!and && self.is_never()) {
            return other.clone().with_approximation_if(approximate);
        }
        if (and && other.is_always()) || (!and && other.is_never()) || self == other {
            return self.clone().with_approximation_if(approximate);
        }
        if self
            .nodes
            .len()
            .checked_add(other.nodes.len())
            .and_then(|n| n.checked_add(1))
            .is_none_or(|n| n > MAX_GRAPH_NODES)
        {
            return Self::refused();
        }
        let offset = self.nodes.len();
        let mut nodes = self.nodes.clone();
        nodes.extend(other.nodes.iter().map(|node| match node {
            CondNode::False => CondNode::False,
            CondNode::True => CondNode::True,
            CondNode::Leaf(leaf) => CondNode::Leaf(leaf.clone()),
            CondNode::Not(i) => CondNode::Not(i + offset),
            CondNode::And(a, b) => CondNode::And(a + offset, b + offset),
            CondNode::Or(a, b) => CondNode::Or(a + offset, b + offset),
            CondNode::Refused => CondNode::Refused,
        }));
        nodes.push(if and {
            CondNode::And(self.root, other.root + offset)
        } else {
            CondNode::Or(self.root, other.root + offset)
        });
        Self {
            root: nodes.len() - 1,
            nodes,
            approximate,
        }
    }
    pub fn and(&self, other: &Self) -> Self {
        self.join(other, true)
    }
    pub fn or(&self, other: &Self) -> Self {
        self.join(other, false)
    }
    pub fn leaves(&self) -> impl Iterator<Item = &L> {
        self.nodes.iter().filter_map(|n| {
            if let CondNode::Leaf(l) = n {
                Some(l)
            } else {
                None
            }
        })
    }
    pub fn map<M: Clone + PartialEq>(&self, mut map: impl FnMut(&L) -> M) -> CondGraph<M> {
        CondGraph {
            root: self.root,
            approximate: self.approximate,
            nodes: self
                .nodes
                .iter()
                .map(|n| match n {
                    CondNode::False => CondNode::False,
                    CondNode::True => CondNode::True,
                    CondNode::Leaf(l) => CondNode::Leaf(map(l)),
                    CondNode::Not(i) => CondNode::Not(*i),
                    CondNode::And(a, b) => CondNode::And(*a, *b),
                    CondNode::Or(a, b) => CondNode::Or(*a, *b),
                    CondNode::Refused => CondNode::Refused,
                })
                .collect(),
        }
    }
}
impl Diagram {
    /// Topological expressions only: every edge names an earlier node. Validate the complete
    /// graph before native BDD operations and bound total pair work as well as each operation.
    pub fn from_graph(
        graph: &CondGraph<Id<EvaluationAtom>>,
    ) -> Result<GraphCondition, KernelBoundary> {
        if graph.nodes.len() > MAX_GRAPH_NODES {
            return Err(KernelBoundary::WorkPreflight);
        }
        if graph.root >= graph.nodes.len() {
            return Err(KernelBoundary::MalformedGraph);
        }
        for (index, node) in graph.nodes.iter().enumerate() {
            let valid = match node {
                CondNode::Not(i) => *i < index,
                CondNode::And(a, b) | CondNode::Or(a, b) => *a < index && *b < index,
                _ => true,
            };
            if !valid {
                return Err(KernelBoundary::MalformedGraph);
            }
        }
        // Only the root closure is truth: orphaned/refused scratch nodes do not alter it.
        let mut live = vec![false; graph.nodes.len()];
        let mut pending = vec![graph.root];
        while let Some(i) = pending.pop() {
            if live[i] {
                continue;
            }
            live[i] = true;
            match graph.nodes[i] {
                CondNode::Not(a) => pending.push(a),
                CondNode::And(a, b) | CondNode::Or(a, b) => {
                    pending.push(a);
                    pending.push(b);
                }
                _ => {}
            }
        }
        let mut built: Vec<Option<Diagram>> = vec![None; graph.nodes.len()];
        let mut work = 0usize;
        for (i, node) in graph.nodes.iter().enumerate().filter(|(i, _)| live[*i]) {
            let get = |i: usize| built[i].as_ref().ok_or(KernelBoundary::MalformedGraph);
            let value = match node {
                CondNode::False => Self::never(),
                CondNode::True => Self::always(),
                CondNode::Leaf(a) => Self::from_atom(*a),
                CondNode::Not(a) => get(*a)?.not()?,
                CondNode::And(a, b) | CondNode::Or(a, b) => {
                    let (a, b) = (get(*a)?, get(*b)?);
                    work = work
                        .checked_add(a.bdd.size().saturating_mul(b.bdd.size()))
                        .ok_or(KernelBoundary::WorkPreflight)?;
                    if work > super::kernel::MAX_PAIR_WORK {
                        return Err(KernelBoundary::WorkPreflight);
                    }
                    if matches!(node, CondNode::And(..)) {
                        a.and(b)?
                    } else {
                        a.or(b)?
                    }
                }
                CondNode::Refused => return Err(KernelBoundary::WorkPreflight),
            };
            work = work
                .checked_add(value.bdd.size())
                .ok_or(KernelBoundary::WorkPreflight)?;
            if work > super::kernel::MAX_PAIR_WORK {
                return Err(KernelBoundary::WorkPreflight);
            }
            built[i] = Some(value);
        }
        Ok(GraphCondition {
            diagram: built[graph.root]
                .take()
                .ok_or(KernelBoundary::MalformedGraph)?,
            approximation: if graph.approximate {
                crate::domain::assertion::Approximation::Unknown
            } else {
                crate::domain::assertion::Approximation::Exact
            },
        })
    }
}
