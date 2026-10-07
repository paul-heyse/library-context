use crate::contracts::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn compatible(left: &Assignment, right: &Assignment) -> bool {
    left.iter()
        .all(|(key, value)| right.get(key).is_none_or(|other| value == other))
}
fn merge(left: &Assignment, right: &Assignment) -> Assignment {
    let mut result = left.clone();
    result.extend(right.clone());
    result
}
fn unique(rows: Vec<Assignment>) -> Vec<Assignment> {
    rows.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn evaluate(
    expression: &Witness,
    leaves: &BTreeMap<String, Vec<Assignment>>,
) -> Vec<Assignment> {
    match expression {
        Witness::Leaf { predicate } => leaves.get(predicate).cloned().unwrap_or_default(),
        Witness::Any { children } => unique(
            children
                .iter()
                .flat_map(|child| evaluate(child, leaves))
                .collect(),
        ),
        Witness::All { children } => {
            if children.is_empty() {
                return Vec::new();
            }
            let mut result = vec![Assignment::new()];
            for child in children {
                let next = evaluate(child, leaves);
                result = unique(
                    result
                        .iter()
                        .flat_map(|left| {
                            next.iter()
                                .filter(|right| compatible(left, right))
                                .map(|right| merge(left, right))
                        })
                        .collect(),
                );
            }
            result
        }
        Witness::Exists { variables, child } => unique(
            evaluate(child, leaves)
                .into_iter()
                .map(|mut row| {
                    for key in variables {
                        row.remove(key);
                    }
                    row
                })
                .collect(),
        ),
    }
}

/// Separate brute-force reference: enumerate complete proof choices, then check all bindings.
/// It intentionally does not call the natural-join implementation.
pub fn exhaustive_reference(
    expression: &Witness,
    leaves: &BTreeMap<String, Vec<Assignment>>,
) -> Vec<Assignment> {
    fn proofs(expr: &Witness, leaves: &BTreeMap<String, Vec<Assignment>>) -> Vec<Vec<Assignment>> {
        match expr {
            Witness::Leaf { predicate } => leaves
                .get(predicate)
                .into_iter()
                .flatten()
                .map(|row| vec![row.clone()])
                .collect(),
            Witness::Any { children } => children
                .iter()
                .flat_map(|child| proofs(child, leaves))
                .collect(),
            Witness::All { children } => {
                if children.is_empty() {
                    return Vec::new();
                }
                let mut choices = vec![Vec::new()];
                for child in children {
                    let child_proofs = proofs(child, leaves);
                    choices = choices
                        .into_iter()
                        .flat_map(|choice| {
                            child_proofs.iter().map(move |proof| {
                                let mut next = choice.clone();
                                next.extend(proof.clone());
                                next
                            })
                        })
                        .collect();
                }
                choices
            }
            Witness::Exists { variables, child } => collapse(proofs(child, leaves))
                .into_iter()
                .map(|mut row| {
                    for key in variables {
                        row.remove(key);
                    }
                    vec![row]
                })
                .collect(),
        }
    }
    fn collapse(proofs: Vec<Vec<Assignment>>) -> Vec<Assignment> {
        let mut result = BTreeSet::new();
        for proof in proofs {
            let mut row = Assignment::new();
            let mut valid = true;
            for part in proof {
                for (key, value) in part {
                    if row.get(&key).is_some_and(|old| old != &value) {
                        valid = false;
                    }
                    row.insert(key, value);
                }
            }
            if valid {
                result.insert(row);
            }
        }
        result.into_iter().collect()
    }
    collapse(proofs(expression, leaves))
}

pub fn validate(
    expression: &Witness,
    names: &BTreeSet<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > 24 {
        return Err("witness nesting exceeds finite kernel bound".into());
    }
    match expression {
        Witness::Leaf { predicate } if !names.contains(predicate) => {
            Err(format!("unknown predicate {predicate}"))
        }
        Witness::All { children } | Witness::Any { children } => {
            if children.is_empty() {
                return Err("empty positive witness expression".into());
            }
            for child in children {
                validate(child, names, depth + 1)?;
            }
            Ok(())
        }
        Witness::Exists { variables, child } => {
            // Projection before a surrounding conjunction could accidentally hide incompatibility.
            // Only the complete top-level conjunction may be existentially projected.
            if depth != 0 || variables.is_empty() {
                return Err("Exists must project the complete top-level witness".into());
            }
            validate(child, names, depth + 1)
        }
        _ => Ok(()),
    }
}
