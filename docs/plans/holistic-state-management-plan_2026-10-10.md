# Holistic state ownership and bounded execution

**Proposed implementation target · 2026-10-10.** This companion integrates the
[holistic state-management review](../design_review/reviews/design_review_holistic-state-management_2026-10-10.md),
including HS-F01–HS-F10, accepted rule changes and narrower investigations. It develops
corrections to the existing unified implementation, not a replacement state platform.
The [persisted coordinator §8](persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns all scheduled finding dispositions; its §9.1 owns actual execution receipts. Coverage
and prerequisites below are not another progress ledger. Existing UP/NE/PC/GK/GR acceptance
remains open. Authoring this plan performs no production, database or storage operation.

## 1. Outcome, baseline and foundation assessment

Compiling, publishing, inspecting, recovering and serving an exact publication should carry
one understandable chain of semantic authority, owned preparation, bounded physical work and
terminal certainty. Sparse requests must not reconstruct unrelated retained state. Necessary
global graph work remains global, with explicit representation lifetimes. Optional retention
must never become uncharged live ownership or authority to reuse stale results.

The inspected baseline is clean `main` at `5cfc1282`, 2026-10-10, containing production
`71c02e70` and the source review. Previous failed publication/MCP runs and blocked Python
acceptance retain their exact scopes in coordinator §9.1. No fresh runtime evidence was
collected for this plan. Static diagnoses are source-established; new remedies are Proposed,
not Tested or Measured. No throughput, memory or elapsed-time improvement is quantified.

Retain exact immutable views, model-owned relation meaning, current provenance on attachment,
full-byte collision checks, independent cold admission, pinned executable definitions, explicit
negative/empty domains and conservative unresolved-effect protection. Keep the stable native
service, patched progressive gRPC, finite Rust kernels and the existing host lifecycle owners.
These foundations already express the required distinctions. The missing integration belongs
at their operation boundaries, rather than in a universal graph/state/workflow manager.

The workload is one operator with concurrent commands/worktrees, repeated validations and
multiple retained releases; sparse requests coexist with complete projections, high-degree
ownership and long-lived history. Suitable physical choices follow from this workload:

- Shared closure declarations eliminate competing interpretations; current-state reads and
  independent semantic checks remain separate assurance decisions.
- Indexed windows and bounded exact batches replace complete server arrays. Streaming transport
  does not make an upstream array, blocking sort or whole-store export bounded.
- Native retirement pages actual ownership edges, not merely parent counts. Permanent fences
  and retained outcomes have different lifetimes; neither age nor a resolved flag is deletion authority.
- Compiler graph lifetimes follow scheduled consumers. Ranked state uses the existing service
  resource pool and request/session ownership, with brief retention-map synchronization.

Future fact families extend the model declaration and its native/dump adapters; new analytics
declare consumed views and graph preparations; new releases coexist through exact identities.
Neither extension should require another closure interpreter, cache validity authority or daemon.
The standard is core/template3.3, heuristics1.0 and code-intelligence1.5 through
[standard.toml](../design_review/design_principles/standard.toml). Semantic §15, storage §6,
serving §11 and existing package owners remain the enduring architectural authorities.

## 2. Confirmed rule changes and decision route

The operator explicitly accepted all three **holistic-state-management** rule impacts on
2026-10-10. These IDs are distinct from the earlier unified-persistence RC01–RC07.

| Source rule impact | Accepted consequence | Route before dependent implementation |
|---|---|---|
| [RC01](../design_review/reviews/design_review_holistic-state-management_2026-10-10.md#RC01) | Replace whole-main physical logical backup with qualified coherent selected-content export; whole-service cold recovery stays separate | HS0 records the selected-export decision and amends storage §6.1 and the recovery runbook. Supersede ADR-0143's whole-main backup mechanism where conflicting, restating its unaffected identity, admission and recovery guarantees; never edit accepted decision prose in place. HS7 depends on that route and snapshot qualification. |
| [RC02](../design_review/reviews/design_review_holistic-state-management_2026-10-10.md#RC02) | Compact eligible rich terminal history while preserving permanent sufficient fencing, references and unresolved work | HS0 records a complementary native lifecycle decision and owner contract. If exact-outcome retention or original authorization meaning changes an ADR-0143 clause, supersede that clause through the ADR lifecycle. HS6 first establishes issuance fencing and explicit outcome horizons; deletion depends on working consumers and qualified maintenance. |
| [RC03](../design_review/reviews/design_review_holistic-state-management_2026-10-10.md#RC03) | Narrow installation coordination only when replacement preserves delayed-operation ordering and backup/recovery exclusion | HS5 initially retains installation coordination in short claim/page/final transactions. A later narrowing requires a complementary decision/owner update, or supersession of a conflicting clause, and equivalent interleaving controls. Narrowing is optional; bounded actual effects are mandatory. |

HS0 settles exact ADR disposition from the selected contracts before changing effects or stored
formats. This is a scheduled architectural decision update, not a new approval round. Do not
rewrite an accepted ADR to describe Proposed code as implemented. Other corrections refine
existing ownership/resource contracts; update their owners alongside the corresponding changes.

## 3. Shared semantic closure and owned audit

### 3.1 Declare closure meaning once — HS-F01 / HS-F06

The model owns the logical recoverable closure of a publication/completed view: exact
dependencies, contribution/view membership, typed roles, aliases, original evidence and required
products. `lctx-surrealdb` owns its physical mapping and current-state selection. Publisher
inspection, logical export and dump preparation consume this meaning through adapters.
This replaces independent `STATE_TABLES`, `state_query` and `restore::PreparedClosure::keep_row`
interpretations, without imposing one universal state schema on runtime coordination or host files.

Inventory every current family and caller before migration. Express declaration extensions with
their native selector, codec and recovery treatment; enforce exhaustive handling where the
model permits it. A shared list of table names alone is insufficient. Do not move native SQL
into model semantic declarations. Physical identities/backing choices are resolved from the
current pinned snapshot, with their own charges and terminal checks.

An audit captures its exact selection once, then lends that immutable operation-owned capture
to state verification, checksum/encoding and completed-state consumers. Fold canonical content
while streaming when possible; avoid cloning another closure-sized identity representation.
Share selection/preparation, not acceptance results. Actual stored rows, missing/extra selected
memberships, aliases, originals and independent cold semantics must still be checked. Producer
expectations and the same derived expected rows cannot become both implementation and oracle.

The isolated audit patch in `wt/audit-closure` is an unverified candidate, not a prerequisite
or accepted implementation. Reassess it against this contract and HS-F10; preserve the dirty
PC3 worktree, caches and evidence. Neither candidate integration nor successful sharing closes
terminal drainage or independent cold admission.

### 3.2 Join all audit work before releasing authority — HS-F10

Publisher inspection owns the reader pin, session and compiler read owner until every admitted
query reaches a terminal local outcome. Refactor `PersistedCompilerStore::from_publication`
so ownership is available before its first fallible asynchronous closure preparation, or return
that owner with construction failure. A read-only audit owner has no attempt-abandon or maintenance
authority. Existing explicit compiler drain/report machinery supplies the terminal join.

On success, semantic early exit, construction error, cancellation and transport failure, fence
new work, join/drain the admitted background tail, then close the pin and invalidate the client.
Preserve primary diagnosis plus independent drainage/pin/client failures and unknown remote
acknowledgement. Cancellation/drop must transfer cleanup to an owned finalizer rather than
detach it or pretend asynchronous Drop completed. An unresolved cleanup remains protected and
reported; it must not become audit success or permission to retire the pinned closure.

This correction can proceed independently of closure sharing. A deliberately delayed stream
tail plus an early semantic failure is the revealing case; a normal successful audit alone is
insufficient. Also exercise failed preparation before the read owner was formerly returned.

## 4. Complete access without complete server arrays — HS-F02

Migrate native compiler-state selection, reader resolution, derived search and reconciliation
together at their actual consumers. Begin with exact roots/views and use indexed membership,
owner, source or document windows; retrieve bounded payload batches and merge/deduplicate using
the existing Rust/DataFusion machinery. Preserve complete roles, occurrences, aliases, isolates,
originals, provenance and required absence domains. Eligibility precedes quotas and ranking
limits. A later window cannot be omitted because an earlier output window is full.

Remove closure-wide `array::concat`, `array::distinct`, nested result arrays and unbounded `IN`
construction from these paths. An exact-ID batch is a bounded fallback when planner behavior
cannot support the preferred indexed stream. Global algorithms may still need a compact full
selected graph; admit it explicitly and release intermediate native/Arrow/typed forms promptly.
Do not silently turn complete operations into approximate or partial ones.

Qualify actual access paths, including ordering/sort behavior, rather than inferring performance
from index declarations or `stream_items`. Use a multi-window selected universe exceeding the
old array allowance and much unrelated retained content, with deliberately missing/extra roles
and late-window occurrences. Unrelated state must not expand sparse preparation or consume
selected search quota. Statement terminals, outer terminal and physical EOF remain checked.
Attributing the historical concat error to one statement is useful investigation, not a gate
on correcting independently established complete-array amplification.

## 5. Native lifecycle: bounded effects and sufficient retained authority

### 5.1 Retirement has recoverable per-object phases — HS-F03

Native control owns object reachability and monotone `retired_through`. Extend its exact object
guard with a durable retiring phase tied to a retirement invocation/item and lifecycle incarnation.
Stable root content identifies requested work, not authority for an old delayed invocation to
resume a newer retirement. Migrate every immutable insert/reactivation and `hold_many` consumer
with this guard; both incoming and outgoing hold creation must respect the retiring phase.

1. **Claim:** a small guarded transaction rechecks incoming holds and exclusive maintenance
   backup exclusion, captures the retirement cutoff and claims the exact item/incarnation.
   A held object is retained without walking its subtree.
2. **Page:** select a bounded indexed owner-equality page of outgoing holds. Atomically nominate
   those exact children as durable work and remove only those exact holds, recording recoverable
   progress. Every effect checks the same retiring incarnation.
3. **Finalize:** a small transaction proves outgoing holds empty, checks the same incarnation,
   deletes the payload and marks the item done. Only afterward may newer authorization reactivate
   identical content; the old retirement cutoff remains permanent.

During retirement, hold insertion/reactivation refuses with an explicit resumable lifecycle
outcome; callers preserve their own durable operation identity. This temporary object exclusion
avoids an old page deleting recreated deterministic `(owner, object)` holds. Incarnation-qualified
hold IDs are a more invasive alternative, unnecessary unless the selected exclusion cannot meet
the supported workflow. Lost page acknowledgements reconcile the original operation before any
retry; restart reads durable phase/remaining work rather than assuming a page committed.

Keep installation coordination initially, but only around short actual-effect transactions.
Qualify native conflicting writes/guards and page atomicity on the pinned engine; snapshot reads
alone do not prevent write skew. Ordinary logical export uses an exact reader pin and database
snapshot, not the exclusive `native_backup_hold` used by maintenance/recovery. Do not introduce
a database-wide logical-export barrier. RC03 narrowing is an independent equivalent-guarantee
investigation, not a condition for delivering bounded retirement.

### 5.2 Index live state; compact history only behind permanent issuance fences — HS-F05

Add selective native live-state indexes and migrate observation/drain/maintenance consumers for
effects, attempts, pins, backup holds and retirement progress. Establish actual indexed plans
with many terminal rows and few or zero live rows. This can ship before destructive compaction;
indexing or smaller per-ID tombstones alone does not bound history growth or close HS-F05.

Current effects allocate an identity before the intent transaction obtains an installation
epoch. Deleting a resolved or epoch-zero pre-intent fence can let that delayed original CREATE
reappear with fresh authorization. Compaction must therefore cover issuance as well as execution.
Select a native authorization era captured before submission, validated by every allocation and
execution path under its real serialization guard. Retry retains original era/owner/epoch and
must never borrow a new era. Inventory attempt begin, effect intent/reconcile, reader pin,
retirement and maintenance paths before enabling collection.

The initial compaction horizon is explicit drained maintenance: close admission, prove actual
borrower drainage, reconcile outstanding effects and settle named pins/backup obligations;
close the old era and durably advance permanent `closed_through` authority before deleting
eligible history. Collector operations belong to the new era. Checkpoint bounded deletion pages;
interruption repeats eligibility checks and resumes. An online range/owner compactor is deferred
unless a concrete workflow shows maintenance-only compaction inadequate.

Retain unresolved effects, incomplete cleanup/retirement and referenced attempt provenance.
Retain exact committed/outcome receipts while a caller, recovery or evidence consumer requires
them. Publish the explicit outcome-retention/release contract: after its horizon, lookup reports
compacted terminal history, never infers “uncommitted” from absence. Preserve sufficient permanent
authorization fences and object retirement cutoffs after payload deletion/reactivation.
Terminal attempts referenced by contributions/bindings require a sufficient retained identity
representation or migrated consumers before rich record deletion. Time, size, heartbeat loss
and “resolved” alone establish none of these facts.

Era closure also needs a legitimate recovery path for protected incomplete work. An explicitly
admitted maintenance successor receives a new durable operation identity under current authority;
it never refreshes or reissues the old request. First reconcile the original effect and establish
its committed disposition or durable fencing; uncertainty keeps the scope protected and prevents
conflicting successor effects. Then atomically claim only the durable remaining idempotent scope,
bound to the original retirement item/incarnation/cutoff or cleanup obligation. Preserve completed
pages and exact outcomes. A successor cannot reset a done item, revive an old job against reactivated
content or replace the original cutoff. Repeated successors reconcile their own identities and
continue only unclaimed remaining work. Old delayed issuance/execution remains permanently refused.
Migrate normal resume, maintenance recovery and their callers together: same-era resume uses its
original valid identity; post-cut recovery requires this distinct successor, never an implicit retry.

Destructive compaction is gated by the issuance/recovery/outcome inventory and delayed-first-intent
qualification. If that contract cannot be established, keep records protected and HS-F05 open;
do not claim indexed history as a completed substitute. The storage manager invokes native
operations and reports protection; it never purges native records or RocksDB internals itself.

## 6. Selected logical backup within one coherent snapshot — HS-F04

The preferred route is selected indexed reads through one explicit transaction on the existing
patched gRPC connection, directly writing only selected content to the provisional data dump.
Source inspection supports this route: vendored SDK `Transaction::query` preserves its transaction
ID through `Query::stream_items` and remote gRPC context; core3.3.0 reuses the external transaction
across streaming requests; RocksDB3.3.0 captures and reuses a transaction snapshot. Context7 was
consulted first; exact source contracts provide the stronger version-specific evidence.
The [pinned official server](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/rpc/grpc.rs)
binds that transaction to its session. This is source-supported Proposed feasibility, not runtime
qualification of the composed exporter.

Acquire the exact publication pin, open that transaction, then verify the publication, manifest
and definition epoch inside it. Read comparison metadata with transaction-bound `INFO`, not a
separate `export(records=false)` call, which opens a different snapshot. Resolve transitive views,
contributions, memberships, recursively selected aliases, payloads, bindings and original chunks
through HS-F01's owner within this same snapshot. Preserve unexpected selected rows/anomalies
needed for independent admission instead of filtering them away as invalid before export.

Use top-level bounded rows, exact-ID batches or indexed keyset pages on this transaction;
inspect ordering and examined-work premises. Existing contribution/membership/alias/binding/chunk
indexes are candidate routes, not proof of complete selective execution. Do not replace whole-main
spooling with a closure-sized server array or a second uncharged client closure copy.
The SDK transaction is write-capable: the exporter exposes only fixed read queries, then explicitly
cancels it. Check every statement terminal, outer End, physical EOF and transaction cancellation;
drain on failure before releasing pin/session. Unknown cleanup cannot yield an accepted backup.
Retain existing staged-file durability, file/parent sync and completion uncertainty semantics.

Restore remains data-only typed lowering with fresh logical staging and independent admission.
Definition text is comparison metadata; credentials, pins, attempts, selection and coordination
from the dump acquire no runtime authority. Whole-service cold recovery retains its separate
maintenance, security and external-asset contract. Logical backup need not export unrelated
payloads or operational history to preserve that separate capability.

Qualify transaction-bound metadata, snapshot reads across concurrent publication/deletion and
explicit cancellation before switching the public backup route. If a concrete metadata/planner
limitation defeats this route, investigate a small server/core selected-export adapter accepting
one read transaction; include its distribution/terminal ownership in the decision. Closed immutable
segments require an independently justified recovery/reuse consumer and larger layout decision.
Neither alternative permits per-page snapshots or a silent whole-main fallback declared closed.

## 7. Active compiler and serving state

### 7.1 Graph preparation follows actual consumers — HS-F07

The compiler driver owns prepared graphs and bindings, including optional/cache-hit stage
decisions. Retain each only until its last actual remaining consumer; use lexical phase ownership
or explicit release in the existing schedule. Summary, Structural and selected Analytic stages
consume graph preparations before later Analysis/Catalog/Synthesis/Retrieval work. Cache hits
may eliminate a consumer; optional stages may add one. Do not release early based on a fixed
stage number or keep every representation to outer-driver exit. Preserve complete graph universes,
canonical order, exact identities and bounded synchronous kernels with small async adapters.

### 7.2 Ranked retention is charged, immutable borrowed state — HS-F08 / HS-F09

Serving's existing service resource pool owns retained ranked bytes; each request owns construction,
decode and replay scratch under its selected limits. Charge incrementally before constructing row
copies/templates. Keep pending construction request-owned until final packing/template completion,
then publish a fully formed immutable entry into the optional retention map. Final packing failure,
cancellation and rejected pages release their owned pending state. Preserve current cursor binding,
snapshot/session, ranking order, channel and retention policy; no public wire change is premised.

Lookup, expiry and LRU mutation use a brief map lock. A charged immutable borrower keeps its entry
alive outside that lock through replay/encoding; eviction releases map retention without releasing
active borrower charge. Existing request admission/session-pin lifetime covers active borrowing.
Service close fences new requests, drains admitted requests, then clears retained entries. Do not
invent independent durable pins when current request/session ownership already supplies authority.

Pass the selected `ResourceLimits` through continuation decoding and final response admission;
remove the hidden `ResourceLimits::default()` substitution. A configured 128 KiB allowance must
not reject a legal 40 KiB structured page at the old 32 KiB decode default, while a smaller selected
allowance remains enforced. Preserve actual final MCP envelope/duplication admission and errors.
HS-F09 is a local policy correction independent of the retained-entry refactor and native service.

## 8. Execution packages and dependencies

Package order expresses capability dependencies, not a requirement to wait for every prior row.
One integrator owns shared model/native schema, compiler/audit surfaces and final acceptance.
Independent packages may proceed together when editing/maintenance ownership is compatible.

| Package | Prerequisite actually needed | Delivered behavior and revealing evidence |
|---|---|---|
| **HS0 — decisions and shared contracts** | Confirmed §2 choices and existing owners | ADR/owner updates; logical closure and native lifecycle/backup/resource contracts. Enumerate all migrated consumers and stored-format effects before dependent effects. No closure claim from agreement alone. |
| **HS1 — terminal audit ownership** | Existing PC1 drain/outcome primitives; §3.2 ownership contract | Read owner retained through failed preparation and delayed tails; primary/secondary errors preserved; pin/client release follows terminal join. Does not wait for HS3. |
| **HS2 — selected continuation policy** | Existing selected policy and final envelope owner | Explicit policy reaches decode; large/small selected allowances and envelope failure controls. Pure focused controls suffice. |
| **HS3 — closure declaration and single audit capture** | HS0 closure contract; working exact-view codecs/selectors | Model/native/dump adapters and all current closure consumers migrated; one operation-local audit selection, independent actual-state/cold checks. HS1 supplies its finalizer. |
| **HS4 — array-free native access** | Exact existing scope plus HS3 declaration slices consumed by each path | Compiler/reader/search/reconciliation use complete bounded physical routes; multi-window roles/occurrences, late corruptions and actual plans. Individual paths need only their working shared selector. |
| **HS5 — phased native retirement** | HS0 lifecycle/incarnation contract; migrated writer/hold guards; post-cut resume consumes HS6's working maintenance-successor slice | Claim/page/finalize with bounded effects, durable child nomination and lost-ack recovery; high degree, late holds, reactivation and independent-session restart. Crash after a page followed by era closure completes only through the authorized successor, never by refreshing the old request. RC03 narrowing is optional. |
| **HS6 — live access and safe history compaction** | Live indexing uses existing schema; era cut/deletion requires HS0 issuance/outcome/recovery decision and migrated authorization/successor consumers | Selective live observation first; qualified maintenance-era closure, permanent fences and bounded collector afterward. Delayed pre-intent/epoch-zero/retry and post-cut interrupted retirement/cleanup controls, including uncertain original acknowledgement and repeated successors, gate era cut/deletion; references/outcomes survive. |
| **HS7 — selected coherent logical backup** | HS3 recoverable closure; working pin/drain primitives; §6 capability qualification | One transaction-bound selected dump with checked cancellation and unchanged independent restore. Small selected view amidst unrelated state, concurrent publish/retire and injected late/cleanup/durability failures. |
| **HS8 — compiler last-consumer release** | Current GK/GR stage schedule and actual consumer inventory | Graph/binding release on cold/warm/optional-stage routes; exact outputs/order unchanged; no huge coroutine or generic scheduler. Independent of HS5–HS7. |
| **HS9 — charged ranked retention** | Existing service pool/request/close ownership; HS2 selected-policy path | Incrementally charged complete entries and live borrowers; replay outside map lock; eviction/close/cancel/final-pack controls. Optional retention failure cannot manufacture success or invalidate an active borrow. |
| **HS10 — bounded reuse/diagnostic investigations** | Named actual consumers and source review §10 | Resolve §9 questions through existing NE/GK/GR/serving owners; adopt only warranted consumer-local corrections, record bounded deferrals with triggers. Does not delay independently established HS corrections. |
| **HS11 — integrated final-source acceptance** | Actual migrated HS1–HS9 consumers, decision-gated capabilities and relevant HS10 outcomes | One coordinated UP9 acceptance including remaining PC/PJ/GK/GR/NE/CU obligations; current failed native cases and Python follow-up; no timeout waiver, cached test success or historical receipt promotion. |

Shared editing surfaces differ from dependencies: HS1/HS3/HS4 all touch native compiler and
publisher inspection; HS5/HS6 share control schema; HS2/HS9 share ranked serving. Coordinate
those integrations explicitly. Establish closure and lifecycle decisions early; the policy fix,
audit join and graph release need not wait for destructive compaction or exporter development.

## 9. Investigations and bounded dispositions

These questions carry explicit outputs and dependent decisions, not invitations to defer known
amplification until a benchmark proves it expensive. Execution records their results in existing
package receipts; this document does not introduce another investigation register.

| Question from the source review | Required outcome / scope consequence |
|---|---|
| Exact historical concat statement | Attribute failure using retained non-sensitive query phases/receipts where available; HS4 fixes established complete-array routes regardless. No standing tracing subsystem is required. |
| Pinned complete windows and actual plans | Establish keys, continuation/order/dedup and complete roles/occurrences for each HS4 consumer. A failed plan selects bounded exact batches or reopens that lowering, not semantic completeness. |
| High-degree atomic unit and late guards | Qualify HS5's shared writer/retiring incarnation and page transaction. Optional RC03 narrowing requires independent equivalent ordering/exclusion evidence. |
| Selected snapshot export | Qualify HS7 transaction metadata/snapshot/cancel route; concrete failure selects a scoped adapter decision. Whole-main route does not satisfy HS-F04 closure. |
| Terminal classes, replay horizon and permanent fences | Inventory HS6 issuance, referenced attempts, cleanup obligations and outcome consumers; establish distinct maintenance successors for protected post-cut work. Gates era cut and destructive collector, not live indexing. |
| Share audit capture without circular oracle; drain | HS3 separates shared selection from actual/cold checks; HS1 owns all admitted tails. Review the isolated candidate against both, not as pre-approved work. |
| Missing-library ResourceRefused versus UnknownLibrary | Trace failed native journey through selected limits, request admission and response packing; preserve absent-library meaning. Do not presume SDK, persistence or one exception class is the cause. Blocks that journey's acceptance, not unrelated corrections. |
| Actual graph/ranked schedule and borrower pool | Enumerate actual consumers/representations in HS8/HS9, including cache-hit, rejected final packing, eviction and close. Source shape plus revealing controls establish structural correction; measurement is separate. |
| Installed-definition coexistence or maintenance | Qualify additive immutable epoch coexistence; if codec/index/table meaning is incompatible, explicit drained maintenance/rebuild is required. Never overwrite an old pinned meaning or auto-maintain in ordinary attachment. |
| Portable encoded/Arrow/typed overlap | NE4/GR2 assess consuming section streaming only where it removes actual overlap while preserving pre-effect checks, independent admission and existing charges. No new universal product format. |
| Repeated `ProducedEntries::get` diagram hydration | Model owner inspects actual repeated callers. Add a charged per-entry immutable borrow only if those consumers benefit; otherwise document no warranted change and reopen on repeated use. No global diagram cache. |
| Cross-attempt program interner | GK/GR inspect production consumers of existing `Workspace::with_program_runtime`; inject compatible runtime only for real repeated compilation, retaining full equality and charged borrower lifetime. Reopen when such a consumer appears; no new cache validity graph. |
| Reusable SCC scratch | Current SCC materialization is once per graph. Preserve canonical ordering; defer additional scratch retention until a repeated-SCC consumer is identified. This does not defer HS8 release. |

Transport alternatives remain subject to progressive rows, cancellation, terminal errors and backup
contracts; no WS mandate or version move is selected. Exact dependency/member/positive/negative
tokens identify new computations, while old immutable pinned views remain valid. Runtime authority
is distinct. Do not introduce blanket invalidation or a second orchestration dependency graph.

## 10. Cross-plan replacement and cutover

| Existing owner | Integrated refinement; surviving obligation |
|---|---|
| Unified UP0/2/3/4/5/6/8/9 | HS closure, audit finality, selected backup, phased retirement, safe history and composed resources replace conflicting physical routes; exact content, provenance, pin/recovery and one-service acceptance survive. |
| Native efficiency NE1/2/3/4/9 | Shared closure preparation and complete bounded lowering; independent native/cold checks survive. NE handoffs/flights/completion retain their existing targets. |
| Persisted corrections PC1/3/4/5/6 | Audit terminal join, authoritative closure/complete physical selection and data-only bounded restore; retain independent fidelity and planner qualification. |
| Graph GK1/2/4/5/6/7 and reuse GR2/3/4/5/6 | Last actual consumer, charged ranked preparation and conditional local reuse; full membership/absence, equality, exact graph scope and original final acceptance survive. |
| Coroutine CU4/CU6 and compilation BC3/BC5 | Preserve small async boundaries/finite kernels; integrate affected final-source correctness once. Test-profile adoption stays a separate BC3 decision. |
| Storage native delegation / SM8 | Invoke native retirement/compaction through its owner; never filesystem-purge native history. Host service-survival/storage acceptance remains separate. |

Migrate semantic declarations, all physical/portable consumers and installed schema together.
When guard/era/codec changes require a schema migration, use explicit maintenance after actual
drainage, preserve protected evidence/recovery assets and rebuild reconstructible project state
from pinned inputs. Remove obsolete closure interpreters, whole-main logical-export path, hidden
policy defaults and superseded runtime formats after functional validation. Add no compatibility-only
reader or scratch database fallback. No operator data, other repository, service activation or
cleanup is authorized by plan authoring; execution must name the exact owned maintenance scope.

## 11. Verification and completion boundary

During implementation use touched compile checks and minimal revealing controls, resolved from
current `just verify --help` / `just verify --print --select FAMILY[:BOUNDARY]` and actual test
names. Native controls attach the stable validation service with logical isolation; disruptive
schema/restart/compaction controls require explicit maintenance. Pure model/policy/kernel controls
need no database. Use the pinned toolchain/profile with normal available compiler/test threading.
No job/worker caps, increased timeouts or higher response defaults substitute for corrections.

In addition to package cases, preserve the source review's earlier lessons: legal singletons spill
without changing limits; corruption controls reach their intended guard; full structured identity
and secondary causes survive errors; complete intermediates are not assumed exact closure;
actual index paths and executable epochs are checked. ENOSPC, disappearing build directories
and Cargo ownership problems remain environment findings, not inferred database causes or
permission for cleanup.

At functional scope completion run affected controls and applicable non-functional leaves once,
then coordinate HS11 with UP9's final-source `just verify --qualify --cli`. Include previously
failed retirement/publication/MCP and blocked Python follow-up, populated selected export/cold
restore/restart, simultaneous worktree attachment, unresolved operations and definition maintenance.
Capture independent failures and rerun affected boundaries. Do not run a broad journey after every
package or promote earlier narrow passes. Real-library/operator adoption and unrelated BC3
profile qualification retain their separate authorization/owners.

Findings close only in coordinator §8 with source-specific implementation and revealing evidence.
In particular HS-F05 requires both selective observation and qualified history disposal; HS-F04
requires snapshot-coherent selected physical export; HS-F06 does not close HS-F10. Independent
target assessment establishes only the Proposed design. Authoring verification is documentation
publication/diff checks and scoped turn-end; product checks are not_run for this document task.

**Authoring checkpoint, 2026-10-10:** the independent
[target-plan assessment](../design_review/reviews/design_review_holistic-state-management-plan_2026-10-10.md)
accepts the amended Proposed contracts. Its F01 prompted the explicit post-era maintenance-successor
contract and prerequisites in §5.2/§8; coordinator §8 records target-only closure. Production
HS-F01–HS-F10 and prior integrated acceptance remain Open. Documentation verification is recorded
in STATUS; no product/runtime result or measured benefit is implied by this authoring checkpoint.
