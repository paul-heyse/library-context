//! Source body outcomes under entry, independently of value transfer or call occurrence.
//! Closed local releases still require the exact callable object to remain externally held.
//! This relation alone never establishes caller continuation.
use crate::codebook::{
    BoundaryReason, Codebook, CompletionKind, ReleaseSafety, SummaryFlowStepKind,
};
use crate::id::{Digest, Id, IdHasher, recipe::SummaryFlowProofStep};
use crate::summary_contract::{ExactRuntimeException, ProofAdmissionError};
use crate::table::table;

table!(SourceBodyCompletions,SourceBodyCompletionsRow="source_body_completions",
    family=Findings,key=[snapshot_id,body_id],
    checks=[("positive_work","work > 0"),("retainer_required","function_retainer_required"),
        ("body_status","(kind = 5 AND reason IS NOT NULL) OR (kind <> 5 AND reason IS NULL)"),
        ("bounded_proof","step_count >= 0 AND step_count <= 64 AND release_count >= 0 AND release_count <= 64")],
    {
        snapshot_id:Id,body_id:Id,function_node_id:Id,declaration_fact_id:Id,syntax_fact_id:Id,
        kind:CompletionKind,terminal_fact_id:Option<Id>,exception:Option<ExactRuntimeException>,
        reason:Option<BoundaryReason>,release_reason:Option<BoundaryReason>,
        /// Mandatory unresolved obligation until an exact call proves its external retainer.
        function_retainer_required:bool,
        runtime_statement_count:i64,step_count:i64,steps_digest:Digest,release_count:i64,releases_digest:Digest,work:i64,
    }
);
table!(SourceBodySteps,SourceBodyStepsRow="source_body_steps",
    family=Findings,key=[snapshot_id,body_id,ordinal],
    checks=[("ordinal_nonnegative","ordinal >= 0")],
    {snapshot_id:Id,body_id:Id,ordinal:i64,kind:SummaryFlowStepKind,evidence_id:Id}
);
table!(SourceBodyReleaseInputs,SourceBodyReleaseInputsRow="source_body_release_inputs",
    family=Findings,key=[snapshot_id,body_id,ordinal],
    checks=[("ordinal_nonnegative","ordinal >= 0")],
    {snapshot_id:Id,body_id:Id,ordinal:i64,syntax_fact_id:Id,safety:ReleaseSafety,
        proof_offset:i64,proof_count:i64,proof_digest:Digest,evaluation_evidence_id:Id}
);

pub fn releases_digest(rows: &[SourceBodyReleaseInputsRow]) -> Digest {
    let mut h = IdHasher::new("source-body-releases");
    h.i64(rows.len() as i64);
    for r in rows {
        h.i64(r.ordinal)
            .id(r.syntax_fact_id)
            .i64(i64::from(r.safety.code()))
            .i64(r.proof_offset)
            .i64(r.proof_count)
            .bytes(&r.proof_digest.0)
            .id(r.evaluation_evidence_id);
    }
    h.finish_digest()
}
pub fn steps_digest(rows: &[SourceBodyStepsRow]) -> Digest {
    crate::completion_proof::proof_digest(
        &rows
            .iter()
            .map(|r| SummaryFlowProofStep {
                kind: r.kind,
                evidence_id: r.evidence_id,
                condition_id: Id::ZERO,
            })
            .collect::<Vec<_>>(),
    )
}
pub fn identity(r: &SourceBodyCompletionsRow) -> Id {
    IdHasher::new("source-body-completion")
        .id(r.function_node_id)
        .id(r.declaration_fact_id)
        .id(r.syntax_fact_id)
        .i64(i64::from(r.kind.code()))
        .opt_id(r.terminal_fact_id)
        .i64(r.exception.map_or(-1, |e| i64::from(e.code())))
        .i64(r.reason.map_or(-1, |e| i64::from(e.code())))
        .i64(r.release_reason.map_or(-1, |e| i64::from(e.code())))
        .i64(i64::from(r.function_retainer_required))
        .i64(r.runtime_statement_count)
        .i64(r.step_count)
        .bytes(&r.steps_digest.0)
        .i64(r.release_count)
        .bytes(&r.releases_digest.0)
        .finish_id()
}

