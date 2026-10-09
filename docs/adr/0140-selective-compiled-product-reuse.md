---
id: ADR-0140
title: Reuse pure compiled products through fresh native ownership
status: accepted
date: 2026-10-09
supersedes: []
superseded-by: null
design: ["§B3", "§14.2", "§15.1", "§15.10"]
evidence: Proposed
revisit: A complete dependency domain or current semantic owner cannot be reconstructed without repeating the entire eligible operation.
---

## Context

ADR-0139 accepts selective reuse while preserving independent native ownership and admission.
The graph/hash review's RC02 and the graph/reuse companion require actual cross-attempt products
and viewer preparation. The [persisted coordinator](../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns implementation acceptance and source-qualified findings.

## Options

1. **Retain only attempt-local preparation.** Simple ownership, but repeats unchanged pure
   normalization and topology across attempts and closure hydration across same-pin requests.
2. **Portable derived products plus fresh owners (selected).** Complete dependency manifests
   avoid unnecessary computation while canonical storage and independent admission retain
   authority. Necessary current owner predicates validate actual cached public rows before ingress.
3. **Persist checked capabilities or adopt a general incremental query engine.** Serialized
   authority cannot retain current provenance, budget or lifecycle. Recursive machinery does not
   replace the existing finite SCC/fixpoint kernels or external effect owners.

## Decision

`lctx-model::domain::compilation_product` owns portable data contracts, framed full BLAKE3 keys,
role-associated dependencies, complete outcomes and value/provenance separation. An explicit
model/runtime owner retains non-evicting exact-equality XXH3 program interning. Existing BDD
canonicalization serves the current expression consumer; no second expression interner or global
map-hasher replacement is added.

`lctx-surrealdb::NativeProductCache` owns an explicitly configured disposable native database
and shared canonical filesystem coordination directory on the managed local host. Ordinary
connection never provisions or migrates it. Immutable chunked products become lookup-visible
only after complete canonical readback and an atomic exact acknowledgment receipt. Generation
and point-stage transaction fences prevent delayed writes after cleanup/retirement; native quota
reservations include uncertain writes. Capacity coordination does not span payload transfer.
Borrowers retain charges and leases until release. Separate fixed entry/stage lock stripes bound
coordination metadata even on misses or uncertain reservation outcomes. Stripe collisions restrict
optional publication/eviction only; full native keys still determine identity. A busy publication
stripe refuses acceleration instead of waiting for a borrow its caller may hold. Coordination
inodes survive until exclusive generation retirement/reset. An immutable native coordination-owner
receipt survives private schema rebuilding and interrupted installation. Explicit reset mutates
the old generation point before destructive DDL and refuses a foreign coordination directory.
Reset schema/header/quota writes share one transaction guarded by the persistent administrative
generation. Compatible explicit installation and retirement rotate that same token, fencing late
unknown administration. A durable pending marker blocks cache admission after an uncertain or
failed administrative operation; explicit fenced installation reconciles it before resuming.
Readonly connection cannot establish administration terminality.

Core checks complete typed canonical bodies and necessary current semantic predicates before replay.
Rejected derived data or optional allocation refusal falls back before current ingress.
Cached rows enter fresh ordinary contributions, with current inputs, provenance and semantic
owners. No cache digest creates a checked capability, and portable products carry no private
owner hints. Receiver/Event/Binding use existing admission predicates. Behavioral Local, Base,
Body and SourceCall execute afresh: validating their full public/private correspondence and
completeness would repeat their production kernels and then add lookup, comparison and replay
work. Catalog NotRequested rows remain eligible under their explicit profile contract. Provider
effects, configuration,
frontier admission and embedding inference retain their existing owners and eligibility rules.

Complete selected Entity/Callable/Signature/Overload/Class/Field/Assessment domains are freshly
discovered before rich hydration, including missing/negative outcomes and native content tokens.
Only misses hydrate rich rows. Full nominal topology includes isolates and typed identity-bearing
parallel/internal-SCC arcs. The SCC schedule is a named value-only consumer: equal topology can
reuse its pure schedule while Summary observes fresh exact sources. Other consumers remain
exact-view dependent. The dependency graph records current computation/provenance edges;
exact dependency manifests select compatible entries without claiming a general dynamic
value-cutoff scheduler.

Serving owns a Moka future cache within one immutable viewer pin. Complete exact keys, charged
values and insertion owners remain repository responsibilities. Requests wait independently;
canceled waiters do not cancel shared initialization. Close fences new owners/retries, cancels
queued admission, drains submitted work, releases cache references, then waits external leases.
Eviction cannot release a live borrower's charge or pin. Required capture/preparation admission
can replace and drop the optional Moka retention owner; logical invalidation alone does not
promise prompt payload destruction. Active initializer cache clones drain normally, and external
value borrows remain charged.

## Consequences

Reusable values are independent of physical attempt IDs; fresh ownership and necessary semantic
validation remain explicit. Separate quota/generation records avoid making unrelated capacity
changes conflict with active chunk transfers. Lost acknowledgments remain uncertain until exact
reconciliation. Optional cache state yields to required computation. No concurrency cap follows.

The [compiler plan](../plans/graph-compilation-kernels-and-hashing-plan_2026-10-09.md) and
[reuse plan](../plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md) own implementation
and qualification. Acceptance does not close findings, establish Measured speedup, change
published artifact formats, or authorize operator installation/activation.

## Amendments

**Implementation eligibility refinement, 2026-10-09 (Implemented / acceptance in progress).**
The SCC value is pure, but the present cached-schedule validator repeats forward/reverse
component traversal and canonical condensation ordering, then adds hashing, comparison and
lookup. Under the decision's beneficial-reuse eligibility rule, this consumer remains Fresh.
The schedule executes once per prepared graph and retains a private, nonserialized
materialization identity for constant-time exact runtime binding. This refines the named SCC
consumer above; no persistent SCC computation cutoff is delivered. Canonical nominal topology
identity remains a portable value contract, and Summary still observes exact current provenance.
No numeric performance claim follows from this static assessment.
