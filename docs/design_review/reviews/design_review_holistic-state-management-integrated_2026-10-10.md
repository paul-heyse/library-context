# Holistic state ownership: assembled implementation review

**Design review · tier design · purpose target · 2026-10-10**

**Decision: Revise.** The implementation makes substantial improvements to audit finality, bounded native retirement, selected logical backup, compiler preparation lifetimes and ranked-result ownership. Those mechanisms support the intended API and evidence product and should be preserved.

The assembled architecture still has three material gaps. Sparse published reads enumerate the complete selected payload universe. Recovery family declarations do not govern all export, import and restore dispatch. Newly retained native selections and their ordering scratch use a separate fixed resource pool, outside the serving operation’s configured capacity. These defects concern execution fit and semantic authority; successful small fixtures would not settle them.

This review assesses the original identified dirty source. Corrections initiated after these findings require an explicit follow-up assessment. Review acceptance, functional verification and installed-service qualification remain separate.

## 1. Scope, baseline and evidence

| Field | Assessment |
|---|---|
| Functional outcome | A version-pinned API/evidence catalog with attributable facts, explicit uncertainty, exact publication identity, recoverable completed content and pinned evidence journeys. |
| Standard | Core principles/template 3.3; efficient-architecture heuristics 1.0; code-intelligence profile 1.5; repository binding. |
| Reviewer | Independent delegated design reviewer; 2026-10-10. The reviewer did not implement the examined changes. |
| Baseline | HEAD `125e74f5775e1354ce900cc8d71b190e311dac27`, plus the production changes inventoried by `git status --short` at review entry. This is the frozen source associated with coordinator candidate `155544` and all-test compilation `155539`. |
| Scope | HS0–HS11 and their interactions with surviving UP/NE/PC/GK/GR obligations: model recovery closure; compiler preparation; native selection, reconciliation and control; publisher audit/backup/restore; serving scope, search and ranked results; host schema-upgrade ownership. |
| Workload premise | One operator with concurrent commands/worktrees, retained releases and a long-lived native service. Sparse evidence requests coexist with complete integrity audits, global projections, large publications and high-degree ownership. |
| Method | Read-only source, type and architectural-owner inspection. No builds, tests, database queries, maintenance operations or file modifications. |
| Evidence strength | **Implemented/source-inspected, 2026-10-10.** Recommendations are **Proposed**. No measured speed, memory or capacity improvement is claimed. |
| Limits | This is not a fresh audit of every provider’s fidelity, every finite kernel, evaluator integrity or all native interleavings. Final-source product/native qualification remains coordinator-owned and open. |

Authorities consulted include the holistic plan and source review, ADR-0145, semantic §15, storage §6, serving §11 and the persisted coordinator. Prior assessments and memory supplied navigation context; the findings below derive from current source.

## 2. Responsibilities and governing meaning

The dependency direction remains appropriate: model declarations govern meaning; native storage realizes persistence and terminal mechanics; core coordinates compilation/admission; publisher exposes admitted content; serving consumes pinned publications.

| Owner | Governing responsibility | Examined realization |
|---|---|---|
| `lctx-model` | Exact completed dependencies, semantic identities, recovery requirements and finite-domain meaning | `domain/recovery_closure.rs`, completed descriptors and model-owned records |
| `cpg-core` | Selected execution, independent admission and preparation lifetime | `compilation.rs`, `compilation/preparation.rs`, `analysis_graphs.rs` |
| `lctx-surrealdb` | Physical selection, immutable rows, guards, issuance, pins, retirement and transport finality | `selection.rs`, `reader.rs`, `compiler.rs`, `control/*.rs` |
| `lctx-publisher` | Publication inspection, selected recovery assets and independent data-only restore | `owned_read.rs`, `inspection.rs`, `selected_backup.rs`, `restore.rs` |
| `lctx-serving` | Scope, ranking policy, request resources and pinned result delivery | `scope.rs`, `search.rs`, `search/state.rs`, `ranked_results.rs`, `service.rs` |
| Host service owner | Explicit disruptive cutover, credentials, executable transfer and borrower drainage | `scripts/surrealdb_service.py`, native upgrade/migration owners |

The governing model now distinguishes several consequential states directly: completed versus admitted content; exact views versus nominal existence; issuance era versus execution epoch; retirement invocation versus object incarnation; original operation versus maintenance successor; retained entry versus active borrower.

The remaining authority gap is narrower. `RecoveryTraversal` governs contribution/view dependencies, but the declared recovery family model does not yet govern the complete family treatment in every adapter. F02 describes this boundary.

### Fact and fidelity

| Family/result | Fidelity and coverage | Identity and consumer boundary |
|---|---|---|
| Provider-attributed facts | Existing extracted/resolved distinctions and explicit coverage remain inputs to this work; provider semantics were not comprehensively re-audited | Exact producer/configuration/source premises remain attached to completed content |
| Completed contributions/views | Exact declared dependencies, including empty views and sibling contribution outputs | Logical identity remains distinct from physical attempt ownership |
| Roles, aliases and originals | Actual stored claims must survive selection for independent comparison | Physical edges/backing rows cannot independently redefine semantic closure |
| Search/ranked results | Explicit heuristic ranking over eligible occurrences | Snapshot, channel and cursor binding preserve the meaning of replay |
| Runtime control history | Operational authority and outcomes, not imported semantic truth | Era/epoch/incarnation and named references govern execution and disposal |

## 3. Contracts and failure boundaries

The inspected contracts generally preserve useful separations.

