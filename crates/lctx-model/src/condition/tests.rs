//! Condition kernel tests, ported from the legacy kernel onto v2 atom identity (WP0.5).

use super::atom::{Atom, Predicate, Value};
use super::kernel::*;
use super::substitution::compose_with_limits;
use super::theory::{self, Assignment, BuiltinNamespace, OperandOrigin, Outcome, TheoryBoundary};
use crate::id::Id;

/// A synthetic atom id whose position in the variable order is `n`.
fn a(n: u8) -> Id {
    let mut bytes = [0u8; 16];
    bytes[0] = n;
    Id(bytes)
}

fn v(n: u8) -> Diagram {
    Diagram::from_atom(a(n))
}

fn and(x: &Diagram, y: &Diagram) -> Diagram {
    x.and(y).unwrap()
}

fn or(x: &Diagram, y: &Diagram) -> Diagram {
    x.or(y).unwrap()
}

fn not(x: &Diagram) -> Diagram {
    x.not().unwrap()
}

#[test]
fn known_answers_have_canonical_ids() {
    let ids: Vec<(&str, String)> = vec![
        ("true", Diagram::always().id().hex()),
        ("false", Diagram::never().id().hex()),
        ("a", v(1).id().hex()),
        ("a & b", and(&v(1), &v(2)).id().hex()),
        ("a | b", or(&v(1), &v(2)).id().hex()),
        ("!a", not(&v(1)).id().hex()),
    ];
    insta::assert_debug_snapshot!(ids);
}

#[test]
fn union_implication_and_factoring_are_exact() {
    let (x, y, c) = (v(1), v(2), v(3));
    let f = or(&and(&x, &c), &and(&y, &c));
    let normal = or(&x, &y);
    assert_eq!(f.implies(&normal), Ok(true));
    assert_eq!(f.compatible(&and(&not(&x), &not(&y))), Ok(false));
    // A two-cube factor nominates no quotient; a proposed one is verified.
    let unproposed = f.given(&normal);
    assert!(!unproposed.factored);
    assert_eq!(unproposed.diagram.id(), f.id());
    let verified = f.verify_quotient(&normal, &c);
    assert!(verified.factored);
    assert_eq!(verified.diagram.id(), c.id());
    // Control: a wrong quotient is refused and the original stays authoritative.
    let wrong = f.verify_quotient(&normal, &x);
    assert!(!wrong.factored);
    assert_eq!(wrong.diagram.id(), f.id());
    // Construction order never changes identity.
    assert_eq!(f.id(), or(&and(&c, &y), &and(&c, &x)).id());
}

#[test]
fn a_single_cube_factor_is_divided_out_and_verified() {
    let (gate, body, other) = (v(1), v(2), v(3));
    let product = and(&gate, &body);
    let given = product.given(&gate);
    assert!(given.factored);
    assert_eq!(given.diagram.id(), body.id());
    let control = product.given(&other);
    assert!(!control.factored);
    assert_eq!(control.diagram.id(), product.id());
}

#[test]
fn structural_identity_ignores_redundant_support() {
    let z = v(9);
    let absorbed = or(&z, &and(&v(1), &z));
    assert_eq!(absorbed.id(), z.id());
    assert_eq!(absorbed.support(), z.support());
}

#[test]
fn the_same_spelling_at_two_evaluations_is_two_atoms() {
    let place = Some(Id([7; 16]));
    let at = |site: u8| Atom {
        evaluation: Id([site; 16]),
        operand: place,
        predicate: Predicate::Truthy,
    };
    assert_ne!(at(1).id(), at(2).id());
    let first = Diagram::from_atom(at(1).id());
    let second = Diagram::from_atom(at(2).id());
    assert_eq!(and(&first, &not(&second)).compatible(&Diagram::always()), Ok(true));
    assert_eq!(and(&first, &not(&first)).compatible(&Diagram::always()), Ok(false));
    // The argument and operand are identity; a canonical member set is order-free.
    let member = |values: Vec<Value>| Atom {
        evaluation: Id([1; 16]),
        operand: place,
        predicate: Predicate::member_of(values),
    };
    assert_eq!(
        member(vec![Value::Str("b".into()), Value::Str("a".into())]).id(),
        member(vec![Value::Str("a".into()), Value::Str("b".into()), Value::Str("a".into())]).id()
    );
    let unplaced = Atom {
        operand: None,
        ..at(1)
    };
    assert_ne!(unplaced.id(), at(1).id());
}

