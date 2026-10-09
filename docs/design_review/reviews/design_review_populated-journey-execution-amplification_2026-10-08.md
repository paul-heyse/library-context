# Populated native journey execution amplification

**Formal design-tier / target review · 2026-10-08**

**Decision: Revise.** The inspected implementation preserves useful semantic and lifecycle boundaries, but its physical composition repeatedly reconstructs an unchanged canonical membership inventory and makes ordinary writes pay for a broad semantic vocabulary. These are structural execution-fit defects. Neither establishes which phase dominates the observed populated-journey duration.

The recommended direction is to preserve typed semantic ownership, exact completed views, independent cold admission and native publication while making physical preparation follow its actual lifetime and demand. Compare a corrected native route with compact immutable prepared inputs and discriminator-directed physical lowering. Reconsider persistence placement or backup transport when their retained benefits do not justify their complete operational burden. A storage pivot, broad orchestration framework, extra proof system or concurrency restriction is not a prerequisite.

## 1. Review contract and evidence boundary

| Field | Scope |
|---|---|
| Subject | Current compiler, native realization, admission, publication, populated serving journeys and backup/restore composition |
| Baseline | HEAD `70b8bf53`; existing uncommitted `STATUS.md` and persisted-execution, persisted-correction and coroutine-compilation plan updates preserved |
| Standard | Repository core principles/template 3.3, efficient-architecture heuristics 1.0, code-intelligence profile 1.5 and library-context binding |
| Reviewer | Fresh independent delegated design reviewer; root coordinator reconciled evidence and publishes this principal review |
| Tier / purpose | Design / target |
| Functional criteria | [API and evidence product target](../../design/sections/api-and-evidence-product.md), particularly §14.9–§14.12; explicit model and operations in [semantic model §15](../../design/sections/semantic-model.md) |
| Workload premise | Compile pinned libraries and original evidence into complete attributed catalog products; support bounded connected evidence/tool answers and complete cold transport. Consider more source records, more semantic kinds, many contributors, skewed scopes and concurrent consumers under the local managed deployment |
| Method | Fresh source/type/document inspection of decisive paths and representative adjacent consumers; attributed existing receipts |
| Excluded qualification | Real FastMCP/live embedding qualification, protected evaluation, operator adoption, product advantage, full-plan acceptance and measured remedy performance |

Current ADRs, implementation plans, pins and working rules describe the subject. They are not the criteria for choosing the best target architecture. Their possible changes are listed in §11 after the architectural judgment.

### What the completed receipt establishes

**Measured operational observation, attributed receipt, local date 2026-10-08 / UTC 2026-10-09:** [run `20261009T023103.223Z-bda1ff`](../../../build/runs/20261009T023103.223Z-bda1ff/summary.json) records `just verify --select serving:mcp` as passed.

- The two native test bodies finished in **1883.02 seconds**, about 31 minutes 23 seconds.
- The native journey step took **2165.532 seconds**, including test-product preparation.
- The complete serving boundary took **2439.61 seconds**.
- The dependent Python selection passed 35 tests in **38.488 seconds**.

The native cases each compile their own catalog. One includes a 4096-dimensional contract embedder, selected exact-neighbour work, ten tools and backup/restore. The other captures four fixture files, uses no embedder and exercises multiple browse scopes. Their definitions are in [native_journey.rs:176](../../../crates/lctx-serving/tests/native_journey.rs) and [native_journey.rs:755](../../../crates/lctx-serving/tests/native_journey.rs).

The receipt has no internal phase durations. It does not establish that both cases individually take more than twenty minutes, that serving itself takes that long, or that embeddings, schema installation, normalization, admission or restore dominate. Passing these journeys establishes their named functional assertions, not production latency, real-library capacity or comparative product value.

Earlier stripped-server sampling and metadata observations in [native-runtime-assessment.md](../../../build/runs/20261008T225907.834Z-dd7ee2/native-runtime-assessment.md) are useful leads. Their executable/source correspondence and causal breakdown remain unresolved. They are not transferred into a current measured hotspot claim.

## 2. Responsibilities, dependencies and fidelity

The implementation has a sound separation between semantic meaning and many effects. The important question is whether physical preparation and coordination also respect these boundaries.

| Owner | Responsibility and contract | Reason for change |
|---|---|---|
| `lctx-model` | Nominal records, graph mappings, identities, qualifications, coverage, operation inputs/outcomes and shared invariants | New domain distinction, policy or semantic operation |
| `cpg-extract` / `cpg-flow` | Pinned independent observations with captured source/configuration/provider context | Provider API or supported observation meaning |
| `cpg-core` | Compose exact completed inputs, selected domain operations and artifact admission | New supported composition or physical preparation route |
| `lctx-surrealdb` | Mechanical schema/body lowering, immutable values/memberships, selection, transport and terminal ownership | Native access path, representation or lifecycle mechanism |
| `lctx-analytics` and model kernels | Named finite graph, transfer and analytical computation | Algorithm or declared projection/model |
| `lctx-publisher` | Derived native edges/search, actual-state reconciliation, sealing and transport | Publication/recovery mechanism |
| `lctx-serving` / PyO3 / FastMCP | Pinned connected hydration, selection/ranking, bounded packets and transport | Product request/delivery policy or adapter contract |

