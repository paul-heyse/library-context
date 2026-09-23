//! Pass A (DESIGN §9.1): which public API exposes the mechanism, and what does it already
//! coordinate?
//!
//! From each seed declaration, an explicit breadth-first search with parent pointers over the
//! invocation projection, under sorted adjacency (target id, then call site, then edge id; never
//! petgraph's walker order).
//! - **Witnesses.** The first witness to a target is the BFS path. Up to `max_witnesses - 1`
//!   alternatives are the next shortest paths that differ in their final arc, in canonical arc
//!   order. Parallel call sites are distinct arcs, so each can be a witness.
//!   `witnesses_omitted` is set when more existed (presentation only).
//! - **Boundaries.** The search stays inside the subsystem and never crosses a dependency or
//!   bundled definition, a synthetic callable or a release callable outside the subsystem: each
//!   such arc is an `implementation_boundary`. `potential` arcs are not in the projection.
//! - **Candidates.** An override-open (`candidate`) arc is followed, but a path through one is
//!   never a `direct_delegation` (§3.6).
//! - **Unresolved sites** of every expanded vertex are `incomplete_resolution` findings.
//! - **Budgets:** depth is the stated model (complete under it); a vertex or arc budget is
//!   operational truncation, so the invocation is `partial` (ADR-0019 review F4).

use std::collections::{BTreeMap, HashMap, VecDeque};

use cpg_schema::codebook::{
    Codebook, CoverageStatus, FindingKind, MemberRole, Modality, NodeKind, StopReason,
};
use cpg_schema::findings::recipe::FindingKey;
use cpg_schema::findings::{
    FINDING_STATUS, FindingMembersRow, FindingsRow, MemberKey, StepKey, WitnessesRow,
};
use cpg_schema::id::Id;
use fixedbitset::FixedBitSet;

use crate::AnalyticsError;
use crate::graph::Projection;

/// Pass A's budgets (§9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budgets {
    pub max_depth: u32,
    pub max_vertices: u32,
    pub max_edges: u32,
    pub max_witnesses: u32,
}

/// A seed: its declaration and the public access paths (export nodes) that name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed {
    pub node: Id,
    /// `(export node, access path)`, sorted by access path.
    pub aliases: Vec<(Id, String)>,
}

/// One seed's results, as rows keyed by the invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct PassAResult {
    pub completion: CoverageStatus,
    pub stop_reason: Option<StopReason>,
    pub vertices_examined: i64,
    pub arcs_examined: i64,
    pub findings: Vec<FindingsRow>,
    pub members: Vec<FindingMembersRow>,
    pub witnesses: Vec<WitnessesRow>,
}

enum Reach {
    Inside,
    Boundary(StopReason),
}

fn reach(p: &Projection, subsystem: &FixedBitSet, t: u32) -> Reach {
    match p.kinds[t as usize] {
        NodeKind::ExternalSymbol => Reach::Boundary(StopReason::ExternalBoundary),
        NodeKind::SyntheticCallable => Reach::Boundary(StopReason::SyntheticBoundary),
        _ if !subsystem.contains(t as usize) => Reach::Boundary(StopReason::SubsystemBoundary),
        _ => Reach::Inside,
    }
}

struct Draft {
    kind: FindingKind,
    related: Option<Id>,
    depth: Option<i64>,
    stop: Option<StopReason>,
    omitted: bool,
    /// Each path's arc rows, in order.
    paths: Vec<Vec<u32>>,
    members: Vec<(Id, String)>,
}

