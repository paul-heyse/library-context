# Phase 4 foundation design and conformance review

**Decision: Accept scoped — 2026-10-01.**

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Production baseline | Frozen commit `0a6095479bb0d9b1131ec954e8007d99eb753c3e`, inspected in `/home/paul/.cache/library-context-phase4-foundation-review`; clean tree |
| Standard | Repository core/template 3.2; code-intelligence principles/review 1.3; library-context binding; `design-review` and its code-intelligence profile |
| Tier · purpose | Design · conformance, bounded to the scheduled R0/R1 foundation checkpoint |
| Reviewer · date | Independent delegated design reviewer, 2026-10-01 |
| Authorities | Phase 4 detailed plan §§3, 5, 10–13; DESIGN §15; ADR-0105/0106/0108 |
| Supported review scope | Vocabulary publication, immutable producing-owner contracts, native premise inventory, finite metrics, captured source receipts, admitted coverage frontiers, qualified support and exact cross-owner discharge |
| Adjacent consumers | N0 authored configuration/native context and preparation publication; integrated N1 receiver contracts |
| Exclusions | Downstream producer acceptance, isolated Dispatch/C0 implementation, complete Phase 4 assembly/retirement, Phase 5, PR6, library pilots/comparisons, RSS and embedding campaigns |
| Method | Independent source/type/contract inspection and inspection of supplied test sources and raw receipts. No tests, source edits, report edits, commits or delegation performed by this reviewer during the review; subsequent publication is limited to this report at the coordinator's explicit instruction. |

The conclusion permits dependent implementation to proceed against the reviewed foundation. It does not establish Phase 4 completion or release qualification. Architectural observations below are **Interface-checked/Implemented, 2026-10-01**. Test claims retain their supplied dates and execution scope.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and contract |
|---|---|
| `lctx-model::domain::stages` | Finite publication boundaries, schedule-bound prefix positions, ordinary writer uniqueness, acknowledged completion and read authority |
| `lctx-model::domain::analysis` | Finite nominal owners; invocation, support, qualification, expected-domain, coverage and discharge meaning |
| `domain::analysis::native` | Mechanical projection of actual native assertion/support pairs, preserving their native attribution and qualification |
| `domain::models` | Immutable parsed authored meaning, catalog identity, declarations and dependency requirements |
| `lctx-postgres::generations` | Private ingestion, transactional close, physical prefix views, grants, stored-content checks and receipts |
| `cpg-core::analysis_prepare` | Compose declared configuration and native-inventory operations with admitted reads and typed publication |
| Normalized receiver owner | Derive and validate expression-relative `ClassOf` evidence without replacing raw receiver uncertainty |

Mechanisms depend on semantic declarations. PostgreSQL executes model-owned publication checks; it does not define analysis completeness or evidence strength independently.

The decisive distinctions are represented and used: named boundary versus physical ordinal; computed delta versus acknowledged completion; native provider attribution versus analysis derivation; computation completion versus evidence completeness; requested empty domain versus missing observations; nominal predecessor subject versus current computation; documentary/heuristic evidence versus behavioral proof.

### Fact and fidelity

| Family | Fidelity and identity | Coverage/uncertainty | Consumer |
|---|---|---|---|
| Native inventory | Exact nominal assertion/support pair; original qualification and fidelity retained | Display-only evidence remains unresolved; absent companions refuse | Qualified analysis support |
| Owner-specific derivation | Invocation, qualified proposition and exact premise membership | Shared conservative modality, approximation and status operations | Derived companions and later proofs |
| Analysis coverage | Invocation × capability × scope × context with identity-bearing lower membership | Complete, Partial, Unavailable, NotRequested and NoScope remain distinct | Scoped producer outcomes |
| Discharge evidence | Exact obligation source, derivation and coverage | Candidate, approximate, unavailable and insufficient negative evidence cannot close the question | Later immutable owners |

The inventory's 51 pair declarations are a finite native projection, not fabricated `ProviderRun` evidence (`analysis/native.rs:133`). The sixteen producing families and their predecessor sets are explicit (`analysis/mod.rs:16`). Early Dispatch excludes transfer and final normalization-coverage dependencies; the inspected closure control challenges that boundary.

## 3. Contracts, constraints and testing boundaries

R0 keeps the exact twelve-relation whitelist (`stages.rs:435`). `PublicationBoundary` retains named codes, while private `PrefixOrdinal` values belong to one checked schedule (`stages.rs:280,336,382`). Execution order does not follow enum-code order.

