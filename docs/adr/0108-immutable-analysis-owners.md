---
id: ADR-0108
title: Instantiate shared analysis contracts at immutable publication owners
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: ['15.1', '15.9', '15.11']
evidence: Proposed
---

## Context

Phase 4 requires stored reads between local behavior, execution, summaries, catalog evidence,
selection and synthesis. A single physical invocation/support/coverage family cannot accept rows
from all those producers: ordinary relations have one frozen writer, while a final writer would
prevent earlier consumers from obtaining validated inputs. A global reference sum spanning future
producers also introduces future dependencies through generated references and invariant inputs.

The [Phase 4 plan](../plans/semantic-model-phase4-detailed-plan_2026-09-30.md) owns execution and
qualification. R0 has focused publication controls; the initial isolated R1 implementation exposes
this contradiction. This decision refines ADR-0105/0106 without changing their immutable-output,
bounded vocabulary, qualification or proof policies.

## Options

1. **One final group with pure handoffs.** Feasible for a small bounded pipeline, but retains
   intermediate outputs across the full behavioral/catalog graph and removes required stored-read
   and validation boundaries. It loses the streaming property retained by ADR-0105.
2. **Generic appendable result tables.** Reuses vocabulary mechanics but changes ordinary-output
   immutability and makes proof/coverage membership an evolving lifecycle. Reject.
3. **Nominal publication-owner instances of shared contracts.** Concrete one-writer relations have
   narrow predecessor references; common operations preserve one semantic policy. Choose this.
4. **Independent copied contracts or a global tagged-ID registry.** The former duplicates policy;
   the latter introduces another writable authority and future dependencies. Reject both.

## Decision

`lctx-model` declares a finite set of immutable producing owners at actual stored-read boundaries.
Late invocation, input, outcome, coverage, proposition, derivation, support and obligation families
are nominal instances of shared contracts. Mechanical declarations/adapters are generated; shared
qualification, conservative coverage, equality, discharge and finding policy are ordinary common
operations. This does not introduce generic multiwriter storage or extend the vocabulary whitelist.

Each owner references native evidence, named completed predecessors and its own finite proof
occurrences. Its reference sums and invariant inputs exclude future owners. The generated global
derivation index and DAG validator remain authoritative across these nominal instances. Later
discharge or precision records reference earlier immutable subjects; they never extend a completed
member set. A final read-only union may provide a common consumer interface, not write authority.

Preflight-known definitions, parameters, authored models/policies and embedding specifications
have early one-shot writers. Completed source receipts and result-dependent invocations do not.
Finding/Member/Support have S0 as their sole writer; earlier producers emit qualified conclusions
for its common emitter. Add a distinct earlier finding owner only if a concrete stored consumer
requires one. Catalog core remains independent of optional behavioral and analytic success.

Split vocabulary closes only where a consumer needs its predecessor's new vocabulary. Ordinary
results needing no new vocabulary complete against the existing prefix. Close order is finite and
model-owned. No stage reads its unfinished group; genuinely bounded pure handoffs remain permitted.

## Consequences

New analyses within an owner reuse its contracts and operations. A new stored dependency boundary
adds a nominal owner, predecessor set and frontier declaration, without changing earlier producers'
semantics. This adds mechanical schema instances but avoids a second proof policy or an unbounded
final in-memory assembly. Macro-generated concrete records fit the existing non-generic declaration
system; arbitrary generic persistence support is not required.

The decision is an **Accepted target / Proposed implementation, 2026-09-30**. R1 migration,
producer scheduling, invalid-member controls and the scheduled foundation review remain pending;
plan §13.2 owns receipts. Revisit if a required producer cannot express its dependencies without a
future-owner edge, or bounded storage-backed execution cannot preserve this finite ownership graph.
