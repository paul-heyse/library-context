# Implementation friction: review evidence

**Inspected2026-10-14.** Supporting evidence for the [principal review](../../reviews/design_review_implementation-friction-target_2026-10-14.md). This folder owns a compact incident/reference index, not a second verdict or execution ledger.

**Current consumer, 2026-10-14:** the [Catalog completion coordinator §6](../../../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint)
owns scheduled F01–F10/O01–O05 and their closure obligations together with surviving Catalog scope.
Plan publication changes neither these historical observations nor the evidence owners/retention.

## Baseline and method

Clean source baseline: 5ffdca70439ac7474f90ebc639453dfc55dd2536. Relevant earlier checkpoints are e54f03cd and426f3892. The examined implementation window is approximately17:00–20:14UTC on2026-10-14. Commit timestamps alone do not describe when source was edited or which binary a run executed.

The root and three independent reviewers inspected current source, current architectural owners, the Catalog coordinator, its integrated review, retained run records/output, and timestamp-filtered Codex transcripts. The root read decisive evidence and reconciled the findings. Reviewers independently covered production execution/finality, production allocation/preparation and development tooling. The active root and all three reviewers were observed as gpt-6.1-sol in their turn-context metadata; repository defaults were not used as proof of actual execution model.

The principal review uses core/template3.3, heuristics1.0 and code-intelligence1.5. Current rules/ADRs explain the implemented design but do not constrain the target judgment.

New product tests/builds, process probes, database queries/campaigns, full Catalog/FastMCP journeys, embedding/MCP runtime and benchmarks were **not_run**. Source and historical observations establish the reported defects without replaying those expensive operations. Proposed alternative containment and remediation controls remain unexecuted.

## Raw evidence and retention

Run records and logs remain under the existing managed run/storage owner in the repository's build/runs directory. Forty-one selected runs, comprising82 receipt/log objects, received the additional consumer **citation:implementation-friction-review**, requiring **cited-report**. All retention commands succeeded; no original consumer was released and no raw output was duplicated into Git.

The selected suffixes are:

a0fc21, b94680, 5a163f, 9692d8, 16b979, 617508, 0f3e99, 250b19, 4eca97, 2675d5, 19cbd7, 7a1f0d, bab4b8, 1069dd, d878b2, 1e738a, 23b992, e7062d, cb8f68, dcb999, d7d2d2, d58994, a4a6ef, a09702, bd7117, ad90ec, b847e8, 989868, fd8c02, f44711, cd0497, 1a6c08, 237082, 01b376, 8e33e7, 9d6692, 8ce755, 5f86e3, 2dfe51, 324b00, 30bb8e.

Use the existing selective commands: just runs status SUFFIX --json and just runs logs SUFFIX --tail N. A suffix is only shorthand while unique; the full run ID returned by status identifies its record. Do not use an all-history JSON listing to resolve one incident.

Codex owns its session files independently. The root transcript is:

/home/paul/.codex/sessions/2026/10/11/rollout-2026-10-11T06-55-16-01a12a9a-48ac-7ea2-bb5e-c8e0b59f804d.jsonl

Streaming inspection filtered timestamps before projecting fields. No credentials, full configuration, encrypted agent messages or unrelated session content was copied. The window contained2484 event messages,1790 response items and four compaction records; these are observed record counts, not a measurement of wasted reasoning or evidence of one causal source.

Historical implementation-review threads were located through their parent/thread metadata. Useful source reports came from compiler_controls_finalization, publication_expectations_execute and catalog_remaining_integrated_design. Their earlier judgments were checked against the actual final source. The current independent reviewers returned source evidence directly; their temporary identifiers were reconciled into principal F01–F10.

## Incident index

