# Unified persistent SurrealDB: independent design and target review

**Decision: Revise.** A single durable SurrealDB service is a sound direction for the requested workflow, but keeping the current server alive would not deliver the target. The present compiler, publication and test contracts use a private database as the unit of attempt isolation, content ownership and sealing. Product-cache hits still decode and replay canonical rows into fresh contributions. These mechanisms preserve important guarantees, but also preserve repeated setup, copies and reconstruction.

The recommended direction is **Proposed**: one durable library-context service and shared canonical content, with completed views, publication, reader pins and attempt effects represented inside a small stable database arrangement. Reuse should attach compatible immutable completed content through fresh checked bindings rather than reconstructing it merely to acquire a new attempt owner. Domain validity, current provenance and independent cold admission remain necessary. Database-wide isolation must be replaced deliberately; removing it without equivalent view-relative enforcement would weaken correctness.

This review does not establish a complete replacement design, a measured speed improvement or runtime qualification. It identifies the architectural decisions that must be resolved before the unified system can be accepted. PSE-arrow is a reference, not a proposed shared cross-project semantic store.

## 1. Scope, standard and evidence boundary

| Field | Assessment |
|---|---|
| Subject | Compiler persistence, completed views, publication, native serving, product reuse, verification fixtures, worktree attachment, recovery and retention |
| Baseline | Initially clean 05cc139b8d126c7e770f238c79eec9d39769e3b6, inspected 2026-10-09 |
| Standard | Core/template 3.3, efficient-architecture heuristics 1.0, code-intelligence profile 1.5 and repository binding |
| Tier and purpose | Design / target |
| Reviewer | Independent delegated design reviewer |
| Functional target | One durable SurrealDB system across production, tests and agent worktrees; shared canonical data; reuse of persisted facts, results and state; fewer repeated derivations, copies and lifecycle mechanisms |
| Deployment premise | One operator with concurrent compilers, tests, serving clients and worktrees; compatible revisions coexist; a few stable logical databases are permitted when they express genuine responsibilities |
| Exclusions | Implementation, builds, tests, probes, service operations, operator activation and interference with running qualification |

The examined baseline includes the semantic-model and storage/publication owners, persisted-execution coordinator, GK/GR and native-efficiency plans, storage-lifecycle plan, and decisive Rust and fixture source paths. Concurrent agent-effectiveness and lifecycle edits appeared during review. Those later changes are **not reviewed** here; source findings refer to the initial baseline. The coordinator's limited diff inspection found the concurrent fixture change wraps subprocess ownership rather than changing namespace/database provisioning. This review does not reopen or duplicate AF1's cleanup findings.

The workload includes repeated compilation of unchanged pinned inputs, changed analyzer or model revisions, selective catalog and evidence requests, global graph analyses, high-degree outliers, concurrent test execution and recovery after interrupted writes. Small returned packets do not imply small examined work. Shared persistence must serve these shapes without reducing ordinary compiler or test parallelism to make controls pass.

Prior reports supplied navigation leads. The judgments below rest on current inspected contracts and source. Existing qualification receipts retain their original dates and source boundaries; neither an accepted ADR nor an earlier binary's pass qualifies this target.

## 2. What should remain, and what must change

The current design has useful semantic boundaries. lctx-model owns nominal identities, relation declarations, provenance, coverage, projections and operations. CompletedContribution, CompletedView and CompletedBinding explicitly distinguish producer semantics, exact membership and frozen boundaries. Native reads exclude pending output. Product dependencies retain membership and absence premises. Publication is separate from selection, and served resources and continuations carry realization identity.

These are preservation constraints, not reasons to retain a database per attempt.

| Responsibility | Current owner and contract | Required target boundary |
|---|---|---|
| Domain meaning | lctx-model declarations and operations | Remain authoritative over shared storage, attachment, validation and serving |
| Canonical content | Private compiler database and native codecs | Immutable content with explicit model/revision compatibility and exact view membership |
| Mutable attempt | Workspace plus native operation guards | Durable attempt effects and fencing, independent of service and completed-content lifetimes |
| Completed inputs | Exact contribution/view/binding descriptors | Reusable checked content, bound to a current consumer without inheriting another attempt's capabilities |
| Publication | Admission followed by database sealing and marker | Short atomic publication of an immutable manifest/read set and executable realization |
| Serving | Database VIEWER and fixed snapshot handle | View-scoped access, evidence closure and definitions pinned for the whole consumer journey |
| Tests/worktrees | Disposable fixture attachment and private databases | Attach to stable service/schema; own logical inputs, attempts and effects |
| Retirement | Database removal or cache retirement | Native owner retires unreachable content under pins, obligations and recovery fences |

The necessary dependency direction remains: orchestration consumes domain operations; native storage realizes their effects; serving consumes a pinned published realization. A service supervisor should own installation, readiness, durable placement and maintenance admission. It should not become another semantic interpreter or generic workflow engine.

**Proposed physical boundary.** Keep canonically linked records and the guards/manifests needed for atomic attachment, publication, pin acquisition and retirement together by default. Semantic ownership does not require separate canonical, control and attempt databases. A separate stable validation database or replaceable cache can be justified by its different mutation/trust contract; it must not require a second authoritative commit protocol merely to preserve an organizational split.

