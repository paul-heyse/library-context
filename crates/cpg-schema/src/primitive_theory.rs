//! Narrow exact-literal reasoning over checked entry-value links (ADR-0024).
//!
//! This layer only translates a query-supplied exact builtin value into assignments for source
//! evaluation atoms whose operand is proved to be the same entry formal. The existing bounded
//! BDD performs the Boolean composition. A satisfiable remainder is **unknown**, not a proof
//! that a Python execution exists. Call only after the shared publication/native validators have
//! admitted the link and leaf tables for one generation.

use std::collections::{BTreeMap, BTreeSet};

use crate::behavior::FlowTestValueLinksRow;
use crate::codebook::TestValueLinkOrigin;
use crate::condition::{Atom, Value};
use crate::condition_kernel::{Diagram, KernelBoundary};
use crate::id::{Digest, Id, IdHasher};
use crate::tables::FlowTestLeavesRow;

const MAX_ASSIGNMENTS: usize = 32;
const MAX_ASSIGNMENT_WORK: usize = 1_000_000;

/// The checked part of a value-link row needed by exact-input reasoning. The full publication
/// row retains source spans and reaching facts; this is its lossless semantic query projection.
#[derive(Clone, Debug)]
pub struct ValueLink {
    pub snapshot_id: Id,
    pub link_id: Id,
    pub operation_node_id: Id,
    pub formal_node_id: Id,
    pub module_node_id: Id,
    pub leaf_fact_id: Id,
    pub atom_id: Id,
    pub condition_id: Id,
    pub place: String,
    pub origin: TestValueLinkOrigin,
    pub effect_model_digest: Digest,
}

