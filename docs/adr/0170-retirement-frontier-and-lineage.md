---
id: ADR-0170
title: Discover retirement from pending frontiers and retain exact invocation lineage
status: accepted
date: 2026-10-14
supersedes: []
superseded-by: null
design: [§6.1]
evidence: Proposed
---

## Context

Catalog creation D11/D12 concern repeated indexed retirement discovery and repeated stable-lineage
reads. The existing resume loop uses one increasing cursor for both pending/retiring and retained
items. A newly nominated child below that cursor can wait for another resume while retained work
proceeds. Fresh final queue observation prevents false completion, but the shared cursor weakens
the intended pending-first progress. Each outgoing page also rereads lineage although the current
invocation has already been validated and maintenance successors copy that lineage.

## Options

1. Keep singleton discovery and per-page lineage reads. This retains simple local execution but
repeats crossings and couples two frontiers whose priority and progress differ.
2. Prepare bounded pending head pages, retain a separate retained cursor, and pass exact lineage
through the existing guarded page effect. This amortizes useful discovery while leaving every
claim, incarnation, hold and terminal proof with its current owner.
3. Retain a complete retirement queue or remove page fences after preparation. This adds retained
mutable state or treats observation as effect authority; neither is needed.

## Decision

Choose option 2. Pending/retiring discovery starts at the head and is bounded by the existing native
window and remaining useful demand. One RPC obtains separately ordered exact pending and retiring
state ranges under the existing `(job,state,id)` index, each limited to the complete useful demand.
Merge by native record identity and truncate to that demand; a state transition can yield a duplicate
only when immutable job/object/incarnation agrees. Existing native row/byte checks remain. The exact
engine's plan supplies equality-prefix order and limit; a combined IN predicate instead filters and
sorts the job population. No new index or atomic observation is implied. Candidates are hints and are freshly checked before action.
Discard buffered hints after child nomination, because new children may sort before them. Retained
discovery remains singleton with its own per-resume cursor; observe pending work before entering
or resuming retained work. This is discovery-time priority, not atomic exclusion of concurrent
nominations. The current guarded final queue-empty proof alone establishes completion.

Keep existing effect accounting: claim and finalization cost one unit, child nomination plus exact
hold deletion cost two. Stop a resume when remaining allowance cannot advance a retiring item with
nonempty outgoing holds; repeated head refilling must not create a busy loop.

Extract required lineage from the already validated current open job after recovery. Constructors
and maintenance successors preserve it. Pass lineage into nomination bindings, include it in the
payload identity, and compare it to the transactionally loaded current owner inside each page effect.
Keep current generation/era/epoch/successor, parent and child incarnation and exact hold checks.
Missing lineage is malformed current state, not an old-format fallback. Root fencing is unchanged.

## Consequences

Pending waves need bounded discovery rather than a singleton read per item; lineage needs one
observation per resume rather than one per hold page. Refetch, effects and final emptiness remain
independent, so this is neither a portable grant nor a queue manager. There is no schema migration,
new library, timer, worker cap or historical-data requirement. No speed claim follows without
measurement. Revisit retained paging only for a consumer whose complete priority/progress contract
is simpler with it. The Catalog coordinator owns implementation and focused/native acceptance;
acceptance of this ADR alone closes no finding.
