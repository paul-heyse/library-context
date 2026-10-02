//! Canonical retrieval metadata and one Rust ranking policy around numerical library callbacks.
use super::catalog_service::{name, wire_error};
use super::{CatalogService, Error, RequestExecution, VectorArtifact};
use lctx_model::domain::resources;
use lctx_model::domain::{
    normalized::Rows,
    retrieval::{
        self, Fragment, Origin, OriginalAnchor, Subject, Unit, UnitSubject,
        consumption::RetrievalEmbeddingUse,
    },
    serving::{
        ranking::{
            self, Channel, ChannelBinding, DocumentScore, LexicalCorpus, MemberLexicalCorpus,
            NumericalScore, Occurrence, PreparedRanking, RankingPolicy, Target, TextOccurrence,
        },
        *,
    },
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
const FAMILIES: [retrieval::Family; 4] = [
    retrieval::Family::ApiOptions,
    retrieval::Family::DocumentationDeployment,
    retrieval::Family::Scenario,
    retrieval::Family::Source,
];
struct Data {
    units: Rows<Unit>,
    fragments: Rows<Fragment>,
    subjects: Rows<Subject>,
    unit_subjects: Rows<UnitSubject>,
    anchors: Rows<OriginalAnchor>,
    origins: Rows<Origin>,
    uses: Rows<RetrievalEmbeddingUse>,
    parents: Rows<input::CorpusLibrary>,
}
impl Data {
    fn new(b: &resources::ResourceBudget) -> Self {
        Self {
            units: Rows::new(b),
            fragments: Rows::new(b),
            subjects: Rows::new(b),
            unit_subjects: Rows::new(b),
            anchors: Rows::new(b),
            origins: Rows::new(b),
            uses: Rows::new(b),
            parents: Rows::new(b),
        }
    }
}
struct State {
    catalog: CatalogService,
    data: Data,
    unit_occurrences: Vec<Occurrence>,
    member_occurrences: Vec<Occurrence>,
    unit_corpus: Arc<LexicalCorpus>,
    member_corpus: MemberLexicalCorpus,
    vectors: Option<Arc<VectorArtifact>>,
    _charge: Box<dyn resources::Reservation>,
}
#[derive(Clone)]
pub struct RetrievalService {
    state: Arc<State>,
}
#[derive(serde::Serialize)]
pub struct NumericalCorpus {
    pub policy: RankingPolicy,
    pub documents: Vec<ranking::LexicalDocument>,
}
#[derive(serde::Serialize)]
pub struct NumericalQuery {
    pub tokens: Vec<(retrieval::Family, Vec<String>)>,
}
/// A prepared request owns exact eligibility and channel identities through numerical work.
pub struct RankingRequest {
    ranking: PreparedRanking,
    candidates: Vec<OperationCandidate>,
    channels: ChannelState,
    promoted: Vec<Id<catalog::CatalogMember>>,
    identity: identity::RequestIdentity,
    member: bool,
    occurrences: Vec<Occurrence>,
    _charge: Box<dyn resources::Reservation>,
}
impl RetrievalService {
    /// Read-only startup consumes canonical receipts; a configured vector channel requires an
    /// explicitly prepared artifact. Lexical-only startup never writes or silently enables it.
    pub async fn prepare(catalog: CatalogService, vector_enabled: bool) -> Result<Self, Error> {
        let guard = catalog.service().guard();
        guard.check().await?;
        let mut held = guard.state.lease.lock().await;
        let lease = held.as_mut().ok_or(Error::State)?;
        let mut data = Data::new(&lease.budget);
        macro_rules! load{($($field:ident:$ty:ty),*$(,)?)=>{$(lease.visit_verified::<$ty>(|batch|{for row in batch.rows(){data.$field.insert(row.clone())?;}Ok(())}).await?;)*};}
        load!(units:Unit,fragments:Fragment,subjects:Subject,unit_subjects:UnitSubject,anchors:OriginalAnchor,origins:Origin,uses:RetrievalEmbeddingUse,parents:input::CorpusLibrary);
        let budget = lease.budget.clone();
        drop(held);
        let generation = GenerationKey(*catalog.service().generation().bytes());
        let retained_guard = guard.clone();
        let (data, unit_occurrences, member_occurrences, unit_corpus, member_corpus, charge) =
            tokio::task::spawn_blocking(move || {
                let _guard = retained_guard;
                let mut charge = budget.reserve("retrieval-occurrence-joins", 0)?;
                let mut bytes = 0usize;
                let mut unit_occurrences = Vec::new();
                let mut member_occurrences = Vec::new();
                let mut text = Vec::new();
                for unit in data.units.iter() {
                    let member_ids = data
                        .unit_subjects
                        .iter()
                        .filter(|s| s.unit == unit.id())
                        .map(|s| data.subjects.get(s.subject).ok_or(Error::Contract))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .filter_map(|s| {
                            if let Subject::Member { member } = s {
                                Some(*member)
                            } else {
                                None
                            }
                        })
                        .collect::<BTreeSet<_>>();
                    let mut anchors = data
                        .anchors
                        .iter()
                        .filter(|a| a.unit == unit.id())
                        .map(|a| Some(a.id()))
                        .collect::<Vec<_>>();
                    if anchors.is_empty() {
                        anchors.push(None);
                    }
                    for fragment in data.fragments.iter().filter(|f| f.corpus == unit.corpus) {
                        for anchor in &anchors {
                            let occurrence = Occurrence {
                                target: Target::Unit { unit: unit.id() },
                                unit: unit.id(),
                                fragment: fragment.id(),
                                context: unit.context,
                                anchor: *anchor,
                                family: unit.family,
                            };
                            bytes = bytes
                                .checked_add(768 + fragment.text.len() * 2 + member_ids.len() * 512)
                                .ok_or(Error::ResourceRefused("retrieval join size"))?;
                            charge.try_resize(bytes)?;
                            unit_occurrences.push(occurrence);
                            text.push(TextOccurrence {
                                occurrence,
                                text: fragment.text.as_str().to_owned(),
                            });
                            for member in &member_ids {
                                member_occurrences.push(Occurrence {
                                    target: Target::Member { member: *member },
                                    ..occurrence
                                });
                            }
                        }
                    }
                }
                let policy = RankingPolicy::default();
                let channel = ChannelBinding::lexical(&policy, "preparation")?;
                let units = data
                    .units
                    .iter()
                    .map(|u| Target::Unit { unit: u.id() })
                    .collect::<Vec<_>>();
                let unit_preparation = PreparedRanking::new(
                    generation,
                    policy.clone(),
                    &[channel],
                    &units,
                    &unit_occurrences,
                    &budget,
                )?;
                let unit_corpus = Arc::new(unit_preparation.prepare_lexical(&text)?);
                drop(text);
                drop(unit_preparation);
                let members = member_occurrences
                    .iter()
                    .map(|o| o.target)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let member_preparation = PreparedRanking::new(
                    generation,
                    policy,
                    &[channel],
                    &members,
                    &member_occurrences,
                    &budget,
                )?;
                let member_corpus =
                    member_preparation.project_member_lexical(unit_corpus.clone())?;
                drop(member_preparation);
                Ok::<_, Error>((
                    data,
                    unit_occurrences,
                    member_occurrences,
                    unit_corpus,
                    member_corpus,
                    charge,
                ))
            })
            .await
            .map_err(|_| Error::State)??;
        guard.check().await?;
        let vectors = if vector_enabled {
            Some(guard.vector_artifact().await?)
        } else {
            None
        };
        Ok(Self {
            state: Arc::new(State {
                catalog,
                data,
                unit_occurrences,
                member_occurrences,
                unit_corpus,
                member_corpus,
                vectors,
                _charge: charge,
            }),
        })
    }
    pub fn catalog(&self) -> &CatalogService {
        &self.state.catalog
    }
    pub fn embedding_spec(&self) -> Option<&embedding::Spec> {
        self.state.vectors.as_ref().map(|v| &v.specification)
    }
    /// Conservative admitted conversion/index buffers over the exact prepared numerical domain.
    /// This allowance is not a measurement of Python, NumPy or total process RSS.
    pub async fn reserve_numerical(&self) -> Result<super::PreparedReservation, Error> {
        let mut bytes = 4096usize;
        for document in self.state.unit_corpus.documents() {
            bytes = bytes
                .checked_add(document.text.len().checked_mul(8).ok_or(Error::Contract)?)
                .and_then(|n| n.checked_add(512))
                .ok_or(Error::Contract)?;
            for token in &document.tokens {
                bytes = bytes
                    .checked_add(token.len().checked_mul(8).ok_or(Error::Contract)?)
                    .and_then(|n| n.checked_add(192))
                    .ok_or(Error::Contract)?;
            }
        }
        self.state
            .catalog
            .service()
            .reserve_prepared("numerical corpus and library index", bytes)
            .await
    }
    pub fn numerical_corpus(&self, e: &RequestExecution) -> Result<NumericalCorpus, Error> {
        self.state.catalog.check_execution(e)?;
        let documents = self.state.unit_corpus.documents();
        let bytes = documents
            .iter()
            .map(|d| d.text.len() * 2 + d.tokens.iter().map(|t| t.len() + 64).sum::<usize>() + 256)
            .sum();
        e.retain("numerical corpus handoff", bytes)?;
        Ok(NumericalCorpus {
            policy: RankingPolicy::default(),
            documents: documents.to_vec(),
        })
    }
    fn belongs(&self, unit: &Unit, library: &Name) -> bool {
        let data = self.state.catalog.prepared().data();
        let subjects = self
            .state
            .data
            .unit_subjects
            .iter()
            .filter(|s| s.unit == unit.id())
            .filter_map(|s| self.state.data.subjects.get(s.subject))
            .collect::<Vec<_>>();
        let explicit = subjects.iter().any(|subject| match subject {
            Subject::Member { member } => data
                .source
                .catalog
                .members
                .get(*member)
                .is_some_and(|m| self.state.catalog.belongs(m, library)),
            Subject::Release { release } => data
                .facts
                .releases
                .get(*release)
                .and_then(|r| data.facts.packages.get(r.package))
                .is_some_and(|p| p.name == library.as_str()),
        });
        if !subjects.is_empty() {
            return explicit;
        }
        // Unassociated captured corpus evidence belongs to its declared acquisition library;
        // a shared installed environment cannot silently confer first-party ownership.
        self.state
            .data
            .parents
            .iter()
            .filter(|p| p.corpus == unit.input)
            .any(|parent| {
                self.state.catalog.distributions().iter().any(|r| {
                    r.input == parent.library
                        && r.role == input::DistributionRole::FirstParty
                        && data
                            .facts
                            .releases
                            .get(r.release)
                            .and_then(|r| data.facts.packages.get(r.package))
                            .is_some_and(|p| p.name == library.as_str())
                })
            })
    }
    pub async fn request(
        &self,
        e: &RequestExecution,
        r: &Request,
        vector: Option<Vec<f32>>,
        degradation: Option<String>,
    ) -> Result<RankingRequest, Error> {
        self.state.catalog.check_execution(e)?;
        let this = self.clone();
        let request = r.clone();
        e.cpu(move |budget| {
            let (library, query, selection, families, member, briefs) = match &request {
                Request::SearchOperations(r) => (
                    r.library.clone(),
                    r.query.clone(),
                    r.selection.0.clone(),
                    FAMILIES.to_vec(),
                    true,
                    false,
                ),
                Request::SearchEvidence(r) => (
                    r.library.clone(),
                    r.query.clone(),
                    selection::Selection::default(),
                    r.families.clone(),
                    false,
                    false,
                ),
                Request::SearchCapabilities(r) => (
                    r.library.clone(),
                    r.query.clone(),
                    selection::Selection::default(),
                    FAMILIES.to_vec(),
                    false,
                    true,
                ),
                _ => return Err(Error::Contract),
            };
            let candidates = if member {
                this.state
                    .catalog
                    .candidates(&selection, &library, budget)?
                    .into_iter()
                    .filter(|(_, outcome, _)| {
                        *outcome == selection::Outcome::Supported
                            || (selection.mode == selection::Mode::Discovery
                                && *outcome != selection::Outcome::Contradicted)
                    })
                    .map(|(c, _, _)| c)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let eligible = candidates
                .iter()
                .map(|c| (c.member, c.analysis))
                .collect::<BTreeSet<_>>();
            let mut occurrences = Vec::new();
            if member {
                for occurrence in &this.state.member_occurrences {
                    let Target::Member { member } = occurrence.target else {
                        return Err(Error::Contract);
                    };
                    if eligible.contains(&(member, occurrence.context)) {
                        occurrences.push(*occurrence);
                    }
                }
            } else {
                for occurrence in &this.state.unit_occurrences {
                    let unit = this
                        .state
                        .data
                        .units
                        .get(occurrence.unit)
                        .ok_or(Error::Contract)?;
                    if this.belongs(unit, &library)
                        && families.contains(&unit.family)
                        && (!briefs
                            || matches!(
                                this.state.data.origins.get(unit.origin),
                                Some(Origin::Brief { .. })
                            ))
                    {
                        occurrences.push(*occurrence);
                    }
                }
            }
            let targets = if member {
                candidates
                    .iter()
                    .map(|c| Target::Member { member: c.member })
                    .collect::<BTreeSet<_>>()
            } else {
                occurrences.iter().map(|o| o.target).collect()
            };
            let policy = RankingPolicy::default();
            let mut bindings = vec![ChannelBinding::lexical(&policy, query.as_str())?];
            let vector_channel = match vector {
                Some(ref value) => {
                    let artifact = this.state.vectors.as_ref().ok_or(Error::State)?;
                    embedding::check_vector(value, 1024).map_err(|_| Error::Contract)?;
                    let digest = embedding::value::value_digest(value);
                    bindings.push(ChannelBinding::vector(
                        &policy,
                        artifact.specification.hash(),
                        digest,
                    )?);
                    VectorChannel::Available {
                        spec: artifact.specification.hash(),
                        query_vector: digest,
                    }
                }
                None => {
                    if let Some(reason) = degradation {
                        VectorChannel::Degraded {
                            reason: name(reason)?,
                        }
                    } else {
                        VectorChannel::Disabled {}
                    }
                }
            };
            let promoted = if member {
                candidates
                    .iter()
                    .filter_map(|c| {
                        let m = this
                            .state
                            .catalog
                            .prepared()
                            .data()
                            .source
                            .catalog
                            .members
                            .get(c.member)?;
                        this.state
                            .catalog
                            .path(m)
                            .ok()
                            .filter(|path| path.join(".") == query.as_str())
                            .map(|_| c.member)
                    })
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            } else {
                vec![]
            };
            let charge = budget.reserve(
                "ranking-candidate-metadata",
                candidates.iter().try_fold(0usize, |bytes, c| {
                    Ok::<_, Error>(bytes.saturating_add(
                        super::catalog_service::serialized_len(c)?.saturating_mul(2),
                    ))
                })?,
            )?;
            let ranking = PreparedRanking::new(
                this.state.catalog.generation(),
                policy,
                &bindings,
                &targets.into_iter().collect::<Vec<_>>(),
                &occurrences,
                budget,
            )?;
            Ok(RankingRequest {
                ranking,
                candidates,
                channels: ChannelState {
                    lexical: true,
                    vector: vector_channel,
                },
                promoted,
                identity: request.canonical_identity().map_err(wire_error)?,
                member,
                occurrences,
                _charge: charge,
            })
        })
        .await
    }
    pub fn numerical_query(
        &self,
        e: &RequestExecution,
        request: &RankingRequest,
        query: &str,
    ) -> Result<NumericalQuery, Error> {
        self.state.catalog.check_execution(e)?;
        let mut tokens = Vec::new();
        for family in FAMILIES {
            let values = if request.member {
                self.state
                    .member_corpus
                    .query_tokens(&request.ranking, family, query)?
            } else {
                self.state
                    .unit_corpus
                    .query_tokens(&request.ranking, family, query)?
            };
            tokens.push((family, values.tokens().to_vec()));
        }
        e.retain("numerical-query-handoff", query.len() * 128 + 512)?;
        Ok(NumericalQuery { tokens })
    }
    pub async fn rank(
        &self,
        e: &RequestExecution,
        request: RankingRequest,
        lexical: Vec<DocumentScore>,
        query_vector: Option<Vec<f32>>,
    ) -> Result<(RankingRequest, ranking::RankedResults), Error> {
        self.state.catalog.check_execution(e)?;
        match (&query_vector, request.ranking.channel(Channel::Vector)) {
            (Some(vector), Some(binding)) => {
                let artifact = self.state.vectors.as_ref().ok_or(Error::State)?;
                embedding::check_vector(vector, 1024).map_err(|_| Error::Contract)?;
                if ChannelBinding::vector(
                    request.ranking.policy(),
                    artifact.specification.hash(),
                    embedding::value::value_digest(vector),
                )?
                .identity()
                    != binding.identity()
                {
                    return Err(Error::Contract);
                }
            }
            (None, None) => {}
            _ => return Err(Error::Contract),
        }
        let eligible: Vec<_> = self
            .state
            .data
            .uses
            .iter()
            .filter(|u| {
                u.availability == embedding::analytic::VectorAvailability::Available
                    && request.ranking.channel(Channel::Vector).is_some()
                    && self
                        .occurrences(&request)
                        .iter()
                        .any(|o| o.fragment == u.fragment)
            })
            .map(Record::id)
            .collect();
        let vectors = match query_vector {
            Some(vector) => Some(
                self.state
                    .vectors
                    .as_ref()
                    .ok_or(Error::State)?
                    .score(e, eligible, vector)
                    .await?,
            ),
            None => None,
        };
        let this = self.clone();
        e.cpu(move |budget| {
            let scores = if request.member {
                this.state
                    .member_corpus
                    .expand_scores(&request.ranking, &lexical)?
            } else {
                this.state
                    .unit_corpus
                    .expand_scores(&request.ranking, &lexical)?
            };
            let mut numerical = scores.rows().to_vec();
            let _conversion = budget.reserve(
                "ranking-vector-conversion",
                this.occurrences(&request).len() * 1024,
            )?;
            if let Some(vectors) = vectors {
                let binding = request
                    .ranking
                    .channel(Channel::Vector)
                    .ok_or(Error::Contract)?;
                let values: BTreeMap<_, _> = vectors.values().iter().copied().collect();
                for occurrence in this.occurrences(&request) {
                    let mut value = None;
                    for row in this
                        .state
                        .data
                        .uses
                        .iter()
                        .filter(|u| u.fragment == occurrence.fragment)
                    {
                        if let Some(score) = values.get(&row.id()) {
                            if value.is_some_and(|old| old != *score) {
                                return Err(Error::Contract);
                            }
                            value = Some(*score);
                        }
                    }
                    numerical.push(NumericalScore {
                        generation: request.ranking.generation(),
                        occurrence: *occurrence,
                        channel: Channel::Vector,
                        channel_identity: binding.identity(),
                        score: value,
                    });
                }
            }
            let ranked = request.ranking.rank(&numerical, &request.promoted)?;
            Ok((request, ranked))
        })
        .await
    }
    fn page_binding(
        &self,
        request: &Request,
        channels: &ChannelState,
    ) -> Result<CursorBinding, Error> {
        Ok(CursorBinding {
            generation: self.state.catalog.generation(),
            request: request.canonical_identity().map_err(wire_error)?,
            policy: RankingPolicy::default().identity()?,
            wire: wire_identity(),
            channels: channels.identity(),
            group: name("ranked")?,
            section: name(request.tool().name())?,
            member: None,
            ordering: ContentHash::of(b"canonical-family-rrf/score-target/v1"),
        })
    }
    fn page<T>(
        &self,
        request: &Request,
        channels: &ChannelState,
        rows: Vec<T>,
    ) -> Result<SectionPage<T>, Error> {
        let size = request.page().size;
        if size == 0 || size > ResourceLimits::default().maximum_page_rows {
            return Err(Error::ResourceRefused("ranking page rows"));
        }
        let binding = self.page_binding(request, channels)?;
        let offset = match &request.page().cursor.0 {
            Some(token) => Cursor::decode(token, &binding).map_err(wire_error)?.offset,
            None => 0,
        };
        let offset = usize::try_from(offset).map_err(|_| Error::Contract)?;
        if offset > rows.len() {
            return Err(Error::Contract);
        }
        let total = rows.len();
        let end = offset.saturating_add(size as usize).min(total);
        let continuation = if end < total {
            Optional(Some(
                Cursor {
                    binding,
                    offset: end as u64,
                }
                .encode()
                .map_err(wire_error)?,
            ))
        } else {
            Optional::default()
        };
        Ok(SectionPage {
            availability: Availability::Available {},
            items: rows.into_iter().skip(offset).take(end - offset).collect(),
            continuation,
            omitted: (total - end) as u64,
            truncated: end < total,
        })
    }
    pub async fn finish(
        &self,
        e: &RequestExecution,
        request: Request,
        prepared: RankingRequest,
        ranked: ranking::RankedResults,
    ) -> Result<Response, Error> {
        self.state.catalog.check_execution(e)?;
        if request.canonical_identity().map_err(wire_error)? != prepared.identity {
            return Err(Error::Contract);
        }
        let channels = prepared.channels.clone();
        let generation = self.state.catalog.generation();
        let mut ranking = ranked.rows().to_vec();
        let response = match &request {
            Request::SearchOperations(_) => {
                let mut results = Vec::new();
                for hit in &ranking {
                    let Target::Member { member } = hit.target else {
                        return Err(Error::Contract);
                    };
                    results.extend(
                        prepared
                            .candidates
                            .iter()
                            .filter(|c| c.member == member)
                            .cloned(),
                    );
                }
                let results = self.page(&request, &channels, results)?;
                let selected = results
                    .items
                    .iter()
                    .map(|r| r.member)
                    .collect::<BTreeSet<_>>();
                ranking.retain(
                    |r| matches!(r.target,Target::Member{member}if selected.contains(&member)),
                );
                Response::SearchOperations(SearchOperationsResponse {
                    generation,
                    extent: SelectionExtent::Ranked {
                        returned: results.items.len() as u64,
                    },
                    results,
                    channels,
                    ranking,
                })
            }
            Request::SearchEvidence(_) => {
                let targets = self.page(
                    &request,
                    &channels,
                    ranking.iter().map(|h| h.target).collect(),
                )?;
                let mut items = Vec::new();
                for target in &targets.items {
                    let Target::Unit { unit } = target else {
                        return Err(Error::Contract);
                    };
                    let row = self.state.data.units.get(*unit).ok_or(Error::Contract)?;
                    let mut originals = Vec::new();
                    for anchor in self.state.data.anchors.iter().filter(|a| a.unit == *unit) {
                        originals.push(
                            self.state
                                .catalog
                                .service()
                                .original_range(
                                    e,
                                    OriginalReference::Anchor {
                                        anchor: anchor.id(),
                                    },
                                    Some(row.context),
                                )
                                .await?,
                        );
                    }
                    let members = self
                        .state
                        .data
                        .unit_subjects
                        .iter()
                        .filter(|s| s.unit == *unit)
                        .filter_map(|s| match self.state.data.subjects.get(s.subject) {
                            Some(Subject::Member { member }) => Some(*member),
                            _ => None,
                        })
                        .collect::<BTreeSet<_>>();
                    items.push(EvidenceHit {
                        unit: *unit,
                        family: row.family,
                        title: name(row.title.as_str())?,
                        originals,
                        associated_members: members.into_iter().collect(),
                    });
                }
                ranking.retain(|r| targets.items.contains(&r.target));
                let results = SectionPage {
                    availability: targets.availability,
                    items,
                    continuation: targets.continuation,
                    omitted: targets.omitted,
                    truncated: targets.truncated,
                };
                Response::SearchEvidence(SearchEvidenceResponse {
                    generation,
                    extent: SelectionExtent::Ranked {
                        returned: results.items.len() as u64,
                    },
                    results,
                    channels,
                    ranking,
                })
            }
            Request::SearchCapabilities(_) => {
                let targets = self.page(
                    &request,
                    &channels,
                    ranking.iter().map(|h| h.target).collect(),
                )?;
                let mut items = Vec::new();
                for target in &targets.items {
                    let Target::Unit { unit } = target else {
                        return Err(Error::Contract);
                    };
                    let row = self.state.data.units.get(*unit).ok_or(Error::Contract)?;
                    let Some(Origin::Brief { brief }) = self.state.data.origins.get(row.origin)
                    else {
                        return Err(Error::Contract);
                    };
                    items.push(
                        self.state
                            .catalog
                            .service()
                            .capability(
                                e,
                                &GetCapabilityRequest {
                                    capability: *brief,
                                    page: request.page().clone(),
                                },
                            )
                            .await?
                            .capability,
                    );
                }
                ranking.retain(|r| targets.items.contains(&r.target));
                let results = SectionPage {
                    availability: targets.availability,
                    items,
                    continuation: targets.continuation,
                    omitted: targets.omitted,
                    truncated: targets.truncated,
                };
                Response::SearchCapabilities(SearchCapabilitiesResponse {
                    generation,
                    extent: SelectionExtent::Ranked {
                        returned: results.items.len() as u64,
                    },
                    results,
                    channels,
                    ranking,
                })
            }
            _ => return Err(Error::Contract),
        };
        let this = self.clone();
        let response = e
            .cpu(move |budget| {
                // Reserve encoding scratch before checking the complete structured and MCP body.
                let length = response.json_len().map_err(wire_error)?;
                let _encoding = budget.reserve(
                    "ranked response encoding",
                    length.checked_mul(8).ok_or(Error::Contract)?,
                )?;
                this.fit_response(&request, response)
            })
            .await?;
        super::catalog_service::retain(e, &response)?;
        e.confirm().await?;
        Ok(response)
    }
    fn fit_response(&self, request: &Request, mut response: Response) -> Result<Response, Error> {
        loop {
            let encoded = response.to_json().map_err(wire_error)?;
            match tool_result(request.tool().name(), &encoded, request.page().expanded) {
                Ok(_) => return Ok(response),
                Err(WireError::ResourceRefused(_)) => {}
                Err(error) => return Err(wire_error(error)),
            }
            // A result row is indivisible. Remove whole rows and bind the next cursor to
            // exactly the number actually returned, rather than the originally requested size.
            match &mut response {
                Response::SearchOperations(r) => {
                    self.trim_page(request, &r.channels, &mut r.results)?;
                    let members = r
                        .results
                        .items
                        .iter()
                        .map(|r| r.member)
                        .collect::<BTreeSet<_>>();
                    r.ranking.retain(
                        |r| matches!(r.target,Target::Member{member}if members.contains(&member)),
                    );
                    r.extent = SelectionExtent::Ranked {
                        returned: r.results.items.len() as u64,
                    };
                }
                Response::SearchEvidence(r) => {
                    self.trim_page(request, &r.channels, &mut r.results)?;
                    let units = r
                        .results
                        .items
                        .iter()
                        .map(|r| r.unit)
                        .collect::<BTreeSet<_>>();
                    r.ranking
                        .retain(|r| matches!(r.target,Target::Unit{unit}if units.contains(&unit)));
                    r.extent = SelectionExtent::Ranked {
                        returned: r.results.items.len() as u64,
                    };
                }
                Response::SearchCapabilities(r) => {
                    // Ranking associates original brief units with authored packets in order.
                    let removed = r
                        .results
                        .items
                        .len()
                        .checked_sub(1)
                        .ok_or(Error::Contract)?;
                    self.trim_page(request, &r.channels, &mut r.results)?;
                    r.ranking.truncate(removed);
                    r.extent = SelectionExtent::Ranked {
                        returned: r.results.items.len() as u64,
                    };
                }
                _ => return Err(Error::Contract),
            }
        }
    }
    fn trim_page<T>(
        &self,
        request: &Request,
        channels: &ChannelState,
        page: &mut SectionPage<T>,
    ) -> Result<(), Error> {
        // Returning an empty page for an indivisible oversized first row would imply absence.
        if page.items.len() <= 1 {
            return Err(Error::ResourceRefused("indivisible ranked result"));
        }
        page.items.pop();
        page.omitted = page.omitted.checked_add(1).ok_or(Error::Contract)?;
        page.truncated = true;
        page.availability = Availability::Partial {
            reason: name("response byte bound")?,
        };
        let binding = self.page_binding(request, channels)?;
        let offset = match &request.page().cursor.0 {
            Some(token) => Cursor::decode(token, &binding).map_err(wire_error)?.offset,
            None => 0,
        };
        let end = offset
            .checked_add(page.items.len() as u64)
            .ok_or(Error::Contract)?;
        page.continuation = Optional(Some(
            Cursor {
                binding,
                offset: end,
            }
            .encode()
            .map_err(wire_error)?,
        ));
        Ok(())
    }
    fn occurrences<'a>(&self, r: &'a RankingRequest) -> &'a [Occurrence] {
        &r.occurrences
    }
}
