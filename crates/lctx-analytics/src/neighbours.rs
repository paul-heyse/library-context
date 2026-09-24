//! Embeddings in analytics (DESIGN §9.7; slice 3.1): exact nearest neighbours between the
//! subsystem's public APIs and the corpus passages, over vectors from the embedding cache (E0).
//!
//! - **Texts.** A passage is its text; an API is `path(parameters)` and its docstring. Each text is
//!   cut into windows of at most `window_bytes` at line ends (a longer line at character
//!   boundaries), so nothing is dropped; an item's similarity to another is the best cosine over
//!   their window pairs (every vector is a unit vector, so the dot product).
//! - **Doc links.** Each API's `k` most similar passages at or above `min_similarity`, ties to the
//!   smaller passage id: `doc_link` findings.
//! - **Community labels.** Each community's centroid (the normalized mean of its public members'
//!   window vectors) takes the heading of its most similar passage, at the same floor:
//!   `community_label` findings.
//!
//! Both are `statistically_derived`: they order and link, never state behaviour, and never feed an
//! Outcome (§10.3).

use std::collections::BTreeMap;

use cpg_schema::codebook::{Codebook, CoverageStatus, FindingKind, MemberRole};
use cpg_schema::findings::{
    FINDING_STATUS, FindingMembersRow, FindingsRow, MemberKey, recipe::FindingKey,
};
use cpg_schema::id::{Digest, Id, content_digest};
use serde::Serialize;

use crate::AnalyticsError;

/// The pre-registered parameters (slice 3.1; deviation log D35), frozen with the others.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Params {
    /// Doc links per API.
    pub k: usize,
    /// The least cosine for a link or a label.
    pub min_similarity: f64,
    /// A window's most bytes: well under the spec's 2,048-token document cap.
    pub window_bytes: usize,
    /// The API-text form's version (`api_text`).
    pub text_version: u32,
}

impl Params {
    pub fn preregistered() -> Self {
        Params {
            k: 3,
            min_similarity: 0.5,
            window_bytes: 4096,
            text_version: 1,
        }
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("parameters serialize")
    }

    pub fn digest(&self) -> Digest {
        content_digest(self.json().as_bytes())
    }
}

/// An API's text: `path(parameters)`, then its docstring (text version 1).
pub fn api_text(path: &str, parameters: Option<&str>, docstring: Option<&str>) -> String {
    let mut text = format!("{path}({})", parameters.unwrap_or_default());
    if let Some(doc) = docstring.map(str::trim).filter(|d| !d.is_empty()) {
        text.push('\n');
        text.push_str(doc);
    }
    text
}

/// `text` cut into windows of at most `cap` bytes at line ends; a longer line is cut at character
/// boundaries. Concatenated, the windows are the text: nothing is dropped.
pub fn windows(text: &str, cap: usize) -> Vec<String> {
    let cap = cap.max(4);
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.split_inclusive('\n') {
        if current.len() + line.len() <= cap {
            current.push_str(line);
            continue;
        }
        if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        let mut rest = line;
        while rest.len() > cap {
            let mut cut = cap;
            while !rest.is_char_boundary(cut) {
                cut -= 1;
            }
            out.push(rest[..cut].to_owned());
            rest = &rest[cut..];
        }
        current.push_str(rest);
    }
    if !current.is_empty() || out.is_empty() {
        out.push(current);
    }
    out
}

/// An embedded item: its node and each window's unit vector.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub node: Id,
    pub vectors: Vec<Vec<f32>>,
}

fn dot(a: &[f32], b: &[f32]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum()
}

