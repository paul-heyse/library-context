//! Direct finite capture provenance, resolved under the selected packet's pinned generation.
//! Snapshot timing is a qualified characterization and never supplies a runtime value.
use super::{Error, packet_reads::PacketLease};
use lctx_model::domain::{
    analysis,
    assertion::*,
    assumptions::AssumptionSet,
    attribution::*,
    calls::*,
    captures::*,
    conditions::Diagram,
    derivation::RowRef,
    execution::{
        capture_bridge::*, enriched_records::*, summary_capture::*, summary_consequences::*,
    },
    flow::*,
    flow_capture::*,
    lexical::*,
    normalized::entities::*,
    serving::*,
    source::*,
    transfer::summary::{SummaryPremise, TransferAlternative, TransferKey},
    *,
};

async fn row<R: Record>(lease: &mut PacketLease<'_>, id: Id<R>) -> Result<R, Error> {
    let batch = lease.read_ids::<R>(&[id]).await?;
    batch
        .rows()
        .iter()
        .find(|r| r.id() == id)
        .cloned()
        .ok_or(Error::Contract)
}
fn reference<R: Record>(id: Id<R>) -> ProofReference {
    ProofReference::from_canonical(RowRef::of(id))
}
fn name(value: impl Into<String>) -> Result<Name, Error> {
    Name::new(value.into()).map_err(|e| Error::Codec(e.to_string()))
}
async fn occurrence(
    lease: &mut PacketLease<'_>,
    id: Id<Occurrence>,
    input: Id<input::InputRevision>,
) -> Result<Occurrence, Error> {
    let site = row(lease, id).await?;
    let source = row(lease, site.source).await?;
    if source.input != input {
        return Err(Error::Contract);
    }
    Ok(site)
}
async fn qualification(
    lease: &mut PacketLease<'_>,
    id: Id<AssertionQualification>,
    input: Id<input::InputRevision>,
    context: Id<AnalysisContext>,
) -> Result<AssertionQualification, Error> {
    let q = row(lease, id).await?;
    if q.context != context {
        return Err(Error::Contract);
    }
    match row(lease, q.scope).await? {
        CoverageScope::Input { input: i } if i == input => {}
        CoverageScope::Artifact { artifact } => {
            if row(lease, artifact).await?.input != input {
                return Err(Error::Contract);
            }
        }
        CoverageScope::Module { module } => {
            let m = row(lease, module).await?;
            if row(lease, m.source).await?.input != input {
                return Err(Error::Contract);
            }
        }
        _ => return Err(Error::Contract),
    }
    Ok(q)
}

