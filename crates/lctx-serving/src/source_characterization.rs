//! Bounded stored source characterization within the original request's immutable source grant.
use super::source_evidence::{EvidenceError as Error, NativePackets, PacketRows};
use lctx_model::domain::{
    analysis::native::NativeAssertionPremise,
    assertion::{Support, *},
    attribution::*,
    catalog::evidence::{
        DiagnosticUseAssessment, DiagnosticUseLink, DiagnosticUsePath, DiagnosticUseTarget,
        OriginalSource, ScenarioAssociation, SourceCharacterization,
        SourceCharacterizationScenario, SourceUsage,
    },
    diagnostics::*,
    serving::*,
    source::{Occurrence, SourceArtifact},
    *,
};
fn wire(error: WireError) -> Error {
    Error::Codec(error.to_string())
}
fn native_text(value: String) -> Result<Text<0, 16384>, Error> {
    Text::new(value)
        .map_err(|_| Error::ResourceRefused("indivisible native source characterization text"))
}
fn required<R: Record>(batch: &PacketRows<R>, id: Id<R>) -> Result<R, Error> {
    batch.get(id)?
        .cloned()
        .ok_or_else(|| {
            Error::Model(ModelError::Invalid(format!(
                "required native evidence row missing: {}",
                R::NAME
            )))
        })
}
fn unavailable() -> Availability {
    Availability::Unavailable {
        reason: Name::new("location outside granted original range or native location unavailable")
            .expect("constant"),
    }
}
impl NativePackets<'_> {
    async fn diagnostic_correlation(
        &mut self,
        characterization: &SourceCharacterization,
        grant: &OriginalRange,
        maximum: usize,
    ) -> Result<Nullable<DiagnosticCorrelationPacket>, Error> {
        let assessments = self
            .read_for::<DiagnosticUseAssessment, SourceCharacterization>(
                "characterization",
                &[characterization.id()],
            )
            .await?;
        if assessments.rows().is_empty() {
            return Ok(Nullable(None));
        }
        if assessments.rows().len() != 1 {
            return Err(Error::Contract);
        }
        let assessment = &assessments.rows()[0];
        let links = self
            .read_for::<DiagnosticUseLink, DiagnosticUseAssessment>(
                "assessment",
                &[assessment.id()],
            )
            .await?;
        let mut charge = charged::StateCharge::new(&self.budget, "diagnostic-correlation-packet");
        charge.grow(links.rows().len().saturating_mul(4096))?;
        let mut links = links.rows().to_vec();
        links.sort_by_key(Record::id);
        let mut items = Vec::new();
        let mut uses = std::collections::BTreeSet::new();
        for link in &links {
            let usage = required(
                &self.read_ids::<SourceUsage>(&[link.usage]).await?,
                link.usage,
            )?;
            uses.insert(link.usage);
            let event = required(
                &self
                    .read_ids::<normalized::events::NormalizedCallEvent>(&[usage.event])
                    .await?,
                usage.event,
            )?;
            let source = required(
                &self
                    .read_ids::<normalized::events::CallEventSource>(&[link.event_source])
                    .await?,
                link.event_source,
            )?;
            let subject = required(
                &self.read_ids::<Occurrence>(&[link.subject]).await?,
                link.subject,
            )?;
            let usage_characterization = required(
                &self
                    .read_ids::<SourceCharacterization>(&[usage.characterization])
                    .await?,
                usage.characterization,
            )?;
            let usage_q = required(
                &self
                    .read_ids::<AssertionQualification>(&[usage_characterization.qualification])
                    .await?,
                usage_characterization.qualification,
            )?;
            if usage_q.context != grant.context
                || source.event != event.id()
                || event.context != grant.context
                || usage_characterization.artifact != grant.artifact
                || subject.source != grant.artifact
            {
                return Err(Error::Contract);
            }
            let native = required(
                &self
                    .read_ids::<calls::ProviderCallSite>(&[source.observation])
                    .await?,
                source.observation,
            )?;
            if native.site != event.site || native.origin != event.origin {
                return Err(Error::Contract);
            }
            // Fully hydrate source ownership and path before page trimming, using the declared mapping.
            let owner = required(
                &self
                    .read_ids::<normalized::entities::OccurrenceOwnership>(&[event.owner])
                    .await?,
                event.owner,
            )?;
            if owner.occurrence != event.site {
                return Err(Error::Contract);
            }
            let paths = self
                .read_for::<DiagnosticUsePath, DiagnosticUseLink>("link", &[link.id()])
                .await?;
            let mut path = paths.rows().to_vec();
            path.sort_by_key(|p| p.ordinal);
            let mut current = link.subject;
            let mut proof = vec![
                ProofReference::from_canonical(derivation::RowRef::of(link.id()))?,
                ProofReference::from_canonical(derivation::RowRef::of(link.event_source))?,
            ];
            for (ordinal, step) in path.iter().enumerate() {
                let placement = required(
                    &self
                        .read_ids::<syntax::SyntaxPlacement>(&[step.placement])
                        .await?,
                    step.placement,
                )?;
                let q = required(
                    &self
                        .read_ids::<AssertionQualification>(&[placement.qualification])
                        .await?,
                    placement.qualification,
                )?;
                if step.ordinal != ordinal as i64
                    || placement.occurrence != current
                    || q.context != grant.context
                {
                    return Err(Error::Contract);
                }
                current = placement.parent.ok_or(Error::Contract)?;
                proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                    step.id(),
                ))?);
            }
            if current != event.site {
                return Err(Error::Contract);
            }
            let targets = self
                .read_for::<DiagnosticUseTarget, DiagnosticUseLink>("link", &[link.id()])
                .await?;
            charge.grow(targets.rows().len().saturating_mul(1024))?;
            let mut target_packets = vec![];
            for target in targets.rows() {
                let association = required(
                    &self
                        .read_ids::<ScenarioAssociation>(&[target.association])
                        .await?,
                    target.association,
                )?;
                let alternative = required(
                    &self
                        .read_ids::<normalized::events::NormalizedCallAlternative>(&[
                            association.alternative
                        ])
                        .await?,
                    association.alternative,
                )?;
                if alternative.event != event.id() {
                    return Err(Error::Contract);
                }
                self.read_ids::<catalog::CatalogMember>(&[association.member])
                    .await?;
                proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                    target.id(),
                ))?);
                target_packets.push(DiagnosticUseTargetPacket {
                    association: association.id(),
                    member: association.member,
                    scenario: association.scenario,
                    alternative: association.alternative,
                    basis: association.basis,
                });
            }
            target_packets.sort_by_key(|t| t.association);
            if items.len() < maximum {
                items.push(DiagnosticUseLinkPacket {
                    link: link.id(),
                    usage: usage.id(),
                    event: event.id(),
                    subject: link.subject,
                    event_source: source.id(),
                    targets: target_packets,
                    proof,
                });
            }
        }
        if assessment.uses != uses.len() as i64 {
            return Err(Error::Contract);
        }
        let omitted = links.len().saturating_sub(items.len()) as u64;
        Ok(Nullable(Some(DiagnosticCorrelationPacket {
            assessment: assessment.id(),
            status: assessment.status,
            uses: uses.len() as u64,
            remainder: assessment.remainder,
            links: SectionPage {
                availability: if omitted > 0 {
                    Availability::Partial {
                        reason: Name::new("diagnostic correlation page bound reached")
                            .map_err(wire)?,
                    }
                } else {
                    Availability::Available {}
                },
                items,
                continuation: Optional::default(),
                omitted,
                truncated: omitted > 0,
            },
        })))
    }

    async fn span_in_grant(
        &mut self,
        span: Option<EvidenceSourceSpanId>,
        grant: &OriginalRange,
    ) -> Result<(Availability, Nullable<SourceCharacterizationSpan>), Error> {
        let Some(span) = span else {
            return Ok((unavailable(), Nullable(None)));
        };
        match required(&self.read_ids::<Evidence>(&[span.id()]).await?, span.id())? {
            Evidence::SourceSpan { source, start, end }
                if source == grant.artifact
                    && start >= 0
                    && end >= start
                    && grant.start <= start as u64
                    && end as u64 <= grant.end =>
            {
                Ok((
                    Availability::Available {},
                    Nullable(Some(SourceCharacterizationSpan {
                        artifact: source,
                        start: start as u64,
                        end: end as u64,
                    })),
                ))
            }
            _ => Ok((unavailable(), Nullable(None))),
        }
    }
    async fn characterization_source(
        &mut self,
        source: Id<OriginalSource>,
    ) -> Result<SourceCharacterizationSpan, Error> {
        let (artifact, start, end) =
            match required(&self.read_ids::<OriginalSource>(&[source]).await?, source)? {
                OriginalSource::Artifact { artifact } => {
                    let row = required(
                        &self.read_ids::<SourceArtifact>(&[artifact]).await?,
                        artifact,
                    )?;
                    (artifact, 0, row.byte_len)
                }
                OriginalSource::Occurrence { occurrence } => {
                    let row = required(
                        &self.read_ids::<Occurrence>(&[occurrence]).await?,
                        occurrence,
                    )?;
                    (row.source, row.start, row.end)
                }
                OriginalSource::Span { span } => {
                    match required(&self.read_ids::<Evidence>(&[span.id()]).await?, span.id())? {
                        Evidence::SourceSpan { source, start, end } => (source, start, end),
                        _ => return Err(Error::Contract),
                    }
                }
            };
        if start < 0 || end < start {
            return Err(Error::Contract);
        };
        Ok(SourceCharacterizationSpan {
            artifact,
            start: start as u64,
            end: end as u64,
        })
    }
    async fn source_support<R: Support>(
        &mut self,
        support: Id<R>,
        q: &AssertionQualification,
        family: FactFamily,
        tool: &str,
    ) -> Result<NativeSourceSupportPacket, Error> {
        let row = required(&self.read_ids::<R>(&[support]).await?, support)?;
        let a = row.attribution().ok_or(Error::Contract)?;
        let run = required(&self.read_ids::<ProviderRun>(&[a.run]).await?, a.run)?;
        let surface = required(
            &self.read_ids::<ProviderSurface>(&[a.surface]).await?,
            a.surface,
        )?;
        let provider = required(
            &self.read_ids::<Provider>(&[run.provider]).await?,
            run.provider,
        )?;
        let context = required(
            &self.read_ids::<AnalysisContext>(&[run.context]).await?,
            run.context,
        )?;
        if run.context != q.context
            || run.provider != surface.provider
            || surface.family != family
            || provider.tool != tool
            || a.origin != Origin::AnalyzerAssertion
            || a.mode != ExtractionMode::NativeTraversal
            || a.fidelity != Fidelity::NativeStructural
        {
            return Err(Error::Contract);
        }
        Ok(NativeSourceSupportPacket {
            support: ProofReference::from_canonical(derivation::RowRef::of(support))?,
            run: run.id(),
            input: run.input,
            context: run.context,
            environment: context.environment_digest,
            provider: Name::new(provider.tool).map_err(wire)?,
            revision: Name::new(provider.revision).map_err(wire)?,
            build: provider.build_digest,
            surface: Name::new(surface.name).map_err(wire)?,
            evidence: ProofReference::from_canonical(derivation::RowRef::of(a.evidence))?,
            fidelity: a.fidelity,
        })
    }
    async fn annotations(
        &mut self,
        subject: DiagnosticSubject,
        grant: &OriginalRange,
        charge: &mut charged::StateCharge,
    ) -> Result<Vec<SourceAnnotationPacket>, Error> {
        let id = subject.id();
        let actual = required(&self.read_ids::<DiagnosticSubject>(&[id]).await?, id)?;
        if actual != subject {
            return Err(Error::Contract);
        }
        let rows = self
            .read_for::<DiagnosticAnnotation, DiagnosticSubject>("diagnostic", &[id])
            .await?;
        if rows.rows().len() > 32 {
            return Err(Error::ResourceRefused(
                "indivisible native diagnostic annotations",
            ));
        }
        charge.grow(rows.rows().iter().fold(0usize, |n, row| {
            n.saturating_add(
                row.heap_bytes()
                    .saturating_add(size_of::<DiagnosticAnnotation>().saturating_mul(4)),
            )
        }))?;
        let mut rows = rows.rows().to_vec();
        rows.sort_by_key(|a| a.ordinal);
        let mut out = vec![];
        for row in rows {
            charge.grow(row.heap_bytes().saturating_add(512))?;
            let (location, span) = self.span_in_grant(row.span, grant).await?;
            out.push(SourceAnnotationPacket {
                annotation: row.id(),
                location,
                span,
                label: Nullable(row.label.map(native_text).transpose()?),
            });
        }
        Ok(out)
    }
    pub(crate) async fn source_characterization(
        &mut self,
        grant: &OriginalRange,
        maximum: usize,
    ) -> Result<SectionPage<SourceCharacterizationPacket>, Error> {
        let mut charge =
            charged::StateCharge::new(&self.budget, "original-source-characterization-output");
        let rows = self
            .read_for::<SourceCharacterization, SourceArtifact>("artifact", &[grant.artifact])
            .await?;
        charge.grow(
            rows.rows()
                .len()
                .saturating_mul(size_of::<SourceCharacterization>().saturating_mul(4)),
        )?;
        let mut rows = rows.rows().to_vec();
        rows.sort_by_key(Record::id);
        let mut items = vec![];
        let mut omitted = 0u64;
        for row in rows {
            let q = required(
                &self
                    .read_ids::<AssertionQualification>(&[row.qualification])
                    .await?,
                row.qualification,
            )?;
            if q.context != grant.context {
                continue;
            }
            let source = self.characterization_source(row.source).await?;
            if source.artifact != grant.artifact
                || source.start < grant.start
                || source.end > grant.end
            {
                continue;
            }
            if items.len() == maximum {
                omitted += 1;
                continue;
            }
            charge.grow(4096)?;
            let premise = required(
                &self
                    .read_ids::<NativeAssertionPremise>(&[row.native])
                    .await?,
                row.native,
            )?;
            let payload = match premise {
                NativeAssertionPremise::ProviderCallSite { assertion, support } => {
                    let raw = required(
                        &self
                            .read_ids::<calls::ProviderCallSite>(&[assertion])
                            .await?,
                        assertion,
                    )?;
                    let basis = self
                        .source_support(support, &q, FactFamily::Calls, "pyrefly")
                        .await?;
                    let usage = self.source_usage(&row, &raw, grant, &mut charge).await?;
                    // The per-characterization precharge covers this fixed payload;
                    // the usage operation separately charges its retained collections.
                    (
                        SourceCharacterizationPayload::Usage {
                            usage: Box::new(usage),
                        },
                        basis,
                    )
                }
                NativeAssertionPremise::RuffDiagnosticObservation { assertion, support } => {
                    let raw = required(
                        &self
                            .read_ids::<RuffDiagnosticObservation>(&[assertion])
                            .await?,
                        assertion,
                    )?;
                    charge.grow(raw.heap_bytes().saturating_add(2048))?;
                    let basis = self
                        .source_support(support, &q, FactFamily::Lexical, "ruff")
                        .await?;
                    let annotations = self
                        .annotations(
                            DiagnosticSubject::Ruff {
                                observation: assertion,
                            },
                            grant,
                            &mut charge,
                        )
                        .await?;
                    (
                        SourceCharacterizationPayload::RuffDiagnostic {
                            observation: assertion,
                            rule: raw.rule,
                            native_id: Name::new(raw.native_id).map_err(wire)?,
                            native_code: Name::new(raw.native_code).map_err(wire)?,
                            primary_location: raw.location,
                            severity: raw.severity,
                            channel: raw.channel,
                            message: native_text(raw.message)?,
                            settings: raw.settings,
                            annotations,
                        },
                        basis,
                    )
                }
                NativeAssertionPremise::PyreflyDiagnosticObservation { assertion, support } => {
                    let raw = required(
                        &self
                            .read_ids::<PyreflyDiagnosticObservation>(&[assertion])
                            .await?,
                        assertion,
                    )?;
                    charge.grow(raw.heap_bytes().saturating_add(2048))?;
                    let basis = self
                        .source_support(support, &q, FactFamily::Types, "pyrefly")
                        .await?;
                    let annotations = self
                        .annotations(
                            DiagnosticSubject::Pyrefly {
                                observation: assertion,
                            },
                            grant,
                            &mut charge,
                        )
                        .await?;
                    (
                        SourceCharacterizationPayload::PyreflyDiagnostic {
                            observation: assertion,
                            category: Name::new(raw.category).map_err(wire)?,
                            primary_location: raw.location,
                            severity: raw.severity,
                            channel: raw.channel,
                            baseline: raw.baseline,
                            header: native_text(raw.header)?,
                            details: Nullable(raw.details.map(native_text).transpose()?),
                            annotations,
                        },
                        basis,
                    )
                }
                NativeAssertionPremise::NativeParameterDefinitionObservation {
                    assertion,
                    support,
                } => {
                    let raw = required(
                        &self
                            .read_ids::<NativeParameterDefinitionObservation>(&[assertion])
                            .await?,
                        assertion,
                    )?;
                    charge.grow(raw.heap_bytes().saturating_add(2048))?;
                    let basis = self
                        .source_support(support, &q, FactFamily::Types, "pyrefly")
                        .await?;
                    let (target_location, target) = self.span_in_grant(raw.target, grant).await?;
                    (
                        SourceCharacterizationPayload::ParameterDefinition {
                            observation: assertion,
                            parameter: raw.parameter,
                            answer: raw.answer,
                            answer_count: u64::try_from(raw.answer_count)
                                .map_err(|_| Error::Contract)?,
                            role: raw.role,
                            reason: Nullable(raw.reason),
                            metadata: Nullable(raw.metadata),
                            symbol_kind: Nullable(raw.symbol_kind),
                            target_location,
                            target,
                            target_name: Nullable(raw.target_name.map(native_text).transpose()?),
                        },
                        basis,
                    )
                }
                _ => return Err(Error::Contract),
            };
            let scenarios = self
                .read_for::<SourceCharacterizationScenario, SourceCharacterization>(
                    "characterization",
                    &[row.id()],
                )
                .await?;
            if scenarios.rows().len() > 64 {
                return Err(Error::ResourceRefused(
                    "indivisible source characterization scenario context",
                ));
            }
            let mut containing_scenarios = scenarios
                .rows()
                .iter()
                .map(|s| s.scenario)
                .collect::<Vec<_>>();
            containing_scenarios.sort();
            containing_scenarios.dedup();
            let mut proof = vec![
                ProofReference::from_canonical(derivation::RowRef::of(row.id()))?,
                ProofReference::from_canonical(derivation::RowRef::of(row.native))?,
                payload.1.support,
            ];
            if let SourceCharacterizationPayload::Usage { usage } = &payload.0 {
                proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                    usage.usage,
                ))?);
            }
            let diagnostic_correlation = self.diagnostic_correlation(&row, grant, maximum).await?;
            if let Some(correlation) = &diagnostic_correlation.0 {
                proof.push(ProofReference::from_canonical(derivation::RowRef::of(
                    correlation.assessment,
                ))?);
            }
            items.push(SourceCharacterizationPacket {
                characterization: row.id(),
                qualification: row.qualification,
                source,
                containing_scenarios,
                diagnostic_correlation,
                payload: payload.0,
                support: payload.1,
                proof,
            });
        }
        Ok(SectionPage {
            availability: if omitted == 0 {
                Availability::Available {}
            } else {
                Availability::Partial {
                    reason: Name::new("source characterization row bound reached").map_err(wire)?,
                }
            },
            items,
            continuation: Optional::default(),
            omitted,
            truncated: omitted > 0,
        })
    }
}
