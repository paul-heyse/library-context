# Unified persistent SurrealDB plan: independent target assessment

**Decision: Accept, at Proposed / Interface-checked strength.** The plan specifies a coherent shared-content architecture and a credible execution route for repeated compilation, pinned serving, concurrent logical tests and worktrees. Its revision resolves the restore isolation defect TF01 identified during authoring: ordinary restore now decodes data into fresh logical staging rather than executing native dump commands against a shared database. This decision accepts the document's selected architecture; it does not establish a working persistent service or production implementation.

## 1. Scope, standard and evidence

| Field | Assessment |
|---|---|
| Subject | [Unified persistent plan](../../plans/unified-persistent-surrealdb-plan_2026-10-09.md), §§1–10, UP0–UP9; coordinator and affected reuse/execution/storage consumers |
| Baseline | HEAD `23daeaa8e7b7b777ff16dee91acfc08aed4e8b49`, inspected 2026-10-09; newly drafted plan and identified concurrent documentation edits |
| Tier / purpose | Design / target |
| Reviewer | Independent delegated design reviewer, 2026-10-09 |
| Standard | Core/template3.3, efficient-architecture heuristics1.0, code-intelligence profile1.5 and repository binding, loaded through [standard.toml](../design_principles/standard.toml) |
| Functional outcome | One durable service and shared content for production, ordinary tests and agent worktrees; a few stable logical databases; no scratch-server or database-per-test fallback; checked no-replay reuse |
| Workload | Unchanged and changed pinned libraries; concurrent worktrees/analyzer revisions; exact catalog/evidence reads; high-degree and global analyses; publication during serving; interruption, restart, restore and retirement |
| Exclusions | Implementation, service installation or inspection, database operations, configuration, cleanup, builds, tests, probes, protected evaluation, real-library qualification and operator activation |

The review independently read the source review, semantic/storage owners and decisive current code. Concurrent AF lifecycle, SQL/MCP diagnostics and run/verification edits were examined only where their contracts intersect this target. Their whole implementation is not certified here, and their dirty files were not changed. The initial source review's clean `05cc139b` baseline is not the examined plan baseline.

The criteria are the loaded standard and functional outcome. Accepted RC01–RC07 and existing ADRs establish context and routing; agreement with those rules is not the architectural argument.

## 2. Responsibilities and domain fidelity

The design separates semantic authority, reusable content, current execution authority and physical lifetime. Those distinctions are necessary for sharing; combining them under a generic cached-result or database handle would weaken the target.

| Owner | Responsibility and contract | Direction / expected change |
|---|---|---|
| `lctx-model` | Nominal/content identities, provider attribution, operation/dependency meaning, coverage, projections and validity | Compiler, native lowering and serving consume these meanings; new fact or analytic semantics begin here |
| `lctx-surrealdb` | Mechanical payload/anchor/membership storage, exact resolution, guarded effects, pins and internal retirement | Implements model contracts; schema or transport changes stay here and in explicit installation |
| `cpg-core` | Bind inputs, reuse or compute outputs, establish current contribution/binding, independent admission | Composes owned operations without reconstructing view predicates or reuse semantics |
| `lctx-publisher` / serving | Publish exact admitted manifests; read the pinned realization across search, evidence and continuations | Visibility remains separate from selection; no database-wide snapshot assumption |
| Service supervisor / AF owners | Installation, attachment, maintenance admission, actual client/child drainage and cleanup certainty | Service/storage outlive each attachment and checkout; no semantic scheduler |
| Native lifecycle / storage manager | Native owner decides reachability/retirement; manager observes and delegates | Filesystem management cannot independently delete native internals or release pins |

**Fact and fidelity preservation:** the plan changes realization and lifetime, not what providers or analyses assert.

