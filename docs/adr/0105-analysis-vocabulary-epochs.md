---
id: ADR-0105
title: Close immutable vocabulary epochs inside cumulative generations
status: proposed
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15.1, §15.11]
evidence: Proposed
---

## Context

Phase 3 completes facts relations before normalized readers start. Phase 4 composition creates
additional places, predicates, evaluation atoms, conditions and assertion qualifications belonging
to that same vocabulary. Making analysis an ordinary contributor places the facts writer after
analysis while analysis requires the facts checkpoint. Reopening completed tables would instead
change content already covered by a receipt.

This is a source-inspected prerequisite, not an implemented extension. The relevant owners are
`lctx-model::domain::stages` (contributors and schedule), `lctx-postgres::generations`
(completion, grants and receipts), and source-bound readers in `cpg-core`.
[The detailed plan §3](../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#3-prerequisite-immutable-shared-vocabulary)
owns the concrete work and acceptance controls.

## Options

1. **Delay all shared-vocabulary completion until analysis ends.** Few storage changes, but facts
   and normalized checkpoints cannot admit their consumers; the schedule remains cyclic.
2. **Make separate facts, normalization and analysis vocabularies.** Gives ordinary one-shot
   tables, but conditions/places require translation or unions at every consumer and lose one
   semantic authority. More independently changing contracts would replace a lifecycle problem.
3. **Reopen completed canonical tables.** Simple writes, but old receipts cease describing
   immutable content and later vocabulary leaks to earlier readers.
4. **Close declared vocabulary epochs.** A finite lifecycle extension preserves one semantic
   vocabulary, stable IDs, immutable earlier content and ordinary single-writer result relations.
   Its additional physical views, deltas and receipts have one store owner.

## Decision

**Proposed:** use option 4 for the finite vocabulary whitelist in the detailed plan. The model
declares one VocabularyAssembly owner and an ordered sequence of publication groups. Ordinary
observations, coverage, support and results remain one-shot outputs.

Each group includes private typed vocabulary deltas and ordinary result deltas referencing them.
Producer computation finishes with sealed-delta receipts, which grant no stored read authority.
Under the attempt lifecycle lock and deterministic relation locks, the store drains COPY, revokes
delta insertion, verifies frozen content, merges vocabulary, loads ordinary outputs, validates
references and semantic invariants, then atomically records ordinary completion and the closed
vocabulary prefix. Every planned output, including empty ones, participates.

The physical vocabulary base records a storage-only introduction epoch. Identity and semantic
schema do not contain this column. Equal IDs with equal payload deduplicate; any conflicting
payload fails. Existing rows and the declared meaning of existing sets never change.

Source-bound readers receive views with literal closed-prefix bounds and explicit semantic
columns. Importers have no direct read access to growing bases and write only private deltas.
CompletedRelation, permits and receipts bind the same closed prefix. Validation checks both
generation identity and prefix visibility: physical existence in a later prefix is insufficient.
Old receipt verification reads the old prefix; final publication verifies the complete closed
generation.

Retain load-then-generated-FK validation inside group closure and permanent FKs at final seal.
This proposal does not claim the current FKs are deferrable. No full-result in-memory buffering is
required. Successful close drops private deltas and retains receipts. Failed or unconfirmed close
poisons the attempt; the existing abort/unconfirmed protocol remains authoritative.

## Consequences

This extends atomic completion to a small publication group, preserving the phase boundaries,
self-contained generation and terminal failure semantics of ADR-0101. It needs model schedule
support, generated physical lowering, exact grants, prefix-bound provider registration and real
PostgreSQL isolation/failure tests before it is Implemented or Tested.

Acceptance must replace the affected one-shot shared-vocabulary and completion clauses of
ADR-0101 through the ADR lifecycle, carrying its remaining cumulative-generation, resource,
admission and terminal-state decisions forward. This proposed record does not edit or supersede
an accepted record yet. A schema/producer revision and clean rebuild follow implementation;
there are no old-format readers or in-place repairs.

Revisit if a required consumer needs to mutate an earlier semantic row or reads its own unfinished
publication group. Do not broaden this mechanism into arbitrary multiwriter storage to avoid
settling that dependency. Analysis and retrieval embedding consumption have distinct one-shot
writers instead.

The parent [cutover findings](../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
remain open until implementation evidence closes them. The detailed plan's R0 controls establish
prefix isolation, immutable deduplication, future-reference refusal, atomic visibility and terminal
failure. Acceptance of this proposal alone establishes none of those runtime properties.
