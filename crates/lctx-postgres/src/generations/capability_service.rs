//! Canonical brief hydration, preserving complete authored renderings and actual support rows.
use super::{Error, GenerationService, RequestExecution};
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
                    let mut scope =
                        super::packet_reads::PacketLease::new::<CapabilityPacket>(lease);
                    let lease = &mut scope;
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
impl super::packet_reads::PacketLease<'_> {
    /// Citation hydration only: lower Model owns question-specific admission.
    async fn terminal_pair<S: assertion::Support>(&mut self, assertion: Id<S::Assertion>, support: Id<S>, input: Id<input::InputRevision>, context: Id<attribution::AnalysisContext>) -> Result<Vec<ProofReference>, Error> {
        need(&self.read_ids::<S::Assertion>(&[assertion]).await?, assertion)?;
        let support = need(&self.read_ids::<S>(&[support]).await?, support)?;
        if support.assertion() != assertion { return Err(Error::Contract); }
        let a = support.attribution().ok_or(Error::Contract)?;
        let run = need(&self.read_ids::<attribution::ProviderRun>(&[a.run]).await?, a.run)?;
        if (run.input,run.context)!=(input,context) { return Err(Error::Contract); }
        need(&self.read_ids::<assertion::ProviderSurface>(&[a.surface]).await?, a.surface)?;
        need(&self.read_ids::<assertion::Evidence>(&[a.evidence]).await?, a.evidence)?;
        Ok(vec![derivation::RowRef::of(assertion),derivation::RowRef::of(support.id()),derivation::RowRef::of(a.run),derivation::RowRef::of(a.surface),derivation::RowRef::of(a.evidence)].into_iter().map(ProofReference::from_canonical).collect())
    }
    async fn terminal_question(&mut self, id: Id<execution::summary_terminal::SummaryTerminalWitness>, q: &assertion::AssertionQualification) -> Result<TerminalQuestionPacket, Error> {
        use synthesis::{terminal, summary};
        use execution::{protocol_interpretation::{ConditionalTerminalFrontier, NormalContinuationRestriction},closed_targets::ClosedTargetAssessment,summary_consequences::SummaryClaim,summary_terminal::SummaryTerminalWitness};
        let budget = self.lease.budget.clone();
        let mut data = terminal::Data::new(&budget);
        let mut summary = summary::Data::new(&budget);
        let witness = need(&self.read_ids::<SummaryTerminalWitness>(&[id]).await?, id)?;
        let frontier = need(&self.read_ids::<ConditionalTerminalFrontier>(&[witness.frontier]).await?, witness.frontier)?;
        let edge = need(&self.read_ids::<NormalContinuationRestriction>(&[witness.restriction]).await?, witness.restriction)?;
        let target = need(&self.read_ids::<ClosedTargetAssessment>(&[frontier.target]).await?, frontier.target)?;
        let model = need(&self.read_ids::<analysis::model::AnalysisInvocation>(&[frontier.invocation]).await?, frontier.invocation)?;
        let invocation = need(&self.read_ids::<analysis::summary::AnalysisInvocation>(&[witness.invocation]).await?, witness.invocation)?;
        let claim = need(&self.read_ids::<SummaryClaim>(&[witness.claim]).await?, witness.claim)?;
        data.witnesses.insert(witness.clone())?;
        data.frontiers.insert(frontier.clone())?;
        data.restrictions.insert(edge.clone())?;
        data.targets.insert(target.clone())?;
        data.model_invocations.insert(model.clone())?;
        data.summary_invocations.insert(invocation)?;
        summary.claims.insert(claim)?;
        let source = analysis::summary::SupportSource::TerminalFrontier { witness: id };
        summary.sources.insert(need(&self.read_ids::<analysis::summary::SupportSource>(&[source.id()]).await?, source.id())?)?;
        let subject = analysis::summary::ObligationSubject::SummaryClaim { transfer: witness.claim };
        summary.subjects.insert(need(&self.read_ids::<analysis::summary::ObligationSubject>(&[subject.id()]).await?, subject.id())?)?;
        let propositions = self.read_for::<analysis::summary::AnalysisProposition,analysis::summary::ObligationSubject>("subject", &[subject.id()]).await?;
        let ids = propositions.rows().iter().map(Record::id).collect::<Vec<_>>();
        let derived = self.read_for::<analysis::summary::AnalysisDerivation,analysis::summary::AnalysisProposition>("proposition", &ids).await?;
        let ids = derived.rows().iter().map(Record::id).collect::<Vec<_>>();
        let premises = self.read_for::<analysis::summary::AnalysisDerivationPremise,analysis::summary::AnalysisDerivation>("derivation", &ids).await?;
        for row in propositions.rows() { summary.propositions.insert(row.clone())?; }
        for row in derived.rows() { summary.derivations.insert(row.clone())?; }
        for row in premises.rows() { summary.premises.insert(row.clone())?; }
        data.sets.insert(need(&self.read_ids::<assumptions::AssumptionSet>(&[q.assumptions]).await?, q.assumptions)?)?;
        let members = self.read_for::<assumptions::AssumptionSetMember, assumptions::AssumptionSet>("set", &[q.assumptions]).await?;
        let ids = members.rows().iter().map(|r|r.assumption).collect::<Vec<_>>();
        let definitions = self.read_ids::<assumptions::Assumption>(&ids).await?;
        for row in members.rows(){data.members.insert(row.clone())?;}
        for row in definitions.rows(){data.assumptions.insert(row.clone())?;}
        let ids = [Some(frontier.native),Some(edge.statement_native),Some(edge.following_native),target.target_native,target.final_native,target.member_native].into_iter().flatten().collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        let native = self.read_ids::<analysis::native::NativeAssertionPremise>(&ids).await?;
        let mut citations = Vec::new();
        for row in native.rows() {
            use analysis::native::NativeAssertionPremise as N;
            let proof = match row {
                N::NativeTerminal {assertion,support} => self.terminal_pair::<protocols::NativeTerminalSupport>(*assertion,*support,model.input,model.context).await?,
                N::SyntaxPlacement {assertion,support} => self.terminal_pair::<syntax::SyntaxPlacementSupport>(*assertion,*support,model.input,model.context).await?,
                N::CallTarget {assertion,support} => self.terminal_pair::<calls::CallTargetSupport>(*assertion,*support,model.input,model.context).await?,
                N::ClassMetadataObservation {assertion,support} => self.terminal_pair::<class_metadata::ClassMetadataSupport>(*assertion,*support,model.input,model.context).await?,
                N::ClassMemberObservation {assertion,support} => self.terminal_pair::<class_metadata::ClassMemberSupport>(*assertion,*support,model.input,model.context).await?,
                _ => return Err(Error::Contract),
            };
            citations.extend(proof);
            data.native.insert(row.clone())?;
        }
        if let Some(ancestry) = target.ancestry { need(&self.read_ids::<symbols::ClassAncestryObservation>(&[ancestry]).await?, ancestry)?; }
        for qid in [target.original_qualification,target.receiver_qualification].into_iter().flatten() {
            if need(&self.read_ids::<assertion::AssertionQualification>(&[qid]).await?, qid)?.context != model.context { return Err(Error::Contract); }
        }
        // Read the actual located declaration and source placements; they are citations, not execution evidence.
        need(&self.read_ids::<types::TypeObservation>(&[frontier.declared_return]).await?, frontier.declared_return)?;
        need(&self.read_ids::<syntax::SyntaxPlacement>(&[edge.statement]).await?, edge.statement)?;
        need(&self.read_ids::<syntax::SyntaxPlacement>(&[edge.following]).await?, edge.following)?;
        for occurrence in [frontier.call,frontier.statement,edge.to] { need(&self.read_ids::<source::Occurrence>(&[occurrence]).await?, occurrence)?; }
        let checked = terminal::checked(&data, &summary, id, q, model.input, model.context)?;
        let mut packet = TerminalQuestionPacket::from_canonical(&checked);
        packet.proof.extend(citations);
        packet.proof.sort_by(|a,b|(&a.relation,a.row).cmp(&(&b.relation,b.row)));
        packet.proof.dedup();
        Ok(packet)
    }
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
            .lease
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
            .lease
            .budget
            .reserve("capability-attributed-assertions", packet_bytes)?;
        let mut packets = Vec::new();
        for id in assertions {
            let value = values
                .rows()
                .iter()
                .find(|a| a.id() == id)
                .ok_or(Error::Contract)?;
            let qualification = need(&q, value.qualification())?;
            let claim_basis = self.claim_basis(&qualification).await?;
            let mut claim_supports = Vec::new();
            let mut terminal_question = None;
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
                    AssertionSource::TerminalSummary { witness } => {
                        if terminal_question.is_some()
                            || value.template != (synthesis::assertions::AssertionTemplate::TerminalSummary { witness }).id()
                            || value.kind() != analysis::policy::AssertionKind::ApplicableCase
                            || value.status() != analysis::policy::EvidenceStatus::StructurallyObserved {
                            return Err(Error::Contract);
                        }
                        terminal_question = Some(self.terminal_question(witness, &qualification).await?);
                        derivation::RowRef::of(witness)
                    }
                };
                let mut proof = vec![
                    ProofReference::from_canonical(derivation::RowRef::of(support.id())),
                    ProofReference::from_canonical(derivation::RowRef::of(support.source)),
                    ProofReference::from_canonical(target),
                ];
                if matches!(source, AssertionSource::TerminalSummary { .. }) {
                    proof.extend(terminal_question.as_ref().ok_or(Error::Contract)?.proof.iter().cloned());
                }
                claim_supports.push(AssertionSupportPacket {
                    support: support.id(),
                    role: support.role,
                    source: support.source,
                    proof,
                });
            }
            claim_supports.sort_by_key(|s| s.support);
            packets.push(AssertionPacket {
                assertion: id,
                kind: value.kind(),
                section: value.section(),
                status: value.status(),
                qualification: value.qualification(),
                claim_basis,
                terminal_question: Nullable(terminal_question),
                text: Text::new(value.text()).map_err(wire)?,
                supports: claim_supports,
            });
        }
        Ok(((brief, rendered), packets, originals))
    }
}
