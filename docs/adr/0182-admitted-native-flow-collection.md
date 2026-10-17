---
id: ADR-0182
title: Consume admitted native flow groups under the provider owner
status: accepted
date: 2026-10-17
supersedes: []
superseded-by: null
design: ["§4.2", "§15.1"]
evidence: Interface-checked
revisit: Exact native facts need indispensable whole-module expanded results, or synchronous delivery cannot preserve candidate completeness and callback stopping.
---

## Context

Python collection review P:F02/F03/F05/F06 identifies repeated native-coordinate scans, duplicate
canonical input preparation, disconnected producer cancellation and adapter-owned expansion before
admission. The workload includes many independent uses, large candidate groups, import cycles,
recovered syntax and abandoned consumers. Model attachment, exact captured context, native facts,
provider joins and private contribution completion already have suitable owners. These are physical
collection corrections, not permission to narrow facts or analyzer requirements.

The [collection coordinator §6](../plans/catalog-collection-efficiency-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint)
owns implementation and acceptance. The source review and plan-set review establish the proposed
route; they do not establish implemented closure or measured benefit.

## Options

1. Admit complete expanded ModuleFlow results. This is a simple safety improvement but retains
   every expanded use/candidate/formula before consuming any of them. Smaller typed batches do
   not remove that raw lifetime and repeated shape lookup.
2. Consume complete groups synchronously, retaining charged module correspondence. This bounds
   overlapping expansion by necessary correspondence plus the current complete group, keeps
   callback stopping explicit and avoids an extra queue/worker owner.
3. Add asynchronous buffering or persistent spill. Either can support a concrete additional
   consumer, but adds concurrent results, storage/finality and error ownership unnecessary for
   current one-module-at-a-time collection. Refusal alone is not a useful alternative route.

For operational failure through infallible native visitors, explicit Results throughout every
visitor/algebra helper would duplicate the upstream callback shapes; a silent failure latch risks
continued allocation or successful partial facts. A private typed nonlocal operational unwind,
recognized only at the public fallible runner boundary, preserves the actual cause and native
stack cleanup without changing public module-model outcomes. Salsa already uses typed unwinding
for native cancellation; arbitrary panics must remain panics.

## Decision

Use synchronous borrowed events for definitions, complete use/capture/candidate/reaching groups,
values, regions, predicates and attributes. Move only admitted module correspondence in the
preamble. A group remains producer-owned and charged until the consumer returns; the receiver
retains charged typed ordinal correspondence through its actual last use. Final module coverage
requires successful terminal delivery; group delivery is not contribution completion.

Prepare one exact canonical occurrence stream for flow lookup and the existing model attacher.
Each owns its useful physical representation and reservation; shared preparation grants no
independent admission. Span buckets preserve ordered raw-node multiplicity. Attachment retains
its filtered/deduplicated shape policy while region lowering retains raw statement uniqueness.

Prepare complete predicate operand correspondence before emitting conditions: stable leaf order,
first equal native Atom including a None operand, original tests-first-by-span selection. Preserve
native traversal/cache-admission order, candidate completeness, ordinals, refusal/approximation,
loop expansion and capture timing. Immutable admitted condition backing may be shared only within
its actual module/consumer lifetime; existing model algebra remains semantic authority.

Use the existing attempt ResourceBudget/StateCharge before adapter-owned allocation, including
source copies, indexes, candidate/snapshot/loop scratch, strings, atoms and formula results. Native
engine and model/BDD internals retain their separate ownership; this decision establishes no RSS
or complete opaque-heap accounting guarantee.

Clone the existing provider control for cooperative checks and synchronized native callback
registration. Connect the real Ruff collector and actual Salsa query handle. Native callbacks only
signal cancellation. Parser/fixpoint work remains cooperatively interruptible at its supported
boundaries; retain all native owners until actual return/unwind and provider join.

The public runner returns typed operational and consumer errors separately from genuine module
failure. Private operational unwinding carries only its private marker; the runner recognizes it
and Salsa cancellation, rethrowing unrelated panics. Resource failures remain typed Resource;
consumer errors remain the original cause. A separate private stop marker exits infallible native
visitors immediately on callback refusal. The original consumer error stays outside the unwind
payload, preserving borrowed and non-Send errors; no later sibling preparation may overwrite it.
No operational refusal becomes OutsideProviderModel,
empty successful output or successful terminal coverage.

## Consequences

Repeated canonical acquisition and complete-node examination disappear. Consumed groups can
release expanded payload without retaining the whole module. Necessary correspondence and a
single high-degree complete group still require admission; no candidate/source/thread caps are
introduced. Quantitative speed and memory benefits remain unmeasured.

The receiver must prepare dependency-sensitive correspondence before streaming. Tests collect
the same events under explicit admission; they are not a second production implementation or an
unlimited fallback. Exact predicates and model operations remain shared authorities. No analyzer
fork, dependency, published schema or codebook change is implied.

This accepted decision records the selected ownership contract at Interface-checked strength.
The coordinator records focused functional results separately from native integration and whole
Catalog qualification. Full
Catalog/selected-complete/assembled validation remains deferred under the coordinator; protected
native Unknowns and existing receiving/finality obligations remain independent.