| Family | Provider / revision authority | Fidelity and coverage | Identity / consumers |
|---|---|---|---|
| Syntax and lexical facts | Pinned independent Ruff producer contract | Extracted observations with source association and explicit coverage | Nominal occurrence/source plus immutable payload; normalization/catalog |
| Typing and runtime-flow facts | Separately pinned Pyrefly and ty contracts | Provider-specific assertions; disagreement, unsupported scope and unknown remain explicit | Attributed facts and evidence; resolution/behavioral operations |
| Catalog/resolution | Model-owned normalization and catalog operations | Derived under exact dependencies and policies; empty does not erase unknown coverage | Typed relationships and support; discovery/serving |
| Structural and behavioral results | Declared projections and finite model kernels | Exact over the projection or conservative under the stated runtime model; NotRequested distinct | Result/proof/context identities; downstream compilation and synthesis |
| Analytic/ranking/vector values | Selected method, encoder and policy specifications | Heuristic/numerical, never proof of behavior; settings and diagnostics retained | Product/specification identities; navigation/ranking |
| Served assertions and originals | Programmatic synthesis and exact publication view | Claims retain supporting facts and original ranges in that same view | Realization-bound references/resources/cursors |

The inspected current [completed descriptors](../../../crates/lctx-model/src/domain/completed.rs) already distinguish contribution specification, immutable view, exact dependency selection and current/frozen binding. They are useful foundations. Their present lifetime and physical address assumptions still need migration; their existence does not establish cross-attempt attachment.

## 3. Shared content and exact-reference contracts

Plan §4 selects content-addressed payloads, nominal anchors and a compact exact view resolver. This resolves the important same-key revision question: distinct views may select different payloads under one nominal key; one view cannot silently select two conflicting meanings. An anchor's existence carries no selected-view membership. Forward references resolve through the view; reverse references also restrict their source payloads to that view. Independent vertices, isolates, parallel arcs, self-loops and role positions survive.

This is a credible replacement for current [schema](../../../crates/lctx-surrealdb/src/schema.rs), whose entity/assertion and backing-row uniqueness is global by nominal key, and [compiler memberships](../../../crates/lctx-surrealdb/src/compiler.rs), which rely on the private database for broader isolation. Native enforced endpoint existence alone is weaker than exact semantic closure. The plan makes that limitation explicit rather than relabelling physical referential integrity as admission.

One native resolver serves compiler reads, admission, reconciliation, publication, serving and export. That avoids separate view predicates in consumers. Ordered bounded completion constructs its compact mapping once rather than enumerating owner/key products. The mapping is derived access state; contribution membership remains authoritative. Model/codec compatibility, semantic content, physical generation and runtime ownership retain separate identities. Existing stable contribution/view identities can survive when their meaning survives; changed serialized formats require explicit revision and rejection before reconstruction.

Exact-byte comparison on a candidate immutable payload addresses insertion/deduplication conflicts. Retained checked attachment should carry established immutable authority rather than repeat whole-content verification merely because another consumer attaches. The plan's no-replay/current-binding contract supports that distinction; implementation must preserve it.

## 4. Reuse, effects, publication and retention compose coherently

Plan §5's attachment consumes compatible completed content, exact dependencies and current provenance requirements, then produces a current checked binding and pin or a refusal. It does not inherit another attempt's runtime grants. Full canonical output shares only under exact semantic equality; provenance-free values share only through an owner that reconstructs all current attribution. That preserves the model's fact semantics while eliminating the unconditional replay in [workspace_products.rs](../../../crates/cpg-core/src/workspace_products.rs), where a hit currently decodes canonical bodies and writes fresh typed batches.

Dependency lookup, readiness and retention consume the same model-owned distinctions: source, operation/model, implementation, settings, membership/absence/coverage, provenance and physical realization. They do not create another mutable validity graph. An older immutable result remains true under its old inputs even when it no longer applies to a new binding. LIVE/changefeeds are optional wakeups; gaps require reconciliation of affected mutable lookup scope, not invalidation of every immutable view or routine whole-store scans.

