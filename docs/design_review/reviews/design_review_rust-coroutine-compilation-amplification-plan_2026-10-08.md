# Design review: Rust coroutine containment plan

**Independent design / target review · 2026-10-08.**

The revised Proposed target is accepted within the scope below. Its useful change is to make
relation inventories select small typed operations while shared traversal and coherent borrowed
phases contain asynchronous execution. The target preserves the semantics that make a pinned
API/evidence catalog trustworthy: exact completed inputs, owned selection, model-derived outcomes,
charged transfer and terminal native ownership. It also requires inspecting children and drops,
so acceptance cannot be obtained by relocating the expanded coroutine into a helper.

One material coverage omission was found during review: the model scoped loader's actual producer
and publication child have the same source-visible composition cause. The author revised the
consumer map, target, CU2 and acceptance cases to include them. §7 retains stable F01 and the
independent follow-up assessment. This resolves the **plan coverage defect**, not the underlying
production amplification. The compilation coordinator remains the sole mutable finding owner.

## 1. Scope, functional outcome and baseline

| Field | Value |
|---|---|
| Subject | [Coroutine companion](../../plans/rust-coroutine-compilation-amplification-plan_2026-10-08.md), its BC1/BC3/BC5 integration in the [compilation coordinator](../../plans/rust-compilation-costs-plan_2026-10-08.md), and the narrow dependency paragraph in the [persisted-execution coordinator](../../plans/persisted-graph-execution-plan_2026-10-07.md) |
| Standard | Core/template 3.3; Heuristics for Efficient Architecture 1.0; code-intelligence principles/review additions 1.5; library-context binding from `standard.toml` |
| Reviewer | Independent delegated design reviewer; root is the plan author |
| Tier / purpose | Design / target |
| Maturity | Proposed correction; source contracts and selected library interfaces inspected independently |
| Functional target | Pinned public API/configuration discovery with connected source, invocation, usage and deployment evidence; supported uncertainty remains explicit |
| Baseline | Main `32ef660c5b258200067c18f481e75eac55f17a86`, with the preserved dirty integrated BC/PC tree, grouped harnesses, C1 correction and shared stream/native ownership |
| Review effects | Read-only repository inspection; only the delegated scratch review is written. No build, compiler pass replay, probe, test, synchronization, process intervention, commit or delegation |

Reviewed final document SHA-256 identities:

| Document | SHA-256 |
|---|---|
| Coroutine companion | `7641ce16720512e049ac16146c68277be4db0793ff8e3fc144c73a45dce19c1b` |
| Compilation coordinator | `cdc6c430044b204a9b4311148bbc5fba0219bfca243990f4a42ea8d6ae22f5a5` |
| Persisted-execution coordinator | `2c6ebe3856f92506f74ce5b9de7fb07c23f5afa8d11f1f8a7710a4eaa4cfd730` |

The workload premise is compilation of pinned libraries with growing relation inventories and
many parent frames, followed by programmatic catalog/evidence construction. Semantic-kind growth
should add necessary codec and domain work without amplifying broad coroutine representations.
Large or skewed selected grains must retain their complete required universe, explicit work/resource
limits and partial outcomes. Per-frame output must remain bounded and charged; cancellation can
outlive a waiting caller once native work is submitted. Ordinary local and optimized acceptance
products are separate compiler/runtime realizations.

This review covers the proposed containment architecture and its named consumers. It does not
certify all async functions, exact LLVM defect attribution, profile installation, complete native
journeys, served multi-call pinning, real-library quality or operator adoption. The earlier source
review supplies leads and historical diagnostic context; its judgments are not inherited as proof.

## 2. Responsibilities and semantic ownership

The [architecture map](../../design/README.md), [semantic model §15](../../design/sections/semantic-model.md),
[analytics](../../design/sections/analytics.md), [behavioral analysis](../../design/sections/behavioral-analysis.md)
and [synthesis/serving](../../design/sections/synthesis-and-serving.md) locate the relevant owners.
The [product target](../../design/sections/api-and-evidence-product.md) establishes the functional
intent; chosen persistence and process rules describe the present design rather than substitute
for the standard's architectural criteria.

