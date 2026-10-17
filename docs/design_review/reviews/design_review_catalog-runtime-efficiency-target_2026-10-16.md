# Catalog runtime efficiency target review

**Design · target · 2026-10-16**

**Decision: Revise.**

**Source baseline:** `2193bbfc3be4e2c93bf2d5374b3c2f0ba394fc1a`.

The catalog compiler has useful semantic and operational foundations: model-owned meaning, exact completed selections, independent receiving authority, bounded native installation, explicit uncertainty and coherent publication. Its remaining runtime problem is broader than an individual graph algorithm. Physical preparation, ownership checks and durable request handling still repeat work at several nested scopes. Some independent producers also share a polling task with synchronous CPU work.

The recommended target retains those foundations while changing the units of preparation, effect execution and assurance. Establish immutable prerequisites once for their actual protected lifetime, carry them through preserving operations, and perform new checks where ownership or uncertainty changes. Keep each native write window atomic, but reconsider why it needs a separately committed intent and immediate reference settlement. Distinguish actual new incidence from an equal immutable replay before creating shared writes. Release the backup transaction after its last source read.

These are source-grounded structural recommendations. They are not measured speedups, a diagnosis of every timeout, or a claim that the reported service memory is a leak.

## 1. Scope, standard and evidence boundary

The governing standard is [core/template 3.3, efficiency heuristics 1.0 and code-intelligence 1.5](../design_principles/standard.toml), including the repository binding. The functional target is the pinned [API and evidence product](../../design/sections/api-and-evidence-product.md), with the explicit Behavioral analyses and lifecycle operations implicated by this inquiry.

ADRs, plans, source comments and current operating choices describe the reviewed design. They do not constrain the target judgment. Required changes to those choices appear in §8.

The review covers:

- captured providers and native fact construction;
- normalization and completed-input preparation;
- Base, SourceCall, Summary and analogous query-linked analyses;
- construction, independent admission, publication and selected backup;
- native request, pin, recovery and drainage composition;
- cross-attempt interactions and significant allocation lifetimes.

Embedding inference, submitted MCP runtime, new real-library qualification and operator adoption are excluded. Existing embedding-value identities remain preservation obligations where they enter publication.

The workload premise is a complete pinned release with many fact families, overlapping dependency closures, sparse and dense selected roots, highly connected call graphs, several concurrent attempts and interrupted consumers. Sixty seconds screens historical controls for inquiry; it is not an SLA or a new supported-input limit.

Evidence consists of current-source inspection, retained run evidence, the pinned library skill and exact source contracts supplied by the library inquiry. Installation/tool descriptors and their selected build sources were read. No live service inspection, database workload, maintenance, build or test ran for this review. The coordinator's evidence directory is [2026-10-16_catalog-runtime-efficiency](../evidence/2026-10-16_catalog-runtime-efficiency/README.md).

The baseline remained unchanged during inspection. The coordinator's new evidence files are separate from that production baseline.

## 2. Responsibilities, contracts and fidelity

| Owner | Responsibility and authoritative distinctions | Adjacent consumers |
|---|---|---|
| `lctx-model` | Provider-attributed facts; semantic identities; requestedness; complete-empty versus unavailable; graph direction and multiplicity; finite proof and residual meaning | Providers, compiler-store, analyses, publisher and serving |
| `cpg-extract` / `cpg-flow` | Independent captured native observations under declared provider roles and inputs | Facts compilation and producer completion |
| `cpg-core` | Producer readiness, completed-input composition, domain kernels and artifact orchestration | Compiler-store, retained products and publisher |
| `lctx-compiler-store` | Exact native-backed relations, selected scope acquisition and independent semantic admission | Compiler kernels and admitted artifacts |
| `lctx-native` | Physical values, incarnations, ownership, pins, atomic native effects, transport finality and protected recovery | Compiler-store, publisher and serving |
| `lctx-publisher` | Private admitted input to immutable unselected publication; exact selected backup and independent restore | Native readers, export and serving |
| Existing run/service/storage owners | Actual host, executor and retained-resource lifetimes | Verification, recovery and disposal |

The dependency direction remains suitable: orchestration consumes capability owners, while model-owned semantics govern lower physical execution. Neither placing work in SurrealDB nor moving it into a joined CPU task transfers semantic authority.

The consequential fidelity distinctions inspected here remain explicit:

| Family or result | Fidelity and identity | Coverage and uncertainty | Preservation requirement |
|---|---|---|---|
| Provider facts | Captured assertions retain provider, run, source and declared role | Incomplete and unavailable differ from complete-empty | Do not infer absent behavior from missing output |
| Completed contributions/views | Exact logical descriptors select physical copies and immutable membership | Pending output grants no completion or admission | Preserve aliases, copy/incarnation correspondence and actual EOF |
| Base / SourceCall | Derived under model-owned runtime and scope definitions | Negative premises and channel availability remain explicit | Shared acquisition must retain each root's proper universe |
| Summary | Finite proofs, canonical states, proof costs and residuals retain separate identities | Exhaustion produces qualified residual obligations | Do not discard distinct proofs or promote partial computation to complete |
| Published catalog | Immutable admitted selection and executable realization | Publication and operator selection remain separate | References and evidence remain inside the pinned realization |

