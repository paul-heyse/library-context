# Catalog runtime efficiency: native effects and protected premises

**Proposed, 2026-10-16.** Companion to the
[coordinator](catalog-runtime-efficiency-plan_2026-10-16.md), which owns rules, dependencies and
sole current disposition. This document develops runtime F03–F05 and the memory investigation.
It preserves inherited admission, ingress, recovery and lifecycle obligations without duplicating
their status. Production implementation and native controls are not_run for this new target.

## 1. Responsibility, foundations and target

`lctx-native` owns actual effects, protected physical content, incarnations, sessions and recovery.
Compiler-store and publisher consume these capabilities without manufacturing receiving validity.
The [storage owner](../design/sections/storage-and-publication.md#section-6-1),
[semantic compilation](../design/sections/semantic-model.md#section-15-11) and ADR-0173 own the
current contracts. The model retains identities, meaning and structured completion observations.

Current `control/authorization.rs` captures expected guards/owner/era, commits intent and payload
separately, records terminal knowledge and settles references. `control/coordination.rs` supplies
complete attempt/installation admission and scalar guard checks. `control/schema.surql` already
uses bulk immutable insertion, but touches declared incidence before determining what changes.
`control/live_view.rs` and `installed_view/{receiving,live}.rs` earn receiving/retained protection
and repeat some established premises. These are existing owners to refine, not missing frameworks.

For many bounded windows and concurrent attempts, reduce repeated reads, independently committed
stages and shared predicate writes while preserving complete window atomicity, collision refusal,
cuts and recovery. Share assurance over an actually held operation lifetime, never across a bare
digest or matching descriptor. These changes address FP-07/A4 and H18–H25; semantic validity and
independent admission remain constraints rather than tradeoffs.

## 2. RN00 — Decide the ordinary bounded effect protocol

Inputs are the existing NativeRequest, exact original bytes/bindings, request identity, target,
owner, era, generation, admission stripe, expected incarnations and reference policy. Inspect
ordinary, witness, attachment, cleanup, freeze/close and recovery callers, including cases that
depend on independently visible pending intent. Establish which metadata is durable before the
first remote issue and who can reconcile it after host loss.

Compare the current split with one transaction that authorizes participation and commits payload,
effect/outcome and required references atomically. Do not choose one transaction for extraction,
an entire compilation or unrelated bounded windows. Complete the following case reasoning before
selecting a replacement:

| Case | Required observation/effect |
|---|---|
| Failure before first issue | No inferred remote effect; durable pending metadata remains discoverable if created |
| COMMIT acknowledgement lost | Unknown until exact original reconciliation; no regenerated identity or refreshed payload |
| Reconciliation observes absent request | Commit an exact terminal uncommitted fence; delayed original apply conflicts or refuses on that identity |
| Apply versus freeze/close in either order | Apply is included before the complete cut, or refuses after it; read-only attempt checks do not replace stripe participation |
| Collision late in a mixed window | Entire current window rolls back; earlier committed windows retain ownership |
| Terminal knowledge persisted, reference release fails | Known disposition and exact unreleased reference remain with actual recipient |
| Permanent original era compacted | Existing complete-cut/generation/witness checks establish CompactedTerminal; absence alone is insufficient |
| Host loss during any stage | Persisted pending action identifies original request/recipient; recovery never restarts production |

The decision states every consumer of separately committed pending intent and whether it survives
the candidate. Select atomic apply only if the complete route closes these cases. Otherwise retain
split execution for necessary consumers and specify independently removable owner-read/preparation
and bounded terminal-settlement work. A credible retained route must explain the protection bought
by each remaining stage. Failure to establish that route leaves R:F03 open.

Deliver the decision in the owning ADR proposal/architectural contract and coordinator disposition,
not a permanent protocol register. Source reasoning can settle design; revealing actual native
cases must establish implementation atomicity/reconciliation before replacing the old route.
The selected protocol and complete case reasoning receive a bounded independent design/target
review before RN01 relies on the replacement; resolve material findings at that boundary. Acceptance
of this plan does not accept an as-yet-unspecified atomic protocol.

## 3. RN01 — Deliver the selected effect route and migrate consumers

Consume RN00's settled protocol and apply the approved RC01 decision route before dependent
production replacement. Preserve NativeRequest/RecoveryScope as owners of exact original actions.
Keep admitted ingress, pending receipt envelopes, actual recovery recipient, RetainCaller versus
ReleaseRecovery, permanent-era handling and source-finalizer ownership. An error or Arc is not a
recovery recipient; acknowledging an operation refusal is not a failed join.

For the atomic route, use one bounded apply transaction carrying all RN00-required predicates,
payload, outcome and references. Reuse exact authorizations within their real protected validity;
changing owner eligibility or incarnation invalidates the affected premise. Terminal reference
settlement may group bounded references only after durable terminal knowledge and with exact
failure retention. Unconditional reconciliation/reference work cannot disappear into task Drop.

Migrate ordinary installation, relation/original writes, witness/attachment, completion, cleanup
and recovery consumers together where they participate in the selected protocol. Keep split paths
only for named remaining capabilities; remove replaced helpers and duplicated protocol lowering.
Do not add a compatibility-only route. Generated control definitions and their required peer move
with code; RE80 handles any target-state replacement before final acceptance.

Closure combines source inspection of removed stages with actual pinned controls for RN00's cases,
mixed collisions, repeated/equal replay, partial cleanup and nested finality failure. Demonstrate
that previously committed work remains observable after a later failure. No measured latency gain
or fixed transaction-count SLA is required; actual unnecessary stages must nevertheless be removed.

## 4. RN02 — Actual incidence and necessary exclusion writes

Separate planned values, equal existing content, new content, existing equal holds, new holds and
real state transitions inside the same checked transaction. Derive the changed predicate set from
actual insertion/removal/transition, then perform each required exclusion/guard mutation once.
Retain complete canonical equality and exact old/new physical incarnation checks. Equal payload
with a new owner's hold is a real change, not a no-op.

Retain operation-stripe admission and complete eligibility cuts even when other rows are equal.
The earlier attempt-cut investigation rejected tracked reads alone as phantom exclusion. Similarly,
new holds must exclude retirement, and changed contribution/completion membership must exclude
the competing parent transition. Classify revision consumers before suppressing guard increments;
a metadata-observation revision and an eligibility predicate may have different necessary writes.

Keep the pinned executor's qualified scalar `ONLY ... FOR UPDATE` reads. A record-ID-array query
is not an equivalent locking primitive. Bulk compare/insert remains useful after necessary locks;
batching transport while retaining every needless shared write does not close this finding.

This can improve the current split route independently of RN01. Coordinate their common guard/SQL
files under one integration owner; compose the final actual write set and cut once.

Controls distinguish equal bytes/equal holds from equal bytes/new hold; cover retirement in both
orders, unequal bytes, later incarnations, completed-parent change and a late-window collision.
Inspection must identify which predicate writes disappear and why no consumer needs them.
Existing atomic rollback and cleanup-cut controls remain. No blanket read-only replay or blind
guard suppression is an acceptable shortcut.

## 5. RN03 — Carry complete immutable premises through preserving reads

Use the operator-selected controlled-write threat model in coordinator §3. First identify normal
writers of canonical rows, memberships, selectors, boundaries, epochs and holds. State how their
protocols preserve immutability or invalidate a premise; ensure privileged direct edits require
quiescence/rebuild rather than quietly remaining a supported live mutation route. Update affected
documentation and controls deliberately when their promised administrative-mutation scope changes.
Foreign/imported corruption and ordinary protocol violations remain strict admission failures.

### Receiving protection

A receiving operation consumes an exact source capability and complete manifest/closure. Accumulate
the outcomes of every successful transfer page CAS with actual receiving holds, root-qualified
copies/incarnations, client/pool, installation context and definition epoch. Publish the usable
capability only when its complete exact expected inventory is protected. The last page, a count,
logical digest or source admission cannot certify the whole handoff.

Source protection stays held across all pages and final handoff. On intermediate failure, retain
already acquired receiving obligations and unsettled source obligations with their actual owners;
do not construct a complete receiver or drop partial holds as if nothing committed. Independent
semantic receiving admission still examines its declared domain. The optimization carries physical
protection, not producer semantic success.

### Retained operation scope

Earn the selected protected premise at operation entry and retain it through all preserving
membership windows/borrowers. Include the exact selection, physical correspondence, owners/holds,
epoch/context and last borrower. Recheck properties whose premises can change at their actual
transition, rather than reacquiring full immutable bytes/holds per small window. New independent
owners earn new protection. Context replacement, closed/released owners, foreign clients/pools,
changed incarnations and incomplete source EOF remain invalidating or refusing boundaries.

Use existing native-earned/borrowed-current mechanisms where sufficient. A new global certificate,
digest-only validity label or shared admission cache is not the target. Capability names are
conceptual requirements, not a demand for a type per check.

Controls cover complete multi-page transfer, late-page failure and source lifetime; repeated small
windows under one protected operation; new-owner independence; stale epoch/incarnation; foreign
client/pool; cancellation and EOF failure. A malformed incoming value must still fail independent
admission even if production never demanded it. Source inspection closes redundant assurance only
after the supported mutation boundary and all invalidation routes are explicit.

## 6. RN04 — Memory ownership investigation and selected correction

The approximately 143 GB resident private memory observation is historical and unclassified.
The runtime review verified the default installation's selected build source, cache formula and
accounting exclusions. Source defaults do not reveal effective environment, detected memory,
cache occupancy, allocator retention or active execution. Client ResourceBudget is not whole-server
RSS. The default selected server and disposed E1 build must not be confused.

Start with retained pending/native obligations, transaction/session lifetimes, receive/queue backing
and exact server cache/allocation owners. Assess whether RN01/RN03/RP01 remove a demonstrated
unnecessary lifetime, without crediting an unmeasured memory reduction. Distinguish warm persistent
cache, active query/transaction buffers, unreleased owners, allocator retention and a possible defect.

Deliver a supported decision: a source-established lifecycle defect with its concrete correction;
a specific missing observation requiring authorized diagnostic work; or no established functional
defect with the unresolved attribution and reopening trigger retained. If a correction is selected,
this package includes its narrow implementation and revealing functional control. It cannot silently
defer a demonstrated leak/lifetime defect and still claim that defect closed.

Runtime inspection, if needed to decide a material design question, uses the existing explicitly
authorized maintenance/diagnostic route and named target/consumer. This plan does not implicitly
restart, purge, resize, cap concurrency, install another service or issue shared queries. No heap
or timing campaign is mandatory merely to classify an unmeasured observation. Reopen on recurrent
growth that survives confirmed owner drainage or a source-exposed allocation without a consumer.

## 7. Integration and acceptance

One coordinator owns shared authorization/schema/receipt edits and native integration. RN00/RN03
contract slices can precede dependent implementation without making every native package a global
barrier. Preserve existing ingress and finality adapters and exact pinned versions; any required
library change follows the pin/source-provenance route and affected functional checks.

RE90 carries inherited participant/cut, backup/retirement, wide/parallel/Resource-negative ingress,
exact recovery and corrected provider finality controls. New cases complement those contracts;
they do not replace them with a favorable local mock or rewrite historical Unknowns. Shared model,
trust or transport changes receive their required assembled acceptance at completion, not after
each slice. RE80 replaces affected project definitions/state only under authenticated drainage.