The dependency direction remains mechanism toward meaning. The core can invoke pure finite kernels, and publication is distinct from selection. Those properties should survive the correction.

### Fact and fidelity table

| Family | Authority / provider | Fidelity and uncertainty | Identity and consumers |
|---|---|---|---|
| Syntax, typing and runtime-flow observations | Distinct pinned Ruff, Pyrefly and ty adapters | Provider assertions remain attributed; unsupported attachment and disagreement are explicit | Captured source/provider/context identities feed normalization |
| Normalized receivers, events and bindings | Model-owned normalization operations | Derived assessments preserve alternatives, incomplete correspondence and profile distinctions | Exact completed views feed upper operations |
| Projection snapshots and topology | Model projection declarations and normalization | Exact over the named universe, relation semantics and simplification policy | Source/snapshot identities map canonical IDs to local graph handles |
| Structural and optional analytics | Owned definitions/settings and finite kernels | Structural evidence remains distinct from heuristic ranks, communities and neighbours; unrequested work has explicit outcomes | Frames/invocations/settings link results to admitted inputs |
| Catalog, selection and evidence | Model-owned catalog/selection/evidence operations | Supported, unknown, conflicting, unavailable and not-requested remain distinct | Public members and original evidence feed connected packets |
| Embedding values and projections | Declared actual encoder/input/projection policies | Analytical and discovery meanings stay separate | Full values, projections and uses survive independent restore |
| Published native realization | Admitted semantic graph plus derived physical definitions | Actual physical mapping is reconciled; handle and reader pin complete realization | Tools, original references and cursors remain realization-bound |

No inspected path establishes a new competing semantic authority or silent fidelity loss. This is a bounded source assessment, not renewed qualification of every analyzer or served claim.

## 3. Contracts that constrain a correction

**Implemented, source-inspected 2026-10-08:**

1. A contribution owns declared outputs and exact dependency views. Pending output cannot widen a completed view. Same-key disagreement is rejected.
2. Completing an output establishes immutable membership and explicit outcome; it is different from globally validating every future graph reference.
3. `CheckedInputs` carries actual admitted immutable descriptors. Its `select` and `require_subset` compare scope, attempt, policy and descriptors without rereading rows: [workspace.rs:2112–2180](../../../crates/cpg-core/src/workspace.rs).
4. Reference validation caches exact model/source-view/field/target-view premises, including missing targets, before executing a query: [workspace.rs:1531–1629](../../../crates/cpg-core/src/workspace.rs). It is incorrect to describe all reference checks as repeated full replay.
5. `PreparedGraphs` borrows normalization-owned admission and shares selected topology. It does not rerun predecessor admission for each consumer: [analysis_graphs.rs:76–169](../../../crates/cpg-core/src/analysis_graphs.rs).
6. Native rows remain provisional until terminal completion. Read setup, descendant transports, reservations and submitted calls retain ownership through cancellation and drainage.
7. Ordinary publication seals the compiler’s admitted database directly: [publisher lib.rs:36–55](../../../crates/lctx-publisher/src/lib.rs). The former portable self-import is already absent.
8. Independent cold import/restore and actual physical reconciliation cover uncertainty that internal descriptor reuse does not.

A compact inventory or prepared kernel cannot substitute a digest for semantic admission, weaken a complete universe, omit aliases, interpret graph-local indices as identities, or release charges while work remains live.

Local semantic tests can continue to exercise pure kernels with explicit values. Native access, transport, persistence and recovery controls legitimately need disposable native fixtures. Introducing another backend solely to make these controls smaller would move rather than remove their obligations.

## 4. Complete execution shape

The observed journey comprises several different operations:

| Operation | Necessary work and retained distinction | Inspected physical shape |
|---|---|---|
| Attempt setup | Private owned database and supported physical schema | Installs canonical and compiler schemas for each attempt |
| Providers | Capture/configuration-sensitive observations | Independent native providers write bounded typed batches |
| Normalization | Complete relevant alternatives and canonical source correspondence | Exact completed-input selections, scoped reads and synchronous owned kernels |
| Analysis/catalog | Required semantic predecessors and explicitly selected optional work | Scheduled upper operations, shared prepared topology and declared demand |
| Contribution completion | Freeze actual output membership, counts/content and exact view union | Streams current membership; bounded new-key checks against prior owners |
| Artifact admission | Complete canonical graph universe, roles, original ranges and derivations | Compact lookup headers plus canonical entity/assertion reads |
| Ordinary sealing | Derived edges/search, actual physical agreement, frozen definitions and publication marker | Same database; canonical reads followed by independent reconciliation |
| Tools | Pinned eligibility, connected evidence and bounded delivery | Native selection/hydration and Rust domain operations |
| Backup | Complete current transport under terminal acknowledgement | Selected native SQL export |
| Restore | Independent admission and rebuilt current executable realization | Imported staging database, fresh destination, canonical copy/state admission/search/reconciliation |

The stages have different semantic and assurance purposes. Their mere existence is not a defect. The defects concern repeated physical preparation within them and unnecessary coupling to the total vocabulary.