Version-matched core 3.3.0 source supports an authorized multi-statement transaction that changes database selection with USE while retaining the same transaction. This is **Interface-checked**, not a qualified running-server guarantee. Separate sessions or transactions do not acquire that property. Native record IDs and ENFORCED role endpoints remain database-local, so cross-database atomic writes do not supply cross-database graph references or joins. Co-location is the simpler default; a split must name its native transaction, principal and explicit qualified descriptors before acceptance.

### Fact and fidelity preservation

| Family | Provider or authority | Fidelity and uncertainty | Identity and consumers |
|---|---|---|---|
| Syntax and lexical observations | Pinned independent Ruff integration | Extracted, with recorded source/context and coverage | Nominal source/occurrence identities; normalization and catalog |
| Typing and runtime-flow observations | Pinned Pyrefly and ty integrations | Provider-specific assertions; disagreement and unsupported scope retained | Attributed observations; resolution and behavioral operations |
| Resolved catalog relationships | Model-owned normalization/catalog operations | Derived under declared policies and exact inputs | Qualified relationships and evidence; discovery and serving |
| Structural and behavioral results | Declared projections and finite model kernels | Exact or conservative under stated models; unknown and NotRequested distinct | Result/proof identities and consumed views |
| Communities, ranking and vectors | Selected analytic/embedding specifications | Heuristic or numerical values under explicit settings | Product/specification identities; ranking and navigation |
| Served assertions and original evidence | Programmatic synthesis and pinned realization | Claims close over supporting facts; no stronger interpretation | Snapshot-bound references, resources and continuations |

Shared storage must not collapse provider assertions, computational equivalence and current attribution into one "cached result." The existing distinctions are materially stronger than a generic graph of nodes and edges.

## 3. Material findings

### <a id="F01"></a>F01 - Service, attachment and attempt lifetimes remain coupled to disposable data

**Principles:** FP-01, FP-06, FP-07; DP-10, DP-19; A1, A3, A4.

**Implemented diagnosis.** NativeCompilerStore::begin_inner constructs a clock/process-derived snapshot database, creates it STRICT and installs schemas for each attempt. abandon removes that database or reports it as an orphan. Fixture attachment creates a new namespace plus core/cache/product databases and scratch coordination state; release normally removes the namespace. Keeping the server process alive therefore does not keep a shared reusable compiler universe.

Evidence: [compiler.rs](../../../crates/lctx-surrealdb/src/compiler.rs), lines 505-607 and 1447-1472; [surrealdb_fixture.py](../../../scripts/surrealdb_fixture.py), baseline lines 869-925 and 1040-1053; [storage/publication section 6](../../design/sections/storage-and-publication.md).

**Consequence.** Repeated worktrees and controls continue paying schema/setup and fresh-ingress costs. A failed client lifetime still owns deletion of a whole content container. Service persistence and reusable data persistence remain different mechanisms.

**Proposed correction.** Separate durable service/storage ownership from client attachment and mutable attempt ownership. Ordinary attachment should validate compatibility and acquire a scoped capability; it should not install schemas or allocate databases. Attempt completion or cancellation releases its effects and pins while preserving independently retained completed content.

A few stable databases are permitted only for concrete physical/trust responsibilities, as qualified in section 2. No scratch-server or per-test-database fallback satisfies this target. Durable state and coordination placement should survive checkout deletion; a stable host application-state location is a reasonable candidate. Client build provenance is distinct from selected server/schema generation. Existing native-extension/environment guards retain their owner; copying PSE-arrow's whole receiver or host-scheduler framework is not justified.

**Closure.** Two independent worktrees attach to the same existing contribution without server/schema creation or canonical replay; closing one leaves the other's pin and reusable content valid. Restarting the same service/storage preserves admitted content. Scope remains unscheduled pending target-plan creation.

### <a id="F02"></a>F02 - Canonical keys and native role enforcement assume a database-scoped universe

**Principles:** FP-04, FP-05; DP-02, DP-03, DP-04, DP-07; CI-03, CI-05, CI-11; A2; G2, G3, CI-G2.

**Implemented diagnosis.** Canonical entity, assertion and backing tables enforce uniqueness on (semantic_type, semantic_key). Native role edges enforce endpoint existence in the database. Exact compiler memberships provide a separate input boundary, but the private database supplies broader isolation for canonical graph, originals and published search.

Evidence: [schema.rs](../../../crates/lctx-surrealdb/src/schema.rs), lines 155-183; [completed.rs](../../../crates/lctx-model/src/domain/completed.rs), lines 42-64 and 135-153; [semantic model sections 15.2-15.3](../../design/sections/semantic-model.md).

**Consequence.** Replacing private databases with an attempt filter is insufficient. Endpoint existence somewhere in a shared database does not establish membership in the selected view. Different model/layout revisions also need an explicit rule for nominal keys and payload compatibility. A changed payload under a retained nominal key must be either a legitimate separately represented revision or a defined conflict; global storage uniqueness must not make that decision accidentally.

