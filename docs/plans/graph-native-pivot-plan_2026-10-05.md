# Graph-native replacement — implementation coordinator

**Implemented with open audit findings, 2026-10-06 (ADR-0128); operator adoption remains not_run.** Replace the PostgreSQL-centered compilation and serving architecture
directly with a Rust-admitted graph and SurrealDB-native persistence, querying and search. This
coordinator owns the combined execution sequence, shared decisions, finding disposition and
completion boundary. Supporting plans develop their respective designs; they do not create
another task ledger. Package evidence below distinguishes implementation from functional acceptance and activation.

The [independent implementation audit](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md) concludes that the runtime migration is
substantially present but the complete target is not fully realized. §7 owns the twelve open
findings; existing §8 receipts and the user-accepted compiler checkpoint remain historical evidence.
This audit performed no remediation and does not authorize Q1 activation.

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
P1/P2/S2/S3, published exports and I1 are implementation-closed; targeted native acceptance
passed, with the exact boundary in §8. Operator Q1 remains **not_run**. Supporting plans supply implementation detail and focused
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
| Q0 — focused functional acceptance | Targeted actual persistent-store/native/MCP journeys and affected contract controls pass on the integrated tree, plus applicable leaves; user-directed execution excludes broad qualification and legacy suite parity. | Integrated required packages; no legacy suite retention prerequisite |
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
`lctx publish-artifact <artifact>` for consuming that admitted artifact. The compiler library
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
this execution uses owned disposable native databases and changes neither operator databases nor client registrations.

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

For this execution the user selected targeted functional controls throughout the hard pivot,
waiving assembled `just qualify`, legacy suite parity and further compiler-stage tests. Q0 uses
actual small native publication, serving, search, projection and MCP journeys, with recipes changed
to the native prerequisites. Run applicable non-functional leaves at scope end; root runs `just turn-end` last. Repair failures and rerun affected boundaries; report
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
portions retain their user-accepted stage boundary; native persistence and serving are implemented,
with their actual focused outcomes recorded in §8. The independent audit identifies additional
unmet obligations below; package implementation does not establish complete target realization. Implementation alone
does not establish a passed journey.

| Source obligation | Owning package / completion evidence |
|---|---|
| Target GN01 — admission distinct from physical execution | **Audit gaps open** (F01/F02/F03/F04/F05/F07/F11 below). Prior M1/C1/C2 implemented and user-accepted: store-free compilation, bounded workspace and completed graph admission; §8 records scoped artifact controls. P2 is implementation-closed / focused Tested: reconciled unselected publication, read-only viewer, canonical backup, fresh restore and retirement; §8 records the native lifecycle control. |
| Target GN02 — efficient connected serving | **Audit gaps open** (F02/F03/F08/F09/F10/F11 below). Prior S1/S2/S3 + P1 implementation-closed / focused Tested: actual ten-tool Catalog and MCP journeys, scoped native selection/search/packets/originals, missing/NotRequested states, cursor and deadline refusal; §8. Indexed/coarse access avoids unrelated startup preparation; no Measured performance claim. |
| Target GN03 — explicit projections | Compiler A1/A2 implemented and user-accepted: prepared topology, semantic IDs/roles, source membership and owner-derived losses; model/analytics and selected artifact controls passed (§8). Published A1 export is implementation-closed / focused Tested: actual native scope, isolates, parallel/self arcs, lineage, coverage and gaps; §8 records the control. |
| Capabilities F01 — fusion semantics | S2 implementation-closed / focused Tested: actual native search checks eligible contextual/member admission before BM25/HNSW caps and witness retention; the operation collapses contributions and uses canonical ties/RRF-K60. §8 records the actual native and Catalog controls; live embedding quality is not_run. |
| Capabilities F02 — API authorization/rollback | Ordinary functions selected; custom API adoption deferred to an HTTP consumer. If triggered, owning package must enforce explicit scope and actual transactional failure, with focused endpoint controls. |
| Capabilities F03 — executable realization identity | **Audit gaps open** (F04/F06 below). Prior P2 + S3 implementation-closed / focused Tested: sealed definition inventory/cold audit, actual analyzer-drift refusal and incompatible Rust-operation refusal; fixed viewer handles survive server restart and both MCP transports. §8. This is trusted-installer integrity, not adversarial-root assurance. |
| Compiler-stage review F01 — shared embedding execution | Implementation-closed: shared actual winners and bounded independent request batches; embedding/retrieval controls passed within the compiler run (§8). Source repair independently accepted. |
| Compiler-stage review F02 — duplicate retrieval replay | Implementation-closed: construction retained, narrow production completion, independent canonical replay retained in controls; retrieval controls passed (§8). Source repair independently accepted. |
| Compiler-stage review F03 — duplicate projection policy | Implementation-closed: model-owned acceptance/exclusion policy feeds artifact losses; model and projection/artifact controls passed (§8). Source repair independently accepted. |

