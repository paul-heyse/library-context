# Graph-native replacement — implementation coordinator

**Accepted target / implementation in progress, 2026-10-05 (ADR-0128).** Replace the PostgreSQL-centered compilation and serving architecture
directly with a Rust-admitted graph and SurrealDB-native persistence, querying and search. This
coordinator owns the combined execution sequence, shared decisions, finding disposition and
completion boundary. Supporting plans develop their respective designs; they do not create
another task ledger. Package evidence below distinguishes implementation from functional acceptance and activation.

## 1. Basis, baseline and intended outcome

The two source reviews are the [graph-native target](../design_review/reviews/design_review_graph-native-target_2026-10-05.md)
and [SurrealDB capabilities](../design_review/reviews/design_review_surrealdb-capabilities_2026-10-05.md).
They are the design basis for this series. Other reviews are not additional grounds or prerequisites.
Current architectural owners and code supply product meaning and the implemented baseline.
Use core 3.3 / code-intelligence 1.4 from [standard.toml](../design_review/design_principles/standard.toml),
including FP-07/A4, rather than retroactively relabelling the source reviews' earlier standards.

Baseline inspected at `6f1a7e98`, clean `main`, 2026-10-05. Native acquisition, the pure typed
domain operations, analytics kernels and FastMCP transport exist. Compilation, admission effects,
embedding cache, source bytes and serving still depend on PostgreSQL. Previous fixture receipts
bound that implementation only. The interrupted FastMCP attempt described by STATUS is disposable;
neither its rows nor its timings are a replacement baseline to reproduce.

The resulting product lets an agent discover a release-scoped API, select compatible invocation
and configuration contexts, obtain an implementation packet, and reach exact original evidence.
Provider disagreement, dynamic boundaries, unknowns, partial coverage and heuristic ranking remain
visible. Catalog readiness remains independent of optional behavioral analysis and brief seeds.

This is a **hard design-phase pivot**. Replace mechanisms wholesale at their ownership boundaries.
Build fresh artifacts from pinned inputs. Retain no old-format reader, old-ID bridge, PostgreSQL
runtime, dual write, intermediate PostgreSQL redesign, migration of old generations, rollback
database or historical artifact archive. Useful domain operations and independent test expectations
can survive without preserving their old storage shape or receipt protocols.

## 2. Plan routes and shared contracts

| Plan | Responsibility |
|---|---|
| [Model and compiler](graph-native-model-compiler-plan_2026-10-05.md) | Graph meanings, identity, admission, spillable workspace, store-free producers and compiler frontiers |
| [SurrealDB realization](graph-native-surrealdb-realization-plan_2026-10-05.md) | Native layout/codecs, server/SDK, cache, loading, reconciliation, executable sealing and publication |
| [Serving](graph-native-serving-plan_2026-10-05.md) | Native queries/functions, selection and search, packets, original evidence, continuation and MCP migration |
| [Projections](graph-native-projections-plan_2026-10-05.md) | Named projections, petgraph/analytics integration, source lineage and useful exports |

Rust owns semantic questions and operation contracts. That ownership does not require their
execution in the host: SurrealQL may be the sole implementation of a model-owned operation.
Generate mechanical envelopes and mappings; do not introduce a universal Rust-to-SurrealQL
compiler. Keep complex existing kernels where that is simpler, obtaining their inputs in coarse
native operations. Python remains transport/presentation, with no second classifier.

The shared products are concepts, not a requirement for one service/type per name:

| Product / owner | Required meaning and consumer |
|---|---|
| Admitted graph / model and compiler | Immutable typed entities, assertions, roles, provenance, originals, coverage and selected outcomes; canonical content manifest and completed admission result. Publisher consumes it without rerunning producers. |
| Compilation workspace / compiler | Attempt-private ordered streams, indexes and spill segments. Completed input visibility follows semantic dependencies, independent of database roles and stage receipts. |
| Published realization / publisher | Semantic content plus physical-format revision, functions, analyzers, index/embedding specs, engine identity and adopted module bytes; exact database and read-only handle. |
| Operation / semantic owner | Inputs, effects, selection/answer meaning, incompleteness and resource controls. A query/function realizes the contract; metrics do not define it. |
| Projection / consuming owner | Source graph, universe, direction, multiplicity, weights, simplifications, algorithm/settings and mapping back to semantic IDs. |