### Analysis and projection records

- **Prepared topology:** names exact projection assessments, snapshots and chunks; preserves declared direction, multiplicity, isolates and universe. Canonical IDs stay distinct from local graph indices. Selected snapshots are hydrated once for borrowing consumers.
- **Structural computation:** uses declared scoped inputs and owned method semantics. A public output selector cannot shrink the intermediate universe required for a structural answer.
- **Optional analytics:** settings select PageRank, communities, concepts, relational concepts and neighbours. `requested`, `demanded_inputs` and `not_requested` already prevent treating registration as execution: [analytics/build.rs:28–75](../../../crates/lctx-model/src/domain/analytics/build.rs) and [analytic.rs:332–355](../../../crates/cpg-core/src/analytic.rs).
- **Behavioral computation:** finite transfer/Summary contracts and global SCC/proof universes have semantic purposes. Plain reachability cannot replace their transfer semantics.
- **Retrieval and synthesis:** consume admitted identities, original bytes and compatible value policies. Ranking is not evidence truth; final delivery is separately bounded.

This review finds no basis to claim every registered method executes, that all graph hydration is repeated, or that all whole-universe examination is waste.

## 5. Findings

Current execution disposition for these new findings is source-owned below. No implementation has been scheduled by this review. If plan creation schedules a finding, transfer its current disposition to the existing coordinator’s findings table and link back here; do not create a second register.

<a id="f01"></a>

### F01 — Physical schema dispatch grows with the whole semantic vocabulary

**Diagnosis: Implemented, source-inspected 2026-10-08.**

**Remedies: Proposed.**

**Owner:** `lctx-surrealdb::schema`, native installation and reconciliation.

**Principles:** FP-07; DP-03, DP-10, DP-15, DP-16; A4.

`NativeCompilerStore::begin` installs canonical and compiler schemas for every attempt: [compiler.rs:303–362](../../../crates/lctx-surrealdb/src/compiler.rs). `compiler_record_schema` constructs one closed union for the complete nongraph inventory. Canonical entity/assertion bodies likewise use broad unions. `scope_schema` constructs a table-wide array of field/family conditions and filtering for the stored `scope_keys` VALUE expression: [schema.rs:179–219](../../../crates/lctx-surrealdb/src/schema.rs).

Exact SurrealDB 3.3.0 source establishes two relevant mechanisms:

