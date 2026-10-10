//! Existing authored briefs and fully attributed assertions; no synthesis runs in serving.
use crate::{
    claims::Claims,
    records::{PacketRows, Prepared as CanonicalPrepared, rows, wire},
};
use lctx_model::domain::{
    resources::ResourceBudget, serving::mappings::PacketOutput, serving::*, *,
};
use lctx_surrealdb::{NativeReader, reader::target_id};
pub async fn get(
    reader: &NativeReader,
    preparation: &crate::preparation::Preparation<'_>,
    id: Id<synthesis::briefs::Brief>,
    budget: &ResourceBudget,
) -> Result<CapabilityPacket, ModelError> {
    let data = hydrate(reader, preparation, &[id], budget).await?;
    let data = CanonicalPrepared::new(&data, budget);
    Prepared::new(&data, budget)?.packet(id, budget)
}
/// Hydrate one finite union of nominated briefs, with the same canonical owned closure as get.
pub async fn hydrate(
    reader: &NativeReader,
    preparation: &crate::preparation::Preparation<'_>,
    ids: &[Id<synthesis::briefs::Brief>],
    _budget: &ResourceBudget,
) -> Result<crate::preparation::PreparedBatches, ModelError> {
    let inputs = CapabilityPacket::binding()
        .lowered()
        .sources
        .iter()
        .map(|r| ValidationInput::of_relation(r, &["id"]))
        .collect::<Vec<_>>();
    let mut fields = crate::scope::OWNED_FIELDS.to_vec();
    fields.extend(["brief", "claim", "derivation"]);
    let roots = ids
        .iter()
        .copied()
        .map(|id| graph::target_for_row(derivation::RowRef::of(id)).map(target_id))
        .collect::<Result<Vec<_>, _>>()?;
    preparation
        .hydrate(reader, roots, &inputs, &inputs, &fields)
        .await
}
type Index<R> = PacketRows<R>;
fn index<R: Record>(data: &CanonicalPrepared<'_>) -> Result<Index<R>, ModelError> {
    let result = rows::<R>(data)?;
    result.require_unique()?;
    Ok(result)
}
fn need<R: Record>(rows: &Index<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)?
        .ok_or(ModelError::Schema("capability companion"))
}
/// Decode claim and packet companions once per bounded union, then lower individual briefs.
pub struct Prepared {
    briefs: Index<synthesis::briefs::Brief>,
    documents: Index<synthesis::briefs::BriefDocument>,
    claims: std::sync::Arc<Claims>,
    assertions: Index<synthesis::assertions::ProgrammaticAssertion>,
    supports: Index<synthesis::assertions::ProgrammaticAssertionSupport>,
    sources: Index<synthesis::assertions::AssertionSource>,
    invocations: Index<catalog::CatalogMemberInvocation>,
    members: Index<catalog::CatalogMember>,
    links: Index<synthesis::briefs::BriefAssertion>,
    terminal: synthesis::terminal::Data,
    summary: synthesis::summary::Data,
    documentary: Index<synthesis::documentary::DocumentaryConclusion>,
    original_links: PacketRows<synthesis::briefs::BriefSource>,
    originals: crate::originals::Prepared,
}
impl Prepared {
    pub fn new(data: &CanonicalPrepared<'_>, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut terminal = synthesis::terminal::Data::new(budget);
        let mut summary = synthesis::summary::Data::new(budget);
        for (name, batch) in &data.batches {
            terminal.visit(name, batch)?;
            summary.visit(name, batch)?;
        }
        Ok(Self {
            briefs: index(data)?,
            documents: index(data)?,
            claims: data.claims()?,
            assertions: index(data)?,
            supports: index(data)?,
            sources: index(data)?,
            invocations: index(data)?,
            members: index(data)?,
            links: index(data)?,
            terminal,
            summary,
            documentary: index(data)?,
            original_links: rows(data)?,
            originals: crate::originals::Prepared::new(data)?,
        })
    }
    pub fn packet(
        &self,
        id: Id<synthesis::briefs::Brief>,
        budget: &ResourceBudget,
    ) -> Result<CapabilityPacket, ModelError> {
        let briefs = &self.briefs;
        let brief = need(briefs, id)?;
        let selected_documents = self.documents.select_for("brief", &[id])?;
        let mut documents = selected_documents.iter().collect::<Vec<_>>();
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
        let claims = &self.claims;
        let assertions = &self.assertions;
        let supports = &self.supports;
        let sources = &self.sources;
        let member_invocations = &self.invocations;
        let members = &self.members;
        let selected_links = self.links.select_for("brief", &[id])?;
        let mut links = selected_links.iter().collect::<Vec<_>>();
        links.sort_by_key(|a| a.ordinal);
        let mut packets = Vec::new();
        for (ordinal, link) in links.iter().enumerate() {
            if link.ordinal != ordinal as i64 {
                return Err(ModelError::Schema("brief assertion ordinal"));
            }
            let assertion = need(assertions, link.assertion)?;
            let q = claims
                .qualifications
                .get(&assertion.qualification())
                .ok_or(ModelError::Schema("brief assertion qualification"))?;
            let input = need(members, need(member_invocations, assertion.member)?.member)?.input;
            let mut attribution = Vec::new();
            let mut terminal_packet = None;
            for support in &supports.select_for("assertion", &[assertion.id()])? {
                let source = need(sources, support.source)?;
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
                            &self.terminal,
                            &self.summary,
                            *witness,
                            q,
                            input,
                            q.context,
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
        let docs = &self.documentary;
        let mut originals = Vec::new();
        for source in &self.original_links.select_for("brief", &[id])? {
            let doc = need(docs, source.documentary)?;
            let q = claims
                .qualifications
                .get(&doc.qualification())
                .ok_or(ModelError::Schema("documentary qualification"))?;
            originals.push(self.originals.range(
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
}
#[cfg(test)]
mod controls {
    use super::*;
    #[tokio::test]
    async fn union_preparation_keeps_each_briefs_documents_separate() {
        use synthesis::briefs::{Brief, BriefDocument, RENDERING_VERSION, ReviewStatus};
        let config = crate::scoped_fixture::config();
        let mut assertions = Vec::new();
        let mut briefs = Vec::new();
        for (byte, text) in [(1u8, "first brief"), (2, "second independent brief")] {
            let brief = Brief {
                seed: serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap(),
                version: RENDERING_VERSION,
                assertions: ContentHash::of(b"empty assertions"),
                originals: ContentHash::of(b"empty originals"),
                title: text.into(),
                rendered: ContentHash::of(text.as_bytes()),
                bytes: text.len() as i64,
                review: ReviewStatus::Unreviewed,
                documentation_only: true,
            };
            assertions.push(graph::Assertion::from_record(brief.clone()).unwrap());
            let document = graph::Assertion::from_record(BriefDocument {
                brief: brief.id(),
                ordinal: 0,
                text: text.into(),
            })
            .unwrap();
            assertions.push(document.clone());
            briefs.push((brief, text));
        }
        let native = crate::scoped_fixture::reader(&config, &[], &assertions)
            .await
            .unwrap();
        let budget = ResourceBudget::fixed(32 * 1024 * 1024).unwrap();
        let limits = ResourceLimits::default();
        let queries = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let cpu = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let cache = crate::preparation::PreparedCache::for_scope(
            &native.reader,
            &budget,
            &limits,
            queries.clone(),
            cpu.clone(),
        )
        .unwrap();
        let admission = crate::preparation::RequestAdmission::new(
            queries,
            cpu,
            tokio::time::Instant::now() + std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(1),
        )
        .await
        .unwrap();
        let preparation = crate::preparation::Preparation {
            cache: &cache,
            admission: &admission,
        };
        {
            let inputs = CapabilityPacket::binding()
                .lowered()
                .sources
                .iter()
                .map(|relation| ValidationInput::of_relation(relation, &["id"]))
                .collect::<Vec<_>>();
            let mut fields = crate::scope::OWNED_FIELDS.to_vec();
            fields.extend(["brief", "claim", "derivation"]);
            let roots = briefs
                .iter()
                .map(|(brief, _)| {
                    graph::target_for_row(derivation::RowRef::of(brief.id())).map(target_id)
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let data = preparation
                .hydrate_scope(&native.reader, roots.clone(), &inputs, &inputs, &fields)
                .await
                .unwrap();
            let repeated = preparation
                .hydrate_scope(&native.reader, roots, &inputs, &inputs, &fields)
                .await
                .unwrap();
            assert!(
                data.shares_value(&repeated),
                "same pinned semantic closure must skip native hydration"
            );
            drop(repeated);
            let data = CanonicalPrepared::new(&data, &budget);
            let prepared = Prepared::new(&data, &budget).unwrap();
            for (brief, text) in &briefs {
                let packet = prepared.packet(brief.id(), &budget).unwrap();
                assert_eq!(packet.rendered.as_str(), *text);
                assert!(packet.assertions.is_empty());
                assert_eq!(packet.capability, brief.id());
            }
            // A nominated cohort's part cannot be substituted for another brief's rendering.
            let mut changed = prepared;
            let mut documents = changed.documents.rows().to_vec();
            let first = documents
                .iter_mut()
                .find(|part| part.brief == briefs[0].0.id())
                .unwrap();
            first.text = "rewritten".into();
            changed.documents = PacketRows::new(documents, &budget).unwrap();
            assert!(changed.packet(briefs[0].0.id(), &budget).is_err());
            assert!(changed.packet(briefs[1].0.id(), &budget).is_ok());
        }
        cache.close().await;
        assert_eq!(
            budget.reserved(),
            0,
            "closed viewer releases preparation after all borrowers"
        );
        native.close().await.unwrap();
    }
}