Publication and selection remain distinct. A running server pins one complete realization;
references, resources and cursors cannot silently resolve against a replacement. Logical content
identity and a new physical/search realization are separate. Definitions cannot change beneath
pinned readers. Original bytes and consumed vectors are content-authoritative; engine indexes are
derived. The embedding owner remains [the exact Qwen specification](../../specs/embedding/qwen3-embedding-8b.json).

## 3. Efficient execution by design

**Native SurrealDB querying is selected.** Functional tests check its implemented behavior;
they do not decide whether native querying belongs in the architecture. Apply FP-07/A4 as design
principles using first principles, workload knowledge and library capabilities. Do not turn them
into accounting rules, performance proofs, planner receipts or an adoption gate.

Workload premise: one operator, one local persistent server shared by compiler/publisher and MCP,
pinned Python-library inputs, many attributed observations, small ordinary API/evidence answers,
skewed graph degrees and optional larger analytical projections. The inspected host has 32 logical
CPUs / 16 physical cores and approximately 188 GiB RAM (`lscpu -J`, `free -b`, 2026-10-05);
concurrent host use matters. These are deployment observations, not process budgets or measured
capacity claims. The source reviews' roughly 16.6M-row attempt indicates meaningful input size,
not a requirement to preserve that representation.

Choose a bounded number of bulk passes and suitable external ordering for admission. Share
immutable input preparation and analytical topology. Keep CPU/embedding work outside store
transactions. Use compact physical families, native adjacency, supported index composition,
filtered KNN, database functions and batched hydration to remove incidental crossings.
Fixed evidence paths execute natively; variable exploration uses native limited segments and
retains a frontier when the answer actually needs continuation. Do not enumerate in the host
solely to obtain exact work counts or duplicate every occurrence's ANN vector merely to simplify
accounting.

Resource control combines sensible operation shapes, parameter/result/byte limits, native query
deadlines and cancellation, server memory/cache configuration and bounded compiler/transport
buffers. Distinguish those controls from an output limit. Explain a partial answer truthfully
when execution stops. No universal “abort after N engine steps” API or exact per-request counter
is required. Native `EXPLAIN` / `EXPLAIN ANALYZE` are available diagnostic tools when a concrete
query-shape question merits inspection, not mandatory checks for every operation or request.
Detailed measurement is necessary only for quantitative speed/capacity claims or a consequential
unresolved implementation choice.

