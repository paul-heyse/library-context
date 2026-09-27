//! Exact fresh zero-argument source binding, independently consumed by invocation and outcome.
use crate::codebook::{Codebook, CompletionKind, SummaryFlowStepKind as K};
use crate::id::{Digest, Id, IdHasher, recipe::SummaryFlowProofStep};
use crate::source_body::{
    SourceBodyCompletionsRow, SourceBodyReleaseInputsRow, SourceBodyStepsRow,
};
use crate::summary_contract::ProofAdmissionError;
use crate::table::table;

table!(SourceCallBindings,SourceCallBindingsRow="source_call_bindings",
    family=Findings,key=[snapshot_id,binding_id],checks=[("bounded_header","header_count > 0 AND header_count <= 64")],
    {snapshot_id:Id,binding_id:Id,function_node_id:Id,call_node_id:Id,call_fact_id:Id,
     syntax_fact_id:Id,callee_node_id:Id,pysa_fact_id:Id,signature_fact_id:Id,declaration_fact_id:Id,
     header_fact_id:Id,statement_fact_id:Id,binding_fact_id:Id,reference_fact_id:Id,resolution_fact_id:Id,
     header_count:i64,header_digest:Digest}
);
table!(SourceCallNormals,SourceCallNormalsRow="source_call_normals",
    family=Findings,key=[snapshot_id,certificate_id],checks=[("bounded_body","body_count >= 0 AND body_count <= 64")],
    {snapshot_id:Id,certificate_id:Id,binding_id:Id,body_id:Id,body_count:i64,body_kind:CompletionKind}
);
table!(SourceCallHeaderSteps,SourceCallHeaderStepsRow="source_call_header_steps",
    family=Findings,key=[snapshot_id,binding_id,ordinal],checks=[("ordinal_nonnegative","ordinal >= 0")],
    {snapshot_id:Id,binding_id:Id,ordinal:i64,kind:K,evidence_id:Id}
);
pub fn header_digest(steps: &[SourceCallHeaderStepsRow]) -> Digest {
    crate::completion_proof::proof_digest(
        &steps
            .iter()
            .map(|s| SummaryFlowProofStep {
                kind: s.kind,
                evidence_id: s.evidence_id,
                condition_id: Id::ZERO,
            })
            .collect::<Vec<_>>(),
    )
}
pub fn binding_identity(r: &SourceCallBindingsRow) -> Id {
    IdHasher::new("source-call-binding")
        .id(r.function_node_id)
        .id(r.call_node_id)
        .id(r.call_fact_id)
        .id(r.syntax_fact_id)
        .id(r.callee_node_id)
        .id(r.pysa_fact_id)
        .id(r.signature_fact_id)
        .id(r.declaration_fact_id)
        .id(r.header_fact_id)
        .id(r.statement_fact_id)
        .id(r.binding_fact_id)
        .id(r.reference_fact_id)
        .id(r.resolution_fact_id)
        .i64(r.header_count)
        .bytes(&r.header_digest.0)
        .finish_id()
}
pub fn identity(r: &SourceCallNormalsRow) -> Id {
    IdHasher::new("source-call-normal")
        .id(r.binding_id)
        .id(r.body_id)
        .i64(r.body_count)
        .i64(i64::from(r.body_kind.code()))
        .finish_id()
}

/// Creation and binding only; no premise here concerns the invoked body or its outcome.
pub fn admit_binding(
    r: &SourceCallBindingsRow,
    header: &[SourceCallHeaderStepsRow],
) -> Result<(), ProofAdmissionError> {
    if r.binding_id != binding_identity(r)
        || r.function_node_id == r.callee_node_id
        || !(1..=64).contains(&r.header_count)
        || header.len() != r.header_count as usize
        || header_digest(header) != r.header_digest
        || header.iter().enumerate().any(|(i, s)| {
            s.snapshot_id != r.snapshot_id || s.binding_id != r.binding_id || s.ordinal != i as i64
        })
        || header
            .last()
            .is_none_or(|s| (s.kind, s.evidence_id) != (K::CompletionStatement, r.header_fact_id))
        || !header.iter().any(|s| {
            s.kind == K::DefinitionHeaderEvidence && s.evidence_id == r.declaration_fact_id
        })
        || !header
            .iter()
            .any(|s| s.kind == K::DefinitionHeaderEvidence && s.evidence_id == r.binding_fact_id)
        || header.iter().any(|s| {
            matches!(
                s.kind,
                K::SourceCallNormal | K::SourceInvocation | K::CalleeSummary
            )
        })
    {
        return Err("source call lacks its exact fresh binding and header".into());
    }
    Ok(())
}