The physical contracts also have valuable boundaries:

- A new receiving owner earns its own protection; an `Arc` or serialized descriptor is not authority.
- Native retries retain the original request identity, bytes, era and generation.
- A missing outcome does not prove noncommit.
- Statement success, application terminal, physical EOF, session close and executor drainage remain different observations.
- An acknowledged publication survives later cleanup failure as a committed effect.
- Unknown execution retains its exact owner and obligation.

The corrections below preserve these contracts. They challenge repeated physical realizations of the contracts.

## 3. Composition and realistic change scenarios

| Scenario | Relevant composition | Current execution concern | Required target behavior |
|---|---|---|---|
| Add an otherwise unused fact family to a scope model | Model declarations → closure plan → provider acquisition | Preparing the closure earns every provider before demand reaches a rule | New metadata must not automatically create native acquisition for unused relations |
| Several independent ready producers | Coordinator futures → synchronous CPU kernel → native completion | One kernel prevents sibling producer futures in that task from being polled | Independently executable work progresses under joined ownership |
| Two attempts install equal canonical support | Immutable row/hold plan → guards/predicates → native transaction | Equal repeated content can still write shared lifecycle addresses | Only necessary new incidence or state transitions create shared writes |
| Transfer protected contributors to a receiver | Exact source capability → receiving CAS → subsequent reads | Full bytes and source holds are recaptured after checked transfer | Carry the committed receiver protection; independently check only changed premises |
| Many small retained membership demands | Protected selector → demand windows → current checks | Selector/boundary/hold/epoch assurance repeats per demand window | Scope assurance to the protected operation, retaining necessary invalidation checks |
| Backup a closure with shared dependencies | Protected publication → transaction → descriptor traversal → spools → carrier | Repeated dependency reads and transaction lifetime beyond source use | Memoize within the exact snapshot and close it before independent carrier work |
| Many overlapping Base read roots | Closure discovery → captured payload → typed assembly → mutable production | Single-root discovery and repeated rich reconstruction survive shared capture | Share immutable support while retaining one deterministic mutable production per root |
| High-degree Summary endpoints after queue exhaustion | Endpoint buckets → proof pairs → refused queue admissions → residuals | Candidate and residual construction continue outside the queue allowance | Establish a credible examined-work route without erasing distinct residuals |

For Base and Summary, the selector cannot shrink the semantic universe. Nearest/first contextual rules can depend on global partitions; unresolved references, reverse edges and absent physical roots require their existing meanings. A narrower acquisition mechanism is acceptable only when it preserves those rules.

## 4. What the historical evidence establishes

The [slow inventory](../evidence/2026-10-16_catalog-runtime-efficiency/slow-inventory.json) covers 34 managed runs and 47 captured Nextest invocations from the selected CE90 period. It contains 3,669 terminal records and 606 occurrences above 60 seconds: 449 timed out, 130 failed and 27 passed.

Normalization yields 392 distinct captured case keys. These are evidence keys, not independent workloads. For each ever-slow key, its latest captured terminal is retained separately: 260 timed out, 114 failed and 18 passed; ten latest results are now at or below 60 seconds. These historical comparisons do not qualify baseline `2193bbfc`.

All 392 keys have a production-path inquiry route:

| Captured family | Keys | Production route and causal limit |
|---|---:|---|
| `cpg-core::fixture_corpus` | 202 | Facts/provider construction and the selected compiler frontier. Paired-layout controls perform two actual facts compilations; their names do not establish Base or Summary execution |
| `cpg-extract` acquisition, syntax, types, normalized, calls, contracts and evidence controls | 96 | Acquisition/captured providers and native completion. Native-backed reference-oracle paths include actual provider construction and subsequent normalization |
| Other `cpg-core` library, artifacts, analysis, behavior, catalog, facts-admission and synthesis controls | 92 | Shared compilation, completed-input acquisition, native effects, admission and the selected downstream consumer. Individual causal shares remain unresolved |
| `lctx-native::compiler_views` | 1 | Exact selected view/member acquisition and ownership |
| `lctx-publisher::publication` | 1 | Compiler setup before publication/backup, followed by the selected lifecycle if reached |

The original failures, cancellations, timeouts and Unknowns remain original outcomes. A later pass is a separate receipt.

### Latest reconstruction

Run `20261016T190337.280Z-029373` records one passed body and six approximately 900-second timeouts. Host cleanup is confirmed; native obligations are retained. The passed BM25 case took 39.958 seconds.

The [phase summary](../evidence/2026-10-16_catalog-runtime-efficiency/phase-summary.json) and its JUnit source support the following localization:

