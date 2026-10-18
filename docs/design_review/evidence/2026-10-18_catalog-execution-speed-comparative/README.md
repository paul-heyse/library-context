# Catalog execution speed: comparative review evidence

**2026-10-18.** Consumer: [principal review](../../reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md).
This folder contains source/retained-evidence assessment, not a benchmark campaign, remediation
plan or second disposition ledger. The [recommendation crosswalk](recommendations.md) covers the
complete external recommendation set. [Trace method](trace-method.md) documents the read-only parse.

## Inputs, baseline and roles

Current source baseline: `b0dd321eb78fa5ca90c5107e887145f85f774b52`; both external reviews name it.
No tracked dirty source at start; the two supplied external inputs were untracked operator files
and remain untouched. Their SHA256s, observed2026-10-18:

| Input | SHA256 |
|---|---|
| `docs/external_review_execution_speed_and_diagnostics.md` (396 lines) | `76b5cfd4e8ffdd56efee89031bc01aa5c4feb8078dd32eb45fbc2c8297cfddf3` |
| `docs/catalog-compile-performance-review_2026-10-18.md` (220 lines) | `a34b623cdeecfb38f20b74e2cf3b9404f507cbbb7424f47b759f6ed08c56175c` |

A fresh design reviewer first inspected current source without either external review, recording
C0 root copying/late lookup, eager contextual adjacency, network effects under visibility exclusion
and repeated capture verification. It then compared all alternatives and recommended private
typed/spillable execution plus requested-frontier durability and justified retained products.
An independent trace auditor reparsed the exact raw capture and historical write-probe JUnit.
A library researcher checked exact source and Context7/primary documentation. The coordinator
inspected decisive sources and owns the integrated argument and every published file.

Criteria are core/template3.3, heuristics1.0 and code-intelligence1.5 as declared by
`docs/design_review/design_principles/standard.toml`. Repository policies/ADRs describe the
incumbent, not constraints on the target judgment. Rule changes are proposed only.

## Retained fixture capture

Managed parent: `20261018T044128.645Z-1d2be6`.
Nested invocation: `b653540a2b744e58a4c8cc85df3ce91a`.
Native nextest run: `d00ab26d-7e78-4b2d-b2f0-9178dd814c14`.

Authoritative local inputs:

- `build/runs/20261018T044128.645Z-1d2be6/record.json`
- `build/runs/20261018T044128.645Z-1d2be6/verify-evidence/b653540a2b744e58a4c8cc85df3ce91a/native.log`
- The same invocation's `junit.xml`, `invocation.json` and `nextest-capture.json`.
- `verify-evidence/contexts/b7f06d6d7e6cfd0976727d7fe61f58016982a56586a13dbe462c1a19447a891d.json`.

Historical helper source is available with:

```sh
git show f1b83562:crates/lctx-publisher/tests/fixtures/publication_expectations.rs
```

It writes one function `discover`, disables optional techniques, compiles through Catalog, joins
retention, freezes native content and attempts raw export. It performs neither full semantic
admission nor public publication. Current direct publication controls were subsequently restored;
this withdrawn helper is evidence, not a recommended testing prerequisite.

The source-context JSON states `source_comparison: not established; invocation/configuration only`
and `source_to_binary: unknown`. An executable digest observed at completion is not immutable
executed-source/build attestation. Commit chronology and historical helper source identify the
intended earlier setup, not an independently proven exact binary closure.

| Phase | UTC interval on2026-10-18 | Actual outcome |
|---|---|---|
| `cn02_compile` |04:41:33.048256–04:46:39.731611|passed306.683356s|
| Workspace child/completion fence |04:46:39.731730–04:46:46.770713|passed7.038983s|
| Native freeze |04:46:46.770721–04:46:48.402144|passed1.631423s|
| Raw export |04:46:48.403235–04:47:33.646819|failed after45.243585s following test timeout|
| Selected test |Whole360.836753s|failed: timeout; no completed export/publication assertion|

Raw log anchors: compile24/67478, Workspace fence67479/67580, native freeze67581/67588,
raw export67589/69640. There are36,696,404 bytes and69,632 `lctx_phase` target lines.