/// A joined view over two independently admitted contracts, never another stored authority.
#[derive(Clone, Copy)]
pub struct NormalSupport<'a> {
    pub normal: &'a SourceCallNormalsRow,
    pub binding: &'a SourceCallBindingsRow,
}
impl NormalSupport<'_> {
    pub fn check_link(self) -> Result<(), ProofAdmissionError> {
        if self.normal.certificate_id != identity(self.normal)
            || self.binding.binding_id != binding_identity(self.binding)
            || self.normal.binding_id != self.binding.binding_id
            || self.normal.snapshot_id != self.binding.snapshot_id
        {
            return Err("normal source call has a foreign binding".into());
        }
        Ok(())
    }
}

/// Duplicate-aware preparation shared by expression, completion, summary and action consumers.
/// Global rows are indexed once; each proof lookup is independent of corpus size.
pub struct SourceCallIndex<'a> {
    normals: std::collections::HashMap<(Id, Id), Option<&'a SourceCallNormalsRow>>,
    bindings: std::collections::HashMap<(Id, Id), Option<&'a SourceCallBindingsRow>>,
}
impl<'a> SourceCallIndex<'a> {
    pub fn new(normals: &'a [SourceCallNormalsRow], bindings: &'a [SourceCallBindingsRow]) -> Self {
        let mut out = Self {
            normals: Default::default(),
            bindings: Default::default(),
        };
        for row in normals {
            out.normals
                .entry((row.snapshot_id, row.certificate_id))
                .and_modify(|r| *r = None)
                .or_insert(Some(row));
        }
        for row in bindings {
            out.bindings
                .entry((row.snapshot_id, row.binding_id))
                .and_modify(|r| *r = None)
                .or_insert(Some(row));
        }
        out
    }
    pub fn binding(&self, snapshot: Id, id: Id) -> Option<&'a SourceCallBindingsRow> {
        self.bindings.get(&(snapshot, id)).copied().flatten()
    }
    pub fn normal(&self, snapshot: Id, id: Id) -> Option<NormalSupport<'a>> {
        let normal = self.normals.get(&(snapshot, id)).copied().flatten()?;
        let binding = self.binding(snapshot, normal.binding_id)?;
        let support = NormalSupport { normal, binding };
        support.check_link().ok()?;
        Some(support)
    }
}

/// Support charged by one source occurrence; the body cost is a separate normal obligation.
pub fn binding_cost(binding: &SourceCallBindingsRow) -> Result<usize, ProofAdmissionError> {
    if binding.binding_id != binding_identity(binding) || !(1..=64).contains(&binding.header_count)
    {
        return Err("invalid source binding proof cost".into());
    }
    Ok(binding.header_count as usize + 6)
}
/// Source reconstruction owns semantic binding. These independent commitments prevent
/// omission and cross-owner substitution at every downstream consumer.
pub fn admit(
    r: &SourceCallNormalsRow,
    binding: &SourceCallBindingsRow,
    header: &[SourceCallHeaderStepsRow],
    body: &SourceBodyCompletionsRow,
    steps: &[SourceBodyStepsRow],
    releases: &[SourceBodyReleaseInputsRow],
) -> Result<(), ProofAdmissionError> {
    crate::source_body::admit(body, steps, releases)?;
    admit_binding(binding, header)?;
    NormalSupport { normal: r, binding }.check_link()?;
    if body.snapshot_id!=r.snapshot_id || body.body_id!=r.body_id || body.function_node_id!=binding.callee_node_id
        || body.syntax_fact_id!=binding.header_fact_id || body.declaration_fact_id!=binding.declaration_fact_id
        || !matches!(body.kind,CompletionKind::Normal|CompletionKind::Return)
        || r.body_count!=body.step_count || r.body_kind!=body.kind
        || body.reason.is_some() || body.release_reason.is_some()
        || header.len()+steps.len()+9>64
        // The initial adapter consumes only base bodies, never its own enriched results.
        || steps.iter().map(|s|s.kind).chain(header.iter().map(|s|s.kind))
            .any(|k|matches!(k,K::SourceCallNormal|K::SourceInvocation|K::CalleeSummary))
    {
        return Err("source call lacks its exact fresh callable, body or closed release".into());
    }
    Ok(())
}

