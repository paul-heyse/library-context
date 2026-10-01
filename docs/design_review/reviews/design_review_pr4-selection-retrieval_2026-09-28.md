# PR4 selection and retrieval — assembled implementation review

Retired source citations below are historical paths within this review's recorded baseline
and dated inspection scope, including any working-tree limitations. They do not point to
replacement owners. Recover committed source through [Git history](../../README.md#historical-recovery);
the findings and their original evidence strength remain unchanged.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | PR4 dirty working tree on `main`, based on `cc6e581ab5e8c9eb8efad29fc437030ca1ad88ee`; inspected 2026-09-28. Source line references describe that inspection and can move during implementation. |
| Standard | [Core 3.0](../design_principles/core/design-principles.md), [template 3.0](../design_principles/core/design-review-template.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md), [repository binding](../design_principles/binding/library-context.md); `design-review` and `design-review-code-intelligence` skills. |
| Tier · purpose | **Design · target**, assembled PR4.8 review against [ADR-0077](../../adr/0077-contextual-selection-and-retrieval.md), [ADR-0078](../../adr/0078-current-design-cutover.md) and the functional target. |
| Reviewer | Independent `design_review_pr4` agent; production corrections were made by the implementation owner. |
| Evidence strength | **Implemented / source inspected, 2026-09-28**, with the specifically attributed focused **Tested** receipts in §10. The reviewer did not execute product tests. |
| Outcome | **Accept at Implemented and bounded Tested strength.** F01–F18 corrections are source-inspected. The composite code gate and later packet/bootstrap focused controls passed. Both fresh profiles compiled after F16; both profile serving probes, 18-relation contract parity, 88-table reconstruction and the current-only operator cutover/cleanup passed. Oversized behavioral operation packets explicitly refuse within the accepted contract. Live embedding qualification remains waived. |
| Supported scope examined | Schema contracts and pure classification; Core domain/render/artifact derivation; bundle validation and current reconstruction; PostgreSQL typed reads, unit ranks and final assembly; PyO3 prepared state; Python lexical/family ranking; existing MCP listing/decoding/response boundaries and evidence expansion. Existing briefs/native tools were examined as adjacent consumers. |
| Exclusions | Embedding-related PR4 qualification is **not_run by operator waiver** during a critical GPU benchmark; real-library pilots use `--embedder none`. Comparative product quality, PR5 browse/compare/new tools and general semantic completion remain outside PR4. Historical recovery is intentionally removed by ADR-0078. Exact retrieval is selected; removed ANN machinery is not a current functional obligation. |
| Current disposition owner | [Forward plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition). The findings below are the dated source assessment, not a second live status register. |

The accepted target remains [§14.7–§14.10](../../design/sections/api-and-evidence-product.md)
and the [PR4 execution sequence](../../plans/behavioral-model-forward-plan_2026-09-24.md).
This document now supplies PR4.8's assembled design/target assessment, retaining F01–F09 from the
bounded implementation review. It remains one dated evidence document; the forward plan owns
execution status. Its architectural decision is separate from the plan's full code, real-library
serving and operator-cutover acceptance. The embedding waiver does not establish embedding behavior.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and semantic boundary | Consumer and expected reason for change |
|---|---|---|
| `cpg-schema::wire::requirements`, `selection` | Finite public predicates, nominal identities, context/basis-aware claims, quantifiers, whole-conjunction applicability and result structure. Pure operators own these meanings. | A new predicate extends the contract and its typed catalog adapter. PostgreSQL and Python do not independently classify it. |
| `cpg-core::catalog_domains` and `cpg-schema::selection::catalog` | Core derives canonical domain closure from source facts and coverage. The schema adapter consumes those domains and attributed catalog facts; projection validation checks domain ownership. | A producer coverage change changes the canonical derivation. The same adapter can be tested without PostgreSQL. |
| `cpg-schema::retrieval::catalog` and `retrieval` | One pure `RetrievalInputs → Unit` renderer; separate fragments, subjects, anchors and vector receipts; shared canonical reconstruction and channel/family fusion policy. | A rendering change has one owner, used by production and projected-artifact validation. It need not alter canonical catalog facts. |
| `cpg-core::retrieval` | Loading, embedding effects, immutable artifact preparation, save and restore. | A new embedding specification can yield a separate artifact for the same canonical snapshot. |
| `lctx-postgres::selection`, `evidence`, `hydration` | Typed projection reads, pinned generations, leases, bounded CPU admission, membership validation and final packets. | Query or storage changes preserve the schema-owned meanings and public identities. |
| Python retrieval and `lctx_storage` | Lexical/vector orchestration and native prepared-selection calls. The native response validates channel ordering and fusion before serving it. | A channel availability change affects ranking and cursor identity, not predicate truth. |

Canonical domain reconstruction is enforced by
`crates/cpg-core/src/catalog.rs`, lines 1410–1443.
Projected domain validation is owned by
[`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs), lines 413–465.
The latter consumes already published canonical assertions; it is not a second source analyzer.

| Fact or derived result | Fidelity, identity and absence boundary | Consumer |
|---|---|---|
| Public member and signature domain | Derived from pinned catalog bindings and coverage. Member identity preserves public spellings; signature identity also carries its binding. Missing signature coverage cannot establish closed absence. | Typed selection and operation hydration. |
| Type observations and configuration links | Provider/source assertions retain source facts. Comparable type observations remain separate claims. Field relationships retain kind, link identity, signature/ordinal and available source nodes. | Type/configuration predicates and witnesses. |
| Scenario, deployment and source alignment | Attributed original records reached through a member or release association. A scenario check is only its stated check; it does not establish a demonstrated runtime combination. | Evidence predicates and original expansion. |
| Retrieval unit and channel rank | A unit has semantic source/subject identity; a fragment is a rendering piece. Scores and fusion are discovery heuristics and do not establish predicate truth. | Search winners and `get_evidence` unit expansion. |

Dependency direction is schema contracts → Core materialization and PostgreSQL effects → native
handles → Python orchestration → MCP transport. Core does not call Python ranking; the schema kernel
does not require a store. Python's BM25/NumPy policy is checked by the Rust final-assembly boundary,
not treated as a source of predicate truth. The permitted duplication is an independently
implemented policy check, with arithmetic controls; it is not an additional classifier.

## 3. Contracts, constraints and testing boundaries

| Contract | Preconditions and enforcement | Failure/lifecycle and isolated verification |
|---|---|---|
| `Selection` and predicate arguments | Rust Serde/Schemars owns tagged finite variants, defaults, bounded text, at most 16 terms and result schemas. Operand identity admission must additionally validate pinned membership: F11. | Unsupported variants fail before query work. Pure adapter controls exercise semantic inputs without PostgreSQL; actual MCP controls exercise generated schemas and native decoding. |
| Domain and comparable claims | Canonical reconstruction binds closure to source coverage. Typed contexts distinguish member/binding/signature/configuration/source/release; conflict reduction precedes quantification. | Incomplete absence remains unresolved. Joint proof intersects the whole conjunction and refuses unsupported runtime combinations. F01/F02/F03 and F12 cover the exposed distinctions. |
| Retrieval unit → fragment → vector | Pure renderer owns source, subjects, text and anchors; fragments own rendering identity; vector inputs bind the full embedding specification. Shared validation reconstructs exact expected units. | Token/embedding refusal preserves lexical fragments. F10 concerns complete persisted realization identity and selection after validation, not the pure renderer. |
| Prepared selection → rank assembly | Prepared state pins the generation/request and carries no DB lease. Both ranking channels consume its eligible members; exact promotion uses the same admission. Native assembly validates ranks, fusion and membership. | Work/byte excess is explicit refusal. Cursors bind generation, normalized selection, group, policy/profile and query channel state. No compatibility `where` adapter is required. |
| Winning unit → original evidence | Typed unit IDs and original references close within the pinned generation. Original content has bounded continuation. | Large unit metadata must not prevent every content page: F13. Unit expansion must use the actual winning source, not a replacement explanatory snippet. |

## 4. Composition and execution

| Stage/question | Projection, method and ownership | Effects/reuse boundary | Bounds and evidence |
|---|---|---|---|
| What is declared and applicable? | Canonical members, bindings, signatures, configuration links and attributed evidence; Core derives domains, schema classifies context/basis observations. | Facts are published once; the classifier consumes immutable typed inputs. | Separate corpus/analyzer coverage; indexed work and witness budgets; no null verdict. |
| What text represents each evidence unit? | Schema `RetrievalInputs` consumes canonical catalog tables; four named families preserve many-to-many subjects and original anchors. | Core materializes fragments/vectors after canonical publication; supplied artifacts pass the same renderer validation. | Deterministic full-key sorting, finite unit/text limits, explicit token/embedding states. F10 corrects persistence identity. |
| Which members meet the query? | PostgreSQL loads predicate-declared relations; one native preparation classifies every member once. | Lease ends before admitted CPU classification and before query embedding. | Maximum 16 terms, 200,000 rows per typed input, aggregate work/byte refusals. No top-k then lossy eligibility filter. |
| Which matching unit ranks best? | Python deduplicates exact family/text inputs before BM25; PostgreSQL computes exact vector best member/family unit/fragment; Python RRF60 composes channels and families. | Read-only generation, no ANN admission or query-time evidence execution. | One winner per channel/family/member, at most eight witnesses; stable identity ties and exact-symbol primary ordering. Independent arithmetic and SQLx receipts are in §10. |
| What does the agent receive? | Native final assembly checks channel ordering, family policy and tuple membership; MCP uses native request/output contracts. | Original unit expansion is a separate pinned read. | Group cursors, explicit ranked-discovery status, original contradicted count; response refusal never claims complete enumeration. F13 covers metadata admission. |

## 5. Change and failure scenarios


| Scenario | Inspected ownership route | Meaningful settling evidence still required |
|---|---|---|
| An analyzer lacks signature coverage but exports are complete | Core keeps signature closure separate from public-exposure closure; the pure classifier preserves unresolved absence. | Incomplete coverage and missing type observations must remain unresolved while intrinsic public-path mismatches remain contradicted. |
| A class name is rebound while an alias retains the old class | Configuration ownership resolves active bindings; it cannot borrow the shadowed declaration of the rebound name. | Retained alias supports its own owner and does not support the rebound class owner. |
| A generated dataclass constructor has no source formal node | `FieldTarget::Parameter { signature, ordinal }` identifies the existing semantic endpoint. | Exact-storage selection supports the generated endpoint and preserves its ordinal in the witness. |
| The only associated original is inside a scenario | `associated_sources` follows the scenario's original spans, shared by derivation and validation. | The scenario's alignment is discoverable; an unrelated artifact in the release cannot establish member alignment. |
| A supplied retrieval artifact changes text or assigns a real unit to a different existing member | Shared validation reconstructs expected units from canonical catalog inputs and compares full units. | Both forgeries are rejected even though each individual identity exists. Shuffling alternative declarations leaves rendering unchanged. |
| A small search page has many large candidate units | Winner validation reads membership tuples, not all candidate bodies; expansion is a separate operation. | The search succeeds within its rank budget; requested originals remain addressable through bounded continuation. |
| Python submits duplicate, sparse or incorrectly ordered channel ranks | Native assembly reconstructs channel winners and family fusion, then checks the supplied rows. | Invalid rank controls are rejected; real PostgreSQL vector winners agree with the independent float64 arithmetic control. |
| A retrieval build is retried after transient embedding refusal, or a renderer changes without changing canonical facts | Pure artifacts can differ while snapshot/spec stay fixed; the old snapshot/spec directory collides. | F10: complete realization identity, validated current selection and current-only reconstruction. |
| A caller reuses a valid-looking type/evidence/parameter ID from another generation | Nominal parsing succeeds, but the lookup must fail before a mismatch can become a selection verdict. | F11: one operand-admission pass using the finite typed inputs. |
| An ordinary class method has no decorator surface row | Stored declaration category is `function`; public member/invocation semantics must also consume class ownership. | F12: plain method versus module function, static/class methods and unknown decorators. |
| A scenario or configuration unit has large subject/anchor metadata | Content fragments can be small while the repeated header exceeds every response budget. | F13: bounded metadata with disclosed omission/continuation, preserving original addressability. |
| Unrelated evidence/types grow while the request uses no such facts | Unconditional canonical-domain hydration broke the default listing; broad signature-domain relation dependencies then broke a non-type parameter request. | F14: request-specific domain/relation dependencies preserve canonical evidence. Corrected default, parameter and union-type routes passed through the catalog-none real-library pilot. |
| Delta/DataFusion returns identical declarations in a different scan order | Unordered nested signature contexts changed serialized canonical domain bytes and identities, causing behavioral publication to fail source equality. | F16: canonicalize set-valued nested collections before hashing; reversed-input regression passed. Rebuild both final profile artifacts from the corrected producer. |
| Behavioral enrichment makes an operation packet exceed 256 KiB while its catalog is small | Explicit refusal is permitted, but the diagnostic must identify the enriched packet and the cap must also apply when no evidence association is present. | F17: final typed-packet check in both Operation branches; record refusal separately and verify every returned winning unit's original evidence. Optional enrichment pagination remains PR5 usability work. |
| A new installation follows the bootstrap after the obsolete PG7 upgrade path is removed | Fresh installation must establish the current four-role and split-configuration contract directly. | F18: shared bootstrap SQL provisions the current extension and roles; real-PG controls authenticate all four configs, check current schema and refuse an existing deployment. |

## 6. Correctness and fidelity gates

Gate verdicts concern the assembled PR4 implementation and the evidence actually inspected.
F10–F18 were corrected and reinspected. F14's named catalog-none runtime controls passed; F15
records the inspected governing-prose correction, and F16 has a red/green scan-order regression.
A source-inspection pass below is not a claim that every possible execution control has run.
The final named qualification receipts are in §10; the embedding waiver and broader-product
exclusions remain explicit. No additional review gate is introduced.

| Gate | Verdict | Evidence and remaining action |
|---|---|---|
| G1 Authority | pass — source inspected | Executable semantic owners remain coherent. F15's ADR transfer and current governing prose preserve the selected contracts and distinguish canonical snapshot identity from the complete retrieval realization required for exact replay. |
| G2 Semantic fidelity | pass — source and named focused controls | Contextual producer controls passed; F12 now derives methods/properties from attributed ownership/surfaces. The source catalog retains its original declaration kind. |
| G3 Validity | pass — source and named focused controls | F11 now admits referenced identities before classification. Catalog forged-unit and absent-operand controls passed; real-PG foreign-fragment/rank controls passed. |
| G4 Hidden behavior | pass — source inspected | Pure catalog classification/rendering have no store or network effects. PostgreSQL preparation releases the lease before CPU classification; embedding remains outside that lease. |
| G5 Consistency and recovery | pass — source and named focused/runtime controls | Complete artifact receipt identity, validated publication before CURRENT selection, pinned cursors and bounded CPU work have enforcement owners. F17 checks typed Operation size unconditionally and reports default/expanded refusal truthfully. Current 88-table reconstruction, both profiles' serving and operator cutover/cleanup passed. |
| G6 Transformation and reuse | pass — source and named focused/runtime controls | Shared renderer/fusion validation, complete artifact identity and independent Python/PG arithmetic controls support the accepted composition route. F16's canonical domain bytes/IDs are invariant under reversed input scans; fresh profile serving, contract parity and current reconstruction passed. F18's fresh bootstrap removes the obsolete upgrade dependency. |
| G7 Truthful capability claims | pass — stated evidence boundary | Findings distinguish Implemented from named Tested receipts and bounded real-library requests. Embedding qualification is waived/not_run. Existing PR3 observations are not promoted to `demonstrated_combination`. |
| G8 Library leverage | pass — source assessment | Domain semantics use existing Arrow/SQLx/native/BDD/BM25 boundaries. No demonstrated generic capability requires a new engine or library; §8 gives the alternatives and scope. |
| CI-G1 Fidelity | pass — source and named focused controls | F11/F12 corrections prevent the identified invalid-ID/category failures. Comparable claims, incomplete absence and scenario-check meaning retain explicit boundaries. |
| CI-G2 Evidence closure | pass — source and named controls | Unit/member/original round trips passed through actual PG. F13 bounds display metadata while retaining all original anchors for continuation; current real-PG and ordinary Rust controls are included in the composite code-gate receipt. |
| CI-G3 Evaluation integrity | n.a. | No product quality evaluation or parameter tuning was performed. The bounded fixture/oracle definitions inspected here do not establish comparative product quality. |

## 7. Findings and applicability

F01–F09 have **Implemented / source-inspected** corrections as of 2026-09-28; §10 establishes only
the named Tested portions. F10–F18 are the additional assembled findings. Findings retain these
source IDs when carried into forward-plan §6.2.

<a id="F01"></a>
### PR4/F01 — Closure and comparable claims must follow the predicate domain

**Principles:** FP-04/05, DP-02/03/08, CI-01/04; A2, G2/G3, CI-G1.

The earlier adapter could collapse type observations and rely on export coverage for signature
closure, while intrinsic member predicates inherited unrelated decorator admission limits.
That could hide disagreement, assert unsupported absence or make a known public-path mismatch
depend on the quantifier.

**Correction inspected:**
`crates/cpg-core/src/catalog_domains.rs`, lines 26–57, separately
requires Exports and Signatures coverage for signature closure and derives public-exposure closure
from its own domain. [`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs),
lines 230–239, emits declared type observations as separate claims.
[`selection.rs`](../../../crates/cpg-schema/src/selection.rs), lines 37–80, classifies comparable
context/basis claims before quantification, retains conflicts and requires nonvacuous closed support.

**Closure evidence:** Pure controls cover incomplete domains, same-context disagreement,
cross-overload disagreement, clean universal counterexample and empty domains. This review read
their definitions without a fresh separate kernel-test receipt. The wrapped intrinsic predicates
and missing-type controls in `tests/catalog/pr4.rs` (`f6562d3^:crates/cpg-core/tests/catalog/pr4.rs`, recover through Git)
passed in the current code-gate account (§10).
Owner: schema selection and Core domain derivation; ongoing disposition: forward-plan §6.2.

<a id="F02"></a>
### PR4/F02 — Configuration ownership and relationship endpoints must remain exact

**Principles:** FP-04/05, DP-02/05/07, CI-02/03; A2, G2/G6.

The earlier owner lookup admitted shadowed bindings, allowing a retained alias of an old class to
claim the newly rebound public name as its configuration owner. A node-only relationship target
also could not name a generated constructor parameter without a source formal node.

**Correction inspected:**
[`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs), lines 254–279,
filters active declaration ownership and distinguishes reader nodes from formal/parameter targets.
[`wire/requirements.rs`](../../../crates/cpg-schema/src/wire/requirements.rs), lines 44–49 and
175, supplies the tagged parameter endpoint and keeps ordinal in the field-link witness.

**Closure evidence:** The active-class plus injected-shadowed-binding control,
cross-field reader and generated exact-storage controls in
`tests/catalog/pr4.rs` (`f6562d3^:crates/cpg-core/tests/catalog/pr4.rs`, recover through Git). The corrected control uses
the actual `Options` declaration and an attributed shadowed alternative; it does not assume that
an alias fixture preserved the original class. Exact relationship kind and endpoint survive the
served packet. The corrected controls are covered by the [current code gate](../evidence/2026-09-28_pr4/raw/code-gate.json).
Owner: schema catalog adapter and wire contracts; ongoing disposition: forward-plan §6.2.

<a id="F03"></a>
### PR4/F03 — Source and release predicates must use their attributed originals

**Principles:** FP-04/05, DP-05/07/08, CI-01/02/11; A2, G2, CI-G1/CI-G2.

Source alignment previously omitted scenario/deployment originals or could be inferred from
unrelated release artifacts. Deployment and relationship predicates also require the producer's
declared field, interpretation and support semantics, rather than a text-shaped approximation.

**Correction inspected:**
[`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs), lines 395–410,
follows member associations through their exact span/scenario/deployment references. Core uses
this shared function at `crates/cpg-core/src/catalog_domains.rs`,
lines 8 and 53; projection validation checks the association at adapter lines 420 and 454.
Alignment reads that span's artifact at lines 300–305. Release/deployment and relationship branches
retain scoped original and typed support evidence at lines 288–328.

**Focused receipt:** Actual evidence-producer controls in
`selection_evidence.rs` (`f6562d3^:crates/cpg-core/tests/catalog/selection_evidence.rs`, recover through Git): package
metadata and version, declared scenario checks, resolved-target edge witness, scenario-only source
alignment, unrelated artifact and failed deployment interpretation ran under
`original_contexts_independent_roots_and_pure_rebuild` and **passed**; the
[current code gate](../evidence/2026-09-28_pr4/raw/code-gate.json), 2026-09-28, includes the catalog
and evidence controls. Owner: schema adapter, Core evidence/domain derivation;
ongoing disposition: forward-plan §6.2.

<a id="F04"></a>
### PR4/F04 — Search promotion and counts must preserve prepared selection

**Principles:** FP-04/06, DP-01/08, CI-04/09; A1/A2, G2/G6.

Exact-symbol promotion previously could include an unresolved member excluded by strict mode,
causing final assembly failure. Counting contradictions after retaining only ranked eligible
members erased the prepared selection's contradicted count.

**Correction inspected:**
[`lctx-postgres/src/selection.rs`](../../../crates/lctx-postgres/src/selection.rs), line 74,
uses the same mode/outcome admission for promotion. Lines 130–137 preserve the original
contradicted count when assembling the ranked page.

**Closure evidence:** Strict unresolved exact-symbol and contradicted-count controls
in [`tests/serving/pr4.rs`](../../../crates/lctx-postgres/tests/serving/pr4.rs), lines 13–17 and
88–92 at their initial definition, passed in [current real-PG continuation](../evidence/2026-09-28_pr4/raw/gate-continuation.log). Owner: native prepared
selection/final assembly; ongoing disposition: forward-plan §6.2.

<a id="F05"></a>
### PR4/F05 — Retrieval receipts must bind the canonical rendering, not only existing IDs

**Principles:** FP-01/04/06, DP-01/03/04/08/11, CI-11/13; A1/A2/A3, G1/G3/G6.

Identity existence and a recomputed receipt alone could accept altered unit text or a unit
reassigned to another existing member. Separately implemented rendering checks would create a
second interpretation of the catalog. Sorting alternative fields only by field identity also made
their rendering depend on input order.

**Correction inspected:** One pure renderer now lives in
[`retrieval/catalog.rs`](../../../crates/cpg-schema/src/retrieval/catalog.rs), lines 7–76. It sorts
fields by the full `(field_id, source_fact_id)` key at line 19. Shared validation reconstructs and
compares full expected units in [`retrieval.rs`](../../../crates/cpg-schema/src/retrieval.rs),
lines 167–168, in addition to subject, fragment, vector and receipt closure. Core retains effectful
load/prepare/save/restore responsibilities.

**Closure evidence:** Changed-text, wrong-existing-member, reversed-input, two-spec/same-snapshot
and artifact restore/corruption controls in
`tests/catalog/pr4.rs` (`f6562d3^:crates/cpg-core/tests/catalog/pr4.rs`, recover through Git) passed in the
[current code gate](../evidence/2026-09-28_pr4/raw/code-gate.json). The
embedders there are synthetic controls, not live embedding or product-quality evidence. F10 names
the persistence controls included in the same code-gate account.
Owner: schema retrieval renderer/validator and Core artifact effects;
ongoing disposition: forward-plan §6.2.

<a id="F06"></a>
### PR4/F06 — Returned member and unit identities must reach the original records

**Principles:** FP-02/05, DP-02/04/08, CI-11; A2/A3, G2/G6, CI-G2.

The wire returns a bare public-member identity, but hydration previously interpreted that shape
only as an operation identity. API/options units also had declaration-fact anchors without an
expandable original anchor, so a winning unit could return rendered text without its source.

**Correction inspected:**
[`hydration.rs`](../../../crates/lctx-postgres/src/hydration.rs), lines 245–249, resolves the
returned member identity. [`retrieval/catalog.rs`](../../../crates/cpg-schema/src/retrieval/catalog.rs),
lines 37 and 44, adds catalog original anchors; their expansion is implemented in
[`evidence.rs`](../../../crates/lctx-postgres/src/evidence.rs), lines 418–457, using the pinned
generation and typed continuation.

**Closure evidence:** Returned-member-to-record and winner-unit-to-original round trips, including
API/options original continuation, passed in [current real-PG continuation](../evidence/2026-09-28_pr4/raw/gate-continuation.log) through the real serving
control in [`tests/serving/pr4.rs`](../../../crates/lctx-postgres/tests/serving/pr4.rs). F13's later
header correction has its separate source/verification boundary.
Owner: retrieval renderer and PostgreSQL hydration/evidence;
ongoing disposition: forward-plan §6.2.

<a id="F07"></a>
### PR4/F07 — Rank validation must reconstruct channel ranks before checking fusion

**Principles:** FP-04/05, DP-03/08/11, CI-09; A2, G3/G6.

Finite positive ranks and recomputed fusion did not reject duplicate, sparse or incorrectly ordered
channel ranks. Such ranks could change the advertised family policy while producing a
self-consistent supplied final score.

**Correction inspected:**
[`lctx-postgres/src/selection.rs`](../../../crates/lctx-postgres/src/selection.rs), lines 107–117,
reconstructs channel winners, compares rank/unit/fragment identities, then checks family fusion and
order. [`cpg-schema/src/retrieval.rs`](../../../crates/cpg-schema/src/retrieval.rs), lines 73–92,
owns deterministic channel ranking, RRF60 within families and equal family-rank fusion; exact
promotion is a primary sort key.

**Focused receipts:** Python's independent family arithmetic and
duplicate-before-BM25 controls **passed** among the seven tests in
[current Python continuation](../evidence/2026-09-28_pr4/raw/gate-continuation.log), 2026-09-28. The actual PostgreSQL vector score/winner/rank oracle
and malformed-rank controls in [`tests/serving/pr4.rs`](../../../crates/lctx-postgres/tests/serving/pr4.rs)
passed in [current real-PG continuation](../evidence/2026-09-28_pr4/raw/gate-continuation.log). Its earlier fixture lacked unit vectors; the corrected fixture
uses synthetic vectors and does not qualify a live embedder. Owner: schema rank policy, native assembly and query;
ongoing disposition: forward-plan §6.2.

<a id="F08"></a>
### PR4/F08 — Membership validation must not hydrate every winning body

**Principles:** FP-01/06, DP-10/18/20, CI-08; A1/A3, G5.

Final assembly previously loaded unit and fragment text for all submitted winners just to validate
membership, before applying the requested page. Aggregate body size could therefore refuse a
small valid page and duplicate readback work.

**Correction inspected:**
[`winner_membership.sql`](../../../crates/lctx-postgres/queries/winner_membership.sql) validates
generation/member/family/unit/fragment tuples without bodies. Native assembly calls it at
[`selection.rs`](../../../crates/lctx-postgres/src/selection.rs), lines 120–128. Original expansion
retains its separate bounded owner.

**Closure evidence:** Inspection of the tuple query establishes that winner validation reads no
bodies. The actual PG foreign-fragment rejection and independent expansion controls passed in
[current real-PG continuation](../evidence/2026-09-28_pr4/raw/gate-continuation.log); no performance measurement is claimed.
Owner: PostgreSQL selection query; ongoing disposition: forward-plan §6.2.

<a id="F09"></a>
### PR4/F09 — Classification and assembly need explicit CPU admission and bounded traversal

**Principles:** FP-01/06, DP-10/18/20, CI-08; A1/A3, G4/G5.

CPU-heavy classification/fusion previously executed on service I/O workers, with some native
preparation under the Python interpreter lock. Repeated unindexed catalog scans and late witness
budget checks could consume substantial work before refusal.

**Correction inspected:**
[`serving.rs`](../../../crates/lctx-postgres/src/serving.rs), lines 284–288, owns two-slot CPU
admission retained until the blocking job ends. Preparation/final assembly use that owner at
[`selection.rs`](../../../crates/lctx-postgres/src/selection.rs), lines 71–73, 105 and 130.
[`lctx_storage/src/lib.rs`](../../../python/lctx_storage/src/lib.rs), lines 381–398, detaches
prepared scope/page work and admits ranked parsing. The schema catalog adapter constructs indexes,
preflights input size and charges indexed visits before evaluation at
[`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs), lines 67–159.
Fusion groups witnesses by member at [`retrieval.rs`](../../../crates/cpg-schema/src/retrieval.rs),
lines 83–92.

**Closure evidence:** The oversized input control passed in the current code-gate account (§10). Actual PG CPU cancellation,
continued I/O responsiveness while both CPU slots were held and eventual capacity release passed
in [current real-PG continuation](../evidence/2026-09-28_pr4/raw/gate-continuation.log). Traversal charging was source inspected; no timing or scalability
measurement is claimed. Owner: native service lifetime and schema adapter;
ongoing disposition: forward-plan §6.2.

<a id="F10"></a>
### PR4/F10 — Persisted retrieval realization identity omitted result-changing inputs

**Principles:** FP-02/03/04/05, DP-04/09/19, CI-13; A1/A2/A3, G5/G6.

The former `retrieval/<snapshot>/<spec>` immutable leaf collided when a renderer changed or a
transient token/embedding failure cleared while the canonical snapshot and specification stayed
fixed. An ordinary valid retry could fail `immutable_write`. Bundle publication also omitted
receipt/snapshot equality until later verify/import, allowing it to return an invalid generation.

**Correction inspected:** [`retrieval.rs`](../../../crates/cpg-core/src/retrieval.rs), lines
98–136, keys the leaf by a hash of the complete file-digest receipt and atomically selects CURRENT
only after all files are written. Restore verifies that selected receipt, file hashes and schemas.
[`bundle.rs`](../../../crates/cpg-core/src/bundle.rs), line 986, binds the receipt snapshot;
`bundle_with_embedding`, lines 1103–1108, validates/builds the generation before save/selection.
The correction preserves immutable artifacts and the current-only runtime policy without a new
general rebuild framework.

**Source disposition:** Implemented / reinspected 2026-09-28. Two-spec and replay controls,
same-spec token-unavailable/rematerialization and foreign-snapshot controls
in `tests/catalog/pr4.rs` (`f6562d3^:crates/cpg-core/tests/catalog/pr4.rs`, recover through Git) are included in the
current composite Rust account: 473 passed in the complete run plus the sole guard repair passed.
Their embedding providers are deterministic fixtures.
Owner: Core artifact persistence and bundle publication; current disposition:
forward-plan §6.2.

<a id="F11"></a>
### PR4/F11 — Predicate operands lacked pinned membership admission

**Principles:** FP-02/05/06, DP-02/03/08, CI-04/11; A2, G3, CI-G1.

A well-formed `CanonicalTerm` or `DeclaredUnionMember` ID absent from the pinned catalog was
compared as a mismatch and could yield Contradicted over a complete signature. Typed parameter
slots and relationship member/evidence/declaration operands also lacked membership admission.
Nominal syntax alone did not establish the precondition assumed by their comparisons.

**Correction inspected:** [`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs),
lines 123 and 144–162, performs one operand-admission pass before member classification. It checks
term IDs, declared node inputs, member/evidence identity and exact `(signature, ordinal)` parameter
slots. [`Requirement::dependencies`](../../../crates/cpg-schema/src/wire/requirements.rs) now
includes the typed parameter and evidence relations needed by those checks. Pure and PG callers
therefore share the same admission meaning.

**Source disposition:** Implemented / reinspected 2026-09-28; absent term, parameter and member
controls passed in the current code-gate account (§10). Owner: schema prepared catalog and hydration dependencies;
current disposition: forward-plan §6.2.

<a id="F12"></a>
### PR4/F12 — Public method predicates reused the stored function category

**Principles:** FP-04/05, DP-02/08/24, CI-02; A2, G2/G7, CI-G1.

The implementation owner identified, and this review confirmed, that `MemberKind::Method`
compared against stored `function` and ordinary `InvocationForm::Method` fell back to that same
category when there was no decorator surface. An ordinary class method could therefore fail both
accepted predicates.

**Correction inspected:** [`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs),
lines 164–169 and 226–235, derives the public category from canonical class ownership and active
property surfaces and uses it for undecorated invocation fallback. The catalog's source
declaration category is preserved. This is a single predicate-owner correction, not a second
source extraction rule.

**Source disposition:** Implemented / reinspected 2026-09-28; plain-method versus module-function
controls passed in the current code-gate account (§10). Owner: schema catalog adapter; current disposition: forward-plan §6.2.

<a id="F13"></a>
### PR4/F13 — Large unit metadata could prevent original-content expansion

**Principles:** FP-02/05, DP-03/20, CI-08/11; A2/A3, G5, CI-G2.

`get_retrieval_unit` formerly repeated every subject and anchor on every page, then applied the
32/256 KiB response limit. An admitted many-member scenario/document or configuration unit could
have a header larger than every response budget, making all requests refuse even when the next
content fragment was small.

**Correction inspected:** [`RetrievalUnitHeader`](../../../crates/cpg-schema/src/wire/evidence.rs)
now bounds title to 500 Unicode scalars and displayed subjects/anchors to 16 each, with explicit
`metadata_omitted`. [`get_retrieval_unit`](../../../crates/lctx-postgres/src/evidence.rs), lines
429–455, uses that display header while walking the complete unit's anchors and original-content
cursors. Metadata omission does not truncate source bytes or replace the winning original.

**Source disposition:** Implemented / reinspected 2026-09-28. Current ordinary Rust and real-PG
unit/original continuation controls are recorded in the composite code gate. Owner: schema evidence
packet and PostgreSQL evidence assembly; current disposition: forward-plan §6.2.

<a id="F14"></a>
### PR4/F14 — Selection hydrated unrelated canonical domains and typed relations

**Principles:** FP-01/06, DP-10/18/20, CI-08; A1/A3. Resource refusal is explicit; the defect is
the functional capacity and dependency boundary, not silent truncation.

The catalog-none real-library smoke calls `find_operations` with `selection: {}` and `limit: 3`.
It initially failed with `ResourceRefused: hydration byte budget` before classifying or returning
members. The corrected interim route passed with three catalog packets; both final profile pilots
were rebuilt after F16 and their serving receipts passed. The
[current evidence owner](../evidence/2026-09-28_pr4/README.md) carries final pilot receipts.
The implementation owner's measurement is 5,097 members and approximately 85 MB of canonical domain
detail, approximately 73 MB belonging to source-artifact contexts. The unconditional
`CatalogSelectionDomains` load in PostgreSQL preparation and the schema constructor's all-seven
domain requirement couple an empty request to the largest unrelated evidence family. Raising the
64 MiB budget would preserve that coupling.

**Correction inspected — Implemented:** the distinct `DomainInput` in
[`selection/catalog.rs`](../../../crates/cpg-schema/src/selection/catalog.rs), lines 42–69,
represents a request projection of an already validated canonical row. Its original `domain_id`
remains the full canonical domain's witness reference. Narrowed JSON is not represented as a
`CatalogSelectionDomainsRow` and is not checked against that full record's hash. Canonical
conversion/publication both use `canonical_domain` to validate the full digest, full seven-domain
inventory and context ownership. One `PreparedCatalog` and one classifier consume canonical
conversions and projected inputs; no second semantic evaluator was introduced.

[`selection_domains.sql`](../../../crates/lctx-postgres/queries/selection_domains.sql) projects
module, class owner, release metadata and only declared input-domain kinds before transferring and
parsing their JSON. It retains complete selected JSON domain records, including completeness flags
and original evidence, and preserves their ordinal ordering. PostgreSQL preparation streams with
200,000-row/64 MiB admission; the existing pure aggregate budget remains. Every member still has its
own projected domain record. `PreparedCatalog::new` validates projected context shape and member
closure; `classify` refuses missing requested domains before iterating candidates. An omitted
unrequested domain means **not loaded**, not empty, incomplete or absent in the canonical corpus.

[`Requirement::input_domain`](../../../crates/cpg-schema/src/wire/requirements.rs), lines 353–363,
owns the finite input dependency separately from the result domain. `FacetMembership` and
`DeploymentDeclaration` with `Launch` or `Configuration` return before consulting a `ScopedDomain`
and therefore require no canonical domain contexts. Both preparation and classifier admission use
this declaration. SQL selects these named kinds without predicate evaluation or context truncation.
A true `SourceAlignment` request still needs source contexts: the measured source family alone
exceeds the existing limit, so domain-only projection establishes no general source-alignment
capacity claim. Its bounded refusal remains explicit. Any later compacting projection must preserve
the exact facts, closure and original witness identities consumed by that predicate.

The implementation owner then reported the same hydration refusal for the real-library
`declares_parameter(name="transport")` probe. Signature-domain dependencies loaded all type
observations, terms and arguments even though this predicate consumes none. The additional
correction in `Requirement::dependencies`, lines 365–374, narrows `DeclaresParameter`,
`ParameterKind`, `ParameterRequired`, `ParameterDefaultState` and `ParameterDefault` to bindings,
signatures and parameters. Their evaluator reads only the canonical signature context, binding
ownership, signature role and parameter fields/source IDs; type observations and structural type
matching are confined to `ParameterType`, whose dependencies remain complete. No fidelity defect
was identified in that reduced dependency set. The relation projection shares the same schema
declaration used by production hydration and the focused equivalence control.

The remaining generic hydration ledger also accumulated JSON-map/column overhead across relations
after selection had consumed those maps into typed rows. [`selection::typed` and `charge_input`](../../../crates/lctx-postgres/src/selection.rs),
lines 160–199, now distinguish those lifetimes: each relation retains the existing transient
hydration bound, while all retained typed inputs share a 64 MiB estimate charged before each typed
record is accumulated. The estimate is three times encoded bytes plus the fixed typed-row size and
128 bytes of overhead. Saturating arithmetic refuses overflow. Projected domains and artifact
alignment participate in the shared charge; both checked queries stream and check before pushing
rows. This corrects the one remaining `fetch_all` alignment allocation found during review.
Transient and retained ledgers can coexist: neither the estimate nor their numeric thresholds is a
claim of a hard 64 MiB total request/process-memory cap. No classifier, context, completeness or
witness meaning changes with this accounting correction.

**Settling controls:** empty selection over a catalog containing large unrequested domain detail;
canonical-versus-projected classification equality for each consumed domain; missing-required-domain
refusal; unchanged canonical `Domain` witness IDs; facet and launch/configuration requests without
unneeded domain contexts; and the existing real-library default/parameter probes. Canonical
digest/all-seven negative controls remain at the canonical publication/conversion boundary. No
embedding run or increased budget is required by this correction.

**Source disposition:** **Implemented / reinspected**, 2026-09-28. Projected-domain,
missing-required-domain and all five narrowed parameter-variant controls are included in the
[current code gate](../evidence/2026-09-28_pr4/raw/code-gate.json). The previously inspected catalog
pilot and `retrieval_probe.py::check` establish
real discovery/strict `declares_parameter(name="transport")`, positive union-category
`parameter_type`, lexical ranking and original-byte equality across all four retrieval families.
These named routes passed in both current profiles, followed by current reconstruction and operator
cutover. Broad SourceAlignment and aggregate-capacity limits remain explicit.
The focused equality fixture currently supplies real parameter records but no facet/deployment
records: its facet/launch/configuration comparisons establish empty-input equivalence, not positive
producer support. Owners: schema input contract and dependency declaration; PostgreSQL request
projection. Current disposition: forward-plan §6.2.

<a id="F15"></a>
### PR4/F15 — ADR retirement must replace obsolete governing claims, not only their links

**Principles:** FP-04/06, DP-01/22; A1/A2, G1/G7.

The in-progress ADR-0068 retirement changed governing references to ADR-0078 while some owning
paragraphs still described the removed implementation: PostgreSQL-independent file serving,
implemented ANN admission/mixed routing, old bundle/projection formats, and generated typed
contracts as merely Proposed. Such paragraphs would make the current design contradict its
replacement decision and executable owners.

**Clause-transfer assessment:** comparing the retired ADR-0068 from Git with ADR-0078, ADR-0077
and their current governing sections found no material surviving-clause omission. ADR-0078
preserves effect/lifespan ownership, immutable readiness, canonical/cache separation, codecs,
credential and transport boundaries, embedding identity/conformance, and brief retrieval policy.
ADR-0077 owns the intentionally changed contextual selection, addressable retrieval and fusion.
Current §6.4/§6.5/§11.3 retain full support validation, content-identity exclusions, explicit
configuration and expected validation-error mapping. The retirement does not accept ADR-0025 or
qualify embeddings, ANN or historical recovery.

**Corrections inspected:** DESIGN §B13/§B14 now states current typed contracts, PostgreSQL-required
online serving and separate retrieval receipts. §6.5 uses bundle16/projection6 and removes claims
of active ANN admission/routing. Repeated/retired Decision references were removed. DESIGN §B12 and
§6.4 Derivation now name the canonical snapshot plus the selected immutable retrieval realization
and complete receipt. Exact replay requires the same complete inputs; changed rendering,
specification or token admission can produce another identified realization over that snapshot.
ADR-0077 declares the changed replay/retrieval section ownership; ADR-0078 preserves the current
effect and lifecycle boundaries. The API owner §14.13 now deletes prior epochs after validation,
points current receipts to the forward plan/runbook and labels PR1–PR4 Implemented with PR5 Proposed.
The retained W15 wording also follows the current ADR-0078 replacement policy.

**Source disposition:** **Implemented / reinspected**, 2026-09-28. No material surviving-clause
omission or unresolved semantic conflict remains identified. The final
[documentation receipt](../evidence/2026-09-28_pr4/raw/docs-check.log) passed with 168 canonical pages
and zero errors; the reviewer read the receipt without running the check. Owners:
ADR-0078/0077 and DESIGN §B12/§B13/§B14, storage
§6.4/§6.5 and serving §11.2. Current disposition: forward-plan §6.2.

<a id="F16"></a>
### PR4/F16 — Canonical domain identity depended on unordered scan arrival

**Principles:** FP-02/04/06, DP-08/11/19; A2/A3, G5/G6, CI-G1.

The earlier behavioral FastMCP pilot failed `catalog-source-equality:catalog_selection_domains`
before publication, as reported by the implementation owner. That pilot log was overwritten by the
successful current compile and is not cited as failure evidence. The implementation owner's
comparison of the rejected attempt found 60 differing members whose signature-context arrays had
different ordering while their values, evidence and completeness agreed. The domain hash bound
serialized JSON, but signature contexts followed relation scan arrival rather than canonical
context identity. Publication correctly refused the inconsistent derivation; no validator was
weakened to admit it.

**Correction inspected — Implemented:**
`crates/cpg-core/src/catalog_domains.rs` sorts each domain's contexts
by the complete typed `SelectionContext` before serialization and hashing. Signature identity still
contains member, binding and signature: sorting does not merge overloads or binding alternatives.
Release rows use release identity; set-valued fact evidence is sorted/deduplicated by fact identity;
module selection/fallback is deterministic. Existing source contexts already union original
references through ordered sets, and repeated scenario contexts carry the same original scenario
reference. Completeness flags and distinct evidence identities remain intact.

`domain_scan_order` (`f6562d3^:crates/cpg-core/tests/catalog/pr4.rs`, recover through Git) loads the real published fixture
facts/evidence, derives contracts, then reverses names, sources, releases, declarations, candidates,
coverage, members, bindings, signatures, constructors, configurations, field links, surfaces and
associations. It compares both complete serialized detail and domain IDs by member. The catalog
target failed before the fix ([focused red receipt](../evidence/2026-09-28_pr4/raw/domain-order-red.log)) and passed afterward
([focused green receipt](../evidence/2026-09-28_pr4/raw/domain-order-green.log), one test). This is a meaningful order-invariance control over
the production producer; it does not substitute for the real behavioral profile rerun.

**Qualification boundary:** canonicalized bytes can change domain IDs in either profile even when
set meaning is unchanged. The earlier catalog-none receipt remains evidence for its tested serving
behavior, not post-fix canonical identity or replay. The implementation owner reports both interim profile stores replaced and fresh compiles published
from the same current binary, with pilot reuse bound to complete CLI/task/profile/embedder inputs. Final artifacts and reconstruction must come from that
corrected producer; historical recovery is not required.

**Source disposition:** **Implemented and focused Tested**, 2026-09-28. The reviewer read the
red/green receipts and inspected the correction without running tests. The composite code gate
passed. Both fresh profile publications are owner-reported passed after this correction; the
reviewer inspected the final [serving](../evidence/2026-09-28_pr4/raw/pilots.json),
[contract parity](../evidence/2026-09-28_pr4/raw/profile-parity.json) and
[88-table reconstruction](../evidence/2026-09-28_pr4/raw/pilot-reconstruction.json) receipts, all passed.
The final [operator cutover](../evidence/2026-09-28_pr4/raw/operator-cutover.json) also passed with
exactly the two current generations and the behavioral profile selected. Owner: Core canonical domain
derivation; current disposition: forward-plan §6.2.

<a id="F17"></a>
### PR4/F17 — Operation packet limits and refusal diagnostics must cover every hydrated packet

**Principles:** FP-02/05/06, DP-03/20/22, CI-08; A2/A3, G5/G7.

The fresh behavioral pilot exposed expanded `get_operation` refusals where optional behavioral
enrichment made the complete packet larger than 256 KiB although the mandatory catalog was much
smaller. The old diagnostic called the whole value the mandatory API contract and recommended
expanded mode even when it was already selected. The 32/256 KiB test ran only while adding evidence
associations, so a packet with no such association could instead fall through to the unrelated
8 MiB generic response limit.

**Correction inspected — Implemented:** `evidence::check_operation_packet` checks serialized size
plus the explicit enclosing-wire reserve after typed assembly in both resolved and unresolved
`Operation` branches (`hydration.rs`, lines 339–340 and 661–662 at inspection). The check also owns
the early association-fit refusal. Diagnostics state default/expanded mode, operation/catalog bytes
and the limit. Default mode recommends `expanded=true`; expanded mode points to original expansion
through a returned retrieval unit. Neither required signatures nor behavioral fates are cleared.

The implementation-owner measurement, also present in the inspected packet probe output, was:

| Behavioral member | Enriched operation bytes | Catalog bytes | Expanded cap |
|---|---:|---:|---:|
| `fastmcp.cli.cli.run` | 567,783 | 30,677 | 262,144 |
| `fastmcp.server.auth.JWTVerifier` | 351,727 | 19,367 | 262,144 |

The same catalog-profile calls succeeded. This is an explicit bounded usability limitation, not a
missing canonical signature or corrupt winning unit. §14.9 expressly allows omitted-section/
continuation **or explicit resource refusal**; PR4 does not require every top-five whole-operation
packet to fit. `retrieval_probe.py` catches only the exact expanded-packet `ResourceRefused` prefix,
records those members/reasons separately from hydrated packets, and still requires original-byte
equality and terminal continuation for **every** winning unit of every refused member. Other
errors fail the probe. A refused operation is never counted as successfully hydrated.

**Focused evidence:** `operation_packets_without_associations_obey_both_wire_limits` passed one
test at the default/expanded boundaries and one byte over; 16 affected Python operation/catalog
tests passed ([packet boundary](../evidence/2026-09-28_pr4/raw/packet-bound.log),
[Python](../evidence/2026-09-28_pr4/raw/packet-python.log), inspected 2026-09-28).
The complete code-gate receipt predates this serving-only change; these are its bounded follow-up
controls. The updated [composite receipt](../evidence/2026-09-28_pr4/raw/code-gate.json) records the
subsequent packet Clippy pass. The final [runtime receipt](../evidence/2026-09-28_pr4/raw/pilots.json)
passed for both profiles: catalog hydrated all 30 probed packets; behavioral hydrated 28 and recorded
the two explicit refusals above, with required original-unit expansion still passing.

**Source disposition:** **Implemented and focused Tested**, 2026-09-28. No new broad gate or
canonical rebuild is required by this serving-only correction. Optional behavioral pagination or
explicit omission is deferred to PR5's bounded-packet usability work, triggered by the measured
refusals above; it must introduce an honest typed continuation/omission contract rather than relabel
selected analysis as `not_requested`. Owners: PostgreSQL typed packet/evidence assembly; PR5 packet
contract for the deferred improvement. Current disposition: forward-plan §6.2.

<a id="F18"></a>
### PR4/F18 — Fresh installation depended on a retired upgrade epoch

**Principles:** FP-01/03/05/06, DP-03/20; A1/A3, G4/G5.

The previous bootstrap created only the application and migration roles and combined their URLs in
one config. The current importer/serving roles, split configs and pgvector setup were provided by
`postgres_expand.py`, whose CLI required the obsolete PG7 backup/upgrade state. Deleting that
historical path without moving the surviving installation contract would strand a fresh deployment.

**Correction inspected — Implemented:**
[`postgres_bootstrap.py`](../../../scripts/postgres_bootstrap.py), lines 24–182, now owns the shared
current provisioning SQL and protected config writer. Its CLI checks all four role names and the
application database before creating server objects, verifies pgvector 0.8.6 availability, provisions
the fixed extension schema and four limited roles, and writes four exclusive mode-0600 credential
files. `postgres-admin.json` matches the existing migration-config lookup. Importer TEMP permission
and serving read-only defaults remain distinct. Failed bootstrap retains protected generated
credentials for diagnosis without automatic rotation or historical recovery. The obsolete upgrade
CLI is deleted; inspected active helper consumers import the bootstrap owner.

The shared disposable-PG fixture uses this same SQL against the current pinned image.
[`test_current_bootstrap_split_credentials_and_existing_database_refusal`](../../../tests/scripts/test_postgres_serving.py)
authenticates each generated config, checks file modes and current schema, and verifies existing
deployment refusal with no elevated roles. Together with the two existing async lifetime/cancellation
controls, `LCTX_POSTGRES_TEST=1 uv run --no-sync pytest tests/scripts/test_postgres_serving.py`
**passed three real-PG tests** ([receipt](../evidence/2026-09-28_pr4/raw/bootstrap-pg.log)). Focused
[Ruff](../evidence/2026-09-28_pr4/raw/bootstrap-lint.log) and
[Pyrefly](../evidence/2026-09-28_pr4/raw/bootstrap-types.log) also passed. This tests the shared
provisioning/configuration contract; it is not a new operator-cluster deployment receipt.

**Source disposition:** **Implemented and focused Tested**, 2026-09-28. The current-only deletion
agrees with ADR-0078 and adds no compatibility branch, dependency pin change or new qualification
gate. The runbook no longer describes pending expansion credentials as an idempotent recovery path.
The implementation owner reports the obsolete protected pending copy removed after credential
comparison as part of the passed current cutover. Owner: operator bootstrap/configuration boundary; current disposition:
forward-plan §6.2.

### Applicable foundations

At **Implemented / source inspected** strength, domain semantics have pure owners, original identity
survives composition, effects remain in service/materialization boundaries, and validation shares
the unit renderer. F10–F13's inspected corrections restore those properties for their identified
scenarios. F14's request-domain, relation dependency and accounting corrections passed the named
catalog-none runtime controls. F15's corrected governing prose now agrees with the authoritative
representations and replay inputs. F16 restores scan-order invariance at the canonical producer and
has a focused red/green regression and current code-gate coverage. Both profiles' serving, current
reconstruction and operator cutover/cleanup passed. F17 preserves explicit refusal
and complete served claims while enforcing the packet boundary independently of evidence
associations. F18 establishes the current installation directly through the existing bootstrap
owner instead of a retired upgrade CLI. This is an
architectural assessment at the stated evidence strength,
not an aggregate score over passing tests. No claim is made about graph algorithms, general
behavioral completion or the unimplemented PR5 tools.

## 8. Library fit and total complexity

| Capability | Assessed choice and boundary | Evidence limit |
|---|---|---|
| Finite selection semantics | A small pure Rust domain kernel is appropriate for the accepted finite vocabulary. PostgreSQL retrieves typed facts; it does not duplicate the classifier as SQL or Python policy. | This review inspects repository semantics, not new library APIs. No generic query language or solver was added. |
| Joint applicability | Whole-conjunction context intersection composes the existing bounded condition contract. Existing scenario task checks do not create runtime condition/evaluation identity. | Runtime demonstrated-combination production remains unavailable without a real linked observation. |
| Retrieval projection | Schema-owned pure rendering plus shared validation is simpler than separately maintained producer/import validators. Immutable retrieval artifacts keep the canonical fact owner distinct. | Broader rebuild commands and clean/reused qualification remain the plan's later obligations. |
| Database/native boundaries | Existing SQLx, Arrow IPC and bounded native execution mechanisms carry typed data and effects. The new tuple query performs a relational membership check without a parallel document hydrator. | No pinned-library upgrade or new API adoption is proposed. Fresh-schema SQLx preparation and the real-PG control qualify their stated interfaces. |
| Lexical and family ranking | bm25s 0.3.11 supplies Lucene BM25; NumPy 2.4.6 supplies score/rank array composition. Family/text deduplication precedes corpus statistics. The finite domain-specific RRF policy has independent Python/Rust implementations and arithmetic controls. | Versions match the repository pins and manifests; the seven Python retrieval controls passed. This establishes implementation arithmetic, not relevance quality. |
| Public wire and native work | Existing Schemars 1.2.2 derives the Rust contract, PyO3 0.29.2 transports prepared handles and detach boundaries, and the existing Tokio lifetime owns bounded blocking work. | Source, focused CPU cancellation and both profiles' actual MCP probes establish the inspected route. These controls do not establish arbitrary request capacity. |

The pinned versions above were checked against [pins](../../pins.md), workspace/package manifests
and the executing call sites. SQLx 0.9.0 checked-query macros are used for the stable new rank,
membership, detail, original-span and alignment reads; descriptor-selected variable relations
continue through the existing typed hydration owner. This is an intentional split between stable
queries and a finite variable projection, not a reason to add an ORM or duplicate query registry.
Arrow schemas remain the shared library data contract. The existing bounded BDD contract handles
the admitted same-evaluation conjunction; no new SMT/Datalog engine is justified by these requests.

The simplest supported route remains exact retrieval with finite family/channel policy. Restoring
deleted ANN or historical-format paths would add unsupported machinery to this scope. A future ANN
proposal must satisfy the accepted admission trigger using addressable units; it is not part of
this assembled PR4 review.

## 9. Alternatives and tradeoffs

| Alternative | Change propagation and test boundary | Judgment |
|---|---|---|
| Duplicate classification in SQL or Python | Predicate additions would repeat completeness, conflict and joint rules in effect/transport owners; pure controls would no longer exercise the served decision. | Reject for this target. Keep the small schema-owned classifier and typed inputs. |
| Keep retrieval vectors/renderings inside canonical facts | A query instruction or renderer change would require a canonical rebuild and couple fact identity to retrieval availability. | Reject. Pure inputs and separately keyed immutable artifacts satisfy the known change without an incremental framework. F10 completes their persisted identity. |
| Introduce a general incremental/plugin/query engine | Adds graph/registry/lifecycle contracts with no additional PR4 consumer; it does not resolve operand ownership or ranking evidence identity. | Defer until a concrete later workload requires it. Existing functions, enums and library boundaries are sufficient here. |
| Independently validate rendered text in the importer | A rendering revision would require synchronized semantic edits in producer and validator. | Reject. Reuse the schema renderer, while retaining adversarial and arithmetic controls independent of it. |
| Raise hydration limits or pass partial JSON as a canonical domain row | Preserves unrelated request costs, or makes canonical hash validation incompatible with the projected content. | Reject. Use a distinct request input and the original canonical identity, with schema-owned finite input dependencies; F14. |
| Keep all legacy retrieval/ANN/recovery routes | Requires preserving unused table/policy/version branches and testing claims the operator explicitly retired. | Reject under ADR-0078. Current exact retrieval and current-format reconstruction are the supported routes. |

## 10. Verification and uncertainty

The reviewer read the stable PR4 raw receipts on **2026-09-28**; these are implementation-owner
runs, not new reviewer executions. The [composite code-gate receipt](../evidence/2026-09-28_pr4/raw/code-gate.json)
is the exact account of the final full run, localized repair and continuation. It records all named
components as passed after repair; it does not claim that the unmodified `just test-all` invocation
exited successfully. The [current evidence owner](../evidence/2026-09-28_pr4/README.md) records
subsequent profile, reconstruction and cutover receipts.

| Command / receipt | Outcome and exact scope | Remaining boundary |
|---|---|---|
| `just fmt`; `just test-all` format/Clippy/Ruff subgates; `just fmt-check`; [format receipt](../evidence/2026-09-28_pr4/raw/fmt-check.log) | **passed:** Rust/Python formatting and named lints. | These are code-gate components, not product quality evidence. |
| `just test-all` release workspace nextest, no fail-fast; [full run](../evidence/2026-09-28_pr4/raw/test-all.log) | **failed:** 473 passed, one failed, 20 skipped. The sole failure was `all_techniques_guard` after canonical domain ordering changed expected output. | The failure is preserved rather than relabelled as an all-green invocation. |
| `cargo nextest run --release -p cpg-core --test syntax --no-fail-fast -E 'test(all_techniques_guard)'`; [repair](../evidence/2026-09-28_pr4/raw/guard-repair.log) | **passed:** the one localized expected-output repair. All 474 ordinary Rust tests are accounted for by the full run plus this rerun. | Deterministic embedding fixtures establish their stated mechanics, not a live provider result. |
| `just py-check rules-scan rules-test lint-agents fixtures-check deps gold test-postgres sqlx-check`; `just adr lint`; [continuation](../evidence/2026-09-28_pr4/raw/gate-continuation.log) | **passed:** 191 Python tests (two optional skips), Pyrefly, seven rule suites, 90 fixture parses, dependency/fork/shear/gold/agent/ADR checks, 19 real-PG Rust tests (three excluded), two real-PG Python tests and SQLx metadata. | The real-PG rank control includes exact score/rank/winner, promotion/count, malformed ranks/foreign fragments, member/unit/original round trips and CPU cancellation. Its vectors are synthetic. |
| Catalog nextest reversed-input control; [before](../evidence/2026-09-28_pr4/raw/domain-order-red.log), [after](../evidence/2026-09-28_pr4/raw/domain-order-green.log) | **failed before correction; passed afterward:** one catalog target, including equality of serialized canonical domains and IDs under reversed scans. | Focused F16 evidence; the owner-reported 60-member diagnosis is not attributed to an overwritten pilot log. |
| F17 [packet-bound test](../evidence/2026-09-28_pr4/raw/packet-bound.log), affected [Python operation/catalog tests](../evidence/2026-09-28_pr4/raw/packet-python.log); `cargo clippy --release -p lctx-postgres --all-targets --quiet -- -D warnings` | **passed:** one exact/over-boundary packet test, 16 Python tests and packet Clippy, the latter recorded by the implementation owner in the composite receipt. Both typed Operation branches were source-inspected. | Serving-only follow-up after the composite gate; it does not change the canonical producer. |
| `LCTX_POSTGRES_TEST=1 uv run --no-sync pytest tests/scripts/test_postgres_serving.py`; [bootstrap](../evidence/2026-09-28_pr4/raw/bootstrap-pg.log), [Ruff](../evidence/2026-09-28_pr4/raw/bootstrap-lint.log), [Pyrefly](../evidence/2026-09-28_pr4/raw/bootstrap-types.log) | **passed:** three real-PG tests covering fresh split-config authentication/current-schema checks/existing-deployment refusal and the two existing lifetime/cancellation controls; focused lint/type checks passed. | F18's follow-up uses shared production bootstrap SQL and the current pinned disposable image. It does not claim a fresh operator-cluster installation. |
| Source/fixture and ADR clause comparison using `nl`, `rg` and Git | **passed:** F01–F18 corrections and surviving ADR-0068 contract transfer inspected, including complete replay inputs, current-only §14.13 wording and current bootstrap ownership. | Source evidence is Implemented; it does not establish throughput or comparative task quality. |
| Fresh catalog/behavioral `--embedder none` pilots after F16; [serving receipts](../evidence/2026-09-28_pr4/raw/pilots.json) | **passed:** both profiles' actual PG/MCP probes, six lexical queries each, four-family original-byte expansion and the named default/selection/type controls. Catalog hydrated 30 packets; behavioral hydrated 28 and reported two explicit expanded-budget refusals whose every winning unit still expanded. | The final runtime reuses both corrected canonical compiles and records canonical/serving binary fingerprints separately after the serving-only packet change. No live embedding or comparative quality claim follows. |
| [Profile contract parity](../evidence/2026-09-28_pr4/raw/profile-parity.json); [current reconstruction](../evidence/2026-09-28_pr4/raw/pilot-reconstruction.json) | **passed:** 18 catalog relations agree as contract multisets under the receipt's explicit profile/source-identity exclusions; 88-table current-format reconstruction preserves both generations and the selected pointer, and both reconstructed generations serve. | Contract parity is not byte identity of provenance-bearing rows. Current reconstruction does not establish historical/mixed-version support. |
| `operator_cutover.py build/pr4-qualified-pilots build/pr4-operator-current`; `cleanup_current.py --apply`; [cutover](../evidence/2026-09-28_pr4/raw/operator-cutover.json), [cleanup](../evidence/2026-09-28_pr4/raw/cleanup.json), [post-cleanup status](../evidence/2026-09-28_pr4/raw/operator-status.json) | **passed:** exactly two current generations remain, behavioral selected; both serve through real PG/MCP and the operator database passes 88-table current reconstruction. Cleanup records 363 obsolete task paths, 48 old artifact files, six temporary reconstruction paths and 118 stale fixture paths removed, with 41 current artifact files retained/verified. Post-cleanup status confirms current schema, ready publication, available generation and no artifact failure. | Default store/generation locations point to the current behavioral profile. Removal of the duplicate pending credential file after matching is implementation-owner reported passed. No prior runtime epoch or rollback archive is retained. |
| Embedding-related PR4 qualification | **not_run — operator waiver** during a critical GPU benchmark | No live embedding or hybrid qualification is inferred from deterministic fixtures or lexical-only pilots. |
| `just docs-check`; [documentation receipt](../evidence/2026-09-28_pr4/raw/docs-check.log); `just adr index` and `just adr lint` | **passed:** 168 canonical pages, zero publication/link errors. The implementation owner reports the ADR index/lint passed with 41 records. | Documentation publication qualifies the current document graph, not comparative product quality or waived embedding behavior. |

## 11. Authority changes and dispositions

| Subject | Authority and disposition route | Review evidence |
|---|---|---|
| Contextual selection, retrieval units and pure/effect separation | ADR-0077 and §14 remain the owning decision/design. The source fixes implement that target; no new ADR is needed for these corrections. | F01–F07, F10–F12. |
| Current-only exact runtime and deletion | ADR-0078 and the forward plan own current reconstruction, deferred ANN admission and cleanup after validation. This review does not reintroduce historical compatibility. | Assembled consumer trace, F10 current artifact selection and passed final cutover/cleanup receipts. |
| Bounded serving and evidence | Existing native/PG owners implement admission, pinned cursors and original expansion; F13 bounds unit display metadata. F14 requires request-scoped inputs to preserve the default pilot route within existing budgets. | F08/F09/F13/F14. |
| ADR retirement and current architecture prose | ADR-0078 supersedes the prior serving/profile/recovery decisions; ADR-0077 owns changed selection/fusion and retrieval replay. Current sections now name snapshot plus the selected immutable retrieval realization. | F15 source correction inspected. |
| Canonical domain scan order | Core derives deterministic canonical nested contexts and set-valued evidence before hashing. Shared publication validation continues to require exact source equality. | F16; focused regression, final profile serving and current reconstruction passed. |
| Bounded enriched operation packets | Explicit refusal remains allowed by §14.9. Both typed Operation branches enforce the cap; PR5 may add honest optional-enrichment pagination/omission for the measured oversized cases. | F17; focused controls and both profiles' required original-unit runtime expansion passed. |
| Fresh current-only provisioning | Bootstrap directly owns current roles, extension and split configs. The backup-dependent upgrade CLI is removed; the owner reports the protected pending credential copy retired after comparison during cutover. | F18; source inspection and three real-PG controls passed. |
| Execution status and closure | Forward-plan §6.2 owns current disposition for PR4/F01–F18. This document remains the dated assessment and source of stable finding IDs. | Attach final receipts there; update owning design/status prose when qualification changes what is true. |

## 12. Architectural judgment and decision

| Judgment | Verdict | Assembled evidence |
|---|---|---|
| A1 Localize change | satisfied — source and named focused/runtime evidence | Predicate semantics and canonical rendering have single owners. F14's request-domain/dependency/accounting corrections preserve the shared classifier and passed the named catalog-none runtime requests. F15's corrected owner prose agrees with this ownership and its replay inputs. |
| A2 Encode meaning structurally | satisfied — source and named focused evidence | Context/basis/closure, nominal IDs with membership admission, generated parameter endpoints, unit/fragment identity and explicit metadata omission carry the distinctions needed by served consumers. |
| A3 Extend through composition | satisfied — source and named focused evidence | Existing pure/effect owners compose rendering, ranks and original expansion. F14 retains full canonical validation and one classifier. F16 canonicalizes unordered producer inputs before identity without changing publication validation or adding recovery exceptions. |

**Assembled PR4 architectural decision: Accept at Implemented and bounded Tested strength.**
F01–F18 corrections are source-inspected; F16's reversed-input regression passed after exposing the
ordering defect. The named code-gate components passed through the explicit composite receipt.
Both fresh profile compiles are reported passed after the canonical producer correction, and their
inspected final serving, 18-relation contract parity and 88-table current reconstruction receipts
passed. F17's later serving-only packet correction passed its focused controls; explicit oversized
behavioral refusals are an admitted boundary, with PR5 usability work deferred. F18's direct current
bootstrap passed its three real-PG controls. The ADR transfer preserves surviving contracts and
complete replay inputs. Final operator cutover, both profiles' serving, selected-pointer
reconstruction and current-only cleanup passed. The forward plan records F01–F18 closed at this
bounded strength under the embedding waiver. The final documentation publication receipt passed
with 168 canonical pages and zero errors; post-cleanup runtime status confirms ready and available
artifacts on the current schema. No qualification work remains pending in this bounded review.

**Enclosing product:** not qualified by this review. The named focused/default receipts do not
establish capacity for every predicate. PR5 tools, general behavioral completion, live embedding
behavior and comparative product superiority remain outside this qualification.

This document supplies evidence for forward-plan §6.2. It changes no architectural authority and
does not supersede ADR-0077/0078 or the PR4 execution plan.
