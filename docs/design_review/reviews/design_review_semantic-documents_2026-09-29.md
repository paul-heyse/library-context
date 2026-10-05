# Representative document contracts — bounded review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Tier · purpose | Change · conformance; compressed template slots 1, 6, 7, 8 and 12 |
| Reviewer · date | Independent Codex reviewer, 2026-09-29 |
| Subject | Document-contract additions over `6ff6179aac118f345d623e16c7d6befd0e930d2e`; source snapshot inspected before the subsequent shared assertion/attribution scope-ownership consolidation |
| Standard | [Core 3.0, template, code-intelligence profile 1.1 and repository binding](../design_principles/standard.toml); design-review and companion code-intelligence skill |
| Authority | [ADR-0085](../../adr/0085-typed-semantic-domain.md), [ADR-0086](../../adr/0086-immutable-postgresql-generations.md), [DESIGN §15](../../design/sections/semantic-model.md), [cutover plan P0-B](../../plans/semantic-model-cutover-plan_2026-09-29.md) |
| Decision | **Accept scoped** for representative document contracts; no actionable in-slice defect identified |
| Evidence | **Implemented / Interface-checked, 2026-09-29:** independent source inspection. Focused test outcomes below are author-reported, not reviewer-executed. |
| Exclusions | Full P2 producer and field mapping, complete document parsing/recognition qualification, production admission, full-phase acceptance, and subsequent shared scope-ownership changes |