pub fn admit_occurrence(
    function: Id,
    proof: &[SummaryFlowProofStep],
    at: usize,
    support: NormalSupport<'_>,
) -> Result<(), ProofAdmissionError> {
    support.check_link()?;
    let r = support.normal;
    let binding = support.binding;
    if binding.function_node_id != function
        || at < 2
        || at >= proof.len()
        || (proof[at - 2].kind, proof[at - 2].evidence_id) != (K::CallSite, binding.call_fact_id)
        || (proof[at - 1].kind, proof[at - 1].evidence_id) != (K::CallTarget, binding.pysa_fact_id)
        || proof[at].evidence_id != r.certificate_id
        || proof[at - 2..=at]
            .iter()
            .any(|p| p.condition_id != proof[at].condition_id)
    {
        return Err("source call completion has a foreign call, target or owner".into());
    }
    Ok(())
}

/// Count retained steps and referenced base-body/header support against the original cap.
/// Repeated occurrences are charged repeatedly, even when they cite one certificate.
pub fn expanded_len<'a>(
    proof: impl IntoIterator<Item = (K, Id)>,
    resolve: impl Fn(Id) -> Option<NormalSupport<'a>>,
) -> Result<usize, ProofAdmissionError> {
    let mut count = 0usize;
    for (kind, id) in proof {
        count = count.checked_add(1).ok_or("source call proof overflow")?;
        if kind == K::SourceCallNormal {
            let support = resolve(id).ok_or("missing source call completion")?;
            support.check_link()?;
            let row = support.normal;
            let binding = support.binding;
            if !(0..=64).contains(&row.body_count) || !(1..=64).contains(&binding.header_count) {
                return Err("invalid source call proof count".into());
            }
            count = count
                .checked_add(row.body_count as usize + binding_cost(binding)?)
                .ok_or("source call proof overflow")?;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: u8) -> Id {
        Id([n; 16])
    }
    fn seal(
        binding: &mut SourceCallBindingsRow,
        row: &mut SourceCallNormalsRow,
        header: &mut [SourceCallHeaderStepsRow],
    ) {
        binding.header_count = header.len() as i64;
        for (i, step) in header.iter_mut().enumerate() {
            step.ordinal = i as i64;
        }
        binding.header_digest = header_digest(header);
        binding.binding_id = binding_identity(binding);
        for step in header {
            step.binding_id = binding.binding_id;
        }
        row.binding_id = binding.binding_id;
        row.certificate_id = identity(row);
    }
    #[test]
    fn resealed_foreign_body_and_header_cannot_replace_call_premises() {
        let mut body = SourceBodyCompletionsRow {
            snapshot_id: id(1),
            body_id: Id::ZERO,
            function_node_id: id(2),
            declaration_fact_id: id(3),
            syntax_fact_id: id(4),
            kind: CompletionKind::Normal,
            terminal_fact_id: None,
            exception: None,
            reason: None,
            release_reason: None,
            function_retainer_required: true,
            runtime_statement_count: 0,
            step_count: 0,
            steps_digest: crate::source_body::steps_digest(&[]),
            release_count: 0,
            releases_digest: crate::source_body::releases_digest(&[]),
            work: 1,
        };
        body.body_id = crate::source_body::identity(&body);
        let mut binding = SourceCallBindingsRow {
            snapshot_id: id(1),
            binding_id: Id::ZERO,
            function_node_id: id(5),
            call_node_id: id(6),
            call_fact_id: id(7),
            syntax_fact_id: id(8),
            callee_node_id: body.function_node_id,
            pysa_fact_id: id(9),
            signature_fact_id: id(10),
            declaration_fact_id: body.declaration_fact_id,
            header_fact_id: body.syntax_fact_id,
            statement_fact_id: id(11),
            binding_fact_id: id(12),
            reference_fact_id: id(13),
            resolution_fact_id: id(14),
            header_count: 0,
            header_digest: Digest::ZERO,
        };
        let mut row = SourceCallNormalsRow {
            snapshot_id: id(1),
            certificate_id: Id::ZERO,
            binding_id: Id::ZERO,
            body_id: body.body_id,
            body_count: 0,
            body_kind: body.kind,
        };
        let mut header = [
            (K::DefinitionHeaderEvidence, body.declaration_fact_id),
            (K::DefinitionHeaderEvidence, binding.binding_fact_id),
            (K::CompletionStatement, binding.header_fact_id),
        ]
        .into_iter()
        .map(|(kind, evidence_id)| SourceCallHeaderStepsRow {
            snapshot_id: id(1),
            binding_id: Id::ZERO,
            ordinal: 0,
            kind,
            evidence_id,
        })
        .collect::<Vec<_>>();
        seal(&mut binding, &mut row, &mut header);
        assert!(admit(&row, &binding, &header, &body, &[], &[]).is_ok());
        let mut foreign = body.clone();
        foreign.function_node_id = id(20);
        foreign.body_id = crate::source_body::identity(&foreign);
        assert!(crate::source_body::admit(&foreign, &[], &[]).is_ok());
        let mut swapped = row.clone();
        swapped.body_id = foreign.body_id;
        swapped.certificate_id = identity(&swapped);
        assert!(admit(&swapped, &binding, &header, &foreign, &[], &[]).is_err());
        for index in [0, 1] {
            let mut changed = header.clone();
            changed[index].evidence_id = id(30);
            seal(&mut binding, &mut row, &mut changed);
            assert!(admit_binding(&binding, &changed).is_err());
            changed = header.clone();
            changed.remove(index);
            seal(&mut binding, &mut row, &mut changed);
            assert!(admit_binding(&binding, &changed).is_err());
        }
        seal(&mut binding, &mut row, &mut header);
        let proof = [
            (K::CallSite, binding.call_fact_id),
            (K::CallTarget, binding.pysa_fact_id),
            (K::SourceCallNormal, row.certificate_id),
        ]
        .map(|(kind, evidence_id)| SummaryFlowProofStep {
            kind,
            evidence_id,
            condition_id: id(40),
        });
        let support = NormalSupport {
            normal: &row,
            binding: &binding,
        };
        assert!(admit_occurrence(binding.function_node_id, &proof, 2, support).is_ok());
        let mut foreign = binding.clone();
        foreign.function_node_id = id(50);
        let mut swapped = row.clone();
        seal(&mut foreign, &mut swapped, &mut header);
        let mut substituted = proof;
        substituted[2].evidence_id = swapped.certificate_id;
        assert!(admit(&swapped, &foreign, &header, &body, &[], &[]).is_ok());
        assert!(
            admit_occurrence(
                binding.function_node_id,
                &substituted,
                2,
                NormalSupport {
                    normal: &swapped,
                    binding: &foreign
                }
            )
            .is_err()
        );
        assert!(
            SourceCallIndex::new(&[row.clone()], &[foreign])
                .normal(id(1), row.certificate_id)
                .is_none()
        );
        assert!(
            SourceCallIndex::new(&[row.clone(), row.clone()], &[binding.clone()])
                .normal(id(1), row.certificate_id)
                .is_none()
        );
        assert!(
            SourceCallIndex::new(&[row.clone()], &[binding.clone(), binding.clone()])
                .normal(id(1), row.certificate_id)
                .is_none()
        );
        // The binding remains valid independently of any changed body outcome.
        let committed = binding.binding_id;
        for kind in [CompletionKind::Raise, CompletionKind::Unknown] {
            body.kind = kind;
            body.body_id = crate::source_body::identity(&body);
            assert_eq!(binding_identity(&binding), committed);
        }
    }
}
