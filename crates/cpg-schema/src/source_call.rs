//! Exact fresh zero-argument source-call completion. The body is qualified independently;
//! this certificate adds the creation, binding and externally retained callable premises.
use crate::codebook::{Codebook,CompletionKind,SummaryFlowStepKind as K};
use crate::id::{Id,Digest,IdHasher,recipe::SummaryFlowProofStep};
use crate::source_body::{SourceBodyCompletionsRow,SourceBodyStepsRow,SourceBodyReleaseInputsRow};
use crate::summary_contract::ProofAdmissionError;
use crate::table::table;

table!(SourceCallNormals,SourceCallNormalsRow="source_call_normals",
    family=Findings,key=[snapshot_id,certificate_id],checks=[("bounded_header","header_count > 0 AND header_count <= 64")],
    {snapshot_id:Id,certificate_id:Id,function_node_id:Id,call_node_id:Id,call_fact_id:Id,
     syntax_fact_id:Id,callee_node_id:Id,pysa_fact_id:Id,signature_fact_id:Id,body_id:Id,
     header_fact_id:Id,statement_fact_id:Id,binding_fact_id:Id,reference_fact_id:Id,resolution_fact_id:Id,
     header_count:i64,header_digest:Digest,body_count:i64,body_kind:CompletionKind}
);
table!(SourceCallHeaderSteps,SourceCallHeaderStepsRow="source_call_header_steps",
    family=Findings,key=[snapshot_id,certificate_id,ordinal],checks=[("ordinal_nonnegative","ordinal >= 0")],
    {snapshot_id:Id,certificate_id:Id,ordinal:i64,kind:K,evidence_id:Id}
);
pub fn header_digest(steps:&[SourceCallHeaderStepsRow])->Digest {
    crate::completion_proof::proof_digest(&steps.iter().map(|s|SummaryFlowProofStep {
        kind:s.kind,evidence_id:s.evidence_id,condition_id:Id::ZERO,
    }).collect::<Vec<_>>())
}
pub fn identity(r:&SourceCallNormalsRow)->Id {
    IdHasher::new("source-call-normal").id(r.function_node_id).id(r.call_node_id).id(r.call_fact_id)
        .id(r.syntax_fact_id).id(r.callee_node_id).id(r.pysa_fact_id).id(r.signature_fact_id).id(r.body_id)
        .id(r.header_fact_id).id(r.statement_fact_id).id(r.binding_fact_id).id(r.reference_fact_id)
        .id(r.resolution_fact_id).i64(r.header_count).bytes(&r.header_digest.0).i64(r.body_count).i64(i64::from(r.body_kind.code())).finish_id()
}
/// Source reconstruction owns semantic binding. These independent commitments prevent
/// omission and cross-owner substitution at every downstream consumer.
pub fn admit(r:&SourceCallNormalsRow,header:&[SourceCallHeaderStepsRow],body:&SourceBodyCompletionsRow,
    steps:&[SourceBodyStepsRow],releases:&[SourceBodyReleaseInputsRow])->Result<(),ProofAdmissionError> {
    crate::source_body::admit(body,steps,releases)?;
    if r.certificate_id!=identity(r) || r.function_node_id==r.callee_node_id
        || body.snapshot_id!=r.snapshot_id || body.body_id!=r.body_id || body.function_node_id!=r.callee_node_id
        || body.syntax_fact_id!=r.header_fact_id || !matches!(body.kind,CompletionKind::Normal|CompletionKind::Return)
        || r.body_count!=body.step_count || r.body_kind!=body.kind
        || body.reason.is_some() || body.release_reason.is_some()
        || !(1..=64).contains(&r.header_count) || header.len()!=r.header_count as usize
        || header_digest(header)!=r.header_digest || header.len()+steps.len()+9>64
        || header.iter().enumerate().any(|(i,s)|s.snapshot_id!=r.snapshot_id
            || s.certificate_id!=r.certificate_id || s.ordinal!=i as i64)
        || header.last().is_none_or(|s|(s.kind,s.evidence_id)!=(K::CompletionStatement,r.header_fact_id))
        || !header.iter().any(|s|s.kind==K::DefinitionHeaderEvidence && s.evidence_id==body.declaration_fact_id)
        || !header.iter().any(|s|s.kind==K::DefinitionHeaderEvidence && s.evidence_id==r.binding_fact_id)
        // The initial adapter consumes only base bodies, never its own enriched results.
        || steps.iter().map(|s|s.kind).chain(header.iter().map(|s|s.kind))
            .any(|k|matches!(k,K::SourceCallNormal|K::CalleeSummary)) {
        return Err("source call lacks its exact fresh callable, body or closed release".into());
    }
    Ok(())
}

pub fn admit_occurrence(function:Id,proof:&[SummaryFlowProofStep],at:usize,r:&SourceCallNormalsRow)
    ->Result<(),ProofAdmissionError> {
    if r.certificate_id!=identity(r) || r.function_node_id!=function || at<2 || at>=proof.len()
        || (proof[at-2].kind,proof[at-2].evidence_id)!=(K::CallSite,r.call_fact_id)
        || (proof[at-1].kind,proof[at-1].evidence_id)!=(K::CallTarget,r.pysa_fact_id)
        || proof[at].evidence_id!=r.certificate_id
        || proof[at-2..=at].iter().any(|p|p.condition_id!=proof[at].condition_id) {
        return Err("source call completion has a foreign call, target or owner".into());
    }
    Ok(())
}

