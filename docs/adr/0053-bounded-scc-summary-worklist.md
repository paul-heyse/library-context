---
id: ADR-0053
title: Use an SCC-local bounded worklist for recursive value summaries
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§B4, §9.9]
evidence: Interface-checked
revisit: Two or more effect, exception or role summary channels need a shared recursive rule with a proved common condition and refusal contract, or the fresh pilot shows SCC-local composition dominates the summary budget.
---

## Context

The finite producer already takes typed source/model/condition inputs and returns cited value
flows, ordered proof steps and typed refusals ([ADR-0050](0050-finite-summary-outcomes.md)).
[§9.9](../design/sections/behavioral-analysis.md#section-9-9) and the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
W12 require a recursion engine before replacing the blanket recursive-member refusal. The
current acyclic caller→callee schedule already uses iterative petgraph SCCs
([ADR-0052](0052-iterative-scc-schedule.md)); the first recursive transfer is one value
relation. A finite base in a cycle may support a path, while a base-free cycle or a budget cut
remains `unknown`.

The [same-state probe](../design_review/evidence/2026-09-26_summary-engine-comparison/README.md)
compared pinned Ascent 0.8.1, datafrog 2.0.1 and petgraph 0.8.3 on a cited finite base, a
recursive SCC, a base-free SCC and reversed input. All computed the same origin reachability
set. The probe did **not** compose BDDs, source proofs or product rows, and it did not measure
performance. Its cap control demonstrated the separate refusal obligation.

## Options

1. **Keep recursive members unknown.** This is safe and smallest, but cannot deliver the
   finite-base path required by Stage 3.
2. **Use Ascent rules.** Its macro supplies semi-naive recursion and is most attractive when
   several mutually recursive relation families share rules, negation or lattices. For one
   value relation it still needs external BDD conjunction, proof identity, cap accounting and
   typed unknown publication, plus an added production dependency and macro-generated
   integration surface.
3. **Use datafrog joins.** Its runtime variables provide explicit semi-naive joins with a small
   dependency, but the caller must key every join, assemble source proofs and impose the same
   condition/refusal limits. This does not reduce the semantic code for the first value channel.
4. **Use a bounded worklist within the existing petgraph SCC schedule.** Chosen for the first
   value channel. It keeps each transfer's condition, source proof and refusal at the same
   point of ownership, reuses the accepted component schedule, and adds no production library.
   It requires bespoke delta propagation and explicit shuffle/cap tests.

## Decision

`lctx-analytics::summaries` will solve the **first recursive value-summary relation** with a
deterministic, SCC-local worklist over the existing attributed call components. A direct or
modeled finite base can seed a component only after its own normal-exit and predecessor proof.
Each new value path composes an attributed call edge, a condition under bounded BDD operations,
declared modality and an ordered source/callee proof. A result or cap is keyed to its source
origin; exhausted work, depth or nodes create explicit `summary_boundaries` unknowns and never
negative transfer claims. Parallel source edges and proof paths remain separate.

Ascent and datafrog are **not production dependencies for this first relation**. Recompare at
the revisit trigger above against the same semantic rows and refusal contract, rather than
extrapolating this narrow reachability probe to all future channels.

> Decision: ADR-0053

## Consequences

This selects the owner and boundary, **not** a completed recursive implementation. The
one-relation probe passed on 2026-09-26, but product finite-base, self/mutual recursion,
condition substitution, proof identity, cap publication, native serving and fresh pilot cost
remain open under W12 and plan order 6. The existing recursive-member refusal stays in force
until a tested producer replaces it. A second recursive channel with a genuinely shared rule
is the point to reevaluate Ascent's library advantages.