Bounded native writes and short guarded completion/publication transitions match the effects they protect. All competing completion, attachment, pin and retirement operations touch relevant guards; there is no global transaction lock or assumption that snapshot isolation serializes arbitrary predicates. Unknown acknowledgment reconciles the original operation/committed identity before retry. Unresolved effects remain protected and unpublished. Recovery reuses completed prerequisites and reruns missing work without pretending arbitrary pending providers are resumable.

Publication in §6 binds exact admitted content, originals, outcomes, vectors and executable definitions. Existing [publication](../../../crates/lctx-publisher/src/native_publication.rs) creates a database VIEWER and `publication:current`; that database-equals-snapshot interpretation cannot survive sharing. The proposed trusted Rust view boundary and immutable named definition epochs replace it consistently across NativeReader, CLI, PyO3/MCP, evaluator, resources and continuations. Raw VIEWER credentials stay internal. Compatible additive definitions preserve old pins; incompatible schema/service changes require explicit maintenance.

Pin acquisition and retirement eligibility share native guards. Retirement first makes the object unavailable to new attachment, rechecks reachability and performs bounded recoverable deletion. Active pins, retained products, published manifests, evidence and unresolved effects remain roots. Worktree removal, client exit, heartbeat loss and generation change do not manufacture content release. This gives the storage manager a real native owner to call rather than a second retention ledger.

## 5. Whole-operation execution fit

The credible benefit is removal of repeated schema/provisioning, canonical row replay and rich reconstruction solely to acquire fresh ownership. A durable daemon alone would remove only startup; the selected content/view/attachment changes address the larger physical work. The plan does not promise that persistence shares client heaps or that bounded output bounds examined work.

| Operation / question | Universe, method and settings | Fidelity, limits and evidence |
|---|---|---|
| Catalog and callable construction | Exact completed source/context domains; declared model operations and policies | Derived; current support and coverage preserved; no retained test pass substitutes for executing a decision |
| Structural topology / SCC | Complete declared vertex inventory and typed directional multigraph; qualified finite kernels | Exact projection; nominal identities remain outside dense indices; compact topology amortizes consumers |
| Behavioral summary | Declared Local/Model/source-call inputs, finite transfer/BDD/fixpoint semantics | Exact/conservative under model; exhaustion/unknown stays distinct from complete absence |
| Discovery/search | Eligible pinned occurrences plus encoder/projection/ranking policy before channel limits | Heuristic navigation; unqualified approximate native filtering is not accepted; exact-vector policy is explicitly identified |
| Evidence retrieval | Exact manifest, selected support and original ranges | Same realization throughout request/resources/cursors; hydration separated from compact selection |
| Backup/recovery | Explicit full authoritative scope and external recovery closure | Necessary whole-scope work; native read snapshot and retention hold; ordinary data-only import and explicit maintenance recovery have distinct authority |

Composite indexes, native selection/joins/aggregation, bounded external ordering and appropriate Arrow/DataFusion transfer can reduce inputs before rich decoding. Global algorithms still consume their necessary universe. Finite graph/BDD/transfer kernels are retained where traversal cannot substitute. Byte-aware queues, cancellation/drain, prompt release and actual representation overlap remain UP4/5/7/8 obligations. Ordinary compiler/test parallelism is preserved; extra thread caps or deadlines are not the remedy for structural amplification.

Compact view mappings add write and retention cost, especially for many similar views. The plan compares immutable segments, then selects mappings because sparse selection and cross-contribution sharing are direct consumers. That is a reasoned initial tradeoff. UP8's skew/retention inspection can reopen the specific representation; no general Merkle-tree framework or scheduler is justified now. No latency, throughput or capacity improvement is Measured.

## 6. Change and failure scenarios

These are Proposed routes, not executed outcomes.

