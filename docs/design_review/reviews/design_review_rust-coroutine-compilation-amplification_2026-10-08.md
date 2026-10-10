# Design review: Rust coroutine compilation amplification

**Design tier · Target purpose · 2026-10-08**

The current producer execution architecture **needs revision**. Its semantic model and checked ownership boundaries are appropriate for the inspected structural and C1 operations, but large inventories of concrete async calls accumulate inside individual coroutines. This makes semantic growth expand compiler control flow and drop machinery far beyond the small orchestration responsibility of those coroutines.

The preferred direction is to retain typed records and pure model kernels, place coherent load and publication operations behind borrowed opaque future boundaries, and dispatch repeated typed loader operations through inventory-derived adapters and shared loops. This must contain the children as well as reduce the parent. It does not require a new task scheduler, a schema-wide runtime model, or boxing every function.

C1 provides encouraging **Measured partial diagnostic** evidence for that direction, including its new helpers. It does not establish whole-build improvement, runtime performance, or native acceptance. The structural reproducer demonstrates severe expansion under a simplified LLVM pipeline; the exact cause of the actual rustc PHI tail remains unresolved.

## 1. Review contract, baseline and coverage

| Field | Scope |
|---|---|
| Reviewer | Independent non-author reviewer, runtime-configured GPT-6.1-sol/high. The coordinator reused the existing reviewer context after fresh-agent creation reached the runtime thread limit. Earlier correctness findings were reconsidered rather than treated as architectural proof. |
| Standard | Core principles and template **3.3**, efficient-architecture heuristics **1.0**, code-intelligence profile and review additions **1.5**, and the library-context binding. |
| Subject | Current dirty tree at `9bd1360c48b59b5fea34ead38e89ee7b203b31b8`, including preserved BC/PC changes and the C1 correction. The retained [baseline snapshot](../../../build/runs/20261008T205144.595Z-adb189/core-ir/assessment/library-context-coroutine-review-baseline_2026-10-08.json) records the coordinator's 286 dirty-path hashes. |
| Functional target | Compile pinned Python library facts into an API and evidence catalog, with explicit catalog/behavioral profiles, exact completed input views, programmatic derivation, and truthful support and coverage. |
| Workload premise | Ordinary development and optimized acceptance must compile the supported Rust workspace. Runtime producers consume bounded typed batches, reuse prepared selected closures and graphs, and retain charged work through cancellation and native terminality. Semantic inventories will grow. |
| Decisive source inspected | Structural orchestration, structural model build/frame operations, analytical scopes, prepared graphs, consumed-row streaming, expected coverage admission, C1 adapters/phases, shared output buffering, and native submission/close ownership. |
| Recurrence coverage | Source inspection of summary/model scopes, synthesis loading/production and retrieval preparation; workspace pattern mapping identified selection and analytic producers and additional execution scopes. Model derives, representative adjacent transfer/read interfaces and fingerprint source were also inspected. This is broad triage, not a complete review of all Rust files. |
| Exclusions | Serving journeys, complete extractor/provider qualification, all graph algorithms, gold/evaluation design, operator activation, and whole-workspace architectural certification. |
| Method | Read-only source, interfaces and retained evidence. **not_run:** builds, tests, compiler passes, probes, formatting and process intervention by this reviewer. The only reviewer write is this authorized scratch document for coordinator publication. |