**Audit.** `inspection::audit_owned` constructs the publication compiler owner before fallible capture, retains it in `owner`, and drains it before pin closure and session invalidation. `owned_read::run_observed` separates caller delivery from operation completion and observes task failure after delivery loss. This addresses the former seam where an audit could lose ownership of a trailing compiler query.

**Selected backup.** `backup_owned` retains the publication pin and one exporter transaction through selected export, explicit cancellation and session invalidation. `selected_backup::export` reads publication and definition metadata through that same transaction. File publication follows finalization. This is a credible coherent-snapshot architecture; successful runtime qualification of its composed native behavior remains necessary.

**Restore.** `PreparedClosure` establishes required selected payload presence from the dump before creating native attempt authority. Restore retains data-only parsing and independent admission. Sharing dependency traversal does not replace actual-state or cold semantic checks.

**Retirement/history.** Retirement claims an incarnation, nominates children and removes exact holds in bounded pages, then finalizes only after an empty proof. Writers and hold creation reject the retiring phase. Issuance-era fencing and distinct maintenance successors prevent an old request from acquiring fresh authority after an era cut. Named outcome references and provenance checks constrain rich-history disposal.

**Ranked delivery.** Pending entries remain request-owned until packing/template completion. Immutable `Arc<Entry>` borrowers retain the service reservation beyond map eviction. Replay uses the selected limits, outside the map lock.

These source mechanisms are architectural evidence, not newly executed acceptance results.

## 4. Composition and physical execution

The implementation improves several complete operations:

- Audit capture is shared across verification and completed-state identity, while independent checks remain separate.
- Selected backup avoids whole-main content export and uses one transaction-bound snapshot.
- Retirement separates claim, outgoing-edge advancement and finalization.
- Compiler graph/binding ownership follows remaining selected consumers, including optional analytics and cache hits.
- Search frontiers and ranked entries carry reservations with their retained values.

The native-reader composition remains problematic. `SelectedPayloads` supplies useful complete enumeration for integrity and recovery operations, but `NativeReader` also routes sparse typed reads through it. Each sparse request then walks every selected pointer of the physical family and performs payload batches before applying its predicate. Caching the pointer inventory does not eliminate that repeated payload work.

A second composition defect is resource ownership. The selected preparation and its ordinary row sorter use `portable_ordering_budget()`, a fresh fixed pool. Their lifetime and allocations therefore do not participate in serving’s existing shared/request budget.

### Analysis and projection contracts

| Operation | Universe/method | Completeness and limits | Output/evidence linkage |
|---|---|---|---|
| Compiler graph preparation | Declared selected projections, complete topology and retained SCC preparation | Global preparation is legitimate; selected projections release after their remaining consumers finish | Canonical identities and stage permits remain required |
| Search | Eligible occurrences, finite document/target frontiers and explicit ranking policy | Eligibility precedes ranking quotas; frontiers and scratch are charged | Candidates retain occurrence/channel identity and publication binding |
| Integrity/recovery enumeration | Exact completed closure and actual backing/derived rows | Complete enumeration is necessary; bounded windows and spill are appropriate | Independent reconciliation and cold admission remain consumers |
| Sparse typed retrieval | Requested nominal keys or declared scopes | Complete payload enumeration is unnecessary; F01 | Returned records still require exact-view membership and conflict checks |

## 5. Change, growth and failure scenarios

| Scenario | Expected route | Observed assessment |
|---|---|---|
| Add a required recoverable family — domain extension | One declared treatment, exhaustive native/dump adapters, independent checks | Family-name mapping is exhaustive, but export/import/filter dispatch remains independently maintained: F02 |
| Add another library release — instance | New immutable content/views; old references retain their meaning | Exact handles, completed dependencies and executable epochs provide a credible route |
| Read one known key as unrelated selected content grows — workload growth | Indexed candidates, exact membership resolution, bounded hydration | General typed-reader path repeats complete selected-family payload batches: F01 |
| Serve concurrent sparse requests under configured capacity — concurrency/resource growth | Shared retained preparation plus request-owned scratch | Selected preparation/sorting uses an independent fixed pool: F03 |
| Abort audit after early failure with delayed native tail — failure | Retain read owner/pin through terminal join and preserve secondary failure | Explicit owner/finalizer structure supports the contract; revealing runtime controls remain acceptance work |
| Cut an era during interrupted retirement — recovery | Permanently fence predecessor, claim only durable remaining scope under a distinct successor | Incarnation/cutoff preservation and predecessor-chain recovery are explicit |
| Evict ranking during active continuation — lifetime | Remove map retention while active borrower retains charge | Immutable entry and reservation remain together |
| Replace progressive gRPC — mechanism substitution | Native transport owner preserves rows, errors, cancellation, terminal certainty and selected transaction semantics | The contract identifies the required substitution boundary; no alternative transport is qualified here |

A mechanism substitution is credible at the native transport boundary, but would be consequential. Replacing gRPC with a buffered client while preserving only successful result values would not preserve the reviewed contract. No new provider framework is warranted.

## 6. Findings

Current execution disposition belongs exclusively in [persisted coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition). The source IDs below remain stable; coordinator aliases do not replace them.

<a id="F01"></a>

### F01 — Sparse published reads hydrate the complete selected payload universe

**High · FP-07, DP-08, DP-10, DP-14 · A4 violated**

`reader.rs:131–149` builds or shares `SelectedPayloads` over every scope view. `record_stream_prepared` at `reader.rs:246–258` routes typed predicates through `selected_payload_rows`. That method invokes `selection.rows`.

In `selection.rs:193–219`, `rows` walks the complete pointer inventory, collects every pointer belonging to the physical table, and issues:

```sql
SELECT * FROM $nodes WHERE (<predicate>)
```