| Case | Observed production context | What it does not establish |
|---|---|---|
| SourceCall equivalence | Normalization, catalog and `analysis_native_inventory`/native output work occur | The SourceCall kernel caused the timeout |
| Ordinary Summary equivalence | `evaluate_base`; a completed cohort-membership interval reports about 115,407 ms | SCC processing or Summary propagation dominated |
| Recursive Summary equivalence | `evaluate_base`; cohort membership reports about 106,832 ms, then unique-payload capture begins | Recursive Summary itself caused the timeout |
| Base channels | Catalog and analysis-native-inventory output occur | The named Base assertions were reached |
| Paired layout | Facts-only provider work, including Pyrefly completion in a distinct attempt | Base/Summary optimization can explain this control |
| Selected backup | The first Catalog compilation reaches `assess_catalog_frontier` | Export, restore or the backup algorithm caused this timeout |

Phase intervals overlap and are not additive. Terminal observations can include cancellation and shutdown work after the test deadline. Phase counts are not RPC counts, complete transaction counts or independent causal shares. Missing terminals indicate incomplete observation, not proven live work or deadlock.

The latest SourceCall payload, for example, records hundreds of closure-provider waits and over a thousand completed-provider/native-write observations. These agree with a repeated-preparation inquiry; they do not measure the benefit of any proposed correction.

The earlier six passed provider bodies with 94/190/204-second outliers remain body-level evidence because the enclosing fixture failed. The 66-second actual-selection pass is scoped evidence. The stale cancelled 275-case selection remains cancelled evidence.

### Memory observation

The reported approximately 143 GB resident private memory remains unclassified.

The library inquiry verified the default installation descriptor, its selected server generation and corresponding build source, separating these from the disposed E1 campaign. That source defaults RocksDB block-cache capacity to `max(detected_memory / 2 − 1 GiB, 16 MiB)`. Its tracked-memory guard defaults to disabled when `SURREAL_MEMORY_THRESHOLD` is absent. Cache accounting includes shared block/row/blob caches and memtable reservations, but excludes such owners as SST readers, compaction/flush buffers, iterator readahead and open-transaction `WriteBatchWithIndex` allocations. Tracked memory is therefore not whole-process RSS.

Effective overrides, the detected memory value, cache occupancy, active transaction allocations and allocator retention were not observed. Source defaults cannot classify the reported memory as cache, active execution or a leak. Do not transfer SurrealDB 3.3.1 running-query memory-intervention claims to this pinned 3.3.0 realization. Client ingress grants likewise do not bound whole-server RSS. No cache purge, concurrency cap or arbitrary memory-limit remedy follows from this evidence. Exact build/source qualifications and primary links are in [library evidence](../evidence/2026-10-16_catalog-runtime-efficiency/library-evidence.md).

## 5. Findings

Priorities reflect consequence, not implementation order. No P0 semantic-corruption finding is established. All remedies remain **Proposed**; the source mechanisms described below are **Implemented**, inspected on 2026-10-16.

These findings were **Deferred, unscheduled** at review publication. They transferred to the runtime-efficiency plan on 2026-10-16 and then to the [collection-efficiency coordinator §6](../../plans/catalog-collection-efficiency-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint) on 2026-10-17. The latter alone owns current execution disposition. This review retains its dated assessment and source IDs; plan approval does not establish implementation or closure.

<a id="F01"></a>

### F01 — Closure preparation loses useful demand before acquisition

**Priority P1 · FP-07/A4 · H5–H8, H12, H16.**

`NominalClosure::prepare` first resolves every table provider in its plan (`lctx-compiler-store/src/consumed_rows.rs:443–453`). Only afterward does it classify native reference rules versus contextual pair work (`455–510`). It then sorts and writes adjacency preparation (`518–566`).

Consequently, an empty or sparse requested root can pay for provider ports and contextual preparation unrelated to the reached closure. Adding a declared but unused relation expands physical startup. The native reference frontier already demonstrates that a semantic rule does not require eager whole-universe adjacency.

**Correction contract.** Separate stable scope-plan metadata from demand acquisition. Earn native providers when a reached rule needs them. For contextual pairs, compare a frontier-constrained query with shared exact-dependency adjacency when repeated demand warrants full preparation. Every reusable preparation retains its actual source/pin, exact selection, client, resource pool, dependency identity, current context and last borrower. Independent semantic admission remains owned by the existing admission boundary; selective production acquisition does not certify an unexamined input or waive a required whole-input invariant.

Demand earning intentionally changes when an unused input's failure becomes visible: an input never consumed by the production operation need not fail that operation merely because eager setup previously inspected it. Required independent admission must still inspect and reject malformed content in its declared domain. State this error-timing and visibility change explicitly; do not promise identical eager failure ordering. Preserve nearest/first ranking partitions, reverse semantics and unresolved-reference distinctions.

**Alternatives.** Keep full adjacency for dense/repeated consumers; acquire selective contextual neighborhoods for sparse consumers. Choose per physical contract rather than one universal strategy.

**Closure evidence.** Source inspection showing unused providers are not earned; revealing empty/sparse/dense, nearest/first, cycle and unresolved-reference cases. Compare semantic membership and coverage, not only row counts.

<a id="F02"></a>

### F02 — CPU placement and producer scheduling do not compose

**Priority P1 · FP-03, FP-07/A3–A4 · DP-20 · H21–H22.**