Vocabulary grows through insert-only private deltas. Close verifies sealed contents, refuses conflicting payloads and additions that change an old literal set, merges and validates the candidate prefix, and records results and prefix receipts transactionally (`generations/vocabulary.rs:130`). Literal-bound security-barrier views expose semantic columns (`generations/ddl.rs:558`). Computed receipts grant no stored reads; confirmed group acknowledgement creates completed handles.

Ordinary completion inherits the greatest prefix of its declared acknowledged inputs, including ordinary handoffs (`stages.rs:1610`). The store checks vocabulary references against that inherited bound and narrower explicit sources before issuing grants or receipts (`generations/stage_validation.rs:16`). Unrelated later closes do not widen authority. Failed, cancelled or uncertain transitions retain terminal poisoning rather than permitting in-place repair.

Captured source metadata comes from actual completed sources and typed permits (`analysis/sources.rs:35`). Publication compares exact persisted snapshot membership with effect-owner sources (`family/source_receipts.rs:52`). Metadata alone cannot authorize a read.

Expected coverage is independently reconstructed from captured input/artifact roles, scopes and lower outcomes. Stored requirements cannot shrink that frontier merely by dropping matching output rows (`analysis/expected.rs`; `family/coverage.rs:359`). Only **LocalTransfers/Transfers** and **Catalog/Catalog** currently have bound selectors. Other nonempty owner invocations explicitly refuse publication (`family/coverage.rs:344,363`).

Local semantic tests use model operations and bounded inputs. Actual publication controls use scheduled attempts through real PostgreSQL 18. The unscheduled testing-only Harness has no `StageCompletion`; it qualifies ordinary schema/algebra invariants, not source-sensitive publication. Scheduled conformance attempts take the same publication-check branch as product attempts (`generations/receipts.rs:71`).

## 4. Composition and execution

| Operation | Inputs, method and output | Limits and evidence linkage |
|---|---|---|
| Vocabulary close | Declared group, sealed deltas and prior prefix; transactional merge/validation | Finite whitelist; attempt budget; atomic acknowledgements and literal-prefix receipts |
| Native preparation | Completed facts, validated native supports and selected authored catalog | Exact native pairs; profile-aware inputs; facts-bound vocabulary |
| Coverage admission/publication | Captured input domain and actual lower coverage; shared conservative fold | Exact expected scopes and members; missing/coupled omissions refuse |
| Qualified derivation | Typed native/predecessor premises; shared BDD conjunction or alternative union | Weakest modality, joined approximation, conservative status and charged allocation |
| Cross-owner discharge | Earlier nominal obligation and later exact-subject proof | Same input/context, subject, channel, phase and qualification; canonical verdict/discharge policy |

The model governs these operations, rather than merely standardizing their output columns. Stored qualification validation reconstructs the result and refuses evidence strengthening (`family/support.rs`). Behavioral discharge rejects documentary and heuristic proof; nominal computation subjects preserve the earlier owner (`family/obligations.rs:90`; `obligation_support.rs:22`).

`FiniteF64` rejects nonfinite values and canonicalizes metric zero. Arrow admission and generated PostgreSQL checks preserve that contract. Diagnostic payloads remain separate from semantic invocation identity.

## 5. Change and failure scenarios

| Scenario and kind | Expected and observed locality | Evidence strength |
|---|---|---|
| Add an analysis within an existing owner — semantic extension | Add its owned method/capability selector and genuinely new operation; reuse nominal records, source capture, qualification and publication checks. Unbound selectors currently refuse activation | Interface-checked |
| Add a stored-read boundary — composition extension | Declare a finite nominal owner and predecessor set; generated declarations follow. Earlier owners need not acquire future result references | Interface-checked |
| Add an authored model using existing channels — instance/domain extension | Catalog parser and requirement traversal own meaning; native configuration binds selection, profile and budget; preparation stores its nominal declarations | Interface-checked; complete model execution excluded |
| Substitute ingestion/batching — mechanism substitution | Preserve sealed insert-only deltas, completion receipts, deterministic locks, prefix visibility and confirmed atomic acknowledgement within the store owner | Interface-checked; no replacement implemented |
| Omit a partial scope and all corresponding result rows — invalid publication | Expected frontier reconstructed from physical sources still requires the scope; coupled omission refuses | Supplied focused Tested evidence |
| Read an old source after a later close — lifecycle composition | Its view and receipt retain the original literal prefix | Supplied focused Tested evidence |