It finishes all such batches and global ordering before returning rows. `records(RecordSelection::Keys)` therefore performs this work for a single key. `search/state.rs::records` repeats the same route for finite Unit/Origin requests.

**Consequence.** A repeated one-key request grows with unrelated selected payload count and crosses the native boundary for every payload window. Array removal bounds individual transfers but does not establish execution fit.

**Correction — Proposed.** Preserve native key/scope/connected candidate access through existing indexes, then intersect candidates with exact membership and alias resolution before bounded hydration. Reserve complete enumeration for integrity/global operations. Retain selected-key conflict detection, view-relative absence and checked terminals; do not treat an existing global anchor as selected existence.

**Closure evidence.** Source trace shows sparse demand reaching selective candidate access without complete payload enumeration. A revealing control should add unrelated selected content while preserving the same sparse demand and exercise missing keys, aliases and conflicting selected revisions. Actual planner behavior remains a native qualification obligation.

<a id="F02"></a>

### F02 — Recovery family declarations do not govern all adapter treatment

**High · FP-02, FP-04, DP-01, DP-06, DP-08, DP-16 · A1/A2 violated; G1 fail**

`recovery_closure.rs:10–38` declares recovery families. `compiler.rs:5210–5226` exhaustively maps them to physical names and derives completed-state table order.

However:

- `selected_backup::export` independently enumerates canonical/backing tables and separately handles roles, search claims, aliases and originals.
- `backup_import.rs::DATA_TABLES` and `DERIVED_TABLES` remain independent allowlists.
- `restore::PreparedClosure::prepare` constructs family treatment separately.
- `restore.rs:666` uses a string match and fallback for selected-row retention.

**Consequence.** Adding a recovery family and satisfying the exhaustive name mapping can still leave export or import silently incomplete. The shared traversal improves dependency authority, but does not yet make the complete recovery declaration govern its consumers. This retains part of the original HS-F01 structural cause.

**Correction — Proposed.** Make native table classification and adapter dispatch derive from an exhaustive semantic family/treatment contract. Keep native names/selectors in the native adapter and typed dump operations in the dump adapter. Explicitly distinguish required recovery content from actual derived integrity claims. Preserve unexpected selected claims for independent comparison.

**Closure evidence.** Every current family has an explicit treatment in all relevant adapters, and an extension cannot merely update a name map while omitting a consumer. A source-traced extension can settle architectural locality; missing/extra-role, alias, original and cold-admission controls retain their independent purpose.

<a id="F03"></a>

### F03 — Retained selected preparation bypasses the composed serving budget

**High · FP-03, FP-07, DP-19, DP-20 · A4 violated; configured resource contract incomplete**

`SelectedPayloads::prepare` creates a fresh `portable_ordering_budget()` and stores it in the selection. Pointer cursors, membership probes and `rows` ordering use that pool. `reader.rs:131–149` retains the prepared selection at reader-scope lifetime, but receives no serving shared/request budget.

`ordered_rows.rs:34–44` constructs the fixed portable pool independently of the existing service resource owner.

**Consequence.** Concurrent requests can admit selected preparation and sorting outside the configured service/request capacity. Charging search frontiers or ranked entries does not account for this underlying retained state and scratch.

**Correction — Proposed.** Bind immutable reader preparation to its real shared owner and per-request cursor/query/sort scratch to the existing request budget. Reuse current reservation mechanisms. Avoid charging each borrower for the same retained bytes, and preserve charge until the actual final borrower releases it.

**Closure evidence.** Source inspection identifies the owner and reservation for each retained representation and request scratch allocation. Revealing pressure/cancellation/close controls show refusal before construction and reservation survival through active borrowing and final drainage.

F01 and F03 share the selection boundary but require different closure evidence. Selective access alone does not compose resources; charging a complete scan does not make it suitable for sparse requests.

## 7. Foundations and gates

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | **violated** | F02 leaves recoverable-family treatment independently editable across adapters |
| A2 Explicit governing domain meaning | **violated** | Dependency traversal governs behavior, but complete family treatment still requires independent interpretations |
| A3 Extend through composition | **satisfied within examined operations** | Audit, snapshot export, retirement successors and ranked borrowing compose existing owners without a universal state engine |
| A4 Fit execution to workload | **violated** | F01 introduces repeated whole-selected-payload work for sparse requests; F03 bypasses composed capacity |

FP-01, FP-03, FP-05 and FP-06 are supported by the inspected responsibility/lifecycle separations, subject to F03’s resource composition limit. FP-02/FP-04 are violated at F02’s extension boundary. FP-07 is violated by F01/F03.

| Gate | Verdict | Evidence or limit |
|---|---|---|
| G1 Authority | **fail** | F02: independently maintained recovery-family treatment |
| G2 Semantic fidelity | **pass, examined state routes** | Exact descriptors, distinct runtime authority and selected cursor policy remain explicit |
| G3 Validity | **pass, source scope** | Typed ingress, selected inventory checks and independent cold admission remain identifiable |
| G4 Hidden behavior | **pass, source scope** | Ordinary reads/imported data do not acquire maintenance or executable authority |
| G5 Consistency/recovery | **unresolved for assembled implementation** | Ownership/snapshot structures are credible; F03 leaves composed resource enforcement incomplete, and final-source native qualification remains open |
| G6 Transformation/reuse | **pass, examined contracts** | Exact views, current bindings and pinned ranking identity remain explicit; F02 prevents broader extension certification |
| G7 Truthful claims | **pass at stated maturity** | This assessment makes no Tested/Measured or whole-product completion claim |
| G8 Library leverage | **pass, examined choices** | Native indexes/transactions, Tokio ownership, established graph kernels and existing reservation mechanisms remain the appropriate foundations |
| CI-G1 Fidelity | **pass, bounded state review** | No new relation relabelling or heuristic-as-fact path was found; provider-wide fidelity was not requalified |
| CI-G2 Evidence closure | **pass, inspected identity contracts** | Exact publication/view/definition and continuation bindings remain explicit |
| CI-G3 Evaluation integrity | **not applicable to changed behavior** | No evaluation population or evaluator route was changed or comprehensively audited |