| Owner | Governing responsibility | Proposed dependency and consumer boundary |
|---|---|---|
| `lctx-model::domain` | Nominal records/identity, definitions, input declarations, qualification, coverage, projections and operation outcomes | Inventories and codecs remain derived from it; no parallel decoder registry or runtime schema authority |
| Completed input / scope owners | Exact immutable type/prefix/view permits and selected closure/table binding | Stage-specific selection remains owned; thin typed adapters bind permits and synchronous batch visitors |
| Core stage owners | Profile branches, configured methods, root/frame order, algorithms and result publication | Compose borrowed phases; own any specialized SQL, visitor or result policy |
| Shared stream owner | Ordered query planning, erased batch visitation, execution/yielding and terminal errors | One physical mechanism below typed permit/selection boundaries; no row-frequency dispatch |
| Producer output / NativeCalls | Pending versus submitted state, bounded checked batches, registration, failure poisoning, terminal drain and completion | Retain complete charged arguments and registered native work; no new scheduler |
| Existing coordinators | Current source finding disposition, profile transition and integrated native acceptance | Companion supplies CU0–CU6 obligations; compilation §9 owns F01; persisted coordinator retains its PC/PG obligations |

The semantic model is adequate for this mechanism change. A source snapshot, a selected physical
table, a typed permit, a configured invocation and an outcome have consequential different
meanings. Current checks and operations enforce those distinctions; the proposal changes their
composition rather than reconstructing their meaning in orchestration.

| Family | Provider/revision and fidelity | Coverage / identity | Consumers and preservation |
|---|---|---|---|
| Native/normalized premises | Captured provider and source revision; extracted and resolved remain distinct | Exact completed membership/prefix, original identity and explicit missing/unknown coverage | Typed structural, model, summary, selection and synthesis consumers |
| Structural and analytic results | Owned method/definition/parameters and projection; structural evidence versus heuristic analytics | Exact invocation/parent/source links; explicit bounded and `NotRequested` outcomes | Coverage publication, C2/S0 and retrieval preserve their original meaning |
| Model and summary evidence | Selected catalog/model, phase and admitted witness context; finite model-relative derivation | Exact vocabulary epoch, witness absence, qualification and channel coverage | Model applications, merged runs, Summary and evidence-backed conclusions |
| Synthesis/retrieval products | Programmatic source-grounded derivation and rendering | Original chunks, member/root identity and admitted evidence closure | Later native publication/serving; no claim of served-journey qualification here |

## 3. Contracts and independent verification boundaries

Decisive source inspection supports the preservation contract:

- [ConsumedInputs](../../../crates/cpg-core/src/consumed_rows.rs), lines 31–90, owns declaration
  ordering/deduplication and undispatched exact type/prefix consumption. Repeated inventory entries
  are not permission to collapse different prefixes or reorder adapters.
- [Checked streaming](../../../crates/cpg-core/src/consumed_rows.rs), lines 93–234, binds typed
  permit identity, selected SQL and declaration ordering before the shared erased batch loop.
  Existing adapter preparation can inspect immutable metadata; the proposed phase boundary must
  preserve observed error delivery, effect timing and precedence rather than introduce eager
  mutation or native submission.
- [FrameScopes::read](../../../crates/cpg-core/src/analytical_scopes.rs), lines 741–764, traverses
  **every** matching declaration and uses its table index and prefix. The nested loop is itself
  in scope, not merely the producer awaiting it.
- [Coverage admission](../../../crates/lctx-model/src/domain/analysis/expected.rs), lines 505–550
  and 824–839, checks handled inputs before decode/mutation; a skipped relation does not run the
  conditional guard. Unconditional prevalidation would change that contract.
- [Typed batches](../../../crates/lctx-model/src/domain/record.rs), lines 605–667, retain nominal
  validation, identity sorting/deduplication, conflicting-row refusal and the encoding reservation.
  Arrow structural consistency alone cannot discharge those guarantees.
- [Output buffering/completion](../../../crates/cpg-core/src/workspace.rs), lines 1900–1920,
  2124–2160 and 2200–2274, distinguishes pending buffers from submitted transfers and completed
  visibility. `push` submits at a flush; it is not a native request for every row.
- [NativeCalls](../../../crates/cpg-core/src/native_calls.rs), lines 29–90, retains registered
  submitted work and late failures through caller cancellation and repeated drainage.

