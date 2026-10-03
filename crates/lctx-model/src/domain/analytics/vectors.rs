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
#[allow(
    clippy::too_many_arguments,
    reason = "Public operation keeps the analytic frame, structural owner, selected domain, independent result layers and budget explicit."
)]
pub fn produce(
    d: &Data,
    f: &AnalyticFrame,
    parent: &parent::Invocation,
    public: &BTreeSet<Id<EntityRef>>,
    r: &mut TechniqueResult,
    layer_invocation: Option<Id<analysis::analytic::Invocation>>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<Option<neighbours::Prepared>, ModelError> {
    let ordinary = r.selected;
    if !ordinary && layer_invocation.is_none() {
        return Ok(None);
    }
    let mut calculation = r.clone();
    calculation.selected = true;
    calculation.status = analysis::AnalysisStatus::Completed;
    calculation.stop = Stop::Converged;
    let layer = layer_invocation.map(|invocation| LayerResult {
        frame: f.id(),
        layer: Layer::Nearest,
        invocation,
        status: calculation.status,
        stop: calculation.stop,
        examined: 0,
    });
    let prepared = calculate(
        d,
        VectorSelectionInputs { f, parent, public },
        &mut calculation,
        ordinary,
        layer.as_ref(),
        out,
        b,
    )?;
    if let Some(mut layer) = layer {
        layer.status = calculation.status;
        layer.stop = calculation.stop;
        layer.examined = calculation.examined;
        out.layer_results.insert(layer)?;
    }
    if ordinary {
        *r = calculation;
    }
    Ok(prepared)
}
struct VectorSelectionInputs<'a> {
    f: &'a AnalyticFrame,
    parent: &'a parent::Invocation,
    public: &'a BTreeSet<Id<EntityRef>>,
}

fn calculate(
    d: &Data,
    vector_selection: VectorSelectionInputs<'_>,
    r: &mut TechniqueResult,
    ordinary: bool,
    layer: Option<&LayerResult>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<Option<neighbours::Prepared>, ModelError> {
    let VectorSelectionInputs { f, parent, public } = vector_selection;
    if !r.selected {
        return Ok(None);
    }
    if !d.vectors.selected()? {
        r.status = analysis::AnalysisStatus::Unavailable;
        r.stop = Stop::VectorsUnavailable;
        return Ok(None);
    }
    let prepared = neighbours::Prepared::admit(
        &d.vectors,
        &d.embedding_invocations,
        &d.embedding_outcomes,
        &d.embedding_uses,
        b,
    )
    .map_err(err)?;
    let _selectors = b.reserve(
        "analytic-vector-domain",
        prepared
            .items()
            .len()
            .checked_mul(1024)
            .ok_or_else(|| invalid("vector domain overflow"))?,
    )?;
    let mut api = BTreeMap::new();
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
    let covered = api.values().copied().collect::<BTreeSet<_>>();
    let keys = api.keys().copied().collect::<Vec<_>>();
    let targets = documents.iter().copied().collect::<Vec<_>>();
    let floor = FiniteF64::new(0.5)?;
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
                let replace = best
                    .get(&(query, target))
                    .is_none_or(|old: &&neighbours::Match| {
                        m.score.get() > old.score.get()
                            || (m.score == old.score
                                && (m.query_window, m.target_window, m.query_use, m.target_use)
                                    < (
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
                for (_, m) in rows.into_iter().take(3) {
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
    Ok(Some(prepared))
}
pub fn labels(
    d: &Data,
    f: &AnalyticFrame,
    parent: &parent::Invocation,
    p: Option<&neighbours::Prepared>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let Some(p) = p else {
        for c in out.communities.iter().cloned().collect::<Vec<_>>() {
            out.label_assessments.insert(CommunityLabelAssessment {
                community: c.id(),
                status: analysis::AnalysisStatus::Unavailable,
                stop: Stop::VectorsUnavailable,
            })?;
        }
        return Ok(());
    };
    let _selectors = b.reserve(
        "analytic-label-selectors",
        p.items()
            .len()
            .checked_mul(512)
            .ok_or_else(|| invalid("label selectors overflow"))?,
    )?;
    let mut api = BTreeMap::new();
    let mut documents = vec![];
    for item in p.items() {
        if !frame_item(d, item, parent)? {
            continue;
        }
        let a = assessment(d, item, parent)?;
        match need(&d.text_subjects, item.key.subject)? {
            embedding::text::TextSubject::Declaration { .. } => {
                if let Some(entity) = a.entity {
                    api.insert(item.key, entity);
                }
            }
            embedding::text::TextSubject::Passage { .. } => {
                if d.uses.iter().any(|u| {
                    u.input == parent.input
                        && u.artifact == a.source
                        && u.role == input::SourceRole::Document
                }) {
                    documents.push(item.key);
                }
            }
        }
    }
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
        let matched = match p.centroid(&keys, &documents, FiniteF64::new(0.5)?, build::MAX_WORK) {
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
