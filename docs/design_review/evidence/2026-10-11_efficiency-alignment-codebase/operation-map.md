# Operation map (facts only)

**Provenance.** This is the verbatim output of one read-only `code-mapper` agent (Opus), produced
on 2026-10-11 for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md)
at baseline `88f33bc0`. Only this header was added.

It is orientation for the inquiries, not a judgment:
- round-trip counts were worked out from the code, not measured;
- "inferred" and "unresolved" items are the mapper's own labels;
- the principal review re-verifies anything decisive.

See the [evidence index](README.md).

---

# Operation map: efficiency-alignment review (facts only)

Baseline: `main` at `88f33bc0dd8bc67ef9291115c3a487b5f54bcb57`. Everything here comes from reading the code: rg, sed, git, no builds, tests, services or queries. Round-trip (RT) counts are worked out from the code, not measured. "Inferred" marks a statement that depends on runtime semantics I did not run. Line numbers are left out on purpose; owners are given as `path::symbol`.

Conventions:
- **RT** means one gRPC request/response with SurrealDB through `surrealdb::Surreal<Client>`. A streamed `stream_items()` query counts as one RT plus streamed delivery.
- **W** = `crates/lctx-surrealdb/src/loader.rs::NATIVE_WINDOW_ROWS` = **128 rows**. Native windows are also capped by `TransferLimits.bytes` = 8 MiB.

---

## 0. Shared mechanics (used by several operations)

### M1. One native "effect" = 4 RTs, with 3 writes to `native_installation:current`
**Owner:** `crates/lctx-surrealdb/src/control/authorization.rs::{effect_for_owner, execute_owner}`. Every `control::effect`, `ensure_rows`, `ensure_rows_owned`, `hold`, `hold_many` and `guarded_effect` goes through it.

| # | RT | Notes |
|---|---|---|
| 1 | `SELECT generation,era FROM $owner`, or `IssuanceEra::capture` for installation-owned effects | read |
| 2 | intent transaction (`BEGIN; ERA_GUARD …; CREATE $__effect …; CREATE $__outcome_ref …; COMMIT`) | `ERA_GUARD` = `SELECT … native_installation:current FOR UPDATE` + `UPDATE native_installation:current SET control_revision+=1` |
| 3 | payload transaction (`BEGIN; ERA_GUARD; … LET $__payload=|| {sql}; …; UPDATE $__effect SET committed=true …; COMMIT`) | second write to `native_installation:current` |
| 4 | outcome-ref release transaction (`UPDATE native_installation:current SET control_revision+=1; DELETE $reference`) | third write to `native_installation:current` |

- **Retries:** `control::run_transaction` loops on retryable transaction errors with `yield_now`. A failure adds `reconcile_request` plus `request_disposition` (two more RTs).
- **Fingerprinting:** each effect computes `ContentHash::of(serde_json::to_vec(&(sql, &bindings)))` twice, once in `effect_for_owner` and again in `execute_owner`. The cost grows with the payload size.

### M2. Native write of one typed batch
**Owner:** `crates/lctx-surrealdb/src/compiler.rs::NativeCompilerStore::write_batch_inner`.

In-process conversion order:
1. `Batch<R>` holds `Vec<R>` **and** its `RecordBatch` (`crates/lctx-model/src/domain/record.rs::Batch`; `R::encode` is `serde_arrow`, generated in `lctx-model-macros`).
2. `adapter.relation.canonical(batch)`.
3. `codec::batch_bodies`: Arrow → `surrealdb::types::Value`, one `Object` per row, built cell by cell.
4. `adapter.graph(&batch)` builds graph rows.
5. For each row, `serde_json::to_vec` of the canonical entity/assertion/body, then `ContentHash::of`.
6. `adapter::physical_row` builds the stored row.
7. `nodes`/`members` are kept as `BTreeMap<String, Value>`.
8. Special relations: `ArtifactChunk` is decoded again (`ArtifactChunk::decode`) and split into 64 KiB `original_chunk` rows; `SourceArtifact` is decoded again for the `original` headers.

Native calls, all per W window:

| table | effects per window | RTs per window |
|---|---|---|
| `original_chunk` (`persist_original_chunk`) | `ensure_rows_owned` + `hold` + `hold_many` | 12 |
| `original` headers (`ensure_immutable_rows`) | `ensure_rows_owned` + `hold` | 8 |
| physical nodes (`ensure_physical_rows`) | `ensure_rows_owned(anchors)` + `ensure_rows_owned(rows)` + `hold(ids)` + `hold_many(refs)`; an empty list skips its effect | 8–16 |
| `compiler_alias` | 2 | 8 (usually empty) |
| `compiler_membership` | 2 | 8 |

- **Inside `ensure_rows_owned`'s payload, per row:** `SELECT … FROM ONLY $guard FOR UPDATE`, `UPSERT native_guard`, `SELECT … FROM ONLY $row.id`, then `CREATE`/`INSERT RELATION` only if the row is absent.
- **Throughput figure:** a 4,096-row batch (default `WorkspaceOptions.batch_rows`) is 32 windows, so about 512 RTs for nodes plus 256 RTs for membership. Worked out from the code, not measured.

### M3. Native read of one completed view
**Owners:** `compiler_provider::NativeTable::scan`, `NativePartition::execute`, `scan_batches`, and `compiler.rs::{scan_rows, scan_bound_rows_inner, prepare_memberships, SelectedRows, MembershipLookup}`.

1. **Membership preparation.** For an unfiltered or SQL-filtered scan, `prepare_memberships` runs one streamed query per contributing owner (`SELECT … FROM compiler_membership WITH INDEX contribution_rows WHERE contribution=$owner AND relation=$relation`). It sorts the rows in `AsyncCandidateSort`, which runs on a `spawn_blocking` worker and can spill to a `tempfile::tempdir` (`ordered_rows.rs`). The result is cached per view identity in `NativeCompilerStore.membership_preparations` and reused across stages within the attempt.
2. **Payload windows.** `SelectedRows::start_payload` issues one query per W nodes (`SELECT <projected body fields> FROM $__compiler_nodes WHERE semantic_type=$relation AND (<pushed predicate>) ORDER BY semantic_key`).
3. **Explicit-key selection** (`MembershipLookup::collect`): a single-owner view costs one RT per W keys. A **multi-owner view costs one RT per key** (`MEMBERSHIP_KEY_QUERY`).
4. **Atomic-field selection:** one query per (owner × W values) against the scope table's `by_scope` index, with a correlated membership subquery.
5. **Value → Arrow:** `projected_arrow::ProjectedBuilder::push`, batched at `batch_rows` (4,096) or 8 MiB. Consumers then decode `R::decode` (serde_arrow → `Vec<R>`).

