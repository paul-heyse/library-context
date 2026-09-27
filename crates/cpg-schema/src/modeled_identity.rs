//! An occurrence-specific lexical input identity composed with an existing exact model proof.
//! Raw provider approximation is retained; only this source/model path is discharged.
use crate::codebook::SummaryFlowStepKind as K;
use crate::completion_proof::proof_digest;
use crate::id::{Digest,Id,IdHasher,recipe::SummaryFlowProofStep};
use crate::table::table;

table!(
    SourceModeledIdentities,SourceModeledIdentitiesRow = "source_modeled_identities",
    family = Findings,
    key = [snapshot_id,identity_id],
    checks = [("span_order","start_byte >= 0 AND end_byte > start_byte"),
        ("proof_count","model_proof_count > 0 AND model_proof_count <= 64")],
    {
        snapshot_id:Id, identity_id:Id, function_node_id:Id, parameter_node_id:Id,
        source_flow_fact_id:Id, source_origin_id:Id, condition_id:Id, return_site_fact_id:Id,
        call_fact_id:Id, call_expression_fact_id:Id, source_argument_fact_id:Id,
        pysa_fact_id:Id, model_id:Id, rule_id:Id, callee_resolution_fact_id:Id,
        expression_fact_id:Id, reference_fact_id:Id, resolution_fact_id:Id,
        binding_fact_id:Id, parameter_fact_id:Id, scope_fact_id:Id,
        module_node_id:Id, start_byte:i64, end_byte:i64,
        model_proof_count:i64, model_proof_digest:Digest,
    }
);

pub fn identity(r:&SourceModeledIdentitiesRow)->Id {
    IdHasher::new("source-modeled-identity").id(r.function_node_id).id(r.parameter_node_id)
        .id(r.source_flow_fact_id).id(r.source_origin_id).id(r.condition_id).id(r.return_site_fact_id)
        .id(r.call_fact_id).id(r.call_expression_fact_id).id(r.source_argument_fact_id)
        .id(r.pysa_fact_id).id(r.model_id).id(r.rule_id).id(r.callee_resolution_fact_id)
        .id(r.expression_fact_id).id(r.reference_fact_id).id(r.resolution_fact_id)
        .id(r.binding_fact_id).id(r.parameter_fact_id).id(r.scope_fact_id)
        .id(r.module_node_id).i64(r.start_byte).i64(r.end_byte)
        .i64(r.model_proof_count).bytes(&r.model_proof_digest.0).finish_id()
}

/// Modeled paths need a value basis independently of their model and completion obligations.
/// Callee-based paths cite a separate summary's value basis instead.
pub fn admit_value_basis(path_depth:i64,proof:&[SummaryFlowProofStep])->bool {
    if path_depth==0 {return !proof.iter().any(|s|s.kind==K::SourceModeledIdentity);}
    if path_depth<0 {return false;}
    if proof.iter().any(|s|s.kind==K::CalleeSummary) {return !proof.iter().any(|s|s.kind==K::SourceModeledIdentity);}
    if !proof.iter().any(|s|s.kind==K::ModelRule) {return false;}
    proof.iter().filter(|s|matches!(s.kind,K::RawIdentity|K::SourceModeledIdentity)).count()==1
}

/// Admit the committed call group after replacing its raw-source marker with this certificate.
/// Source reconstruction owns the lexical and model premises; completion remains independent.
pub fn admits(r:&SourceModeledIdentitiesRow,function:Id,parameter:Id,source:Id,origin:Id,
    condition:Id,return_site:Id,proof:&[SummaryFlowProofStep])->bool {
    if r.identity_id!=identity(r) || r.function_node_id!=function || r.parameter_node_id!=parameter
        || r.source_flow_fact_id!=source || r.source_origin_id!=origin || r.condition_id!=condition
        || r.return_site_fact_id!=return_site || r.start_byte<0 || r.end_byte<=r.start_byte
        || !(1..=64).contains(&r.model_proof_count) {return false;}
    let starts:Vec<_>=proof.iter().enumerate().filter(|(_,s)|s.kind==K::CalleeResolution
        && s.evidence_id==r.callee_resolution_fact_id).map(|(i,_)|i).collect();
    let [start]=starts.as_slice() else {return false;};
    let Some(group)=proof.get(*start..start.saturating_add(r.model_proof_count as usize)) else {return false;};
    if group.len()<5 || group.iter().any(|s|s.condition_id!=condition)
        || group[group.len()-3..].iter().map(|s|(s.kind,s.evidence_id)).collect::<Vec<_>>()
            !=[(K::CallSite,r.call_fact_id),(K::CallTarget,r.pysa_fact_id),(K::ModelRule,r.rule_id)]
        || proof.iter().filter(|s|s.kind==K::SourceModeledIdentity).count()!=1
        || group.iter().filter(|s|s.kind==K::SourceModeledIdentity && s.evidence_id==r.identity_id).count()!=1
        || group.iter().any(|s|s.kind==K::RawIdentity)
        || !proof[start+group.len()..].iter().any(|s|s.kind==K::ReturnExit && s.evidence_id==return_site) {
        return false;
    }
    let original:Vec<_>=group.iter().map(|s|SummaryFlowProofStep {
        kind:if s.kind==K::SourceModeledIdentity {K::RawIdentity} else {s.kind},
        evidence_id:if s.kind==K::SourceModeledIdentity {source} else {s.evidence_id},
        condition_id:s.condition_id,
    }).collect();
    proof_digest(&original)==r.model_proof_digest
}
