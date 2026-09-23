//! Pass A and the §5 adapter on a hand-built projection whose answers are worked out by hand
//! (DESIGN §9.1, §12: parallel calls, two paths to one helper, a candidate arc, boundaries of
//! each kind, an unresolved site, the depth budget).

use std::sync::Arc as Shared;

use arrow_array::{
    ArrayRef, BooleanArray, FixedSizeBinaryArray, Int16Array, RecordBatch, StringArray,
};
use cpg_schema::codebook::{
    Codebook, CoverageStatus, FindingKind, InvocationPhase, Modality, NodeKind, SourceRole,
    StopReason,
};
use cpg_schema::id::Id;
use cpg_schema::projection::schemas;
use lctx_analytics::graph::Projection;
use lctx_analytics::pass_a::{Budgets, Seed, run};

fn id(k: u8) -> Id {
    Id([k; 16])
}

fn ids(values: &[Id]) -> ArrayRef {
    Shared::new(FixedSizeBinaryArray::try_from_iter(values.iter().map(|v| v.0)).expect("16 bytes"))
}

// Vertices: S=1 (seed), A=2, B=3, C=4, D=5 in the subsystem; X=6 a dependency symbol; Y=7 a
// synthetic callable; Z=8 a release function outside the subsystem.
const S: u8 = 1;
const A: u8 = 2;
const B: u8 = 3;
const C: u8 = 4;
const D: u8 = 5;
const X: u8 = 6;
const Y: u8 = 7;
const Z: u8 = 8;

fn vertices() -> RecordBatch {
    let rows: [(u8, NodeKind, Option<&str>); 8] = [
        (S, NodeKind::Function, Some("pkg.server")),
        (A, NodeKind::Function, Some("pkg.server")),
        (B, NodeKind::Function, Some("pkg.server.tools")),
        (C, NodeKind::Function, Some("pkg.server")),
        (D, NodeKind::Function, Some("pkg.server")),
        (X, NodeKind::ExternalSymbol, None),
        (Y, NodeKind::SyntheticCallable, Some("pkg.server")),
        (Z, NodeKind::Function, Some("pkg.other")),
    ];
    RecordBatch::try_new(
        schemas::vertices(),
        vec![
            ids(&rows.iter().map(|r| id(r.0)).collect::<Vec<_>>()),
            Shared::new(Int16Array::from_iter_values(
                rows.iter().map(|r| r.1.code()),
            )),
            Shared::new(rows.iter().map(|r| r.2).collect::<StringArray>()),
            Shared::new(
                rows.iter()
                    .map(|r| r.2.map(|_| SourceRole::Release.code()))
                    .collect::<Int16Array>(),
            ),
        ],
    )
    .unwrap()
}

/// `(src, dst, call site, modality)`; the edge id is the call site's byte plus 100.
type ArcSpec = (u8, u8, u8, Modality);

const ARCS: &[ArcSpec] = &[
    (S, A, 11, Modality::Definite),
    (S, A, 12, Modality::Definite),
    (S, B, 13, Modality::Candidate),
    (S, X, 16, Modality::Definite),
    (S, Z, 18, Modality::Definite),
    (A, C, 14, Modality::Definite),
    (A, Y, 17, Modality::Definite),
    (B, C, 15, Modality::Definite),
    (C, D, 19, Modality::Definite),
];

fn arcs(specs: &[ArcSpec]) -> RecordBatch {
    RecordBatch::try_new(
        schemas::arcs(),
        vec![
            ids(&specs.iter().map(|a| id(a.0)).collect::<Vec<_>>()),
            ids(&specs.iter().map(|a| id(a.1)).collect::<Vec<_>>()),
            ids(&specs.iter().map(|a| id(a.2)).collect::<Vec<_>>()),
            ids(&specs.iter().map(|a| id(a.2 + 100)).collect::<Vec<_>>()),
            Shared::new(Int16Array::from_iter_values(
                specs.iter().map(|_| InvocationPhase::Call.code()),
            )),
            Shared::new(Int16Array::from_iter_values(
                specs.iter().map(|a| a.3.code()),
            )),
            Shared::new(specs.iter().map(|_| Some(false)).collect::<BooleanArray>()),
        ],
    )
    .unwrap()
}