#[test]
fn persisted_node_closure_rejects_loss_and_corruption() {
    let (root, rows) = and(&v(1), &v(2)).root_and_nodes();
    assert_eq!(validate_nodes(root, &rows), Ok(()));
    assert_eq!(validate_nodes(root, &rows[1..]), Err(NodeValidationError::MissingNode));
    let mut corrupt = rows.clone();
    corrupt[0].low = corrupt[0].high;
    assert_eq!(validate_nodes(root, &corrupt), Err(NodeValidationError::Unreduced));
    let mut duplicate = rows.clone();
    duplicate.push(rows[0].clone());
    assert_eq!(validate_nodes(root, &duplicate), Err(NodeValidationError::DuplicateId));
    let mut forged = rows.clone();
    forged[0].atom = a(200);
    assert!(validate_nodes(root, &forged).is_err());
    let restored = Diagram::from_root_and_nodes(root, &rows).unwrap();
    assert_eq!(restored.id(), and(&v(1), &v(2)).id());
}

#[test]
fn shared_catalog_hydration_rejects_orphans_duplicates_and_size() {
    let x = v(1);
    let (root, mut nodes) = x.root_and_nodes();
    let row = ConditionRoot {
        condition_id: x.id(),
        root_id: root,
    };
    assert!(hydrate_catalog(std::slice::from_ref(&row), &nodes).is_ok());
    assert!(hydrate_catalog(&[row, row], &nodes).unwrap_err().contains("duplicate condition"));
    nodes.extend(v(2).root_and_nodes().1);
    assert!(hydrate_catalog(&[row], &nodes).unwrap_err().contains("not reachable"));
    let too_many = vec![row; MAX_CATALOG_CONDITIONS + 1];
    assert!(hydrate_catalog(&too_many, &[]).unwrap_err().contains("exceeds limits"));
    let wrong_id = ConditionRoot {
        condition_id: v(2).id(),
        root_id: root,
    };
    assert!(hydrate_catalog(&[wrong_id], &x.root_and_nodes().1).unwrap_err().contains("differs"));
}

#[test]
fn retained_limit_counts_shared_closures_before_hydration() {
    let first = and(&v(1), &v(9));
    let second = and(&v(2), &v(9));
    let mut catalog = std::collections::BTreeMap::new();
    let rows: Vec<_> = [&first, &second]
        .into_iter()
        .map(|d| {
            let (root, nodes) = d.root_and_nodes();
            for node in nodes {
                catalog.insert(node.node_id, node);
            }
            ConditionRoot {
                condition_id: d.id(),
                root_id: root,
            }
        })
        .collect();
    let nodes: Vec<_> = catalog.into_values().collect();
    assert_eq!(nodes.len(), 3);
    let refusal = hydrate_catalog_with_retained_limit(&rows, &nodes, 3).unwrap_err();
    assert!(refusal.contains("3 stored, over 3 expanded"), "{refusal}");
    assert_eq!(hydrate_catalog_with_retained_limit(&rows, &nodes, 4).unwrap().len(), 2);
}

/// The rendering truncation control: more paths than the cap is marked truncated, never
/// presented as the whole condition; exactly enough paths is not.
#[test]
fn rendering_is_bounded_and_marks_truncation() {
    let either = or(&v(1), &v(2));
    let one = either.render_terms(1).unwrap();
    assert_eq!(one.terms.len(), 1);
    assert!(one.truncated);
    let all = either.render_terms(8).unwrap();
    assert!(!all.truncated);
    assert!(all.terms.len() >= 2);
    assert_eq!(either.render_terms(MAX_PAIR_WORK + 1).err(), Some(KernelBoundary::WorkPreflight));
    assert_eq!(Diagram::never().render_terms(4).unwrap().terms.len(), 0);
}

