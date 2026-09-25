# Stage 3 source-to-model application — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `model_applications` Arrow relation, DataFusion producer, publication validation and source/model fixture |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as site-local application evidence only |

The change links a release call-target fact to an applicable pinned external model, while preserving candidate-set openness. I read the target/resolution/model source tables, relation, schema, publication and validator, and the focused positive, shadowing and tamper case. The 2026-09-25 release Nextest selection passed 7/7, including snapshots, ledger, model calls and forged completeness. L2 callback/resource action, argument binding, finite summaries, serving, integrated tests and the pilot remain outside this slice.

## 2. Authority and fidelity

| Fact or relation | Authority and identity | Fidelity and coverage | Consumer |
|---|---|---|---|
| Call site and target | Ruff `call_syntax`; Pysa `call_targets`/fact, keyed by call and Pysa fact | Potential/definite target with original modality, origin and phase | `model_applications` |
| Target-set status | `resolutions` for the same call fact | Complete or open, with unresolved remainder retained | `model_applications` |
| Pinned model | Committed catalog bound to `context_definitions` and exact pin | Synthetic assertion, revision and cited target/module facts | `model_applications`; future L2/L3 |
| Application row | One call-target/model pair under the snapshot | Evidence of applicability only; no callback, resource or transfer fate | Publication validator; future L2/L3 |

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Source call/Pysa target and committed pinned catalog have separate owners; the join cites both. | — |
| G2 Semantic fidelity | Pass, scoped | Target modality, origin, phase, model revision and completeness remain typed; higher-order argument targets and annotation calls are excluded. | Do not promote to a completed action. |
| G3 Validity | Pass | Key/referential/codebook rules, resolution-consistency rule and shared reconstruction reject forged rows. | — |
| G4 Hidden behavior | Pass | Models come from committed bytes; the join reads only published attempt tables. | — |
| G5 Consistency and recovery | Pass | The application is written before shared validation and one snapshot publication. | — |
| G6 Transformation and reuse | Pass | Compiler output version 30 and the relation/schema digest invalidate stale results. | — |
| G7 Truthful claims | Pass, scoped | DESIGN and row docs state applicability, not callback invocation or release. | — |
| G8 Library leverage | Pass | DataFusion owns the exact join; Arrow and Delta own the table/persistence; no alternate parser or graph is introduced. | — |
| CI-G1 Fidelity | Pass, scoped | A shadowed `open` cannot acquire the builtin model; open candidate sets stay visible. | — |
| CI-G2 Evidence closure | Not applicable | No served summary or compatibility verdict reads this relation yet. | Revisit at L3/FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Fixture is isolated from compiler inputs and the capability gold. | — |

## 7. Findings and principle verdicts

No in-scope defect was established by inspection or the focused cases. Two dependencies remain before behavior can be claimed:

| ID | Deferred boundary | Trigger and consequence | Route |
|---|---|---|---|
| D01 | A model formal is not yet bound to a source argument/value. | A callback or transfer summary that reads this row alone could attribute the action to the wrong value. | Map every applicable signature to the same exact argument/use, or retain unknown. |
| D02 | An applicable target does not close dispatch or prove modeled effects occurred. | An open target set or an unproven exit could yield a false definite behavior. | Compose call modality, target-set status, condition and exit in L2/L3. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are satisfied within the site-local application scope by the authority table and gates. DP-07/12/20 and CI-03/05/07/08/09/11/13 remain obligations of the later argument mapping, graph composition and serving work.

## 8. Library-leverage ledger

| Capability | Own code | Qualified built-in | Fit and decision |
|---|---|---|---|
| Source/model join | Domain eligibility SQL in `behavior.rs` | DataFusion relation and typed Arrow result | Keep one relation; no copied Rust resolver. |
| Provenance and publication | Existing shared validator and attempt writer | Arrow/Delta snapshot tables | Reconstruct before publication; no second source. |
| Recursive summary | Not yet implemented | petgraph SCC and bounded condition kernel | Consume this row with target-set status; do not infer a summary from the join alone. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Four pinned source calls, shadowed builtin withheld, forged completeness rejected, schema/rules/ledger | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass` | `passed` (7/7) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as site-local application evidence only.** Exact source/Pysa/model identities and the candidate-set boundary survive publication. D01 and D02 prevent this row alone from becoming a callback, resource or transfer verdict.
