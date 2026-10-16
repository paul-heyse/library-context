//! Native operation orchestration over scoped semantic owners.
use crate::{candidates, preparation::OperationRead, records::wire, service::QueryVector};
use lctx_model::domain::{
    resources::ResourceBudget,
    serving::ranking::{self, CandidateFusion, ChannelBinding, RankingPolicy},
    serving::*,
    *,
};
use lctx_native::{NativeReader, RecordSelection};
fn inputs(domains: &[LibraryDomainPacket]) -> Vec<[u8; 16]> {
    domains
        .iter()
        .flat_map(|d| &d.captures)
        .flat_map(|c| std::iter::once(c.release.input).chain(c.corpora.iter().copied()))
        .map(|i| *i.bytes())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn quiet() -> ChannelState {
    ChannelState {
        lexical: false,
        vector: VectorChannel::Disabled {},
    }
}
fn page<T>(
    values: Vec<(ContentHash, T)>,
    request: &Request,
    reader: &NativeReader,
    group: &str,
    section: &str,
    member: Option<Id<catalog::CatalogMember>>,
    availability: Availability,
) -> Result<SectionPage<T>, ModelError> {
    crate::pagination::page(
        values,
        request,
        reader.handle(),
        &quiet(),
        group,
        section,
        member,
        availability,
    )
    .map_err(wire)
}
pub(crate) fn hydrate_page<T, U>(
    compact: SectionPage<T>,
    mut hydrate: impl FnMut(T) -> Result<U, ModelError>,
) -> Result<SectionPage<U>, ModelError> {
    Ok(SectionPage {
        availability: compact.availability,
        continuation: compact.continuation,
        omitted: compact.omitted,
        truncated: compact.truncated,
        items: compact
            .items
            .into_iter()
            .map(&mut hydrate)
            .collect::<Result<_, _>>()?,
    })
}
type KeyedCandidate<'a> = (ContentHash, &'a selection::evaluate::CandidateSelection);
fn compact_candidates<'a>(
    candidates: impl IntoIterator<Item = &'a selection::evaluate::CandidateSelection>,
    data: &selection::classification::ClassificationData,
    domains: &[LibraryDomainPacket],
    budget: &ResourceBudget,
) -> Result<(Vec<KeyedCandidate<'a>>, charged::StateCharge), ModelError> {
    let mut values = Vec::new();
    let mut charge = charged::StateCharge::new(budget, "compact-operation-page");
    for candidate in candidates {
        candidates::validate(candidate, data, domains)?;
        if values.len() == values.capacity() {
            let additional = values.capacity().max(4);
            // Stable key sorting and the selected reference page may coexist with this
            // vector. Admit all three compact allocations before growing it.
            charge.grow(
                additional
                    .checked_mul(
                        3 * size_of::<(ContentHash, &selection::evaluate::CandidateSelection)>(),
                    )
                    .ok_or(ModelError::Schema("compact candidate page overflow"))?,
            )?;
            values.reserve_exact(additional);
        }
        values.push((candidates::selection_key(candidate), candidate));
    }
    Ok((values, charge))
}
fn extent(selected: &selection::evaluate::Selected) -> SelectionExtent {
    SelectionExtent::CompleteDomain {
        total: selected.candidates.len() as u64,
    }
}
async fn chosen(
    reader: &NativeReader,
    preparation: &crate::preparation::Preparation<'_>,
    library: &Name,
    selector: Option<&OperationSelector>,
    selection: &selection::Selection,
    b: &ResourceBudget,
) -> Result<crate::selection::MemberSelection, ModelError> {
    let members = crate::selection::members(reader, Some(library), selector, b).await?;
    crate::selection::classify_prepared(reader, &members, selection, b, preparation).await
}
pub(crate) async fn dispatch(
    read: OperationRead<'_, '_>,
    request: &Request,
    channels: &ChannelState,
    vector: Option<&QueryVector>,
    limits: &ResourceLimits,
    retained: &mut crate::ranked_results::RankedRequest<'_>,
    b: &ResourceBudget,
) -> Result<Response, ModelError> {
    prepare_dispatch_execution(read, request, channels, vector, limits, retained, b)?.await
}
type OperationExecution<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ModelError>> + Send + 'a>>;
// All operation callers share one admitted execution continuation. Construction
// happens only when dispatch is polled, before any query or retained-state effect.
fn prepare_dispatch_execution<'a>(
    read: OperationRead<'a, '_>,
    request: &'a Request,
    channels: &'a ChannelState,
    vector: Option<&'a QueryVector>,
    limits: &'a ResourceLimits,
    retained: &'a mut crate::ranked_results::RankedRequest<'_>,
    b: &'a ResourceBudget,
) -> Result<OperationExecution<'a>, ModelError> {
    let OperationRead {
        reader,
        preparation,
    } = read;
    let body = async move {
        let snapshot = reader.handle().clone();
        match request {
            Request::GetCapability(r) => Ok(Response::GetCapability(GetCapabilityResponse {
                delivery: Optional::default(),
                snapshot,
                capability: crate::capability::get(reader, preparation, r.capability, b).await?,
            })),
            Request::GetEvidence(r) => Ok(Response::GetEvidence(GetEvidenceResponse {
                delivery: Optional::default(),
                snapshot,
                evidence: crate::evidence::get(
                    reader,
                    preparation,
                    &r.source,
                    request,
                    channels,
                    limits,
                    b,
                )
                .await?,
            })),
            Request::InspectValuePaths(r) => {
                crate::inspection::get(reader, preparation, r, request, limits, b)
                    .await
                    .map(Response::InspectValuePaths)
            }
            Request::FindOperations(r) => {
                let domains =
                    crate::library::resolve_prepared(reader, Some(&r.library), b, preparation)
                        .await?;
                let data = chosen(reader, preparation, &r.library, None, &r.selection.0, b).await?;
                let group = |outcome, group| {
                    let (values, _compact) = compact_candidates(
                        data.selected.group(outcome).filter(|_| {
                            r.selection.0.mode == selection::Mode::Discovery
                                || outcome == selection::Outcome::Supported
                        }),
                        data.prepared.data(),
                        &domains,
                        b,
                    )?;
                    let compact = page(
                        values,
                        request,
                        reader,
                        group,
                        "results",
                        None,
                        Availability::Available {},
                    )?;
                    hydrate_page(compact, |candidate| {
                        candidates::packet(candidate, data.prepared.data(), &domains)
                    })
                };
                Ok(Response::FindOperations(FindOperationsResponse {
                    delivery: Optional::default(),
                    snapshot,
                    domains: domains.clone(),
                    supported: group(selection::Outcome::Supported, "supported")?,
                    unresolved: group(selection::Outcome::Unresolved, "unresolved")?,
                    conflicting: group(selection::Outcome::Conflicting, "conflicting")?,
                    extent: extent(&data.selected),
                }))
            }
            Request::CompareOperations(r) => {
                let domains =
                    crate::library::resolve_prepared(reader, Some(&r.library), b, preparation)
                        .await?;
                let mut operations = Vec::new();
                for selector in &r.operations {
                    let d = chosen(
                        reader,
                        preparation,
                        &r.library,
                        Some(selector),
                        &r.selection.0,
                        b,
                    )
                    .await?;
                    let candidates = d
                        .selected
                        .eligible()
                        .map(|c| candidates::packet(c, d.prepared.data(), &domains))
                        .collect::<Result<Vec<_>, _>>()?;
                    let members = candidates
                        .iter()
                        .map(|c| c.member)
                        .collect::<std::collections::BTreeSet<_>>();
                    operations.push(ComparisonEntry {
                        requested: selector.clone(),
                        candidates,
                        ambiguous: members.len() > 1,
                    });
                }
                Ok(Response::CompareOperations(CompareOperationsResponse {
                    delivery: Optional::default(),
                    snapshot,
                    domains,
                    operations,
                }))
            }
            Request::GetOperation(r) => {
                let domains =
                    crate::library::resolve_prepared(reader, Some(&r.library), b, preparation)
                        .await?;
                let members =
                    crate::selection::members(reader, Some(&r.library), Some(&r.operation), b)
                        .await?;
                let operation = match members.as_slice() {
                    [] => OperationResolution::Missing {
                        coverage: Availability::Partial {
                            reason: Name::new("captured_public_domain_only").map_err(wire)?,
                        },
                    },
                    [member] => OperationResolution::Unique {
                        packet: crate::operation_packet::get(
                            read, member, &domains, r, request, limits, b,
                        )
                        .await?,
                    },
                    _ => {
                        let d = crate::selection::classify_prepared(
                            reader,
                            &members,
                            &selection::Selection::default(),
                            b,
                            preparation,
                        )
                        .await?;
                        OperationResolution::Ambiguous {
                            candidates: d
                                .selected
                                .eligible()
                                .map(|c| candidates::packet(c, d.prepared.data(), &domains))
                                .collect::<Result<Vec<_>, _>>()?,
                        }
                    }
                };
                Ok(Response::GetOperation(GetOperationResponse {
                    delivery: Optional::default(),
                    snapshot,
                    domains,
                    operation,
                }))
            }
            Request::BrowseLibrary(r) => browse(reader, preparation, r, request, b)
                .await
                .map(Response::BrowseLibrary),
            Request::SearchOperations(r) => {
                search_operations(read, r, request, channels, vector, retained, b)
                    .await
                    .map(Response::SearchOperations)
            }
            Request::SearchEvidence(r) => {
                search_evidence(read, r, request, channels, vector, retained, b)
                    .await
                    .map(Response::SearchEvidence)
            }
            Request::SearchCapabilities(r) => {
                search_capabilities(read, r, request, channels, vector, retained, b)
                    .await
                    .map(Response::SearchCapabilities)
            }
        }
    };
    let bytes = std::mem::size_of_val(&body)
        .checked_add(std::mem::size_of::<
            Box<dyn lctx_model::domain::resources::Reservation>,
        >())
        .and_then(|bytes| bytes.checked_add(2 * std::mem::align_of_val(&body)))
        .ok_or(ModelError::Schema(
            "native operation execution continuation size",
        ))?;
    let grant = b.reserve("native operation execution continuation", bytes)?;
    Ok(Box::pin(async move {
        let _grant = grant;
        body.await
    }))
}
#[allow(
    clippy::too_many_arguments,
    reason = "Channel aggregation keeps library scope, context pairs, target mode, admitted units and query vector explicit"
)]
async fn scores(
    reader: &NativeReader,
    query: &str,
    families: &[retrieval::Family],
    domains: &[LibraryDomainPacket],
    pairs: Option<&[([u8; 16], [u8; 16])]>,
    member_mode: bool,
    units: crate::search::UnitScope,
    vector: Option<&QueryVector>,
    b: &ResourceBudget,
) -> Result<crate::search::SearchRows<ranking::CandidateScore>, ModelError> {
    let mut scores = crate::search::SearchRows::new(b);
    let policy = RankingPolicy::default();
    let inputs = inputs(domains);
    for family in families {
        scores.append(
            crate::search::lexical(
                reader,
                query,
                *family,
                &inputs,
                pairs,
                member_mode,
                units,
                128,
                &policy,
                b,
            )
            .await?,
        )?;
        if let Some(v) = vector {
            let _projection = b.reserve("search-query-projection", 1024 * size_of::<f32>())?;
            scores.append(
                crate::search::vector(
                    reader,
                    &embedding::projection::project_prefix(&v.vector, 1024)
                        .map_err(ModelError::Invalid)?,
                    v.spec,
                    embedding::value::value_digest(&v.vector),
                    v.recipe.identity(),
                    v.projection,
                    *family,
                    &inputs,
                    pairs,
                    member_mode,
                    units,
                    128,
                    &policy,
                    b,
                )
                .await?,
            )?;
        }
    }
    if let Some(v) = vector {
        let full = crate::search::rescore_union(reader, v, &scores, &policy, b).await?;
        scores.retain(|row| row.channel == ranking::Channel::Lexical);
        scores.append(full)?;
    }
    Ok(scores)
}
fn fusion(
    reader: &NativeReader,
    query: &str,
    vector: Option<&QueryVector>,
    scores: &[ranking::CandidateScore],
    promoted: &[Id<catalog::CatalogMember>],
    b: &ResourceBudget,
) -> Result<Vec<ranking::RankedHit>, ModelError> {
    let policy = RankingPolicy::default();
    let mut channels = vec![ChannelBinding::lexical(&policy, query)?];
    if let Some(v) = vector {
        channels.push(ChannelBinding::vector(
            &policy,
            v.spec,
            embedding::value::value_digest(&v.vector),
            v.recipe.identity(),
            v.projection,
        )?);
    }
    let mut eligible = scores
        .iter()
        .map(|s| s.occurrence.target)
        .collect::<std::collections::BTreeSet<_>>();
    eligible.extend(
        promoted
            .iter()
            .map(|m| ranking::Target::Member { member: *m }),
    );
    let occurrences = scores
        .iter()
        .map(|s| s.occurrence)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    Ok(CandidateFusion::new(
        reader.handle().clone(),
        policy,
        &channels,
        &eligible.into_iter().collect::<Vec<_>>(),
        &occurrences,
        b,
    )?
    .rank(scores, promoted)?
    .rows()
    .to_vec())
}
async fn search_operations(
    read: OperationRead<'_, '_>,
    r: &SearchOperationsRequest,
    request: &Request,
    channels: &ChannelState,
    vector: Option<&QueryVector>,
    retained: &mut crate::ranked_results::RankedRequest<'_>,
    b: &ResourceBudget,
) -> Result<SearchOperationsResponse, ModelError> {
    let OperationRead {
        reader,
        preparation,
    } = read;
    let domains =
        crate::library::resolve_prepared(reader, r.library.0.as_ref(), b, preparation).await?;
    let members = crate::selection::members(reader, r.library.0.as_ref(), None, b).await?;
    let selected =
        crate::selection::classify_prepared(reader, &members, &r.selection.0, b, preparation)
            .await?;
    let pairs = selected
        .selected
        .eligible()
        .map(|c| (*c.member.bytes(), *c.analysis.bytes()))
        .collect::<Vec<_>>();
    let scores = scores(
        reader,
        r.query.as_str(),
        &[
            retrieval::Family::ApiOptions,
            retrieval::Family::DocumentationDeployment,
            retrieval::Family::Scenario,
            retrieval::Family::Source,
        ],
        &domains,
        Some(&pairs),
        true,
        crate::search::UnitScope::All,
        vector,
        b,
    )
    .await?;
    let promoted = selected
        .selected
        .eligible()
        .filter(|c| {
            c.path.join(".") == r.query.as_str()
                || selected
                    .prepared
                    .data()
                    .source
                    .catalog
                    .members
                    .get(c.member)
                    .is_some_and(|m| m.name == r.query.as_str())
        })
        .map(|c| c.member)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let ranked = fusion(reader, r.query.as_str(), vector, &scores, &promoted, b)?;
    let mut nominated = Vec::new();
    for hit in ranked {
        let ranking::Target::Member { member } = hit.target else {
            return Err(ModelError::Schema("member ranking target"));
        };
        for candidate in selected.selected.eligible().filter(|candidate| {
            candidate.member == member
                && candidate.analysis == hit.context
                && (promoted.contains(&member)
                    || hit
                        .witnesses
                        .iter()
                        .any(|w| w.occurrence.context == candidate.analysis))
        }) {
            retained.admit_copy(&hit).map_err(wire)?;
            nominated.push((hit.clone(), candidate));
        }
    }
    let mut assembly = retained
        .begin(nominated.len(), request, reader.handle(), channels)
        .map_err(wire)?;
    for (hit, candidate) in nominated {
        let packet = candidates::packet(candidate, selected.prepared.data(), &domains)?;
        assembly
            .push(hit, candidates::selection_key(candidate), packet)
            .map_err(wire)?;
    }
    let (results, ranking) = assembly.finish().map_err(wire)?;
    Ok(SearchOperationsResponse {
        delivery: Optional::default(),
        snapshot: reader.handle().clone(),
        domains,
        extent: SelectionExtent::Ranked {
            returned: results.items.len() as u64,
        },
        results,
        channels: channels.clone(),
        ranking,
    })
}
async fn search_evidence(
    read: OperationRead<'_, '_>,
    r: &SearchEvidenceRequest,
    request: &Request,
    channels: &ChannelState,
    vector: Option<&QueryVector>,
    retained: &mut crate::ranked_results::RankedRequest<'_>,
    b: &ResourceBudget,
) -> Result<SearchEvidenceResponse, ModelError> {
    let OperationRead {
        reader,
        preparation,
    } = read;
    let domains =
        crate::library::resolve_prepared(reader, r.library.0.as_ref(), b, preparation).await?;
    let families = if r.families.is_empty() {
        [
            retrieval::Family::ApiOptions,
            retrieval::Family::DocumentationDeployment,
            retrieval::Family::Scenario,
            retrieval::Family::Source,
        ]
        .to_vec()
    } else {
        r.families.clone()
    };
    let scores = scores(
        reader,
        r.query.as_str(),
        &families,
        &domains,
        None,
        false,
        crate::search::UnitScope::All,
        vector,
        b,
    )
    .await?;
    let ranked = fusion(reader, r.query.as_str(), vector, &scores, &[], b)?;
    let batches = if ranked.is_empty() {
        None
    } else {
        Some(crate::evidence::hydrate_hits(reader, preparation, &ranked).await?)
    };
    let data = batches
        .as_ref()
        .map(|batches| crate::records::Prepared::new(batches, b));
    let mut assembly = retained
        .begin(ranked.len(), request, reader.handle(), channels)
        .map_err(wire)?;
    for hit in ranked {
        let packet = crate::evidence::hit_packet(
            data.as_ref().expect("nonempty evidence preparation"),
            &hit,
            &domains,
        )?;
        let ranking::Target::Unit { unit } = hit.target else {
            return Err(ModelError::Schema("evidence ranking target"));
        };
        assembly
            .push(
                hit,
                crate::pagination::target_key(ranking::Target::Unit { unit }),
                packet,
            )
            .map_err(wire)?;
    }
    let (results, ranking) = assembly.finish().map_err(wire)?;
    Ok(SearchEvidenceResponse {
        delivery: Optional::default(),
        snapshot: reader.handle().clone(),
        domains,
        extent: SelectionExtent::Ranked {
            returned: results.items.len() as u64,
        },
        results,
        channels: channels.clone(),
        ranking,
    })
}
async fn search_capabilities(
    read: OperationRead<'_, '_>,
    r: &SearchCapabilitiesRequest,
    request: &Request,
    channels: &ChannelState,
    vector: Option<&QueryVector>,
    retained: &mut crate::ranked_results::RankedRequest<'_>,
    b: &ResourceBudget,
) -> Result<SearchCapabilitiesResponse, ModelError> {
    let OperationRead {
        reader,
        preparation,
    } = read;
    let domains =
        crate::library::resolve_prepared(reader, r.library.0.as_ref(), b, preparation).await?;
    let scores = scores(
        reader,
        r.query.as_str(),
        &[retrieval::Family::ApiOptions],
        &domains,
        None,
        false,
        crate::search::UnitScope::BriefOrigins,
        vector,
        b,
    )
    .await?;
    let ranked = fusion(reader, r.query.as_str(), vector, &scores, &[], b)?;
    let mut unit_rows = std::collections::BTreeMap::new();
    let unit_keys = ranked
        .iter()
        .map(|hit| match hit.target {
            ranking::Target::Unit { unit } => Ok(*unit.bytes()),
            _ => Err(ModelError::Schema("capability ranking target")),
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?
        .into_iter()
        .collect::<Vec<_>>();
    for chunk in unit_keys.chunks(64) {
        for unit in reader
            .records::<retrieval::Unit>(RecordSelection::Keys(chunk.to_vec()))
            .await?
        {
            unit_rows.insert(unit.id(), unit);
        }
    }
    let origin_keys = unit_rows
        .values()
        .map(|unit| *unit.origin.bytes())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut origins = std::collections::BTreeMap::new();
    for chunk in origin_keys.chunks(64) {
        for origin in reader
            .records::<retrieval::Origin>(RecordSelection::Keys(chunk.to_vec()))
            .await?
        {
            origins.insert(origin.id(), origin);
        }
    }
    let mut nominated = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for hit in ranked {
        let ranking::Target::Unit { unit } = hit.target else {
            return Err(ModelError::Schema("capability ranking target"));
        };
        let unit = unit_rows
            .get(&unit)
            .ok_or(ModelError::Schema("capability unit"))?;
        if unit.context != hit.context {
            return Err(ModelError::Conflict("capability hit context"));
        }
        if let retrieval::Origin::Brief { brief } = origins
            .get(&unit.origin)
            .ok_or(ModelError::Schema("capability origin"))?
            && seen.insert((*brief, hit.context))
        {
            nominated.push((hit, *brief));
        }
    }
    let brief_ids = nominated
        .iter()
        .map(|(_, brief)| *brief)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut assembly = retained
        .begin(nominated.len(), request, reader.handle(), channels)
        .map_err(wire)?;
    for cohort in brief_ids.chunks(64) {
        let data = crate::capability::hydrate(reader, preparation, cohort, b).await?;
        let data = crate::records::Prepared::new(&data, b);
        let prepared = crate::capability::Prepared::new(&data, b)?;
        for brief in cohort {
            let packet = prepared.packet(*brief, b)?;
            let key = match graph::target_for_row(derivation::RowRef::of(*brief))? {
                graph::Target::Assertion(id) => id.0,
                graph::Target::Entity(id) => id.0,
                _ => return Err(ModelError::Schema("brief graph key")),
            };
            for (ordinal, (hit, _)) in nominated
                .iter()
                .enumerate()
                .filter(|(_, (_, id))| id == brief)
            {
                // Only one brief body is live; its frozen row is installed at the
                // original ranked ordinal even though hydration is grouped by brief.
                assembly
                    .push_ref_at(ordinal, hit, key, &packet)
                    .map_err(wire)?;
            }
        }
    }
    let (results, ranking) = assembly.finish().map_err(wire)?;
    Ok(SearchCapabilitiesResponse {
        delivery: Optional::default(),
        snapshot: reader.handle().clone(),
        domains,
        extent: SelectionExtent::Ranked {
            returned: results.items.len() as u64,
        },
        results,
        channels: channels.clone(),
        ranking,
    })
}
async fn browse(
    reader: &NativeReader,
    preparation: &crate::preparation::Preparation<'_>,
    r: &BrowseLibraryRequest,
    request: &Request,
    b: &ResourceBudget,
) -> Result<BrowseLibraryResponse, ModelError> {
    let domains =
        crate::library::resolve_prepared(reader, Some(&r.library), b, preparation).await?;
    if let BrowseScope::Module { module } = r.scope {
        let modules = reader
            .records::<source::Module>(RecordSelection::Keys(vec![*module.bytes()]))
            .await?;
        let module = modules
            .first()
            .ok_or(ModelError::Serving(FailureKind::Incompatible))?;
        let sources = reader
            .records::<source::SourceArtifact>(RecordSelection::Keys(vec![*module.source.bytes()]))
            .await?;
        let source = sources
            .first()
            .ok_or(ModelError::Serving(FailureKind::Corrupt))?;
        if !domains
            .iter()
            .flat_map(|domain| &domain.captures)
            .any(|capture| capture.release.input == source.input)
        {
            return Err(ModelError::Serving(FailureKind::Incompatible));
        }
    }
    let d = chosen(reader, preparation, &r.library, None, &r.selection.0, b).await?;
    let data = d.prepared.data();
    if let BrowseScope::Class { member } = r.scope {
        let admitted = data
            .source
            .catalog
            .members
            .get(member)
            .is_some_and(|member| {
                domains
                    .iter()
                    .flat_map(|domain| &domain.captures)
                    .any(|capture| capture.release.input == member.input)
            });
        if !admitted
            || !data
                .source
                .catalog
                .classes
                .iter()
                .any(|class| class.member == member)
        {
            return Err(ModelError::Serving(FailureKind::Incompatible));
        }
    }
    let mut values = Vec::new();
    // Build direct ownership once. Candidate paths can repeat a member; all counts use member
    // identity, while classifications retain their distinct analysis contexts.
    let mut owners = std::collections::BTreeMap::<
        Id<catalog::CatalogMember>,
        std::collections::BTreeSet<Id<catalog::CatalogMember>>,
    >::new();
    let mut members_by_path = std::collections::BTreeMap::new();
    for candidate in data.source.catalog.candidates.iter() {
        if let Some(path) = candidate.path
            && let Some(exposure) = data.source.catalog.exposures.get(candidate.exposure)
        {
            members_by_path
                .entry(path)
                .or_insert_with(std::collections::BTreeSet::new)
                .insert(exposure.member);
        }
    }
    for path in data.source.catalog.paths.iter() {
        let Some(parent) = data
            .source
            .catalog
            .candidates
            .get(path.parent)
            .and_then(|candidate| data.source.catalog.exposures.get(candidate.exposure))
        else {
            continue;
        };
        if let Some(members) = members_by_path.get(&path.id()) {
            for child in members {
                owners.entry(*child).or_default().insert(parent.member);
            }
        }
    }
    let mut unknown_members = std::collections::BTreeSet::new();
    let mut eligible = Vec::new();
    for candidate in d.selected.eligible() {
        let member = data
            .source
            .catalog
            .members
            .get(candidate.member)
            .ok_or(ModelError::Schema("browse member"))?;
        let include = match r.scope {
            BrowseScope::Library {} => true,
            BrowseScope::Module { module } => member.access == module,
            BrowseScope::Class { member: class } => {
                let owned = owners.get(&candidate.member);
                if owned.is_none_or(|parents| parents.is_empty()) {
                    unknown_members.insert(candidate.member);
                }
                owned.is_some_and(|parents| parents.contains(&class))
            }
        };
        if include {
            eligible.push(candidate);
        }
    }
    let scoped_members = eligible
        .iter()
        .map(|candidate| candidate.member)
        .collect::<std::collections::BTreeSet<_>>();
    let unknown = unknown_members.len() as u64;
    let mut emitted_groups = std::collections::BTreeSet::new();
    for c in &eligible {
        let member = data
            .source
            .catalog
            .members
            .get(c.member)
            .ok_or(ModelError::Schema("browse member"))?;
        if r.view != BrowseView::Members {
            candidates::validate(c, data, &domains)?;
        }
        match r.view {
            BrowseView::Members => {}
            BrowseView::Classes => {
                if data
                    .source
                    .catalog
                    .classes
                    .iter()
                    .any(|cl| cl.member == member.id())
                    && emitted_groups.insert(graph::EntityId::of(member.id()).0)
                {
                    values.push((
                        graph::EntityId::of(member.id()).0,
                        BrowseEntry::Class {
                            member: member.id(),
                            name: Name::new(c.path.join(".")).map_err(wire)?,
                            members: scoped_members
                                .iter()
                                .filter(|child| {
                                    owners
                                        .get(child)
                                        .is_some_and(|parents| parents.contains(&member.id()))
                                })
                                .count() as u64,
                        },
                    ));
                }
            }
            BrowseView::Modules => {
                let module = data
                    .source
                    .core
                    .modules
                    .get(member.access)
                    .ok_or(ModelError::Schema("browse module"))?;
                let key = graph::EntityId::of(module.id()).0;
                if emitted_groups.insert(key) {
                    values.push((
                        key,
                        BrowseEntry::Module {
                            module: module.id(),
                            name: Name::new(module.qualified_name.clone()).map_err(wire)?,
                            members: scoped_members
                                .iter()
                                .filter(|id| {
                                    data.source
                                        .catalog
                                        .members
                                        .get(**id)
                                        .is_some_and(|member| member.access == module.id())
                                })
                                .count() as u64,
                        },
                    ));
                }
            }
            BrowseView::Vocabulary => {}
        }
    }
    if r.view == BrowseView::Vocabulary {
        let mut facets = std::collections::BTreeMap::<
            selection::Facet,
            (
                std::collections::BTreeSet<Name>,
                std::collections::BTreeSet<Id<catalog::CatalogMember>>,
            ),
        >::new();
        for (facet, text, predicate) in crate::vocabulary::questions(data) {
            let requirement = selection::Requirement {
                predicate,
                quantifier: selection::Quantifier::AnyApplicable,
            };
            for c in &eligible {
                let result = d.prepared.classify(c.member, c.analysis, &requirement, b)?;
                if result.outcome == selection::Outcome::Supported {
                    let entry = facets.entry(facet).or_default();
                    entry.0.insert(Name::new(text.clone()).map_err(wire)?);
                    entry.1.insert(c.member);
                }
            }
        }
        for (facet, (names, members)) in facets {
            values.push((
                ContentHash::of(format!("native-vocabulary:{facet:?}").as_bytes()),
                BrowseEntry::Vocabulary {
                    facet,
                    values: names.into_iter().collect(),
                    members: members.len() as u64,
                },
            ));
        }
    }
    let entries = if r.view == BrowseView::Members {
        let (compact, _compact) = compact_candidates(eligible.iter().copied(), data, &domains, b)?;
        let compact = page(
            compact,
            request,
            reader,
            request.tool().name(),
            "entries",
            None,
            Availability::Available {},
        )?;
        hydrate_page(compact, |candidate| {
            Ok(BrowseEntry::Member {
                candidate: candidates::packet(candidate, data, &domains)?,
                ownership: if matches!(r.scope, BrowseScope::Class { .. }) {
                    Availability::Available {}
                } else {
                    Availability::NotRequested {}
                },
            })
        })?
    } else {
        page(
            values,
            request,
            reader,
            request.tool().name(),
            "entries",
            None,
            Availability::Available {},
        )?
    };
    Ok(BrowseLibraryResponse {
        delivery: Optional::default(),
        snapshot: reader.handle().clone(),
        domains,
        scope: r.scope.clone(),
        view: r.view,
        entries,
        extent: SelectionExtent::CompleteDomain {
            total: scoped_members.len() as u64,
        },
        unknown_ownership: unknown,
    })
}

#[cfg(test)]
mod compact_page_controls {
    use super::*;
    fn id<R: Record>(byte: u8) -> Id<R> {
        serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
    }
    fn fixture(
        budget: &ResourceBudget,
    ) -> (selection::evaluate::Prepared, Vec<LibraryDomainPacket>) {
        let mut data = selection::classification::ClassificationData::new(budget);
        let mut output = selection::build::Output::new(budget);
        let module = data
            .source
            .core
            .modules
            .insert(source::Module {
                source: id(1),
                qualified_name: "api".into(),
            })
            .unwrap();
        let mut captures = Vec::new();
        for byte in 2..18 {
            let member = data
                .source
                .catalog
                .members
                .insert(catalog::CatalogMember {
                    input: id(byte),
                    access: module,
                    path: vec![format!("member{byte:02}")],
                    name: format!("member{byte:02}"),
                })
                .unwrap();
            output
                .domains
                .insert(selection::SelectionDomain {
                    member,
                    analysis: id(1),
                    kind: selection::DomainKind::PublicExposures,
                    corpus_complete: false,
                    analyzer_complete: false,
                    contexts: KeySink::new("selection-domain-contexts/v1").finish(),
                    closure: KeySink::new("selection-domain-closure/v1").finish(),
                })
                .unwrap();
            captures.push(LibraryCapturePacket {
                release: ReleaseIdentity {
                    input: id(byte),
                    release: id(byte),
                    distribution: Name::new("fixture").unwrap(),
                    version: Name::new("1").unwrap(),
                },
                corpora: Vec::new(),
                collection: Availability::Available {},
                coverage: Vec::new(),
                unavailable_inputs: Vec::new(),
            });
        }
        (
            selection::evaluate::Prepared::from_local_rows(data, output, budget).unwrap(),
            vec![LibraryDomainPacket {
                name: Name::new("fixture").unwrap(),
                captures,
            }],
        )
    }
    fn snapshot() -> SnapshotHandle {
        let hash = ContentHash::of(b"compact-page-control");
        SnapshotHandle {
            eligibility: lctx_model::domain::completed_transport::EligibilityIdentity(hash),
            publication: hash,
            semantic: hash,
            realization: hash,
            view: hash,
            service_generation: hash,
            definition_epoch: hash,
            database: DatabaseIdentity {
                namespace: Name::new("ns").unwrap(),
                database: Name::new("db").unwrap(),
            },
        }
    }
    #[test]
    fn compact_pages_match_complete_packets_and_hydrate_only_requested_items() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let (prepared, domains) = fixture(&budget);
        let selected = prepared
            .select(&selection::Selection::default(), &budget)
            .unwrap();
        let mut reference = selected
            .candidates
            .iter()
            .map(|candidate| {
                let packet = candidates::packet(candidate, prepared.data(), &domains).unwrap();
                (candidates::key(&packet), packet)
            })
            .collect::<Vec<_>>();
        reference.sort_by_key(|(key, _)| *key);
        let mut request = Request::FindOperations(
            serde_json::from_value(serde_json::json!({"library":"fixture","page":{"size":1}}))
                .unwrap(),
        );
        let mut actual = Vec::new();
        let mut hydration_count = 0;
        loop {
            let (values, _charge) =
                compact_candidates(&selected.candidates, prepared.data(), &domains, &budget)
                    .unwrap();
            let compact = crate::pagination::page(
                values,
                &request,
                &snapshot(),
                &quiet(),
                "supported",
                "results",
                None,
                Availability::Available {},
            )
            .unwrap();
            let page = hydrate_page(compact, |candidate| {
                hydration_count += 1;
                candidates::packet(candidate, prepared.data(), &domains)
            })
            .unwrap();
            assert_eq!(page.items.len(), 1);
            assert_eq!(
                hydration_count,
                actual.len() + 1,
                "each tiny page builds one rich packet"
            );
            actual.extend(page.items);
            let Some(cursor) = page.continuation.0 else {
                break;
            };
            let Request::FindOperations(r) = &mut request else {
                unreachable!()
            };
            r.page.cursor = Optional(Some(cursor));
        }
        assert_eq!(
            actual,
            reference
                .into_iter()
                .map(|(_, packet)| packet)
                .collect::<Vec<_>>()
        );
        // Remove the release of the last ordered candidate: first-page hydration alone
        // would miss this corruption, while global compact validation must refuse it.
        let last = selected
            .candidates
            .iter()
            .max_by_key(|candidate| candidates::selection_key(candidate))
            .unwrap();
        let missing = prepared
            .data()
            .source
            .catalog
            .members
            .get(last.member)
            .unwrap()
            .input;
        let mut corrupt = domains;
        corrupt[0]
            .captures
            .retain(|capture| capture.release.input != missing);
        assert!(matches!(
            compact_candidates(&selected.candidates, prepared.data(), &corrupt, &budget),
            Err(ModelError::Schema("selected member release admission"))
        ));
    }
}
