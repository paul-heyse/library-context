---
id: ADR-0128
title: Compile admitted semantic graphs without a store and realize them natively in SurrealDB
status: accepted
date: 2026-10-05
supersedes: [ADR-0086]
superseded-by: null
design: ["§1.2", "§B2", "§B3", "§B7", "§B12", "§B13", "§5", "§6", "§15"]
evidence: Proposed
---

## Context

The graph-native target and SurrealDB capability reviews of 2026-10-05 select a direct replacement
of store-bound compilation and serving. Persistent stage checkpoints and remote reference checks
couple pure computation to store lifecycle and add repeated physical work. Compilation must
produce a complete attributed graph without a database; a separate publisher realizes that graph.
The [replacement coordinator](../plans/graph-native-pivot-plan_2026-10-05.md) owns scope and current
acceptance. Existing PostgreSQL receipts do not establish the replacement's behavior.

## Options

1. **Repair the PostgreSQL implementation.** Retains useful typed semantics, but preserves
   store-shaped producer dependencies and indirect connected retrieval. This is not the selected
   design-phase target and would spend effort on an intermediate architecture.
2. **Compile an in-memory copy of the old relation store.** Removes network crossings but retains
   lifecycle/receipt machinery and full resident copies. Streaming the final result does not fix
   the intermediate working set.
3. **Store-free graph construction, then native SurrealDB realization.** Retains Rust semantic
   ownership and useful Arrow/native kernels while separating completed inputs, admission,
   persistence and querying. Temporary columnar segments and spillable bulk operations support
   larger inputs; native adjacency/functions/search support connected product requests.

## Decision

Select option 3 as a hard design-phase pivot. Rust `lctx-model` owns typed entities, attributed
assertions, participant roles, domain operations, original bytes, uncertainty and admission.
`cpg-core` becomes the sole store-free compiler, consuming completed attempt-local inputs and
returning an immutable admitted artifact. Arrow/DataFusion and native kernels remain internal
computation tools, not a second published store. Codebooks remain append-only.

Separate logical identity, captured source, semantic contract, producer implementation, complete
graph content and physical realization. Source comments/global lockfile bytes are not semantic
compatibility; actual implementation dependencies remain provenance and reuse inputs.

SurrealDB native persistence, functions, graph querying and search are selected. The realization
owner installs strict per-snapshot databases on a managed local RocksDB server, reconciles stored
content once after writers drain, seals executable definitions and publishes separately from
selection. Those runtime implementations follow the compiler stage; acceptance of this decision
does not claim they exist. Native query adoption requires no benchmark or work-accounting proof.

Retain original-source chunk fidelity, provider attribution/disagreement, coverage/outcome meanings,
catalog independence from optional analysis, programmatic synthesis, pinned analyzed inputs and
the exact embedding specification. Reject persisted compiler grants/epochs, PostgreSQL runtime,
old-format readers, ID bridges, dual writes and historical runtime archives. Replace the owning
mechanism and remove its obsolete consumers at the boundary.

## Consequences

The compiler can be tested through its actual artifact boundary without database setup. Efficient
execution follows first principles: shared completed views/topology, spillable bulk work and
coarse native requests. Practical buffer, cancellation and storage limits do not imply detailed
runtime accounting or additional proof machinery. Quantitative performance claims still require
measurements.

The compiler stage delivers artifact-only CLI operation and all admitted frontiers. Ordinary
compile-and-publish remains unavailable until its publisher exists; no silent PostgreSQL fallback
or artifact-only default is provided. Replaced product operations may be unavailable between
ownership cuts. No operator database, registration or live service is changed by this decision.

Implementation and functional acceptance remain in progress under the coordinator; findings are
not closed by ADR acceptance. Revisit a mechanism if actual input/operation semantics cannot be
represented faithfully or a concrete deployment requirement invalidates the selected local route.