The upper/normalization scheduler places ordinary producer futures in `FuturesUnordered` (`cpg-core/src/compilation.rs:1257–1312`, `1488`). `borrowed_cpu` runs synchronously, either inline or through `block_in_place` (`lctx-compiler-store/src/stage_runtime.rs:16–27`).

`block_in_place` permits other runtime tasks to progress. It does not poll sibling futures held inside the same parent task. A long kernel therefore blocks those producers' progress even on a multithread runtime.

This is not a diagnosis of every provider stall. Facts providers already run on owned native threads with explicit joins (`cpg-extract/src/bundle.rs:633–677`). Native fixtures also retain an independent SDK runtime. Those strengths should remain.

**Correction contract.** Move an independently executable stage or CPU kernel into an independently scheduled, joined owner using the existing runtime. Transfer its exact prepared inputs, charges, attempt, producing authority, cancellation and tracing context. The coordinator alone integrates completed results and controls freeze/publication.

No detached work, cancellation-by-dropping or new generic scheduler is required. A synchronous region can remain where it has no independent sibling-progress obligation.

**Closure evidence.** Two ready producers with a gated CPU kernel demonstrate sibling progress; cancellation still joins actual work and preserves borrowed-input charges until the join. Existing facts-provider thread ownership remains unchanged.

<a id="F03"></a>

### F03 — Every bounded native window reproduces a complete durable request lifecycle

**Priority P1 · FP-07/A4 · DP-19–20 · H2, H4, H8, H18–H20, H23–H27.**

Grouped immutable installation already exists. The remaining amplification is outside the bulk insert.

`effect_for_owner` captures guard expectations and rereads owner/era (`control/authorization.rs:593–625`; `coordination.rs:198–286`). `owner_effect_plan` creates separately committed intent and payload transactions (`authorization.rs:799–909`). Successful execution then persists terminal knowledge and settles its temporary outcome reference (`921–1006`, `1038–1080`, `1160–1224`). These files are under `crates/lctx-native/src/control/`.

This entire lifecycle repeats per bounded window, including witness, attachment and cleanup windows. Application recovery and safety have real purposes, but semantic/window boundaries alone do not establish that each physical window needs a separately committed pending intent plus immediate reference-release transaction.

**Recommended target candidate.** Retain the current bounded atomic window and existing `NativeRequest`, `RecoveryScope`, original era/generation, attempt cuts, object incarnations and reference policies. Compare folding effect creation, authorization, stripe participation, payload, committed outcome and outcome reference into **one atomic apply transaction**.

The exact absent-request reconciliation fence remains essential. Existing `reconcile_request` already creates a terminal uncommitted fence for an absent exact request (`authorization.rs:1718–1742`). A delayed original apply must conflict on, or observe and refuse against, that same effect/predicate identity.

The complete behavior must be:

- Persist exact pending recovery metadata before issue.
- A successful transaction commits payload, outcome and required references together.
- A lost response remains Unknown until exact reconciliation observes or fences the original request.
- Reconciliation never regenerates request identity, refreshes payload or infers noncommit from snapshot absence.
- Close/freeze cuts conflict with participating apply transactions: an apply precedes the cut or refuses.
- A late collision rolls back the whole current window; earlier committed windows remain owned.
- Failed reference settlement retains the exact reference and owner.
- Permanent-era compaction retains its current distinction from a known uncommitted outcome.

Temporary reference settlement can also be grouped at a bounded terminal boundary after durable terminal metadata, provided failure retains every unresolved reference.

**Countercase and alternative.** Keep separate intent where its independently visible pending state has a named consumer or necessary recovery exposure. A single transaction spanning extraction, computation or an entire compilation is not the remedy. Nor is a transport batch that hides the same scalar lifecycle.

**Maturity limit.** The single-apply route is technically credible, not a completed protocol design. Its lost-response, reconciliation and cut races must close before adoption. If those cases require the existing split, retain it explicitly and remove repeated owner reads/preparation and settlement work that do not purchase independent protection.

**Closure evidence.** A source-level protocol case table plus revealing native cases for crash-before-issue, unknown COMMIT, delayed apply versus reconciliation, both freeze/close orders, mixed-row collision, reference-release failure and compacted-era recovery. No generic lifecycle framework is required.

<a id="F04"></a>

### F04 — Equal immutable replay still creates shared lifecycle writes

**Priority P1 · FP-07/A4 · H10, H18–H20, H22–H24.**

`fn::native_immutable_install` touches all distinct incidence targets before separating existing equal content from new rows/holds (`control/schema.surql:229–267`). `EXPECTED_OBJECT_GUARD` also advances active guard revisions, except for the admitted attempt special case (`coordination.rs:388–423`). These files are under `crates/lctx-native/src/control/`.

Attempt stripes remove one common write address. They do not remove shared canonical, contribution, completion or endpoint predicate writes. Equal replay can therefore enlarge write sets and expose unrelated attempts to conflicts even where some rows and ownership edges already exist unchanged. Retries repeat the full original window.

