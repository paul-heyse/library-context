# Operation-shaped native lowering and final canonical preparation

**Accepted implementation target / execution in progress · 2026-10-09 (ADR-0138).** The operator authorized implementation after detailed execution planning. The operator accepted populated-review RC01/RC02 and retained RC03/RC04 during plan preparation. Implementation, independent controls and benefit measurement remain separate.

This supporting plan develops the [populated journey target review](../design_review/reviews/design_review_populated-journey-execution-amplification_2026-10-08.md). The [persisted-execution coordinator](persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) owns scheduled finding disposition, combined dependencies and actual receipts. Its §9.1 records the successful populated boundary and outstanding timeout/whole-plan limits. Existing PC/CU/BC packages retain their identities and remaining obligations.

**Unified persistence integration, Proposed, 2026-10-09:** the
[unified companion](unified-persistent-surrealdb-plan_2026-10-09.md) replaces private-database
publication with exact immutable manifests. PJ's typed lowering, mutation-only freeze, shared
pointer preparation and independent cold reconciliation remain. UP2/UP5 scope them to shared
payloads/views; UP7 migrates fixtures and fresh corruption controls; UP9 integrates PJ5 and
surviving acceptance. Existing populated passes retain their preceding physical-schema scope.

## 1. Outcome, baseline and workload

**Proposed replacement target, 2026-10-09:** the accepted graph/hash RC01/RC02 now schedule
[model-owned relation compilation](graph-compilation-kernels-and-hashing-plan_2026-10-09.md)
and [selective reuse](graph-compilation-reuse-and-invalidation-plan_2026-10-09.md). GK2/GK3/GK5 supersede this plan's normalization/validation/browse preparation deferrals where
reviewed amplification exists. The [native-efficiency companion](native-execution-efficiency-plan_2026-10-09.md)
now refines those implemented routes: NE1–NE3 share exact binding/access/admission preparation,
NE5/NE6 replace polling and unrelated-reader waits at local completion, and NE7 preserves one
live preparation across retention pressure. Final content freeze/global closure and independent
stored-state admission remain distinct. Native union-IN is not a qualified bulk alternative. PJ2/PJ3 remain implemented preservation constraints; coordinator §7.1/§8
own replacement mapping and finding status. Plan authoring still does not resume tests or production.

At source baseline `3c76f172`, two independently populated native journeys passed in 1883.02s of test execution; the complete `just verify --select serving:mcp` boundary passed in 2439.61s, including builds and 35 dependent Python controls. These are attributed 2026-10-08 local receipts, not new checks or internal phase measurements. They establish neither the dominant runtime phase nor any remedy's speed.

The intended workload is pinned-library compilation into complete attributed facts/catalogs, followed by repeated admission/publication reads and connected tools. Relevant growth axes are records, semantic families, completed contributors, aliases, scope skew and concurrent independent requests. Typed model authority, uncertainty, graph meaning and actual originals remain unchanged.

The target removes two source-established amplification mechanisms:

- **Review F01:** broad native body unions and table-wide scope VALUE programs make existing writes depend on unrelated declared kinds/fields.
- **Review F02:** canonical header, payload and sealing readers repeatedly reconstruct the same completed pointer union.

It retains native persistence at every compiler frontier, complete contribution/view/binding state, one canonical value owner, synchronous finite kernels, direct ordinary sealing and SQL staging-to-fresh restore. It does not require a storage pivot, a general cache/executor, test serialization or bigger deadlines.

### Foundations that already work

Source inspection on 2026-10-09 confirms exact `CheckedInputs`, reference-validation premise reuse, shared selected `PreparedGraphs`, explicit optional outcomes, retained read setup and drain ownership. They are foundations, not new defects to relabel. Registration is not execution; output selection cannot shrink a required intermediate/global universe. Behavioral transfer/fixpoint meaning cannot become ordinary reachability.

The authoring assessment follows core/template 3.3, efficient-architecture heuristics 1.0 and code-intelligence 1.5. Preparation lifetime, enforcement placement, coarse physical boundaries and total lifecycle burden inform the selected route; no additional cost model or proof system is introduced.

## 2. Confirmed rule decisions and architectural route

These source-qualified IDs differ from earlier reviews' RC01/RC02.

