# Catalog runtime efficiency: assembled target review

**Accept scoped, Interface-checked, 2026-10-16; latest bounded source reassessment `a9a6db1a` / `cf9b97ef`, with control correction `967cf1eb` (2026-10-17 UTC).** The assembled implementation has a coherent route for demand-shaped acquisition, independently joined producers, bounded atomic native effects, protected receiving, shared Base preparation and early backup source finalization. These changes remove identifiable repeated preparation, committed protocol stages and unnecessary resource lifetimes without moving semantic authority into orchestration. The initial `a5f50a53` assessment required two bounded corrections; both are now source-supported, as explained in §9. The subsequent F03 source reassessment in §12 accepts the completed synchronous adapter corrections; §13 accepts the mechanical facts-future ownership refinement at source strength. Sections 14–15 assess terminal reference settlement, cleanup/pin bulk lowering and the stopped-release control at source strength. Functional acceptance and native concurrency closure remain pending. Earlier judgments retain their named baselines. Functional/native qualification remains with the coordinator, and no measured benefit is claimed.

At the initial baseline, Summary still enumerated proof Cartesian products after queue exhaustion despite a smaller exact residual representation. A second defect affected singleton feasibility: an acquisition marker covered pure preparation before actual native execution, prohibiting narrowing after a locally recoverable reservation refusal. Those diagnoses retain stable IDs F01/F02 below. Their corrections preserve original queue admissions, admitted proofs and the prohibition on retrying issued/uncertain acquisition.

