# Stage 3 modeled argument evaluation — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Ordered `modeled_argument_evaluations` for exact value-transfer candidates; `semantic:call-argument-coverage` |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as candidate-local evidence; defer call-completion claims |

I inspected the Arrow contract, derivation, source-equality validator, argument coverage rule, model-shape fixture and reviewed snapshots. This review does not certify arbitrary expression completion, callee evaluation, nested calls, exception paths or whole-call normal return.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff argument and syntax facts own source order and literal shape; the raw flow fact owns the candidate operand path. | — |
| G2 Fidelity | Pass, scoped | `SourceOperand`, `LiteralNormal` and `Unknown` distinguish candidate path evidence from a normal literal; `condition_id` stays attached. | Do not promote a source operand into a universal normal-exit proof. |
| G3 Validity | Pass | Shared source-equality reconstruction rejects missing/forged rows; call-argument coverage detects missing argument rows and non-dense ordinals. | — |
| G4 Hidden behaviour | Pass | One declarative DataFusion relation; no second Python expression evaluator. | — |
| G5 Consistency | Pass | Output version 59, append-only codebook and reviewed schema/rule snapshots identify the migration. | — |
| G6 Transformation | Pass | Rows retain candidate, rule, call, argument, ordinal, status and evidence identity. | — |
| G7 Claims | Pass, scoped | Unknown siblings remain explicit. The table cannot itself assert whole-call completion. | Add callee, argument-exit and enclosing-exit witnesses before summary promotion. |
| G8 Library leverage | Pass | Arrow contract, DataFusion window cardinality, relation and shared validation replace a custom walker. | — |
| CI-G1 Fidelity | Pass, scoped | A dynamic sibling is unknown while a direct literal sibling is cited. | Challenge nested/raising calls before a positive summary. |
| CI-G2 Evidence closure | Not applicable | No served answer consumes these rows yet. | Trace proof steps through FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Local fixture tests the derivation without using gold as compiler input. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | The raw source-operand flow and direct literal syntax witness are local facts; neither proves every earlier argument or callee expression reaches this call normally. | DP-08, CI-06 · CI-G1 | An immediate positive `ModelTransfer` summary could omit a raising sibling or callee. | Require complete ordered normal-evaluation and exit witnesses; retain unknown otherwise. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/07/10/12 are satisfied for the candidate-local relation. Graph, heuristic and serving principles are outside this slice. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Built-in feature | Fit |
|---|---|---|---|
| Candidate-local argument enumeration | Status meanings and exact evidence contract | DataFusion joins, `count(*) OVER (PARTITION BY ...)`, Arrow table contract | The window rejects ambiguous same-span literal matches without a correlated subquery unsupported in the physical join plan. |
| Stored/derived equality | No duplicate semantic implementation | Existing shared relation validator | The publication check uses the same derivation as compilation. |

## 12. Decision

**Accept this local evidence layer.** `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E 'test(derivations_snapshot) | test(rules_snapshot) | test(registry_snapshot) | test(references_name_real_columns) | test(keys_and_checks_name_real_columns) | test(pinned_identity_models_require_and_publish_their_real_formals)' --no-tests=pass --no-fail-fast` passed 6/6 on 2026-09-25 after snapshot review. Targeted release Clippy, `just adr lint` and `git diff --check` passed. Formatting, full integrated gates and pilot remain `not_run` until the functional Stage 3 scope is implemented.

### Same-family extension (2026-09-25)

**Tested in focused cases:** an exact unshadowed builtin-name argument now cites its lexical
`reference_resolutions` fact as `builtin_name_normal`; a same-spelled formal remains `unknown`.
The append-only codebook, schema check and generated evidence reference were reviewed. Focused
release Nextest (`contracts_snapshot`, `registry_snapshot`, `rules_snapshot`,
`derivations_snapshot`, `pinned_identity_models_require_and_publish_their_real_formals`) passed
5/5; targeted release Clippy, ADR lint and diff check passed. G1–G8 and CI-G1–G3 retain the
scoped verdicts above; F01 remains open. Output version 60 marks the migration. Integrated
testing and formatting remain `not_run`.
