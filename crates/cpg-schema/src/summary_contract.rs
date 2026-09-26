//! Shared admission of linked, fixed Boolean controls in finite summary proofs.
//! These checks do not prove argument-to-formal binding; the source producer must also
//! reconstruct the ordered argument group and the actual substitution result.
use std::collections::{BTreeSet, HashMap};

use crate::codebook::{BoundaryReason, SummaryFlowStepKind};
use crate::condition::Atom;
use crate::condition_kernel::{Diagram, KernelBoundary};
use crate::id::{Id, recipe::SummaryFlowProofStep};

crate::query_row! {
    pub struct LocalCallArgument {
        call_fact_id: Id,
        callee_node_id: Id,
        argument_fact_id: Id,
        ordinal: i64,
        formal_node_id: Option<Id>,
        evaluation_fact_id: Option<Id>,
        evaluation_reason: Option<BoundaryReason>,
        boolean_value: Option<bool>,
        source_formal_node_id: Option<Id>,
    }
}

crate::query_row! {
    /// A direct entry-value link, admitted by the source/value-link boundary before use here.
    pub struct DirectValueLink {
        operation_node_id: Id,
        formal_node_id: Id,
        link_id: Id,
        atom: String,
    }
}

pub fn fixed_truth(condition: &Diagram, atom: &str) -> Result<Option<bool>, KernelBoundary> {
    let atom = Atom::parse_encoded(atom).map_err(|_| KernelBoundary::TransferUnsupported)?;
    if !matches!(&atom, Atom::Evaluated { atom, .. } if matches!(**atom, Atom::Truthy { .. })) {
        return Err(KernelBoundary::TransferUnsupported);
    }
    let predicate = Diagram::from_atom(&atom)?;
    if condition.implies(&predicate)? { return Ok(Some(true)); }
    if condition.implies(&predicate.not()?)? { return Ok(Some(false)); }
    Ok(None)
}

/// Validate the complete contiguous control group immediately before a conditional callee
/// reference. The same rule serves source composition and immutable native admission.
pub fn validate_fixed_control_proof(
    caller: Id, caller_condition: &Diagram, callee: Id, callee_condition: &Diagram,
    steps: &[SummaryFlowProofStep], links: &HashMap<Id, DirectValueLink>,
) -> Result<(), &'static str> {
    if steps.is_empty() {
        return Err("conditional callee lacks a test link");
    }
    let mut atoms = BTreeSet::new();
    let mut pending_caller = false;
    for step in steps {
        match step.kind {
            SummaryFlowStepKind::CallerConditionLink => {
                let supported = !pending_caller && step.condition_id == caller_condition.id()
                    && links.get(&step.evidence_id).is_some_and(|link|
                        link.operation_node_id == caller
                        && caller_condition.support().contains(&link.atom)
                        && matches!(fixed_truth(caller_condition, &link.atom), Ok(Some(_))));
                if !supported { return Err("caller condition link does not fix its direct formal"); }
                pending_caller = true;
            },
            SummaryFlowStepKind::CalleeConditionLink => {
                let Some(link) = links.get(&step.evidence_id) else {
                    return Err("conditional callee lacks a cited exact test link");
                };
                if link.operation_node_id != callee || step.condition_id != callee_condition.id()
                    || !callee_condition.support().contains(&link.atom)
                    || !matches!(Atom::parse_encoded(&link.atom), Ok(Atom::Evaluated { atom, .. })
                        if matches!(*atom, Atom::Truthy { .. }))
                    || !atoms.insert(link.atom.as_str()) {
                    return Err("conditional callee lacks a cited exact test link");
                }
                pending_caller = false;
            },
            _ => return Err("unexpected step in conditional callee control proof"),
        }
    }
    if pending_caller { return Err("caller condition link does not fix its direct formal"); }
    if atoms.len() != callee_condition.support().len() {
        return Err("conditional callee control proof has incomplete atom coverage");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EvaluationIdentity;

    #[test]
    fn every_callee_atom_needs_its_own_scoped_link() {
        let caller = Id([1; 16]);
        let callee = Id([2; 16]);
        let atoms: Vec<_> = ["a", "b"].into_iter().enumerate().map(|(index, name)|
            Atom::Truthy { place: name.to_owned() }.evaluated(EvaluationIdentity::Synthetic {
                module: "00".repeat(16), predicate: format!("{:02x}", index + 1).repeat(16),
            })).collect();
        let condition = Diagram::from_atom(&atoms[0]).unwrap()
            .and(&Diagram::from_atom(&atoms[1]).unwrap()).unwrap();
        let links: HashMap<_, _> = atoms.iter().enumerate().map(|(index, atom)| {
            let id = Id([index as u8 + 10; 16]);
            (id, DirectValueLink { operation_node_id: callee, formal_node_id: id,
                link_id: id, atom: atom.encode() })
        }).collect();
        let steps: Vec<_> = (10..12).map(|byte| SummaryFlowProofStep {
            kind: SummaryFlowStepKind::CalleeConditionLink, evidence_id: Id([byte; 16]),
            condition_id: condition.id(),
        }).collect();
        assert!(validate_fixed_control_proof(caller, &Diagram::always(), callee,
            &condition, &steps, &links).is_ok());
        assert!(validate_fixed_control_proof(caller, &Diagram::always(), callee,
            &condition, &steps[1..], &links).is_err());
        let mut wrong_scope = links.clone();
        wrong_scope.get_mut(&Id([10; 16])).unwrap().operation_node_id = caller;
        assert!(validate_fixed_control_proof(caller, &Diagram::always(), callee,
            &condition, &steps, &wrong_scope).is_err());
        let duplicate = vec![steps[0].clone(), steps[0].clone()];
        assert!(validate_fixed_control_proof(caller, &Diagram::always(), callee,
            &condition, &duplicate, &links).is_err());
    }
}