### Implementation audit disposition — 2026-10-06

The [dated audit](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md) owns diagnosis/evidence; this table is the sole current disposition
owner. Every row is **open**: correction is required for complete target realization, but no
remediation was executed or authorized by the audit. The report supplies priority and compatible
correction directions; it does not create a second execution ledger. Existing accepted compiler
completion and stopped-test boundaries remain intact.

| Source finding | Disposition / cause | Responsible component | Required closure evidence |
|---|---|---|---|
| [Audit F01](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F01) | **Open** — Source-owned semantic support checks absent at actual admission | M1/C1/C2 admission | Independent unsupported/wrong-context support refuses; focused checks without blanket replay |
| [Audit F02](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F02) | **Open** — Whole-resident normalization lacks promised spill route | C2-N/preparation | Bulk/partitioned preparation preserves canonical output under smaller memory envelope |
| [Audit F03](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F03) | **Open** — Upper consumers replay completed upstream normalization | C2-U preparation | SourceCalls/Models/Summary consume shared checked immutable preparation |
| [Audit F04](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F04) | **Open** — Cold audit omits derived serving integrity | P2/realization | Read-only canonical-to-derived checks reject altered/deleted/extra query-visible rows |
| [Audit F05](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F05) | **Open** — Selection publishes two authorities separately | P2/S3 configuration | Atomic selection agrees across CLI/MCP launch/retirement under failure |
| [Audit F06](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F06) | **Open** — Executable guard misses answer-affecting helper | P2/S3 identity | Schema-preserving helper change changes guard and refuses old realization |
| [Audit F07](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F07) | **Open** — HTTP backup EOF lacks engine terminal success | P2 backup | Late export failure leaves no completed destination; normal fresh restore succeeds |
| [Audit F08](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F08) | **Open** — Scoped vocabulary uses whole-library members | S3 browsing | Two-module/class native scoped vocabulary excludes foreign values/counts |
| [Audit F09](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F09) | **Open** — Nested diagnostic cursor loses parent page | S3 continuations | Second scenario child continuation returns its remaining diagnostics |
| [Audit F10](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F10) | **Open** — Native failures bypass typed safe envelope | S3 Rust/PyO3/MCP | Recognized causes and safe messages survive actual MCP tool/resource envelopes |
| [Audit F11](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F11) | **Open** — Native expansion/crossings escape bulk boundary | P1 cache/P2 search | Bounded streamed search lowering and exact reconciled batch cache writes |
| [Audit F12](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md#F12) | **Open** — Current assurance routes retain removed PG owners | G0/I1 documentation | Current task routes point to consistent native owners/controls |

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

**Prior native execution checkpoint: implementation-closed / focused Tested, 2026-10-06.**
The subsequent implementation audit identifies open completeness/correctness/execution gaps in §7;
this receipt records the earlier exercised scope, not closure of the audit findings.
The replacement SDK, publisher, native queries/search, projection exports, CLI, PyO3 and MCP
consumers are integrated. Targeted actual native and MCP controls passed within the boundaries below.
Real-library/live-vector and operator activation remain separately authorized Q1 work.

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

`lctx compile --artifact-only --output DIR` exports admitted artifacts. Ordinary compile now admits and publishes an unselected native snapshot; explicit runtime
configuration is checked before acquisition. Native CLI and MCP readers pin the complete handle.
PostgreSQL backend/binding, generation commands and old serving effects are retired. The broad CLI model-description snapshot control and its obsolete baseline
are retired for this hard pivot; the actual model-description tool and focused model controls remain.

Preserved user-accepted compiler-stage verification, 2026-10-05, on production baseline `b964e807` (later changes are documentation,
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
- At that compiler-stage checkpoint native persistent-store/MCP journeys were **not_run**; the
  current execution owns their outcomes below. Real-library/live vectors, operator activation and
  Measured performance remain **not_run**, separately authorized Q1 or measurement work.

Current native verification consumes the completed admitted-artifact boundary without reopening
the compiler-stage suite. Q1 owns separately authorized fresh operator adoption. No further
compiler-stage test or legacy snapshot work is scheduled by this execution.

### Native execution receipt — 2026-10-05–06

P1/P2/S2/S3, published A1 exports and remaining I1 are implemented on shared main. Core native
operations and their scoped data/index routes preceded publisher and transport integration.
`lctx-surrealdb` owns remote SDK/typed codecs, sparse scope keys, enforced roles, original chunks,
exact cache winners and native materialization. `lctx-publisher` loads/reconciles admitted exports,
seals read-only realizations, and reconstructs fresh current definitions on portable restore.
`lctx-serving` owns all ten tools and retained exact semantic kernels over scoped inputs; PyO3
and FastMCP own the pinned lifecycle, bounded wire admission and draining worker shutdown.
CLI and verification recipes now use the native owners; no PostgreSQL runtime dependency remains.
The obsolete Python rule requiring pyarrow execution and banning every `.execute` call, together
with its fixtures, is retired. Native operation dispatch remains Rust-owned; Python builds no SQL.

Native runtime controls exposed SDK nullable readback, incomplete graph transport of retained
consumer inputs, an internal conflict-vocabulary request above the public selector bound, capability search
routed to the wrong retrieval family, and native packet preparation mixing
its strict inventory with other scoped packet rows. Repairs preserve validators, keys, roles
and algorithms: exact typed graph owners and scoped hydration, independent conflict batches
sharing one prepared view, the existing API/options family for authored briefs, and each native
preparation owner receiving its declared inputs. Final affected
controls below own closure; no validator was weakened to obtain a pass.

Actual focused outcomes:

- `cargo check -p lctx-surrealdb -p lctx-publisher --message-format short`: **passed**, sparse
  canonical layout and publisher compile. Earlier integrated publisher/bridge/consumer checks passed.
- `cargo test --offline -p lctx-model --test graph_contract selected_semantic_inventory_has_nominally_closed_reference_types -- --exact`:
  **passed, 2026-10-06**, stable main source after exact evidence, selection, signature, raw-packet and conditional
  native binding input owners were integrated. The control checks every declared raw packet
  source and actual selection/native/binding input against the finite graph inventory and nominal
  reference closure. Keys, codebooks, roles and validators are preserved; this is not compiler-stage
  suite parity or execution of every optional behavioral branch.
- `cargo test --release -p lctx --locked --bin lctx tests::native_commands_keep_artifact_and_selection_boundaries_explicit -- --exact`:
  **passed, 2026-10-06**, explicit command/handle/configuration/export/retirement parsing.
- `cargo test --release -p lctx --locked --test compile_artifact ordinary_compile_requires_native_configuration_before_acquisition -- --exact`:
  **passed**, actual CLI refuses missing runtime configuration before acquisition.
- `UV_NO_SYNC=1 uv run --no-sync pytest tests/scripts/test_verify.py -q`: **passed**, ten native
  verification-launcher controls. This tests orchestration, not a provider or product journey.
- `UV_NO_SYNC=1 uv run --no-sync pytest tests/scripts/test_product_evidence.py -q`: **passed**,
  independent original-source comparison baseline. Its BM25 dependency is development-only;
  product search uses native SurrealDB BM25/HNSW.
- `LCTX_SURREAL_TEST_CONFIG=<owned fixture> UV_NO_SYNC=1 cargo test --release -p lctx-publisher --test publication compiled_export_publishes_unselected_and_viewer_is_immutable -- --exact`:
  **passed, 2026-10-06**. Actual unselected publication, read-only viewer, originals, live listing,
  cold audit and analyzer-drift refusal, no-clobber canonical backup, fresh unselected restore,
  restored audit/original bytes and explicitly quiesced retirement. This is a native lifecycle
  control over an independently verified normalized fixture, not real-library activation.
- `UV_NO_SYNC=1 uv run --no-sync pytest tests/scripts/test_build_measurements.py -q`:
  **passed, 2026-10-06**, six build-measurement script controls. The explicit full measurement
  runner now owns its native fixture; no build-performance campaign was run.
- `UV_NO_SYNC=1 uv run --no-sync pytest python/lctx_mcp/tests/test_native_session.py python/lctx_mcp/tests/test_wire_contract.py python/lctx_mcp/tests/test_transport_envelope.py -q -k 'not test_native_mcp_lifespan_uses_one_pinned_viewer_snapshot'`:
  **passed, 2026-10-06**, fifteen isolated wire/schema/lifecycle controls after protocol typing repairs. Actual native MCP
  journeys have their separate receipt below.
- `UV_NO_SYNC=1 just ready`: **passed, 2026-10-06**, selected skill/tool readiness.
- `LCTX_SURREAL_TEST_CONFIG=<owned fixture> UV_NO_SYNC=1 cargo test --release -p lctx-serving --test native_search -- --nocapture`:
  **passed, 2026-10-06**, eligible contextual/member occurrences are admitted before lexical
  and HNSW candidate caps; an excluded nearer vector cannot displace its eligible witness.
  The operation refuses an incompatible sealed implementation. Known-positive BM25 fixture
  terms have independent nonmatching background documents; this checks channel semantics,
  not legacy ranking parity or live embedding quality.
- `LCTX_SURREAL_TEST_CONFIG=<owned fixture> UV_NO_SYNC=1 cargo test --release -p lctx-surrealdb --test native --test projections -- --nocapture`:
  **passed, 2026-10-05**, two actual persistent-server controls on the final sparse layout.
  SDK point/scoped/adjacency reads, strict shape refusal and sequential exact embedding-cache
  winner reuse; input/context projection scope, isolates, self-loops, parallel attributed arcs,
  semantic lineage, coverage, gaps and foreign-input exclusion. These precede later semantic graph-owner
  additions; their tested physical layout and projection policy are unchanged by those additions.
- `LCTX_SURREAL_TEST_CONFIG=<owned fixture> LCTX_RETAIN_NATIVE_FIXTURE_CONFIG=<owned viewer file> UV_NO_SYNC=1 cargo test --release -p lctx-serving --test native_journey -- --nocapture`:
  **passed, 2026-10-06**, actual Catalog compile/admission/publication and all ten native tools
  over the installed first-party synthesis-sources fixture (29.15s runtime). Scoped selection,
  browse/cursor and foreign-realization refusal, operation sections, comparison, authored capability
  search/expanded packet, source originals and inspection with actual source formals; immediate
  deadline refusal. Independent original-byte comparison remains active. Catalog-profile optional
  NotRequested/Unavailable states are preserved; this is not every behavioral branch or real FastMCP.
- `UV_NO_SYNC=1 uv build --package lctx-semantics --wheel --out-dir <owned wheel directory>` and
  `UV_NO_SYNC=1 uv pip install --python .venv/bin/python --no-deps --reinstall <current wheel>`:
  **passed, 2026-10-06**. Built and installed the matching NativeSession wheel after native workers
  drained; source baseline `816fdc82` supplied the actual served semantic implementation.
- `python3 <owned fixture>/runtime.py restart`: **passed, 2026-10-06**. Restarted the persistent
  SurrealDB 3.3 fixture before exercising MCP; no operator service was touched.
- `LCTX_NATIVE_SERVING_CONFIG=<owned viewer file> LCTX_NATIVE_TEST_LIBRARY=synthesis-sources UV_NO_SYNC=1 uv run --no-sync pytest python/lctx_mcp/tests/test_native_session.py -q -k test_native_mcp_lifespan_uses_one_pinned_viewer_snapshot`:
  **passed, 2026-10-06**, two actual Rust/SDK/native-server MCP journeys after restart, in-process
  and stdio. Ten-tool inventory, pinned viewer, lexical search, expanded capability packet/resource
  Markdown and attributed metadata, strict invalid-request refusal, draining close and reopening.
  Latest Python baseline `3c8bc590`; actual native fixture, no mocked provider or live Qwen service.
- `UV_NO_SYNC=1 cargo test --release -p lctx-serving --lib`:
  **passed, 2026-10-06**, three affected pure controls: exact diagnostic correlation and two
  pagination controls. These do not substitute for the actual native journey above.

Scope-end leaves, **passed, 2026-10-06**:

- `UV_NO_SYNC=1 cargo clippy --release -p lctx-model -p lctx-surrealdb -p lctx-publisher -p lctx-serving -p lctx-semantics -p lctx --all-targets --keep-going -- -D warnings`
  after applying `scripts/build_environment.py --shell`: scoped all-target compile/lint checks,
  including their actual compiler/provider dependencies, on formatted source `599010a0`.
- `UV_NO_SYNC=1 just ruff`, `UV_NO_SYNC=1 just types`, `UV_NO_SYNC=1 just deps`:
  current formatting, Python types, nominal families/forks, unused dependencies and generated
  feature union. Shared Catalog fixture imports are explicit development dependencies with narrow
  scanner exceptions. Hakari's owning exclusions keep all native libraries outside the CLI union.
- `UV_NO_SYNC=1 just adr-lint`, `UV_NO_SYNC=1 just lint-agents`, `UV_NO_SYNC=1 just rules-scan`,
  `UV_NO_SYNC=1 just rules-test`, `UV_NO_SYNC=1 just docs-check`: governing references,
  instructions, retained rules and publication links. The obsolete Python execution rule and
  dead current-tree retrieval links are retired; dated reviews keep their original conclusions.
- `UV_NO_SYNC=1 just ready`, `UV_NO_SYNC=1 just build-features`, `UV_NO_SYNC=1 just turn-end`:
  selected tool/skill readiness and generated/formatting bookkeeping. Generator inspection exposed
  the missing native-library exclusions; its owner was repaired and generated/dependency checks
  rerun. No operator store or client registration was inspected or activated.

Initial lint/style, protocol typing, dependency and governing-reference failures were repaired.
Later compile/lint repairs and formatting preserve keys, validators, functional expectations,
queries, limits and the existing explicit source-byte identity policy. Native runtime receipts
remain attributed to their exercised baselines above; raw source changes naturally require a
fresh realization and matching wheel. No source normalization or compatibility reader was added.
The final `UV_NO_SYNC=1 uv build --package lctx-semantics --wheel --out-dir <owned directory>`
and `UV_NO_SYNC=1 uv pip install --python .venv/bin/python --no-deps --reinstall <final wheel>`
**passed, 2026-10-06**, after formatting, on production source `599010a0`. Importing NativeSession
from the installed wheel with `UV_NO_SYNC=1 uv run --no-sync python -c 'from lctx_semantics import NativeSession; print(NativeSession.__name__)'`
**passed**. This replaces the earlier installed wheel with the final linked definition; no old
fixture was reactivated and no additional compiler suite or real-library pilot was run.
Logs are `/tmp/graph-native-clippy-formatted.log`, `/tmp/graph-native-types-formatted.log`,
`/tmp/graph-native-deps-final-passed.log`, `/tmp/graph-native-ruff-rechecked.log`,
`/tmp/graph-native-docs-final-receipt.log`, `/tmp/graph-native-wheel-final-formatted.log`,
`/tmp/graph-native-wheel-final-install.log` and the focused native/MCP logs.
Owned fixture/container, credentials, probe/wheel scratch and the integrated MCP worktree are
removed. Unrelated worktrees and shared Cargo caches are preserved.

Owned persistent Docker databases exercised actual authentication and query paths. Live Qwen inference, real FastMCP acquisition/compilation,
operator activation, broad `just qualify`, legacy suite parity and performance measurement are
**not_run** under this user-selected execution scope. Compiler-stage tests remain stopped.
