//! Original captured bytes and bounded, relation-qualified proof expansion.
use super::catalog_service::retain;
use super::{Error, GenerationService, RequestExecution, qualified};
use arrow_array::{Array, FixedSizeBinaryArray};
use lctx_model::domain::{
    artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunk, ArtifactVerifier},
    assertion::Evidence,
    attribution::{AnalysisContext, ProviderRun},
    catalog::evidence::OriginalSource,
    input::{
        ArtifactOwnership, CorpusLibrary, DistributionRole, DistributionVerification,
        InputDistribution,
    },
    serving::*,
    source::{Occurrence, SourceArtifact},
    *,
};
use std::collections::{BTreeMap, BTreeSet};

fn wire(error: WireError) -> Error {
    Error::Codec(error.to_string())
}
fn required<R: Record>(batch: &Batch<R>, id: Id<R>) -> Result<R, Error> {
    batch
        .rows()
        .iter()
        .find(|r| r.id() == id)
        .cloned()
        .ok_or(Error::Contract)
}
fn unique<T: Copy + Ord>(values: impl IntoIterator<Item = T>) -> Result<T, Error> {
    let values: BTreeSet<_> = values.into_iter().collect();
    if values.len() != 1 {
        return Err(Error::Contract);
    }
    Ok(*values.first().ok_or(Error::Contract)?)
}
impl super::packet_reads::PacketLease<'_> {
    async fn resolve_original(
        &mut self,
        source: OriginalReference,
        context: Option<Id<AnalysisContext>>,
    ) -> Result<OriginalRange, Error> {
        let address = source.clone();
        let mut current = source;
        let mut relative = None;
        let (artifact, mut start, mut end) = loop {
            match current {
                OriginalReference::Catalog { source } => {
                    let original =
                        required(&self.read_ids::<OriginalSource>(&[source]).await?, source)?;
                    current = match original {
                        OriginalSource::Artifact { artifact } => {
                            OriginalReference::Artifact { artifact }
                        }
                        OriginalSource::Occurrence { occurrence } => {
                            OriginalReference::Occurrence { occurrence }
                        }
                        OriginalSource::Span { span } => OriginalReference::Span { span },
                    };
                }
                OriginalReference::Anchor { anchor } => {
                    let row = required(
                        &self
                            .read_ids::<retrieval::OriginalAnchor>(&[anchor])
                            .await?,
                        anchor,
                    )?;
                    let original = required(
                        &self
                            .read_ids::<retrieval::AnchorSource>(&[row.original])
                            .await?,
                        row.original,
                    )?;
                    current = match original {
                        retrieval::AnchorSource::Original { source } => {
                            OriginalReference::Catalog { source }
                        }
                        retrieval::AnchorSource::Span { span } => OriginalReference::Span { span },
                        retrieval::AnchorSource::Artifact { artifact } => {
                            OriginalReference::Artifact { artifact }
                        }
                        retrieval::AnchorSource::Prose { slice } => {
                            OriginalReference::Prose { slice }
                        }
                    };
                }
                OriginalReference::Prose { slice } => {
                    let row = required(
                        &self
                            .read_ids::<synthesis::documentary::ProseSlice>(&[slice])
                            .await?,
                        slice,
                    )?;
                    let prose = required(
                        &self
                            .read_ids::<synthesis::documentary::ProseSource>(&[row.source])
                            .await?,
                        row.source,
                    )?;
                    current = match prose {
                        synthesis::documentary::ProseSource::Occurrence { occurrence } => {
                            relative = Some((row.start, row.end));
                            OriginalReference::Occurrence { occurrence }
                        }
                        synthesis::documentary::ProseSource::Span { span } => {
                            relative = Some((row.start, row.end));
                            OriginalReference::Span { span }
                        }
                        // Decoded literal offsets are not coordinates in a Python source expression.
                        // Expand its exact original occurrence, including its syntax and quoting.
                        synthesis::documentary::ProseSource::Literal { occurrence, .. } => {
                            OriginalReference::Occurrence { occurrence }
                        }
                    };
                }
                OriginalReference::Artifact { artifact } => {
                    let row = required(
                        &self.read_ids::<SourceArtifact>(&[artifact]).await?,
                        artifact,
                    )?;
                    break (artifact, 0, row.byte_len);
                }
                OriginalReference::Occurrence { occurrence } => {
                    let row = required(
                        &self.read_ids::<Occurrence>(&[occurrence]).await?,
                        occurrence,
                    )?;
                    break (row.source, row.start, row.end);
                }
                OriginalReference::Span { span } => {
                    match required(&self.read_ids::<Evidence>(&[span.id()]).await?, span.id())? {
                        Evidence::SourceSpan { source, start, end } => break (source, start, end),
                        _ => return Err(Error::Contract),
                    }
                }
            }
        };
        if let Some((lo, hi)) = relative {
            if lo < 0 || hi < lo || hi > end - start {
                return Err(Error::Contract);
            }
            end = start + hi;
            start += lo;
        }
        let row = required(
            &self.read_ids::<SourceArtifact>(&[artifact]).await?,
            artifact,
        )?;
        let ownership = self
            .read_for::<ArtifactOwnership, SourceArtifact>("artifact", &[artifact])
            .await?;
        let release = if ownership.rows().is_empty() {
            let links = self
                .read_for::<CorpusLibrary, input::InputRevision>("corpus", &[row.input])
                .await?;
            let mut inputs = vec![row.input];
            inputs.extend(links.rows().iter().map(|r| r.library));
            let distributions = self
                .read_for::<InputDistribution, input::InputRevision>("input", &inputs)
                .await?;
            unique(
                distributions
                    .rows()
                    .iter()
                    .filter(|d| d.role == DistributionRole::FirstParty)
                    .map(|d| d.release),
            )?
        } else {
            let ids: Vec<_> = ownership.rows().iter().map(|r| r.distribution).collect();
            unique(
                self.read_ids::<DistributionVerification>(&ids)
                    .await?
                    .rows()
                    .iter()
                    .map(|r| r.release),
            )?
        };
        let runs = self
            .read_for::<ProviderRun, input::InputRevision>("input", &[row.input])
            .await?;
        let context = match context {
            Some(c) if runs.rows().iter().any(|r| r.context == c) => c,
            Some(_) => return Err(Error::Contract),
            None => unique(runs.rows().iter().map(|r| r.context))?,
        };
        if start < 0 || end < start || end > row.byte_len {
            return Err(Error::Contract);
        }
        Ok(OriginalRange {
            source: address,
            artifact,
            start: start as u64,
            end: end as u64,
            digest: row.content,
            encoding: Name::new("raw_bytes").map_err(wire)?,
            release,
            context,
        })
    }
}
impl GenerationService {
    pub async fn original_range(
        &self,
        execution: &RequestExecution,
        source: OriginalReference,
        context: Option<Id<AnalysisContext>>,
    ) -> Result<OriginalRange, Error> {
        if !execution.shares_guard(&self.guard()) {
            return Err(Error::State);
        }
        let result = execution
            .query(move |lease| {
                Box::pin(async move {
                    let mut scope = super::packet_reads::PacketLease::new::<OriginalRange>(lease);
                    let lease = &mut scope;
                    lease.resolve_original(source, context).await
                })
            })
            .await?;
        retain(execution, &result)?;
        Ok(result)
    }
    pub async fn evidence(
        &self,
        execution: &RequestExecution,
        request: &GetEvidenceRequest,
    ) -> Result<GetEvidenceResponse, Error> {
        if !execution.shares_guard(&self.guard()) {
            return Err(Error::State);
        }
        let original = self
            .original_range(execution, request.source.clone(), None)
            .await?;
        let binding = CursorBinding {
            generation: GenerationKey(*self.generation().bytes()),
            request: Request::GetEvidence(request.clone())
                .canonical_identity()
                .map_err(wire)?,
            policy: ranking::RankingPolicy::default().identity()?,
            wire: wire_identity(),
            channels: ChannelState {
                lexical: false,
                vector: VectorChannel::Disabled {},
            }
            .identity(),
            group: Name::new("evidence").map_err(wire)?,
            section: Name::new("bytes").map_err(wire)?,
            member: None,
            ordering: ContentHash::of(b"original-byte-offset/v1"),
        };
        let offset = request
            .page
            .cursor
            .0
            .as_ref()
            .map(|c| Cursor::decode(c, &binding).map(|c| c.offset))
            .transpose()
            .map_err(wire)?
            .unwrap_or(0);
        let length = original.end - original.start;
        if offset > length {
            return Err(Error::Contract);
        }
        // JSON byte arrays need up to four bytes per octet. Leave room for provenance, proof
        // metadata and the final MCP envelope; final admission remains the transport's check.
        let maximum = if request.page.expanded { 32_768 } else { 4_096 };
        let page_start = original.start + offset;
        let page_end = original.end.min(page_start + maximum);
        let captured = original.clone();
        let retained = execution.clone();
        let bytes = execution
            .query(move |lease| {
                Box::pin(async move {
                    let mut scope = super::packet_reads::PacketLease::new::<EvidencePacket>(lease);
                    let lease = &mut scope;
                    let artifact = required(
                        &lease
                            .read_ids::<SourceArtifact>(&[captured.artifact])
                            .await?,
                        captured.artifact,
                    )?;
                    let mut verifier = ArtifactVerifier::new(&artifact)?;
                    let mut body = Vec::new();
                    let _charge = lease
                        .lease
                        .budget
                        .reserve("original-byte-page", (page_end - page_start) as usize)?;
                    lease
                        .visit_for::<ArtifactChunk, SourceArtifact>(
                            "artifact",
                            &[captured.artifact],
                            &["ordinal"],
                            |batch| {
                                for chunk in batch.rows() {
                                    verifier.push(chunk)?;
                                    let start = chunk.ordinal as u64 * ARTIFACT_CHUNK_BYTES as u64;
                                    let lo = page_start.max(start);
                                    let hi = page_end.min(start + chunk.body.0.len() as u64);
                                    if lo < hi {
                                        body.extend_from_slice(
                                            &chunk.body.0
                                                [(lo - start) as usize..(hi - start) as usize],
                                        );
                                    }
                                }
                                Ok(())
                            },
                        )
                        .await?;
                    verifier.finish()?;
                    if body.len() as u64 != page_end - page_start {
                        return Err(Error::Contract);
                    }
                    retained.retain("original-byte-page-output", body.len())?;
                    Ok(body)
                })
            })
            .await?;
        let flow_grant=original.clone();
        let flow_execution=execution.clone();
        let maximum_inventories=if request.page.expanded {64}else{16};
        let flow_inventory=execution.query(move |lease|Box::pin(async move {
            let mut packet=super::packet_reads::PacketLease::new::<EvidencePacket>(lease);
            packet.flow_inventory(&flow_grant,maximum_inventories,&flow_execution).await
        })).await?;
        let granted = original.clone();
        let maximum = if request.page.expanded { 64 } else { 16 };
        let source_characterization = execution
            .query(move |lease| {
                Box::pin(async move {
                    let mut packet = super::packet_reads::PacketLease::new::<EvidencePacket>(lease);
                    packet.source_characterization(&granted, maximum).await
                })
            })
            .await?;
        retain(execution, &source_characterization)?;
        let continuation = if page_end < original.end {
            Optional::supplied(
                Cursor {
                    binding,
                    offset: page_end - original.start,
                }
                .encode()
                .map_err(wire)?,
            )
        } else {
            Optional::default()
        };
        let root = match &request.source {
            OriginalReference::Catalog { source } => derivation::RowRef::of(*source),
            OriginalReference::Anchor { anchor } => derivation::RowRef::of(*anchor),
            OriginalReference::Prose { slice } => derivation::RowRef::of(*slice),
            OriginalReference::Artifact { artifact } => derivation::RowRef::of(*artifact),
            OriginalReference::Occurrence { occurrence } => derivation::RowRef::of(*occurrence),
            OriginalReference::Span { span } => derivation::RowRef::of(span.id()),
        };
        let derivation = self
            .explanation(execution, ProofReference::from_canonical(root))
            .await?;
        let omitted = original.end - page_end;
        execution.confirm().await?;
        let response = GetEvidenceResponse {
            generation: GenerationKey(*self.generation().bytes()),
            evidence: EvidencePacket {
                original,
                source_characterization,
                flow_inventory,
                body: EvidenceBodyPage {
                    start: page_start,
                    end: page_end,
                    bytes,
                    continuation,
                    omitted,
                    truncated: omitted > 0,
                },
                status: analysis::policy::EvidenceStatus::StructurallyObserved,
                derivation,
            },
        };
        let expanded = request.page.expanded;
        let retained = execution.clone();
        execution
            .cpu(move |budget| {
                let _encoding = budget.reserve(
                    "evidence-response-encoding",
                    ResourceLimits::default().expanded_response_bytes as usize * 8,
                )?;
                let bound = ResourceLimits::default().response_bytes(expanded) as usize;
                let mut response = Response::GetEvidence(response);
                loop {
                    if response.json_len().map_err(wire)? <= bound {
                        let raw = response.to_json().map_err(wire)?;
                        match tool_result("get_evidence", &raw, expanded) {
                            Ok(_) => {
                                let Response::GetEvidence(response) = response else {
                                    return Err(Error::Contract);
                                };
                                retain(&retained, &response)?;
                                return Ok(response);
                            }
                            Err(WireError::ResourceRefused(_)) => {}
                            Err(error) => return Err(wire(error)),
                        }
                    }
                    let Response::GetEvidence(value) = &mut response else {
                        return Err(Error::Contract);
                    };
                    if value.evidence.flow_inventory.items.pop().is_some() {
                        let section=&mut value.evidence.flow_inventory;
                        section.omitted=section.omitted.checked_add(1).ok_or(Error::Contract)?;
                        section.truncated=true;
                        section.availability=Availability::Partial {reason:Name::new("flow inventory byte bound reached").map_err(wire)?};
                        continue;
                    }
                    if value.evidence.source_characterization.items.pop().is_some() {
                        let section = &mut value.evidence.source_characterization;
                        section.omitted = section.omitted.checked_add(1).ok_or(Error::Contract)?;
                        section.truncated = true;
                        section.availability = Availability::Partial {
                            reason: Name::new("source characterization byte bound reached")
                                .map_err(wire)?,
                        };
                        continue;
                    }
                    let explanation = &mut value.evidence.derivation;
                    if explanation.items.pop().is_none() {
                        return Err(Error::ResourceRefused("indivisible original evidence page"));
                    }
                    explanation.omitted =
                        explanation.omitted.checked_add(1).ok_or(Error::Contract)?;
                    explanation.truncated = true;
                    explanation.availability = Availability::Partial {
                        reason: Name::new(
                            "explanation byte bound reached; remaining count may be unknown",
                        )
                        .map_err(wire)?,
                    };
                }
            })
            .await
    }
    pub async fn explanation(
        &self,
        execution: &RequestExecution,
        root: ProofReference,
    ) -> Result<SectionPage<DerivationStep>, Error> {
        if !execution.shares_guard(&self.guard()) {
            return Err(Error::State);
        }
        let retained = execution.clone();
        execution
            .query(move |lease| {
                Box::pin(async move {
                    let mut scope = super::packet_reads::PacketLease::new::<EvidencePacket>(lease);
                    let lease = &mut scope;
                    lease.explain(root, &retained).await
                })
            })
            .await
    }
}
fn reference(relation: String, id: Vec<u8>) -> Result<ProofReference, Error> {
    Ok(ProofReference {
        relation: Name::new(relation).map_err(wire)?,
        row: id.try_into().map_err(|_| Error::Contract)?,
    })
}
impl super::packet_reads::PacketLease<'_> {
    async fn verify_proof_rows(
        &mut self,
        refs: &BTreeSet<(String, [u8; 16])>,
    ) -> Result<(), Error> {
        let mut groups: BTreeMap<&str, Vec<Vec<u8>>> = BTreeMap::new();
        for (relation, id) in refs {
            if !self.lease.relations.contains(relation.as_str())
                || !self
                    .lease
                    .model
                    .relations()
                    .iter()
                    .any(|r| r.name() == relation)
            {
                return Err(Error::Contract);
            }
            groups.entry(relation).or_default().push(id.to_vec());
        }
        for (relation, ids) in groups {
            self.check_relation(relation)?;
            let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {} WHERE id=ANY($1)",
                qualified(self.lease.generation(), relation)
            )))
            .bind(&ids)
            .fetch_one(&mut *self.lease.connection)
            .await?;
            if count != ids.len() as i64 {
                return Err(Error::Contract);
            }
        }
        Ok(())
    }
    async fn explain(
        &mut self,
        root: ProofReference,
        retained: &RequestExecution,
    ) -> Result<SectionPage<DerivationStep>, Error> {
        let limits = ResourceLimits::default();
        let _charge = self.lease.budget.reserve(
            "bounded-explanation",
            512 + limits.explanation_nodes as usize * 4096
                + limits.explanation_edges as usize * 2048,
        )?;
        let mut seen = BTreeSet::from([(root.relation.as_str().to_owned(), root.row)]);
        self.verify_proof_rows(&seen).await?;
        let mut frontier = seen.clone();
        let mut steps = BTreeMap::new();
        let mut edges = 0usize;
        let mut truncated = false;
        for depth in 0..limits.explanation_depth {
            if frontier.is_empty() {
                break;
            }
            // Read proofs directly from their declared canonical source records. Generated
            // browsing views cannot hide an entire support chain or substitute a valid foreign
            // premise: nominal fields supply every conclusion, role and target here.
            let mut rows = BTreeMap::new();
            let model = self.lease.model.clone();
            for declaration in model
                .relations()
                .iter()
                .filter(|r| self.lease.relations.contains(r.name()) && r.derivation().is_some())
            {
                self.check_relation(declaration.name())?;
                let rule = declaration.derivation().ok_or(Error::Contract)?;
                let (target, field) = rule
                    .conclusion
                    .as_ref()
                    .map_or((declaration.name(), "id"), |c| (c.target().1, c.name()));
                let ids = frontier
                    .iter()
                    .filter(|(name, _)| name == target)
                    .map(|(_, id)| id.to_vec())
                    .collect::<Vec<_>>();
                if ids.is_empty() {
                    continue;
                }
                let maximum = limits.explanation_nodes as usize;
                let generation = self.lease.generation();
                let expected:Option<(i64,Vec<u8>)>=sqlx::query_as("SELECT row_count,content_digest FROM lctx_model_store.receipts WHERE generation_id=$1 AND relation_name=$2").bind(generation.bytes().to_vec()).bind(declaration.name()).fetch_optional(&mut *self.lease.connection).await?;
                let expected = expected.ok_or(Error::Contract)?;
                let mut hash = declaration.content();
                super::visit_named(
                    &mut self.lease.connection,
                    generation,
                    declaration,
                    declaration.name(),
                    &["id"],
                    &self.lease.budget,
                    |batch| {
                        let batch = declaration.canonical(&batch)?;
                        declaration.hash_rows(&batch, &mut hash)?;
                        let value = |field: &str,
                                     index: usize|
                         -> Result<Option<[u8; 16]>, Error> {
                            let array = batch
                                .column_by_name(field)
                                .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
                                .ok_or(Error::Contract)?;
                            if array.is_null(index) {
                                Ok(None)
                            } else {
                                Ok(Some(
                                    array.value(index).try_into().map_err(|_| Error::Contract)?,
                                ))
                            }
                        };
                        for index in 0..batch.num_rows() {
                            let Some(conclusion) = value(field, index)? else {
                                continue;
                            };
                            if !ids.iter().any(|id| id.as_slice() == conclusion.as_slice()) {
                                continue;
                            }
                            if rows.len() == maximum {
                                truncated = true;
                                continue;
                            }
                            let id = value("id", index)?.ok_or(Error::Contract)?;
                            let mut premises = Vec::new();
                            for premise in &rule.premises {
                                match value(premise.name(), index)? {
                                    Some(id) => premises.push(PremisePacket {
                                        role: Name::new(premise.name()).map_err(wire)?,
                                        premise: reference(
                                            premise.target().1.to_owned(),
                                            id.to_vec(),
                                        )?,
                                    }),
                                    None if premise.nullable() => {}
                                    None => return Err(Error::Contract),
                                }
                            }
                            let source = reference(declaration.name().to_owned(), id.to_vec())?;
                            rows.insert(
                                (source.relation.as_str().to_owned(), source.row),
                                DerivationStep {
                                    source,
                                    rule: Name::new(rule.rule).map_err(wire)?,
                                    conclusion: reference(target.to_owned(), conclusion.to_vec())?,
                                    premises,
                                },
                            );
                        }
                        Ok(())
                    },
                )
                .await?;
                let (count, digest) = hash.finish();
                if u64::try_from(expected.0).ok() != Some(count) || expected.1 != digest.0 {
                    return Err(Error::Contract);
                }
                if truncated {
                    break;
                }
            }
            let mut next = BTreeSet::new();
            for (key, row) in rows {
                if edges.saturating_add(row.premises.len()) > limits.explanation_edges as usize
                    || steps.len() == limits.explanation_nodes as usize
                {
                    truncated = true;
                    break;
                }
                for premise in &row.premises {
                    next.insert((
                        premise.premise.relation.as_str().to_owned(),
                        premise.premise.row,
                    ));
                }
                edges += row.premises.len();
                next.insert((row.source.relation.as_str().to_owned(), row.source.row));
                steps.insert(key, row);
            }
            self.verify_proof_rows(&next).await?;
            next.retain(|r| !seen.contains(r));
            if seen.len() + next.len() > limits.explanation_nodes as usize {
                truncated = true;
                break;
            }
            seen.extend(next.iter().cloned());
            frontier = next;
            if depth + 1 == limits.explanation_depth && !frontier.is_empty() {
                truncated = true;
            }
        }
        let page = SectionPage {
            availability: if truncated {
                Availability::Partial {
                    reason: Name::new("explanation bound reached; remaining count unknown")
                        .map_err(wire)?,
                }
            } else {
                Availability::Available {}
            },
            items: steps.into_values().collect(),
            continuation: Optional::default(),
            omitted: 0,
            truncated,
        };
        // Retain the actual handoff before releasing the bounded construction reservation.
        retain(retained, &page)?;
        Ok(page)
    }
}