Filter pushdown: `SupportedFilter::interpret` → SurrealQL `NativePredicate`. Fetch/limit is not pushed down. The plan node is `StreamingTableExec` with one partition per scan.

### M4. Thread and runtime topology of the compile process
- **Main runtime:** `tokio::runtime::Runtime::new()`, multi-thread (`crates/lctx/src/main.rs`, `runtime` closure).
- **Workspace:** `cpg_core::workspace::Workspace::new` creates:
  - a DataFusion `RuntimeEnv` with a memory pool of `memory_bytes` and a disk spill dir in a `tempfile::tempdir`;
  - `target_partitions` = default;
  - a **`NativeBridge`**: a dedicated `std::thread` "lctx-native-provider-bridge" running its own **current-thread** Tokio runtime, fed by an `mpsc::channel(2)`, spawning each request into a `JoinSet`;
  - **`NativeCalls`**: `Semaphore(8)`, each call `tokio::spawn`ed on whichever runtime polls it (inferred: the bridge runtime for provider-thread writes, the main runtime for async stage writes).
- **Provider threads:** each facts provider runs on its own `std::thread` with a 512 MiB stack (`cpg_extract::bundle::run_provider`, `PROVIDER_STACK_BYTES`). Its synchronous writes block in `futures::executor::block_on` inside `NativeBridge::call_boxed` until acknowledged.
  - Provider join: `ProducerOutput::drain_provider` → `spawn_blocking(thread.join)`.
- **Pyrefly:** `pyrefly_stage::analysis_pool`, a process-static `pyrefly_util::thread_pool::ThreadPool::new(ThreadCount::AllThreads)`. `transaction.run` uses it.
- **Canonical preparation:** `NativeCompilerStore::prepared_canonical` uses `self.runtime.spawn` on the main runtime, as a shared future.

---

## 1. Cold catalog compile: `lctx compile fastmcp --profile catalog --through catalog` (native target)

Entry: `crates/lctx/src/main.rs` → `crates/lctx/src/compile.rs::compile`, with `Target::Native`. Default `Options`: `embedder = None`, techniques `"default"`, which turns every technique flag off (`lctx_model::domain::analysis::settings::Techniques::parse`).

### 1.1 Setup (`Phase "cli_setup"`)

| Step | Owner | Crossings | Live state |
|---|---|---|---|
| Read runtime config | `crates/lctx/src/newnative.rs::config` → `RuntimeConfig::read` | 1 file read | `RuntimeConfig` |
| Readiness | `newnative::ready` → `compiler::check_installation` → `reader::connect` | **new gRPC session #1**: connect + `signin` + `use_ns/use_db` (≈3 RTs), `version()` (1 RT), `control::check_installation` `SELECT … native_installation:current` (1 RT) | The returned `Arc<Surreal<Client>>` is discarded by `compile` |
| Model | `lctx_model::domain::model()` | CPU | `Arc<ValidatedModel>` |
| Store | `NativeCompilerStore::begin` → `begin_inner` | **gRPC session #2**: repeats `check_installation` (≈5 RTs), then `control::issue_attempt` = `IssuanceEra::capture` (1 RT) + `begin_attempt_in` transaction (1 RT, updates `native_installation:current.admission_revision`) | `Arc<NativeCompilerStore>` holding `Handle::current()` |
| Workspace | `Workspace::new` | tempdir, spawns the bridge thread and runtime | DataFusion `SessionContext`, `ResourceBudget`, `NativeBridge`, `NativeCalls` |
| Product cache | `lctx_surrealdb::NativeProductCache::connect` → `attach_partition` (only when `runtime.reuse` is configured) | `create_dir_all` and canonicalize of the lease dir; `lifecycle()` file lock; reads `administration_pending`; **gRPC session #3** via `check_installation` (≈5 RTs); one installation-owned `effect` (4 RTs, M1); `select product_owner` (1 RT); `select product_header` (1 RT) | `NativeProductCache` |

### 1.2 Configuration and acquisition (`Phase "configuration_and_acquisition"`)

| Step | Owner | Crossings |
|---|---|---|
| Upper config | `compile_options::Options::prepare` | reads `libraries/fastmcp/analytics.toml`; `embedder = None`, so **no HTTP to vLLM** |
| Native context | `cpg_extract::native_context::NativeContextConfig::committed` | CPU |
| Prepared compilation | `cpg_core::compilation::PreparedCompilation::new` | CPU; `Frontier::Catalog` adds selection, synthesis and retrieval definitions; `RetrievalDefinition::builtin(false)` |
| Embedding cache | skipped (`embedder` is None) | — |
| Acquisition lease | `crates/lctx/src/acquisition.rs::Lease::open` | **subprocess** `uv run --no-project --offline --no-python-downloads --python 3.14.7 python scripts/storage_acquisition.py` (uv plus a Python child), JSON request on stdin, one reply line on stdout. Request: `synchronize=true`, `reinstall=false`, `source` set |
| Helper | `scripts/storage_acquisition.py::Lease.prepare` (detail below) | flocks, durable JSON, maybe `uv sync`/git |
| Inventory | `cpg_extract::acquisition::inventory` → `library_inventory` | reads `pyproject.toml`, `uv.lock`, `.python-version`, `pyvenv.cfg`; reads every installed distribution's `RECORD`; walkdir over `site-packages` with stat only; corpus inventory for the source tree |
| Capture | `cpg_extract::acquisition::capture_receipts` → `CapturedInput::capture` | **copies every closure file** into a private `tempfile` dir (`lctx-input-*`) with a streaming `ContentHasher` digest, plus two stats per file and a re-stat sweep at the end; then **re-reads every copied file to compute SHA-256** against `RECORD`; corpus copy via `capture_derived` (derived doc blocks are written into the frozen copy) |
| Finish | `Lease::finish` | writes `{"action":"finish"}` to the helper, which runs `Lease.close` and exits; `child.wait()` |

`scripts/storage_acquisition.py::Lease.prepare`, in order:
1. `storage_lifecycle.admission([library, environment_root, source_root])`: one `flock(LOCK_SH)` per lexical ancestor of each path, sharing common prefixes, under `state/admission/<sha256(path)>.lock`.
2. `configuration()`: SHA-256 of `pyproject.toml` + `uv.lock` + `.python-version`.
3. Takes `hold_lock(owner.lock)`, then `prune_finished_leases`, which **globs and reads every `leases/*.json` file** (grows with lease history, retained up to `action_receipt_days`).
4. Per kind (environment, source):
   - a current group whose key matches is reused, after an `admission` on its path and a shared `flock` on its group lock; `check_source` runs for the source;
   - a miss creates a new generation: `durable_json` ×2 (each is a file fsync plus a directory fsync), `storage_owners.enroll` (a `Storage` publish: 3 `durable_json`), then a **subprocess** `uv sync --frozen …` or a git fetch, then `durable_json`.
