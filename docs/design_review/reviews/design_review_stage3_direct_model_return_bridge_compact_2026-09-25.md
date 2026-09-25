# Stage 3 direct model return bridge — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `modeled_direct_return_transfers`, ADR-0028's first model/source composition |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a candidate-local one-call path only |

The derived relation joins an unmerged source-parameter contribution to one exact call/argument step and a pinned model transfer whose input/output nodes agree. The relation retains condition, raw-fact approximation, provenance, both modalities and dispatch openness. The focused `model_shapes` fixture asserts three direct identity paths, withholds assignment and nested-call shapes, and rejects a dropped publication row. Final focused release Nextest passed 3/3 (175 skipped) after the reviewed schema rename, and targeted release Clippy passed on 2026-09-25. The integrated pilot and serving were `not_run` by operator direction.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Raw `flow_values` fact, contribution, ordered call step, exact Ruff link and pinned model are all cited. | — |
| G2 Semantic fidelity | Pass, scoped | One local call, unchanged upstream parameter and matching model endpoints are required; no call completion is inferred. | Follow predecessor facts and nested steps before widening. |
| G3 Validity | Pass | Arrow schema, generated references and shared full reconstruction reject missing rows. | — |
| G4 Hidden behaviour | Pass | DataFusion SQL composes persisted facts; it does not execute the analyzed package. | — |
| G5 Recovery | Pass, scoped | Version 47 distinguishes this table contract and validation runs before publication. | End-of-scope integrated gate remains. |
| G6 Transformation | Pass, scoped | Exact call and argument ids preserve the provider-to-model seam; merged sink spans are not used. | — |
| G7 Claims | Pass, scoped | DESIGN and ADR-0028 call these conditional candidate paths, not `summary_flows` verdicts. | Keep candidate modality and open dispatch in L3. |
| G8 Library leverage | Pass | DataFusion joins/aggregation and Arrow contracts serve the fixed-depth composition. | Use petgraph SCC for subsequent recursion. |
| CI-G1 Fidelity | Pass | The source parameter and sink callable must agree, excluding captured foreign owners in this narrow bridge. | Widen only with a closure proof. |
| CI-G2 Evidence closure | N.a. | No served claim consumes the new relation. | Check FORMAT 7 consumer. |
| CI-G3 Evaluation integrity | Pass | Fixture is independent of the FastMCP gold capability families. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | An assigned intermediate and nested call have predecessor steps this row cannot prove. | DP-08, CI-04 · G6 | The fixture's `indirect_identity` and `nested_identity` yield no direct row. | Follow cited raw predecessor facts or emit `summary_boundaries`; never use a sink span as substitute. |
| F02, deferred to L3 | Raw fact approximation is not a complete path approximation or return-completion proof. | CI-06, DP-11 · G7 | The field is explicitly `raw_flow_approximated`; model and dispatch modalities remain. | Aggregate path and exit evidence before any verdict. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within the stated scope. Other principles add no local obligation. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Exact one-step join | Domain admission predicate | DataFusion grouping and equijoins | Grouped step count and exact node joins avoid a second AST interpreter. | Keep relational bridge. |
| Recursive summary | None in this slice | petgraph SCC, existing BDD kernel | This row supplies provenance but not fixpoint composition. | Implement L3 with explicit bounds and unknowns. |

## 12. Decision

**Accept only the one-call candidate bridge.** Final focused release Nextest passed 3/3 and targeted Clippy passed on 2026-09-25. Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until complete Stage 3 scope.
