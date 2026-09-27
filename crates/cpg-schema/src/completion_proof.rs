//! Source-derived commitments to return entry and frame-exit obligations. The completion
//! evaluator owns semantics. Both source summaries and immutable readers verify these exact
//! ordered obligations, including empty ones, without interpreting Python again.
use crate::behavior::{
    ReturnEntryStatusesRow, ReturnEntryStepsRow, ReturnExitStatusesRow, ReturnExitStepsRow,
};
use crate::codebook::{Codebook, SummaryFlowStepKind as K};
use crate::condition_kernel::Diagram;
use crate::id::{Digest, Id, IdHasher, recipe::SummaryFlowProofStep};
use crate::summary_contract::ProofAdmissionError;
use crate::table::table;

table!(
    ReturnCompletionCertificates, ReturnCompletionCertificatesRow = "return_completion_certificates",
    family = Findings,
    key = [snapshot_id, certificate_id],
    checks = [("counts_nonnegative", "entry_count >= 0 AND exit_count >= 0")],
    {
        snapshot_id: Id,
        certificate_id: Id,
        function_node_id: Id,
        return_site_fact_id: Id,
        entry_condition_id: Id,
        exit_condition_id: Id,
        entry_count: i64,
        exit_count: i64,
        entry_digest: Digest,
        exit_digest: Digest,
    }
);

pub fn proof_digest(steps: &[SummaryFlowProofStep]) -> Digest {
    let mut h = IdHasher::new("completion-obligation");
    h.i64(steps.len() as i64);
    for s in steps {
        h.i64(i64::from(s.kind.code()))
            .id(s.evidence_id)
            .id(s.condition_id);
    }
    h.finish_digest()
}

pub fn identity(row: &ReturnCompletionCertificatesRow) -> Id {
    IdHasher::new("return-completion-certificate")
        .id(row.function_node_id)
        .id(row.return_site_fact_id)
        .id(row.entry_condition_id)
        .id(row.exit_condition_id)
        .i64(row.entry_count)
        .i64(row.exit_count)
        .bytes(&row.entry_digest.0)
        .bytes(&row.exit_digest.0)
        .finish_id()
}

pub fn certify(
    entries: &[ReturnEntryStatusesRow],
    entry_steps: &[ReturnEntryStepsRow],
    exits: &[ReturnExitStatusesRow],
    exit_steps: &[ReturnExitStepsRow],
) -> Vec<ReturnCompletionCertificatesRow> {
    let mut out = Vec::new();
    for entry in entries.iter().filter(|r| r.reason.is_none()) {
        if entries
            .iter()
            .filter(|r| {
                r.snapshot_id == entry.snapshot_id
                    && r.function_node_id == entry.function_node_id
                    && r.return_site_fact_id == entry.return_site_fact_id
                    && r.condition_id == entry.condition_id
            })
            .count()
            != 1
        {
            continue;
        }
        let exits: Vec<_> = exits
            .iter()
            .filter(|r| {
                r.snapshot_id == entry.snapshot_id
                    && r.function_node_id == entry.function_node_id
                    && r.source_fact_id == entry.return_site_fact_id
            })
            .collect();
        let [exit] = exits.as_slice() else {
            continue;
        };
        if exit.reason.is_some() {
            continue;
        }
        let mut before: Vec<_> = entry_steps
            .iter()
            .filter(|s| {
                s.snapshot_id == entry.snapshot_id
                    && s.return_site_fact_id == entry.return_site_fact_id
                    && s.condition_id == entry.condition_id
            })
            .collect();
        let mut after: Vec<_> = exit_steps
            .iter()
            .filter(|s| {
                s.snapshot_id == entry.snapshot_id
                    && s.return_site_fact_id == entry.return_site_fact_id
            })
            .collect();
        before.sort_by_key(|s| s.ordinal);
        after.sort_by_key(|s| s.ordinal);
        if before
            .iter()
            .enumerate()
            .any(|(i, s)| s.ordinal != i as i64)
            || after
                .iter()
                .enumerate()
                .any(|(i, s)| s.ordinal != i as i64 || s.condition_id != exit.condition_id)
        {
            continue;
        }
        let before: Vec<_> = before
            .into_iter()
            .map(|s| SummaryFlowProofStep {
                kind: s.kind,
                evidence_id: s.evidence_id,
                condition_id: s.condition_id,
            })
            .collect();
        let after: Vec<_> = after
            .into_iter()
            .map(|s| SummaryFlowProofStep {
                kind: s.kind,
                evidence_id: s.evidence_id,
                condition_id: s.condition_id,
            })
            .collect();
        let mut row = ReturnCompletionCertificatesRow {
            snapshot_id: entry.snapshot_id,
            certificate_id: Id::ZERO,
            function_node_id: entry.function_node_id,
            return_site_fact_id: entry.return_site_fact_id,
            entry_condition_id: entry.condition_id,
            exit_condition_id: exit.condition_id,
            entry_count: before.len() as i64,
            exit_count: after.len() as i64,
            entry_digest: proof_digest(&before),
            exit_digest: proof_digest(&after),
        };
        row.certificate_id = identity(&row);
        out.push(row);
    }
    out.sort_by_key(|r| (r.snapshot_id, r.certificate_id));
    out
}

