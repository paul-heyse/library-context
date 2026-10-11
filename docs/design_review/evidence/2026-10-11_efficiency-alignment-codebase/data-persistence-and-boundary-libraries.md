# Inquiry L1: Data, persistence and boundary libraries

**Provenance.** This is the verbatim final report of supporting inquiry L1, from one read-only
`library-research` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `L1-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

## L1: Data, persistence and boundary libraries (supporting assessment)

**Baseline:** `main` at `88f33bc0`, read on 2026-10-11. **Files written:** none. One scratch copy of the plan at HEAD sits outside the repository at `/tmp/claude-1000/-home-paul-library-context/d8843fdf-9334-49a2-9a36-7d33b58be90d/scratchpad/plan.md`.

**Evidence route:**
- Skills: `neo4j-surrealdb` (its `find` briefs) and `serde-arrow` (upstream benchmark history).
- Context7: `/surrealdb/docs.surrealdb.com` for HNSW filtering. Its docs agree with source and served only as a lead.
- Pinned source settled every decisive question:
  - `~/.cargo/registry/src/*/surrealdb-{core,kvs-rocksdb,idx,cnf,rpc,protocol}-3.3.0` (protocol is 0.13.1);
  - `moka-0.12.16`;
  - `datafusion-optimizer-55.1.0`;
  - `third_party/surrealdb` (patched SDK 3.3.0);
  - `third_party/surrealdb-server`.

**Versions checked:**
- The server binary is the official v3.3.0 plus the timeout TreeStack patch (`third_party/surrealdb-server/provenance.json`). That patch does not touch the KV or index layers, so the core/kvs/idx 3.3.0 registry source applies to the server.
- `Cargo.toml` `[workspace.dependencies]` pins DataFusion =55.1.0 (`sql`, `parquet`, `nested_expressions`), Arrow =59.3.0, serde_arrow =0.15.1 (`arrow-59`), pyo3 =0.29.2, tokenizers =0.23.2, reqwest =0.12.28 and blake3 =1.8.6.
- `crates/lctx-serving/Cargo.toml` pins moka =0.12.16 (`future`).
- `uv.lock` holds fastmcp 4.0.5 and httpx2 2.13.1.

---

### 1. Scope examined and coverage limits

**Examined:**
- SurrealDB:
  - the control/effect protocol (`crates/lctx-surrealdb/src/control.rs`, `control/authorization.rs`, with a partial read of `control/{cleanup,retirement,history}.rs`);
  - immutable ingress (`control::ensure_rows_owned`, `control::hold_many`);
  - membership lookup (`compiler.rs::MembershipLookup`, `MEMBERSHIP_KEY_QUERY`);
  - search definitions (`materialization.rs::native_definitions`);
  - vector scoring (`lctx-serving/src/search.rs::vector_selection_sql` and full-winner rescoring);
  - backup (`lctx-publisher/src/{backup,selected_backup}.rs`);
  - the gRPC SDK batch decoding (`third_party/surrealdb/src/engine/remote/grpc.rs`);
  - server settings (`scripts/surrealdb_service.py::SERVER_SETTINGS`).
- DataFusion: `compiler_provider.rs::{NativeTable, NativeExec, row_statistics}` and the workspace `RuntimeEnvBuilder` (`cpg-core/src/workspace.rs`).
- serde_arrow: the generated `encode`/`decode` in `lctx-model-macros/src/lib.rs`, plus `codec.rs::{batch_bodies, decode_bodies}`.
- External ordering: `lctx-surrealdb/src/ordered_rows.rs`.
- PyO3: `python/lctx_semantics/src/{lib,session}.rs`.
- moka: `lctx-serving/src/preparation.rs::PreparedCache`.
- HTTP: `lctx-embed/src/lib.rs` (reqwest) and `python/lctx_mcp/src/lctx_mcp/embedder.py` (httpx2).
- MCP wire: `lctx_mcp/wire.py`.
- Hashing and tracing: surveyed by `rg`.

**Not read, or read only shallowly:**
- the FULLTEXT `@OR@` server cost model (only the nomination semantics);
- `derived_search.rs` and `lexical_stats.rs` beyond their headers;
- `restore.rs` and `backup_import.rs` internals;
- the `lctx-embed` request scheduling (concurrency across batches);
- vLLM server-side batching and MRL configuration (only the client contract);
- `DEFINE FUNCTION` costs (one `RETURN fn()` per session only, per the operation map);
- tokio runtime topology beyond what the operation map states (H owns it).

---

### 2. Complete operations examined

- **Native effect.** Stage `control::effect_for_owner`/`execute_owner` → owner SurrealDB → three gRPC transactions: intent, payload and outcome-ref release. Each one reads `native_installation:current` with `FOR UPDATE` and writes `control_revision += 1` (`authorization.rs::ERA_GUARD` and the release SQL). Each commits durably: RocksDB `SyncMode::Every` by default (`surrealdb-kvs-rocksdb-3.3.0/src/cnf.rs`), and the service sets no override (`SERVER_SETTINGS` holds only the gRPC message size and export batch size). Live state is the server-side optimistic transaction snapshot, from BEGIN to COMMIT.
- **Immutable ingress of one 128-row window.** Stage `ensure_rows_owned` → payload is a SurrealQL `FOR $i IN 0..len` loop. Per row it runs five statements: `SELECT … $guard FOR UPDATE`, `UPSERT $guard`, `SELECT * FROM ONLY $row.id`, the comparison, then `CREATE` or `INSERT RELATION`. Physical nodes cost two `ensure_rows_owned` effects, plus `hold` and `hold_many` effects. That is 4 effects, so 12 durable commits per window.
- **Compiler view read.** DataFusion `NativeTable` → `StreamingTableExec` wrapped in `NativeExec` → one gRPC query per 128 rows → Value to Arrow (`projected_arrow::ProjectedBuilder`) → serde_arrow `R::decode`. Membership preparation is sorted in the `ordered_rows` external sorter, which has a 1 MiB in-memory run.
- **Vector search (S06).** Python httpx2, with a new `AsyncClient` per call → vLLM → PyO3 `execute` (`py.detach`) → nominate every eligible `vec_occurs` row in scope → `SELECT vector::similarity::cosine(…) FROM $documents … ORDER BY score DESC LIMIT tier` per 128 documents (`search.rs::vector_selection_sql`) → full-4096 rescoring in Rust. The HNSW index `neighbor` (`materialization.rs::native_definitions`) is never queried.
- **Selected backup.** `backup::backup_owned` opens one exporter session and one client transaction, then `selected_backup::export` streams the closure into a bespoke SurrealQL file. Restore re-admits through typed admission.
- **MCP response.** Rust `encode_json`, then three rounds of validation and re-encoding:
  - PyO3 `wire_tool_result` (Rust revalidates);
  - pydantic `CallToolResult.model_validate_json`;
  - two `model_dump_json` + `admit_envelope` pairs.

  All Rust work runs detached (`lib.rs::{wire_decode, wire_tool_result, admit_envelope}` use `py.detach`).

---

### 3. Candidate findings

#### L1-01: One shared record written by every effect makes RocksDB optimistic concurrency serialize all native writes through abort-and-retry

**Observed cause.**
- **Every effect writes one record.** `control/authorization.rs::ERA_GUARD` is `SELECT * FROM ONLY native_installation:current FOR UPDATE; … UPDATE native_installation:current SET control_revision=…+1`. The guard sits in both the intent and payload transactions; the outcome-ref release transaction (`authorization.rs`, release SQL) does the same read-and-update. The comment states the intent: "Every mutation conflicts with era closure, even if its payload writes disjoint records."
- **At 3.3.0 on RocksDB, `FOR UPDATE` is an optimistic conflict registration, not a lock.**
  - `surrealdb-kvs-rocksdb-3.3.0/src/lib.rs` opens an `OptimisticTransactionDB` with `set_snapshot(true)`.
  - `fn getu` calls `get_for_update_opt` "registering the key for commit-time conflict detection".
  - `Busy`/`TryAgain` map to `Error::TransactionConflict`.
  - `surrealdb-core-3.3.0/src/exec/operators/scan/record_id.rs::execute_record_lookup` documents `for_update` as "a locked read: the record key is registered on the storage engine for commit-time conflict detection".
  - Nothing blocks or waits. Any two transactions whose BEGIN-to-COMMIT windows overlap, and which both write `native_installation:current`, cannot both commit; the later committer gets `TransactionConflict`.
  - The server does not retry; core only classifies the error (`dbs/executor.rs`, `is_retryable_transaction_conflict`).
- **The repository retries without bound or backoff.** `control.rs::run_transaction` loops, resending the whole SQL after `tokio::task::yield_now()` when `retryable_transaction` holds. `control.rs::retryable_conflict` matches the exact RocksDB Busy message, which `lctx-surrealdb/src/cache.rs::retryable_conflict` comments as "observed on 3.3". So conflicts already occur in practice.
- **Exact critical section (safeguard 9).** It is each transaction's server-side execution from BEGIN to COMMIT. For the payload transaction this includes the whole caller SQL; for ingress, that is the 128-row FOR loop. Short intent and release transactions that commit inside a payload's window abort that payload, and the whole payload is re-executed.

**Consequences.**
- **S01.** `cpg-core/src/native_calls.rs::NativeCalls::new` admits 8 concurrent native calls. Each window issues about 4 effects, so 12 conflicting commits. The intended 8-way write concurrency collapses: about one installation-writing transaction commits per overlapping round, and the others redo their full payload. Long payloads compete against a stream of short intent and release commits and can starve, because the retry is unbounded and only yields.
  - Mechanisms: H-level hot-spot serialization and wasted repeated work (FP-07/A4: avoidable repeated work and contention), and DP-14 (a guarantee realized more expensively than its contract needs).
- **S10.** Concurrent fixture-backed tests in the shared `validation` database all write the same record, so test parallelism turns into abort contention.
- **S05/S06.** Reader pin acquisition (`ReaderPin::acquire` → installation-owned effects) conflicts with any concurrent compile or publication. MCP cold start and selection therefore retry against compile traffic.
- **Magnitude.** Unmeasured.

**Simpler conforming alternative: stripe the fence.**
- Effects write one of K `native_fence:<n>` records, chosen by hash of the effect or owner id, instead of the single installation record. They read the installation with `FOR UPDATE` for the era check.
- Era closure and any other "must conflict with every mutation" operation (`history.rs` era closure, upgrade publication) reads and writes all K stripes, and the installation.
- This keeps the bidirectional optimistic exclusion the design needs:
  - an effect conflicts with closure through the installation read-registration and its own stripe write;
  - closure conflicts with any effect committed after its snapshot, because closure writes every stripe.
- Effects conflict with each other only on a shared stripe.
- Secondary: bounded exponential backoff with jitter in `run_transaction`, so a long payload is not starved.
- `admission_revision` (attempt epoch sequencing) stays on the installation record; attempt issuance is rare.

**Preservation constraints.**
- Era-cut and delayed-effect exclusion under snapshot isolation, restart and recovery (the plan's SA8 "Coordination" constraint, `docs/plans/surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md`).
- Closure's write must reach every stripe. As the plan notes, "read-only optimism is not equivalent to a conflicting write". Range or index scans are not conflict-tracked in RocksDB optimistic transactions; only point `getu` and writes are. That is why the closure side must write the stripes rather than scan for effects.
- Consumers that read `control_revision` (`control/history.rs` qualification snapshots, `upgrade.rs::source_marker_fields`, `tests/native_control.rs`) would read the stripe set.

**Uncertainty.**
- Abort frequency and the redo cost are unmeasured.
- The real concurrency of payload transactions depends on how often several `NativeCalls` overlap. Static code allows 8.
- RocksDB may also return `TryAgain` when memtable history is insufficient, which adds to aborts.

**Label.** Interface-checked: semantics from pinned source; consequence by structure.

**Closure evidence.** A counter of `run_transaction` retries per effect kind during one S01 compile and one concurrent `store:rust` run, before and after striping. Era-closure controls (`tests/native_control.rs`) must pass with interleavings in both orders.

**Candidate rule impact.** The plan's SA8 "Keep installation ordering initially" (companion §6 Coordination row). Proposed change: allow a striped fence that preserves bidirectional conflict. If the rule is kept, only the bounded-backoff part remains.

---

#### L1-02: Immutable ingress runs about five statements per row in a SurrealQL loop where set-based forms exist at 3.3

**Observed cause.**
- `control.rs::ensure_rows_owned` builds `FOR $i IN 0..array::len($rows) { LET $prior=SELECT * FROM ONLY $guard FOR UPDATE; …; UPSERT $guard SET …; LET $existing=SELECT * FROM ONLY $row.id; IF … THROW; IF $existing=NONE { INSERT RELATION $row | CREATE $row.id CONTENT $row } }`.
- `hold_many` builds the same per-row shape.
- `guarded_effect` adds a per-guard loop.

**Library capability at 3.3.0** (the skill's bulk-writes and SDK-CRUD briefs, `content/catalogs/surrealql.md` `upsert`/`insert`, the UPSERT `@targets` grammar):
- `SELECT * FROM $ids` over an array of record IDs is a point multi-read. RocksDB `getm` uses `multi_get_opt` (`surrealdb-kvs-rocksdb-3.3.0/src/lib.rs::getm`).
- `UPSERT $guards SET revision=(revision ?? 0)+1, incarnation=…` takes an array of targets, and SET expressions can reference the record's current fields.
- `INSERT INTO t $rows` and `INSERT RELATION INTO t $edges` are single all-or-nothing statements.

  A set-based payload could be:
  1. one multi-read of guards;
  2. a THROW if any guard is `retiring` or below `retired_through`;
  3. one array `UPSERT` of guards;
  4. one multi-read of existing rows and an array comparison (THROW on collision);
  5. one `INSERT`/`INSERT RELATION` per table for the absent rows (grouped client-side by table).

**Consequence (S01/S02).** Server-side statement planning and execution grow as about 5 × rows per window, on top of L1-01's redo. The same shape appears in every publication `Loader::insert` window (operation map M2/§1.7). This is H-level per-item crossing where a bulk form exists (FP-07, DP-13). No speed is claimed.

**Preservation constraints.**
- **Per-row collision detection.** Content hashes are addresses, not equality. `INSERT IGNORE` is not a substitute: it silently skips, including UNIQUE violations (skill `SB048`).
- **The guard write.** It is what makes a concurrent retirement abort under optimistic transactions. Retirement detects holds by an index scan, which is not conflict-tracked. Do not drop it.
- **All-or-nothing per effect.**
- **The 64 MiB single-row bound** and the transaction row and byte limits.

**Seam (D).**
- The window pays 4 effects, so 12 durable commits, because anchors, rows, `hold` and `hold_many` are separate effects.
- Folding them into one effect payload, with one intent and one release, is the protocol-level counterpart.
- D owns that choice. It has a different closure obligation (outcome-reference semantics), so it is not bundled here.

**Uncertainty.**
- Whether the 3.3 executor actually plans an in-loop `SELECT … FROM ONLY $param` once or per iteration is not established.
- Array `UPSERT` with per-record expressions referencing prior fields needs a probe for the incarnation arithmetic.

**Label.** Proposed for the remedy; Interface-checked for the capability.

**Closure evidence.** Equivalence controls (collision THROW, a retiring guard, a mixed absent/present window, relation rows), plus the measured window latency before and after.

---

#### L1-03: The bespoke external sorter keeps at most 1 MiB in memory and merges runs pairwise, rewriting rows about log₂(runs) times

**Observed cause.**
- `lctx-surrealdb/src/ordered_rows.rs`:
  - `RUN_BYTES = 1024*1024`;
  - `Sorter::new` rejects `limit > RUN_BYTES`, so all constructors are capped at 1 MiB whatever the `ResourceBudget`;
  - the comment reads "Binary carry merging: logarithmic descriptors and two live heads";
  - `fn merge` is two-way.
- `Sorter::new` always calls `tempfile::tempdir()`, even when nothing spills.
- **Consumers:**
  - compile membership preparation (`AsyncCandidateSort`, operation map M3);
  - publication reconciliation (`reconciliation.rs`);
  - selected backup and restore;
  - serving search, where `search.rs` creates two `SortedRows::with_budget` per lexical hydration, per family, per request.

**Consequences.**
- **S01/S02.** Any view whose membership exceeds about 1 MiB spills. Each row is then written and read about log₂(size/1 MiB) times; a 256 MiB set means about 8 rewrites.
- **S06.** Every search request creates and removes several temporary directories per family on the hot path.
- Mechanisms: H-level eager and repeated I/O, and a live-working-set bound set far below the configured budget (FP-07 A4: physical realization not matched to resources). Magnitude unmeasured.

**Library alternatives.**
1. **DataFusion `SortExec`/`ExternalSorter`**, already linked, with the workspace `MemoryPool` and `DiskManager` already configured (`cpg-core/src/workspace.rs` `RuntimeEnvBuilder`). It spills only under pool pressure and does a k-way streaming merge. Sort keys can be encoded with `arrow-row`, which is already a pinned workspace dependency.
2. **Keep the bespoke owner** but derive the run size from the reservation, use a single k-way `BinaryHeap` merge, and create the temporary directory lazily on first spill.

Option 2 preserves the equal-key byte-identity conflict check that `merge` performs (`A::CONFLICT`). DataFusion has no built-in "duplicate keys must be byte-identical" semantics, so option 1 would need a post-sort adjacent-duplicate check.

**Preservation constraints.** Charged reservations per frame, deterministic global order, duplicate-conflict refusal, spill confined to an owned directory, and cancellation and poisoning.

**Uncertainty.** Real set sizes for FastMCP are unknown (operation map §5.6). If most sets are under 1 MiB, only the per-request tempdir cost remains.

**Label.** Interface-checked.

**Closure evidence.** Spill-count and bytes-rewritten metrics for one S01 compile and one search, before and after.

**Lead position.** The plan's "PC4 bounded external ordering" is *confirmed still present*; this finding refines its physical parameters.

---

#### L1-04: An HNSW index is maintained with no KNN consumer, and serving scores every eligible document exactly, unlike the design text

**Observed cause.**
- `materialization.rs::native_definitions` defines `DEFINE INDEX neighbor ON vector FIELDS embedding HNSW DIMENSION 1024 DIST COSINE TYPE F32`.
- `rg '<\|[0-9a-zA-Z$]'` over the whole repository, excluding `docs/`, `third_party/` and `target/`, finds no KNN operator. The same search found no `WITH INDEX neighbor`.
- Serving uses `search.rs::vector_selection_sql`, which is brute-force cosine over the nominated `$documents` with `ORDER BY score DESC … LIMIT tier`. The documents come from nominating *all* eligible `vec_occurs` rows for the scope (operation map §3.2).
- The design states otherwise:
  - `api-and-evidence-product.md` names "native HNSW candidate ranks" and "scoped HNSW candidate paths remain selected";
  - ADR-0131 says "declared approximate HNSW";
  - `docs/surrealdb.md` says "BM25 and shared-vector HNSW candidates are restricted to eligible contextual occurrences".

**Library facts at 3.3.0.**
- HNSW `<|k,ef|>` is approximate (ef-bounded beam; `surrealdb-idx-3.3.0/src/trees/hnsw/mod.rs::knn_search_with_filter`; docs: "Both algorithms are approximate").
- WHERE conjuncts are pushed down. Covered predicates build a roaring allow-list over shared doc IDs, triaged into tiers by `surrealdb-core-3.3.0/src/exec/operators/scan/knn.rs`:
  - **Exact:** graph-free exact scoring when the allow-list has ≤ `SURREAL_KNN_PREFILTER_EXACT_THRESHOLD`, default 2,000 (`surrealdb-cnf-3.3.0`);
  - **Graph:** allow-list gated with ef ×4 boost up to 1,024;
  - **GraphUnboosted:** above 100,000;
  - **Fallback:** residual evaluated in-traversal when the bitmap build exceeds 250,000 branches.
- `EXPLAIN` exposes the tier.
- Without the index, `<|k,ef|>` returns `[]` silently (skill `SB038`).

**Consequences.**
- **S01 with embeddings, and S02.** Every vector row pays HNSW insertion, deferred through the pending queue and compaction, plus server memory for the graph, with no reader. This does not apply to the default catalog compile, which has no embedder.
- **S06 at S02 scale.** The exact path's per-query work grows linearly with the eligible scope.
- Mechanisms: DP-13/G8 (a library capability paid for but unused), and an H-level eager structure with no consumer.

**Alternatives.**
- **(a)** If exact scoring over eligible scope is the intended contract, remove the `neighbor` index and correct the design text.
- **(b)** If the design's "declared approximate HNSW" is intended, express eligibility as indexed predicates on `vector`. Today eligibility lives on `vec_occurs` (`eligible_input`), so this needs either an eligibility-scoped vector document or an allow-list predicate. Then use `<|k,ef|>` with the prefilter. Small scopes are automatically exact (≤ 2,000) and large ones are labelled approximate.

**Preservation constraints.**
- CI-04 and complete answers: approximate nomination must be declared as such, never presented as exhaustive. The design already treats candidate tiers as "explicit experimental policies, never completeness guarantees".
- Exact full-4096 rescoring of winners.

**Uncertainty.**
- Whether the HNSW route was deliberately deferred is not recorded in the files read. The coordinator should check the evidence/retrieval plan.
- Recall at the 1,024-dimension cosine with ef boost is unmeasured.

**Label.** Interface-checked for the absence; search coverage is stated above.

**Closure evidence.**
- Either: the index is removed and the design is aligned;
- or: an `EXPLAIN` showing the `KnnScan` tier, plus a recall comparison against the exact path on a qualified sample.

---

#### L1-05: Multi-owner explicit-key membership lookup costs one round trip per key, though membership IDs are deterministic

**Observed cause.** `compiler.rs::MembershipLookup` handles two cases:
- **Single owner:** it builds deterministic `membership_id(owner, relation, key)` IDs and reads `SELECT … FROM $ids`, one round trip per 128 keys.
- **Multiple owners:** it issues `MEMBERSHIP_KEY_QUERY` (`… WITH INDEX member_keys WHERE relation=$relation AND semantic_key=$key`) once per key. The code comments: "Scalar equality keeps member_keys on one prefix; no native union/distinct/order state."

**Library fit.** The same deterministic IDs work for owners × keys. `SELECT … FROM $ids` over the cross product is a multi-get (RocksDB `multi_get_opt`). It holds no union, distinct or order state, returns nothing for missing IDs, and needs one round trip per window of (owners × keys).

**Consequence.** S03 attach and S02 views with several contributing owners: the number of round trips equals the key count (operation map M3.3). This is per-item crossing (FP-07, H-level batching).

**Preservation constraints.** Missing-key handling, cross-owner conflict detection (`selected nominal backing collision`), global ordering and the setup/charge ownership.

**Uncertainty.** The typical owner count is unknown. The plan's SA5/SA-F04 targets this reader, so B and the coordinator should check whether SA5 already plans this.

**Label.** Proposed.

**Closure evidence.** A multi-owner, sparse/missing/conflict control with round trips counted.

---

#### L1-06: DataFusion's `parquet` feature is compiled with no consumer (seam to F1)

**Observed cause.**
- `Cargo.toml` declares `datafusion = {…, features = ["sql","parquet","nested_expressions"]}`.
- `rg -i parquet` over `crates`, `python` and `scripts`, including tests, finds only the family-pattern regex in `scripts/check_family.py`.
- `cargo tree --offline -p parquet@59.3.0 --depth 1` shows it pulls in flate2, zstd, snap, brotli and lz4_flex. Some of these may also arrive through other paths.

**Consequence.** S10: extra compile units in every cold build of the DataFusion consumers. The magnitude belongs to F1. Mechanism: DP-13 (unused capability cost).

**Alternative.** Drop the feature and check the DataFusion consumers compile. DataFusion spill uses Arrow IPC, not Parquet.

**Preservation.** None identified.

**Uncertainty.** Whether the codecs remain in the graph through other crates (rocksdb, for example), which limits the saving.

**Label.** Interface-checked for the absence of use.

**Closure evidence.** `cargo tree -e features -i parquet` empty after the change, and the build-time delta measured by F1.

---

### 4. Opportunities and uncertainties that are not findings

- **The serde_arrow cost is proportionate.** Upstream's own benchmark history, from 0.14.2 rather than the pinned 0.15.1 (`serde-arrow/content/corpus/history/0.14.2/Readme.md`), puts `ArrayBuilder` at about 2–3× hand-built derive builders. That is a CPU cost per batch, and it is dwarfed by the per-128-row gRPC and transaction structure.
  - The write path goes `Vec<R>` → Arrow → per-cell `surrealdb::types::Value` (`codec.rs::batch_bodies`). A direct `R` → `Value` (`SurrealValue` derive) would remove one representation on writes.
  - But canonicalization (`relation.canonical`) is defined over Arrow, and the read path needs Arrow for DataFusion. Not material without measurement.
- **Arrow columnar gRPC results are not available.** The pinned `surrealdb-protocol-0.13.1` defines `ArrowBatch` but documents it as "Not served yet". The patched SDK refuses it (`grpc.rs`: "Server sent a columnar batch, which was not requested"). The bespoke Value → Arrow `ProjectedBuilder` is therefore necessary at 3.3, which confirms plan §10's "raw Arrow protocol: not selected".
- **Native EXPORT/IMPORT versus the selected backup.** Native export (`surrealdb-core-3.3.0/src/kvs/ds.rs::export_with_config`) is snapshot-consistent: one read transaction. It filters by table (`ExportConfig.tables`, records on/off) but not by row closure. Native IMPORT replays live control tables and bypasses admission.
  - The selected-closure backup and admitted restore are justified by the "logical content restore performs independent admission" obligation (`backup.rs::recovery_metadata`). Native export suits only whole-database cold archives.
  - Side note: `SERVER_SETTINGS["SURREAL_EXPORT_BATCH_SIZE"]="1"` caps native export at one record per `INSERT`. Since import runs one statement per transaction (`kvs/export.rs` docs), a native import would take one durable fsync per record. Searching `scripts/` and `crates/` found no production native-export consumer (only SDK fault tests in `backup.rs`), so the setting is inert. If a native cold export is ever used, revisit it.
- **The DataFusion provider is adequate.**
  - `supports_filters_pushdown` returns Exact for interpretable predicates.
  - Projection is pushed natively.
  - `statistics`/`partition_statistics` report `num_rows`, exact only when unfiltered.
  - Output ordering on `id` is declared, so sorts are eliminated.
  - The `scan` `_limit` is ignored, but DF 55.1 `push_down_limit` keeps the outer limit and `NativeExec::with_fetch` forwards fetch to `StreamingTableExec`, which stops polling windows.
  - Remaining waste is at most one prefetched window plus membership preparation. The latter is not a limit-pushdown issue: SA5/B own it.
- **moka coalescing.** moka 0.12.16's `get_with`/`try_get_with` re-runs `init` in another waiter when the first caller's future is aborted, and panics after `MAX_RETRIES` (`moka-0.12.16/src/future/value_initializer.rs`). The bespoke owned flight in `PreparedCache` (a spawned loader, generation/closing fencing, charged waiters) gives guarantees moka does not: cancellation-independent ownership and charge retention. Using moka only as weighted retention is justified. Not duplication.
- **PyO3.** Every non-trivial entry point detaches (`py.detach` in `wire_decode`, `wire_tool_result`, `admit_envelope`, `NativeSession::{new, execute, close}`). The crossings are string copies only.
  - The six crossings with triple validation and re-encoding of the response are an MCP-layer choice (FastMCP/pydantic own the final stdio bytes, so the final `BoundedStdioWriter` check is necessary).
  - The pre-send `_admit` duplicates it so that a refusal can be returned instead. E owns that trade-off.
- **HTTP.**
  - `lctx-embed` reuses one `reqwest::Client` and sends batches of 32.
  - The MCP `HttpEmbedder.embed` opens a new `httpx2.AsyncClient` per query (`embedder.py`), so every vector search pays connection setup. A lifespan-owned client is a small S06 improvement, keeping the per-call timeout. Minor, E's area.
- **vLLM.**
  - Requesting full 4096 dimensions once and projecting the 1,024-dimension prefix locally is the right use of MRL: one call serves rescoring and search.
  - The local tokenizer replaces `/tokenize`, which is kept as a parity oracle (`lctx-embed/src/lib.rs` header).
  - Adequate.
- **tokenizers.** `LocalTokenizer::encode` encodes one text at a time. `encode_batch` (rayon) exists, but it applies only on the embedding path, and I did not trace the callers' loop.
- **blake3.** Used without `rayon`/`mmap`. For per-file source-closure hashing during a streaming copy, neither helps (inputs are small and streamed). The real cost is A's re-reading of the closure five times, not the hasher.
  - Per-effect `ContentHash::of(serde_json::to_vec(&(sql,&bindings)))` runs twice (operation map M1). That is serialization cost proportional to payload, which D owns.
- **BM25.** Native FULLTEXT scores use corpus statistics across the whole table, which is shared across publications. The bespoke exact-view BM25 (`lexical_stats.rs`: "FULLTEXT is nomination, never corpus authority") is necessary for pinned, view-exact ranking. I did not examine the `@OR@` nomination cost.
- **LIVE queries and change feeds.** No consumer. Plan SA8's trigger stands.
- **tracing.** Six spans outside tests and no subscriber in the `lctx` CLI, so the overhead is negligible. `datafusion-tracing` is unused, which is a diagnosability gap, not an efficiency one.
- **Unused patch file.** `third_party/datafusion-table-providers-df55.patch` has no manifest reference (`rg` over `Cargo.toml` files). It looks like an orphaned patch; housekeeping only.

---

### 5. Strengths and areas checked and found adequate

- **The concurrency design understands RocksDB optimistic semantics.**
  - It writes guards rather than relying on scans; read-only optimism is explicitly not treated as exclusion (plan SA8).
  - Conflict classification is exact-message and typed (`control.rs::retryable_conflict`, `cache.rs::retryable_conflict`), not substring guessing.
  - Payload retries are idempotent: `IF $__operation.resolved` short-circuits.
  - Keep all of this when adopting L1-01.
- **The SurrealDB integration is owned in one crate** (`lctx-surrealdb`), with the publisher and serving as consumers. The SDK patch is narrow and documented (`third_party/surrealdb/README.lctx.md`). Physical completion of the gRPC stream is checked.
- **The selected backup is coherent:** one client transaction across metadata, closure and content reads; staged bytes published only after checked EOF, cancel and invalidate; `persist_noclobber` plus a parent fsync.
- **The DataFusion provider contract is correct and conservative:** inexact statistics whenever a selection or filter applies, so a zero estimate never skips authority checks; ordering is declared.
- **Every PyO3 entry point detaches.** The serving runtime is per session, not per call.
- **Exact vector scoring with full-vector rescoring is a valid exactness choice.** The defect is the unused index and the design mismatch (L1-04), not exactness.
- **Durability:** `SyncMode::Every` with grouped commit, retained as the plan states. No weakening is proposed.

---

### 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| Plan SA8 "Coordination: keep installation ordering initially" | **Confirmed still present**; a new consequence was identified | `authorization.rs::ERA_GUARD`; RocksDB optimistic semantics (L1-01). The plan judged correctness, not contention |
| SA-F07 / SA7 "bounded lifecycle crossings" (window-level guards) | **Residual**: SA7 targets history, retirement and cleanup; ingress `ensure_rows_owned` and `hold_many` remain per-row loops | `control.rs::ensure_rows_owned` (L1-02) |
| SA-F04 / SA5 relation-aware membership access | **Confirmed still present** for the multi-owner explicit-key path | `compiler.rs::MEMBERSHIP_KEY_QUERY` (L1-05) |
| PC4 bounded external ordering ("useful lowering", plan §10) | **Confirmed still present**; the parameters are the issue | `ordered_rows.rs::RUN_BYTES`, `merge` (L1-03) |
| NE-F07 / SA8 cache: one live flight under eviction | **Refuted as a library gap**: moka coalescing lacks the needed cancellation and charge semantics | `moka-0.12.16/src/future/value_initializer.rs`; `preparation.rs` |
| Plan §10 "raw Arrow protocol: not selected" | **Confirmed**: not served at the 3.3 protocol | `surrealdb-protocol-0.13.1` `ArrowBatch` doc |
| Plan §10 "gRPC SDK streaming / `stream_items`" | **Confirmed adequate** | `grpc.rs` batch-frame decoding |
| Plan §10 "native durability Every; no weaker sync" | **Confirmed**; it multiplies L1-01/L1-02 commit counts but is not itself a defect | `kvs-rocksdb cnf.rs` default `SyncMode::Every`; `SERVER_SETTINGS` |
| Design "native HNSW candidate ranks" (product §14, ADR-0131) | **Refuted as implemented**: there is no KNN consumer | L1-04 |
| SA-F05 / SA6 lexical nomination and frozen scoring | **Not examined** beyond the necessity of view-exact BM25 | `lexical_stats.rs` header |
| SA8 Transport (patched gRPC retained) | **Confirmed adequate** within L1 scope | `README.lctx.md` |

---

### 7. Candidate rule impacts

1. **SA8 Coordination row** (`docs/plans/surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md` §6; persisted-graph plan §7.5): "Keep installation ordering initially."
   - Proposed change: permit a striped-fence protocol in which closure writes every stripe, which keeps the bidirectional optimistic conflict.
   - If the rule is kept: add bounded backoff only, and accept serialized native writes per database as a documented capacity limit for S01, S06 and S10.
2. **Design text on HNSW** (`docs/design/sections/api-and-evidence-product.md` §14, ADR-0131, `docs/surrealdb.md`).
   - Either amend it to "exact scoped scoring", if L1-04 option (a) is chosen;
   - or keep it and schedule the KNN route.

   The current state, where the design claims HNSW and the code is exact, is a divergence either way.

---

### 8. Proposed light probes (no build, service or fixture)

| # | Question | Command | Discriminating outcome |
|---|---|---|---|
| P-L1-1 | Is any KNN or HNSW consumer present anywhere, including generated SQL in Python and scripts? | `rg -n '<\|[0-9a-zA-Z$]\|vector::distance::knn\|WITH INDEX neighbor' --glob '!docs/**' --glob '!third_party/**' --glob '!target/**' /home/paul/library-context` | Any hit outside definitions refutes L1-04's absence claim |
| P-L1-2 | Do existing run logs show conflict retries? Silent retries would show only as error text when a retry finally failed | `rg -l 'Transaction conflict: Resource busy' /home/paul/library-context/build/runs/*/ 2>/dev/null \| head` | Hits confirm that conflicts occur in fixture runs. Absence proves nothing, because retries are silent |
| P-L1-3 | Is `parquet` reachable only through the DataFusion feature? | `cargo tree --offline -e features -i parquet` in `/home/paul/library-context` | Only a `datafusion feature "parquet"` edge confirms L1-06 is removable |
| P-L1-4 | Do the codecs stay without parquet? | `cargo tree --offline -i zstd -e normal; cargo tree --offline -i brotli -e normal` | Other parents bound L1-06's saving |
| P-L1-5 | Which effects contend? | `rg -n 'native_installation:current' /home/paul/library-context/crates --type rust -c` | Enumerates every writer that would move to stripes in L1-01; check each one's conflict obligation |

Closure for L1-01 and L1-02 needs instrumentation (retry counters, commits per window) under a fixture run, which is not a light probe. It is recommended as acceptance evidence, not as a review probe.