| Scenario / kind | Owner and intended propagation | Revealing evidence / constraint |
|---|---|---|
| Add fact family / domain extension | Model/provider meaning plus generated lowering; shared membership/admission consumers follow | No extra service/database per kind; provenance, coverage and relationship identity retained |
| Add analytic / composition | Declared projection/settings/fidelity and specialized kernel; reuse exact inputs | No duplicate extraction/publication semantics; heuristic output cannot become proof |
| Analyzer/model revision in another worktree / contract binding | Provider and explicit model/codec identities; views coexist or refuse before effects | Equal nominal keys may select different payloads across views; same-view conflict remains |
| Replace gRPC / mechanism | Native transport adapter preserves progressive rows, all terminals, late errors, cancellation and reconnect | Similar item shapes or socket closure cannot stand in for per-request finality; backup transport retained |
| Recompile unchanged input / instance | Checked attachment and fresh current binding over retained compatible content | No canonical replay or inherited runtime/provenance authority |
| Addition/deletion/empty domain / dependency change | Model-owned exact membership/absence/coverage tokens | Compare to independent clean expected results; positive keys alone are insufficient |
| Publish B while serving A / concurrent lifecycle | Manifest publication and definition epoch/pin owner | A's evidence, functions, search quota and cursors remain unchanged; pending B excluded |
| High degree/global analysis / growth | Native selection plus suitable compact/spill/kernel route | Necessary universe complete; inspect examined work and simultaneous representations |
| Client death/restart / failure | Durable effect identities and supervisor maintenance/drain | Unknown effects reconcile; same service/storage preserves completed prerequisites |
| Worktree removal/retirement / ownership | Native reachability plus AF client cleanup and manager delegation | Removing one checkout/consumer cannot retire another view's payload, definition epoch or pin |
| Concurrent logical restores/tests / composition | Publisher data-only restore lowering and validation attempt owner | Definitions are metadata, executable input rejects and mutable identities are fresh; same-dump imports may share only immutable values |

Ordinary negative controls must use fresh owned synthetic records and targeted mutations, never corrupt a shared immutable prerequisite. Global DDL and real fail-stop/restart controls use explicit maintenance admission and actual drainage. Boundary request-loss controls remain ordinary scoped cases; destructive engine corruption is a distinct unsupported campaign requiring an explicit recovery contract.

## 7. Stable authoring finding and static resolution

### <a id="TF01"></a>TF01 — Native logical replay lacks a safe lowering into shared validation storage

**Principles:** FP-02, FP-04, FP-05, FP-06; DP-03, DP-08, DP-18, DP-19; A1/A2/A3; G3/G5/G6.

**Initial plan gap, Proposed:** §6 selected whole-main logical export and untrusted logical attempt staging in stable validation; UP6 named PC5 parser-owned import as its prerequisite. Neither selected the operation that turns database-wide SQL definitions and globally addressed runtime records into isolated logical staging.

**Inspected implementation evidence:** [backup_import.rs](../../../crates/lctx-publisher/src/backup_import.rs), lines28–51, executes bounded units through `client.import`; lines165–195 serialize arbitrary accepted top-level expressions. It bounds and groups SQL but does not implement a data-only allowlist or attempt remapping. [backup.rs](../../../crates/lctx-publisher/src/backup.rs), lines27–45 and399–413, authenticates Root and imports into the selected database. Current SurrealDB3.3 export source emits metadata and table definitions before records. This is not an assertion that the current trusted-local/private-database route violates its original contract; it is a concrete mismatch with the new shared target.

**Consequence:** even a legitimate dump can redefine shared validation tables/functions and collide with existing runtime records while another test/import is active. A malformed scope-switching or definition statement becomes executable authority if the Root path survives. A fresh attempt identifier or later semantic admission does not undo those already global effects. Routing every ordinary restore through exclusive maintenance would avoid some concurrency damage but would leave the claimed attempt-scoped ordinary workflow unspecified.

**Correction selected in revised plan §6/UP6:** the publisher/native restore owner treats logical SQL as untrusted structured input, not directly executable commands. The pinned grammar's bounded parser recognizes supported data forms, compares captured definitions as metadata against installed selected contracts, rejects executable/scope-changing input, and lowers validated rows into fresh owned logical staging. Canonical semantic IDs survive where required; runtime attempt/pin/fence identities are freshly bound and captured live state becomes protected recovery obligations. Only separately authorized maintenance may restore approved definition/service assets. Cold semantic admission remains independent and a failed import retains only its scoped effects and uncertainty. PC5's parser/byte/terminal primitives remain useful; ordinary raw SQL replay and Root import authority are removed or confined to explicit maintenance.