The proposed controls challenge overlap, missing/foreign views, first-error precedence,
conditional guards, unpolled operations, unrelated payload isolation, conflicting/empty outputs
and cancelled submitted writes. These are independent semantic expectations, not adapter-count
checks or tests regenerated from the same inventory.

## 4. Composition and execution fit

Structural currently expands inventory, declaration, scoped-read and emission awaits in
[one producer](../../../crates/cpg-core/src/structural.rs), lines 114–147 and 234–308. Its
`load<R>` also embeds a declaration loop and nested admission/stream awaits, lines 48–81.
The selected target removes unnecessary coupling between inventory growth and enclosing execution
representation while retaining necessary per-record validation/decoding.

CU1's driver has a small sequence of coherent borrowed operations. Typed adapter selection derives
from existing inventories; a shared loop awaits uniform futures. A child containing many concrete
awaits must use contained dispatch or smaller coherent children. Compiler inspection includes
those children and drop paths across normal/harness CGUs. This is a credible containment mechanism;
boxing only the outer future would leave the cause intact.

The runtime route remains credible as grain and parent counts grow: prepared scopes/topology are
reused, graph kernels borrow their representations, and result rows release per parent. Structural
drops its grain before model computation and its frame state before the next parent; scope,
admission and captured source owners release before final completion. Borrowed phases need neither
all-parent materialization nor uncharged cloning to obtain a lifetime. Coarse allocation and
indirect polling are a justified tradeoff for containing concrete execution variation. No new
row-frequency allocation or scheduling is selected.

| Question / operation | Universe and selector | Method / accuracy / model | Resource and evidence obligation |
|---|---|---|---|
| Structural derivation | Exact prepared callable/containment graphs and admitted parent frame; output selection does not remove required intermediates | Existing configured model methods, canonical definitions/parameters and qualified evidence | Preserve traversal/work bounds, explicit stops/outcomes and per-parent charged publication |
| Analytic production | Exact selected structural parent and complete selected technique set | Retained settings/policy; ranking/grouping remain heuristic | Preserve disabled/unsupported distinctions and definition/projection linkage |
| Model / Summary | Exact selected witness/catalog epoch and model-relative scope | `apply_selected`, `merge_run` and finite Summary semantics remain model-owned | Preserve absence/unknown states, phase roots, qualification and run/evidence aggregation |
| Synthesis / retrieval | Documentary/member/conclusion chunks and root/brief-selected original source | Programmatic derivation/rendering; explicit admitted evidence status | Preserve projected metadata admission, source isolation and per-grain release |

Summary's optional selection and occurrence/artifact/symbol property visitors, model's captured
catalog and witness policies, S0 chunk phases and retrieval's projected columns are not interchangeable
loader policy. The proposal keeps them with their owners and shares only uniform traversal.
Normalization's [event/binding loaders](../../../crates/cpg-core/src/normalize/call_scope.rs),
lines 513–544, use first-match lookup. Its ordinary construction uses model-generated EventData /
BindingData declarations, but broader `from_tables_with` callers exist. CU0 correctly makes the
cardinality contract an explicit investigation before sharing an all-prefix reader. No semantic
change follows merely from uniformizing the helper.

Adjacent publisher search uses bounded typed key windows; serving evidence's inspected async
accessors select synchronous prepared rows; native loading uses bounded canonical windows. These
source shapes do not independently prove inventory amplification. CU0/CU4 bounded triage and
justified exclusions are proportionate. No equal-depth serving audit or wholesale async migration
is selected.

## 5. Change and failure scenarios