| Source review item | Operator decision, 2026-10-09 | Consequence and route |
|---|---|---|
| [RC01](../design_review/reviews/design_review_populated-journey-execution-amplification_2026-10-08.md#rc01), native body/layout enforcement | **Accepted** | Replace vocabulary-wide native closed-union enforcement with complete typed ingress and independently reconstructed physical agreement; retain the three backing families and complete native bodies. PJ0 records the physical-enforcement decision before PJ2. |
| RC02, stored scope VALUE | **Accepted** | Supply mechanically derived typed scope fields without VALUE replay. Independent imported/stored scope equality remains mandatory. PJ0 records this before PJ2. |
| RC03, native compiler placement | **Retained; change rejected for this target** | Every frontier/profile, including artifact-only, continues to use the native compiler. Different intermediate placement remains a triggered investigation, requiring a new operator decision if selected later. |
| RC04, native SQL transport | **Retained; change rejected for this target** | Preserve staging isolation and fresh reconstruction. Data-only transport remains an investigation; this plan does not implement it or serve imported staging. |

PJ0 installed ADR-0138, consolidating the affected physical-enforcement decisions and every surviving native-placement, supplier, completion, transport and publication obligation. Semantic model §15.11 and storage/publication §6.1–§6.3 distinguish typed semantic admission, fixed envelope enforcement and complete independent physical validation. Superseded records retire after all governing references transfer; Git retains their provenance.

The new physical schema is a migration. Rebuild owned disposable/current project realizations from pinned inputs; change realization identity and reviewed schema snapshots coherently. Keep completed-state format2 and artifact format3: the selected correction changes enforcement and private preparation, not their transported record meaning. No old-schema reader, dual writes or compatibility generation is added. Operator database adoption/quiescence remains separately authorized; implementation controls use disposable owned databases.

## 3. Target design and contracts

### 3.1 Validate the actual typed record, lower through a fixed native envelope — F01

Keep `entity`, `assertion` and `compiler_record` with their current nominal keys, canonical bytes, complete native bodies and membership ownership. Keep fixed envelope types, uniqueness indexes, relation endpoint enforcement, existing scope index and separately owned search projections.

Define body as `TYPE object FLEXIBLE` under SCHEMAFULL and define scope keys as supplied `TYPE array<string>`. Graph families retain supplied `scope_context TYPE option<string>`; compiler backing retains its absence of that scalar. Remove the broad body unions, generated nested body definitions and scope VALUE programs. Do not replace them with another vocabulary-wide IF/CASE/ASSERT program or a map built for each row.

The enforcement boundary is explicit:

1. Existing model decode/canonical callbacks validate the selected relation's exact schema, records, nominal identity, finite/code values and sum-arm requirements before physical writes. Graph loaders validate typed entities/assertions. Semantic/reference/global admission remains at its existing completed-input boundary.
2. Mechanical physical lowering supplies exact body, canonical bytes, content/key fields and scope values from that validated typed input. Original-chunk metadata retains its separate byte owner. Selected scoped fields and exact SDK string conversion govern scope keys; fields are ordered/deduplicated as required by the existing mapping.
3. Cold compiler reconstruction and graph reconciliation regenerate complete expected rows from canonical/model values and compare full actual body/envelope/scope data. They reject extra/missing/unknown/malformed fields, wrong keys/content and imported derived-value disagreement. Header/digest agreement is not sufficient.
4. Publication still examines actual stored rows and edges after writers drain. A reused pointer inventory is not proof that bodies or relations are correct.

**Intentional enforcement change:** arbitrary private raw SQL can store a malformed flexible body. Standalone statement-time rejection of every semantic body shape is no longer guaranteed. Production ingress cannot accept that shape, and independent cold/publication admission cannot expose it as valid. Raw SQL is not an alternative supported semantic writer. Published viewer credentials and pinned executable realization retain their existing authority boundary.

SurrealDB 3.3 removes object-valued NONE during field cleanup, including under FLEXIBLE; NULL is retained. Preserve the codec's explicit NULL values for nullable/inactive body fields, exact required-field presence and the distinction between missing/NONE/NULL. Do not claim object-property NONE survives. Scope omission follows the actual existing field semantics, including graph context conversion.

### 3.2 Select the physical adapter once, rather than moving the union into Rust

The current grouped body codecs still inspect every generated kind, and some relation lookups linearly search complete inventories. PJ2 replaces those hot selections with one declaration-derived prepared relation-to-adapter lookup. It selects a synchronous body/graph/scope adapter once per actual relation/group; construction occurs once at an appropriate static owner, not per row or scan.

Use existing typed enum/record callbacks and generated declaration metadata. Do not create a manually maintained whitelist, competing model registry or native-library dependency in `lctx-model`. The physical adapter belongs in `lctx-surrealdb`. Unknown relations fail closed; inner sum dispatch remains local to the selected record. Existing ordering, heterogeneous grouped output positions, text/bytes fidelity, graph aliases and canonical identity remain exact.

The shared coarse future/stream boundaries corrected under CU/BC remain. Generated dispatch invokes synchronous typed leaves; it must not recreate a large generic async inventory loop or per-row task/trait-object machinery. Adding an unrelated record can extend the one prepared lookup and model declarations, but not an existing row's body/scope validation program or native DDL union.

### 3.3 Freeze compiler content at final admission — F02

Under the unified target, this freeze closes the current attempt's content/effects and exact
read set, not all shared-store writers. UP3 preserves the native visibility handoff and
acknowledgment certainty; UP5 publishes its manifest while unrelated attempts/readers proceed.
Do not interpret the implementation-era global seal/removal wording below as database-wide
shutdown or deletion. Its independent frozen-state admission obligations remain.

Final artifact admission follows completed compilation; no production caller completes another contribution after successful admission. Select an **irreversible final compiler-content freeze**, rather than caching every mutable attempt generation.

Check immutable request/profile/frontier/capture compatibility first, so a wrong request does not finalize the attempt. Then atomically close admission for compiler-content mutation and drain already admitted content mutations. Refuse pending contributions or failed/uncertain state. The freeze covers contribution registration/writes/completion, membership/alias/backing changes, bindings, state import and frontier changes. A writer cannot enter between closing admission and establishing the frozen state.

The finalization owner is Workspace together with its native authority. Reuse the existing async `completion_gate` to fence the entire output-publication operation: native completion, each current/frozen binding update and publication of Workspace's completed descriptors. Ordinary `finish_compilation` already uses that gate and makes the workspace immutable; preserve that invariant. Acquire the gate before closing native content admission, so an already admitted publication can finish its binding/visibility handoff rather than being cut off between native calls. Native operation counters alone do not establish this handoff. Restored/from-existing paths finish `runtime.restore` before finalization and establish an equally immutable read context; do not invent a normal compilation attestation for them.

Check actual contribution completeness once at this boundary, reusing the owner-enumeration pass or a bounded pending-existence query. An empty in-memory specification cache is not proof that an imported/existing database has no pending contribution. Imported state has already passed independent full cold admission; a fresh attempt carries its acknowledged mutation history. Freeze must distinguish those premises without trusting a serialized preparation marker.

Only after mutations reach terminality and the exact state is frozen perform final facts availability, semantic and frontier admission. Earlier state-dependent checks do not establish validity for a state changed during drainage. Reuse checked premises only for their exact frozen descriptors. An already admitted completion or binding may finish during the transition; either its resulting state is admitted after drain or the attempt fails, rather than validating one state and exposing another. Build/use canonical spools only after the frozen-state admission succeeds.

The existing `require_compilation` includes current Workspace content identity, not only immutable request fields. Its full attestation comparison therefore runs after the workspace/native handoff is frozen. Compare actual frozen current/boundary bindings and completed-owner metadata with the Workspace's exact admitted descriptors; reject mismatches, unrepresented direct mutations or a changed requested compilation. Cold restored admission instead uses the manifest's independent captured producer/outcome contract. This is compact descriptor agreement, not another rich payload replay.

Keep read admission open. Existing `end_writes` closes all operation admission and therefore cannot be reused at this point. Final global drain/seal/removal remains later. Do not hold synchronous mutexes or database transactions across I/O or finite computation; the existing scoped async completion gate protects the required visibility transition and is released after that handoff, before semantic scans and preparation.

The preparation identity is the exact frozen attempt, completed owners/descriptors, alias closure and physical-lowering policy. Do not compute `completed_state()` or enumerate memberships to decide whether an already prepared inventory is reusable. Keep the independent completed-state identity in the manifest at its existing owner.

Pre-final completed-view reads retain current semantics. No mutable prepared cache is installed for them. New completion after freeze is an explicit refusal; a different final state requires a new attempt. This replaces invalidation with a narrower finalization contract and must be tested as such.

### 3.4 Prepare complete pointer runs once and give each consumer its own cursor

For each canonical family, enumerate exact completed owners/memberships, expand required entity aliases, and sort/deduplicate physical pointers once after freeze. Include companions absent from direct family membership, isolates and complete-empty state. All source streams and sorting workers must reach successful terminality before the inventory becomes available.

Reuse the existing external-run kernel. Separate its immutable final scratch owner from mutable reading state; independently opened cursors have separate offsets, bounded decode/window reservations and normal terminal handling. Retain scratch until its last handle/cursor and blocking operation finish. A scratch handle is not an active native transport/scan lease, so an idle prepared inventory must not block completion/final drain.

This is a compact pointer spool, not a resident rich-payload/header cache or another canonical authority. Header and payload consumers still read the actual selected native rows and enforce their own semantics. Distinct concurrent cursors may use available runtime capacity; never coordinate them by rewinding a shared reader.

Migrate the complete affected route:

- ordinary `artifact::admit`: prepare before header lookup; header and entity/assertion payload readers use the prepared runs;
- `AdmittedArtifact` export and `seal_completed`: retain/use the same frozen native preparation;
- detached readmission and SQL restore: finish canonical/original/state loading and independent cold checks before freezing; no imported preparation/validity marker is trusted;
- `VerifiedExport` publication: consume its already readmitted native authority. Retire the post-admission portable canonical/original reload in `publisher::load`; derive edges/search and seal through the shared native publication owner after independent external admission;
- restored finalization: use fresh destination preparation where canonical reads are needed, retain actual graph/state/search reconciliation and physical-name independence.

Detached publication is not ordinary self-import: initial external transport and semantic admission remain independent. The common publication routine receives the admitted native authority, its exact manifest and preparation; wrappers retain their actual trust provenance. It does not admit arbitrary native state. Publication is private to publisher and accepts only core's nominal admitted artifact/export/restored owners, bound to the exact store and manifest. A separate private authenticated effect session runs under native lifecycle guards; no native public seal, marker writer or compiler administrator client escapes.

Publisher edge/search/definition/publication writes remain allowed after compiler-content freeze because they do not mutate backing, memberships or aliases. Restrict all identified production content-mutating escape paths, including loader/shared-client callers. Already derived reference edges copied during restore may be reconciled; they cannot widen canonical membership.

### 3.5 Runtime representativeness and proportionate phase evidence

Retain borrowed synchronous finite kernels and their owned inputs. Populated compiler journeys should use the multithread runtime topology of the CLI, with available workers and test parallelism; small current-thread controls remain useful for their own contracts. This is control representativeness, not a proven remedy for the observed duration. Do not replace borrowed leaves with detached `spawn_blocking` calls.

PJ1 establishes coarse phase visibility for the operational question that prompted this plan: setup, providers, normalization, upper operations, final preparation/admission, sealing, tool groups and restore. Use the existing tracing/logging and run-output routes, with explicit begin/terminal status and elapsed phase observations retained if the command is interrupted. Library owners emit their own phase boundaries; the journey/CLI exposes those through an appropriately scoped subscriber. Preserve concurrent attribution and avoid global subscriber collisions, secrets or evidence payload logging. No event ledger, generic telemetry framework, per-row tracing or new profiler is required.

Implementation observation, 2026-10-09: a composed core binary captured ordinary scoped INFO but suppressed native macro phase callsites. Fixed-metadata phase events now use the captured Dispatch directly, honoring metadata/event filters and the retained parent; the underlying library/compiler cause is unproven. Facts, invariant and frontier admission have distinct coarse observations. Fixture thread handoffs retain the caller dispatch/span. This changes diagnostics, not operation or admission semantics.

Already emitted observations survive cancellation/process death through the owned run log. A missing terminal remains incomplete; forced termination cannot fabricate a successful/cancelled phase terminal or completed elapsed duration. Graceful cancellation records terminal evidence only after actual work drainage. Keep build/preparation time separate from executed compiler/journey phases.

Phase timings guide later runtime/serving work and benefit claims. They are not prerequisites for recognizing F01/F02, nor reasons to restart the deferred timeout campaign while authoring. No speed threshold is invented from the single receipt.

## 4. Alternatives and extension/failure scenarios

| Choice | Benefit mechanism and reason selected | Burden / reopening evidence |
|---|---|---|
| Fixed native envelope plus typed ingress | Removes vocabulary-wide DB body/scope programs; existing model and codecs govern complete meaning | Deliberate loss of raw-SQL semantic-shape rejection; reopening requires a legitimate writer needing native closed enforcement |
| Discriminator-directed native ASSERT | Could retain immediate DB shape rejection | Not selected: another generated validation program and sequential dispatch unless an actually smaller qualified route is established |
| Per-kind tables/fields | Direct native closed validation and grouping | Not selected without an access/locality consumer; adds definitions, indexes and routing proportional to vocabulary |
| Frozen compact pointer runs | Reuses actual final membership/alias closure across headers/payloads/publication | Scratch/cursor lifetime and content-freeze contract; cannot support same-attempt mutation after final admission |
| Mutable generation cache | Could reuse canonical preparation before final admission | Not selected: adds alias/version/invalidation ownership without a current consumer requiring it |
| Native membership set processing | Could reduce candidate × owner probes | Remains bounded investigation until actual plans and exact deduplication/completed membership establish fit |
| Eager DataFusion cache | Can reuse projected relational results | Not selected for canonical pointer reuse; adds resident MemTable state and does not replace current shared RuntimeEnv |
| Different intermediate placement / data-only restore | Could reduce effect crossings or staging/copy work | Retained as investigations under RC03/RC04; rule changes require a later explicit decision |

Adding an unrelated record extends declaration-derived lookup construction, not ordinary native validation. A new analytic reuses exact input declarations and compatible prepared topology; a required graph algorithm trait adapter is added only for a named consumer. A new overlapping pre-final consumer reopens F03, not the final freeze contract.

A changed record/sum policy changes the model/codec compatibility at its semantic owner; the fixed DDL does not hide that change. A changed native mapping updates realization identity, reconstruction and writers together. High-degree/cyclic scope preserves complete intermediates, directional parallel edges and finite transfer semantics. Cursor limits do not certify complete absence.

Cancelling preparation must drain submitted reads and acknowledged sorting work and expose no ready inventory. A cancelled cursor cannot delete scratch retained by another cursor. Missing/extra alias or membership, conflicting pointers, corrupt scratch framing, changed physical body or unknown acknowledgement must fail through the existing structured completion owner. Failure after committed publication retains the committed handle; cleanup cannot imply rollback.

## 5. Packages, prerequisites and integration

PJ packages extend the existing undertaking; they do not renumber or close PC/CU/BC obligations. The root owns shared model/physical declarations, lifecycle contracts, decisions, plan disposition and integration.

| Package | Required input | Delivered behavior and completion boundary |
|---|---|---|
| **PJ0 — decision and consumer contract** | Operator RC01/RC02 decisions and reviewed target | Superseding physical-enforcement decision and owning architecture updates; complete writer/readmission/publication map; explicit final freeze versus final drain. Must precede dependent production changes. |
| **PJ1 — attributable runtime controls** | Existing phase owners/run logging and current runtime map | Coarse attributable phase/terminal evidence, cancellation retention and representative multithread populated journeys. Supplies runtime diagnosis without imposing caps or assuming cause. |
| **PJ2 — operation-shaped physical lowering** | PJ0; full selected adapter and cold-reconstruction contracts | Fixed native envelopes, supplied scopes, prepared declaration-derived adapters, all compiler/loader/alias/restore consumers migrated; new schema identity/snapshots and independent malformed-body rejection at declared boundaries. |
| **PJ3 — frozen reusable canonical preparation** | PJ0; existing read/worker terminal ownership | Mutation-only freeze, two complete immutable pointer runs, independent charged cursors, all admission/export/sealing/detached/restored consumers migrated. No post-freeze canonical reload or stale inventory path. |
| **PJ4 — consequential residual investigations** | Working PJ1 observations or a specific source-supported trigger | Resolve candidate-owner query plans, runtime/permit contention, browse preparation, batch live state or needed graph adapter only where material. Retained larger alternatives and F03 keep explicit triggers. Each selected change gets concrete design/consumer/acceptance content before execution. |
| **PJ5 — integrated acceptance and handoff** | PJ2/PJ3 integrated; PJ1 available; required PJ4 decisions settled | Affected current-schema controls, both profiles/frontiers, independent cold transport and populated native/MCP/evaluator acceptance; applicable leaves, plan reviews, qualified labels and obsolete-route retirement. |

PJ1 can be developed alongside PJ0. PJ2 and PJ3 are logically independent after their shared contracts are settled, but share native/loader/cold paths: coordinate a single writer or isolated revisions. Neither waits for unrelated BC3 profile installation. Integrate both before long populated acceptance; do not repeat that journey after every slice.

GK/GR now provide the specified redesign for model-owned scopes, shared preparation and qualified
reuse; it does not wait for PJ4 or older acceptance to finish. Other PJ4 avenues still need their
named semantic/consumer decision. PJ5 consumes matching-source/profile receipts at the coordinator
and joins GK7/GR6 affected final acceptance. Runtime timeout causation is unresolved; the reviewed
normalization/validation structural corrections are scheduled, not deferred by overlap.

## 6. Investigation routes and support limits

| Source avenue | Decision owner / settling evidence | Route and trigger |
|---|---|---|
| Review §10 phase attribution | Compiler, publisher and journey owners; actual attributable phase terminals from comparable current-source runs | PJ1; first authorized execution of the corrected journey. Required before claiming a measured hotspot or benefit. |
| F01 closed-shape lowering | Native codec/schema and independent reconstruction; exact unknown/extra/missing tags/fields, required/inactive arms, NONE/NULL and supplied-scope disagreement | Target selected in §3.1; PJ2 functional controls establish equivalence at supported ingress/cold/publication boundaries, not raw SQL. |
| F02 candidate × owner access | Completed-view selection owner; exact pinned-engine plans, duplicates, candidate/contributor skew and terminal behavior | PJ4 if contributor growth materially affects selection; existing bounded point lookup remains until a better complete route is qualified. |
| Runtime topology and aggregate permits | Compiler/runtime/native-call owners; actual caller/bridge/server/worker topology, overlap, admission/charge duration and drain | PJ1 representativeness; PJ4 placement/permit change only for demonstrated contention or a new overlap requirement. Started blocking tasks must be retained/drained. |
| Browse static preparation | Serving classification/browse owner; immutable member/context/capture closure versus request policy, complete counts/alternatives/unknowns and retained-state burden | GK5/GR5 schedule model-owned same-pin preparation now; complete evidence/policy dependencies and retained-state controls, without narrowing the semantic universe. |
| Data-only restore | Publisher transport/admission owner; complete graph/original/state codecs, executable isolation and cold corruption rejection | Deferred investigation; revisit RC04 only if staging/copy burden or a new transport consumer warrants replacement. No replacement implemented here. |
| Borrowed graph algorithm adapter | Projection and named algorithm owner; exact trait bounds, nominal/local mapping, isolates/direction/multiplicity | GK2/GK4 qualify contrasting compact layouts against the actual owned operation. Existing branded views remain where fit; output filtering cannot shrink required intermediates. |
| Batch lowering live intermediates | Native ingestion/resource owner; simultaneous Arrow/bodies/decoded rows/maps/aliases before loader windows, actual accounting/lifetimes | GK2/GK3 shared hydration must account for union/owner views and simultaneous state. Incoming bounds apply; downstream windows alone do not establish aggregate live state. |
| Alternative intermediate persistence | Compiler/native dependency owner; durable inspection/recovery value versus pure transformation/effect crossings | Deferred under retained RC03; new operator rule decision before selecting another placement. One canonical semantic authority remains required. |
| Review F03 independent read/completion | Native lifecycle and overlapping consumer owner; retained old-view stream, distinct completion, late failure/cancellation and global refusal | Source-owned Deferred until supported independent overlap or demonstrated unrelated blocking. Sequential pre-final composition remains the scoped supported premise. |
| Residual optional execution | Upper-operation owner; actual demanded predecessors, not registration inventory | GK4/GR4 cover actual method demand and eligible pure products; keep explicit NotRequested, required full universes and existing embedding cache. |

These are local decision routes, not a second finding register. Quantitative runtime/peak-memory benefits remain unmeasured. Static evidence may establish removal of a repeated program or inventory; representative measurement is needed to quantify its consequence.

**Bounded residual assessment, 2026-10-09:** current frontier phase logs expose prolonged
normalization before final admission. Static inspection found unchanged per-root selected work:
callable-aspect normalization invokes `PreparedEdges::grain` and `AspectScopes::load_data` for
each root across three namespaces, loading every declared input; entity normalization similarly
builds closures for selected observation roots. The owners are
[`normalize/mod.rs`](../../crates/cpg-core/src/normalize/mod.rs),
[`scoped_aspects.rs`](../../crates/cpg-core/src/scoped_aspects.rs) and
[`consumed_rows.rs`](../../crates/cpg-core/src/consumed_rows.rs).
Those files and the native scan path are unchanged by PJ2/PJ3; scope indexes/predicates remain.
No bounded correctness regression or supported freeze/read cycle was identified. Preserve these
specific repeated-effect paths as GK2/GK3/GR4 migration cases, now explicitly scheduled by the
operator's graph/hash decision. Receiver CPU observations do not bound callable-aspect CPU
work, whose synchronous kernel is not separately instrumented. The coordinator retains actual
durations, outcomes and the incomplete Behavioral entity-normalization observations.

The populated run also exposes normalized admission as a distinct interval. Its unchanged
semantic path registers the completed relation set, checks uncached nominal-reference fields
with separate queries, then runs applicable invariants and scoped closure validators. It does
not freeze content, prepare canonical rows or call `wait_scans`; the new completion-owner handoff
has already returned. This supports a further concrete validation/access-amplification avenue,
not a PJ2/PJ3 wait-cycle diagnosis or a measured attribution. GK3 covers this distinct
reference/invariant demand under coordinator disposition; causal runtime investigation remains separate.

## 7. Verification, migration and completion

**During implementation:** compile checks and minimal revealing controls for the changed boundary, owned disposable fixtures, normal available parallelism and pinned release artifacts. Resolve exact selectors with `just verify --print`; no long family/journey after a slice. The authoring session lacked its user systemd bus. Execution repaired this with the existing /run/user/1000 manager socket and explicit XDG_RUNTIME_DIR/DBUS_SESSION_BUS_ADDRESS; readiness passed on 2026-10-09, without touching operator databases.

| Boundary | Revealing independent controls |
|---|---|
| PJ2 typed ingress | Unknown/missing/extra fields; unsupported relation/tag; required/inactive sum values; wrong code/nominal key; exact UTF-8/native bytes/finite values; NULL versus missing; original metadata. Invalid ingress must fail before its write, while unchanged valid values remain exact. |
| PJ2 native and cold behavior | Actual generic-body storage under fixed envelope; intentionally injected private malformed native bodies, supplied-scope errors and altered canonical bytes fail cold/publication reconstruction. Keep top-level typing, uniqueness and ENFORCED endpoint behavior. Read schema .snap.new before accepting; state that the snapshot is a schema migration. |
| PJ2 extension locality | Add a representative unrelated family/field through declarations and show existing-family native DDL/body/scope programs remain unchanged. Exercise selected adapter coverage and heterogeneous order without a manual type whitelist. |
| PJ3 content freeze | Pending contribution, failed attempt and in-flight admitted completion/binding/Workspace visibility handoff; atomic refusal of late registration/write/completion/bind/import; full requested-compilation attestation and final facts/semantic/frontier checks observe exactly the drained frozen state, never an earlier one; imported contexts do not acquire invented compilation attestations; reads continue and derived publication writes remain permitted. Wrong immutable request/profile/frontier checks do not prematurely freeze. |
| PJ3 preparation and cursors | One discovery/sort per family across header/payload/sealing; complete empty universe, overlapping contributors, alias-only companions, isolates, conflicts and multiple spill runs. Interleave two independent cursors and cancel one while the other completes; scratch and reservations survive until actual terminality. |
| PJ3 independent boundaries | Fresh detached import and SQL restore across physical names, exact captured suppliers/outcomes, full/projected vectors and original bytes; no trusted serialized inventory; cold corruption still rejects. Post-freeze publication does not insert/reload canonical content. |
| PJ1 diagnostics/runtime | Concurrent attribution, success/failure/cancelled phase terminals, interrupted retained output and no global subscriber collision. Representative multithread topology uses available workers; no test/thread caps or invented speed assertions. |
| PJ5 assembled scope | Both Catalog/Behavioral profiles and affected frontier boundaries, fresh compile/admit/direct seal, complete backup/restore/inspection and ten tools with expected originals/cursors; dependent Python wire/native evaluator controls and correct source extension. |

Current command routes, statically resolved on 2026-10-09:

- `store:rust` runs lctx-surrealdb/lctx-publisher on a disposable fixture; `--cli` enables its CLI preparation.
- `compiler:producer` selects cpg-core lib/tests; `compiler:cli` selects lctx CLI/artifact controls.
- `serving:rust` selects native serving. Narrow these with actual affected target/filter arguments.
- `serving:mcp` builds CLI/evaluator, runs both native journeys, then Python controls. A pytest filter does not skip native preparation.

At functional completion PJ5 schedules affected leaves once. The preceding operator selection
completed a targeted populated boundary; its historical passes remain scoped. NE9/GK7/GR6 now schedule
affected final-source assembled qualification and surviving timeout/PC/CU/BC controls after the
replacement works. Authoring runs no product tests. Record each failed/blocked/not_run obligation;
BC3 retains its qualified-default gate and release stays current until it passes. No floating
toolchain, wholesale dependency re-resolution, cargo clean or job/worker caps are introduced.

A new schema uses fresh fixtures. No operator rebuild/selection, real FastMCP/Qwen pilot, protected evaluation, wheel, benchmark campaign or push is implied. Preserve profiling captures, shared caches and the dirty PC3 worktree. No performance threshold or real-library claim is an acceptance substitute.

Before implementation, a fresh independent design/target reviewer assesses this plan and revised coordinator, including alternatives and adjacent trust/lifecycle consumers. At integration, the binding's assembled review assesses actual implementation. Authoring runs `just docs-check` and scoped turn-end only; it does not run product qualification or apply PJ packages.

## 8. Source coverage and handoff

| Source obligation | Plan route / current disposition owner |
|---|---|
| Review F01, local sum countercase and extension/cold closure | §3.1–§3.2, PJ0/PJ2/PJ5; coordinator §8 populated-review F01 |
| Review F02, alias countercase, separate assurance and contributor-growth concern | §3.3–§3.4, PJ3/PJ5; coordinator §8 populated-review F02; candidate-access investigation §6 |
| Review F03 and sequential acceptance exclusion | §4/§6; source review retains Deferred ownership and exact overlap/failure trigger |
| Review §3 preserved semantic/lifecycle contracts | §1/§3/§7, every affected consumer and PJ5; existing PC/CU/BC owners remain |
| Review §6 runtime, demand, restore and serving observations | §3.5/§4/§6, PJ1 and conditional PJ4; retained mechanisms are explicit |
| Review §7 extension/failure scenarios and §8 library composition | §3–§4/§7; pinned contracts and independent controls, no automatic capability adoption |
| Review §9 larger alternatives and §10 eight investigations | §4/§6; selected, conditional and deferred routes distinguished |
| Review RC01–RC04 | §2, operator outcomes; PJ0 precedes dependent implementation |
| Existing timeout and PC6/CU6/BC3/BC5 obligations | Coordinator §9.1 and their original finding owners; neither renamed nor closed here |

F01/F02 scheduling transfers their mutable disposition to the coordinator, with source links retained. This plan owns design and package prerequisites, not another progress ledger. F03 and unscheduled alternatives retain source-owned deferrals until their triggers warrant a scoped decision.

**Execution checkpoint, 2026-10-09:** PJ0–PJ3 and the bounded assembled-review corrections are **Implemented**; targeted PJ5 acceptance is recorded with remaining timeout limits. The [independent assembled review](../design_review/reviews/design_review_assembled-populated-execution-correction_2026-10-09.md) accepts scoped at static / Implemented strength. Current passed controls, normalization timeouts and the passed populated native/MCP/evaluator boundary are recorded only in [coordinator §9.1](persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07). The earlier target-plan review accepted Proposed strength and corrected finalization order and the complete workspace/native handoff before implementation. Neither review acceptance nor focused controls establish enclosing finding closure or a measured speed improvement. Enduring physical contracts are recorded in their semantic/storage owners through ADR-0138; failed timeout and PC6/CU6/BC3/BC5 acceptance remain open under the GK7/GR6 final-source route.

### Pinned capability evidence

The source review's 2026-10-08 library evidence was reused and the selected native field route was checked against SurrealDB 3.3.0 source on 2026-10-09. Context7 resolve/query supplied current field/schema documentation; exact pinned processing governs FLEXIBLE, NULL/NONE and VALUE claims. [Official field reference](https://surrealdb.com/docs/surrealql/statements/define/field), [pinned field processing](https://docs.rs/crate/surrealdb-core/3.3.0/source/src/doc/field.rs).

The focused foundation map also inspected [typed physical bodies](../../crates/lctx-surrealdb/src/codec.rs), [actual-row reconciliation](../../crates/lctx-surrealdb/src/reconciliation.rs), [native finality/cold backing](../../crates/lctx-surrealdb/src/compiler.rs), [scratch ordering ownership](../../crates/lctx-surrealdb/src/ordered_rows.rs), [ordinary/detached artifact admission](../../crates/cpg-core/src/artifact.rs) and [native publication](../../crates/lctx-publisher/src/lib.rs). Static evidence supports the selected composition; no new native probe, build or runtime measurement was performed for authoring.
