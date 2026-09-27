//! Discharge of call-transfer claims by finite summaries (ADR-0064).
//!
//! One outcome has three origin-level views. `summary_flows` holds the positive proofs.
//! `summary_boundaries` holds the residual: an origin that is unproved, or refused although a path
//! exists. `summary_origin_coverage` holds completeness. An origin is **proved** for discharge only
//! when an established or conditional summary cites it *and* the residual view has no boundary for
//! it. A claim is graded from the set of its member origins, so the grade does not depend on the
//! order in which the claim's rows were merged. An open member is never a negative conclusion.

use std::collections::{BTreeMap, BTreeSet};

use cpg_schema::behavior::{BehaviorDischargesRow, SummaryBoundariesRow, SummaryFlowsRow};
use cpg_schema::codebook::{
    BoundaryReason, Codebook, DischargeDecision, DischargeProofKind, Verdict,
};
use cpg_schema::id::Id;

/// One origin's standing: the summary that proves it, or why it stays open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Proved { summary_id: Id },
    Open { reason: BoundaryReason },
}

/// Per-origin decisions over one finite outcome.
#[derive(Debug, Default, Clone)]
pub struct Decisions {
    proved: BTreeMap<Id, Id>,
    open: BTreeMap<Id, BoundaryReason>,
}

impl Decisions {
    /// The lowest cited summary id proves an origin; the lowest-coded boundary reason keeps it
    /// open. Both choices are deterministic over unordered inputs.
    pub fn from_outcome(flows: &[SummaryFlowsRow], boundaries: &[SummaryBoundariesRow]) -> Self {
        let mut open: BTreeMap<Id, BoundaryReason> = BTreeMap::new();
        for b in boundaries {
            open.entry(b.source_origin_id)
                .and_modify(|r| {
                    if b.reason.code() < r.code() {
                        *r = b.reason;
                    }
                })
                .or_insert(b.reason);
        }
        let mut proved: BTreeMap<Id, Id> = BTreeMap::new();
        for f in flows.iter().filter(|f| {
            matches!(f.verdict, Verdict::Established | Verdict::Conditional)
                && !open.contains_key(&f.source_origin_id)
        }) {
            proved
                .entry(f.source_origin_id)
                .and_modify(|s| *s = (*s).min(f.summary_id))
                .or_insert(f.summary_id);
        }
        Self { proved, open }
    }

    /// An origin no summary considered (not a candidate: a field, captured or non-return origin)
    /// stays open as `call_transfer`.
    pub fn decide(&self, origin: Id) -> Decision {
        if let Some(&summary_id) = self.proved.get(&origin) {
            return Decision::Proved { summary_id };
        }
        Decision::Open {
            reason: self
                .open
                .get(&origin)
                .copied()
                .unwrap_or(BoundaryReason::CallTransfer),
        }
    }
}

