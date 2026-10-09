# Native execution efficiency: shared preparation and scoped ownership

**Accepted target; implementation in progress · 2026-10-09.** Native-efficiency review RC01–RC05
were explicitly accepted by the operator during plan preparation. Execution is authorized;
ADR-0141 records the complementary access/preparation/completion decision. The coordinator
§8/§9.1 owns finding disposition and current implementation/verification receipts.

This companion develops the [native-execution efficiency review](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md).
The [persisted coordinator](persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns scheduled finding disposition, combined sequencing and actual acceptance receipts.
The [kernel plan](graph-compilation-kernels-and-hashing-plan_2026-10-09.md) and
[reuse plan](graph-compilation-reuse-and-invalidation-plan_2026-10-09.md) retain semantic scope
and product contracts. This companion refines their physical execution and lifecycle composition;
it is neither a second scheduler nor a second progress ledger.

**Unified persistence integration, Proposed, 2026-10-09:** the
[unified companion](unified-persistent-surrealdb-plan_2026-10-09.md) replaces private database
ownership, replay-only canonical reuse and database-wide snapshot access. Preserve NE's exact
bindings, independent admission, wake-driven handoffs, relevant completion, stable flights and
deliberate observation. UP4/UP5 extend those actual consumers; UP7 rebases fixture/diagnostic
interfaces against current AF work; UP8 composes client/server retention. UP9 integrates the
remaining NE9/GK7/GR6 acceptance; existing timeouts and assertion receipts remain unchanged.

## 1. Outcome, baseline and assessed foundations

Make necessary native work follow actual immutable membership and compatible physical inputs,
while separate logical roots, independent checks and fresh native effects retain their owners.
The resulting compiler resolves declared inputs once, prepares compatible access once, and
shares compact discovery and typed values. It uses wake-driven handoffs, contribution-relevant
completion and stable serving initialization whose lifetime survives optional retention changes.

Authoring baseline: main `d2341cc4`, 2026-10-09, plus the frozen dirty GK/GR/native/serving tree.
The source review inspected `85e8b9cb` plus that implementation; the intervening commit published
its review and handoff, not production corrections. At authoring, the stable SDK-session patch was
**not compiled or runtime-qualified**; its execution receipt now lives in coordinator §9.1.
The operator's hashing references, PC3 worktree, profiling
captures and shared Cargo intermediates remain preserved.

Assessment uses core/template3.3, efficient-architecture heuristics1.0 and code-intelligence1.5
through [the standard](../design_review/design_principles/standard.toml). Existing source review
reasoning is reused. Static foundation inspection and locked-library investigation are sufficient
for this target; neither a numeric benefit nor the dominant timeout cause is established.

Useful implemented foundations are model-owned scope/stage declarations, exact completed
contributions/views, captured supplier bindings, union root discovery, separate owner partitions,
projected native providers, exact checked-premise memoization, independent admission, final
content freeze, spillable ordering and direct ordinary sealing. Their actual consumers are in
[semantic-model §15](../design/sections/semantic-model.md) and
[storage/publication](../design/sections/storage-and-publication.md).

The residual mismatch is physical composition. Native membership demand enumerates absent
owner/key combinations; support checks plan and read overlapping inputs per grain; optional
product domains fetch overlapping tokens; replay repeatedly decodes the same public rows;
synchronous native handoffs poll; contribution completion waits for unrelated scans; replacing
optional retention splits initialization; selected test observation reconstructs all relations.
These mechanisms can amplify a valid workload even with bounded transfer windows.

The workload includes many overlapping roots and producer owners, sparse requested keys,
reconvergent/high-degree support, independent reads, cold execution and enabled reuse. Required
whole-universe analyses and negative admission domains remain whole. Small output is not a bound
on examined input, and refusal is not evidence of a suitable physical route.

### Qualification boundary

- `20261009T170649.056Z-032c30` (`just qualify`) was canceled after21m01s, exit143. Its scoped
  model/analytics/flow passes remain historical. Extract retained an authentication failure and
  timeouts; both core300s matrices were still in their first cache-off attempt. Cancellation is
  not an additional assertion failure. Remaining boundaries were not_run.
- `20261009T163757.029Z-9b0811` reached frozen normalized-artifact invariant admission in both
  profiles and timed out at900s. It does not identify the costly individual check.
- Optional token lookup and warm replay cannot explain the first cache-off attempts. The cold
  access/admission/coordination corrections therefore do not depend on cache improvement.
- The preceding native readiness receipt predates the SDK patch. Source reasoning about cloned
  sessions does not prove the authentication failure's server-side cause.

Exact historical commands and outcomes stay in coordinator §9.1. No result above is a new test,
whole-plan pass, candidate-profile qualification or Measured improvement.

## 2. Operator decisions and authority route

All five outcomes below were explicit on2026-10-09; acceptance of a correction direction does
not establish a working prerequisite or finding closure.

| Source rule impact | Operator decision | Route before dependent implementation |
|---|---|---|
| [RC01](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#rc01) | **Accepted:** exact immutable membership selection may replace mandatory deterministic point reads, preserving conflicts, ordering and terminality | NE0 records a complementary native-access/lifecycle ADR and revises storage §6.2. ADR-0138's canonical native ownership remains; the restrictive reader wording changes, not semantic identity |
| [RC02](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#rc02) | **Accepted:** local contribution completion waits for relevant work; final closure retains global drainage/certainty | The same NE0 decision explicitly distinguishes local completion, content freeze and final closure in semantic §15.11 and storage lifecycle owners before NE6 |
| [RC03](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#rc03) | **Accepted:** share prepared relational/typed representations without sharing semantic acceptance | NE1/NE3/NE4 refine existing model/core mechanisms. Keep exact premises, independent check objects, complete negative domains and reject-before-ingress. SQL statement prohibition remains |
| [RC04](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#rc04) | **Accepted:** stable in-flight ownership is separate from optional retention; use actual wakeups | NE5/NE7 remain within retained ownership/finality contracts. Do not add a task engine, retry framework or compiler/test thread caps |
| [RC05](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#rc05) | **Accepted:** controls declare selected versus complete-inventory observation | NE8 changes harness observation, preserving actual native realization and independent full-inventory controls; no test requirement is silently reduced |

NE0 changes governed owners through the ADR skill, not by editing accepted decision text.
A genuinely conflicting decision revealed during implementation follows supersession with its
surviving obligations carried forward. No further semantic rule relaxation is selected here.

Authoring corrects DESIGN §B3's missing ADR-0140 decision reference to match the already accepted
record and linked owners. That repair is documentation consistency, not adoption of NE0 or a
new claim of product qualification.

## 3. Target responsibilities and shared contracts

### 3.1 Resolve exact declared inputs once

The model's Schedule/Stage declarations remain the authority for required input roles, selectors,
producer groups and visibility. Core resolves them against a completed prefix into one immutable
binding inventory: ordered nominal relations/epoch roles, exact completed view descriptors,
provider/model/profile/configuration and relevant executable/schema identities. An implicit
selector binds its actual frozen epoch; explicit distinct epochs stay distinct even with equal
content. A relation alias alone cannot identify a binding.

Provider registration, logical-plan preparation, product dependency keys and native view
resolution consume this same inventory. Changing a completed prefix creates a new inventory;
no cache entry is updated in place and no checked capability crosses attempts. Keep plain aliases
only when every declared role resolves to the same exact view, as current CompletedInputs does.
Preparation belongs to the existing attempt or pinned reader, not an ambient global catalog.

The existing petgraph dependency metadata may expose affected/reverse closures for a named
consumer. It does not become an effect scheduler or proof of cache validity. Required selector
membership and absence are rediscovered against current exact views. Semantic relation edges,
computation/provenance edges and effect/lifetime relationships retain distinct meanings. Resolve
bindings for real consumers first; a universal graph/state-management engine has no selected
consumer or scope in this plan.

### 3.2 Native access follows actual membership — F01

Replace multi-owner Cartesian MembershipLookup and owner-count-driven payload shrinkage with
one exact membership lowering used by selected hydration, row tokens and overlap/completion
queries. Keep physical record identity distinct from nominal keys and logical completed owners.

The selected first migration candidate is **unary equality key-prefix access**, forced through
`member_keys` using one relation and one semantic key, with exact-owner residual filtering in
checked Rust. Maintain one active native branch; candidate rows stream through existing compact
ordering/dedup owners before rich payload hydration. Do not add native ORDER BY/DISTINCT/OR/IN
or joins to this candidate unless their physical state is separately justified. Payload windows
follow selected distinct pointers/bytes, not the count of owners with no matching row.

This choice follows new locked-source qualification during authoring. SurrealDB3.3 compound
multi-value prefixes construct Cartesian combinations (`idx/planner/plan.rs:323–365`). Even a
bounded key-only IN can use `IndexOperator::Union`, which unconditionally requires distinct
(`plan.rs:605–606`, `planner/mod.rs:315–316`). `dbs/distinct.rs:8–14,27–31` retains processed record
IDs in an in-memory HashSet. Key-list length does not bound that candidate-result state. Thus
"one bulk query" is not the chosen remedy. Unary branches trade more key-level crossings for
avoiding absent owner/key enumeration and result-sized native union state.

Where there is exactly one verified owner, bounded deterministic record-ID lookup remains a
legitimate direct path with no owner/key amplification. Candidate streams that already carry
verified exact-view pointers must not rediscover the same ownership through Cartesian lookup;
retain their private source/view binding and conflict checks through hydration.

Unary prefix access can still examine unrelated owners sharing the key. Stream and charge that
work, filter exact owners before payload reads, and reject resource exhaustion explicitly. For
repeated broad reads where this is structurally disproportionate, NE2 also prepares a compact
exact-view key→pointer/content selection once from actual owner memberships, using existing
spillable ordering and charged independent cursors. It is a derived attempt/view-owned selection,
not another canonical store. It cannot become mandatory whole-view startup for a single sparse
lookup. Batched consumers explicitly retain it where they reuse its enumeration; invalidation is
exact-view replacement, not TTL or mutable semantic-graph interpretation.

NE2's first bounded investigation qualifies actual equality-prefix access and the derived
selection's startup/reuse boundary before switching consumers. Reject a candidate with table
scanning, owner Cartesian preparation or growing native union state; use the exact-view prepared
route where justified, rather than compensating with larger limits or truncation. Atomic field
selection retains scalar equality/containment branches and external ordering until a qualified
alternative establishes a genuine bulk path without those defects.

Empty demand still validates view authority, relation/projection shape and bound owners before
skipping row queries. Foreign owners, wrong nominal relations, inconsistent pointers, duplicate
keys, late errors and cancellation remain rejection/finality obligations.

### 3.3 Prepare admission once; enforce independently — F02

Capture immutable TableProviders and checked typed logical input templates inside the existing
input/scope preparation owner. Share only with complete compatible binding, model/schema/program,
function and runtime/configuration identities. Avoid repeated catalog registration and SQL
parsing for the same fixed input meaning. Direct provider scans handle simple projected/key/field
reads; relational joins/order/filtering remain DataFusion logical operations.

DataFusion55.1 prepared statements retain logical plans, not results or physical execution.
Selected templates use typed Expr/LogicalPlanBuilder or the existing checked read-only plan
boundary; do not expose PREPARE/EXECUTE through the effect-disallowing SQL helper. Physical plans,
streams and validator state remain fresh for each demand. TableScan captures its provider: changing
selected inputs requires a checked typed rebinding of the selected port, not merely substitution
of a parameter or mutation of a shared catalog beneath another grain.

Group compatible scope/context grains within the existing charged working set. Discover and
hydrate their union once, preserve each root partition, and feed independent validator objects
borrowed typed input. Model-owned OwnershipRows::Full inputs remain complete; prepare their
immutable typed/indexed representation or replayable sorted runs once when that removes repeated
native decoding. If an invariant's semantics require revisiting it, retain that fold rather than
claiming one prior acceptance proves another check. Full-universe inputs may spill; this is not
permission to retain every relation richly in memory.

Apply the prepared-input contract to support/invariant admission, receiver/event/binding/callable
admission and existing general Workspace invariants where premises actually coincide. Preserve
current exact-check and reference-constraint memoization; do not coalesce different checks into
one validity bit. Global orphan queries and roots derived from actual stored output remain.
Independent detached imports retain their own admission owner and cannot inherit producer trust.

<a id="product-pipeline"></a>

### 3.4 Reuse compact tokens and one typed product — F03/F04

**Replacement target:** UP3/UP4 attach compatible admitted immutable content through a current
checked binding without canonical replay. The typed-candidate route below remains relevant for
untrusted portable values and cold validation; its implementation-era mandatory fresh-ingress
wording does not require replaying already admitted canonical content. Current attribution,
complete membership/absence, independent semantic predicates and effect certainty remain.

For enabled selected products, fetch union content tokens once per exact input/relation in a
prepared root group. Derive each root's role-associated membership/presence/content domain from
that shared compact inventory. Keep missing and empty outcomes, complete negative dependencies,
context/configuration/model/code identity and distinct root partitions. Preserve durable BLAKE3
identity and exact-byte equality; XXH buckets never certify a match. Complete membership discovery
still precedes token-based reuse. This does not change the cache-off path.

Replace encoded ProductCandidate sections plus repeated JSON/Arrow decoders with a charged typed
candidate. Decode portable rows once; validate canonical field shape, row count, duplicate/global
order and content using the same batches. Current semantic predicates borrow those values through
selected providers; after every required check passes, transfer the same owned batches into fresh
native ingress. A private readiness witness is process/attempt-bound, never portable validity.
Extend this route to whole-stage products and selected normalization/kernel products; keep the
existing eligibility decisions, including Fresh behavioral private-state families and SCC schedules.

All candidate validation and allocations needed for replay occur before registration/mutation.
A known pre-effect invalid optional entry can be invalidated and computed afresh. A failed,
committed or uncertain native effect is fatal through drainage; no fallback/retry conceals it.
Native cache acknowledgement/readback, quota, generation/admin fences and corruption checks remain.

Cold capture is an explicit part of NE4. First consolidate one actual completed-output capture
owner and one canonical encoding route with bounded/optional retention; keep an independent
native readback when it establishes a different stored-state property. Compare capture from
already submitted immutable typed batches against that route. Adopt write-time capture only if
its complete key/content/count fold agrees with actual acknowledged native completion, duplicates
have the same semantics, and charges can transfer without another full resident output. If those
premises cannot be established, retain the single actual-output read and retire only redundant
representations. No producer certificate replaces native completion or artifact admission. This
bounded decision is required before claiming cold capture work was removed.

### 3.5 Wake-driven handoffs and relevant completion — F05/F06

Native operations retain the existing owned router/runtime, submitted-call guards, charged
arguments and terminal acknowledgements. Use event/response notifications that wake synchronous
callers and cancellation, replacing try_recv/sleep polling. A Tokio blocking receive is not a
universal replacement: it must not be invoked on an async runtime worker. Async compiler paths
await coarse flush/completion directly; provider interfaces that genuinely require synchronous
callbacks use their existing blocking execution owner with wakeable handoff.

Register a producer once and reuse its owned registration for kind declarations. Move complete
charged batches into submitted work. Coalesce compatible submissions only where original semantic
order, batch/write error attribution and acknowledgements remain observable. Queue pressure must
wake or refuse explicitly; a canceled caller does not release submitted work or its native client.
Keep native read batches under the same event-driven/cancellation contract. This changes internal
plumbing, not provider meanings or public tool results.

Contribution completion awaits its writers/submitted effects and producing read descendants
whose terminality is necessary for that outcome. New unrelated exact-view reads cannot extend
this local wait. Workspace ProducerOutput/drain ownership is the primary dependency owner; native
completion must require that relevant terminal state rather than merely deleting wait_scans.
Use a private scoped handle or equivalent existing typed owner, not a caller-supplied claim or
store-global "valid" flag. Native reads/writes still register globally for failure propagation and
final drainage. A failure observed by any admitted attempt-owned operation remains fatal where
it compromises final admission, even if another contribution became locally complete earlier.

Distinguish local output visibility, irreversible content freeze, publication and final seal/
abandon. Preserve exact descriptor agreement, frozen mutation fences, retained parent descendants,
late transport checks, acknowledged committed identity and final global drainage. No checkpoint,
resume protocol or early publication capability is introduced.

<a id="stable-preparation"></a>

### 3.6 Stable initialization and deliberate observation — F07/F08

Keep Moka0.12.16 for optional completed-value retention. Give the pinned viewer one stable,
charged live-initialization owner keyed by exact preparation bytes, separate from replaceable
retention generations. An admitted first request atomically installs one flight before spawning;
subsequent same-key requests join it, with independent waiter/cancellation ownership. Pressure
may replace optional retained values but cannot remove/split a live flight. The loader continues
to terminality if a waiter disappears. Completed flights hand their result to current waiters and
optional retention, then remove their exact flight identity; stale completion cannot remove a
new flight. Joining, publication and removal must close the miss/completion race.

Use a small viewer-local lifecycle map/notification owner composed with Moka, not a generic
cache/workflow implementation. Charges bound admitted metadata and work; no test/compiler thread
cap is added. Do not let deferred cache maintenance retain a completed flight's rich result.
Existing preparation permits and request suspension remain application admission mechanisms;
waiters cannot hold the last permit needed by their loader. Closing fences new flights/retries,
drains submitted owners, releases optional retention, waits external value leases and only then
invalidates the reader. Exact pins and evidence references never change beneath a borrower.

The extraction harness declares the inspector's relation demand before observation. Selected
controls read only requested completed families in bounded batches without holding the observer
mutex across native I/O. Complete-inventory/determinism/orphan controls explicitly request that
universe. Neither choice skips workspace admission required by the control's purpose. Pure
transformations retain explicit-value seams; acquisition/native-backed tests retain actual native
realization. Stable process transport/runtime may be shared under one live owner, but attempt-private
databases/output and relevant DDL remain isolated. Setup times do not establish thread exhaustion
or justify shared mutable fixtures, test serialization or larger deadlines.

## 4. Packages and prerequisites

One root owns integration, governed declarations, shared native/core files and disposition.
Independent packages can proceed on working contract slices, but logical readiness does not
permit conflicting writers. Existing GK/GR/PJ/PC/BC/CU IDs are not reset.

| Package | Prerequisite / supplied contract | Delivered behavior and revealing evidence |
|---|---|---|
| **NE0 — decisions and transport baseline** | Explicit RC01–RC05; independent plan-target review resolved | Complementary ADR/owner updates for access/local completion; frozen source baseline. Compile and focused actual native control qualify the existing SDK session-retention patch, concurrent streams, dropped initiator and late failure. Authentication causality remains a separate investigation, not a required invented explanation |
| **NE1 — exact binding and preparation contract** | NE0 decisions; current working model scopes/completed views | One immutable resolved inventory used by provider registration, product dependencies and native resolution; compatibility identity and typed selected-port binding. Different epoch/role/schema/configuration refuses sharing; ordinary finite tests need no database |
| **NE2 — actual membership access** | NE1 exact views; NE0 session/finality baseline for runtime controls | Bounded planner qualification then unary equality/one-owner/verified-pointer routes; compact exact-view preparation where repeated access needs it; shared native consumers migrated. Sparse/missing/foreign/conflicting owners, projection/order, cancellation and late errors preserve independent expected results |
| **NE3 — shared admission inputs** | NE1; NE2 native reads for native migration | Checked logical preparation, union input hydration and partitioned independent checks across support/normalization/general compatible invariants. Full ownership/negative and stored-output orphan domains remain complete; invalid rows outside producer positives still refuse |
| **NE4 — token and typed product pipeline** | NE1; NE3 predicate-input slice; NE2 token access for native integration | Union root tokens; one typed decode through all pre-effect checks and replay; explicit cold capture decision. Missing/deleted domains, wrong-but-canonical products and uncertain effects retain current outcomes; Fresh families stay Fresh |
| **NE5 — event-driven native handoffs** | NE0 lifetime/finality contract; current BC/CU contained plumbing | Wakeable synchronous handoffs and async coarse flush, registration reuse and batch charge transfer across provider/core reads/writes. Queue-full cancellation, vanished receiver and late failed effects settle in retained owners |
| **NE6 — contribution-relevant completion** | NE0 decision; NE1 dependency binding; working NE5 owner/ack slice | Actual producing work terminality gates local completion; independent retained readers no longer block it. Own read/write failure and an unrelated late failure still prevent invalid final admission; final closure drains everything relevant globally |
| **NE7 — stable viewer flights** | Current GK5/GR5 exact pins/close ownership; NE0 retained-session control | Same-key initialization remains one across retention pressure, with independently cancellable waiters and prompt terminal flight removal. Pressure/complete/close races, external borrows and separate pins retain charges and actual native behavior |
| **NE8 — purposeful extraction observation** | Current typed inspector/native workspace contracts; NE5 needed handoff slice | Selected and complete observation are explicit, bounded/coarse, native-backed and independently revealing. Whole-inventory negative controls stay whole; selected controls do not reconstruct unrelated relations |
| **NE9 — integrated retirement and qualification** | All actual migrated consumers; ready GK7/GR6 and surviving PC/CU/BC contracts | Replaced Cartesian/reparse/decode/poll/global-local-wait/flight-split/observer routes removed. One final-source coordinated functional acceptance and applicable leaves; source findings close only on their own evidence |

NE1 enables sharing but does not make every later correction wait for a whole-document milestone.
NE5 and NE7 can proceed alongside NE2/NE3 once their consumed contracts work. NE4 is not a
prerequisite to cold-path correction. NE6 uses a delivered owner/ack slice rather than postponing
all lifecycle work until every polling consumer migrates. NE8 does not authorize weakening the
existing selected controls; it supplies better observation. NE9 preserves release-default and
candidate-profile distinction: BC3 needs its own prescribed gate and cannot inherit a release pass.

## 5. Cross-plan integration and change scenarios

| Existing scope | Target refinement / preserved obligation |
|---|---|
| GK2/GK3 | NE1–NE3 refine exact binding, native lowering and shared admission; model scope meaning, partitions and contrasting finite/native controls remain |
| GK5/GK6 | NE1/NE7 improve actual reader preparation and driver composition; published pins, current evidence and independent admission remain |
| GR2–GR5 | NE4/NE7 refine tokens, canonical typed replay and initialization; cache persistence/fencing/uncertainty and Fresh eligibility remain |
| PC3/PC4 and PG selected readers | NE2 replaces multi-owner point enumeration where amplified; preserve actual cold reconciliation, ordering, exact membership/projection and every terminal check |
| PJ0–PJ3 | Retain fixed envelopes, typed ingress, final freeze, one canonical preparation and direct seal; NE3/NE6 distinguish independent assurance and routine local completion |
| BC1/CU contained execution | NE5 uses shared erased plumbing and small typed leaves; no renewed vocabulary-sized coroutine or unrelated rebuild fingerprint expansion |
| GK7/GR6, PC6/CU6/BC5 and BC3 | NE9 joins surviving final-source obligations. Historical failures remain failures, default remains release until BC3, and authoring does not restart them |

For a new fact family, model declarations, genuine provider behavior and invariant meaning are
its semantic edits; generated backing and input registration follow mechanically. Its arrival
must not force every existing observer, cache or native body dispatcher to enumerate it. Replacing
the native selection mechanism changes the adapter and its evidence, not scope/root meaning.
A new invariant can reuse prepared immutable inputs but owns its new rejection semantics.
Different settings/source/model versions create fresh bindings/products; matching row contents
alone cannot keep a negative dependency valid. Concurrency changes lifetimes through explicit
owners, not unrelated semantic edges.

No schema migration is presumed solely from these internal changes. If NE2's selected derived
representation or a governed contract requires one, schedule its declaration/snapshot/identity
change in NE0/NE2 and rebuild disposable derived state; do not add old readers or dual authority.
Operator state, deployment, real-library/protected/Qwen runs, wheels and push remain held.

## 6. Investigations and alternatives

Investigations below belong to the named packages. No new recurring review, cost model,
benchmark campaign, telemetry service or proof ledger is required.

| Question / owner | Decision evidence and consequence |
|---|---|
| **Native planner and unrelated-owner work — NE2** | Inspect exact3.3 equality-prefix plan and a revealing disposable query when needed. Reject union distinct, Cartesian preparation, whole-match arrays and unqualified materialization. Qualify compact exact-view preparation for repeated demand where scalar-prefix residuals are disproportionate; do not hide startup cost |
| **Grain/full-input compatibility — NE1/NE3** | Trace actual invariant input order, roles, scope/context and negative domains. Select a typed shared representation/sorted replay owner only where it preserves separate checks and stays charged. If one invariant requires new state, retain its independent fold |
| **Cold product capture — NE4** | Execution decision2026-10-09: retain one actual completed-output read and canonical encoding. Submitted transfer values are not an independent fold of acknowledged exact singleton membership, and retaining them through completion would add output residency without establishing that equality. Capture now uses a checked read-only singleton selection, avoiding optional canonical-view metadata. Typed replay removes redundant representation work; no write-time certificate replaces native completion/readback. Focused state identity/refusal controls passed; full matrix qualification remains open in coordinator §9.1 |
| **Session/authentication cause — NE0** | Existing patch must pass actual same-session/lifetime/late-error controls. Historical disposed fixture logs cannot prove server auth cause; reopen attribution only with an actual initiating failure. No token-expiry, query-budget or timeout increase is selected |
| **Transport terminality and WebSocket — NE0/NE9, refined during execution2026-10-09** | Repeated selected reads reproduce gRPC cancellation/h2 failure even in isolation. Independent exact SDK/tonic/h2 inspection establishes skipped physical terminal/status, while causality remains subject to actual corrected reruns. Backport physical drainage at SDK3.3.0; published3.3.2 retains the gap. Current PSE-arrow WebSocket and stock Rust SDK still buffer whole results. A progressive server `query_stream` integration is a future alternative requiring equivalent bounded transfer, cancellation and backup support, not an endpoint-only replacement |
| **Bridge/descendant ownership — NE5/NE6** | Trace where synchronous callbacks execute and which read/write work produces each outcome. Require wakeups without runtime-worker deadlock and terminal leases for relevant descendants. Global error poisoning/final drainage must survive local completion |
| **Flight/retention pressure — NE7** | Same-key request across generation replacement must join the same admitted flight. Deterministic completion/removal/close cases settle ownership; metadata, waiters and external results remain charged. Moka alone does not certify this lifetime |
| **Setup/runtime sharing — NE8** | Keep fresh attempt isolation; share stable transport/runtime/compiled schema preparation only with a suitable process owner. No evidence currently makes thread exhaustion or setup the dominant failure |
| **Graph orchestration — NE1, conditional extension** | Resolve existing declarations for named binding consumers now. Consider reverse closure or operational projection only for a consumer that needs it and can state exact edge/lifetime semantics. Universal scheduler/mutable invalidation remains deferred; reopen on a concrete dependency consumer that ordinary typed owners cannot express coherently |
| **Alternative intermediate placement — conditional decision** | Typed/spilled stages followed by final native materialization remain a credible comparison. Reopen if qualified exact access/preparation cannot serve required membership/admission without disproportionate work. Compare exact epochs, negative domains, spill, cold integrity and final authority; a selected better design may supersede placement decisions, but no parallel backend is scheduled here |
| **Benefit measurement / unresolved enclosing gates — NE9** | Use final-source controls for actual semantic/finality readiness and affected evidence journeys. Quantitative speed/capacity requires representative cold/no-hit/warm/reload measurement later; structural closure need not wait for that campaign. Incomplete served-evidence breadth and dirty transport qualification remain explicitly open until their own controls pass |

The preferred target combines suitable existing Surreal access, DataFusion providers/logical plans,
Moka retention and petgraph/domain declarations under ordinary typed owners. The simplest viable
route shares resolved inputs/representations and corrects specific loops/barriers. Whole-graph
residency or Salsa adoption adds mutable-query/lifetime integration without a demonstrated consumer
and is not selected. Salsa's indexed skill0.28.4 does not qualify workspace0.28.5 behavior.

Library claims are scoped to SurrealDB3.3.0, DataFusion55.1.0, petgraph0.8.3, fixedbitset0.5.7 and
Moka0.12.16. Reused Context7 documentation provides discovery, not pinned proof; the source review
lists exact sources. Additional native distinct qualification above is direct locked registry
inspection. No dependency bump is required by this plan. Any later capability claim follows the
selected skill/Context7 and exact-lock qualification route before implementation relies on it.

## 7. Verification, retirement and completion

The unified target's UP9 is the combined final-source acceptance owner, with this plan's NE9,
GK7/GR6 and surviving PC/PJ/CU/BC controls as explicit contributors. UP7 replaces disposable
fixtures with stable service/logical attempts; controls still execute their decision under test.
UP8 extends retention to canonical reachability/pins, not generic cache eviction. Earlier dated
receipts below are neither rerun nor promoted by this planning update.

During implementation, compile checks and minimal revealing affected controls establish correct
behavior and its first-principles value. Resolve recipes/filters with `just verify --print`, select
actual targets and use cached release-profile Rust. Pure model/finite controls need no database.
Native controls use owned disposable fixtures. Verification observes readiness and does not sync;
refresh native Python only when needed, in a guard-free interval.

| Required scenario | Revealing independent distinction |
|---|---|
| Sparse key, many owners; unrelated/shared owners | Actual selected memberships and conflicts agree with independently enumerated expected owners; no Cartesian or union-seen replacement. Empty and wrong-relation demands retain authority checks |
| Overlap/cycle/high-degree and missing support | Union physical input does not merge logical partitions or omit complete negative universes; absent physical root remains distinct from virtual/empty |
| Invalid actual output/orphan/released ownership | Stored rows outside producer positives and missing qualification still fail independent admission; a producer expected-set oracle is insufficient |
| Different epochs/context/config/model | Equal aliases or payloads cannot authorize preparation/product sharing; no selected-membership history replaces current completeness |
| Cache-off/cold/hit/reload/changed/deleted input | Exact both-profile/frontier outputs, originals, evidence and fresh native ownership; matching positive contents do not hide added/deleted/previously empty domains |
| Canonically valid wrong product; resource/uncertain effect | Current semantic checks refuse before ingress; pre-effect refusal differs from unknown/committed mutation; native cache fencing/readback remains |
| Queue-full/canceled/vanished receiver/late error | Submitted batches, client and charges survive caller cancellation to true terminality; no polling or hidden runtime blocking dependency is required |
| Independent retained reader and own producing reader | Only relevant work delays local completion; an unrelated late failure still poisons compromised final admission; final seal/abandon drain remains global |
| Retention pressure plus same-key loader; completion/retire race | One live initialization, independent waiters/pins, no stale flight erases a new flight, no deferred retention owns a rich result after terminal removal |
| Selected versus whole observation | Narrow inspector avoids unrelated scans; separate inventory/orphan control catches unexpected families/rows; actual native realization remains |
| Detached damage / publication and evidence journey | Independent transport and stored-state admission refuses damaged originals/body/binding/coverage; served references stay on one complete realization |

Relevant boundaries are `model:rust`, `providers:extract`, `compiler:producer`, `compiler:cli`,
`store:rust`, `serving:rust` and `serving:mcp`; narrow each with the actual affected targets/filters.
A pytest filter does not omit serving:mcp's native preparation. NE0 specifically selects
`prepared_streams_share_authenticated_session_and_keep_it_alive_through_drainage`; no whole
native family is required merely to qualify that patch. NE9 integrates final both-profile/frontier,
original/native/backup/restore/MCP controls with the already prescribed coordinator obligations.
Run applicable leaves once at functional completion and assembled `just qualify` at the common
NE9/GK7/GR6 boundary; rerun affected failed boundaries, not independent successful ones by habit.
BC3 profile adoption retains its distinct gate. Do not cap jobs/workers/threads, raise deadlines,
serialize failing tests or narrow semantic universes to obtain a pass.

Retire replaced routes alongside migrated consumers: multi-owner Cartesian lookup, owner-driven
payload windows, redundant catalog/query/decode steps, readiness polling, contribution-global
scan waits, in-flight retention coupling and accidental whole-observer reconstruction. Keep
legitimate single-owner point access, semantic validators, original integrity checks, independent
cold admission, actual cache readback and final global closure. Rebuild incompatible disposable
derived products after an explicit format change; keep no compatibility readers or historical
runtime generations without a current consumer.

Plan authoring runs `just docs-check`, scoped `just turn-end` and an independent target-plan review.
These establish document consistency and Proposed design quality, not implementation. Functional
acceptance and Measured benefit remain separate. Finding closure is owned only by the coordinator
and requires the source-specific evidence; an accepted ADR, quick phase or cache matrix cannot
close unrelated obligations.

## 8. Source coverage and handoff

| Source obligation | Design/package route | Disposition owner |
|---|---|---|
| Native-efficiency F01 / RC01 | §3.2, NE0/NE2/NE9; scalar-prefix/union-distinct qualification | Coordinator §8 |
| F02 / RC03 | §3.1/§3.3, NE1/NE3/NE9; independent Full/grain admission | Coordinator §8 |
| F03 / RC03 | §3.4, NE4/NE9; union tokens/complete root domains | Coordinator §8 |
| F04 / RC03 | §3.4, NE4/NE9; typed replay and cold capture decision | Coordinator §8 |
| F05 / RC04 | §3.5, NE5/NE9; wakeups/coarse handoff/charge lifetime | Coordinator §8 |
| F06 / RC02 | §3.5, NE0/NE6/NE9; relevant completion/final certainty | Coordinator §8 |
| F07 / RC04 | §3.6, NE7/NE9; stable flights and evictable retention | Coordinator §8 |
| F08 / RC05 | §3.6, NE8/NE9; purposeful selected/full observation | Coordinator §8 |
| SDK patch and timed failures / review §1 | §1, NE0/NE9; matching-source transport and independent historical boundaries | Coordinator §9.1; focused SDK control passed2026-10-09, integrated acceptance open |
| Existing sharing / contracts / fidelity tables | §1/§3/§5/§7; retained model, graph, original, admission and pin guarantees | Existing owner/plan obligations unchanged |
| Review graph integration / extension scenarios | §3.1/§5/§6; resolved bindings now, named-consumer trigger for wider orchestration | Selected contract plus explicit conditional investigation |
| Review alternatives/library fit and uncertainties | §3.2/§3.3/§6; exact locked contracts and bounded package decisions | Selected, conditional and deferred routes stated |
| Review A1/A3/A4 and unresolved G3/G5/G7/G8/CI-G2 breadth | §3/§6/§7, NE9; independent design and matching-source enforcement/evidence checks | No enclosing acceptance inferred from authoring |

This table maps obligations; mutable status remains in coordinator §8/§9.1. Source-review
assessments remain dated, with their disposition links transferred. Execution now integrates NE0–NE9 from the actual native/query/lifetime prerequisite slices.
The authoring-only boundary below remains historical; it does not restrict the subsequently
authorized implementation. Operator action and a measured-benefit campaign remain outside scope.

**Independent target assessment, 2026-10-09:** the [fresh plan-target review](../design_review/reviews/design_review_native-execution-efficiency-plan_2026-10-09.md)
accepts the coordinated target at Proposed / Interface-checked strength, with no new blocking
finding. It does not qualify the current implementation, actual query/session/lifetime behavior
or measured benefit; source F01–F08 remain Open only in coordinator §8.

**Authoring checks, 2026-10-09:** `just docs-check` **passed** (360 canonical pages, zero link
errors), after repairing the ADR-0140/§B3 reference mismatch and two new section links.
`git diff --check` and preservation of73 pre-existing unowned files **passed**. Product builds,
native controls and qualification are **not_run** for this documentation-only scope;
[coordinator §9.1](persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07)
retains the combined receipt boundary and historical failures.
