# Catalog upper execution: readiness and parallel computation

**Functional source audit at `160c5254`,2026-10-17: US01 prerequisite readiness is implemented; US02 remains partial on FA01/FA02 failure-state retirement. Native Summary confirmation remains separately pending.** Companion to the [coordinator](catalog-upper-execution-plan_2026-10-17.md), whose [§6](catalog-upper-execution-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint) is the sole disposition owner. Source: [target review](../design_review/reviews/design_review_catalog-upper-execution-performance-target_2026-10-17.md), F12/F13, I02/I06 and approved RC01/RC08. Baseline d37a14add3e18d6a101aa4710dcd279b5445e896. This set does not supersede, activate or reconcile existing plans.

## 1. Target and foundation assessment

Make execution order follow actual complete prerequisites, not broad phase loops or one-at-a-time orchestration. Expose useful parallel preparation and pure computation while retaining exact immutable inputs, semantic assurance, deterministic integration and actual worker finality. The target is shorter necessary execution, not simply more tasks.

Source-inspected,2026-10-17:

- lctx-model Schedule::ready already checks declared prerequisite sets, including more than direct inputs. compilation.rs nevertheless fully completes facts/availability before normalization and every normalization/whole admission before upper stages.
- ConfigurationGroup declarations have no completed inputs. They can be prepared independently; their actual native output completion/visibility must still follow the chosen producer contract.
- freeze_completed_inputs imposes ordered publication-group prefixes. Some order protects complete shared vocabulary and exact epoch bindings; inspect those obligations before relaxing it.
- Workspace StageExecution already registers actual producer joins before launch and retains task/result ownership; dropped result delivery is not worker completion. coordinate_with_pending and settle_upper_pending keep outstanding work progressing/draining while the coordinator waits or fails.
- GraphPreparation already coalesces immutable graph hydration, and preparation::Needs releases owners after their actual remaining consumers. Preserve both when readiness expands.
- Native audit contributor loops, C0 windows, C1 artifact builders, retrieval roots and analytic technique calls contain serial independent work inside otherwise concurrent stages. Higher-layer borrowed_cpu prevents inappropriate multithread worker blocking but does not create intra-kernel parallelism.

Use these existing owners. A new universal DAG engine, queue registry, global result cache, second thread pool or new durable scheduler has no established need. Core3.3/heuristics1.0/code-intelligence1.5 apply, particularly FP-07/A4, DP-08/11/19/20 and H17/H21/H22/H23/H28.

## 2. Shared execution contract

The stage owner distinguishes **eligible to prepare**, **eligible to consume**, **completed**, **semantically admitted** and **eligible for coherent publication**. These are required distinctions, not a demand for a new persisted record or physical stage for each state.

Eligibility includes the full static producer set, selected view/epoch identity, invariant/coverage dependencies and actual private preparation authority. No consumer reads a growing union, treats one completed producer as a complete family, or infers absence from a not-yet-finished sibling. Private preparation may start earlier when it does not rely on unearned semantic properties. Final whole-frontier admission and coherent public publication remain.

Pure grains receive explicit immutable inputs and resource ownership and return private results. The single semantic output owner integrates them in a defined canonical order. Worker scheduling cannot choose semantic priority, alter numerical reduction class, overwrite another grain's actual-owner state or publish after a sibling failure. Supporting references never become new analysis roots.

All accepted workers, native transport, blocking commands and result queues retain actual joins and backing reservations until terminality. Cancellation stops further useful admission and drains started work; original and secondary failures remain structured. Resource admission covers combined inputs, workspace, results and retained backing. Do not treat a library's large blocking pool as a scheduling policy or impose arbitrary job/thread caps. I06 must settle the concrete launch/output-headroom policy before general concurrent grain integration.

## 3. Packages

<a id="us01"></a>
### US01 — Dependency-ready compilation across broad epochs

