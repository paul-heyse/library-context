# Stage 3 return exit status — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Bounded same-function return ancestry and finite summary admission |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the local normal-return candidate boundary; broader L2 fates remain open |

I traced `exit_sites` through the bounded recursive syntax walk to the persisted
`return_exit_statuses` table, its shared source-equality validator, and all four finite value
summary seed relations. The structural fixture distinguishes an ordinary nested branch from a
pending `finally` and a `with` frame. The analyzed fixture confirms that a nested branch may
produce a finite identity summary while both controlling frames withhold one. The latter retain
an explicit summary boundary; no handler or context-manager completion is inferred.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff syntax ancestry, ty return region and attributed source facts are persisted; the shared validator reconstructs every status row. | — |
| G2 Fidelity | Pass, scoped | A status without a controller is only a candidate. Pending `finally`, `with` and capped ancestry cannot produce a positive finite summary. | Prove more normal/exceptional frame outcomes in later L2 slices. |
| G3 Validity | Pass | The new Arrow contract, key, nonnegative depth check, codebook check and source equality are tested; doctored depth is rejected. | — |
| G4 Hidden behaviour | Pass | The source is analyzed but never executed; fixture runtime behavior is not a compiler input. | — |
| G5 Consistency | Pass | Compiler output version 65 and accepted contract/rule snapshots name the migration. | — |
| G6 Transformation | Pass, scoped | The recursive walk is capped at 128 ancestors, constrained to one owning function, and picks the nearest controller deterministically. | Record deeper-frame propagation when implemented. |
| G7 Claims | Pass, scoped | A positive summary remains a may-path under its BDD condition, not a guarantee of a concrete execution. | — |
| G8 Library leverage | Pass | DataFusion recursive CTE/window ranking and Arrow contracts express the relation without a second AST walker. | — |
| CI-G1 Fidelity | Pass | The nearest frame identity and source fact survive; no relation is relabelled as a completed exit. | — |
| CI-G2 Evidence closure | Pass, scoped | Each stored status cites the return and controlling frame; publication checks source equality. | Add handler/callback/resource exit chains separately. |
| CI-G3 Evaluation integrity | Pass | The analyzed source fixture is not the FastMCP evaluation gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | The relation does not prove a `with` exit or a `finally` suite completes. | CI-05, CI-10 · G2 | Such returns stay unknown even when a concrete run would return. | Derive normal/exceptional completion with effect and exit witnesses. |
| F02, deferred | Async context-manager syntax and nested handler propagation are outside this local status. | CI-03, DP-20 · G2 | No negative conclusion follows from their absence. | Extend the L2 frame model and explicit boundaries. |

No SHOULD exception is requested. The append-only codebooks were unchanged.

## 12. Decision

**Accept the return-status and summary-admission slice.** On 2026-09-25, the focused compiler
positive/withholding and tamper cases, schema/rule snapshots and targeted Clippy passed. The
full `just test-all`, fresh pilot, structured Stage 3 evaluation, clean wheel and formatting
remain `not_run` until the entire Stage 3 functionality is implemented.
