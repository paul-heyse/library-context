# Stage 3 upstream transfer provenance — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Transfer state before a raw `flow_values` fact enters its local call path |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a bounded prerequisite for direct modeled-call discharge |

I inspected `Model::sink_contributions`, the Arrow contract and shared reconstruction validator, reviewed the schema migration, and ran a focused release selection (2/2 passed on 2026-09-25). The positive fixture distinguishes a direct call with identity upstream from returning an earlier call result. It does not reconstruct the predecessor fact chain, compose any model, or prove a complete summary.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | The upstream state comes from the flow producer's reaching-source transfer, before combining it with the local fact. | — |
| G2 Fidelity | Pass | Identity, derived and prior-call states remain distinct through two exclusive booleans. | — |
| G3 Validity | Pass | Schema checks reject contradictory flags; shared full-row reconstruction rejects forged provenance. | — |
| G4 Hidden behavior | Pass | No source-text interpretation or ambient execution enters the derivation. | — |
| G5 Recovery | Pass | Version 44 changes the compiler identity; publication still validates before commit. | — |
| G6 Transformation | Pass, scoped | A prior call is not attributed to the local fact's ordered call links. | Follow predecessor facts for multi-assignment summaries. |
| G7 Claims | Pass, scoped | DESIGN reports only a provenance distinction; no call-transfer verdict is asserted. | — |
| G8 Library leverage | Pass | Existing Rust transfer lattice and Arrow schema are reused; no generic recursive engine is added. | — |
| CI-G1 Fidelity | Pass | A combined `through_call` observation is not misread as a local call witness. | — |
| CI-G2 Evidence closure | N.a. | No served result consumes these fields yet. | Trace the eventual FORMAT 7 summary. |
| CI-G3 Evaluation integrity | Pass | Source fixtures are separate from the FastMCP gold family reference. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | An upstream-call flag identifies uncertainty but not the earlier raw fact or its ordered path. | DP-08, CI-04 · G6 | Returning an assigned call result cannot be discharged by joining the return fact to its own call links. | Carry a bounded predecessor witness chain or emit `call_transfer` unknown with the exact boundary. |

DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied for this narrow provenance relation. Recursive summaries and serving remain open, with no SHOULD exception requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in | Fit and limit | Recommendation |
|---|---|---|---|---|
| Transfer-state retention | Two booleans from the existing `Transfer` lattice | Arrow schema checks and DataFusion publication rules | No new interpreter or graph traversal; predecessor identity remains absent. | Use only direct upstream-identity paths until L3 can follow predecessors. |

## 12. Decision

**Accept the upstream-state fields.** The reviewed schema snapshot and focused release selection passed 2/2 on 2026-09-25. The full repository/pilot/evaluation gates and formatting remain `not_run` under the operator's end-of-scope policy.
