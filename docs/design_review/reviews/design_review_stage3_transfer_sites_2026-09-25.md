# Stage 3 modeled transfer sites — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Typed transfer path kinds, source application relation, schema and validation |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as candidate-local transfer evidence only |

I traced the transfer from committed model AST and pinned target through formal binding,
source call application, Arrow/Delta publication and shared reconstruction. The fixture covers
`typing.cast` and `atexit.register`; no served transfer conclusion, recursive summary or pilot
measurement is in scope. Integrated gates remain `not_run`.

## 2. Authority and fidelity

| Fact or relation | Authority and identity | Fidelity and boundary | Consumer |
|---|---|---|---|
| Authored transfer | Committed model rule tied to target pin/revision | Typed input/output path kinds and identity/transform modality | `model_transfers` |
| Source call | Ruff call, Pysa target, resolution candidate set | Candidate target, phase and unresolved remainder | `model_applications` |
| Input expression | Pinned signature and exact source argument | Bound or unknown; no starred/implicit inference | `model_argument_bindings` |
| Output expression | Call syntax and its fact | Call result expression, not evidence of normal completion | `modeled_transfer_sites` |

## 4–5. Transformation and known-answer journey

The catalog compiler emits typed path kinds rather than a second parser of access-path
display strings. DataFusion applies the transfer to a candidate call and joins the input
formal to the exact source argument if all pinned signatures agree. A `ReturnValue` output
names the call expression. Other input/output path kinds stay unknown. The fixture asks for
the cited `typing.cast` keyword input and result, and asks an invalid positional-only
keyword and a starred `atexit.register` call to withhold their input endpoints. The shared
validator reconstructs every row; changing an endpoint status is rejected.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Model rule, source call and argument binding retain separate origins and ids. | — |
| G2 Fidelity | Pass, scoped | Input/output roles, path kinds, transfer kind, modalities and unknown endpoints are typed. | — |
| G3 Validity | Pass | Codebook/reference/shape rules and shared reconstruction cover publication. | — |
| G4 Effects | Pass | Compilation reads facts and committed models; analyzed code is not executed. | — |
| G5 Consistency | Pass | Relation is written and validated within one snapshot attempt. | — |
| G6 Transformations | Pass, scoped | Compiler version 35 names the schema migration; no summary equivalence is claimed. | — |
| G7 Claims | Pass, scoped | DESIGN and row contract distinguish candidate model transfer from observed flow. | — |
| G8 Library leverage | Pass | DataFusion does the relational work; Arrow/Delta own contract and persistence. | — |
| CI-G1 Fidelity | Pass, scoped | Unknown binding does not become identity flow; open dispatch survives. | — |
| CI-G2 Evidence closure | Not applicable | No served conclusion consumes this relation yet. | Revisit in L3/FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Fixture is independent of model catalog and gold reference. | — |

## 7. Findings and principle verdicts

No in-scope defect was established. **Deferred T01:** a candidate modeled identity transfer
does not prove that the source call returned normally or that all possible dispatch targets
share the transfer. L3 must join condition, exit and target-set coverage before changing a
`call_transfer` verdict. **Deferred T02:** non-parameter inputs and non-return outputs remain
unknown until a cited field/global/raised-value bridge exists. Missing rows do not refute a
transfer.

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are
satisfied within this candidate-site scope. DP-07/12/20 and CI-03/05/07/08/09/11/13 are
obligations for summary recursion and serving.

## 8–9. Library leverage and alternatives

| Alternative | Fit | Decision |
|---|---|---|
| Parse rendered path spellings | Duplicates the tagged catalog grammar and risks silent drift. | Reject. |
| Use DataFusion joins over typed model/call/binding rows | Preserves provenance and unknowns with no generic join implementation. | Use. |
| Directly turn model assertion into a caller verdict | Omits dispatch, path condition and return completion. | Reject. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Transfer endpoints, unknown shapes, tamper, schema/rules/ledger | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass --no-fail-fast` | `passed` (7/7) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as candidate-local transfer evidence only.** T01 and T02
bar whole-operation flow or negative claims until the summary layer supplies their proofs.