fn unresolved() -> RecordBatch {
    RecordBatch::try_new(
        schemas::unresolved(),
        vec![
            ids(&[id(S)]),
            ids(&[id(20)]),
            Shared::new(arrow_array::Int64Array::from(vec![0])),
            Shared::new(BooleanArray::from(vec![true])),
            Shared::new(Int16Array::from(vec![None])),
            Shared::new(Int16Array::from(vec![None])),
        ],
    )
    .unwrap()
}

fn projection() -> Projection {
    Projection::build(&[vertices()], &[arcs(ARCS)], &[unresolved()]).unwrap()
}

fn subsystem(p: &Projection) -> fixedbitset::FixedBitSet {
    p.mask(|i| {
        p.modules[i]
            .as_deref()
            .is_some_and(|m| m == "pkg.server" || m.starts_with("pkg.server."))
    })
}

const BUDGETS: Budgets = Budgets {
    max_depth: 2,
    max_vertices: 128,
    max_edges: 512,
    max_witnesses: 3,
};

fn seed() -> Seed {
    Seed {
        node: id(S),
        aliases: vec![
            (id(90), "pkg.S".to_owned()),
            (id(91), "pkg.server.S".to_owned()),
        ],
    }
}

fn kind_of(
    r: &lctx_analytics::pass_a::PassAResult,
    related: Option<u8>,
    kind: FindingKind,
) -> usize {
    r.findings
        .iter()
        .filter(|f| f.finding_kind == kind && f.related_node_id == related.map(id))
        .count()
}

#[test]
fn pass_a_finds_delegations_boundaries_and_gaps_with_witnesses() {
    let p = projection();
    let r = run(&p, &seed(), &subsystem(&p), BUDGETS, id(200), id(201)).unwrap();
    assert_eq!(r.completion, CoverageStatus::CompleteUnderStatedModel);
    // C (depth 2) has an arc to D, which the depth budget does not examine.
    assert_eq!(r.stop_reason, Some(StopReason::DepthLimit));
    assert_eq!(r.findings.len(), 8, "{:#?}", r.findings);
    assert_eq!(kind_of(&r, None, FindingKind::PublicAlias), 1);
    assert_eq!(kind_of(&r, Some(A), FindingKind::DirectDelegation), 1);
    // The candidate arc to B is followed, but never a direct delegation (§3.6).
    assert_eq!(kind_of(&r, Some(B), FindingKind::BoundedDelegationPath), 1);
    assert_eq!(kind_of(&r, Some(C), FindingKind::BoundedDelegationPath), 1);
    assert_eq!(kind_of(&r, Some(X), FindingKind::ImplementationBoundary), 1);
    assert_eq!(kind_of(&r, Some(Y), FindingKind::ImplementationBoundary), 1);
    assert_eq!(kind_of(&r, Some(Z), FindingKind::ImplementationBoundary), 1);
    assert_eq!(kind_of(&r, Some(20), FindingKind::IncompleteResolution), 1);
    assert_eq!(kind_of(&r, Some(D), FindingKind::BoundedDelegationPath), 0);

    let stop = |t: u8| {
        r.findings
            .iter()
            .find(|f| f.related_node_id == Some(id(t)))
            .unwrap()
            .stop_reason
    };
    assert_eq!(stop(X), Some(StopReason::ExternalBoundary));
    assert_eq!(stop(Y), Some(StopReason::SyntheticBoundary));
    assert_eq!(stop(Z), Some(StopReason::SubsystemBoundary));

    let paths = |t: u8| -> Vec<Vec<u8>> {
        let f = r
            .findings
            .iter()
            .find(|f| f.related_node_id == Some(id(t)))
            .unwrap();
        let mut steps: Vec<_> = r
            .witnesses
            .iter()
            .filter(|w| w.finding_id == f.finding_id)
            .collect();
        steps.sort_by_key(|w| (w.path, w.step));
        let mut out: Vec<Vec<u8>> = Vec::new();
        for w in steps {
            if w.step == 0 {
                out.push(Vec::new());
                assert_eq!(w.caller_node_id, id(S), "every path starts at the seed");
            }
            out.last_mut().unwrap().push(w.call_site_node_id.0[0]);
        }
        out
    };
    // Two parallel call sites to A: two witnesses, each its own arc.
    assert_eq!(paths(A), vec![vec![11], vec![12]]);
    // Two shortest paths to C, differing in their final arc, the BFS path first.
    assert_eq!(paths(C), vec![vec![11, 14], vec![13, 15]]);
    assert_eq!(paths(Y), vec![vec![11, 17]]);
    assert_eq!(paths(X), vec![vec![16]]);
    // Each witness step chains from the previous step's callee.
    for f in &r.findings {
        let mut steps: Vec<_> = r
            .witnesses
            .iter()
            .filter(|w| w.finding_id == f.finding_id)
            .collect();
        steps.sort_by_key(|w| (w.path, w.step));
        for pair in steps.windows(2) {
            if pair[1].path == pair[0].path {
                assert_eq!(pair[0].callee_node_id, pair[1].caller_node_id);
            }
        }
    }
    let alias: Vec<_> = r.members.iter().map(|m| m.label.clone().unwrap()).collect();
    assert_eq!(alias, ["pkg.S", "pkg.server.S"]);
}