**Proposed correction.** Define immutable content identity, semantic identity and view membership separately. Shared storage must resolve typed references under the same exact view and model contract, preserving isolates, parallel relationships, source associations and original chunks. Within one view, conflicting definitions still fail. Across views, accepted revision distinctions need explicit physical mapping.

Possible realizations include closed immutable segments with compact membership manifests, or content-addressed rows with indexed view-relative mappings. Neither is accepted merely by naming it. Native role projections must retain their promised meaning without requiring a full duplicate graph per view. Share only where equality and reference interpretation permit; do not weaken same-view conflict detection to increase deduplication.

**Closure.** Inspect and exercise same-key/equal-payload reuse, permitted revision variation, incompatible conflicts, foreign-view endpoints, isolates, parallel arcs and missing original evidence. A globally present endpoint must not repair an invalid selected view. This is a correctness prerequisite for sharing, not an observed corruption in the current isolated design.

### <a id="F03"></a>F03 - Publication, permissions and executable pinning cannot remain database-wide

**Principles:** FP-02, FP-04, FP-05; DP-19, DP-24; CI-13; A2, A3; G5, CI-G2.

**Implemented diagnosis.** Publication fingerprints effective database/table definitions, creates a database VIEWER and inserts publication:current. SnapshotHandle combines semantic content, realization and database identity. This works with one sealed database per realization. A stable shared database contains concurrent unpublished content and may serve several compatible executable revisions.

Evidence: [native_publication.rs](../../../crates/lctx-publisher/src/native_publication.rs), lines 169-236; [realization.rs](../../../crates/lctx-surrealdb/src/realization.rs), lines 48-104; [serving identity](../../../crates/lctx-model/src/domain/serving/identity.rs), lines 10-37; [storage/publication sections 6.1-6.2](../../design/sections/storage-and-publication.md).

**Consequence.** Database VIEWER credentials alone no longer imply a single published universe. Unrelated schema/function changes can invalidate a database-wide fingerprint, while in-place definition replacement can change an older reader's meaning. Publication cannot freeze all unrelated writers each time one attempt completes.

**Proposed correction.** Publish an immutable manifest containing exact content/read-set membership and answer-affecting definition identities. Freeze and reconcile the publishing attempt's relevant effects, then make that manifest visible through a short atomic transition. Selection remains separate.

The access boundary must enforce the pinned view for every query, resource, evidence fetch and continuation. Native record access or restricted functions may discharge part of that contract; a trusted Rust request boundary may discharge another part. Arbitrary administrative SQL must not be presented as snapshot-scoped access. Versioned executable names or another explicit immutable-definition mechanism must permit compatible worktree revisions without silently changing old pins.

**Closure.** Publish B while A remains readable; A's references, definitions and continuations retain their meaning. Pending B content is inaccessible through A. Unrelated compatible content does not force A's re-attestation. Incompatible engine/schema changes require an explicit service cutover rather than silent fallback.

### <a id="F04"></a>F04 - Persisted reuse remains value replay rather than reusable completed authority

**Principles:** FP-03, FP-07; DP-09, DP-10, DP-16, DP-23; A3, A4; G6.

**Implemented diagnosis.** GR section 4 deliberately treats products as derived acceleration data. A hit validates and decodes canonical bodies into typed batches, registers a fresh contribution and writes those batches through ordinary ingress. CompletedInputs::session also requires current Workspace/file ownership. These protections are valid for the current lifecycle, but they preserve a substantial replay boundary.

