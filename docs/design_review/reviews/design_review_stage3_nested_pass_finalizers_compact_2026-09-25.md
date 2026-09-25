# Stage 3 ordered pass finalizers — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0037, bounded return status and finite direct/modeled summary proof steps |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept this pass-only exit slice; preserve other finalizer fates as unknown |

**Outcome and baseline.** ADR-0036 admitted one literal `finally: pass` but withheld two
nested passes because its status had one nullable pass slot. This change makes the status
admit any depth-bounded chain only when every pending frame is a sole pass. The direct and
modeled summaries cite every pass in inner-to-outer order. Examined the two source queries,
finite producer, shared validator, direct/modeled fixtures and a separate CPython monitoring
control. This review does not assess handler suppression, arbitrary finalizer effects,
operation-level serving or the full Stage 3 pilot.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff syntax facts and the bounded same-function ancestry decide frame shape (`behavior.rs:2800-2864`). | — |
| G2 Fidelity | Pass, scoped | `unsafe_count` withholds `with` or any non-pass finalizer; one-pass nullable fields keep their original meaning (`behavior.rs:2820-2855`). | Keep complex finalizers unknown. |
| G3 Validity | Pass | The shared validator regenerates statuses and finite proof steps; removing or swapping pass steps fails `summary-flow-step-source-equality` (focused compile test). | — |
| G4 Hidden behaviour | Pass | The compiler reads analyzed source; an independent CPython worker alone executes generated oracle code. | — |
| G5 Consistency | Pass, scoped | The ancestor walk has an explicit cap and writes through the existing snapshot attempt. The proof query runs only on clear statuses. | — |
| G6 Transformation | Pass | `return_site_fact_id` indexes ordered steps and the canonical summary id hashes them (`summaries.rs:207-224,311-329,415-445`). | — |
| G7 Claims | Pass, scoped | The frame status is a local normal-exit candidate; the modeled producer still requires evaluated arguments, target closure and normal-return assertion. | Do not promote remaining call paths from this frame result alone. |
| G8 Library leverage | Pass | DataFusion owns the bounded relational walk and window ordering; Rust only groups the rows and builds existing typed proof steps. | — |
| CI-G1 Fidelity | Pass, scoped | No effectful frame becomes pass-only; the mixed outer-finalizer and `with` fixtures retain unknown. | — |
| CI-G2 Evidence closure | Pass for these canonical rows | Each positive path cites the source pass facts, and publication rejects reordered evidence. FORMAT 7 source spans are outside this slice. | Complete FORMAT 7 before serving the claim. |
| CI-G3 Evaluation integrity | Pass | The monitored program is generated oracle input, separate from compiler fixtures and gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | The exit-status and pass-step queries independently express the `try/finalbody/pass` shape. | DP-06, DP-16 · G8 | A future widening to other harmless actions could update only one query and leave a clear status with missing proof. Current sole-pass fixtures and shared validation cover this slice. | If another exit action is added, derive frame actions once in a typed relation and consume it for both status and proof. |
| F02, deferred | Effectful finalizers and context-manager exits still lack a normal-completion witness. | CI-06, CI-07 · CI-G1 | `nested_effectful_finalizer` and `with_identity` remain unknown; claiming their return value would be ungrounded. | Add source/model effect and exit witnesses before widening. |

Applicable DP-01/02/03/04/05/07/08/11/13/15/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/11/12 are satisfied within this bounded change by the cited source relations, typed step contract, cap and shared reconstruction. DP-06 and DP-16 remain unresolved for future exit-action extension as F01. Other principles and graph/retrieval journeys do not bear on this local exit rule. The `return_exit_statuses` schema and append-only `FinalizerPass` code remain unchanged; compiler output version 70 changes derivation identity.

## 8. Library leverage

| Capability | Current owner | Alternative | Assessment |
|---|---|---|---|
| Bounded syntax ancestry, safe-frame count and execution ordering | DataFusion recursive CTE, filtered window count and row number | A second Rust AST walker | DataFusion already owns source joins and the compiler has no second Python parser; retain the relational implementation. |
| Python pending-return semantics | Local pass-only rule, checked with CPython 3.14 monitoring | General-purpose static exception engine | No adopted library proves arbitrary Python finalizer completion from this fact set; keep wider cases unknown. |

## 12. Decision

**Accept the bounded slice.** On 2026-09-25, the focused direct, modeled and exit-site
compile tests passed; the direct test verified ordered pass source facts, mixed-frame
withholding and shared-validator rejection of missing/reordered steps. A separate
`sys.monitoring` nested-pass oracle, targeted Clippy and Ruff checks passed. Formatting,
`just test-all`, a fresh pilot, structured evaluation and clean-wheel serving remain
`not_run` until the entire Stage 3 functional scope is implemented.
