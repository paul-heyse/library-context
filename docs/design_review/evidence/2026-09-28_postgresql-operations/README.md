# PostgreSQL PG16–PG17 deployment and recovery evidence

Dated 2026-09-28. [PG16/PG17](../../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream)
owns execution; [forward §6.1](../../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
owns findings. ADR-0070 binds physical ANN admission, mixed routing and complete recovery.
Stage 3 semantic acceptance and arbitrary remote endpoint attestation are excluded.

## Environment and preservation

All builds use the root editable uv environment and existing release Cargo target. No wheels,
new database drivers, alternate migration owner or dependency pins were introduced. PG18.6 and
pgvector 0.8.6 run in an isolated pinned container for functional/assembled qualification.
The existing operator schema004 dump, binary, native extensions, Python sources, manifests and
protected configurations were preserved in `build/postgresql-pg16-baseline-9471b93/` before edits.
A separately restored schema007 database was upgraded with the retained binary, imported and
served through the retained native bindings, then dumped as `schema007.dump`. Both this pair and
`build/postgresql-pg8-baseline-6403b60/` remain rollback assets. No retained ready data is deleted.

## Focused functional controls

| Command/selector | Outcome and boundary |
|---|---|
| `cargo test --release -p lctx-postgres exact_identity_and_mixed_boundaries --lib --locked` | **passed:** exact format-1 identity, brief/count/selectivity/class routing boundaries |
| `cargo test --release -p lctx-postgres --test serving physical_admission_reindex_and_artifact_relocation -- --ignored --nocapture` with `LCTX_TEST_PROJECTION` | **passed:** actual migration008, stale physical admission, explicit selection/pin rejection, relocated/corrupt/missing artifacts and unavailable/incompatible diagnostic paths. Synthetic measurement documents challenge the finite SQL transition and lifecycle only; it is not ANN quality evidence |
| `uv run pytest -q python/lctx_mcp/tests/test_recovery.py` | **passed:** populated dump, logical-root hashes, non-UTC source, restored sequences/event writer, artifact relocation, native/MCP lifecycle, corrupt/incomplete/omitted inventory rejection |
| `uv run pytest -q python/lctx_mcp/tests/test_qualification.py::test_mixed_calibration_preserves_exact_controls` | **passed:** format-2 independent reference, both count-floor candidates retain exact controls, no candidate/admission fabricated from tiny data |
| `postgres_workload.py` over two retained fake pilot generations, `--embedder none` | **passed:** two stdio servers, cursor continuity after selection, cross-generation cursor refusal, concurrency1/4 and repeated hydration transport costs |
| `postgres_backup.py backup/restore-drill` over two retained ready pilot generations | **passed on repeat:** both generations served, captured non-last selected generation retained as exact; 11.86 seconds including verification and serving. Initial subprocess failure retained; readiness now waits on TCP instead of the initialization-only Unix socket |

The bounded review found and corrected recovery selection, receipt closure, timezone serialization,
qualification role/timing, diagnostic privilege/identity, foreign qualification membership and RTO
measurement defects. New receipt checks compare derived inventory with restored database truth
before any mutation. Qualification uses serving-role RLS/plans, including routing and physical
checks, under its generation lock; the writer rechecks the realization atomically afterward.

## Frozen retrieval inputs and workload meaning

`calibration-requests.json` and `confirmation-requests.json` contain distinct unsealed development
queries, frozen before measurements. They do not consume behavioral gold or heldout outcomes.
Each has eight unfiltered, eight broad-filter, one brief and one selective request. Actual vector
population determines routing; labels never force a small/selective request onto ANN.

`measure_import.py` records actual import/index wall time, database/index/artifact bytes and
cluster-wide WAL. `scripts/postgres_workload.py` measures stdio search including embedding separately
from rank-stage qualification. Its 1x/10x fixture repeats the same complete hydration payload 10/100
times; it measures transport volume, not a claim that a corpus ten times larger has been qualified.
RSS sums the harness and owned MCP processes and excludes PG/vLLM. Other metrics identify their
scope explicitly. Exact-only completion is valid if no real ANN class meets the preregistered gates.

## Assembled qualification

**Passed for the supported local exact route, 2026-09-28.** The raw receipt inventory includes
source paths, sizes and SHA256 hashes in `raw/receipts.json`; raw files use Git LFS. Protected
connection files, dumps and external artifact backups stay under `build/` and are not committed.

| Command / measured consumer | Outcome and receipt |
|---|---|
| `just fmt`, then `just test-all` | **passed** on repair run: 439 ordinary Rust tests across 47 binaries, 171 Python tests, 18 real PG tests and two async controls; strict Rust/Python checks, 84 fixture parses, seven rule suites, 37 ADRs, agent/dependency/gold policy and fresh SQLx metadata. `raw/postgresql-pg17-test-all-repair.log`; initial Clippy failure retained in `raw/postgresql-pg17-test-all.log` |
| `cargo test --release -p lctx-postgres --lib reference_checks_numerics_and_declared_order_without_fuzzy_ties` | **passed:** numerical crossing, each reference's ordering, true ties, nonfinite/error and rank controls; `raw/postgresql-pg17-numeric-regression-final.log`. This followed the full gate and did not rerun unrelated tests |
| `cargo clippy --release -p lctx-postgres --all-targets --quiet -- -D warnings`; `uv run pyrefly check` | **passed:** affected Rust and Python checks; final logs retained. `just docs-check` passed 155 canonical pages plus ADR/agent and offline links; `just fmt-check` and affected Ruff passed in the named `*-closure.log` receipts |
| `uv run pytest -q python/lctx_mcp/tests/test_recovery.py python/lctx_mcp/tests/test_qualification.py` | **passed:** three real-DB controls, `raw/postgresql-pg17-final-focused.log`; the subsequent format-2 foreign-ID regression passed its single selector in `raw/postgresql-pg17-membership-final.log` |
| `LCTX_TEST_PROJECTION="$PWD/build/generations/81b65b0319255a5a" cargo test --release -p lctx-postgres --test serving physical_admission_reindex_and_artifact_relocation -- --ignored --nocapture` | **passed:** real pilot population also challenges an already-pinned ANN reader after REINDEX; `raw/postgresql-pg17-physical-final.log` |
| `LCTX_DATABASE_CONFIG=build/postgresql-pg17-qualification/postgres.json just pilot build/postgresql-pg17-fake-store build/postgresql-pg17-fake-pilot.log build/postgresql-pg17-qualification/postgres-serving.json` | **passed:** fresh fake-vector canonical compile, import and 20 stdio MCP brief checks; `raw/postgresql-pg17-pilot-gate.log` |
| `target/release/lctx compile fastmcp --store build/postgresql-pg17-live-store --embedder vllm`, with isolated application config | **passed:** fresh live 1024 canonical compile; 249.8 seconds total, 56.6 seconds extraction, peak 4,287,652 KiB; live pilot/cost/smoke logs retained |
| Same live compile to `build/postgresql-pg17-warm-store`; direct vector-file byte comparison | **passed:** cold/warm `vectors` and `operation_vectors` byte-equal; warm 249.7 seconds with 31.9-second extraction. Concurrent builds/workloads prevent a controlled compiler speedup claim; `raw/postgresql-pg17-warm-equality.json` |
| `just embed-conformance` | **passed:** Rust/Python live request/1024 output contract, worst cosine 0.9999364; `raw/postgresql-pg17-conformance.log`. No 4096/1024 quality reevaluation |
| `lctx bundle` from the live canonical snapshot with PG and embedder URLs unreachable | **passed:** all 52 output files byte-equal; `raw/postgresql-pg17-offline-rebuild.log` |
| Frozen format-2 `lctx serving qualify-hnsw --pack … --serving-config …` | **failed ANN admission as intended:** no qualifying candidate; corrected numeric reference passed. Original and comparison2 reports retained; details below |
| `postgres_workload.py` against two retained generations, fake/live embedding and compile/import/report contention | **passed:** pin/cursor rollover, concurrency1/4 and repeated hydration. Lexical, contended and live workload JSON receipts retain individual scopes |
| Operator `lctx db migrate`; `lctx db check`; import/reconcile/select/status/report; the repository stdio MCP smoke helper | **passed:** explicit 001–008 deployment, live exact selection, admitted federation and all 20 stdio brief smoke checks; operator logs/JSON retained |
| `uv run python scripts/postgres_backup.py backup build/postgresql-pg17-operator.dump`; `… restore-drill build/postgresql-pg17-operator.dump` | **passed:** 69 logical roots and two ready generations restored, both native/MCP lifespans usable, captured exact selection recovered; 15.313 seconds including verification/serving, versus 900-second objective; operator backup/restore logs retained |

The full gate's 19 ordinary Rust and two ordinary Python explicit-PG skips are not passes; the
separate real-PG leg ran 18 tests plus two async controls. Its two skipped selectors are the
explicit captured-reference probe and the non-ignored role-configuration unit already covered by
the ordinary gate. Prior same-input full reference evidence is retained; this packet separately
supplies live compilation/conformance/retrieval. Positive ANN confirmation is not a skip
concealed as success: no candidate qualified to reach that step.

### Live numerical and ANN results

Canonical cold snapshot: `62d5341611257e5e91482022659df6b8`; bundle
`build/generations/81b65b0319255a5a`; projection
`32b4cb5d2ec432ec58281f38146851e7c5f256543e6803a97f9ead909dbd1d1c`; spec
`7203d2bb3942fb9d81aa98a715d9fe3a364ff809ba7ca325e28cbc9a037466ae`.
The frozen calibration pack SHA256 is
`16469e5eeda5dc2f71ba071236bfddd323fe565938c89c548252f18de3278482`.
It covers 1,171 distinct vector-bearing operations, 2,454 chunks, 1,534 total operations and 20
briefs. The broad prefix selects 706 vector entities and the selective control 50, so both stay
exact under either floor. The 4,096 floor routes every request exact; only the 1,024 floor's eight
unfiltered queries can nominate an ANN class.

| Rank-stage run (80 timed samples per route) | Exact p95 | ANN p95 | Per-view / fused recall | Natural plans / benefit |
|---|---:|---:|---|---|
| 1 | 21.771568 ms | 53.256070 ms | 1.0 / 1.0 | failed / failed |
| 2 | 15.299343 ms | 44.959229 ms | 1.0 / 1.0 | failed / failed |

Both paths use the serving role; rank timing includes mixed routing, physical catalog lookup,
shared transaction lock and compact IPC. The qualifier holds its generation maintenance lock,
then the writer atomically rechecks the realization. No forced sequential-scan disable or fallback
is counted as successful ANN. `chosen_policy` is null. Confirmation is **not_run: no admissible
candidate**; the separate confirmation requests were not retuned or used as ANN acceptance.

The original full ordinal assertion failed at signature_doc ranks382/383 for one broad query.
Comparison revision2 preserves that observation while checking complete per-entity numerical
agreement and each engine's own ordering. Maximum score error is 1.781234980069435e-7, below the
1e-5 contract; every vector top-10 and fused top-10 agrees. The pair is unequal in both engines:
PostgreSQL orders `37b476…` before `a1d88d…`; float64 reverses them. No fuzzy tie rule was added.
The original report and frozen pack remain alongside the corrected report. The final malformed-
reference-order guard was added after measurement, validated by its regression; the reviewer
independently checked the frozen references are sorted. It does not relabel full ordinal equality.

### Operational costs and deployed state

On the isolated cluster, actual import took **6.952 seconds** and HNSW construction **0.381 seconds**.
Database bytes were 192,796,351 before import, 280,286,911 after import and 300,488,383 after index
construction. Cluster-wide WAL counters were 231,448,404 / 340,650,383 / 351,618,315 and include
concurrent work. The original `live-import-cost.json` field named `vector_index_bytes` actually
sums **all serving-table index bytes** (45,277,184 / 80,207,872 / 100,409,344); the helper now labels
this `serving_index_bytes`. The raw receipt is preserved without rewriting measurements. Retained
artifact-root bytes were 11,715,626. These are this two-generation fixture's costs, not a scaling law.

The operator's live workload passed 54 searches each at concurrency1/4, with embedding-inclusive
p95 **86.97 / 99.35 ms**. Two MCP servers started in 4.28 seconds; peak aggregate harness/MCP RSS
was 584,556,544 bytes, excluding PostgreSQL/vLLM. Repeating the same 74,851-byte hydration payload
10/100 times took 0.489 / 2.869 seconds. This is a 1x/10x transport fixture, not a tenfold corpus.
The compile-contended live run was separately slower (445.20 / 493.54 ms p95) and is retained.

Operator schema008 selects the projection above with exact policy
`ff645e4a55461d3041fa4dbbd56f01d90da07b74a55040ec350933ac23ef4586`. The previous ready generation
`712eb62155beb656b960fb5fafc1128bab6a5f70d94402caa81bceff68ec798a` remains available. No HNSW
index/admission was installed on the operator cluster after its candidate failed qualification.
Serving budgets are four SQLx plus two provider connections per process; importer two and cache
application six. Ordinary statements remain 30 seconds; finite maintenance is 300 seconds with
305-second cancelled-lease reaping, 256 MiB maintenance memory and two workers.

Retain-all and daily operator backups remain the policy. The post-cutover receipt includes both
generations and actual authored/cache history; logical restore explicitly recovers exact, because
new physical indexes need fresh admission. The schema004/schema007 rollback pairs and independent
PG7 assets remain protected. No automated retention, scheduler, remote HA/PITR, native ADBC,
Python ORM or Stage 3 completion is implied by this local acceptance.

The owned temporary qualification database and vLLM server were stopped after measurements.
The operator PostgreSQL cluster and preserved schema007 rollback pair remain available. Start
the embedding service with `uv run python scripts/embed_serve.py` when live-query embeddings are
needed; this work did not install an embedding-service supervisor.