async fn placements(
    lease: &mut PacketLease<'_>,
    field: &'static str,
    id: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> Result<Vec<syntax::SyntaxPlacement>, Error> {
    let batch = lease
        .read_for::<syntax::SyntaxPlacement, Occurrence>(field, &[id])
        .await?;
    let mut selected = Vec::new();
    for placement in batch.rows() {
        if row(lease, placement.qualification).await?.context == context {
            selected.push(placement.clone());
        }
    }
    Ok(selected)
}

/// Native support admission is local to this capture answer. Candidate timing keeps its
/// actual qualification, while the origin and native capture require unconditional exactness.
/// The shared assumption-premise admission remains stricter and unchanged.
async fn native<S: Support>(
    lease: &mut PacketLease<'_>,
    support: &S,
    qid: Id<AssertionQualification>,
    input: Id<input::InputRevision>,
    context_id: Id<AnalysisContext>,
    characterization: bool,
) -> Result<CaptureNativeProofPacket, Error> {
    let q = qualification(lease, qid, input, context_id).await?;
    let a = support.attribution().ok_or(Error::Contract)?;
    if q.assumptions != AssumptionSet::empty_id()
        || (!characterization
            && (q.condition != Diagram::always().id()
                || q.modality != Modality::Definite
                || q.approximation != Approximation::Exact))
        || a.origin != Origin::AnalyzerAssertion
        || a.mode != ExtractionMode::NativeTraversal
    {
        return Err(Error::Contract);
    }
    let run = row(lease, a.run).await?;
    let context = row(lease, run.context).await?;
    let provider = row(lease, run.provider).await?;
    let surface = row(lease, a.surface).await?;
    let families = lease
        .read_for::<RunFamily, ProviderRun>("run", &[run.id()])
        .await?;
    if (run.input, run.context) != (input, context_id)
        || surface.provider != run.provider
        || surface.family != S::Assertion::FAMILY
        || a.fidelity != Fidelity::NativeStructural
        || !families.rows().iter().any(|f| f.family == surface.family)
    {
        return Err(Error::Contract);
    }
    let evidence = match row(lease, a.evidence).await? {
        Evidence::Invocation { run: r } => {
            if r != run.id() {
                return Err(Error::Contract);
            }
            AssumptionEvidencePacket::Invocation { run: r }
        }
        Evidence::Occurrence { occurrence: id } => {
            let site = occurrence(lease, id, input).await?;
            source_evidence(lease, site.source, site.start, site.end, input).await?
        }
        Evidence::SourceSpan { source, start, end } => {
            source_evidence(lease, source, start, end, input).await?
        }
    };
    let basis = lease.claim_basis(&q).await?;
    Ok(CaptureNativeProofPacket {
        assertion: reference(support.assertion()),
        qualification: q.id(),
        scope: q.scope,
        condition: q.condition,
        modality: q.modality,
        approximation: q.approximation,
        claim_basis: basis,
        support: AssumptionNativeSupportPacket {
            support: reference(support.id()),
            run: run.id(),
            input,
            context: context_id,
            environment: context.environment_digest,
            provider: name(provider.tool)?,
            provider_revision: name(provider.revision)?,
            provider_build: provider.build_digest,
            surface: name(surface.name)?,
            evidence,
            fidelity: a.fidelity,
        },
    })
}
async fn source_evidence(
    lease: &mut PacketLease<'_>,
    id: Id<SourceArtifact>,
    start: i64,
    end: i64,
    input: Id<input::InputRevision>,
) -> Result<AssumptionEvidencePacket, Error> {
    let source = row(lease, id).await?;
    if source.input != input || start < 0 || end < start || end > source.byte_len {
        return Err(Error::Contract);
    }
    Ok(AssumptionEvidencePacket::Source {
        artifact: id,
        path: name(source.path)?,
        content: source.content,
        start,
        end,
    })
}
macro_rules! lower_proof {
    ($lease:expr,$record:expr,$support:ty,$input:expr,$context:expr,$characterization:expr) => {{
        let supports = $lease
            .read_for::<$support, _>("assertion", &[$record.id()])
            .await?;
        let mut eligible = None;
        for support in supports.rows() {
            if support.assertion != $record.id()
                || support.origin != Origin::AnalyzerAssertion
                || support.mode != ExtractionMode::NativeTraversal
                || support.fidelity != Fidelity::NativeStructural
            {
                continue;
            }
            let run = row($lease, support.run).await?;
            let surface = row($lease, support.surface).await?;
            if (run.input, run.context) != ($input, $context)
                || surface.provider != run.provider
                || surface.family != <<$support as Support>::Assertion as Assertion>::FAMILY
            {
                continue;
            }
            if eligible.replace(support).is_some() {
                return Err(Error::Contract);
            }
        }
        let support = eligible.ok_or(Error::Contract)?;
        native(
            $lease,
            support,
            $record.qualification,
            $input,
            $context,
            $characterization,
        )
        .await?
    }};
}

/// Declaration projection supports the exact Source formal correspondence. Its attribution
/// is retained separately from the NativeStructural capture/origin runtime proof requirements.
async fn source_declaration(
    lease: &mut PacketLease<'_>,
    declared: &declarations::ParameterDeclaration,
    signature: &Signature,
    input: Id<input::InputRevision>,
    context_id: Id<AnalysisContext>,
) -> Result<CaptureSourceDeclarationPacket, Error> {
    let q = qualification(lease, declared.qualification, input, context_id).await?;
    if q.assumptions != AssumptionSet::empty_id()
        || q.condition != Diagram::always().id()
        || q.modality != Modality::Definite
        || q.approximation != Approximation::Exact
    {
        return Err(Error::Contract);
    }
    let symbol = row(lease, signature.symbol).await?;
    if signature.role != SignatureRole::Source || symbol.context != context_id {
        return Err(Error::Contract);
    }
    let supports = lease
        .read_for::<declarations::ParameterDeclarationSupport, _>("assertion", &[declared.id()])
        .await?;
    let mut eligible = None;
    for support in supports.rows() {
        if support.assertion != declared.id()
            || support.origin != Origin::AnalyzerAssertion
            || support.mode != ExtractionMode::NativeTraversal
            || !matches!(
                support.fidelity,
                Fidelity::ReportProjection | Fidelity::NativeStructural
            )
        {
            continue;
        }
        let run = row(lease, support.run).await?;
        let surface = row(lease, support.surface).await?;
        if (run.input, run.context, run.provider) != (input, context_id, symbol.provider)
            || surface.provider != run.provider
            || surface.family != FactFamily::Signatures
        {
            continue;
        }
        if eligible.replace(support).is_some() {
            return Err(Error::Contract);
        }
    }
    let support = eligible.ok_or(Error::Contract)?;
    let run = row(lease, support.run).await?;
    let context = row(lease, run.context).await?;
    let provider = row(lease, run.provider).await?;
    let surface = row(lease, support.surface).await?;
    let families = lease
        .read_for::<RunFamily, ProviderRun>("run", &[run.id()])
        .await?;
    if !families
        .rows()
        .iter()
        .any(|family| family.family == FactFamily::Signatures)
    {
        return Err(Error::Contract);
    }
    let evidence = match row(lease, support.evidence).await? {
        Evidence::Invocation { run: r } => {
            if r != run.id() {
                return Err(Error::Contract);
            }
            AssumptionEvidencePacket::Invocation { run: r }
        }
        Evidence::Occurrence { occurrence: id } => {
            let site = occurrence(lease, id, input).await?;
            source_evidence(lease, site.source, site.start, site.end, input).await?
        }
        Evidence::SourceSpan { source, start, end } => {
            source_evidence(lease, source, start, end, input).await?
        }
    };
    Ok(CaptureSourceDeclarationPacket {
        assertion: declared.id(),
        qualification: q.id(),
        scope: q.scope,
        condition: q.condition,
        modality: q.modality,
        approximation: q.approximation,
        claim_basis: lease.claim_basis(&q).await?,
        source_correspondence_only: true,
        support: CaptureSourceSupportPacket {
            support: reference(support.id()),
            run: run.id(),
            input,
            context: context_id,
            environment: context.environment_digest,
            provider_id: run.provider,
            provider: name(provider.tool)?,
            provider_revision: name(provider.revision)?,
            provider_build: provider.build_digest,
            surface: name(surface.name)?,
            evidence,
            origin: support.origin,
            mode: support.mode,
            fidelity: support.fidelity,
        },
    })
}

/// A composed result without this exact direct contribution deliberately has no capture DTO.
/// This is provenance of one answer, not a global capture-absence claim.
pub(super) async fn captures(
    lease: &mut PacketLease<'_>,
    proof: &ClaimProof,
    invocation: &analysis::summary::AnalysisInvocation,
    q: &AssertionQualification,
    basis: &ClaimBasisPacket,
) -> Result<
    (
        Vec<BehavioralCapturePacket>,
        Box<dyn lctx_model::domain::resources::Reservation>,
    ),
    Error,
> {
    let mut charge = lease.lease.budget.reserve("behavior-direct-captures", 0)?;
    let ClaimProof::Finite {
        claim,
        source,
        qualification: proof_q,
        ..
    } = proof
    else {
        return Ok((Vec::new(), charge));
    };
    let claim = row(lease, *claim).await?;
    let SummaryClaim::FiniteAlternative {
        transfer,
        qualification: claim_q,
        ..
    } = claim
    else {
        return Ok((Vec::new(), charge));
    };
    if *proof_q != q.id() || claim_q != q.id() || q.context != invocation.context {
        return Err(Error::Contract);
    }
    let key = row(lease, transfer).await?;
    if key.context != invocation.context {
        return Err(Error::Contract);
    }
    // Local and declared-model seeds have no composed transfer alternative. Their
    // finite proof remains valid, but it has no direct captured-entry contribution.
    let premise = row(lease, *source).await?;
    if matches!(
        premise,
        SummaryPremise::Local { .. } | SummaryPremise::Model { .. }
    ) {
        return Ok((Vec::new(), charge));
    }
    let alternatives = lease
        .read_for::<TransferAlternative, TransferKey>("transfer", &[transfer])
        .await?;
    let selected = alternatives
        .rows()
        .iter()
        .filter(|a| a.qualification == q.id())
        .collect::<Vec<_>>();
    if selected.len() != 1 || selected[0].scope != q.scope {
        return Err(Error::Contract);
    }
    let alternative = selected[0];
    let contributions = lease
        .read_for::<SummaryCaptureContribution, TransferAlternative>(
            "alternative",
            &[alternative.id()],
        )
        .await?;
    if contributions.rows().len() > 64 {
        return Err(Error::ResourceRefused("direct capture packet rows"));
    }
    let _scratch = lease.lease.budget.reserve(
        "behavior-capture-hydration",
        contributions.rows().len().saturating_mul(65536),
    )?;
    let mut packets = Vec::new();
    for contribution in contributions.rows() {
        let witness = row(lease, contribution.witness).await?;
        if (witness.invocation, witness.transfer, witness.qualification)
            != (invocation.id(), transfer, q.id())
        {
            return Err(Error::Contract);
        }
        let packet = capture(
            lease,
            alternative,
            contribution,
            &witness,
            invocation,
            q,
            basis,
        )
        .await?;
        let bytes = serde_json::to_vec(&packet)
            .map_err(|e| Error::Codec(e.to_string()))?
            .len();
        charge.try_resize(charge.size().saturating_add(bytes))?;
        packets.push(packet);
    }
    packets.sort_by_key(|packet| packet.witness);
    Ok((packets, charge))
}

#[allow(
    clippy::too_many_arguments,
    reason = "Selected canonical claim and its actual contribution retain separate identities."
)]
async fn capture(
    lease: &mut PacketLease<'_>,
    alternative: &TransferAlternative,
    contribution: &SummaryCaptureContribution,
    witness: &SummaryCaptureWitness,
    invocation: &analysis::summary::AnalysisInvocation,
    q: &AssertionQualification,
    basis: &ClaimBasisPacket,
) -> Result<BehavioralCapturePacket, Error> {
    let (input, context) = (invocation.input, invocation.context);
    let binding = row(lease, witness.binding).await?;
    let enriched = row(lease, binding.invocation).await?;
    // Check frame scope before any source/native hydration.
    if (enriched.input, enriched.context) != (input, context)
        || binding.qualification != q.id()
        || !binding.under_caller_entry
        || q.assumptions != AssumptionSet::empty_id()
        || q.modality != Modality::Definite
        || q.approximation != Approximation::Exact
        || q.condition != Diagram::always().id()
    {
        return Err(Error::Contract);
    }
    qualification(lease, q.id(), input, context).await?;
    let header = row(lease, binding.header).await?;
    let source_invocation = row(lease, header.invocation).await?;
    let call = row(lease, witness.call).await?;
    let caller = row(lease, witness.body).await?;
    let callee = row(lease, call.body).await?;
    let event = row(lease, header.event).await?;
    if (source_invocation.input, source_invocation.context) != (input, context)
        || (header.owner, header.callee, header.qualification)
            != (binding.caller, binding.callee, q.id())
        || (call.invocation, call.header, call.event, call.qualification)
            != (binding.invocation, header.id(), event.id(), q.id())
        || (caller.invocation, caller.owner, caller.qualification)
            != (binding.invocation, binding.caller, q.id())
        || (callee.invocation, callee.owner, callee.qualification)
            != (binding.invocation, binding.callee, q.id())
        || callee.declaration != header.declaration
        || event.context != context
        || row(lease, call.outcome).await? != ExecutionOutcome::Normal
    {
        return Err(Error::Contract);
    }
    let ExecutionOutcome::Return { site: returned_at } = row(lease, caller.outcome).await? else {
        return Err(Error::Contract);
    };
    let returned_placements = placements(lease, "parent", returned_at, context).await?;
    let returned = returned_placements
        .iter()
        .filter(|p| p.field == SyntaxField::Value)
        .collect::<Vec<_>>();
    if returned.len() != 1 || returned[0].occurrence != event.site {
        return Err(Error::Contract);
    }
    for id in [
        binding.read,
        caller.declaration,
        callee.declaration,
        event.site,
        returned_at,
    ] {
        occurrence(lease, id, input).await?;
    }
    let origin = row(lease, binding.origin).await?;
    let definition = row(lease, origin.definition).await?;
    let origin_site = occurrence(lease, definition.occurrence, input).await?;
    let outer = row(lease, origin.scope).await?;
    let native_capture = row(lease, binding.capture).await?;
    let timing = row(lease, binding.timing).await?;
    let inner = row(lease, timing.nested_scope).await?;
    let use_ = row(lease, timing.use_).await?;
    if outer.owner != caller.declaration
        || inner.owner != callee.declaration
        || timing.enclosing_scope != outer.id()
        || use_.occurrence != binding.read
        || timing.origin != FlowCaptureOrigin::OuterLocal
        || native_capture.origin != CaptureOrigin::OuterFunction
        || native_capture.declaring.is_none()
    {
        return Err(Error::Contract);
    }
    let origin_proof = lower_proof!(lease, origin, FlowDefinitionSupport, input, context, false);
    let capture_proof = lower_proof!(lease, native_capture, CaptureSupport, input, context, false);
    let timing_proof = lower_proof!(
        lease,
        timing,
        FlowCaptureTimingSupport,
        input,
        context,
        true
    );
    let mut proof = vec![
        reference(contribution.id()),
        reference(witness.id()),
        reference(alternative.id()),
        reference(binding.id()),
        reference(binding.value_source),
        reference(header.id()),
        reference(call.id()),
        reference(caller.id()),
        reference(callee.id()),
        reference(binding.origin),
        reference(binding.capture),
        reference(binding.timing),
        reference(origin.definition),
        reference(origin.scope),
        reference(timing.use_),
        reference(timing.nested_scope),
        reference(event.id()),
        reference(returned[0].id()),
    ];
    let value_source = match row(lease, binding.value_source).await? {
        CapturedValueSource::Entry {
            formal,
            parameter,
            declaration,
        } => {
            if row(lease, formal).await? != (ParameterEntity::Source { declaration }) {
                return Err(Error::Contract);
            }
            let parameter_row = row(lease, parameter).await?;
            let signature = row(lease, parameter_row.signature).await?;
            if signature.role != SignatureRole::Source {
                return Err(Error::Contract);
            }
            qualification(lease, signature.qualification, input, context).await?;
            occurrence(lease, declaration, input).await?;
            let links = lease
                .read_for::<ParameterEntityLink, ParameterEntity>("entity", &[formal])
                .await?;
            let links = links
                .rows()
                .iter()
                .filter(|l| l.parameter == parameter)
                .collect::<Vec<_>>();
            if links.len() != 1 {
                return Err(Error::Contract);
            }
            let declared = row(lease, links[0].declaration.ok_or(Error::Contract)?).await?;
            if declared.declaration != declaration || declared.parameter != parameter {
                return Err(Error::Contract);
            }
            let declared_proof =
                source_declaration(lease, &declared, &signature, input, context).await?;
            // Source formal and native identifier are distinct containers joined only by the actual child edge.
            let parent =
                origin_site_parent(lease, &origin_site, declaration, input, context).await?;
            let children = placements(lease, "parent", parent, context).await?;
            let child = children
                .iter()
                .filter(|p| {
                    p.field == SyntaxField::Child
                        && p.ordinal == 0
                        && p.occurrence == definition.occurrence
                })
                .collect::<Vec<_>>();
            if child.len() != 1 {
                return Err(Error::Contract);
            }
            let shape = row(lease, parameter_row.shape).await?;
            proof.extend([
                reference(formal),
                reference(parameter),
                reference(signature.id()),
                reference(declared.id()),
                reference(links[0].id()),
                reference(child[0].id()),
                declared_proof.support.support.clone(),
            ]);
            CapturedValueSourcePacket::Entry {
                formal,
                parameter,
                declaration,
                signature: signature.id(),
                ordinal: parameter_row.ordinal,
                name: Nullable(shape.name.map(|n| name(n.as_str())).transpose()?),
                source_correspondence: Box::new(declared_proof),
            }
        }
        CapturedValueSource::Literal {
            literal,
            value,
            statement,
        } => {
            occurrence(lease, value, input).await?;
            occurrence(lease, statement, input).await?;
            if origin.value != Some(value) {
                return Err(Error::Contract);
            }
            let literal_row = row(lease, literal).await?;
            proof.push(reference(literal));
            CapturedValueSourcePacket::Literal {
                value,
                statement,
                literal: LiteralPacket::from_canonical(&literal_row)?,
            }
        }
    };
    let mut candidates = Vec::new();
    let _candidate_charge = lease
        .lease
        .budget
        .reserve("capture-candidate-packets", MAX_CAPTURE_CANDIDATES * 1024)?;
    if let Some(inventory) = timing.inventory {
        let inventory_row = row(lease, inventory).await?;
        let rows = lease
            .read_for::<FlowCaptureCandidate, FlowCaptureInventory>("inventory", &[inventory])
            .await?;
        if rows.rows().len() > MAX_CAPTURE_CANDIDATES {
            return Err(Error::ResourceRefused("capture candidate packet rows"));
        }
        let mut rows = rows.rows().to_vec();
        rows.sort_by_key(|r| r.ordinal);
        let tuples = rows
            .iter()
            .map(|r| (r.target, r.condition, r.narrowing, r.precision_lost))
            .collect::<Vec<_>>();
        if FlowCaptureInventory::new(&tuples)?.0 != inventory_row
            || rows.iter().enumerate().any(|(i, r)| r.ordinal != i as i64)
        {
            return Err(Error::Contract);
        }
        proof.push(reference(inventory));
        for candidate in rows {
            let target = match row(lease, candidate.target).await? {
                FlowCaptureTarget::Bound { definition } => {
                    let bound = row(lease, definition).await?;
                    occurrence(lease, bound.occurrence, input).await?;
                    CaptureCandidateTargetPacket::Bound {
                        definition,
                        occurrence: bound.occurrence,
                        place: bound.place,
                    }
                }
                FlowCaptureTarget::Undefined => CaptureCandidateTargetPacket::Undefined {},
                FlowCaptureTarget::Deleted => CaptureCandidateTargetPacket::Deleted {},
                FlowCaptureTarget::Nested => CaptureCandidateTargetPacket::Nested {},
                FlowCaptureTarget::LoopHeader => CaptureCandidateTargetPacket::LoopHeader {},
                FlowCaptureTarget::Unattached => CaptureCandidateTargetPacket::Unattached {},
            };
            proof.extend([reference(candidate.id()), reference(candidate.target)]);
            candidates.push(CaptureCandidatePacket {
                ordinal: candidate.ordinal,
                target,
                condition: candidate.condition,
                narrowing: candidate.narrowing,
                precision_lost: candidate.precision_lost,
            });
        }
    }
    proof.extend([
        origin_proof.support.support.clone(),
        capture_proof.support.support.clone(),
        timing_proof.support.support.clone(),
    ]);
    Ok(BehavioralCapturePacket {
        witness: witness.id(),
        binding: binding.id(),
        alternative: alternative.id(),
        transfer: witness.transfer,
        input,
        context,
        read: binding.read,
        value_source,
        frame: CaptureFramePacket {
            header: header.id(),
            call: call.id(),
            caller_body: caller.id(),
            callee_body: callee.id(),
            event: event.id(),
            call_site: event.site,
            caller: binding.caller,
            callee: binding.callee,
            caller_declaration: caller.declaration,
            callee_declaration: callee.declaration,
            returned_at,
            under_caller_entry: binding.under_caller_entry,
            qualification: binding.qualification,
        },
        origin: CaptureOriginPacket {
            definition: origin.definition,
            occurrence: definition.occurrence,
            place: definition.place,
            scope: origin.scope,
            declaring: outer.owner,
            kind: origin.kind,
            proof: origin_proof,
        },
        native_capture: NativeCapturePacket {
            function: native_capture.function,
            declaring: Nullable(native_capture.declaring),
            name: name(native_capture.name)?,
            origin: native_capture.origin,
            mutable: Nullable(native_capture.mutable),
            timing: native_capture.timing,
            proof: capture_proof,
        },
        timing: CaptureTimingPacket {
            use_: timing.use_,
            nested_scope: timing.nested_scope,
            enclosing_scope: timing.enclosing_scope,
            origin: timing.origin,
            timing: timing.timing,
            state: timing.state,
            candidates,
            constraint: Nullable(timing.constraint),
            constraint_precision_lost: timing.constraint_precision_lost,
            characterization_only: true,
            proof: timing_proof,
        },
        claim_basis: basis.clone(),
        proof,
    })
}