| Scenario / kind | Owned change and affected consumers | Judgment |
|---|---|---|
| Add a premise relation / domain extension | Model declaration/codec and semantic visitor change; derived adapter inventory follows; broad async traversal stays shared | Necessary semantic propagation is preserved while avoidable implementation expansion is contained |
| Add a configured structural method / domain concept | Definition, parameters, frame/coverage/outcome rules and specialized algorithm have existing model owners | Coherent phases compose the new operation without a generic executor DSL |
| Replace stream acquisition / mechanism | Exact completed-view permits and stage selection remain contracts; shared physical planning/stream mechanics change beneath them | Plausible substitution without new semantic authority |
| Replace concrete future composition / mechanism | Borrowed opaque operation boundaries preserve public products, error ordering and native lifecycle | Existing C1 and stream interfaces establish a usable pattern; children require independent containment |
| Distinct same-type prefixes or skipped summary input / binding | Keep every applicable exact prefix and owned optional selection; normalization retains its established cardinality | No name/type-only deduplication or eager superset load |
| Grow selected graph or parent count / workload | Reuse prepared views; bound selected state and outputs; retain all required intermediates | No all-parent result collection, traversal narrowing or cloned uncharged backing state |
| Cancel after submitted write / failure | Native owner keeps charged batch/writer and terminal joins; waiting caller may disappear | Coherent phase extraction cannot detach ownership or erase late failures |
| Unpolled phase or first loader failure / failure | Preserve lazy effects and first-error ordering; later adapters do not run | Independent controls target meaningful observable behavior |

## 6. Gates, settled independently

These are Proposed/static review judgments, not executed test outcomes.

| Gate | Verdict | Independent basis and limit |
|---|---|---|
| G1 Authority | Pass, scoped | Model inventories/operations and exact completed-view owners govern adapters; no competing maintained registry |
| G2 Semantic fidelity | Pass, scoped | Prefix, cardinality, profile, selection, qualification and outcome preservation are explicit; no selected silent semantic change |
| G3 Validity | Pass, scoped | Typed permits, guarded admission, nominal batch checks and invalid output refusal remain enforced; independent refusal cases scheduled |
| G4 Hidden behavior | Pass, scoped | Pure computation remains model-owned; query/native effects and poll-time behavior are explicit |
| G5 Consistency and recovery | Pass for target | Pending/submitted/completed distinctions, terminal charges/joins and poisoned/late-failure behavior survive composition; actual native qualification pending |
| G6 Transformation and reuse | Pass, scoped | Prepared exact scopes, original identity mappings, typed model products and complete source dependencies are preserved |
| G7 Truthful capability claims | Pass | Target remains Proposed; static recurrence does not claim elapsed-cost ranking; partial diagnostic and actual native acceptance remain distinct |
| G8 Library leverage | Pass, scoped | Existing futures and batch-stream capabilities supply required mechanisms; no new bespoke scheduler, parser or universal visitor |
| CI-G1 Fidelity | Pass, scoped | Provider/resolved/derived/heuristic distinctions, model-relative outcomes and explicit coverage/unknowns remain governing |
| CI-G2 Evidence closure | Pass for producer transformation; served journeys n.a. | Synthesis/model evidence and exact original-source membership are preservation obligations; multi-call serving realization is not reviewed |
| CI-G3 Evaluation integrity | n.a. | No evaluation mechanism or private-truth input path is selected or changed; revealing controls must still have independent expectations |

## 7. Stable finding and follow-up assessment

### <a id="F01"></a>F01 — Model production was omitted from whole-operation recurrence coverage

**Priority P2; responsible owner: model producer/scoped-loader owner and companion author.**
**Principles/judgment:** FP-01, FP-03, FP-06, FP-07; DP-08, DP-16, DP-17; A1/A3/A4.
No semantic gate failure is established.

**Diagnosis, source-inspected 2026-10-08.** The initial draft explicitly scheduled ModelScopes::load
but its named producer packages omitted the actual `semantic_models::apply` consumer and
`publish_records` child. [Model production](../../../crates/cpg-core/src/semantic_models.rs),
lines 93–114, expands metadata/expected typed loader awaits; lines 148–180 expand output declarations;
lines 332–343 compose typed root query/stream/grain/load/publication awaits; lines 433–480 expand
record-specific output awaits. These are concrete manifestations of the selected composition
cause, not inferred hotspots from an async signature or raw IR count.

**Consequence.** Correcting only the scoped loader could satisfy CU2 while leaving the actual
model operation and emitter expanded as relation inventories grow. The plan's whole-operation
closure would depend on an unstated later discovery. This is a coverage defect even without a
claim that model production is the dominant elapsed compiler cost.

**Correction.** Include model application and publication in the named mandatory assessment and
confirmed correction. Preserve captured catalog filtering, Catalog/Behavioral branches, ordered
ProductionScope roots, synchronous `apply_selected`, `merge_run` aggregation and every selected
result's evidence publication. Inspect its children/drops, not merely the revised parent.

