//! Mechanical hydration of the model-owned source usage chain, bounded by the original grant.
use super::source_evidence::{EvidenceError as Error, NativePackets, PacketRows};
use lctx_model::domain::{
    assertion::{AssertionQualification, ProviderSurface, Support},
    attribution::*,
    calls::*,
    catalog::{CatalogMember, evidence::*},
    normalized::{bindings::*, events::*},
    serving::*,
    source::Occurrence,
    types::*,
    *,
};
fn need<R: Record>(rows: &PacketRows<R>, id: Id<R>) -> Result<R, Error> {
    rows.rows()
        .iter()
        .find(|r| r.id() == id)
        .cloned()
        .ok_or_else(|| Error::Model(ModelError::Invalid(format!("required native evidence row missing: {}", R::NAME))))
}
fn unavailable(reason: &str) -> Availability {
    Availability::Unavailable {
        reason: Name::new(reason).expect("constant"),
    }
}
fn text(value: String) -> Result<Text<0, 16384>, Error> {
    Text::new(value).map_err(|_| Error::ResourceRefused("indivisible native usage text"))
}
fn bounded<T>(rows: &[T], maximum: usize) -> Result<(), Error> {
    if rows.len() > maximum {
        Err(Error::ResourceRefused(
            "indivisible native usage membership",
        ))
    } else {
        Ok(())
    }
}
impl NativePackets<'_> {
    async fn usage_native_overloads(
        &mut self,
        site: Id<Occurrence>,
        context: Id<AnalysisContext>,
        grant: &OriginalRange,
        charge: &mut charged::StateCharge,
    ) -> Result<Vec<UsageNativeOverloadPacket>, Error> {
        use lctx_model::domain::normalized::overload_association::*;
        use lctx_model::domain::normalized::{
            callables::SignatureVariant, entities::SymbolEntityResolution,
        };
        let traces = self
            .read_for::<NativeOverloadObservation, Occurrence>("site", &[site])
            .await?;
        bounded(traces.rows(), 64)?;
        charge.grow(traces.rows().len().saturating_mul(4096))?;
        let mut packets = Vec::new();
        for trace in traces.rows() {
            let q = need(
                &self
                    .read_ids::<AssertionQualification>(&[trace.qualification])
                    .await?,
                trace.qualification,
            )?;
            if q.context != context {
                continue;
            }
            let support = self
                .usage_supports::<NativeOverloadSupport>(
                    trace.id(),
                    &q,
                    grant,
                    FactFamily::Types,
                    "pyrefly",
                    Origin::AnalyzerAssertion,
                    false,
                )
                .await?;
            if support.is_empty() {
                continue;
            }
            if !matches!(
                self.usage_occurrence(trace.arguments, grant).await?.0,
                Availability::Available { .. }
            ) {
                return Err(Error::Contract);
            }
            let rows = self
                .read_for::<NativeOverloadCandidate, NativeOverloadObservation>(
                    "trace",
                    &[trace.id()],
                )
                .await?;
            bounded(rows.rows(), MAX_OVERLOAD_CANDIDATES)?;
            charge.grow(rows.rows().len().saturating_mul(8192))?;
            let mut candidates = rows.rows().to_vec();
            candidates.sort_by_key(|r| r.ordinal);
            trace
                .verify_candidates(&candidates)
                .map_err(|_| Error::Contract)?;
            let mut member_packets = Vec::new();
            for candidate in candidates {
                let candidate_support = self
                    .usage_supports::<NativeOverloadCandidateSupport>(
                        candidate.id(),
                        &q,
                        grant,
                        FactFamily::Types,
                        "pyrefly",
                        Origin::AnalyzerAssertion,
                        false,
                    )
                    .await?;
                if candidate_support.is_empty() {
                    return Err(Error::Contract);
                }
                let assessments = self
                    .read_for::<OverloadVariantAssessment, NativeOverloadCandidate>(
                        "candidate",
                        &[candidate.id()],
                    )
                    .await?;
                if assessments.rows().len() != 1 {
                    return Err(Error::Model(ModelError::Invalid("native overload candidate requires one variant assessment".into())));
                }
                let assessment = &assessments.rows()[0];
                if assessment.context != context || assessment.policy != definition() {
                    return Err(Error::Contract);
                }
                let members = self
                    .read_for::<OverloadVariantCandidate, OverloadVariantAssessment>(
                        "assessment",
                        &[assessment.id()],
                    )
                    .await?;
                bounded(members.rows(), 4096)?;
                charge.grow(members.rows().len().saturating_mul(4096))?;
                let mut variants = Vec::new();
                let mut proof = vec![
                    ProofReference::from_canonical(derivation::RowRef::of(candidate.id()))?,
                    ProofReference::from_canonical(derivation::RowRef::of(assessment.id()))?,
                ];
                for member in members.rows() {
                    let variant = need(
                        &self.read_ids::<SignatureVariant>(&[member.variant]).await?,
                        member.variant,
                    )?;
                    let native = need(
                        &self
                            .read_ids::<NativeSignatureObservation>(&[member.native])
                            .await?,
                        member.native,
                    )?;
                    let nq = need(
                        &self
                            .read_ids::<AssertionQualification>(&[native.qualification])
                            .await?,
                        native.qualification,
                    )?;
                    if nq.context != context
                        || native.metadata_origin != candidate.origin
                        || variant.native != Some(native.id())
                        || variant.context != context
                    {
                        return Err(Error::Contract);
                    }
                    // Hydrate the nominal owner and the actual declaration support, not just IDs.
                    let _owner = need(
                        &self
                            .read_ids::<SymbolEntityResolution>(&[variant.resolution])
                            .await?,
                        variant.resolution,
                    )?;
                    let ns = need(
                        &self
                            .read_ids::<NativeSignatureSupport>(&[member.native_support])
                            .await?,
                        member.native_support,
                    )?;
                    let ts = need(
                        &self
                            .read_ids::<NativeOverloadSupport>(&[member.trace_support])
                            .await?,
                        member.trace_support,
                    )?;
                    let na = ns.attribution().ok_or(Error::Contract)?;
                    let ta = ts.attribution().ok_or(Error::Contract)?;
                    if ns.assertion() != native.id()
                        || ts.assertion() != trace.id()
                        || na.run != ta.run
                        || na.surface != ta.surface
                    {
                        return Err(Error::Contract);
                    }
                    proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                        ns.id(),
                    ))?);
                    proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                        ts.id(),
                    ))?);
                    proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                        member.id(),
                    ))?);
                    proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                        variant.id(),
                    ))?);
                    proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                        native.id(),
                    ))?);
                    variants.push(variant.id());
                }
                if let Some(origin) = candidate.origin {
                    let _origin = need(&self.read_ids::<ProviderSymbol>(&[origin]).await?, origin)?;
                    proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                        origin,
                    ))?);
                }
                variants.sort();
                variants.dedup();
                proof.sort();
                proof.dedup();
                member_packets.push(UsageNativeOverloadCandidatePacket {
                    candidate: candidate.id(),
                    ordinal: candidate.ordinal as u64,
                    term: candidate.term,
                    origin: Nullable(candidate.origin),
                    assessment: assessment.id(),
                    resolution: assessment.status,
                    reason: assessment.reason,
                    variant: Nullable(assessment.variant),
                    variants,
                    support: candidate_support,
                    proof,
                });
            }
            packets.push(UsageNativeOverloadPacket {
                trace: trace.id(),
                arguments: trace.arguments,
                selection: trace.selection,
                closest_ordinal: trace.closest_ordinal as u64,
                candidates: member_packets,
                support,
            });
        }
        packets.sort_by_key(|r| r.trace);
        Ok(packets)
    }
    async fn usage_occurrence(
        &mut self,
        occurrence: Id<Occurrence>,
        grant: &OriginalRange,
    ) -> Result<(Availability, Nullable<SourceCharacterizationSpan>), Error> {
        let row = need(
            &self.read_ids::<Occurrence>(&[occurrence]).await?,
            occurrence,
        )?;
        if row.source == grant.artifact
            && row.start >= 0
            && row.end >= row.start
            && grant.start <= row.start as u64
            && row.end as u64 <= grant.end
        {
            Ok((
                Availability::Available {},
                Nullable(Some(SourceCharacterizationSpan {
                    artifact: row.source,
                    start: row.start as u64,
                    end: row.end as u64,
                })),
            ))
        } else {
            Ok((
                unavailable("usage location outside granted original range"),
                Nullable(None),
            ))
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "assertion, qualification and original grant retain separate identities; provider family, tool, origin and fidelity are independently checked admission requirements"
    )]
    async fn usage_supports<R: Support>(
        &mut self,
        assertion: Id<R::Assertion>,
        q: &AssertionQualification,
        grant: &OriginalRange,
        family: FactFamily,
        tool: &str,
        origin: Origin,
        structural: bool,
    ) -> Result<Vec<NativeSourceSupportPacket>, Error> {
        let rows = self
            .read_for::<R, R::Assertion>("assertion", &[assertion])
            .await?;
        bounded(rows.rows(), 16)?;
        let mut out = vec![];
        for row in rows.rows() {
            let Some(a) = row.attribution() else { continue };
            let run = need(&self.read_ids::<ProviderRun>(&[a.run]).await?, a.run)?;
            let surface = need(
                &self.read_ids::<ProviderSurface>(&[a.surface]).await?,
                a.surface,
            )?;
            let provider = need(
                &self.read_ids::<Provider>(&[run.provider]).await?,
                run.provider,
            )?;
            if q.context != grant.context
                || run.context != q.context
                || run.input != grant_input(self, grant).await?
                || run.provider != surface.provider
                || surface.family != family
                || provider.tool != tool
                || a.origin != origin
                || a.mode != ExtractionMode::NativeTraversal
                || (structural && a.fidelity != Fidelity::NativeStructural)
            {
                continue;
            }
            let context = need(
                &self.read_ids::<AnalysisContext>(&[run.context]).await?,
                run.context,
            )?;
            out.push(NativeSourceSupportPacket {
                support: ProofReference::from_canonical(derivation::RowRef::of(row.id()))?,
                run: run.id(),
                input: run.input,
                context: run.context,
                environment: context.environment_digest,
                provider: Name::new(provider.tool).map_err(|_| Error::Contract)?,
                revision: Name::new(provider.revision).map_err(|_| Error::Contract)?,
                build: provider.build_digest,
                surface: Name::new(surface.name).map_err(|_| Error::Contract)?,
                evidence: ProofReference::from_canonical(derivation::RowRef::of(a.evidence))?,
                fidelity: a.fidelity,
            });
        }
        out.sort_by_key(|s| s.run);
        Ok(out)
    }
    pub(crate) async fn source_usage(
        &mut self,
        characterization: &SourceCharacterization,
        raw: &ProviderCallSite,
        grant: &OriginalRange,
        charge: &mut charged::StateCharge,
    ) -> Result<SourceUsagePacket, Error> {
        let rows = self
            .read_for::<SourceUsage, SourceCharacterization>(
                "characterization",
                &[characterization.id()],
            )
            .await?;
        if rows.rows().len() != 1 {
            return Err(Error::Model(ModelError::Invalid("source characterization requires one usage correspondence".into())));
        };
        let usage = &rows.rows()[0];
        let event = need(
            &self.read_ids::<NormalizedCallEvent>(&[usage.event]).await?,
            usage.event,
        )?;
        if event.site != raw.site || event.origin != raw.origin || event.context != grant.context {
            return Err(Error::Contract);
        }
        let syntax = self
            .read_for::<CallSyntax, Occurrence>("site", &[event.site])
            .await?;
        let mut syntaxes = vec![];
        for row in syntax.rows() {
            let q = need(
                &self
                    .read_ids::<AssertionQualification>(&[row.qualification])
                    .await?,
                row.qualification,
            )?;
            if q.context == event.context {
                syntaxes.push(row.clone());
            }
        }
        bounded(&syntaxes, 1)?;
        let mut callee = None;
        let mut callee_location = unavailable("canonical call syntax unavailable");
        let mut callee_span = Nullable(None);
        let mut arguments = vec![];
        let mut arguments_support = vec![];
        for row in &syntaxes {
            let q = need(
                &self
                    .read_ids::<AssertionQualification>(&[row.qualification])
                    .await?,
                row.qualification,
            )?;
            arguments_support = self
                .usage_supports::<CallSyntaxSupport>(
                    row.id(),
                    &q,
                    grant,
                    FactFamily::Syntax,
                    "ruff",
                    Origin::SourceObservation,
                    true,
                )
                .await?;
            if arguments_support.is_empty() {
                continue;
            }
            callee = Some(row.callee);
            (callee_location, callee_span) = self.usage_occurrence(row.callee, grant).await?;
            let args = self
                .read_for::<CallArgument, CallSyntax>("call", &[row.id()])
                .await?;
            bounded(args.rows(), 64)?;
            let mut args = args.rows().to_vec();
            args.sort_by_key(|a| a.ordinal);
            for arg in args {
                charge.grow(arg.heap_bytes().saturating_add(512))?;
                let (location, span) = self.usage_occurrence(arg.value, grant).await?;
                arguments.push(UsageArgumentPacket {
                    argument: arg.id(),
                    ordinal: arg.ordinal.try_into().map_err(|_| Error::Contract)?,
                    kind: arg.kind,
                    keyword: Nullable(arg.keyword.map(text).transpose()?),
                    value: arg.value,
                    location,
                    span,
                });
            }
        }
        let rows = self
            .read_for::<NormalizedCallAlternative, NormalizedCallEvent>("event", &[event.id()])
            .await?;
        bounded(rows.rows(), 64)?;
        let mut alternatives = rows.rows().to_vec();
        alternatives.sort_by_key(Record::id);
        let mut targets = vec![];
        for alternative in alternatives {
            charge.grow(4096)?;
            let source = need(
                &self
                    .read_ids::<CallAlternativeSource>(&[alternative.source])
                    .await?,
                alternative.source,
            )?;
            let target = need(
                &self.read_ids::<CallTarget>(&[source.target()]).await?,
                source.target(),
            )?;
            let q = need(
                &self
                    .read_ids::<AssertionQualification>(&[target.qualification])
                    .await?,
                target.qualification,
            )?;
            if target.site != event.site
                || target.origin != event.origin
                || q.context != event.context
            {
                return Err(Error::Contract);
            }
            let support = self
                .usage_supports::<CallTargetSupport>(
                    target.id(),
                    &q,
                    grant,
                    FactFamily::Calls,
                    "pyrefly",
                    Origin::AnalyzerAssertion,
                    true,
                )
                .await?;
            if support.is_empty() {
                return Err(Error::Contract);
            }
            let channel = need(
                &self.read_ids::<CallChannel>(&[target.channel]).await?,
                target.channel,
            )?;
            let channel_kind = match channel {
                CallChannel::Direct => UsageChannel::Direct {},
                CallChannel::HigherOrder { argument_index } => UsageChannel::HigherOrder {
                    argument_index: argument_index.try_into().map_err(|_| Error::Contract)?,
                },
            };
            let claim_basis = self.claim_basis(&q).await?;
            charge.grow(serde_json::to_vec(&claim_basis).map_err(|e| Error::Codec(e.to_string()))?.len().saturating_mul(2))?;
            let destination = need(
                &self
                    .read_ids::<CallDestination>(&[target.destination])
                    .await?,
                target.destination,
            )?;
            let (unresolved, native_unresolved) = match &destination {
                CallDestination::Unresolved { reason, native } => (Some(*reason), *native),
                _ => (None, None),
            };
            let receiver = need(
                &self.read_ids::<Receiver>(&[target.receiver]).await?,
                target.receiver,
            )?;
            let (receiver_location, receiver_span) = match receiver {
                Receiver::Bound { actual } => self.usage_occurrence(actual, grant).await?,
                Receiver::None => (unavailable("call has no bound receiver"), Nullable(None)),
                Receiver::Unknown { .. } => {
                    (unavailable("native receiver is unresolved"), Nullable(None))
                }
            };
            let associations = self
                .read_for::<ScenarioAssociation, NormalizedCallAlternative>(
                    "alternative",
                    &[alternative.id()],
                )
                .await?;
            bounded(associations.rows(), 64)?;
            let mut associated = vec![];
            for row in associations.rows() {
                charge.grow(row.heap_bytes().saturating_add(512))?;
                let member = need(
                    &self.read_ids::<CatalogMember>(&[row.member]).await?,
                    row.member,
                )?;
                charge.grow(member.heap_bytes())?;
                associated.push(UsageAssociationPacket {
                    association: row.id(),
                    member: row.member,
                    path: member
                        .path
                        .into_iter()
                        .map(|s| {
                            Name::new(s)
                                .map_err(|_| Error::ResourceRefused("native usage public path"))
                        })
                        .collect::<Result<_, _>>()?,
                    basis: row.basis,
                });
            }
            associated.sort_by_key(|a| a.association);
            let attempts = self
                .read_for::<CallBindingAttempt, NormalizedCallAlternative>(
                    "alternative",
                    &[alternative.id()],
                )
                .await?;
            bounded(attempts.rows(), 64)?;
            let mut applicability = vec![];
            for attempt in attempts.rows() {
                charge.grow(attempt.heap_bytes().saturating_add(1024))?;
                let role = match attempt.signature {
                    Some(id) => Some(need(&self.read_ids::<Signature>(&[id]).await?, id)?.role),
                    None => None,
                };
                let rows = self
                    .read_for::<CallBinding, CallBindingAttempt>("attempt", &[attempt.id()])
                    .await?;
                bounded(rows.rows(), 64)?;
                let mut bindings = vec![];
                for binding in rows.rows() {
                    charge.grow(binding.heap_bytes().saturating_add(512))?;
                    let source = need(
                        &self.read_ids::<BindingSource>(&[binding.source]).await?,
                        binding.source,
                    )?;
                    let (location, span) = match source {
                        BindingSource::Actual { occurrence }
                        | BindingSource::ClassOf { actual: occurrence } => {
                            self.usage_occurrence(occurrence, grant).await?
                        }
                        _ => (
                            unavailable("binding has no authored actual occurrence"),
                            Nullable(None),
                        ),
                    };
                    bindings.push(UsageBindingPacket {
                        binding: binding.id(),
                        slot: binding.slot,
                        kind: binding.kind,
                        source: binding.source,
                        location,
                        span,
                    });
                }
                bindings.sort_by_key(|b| b.binding);
                applicability.push(UsageApplicabilityPacket {
                    attempt: attempt.id(),
                    variant: Nullable(attempt.variant),
                    signature: Nullable(attempt.signature),
                    role: Nullable(role),
                    receiver: attempt.receiver,
                    adjustment: attempt.adjustment,
                    authority: attempt.authority,
                    authority_reason: attempt.authority_reason,
                    outcome: attempt.outcome,
                    reason: attempt.reason,
                    refusal: Nullable(attempt.refusal),
                    bindings,
                });
            }
            applicability.sort_by_key(|a| a.attempt);
            charge.grow(support.len().saturating_mul(2048))?;
            targets.push(UsageTargetPacket {
                alternative: alternative.id(),
                target: target.id(),
                qualification: q.id(),
                scope: q.scope,
                condition: q.condition,
                modality: q.modality,
                approximation: q.approximation,
                claim_basis,
                channel: target.channel,
                channel_kind,
                implicit: target.implicit,
                destination: target.destination,
                symbol: Nullable(destination.symbol()),
                unresolved: Nullable(unresolved),
                native_unresolved: Nullable(native_unresolved),
                entity: Nullable(alternative.entity),
                resolution: alternative.status,
                reason: alternative.reason,
                phase: target.phase,
                receiver: target.receiver,
                receiver_location,
                receiver_span,
                receiver_class: Nullable(target.receiver_class),
                passing: Nullable(target.passing),
                associations: associated,
                applicability,
                support,
            });
        }
        let rows = self
            .read_for::<TypeObservation, Occurrence>("subject", &[event.site])
            .await?;
        let mut overloads = vec![];
        for row in rows.rows() {
            if !matches!(
                row.role,
                TypeRole::ChosenOverload | TypeRole::OverloadCandidates
            ) {
                continue;
            }
            let q = need(
                &self
                    .read_ids::<AssertionQualification>(&[row.qualification])
                    .await?,
                row.qualification,
            )?;
            if q.context != event.context {
                continue;
            }
            let support = self
                .usage_supports::<TypeSupport>(
                    row.id(),
                    &q,
                    grant,
                    FactFamily::Types,
                    "pyrefly",
                    Origin::AnalyzerAssertion,
                    false,
                )
                .await?;
            if support.is_empty() {
                continue;
            }
            charge.grow(
                row.heap_bytes()
                    .saturating_add(support.len().saturating_mul(2048))
                    .saturating_add(512),
            )?;
            overloads.push(UsageOverloadPacket {
                observation: row.id(),
                role: row.role,
                term: row.term,
                variant_availability: unavailable(
                    "structural type observations do not identify a candidate; see original native overload membership",
                ),
                variant: Nullable(None),
                support,
            });
        }
        let native_overloads = self
            .usage_native_overloads(event.site, event.context, grant, charge)
            .await?;
        bounded(&overloads, 64)?;
        overloads.sort_by_key(|o| o.observation);
        charge.grow(arguments_support.len().saturating_mul(2048))?;
        let chosen = if overloads.iter().any(|o| o.role == TypeRole::ChosenOverload) {
            Availability::Available {}
        } else {
            unavailable("no supported native chosen overload trace")
        };
        let association = if targets.iter().any(|t| !t.associations.is_empty()) {
            Availability::Available {}
        } else {
            unavailable("native call has no API target association")
        };
        Ok(SourceUsagePacket {
            usage: usage.id(),
            event: event.id(),
            site: event.site,
            syntax: Nullable(syntaxes.first().map(Record::id)),
            syntax_location: if arguments_support.is_empty() {
                unavailable("canonical call syntax support unavailable")
            } else {
                Availability::Available {}
            },
            callee: Nullable(callee),
            callee_location,
            callee_span,
            arguments,
            arguments_support,
            targets,
            association,
            overloads,
            native_overloads,
            chosen,
        })
    }
}
async fn grant_input(
    packet: &mut NativePackets<'_>,
    grant: &OriginalRange,
) -> Result<Id<input::InputRevision>, Error> {
    let source = need(
        &packet
            .read_ids::<source::SourceArtifact>(&[grant.artifact])
            .await?,
        grant.artifact,
    )?;
    Ok(source.input)
}