5. **Always** (`synchronize=true`): `durable_json(record.json)` and `durable_json(leases/<id>.json)`.

`Lease.close(acknowledged=True)`: `hold_lock(owner.lock)` + `durable_json(lease)`, then per group: `required_record`, plus `generation_busy` (a non-blocking flock probe) for unselected groups.

### 1.3 Facts (`cpg_core::compilation::compile_driver` → `Phase "providers"`)

1. **Setup** (`Phase "compiler_setup"`): `set_frontier`, `Workspace::captures`, `PreparedCompilation::schedule`. The schedule covers 6 facts providers, the 9 `Normalization::ALL` stages and all 21 `UpperStage::ALL` bindings, because the catalog frontier admits Analysis- and Catalog-phase stages.
2. `cpg_core::facts::compile_facts`:
   - **`input.captured().verify()`** re-reads and re-chunks every captured file (`CapturedInput::emit_chunks`; this is the third full read of the closure).
   - `contracts::captured_sources` runs `pyrefly_stage::analysis_context` per input.
   - `Schedule::build`, then the providers run **sequentially** in schedule order (`cpg_extract::contracts::composition`): Acquisition, Pyrefly, TyFlow, Documents, Deployment, Assembly.
3. **Per provider:**
   - `workspace.inputs(...)`, `workspace.producer(...)` (`ProducerOutput`);
   - `bundle::run_provider` runs `provider.run(&mut StageContext)` on its own provider thread (M4) and awaits a `oneshot`;
   - `ProducerOutput::complete`.
4. **Acquisition provider:** `acquisition::Acquire::run`. Its `emit_chunks` per captured artifact is the fourth full read of the closure. It also calls `frozen.verify()` (`cpg-extract/src/acquisition.rs`), which is **a fifth full read and chunking**.
5. **Pyrefly provider** (`pyrefly_stage::session`), per captured input:
   - `std::fs::read` of every root module (UTF-8 check);
   - one `pyrefly::state::State` with one `transaction.run(handles, Require::Everything, analysis_pool())` over all modules (AllThreads pool), plus a Pysa reporter configured with `write_files: false`;
   - extraction then emits rows. Embedded Ruff context and lexical outputs are contributed.
6. **Row path** (`StageContext::emit` / `contribute` / `emit_batch`):
   - provider-side `lctx_model::domain::batching::BatchWriter<R>` (4,096 rows or 8 MiB; keeps a `seen: HashMap<Id<R>, ContentHash>` for duplicate suppression across flushes);
   - `ProducerOutput::write` → `Writer<R>::flush` / `write_batch`;
   - `owned_batch_write` → `NativeBridge::call_boxed` (`futures::executor::block_on`; `mpsc(2)` into the bridge runtime);
   - `NativeCalls::call_boxed` (`Semaphore 8`, `tokio::spawn`) → `NativeCompilerStore::write_batch` (M2);
   - `ProducerOutput::push_sync` accumulates in the workspace `Writer.pending` (`batch_rows` / `memory_bytes/8`, capped at 8 MiB) and flushes through the same path. `ProducerOutput::push` (async) uses `submit_batch_write` directly.
   - **Live representations per in-flight batch:** the provider-side `BatchWriter.pending: Vec<R>`, then `Batch { rows: Vec<R>, arrow: RecordBatch }`, then the canonicalised `RecordBatch`, then `Vec<Value>` bodies, then the `nodes`/`members` maps. Up to 8 native calls are in flight; the provider blocks per batch.
   - Contribution registration: `ProducerOutput::registration_request` → `NativeCompilerStore::begin_contribution` → `guarded_effect` (4 RTs) per producer.
7. **Completion** (`ProducerOutput::complete`):
   - closes the writers, flushing the remaining pending batch;
   - `producing_scope.close_and_wait`, then the `completion_gate` mutex;
   - `NativeCompilerStore::complete_contribution_scoped` → `complete_contribution_inner`:
     - re-reads **all of the contribution's membership rows** in one streamed RT (`SELECT relation,semantic_key,content FROM compiler_membership WHERE contribution=$contribution ORDER BY relation,semantic_key`), hashing them into per-output `KeySink` digests;
     - for shared relations that already have rows, `new_membership_keys` runs once per W keys;
     - `control::effect` to mark the contribution completed (4 RTs);
     - `registered_or_install_views`;
   - `completed_contribution(id)` read-back (≥1 RT);
   - for each completed relation, a `CompletedBinding` is bound (`native.bind` → `bind_inner`).
8. **After the providers:** `workspace.facts_availability_async(profile)`, then `freeze_completed_inputs`. For each publication group whose stages are all complete, `Workspace::freeze_inputs_async` runs `native.bind(CompletedBinding{boundary})` once per epoch-shared completed relation.

### 1.4 Normalization: the 9 stages of `normalize::pipeline::Normalization::ALL`

Order: Entities, Relations, Callables, Receivers, CallableAspects, Events, Bindings, Projections, Coverage.

Per stage (`compile_driver`, normalization loop):
1. `Workspace::stage_inputs_selected` builds a `CompletedInputs`, then `ProducerOutput`, `product_request()` (a `KeySink` over model digest, implementation, configuration, profile and the input view identities), `products.bind_selected`.
2. **Reuse.** `Normalization::run` calls `ProducerOutput::reuse_product` when `reusable_rows()` holds (Entities, Relations, Callables, CallableAspects, Projections). Receivers, Events and Bindings use `reuse_product_checked_async` inside `normalize::{receivers_produced, events_produced, bindings_prepared}`. Coverage never reuses. On a cold run, every lookup is the single RT miss in `attach_retained_product` (`SELECT contribution.* AS stored FROM native_product WHERE request=$request AND contribution.completed=true AND contribution.attempt.admitted=true ORDER BY id LIMIT 1`; `native_product` has an index `request_product`). A miss sets `product_capture`. After `complete`, `Workspace::retain_product` → `retain_product_identity` adds `completed_contribution` (≥1 RT) + `ensure_rows` (4) + `hold` (4).
3. **Read path:**
   - `CompletedInputs::session` creates **one DataFusion `SessionContext` per `CompletedInputs`** (OnceCell `prepared_session`), with one `NativeTable` (`compiler_provider::table_provider`) registered per input binding, plus plain-name aliases;
   - `cpg_core::sql::query` (`ctx.sql_with_options`, read-only) or `query_template` (cached logical plan templates);
   - DataFusion execution → `NativeExec` / `StreamingTableExec` → M3 reads (RT per W rows);
   - `R::decode` (serde_arrow) into typed rows;
   - the stage kernel (normalize module);
   - write-back through `ProducerOutput::push` / `write` (M2).