impl From<&FlowTestValueLinksRow> for ValueLink {
    fn from(row: &FlowTestValueLinksRow) -> Self {
        Self {
            snapshot_id: row.snapshot_id,
            link_id: row.link_id,
            operation_node_id: row.operation_node_id,
            formal_node_id: row.formal_node_id,
            module_node_id: row.module_node_id,
            leaf_fact_id: row.leaf_fact_id,
            atom_id: row.atom_id,
            condition_id: row.condition_id,
            place: row.place.clone(),
            origin: row.origin,
            effect_model_digest: row.effect_model_digest,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TestLeaf {
    pub snapshot_id: Id,
    pub fact_id: Id,
    pub module_node_id: Id,
    pub atom_id: Id,
    pub condition_id: Id,
    pub atom: String,
}

impl From<&FlowTestLeavesRow> for TestLeaf {
    fn from(row: &FlowTestLeavesRow) -> Self {
        Self {
            snapshot_id: row.snapshot_id,
            fact_id: row.fact_id,
            module_node_id: row.module_node_id,
            atom_id: row.atom_id,
            condition_id: row.condition_id,
            atom: row.atom.clone(),
        }
    }
}

/// Whether a caller has explicitly admitted the standard CPython builtin-name model for this
/// generation. Lexical reference resolution alone cannot establish runtime namespace identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinNamespace {
    Unknown,
    StandardAssumed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TheoryBoundary {
    Kernel(KernelBoundary),
    AssignmentBudget,
    ConflictingProof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refutation {
    /// An irredundant set of value links when the bounded minimization completed; otherwise a
    /// valid deterministic superset. Every retained link is an admitted exact assignment.
    pub value_link_ids: Vec<Id>,
}

/// A satisfiable diagram is only a may-model result when an applicable value link was checked,
/// or the diagram is unconditionally true. With no bridge, it remains unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactInputOutcome {
    Refuted(Refutation),
    CompatibleUnderModel { value_link_ids: Vec<Id> },
    Unknown,
}

/// Work performed for one path. `bdd_preflight_pairs` is a conservative restriction-work
/// estimate (diagram nodes times assigned atoms), not a count of visited BDD nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TheoryWork {
    pub links_examined: usize,
    pub assignments_applied: usize,
    pub bdd_preflight_pairs: usize,
    pub peak_bdd_nodes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactInputAssessment {
    pub outcome: Result<ExactInputOutcome, TheoryBoundary>,
    pub work: TheoryWork,
}

/// One operation/formal and its exact query value under an explicit builtin namespace model.
#[derive(Clone, Copy)]
pub struct ExactInput<'a> {
    pub operation_node_id: Id,
    pub formal_node_id: Id,
    pub value: &'a Value,
    pub builtin_namespace: BuiltinNamespace,
    pub effect_model_digest: Digest,
}

/// Refute one condition for one exact query argument. `Ok(None)` is unknown: missing links,
/// unsupported Python operators and a satisfiable BDD all leave executions possible.
pub fn refute_exact_input(
    condition: &Diagram,
    query: ExactInput<'_>,
    links: &[ValueLink],
    leaves: &[TestLeaf],
) -> Result<Option<Refutation>, TheoryBoundary> {
    match assess_exact_input(condition, query, links, leaves)? {
        ExactInputOutcome::Refuted(proof) => Ok(Some(proof)),
        ExactInputOutcome::CompatibleUnderModel { .. } | ExactInputOutcome::Unknown => Ok(None),
    }
}

/// Path-local exact-input assessment. `CompatibleUnderModel` means the checked assignments
/// leave a Boolean model; it is not a witness of concrete Python execution.
pub fn assess_exact_input(
    condition: &Diagram,
    query: ExactInput<'_>,
    links: &[ValueLink],
    leaves: &[TestLeaf],
) -> Result<ExactInputOutcome, TheoryBoundary> {
    assess_exact_input_with_work(condition, query, links, leaves).outcome
}

/// Retain bounded-work evidence even when an assignment or kernel budget refuses a path.
pub fn assess_exact_input_with_work(
    condition: &Diagram,
    query: ExactInput<'_>,
    links: &[ValueLink],
    leaves: &[TestLeaf],
) -> ExactInputAssessment {
    let mut work = TheoryWork {
        peak_bdd_nodes: condition.node_count(),
        ..TheoryWork::default()
    };
    let outcome = assess_exact_input_inner(condition, query, links, leaves, &mut work);
    ExactInputAssessment { outcome, work }
}

fn assess_exact_input_inner(
    condition: &Diagram,
    query: ExactInput<'_>,
    links: &[ValueLink],
    leaves: &[TestLeaf],
    work: &mut TheoryWork,
) -> Result<ExactInputOutcome, TheoryBoundary> {
    if condition.is_false() {
        return Ok(ExactInputOutcome::Refuted(Refutation {
            value_link_ids: Vec::new(),
        }));
    }
    if condition.is_true() {
        return Ok(ExactInputOutcome::CompatibleUnderModel {
            value_link_ids: Vec::new(),
        });
    }
    let leaves: BTreeMap<Id, &TestLeaf> = leaves.iter().map(|leaf| (leaf.fact_id, leaf)).collect();
    let support: BTreeSet<&str> = condition.support().iter().map(String::as_str).collect();
    let mut assignments: BTreeMap<String, (bool, Id)> = BTreeMap::new();
    for link in links {
        work.links_examined += 1;
        if link.operation_node_id != query.operation_node_id
            || link.formal_node_id != query.formal_node_id
            || link.effect_model_digest != query.effect_model_digest
        {
            continue;
        }
        let Some(leaf) = leaves.get(&link.leaf_fact_id) else {
            continue;
        };
        if leaf.snapshot_id != link.snapshot_id
            || leaf.module_node_id != link.module_node_id
            || leaf.atom_id != link.atom_id
            || leaf.condition_id != link.condition_id
            || IdHasher::new("bdd-atom").str(&leaf.atom).finish_id() != link.atom_id
            || !support.contains(leaf.atom.as_str())
        {
            continue;
        }
        let Ok(atom @ Atom::Evaluated { .. }) = Atom::parse_encoded(&leaf.atom) else {
            continue;
        };
        if atom.place() != Some(link.place.as_str()) {
            continue;
        }
        let Some(value) =
            evaluate_exact_input(&atom, query.value, link.origin, query.builtin_namespace)
        else {
            continue;
        };
        match assignments.entry(leaf.atom.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert((value, link.link_id));
            }
            std::collections::btree_map::Entry::Occupied(entry) if entry.get().0 != value => {
                return Err(TheoryBoundary::ConflictingProof);
            }
            _ => {}
        }
    }
    let mut selected = Vec::new();
    let mut remaining_work = MAX_ASSIGNMENT_WORK;
    for (encoded, (value, link_id)) in assignments {
        if selected.len() >= MAX_ASSIGNMENTS {
            return Err(TheoryBoundary::AssignmentBudget);
        }
        let restriction_work = condition.node_count().saturating_mul(selected.len() + 1);
        work.bdd_preflight_pairs = work.bdd_preflight_pairs.saturating_add(restriction_work);
        remaining_work = remaining_work
            .checked_sub(restriction_work)
            .ok_or(TheoryBoundary::AssignmentBudget)?;
        selected.push((encoded, value, link_id));
        let fixed: Vec<(&str, bool)> = selected
            .iter()
            .map(|(atom, value, _)| (atom.as_str(), *value))
            .collect();
        let constrained = condition
            .restrict_atoms(&fixed)
            .map_err(TheoryBoundary::Kernel)?;
        work.assignments_applied += 1;
        work.peak_bdd_nodes = work.peak_bdd_nodes.max(constrained.node_count());
        if constrained.is_false() {
            // Deletion gives an irredundant proof when the remaining work budget permits. On
            // exhaustion, the current set is still a sound refutation; no extra work is spent.
            let mut index = 0;
            while index < selected.len() {
                let trial: Vec<(&str, bool)> = selected
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (atom, value, _))| (i != index).then_some((atom.as_str(), *value)))
                    .collect();
                let cost = condition.node_count().saturating_mul(trial.len());
                let Some(left) = remaining_work.checked_sub(cost) else {
                    break;
                };
                remaining_work = left;
                work.bdd_preflight_pairs = work.bdd_preflight_pairs.saturating_add(cost);
                let reduced = condition
                    .restrict_atoms(&trial)
                    .map_err(TheoryBoundary::Kernel)?;
                work.peak_bdd_nodes = work.peak_bdd_nodes.max(reduced.node_count());
                if reduced.is_false() {
                    selected.remove(index);
                } else {
                    index += 1;
                }
            }
            return Ok(ExactInputOutcome::Refuted(Refutation {
                value_link_ids: selected.into_iter().map(|(_, _, link_id)| link_id).collect(),
            }));
        }
    }
    if selected.is_empty() {
        Ok(ExactInputOutcome::Unknown)
    } else {
        Ok(ExactInputOutcome::CompatibleUnderModel {
            value_link_ids: selected.into_iter().map(|(_, _, link_id)| link_id).collect(),
        })
    }
}