This is an independent **design / target** review of the complete [runtime coordinator](../../plans/catalog-runtime-efficiency-plan_2026-10-16.md) and its native, compiler, analysis and completion companions. The initial inspected production baseline is **`a5f50a53`**, including the complete recovered-owner correction. Sections 2–8 preserve that baseline assessment; §9 supplies the Summary/capture correction judgment through **`58e6cf67`**, including capture correction `99f42979`, fixture refinement `32460297`, and Summary corrections `ad0813ae` / `58e6cf67`. §10 assesses the subsequent named native and fixture follow-ups through **`6eb0f01d`**. Concurrent coordinator documents and unrelated source work are not final-source qualification. This review changes only this document. Current execution disposition belongs solely to [coordinator §6](../../plans/catalog-runtime-efficiency-plan_2026-10-16.md#6-sole-disposition-and-current-checkpoint); the dated assessments below do not form another status ledger.

## 1. Scope, functional intent and evidence boundary

The criteria are core principles/template **3.3**, efficient-architecture heuristics **1.0**, code-intelligence profile **1.5**, and the repository binding declared by [standard.toml](../design_principles/standard.toml). The functional target is the pinned API/evidence catalog and supported Behavioral enrichment, located through the [product owner](../../design/sections/api-and-evidence-product.md) and [semantic owner](../../design/sections/semantic-model.md). Architectural sections, plans and ADRs describe the subject; their chosen mechanisms are not criteria that shield the implementation from better alternatives.

The workload is a pinned release containing many fact families, sparse and dense demand, overlapping and disjoint root closures, high-degree call endpoints, multiple proof alternatives, concurrent compiler/reader owners and interrupted native operations. Preparation, intermediate state, publication and recovery must remain credible under one actual resource pool. The profile's evidence and analysis journeys remain relevant to design reasoning even where runtime execution is excluded.

Included are R:F01–F09 and their interactions, including the native refinement and inherited semantic/finality constraints. Embedding inference, submitted MCP runtime, real-library/operator adoption and the older paused execution tracks are not qualified here. Their exclusion does not narrow the supported high-degree Behavioral scenario to a convenient fixture. This review is not a wider repository audit or acceptance run.

Method: direct source/document inspection, changed-boundary comparison from `1f8a5537`, and inspection of exact local library sources. Previous reviews and supplied receipts were leads. No Cargo, tests, process inspection, database queries or service operations were performed. All source behavior claims below are **Implemented / Interface-checked, 2026-10-16**; proposed corrections are labelled separately. No latency, throughput, capacity or memory improvement is Measured.

## 2. Responsibilities and semantic authority

| Owner | Governing responsibility and contract | Representation / consumer boundary |
|---|---|---|
| `lctx-model::domain` | Fact identity/fidelity, scope programs, root roles, coverage, proof and residual meaning, work/refusal outcomes | Native/Arrow forms and compiler preparation consume these definitions; a selected output is not the traversal universe |
| `lctx-compiler-store::consumed_rows` | Exact provider binding, reached adjacency, nominal closure and independent root membership | Immutable provider owner, typed namespace and charged exact dependency scope remain attached to preparation |
| `cpg-core::compilation` and `WorkspaceExecutionOwner` | Model readiness, launch ownership, terminal-result integration, prefix completion and drainage | Independent tasks own execution; one coordinator integrates results and retains semantic barriers |
| `lctx-native::control` | Original request identity, era/owner/incarnation authorization, atomic effects, complete cuts, recovery and references | Callers submit bounded owned actions; absence and host exit cannot invent noncommit or physical finality |
| `installed_view` / `NativeReader` | Exact source/receiving physical protection and operation currency | Complete receiving inventory and actual pin/client/pool/context authorize preserving reads; semantic admission remains independent |
| Base / Summary model kernels | Root-local evaluation/read meaning; finite admitted proofs, recursive progress and residual obligations | Shared immutable preparation preserves root partitions; proof identity and residual identity are different concepts |
| `lctx-publisher` | Exact snapshot closure export, checked source finalization, local carrier and final publication | Snapshot-local descriptors are reused; owned spools survive source release without becoming admission authority |

The consequential model distinctions govern executing verbs, not just output schemas. Native apply consumes an original request and owner context, participates in a cut, changes a bounded payload and produces an exact terminal outcome. Receiving consumes complete exact source manifests and earns independent holds before publishing a usable receiver. Base partitions select typed rows before mutable production. Summary queue admission, proof retention and residual publication have separate operations and identities.

The code-intelligence fidelity map is unchanged by physical fusion/sharing:

| Family / product | Fidelity and authority | Coverage, identity and downstream meaning |
|---|---|---|
| Canonical syntax and typing/flow observations | Independent pinned Ruff, Pyrefly and ty providers; attributed assertions | Provider/model/context and source identities remain; unavailable or unresolved input is not complete-empty |
| Normalized calls, bindings and contextual scope links | Model-owned resolution/derivation over exact completed predecessors | Physical/virtual roots, nearest/first ranking and typed namespaces remain distinct |
| Base expression/read products | Model-derived evidence under actual Local and source roots | Full physical literals, expression opacity, requestedness and exact negative universes remain per root |
| Summary proofs and residuals | Finite proof evidence versus qualified over-approximate obligations | Proof IDs/costs survive separately; residual identity omits irrelevant proof-pair distinctions without upgrading partial meaning |
| Published / backed-up content | Exact admitted immutable graph plus root-qualified physical correspondence | Each destination earns independent admission/ownership; a transport receipt is not semantic success |

## 3. How the changes compose

### Demand, joined execution and drainage

`NominalClosure::prepare` retains program metadata without acquiring every table. `EdgeSource::provider` and `source` earn reached namespaces through separate `OnceCell` slots; contextual adjacency is prepared once for a reached source namespace, while declared native references retain selected forward/reverse frontiers ([consumed_rows.rs](../../../crates/lctx-compiler-store/src/consumed_rows.rs), lines 327–364, 576–645, 1007–1190). Whole contextual partitions remain where ranking/negative semantics require them. This is a meaningful removal of unused acquisition, rather than predicate pushdown that changes the answer. Failed initialization is not published as reusable success. Independent complete-domain admission still has its own job.

`StageExecution::spawn` registers the actual shared JoinHandle before a launch channel admits useful work. It carries task/result reservations and rebinds tracing, retry cancellation and recovery scope. Dropping a coordinator result future does not abort or release the actual work owner. `WorkspaceExecutionOwner` seals/joins stage tasks **before** snapshotting provider descendants and draining bridge/native/store finality ([stage_execution.rs](../../../crates/cpg-core/src/workspace/stage_execution.rs), lines 36–117; [workspace.rs](../../../crates/cpg-core/src/workspace.rs), lines 132–203). This ordering closes the important interaction introduced by independent placement: a late stage cannot register an unobserved provider after the provider snapshot.

The coordinator still uses declared readiness and completed membership, retains gate-held handoff/rebase, and collects pending results while waiting for its own action. Failure stops admission and settles siblings; completed-prefix membership, rather than launch order, controls preparation release ([compilation.rs](../../../crates/cpg-core/src/compilation.rs), lines 663–730, 1263–1410, 1520–1600). `CompilerCancellation` requests stop on coordinator loss. Actual join finality and operation refusals retain separate meanings. Multithread execution can progress during another task's borrowed CPU region; current-thread correctness does not promise CPU overlap.

### Atomic native windows, actual incidence and protected premises

Ordinary effects now use the same complete `owner_effect_plan` for sizing and execution. One transaction checks original-era admission, locks the exact effect identity, authorizes the owner, participates in its stripe, installs the payload and outcome/reference atomically. Durable host pending metadata precedes issue, terminal knowledge precedes reference release, and reconciliation fences an absent original on the same native identity ([authorization.rs](../../../crates/lctx-native/src/control/authorization.rs), lines 788–955, 976–1173, 1663–1687). There is no ordinary separately committed intent stage to recover or observe. A whole-compilation transaction is unnecessary and would broaden retry/live-state scope.

The installation stripe and attempt stripe protect different cuts. An ordinary attempt window writes one exact participant stripe; freeze/close/admission/completion validate and write the complete finite set. A delayed request must precede the cut or refuse. Tracked owner reads alone would not close that phantom. Scalar native locked reads also protect an absent effect identity. Current-source recovered-commit validation uses the complete `ReplayOwnerContext` before durable pending knowledge or reference settlement, including owner kind, attempt, slot and terminal-owner epoch where applicable (lines 1072–1100, 1138–1159, 1636–1655). This prevents another owner's matching request digest from becoming this caller's committed success or released reference. Actual engine collision/recovery qualification remains the integrator's obligation.

`fn::native_immutable_install` compares planned/equal/new rows and holds before deriving distinct changed incidence targets. Equal rows/equal existing holds do not require the same predicate writes as a newly acquired hold. Declared incidence admission is still checked even when all bytes are equal; completed-parent immutability, foreign-attempt refusal and old/new incarnations remain enforced ([schema.surql](../../../crates/lctx-native/src/control/schema.surql), `native_immutable_install`, `native_attempt_participate`, `native_check_admitted`; [coordination.rs](../../../crates/lctx-native/src/control/coordination.rs), lines 388 onward). A new hold over equal content remains a real retirement exclusion event. Operation admission still participates on equal replay.

`WindowWriter` reuses only the original owner's issuance. Each window captures its actual expectations and checks current eligibility; cached era does not grant later permission. Complete wire/control sizing and resident preparation reservations precede issue ([immutable_window.rs](../../../crates/lctx-native/src/control/immutable_window.rs), lines 313–430). This composes with actual-incidence lowering without a global authorization cache.

Receiving retains every successful page's contiguous inventory range and exact request under the real receiving pin. Full row/incarnation comparison and hold acquisition happen per page; only the complete page inventory constructs `TransferredExpected`. Retained selectors/parents and earned contributor manifests are both covered. Receiving copies share immutable preparation while rebinding to the receiver, and do not retain source pin ownership indefinitely ([live_view.rs](../../../crates/lctx-native/src/control/live_view.rs), lines 160–331; [receiving.rs](../../../crates/lctx-native/src/installed_view/receiving.rs); [earned.rs](../../../crates/lctx-native/src/installed_view/earned.rs), lines 151–199).

`NativeReader::holds_installed_premise` selects the actual operation's exact source/received capability and checks local liveness, client, pool and context. It does not treat descriptor or digest equality as authority ([reader.rs](../../../crates/lctx-native/src/reader.rs), lines 1201–1227, 2873–2951). Intermediate receiving failure leaves existing pin/recovery obligations with their owner; it produces no complete receiver. Under the explicit controlled-write boundary, new owners re-earn protection and maintenance mutation requires quiescence/rebuild. This threat-model change buys the removal of repeated immutable full-byte/hold replay; it would be unsound if arbitrary privileged live edits remained supported.

### Shared Base preparation and backup source release

Base discovery captures checked successful native windows, including complete-empty key coverage, before local membership adoption. Sparse ordered routing selects only intersecting windows. Optional coverage compaction preserves the unmerged successful runs when its pure workspace cannot fit; native nominal streams use bounded sequential readers rather than a footer growing with fanout. The latest first-poll deferral prevents an unconsumed prepared read from starting physical DataFusion tasks ([frontier_capture.rs](../../../crates/lctx-compiler-store/src/frontier_capture.rs), lines 49–240; [coverage_index.rs](../../../crates/lctx-compiler-store/src/consumed_rows/coverage_index.rs); [invocation_ipc.rs](../../../crates/lctx-compiler-store/src/invocation_ipc.rs), lines 38–102). Charges and exact sealed owners follow readers to EOF/drop. A failed acquisition does not become successful cached coverage.

`CapturedBase` captures unique rich payload from the first consuming root, then reconstructs a compatible charged typed union. `BasePartition` exposes exact root-selected row views, preserving negative universes and physical-root literals. Read partitions are all prepared before the first mutable root; local preparation can shrink, while mutation/emission failures are terminal. Expression fallback advances `local_start` after each successful emission and retries only unconsumed work ([base_scope/capture.rs](../../../crates/cpg-core/src/base_scope/capture.rs), lines 65–219, 266–365; [base_scope.rs](../../../crates/cpg-core/src/base_scope.rs), lines 154–272, 1458–1526; [semantic_execution.rs](../../../crates/cpg-core/src/semantic_execution.rs), lines 465–595). This makes sharing useful for overlap while retaining a narrower route for disjoint working sets. It does not merge invocation-local mutable read state or replay completed roots.

Backup descriptor acquisition is keyed within **one exact transaction**, sharing validated raw rows/descriptors between dependency discovery and final view export. Root-qualified physical copies remain separately retained. After all source streams reach checked EOF, explicit transaction cancellation/session close and source-pin finalization precede local carrier work. A failed finalizer prevents carrier construction/publication and retains exact uncertainty; a later local carrier failure does not reopen successfully closed native source uncertainty ([selected_backup.rs](../../../crates/lctx-publisher/src/selected_backup.rs), lines 155–237, 332–441; [backup.rs](../../../crates/lctx-publisher/src/backup.rs), lines 80–258, 263–273). These refinements independently address R:F06/F07 and compose with receiving/cuts through existing owners.

## 4. Execution and change scenarios

| Scenario / change kind | Expected propagation and observed route | Architectural assessment |
|---|---|---|
| Add a fact family / domain extension | Model declaration, provider-specific observation and genuinely new scope/coverage semantics; existing identities, codecs and typed partition selection follow their owners | No family-count-driven acquisition at prepare time. An additional reached contextual relation may legitimately extend the complete partition universe |
| Upgrade an analyzer / mechanism revision | Adapter/source-role correspondence and pinned provider context change; downstream semantic contracts admit or reject the new meaning explicitly | Independent providers and immutable completed inputs remain the seam; this work does not relabel typing as runtime flow |
| Sparse query over many unrelated families / instance | Reach only selected namespaces/frontiers; retain wider context only for a rule that needs it | Demand propagation removes unused startup without narrowing negative/ranking semantics |
| Overlap, reconvergence and cycles / workload growth | Share exact native coverage and typed input rows, propagate per-root membership through compact adjacency | Credible sparse/overlap route; cycles do not erase root ownership, complete-empty coverage or once-only effects |
| Disjoint roots and low local headroom / resource variation | Shrink pure preparation from sealed successful sources; singleton must remain viable when its necessary preparation fits | Optional compaction and bounded nominal readers address one pressure issue, but the overbroad acquisition marker still defeats a legitimate pre-issue retry; F02 |
| Concurrent compile/read, freeze/close and retirement / failure | Independent tasks retain joins; atomic windows participate in complete cuts; new holds exclude collection | Physical exclusion is not inferred from read-only checks or pending-row absence; source and host finality remain separate |
| Lost acknowledgement or wrong replay owner / failure | Reconcile the original identity; validate complete owner context before terminal/reference action | Source-supported route through the corrected common boundary; actual concurrent/collision controls remain required |
| Backup source finishes but local carrier is slow/fails / lifetime change | Release exact transaction/pin before independent local work; publish only after complete finalization | Removes an unnecessary native lifetime without requiring an additional backup service |
| Same-residual high-degree Summary / workload skew | Current code still scans proof pairs after queue exhaustion | **Violates A4; F01 below.** Correct residual output alone does not establish a credible examined-work route |
| Substitute execution mechanism | Tokio joined task ownership or a different bounded CPU executor must preserve actual join/context/charges; native/Arrow acquisition can change without changing root meaning | Existing capabilities suffice. No generic scheduler, global cache, new graph backend or neutralized lowest-common-denominator interface is justified |

The in-scope analysis contracts remain explicit:

| Analysis | Universe / method and model | Limits and evidence |
|---|---|---|
| Scope discovery | Typed declared directions and multiplicities, contextual complete partitions, native frontiers | Exact root coverage/absence; resource refusal is not a complete negative |
| Base | Selected root-local typed rows under captured input/context and actual Local owner | Physical/virtual and literal distinctions retained; per-root production/evidence; no mutation replay |
| Summary | Complete selected invocation topology, callee-first SCCs, canonical states and Pareto proof representatives | Finite admitted queue work and qualified residuals; distinct proofs retained; post-exhaustion examined work still needs correction |
| Structural / Analytic consumers | Existing declared projections, exact pair/evidence and governed heuristic products | This track preserves their universes/settings; it provides no new analytic meaning or measured performance claim |

## 5. Findings

<a id="F01"></a>

### F01 — Summary output deduplication leaves avoidable post-exhaustion Cartesian examination

**Priority: material supported-workload defect. Principles: FP-07, DP-10, DP-12; A4 violated. Implemented diagnosis, 2026-10-16.** This refines [R:F09](design_review_catalog-runtime-efficiency-target_2026-10-16.md#F09); it does not create a second independently scheduled Summary redesign. Current disposition routes to [coordinator §6](../../plans/catalog-runtime-efficiency-plan_2026-10-16.md#6-sole-disposition-and-current-checkpoint).

`EnqueueContext::candidates` walks every current proof in the opposite owner's bucket for each newly admitted proof ([summary_production.rs](../../../crates/lctx-model/src/domain/execution/summary_production.rs), lines 1768–1787). `enqueue` checks processed membership and calls `queue.insert` for each candidate (lines 1963–1982). Once the work allowance is exhausted, insertion refuses every nonpending candidate, but each refusal still calls `materialize_residual`. That function resolves event/alternative/target/seed, clones qualification, builds the complete residual tuple and computes its identity before deduplication (lines 1814–1918).

A legitimate shape has **C distinct caller inputs** and **K distinct callee inputs sharing one output**. These are distinct canonical states and can all remain current; proof deduplication does not remove them. After exhaustion, the model's residual tuple depends on caller origin/input, event, phase, callee **output** and qualification, but not callee input/proof identity. The necessary residual product can therefore contain C rows while the executing path still examines C×K proof pairs and repeatedly reconstructs the same keys. Multiple incomparable representatives can add work without adding residual distinctions. The queue's finite member/work limits bound other state; they neither charge this examination nor make it necessary.

The new tests independently compare same/distinct residual outputs and preserve proof inventories (lines 3682 onward), but `constructed` counts only new rich publication after the residual lookup. It is not a count of examined candidates or repeated residual-key construction. Those tests establish a useful semantic preservation boundary, not closure of this remaining execution-fit defect. No historical timeout is attributed to Summary by this finding, and no numerical speed claim is needed.

**Proposed correction.** Keep ordinary deterministic queue admission and every admitted proof/outcome unchanged. Once no new pair can be admitted, consume exact residual endpoint groups, accounting for already pending/processed exceptions, and publish each necessary residual tuple without walking every equivalent refused proof combination. The owning model operation decides the full equivalence key; orchestration should not reinterpret it. A specialized charged endpoint index or lazy grouped traversal within the existing model is preferable to another graph engine or a broad pair cutoff. Keep original proof inventory independently; grouping residual demand is not proof collapse.

Challenge the correction with a legitimate class containing only already processed/pending pairs: it must emit no invented residual. Conversely, one genuinely refused pair in that class requires the residual even if another pair was admitted. Distinct origins, caller inputs, callee outputs, phases, qualifications, alias/path meanings and refusal reasons must remain distinct; recursive calls must retain both directions without duplicate scheduling. Grouping must not move exhaustion earlier, change which initial pairs were admitted, or erase proofs/witnesses.

**Closure evidence.** Inspect the stable grouped implementation and compare against independent finite expectations across same-residual and distinct-residual classes, exhaustion before/after admission, pending/processed-only classes, cycles and incomparable representatives. A focused service-free control should distinguish actual candidate/key work from new output count and demonstrate that equivalent post-exhaustion combinations are skipped. Necessary distinct residual output remains charged and complete. This requires no production accounting framework, new cutoff or semantic rule change.

<a id="F02"></a>

### F02 — Acquisition poisoning begins before the local/native boundary it must protect

**Priority: supported resource-fallback defect. Principles: FP-05, FP-07, DP-08, DP-10; A4 violated. Implemented diagnosis, 2026-10-16.** This refines R:F08's singleton-feasibility obligation; it is distinct from the earlier coverage-compaction correction. Current disposition belongs to [coordinator §6](../../plans/catalog-runtime-efficiency-plan_2026-10-16.md#6-sole-disposition-and-current-checkpoint).

`discover` calls `capture.begin_acquisition()` before `EdgeSource::source(...).await`, resetting it only after all source preparation succeeds ([consumed_rows.rs](../../../crates/lctx-compiler-store/src/consumed_rows.rs), lines 1556–1558). `source` may initialize `prepare_adjacency`, whose fixed/index and contextual staging reservations occur before the first contextual query (lines 584–645, 681–719). `FrontierCapture::read` similarly begins acquisition before calling `issue`, while the forward/reverse `issue` closure still creates selected providers and physical scan plans (lines 1051–1059, 1145–1153; [frontier_capture.rs](../../../crates/lctx-compiler-store/src/frontier_capture.rs), acquisition loop).

These are not all remote-issue boundaries. `NativeTable::scan` constructs schema, filter demand and `NativePartition` without querying the native store ([compiler_provider.rs](../../../crates/lctx-compiler-store/src/compiler_provider.rs), lines 319–420). `NativePartition::execute` constructs lazy streams whose scan begins on polling (lines 442–505). Pure resource refusal during source metadata or scan preparation can therefore leave `permits_local_retry()` false before any relevant native read has started. `CapturedBase::prepare` then refuses to shrink its multi-root group (lines 81–93), even where releasing larger-group local state would let the singleton preparation fit.

The marker correctly prevents unsafe replay once native work may exist; the problem is the distinction it omits. A blanket conservative marker makes a selected pure failure terminal for the wrong reason and loses the declared feasible fallback. Catching every `Resource` error would invert the defect and risk reissuing uncertain work.

**Proposed correction.** Represent or expose the actual prepare/start boundary through the existing provider/capture owners. Pure local failures before any source execution/issued acquisition may narrow. Once source execution or an unknown/failing acquisition is admitted, retain the no-reissue state and original finality obligations until checked EOF/sealing. Provider initialization and contextual queries need the same honest distinction; merely moving the marker to one syntactic `execute` call is insufficient when execution is lazy or a contextual plan starts descendants. No new broad retry policy or recovery owner is needed.

**Closure evidence.** A revealing service-free preparation-pressure case must fail before native start under the larger group, retain zero issue count, narrow, and succeed at singleton size. A complementary failure after actual source start/first polling must prohibit reissue and retain its original ownership/error, including contextual/provider paths, cancellation and incomplete EOF. Inspect the final boundaries against the actual lazy native/DataFusion execution route. Successful captures and complete-empty authority must retain their current protections.

The two findings have independent causes and compatible remedies: F01 groups semantically equivalent refused Summary work; F02 identifies when immutable input acquisition has actually crossed an effect/finality boundary. Neither permits changing queue meaning, retrying uncertain native work or replaying mutated Base roots.

## 6. Library fit and alternatives

Pins were checked in `Cargo.lock`: DataFusion **55.1.0**, Arrow/IPC **59.3.0**, Tokio **1.53.2**, SurrealDB SDK **3.3.0**. The selected capability skills were used as routes and exact local sources were inspected; a current documentation example is not evidence for a different resolved release.

| Capability | Compared routes and complete-operation tradeoff | Judgment |
|---|---|---|
| Demand and reusable closure | Native selected reference/frontier access plus DataFusion contextual sets, ordering and spill; eager all-family preparation; blanket in-memory cache | Selected split fits semantic access. DataFusion `cache()` without a configured cache factory collects partitions into `MemTable` (`dataframe/mod.rs:2412`), so it is not the existing spillable capture owner |
| Joined producer execution | Tokio spawned tasks plus existing borrowed CPU; parent-only futures; separately managed CPU pool/framework | Tokio `block_in_place` suspends sibling futures in the same task but hands off independent tasks (`tokio/src/task/blocking.rs:14`). The selected join registry adds the actual ownership gap; a new executor would also need cancellation/context/charge/finality integration |
| Bounded native effects | Atomic bounded payload/outcome; split remote intent; whole-compilation transaction | One bounded transaction removes the unnecessary independently committed stage. Recovery metadata, absent-request fencing and exact reference settlement remain application obligations |
| Native locking and equality | Scalar locked reads plus bulk compare/install; unchecked insert-ignore or record-array lock substitution | Exact selected server source `238bfeb1` in the named `cb1f97ec…` source checkout implements locked point lookup including absent records (`exec/operators/scan/record_id.rs:276`); dynamic `FOR UPDATE` rejects non-record targets (`scan/dynamic.rs:269`). Preserve this boundary; bulk API availability alone is not semantic equivalence |
| Capture and resource scope | Arrow IPC stream/file forms, existing ordered cursor/index, DataFusion streaming/sort; generic result cache | Stream capture avoids total-fanout footer admission. Sparse routing and bounded merge serve exact successful native coverage; the specialized owner is justified by no-reissue/complete-empty semantics, not by replacing generic serialization/sorting |
| Summary skew | Existing SCC/worklist plus model-owned grouped residual endpoints; post-output dedup; a new graph/query engine | Keep SCCs and proof semantics. The smallest viable correction is exact post-exhaustion grouping; moving the same Cartesian expansion to another engine would not remove the demonstrated work |
| Backup | One-snapshot descriptor map and owned spools; cross-operation cache or holding the snapshot through carrier work | Snapshot-local reuse needs no invalidation framework. Source finalization before carrier keeps recovery scope proportional to native work |

DataFusion `execute_stream` creates the physical plan and begins execution (`dataframe/mod.rs:1601`); therefore first-poll deferral is a meaningful lifetime boundary, not merely an API rename. Its stream-drop cancellation does not itself prove immediate disappearance of every physical descendant, so the focused lifetime control remains material. Arrow `StreamReader` can accept physical EOF (`arrow-ipc/src/reader.rs:1638`); the private sealed carrier additionally checks exact length and explicit EOS. These qualifications support the chosen mechanisms without claiming whole-query RSS or actual server performance.

The historical approximately 143 GB resident-memory observation remains unclassified. Shorter backup source lifetime is a source-established improvement, not an attribution or measured reduction. Warm cache, active transaction state, allocator retention and unreleased ownership require observations from the same authenticated lifetime before selecting a memory remedy. No new cache purge, runtime cap or operational mechanism follows from this review.

## 7. Independent judgments, gates and limits

| Judgment / foundations | Verdict at inspected source strength | Reason |
|---|---|---|
| A1 / FP-01, FP-02, FP-06 | **Satisfied, scoped** | Semantic owners remain stable; provider acquisition, execution placement, native lowering and carrier work have coherent contracts and bounded local reasoning |
| A2 / FP-04, FP-05 | **Satisfied, scoped** | Root roles/coverage, proof versus residual identity, original owner context, receiving inventory and completion axes deliberately govern behavior |
| A3 / FP-03 | **Satisfied, scoped** | Existing DataFusion/native/model capabilities compose; tasks/pins/captures retain real lifetimes without a new scheduler or authority cache |
| A4 / FP-07 | **Violated** | F01 retains unnecessary Cartesian work in a supported high-degree scenario; F02 prevents a legitimate singleton-fit pre-issue fallback. Other inspected routes are credible at source strength |

DP-01–DP-09, DP-11, DP-15 and DP-18–DP-24 are source-supported within the inspected semantic/safety contracts; F02 identifies a material missing execution-phase distinction rather than an unsafe retry. DP-10/DP-12's execution-economy concern remains in F01; no silent truncation or changed residual meaning was established. FP-05 remains satisfied for safety ownership, with F02 requiring finer preparation/start structure for the supported resource route. CI-01–CI-06, CI-10–CI-13 are preserved in the changed boundaries. CI-07 favors the selected native/set/model placement. CI-08's finite/partial distinction remains explicit, but finite admitted state and truthful refusal do not excuse F01's avoidable examined work. Heuristic analytic meanings are preserved; this review does not recertify their enclosing implementation.

| Gate | Verdict | Evidence / limit |
|---|---|---|
| G1 Authority | **pass, scoped** | Model definitions govern semantic membership/proofs; physical preparation remains derived and exact-owner scoped |
| G2 Semantic fidelity | **pass, scoped** | No provider relabelling, root-universe merge, proof collapse or unknown-as-absence path found in inspected changes |
| G3 Validity | **pass, Interface-checked** | Full equality, exact owner/epoch/incarnation checks and complete receiving inventory have observable refusal boundaries |
| G4 Hidden behavior | **pass, scoped** | Acquisition/task/native effects are explicit; selective production's unused-error timing change is declared, with independent admission retained |
| G5 Consistency / recovery | **pass, Interface-checked** | Atomic outcome, absent fence, cuts, durable pending handback, actual joins and source finalization have composed ownership; runtime qualification remains separate |
| G6 Transformation / reuse | **pass, scoped** | Exact dependencies, per-root partitions, source-bound coverage, snapshot-local maps and residual tuple identity preserve meaning |
| G7 Truthful claims | **pass for this assessment** | Source implementation, supplied historical receipts, current testing and measurement are explicitly separated; no speed/memory conclusion is promoted |
| G8 Library leverage | **pass, scoped** | Native atomicity/locking, Arrow carriers, DataFusion sets/spill, Tokio tasks and existing SCC/model kernels are reused; specialized gap ownership has a concrete purpose |
| CI-G1 Fidelity | **pass, scoped** | Coverage, requestedness, qualifications, finite evidence and partial residuals remain distinct |
| CI-G2 Evidence closure | **pass for changed products, Interface-checked** | Root/proof identity and backup complete copy/dependency preservation remain; whole serving journeys are not qualified |
| CI-G3 Evaluation integrity | **n.a. to changed behavior** | No evaluator, protected population, reference oracle or production input-selection rule is changed; no evaluation advantage is claimed |

These gate verdicts concern architectural/source contracts. They do not report an executed native, functional or assembled pass. No positive gate offsets the independent A4 violation.

## 8. Verification, rule impacts and decision

| Evidence / settling obligation | Current boundary |
|---|---|
| Direct source and exact local library inspection | **passed** as an inspection task, 2026-10-16; `git diff 1f8a5537..a5f50a53`, targeted `rg`/`sed` reads. Establishes Implemented / Interface-checked claims only |
| New Summary correction | **not_run** here; required stable-source reassessment and focused independent finite/candidate-work controls are described in F01 |
| Capture preparation/start correction | **not_run** here; required stable-source reassessment and complementary pre-/post-start pressure controls are described in F02 |
| Native collisions, lost acknowledgement, cuts, receiving/retirement and session finality | **not_run** by this reviewer; coordinator-owned actual pinned controls must establish the final-source implementation |
| Compiler/Base/SourceCall/ordinary and recursive Summary consumers; publication/restore; selected Rust serving/tooling | **not_run** by this reviewer; the completion companion retains their exact acceptance requirements |
| Final compile, warnings-inclusive Clippy and applicable leaves / shared-contract acceptance | **not_run** by this reviewer; supplied earlier receipts do not qualify the later corrected source |
| Real-library/operator, embedding and submitted MCP execution | **not_run**, outside assigned runtime verification; no enclosing product qualification follows |

**Additional rule impacts: none.** F01's correction preserves existing queue/admission, residual, proof, partial-result and support meanings. F02 preserves the prohibition on native reissue and identifies pure pre-issue work more accurately. No existing rule must be relaxed. The already selected RC01–RC05 and controlled-write directions are subject changes realized through ADR-0178–0180 and the affected owners, not new approvals created by this review. If grouping required deleting distinct residual/proof products or changing work/refusal semantics, that would be a different recommendation requiring a fresh rule-impact assessment; it is not the proposed remedy.

**Bounded decision: Revise the assembled runtime-efficiency scope at `a5f50a53` for F01/F02.** The other inspected interactions are source-supported and should be preserved during correction. The diagnoses are established by executing structure; their remedies remain Proposed until stable implementation is inspected. The model/analysis and compiler-store capture owners should resolve these boundaries, after which the coordinator integrates and performs RE90's final-source acceptance. Root disposition alone owns whether R:F01–F09 and inherited obligations close.

**Enclosing architecture:** source-supported for the other named ownership/composition scenarios; needs the bounded Summary and pre-issue fallback revisions for full reviewed-scope architectural acceptance. Release qualification, real-library fitness, historical memory attribution and older paused-track completion remain unresolved or unassessed at their stated boundaries. This review adds no schedule, gate, evidence folder or whole-product audit.

## 9. Bounded correction reassessment

**F02 source reassessment, Interface-checked, 2026-10-16, `99f42979`.** The subsequent stable capture correction in `consumed_rows.rs` and `frontier_capture.rs` resolves the inspected structural cause. `provider_with_capture` guards fresh provider earning and reuses cached success. Adjacency metadata reservations precede guarded contextual execution, while contextual query-through-EOF/order/seal remains terminal on failure. Native forward/reverse preparation resolves the actual earned provider, constructs the pure native scan, and calls an explicit start only after local preparation succeeds. Presence planning preserves its exact DataFrame task context. `FrontierCapture::read` marks acquisition immediately before that start; source-start/admission failure remains nonretryable.

The added controls distinguish a real adjacency-metadata reservation refusal, pure plan refusal with already retained successful coverage, and admitted start Resource/Unknown failure. They retain zero reissue of successful keys and prohibit replay after source admission. This is a bounded source acceptance of the correction, **not** an executed test/compile pass; focused and enclosing acceptance remain with the coordinator. The original F02 diagnosis above remains the dated baseline assessment. F01 and the overall `a5f50a53` Revise judgment remain unaffected until the Summary correction is inspected.

**F01 partial correction reassessment, Interface-checked, 2026-10-16, `ad0813ae`.** The new `summary_enqueue.rs` factors exhausted **Call** work using exact endpoint groups. Caller owner/origin/input and callee owner/output, qualified by actual binding attempt, preserve the full dependence of the original call residual. Seed qualification is fixed by caller origin; evidence and proof costs are not residual identity. `WorkQueue::exhausted` is monotonic, so the original per-proof admission prefix remains intact. Pending and processed pairs are checked as exceptions, and a group is marked complete only after finding an actually refused witness. Exception-only groups remain deferred and are reconsidered on a new current proof. The endpoint index retires dominated representatives while retaining every admitted proof separately; the full semantic key includes owner/origin/input/output, so replacement preserves the endpoint group. Recursive calls retain both call directions and share exact group completion. Charges belong to actual progress/component owners. The new controls challenge same/distinct outputs, measured candidate examinations within the control, processed/pending-only exceptions and representative replacement. They were inspected, not executed by this reviewer.

This corrects the original call-bucket Cartesian mechanism, but does not yet support full F01 architectural acceptance:

- `ExhaustedGroups::deferred_for` scans the entire deferred map and reserves a vector allowance from its total length on every repeated endpoint (`summary_enqueue.rs:105`). Unrelated pending/processed-only groups can therefore amplify work and local reservation for repeated proofs whose own group is already complete. The exception population is bounded by ordinary work, but remains a material independent input dimension. Endpoint-qualified primary/secondary access can remove that unrelated scan without changing semantic meaning. The implementation owner acknowledged this gap and queued a correction after the coordinator's frozen checks; no such correction is assumed here.
- `enqueue` still invokes unchanged `path_candidates` after exhaustion (`summary_production.rs:1974`). Alias generation enumerates support combinations while `materialize_residual(WorkItem::Alias)` ignores those support IDs; Path generation similarly combines value/path supports that its residual tuple omits. The inspected `Discovery` keeps vectors of supports per assertion, and native support identities explicitly include run/surface/evidence/provenance (`lctx-model-macros/src/lib.rs:151`), not an enforced singleton per assertion. No singleton invariant was found in the scoped Summary, Flow/Entry and support-admission inspection. The implementation owner is checking the exact supported multiplicities. Full acceptance needs either a governing invariant that eliminates the revealing supported shape, or exact post-exhaustion grouping of those residual classes with the same pending/processed protection. A path/alias cutoff or proof deletion would not be an equivalent correction.

These are remaining manifestations of **F01**, not new unrelated findings or a demand for another engine. Correcting them should preserve the source-supported Call grouping, ordinary admissions and F02's exact pre-issue boundary. **Current assembled judgment at `ad0813ae`: Revise; A1–A3 remain satisfied within the stated scope, A4 remains violated by F01's remaining examined-work routes.** Scoped gate/source judgments remain unchanged. F02's correction is source-accepted. Runtime/compile/functional acceptance remains coordinator-owned and is not established by this reassessment.

### Final source reassessment: `58e6cf67`

**Accept scoped, Interface-checked, 2026-10-16.** Direct inspection of the stable final refinement supports the corrected physical routes for F01/F02. The dated diagnoses and intermediate judgments above remain evidence about their named source; they do not override this current assembled assessment. Current execution disposition and closure evidence still belong solely to coordinator §6.

`ExhaustedGroups::next_deferred` now uses an exact caller-prefix range or the charged callee secondary index. A copied-key cursor advances only through matching groups and remains valid when successful grouping removes the current deferred entry. Completion removes both primary and secondary memberships; an empty reverse bucket is removed. Unrelated exception regions require neither a scan nor a vector/reservation proportional to global deferred state (`summary_enqueue.rs:115–146`). The new control applies full local pressure after populating unrelated regions and distinguishes zero unrelated visits from the exact matching sets, including removal consistency. This eliminates the remaining global-D mechanism without weakening exception ownership or moving a semantic decision into orchestration.

Alias/Path candidate enumeration now has an explicit stop visitor. Before exhaustion it retains the complete original support-tuple order and queue admissions. After exhaustion it visits actual pending/processed exceptions, then stops that entire Alias or Path section only after observing an unexcepted, necessarily refused candidate (`summary_production.rs:1583–1711`, `1950–1987`). Alias residuals use the branch's original call event; Path residuals use the output-site bucket, whose indexed steps all have that same call site. The two sections remain separate because their events can differ. A section containing only admitted exceptions emits nothing. A new improving proof has different candidate identities and therefore reopens examination rather than inheriting another proof's exception-only result. Actual residual tuples retain origin, event, input/output, qualification and reason; support IDs stay in admitted proof evidence rather than being inserted into a residual meaning that never contained them.

The inspected control retains the independently enumerated canonical 32 Alias combinations, adds eight Path combinations and two distinct events, then challenges both an exhausted queue with pending/processed exceptions and the all-excepted population. It asserts full residual/obligation/subject/qualification equality, unchanged queue work/pending identity and retained old/new proof inventory across representative replacement. Candidate visit assertions distinguish expansion from output count. Subsequent fixture-only correction `df99020c` was also inspected: each event now names its actual `OccurrenceOwnership` row, preserving typed event/owner meaning; production is unchanged. These are meaningful controls in source; they were **not_run by this reviewer** and are not reported as executed passes.

A separate cross-proof completed cache is not necessary for this correction's execution fit. The new support route performs at most the actual exceptions plus one refused support witness per section/visited proof, instead of proofs multiplied by support combinations. Repeated representatives may still form a fixed-width residual key. `AssertionQualification` contains IDs and enums, so its clone and the tuple construction do not rebuild rich condition/evidence data. `materialize_residual` performs exact existing-row equality lookup **before** vocabulary/condition publication and obligation/subject insertion. Thus repeated proof visits do not repeat rich residual reconstruction. Adding another completed cache would buy avoidance of fixed identity lookups while introducing another retained index and exception-only reopening contract; no material need was established. Reopen that choice if a supported shape exposes significant remaining work beyond required proof/exception visits.

The final F02 control refinement `32460297` targets the actual adjacency payload reservation through the real resource-pool contract, allowing the earlier `StateCharge` bookkeeping to succeed. This repairs the revealing case rather than weakening the production pre-issue/start contract. The correction does not turn a native Resource/Unknown into a local retry.

| Current architectural judgment | Verdict at `58e6cf67` | Basis |
|---|---|---|
| A1 Localize change | **Satisfied, scoped** | Existing model, compiler-store, execution, native and publisher owners absorb their own changes; the Summary child module is private to the model operation |
| A2 Encode domain meaning | **Satisfied, scoped** | Exact residual equivalence, exception versus refusal, current representative versus retained proof, and pre-issue versus admitted acquisition govern behavior |
| A3 Extend through composition | **Satisfied, scoped** | Existing native/DataFusion/Arrow/Tokio capabilities remain composed through their owners; grouped bookkeeping does not become another scheduler or semantic authority |
| A4 Fit execution to workload | **Satisfied, scoped** | Demand follows necessary scope; joins retain actual lifetimes; native effects/cuts are bounded; Base retries preserve acquisition; exhausted Summary work follows distinct residual groups and actual exceptions; source finalization precedes carrier work |

FP-01–FP-07 are now source-supported for the named assembled scenarios. The original §7 core/profile gate verdicts remain scoped **pass / Interface-checked**, with CI-G3 inapplicable to changed evaluation behavior; no source inspection substitutes for actual native/functional acceptance. **Additional rule impacts remain none.** The fixes preserve queue/refusal, proof/evidence, receiving/admission and no-reissue meanings rather than changing supported semantics or adding a cutoff.

**Current bounded decision: Accept scoped at source strength through `58e6cf67`.** No material architectural issue remains in the inspected runtime-efficiency interactions. The coordinator must still integrate any later repairs, run final-source selected functional/shared-contract and non-functional acceptance, and preserve exact native uncertainty/receipts independently. Those pending checks, historical memory attribution, real-library/operator and excluded runtime journeys prevent any claim of whole-product/release qualification; they do not create an unobserved source defect. A subsequent production change reopens its affected review/acceptance boundary.


## 10. Follow-up source reassessment: `6eb0f01d`

**Accept scoped, Interface-checked, 2026-10-16.** Independent read-only inspection covers fixture
connection finalization (`b3d8ce74`), cleanup cut coalescing (`6040916e`), acknowledged declaration
provenance (`9d706e1b`/`37c6ebcc`), checked audit-pin context (`c681594a`, incorporating
`640b153c`/`8bcf5a66`), and fixture preparation/hydration/borrower corrections
(`7425458f`/`6eb0f01d`). No new material defect was found in those assigned interactions. This
assessment does not certify later changes, the running acceptance campaign or any executed result.

Cleanup coalescing preserves the actual exclusion boundary. The authorization wrapper writes the
cleanup row and its complete incidence before entering the payload. The payload verifies exact
cleanup/origin/owner/page correspondence, reuses only already written predicates, and writes every
remaining distinct hold target before nomination and deletion. Nomination still checks positive
active incarnations and creates the entire retirement page atomically before deleting holds.
Identical-origin reuse removes repeated complete cuts; distinct attempts and nominated objects
retain their cuts. Added controls distinguish same/distinct origins and owners, attempt-valued
roots and late-page rollback. Actual native execution remains pending.

Declaration reuse records acknowledged effects rather than deriving authority from metadata.
The inline marker binds actual operation/recovery ownership and resource pool; reuse also requires
the full known descriptor and exact physical mappings. Production records it only after declaration
and all dependency holds acknowledge. Fresh selector earning still performs independent native
descriptor, membership, alias and incarnation checks. Inline storage uses the existing charged
mapping header rather than another descriptor copy or optional reservation minimum. The pressure
control clears earlier provenance before first minting multiple markers at the full pool ceiling,
and challenges foreign ownership and unequal selection. Its executed outcome remains pending.

[ADR-0181](../../adr/0181-checked-audit-pin-context.md)'s context is earned by a successful post-pin
native comparison, never constructor capture. Reuse retains exact pin/client/pool and closing
checks; admitted workers remain drainable. Weak source references do not extend authority, and
independent receiving copies omit source context provenance. The corrected control isolates the
production live-pin predicate from unrelated attempt blockers. The actual handoff control starts
with a genuine checked token, transfers protection, drops the source, and exercises the receiver's
independent remote/current/hold route. Final receiving context CAS remains separate. These
implementations and controls are source-supported, not runtime-qualified.

Fixture finalization composes operation failure and owner finality before applying the shared
recursive cleanup-permission decision. Acknowledged refusal remains a failure while permitting the
SDK join after handles drop; Unknown or Outstanding still prevents that join. Fixture budget
corrections retain compact preparation in its actual pool while preserving the smaller selected
typed-hydration ceiling. Borrower controls distinguish source-owned adjacency from temporary grain
state, require repeated preparation to plateau, and retain zero charge after the last borrower
drops. These changes clarify the tested boundary without increasing production limits.

A1–A3 remain **Satisfied, scoped**: existing owners retain semantic meaning, effect authority and
lifecycle responsibility. A4 remains **Satisfied, scoped at source strength** for the named
mechanisms: duplicate declaration effects, repeated protected context reads and redundant cleanup
cuts are removed without relocating their guarantees. Core/profile judgments retain their earlier
scope. Rule impact is the approved RC03/ADR-0181 refinement; no additional rule change is recommended.

Selected functional acceptance must still cover cleanup cut/rollback cases, declaration first-mint
and independent receiver earning, post-pin/closing/receiving controls, provider refusal finalization,
and affected scope/budget/borrower controls. Provider timeouts and retry conflicts retain their
original outcomes until actual reruns settle them. The subsequent compiler stack-overflow signal
requires its own diagnosis and correction assessment. This review establishes no whole-product
qualification, historical memory attribution or excluded-runtime result. Coordinator §6 remains
the sole disposition owner.


## 11. Producer launch and destruction reassessment: `0df21c7d`

**Accept scoped, Interface-checked, 2026-10-16.** Independent source inspection of `f16ff276` /
`0df21c7d` supports boxing an already charged concrete producer body before retry, recovery,
tracing and Tokio execution wrappers compose it. Those wrappers carry an indirect future rather
than repeatedly embedding the full producer state. Launch-before-work registration, cancellation,
context rebinding, result delivery and actual join ownership remain intact.

The initial refinement retained task and sender-delivery grants as separate async captures, without
explicit future-before-grant destruction in the never-polled state. The final correction uses the
existing `continuation::retain` carrier around the complete traced small task and both grants.
`WithContext` destroys its inner future under the captured dispatcher before the carrier releases
those grants. The receiver's independent delivery reservation still protects a queued result after
actual task exit. The shutdown sentinel schedules onto an unpolled current-thread runtime, drops
that runtime, observes the full reservation in the producer destructor, requires cancellation on
result/join and eventual zero charge. Large-body refusal and interrupted-cleanup controls preserve
the concrete allocation allowance and actual task lifetime.

This is a source-supported physical refinement within CX01/ADR-0179, with no new semantic rule
impact. Actual normalized-frontier reruns remain mandatory. The normalization launcher still
constructs its concrete body before this boundary, and nested producer polling remains unchanged.
This assessment neither proves exclusive attribution nor establishes runtime closure of the observed
stack overflows. Recurring overflow must reopen attribution rather than prompt stack, concurrency
or deadline overrides. Builds, tests and service operations were **not_run by this reviewer**.


<a id="F03"></a>

## 12. F03 — Nested synchronous handoffs inherit a budget they cannot replenish

**Priority: supported progress/correctness defect. Principles: FP-05, FP-07; A4 violated at the
susceptible adapter. Interface-checked diagnosis, 2026-10-16.** This refines R:F02/E:I08 within
existing compiler execution responsibilities. [Coordinator §6](../../plans/catalog-runtime-efficiency-plan_2026-10-16.md#6-sole-disposition-and-current-checkpoint)
owns its disposition. No semantic rule change is required.

The actual fifteen-control run `20261017T014030.401Z-a8ba5a` passed fourteen bodies, including the
normalized-frontier admission/export journey and provider Resource-negative zero-charge control.
Only the CallScope control timed out. Its whole fixture retained Unknown in attachment
`f339481b-bb73-44a9-adc4-922c69b01b3c`; the successful bodies do not establish enclosing acceptance.
A bounded client-only reproduction `20261017T015345.341Z-504076` retained a second attachment,
`de405073-4543-4c72-8fc6-bb12bf36c789`. No server diagnostics or state mutations were used for
this attribution. The owned stack receipt `20261017T015402.962Z-778cc6` identifies the test thread
parked in `futures::executor::block_on` → `NativeBridge::call<ContentHash>` →
`ProducerOutput::declare_kind<CallDestination>`. The independent bridge actor was idle. An earlier
stack selection observed the actor rather than the test thread; later attempt `015236…6e28f1`
found an already terminated process and supplies no client stack.

Independent inspection of pinned Tokio **1.53.2** supports the mechanism: oneshot receiving checks
cooperative budget before inspecting ready state (`sync/oneshot.rs:1320`); exhausted budget
registers a deferred enclosing-runtime wake (`task/coop/mod.rs:343–397`). The nested synchronous
executor parks while preventing that runtime task from returning control to its scheduler.
Repeated handoffs can therefore stall without native work remaining. This is a progress-contract
mismatch, not evidence that graph traversal or native query execution caused this particular stall.
It does not exclusively explain historical timeouts.

`e6dcefc8` corrects the bridge's synchronous call and launch adapters by wrapping only channel
handoff polling in Tokio's existing `unconstrained` future. Submitted work remains actor-owned and
cooperatively executed; async bridge calls remain cooperative. Cancellation, bounded queue
admission, late diagnostics and explicit drainage retain their existing paths. The wrapper restores
the prior budget after each poll. Moving every synchronous callback to another worker would add
scheduling and ownership machinery without addressing this narrow adapter directly.

The equivalent synchronous `NativeBatches::Iterator::next` still blocks on Tokio MPSC receiving.
Ordinary provider threads avoid an inherited Tokio task budget, but synchronous typed iteration in
Tokio controls is supported. Apply the same adaptation only to that synchronous handoff; preserve
`next_async` and the independent driver. This is another susceptible adapter established by source
inspection, not attribution of the captured CallScope stall. The source judgment at `e6dcefc8`
therefore remains **Revise bounded** until this route is corrected and inspected.

The CallScope fixture correction `608d269b` uses existing awaited declaration/freeze APIs and
preserves all semantic assertions. Explicit teardown checks selected hydration release, original
owner/SDK finality and zero final reservations. Its rerun alone cannot validate the synchronous
bridge correction. Central current-thread regressions must cross the cooperative-budget boundary
with repeated synchronous requests, launches and batches, then establish actual drainage and
retained cancellation/error behavior. Actual CallScope acceptance remains required. These are
functional controls, not timing benchmarks or authority to increase timeouts, stacks or concurrency
limits. No new executor or diagnostic platform is proposed.


### F03 final source reassessment: `2dbc6b0e`

**Accept scoped, Interface-checked, 2026-10-16, including fixture `f8f44159`.** The three synchronous
compiler/provider handoff adapters now exclude only their channel polls from an enclosing Tokio
cooperative budget that the independent blocking executor cannot replenish. Submitted native
operations and asynchronous consumers retain ordinary scheduling, cancellation, ownership and
finality. `NativeBatches::next_async`, its driver and reservation ownership are unchanged.

The new batch control exercises512 actual charged transfers from an independent producer in each
natural-EOF and typed-error variant. It checks order, held transfer charge, original error delivery,
repeated EOF, producer join and final zero charge. The bridge control independently exercises512
requests and512 launches. The bounded sweep of `cpg-core/src`, `cpg-extract/src` and
`lctx-compiler-store/src` found no fourth susceptible production adapter; remaining blocking-entry
matches drive an owned runtime or belong to test-only controls. This is scoped coverage, not a
whole-codebase absence claim.

The CallScope fixture supplies original chunks matching both declared artifacts (`b"call"` and
`b"foreign"`) through the actual native writer. This satisfies strict original-byte readback while
preserving selected-data, malformed-context, unchanged hydration ceiling and explicit teardown
assertions. Its prior fast failure exposed a fixture prerequisite; it did not justify weakening
native readback or relying on shared warm content.

The source remedy for F03 is complete within this inspected scope. Actual batch/CallScope controls
and final-source acceptance remain coordinator-owned; the reviewer's builds/tests/services were
**not_run**. The coordinator's bridge regression pass remains separately scoped execution evidence.
This assessment attributes neither all historical timeouts nor numerical performance changes to
F03. No semantic rule change is required. A1–A4 and the earlier scoped core/profile judgments are
source-supported through this reassessment, with actual enclosing functional acceptance still open.

## 13. Facts producer future boundary reassessment: `9297f305`

**Accept scoped, Interface-checked, 2026-10-17 UTC.** Direct comparison with the preceding source
supports the private `provider_task` extraction in
[`cpg-core::facts`](../../../crates/cpg-core/src/facts.rs). Its explicit `BoxFuture` return places
the complete coroutine's type-erasure boundary inside the owning crate. The existing `.boxed()`
moves into this helper: there is still one allocation at the same scheduling point. `run_provider`
remains inside the async body, so input/output setup, captured-request checks and effects remain
lazy until polling. Readiness checks, provider removal, started-set insertion and pending-future
insertion order are unchanged.

The future owns the same workspace, provider, declaration and captured-run guard. Provider
completion, cancellation, sibling settlement, accepted-ingress drainage and tracing retain their
existing paths. This is a mechanical ownership-boundary refinement; it introduces no new finding,
scheduling policy, semantic rule or recursion-limit override. Earlier architectural judgments and
their named baselines remain unchanged.

Whether this resolves the downstream codegen-only recursion-depth failures requires the affected
rebuild and functional rerun. Ongoing extraction against preceding binaries cannot qualify this
revision. Codegen/runtime acceptance remains pending with the coordinator; builds, tests and
service operations were **not_run by this reviewer**. No performance benefit is Measured.

## 14. Terminal recovery-reference settlement reassessment: `53fb831a`

**Accept scoped, Interface-checked, 2026-10-17 UTC.** Independent read-only comparison with
`53fb831a^` covers `control/coordination.rs`, `control/authorization.rs` and
`control/authorization/recovery_controls.rs`, following their recovery/history consumers and
ADR-0178. No material correctness regression was found in this bounded change. Earlier judgments
retain their named baselines. Subsequent cleanup/pin bulk lowering is outside this assessment.

The preceding installation `ERA_GUARD` literal is byte-identical to the new shared check literal
plus its original stripe revision update. Only `ReleaseRecovery` omits that update;
`RetainCaller`, ordinary apply, reconciliation and complete cuts retain their previous writes.
Marker/schema/generation/era/closed-through/admission correspondence, locked stripe reads and
exhaustion refusal remain shared. Settlement still writes the exact effect/reference predicates
and checks the original terminal outcome and committed owner context before deleting its exact
recovery reference ([coordination.rs](../../../crates/lctx-native/src/control/coordination.rs),
lines 57–65; [authorization.rs](../../../crates/lctx-native/src/control/authorization.rs),
lines 1180–1212). Removing protection cannot introduce another borrower or payload; release-first
therefore does not invalidate a cut's drain proof. Actual cut-race behavior still needs native
evidence. Exact effect predicate writes retain history-collection exclusion.

Durable terminal knowledge still precedes release, and pending metadata clears only after checked
settlement. Failed release retains its actual recipient and Outstanding/Unknown completion.
Cancellation still stops subsequent issuance while awaiting an already submitted terminal.
Closed-era settlement retains current stripe authority and original outcome-era correspondence;
`CompactedTerminal` retains the separate permanent-cut/generation/absent-witness proof
(`authorization.rs`, lines 926–1084, 1121–1146). This change does not rewrite original outcomes or
relax session/source finality.

The new native control shares production settlement lowering and observes its stripe delta inside
the transaction. It checks release delta 0, retention delta 1, foreign-owner/generation refusal,
exact metadata replay and unchanged outcome revision. Error/panic cleanup resumes pending recovery,
releases caller retention and requires owned-session drainage. No probe schema, function or synthetic
persistent authority is added; ordinary resolved effect/owner/predicate history remains, so this is
not a zero-persistence control. Wrong-reference mutation refuses during envelope validation rather
than testing corrupted native reference content. Repetition after a checked commit demonstrates
idempotency, not actual settlement-acknowledgement loss
([recovery_controls.rs](../../../crates/lctx-native/src/control/authorization/recovery_controls.rs),
lines 285–385).

**Runtime and concurrency closure remain pending.** The coordinator must run the new control,
retain lost-apply-acknowledgement/stopped-release and compaction/cross-owner expectations, exercise
release versus installation cut in both orders and closed-era/history overlap, and rerun the failing
native concurrency controls at normal concurrency. Removing this write does not establish remedy
sufficiency for the observed failures. Builds, tests and services were **not_run by this reviewer**;
no measured benefit or enclosing functional acceptance follows. Additional rule impacts: none.

## 15. Cleanup/pin bulk lowering and stopped-release control reassessment

**Accept scoped, Interface-checked, 2026-10-17 UTC.** Independent comparison of `a9a6db1a` with its
parent covers `control.rs`, `control/cleanup.rs`, `control/coordination.rs` and
`control/retirement.rs`. Separate inspection covers the control-only `cf9b97ef` extension and its
SDK decoding correction `967cf1eb`. No material production correctness regression was found;
earlier judgments and findings retain their named source boundaries.

Cleanup preserves exact origin/owner/page equality, physical incarnation/phase checks and the
protected-origin exclusions. Every distinct remaining attempt target still calls `native_touch`,
retaining the full 64-stripe cut and predicate write. Ordinary targets retain the same exact
`native_predicate:[$record]` identity, scalar `ONLY ... FOR UPDATE` read, MAX refusal,
absent-to-one insertion and existing-row increment
([coordination.rs](../../../crates/lctx-native/src/control/coordination.rs), lines 81–95;
[cleanup.rs](../../../crates/lctx-native/src/control/cleanup.rs), lines 186–199). The private
lowering variables do not collide with the inspected enclosing authorization/schema locals.
The unchanged retirement page already uses this scalar-lock/bulk-increment pattern; sharing it
does not import retirement-specific lineage or child-recreation authority.

All nomination guard, job and item checks precede bulk mutations; duplicate-item refusal strengthens
the complete-root count. Job creation, item insertion and exact hold-ID deletion remain in one
owner-effect transaction. Pin release retains its separately committed released fence and derives
deletion IDs solely from selected actual holds belonging to that pin
([retirement.rs](../../../crates/lctx-native/src/control/retirement.rs), lines 247–266, 790–828;
[control.rs](../../../crates/lctx-native/src/control.rs), lines 624–657). Both paths retain complete
owner-effect wire preflight. Cleanup retains its row/byte window check; pin pages remain bounded
to 128 hold IDs. New predicate-row, ID and observed-row arrays add transient native state proportional
to the page: at most `2H+1` pin targets or `3H+1` cleanup targets before deduplication. Wire admission
does not measure that native scratch footprint. Fewer individual predicate/item/deletion dispatches
are visible in source; peak memory, numerical dispatch estimates and speed are not Measured.

The pin control challenges owner/object overlap, a complete foreign-attempt cut, exhausted-predicate
rollback, absent-to-one insertion and surviving foreign holds. Existing distinct-origin/owner cleanup
controls retain foreign-target cuts. Strengthened cleanup refusals cover phase/incarnation/page,
root-count, duplicate-item, predicate exhaustion and late item collision, asserting rollback of jobs,
items, holds, ordinary predicates and origin stripes (`cleanup.rs`, lines 629–720, 722–784, 786–918).
These are source-inspected controls; small cases do not establish maximum-width/large-ID memory
behavior or concurrent overlap.

`cf9b97ef` enrolls an actual receipt-bound successor signal owner. The stopped original token is
never reset: checked parent recovery consumes the original sidecar only after exact settlement,
and new production preparation still refuses. Sidecar paths match the existing recipient/reader
correspondence checks. Cleanup drains the original owned session before completing the successor
receipt. The control models an observed commit followed by stopped release, not transport
acknowledgement loss; ordinary resolved native history remains, so it is not zero-persistence
([recovery_controls.rs](../../../crates/lctx-native/src/control/authorization/recovery_controls.rs),
lines 315–454 at `cf9b97ef`). Coordinator-reported full check `1ea6e1` and focused native selection
`c66542` **failed during build; no tests ran**: the SDK's indexed result does not support
`take<Object>`. The inspected `967cf1eb` correction changes only those two reads to `take<Value>`
with strict `Value::Object` matching and explicit schema refusal; semantic assertions are unchanged.
It establishes neither a successful rebuild nor an executed control pass.

**Runtime acceptance and native concurrency closure remain pending with the coordinator.** No
result is inferred for its subsequent checks. Builds, tests and services were **not_run by this
reviewer**. Normal-concurrency reruns and the existing cut/compaction/collision/finality expectations
remain necessary; these source refinements do not establish remedy sufficiency for historical
failures or enclosing functional qualification. Additional rule impacts: none.
