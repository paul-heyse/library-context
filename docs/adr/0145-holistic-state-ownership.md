---
id: ADR-0145
title: Unify recoverable closure, bounded native lifecycle and charged active state
status: accepted
date: 2026-10-10
supersedes: [ADR-0143]
superseded-by: null
design: ["§1.2", "§B2", "§B3", "§B7", "§B12", "§B13", "§4", "§5", "§6", "§11", "§14.2", "§15"]
evidence: Proposed
revisit: A supported consumer cannot preserve coherent selected recovery, permanent delayed-operation fencing or charged borrower lifetime through the owned native service.
---

## Context

The operator accepted holistic RC01–RC03 on 2026-10-10. The [holistic plan](../plans/holistic-state-management-plan_2026-10-10.md) and its accepted target assessment refine the unified implementation after populated validation exposed repeated closure preparation, complete server arrays, unbounded retirement effects and active-state ownership gaps. This decision supersedes ADR-0143 and carries forward its exact identities, typed ingress, independent admission, one-service, transport/finality, provenance and finite-kernel guarantees. Whole-main logical export and retention without explicit issuance/outcome horizons are replaced. Implementation and acceptance remain open.

## Options

1. **Keep independent closure interpreters, whole-main backup and terminal history indefinitely.** Avoids migration but preserves repeated preparation and unrelated work; pruning by age or resolved state would revive delayed issuance or erase needed outcomes.
2. **Shared semantic closure, bounded native phases and charged active borrowers (selected).** Reuses existing model declarations, native guards, exact identities, transaction streams and resource pools. Requires coordinated consumer migration and explicit schema cutover, while retaining independent assurance and recovery certainty.
3. **Generic state/workflow engine, new validity graph or closed immutable segments.** May serve future consumers but adds authority and layout complexity without resolving current operation boundaries more directly. Reconsider only for a concrete unsupported workflow.

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

Logical backup selects only the exact recoverable publication closure through bounded indexed reads within one transaction-bound snapshot and reader pin. Capture manifest and definition comparison metadata in that same snapshot. Check application terminals, physical EOF and explicit transaction cancellation before accepting staged bytes. Include selected closed content and completed state; protect service/security and external coordination recovery assets separately. Ordinary restore parses supported literal data and definition metadata, never executes imported SQL/DDL/access definitions, and creates fresh staging/guards before independent admission. Imported attempts, pins and publication markers confer no authority. Concurrent same-dump imports share immutable content only. Whole-service administrative recovery is a separate maintenance operation. Preserve structured primary/finalization errors, committed effects and explicit uncertain/orphan disposition.

### Tests, resources and retirement

All native controls use the same stable service with logical attempts. Reuse unaffected immutable prerequisites, not the success of the decision under test. Malformed/corruption controls allocate fresh owned validation records and exact predicates; global DDL/crash controls require maintenance. No ordinary fixture removes a database/namespace or falls back to scratch storage. Pure finite controls remain database-independent. Arbitrary SQL/native MCP is limited to non-sensitive validation maintenance credentials excluded from main.

Native storage owns shared-content reachability, pins, protected evidence obligations, unresolved effects and bounded recoverable retirement. Reference creation and deletion participate in the corresponding guards. The storage manager observes and delegates; it never deletes RocksDB internals or evicts authoritative warm content merely by age. Preserve byte-aware charges, queues and timely scratch release across composed operations.

Dependency direction remains cpg-core -> lctx-surrealdb -> lctx-model. Core owns orchestration/admission, native storage owns mechanics/finality and publisher owns visibility. Arrow/DataFusion and compact graph projections are derived compute/transfer, not another canonical authority. No transaction spans extraction, CPU computation, inference or external files. Preserve pinned acquisition, independent providers/coverage/disagreement, programmatic assertions/evaluation and exact consumed embedding values. Rebuild obsolete owned projections from pinned inputs without compatibility readers, dual canonical writes or historical runtime archives.


### Shared closure and active ownership

The model declares recoverable families and dependency meaning; native storage and portable dump preparation supply exhaustive physical adapters. Audits capture selection once, lend it to independent actual-state checks and retain their read owner through failed preparation, cancellation and all terminal stream tails. Release the pin and session only after explicit drainage. Native readers, reconciliation and search use complete indexed windows or bounded exact batches instead of complete server arrays. Eligibility precedes quotas; global kernels still admit their complete selected graph explicitly.

Compiler preparation follows the last remaining selected consumer, including optional and cache-hit decisions. Ranked retention uses the existing service resource pool, request-scoped scratch and fully formed immutable charged borrowers. Pending construction stays request-owned through final packing. Replay runs outside the retention-map lock; eviction cannot release an active borrower's reservation. Selected policy reaches decode and final envelope admission without hidden defaults.

### Issuance, retirement and history

Native control version 4 separates pre-submission issuance era, original owner epoch, durable obligation and identified executor. A retry retains its original request handle. Permanent closed-through fencing covers both allocation and execution under a conflicting guard write. Preserve monotonic epochs and object retirement cutoffs. Keep installation coordination initially, restricted to short actual-effect transactions; narrower coordination requires equivalent delayed-operation and recovery exclusion guarantees.

A fresh retirement invocation claims an exact object incarnation, atomically nominates children while deleting bounded outgoing-hold pages, and finalizes only after an empty proof. Incoming/outgoing hold creation and immutable reactivation respect the retiring phase. The first claimed cutoff and completed pages are immutable. Same-era resume retains original authority; post-cut recovery uses a distinct named maintenance successor after reconciling or fencing its predecessor, claiming only durable remaining scope. Unknown outcomes protect that scope.

Index live effects, attempts, pins, backup holds and retirement progress. Rich history collection requires explicit drained maintenance, permanent issuance fencing, terminal outcomes and released caller/recovery/evidence references. Referenced terminal attempt rows remain sufficient provenance. Each bounded collector page has a recoverable current-era receipt and rechecks eligibility atomically. Missing closed-era detail reports compacted terminal history, never inferred uncommitted execution. No age, heartbeat or resolved flag alone authorizes disposal.

Version-3 cutover preserves immutable content, generation, ownership, outcomes and watermarks. An exclusive checkpointed upgrade owner drains legacy clients, confirms daemon termination, replaces stale owned credentials and transfers the approved maintenance executable. Legacy metadata is permanently fenced; ambiguous old retirement scope is protected or blocks migration, never refreshed against today's incarnation. Ordinary installation refuses incompatible state before effects. Completed-state format 3 is independent and unchanged by these runtime-control changes.


## Consequences

One closure meaning supports independently checked current and cold state. Sparse work avoids unrelated retained content, and high-degree retirement advances through bounded recoverable effects. Active preparation and optional retention have explicit charged owners. These changes do not make every global algorithm incremental or establish a measured speed/memory improvement.

Native schema migration, issuance/successor interleavings, selected snapshot export and composed resource lifetimes require actual qualification. Destructive collection remains disabled until its consumers and horizons are qualified. Whole-service cold recovery, BC3 profile adoption, SM8 acceptance, protected evaluation, real-library adoption and operator activation retain their separate boundaries. The [persisted coordinator §8/§9.1](../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) owns finding dispositions and actual receipts. Accepted at Proposed strength on 2026-10-10; no runtime closure follows from this record.