/// Structural commitment only. Publication additionally replays the source suite and each
/// actual expression evaluation. A call consumer must discharge function_retainer_required.
pub fn admit(
    r: &SourceBodyCompletionsRow,
    steps: &[SourceBodyStepsRow],
    releases: &[SourceBodyReleaseInputsRow],
) -> Result<(), ProofAdmissionError> {
    use CompletionKind as C;
    use SummaryFlowStepKind as K;
    if r.body_id != identity(r)
        || !r.function_retainer_required
        || r.work <= 0
        || r.runtime_statement_count < 0
        || !(0..=64).contains(&r.step_count)
        || !(0..=64).contains(&r.release_count)
        || r.step_count as usize != steps.len()
        || r.release_count as usize != releases.len()
        || steps_digest(steps) != r.steps_digest
        || releases_digest(releases) != r.releases_digest
        || steps.iter().enumerate().any(|(i, s)| {
            s.snapshot_id != r.snapshot_id || s.body_id != r.body_id || s.ordinal != i as i64
        })
        || releases.iter().enumerate().any(|(i, s)| {
            s.snapshot_id != r.snapshot_id || s.body_id != r.body_id || s.ordinal != i as i64
        })
    {
        return Err("source body omitted or changed its outcome/release obligations".into());
    }
    match (r.kind, r.terminal_fact_id, r.exception, r.reason) {
        (C::Normal, None, None, None)
        | (C::Return, Some(_), None, None)
        | (C::Raise, Some(_), Some(_), None) => {}
        (C::Unknown, None, None, Some(_))
            if steps.is_empty() && releases.is_empty() && r.release_reason.is_some() => {}
        _ => return Err("source body has an invalid outcome".into()),
    }
    if r.kind != C::Unknown {
        if (steps.is_empty()
            && (r.runtime_statement_count != 0 || r.kind != C::Normal || !releases.is_empty()))
            || (r.release_reason.is_none()
                && releases.iter().any(|s| s.safety != ReleaseSafety::Closed))
        {
            return Err("source body lacks closed release evidence".into());
        }
        if r.terminal_fact_id.is_some_and(|id| {
            steps
                .iter()
                .filter(|s| s.kind == K::CompletionStatement && s.evidence_id == id)
                .count()
                != 1
        }) {
            return Err("source body terminal is not anchored in its suite proof".into());
        }
    }
    let mut covered = [false; crate::summary_contract::MAX_SUMMARY_PROOF_STEPS];
    let mut end = 0;
    for release in releases {
        if release.proof_offset < end as i64
            || release.proof_count <= 0
            || release.proof_offset < 0
            || release.proof_offset as usize > steps.len()
            || release.proof_count as usize > steps.len() - release.proof_offset as usize
        {
            return Err("source body release segment is missing or out of order".into());
        }
        let start = release.proof_offset as usize;
        end = start + release.proof_count as usize;
        let segment = &steps[start..end];
        let root = &segment[segment.len() - 1];
        if segment.iter().any(|s| {
            matches!(
                s.kind,
                K::CompletionStatement | K::LocalAssignmentBinding | K::FinalizerPass
            )
        }) || steps_digest(segment) != release.proof_digest
            || root.evidence_id != release.evaluation_evidence_id
            || !((root.kind == K::ExpressionSyntax && root.evidence_id == release.syntax_fact_id)
                || (root.kind == K::ArgumentEvaluation && release.safety != ReleaseSafety::Closed))
        {
            return Err("source body release lacks its exact expression root".into());
        }
        covered[start..end].fill(true);
    }
    if steps.iter().enumerate().any(|(i, s)| {
        !covered[i]
            && !matches!(
                s.kind,
                K::CompletionStatement | K::LocalAssignmentBinding | K::FinalizerPass
            )
    }) {
        return Err("source body omitted an evaluated release input".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callable_retention_and_body_outcome_are_independent_required_obligations() {
        let mut steps = vec![SourceBodyStepsRow {
            snapshot_id: Id::ZERO,
            body_id: Id::ZERO,
            ordinal: 0,
            kind: SummaryFlowStepKind::FinalizerPass,
            evidence_id: Id([1; 16]),
        }];
        let mut body = SourceBodyCompletionsRow {
            snapshot_id: Id::ZERO,
            body_id: Id::ZERO,
            function_node_id: Id([2; 16]),
            declaration_fact_id: Id([3; 16]),
            syntax_fact_id: Id([4; 16]),
            kind: CompletionKind::Normal,
            terminal_fact_id: None,
            exception: None,
            reason: None,
            release_reason: None,
            function_retainer_required: true,
            runtime_statement_count: 1,
            step_count: 1,
            steps_digest: steps_digest(&steps),
            release_count: 0,
            releases_digest: releases_digest(&[]),
            work: 1,
        };
        let seal = |row: &mut SourceBodyCompletionsRow, steps: &mut [SourceBodyStepsRow]| {
            row.body_id = identity(row);
            for s in steps {
                s.body_id = row.body_id;
            }
        };
        seal(&mut body, &mut steps);
        assert!(admit(&body, &steps, &[]).is_ok());
        // Resealing an unqualified caller-continuation claim still violates the type contract.
        let mut unqualified = body.clone();
        unqualified.function_retainer_required = false;
        seal(&mut unqualified, &mut steps);
        assert!(admit(&unqualified, &steps, &[]).is_err());
        seal(&mut body, &mut steps);
        assert!(admit(&body, &[], &[]).is_err());
        let mut oversized = body.clone();
        oversized.release_count = i64::MAX;
        seal(&mut oversized, &mut steps);
        assert!(admit(&oversized, &steps, &[]).is_err());
        let mut false_return = body.clone();
        false_return.kind = CompletionKind::Return;
        seal(&mut false_return, &mut steps);
        assert!(admit(&false_return, &steps, &[]).is_err());
    }
    #[test]
    fn terminal_and_ordered_release_roots_survive_resealed_deletions() {
        use SummaryFlowStepKind as K;
        let mut steps = vec![
            (K::ExpressionSyntax, 5),
            (K::CompletionStatement, 4),
            (K::ExpressionSyntax, 7),
            (K::CompletionStatement, 6),
        ]
        .into_iter()
        .enumerate()
        .map(|(ordinal, (kind, id))| SourceBodyStepsRow {
            snapshot_id: Id::ZERO,
            body_id: Id::ZERO,
            ordinal: ordinal as i64,
            kind,
            evidence_id: Id([id; 16]),
        })
        .collect::<Vec<_>>();
        let mut releases = vec![
            SourceBodyReleaseInputsRow {
                snapshot_id: Id::ZERO,
                body_id: Id::ZERO,
                ordinal: 0,
                syntax_fact_id: Id([5; 16]),
                safety: ReleaseSafety::Closed,
                proof_offset: 0,
                proof_count: 1,
                proof_digest: steps_digest(&steps[..1]),
                evaluation_evidence_id: Id([5; 16]),
            },
            SourceBodyReleaseInputsRow {
                snapshot_id: Id::ZERO,
                body_id: Id::ZERO,
                ordinal: 1,
                syntax_fact_id: Id([7; 16]),
                safety: ReleaseSafety::Closed,
                proof_offset: 2,
                proof_count: 1,
                proof_digest: steps_digest(&steps[2..3]),
                evaluation_evidence_id: Id([7; 16]),
            },
        ];
        let mut body = SourceBodyCompletionsRow {
            snapshot_id: Id::ZERO,
            body_id: Id::ZERO,
            function_node_id: Id([2; 16]),
            declaration_fact_id: Id([3; 16]),
            syntax_fact_id: Id([4; 16]),
            kind: CompletionKind::Return,
            terminal_fact_id: Some(Id([6; 16])),
            exception: None,
            reason: None,
            release_reason: None,
            function_retainer_required: true,
            runtime_statement_count: 2,
            step_count: 4,
            steps_digest: steps_digest(&steps),
            release_count: 2,
            releases_digest: releases_digest(&releases),
            work: 4,
        };
        let seal = |row: &mut SourceBodyCompletionsRow,
                    steps: &mut [SourceBodyStepsRow],
                    releases: &mut [SourceBodyReleaseInputsRow]| {
            row.release_count = releases.len() as i64;
            row.releases_digest = releases_digest(releases);
            row.body_id = identity(row);
            for s in steps {
                s.body_id = row.body_id;
            }
            for (i, r) in releases.iter_mut().enumerate() {
                r.body_id = row.body_id;
                r.ordinal = i as i64;
            }
        };
        seal(&mut body, &mut steps, &mut releases);
        assert!(admit(&body, &steps, &releases).is_ok());
        let mut bad = body.clone();
        bad.terminal_fact_id = Some(Id([9; 16]));
        seal(&mut bad, &mut steps, &mut releases);
        assert!(admit(&bad, &steps, &releases).is_err());
        let mut foreign = releases.clone();
        foreign[1].syntax_fact_id = Id([9; 16]);
        seal(&mut body, &mut steps, &mut foreign);
        assert!(admit(&body, &steps, &foreign).is_err());
        let mut merged = vec![releases[1].clone()];
        merged[0].ordinal = 0;
        merged[0].proof_offset = 0;
        merged[0].proof_count = 3;
        merged[0].proof_digest = steps_digest(&steps[..3]);
        seal(&mut body, &mut steps, &mut merged);
        assert!(admit(&body, &steps, &merged).is_err());
        releases.swap(0, 1);
        for (i, r) in releases.iter_mut().enumerate() {
            r.ordinal = i as i64;
        }
        seal(&mut body, &mut steps, &mut releases);
        assert!(admit(&body, &steps, &releases).is_err());
        seal(&mut body, &mut steps, &mut []);
        assert!(admit(&body, &steps, &[]).is_err());
    }
}