Gate passes are source judgments for the stated scope, not fresh runtime receipts.

## 8. Library fit, alternatives and total machinery

The pinned SurrealDB skill and vendored SDK source support using native indexed candidates and transaction-bound queries. `Transaction::query` retains its transaction identity; the local `cancel_ref` and session invalidation methods expose the finalization needed by selected backup. This interface inspection does not establish composed server execution or planner performance.

| Alternative | Assessment |
|---|---|
| Complete selected enumeration for every read | Appropriate for integrity/global consumers; disproportionate for sparse typed retrieval |
| Indexed candidates plus exact membership | Preferred for sparse requests; retains native selectivity without surrendering view authority |
| Whole-main logical export | Adds unrelated transport/spool work; selected transaction-bound export is the better target |
| One large retirement transaction | Mismatches high-degree failure scope; bounded durable phases are preferable |
| Generic state/workflow framework | Adds authority and lifecycle machinery without resolving these concrete gaps more directly |
| Existing ownership/reservation primitives | Sufficient for the corrections; no new cache validity graph or resource framework is justified |

The added native lifecycle protocol has real complexity, but its distinctions purchase delayed-operation fencing, exact recovery and safe content reuse. Retain them. The simpler correction is to narrow consumer dispatch and physical selection, rather than replace the state platform.

## 9. Verification and uncertainty

**Performed:** `git status --short`, `git rev-parse HEAD`, and targeted read-only source/authority inspection.

**not_run:** Builds, tests, native plans, service migration, selected export/restore journeys and assembled qualification. The coordinator owns these commands and receipts.

The assignment reports three model and 69 host controls passed earlier on 2026-10-10. Those are attributed context, not this reviewer’s executions or final-source product qualification.

The diagnoses in F01–F03 do not require new probes: their executing paths establish the amplification and authority/resource seams. Their corrections require source-specific review and the relevant revealing controls. Snapshot consistency, conflicting native guards, delayed issuance, repeated successors and final-source independent restore remain consequential runtime qualification boundaries.

## 10. Rule impacts and disposition

**Authority changes: none.** These recommendations refine the already selected target. They do not require replacing ADR-0145’s architectural direction or the operator’s accepted holistic RC01–RC03.

The coordinator remains sole owner of scheduled disposition in §8 and actual receipts in §9.1. This review must not be rewritten as accepted because repairs are underway; a dated follow-up should identify the changed source and settle the affected findings.

## 11. Architectural decision and next action

**Bounded decision: Revise.** Preserve the inspected audit finality, retirement/successor fencing, selected-snapshot backup, compiler release and ranked-borrower structures. Correct F01–F03 before accepting the assembled architecture for sparse, concurrent catalog/evidence use and recoverable-family evolution.

**Enclosing architecture:** needs revision at the identified reader-selection and recovery-adapter boundaries. Other inspected mechanisms have credible source-level designs, with product/native qualification still open.

The next architectural action belongs to the native reader/selection and recovery-family adapter owners. After their corrections stabilize, an independent follow-up should assess the actual delta. The coordinator then integrates that judgment with final-source functional/native receipts; neither substitutes for the other.

## Bounded repair follow-up — 2026-10-10

**Verdict: Accept the reviewed repairs at Implemented/source-inspected maturity.** F01, F02 and F03 are resolved in the frozen source. This follow-up preserves the original **Revise** assessment as the historical baseline. Functional/native qualification and current disposition remain coordinator-owned.

**Identity and scope.** This assessment covers the frozen closure, sparse-reader/resource and host-replacement deltas, including the production/test freeze associated with managed all-test check `20261010T162755.703Z-56c28b`. It also covers the coordinator’s explicit indexed-ID declarations and history-generation guard. The principal review’s standard, profile, domain model and workload premises continue to apply. Review was read-only; no builds, tests, native effects, formatting or writes were performed.

### Finding reassessment

| Finding | Bounded assessment |
|---|---|
| **F01 — sparse reads enumerate selected payloads** | **Resolved in source.** `NativeReader::candidate_payload_rows` bypasses `selection_preparation`, including on the first request. Finite indexed nominations enter direct `view_nodes` membership probes and indexed reverse-alias ancestry. Only accepted physical identities are hydrated. Keys, atomic scopes, connected reads, serving scope/projection/original checks and constrained occurrence reads use this boundary. Complete preparation remains available for broad/global/integrity work. |
| **F02 — recovery families do not govern dispatch** | **Resolved in source.** The model owns the family inventory and integrity classification. Exhaustive native mapping, export dispatch, data-only codecs, restore selection and cold-comparison grouping require explicit treatment of new families. Actual graph/search claims remain preserved independently of producer expectations. |
| **F03 — native preparation bypasses caller resources** | **Resolved in source.** Readers distinguish retained shared capacity from scoped request scratch. Retained selection checks pool identity; ordering, membership probes and cursors use supplied pools. The reviewed production selection paths no longer create a separate fixed pool. Immutable borrowers retain their original charges. |

The first-request distinction matters: caching a complete membership inventory would still conflict with storage §6.2’s sparse-startup target. The final sparse route removes that dependency. Reverse alias traversal performs relevant reachability work without loading unrelated view membership or payloads.