/// The best cosine over two items' window pairs.
pub fn similarity(a: &Item, b: &Item) -> f64 {
    a.vectors
        .iter()
        .flat_map(|x| b.vectors.iter().map(move |y| dot(x, y)))
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Each query's `k` most similar targets at or above `min`, most similar first, ties to the
/// smaller target node.
pub fn nearest(queries: &[Item], targets: &[Item], k: usize, min: f64) -> Vec<Vec<(usize, f64)>> {
    queries
        .iter()
        .map(|q| {
            let mut scored: Vec<(usize, f64)> = targets
                .iter()
                .enumerate()
                .map(|(t, target)| (t, similarity(q, target)))
                .filter(|(_, s)| *s >= min)
                .collect();
            scored.sort_by(|a, b| {
                b.1.total_cmp(&a.1)
                    .then(targets[a.0].node.cmp(&targets[b.0].node))
            });
            scored.truncate(k);
            scored
        })
        .collect()
}

/// The normalized mean of the items' window vectors, as a one-window item.
pub fn centroid(node: Id, items: &[&Item]) -> Option<Item> {
    let dims = items.iter().flat_map(|i| i.vectors.first()).next()?.len();
    let mut sum = vec![0f64; dims];
    for v in items.iter().flat_map(|i| &i.vectors) {
        for (s, x) in sum.iter_mut().zip(v) {
            *s += f64::from(*x);
        }
    }
    let norm = sum.iter().map(|x| x * x).sum::<f64>().sqrt();
    (norm > 0.0).then(|| Item {
        node,
        vectors: vec![sum.iter().map(|x| (x / norm) as f32).collect()],
    })
}

/// A passage: its item, and the heading a label shows (its document path when it has none).
#[derive(Debug, Clone, PartialEq)]
pub struct Passage {
    pub item: Item,
    pub label: String,
}

#[derive(Debug, Serialize)]
struct Diagnostics {
    apis: usize,
    passages: usize,
    api_windows: usize,
    passage_windows: usize,
    doc_links: usize,
    apis_linked: usize,
    communities: usize,
    labelled: usize,
}

/// The kNN results.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub candidate_pairs: usize,
    pub diagnostics: String,
    pub findings: Vec<FindingsRow>,
    pub members: Vec<FindingMembersRow>,
}

/// Doc links from `apis` to `passages`, and a label for each community `(subject, members)`.
pub fn run(
    apis: &[Item],
    passages: &[Passage],
    communities: &[(Id, Vec<Id>)],
    params: &Params,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<Outcome, AnalyticsError> {
    let status = |kind: FindingKind| {
        FINDING_STATUS
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, s)| *s)
            .ok_or_else(|| AnalyticsError::Graph(format!("no status policy for {kind:?}")))
    };
    let targets: Vec<Item> = passages.iter().map(|p| p.item.clone()).collect();
    let mut findings = Vec::new();
    let mut members = Vec::new();
    let mut push = |kind: FindingKind,
                    subject: Id,
                    related: Id,
                    score: f64,
                    rows: Vec<(MemberRole, String)>|
     -> Result<(), AnalyticsError> {
        let status = status(kind)?;
        let keys: Vec<MemberKey> = rows
            .iter()
            .enumerate()
            .map(|(ordinal, (role, label))| MemberKey {
                role: role.code(),
                ordinal: ordinal as i64,
                node: None,
                cited_fact: None,
                label: Some(label.clone()),
            })
            .collect();
        let finding_id = FindingKey {
            finding_kind: kind.code(),
            subject,
            related: Some(related),
            condition: None,
            evidence_status: status.code(),
            depth: None,
            stop_reason: None,
            witnesses_omitted: false,
            paths: &[],
            members: &keys,
        }
        .id();
        for (ordinal, (role, label)) in rows.into_iter().enumerate() {
            members.push(FindingMembersRow {
                snapshot_id,
                finding_id,
                role,
                ordinal: ordinal as i64,
                node_id: None,
                cited_fact_id: None,
                label: Some(label),
                weight: None,
            });
        }
        findings.push(FindingsRow {
            snapshot_id,
            finding_id,
            invocation_id,
            finding_kind: kind,
            subject_node_id: subject,
            related_node_id: Some(related),
            evidence_status: status,
            depth: None,
            stop_reason: None,
            witnesses_omitted: false,
            score: Some(score),
            condition_node_id: None,
        });
        Ok(())
    };
    let links = nearest(apis, &targets, params.k, params.min_similarity);
    let mut linked = 0usize;
    let mut doc_links = 0usize;
    for (api, found) in apis.iter().zip(&links) {
        linked += usize::from(!found.is_empty());
        for &(t, score) in found {
            push(
                FindingKind::DocLink,
                api.node,
                passages[t].item.node,
                score,
                vec![(MemberRole::Label, passages[t].label.clone())],
            )?;
            doc_links += 1;
        }
    }
    let by_node: BTreeMap<Id, &Item> = apis.iter().map(|a| (a.node, a)).collect();
    let mut labelled = 0usize;
    for (subject, community) in communities {
        let items: Vec<&Item> = community
            .iter()
            .filter_map(|m| by_node.get(m).copied())
            .collect();
        let Some(center) = centroid(*subject, &items) else {
            continue;
        };
        if let Some(&(t, score)) = nearest(&[center], &targets, 1, params.min_similarity)
            .first()
            .and_then(|v| v.first())
        {
            push(
                FindingKind::CommunityLabel,
                *subject,
                passages[t].item.node,
                score,
                vec![(MemberRole::Label, passages[t].label.clone())],
            )?;
            labelled += 1;
        }
    }
    let diagnostics = serde_json::to_string(&Diagnostics {
        apis: apis.len(),
        passages: passages.len(),
        api_windows: apis.iter().map(|a| a.vectors.len()).sum(),
        passage_windows: targets.iter().map(|t| t.vectors.len()).sum(),
        doc_links,
        apis_linked: linked,
        communities: communities.len(),
        labelled,
    })
    .expect("diagnostics serialize");
    Ok(Outcome {
        candidate_pairs: apis.len() * passages.len(),
        diagnostics,
        findings,
        members,
    })
}