This finding does **not** classify every equal canonical row as a no-op. A new owner's hold is a real incidence change and must exclude retirement. A new membership must still conflict with parent completion.

**Correction contract.** Distinguish actual inserted/removed incidence, state transitions and immutable equality checks under the same checked transaction. Touch the exact targets whose protected predicate changes. Preserve full collision comparison and exact incarnation checks. An equal row plus already-existing equal holds should not automatically reproduce mutation traffic.

For a new hold, retain its anti-retirement exclusion. For a changing contribution/completion inventory, retain its close/completion cut. Do not replace necessary scalar locks with a record-ID-array query that the pinned streaming executor does not lock.

**Closure evidence.** Equal replay with unchanged holds has no unnecessary predicate/guard mutations; a new owner's attachment still races safely with retirement in both orders; unequal canonical bytes, later incarnations and completed-parent changes still refuse atomically.

The captured conflict observations identify contention, not the exact conflicting keys or the fraction attributable to this mechanism.

<a id="F05"></a>

### F05 — Receiving and retained reads repeatedly discard established immutable premises

**Priority P1 · FP-07/A4 · H12, H14, H18–H20.**

`transfer_expected_many` atomically checks exact contributor bytes and incarnations and installs receiver holds (`crates/lctx-native/src/control/live_view.rs:157–259`). `protect_receiving_views` subsequently rereads contributor rows/guards and source holds (`crates/lctx-native/src/installed_view/receiving.rs:93–118`).

Retained membership demand also calls `ensure_current` through `members_inner` (`installed_view/live.rs:260–278`). The retained path rereads selectors, compound boundaries, holds, publication/physical witnesses and installation context (`49–163`). Internal earned scopes have a separate borrowed-current path, demonstrating that established protection can be retained without full contributor replay.

The new receiving boundary is necessary. Repeating the same immutable premises after that boundary, or per small demand window, needs a separately identified uncertainty.

**Correction contract.** Carry the exact committed receiver protection and operation-scoped retained premise through preserving reads. A receiving capability must cover the complete exact manifest/closure, every successful page CAS, all required receiving holds, physical copies/incarnations and the current context/definition epoch. It cannot be constructed from only the last successful page or an expected count. Source protection remains held through the complete cross-page handoff; failure preserves already-acquired receiving obligations and unsettled source obligations. Independent receiver admission remains separate. Recheck when these premises can change, when entering a new independent owner, or when the supported mutation threat requires new observation.

First state the mutation exposure. If the product promises detection of privileged arbitrary modification beneath a live pin, an initial CAS cannot replace that protection. Otherwise, the protocol should not implicitly discard immutable validity merely because a value crosses an internal layer.

**Countercase.** A matching logical descriptor, digest or `Arc` from another owner/pool remains insufficient. Independent cold receiving and arbitrary imported data still earn their own admission.

**Closure evidence.** Inspection establishes which mutation classes each retained check covers; an operation with many small membership windows reuses the protected premise; foreign client/pool, stale incarnation, changed context, incomplete EOF and failed transfer remain strict.

<a id="F06"></a>

### F06 — Backup repeats exact dependency-descriptor acquisition inside one snapshot

**Priority P2 · FP-07/A4 · H5, H8, H12, H14.**

Selected backup reads each physical contribution separately, then traverses logical views. It creates a fresh dependency-input map for each requested view and calls `view()` for those dependencies (`crates/lctx-publisher/src/selected_backup.rs:270–355`). A queued dependency is read again when visited. `view_rows` retains prior observations but does not serve these repeated acquisitions.

A shared dependency graph can therefore issue repeated exact descriptor queries inside the same immutable transaction context.

**Correction contract.** Maintain one snapshot-local exact descriptor map and a useful set-demand interface. Batch independent metadata reads where the pinned executor supports it. Preserve exact copy labels, logical/physical correspondence, dependency completeness and unequal-copy refusal.

This memoization conveys no authority across transactions, clients, epochs or independent receiving owners.

**Closure evidence.** A diamond/shared-dependency closure observes each exact descriptor once per snapshot while retaining all selected physical copies; conflicting descriptors and missing dependencies still refuse. Source inspection can close the acquisition duplication.

The latest selected-backup timeout occurred before backup execution, so it is not evidence that this defect caused that timeout.

<a id="F07"></a>

### F07 — Backup retains its external transaction beyond the last database read

**Priority P2 · FP-07/A4 · DP-19 · H17, H23, H27.**

`backup_owned` opens the external transaction, exports selected content into owned spools, appends local metadata, constructs the carrier and validates its receipt before cancelling and closing the transaction (`crates/lctx-publisher/src/backup.rs:143–206`). Carrier construction copies and hashes local sections (`crates/lctx-publisher/src/backup_import.rs:116–147`).

The transaction thus survives pure local archive work after its last source use. The resulting snapshot/session lifetime is unnecessary for carrier construction and can retain backend resources longer than the database operation requires.

**Correction contract.** After the final source stream reaches checked EOF, cancel and close the exact transaction/session. Only after successful finalization construct and publish the carrier from owned spools. Preserve cancellation, structured finalization errors, source pin obligations and staged-output visibility.