**Static closure, 2026-10-09:** the corrected §6 explicitly selects data-only lowering, definition comparison without execution, rejection of USE/administrative/arbitrary expressions, exact parser/codec contracts, canonical-address validation, regenerated roles/index forms, fresh staging/guards and independent admission. UP6 now requires concurrent same-dump isolation and refusal of scope/DDL/control-ID injection. §7.1 additionally requires targeted fresh validation mutations and prohibits tampering with shared immutable prerequisites. These changes settle the authoring boundary. Runtime implementation closure belongs to UP6/UP9 under [coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition). The source review's F01–F08 remain independent open implementation findings.

**Assessment:** resolved in the Proposed target by independent static rereading; implementation and runtime verification not_run. No remaining material authoring finding was identified in the inspected scope.

## 8. Library fit and alternatives

The locked SurrealDB3.3.0 SDK/engine contract and narrow vendored gRPC backport were inspected at the decisive boundaries; no installed-server identity was inspected. The selected neo4j-surrealdb capability skill provided navigation to version-matched primary source, including core `kvs/ds.rs`, `kvs/export.rs`, `ctx/context.rs`, `doc/edges.rs` and function-definition/lookup paths. Source assertions are Interface-checked, not current runtime results.

- Native record IDs, indexes, relations and bounded transactions are credible primitives for payload/anchor/membership. They do not know the application's exact selected universe or current provenance; the shared resolver and attachment operation supply concrete missing semantics.
- Scoped system VIEWER access does not enforce application view predicates. A trusted Rust boundary is simpler for existing clients than adding direct native ACL machinery without a consumer. Raw native production access remains a distinct later qualification.
- Named immutable definition epochs avoid relying on complete temporal-schema behavior. `IF NOT EXISTS` is not definition equality; the inspected function-definition path returns without comparing an existing body. Version-stamped function lookup alone does not establish complete historical index/restore correctness.
- The vendored gRPC route has explicit application and transport terminal handling. Optional progressive WS remains credible but requires a maintained request adapter and qualified backup combination. `Surreal::clone` creates a new session; sharing an Arc does not justify mutating concurrent database selection.
- One native export read transaction supplies a credible main-database backup point; it does not export all engine history or recover external credentials/configuration. The corrected plan retains the native grammar/parser while supplying the smaller authority needed by application logical restore.

| Alternative | Judgment and tradeoff |
|---|---|
| Keep private databases on a durable daemon | Simplest code change, but repeats canonical ingress/replay and retains database lifecycle coupling; fails the intended shared-content workflow |
| Selected payload/anchor/exact view plus main/validation | Reuses native primitives; adds purposeful resolver/fence/pin lifecycle; removes unconditional data copies and per-attempt schemas |
| Immutable closed segments | Credible if mapping/retention costs dominate; less direct sparse/cross-contribution reuse; revisit through UP8 evidence |
| Push all kernels into SurrealQL | Native reduction is useful, but traversal does not implement BDD/transfer/fixpoint/general analytics contracts; reject universal placement |
| Add generic incremental engine/scheduler | No demonstrated missing owner after model dependencies and checked attachment compose; defer until it replaces real machinery |
| Container or different engine | Viable packaging/storage substitutions with exact identity/durability/workload qualification; no presumed speed advantage over current native RocksDB |
| Execute dump under exclusive Root maintenance | Useful only for explicit whole-system recovery; not a substitute for ordinary attempt-scoped logical import |

This compares complete mechanisms, including setup, indexing, mutation, recovery and operator work. Library-first does not require an unsafe direct import when the native operation has a broader authority than its application consumer.

## 9. Foundations, gates and architectural judgments

These verdicts concern the Proposed document. Passing a planned contract would not establish implementation or release qualification.