/// The declaration is the Source formal root. Hydrate its actual Parameter child before its
/// Identifier child; neither equal ranges nor a synthesized formal grants correspondence.
async fn origin_site_parent(
    lease: &mut PacketLease<'_>,
    origin: &Occurrence,
    declaration: Id<Occurrence>,
    input: Id<input::InputRevision>,
    context: Id<AnalysisContext>,
) -> Result<Id<Occurrence>, Error> {
    let children = placements(lease, "occurrence", origin.id(), context).await?;
    let edges = children
        .iter()
        .filter(|p| p.field == SyntaxField::Child && p.ordinal == 0 && p.parent.is_some())
        .collect::<Vec<_>>();
    if edges.len() != 1 {
        return Err(Error::Contract);
    }
    let parent = occurrence(lease, edges[0].parent.ok_or(Error::Contract)?, input).await?;
    let root = occurrence(lease, declaration, input).await?;
    if parent.syntax_kind != SyntaxKind::Parameter
        || origin.syntax_kind != SyntaxKind::Identifier
        || origin.source != parent.source
        || origin.start != parent.start
        || origin.end != parent.end
        || origin.structural_path.len() != parent.structural_path.len() + 1
        || !origin.structural_path.starts_with(&parent.structural_path)
        || root.source != parent.source
        || root.start > parent.start
        || root.end < parent.end
        || !parent.structural_path.starts_with(&root.structural_path)
    {
        return Err(Error::Contract);
    }
    Ok(parent.id())
}
