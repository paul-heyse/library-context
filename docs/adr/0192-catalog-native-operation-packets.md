---
id: ADR-0192
title: Bound native compiler packets and publish complete binding sets under existing protection
status: accepted
date: 2026-10-18
supersedes: []
superseded-by: null
design: ["§6.1", "§6.2", "§15.11"]
evidence: Implemented
revisit: A required native producer cannot preserve exact guards, original evidence, or terminal recovery inside bounded operation pages.
---

## Context

The execution-speed review F02/F03/F05/F06 and I02 identifies separate physical-category
crossings, repeated scalar native lookups, serialized independent materialization and per-slot
binding effects. ADR-0191 removes unnecessary private persistence but preserves genuine native
output and receiving authority. SN01–SN03 now implement those actual boundary improvements;
the [coordinator §6](../plans/catalog-execution-speed-plan_2026-10-18.md#6-sole-disposition-and-current-checkpoint)
owns current verification and disposition.

## Options

1. Keep separate endpoint, payload, alias, block and binding calls. Existing protection works,
   but repeated preparation and network/transaction boundaries follow physical categories rather
   than the actual operation. This loses on FP-07/A4 and the efficient-architecture principles.
2. Put a whole artifact in one transaction, or replace existing native recovery with a new bulk
   framework. Both increase resource/failure coupling; neither is necessary for bounded progress.
3. Extend existing immutable windows and operation roots (selected). Compatible rows and their
   exact holds share pages, native set observations and restricted exact indexes remove repeated
   search, and completed binding visibility follows the actual set owner. Existing pinned SDK,
   SurrealQL, resource, recovery and task owners remain the integration boundary.

## Decision

Compiler typed output lowers compatible endpoints, payloads, aliases, blocks, locators and holds
into bounded pages of its existing contribution operation. Original ingress retains its necessary
protected-endpoint ordering and byte stream. Complete conservative wire size and actual working
memory admission are separate; final serialized native preflight remains authoritative. A page
resource refusal may split only an unissued/drained read slice, never replay an accepted write.

Native preparation reads bounded exact ID sets. A packet-local object index applies only to
known physical tables with generated lowercase 64-hex string keys. Reversing entries before
last-wins object construction preserves original first-match semantics; full native record equality
still checks each result. Compound, numeric, other-table and nonconforming keys retain ordered
native equality scans. No generic Rust/string hashing rule is substituted for native equality.

Pinned3.3 refuses array-valued targets for `SELECT … FOR UPDATE`. Changed-incidence guards
already require real revision writes, so one conditional `UPDATE … RETURN BEFORE` both advances
the exact keys and observes their actual prior state. Complete returned count and native ID-set
equality are required; missing, retiring or exhausted guards throw within the same transaction,
rolling back every peer. The write itself retains commit-time conflict protection. Read-only
decision inputs keep their necessary scalar `FOR UPDATE` registration. This uses existing native
mutation rather than weakening protection or adding an unsupported locked-array read.

Compiler view creation atomically earns its attempt hold. Contribution creation and attempt
retention share a bounded registration operation; physical input-view targets are deduplicated
without changing ordered semantic roles, then retained through bounded pages before Producing
visibility. Registration finishes its recovery-metadata operation before later payload production
earns its own caller-budget operation. Actual qualification grants retain their required pool;
independent recovery metadata and payload budgets remain supported.
No collector may observe a successful initially unheld declaration.

Creation, first placement, completed-parent, hold incidence, collector/retirement, guard and
predicate hazards retain their exact protection. The fixed64 issuance stripes and64 attempt
participant slots survive. Optimistic FOR UPDATE registration is not a blocking lock or protection
for an unregistered predicate. Retry observations identify operation root/page/ordinal and query
shape without payloads. The existing request allowance begins at the first acknowledged conflict;
issued queries always retain physical terminal observation/reconciliation before retry or return.

The compiler derives required materialization from current/frozen bindings and recursively exact
producer dependencies, preserving each retained producer's complete outputs and declared empty
views. Ready independent producers use accepted task/native-call ownership and actual allocation
grants instead of a fixed call-count semaphore; all accepted work settles before failure returns.
A complete mutable binding set validates before effects, owns one
operation root and bounded guarded pages, and publishes one inventory revision. Same-attempt
readers wait for set completion; frozen/admitted consumers require terminal operation roots.
Failed partial sets poison their attempt and preserve actual recovery ownership, never successful
completed visibility. Single-binding callers share the same visibility exclusion.

Native schema12 installs the changed functions. Completed-state5, contribution identity3,
logical-source identity2 and public transport format stay unchanged. Regenerate only the explicitly
owned campaign in this selected scope; broad default regeneration remains SN04. ADR-0190's semantic
publication/receiving obligations and ADR-0191's private semantic authority remain in force.

## Consequences

Physical page grouping follows real dependencies while preserving independently admitted incoming
content, exact originals and unselected publication semantics. Ordinary compiler roots may retain
every producer when all their outputs are genuine current bindings; this is not a license to prune
semantic intermediates by name. The implementation introduces no new persistent certificate,
general backend framework, thread cap, raw-export test prerequisite or compatibility reader.
Actual successful private qualification can require exact singleton views beyond published
bindings and producer-declared inputs. These explicit selection roots travel through required
materialization and registered-view validation into the existing physical completion traversal
before freeze. They are neither new artifact bindings nor retained cache products; completed
native capabilities are still independently earned. Post-freeze known-view lookup cannot widen
the exact completed inventory, and incoming owners receive no private roots.
Focused actual native controls and integrated source review must establish the changed protection
and publication behavior. No measured speedup, full FastMCP/SP95 acceptance, operator adoption,
embedding inference or submitted MCP runtime is established by accepting this decision.