### Reproduced counts and scopes

| Category | Whole helper | Completed before compile end | Meaning |
|---|---|---|---|
| Legacy `native_effect_apply` |2,481;111.146970s sum|2,280;104.512630s sum|Legacy wrapper intervals, not all native pages/transactions|
| Legacy recovery settlement |2,481;52.793290s sum|2,280;45.930916s sum|Nested lifecycle intervals, not additive with applies|
| Immutable windows |1,785;15,498 physical rows|No throughput inference|Median1;1,376 windows≤4 physical records|
| Logical batch submissions |527 phases;5,986 rows;289 relation names|Catalog producer work|Not distinct final canonical-row cardinality|
| Small submitted relations |244 names≤10 rows|Same submission grouping|Not proof of constant cost or FastMCP scaling|
| Completion-gate waits |47;sum80.527595s;median1.578588s|Wall union28.636295s|Neither sum nor union is measured removable latency|
| Retention |15;sum149.878192s|14 ended;sum111.936243s|Last retention overlaps frontier and outlives compile|

Retentions clipped to the compile interval have sum142.840014s and union136.282358s. These
are occupancy observations, not causal savings. Retrieval retention runs at275.848954–313.790906s
relative to the first event. The Catalog-frontier stage287.152340–306.747181s runs during it;
compile ends306.752709s. The final Workspace fence joins the already accepted child afterward.
This positively refutes blanket claims that every dependent stage waits for retention.

There are34 instrumented `compiler_producer` intervals. Their sum is312.954154s; their wall
union is165.766252s. For each stage, union its own-context native-call admission/acknowledgement
and completion-gate intervals, then sum the stage results:296.016963s. Residual16.937192s.
Approximately94.6% is a fraction of **summed stage context elapsed**, not of end-to-end CPU/DB
wall time. `native_calls.rs:64–113` wraps arbitrary owned futures, including client computation,
validation and scheduling. Absence of a category in the residual is not a CPU measurement.

The reported1.34 concurrency is111.146970s/82.643390s union=1.344898 for whole-helper
**legacy effect applies only**. Compile-only legacy ratio is approximately1.375. Successful
operation pages are omitted by that vocabulary; neither number characterizes all native writes.

### Conflict calls

91 logged retry terminals all committed, reporting446 retries.58 calls are `operation-page`,
30 `apply`,2 `operation-root`,1 `outcome_reference_settlement`. Full-call elapsed sum117.464s;
interval union65.697587s. Each includes attempted/successful execution, handling and backoff;
neither is an incremental conflict penalty.

| Producer / protocol | Relative interval | Retries | Full call elapsed |
|---|---|---|---|
| Native inventory / operation-page |49.088–83.147s|98|34.059s|
| Normalize relations / apply |64.910–83.366s|65|18.456s|
| Analytic text / apply |65.781–83.063s|58|17.282s|

These three sum69.797s but union34.278174s. During49.088–83.063s there are493 successful
immutable-window,85 batch-write and459 legacy-effect terminal phases. Expensive retrying
operations are real; global frozen execution or mutual livelock is not demonstrated. Logs identify
query shapes, not conflicting keys. Conflict observations are sampled at powers of two; terminal
retry counts supply full counts. Retry issuance/backoff observes cancellation and never aborts an
already issued effect simply to report a timeout.

Actual source distinction: `immutable_window.rs:675–680` dispatches to operation pages when a
writer is supplied. `compiler.rs:5780–5830` supplies one for ordinary contributions. Successful
`operation_pages.rs:187–248` submits the page transaction and advances its acknowledged prefix;
it does not run the per-page legacy apply/settlement wrapper. That wrapper remains the fallback
at`immutable_window.rs:682–704`; legacy phase emission is in`authorization.rs:979–1004`.
A count of those events therefore cannot prove total submissions or transactions.

## Historical write probe

