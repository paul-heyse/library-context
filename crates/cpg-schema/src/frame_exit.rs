//! Release-domain certificates under entry to one exact pinned direct-return body.
//! Invocation, body completion and frame release are separate obligations.
use crate::codebook::ReleaseSafety;
use crate::id::{Digest,Id,IdHasher};
use crate::table::table;
use crate::summary_contract::ProofAdmissionError;

table!(ModelFrameExits,ModelFrameExitsRow = "model_frame_exits",
    family = Findings, key = [snapshot_id,frame_exit_id],
    checks = [("bounded_arguments","argument_count >= 0 AND argument_count <= 128")],
    {
        snapshot_id:Id,frame_exit_id:Id,function_node_id:Id,call_node_id:Id,
        call_fact_id:Id,syntax_fact_id:Id,target_node_id:Id,pysa_fact_id:Id,model_id:Id,
        return_parameter:String,return_argument_fact_id:Id,
        argument_count:i64,signature_count:i64,arguments_digest:Digest,
        invocation_count:i64,invocation_digest:Digest,
    }
);
table!(ModelFrameExitArguments,ModelFrameExitArgumentsRow = "model_frame_exit_arguments",
    family = Findings,key = [snapshot_id,frame_exit_id,ordinal],
    checks = [("ordinal_nonnegative","ordinal >= 0"),("qualified","safety <> 2")],
    {
        snapshot_id:Id,frame_exit_id:Id,ordinal:i64,argument_fact_id:Id,
        expression_fact_id:Id,safety:ReleaseSafety,
        parameter_name:String,expression_offset:i64,expression_count:i64,expression_digest:Digest,
        parameters_digest:Digest,
    }
);
table!(ModelFrameExitSteps,ModelFrameExitStepsRow = "model_frame_exit_steps",
    family = Findings,key = [snapshot_id,frame_exit_id,ordinal],
    checks = [("ordinal_nonnegative","ordinal >= 0")],
    { snapshot_id:Id,frame_exit_id:Id,ordinal:i64,operand_fact_id:Id,evidence_id:Id,
      status:crate::codebook::ModeledArgumentEvaluationStatus,kind:crate::codebook::SummaryFlowStepKind }
);
pub fn steps_digest(rows:&[crate::behavior::ExpressionEvaluationStepsRow])->Digest {crate::action::normal_digest(rows)}
pub fn parameters_digest(ids:&[Id])->Digest {crate::call_execution::defaults_digest(ids)}
pub fn arguments_digest(rows:&[ModelFrameExitArgumentsRow])->Digest {
    use crate::codebook::Codebook;
    let mut h=IdHasher::new("model-frame-exit-arguments");h.i64(rows.len() as i64);
    for r in rows {h.i64(r.ordinal).id(r.argument_fact_id).id(r.expression_fact_id).i64(i64::from(r.safety.code()))
        .str(&r.parameter_name).i64(r.expression_offset).i64(r.expression_count)
        .bytes(&r.expression_digest.0).bytes(&r.parameters_digest.0);}
    h.finish_digest()
}
pub fn identity(r:&ModelFrameExitsRow)->Id {
    IdHasher::new("model-frame-exit").id(r.function_node_id).id(r.call_node_id)
        .id(r.call_fact_id).id(r.syntax_fact_id).id(r.target_node_id).id(r.pysa_fact_id).id(r.model_id)
        .str(&r.return_parameter).id(r.return_argument_fact_id).i64(r.argument_count).i64(r.signature_count)
        .bytes(&r.arguments_digest.0).i64(r.invocation_count).bytes(&r.invocation_digest.0).finish_id()
}
/// The independent call/binding count and commitment remain when retained members are removed.
/// Publication additionally reconstructs the pinned body and each source retention window.
pub fn admit(r:&ModelFrameExitsRow,args:&[ModelFrameExitArgumentsRow],steps:&[crate::behavior::ExpressionEvaluationStepsRow])->Result<(),ProofAdmissionError> {
    use crate::codebook::{SummaryFlowStepKind as K,ModeledArgumentEvaluationStatus as E};
    if r.frame_exit_id!=identity(r) || !(1..=128).contains(&r.argument_count)
        || !(1..=128).contains(&r.signature_count) || r.return_parameter.is_empty() || args.len()!=r.argument_count as usize
        || arguments_digest(args)!=r.arguments_digest
        || args.iter().enumerate().any(|(i,a)|a.snapshot_id!=r.snapshot_id || a.frame_exit_id!=r.frame_exit_id
            || a.ordinal!=i as i64 || a.safety==ReleaseSafety::Unknown)
        || args.iter().map(|a|a.argument_fact_id).collect::<std::collections::BTreeSet<_>>().len()!=args.len()
        || args.iter().filter(|a|a.argument_fact_id==r.return_argument_fact_id && a.parameter_name==r.return_parameter).count()!=1
        || steps.len()>crate::summary_contract::MAX_SUMMARY_PROOF_STEPS || steps.len()<5
        || r.invocation_count as usize!=steps.len() || steps_digest(steps)!=r.invocation_digest
        || steps.iter().enumerate().any(|(i,s)|s.snapshot_id!=r.snapshot_id || s.syntax_fact_id!=r.syntax_fact_id || s.ordinal!=i as i64 || s.status==E::Unknown)
        || steps[..3].iter().map(|s|s.kind).collect::<Vec<_>>()!=[K::ModuleImportBinding,K::ModuleImportRegion,K::CalleeResolution]
        || steps[steps.len()-2..].iter().map(|s|(s.kind,s.evidence_id)).collect::<Vec<_>>() !=[(K::CallSite,r.call_fact_id),(K::CallTarget,r.pysa_fact_id)] {
        return Err("frame release differs from its complete bound argument domain".into());
    }
    let mut at=3;
    for (i,s) in steps.iter().enumerate() {
        if s.kind==K::ExpressionSyntax && s.status==E::SourceCallNormal
            && (i==0 || steps[i-1].kind!=K::SourceCallNormal || steps[i-1].operand_fact_id!=s.operand_fact_id) {
            return Err("source operand omitted its call completion".into());
        }
        if s.kind!=K::ExpressionSyntax || s.status!=E::PinnedCallNormal {continue;}
        let normal=if i>0 && steps[i-1].kind==K::ModelRule {i-2} else {i.saturating_sub(1)};
        if normal==0 || steps[normal].kind!=K::PrecedingCallNormal || steps[normal-1].kind!=K::ModelFrameExit
            || steps[normal-1..=i].iter().any(|p|p.operand_fact_id!=s.operand_fact_id) {
            return Err("normal operand omitted its frame completion".into());
        }
    }
    for a in args {
        if a.expression_offset!=at as i64 || a.expression_count<=0 || a.expression_count as usize>steps.len().saturating_sub(at) {
            return Err("frame operand segment missing or out of order".into());
        }
        let end=at+a.expression_count as usize;
        let expression=&steps[at..end];
        if expression.last().is_none_or(|s|s.operand_fact_id!=a.expression_fact_id)
            || steps_digest(expression)!=a.expression_digest || r.signature_count as usize>steps.len()-end {
            return Err("frame operand evidence differs".into());
        }
        let bindings=&steps[end..end+r.signature_count as usize];
        if bindings.iter().any(|s|s.kind!=K::ParameterBinding || s.operand_fact_id!=a.expression_fact_id)
            || parameters_digest(&bindings.iter().map(|s|s.evidence_id).collect::<Vec<_>>())!=a.parameters_digest {
            return Err("frame parameter binding differs".into());
        }
        at=end+r.signature_count as usize;
    }
    if at!=steps.len()-2 {return Err("frame release has foreign invocation evidence".into());}
    Ok(())
}