**Independent follow-up, 2026-10-08.** The revised companion's §2 consumer row, §3.1 model paragraph,
CU2 and §6 stage-selection controls explicitly establish that scope and preservation contract.
I read those revisions against the caller/emitter source. The **plan coverage defect is corrected**
at Proposed strength; no further text remedy is required. Implementation/compiler/runtime evidence
for the source recurrence remains outstanding.

**Disposition route:** [compilation coordinator §9 F01](../../plans/rust-compilation-costs-plan_2026-10-08.md#9-sole-compilation-cost-finding-disposition),
through CU2/CU6 and BC1/BC5. This review retains its dated diagnosis/follow-up; it creates no second
mutable disposition table and does not close the original source F01.

Applicable FP-01–FP-06 and DP-01–DP-12/DP-18–DP-24 are satisfied for the proposed changed boundaries
and scenarios. FP-07 and DP-13–DP-17 are satisfied by coarse library-backed composition and removal
of the diagnosed unnecessary broad execution representation. CI-01–CI-10 and CI-11's producer-side
preservation are satisfied within inspected responsibilities; complete served CI-11/CI-13 journeys
and CI-12 evaluation are outside scope. These aggregate assessments do not certify untouched owners.

## 8. Library fit and total integration burden

The resolved workspace declares futures 0.3.34, Tokio 1.53.2, Arrow 59.3.0, DataFusion 55.1.0 and
petgraph 0.8.3. Cargo.lock and selected local sources were checked for the interfaces used here;
transitive older futures/petgraph entries are not treated as this boundary's API.

`futures-core` 0.3.34's `future.rs:18` defines `BoxFuture<'a, T>` with a borrow lifetime and `Send`.
It supports the proposed borrowed function-pointer/phase boundary without imposing `'static`.
C1's [inventory and scoped adapters](../../../crates/cpg-core/src/catalog_evidence.rs), lines 20–81
and 122–198, demonstrate the relevant composition in this checkout. Reuse the pattern where
contracts agree; its specific phase names are not a universal design.

DataFusion 55.1.0's local `dataframe/mod.rs:1601` returns a SendableRecordBatchStream and documents
query abortion on stream drop. The current checked stream owner already uses that capability;
new loader structure must preserve its selected SQL/order and batch callback. Query abortion is
not native write drainage. The DataFusion skill's pin matches the selected workspace family.

Existing graph hydration and synchronous borrowed domain kernels remain positive foundations.
No graph replacement addresses coroutine inventory expansion. Complete `Batch<R>` and native
request ownership are essential integration guarantees, not library-availability claims.

Coarse boxes/indirect polling add bounded operation-frequency machinery. Their benefit is to
contain implementation variation while preserving useful streaming, batching and borrowed kernels.
Per-row boxes/tasks, a new executor or an independently maintained decoder registry would add
unnecessary lifecycle/semantic burden. No dependency addition or version move is justified.

## 9. Alternatives and decision relationships

| Alternative | Assessment |
|---|---|
| Retain the expanded concrete coroutine | Keeps avoidable inventory-to-coroutine coupling; not selected |
| Box only the outer producer or move its body unchanged | Does not contain child execution/drop machinery; insufficient |
| Coherent borrowed phases plus ordered thin adapters | Simplest credible correction using current contracts/libraries; selected |
| Universal runtime schema visitor / new executor | Adds independent semantic or lifecycle machinery without a current need |
| Box/spawn each output row | Introduces costs and ownership pressure at the wrong granularity |
| Checked bulk append | Conditional only after residual source/compiler evidence and buffering/error/cancel semantics justify it |
| Lower optimization / arbitrary crate split / worker cap | Does not establish the corrected operation's composition fit; no new rule change recommended |
| Deliberate toolchain correction | Possible for a matched residual compiler pathology; exact pinned-settings attribution first |

CU5 now permits a justified keep-current output decision using available source/first-build evidence;
it does not require proving a positive hotspot before that decision. CU6's final closure follows
CU5 resolution. There is no mandatory bulk API or circular requirement for final closure evidence
before deciding whether output refinement is needed.

The shared reader contract is a prerequisite for its actual migrations, not a reason to wait for
unrelated PC6 journeys. Model/summary/synthesis policies remain independent owners. BC3 cannot
install a default using smaller-parent evidence while its exercised paths retain known blocking
composition. Full source F01 closure additionally requires all confirmed migrations and justified
exclusions. Final optimized acceptance and matching PC6 receipts remain distinct from a local
candidate pass. These package relationships preserve necessary assurance without repeating full
qualification after each source slice.

## 10. Verification and material uncertainty

**Passed, read-only inspection (2026-10-08):** `git status --short`, `git rev-parse HEAD`, focused
`rg`/`sed` source/authority reads and `sha256sum` on the reviewed documents. These establish the
stated source/document baseline and interface evidence only. No product check was run by this reviewer.

**not_run:** Cargo checks/builds, nextest/pytest, compiler probes/pass replay, native fixtures,
`just qualify`, documentation publication checks and any real-library journey. The authoring
assignment prohibits new build/probe/process activity; existing native run ownership is untouched.
The coordinator's reported `just verify --print` resolution/readiness observations are its own
receipts, not executions or qualification by this review.

The target is sufficiently specified for Proposed acceptance from static evidence. Remaining
implementation obligations are concrete: preserve exact overlap/profile/cardinality behavior,
contain parents/children/drops under actual pinned settings, establish final-source native terminal
ownership and reject material runtime regressions. Partial diagnostics do not prove a full-build
speedup, and no unchanged throughput or peak-memory result is asserted here.

Exact actual-rustc optimized PHI/pass attribution could change a compiler-defect remedy, but does
not undermine the source-supported containment direction. If coherent containment leaves the
matched pathology, investigate the pinned compiler under its real settings. Adjacent exclusions
reopen on confirmed source amplification or retained attributed cost. Normalization cardinality
is settled before that consumer migrates; a separately discovered semantic defect needs its own
explicit correction and acceptance.

## 11. Rule impacts and current disposition

**Rule impacts: none.** The recommended target needs no change to dependency/toolchain pins,
profile policy, compiler/test parallelism, semantic authority, selected publication mechanism,
exact completed-view contracts or charged cancellation/drain obligations. No new hook, gate,
register, document type or persistent output API is required. The review judged those mechanisms
against the loaded standard and functional target, rather than accepting them solely because
repository rules select them.

The compilation coordinator's §9 remains the sole mutable source-finding disposition. Its §8
and companion §5 schedule investigations; companion CU0–CU6 schedules corrections/controls.
Persisted-execution PC/PG findings, cold restored coverage and native/MCP journeys retain their
own coordinator. Ordinary internal phase/adapter extraction needs no new architecture decision;
a later change to a consequential semantic/lifecycle boundary must receive its own decision.

## 12. Architectural judgments and bounded decision

| Judgment | Verdict | Independent scenario evidence |
|---|---|---|
| A1 Localize change | Satisfied for revised Proposed target | Model/selection/stage/effect owners stay coherent; F01 revision supplies the actual model caller/emitter; generated adapters follow owned inventories |
| A2 Encode domain meaning explicitly | Satisfied, scoped | Exact views/prefixes, roles, qualifications, coverage, model-relative outcomes and publication lifecycle have governing definitions/operations that the mechanism preserves |
| A3 Extend through composition | Satisfied, scoped | Borrowed phases and shared ordered traversal compose existing typed, streaming, model and native capabilities without a universal framework |
| A4 Fit execution to the supported workload | Satisfied for Proposed correction | Inventory growth is contained below broad drivers; child/drop inspection prevents relocation; prepared selection, borrowed kernels, charged per-grain state and terminal recovery remain credible as inventories/parents grow |

**Bounded decision: Accept the revised Proposed target within the stated scope.** Stable F01's
plan coverage defect was corrected and independently re-inspected. No blocking target finding or
rule impact remains. This is architecture/plan acceptance, not implementation or native qualification.

**Enclosing architecture:** the current source still needs the source review's coroutine correction
and the coordinators' actual acceptance evidence. This review does not certify the Rust workspace,
profile default transition, complete serving journey or product quality.

The next consequential implementation step belongs to the core producer/shared-reader owners:
select and implement CU1's structural loader/publication boundaries, preserving exact input and
terminal ownership contracts, then perform the scheduled named-consumer corrections including CU2's
model operation. Compiler and runtime closure belongs to CU6/BC5 on final source; PC6 can consume
only matching optimized evidence.
