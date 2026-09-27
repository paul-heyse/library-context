//! Ordered proof that an eager source call is reached and its invocation inputs complete.
//! This never states that the invoked callable returns, performs an action or releases a
//! resource. Actions must separately cite their model, trigger and value identity.
use crate::codebook::{BoundaryReason,InvocationPhase,SummaryFlowStepKind as K};
use crate::id::{Digest,Id,IdHasher,recipe::SummaryFlowProofStep};
use crate::table::table;
use crate::completion_proof::proof_digest;
use crate::summary_contract::{ProofAdmissionError,MAX_SUMMARY_PROOF_STEPS};

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
        model_id:Id,
        condition_id:Id,
        phase:InvocationPhase,
        argument_count:i64,
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

pub fn identity(r:&CallExecutionsRow)->Id {
    use crate::codebook::Codebook;
    IdHasher::new("call-execution").id(r.function_node_id).id(r.call_node_id).id(r.call_fact_id)
        .id(r.syntax_fact_id).id(r.target_node_id).id(r.pysa_fact_id).id(r.model_id).id(r.condition_id)
        .i64(i64::from(r.phase.code())).i64(r.argument_count).i64(r.prefix_count).i64(r.invocation_count)
        .bytes(&r.prefix_digest.0).bytes(&r.invocation_digest.0)
        .i64(r.reason.map_or(-1,|x|i64::from(x.code()))).i64(r.work).finish_id()
}

/// Shared structural admission; publication additionally reconstructs source semantics.
/// A refused entry carries no retained proof. Successful entries commit both independent
/// groups, including empty prefixes, and end at this call's invocation rather than an exit.
pub fn admit(r:&CallExecutionsRow,steps:&[SummaryFlowProofStep])->Result<(),ProofAdmissionError> {
    if r.execution_id!=identity(r) || r.phase!=InvocationPhase::Call || r.argument_count<0
        || r.prefix_count<0 || r.invocation_count<0 || r.work<=0 {
        return Err("invalid call execution identity or scope".into());
    }
    if r.reason.is_some() {
        if !steps.is_empty() || r.prefix_count!=0 || r.invocation_count!=0
            || r.prefix_digest!=proof_digest(&[]) || r.invocation_digest!=proof_digest(&[]) {
            return Err("refused call execution has positive evidence".into());
        }
        return Ok(());
    }
    if r.argument_count>128 || steps.len()>MAX_SUMMARY_PROOF_STEPS || r.prefix_count as usize>steps.len()
        || r.invocation_count as usize!=steps.len()-r.prefix_count as usize
        || steps.iter().any(|s|s.condition_id!=r.condition_id) {
        return Err("call execution proof count or condition mismatch".into());
    }
    let(before,invoke)=steps.split_at(r.prefix_count as usize);
    if proof_digest(before)!=r.prefix_digest || proof_digest(invoke)!=r.invocation_digest || invoke.len()<6 {
        return Err("call execution obligations differ".into());
    }
    let tail=&invoke[invoke.len()-3..];
    if tail.iter().map(|s|(s.kind,s.evidence_id)).collect::<Vec<_>>()
        !=[(K::CallSite,r.call_fact_id),(K::CallTarget,r.pysa_fact_id),(K::ModelInvocation,r.model_id)]
        || steps.iter().filter(|s|s.kind==K::ModelInvocation).count()!=1
        || invoke[..3].iter().map(|s|s.kind).collect::<Vec<_>>()
            !=[K::ModuleImportBinding,K::ModuleImportRegion,K::CalleeResolution]
        || steps.iter().any(|s|matches!(s.kind,K::ReturnExit|K::ContextEntry|K::ContextExit)) {
        return Err("call execution has a foreign invocation or unsupported frontier".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_refused_oversized_invocation_retains_its_actual_source_count() {
        let id=Id([1;16]);
        let mut row=CallExecutionsRow {snapshot_id:id,execution_id:Id::ZERO,function_node_id:id,
            call_node_id:id,call_fact_id:id,syntax_fact_id:id,target_node_id:id,pysa_fact_id:id,model_id:id,
            condition_id:crate::condition_kernel::Diagram::always().id(),phase:InvocationPhase::Call,
            argument_count:130,prefix_count:0,invocation_count:0,prefix_digest:proof_digest(&[]),
            invocation_digest:proof_digest(&[]),reason:Some(BoundaryReason::InvocationArgumentLimit),work:1};
        row.execution_id=identity(&row);
        assert!(admit(&row,&[]).is_ok());
        row.reason=None;row.execution_id=identity(&row);
        assert!(admit(&row,&[]).is_err());
    }
}
