---
id: ADR-0139
title: Compile model-owned relation scopes and retain exact reuse boundaries
status: accepted
date: 2026-10-09
supersedes: []
superseded-by: null
design: ["§B3", "§14.2", "§15.1", "§15.10"]
evidence: Proposed
revisit: An operation cannot retain its complete negative domain or independent admission when compiled to the selected physical lowering.
---

## Context

The graph/hash review identifies repeated root traversal/hydration (F01) and independently
authored body/coverage scope meaning (F02). The operator accepted RC01/RC02 on 2026-10-09.
Existing immutable native contributions, bounded transfer and pure kernels are foundations;
they do not establish this correction's implementation or qualification.

## Options

1. **Keep per-root selectors and add batching locally.** Lowest initial change, but native,
   relational and finite interpretations continue to evolve separately; preparation and
   decode sharing remain caller-specific.
2. **Model-owned bounded scope programs (selected).** One declaration governs references,
   owned membership, body and context joins. Indexed native/relational and compact graph
   lowerings share intent and retain independent owner partitions. Construction and retained
   compact state must earn their lifetime and remain charged.
3. **General interpreter or replace persistence with an in-memory graph.** Unnecessary
   language/runtime or whole-universe conversion; neither is needed to retain existing
   specialized algorithms and native canonical authority.

## Decision

`lctx-model` owns bounded relation scope intent, complete roots and negative domains,
ordered bindings, body/coverage meaning and kernel/admission revisions. Physical ports are
mechanical lowerings; virtual owner ports activate only from explicit operation roots.
`cpg-core` binds exact completed inputs and drives fresh effects; `lctx-surrealdb` owns native
index/transport realization. Serving uses model and native mechanisms without a core dependency.

Compatible logical roots share discovery and decoded immutable premises while retaining
separate memberships, missing/empty outcomes and actual admission predicates. Supporting
references do not activate owner roots. Preserve first-prefix versus ordered multi-prefix
contracts, identity-bearing arcs, isolates, witness order, five verdicts and optional outcomes.
Compact petgraph/FixedBitSet kernels complement sparse indexed access; neither replaces
canonical graph data or specialized BDD/SCC/FCA algorithms.

Canonical program bytes use the existing tagged scalar framing. XXH3-128 selects in-memory
interner buckets; full byte equality decides reuse. Full BLAKE3 identifies portable programs
and future products. Physical aliases, runtime TypeId and root values are not program identity.
Interner/preparation state is charged and bounded by its actual owner lifetime.

Selective cross-run reuse is now an **accepted target** under the reuse companion, replacing
the deferral in ADR-0138 for that target only. ADR-0138's current native ownership, content
freeze, complete reconstruction, terminal drainage, independent admission and publication
effect contracts remain in force. A reused pure product never grants admitted capability,
replaces provenance or skips fresh publication. Persistent reuse and invalidation are separate
implementation packages; this decision does not create a cache store or activate it.

## Consequences

Scope extension/substitution can be tested against one semantic owner. Overlapping demand
can share transport and decode without changing logical validity. Charged conversion,
outlier handling, cancellation and late failures remain implementation obligations;
quantitative speed claims require measurement. No unrestricted operation language, new
canonical store, compatibility reader or concurrency cap follows.

The [compiler plan](../plans/graph-compilation-kernels-and-hashing-plan_2026-10-09.md) and
[reuse plan](../plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md) sequence
implementation. The [persisted coordinator](../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
retains finding disposition and final qualification. Acceptance of this decision does not
close F01/F02, establish measured benefit or authorize operator activation.