/// Require every normal-call conclusion to retain its exact frame certificate, and every
/// certificate to belong to the immediately surrounding call occurrence. Source reconstruction
/// owns semantic retention; this check is shared by persisted summary and native consumers.
pub fn admit_proof<'a>(function:Id,proof:&[crate::id::recipe::SummaryFlowProofStep],
    resolve:impl Fn(Id)->Option<&'a ModelFrameExitsRow>,
    source:impl Fn(Id)->Option<crate::source_call::NormalSupport<'a>>)->Result<(),ProofAdmissionError> {
    use crate::codebook::SummaryFlowStepKind as K;
    if crate::source_call::expanded_len(proof.iter().map(|s|(s.kind,s.evidence_id)),&source)?>64 {
        return Err(ProofAdmissionError {reason:crate::codebook::BoundaryReason::SummaryProofLimit,
            message:"expanded source call proof exceeds its original cap".into()});
    }
    for (i,s) in proof.iter().enumerate() {
        if matches!(s.kind,K::SourceInvocation|K::ModelInvocation) {return Err("invocation cannot prove completed evaluation".into());}
        if s.kind==K::CallSite && (i+1>=proof.len() || proof[i+1].kind!=K::CallTarget) {
            return Err("call occurrence omitted its exact target".into());
        }
        if s.kind==K::CallTarget {
            if i==0 || proof[i-1].kind!=K::CallSite {return Err("orphan call target".into());}
            let tail=&proof[i+1..];
            // Every retained occurrence has its own completion. The local form is explicit;
            // its controls, callee identity and decreasing depth have separate shared admission.
            let local=tail.iter().skip_while(|p|matches!(p.kind,K::CallerConditionLink|K::CalleeConditionLink))
                .next().is_some_and(|p|p.kind==K::CalleeSummary);
            if !local && tail.first().is_none_or(|p|!matches!(p.kind,K::ModelFrameExit|K::SourceCallNormal)) {
                return Err("modeled call omitted its normal frame completion".into());
            }
        }
        if s.kind==K::ModelRule && (i<2 || proof[i-1].kind!=K::PrecedingCallNormal || proof[i-2].kind!=K::ModelFrameExit) {
            return Err("modeled call omitted its normal frame completion".into());
        }
        if s.kind==K::PrecedingCallNormal && (i==0 || proof[i-1].kind!=K::ModelFrameExit) {
            return Err("normal call omitted its frame release obligation".into());
        }
        if s.kind==K::SourceCallNormal {
            crate::source_call::admit_occurrence(function,proof,i,source(s.evidence_id).ok_or("missing source call completion")?)?;
        }
        if s.kind!=K::ModelFrameExit {continue;}
        let r=resolve(s.evidence_id).ok_or("missing model frame release")?;
        if r.function_node_id!=function || i<2 || i+1>=proof.len()
            || (proof[i-2].kind,proof[i-2].evidence_id)!=(K::CallSite,r.call_fact_id)
            || (proof[i-1].kind,proof[i-1].evidence_id)!=(K::CallTarget,r.pysa_fact_id)
            || (proof[i+1].kind,proof[i+1].evidence_id)!=(K::PrecedingCallNormal,r.model_id)
            || proof[i-2..=i+1].iter().any(|p|p.condition_id!=s.condition_id) {
            return Err("frame release has a foreign call, body or condition".into());
        }
    }
    Ok(())
}
