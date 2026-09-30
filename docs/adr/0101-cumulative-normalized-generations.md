---
id: ADR-0101
title: Build normalized frontiers in self-contained cumulative generations
status: proposed
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§15.1, §15.11]
evidence: Proposed
---

## Context

P0–P2 publish immutable typed facts. Phase 3 needs those facts plus intermediate normalized
relations while preserving bounded memory, source identity and generation retirement. Current
stage inputs retain batches until their last reader; store receipts are finalized at seal;
DataFusion registration can accept an arbitrary provider with a matching schema. Facts-specific
store checks do not yet express a cumulative normalized frontier. These are interface-inspected
limitations, not failures of the qualified facts-only path.

[DESIGN §15.1 and §15.11](../design/sections/semantic-model.md) own the boundary.
Provider-session F04/F06/F07 and store-lifecycle F08 remain open under the
[cutover plan §8](../plans/semantic-model-cutover-plan_2026-09-29.md).

## Options

1. **Retain all inputs as in-memory handoffs.** The simplest API extension, but it extends the
   lifetime of the facts dataset and competes with normalization's joins and indexes.
2. **Link a normalized generation to a published facts generation.** Reuses facts without copying,
   but adds multi-generation closure, leases, retention and reference validation to every reader.
3. **Publish facts and copy them into a fresh normalized generation.** Self-contained output, but
   adds an intermediate publication and import protocol without a current incremental consumer.
4. **Build a fresh cumulative generation and read frozen completed stages.** Reuses the current
   single-generation lifecycle and PostgreSQL materialization. It requires a private stage-read
   protocol and more PostgreSQL reads, but avoids whole-facts retention and linked retirement.

## Decision

**Proposed:** choose option 4. `compile --through normalized` captures input once and runs facts
and normalization in one attempt, producing a self-contained L0+L1 generation. It does not publish
or select intermediate facts. Facts compilation remains supported; future serving admission is
independent of the operator's generation-selection pointer.

The model owns a `FrontierDescriptor` for relation closure, coverage, checkpoints, validators and
selectability. Store DDL and lifecycle lower the descriptor. A private validated frontier admission
binds model, schedule, policy, content and coverage; an arbitrary string cannot authorize publication.

Completion freezes every stage output in one transaction: after closing input readers and COPY
streams, lock the generation and output tables, revoke importer writes, persist content receipts
and the stage outcome, and grant importer reads. A confirmed commit alone produces completed
handles. Validate an immutable facts checkpoint before normalization. Final publication still
requires cumulative validation and all planned outputs.

An attempt read is a private capability bound to the live attempt, completed stage receipts and
declared relation set. It uses read-only importer connections; published-reader authorization
remains separate. Source-bound table handles replace arbitrary provider registration. Leases
protect streams through drain, and stages close their readers before exclusive transitions.
The query wrapper admits the physical plan's complete scan demand before execution. One attempt
runtime accounts for capture, facts, reads, normalization and validation; importer provider
capacity is explicitly reserved alongside SQLx capacity under a non-superuser role.

The exact protocol, refusal semantics and implementation packages are in the
[Phase 3 plan §§6–8](../plans/semantic-model-phase3-detailed-plan_2026-09-30.md).

## Consequences

Retirement and reference closure remain local to one generation. A relation's availability and
origin travel with its read capability. Production implementation still needs real PostgreSQL
race/permission controls, a pinned provider-fork change and measured memory/connection envelopes.
No throughput benefit is claimed; PostgreSQL work increases relative to memory-only handoffs.

P3 deliberately has no reuse of an external facts generation. Reconsider a validated copy/import
stage or this decision when a measured rebuild cost and a named incremental consumer justify it.
Acceptance of this proposal would authorize the design, not close its implementation findings.
