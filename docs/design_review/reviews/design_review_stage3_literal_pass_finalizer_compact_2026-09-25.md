# Stage 3 literal pass finalizer — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Source-cited pending return through one literal `finally: pass` |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept ADR-0036's single-frame case; retain other exit fates as unknown |

The [Python 3.14 language reference](https://docs.python.org/3/reference/compound_stmts.html#the-finally-clause)
specifies execution of `finally` on the way out of a pending return. The DataFusion
ancestry walk admits only one frame whose entire direct finalbody is a Ruff `pass`. It
publishes the frame and pass source facts, and the summary seed still needs its prior
condition and expression-evaluation evidence. An analyzed fixture checks one positive,
a nontrivial finalizer, two nested pass finalizers and a `with` withholding case.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff syntax and the same-function ancestry own finalizer shape; ty owns return region. | — |
| G2 Fidelity | Pass, scoped | Only the sole literal pass frame is discharged. | Build ordered witnesses for multiple and effectful frames. |
| G3 Validity | Pass | Published status and summary are reconstructed by shared validation; pass-fact tampering is rejected. | — |
| G4 Hidden behaviour | Pass | The analyzed fixture is not executed; the rule uses language semantics and cited syntax. | — |
| G5 Consistency | Pass | All four finite summary seed relations consume one status relation. | — |
| G6 Transformation | Pass, scoped | Depth cap and frame count are explicit; the two evidence fields migrate the contract. | — |
| G7 Claims | Pass, scoped | A clear exit status does not claim that the return expression evaluates normally. | Preserve argument and call-completion checks. |
| G8 Library leverage | Pass | DataFusion handles the bounded relational walk and Ruff supplies statement kinds. | — |
| CI-G1 Fidelity | Pass | A `pass` nested under another statement does not masquerade as the sole finalbody child. | — |
| CI-G2 Evidence closure | Partial | The status cites pass and frame facts; the summary's `ReturnExit` step still cites only the return site. | Project the finalizer proof step into full served evidence. |
| CI-G3 Evaluation integrity | Pass | The fixture is independent analyzed source, not gold compiler input. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | Multiple pass frames still need ordered proof identity. | CI-07, CI-11 · CI-G2 | The current status withholds two nested frames even though each is individually harmless. | Compose each frame's exit step before widening. |
| F02, deferred | A simple assignment in `finally` may preserve return, but requires its own no-raise/effect proof. | CI-05, DP-20 · G2 | The `marker = 1` fixture remains unknown. | Derive an evaluated finalizer action, not a syntax-only allowlist. |

## 12. Decision

**Accept the narrow exit proof.** On 2026-09-25, focused return and model fixture tests,
shared publication validation, schema/rule snapshots and pass-fact tamper detection passed.
The `return_exit_statuses` schema migration is accepted at compiler output version 66.
Formatting, `just test-all`, the fresh pilot, structured evaluation and clean-wheel query
remain `not_run` until the full Stage 3 functionality is implemented.