/// The source entry condition may be specialized to a stronger summary condition; the
/// finalizer proof keeps its original scope. Neither scope mapping is inferred from labels.
pub fn admit(
    row: &ReturnCompletionCertificatesRow,
    function: Id,
    return_site: Id,
    condition: &Diagram,
    entry_premise: &Diagram,
    exit_premise: &Diagram,
    steps: &[SummaryFlowProofStep],
) -> Result<(), ProofAdmissionError> {
    if row.certificate_id != identity(row)
        || row.function_node_id != function
        || row.return_site_fact_id != return_site
        || entry_premise.id() != row.entry_condition_id
        || exit_premise.id() != row.exit_condition_id
        || !condition.implies(entry_premise)?
        || !condition.implies(exit_premise)?
    {
        return Err("return completion certificate has a foreign scope".into());
    }
    let before = usize::try_from(row.entry_count).map_err(|_| "invalid completion entry length")?;
    let after = usize::try_from(row.exit_count).map_err(|_| "invalid completion exit length")?;
    if before > steps.len() || after > steps.len() {
        return Err("missing completion obligations".into());
    }
    let anchors: Vec<_> = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == K::ReturnExit)
        .collect();
    let [(anchor, step)] = anchors.as_slice() else {
        return Err("return completion requires one exit anchor".into());
    };
    if step.evidence_id != return_site
        || step.condition_id != row.exit_condition_id
        || *anchor < before.saturating_add(after)
    {
        return Err("return completion has an invalid exit anchor".into());
    }
    let mut entry = steps[..before].to_vec();
    if entry.iter().any(|s| s.condition_id != condition.id()) {
        return Err("completion entry condition was changed".into());
    }
    for s in &mut entry {
        s.condition_id = row.entry_condition_id;
    }
    let exit = &steps[anchor - after..*anchor];
    if proof_digest(&entry) != row.entry_digest || proof_digest(exit) != row.exit_digest {
        return Err("return completion obligations were changed or omitted".into());
    }
    for (i, s) in steps.iter().enumerate() {
        if matches!(
            s.kind,
            K::ContextConstruction | K::ContextEntry | K::ContextExit | K::ContextExceptionEvidence
        ) && !(i < before || (i >= anchor - after && i < *anchor))
        {
            return Err("orphan context lifecycle evidence".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::Atom;
    fn id(n: u8) -> Id {
        Id([n; 16])
    }
    #[test]
    fn obligations_bind_all_ordered_evidence_and_explicit_condition_specialization() {
        let premise = Diagram::always();
        let selected = Diagram::from_atom(&Atom::Truthy {
            place: "Parameter[flag]".into(),
        })
        .unwrap();
        let step = |kind, evidence, condition| SummaryFlowProofStep {
            kind,
            evidence_id: id(evidence),
            condition_id: condition,
        };
        let entry = vec![
            step(K::ContextConstruction, 4, premise.id()),
            step(K::ContextEntry, 4, premise.id()),
            step(K::ContextExceptionEvidence, 5, premise.id()),
            step(K::ContextExit, 4, premise.id()),
        ];
        let after = vec![
            step(K::ContextConstruction, 6, premise.id()),
            step(K::ContextEntry, 6, premise.id()),
            step(K::ContextExit, 6, premise.id()),
        ];
        let mut certificate = ReturnCompletionCertificatesRow {
            snapshot_id: id(1),
            certificate_id: Id::ZERO,
            function_node_id: id(2),
            return_site_fact_id: id(3),
            entry_condition_id: premise.id(),
            exit_condition_id: premise.id(),
            entry_count: entry.len() as i64,
            exit_count: after.len() as i64,
            entry_digest: proof_digest(&entry),
            exit_digest: proof_digest(&after),
        };
        certificate.certificate_id = identity(&certificate);
        let mut proof: Vec<_> = entry
            .iter()
            .cloned()
            .map(|mut s| {
                s.condition_id = selected.id();
                s
            })
            .collect();
        proof.push(step(K::RawIdentity, 7, selected.id()));
        proof.extend(after);
        proof.push(step(K::ReturnExit, 3, premise.id()));
        let check = |proof: &[_]| {
            admit(
                &certificate,
                id(2),
                id(3),
                &selected,
                &premise,
                &premise,
                proof,
            )
        };
        assert!(check(&proof).is_ok());
        let mut missing = proof.clone();
        missing.remove(2);
        assert!(check(&missing).is_err());
        let mut swapped = proof.clone();
        swapped.swap(0, 1);
        assert!(check(&swapped).is_err());
        let mut foreign = proof.clone();
        foreign[1].evidence_id = id(8);
        assert!(check(&foreign).is_err());
        let mut condition = proof.clone();
        condition[1].condition_id = premise.id();
        assert!(check(&condition).is_err());
        let without_group = proof[entry.len()..].to_vec();
        assert!(check(&without_group).is_err());
        assert!(
            admit(
                &certificate,
                id(8),
                id(3),
                &selected,
                &premise,
                &premise,
                &proof
            )
            .is_err()
        );
        let mut wrong = certificate.clone();
        wrong.return_site_fact_id = id(8);
        wrong.certificate_id = identity(&wrong);
        assert!(admit(&wrong, id(2), id(3), &selected, &premise, &premise, &proof).is_err());
    }
    #[test]
    fn implication_budget_refusal_keeps_its_typed_reason() {
        let premise = Diagram::from_atom(&Atom::Truthy {
            place: "entry".into(),
        })
        .unwrap();
        let mut condition = Diagram::always();
        for n in 0..128 {
            condition = condition
                .and(
                    &Diagram::from_atom(&Atom::Truthy {
                        place: format!("selected{n}"),
                    })
                    .unwrap(),
                )
                .unwrap();
        }
        let exit = Diagram::always();
        let mut certificate = ReturnCompletionCertificatesRow {
            snapshot_id: id(1),
            certificate_id: Id::ZERO,
            function_node_id: id(2),
            return_site_fact_id: id(3),
            entry_condition_id: premise.id(),
            exit_condition_id: exit.id(),
            entry_count: 0,
            exit_count: 0,
            entry_digest: proof_digest(&[]),
            exit_digest: proof_digest(&[]),
        };
        certificate.certificate_id = identity(&certificate);
        let error =
            admit(&certificate, id(2), id(3), &condition, &premise, &exit, &[]).unwrap_err();
        assert_eq!(
            error.reason,
            crate::codebook::BoundaryReason::ConditionAtomLimit
        );
    }
}
