# Stage 3 model argument binding — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `model_argument_bindings` relation, Arrow/codebook contracts, publication validation and source fixture |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as candidate-local argument identity only |

The target is a positive source argument for an authored model formal when every pinned Pysa signature maps it to the same explicit call argument. I read the typed model-formal producer, Pysa signature and source argument contracts, DataFusion relation, validation path, and focused positive/withholding/tamper cases. The valid keyword case uses `typing.cast(val=...)`; the pinned `atexit.register(func, /, ...)` signature correctly withholds `func=`. No transfer, callback, resource or served summary is claimed by this slice.

## 2. Authority and fidelity

| Fact or relation | Authority and identity | Fidelity and coverage | Consumer |
|---|---|---|---|
| Model formal | Committed catalog AST and every pinned Pysa signature | Typed path and formal, independent of rendered spelling | Argument relation |
| Candidate source call | Ruff `call_syntax`/`arguments` and Pysa target fact | Target may be potential or open; application keeps that status | Argument relation |
| Argument binding | One source argument ordinal agreed by every pinned signature | `bound` cites argument id/fact; `unknown` has a boundary reason | Future L2/L3 |

A source argument binding is local to the candidate modeled target. It is not a claim that the callee ran, that the value stayed stable, or that a modeled rule completed.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Catalog AST, context signatures and source argument facts each own their meaning; one relation joins them. | — |
| G2 Semantic fidelity | Pass, scoped | Positional-only, positional-or-keyword and keyword-only kinds are respected; stars and implicit receivers withhold. | — |
| G3 Validity | Pass | Typed status/reason shape, references and shared row reconstruction reject tampering. | — |
| G4 Hidden behavior | Pass | No runtime call, source-text name parser or ambient signature lookup. | — |
| G5 Consistency and recovery | Pass | The relation is written and validated with one attempt snapshot. | — |
| G6 Transformation and reuse | Pass | Model path/target/call identities and compiler version 32 prevent stale cross-generation binding. | — |
| G7 Truthful claims | Pass, scoped | `bound` means same explicit argument across signatures for one target; uncertain calls remain unknown. | — |
| G8 Library leverage | Pass | DataFusion joins and aggregates decide signature agreement; Arrow/Delta persist the typed result. | — |
| CI-G1 Fidelity | Pass, scoped | A starred or invalid keyword call does not become an argument flow. | — |
| CI-G2 Evidence closure | Not applicable | No served behavioral or compatibility verdict reads this row yet. | Revisit at L3/FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Test fixtures are independent of the committed model compiler and gold families. | — |

## 7. Findings and principle verdicts

No in-scope defect was established. These are deferred prerequisites for consumers:

| ID | Boundary | Trigger and consequence | Route |
|---|---|---|---|
| D01 | A bound argument is not a value-stability proof. | A modeled identity transfer could be attributed to a value changed before its sink. | Compose with `flow_values`/entry links and effects under a cited condition. |
| D02 | A bound argument is per target, even when dispatch is open. | A summary could incorrectly state a definite callback or effect. | Join `model_applications` completeness/modality and all candidate summaries; unknown on an open remainder. |
| D03 | Varargs and implicit receivers are withheld. | Their source value may be knowable but is not bound by this first exact mapper. | Add separate positive mappings with positional expansion or receiver proof, never a fallback by spelling. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are satisfied for this candidate-local relation by the gates and authority table. DP-07/12/20 and CI-03/05/07/08/09/11/13 remain obligations for condition composition, graph summaries and served claims.

## 8. Library-leverage ledger

| Capability | Own code | Qualified built-in | Fit and decision |
|---|---|---|---|
| Signature agreement | Domain eligibility SQL | DataFusion joins, grouping and `COUNT(DISTINCT)` | Keep a declared relation; no second model parser or custom signature walker. |
| Published contract | Status/reason and citations | Arrow schema, Delta snapshot and shared validator | Preserve unknown rows; no silent inner-join loss. |
| Future summary | Not in this slice | petgraph SCC and condition kernel | Consume bindings only with candidate and effect closure. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Positional/keyword positive, positional-only/star withholding and tamper | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass` | `passed` (8/8 selection) |
| Schema/codebook/rules/ledger and compiler digest | Same focused Nextest selection | `passed` |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as candidate-local argument identity only.** The relation gives an exact argument for a pinned model formal when all signature forms agree, and keeps unresolved cases explicit. D01–D03 bar promotion to a behavioral verdict without further evidence.
