# Stage 3 handler-type source — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `handler_types` source relation, its published Arrow contract, derivation and shared validation |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as a positive class-identity source only |

**Outcome sought.** Preserve every authored `except` clause while identifying a class only when an exact lexical built-in reference resolves to one pinned typeshed class. A bare clause and any unsupported or ambiguous type expression retain distinct, explicit statuses. No row asserts that a raise is caught.

**Method and coverage.** I read the source relation and its dependencies, the Arrow schema, codebook and rules, the publication and reconstruction paths, and the focused fixture. The 2026-09-25 release Nextest selection passed 10/10; package Clippy passed with warnings denied. Handler matching, tuple/attribute/alias class resolution, exception fates, summary propagation and serving are outside this slice. Integrated tests and the pilot remain `not_run`.

## 2. Authority and fidelity

| Fact or relation | Authority and identity | Fidelity and coverage | Consumer |
|---|---|---|---|
| Authored handler | Ruff `handler_clauses` with source and flow citations | Includes bare, typed and unsupported expressions | `handler_types` |
| Lexical resolution | Exact `ExprName` reference and reference resolution, with no lexical binding | Positive built-in name only; shadowing withholds the class | `handler_types` |
| Pinned class | One `context_definitions` class in bundled typeshed's `builtins` module | Class identity, not match or completion | Publication and shared validator |
| Unknown status | `HandlerTypeStatus::Unknown` and a `BoundaryReason` | Explicitly preserves resolution failure | Future L2 fate derivation |

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Ruff syntax, lexical resolution and pinned context facts each own one source fact; the new relation joins them. | — |
| G2 Semantic fidelity | Pass, scoped | Bare, pinned and unknown are separate; unknown carries a reason. | Keep catch decisions separate. |
| G3 Validity | Pass | Referential/status checks and shared reconstruction validate stored rows; the tamper test rejects a forged status. | — |
| G4 Hidden behavior | Pass | No runtime import or ambient class lookup is used. | — |
| G5 Consistency and recovery | Pass | The attempt publishes the new relation before one snapshot validation/publication. | — |
| G6 Transformation and reuse | Pass | The row cites handler, reference, resolution and context facts; compiler version and schema migrate together. | — |
| G7 Truthful claims | Pass, scoped | DESIGN calls this a class source, with no catch verdict. | — |
| G8 Library leverage | Pass | DataFusion owns the join/window relation, Arrow its contract, and the existing validator owns reconstruction. | — |
| CI-G1 Fidelity | Pass, scoped | Shadowed and unsupported forms stay unknown; a syntactic handler is not relabelled as a handled exception. | — |
| CI-G2 Evidence closure | Not applicable | This relation produces no served verdict. | Revisit for summary/FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | The fixture probes source forms and tampering; it is not an analysis input or gold-derived producer. | — |

## 7. Findings and principle verdicts

No in-scope defect was established by source inspection and the focused cases. The following are required boundaries for the later L2 consumer.

| ID | Deferred boundary | Trigger and consequence | Route |
|---|---|---|---|
| D01 | A class identity is not a catch verdict. | A future summary that converts a raise solely because a named handler exists would falsely claim suppression across candidate/open dispatch or control-flow paths. | Resolve raised class, handler order, region and exit action with cited conditions; otherwise unknown. |
| D02 | Tuple, attribute and aliased handler expressions are unresolved. | Treating the current unknown rows as no catch would falsely claim escape. | Add positive resolvers with exact provenance or preserve unknown. |

Applicable DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are **Satisfied within this source-identity scope** by the gates and authority table. DP-07/12/20 and CI-03/05/07/08/09/11/13 are not exercised by this source-only relation; their behavior, graph and served-claim obligations remain open.

## 8. Library-leverage ledger

| Capability | Own code | Qualified built-in | Fit and decision |
|---|---|---|---|
| Resolution join | Domain-specific exactness and status policy | DataFusion joins and window counts | Keep the relational producer and avoid a second ad-hoc Rust resolver. |
| Published type contract | `HandlerTypeStatus` and boundary meaning | Arrow table/schema, shared DataFusion validation | Keep the codebook append-only and validate the one canonical row shape. |
| Future exception fate | Not built here | DataFusion relations plus condition kernel and petgraph summaries | Do not promote the source row into a fate prematurely. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Source status, withholding, tamper, schema/rules and publication cases | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass` | `passed` (10/10) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as a positive class-identity source only.** The distinct statuses and cited source/pin identity are adequate for later L2 use. D01 and D02 must be honored by exception-fate and summary consumers.