4. **Per-root kernel product cache** (only when `NativeProductCache` is attached, and only on a whole-stage miss): `normalize/mod.rs` entity, callable and aspect `produce`. **Per selected root:**
   - `kernel_products::{entity_hit, callable…, aspect…}` → `NativeProductCache::lookup`: a shared `lifecycle()` flock, an entry-file flock, `current()` (1 RT), `entry()` (1 RT), then a `decode`;
   - the hit path runs `decode_product_rows`: JSON → `Vec<Value>` → `codec::decode_bodies` (Value → Arrow) → `hash_rows` → `canonical_bodies` (Arrow → Value) for comparison;
   - a miss calls `retain_*` → `NativeProductCache::insert`, which encodes `R::encode` → `codec::batch_bodies` → `serde_json`.
5. **After each stage:** `freeze_completed_inputs` and `products.complete(...)`.

Then `Phase "normalized_admission"`: `Workspace::admit_semantics(profile)`.
- `inputs("semantic-admission", …)` covers all completed relations, plus `bind_admission_epochs`.
- `validate_scope` → `CompletedInputs::session` (a fresh `SessionContext` with every completed relation registered) → `validate_references`.
- Then, **per model invariant in scope**: a `SELECT id … LIMIT 1` emptiness probe or a full check, both via `sql::query` over `NativeTable` (M3).
- Checked premises are memoised in `Workspace.checked_premises`, so an invariant whose premise was already checked is skipped.

### 1.5 Upper stages under `--profile catalog --through catalog`

The driver **runs all 21 bindings**; none is skipped by the scheduler (`compile_driver` upper loop). Per stage:
1. `stage_inputs_selected`, `producer`, `product_request`.
2. `UpperStage::reuse(effect, profile)`. If eligible (`Rows`): `reuse_product` (1 RT miss on a cold run).
3. On a miss: `PreparedGraphs::load` **once**, at the first stage with non-empty `graphs(profile)`. Under catalog with default techniques that is **Structural**: CallableInvocation + DefinitionContainment projections. `Analytic` needs graphs only when analytics are requested, and they are not.
4. Graphs are retained or released by `preparation::Needs::remaining` after each stage; bindings are always `None` under catalog.

| UpperStage (`name()`) | Reuse eligibility under catalog | Catalog behaviour found in code |
|---|---|---|
| Configuration `analysis_configuration` | Fresh | runs (`analysis_prepare::configuration`) |
| Native `analysis_native_inventory` | Fresh | runs |
| EmbeddingConfiguration | Fresh | runs with `prepared.embedding = None` |
| Text `analytic_text` | Rows | `TextDefinition.requested=false`: pushes the definition and finishes Complete (`analytic_text::publish`) |
| CatalogCore | Rows | runs |
| CatalogEvidence | Rows | runs |
| Local `analyze_local` | Rows (catalog) | runs over all provider runs; outcome `AnalysisStatus::NotRequested` (`local_semantics.rs`); returns `None` |
| Base `evaluate_base` | Rows (catalog) | reduced catalog path (behavioural inputs skipped); returns `None`; status set in `lctx-model` `execution::production` (`profile != Behavioral` → `NotRequested`) |
| Completion `complete_base` | Rows (catalog) | reduced; `completion_production`: `NotRequested` when not behavioral |
| SourceCalls `prepare_source_calls` | Rows (catalog) | reduced catalog inputs (`SourceCallData::consumed_inputs(profile)`); outcome status not traced (**unresolved**) |
| Enriched `enrich_execution` | Rows | catalog inputs only (`enriched_production::consumed_inputs`); status not traced (**unresolved**) |
| Models `apply_models` | Rows | catalog branch; status not traced (**unresolved**) |
| Summary `analyze_summaries` | Rows | `summary_replay` produces with `Profile::Catalog` when the run is not requested |
| Structural `analyze_structural` | Rows | **runs with prepared graphs** |
| Embedding `analytic_embedding` | Fresh ("own inference cache") | not selected, so no service session is opened |
| Analytic `analyze_analytic` | Rows | `analytics::build::not_requested` (all techniques off) |
| Selection `catalog_selection` | Rows | runs |
| Synthesis | Rows | runs |
| Retrieval | Rows | runs with `RetrievalDefinition::builtin(false)` (no embedding) |
| AnalysisFrontier | Fresh | runs (`final_coverage::produce`) |
| CatalogFrontier | Fresh | runs |

Then `Phase "compiler_completion"`: `Workspace::finish_compilation`. This is CPU only: `self.identity()` plus a `CompilationCompletion` record.

### 1.6 Artifact admission: `cpg_core::artifact::admit`

1. `Workspace::freeze_for_admission` → `NativeCompilerStore::freeze_content`:
   - closes the content lane and waits until mutations reach zero;
   - pending-contribution probe (1 RT);
   - `contributions_inner` + `bindings()` reads.
2. `facts_availability_async`.
3. **`admit_semantics` a second time**, with memoised premises skipped.
4. `admit_frontier`.
5. `NativeCompilerStore::prepare_canonical` (spawned shared future on the main runtime): `bindings_inner`, `selection::SelectedPayloads::for_views`, `discover_graph(entities)` + `discover_graph(assertions)` + `.prepare()` (sorted candidates).
6. `native_canonical::Lookup::load`.
7. **Full scan of entities, then assertions** (`native_canonical::{entities, assertions}` → `scan_canonical`): per row `admit_entity`/`admit_assertion` and `FamilyHasher::push`. Derivation proofs are collected in `Vec<GraphDerivation>`, then `admit_graph_derivations`.
8. Manifest: `artifact_manifest::populate`; the originals list is built from the captured inputs (in memory); `mark_attempt_admitted` (native effect).

**Live at this point:**
- `AdmittedArtifact` (manifest, `original_keys`);
- the prepared canonical candidate state inside the store;
- `membership_preparations` per view;
- the `Workspace` with all `CompletedRelation`s.

### 1.7 Publication and seal: `lctx_publisher::seal_completed` (`crates/lctx-publisher/src/lib.rs`, `native_publication.rs`)

| Step | Owner | RT structure |
|---|---|---|
| `Publication::new` | `native_publication.rs::Publication::new` | `bindings()` read; **gRPC session #4** (`reader::connect`, ≈3 RTs); `Loader::for_attempt_views` |
| `install_definitions` | `definitions::verify_epoch` | reads and compares the installed function epoch, no DDL; RT count not traced |
| Entity references | `artifact.entities()` (a **second full canonical entity scan**), buffered to 128 → `Loader::entity_references` → `flush_references` | per 128 entities: `ensure("external", endpoints)` per W (`ensure_window`: `SELECT * FROM $ids` + `ensure_rows`) and `insert("reference", edges)` per W (`ensure_rows(anchors)`, `ensure_rows(rows)`, `hold_many`, so ≤12 RTs per W) |
| Assertion references | `artifact.assertions()` (**second full assertion scan**) → `Loader::assertion_references` (participants + references) | same window structure |
| `materialize_search` | `lctx_surrealdb::derived_search::materialize_search` | see below |
| `seal` | `Publication::seal` | see below |

