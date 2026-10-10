---
id: ADR-0143
title: Share immutable native content through exact views and durable execution ownership
status: accepted
date: 2026-10-09
supersedes: [ADR-0138, ADR-0140]
superseded-by: null
design: ["§1.2", "§B2", "§B3", "§B7", "§B12", "§B13", "§4", "§5", "§6", "§14.2", "§15"]
evidence: Proposed
revisit: A supported writer or consumer cannot preserve exact view-relative meaning, immutable definition epochs, independent cold admission or durable effect certainty under shared storage.
---

## Context

The operator accepted all seven rule impacts in the [unified persistent review](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md) on2026-10-09. The [unified plan](../plans/unified-persistent-surrealdb-plan_2026-10-09.md) and its [independent target assessment](../design_review/reviews/design_review_unified-persistent-surrealdb-plan_2026-10-09.md) define the replacement. Existing private databases supplied isolation, sealing and cleanup; retaining a daemon alone cannot remove repeated content replay or preserve snapshot meaning when several attempts share it. This record consolidates ADR-0138 and ADR-0140. Their surviving typed-ingress, exact dependency, independent admission, finite-kernel, bounded-transfer and terminal-effect guarantees remain in force. Private-database sealing, mandatory canonical replay and ordinary administrative dump execution are replaced.

## Options

1. **Keep private databases on a persistent daemon.** The smallest lifecycle change, but repeats schemas and canonical rows, preserves per-attempt database abandonment and prevents shared admitted-content attachment.
2. **Shared immutable payloads, nominal anchors and exact manifests (selected).** Existing typed identities, deterministic memberships and native indexes supply the foundation. Readers resolve both directions through exact membership, while short object guards protect effects, pins and retirement. This requires coordinated writer/reader/publication/test migration but removes repeated canonical copying and service startup.
3. **Closed immutable segments.** Can amortize near-identical memberships, but complicates sparse views and cross-contribution sharing before a consumer establishes that need.
4. **Generic incremental/workflow engine or changefeed-owned validity.** Adds another runtime authority; it does not replace domain-specific dependencies, provenance, admission or uncertainty. Existing declarations and finite SCC/BDD/fixpoint kernels remain sufficient.

## Decision

### One installed service and explicit ownership

Use one authenticated library-context SurrealDB3.3.0 RocksDB user service with stable main and validation databases. Production content, manifests, attempts, guards and pins co-locate in main; synthetic controls and untrusted imports use validation with the same supported schema. Ordinary clients attach compatible installed state and never provision databases, start fallback services or perform schema migration. Installation, incompatible cutover, restart, recovery and privileged diagnostics require explicit maintenance admission and actual borrower/effect drainage. Worktree and command lifetimes release their own attachments, not service storage. Credentials stay private; installation identity and native-extension provenance remain separate.

Retain patched progressive gRPC initially. WebSocket/HTTP alternatives must preserve per-request rows, retraction/error/finality, cancellation, reconnect, export/import and backup contracts before substitution. Share only compatible immutable session context; never mutate a shared client's database selection. Run compiler and test workers with normal available parallelism. Use bounded transfer/backpressure and prompt representation release rather than job/thread or fixed service-memory caps.

### Canonical content and exact views

Rust lctx-model owns typed nominal records, attributed assertions, roles, original correspondence, coverage, uncertainty, identities and invariants. Fixed SCHEMAFULL envelopes plus declaration-derived synchronous adapters validate and lower bounded typed windows before writes. Flexible object bodies preserve missing/NULL, bytes/text, local sums and supplied scopes. Independent cold admission reconstructs complete expected envelopes, bodies, scopes, graph/state and originals. Privileged raw SQL is not a semantic writer or admitted validity.

Canonical payload addresses include explicit model/codec identity, relation, nominal key and full content digest; digest hits compare full canonical values. Nominal anchors contain identity without implying selected existence. Native role edges derive once from payloads and preserve field, direction, position and multiplicity. Exact contribution membership selects payloads. Different views may choose different revisions of one nominal key; unequal payloads in one view conflict. Build compact completed-view resolution by bounded ordered merge, never owner-by-key Cartesian enumeration. Forward and reverse access use the same exact resolver; ENFORCED anchor existence alone is insufficient. Originals retain bounded content/range verification. Append-only codebooks and semantic identity remain independent of physical schema and runtime generation.

### Completion, reuse and recovery

Pending membership is not completed authority. Model-owned declarations bind complete exact source/model/implementation/settings, captured supplier/specification, membership/absence/coverage and provenance premises. Retain complete-empty and optional outcomes. Current attribution never silently becomes an old producer's provenance. Local completion closes and drains its producing descendants; final admission freezes its exact read set and effects without stopping unrelated service writers.