| Foundation / supporting rules | Verdict | Evidence |
|---|---|---|
| FP-01/03/07; DP-09/10/13/16/20 | Satisfied for the specified shared compile/serve route | Owned composition, no unconditional replay, compact selected access, appropriate kernels and scoped lifecycle |
| FP-02/04/05/06; DP-03/08/18/19 | Satisfied at Proposed strength | Exact shared views, complete operation contracts, logical staging and explicit maintenance bound authority; revised restore resolves TF01 |
| DP-01/02/04/05/07/11/21/22/24; CI-01–06/08–13 | Satisfied at specified contract strength | Exact views, layered identity, current attribution, fidelity, coverage, evidence and explicit version/policy/claim boundaries |
| CI-07; DP-14/15/23 | Satisfied at Proposed/interface strength | Qualified placement and retained independent controls; unresolved native plans have explicit safe routes and acceptance obligations |

| Gate | Verdict | Basis / action |
|---|---|---|
| G1 Authority | Pass, Proposed scope | Model semantics/native realization/manifest authority distinguished; no second mutable validity owner |
| G2 Semantic fidelity | Pass, Proposed scope | Revisions, nominal identity, view membership, provenance and uncertainty explicitly separated |
| G3 Validity | Pass, Proposed contract | Exact view/reference and independent admission boundaries; revised data-only restore validates input before scoped effects |
| G4 Hidden behavior | Pass, specified effects | Attachment/inspection cannot install/start/select; restore is explicitly effectful and TF01 bounds its authority |
| G5 Consistency/recovery | Pass, Proposed contract | Relevant guards, exact committed identity, protected unknown effects, pins and fresh logical restore owners |
| G6 Transformation/reuse | Pass, Proposed contract | Complete dependencies and current checked bindings; revised grammar-to-data restore preserves semantics without imported execution authority |
| G7 Truthful claims | Pass | Proposed versus Interface-checked and runtime/measurement exclusions remain explicit |
| G8 Library leverage | Pass | Native storage/parser/index primitives plus thin domain-specific lowering; no justified universal framework |
| CI-G1 Fidelity | Pass, Proposed scope | Provider meanings, current support, coverage, model and heuristic distinctions retained |
| CI-G2 Evidence closure | Pass, Proposed contract | Exact resolver/original/search/resource/cursor closure required throughout the pinned journey |
| CI-G3 Evaluation integrity | Pass for changed boundary | Private evaluation data excluded; evaluator adapter migration preserves fixed snapshot semantics; evaluation execution not assessed |

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | Satisfied at Proposed strength | Model/native/compiler/publisher/supervisor/lifecycle owners align with distinct changes; revised restore has bounded data/definition/maintenance contracts |
| A2 Encode domain meaning explicitly | Satisfied at Proposed strength | Content revision, exact membership, attribution, runtime grants, effects and recovery obligations have deliberate contracts consumed by all named adapters |
| A3 Extend through composition | Satisfied at Proposed strength | Owned attachment, resolver, admission, publication and retirement operations compose; new facts/analytics and qualified mechanism substitution have clear routes |
| A4 Fit execution to workload | Satisfied at Proposed strength | Removes concrete repeated work, retains necessary full analyses, compact indexed selection and bounded recovery; no speed claim |

## 10. Verification and uncertainty

**Passed, 2026-10-09:** read-only identification with `git rev-parse HEAD`, `git status --short`, scoped `git diff` and static inspections listed above; independent rereading of revised §6/§7.1/UP6 and coordinator acceptance wording closed TF01 at document strength. These establish the examined source and document assessment only.

**Not_run:** builds, tests, runtime probes, benchmarks, service/database actions, configuration, cleanup, real-library qualification and operator activation, per the assignment. Documentation publication checks belong to the coordinator and are not represented as this reviewer's runs.

