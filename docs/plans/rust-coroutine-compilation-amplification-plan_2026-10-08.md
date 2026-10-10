# Rust coroutine containment: checked loaders and coherent producer phases

**Holistic integration, Proposed, 2026-10-10:** the [holistic companion](holistic-state-management-plan_2026-10-10.md) §7/§9
releases driver graph/binding preparation at its last actual consumer while preserving the
contained async phases and synchronous finite kernels qualified here. Reusable SCC scratch
remains conditional on a real repeated-SCC consumer; it is not required for last-use release.
CU6 affected acceptance joins the persisted coordinator; BC3 profile adoption remains separate.

**Implemented / acceptance in progress · 2026-10-08.** CU0–CU5 source corrections are integrated; CU6 compiler/runtime acceptance remains open.

## 1. Outcome, authority and confirmed coverage

Make growth in typed relation inventories add small record-specific operations rather than expand
large enclosing async state machines and their cleanup graphs. Preserve the domain semantics,
selected immutable inputs, batch accounting and native completion guarantees that those operations
already implement. The [source design review](../design_review/reviews/design_review_rust-coroutine-compilation-amplification_2026-10-08.md)
requires revision for A4 while accepting the inspected semantic and lifecycle boundaries.
Its F01 extends [original compilation-cost F01](../design_review/reviews/design_review_rust-compilation-costs_2026-10-08.md#F01).

The [compilation-cost coordinator §9](rust-compilation-costs-plan_2026-10-08.md#9-sole-compilation-cost-finding-disposition)
remains the sole owner of current finding disposition. This companion owns the coroutine target,
consumer corrections and package acceptance criteria within BC1; the coordinator owns BC3 profile
installation and BC5 integrated qualification. The [persisted-execution coordinator](persisted-graph-execution-plan_2026-10-07.md)
retains PG/PC findings and native/CLI/cold/MCP acceptance. No companion status table duplicates F01.
[Semantic model §15](../design/sections/semantic-model.md) and [analytics](../design/sections/analytics.md)
retain semantic and runtime contracts; this plan does not replace them.

**Operator-confirmed coverage, 2026-10-08:** structural correction first, then mandatory assessment
of every named recurrence candidate and correction wherever the same cause is confirmed. The
alternative of leaving all other candidates as an unscheduled backlog was not selected. Adjacent
publisher/serving/native-loader leads receive bounded triage, not an equal-depth serving audit.

**Rule impacts: no new architectural rule.** Coroutine containment preserves pins, profiles,
semantic authority and native lifecycle. The operator resource follow-up in §7 aligns ordinary
workspace/fixture defaults with shared64GiB headroom and automatic available-CPU partitioning;
intentional refusal/invariance controls retain their chosen bounds. Existing ADR-0138/0137 decisions stay
with their owners. Ordinary internal phase/adapter extraction needs no new ADR; a later change
that alters a consequential architectural decision must use the existing decision route before
its dependent implementation. No such change is needed for the selected target.

## 2. Foundation assessment and evidence boundary

Authoring starts from main `32ef660c5b258200067c18f481e75eac55f17a86` and the preserved integrated
BC/PC dirty tree. Ordered C1 loader groups, conditional coverage, shared stream setup, native
submission and charged writer completion are implemented and independently source-reviewed.
Their focused compile/model receipts are retained at coordinator §12; actual native acceptance
is pending. None is re-reported as a test run performed during this authoring scope.

The source review applies core/template3.3, heuristics1.0 and code-intelligence1.5. Useful foundations
are nominal records and mechanically derived codecs, exact completed-view permits, model-owned
frame/outcome operations, prepared selected closures and borrowed synchronous graph kernels.
Preserving those foundations is cheaper and clearer than introducing a runtime schema visitor,
new executor, task-per-row interface or alternative publication authority.

C1's helper-inclusive interrupted comparison supports coherent boundaries: parent MIR25,100→591
and retained ≥1ms completed InstCombine totals350.33s/353.16s→0.185s/0.266s (normal/harness).
Both captures are partial and filtered. This is **Measured partial diagnostic**, not whole-build
improvement or runtime performance. The retained structural raw body has7,949 blocks and37,772
instructions, with zero pre-optimization PHIs. A simplified pinned-LLVM `default<O1>` reproducer
with a null target machine expands its named body beyond2.25 million instructions and samples
PHI optimization heavily. The actual rustc pass sequence and transformed PHI shape are unresolved.
The source review and coordinator §12 own raw run IDs, scripts, hashes and attribution limits.

Static source establishes avoidable propagation independently of the optimizer's exact inner
loop: inventory macros insert many concrete await sites into an operation that only needs ordered
checked dispatch. Raw sizes identify candidates; they do not rank elapsed costs. Source-inspected
recurrence is adequate to schedule correction. Compiler evidence is still needed for diagnostic
closure and any quantitative benefit claim.

### Named consumers and correction boundaries

| Consumer / decisive source | Existing variation that must remain owned | Planned treatment |
|---|---|---|
| [Structural producer](../../crates/cpg-core/src/structural.rs), inventory48–117, declaration121–129, grain142–147, publication234–308 | Profile-sensitive inventories, artifact admission, exact coverage and configured methods | CU1: primary correction; separate load/declaration/grain/publication phases and shared ordered adapter dispatch |
| [FrameScopes reader](../../crates/cpg-core/src/analytical_scopes.rs),741–764 | Every matching declaration/prefix; grain-selected SQL; typed permit and callback | CU1: shared declaration traversal with thin typed admission/decoding; migrate actual structural and analytic consumers |
| [Summary scopes](../../crates/cpg-core/src/semantic_summaries/scope.rs),226; [model scopes](../../crates/cpg-core/src/semantic_models/scope.rs),236; [model producer](../../crates/cpg-core/src/semantic_models.rs),93/113/148/332/433 | Summary's specialized visitors and selection; model's selected witnesses, captured-catalog filter, ProductionScope roots, merge_run and evidence publication | CU2: same macro-expanded loader/producer cause is source-confirmed; stage-specific adapters and coherent model application/publication phases preserve special branches |
| [Selection](../../crates/cpg-core/src/catalog_selection.rs),352/376; [synthesis](../../crates/cpg-core/src/synthesis.rs),407/596 | Exact consumed declarations; selection ownership; documentary/member/conclusion phases and chunk selection | CU3: confirmed loading pattern; inspect producer/declaration/emission children and correct confirmed enclosing expansion |
| [Retrieval preparation](../../crates/cpg-core/src/retrieval_preparation.rs),44 and scoped load | Projected artifact-property admission, frame metadata, exact roots/briefs and per-root release | CU3: confirmed inventory pattern; preserve owner-selected projected columns and selected render scopes |
| [Analytic producer](../../crates/cpg-core/src/analytic.rs),24; [summary producer](../../crates/cpg-core/src/semantic_summaries.rs) | Configured analysis, selected technique sets, model computation and coverage/outcomes | CU4: inspect whole operations and contain confirmed inventory/declaration/emission expansion; retain pure kernels |
| [Semantic execution](../../crates/cpg-core/src/semantic_execution.rs),80/421/754/1083 | Base evaluation/completion, source calls and enrichment have distinct inventories and model operations | CU4: source-visible typed await inventories warrant correction; design each operation's coherent phases without merging meanings |
| [Normalization CallScopes](../../crates/cpg-core/src/normalize/call_scope.rs),513/530 | Event/binding visitors and existing first-match versus all-match behavior | CU4: confirm declaration-cardinality contract, then contain await composition without silently changing selection |
| [Output buffering](../../crates/cpg-core/src/workspace.rs),2124 | Typed validation, pending buffer, cross-flush state and native terminal ownership | CU5 conditional refinement; `push` submits only at flush, not once per row |
| [Publisher search](../../crates/lctx-publisher/src/search.rs),163; [serving source evidence](../../crates/lctx-serving/src/source_evidence.rs),43; [native loader](../../crates/lctx-surrealdb/src/loader.rs),72 | Bounded transfer and prepared selection | CU0 bounded triage: correct only confirmed repeated nested-await mechanics; otherwise record exclusion and revisit trigger |

Model derives, typed key/digest/validation/codec leaves and nominal `Batch<R>` have no attributed
PHI defect. Retain them. Existing petgraph views/iterators, graph hydration and pure computations
are positive examples. Already shared stream/native request implementations remain the transport
foundation; do not rebuild them under another owner. Fingerprint and harness findings remain F02/F04
at the coordinator, not new defects in this plan.

## 3. Target operations and internal interfaces

### 3.1 Loader composition

**Proposed replacement composition, 2026-10-09:** [GK1–GK4](graph-compilation-kernels-and-hashing-plan_2026-10-09.md)
move complete scope meaning to model-owned programs and compile shared demand; [GR1–GR4](graph-compilation-reuse-and-invalidation-plan_2026-10-09.md)
add qualified program/product reuse. The containment contract below remains: thin typed leaves,
uniform non-generic async mechanics, exact ordering/cardinality, charged lifetimes and drainage.
GK migrations supersede open duplicate loader routes rather than implementing both designs.

Generate ordered typed adapter entries directly from each existing semantic inventory. A typed
adapter is an ordinary function returning a borrowed opaque future before the broad driver owns
it. Its record-specific work acquires the exact permit and supplies the synchronous decode/visit
callback. A shared non-generic loop owns sequential traversal and awaits uniform futures.

Keep scope policy with its model owner: frame, summary, model, selection, synthesis and retrieval
declare their complete operation; native/DataFusion/graph compilers realize it without independently
deciding its meaning. Reuse uniform traversal only where contracts agree. No independent
maintained decoder registry, schema-wide runtime data model or universal operation context is
introduced. Inventories that overlap remain ordered entries; `ConsumedInputs` remains responsible
for whether a declaration still needs work. A repeated type is not sufficient grounds to remove
an entry, sort the dispatch list or merge prefixes.

CU2 includes the model loader's actual producer consumer, `semantic_models::apply`, and its
`publish_records` child. Contain its metadata/expected-input inventory, output declaration,
selected-root application and typed evidence emission, not only `ModelScopes::load`. Preserve
captured-catalog filtering, Catalog/Behavioral branch distinctions, externally ordered
`ProductionScope` roots, synchronous `apply_selected`, `merge_run` aggregation and publication
of each selected result. Do not reorder roots, omit merged run records or leave the typed
publication inventory expanded inside a newly boxed child.

For the generic `FrameScopes::read<R>` path, contain its declaration iteration and nested awaits
below an opaque boundary as well as changing its parent. Keep exact typed permit admission in a
thin adapter and move uniform physical traversal to a shared implementation. Preserve each matching
prefix, table-index binding and grain selection before decoding. Callback erasure occurs before
shared async mechanics; there is no dynamic call or allocation per decoded row.

Summary's optional `select_actual` and specialized occurrence/artifact/provider-symbol visitors,
model selection, retrieval's projected artifact columns, and synthesis's phase-specific chunk
selection stay in their current owners. Normalization currently uses `.find` in binding/event
loading rather than a loop over all prefixes. Establish whether uniqueness is a construction
invariant; preserve current cardinality unless a separately evidenced semantic correction is
necessary. Do not make normalization consume all prefixes merely to fit a common loader helper.

### 3.2 Structural phases and publication

The driver retains captured sources/admission and parent enumeration. It awaits a small sequence:
initial inventory acquisition; output declaration and selected-scope preparation; selected grain
acquisition for each parent; synchronous model derivation; result and coverage publication; final
producer completion. Borrowed phase interfaces receive only the owners they need and return the
existing typed products or `Result<(), ModelError>`; no public transport or schema change is needed.

Grain selection drops before structural computation. Graphs and data remain borrowed through the
existing `borrowed_cpu` route. Result rows, receipts and projections release per parent; coverage
admission and captured sources survive until all dependent outcomes are produced. Retain final
scope/parent/admission/source release before `output.finish`. Do not collect all parents to simplify
a lifetime, clone charged backing buffers without their charge, or spawn to manufacture `'static`.

Publication has separate responsibilities: register declared outputs, emit typed results and links,
derive model-owned outcomes, then publish admitted expectations/observations/assessments. Place
coherent emission and coverage boundaries behind borrowed opaque futures. A phase with many typed
awaits must itself dispatch through contained typed adapters or coherent smaller children; moving
the same expanded body into a boxed helper does not satisfy containment. Include drop machinery
when inspecting the result, not just the public driver.

Preserve row validation and registration order, first-error precedence, profile-dependent
`NotRequested`, configured methods, projection keys, relationship/condition/qualification meaning,
coverage guard laziness and the existing yield point. An operation created but never polled must
not perform new checks, mutation or native submission merely because preparation moved into an
ordinary function. Use `Box::pin(async move { ... })` or an equivalent existing borrowed boundary
to preserve poll-time behavior where required.

### 3.3 Existing library capabilities and costs

The source review qualified futures0.3.34, Tokio1.53.2, Arrow59.3.0, DataFusion55.1.0 and
petgraph0.8.3 against exact local sources and documentation. Borrowed `BoxFuture` supports phase
and function-pointer adapters without new task ownership. Keep required `Send` bounds; do not
substitute `LocalBoxFuture` into a consumer requiring `Send`. Allocations and indirect polling
occur per coherent operation/adapter, never per row for this correction.

Keep DataFusion's existing erased batch stream and owner-selected query. Query cancellation does
not discharge native write ownership. Keep registered native joins, charged batch/writer retention
and late-failure drainage. Arrow's structural consistency checks do not replace nominal validation.
Petgraph's synchronous borrowed visitation already fits pure graph computation; plain DFS cannot
replace delegation semantics. `Dfs::reset` allocation reuse is outside this correction unless a
matched repeated-traversal consumer supplies a separate need.

Public producer signatures, completed-view authority, wire/schema/codebooks and model algorithms
remain unchanged. CU5 may introduce an internal checked transfer-granularity output operation;
its exact contract must be settled before implementation and is not selected by this document.
No dependency upgrade, feature expansion, profile change or library adoption is needed.

The [native-efficiency companion NE5/NE6/NE9](native-execution-efficiency-plan_2026-10-09.md#4-packages-and-prerequisites)
refines actual native handoffs/completion while retaining CU's contained synchronous kernels,
shared erased request paths, exact semantic order and full charge/terminal ownership. These
runtime corrections do not restart CU source migration or authorize compiler profiling. CU6's
surviving native/CLI/finality obligations join NE9/GK7/GR6 on matching final source; BC3 candidate
adoption remains distinct. This link changes the completion route, not historical receipts.

## 4. Execution packages and concrete dependencies

CU0–CU4 corrections and CU5’s keep-current decision are **implemented / source reviewed**; CU6 acceptance is in progress. The root owns shared reader,
output/native editing surfaces, disposition and integration. Stage migrations can proceed separately
once their working prerequisites exist; shared files have one editing owner. Logical independence
does not permit competing changes to cancellation, accounting or scope-selection contracts.

| Package | Prerequisite supplied | Delivered behavior and completion evidence |
|---|---|---|
| **CU0 — refresh baseline and settle candidate boundaries** | Source review, retained evidence and current integrated BC/PC contracts | Capture current tree/configuration; inspect each named owner and adjacent leads. Confirm same-pattern corrections, preserve special policies, and record justified exclusions/triggers. Establish normalization cardinality and output decision questions. No compiler pass needed merely for triage. |
| **CU1 — structural operation and shared frame reader** | CU0 exact-input/order/lifetime contracts; working BC1 streaming/native ownership | Inventory-derived adapters, shared ordered loops, coherent structural phases and contained nested frame reading; migrate structural/analytic reader callers. Independent overlap/profile/failure/ownership controls plus parent/child/drop inspection. Structural is first because retained evidence supports this operation, not because raw size ranks it. |
| **CU2 — summary/model loaders and model production** | CU0 confirmed special visitors/selection; CU1's working composition pattern where reusable | Correct both loaders plus `semantic_models::apply` and `publish_records`; contain metadata/expected inventories, declarations, ordered selected-root application and typed evidence emission. Preserve optional selection, exact prefixes, captured catalog, ProductionScope/merge_run and profile semantics; actual scope and model-publication controls. Can run alongside CU3 with distinct editing ownership. |
| **CU3 — selection, synthesis and retrieval operations** | CU0 exact consumed/chunk/metadata contracts; working shared reader/stream interfaces | Migrate confirmed inventory/scoped loaders and necessary producer declaration/emission phases with each owning algorithm. Preserve all synthesis phases, selection membership and retrieval projected artifact admission; focused actual scope/producer controls. |
| **CU4 — remaining named production recurrence and adjacent triage** | CU0 cardinality decisions and CU1–CU3 working interfaces when consumed | Summary/analytic producers, semantic execution operations and normalization loading receive confirmed containment corrections with their actual consumers. Adjacent mapped leads receive explicit corrected-or-excluded decisions. No evidence-free migration of all async functions. |
| **CU5 — output decision and conditional refinement** | Working contained loader/publication operations and available source/first-build evidence; no positive hotspot prerequisite for the decision | First decide keep-current versus checked bulk operation from residual source/compiler attribution and a buffering-contract comparison. If not warranted, document a trigger and retain `push`. If warranted, design and integrate the checked internal operation and affected callers together, then verify cross-flush/error/cancel contracts. |
| **CU6 — whole-operation and integrated closure** | CU1–CU4 migrations and CU5 resolved, current final tree; coordinator BC3/BC4 prerequisites | Comparable actual-settings compiler evidence across normal/harness products; revealing native/runtime controls, required stage acceptance and applicable leaves. Share valid BC5/PC6 optimized receipts. Exact compiler-defect investigation follows residual evidence; no finding closes from parent shrinkage alone. |

CU0 is a bounded execution refresh, not a requirement to repeat authoring or the source design
review. CU1 can begin without unrelated PC6 acceptance. CU2–CU4 do not wait for a full structural
journey when their shared interfaces and revealing controls are working. Consume already integrated
PC completion/captured-binding contracts and keep unrelated cold-restore obligations open.

BC3 qualification requires working containment on its exercised compilation/operation paths;
CU0 assessment of all named candidates is not an excuse for silently installing the default while
known blocking paths remain. Final F01 closure additionally requires complete planned migrations,
resolved exclusions and operational evidence. BC5 assembles optimized acceptance once at functional
scope completion; individual CU packages do not each trigger `qualify`.

## 5. Investigations and decision consequences

| Source obligation | Owner / scheduled route | Settling evidence and consequence |
|---|---|---|
| Review §10.1: actual rustc attribution | Compiler diagnostics owner, CU6 | Match hot function and transformed PHI shape to actual pinned pass/target settings if residual attribution could change the remedy or support a compiler-defect claim. Simplified reproduction alone is insufficient. Does not block source-supported containment. |
| Review §10.2: whole corrected operation | Core stage/reader owners, CU1–CU4/CU6 | Inspect concrete children and drops across CGUs; comparable actual compiler capture. A relocated large child reopens containment even if the parent shrinks. |
| Review §10.3: runtime/ownership adequacy | Shared output/native and stage owners, all migrations/CU6 | Actual selected-input, first-error, buffering and terminal charge controls. Static source preservation is not a throughput or peak-memory result. |
| Review §10.4: recurrence | CU0/CU2–CU4 owners | Mandatory assessment of all named candidates, confirmed same-cause migration or supported exclusion with a concrete revisit trigger. No ranking from raw IR counts. |
| Review §10.5: residual compiler defect | Compiler/toolchain owner, CU6 conditional followup | After coherent containment, a matched residual pathology warrants exact pipeline/IR investigation and only then a deliberate toolchain proposal. Do not disable passes or change pins from a related native stack alone. |
| Review §5/§8: shared checked bulk append | Output owner, CU5 | Demonstrated residual output composition plus a contract comparison of order, flush, validation/dedup, poisoning, bytes, backpressure and cancellation. If either justification or preserved semantics is missing, retain current API and record the specific next evidence needed. |
| First-match normalization declaration | Normalization owner, CU0/CU4 | Inspect construction and consumers for uniqueness. Preserve current matching cardinality; an exposed semantic defect needs its own supported correction/acceptance rather than a silent loader change. |
| Review's mapped adjacent interfaces | Publisher/serving/native-loader owners, CU0/CU4 | Determine whether repeated concrete nested-await mechanics exist. An async signature, bounded transfer loop or necessary concrete codec alone does not justify migration. Revisit an excluded path on confirmed source amplification or retained attributed cost. |

A bulk-output decision is not a new mandatory API or separate finding. Its dependent implementation
cannot begin while buffering/error/cancellation semantics remain undecided. The overall containment
work can complete with a justified keep-current outcome. Compiler investigation may similarly remain
triggered while the demonstrated architectural correction closes; any remaining uncertainty that
undermines actual compile fit must be resolved rather than called an optional measurement.

## 6. Verification and acceptance

The commands below define implementation verification. Current execution receipts and their limits are in §7. Resolve current selectors with `just verify --print` before executing; use compile checks and minimal revealing controls during each package.

| Boundary | Revealing scenarios and independent evidence |
|---|---|
| Inventory/selected reads | Same relation at distinct prefixes/completed views; overlapping ordered inventory entries; first loader failure prevents later work; missing/foreign view and unfinished consumption refuse; catalog omits behavioral premises rather than eagerly loading them. |
| Conditional coverage and laziness | Skipped relation does not run its guard; handled relation refuses before mutation. Creating and dropping an unpolled adapter/phase performs no newly eager effect. Expected values/error order come from the contract, not a generated adapter count. |
| Stage selection | Summary occurrence/artifact/symbol specialization; model witness epoch/absence, captured-catalog isolation and distinct ordered ProductionScope roots with independently expected merged/evidence rows; synthesis documentary/member/conclusion chunk selection; retrieval projected artifact admission; selection membership and normalization's established cardinality. Independent unrelated-payload cases challenge source isolation. |
| Computation and lifetimes | Canonical definitions/parameters and exact projection keys; configured structural methods and explicit outcomes; unchanged traversal universe and bounded/partial evidence. Grain releases before computation; per-parent result charges release before the next parent; graphs remain borrowed. |
| Publication and native ownership | Declared/empty/duplicate outputs; first write/close failure and poisoned attempt; pending versus submitted state; cancellation before/after submit; complete batch/writer charge retained until terminality; receiver disappearance and late error survive drainage. |
| CU5 only if adopted | Compare small versus threshold/oversized rows, duplicates crossing flushes, conflicting rows, reservation refusal, failure after partial admission and cancellation with an outstanding transfer. No synchronous bridge fallback or uncharged batch clone. |
| Compiler composition | Inspect parent, loader/emitter children and drop bodies in normal/harness products across CGUs. Use comparable pinned settings, inventory membership and source identities; keep partial/filtered artifacts explicit. No fixed instruction-count cutoff substitutes for execution fit. |

Current grouped routes include:

```sh
cargo check --locked -p cpg-core -p lctx-model --tests
just verify --print --select compiler:producer --nextest-args='--test compiler_analysis -E test(structural::)'
just verify --print --select compiler:producer --nextest-args='--test compiler_artifacts'
just verify --print --select compiler:producer --nextest-args='--test compiler_synthesis'
just verify --print --select compiler:producer --nextest-args='--test compiler_catalog'
```

During correction use the existing named candidate explicitly where its controls apply; until BC3
qualifies and installs the default, unqualified verification still resolves release. Actual native
checks use owned fixtures and readiness repairs from their run owner. A missing user systemd bus is
**blocked** with the session-readiness repair, not a test failure; do not synchronize a live native
environment. Pure model/kernel controls need no database. Preserve normal available parallelism.

At completion select affected stage controls and non-functional leaves once, then coordinate BC5's
release-resolved assembled acceptance under the existing shared transport/identity rule. PC6 may
consume the same final-source optimized receipt only for matching cases/contracts/profiles. Neither
coordinator inherits acceptance from a differently profiled or older-tree run. No real-library or
operator activation follows from this plan.

Architectural closure needs the intended shared composition, migrated consumers, justified candidate
exclusions and actual compiling/runtime fit. Parent-only reductions, passing adapter-count tests or
source review alone do not close F01. Quantitative compile-time/throughput/memory claims require
explicit equivalent workloads and complete evidence. A separate broad performance campaign is not
required to correct the architecture; unfinished captures remain partial.

## 7. Source obligation coverage and current handoff

**Current continuation, 2026-10-08:** the operator committed/pushed the preceding PC/BC/CU implementation as `dcb505d5a8c8c4760893f4d49b459b3d0bfce69d`. The [persisted coordinator §9.1](persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07) now owns follow-up execution receipts for native cause attribution, bounded selection/eligibility, cold transport and repaired verification prerequisites. The four300s Behavioral controls remain failed/open; GK/GR now schedule the replacement structural correction and final-source acceptance. CU6 runtime acceptance, BC3 installation and BC5 release acceptance remain open. Earlier receipts below retain their original source/date and are not promoted to current-source acceptance.

Both populated native journeys and the 35 dependent Python controls passed in that coordinator's `20261009T023103.223Z-bda1ff` receipt. Their 31m23s native execution prompted the [populated journey target review](../design_review/reviews/design_review_populated-journey-execution-amplification_2026-10-08.md). The [populated companion](populated-journey-execution-amplification-plan_2026-10-09.md) now develops its Proposed physical lowering/preparation; the persisted coordinator owns scheduled findings. PJ2 must retain coarse shared future/stream boundaries and synchronous typed leaves rather than recreating vocabulary-wide generic async dispatch. PJ1/PJ3/PJ5 preserve owned kernels, retained read setup and acknowledged scratch/cursor drainage; runtime topology observations are distinct from compiler-cost evidence. CU6 can consume valid matching-source/profile acceptance at that coordinator, but this earlier successful boundary does not close deferred CU6/BC/PC obligations.

| Review reference | Route in this plan | Current disposition owner |
|---|---|---|
| F01 loader composition | §3.1; CU0–CU4; exact-prefix/order/failure controls | Coordinator §9 F01 |
| F01 phase/child/drop composition | §3.2; CU1–CU4/CU6; whole-operation compiler inspection | Coordinator §9 F01 |
| F01 output composition | §3.2/§5; CU5; existing output contract retained unless justified refinement | Coordinator §9 F01 |
| F01 ownership | §3.2/§6; every migration and CU6 native terminal controls | Coordinator §9 F01 |
| Recommendations and legitimate countercases (§6–§8) | §2–§6; borrowed phases/shared loops, pure kernels, coarse erasure and valid overlap/profile/failure cases | Coordinator §9 F01; no new finding |
| Five remaining investigations (§10) | §5 explicit evidence/branches; CU0–CU6 | Coordinator §8/§9 |
| Positive counterexamples and F02–F04 limits | §2/§3.3; leave semantic leaves, graphs, fingerprint/harness/profile owners intact | Existing coordinator rows |
| Rule impacts (§11) | §1: none; no policy-dependent implementation | Existing decisions unchanged |

**Execution checkpoint, 2026-10-08:** CU0–CU4 source changes are integrated in the preserved dirty main tree, starting from `237341701e9b06359e6148df17dac27d98c115ee`. The exact pre-execution file/index snapshot is `/tmp/library-context-cu-execution-baseline_2026-10-08.json`; task-only source deltas are retained in `/tmp/library-context-cu-task-only.patch`. Existing PC/BC changes remain part of the compilation baseline and have not been reset or claimed as newly accepted.

Structural loading, declarations, selected-grain acquisition, pure computation, result/link/coverage publication and final release now have coherent borrowed boundaries. The shared frame reader preserves every matching completed prefix. Summary/model, selection/synthesis/retrieval, analytic, semantic execution and normalization inventories now derive ordered opaque adapters from their existing declarations. Normalization’s constructors deduplicate nominal facts inputs; event/binding readers retain first-match behavior. Additional source-confirmed children include C0 inventory/publication, synthesis documentary preparation, BaseScopes/CompletionScopes, receiver/callable loaders and callable admission, local-analysis loading/publication and native premise inventory. Their parents and actual children are corrected together.

**Bounded exclusions:** publisher search, serving source evidence and native loader already have bounded erased/shared traversal; SourceCallScopes data/enriched data use uniform physical table/batch loops with synchronous visitors. Graph preparation, nominal codecs/derives and pure kernels retain their synchronous meaning. Revisit an exclusion only on source-confirmed inventory-await expansion or attributed material compiler cost; raw MIR size alone does not establish that cause.

**CU5 decision: retain current `push`.** Ordered typed emission adapters contain publication composition while preserving existing validation, registration, cross-flush deduplication, poisoning, backpressure and charged native completion. No attributed residual output cost currently warrants a checked bulk API. Revisit only if a matched complete capture identifies material output composition and a contract comparison preserves those semantics. No profile, dependency, runtime limit, available parallelism or public contract changed.

**Independent source review, 2026-10-08: no material findings** across 24 source files compared with the exact pre-execution baseline. Reviewed composite SHA-256 is `05664db988cac05cc6da26565a21864d4a77fc829ae07457429ca58d01938769` (sorted path + NUL + file SHA-256 + newline). It includes scoped children and local/native recurrence; source review is not native or compiler acceptance.

**passed (2026-10-08):** final `cargo check --locked -p cpg-core -p lctx-model --tests` (2.34s), following successful structural and integration checks. One earlier integration check failed on test qualification and a macro-local name shadow; both were repaired before the successful reruns. Existing third-party future-incompatibility warning remains.

**Retained compiler runs:** operator-authorized cancellation of old native build `20261008T202337.170Z-114602` completed at22:28:43UTC without a test verdict. First CU capture `20261008T224204.565Z-5a9f8e` completed the normal library (125.42s compiler wall time), then filesystem exhaustion interrupted the harness and remaining targets; closed derived reports and provenance remain retained. The operator authorized obsolete raw retirement on2026-10-10 through [storage plan §10](storage-lifecycle-management-plan_2026-10-09.md#10-current-checkpoint); the dated compiler observations remain historical diagnostics. This is a failed/interrupted instrumented build, not failed native assertions. The operator freed storage before the replacement capture. Its source-supported nested loader corrections were integrated before the next build.

**Compiler acceptance: passed, 2026-10-08.** Replacement actual-settings capture `20261008T225117.998Z-e84d9b` completed normal/harness and seven affected integration products, exit0 in2m51s, using pinned `local-test-candidate` settings and available parallelism. Normal/harness rustc totals were64.622s/169.589s. Exact-reader reports and six closed perf chunks passed. The retained run-local assessment `build/runs/20261008T225117.998Z-e84d9b/coroutine-final-capture-assessment.md` includes29 explicit phase/child/drop match rules per product. Base/completion data-driver mono estimates8707→169 and4745→113 accompany typed leaves of60; inventories/settings are unchanged. Largest retained individual InstCombine event was0.352s, without an attributed replacement runaway. These are **Measured compiler diagnostics**, not LLVM instruction/PHI counts or whole-build speed measurements: incremental CGU workloads differ, timelines retain ≥1ms events and durations overlap. Exact pipeline/transformed-PHI investigation remains triggered by a material residual or compiler-defect claim.

**Runtime acceptance: focused passed; release qualification has failures.** Run `20261008T225434.109Z-fe57d0` passed13 actual candidate controls across lib/synthesis on an owned native fixture: exact prefix/error/laziness, selected call scope, produced-input authority and documentary publication. Command: `just verify --select compiler:producer --cargo-profile local-test-candidate --nextest-args='--lib --test compiler_synthesis -E "test(frame_reader_preserves_exact_prefix_order_failure_and_unpolled_charge) | test(input_phase_is_lazy) | test(documentary_publication) | test(unpolled_documentary) | test(stream_adapter_refuses) | test(call_scope) | test(produced_authority)"'`. Native preparation `just sync native` passed before full release `just qualify` run `20261008T225907.834Z-dd7ee2`. The frozen release run failed model10 stale contracts; providers extraction44passed/1failed/22timed out; compiler producer153passed/5failed/168timed out; CLI16passed/2failed; store87passed/3failed/1timed out; serving Rust failed compilation on an ambiguous resource import. Analytics and flow passed. Both native MCP journeys subsequently failed their first representative member-selection tool request with sanitized `Unavailable`. The assembled run completed at00:41:09UTC with failures. Additional failures are flow-oracle CLI execution; tooling Python missing the serving content/configuration that the failed MCP boundary did not produce; Clippy; Ruff; and dependency-policy discovery. Doc contracts, lint-agents, ADR metadata, fixture registration, gold, rules scan/test, Python types and docs publication passed. The root preserves this mixed-source/composite boundary rather than promoting later focused repairs to a clean assembled pass. This is not clean assembled acceptance. BC3 default installation and BC5 release acceptance stay with the coordinator; PC6 retains distinct final-source/profile matching requirements.

**Qualification corrections integrated, source-reviewed / focused tested, 2026-10-08:** the ongoing MCP test retained its previously built binary while the reviewed corrections were integrated at the recorded source cutover. `build/runs/20261008T225907.834Z-dd7ee2/qualification-source-boundaries.json` preserves old hashes and patch identities; completed boundaries and later source builds are not one matching-source acceptance receipt. The corrections repair stale nominal/proof/coverage fixtures, await actual provider-thread join acknowledgement, distinguish structural initial/scoped decoder inventories, restrict synthesized Catalog premises to the supplied model, and move original-byte evidence to a real private full-driver provider wrapper. Fixture profiles become separate independent tests without losing either profile or changing timeout/concurrency settings. The current explicit native artifact CLI contract and serving resource import are corrected in their controls. Eleven fixture/private-driver paths were independently re-reviewed after a JSON macro compile defect was fixed; the twelfth CLI parser control and separate serving import received root review.

The unchanged4MiB selected-body control exposed redundant DataFusion sorting and overlapping native/Arrow/decoded-row reservations, not host RAM exhaustion. The reviewed three-module correction advertises only the native projected `id` order, borrows byte/text payloads, reconciles ended scratch against actual live Arrow/builder/row ownership, and admits constructor/replacement-offset allocations before growth. Independent review closed both allocation findings; patch SHA-256 `80caa700a18229af71c021a1db959b9c7ae6ff7940762591567fe2314b892eb2`. Its local controls and the unchanged actual native control **passed** in the integrated focused release run below. No schema, small-budget refusal, public resource API or codec semantics are relaxed.

Two store controls failed during cache schema installation, before their intended conflict/acknowledgement behavior. The separate reviewed correction retries the idempotent declaration inventory only when **every** returned statement error is a source-confirmed definite conflict, using the existing finite8-attempt classifier/limit; mixed, permanent, unknown and uncertain errors refuse replay. Its three controls, actual conflict/acknowledgement cases and retained-primary oversized-stream assertion **passed** in the integrated store selection below; SHA-256 `97dabd7046b62cb49afb4995f184a090d415570c2e96e3733010a2a67e6441f4`.

**Operator resource correction implemented / source-reviewed / compile checked:** ordinary Workspace defaults, compiler/MCP fixtures and source-corpus captures share the existing64GiB admission ceiling and DataFusion55.1 automatic available-CPU partition target, replacing ordinary256MiB/1GiB and fixed partition overrides. Transfer batching and intentional small-budget/partition-invariance controls remain explicit. The corpus’s generic4GiB capture allowance was also replaced by the shared default; `cargo check --locked -p cpg-core --test fixture_corpus` passed0.52s after that final test-only change. This physical partition target is not a thread-pool cap or a speed claim; allowance does not allocate64GiB upfront. SHA-256 `ce74252bfdf48ccb09f8016c24ac6a5c3050f37019b311385e9aa049bf55755d`. The original MCP binary retained its older settings; integrated-source checks exercise rebuilt products. Host observations showed about109GiB available; the disposable server/client used less than1GiB each, with no observed server-limit exhaustion.

**Integrated verification (2026-10-08 local; run IDs use UTC2026-10-09):** `cargo check --locked -p cpg-core -p lctx-model -p cpg-extract -p lctx-surrealdb -p lctx-serving -p lctx --tests` **passed**, run `20261009T002030.922Z-d8b484`, after a test-only DataFusion55.1 downcast API mismatch in the first check was corrected. Focused release run `20261009T002147.055Z-2d8264` passed17 compiler and12 store controls, including real full-driver original-byte transport, exact structural declarations, call-scope/input authority, documentary/lazy contracts, the unchanged4MiB unrelated-body isolation oracle, projected native ordering and allocation refusal, finite definite installation conflicts and primary stream-error retention. Its first model boundary passed756/757 and exposed one remaining stale C2 qualification expectation; production C2 consumes admitted C1 links, not Local qualifications. The consumer-specific test correction retains shared decoder separation assertions. Actual `just verify --select model:rust` rerun `20261009T003224.148Z-c20c03` **passed**757 tests. These are repaired, bounded receipts, not a clean first assembled pass.

**Native codec/cancellation passed:** `just fixture -- cargo nextest run --locked --cargo-profile release -p cpg-extract -p lctx-surrealdb --test bundle --test native --no-fail-fast -E 'test(dropping_the_future_drains_native_work_and_refuses_completion) | test(native_codec_graph_search_and_immutable_winners) | test(native_binary_backed_text_matches_closed_schema)'`, run `20261009T003224.316Z-18d0b2`, passed3 actual controls. The cancellation control waits for real native thread join acknowledgement, preserving zero retained charge and refusal to complete; codecs use nominal records and actual native graph hydration rather than legacy integer payloads.

**Native member-selection correction: implemented / independently reviewed / focused passed.** The direct native260-member/window control failed with `Codec("invalid type: sequence, expected string or map")`, run `20261009T003420.522Z-3051b8`; its companion channel/context/cap control passed. Exact SDK3.3 source shows `SerdeWrapper<Vec<Value>>` treating nominal input arrays as serde enum encodings. `NativeReader::query_native<T: SurrealValue>` now uses native conversion; both query APIs share the same checked response/terminal/result execution helper. The sole SDK-value selection caller migrates to it. No JSON adaptation, independent query driver, schema or trust relaxation is introduced. Independent review of patch `be865fe571d140bfd84fba2388b790e4327714d52bf7a3f8f24e8ae1c31200dd` found no material findings. `cargo check --locked -p lctx-surrealdb -p lctx-serving --tests` passed (`20261009T003933.831Z-03c9d1`); the unchanged actual native-search target rerun passed both controls in5.496s (`20261009T003934.014Z-d90abb`). This identifies a concrete bug on the path shared by the MCP failures; a full corrected-source MCP journey is **not_run**, so PC6 remains open.

**Latest compiler diagnostic passed:** `20261009T003236.093Z-dfb25d` completed the same normal/harness/seven integration capture command,4m21s; exact readers and14 closed perf reports passed. Normal/harness rustc totals95.578s/161.839s and mono estimates BaseScopes169/CompletionScopes113 show retained containment. The29 phase/child/drop rules and broad inventories identify a largest harness InstCombine event of1.149s in `complete_base`, without renewed loader expansion. Source/model changes, incremental reuse and173/228 CGUs differ; these remain **Measured diagnostics**, not equivalent-workload speed comparisons. The run-local `coroutine-integrated-capture-assessment.md/json` owns raw hashes, matching rules and limitations. Two subsequent needless-borrow lint fixes and the later native-query correction are outside its exact source identity. Latest six-crate all-test `cargo check` passed in5.54s, run `20261009T004730.639Z-63cf42`, after those changes.

**Required native stage acceptance failed:** `20261009T003521.954Z-c00be9` selected four actual Behavioral model, summary, local-transfer and completion controls across compiler_behavior/compiler_artifacts. All four hit unchanged300s timeouts after a2m01s release build. The attempted normalization filters were in compiler_catalog and therefore were not selected; they remain **not_run** on this correction cutover. No job/thread limits, higher oracle budgets or relaxed timeouts were used. CU6 operational acceptance remains open with PC6; compiler diagnostics and source-preserving boundaries do not replace these controls. **Operator deferral, 2026-10-08:** leave the timed-out tests for a later focused review. No further timeout-driven run/review is scheduled in this continuation. CU6/PC6 acceptance remains open. When reactivated, identify the stalled native phase and its actual statement/error using owned state and the PC3 schema/VALUE investigation, then rerun affected stage boundaries and final matching-source acceptance.

**Remaining native investigation:** the first CLI Catalog/Facts case refused a closed/failed native owner with remote uncertainty; generic `ModelError::Conflict` display does not establish a payload collision. The initiating unconfirmed/unfinished lease is not identified by that log. Read-only candidate/window inspection found no proven repeated-window/nonprogress bug. Native schema/VALUE amplification and first uncertainty attribution remain with persisted-correction owners; no compiler defect or measured runtime cause is inferred. Run-local assessment is `build/runs/20261008T225907.834Z-dd7ee2/native-runtime-assessment.md`.

**Applicable leaves / preservation, 2026-10-08:** after repairing the two CU-introduced needless borrows, affected keep-going Clippy rerun `20261009T004730.853Z-6da023` still failed on unchanged baseline14-argument facts/availability interfaces and loader item placement. The original model-test initializer, Python formatting and dependency-discovery defects remain outside the CU correction; `cargo shear` reports the core build dependencies as unused despite `build.rs` including the shared producer-fingerprint helper that uses them. Do not remove those dependencies merely to satisfy that false-positive report. No rule was suppressed. Current `just docs-check` **passed**346 canonical pages/zero link errors; source diff checks passed. Exact baseline preservation audit found no changed unowned files, unchanged HEAD/index and54 owned paths. Scoped `just turn-end --paths …` is the final maintenance step; compiler/test receipts precede this formatting, and subsequent native readiness/final-source acceptance must use the formatted source. No task commit was made across the preserved uncommitted PC/BC baseline.

**Prior authoring receipt (2026-10-08):** the independent target review accepts the revised Proposed architecture, including explicit model application/publication coverage. `just docs-check` passed346 pages; four grouped selector resolutions and preservation checks passed. Those authoring checks are not re-reported as execution checks. Source F01 remains open at coordinator §9 until its operational/compiler obligations are met.