/// A claim's discharge evidence (one row per member origin, ordered by origin) and whether every
/// member is proved. An empty member set proves nothing.
pub fn grade(
    snapshot_id: Id,
    behavior_id: Id,
    members: &BTreeSet<Id>,
    decisions: &Decisions,
) -> (bool, Vec<BehaviorDischargesRow>) {
    let rows: Vec<BehaviorDischargesRow> = members
        .iter()
        .map(|&origin_id| {
            let (decision, summary_id, reason) = match decisions.decide(origin_id) {
                Decision::Proved { summary_id } => {
                    (DischargeDecision::Proved, Some(summary_id), None)
                }
                Decision::Open { reason } => (DischargeDecision::Open, None, Some(reason)),
            };
            BehaviorDischargesRow {
                snapshot_id,
                behavior_id,
                origin_id,
                proof_kind: DischargeProofKind::CallerReturnSummary,
                decision,
                summary_id,
                reason,
            }
        })
        .collect();
    let proved = !rows.is_empty() && rows.iter().all(|r| r.decision == DischargeDecision::Proved);
    (proved, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpg_schema::codebook::SummaryFlowKind;

    fn id(b: u8) -> Id {
        Id([b; 16])
    }

    fn flow(summary: u8, origin: u8, verdict: Verdict) -> SummaryFlowsRow {
        SummaryFlowsRow {
            snapshot_id: id(1),
            summary_id: id(summary),
            function_node_id: id(2),
            parameter_node_id: id(3),
            input_path: "Parameter[value]".to_owned(),
            output_path: "ReturnValue".to_owned(),
            kind: SummaryFlowKind::Value,
            condition_id: id(4),
            verdict,
            boundary_reason: (verdict == Verdict::Unknown).then_some(BoundaryReason::CallTransfer),
            source_flow_fact_id: id(5),
            source_origin_id: id(origin),
            return_site_fact_id: id(6),
            return_region_fact_id: id(7),
            approximated: false,
            path_depth: 1,
        }
    }

    fn boundary(origin: u8, reason: BoundaryReason) -> SummaryBoundariesRow {
        SummaryBoundariesRow {
            snapshot_id: id(1),
            function_node_id: id(2),
            parameter_node_id: id(3),
            source_flow_fact_id: id(5),
            source_origin_id: id(origin),
            condition_id: id(4),
            reason,
            local_through_call: true,
            upstream_through_call: false,
            raw_approximated: false,
        }
    }

    #[test]
    fn proved_needs_an_admitted_summary_and_no_residual_boundary() {
        let decisions = Decisions::from_outcome(
            &[
                flow(30, 10, Verdict::Established),
                flow(20, 10, Verdict::Conditional),
                flow(40, 11, Verdict::Established),
                flow(50, 12, Verdict::Unknown),
            ],
            &[
                boundary(11, BoundaryReason::SummaryDepthLimit),
                boundary(12, BoundaryReason::CallTransfer),
            ],
        );
        assert_eq!(
            decisions.decide(id(10)),
            Decision::Proved { summary_id: id(20) }
        );
        assert_eq!(
            decisions.decide(id(11)),
            Decision::Open {
                reason: BoundaryReason::SummaryDepthLimit
            }
        );
        assert_eq!(
            decisions.decide(id(12)),
            Decision::Open {
                reason: BoundaryReason::CallTransfer
            }
        );
        // Not a candidate at all.
        assert_eq!(
            decisions.decide(id(99)),
            Decision::Open {
                reason: BoundaryReason::CallTransfer
            }
        );
    }

    #[test]
    fn a_claim_is_proved_only_when_every_member_is() {
        let decisions = Decisions::from_outcome(&[flow(20, 10, Verdict::Established)], &[]);
        let one: BTreeSet<Id> = [id(10)].into();
        let (proved, rows) = grade(id(1), id(70), &one, &decisions);
        assert!(proved);
        assert_eq!(rows[0].summary_id, Some(id(20)));
        let mixed: BTreeSet<Id> = [id(10), id(11)].into();
        let (proved, rows) = grade(id(1), id(70), &mixed, &decisions);
        assert!(!proved);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].decision, DischargeDecision::Open);
        assert_eq!(rows[1].reason, Some(BoundaryReason::CallTransfer));
        assert!(!grade(id(1), id(70), &BTreeSet::new(), &decisions).0);
    }

    #[test]
    fn decisions_ignore_input_order() {
        let flows = [
            flow(30, 10, Verdict::Established),
            flow(20, 10, Verdict::Established),
        ];
        let bounds = [
            boundary(11, BoundaryReason::CallTransfer),
            boundary(11, BoundaryReason::SummaryDepthLimit),
        ];
        let mut rflows = flows.clone();
        rflows.reverse();
        let mut rbounds = bounds.clone();
        rbounds.reverse();
        for origin in [10, 11] {
            assert_eq!(
                Decisions::from_outcome(&flows, &bounds).decide(id(origin)),
                Decisions::from_outcome(&rflows, &rbounds).decide(id(origin))
            );
        }
    }
}
