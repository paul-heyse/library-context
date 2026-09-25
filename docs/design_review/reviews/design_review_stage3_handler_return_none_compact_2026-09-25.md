# Stage 3 handler return-None witness — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | One direct `return None` handler-body witness and its publication contract |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a source-local witness, not a completed catch |

I inspected Ruff syntax placement, the ty region join, SQL statement/value cardinality, schema references and shared validator. The focused release selection passed 3/3 on 2026-09-25 after testing a direct literal return, a computed return, a preceding statement and missing-row tampering. Ty marks the positive handler region approximate; the output retains that fact. No nested-frame propagation, `finally` completion, integrated pilot or served answer was inspected.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff syntax gives the direct action/literal; ty supplies a separate region fact. | — |
| G2 Fidelity | Pass | The relation requires one authored body statement and an exact `None` child; approximation is retained. | — |
| G3 Validity | Pass | Declared references plus shared full-row reconstruction reject missing rows in the focused fixture. | — |
| G4 Hidden behavior | Pass | No Python source-text parser or execution is used. | — |
| G5 Recovery | Pass | The output version is 45 and validation precedes snapshot publication. | — |
| G6 Transformation | Pass, scoped | The query counts direct syntax children, so a missing ty region for an earlier action cannot make a multi-statement handler appear to have one action. | — |
| G7 Claims | Pass, scoped | DESIGN states this is conditional on handler entry and before `finally`; no catch is claimed. | — |
| G8 Library leverage | Pass | DataFusion aggregates syntax children and joins typed facts; Arrow declares the relation. | — |
| CI-G1 Fidelity | Pass | Approximate ty reachability is not relabelled as a complete handler path. | — |
| CI-G2 Evidence closure | N.a. | No served claim consumes this local witness. | Verify on FORMAT 7 use. |
| CI-G3 Evaluation integrity | Pass | Fixture cases remain separate from the FastMCP gold family reference. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L2 fate composition | A direct `return None` does not prove the modeled exception selects this clause or survives an enclosing `finally`. | CI-06, DP-08 · CI-G1 | Promoting `handler_return_none_sites` to a completed catch would misstate nested-frame and finalizer behavior. | Compose clause order, inner frames, enclosing control and exit state before publishing a fate. |

DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within this local witness scope. Graph, heuristic, recursive summary and serving duties are not claims of this slice. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Direct body cardinality | Domain predicate for sole `return None` | DataFusion `COUNT`, joins and Arrow schema | Positive and withholding source shapes passed. | Keep the bounded SQL; compose final fates separately. |

## 12. Decision

**Accept the local source witness.** The focused release selection passed 3/3 on 2026-09-25 after reviewing and accepting the new schema/rule snapshots. Handler selection/completion, integrated `just test-all`, fresh pilot, structured evaluation and formatting remain `not_run` under the operator's end-of-scope policy.
