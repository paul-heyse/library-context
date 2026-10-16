---
id: ADR-0180
title: Share captured Base discovery and exact typed root partitions
status: accepted
date: 2026-10-16
supersedes: [ADR-0176]
superseded-by: null
design: ["§9.9", "§9.4", "§15.6"]
evidence: Proposed
revisit: Exact invocation capture cannot preserve singleton feasibility or once-only mutable production.
---

## Context

Runtime R:F08 and approved RC05 extend ADR-0176's invocation sharing beyond singleton discovery
and width-one typed reconstruction. [Runtime coordinator §6](../plans/catalog-runtime-efficiency-plan_2026-10-16.md#6-sole-disposition-and-current-checkpoint)
owns delivery. This accepted target replaces singleton-only discovery/reconstruction;
unaffected Summary and analytic decisions are carried below. Behavioral execution stays optional.

## Options

1. Retain singleton discovery/reconstruction. This bounds local work but repeats overlapping native
   frontiers and rich typed support. Sharing IPC bytes alone does not remove that repeated work.
2. Share successful spillable discovery and charged typed inputs within the exact invocation, with
   independent root partitions and once-only mutable production. Selected candidate.
3. Catch arbitrary native Resource failures and retry smaller groups, or retain all disjoint rich
   closures. The former can replay uncertain issued work; the latter can lose singleton feasibility.
   Neither is selected. No cross-invocation cache or new scope engine is introduced.

## Decision

Capture each successfully checked native frontier window completely into existing invocation-owned
spillable IPC before local membership adoption. Bind coverage, including complete-empty and reverse
owner presence, to exact provider/context/input keys and source owners. Failed acquisition, cancellation,
incomplete EOF or Unknown never publishes reusable coverage and never permits native retry. Successful
captured windows can serve later overlapping groups without reissuing those native windows.

Share compatible typed union inputs, preserving each exact root's selected rows, negative universe,
coverage, physical/virtual role and literal representation. Prepare all exact partitions before any
mutable read root begins; mutable production executes once in deterministic order. Physical read roots
retain full literals; expression opaque/virtual rules do not substitute for that input. Dynamic class
support may share an unchanged negative universe across names, while field conclusions remain local.

Shrink/retry only classified pure local preparation or unconsumed computation after all issued native
work has checked finality. No already-emitted or partially produced root is replayed. Shared groups
are resource-shaped; singleton remains a feasible route rather than a new fixed root/worker cap.
Retain immutable capture, exact source ownership and charges through the last borrower. Acquisition
of a selected later root can fail earlier; unrelated companions are never hydrated for sharing.

Summary builds one charged borrowed ordered inventory per exact invocation and lazily shares
immutable signature ingredients. Retain all competing rows and original first-use error gates.
Caller-bound ports, binding, admission, pair outcomes and proofs remain independently owned.
Release correspondence after its last SCC/pair consumer. The existing graph schedule, worklist,
Pareto frontier and residual semantics remain unchanged.

Analytics retains PairContribution, LayerPair, CombinedPair, concept/lattice evidence and the dual
FixedBitSet FCA representation. A private admitted ordered-count/universe route avoids redundant
vector/map/sort preparation; the checked public adapter uses the same kernel. Preserve isolates,
touched universe, deterministic order/seed, Leiden policy and exhausted-layer semantics.

## Consequences

Overlapping roots reuse native discovery and typed support without sharing semantic authority.
Disjoint/high-degree roots retain resource-shaped preparation and truthful refusal. Local scratch
remains owned through final consumers and source finality; no compatibility-only runtime path or
historical data requirement is added. Typed sharing and successful native capture are integrated,
and current-source functional/native acceptance remains open. No timing or memory improvement is measured.
