# Stage 3 exact model value steps — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `modeled_exact_value_transfers`, replacing the return-only model bridge |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the whole-expression model step for return and definition sinks |

The source/model join now applies to a whole definition value as well as a whole return expression. It retains the same exact one-call, unique Ruff argument, unchanged upstream parameter and model endpoint requirements; sink kind/span are explicit. The assigned-result fixture can therefore cite a model step on the assignment value and a separate reaching-definition edge to the later return. These rows remain candidates, and no completed flow or operation verdict follows. Focused Nextest (`contracts_snapshot`, `rules_snapshot`, `pinned_identity_models_require_and_publish_their_real_formals`) passed 3/3 on 2026-09-25 after review and acceptance of the schema/rule snapshots. Targeted `cargo clippy --release -p cpg-schema -p cpg-core --all-targets -- -D warnings` and `just adr lint` passed. Integrated gates remain deferred by operator direction.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Raw value fact and call step, Ruff call/argument and pinned model target remain cited. | — |
| G2 Semantic fidelity | Pass, scoped | Call span equals entire sink value for both kinds; nested/outer computations remain withheld. | — |
| G3 Validity | Pass, scoped | Arrow contract, generated references and shared full reconstruction cover the replacement table. | — |
| G4 Hidden behaviour | Pass | DataFusion join uses persisted facts; no source text parser. | — |
| G5 Recovery | Pass, scoped | Output version 52 identifies the schema migration. | Run full publication gate at Stage 3 end. |
| G6 Transformation | Pass, scoped | Sink kind/span and parent fact survive; predecessor edge can join by raw fact id. | Compose conditions and exits before a summary. |
| G7 Claims | Pass, scoped | DESIGN labels it a model step candidate; assigned result is not automatically a return flow. | — |
| G8 Library leverage | Pass | One DataFusion relation covers both sink kinds with an exact span predicate. | — |
| CI-G1 Fidelity | Pass | No joined row borrows a sibling call's model input or an outer expression's result. | — |
| CI-G2 Evidence closure | N.a. | No served claim consumes the new step. | Check FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Source fixture is independent of gold capability families. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred to L3 | A definition model step and may-compatible predecessor edge do not yet prove the call's normal completion or all exit paths. | DP-08, CI-06 · G7 | The relation retains model/target modality and open dispatch; the later edge has its own compatibility status. | Compose only after candidate coverage and exit evidence; otherwise unknown. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within this source-step scope. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Exact model step | Domain endpoint and whole-value predicates | DataFusion equijoins and filtered sink codebook | One relation serves return/definition cases without a second evaluator. | Keep generic step for L3. |

## 12. Decision

**Accept the generic whole-value model step after focused verification.** Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until full Stage 3 functionality is ready.
