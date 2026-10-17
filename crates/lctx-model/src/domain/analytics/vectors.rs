//! API pairs, original passage links and community labels share exact admitted E1 values.
use super::{
    build::{Data, invalid, need},
    *,
};
use crate::domain::{
    analysis::structural as parent, normalized::entities::EntityRef, resources::ResourceBudget,
};
use std::collections::{BTreeMap, BTreeSet};
fn err(e: neighbours::Error) -> ModelError {
    invalid(e.to_string())
}
fn frame_item(
    d: &Data,
    item: &neighbours::Item,
    parent: &parent::Invocation,
) -> Result<bool, ModelError> {
    Ok(item.context == parent.context
        && need(&d.embedding_invocations, item.key.invocation)?.input == parent.input)
}
fn assessment<'a>(
    d: &'a Data,
    item: &neighbours::Item,
    parent: &parent::Invocation,
) -> Result<&'a embedding::text::TextAssessment, ModelError> {
    let mut rows = d.vectors.assessments.iter().filter(|a| {
        a.subject == item.key.subject && a.input == parent.input && a.context == parent.context
    });
    let a = rows
        .next()
        .ok_or_else(|| invalid("vector selection has no exact text assessment"))?;
    if rows.next().is_some() {
        return Err(invalid("ambiguous original text assessment"));
    }
    Ok(a)
}
pub(super) struct Prepared {
    pub(super) values: Option<std::sync::Arc<neighbours::Prepared>>,
    api: BTreeMap<neighbours::ItemKey, Id<EntityRef>>,
    label_api: BTreeMap<neighbours::ItemKey, Id<EntityRef>>,
    documents: BTreeSet<neighbours::ItemKey>,
    public: BTreeSet<Id<EntityRef>>,
    incomplete: bool,
    _charge: Option<Box<dyn resources::Reservation>>,
}
pub(super) struct Preparation {
    prepared: std::sync::Arc<Prepared>,
    result: TechniqueResult,
    ordinary: bool,
    layer: Option<LayerResult>,
    out: Output,
    budget: ResourceBudget,
}
pub(super) struct Computed {
    pub(super) prepared: std::sync::Arc<Prepared>,
    pub(super) out: Output,
}
impl Preparation {
    pub(super) fn new(
        d: &Data,
        f: &AnalyticFrame,
        parent: &parent::Invocation,
        public: &BTreeSet<Id<EntityRef>>,
        r: &TechniqueResult,
        layer_invocation: Option<Id<analysis::analytic::Invocation>>,
        b: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let ordinary = r.selected;
        let mut result = r.clone();
        let mut layer = layer_invocation.map(|invocation| LayerResult {
            frame: f.id(),
            layer: Layer::Nearest,
            invocation,
            status: analysis::AnalysisStatus::Completed,
            stop: Stop::Converged,
            examined: 0,
        });
        let mut out = Output::new(b);
        if !ordinary && layer.is_none() {
            return Ok(Self {
                prepared: std::sync::Arc::new(Prepared {
                    values: None,
                    api: BTreeMap::new(),
                    label_api: BTreeMap::new(),
                    documents: BTreeSet::new(),
                    public: BTreeSet::new(),
                    incomplete: false,
                    _charge: None,
                }),
                result,
                ordinary,
                layer,
                out,
                budget: b.clone(),
            });
        }
        if !d.vectors.selected()? {
            if ordinary {
                result.status = analysis::AnalysisStatus::Unavailable;
                result.stop = Stop::VectorsUnavailable;
            }
            if let Some(layer) = &mut layer {
                layer.status = analysis::AnalysisStatus::Unavailable;
                layer.stop = Stop::VectorsUnavailable;
            }
            return Ok(Self {
                prepared: std::sync::Arc::new(Prepared {
                    values: None,
                    api: BTreeMap::new(),
                    label_api: BTreeMap::new(),
                    documents: BTreeSet::new(),
                    public: BTreeSet::new(),
                    incomplete: false,
                    _charge: None,
                }),
                result,
                ordinary,
                layer,
                out,
                budget: b.clone(),
            });
        }
        let prepared = neighbours::Prepared::admit(
            &d.vectors,
            &d.embedding_invocations,
            &d.embedding_outcomes,
            &d.embedding_uses,
            b,
        )
        .map_err(err)?;
        let selectors = b.reserve(
            "analytic-vector-domain",
            prepared
                .items()
                .len()
                .checked_mul(1024)
                .and_then(|n| n.checked_add(public.len().checked_mul(128)?))
                .ok_or_else(|| invalid("vector domain overflow"))?,
        )?;
        let mut api = BTreeMap::new();
        let mut label_api = BTreeMap::new();
        let mut documents = BTreeSet::new();
        let mut incomplete = false;
        for item in prepared.items() {
            if !frame_item(d, item, parent)? {
                continue;
            }
            let a = assessment(d, item, parent)?;
            let subject = need(&d.text_subjects, item.key.subject)?;
            let entity = match subject {
                embedding::text::TextSubject::Declaration { .. } => {
                    let Some(entity) = a.entity else {
                        continue;
                    };
                    label_api.insert(item.key, entity);
                    if !public.contains(&entity) {
                        continue;
                    }
                    api.insert(item.key, entity);
                    Some(entity)
                }
                embedding::text::TextSubject::Passage { .. } => {
                    if !d.uses.iter().any(|u| {
                        u.input == parent.input
                            && u.artifact == a.source
                            && u.role == input::SourceRole::Document
                    }) {
                        continue;
                    }
                    documents.insert(item.key);
                    None
                }
            };
            if entity.is_some() || ordinary {
                incomplete |= item.availability != neighbours::Availability::Complete;
            }
            out.vectors.insert(VectorSelection {
                frame: f.id(),
                entity,
                subject: item.key.subject,
                invocation: item.key.invocation,
                expected_windows: item.expected_windows as i64,
                available_windows: item.available_windows as i64,
            })?;
        }
        let prepared = std::sync::Arc::new(Prepared {
            values: Some(std::sync::Arc::new(prepared)),
            api,
            label_api,
            documents,
            public: public.clone(),
            incomplete,
            _charge: Some(selectors),
        });
        Ok(Self {
            prepared,
            result,
            ordinary,
            layer,
            out,
            budget: b.clone(),
        })
    }
    pub(super) fn run(self) -> Result<Computed, ModelError> {
        let Self {
            prepared: owner,
            mut result,
            ordinary,
            mut layer,
            mut out,
            budget: b,
        } = self;
        let mut calculation = result.clone();
        if let Some(prepared) = &owner.values {
            calculation.selected = true;
            calculation.status = analysis::AnalysisStatus::Completed;
            calculation.stop = Stop::Converged;
            let public = &owner.public;
            let api = &owner.api;
            let documents = &owner.documents;
            let incomplete = owner.incomplete;
            let r = &mut calculation;
            let layer = layer.as_ref();
            let b = &b;
            let covered = api.values().copied().collect::<BTreeSet<_>>();
            let keys = api.keys().copied().collect::<Vec<_>>();
            let targets = documents.iter().copied().collect::<Vec<_>>();
            let floor = FiniteF64::new(super::policy::RETAINED.neighbour_floor)?;
            if incomplete || covered != *public {
                r.status = analysis::AnalysisStatus::Partial;
                r.stop = Stop::VectorsUnavailable;
            }
            if public.is_empty() {
                r.stop = Stop::EmptyDomain;
            } else if covered.is_empty() {
                r.status = analysis::AnalysisStatus::Unavailable;
                r.stop = Stop::VectorsUnavailable;
            }
            if !keys.is_empty() {
                let p = neighbours::Parameters {
                    k: keys.len().max(targets.len()).max(1),
                    min_similarity: floor,
                    max_window_pairs: build::MAX_WORK.saturating_sub(r.examined as u64),
                    exclude_self: true,
                };
                for docs in [false, true] {
                    if docs && !ordinary {
                        continue;
                    }
                    let target = if docs { &targets } else { &keys };
                    let matches = match prepared.nearest(&keys, target, p) {
                        Ok(m) => m,
                        Err(neighbours::Error::WorkLimit { .. }) => {
                            r.status = analysis::AnalysisStatus::Partial;
                            r.stop = Stop::WorkLimit;
                            continue;
                        }
                        Err(e) => return Err(err(e)),
                    };
                    r.examined = r
                        .examined
                        .checked_add(matches.window_pairs() as i64)
                        .ok_or_else(|| invalid("neighbour work overflow"))?;
                    if matches.incomplete() {
                        r.status = analysis::AnalysisStatus::Partial;
                        r.stop = Stop::VectorsUnavailable;
                    }
                    let _group = b.reserve(
                        "analytic-neighbour-collapse",
                        matches
                            .rows()
                            .len()
                            .checked_mul(768)
                            .ok_or_else(|| invalid("neighbour collapse overflow"))?,
                    )?;
                    let mut best = BTreeMap::new();
                    for m in matches.rows() {
                        let query = api[&m.query];
                        let target = if docs {
                            *m.target.subject.bytes()
                        } else {
                            *api[&m.target].bytes()
                        };
                        if !docs && query == api[&m.target] {
                            continue;
                        }
                        let replace =
                            best.get(&(query, target))
                                .is_none_or(|old: &&neighbours::Match| {
                                    m.score.get() > old.score.get()
                                        || (m.score == old.score
                                            && (
                                                m.query_window,
                                                m.target_window,
                                                m.query_use,
                                                m.target_use,
                                            ) < (
                                                old.query_window,
                                                old.target_window,
                                                old.query_use,
                                                old.target_use,
                                            ))
                                });
                        if replace {
                            best.insert((query, target), m);
                        }
                    }
                    for query in public {
                        let mut rows = best
                            .iter()
                            .filter(|((q, _), _)| q == query)
                            .map(|((_, target), m)| (*target, *m))
                            .collect::<Vec<_>>();
                        rows.sort_by(|(a, x), (c, y)| {
                            y.score
                                .get()
                                .total_cmp(&x.score.get())
                                .then_with(|| a.cmp(c))
                        });
                        for (_, m) in rows.into_iter().take(super::policy::RETAINED.neighbours) {
                            if docs {
                                out.document_neighbours.insert(DocumentNeighbour {
                                    result: r.id(),
                                    query: *query,
                                    subject: m.target.subject,
                                    query_use: m.query_use,
                                    target_use: m.target_use,
                                    score: m.score,
                                })?;
                            } else {
                                if ordinary {
                                    out.neighbours.insert(Neighbour {
                                        result: r.id(),
                                        query: *query,
                                        target: api[&m.target],
                                        query_use: m.query_use,
                                        target_use: m.target_use,
                                        score: m.score,
                                    })?;
                                }
                                if let Some(layer) = layer {
                                    out.layer_neighbours.insert(LayerNeighbour {
                                        result: layer.id(),
                                        query: *query,
                                        target: api[&m.target],
                                        query_use: m.query_use,
                                        target_use: m.target_use,
                                        score: m.score,
                                    })?;
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(ref mut layer) = layer {
            if owner.values.is_some() {
                layer.status = calculation.status;
                layer.stop = calculation.stop;
                layer.examined = calculation.examined;
            }
            out.layer_results.insert(layer.clone())?;
        }
        if ordinary {
            result = calculation;
        }
        out.results.insert(result)?;
        Ok(Computed {
            prepared: owner,
            out,
        })
    }
}

pub(super) fn labels(
    f: &AnalyticFrame,
    prepared: &Prepared,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let Some(p) = prepared.values.as_ref() else {
        for c in out.communities.iter().cloned().collect::<Vec<_>>() {
            out.label_assessments.insert(CommunityLabelAssessment {
                community: c.id(),
                status: analysis::AnalysisStatus::Unavailable,
                stop: Stop::VectorsUnavailable,
            })?;
        }
        return Ok(());
    };
    let api = &prepared.label_api;
    let documents = prepared.documents.iter().copied().collect::<Vec<_>>();
    let _selectors = b.reserve(
        "analytic-label-selectors",
        documents
            .len()
            .checked_mul(128)
            .and_then(|n| n.checked_add(api.len().checked_mul(128)?))
            .ok_or_else(|| invalid("label selectors overflow"))?,
    )?;
    for community in out
        .communities
        .iter()
        .filter(|c| out.results.get(c.result).is_some_and(|r| r.frame == f.id()))
        .cloned()
        .collect::<Vec<_>>()
    {
        let members = out
            .community_members
            .iter()
            .filter(|m| m.community == community.id())
            .map(|m| m.entity)
            .collect::<BTreeSet<_>>();
        let keys = api
            .iter()
            .filter(|(_, entity)| members.contains(entity))
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        let nearest = out
            .results
            .iter()
            .find(|r| r.frame == f.id() && r.method == analysis::AnalysisMethod::Neighbours)
            .ok_or_else(|| invalid("label neighbour result missing"))?;
        let mut a = CommunityLabelAssessment {
            community: community.id(),
            status: nearest.status,
            stop: nearest.stop,
        };
        let matched = match p.centroid(
            &keys,
            &documents,
            FiniteF64::new(super::policy::RETAINED.neighbour_floor)?,
            build::MAX_WORK,
        ) {
            Ok(v) => v,
            Err(neighbours::Error::WorkLimit { .. }) => {
                a.status = analysis::AnalysisStatus::Partial;
                a.stop = Stop::WorkLimit;
                out.label_assessments.insert(a)?;
                continue;
            }
            Err(e) => return Err(err(e)),
        };
        if let Some(m) = matched {
            let row = CommunityLabel {
                community: community.id(),
                subject: m.target.subject,
                target_use: m.target_use,
                members: build::digest("analytic-community-centroid", &m.members),
                score: m.score,
            };
            for use_ in &m.members {
                out.label_members.insert(CommunityLabelMember {
                    label: row.id(),
                    use_: *use_,
                })?;
            }
            out.labels.insert(row)?;
        }
        out.label_assessments.insert(a)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/fixtures/embedding_consumption.rs"]
mod consumption_fixture;
#[cfg(test)]
mod domain_controls {
    use super::consumption_fixture as fixture;
    use super::*;
    #[test]
    fn public_neighbour_domain_does_not_truncate_admitted_community_label_domain() {
        let b = ResourceBudget::fixed(16 << 20).unwrap();
        let mut fixture = fixture::fixture(&b, true);
        let subject = embedding::text::TextSubject::Declaration {
            declaration: fixture::id(7),
        };
        let mut assessment = fixture.data.assessments.iter().next().unwrap().clone();
        assessment.subject = subject.id();
        fixture.data.assessments = normalized::Rows::new(&b);
        fixture.data.assessments.insert(assessment.clone()).unwrap();
        fixture.window.assessment = assessment.id();
        fixture.data.windows = normalized::Rows::new(&b);
        fixture.data.windows.insert(fixture.window.clone()).unwrap();
        let (full, projection, published) = fixture::winner(&fixture, &b);
        fixture
            .data
            .values
            .admit_full(&full, &fixture.encoder, &fixture.policy)
            .unwrap();
        fixture.data.values.admit_projection(&projection).unwrap();
        fixture.data.projected_values.insert(projection).unwrap();
        let uses = fixture::consume(&fixture, &published, &b);
        let (invocations, outcomes) = fixture::outcomes(&fixture, &uses, &b);
        let mut d = Data::new(&b);
        d.vectors = fixture.data;
        d.embedding_invocations = invocations;
        d.embedding_outcomes = outcomes;
        d.embedding_uses = uses;
        d.text_subjects.insert(subject).unwrap();
        let parent = parent::Invocation::new(
            assessment.input,
            assessment.context,
            fixture::id(8),
            None,
            [],
        )
        .0;
        let frame = AnalyticFrame {
            structural: fixture::id(9),
            configuration: fixture::id(10),
        };
        let result = TechniqueResult {
            frame: frame.id(),
            method: analysis::AnalysisMethod::Neighbours,
            invocation: fixture::id(11),
            selected: true,
            status: analysis::AnalysisStatus::Completed,
            stop: Stop::Converged,
            iterations: 0,
            examined: 0,
            residual: None,
            input_partial: false,
        };
        let prepared =
            Preparation::new(&d, &frame, &parent, &BTreeSet::new(), &result, None, &b).unwrap();
        assert!(prepared.prepared.api.is_empty());
        assert_eq!(
            prepared
                .prepared
                .label_api
                .values()
                .copied()
                .collect::<Vec<_>>(),
            vec![fixture::id(4)]
        );
        assert_eq!(prepared.prepared.values.as_ref().unwrap().items().len(), 1);
        let output = prepared.run().unwrap();
        assert!(output.out.neighbours.is_empty());
        assert_eq!(
            output.out.results.iter().next().unwrap().stop,
            Stop::EmptyDomain
        );
        drop((output, d));
        assert_eq!(b.reserved(), 0);
    }
}