Some row serialization belongs inside streaming export. This finding targets work after source acquisition, not a prohibition on files or all materialization.

**Closure evidence.** Source ordering puts exact transaction finalization before carrier copying/hashing; failed cancellation or close leaves output unpublished and preserves the exact Unknown owner; local carrier failure after successful close does not reopen native uncertainty.

No current measurement attributes the reported server memory to this lifetime.

<a id="F08"></a>

### F08 — Base shared capture still rediscovers and reconstructs overlapping support per root

**Priority P2 · FP-07/A4 · H5, H7–H10, H12, H15–H17.**

Base capture invokes closure discovery one root at a time (`crates/cpg-core/src/base_scope/capture.rs:70–84`) and uses per-partition anti-prior queries to find unique payload (`150–162`). Mutable read production then assembles a width-one rich cohort for each captured root (`crates/cpg-core/src/base_scope.rs:1443–1474`).

The dynamic-class field loop also reacquires its class support for each `(class,name)` (`1498–1522`). Shared native capture removes some duplicated payload reads, but repeated discovery, planning and typed reconstruction remain.

The two latest Summary controls reached expensive Base membership acquisition. That localizes real work to this boundary; it does not establish a measured causal share or prove the Summary kernel ran.

**Correction contract.** Reuse compatible root-batch discovery and one charged typed union preparation with root-specific row views where overlap warrants it. Share immutable dynamic-class support across field keys, retaining each field's proper negative universe.

Keep exactly one deterministic mutable read production per root. Shrink or retry acquisition only before mutation; do not replay native or semantic effects after a partially executed production.

**Countercase.** A single high-degree root may not fit a larger union. Retain a charged single-root route and local pre-mutation fallback. Batching is an option, not a mandatory width or concurrency cap.

**Closure evidence.** Overlapping and disjoint roots, cycles, missing physical payload, small-budget fallback and stateful root-order equivalence. Compare root membership, witnesses and negative claims, not only total output.

<a id="F09"></a>

### F09 — Summary's queue allowance leaves high-degree candidate and residual work unresolved

**Priority P2 · FP-07/A4 unresolved for the skew scenario · DP-20, CI-08 · H9–H11.**

Summary already uses a suitable SCC schedule, indexed endpoint discovery, delta-like enqueue, pending-pair deduplication and a Pareto frontier. No evidence supports replacing SCC processing.

Residual skew remains: candidate generation walks compatible opposite-owner proof buckets (`crates/lctx-model/src/domain/execution/summary_production.rs:1768–1785`). When queue admission is exhausted, each candidate can still construct residual qualification and obligation records (`1815–1910`, `1943–1962`).

The queue contract explicitly bounds admission/pop work (`crates/lctx-model/src/domain/execution/summary_worklist.rs:151–198`), not this candidate scan. This is not a misimplementation of that queue limit. It is an unresolved composed execution-fit premise for high-degree inputs.

**Correction direction.** Factor or index join-eligible semantic endpoints and coalesce exactly equivalent residual tuples before repeated expansion, where the model permits it. Preserve all proof identities, genuinely distinct residuals, deterministic partial semantics and evidence closure.

A genuinely quadratic required result remains quadratic. It must be streamed/retained under a credible route or the supported result contract must be deliberately revised. A blanket pair cutoff or witness truncation is not a remedy.

**Closure evidence.** A pure high-degree same-residual case and a genuinely distinct-residual case settle the actual factorization opportunity. Until then, neither avoidable Cartesian work nor a speed improvement is established.

This is an analogous structural concern, not an attributed cause of the latest Summary timeouts.

## 6. Library fit and complete alternatives

| Alternative | Fit and burden | Judgment |
|---|---|---|
| Current engine and protocol, with faster local code | Retains ownership but also repeated preparation, shared mutations and per-window request lifecycle | Useful local improvements cannot close the identified architecture |
| Same SurrealDB realization with demand-shaped preparation, sufficient protected premises and narrower effects | Preserves model/physical boundaries; changes existing owners rather than adding another authority | Preferred target direction |
| Single atomic apply/outcome per existing bounded window | Uses native atomicity while retaining exact reconciliation and cuts; removes a committed protocol stage | Credible F03 candidate, conditional on complete race/recovery closure |
| Larger transport batches with unchanged server work | Reduces crossings but can leave scalar checks, writes and lifecycle stages intact | Insufficient by itself |
| Whole-compilation transaction | Retains snapshots/exclusion across CPU and external work; broadens retry and failure scope | Reject for this target |
| Entirely ephemeral compilation with final persistence only | Simpler for truly private disposable work, but discards supported independent completed-input/product lifetimes if applied wholesale | Selective option, not a general replacement |
| Backend replacement | Could change physical costs, but this inquiry establishes no complete superior replacement preserving current contracts | Unscheduled; not required to remove the demonstrated work |

