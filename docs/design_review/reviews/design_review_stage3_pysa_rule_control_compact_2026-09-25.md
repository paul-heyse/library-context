# Stage 3 Pysa rule control — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Independent Pysa TITO oracle readiness probe |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the oracle control; full FastMCP differential remains open |

The [probe](../evidence/2026-09-25_pysa-tito-rule/README.md) uses pinned pyre-check
0.10.0, explicit Pyrefly 1.3.1, a real source-to-sink rule, and verified models. It
checks two exact TITO ports, the source-to-sink issues, a constant counter-control and
absence of obscure-callee features on the positive ports.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, scoped | Pysa consumes its own source and verified model files; no compiler fact is an input. | Compare against pinned compiler summaries later. |
| G2 Fidelity | Pass, scoped | The checker distinguishes positive TITO from silence and refuses obscure-positive evidence. | Classify unresolved/obscure disagreements in the differential. |
| G3 Validity | Pass | Metadata has zero model errors and rule 9001 produces two control issues. | — |
| G4 Hidden behaviour | Pass | The Python source is statically analyzed, never executed. | — |
| G5 Consistency | Pass | A local Pyrefly project and explicit binary path prevent accidental workspace configuration reuse. | — |
| G6 Transformation | Pass, scoped | The checker parses raw JSON Lines and AST call sites; it does not rewrite compiler output. | — |
| G7 Claims | Pass | The probe is labelled oracle readiness, not compiler agreement. | Run matched-source comparison. |
| G8 Library leverage | Pass | Pysa's inferred TITO model and rule engine supply the independent assertion. | — |
| CI-G1 Fidelity | Pass, scoped | Exact `formal(value, position=0)` and `LocalReturn` are required. | — |
| CI-G2 Evidence closure | Partial | Raw issue/model/metadata files are retained through LFS; no compiler proof path is compared yet. | Join on the same pinned source and formal. |
| CI-G3 Evaluation integrity | Pass | Counter-control checks that a constant return does not trigger the rule. | — |

## 12. Decision

**Accept the control.** On 2026-09-25, Pysa configuration verification, targeted
analysis and the JSON Lines checker passed. The pinned FastMCP differential, formatting,
`just test-all`, fresh pilot and structured evaluation remain `not_run` until the full
Stage 3 implementation is assembled.