`derived_search::materialize_search(loader)`:
1. `lower(reader, expected, Some(loader))`:
   - `lower_vectors`;
   - stream all `SearchWindow` records, in batches through `window_batch`, with `Basic::load` → expected rows;
   - `Expected::finish(index)` for 5 family tables, each writing via `derived_search::Batch` → `Loader::insert` (≤12 RTs per W; `BATCH_ROWS` 128, 1 MiB);
   - **a second stream of all `SearchWindow`s** with `Basic::load` + `Companions::load` → `emit_witness` → `lex_occurs` and `vec_occurs` batches.
2. `Expected::reconcile(reader)`: reads the actual rows and compares.
3. `lexical_stats::materialize`.

`Publication::seal`:
1. `begin_finalization` (drains every operation).
2. `completed_state_after_closure` compared to `manifest.completed_state`.
3. `Loader::reconcile(manifest)` (`reconciliation.rs`): **a third full read of entity and assertion payloads** (`selected_payload_rows`), decode, recompute `entity_views`/`assertion_views` and the expected edges into a `SortedRows` (may spill), plus a family-hash comparison.
4. `derived_search::reconcile_search`: **a second full cold lowering** (two more `SearchWindow` streams, no writes), `Expected::reconcile`, `lexical_stats::reconcile`.
5. `seal_staging`:
   - `verify_epoch` again;
   - `hold(publication → views)` (4 RTs);
   - stream `SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt` and `hold` per 128 roots (4 RTs each). This **grows with every object held by the attempt**;
   - marker `ensure_rows` (4 RTs);
   - `close_attempt`;
   - `invalidate`;
   - readback: **gRPC session #5** `NativeReader::connect` (pin acquire; see §3.1), then `close` + `invalidate`.

Then back in `compile`: `workspace.drain_report()`, and the result is printed.

**gRPC sessions per cold compile:** ready, store, product cache (if configured), publication, readback; that is 4–5.

---

## 2. Unchanged warm recompile (same command, existing product cache)

Everything below comes from reading the code; nothing was run.

