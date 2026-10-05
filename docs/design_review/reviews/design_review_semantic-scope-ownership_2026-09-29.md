# Scope ownership consolidation — bounded review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Tier · purpose | Change · conformance; compressed template slots 1, 6, 7, 8 and 12 |
| Reviewer · date | Independent Codex reviewer, 2026-09-29 |
| Subject | Uncommitted scope-ownership consolidation inspected over `fcfcaf2` (document-contract baseline); subsequent concurrent contracts excluded |
| Standard | [Core 3.0, template, code-intelligence profile 1.1 and repository binding](../design_principles/standard.toml); design-review and companion code-intelligence skill |
| Authority | [ADR-0085](../../adr/0085-typed-semantic-domain.md), [ADR-0086](../../adr/0086-immutable-postgresql-generations.md), [cutover plan P0-D](../../plans/semantic-model-cutover-plan_2026-09-29.md) |
| Decision | **Accept scoped**; no actionable regression or authorization gap identified in the inspected slice |
| Evidence | **Implemented / Interface-checked, 2026-09-29:** independent source inspection. Focused test passes are author-reported, not reviewer-executed. |
| Exclusions | Full expected coverage-matrix construction, production admission, coordinated resource budgeting, assembled phase qualification, and syntax-emitter changes |

Reviewed [`ownership.rs`](../../../crates/lctx-model/src/domain/ownership.rs),
[`attribution.rs`](../../../crates/lctx-model/src/domain/attribution.rs),
[`assertion.rs`](../../../crates/lctx-model/src/domain/assertion.rs), root membership in
[`mod.rs`](../../../crates/lctx-model/src/domain/mod.rs), and the
[`model`](../../../crates/lctx-model/tests/domain_coverage.rs) and
[`PostgreSQL` (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/tests/domain_coverage.rs) coverage tests.
Adjacent inspection covered acquisition boundaries and sealed-generation validation. Source line
references below describe the inspected snapshot; concurrent work can move them.

**Responsibilities and scenarios — Interface-checked, 2026-09-29.** `ScopeIndex` owns the shared
scope/input policy. Assertion validation retains subject/evidence interpretation and invocation
attribution; coverage validation adds scope ownership without replacing invocation-family checks.
Acquisition validation separately establishes valid corpus and distribution declarations.

| Scenario | Observed contract and enforcement |
|---|---|
| Matching provider/context claims an unrelated scope | `CoverageOwnership` resolves the run input and calls `owns_scope`; unrelated input ownership refuses (`attribution.rs:284–316`). |
| Corpus run claims member artifact/module/release scope | Explicit direct, directional `CorpusLibrary` membership permits the claim; no path/name inference or transitive membership is introduced (`ownership.rs:41–59`). |
| Corpus run claims a member's input scope | Input scope remains exact: the scope input must equal the invocation input (`ownership.rs:54`). |
| Assertion condition, subject or evidence crosses a boundary | Existing checks still require both scope containment and invocation ownership through `ScopeIndex`; document/lexical optional subjects retain their source checks (`assertion.rs:283–320`). |
| Forged ownership declarations reach persistence | Existing acquisition validation requires acquired corpus/library inputs (`input.rs:267–278`); sealed PG validation runs every model invariant before publishing (`generations/mod.rs:134–148`). |

Assertion ownership semantics are preserved by the consolidation. Coverage, assertion support
and acquisition validation remain complementary obligations. `NotRequested` has no invocation
under the existing local contract; this slice does not establish the expected schedule/matrix
needed to authorize omission or prove completeness.

### Focused receipts and limits

All receipts are **author-reported, 2026-09-29**. Commands identify the reported suites;
original invocation logs were not independently verified.

| Suite command | Reported outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain_coverage` | **passed**: one test with the 16-case ownership matrix, plus exact run-input controls |
| `cargo test --release -p lctx-model --test domain_assertions --test domain_lexical --test domain_documents --test domain_transfer` | **passed**: assertions 3, lexical 4, documents 3, transfer 6 |
| `cargo test --release -p lctx-postgres --test domain_coverage --test domain_documents` | **passed**: real PG18 coverage good/foreign cases and document good/foreign cases, one two-case test per suite |
| Reviewer tests, formatting, linting and integrated gates | **not_run** |

The model matrix exercises the ownership predicate directly, not complete valid acquired-input
datasets through every invariant. The PG coverage test exercises good/foreign input scopes;
it does not persist every artifact/module/release/corpus matrix combination. Shared-validator
agreement establishes execution through storage, not an independent policy oracle. These receipts
do not qualify full coverage admission or resource accounting.

## 6. Correctness and fidelity gates

Verdicts concern the inspected slice at source-inspection strength.

| Gates | Verdict | Evidence / limitation |
|---|---|---|
| G1 Authority | pass | One implementation of scope/input policy serves coverage and assertion consumers. |
| G2 Semantic fidelity; CI-G1 Fidelity | pass scoped | Coverage statuses and attributed assertions retain their distinctions; direct corpus membership does not become inferred or transitive authority. |
| G3 Validity | pass scoped | Stored ownership, invocation membership and acquisition checks compose before publication; assertion conditions/subjects/evidence retain their checks. |
| G4 Hidden behavior | pass | The shared index is deterministic, store-free domain validation; no new ambient input or execution effect. |
| G5 Consistency and recovery | pass for validator integration only | The new invariant participates in existing sealed validation and receipt checking. Lifecycle design and shared allocation admission are not requalified. |
| G6 Transformation and reuse | pass scoped | Moving assertion policy into the shared index preserves the inspected ownership behavior; no new caching protocol. |
| G7 Truthful capability claims | pass scoped | Predicate tests, PG cases, author receipts and full-admission exclusions remain explicit. |
| G8 Library leverage | pass | Standard maps/sets and the existing invariant interface suffice; no new generic framework or dependency. |
| CI-G2 Evidence closure; CI-G3 Evaluation integrity | n.a. | No serving or evaluation-reference path changed. |

## 7. Findings and applicability

**No actionable in-slice finding identified.** FP-01–06 and applicable DP-01/03/05/08/17/18/22/23
and CI-01/04 obligations are satisfied for the named ownership scenarios. The refactor consolidates
policy without claiming that ownership alone proves acquisition completeness or provider coverage.

Full expected-matrix construction, admission and resource work remain with the existing cutover
plan. Current scheduled finding disposition belongs to
[plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
The known Ruff `Skip`/total-walk termination limitation remains open and outside this review;
no syntax-emitter correction or qualification is implied.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29.** `ScopeIndex` is a small domain-policy owner using standard
collections. Consumers reuse it through the existing invariant protocol. No external API,
dependency, graph framework or parallel authorization mechanism is added. Resource cardinality
guards do not establish coordinated allocation budgeting; that remains explicitly deferred.

## 12. Architectural judgment and decision

| Judgment | Verdict | Bounded evidence |
|---|---|---|
| A1 Localize change | satisfied | Scope/input policy has a coherent owner with bounded dependencies and store-free checks. |
| A2 Encode meaning structurally | satisfied | Typed scope variants and explicit corpus/distribution relationships govern authorization; assertion and coverage enforcement call the same policy. |
| A3 Extend through composition | satisfied | Coverage gains ownership enforcement through the existing index/invariant mechanisms while assertion-specific interpretation remains with its owner. |

**Bounded decision: Accept scoped**, based on independent inspection and the separately attributed
focused receipts. No further in-slice implementation change is requested.

**Enclosing architecture: not assessed as complete.** No full P0-D, phase, production-admission or
resource-budget claim follows. Revisit assembled admission when the expected matrix and complete
producer roster are constructed, and reassess ownership if scope or corpus semantics change.
