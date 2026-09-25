# Stage 3 model-rule compilation — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `cpg-schema`'s committed external model catalog and five compiled rule tables; `cpg-core` publication and validation |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept for model assertions only |

**Outcome sought.** A pinned external callable can carry typed transfer, effect, callback,
resource and exception assertions without converting any of them into an observed release
behavior. Completeness is declared separately for each channel.

**Method and coverage.** I read the catalog parser and compiler, table contracts, publication
and shared validation, the committed models and focused tests. The focused release Nextest
selection of ten tests passed on 2026-09-25, including positive bindings, forged rows,
snapshot contracts and the table-count/ledger cases. Package-level Clippy with `-D warnings`
passed. The integrated repository gate, fresh pilot, summary consumer, serving and broader
oracles are outside this review and remain `not_run` or unimplemented. No performance claim is
made. The authoritative current boundary is DESIGN §9.9.

## 2. Authority and fidelity

| Fact or relation | Authority and identity | Fidelity and coverage | Consumer |
|---|---|---|---|
| Authored rule | Committed `external.toml`; catalog bytes, target pin, revision and ordinal determine ids (`models.rs:565-608`, `:737-759`) | Synthetic model; channels are complete, partial or unspecified (`models.rs:44-81`) | Model compiler |
| Pinned target | Pysa context definition and module facts, matched to exact origin and pin (`models.rs:616-704`) | Resolved context binding, dormant when not referenced | Model rule rows |
| Compiled rule | Arrow contracts (`behavior.rs:40-127`, `:227-252`); shared reconstruction (`validate.rs:218-355`) | Synthetic model assertion; paths have typed ids; modality and exit are explicit | Publication; future summary application |

The `class` on a model exception is an authored dotted name, not a resolved caught-class proof.
No current served answer reads these tables as a release behavior.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Committed catalog is the sole rule author; validator reconstructs stored rows. | — |
| G2 Semantic fidelity | Pass | Distinct rule tables, typed path ids, channel coverage, modality, exit and synthetic origin preserve the scoped meanings. | — |
| G3 Validity | Pass | Serde rejects unknown fields; formal binding checks every signature; relational and catalog-equality validators reject forged rows (`compile.rs:401-580`). | — |
| G4 Hidden behavior | Pass | `include_str!`/`include_bytes!` bind committed bytes; no ambient model file or gold path is read. | — |
| G5 Consistency and recovery | Pass | The existing attempt writes all model rows before validation and one snapshot publication (`attempt.rs:548-611`). | — |
| G6 Transformation and reuse | Pass | Rule ids include model and ordinal; typed path ids are structural; catalog bytes join compiler digest. | — |
| G7 Truthful claims | Pass, scoped | Rows are documented as model assertions; the summary and pilot are not claimed as passed. | Keep this boundary when L3 consumes them. |
| G8 Library leverage | Pass | Serde owns tagged parsing, Arrow the contract, DataFusion the shared relational validation; own code binds domain semantics. | — |
| CI-G1 Fidelity | Pass, scoped | `synthetic_model` stays separate from source observations and no row is a caught/escaped fate. | — |
| CI-G2 Evidence closure | N.a. | No served claim is produced by this slice. | Revisit at FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | The compiler reads committed model bytes, not `.claude/skills` or evaluation files. | — |

## 7. Findings and principle verdicts

There is no in-scope defect established by the examined code and focused tests. The following
boundary is deferred rather than treated as an implemented guarantee.

| ID | Deferred boundary | Trigger and consequence | Route |
|---|---|---|---|
| D01 | Exception classes are authored names, not resolved class identities. | Before a summary uses `model_exceptions` to convert, suppress or refute an exception, unresolved names could be mistaken for caught classes. | Resolve against the pinned context or retain `unknown`; test a shadowed/absent class. |
| D02 | A model's `complete` channel is an authored assertion. | Before a complete channel supports `refuted_under_model`, an incorrect model could produce a false negative. | CrossHair or exact source proof for pure models, and independent scenario checks for effectful ones. |

Applicable principle verdicts: DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and
CI-01/02/04/06/10/12 are **Satisfied within the assertion scope** by the mechanisms in the
gates and authority table. DP-07/12/20, CI-03/05/07/08/09/11/13 are not exercised by an
assertion-only compilation; their summary, graph or serving obligations are not waived.

## 8. Library-leverage ledger

| Capability | Own code | Qualified built-in | Fit and decision |
|---|---|---|---|
| Tagged model input | `models.rs`'s domain enums and binding | Serde TOML with unknown-field rejection | Keep Serde; no extra grammar/parser. |
| Rule tables and invariants | `behavior.rs`, `rules.rs`, `validate.rs` | Arrow schemas and DataFusion SQL | Keep one shared validation path; no test-only validator. |
| Model rule application | Planned L3 | DataFusion joins and petgraph SCC | Not yet in scope; use the pinned libraries at the composition boundary. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Positive binding, forged-row rejection, schemas, codebooks, rules and ledger | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass` | `passed` (10/10 in the final focused selection) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 behavior and pilot | `just test-all`; `just pilot` | `not_run`, reserved for assembled Stage 3 |

## 12. Decision

**Accept for model assertions only.** The compiled representation is typed, pinned and checked
against its one authored source. D01 and D02 become mandatory preconditions when L3 or serving
uses these rows for negative or exception-fate claims. The next ordinary model rule needs one
catalog declaration and focused evidence; a genuinely new family would need its own typed
contract and validator.