The governing product and semantic owners are [DESIGN](../../design/DESIGN.md), [analytics](../../design/sections/analytics.md) and [semantic model §15](../../design/sections/semantic-model.md). Current execution disposition belongs to [compilation-cost plan §9](../../plans/rust-compilation-costs-plan_2026-10-08.md#9-sole-compilation-cost-finding-disposition), not this dated review.

## 2. Responsibilities and domain authority

The architecture already has useful separation:

| Owner | Responsibility and consumer contract | Appropriate reason for change |
|---|---|---|
| Model records, declarations and codecs | Nominal relation identity, fields, keys, validation and mechanical transfer representations | A changed semantic fact or representation contract |
| Structural model operations | Define methods/settings, validate frames and projection identities, derive structural results and outcomes | A changed analysis meaning, abstraction or policy |
| Core structural producer | Acquire exact inputs, prepare scopes, arrange computation and publish admitted results | A changed acquisition or execution mechanism |
| Consumed-row and analytical-scope operations | Check completed-view permits, select exact declared inputs and stream selected batches | A changed checked access or physical selection mechanism |
| Prepared graphs | Reuse topology bound to the admitted collection and projection key | A changed preparation or graph realization |
| Producer output and native owners | Buffer typed rows, form charged batches, submit effects, drain and complete contributions | A changed transfer, admission or lifecycle mechanism |

These distinctions govern executing behavior, rather than appearing only in output type names.

For example, `build::Data::consumed_inputs` retains profile-sensitive native premises and deduplicates declarations by **name and prefix**, while `visit` rejects vocabulary consumption without an explicit completed view. `build::produce` validates configuration, invocation and both projection identities before computation. `frames::frame` requires canonical definitions and parameters and independently selects all four structural invocations. See [structural build](../../../crates/lctx-model/src/domain/structural/build.rs), lines 75–157, and [structural frames](../../../crates/lctx-model/src/domain/structural/frames.rs), lines 78–168.

### Fact and fidelity boundary

| Family | Fidelity and governing operation | Coverage, identity and consumers |
|---|---|---|
| Provider facts and native premises | Attributed provider assertions; structural consumers use the selected completed vocabulary and nominal premises | Producer/profile/model/view identity remains attached. Structural compilation does not promote unsupported native assignment into completeness. |
| Normalized projections and catalog bindings | Derived resolution and declared relationship projections | Projection identity, input/context and source assessments govern prepared graph use. Output selection does not redefine the graph universe. |
| Structural results | Programmatically derived under configured Delegation, DirectUsage, Handoffs and Controls methods | Definitions, parameters, invocations, qualification, conditions and assumptions remain explicit. Bounded or unresolved evidence retains its status. |
| Coverage and outcomes | Independently admitted expected domains and observed sources, then model-owned assessment | Missing candidates, `NotRequested`, incomplete work and successful absence remain distinguishable. |
| C1 evidence | Derived from exact selected premises and linked to admitted invocations | Source receipts and invocation links survive phase separation; Arrow batches do not become publication authority. |

**A2 assessment:** the inspected model is adequate for these operations and authoritatively realized. No model replacement is justified by the compiler symptom.

## 3. Operation contracts that constrain the remedy

The important contracts are more precise than “load, compute, write.”

* **Inventory acquisition:** consume every declared matching type/prefix through its exact completed-view permit, preserve established loader ordering, stop at the first error, and reject unhandled declarations at `ConsumedInputs::finish`.
* **Scoped acquisition:** select the prepared grain closure before decoding. A record type can have multiple legitimate prefix declarations; every matching declaration remains eligible. `FrameScopes::read<R>` currently owns this loop, permit acquisition and nested stream await. See [analytical scopes](../../../crates/cpg-core/src/analytical_scopes.rs), lines 741–764.
* **Structural computation:** validate the frame and graph keys, derive results under authored settings, preserve relationship kind and conditional meaning, and report limits and unknowns explicitly.
* **Coverage admission:** check a permit only when the relation is handled, before decoding or mutating the frontier. Moving all checks to an unconditional outer pass changes legitimate skipped-relation behavior. See [expected coverage](../../../crates/lctx-model/src/domain/analysis/expected.rs), lines 505 onward.
* **Publication:** validate rows, preserve buffering and failure behavior, transfer a bounded charged `Batch<R>`, and complete only through the existing contribution lifecycle.
* **Cancellation:** caller cancellation stops further admission and fails the attempt; already submitted native operations remain registered through terminality, including late failures and charged arguments.

`Batch<R>` owns rows, Arrow representation and reservation together. Its construction validates, sorts and deduplicates by record identity, rejects conflicting duplicates, and admits encoding memory before allocation. An Arrow `RecordBatch` alone does not provide this contract. See [record batches](../../../crates/lctx-model/src/domain/record.rs), lines 605–667.

Prepared graph access checks collection admission and exact projection identity. The borrowed CPU route keeps graph/data lifetimes in the caller and uses `block_in_place` only on a multithread runtime, otherwise running inline. These are positive counterexamples to indiscriminate spawning or type erasure. See [prepared graphs](../../../crates/cpg-core/src/analysis_graphs.rs), lines 190–218, and [stage runtime](../../../crates/cpg-core/src/stage_runtime.rs).

## 4. Physical composition and observed amplification

The structural producer performs several coherent operations inside one concrete async body:

1. Capture source identity and coverage admission; load inventory metadata.
2. Declare publication relations and prepare exact structural scopes.
3. For each parent, load the selected grain and derive configured invocation/frame state.
4. Run pure structural computation using prepared graphs.
5. Emit result relations, receipts, projections, coverage and outcomes.
6. Drop grain/frame owners and finish the producer.

The implementation expands inventory, declaration, scoped-read and write macros into separate concrete awaits inside that body. See [structural producer](../../../crates/cpg-core/src/structural.rs), lines 114–147 and 234–308. The generic `load<R>` additionally contains admission selection and streaming awaits, lines 48–81. Beneath scoped loading, `FrameScopes::read<R>` still repeats a generic declaration loop and nested await.

The issue is therefore not simply that a source function is long. Compile-time variation from record inventories, callbacks, concrete child futures and typed output handling enters the parent's state and cleanup graph. More relations increase this machinery even where runtime sequencing is merely “call the next checked loader.” Boxing only the outer producer preserves the large concrete implementation inside the box.

### What the retained evidence establishes

| Evidence, dated 2026-10-08 | Observation | Limit |
|---|---|---|
| [C1 comparison](../../../build/runs/20261008T200802.033Z-6761de/compile-profile-reports/assessment/correction-comparison.json) and [plan §12](../../plans/rust-compilation-costs-plan_2026-10-08.md#12-interrupted-capture-assessment-and-remaining-correction) | Parent MIR estimate changed from 25,100 to 591. Retained completed C1-related InstCombine events changed from 350.325s/353.164s to 0.185s/0.266s for normal/harness units. | Interrupted instrumented captures; ≥1ms event filtering omits short events. These are not whole-build durations or runtime results. |
| Same comparison, new children included | Normal `publish_frames` accounts for 0.096397s and `emit_artifact` for 0.070005s across their retained passes; harness values are 0.101150s and 0.088992s. | Helper-inclusive evidence is stronger than parent shrinkage alone, but does not establish every helper's total unfiltered cost or acceptance. |
| [Saved IR inspection](../../../build/runs/20261008T205144.595Z-adb189/core-ir/review-ir-stats.json) | Structural parent: 7,949 blocks, 37,772 instructions, **zero pre-optimization PHIs**. C1 parent in this CGU: 456 blocks, 2,191 instructions. | Selected raw functions, not whole-crate costs; C1 children in other CGUs must not be treated as absent. |
| [Structural reproducer assessment](../../../build/runs/20261008T205144.595Z-adb189/core-ir/assessment/structural-reproducer-assessment.json) | Simplified `default<O1>` processing reaches the named structural coroutine at 2,250,997 instructions; a five-second sample records 545 samples with 99.08% self CPU in `visitPHINode`. | `target_machine=NULL` differs from rustc defaults. Transformed inner PHI structure was not retained. This does not prove the exact actual rustc pass or inner scan. |
| [Actual control sample](../../../build/runs/20261008T202337.170Z-114602/assessment/uninstrumented-self.txt) | Normal and harness optimizer threads together account for about 99% sampled self CPU in `visitPHINode`. | A native execution-path observation, not exact source-function attribution. |

At the coordinator's 21:38 UTC checkpoint, the actual control remained compiling `cpg-core`; no tests had started. Its result was **not yet available**. No full compile-time improvement is claimed.

The raw IR has no PHIs, so raw join counts cannot establish the expensive transformed PHI shape. A compiler pathology may contribute. Nevertheless, the repeated concrete coroutine composition is an architectural susceptibility supported by source and reproduced expansion; there is a credible smaller execution representation that preserves the same operations.

### Workspace coverage and anticipation

The workspace map searched patterns across 717 Rust files and supplied specific candidate owners. Decisive source and retained artifacts were inspected by the principal reviewer; mapping is not a claim that every listed module was reviewed completely. The following raw functions from the [saved IR inspection](../../../build/runs/20261008T205144.595Z-adb189/core-ir/review-ir-stats.json) are **candidate sizes, not a cost ranking**:

| Pre-optimization function | Blocks | Instructions | Review significance |
|---|---:|---:|---|
| `SummaryScopes::load` | 69,669 | 307,728 | Inspected typed inventory/selected-input loading; recurrence candidate |
| `synthesis::produce` | 32,518 | 148,347 | Inspected declaration/emission and publication composition; recurrence candidate |
| `Preparation::prepare` in retrieval preparation | 28,849 | 132,350 | Inspected checked inventory acquisition; recurrence candidate |
| `semantic_summaries::produce` | 27,217 | 126,544 | Adjacent producer candidate; not separately timed |
| `ModelScopes::load` | 26,975 | 126,104 | Inspected typed selected-input loading; recurrence candidate |
| `SynthesisScopes::load` | 20,737 | 94,136 | Inspected acquisition loops and artifact selection; recurrence candidate |
| `SummaryScopes` drop implementation | 10,442 | 32,171 | Cleanup is part of the operation; parent-only correction is insufficient |

All these selected raw functions have zero pre-optimization PHIs. Their sizes justify directed inspection, not automatic migration. Source routes include [summary scopes](../../../crates/cpg-core/src/semantic_summaries/scope.rs), line 226; [model scopes](../../../crates/cpg-core/src/semantic_models/scope.rs), line 236; [synthesis](../../../crates/cpg-core/src/synthesis.rs), lines 407 and 596; and [retrieval preparation](../../../crates/cpg-core/src/retrieval_preparation.rs), line 44. Mapping additionally identified selection scopes and analytic production. This extends the recurrence search beyond C1 without claiming exact pass costs for those functions.

Model expansion is a different category. [Domain derives](../../../crates/lctx-model-macros/src/lib.rs), lines 212–225 and 441–499, enforce concrete records and generate actual key, digest, validation and codec operations; `DomainSum`, line 642 onward, lowers finite typed alternatives; `DomainCode`, lines 502–638, preserves explicitly numbered codebooks. These necessary concrete leaves and nominal `Batch<R>` have **no attributed PHI defect** in this review. Inventory growth can increase their legitimate work without justifying their replacement by a new runtime semantic authority.

Adjacent interfaces were mapped and spot-inspected: [publisher keyed search](../../../crates/lctx-publisher/src/search.rs), line 163, has typed bounded native reads; [serving source evidence](../../../crates/lctx-serving/src/source_evidence.rs), line 43, has async typed selection over prepared data; [native loading](../../../crates/lctx-surrealdb/src/loader.rs), line 72, awaits bounded canonical transfer windows. They are mapped candidates, **not defect verdicts**. In particular, an async signature over synchronous prepared selection is not evidence of the nested-await expansion found in structural loading. Serving journeys remain outside the architectural acceptance scope.

Grouped integration drivers and the normal-library/harness products are distinct compilation units. C1's evidence covers both; it does not eliminate their separate code generation. Existing compilation-cost F04 owns harness duplication and qualification, and is not reopened here. Likewise, the inspected [production fingerprint closure](../../../scripts/producer_fingerprint.rs), lines 24–77 and 119 onward, now follows the production owner and dependencies with conservative manifests/lockfile/patch inputs. The old all-workspace fingerprint diagnosis is not duplicated against this source. Cargo's re-resolved feature composition was not independently audited.

Graph composition is a positive counterexample. Structural computation passes borrowed materialized graphs to synchronous model operations, while prepared graphs separate selected async chunk acquisition from synchronous hydration. The domain's [borrowed native graph view](../../../crates/lctx-model/src/domain/projection/native.rs), line 38, keeps physical indices private and retains the materialization borrow and charge. [Reachability](../../../crates/lctx-model/src/domain/projection/snapshot.rs), line 191, already composes `Dfs`, `EdgeFiltered` and `Reversed`, traverses the complete intermediate universe before output selection, and returns sorted identities with its reservation. The library research checked petgraph **0.8.3** borrowed `GraphRef`/neighbor/edge iterator capabilities and [Dfs](https://docs.rs/petgraph/0.8.3/petgraph/visit/struct.Dfs.html). Allocation reuse through `Dfs::reset` could be considered for a matched traversal contract, but is not a concrete coroutine remedy. Domain delegation combines two graphs with canonical evidence, bounds and statuses; plain DFS does not replace that operation. No new graph adoption is indicated.

## 5. Finding

### <a id="F01"></a>F01 — Typed await inventories expand coherent producer operations into oversized coroutines

**Priority: P1. Owner: core producer/scoped-loader owners, with shared output ownership where required.**

**Principles and judgment:** FP-07 and FP-06; DP-08, DP-10, DP-16, DP-17, DP-20 and DP-22; **A4 violated**. No semantic gate failure is established.

**Consequence.** Adding supported semantic relations increases compile-time control-flow and cleanup machinery inside existing producer coroutines. The structural source exposes the same composition pattern that was costly in C1; the simplified structural reproducer demonstrates severe expansion. The actual optimized control remains compiling with the sampled related native hot path. This is disproportionate execution structure for orchestration that can be expressed through shared ordered loops and coherent borrowed phases.

**Grouped manifestations.**

* Structural inventory loading, declarations, scoped loading, result emission and coverage publication share one concrete coroutine.
* `load<R>` and `FrameScopes::read<R>` retain generic loops and nested awaits below that parent.
* Summary/model scopes, retrieval preparation and synthesis contain comparable macro-expanded loader or producer shapes. Saved pre-optimization sizes identify additional candidates, not measured individual hotspots.
* `ProducerOutput::push<R>` retains typed validation, mutex/downcast buffering, threshold handling and a concrete async body. It submits native work **only at a flush**, not once per row. It is a contributing boundary to inspect, not an independently established hotspot.

**Correction — Proposed.** Preserve the public typed contracts and nominal record operations. Introduce borrowed opaque boundaries for coherent acquisition and publication phases, with inventory-derived thin typed adapters dispatched through shared loops where the operation is uniform. Keep pure derivation synchronous and model-owned. Contain concrete await variation before it enters the broad phase driver.

The remedy has distinct closure obligations:

| Obligation | Required property |
|---|---|
| Loader composition | Generated adapters reuse existing semantic inventories. Ordered dispatch retains overlapping entries; exact type/prefix consumption and the existing owner determine whether work remains. |
| Phase composition | Inventory, scoped acquisition and publication children are individually assessed. A small parent cannot conceal a newly oversized child or drop implementation. |
| Output composition | Preserve pending-row order, thresholds, deduplication, poisoning, row-byte limits, backpressure and batch ownership. Do not replace async native transfer with a synchronous provider-bridge call. |
| Ownership | Preserve short grain lifetime, frame charge lifetime, retained admission/source state, native writer keepalive and terminal drain. No uncharged clones or detached work are introduced to satisfy a future signature. |

A checked bulk append operation could further reduce per-row async composition: typed synchronous preparation followed by bounded async transfer. This is **a conditional refinement requiring design of buffering/error/cancellation semantics**, not a mandatory second finding or an instruction to replace `push` immediately.

**Closure evidence.** Inspect the resulting parent, children and drop paths; show that inventory growth dispatches through the intended shared operation rather than re-expanding its async mechanics. Confirm with a retained comparable compiler capture under actual pinned settings. Use focused revealing controls for exact-prefix overlap, first-error behavior, poll-delayed failures, profile-sensitive loading, output failure and cancelled submitted writes. Native/runtime acceptance remains separately required; a parent MIR decrease alone cannot close the finding.

This finding extends the cause addressed by [original compilation-cost F01](design_review_rust-compilation-costs_2026-10-08.md#F01). Add this source reference and its distinct closure obligations to the existing [plan §9 disposition](../../plans/rust-compilation-costs-plan_2026-10-08.md#9-sole-compilation-cost-finding-disposition); do not create a competing status table.

## 6. Proposed architecture and legitimate countercases

The proposed driver awaits a small sequence of operations. Inventory-derived loader adapters preserve record-specific permit and decoding work, while a shared loop owns sequential traversal. Selected-frame computation continues to borrow prepared graphs and model data. Publication phases own their result rows and release them at the established boundary.

C1 already demonstrates this composition: ordered inventory groups, typed inventory/scoped adapters and bounded phase helpers. See [C1 loader groups](../../../crates/cpg-core/src/catalog_evidence.rs), lines 35–81, loaders/phases at 122–198, and artifact computation/emission at 320–355. Its measured helper-inclusive result supports reuse of the pattern; it does not mandate copying C1's exact phase boundaries into every stage.

The corrective design must handle these valid cases:

| Legitimate case | Required behavior and implication |
|---|---|
| The same relation appears at two prefixes or in overlapping inventories | Retain every exact declared view. Do not deduplicate by name or replace ordered consumption with a set of types. |
| Catalog profile omits behavioral native inputs | Preserve profile selection and explicit `NotRequested` parent/outcome semantics. Do not eagerly load a superset to simplify adapters. |
| Coverage visitor skips a relation | Skip the conditional guard as today. A handled relation must pass its guard before mutation. |
| The first loader fails | Return that failure before later loaders run. Preserve direct rejection and error precedence. |
| An operation is created but never polled | Do not move checks, preparation, mutation or task submission out of the existing lazy boundary without an intentional contract change. |
| A large selected graph has bounded results | Keep examined-work limits, stop evidence and partial outcomes. Smaller futures do not justify shrinking the traversal universe. |
| Caller cancellation follows native submission | Retain charged batch/writer arguments and registered joins through terminality; retain late failures. |
| A borrowed graph kernel runs | Keep caller ownership through completion. An opaque future boundary is not a reason to add `spawn_blocking` and `'static` clones. |
| Many small output rows | Avoid per-row boxes, task launches or new materialization solely to reduce compilation. Batch/phase boundaries are the useful granularity. |

The existing drops matter: structural grain selection is dropped before computation, frame result/context/data owners before the next parent, and scopes/admission/source owners before final completion. C1 drops frame premises before artifact scope construction and moves charged computation inputs into its existing worker. Shared native close keeps its writer alive inside the submitted operation. See [structural producer](../../../crates/cpg-core/src/structural.rs), lines 147 and 303–314, and [writer submission/close](../../../crates/cpg-core/src/workspace.rs), lines 1900–1920.

## 7. Change, growth and substitution scenarios

| Scenario and kind | Expected ownership and propagation | Assessment |
|---|---|---|
| Add a structural premise relation — domain extension | Model declaration/codec, consumed-input selection and the owning semantic visitor change. The loader adapter is mechanically derived from the inventory; orchestration sequencing remains shared. | Meaning changes legitimately affect several owners. Current macros additionally expand the containing async implementation, producing avoidable compiler propagation. |
| Add a structural method — domain concept | Method/parameters, definition, frame/coverage/outcome rules and actual kernel belong to model owners; producer composition acquires the new configured operation. | Some coordinated semantic migration is necessary. A generic executor DSL would obscure that responsibility; ordinary owned functions suffice. |
| Grow records and parent frames — input growth | Prepared scope/topology work remains reusable; selected grain data and output batches remain charged and bounded. | Preserve existing access paths and drop points. Do not solve compilation by collecting all parents' results or duplicating graphs. |
| Grow relation inventory — schema growth | Shared dispatch loops accommodate additional typed adapters without embedding each loader's async implementation in the parent. | This is the principal corrective scenario for F01. Inspect children as well as the driver. |
| Substitute acquisition mechanics — mechanism change | Exact completed-view permits and selected closure semantics remain the consumer contract; query/setup/stream implementation changes below it. | DataFusion streaming can remain the erased batch mechanism. The native complete view retains authority. |
| Substitute coroutine composition — execution mechanism | Borrowed phase futures replace embedded concrete await chains; public typed signatures and semantic results remain stable. | Credible route with existing futures capabilities. No new scheduling ownership is needed. |
| Cancel or fail during publication — failure scenario | Pending and submitted states remain distinct, and submitted operations drain through terminality. | A phase split must preserve the existing recovery route rather than create detached helpers. |

Pure model/kernel controls remain independently runnable without store setup. Checked streaming and native publication controls require their actual production boundaries and owned fixtures. Tests that simply count adapters or mirror generated inventories cannot replace semantic overlap, failure or ownership controls.

## 8. Library fit and alternatives

Interfaces were checked against the resolved local sources and exact versions. Library capabilities constrain the design; their availability does not establish a need for a new abstraction.

| Capability | Existing library fit | Choice and total-cost assessment |
|---|---|---|
| Borrowed opaque future | [futures 0.3.34 `BoxFuture`](https://docs.rs/futures/0.3.34/futures/future/type.BoxFuture.html) is a pinned boxed `Send` future with an explicit borrow lifetime. | Use at coherent phase/adapter boundaries. It does not require `'static`, cloning or spawning. Heap allocation and indirect polling warrant coarse granularity. |
| Task lifetime | [Tokio 1.53.2 `JoinHandle`](https://docs.rs/tokio/1.53.2/tokio/task/struct.JoinHandle.html) detaches on drop; started blocking tasks cannot be aborted. | Preserve registered native joins and charged ownership. Do not add tasks merely to separate future types. |
| Borrowed CPU work | [Tokio `block_in_place`](https://docs.rs/tokio/1.53.2/tokio/task/fn.block_in_place.html) fits the existing multithread borrowed-work route. | Keep existing placement and inline fallback; phase separation does not imply new CPU placement. |
| Batch query streaming | [DataFusion 55.1.0 `execute_stream`](https://docs.rs/datafusion/55.1.0/datafusion/dataframe/struct.DataFrame.html#method.execute_stream) already returns an erased record-batch stream. Dropping it aborts query execution. | Keep shared streaming mechanics. Query abort is distinct from native write drainage and does not establish canonical order. |
| Structural batch representation | [Arrow 59.3.0 `RecordBatch`](https://docs.rs/arrow-array/59.3.0/arrow_array/struct.RecordBatch.html) checks structural array/schema consistency. | Preserve model-owned nominal validation and charged `Batch<R>`; shared backing buffers do not make clones independently owned charges. |
| Graph computation | Existing petgraph 0.8.3 borrowed views, iterators, DFS adapters, prepared graphs and pure domain kernels | No graph-library replacement addresses coroutine amplification. Retain the semantic graph boundary and reuse mechanism described in §4. |

### Alternatives

| Alternative | Benefit | Cost or limit | Judgment |
|---|---|---|---|
| Current concrete macro-expanded coroutines | Direct typed source and no added dispatch abstraction | Inventory growth expands coroutine/drop control flow; pathological optimizer interaction remains | Revise |
| Box only the outer producer | Small caller-facing type | Leaves the concrete expanded implementation inside the box | Insufficient |
| Coherent borrowed phases plus shared inventory dispatch | Contains implementation variation while retaining typed leaves and current ownership | Adds modest adapter/phase allocation and indirect polling; child sizes and drop boundaries require inspection | Preferred direction |
| Uniform runtime visitor for the entire schema | Can centralize some decoding/dispatch | Risks replacing nominal semantics, reproducing declaration authority and introducing a broad framework | Not justified |
| Box or spawn each output row | Reduces some concrete caller composition | Adds allocation/scheduling and lifetime burden at record frequency | Reject for this purpose |
| Shared checked bulk append | May contain typed buffering and await variation at transfer granularity | Requires careful cross-flush ordering, deduplication, poisoning and cancellation design | Investigate only if residual evidence warrants |
| Crate/source-file splits | May improve coherent dependency boundaries | File movement alone leaves the future shape; crate splits add build/link boundaries and can relocate costs | Not the first remedy |
| Lower optimization, blanket `inline(never)` or compiler worker caps | May alter the immediate symptom | Does not settle supported execution fit; can weaken generated code or conceal composition defects | No recommendation |
| Toolchain change | Could remove a genuine optimizer regression | Changes another variable and needs actual-settings qualification | Revisit for a matched residual compiler defect |

The simplest viable alternative is ordinary borrowed phase functions and thin adapters using the existing inventories and libraries. There is no established missing generic library capability warranting a G8 failure.

## 9. Independent judgments and gates

### Architectural judgments

| Judgment | Verdict | Evidence and scope |
|---|---|---|
| A1 Localize change | **Satisfied, bounded** | Inspected producer/adapters separate model semantics, selection and effects. A semantic extension has identifiable owners. This does not certify every workspace stage. |
| A2 Encode domain meaning explicitly | **Satisfied, bounded** | Profile, exact view/prefix, frame, method/settings, projection, qualification, coverage and completion distinctions govern the inspected operations. |
| A3 Extend through composition | **Satisfied, bounded** | Typed model kernels, prepared graphs, checked streams and shared native effects compose through usable contracts. C1 adapters preserve those contracts. |
| A4 Fit execution to the supported workload | **Violated** | Concrete await inventories unnecessarily couple semantic-kind growth to large coroutine/cleanup representations; retained evidence demonstrates susceptibility to severe compiler amplification. F01 requires revision. |

FP-01 through FP-05 are **satisfied for the inspected responsibilities and semantic boundaries**. FP-06 is **satisfied for semantic ownership and bounded testing**, with coroutine complexity limiting execution reasoning as described in F01. FP-07 is **violated**. Applicable fidelity and lifecycle obligations under DP-01–DP-12, DP-18–DP-19 and CI-01–CI-08 are supported at the inspected boundary. DP-16/DP-17's economy and separation considerations contribute to F01; they do not justify erasing model-specific operations.

### Correctness and fidelity gates

These are static review verdicts, not test outcomes.

| Gate | Verdict | Basis and limit |
|---|---|---|
| G1 Authority | **Pass, scoped** | Model declarations and operations remain authoritative; adapters derive from existing inventories. |
| G2 Semantic fidelity | **Pass, scoped** | Exact prefix/view, profile, relationship, condition, assumption and outcome distinctions are preserved in inspected paths. |
| G3 Validity | **Pass, scoped** | Checked permits, frame/configuration validation, guarded admission and nominal batch validation provide rejection points. |
| G4 Hidden behavior | **Pass, scoped** | Pure kernels remain pure; query, native transfer and completion effects are explicit. |
| G5 Consistency and recovery | **Pass, scoped** | Pending versus complete state and registered native drain ownership remain explicit. End-to-end native acceptance is pending. |
| G6 Transformation and reuse | **Pass, scoped** | Prepared closure/graph reuse retains exact source and projection identity. The proposed transformation must preserve the contracts in §3. |
| G7 Truthful capability claims | **Pass for this assessment** | Diagnostic, implementation and acceptance claims are separated; unfinished builds and simplified reproduction are not promoted into performance or product qualification. |
| G8 Library leverage | **Pass, scoped** | Existing futures, Tokio, DataFusion and graph capabilities provide the required primitives. No bespoke replacement for an established missing capability is demonstrated. |
| CI-G1 Fidelity | **Pass, scoped** | Derived results retain method/model, qualification and independent coverage; unknown and `NotRequested` are explicit. |
| CI-G2 Evidence closure | **n.a. for served journeys** | Source/invocation linkage is inspected, but serving across calls and physical realizations is outside this review. |
| CI-G3 Evaluation integrity | **n.a.** | No evaluator or private-truth input path is changed or assessed. |

## 10. Remaining investigations and evidence limits

The next evidence should resolve decisions, rather than create a standing measurement campaign:

1. **Actual compiler attribution.** Match the hot normal/harness function and transformed PHI shape under rustc's real pipeline and target-machine settings. The simplified reproduction is a useful lead, not an exact compiler regression diagnosis.
2. **Whole operation after correction.** Inspect loader/phase/emitter children and drop implementations across CGUs; compare actual-settings compiler evidence. Do not close F01 from parent shrinkage or raw block counts.
3. **Runtime and ownership qualification.** Confirm adapter/phase allocation granularity, selected-input behavior, batch backpressure and terminal charge retention through actual affected controls. No unchanged throughput or memory claim follows from static preservation alone.
4. **Recurrence prioritization.** Summary/model scopes, synthesis, retrieval preparation, selection and analytic producers warrant targeted inspection when their source shape or retained costs justify it. Their raw sizes are leads, not an ordering of elapsed cost.
5. **Residual compiler defect.** If coherent boundaries leave the same matched optimizer pathology, investigate the pinned compiler or a deliberate toolchain change. Do not assume either that LLVM alone is responsible or that source restructuring must cure every tail.

Positive counterexamples remain outside wholesale migration: necessary typed codecs/key/digest operations, synchronous model algorithms, borrowed CPU access, prepared topology reuse and already monomorphic stream/native submission machinery. Their presence argues for selective boundaries.

Original compilation-cost F02–F04 remain at their current plan disposition. This review does not re-establish the old fingerprint defect against its corrected source, certify the local-profile switch, or qualify grouped harnesses.

## 11. Authority changes and disposition

**Rule impacts: none.** The recommended architecture does not require changing dependency pins, typed semantic authority, publication identity, optimized acceptance, ordinary compiler parallelism, or cancellation/drain obligations. These rules were evaluated as the current design's choices, not used as acceptance criteria.

| Required action | Route and owner | Disposition and closure |
|---|---|---|
| Contain structural and recurring loader coroutine composition | Core producer/scoped-loader owners, within the existing checked execution contracts | Link this review's F01 into compilation-cost plan §9; retain the loader, phase, output and ownership closure obligations separately. |
| Decide whether shared bulk append is necessary | Shared output owner, informed by residual attribution and buffering-contract analysis | No new API required by this review. Reopen only if phase/loader containment leaves material output composition costs. |
| Resolve a residual matched compiler pathology | Compiler/toolchain owner | Exact rustc settings and transformed-state evidence first; no default/toolchain change inferred from simplified reproduction. |

An ADR is not required merely for code movement inside these existing contracts. A later change to semantic authority, publication behavior or a consequential execution decision must use the appropriate architectural decision route.

## 12. Decision

**Bounded architectural decision: Revise.** The inspected semantic and lifecycle boundaries hold, but A4 fails for the current coroutine composition. Correct output and C1's successful local diagnostic improvement do not certify the remaining producer execution structure.

**Proposed direction: supported at Interface-checked/static design strength.** Borrowed coherent phases and inventory-derived adapters offer a credible route to contain compilation amplification while retaining nominal semantics, selected acquisition and charged terminal ownership. Structural implementation and integrated acceptance remain outstanding.

**Enclosing architecture:** needs revision for producer/scoped-loader compilation fit. Semantic architecture is accepted only for the inspected operations and scenarios; the Rust workspace and product are not qualified by this review.

The next consequential decision belongs to the core execution owners: select the structural loader and publication boundaries that contain both parent and child variation, then verify them under the actual pinned compiler settings and affected ownership controls.
