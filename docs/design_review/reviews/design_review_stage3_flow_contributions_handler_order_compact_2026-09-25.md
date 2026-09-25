# Stage 3 raw flow contributions and handler clause order — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `value_flow_contributions`, within-frame modeled handler ordering, and their publication validation |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept for source provenance and conditional within-frame selection only |

The change retains raw value-fact identity before the presentation flow merge and adds a clause-order predicate conditional on a modeled raise reaching its `try` frame. I inspected the producer, schema SQL, validator, migration snapshots, and focused fixture. The release Nextest selection passed 5/5 (173 skipped) and targeted release Clippy passed on 2026-09-25. I did not inspect or run the integrated pilot, complete handler fates, SCC summaries or serving; none is claimed by this slice.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | `flow_values.fact_id` and pinned class/MRO facts remain authorities; the new rows derive from them (`flow_model.rs:783`, `behavior.rs:1612`). | — |
| G2 Semantic fidelity | Pass | `local_through_call` distinguishes this fact's call path from inherited call uncertainty (`flow_model.rs:793`); handler flags are explicitly frame-conditional. | — |
| G3 Validity | Pass | Arrow checks and shared full-row reconstruction reject missing contributions (`validate.rs:161`; `compile.rs:830`). | — |
| G4 Hidden behaviour | Pass | DataFusion reads persisted syntax/model facts; the flow pass uses pinned flow facts, without evaluating analyzed Python. | — |
| G5 Recovery | Pass, scoped | Validation precedes publication and compares all contribution rows; no new partial publication path. | — |
| G6 Transformation | Pass, scoped | Exact parent fact IDs survive the analysis seam; window ordering partitions by modeled raise and frame (`behavior.rs:1638`). | Follow predecessor facts before discharging inherited calls in L3. |
| G7 Claims | Pass, scoped | DESIGN §9.9 withholds a completed catch, transfer and operation verdict. | Keep that boundary in FORMAT 7. |
| G8 Library leverage | Pass | DataFusion's window aggregate handles clause precedence; Arrow's declared table and existing Rust flow transfer domain handle the other work. | — |
| CI-G1 Fidelity | Pass | Candidate raise, handler, MRO and raw value fact remain separately cited; unknown earlier match prevents first-match proof. | — |
| CI-G2 Evidence closure | N.a. | No served behavioral claim consumes these new rows. | Verify when FORMAT 7 uses them. |
| CI-G3 Evaluation integrity | Pass | Fixture and source validator are independent of the FastMCP gold capability families. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | An inherited call is marked but its predecessor path is not yet linked by this relation. | DP-08, CI-04 · G6 | `flow_model.rs:1294` retains the immediate raw fact and `local_through_call=false`. Joining that fact to call links would lose the earlier call or misattribute it. | Follow cited reaching-definition facts, or emit an explicit summary boundary; test assignment then return. |
| F02, deferred to L2 | Within-frame first-match does not establish operation-level catch or completion. | CI-06, DP-11 · CI-G1 | `behavior.rs:1658` has no proof that an inner frame propagated or that the handler body completes. Promoting it would overstate an exception fate. | Compose nested frames and handler exits; preserve unknown where proof is absent. |

Within the stated source and frame-conditional scope, DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied. DP-09/10/12/16/17/20 and CI-05/09/11/13 add no new obligation to this bounded change; Stage 3 summary and serving obligations remain open. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Clause precedence | Domain class-match classification | DataFusion 55.1.0 window `SUM ... ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING` | Keeps partitions per modeled raise and `try` frame; focused first/unknown-prior cases passed. | Keep SQL; no custom clause loop. |
| Flow source provenance | `Model::sink_contributions` over ty-derived facts | Existing Arrow table/schema and `flow_value_call_links` | Raw fact ID is a join key; Arrow does not itself reconstruct transitive reaching paths. | Compose through existing flow transfer domain in L3. |

## 12. Decision

**Accept the source-provenance and conditional clause-order slice.** Focused release Nextest passed 5/5 on 2026-09-25, including a missing-row validator tamper, and targeted release Clippy passed. F01 and F02 must be resolved or explicitly bounded before any summary or served catch/transfer verdict. Integrated `just test-all`, fresh-store `just pilot`, structured evaluation and formatting are `not_run` by the operator's end-of-scope policy.
