//! Ordered proof that an eager source call is reached and its invocation inputs complete.
//! This never states that the invoked callable returns, performs an action or releases a
//! resource. Actions must separately cite their model, trigger and value identity.
use crate::codebook::{
    BoundaryReason, CallExecutionTarget, InvocationPhase, SummaryFlowStepKind as K,
};
use crate::completion_proof::proof_digest;
use crate::id::{Digest, Id, IdHasher, recipe::SummaryFlowProofStep};
use crate::summary_contract::{MAX_SUMMARY_PROOF_STEPS, ProofAdmissionError};
use crate::table::table;

table!(
    CallExecutions,CallExecutionsRow = "call_executions",
    family = Findings,
    key = [snapshot_id,execution_id],
    checks = [("counts_nonnegative","argument_count >= 0 AND prefix_count >= 0 AND invocation_count >= 0 AND work > 0")],
    {
        snapshot_id:Id,
        execution_id:Id,
        function_node_id:Id,
        call_node_id:Id,
        call_fact_id:Id,
        syntax_fact_id:Id,
        target_node_id:Id,
        pysa_fact_id:Id,
        target_kind:CallExecutionTarget,
        model_id:Option<Id>,
        source_binding_id:Option<Id>,
        condition_id:Id,
        phase:InvocationPhase,
        argument_count:i64,
        /// Derived from all bound signatures independently of retained invocation proof.
        default_formal_count:Option<i64>,
        default_formals_digest:Option<Digest>,
        prefix_count:i64,
        invocation_count:i64,
        prefix_digest:Digest,
        invocation_digest:Digest,
        reason:Option<BoundaryReason>,
        work:i64,
    }
);

table!(
    CallExecutionSteps,CallExecutionStepsRow = "call_execution_steps",
    family = Findings,
    key = [snapshot_id,execution_id,ordinal],
    checks = [("ordinal_nonnegative","ordinal >= 0")],
    {
        snapshot_id:Id,
        execution_id:Id,
        ordinal:i64,
        kind:K,
        evidence_id:Id,
        condition_id:Id,
    }
);

pub fn identity(r: &CallExecutionsRow) -> Id {
    use crate::codebook::Codebook;
    IdHasher::new("call-execution")
        .id(r.function_node_id)
        .id(r.call_node_id)
        .id(r.call_fact_id)
        .id(r.syntax_fact_id)
        .id(r.target_node_id)
        .id(r.pysa_fact_id)
        .i64(i64::from(r.target_kind.code()))
        .opt_id(r.model_id)
        .opt_id(r.source_binding_id)
        .id(r.condition_id)
        .i64(i64::from(r.phase.code()))
        .i64(r.argument_count)
        .opt_i64(r.default_formal_count)
        .opt_digest(r.default_formals_digest)
        .i64(r.prefix_count)
        .i64(r.invocation_count)
        .bytes(&r.prefix_digest.0)
        .bytes(&r.invocation_digest.0)
        .i64(r.reason.map_or(-1, |x| i64::from(x.code())))
        .i64(r.work)
        .finish_id()
}

/// Canonical binding commitment; signature-specific fact IDs are preserved in binder order.
pub fn defaults_digest(formals: &[Id]) -> Digest {
    let mut h = IdHasher::new("pinned-call-default-formals");
    h.i64(formals.len() as i64);
    for &formal in formals {
        h.id(formal);
    }
    h.finish_digest()
}