- A supplied non-NONE value does not bypass VALUE: the engine checks its type, evaluates VALUE, then checks the result type. See [field.rs:368–395](https://docs.rs/crate/surrealdb-core/3.3.0/source/src/doc/field.rs).
- `Kind::Either` searches candidate arms with `can_coerce_to_kind` and then coerces the selected arm. See [coerce.rs:729–756](https://docs.rs/crate/surrealdb-expr/3.3.0/source/src/val/value/convert/coerce.rs).

Closed literal objects reject unknown fields, and `__type` gives inexpensive early mismatches for unrelated arms. The source does **not** support per-failed-arm full-body cloning or error formatting. Exact running-server correspondence remains unresolved.

The architectural consequence is independent of the measured duration: adding a semantic family or scope field expands attempt preparation and shared write-time dispatch even for unchanged existing rows. One generated declaration can therefore create broader runtime work without a workload reason. Supplying already computed scope keys cannot remove VALUE evaluation under this mechanism.

**Correction direction.** Keep one declaration authority and exact closed-record validation, but select the physical body/scope program by the actual discriminator and active fields. Compare:

1. A narrower union or physical grouping with useful access locality.
2. Generated discriminator-directed validation and scope lowering.
3. Complete model validation at typed ingress plus independently checked native representation, where database enforcement need not repeat the complete vocabulary.
4. Specialized native fields/tables only when their reduced dispatch outweighs extra layout/index/lifecycle machinery.

A giant sequential CASE, constructing a lookup map on every row, or a table per semantic kind can preserve the amplification in another form. Moving from a closed union to FLEXIBLE object/ASSERT enforcement is not an accepted equivalent: exhaustive tag coverage, closed-field rules, optional NONE/NULL behavior, inactive arms and error behavior need qualification.

**Legitimate countercase.** A newly introduced tagged arm may share a discriminator and require genuine arm-specific validation. Selecting by only the outer type would admit invalid combinations. The replacement must retain the complete local type/arm contract, original-chunk specialization, exact string conversion and independent imported-key reconciliation.

**Closure evidence.** Inspect an extension adding an unrelated family and demonstrate that existing-family write preparation does not expand with the whole inventory. Challenge unknown tags/fields, malformed active arms, NONE/NULL, wrong supplied scope keys and imported physical disagreement through the real selected lowering. Representative measurements are required before claiming elapsed-time improvement.

**Disposition:** Deferred pending the next populated-journey physical-lowering decision. Trigger: plan creation selects schema/write correction or a new family materially expands the shared lowering.

<a id="f02"></a>

### F02 — Canonical-family reads reconstruct the same completed membership inventory repeatedly

**Diagnosis: Implemented, source-inspected 2026-10-08.**

**Remedies: Proposed.**

**Owner:** `NativeCompilerStore` canonical view preparation and `cpg-core` artifact consumers.

**Principles:** FP-07; DP-09, DP-10, DP-20, DP-23; CI-05, CI-08; A4.

[compiler.rs:549–620](../../../crates/lctx-surrealdb/src/compiler.rs) shows each canonical family scan:

1. Enumerates completed contribution owners.
2. Reads their membership pointers.
3. Filters native node family in the caller.
4. Expands required aliases.
5. Sorts/deduplicates the complete physical pointer inventory before payload delivery.

The operation is bounded/spillable and does not hydrate every prior rich payload. Nevertheless, its membership/alias preparation repeats while the completed state is unchanged.

Artifact admission first loads entity/assertion headers through `Lookup::load`, then reads canonical entities and assertions: [native_canonical.rs:45–67](../../../crates/cpg-core/src/native_canonical.rs) and [artifact.rs:1006–1021](../../../crates/cpg-core/src/artifact.rs). That is four canonical-family preparations. Ordinary sealing requests the two families again: [publisher lib.rs:47–55](../../../crates/lctx-publisher/src/lib.rs). The final completed membership state has not changed between these consumers.

An adjacent manifestation is bounded nominal-key membership probing across every selected contributor. `MembershipLookup` constructs candidate-key × owner membership IDs: [compiler.rs:2919–2972](../../../crates/lctx-surrealdb/src/compiler.rs). This protects exact view membership and avoids whole-prior-payload construction, but more contributors increase lookup work independently of matching payload count.

**Consequence.** Complete admission and publication need the complete universe, but they do not need to independently rediscover its immutable pointer union at every header/payload boundary. As records, aliases and contributions grow, preparation, crossings and sorting repeat without adding a different semantic check.

**Correction direction.** Give completed canonical-view preparation a lifetime matching the exact completed state:

- Consume exact completed owners/memberships, canonical family, alias mapping and physical-lowering identity.
- Produce a compact immutable pointer/header inventory or suitable indexed/set-based view.
- Let admission and ordinary sealing select headers or canonical payloads from that same inventory.
- Invalidate on changed completed membership or aliases; exclude pending contributions.
- Preserve complete-empty state, deterministic order, collision checks, charges, terminal success and cancellation/drain.

The inventory may spill; it need not be an eager rich cache. Constructing its key by rescanning the same complete membership at every lookup would defeat the correction. Current owner state and immutable descriptors should supply the validity boundary.

For selected nominal demand, compare the existing deterministic point reads with native indexed set membership and bounded joins. Native `IN` syntax alone does not establish efficient access, bounded deduplication or the correct contributor intersection.

**Legitimate countercase.** Place companions and aliases can belong to the canonical exposure without appearing as direct members of the requested family’s original contribution. A reused inventory containing only direct nodes would silently omit valid identities. The closed inventory and its dependencies must include actual alias expansion. A header cache also cannot be treated as proof that stored bodies or physical edges remain correct.

**Independent assurance retained.** [reconciliation.rs:30–161](../../../crates/lctx-surrealdb/src/reconciliation.rs) examines actual stored canonical/body/scope/edge state. It catches physical disagreement that an internally prepared pointer inventory does not. Cold imports must independently reconstruct and validate their captured membership and model contents.

**Closure evidence.** Show one preparation for a fixed completed state across lookup/admission/sealing, and correct invalidation after new completion. Challenge aliases, overlapping contributors, isolates, empty views, pending rows, same-key disagreement, changed membership and cold corruption. Preserve independent physical readback. Measure complete journeys only to claim runtime benefit.

**Disposition:** Deferred pending plan creation for populated canonical admission/sealing. Trigger: another repeated canonical consumer or selection of the compact/indexed completed-view remedy.

<a id="f03"></a>

### F03 — Contribution completion has an attempt-wide read barrier

**Diagnosis: Implemented composition limitation, source-inspected 2026-10-08.**

**Correction: Proposed for the overlapping-consumer scenario.**

**Owner:** Native operation/read ownership and contribution completion.

**Principles:** FP-03, FP-07; DP-19, DP-20.

`complete_contribution_inner` calls `wait_scans` before completing a contribution: [compiler.rs:1542–1549](../../../crates/lctx-surrealdb/src/compiler.rs). `wait_operations(true)` waits until the **entire attempt’s** scan count is zero: [compiler.rs:884–908](../../../crates/lctx-surrealdb/src/compiler.rs).

The revealing scenario is an independent consumer retaining a stream of an already completed immutable view while another producer completes a distinct output contribution. Pending outputs cannot widen that old view. Yet the unrelated retained stream delays completion because the barrier has attempt scope.

The current sequential compilation driver may avoid this overlap. This finding does not establish that the populated journeys encounter the barrier, or that current completion waits dominate runtime. It identifies the physical dependency a new independent analytic/consumer composition would inherit.

**Correction direction.** Keep terminal ownership for the producer’s actual inputs and descendants, and keep final attempt/seal/removal drainage global. If overlapping independent consumers are supported, distinguish their read lifetimes from the reads required to complete this contribution. Do not simply remove the global counter or presume a returned stream is terminal.

**Legitimate countercase.** A supposedly independent scan can still have an unobserved terminal failure that poisons the attempt. Private contribution completion cannot authorize global publication before that failure is observed. Narrowing the barrier must retain final global failure/drain checks and the producer’s own dependency terminality.

**Closure evidence.** An owned control should retain an unrelated completed-view stream, complete a distinct contribution, then exercise terminal failure and cancellation without publishing a failed attempt. Alternatively, explicitly retain sequential-only internal composition and reopen the boundary when a real overlapping consumer is introduced.

**Disposition:** Deferred. Trigger: introduction of independent overlapping producers/consumers or evidence that a current retained reader blocks unrelated completion. This limitation is excluded from the current sequential A3 acceptance below.

## 6. Runtime placement and analogous patterns

### CPU placement is a separate unresolved operational premise

**Implemented, source-inspected 2026-10-08:** [stage_runtime.rs:7–23](../../../crates/cpg-core/src/stage_runtime.rs) uses borrowed `block_in_place` on a multithread Tokio runtime and executes inline otherwise. Normalization and analytical leaves use it; some yield only after the synchronous closure finishes.

The journey tests use plain `#[tokio::test]`, while the CLI constructs `Runtime::new`: [main.rs:683–689](../../../crates/lctx/src/main.rs). Exact Tokio 1.53.2 contracts distinguish the multithread blocking region from current-thread execution. A sufficiently long inline leaf suspends progress on its calling current-thread runtime until it returns. That does not prove a deadlock, a production defect or a material contribution to this receipt. Providers, bridges and the native server have other runtime owners.

The next decision should make the intended compiler runtime contract and control representativeness explicit. Compare:

- A production runtime owner using available multithread capacity with borrowed synchronous leaves.
- Scoped synchronous execution for finite leaves where concurrent caller-runtime progress is unnecessary.
- Owned blocking execution for suitable work, preserving borrowed-input lifetime and terminal drainage.

[Tokio `spawn_blocking` 1.53.2](https://docs.rs/tokio/1.53.2/tokio/task/fn.spawn_blocking.html) cannot abort an already started closure. Dropping its join handle detaches it. Replacing borrowed execution with spawned blocking work without retaining input/charge/join ownership would regress the existing contract. A fixed CPU/test-thread cap is not a correction.

### Demand already reaches some upstream work

Method-specific consumed-input selection, selected analytic preparation and explicit `NotRequested` outcomes are strengths. `graph_needs` derives a union from scheduled upper stages, and topology loads lazily: [compilation.rs:683–722](../../../crates/cpg-core/src/compilation.rs).

A future consumer-driven route should use the existing owned dependency declarations to distinguish mandatory catalog/evidence support from optional analytics. It must not infer that registration alone means computation, remove availability metadata simply because a method is disabled, or shrink the universe needed for complete selection/negative answers. Whether remaining frontier bookkeeping is material needs a concrete consumer and source/phase evidence; it is not an additional established defect here.

### Restore’s two databases protect a real boundary

[backup.rs:223–270](../../../crates/lctx-publisher/src/backup.rs) imports a current-format trusted local SQL dump into staging, then copies canonical content/state into a fresh realization. The served destination does not inherit imported markers, permissions or functions.

The cost includes staging reconciliation, completed-state examination, fresh schema installation, endpoint and edge passes, original copying, completed-state export/import, independent restored-artifact admission, search reconstruction and final physical reconciliation: [backup.rs:368–534](../../../crates/lctx-publisher/src/backup.rs).

These operations must not be declared redundant just because they traverse the same records. Import changes trust; copy creates another physical realization; search and executable definitions must be current. Endpoint-before-edge ordering also has an enforced-reference purpose.

The larger alternative is **data-only logical transport into one fresh realization**, using existing canonical/state codecs where they fit. Imported data would never acquire executable authority. This could remove staging DDL execution and a complete physical copy, but it requires a deliberate transport contract and independent complete admission. It is Proposed, not a demonstrated replacement or speed improvement. Merely serving the imported staging database would abandon the current isolation guarantee.

### Serving preparation has immutable and request-dependent inputs

**Implemented structure; reuse alternative Proposed, source-inspected 2026-10-08.** `browse` calls `chosen` over the complete library member domain before applying its browse scope: [operations.rs:628–683](../../../crates/lctx-serving/src/operations.rs). `chosen` invokes classification at lines 52–60; classification hydrates declared member inputs, constructs `Prepared`, then applies the request’s selection: [selection.rs:100–136](../../../crates/lctx-serving/src/selection.rs). Browse subsequently reconstructs direct ownership from candidate paths and exposures.

Complete-domain classification can legitimately preserve counts, alternatives and unknown ownership. The candidate correction is to separate immutable, realization-and-exact-member-closure-bound classification/ownership preparation from request-specific selection and presentation. Reuse must preserve analysis contexts, release/capture membership, complete counts and unknowns. Retention must earn its preparation and resident-state costs; this inspection does not establish a material latency contribution or require a cache.

## 7. Change and failure scenarios

| Scenario | Architectural assessment |
|---|---|
| Add an unrelated fact/assertion family | Semantic declaration ownership is good. F01 shows global physical preparation/dispatch growth affecting existing writes |
| Add an analytic over existing completed facts | Existing definitions, consumed inputs and pure kernels support composition. Reuse exact prepared topology; do not manufacture a registry. F03 reopens if it overlaps retained independent reads |
| Substitute a native physical lowering | Meaning remains model-owned; body/scope/index/codec/reconciliation identities must move coherently. No old-format reader is required |
| Substitute a compact Rust prepared view | Preserve canonical IDs, exact source-view dependencies, aliases, direction, parallel relationships and isolates. Graph-local indices remain private |
| Grow unrelated records and contributors | F02 repeats complete inventory preparation; candidate × owner probing is an adjacent growth axis. Small output limits do not bound this examined work |
| High-degree or cyclic scope | Retain the complete intermediate universe and finite transfer/fixpoint semantics. A bounded traversal cannot certify an omitted global answer |
| Concurrent compile/consumer work | Global finalization remains necessary; contribution-specific independence is limited by F03. CPU runtime placement and aggregate live state need explicit composition |
| Cancellation during CPU/read/write | Preserve submitted ownership, first failure, reservations and descendant joins until actual terminality; no detached replacement |
| Corrupt cold import or stale physical body | Independent model/state/physical admission must reject; internal prepared validity cannot waive the trust boundary |
| Backup/restore without an embedder | Preserve exact captured full/projected values and policies; restore does not infer current service values |
| Changed embedding/query/render policy | Preserve separate actual input/value/projection/render dependencies; unchanged semantic facts do not require unrelated reinterpretation |

## 8. Composed library fit

**Interface-checked at pinned versions, 2026-10-08.** Relevant offline capability skills were loaded. Current Context7 documentation supplied discovery through the coordinator’s library research; exact pinned source/contracts govern the claims here. No new library probes were run.

| Capability | Fit and integration judgment |
|---|---|
| SurrealDB 3.3 typed bodies, VALUE and ASSERT | Closed schemas provide real rejection. Broad union dispatch and unconditional VALUE are relevant costs. Discriminator-directed ASSERT/object alternatives need qualified exact syntax, exhaustiveness and closedness; DEFAULT is not derived-value validation |
| SurrealDB indexed/set processing | Existing indexes, bounded bulk operations and point IDs are useful. Compare actual filter/order/deduplication plans for the complete membership operation; syntax or one query does not prove fit |
| DataFusion 55.1 / Arrow 59.3 | Suitable for projected relational input, joins, ordering and spilling. Session table registration is not a scan-result cache; the workspace already shares its RuntimeEnv. Eager DataFrame caching adds a MemTable/live-state burden |
| DataFusion ordering | Native provider order is semantic ID ascending, not snapshot ordinal. The chunk ordinal sort cannot be removed unless another layer actually guarantees that order |
| petgraph 0.8.3 and existing graph kernels | Borrowed graph visit interfaces can reuse compact topology for compatible DFS/SCC/topological operations. Algorithms requiring indexability may need a bounded mapped adapter. Reachability does not replace transfer semantics |
| Existing analytical libraries | Retain qualified named algorithms and semantic settings. Library availability does not justify computing an unrequested method |
| Tokio 1.53.2 / futures | Preserve actual task, closure, acknowledgement and drain ownership. A combinator that drops siblings on failure does not by itself drain detached work |
| Existing codecs and logical transport | Prefer reuse for a data-only backup alternative. Do not introduce another parser/ORM/workflow engine when the residual gap is typed admission and effect ownership |

The present use of libraries does not establish a G8 failure. The issue is their physical composition and preparation lifetime, not a demonstrated wholesale reinvention of an available generic capability.

## 9. Alternatives and recommendation

| Alternative | Benefit mechanism | Burden / legitimate limitation | Judgment |
|---|---|---|---|
| Correct current native route | Reuse completed canonical pointer preparation; select narrow physical demand; improve declaration-derived dispatch | Retains per-attempt native setup and persistence crossings; must preserve cold checks | Strong near-term candidate, Proposed |
| Compact immutable prepared inputs with synchronous kernels | Share topology/indexes/headers at exact completed-view lifetime and hydrate detail on demand | Complete keys, budgets, aliases and source lifetimes still required; an eager rich cache can be worse | Compatible with current semantic/native authority; extend existing preparation rather than build a framework |
| Discriminator-specialized native lowering | Avoid unrelated type/scope branches for an actual row | Extra layouts or ASSERT semantics can add complexity; needs coherent codec/index/reconciliation cutover | Compare explicitly for F01 |
| Consumer-driven execution | Carry actual product/method demand to dependencies and termination | Complete universe, coverage and explicit optional outcomes remain required; some of this is already implemented | Extend only demonstrated residual gaps |
| Different intermediate persistence placement | Keep pure local transformations in operation-shaped values and persist required content/state at meaningful effect boundaries | Can change dependency inspection, recovery and transport; no parallel mutable canonical authority | Larger alternative requiring explicit contract decision |
| Data-only backup/restore | Avoid importing executable metadata and rebuilding through staging before fresh destination | Requires complete logical transport/admission; native SQL compatibility is intentionally replaced | Credible larger alternative, not selected or qualified |
| More concurrency or larger limits alone | May overlap existing work | Retains repeated work and can enlarge live state/contention | Insufficient structural remedy |

These directions are compatible when preparation is derived from one exact semantic authority and effects remain owned. They conflict if each consumer invents its own inventory/cache, a new execution framework duplicates scheduling, or native and compact forms become independently writable meanings.

The next architectural choice should settle the completed-view preparation boundary and native body/scope lowering together. Those remove established amplification without requiring an immediate database pivot. Persistence and restore placement should remain eligible alternatives, judged by their actual inspection/recovery/assurance value. Phase attribution would help order consequential runtime work, but is not required to recognize F01/F02.

## 10. Verification and uncertainty

| Claim | Evidence / outcome |
|---|---|
| Populated native and dependent Python controls pass their existing assertions | **Tested / Measured attributed receipt**, `just verify --select serving:mcp`, local 2026-10-08; bounded to that receipt |
| F01 generation and pinned engine mechanics exist | **Implemented / Interface-checked**, fresh source inspection 2026-10-08 |
| F02 repeated completed-universe preparation exists | **Implemented**, fresh source inspection of producer and consumers 2026-10-08 |
| F03 attempt-wide barrier exists | **Implemented**, source inspection; effect on current journey timing unestablished |
| Runtime phase or server hotspot attribution | **Unresolved**; no internal phase timing, symbolized correspondence or discriminating current capture |
| Proposed lowering/preparation alternatives preserve full behavior | **Proposed**; needs selected-boundary qualification |
| Proposed alternatives improve elapsed time or peak memory | **not_run**; no performance measurement |
| Fresh review builds/tests/probes | **not_run**; read-only assignment |
| Full plan, real library, product differentiation and operator adoption | Outside this review’s qualification; existing owners retain their boundaries |

Useful settling evidence is proportionate: source/known-answer controls for exact inventory and lowering contracts; an owned overlapping-read control if F03 becomes supported; representative complete-phase measurements for runtime selection. No additional ledger, benchmark framework or blanket assurance replay is required.

The following questions identify investigations needed when selecting a remedy. They do not add confirmed failures or change the review verdict.

| Material question | Owner | Settling evidence and trigger |
|---|---|---|
| Which phases dominate populated journeys? | Compiler/publisher journey owner | Attribute setup, providers, normalization, upper operations, admission, sealing, tools and restore separately on comparable runs. Trigger: ordering runtime corrections or making speed claims |
| Does discriminator-directed lowering preserve closed shapes? | Native schema/codec/reconciliation | Inspect and exercise exhaustive tags, unknown fields, active/inactive arms, NONE/NULL and supplied derived-key disagreement. Trigger: selecting F01’s replacement |
| Does indexed membership improve candidate × owner probing? | Native completed-view selection | Inspect actual plans and complete membership/deduplication behavior at the pinned engine; retain bounds and terminal handling. Trigger: replacing current point lookup |
| Do runtime topology and aggregate permits fit the operation? | Compiler/runtime and native-call owners | Establish actual caller, bridge, blocking-worker and server topology; examine overlapping work, admission, charges and cancellation/drain. Trigger: placement change or demonstrated contention; preserve available parallelism |
| Can browse reuse static preparation? | Serving classification/browse | Identify immutable versus request-dependent inputs; compare exact counts, classifications, ownership unknowns and retained-state burden. Trigger: repeated browse preparation becomes material |
| Can data-only restore replace staging? | Publisher transport/admission | Map complete graph/original/state codecs into fresh admission; establish imported executable metadata never gains authority and cold corruption still rejects. Trigger: selecting transport replacement |
| Is a graph-trait adapter required? | Projection and consuming algorithm | Check the selected algorithm’s exact trait bounds and identity/universe preservation. Trigger: a named consumer cannot use the current borrowed view |
| Does batch lowering amplify live intermediates? | Native ingestion/resource owner | Examine simultaneous canonical Arrow, native bodies, decoded graph rows, node/member maps and alias values before loader windows: [compiler.rs:1088–1329](../../../crates/lctx-surrealdb/src/compiler.rs). Establish actual peak ownership/accounting and worthwhile windowing. Trigger: memory-shaped failure or lowering redesign |

Incoming batch bounds apply to the final question. Downstream loader windows alone do not establish the combined live-state bound; this review does not establish that intermediate collections are unbounded or uncharged.

## 11. Rule impacts and dispositions

These are conditional recommendations, not applied changes. Plan creation presents the selected impacts to the operator through the existing decision route.

<a id="rc01"></a>

| ID | Current rule | Proposed change and dependent recommendation | If retained |
|---|---|---|---|
| RC01 | ADR-0133 and semantic model §15.11 select complete native bodies and one generated nongraph backing family; current PC3 schema uses broad closed unions | Permit discriminator-specialized or operation-shaped physical lowering while retaining complete typed meaning and independent physical admission; F01 | Narrow/optimize unions within the current layout; retain residual global dispatch burden explicitly |
| RC02 | ADR-0135 / correction plan §4.1 derive scope keys through stored VALUE and reconcile imported values | Permit a smaller discriminator-directed scope program or qualified typed-ingress plus independently checked representation; F01 | Correct supplied keys still pay VALUE evaluation; preparation alone cannot eliminate it |
| RC03 | ADR-0133 requires every frontier/profile, including artifact-only, to use the native compiler/runtime | Change only if the larger intermediate-placement alternative is selected; retain complete contribution/view/state meaning and a single canonical authority | Compact views remain derived in-attempt preparation over native completed inputs |
| RC04 | Storage/publication §6.1 and `backup.rs` use native SQL transport with staging-to-fresh reconstruction | If selected, replace with data-only logical transport and current fresh realization; independent cold admission and executable isolation remain | Keep two-database restore; optimize its access/preparation without serving imported metadata |

F02’s immutable preparation correction does not require weakening ADR-0133. Narrowing F03’s contribution barrier does not waive global cancellation/sealing/removal drainage. Making runtime assumptions explicit does not require a CPU/thread cap.

All new findings remain **Deferred in this source review** until scheduled. Existing PC/CU/BC findings retain their current owners and receipts; these findings do not reopen a previously corrected nominal-pointer lookup by relabelling it.

## 12. Judgments and bounded decision

### Architectural judgments

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | **Satisfied, inspected scope** | Coherent semantic, mechanism and effect owners; meaningful pure kernels and bounded contracts. Generated operational expansion is assessed separately under A4 |
| A2 Encode domain meaning explicitly | **Satisfied, inspected scope** | Exact nominal identity, attribution, complete views, coverage/outcomes, projection meaning and lifecycle contracts govern the inspected behavior |
| A3 Extend through composition | **Satisfied for current sequential composition** | Owned demand, prepared topology and operation contracts compose. Independent overlapping read/completion is excluded with F03’s explicit trigger |
| A4 Fit execution to the supported workload | **Violated** | F01 expands ordinary physical work with unrelated vocabulary; F02 repeats complete preparation under unchanged state. Timing share and remedy speed remain unresolved |

### Correctness and fidelity gates

| Gate | Verdict | Bounded evidence |
|---|---|---|
| G1 Authority | Pass | Typed model ownership; derived native/Arrow/graph forms; no new competing definition found |
| G2 Semantic fidelity | Pass | Nominal keys, alternatives, qualifications and coverage distinctions remain explicit |
| G3 Validity | Pass | Shared model admission, closed native ingress and explicit conflicts; proposed replacements are not covered |
| G4 Hidden behavior | Pass | Persistence, embedding, publication and transport effects are explicit |
| G5 Consistency/recovery | Pass | Owned attempts, terminal drainage, private failure and publication/selection separation |
| G6 Transformation/reuse | Pass | Exact checked descriptors, shared topology and cold reconstruction preserve the inspected contracts |
| G7 Truthful claims | Pass | This review preserves functional/performance/real-library boundaries and labels replacements Proposed |
| G8 Library leverage | Pass, bounded | No demonstrated bespoke replacement of a clearly fitting established capability; composed execution still fails A4 |
| CI-G1 Fidelity | Pass | Attributed observations, typed uncertainty and heuristic separation |
| CI-G2 Evidence closure | Pass, bounded | Complete graph admission and realization-bound connected serving; existing journey assertions are attributed |
| CI-G3 Evaluation integrity | Pass, bounded | No private oracle input or protected tuning introduced; comparative qualification remains excluded |

These are independent bounded design judgments. They are not a newly executed composite gate.

### Foundations and supporting rules

FP-01, FP-02, FP-04, FP-05 and FP-06 are satisfied within the inspected ownership and contract scope. FP-03 is satisfied for current sequential compositions; F03 identifies its excluded overlapping-consumer scenario. FP-07 is violated by F01/F02.

DP-01–DP-05, DP-07–DP-09, DP-11–DP-12 and DP-18–DP-24 retain their inspected semantic/lifecycle mechanisms. DP-10 and DP-16 are violated where unchanged preparation and vocabulary expansion impose unnecessary physical work. DP-13–DP-15 remain satisfied at the qualified inspected library boundaries; proposed replacement mechanisms still need qualification. Relevant CI authority/fidelity/universe/pinning principles retain their mechanisms, while CI-08’s execution-economy concern contributes to F02.

**Bounded decision: Revise at Implemented structural diagnosis and Proposed corrective strength.** Preserve the current semantic and lifecycle contracts, choose a completed-view preparation boundary and compare a cheaper declaration-derived body/scope lowering. Keep independent cold and actual-state assurance.

**Enclosing architecture:** needs revision for the supported populated/growing compilation-publication workload. The current functional receipt is valuable but does not offset A4. Real-library, release, performance and product qualification remain with their existing owners.

**Priority:** F01/F02 are the consequential current structural corrections. Their integration requires exact completed-state/physical-lowering dependencies before reuse. F03 is a conditional composition obligation, and runtime placement/restore require decisions only where their operational premises become material. No numerical speed ranking is established.

The coordinator owns reconciliation and publication of this review. Subsequent plan creation should make the selected operation consume exact immutable inputs, own one preparation lifetime and produce typed results through explicit effects; consumers should stop rediscovering unchanged membership or interpreting native mechanics independently.
