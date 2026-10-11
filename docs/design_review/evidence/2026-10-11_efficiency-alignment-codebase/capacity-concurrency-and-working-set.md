# Inquiry H: Capacity, concurrency and live working set

**Provenance.** This is the verbatim final report of supporting inquiry H, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `H-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# Inquiry H: capacity, concurrency and live working set (supporting assessment)

Baseline: `main` at `88f33bc0`, read on 2026-10-11. This is read-only static review plus reading of the pinned and patched library source. I ran no builds, tests, services, SurrealDB queries or `just` recipes. Candidate labels (`H-nn`) are local to this inquiry and are not finding IDs.

## 1. Scope examined and coverage limits

**Examined.**
- **Native control protocol.** In `crates/lctx-surrealdb/src/`:
  - `control/authorization.rs` (`ERA_GUARD`, `effect_for_owner`, `execute_owner`, `begin_attempt_in`);
  - `control.rs` (`run_transaction`, `retryable_conflict`, `ensure_rows_owned`, `hold`, `hold_many`, `guarded_effect`, `pin_views`, `release_pin`, `ReaderPin`);
  - `control/history.rs` (era closure);
  - `control/retirement.rs`.
- **Store paths.**
  - `compiler.rs`: `write_batch_inner`, `ensure_physical_rows`, `complete_contribution{,_scoped,_inner}`, `attach_retained_product`, `retain_attached_members`, `wait_operations`, `scan_graph_inner`.
  - `reader.rs`: `NativeReader` scope, `selected_membership`, `records`, `selection_preparation`.
  - `selection.rs::SelectedPayloads::prepare`.
  - `config.rs::RuntimeConfig::lock_selection`.
  - `product_cache.rs::lock`.
- **Compiler side.**
  - `cpg-core`: `native_bridge.rs`, `native_calls.rs`, and in `workspace.rs` `Workspace::new`, `Writer::write_batch`, `ProducerOutput::{push, push_sync, write}`, `owned_batch_write`, `submit_batch_write`.
  - `cpg-extract`: `bundle.rs::run_provider` and `pyrefly_stage.rs::analysis_pool`.
- **Publication.** `crates/lctx-publisher/src/lib.rs` (reference loop), `native_publication.rs::seal_staging` and `backup.rs::retire`.
- **Serving admission.**
  - Rust: `crates/lctx-serving/src/service.rs::NativeService::{new, run}`, `preparation.rs::{RequestAdmission, PreparedCache::initialize, release_optional_retention, admit_request}`.
  - Model: `lctx_model::domain::serving::resources::ResourceLimits`, `lctx_model::domain::resources`.
  - Python: `python/lctx_semantics/src/session.rs::NativeSession::new`, `python/lctx_mcp/src/lctx_mcp/native.py::NativeExecutor` and `__main__.py`.
- **Service configuration.** `scripts/surrealdb_service.py::unit_text`.
- **Pinned library semantics.**
  - `~/.cargo/registry/src/*/surrealdb-kvs-rocksdb-3.3.0/src/{lib.rs, owned_tx.rs}`.
  - In `surrealdb-core-3.3.0/src/`: `exec/operators/scan/record_id.rs`, `dbs/executor.rs` (`execute_plan_impl_inner`, the streaming statement loop) and `rpc/protocol.rs` (streamed query timeout).
  - `surrealdb-rpc-3.3.0/src/stream.rs::QUERY_STREAM_BUFFER`.
  - The patched SDK `third_party/surrealdb/src/engine/remote/grpc.rs` and the server provenance in `third_party/surrealdb-server/provenance.json`.
  - The skill brief `neo4j-surrealdb/surrealdb/content/capabilities/surrealdb.transactions-and-concurrency.md` (sb090, sb091, sb092, sb096).

**Not read, or only skimmed.**
- The interior of `control/retirement.rs` beyond its effect SQL.
- `lexical_stats`, the `derived_search` lowering internals and `Loader::reconcile`.
- Embedding batching to vLLM (`lctx-embed`).
- The tooling side of S10 (owned by F2).
- Server-side gRPC per-session scheduling: I did not locate the server's request handler, so whether one gRPC session serializes its requests server-side is unverified. The SDK documents HTTP/2 multiplexing (`grpc.rs` module doc).

## 2. Complete operations examined

| Operation | Stage → owner → crossing → live state |
|---|---|
| **Compile, facts** | Provider `std::thread` (512 MiB stack, `bundle::run_provider`) → `Writer::write_batch` → `NativeBridge::call_boxed` (`futures::executor::block_on`; mpsc(2) into a current-thread runtime) → `NativeCalls::call_boxed` (`Semaphore(8)`, `tokio::spawn` on the bridge runtime) → `write_batch_inner` → per W=128 window, sequential effects, each 3 transactions plus reads. **The provider blocks until each batch is acknowledged.** Providers run sequentially. Live state: one batch in roughly 5 representations, plus `BatchWriter.seen` (charged, O(output rows)). |
| **Compile, stages** | `ProducerOutput::push` awaits `submit_batch_write` per flushed batch. Stages run sequentially. Kernels run on `spawn_blocking` (`catalog_core`, `catalog_evidence`). DataFusion `RuntimeEnv` has a pool of `memory_bytes`, default **64 GiB** (`DEFAULT_MEMORY_BYTES`), with a spill directory in a tempdir. |
| **Compile, admission and publication** | `Lookup::load` holds every entity and assertion header (charged 192 B each). The canonical scan is windowed through prepared pointer candidates (`scan_graph_inner`). `seal_staging` keeps **one streamed** `native_hold` query open while issuing a `hold` effect per 128 roots. |
| **Warm reuse** | `attach_retained_product` keeps **one streamed** source-membership query open while `retain_attached_members` writes 128-row windows (`ensure_rows_owned` + `hold_many`). |
| **Contribution completion** | `complete_contribution_inner` keeps **one streamed** membership query open while calling `new_membership_keys` per 128 keys. |
| **MCP start** | `NativeSession::new` builds one multi-thread Tokio runtime per process (default worker count) → `NativeReader::connect` → `pin_views` (effect plus `hold`) and `protect(publication)` (`hold`), so 3 effects and 9 writes to `native_installation:current`. |
| **MCP request** | Python `Semaphore(2)` (1 s wait), held until the shielded worker finishes → the HTTP embed (when searching) runs **inside** that slot → `to_thread` → `block_on` → Rust `RequestAdmission`: 1 query + 1 CPU permit (each pool sized 2), suspended around shared preparation flights → `ResourceBudget::scoped(shared 256 MiB, request 64 MiB)`. **No store effects on the request path**: all reads are snapshot reads. Detached `selected_membership` tasks outlive a timed-out request; the configured server query timeout bounds them (acknowledged in synthesis-and-serving §11.3). |
| **Service** | `--query-timeout 20s --transaction-timeout 10s`, RocksDB (`scripts/surrealdb_service.py::unit_text`). One `native_installation:current` record per database. Concurrent fixture tests share the `validation` database (docs/surrealdb.md: "No database per attempt or test"). |

## 3. Candidate findings

### H-01: One installation record is written by every effect transaction, so concurrent effect writers across processes conflict

**Observed cause.** `control/authorization.rs::ERA_GUARD` runs `SELECT … native_installation:current FOR UPDATE` and then `UPDATE native_installation:current SET control_revision += 1`. `execute_owner` sends three transactions per effect, and each one writes that key:
- the intent transaction (`ERA_GUARD`, plus `admission_revision` for installation-owned effects);
- the payload transaction (`ERA_GUARD`, then the caller's SQL);
- the outcome-reference release transaction (`UPDATE … control_revision`).

`begin_attempt_in`, `reconcile_request`, `reconcile_effect`, era closure and upgrade publication write the same key.

**Pinned semantics (settled).**
- The server is SurrealDB 3.3.0 with a timeout-stack patch only (`third_party/surrealdb-server/provenance.json`).
- Its RocksDB backend opens every transaction on an `OptimisticTransactionDB` with `set_snapshot(true)` (`surrealdb-kvs-rocksdb-3.3.0/src/lib.rs::Datastore::transaction`).
- `FOR UPDATE` lowers to `getu` → `get_for_update_opt(key, true, …)`, which "register[s] the key for commit-time conflict detection" (`lib.rs::getu`, `core exec/operators/scan/record_id.rs::for_update`).
- A plain `put` also "records the key for commit-time conflict detection" (`lib.rs`, the set/put path).
- There are **no locks**. A transaction fails at commit with `Busy` ("Transaction conflict: Resource busy") if a tracked key was written after its snapshot. The skill's runtime probes sb090 and sb091 show this on embedded RocksDB.
- The server does not retry user transactions; the conflict surfaces to the client (`core dbs/executor.rs`, the conflict handling around commit).

**Exact critical section (calibration safeguard 9).** The section is each transaction's interval from snapshot to commit, not the effect as a whole and not the SQL text. Every such transaction writes the same key. Among any set of transactions whose intervals pairwise overlap, only the first to commit succeeds; the others abort after doing their full server-side work. `control.rs::run_transaction` then resends the whole query immediately after `tokio::task::yield_now()`.

**What is not serialized.**
- *Within one compile*, effects are already strictly sequential. Providers run sequentially, each write awaits its acknowledgement (`Writer::write_batch` → `bridge.call_boxed`; `ProducerOutput::push` awaits), windows and effects inside `write_batch_inner` are awaited in order, and there is no `join!`, `join_all`, `buffer_unordered` or `JoinSet` outside tests in `lctx-surrealdb/src`, `lctx-publisher/src`, `lctx-serving/src` or `cpg-core/src` (`rg` over those trees; the hits were tests, the bridge actor and `spawn_blocking`). So one compile does not conflict with itself.
- *Serving requests* perform no effects and use snapshot reads, so a compile or publication cannot abort or block them at the key-value layer.

**Consequence.**
- **S10.** Up to 32 nextest processes run fixture-backed tests on the one `validation` database, for example `cpg-extract::extraction_calls`, where each test runs `compile_facts` against a native store (`typed_driver/mod.rs`). Every effect transaction from every process contends on one key. Losers redo full payloads such as 128-row `ensure_rows_owned` guard loops. Short intent and release transactions from other processes repeatedly invalidate long payload transactions in flight.
- **Concurrent compiles (S02, several libraries), and MCP pin acquisition or release during a publication (S05/S06).** The same mechanism applies at lower rates.
- **Future parallelism.** The protocol also makes any intra-compile concurrency (H-05, or parallel providers raised by inquiry A) convert directly into conflicts.

The mechanisms are H21 and H23 (global exclusion where per-object guards already order real dependencies), H24 (retry multiplication) and H22. The `NativeCalls` `Semaphore(8)` capacity cannot be realized under this protocol.

**Why it exists (to be preserved).** The `control.rs` module header says "snapshot reads and predicate scans alone do not provide this protection". ADR-0145 (§Issuance) says "Keep installation coordination initially … narrower coordination requires equivalent delayed-operation and recovery exclusion guarantees". Era closure (`history.rs`) and retirement (`retirement.rs`) run predicate scans over `native_effect`, `native_attempt`, `native_pin` and `native_backup_hold`, and RocksDB optimistic transactions do not track ranges. The write on the installation record materializes those phantoms.

**Simpler conforming alternative (Proposed).** Put the cost on the rare side.
- Effects **read** the installation record `FOR UPDATE` but do not write it.
- Fences (era closure, upgrade publication, maintenance drain) become **two-phase**: commit a fence write first, then scan in a transaction that begins after that commit.
- **Case 1: the fence commits during an effect.** The effect's `FOR UPDATE` key changed after its snapshot, so the effect aborts.
- **Case 2: the effect commits before the fence commits.** A scan that starts after the fence sees the effect.
- **Case 3: the effect starts after the fence.** It reads the fence and throws.
- Where a scan cannot be split this way, use K striped lane records: an effect writes the lane `hash(owner) mod K`, and fences read all K lanes `FOR UPDATE`.
- Per-object `native_guard` records remain the real same-object serialization. The library has no alternative: 3.3's RocksDB backend opens only optimistic transactions (`owned_tx.rs::Db`), so explicit conflict materialization stays necessary. The change narrows where it happens.

**Preservation constraints and challenge (safeguard 12).**
- Delayed and retried requests must still be fenced by era and `closed_through`. The payload read of the installation is kept.
- Retirement's `native_backup_hold` scan currently relies on backup-hold creation bumping `backup_revision` and on retirement's own installation write. Each predicate-scan consumer needs its phantom partner identified and its fence re-derived. The concrete counterexample to check is a backup hold committing just after a retirement that no longer writes the installation record.
- Outcome-reference release must stay ordered after commit.

**Uncertainty.** The magnitude is unmeasured. Historical P4 shows 28 `extraction_calls` tests each lasting about 280.6–280.8 s under concurrency. That is consistent with a shared serializing resource, but the near-identical end times suggest a common gate, and CPU contention is equally plausible. It is unattributed.

**Evidence label.** Interface-checked (engine semantics at the pinned source), Implemented (protocol); consequence Proposed.

**Closure evidence.** A per-site inventory of phantom dependencies with each one's fence. A revealing control: N concurrent processes issuing disjoint effects show no cross-process conflict retries, while a fence racing an effect still aborts one side in each interleaving. A conflict-retry counter (see H-03) on a concurrent S10 boundary.

### H-02: Consumer-paced single statements span per-window round trips under fixed 10 s / 20 s server deadlines

**Observed cause.** Three operations keep one streamed SELECT open while issuing further round trips per 128 rows:
1. `lctx-publisher/src/native_publication.rs::seal_staging` streams `SELECT VALUE object FROM native_hold … WHERE owner=$attempt ORDER BY id` and runs a `control::hold` effect (3 transactions) per 128 roots.
2. `compiler.rs::attach_retained_product` streams every source `compiler_membership` row and calls `retain_attached_members` (`ensure_rows_owned` + `hold_many`) per window.
3. `compiler.rs::complete_contribution_inner` streams all contribution memberships and calls `new_membership_keys` per 128 keys for shared relations.

**Pinned semantics (settled).**
- A streamed bare statement runs in its own transaction under `transaction_timeout` (`core dbs/executor.rs::execute_bare_statement` → `execute_plan_impl_inner`).
- The whole stream is bounded by `query_timeout`, "covering producing every row" (`core rpc/protocol.rs`, the streaming-job timeout).
- Production is paced by the consumer through a 2-item buffer (`surrealdb-rpc-3.3.0::QUERY_STREAM_BUFFER = 2`).
- The installed unit sets `--query-timeout 20s --transaction-timeout 10s` (`surrealdb_service.py::unit_text`). docs/surrealdb.md records these deadlines as fixed.

**Consequence.**
- **S02, and S03 warm reuse of a large product.** Each stream lasts roughly (rows / 128) × (window round trips × round-trip latency), plus the server-side `ORDER BY` sort. That grows linearly with library or attempt size and hits a fixed 10 s wall. This is a predictable failure of supported work, not a protective refusal (H11, H23: a transaction or snapshot held open across unrelated effects).
- The open read transaction also pins a RocksDB snapshot for its whole duration.
- Historical receipts (persisted-graph plan §9.1, 2026-10-10, attributed to other operations) record real failures on the 10 s deadline: a full recovery "failed after 24m50s on 10s transaction timeout", and a 10 s timeout in derived-search reconciliation. The deadline already binds in practice.

**Simpler conforming alternative.**
- Page each of these by keyset into short independent statements. The repository already does this in `control::keyset_source` (history and migration scans) and in `release_pin`'s `LIMIT 128` loop.
- Alternatively, drain the stream into the existing disk-backed `SortedRows` before issuing effects.
- Each premise is immutable during the operation, so paging cannot miss or duplicate rows:
  - the attempt is frozen before `seal_staging` (content lane closed);
  - the source contribution is completed and admitted before attachment;
  - the producing scope is closed before the completion digest.
- Paging must preserve the ordering that feeds `KeySink` digests (`relation, semantic_key`).

**Uncertainty.** Whether an index-plus-keyset predicate (`owner=$attempt AND id>$after ORDER BY id LIMIT 128`) is served by the `owner_holds` index without a re-scan at 3.3. Volumes per site for FastMCP are unknown.

**Evidence label.** Interface-checked (pinned server semantics), Implemented (three sites); consequence Proposed. Supported by historical receipts of the deadline binding elsewhere.

**Closure evidence.** Source inspection showing no statement whose duration scales with library or attempt size. A revealing control with an attempt holding more than 10 s worth of roots at a known round-trip latency.

### H-03: The conflict-retry loop is unbounded, immediate and unobserved

**Observed cause.** `control.rs::run_transaction` loops forever on a retryable conflict. It has no backoff, jitter, bound, cancellation check or counter. `cache.rs` has a parallel `retryable_conflict` matching a hard-coded RocksDB message string.

**Consequence.** Under H-01 contention (S10), immediate resends maximize the overlap between transactions and therefore the abort rate (H24). Nothing records how much work retries consume, so H-01 cannot be attributed, and the P4 uniformity stays unresolved (H26: an observation proportionate to a real diagnostic need is cheap here).

**Alternative.** Bounded exponential backoff with jitter. Count conflicts per transaction kind and emit the count through the existing `Phase`/`lctx_phase` events. Keep exact-message matching, which correctly avoids inferring retryability from generic errors.

**Preservation.** Keep the identity-retaining retry: the same request handle and the same bytes.

**Evidence label.** Implemented; consequence Proposed. **Closure evidence.** The retry count is visible in a concurrent boundary's evidence.

### H-04: The per-process memory budget is a host-independent 64 GiB constant that also sets DataFusion's spill threshold

**Observed cause.**
- `lctx_model::domain::resources::DEFAULT_MEMORY_BYTES = 64 GiB` is the default for `WorkspaceOptions`, the CLI flags (`crates/lctx/src/main.rs`) and test fixtures.
- `cpg_core::workspace::Workspace::new` uses it both as the `ResourceBudget` ceiling (`WorkspacePool`) and as `RuntimeEnvBuilder::with_memory_limit`.
- Its own doc says reservations "account admitted buffers, not process RSS".
- Each process admits independently. Nothing relates the ceiling to host or cgroup availability or to other concurrent consumers.

**Consequence.**
- **S10.** 32 concurrent test processes, nominally 32 × 64 GiB on a 188 GiB shared host.
- **S02.** Concurrent compiles.
- In both cases DataFusion sorts and joins spill only after 64 GiB per process. On a contended host, swap and memory pressure arrive before any spill triggers (H17, H21: downstream capacity does not constrain upstream admission). A historical receipt (persisted plan §9.1, 2026-10-10) recorded 178/188 GiB used, 7.9 GiB swap and high memory PSI during a failed recovery. It is unattributed but shows the regime occurs.

**Alternative (Proposed).** Coordinate capacity rather than capping parallelism:
- derive the default envelope at start from cgroup `memory.max` or host-available memory;
- or have the run owner (`just run` / `verify`) give each child an explicit envelope from one shared host budget;
- keep the per-request and per-stage reservation machinery unchanged.

Concurrency itself is untouched (see RC-H2).

**Challenge.** A fixed small ceiling would refuse valid S02 work. The envelope must be capacity-derived, and refusal must stay explicit.

**Uncertainty.** Actual peaks per fixture test and per compile are unmeasured. Most fixtures may be small.

**Evidence label.** Implemented; consequence Proposed. **Closure evidence.** The stage peak (`ResourcePool::stage_peak`, which exists) recorded for S01 and for a concurrent S10 boundary, compared with the derived envelope. A spill observed under pressure rather than swap.

### H-05: Acknowledgement per batch serializes production with store round trips (seam with A)

**Observed cause.** `Writer::write_batch` blocks in `NativeBridge::call_boxed` until the batch is acknowledged, and `ProducerOutput::push` awaits `submit_batch_write`. Provider computation and write round trips therefore alternate strictly. The declared `NativeCalls` `Semaphore(8)` and the bridge's `JoinSet` are never reached by one producer.

**Consequence (S01/S02).** Elapsed time is the sum of compute and write latency rather than the larger of the two (H22). The live set stays at one batch, which is a strength.

**Alternative.** A bounded in-flight depth of 2 per producer, keeping the charged ownership and backpressure. Failure is still observed before completion through `ProducingScope::close_and_wait`. **Prerequisite:** H-01. Without it, overlapping a producer's own effects turns into installation conflicts.

**Uncertainty.** Whether compute or round trips dominate is unmeasured. Inquiry A owns the producer-pipeline judgment.

**Evidence label.** Implemented; consequence Proposed.

### H-06: No external cancellation route reaches the compile's owned drain

**Observed cause.**
- No signal handling exists in `crates/lctx/src` or `crates/cpg-core/src`: `rg "tokio::signal|ctrl_c|SIGTERM|SIGINT"` returned nothing.
- `lctx/src/compile.rs` calls `store.abandon()` only on error paths.
- Provider cancellation is checked at emit points (`bundle.rs`, the `cancelled` checks), not inside Pyrefly's `transaction.run`.
- Run cancellation is SIGTERM; plan receipts show "gracefully cancelled (exit143)".

**Consequence (S07, S08, and S10 via nextest `terminate-after`).** A cancelled compile or test leaves its attempt `open`, with attempt-owned holds that keep content out of retirement and possibly an unresolved effect. Only maintenance (`drain_installation`, which requires closed admission) resolves them. Work with no consumer is not drained promptly (H21), and recovery is broader than the cancelled unit (H24).

**Alternative.** Translate SIGINT/SIGTERM into the existing `Cancellation`. The existing owners (`NativeCalls` `CallGuard`, the bridge drain, `ProducerOutput::drain_provider`) then run to their terminal results, followed by `abandon()`. Crash recovery through maintenance remains for SIGKILL and crashes.

**Preservation.** Committed effects keep their identity. Unknown outcomes stay protected, not deleted.

**Evidence label.** Implemented (absence with the stated search scope); consequence Proposed.

## 4. Opportunities and uncertainties that are not findings

- **Pyrefly `ThreadPool(AllThreads)` per process** (`pyrefly_stage.rs::analysis_pool`), multiplied by nextest's process-per-test model and the per-process Tokio, DataFusion (`target_partitions` = CPUs) and Python runtimes. CPU oversubscription mostly redistributes the same work and does not fail, so this is not a finding. It is the leading alternative explanation for P4's uniform ~280 s. Probe P-H2 discriminates.
- **The Python `Semaphore(2)` slot spans the vLLM embed** (`NativeExecutor._execute`). A slow embedder occupies store capacity for non-search tools. For one agent issuing sequential calls this is immaterial. The Rust permits (2 + 2) are then effectively never contended, so the two layers duplicate one bound. This is coordinated rather than nested independently, which is acceptable.
- **Each stdio MCP process has its own runtime, pin (3 effects at start), 256 MiB budget and caches.** Several agents multiply these against one server with no server-side workload classes. On the request path, records are hydrated sparsely (`NativeReader::records` → `candidate_payload_rows`), so the cost duplicated per process is small. Capacity under S06 is unmeasured.
- **The selection lock spans retirement's store work** (`backup.rs::retire` holds `lock_selection` across `retire_reachable(…, 4096)`). A concurrent `select` fails with `Contention` after 30 s (H23). Both are rare operator actions, so this is low consequence.
- **Fixed-interval polling remains for file locks**: `product_cache.rs::lock` at 10 ms with no deadline, `lock_selection` at 10 ms up to 30 s, and maintenance index readiness at 100 ms. Each wait is short-lived, so the cost is immaterial.
- **Compile-side retained structures grow with the graph under S02**: `Lookup::load` (all headers in memory), `membership_preparations` per view, and `BatchWriter.seen`. All are charged and some are disk-backed. Peaks are unknown (see H-04).

## 5. Strengths, preservation constraints and adequate areas

- **Adequate: compile versus serving contention (S05/S06) at the transaction layer.** The serving request path writes nothing, and snapshot reads under optimistic concurrency neither block nor abort. A publication cannot stall serving except through shared CPU and IO.
- **Admission ownership.** Python holds a slot until the shielded worker completes, and Rust receives the remaining deadline. `RequestAdmission::{suspend, resume}` around shared preparation avoids permit deadlock. Preparation flights have a charged owner, and waiters own only their wait.
- **Budget composition.** The serving shared budget now covers reader-scope selection (`NativeService::new` → `reader.with_budget(&shared)`).
- **Ownership through cancellation.** The bridge is event-driven, with mpsc, oneshot and `Notify`. `NativeCalls` keeps task ownership through caller cancellation (`CallGuard` fails the attempt). `ProducingScope::close_and_wait` replaced the attempt-wide barrier for ordinary producers.
- **Bounded live set per producer.** One batch is in flight at a time, and charges transfer rather than duplicate (`Writer::flush`).
- **Precise retryability.** It is matched on the typed detail or the exact RocksDB message, never inferred from generic errors.
- **Preservation constraints:** era and closed-through fencing of delayed requests, per-object guards, outcome-reference ordering, and the 20 s / 10 s deadlines as protection for residual serving reads.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| GK-F03 (reservations far beyond allocation) | **resolved** for the diagnosed amplification; **residual** heuristic envelope | `scope_program.rs` uses `KeySink::counting` for exact framed length. `compiler_scope_program::reserve_construction` keeps a conservative arena (512 KiB + 8 KiB per input + 4 KiB per field + 32 KiB per target), which never binds under H-04's 64 GiB. |
| C-F02 (optional discovery caches) | **resolved** | No `discovery_cache`/`DiscoveryCache` symbol under `crates/`. `consumed_rows.rs::PreparedEdges` keeps exact per-frontier demand only. |
| compiled-product-reuse F01 (unknown completion outliving ownership) | **not examined** | Owned by D and C |
| compiled-product-reuse F02 (detached wait owners) | **resolved** | `preparation.rs::PreparedCache` flights map with the charged `viewer-preparation-flight`; waiters own only their wait |
| holistic-state-management-integrated F03 (selection bypasses serving budget) | **resolved** | `reader.rs::selection_preparation` requires the caller budget; `SelectedPayloads::for_cancellable_views(…, &retained_budget)` |
| native-execution-efficiency F05 (fixed-interval polling) | **resolved** for the bridge; **residual** for file locks | `native_bridge.rs::next_event` and `call_boxed_async`. Remaining sleeps are in `product_cache.rs::lock` and `config.rs::lock_selection` (§4). |
| native-execution-efficiency F06 / populated-journey F03 (attempt-wide read barrier) | **moved** (resolved for ordinary producers) | `complete_contribution_scoped` → `ProducingScope::close_and_wait`. The global `wait_scans` remains in the public `complete_contribution` (tests and manual clients) and `state_identity_inner`. |
| native-execution-efficiency F07 (Moka generation split in-flight) | **resolved** | `release_optional_retention` replaces retention only; flights live in lifecycle state |

## 7. Candidate rule impacts

- **RC-H1: ADR-0145 §"Issuance, retirement and history"**, "Keep installation coordination initially, restricted to short actual-effect transactions", and the `control.rs` header.
  - *Change:* effects read the installation fence; fences become two-phase (with lane striping where a scan cannot split). This depends on H-01.
  - *If kept:* H-01 becomes an explicit support limit. Concurrent writer processes (S10, concurrent compiles) serialize on one record. H-03's backoff and counter still apply, with a revisit trigger on measured retry share.
- **RC-H2: the policy that selects no fixed memory cap** (docs/surrealdb.md "…job/thread cap or fixed memory cap is selected"; AGENTS.md on not imposing job, worker or thread caps; operator memory "never constrain test parallelism").
  - *Change:* treat the current 64 GiB constant as the fixed cap it is, and replace it with a capacity-derived envelope from the run owner or cgroup. Parallelism is unchanged. This depends on H-04.
  - *If kept literally:* at minimum, document that the ceiling does not bound host memory under concurrency.
- **The 20 s / 10 s deadline rule** (docs/surrealdb.md) is unchanged. H-02 conforms to it. Raising the deadlines is not recommended.

## 8. Proposed light probes

**P-H1 (H-02): do streamed-query timeouts already occur?**
- *Command:* `journalctl --user -u library-context-surrealdb.service --since 2026-10-08 --no-pager | grep -cE "Streaming query exceeded the configured query timeout|did not stop within the grace period"`
- *Discriminating outcome:* a count above 0 shows consumer-paced or size-scaled streams hitting the deadline in practice. The server logs at `warn`, so a zero count is weak evidence of absence.

**P-H2 (H-01 versus CPU contention for P4's uniform ~280 s): what dominated those tests?**
- *Command:*
  ```
  find ~/.local/share/nextest target/nextest -path '*20261011T0712*' -type f 2>/dev/null | head
  ```
  Then, on the stored output of one `extraction_calls` test:
  ```
  rg -o 'phase="(native_batch_write|native_contribution_complete_scoped)"[^\n]*elapsed_ms=[0-9]+'
  ```
  Use the record-store location if `.config/nextest.toml` or the user config names one.
- *Discriminating outcome:*
  - Write phases summing to most of the ~280 s points to store serialization (H-01).
  - Small write phases with a long Pyrefly or provider span points to CPU oversubscription.
  - No retained output for passing tests means the question stays unresolved, and a heavier controlled run (one test alone against 28 concurrent) would be the coordinator's decision.