The pure sparse-driver control rejects whole-view/payload enumeration and checks scratch refusal, release and retained-borrower charging. The native control exercises sparse keys, scope, aliases, selected absence and conflicts **before** explicit global preparation. These controls were source-inspected, not executed by this reviewer.

### Installer replacement and recovery

The explicit replacement route is accepted within its narrow recovery contract:

- It requires dead-owner recovery, exclusive maintenance/drainage ownership and an owned-daemon restart before inspection.
- Both scopes must retain exact source-3/current-generation markers with admission closed. Every predecessor native journal must be absent or exactly `intent`; advanced or changed journals are rejected.
- A distinct immutable candidate and operation preserve predecessor plans, native-journal evidence and journaled rotated authentication assets.
- A changed executable may retain the same target schema hash, allowing migration-code repairs.
- Candidate-directory and operation-directory ancestry are flushed before durable marker publication. A crash before switching the marker leaves an inert successor bundle.
- Explicit private-bundle dependencies protect predecessor configurations and failure diagnostics alongside executables and plans.

Source marker 3 remains an unpublished identity, **not evidence of pristine DDL**. The successor deliberately handles supported partial declarations. The route does not authorize replacement after translation may have begun.

The owner reports **99 mocked host/storage controls passed**. That evidence supports the host state machine; it does not establish native migration acceptance.

### Additional bounded corrections

The explicit `id TYPE string` declarations cover all 13 identified indexes containing `id`. Current and baseline owned constructors use string/hash keys. Pinned SurrealDB 3.3 source confirms that this declaration constrains the record key while preserving outer record identity.

History collection now requires exact current generation inside the locked deletion transaction. Retirement items obtain generation authority from their locked parent. Foreign-generation candidates remain protected while the cursor advances. The extended control asserts this boundary using an exact freshly owned corruption fixture; separately qualified inventory remains required.

### Principles, gates and limits

A1/A2’s F02 authority defect and A4’s F01/F03 execution-fit defects are resolved in the reviewed source. A3’s prior bounded assessment remains applicable. The corresponding G1 defect is closed; G5’s final-source verification boundary remains open. No new framework gate, numerical performance claim or standard/profile rule change is introduced.

No remaining material source finding was identified in these assigned deltas. This is **bounded repair acceptance**, not assembled product/native qualification.

**Coordinator qualification update, 2026-10-10:** The current all-test compilation exposed one missed test caller budget argument. Its correction does not change the reviewed production design. Compilation and final-source functional/native outcomes remain coordinator-owned and are not claimed as passed here.

## Additional recovery and server-tooling follow-up — 2026-10-10

**Verdict: Accept the bounded source deltas identified below at Implemented/source-inspected maturity.** This assessment preserves the original Revise baseline, the earlier F01–F03 source closure and the open functional/native qualification boundary. It does not assess the ongoing host server-handoff changes. The exact hashes identify the examined snapshots even where concurrent work subsequently changes the same files.

| Examined snapshot | SHA-256 |
|---|---|
| `scripts/surrealdb_service.py` — published-scope candidate replacement | `b1108569c97fd84c9e75162df21b46d7ad849810489d4a549a0e12730340596f` |
| `crates/lctx-surrealdb/src/loader.rs` — index readiness and allocation-free helper | `737472029660db1db6aab1db7d8606ad9efcc460b8bb8c4e32859874cd08893c` |
| `crates/lctx-publisher/src/backup_import.rs` — actual-size literal admission | `3446525fa7490f7a1d880f447622596b10d984d9fdda9e6000fe312bc2aa5c45` |
| `crates/lctx-publisher/src/restore.rs` — explicit alias-retention pressure control | `2ba4e0a4e1d1d41ea85438af19a462c77214e2fa87240e785e9e398ed8aa2b93` |
| `scripts/surrealdb_server.py` — exact patched-server build and verification owner | `bd76ca39677437684bba113fe17b8f8892d67f0908f8be5cf610268dcbccb555` |
| `scripts/storage_service.py` — immutable server-generation preparation | `6998d8459a9dd44fda64c2e9bd84eb1c1fea740d4c71cff6e093694f8a617097` |
| `third_party/surrealdb-server/timeout-tree-stack.patch` | `96ee7aeb1c733baba3b57e4850cbc8b3b5db69650252111ecf3e751b7a39988a` |
| `third_party/surrealdb-server/provenance.json` | `9bd3cb1660d92b867db3675eb22d8ec3b887b5383879c995883479c7f067f928` |

### Published-scope candidate replacement

The new route extends the earlier pretranslation-only replacement boundary to a checkpointed published scope, without treating publication as permission to change schema identity. A new immutable **host operation** owns the successor executable, private files and diagnostics; the retained **native operation** preserves the already published migration identity.

After owned-daemon restart and drainage, each completed host scope checkpoint must agree with the exact closed format-4/current-generation marker and the retained native journal's published state. Remaining format-3 scopes still permit only absent or intent journals; declarations and translated states remain refused. If any scope has published, the candidate must declare the identical complete target object. Completed scopes are retained in the immutable successor, skipped during migration and protected against lost host checkpoints.

Historical observations remain immutable. Only the same retained native operation and complete target may advance from absent/intent observations under the new preflight proofs; unrelated ancestors remain exact. Journaled credentials and their original credential operation remain preserved. This is a coherent recovery capability for an execution-only binary repair after partial scope publication. It does not authorize implicit schema substitution or operator adoption.

### Loader readiness, resource admission and revealing controls