/// Run Pass A from one seed. `subsystem` marks the vertices the search may enter.
pub fn run(
    p: &Projection,
    seed: &Seed,
    subsystem: &FixedBitSet,
    budgets: Budgets,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<PassAResult, AnalyticsError> {
    let s = p
        .dense(seed.node)
        .ok_or_else(|| AnalyticsError::UnknownVertex(seed.node.hex()))?;
    let n = p.ids.len();
    let mut depth = vec![u32::MAX; n];
    let mut parent: Vec<Option<u32>> = vec![None; n];
    let mut queue = VecDeque::from([s]);
    depth[s as usize] = 0;
    let mut visited = 1u32;
    let mut arcs_examined = 0u32;
    let mut budget_stop = None;
    let mut depth_limited = false;
    let mut expanded = Vec::new();
    // Arc rows into each reached target, from expanded vertices.
    let mut into: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut boundary: BTreeMap<u32, StopReason> = BTreeMap::new();

    'search: while let Some(u) = queue.pop_front() {
        let rows = p.out_arcs(u);
        if depth[u as usize] >= budgets.max_depth {
            depth_limited |= !rows.is_empty();
            continue;
        }
        for row in rows {
            if arcs_examined >= budgets.max_edges {
                budget_stop = Some(StopReason::EdgeBudget);
                break 'search;
            }
            arcs_examined += 1;
            let t = p.arcs[row as usize].dst;
            match reach(p, subsystem, t) {
                Reach::Boundary(reason) => {
                    boundary.entry(t).or_insert(reason);
                }
                Reach::Inside if depth[t as usize] == u32::MAX => {
                    if visited >= budgets.max_vertices {
                        budget_stop = Some(StopReason::VertexBudget);
                        break 'search;
                    }
                    visited += 1;
                    depth[t as usize] = depth[u as usize] + 1;
                    parent[t as usize] = Some(row);
                    queue.push_back(t);
                }
                Reach::Inside => {}
            }
            into.entry(t).or_default().push(row);
        }
        expanded.push(u);
    }

    let path_to = |v: u32| -> Vec<u32> {
        let mut rows = Vec::new();
        let mut at = v;
        while let Some(row) = parent[at as usize] {
            rows.push(row);
            at = p.arcs[row as usize].src;
        }
        rows.reverse();
        rows
    };
    // The config refuses a zero witness budget; a direct caller still keeps the first witness.
    let w = (budgets.max_witnesses as usize).max(1);
    let mut drafts = Vec::new();

    if !seed.aliases.is_empty() {
        drafts.push(Draft {
            kind: FindingKind::PublicAlias,
            related: None,
            depth: None,
            stop: None,
            omitted: false,
            paths: Vec::new(),
            members: seed.aliases.clone(),
        });
    }

    for (&t, rows) in &into {
        if t == s {
            continue;
        }
        // The shortest final arcs, in canonical order: sources one layer above the target.
        let best = rows
            .iter()
            .map(|&r| depth[p.arcs[r as usize].src as usize] + 1)
            .min()
            .expect("a target has an arc");
        let mut finals: Vec<u32> = rows
            .iter()
            .copied()
            .filter(|&r| depth[p.arcs[r as usize].src as usize] + 1 == best)
            .collect();
        finals.sort_unstable();
        finals.dedup();
        let first = match boundary.get(&t) {
            Some(_) => finals[0],
            None => parent[t as usize].expect("an inside target has a parent"),
        };
        let mut chosen = vec![first];
        chosen.extend(finals.iter().copied().filter(|&r| r != first).take(w - 1));
        let omitted = finals.len() > w;
        let paths: Vec<Vec<u32>> = chosen
            .iter()
            .map(|&r| {
                let mut path = path_to(p.arcs[r as usize].src);
                path.push(r);
                path
            })
            .collect();
        let (kind, stop) = match boundary.get(&t) {
            Some(reason) => (FindingKind::ImplementationBoundary, Some(*reason)),
            None => {
                let definite = |path: &Vec<u32>| {
                    path.iter()
                        .all(|&r| p.arcs[r as usize].modality == Modality::Definite)
                };
                if best == 1 && paths.iter().any(definite) {
                    (FindingKind::DirectDelegation, None)
                } else {
                    (FindingKind::BoundedDelegationPath, None)
                }
            }
        };
        drafts.push(Draft {
            kind,
            related: Some(p.ids[t as usize]),
            depth: Some(i64::from(best)),
            stop,
            omitted,
            paths,
            members: Vec::new(),
        });
    }

    let mut unresolved: HashMap<u32, Vec<Id>> = HashMap::new();
    for site in &p.unresolved {
        unresolved
            .entry(site.caller)
            .or_default()
            .push(site.call_site);
    }
    for &u in &expanded {
        let Some(sites) = unresolved.get(&u) else {
            continue;
        };
        let to_caller = path_to(u);
        for &site in sites {
            drafts.push(Draft {
                kind: FindingKind::IncompleteResolution,
                related: Some(site),
                depth: Some(i64::from(depth[u as usize]) + 1),
                stop: Some(StopReason::UnresolvedSite),
                omitted: false,
                paths: if to_caller.is_empty() {
                    Vec::new()
                } else {
                    vec![to_caller.clone()]
                },
                members: Vec::new(),
            });
        }
    }

    let mut findings = Vec::new();
    let mut members = Vec::new();
    let mut witnesses = Vec::new();
    for d in drafts {
        let steps: Vec<Vec<StepKey>> = d
            .paths
            .iter()
            .map(|path| {
                path.iter()
                    .map(|&r| {
                        let a = &p.arcs[r as usize];
                        StepKey {
                            call_site: a.call_site,
                            callee: p.ids[a.dst as usize],
                            modality: a.modality.code(),
                            phase: a.phase.code(),
                        }
                    })
                    .collect()
            })
            .collect();
        let member_keys: Vec<MemberKey> = d
            .members
            .iter()
            .enumerate()
            .map(|(ordinal, (node, label))| MemberKey {
                role: MemberRole::AccessPath.code(),
                ordinal: ordinal as i64,
                node: Some(*node),
                cited_fact: None,
                label: Some(label.clone()),
            })
            .collect();
        let status = FINDING_STATUS
            .iter()
            .find(|(k, _)| *k == d.kind)
            .map(|(_, s)| *s)
            .ok_or_else(|| AnalyticsError::Graph(format!("no status policy for {:?}", d.kind)))?;
        let finding_id = FindingKey {
            finding_kind: d.kind.code(),
            subject: seed.node,
            related: d.related,
            condition: None,
            evidence_status: status.code(),
            depth: d.depth,
            stop_reason: d.stop.map(Codebook::code),
            witnesses_omitted: d.omitted,
            paths: &steps,
            members: &member_keys,
        }
        .id();
        for (ordinal, (node, label)) in d.members.iter().enumerate() {
            members.push(FindingMembersRow {
                snapshot_id,
                finding_id,
                role: MemberRole::AccessPath,
                ordinal: ordinal as i64,
                node_id: Some(*node),
                cited_fact_id: None,
                label: Some(label.clone()),
                weight: None,
            });
        }
        for (path, rows) in d.paths.iter().enumerate() {
            for (step, &r) in rows.iter().enumerate() {
                let a = &p.arcs[r as usize];
                witnesses.push(WitnessesRow {
                    snapshot_id,
                    finding_id,
                    path: path as i64,
                    step: step as i64,
                    caller_node_id: p.ids[a.src as usize],
                    call_site_node_id: a.call_site,
                    callee_node_id: p.ids[a.dst as usize],
                    edge_id: a.edge_id,
                    modality: a.modality,
                    phase: a.phase,
                });
            }
        }
        findings.push(FindingsRow {
            snapshot_id,
            finding_id,
            invocation_id,
            finding_kind: d.kind,
            subject_node_id: seed.node,
            related_node_id: d.related,
            evidence_status: status,
            depth: d.depth,
            stop_reason: d.stop,
            witnesses_omitted: d.omitted,
            score: None,
            condition_node_id: None,
        });
    }

    Ok(PassAResult {
        completion: if budget_stop.is_some() {
            CoverageStatus::Partial
        } else {
            CoverageStatus::CompleteUnderStatedModel
        },
        stop_reason: budget_stop.or(depth_limited.then_some(StopReason::DepthLimit)),
        vertices_examined: i64::from(visited),
        arcs_examined: i64::from(arcs_examined),
        findings,
        members,
        witnesses,
    })
}