| Stage | Warm behaviour |
|---|---|
| Setup | identical to cold (§1.1): new attempt issued; product cache attach re-runs |
| Acquisition helper | environment group matches (`key == configuration()` hash, `cleanup=="confirmed"`, path exists), so **no `uv sync`**; the source group matches, so `check_source` runs. Still: admission flocks, `prune_finished_leases` lease scan, `durable_json(record.json)` + lease receipt, and close `durable_json` |
| Inventory and capture | **identical full work**: RECORD reads, walkdir, full copy and digest of the closure, SHA-256 re-read against RECORD |
| Facts providers | **always re-run in full.** No product-reuse path in `facts::compile_facts`, `bundle::run_provider`, `pyrefly_stage.rs` or `ty_flow.rs` (search coverage in §6). The 5 full closure reads, pyrefly and ty analysis, and row emission all repeat |
| Facts writes | a new contribution identity per attempt (`attempt_contribution(attempt, spec)`). Server-side existence check in `ensure_rows_owned`: existing identical payload rows are not re-created, but **per row the guard is still `UPSERT`ed**, and `hold`/`hold_many` rows are created for the new owner. `compiler_membership` rows are new (contribution-scoped IDs). The RT count per window is the same as cold (M2) |
| Facts completion | the same membership re-read and digest. A view's identity is `CompletedView::new(relation, {logical contribution identity}, rows)`; the logical identity hashes `semantic_key`+`content` (and `CompletedContribution` spec/outcome), so unchanged outputs give equal view identities |
| Reuse eligibility | `ProductRequest` identity = operation, model digest, implementation, policy revision, result contract, configuration, profile and **input view identities** (`workspace_products.rs::product_request`). Lookup: `attach_retained_product` on `native_product.request` (indexed), requiring `contribution.completed` and `contribution.attempt.admitted=true` (set by `mark_attempt_admitted` during the prior run's admission). `descriptor.spec != spec` gives a miss |
| Hit path ("checked attachment reuses compatible admitted content", ADR-0145) | `NativeCompilerStore::attach_retained_product`: `validate_inputs`; `hold(attempt → source contribution)` (4 RTs); `guarded_effect` creates a new contribution copied from the source (4 RTs); `hold_contribution_inputs`; **streams all source membership rows** and rewrites them as new deterministic-ID membership rows in W windows via `retain_attached_members` (inferred ≈8 RTs per W); effect `completed=true` (4 RTs); `registered_or_install_views`. **No payload replay**, but O(rows) membership pointer copying |
| Checked hits (Receivers, Events, Bindings) | additionally `scan_contribution_batches` per output relation (M3 read into `Vec<RecordBatch>` in a `ProductCandidate`), then the stage's `reuse::{receivers,events,bindings}`: `normalize/reuse.rs::inputs` builds a `predicate_session` (a new `SessionContext` borrowing providers) + `MemTable`s of the candidate batches, and reruns the owner predicates |
| Normalization stages that re-run | Coverage (not `reusable_rows`); any stage whose request misses |
| Upper stages | eligible "Rows" stages hit via attach. **Always fresh:** Configuration, Native, EmbeddingConfiguration, Embedding, AnalysisFrontier, CatalogFrontier. `PreparedGraphs::load` only runs when a graph-needing stage misses (Structural hit, so no load; inferred) |
| `admit_semantics` ×2, `prepare_canonical`, full entity/assertion admission scans | **re-run in full**; they are not memoised across attempts |
| Publication | `seal_completed` **re-runs completely**: references, `materialize_search` (full lowering; `Loader::insert` existence checks skip existing rows, but RTs are still issued per window), `reconcile`, `reconcile_search`, `seal_staging`. Publication identity = `publication_identity(manifest content, realization, view identity, service generation, definition epoch)`; unchanged inputs give the same handle (inferred). `ensure_rows` on the marker finds an identical existing row (no create). The retained-root transfer still holds every attempt-held object |
| Portable product cache | per-root kernel lookups happen only on a whole-stage normalization miss (§1.4.4); on a warm whole-stage hit they are not reached |

---

## 3. Serving through MCP

### 3.1 Server startup

| Step | Owner | Crossings |
|---|---|---|
| CLI | `python/lctx_mcp/src/lctx_mcp/__main__.py::main` → `create_server` → `anyio.run(run_stdio)` | — |
| Tool registration | `lctx_mcp.wire.register` | PyO3: `wire_tools()` (1), `wire_tool(name)` per tool, `wire_resources()` (1), plus `wire_tool("get_capability")` |
| Lifespan | `create_server.lifespan` → `anyio.to_thread.run_sync(NativeSession, config)` | thread hop |
| Session | `python/lctx_semantics/src/session.rs::NativeSession::new` (inside `py.detach`) | `ViewerConfig::read` (file); builds a **multi-thread Tokio runtime** per session |
| Reader pin | `NativeReader::connect` (`crates/lctx-surrealdb/src/reader.rs`) → `reader::connect`, then `NativeReader::open` | gRPC connect + signin + `use_ns/db` (≈3 RTs); `control::check_installation` (1 RT); `SELECT handle,views,manifest,definition_epoch FROM $publication` (1 RT); manifest decode; `ReaderPin::acquire` → `control::pin_views`: installation-owned effect (4 RTs) + `hold(pin → views)` (4 RTs); `pin.protect(publication)` → `hold` (4 RTs) |
| Service | `lctx_serving::service::NativeService::new` | no I/O: `ResourceBudget::fixed(shared_bytes)`, `Semaphore`s (`query_connections`, `cpu_jobs`), `PreparedCache` (moka), `RankedResults` |
| Definition check | `NativeService::run` → `definition: OnceCell` | **lazy, on the first request**: 1 RT `RETURN <operation_definition_function>()` compared to `operation_definition()` |

**Live state for the session's lifetime:** one gRPC session, the reader pin, `ReaderScope.originals = manifest.originals` (the full originals list in memory), `PreparedCache`, `RankedResults`.

### 3.2 `search_operations` (lexical + vector, embedder configured)

**Python side** (`lctx_mcp`):
1. `EnvelopeAdmission.on_request`.
2. `on_call_tool` → `_admit` (pydantic `model_dump_json` of a refusal envelope + PyO3 `admit_envelope`).
3. `SchemaTool.run` → `Executor.execute` → `NativeExecutor.execute`:
   - PyO3 `wire_decode` (validated JSON string in and out);
   - `asyncio.Semaphore(2)` admission (1 s wait);
   - task `_execute`.
4. **HTTP to vLLM:** `HttpEmbedder.embed([query_text])` (`embedder.py`) opens **a new `httpx2.AsyncClient` per call** and makes one POST. The reply is parsed into a numpy array, then serialised to `vector_json` (a JSON list of 4,096 floats plus metadata).
5. `asyncio.to_thread(session.execute, tool, request, vector_json, remaining_ms)`.

**PyO3** (`NativeSession::execute`, `py.detach`): `serde_json::from_str(vector_json)` into `QueryVector`, then `runtime.block_on(NativeService::execute_encoded_for)`.

**Rust** (`NativeService::run`):
1. `admit_request`, `decode_request` (the request JSON is decoded again).
2. `RequestAdmission` (query and cpu semaphores), then a request `ResourceBudget::scoped`.
3. Definition OnceCell.
4. `ranked.resume` (no cursor, so `None`).
5. Vector validation: `reader.records::<EmbeddingSpec>` (point read, ≥1 RT); `records::<ProjectionDefinition>` (≥1 RT).
6. `pagination::validate`, then `operations::dispatch` → `search_operations`.

`operations.rs::search_operations`:

| Sub-step | Owner | Reads |
|---|---|---|
| Domains | `library::resolve_prepared` → `PreparedCache` (moka; first use hydrates via `CanonicalBatches::canonical_point_batches`, 1 RT per 128 IDs, canonical JSON bytes → typed `RecordBatch`) | cached across requests |
| Members | `selection::members` | **per request, not cached**: `query_stream(library_packages_sql)` + candidate collection into `AsyncCandidateSort` |
| Classification | `selection::classify_prepared` | `PreparedCache` |
| Scores | `operations::scores`: for each of **4 families** (ApiOptions, DocumentationDeployment, Scenario, Source), `search::lexical`, then `search::vector`; then `search::rescore_union` | below |
| Fusion and ranking | `fusion`, `pagination::ranked` → `RankedResults` retention (`admit_copy`) | in memory |

`search::lexical_scoped`, per family:
1. `lexical_stats::scope_identity`, then `derived_search::lexical_occurrences`, which runs as an owned producer task with an mpsc sender:
   - FULLTEXT nomination `SELECT id FROM search_<family> WITH INDEX lexical WHERE text @OR@ $query` (1 streamed RT);
   - per 128 nominated documents: `SELECT … FROM lex_occurs WITH INDEX document_occurrences WHERE in IN $documents AND (ELIGIBLE)` (1 RT);
   - then **3 exact-equality queries** (`exact_name`, `exact_path`, `exact_option`), 1 RT each.
2. `hydrate_occurrence_nominations`, per 128 nominations:
   - `NativeReader::selected_membership` (`tokio::spawn`) → `SelectedPayloads::sparse_membership`: per W nodes **× per publication view**, one `compiler_view_member WITH INDEX view_nodes` query, plus ancestry iteration for entity nodes;
   - `SELECT * FROM $nodes` payload hydration (1 RT);
   - rows go into `SortedRows` (may spill to a tempdir).
3. `occurrence_frontier`: `SortedRows` for occurrences + documents, in batches of `POINT_BATCH`=128 (for `BriefOrigins`, also unit/origin point reads).
4. `score_documents`: per 128 documents, one BM25 SurrealQL `RETURN { … }` block (1 RT; per-document correlated `lexical_member` lookups server-side). Keeps a frontier ≤1,024.
5. `rank_occurrences`: up to 4 rewinds over the occurrence rows (tiers 128/256/512/1024), in memory.

`search::vector_scoped`, per family:
1. Prefix projection to 1,024 dims (`embedding::projection::project_prefix`).
2. `selected_frontier` → `derived_search::selected_occurrences(vec_occurs, ELIGIBLE)` → `indexed_occurrence_rows` (`WITH INDEX eligible_input`, 1 streamed RT) + `hydrate_occurrence_nominations`, as above. This **nominates all eligible `vec_occurs` rows for the selected inputs and pairs**, not only the near ones.
3. `score_documents` with `vector_selection_sql(1024)`: 1 RT per 128 documents.

`search::rescore_union`:
1. Per 128 nominated occurrences: `selected_occurrences(vec_occurs, …window_keys…)` (`WITH INDEX window_occurrences`) + `hydrate` + per batch `SELECT id,full_key FROM $documents`.
2. Per 64 full keys: `state::records::<FullValue>` and `records::<EmbeddingSpec>` (for missing encoders).
3. Each vector is decoded with `decode_vector` (4,096 floats) and the cosine is computed in Rust.

**Response:**
1. `delivery::finalize`, `response.encode_json` (byte-capped) → `EncodedJson` → Python `str` (`PyString::new`).
2. Python: PyO3 `wire_tool_result` (Rust validates and re-encodes) → `CallToolResult.model_validate_json` (pydantic) → `ToolResult`.
3. `on_call_tool._admit` → `model_dump_json` envelope + PyO3 `admit_envelope`.
4. `BoundedStdioWriter.send` → `model_dump_json` again + PyO3 `admit_envelope` → stdout.

**PyO3 crossings per tool call:** `admit_envelope` (pre-check), `wire_decode`, `execute`, `wire_tool_result`, `admit_envelope` (result), `admit_envelope` (send) = **6**. **Thread hops:** asyncio → `to_thread` → Rust `block_on` on the session runtime → `tokio::spawn` tasks (`selected_membership`, the owned `NativeRows` drivers).

### 3.3 `get_evidence` on a hit
`operations::dispatch` → `evidence::get` (`crates/lctx-serving/src/evidence.rs`):
1. `preparation.hydrate_scope(...)` for the target source plus `source_evidence::inputs()` / `owner_inputs()`, through `PreparedCache` (`CanonicalBatches`, 1 RT per 128 IDs, into a typed `RecordBatch`).
2. `originals::Prepared::new`.
3. `body(...)`: original bytes. The `original_chunk` reads are in the originals owner; the exact query was not traced (**unresolved**).
4. `source_evidence::sections` (flow inventory, source characterization).
5. `derivations(...)`.
6. `defaults::qualified` + `defaults::read_originals`.

No embedder call: the tool is not a search tool. The same 6 PyO3 crossings apply.

### 3.4 Continuation page (cursor)
- `NativeExecutor._execute` **skips the embedder** when `page.cursor` is present, so vector JSON is `None`.
- In Rust: `NativeService::run` → `RankedResults::resume`:
  - decodes the cursor (hex JSON);
  - `cache.lookup(result)`, an in-memory retained ranking, checked against the session and digest;
  - re-checks the vector channel identity only if a vector is supplied (here none);
  - slices `entry.rows[start..end]`, then `delivery::finalize`, `encode_json`.
- **No store RT** on this path once the definition OnceCell is set (search coverage: `ranked_results.rs::resume` and the `service.rs::run` early return).
- A cursor whose retained entry has been evicted returns `unavailable`. Search tools never re-execute on a cursor miss.

---

## 4. One fixture-backed boundary: `just verify --select store:rust`

The `store:rust` boundary (`scripts/verify.py::BOUNDARIES`) has `requirements={"native-store"}` and `fixture=True`. Its steps:
- `build-cli`, cargo, `when="cli"`: **skipped** unless `--cli` is given;
- `nextest`, packages `lctx-surrealdb` and `lctx-publisher`.

### 4.1 Process chain
1. `just` → `uv run --no-project --offline --no-python-downloads python scripts/verify.py --select store:rust` (processes: just, uv, python #1).
2. `verify.main` (#1): `legacy` → `parse_args` → `resolve`.
   - No nextest target args, so `cargo_targets()` (`cargo metadata` subprocess) is **not** called.
   - `base_environment` → `build_environment.normalized_env`; `target_directory` reads `.cargo/config.toml`.
   - `runs.current_run()` is None, so **`wrap_in_run`: `os.execv` into `scripts/runs.py run --label … -- python verify.py …`** (same PID, now runs.py).
3. `runs.cmd_run` → `new_run_dir` (`admission([run_dir])`, `mkdir`, `storage_owners.enroll` → `Storage()` + `publish`) → `Owner.run`:
   - `admission([run_dir, native env path, vllm env path])`;
   - `enroll` (second publish, same path: an update);
   - `_run_owned`: `hold_lock(owner lock)`, **Popen python verify.py (#2)**;
   - `_own` polls every **0.1 s** (`POLL`): it reads the progress JSON and writes `record.json` (fsync'd `write_json_atomic`) when the fold changes.
4. `verify.main` (#2) repeats parse and `resolve`.
5. `optional_owner(run_dir)` → `admission` (3 paths) → `CaptureOwner.__init__`: 2 `storage_owners.enroll` + 2 `write_json_atomic` + a symlink.
6. `execute` → `run_boundary` → `_run_boundary`:
   - `environment_owner({"native-store"})` → `workspace_env.ownership("shared", [])`, with **no resources** (empty `resources_for`), so no environment locks; `admission([])` creates the state directories only;
   - `observe_requirements` → `surrealdb_fixture.substrate_readiness("native-store")`;
   - `real_fixture` → `Server.open` + `Attachment.create`;
   - `_run_steps` → evidence `start`, `launch` (**Popen** the nextest argv), `command_finished`, `server_end_outcome`;
   - `Attachment.release`, `close_boundary`;
   - `Summary.update` / `finish` (write `summary.json`); `runs.set_progress` (read and fsync-write the progress JSON) on each progress change.
7. `Owner.run` finalization: `storage_owners.complete` ×2; because `verify-evidence.json` exists, **`Storage().records()`** runs.

### 4.2 Subprocesses per boundary

| # | Subprocess | Owner |
|---|---|---|
| 1 | `uv run …` | justfile |
| 2 | `runs.py`/`verify.py` (execv, same PID), then **python verify.py #2** | `verify.wrap_in_run`, `runs.Owner._run_owned` |
| 3–5 | **`surreal version`** ×N (N = 4 or 6, below) | `surrealdb_service._installed_binary` → `verify_binary`, or `surrealdb_server.validate_descriptor` → `execute` |
| — | `systemctl --user show` ×(2 + 1 per step) | `Installation.check_daemon` ×2 (readiness, `borrow`), `Server.exited` per step |
| — | `git ls-files -z --cached --others --exclude-standard` and `git rev-parse HEAD`, **×2 per step** (context before and after) | `verify_evidence/context.py::SourceObserver.observe` |
| — | `cargo nextest --version` | `verify_evidence/nextest.py::prepare` |
| — | `<.venv>/bin/python config_helper.py` (nextest config composition, under `workspace_env.ownership("shared","tools")`) | `nextest.prepare` |
| — | **`cargo nextest run --cargo-profile <profile> -p lctx-surrealdb -p lctx-publisher …`**: Cargo build of both crates' test targets, then one process per test (nextest model). Test functions counted with `#[test]`/`#[tokio::test]`: about 236 in lctx-surrealdb and 49 in lctx-publisher. That count reads the working tree, which has uncommitted publisher test edits, so it is approximate | `verify.launch` |
| — | `cargo nextest store list`, `store info <id>`, `store export <id> --archive-file …`, `store info <zip>` (4) | `nextest.finalize` |

**`surreal version` and full executable SHA-256 per boundary.** Both happen at once in `_installed_binary` and `_owned_server_generation` / `validate_descriptor`, at these call sites:
- `substrate_readiness`: `Installation.load` (validates the generation descriptor only if the install uses an owned generation binary), then `check()` → `check_daemon` → `_installed_binary`, then `_server_identity` → `_installed_binary`;
- `Server.open` → `Installation.load` (same conditional);
- `Attachment.create` → `service.borrow` → `installation.check()` → `check_daemon` → `_installed_binary`, then `_server_identity` → `_installed_binary`.

That gives **6** with an owned generation binary and **4** with the official-pin binary. Which path the installed service uses is **unresolved**: it depends on the installed state, which I did not inspect.

Native HTTP per `Installation.check` (×2 per boundary): `/ready`, `/version`, plus `/sql` ×2 (one per database: `main`, `validation`). It also reads `/proc/<pid>/{exe,cmdline,environ}`.

### 4.3 Full-file hashes per boundary
- **The surreal executable:** 4 or 6 full SHA-256 passes, as above.
- **The source tree:** the first `SourceObserver.observe` **hashes every git-listed file** (tracked + untracked, not ignored) whose path contains none of the parts `fixtures`, `gold`, `heldout`, `.venv`, `target`, `build`, `.git`, after a sensitive-name filter. The second observation re-hashes only files whose stat identity changed. The cache lives per `CaptureOwner`, so **every verify invocation hashes the whole allowed tree once**. That grows with repository size, not with history.
- **Test binaries:** `command_finished` reads `nextest-events.jsonl` and hashes **each `compiler-artifact` executable** (cache per owner).
- **Small hashes:** nextest repo and user config canonical hashes; JSON identity hashes.

### 4.4 Locks per boundary
Enumerated, not summed. `admission` takes one `flock` per lexical ancestor of each path, so the count depends on path depth.
- `storage_lifecycle.admission`: run dir (`new_run_dir`), run dir + native and vllm env paths (`Owner.run`), 3 evidence paths (`optional_owner`, nested again in `CaptureOwner`), the attachment directory (`Attachment.create`), and inside every `Storage.publish`/`complete` (`admission([path])` + `metadata.lock` LOCK_EX).
- `Storage.publish` runs 6× per boundary (2 run, 2 evidence, 1 attachment, 1 workspace_env enrolment under nextest config composition if the tools env exists; **inferred**). `Storage.complete` runs ≥3×.
- Fixture: `admission.lock` (LOCK_SH, non-blocking) + `borrowers.lock` (LOCK_SH) in `service.borrow`; attachment `owner.lock` (EX); run `owner.lock` (EX).
- `workspace_env.ownership("shared","tools")` during nextest config composition: environment resource locks plus admission.

### 4.5 Durable writes per boundary
- `durable_json` (file fsync + directory fsync): each `Storage.save` writes the object descriptor + path index (2), each `publish` with `managed` adds a scope pointer (+1), and `enqueue` adds one only if `hooks-enabled.json` exists.
- `write_json_atomic` (file fsync, no directory fsync): run `record.json` (per progress change), the progress JSON (per `set_progress`), `summary.json` (per boundary), attachment `record.json` (×2: create, release), the evidence manifest (≥3), `invocation.json` (**about 8 saves per step**: start, effective, command, finalization, `context_after`, binaries, postprocessing ×2, `boundary_final`), and context JSON (once per distinct context ID).
- `service.private_json` ×2 (attachment runtime config and compiler runtime config).

### 4.6 What grows with retained history
- **`Storage().records()`**, called by `runs.Owner.run` finalization whenever `verify-evidence.json` exists, which is every verify run: it globs **every** `state/objects/*.json`, then reads and validates each. That is all storage descriptors on the host state root, including those of retired objects that were not trimmed.
- Run directories under `runs_root()` accumulate, but they are not enumerated on this path. `all_runs` is used by list, resolve and prune only.
- Attachment directories under `<installation>/attachments/*`: `service.borrowers()` scans them, but only on maintenance paths, not on `create`/`release` (searched `borrowers(` in `surrealdb_fixture.py` and `surrealdb_service.py`).
- The native store itself: each fixture-backed test creates logical attempts in the `validation` database. Its growth is not observed by the harness on this path.

---

## 5. Unresolved

1. **Profile-specific outcomes:** the exact outcome status under catalog for `prepare_source_calls`, `enrich_execution` and `apply_models` (set inside `lctx-model` execution producers; not traced).
2. **Whole-stage reuse hits:** whether the warm run actually hits for each normalization and upper stage. This depends on runtime equality of view identities, which is not executed.
3. **Bridge runtime binding:** which runtime the `surrealdb` gRPC client drives its I/O on when futures are polled on the bridge's current-thread runtime. The client was created on the main runtime; tokio and SDK semantics were not checked.
4. **Unmeasured costs:** the RT counts of `definitions::verify_epoch`, `lexical_stats::{materialize, reconcile}` and `retain_attached_members`, and the exact `original_chunk` read query in `serving::originals` / `evidence::body`.
5. **Fixture install path:** whether the installed service uses an owned server-generation binary (6 vs 4 `surreal version` runs and executable hashes).
6. **Volumes:** the number of rows per relation for FastMCP 4.0.5, which determines every "per W" multiplier. No product run was performed.

## 6. Search coverage for absence claims

- **No product reuse for facts providers:** `rg "reuse|attach_retained|retain_product"` over `crates/cpg-core/src/facts.rs`, `crates/cpg-extract/src/{bundle,pyrefly_stage,ty_flow}.rs` returned no hits; I also read `facts::compile_facts` in full.
- **No short-circuit for an identical existing publication:** I read `lctx_publisher::{seal_completed, publish}`, `Publication::{new, seal}` and `seal_staging` in full, and `rg "existing publication|already published|publication_exists"` over `crates/lctx-publisher` and `crates/lctx-surrealdb/src` returned no hits.
- **No vLLM HTTP in a default compile:** `compile_options::Options::prepare` sets the embedder to `EmbeddingChoice::None` when the flag is absent; `NativeEmbeddingCache` is connected only when an embedder is present (`compile.rs`).
- **No store RT on the continuation path:** read `NativeService::run` up to the `ranked.resume` early return and `RankedResults::resume`; this assumes the definition OnceCell is already initialised.
- **`store:rust` takes no environment ownership at boundary level:** `workspace_env.resources_for` returns empty for `native-store`. The nextest adapter does take `tools` ownership for config composition (`verify_evidence/nextest.py::prepare`).
