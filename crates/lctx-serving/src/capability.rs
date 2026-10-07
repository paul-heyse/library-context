//! Existing authored briefs and fully attributed assertions; no synthesis runs in serving.
use crate::{
    claims::Claims,
    records::{rows, wire},
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
    let data = hydrate(reader, &[id], budget).await?;
    Prepared::new(&data, budget)?.packet(id, budget)
}
/// Hydrate one finite union of nominated briefs, with the same canonical owned closure as get.
pub async fn hydrate(
    reader: &NativeReader,
    ids: &[Id<synthesis::briefs::Brief>],
    budget: &ResourceBudget,
) -> Result<CanonicalBatches, ModelError> {
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
    crate::scope::hydrate_with(reader, roots, &inputs, &fields, budget).await
}
type Index<R> = std::collections::BTreeMap<Id<R>, R>;
fn index<R: Record>(data: &CanonicalBatches) -> Result<Index<R>, ModelError> {
    let mut result = Index::new();
    for row in rows::<R>(data)? {
        if result.insert(row.id(), row).is_some() {
            return Err(ModelError::Conflict("capability companion identity"));
        }
    }
    Ok(result)
}
fn need<R: Record>(rows: &Index<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(&id)
        .ok_or(ModelError::Schema("capability companion"))
}
/// Decode claim and packet companions once per bounded union, then lower individual briefs.
pub struct Prepared {
    briefs: Index<synthesis::briefs::Brief>,
    documents: Index<synthesis::briefs::BriefDocument>,
    claims: Claims,
    assertions: Index<synthesis::assertions::ProgrammaticAssertion>,
    supports: Index<synthesis::assertions::ProgrammaticAssertionSupport>,
    sources: Index<synthesis::assertions::AssertionSource>,
    invocations: Index<catalog::CatalogMemberInvocation>,
    members: Index<catalog::CatalogMember>,
    links: Index<synthesis::briefs::BriefAssertion>,
    terminal: synthesis::terminal::Data,
    summary: synthesis::summary::Data,
    documentary: Index<synthesis::documentary::DocumentaryConclusion>,
    original_links: Vec<synthesis::briefs::BriefSource>,
    originals: crate::originals::Prepared,
}
impl Prepared {
    pub fn new(data: &CanonicalBatches, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut terminal = synthesis::terminal::Data::new(budget);
        let mut summary = synthesis::summary::Data::new(budget);
        for (name, batch) in &data.batches {
            terminal.visit(name, batch)?;
            summary.visit(name, batch)?;
        }
        Ok(Self {
            briefs: index(data)?,
            documents: index(data)?,
            claims: Claims::new(data, budget)?,
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
        let brief = need(&briefs, id)?;
        let mut documents = self
            .documents
            .values()
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
        let claims = &self.claims;
        let assertions = &self.assertions;
        let supports = &self.supports;
        let sources = &self.sources;
        let member_invocations = &self.invocations;
        let members = &self.members;
        let mut links = self
            .links
            .values()
            .filter(|a| a.brief == id)
            .collect::<Vec<_>>();
        links.sort_by_key(|a| a.ordinal);
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
            for support in supports.values().filter(|s| s.assertion == assertion.id()) {
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
        for source in self.original_links.iter().filter(|s| s.brief == id) {
            let doc = need(&docs, source.documentary)?;
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
        use lctx_surrealdb::{Credentials, Loader, reader};
        use synthesis::briefs::{Brief, BriefDocument, RENDERING_VERSION, ReviewStatus};
        let cfg: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                std::env::var("LCTX_SURREAL_TEST_CONFIG")
                    .expect("owned disposable native fixture required"),
            )
            .unwrap(),
        )
        .unwrap();
        let db = format!("capability_union_{}", std::process::id());
        let ns = "gn_capability_controls";
        let client = reader::connect(
            cfg["grpc_endpoint"].as_str().unwrap(),
            &Credentials::Root {
                username: cfg["admin_user"].as_str().unwrap().into(),
                password: cfg["admin_password"].as_str().unwrap().into(),
            },
            ns,
            &db,
        )
        .await
        .unwrap();
        client
            .query(format!(
                "DEFINE NAMESPACE IF NOT EXISTS {ns}; DEFINE DATABASE OVERWRITE {db} STRICT;"
            ))
            .await
            .unwrap()
            .check()
            .unwrap();
        let loader = Loader::new(client.clone());
        loader.install(&crate::native_definitions()).await.unwrap();
        let mut assertions = Vec::new();
        let mut documents = Vec::new();
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
            documents.push(document);
            briefs.push((brief, text));
        }
        loader.assertions(&assertions).await.unwrap();
        // Brief seeds are intentionally absent in this packet-only cohort fixture.
        // Retain the real document-to-brief ownership edges used by union hydration.
        loader.assertion_references(&documents).await.unwrap();
        let handle = SnapshotHandle {
            semantic: ContentHash::of(b"partial brief fixture"),
            realization: lctx_surrealdb::schema::realization_identity(&crate::native_definitions()),
            database: DatabaseIdentity {
                namespace: Name::new(ns).unwrap(),
                database: Name::new(&db).unwrap(),
            },
        };
        let native = NativeReader::new(client.clone(), handle);
        let budget = ResourceBudget::fixed(32 * 1024 * 1024).unwrap();
        let data = hydrate(
            &native,
            &briefs
                .iter()
                .map(|(brief, _)| brief.id())
                .collect::<Vec<_>>(),
            &budget,
        )
        .await
        .unwrap();
        let prepared = Prepared::new(&data, &budget).unwrap();
        for (brief, text) in &briefs {
            let packet = prepared.packet(brief.id(), &budget).unwrap();
            assert_eq!(packet.rendered.as_str(), *text);
            assert!(packet.assertions.is_empty());
            assert_eq!(packet.capability, brief.id());
        }
        // A nominated cohort's part cannot be substituted for another brief's rendering.
        let mut changed = prepared;
        let first = changed
            .documents
            .values_mut()
            .find(|part| part.brief == briefs[0].0.id())
            .unwrap();
        first.text = "rewritten".into();
        assert!(changed.packet(briefs[0].0.id(), &budget).is_err());
        assert!(changed.packet(briefs[1].0.id(), &budget).is_ok());
        client
            .query(format!("REMOVE DATABASE {db}"))
            .await
            .unwrap()
            .check()
            .unwrap();
    }
}
