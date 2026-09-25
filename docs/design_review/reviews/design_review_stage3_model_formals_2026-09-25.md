# Stage 3 model formal paths — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Typed `model_formal_paths` compiler output, Arrow contract and shared validation |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a typed model-formal source only |

The purpose is to expose the formal referred to by an authored model path without parsing its rendered string. I read the catalog AST, all rule variants, pinned formal validation, the output schema, publication and reconstruction, and the focused catalog/tamper test. This slice does not bind a caller argument or claim a transfer, callback or resource action. The integrated gate and pilot remain `not_run`.

## 2. Authority and fidelity

| Fact or relation | Authority and identity | Fidelity and coverage | Consumer |
|---|---|---|---|
| Authored path | Committed model TOML parsed as `InputPath`, `OutputPath` or `ResourcePath` | Typed input/output role and structural path id; display is non-authoritative | One catalog compiler pass |
| Formal name | `formal()` on the parsed path AST; checked against each pinned Pysa signature | Present only for a parameter or receiver-root path; absent for return/global/raise | `model_formal_paths` |
| Published formal row | Model, pinned target, rule, role and path id in one snapshot | Synthetic assertion, not a source argument | Future L2/L3 argument binding; shared validator |

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | One typed catalog AST emits the formal; no second display grammar. | — |
| G2 Semantic fidelity | Pass, scoped | Input/output roles and rule/path identities stay distinct; no return value is mislabeled as a formal. | — |
| G3 Validity | Pass | Existing all-signature checks reject unresolved names; table and reconstructed-row checks reject forged output. | — |
| G4 Hidden behavior | Pass | Catalog bytes are embedded; no ambient model file or import. | — |
| G5 Consistency and recovery | Pass | Rows are written in the same validated attempt as other model rules. | — |
| G6 Transformation and reuse | Pass | Compiler output version 31 and path/model ids govern rebuilds. | — |
| G7 Truthful claims | Pass, scoped | The row names an authored model formal only; source argument binding remains open. | — |
| G8 Library leverage | Pass | Serde owns tagged input; Arrow/Delta own persistence; the AST's small domain accessor owns the semantic projection. | — |
| CI-G1 Fidelity | Pass, scoped | Model formal identity remains separate from source value flow. | — |
| CI-G2 Evidence closure | Not applicable | No served claim reads the new row. | Revisit at summary/FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | The committed catalog, not gold or skill text, is compiled. | — |

## 7. Findings and principle verdicts

No in-scope defect was found. The required next boundary is **D01**: an authored formal path does not identify which source argument reaches it. A summary that reads the formal row alone could attribute a callback or transfer to the wrong value. Derive a call-site mapping that agrees across all applicable signatures and refuses starred, implicit-receiver or overloaded ambiguities; otherwise preserve unknown.

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are satisfied within this typed-model source scope by the authority and gates above. DP-07/12/20 and CI-03/05/07/08/09/11/13 are not exercised by this source-only compilation; argument flow, summary and serving obligations remain.

## 8. Library-leverage ledger

| Capability | Own code | Qualified built-in | Fit and decision |
|---|---|---|---|
| Tagged model paths | `models.rs` domain enums and `formal()` | Serde tagged variants | Retain one parsed AST; never parse the display string. |
| Published contract | `ModelFormalPathsRow` | Arrow schema and shared Delta validation | Keep the formal source with the pinned model generation. |
| Future argument mapping | Not built in this slice | DataFusion joins over arguments/context signatures | Bind all signatures conservatively before behavior claims. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Model formal count, forged-name rejection, schema, rules, ledger and compiler digest | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass` | `passed` (7/7) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as a typed model-formal source only.** The catalog AST, pinned signature validation and shared reconstruction preserve the authored meaning without introducing a string interpreter. D01 is a precondition for any source-value or behavior consumer.
