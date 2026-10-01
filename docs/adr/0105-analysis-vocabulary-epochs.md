---
id: ADR-0105
title: Close immutable vocabulary epochs inside cumulative generations
status: accepted
date: 2026-09-30
supersedes: [ADR-0101]
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

**Accepted target, 2026-09-30:** use option 4 for the finite vocabulary whitelist in the detailed plan. The model
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
This decision does not claim the current FKs are deferrable. No full-result in-memory buffering is
required. Successful close drops private deltas and retains receipts. Failed or unconfirmed close
poisons the attempt; the existing abort/unconfirmed protocol remains authoritative.

### Retained cumulative-generation contract

This record consolidates ADR-0101. Capture input once and build facts, normalized, analysis and
catalog frontiers in one fresh, self-contained attempt. Do not publish or select intermediate
facts; facts-only compilation remains supported. There is no linked external generation, copy/import
stage, legacy-ID bridge or compatibility reader. Generation selection never grants serving admission.
Single-generation retirement and reference closure remain local to the generation.

`lctx-model` owns the FrontierDescriptor: relation closure, scoped coverage, checkpoints,
validators and selectability. Store DDL and lifecycle lower it. Only private validated admission
binding model, schedule, policy, content, exact planned/completed outputs and coverage authorizes
publication. Validate the immutable facts checkpoint before normalization. Required input
invariants pass on frozen receipts before their consumer runs; absence or empty rows cannot
promote Partial, Unavailable or NotRequested evidence to Complete.

Ordinary outputs retain one-shot atomic completion: drain input readers and COPY, take lifecycle
and deterministic output locks, revoke insertion, persist all content/outcome receipts (including
empty outputs), grant completed reads, then acknowledge the transaction. Only a confirmed commit
produces completed handles. Declared vocabulary publication groups replace this whole-table
one-shot rule only for the finite whitelist; unfinished groups and sealed deltas grant no reads.
Final publication still validates the complete cumulative frontier and every planned output.

Private attempt reads bind the live attempt, completed receipts, declared relation set and closed
prefix. They use read-only importer connections and source-bound tables; an arbitrary matching-schema
provider cannot authorize a read. Published-reader authorization is separate. Relation availability
and origin travel with the capability. Leases protect all streams through drain; stages close
readers before exclusive transitions. Retained table handles refuse after close.

One AttemptRuntime accounts for capture, facts, computation, retained inputs, graph buffers,
reads and validation. Shared reservations remain charged for retained ownership and through
connection drain. Preserve the current declared row, batch, memory and connection ceilings;
they are reservation ceilings, not a total-RSS promise. Importer provider capacity is reserved
alongside SQLx capacity under the non-superuser importer role. Before execution, atomically admit
the physical plan's complete remote-scan demand, including repeated scans/partitions. Unknown or
excessive demand refuses before any scan; partially started joins cannot wait for capacity.
No ambient disk spill is admitted; a future spill policy needs its own measured consumer,
attempt-owned directory, quota and cleanup contract.

Shared Closing/Closed state prevents acquisitions through every session clone. Cancellation guards
start before transport awaits; confirmed drains return capacity only after acknowledgement.
Transport loss or an unconfirmed bounded drain makes the session terminal, with no reconnection.
Failed or unconfirmed completion poisons the attempt; no retry repairs it in place. Existing
acknowledged abort/cleanup and unconfirmed recovery semantics remain authoritative.

The [Phase 3 protocol §§6–7](../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#6-cumulative-frontiers-and-private-stage-reads)
owns the established controls/resource envelope; Phase 4 extends it through closed prefixes.
The earlier rejection of whole-input memory handoffs, linked generations and an intermediate
facts-publication/import protocol survives: they increase retention or cross-generation lifecycle
burden without a named current consumer. Reconsider them only for measured rebuild cost and a
selected incremental consumer, through a new decision.

## Consequences

This extends atomic completion to a small publication group, preserving the phase boundaries,
self-contained generation, resource/admission and terminal failure semantics restated above. It needs model schedule
support, generated physical lowering, exact grants, prefix-bound provider registration and real
PostgreSQL isolation/failure tests before it is Implemented or Tested.

ADR-0101 is superseded and retired after transferring its surviving clauses here. The rejected
clause is whole-table one-shot completion for the finite shared-vocabulary whitelist; ordinary
relations keep it. The accepted target is authorized by the unchanged independently reviewed
[Phase 4 design](../design_review/reviews/design_review_phase4-plan_2026-09-30.md).
A schema/producer revision and clean rebuild follow implementation; there are no old-format
readers or in-place repairs. Existing Phase 3 receipts retain their date and scope; acceptance
establishes no new runtime property.

Revisit if a required consumer needs to mutate an earlier semantic row or reads its own unfinished
publication group. Do not broaden this mechanism into arbitrary multiwriter storage to avoid
settling that dependency. Analysis and retrieval embedding consumption have distinct one-shot
writers instead.

The parent [cutover findings](../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
remain open until implementation evidence closes them. The detailed plan's R0 controls establish
prefix isolation, immutable deduplication, future-reference refusal, atomic visibility and terminal
failure. Acceptance of this decision alone establishes none of those runtime properties.