The loader changes only the index AST's `CONCURRENTLY` execution flag and normalizes that flag out of declaration identity. It checks declaration readback and then waits for **all desired indexes**, including catalog-present retries, to report terminal ready status. Malformed, unknown, failed or aborted states fail closed with phase/index context. The implementation adds no healthy-command timeout or compiler concurrency cap.

The native payload helper is public at the adapter boundary and traverses every record-key family without allocating SQL. The dump writer first validates its closed literal families, reserves a conservative actual-value bound for escaping and String growth, then encodes and charges the resulting capacity. It retains the legal statement-size limit and releases its encoding charge after writing. Tiny escaped/binary records no longer require the maximum legal statement allowance. The revealing control checks roundtrip preservation and resource refusal before any output.

The restore pressure control now reserves explicit shared-pool pressure after dump parsing. Its alias-retention refusal therefore tests the intended allocation boundary instead of depending accidentally on the number of spool families.

### Patched-server tooling and verification boundary

The recipe binds the exact upstream commit, lock, manifest and patched executor hashes, checks the entire source status before and after compilation, and retains default features plus `cjk`. The two patch hunks replace the dropped evaluator TreeStack in transaction-timeout error branches; they change neither deadlines nor feature coverage.

The build uses available logical CPUs, normal Cargo parallelism, the pinned toolchain and explicit compiler/linker settings. Foreign incremental and jobserver overrides are removed locally. Storage admission precedes acquisition and persists through shared process-owner child drainage. Source and build caches retain named warm consumers. Immutable server bytes and their recipe-bound descriptor are verified and their directory ancestry flushed before publication. Descriptor verification holds generation admission through its owned version-check child and confirmed cleanup.

The owners report **114 mocked host/storage controls passed** for the published-scope recovery snapshot and **15 pure server-tooling controls passed** for the final tooling snapshot. These are attributed evidence, not reviewer executions or native acceptance. The reviewer performed no acquisition, build, test or native operation.

The new ignored maintenance-only `transaction_timeouts_preserve_executor_stack_and_following_queries` control was source-inspected in `native_control.rs` snapshot `c76f52871cbb5a09e445ad0b78725d3fae8bb79c6d93f524ba8c5ea80c168989`. It exercises implicit and explicit BEGIN/COMMIT timeout branches using the unchanged ten-second transaction and twenty-second query deadlines, requires an actual timeout error, and checks succeeding same-batch and fresh queries without durable rows. Its execution remains pending coordinator-owned patched-server adoption and migration.

The principal review's standard, profile and qualitative principles continue to apply. These deltas preserve explicit authority, recovery lineage and composed resource ownership; no new standard/profile rule, framework gate or numerical performance claim is introduced. No remaining material source blocker was identified in the listed snapshots. Server acquisition/build, host handoff, patched runtime behavior and final-source product/native qualification remain separate coordinator-owned outcomes.

## Server handoff and dependency-repair follow-up — 2026-10-10

**Verdict: Accept these bounded source deltas at Implemented/source-inspected maturity.** The original Revise baseline and previous finding dispositions remain intact. This assessment covers server-generation publication recovery, the frozen host handoff and the DiskANN successor recipe; it does not establish runtime handoff, migration or patched-server acceptance.

| Examined final snapshot | SHA-256 |
|---|---|
| `scripts/surrealdb_service.py` — exact server handoff | `220b8f4642482a1264647dda184de9c03c89e605ddf0228325a734824629662c` |
| `scripts/storage_service.py` — publication crash recovery and dependency bridge | `20e6848d5fb41701ebfd1cbc81b49b1323c818aef4754bd37c6f24bd8bc03bee` |
| `scripts/surrealdb_server.py` — successor recipe and network-enabled locked metadata | `ceb8818357d1e2695dcd051da892bc12ead4a374b1d4f4da77a8d45f5dfdb998` |
| `tests/scripts/test_surrealdb_service.py` | `cf45e6e6cac847c6c02f7898db9594e8414b739ee537b2ac8c3cfbf95cdbaa5c` |
| `tests/scripts/test_surrealdb_server.py` | `36970308e247702e1ea9ca00460b244852476892d4092d515ee537073f6f4791` |
| `third_party/surrealdb-server/diskann-provenance.json` | `fdb57b0d06728f57648683177936006b6803f10694c956c728b3470446cff734` |
| `third_party/surrealdb-server/diskann-batch-borrow.patch` | `eaaa0904e7b2e1b574c3445cae529c641de95d609b9d739ecb5f36a6580a9783` |
| `crates/lctx-surrealdb/tests/native_control.rs` — explicit integer constructors | `acf380e1f2f48c50b6a7664e23f60091e08afe431eca44840b000d498cfc6a5c` |

### Exact handoff and recoverable publication

Server-generation preparation now repairs the crash window after writing an equal descriptor with private writable mode. It opens the exact marker without following links, checks its owner, regular-file shape and complete value, then applies immutable mode and flushes that same descriptor before publication. Foreign-owner or symlinked markers remain refused.

The host handoff owns a separate immutable plan binding both server generations, exact previous/successor unit texts and the pending native upgrade object. Exclusive admission, dead-owner checks, borrower drainage and confirmed daemon/descendant stop precede the durable unit and installation switch. Resume accepts only the same generation and native upgrade, permitting the journaled old/new unit and installation combinations needed to recover the unit-before-installation crash window.

For the mixed main-4/validation-3 case, the native operation, completed-scope checkpoints, private credentials and environment remain unchanged. The pending path performs no migration, credential rotation or admission reopening. Exact executable, settings, storage, endpoint, CLI identity and HTTP readiness/version checks precede both closed scope-marker checks and read-only native-journal preflight. Failed checks retain maintenance; successful pending handoff removes only the handoff member from the maintenance marker.