#[test]
fn pass_a_is_deterministic_and_its_ids_are_content_derived() {
    let p = projection();
    let a = run(&p, &seed(), &subsystem(&p), BUDGETS, id(200), id(201)).unwrap();
    let b = run(
        &projection(),
        &seed(),
        &subsystem(&p),
        BUDGETS,
        id(200),
        id(201),
    )
    .unwrap();
    assert_eq!(a, b);
    // Another snapshot and invocation (a parameter change) keep every finding's id.
    let c = run(&p, &seed(), &subsystem(&p), BUDGETS, id(210), id(211)).unwrap();
    let ids = |r: &lctx_analytics::pass_a::PassAResult| {
        let mut v: Vec<Id> = r.findings.iter().map(|f| f.finding_id).collect();
        v.sort();
        v
    };
    assert_eq!(ids(&a), ids(&c));
}

#[test]
fn budgets_truncate_and_say_so() {
    let p = projection();
    let sub = subsystem(&p);
    let vertices = run(
        &p,
        &seed(),
        &sub,
        Budgets {
            max_vertices: 2,
            ..BUDGETS
        },
        id(1),
        id(2),
    )
    .unwrap();
    assert_eq!(vertices.completion, CoverageStatus::Partial);
    assert_eq!(vertices.stop_reason, Some(StopReason::VertexBudget));
    assert_eq!(vertices.vertices_examined, 2);

    let edges = run(
        &p,
        &seed(),
        &sub,
        Budgets {
            max_edges: 1,
            ..BUDGETS
        },
        id(1),
        id(2),
    )
    .unwrap();
    assert_eq!(edges.stop_reason, Some(StopReason::EdgeBudget));
    assert_eq!(edges.arcs_examined, 1);

    let one = run(
        &p,
        &seed(),
        &sub,
        Budgets {
            max_witnesses: 1,
            ..BUDGETS
        },
        id(1),
        id(2),
    )
    .unwrap();
    let a = one
        .findings
        .iter()
        .find(|f| f.related_node_id == Some(id(A)))
        .unwrap();
    assert!(a.witnesses_omitted, "a second parallel call site existed");
    assert_eq!(
        one.witnesses
            .iter()
            .filter(|w| w.finding_id == a.finding_id)
            .count(),
        1
    );
}

#[test]
fn the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder() {
    let p = projection();
    assert_eq!(p.graph.node_count(), 8);
    assert_eq!(p.graph.edge_count(), ARCS.len());
    // Z has no arcs of its own: an isolate as a source is still a vertex.
    assert!(p.out_arcs(p.dense(id(Z)).unwrap()).is_empty());
    // Out-arcs come back in canonical row order, not petgraph's newest-first order.
    let s = p.dense(id(S)).unwrap();
    assert_eq!(p.out_arcs(s), vec![0, 1, 2, 3, 4]);
    let mut shuffled = ARCS.to_vec();
    shuffled.swap(0, 3);
    assert!(Projection::build(&[vertices()], &[arcs(&shuffled)], &[]).is_err());
    let stray = [(S, 42, 30, Modality::Definite)];
    assert!(Projection::build(&[vertices()], &[arcs(&stray)], &[]).is_err());
}
