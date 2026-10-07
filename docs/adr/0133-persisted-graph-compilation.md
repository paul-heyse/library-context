---
id: ADR-0133
title: Compile through immutable native graph views and seal the same persisted result
status: accepted
date: 2026-10-07
supersedes: [ADR-0128]
superseded-by: null
design: ["§1.2", "§B2", "§B3", "§B7", "§B12", "§B13", "§5", "§6", "§15"]
evidence: Proposed
---

## Context

The catalog-speed review F01–F05 identifies repeated rich selection, cumulative prefix
reconstruction, repeated admission/self-import, eager disabled-analysis preparation and scalar
external endpoint loading. The operator selected persisted graph compilation and comparable
shared corrections throughout the codebase, approved direct sealing and exact internal
contribution/view identity, and selected dependency foundations without a cross-run executor.
The [execution plan](../plans/persisted-graph-execution-plan_2026-10-07.md) owns all corrections;
the independent target-plan review accepts this Proposed architecture. Earlier focused receipts
establish only the previous implementation, not this migration or a speedup.

## Options

1. **Indexed in-process immutable inputs.** Simpler for several individual defects, but does not
   provide the selected persistent graph/compiler-dependency substrate.
2. **Persist every typed relation alongside the complete graph.** Makes row addressing direct,
   but duplicates payload authority and synchronization. Reject the second full canonical store.
3. **Compile in the native graph and seal that database.** Reuses the managed runtime, indexed
   selection, bulk writes and existing publication lifecycle. Native persistence/index work buys
   current compiler access and durable dependency inspection; finite Rust kernels and spillable
   Arrow/DataFusion remain appropriate computation mechanisms.

## Decision

Select option 3 as a hard design-phase pivot. Rust lctx-model owns typed records, attributed
assertions, roles, source correspondence, uncertainty, domain operations and validators.
One private STRICT SurrealDB database contains pending and immutable completed contributions;
exact frozen memberships govern every read. Graph-registered records have one immutable payload
with mechanically derived canonical bytes and complete native fields. Actual non-graph products
have one generated backing family; original bytes retain their chunk owner.

Model-owned contributions, views and dependency selections distinguish completion from global
reference closure. Pending outputs are invisible; complete-empty and actual availability remain
explicit. Full typed keys and payload equality determine deduplication/conflict. Retain exact
predecessors and answer-affecting original/model/configuration/implementation/specification inputs;
absence-sensitive dependencies bind the selected universe. No event ledger or resumability grants.

Internal identities bind immutable contributions/views, not repeatedly hashed rich prefixes.
Final graph-family content and completed-state content remain deterministic separate identities;
the aggregate manifest includes both. Complete artifact/backup/restore preserves state and its
required backing across physical database names, without old-format readers or inferred state.

cpg-core depends on lctx-surrealdb, which depends on lctx-model. Native storage owns mechanical
private database/read/write/finality; core owns operation orchestration; publisher owns admission,
external visibility and sealing. Ordinary compilation seals its own fully admitted native state
without self-export/readmission/re-ingestion. Detached external import and independent actual
stored reconciliation remain genuine trust boundaries. Writers/tasks drain, indexes/definitions
are ready, writable ownership ends and a VIEWER verifies the marker before an unselected handle.
Publication is separate from selection; readers pin one complete semantic/physical realization.

Every compile frontier/profile, including artifact-only, uses this same native compiler/runtime.
Pure finite model/kernel controls remain explicit-value operations without a database. Shared
indexed access, bounded bulk codecs, original-range unions, demand preparation and bounded wire
encoding replace identified variants; no generic backend framework or new block cache.

Restate surviving ADR-0128 obligations: nominal identity/role fidelity, pinned acquisition,
provider attribution/disagreement/coverage, programmatic assertions and primary evaluation,
complete embedding values/specifications and explicit projections, append-only codebooks,
terminal streaming, immutable reader pins and fresh rebuilding from pinned inputs. Native querying
remains selected; neither adoption nor efficient design requires work-accounting proofs.

## Consequences

Native readiness is required for all compilation outputs. Short bounded write/completion
transitions preserve durable Every synchronization; no transaction spans extraction, compute,
inference or external files. Unknown write/commit acknowledgement fails the disposable attempt;
cleanup drains owned work and reports an unselected orphan if removal fails.

Persisted inputs/products/dependencies support inspection and complete transport now. Automatic
cross-run invalidation/recompilation remains deferred until a concrete incremental workflow is
requested. Reopen a mechanism if actual semantics cannot be represented faithfully or a deployment
requirement invalidates the managed local route. Quantitative speed claims require measurements.

This accepted decision establishes the target, not implementation or finding closure. PG1–PG9
remain open until their actual producer/consumer migrations, obsolete-path removal and targeted
functional checks pass. Operator stores, selection and real-library adoption are not activated by
this record. No legacy IDs, parallel compiler/store, rollback assets or historical runtime archives.