Dependency traversal protects both generations, each immutable handoff bundle and predecessor lineage. Patched cold backup/restore binds the archived descriptor and executable within the same installed recovery identity. Source, build, original-cache and override/archive paths remain provenance references, with no existence or rebuild requirement. The final source control explicitly exercises their absence.

### DiskANN repair and borrowed build caches

The one-line DiskANN 0.56.0 repair passes `batch_clone.as_ref()` to `set_chunk<B: Batch>`, preserving the captured `Arc<B>` through the awaited call. The reviewer independently inspected that exact cached signature and caller, and verified that archive, normalized manifest and original source hashes match the declared pins. The repair changes neither the manifest nor feature selection.

The successor recipe has its own record, checksum-bound archive and exact extracted override. Removing only `dependency_patches` recovers the original recipe identity and physical source/target/intermediate paths. Both roots are exclusively admitted through owned child cleanup. An existing original record is preserved byte-for-byte and its digest enters successor provenance; the original source remains subject to the same pinned patch/status checks. Exact override verification rejects missing, changed or additional files.

Cargo receives the same `paths` override during metadata and build. Metadata requires the owned DiskANN manifest, exact version and unchanged upstream lock. Removing `--offline` allows missing locked dependencies to be acquired; `--locked`, target/features, source selection and lock-digest checks remain. This changes acquisition availability without changing recipe identity. Retained dependencies name both cache roots, the original record and successor override/archive.

### Evidence, principles and remaining boundary

The owners report `uv run --no-sync pytest -o addopts='' -q tests/scripts/test_surrealdb_service.py tests/scripts/test_storage_service.py tests/scripts/test_surrealdb_server.py` **passed: 154 controls in 17.51 seconds**, with scoped diff checks passed. The separate final server/storage run reports **passed: 23 controls in 1.71 seconds**. These mocked/pure controls support the host/tooling state machines; they are not native acceptance. The fixture-only provenance update uses the production builder with an explicit fake original-record digest; host production code retains the hash above.

The coordinator reports actual locked metadata run `20261010T173605.150Z-dec4a4` **passed**, exit 0 with confirmed cleanup. Native integration binary listing run `20261010T173319.807Z-8b6d54` also **passed**, establishing compilation/listability rather than executed native behavior. Server build `20261010T173627.265Z-57c000` was running when this assessment was appended. Server adoption, actual handoff, timeout regression, resumed migration and final-source product/native qualification remain pending. The timeout control's explicit `Number::Int` constructors preserve its previously reviewed contract.

A1–A3's authority and recovery assessments remain applicable; A4 accepts reuse of warm build caches with exclusive ownership and unchanged compiler parallelism. No additional material source blocker, standard/profile rule impact, framework gate or numerical performance claim is introduced. The reviewer ran no tests, acquisition, metadata, build or native operation; only this assigned review appendix was written.

## Fixture server-identity follow-up — 2026-10-10

**Verdict: Accept the bounded source correction at Implemented/source-inspected maturity.** Frozen `scripts/surrealdb_fixture.py` SHA-256 is `ddd402b95ed12e5fe06cb5ef461bd871747e3b9919c4a9a32922ad1e22ac4bc4`; affected fixture-test SHA-256 is `e13547329edabd5e256360722893783be464b8914234eccbd5d6935a75981cba`, and service-test SHA-256 is `017a4962d70ee774eb610fc3fd2b1f1bbe8473db3bd93396991b4946128ab2c6`. Production service code remains the previously reviewed `220b8f4642482a1264647dda184de9c03c89e605ddf0228325a734824629662c` snapshot.

Fixture identity now delegates verification to the canonical installed-binary owner and projects its actual version, binary digest and optional owned-generation identity. Readiness, attachment configuration, retained configuration and producing/reuse inputs consume that projection. The inspected Python call sites all supply their installation; the fixture no longer substitutes official-release constants for the installed patched server.

Retained immutable publications remain reusable across a server-generation change. Their original producing identity and content are preserved, while reuse records the verified current server and the input difference. Configuration/selection digests, installation readiness and native opening retain their independent checks. This gives truthful provenance without confusing execution provenance with immutable publication identity. The revealing mocked controls cover official and patched identities, readiness detail, reuse without rewriting the original receipt and rejection of a tampered installed identity.

The owner reports `uv run --no-sync pytest -o addopts='' -q tests/scripts/test_surrealdb_fixture.py tests/scripts/test_surrealdb_service.py tests/scripts/test_storage_service.py tests/scripts/test_surrealdb_server.py` **passed: 185 controls in 25.60 seconds**, with scoped diff check passed. An earlier composite had 184 passes and one stale fake-readiness mock failure; the narrow mock correction preceded the final receipt. This is attributed mocked/pure evidence, not native acceptance. No new material source finding or standard/profile rule impact was identified. Original verdicts and pending server build/adoption, migration and product/native qualification remain unchanged. The reviewer ran no tests or runtime effects and edited only this review appendix.

## Migration origin-memoization follow-up — 2026-10-10

**Verdict: Accept the bounded source delta at Implemented/source-inspected maturity.** Examined `crates/lctx-surrealdb/src/control/migration.rs` SHA-256 `ecd7801eda9b72e56288d1cb06dc7d7b8dec938b0684a03350b5443b167546f4`, specifically the two cleanup-origin helpers and their page-local consumers in `verify_legacy_state`.

Both maps use complete `RecordId` keys, preserving table and key identity, and are recreated for every bounded verification page. Their first entries come from the actual native attempt/contribution queries with the existing cardinality and decoding behavior. They do not use translated expectations as an oracle. Cleanup epoch, terminal-state and unadmitted checks remain in sequence; each owner still undergoes type, product-cardinality, contribution and exact attempt-origin validation. Errors return at the same validation boundary, and individual product lookups remain independent.