**Source:** F12/I02/RC01. **Prerequisites:** UE00 shared contracts and the relevant recorded ADR/owner decision before changed governed behavior. Input-free preparation is an early slice; broader consuming stages wait for a working complete selected-assurance contract, not merely its proposed interface. **Owner:** cpg-core coordinator plus model stage/admission declarations.

First classify each provider, normalization, upper operation and publication group by its real prerequisites. Reuse Schedule's full declared/invariant/assembly dependencies. Identify where code currently requires a normalized CheckedInputs owner, where facts-only permits suffice, and where final cumulative availability/coverage truly requires a whole-frontier barrier. Define exact selected assurance at its existing model/store owner; do not reinterpret completed descriptors as semantic admission.

Migrate the driver so eligible operations can run across former phase labels. Start input-free configuration/private preparation early; make normalization and upper consumer launch depend on complete selected prerequisites and necessary assurance. Retain original producer identities and capture ownership while overlapping stages. Freeze exact bindings only when every contributor required for that binding is complete. Preserve full final admission, failure propagation, current cuts and coherent publication.

Inspect ordered prefix freezing separately. Relax only ordering whose absence is proven compatible with each consumer's exact captured epoch and shared-vocabulary completeness. Keep genuine order in the schedule/declarations. If a subset cannot yet earn required assurance, keep **that dependency** and record the missing prerequisite; do not silently retain a whole phase barrier or claim full F12 closure after only configuration overlap.

Remove replaced phase-loop coordination and stale readiness assumptions as each route migrates. Keep diagnostic phase names where they remain useful; labels need not impose a scheduler epoch. Any changed stored readiness/binding realization follows UE00's declared cutover/regeneration route, not an old-format reader.

**Closure:** block an unrelated provider/normalization and show only truly eligible configuration, preparation and facts-only consumers progress; a needed sibling still blocks its consumer. Exercise late malformed output, absent and shared-vocabulary producers, mixed frozen epochs, cancellation during binding/freeze and no public visibility before full admission. Both Catalog and Behavioral use the same readiness contract with their actual profile dependencies. Verify no lost join, premature preparation release or coordinator/gate deadlock. Source tracing plus revealing controls must establish the actual overlap; a FuturesUnordered declaration alone does not.

<a id="us02"></a>
### US02 — Owned parallel contributors, windows and pure grains

**Source:** F13/I06/RC08. **Prerequisites:** UE00's worker/integration contract; relevant immutable preparation/window delivery; native UN05 where blocking sort is involved. Not every grain waits for US01: ready independent work inside a current stage can overlap before cross-epoch scheduling is complete. **Owner:** cpg-core/native execution adapters with semantic eligibility supplied by each domain kernel.

Inventory the actual independent units: cold native contributor assurance/preparation, C0 public windows, C1 artifact builders, retrieval render partitions, compatible Completion/frame kernels, model partitions and analytic techniques/namespace contexts. For each, declare its immutable inputs, mutable state, output integration order, resource/reservation needs and real dependencies. Exclude ordered bodies, dependent transfer steps and recursive SCC operations unless their owner has established a conforming independent unit. UA04 specifically decides Summary component eligibility.

Resolve I06 before migrating shared concurrency. Choose existing task/blocking ownership and a launch policy that uses normal available concurrency with actual admitted work/result capacity; account for DataFusion/analyzer/native execution already in flight. Bound live results through actual output ownership and deterministic integration, not unbounded submission or arbitrary worker counts. Do not convert ordinary allocation refusal into a generic retry loop or introduce cost-accounting machinery merely to estimate concurrency. A capacity-related wait or retry needs explicit progress and ownership semantics and must not deadlock behind unintegrated results.

Launch compatible private work independently; coalesce equal immutable preparation through the existing owner. Integrate typed results through the one contribution/actual-owner writer in the required canonical order. Where domain output is order-independent, declare and test that equivalence rather than inheriting completion order. CPU placement uses borrowed_cpu or acknowledged owned blocking work as appropriate to borrowed lifetimes and runtime flavor. Register actual joins before exposing launched work.

