//! Existing authored briefs and fully attributed assertions; no synthesis runs in serving.
use crate::{
    claims::Claims,
    records::{need, rows, wire},
};
use lctx_model::domain::{
    resources::ResourceBudget, serving::mappings::PacketOutput, serving::*, *,
};
use lctx_surrealdb::{NativeReader, batches::CanonicalBatches, reader::target_id};
pub async fn get(
    reader: &NativeReader,
    id: Id<synthesis::briefs::Brief>,
    budget: &ResourceBudget,
) -> Result<CapabilityPacket, ModelError> {
    let inputs = CapabilityPacket::binding()
        .lowered()
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect::<Vec<_>>();
    let mut fields = crate::scope::OWNED_FIELDS.to_vec();
    fields.extend(["brief", "claim", "derivation"]);
    let data = crate::scope::hydrate_with(
        reader,
        vec![target_id(graph::target_for_row(derivation::RowRef::of(
            id,
        ))?)],
        &inputs,
        &fields,
        budget,
    )
    .await?;
    packet(&data, id, budget)
}
pub fn packet(
    data: &CanonicalBatches,
    id: Id<synthesis::briefs::Brief>,
    budget: &ResourceBudget,
) -> Result<CapabilityPacket, ModelError> {
    let briefs = rows::<synthesis::briefs::Brief>(data)?;
    let brief = need(&briefs, id)?;
    let mut documents = rows::<synthesis::briefs::BriefDocument>(data)?
        .into_iter()
        .filter(|p| p.brief == id)
        .collect::<Vec<_>>();
    documents.sort_by_key(|p| p.ordinal);
    let _bytes = budget.reserve(
        "native-capability-rendered",
        usize::try_from(brief.bytes)
            .map_err(ModelError::codec)?
            .saturating_mul(2),
    )?;
    let mut rendered = String::new();
    for (ordinal, part) in documents.iter().enumerate() {
        if part.ordinal != ordinal as i64 {
            return Err(ModelError::Schema("brief part ordinal"));
        }
        rendered.push_str(part.text.as_str());
    }
    if rendered.len() as i64 != brief.bytes
        || ContentHash::of(rendered.as_bytes()) != brief.rendered
    {
        return Err(ModelError::Identity("brief rendered bytes"));
    }
    let claims = Claims::new(data, budget)?;
    let assertions = rows::<synthesis::assertions::ProgrammaticAssertion>(data)?;
    let supports = rows::<synthesis::assertions::ProgrammaticAssertionSupport>(data)?;
    let sources = rows::<synthesis::assertions::AssertionSource>(data)?;
    let member_invocations = rows::<catalog::CatalogMemberInvocation>(data)?;
    let members = rows::<catalog::CatalogMember>(data)?;
    let mut links = rows::<synthesis::briefs::BriefAssertion>(data)?
        .into_iter()
        .filter(|a| a.brief == id)
        .collect::<Vec<_>>();
    links.sort_by_key(|a| a.ordinal);
    let mut terminal = synthesis::terminal::Data::new(budget);
    let mut summary = synthesis::summary::Data::new(budget);
    for (name, batch) in &data.batches {
        terminal.visit(name, batch)?;
        summary.visit(name, batch)?;
    }
    let mut packets = Vec::new();
    for (ordinal, link) in links.iter().enumerate() {
        if link.ordinal != ordinal as i64 {
            return Err(ModelError::Schema("brief assertion ordinal"));
        }
        let assertion = need(&assertions, link.assertion)?;
        let q = claims
            .qualifications
            .get(&assertion.qualification())
            .ok_or(ModelError::Schema("brief assertion qualification"))?;
        let input = need(
            &members,
            need(&member_invocations, assertion.member)?.member,
        )?
        .input;
        let mut attribution = Vec::new();
        let mut terminal_packet = None;
        for support in supports.iter().filter(|s| s.assertion == assertion.id()) {
            let source = need(&sources, support.source)?;
            let target = match source {
                synthesis::assertions::AssertionSource::Documentary { conclusion } => {
                    derivation::RowRef::of(*conclusion)
                }
                synthesis::assertions::AssertionSource::Structural { conclusion } => {
                    derivation::RowRef::of(*conclusion)
                }
                synthesis::assertions::AssertionSource::Analytic { conclusion } => {
                    derivation::RowRef::of(*conclusion)
                }
                synthesis::assertions::AssertionSource::Summary { facet } => {
                    derivation::RowRef::of(*facet)
                }
                synthesis::assertions::AssertionSource::AuthoredCode { conclusion } => {
                    derivation::RowRef::of(*conclusion)
                }
                synthesis::assertions::AssertionSource::TerminalSummary { witness } => {
                    let checked = synthesis::terminal::checked(
                        &terminal, &summary, *witness, q, input, q.context,
                    )?;
                    let packet = TerminalQuestionPacket::from_canonical(&checked)?;
                    if terminal_packet
                        .as_ref()
                        .is_some_and(|existing| existing != &packet)
                    {
                        return Err(ModelError::Conflict("brief terminal question ambiguity"));
                    }
                    terminal_packet = Some(packet);
                    derivation::RowRef::of(*witness)
                }
            };
            attribution.push(AssertionSupportPacket {
                support: support.id(),
                role: support.role,
                source: support.source,
                proof: vec![
                    ProofReference::from_canonical(derivation::RowRef::of(support.id()))?,
                    ProofReference::from_canonical(target)?,
                ],
            });
        }
        attribution.sort_by_key(|s| s.support);
        packets.push(AssertionPacket {
            assertion: assertion.id(),
            kind: assertion.kind(),
            section: assertion.section(),
            status: assertion.status(),
            qualification: assertion.qualification(),
            claim_basis: claims.basis(assertion.qualification())?,
            terminal_question: Nullable(terminal_packet),
            text: Text::new(assertion.text()).map_err(wire)?,
            supports: attribution,
        });
    }
    let docs = rows::<synthesis::documentary::DocumentaryConclusion>(data)?;
    let mut originals = Vec::new();
    for source in rows::<synthesis::briefs::BriefSource>(data)?
        .iter()
        .filter(|s| s.brief == id)
    {
        let doc = need(&docs, source.documentary)?;
        let q = claims
            .qualifications
            .get(&doc.qualification())
            .ok_or(ModelError::Schema("documentary qualification"))?;
        originals.push(crate::originals::range(
            data,
            &OriginalReference::Prose { slice: doc.prose },
            Some(q.context),
            None,
        )?);
    }
    Ok(CapabilityPacket {
        capability: id,
        title: Name::new(brief.title.as_str()).map_err(wire)?,
        rendered: Text::new(rendered).map_err(wire)?,
        assertions: packets,
        originals,
        availability: Availability::Available {},
        unreviewed: true,
        documentation_only: brief.documentation_only,
    })
}
