# Catalog collection efficiency: runtime decisions and safe integration

**Approved target; NI00–NI03 implemented / focused-Tested, 2026-10-17.** Companion to the
[coordinator](catalog-collection-efficiency-plan_2026-10-17.md), whose §6 alone owns current
disposition and protected native checkpoint. This document develops residual functional leads
and integration; it does not restart the delivered runtime redesign or its full acceptance campaign.

## 1. Responsibility, foundations and questions

Native control owns atomic effects, actual incidence, complete attempt cuts, protected receiving,
recovery and physical finality. Compiler-store owns admitted physical ingress and its writer lifetime;
publisher and analyses consume established contracts without manufacturing receiving validity.
The [storage owner](../design/sections/storage-and-publication.md#section-6-1),
[semantic compilation owner](../design/sections/semantic-model.md#section-15-1) and accepted
ADR-0178–0181 retain authority. Model meaning, identity, coverage and partial-result semantics stay unchanged.

The runtime source redesign is delivered: atomic bounded windows, changed-incidence lowering,
operation-held premises, joined demand-shaped producer execution, shared Base/Summary preparation,
snapshot-local descriptor reuse and early checked source finalization. Focused native and pure
controls support those implementations; full consumer acceptance remains deferred. Historical
timeouts do not establish that any of these designs is absent or that the remaining four leads
are the exclusive cause. Ordered ingress already pipelines accepted batches; same-contribution
writer fan-out is not the default remedy.

NI00–NI02's selected decisions are recorded below and in ADR-0184. The attempt's registered read
is retained; packet classification, native first-match lookup, compatible ingress writer lifetime
and exact scalar coalescing are improved. Source reasoning supports avoiding repeated work;
no performance gain has been measured. The coordinator §6 owns actual implementation/acceptance
and the NI03 target/cleanup boundary.

## 2. NI00 — Attempt authorization and complete-cut exclusion

**Selected,2026-10-17: retain ordinary attempt `FOR UPDATE`.** The exact pinned RocksDB source
uses optimistic commit-time read validation, not an exclusive waiting row lock. Registered reads
of one owner plus distinct stripe writes do not alone conflict. Removing that read would lose
independent stale-owner protection without an established concurrency benefit. Preserve all64
cut stripes, original replay, absent-request fencing and terminal references. Expanded owned
controls must cover both disjoint-slot commit orders, intentional same-slot conflicts, cut winning
after an apply body has evaluated, and both overlapping absent-fence commit orders. These are
functional conflict/rollback checks, not latency benchmarks.

Start at `crates/lctx-native/src/control/authorization.rs::owner_effect_plan`: ordinary attempt
authorization selects the shared owner `FOR UPDATE` before `native_attempt_participate` uses the
request's slot. Inspect `control/coordination.rs`, the generated schema authority, all open/terminal
attempt owners, original replay, delayed issuance, freeze/close, reconciliation and retirement.
State which predicates are stable, which can change concurrently, and which read/write conflict
actually serializes each race on the pinned server/SDK realization. Do not assume PostgreSQL lock
behavior or infer safe removal from the existence of admission stripes.

Compare retaining necessary shared locking, an authoritative read with complete slot participation,
or another existing native primitive that localizes ordinary contention. Use the selected skill
and exact server source first, Context7/current primary documentation for library questions as
required by AGENTS. Source semantics may settle the design; a revealing actual native control must
establish a changed persistence/conflict contract. No database feature is adopted just because its
name resembles the desired isolation. No whole-compilation transaction or separate application lock
service is needed.

The decision must close apply-before/after freeze and close, original request delayed beyond its
cut, absent-request terminal fencing, concurrent effects on different slots, request collision,
epoch/incarnation change and recovered terminal reference settlement. Complete cuts and new-hold
retirement exclusion remain mandatory. Equal bytes do not waive new participation. Ordinary
read-only authorization cannot replace a required conflicting effect.

Deliver the selected protocol in existing code, affected callers and architectural/ADR owner if
its guarantee changes. Remove only proven redundant work. Use minimal explicit synchronization
cases for both race orders, exact original bindings and unchanged pending/reference assertions.
Inspect native refusal versus timeout/finality separately. Keep normal threading and deadlines;
a wider lock is not disproved merely by a noisy wall-clock result.

## 3. NI01 — Immutable installation preparation and lookup

**Selected,2026-10-17: pure packet classification plus native first match.** Separate missing
physical-guard acquisition from metadata classification; supplied observations are not refreshed.
Attach each expectation's deferred flag before request hashing and consume it directly. Replace
native deferred `.filter(...)[0]` with pinned `array::find`, preserving original candidate order,
first exact match and NONE. Do not collapse compound numeric candidates or replace necessary
scalar locked reads with ordinary bulk reads. Original-blob mutable completion, parent fences,
new-holder incidence and whole-window rollback remain. New scratch/metadata allocations need
admission on both writer and generic effect routes. Generated control changes alter schema identity;
the owning service must regenerate affected project state under NI03.

Inspect `crates/lctx-native/src/control/schema.surql::native_immutable_install`, generated assets,
`control/immutable_window.rs`, exact equality/collision validation, original blobs, deferred guards,
incidence, new holds and authorized participant windows. Find the actual necessary read/lock set
and repeated array search routes, including equal replay and late conflicting rows. NI00's complete
authorization contract constrains any touched participation logic; independent preparation/index
work need not wait for its implementation.

Compare existing indexed bulk/set operations, operation-local keyed preparation and necessary
scalar locks on the pinned native API. Reduce repeated preparation/search where the route preserves
same-ID full-value equality, collision rollback, legitimate absence/insertion races and all receiving
and lifecycle protection. A record ID/digest alone is not equality. A set read is not a write fence.
Keep bounded window atomicity and exact outcomes rather than merge unrelated windows into one
transaction. Carry actual changed incidence; do not restore unconditional broad predicate rewrites.

Deliver any selected change with all lowerers/generated control assets and actual native consumers.
Admit owned prepared indexes before allocation and retain charges to last use; no permanent
descriptor cache. Minimal revealing cases include duplicate equal records, a late unequal duplicate,
equal rows with a new hold, missing/changed guard, deferred shared guard and concurrent insertion/
retirement. Verify rollback leaves no partial current-window effect while previous windows keep
their exact ownership. Test through existing fixture ownership after NI03 readiness; pure plan/
coalescing evidence alone does not establish server behavior.

## 4. NI02 — Ingress preparation lifetime and exact coalescing

**Selected,2026-10-17: driver-owned writer and exact candidate-position indexes.** One writer
spans compatible ingress batches and physical/original/alias/member phases. Reuse only issuance
and fixed control sizing under exact client/attempt/contribution/producing scope/pool; per-window
guards, identity, authorization and recovery remain fresh. Standalone public calls remain bounded,
and post-freeze completion uses independent preparation. Tagged String/i64/UUID identities index
candidate positions, followed by full original equality; compound/unkeyed values retain ordered
scans. Membership indexes retain every source candidate. Unequal immutable collisions still refuse,
unequal auxiliary entries remain available to native validation, and first order is preserved.
Admit keys/positions/output scratch before growth and retain their charges to final use. No generic
Value hash/ordering, global cache, fan-out or increased limits are selected.

At authoring baseline, `crates/lctx-compiler-store/src/compiler.rs` creates `WindowWriter` in
physical write paths near lines2455/5927. Establish the actual phase/batch/ingress lifetimes and
what each preparation contains before moving it. Compare reusing a writer or a smaller immutable
prepared context for one compatible ingress lifetime with necessary phase-local state. Scope
identity includes exact attempt/generation/epoch, contribution and dependency premises, budget,
ordering, failure and finality. No reuse across independently admitted receivers or changed premises.

The compatible route prepares stable inputs once, pipelines bounded accepted windows and retires
their charges/acknowledgements through existing owners. Preserve schema/profile/batch validation,
ordinal order, pre-mutation refusal, producer finality, exact source EOF, recovered recipient and
interrupted/error cleanup. Identify which state must reset between phases; a long-lived mutable
writer must not accidentally retain flushed bodies or change outcome/reference ownership. TF02's
raw flow lifetime feeds this boundary but does not transfer its admission obligation downstream.

`control/immutable_window.rs::coalesce` currently deduplicates several arrays with
`unique.contains(&value)`. Compare keyed stable deduplication using supported `Value` semantics or
typed domain keys. Preserve first-observed order when consumed, exact equality for non-key fields,
and distinct unequal values that must trigger existing collision handling. Hash collisions never
mean semantic equality. If generic Values lack a sound reusable key, qualify typed branches or
retain the bounded route rather than stringify values or silently overwrite same-ID candidates.
Charge new keys/index storage alongside already admitted construction workspace before growth;
no underestimated admission or second uncharged full copy.

Decision and implementation include all compatible ingress callers, cancellation/refusal paths
and replaced helpers. Use pure adversarial coalescing cases for equal/unequal/nested values and
stable order, plus a focused real ingress case spanning multiple phases/batches with interruption
where changed. Establish once-per-compatible-lifetime preparation and release at final consumer
by explicit counters/lifetime gates if needed; no performance benchmark or production telemetry
platform is required. Do not fan out same-contribution writers or add job/thread caps.

## 5. NI03 — Exact readiness and conditional state cutover

**Approved execution route,2026-10-17: campaign, then cutover.** The operator selected one
wholly owned disposable campaign for focused native/race checks, using the already qualified
server generation and current installer. After actual pass and independent source review,
regenerate exact default installation58abe9ea-2088-4109-95ab-8440a3749c51 once over main/validation
through authenticated service drainage, then run small final-target controls and dispose the
campaign through its owner. Original historical Unknowns remain unchanged; a completed exact
whole-target discard may acknowledge their ownership separately. No default mutation precedes
that pre-cutover evidence, and unknown drainage still blocks destructive action.

DB-free investigation and implementation can proceed while native ownership is unresolved. Before
native effects, read the coordinator's protected checkpoint and the current existing owner/target.
The later attachment `a5bd9a10-6a7f-437a-8245-275f9837f0f2` has no observed native completion or
discard acknowledgement at the transferred checkpoint. Earlier target replacements or host exit
do not settle it. Establish exact authenticated completion or use only an explicitly authorized
recovery/discard route; otherwise report the affected native control blocked and preserve the owner.
This plan's authoring performs none of those operations.

Use current installed readiness and fixture owners, no operator inspection/adoption or scratch
fallback. Do not assume the last documented generation is live. Refresh affected binaries after
source changes. Existing source-shaped readiness names its repair route; never sync while guarded
native workers are live. No new campaign is required merely to reproduce historical failures.

Determine whether selected implementation changes native schema/definitions/identity or projected
state. If unchanged, fresh logical attempts and focused controls may suffice. If state replacement
is necessary, first deliver current installer/assets and revealing pre-cutover controls, authenticate
all required reader/worker/native executor drainage, then use existing exact service/run/storage
owners to regenerate project state from pinned inputs. Unknown drainage blocks destructive action.
Remove obsolete generations/runtime copies/compatibility-only paths without legacy readers or
historical fixture prerequisites. Preserve cited evidence, original failures/Unknowns, warm caches
and current consumers. No broad purge or unrelated target reset is authorized.

Run affected focused integration against the final selected target. Any later code/state change
reopens the affected evidence boundary. New confirmed whole-target discard may settle ownership
separately, never relabel the original Unknown as a successful workload. Full selected-complete,
backup/restore matrix and assembled qualification wait for later planning under coordinator §7.

## 6. PC90/PC99 integration and retained obligations

The root integrates selected analyzer/flow/native changes and reviews actual cross-owner behavior:
one canonical preparation, admitted raw construction, cooperative cancellation, joined provider
ownership, deterministic semantic integration, admitted ingress and complete native cuts. Independent
source judgment and focused controls establish this phase's correctness; native claims need actual
native evidence. Scope-end full compile, warnings-inclusive strict keep-going Clippy and applicable
Python hygiene remain required once, distinct from deferred full validation.

Retain RN04's qualified conclusion: no leak established; historical ~143GB server-memory attribution
remains unresolved. RP01 removed an unnecessary transaction lifetime without proving a memory gain.
Reopen memory investigation only for recurrent growth after authenticated drainage or an identified
allocation without a legitimate consumer. Do not require a mandatory memory campaign, infer RSS
from client grants, purge caches or cap concurrency.

The old runtime [completion record](catalog-runtime-efficiency-completion-plan_2026-10-16.md)
preserves detailed receipts and surviving consumer requirements. Coordinator §6 transfers every
R/E/IR disposition; §7 retains wider model/provider/compiler/native/analysis/publication/lexical/
tooling and assembled acceptance. Reconcile later evidence with actual reached assertions,
not test names, nested phase totals, historical timeouts or a successful regeneration. Full original
copies, independent cold receiving, current-witness eligibility, backup exclusion and source/session
finality remain guarantees even while their enclosing acceptance is pending.

PC99 updates enduring owners and statuses to the evidence obtained. Later paused Catalog-completion
assumptions about original long journeys, SA9 split-intent expectations, persisted eager/singleton
preparation and per-window assurance need reconciliation against ADR-0178–0181 and new evidence.
Document those dependencies; do not execute or close the paused tracks. Phase completion leaves
full validation pending and any blocked/unresolved ownership explicit. Old runtime documents stay
reference-only while immutable ADR/evidence consumers remain; delete only when their last real
consumer is gone, preserving surviving obligations through the current owner.