Reuse of a first observation is sound within the existing closed, drained, exclusively owned migration interval. The helpers introduce neither cross-page/invocation state nor new admission authority. This improves repeated-origin execution fit without weakening independent native verification or changing publication order. No new material source finding, standard/profile rule impact or numerical performance claim was identified.

The owner reports `cargo check --release -p lctx-surrealdb --lib` **passed in 2.19 seconds**. This establishes compilation only. The running immutable migration executable was untouched; the new source has no runtime or profiling acceptance here. Previous verdicts and qualification obligations remain intact. The reviewer ran no checks or native effects and edited only this review appendix.

## Final assembled HS0–HS10 source assessment — 2026-10-10

**Verdict: Accept the assembled HS0–HS10 source at Implemented/source-inspected maturity. HS11 is not accepted.** The original Revise baseline remains the historical principal assessment. Its three material source findings are integrated and resolved by the reviewed repairs; the intervening follow-ups preserve their exact evidence boundaries. No additional material structural or ownership omission was identified in this final bounded reassessment. Current scheduled disposition and execution receipts remain exclusively coordinator-owned.

**Source identity and method.** HEAD remains `125e74f5775e1354ce900cc8d71b190e311dac27`, with the frozen concurrent dirty source inventoried by `git status --short`. A read-only identity inventory of 481 source/schema/manifest files has SHA-256 `be7857aad48d21d6c21d9746df4cd48baa91c667b788e1ea3093f4655c4229c1`. This hashes relative `Path` values in path-component order via `sorted(paths)`, adding each path's POSIX text, a NUL byte and its binary SHA-256 digest: `.rs`/`.surql` files below the six reviewed product crates' `src` directories (`lctx-model`, `cpg-core`, `lctx-surrealdb`, `lctx-publisher`, `lctx-serving`, `lctx`); their Cargo manifests; root Cargo manifest/lock and toolchain; the four reviewed service/fixture/server/storage scripts; vendored SDK begin/transaction methods; and files under `third_party/surrealdb-server`. This identifies the source snapshot, not line-by-line coverage or a new acceptance gate.

The reviewer reread the holistic plan's package and investigation contracts, current disposition context and decisive integration boundaries. The original standard, profiles, domain model, workload scenarios and mechanism alternatives continue to govern; the existing principal assessment plus follow-ups provide proportionate assembled coverage. This reassessment sought remaining integration defects rather than repeating every already reviewed implementation.

| Assembled source scope | Independent judgment |
|---|---|
| HS0 / HS3 — governing closure and decisions | ADR-0145 and the enduring owners distinguish semantic recovery from physical mapping and runtime authority. Model family inventory and exhaustive adapter dispatch govern actual export/import/restore treatment. Actual claims and independent cold admission remain separate from expected lowering. F02 is integrated. |
| HS1 / HS7 — audit and selected recovery | Audit retains its compiler owner before fallible capture and drains before pin/session release. Delivery loss retains an observed finalizer. Selected export owns one transaction and exact pin through metadata/content reads and checked cancellation; data-only restore retains independent admission. |
| HS4 — complete physical access | Sparse finite candidates bypass global selection preparation on first use, intersect exact indexed membership/reverse aliases before hydration and use supplied resources. Broad integrity/global enumeration remains explicit. F01 and F03 are integrated; actual planner and terminal behavior still need native qualification. |
| HS5 / HS6 — lifecycle and retained authority | Claim/page/finalize retirement, shared writer guards, immutable issuance eras, distinct maintenance successors, named references and qualified history horizons remain composed at their actual effect owners. Current-generation collector protection and page-local migration origin reuse preserve those boundaries. Source acceptance does not authorize unqualified destructive collection. |
| HS2 / HS8 / HS9 — active execution state | Selected continuation limits reach replay/final admission. Compiler preparations release after their last remaining selected consumer, including optional/cache-hit routes. Ranked pending entries stay private through packing; immutable borrowers retain service charges outside the map lock. |
| HS10 — bounded investigation outcomes | Per-entry immutable hydration serves repeated real consumers with charged lifetime. Cross-attempt interner injection and extra SCC scratch remain unwarranted without named repeated consumers. Historical concat attribution is not invented. Missing-library behavior, executable-epoch coexistence, planner behavior and selected snapshot semantics retain their actual functional/native obligations. |

**Foundations and gates.** A1 and A2 now satisfy the examined extension/authority scenarios; A3's composition judgment remains satisfied. A4 now fits sparse demand, retained active borrowers and necessary complete work without a separate resource framework or compiler concurrency cap. The original G1 failure is resolved in source. G2–G4, G6–G8 and the bounded CI judgments retain their stated source scope. G5 remains unresolved for assembled runtime consistency/recovery until actual qualification supplies its evidence. No standard/profile rule change, new framework gate or numerical performance claim is introduced.

**Actual acceptance boundary.** The coordinator reports the exact patched server built and was adopted through the reviewed handoff. The immutable `53f28eeb…` native migration is healthy and running; that is progress, not completed migration or acceptance of the subsequently memoized source. The coordinated final native sequence `d35925…` awaits migration, final CLI, pure and authentication prerequisites. Planner/access-path qualification, snapshot coherence/cancellation, independent populated cold restore, delayed issuance and successor interleavings, final native/provider/serving journeys and applicable leaves remain actual acceptance work. Earlier narrow passes, compiled/listed binaries and source acceptance do not satisfy HS11. The reviewer ran no tests, probes, formatting, service or database operations and edited only this integrated review.
