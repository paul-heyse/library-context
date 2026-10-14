---
id: ADR-0169
title: Publish partial analytic layers without incomplete combined partitions
status: accepted
date: 2026-10-14
supersedes: []
superseded-by: null
design: [§9.4, §9.8]
evidence: Proposed
---

## Context

Catalog creation decision D05 compares owner incidence and direct concept queries with the
published pair and concept products. Current consumers require source-specific pair evidence,
normalized weights, concept extents/intents and implications. Their enumeration is optional and
bounded, but its result cannot be replaced with an on-demand question without changing consumers.
Co-use preflights aggregate degree-squared work while type/mention previously checked individual
groups and failed the producer. Co-use could also proceed to a partition after omitting a required
layer. That partition represented a different graph from its authored selected recipe.

The operator selected explicit partial results during execution planning on 2026-10-14 and
approved their implementation. The [Catalog coordinator §6](../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint)
owns implementation, evidence and remaining scope. This decision supplements retained analytic
contracts; it changes neither default technique selection nor published record schemas.

## Options

1. Fail the producer when a selected layer exceeds its work limit. This is simple locally but
throws away independent selected techniques and contradicts the existing explicit partial vocabulary.
2. Publish a prefix or cluster the remaining layers. This retains a useful-looking result but
changes source membership, normalized weights and the requested graph without an authored recipe.
3. Admit complete layers independently and withhold combined partitions until every selected layer
is complete. This preserves useful independent evidence and makes incomplete meaning explicit.
4. Replace pair/concept products with incidence and direct queries. This could serve a narrower
consumer, but current evidence and synthesis contracts require the enumerated products. No such
consumer replacement is selected.

## Decision

Choose option 3. Co-use, type and mention each prepare canonical source-owner groups and preflight
checked aggregate sum(degree squared) before pair expansion. The existing limit applies per complete
layer, separately from the Leiden run-work gate. Overflow of required expansion work is work
exhaustion; malformed premises remain errors and allocation refusal retains its resource semantics.

An exhausted layer emits no contribution/pair prefix and publishes its existing LayerResult as
Partial/WorkLimit under the actual Communities invocation. Communities also remains partial.
Complete independent layer evidence and independently selected technique outcomes remain available.
Empty complete layers are valid. If any required selected layer is incomplete, unavailable or
omitted, publish no combined graph, partition, community or dependent community conclusion. Never
renormalize the remaining layers as if they were the complete selected recipe. Preserve separate
layer outcomes when several limits or availability conditions apply.

The authored analytic semantic rule revision changes with this policy. Production and independent
admission/replay use the same declared meaning. Existing current products are regenerated on demand;
no compatibility reader, dual runtime route or retained historical-data prerequisite is introduced.
Weighted PageRank, Leiden resolutions/seeds, exact finite concept/implication products, one-step RCA,
qualified heuristic interpretation and full declared graph universes remain unchanged.

## Consequences

Work admission is bounded at the useful layer grain and cannot accidentally escape through many
small groups. Consumers can retain independently complete evidence without mistaking a reduced graph
for the requested partition. Full pair output still has inherent quadratic growth; this decision
makes no latency, capacity or best-workload claim merely because refusal is safe.

CA03 owns implementation and focused acceptance; the coordinator records their current scope.
Acceptance of this record alone closes no finding or enclosing Catalog journey. Revisit incidence/deferred evidence or direct
concept-query products only when a named consumer accepts that changed output contract. Revisit a
single cross-layer work allowance only with an explicit authored policy and its affected consumers.

## Amendments

2026-10-14 — Explicit operator authorization permits this navigation-only correction: Catalog-plan links now name the replacement Catalog completion coordinator. Decisions and dated evidence are unchanged.
