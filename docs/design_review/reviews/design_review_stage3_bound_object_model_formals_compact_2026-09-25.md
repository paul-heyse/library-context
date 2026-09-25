# Stage 3 bound object-method model formals — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Pinned object-method argument binding and `Logger.warning` candidate effect |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept ADR-0035 within its object-receiver/direct-attribute boundary |

The Pysa `true_with_object_receiver` target flag and Ruff direct attribute-callee syntax
together authorize a one-slot positional shift past `self`. A keyword still matches its
formal by name. The fixture demonstrates a bound `logging.Logger.warning(msg)` message,
an untyped receiver without the pinned effect, and an unpacked message that retains
`unsupported_unpacking`. [Python's logging documentation](https://docs.python.org/3/library/logging.html)
and the pinned CPython 3.14.7 signature support only a potential log effect; a disabled
logger need not emit anything.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Pysa supplies receiver kind, Ruff supplies callee shape, pinned context supplies formal ordinal. | — |
| G2 Fidelity | Pass, scoped | Only object-bound direct attribute calls shift; class receivers and unpacking remain unknown. | Add distinct proofs for other descriptors if needed. |
| G3 Validity | Pass | The analyzed fixture checks the actual Pysa receiver code and source-bound message; shared validation reconstructs rows. | — |
| G4 Hidden behaviour | Pass | The relation is a DataFusion join over published facts; no Python call is executed. | — |
| G5 Consistency | Pass | Publication and validator use the same binder relation. | — |
| G6 Transformation | Pass | One typed ordinal adjustment applies after exact target/signature selection. | — |
| G7 Claims | Pass, scoped | The effect remains potential and candidate local; no call completion or log emission is asserted. | Keep operation-level claims unknown pending L2/L3. |
| G8 Library leverage | Pass | DataFusion joins and Ruff/Pysa facts replace a bespoke Python call binder. | — |
| CI-G1 Fidelity | Pass, scoped | Method target and formal identity stay distinct; shadowed receiver has no pinned application. | — |
| CI-G2 Evidence closure | Partial | The message argument and model rule are cited; logger configuration and handler execution are not. | Derive those fates before a completed log summary. |
| CI-G3 Evaluation integrity | Pass | Source fixture is analyzed, not executed or drawn from the gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | A class receiver or dynamic descriptor may have a different implicit slot. | CI-02, CI-05 · G2 | The current Pysa enum distinguishes it, and the binder withholds it. | Add a separate cited contract only when an in-scope consumer needs one. |
| F02, deferred | Logger handlers and levels control actual emission. | DP-20, CI-06 · G7 | `log` is potential and effect coverage partial. | Use L2 effects and configuration evidence before discharging behavior. |

## 12. Decision

**Accept the scoped binder.** On 2026-09-25, the analyzed fixture, catalog tests,
derivation snapshot and targeted Clippy passed. The fixture invokes shared publication
validation and checks positive/withholding cases. Formatting, `just test-all`, the fresh
pilot, structured evaluation and clean-wheel query remain `not_run` until the full Stage 3
functionality is implemented.