These scenarios support locality and composition without a general provider registry or workflow engine. They make no measured change-cost claim.

## 6. Correctness and fidelity gates

| Gate | Verdict | Scoped evidence |
|---|---|---|
| G1 Authority | pass | Model-owned semantics; generated nominal families/lowerings; native attribution preserved |
| G2 Semantic fidelity | pass | Explicit prefix, availability, modality, approximation, evidence and nominal-subject distinctions |
| G3 Validity | pass | Typed admission, stored semantic invariants and source-sensitive checks before scheduled publication acknowledgement |
| G4 Hidden behavior | pass | Pure analysis operations receive explicit inputs; selected catalog/profile/budget are captured; storage effects remain explicit |
| G5 Consistency and recovery | pass | Atomic group/result publication, immutable earlier prefixes and terminal failure/uncertain-commit handling |
| G6 Transformation and reuse | pass | Exact snapshot/member comparisons; shared qualification and discharge; canonical finite-metric lowering |
| G7 Truthful capability claims | pass | Unbound owners refuse activation; no complete producer, serving or performance claim follows from declarations |
| G8 Library leverage | pass | Existing PostgreSQL/SQLx, Arrow/DataFusion and BDD mechanisms retained; bespoke code supplies scoped semantics |
| CI-G1 Fidelity | pass | Native and derived evidence remain separate; uncertainty and heuristic restrictions survive composition |
| CI-G2 Evidence closure | pass for foundation references | Same-generation nominal references, captured inputs and generated derivation closure. Served-answer qualification excluded |
| CI-G3 Evaluation integrity | n.a. | No evaluation campaign or reference-derived compiler input introduced by this reviewed slice |

These are review verdicts, not execution outcomes or a full-gate receipt.

## 7. Findings and applicability

**No material in-scope finding.**

FP-01–FP-06 are **satisfied** for the selected scenarios. Their evidence supports separation of semantic meaning from persistence, explicit lifecycle/constraints, common composition operations and bounded local verification. Applicable DP authority, fidelity, validity, transformation, effects, publication and library rules are satisfied in this scope. CI-01/02/04/06/08/09 are satisfied at the reviewed evidence boundary.

Two observations define the acceptance envelope:

<a id="o1"></a>
**O1 — Downstream activation remains a package prerequisite.** Sixteen schema families do not establish sixteen operational analyses. Each downstream owner must bind its actual expected-domain selector, method/capability policy and completed inputs before producing nonempty invocations. Current refusal is correct. Owners: the corresponding Phase 4 producer package and `domain::analysis::expected`. Revisit at first activation and the scheduled B3/assembled reviews.

<a id="o2"></a>
**O2 — Early preparation qualification must use its actual boundary.** The original preparation test scheduled facts/configuration/native inventory but invoked final validation over the full model, whose normalized producers had not run. It reached native publication, parity and revoked-grant checks before failing at final validation. The inspected test-only correction uses facts plus configuration/native declarations and retains their ordinary and sealed validators. Its successful PostgreSQL receipt qualifies that early boundary. It does not qualify all-normalized or F0 assembly.

## 8. Library fit and total complexity

The pinned `sqlx-postgres` reference was consulted alongside `docs/pins.md`; no claim was transferred from its older table-provider fork to the current fork.

PostgreSQL transactions, locks, grants, views and generated constraints supply persistence and atomicity. Existing SQLx/Arrow/DataFusion boundaries supply transport and relational access. Existing BDD operations supply condition composition. Model-specific finite inventories, conservative coverage and exact-question comparison appropriately remain ordinary domain code.

Nominal families add mechanical schema surface, but avoid independently copied proof policy and mutable result tables. No additional generic engine or dependency is justified by the reviewed scenarios.

## 9. Alternatives and tradeoffs

| Alternative | Assessment |
|---|---|
| Reopen completed relations | Violates old receipts and prefix isolation |
| Separate translated vocabularies | Adds competing identities and interpretation at consumers |
| One final retained group | Removes required stored-read boundaries and extends retention |
| Generic appendable result families | Changes immutable membership and ownership semantics |
| Current finite vocabulary closes plus nominal immutable owners | Preserves shared meaning and actual dependency boundaries, at the cost of explicit groups and mechanically repeated schemas |

ADR-0105/0108 select the last alternative. Inspection supports that choice within this foundation. Revisit if a real consumer cannot express its inputs without a future-owner dependency or mutation of an earlier semantic row.

