---
id: ADR-0173
title: Admit attempt effects through complete cuts and retain owned recovery requests
status: accepted
date: 2026-10-15
supersedes: [ADR-0145, ADR-0165]
superseded-by: null
design: ["§1.2", "§B2", "§B3", "§B7", "§B12", "§B13", "§4", "§5", "§6", "§6.1", "§6.2", "§6.3", "§11", "§14.2", "§15", "§15.1", "§15.11"]
evidence: Proposed
revisit: Attempt cuts cannot completely fence late effects, recovery loses its actual recipient, or ingress allocates outside its admitted ownership.
---

## Context

The [efficiency review](../design_review/reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md)
requires F03/F07/F08 corrections. RC01/RC03/RC04 directions were operator-approved on 2026-10-15;
the [coordinator](../plans/catalog-efficiency-plan_2026-10-15.md#6-sole-disposition-and-current-checkpoint)
owns implementation and evidence. The execution decision also selects task-owned RocksDB campaigns
and exact request recovery across host loss. Current code has attempt-wide incidence writes,
transport allocations before application admission and cancellation-free recovery retries.

This accepted successor carries the unaffected ADR-0145/0165 decisions below. It
supersedes those records' coordination/recovery directions; unchanged old-plan consumers
retain their dated predecessor references until the operator's later reconciliation. No old plan
is rewritten or historical outcome promoted. Consumer migration and focused native evidence remain required; acceptance alone grants no implementation or qualification claim.

## Options

1. Keep one attempt write address and unbounded recovery reissuance. Simple local checks, but
   disjoint native effects conflict and cancellation cannot stop new recovery attempts.
2. Use attempt-local fixed stripes plus existing effect participants, and explicit protected recovery
   owners (selected). Reuses exact request/outcome/epoch and installed admission owners. Adds bounded
   cut work and small pending-request envelopes, without a producer registry or task executor.
3. Use read-only attempt checks, a host mutex, or per-producer registrations alone. The first two
   cannot prevent native inventory phantoms/delayed RPCs; the third needs another registration cut.
4. Lower only message limits or use a custom decoder after Tonic framing. These do not admit earlier
   buffer/decompression allocations or native conversion. Preserve library decoding behind actual
   pre-allocation admission seams and per-query framing.

## Decision

### Complete attempt admission

Initialize a complete versioned set of 64 exact attempt stripes atomically with a new attempt.
Each binds the exact attempt, generation, original era and positive owner epoch. Ordinary first
intent and unresolved payload write the deterministic operation stripe and read/check the attempt
record; installation admission remains an independent required cut. Existing native_effect rows
are durable participants. Equal resolved replay never reexecutes the payload.

Every transition changing attempt eligibility writes/validates the complete stripe set and exact
attempt record in one transaction. Include content freeze, close, maintenance drainage and relevant
finalization. A successful cut sees all prior admitted effects: an overlapping effect either precedes
and is visible, conflicts, or retries against the new eligibility and refuses. Freeze refuses unresolved
participants except its own transition; close fences new canonical production while preserving exact
terminal cleanup/reconciliation. Missing, duplicate, divergent or exhausted stripes refuse.

Classify incidence by its actual consumer. An exact admitted context contains attempt, effect,
generation, original era and epoch, validated against the durable participant. Share its one stripe
participation across a bounded rows-and-holds window. Preserve the complete incidence-target
projection; suppress only that admitted attempt's ordinary invalidation. Unrelated-attempt mutation
refuses unless a narrow owner supplies its complete cut and transition purpose. Do not replace
per-row attempt touches with per-row stripe fanout.

Explicit eligibility cuts cover open-to-frozen content, initial close, final terminal close,
maintenance closure, semantic admission and any physical-completion update that changes the
predicates consumed by a competing owner. Completion metadata that does not change eligibility
retains its actual narrow predicate instead. Cleanup/reconciliation/reference release retain
object, outcome and history exclusion independently of attempt admission. History accounts for
stripe children in its charged bounded destructive work; one parent is not one deletion.

Classify incidence by its actual consumer. Move attempt-admission invalidation from ordinary shared
attempt touches to the admitted operation stripe, including indirect owner/effect/hold/staging paths.
Preserve contribution/boundary/object predicates and their exact revision semantics. Keep actual
binding-publication revision CAS and completed_revision meaning: it is a narrower shared publication
boundary, not a reason to serialize every payload. Stripe success never establishes semantic or
physical inventory correctness by itself. No worker/thread/concurrency limit is introduced.

History deletes stripe children only after the attempt and its effects/outcomes are eligible behind
permanent issuance fencing; no stripe disappears while delayed issuance can revive work. Preserve
original generation/era/epoch/incarnation and request identities. New candidate installations regenerate
current schema directly; no compatibility-only old-format reader is added.

### Owned recovery and exact host-loss handback

Recovery admission governs new issuance, separately from awaiting an already issued checked terminal.
Replace the empty recovery cancellation context with an explicit independent recovery owner and stop
policy. Conclusive uncommitted conflict permits another attempt only while that owner admits it.
Cancellation never drops an issued future or declares an unknown operation uncommitted.

A bounded owning recovery scope retains two actions: ReconcileOriginalRequest and
ReleaseTerminalReference. It carries exact NativeRequest, target/owner/reference identity, known
commit disposition, actual client/session ownership and charged retained state. It has no payload SQL,
payload bindings or route to restart original production. Transfer to the parent's actual finalization
owner before returning handback; avoid strong cycles. Native effects/outcome refs retain durable database
obligations once intent exists. NativeRequestError, EffectReceipts and Completion are observations, not
ownership recipients. A FactoryHandback error is different: it holds the actual factory capability,
with an explicit resume operation and its independent recovery admission. The caller retains the
factory before awaiting construction, and every constructor/finalizer failure returns that capability.

Compiler OperationAdmission's source-finalization owner retains the bounded recovery scope;
maintenance/direct commands retain an equivalent explicit scope through their checked session
finalization. The scope owns the actual Arc session, reservations, cancellation policy and durable
recipient capability. The scope has only a weak parent/lease link; the factory retains the actual
child and session, while accepted operations retain a lease. Admit scope, child and index storage
before a session clone can issue, and attach the actual session before bootstrap or other first effects.
Serialize settlement of each child, without serializing unrelated native work. Managed fixture attachment receipts
and explicitly bound bare-command native-owner receipts supply that recipient. An absent recipient
refuses before first issue; an error receipt is not a substitute. Pending actions prevent source,
session and signal-owner completion.

Before first remote issue, persist compact exact pending-request envelopes under the existing native
owner receipt/storage scope. Cover absent or uncertain intent through host loss. Bound records to
outstanding work; do not rewrite accumulated history. After observing Committed or FencedUncommitted, durably replace
the envelope with the exact ReleaseTerminalReference action and known terminal disposition
before releasing or transferring the last native outcome reference. The envelope distinguishes
RetainCaller from ReleaseRecovery: the former preserves the exact caller reference, including
when reconciliation fenced an absent intent; only the latter consumes the temporary recovery reference. Failed persistence leaves that witness held.
For a still-pending owner whose exact outcome and original reference are absent behind the
permanent complete era cut, discharge cleanup as CompactedTerminal only after rechecking exact
generation and absence of all atomically retained witnesses. Source/pin markers additionally
require AllocationPending and absence of the allocated object and lifecycle reference. Preserve
CompactedTerminal as the observable result; never relabel generic missing history uncommitted,
recreate history or invent a caller reference. An active/known-committed owner missing its witness
refuses. Every envelope validates its exact operation/policy-derived original reference address.
Only checked release terminal permits settling the envelope. Recovery reports independent settlement
without rewriting the original failure/Unknown. This order preserves committed knowledge even if
closed-era collection runs after a host loss. Store no credentials, SQL or payload. Managed
and bare product entrypoints supply a real owning scope. Failed persistence refuses before issue;
failed handoff preserves Outstanding/Unknown and actual ownership. Explicit authenticated recovery
consumes the recorded envelope, fences/reconciles the exact request, or settles only its exact known
terminal reference under the recorded retention policy. It never refreshes or replays original production SQL. Original receipts stay
original; recovery adds independently attributable settlement. Before allocating an attempt or pin, persist a separate compact source-finalization marker naming
its exact allocation request. The allocation atomically creates its object-retention reference;
its caller outcome reference also remains held. Finalization settles original pending requests,
joins the actual retained query owners, and checks or earns exact source/pin terminality. Persist
that terminal knowledge before releasing either witness. Pins settle before their source attempt.
Live or uncertain pin borrowers prevent automatic release; host-loss maintenance requires its own
authenticated authority. Retained private finalizer readers preserve accepted query registries across
future cancellation, without owning the enclosing factory. Session/pin/source finality cannot be
reported complete while required recovery ownership remains.

Explicit raw schema installation remains excluded-target maintenance work. Await its issued
pipeline without an aggregate serving-request timeout, and explicitly close its authenticated
session on a checked return. Transport uncertainty preserves that session and the target
obligation. A session guard does not retain an interrupted raw-DDL future or grant callable
source/pin recovery: require exact executor drainage and current catalog/index reconciliation,
or authenticated whole-target discard. Session invalidation alone establishes neither index
drainage nor whole-target finality.

### Admitted native ingress

Connection/bootstrap and each query have explicit admitted resource owners. Capability-gated query
framing bounds uncompressed encoded frames, retaining adaptive record batching and honest wide-row
refusal. Configure framing together with transport buffer/decompression admission before those
allocations. Structural pre-admission uses existing Prost wire primitives and generated decoders;
native Value conversion must also have qualified bounds or explicit allocation hooks.

Carry reservations through encoded/protobuf/native overlap, queue and delivered values. SDK-facing
admission interfaces are library-neutral and do not import repository model types. Preserve configured
useful compression, sufficiently admitted wide values, SINGLE/Rows/EmptyValue, ordered terminals,
physical EOF, retained sessions and executor/delivery drainage. Materialize shared byte backing
under destination admission before releasing source leases; owned row queues retain both the
queued and currently delivered row reservations until their separate acknowledgments. Admission
refusal propagates as an error, never as an ordinary closed receiver or successful truncated stream. Missing capability refuses before
issue; no unbudgeted fallback or guessed multiplier establishes the complete guarantee. Exact pinned
patch/provenance and every native consumer migrate together. Incomplete seams leave F07 open.

### Preserved holistic ownership decisions


### One installed service and explicit ownership

Use one authenticated library-context SurrealDB3.3.0 RocksDB user service with stable main and validation databases. Production content, manifests, attempts, guards and pins co-locate in main; synthetic controls and untrusted imports use validation with the same supported schema. Ordinary clients attach compatible installed state and never provision databases, start fallback services or perform schema migration. Installation, incompatible cutover, restart, recovery and privileged diagnostics require explicit maintenance admission and actual borrower/effect drainage. Worktree and command lifetimes release their own attachments, not service storage. Credentials stay private; installation identity and native-extension provenance remain separate.

Retain patched progressive gRPC initially. WebSocket/HTTP alternatives must preserve per-request rows, retraction/error/finality, cancellation, reconnect, export/import and backup contracts before substitution. Share only compatible immutable session context; never mutate a shared client's database selection. Run compiler and test workers with normal available parallelism. Use bounded transfer/backpressure and prompt representation release rather than job/thread or fixed service-memory caps.

### Canonical content and exact views

Rust lctx-model owns typed nominal records, attributed assertions, roles, original correspondence, coverage, uncertainty, identities and invariants. Fixed SCHEMAFULL envelopes plus declaration-derived synchronous adapters validate and lower bounded typed windows before writes. Flexible object bodies preserve missing/NULL, bytes/text, local sums and supplied scopes. Independent cold admission reconstructs complete expected envelopes, bodies, scopes, graph/state and originals. Privileged raw SQL is not a semantic writer or admitted validity.

Canonical payload addresses include explicit model/codec identity, relation, nominal key and full content digest; digest hits compare full canonical values. Nominal anchors contain identity without implying selected existence. Native role edges derive once from payloads and preserve field, direction, position and multiplicity. Exact contribution membership selects payloads. Different views may choose different revisions of one nominal key; unequal payloads in one view conflict. Build compact completed-view resolution by bounded ordered merge, never owner-by-key Cartesian enumeration. Forward and reverse access use the same exact resolver; ENFORCED anchor existence alone is insufficient. Originals retain bounded content/range verification. Append-only codebooks and semantic identity remain independent of physical schema and runtime generation.

### Completion, reuse and recovery

Pending membership is not completed authority. Model-owned declarations bind complete exact source/model/implementation/settings, captured supplier/specification, membership/absence/coverage and provenance premises. Retain complete-empty and optional outcomes. Current attribution never silently becomes an old producer's provenance. Local completion closes and drains its producing descendants; final admission freezes its exact read set and effects without stopping unrelated service writers.

Durable attempts, operation identities, fences and corresponding object guards protect bounded writes, completion, checked attachment, pins and retirement. Snapshot isolation and predicate scans do not serialize arbitrary invariants. Unknown acknowledgments reconcile the original operation before retry; acknowledged committed identity survives later cleanup or cancellation. Client death or heartbeat expiry alone does not prove remote terminality. Recovery retains compatible completed prerequisites and re-executes missing work, without resuming arbitrary pending coroutines or introducing a durable task executor.

Checked attachment may reuse retained admitted content without canonical row replay. It establishes current bindings and pins under exact compatibility/provenance/retention checks; serialized data never grants runtime authority. Untrusted portable products retain typed ingress and independent cold admission. Avoid a second serialized canonical cache without a distinct consumer. Existing exact-equality XXH3 program interning, value/provenance distinctions and finite graph/BDD kernels remain. Configuration, external effects, frontier admission and inference retain their actual owners. Serving preparation shares one stable initialization under its exact pin, with independent waiter cancellation and charged borrowers through actual terminality.

### Publication, search and recovery assets

Publisher accepts only privately constructed admitted artifact/restored authorities. Validate and reconcile their exact views, originals, outcomes, consumed vector values and immutable named definition epoch, then expose an unselected manifest through a short guarded transition. Ordinary compilation performs no self-export/import or canonical reload. Selection is separate. Raw database VIEWER credentials are internal, not snapshot ACLs; ordinary Rust reads, resources, evidence and continuations acquire fresh runtime pins on the exact publication. Compare actual definitions and retain pinned epochs; incompatible schema/service changes use maintenance.

Search eligibility precedes quotas and limits. An unqualified ANN route cannot filter after candidate limits and call itself exact-view search. Use an explicitly identified exact eligible-vector policy until native approximate behavior preserves its declared contract. Model-owned ranking fidelity remains explicit.

Logical backup selects only the exact recoverable publication closure through bounded indexed reads within one transaction-bound snapshot and reader pin. Capture manifest and definition comparison metadata in that same snapshot. Check application terminals, physical EOF and explicit transaction cancellation before accepting staged bytes. Include selected closed content and completed state; protect service/security and external coordination recovery assets separately. Ordinary restore parses supported literal data and definition metadata, never executes imported SQL/DDL/access definitions, and creates fresh staging/guards before independent admission. Imported attempts, pins and publication markers confer no authority. Concurrent same-dump imports share immutable content only. Whole-service administrative recovery is a separate maintenance operation. Preserve structured primary/finalization errors, committed effects and explicit uncertain/orphan disposition.

### Tests, resources and retirement

All native controls use the same stable service with logical attempts. Reuse unaffected immutable prerequisites, not the success of the decision under test. Malformed/corruption controls allocate fresh owned validation records and exact predicates; global DDL/crash controls require maintenance. No ordinary fixture removes a database/namespace or falls back to scratch storage. Pure finite controls remain database-independent. Arbitrary SQL/native MCP is limited to non-sensitive validation maintenance credentials excluded from main.

Native storage owns shared-content reachability, pins, protected evidence obligations, unresolved effects and bounded recoverable retirement. Reference creation and deletion participate in the corresponding guards. The storage manager observes and delegates; it never deletes RocksDB internals or evicts authoritative warm content merely by age. Preserve byte-aware charges, queues and timely scratch release across composed operations.

Dependency direction remains cpg-core -> lctx-surrealdb -> lctx-model. Core owns orchestration/admission, native storage owns mechanics/finality and publisher owns visibility. Arrow/DataFusion and compact graph projections are derived compute/transfer, not another canonical authority. No transaction spans extraction, CPU computation, inference or external files. Preserve pinned acquisition, independent providers/coverage/disagreement, programmatic assertions/evaluation and exact consumed embedding values. Rebuild obsolete owned projections from pinned inputs without compatibility readers, dual canonical writes or historical runtime archives.


### Shared closure and active ownership

The model declares recoverable families and dependency meaning; native storage and portable dump preparation supply exhaustive physical adapters. Audits capture selection once, lend it to independent actual-state checks and retain their read owner through failed preparation, cancellation and all terminal stream tails. Release the pin and session only after explicit drainage. Native readers, reconciliation and search use complete indexed windows or bounded exact batches instead of complete server arrays. Eligibility precedes quotas; global kernels still admit their complete selected graph explicitly.

Compiler preparation follows the last remaining selected consumer, including optional and cache-hit decisions. Ranked retention uses the existing service resource pool, request-scoped scratch and fully formed immutable charged borrowers. Pending construction stays request-owned through final packing. Replay runs outside the retention-map lock; eviction cannot release an active borrower's reservation. Selected policy reaches decode and final envelope admission without hidden defaults.

### Issuance, retirement and history

Native control version 4 separates pre-submission issuance era, original owner epoch, durable obligation and identified executor. A retry retains its original request handle. Permanent closed-through fencing covers both allocation and execution through complete conflicting admission cuts; attempt cuts follow the explicit protocol below. Preserve monotonic epochs and object retirement cutoffs. Keep installation coordination initially, restricted to short actual-effect transactions; narrower coordination requires equivalent delayed-operation and recovery exclusion guarantees.

A fresh retirement invocation claims an exact object incarnation, atomically nominates children while deleting bounded outgoing-hold pages, and finalizes only after an empty proof. Incoming/outgoing hold creation and immutable reactivation respect the retiring phase. The first claimed cutoff and completed pages are immutable. Same-era resume retains original authority; post-cut recovery uses a distinct named maintenance successor after reconciling or fencing its predecessor, claiming only durable remaining scope. Unknown outcomes protect that scope.

Index live effects, attempts, pins, backup holds and retirement progress. Rich history collection requires explicit drained maintenance, permanent issuance fencing, terminal outcomes and released caller/recovery/evidence references. Referenced terminal attempt rows remain sufficient provenance. Each bounded collector page has a recoverable current-era receipt and rechecks eligibility atomically. Missing closed-era detail reports compacted terminal history, never inferred uncommitted execution. No age, heartbeat or resolved flag alone authorizes disposal.

Version-3 cutover preserves immutable content, generation, ownership, outcomes and watermarks. An exclusive checkpointed upgrade owner drains legacy clients, confirms daemon termination, replaces stale owned credentials and transfers the approved maintenance executable. Legacy metadata is permanently fenced; ambiguous old retirement scope is protected or blocks migration, never refreshed against today's incarnation. Ordinary installation refuses incompatible state before effects. Completed-state format 3 is independent and unchanged by these runtime-control changes.



### Preserved prepared-completion and tooling decisions


### Completion and prepared access

The compiler/native/model owners distinguish independently validated contribution completion,
operation-local exact internal view selection and frozen independently retained boundary completion.
A contribution's capability protects actual immutable descriptors, membership/backing/originals,
local aliases, selected positive active incarnations and terminal native results. Exact internal
selectors preserve relation/dependency role, complete-empty versus absent, profile/model meaning and
composed directed aliases; they do not automatically allocate durable member copies or logical roots.
Retain actual parent ownership through borrowers; a descriptor surviving Workspace drop grants no
independent capability. Failure, cancellation or Unknown publishes no reusable success.

A frozen admitted/published operation earns a compound physical boundary over exact contributors and
bindings; logical view descriptors select only their declared content within it. Separately retained
products earn separate boundaries when their lifetime demands it. A retained view's validated exact
selector plus protected completed boundary constitutes its view-local physical realization under
ADR-0152: complete selected contributor/dependency closure, incarnations, codec and full member/backing,
collision and missing/extra validation remain required. Boundary completion alone grants no unchecked
selector authority or wider membership. Freeze selectors/mappings among identity-bearing surfaces;
internal operation capabilities cannot be serialized into retained authority. Useful member/access
indexes are prepared once per actual boundary. Version changed physical selector codec/schema/definition identity
and migrate internal, frozen, admitted/published, retained, sparse, backup and cold/import consumers
together. Keep exact selected-copy and per-selector absence semantics; union authority does not grant
a view extra members. No old-format runtime reader or compatibility authority survives hard cutover.

Normalize logical IDs before `RecordId::Ord`; fold the six canonical v4 JSON families with exact
key/equal-payload deduplication and unequal-payload refusal. Operation-owned immutable sorted runs
are reusable non-consumingly across unions. Bind preparation to complete family dependencies,
physical incarnations/full expected content and interpretation context; logical IDs alone do not
supply authority. Hash only required exact boundary unions, with successful EOF/finality before
publishing identity. Flat v4 remains authoritative; do not concatenate fragment digests. Direct
logical checksum and separately demanded portable export share useful immutable preparation.
Export requires frozen or independently retained read-only content, not merely read admission.
Detached import/restore independently validate actual incoming and installed state.

Remaining native assurance uses charged indexed identity-correlated record/guard/exact-hold windows,
with optional discovery distinct from required refusal and receiving effects still atomically
checking their exact premises. Directed aliases use prepared source adjacency/frontiers, preserving
cross-contributor reachability and audit's extra unreachable-source refusal. Semantic distinctions
do not require another physical stage. No generic proof registry, global compile lock or extra
platform follows; a retained cross-owner witness remains a separate consumer-driven fit decision.

### Preserved lifecycle, mutation and diagnostic contracts

Selected access resolves exact physical copies and contributor aliases under real live readers.
Independent observation may be reused only under owners protecting exact incarnation and validity;
per-layout/range/association admission remains independent. An already completed blob's pin and
independently validated blob-to-piece holds protect its exact immutable declaration/bytes. Check
same active positive incarnation and declaration before/after observation and on reuse; equal replay's
generic guard churn alone does not invalidate those bytes. Pre-completion finalization still captures
its revision and requires the owned recapture protocol. Retirement/recreation and privileged
maintenance cannot reuse stale observations; maintenance requires real quiescence/invalidation.

Equal peers recapture and fully revalidate only after conclusively observed same-incarnation premise
change. The original request atomically earns a bounded typed `applied` or `premise_changed` outcome;
read/validate it while its outcome reference remains held, including reconciliation/replay.
Premise-change records terminal outcome without a blob effect. Missing/mismatched outcome or uncertain
request never authorizes a fresh effect; retirement, ABA and owner fencing remain nonretryable.
Preserve cancellation/deadline ownership and normal concurrency. Continuous equal replay retains
conservative recapture unless transaction-local specialization preserves every owner/incarnation/
declaration, retirement/completion exclusion, incidence predicate and new-hold obligation. A client
precheck or skipped predicate touch is insufficient; the plan owns this bounded decision.

Model-owned bounded diagnostics retain known native cause identities and causal chains without native
implementation types. Opaque/pre-admission identities remain absent. Actual writers/readers migrate
together. Diagnostics grant no release authority and never collapse host, remote, storage or committed
outcomes. Inspection composes real owners and honest unobserved state; it does not reopen maintenance.

### Preserved development-target and tooling contracts

Managed main/validation remain the ordinary default. Privileged schema/permission/destructive controls
require explicitly issued wholly owned remote campaigns. Persist purpose, installation/generation,
endpoint, credential reference, effect scope and consumer before launch. Beta private root sessions
never attach to shared/operator targets. Reuse host admission, managed processes and executable/storage
owners. The parent owns the whole target even if the worker emits no report or creates private databases.

Report command outcome, capture health, worker cleanup, native certainty and target disposal separately.
Authenticate the exact campaign, close admission and drain owned workers/native executors before exact
daemon shutdown and datastore release. Logical DROP alone proves neither drainage nor reclamation.
Bounded disposal-only logical children are journaled and reconciled through their existing owners.
Confirmed whole-target shutdown/discard can settle failed logical cleanup without retaining old data;
Unknown executor drainage retains the obligation. Token substitution journals original placeholders
before mutation and restores through exact authenticated ownership before credential cleanup.

Keep exact SurrealKit 1.0.0-beta.1 development-only, outside product dependencies; product SDK/RPC
remain patched 3.3.0. Direct Any/Sync clients are separately configured/owned. Beta private clients
are contained by the whole target, not claimed as injected sessions. Deterministic native/model-owned
schema assets serve Sync and full ordered module-closure tester projections; no native build dependency
or handwritten semantic equivalent. Render lifecycle-sensitive inputs before hashing, refuse unsupported
combinations and compare actual required definitions/permissions/functions. Canonical installation
retains maintenance ownership; watch/pruning remain disposable-tool concepts. Feature availability,
source recipes and running binary/session qualification are distinct. Preserve explicit current support
limits and all conditional capability decisions; new adoption needs a concrete complete consumer.


## Consequences

Disjoint payloads can avoid the attempt-wide conflict address while actual shared content and
binding publication still coordinate. Cuts do bounded complete work; 64 stripes distribute conflicts
and do not guarantee conflict freedom. Exact host-loss recovery requires compact durable pre-issue
metadata, not another payload journal or generic executor. Ingress may require narrow pinned library
extensions because the current decoder hook is after allocation. Benefits are structural and unmeasured.

Acceptance of this decision is not implementation, focused native qualification or CE90 acceptance.
The coordinator retains all open source findings and required countercases. Owned campaigns contain
schema/protocol tests; shared/operator services and historical Unknowns remain unchanged.