/// Shared structural admission; publication additionally reconstructs source semantics.
/// A refused entry carries no retained proof. Successful entries commit both independent
/// groups, including empty prefixes, and end at this call's invocation rather than an exit.
pub fn admit(
    r: &CallExecutionsRow,
    steps: &[SummaryFlowProofStep],
) -> Result<(), ProofAdmissionError> {
    if r.execution_id != identity(r)
        || r.phase != InvocationPhase::Call
        || r.argument_count < 0
        || r.prefix_count < 0
        || r.invocation_count < 0
        || r.work <= 0
        || r.default_formal_count.is_some() != r.default_formals_digest.is_some()
        || r.default_formal_count.is_some_and(|n| n < 0)
    {
        return Err("invalid call execution identity or scope".into());
    }
    let conclusion = match r.target_kind {
        CallExecutionTarget::Model if r.model_id.is_some() && r.source_binding_id.is_none() => {
            Some((K::ModelInvocation, r.model_id.unwrap()))
        }
        CallExecutionTarget::Source
            if r.model_id.is_none() && (r.source_binding_id.is_some() || r.reason.is_some()) =>
        {
            r.source_binding_id.map(|id| (K::SourceInvocation, id))
        }
        _ => return Err("mixed or missing call execution target support".into()),
    };
    if r.reason.is_some() {
        if !steps.is_empty()
            || r.prefix_count != 0
            || r.invocation_count != 0
            || r.prefix_digest != proof_digest(&[])
            || r.invocation_digest != proof_digest(&[])
        {
            return Err("refused call execution has positive evidence".into());
        }
        return Ok(());
    }
    if r.argument_count > 128
        || steps.len() > MAX_SUMMARY_PROOF_STEPS
        || r.prefix_count as usize > steps.len()
        || r.invocation_count as usize != steps.len() - r.prefix_count as usize
        || steps.iter().any(|s| s.condition_id != r.condition_id)
    {
        return Err("call execution proof count or condition mismatch".into());
    }
    let (before, invoke) = steps.split_at(r.prefix_count as usize);
    if proof_digest(before) != r.prefix_digest
        || proof_digest(invoke) != r.invocation_digest
        || invoke.len() < 3
    {
        return Err("call execution obligations differ".into());
    }
    let tail = &invoke[invoke.len() - 3..];
    if tail
        .iter()
        .map(|s| (s.kind, s.evidence_id))
        .collect::<Vec<_>>()
        != [
            (K::CallSite, r.call_fact_id),
            (K::CallTarget, r.pysa_fact_id),
            conclusion.ok_or("missing invocation target")?,
        ]
        || steps
            .iter()
            .filter(|s| matches!(s.kind, K::ModelInvocation | K::SourceInvocation))
            .count()
            != 1
        || steps
            .iter()
            .any(|s| matches!(s.kind, K::ReturnExit | K::ContextEntry | K::ContextExit))
    {
        return Err("call execution has a foreign invocation or unsupported frontier".into());
    }
    if r.target_kind == CallExecutionTarget::Source {
        if invoke.len() != 3
            || r.argument_count != 0
            || r.default_formal_count != Some(0)
            || r.default_formals_digest != Some(defaults_digest(&[]))
        {
            return Err(
                "source invocation exceeds the admitted zero-argument binding domain".into(),
            );
        }
        return Ok(());
    }
    if invoke.len() < 6
        || invoke[..3].iter().map(|s| s.kind).collect::<Vec<_>>()
            != [
                K::ModuleImportBinding,
                K::ModuleImportRegion,
                K::CalleeResolution,
            ]
    {
        return Err("modeled invocation lacks its exact callee loading evidence".into());
    }
    let count = r
        .default_formal_count
        .ok_or("call lacks independently bound default obligations")? as usize;
    if count > invoke.len().saturating_sub(7) {
        return Err("call default obligations exceed its bounded invocation proof".into());
    }
    // This occurrence owns exactly one root group. Nested default groups need their own
    // independent commitments before this flattened invocation contract can admit them.
    let markers: Vec<_> = invoke
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.kind, K::ModelDefaultsAvailable | K::ModelDefaultFormal))
        .map(|(i, _)| i)
        .collect();
    let expected: Vec<_> = if count == 0 {
        Vec::new()
    } else {
        (3..4 + count).collect()
    };
    if markers != expected {
        return Err("foreign or duplicate default availability group".into());
    }
    if count == 0 {
        if r.default_formals_digest != Some(defaults_digest(&[]))
            || matches!(
                invoke[3].kind,
                K::ModelDefaultsAvailable | K::ModelDefaultFormal
            )
        {
            return Err("call has foreign default availability evidence".into());
        }
    } else {
        if invoke[3].kind != K::ModelDefaultsAvailable || Some(invoke[3].evidence_id) != r.model_id
        {
            return Err("call lacks its required default availability group".into());
        }
        let formals = &invoke[4..4 + count];
        let ids: Vec<_> = formals.iter().map(|s| s.evidence_id).collect();
        if formals.iter().any(|s| s.kind != K::ModelDefaultFormal)
            || ids
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != count
            || r.default_formals_digest != Some(defaults_digest(&ids))
            || matches!(
                invoke[4 + count].kind,
                K::ModelDefaultsAvailable | K::ModelDefaultFormal
            )
        {
            return Err("call default evidence differs from its bound obligations".into());
        }
    }
    Ok(())
}

