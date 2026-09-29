---
id: ADR-0082
title: Represent the codebase as one declared relation model with a single owner per semantic question
status: accepted
date: 2026-09-29
supersedes: [ADR-0024]
superseded-by: null
design: [§15, §B2, §B6, §B10, §3.8, §3.9, §5, §9.9]
evidence: Proposed
revisit: A layer cutover finds a supported semantic question that no declared relation or named policy can express without a second writable copy; the P0 known-answer shapes cannot be answered from transfers, bindings and policy views after phase 4; or generated derivation views cannot answer a served explanation within its response budget.
---

## Context

The [semantic data model target review](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md)
examined the pipeline at `35afc09`. The relational foundation is sound: Arrow contracts, DataFusion
derivations, generated validation, typed wire contracts. On top of it, however, semantic decisions are
repeatedly re-derived per consumer:

| Semantic decision | Independent definitions |
|---|---|
| What counts as a call edge | 8 SQL/Rust sites |
| Argument→formal binding | 1 Rust binder and 3 SQL binders |
| Transfer strength | 8 interpretations |
| Refusal priority | 4 policies |
| Discharge validity | 4 restatements |
| Conditions | 2 parallel representations |
| Serving schemas | up to 5 hand-written statements |

**Places have no type.** They are spelled in seven ways, so configuration, relationship and
explanation features (§14.4, §14.5, §14.7) must add yet another encoding and binder. The review's
probes showed two things:
- the existing relations already answer the known-answer controls;
- they cannot compose read access paths or per-branch supply through a summarized callee (P0).

**The operator's direction (2026-09-29).** Build the ideal consolidated model and its new vocabulary
together, as new core code. Cut over hard, layer by layer (ADR-0084). Make PostgreSQL the single
relational store (ADR-0083). Treat the review's served-fidelity findings as defects the new contracts
close by construction, not as separate repairs.

## Options

1. **Consolidate within existing shapes (simplest viable).** Derive the duplicated decisions from the
   existing registries without a new vocabulary. This removes the second authorities for call edges,
   binders, refusal policy and serving files. It still needs a new place encoding and composition for
   plain-class configuration and per-branch supply (review F06). It is kept as the consolidation half of
   the chosen design, not as the whole.
2. **The external recommendation as written.** A shared semantic model, projections of all four
   constructions, and a generic operator framework with one fixed-point driver. The driver is not
   justified: only two of the twelve propagation engines iterate to a fixed point, and they differ on
   merge, cap semantics and key/witness split. Infrastructure for all four constructions exceeds
   their consumers.
3. **A Datalog core (ascent) owning derived semantics.** It admits a BDD-valued lattice (P3). Its only
   budget is wall-clock time, so refusals are not deterministic (DP-11, CI-08). It has no provenance,
   and it would rewrite Tested Stage-3 semantics. It stays conditional under §14.11.
4. **A salsa query database as the semantic model.** It misses compiler-code invalidation, which is
   the dominant change here. Its persistence is layout-bound, and its version is frozen by ty. It stays
   conditional.
5. **A relation-centric model (chosen).** One declared relation model and a single owner for each
   semantic question. Graphs are an execution mechanism for topology only.

## Decision

[§15](../design/sections/semantic-model.md) is the accepted target. Its binding points:

- **Layers and ownership.**
  - The layers are L0 observations, L1 normalized relations, L2 derived relations, L3 product relations
    and L4 serving.
  - Each relation has one layer and one producing stage.
  - A new pure crate, `lctx-model`, owns declarations, identity, vocabulary, policies, algebra,
    conditions, obligations, the derivation-source registry, the stage table and generated DDL/view
    text.
  - `cpg-schema` is retired by the cutover.
- **One declaration authority.**
  - Each relation declares its columns, a generation-qualified key, an identity recipe, participant
    roles, a coverage scope, polarity, a fidelity class and serving exposure.
  - Generated mechanically from it: Rust and Arrow types, PostgreSQL DDL and constraints, codebook
    foreign keys, validators, graph-catalog and derivation-source entries, serving views, wire DTOs and
    every inventory.
- **Identity.**
  - Recipes take a declared `IdKind` (`lctx-id/v2`); ad hoc tag strings are refused.
  - Identity is the semantic key; provenance never enters it.
  - Occurrences are keyed by module, span and syntax kind. Atoms are keyed by evaluation occurrence,
    predicate and operands, with no provider-internal indices.
  - Entities additionally carry release-independent symbol keys.
