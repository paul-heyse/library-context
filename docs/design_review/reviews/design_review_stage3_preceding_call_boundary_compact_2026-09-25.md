# Stage 3 preceding-call return boundary — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0038 direct-return admission from `summary_flow_seeds` |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a conservative interim screen; require path-sensitive predecessor proof next |

The prior SCC-wide refusal prevented a post-recursion false positive but also lost a finite
base return before recursion. This change joins pinned Ruff call and return syntax by owner
and byte order, while preserving modeled-call and assignment proof routes. Examined the
source SQL, finite producer, `summary_boundaries`, analyzed fixture and shared validator.
General predecessor effects and full SCC composition remain outside this slice.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | `call_syntax` and `syntax_nodes` in one snapshot supply source order; no parser is added. | — |
| G2 Fidelity | Pass, conservative | Earlier calls withhold direct return; a later recursive call cannot contaminate an earlier base return. Alternate branches may be over-withheld. | Replace lexical order with a path witness. |
| G3 Validity | Pass | Shared publication validation reconstructs finite summaries and remaining boundaries; focused compile passed. | — |
| G4 Hidden behaviour | Pass | Analyzed recursive source is not executed. | — |
| G5 Consistency | Pass | Omitted positives retain an explicit `summary_boundaries` unknown. | — |
| G6 Transformation | Pass, scoped | The screen uses stable source spans and is independent of provider row order. | Carry condition-compatible predecessor steps later. |
| G7 Claims | Pass, scoped | The rule is documented as interim and does not claim all preceding actions are safe. | — |
| G8 Library leverage | Pass | DataFusion anti-join constructs the bounded source relation; no second AST interpreter is introduced. | — |
| CI-G1 Fidelity | Pass, scoped | Self-recursive and nonrecursive preceding calls remain unknown; a base return before recursion is admitted. | Cover operators, attributes and exceptional exits later. |
| CI-G2 Evidence closure | Pass for emitted paths | The finite proof still cites value, condition and exit facts; unsupported earlier calls do not produce a positive proof. | — |
| CI-G3 Evaluation integrity | Pass | The analyzed fixtures and independent oracle data remain outside model inputs. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | Byte order is not execution order across branches. | DP-08, CI-07 · G6 | `summary_flow_seeds` excludes any earlier same-function `call_syntax`, even an alternate branch that cannot precede the return at runtime. | Build a condition-compatible path predecessor relation over ty flow and Ruff statement order. |
| F02, deferred | Non-call syntax can execute user code or fail before a return. | CI-06, DP-22 · CI-G1 | The screen does not classify overloaded operators, descriptors, iterators, raises or exits. | Require typed normal-outcome witnesses for each path predecessor; unknown remains a boundary. |

DP-01/02/03/04/07/08/11/13/15/19/21/22/23 and CI-01/02/03/04/06/07/08/10/11/12 are satisfied for this conservative source rule and its reconstruction. Full Stage 3 execution fidelity is unresolved at F01–F02; no empty summary is a negative. Other graph, retrieval and serving concerns are outside this review.

## 8. Library leverage

| Capability | Current owner | Alternative | Assessment |
|---|---|---|---|
| Attributed earlier-call selection | DataFusion `NOT EXISTS` over Ruff source facts | A new Rust AST walk | The relational source contract and shared validation already exist; retain DataFusion. |
| Path-sensitive normal outcome | Pending ty/Ruff/BDD composition | Lexical allowlists | The built-in flow/index and BDD owners should supply control and condition facts; more syntax allowlists would duplicate semantics. |

## 12. Decision

**Accept the interim screen.** On 2026-09-25, the focused return fixture passed for the
unconditional self-call, a nonrecursive prior call and a terminating base return; the focused
modeled fixture passed after the SQL change. Full predecessor completion, SCC worklist,
formatting and integrated Stage 3 acceptance remain open and `not_run` where applicable.