The [serving plan](graph-native-serving-plan_2026-10-05.md#3-native-query-design-and-resource-controls)
records the investigated native plan capabilities and their limits. The design order is remove
unnecessary work, use better native/bulk access and locality, reuse stable preparation, then tune
concurrency if needed. A refusal at realistic size is not the performance objective.

## 4. Coherent packages and dependency order

The complete compiler stage is **complete / user-accepted, 2026-10-05**, with partial verification
and the user-directed stop recorded in §8. M1/C1/C2-N/C2-U and compiler-side A1/A2 are implemented,
including their G0/S1 interfaces. G0 adopts ADR-0128; S1 supplies the agreed native operation contracts;
P1/P2/S2/S3, published exports, assembled Q0 and operator Q1 remain **not_started**. Supporting plans supply implementation detail and focused
controls; this table owns current package state. Root owns shared declarations, manifests,
architectural decisions and integration. Independent investigation may run in parallel; ordinary
edits remain on shared main unless genuinely concurrent production edits justify worktrees.

| Package | Delivered behavior | Actual prerequisite |
|---|---|---|
| G0 — record the replacement | Superseding ADR(s), owning DESIGN sections/binding, public command target and actual consumer/deletion inventory adopt this one pivot. Remove obsolete prescriptions once their surviving obligations have a current owner. | This series and its two source reviews; no old-green gate or additional store-choice review |
| M1 — graph and identity boundary | Typed graph construction, narrow semantic identity, layered manifest/admission contracts and one representative catalog/evidence graph. | G0; include native retrieval requirements from S1 in the shared design |
| C1 — bounded facts workspace | Store-free capture/extraction and facts admission with attempt-owned streams/spill. No database connection required by compilation. | M1 |
| P1 — native realization foundation | Managed server, strict compact layout, typed SDK/codecs, original chunks, control record/cache and one coarse evidence operation. | M1 + S1 contract; full upper compiler not required |
| S1 — native operation design | Predicate-specific selection inputs, shared-vector eligible-occurrence retrieval, packets and native fixed paths designed with their indexes. | M1 design; feed layout/function needs into P1 before freezing it |
| C2-N — normalized input slice | All normalization consumers use completed workspace views, including unresolved and conflicting observations. | C1 |
| A1/A2 — projections and analyses | A1 prepares reusable workspace topology/contracts; A2 integrates each kernel when its actual semantic/vector inputs exist. Published export follows P1/P2. | M1 + C2-N for topology; Summary/Structural/analytic kNN additionally consume their relevant C2-U Local/execution/Model/vector slices, not final upper admission |
| C2-U — upper compiler | Local/execution/transfer/models/Summary/Structural, catalog/selection/synthesis/retrieval and selected analytics construct the admitted graph. | C2-N + relevant A1/A2; cache effect from P1 only when embeddings requested |
| P2 — publish admitted content | Checked bulk load, one persisted reconciliation, ready indexes/functions, drained writers, sealed handle, short visibility transition. | P1 + admitted C1 artifact; repeats with C2-U artifact for upper-frontier acceptance |
| S2/S3 — migrate serving | Native search/hydration, retained authoritative complex kernels, evidence/continuations and all CLI/native/MCP consumers. | P2 + S1; representative lower controls may precede full C2-U |
| I1 — close the pivot | Replace tool/readiness/test recipes and remove every PostgreSQL consumer/dependency, old lifecycle and historical runs interface. | M/C/P/S/A replacements available for each affected consumer; deletion occurs with its replacement, not after a parallel runtime period |
| Q0 — assembled functional acceptance | Affected contract families and representative actual persistent-store/native/MCP journeys pass on one tree, plus applicable leaves. | Integrated required packages; no legacy suite retention prerequisite |
| Q1 — fresh operator adoption | Fresh pinned-library compilation/publication, separately authorized live-embedding checks and activation; dispose obsolete state after readers close. | Q0 and explicit authorization for real-library/runtime actions |

C2-N is an early slice of C2, not another published stage. A1 needs those normalized inputs;
each A2 integration additionally waits for its real C2-U predecessors (Local, execution/Model,
source-call frames or vectors as applicable). Final C2-U admission consumes their outputs.
These completed-input slices break the apparent compiler/analysis cycle without pretending all
analyses can run over normalization alone.
P2 initially accepts a lower-frontier artifact, without claiming catalog readiness. A compile
dependency does not create a new durable store stage or a synchronization barrier per relation.

## 5. Replacement and public interfaces

Keep `lctx compile <library> --through facts|normalized|analysis|catalog --profile catalog|behavioral`
as the cumulative user capability. Default orchestration compiles, admits and publishes an
**unselected** snapshot; add `--artifact-only` for store-free compilation and a separate
`lctx snapshot publish <artifact>` for consuming that admitted artifact. The compiler library
itself takes no store credentials. An artifact remains only while a current publisher/export/debug
consumer needs it; it is not a runtime archive.

Replace `generation` management with `snapshot list|show|select|retire|audit`; identify complete
realizations in output, resources and continuation. Replace store install/check/reset and query
commands with their SurrealDB realization semantics, without old SQL/role/schema aliases.
The product MCP tool names and domain outcomes remain; their storage identifiers deliberately
migrate to the complete snapshot/realization contract. Store configuration takes explicit server,
namespace/control database and credentials, with no PostgreSQL environment fallback.

I1's closure includes `lctx-postgres`, `cpg-core::postgres` and generation-bound compute providers;
SQLx/pgpq/pgvector dependencies where no remaining consumer exists; Python storage/native build
inputs; PostgreSQL specs/migrations/bootstrap/backup/readiness scripts; query/store/generation/runs
CLI; tests, rules and docs that only describe those mechanisms. Preserve shared compute dependencies
for real consumers. Migrate the embedding cache afresh. Delete historical `lctx_ops`/`runs` records
and interfaces: new compilation does not write them, so rebuilding that historical service has no
current consumer. Current attempt diagnostics use structured logging and minimal publication state.

Do not import existing PostgreSQL bytes or cache rows. Fresh source chunks, vectors and indexes
come from pinned inputs. Operator-state deletion requires quiesced readers and execution authority;
this documentation session changes neither the databases nor client registrations.

## 6. Functional verification and completion

During implementation use touched-crate compile checks and the explicit affected `verify-*`
family/filter. Replace obsolete PG controls with small controls for the new actual boundary;
do not port every receipt/grant test or create test-only copies of semantic validators.

Independent known-answer controls challenge assertion identity/multiplicity, n-ary roles, scope,
provider uncertainty, same-context joint support, full-content roundtrip, original bytes, duplicate
search contributions/ties, partial evidence and publication interruption. Actual persistent-store
tests cover durability, index availability, write/error handling and sealed definitions. Real MCP
listing/calls and generated CPython oracles challenge served meaning. Mem-only checks do not
establish persistent-server or transport behavior.

At Q0 run the affected families and one assembled `just qualify` for this shared-contract pivot,
with the family recipes changed to the target prerequisites. Run applicable non-functional leaves
at scope end; root runs `just turn-end` last. Repair failures and rerun affected boundaries; report
composite outcomes honestly. No broad gate after every slice. Scope-end documentation-only checks
are independent of product qualification.

Q1 exercises fresh FastMCP catalog first, then the behavioral profile and explicitly requested
optional methods, plus discovery → packet → original evidence and restart/pinning/retirement.
Fake vectors can check their seam but cannot establish live embedding usability. Product benefit,
Context7 comparison, heldout confirmation and broad superiority remain their own product protocol;
they do not gate this architectural replacement or feed compiler inputs.

Complete the implementation at Q0 only when all required packages and deletion obligations are
closed; describe operator adoption as pending until Q1 is authorized and completed. Do not call the
whole operational pivot complete while required fresh reconstruction is pending. No Measured
performance claim follows from functional acceptance or source-level simplification.

## 7. Sole finding disposition and optional capabilities

IDs below are qualified by their source review to avoid the distinct meanings of F01. Compiler
portions are implementation-closed at the user-accepted stage boundary; native persistence and
serving obligations remain open. Scoped tests do not establish the later published journeys.

| Source obligation | Owning package / completion evidence |
|---|---|
| Target GN01 — admission distinct from physical execution | M1/C1/C2 implemented and user-accepted: store-free compilation, bounded workspace and completed graph admission; §8 records scoped artifact controls. P2 faithful stored realization remains open. |
| Target GN02 — efficient connected serving | S1/S2/S3 + P1: native indexed/coarse journeys, original evidence, honest missing/partial outcomes, no broad startup preparation or unrelated relation hashing |
| Target GN03 — explicit projections | Compiler A1/A2 implemented and user-accepted: prepared topology, semantic IDs/roles, source membership and owner-derived losses; model/analytics and selected artifact controls passed (§8). Published exports remain open. |
| Capabilities F01 — fusion semantics | S2: contribution collapse, canonical ties and separate eligible occurrence/channel witnesses through the actual operation |
| Capabilities F02 — API authorization/rollback | Ordinary functions selected; custom API adoption deferred to an HTTP consumer. If triggered, owning package must enforce explicit scope and actual transactional failure, with focused endpoint controls. |
| Capabilities F03 — executable realization identity | P2 + S3: changed functions/analyzers/modules/index specs cannot silently substitute beneath a pinned handle |
| Compiler-stage review F01 — shared embedding execution | Implementation-closed: shared actual winners and bounded independent request batches; embedding/retrieval controls passed within the compiler run (§8). Source repair independently accepted. |
| Compiler-stage review F02 — duplicate retrieval replay | Implementation-closed: construction retained, narrow production completion, independent canonical replay retained in controls; retrieval controls passed (§8). Source repair independently accepted. |
| Compiler-stage review F03 — duplicate projection policy | Implementation-closed: model-owned acceptance/exclusion policy feeds artifact losses; model and projection/artifact controls passed (§8). Source repair independently accepted. |

The compiler-stage F01–F03 rows refer to the dated [implementation reassessment](../design_review/reviews/design_review_graph-native-compiler-stage_2026-10-05.md),
not an additional architectural basis. Its static judgment accepts those source repairs; the user
accepted stage completion with partial verification.

Earlier F01–F14 referenced by the named reviews are not a separate PostgreSQL repair queue.
Their surviving causes expressly carried by GN01/GN02, together with the preserved product
guarantees in these two reviews, are scheduled above. Earlier implementation-specific remedies
are superseded by G0. This series neither imports another review's architecture nor claims
unexamined historical findings are verified closed. Existing Q1/product obligations retain their
current owners until G0 transfers or explicitly defers them; previous receipts are not new evidence.

Adopt now: native graph/documents, strict typed envelopes, functions, exact-symbol/full-text/vector
search, native query composition, bulk/error/index readiness, simple telemetry and petgraph export.
Design around: bitmap filtering, appropriate INLINE fields/caches, streaming and alternative vector
index residency. Rust/WASM gets a bounded optional kernel assessment in S2; no full-model port.
Buckets, custom APIs/GraphQL, reactive enrichment, connectors, external exporters and distributed
deployment are consumer-triggered. No canonical lightweight edges replace attributed assertions.

## 8. Current checkpoint

**Complete compiler stage: complete / user-accepted, 2026-10-05.** The user explicitly directed
that tests stop and this plan scope be considered complete. This closes M1, C1, C2-N, C2-U,
compiler-side A1/A2 and their G0/S1 interfaces as Implemented, with scoped Tested evidence below.
It does not claim every check passed, complete-stage Tested assurance or Measured performance.

`cpg-core` is the sole store-free compiler; `lctx-model` owns finite graph types, nominal references,
participant roles, semantic policies and graph admission. Immutable completed Arrow IPC inputs
replace store reads. Ordinary output completion is atomic; explicit vocabulary contributions merge
at their owner. One charged spillable DataFusion runtime serves workspace consumers. Canonical
family ordering, bulk reference/span closure, original bytes and exact manifest settings/outcomes,
projection definitions and consumed vector values are integrated. Summary preparation shares
indexed immutable inputs and condition preparation without replaying every producer in production.

`lctx compile --artifact-only --output DIR` exports admitted artifacts. Ordinary compile reports
unavailable before acquisition until the native publisher exists. PostgreSQL backend/binding,
generation/store/query commands and old serving effects are retired; MCP remains unavailable
until native serving. The broad CLI model-description snapshot control and its obsolete baseline
are retired for this hard pivot; the actual model-description tool and focused model controls remain.

Current verification, 2026-10-05, on production baseline `b964e807` (later changes are documentation,
snapshot-control retirement and the test-only unrequested-Flow repair `e9bcdd4a`):

- `cargo check --locked -p cpg-core -p lctx-model -p lctx --tests`: **passed** (33.22s).
- `just ready`: **passed**, native adapters synchronized before tests.
- `just verify-model`: **passed**, 648 controls.
- `just verify-analytics`: **passed**, 14 controls.
- `just verify-compiler --command producer`: **failed / interrupted at the user's request**;
  188 controls ran, 186 passed, one obsolete test premise failed and one Summary control received
  SIGTERM. Passing controls include all 98 named native fixtures with both profiles, admitted graph
  roundtrips at all four frontiers with both profiles, original/integrity controls, selected analytic
  memberships/provenance/losses, embeddings and retrieval. The Catalog test incorrectly assumed
  a precreated Flow table for explicitly NotRequested Flow; `e9bcdd4a` repairs the assertion to use
  absent completed streams and explicit coverage. Repair verification is **not_run**, by user direction.
  The Summary control was stopped, not an observed production failure; its final-baseline completion
  remains unverified. Logs: `/tmp/graph-stage-{model,analytics,compiler-producer}-assembled.log`.
- `just verify-compiler --command cli` and
  `just verify-providers --command extract -- --test summary_consequences`: **not_run**; their queued
  processes were stopped before execution. Subsequent `just verify-providers --command flow`,
  `just verify-oracles` and `just verify-tooling`: **not_run**, the orchestrator was stopped before launch.
- Scope-end non-functional leaves and `just qualify`: **not_run**, further verification waived by
  the user's stop/completion instruction. Root `just turn-end` is final repository bookkeeping,
  not additional functional verification; STATUS records its outcome.
- Native persistent-store/MCP journeys, real-library/live vectors, operator activation and Measured
  performance: **not_run**. They belong to later packages/Q0 or separately authorized Q1.

Next: P1/P2 native realization/publication and S2/S3 serving consume the completed admitted-artifact
boundary; published projection exports and remaining I1 integration follow those consumers. Q0
owns assembled acceptance after their integration; Q1 owns separately authorized fresh operator
adoption. No further compiler-stage test or legacy snapshot work is scheduled by this completion.
