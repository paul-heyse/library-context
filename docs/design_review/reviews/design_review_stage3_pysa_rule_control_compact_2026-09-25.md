# Stage 3 Pysa rule control — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Independent Pysa TITO oracle readiness probe |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the matched-source control; full FastMCP differential remains open |

The [probe](../evidence/2026-09-25_pysa-tito-rule/README.md) uses pinned pyre-check
0.10.0, explicit Pyrefly 1.3.1, a real source-to-sink rule, and verified models. It
checks two exact TITO ports, the source-to-sink issues, a constant counter-control and
absence of obscure-callee features on the positive ports. A byte-identical compiler fixture
publishes finite paths for the same two functions and none for the constant, with shared
publication validation.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, scoped | Pysa consumes verified model files; the compiler analyzes byte-identical source without Pysa results. | Compare pinned FastMCP paths later. |
| G2 Fidelity | Pass, scoped | The checker distinguishes positive TITO from silence and refuses obscure-positive evidence. | Classify unresolved/obscure disagreements in the differential. |
| G3 Validity | Pass | Metadata has zero model errors and rule 9001 produces two control issues. | — |
| G4 Hidden behaviour | Pass | The Python source is statically analyzed, never executed. | — |
| G5 Consistency | Pass | A local Pyrefly project and explicit binary path prevent accidental workspace configuration reuse. | — |
| G6 Transformation | Pass, scoped | The checker parses raw JSON Lines and AST call sites; it does not rewrite compiler output. | — |
| G7 Claims | Pass, scoped | Agreement is limited to two tiny positive cases and one control. | Run the pinned FastMCP comparison. |
| G8 Library leverage | Pass | Pysa's inferred TITO model and rule engine supply the independent assertion. | — |
| CI-G1 Fidelity | Pass, scoped | Exact `formal(value, position=0)` and `LocalReturn` are required. | — |
| CI-G2 Evidence closure | Partial | Raw oracle files and the compiled fixture are retained; matching is by exact source and formal, not a full path-shape comparison. | Compare richer pinned FastMCP paths. |
| CI-G3 Evaluation integrity | Pass | Counter-control checks that a constant return does not trigger the rule. | — |

## 12. Decision

**Accept the targeted differential.** On 2026-09-25, Pysa configuration verification,
targeted analysis, the JSON Lines checker and the matched-source compiler fixture passed.
The pinned FastMCP differential, formatting,
`just test-all`, fresh pilot and structured evaluation remain `not_run` until the full
Stage 3 implementation is assembled.
