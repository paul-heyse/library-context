//! Bounded Boolean condition questions over evaluation atoms (ADR-0024).
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::condition::Condition;
use crate::id::{Id, IdHasher};
use biodivine_lib_bdd::{Bdd, BddPointer, BddVariableSet, op_function};

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

/// A content-addressed nonterminal. No library-local variable index or pointer is persisted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagramNode {
    pub node_id: Id,
    pub atom: String,
    pub low: Id,
    pub high: Id,
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
    visit(
        root,
        &by_id,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut BTreeSet::new(),
        1,
    )
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
        Ok(Self { support, ctx, bdd })
    }

    pub fn node_count(&self) -> usize {
        self.bdd.size()
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
        Ok(Self { support, ctx, bdd })
    }

    pub fn or(&self, other: &Self) -> Result<Self, KernelBoundary> {
        let (support, ctx, left, right) = self.union(other)?;
        let bdd = apply(&left, &right, op_function::or)?;
        Ok(Self { support, ctx, bdd })
    }

    pub fn not(&self) -> Result<Self, KernelBoundary> {
        if self.node_count() > MAX_NODES {
            return Err(KernelBoundary::NodeLimit);
        }
        Ok(Self {
            support: self.support.clone(),
            ctx: self.ctx.clone(),
            bdd: self.bdd.not(),
        })
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
        Ok(!apply(&left, &right, op_function::and)?.is_false())
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
        if right.size() > MAX_NODES {
            return Err(KernelBoundary::NodeLimit);
        }
        Ok(apply(&left, &right.not(), op_function::and)?.is_false())
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
        assert!(first.support.len() > second.support.len());
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