/// Complete every predecessor and evaluated operand, while leaving this root's invoked body
/// unresolved. This is shared by the producer and action consumers, including compact source
/// call references whose expanded support still counts against the original cap.
pub fn admit_with_calls<'a>(
    r: &CallExecutionsRow,
    steps: &[SummaryFlowProofStep],
    frames: impl Fn(Id) -> Option<&'a crate::frame_exit::ModelFrameExitsRow>,
    source: impl Fn(Id) -> Option<crate::source_call::NormalSupport<'a>>,
    bindings: impl Fn(Id) -> Option<&'a crate::source_call::SourceCallBindingsRow>,
) -> Result<(), ProofAdmissionError> {
    admit(r, steps)?;
    if r.reason.is_some() {
        return Ok(());
    }
    let mut count =
        crate::source_call::expanded_len(steps.iter().map(|s| (s.kind, s.evidence_id)), &source)?;
    if r.target_kind == CallExecutionTarget::Source {
        let binding = bindings(
            r.source_binding_id
                .ok_or("source invocation lacks binding")?,
        )
        .ok_or("source invocation binding is unavailable")?;
        if binding.binding_id != crate::source_call::binding_identity(binding)
            || binding.snapshot_id != r.snapshot_id
            || binding.function_node_id != r.function_node_id
            || binding.call_node_id != r.call_node_id
            || binding.call_fact_id != r.call_fact_id
            || binding.syntax_fact_id != r.syntax_fact_id
            || binding.callee_node_id != r.target_node_id
            || binding.pysa_fact_id != r.pysa_fact_id
            || !(1..=64).contains(&binding.header_count)
        {
            return Err("source invocation has a foreign binding".into());
        }
        count += crate::source_call::binding_cost(binding)?;
    }
    if count > MAX_SUMMARY_PROOF_STEPS {
        return Err(ProofAdmissionError {
            reason: BoundaryReason::SummaryProofLimit,
            message: "call execution exceeds its expanded proof limit",
        });
    }
    // admit() checked the exact final root CallSite/CallTarget/invocation triple.
    crate::frame_exit::admit_proof(
        r.function_node_id,
        &steps[..steps.len() - 3],
        frames,
        source,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_invocation_requires_only_its_binding_and_cannot_prove_completion() {
        use crate::source_call::{
            SourceCallBindingsRow, SourceCallHeaderStepsRow, admit_binding, binding_identity,
            header_digest,
        };
        let id = |n| Id([n; 16]);
        let condition = crate::condition_kernel::Diagram::always().id();
        let mut binding = SourceCallBindingsRow {
            snapshot_id: id(1),
            binding_id: Id::ZERO,
            function_node_id: id(2),
            call_node_id: id(3),
            call_fact_id: id(4),
            syntax_fact_id: id(5),
            callee_node_id: id(6),
            pysa_fact_id: id(7),
            signature_fact_id: id(8),
            declaration_fact_id: id(9),
            header_fact_id: id(10),
            statement_fact_id: id(11),
            binding_fact_id: id(12),
            reference_fact_id: id(13),
            resolution_fact_id: id(14),
            header_count: 3,
            header_digest: Digest::ZERO,
        };
        let mut header: Vec<_> = [
            (K::DefinitionHeaderEvidence, id(9)),
            (K::DefinitionHeaderEvidence, id(12)),
            (K::CompletionStatement, id(10)),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (kind, evidence_id))| SourceCallHeaderStepsRow {
            snapshot_id: id(1),
            binding_id: Id::ZERO,
            ordinal: i as i64,
            kind,
            evidence_id,
        })
        .collect();
        binding.header_digest = header_digest(&header);
        binding.binding_id = binding_identity(&binding);
        for s in &mut header {
            s.binding_id = binding.binding_id;
        }
        assert!(admit_binding(&binding, &header).is_ok());
        let proof: Vec<_> = [
            (K::CallSite, id(4)),
            (K::CallTarget, id(7)),
            (K::SourceInvocation, binding.binding_id),
        ]
        .into_iter()
        .map(|(kind, evidence_id)| SummaryFlowProofStep {
            kind,
            evidence_id,
            condition_id: condition,
        })
        .collect();
        let mut row = CallExecutionsRow {
            snapshot_id: id(1),
            execution_id: Id::ZERO,
            function_node_id: id(2),
            call_node_id: id(3),
            call_fact_id: id(4),
            syntax_fact_id: id(5),
            target_node_id: id(6),
            pysa_fact_id: id(7),
            target_kind: CallExecutionTarget::Source,
            model_id: None,
            source_binding_id: Some(binding.binding_id),
            condition_id: condition,
            phase: InvocationPhase::Call,
            argument_count: 0,
            default_formal_count: Some(0),
            default_formals_digest: Some(defaults_digest(&[])),
            prefix_count: 0,
            invocation_count: 3,
            prefix_digest: proof_digest(&[]),
            invocation_digest: proof_digest(&proof),
            reason: None,
            work: 1,
        };
        row.execution_id = identity(&row);
        assert!(admit_with_calls(&row, &proof, |_| None, |_| None, |_| Some(&binding)).is_ok());
        assert!(admit_with_calls(&row, &proof, |_| None, |_| None, |_| None).is_err());
        assert!(crate::frame_exit::admit_proof(id(2), &proof, |_| None, |_| None).is_err());
        assert!(crate::frame_exit::admit_proof(id(2), &proof[2..], |_| None, |_| None).is_err());
        for mutation in 0..4 {
            let mut changed = row.clone();
            match mutation {
                0 => changed.model_id = Some(id(30)),
                1 => changed.target_kind = CallExecutionTarget::Model,
                2 => changed.source_binding_id = None,
                _ => changed.function_node_id = id(30),
            }
            changed.execution_id = identity(&changed);
            assert!(
                admit_with_calls(&changed, &proof, |_| None, |_| None, |_| Some(&binding)).is_err(),
                "mutation {mutation}"
            );
        }
        let mut foreign = binding.clone();
        foreign.function_node_id = id(30);
        foreign.binding_id = binding_identity(&foreign);
        let mut changed = row.clone();
        changed.source_binding_id = Some(foreign.binding_id);
        let mut substituted = proof.clone();
        substituted[2].evidence_id = foreign.binding_id;
        changed.invocation_digest = proof_digest(&substituted);
        changed.execution_id = identity(&changed);
        assert!(
            admit_with_calls(
                &changed,
                &substituted,
                |_| None,
                |_| None,
                |_| Some(&foreign)
            )
            .is_err()
        );
        for (prefix, accepted) in [(52, true), (53, false)] {
            let before = vec![
                SummaryFlowProofStep {
                    kind: K::ExpressionSyntax,
                    evidence_id: id(20),
                    condition_id: condition
                };
                prefix
            ];
            let mut changed = row.clone();
            changed.prefix_count = prefix as i64;
            changed.prefix_digest = proof_digest(&before);
            changed.execution_id = identity(&changed);
            let all: Vec<_> = before.into_iter().chain(proof.clone()).collect();
            assert_eq!(
                admit_with_calls(&changed, &all, |_| None, |_| None, |_| Some(&binding)).is_ok(),
                accepted
            );
        }
        // Failed binding is still an attributed source-target refusal, without fabricated support.
        row.source_binding_id = None;
        row.reason = Some(BoundaryReason::ScopeBoundary);
        row.invocation_count = 0;
        row.invocation_digest = proof_digest(&[]);
        row.execution_id = identity(&row);
        assert!(admit(&row, &[]).is_ok());
    }
    #[test]
    fn a_refused_oversized_invocation_retains_its_actual_source_count() {
        let id = Id([1; 16]);
        let mut row = CallExecutionsRow {
            snapshot_id: id,
            execution_id: Id::ZERO,
            function_node_id: id,
            call_node_id: id,
            call_fact_id: id,
            syntax_fact_id: id,
            target_node_id: id,
            pysa_fact_id: id,
            target_kind: CallExecutionTarget::Model,
            model_id: Some(id),
            source_binding_id: None,
            condition_id: crate::condition_kernel::Diagram::always().id(),
            phase: InvocationPhase::Call,
            argument_count: 130,
            default_formal_count: None,
            default_formals_digest: None,
            prefix_count: 0,
            invocation_count: 0,
            prefix_digest: proof_digest(&[]),
            invocation_digest: proof_digest(&[]),
            reason: Some(BoundaryReason::InvocationArgumentLimit),
            work: 1,
        };
        row.execution_id = identity(&row);
        assert!(admit(&row, &[]).is_ok());
        row.reason = None;
        row.execution_id = identity(&row);
        assert!(admit(&row, &[]).is_err());
    }
    #[test]
    fn default_obligations_survive_omission_of_the_entire_evidence_group() {
        let id = |n| Id([n; 16]);
        let condition = crate::condition_kernel::Diagram::always().id();
        let ids = [id(8), id(9)];
        let proof: Vec<_> = [
            (K::ModuleImportBinding, id(10)),
            (K::ModuleImportRegion, id(11)),
            (K::CalleeResolution, id(12)),
            (K::ModelDefaultsAvailable, id(7)),
            (K::ModelDefaultFormal, ids[0]),
            (K::ModelDefaultFormal, ids[1]),
            (K::CallSite, id(3)),
            (K::CallTarget, id(4)),
            (K::ModelInvocation, id(7)),
        ]
        .into_iter()
        .map(|(kind, evidence_id)| SummaryFlowProofStep {
            kind,
            evidence_id,
            condition_id: condition,
        })
        .collect();
        let mut row = CallExecutionsRow {
            snapshot_id: id(1),
            execution_id: Id::ZERO,
            function_node_id: id(2),
            call_node_id: id(5),
            call_fact_id: id(3),
            syntax_fact_id: id(6),
            target_node_id: id(13),
            pysa_fact_id: id(4),
            target_kind: CallExecutionTarget::Model,
            model_id: Some(id(7)),
            source_binding_id: None,
            condition_id: condition,
            phase: InvocationPhase::Call,
            argument_count: 0,
            default_formal_count: Some(2),
            default_formals_digest: Some(defaults_digest(&ids)),
            prefix_count: 0,
            invocation_count: proof.len() as i64,
            prefix_digest: proof_digest(&[]),
            invocation_digest: proof_digest(&proof),
            reason: None,
            work: 1,
        };
        row.execution_id = identity(&row);
        assert!(admit(&row, &proof).is_ok());
        let mut oversized = row.clone();
        oversized.default_formal_count = Some(i64::MAX);
        oversized.execution_id = identity(&oversized);
        assert!(
            admit(&oversized, &proof).is_err(),
            "forged counts must refuse before allocation"
        );
        for mutation in 0..8 {
            let mut altered = proof.clone();
            let mut row = row.clone();
            match mutation {
                0 => {
                    altered.drain(3..6);
                }
                1 => {
                    altered.remove(4);
                }
                2 => altered.swap(4, 5),
                3 => altered[5].evidence_id = ids[0],
                4 => altered[3].evidence_id = id(90),
                5 => altered[4].evidence_id = id(90),
                6 => {
                    altered.insert(6, altered[3]);
                }
                _ => {
                    altered.insert(6, altered[4]);
                }
            }
            // A self-consistent retained-proof commitment cannot erase the binding obligation.
            row.invocation_count = altered.len() as i64;
            row.invocation_digest = proof_digest(&altered);
            row.execution_id = identity(&row);
            assert!(admit(&row, &altered).is_err(), "mutation {mutation}");
        }
        let mut no_defaults = proof.clone();
        no_defaults.drain(3..6);
        row.default_formal_count = Some(0);
        row.default_formals_digest = Some(defaults_digest(&[]));
        row.invocation_count = no_defaults.len() as i64;
        row.invocation_digest = proof_digest(&no_defaults);
        row.execution_id = identity(&row);
        assert!(admit(&row, &no_defaults).is_ok());
        for kind in [K::ModelDefaultsAvailable, K::ModelDefaultFormal] {
            let mut altered = no_defaults.clone();
            let mut row = row.clone();
            altered.insert(
                3,
                SummaryFlowProofStep {
                    kind,
                    evidence_id: id(8),
                    condition_id: condition,
                },
            );
            row.invocation_count = altered.len() as i64;
            row.invocation_digest = proof_digest(&altered);
            row.execution_id = identity(&row);
            assert!(
                admit(&row, &altered).is_err(),
                "zero-default call accepts foreign group"
            );
        }
    }
}