The inspected workspace pins Tokio 1.53.2, DataFusion 55.1.0, Arrow 59.3.0, petgraph 0.8.3, leiden-rs 0.8.1 and biodivine-lib-bdd 0.6.3. Tokio's exact source confirms F02: `block_in_place` suspends concurrent work within the same task while allowing independently scheduled tasks to progress (`tokio-1.53.2/src/task/blocking.rs:14–19`). Once started, `spawn_blocking` cannot be aborted; runtime shutdown timeout stops waiting without stopping that work (`104–119`). Any such replacement therefore requires cooperative cancellation and an actual retained join before releasing inputs or claiming drainage. These library contracts support changing execution placement; they do not justify a new backend or scheduler. See the pinned [block_in_place](https://docs.rs/tokio/1.53.2/tokio/task/fn.block_in_place.html) and [spawn_blocking](https://docs.rs/tokio/1.53.2/tokio/task/fn.spawn_blocking.html) contracts.

SurrealDB 3.3 already supplies bulk ordinary and relation insertion, exact record reads and atomic transactions. The implementation already uses those capabilities. Canonical immutable installation must not substitute `INSERT IGNORE`: silently skipping a collision does not discharge full canonical equality.

The pinned streaming executor's scalar `ONLY ... FOR UPDATE` behavior matters. A record-ID array is not a proven equivalent lock. Existing retirement code explicitly maps bounded distinct IDs to scalar locked reads before batched comparisons/mutations (`crates/lctx-native/src/control/retirement.rs:760–807`). Preserve that capability distinction when changing F03/F04.

The patched SDK's physical EOF, provisional streaming results and exact-session transaction finalization remain necessary. `Surreal::clone` creates a fresh session ID and queues session cloning (`third_party/surrealdb/src/lib.rs:406–419`); the gRPC lifecycle establishes that separate session (`src/engine/remote/grpc.rs:1747–1759`). Replacing the exact transaction owner with a cloned `Surreal` does not settle its original session obligation. Cloning an `Arc<Surreal<_>>` instead shares the existing client and must not be confused with invoking `Surreal::clone`. Local provenance remains [third_party/surrealdb/README.lctx.md](../../../third_party/surrealdb/README.lctx.md).

DataFusion remains useful for set processing, contextual joins, external ordering and spill. Native indexed frontiers remain useful for selective persisted acquisition. Petgraph SCCs and model-owned transfer/fixpoint kernels remain suitable for Summary. Changing placement does not require changing these semantic owners. The [library evidence](../evidence/2026-10-16_catalog-runtime-efficiency/library-evidence.md) records physical-plan reset limits, MemTable versus spill behavior and exact server-source qualifications.

No new cache registry, universal lifecycle state machine, scheduler, cost-accounting framework, broad source hash or permanent performance gate is justified by this review.

## 7. Prerequisites, verification and uncertainty

Priority is not implementation order:

1. Define sufficient protected-premise and actual-incidence contracts before removing repeated assurance or lifecycle writes.
2. Resolve F03's exact request/reconciliation/cut cases before changing its durable protocol.
3. Preserve demand and joined ownership before broadening shared preparation.
4. Reuse those contracts in Base and snapshot-local backup paths.
5. Settle Summary's skew premise independently; its queue semantics must not be silently broadened or relabelled.

F06/F07 can be corrected independently of the native protocol redesign. F01/F02 can also improve execution without waiting for F03. F04 and F05 share preservation questions but have different closure obligations: one governs writes and retirement exclusion; the other governs retained validity and independent ownership.

No implementation acceptance ran during this review. Builds, tests, native controls and measured comparisons are **not_run** here. Historical commands and results remain in the linked inventory and original run captures.

During later implementation, use source inspection and minimal revealing controls for the changed contracts. Native atomicity, cross-attempt races, lost acknowledgements and exact finality need actual pinned native evidence. Pure closure, scheduling ownership and Summary shapes can first be challenged without a service.

At functional completion, perform the active plan's selected integration and applicable non-functional acceptance once. A genuine publication/backup/restore journey must reach those consumers before claiming their correction. Preserve the existing separation between focused controls, assembled acceptance, real-library qualification and operator adoption.

Material unresolved questions are:

- Which current-source mechanisms dominate each historically slow case? The inventory and phase routes narrow inquiry but do not settle all causal shares.
- Does any real consumer require a separately durable pending intent for every ordinary window?
- Which privileged mutation threats require repeated full-byte assurance beneath a live pin?
- How much candidate work can Summary factor without losing distinct semantic output?
- What portion of the observed service memory belongs to warm cache, active execution, retained transactions, allocator behavior or defects?

These questions do not justify retrying broad workloads or disturbing shared services automatically. They identify evidence needed for consequential decisions.

## 8. Required rule changes

These are proposed directions, not applied authority changes. Plan creation should present them through the existing operator decision route; accepted ADRs remain immutable.