Reviewed [`documents.rs`](../../../crates/lctx-model/src/domain/documents.rs), document subject
support in [`assertion.rs`](../../../crates/lctx-model/src/domain/assertion.rs), root membership in
[`mod.rs`](../../../crates/lctx-model/src/domain/mod.rs), the
[`model tests`](../../../crates/lctx-model/tests/domain_documents.rs),
[`shared fixture`](../../../crates/lctx-model/tests/fixtures/documents.rs),
[`PG tests` (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/tests/domain_documents.rs) and
`fixtures/python/semantic_documents/guide.mdx`.
Adjacent evidence was the old document producer (`9efce30:crates/cpg-extract/src/docs.rs`, recover through Git)
and [`tables`](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/cpg-schema/src/tables.rs), plus existing source/evidence validation.
Line references below identify the inspected version; concurrent changes can move them.

**Responsibilities and fidelity — Interface-checked, 2026-09-29.** The document domain owns
passages, code blocks, links, mentions, components, attribute alternatives and their structural
invariants. Qualified observations and generated supports retain interpretation separately from
nominal source-backed nodes. Shared assertion validation resolves subjects to captured sources
and checks scope/input ownership. Existing derives and PostgreSQL lowering consume those contracts.
Synthetic fixture attribution establishes representation coverage, not actual producer fidelity.

| Change scenario | Owner, contract and observed consequence |
|---|---|
| Another provider contributes a different interpretation | Qualified assertions/supports preserve alternatives; document validation imposes no ordinal uniqueness across attribution. No normalization policy is introduced. |
| A component has several qualified parent interpretations | `DocumentCheck::finish` matches any parent alternative at the same qualification and preceding depth (`documents.rs:231–243`); it does not overwrite alternatives or require all depths to agree. |
| A parent component crosses passage headings | Ownership uses the child's start (`documents.rs:185–191`), while parent containment uses component spans and preceding ordinals (`216–220`). Parent and child may have different passages; the fixture exercises this. The old producer assigns passages by start (`docs.rs:599–602,702–723`). |
| An optional inner/lead span names another source | Present optional subjects participate in shared source checks (`assertion.rs:250–260`); document validation also requires containment. The foreign-span fixture exercises refusal. |

Typed document-node arms and source-span evidence reject wrong subtypes. Strictly decreasing
parent ordinals prevent component cycles; depth/parent consistency has local and relational checks.
Literal, expression, bare and spread attributes remain separate variants. Expressions and spreads
are stored source text. No synthetic Python occurrence or captured-code execution is introduced.
Passage length and code/content-hash checks do not establish parser parity or exact extraction
fidelity; those remain producer obligations.

**Author follow-up, 2026-09-29; not independently reinspected.** The initial fixture included the
current heading in `heading_path`, while the old producer collected enclosing headings before
pushing the current heading (`docs.rs:557–559`). The author reports changing the fixture to
enclosing-only and documenting that meaning on the declaration. This addresses the review's P2
mapping caveat; it was not a blocking finding. The author reports the model rerun after this
adjustment **passed**, three tests, on 2026-09-29; no reviewer execution is claimed.

### Focused receipts

Commands identify the reported suites; original invocation logs were not independently verified.
All receipt dates are **2026-09-29**.

| Suite / command | Outcome and attribution |
|---|---|
| `cargo test --release -p lctx-model --test domain_documents` | **passed — author-reported**, three tests, including the rerun after the heading-path correction. **not_run — reviewer**. |
| `cargo test --release -p lctx-postgres --test domain_documents` | **passed — author-reported**, one real PG18 test with good/foreign-span cases for the inspected snapshot. No post-adjustment result supplied. **not_run — reviewer**. |
| Formatting, linting, integrated gates and pilot | **not_run — reviewer**; outside this bounded review. |

The PG test checks sealed stored-content validation, refusal/publication and readback using the
shared model validators. It is persistence evidence, not an independent document-parser oracle.

## 6. Correctness and fidelity gates

These are source-inspection judgments for the supported slice, not executed test outcomes.

| Gates | Verdict | Evidence / limitation |
|---|---|---|
| G1 Authority; G2 Semantic fidelity | pass scoped | Domain declarations own meaning; generated forms follow. Typed alternatives and attributed disagreement survive. Full field mapping is excluded. |
| G3 Validity | pass scoped | Evidence bounds, subtype/source checks, optional-subject ownership, parent containment/order and qualified depth matching have enforcement paths. |
| G4 Hidden behavior | pass | Inspected definitions/validators are store-free; source text gains no execution authority. |
| G5 Consistency and recovery | n.a. to changed lifecycle | No lifecycle mechanism changed; adjacent PG conformance is covered only by the attributed focused receipt. |
| G6 Transformation and reuse | pass scoped | Document subjects retain subtype/source identity through shared validation; no new cache or reuse protocol. |
| G7 Truthful capability claims | pass scoped | Contract fixtures, author receipts, source inspection and deferred producer qualification are distinguished. |
| G8 Library leverage | pass | Existing derive/Arrow/PG mechanisms and standard collections suffice; no bespoke parser or generic framework added. |
| CI-G1 Fidelity | pass scoped | Attributed alternatives and typed attribute payloads remain distinct; no inferred production coverage. |
| CI-G2 Evidence closure; CI-G3 Evaluation integrity | n.a. | No served-answer or evaluation-reference path changed. |

## 7. Findings and applicability

**No actionable in-slice finding identified.** FP-01–06 and the relevant DP-01–08,
DP-18/22/23 and CI-01–03 obligations are satisfied for the named source, attribution, subtype
and topology scenarios. This is not a completeness assessment of the domain or its producers.

The heading-path observation above remains dated mapping evidence, with the author's correction
explicitly attributed. Full field disposition and producer qualification remain existing P2 work.
Any scheduled finding's current disposition belongs to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition),
not a competing register in this review.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29.** No new dependency or external API is introduced. Ordinary
typed declarations, bounded domain predicates, standard maps/sets and the existing generated
lowerings provide the required mechanisms. A separate graph framework or parser inside validation
would add responsibilities outside this representative contract slice. Producer-library fidelity
is revisited when P2 migrates the actual document producer.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | satisfied | Document meaning has one domain owner; source attribution remains shared; model checks run without database setup. |
| A2 Encode meaning structurally | satisfied scoped | Nominal source-backed nodes, typed subtypes/attribute alternatives and explicit parent/depth relationships retain their meaning. |
| A3 Extend through composition | satisfied scoped | Independent observations and qualified parent alternatives use existing assertion/support contracts without a consensus or ordinal-uniqueness rule. |

**Bounded decision: Accept scoped**, at independent source-inspection strength with the focused
author receipts above. No further document-slice implementation is requested. The subsequent
heading-path adjustment and its passing three-test model rerun are author-reported.

**Enclosing architecture: not assessed as complete.** No full P0/P1/P2 or production-admission
claim follows. Revisit extraction fidelity and complete field mapping during P2. Subsequent
shared assertion/attribution scope-ownership consolidation needs its own applicable review and
is not accepted by this artifact.
