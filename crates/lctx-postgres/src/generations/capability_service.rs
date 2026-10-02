//! Canonical brief hydration, preserving complete authored renderings and actual support rows.
use super::{Error, GenerationLease, GenerationService, RequestExecution};
use lctx_model::domain::{
    serving::*,
    synthesis::{
        assertions::{AssertionSource, ProgrammaticAssertion, ProgrammaticAssertionSupport},
        briefs::*,
        documentary::{DocumentaryConclusion, ProseSlice, ProseSource},
    },
    *,
};
use std::collections::BTreeSet;
fn wire(e: WireError) -> Error {
    Error::Codec(e.to_string())
}
fn need<R: Record>(b: &Batch<R>, id: Id<R>) -> Result<R, Error> {
    b.rows()
        .iter()
        .find(|r| r.id() == id)
        .cloned()
        .ok_or(Error::Contract)
}
impl GenerationService {
    pub async fn capability(
        &self,
        e: &RequestExecution,
        r: &GetCapabilityRequest,
    ) -> Result<GetCapabilityResponse, Error> {
        if !e.shares_guard(&self.guard()) {
            return Err(Error::State);
        }
        let id = r.capability;
        let retained = e.clone();
        let (brief, assertions, originals) = e
            .query(move |lease| {
                Box::pin(async move {
                    let brief = lease.brief(id).await?;
                    retained.retain(
                        "capability-rendered-output",
                        brief.0.1.len() * 2
                            + brief
                                .1
                                .iter()
                                .try_fold(0usize, |bytes, a| {
                                    Ok::<_, Error>(
                                        bytes.saturating_add(
                                            super::catalog_service::serialized_len(a)?,
                                        ),
                                    )
                                })?
                                .saturating_mul(2)
                            + brief.2.len() * 256,
                    )?;
                    Ok(brief)
                })
            })
            .await?;
        let mut ranges = Vec::new();
        for (source, context) in originals {
            ranges.push(self.original_range(e, source, Some(context)).await?);
        }
        e.confirm().await?;
        Ok(GetCapabilityResponse {
            generation: GenerationKey(*self.generation().bytes()),
            capability: CapabilityPacket {
                capability: id,
                title: Name::new(brief.0.title.as_str()).map_err(wire)?,
                rendered: Text::new(brief.1).map_err(wire)?,
                assertions,
                originals: ranges,
                availability: Availability::Available {},
                unreviewed: true,
                documentation_only: brief.0.documentation_only,
            },
        })
    }
}
impl GenerationLease {
    async fn brief(
        &mut self,
        id: Id<Brief>,
    ) -> Result<
        (
            (Brief, String),
            Vec<AssertionPacket>,
            Vec<(OriginalReference, Id<attribution::AnalysisContext>)>,
        ),
        Error,
    > {
        let brief = need(&self.read_ids::<Brief>(&[id]).await?, id)?;
        if brief.bytes < 0 || brief.bytes as u64 > ResourceLimits::default().expanded_response_bytes
        {
            return Err(Error::ResourceRefused("indivisible capability rendering"));
        }
        let documents = self
            .read_for::<BriefDocument, Brief>("brief", &[id])
            .await?;
        let mut parts = documents.rows().iter().collect::<Vec<_>>();
        parts.sort_by_key(|p| p.ordinal);
        let _render_charge = self
            .budget
            .reserve("brief-rendering", brief.bytes as usize * 2)?;
        let mut rendered = String::with_capacity(brief.bytes as usize);
        for (ordinal, part) in parts.iter().enumerate() {
            if part.ordinal != ordinal as i64 {
                return Err(Error::Contract);
            }
            rendered.push_str(part.text.as_str());
        }
        if rendered.len() as i64 != brief.bytes
            || ContentHash::of(rendered.as_bytes()) != brief.rendered
        {
            return Err(Error::Contract);
        }
        let assertion_refs = self
            .read_for::<BriefAssertion, Brief>("brief", &[id])
            .await?;
        let mut refs = assertion_refs.rows().to_vec();
        refs.sort_by_key(|r| r.ordinal);
        if refs.iter().enumerate().any(|(n, r)| r.ordinal != n as i64) {
            return Err(Error::Contract);
        }
        let assertions: Vec<_> = refs.iter().map(|r| r.assertion).collect();
        let values = self.read_ids::<ProgrammaticAssertion>(&assertions).await?;
        if values.rows().len() != assertions.len() {
            return Err(Error::Contract);
        }
        let supports = self
            .read_for::<ProgrammaticAssertionSupport, ProgrammaticAssertion>(
                "assertion",
                &assertions,
            )
            .await?;
        for assertion in &assertions {
            if !supports.rows().iter().any(|s| s.assertion == *assertion) {
                return Err(Error::Contract);
            }
        }
        let source_ids: Vec<_> = supports
            .rows()
            .iter()
            .map(|s| s.source)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let sources = self.read_ids::<AssertionSource>(&source_ids).await?;
        if sources.rows().len() != source_ids.len() {
            return Err(Error::Contract);
        }
        let documentary = self.read_for::<BriefSource, Brief>("brief", &[id]).await?;
        let conclusions: Vec<_> = documentary.rows().iter().map(|r| r.documentary).collect();
        let conclusions = self.read_ids::<DocumentaryConclusion>(&conclusions).await?;
        if conclusions.rows().len() != documentary.rows().len() {
            return Err(Error::Contract);
        }
        let slice_ids: Vec<_> = conclusions
            .rows()
            .iter()
            .map(|r| r.prose)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let slices = self.read_ids::<ProseSlice>(&slice_ids).await?;
        if slices.rows().len() != slice_ids.len() {
            return Err(Error::Contract);
        }
        let prose_ids: Vec<_> = slices
            .rows()
            .iter()
            .map(|r| r.source)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let prose = self.read_ids::<ProseSource>(&prose_ids).await?;
        if prose.rows().len() != prose_ids.len() {
            return Err(Error::Contract);
        }
        let qualifications: Vec<_> = conclusions
            .rows()
            .iter()
            .map(|r| r.qualification())
            .chain(
                values
                    .rows()
                    .iter()
                    .map(ProgrammaticAssertion::qualification),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let q = self
            .read_ids::<assertion::AssertionQualification>(&qualifications)
            .await?;
        let mut originals = Vec::new();
        for conclusion in conclusions.rows() {
            let context = need(&q, conclusion.qualification())?.context;
            originals.push((
                OriginalReference::Prose {
                    slice: conclusion.prose,
                },
                context,
            ));
        }
        let packet_bytes = values
            .rows()
            .iter()
            .map(|a| {
                a.heap_bytes()
                    .saturating_mul(4)
                    .saturating_add(2 * size_of::<AssertionPacket>())
            })
            .sum::<usize>()
            .saturating_add(supports.rows().len().saturating_mul(1024));
        let _packets = self
            .budget
            .reserve("capability-attributed-assertions", packet_bytes)?;
        let mut packets = Vec::new();
        for id in assertions {
            let value = values
                .rows()
                .iter()
                .find(|a| a.id() == id)
                .ok_or(Error::Contract)?;
            need(&q, value.qualification())?;
            let mut claim_supports = Vec::new();
            for support in supports.rows().iter().filter(|s| s.assertion == id) {
                let source = need(&sources, support.source)?;
                let target = match source {
                    AssertionSource::Documentary { conclusion } => {
                        derivation::RowRef::of(conclusion)
                    }
                    AssertionSource::Structural { conclusion } => {
                        derivation::RowRef::of(conclusion)
                    }
                    AssertionSource::Analytic { conclusion } => derivation::RowRef::of(conclusion),
                    AssertionSource::Summary { facet } => derivation::RowRef::of(facet),
                    AssertionSource::AuthoredCode { conclusion } => {
                        derivation::RowRef::of(conclusion)
                    }
                };
                claim_supports.push(AssertionSupportPacket {
                    support: support.id(),
                    role: support.role,
                    source: support.source,
                    proof: vec![
                        ProofReference::from_canonical(derivation::RowRef::of(support.id())),
                        ProofReference::from_canonical(derivation::RowRef::of(support.source)),
                        ProofReference::from_canonical(target),
                    ],
                });
            }
            claim_supports.sort_by_key(|s| s.support);
            packets.push(AssertionPacket {
                assertion: id,
                kind: value.kind(),
                section: value.section(),
                status: value.status(),
                qualification: value.qualification(),
                text: Text::new(value.text()).map_err(wire)?,
                supports: claim_supports,
            });
        }
        Ok(((brief, rendered), packets, originals))
    }
}
