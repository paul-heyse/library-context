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
    /// Every value link whose atom assignment was conjoined before the diagram became false.
    pub value_link_ids: Vec<Id>,
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
    let leaves: BTreeMap<Id, &TestLeaf> =
        leaves.iter().map(|leaf| (leaf.fact_id, leaf)).collect();
    let support: BTreeSet<&str> = condition.support().iter().map(String::as_str).collect();
    let mut assignments: BTreeMap<String, (bool, Id)> = BTreeMap::new();
    for link in links {
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
        let Some(value) = evaluate_exact_input(&atom, query.value, link.origin, query.builtin_namespace) else {
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
    let mut constrained = condition.clone();
    let mut witnesses = Vec::new();
    let mut remaining_work = MAX_ASSIGNMENT_WORK;
    for (encoded, (value, link_id)) in assignments {
        if witnesses.len() >= MAX_ASSIGNMENTS {
            return Err(TheoryBoundary::AssignmentBudget);
        }
        let atom = Atom::parse_encoded(&encoded).map_err(|_| TheoryBoundary::ConflictingProof)?;
        let literal = Diagram::from_atom(&atom).map_err(TheoryBoundary::Kernel)?;
        let literal = if value {
            literal
        } else {
            literal.not().map_err(TheoryBoundary::Kernel)?
        };
        remaining_work = remaining_work
            .checked_sub(
                constrained
                    .node_count()
                    .saturating_mul(literal.node_count()),
            )
            .ok_or(TheoryBoundary::AssignmentBudget)?;
        constrained = constrained.and(&literal).map_err(TheoryBoundary::Kernel)?;
        witnesses.push(link_id);
        if constrained.is_false() {
            return Ok(Some(Refutation {
                value_link_ids: witnesses,
            }));
        }
    }
    Ok(None)
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

    fn fixture(
        atom: Atom,
        origin: TestValueLinkOrigin,
    ) -> (Diagram, ValueLink, TestLeaf) {
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

    fn exact<'a>(link: &ValueLink, value: &'a Value, builtin_namespace: BuiltinNamespace)
        -> ExactInput<'a>
    {
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
                exact(&link, &Value::Str("http".to_owned()), BuiltinNamespace::Unknown),
                std::slice::from_ref(&link),
                std::slice::from_ref(&leaf),
            )
            .unwrap()
            .is_some()
        );
        assert!(
            refute_exact_input(
                &diagram,
                exact(&link, &Value::Str("sse".to_owned()), BuiltinNamespace::Unknown),
                std::slice::from_ref(&link),
                std::slice::from_ref(&leaf),
            )
            .unwrap()
            .is_none()
        );
        assert!(
            refute_exact_input(
                &diagram,
                exact(&link, &Value::Str("http".to_owned()), BuiltinNamespace::Unknown),
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
}