| Incident | Actual observation / source limit | Review relevance |
|---|---|---|
| ENOSPC / cold build | a0fc21 began17:01:35, ended17:04:07UTC after a rustc code-generation panic and “No space left on device”; coordinator cancellation ended SIGTERM with direct cleanup confirmed. No tests ran. | Infrastructure failure and cold-compilation context; not an analyzer/runtime diagnosis or authority for cache deletion |
| Type/owner proof friction | b94680 build passed with a future-incompatible Send-proof-depth warning;5a163f rebuilt without it after identity-only owner links changed. | Future/ownership type composition merits explicit boundaries; no evidence of a strong runtime retention cycle solely from the compiler warning |
| Wrong CLI route | 9692d8 failed2 at17:30:51 for service dev;16b979 corrected surreal-dev inspection and passed. Root session call line68139. | Agent misuse; existing parser/help were available |
| Planner evidence | 16b979 showed job-only access with filtering/sort;617508 demonstrated exact-state indexed head scans. | Credit completion-integrated#F01's indexed correction; no fresh query in this review |
| Missing native test setup | 0f3e99 ran6 controls,5 bodies passed and one failed at frozen native owner handoff; setup omitted real persisted bindings. Native Unknown was retained despite host cleanup. | Test-construction failure must not be reported as weakened production validity; exact binding setup repaired separately |
| First Summary timeout | 250b19 ran availability and Summary: one passed, one timed out360s. | An availability pass does not establish Summary/Catalog completion |
| Cold assertions versus cleanup | 4eca97 failed after862.378s; the coordinator/source trace identifies final retirement native10s timeout after the cold assertions progressed. | Body, cleanup and native finality are different outcomes; no passing cold-control claim for this run |
| Focused union/finality controls | 2675d5 native6 passed1.218s;19cbd7 compiler7 passed2.158s; enclosing fixtures0/confirmed cleanup. | Supports prior bounded F02/F03 corrections, not every dispatcher composition |
| Locked query qualification | 7a1f0d actual native3 failed on array-valued FOR UPDATE;bab4b8 scalar lowering actual/pure6 passed1.015s. | Preserve actual pinned-engine semantics and all-page locks/prechecks |
| Final cold control | 1069dd passed its single full cold/original/cleanup control467.636s. | Positive evidence for bounded F04; not a benchmark or later F07 runtime pass |
| Memory failure before attribution | d878b2 Summary failed580.287s at a shared1GiB refusal. | The failing requested allocation alone does not identify the dominant reservation |
| Memory attribution | 1e738a Summary failed454.259s; output lines629–642 report installed inventory507.1MiB, total admitted1018.9MiB/1024MiB and requested5.8MiB. native_product_retention failed; actual finality was terminal/confirmed. | Reservation attribution, not RSS; optional capture/current memory routes require independent review |
| Linked Catalog stall | 23b992 timed out910.035s including grace. Local/Summary/Models/Completion registration IDs389/391/393/395 passed; Enriched396 began without its own registration. Later pass370 belongs to NativeInventory. | Exact phase pairing narrows the path; historical blocked read/cycle is not fully observed |
| Mutation controls | e7062d timed out both controls360s plus grace; maintenance's validation drain failed and left admission closed. | No successful mutation/refusal assertions; local phase diagnostics were subsequently added but final rerun did not execute |
| Successor control setup | cb8f68 failed because fresh final retirement ran with admission closed after successor assertions. dcb999 passed0.720s after explicit opening for fresh teardown under the exclusive owner. | Production fences were right; final teardown setup was wrong |
| completion-integrated#F06 controls | d7d2d2 actual native2 passed5.805s, enclosing0/cleanup confirmed. | Overlapping closure lifetime/cold identity/late refusal evidence at its actual source |
| completion-integrated#F07 preliminary controls | d58994 selected four controls passed3.534s; binary predates final parent-hold/sort refinement. | Bounded runtime credit only |
| Final source checks | a4a6ef final metadata passed; a09702 affected all-target keep-going Clippy with -D warnings passed. | Compile/lint evidence, not final runtime acceptance |
| Nested cancellation | bd7117 outer cleanup confirmed19:10:58 while fixture compiler group remained visible19:12:17; ad90ec local recovery refused; group later drained and b847e8 retry passed19:13:40. | Parent/delegated host composition gap; exact signal interruption remains unproven |
| Final cancellation | 989868/1a6c08/fd8c02/f44711/cd0497 cancelled during compilation before assertions; managed cleanup confirmed. Three fixture groups outlived their leaders briefly. | Interrupted, not passed; direct cleanup cannot release inherited log obligations by association |
| Exact local/native recovery | 237082/8e33e7/9d6692 initially refused unresolved command cleanup;8ce755/5f86e3/2dfe51 passed after natural drainage.01b376 confirmed local cleanup but refused native Unknown. | Conservative recovery is correct; native completion is not process disappearance |
| Whole-target disposal | 324b00 passed close-admission, native drain, authenticated current quiescence, executor shutdown and exact datastore/attachment/installer/unit/storage/credential release. | Settles the owned R7 lifecycle, never rewrites original cancelled outcomes |
| Oversized discovery | Root session line68945 at18:17:01 invoked runs list --json; following result was enormous/truncated. | Poor agent choice plus no bounded supported discovery route |
| Status spelling | Root session line72185 invoked service status --json; parser refused the unsupported flag. | Agent misuse; status already emits structured output |
| Documentation check | 30bb8e failed before publication on existing14 ADR reference defects plus stale index. | Carry forward; not repaired or treated as product failure by this review |

