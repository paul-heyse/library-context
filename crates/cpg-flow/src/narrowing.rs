//! Narrowing uses a don't-care disjunction, independently of runtime reachability.
use std::collections::HashMap;

use ty_python_core::narrowing_constraints::{NarrowingConstraints, ScopedNarrowingConstraint};
use ty_python_core::NarrowingEvaluator;

use crate::{Condition, native::EvaluationSite, predicate::Translator};

pub(crate) fn lower(t: &Translator<'_>, evaluator: &NarrowingEvaluator<'_, '_>) -> Condition {
    let graph = evaluator.narrowing_constraints();
    let synthetic = EvaluationSite::Unavailable {
        reason: "native narrowing predicate has no source coordinate",
    };
    formula(graph, evaluator.constraint(), graph.precision_lost(), |atom| {
        t.predicate(&evaluator.predicates()[atom], &synthetic)
    })
}

fn formula(
    graph: &NarrowingConstraints,
    root: ScopedNarrowingConstraint,
    precision_lost: bool,
    mut atom: impl FnMut(ty_python_core::predicate::ScopedPredicateId) -> Condition,
) -> Condition {
    fn visit(
        graph: &NarrowingConstraints,
        id: ScopedNarrowingConstraint,
        depth: usize,
        memo: &mut HashMap<ScopedNarrowingConstraint, Condition>,
        atom: &mut impl FnMut(ty_python_core::predicate::ScopedPredicateId) -> Condition,
    ) -> Condition {
        if depth > 256 || memo.len() >= 4096 {
            return Condition::refused();
        }
        if id == ScopedNarrowingConstraint::ALWAYS_TRUE {
            return Condition::always();
        }
        if id == ScopedNarrowingConstraint::ALWAYS_FALSE {
            return Condition::never();
        }
        if let Some(value) = memo.get(&id) {
            return value.clone();
        }
        let node = graph.get_interior_node(id);
        let predicate = atom(node.atom);
        let yes = visit(graph, node.if_true, depth + 1, memo, atom);
        let indifferent = visit(graph, node.if_uncertain, depth + 1, memo, atom);
        let no = visit(graph, node.if_false, depth + 1, memo, atom);
        let value = indifferent.or(&predicate.and(&yes)).or(&predicate.not().and(&no));
        memo.insert(id, value.clone());
        value
    }
    let value = visit(graph, root, 0, &mut HashMap::new(), &mut atom);
    if precision_lost { value.with_approximation() } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ty_python_core::narrowing_constraints::InteriorNode;
    use ruff_index_latest::Idx;

    #[test]
    fn uncertain_edge_is_a_disjunction_not_an_unknown_verdict() {
        let graph = NarrowingConstraints::from_test_nodes(vec![InteriorNode {
            atom: ty_python_core::predicate::ScopedPredicateId::new(0),
            if_true: ScopedNarrowingConstraint::ALWAYS_FALSE,
            if_uncertain: ScopedNarrowingConstraint::ALWAYS_TRUE,
            if_false: ScopedNarrowingConstraint::ALWAYS_FALSE,
        }]);
        let value = formula(&graph, ScopedNarrowingConstraint::new(0), false, |_| Condition::atom(crate::native::Atom::opaque("p")));
        assert!(value.is_always());
        assert!(!value.approximated());
    }

    #[test]
    fn limited_true_is_not_an_exact_tautology() {
        let graph = NarrowingConstraints::from_test_nodes(vec![]);
        let value = formula(&graph, ScopedNarrowingConstraint::ALWAYS_TRUE, true, |_| panic!("terminal"));
        assert!(value.is_always());
        assert!(value.approximated());
    }

    #[test]
    fn polarity_and_false_edges_remain_distinct() {
        let graph = NarrowingConstraints::from_test_nodes(vec![InteriorNode {
            atom: ty_python_core::predicate::ScopedPredicateId::new(0),
            if_true: ScopedNarrowingConstraint::ALWAYS_FALSE,
            if_uncertain: ScopedNarrowingConstraint::ALWAYS_FALSE,
            if_false: ScopedNarrowingConstraint::ALWAYS_TRUE,
        }]);
        let value = formula(&graph, ScopedNarrowingConstraint::new(0), false, |_| Condition::never());
        assert!(value.is_always());
        let value = formula(&graph, ScopedNarrowingConstraint::new(0), false, |_| Condition::always());
        assert!(value.is_never());
    }
}
