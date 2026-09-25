# Stage 3 sink-callable identity — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `value_flow_contributions.sink_function_node_id`, its producer and reference rule |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the separate sink-owner coordinate for L3; captured-owner divergence remains untested |

The source origin's `function_node_id` can answer who owns a parameter, while a summary must answer which callable contains the sink. I inspected the raw use lookup, Arrow contract, foreign-key rule and reviewed schema snapshot. The focused release selection passed 3/3 on 2026-09-25 and checks a populated sink owner. I attempted the existing named nested-function fixture and observed no corresponding captured contribution, so it does **not** establish the divergent-owner case; that limitation is recorded below. No integrated test or pilot ran.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | The raw `flow_uses.use_id` is the source for the sink owner (`flow_model.rs:1317`); the parameter owner remains separate. | — |
| G2 Fidelity | Pass | Two function identities are separate typed IDs; null sink ownership is explicit for nonfunction uses (`behavior.rs:887`). | — |
| G3 Validity | Pass | The schema snapshot and FK rule cover the new column; full contribution reconstruction checks stored values. | — |
| G4 Hidden behavior | Pass | The producer uses pinned flow rows only. | — |
| G5 Recovery | Pass | The compiler version increments to 43 and validation precedes publication. | — |
| G6 Transformation | Pass, scoped | The immediate raw use's owner is preserved without deriving it from the source parameter or merged sink span. | — |
| G7 Claims | Pass, scoped | The design reports the tested direct case and leaves divergent-owner behavior unverified. | Add a covered captured-function fixture before L3 relies on it. |
| G8 Library leverage | Pass | Existing Arrow schema and DataFusion FK rule handle persistence; no generic graph or query machinery was added. | — |
| CI-G1 Fidelity | Pass | A source owner is not relabelled as a sink owner. | — |
| CI-G2 Evidence closure | N.a. | No served summary consumes the field yet. | Trace when FORMAT 7 uses it. |
| CI-G3 Evaluation integrity | Pass | The fixture is source input, outside the FastMCP gold reference. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | Current focused fixtures do not establish a contribution whose source and sink callable IDs differ. | DP-22, CI-04 · G7 | The attempted `nested_function` assertion found no such row. Using this field as proof of captured-flow coverage would overclaim. | Add a fixture the flow provider actually covers, or retain a captured-flow boundary in summaries. |

DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/10/12 are satisfied for the persisted coordinate. Summary closure, graph, heuristic and served-projection principles carry no new claim here. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in | Fit and limit | Recommendation |
|---|---|---|---|---|
| Sink owner citation | One lookup by raw use ID | Existing Arrow table and generated DataFusion reference rule | Keeps source and sink identities distinct; no new general algorithm. | Use the field as L3's owner key only when nonnull. |

## 12. Decision

**Accept the additional identity coordinate.** The reviewed schema migration adds one nullable function ID and one reference rule; the focused release selection passed 3/3 on 2026-09-25. The captured-owner divergence remains unverified and must stay an explicit L3 boundary until exercised. Formatting, integrated `just test-all`, pilot and structured evaluation remain `not_run` under the operator's policy.