All test durations above come from historical Nextest summaries and their actual conditions. They exclude some compilation/waiting time and cannot be added as independent phase costs or compared as controlled speed measurements.

## Exact cancellation correspondence

| Cancelled command | Fixture attachment / initial local recovery / later result |
|---|---|
| 989868 final native controls | 426dea80-2bd6-4a6f-ac3d-5e2c97baaa01;01b376 confirmed local cleanup, refused native Unknown |
| fd8c02 Summary | 356cca46-debf-4f9c-94e6-68937c26e000;237082 refused,8ce755 passed after drainage |
| f44711 linked Catalog | d5bd5754-6fd5-4667-bcd4-64f6a9f2e49c;8e33e7 refused,5f86e3 passed after drainage |
| cd0497 cold | e8b15cfe-3a9a-40d0-a995-87b4abf44985;9d6692 refused,2dfe51 passed after drainage |

The R7 installation was18ad7757-24f0-44ff-994a-332ad67d1fa7, generationf4bf35bfb3852cfb9cbb30d03030282d08508c115518a03689e1946f6bfc214a. Its exact disposal is324b00. Shared/operator installations and protected earlier attachments were not inspected or altered.

Root-session lines70615/70627 record bd7117 cancellation/direct status;70706 records live descendants;70714 records unavailable group-leader identity;70764 records later drainage. Those plain command/results support the finding. Counts or encrypted send_message payloads do not establish the claimed exact idle-agent delay; the principal treats that as reported agent-use friction.

## Library and environment qualification

- Cargo.lock and the selected offline skill match DataFusion55.1.0 and Arrow59.3.0. The exact local datafusion-execution55.1 memory_pool source was inspected: MemoryReservation offers fallible growth/resize, shrink and split; TrackConsumersPool.report_top returns formatted text. No structured top-consumer-vector API or RSS measurement was inferred.
- Native PreparedRows/PreparedCandidates supply shared immutable backing, cursors and spill. Existing native view/audit factories already demonstrate separate typed decoding and retained backing; their use is the local alternative, not a new memory framework.
- SurrealDB actual historical controls refer to patched3.3.0 generation405f6c02; current scalar-lock/index code was inspected. No assumption that ordinary array SELECT establishes array FOR UPDATE support was transferred.
- Installed systemd-run reports255.4 with unified cgroups. Installed service/kill manuals and official version255 sources support ExitType=cgroup and KillMode=control-group. Context7's261.3 index was a discovery lead only. No transient unit was started.
- Official Codex subagent/config documentation was fetched through Context7 and opened from OpenAI's documentation. Model and role/default overrides can compete with inheritance; actual thread metadata is separate evidence. No configuration was changed.

Primary external references consumed:

- [Codex subagents and override order](https://learn.chatgpt.com/docs/agent-configuration/subagents).
- [Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference).
- [systemd255 service lifecycle](https://raw.githubusercontent.com/systemd/systemd/v255/man/systemd.service.xml).
- [systemd255 group termination](https://raw.githubusercontent.com/systemd/systemd/v255/man/systemd.kill.xml).

## Coverage limits

Scope-end documentation checks,2026-10-14: an inline Python relative-link/explicit-anchor inspection of the two new Markdown files checked47 references with0 failures. This is a focused navigation check, not publication qualification. just docs-check under managed run20261014T205646.783Z-8d8a71 failed before publication on the same14 pre-existing architectural ADR-reference defects plus stale index seen in30bb8e; cleanup was confirmed. Its two receipt/log objects also retain citation:implementation-friction-review/cited-report. No unrelated metadata repair was attempted.

Source searches followed actual owners and callers through cpg-core, compiler-store, native completed/audit/retained/ordered state, model charging/normalization/Summary and host run/harness/fixture/service/evidence code. Matching syntax was not treated as a defect without its contract and consumer. The principal distinguishes real fixed manifestations, residual findings and opportunities.

All analyzer heaps, every model kernel, complete query/serving caches, engine internals, all recovery races, alternative execution-unit integration and whole-product functional acceptance were not exhaustively examined. The review does not turn those unexamined areas into findings. Detailed controls proposed in F01–F10 are closure criteria, not new test receipts.
