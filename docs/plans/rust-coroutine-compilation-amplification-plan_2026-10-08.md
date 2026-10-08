# Rust coroutine containment: checked loaders and coherent producer phases

**Proposed target / authoring · 2026-10-08.** This document schedules corrections, not implementation or native acceptance.

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

**Rule impacts: none.** No pin, profile, runtime limit, available compiler/test parallelism,
semantic authority or native lifecycle change is selected. Existing ADR-0135/0137 decisions stay
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

Generate ordered typed adapter entries directly from each existing semantic inventory. A typed
adapter is an ordinary function returning a borrowed opaque future before the broad driver owns
it. Its record-specific work acquires the exact permit and supplies the synchronous decode/visit
callback. A shared non-generic loop owns sequential traversal and awaits uniform futures.

Keep scope policy local: frame, summary, model, selection, synthesis and retrieval owners decide
which declaration and SQL apply. Reuse uniform traversal only where contracts agree. No independent
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

## 4. Execution packages and concrete dependencies

All packages are **planned / not_run** for production implementation. The root owns shared reader,
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

Commands here are **planned / not_run** for implementation. Resolve current selectors with `just
verify --print` before executing; use compile checks and minimal revealing controls during each
package. Do not launch competing diagnostic builds during this document-authoring scope.

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

**Authoring checkpoint, 2026-10-08:** the target and packages above are Proposed. Production
implementation, new compiler probes and product verification are **not_run** in this document
scope. The existing actual native run `20261008T202337.170Z-114602` remains running and was inspected
read-only; its outcome is not replaced by this plan. Source/configuration baseline preservation,
independent target review and documentation publication checks are recorded at scope completion.

The [independent design/target review](../design_review/reviews/design_review_rust-coroutine-compilation-amplification-plan_2026-10-08.md)
**accepts the revised Proposed target within scope**: A1–A4 satisfied; no blocking target findings
or rule impacts remain. Target-review F01 prompted explicit model application/publication coverage
in CU2 and was independently re-inspected. Its current authoring disposition belongs to the
compilation coordinator §9; source F01 remains open. Reviewed document hashes identify the target
snapshot before these publication/checkpoint links, not a claim of production implementation.

**Authoring checks, 2026-10-08: passed** `just docs-check` (346 canonical pages, zero offline link
errors), scoped `git diff --check` and four `just verify --print` command resolutions for
`compiler_analysis`/structural, `compiler_artifacts`, `compiler_synthesis` and `compiler_catalog`.
Those static commands build nothing and start no fixture; they resolve the unchanged release
default. Their native readiness observation is **blocked** by this shell's missing user systemd
bus, with the login-session repair printed; no readiness repair or live-run intervention occurred.
**not_run:** production changes, new compiler/profile probes, product tests or assembled acceptance.
The published target review is byte-identical to the independent scratch
(`0f01815352dd2a2dab163127fe982003c6dfd186f5b8993b5860ab736f5a9f13`).
Hash comparison preserves all282 pre-existing non-owned dirty/untracked files. The two already
dirty documentation owners retain their earlier edits; only this task's navigation/dependency
hunks are committed. Next: CU0 execution refresh and CU1 structural/shared-reader implementation,
with its explicit focused acceptance boundary and no assumption that source F01 is closed.