#[test]
fn ten_atom_truth_table_matches_decisions_and_restriction() {
    let atoms: Vec<Diagram> = (0..10).map(v).collect();
    let mut left = Diagram::always();
    for pair in 0..5 {
        left = and(&left, &or(&atoms[2 * pair], &atoms[2 * pair + 1]));
    }
    let right = and(&atoms[0], &atoms[2]);
    let both = and(&left, &right);
    let either = or(&left, &right);
    let complement = not(&left);
    let mut any_both = false;
    let mut counterexample = false;
    for mask in 0..(1u32 << 10) {
        let values: Vec<bool> = (0..10).map(|i| mask & (1 << i) != 0).collect();
        let value_of = |d: &Diagram| {
            let relevant: Vec<(Id, bool)> = (0..10u8)
                .filter(|i| d.support().binary_search(&a(*i)).is_ok())
                .map(|i| (a(i), values[i as usize]))
                .collect();
            d.restrict_atoms(&relevant).unwrap().is_true()
        };
        let expected_left = (0..5).all(|p| values[2 * p] || values[2 * p + 1]);
        let expected_right = values[0] && values[2];
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

/// Budgets refuse instead of answering: a node cap, a pair-work preflight and a decision that
/// exceeds its task budget are each distinct from `false`.
#[test]
fn budgets_refuse_rather_than_answer() {
    // Bit-equality between two blocks ordered a-block then b-block: exponential in the bits.
    fn equality(bits: u8, first_half: bool, second_half: bool, gate: Option<bool>) -> Diagram {
        let mut parts = Vec::new();
        for bit in 0..bits {
            let in_first = bit < bits / 2;
            if (in_first && first_half) || (!in_first && second_half) {
                let x = CondExpr::Atom(a(10 + bit));
                let y = CondExpr::Atom(a(100 + bit));
                parts.push(CondExpr::Or(vec![
                    CondExpr::And(vec![x.clone(), y.clone()]),
                    CondExpr::And(vec![
                        CondExpr::Not(Box::new(x)),
                        CondExpr::Not(Box::new(y)),
                    ]),
                ]));
            }
        }
        if let Some(g) = gate {
            let atom = CondExpr::Atom(a(1));
            parts.push(if g { atom } else { CondExpr::Not(Box::new(atom)) });
        }
        Diagram::from_expr(&CondExpr::And(parts)).unwrap()
    }
    let left = equality(16, true, false, None);
    let right = equality(16, false, true, None);
    assert!(left.node_count() < MAX_NODES && right.node_count() < MAX_NODES);
    assert_eq!(left.compatible(&right), Ok(true));
    assert_eq!(left.and(&right).err(), Some(KernelBoundary::NodeLimit));

    let left = equality(10, true, true, Some(true));
    let right = equality(10, true, true, Some(false));
    assert!(left.node_count() * right.node_count() > MAX_PAIR_WORK);
    assert_eq!(left.compatible(&right), Ok(false));
    assert_eq!(left.and(&right).err(), Some(KernelBoundary::WorkPreflight));

    let left = equality(20, true, false, None);
    let right = equality(20, false, true, None);
    assert_eq!(left.compatible(&right), Err(KernelBoundary::WorkPreflight));
    assert_eq!(left.implies(&right), Err(KernelBoundary::WorkPreflight));
}

#[test]
fn expressions_build_under_their_budgets() {
    let expr = CondExpr::Or(vec![
        CondExpr::And(vec![CondExpr::Atom(a(1)), CondExpr::Atom(a(2))]),
        CondExpr::Not(Box::new(CondExpr::Atom(a(3)))),
    ]);
    let built = Diagram::from_expr(&expr).unwrap();
    assert_eq!(built.id(), or(&and(&v(1), &v(2)), &not(&v(3))).id());
    assert!(Diagram::from_expr(&CondExpr::True).unwrap().is_true());
    assert!(Diagram::from_expr(&CondExpr::And(vec![CondExpr::False, CondExpr::Atom(a(1))]))
        .unwrap()
        .is_false());
    let wide = CondExpr::Or((0..=128u8).map(|i| CondExpr::Atom(a(i))).collect());
    assert_eq!(Diagram::from_expr(&wide).err(), Some(KernelBoundary::AtomLimit));
    let deep = CondExpr::And(vec![CondExpr::True; 5_000]);
    assert_eq!(Diagram::from_expr(&deep).err(), Some(KernelBoundary::WorkPreflight));
}

/// Existential elimination of callee-local atoms (§15.6).
#[test]
fn existential_elimination_known_answers() {
    let (x, y, z) = (v(1), v(2), v(3));
    assert_eq!(and(&x, &y).exists(&[a(1)]).unwrap().id(), y.id());
    // (x ∧ y) ∨ (¬x ∧ z), eliminating x, is y ∨ z.
    let select = or(&and(&x, &y), &and(&not(&x), &z));
    assert_eq!(select.exists(&[a(1)]).unwrap().id(), or(&y, &z).id());
    // x ∧ ¬x stays false; x alone becomes true.
    assert!(x.exists(&[a(1)]).unwrap().is_true());
    assert!(and(&x, &not(&x)).exists(&[a(1)]).unwrap().is_false());
    // An atom outside the support changes nothing.
    assert_eq!(y.exists(&[a(1)]).unwrap().id(), y.id());
    // Eliminating everything leaves a constant.
    assert!(select.exists(&[a(1), a(2), a(3)]).unwrap().is_true());
}

#[test]
fn simultaneous_substitution_swaps_without_capture() {
    let (x, y) = (v(1), v(2));
    let original = and(&x, &not(&y));
    let swapped = original.substitute_atoms(&[(a(1), &y), (a(2), &x)]).unwrap();
    let reversed = original.substitute_atoms(&[(a(2), &x), (a(1), &y)]).unwrap();
    assert_eq!(swapped.id(), reversed.id());
    for vx in [false, true] {
        for vy in [false, true] {
            assert_eq!(
                swapped.restrict_atoms(&[(a(1), vx), (a(2), vy)]).unwrap().is_true(),
                vy && !vx
            );
        }
    }
    assert_eq!(x.substitute_atoms(&[(a(1), &not(&x))]).unwrap().id(), not(&x).id());
    assert_eq!(original.substitute_atoms(&[]).unwrap().id(), original.id());
}

#[test]
fn substitution_shares_targets_and_refuses_malformed_bindings() {
    let (x, y, z) = (v(1), v(2), v(3));
    let source = and(&x, &not(&y));
    let collapsed = source.substitute_atoms(&[(a(1), &z), (a(2), &z)]).unwrap();
    assert!(collapsed.is_false());
    assert!(collapsed.support().is_empty());
    assert_eq!(
        source.substitute_atoms(&[(a(1), &Diagram::always())]).unwrap().id(),
        not(&y).id()
    );
    assert_eq!(
        x.substitute_atoms(&[(a(1), &y), (a(1), &y)]).err(),
        Some(KernelBoundary::TransferUnsupported)
    );
    assert_eq!(
        x.substitute_atoms(&[(a(2), &y)]).err(),
        Some(KernelBoundary::TransferUnsupported)
    );
    assert_eq!(
        compose_with_limits(&x, &[(a(1), &y)], 0, 1_000_000).err(),
        Some(KernelBoundary::WorkPreflight)
    );
    assert_eq!(
        compose_with_limits(&x, &[(a(1), &y)], MAX_PAIR_WORK, 0).err(),
        Some(KernelBoundary::NodeLimit)
    );
    let conjunction = and(&x, &y);
    assert_eq!(
        compose_with_limits(&conjunction, &[], 5, 1_000_000).err(),
        Some(KernelBoundary::WorkPreflight)
    );
}

#[test]
fn boundary_codes_round_trip() {
    for b in [
        KernelBoundary::AtomLimit,
        KernelBoundary::WorkPreflight,
        KernelBoundary::NodeLimit,
        KernelBoundary::TransferUnsupported,
    ] {
        assert_eq!(KernelBoundary::from_code(b.code()), Some(b));
    }
    assert_eq!(KernelBoundary::from_code("source_over_budget"), None);
}

#[test]
fn the_primitive_whitelist_decides_only_exact_builtins() {
    use BuiltinNamespace::*;
    use OperandOrigin::*;
    let s = |x: &str| Value::Str(x.into());
    assert_eq!(theory::evaluate(&Predicate::IsNone, &Value::None, DirectNoEffect, Unknown), Some(true));
    assert_eq!(theory::evaluate(&Predicate::IsNone, &Value::Int(0), DirectNoEffect, Unknown), Some(false));
    assert_eq!(
        theory::evaluate(&Predicate::Equals(s("a")), &s("b"), DirectNoEffect, Unknown),
        Some(false)
    );
    // Cross-type equality stays unknown (bool is an int subclass).
    assert_eq!(
        theory::evaluate(&Predicate::Equals(Value::Int(1)), &Value::Bool(true), DirectNoEffect, Unknown),
        None
    );
    assert_eq!(
        theory::evaluate(&Predicate::member_of([s("x"), s("y")]), &s("y"), DirectNoEffect, Unknown),
        Some(true)
    );
    assert_eq!(theory::evaluate(&Predicate::Truthy, &s(""), DirectNoEffect, Unknown), Some(false));
    // `type(p) is C` needs the resolved builtin operand and the assumed standard namespace.
    let type_is = Predicate::TypeIs("str".into());
    assert_eq!(theory::evaluate(&type_is, &s("x"), ResolvedBuiltinType, StandardAssumed), Some(true));
    assert_eq!(theory::evaluate(&type_is, &s("x"), ResolvedBuiltinType, Unknown), None);
    assert_eq!(theory::evaluate(&type_is, &s("x"), DirectNoEffect, StandardAssumed), None);
    assert_eq!(
        theory::evaluate(&Predicate::IsInstance("str".into()), &s("x"), DirectNoEffect, StandardAssumed),
        None
    );
    assert_eq!(theory::evaluate(&Predicate::opaque("f(x)"), &s("x"), DirectNoEffect, StandardAssumed), None);
}

#[test]
fn exact_assignments_refute_minimally_or_stay_unknown() {
    let (x, y, z) = (v(1), v(2), v(3));
    let condition = and(&x, &or(&y, &z));
    let assign = |n: u8, value: bool, link: u8| Assignment {
        atom: a(n),
        value,
        link: Id([link; 16]),
    };
    // x false refutes; the y assignment is redundant and minimized away.
    let (outcome, work) = theory::assess(&condition, &[assign(2, true, 20), assign(1, false, 10)]);
    assert_eq!(outcome, Ok(Outcome::Refuted { links: vec![Id([10; 16])] }));
    assert!(work.assignments_applied >= 1);
    // Satisfiable remainder: compatible under the model, not a proof of execution.
    let (outcome, _) = theory::assess(&condition, &[assign(1, true, 10)]);
    assert_eq!(outcome, Ok(Outcome::CompatibleUnderModel { links: vec![Id([10; 16])] }));
    // No applicable assignment: unknown.
    let (outcome, _) = theory::assess(&condition, &[assign(9, true, 90)]);
    assert_eq!(outcome, Ok(Outcome::Unknown));
    // Conflicting proofs for one atom.
    let (outcome, _) = theory::assess(&condition, &[assign(1, true, 10), assign(1, false, 11)]);
    assert_eq!(outcome, Err(TheoryBoundary::ConflictingProof));
}