| ID | Current rule/choice | Proposed change and dependent findings | If retained |
|---|---|---|---|
| <a id="RC01"></a>RC01 | ADR-0173 and storage/publication require separate durable intent and execution in the current ordinary effect protocol | Permit the F03 single atomic apply/outcome route where exact pending metadata, absent-request fencing, cuts and references discharge every obligation | Retain split protocol; remove only independently unnecessary reads, preparation and settlement. Do not claim the extra committed stage has been removed |
| <a id="RC02"></a>RC02 | Existing incidence/guard lowering touches every declared target, including equal immutable replay; storage/publication retains complete incidence projection | Define exclusion over actual changed predicates, preserving exact new-hold, retirement and parent-close guarantees; F04 | Preserve required shared writes and report remaining contention exposure; do not weaken exclusion to obtain a pass |
| <a id="RC03"></a>RC03 | Receiving and retained-current source contracts repeatedly reacquire full bytes/holds for already protected immutable content | Specify sufficient operation-owned immutable premises and explicit invalidation/mutation threats; F05 | Keep genuinely independent checks, batch/share their acquisition where possible, and retain their physical cost |
| <a id="RC04"></a>RC04 | Semantic-model §15.1 describes coordinator-owned unspawned producer futures and borrowed synchronous CPU placement | Permit joined independent stage/CPU ownership under the existing runtime while retaining coordinator integration and exact drainage; F02 | Sibling producer progress remains serialized during synchronous kernels; do not claim full independent progress |
| <a id="RC05"></a>RC05 | Base capture's physical choice avoids combining disjoint roots' rich payloads and discovers roots singly | Permit compatible charged union acquisition/typed preparation while retaining root partitions and one mutable production per root; F08 | Preserve single-root acquisition; share only support that does not violate that choice and state the remaining repeated work |

F01, F06 and F07 refine physical work within the existing semantic target. F09 requires a decision only if its eventual correction changes model-owned work/partial-result semantics. Any such change must be explicit rather than hidden in queue mechanics.

The review creates no schedule and does not reconcile or close older Catalog/SA9/efficiency-plan findings.

## 9. Architectural judgments, gates and decision

| Judgment | Verdict | Basis |
|---|---|---|
| A1 Localize change | **Satisfied, scoped** | Model, provider, native storage, compiler admission and publication retain identifiable responsibilities; remedies have existing owners |
| A2 Encode domain meaning explicitly | **Satisfied, scoped** | Requestedness, unknown/absence, exact selection, proof/residual meaning, physical incarnation and finality govern the inspected behavior |
| A3 Extend through composition | **Satisfied, scoped** | Native frontiers, set processing, prepared graph kernels and independent admission compose through explicit contracts; F02 requires better execution placement |
| A4 Fit execution to supported workload | **Violated** | F01–F08 establish unnecessary acquisition, reconstruction, lifecycle writes/crossings or resource lifetime. F09 retains an additional unresolved skew premise |

The positive A1–A3 judgments do not certify unexamined extraction or serving breadth.

| Gate | Verdict | Evidence strength and limit |
|---|---|---|
| G1 Authority | **Pass, scoped** | No competing semantic authority identified in the inspected paths |
| G2 Semantic fidelity | **Pass, scoped** | Consequential identities and uncertainty remain distinct; remedies must preserve them |
| G3 Validity | **Pass, scoped** | Exact capabilities and checked native transitions reject invalid construction in the inspected contracts |
| G4 Hidden behavior | **Pass, scoped** | Effects, independent receiving and snapshot operations are explicit |
| G5 Consistency and recovery | **Pass at Interface-checked strength** | Atomic windows, explicit partial/Unknown outcomes and retained recovery owners are present. Composed runtime qualification remains open |
| G6 Transformation and reuse | **Pass, scoped** | Exact view/dependency identities and independent admission constrain reuse; proposed optimizations are not yet accepted implementations |
| G7 Truthful capability claims | **Pass within this review** | Historical bodies, enclosing failures, timeouts, current source and unmeasured recommendations remain separate |
| G8 Library leverage | **Pass, scoped** | Bulk insertion, native frontiers, DataFusion and established SCC/worklist mechanisms are already used. The remaining custom safety work cannot be dismissed merely because the engine offers transactions |
| CI-G1 Fidelity | **Pass, scoped** | No relabelled relation, heuristic-as-fact or unknown-as-absence identified in the reviewed contracts |
| CI-G2 Evidence closure | **Pass, scoped** | Exact selected publication/evidence boundaries remain explicit |
| CI-G3 Non-circular evaluation | **Not applicable** | Evaluation construction and protected-reference use are outside this inquiry; no evaluation acceptance is asserted |

FP-01–FP-06 are source-supported within these inspected ownership and semantic boundaries. **FP-07 is violated** by the demonstrated execution amplification. DP-19/DP-20 require the narrower effect/resource lifetimes and composed progression described above. CI-08 remains a preservation obligation and an unresolved execution-fit question for F09; the existing queue allowance must not be represented as a bound on all candidate work.

**Decision: Revise the runtime target.** Preserve the semantic model, exact owner capabilities, independent receiving, coherent publication, original failures/Unknowns and physical-finality protections. Change preparation, request granularity, shared write sets and assurance lifetimes through their existing owners.

**Enclosing architectural status:** needs revision for the supported growth/concurrency/lifecycle scenarios. This is a source-grounded target judgment, not release qualification, measured performance acceptance or a complete causal explanation of every retained timeout.