Reparsed testcase:
`build/runs/20261017T211603.926Z-68c73f/verify-evidence/9fc2ffcec9e54e8fb72a513c608cd4b1/junit.xml`.
Test name:`native_summary_pair_outcome_batch_has_committed_window_receipts`.
The original command is recorded in the [retained-execution notes](../2026-10-17_catalog-publication-performance/retained-execution.md).

Passed15.714s. Ingress15.152374s; logical lowering0.109045s;73 immutable windows14.817988s;
84 effect applies12.697163s;84 settlements1.496342s. These intervals overlap. Physical windows
are32×128+40×102+1×16=8,192 records for4,096 synthetic logical rows.12.697/4,096≈3.1ms
is a descriptive ratio including lifecycle effects, not a current marginal cost or FastMCP forecast.

## Pinned capability evidence

Inspected locked families: Arrow59.3.0, DataFusion55.1.0, tracing0.1.44/subscriber0.3.23,
Pyrefly fork`b8a668da`, vendored SurrealDB3.3.0. Library skills' exact source contracts were checked;
Context7 resolve→query covered SurrealDB, DataFusion, Perfetto and tracing, with primary rustdoc
fallback for tracing-chrome. No dependency was added/bumped and no diagnostic integration compiled.

Decisive locations include:

- SDK timing:`third_party/surrealdb/src/engine/remote/grpc.rs:1702–1707,2917–2922`;
  ordinary/admitted query routes1091–1183. Present valid duration is mapped; absence/invalidity is
  zero-defaulted. Transport presence must survive before this conversion.
- Native readers:`reader.rs:451–453,3735–3747,3800–3802,3913–3915` discard or omit those statistics.
  Existing typed read observations at3455–3494 differ from Phase events.
- Actual Phase context and pairing:`lctx-native/src/phase.rs:132–198`; originating dispatcher
  propagation:`lctx-tracing/src/lib.rs:14–53`. JSON adapter span context already exists
  at`scripts/verify_evidence/runtime_adapter.py:29–45`.
- DataFusion55.1 `DataFrame::execute_stream` constructs its physical plan internally; metrics
  must belong to that exact executed plan. Provider wrapper forwards inner metrics
  (`compiler_provider.rs:576–577`). Shared provider identity is not shared result execution.
- Exact Pyrefly fork:`state/state.rs:615–677,778–800`, `state/subscriber.rs:54–80`.
  Subscriber hooks and demand counters exist; timing accessor is private. CLI timing mode
  changes module execution (`commands/check.rs:317–319,532`).
- Existing server diagnostics and recorder probes are the two2026-10-15 folders linked below.
  Their memory-engine/synthetic observations are not new RocksDB or production overhead evidence.

[Server diagnostic evidence](../2026-10-15_operation-diagnostics-surrealdb/README.md),
[instrumentation evidence](../2026-10-15_operation-diagnostics-instrumentation/README.md).
Primary library links and exact implications are in the principal review and crosswalk D01–D26.

## Outcomes and preservation

**passed:** read-only retained-log/JUnit parsing and source/contract inspection completed2026-10-18.
Documented trace method rerun `20261018T061845.504Z-e2c63d` passed with confirmed host cleanup and reproduced the stated stage/effect/retry arithmetic. These are successful investigative operations, not new product test passes. No mocked provider was used.
**not_run:** new product tests/builds, full FastMCP, benchmarks, server queries, runtime instrumentation,
profile/overhead comparisons, schema regeneration or operator adoption. Those require no invented
passing status. Whole publication `just docs-check` run `20261018T061845.682Z-19ce6f` failed before rendering because the supplied E1 input has no H1 title. It remains unchanged. Scoped source-document check `20261018T062027.929Z-cde2f9` passed for six owned Markdown documents,124 existing local file targets,76 recommendation anchors,13 findings and9 rule impacts; this does not claim rendered HTML or implicit-fragment validation.

Cited run/native objects are retained for this report through the existing storage owner under
`citation:catalog-execution-speed-comparative-2026-10-18`. Original failures/Unknowns, host cleanup
and authenticated earlier campaign disposal keep their original meanings. Fourteen cited run/native objects were retained. No raw capture was copied,
changed or deleted, and no campaign/service was created or disturbed.