/// Count retained steps and referenced base-body/header support against the original cap.
/// Repeated occurrences are charged repeatedly, even when they cite one certificate.
pub fn expanded_len<'a>(proof:impl IntoIterator<Item=(K,Id)>,resolve:impl Fn(Id)->Option<&'a SourceCallNormalsRow>)
    ->Result<usize,ProofAdmissionError> {
    let mut count=0usize;
    for (kind,id) in proof {
        count=count.checked_add(1).ok_or("source call proof overflow")?;
        if kind==K::SourceCallNormal {
            let row=resolve(id).ok_or("missing source call completion")?;
            if !(0..=64).contains(&row.body_count) || !(1..=64).contains(&row.header_count) {
                return Err("invalid source call proof count".into());
            }
            count=count.checked_add((row.body_count+row.header_count+6) as usize).ok_or("source call proof overflow")?;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n:u8)->Id {Id([n;16])}
    fn seal(row:&mut SourceCallNormalsRow,header:&mut [SourceCallHeaderStepsRow]) {
        row.header_count=header.len() as i64;
        for (i,step) in header.iter_mut().enumerate() {step.ordinal=i as i64;}
        row.header_digest=header_digest(header);row.certificate_id=identity(row);
        for step in header {step.certificate_id=row.certificate_id;}
    }

    #[test]
    fn resealed_foreign_body_and_header_cannot_replace_call_premises() {
        let mut body=SourceBodyCompletionsRow {snapshot_id:id(1),body_id:Id::ZERO,
            function_node_id:id(2),declaration_fact_id:id(3),syntax_fact_id:id(4),
            kind:CompletionKind::Normal,terminal_fact_id:None,exception:None,reason:None,release_reason:None,
            function_retainer_required:true,runtime_statement_count:0,step_count:0,
            steps_digest:crate::source_body::steps_digest(&[]),release_count:0,
            releases_digest:crate::source_body::releases_digest(&[]),work:1};
        body.body_id=crate::source_body::identity(&body);
        let mut row=SourceCallNormalsRow {snapshot_id:id(1),certificate_id:Id::ZERO,
            function_node_id:id(5),call_node_id:id(6),call_fact_id:id(7),syntax_fact_id:id(8),
            callee_node_id:body.function_node_id,pysa_fact_id:id(9),signature_fact_id:id(10),body_id:body.body_id,
            header_fact_id:body.syntax_fact_id,statement_fact_id:id(11),binding_fact_id:id(12),
            reference_fact_id:id(13),resolution_fact_id:id(14),header_count:0,header_digest:Digest::ZERO,
            body_count:0,body_kind:body.kind};
        let mut header=[(K::DefinitionHeaderEvidence,body.declaration_fact_id),
            (K::DefinitionHeaderEvidence,row.binding_fact_id),(K::CompletionStatement,row.header_fact_id)]
            .into_iter().map(|(kind,evidence_id)|SourceCallHeaderStepsRow {snapshot_id:id(1),
                certificate_id:Id::ZERO,ordinal:0,kind,evidence_id}).collect::<Vec<_>>();
        seal(&mut row,&mut header);
        assert!(admit(&row,&header,&body,&[],&[]).is_ok());

        let mut foreign=body.clone();foreign.function_node_id=id(20);
        foreign.body_id=crate::source_body::identity(&foreign);
        assert!(crate::source_body::admit(&foreign,&[],&[]).is_ok());
        let mut swapped=row.clone();swapped.body_id=foreign.body_id;
        seal(&mut swapped,&mut header);
        assert!(admit(&swapped,&header,&foreign,&[],&[]).is_err());

        for index in [0,1] {
            let mut changed=header.clone();changed[index].evidence_id=id(30);
            seal(&mut row,&mut changed);
            assert!(admit(&row,&changed,&body,&[],&[]).is_err());
            changed=header.clone();changed.remove(index);seal(&mut row,&mut changed);
            assert!(admit(&row,&changed,&body,&[],&[]).is_err());
        }
        seal(&mut row,&mut header);
        let proof=[(K::CallSite,row.call_fact_id),(K::CallTarget,row.pysa_fact_id),
            (K::SourceCallNormal,row.certificate_id)].map(|(kind,evidence_id)|SummaryFlowProofStep {
                kind,evidence_id,condition_id:id(40)});
        assert!(admit_occurrence(row.function_node_id,&proof,2,&row).is_ok());
        let mut foreign=row.clone();foreign.function_node_id=id(50);seal(&mut foreign,&mut header);
        let mut substituted=proof;substituted[2].evidence_id=foreign.certificate_id;
        assert!(admit(&foreign,&header,&body,&[],&[]).is_ok());
        assert!(admit_occurrence(row.function_node_id,&substituted,2,&foreign).is_err());
    }
}