fn evaluate_exact_input(
    atom: &Atom,
    input: &Value,
    link_origin: TestValueLinkOrigin,
    builtin_namespace: BuiltinNamespace,
) -> Option<bool> {
    let Atom::Evaluated { atom, .. } = atom else {
        return None;
    };
    match atom.as_ref() {
        Atom::IsNone { .. } => Some(matches!(input, Value::None)),
        Atom::IsValue { value, .. } if matches!(value, Value::None | Value::Bool(_)) => {
            Some(input == value)
        }
        Atom::Equals {
            value: Value::Str(expected),
            ..
        } => match input {
            Value::Str(actual) => Some(actual == expected),
            _ => None,
        },
        Atom::Equals {
            value: Value::Int(expected),
            ..
        } => match input {
            // Python bool is an int subclass. Keep that cross-type case unknown here rather
            // than importing Python's coercion rules into this narrow literal theory.
            Value::Int(actual) => Some(actual == expected),
            _ => None,
        },
        Atom::MemberOf { values, .. }
            if values.iter().all(|value| matches!(value, Value::Str(_))) =>
        {
            match input {
                Value::Str(actual) => Some(values.contains(&Value::Str(actual.clone()))),
                _ => None,
            }
        }
        Atom::Truthy { .. } => Some(match input {
            Value::None => false,
            Value::Bool(value) => *value,
            Value::Int(value) => *value != 0,
            Value::Str(value) => !value.is_empty(),
        }),
        Atom::TypeIs { class, .. }
            if link_origin == TestValueLinkOrigin::ResolvedBuiltinTypeOperand
                && builtin_namespace == BuiltinNamespace::StandardAssumed
                && matches!(class.as_str(), "str" | "int" | "bool") =>
        {
            Some(matches!(
                (class.as_str(), input),
                ("str", Value::Str(_)) | ("int", Value::Int(_)) | ("bool", Value::Bool(_))
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::LexicalScopeKind;
    use crate::condition::EvaluationIdentity;

    fn fixture(atom: Atom, origin: TestValueLinkOrigin) -> (Diagram, ValueLink, TestLeaf) {
        let atom = atom.evaluated(EvaluationIdentity::Site {
            module: Id([2; 16]).hex(),
            start: 10,
            end: 20,
        });
        let diagram = Diagram::from_atom(&atom).unwrap();
        let encoded = atom.encode();
        let atom_id = IdHasher::new("bdd-atom").str(&encoded).finish_id();
        let snapshot = Id([1; 16]);
        let module = Id([2; 16]);
        let operation = Id([3; 16]);
        let formal = Id([4; 16]);
        let leaf = Id([5; 16]);
        let use_id = Id([6; 16]);
        let link = FlowTestValueLinksRow {
            snapshot_id: snapshot,
            link_id: Id([7; 16]),
            operation_node_id: operation,
            formal_node_id: formal,
            module_node_id: module,
            leaf_fact_id: leaf,
            atom_id,
            use_id,
            use_fact_id: Id([8; 16]),
            reaching_fact_id: Id([9; 16]),
            definition_fact_id: Id([10; 16]),
            operand_start_byte: 15,
            operand_end_byte: 16,
            place: "x".to_owned(),
            condition_id: diagram.id(),
            origin,
            effect_model_digest: Digest([11; 32]),
            stability_origin_id: None,
            stability_condition_id: None,
        };
        let leaf_row = FlowTestLeavesRow {
            snapshot_id: snapshot,
            fact_id: leaf,
            module_node_id: module,
            scope_kind: LexicalScopeKind::Function,
            scope_start_byte: Some(1),
            scope_end_byte: Some(2),
            predicate_key: "fixture".to_owned(),
            test_start_byte: 10,
            test_end_byte: 20,
            condition_id: diagram.id(),
            atom_id,
            atom: encoded,
            leaf_start_byte: 10,
            leaf_end_byte: 20,
        };
        (diagram, ValueLink::from(&link), TestLeaf::from(&leaf_row))
    }

    fn exact<'a>(
        link: &ValueLink,
        value: &'a Value,
        builtin_namespace: BuiltinNamespace,
    ) -> ExactInput<'a> {
        ExactInput {
            operation_node_id: link.operation_node_id,
            formal_node_id: link.formal_node_id,
            value,
            builtin_namespace,
            effect_model_digest: link.effect_model_digest,
        }
    }

    #[test]
    fn exact_string_input_refutes_unequal_string_equality_with_a_link() {
        let (diagram, link, leaf) = fixture(
            Atom::Equals {
                place: "x".to_owned(),
                value: Value::Str("sse".to_owned()),
            },
            TestValueLinkOrigin::DirectParameterReachNoEffect,
        );
        assert!(
            refute_exact_input(
                &diagram,
                exact(
                    &link,
                    &Value::Str("http".to_owned()),
                    BuiltinNamespace::Unknown
                ),
                std::slice::from_ref(&link),
                std::slice::from_ref(&leaf),
            )
            .unwrap()
            .is_some()
        );
        let assessment = assess_exact_input_with_work(
            &diagram,
            exact(
                &link,
                &Value::Str("http".to_owned()),
                BuiltinNamespace::Unknown,
            ),
            std::slice::from_ref(&link),
            std::slice::from_ref(&leaf),
        );
        assert!(matches!(
            assessment.outcome,
            Ok(ExactInputOutcome::Refuted(_))
        ));
        assert_eq!(assessment.work.links_examined, 1);
        assert_eq!(assessment.work.assignments_applied, 1);
        assert!(assessment.work.bdd_preflight_pairs > 0);
        assert!(assessment.work.peak_bdd_nodes >= diagram.node_count());
        assert!(
            refute_exact_input(
                &diagram,
                exact(
                    &link,
                    &Value::Str("sse".to_owned()),
                    BuiltinNamespace::Unknown
                ),
                std::slice::from_ref(&link),
                std::slice::from_ref(&leaf),
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            assess_exact_input(
                &diagram,
                exact(
                    &link,
                    &Value::Str("sse".to_owned()),
                    BuiltinNamespace::Unknown
                ),
                std::slice::from_ref(&link),
                std::slice::from_ref(&leaf),
            )
            .unwrap(),
            ExactInputOutcome::CompatibleUnderModel {
                value_link_ids: vec![link.link_id]
            }
        );
        assert_eq!(
            assess_exact_input(
                &diagram,
                exact(
                    &link,
                    &Value::Str("sse".to_owned()),
                    BuiltinNamespace::Unknown
                ),
                &[],
                std::slice::from_ref(&leaf),
            )
            .unwrap(),
            ExactInputOutcome::Unknown
        );
        let unlinked = assess_exact_input_with_work(
            &diagram,
            exact(
                &link,
                &Value::Str("sse".to_owned()),
                BuiltinNamespace::Unknown,
            ),
            &[],
            std::slice::from_ref(&leaf),
        );
        assert_eq!(unlinked.outcome, Ok(ExactInputOutcome::Unknown));
        assert_eq!(unlinked.work.links_examined, 0);
        assert_eq!(unlinked.work.bdd_preflight_pairs, 0);
        assert!(
            refute_exact_input(
                &diagram,
                exact(
                    &link,
                    &Value::Str("http".to_owned()),
                    BuiltinNamespace::Unknown
                ),
                &[],
                &[leaf],
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn bool_integer_cross_equality_stays_unknown() {
        let (diagram, link, leaf) = fixture(
            Atom::Equals {
                place: "x".to_owned(),
                value: Value::Int(1),
            },
            TestValueLinkOrigin::DirectParameterReachNoEffect,
        );
        assert!(
            refute_exact_input(
                &diagram,
                exact(&link, &Value::Bool(true), BuiltinNamespace::Unknown),
                &[link],
                &[leaf],
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn assignment_cap_retains_work_and_never_refutes() {
        let mut condition = Diagram::always();
        let mut links = Vec::new();
        let mut leaves = Vec::new();
        for index in 0..=MAX_ASSIGNMENTS {
            let (atom_condition, mut link, mut leaf) = fixture(
                Atom::IsNone {
                    place: format!("x{index}"),
                },
                TestValueLinkOrigin::DirectParameterReachNoEffect,
            );
            condition = condition.and(&atom_condition).unwrap();
            link.place = format!("x{index}");
            link.link_id = Id([index as u8 + 1; 16]);
            link.leaf_fact_id = Id([index as u8 + 1; 16]);
            leaf.fact_id = link.leaf_fact_id;
            links.push(link);
            leaves.push(leaf);
        }
        for (link, leaf) in links.iter_mut().zip(&mut leaves) {
            link.condition_id = condition.id();
            leaf.condition_id = condition.id();
        }
        let assessment = assess_exact_input_with_work(
            &condition,
            exact(&links[0], &Value::None, BuiltinNamespace::Unknown),
            &links,
            &leaves,
        );
        assert_eq!(assessment.outcome, Err(TheoryBoundary::AssignmentBudget));
        assert_eq!(assessment.work.links_examined, MAX_ASSIGNMENTS + 1);
        assert_eq!(assessment.work.assignments_applied, MAX_ASSIGNMENTS);
        assert!(assessment.work.bdd_preflight_pairs > 0);
    }

    #[test]
    fn guarded_builtin_type_excludes_none_only_with_its_origin() {
        let (diagram, link, leaf) = fixture(
            Atom::TypeIs {
                place: "x".to_owned(),
                class: "str".to_owned(),
            },
            TestValueLinkOrigin::ResolvedBuiltinTypeOperand,
        );
        assert!(
            refute_exact_input(
                &diagram,
                exact(&link, &Value::None, BuiltinNamespace::StandardAssumed),
                std::slice::from_ref(&link),
                std::slice::from_ref(&leaf),
            )
            .unwrap()
            .is_some()
        );
        assert!(
            refute_exact_input(
                &diagram,
                exact(&link, &Value::None, BuiltinNamespace::Unknown),
                std::slice::from_ref(&link),
                std::slice::from_ref(&leaf),
            )
            .unwrap()
            .is_none()
        );
        let mut unproved = link;
        unproved.origin = TestValueLinkOrigin::DirectParameterReachNoEffect;
        assert!(
            refute_exact_input(
                &diagram,
                exact(&unproved, &Value::None, BuiltinNamespace::StandardAssumed),
                &[unproved],
                &[leaf],
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn string_membership_and_non_bool_integer_equality_refute_exact_inputs() {
        let cases = [
            (
                Atom::member_of(
                    "x".to_owned(),
                    vec![Value::Str("sse".to_owned()), Value::Str("http".to_owned())],
                ),
                Value::Str("stdio".to_owned()),
                true,
            ),
            (
                Atom::Equals {
                    place: "x".to_owned(),
                    value: Value::Int(2),
                },
                Value::Int(3),
                true,
            ),
            (
                Atom::member_of("x".to_owned(), vec![Value::Int(1)]),
                Value::Bool(true),
                false,
            ),
        ];
        for (atom, value, refuted) in cases {
            let (diagram, link, leaf) =
                fixture(atom, TestValueLinkOrigin::DirectParameterReachNoEffect);
            assert_eq!(
                refute_exact_input(
                    &diagram,
                    exact(&link, &value, BuiltinNamespace::Unknown),
                    &[link.clone()],
                    &[leaf],
                )
                .unwrap()
                .is_some(),
                refuted,
            );
        }
    }

    #[test]
    fn refutation_drops_an_irrelevant_earlier_link() {
        let input = Value::Str("query".to_owned());
        let mut diagrams = Vec::new();
        let mut links = Vec::new();
        let mut leaves = Vec::new();
        for (index, expected) in ["query", "other", "another"].into_iter().enumerate() {
            let (diagram, mut link, mut leaf) = fixture(
                Atom::Equals {
                    place: format!("x{index}"),
                    value: Value::Str(expected.to_owned()),
                },
                TestValueLinkOrigin::DirectParameterReachNoEffect,
            );
            link.place = format!("x{index}");
            link.link_id = Id([index as u8 + 20; 16]);
            link.leaf_fact_id = Id([index as u8 + 30; 16]);
            leaf.fact_id = link.leaf_fact_id;
            diagrams.push(diagram);
            links.push(link);
            leaves.push(leaf);
        }
        // x0 is true for this input, but x1 and x2 being false refutes the disjunction
        // independently of x0. The old prefix citation included all three links.
        let condition = diagrams[0].and(&diagrams[1].or(&diagrams[2]).unwrap()).unwrap();
        for (link, leaf) in links.iter_mut().zip(&mut leaves) {
            link.condition_id = condition.id();
            leaf.condition_id = condition.id();
        }
        let proof = refute_exact_input(
            &condition,
            exact(&links[0], &input, BuiltinNamespace::Unknown),
            &links,
            &leaves,
        )
        .unwrap()
        .unwrap();
        assert_eq!(proof.value_link_ids, vec![links[1].link_id, links[2].link_id]);
    }
}