Migrate each identified eligible route and remove its serial launch-and-immediate-await loop. A deliberate retained serialization needs a specific dependency/resource reason and a recorded limit; one passing parallel C1 case does not establish all F13 consumers migrated. Recheck preparation::Needs against started, running and remaining consumers, not launch position or predicted cache hits. Cross-stage and intra-stage overlap must compose without nested uncoordinated pools.

**Closure:** deliberately block an independent grain and demonstrate another starts/finishes; shuffle completion and compare canonical results, sources, numerical policy and effects. Cover overlap/duplicates, sparse/single-grain work, skewed grains, result headroom, Resource refusal, current-thread and multithread ownership where actually used, lost waiters and cancellation during computation/publication. Started workers and submitted native work drain; results are not admitted after failure; resources release after actual last use. No timing threshold or thread-count cap is a pass criterion.

**Remaining first-failure lifetime correction,2026-10-17 (FA01/FA02; coordinator §6 owns disposition).** Actual independent routes are launched; the missing behavior is immediate retirement of coordinator payloads which cannot integrate after failure. In `analytic::run_techniques`, consume/drop `TechniquePreparation` on the first ready/launch/cancellation/integration/worker refusal, including integrated output, retained vector preparation and unlaunched dependent jobs. In `compilation::coordinate_with_pending`, clear previously queued terminal `ReadyStages.rows` when the first failure is observed, rather than waiting for the in-flight action to return. Live jobs/actions retain their own authentic borrows; keep ordinal/queue capacity charges, pending futures and actual registered joins until drainage. Preserve canonical primary/secondary errors, cancellation and refusal of successful integration after failure.

Use separate DB-free gated-peer and gated-action cases to distinguish dead preparation release from live worker completion: keep an accepted peer/action paused, observe deep dead-payload reservations return immediately after failure, verify live backing remains held, then release the gate and require full drainage and unchanged errors. Native latency, historical timeouts and full-frontier compilation are unnecessary for these two corrections. US01's readiness algorithm needs no redesign; this is completion of the common SC05/I06 ownership contract.

## 4. Interactions, validation and handoff

[Native UN01/UN05](catalog-upper-execution-native-plan_2026-10-17.md) supply audited input and blocking-sort contracts; native contributor overlap is implemented under US02 without merging producer/receiver trust. [Consumers UC01–UC05](catalog-upper-execution-consumers-plan_2026-10-17.md) supply compact/window inputs and migrate acquisition; US02 separately migrates eligible computation/emission. [Analysis UA01/UA02](catalog-upper-execution-analysis-plan_2026-10-17.md) reduce repeated kernel preparation before parallel method work. UA03/UA04 may decide additional eligible algorithms/components, but do not block unrelated scheduling.

Coordinate edits to compilation/workspace execution, native audit workers, scoped positions and model declarations explicitly. Separate logical prerequisites from shared-file contention. Root owns readiness, shared contracts, integration and acceptance; delegation does not create another disposition ledger.

Use affected crate compilation and focused readiness/worker/integration controls during implementation. Database-free blocked-peer cases reveal many scheduler failures; native visibility/admission cases must use genuine persisted logical attempts after existing stable readiness is established. UE90 composes these with changed consumer controls and applicable hygiene. Full Catalog/assembled qualification, embedding inference, MCP query runtime and operator adoption remain outside this endpoint.

US01 closes only when complete prerequisite-based readiness replaces unjustified broad barriers in the selected scope. US02 closes only when identified independent routes are migrated or have an explicit justified retention decision; unjustified remaining serialization keeps its finding open. Performance benefits are mechanistic and unmeasured. UE99 hands off later integrated validation and any exact remaining uncertainty without rewriting the old failures or protected Unknowns.