/// kNN is a complete, exact search under its stated model.
pub const COMPLETION: CoverageStatus = CoverageStatus::CompleteUnderStatedModel;

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Id {
        Id([n; 16])
    }

    fn unit(v: &[f32]) -> Vec<f32> {
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter().map(|x| x / n).collect()
    }

    #[test]
    fn windows_never_drop_text() {
        let text = "one\ntwo two\nthree three three\n";
        let w = windows(text, 8);
        assert_eq!(w.concat(), text);
        assert!(w.iter().all(|x| x.len() <= 8), "{w:?}");
        let long = "é".repeat(10);
        let w = windows(&long, 5);
        assert_eq!(w.concat(), long);
        assert!(w.iter().all(|x| x.len() <= 5));
        assert_eq!(windows("", 8), vec![String::new()]);
    }

    #[test]
    fn nearest_is_exact_thresholded_and_breaks_ties_by_node() {
        let q = Item {
            node: id(1),
            vectors: vec![unit(&[1.0, 0.0])],
        };
        let targets = vec![
            Item {
                node: id(9),
                vectors: vec![unit(&[1.0, 1.0])],
            },
            Item {
                node: id(3),
                vectors: vec![unit(&[1.0, 1.0])],
            },
            Item {
                node: id(4),
                // Its second window matches exactly: an item takes its best window.
                vectors: vec![unit(&[0.0, 1.0]), unit(&[1.0, 0.0])],
            },
            Item {
                node: id(5),
                vectors: vec![unit(&[-1.0, 0.1])],
            },
        ];
        let found = nearest(&[q], &targets, 3, 0.5);
        let order: Vec<Id> = found[0].iter().map(|(t, _)| targets[*t].node).collect();
        assert_eq!(order, vec![id(4), id(3), id(9)]);
        assert!((found[0][0].1 - 1.0).abs() < 1e-6);
        assert!(nearest(&[targets[3].clone()], &targets[..2], 3, 0.5)[0].is_empty());
    }

    #[test]
    fn a_community_is_labelled_by_its_centroids_nearest_heading() {
        let apis = vec![
            Item {
                node: id(1),
                vectors: vec![unit(&[1.0, 0.2])],
            },
            Item {
                node: id(2),
                vectors: vec![unit(&[1.0, -0.2])],
            },
        ];
        let passages = vec![
            Passage {
                item: Item {
                    node: id(7),
                    vectors: vec![unit(&[1.0, 0.0])],
                },
                label: "Tools".to_owned(),
            },
            Passage {
                item: Item {
                    node: id(8),
                    vectors: vec![unit(&[0.0, 1.0])],
                },
                label: "Other".to_owned(),
            },
        ];
        let out = run(
            &apis,
            &passages,
            &[(id(1), vec![id(1), id(2)])],
            &Params::preregistered(),
            Id::ZERO,
            Id::ZERO,
        )
        .unwrap();
        let label = out
            .findings
            .iter()
            .find(|f| f.finding_kind == FindingKind::CommunityLabel)
            .unwrap();
        assert_eq!(label.related_node_id, Some(id(7)));
        assert!(
            out.members
                .iter()
                .any(|m| m.label.as_deref() == Some("Tools"))
        );
        // Each API links "Tools" (cos ≈ 0.98) and not "Other" (≈ 0.2 or less).
        let links: Vec<_> = out
            .findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::DocLink)
            .collect();
        assert_eq!(links.len(), 2);
        assert!(links.iter().all(|f| f.related_node_id == Some(id(7))));
    }
}
