//! The typed primitive theory (DESIGN §15.7, §3.9): narrow exact-literal reasoning.
//!
//! Given an exact builtin input value that a checked link proves reaches an atom's operand, the
//! closed whitelist in [`evaluate`] decides the atom's truth; [`assess`] then restricts the
//! condition by those assignments. A satisfiable remainder is **unknown**, not a proof that a
//! Python execution exists. Everything outside the whitelist stays undecided.

use std::collections::BTreeMap;

use super::atom::{Predicate, Value};
use super::kernel::{Diagram, KernelBoundary};
use crate::id::Id;

const MAX_ASSIGNMENTS: usize = 32;
const MAX_ASSIGNMENT_WORK: usize = 1_000_000;

/// Whether the caller admits the standard CPython builtin-name model. Lexical resolution alone
/// cannot establish runtime namespace identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinNamespace {
    Unknown,
    StandardAssumed,
}

/// How the input reaches the atom's operand, as a checked link states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandOrigin {
    /// The entry formal reaches the operand directly, with no intervening effect.
    DirectNoEffect,
    /// The only intervening call is the evaluation's resolved one-argument builtin `type(x)`.
    ResolvedBuiltinType,
    /// A later use on the true branch of a cited, effect-free exact-type guard.
    StableAfterExactTypeGuard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TheoryBoundary {
    Kernel(KernelBoundary),
    AssignmentBudget,
    /// Two checked links assign one atom different truth values.
    ConflictingProof,
}

/// The truth of `predicate` for an exact `input`, if the whitelist decides it.
pub fn evaluate(
    predicate: &Predicate,
    input: &Value,
    origin: OperandOrigin,
    namespace: BuiltinNamespace,
) -> Option<bool> {
    match predicate {
        Predicate::IsNone => Some(matches!(input, Value::None)),
        Predicate::IsValue(value) if matches!(value, Value::None | Value::Bool(_)) => {
            Some(input == value)
        }
        Predicate::Equals(Value::Str(expected)) => match input {
            Value::Str(actual) => Some(actual == expected),
            _ => None,
        },
        // Python bool is an int subclass; that cross-type case stays unknown here.
        Predicate::Equals(Value::Int(expected)) => match input {
            Value::Int(actual) => Some(actual == expected),
            _ => None,
        },
        Predicate::MemberOf(values) if values.iter().all(|v| matches!(v, Value::Str(_))) => {
            match input {
                Value::Str(actual) => Some(values.contains(&Value::Str(actual.clone()))),
                _ => None,
            }
        }
        Predicate::Truthy => Some(match input {
            Value::None => false,
            Value::Bool(value) => *value,
            Value::Int(value) => *value != 0,
            Value::Str(value) => !value.is_empty(),
        }),
        Predicate::TypeIs(class)
            if origin == OperandOrigin::ResolvedBuiltinType
                && namespace == BuiltinNamespace::StandardAssumed
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

/// One checked assignment: an atom's truth under the exact input, and the link proving it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub atom: Id,
    pub value: bool,
    pub link: Id,
}

/// Where a condition stands under exact-input assignments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The assignments make the condition false. `links` is irredundant when the bounded
    /// minimization completed, otherwise a valid deterministic superset.
    Refuted { links: Vec<Id> },
    /// The assignments leave a Boolean model: not a witness of a concrete execution.
    CompatibleUnderModel { links: Vec<Id> },
    /// No applicable assignment.
    Unknown,
}

/// Work performed for one assessment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub assignments_applied: usize,
    pub restriction_pairs: usize,
    pub peak_nodes: usize,
}

/// Restrict `condition` by the assignments to atoms in its support, in atom order, refuting as
/// soon as it becomes false and then minimizing the refuting set within the remaining budget.
pub fn assess(
    condition: &Diagram,
    assignments: &[Assignment],
) -> (Result<Outcome, TheoryBoundary>, Work) {
    let mut work = Work {
        peak_nodes: condition.node_count(),
        ..Work::default()
    };
    let outcome = assess_inner(condition, assignments, &mut work);
    (outcome, work)
}

fn assess_inner(
    condition: &Diagram,
    assignments: &[Assignment],
    work: &mut Work,
) -> Result<Outcome, TheoryBoundary> {
    if condition.is_false() {
        return Ok(Outcome::Refuted { links: Vec::new() });
    }
    if condition.is_true() {
        return Ok(Outcome::CompatibleUnderModel { links: Vec::new() });
    }
    let mut by_atom: BTreeMap<Id, (bool, Id)> = BTreeMap::new();
    for a in assignments {
        if condition.support().binary_search(&a.atom).is_err() {
            continue;
        }
        match by_atom.get(&a.atom) {
            None => {
                by_atom.insert(a.atom, (a.value, a.link));
            }
            Some((value, _)) if *value != a.value => return Err(TheoryBoundary::ConflictingProof),
            Some(_) => {}
        }
    }
    let mut selected: Vec<(Id, bool, Id)> = Vec::new();
    let mut remaining = MAX_ASSIGNMENT_WORK;
    for (atom, (value, link)) in by_atom {
        if selected.len() >= MAX_ASSIGNMENTS {
            return Err(TheoryBoundary::AssignmentBudget);
        }
        let cost = condition.node_count().saturating_mul(selected.len() + 1);
        work.restriction_pairs = work.restriction_pairs.saturating_add(cost);
        remaining = remaining
            .checked_sub(cost)
            .ok_or(TheoryBoundary::AssignmentBudget)?;
        selected.push((atom, value, link));
        let fixed: Vec<(Id, bool)> = selected.iter().map(|(a, v, _)| (*a, *v)).collect();
        let constrained = condition
            .restrict_atoms(&fixed)
            .map_err(TheoryBoundary::Kernel)?;
        work.assignments_applied += 1;
        work.peak_nodes = work.peak_nodes.max(constrained.node_count());
        if constrained.is_false() {
            // Deletion gives an irredundant proof while the budget permits; on exhaustion the
            // current set is still a sound refutation.
            let mut index = 0;
            while index < selected.len() {
                let trial: Vec<(Id, bool)> = selected
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (a, v, _))| (i != index).then_some((*a, *v)))
                    .collect();
                let cost = condition.node_count().saturating_mul(trial.len());
                let Some(left) = remaining.checked_sub(cost) else {
                    break;
                };
                remaining = left;
                work.restriction_pairs = work.restriction_pairs.saturating_add(cost);
                let reduced = condition
                    .restrict_atoms(&trial)
                    .map_err(TheoryBoundary::Kernel)?;
                work.peak_nodes = work.peak_nodes.max(reduced.node_count());
                if reduced.is_false() {
                    selected.remove(index);
                } else {
                    index += 1;
                }
            }
            return Ok(Outcome::Refuted {
                links: selected.into_iter().map(|(_, _, link)| link).collect(),
            });
        }
    }
    if selected.is_empty() {
        Ok(Outcome::Unknown)
    } else {
        Ok(Outcome::CompatibleUnderModel {
            links: selected.into_iter().map(|(_, _, link)| link).collect(),
        })
    }
}