Durable attempts, operation identities, fences and corresponding object guards protect bounded writes, completion, checked attachment, pins and retirement. Snapshot isolation and predicate scans do not serialize arbitrary invariants. Unknown acknowledgments reconcile the original operation before retry; acknowledged committed identity survives later cleanup or cancellation. Client death or heartbeat expiry alone does not prove remote terminality. Recovery retains compatible completed prerequisites and re-executes missing work, without resuming arbitrary pending coroutines or introducing a durable task executor.

Checked attachment may reuse retained admitted content without canonical row replay. It establishes current bindings and pins under exact compatibility/provenance/retention checks; serialized data never grants runtime authority. Untrusted portable products retain typed ingress and independent cold admission. Avoid a second serialized canonical cache without a distinct consumer. Existing exact-equality XXH3 program interning, value/provenance distinctions and finite graph/BDD kernels remain. Configuration, external effects, frontier admission and inference retain their actual owners. Serving preparation shares one stable initialization under its exact pin, with independent waiter cancellation and charged borrowers through actual terminality.

### Publication, search and recovery assets

Publisher accepts only privately constructed admitted artifact/restored authorities. Validate and reconcile their exact views, originals, outcomes, consumed vector values and immutable named definition epoch, then expose an unselected manifest through a short guarded transition. Ordinary compilation performs no self-export/import or canonical reload. Selection is separate. Raw database VIEWER credentials are internal, not snapshot ACLs; ordinary Rust reads, resources, evidence and continuations acquire fresh runtime pins on the exact publication. Compare actual definitions and retain pinned epochs; incompatible schema/service changes use maintenance.

Search eligibility precedes quotas and limits. An unqualified ANN route cannot filter after candidate limits and call itself exact-view search. Use an explicitly identified exact eligible-vector policy until native approximate behavior preserves its declared contract. Model-owned ranking fidelity remains explicit.

Logical backup streams main under one native read snapshot and retention hold with checked application and physical terminals. Include exact closed content and completed state; protect service/security and external coordination recovery assets separately. Ordinary restore parses supported literal data and definition metadata, never executes imported SQL/DDL/access definitions, and creates fresh staging/guards before independent admission. Imported attempts, pins and publication markers confer no authority. Concurrent same-dump imports share immutable content only. Whole-service administrative recovery is a separate maintenance operation. Preserve structured primary/finalization errors, committed effects and explicit uncertain/orphan disposition.

### Tests, resources and retirement

All native controls use the same stable service with logical attempts. Reuse unaffected immutable prerequisites, not the success of the decision under test. Malformed/corruption controls allocate fresh owned validation records and exact predicates; global DDL/crash controls require maintenance. No ordinary fixture removes a database/namespace or falls back to scratch storage. Pure finite controls remain database-independent. Arbitrary SQL/native MCP is limited to non-sensitive validation maintenance credentials excluded from main.

Native storage owns shared-content reachability, pins, protected evidence obligations, unresolved effects and bounded recoverable retirement. Reference creation and deletion participate in the corresponding guards. The storage manager observes and delegates; it never deletes RocksDB internals or evicts authoritative warm content merely by age. Preserve byte-aware charges, queues and timely scratch release across composed operations.

Dependency direction remains cpg-core -> lctx-surrealdb -> lctx-model. Core owns orchestration/admission, native storage owns mechanics/finality and publisher owns visibility. Arrow/DataFusion and compact graph projections are derived compute/transfer, not another canonical authority. No transaction spans extraction, CPU computation, inference or external files. Preserve pinned acquisition, independent providers/coverage/disagreement, programmatic assertions/evaluation and exact consumed embedding values. Rebuild obsolete owned projections from pinned inputs without compatibility readers, dual canonical writes or historical runtime archives.

## Consequences

The selected route removes repeated canonical insertion and schema/service startup while making view, runtime and retention ownership explicit. Compact mappings and retained revisions still consume space; persistence does not bound client heaps or make whole-universe algorithms incremental. Native indexes, scoped search, concurrency guards and full recovery closure require actual qualification. Quantitative speed or capacity claims require measurement.

Accepted at Proposed strength on2026-10-09; acceptance is not implementation closure. UP0–UP9 schedule the migration. The [persisted coordinator §8/§9.1](../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) owns findings and source-specific receipts, including surviving PC/PJ/GK/GR/NE/CU/BC acceptance. BC3 candidate-profile qualification, protected evaluation, real-library adoption and operator activation remain separate. Earlier receipts retain their original source and scope. The independent target review's TF01 data-only restore refinement is a required correctness boundary, not optional hardening.