- **Vocabulary.**
  - Entities, occurrences, places (root plus bounded access path with an unknown suffix), call sites,
    call targets, resolutions, effective callables and call bindings.
  - The call bindings come from one pure binder over call site, target alternative and signature
    variant.
- **Named policies.**
  - Every semantic question has one owned implementation, with its SQL projection generated or bound
    from it: the owner/caller rule, call admission (`invocation`, `dataflow`, `summary`, `usage`,
    `association`), unresolved status, modality admission, verdict-from-condition, obligation priority,
    discharge validity, atom identity and transfer composition.
  - Consumers select a policy view and never re-filter the catalog.
- **Transfers.**
  - One relation carries the provenance classes `flow_local`, `derived_summary`, `composed`,
    `authored_model`, `provider_summary` and `catalog_field_link`. Models are data; disagreement is
    retained.
  - The kinds are `identity`, `derived` and `control`, with an explicit composition table. An open call
    is an obligation, not a kind.
  - Composition across a call is matched by call site. It uses substitution and bounded existential
    elimination of callee-local atoms.
- **Conditions.**
  - The BDD is canonical. A rendering is bounded DNF from capped satisfying paths with a truncation
    marker.
  - No parallel DNF is computed. Served conditions carry `condition_id`.
- **Obligations, coverage and verdicts.**
  - One obligation codebook and priority.
  - Named deterministic budgets.
  - Coverage scopes gate negative claims.
  - One verdict function, shared by producers, validators and the native executor.
- **Derivations.**
  - Each step and proof relation declares itself a derivation source. PostgreSQL views `derivations`
    and `derivation_premises` are generated over them.
  - Alternatives are separate derivations; premise graphs are acyclic.
  - Witnesses are selected at serve time. One findings emitter records its input invocations.
- **Projections.**
  - Generated from roles and policies, for topology analyses only.
  - They run on `petgraph::Graph` with arc-ID weights, a shared dense index, canonical adjacency in
    both directions, and filtered and reversed views.
  - `nodes`/`edges` stay a materialized navigation and reference catalog.

**Library decisions**

| Decision | Libraries | Reason |
|---|---|---|
| Keep | petgraph `Graph`, `kosaraju_scc` and views; biodivine-lib-bdd; DataFusion; arrow-ipc | Pinned and sufficient |
| Keep our own code | PageRank, bounded simultaneous substitution | petgraph's PageRank ignores weights; biodivine's substitution is single-variable and unbounded (P3) |
| Reject | `Csr`, recursive `TarjanScc` on deep graphs, rustworkx-core, differential dataflow, Delta CDF, graph databases, a generic fixed-point driver | See Options and P3 |
| Keep conditional | ascent, salsa, moka, typed-index-collections, roaring | Their triggers are unchanged |

**Superseding ADR-0024 (proposed condition kernel).** Its kernel clauses carry forward unchanged. They
are owned by [§3.9](../design/sections/behavior-model.md#section-3-9) and §15.7:
- the operations and their bounded preflights and caps;
- verified `given`;
- Merkle node ids and catalog validation;
- test leaves;
- the typed primitive-theory whitelist.

Two clauses change:
- the atom identity recipe no longer hashes the provider's synthetic predicate digest;
- the Stage 2 DNF is no longer computed in parallel, and is only a rendering.

**Stage-3 semantic ADRs.** ADR-0045 and ADR-0050–0064 keep their semantics in force. Their relation
representations move onto transfers, obligations and derivations during phase 4 of the cutover. An
engine retired by the ADR-0084 ablation triage has its record superseded at that time.

## Consequences

- **What gets easier.**
  - A new provider for an existing relation is one adapter plus one policy decision.
  - A new transfer kind is one algebra entry.
  - A condition change is one kernel edit.
  - A served column is one declaration plus its generated migration.
  - A feature needing places or composition (plain-class configuration, facade delegation, per-branch
    supply, "why unresolved") composes existing relations instead of adding an encoding.
- **What gets harder.** The model must be designed carefully up front. The migration rewrites
  producers across every layer and temporarily runs legacy adapters and parity (ADR-0084).
- **The findings are closed by construction.** Review F01–F13 close in the cutover phase the plan
  names, not by acceptance of this record. The
  [cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) owns their disposition.
- **Status.** Implementation is Proposed; the plan's phase exits establish it.