## 10. Verification and uncertainty

No command below was executed by this reviewer. Supplied receipts were inspected and remain attributed.

| Outcome | Command/evidence and boundary |
|---|---|
| passed, supplied 2026-09-30 | `python3 scripts/build_environment.py -- cargo test --release -p lctx-model -p lctx-postgres --features lctx-postgres/testing --test vocabulary_epochs`: 7 model and 12 PG controls at earlier integrated R0 baseline; plan §13.2 |
| passed, supplied 2026-09-30 | Wrapped core tests `--test vocabulary_read --test normalized_generation --test stage_checkpoint`: 2+4+1; earlier R0 baseline |
| passed, supplied 2026-09-30 | Wrapped model tests `--test analysis_discharge --test analysis_expected --test analysis_sources --test domain_analysis_owners`: 4+3+3+9; `/tmp/phase4-r1-final-model.log` |
| passed, supplied 2026-09-30 | Wrapped PG `analysis_publication`: six profile/case combinations; `/tmp/phase4-analysis-publication.log`; predates authored catalog registry integration |
| passed, supplied 2026-09-30 | Wrapped PG tests `domain_transfer`, `domain_composition`, `domain_stability`, `analysis_publication`, `publication_checks`; `/tmp/phase4-conformance-boundary.log`. Includes actual scheduled R1 publication and sixteen generic ordinary/group/profile callback cases |
| failed, retained receipt | Wrapped core `analysis_preparation` at full-model test scope; `/tmp/phase4-preparation-profile.log`. Final validation expected absent normalized producers |
| passed, inspected 2026-10-01 | `python3 scripts/build_environment.py -- cargo test --release -p lctx-model -p cpg-core --test analysis_preparation`: core PG test passed both profiles with corrected early schema; `/tmp/phase4-preparation-scoped.log` |
| failed, retained receipt | Same combined command: model foreign-selection fixture used unsupported format 1. Native-stage closure control passed |
| passed, composite receipt inspected 2026-10-01 | Same combined command after fixture format changed to 7; `/tmp/phase4-preparation-final.log` at main `f8266cc2394d1cf5d8d5a8ce98e01d5007503648`: core PG 1 test covering both profiles and model 2 tests passed. Test-only corrections were separately inspected on the coordinator's later tree |
| not_run | `just test-all`; full authorized functional scope and retirement are incomplete |
| not_run | Full-library pilots/comparisons, RSS/performance and embedding campaigns; explicitly excluded |
| not_run by reviewer | Formatting, hygiene and after-turn checks; automatic hook ownership retained |

The production review remains anchored to `0a609547`. Test-only follow-ups were separately inspected on the coordinator's later tree; subsequently integrated Dispatch functionality is excluded. Earlier tests are not silently promoted to a fresh run of the entire frozen or current tree.

## 11. Authority changes and dispositions

No new ADR or architecture pivot is required by this review.

The coordinator should publish this judgment and update current owner maturity/status through the existing Phase 4 plan, DESIGN and handoff routes. The parent cutover plan §8 remains the cross-phase disposition owner. Acceptance closes neither downstream implementation obligations nor complete Phase 4 qualification.

Future activation under O1 must preserve owner-specific selectors, actual completed inputs and independent missing/forged-frontier controls. O2's final test receipt retains the initial failures and composite provenance without changing the architectural judgment.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied | Semantic extensions belong to explicit model owners; persistence/batching substitutions remain within the store; local operations have bounded verification inputs |
| A2 Encode domain meaning explicitly | satisfied | Adequate distinctions govern capture, admission, qualification, publication and discharge; output structures are backed by authoritative operations |
| A3 Extend through composition | satisfied | Common contracts compose through nominal predecessor sets and acknowledged boundaries; unsupported activation refuses rather than inventing authority |

**Bounded decision: Accept scoped.** R0/R1 foundation contracts conform to the accepted architecture at the inspected production baseline. No material supported-scope MUST gap or failed architectural/fidelity gate was identified.

**Enclosing architecture: not accepted as complete.** Downstream selectors/producers, full assembly, retirement and integrated qualification remain open. Phase 5, PR6, pilots and performance/product-quality claims remain outside this decision.

The next implementation owners are the dependency-ordered Phase 4 packages, using this foundation and preserving its refusal boundaries. Their completion and the scheduled later reviews establish the enclosing acceptance.