Evidence: [GR plan section 4](../../plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md#4-persisted-store-concurrency-and-fresh-admission); [workspace_products.rs](../../../crates/cpg-core/src/workspace_products.rs), lines 19-120; [workspace.rs](../../../crates/cpg-core/src/workspace.rs), lines 2750-2783.

**Consequence.** Equal completed content may exist as canonical storage, portable cached output and newly ingested canonical storage. Cache hits skip pure computation while retaining decoding, copying, membership construction and native write work. Identical concurrent attempts can still duplicate preparation before converging on a cache winner.

**Proposed correction.** Make attachment of compatible immutable completed content an owned operation. It consumes retained content, exact dependencies, model/operation compatibility and current attribution requirements; it produces a current checked binding or a defined refusal. It must not copy another attempt's runtime grants, admission capability or provenance.

Where attribution is already part of the canonical result, sharing requires exact equality. Where an owner explicitly separates reusable values from current support bindings, only those values may be shared. Changed source coordinates or observed provenance must still regenerate affected output. Computational shape caches remain useful when they have distinct consumers; a second serialized copy of unchanged canonical rows needs a demonstrated purpose.

**Closure.** Trace unchanged repeated compilation and compatible concurrent attachment without canonical row replay. Compare changed membership, deletion, empty domains, analyzer/model/settings changes and current attribution against clean recomputation. Cold admission remains independently exercised. Diagnosis is Implemented; the attachment operation remains Proposed.

### <a id="F05"></a>F05 - Attempt recovery needs durable effect fencing once database removal disappears

**Principles:** FP-05; DP-19, DP-20, DP-21; A2; G5.

**Implemented diagnosis.** Current compiler guards retain local terminality, remote certainty and committed identity. Uncertain cleanup preserves an orphan rather than claiming success. Those are strengths. However, attempt failure is presently bounded by abandoning a private database; sharing removes that cleanup unit.

The product cache already demonstrates a narrower pattern: persistent generation records fence delayed effects, while local leases and acknowledgments coordinate a managed local store.

Evidence: [compiler.rs](../../../crates/lctx-surrealdb/src/compiler.rs), lines 1447-1539; [product_cache.rs](../../../crates/lctx-surrealdb/src/product_cache.rs), lines 151-219 and 450-507; [completion/publication owner](../../design/sections/storage-and-publication.md).

**Consequence.** A dead client or released filesystem lock does not prove its server request stopped. An unknown write could complete after cleanup or attachment unless all competing operations consult a durable fence. Blind retries can duplicate effects or confuse incomplete and completed membership.

**Proposed correction.** Give each effectful attempt/operation an identity, generation and explicit recoverable outcome. Mutable staging stays unreachable from completed views. Short transactions perform guarded completion, attachment, publication and retirement; unknown acknowledgment is reconciled against exact committed identity. Retry the complete decision unit, not selected statements from it.

Version-matched SurrealDB source restricts FOR UPDATE to specific record targets; it does not supply predicate/range serializability. Shared invariants therefore need appropriate guard records touched by every competing path. Do not serialize all work through one global guard. Co-locate coupled atomic participants unless a qualified scope-switching transaction has a concrete benefit.

**Closure.** Delayed writes after cancellation, lost acknowledgment before/after commit, client death, concurrent equal insertion, conflict and cleanup races leave either one exact completed result or explicit unresolved state. Local locks may remain in one durable host coordination directory; moving all coordination into the database is not itself a requirement.

### <a id="F06"></a>F06 - Concurrent verification requires logical isolation and explicit disruptive-test semantics

**Principles:** FP-01, FP-06; DP-18, DP-19, DP-23; A1, A3; G4, G5.

**Implemented diagnosis.** Fixture attachments and sampled native controls create and remove namespaces/databases. Product-cache tests use scratch lease directories. These mechanisms provide isolated mutation but defeat stable shared-data reuse.

Evidence: [fixture baseline](../../../scripts/surrealdb_fixture.py), lines 869-925 and 1040-1053; [cache controls](../../../crates/lctx-surrealdb/tests/cache.rs), lines 35-86; [product-cache controls](../../../crates/lctx-surrealdb/src/product_cache.rs), lines 523-527. This is decisive sampled evidence, not an exhaustive teardown inventory.

**Proposed correction.** Ordinary tests use immutable named fixture inputs plus owned attempts and scoped effect identities. Tests of derivation or admission must execute the operation under test; a retained success cannot replace their independent expectation. Reuse fixture prerequisites and unaffected completed inputs, not the decision being asserted. Negative controls use fresh untrusted payloads and independently enforced rejection.

A stable validation responsibility may hold deliberately malformed synthetic state. It must not clone production datasets or become a database per test. Tests that mutate global definitions or deliberately stop the service need exclusive maintenance admission. A real crash/restart runs against the same durable service/storage after borrowers drain; it necessarily disrupts clients and sacrifices concurrency during that interval.

Physical corruption of the shared storage engine cannot be claimed safe merely because its initiating test has a unique attempt ID. Select supported fault mechanisms explicitly. Boundary fault injection, request loss and fail-stop restart cover different guarantees from destructive file corruption. Any required destructive recovery campaign needs a separately authorized maintenance/recovery contract; a scratch server is not an implicit escape route.

**Closure.** Inventory every setup/reset/teardown path, demonstrate ordinary concurrent tests over stable schema without cross-test visibility, and retain true cold/admission controls. Report disruptive controls separately from ordinary parallel verification.

### <a id="F07"></a>F07 - Shared persistence does not yet provide a composed working-set and execution route

**Principles:** FP-07; DP-10, DP-13, DP-20; CI-07, CI-08; A4.

**Implemented diagnosis.** Native-to-Arrow conversion, canonical product decoding, compact topology, kernel state, queued native values and server caches can overlap. Product candidates retain complete typed sections; native stream queues are row-bounded rather than byte-bounded. Database persistence alone does not release those representations.

Evidence: [workspace_products.rs](../../../crates/cpg-core/src/workspace_products.rs), lines 5-15 and 133 onward; [projected_arrow.rs](../../../crates/lctx-surrealdb/src/projected_arrow.rs), lines 20-145; [reader.rs](../../../crates/lctx-surrealdb/src/reader.rs), lines 515-549; [native-efficiency plan](../../plans/native-execution-efficiency-plan_2026-10-09.md).

**Consequence.** Concurrent warm clients can still repeatedly hydrate the same input or maintain overlapping large graphs. A bounded packet, transport batch or cache allowance does not establish feasibility for global analytics or skewed adjacency. Conversely, moving every kernel into SurrealQL could replace good finite algorithms with repeated expansion.

**Proposed correction.** Preserve operation-specific placement. Native indexed selection, joins, aggregation and coarse connected retrieval are candidates for execution near stored data. SCC scheduling, behavioral transfer/fixpoint operations, BDD semantics and qualified graph algorithms remain model-owned kernels unless a native substitute preserves their complete contracts.

Retain compact operation-shaped products when they amortize real work. Hydrate evidence separately. Compose server/cache memory, active client buffers, conversions, kernel scratch and queues; apply backpressure and timely release without forced compiler/test thread caps. Shared service cache residency is not shared client heap.

**Closure.** Inspect complete warm compile/serve routes and representative high-degree/global-analysis cases, including examined edges and simultaneous representations. Existing native-efficiency small-budget corrections and timeout investigations remain with their coordinator. No claim is made that startup, HTTP/2 or persistence explains all historical failures.

### <a id="F08"></a>F08 - Native retention must become part of the shared content contract

**Principles:** FP-05, FP-07; DP-09, DP-19, DP-20; A2, A4.

**Implemented diagnosis.** The storage-lifecycle plan correctly preserves warm caches and delegates native stores to their product owner. It does not authorize routine operator-store access or deletion. Shared canonical content introduces internal lifetime obligations that database-per-attempt retirement currently avoids.

Evidence: [storage-lifecycle plan](../../plans/storage-lifecycle-management-plan_2026-10-09.md), sections 3.3-3.4 and section 4 native-store row; [GR retention contract](../../plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md#4-persisted-store-concurrency-and-fresh-admission).

**Proposed correction.** The native owner determines reachability and retirement from published manifests, retained completed products, active pins, evidence/replay obligations and unresolved effects. A product referenced by published content is no longer merely disposable cache data. Reader release, attempt termination and content eligibility remain different facts.

The storage manager should observe declared allocation and call native lifecycle operations, never delete RocksDB internals. Service data and host coordination roots must survive worktree removal. Capacity pressure reports protected footprint and placement choices; it does not manufacture release or evict warm authoritative content.

**Closure.** Exercise shared content retained by two views, release of one consumer, pin-versus-retirement races, interrupted cleanup, worktree removal and unresolved effects. Coordinate native policy with the existing storage plan rather than creating a competing retention ledger.

## 4. Model-owned dependency reuse and execution

**Proposed composition.** One domain-owned interpretation of dependencies should serve product lookup, execution readiness and retained-content reachability. It need not be a universal incremental query engine or persistent executor. The existing cumulative driver can compose ordinary operations: bind requested inputs and policy; resolve exact dependencies; reuse or produce compatible immutable output; establish current completion/attribution; admit and publish.

The distinctions matter:

| Dependency | Governing meaning and reuse consequence |
|---|---|
| Source content | Canonical analyzer-visible bytes and scope, independent of checkout paths or acquisition labels |
| Model/operation contract | Vocabulary, relation/projection meaning, invariants and explicit policy revisions |
| Membership and absence | Complete selected domain, additions/deletions, empty/missing outcomes and coverage; positive returned keys are insufficient |
| Implementation | Relevant provider/kernel/lowering revision affecting the result; not every incidental build change |
| Parameters and settings | Requested profile, method, seeds, representation and binding values actually observed |
| Provenance | Current provider/acquisition/run/source association; separate from pure value equality where the owner permits that separation |
| Physical realization | Native schema, executable functions, indexes and engine compatibility; not interchangeable with semantic content identity |

The graph of these dependencies is not the semantic call graph. Native storage holds exact manifests and immutable identities; the domain owner defines substitutability. Recovery can discover compatible completed prerequisites and rerun missing work without treating arbitrary pending state as resumable. Retention can follow those same explicit references while retaining additional live-pin and unresolved-effect obligations. Derived reachability indexes may accelerate these operations but cannot become another independently editable validity authority.

Creating a new source view does not make an older immutable result false under its original inputs. It changes which result is applicable to the new binding. Recompute affected domains, then reuse or regenerate consumers under their contracts; preserve older published interpretations until released.

LIVE notifications and changefeeds may wake clients or invalidate derived lookup state. They cannot certify complete dependencies or completed content. Reconnect, cursor expiry and retention gaps require explicit detection and reconciliation of the affected authoritative scope before declaring a current result reusable. Immutable view handles remain valid without observing every later write. Do not solve notification gaps with unconditional whole-store rescans on ordinary attachment.

### Analysis records

| Operation/question | Universe and method | Fidelity, limits and evidence |
|---|---|---|
| Callable/catalog construction | Exact completed source/context domains; model-owned metadata and selection operations | Derived; empty/unknown coverage retained; canonical outputs cite current inputs |
| Structural topology and SCC schedule | Independent vertex inventory and typed directional multigraph; qualified graph kernels | Exact over declared projection; isolates/parallel arcs preserved; dense indices private |
| Behavioral summaries | Declared Local/Model/source-call inputs and finite transfer/fixpoint semantics | Conservative or exact under stated model; exhaustion/unknown cannot become complete absence |
| Discovery/search | Pinned eligible occurrences, encoder/projection and ranking policy | Heuristic navigation; view eligibility must precede channel quotas and final evidence hydration |
| Evidence retrieval | Exact published manifest, referenced facts and original ranges | Complete or explicitly bounded; every resource/continuation retains the same realization |

The current vector query combines encoder, policy, family, library input and eligible occurrence adjacency before the ANN tier. In a shared database, those predicates must additionally identify the exact published universe. Unrelated occurrences must not qualify a shared vector or consume candidate capacity merely because they exist globally. Filtering leaked candidates after a limit is not equivalent.

Evidence: [search.rs](../../../crates/lctx-serving/src/search.rs), lines 446-452.

## 5. Realistic change and failure scenarios

| Scenario | Required locality and assessment |
|---|---|
| Add a fact family - domain extension | One authoritative declaration plus new extraction/meaning where necessary; shared codecs, membership and admission follow. New semantics may change contracts; another database/schema lifecycle per family is unjustified. |
| Add an analytic - composition | Declare projection, settings, result class and consumer; reuse completed inputs/topology. Do not invent extraction or publication semantics inside the analytic. |
| Upgrade analyzer/model in one worktree - binding/contract change | Provider-private changes remain local; changed meanings receive explicit identities. Compatible views coexist; incompatible service/schema requirements are reported before effects. |
| Replace gRPC with progressive WebSocket - mechanism substitution | Preserve provisional rows, all terminals, cancellation, late errors, scoped session and backup contract. Similar item types do not establish equivalent execution. |
| Recompile unchanged pinned input - instance | Reuse unchanged admitted content and applicable pure products through fresh bindings; no unconditional canonical replay. |
| Add/delete a matching row - membership change | Complete selection/absence tokens change; affected results recompute. Existing positive keys alone cannot justify reuse. |
| Publish while serving - concurrent lifecycle | Publication freezes its own read set/effects; old readers retain content and definitions. |
| High-degree evidence request/global analytic - growth | Required universe remains complete; compact access, work bounds, spill or qualified kernels address expansion. Honest refusal alone does not establish fit. |
| Client death/service restart - failure | Durable effect fences and exact reconciliation govern recovery; service restart uses the same storage and explicit borrower disruption. |
| Worktree removal/retention - ownership change | Service/content/pins survive removal; current client/environment holders retain their existing guards; native retirement governs unreachable content. |

These scenarios support architectural revision before a benchmark: repeated schema installation and canonical replay are identifiable work. Their removal does not establish a quantitative latency or memory improvement.

## 6. Library fit and alternatives

**Interface-checked, 2026-10-09.** The locked SDK/engine API is 3.3.0 with the repository's narrow gRPC backport. Version-matched cached core/storage source informs capabilities but is not proof of the running server binary.

Context7 documentation discovery, official documentation and GitHub release/source inspection were combined with version-matched local source. The SDK upstream revision is `238bfeb11f5725bebed370167656748df8067595`; the local change is documented in [README.lctx.md](../../../third_party/surrealdb/README.lctx.md). The [3.3.0 release](https://github.com/surrealdb/surrealdb/releases/tag/v3.3.0) and committed Cargo lock identify client dependencies, not the separately installed service or its storage settings. Explicit installation/readiness must verify that selected server generation before attachment.

| Candidate | Fit and burden | Judgment |
|---|---|---|
| Current private databases plus disposable fixtures | Strong coarse isolation and familiar cleanup; repeats setup/content and retains lifecycle coupling | Does not meet requested target |
| One durable RocksDB service, stable databases, shared immutable content, existing patched gRPC | Uses current native schema/index/transaction and progressive SDK capabilities; requires scoped lifecycle/admission redesign | Preferred immediate direction, Proposed |
| Same service with progressive WebSocket adapter | Server protocol offers streaming/cancellation, but inspected SDK WebSocket path buffers query results; needs maintained request-terminal adapter | Credible alternative, not presently simpler |
| gRPC queries plus HTTP administration on same service | Preserves qualified query streaming while using explicit import/admin capability | Compatible combination where required |
| Push all analysis into SurrealDB | Removes some transfers, but traversal does not supply behavioral transfer, BDD or general qualified analytics semantics | Reject as universal placement rule |
| Add a general incremental engine alongside existing products | Could serve future fine-grained mutable workload; risks another dependency/retention owner now | Defer unless it replaces existing responsibility |
| Native managed daemon versus pinned container | Both can own one durable service and stable storage; differ in packaging, identity verification, host integration and maintenance | Keep current native owner unless container reproducibility has a concrete benefit; no presumed speed difference |
| RocksDB versus SurrealKV | RocksDB is the existing reviewed persistent route; alternative engine needs its own durability, recovery and workload qualification | Retain RocksDB; SurrealKV is not a shortcut around lifecycle redesign |

The engine API's default query streaming replays buffered results. The vendored gRPC implementation overrides it and now requires application End plus checked transport completion. NativeRows retains ordered statement terminals and late failure. These are concrete reasons to retain the current baseline, not evidence that WebSocket is inherently unsuitable.

Evidence: surrealdb-engine-api-3.3.0/src/lib.rs, lines 304-389; [vendored grpc.rs](../../../third_party/surrealdb/src/engine/remote/grpc.rs), lines 914-950 and 1102 onward; [reader.rs](../../../crates/lctx-surrealdb/src/reader.rs), lines 453-549.

A WebSocket adapter must finish each request from its own terminal/error/retraction protocol; closing a shared socket is not a substitute for gRPC EOF. Cancellation acknowledgment does not prove rollback. The inspected SDK default export/import operations are unsupported, while gRPC implements file transport; a WebSocket choice must preserve backup through a qualified same-service combination. Official current documentation identifies [server streaming RPC](https://surrealdb.com/docs/reference/rest-api/rpc-protocol#query_stream) separately from [SDK query behavior](https://surrealdb.com/docs/reference/rust/methods/query); those current pages are discovery/reference evidence, not a transfer of every later patch behavior to locked 3.3.0.

Session reuse also needs exact ownership: SDK 3.3.0 `Surreal::clone` creates a new session, while cloning an `Arc<Surreal<_>>` retains the existing client/session. Share authenticated preparation only across compatible principal and selected scope; changing a shared session's database is not an independent caller-local operation. A reconnect must not turn lost pending requests or cleared LIVE registrations into successful completion. Source: vendored `src/lib.rs`, line337, and `src/engine/remote/ws/mod.rs`, lines1028-1050.

The [changefeed documentation](https://surrealdb.com/docs/learn/querying/real-time/changefeeds) informs durable cursor/retention handling; LIVE remains a connection-lifetime notification mechanism. Neither replaces exact dependency manifests. The [backup/recovery guidance](https://surrealdb.com/docs/manage/self-hosted/backups-and-recovery) and [Rust export contract](https://surrealdb.com/docs/reference/rust/methods/export) must be qualified against the selected version: a logical export is not automatically a complete historical-version or credential backup, and copying live engine files is not a qualified snapshot procedure. Later patch documentation must not silently define 3.3.0 recovery behavior.

For deployment, the reproducible unit is the selected server build, configuration and storage contract. A pinned native binary and a digest-pinned container can both provide it. Container removal must not own canonical volume removal. An embedded engine per client would reintroduce conflicting process/storage lifetimes and is not the selected shared-service target. Preserve explicit synchronous durability; a backend's internal fsync grouping is not evidence of unsynchronized acknowledgment. Do not change backend merely from an isolated throughput or beta-feature impression. The [storage-engine documentation](https://surrealdb.com/docs/build/embedding/storage-engines) informs support maturity; final running-server claims require its exact binary/configuration.

The inspected core 3.3.0 transaction executor shares one transaction across runnable statements, including authorized USE changes. Its native edge enforcement resolves both endpoints in the relation document's namespace/database. Source references: surrealdb-core-3.3.0/src/dbs/top_level.rs, lines 47-72; src/dbs/executor.rs, lines 1143-1170, 1570-1577 and 2006-2015; src/doc/edges.rs, lines 43-81. No cross-database rollback probe was run. Use this as a qualified alternative capability, not a reason to split atomically coupled records.

PSE-arrow supplies a useful dirty-source reference for service/context lifetime separation and bounded WebSocket configuration. Its service supervisor fences context admission and retains immutable receiver descriptors. It also retains per-test database and disposable lifecycle practices, so it is not target acceptance or a package to copy wholesale. Inspected reference baseline: 46545b2ad3e2999b4335692ac49af6091438c3fc plus concurrent dirty changes; no runtime claims transfer. Decisive references are pse-arrow/scripts/surreal_server.py, lines 297-372 and 533-590; crates/pse-operations/src/canonical.rs, lines 1197-1238; vendor/surrealdb/src/engine/remote/ws/mod.rs, lines 835-849.

## 7. Independent judgments and gates

### Architectural judgments

| Judgment | Verdict | Basis |
|---|---|---|
| A1 - Localize change | Violated for unified target | Service attachment, attempt cleanup, test setup and database ownership remain coupled; F01/F06 |
| A2 - Encode domain meaning explicitly | Violated at shared-lifecycle boundary | Existing domain model is valuable, but shared content revision, view-relative publication/access and durable attempt retirement are not adequately realized; F02/F03/F05/F08 |
| A3 - Extend through composition | Violated for cross-attempt reuse | Reuse composes through fresh row replay; checked immutable attachment operation is missing; F04 |
| A4 - Fit execution to workload | Violated | Repeated setup/replay and overlapping representations remain structural amplification; proposed shared execution still needs scoped skew/resource design; F01/F04/F07 |

### Correctness and fidelity gates

These verdicts concern the reviewed target boundary, not every existing subsystem. An unresolved new target obligation is not evidence that the current isolated mechanism violates its accepted correctness contract.

| Gate | Verdict | Evidence and required action |
|---|---|---|
| G1 - Authority | Pass, examined scope | Model declarations remain authoritative; preserve one native content/reuse owner |
| G2 - Semantic fidelity | Unresolved for shared target | Shared model/revision keys and view-relative references require F02 decisions |
| G3 - Validity | Unresolved for shared target | Exact shared ingress, attachment and reference enforcement remain Proposed |
| G4 - Hidden behavior | Pass, examined boundary | Effects are explicit; target attachment/inspection must not install or select implicitly |
| G5 - Consistency/recovery | Unresolved for shared target | Shared publication, delayed effects and retirement need F03/F05/F06 closure |
| G6 - Transformation/reuse | Unresolved for replacement | Current replay retains checks; replacement attachment must establish substitutability |
| G7 - Truthful claims | Pass for this assessment | Replacement is Proposed; no new Tested/Measured or whole-plan claim |
| G8 - Library leverage | Pass, examined choices | Native persistence/query mechanisms and qualified kernels have reasons; no universal bespoke replacement recommended |
| CI-G1 - Fidelity | Pass for preserved contracts | Provider attribution, uncertainty, models and heuristic distinctions remain required |
| CI-G2 - Evidence closure | Unresolved for shared realization | View-conditioned role/original/search/resource closure needs F02/F03 |
| CI-G3 - Evaluation integrity | Not applicable to evaluation execution | Evaluation is not changed or qualified here; shared test infrastructure must continue excluding private truth from production inputs |

FP-01/03/06/07 are violated for the target scenarios; FP-04/05 are violated at the newly required lifecycle/authority boundary. FP-02 remains unresolved for shared realization and transport substitution. These results do not negate the inspected strengths of exact views, typed facts and independent admission.

## 8. Rule impacts and disposition

The following are recommended changes, not applied decisions.

| ID | Current rule | Proposed change and dependent findings | If retained |
|---|---|---|---|
| <a id="RC01"></a>RC01 | ADR-0138; B7; semantic section 15.11: private database per compilation, direct database sealing | Replace with shared immutable content, scoped attempts and manifest publication; F01-F05 | Target remains unmet |
| <a id="RC02"></a>RC02 | Storage sections 6.1-6.2/runbook: database-wide fingerprint, VIEWER and snapshot isolation | Pin exact content and executable definitions; enforce view-relative access; F02/F03 | Stable shared database cannot safely stand in for sealed snapshots |
| <a id="RC03"></a>RC03 | ADR-0140/GR section 4: products replay through fresh typed ingress and remain non-authoritative | Permit checked attachment of retained admitted content while preserving current provenance and independent cold admission; F04 | Pure computation can be skipped, but substantial replay/copy work remains |
| <a id="RC04"></a>RC04 | AGENTS/testing/runbook: disposable native fixtures and private test databases | Stable service/schema and logical test isolation; explicit disruptive maintenance controls; F01/F06 | Production/test/worktree unification is not achieved |
| <a id="RC05"></a>RC05 | Failed attempts restart from pinned captures; private databases abandoned | Preserve arbitrary-pending-resume exclusion, but retain compatible completed content and reconcile durable unknown effects; F05 | Failures discard more reusable work than target requires |
| <a id="RC06"></a>RC06 | Storage lifecycle: native stores external; existing native cache retirement unchanged | Native owner gains shared internal reachability/pin/obligation retirement; storage manager retains delegation; F08 | Shared authoritative content has incomplete lifecycle coverage |
| <a id="RC07"></a>RC07 | Current gRPC-specific client implementation | Retain initially; permit qualified WS/HTTP combination only against complete capability contract | No target conflict; transport replacement is optional |

Existing persisted/GK/GR/NE scheduled findings and receipts remain with the [persisted coordinator](../../plans/persisted-graph-execution-plan_2026-10-07.md). Storage work remains with its [plan disposition owner](../../plans/storage-lifecycle-management-plan_2026-10-09.md#9-finding-disposition-owner). This review creates no parallel implementation ledger.

**F01-F08 are deferred from implementation pending a unified-persistent target plan.** The trigger is authorization of that design/implementation scope. Plan creation should transfer these stable source IDs into the existing coordinator's disposition route and present RC01-RC07 explicitly. Deferral does not establish conformance.

## 9. Verification, uncertainty and final decision

**Passed, 2026-10-09:** read-only baseline identification with git rev-parse HEAD and initial git status --short; source/contract inspection described above.

**Not_run:** builds, tests, probes, benchmarks, service/database operations, cleanup, real-library qualification and activation, as required by this assignment.

**Publication checks by the coordinator, 2026-10-09:** `just docs` **passed**,371 canonical pages and zero link errors; scoped `git diff --check` **passed**. `just docs-check` **failed** before publication on the pre-existing stale concurrent ADR index (`docs/adr/README.md`); that unrelated generated output was left untouched. These checks establish documentation publication, not product qualification.

Historical integrated qualification remains incomplete. The coordinator records a pre-correction compiler composite with assertion failures and timeouts, followed by committed corrections requiring source-specific confirmation. Those failures are not all attributed to database startup or transport. This review neither reruns nor closes them.

The decisive remaining uncertainties concern the proposed remedy: physical representation of shared content/reference closure; executable-definition coexistence; exact attachment assurance; transaction guard scope; and supported disruptive/destructive testing. Suitable closure combines explicit contracts, source inspection and revealing focused cases. Representative end-to-end measurement is required only for subsequent quantitative speed/capacity claims.

**Bounded decision: Revise. Enclosing architecture: needs revision for the requested unified-persistent scenarios.** Preserve the explicit domain model, immutable completed views, current attribution, independent cold admission and qualified finite kernels. Replace database-per-attempt ownership and unconditional replay with scoped shared-content operations. Retain patched gRPC as the immediate lowest-burden transport unless a complete capability comparison justifies another route.

The next consequential decision belongs to the coordinator and operator: approve the shared content, publication and recovery boundaries before adapting service or test wiring. F02/F03/F05 are correctness prerequisites; F01/F04 offer direct removal of repeated work. Their remedies must be designed together so that reuse removes machinery without transferring semantic decisions into every caller.
