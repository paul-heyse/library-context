---
id: ADR-0155
title: Coordinate native effects through write stripes and exact predicate guards
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: ["storage-and-publication §6.1", "storage-and-publication §6.3"]
evidence: Proposed
---

## Context

Ordinary native effects currently mutate one installation record, including effects over unrelated
content. The reconciled efficiency track accepts narrowing that coordination only after preserving
all protected predicates and recovery paths. The actual engine investigation established named
record write conflicts and an unguarded range phantom; a read-only era comparison cannot replace
the shared write. The later concurrent analysis run timed out amid transaction conflicts. This
observation motivates implementation but does not isolate a performance cause or prove this repair.
[Native N03](../plans/efficiency-alignment-native-plan_2026-10-11.md#N03) owns the full contract and
[decision evidence](../design_review/evidence/2026-10-11_efficiency-design-decisions/native.md#n03--finite-write-stripes-plus-complete-exact-predicate-guards)
records the engine/predicate investigation. No target-format migration has occurred.

## Options

1. Retain the global installation write. It provides simple shared conflict coordination and stays
   the safe fallback until the complete replacement is implemented, but every unrelated effect
   conflicts at this record.
2. Compare installation state without writing shared coordination. This removes a contention point
   but permits stale-read and empty-range phantom outcomes, so it is rejected.
3. Use finite write stripes for era/admission transitions and exact guards for every object/history
   predicate. This preserves bounded global transitions while ordinary work follows its actual
   owner. It requires a complete incidence inventory and one drained format transition.

FP-07/A4 and heuristics H13/H16/H18 favor operation-sized effects and explicit lifetime ownership.
The stripes are a finite write-conflict mechanism, with no worker, job, thread or runtime limit.
There is no new scheduler or quantitative cost model.

## Decision

The accepted target uses64 format-versioned native write stripes selected from the immutable
operation identity. Intent, payload and same-request recovery write the selected stripe and check
its generation/era/closed-through values. Admission close/open and era cut atomically write all64
stripes with the marker. Closed admission still permits current-era maintenance; remote terminal
proof and host maintenance authority remain separate requirements. Whole transition SQL, bindings
and metadata must fit the bounded native operation before any effect; an atomic cut is never split.

Ordinary installation effects use a durable owner derived from installation identity and operation
UUID, with an exact guard/local epoch. They allocate no global ordinal. Only actual attempt, pin,
backup or lifecycle owners with a named ordinal consumer retain monotone allocation. Object
activation/holds/retirement use exact incarnation and phase; retired/vacant activation additionally
compares the predecessor tombstone revision. Retrying an old request never refreshes authority.
A fenced, uncommitted request may be replanned under a new identity; an ambiguous one may not.

Both ends of every hold write their exact guards. A dedicated backup gate protects the actual
global cold-backup predicate. Outcome references, owner effects, contributions, bindings, cleanup
incidence, retirement items and successor/predecessor references write the exact target/parent/
lineage guard rechecked by their collector. History page horizon, expected revision, cursor and
receipt commit together. An empty scan is safe only when every possible insertion partner conflicts
on the representing guard. The complete N03 inventory includes every history eligibility phase;
hold-versus-retirement coverage alone is insufficient.

All authorization, cleanup, retirement, history, installation and recovery consumers move together
through a closed, actually drained native-format transition. Old scalar retired-through authorization
must not coexist with operation-local epochs. Existing durable request/outcome reconciliation,
committed-effect reporting, resumable pages, explicit logical backup pins and shared-content
preservation remain required. No new permission to alter the shared paused service is implied.

## Consequences

Unrelated native effects can commit independently while finite maintenance transitions retain a
shared write conflict. Safety depends on complete actual consumer migration and functional race,
cut-before-intent, backup, history and lineage controls. Revisit the choice if an incidence has no
sufficient bounded conflict guard or the atomic transition cannot fit its declared operation bound.
The existing global path remains in use until target-format acceptance; accepted design and finite
engine evidence do not establish implemented or tested closure. The [coordinator §4](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns current progress and all source finding dispositions. Other ADR-0145/0146 logical backup,
recovery, publication and migration obligations remain in force.