Necessary future evidence is already allocated to actual consumers: payload/reference/index plans in UP2; guards/current attachment in UP3; clean-versus-reused current provenance and membership in UP4; pinned definition/search/index behavior in UP5; complete recovery and safe logical import in UP6; all destructive fixture paths and true cold controls in UP7; pin/retire/resource composition in UP8. A bad native realization blocks its dependent package and UP9 rather than weakening semantic guarantees or stopping unrelated ready work.

Implementation controls should include isolates, parallel roles, self-loops, foreign endpoints, absent/empty domains, mismatched model/provenance, concurrent equal insertion, late writes and lost commits. They challenge independent expected semantics; a warm retained success cannot replace executing the decision being tested. A representative real-library or quantitative performance campaign remains separately authorized.

## 11. Rule impacts and integration

No additional rule change beyond source RC01–RC07 is recommended. TF01 refines the mechanism needed to satisfy their shared-content and logical-isolation outcome; it does not request another operator decision or waive independent cold admission.

UP0 routes the accepted changes through superseding/complementary ADRs and governed semantic/storage/serving owners before implementation. ADR-0138/0140 bodies remain immutable; useful ADR-0141 access and completion guarantees survive. The existing private-database lifecycle, unconditional replay and database-wide reader authority become dated implementation descriptions, not prerequisites to completing the new target.

The [persisted coordinator](../../plans/persisted-graph-execution-plan_2026-10-07.md) retains sole scheduled source-finding disposition and actual receipts. UP9 combines affected surviving PC/PJ/GK/GR/NE/CU/BC acceptance after their actual consumer migrations; BC3 profile qualification, operator adoption and real-library campaigns remain separate. AF1/4 cleanup/outcome and AF6/7 diagnostics are consumed at their working boundaries without certifying all concurrent AF changes. Storage management delegates native retirement and cannot infer release from a removed checkout or missing heartbeat.

**Final static integration check, passed 2026-10-09:** scoped `git diff` and current section reads confirmed the following routes. This is document/source inspection, not execution qualification.

| Consumer / owner | Inspected integration |
|---|---|
| Persisted coordinator §§4/6/7.3/8/9.1 | Active target now uses shared payloads/manifests and guarded recovery; UP9 integrates surviving acceptance; source F01–F08 remain open at the sole owner and old receipts retain their sources |
| GR §4 and packages/acceptance | Disposable/replay-only storage replaced by checked admitted-content attachment; optional untrusted portable values retain typed admission; authoritative content excluded from generic cache eviction |
| NE/GK | Preserve owned operations, exact inputs, preparation, independent controls and finite kernels; actual reader/reuse/fixture consumers migrate through UP2/4/5/7/8 |
| PC §5 and PJ §3.3 | Data-only restore explicitly replaces Root dump execution; final freeze covers the current attempt/read set rather than unrelated shared writers |
| Storage / AF integration blocks | Native retirement stays native-owned; service/coordination survives checkout removal; AF cleanup/outcomes survive while SQL/MCP diagnostics are confined to synthetic maintenance scope |
| Retained graph-native pivot/realization/serving plans | Explicit precedence links replace private/disposable and database-wide reader assumptions; dated descriptions remain their original baseline |
| Source review disposition | Transfer points to coordinator §8 without rewriting the original Revise judgment or claiming implementation closure |

STATUS and documentation publication receipts are the coordinator's final handoff; they do not change this architectural decision. This review does not certify unrelated concurrent AF edits shown in the same tree.

## 12. Decision and next boundary

**Bounded decision: Accept the selected document architecture at Proposed / Interface-checked strength. Enclosing implemented architecture: not accepted or runtime-qualified for the unified-persistent scenarios by this document review.** TF01's logical restore/staging boundary is resolved in the plan. A1–A4 and applicable gates are satisfied at document strength; no remaining material plan defect was identified.

The coordinator owns publication and integration of this assessment. Subsequent implementation must establish the explicit UP0 contracts and working service/resolver/effect prerequisites before consumers depend on them. Source F01–F08 remain open until their coordinator-owned closure evidence is actually delivered; this plan acceptance does not close them.
